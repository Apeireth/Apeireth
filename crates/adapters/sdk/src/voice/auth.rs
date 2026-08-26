//! # Voice 鉴权 (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! Voice SDK 鉴权 (per v0.9.21 商业版):
//! 1. **API Key** — `@anthropic-ai/voice` 主鉴权 (e.g. `sk-ant-voice-...`)
//! 2. **Access Token** — OAuth 颁发的 bearer token (R21 续真接时估补)
//!
//! **P0 安全铁律** (主人 19:50 拍板): API Key 0 明文存盘.
//! - Windows: 走 Credential Manager
//! - macOS:   走 Keychain
//! - Linux:   走 Secret Service (libsecret)
//! - BSD:     走 BSD Keychain
//!
//! **fallback** (per OWASP 2023 + apeireth-keyring §2.4.1):
//! - AES-256-GCM 加密
//! - PBKDF2 600_000 迭代派生 key
//! - 走 `apeireth_keyring::KeyringStore::set` / `get`
//!
//! **6 K-1 强校验** (per task spec): api_key / audio_format / sample_rate / bit_depth /
//! channels / language. api_key 在本模块校验, 其他 5 个在 error.rs 校验.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// 平台名 (keyring "service" 字段 / protocol 平台标识).
///
/// 跟 livekit / sandbox / lark PLATFORM_NAME 1:1, 锁 "apeireth" 避免跟其他 app 冲突.
pub const PLATFORM_NAME: &str = "apeireth";

/// Provider 名 (keyring "account" 字段).
///
/// 1:1 翻译 v0.9.21 商业版 `@anthropic-ai/voice` config.
pub const PROVIDER_NAME: &str = "anthropic-voice";

/// Voice SDK schema 版本 (1:1 翻译 @anthropic-ai/voice v0.9.21).
pub const VOICE_SCHEMA_VERSION: &str = "1";

/// 默认 Voice API 基础 URL (per v0.9.21 商业版, https:// 强制).
///
/// 真实部署时用户应改成自己的 Anthropic Voice endpoint (e.g. 自建代理).
pub const DEFAULT_VOICE_API_BASE: &str = "https://api.anthropic.com/v1/voice";

/// 默认 access token TTL (1h = 3600s, per Anthropic API 文档).
pub const DEFAULT_TOKEN_TTL_SECONDS: u64 = 3600;

/// Token 最大 TTL (24h, per Anthropic API 上限, 防长占).
pub const MAX_TOKEN_TTL_SECONDS: u64 = 86_400;

/// API Key 最小长度 (per K-1 #1 强校验, 16 char).
pub const MIN_API_KEY_LENGTH: usize = 16;

/// API Key 典型长度 (32 char, per Anthropic voice 规范).
pub const TYPICAL_API_KEY_LENGTH: usize = 32;

// ============================================================================
// §2 ApiKeyHolder (per P0 安全铁律 + apeireth-keyring 模式)
// ============================================================================

/// API Key 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (跟 livekit / sandbox / lark 1:1 对齐). R21 续真接时
/// 改成 `apeireth_keyring::SecretBytes` 或 `secrecy::SecretString`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyHolder {
    /// API Key (从 keyring get, **绝不存明文**)
    api_key: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl ApiKeyHolder {
    /// 创建空 holder.
    pub fn empty() -> Self {
        Self {
            api_key: None,
            loaded_from_keyring: false,
        }
    }

    /// 从 keyring 加载 API Key.
    ///
    /// **R21 续真接时** 调 `apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "anthropic-voice-api-key")`.
    /// 当前 skeleton 返 None (per 0 假装已调通 keyring).
    pub fn from_keyring(_account: &str) -> Self {
        // ⏳ R20 阶段 4 skeleton: 不真接 keyring, 仅 holder
        // R21 续真接: apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "anthropic-voice-api-key")
        Self::empty()
    }

    /// 设置 API Key (per K-1 #1 强校验).
    pub fn set(&mut self, api_key: String) -> Result<(), VoiceError> {
        VoiceError::validate_api_key(&api_key)?;
        self.api_key = Some(api_key);
        self.loaded_from_keyring = false;
        Ok(())
    }

    /// 读 API Key (cloned, 不暴露 &str 防止意外日志).
    pub fn get(&self) -> Option<String> {
        self.api_key.clone()
    }

    /// 检查是否已设置.
    pub fn is_set(&self) -> bool {
        self.api_key.is_some()
    }

    /// 是否从 keyring 加载.
    pub fn loaded_from_keyring(&self) -> bool {
        self.loaded_from_keyring
    }

    /// 清空.
    pub fn clear(&mut self) {
        self.api_key = None;
        self.loaded_from_keyring = false;
    }
}

impl Default for ApiKeyHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §3 AccessToken (per Anthropic Voice API OAuth 流程, R21 续真接估补)
// ============================================================================

/// access token (per Anthropic Voice API 文档).
///
/// Anthropic server 验证时要求 client 发 `Authorization: Bearer <access_token>`, 包含:
/// - `api_key` POST 到 `/v1/oauth/token` (估)
/// - 响应: `{ "access_token": "eyJ...", "expires_in": 3600, "token_type": "Bearer" }`
///
/// **当前 skeleton 不真调 API** (per R20 阶段 4 估补, R21 续真接 `reqwest` crate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// API Key 标识 (per token 来源标识, 不暴露原值)
    pub api_key_id: String,
    /// Token 值 (per `access_token` 字段, 走 keyring 不明文)
    pub token: String,
    /// 过期时间戳 (秒, UNIX_EPOCH 起, per `expires_in` 字段)
    pub expire_at_secs: u64,
    /// 创建时间戳 (秒, UNIX_EPOCH 起, 用于判断是否需要刷新)
    pub created_at_secs: u64,
}

