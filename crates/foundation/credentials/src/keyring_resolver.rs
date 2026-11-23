//! P-arch (2026-08-27): KeyringCredentialResolver (O-6 重构批次 Refactor-4).
//!
//! **位置**: 在 `apeireth-credentials` (foundation) 实现 `apeireth-plugin::CredentialResolver` trait.
//! 文件名 `keyring_resolver.rs` 比旧名 `plugin_bridge.rs` 更准确 (impl only, 不是 bridge).
//!
//! **0 装 PASS**：
//! - 本文件是**接线**, 不新增 backend
//! - backend trait (`KeyringBackend`) 和 impls (`InMemoryKeyring` / `PlatformKeyring` / `EncryptedFileBackend`) 已存在
//! - resolver 只把 `KeyringError` 转 `Option<Secret>`：找不到/出错 = None（与 EnvCredentialResolver 行为一致）
//! - 真 secret 0 装脱敏: 走 `Secret::new(buf.expose())` 转 string，`Secret::Debug` 是 `Secret(<redacted>)`
//!
//! **架构收益** (O-6 锚 9, commit message 必写明):
//! - `apeireth-credentials` 不再是孤儿（之前 0 依赖方）
//! - provider / plugin 可以用 keyring 而不只是 env
//! - runtime 可以用 `KeyringSelector::select(...)` 选 backend 后包装成 `KeyringCredentialResolver` 注入
//!
//! **不在本 bridge 范围**：
//! - `KeyringSelector` 自动选 backend（per 0 装 PASS § 平台 keyring crate 3.6 不支持 list, 降级路径由调用方处理）
//! - 高危凭据审批门（`GatedCredentialsStore` + `DenyAllGate`）—— v2.0 runtime 不强制挂；装配侧按需
//! - 真 Ed25519/Ed25519-Ph 签名验证 —— v2.1 路线

use std::sync::Arc;

use apeireth_plugin::{CredentialResolver, CredentialSnapshot, Secret};

use crate::credential_record::{
    bound_record_service, decode_credential_record, is_bound_credential_reference, CredentialRecord,
};
use crate::keyring::KeyringBackend;

/// 把 `KeyringBackend` 包成 `CredentialResolver` trait。
///
/// **契约**：
/// - `resolve(name)` → `Ok(Secret(buf))` 转 `Some(Secret(string))`（keyring 拿到的 bytes 按 UTF-8 解释）
/// - `resolve(name)` → 任何 `KeyringError`（Unknown / Backend / Encoding）→ `None`（不报警，
///   与 `EnvCredentialResolver` 在 env var 不存在时返 None 一致）
/// - 这是**只读** wrapper：不实现 `set/delete`（`CredentialResolver` trait 也没有这两个方法）
pub struct KeyringCredentialResolver {
    keyring: Arc<dyn KeyringBackend>,
}

impl KeyringCredentialResolver {
    /// 包装一个 keyring backend。
    ///
    /// **典型用法**（v2.0.0-alpha.1）：
    /// ```ignore
    /// use apeireth_credentials::{KeyringSelector, KeyringCredentialResolver};
    /// use apeireth_plugin::CredentialResolver;
    ///
    /// let backend = KeyringSelector::select_default();  // 选 Linux/macOS/Windows keyring
    /// let resolver = KeyringCredentialResolver::new(backend);
    /// assert!(resolver.resolve("provider.minimax.api_key").is_some());
    /// ```
    pub fn new(keyring: Arc<dyn KeyringBackend>) -> Self {
        Self { keyring }
    }

    /// 内部 keyring 引用（用于装配期健康检查或调换）。
    pub fn keyring(&self) -> &Arc<dyn KeyringBackend> {
        &self.keyring
    }

    fn record(&self, name: &str) -> Option<CredentialRecord> {
        let legacy = self.keyring.get(name).ok()?;
        let referenced = is_bound_credential_reference(&legacy)
            .map_err(|error| {
                eprintln!(
                    "[apeireth-credentials] keyring credential reference refused: service={} error={}",
                    name, error
                );
            })
            .ok()?;
        let stored = if referenced {
            self.keyring.get(&bound_record_service(name)).ok()?
        } else {
            legacy
        };
        let record = decode_credential_record(&stored)
            .map_err(|error| {
                eprintln!(
                    "[apeireth-credentials] keyring credential record refused: service={} error={}",
                    name, error
                );
            })
            .ok()?;
        if !referenced && record.endpoints().is_some() {
            eprintln!(
                "[apeireth-credentials] unreferenced bound credential refused: service={} error=unsafe record placement",
                name
            );
            return None;
        }
        if referenced && record.endpoints().is_none() {
            eprintln!(
                "[apeireth-credentials] keyring bound credential companion refused: service={} error=missing binding",
                name
            );
            return None;
        }
        Some(record)
    }

