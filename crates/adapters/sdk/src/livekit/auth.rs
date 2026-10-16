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
//!   - K-1 #4: URL 必须 `wss://` 开头 (per LiveKit 协议 强制要求)

use serde::{Deserialize, Serialize};

use crate::livekit::error::LiveKitError;

// ============================================================================
// §1 API Key + Secret holders (per task spec §4 提到 `SecretString`, 但 workspace 无 secrecy, 用 String + 内存存)
// ============================================================================

/// API Key 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (task spec 提到 SecretString, 但 workspace
/// 无 secrecy crate, 改用 String, 与同类工程做法一致). R21 续真接时
/// 改成 `apeireth_keyring::SecretBytes` 或 `secrecy::SecretString`.
///
/// **M5 修复**: Debug 手写脱敏 — derive(Debug) 会让一次 `{:?}` / `dbg!` 把 API Key
/// 明文落进日志/错误面板. Serialise 保持 (wire 兼容), 只修 Debug 泄露面.
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiKeyHolder {
    /// API Key (从 keyring get, **绝不存明文**)
    api_key: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl std::fmt::Debug for ApiKeyHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyHolder")
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("loaded_from_keyring", &self.loaded_from_keyring)
            .finish()
    }
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
///
/// **M5 修复**: Debug 手写脱敏 (api_secret 是 HMAC 签名密钥, derive(Debug) 即泄露面).
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiSecretHolder {
    /// API Secret (从 keyring get, **绝不存明文**)
    api_secret: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl std::fmt::Debug for ApiSecretHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiSecretHolder")
            .field(
                "api_secret",
                &self.api_secret.as_ref().map(|_| "[redacted]"),
            )
            .field("loaded_from_keyring", &self.loaded_from_keyring)
            .finish()
    }
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

/// LiveKit access token (JWT, per RoomService 鉴权面).
///
/// 签发的 JWT 载荷包含:
/// - `iss`: API Key
/// - `sub`: room name
/// - `identity`: 参与者身份
/// - `iat` / `exp`: 签发 / 过期时间戳
/// - 签名: HS256 = HMAC-SHA256(API Secret, `header.payload`)
///
/// **M5 修复**: Debug 手写脱敏 (`api_key` 是 `iss` claim = 长期 API Key,
/// derive(Debug) 即泄露面).
#[derive(Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// API Key (per `iss` claim)
    pub api_key: String,
    /// Room Name (per `sub` claim)
    pub room_name: String,
    /// 参与者身份 (per `identity` claim)
    pub identity: String,
    /// TTL (秒, 默认 3600 = 1h)
    pub ttl_seconds: u64,
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field("api_key", &"[redacted]")
            .field("room_name", &self.room_name)
            .field("identity", &self.identity)
            .field("ttl_seconds", &self.ttl_seconds)
            .finish()
    }
}

impl AccessToken {
    /// 创建新 access token (载荷描述; 真实签名走 [`AccessToken::sign_hs256`]).
    pub fn new(api_key: String, room_name: String, identity: String) -> Result<Self, LiveKitError> {
        LiveKitError::validate_api_key(&api_key)?;
        LiveKitError::validate_room_name(&room_name)?;
        if identity.is_empty() {
            return Err(LiveKitError::InvalidArgument(
                "identity is empty".to_string(),
            ));
        }
        Ok(Self {
            api_key,
            room_name,
            identity,
            ttl_seconds: 3600,
        })
    }

    /// 设置 TTL (带上界 `MAX_TOKEN_TTL_SECONDS` = 24h, 防长占 / 永不过期 token).
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds.min(MAX_TOKEN_TTL_SECONDS);
        self
    }

    /// 用 API Secret 签发 HS256 JWT (`header.payload.signature`).
    ///
    /// - header: `{"alg":"HS256","typ":"JWT"}`
    /// - payload: `iss` / `sub` / `identity` / `iat` / `exp`
    /// - signature: HMAC-SHA256(API Secret, `header.payload`), base64url 无填充
    pub fn sign_hs256(
        &self,
        api_secret: &str,
        issued_at_unix_secs: u64,
    ) -> Result<String, LiveKitError> {
        LiveKitError::validate_api_secret(api_secret)?;
        let header = br#"{"alg":"HS256","typ":"JWT"}"#;
        let payload = serde_json::json!({
            "iss": self.api_key,
            "sub": self.room_name,
            "identity": self.identity,
            "iat": issued_at_unix_secs,
            "exp": issued_at_unix_secs.saturating_add(self.ttl_seconds),
        });
        let payload_bytes =
            serde_json::to_vec(&payload).map_err(|e| LiveKitError::Internal(e.to_string()))?;
        let signing_input = format!(
            "{}.{}",
            base64url_nopad(header),
            base64url_nopad(&payload_bytes)
        );
        let signature = hmac_sha256(api_secret.as_bytes(), signing_input.as_bytes());
        Ok(format!("{}.{}", signing_input, base64url_nopad(&signature)))
    }
}

