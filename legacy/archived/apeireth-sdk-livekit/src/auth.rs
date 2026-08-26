//! LiveKit API Key + Secret 鉴权 (per apeireth-keyring §2.4.1 + P0 安全铁律)
//!
//! **P0 安全铁律** (主人 19:50 拍板): API Key + Secret 0 明文存盘.
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
//! **4 K-1 强校验** (per task spec): API Key + Secret + Room Name + wss:// URL.
//!   - K-1 #1: API Key 格式 (空 / 错 / 真, per `LiveKitError::ApiKeyMissing` / `ApiKeyInvalid`)
//!   - K-1 #2: API Secret 格式 (空 / 错, per `LiveKitError::ApiSecretMissing` / `ApiSecretInvalid`)
//!   - K-1 #3: Room Name 1..=256 chars alphanumeric + `-` + `_`
//!   - K-1 #4: URL 必须 `wss://` 开头 (per livekit-client v0.9.21 强制要求)

use serde::{Deserialize, Serialize};

use crate::error::LiveKitError;

// ============================================================================
// §1 API Key + Secret holders (per task spec §4 提到 `SecretString`, 但 workspace 无 secrecy, 用 String + 内存存)
// ============================================================================

/// API Key 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (task spec 提到 SecretString, 但 workspace
/// 无 secrecy crate, 改用 String, 跟 gemini-cli / claude-code 1:1 对齐). R21 续真接时
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
    /// **R21 续真接时** 调 `apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "livekit-api-key")`.
    /// 当前 skeleton 返 None (per 0 假装已调通 keyring).
    pub fn from_keyring(_account: &str) -> Self {
        // ⏳ R20 阶段 4 skeleton: 不真接 keyring, 仅 holder
        // R21 续真接: apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "livekit-api-key")
        // .map(|opt| opt.map(|k| Self { api_key: Some(k), loaded_from_keyring: true }))
        // .unwrap_or_else(|_| Self::empty())
        Self::empty()
    }

    /// 设置 API Key (per task spec set_api_key).
    ///
    /// **当前 skeleton 仅存内存** (per 任务规范 0 明文存盘, 走 keyring 阶段 R21 续).
    pub fn set(&mut self, api_key: String) -> Result<(), LiveKitError> {
        LiveKitError::validate_api_key(&api_key)?;
        self.api_key = Some(api_key);
        self.loaded_from_keyring = false; // 内存存, 不是从 keyring 加载
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

    /// 清空 (per disconnect 工具).
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

/// API Secret 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (跟 ApiKeyHolder 同模式). R21 续真接时改成 SecretString.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSecretHolder {
    /// API Secret (从 keyring get, **绝不存明文**)
    api_secret: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl ApiSecretHolder {
    /// 创建空 holder.
    pub fn empty() -> Self {
        Self {
            api_secret: None,
            loaded_from_keyring: false,
        }
    }

    /// 从 keyring 加载 API Secret.
    ///
    /// **R21 续真接时** 调 `apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "livekit-api-secret")`.
    pub fn from_keyring(_account: &str) -> Self {
        Self::empty()
    }

    /// 设置 API Secret (per task spec set_api_secret).
    pub fn set(&mut self, api_secret: String) -> Result<(), LiveKitError> {
        LiveKitError::validate_api_secret(&api_secret)?;
        self.api_secret = Some(api_secret);
        self.loaded_from_keyring = false;
        Ok(())
    }

    /// 读 API Secret (cloned, 不暴露 &str).
    pub fn get(&self) -> Option<String> {
        self.api_secret.clone()
    }

    /// 检查是否已设置.
    pub fn is_set(&self) -> bool {
        self.api_secret.is_some()
    }

    /// 是否从 keyring 加载.
    pub fn loaded_from_keyring(&self) -> bool {
        self.loaded_from_keyring
    }

    /// 清空 (per disconnect 工具).
    pub fn clear(&mut self) {
        self.api_secret = None;
        self.loaded_from_keyring = false;
    }
}

impl Default for ApiSecretHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §2 LiveKit access token 生成 (per livekit-server JWT spec)
// ============================================================================

/// LiveKit access token (JWT, per livekit-server RoomService auth).
///
/// LiveKit 服务端验证时要求 client 发 JWT, 包含:
/// - `iss`: API Key
/// - `sub`: room name
/// - `exp`: expiration timestamp
/// - 签名: HMAC-SHA256(API Secret, header.payload)
///
/// **当前 skeleton 不真生成 JWT** (per R20 阶段 4 估补, R21 续真接 `jsonwebtoken` crate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// API Key (per `iss` claim)
    pub api_key: String,
    /// Room Name (per `sub` claim)
    pub room_name: String,
    /// 参与者身份 (per LiveKit `identity` claim, 默认 UUID)
    pub identity: String,
    /// TTL (秒, 默认 3600 = 1h, per LiveKit 默认)
    pub ttl_seconds: u64,
    /// 序列化为 JWT 字符串 (placeholder, R21 用 jsonwebtoken crate 真签)
    pub jwt_placeholder: String,
}

impl AccessToken {
    /// 创建新 access token (STUB 模式不真签 JWT).
    pub fn new(api_key: String, room_name: String, identity: String) -> Result<Self, LiveKitError> {
        LiveKitError::validate_api_key(&api_key)?;
        LiveKitError::validate_room_name(&room_name)?;
        if identity.is_empty() {
            return Err(LiveKitError::RoomNameEmpty); // 复用 empty error
        }
        let jwt_placeholder = format!("stub.jwt.{}", identity);
        Ok(Self {
            api_key,
            room_name,
            identity,
            ttl_seconds: 3600,
            jwt_placeholder,
        })
    }