    fn snapshot(&self, name: &str) -> Option<CredentialSnapshot> {
        let (secret, endpoints) = self.record(name)?.into_parts();
        if secret.is_empty() {
            return None;
        }
        let value = std::str::from_utf8(secret.expose())
            .map_err(|error| {
                eprintln!(
                    "[apeireth-credentials] keyring secret not utf-8: service={} error={}",
                    name, error
                );
            })
            .ok()?;
        Some(CredentialSnapshot::new(
            Secret::new(value.to_string()),
            endpoints,
        ))
    }
}

impl CredentialResolver for KeyringCredentialResolver {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.snapshot(name).map(CredentialSnapshot::into_secret)
    }

    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        self.record(name)
            .and_then(|record| record.endpoints().map(|endpoints| endpoints.to_vec()))
    }

    fn resolve_snapshot(&self, name: &str) -> Option<CredentialSnapshot> {
        self.snapshot(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential_record::encode_credential_record;
    use crate::keyring::{InMemoryKeyring, KeyringError};
    use std::sync::Arc;

    #[test]
    fn resolves_an_existing_service() {
        let k = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        k.set(
            "provider.fake.api_key",
            &crate::secret::SecretBuf::from_str("sk-test"),
        )
        .unwrap();
        let r = KeyringCredentialResolver::new(k);
        let got = r.resolve("provider.fake.api_key");
        assert!(got.is_some());
        assert_eq!(got.unwrap().expose(), "sk-test");
    }

    #[test]
    fn unknown_service_returns_none_not_error() {
        // 0 装 PASS: 不存在 = None（与 EnvCredentialResolver 一致）
        let k = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        let r = KeyringCredentialResolver::new(k);
        assert!(r.resolve("does.not.exist").is_none());
    }

    #[test]
    fn redaction_survives_formatting() {
        // 0 装 PASS: Secret 的 Debug 不暴露 value
        let k = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        k.set(
            "provider.fake.api_key",
            &crate::secret::SecretBuf::from_str("sk-super-secret-do-not-leak"),
        )
        .unwrap();
        let r = KeyringCredentialResolver::new(k);
        let s = r.resolve("provider.fake.api_key").unwrap();
        assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
        assert_eq!(format!("{s}"), "<redacted>");
        assert!(!format!("{s:?}").contains("sk-super-secret"));
    }

    #[test]
    fn backend_errors_become_none() {
        // 0 装 PASS: backend 内部错误不污染调用方
        // 这里用 InMemoryKeyring 删一个不存在的服务 → KeyringError::UnknownService
        let k = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        let r = KeyringCredentialResolver::new(k);
        // get 失败 → resolve 返 None
        let result = r.resolve("never.written");
        assert!(result.is_none());
        // 显式验证 KeyringError::UnknownService 存在（编译期保证接口对齐）
        let _: KeyringError = KeyringError::UnknownService {
            service: "x".into(),
        };
    }

    #[test]
    fn one_companion_record_restores_the_value_and_binding() {
        let keyring = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        keyring
            .set(
                &bound_record_service("provider.bound.api_key"),
                &encode_credential_record(
                    "synthetic-bound-value",
                    Some(&["https://model.example/v1".to_string()]),
                )
                .unwrap(),
            )
            .unwrap();
        keyring
            .set(
                "provider.bound.api_key",
                &crate::bound_credential_reference(),
            )
            .unwrap();
        let resolver = KeyringCredentialResolver::new(keyring);
        let snapshot = resolver
            .resolve_snapshot("provider.bound.api_key")
            .expect("bound snapshot");
        assert_eq!(snapshot.secret().expose(), "synthetic-bound-value");
        assert_eq!(
            snapshot.bound_endpoints(),
            Some(["https://model.example/v1".to_string()].as_slice())
        );
    }

    #[test]
    fn malformed_bound_record_fails_closed() {
        let keyring = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        keyring
            .set(
                &bound_record_service("provider.bound.api_key"),
                &crate::SecretBuf::from_str("apeireth:bound-credential:v1:{bad"),
            )
            .unwrap();
        keyring
            .set(
                "provider.bound.api_key",
                &crate::bound_credential_reference(),
            )
            .unwrap();
        let resolver = KeyringCredentialResolver::new(keyring);
        assert!(resolver
            .resolve_snapshot("provider.bound.api_key")
            .is_none());
    }

    #[test]
    fn envelope_under_legacy_name_is_refused() {
        let keyring = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        keyring
            .set(
                "provider.bound.api_key",
                &encode_credential_record(
                    "synthetic-bound-value",
                    Some(&["https://model.example".to_string()]),
                )
                .unwrap(),
            )
            .unwrap();
        let resolver = KeyringCredentialResolver::new(keyring);
        assert!(resolver
            .resolve_snapshot("provider.bound.api_key")
            .is_none());
    }

    #[test]
    fn missing_bound_companion_fails_closed() {
        let keyring = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        keyring
            .set(
                "provider.bound.api_key",
                &crate::bound_credential_reference(),
            )
            .unwrap();
        let resolver = KeyringCredentialResolver::new(keyring);
        assert!(resolver
            .resolve_snapshot("provider.bound.api_key")
            .is_none());
    }
}
