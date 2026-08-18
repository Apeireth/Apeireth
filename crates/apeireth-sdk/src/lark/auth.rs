//! # Lark 鉴权 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书开放平台 5 鉴权方式 (per v0.9.21 商业版):
//! 1. **App ID** — 应用唯一标识 (e.g. `cli_a1b2c3d4e5f6g7h8`)
//! 2. **App Secret** — 应用密钥 (e.g. 32 char random, 走 keyring)
//! 3. **tenant_access_token** — 应用级 access token, TTL 2h, 走 `/auth/v3/tenant_access_token/internal`
//! 4. **user_access_token** — 用户级 access token, 走 OAuth `code` → `user_access_token`
//! 5. **webhook_token** — 事件订阅校验 token (per `im/v1/event` 回调)
//!
//! **P0 安全铁律** (主人 19:50 拍板): App ID + App Secret + 3 token 0 明文存盘.
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
//! **6 K-1 强校验** (per task spec): app_id / app_secret / chat_id / open_id / email / mobile.
//! chat_id / open_id / email / mobile 在 error.rs 校验, app_id / app_secret 在本模块校验.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;

// ============================================================================
// §1 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// 平台名 (keyring "service" 字段 / protocol 平台标识).
///
/// 跟 livekit / sandbox PLATFORM_NAME 1:1, 锁 "apeireth" 避免跟其他 app 冲突.
pub const PLATFORM_NAME: &str = "apeireth";

/// Provider 名 (keyring "account" 字段).
///
/// 1:1 翻译 v0.9.21 商业版 `Lark.Client.config.serviceName = 'lark'`.
pub const PROVIDER_NAME: &str = "lark";

/// Lark SDK schema 版本 (1:1 翻译 @larksuiteoapi/lark-sdk v0.9.21).
pub const LARK_SCHEMA_VERSION: &str = "1";

/// 默认 Lark 服务器 URL (per 飞书 Open Platform 官方, https:// 强制).
///
/// 真实部署时用户应改成自己的飞书自建应用 URL (e.g. `https://open.feishu.cn`).
pub const DEFAULT_LARK_API_BASE: &str = "https://open.feishu.cn/open-apis";

/// 默认 tenant_access_token TTL (2h = 7200s, per 飞书 Open Platform 文档).
pub const DEFAULT_TENANT_TOKEN_TTL_SECONDS: u64 = 7200;

/// 默认 user_access_token TTL (2h = 7200s, per 飞书 Open Platform OAuth 文档).
pub const DEFAULT_USER_TOKEN_TTL_SECONDS: u64 = 7200;

/// Token 最大 TTL (24h, per 飞书 Open Platform 上限, 防长占).
pub const MAX_TOKEN_TTL_SECONDS: u64 = 86_400;

/// App ID 最小长度 (cli_ + 8 char = 12, per 飞书规范).
pub const MIN_APP_ID_LENGTH: usize = 12;

/// App Secret 最小长度 (per 飞书规范, 16 char).
pub const MIN_APP_SECRET_LENGTH: usize = 16;

/// App Secret 典型长度 (32 char, per 飞书默认).
pub const TYPICAL_APP_SECRET_LENGTH: usize = 32;

// ============================================================================
// §2 AppIdHolder (per P0 安全铁律 + apeireth-keyring 模式)
// ============================================================================

/// App ID 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (跟 livekit / sandbox 1:1 对齐). R21 续真接时
/// 改成 `apeireth_keyring::SecretBytes` 或 `secrecy::SecretString`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIdHolder {
    /// App ID (从 keyring get, **绝不存明文**)
    app_id: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl AppIdHolder {
    /// 创建空 holder.
    pub fn empty() -> Self {
        Self {
            app_id: None,
            loaded_from_keyring: false,
        }
    }

    /// 从 keyring 加载 App ID.
    ///
    /// **R21 续真接时** 调 `apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "lark-app-id")`.
    /// 当前 skeleton 返 None (per 0 假装已调通 keyring).
    pub fn from_keyring(_account: &str) -> Self {
        // ⏳ R20 阶段 4 skeleton: 不真接 keyring, 仅 holder
        // R21 续真接: apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "lark-app-id")
        Self::empty()
    }

    /// 设置 App ID (per K-1 #1 强校验).
    pub fn set(&mut self, app_id: String) -> Result<(), LarkError> {
        LarkError::validate_app_id(&app_id)?;
        self.app_id = Some(app_id);
        self.loaded_from_keyring = false;
        Ok(())
    }

    /// 读 App ID (cloned, 不暴露 &str 防止意外日志).
    pub fn get(&self) -> Option<String> {
        self.app_id.clone()
    }

    /// 检查是否已设置.
    pub fn is_set(&self) -> bool {
        self.app_id.is_some()
    }

    /// 是否从 keyring 加载.
    pub fn loaded_from_keyring(&self) -> bool {
        self.loaded_from_keyring
    }

    /// 清空.
    pub fn clear(&mut self) {
        self.app_id = None;
        self.loaded_from_keyring = false;
    }
}