impl AccessToken {
    /// 创建新 access token (STUB 模式不真调 Anthropic Voice API).
    pub fn new(api_key_id: String, token: String, ttl_seconds: u64) -> Result<Self, VoiceError> {
        if api_key_id.is_empty() {
            return Err(VoiceError::ApiKeyMissing);
        }
        if token.is_empty() {
            return Err(VoiceError::TokenExpired);
        }
        if ttl_seconds == 0 || ttl_seconds > MAX_TOKEN_TTL_SECONDS {
            return Err(VoiceError::Other(format!(
                "invalid ttl: {ttl_seconds} (1..=MAX_TOKEN_TTL_SECONDS={MAX_TOKEN_TTL_SECONDS})"
            )));
        }
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            api_key_id,
            token,
            expire_at_secs: now_secs + ttl_seconds,
            created_at_secs: now_secs,
        })
    }

    /// 默认 TTL 创建.
    pub fn with_default_ttl(api_key_id: String, token: String) -> Result<Self, VoiceError> {
        Self::new(api_key_id, token, DEFAULT_TOKEN_TTL_SECONDS)
    }

    /// 是否已过期.
    pub fn is_expired(&self) -> bool {
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now_secs >= self.expire_at_secs
    }

    /// 剩余 TTL (秒).
    pub fn remaining_ttl_secs(&self) -> u64 {
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.expire_at_secs.saturating_sub(now_secs)
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 编译期 hardcode ----

    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "anthropic-voice");
        assert_eq!(VOICE_SCHEMA_VERSION, "1");
        assert!(DEFAULT_VOICE_API_BASE.starts_with("https://"));
        assert_eq!(DEFAULT_TOKEN_TTL_SECONDS, 3600);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
        assert_eq!(MIN_API_KEY_LENGTH, 16);
        assert_eq!(TYPICAL_API_KEY_LENGTH, 32);
    }

    // ---- §2 ApiKeyHolder ----

    #[test]
    fn k1_api_key_holder_empty() {
        let holder = ApiKeyHolder::empty();
        assert!(!holder.is_set());
        assert!(holder.get().is_none());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_api_key_holder_set_valid() {
        let mut holder = ApiKeyHolder::empty();
        holder
            .set("sk-ant-voice-abcdef1234567890xyz".to_string())
            .expect("valid api key must succeed");
        assert!(holder.is_set());
        assert_eq!(
            holder.get().as_deref(),
            Some("sk-ant-voice-abcdef1234567890xyz")
        );
    }

    #[test]
    fn k1_api_key_holder_set_rejects_empty() {
        let mut holder = ApiKeyHolder::empty();
        let result = holder.set(String::new());
        assert!(matches!(result, Err(VoiceError::ApiKeyMissing)));
    }

    #[test]
    fn k1_api_key_holder_set_rejects_too_short() {
        let mut holder = ApiKeyHolder::empty();
        let result = holder.set("short".to_string());
        assert!(matches!(result, Err(VoiceError::ApiKeyInvalid(5))));
    }

    #[test]
    fn k1_api_key_holder_clear() {
        let mut holder = ApiKeyHolder::empty();
        holder
            .set("sk-ant-voice-abcdef1234567890xyz".to_string())
            .expect("valid api key must succeed");
        assert!(holder.is_set());
        holder.clear();
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_api_key_holder_from_keyring_returns_empty() {
        let holder = ApiKeyHolder::from_keyring("anthropic-voice-api-key");
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    // ---- §3 AccessToken ----

    #[test]
    fn access_token_creation_valid() {
        let token = AccessToken::new(
            "ak_abc123def456".to_string(),
            "eyJhbGciOiJIUzI1NiJ9.test".to_string(),
            3600,
        )
        .expect("valid access token must succeed");
        assert_eq!(token.api_key_id, "ak_abc123def456");
        assert!(token.expire_at_secs > token.created_at_secs);
        assert!(!token.is_expired());
    }

    #[test]
    fn access_token_rejects_empty_token() {
        let result = AccessToken::new("ak_abc123".to_string(), String::new(), 3600);
        assert!(matches!(result, Err(VoiceError::TokenExpired)));
    }

    #[test]
    fn access_token_rejects_empty_api_key_id() {
        let result = AccessToken::new(String::new(), "eyJ.test".to_string(), 3600);
        assert!(matches!(result, Err(VoiceError::ApiKeyMissing)));
    }

    #[test]
    fn access_token_rejects_invalid_ttl() {
        let result = AccessToken::new("ak_abc123".to_string(), "eyJ.test".to_string(), 0);
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn access_token_with_default_ttl() {
        let token = AccessToken::with_default_ttl("ak_abc123".to_string(), "eyJ.test".to_string())
            .expect("valid");
        // TTL 应 ≈ 3600s (差 < 5s 允许)
        let ttl = token.expire_at_secs - token.created_at_secs;
        assert!(ttl >= 3595 && ttl <= 3600);
    }

    #[test]
    fn access_token_remaining_ttl() {
        let token =
            AccessToken::new("ak_abc123".to_string(), "eyJ.test".to_string(), 3600).expect("valid");
        let remaining = token.remaining_ttl_secs();
        // 剩余 TTL 应 ≥ 3595s
        assert!(remaining >= 3595);
    }
}
