//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-9: KeyringSelector 真接入 CLI/gateway bootstrap.
//!
//! **位置**: 本模块在 `apeireth-cli` (adapter), 用 `apeireth-credentials::KeyringSelector` +
//! `KeyringCredentialResolver` 真接 OS keyring / EncryptedFile / InMemory.
//! 0 装诚实: 4 backend + selector 已在 alpha 真 impl (per `apeireth-credentials::keyring`),
//! 本模块只做**接入** (per v2.0.0-rc-roadmap.md §3 RC-9: "keyring 真正接到 EnvCredentialResolver 之前").
//!
//! **设计**: `build_keyring_resolver()` 优先用 `KeyringSelector` (按 `APEIRETH_KEYRING_BACKEND`
//! env 选 backend), 没有 env 时 fallback 到 `EnvCredentialResolver` (alpha 已有).
//! 这样 alpha 用户**无感升级** (没设 env → 走 env resolver, 0 行为变化), 部署 v2.0
//! 时设 env → 走真 keyring (Linux Secret Service / macOS Keychain / Windows Credential Manager).
//!
//! **真热更** (凭据按请求现解析): resolver 主层是进程级 [`hot_credential_store`]
//! (admin 运行时凭据库), keyring/env 只作启动值次层; `/v1/admin/config` 的写入端口
//! (`build_keyring_credential_writer`) 与取值端共享同一 store, 一次写入对下一次
//! 请求的 `resolve` 即刻生效, 凭据值不被任何请求路径对象跨请求持有。
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与 RC-1 真 SQL 同样模式 (alpha 写真完整, 接 bootstrap 即可)
//! 2. 系统: bootstrap 选择 resolver, 不引入新 cross-crate 依赖 (KeyringSelector 在 credentials
//!    crate, 已在依赖图内)
//! 3. 架构: `KeyringCredentialResolver` 已在 plugin::CredentialResolver trait 上 impl,
//!    runtime 拿 `Arc<dyn CredentialResolver>` 注入, 0 改 Runtime 接口
//!
//! **0 装诚实**:
//! - alpha 0 设 `APEIRETH_KEYRING_BACKEND` → fallback `EnvCredentialResolver` (0 行为变化)
//! - 设 `platform` / `encrypted-file` / `in-memory` / `auto` → 真接 selector 4 backend
//! - 真接 OS keyring 是 `KeyringSelector::select()` 真实实现, 0 在本模块写
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use apeireth_credentials::keyring::{
    AuditSink, CountingAudit, KeyringBackend, KeyringError, NoopAudit,
};
use apeireth_credentials::keyring_resolver::KeyringCredentialResolver;
use apeireth_credentials::{
    bound_credential_reference, bound_record_service, decode_credential_record,
    encode_credential_record, is_bound_credential_reference, HotCredentialStore, KeyringSelector,
    LayeredCredentialResolver, SecretBuf,
};
use apeireth_gateway::CredentialWriter;
use apeireth_plugin::CredentialResolver;

/// 进程级 admin 运行时凭据库 (真热更主层)。
///
/// `/v1/admin/config` 的 api_key 写入端口与 provider 的取值端共享这一实例,
/// 所以一次写入对**下一次**请求的 `resolve` 即刻生效。它只存脱敏
/// [`apeireth_plugin::Secret`], 明文不入任何日志。
pub fn hot_credential_store() -> Arc<HotCredentialStore> {
    static HOT: OnceLock<Arc<HotCredentialStore>> = OnceLock::new();
    HOT.get_or_init(|| Arc::new(HotCredentialStore::new()))
        .clone()
}