impl Default for AppIdHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §3 AppSecretHolder (per P0 安全铁律)
// ============================================================================

/// App Secret 持有者 (per P0 安全铁律 + apeireth-keyring 模式).
///
/// **当前 skeleton 用 String 包装** (跟 AppIdHolder 同模式). R21 续真接时改成 SecretString.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSecretHolder {
    /// App Secret (从 keyring get, **绝不存明文**)
    app_secret: Option<String>,
    /// 是否已从 keyring 加载
    loaded_from_keyring: bool,
}

impl AppSecretHolder {
    /// 创建空 holder.
    pub fn empty() -> Self {
        Self {
            app_secret: None,
            loaded_from_keyring: false,
        }
    }

    /// 从 keyring 加载 App Secret.
    ///
    /// **R21 续真接时** 调 `apeireth_keyring::KeyringStore::get(PLATFORM_NAME, "lark-app-secret")`.
    pub fn from_keyring(_account: &str) -> Self {
        Self::empty()
    }

    /// 设置 App Secret (per K-1 #2 强校验).
    pub fn set(&mut self, app_secret: String) -> Result<(), LarkError> {
        LarkError::validate_app_secret(&app_secret)?;
        self.app_secret = Some(app_secret);
        self.loaded_from_keyring = false;
        Ok(())
    }

    /// 读 App Secret (cloned, 不暴露 &str).
    pub fn get(&self) -> Option<String> {
        self.app_secret.clone()
    }

    /// 检查是否已设置.
    pub fn is_set(&self) -> bool {
        self.app_secret.is_some()
    }

    /// 是否从 keyring 加载.
    pub fn loaded_from_keyring(&self) -> bool {
        self.loaded_from_keyring
    }

    /// 清空.
    pub fn clear(&mut self) {
        self.app_secret = None;
        self.loaded_from_keyring = false;
    }
}

impl Default for AppSecretHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §4 TenantAccessToken (per 飞书 Open Platform /auth/v3/tenant_access_token/internal)
// ============================================================================

/// tenant_access_token (per 飞书 Open Platform 文档).
///
/// 飞书 server 验证时要求 client 发 `Authorization: Bearer <tenant_access_token>`,
/// 包含:
/// - `app_id` + `app_secret` POST 到 `/auth/v3/tenant_access_token/internal`
/// - 响应: `{ "code": 0, "msg": "ok", "tenant_access_token": "t-xxx", "expire": 7200 }`
///
/// **当前 skeleton 不真调 API** (per R20 阶段 4 估补, R21 续真接 `reqwest` crate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantAccessToken {
    /// App ID (per token 来源标识)
    pub app_id: String,
    /// Token 值 (per `tenant_access_token` 字段, 走 keyring 不明文)
    pub token: String,
    /// 过期时间戳 (秒, UNIX_EPOCH 起, per `expire` 字段)
    pub expire_at_secs: u64,
    /// 创建时间戳 (秒, UNIX_EPOCH 起, 用于判断是否需要刷新)
    pub created_at_secs: u64,
}

impl TenantAccessToken {
    /// 创建新 tenant_access token (STUB 模式不真调飞书 API).
    pub fn new(app_id: String, token: String, ttl_seconds: u64) -> Result<Self, LarkError> {
        LarkError::validate_app_id(&app_id)?;
        if token.is_empty() {
            return Err(LarkError::TokenExpired);
        }
        if ttl_seconds == 0 || ttl_seconds > MAX_TOKEN_TTL_SECONDS {
            return Err(LarkError::Other(format!(
                "invalid ttl: {ttl_seconds} (1..=MAX_TOKEN_TTL_SECONDS={MAX_TOKEN_TTL_SECONDS})"
            )));
        }
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            app_id,
            token,
            expire_at_secs: now_secs + ttl_seconds,
            created_at_secs: now_secs,
        })
    }

    /// 默认 TTL 创建.
    pub fn with_default_ttl(app_id: String, token: String) -> Result<Self, LarkError> {
        Self::new(app_id, token, DEFAULT_TENANT_TOKEN_TTL_SECONDS)
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
// §5 UserAccessToken (per 飞书 OAuth 流程)
// ============================================================================

/// user_access_token (per 飞书 OAuth 文档).
///
/// 跟 tenant_access_token 类似, 但:
/// - TTL 更短 (通常 2h, refresh_token 30d)
/// - 携带用户身份 (open_id / union_id / user_id)
/// - 走 OAuth 流程 (`code` → `user_access_token`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessToken {
    /// App ID (per token 来源标识)
    pub app_id: String,
    /// Token 值 (per `access_token` 字段, 走 keyring 不明文)
    pub access_token: String,
    /// refresh_token (per OAuth, 30d, 走 keyring 不明文)
    pub refresh_token: String,
    /// 用户 Open ID (per 飞书 Open Platform 响应)
    pub open_id: String,
    /// 过期时间戳 (秒, UNIX_EPOCH 起, per `expires_in` 字段)
    pub expire_at_secs: u64,
    /// 创建时间戳 (秒, UNIX_EPOCH 起)
    pub created_at_secs: u64,
}