/// 校验 HS256 JWT (签名 + 结构), 过期时间由调用方按 `exp` claim 判定.
pub fn verify_hs256(token: &str, api_secret: &str) -> Result<(), LiveKitError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(LiveKitError::Authentication(
            "token must have 3 segments".to_string(),
        ));
    }
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected = hmac_sha256(api_secret.as_bytes(), signing_input.as_bytes());
    let provided = base64url_decode(parts[2]).ok_or_else(|| {
        LiveKitError::Authentication("token signature is not base64url".to_string())
    })?;
    if provided.as_slice() != expected.as_slice() {
        return Err(LiveKitError::Authentication(
            "token signature mismatch".to_string(),
        ));
    }
    Ok(())
}

/// base64url 编码 (无填充, RFC 4648 §5 字母表).
fn base64url_nopad(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 0x3F] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 0x3F] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(triple >> 6) as usize & 0x3F] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[triple as usize & 0x3F] as char);
        }
    }
    out
}

/// base64url 解码 (无填充), 非法输入返 `None`.
fn base64url_decode(data: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4 + 3);
    for chunk in bytes.chunks(4) {
        let mut triple = 0u32;
        let mut got = 0u32;
        for (i, b) in chunk.iter().enumerate() {
            triple |= val(*b)? << (18 - 6 * i);
            got += 1;
        }
        out.push((triple >> 16) as u8);
        if got > 2 {
            out.push((triple >> 8) as u8);
        }
        if got > 3 {
            out.push(triple as u8);
        }
    }
    Some(out)
}

/// HMAC-SHA256 (RFC 2104, 基于 `sha2::Sha256` 实现; 已过 RFC 4231 向量测试).
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        key_block[..32].copy_from_slice(&hasher.finalize());
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let digest = outer.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

// ============================================================================
// §3 平台名 + 服务地址 + schema 版本 (per task spec §1 编译期 hardcode)
// ============================================================================

/// 平台名 (keyring "service" 字段 / protocol 平台标识).
///
/// 与同类工程 PLATFORM_NAME 惯例一致, 锁 "apeireth" 避免跟其他 app 冲突.
pub const PLATFORM_NAME: &str = "apeireth";

/// Provider 名 (keyring "account" 字段).
///
/// 对齐既有实现 livekit-client `serviceName = 'livekit'`.
pub const PROVIDER_NAME: &str = "livekit";