/// CLI/gateway 启动时构造 `Arc<dyn CredentialResolver>`, 优先用 KeyringSelector 真接
/// OS keyring, 退化到 `EnvCredentialResolver` (alpha 0 装路径).
///
/// **分层** (真热更): 返回值 = [`LayeredCredentialResolver`], 主层是共享的
/// [`hot_credential_store`] (admin 运行时写入), 次层是下述 keyring/env 启动值。
/// provider 每次请求现解析, 因此 admin 更新凭据后**下一请求**即用新值。
///
/// **优先级** (per v2.0.0-rc-roadmap.md §3 RC-9):
/// 1. `APEIRETH_KEYRING_BACKEND` env 已设 → `KeyringSelector::select(env, audit, fallback_dir)`
///    拿 SelectedBackend → `KeyringCredentialResolver::new(backend)` (per `keyring_resolver.rs`)
/// 2. 1 失败 (KeyringSelector 选 backend 失败, e.g. EncryptedFileBackend::open IO error)
///    → fallback `EnvCredentialResolver` (0 装诚实: 真 fallback, 不静默 0 装)
/// 3. 没设 env → 直接 `EnvCredentialResolver` (alpha 路径, 0 行为变化)
///
/// **返回**: Send+Sync `Arc<dyn CredentialResolver>`, runtime 拿它注入.
pub fn build_keyring_resolver() -> Arc<dyn CredentialResolver> {
    // 受管桌面：启动时 `desktop_model_connection::prepare` 已把唯一 profile
    // 的凭据恢复到进程热库；不得再落到别的 profile 的 keyring / env。
    if crate::desktop_model_connection::managed() {
        return hot_credential_store();
    }
    // 次层: 优先 KeyringSelector 真接 (RC-9)
    let fallback: Arc<dyn CredentialResolver> = match try_build_keyring_resolver() {
        Ok(resolver) => resolver,
        Err(reason) => {
            // 0 装诚实: 退化时**真**用 EnvCredentialResolver, 不假装"我有 keyring"
            // 0 装诚实: 退化原因写到 stderr, 不静默 (运维可看到为什么)
            eprintln!(
                "[keyring] KeyringSelector 退化到 EnvCredentialResolver: {reason} \
                 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)"
            );
            Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new())
        }
    };
    // 主层: admin 运行时凭据库 (写入端口与本 resolver 共享同一实例)
    Arc::new(LayeredCredentialResolver::new(
        hot_credential_store(),
        fallback,
    ))
}

/// 真接 KeyringSelector, 失败返 Err (退化由 caller 处理)
fn try_build_keyring_resolver() -> Result<Arc<dyn CredentialResolver>, String> {
    Ok(Arc::new(KeyringCredentialResolver::new(
        try_build_keyring_backend()?,
    )))
}

/// 真接 KeyringSelector 选 backend, 失败返 Err (退化由 caller 处理)。
/// 与 [`try_build_keyring_resolver`] 共用同一选择逻辑, 供 `/v1/admin/config`
/// 的 api_key 热写入口复用同一个 keyring backend。
fn try_build_keyring_backend() -> Result<Arc<dyn KeyringBackend>, String> {
    // 读 env: APEIRETH_KEYRING_BACKEND (per v2.0.0-rc-roadmap.md §3 RC-9: "KeyringSelector::select()
    // 真实按 APEIRETH_KEYRING_BACKEND env 选择")
    let env_value = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
    if env_value.is_none() {
        // 0 设 env: 不算错, 退化由 caller 处理
        return Err("APEIRETH_KEYRING_BACKEND 未设".to_string());
    }
    // 构造 audit sink (CountingAudit 0 装, 真生产换真 audit)
    let audit: Arc<dyn AuditSink> = if cfg!(test) {
        Arc::new(NoopAudit)
    } else {
        Arc::new(CountingAudit::new())
    };
    // fallback dir (EncryptedFile backend 用, 默认 ~/.apeireth/keyring/)
    let fallback_dir: Option<PathBuf> = std::env::var_os("APEIRETH_KEYRING_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            // 默认 ~/.apeireth/keyring/ (per `keyring.rs::default_dir`)
            if let Some(home) = std::env::var_os("HOME") {
                Some(PathBuf::from(home).join(".apeireth").join("keyring"))
            } else if let Some(profile) = std::env::var_os("USERPROFILE") {
                Some(PathBuf::from(profile).join(".apeireth").join("keyring"))
            } else {
                Some(PathBuf::from(".apeireth/keyring"))
            }
        });
    // KeyringSelector::select (4 backend: platform / encrypted-file / in-memory / auto)
    let selected = KeyringSelector::select(env_value.as_deref(), audit, fallback_dir)
        .map_err(|e| format!("KeyringSelector::select 失败: {e}"))?;
    // 0 装诚实: backend 名字 (platform / encrypted-file / in-memory / auto) 写到 stderr
    // 运维可见 (per `selected.kind`)
    eprintln!("[keyring] KeyringSelector 选 backend: {:?}", selected.kind);
    // KeyringCredentialResolver::new 接受 Arc<dyn KeyringBackend>,
    // selected.backend 是 Box<dyn KeyringBackend>, 转 Arc
    Ok(selected.backend.into())
}

