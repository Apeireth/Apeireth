//! # lark 鉴权 (凭证持有 + token 生命周期 + 持久化 token 缓存)
//!
//! 平台开放 API 的 5 个鉴权要素:
//! 1. **App ID** — 应用唯一标识 (`cli_` 前缀, K-1 #1 强校验)
//! 2. **App Secret** — 应用密钥 (≥ 16 字符, K-1 #2 强校验)
//! 3. **tenant_access_token** — 应用级访问令牌 (TTL 通常 2h,
//!    走 `/auth/v3/tenant_access_token/internal` 颁发)
//! 4. **user_access_token** — 用户级访问令牌 (授权码换发, 携带用户身份)
//! 5. **webhook_token** — 事件订阅校验 token + 加密密钥
//!
//! ## 安全铁律
//!
//! - App ID / App Secret / 3 类 token **不以明文落盘**; 调试输出一律脱敏
//!   (`[redacted]`), 任何 `{:?}` / 日志不会带出秘密值。
//! - token 持久化缓存 ([`TokenCache`]) 是唯一落盘的秘密面: 走
//!   `storage_atomic` 原子写 + 文件锁 + `0600` 属主可读写权限。
//! - 【显式不支持】`from_credential_store`: 系统凭据库 (操作系统凭据设施)
//!   的接入依赖部署环境, 本客户端不自带 —— 返回
//!   `Err(LarkError::Unsupported("credential_store"))`, 由部署方在外围注入
//!   凭证后调用 `set_*`。
//!
//! ## token 过期口径
//!
//! `is_expired()` 按绝对过期戳判定; 传输层刷新用 `is_expired_with_skew()`
//! (提前 [`TOKEN_REFRESH_SKEW_SECS`] 秒视为需刷新), 避免临期 token 在飞行中过期。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::lark::error::{LarkError, LarkResult};

// ============================================================================
// §1 编译期常量
// ============================================================================

/// 平台名 (持久化文件命名空间 / 协议平台标识)。
pub const PLATFORM_NAME: &str = "apeireth";

/// Provider 名 (协议面标识)。
pub const PROVIDER_NAME: &str = "lark";

/// lark 协议 schema 版本 (wire 契约版本)。
pub const LARK_SCHEMA_VERSION: &str = "1";

/// 默认平台开放 API base URL。
///
/// **这是中性占位默认值** —— 部署时必须用 [`crate::lark::LarkClientImpl`]
/// 的配置覆盖为组织协作平台的实际开放端点 (https 强制)。
pub const DEFAULT_LARK_API_BASE: &str = "https://open.lark.example.com/open-apis";

/// 默认 tenant_access_token TTL (2h = 7200s)。
pub const DEFAULT_TENANT_TOKEN_TTL_SECONDS: u64 = 7200;

/// 默认 user_access_token TTL (2h = 7200s)。
pub const DEFAULT_USER_TOKEN_TTL_SECONDS: u64 = 7200;

/// token 最大 TTL (24h, 防长占)。
pub const MAX_TOKEN_TTL_SECONDS: u64 = 86_400;

/// token 刷新提前量 (秒): 剩余 TTL 低于此值即视为需刷新。
pub const TOKEN_REFRESH_SKEW_SECS: u64 = 60;

/// App ID 最小长度 (`cli_` + 8 字符 = 12)。
pub const MIN_APP_ID_LENGTH: usize = 12;

/// App Secret 最小长度 (16 字符)。
pub const MIN_APP_SECRET_LENGTH: usize = 16;

/// App Secret 典型长度 (32 字符)。
pub const TYPICAL_APP_SECRET_LENGTH: usize = 32;

/// 编译期守门: 最小长度常量与 K-1 校验口径一致。
const _: () = assert!(MIN_APP_ID_LENGTH == 12);
const _: () = assert!(MIN_APP_SECRET_LENGTH == 16);

// ============================================================================
// §2 AppIdHolder (内存持有, 0 明文落盘)
// ============================================================================

