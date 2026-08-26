//! # 凭证 Provider trait + 5 实现
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main/chunks/credentials-*.js` 集成面. 5 Provider 各有独立实现,
//! 但当前阶段 (R20 阶段 6 skeleton) 全部返 `Err(CredentialsError::NotImplemented)`.
//!
//! ## 5 Provider
//!
//! | # | Provider | 鉴权 | 头 | 商业版源 |
//! |---|---------|------|----|---------|
//! | 1 | `Anthropic` | ApiKey | `x-api-key` | `@anthropic-ai/sdk` |
//! | 2 | `OpenAI` | ApiKey | `Authorization: Bearer` | `openai` SDK |
//! | 3 | `Google` | OAuth2 + ApiKey | `Authorization: Bearer` | `@google/generative-ai` |
//! | 4 | `Azure` | ApiKey | `api-key` | `@azure/openai` |
//! | 5 | `Local` | custom | `X-Apeireth-Token` | self-hosted |
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **5 Provider 编译期 hardcode**: 不可运行时增删
//! 2. **Provider trait 抽象**: 5 实现共享同一 trait, 5 stub 都返 `NotImplemented`
//! 3. **ProviderKind 枚举**: 用于 `CredentialsManager` 切换 provider
//! 4. **1:1 翻译 v0.9.21**: 每个 Provider 对应 1:1 的商业版 SDK / API
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 5 Provider 1:1 翻译 v0.9.21 商业版, 0 业务重设计
//! - **S-2 实事求是**: 5 Provider 够用 99% 场景, 不发明 `Bedrock` / `Vertex` 等花哨 provider
//! - **O-2 走在前人肩上**: 借鉴 Anthropic / OpenAI / Google 官方 SDK API 形状
//! - **O-3 干到底**: Provider trait + 5 impl + 5 fixture 测试
//! - **O-4 任何人都能接手**: 跟 keyring / i18n 同模式 (enum + trait + Display)
//! - **O-5 不假装**: 5 Provider stub 全部返 `NotImplemented` + `warn!`, 0 假装已对接

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::auth::AuthMethod;
use crate::error::{CredentialsError, CredentialsResult};

// ============================================================================
// §1 ProviderKind 枚举 (5 种, 编译期 hardcode)
// ============================================================================

/// 凭证 Provider 类型 (5 种, K-1 强校验).
///
/// 1:1 翻译 v0.9.21 商业版 5 个 credential provider. 顺序固定, 不可运行时增删.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// **Anthropic** (Claude API): `x-api-key: <key>` 头.
    Anthropic,
    /// **OpenAI** (GPT API): `Authorization: Bearer <key>` 头.
    OpenAI,
    /// **Google** (Gemini API): `Authorization: Bearer <oauth>` 头 (OAuth 2.0 + API key 双轨).
    Google,
    /// **Azure** (OpenAI on Azure): `api-key: <key>` 头 + endpoint URL.
    Azure,
    /// **Local** (self-hosted): `X-Apeireth-Token: <token>` 头.
    Local,
}

impl ProviderKind {
    /// Provider 字符串 (snake_case, 跟 serde rename_all 对齐).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Azure => "azure",
            Self::Local => "local",
        }
    }

    /// 5 Provider 名 (编译期 hardcode).
    pub const ALL_NAMES: [&'static str; 5] = ["anthropic", "openai", "google", "azure", "local"];

    /// 5 Provider (K-1 强校验: 编译期 hardcode).
    pub const COUNT: usize = 5;

    /// 5 Provider 列表 (K-1 强校验).
    #[must_use]
    pub fn all() -> [ProviderKind; 5] {
        [
            Self::Anthropic,
            Self::OpenAI,
            Self::Google,
            Self::Azure,
            Self::Local,
        ]
    }

    /// Provider 官方 API base URL (R21+ 真接时用).
    ///
    /// **skeleton 阶段不实际发请求, 仅作 1:1 翻译参考**.
    #[must_use]
    pub fn api_base_url(&self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAI => "https://api.openai.com",
            Self::Google => "https://generativelanguage.googleapis.com",
            Self::Azure => "https://{resource}.openai.azure.com",
            Self::Local => "http://localhost:8080",
        }
    }

    /// Provider 鉴权头名 (per 商业版 API 规范).
    #[must_use]
    pub fn auth_header_name(&self) -> &'static str {
        match self {
            Self::Anthropic => "x-api-key",
            Self::OpenAI => "Authorization",
            Self::Google => "Authorization",
            Self::Azure => "api-key",
            Self::Local => "X-Apeireth-Token",
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 Provider trait (5 stub impl 共享)
// ============================================================================

/// 凭证 Provider 通用 trait.
///
/// 1:1 翻译 v0.9.21 商业版 `out/main` 中 5 个 credential provider 的核心接口.
#[async_trait]
pub trait Provider: Send + Sync + std::fmt::Debug {
    /// Provider 类型.
    fn kind(&self) -> ProviderKind;

    /// **核心**: 获取凭证 (token / API key).
    ///
    /// R21+ 真接时, 5 Provider 各自实现:
    /// - Anthropic: 直接返 api_key
    /// - OpenAI: 直接返 api_key
    /// - Google: 调 OAuth2 token endpoint 拿 access_token
    /// - Azure: 直接返 api_key
    /// - Local: 直接返 custom token
    async fn get_token(&self) -> CredentialsResult<SecretString>;

    /// **核心**: 刷新凭证 (OAuth2 / IAM / mTLS 需要, API key 不需要).
    async fn refresh(&self) -> CredentialsResult<SecretString>;

    /// **核心**: 撤销凭证 (OAuth2 RFC 7009 / IAM / mTLS).
    async fn revoke(&self) -> CredentialsResult<()>;

    /// 凭证是否有效 (未过期).
    async fn is_valid(&self) -> bool;

    /// 凭证过期时间 (None = 永不过期, e.g. API key).
    async fn expires_at(&self) -> Option<DateTime<Utc>>;
}

/// 简单字符串包装, 防止误打印凭证 (Display 隐藏实际值, Debug 也隐藏).
///
/// 0 暴露凭证: 打印只显示 `<redacted, N bytes>`.
#[derive(Debug, Clone, Default)]
pub struct SecretString(String);

impl SecretString {
    /// 构造 (skeleton 阶段未用, 留 R21+ 真接时使用).
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 长度 (字节数).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 原始值 (仅在 R21+ 真接时使用, 调外部 SDK).
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<redacted, {} bytes>", self.0.len())
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// ============================================================================
// §3 5 Provider stub 实现 (skeleton: 全部 NotImplemented)
// ============================================================================

/// **Anthropic** provider stub (1:1 翻译 @anthropic-ai/sdk).
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    /// 鉴权方式 (5 种之一, 编译期 K-1 强校验).
    pub auth: AuthMethod,
}

impl AnthropicProvider {
    /// 构造.
    pub fn new(auth: AuthMethod) -> CredentialsResult<Self> {
        auth.validate()?;
        Ok(Self { auth })
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        warn!("anthropic_provider_get_token: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "anthropic_provider_get_token",
        ))
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        warn!("anthropic_provider_refresh: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "anthropic_provider_refresh",
        ))
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        warn!("anthropic_provider_revoke: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "anthropic_provider_revoke",
        ))
    }
    async fn is_valid(&self) -> bool {
        // skeleton 阶段永远 false
        false
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}

/// **OpenAI** provider stub (1:1 翻译 openai SDK).
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    /// 鉴权方式.
    pub auth: AuthMethod,
}

impl OpenAIProvider {
    /// 构造.
    pub fn new(auth: AuthMethod) -> CredentialsResult<Self> {
        auth.validate()?;
        Ok(Self { auth })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        warn!("openai_provider_get_token: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "openai_provider_get_token",
        ))
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        warn!("openai_provider_refresh: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "openai_provider_refresh",
        ))
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        warn!("openai_provider_revoke: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "openai_provider_revoke",
        ))
    }
    async fn is_valid(&self) -> bool {
        false
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}

/// **Google** (Gemini) provider stub (1:1 翻译 @google/generative-ai).
#[derive(Debug, Clone)]
pub struct GoogleProvider {
    /// 鉴权方式.
    pub auth: AuthMethod,
}

impl GoogleProvider {
    /// 构造.
    pub fn new(auth: AuthMethod) -> CredentialsResult<Self> {
        auth.validate()?;
        Ok(Self { auth })
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        warn!("google_provider_get_token: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "google_provider_get_token",
        ))
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        warn!("google_provider_refresh: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "google_provider_refresh",
        ))
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        warn!("google_provider_revoke: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "google_provider_revoke",
        ))
    }
    async fn is_valid(&self) -> bool {
        false
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}

/// **Azure** (OpenAI on Azure) provider stub (1:1 翻译 @azure/openai).
#[derive(Debug, Clone)]
pub struct AzureProvider {
    /// 鉴权方式.
    pub auth: AuthMethod,
    /// Azure resource name (e.g. `myresource`), 必非空.
    pub resource: String,
}

impl AzureProvider {
    /// 构造.
    pub fn new(auth: AuthMethod, resource: impl Into<String>) -> CredentialsResult<Self> {
        auth.validate()?;
        let resource = resource.into();
        if resource.trim().is_empty() {
            return Err(CredentialsError::Internal(
                "azure resource must not be empty".to_string(),
            ));
        }
        Ok(Self { auth, resource })
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Azure
    }
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        warn!("azure_provider_get_token: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "azure_provider_get_token",
        ))
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        warn!("azure_provider_refresh: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "azure_provider_refresh",
        ))
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        warn!("azure_provider_revoke: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "azure_provider_revoke",
        ))
    }
    async fn is_valid(&self) -> bool {
        false
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}

/// **Local** (self-hosted) provider stub.
#[derive(Debug, Clone)]
pub struct LocalProvider {
    /// 鉴权方式.
    pub auth: AuthMethod,
    /// Local 服务地址 (e.g. `http://localhost:8080`), 必非空.
    pub base_url: String,
}