/// `/v1/admin/config` 的 api_key 热写端口。
///
/// 写入两层: 可选 keyring 持久层先行，成功后才原子替换运行时值与绑定。
/// 因此任何持久化错误都不会留下一个仅热生效的新凭据版本。未配置 keyring
/// 时只更新进程热库，重启后仍按既有启动值路径恢复。
pub fn build_keyring_credential_writer() -> Option<Arc<dyn CredentialWriter>> {
    Some(Arc::new(HotCredentialWriter {
        store: hot_credential_store(),
        durable: try_build_keyring_backend().ok(),
    }))
}

struct HotCredentialWriter {
    store: Arc<HotCredentialStore>,
    durable: Option<Arc<dyn KeyringBackend>>,
}

enum DurableBinding {
    Missing,
    Unmanaged,
    Bound(Vec<String>),
}

fn keyring_error_class(error: &KeyringError) -> &'static str {
    match error {
        KeyringError::UnknownService { .. } => "unknown_service",
        KeyringError::InvalidServiceName(_) => "invalid_service_name",
        KeyringError::ServiceNameTooLong { .. } => "service_name_too_long",
        KeyringError::BackendUnavailable(_) => "backend_unavailable",
        KeyringError::AccessDenied { .. } => "access_denied",
        KeyringError::Io { .. } => "io",
        KeyringError::Crypto(_) => "crypto",
        KeyringError::Format(_) => "format",
        KeyringError::SecretTooLong { .. } => "secret_too_long",
        KeyringError::Backend(_) => "backend",
    }
}

impl HotCredentialWriter {
    fn read_durable_binding(&self, name: &str) -> Result<DurableBinding, String> {
        let Some(backend) = &self.durable else {
            return Ok(DurableBinding::Missing);
        };
        let original = match backend.get(name) {
            Ok(stored) => stored,
            Err(KeyringError::UnknownService { .. }) => return Ok(DurableBinding::Missing),
            Err(error) => {
                return Err(format!(
                    "keyring read failed ({})",
                    keyring_error_class(&error)
                ))
            }
        };
        let referenced = is_bound_credential_reference(&original)
            .map_err(|error| format!("keyring credential reference refused: {error}"))?;
        let stored = if referenced {
            backend.get(&bound_record_service(name)).map_err(|error| {
                format!(
                    "keyring bound credential read failed ({})",
                    keyring_error_class(&error)
                )
            })?
        } else {
            original
        };
        let record = decode_credential_record(&stored)
            .map_err(|error| format!("keyring credential record refused: {error}"))?;
        match record.endpoints() {
            Some(_) if !referenced => {
                Err("unreferenced bound credential record refused".to_string())
            }
            Some(endpoints) => Ok(DurableBinding::Bound(endpoints.to_vec())),
            None if referenced => {
                Err("keyring bound credential is missing its binding".to_string())
            }
            None => Ok(DurableBinding::Unmanaged),
        }
    }