/// App ID 持有者 (内存持有; 调试输出不带出秘密)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIdHolder {
    /// App ID 值 (仅内存)。
    app_id: Option<String>,
    /// 是否由外部凭据库加载 (当前恒 false —— 见 `from_credential_store`)。
    loaded_from_store: bool,
}

impl AppIdHolder {
    /// 创建空 holder。
    pub fn empty() -> Self {
        Self {
            app_id: None,
            loaded_from_store: false,
        }
    }

    /// 从部署方系统凭据库加载 App ID。
    ///
    /// 【显式不支持】操作系统凭据库接入依赖部署环境 (凭据设施 / 权限策略),
    /// 客户端库不自带; 返回 `Err(LarkError::Unsupported("credential_store"))`
    /// 而不是静默给空值。需要该能力的部署方在外围取到凭证后调用 [`Self::set`]。
    pub fn from_credential_store(_account: &str) -> LarkResult<Self> {
        Err(LarkError::Unsupported("credential_store"))
    }

    /// 设置 App ID (K-1 #1 强校验)。
    pub fn set(&mut self, app_id: String) -> LarkResult<()> {
        LarkError::validate_app_id(&app_id)?;
        self.app_id = Some(app_id);
        self.loaded_from_store = false;
        Ok(())
    }

    /// 读 App ID (cloned; 不暴露 &str, 防止意外进日志)。
    pub fn get(&self) -> Option<String> {
        self.app_id.clone()
    }

    /// 是否已设置。
    pub fn is_set(&self) -> bool {
        self.app_id.is_some()
    }

    /// 是否由外部凭据库加载。
    pub fn loaded_from_store(&self) -> bool {
        self.loaded_from_store
    }

    /// 清空。
    pub fn clear(&mut self) {
        self.app_id = None;
        self.loaded_from_store = false;
    }
}

impl Default for AppIdHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §3 AppSecretHolder (内存持有, Debug 脱敏)
// ============================================================================

/// App Secret 持有者 (内存持有)。
///
/// **Debug 手写脱敏**: derive(Debug) 会让一次 `{:?}` / `dbg!` 把 App Secret
/// 明文落进日志/错误面板, 违反「0 明文落盘/落日志」铁律。Serialize 保留
/// (内存态结构自身可序列化), 只修 Debug 泄露面。
#[derive(Clone, Serialize, Deserialize)]
pub struct AppSecretHolder {
    /// App Secret 值 (仅内存)。
    app_secret: Option<String>,
    /// 是否由外部凭据库加载 (当前恒 false)。
    loaded_from_store: bool,
}

impl std::fmt::Debug for AppSecretHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSecretHolder")
            .field(
                "app_secret",
                &self.app_secret.as_ref().map(|_| "[redacted]"),
            )
            .field("loaded_from_store", &self.loaded_from_store)
            .finish()
    }
}

impl AppSecretHolder {
    /// 创建空 holder。
    pub fn empty() -> Self {
        Self {
            app_secret: None,
            loaded_from_store: false,
        }
    }

    /// 从部署方系统凭据库加载 App Secret。
    ///
    /// 【显式不支持】同 [`AppIdHolder::from_credential_store`]:
    /// `Err(LarkError::Unsupported("credential_store"))`。
    pub fn from_credential_store(_account: &str) -> LarkResult<Self> {
        Err(LarkError::Unsupported("credential_store"))
    }

    /// 设置 App Secret (K-1 #2 强校验)。
    pub fn set(&mut self, app_secret: String) -> LarkResult<()> {
        LarkError::validate_app_secret(&app_secret)?;
        self.app_secret = Some(app_secret);
        self.loaded_from_store = false;
        Ok(())
    }

    /// 读 App Secret (cloned; 不暴露 &str)。
    pub fn get(&self) -> Option<String> {
        self.app_secret.clone()
    }

    /// 是否已设置。
    pub fn is_set(&self) -> bool {
        self.app_secret.is_some()
    }