/// LiveKit SDK schema 版本 (语义对齐 LiveKit 协议).
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

    /// L 组: with_ttl 上界 clamp 到 MAX_TOKEN_TTL_SECONDS (24h), 防常量形同虚设.
    #[test]
    fn k1_access_token_with_ttl_clamps_to_max() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid access token must succeed")
        .with_ttl(MAX_TOKEN_TTL_SECONDS * 100);
        assert_eq!(token.ttl_seconds, MAX_TOKEN_TTL_SECONDS);
        // 边界值本身透传
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid access token must succeed")
        .with_ttl(MAX_TOKEN_TTL_SECONDS);
        assert_eq!(token.ttl_seconds, MAX_TOKEN_TTL_SECONDS);
    }

    /// M5: AccessToken Debug 脱敏 (api_key = iss claim = 长期秘密).
    #[test]
    fn k1_access_token_debug_is_redacted() {
        let token = AccessToken::new(
            "API12345678secret".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid access token must succeed");
        let dbg = format!("{token:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("API12345678secret"),
            "Debug 0 泄 api_key: {dbg}"
        );
        // 非秘密字段保留 (room_name / identity 可见)
        assert!(dbg.contains("my-room-1"), "room_name 应可见: {dbg}");
    }

    /// M5: ApiKeyHolder Debug 脱敏.
    #[test]
    fn k1_api_key_holder_debug_is_redacted() {
        let mut holder = ApiKeyHolder::empty();
        holder.set("APIsecretkey123456".to_string()).expect("valid");
        let dbg = format!("{holder:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("APIsecretkey123456"),
            "Debug 0 泄 api_key: {dbg}"
        );
    }

    /// M5: ApiSecretHolder Debug 脱敏.
    #[test]
    fn k1_api_secret_holder_debug_is_redacted() {
        let mut holder = ApiSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid API secret must succeed");
        let dbg = format!("{holder:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("abcdef1234567890abcdef1234567890"),
            "Debug 0 泄 api_secret: {dbg}"
        );
    }

    // ===== HS256 签名面 (真实签名 / 验签, 零占位) =====

    #[test]
    fn hmac_sha256_matches_rfc4231_vector() {
        // RFC 4231 测试用例 2: key="Jefe", data="what do ya want for nothing?"
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_handles_long_keys_by_hashing() {
        // key > 块长 64 字节: 先哈希再 HMAC (RFC 2104)
        let long_key = [0xABu8; 100];
        let mac = hmac_sha256(&long_key, b"data");
        assert_eq!(mac.len(), 32);
        let again = hmac_sha256(&long_key, b"data");
        assert_eq!(mac, again, "确定性");
    }

    #[test]
    fn base64url_known_vector_and_roundtrip() {
        assert_eq!(base64url_nopad(b"Man"), "TWFu");
        assert_eq!(base64url_nopad(b"Ma"), "TWE");
        assert_eq!(base64url_nopad(b"M"), "TQ");
        // url-safe 字母表: 输出不得含标准 base64 的 '+' / '/'
        let out = base64url_nopad(&[0xFB, 0xEF, 0xBE, 0xFF]);
        assert!(
            !out.contains('+') && !out.contains('/'),
            "must be url-safe: {out}"
        );
        for data in [
            b"" as &[u8],
            b"M",
            b"Ma",
            b"Man",
            b"Many hands make light work".as_ref(),
            &[0xFB, 0xEF, 0xBE, 0xFF],
        ] {
            let encoded = base64url_nopad(data);
            assert_eq!(base64url_decode(&encoded), Some(data.to_vec()));
        }
        assert_eq!(base64url_decode("!!invalid!!"), None);
    }

    #[test]
    fn sign_and_verify_hs256_roundtrip_with_claims() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid token")
        .with_ttl(7200);
        let secret = "abcdef1234567890abcdef1234567890";
        let signed = token.sign_hs256(secret, 1_700_000_000).expect("sign");
        verify_hs256(&signed, secret).expect("verify must pass");

        // 结构: 3 段, header 解出 HS256, payload 带全 claims
        let parts: Vec<&str> = signed.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: serde_json::Value =
            serde_json::from_slice(&base64url_decode(parts[0]).expect("decode")).expect("json");
        assert_eq!(header["alg"], "HS256");
        assert_eq!(header["typ"], "JWT");
        let payload: serde_json::Value =
            serde_json::from_slice(&base64url_decode(parts[1]).expect("decode")).expect("json");
        assert_eq!(payload["iss"], "API12345678");
        assert_eq!(payload["sub"], "my-room-1");
        assert_eq!(payload["identity"], "user-1");
        assert_eq!(payload["iat"], 1_700_000_000u64);
        assert_eq!(payload["exp"], 1_700_007_200u64);
    }

    #[test]
    fn verify_hs256_rejects_tampering_and_wrong_secret() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid token");
        let secret = "abcdef1234567890abcdef1234567890";
        let signed = token.sign_hs256(secret, 1_700_000_000).expect("sign");

        // 换密钥 → 认证失败
        let wrong = "0000000000000000000000000000000000";
        assert!(matches!(
            verify_hs256(&signed, wrong),
            Err(LiveKitError::Authentication(_))
        ));

        // 篡改 payload → 签名失配
        let parts: Vec<&str> = signed.split('.').collect();
        let tampered = format!("{}.{}.{}", parts[0], "eyJzdWIiOiJoYWNrZXIifQ", parts[2]);
        assert!(matches!(
            verify_hs256(&tampered, secret),
            Err(LiveKitError::Authentication(_))
        ));

        // 结构破坏 → 认证失败
        assert!(matches!(
            verify_hs256("only.two", secret),
            Err(LiveKitError::Authentication(_))
        ));
    }

    #[test]
    fn sign_hs256_requires_valid_secret() {
        let token = AccessToken::new(
            "API12345678".to_string(),
            "my-room-1".to_string(),
            "user-1".to_string(),
        )
        .expect("valid token");
        assert!(matches!(
            token.sign_hs256("short", 1_700_000_000),
            Err(LiveKitError::ApiSecretInvalid(_))
        ));
    }
}
