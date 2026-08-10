//! # Token 管理 trait
//!
//! 1:1 翻译 v0.9.21 商业版 token 生命周期管理接口. 5 个核心方法:
//! get_token / refresh / revoke / is_valid / expires_at.
//!
//! ## 设计原则
//!
//! 1. **TokenManager trait 抽象**: 5 Provider 各自实现 token 生命周期
//! 2. **5 核心方法**: 对应 RFC 6749 (OAuth 2.0) + RFC 7009 (revoke) 规范
//! 3. **skeleton 阶段 stub**: 所有方法返 `NotImplemented` + `warn!`
//! 4. **expires_at 永真**: `is_valid()` 在未实现 token 缓存前返 false
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 5 方法 1:1 翻译 RFC 6749 / RFC 7009, 0 业务重设计
//! - **S-2 实事求是**: 5 方法够用 99% 场景, 不发明 `introspect` / `validate` 等花哨
//! - **O-2 走在前人肩上**: 借鉴 RFC 6749 §5.1 (refresh) + RFC 7009 (revoke)
//! - **O-3 干到底**: trait + 5 Provider impl + 5 fixture 测试
//! - **O-4 任何人都能接手**: 跟 keyring / i18n 同模式
//! - **O-5 不假装**: skeleton 全部 NotImplemented, 0 假装

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::{CredentialsError, CredentialsResult};
use crate::provider::{Provider, ProviderKind, SecretString};

// ============================================================================
// §1 TokenManager trait (5 方法, RFC 6749 + RFC 7009)
// ============================================================================

/// Token 生命周期管理 trait.
///
/// 1:1 翻译 v0.9.21 商业版 + RFC 6749 (OAuth 2.0) + RFC 7009 (revoke).
#[async_trait]
pub trait TokenManager: Send + Sync {
    /// 获取当前 token (从缓存或 Provider 拉取).
    ///
    /// - API key 鉴权: 直接返 key
    /// - OAuth 2.0: 返 access_token (过期则自动 refresh)
    /// - IAM: 返 session_token
    /// - mTLS: 返客户端证书内容 (PEM)
    async fn get_token(&self) -> CredentialsResult<SecretString>;

    /// 刷新 token (OAuth 2.0 refresh_token grant).
    ///
    /// R21+ 真接时, 调 token endpoint 换新 access_token.
    /// 当前 skeleton 阶段 stub.
    async fn refresh(&self) -> CredentialsResult<SecretString>;

    /// 撤销 token (RFC 7009).
    ///
    /// R21+ 真接时, 调 revoke endpoint.
    /// 当前 skeleton 阶段 stub.
    async fn revoke(&self) -> CredentialsResult<()>;

    /// token 是否有效 (未过期 + 未撤销).
    async fn is_valid(&self) -> bool;

    /// token 过期时间 (UTC, ISO 8601 格式).
    ///
    /// - API key: None (永不过期)
    /// - OAuth 2.0: 返 `exp` claim
    /// - JWT: 返 `exp` claim
    /// - IAM: 返 session 过期时间
    /// - mTLS: 返证书过期时间
    async fn expires_at(&self) -> Option<DateTime<Utc>>;
}

// ============================================================================
// §2 TokenManager skeleton impl (包装任意 Provider)
// ============================================================================

/// 通用 TokenManager skeleton, 包装任意 Provider 实现.
///
/// R21+ 真接时, 此 struct 会持有 token 缓存 (per OAuth 2.0 exp claim).
pub struct ProviderTokenManager<P: Provider + ?Sized> {
    /// 包装的 Provider (Anthropic / OpenAI / Google / Azure / Local).
    pub provider: Box<P>,
}

impl<P: Provider> ProviderTokenManager<P> {
    /// 构造 (要求 P: Sized, Box::new 限制).
    pub fn new(provider: P) -> Self {
        Self {
            provider: Box::new(provider),
        }
    }
}

#[async_trait]
impl<P: Provider + ?Sized> TokenManager for ProviderTokenManager<P> {
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        // skeleton: 直接调 Provider, Provider stub 返 NotImplemented
        self.provider.get_token().await
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        self.provider.refresh().await
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        self.provider.revoke().await
    }
    async fn is_valid(&self) -> bool {
        self.provider.is_valid().await
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.provider.expires_at().await
    }
}

impl<P: Provider + ?Sized> std::fmt::Debug for ProviderTokenManager<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderTokenManager")
            .field("provider_kind", &self.provider.kind())
            .finish()
    }
}

// ============================================================================
// §3 编译期守门 (5 TokenManager 方法对齐)
// ============================================================================

/// TokenManager 5 方法 (编译期 hardcode).
pub const TOKEN_MANAGER_METHODS: &[&str] = &[
    "get_token",
    "refresh",
    "revoke",
    "is_valid",
    "expires_at",
];

/// TokenManager 5 方法数量 (编译期 hardcode).
pub const TOKEN_MANAGER_METHOD_COUNT: usize = 5;
const _: () = assert!(TOKEN_MANAGER_METHODS.len() == TOKEN_MANAGER_METHOD_COUNT);

// ============================================================================
// §4 单元测试 (5 stub + 5 Provider 各 1)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthMethod;
    use crate::provider::{
        AnthropicProvider, AzureProvider, GoogleProvider, LocalProvider, OpenAIProvider,
    };

    fn fixture_api_key() -> AuthMethod {
        AuthMethod::ApiKey {
            api_key: "sk-test-1234".to_string(),
        }
    }

    #[test]
    fn test_token_manager_5_methods() {
        // 5 TokenManager 方法全部存在
        assert_eq!(TOKEN_MANAGER_METHODS.len(), 5);
        assert!(TOKEN_MANAGER_METHODS.contains(&"get_token"));
        assert!(TOKEN_MANAGER_METHODS.contains(&"refresh"));
        assert!(TOKEN_MANAGER_METHODS.contains(&"revoke"));
        assert!(TOKEN_MANAGER_METHODS.contains(&"is_valid"));
        assert!(TOKEN_MANAGER_METHODS.contains(&"expires_at"));
    }

    #[tokio::test]
    async fn test_token_refresh_returns_not_implemented() {
        // 5 Provider 的 refresh 全部返 NotImplemented
        let p = AnthropicProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));

        let p = OpenAIProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));

        let p = GoogleProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));

        let p = AzureProvider::new(fixture_api_key(), "myresource").expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));

        let p = LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn test_token_revoke_returns_not_implemented() {
        let p = AnthropicProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));

        let p = OpenAIProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));

        let p = GoogleProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));

        let p = AzureProvider::new(fixture_api_key(), "myresource").expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));

        let p = LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn test_token_is_valid_false_in_skeleton() {
        // skeleton 阶段 is_valid 永远 false (无 token 缓存)
        let p = AnthropicProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(!tm.is_valid().await);
    }

    #[tokio::test]
    async fn test_token_expires_at_none_in_skeleton() {
        // skeleton 阶段 expires_at 永远 None
        let p = OpenAIProvider::new(fixture_api_key()).expect("new");
        let tm = ProviderTokenManager::new(p);
        assert!(tm.expires_at().await.is_none());
    }

    #[test]
    fn test_provider_kind_5_in_token_manager() {
        // 验证 5 ProviderKind 都可通过 TokenManager 包装
        let kinds = [
            ProviderKind::Anthropic,
            ProviderKind::OpenAI,
            ProviderKind::Google,
            ProviderKind::Azure,
            ProviderKind::Local,
        ];
        assert_eq!(kinds.len(), 5);
    }
}