impl LocalProvider {
    /// 构造.
    pub fn new(auth: AuthMethod, base_url: impl Into<String>) -> CredentialsResult<Self> {
        auth.validate()?;
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(CredentialsError::Internal(
                "local base_url must not be empty".to_string(),
            ));
        }
        Ok(Self { auth, base_url })
    }
}

#[async_trait]
impl Provider for LocalProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }
    async fn get_token(&self) -> CredentialsResult<SecretString> {
        warn!("local_provider_get_token: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "local_provider_get_token",
        ))
    }
    async fn refresh(&self) -> CredentialsResult<SecretString> {
        warn!("local_provider_refresh: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "local_provider_refresh",
        ))
    }
    async fn revoke(&self) -> CredentialsResult<()> {
        warn!("local_provider_revoke: R20 阶段 6 skeleton, R21+ 真接");
        Err(CredentialsError::NotImplemented(
            "local_provider_revoke",
        ))
    }
    async fn is_valid(&self) -> bool {
        false
    }
    async fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}

// ============================================================================
// §4 编译期守门 (5 Provider 对齐)
// ============================================================================

/// 5 Provider 编译期常量.
pub const PROVIDER_KINDS: &[ProviderKind] = &[
    ProviderKind::Anthropic,
    ProviderKind::OpenAI,
    ProviderKind::Google,
    ProviderKind::Azure,
    ProviderKind::Local,
];