impl UserAccessToken {
    /// 创建新 user_access token (STUB 模式不真调飞书 OAuth).
    pub fn new(
        app_id: String,
        access_token: String,
        refresh_token: String,
        open_id: String,
        ttl_seconds: u64,
    ) -> Result<Self, LarkError> {
        LarkError::validate_app_id(&app_id)?;
        LarkError::validate_open_id(&open_id)?;
        if access_token.is_empty() {
            return Err(LarkError::TokenExpired);
        }
        if ttl_seconds == 0 || ttl_seconds > MAX_TOKEN_TTL_SECONDS {
            return Err(LarkError::Other(format!(
                "invalid ttl: {ttl_seconds} (1..=MAX_TOKEN_TTL_SECONDS={MAX_TOKEN_TTL_SECONDS})"
            )));
        }
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            app_id,
            access_token,
            refresh_token,
            open_id,
            expire_at_secs: now_secs + ttl_seconds,
            created_at_secs: now_secs,
        })
    }

    /// 是否已过期.
    pub fn is_expired(&self) -> bool {
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now_secs >= self.expire_at_secs
    }
}

// ============================================================================
// §6 WebhookToken (per 飞书事件订阅 URL 校验)
// ============================================================================

/// Webhook verification token (per 飞书事件订阅 URL 校验).
///
/// 飞书 server 在配置事件订阅 URL 时, 发送 `url_verification` 事件包含 `challenge` 字段,
/// 客户端必须原样返回 `challenge`. 配置完成后, 所有回调都带 `encrypt` + `token` 字段,
/// 客户端需用 `encrypt_key` 解密 + 校验 `token` 一致.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookToken {
    /// Verification token (per 飞书事件订阅配置, 走 keyring 不明文)
    pub token: String,
    /// Encrypt key (per 飞书事件加密, 走 keyring 不明文)
    pub encrypt_key: String,
}

impl WebhookToken {
    /// 创建新 webhook token (STUB 模式不真调飞书 API).
    pub fn new(token: String, encrypt_key: String) -> Result<Self, LarkError> {
        if token.is_empty() {
            return Err(LarkError::Other("webhook token is empty".to_string()));
        }
        if encrypt_key.is_empty() {
            return Err(LarkError::Other("webhook encrypt_key is empty".to_string()));
        }
        Ok(Self { token, encrypt_key })
    }

    /// 验证 token 一致 (per 飞书回调校验, 简单字符串比较).
    pub fn verify(&self, incoming_token: &str) -> bool {
        self.token == incoming_token
    }
}