    /// 是否由外部凭据库加载。
    pub fn loaded_from_store(&self) -> bool {
        self.loaded_from_store
    }

    /// 清空。
    pub fn clear(&mut self) {
        self.app_secret = None;
        self.loaded_from_store = false;
    }
}

impl Default for AppSecretHolder {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// §4 TenantAccessToken (应用级访问令牌)
// ============================================================================

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// tenant_access_token (应用级访问令牌)。
///
/// 颁发协议: `POST /auth/v3/tenant_access_token/internal`
/// body `{"app_id": "...", "app_secret": "..."}`,
/// 响应 `{"code": 0, "msg": "ok", "tenant_access_token": "t-...", "expire": 7200}`。
/// 调用 API 时携带 `Authorization: Bearer <token>`。
///
/// **Debug 手写脱敏** (`token` 是长期访问令牌)。
#[derive(Clone, Serialize, Deserialize)]
pub struct TenantAccessToken {
    /// App ID (token 来源标识)。
    pub app_id: String,
    /// token 值 (仅内存 / TokenCache 落盘, 不进日志)。
    pub token: String,
    /// 绝对过期时间戳 (秒, UNIX_EPOCH 起)。
    pub expire_at_secs: u64,
    /// 创建时间戳 (秒, UNIX_EPOCH 起)。
    pub created_at_secs: u64,
}

impl std::fmt::Debug for TenantAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantAccessToken")
            .field("app_id", &self.app_id)
            .field("token", &"[redacted]")
            .field("expire_at_secs", &self.expire_at_secs)
            .field("created_at_secs", &self.created_at_secs)
            .finish()
    }
}

impl TenantAccessToken {
    /// 创建 token (TTL 秒数, 上限 [`MAX_TOKEN_TTL_SECONDS`])。
    pub fn new(app_id: String, token: String, ttl_seconds: u64) -> LarkResult<Self> {
        LarkError::validate_app_id(&app_id)?;
        if token.is_empty() {
            return Err(LarkError::TokenExpired);
        }
        if ttl_seconds == 0 || ttl_seconds > MAX_TOKEN_TTL_SECONDS {
            return Err(LarkError::Other(format!(
                "invalid ttl: {ttl_seconds} (1..=MAX_TOKEN_TTL_SECONDS={MAX_TOKEN_TTL_SECONDS})"
            )));
        }
        let now_secs = unix_now_secs();
        Ok(Self {
            app_id,
            token,
            expire_at_secs: now_secs + ttl_seconds,
            created_at_secs: now_secs,
        })
    }

    /// 用颁发响应里的 `expire` 值构造 (协议口径: 相对 TTL 秒)。
    pub fn from_issue_response(
        app_id: String,
        token: String,
        expire_seconds: u64,
    ) -> LarkResult<Self> {
        let ttl = if expire_seconds == 0 {
            DEFAULT_TENANT_TOKEN_TTL_SECONDS
        } else {
            expire_seconds.min(MAX_TOKEN_TTL_SECONDS)
        };
        Self::new(app_id, token, ttl)
    }

    /// 默认 TTL 构造。
    pub fn with_default_ttl(app_id: String, token: String) -> LarkResult<Self> {
        Self::new(app_id, token, DEFAULT_TENANT_TOKEN_TTL_SECONDS)
    }

    /// 是否已过期 (绝对口径)。
    pub fn is_expired(&self) -> bool {
        unix_now_secs() >= self.expire_at_secs
    }

    /// 是否需刷新 (提前 [`TOKEN_REFRESH_SKEW_SECS`] 秒判定)。
    pub fn is_expired_with_skew(&self) -> bool {
        self.remaining_ttl_secs() <= TOKEN_REFRESH_SKEW_SECS
    }

    /// 剩余 TTL (秒)。
    pub fn remaining_ttl_secs(&self) -> u64 {
        self.expire_at_secs.saturating_sub(unix_now_secs())
    }
}