const _: () = assert!(PROVIDER_KINDS.len() == ProviderKind::COUNT);
const _: () = assert!(ProviderKind::ALL_NAMES.len() == ProviderKind::COUNT);

/// 默认 HTTP 请求超时 (per RFC 7231 §6.5, 30s).
pub const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 30;

/// 转 chrono Duration 辅助.
#[must_use]
pub fn default_http_timeout() -> Duration {
    Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS)
}

// ============================================================================
// §5 单元测试 (5 Provider fixture + NotImplemented 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_api_key() -> AuthMethod {
        AuthMethod::ApiKey {
            api_key: "sk-test-1234".to_string(),
        }
    }

    #[test]
    fn test_provider_5_enums() {
        // 5 Provider 全部存在
        assert_eq!(ProviderKind::all().len(), 5);
        assert!(ProviderKind::all().contains(&ProviderKind::Anthropic));
        assert!(ProviderKind::all().contains(&ProviderKind::OpenAI));
        assert!(ProviderKind::all().contains(&ProviderKind::Google));
        assert!(ProviderKind::all().contains(&ProviderKind::Azure));
        assert!(ProviderKind::all().contains(&ProviderKind::Local));
    }

    #[test]
    fn test_provider_as_str_snake_case() {
        assert_eq!(ProviderKind::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderKind::OpenAI.as_str(), "openai");
        assert_eq!(ProviderKind::Google.as_str(), "google");
        assert_eq!(ProviderKind::Azure.as_str(), "azure");
        assert_eq!(ProviderKind::Local.as_str(), "local");
    }

    #[test]
    fn test_provider_auth_header_per_provider() {
        // 每个 Provider 鉴权头独立 (1:1 翻译商业版)
        assert_eq!(ProviderKind::Anthropic.auth_header_name(), "x-api-key");
        assert_eq!(ProviderKind::OpenAI.auth_header_name(), "Authorization");
        assert_eq!(ProviderKind::Google.auth_header_name(), "Authorization");
        assert_eq!(ProviderKind::Azure.auth_header_name(), "api-key");
        assert_eq!(ProviderKind::Local.auth_header_name(), "X-Apeireth-Token");
    }

    #[tokio::test]
    async fn test_anthropic_provider_not_implemented() {
        let p = AnthropicProvider::new(fixture_api_key()).expect("new");
        assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
        assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
        assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
        assert_eq!(p.kind(), ProviderKind::Anthropic);
        assert!(!p.is_valid().await);
        assert!(p.expires_at().await.is_none());
    }

    #[tokio::test]
    async fn test_openai_provider_not_implemented() {
        let p = OpenAIProvider::new(fixture_api_key()).expect("new");
        assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
        assert_eq!(p.kind(), ProviderKind::OpenAI);
    }

    #[tokio::test]
    async fn test_google_provider_not_implemented() {
        let p = GoogleProvider::new(fixture_api_key()).expect("new");
        assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
        assert_eq!(p.kind(), ProviderKind::Google);
    }

    #[tokio::test]
    async fn test_azure_provider_not_implemented() {
        let p = AzureProvider::new(fixture_api_key(), "myresource").expect("new");
        assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
        assert_eq!(p.kind(), ProviderKind::Azure);
    }

    #[tokio::test]
    async fn test_azure_provider_empty_resource_rejected() {
        let err = AzureProvider::new(fixture_api_key(), "").unwrap_err();
        assert!(matches!(err, CredentialsError::Internal(_)));
    }

    #[tokio::test]
    async fn test_local_provider_not_implemented() {
        let p = LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("new");
        assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
        assert_eq!(p.kind(), ProviderKind::Local);
    }

    #[tokio::test]
    async fn test_local_provider_empty_base_url_rejected() {
        let err = LocalProvider::new(fixture_api_key(), "").unwrap_err();
        assert!(matches!(err, CredentialsError::Internal(_)));
    }

    #[test]
    fn test_secret_string_redacts_on_display() {
        let s = SecretString::new("sk-very-secret-key");
        let displayed = format!("{s}");
        assert!(!displayed.contains("sk-very-secret-key"));
        assert!(displayed.contains("redacted"));
    }

    #[test]
    fn test_secret_string_expose_secret_returns_raw() {
        // expose_secret 只在 R21+ 真接时使用, 当前 skeleton 阶段仅测试访问
        let s = SecretString::new("sk-1234");
        assert_eq!(s.expose_secret(), "sk-1234");
        assert_eq!(s.len(), 7);
        assert!(!s.is_empty());
    }
}