    /// 设置 TTL.
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }
}

// ============================================================================
// §3 平台名 + 服务地址 + schema 版本 (per task spec §1 编译期 hardcode)
// ============================================================================

/// 平台名 (keyring "service" 字段 / protocol 平台标识).
///
/// 跟 gemini-cli PLATFORM_NAME 1:1, 锁 "apeireth" 避免跟其他 app 冲突.
pub const PLATFORM_NAME: &str = "apeireth";

/// Provider 名 (keyring "account" 字段).
///
/// 1:1 翻译 v0.9.21 livekit-client `serviceName = 'livekit'`.
pub const PROVIDER_NAME: &str = "livekit";

/// LiveKit SDK schema 版本 (1:1 翻译 livekit-client v0.9.21).
pub const LIVEKIT_SCHEMA_VERSION: &str = "1";

/// 默认 LiveKit 服务器 URL (per livekit-cloud 官方, wss:// 强制).
///
/// 真实部署时用户应改成自己的 LiveKit server URL, e.g. `wss://my-livekit.example.com`.
pub const DEFAULT_LIVEKIT_URL: &str = "wss://livekit.example.com";

/// 默认 access token TTL (1h, per livekit-server 默认).
pub const DEFAULT_TOKEN_TTL_SECONDS: u64 = 3600;

/// Token 最大 TTL (24h, per livekit-server 上限).
pub const MAX_TOKEN_TTL_SECONDS: u64 = 86_400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "livekit");
        assert_eq!(LIVEKIT_SCHEMA_VERSION, "1");
        assert!(DEFAULT_LIVEKIT_URL.starts_with("wss://"));
        assert_eq!(DEFAULT_TOKEN_TTL_SECONDS, 3600);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
    }

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
            .set("API12345678".to_string())
            .expect("valid API key must succeed");
        assert!(holder.is_set());
        assert_eq!(holder.get().as_deref(), Some("API12345678"));
    }

    #[test]
    fn k1_api_key_holder_set_rejects_empty() {
        let mut holder = ApiKeyHolder::empty();
        let result = holder.set(String::new());
        assert!(matches!(result, Err(LiveKitError::ApiKeyMissing)));
    }

    #[test]
    fn k1_api_key_holder_set_rejects_too_short() {
        let mut holder = ApiKeyHolder::empty();
        let result = holder.set("short".to_string());
        assert!(matches!(result, Err(LiveKitError::ApiKeyInvalid(_))));
    }

    #[test]
    fn k1_api_key_holder_clear() {
        let mut holder = ApiKeyHolder::empty();
        holder
            .set("API12345678".to_string())
            .expect("valid API key must succeed");
        assert!(holder.is_set());
        holder.clear();
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_api_key_holder_from_keyring_returns_empty() {
        // R20 阶段 4 skeleton: from_keyring 不真接, 返空
        let holder = ApiKeyHolder::from_keyring("livekit-api-key");
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_api_key_holder_default_is_empty() {
        let holder = ApiKeyHolder::default();
        assert!(!holder.is_set());
    }

    #[test]
    fn k1_api_secret_holder_empty() {
        let holder = ApiSecretHolder::empty();
        assert!(!holder.is_set());
        assert!(holder.get().is_none());
    }

    #[test]
    fn k1_api_secret_holder_set_valid() {
        let mut holder = ApiSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid API secret must succeed");
        assert!(holder.is_set());
    }

    #[test]
    fn k1_api_secret_holder_set_rejects_empty() {
        let mut holder = ApiSecretHolder::empty();
        let result = holder.set(String::new());
        assert!(matches!(result, Err(LiveKitError::ApiSecretMissing)));
    }

    #[test]
    fn k1_api_secret_holder_set_rejects_too_short() {
        let mut holder = ApiSecretHolder::empty();
        let result = holder.set("short".to_string());
        assert!(matches!(result, Err(LiveKitError::ApiSecretInvalid(_))));
    }

    #[test]
    fn k1_api_secret_holder_clear() {
        let mut holder = ApiSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid API secret must succeed");
        assert!(holder.is_set());
        holder.clear();
        assert!(!holder.is_set());
    }

    #[test]
    fn k1_access_token_creation_valid() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid access token must succeed");
        assert_eq!(token.api_key, "API12345678");
        assert_eq!(token.room_name, "my-room-1");
        assert_eq!(token.identity, "user-1");
        assert_eq!(token.ttl_seconds, 3600);
        assert!(token.jwt_placeholder.starts_with("stub.jwt."));
    }

    #[test]
    fn k1_access_token_creation_invalid_api_key() {
        let result = AccessToken::new(
            "short".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        );
        assert!(matches!(result, Err(LiveKitError::ApiKeyInvalid(_))));
    }

    #[test]
    fn k1_access_token_creation_invalid_room_name() {
        let result = AccessToken::new(
            "API12345678".to_string(),
            "".to_string(),
            "user-1".to_string(),
        );
        assert!(matches!(result, Err(LiveKitError::RoomNameEmpty)));
    }

    #[test]
    fn k1_access_token_with_ttl() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid access token must succeed")
        .with_ttl(7200);
        assert_eq!(token.ttl_seconds, 7200);
    }
}
