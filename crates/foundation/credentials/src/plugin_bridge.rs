//! P-arch (2026-08-27): 把 `apeireth-credentials` 接到 `apeireth-plugin::CredentialResolver` trait。
//!
//! **0 装 PASS**：
//! - 本文件是**接线 bridge**，不新增 backend
//! - backend trait (`KeyringBackend`) 和 impls (`InMemoryKeyring` / `PlatformKeyring` / `EncryptedFileBackend`) 已存在
//! - bridge 只把 `KeyringError` 转 `Option<Secret>`：找不到/出错 = None（与 EnvCredentialResolver 行为一致）
//! - 真 secret 0 装脱敏: 走 `Secret::new(buf.expose())` 转 string，`Secret::Debug` 是 `Secret(<redacted>)`
//!
//! **架构收益**：
//! - `apeireth-credentials` 不再是孤儿（之前 0 依赖方）
//! - provider / plugin 可以用 keyring 而不只是 env
//! - runtime 可以用 `KeyringSelector::select(...)` 选 backend 后包装成 `KeyringCredentialResolver` 注入
//!
//! **不在本 bridge 范围**：
//! - `KeyringSelector` 自动选 backend（per 0 装 PASS § 平台 keyring crate 3.6 不支持 list, 降级路径由调用方处理）
//! - 高危凭据审批门（`GatedCredentialsStore` + `DenyAllGate`）—— v2.0 runtime 不强制挂；装配侧按需
//! - 真 Ed25519/Ed25519-Ph 签名验证 —— v2.1 路线

use std::sync::Arc;

use apeireth_plugin::{CredentialResolver, Secret};

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
}

impl CredentialResolver for KeyringCredentialResolver {
    fn resolve(&self, name: &str) -> Option<Secret> {
        // 0 装 PASS: 任何 KeyringError 都转 None（不污染调用方）
        // 真实错误在 keyring 自己的 audit sink / 日志里可见
        let buf = self.keyring.get(name).ok()?;
        // SecretBuf 是 bytes；CredentialResolver 的 Secret 是 String (new)
        // UTF-8 转换失败（罕见）→ None（不假装）
        // 0 装 PASS (v2.0): 走 stderr 调试日志。v2.1 接 tracing + 审计 sink
        let s = std::str::from_utf8(buf.expose())
            .map_err(|e| {
                eprintln!(
                    "[apeireth-credentials] keyring secret not utf-8: service={} error={}",
                    name, e
                );
            })
            .ok()?;
        Some(Secret::new(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyring::{InMemoryKeyring, KeyringError};
    use std::sync::Arc;

    #[test]
    fn resolves_an_existing_service() {
        let k = Arc::new(InMemoryKeyring::new(Arc::new(crate::keyring::NoopAudit)));
        k.set("provider.fake.api_key", &crate::secret::SecretBuf::from_str("sk-test"))
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
}