// ============================================================================
// §5 UserAccessToken (用户级访问令牌)
// ============================================================================

/// user_access_token (用户级访问令牌)。
///
/// 跟 tenant token 的差别: 携带用户身份 (open_id), 附带 refresh_token。
/// 授权码换发 (`code` → `user_access_token`) 需要交互式授权面,
/// 本客户端只管理已颁发令牌的生命周期。
///
/// **Debug 手写脱敏** (`access_token` / `refresh_token` 均为长期秘密)。
#[derive(Clone, Serialize, Deserialize)]
pub struct UserAccessToken {
    /// App ID (token 来源标识)。
    pub app_id: String,
    /// access token 值 (仅内存)。
    pub access_token: String,
    /// refresh token 值 (仅内存)。
    pub refresh_token: String,
    /// 用户 Open ID。
    pub open_id: String,
    /// 绝对过期时间戳 (秒, UNIX_EPOCH 起)。
    pub expire_at_secs: u64,
    /// 创建时间戳 (秒, UNIX_EPOCH 起)。
    pub created_at_secs: u64,
}

impl std::fmt::Debug for UserAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserAccessToken")
            .field("app_id", &self.app_id)
            .field("access_token", &"[redacted]")
            .field("refresh_token", &"[redacted]")
            .field("open_id", &self.open_id)
            .field("expire_at_secs", &self.expire_at_secs)
            .field("created_at_secs", &self.created_at_secs)
            .finish()
    }
}

impl UserAccessToken {
    /// 创建 token (K-1 #1 app_id + K-1 #4 open_id 强校验)。
    pub fn new(
        app_id: String,
        access_token: String,
        refresh_token: String,
        open_id: String,
        ttl_seconds: u64,
    ) -> LarkResult<Self> {
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
        let now_secs = unix_now_secs();
        Ok(Self {
            app_id,
            access_token,
            refresh_token,
            open_id,
            expire_at_secs: now_secs + ttl_seconds,
            created_at_secs: now_secs,
        })
    }

    /// 是否已过期 (绝对口径)。
    pub fn is_expired(&self) -> bool {
        unix_now_secs() >= self.expire_at_secs
    }

    /// 是否需刷新 (提前 [`TOKEN_REFRESH_SKEW_SECS`] 秒判定)。
    pub fn is_expired_with_skew(&self) -> bool {
        let now_secs = unix_now_secs();
        self.expire_at_secs.saturating_sub(now_secs) <= TOKEN_REFRESH_SKEW_SECS
    }
}

// ============================================================================
// §6 WebhookToken (事件订阅校验 token + 加密密钥)
// ============================================================================

/// Webhook 校验 token + 事件加密密钥 (成对配置的共享秘密)。
///
/// **Debug 手写脱敏** (`token` / `encrypt_key` 均为长期共享秘密)。
#[derive(Clone, Serialize, Deserialize)]
pub struct WebhookToken {
    /// 校验 token (入站回调必须携带一致的 token)。
    pub token: String,
    /// 事件加密密钥 (入站加密事件体用其派生 AES 密钥解密)。
    pub encrypt_key: String,
}

impl std::fmt::Debug for WebhookToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookToken")
            .field("token", &"[redacted]")
            .field("encrypt_key", &"[redacted]")
            .finish()
    }
}

impl WebhookToken {
    /// 创建 webhook token (两侧均非空)。
    pub fn new(token: String, encrypt_key: String) -> LarkResult<Self> {
        if token.is_empty() {
            return Err(LarkError::Other("webhook token is empty".to_string()));
        }
        if encrypt_key.is_empty() {
            return Err(LarkError::Other("webhook encrypt_key is empty".to_string()));
        }
        Ok(Self { token, encrypt_key })
    }