// ============================================================================
// §7 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 编译期 hardcode ----

    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "lark");
        assert_eq!(LARK_SCHEMA_VERSION, "1");
        assert!(DEFAULT_LARK_API_BASE.starts_with("https://"));
        assert_eq!(DEFAULT_TENANT_TOKEN_TTL_SECONDS, 7200);
        assert_eq!(DEFAULT_USER_TOKEN_TTL_SECONDS, 7200);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
        assert_eq!(MIN_APP_ID_LENGTH, 12);
        assert_eq!(MIN_APP_SECRET_LENGTH, 16);
        assert_eq!(TYPICAL_APP_SECRET_LENGTH, 32);
    }

    // ---- §2 AppIdHolder ----

    #[test]
    fn k1_app_id_holder_empty() {
        let holder = AppIdHolder::empty();
        assert!(!holder.is_set());
        assert!(holder.get().is_none());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_app_id_holder_set_valid() {
        let mut holder = AppIdHolder::empty();
        holder
            .set("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id must succeed");
        assert!(holder.is_set());
        assert_eq!(holder.get().as_deref(), Some("cli_a1b2c3d4e5f6"));
    }

    #[test]
    fn k1_app_id_holder_set_rejects_empty() {
        let mut holder = AppIdHolder::empty();
        let result = holder.set(String::new());
        assert!(matches!(result, Err(LarkError::AppIdMissing)));
    }

    #[test]
    fn k1_app_id_holder_set_rejects_invalid_prefix() {
        let mut holder = AppIdHolder::empty();
        let result = holder.set("app_a1b2c3d4".to_string());
        assert!(matches!(result, Err(LarkError::AppIdInvalid(_))));
    }

    #[test]
    fn k1_app_id_holder_clear() {
        let mut holder = AppIdHolder::empty();
        holder
            .set("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id must succeed");
        assert!(holder.is_set());
        holder.clear();
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    #[test]
    fn k1_app_id_holder_from_keyring_returns_empty() {
        let holder = AppIdHolder::from_keyring("lark-app-id");
        assert!(!holder.is_set());
        assert!(!holder.loaded_from_keyring());
    }

    // ---- §3 AppSecretHolder ----

    #[test]
    fn k1_app_secret_holder_empty() {
        let holder = AppSecretHolder::empty();
        assert!(!holder.is_set());
        assert!(holder.get().is_none());
    }

    #[test]
    fn k1_app_secret_holder_set_valid() {
        let mut holder = AppSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid app secret must succeed");
        assert!(holder.is_set());
    }

    #[test]
    fn k1_app_secret_holder_set_rejects_empty() {
        let mut holder = AppSecretHolder::empty();
        let result = holder.set(String::new());
        assert!(matches!(result, Err(LarkError::AppSecretMissing)));
    }

    #[test]
    fn k1_app_secret_holder_set_rejects_too_short() {
        let mut holder = AppSecretHolder::empty();
        let result = holder.set("short".to_string());
        assert!(matches!(result, Err(LarkError::AppSecretInvalid(5))));
    }

    // ---- §4 TenantAccessToken ----

    #[test]
    fn tenant_token_creation_valid() {
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123def456".to_string(),
            7200,
        )
        .expect("valid tenant token must succeed");
        assert_eq!(token.app_id, "cli_a1b2c3d4e5f6");
        assert_eq!(token.token, "t-abc123def456");
        assert!(token.expire_at_secs > token.created_at_secs);
        assert!(!token.is_expired());
    }

    #[test]
    fn tenant_token_rejects_empty_token() {
        let result = TenantAccessToken::new("cli_a1b2c3d4e5f6".to_string(), String::new(), 7200);
        assert!(matches!(result, Err(LarkError::TokenExpired)));
    }

    #[test]
    fn tenant_token_rejects_invalid_app_id() {
        let result =
            TenantAccessToken::new("invalid".to_string(), "t-abc123def456".to_string(), 7200);
        assert!(matches!(result, Err(LarkError::AppIdInvalid(_))));
    }

    #[test]
    fn tenant_token_rejects_invalid_ttl() {
        let result = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123def456".to_string(),
            0,
        );
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn tenant_token_with_default_ttl() {
        let token = TenantAccessToken::with_default_ttl(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123def456".to_string(),
        )
        .expect("valid");
        // TTL 应 ≈ 7200s (差 < 5s 允许)
        let ttl = token.expire_at_secs - token.created_at_secs;
        assert!(ttl >= 7195 && ttl <= 7200);
    }

    // ---- §5 UserAccessToken ----

    #[test]
    fn user_token_creation_valid() {
        let token = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc123".to_string(),
            "ur-xyz789".to_string(),
            "ou_user1234567890abcdef".to_string(),
            7200,
        )
        .expect("valid user token must succeed");
        assert_eq!(token.open_id, "ou_user1234567890abcdef");
        assert!(!token.is_expired());
    }

    #[test]
    fn user_token_rejects_invalid_open_id() {
        let result = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc123".to_string(),
            "ur-xyz789".to_string(),
            "cli_invalid".to_string(),
            7200,
        );
        assert!(matches!(result, Err(LarkError::OpenIdInvalid(_))));
    }

    // ---- §6 WebhookToken ----

    #[test]
    fn webhook_token_creation_valid() {
        let wh = WebhookToken::new(
            "verify_token_xxx".to_string(),
            "encrypt_key_xxx".to_string(),
        )
        .expect("valid");
        assert!(wh.verify("verify_token_xxx"));
        assert!(!wh.verify("wrong_token"));
    }

    #[test]
    fn webhook_token_rejects_empty() {
        let result = WebhookToken::new(String::new(), "encrypt_key".to_string());
        assert!(matches!(result, Err(LarkError::Other(_))));
        let result = WebhookToken::new("token".to_string(), String::new());
        assert!(matches!(result, Err(LarkError::Other(_))));
    }
}
