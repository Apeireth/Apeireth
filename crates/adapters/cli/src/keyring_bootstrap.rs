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

use apeireth_credentials::keyring::{AuditSink, CountingAudit, KeyringBackend, NoopAudit};
use apeireth_credentials::keyring_resolver::KeyringCredentialResolver;
use apeireth_credentials::{
    HotCredentialStore, KeyringSelector, LayeredCredentialResolver, SecretBuf,
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
/// 写入两层:
/// 1. [`hot_credential_store`] (主层, 总是成功) —— 与 `build_keyring_resolver`
///    共享同一实例, 下一次 provider 请求的 `resolve` 即取新值 (真热更);
/// 2. keyring backend (可选持久层) —— 仅服务进程重启后的存活; 未配置 keyring
///    backend 时跳过 (热更仍生效, 重启回退启动值)。
///
/// 只有持久层写入失败才返 Err (此时热更已生效, 错误文案如实说明)。
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

impl CredentialWriter for HotCredentialWriter {
    fn write(&self, name: &str, value: &str) -> Result<(), String> {
        // 主层先写: 每次请求现解析的 resolver 立即可见。
        self.store.set(name, value);
        // 持久层尽力而为: 失败不影响本次热更, 但要如实上报。
        if let Some(backend) = &self.durable {
            backend
                .set(name, &SecretBuf::from_str(value))
                .map_err(|error| {
                    format!("已热更生效（下次请求使用），但 keyring 持久化失败: {error}")
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真热更接线: admin 写入端口 (`build_keyring_credential_writer`) 与 provider
    /// 取值端 (`build_keyring_resolver`) 共享同一运行时凭据库 —— 写入后**下一次**
    /// resolve 现取新值 (诱饵实验的最小断言: 热更 key 后下一请求不再用旧 key)。
    #[test]
    fn credential_writer_and_resolver_share_the_runtime_store() {
        let name = "provider.seam-test.api_key";
        let writer = build_keyring_credential_writer().expect("hot writer is always mounted");
        writer.write(name, "sk-decoy-seam").expect("hot write");
        let resolver = build_keyring_resolver();
        let got = resolver
            .resolve(name)
            .expect("hot override visible on the next resolve");
        assert_eq!(got.expose(), "sk-decoy-seam");
        // 清理共享 store, 不污染同进程其它测试。
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