    fn effective_binding(&self, name: &str) -> Result<Option<Vec<String>>, String> {
        if let Some(snapshot) = self.store.resolve_snapshot(name) {
            if let Some(endpoints) = snapshot.bound_endpoints() {
                return Ok(Some(endpoints.to_vec()));
            }
        }
        match self.read_durable_binding(name)? {
            DurableBinding::Bound(endpoints) => Ok(Some(endpoints)),
            DurableBinding::Missing | DurableBinding::Unmanaged => Ok(None),
        }
    }

    fn persist_bound(&self, name: &str, value: &str, endpoints: &[String]) -> Result<(), String> {
        let Some(backend) = &self.durable else {
            return Ok(());
        };
        let companion = bound_record_service(name);
        let encoded = encode_credential_record(value, Some(endpoints))
            .map_err(|error| format!("keyring bound credential encode failed: {error}"))?;
        let marker_active = match backend.get(name) {
            Ok(stored) => is_bound_credential_reference(&stored)
                .map_err(|error| format!("keyring credential reference refused: {error}"))?,
            Err(KeyringError::UnknownService { .. }) => false,
            Err(error) => {
                return Err(format!(
                    "keyring read failed ({})",
                    keyring_error_class(&error)
                ))
            }
        };
        backend.set(&companion, &encoded).map_err(|error| {
            format!(
                "keyring bound credential persistence failed ({})",
                keyring_error_class(&error)
            )
        })?;
        if marker_active {
            return Ok(());
        }
        if let Err(error) = backend.set(name, &bound_credential_reference()) {
            let cleanup_failed = backend.delete(&companion).is_err();
            return Err(format!(
                "keyring credential reference persistence failed ({}); companion cleanup {}",
                keyring_error_class(&error),
                if cleanup_failed {
                    "failed"
                } else {
                    "completed"
                }
            ));
        }
        Ok(())
    }

    fn persist_unmanaged(&self, name: &str, value: &str) -> Result<(), String> {
        let Some(backend) = &self.durable else {
            return Ok(());
        };
        backend
            .set(name, &SecretBuf::from_str(value))
            .map_err(|error| {
                format!(
                    "keyring credential persistence failed ({})",
                    keyring_error_class(&error)
                )
            })
    }
}