    /// 校验入站 token 一致 (**恒定时间比较**)。
    ///
    /// `String == String` 先比长度再逐字节早退, 时序侧信道可逐字节恢复共享秘密;
    /// 本实现 0 早退: 先按长度差置位, 再对 `max_len` 次循环做填充 OR 累加。
    pub fn verify(&self, incoming_token: &str) -> bool {
        let expected = self.token.as_bytes();
        let incoming = incoming_token.as_bytes();
        // 长度不一致即置 diff (长度本身不作为秘密处理; 字节内容走填充 + OR 累加)
        let mut diff = u8::from(expected.len() != incoming.len());
        let max_len = expected.len().max(incoming.len());
        for i in 0..max_len {
            // 恒定时间填充: 越界侧填 0, 循环次数只依赖 max_len 不依赖内容
            let x = if i < expected.len() { expected[i] } else { 0 };
            let y = if i < incoming.len() { incoming[i] } else { 0 };
            diff |= x ^ y;
        }
        diff == 0
    }
}

// ============================================================================
// §7 TokenCache (持久化 tenant token 缓存, storage_atomic 原子写)
// ============================================================================

/// tenant token 持久化缓存。
///
/// 唯一允许落盘的秘密面, 安全口径:
/// - 写入走 `storage_atomic::write_atomic_durable` (原子替换 + fsync 前置),
///   权限 `OWNER_ONLY_MODE` (0600, 属主可读写);
/// - 读改写在 `storage_atomic::with_file_lock` 文件锁下串行化 (跨线程/跨进程同序);
/// - 读到损坏/不匹配的缓存按「无缓存」处理 (容忍, 不阻断调用)。
#[derive(Debug, Clone)]
pub struct TokenCache {
    /// 缓存文件路径。
    path: PathBuf,
}

/// 缓存文件的序列化形态 (稳定字段名)。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenCacheRecord {
    /// schema 版本 (向前兼容锚点)。
    schema_version: String,
    /// 缓存的 token。
    token: TenantAccessToken,
}

impl TokenCache {
    /// 创建缓存 (指定文件路径)。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 缓存文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 加载缓存 token (app_id 不匹配 / 文件缺失 / 内容损坏 → `Ok(None)`)。
    pub fn load(&self, app_id: &str) -> LarkResult<Option<TenantAccessToken>> {
        let lock_path = apeireth_core::storage_atomic::lock_path_for(&self.path);
        let read =
            apeireth_core::storage_atomic::with_file_lock(&lock_path, || {
                match fs_err::read(&self.path) {
                    Ok(bytes) => Some(bytes),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                    Err(err) => {
                        tracing::warn!(
                            target: "apeireth_sdk_lark",
                            "token cache read failed (treated as empty): {}", err
                        );
                        None
                    }
                }
            })
            .map_err(|e| LarkError::Other(format!("token cache lock failed: {e}")))?;
        let Some(bytes) = read else {
            return Ok(None);
        };
        match serde_json::from_slice::<TokenCacheRecord>(&bytes) {
            Ok(record)
                if record.schema_version == LARK_SCHEMA_VERSION
                    && record.token.app_id == app_id =>
            {
                Ok(Some(record.token))
            }
            Ok(_) => Ok(None),
            Err(err) => {
                // 容忍损坏缓存: 记录后按无缓存处理, 下次 store 会原子覆盖
                tracing::warn!(
                    target: "apeireth_sdk_lark",
                    "token cache corrupt (treated as empty): {}", err
                );
                Ok(None)
            }
        }
    }

    /// 落盘 token (原子写 + 0600)。
    pub fn store(&self, token: &TenantAccessToken) -> LarkResult<()> {
        let record = TokenCacheRecord {
            schema_version: LARK_SCHEMA_VERSION.to_string(),
            token: token.clone(),
        };
        let json = serde_json::to_vec(&record)
            .map_err(|e| LarkError::Other(format!("token cache serialize failed: {e}")))?;
        let lock_path = apeireth_core::storage_atomic::lock_path_for(&self.path);
        apeireth_core::storage_atomic::with_file_lock(&lock_path, || {
            apeireth_core::storage_atomic::write_atomic_durable(
                &self.path,
                &json,
                apeireth_core::storage_atomic::OWNER_ONLY_MODE,
            )
        })
        .map_err(|e| LarkError::Other(format!("token cache lock failed: {e}")))?
        .map_err(|e| LarkError::Other(format!("token cache write failed: {e}")))
    }