impl CredentialWriter for HotCredentialWriter {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        if value.is_empty() {
            return Err("credential value must not be empty".to_string());
        }
        let endpoints = self.effective_binding(name)?;
        if let Some(endpoints) = endpoints {
            self.persist_bound(name, value, &endpoints)?;
            self.store.set_with_endpoints(name, value, endpoints);
        } else {
            self.persist_unmanaged(name, value)?;
            self.store.set(name, value);
        }
        Ok(())
    }

    /// 该凭据**已声明**的端点（`None` = 未声明 ⇒ 不参与绑定闸）。
    ///
    /// 查询不向调用方暴露密文；为保证绑定与值同版本，必要时会读取并解码
    /// 同一个受保护的 keyring 伴随记录。
    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        self.effective_binding(name).ok().flatten()
    }

    /// 写入凭据**并**登记它的端点绑定（安全审计 P1 的「凭据库维度」）。
    ///
    /// 运行时 store 与 keyring 伴随记录都把绑定和值作为一个凭据版本处理。
    /// 旧服务名只保存不含密钥的引用标记，旧二进制不会误把结构化记录连同
    /// 真密钥发送到网络。
    fn write_with_endpoints(
        &self,
        name: &str,
        value: &str,
        endpoints: &[String],
    ) -> Result<(), String> {
        if value.is_empty() {
            return Err("credential value must not be empty".to_string());
        }
        self.persist_bound(name, value, endpoints)?;
        self.store
            .set_with_endpoints(name, value, endpoints.iter().cloned());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_credentials::keyring::{InMemoryKeyring, NoopAudit};
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    struct RejectingBackend;

    impl KeyringBackend for RejectingBackend {
        fn get(&self, service: &str) -> apeireth_credentials::keyring::Result<SecretBuf> {
            Err(KeyringError::UnknownService {
                service: service.to_string(),
            })
        }

        fn set(
            &self,
            _service: &str,
            _secret: &SecretBuf,
        ) -> apeireth_credentials::keyring::Result<()> {
            Err(KeyringError::Backend(
                "synthetic rejection containing sk-must-not-escape".to_string(),
            ))
        }

        fn delete(&self, service: &str) -> apeireth_credentials::keyring::Result<()> {
            Err(KeyringError::UnknownService {
                service: service.to_string(),
            })
        }

        fn list(&self) -> apeireth_credentials::keyring::Result<Vec<String>> {
            Ok(Vec::new())
        }

        fn backend_name(&self) -> &'static str {
            "synthetic-rejecting"
        }
    }

    struct MarkerRejectingBackend {
        marker_name: String,
        values: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    impl KeyringBackend for MarkerRejectingBackend {
        fn get(&self, service: &str) -> apeireth_credentials::keyring::Result<SecretBuf> {
            self.values
                .lock()
                .unwrap()
                .get(service)
                .map(|value| SecretBuf::new(value.clone()))
                .ok_or_else(|| KeyringError::UnknownService {
                    service: service.to_string(),
                })
        }

        fn set(
            &self,
            service: &str,
            secret: &SecretBuf,
        ) -> apeireth_credentials::keyring::Result<()> {
            if service == self.marker_name {
                return Err(KeyringError::Backend(
                    "marker rejection containing sk-must-not-escape".to_string(),
                ));
            }
            self.values
                .lock()
                .unwrap()
                .insert(service.to_string(), secret.expose().to_vec());
            Ok(())
        }

        fn delete(&self, service: &str) -> apeireth_credentials::keyring::Result<()> {
            self.values
                .lock()
                .unwrap()
                .remove(service)
                .map(|_| ())
                .ok_or_else(|| KeyringError::UnknownService {
                    service: service.to_string(),
                })
        }

        fn list(&self) -> apeireth_credentials::keyring::Result<Vec<String>> {
            Ok(self.values.lock().unwrap().keys().cloned().collect())
        }

        fn backend_name(&self) -> &'static str {
            "synthetic-marker-rejecting"
        }
    }

    #[test]
    fn failed_durable_compound_write_leaves_the_hot_store_unchanged() {
        let store = Arc::new(HotCredentialStore::new());
        let writer = HotCredentialWriter {
            store: Arc::clone(&store),
            durable: Some(Arc::new(RejectingBackend)),
        };
        let name = "provider.atomic-test.api_key";

        let result = writer.write_with_endpoints(
            name,
            "synthetic-value",
            &["https://endpoint.example/v1".to_string()],
        );
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("sk-must-not-escape"));
        assert!(store.resolve(name).is_none());
        assert_eq!(store.declared_endpoints(name), None);
    }

    #[test]
    fn failed_reference_write_rolls_back_the_unreferenced_companion() {
        let name = "provider.marker-failure.api_key";
        let backend = Arc::new(MarkerRejectingBackend {
            marker_name: name.to_string(),
            values: Mutex::new(BTreeMap::new()),
        });
        let store = Arc::new(HotCredentialStore::new());
        let writer = HotCredentialWriter {
            store: Arc::clone(&store),
            durable: Some(backend.clone()),
        };

        let error = writer
            .write_with_endpoints(
                name,
                "synthetic-value",
                &["https://endpoint.example".to_string()],
            )
            .expect_err("marker write must fail");
        assert!(!error.contains("sk-must-not-escape"));
        assert!(store.resolve(name).is_none());
        assert!(backend.get(&bound_record_service(name)).is_err());
    }

    #[test]
    fn bound_keyring_record_survives_restart_and_key_only_rotation() {
        let backend = Arc::new(InMemoryKeyring::new(Arc::new(NoopAudit)));
        let name = "provider.restart-test.api_key";
        let endpoints = vec!["https://model.example/v1".to_string()];
        let writer = HotCredentialWriter {
            store: Arc::new(HotCredentialStore::new()),
            durable: Some(backend.clone()),
        };
        writer
            .write_with_endpoints(name, "synthetic-first-value", &endpoints)
            .expect("persist bound credential");

        let marker = backend.get(name).expect("reference marker");
        assert!(is_bound_credential_reference(&marker).unwrap());
        assert!(!std::str::from_utf8(marker.expose())
            .unwrap()
            .contains("synthetic-first-value"));

        let restarted = KeyringCredentialResolver::new(backend.clone());
        let snapshot = restarted.resolve_snapshot(name).expect("restart snapshot");
        assert_eq!(snapshot.secret().expose(), "synthetic-first-value");
        assert_eq!(snapshot.bound_endpoints(), Some(endpoints.as_slice()));

        let rotated_writer = HotCredentialWriter {
            store: Arc::new(HotCredentialStore::new()),
            durable: Some(backend.clone()),
        };
        rotated_writer
            .write(name, "synthetic-rotated-value")
            .expect("rotate while retaining binding");
        let rotated = KeyringCredentialResolver::new(backend)
            .resolve_snapshot(name)
            .expect("rotated snapshot");
        assert_eq!(rotated.secret().expose(), "synthetic-rotated-value");
        assert_eq!(rotated.bound_endpoints(), Some(endpoints.as_slice()));
    }

    /// 真热更接线: admin 写入端口 (`build_keyring_credential_writer`) 与 provider
    /// 取值端 (`build_keyring_resolver`) 共享同一运行时凭据库 —— 写入后**下一次**
    /// resolve 现取新值 (诱饵实验的最小断言: 热更 key 后下一请求不再用旧 key)。
    #[test]
    fn credential_writer_and_resolver_share_the_runtime_store() {
        let name = "provider.seam-test.api_key";
        // 测试写入端不挂持久层（durable=None）：断言只关心运行时凭据库的共享语义，
        // 任何测试数据都不落进真实钥匙串（测试隔离）。
        let writer = HotCredentialWriter {
            store: hot_credential_store(),
            durable: None,
        };
        writer.write(name, "sk-decoy-seam").expect("hot write");
        let resolver = build_keyring_resolver();
        let got = resolver
            .resolve(name)
            .expect("hot override visible on the next resolve");
        assert_eq!(got.expose(), "sk-decoy-seam");
        // 清理共享 store, 不污染同进程其它测试。
        hot_credential_store().remove(name);
    }

    /// `bound_endpoints` 是网关**如实告警**的数据来源（2026-11-04）。
    ///
    /// ⭐ 必须钉住三件事：
    /// ① 写了带绑定的凭据 ⇒ 查询**得到**绑定（否则网关会误报「未声明绑定」⇒ 告警成噪音）；
    /// ② 只写值（无绑定）⇒ 查询为 `None`（未声明 ⇒ 不参与绑定闸）；
    /// ③ 查询**不接触密文**（只读绑定表 ⇒ 绝不返回任何 key）。
    #[test]
    fn bound_endpoints_reports_the_declared_binding_without_touching_the_secret() {
        let writer = HotCredentialWriter {
            store: hot_credential_store(),
            durable: None,
        };
        let name = "provider.bound-lookup.api_key";

        // ② 未写 ⇒ 未声明 ⇒ None。
        assert_eq!(
            writer.bound_endpoints(name),
            None,
            "没写过 ⇒ 没声明绑定 ⇒ 必须 None（否则网关会误报「未声明绑定」）"
        );

        // ① 写了带绑定的 ⇒ 查得到。
        writer
            .write_with_endpoints(
                name,
                "sk-lookup-test",
                &["https://a.example/v1".to_string()],
            )
            .expect("write with endpoints");
        assert_eq!(
            writer.bound_endpoints(name),
            Some(vec!["https://a.example/v1".to_string()]),
            "同笔登记的绑定必须查得到"
        );

        // Same-provider key rotation is value-only: it must not erase the
        // already declared destination and silently return the key to an
        // unmanaged state.
        writer
            .write(name, "synthetic-rotated-value")
            .expect("rotate key");
        assert_eq!(
            writer.bound_endpoints(name),
            Some(vec!["https://a.example/v1".to_string()]),
            "只换同名凭据值必须保留原端点声明"
        );

        // ③ 查询结果里**不含任何密文**。
        let listed = format!("{:?}", writer.bound_endpoints(name));
        assert!(
            !listed.contains("sk-lookup-test"),
            "绑定查询**不得**返回或泄漏密文：{listed}"
        );

        hot_credential_store().remove(name);
    }

    /// RC-9 验收: 没设 env → 退化到 EnvCredentialResolver (alpha 路径 0 行为变化)
    #[test]
    fn no_env_falls_back_to_env_resolver() {
        // 测试时清空 env (避免宿主环境干扰)
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        let resolver = build_keyring_resolver();
        // 退化到 EnvCredentialResolver: 应返 None (env 未设)
        let result = resolver.resolve("provider.minimax.api_key");
        assert!(
            result.is_none(),
            "退化路径: EnvCredentialResolver 没拿到 key, 返 None"
        );
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        }
    }

    /// RC-9 验收: 设 `in-memory` → KeyringSelector 真选 InMemoryKeyring
    #[test]
    fn in_memory_backend_via_selector() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "in-memory");
        let resolver = build_keyring_resolver();
        // InMemoryKeyring: 空, 返 None (没存任何 secret)
        let result = resolver.resolve("provider.minimax.api_key");
        assert!(result.is_none(), "InMemoryKeyring 空, 返 None");
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
    }

    /// RC-9 验收: 设 `encrypted-file` + fallback dir 不存在 → 退化 (EncryptedFileBackend::open IO error)
    #[test]
    fn encrypted_file_missing_dir_falls_back() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        let saved_dir = std::env::var_os("APEIRETH_KEYRING_DIR");
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "encrypted-file");
        std::env::set_var(
            "APEIRETH_KEYRING_DIR",
            "/nonexistent/apeireth/keyring/that/does/not/exist",
        );
        let resolver = build_keyring_resolver();
        // 退化到 EnvCredentialResolver: 不崩
        let result = resolver.resolve("any.service");
        assert!(result.is_none());
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
        if let Some(d) = saved_dir {
            std::env::set_var("APEIRETH_KEYRING_DIR", d);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_DIR");
        }
    }

    /// RC-9 验收: 设 `auto` → KeyringSelector::select 走 select_auto 路径 (probe + fallback)
    /// 测试环境无 OS keyring (CI / Linux container) → probe fail → EncryptedFile fallback
    /// 没 fallback dir → EncryptedFile open 失败 → in-memory stub
    #[test]
    fn auto_select_with_no_backend_available() {
        let saved = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
        let saved_dir = std::env::var_os("APEIRETH_KEYRING_DIR");
        std::env::set_var("APEIRETH_KEYRING_BACKEND", "auto");
        std::env::set_var("APEIRETH_KEYRING_DIR", "/nonexistent/dir");
        let resolver = build_keyring_resolver();
        // auto 在无 backend 环境下走 in-memory → 没存, 返 None
        let result = resolver.resolve("any.service");
        assert!(result.is_none());
        if let Some(s) = saved {
            std::env::set_var("APEIRETH_KEYRING_BACKEND", s);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_BACKEND");
        }
        if let Some(d) = saved_dir {
            std::env::set_var("APEIRETH_KEYRING_DIR", d);
        } else {
            std::env::remove_var("APEIRETH_KEYRING_DIR");
        }
    }
}