    /// 清空缓存 (文件不存在视为成功)。
    pub fn clear(&self) -> LarkResult<()> {
        let lock_path = apeireth_core::storage_atomic::lock_path_for(&self.path);
        apeireth_core::storage_atomic::with_file_lock(&lock_path, || {
            match fs_err::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(err),
            }
        })
        .map_err(|e| LarkError::Other(format!("token cache lock failed: {e}")))?
        .map_err(|e| LarkError::Other(format!("token cache remove failed: {e}")))
    }
}

// ============================================================================
// §8 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 常量 ----

    #[test]
    fn constants_are_stable() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "lark");
        assert_eq!(LARK_SCHEMA_VERSION, "1");
        assert!(DEFAULT_LARK_API_BASE.starts_with("https://"));
        assert_eq!(DEFAULT_TENANT_TOKEN_TTL_SECONDS, 7200);
        assert_eq!(DEFAULT_USER_TOKEN_TTL_SECONDS, 7200);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
        assert_eq!(TOKEN_REFRESH_SKEW_SECS, 60);
        assert_eq!(MIN_APP_ID_LENGTH, 12);
        assert_eq!(MIN_APP_SECRET_LENGTH, 16);
        assert_eq!(TYPICAL_APP_SECRET_LENGTH, 32);
    }

    // ---- §2 AppIdHolder ----

    #[test]
    fn app_id_holder_set_get_clear() {
        let mut holder = AppIdHolder::empty();
        assert!(!holder.is_set());
        holder
            .set("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id");
        assert!(holder.is_set());
        assert_eq!(holder.get().as_deref(), Some("cli_a1b2c3d4e5f6"));
        assert!(!holder.loaded_from_store());
        holder.clear();
        assert!(!holder.is_set());
    }

    #[test]
    fn app_id_holder_set_rejects_invalid() {
        let mut holder = AppIdHolder::empty();
        assert!(matches!(
            holder.set(String::new()),
            Err(LarkError::AppIdMissing)
        ));
        assert!(matches!(
            holder.set("app_a1b2c3d4".to_string()),
            Err(LarkError::AppIdInvalid(_))
        ));
    }

    #[test]
    fn app_id_holder_credential_store_is_explicitly_unsupported() {
        // 显式不支持: 返回带稳定标识的 Err, 不静默给空值
        let result = AppIdHolder::from_credential_store("lark-app-id");
        match result {
            Err(LarkError::Unsupported("credential_store")) => {}
            other => panic!("expected Unsupported(credential_store), got {other:?}"),
        }
    }

    // ---- §3 AppSecretHolder ----

    #[test]
    fn app_secret_holder_set_get_clear() {
        let mut holder = AppSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid app secret");
        assert!(holder.is_set());
        holder.clear();
        assert!(!holder.is_set());
    }

    #[test]
    fn app_secret_holder_set_rejects_invalid() {
        let mut holder = AppSecretHolder::empty();
        assert!(matches!(
            holder.set(String::new()),
            Err(LarkError::AppSecretMissing)
        ));
        assert!(matches!(
            holder.set("short".to_string()),
            Err(LarkError::AppSecretInvalid(5))
        ));
    }

    #[test]
    fn app_secret_holder_credential_store_is_explicitly_unsupported() {
        let result = AppSecretHolder::from_credential_store("lark-app-secret");
        assert!(matches!(
            result,
            Err(LarkError::Unsupported("credential_store"))
        ));
    }

    // ---- §4 TenantAccessToken ----

    #[test]
    fn tenant_token_lifecycle() {
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123def456".to_string(),
            7200,
        )
        .expect("valid tenant token");
        assert_eq!(token.app_id, "cli_a1b2c3d4e5f6");
        assert!(!token.is_expired());
        assert!(!token.is_expired_with_skew());
        assert!(token.remaining_ttl_secs() > 7000);
    }

    #[test]
    fn tenant_token_rejects_invalid_inputs() {
        assert!(matches!(
            TenantAccessToken::new("cli_a1b2c3d4e5f6".to_string(), String::new(), 7200),
            Err(LarkError::TokenExpired)
        ));
        assert!(matches!(
            TenantAccessToken::new("invalid".to_string(), "t-abc".to_string(), 7200),
            Err(LarkError::AppIdInvalid(_))
        ));
        assert!(matches!(
            TenantAccessToken::new("cli_a1b2c3d4e5f6".to_string(), "t-abc".to_string(), 0),
            Err(LarkError::Other(_))
        ));
        assert!(matches!(
            TenantAccessToken::new(
                "cli_a1b2c3d4e5f6".to_string(),
                "t-abc".to_string(),
                MAX_TOKEN_TTL_SECONDS + 1
            ),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn tenant_token_from_issue_response_clamps_ttl() {
        // expire = 0 → 默认 TTL; 超上限 → 钳到上限
        let t = TenantAccessToken::from_issue_response(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123".to_string(),
            0,
        )
        .expect("valid");
        let ttl = t.expire_at_secs - t.created_at_secs;
        assert_eq!(ttl, DEFAULT_TENANT_TOKEN_TTL_SECONDS);

        let t = TenantAccessToken::from_issue_response(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123".to_string(),
            MAX_TOKEN_TTL_SECONDS * 2,
        )
        .expect("valid");
        let ttl = t.expire_at_secs - t.created_at_secs;
        assert_eq!(ttl, MAX_TOKEN_TTL_SECONDS);
    }

    #[test]
    fn tenant_token_skew_flags_near_expiry() {
        // TTL 30s < 60s 刷新提前量 → is_expired_with_skew 为 true, is_expired 为 false
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123".to_string(),
            TOKEN_REFRESH_SKEW_SECS / 2,
        )
        .expect("valid");
        assert!(!token.is_expired());
        assert!(token.is_expired_with_skew());
    }

    // ---- §5 UserAccessToken ----

    #[test]
    fn user_token_lifecycle_and_validation() {
        let token = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc123".to_string(),
            "ur-xyz789".to_string(),
            "ou_user1234567890abcdef".to_string(),
            7200,
        )
        .expect("valid user token");
        assert_eq!(token.open_id, "ou_user1234567890abcdef");
        assert!(!token.is_expired());
        assert!(!token.is_expired_with_skew());

        let bad = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc123".to_string(),
            "ur-xyz789".to_string(),
            "cli_invalid".to_string(),
            7200,
        );
        assert!(matches!(bad, Err(LarkError::OpenIdInvalid(_))));
    }

    // ---- §6 WebhookToken ----

    #[test]
    fn webhook_token_verify_constant_time_rejects_variants() {
        let wh = WebhookToken::new(
            "verify_token_xxx".to_string(),
            "encrypt_key_xxx".to_string(),
        )
        .expect("valid");
        assert!(wh.verify("verify_token_xxx"));
        // 同长度, 末字节不同 (早退比较会泄漏的信息, 恒定时间比较必须拒)
        assert!(!wh.verify("verify_token_xxy"));
        // 更长 (填充路径: 越界侧填 0)
        assert!(!wh.verify("verify_token_xxx_extra"));
        // 更短 (前缀匹配但短一截)
        assert!(!wh.verify("verify_token_xx"));
        // 空前串
        assert!(!wh.verify(""));
    }

    #[test]
    fn webhook_token_rejects_empty() {
        assert!(matches!(
            WebhookToken::new(String::new(), "encrypt_key".to_string()),
            Err(LarkError::Other(_))
        ));
        assert!(matches!(
            WebhookToken::new("token".to_string(), String::new()),
            Err(LarkError::Other(_))
        ));
    }

    // ---- M5: Debug 脱敏 (4 个秘密持有面) ----

    #[test]
    fn webhook_token_debug_is_redacted() {
        let wh = WebhookToken::new(
            "verify_token_secret_xxx".to_string(),
            "encrypt_key_secret_xxx".to_string(),
        )
        .expect("valid");
        let dbg = format!("{wh:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("verify_token_secret_xxx"),
            "Debug 0 泄 token: {dbg}"
        );
        assert!(
            !dbg.contains("encrypt_key_secret_xxx"),
            "Debug 0 泄 encrypt_key: {dbg}"
        );
    }

    #[test]
    fn tenant_access_token_debug_is_redacted() {
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-secret-abc123def456".to_string(),
            7200,
        )
        .expect("valid");
        let dbg = format!("{token:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("t-secret-abc123def456"),
            "Debug 0 泄 token: {dbg}"
        );
        assert!(dbg.contains("cli_a1b2c3d4e5f6"), "app_id 可见: {dbg}");
    }

    #[test]
    fn user_access_token_debug_is_redacted() {
        let token = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-secret-abc123".to_string(),
            "ur-secret-xyz789".to_string(),
            "ou_user1234567890abcdef".to_string(),
            7200,
        )
        .expect("valid");
        let dbg = format!("{token:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("u-secret-abc123"),
            "Debug 0 泄 access_token: {dbg}"
        );
        assert!(
            !dbg.contains("ur-secret-xyz789"),
            "Debug 0 泄 refresh_token: {dbg}"
        );
    }

    #[test]
    fn app_secret_holder_debug_is_redacted() {
        let mut holder = AppSecretHolder::empty();
        holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        let dbg = format!("{holder:?}");
        assert!(dbg.contains("[redacted]"), "Debug 应脱敏: {dbg}");
        assert!(
            !dbg.contains("abcdef1234567890abcdef1234567890"),
            "Debug 0 泄 app_secret: {dbg}"
        );
    }

    // ---- §7 TokenCache (storage_atomic 持久化) ----

    #[test]
    fn token_cache_store_load_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = TokenCache::new(dir.path().join("tenant-token.json"));
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-cached-abc123".to_string(),
            7200,
        )
        .expect("valid");
        cache.store(&token).expect("store");
        let loaded = cache
            .load("cli_a1b2c3d4e5f6")
            .expect("load")
            .expect("cache hit");
        assert_eq!(loaded.token, "t-cached-abc123");
        assert_eq!(loaded.expire_at_secs, token.expire_at_secs);
    }

    #[test]
    fn token_cache_miss_on_absent_or_mismatch_or_corrupt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = TokenCache::new(dir.path().join("tenant-token.json"));
        // 缺失 → None
        assert!(cache.load("cli_a1b2c3d4e5f6").expect("load").is_none());

        // app_id 不匹配 → None (防串用其它应用的缓存)
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-cached-abc123".to_string(),
            7200,
        )
        .expect("valid");
        cache.store(&token).expect("store");
        assert!(cache.load("cli_other00000000").expect("load").is_none());

        // 内容损坏 → None (容忍, 不阻断)
        fs_err::write(cache.path(), b"{not json").expect("corrupt write");
        assert!(cache.load("cli_a1b2c3d4e5f6").expect("load").is_none());
    }

    #[test]
    fn token_cache_clear_removes_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = TokenCache::new(dir.path().join("tenant-token.json"));
        let token = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-cached-abc123".to_string(),
            7200,
        )
        .expect("valid");
        cache.store(&token).expect("store");
        cache.clear().expect("clear");
        assert!(cache.load("cli_a1b2c3d4e5f6").expect("load").is_none());
        // 幂等: 再 clear 一次仍成功
        cache.clear().expect("clear again");
    }
}
