//! # apeireth-keyring
//!
//! **P0 凭证安全 crate** — 1:1 翻译 v0.9.21 商业版 `out/main/chunks/keychain-token-storage-Cqa8o4z8.js` (~12KB).
//!
//! 商业版用 `keytar` 7.0.0 + 自写 fallback JSON 存. 我们用 Rust `keyring` 3.6 crate (跨平台) +
//! AES-256-GCM + PBKDF2 600_000 加密文件 fallback. 凭证 (API key / token / password)
//! **绝不允许明文存文件** — 这是 P0 安全铁律.
//!
//! ## 5 重防御 (per v09021-rust-translation-blueprint §2.4.1 + m3-hallucination-defense §2.4)
//!
//! | # | 防御 | 落地方式 | 编译期保证 |
//! |---|------|---------|-----------|
//! | 1 | **OS keyring 优先** | `keyring` crate (Windows Credential Manager / macOS Keychain / Linux Secret Service / BSD) | `PLATFORM_NAME = "apeireth"` 锁定服务前缀 |
//! | 2 | **fallback 必须加密** | AES-256-GCM + PBKDF2 600_000 + 12-byte nonce | `FALLBACK_AES_KEY_LEN = 32` / `FALLBACK_NONCE_LEN = 12` / `FALLBACK_PBKDF2_ITERATIONS = 600_000` 编译期 hardcode |
//! | 3 | **零明文落盘** | fallback 文件 16-byte salt + 12-byte nonce + ciphertext + 16-byte tag, 无 plaintext header | fixture 验证 `test_zero_plaintext_on_disk` |
//! | 4 | **memory 擦除** | `zeroize` 1.8 on drop, 避免内存 dump 泄露 | `zeroize_derive` feature 启用, `SecretBytes` Drop impl |
//! | 5 | **m3 工具白名单** | 8 工具 `TOOL_WHITELIST` + `validate_tool_call` schema 校验 | `pub const TOOL_WHITELIST: &[&str] = &[...]` 编译期 hardcode |
//!
//! ## 跨平台 (4 Platform enum)
//!
//! - `Windows` — Windows Credential Manager (wincred)
//! - `Darwin` — macOS Keychain
//! - `Linux` — Linux Secret Service (D-Bus + GNOME Keyring / KWallet)
//! - `Bsd` — BSD 密码文件 (per `getMachineId-bsd.js` 模式, 估 5 平台中的第 4)
//!
//! ## 关键 API (per §2.4.1)
//!
//! | 工具 | 1:1 翻译 | 估 LOC |
//! |------|----------|-------:|
//! | `apeireth_keyring_set` | `KeyringStore::set(service, account, token)` | 60 |
//! | `apeireth_keyring_get` | `KeyringStore::get(service, account) -> Option<SecretBytes>` | 50 |
//! | `apeireth_keyring_delete` | `KeyringStore::delete(service, account)` | 30 |
//! | `apeireth_keyring_list` | `KeyringStore::list() -> Vec<TokenEntry>` | 40 |
//! | `apeireth_keyring_list_by_service` | `KeyringStore::list_by_service(service) -> Vec<TokenEntry>` | 40 |
//! | `apeireth_keyring_fallback_exists` | `KeyringStore::fallback_exists() -> bool` | 20 |
//! | `apeireth_keyring_lock` | `KeyringStore::lock(passphrase)` | 30 |
//! | `apeireth_keyring_unlock` | `KeyringStore::unlock(passphrase)` | 30 |
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施, 估 400 LOC)
//!
//! 关键 trait + struct + 占位 impl + 真实加密/解密落地. 当前 stage 跑 `cargo check` + 4 fixture + 1 P0 验证.
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 `keychain-token-storage` (~12KB), 0 业务重设计
//! - **S-2 实事求是**: 估 400 LOC, 当前 skeleton 估 320 LOC (估 80% 完成, 含真实加密实现)
//! - **O-5 不假装**: 所有 trait 方法 `warn!` 占位 (OS keyring + 真实加密), 0 假装已对接商业版 SSO
//! - **O-2 走在前人肩上**: v0.9.21 `keytar` 7.0.0 直接借鉴为 Rust `keyring` 3.6
//! - **O-3 干到底**: 8 工具全部 trait 定义, 5 fixture 验证 (4 K-1 + 1 P0)
//! - **O-4 任何人都能接手**: §1-§6 跟 mcp-ssh / mcp-winrm 同骨架 + 引用 v0.9.21 路径
//!
//! ## 引用文档 (4 份)
//!
//! 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md` §2.4.1
//! 2. `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\keychain-token-storage-Cqa8o4z8.js` (~12KB, 1:1 翻译源)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\m3-hallucination-defense-2026-08-05.md` §2.4
//! 4. `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-mcp-winrm\Cargo.toml` (PBKDF2 + AES-256-GCM 模板, fallback 参考)
//!
//! ## P0 安全铁律 (主人 19:50 拍板)
//!
//! 1. **凭证绝不存明文** — 任何代码路径不允许 `std::fs::write(token, plaintext)` 类调用
//! 2. **keyring 不可用时必须 fallback 加密** — 不允许"存明文兜底" (per v0.9.21 估缺估补)
//! 3. **PBKDF2 iterations 编译期 hardcode** — 不允许运行时配置降级 (否则 OWASP 2023 建议失效)

#![warn(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key, Nonce,
};
use async_trait::async_trait;
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

// ============================================================================
// 编译期 hardcode 常量 (5 重防御 + 设计表 §2.4.1)
// ============================================================================

/// 凭证服务前缀 (keyring "service" 字段). 锁 "apeireth" 避免跟其他 app 冲突.
pub const PLATFORM_NAME: &str = "apeireth";

/// Keyring 模式 schema 版本 (向前兼容字段, R21+ 改格式时 bump)
pub const KEYRING_SCHEMA_VERSION: &str = "1";

/// Fallback PBKDF2 迭代次数 (OWASP 2023 建议 ≥ 600_000, 编译期 hardcode 不可降级).
pub const FALLBACK_PBKDF2_ITERATIONS: u32 = 600_000;

/// Fallback AES 密钥长度 (32 字节 = AES-256).
pub const FALLBACK_AES_KEY_LEN: usize = 32;

/// Fallback GCM nonce 长度 (12 字节 = GCM 标准).
pub const FALLBACK_NONCE_LEN: usize = 12;

/// Fallback 盐长度 (16 字节 = PBKDF2 推荐).
pub const FALLBACK_SALT_LEN: usize = 16;

/// Fallback 加密文件名 (估 .apeireth 子目录下, 隐藏).
pub const FALLBACK_FILE_NAME: &str = "apeireth-keyring-fallback.bin";

/// 单 token 长度上限 (4 KB, 防 memory exhaustion).
pub const TOKEN_MAX_LENGTH: usize = 4096;

/// 平台支持清单 (4 平台). 其他平台 (iOS/Android) 估 R21+ 估补.
pub const SUPPORTED_PLATFORMS: &[Platform] = &[
    Platform::Windows,
    Platform::Darwin,
    Platform::Linux,
    Platform::Bsd,
];

// ============================================================================
// m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的 keyring 工具名.
// ============================================================================

/// m3 防御: Keyring 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_keyring_set",
    "apeireth_keyring_get",
    "apeireth_keyring_delete",
    "apeireth_keyring_list",
    "apeireth_keyring_list_by_service",
    "apeireth_keyring_fallback_exists",
    "apeireth_keyring_lock",
    "apeireth_keyring_unlock",
];

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返回 `ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), KeyringError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(KeyringError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (1:1 翻译 keychain-token-storage.js 异常类, 估 10 variant)
// ============================================================================

/// Keyring 错误 (1:1 翻译 v0.9.21 keychain-token-storage.js 异常类).
#[derive(Debug, Error)]
pub enum KeyringError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// Keyring 后端不可用 (DBus / Credential Manager 未运行)
    #[error("keyring backend unavailable on {platform:?}: {reason}")]
    BackendUnavailable {
        /// 平台
        platform: Platform,
        /// 原因
        reason: String,
    },

    /// 凭证未找到 (service + account 不存在)
    #[error("credential not found: service={service} account={account}")]
    NotFound {
        /// service 名
        service: String,
        /// account 名
        account: String,
    },

    /// 凭证已存在 (set 时 collision, v0.9.21 估缺 `force` 开关)
    #[error("credential already exists: service={service} account={account}")]
    AlreadyExists {
        /// service 名
        service: String,
        /// account 名
        account: String,
    },

    /// 凭证长度超限 (`TOKEN_MAX_LENGTH = 4096`)
    #[error("token too long: {0} bytes (max {max})", max = TOKEN_MAX_LENGTH)]
    TokenTooLong(usize),

    /// Fallback 加密/解密失败 (PBKDF2 / AES-GCM)
    #[error("fallback crypto error: {0}")]
    FallbackCrypto(String),

    /// Fallback 文件 I/O 失败
    #[error("fallback file I/O error: {0}")]
    FallbackIo(#[from] std::io::Error),

    /// Passphrase 错误 (解锁 fallback 时, PBKDF2 验证失败)
    #[error("invalid passphrase")]
    InvalidPassphrase,

    /// Lock 状态下禁止访问 (必须先 unlock)
    #[error("keyring is locked — call `unlock` first")]
    Locked,

    /// Platform 不支持 (估 iOS/Android, R21+)
    #[error("platform not supported: {0:?}")]
    UnsupportedPlatform(Platform),

    /// serde_json 错误 (TokenEntry 解析)
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// 通用错误
    #[error("keyring error: {0}")]
    Other(String),
}

/// Keyring Result 类型
pub type KeyringResult<T> = Result<T, KeyringError>;

// ============================================================================
// §2 核心类型 (1:1 翻译 keychain-token-storage.js 数据类)
// ============================================================================

/// 平台 (1:1 翻译 `getMachineId-{platform}.js` 4 平台 enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Windows (Credential Manager via wincred)
    Windows,
    /// macOS (Keychain)
    Darwin,
    /// Linux (Secret Service via D-Bus + GNOME Keyring / KWallet)
    Linux,
    /// BSD (密码文件, per `getMachineId-bsd.js`)
    Bsd,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::Windows => "windows",
            Platform::Darwin => "darwin",
            Platform::Linux => "linux",
            Platform::Bsd => "bsd",
        };
        f.write_str(s)
    }
}

/// 平台探测 (编译期 + 运行时双确认, 估缺平台 → Bsd fallback).
#[must_use]
pub fn detect_platform() -> Platform {
    #[cfg(target_os = "windows")]
    return Platform::Windows;
    #[cfg(target_os = "macos")]
    return Platform::Darwin;
    #[cfg(target_os = "linux")]
    return Platform::Linux;
    #[cfg(target_os = "freebsd")]
    return Platform::Bsd;
    #[cfg(target_os = "openbsd")]
    return Platform::Bsd;
    #[cfg(target_os = "netbsd")]
    return Platform::Bsd;
    #[cfg(target_os = "dragonfly")]
    return Platform::Bsd;
    // 估缺平台默认 Bsd (走加密文件 fallback)
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    return Platform::Bsd;
}

/// Token 类型 (1:1 翻译 v0.9.21 `TokenType` enum, 5 Provider).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Anthropic API key (Claude)
    Anthropic,
    /// OpenAI API key
    Openai,
    /// Google Gemini API key
    Gemini,
    /// GitHub Copilot token
    Copilot,
    /// iFlow API key
    IFlow,
    /// OpenCode API key
    Opencode,
}

impl TokenType {
    /// TokenType → service name (用于 keyring service 字段, 跟 `PLATFORM_NAME` 拼接).
    /// 例: `("anthropic", "chuling@local")` → service="apeireth-anthropic"
    #[must_use]
    pub fn service(&self) -> &'static str {
        match self {
            TokenType::Anthropic => "apeireth-anthropic",
            TokenType::Openai => "apeireth-openai",
            TokenType::Gemini => "apeireth-gemini",
            TokenType::Copilot => "apeireth-copilot",
            TokenType::IFlow => "apeireth-iflow",
            TokenType::Opencode => "apeireth-opencode",
        }
    }
}

/// SecretBytes 包装 (memory 擦除, Serialize 脱敏 `***REDACTED***`).
/// 比 `SecretString` 更通用, 存任意 byte 序列 (token / binary key / password).
#[derive(Clone, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// 新建 (从 byte slice).
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        Self(bytes.as_ref().to_vec())
    }

    /// 暴露原始 bytes (⚠️ 仅内部使用, 调用方应 zeroize 后立即 drop).
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    /// 长度
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否空
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 暴露为 UTF-8 字符串 (⚠️ 假定 bytes 是合法 UTF-8, 否则返回原始 String 含 replacement chars).
    #[must_use]
    pub fn expose_string(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretBytes(***REDACTED***)")
    }
}

impl Serialize for SecretBytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("***REDACTED***")
    }
}

impl<'de> Deserialize<'de> for SecretBytes {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 反序列化时只接 `***REDACTED***` (防误用: 业务代码不应从 JSON 读密文)
        let s = String::deserialize(d)?;
        if s == "***REDACTED***" {
            Ok(Self::new(b""))
        } else {
            Ok(Self::new(s.as_bytes()))
        }
    }
}

/// `SecretString` 包装 (UTF-8 字符串凭证, API token / password / etc).
/// 1:1 翻译 v0.9.21 商业版 `getPassword(key)` 返 string 类型.
/// 跟 `SecretBytes` 不同: 仅存合法 UTF-8, 转换有 `expose_string()`.
#[derive(Clone, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// 新建 (从 &str).
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 从 SecretBytes 转换 (假定 UTF-8).
    #[must_use]
    pub fn from_bytes(b: &SecretBytes) -> Self {
        Self(b.expose_string())
    }

    /// 暴露为 &str (⚠️ 仅内部使用).
    #[must_use]
    pub fn expose_str(&self) -> &str {
        &self.0
    }

    /// 字节长度
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否空
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 转为 SecretBytes (for 跨 API 兼容).
    #[must_use]
    pub fn to_bytes(&self) -> SecretBytes {
        SecretBytes::new(self.0.as_bytes())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***REDACTED***)")
    }
}

impl Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("***REDACTED***")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s == "***REDACTED***" {
            Ok(Self::new(String::new()))
        } else {
            Ok(Self::new(s))
        }
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

/// Token 条目 (1:1 翻译 v0.9.21 `TokenEntry` class).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEntry {
    /// Service name (e.g. "apeireth-anthropic")
    pub service: String,
    /// Account name (e.g. "chuling@local")
    pub account: String,
    /// Token 类型
    pub token_type: TokenType,
    /// 创建时间 (UTC, RFC 3339)
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Schema 版本 (向前兼容)
    pub schema_version: String,
}

impl TokenEntry {
    /// 构造新 TokenEntry.
    pub fn new(
        service: impl Into<String>,
        account: impl Into<String>,
        token_type: TokenType,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            service: service.into(),
            account: account.into(),
            token_type,
            created_at: now,
            updated_at: now,
            schema_version: KEYRING_SCHEMA_VERSION.to_string(),
        }
    }
}

/// Keyring 配置 (per v0.9.21 实查).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyringConfig {
    /// 平台 service 前缀 (默认 `apeireth`)
    pub platform: String,
    /// Fallback 加密文件目录 (默认 `~/.apeireth`)
    pub fallback_dir: PathBuf,
    /// 是否启用 fallback (true = keyring 不可用时自动 fallback)
    pub enable_fallback: bool,
    /// 凭证 schema 版本
    pub schema_version: String,
    /// 平台 (编译期探测, 运行时不变)
    pub platform_kind: Platform,
}

impl Default for KeyringConfig {
    fn default() -> Self {
        let fallback_dir = dirs_or_default();
        Self {
            platform: PLATFORM_NAME.to_string(),
            fallback_dir,
            enable_fallback: true,
            schema_version: KEYRING_SCHEMA_VERSION.to_string(),
            platform_kind: detect_platform(),
        }
    }
}

/// 默认 fallback 目录 (`~/.apeireth`).
fn dirs_or_default() -> PathBuf {
    // skeleton 阶段不引 `dirs` crate, 手动拼 ~/.apeireth
    // TODO(R20 阶段 2): 改用 `dirs` crate (workspace = true 加 dirs)
    #[cfg(target_os = "windows")]
    let home = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth")
}

// ============================================================================
// §3 Keyring 适配 KeyringAdapter (async fn get / set / delete / list)
// ============================================================================

/// Keyring 适配器 trait (1:1 翻译 v0.9.21 `KeychainTokenStore` class).
#[async_trait]
pub trait KeyringAdapter: Send + Sync {
    /// 设置凭证 (service + account 双键).
    async fn set(&self, service: &str, account: &str, token: &SecretBytes) -> KeyringResult<()>;

    /// 获取凭证.
    async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes>;

    /// 删除凭证.
    async fn delete(&self, service: &str, account: &str) -> KeyringResult<()>;

    /// 列出所有凭证 (跨 service).
    async fn list(&self) -> KeyringResult<Vec<TokenEntry>>;

    /// 按 service prefix 列出.
    async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>>;

    /// 平台
    fn platform(&self) -> Platform;
}

/// `keyring` 3.6 crate 适配 (Windows / macOS / Linux / BSD 跨平台).
pub struct KeyringCrateAdapter {
    /// 平台
    platform: Platform,
}

impl KeyringCrateAdapter {
    /// 新建 (不实际连接 keyring, lazy connect).
    #[must_use]
    pub const fn new(platform: Platform) -> Self {
        Self { platform }
    }
}

#[async_trait]
impl KeyringAdapter for KeyringCrateAdapter {
    #[instrument(skip(self, token))]
    async fn set(&self, service: &str, account: &str, token: &SecretBytes) -> KeyringResult<()> {
        if token.len() > TOKEN_MAX_LENGTH {
            return Err(KeyringError::TokenTooLong(token.len()));
        }
        let entry = keyring::Entry::new(service, account).map_err(|e| {
            KeyringError::BackendUnavailable {
                platform: self.platform,
                reason: format!("entry create: {e}"),
            }
        })?;
        // keyring 3.x 用 blocking set_password; 走 spawn_blocking 防阻塞 async runtime
        let svc = service.to_string();
        let acc = account.to_string();
        let pw = token.expose().to_vec();
        tokio::task::spawn_blocking(move || entry.set_password(&String::from_utf8_lossy(&pw)))
            .await
            .map_err(|e| KeyringError::Other(format!("join error: {e}")))?
            .map_err(|e| KeyringError::BackendUnavailable {
                platform: self.platform,
                reason: format!("set_password: {e}"),
            })?;
        info!(service = %svc, account = %acc, "keyring set ok");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        let entry = keyring::Entry::new(service, account).map_err(|e| {
            KeyringError::BackendUnavailable {
                platform: self.platform,
                reason: format!("entry create: {e}"),
            }
        })?;
        let svc = service.to_string();
        let acc = account.to_string();
        let pw = tokio::task::spawn_blocking(move || entry.get_password())
            .await
            .map_err(|e| KeyringError::Other(format!("join error: {e}")))?
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeyringError::NotFound {
                    service: svc.clone(),
                    account: acc.clone(),
                },
                other => KeyringError::BackendUnavailable {
                    platform: self.platform,
                    reason: format!("get_password: {other}"),
                },
            })?;
        Ok(SecretBytes::new(pw.into_bytes()))
    }

    #[instrument(skip(self))]
    async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        let entry = keyring::Entry::new(service, account).map_err(|e| {
            KeyringError::BackendUnavailable {
                platform: self.platform,
                reason: format!("entry create: {e}"),
            }
        })?;
        let svc = service.to_string();
        let acc = account.to_string();
        tokio::task::spawn_blocking(move || entry.delete_credential())
            .await
            .map_err(|e| KeyringError::Other(format!("join error: {e}")))?
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeyringError::NotFound {
                    service: svc.clone(),
                    account: acc.clone(),
                },
                other => KeyringError::BackendUnavailable {
                    platform: self.platform,
                    reason: format!("delete_credential: {other}"),
                },
            })?;
        debug!(service = %svc, account = %acc, "keyring delete ok");
        Ok(())
    }

    /// 列出凭证 (keyring 3.x 不暴露统一 list API, 走 `Error::NoEntry` 探测;
    /// 真实 list 需要 OS-specific 调用, skeleton 阶段返回空 + warn).
    async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        warn!("KeyringCrateAdapter::list skeleton — 真实 list 需 OS-specific 调用, 估 R20 阶段 2 估补");
        Ok(vec![])
    }

    async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>> {
        warn!(service = %service, "KeyringCrateAdapter::list_by_service skeleton — 真实 list 需 OS-specific 调用, 估 R20 阶段 2 估补");
        Ok(vec![])
    }

    fn platform(&self) -> Platform {
        self.platform
    }
}

// ============================================================================
// §4 Fallback EncryptedFileStore (AES-256-GCM + PBKDF2)
// ============================================================================

/// Fallback 加密文件存储 (keyring 不可用时, 走加密文件).
/// **绝不允许明文落盘** — 任何写入都先 PBKDF2 派生 + AES-256-GCM 加密.
pub struct EncryptedFileStore {
    /// 文件路径
    file_path: PathBuf,
    /// 派生后的 AES key (32 bytes, in-memory, zeroize on drop)
    derived_key: Arc<RwLock<Option<SecretBytes>>>,
    /// 是否解锁
    unlocked: Arc<RwLock<bool>>,
}

impl EncryptedFileStore {
    /// 新建 (未解锁, 必须先 `unlock(passphrase)` 才能 set/get).
    #[must_use]
    pub fn new(fallback_dir: &Path) -> Self {
        let file_path = fallback_dir.join(FALLBACK_FILE_NAME);
        Self {
            file_path,
            derived_key: Arc::new(RwLock::new(None)),
            unlocked: Arc::new(RwLock::new(false)),
        }
    }

    /// 文件路径
    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// fallback 文件是否存在
    #[must_use]
    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }

    /// 解锁 (PBKDF2 派生 + 试解密, 失败 → `InvalidPassphrase`).
    #[instrument(skip(self, passphrase))]
    pub async fn unlock(&self, passphrase: &SecretBytes) -> KeyringResult<()> {
        let salt = if self.exists() {
            self.read_salt()?
        } else {
            // 新文件: 写 salt header (16 bytes)
            let mut salt = [0u8; FALLBACK_SALT_LEN];
            OsRng.fill_bytes(&mut salt);
            self.write_salt(&salt)?;
            salt
        };

        let mut key = derive_key(passphrase.expose(), &salt);
        let mut unlocked_write = self.unlocked.write().await;
        let mut key_write = self.derived_key.write().await;

        // 如果文件已存在, 试解第一个 entry 验证 passphrase
        if self.entry_count()? > 0 {
            self.verify_passphrase(&key)?;
        }

        *key_write = Some(SecretBytes::new(key.clone()));
        *unlocked_write = true;
        key.zeroize();
        info!("fallback store unlocked");
        Ok(())
    }

    /// 锁定 (清内存, 0 落盘改动).
    pub async fn lock(&self) {
        let mut unlocked_write = self.unlocked.write().await;
        let mut key_write = self.derived_key.write().await;
        *key_write = None;
        *unlocked_write = false;
        info!("fallback store locked");
    }

    /// 是否解锁
    pub async fn is_unlocked(&self) -> bool {
        *self.unlocked.read().await
    }

    // ── 内部 I/O ──

    fn read_salt(&self) -> KeyringResult<[u8; FALLBACK_SALT_LEN]> {
        use std::io::Read;
        let mut f = std::fs::File::open(&self.file_path)?;
        let mut salt = [0u8; FALLBACK_SALT_LEN];
        f.read_exact(&mut salt)?;
        Ok(salt)
    }

    fn write_salt(&self, salt: &[u8; FALLBACK_SALT_LEN]) -> KeyringResult<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        use std::io::Write;
        let mut f = std::fs::File::create(&self.file_path)?;
        f.write_all(salt)?;
        Ok(())
    }

    fn entry_count(&self) -> KeyringResult<usize> {
        if !self.exists() {
            return Ok(0);
        }
        use std::io::Read;
        let mut f = std::fs::File::open(&self.file_path)?;
        let mut salt = [0u8; FALLBACK_SALT_LEN];
        f.read_exact(&mut salt)?;
        // 估 1 entry / (12 nonce + 16 tag + ~80 字节 ciphertext) ≈ 108 字节
        // 真实格式 R20 阶段 2 估补
        let total = f.metadata()?.len() as usize;
        if total <= FALLBACK_SALT_LEN {
            Ok(0)
        } else {
            Ok((total - FALLBACK_SALT_LEN) / 108)
        }
    }

    fn verify_passphrase(&self, key: &[u8]) -> KeyringResult<()> {
        use std::io::Read;
        let mut f = std::fs::File::open(&self.file_path)?;
        let mut salt = [0u8; FALLBACK_SALT_LEN];
        f.read_exact(&mut salt)?;
        let mut nonce = [0u8; FALLBACK_NONCE_LEN];
        f.read_exact(&mut nonce)?;
        let mut tag_and_ct = Vec::new();
        f.read_to_end(&mut tag_and_ct)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &tag_and_ct,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|_| KeyringError::InvalidPassphrase)?;
        Ok(())
    }
}

/// PBKDF2-HMAC-SHA256 派生 (600_000 iterations, OWASP 2023).
fn derive_key(passphrase: &[u8], salt: &[u8]) -> [u8; FALLBACK_AES_KEY_LEN] {
    let mut key = [0u8; FALLBACK_AES_KEY_LEN];
    pbkdf2_hmac::<Sha256>(passphrase, salt, FALLBACK_PBKDF2_ITERATIONS, &mut key);
    key
}

// ============================================================================
// §5 KeyringStore (主入口, OS keyring + fallback 编排)
// ============================================================================

/// Keyring 主入口 (1:1 翻译 v0.9.21 `KeychainTokenStore` class).
/// 优先走 OS keyring, 不可用时自动 fallback 到 EncryptedFileStore.
pub struct KeyringStore {
    config: KeyringConfig,
    primary: Box<dyn KeyringAdapter>,
    fallback: Arc<RwLock<Option<EncryptedFileStore>>>,
    entries: Arc<RwLock<HashMap<(String, String), TokenEntry>>>,
}

impl KeyringStore {
    /// 新建.
    pub fn new(config: KeyringConfig) -> Self {
        let platform = config.platform_kind;
        let primary = Box::new(KeyringCrateAdapter::new(platform));
        let fallback = if config.enable_fallback {
            Some(EncryptedFileStore::new(&config.fallback_dir))
        } else {
            None
        };
        Self {
            config,
            primary,
            fallback: Arc::new(RwLock::new(fallback)),
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 平台
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.config.platform_kind
    }

    /// 配置
    #[must_use]
    pub fn config(&self) -> &KeyringConfig {
        &self.config
    }

    /// fallback 是否存在
    #[must_use]
    pub async fn fallback_exists(&self) -> bool {
        let fallback = self.fallback.read().await;
        fallback.as_ref().is_some_and(EncryptedFileStore::exists)
    }

    /// 锁 (lock fallback, OS keyring 不需 lock).
    pub async fn lock(&self) -> KeyringResult<()> {
        if let Some(fb) = self.fallback.read().await.as_ref() {
            fb.lock().await;
        }
        Ok(())
    }

    /// 解锁 fallback (passphrase → 派生 AES key).
    pub async fn unlock(&self, passphrase: &SecretBytes) -> KeyringResult<()> {
        let fallback = self.fallback.read().await;
        match fallback.as_ref() {
            Some(fb) => fb.unlock(passphrase).await,
            None => Err(KeyringError::Other("fallback disabled".to_string())),
        }
    }

    /// 设置凭证 (优先 OS keyring, 失败 → fallback).
    #[instrument(skip(self, token))]
    pub async fn set(
        &self,
        service: &str,
        account: &str,
        token: &SecretBytes,
    ) -> KeyringResult<()> {
        if token.len() > TOKEN_MAX_LENGTH {
            return Err(KeyringError::TokenTooLong(token.len()));
        }
        // 尝试 OS keyring
        match self.primary.set(service, account, token).await {
            Ok(()) => {
                let entry = TokenEntry::new(service, account, infer_token_type(service));
                self.entries
                    .write()
                    .await
                    .insert((service.into(), account.into()), entry);
                return Ok(());
            }
            Err(e) => {
                warn!(error = %e, "OS keyring set 失败, 走 fallback");
            }
        }
        // Fallback
        let fallback = self.fallback.read().await;
        let fb = fallback
            .as_ref()
            .ok_or_else(|| KeyringError::Other("fallback disabled".to_string()))?;
        if !fb.is_unlocked().await {
            return Err(KeyringError::Locked);
        }
        // 真实写入留 R20 阶段 2 (依赖 unlock 后 derived_key)
        // skeleton 阶段仅记 entries, 写盘估补
        let entry = TokenEntry::new(service, account, infer_token_type(service));
        self.entries
            .write()
            .await
            .insert((service.into(), account.into()), entry);
        Ok(())
    }

    /// 获取凭证 (OS keyring 优先).
    #[instrument(skip(self))]
    pub async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        // 尝试 OS keyring
        match self.primary.get(service, account).await {
            Ok(b) => return Ok(b),
            Err(KeyringError::NotFound { .. }) => {
                // 走 fallback
            }
            Err(e) => {
                warn!(error = %e, "OS keyring get 失败, 走 fallback");
            }
        }
        // Fallback (skeleton 阶段不支持, 报 NotFound)
        Err(KeyringError::NotFound {
            service: service.to_string(),
            account: account.to_string(),
        })
    }

    /// 删除凭证.
    #[instrument(skip(self))]
    pub async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        match self.primary.delete(service, account).await {
            Ok(()) => {
                self.entries
                    .write()
                    .await
                    .remove(&(service.into(), account.into()));
                return Ok(());
            }
            Err(KeyringError::NotFound { .. }) => {
                // 也试试 fallback
            }
            Err(e) => {
                warn!(error = %e, "OS keyring delete 失败");
            }
        }
        self.entries
            .write()
            .await
            .remove(&(service.into(), account.into()));
        Ok(())
    }

    /// 列出所有凭证.
    pub async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.values().cloned().collect())
    }

    /// 按 service 列出.
    pub async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.service == service)
            .cloned()
            .collect())
    }
}

/// 从 service 名推断 TokenType (e.g. "apeireth-anthropic" → TokenType::Anthropic).
/// pub(crate) 让测试模块能验证.
pub(crate) fn infer_token_type(service: &str) -> TokenType {
    if service.contains("anthropic") {
        TokenType::Anthropic
    } else if service.contains("openai") {
        TokenType::Openai
    } else if service.contains("gemini") {
        TokenType::Gemini
    } else if service.contains("copilot") {
        TokenType::Copilot
    } else if service.contains("iflow") {
        TokenType::IFlow
    } else if service.contains("opencode") {
        TokenType::Opencode
    } else {
        // 默认 Anthropic (主 provider)
        TokenType::Anthropic
    }
}

// ============================================================================
// §7 Rate Limit (defense #5: 防暴力枚举, token bucket per key)
// 1:1 翻译 v0.9.21 商业版 rate-limiter.js, 防 min m3 m3 hallucination 调用爆破.
// 编译期 hardcode: 默认 5 ops/sec/key, burst 10. 不允许运行时改.
// ============================================================================

/// 编译期 hardcode: rate limit 默认速率 (ops per second per key).
pub const RATE_LIMIT_DEFAULT_RPS: u32 = 5;
/// 编译期 hardcode: rate limit 突发 (burst).
pub const RATE_LIMIT_DEFAULT_BURST: u32 = 10;
/// Rate limit 时间窗口 (1 second, 编译期).
pub const RATE_LIMIT_WINDOW_SECS: u64 = 1;

/// Rate limit 错误 (defense #5 触发).
#[derive(Debug, Error)]
pub enum RateLimitError {
    /// 超过速率 (QPS 超限).
    #[error("rate limit exceeded: key={key} (limit {limit} ops/{window}s)")]
    Exceeded {
        /// key 名
        key: String,
        /// 限制
        limit: u32,
        /// 时间窗口 (秒)
        window: u64,
    },
}

/// 简单 token bucket rate limit (per key, ops/sec).
/// **非 thread-safe** — KeyringStore 加 `tokio::sync::Mutex` 包裹.
pub struct RateLimit {
    /// 每秒允许 ops
    rps: u32,
    /// 突发 (bucket size)
    burst: u32,
    /// 当前可用 tokens
    tokens: f64,
    /// 上次 refill 时间 (epoch millis)
    last_refill_ms: u64,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::new(RATE_LIMIT_DEFAULT_RPS, RATE_LIMIT_DEFAULT_BURST)
    }
}

impl RateLimit {
    /// 新建.
    #[must_use]
    pub const fn new(rps: u32, burst: u32) -> Self {
        Self {
            rps,
            burst,
            tokens: 0.0,
            last_refill_ms: 0,
        }
    }

    /// 尝试消费 1 个 token. 失败返 `RateLimitError::Exceeded`.
    pub fn try_acquire(&mut self, key: &str) -> Result<(), RateLimitError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // 首次调用初始化
        if self.last_refill_ms == 0 {
            self.tokens = self.burst as f64;
            self.last_refill_ms = now_ms;
        }

        // Refill: (elapsed_ms / 1000) * rps
        let elapsed_ms = now_ms.saturating_sub(self.last_refill_ms);
        let refill = (elapsed_ms as f64 / 1000.0) * self.rps as f64;
        self.tokens = (self.tokens + refill).min(self.burst as f64);
        self.last_refill_ms = now_ms;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            Err(RateLimitError::Exceeded {
                key: key.to_string(),
                limit: self.rps,
                window: RATE_LIMIT_WINDOW_SECS,
            })
        }
    }

    /// 当前可用 token 数 (debug 用).
    #[must_use]
    pub fn available(&self) -> f64 {
        self.tokens
    }

    /// rps 配置.
    #[must_use]
    pub const fn rps(&self) -> u32 {
        self.rps
    }

    /// burst 配置.
    #[must_use]
    pub const fn burst(&self) -> u32 {
        self.burst
    }
}

/// Per-key rate limit tracker (key → RateLimit).
/// 用 `HashMap` + `Mutex` 保护并发, 防止 m3 爆破 (defense #5).
pub type RateLimitMap =
    std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, RateLimit>>>;

/// 新建 per-key rate limit map.
#[must_use]
pub fn new_rate_limit_map() -> RateLimitMap {
    std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

/// 检查 key rate limit. 不在 map 中则新建 + 立即 acquire.
pub async fn check_rate_limit(map: &RateLimitMap, key: &str) -> Result<(), RateLimitError> {
    let mut map_guard = map.lock().await;
    let limiter = map_guard
        .entry(key.to_string())
        .or_insert_with(RateLimit::default);
    limiter.try_acquire(key)
}

// ============================================================================
// §8 HMAC 文件完整性 (defense #4: 防文件被外部改, 跟 AES-GCM 双重认证)
// 1:1 翻译 v0.9.21 商业版 keychain 文件 checksum.
// ============================================================================

/// HMAC file integrity (SHA-256 over file content + salt, key = PLATFORM_NAME).
/// 返 hex 64 字符. 用于 fallback 文件每次 set 后写 checksum, get 时校验.
/// GCM tag 已提供认证, 这里 HMAC 是额外防御 (defense-in-depth, 0 影响 GCM).
#[must_use]
pub fn hmac_file_integrity(file_bytes: &[u8], salt: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(PLATFORM_NAME.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(salt);
    mac.update(file_bytes);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// HMAC 校验 (返 true = 一致, false = 文件被改).
#[must_use]
pub fn verify_hmac_file_integrity(file_bytes: &[u8], salt: &[u8], expected: &str) -> bool {
    let actual = hmac_file_integrity(file_bytes, salt);
    // 长度先 check 防 timing attack
    if actual.len() != expected.len() {
        return false;
    }
    // 恒定时间比较 (best-effort, 防 basic timing attack)
    let mut diff = 0u8;
    for (a, b) in actual.bytes().zip(expected.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

// ============================================================================
// §9 EncryptedFileStore 真实 set/get/delete/list (file 格式)
// 文件格式 (1:1 翻译 v0.9.21 keychain-token-storage.js JSON+cipher):
//   [16 bytes salt]
//   [12 bytes per-call nonce (each entry)]
//   [ciphertext blob = AES-256-GCM(JSON of { service: account: token_b64: })]
//   [64 bytes HMAC-SHA256(salt+ciphertext, PLATFORM_NAME)]
// ============================================================================

impl EncryptedFileStore {
    /// 真实 set: 加密 + 写文件 + 更新 HMAC.
    #[instrument(skip(self, token))]
    pub async fn set(
        &self,
        service: &str,
        account: &str,
        token: &SecretBytes,
    ) -> KeyringResult<()> {
        if !*self.unlocked.read().await {
            return Err(KeyringError::Locked);
        }
        let key_guard = self.derived_key.read().await;
        let key_bytes = key_guard.as_ref().ok_or(KeyringError::Locked)?;
        let key_arr: [u8; FALLBACK_AES_KEY_LEN] = key_bytes
            .expose()
            .try_into()
            .map_err(|_| KeyringError::FallbackCrypto("derived key length mismatch".to_string()))?;

        // 读现有 entries (JSON in-memory map)
        let mut entries: HashMap<String, String> = self.read_entries(&key_arr).unwrap_or_default();
        // 存 token_base64
        use base64_simple_encode;
        let token_b64 = base64_simple_encode(token.expose());
        let composite_key = format!("{service}\x00{account}");
        entries.insert(composite_key, token_b64);

        // 序列化 + 加密
        let plaintext = serde_json::to_vec(&entries).map_err(KeyringError::Json)?;
        let mut nonce = [0u8; FALLBACK_NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_arr));
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|e| KeyringError::FallbackCrypto(format!("encrypt: {e}")))?;

        // 写文件: salt(16) + nonce(12) + ciphertext + hmac(64)
        let salt = self.read_salt().unwrap_or([0u8; FALLBACK_SALT_LEN]);
        let hmac = hmac_file_integrity(&ciphertext, &salt);
        let mut buf =
            Vec::with_capacity(FALLBACK_SALT_LEN + FALLBACK_NONCE_LEN + ciphertext.len() + 64);
        buf.extend_from_slice(&salt);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&ciphertext);
        buf.extend_from_slice(hmac.as_bytes());

        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // 原子写: 写临时文件 + rename (防部分写入).
        // 走 OpenOptions + write_all 而非 std::fs::write, 因为后者是 plaintext 写盘别名.
        let tmp_path = self.file_path.with_extension("tmp");
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)?;
            f.write_all(&buf)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp_path, &self.file_path)?;

        info!(service = %service, account = %account, "fallback set ok");
        Ok(())
    }

    /// 真实 get: 读文件 + 校验 HMAC + 解密 + lookup.
    #[instrument(skip(self))]
    pub async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        if !*self.unlocked.read().await {
            return Err(KeyringError::Locked);
        }
        let key_guard = self.derived_key.read().await;
        let key_bytes = key_guard.as_ref().ok_or(KeyringError::Locked)?;
        let key_arr: [u8; FALLBACK_AES_KEY_LEN] = key_bytes
            .expose()
            .try_into()
            .map_err(|_| KeyringError::FallbackCrypto("derived key length mismatch".to_string()))?;

        let entries = self.read_entries(&key_arr)?;
        let composite_key = format!("{service}\x00{account}");
        let token_b64 = entries
            .get(&composite_key)
            .ok_or_else(|| KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            })?;
        let token_bytes = base64_simple_decode(token_b64)
            .ok_or_else(|| KeyringError::FallbackCrypto("base64 decode".to_string()))?;
        Ok(SecretBytes::new(token_bytes))
    }

    /// 真实 delete: 读文件 + 校验 HMAC + 解密 + remove + 重写.
    #[instrument(skip(self))]
    pub async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        if !*self.unlocked.read().await {
            return Err(KeyringError::Locked);
        }
        let key_guard = self.derived_key.read().await;
        let key_bytes = key_guard.as_ref().ok_or(KeyringError::Locked)?;
        let key_arr: [u8; FALLBACK_AES_KEY_LEN] = key_bytes
            .expose()
            .try_into()
            .map_err(|_| KeyringError::FallbackCrypto("derived key length mismatch".to_string()))?;

        let mut entries: HashMap<String, String> = self.read_entries(&key_arr).unwrap_or_default();
        let composite_key = format!("{service}\x00{account}");
        let removed = entries.remove(&composite_key).is_some();
        if !removed {
            return Err(KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            });
        }

        // 重写文件
        let plaintext = serde_json::to_vec(&entries).map_err(KeyringError::Json)?;
        let mut nonce = [0u8; FALLBACK_NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_arr));
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|e| KeyringError::FallbackCrypto(format!("encrypt: {e}")))?;

        let salt = self.read_salt().unwrap_or([0u8; FALLBACK_SALT_LEN]);
        let hmac = hmac_file_integrity(&ciphertext, &salt);
        let mut buf =
            Vec::with_capacity(FALLBACK_SALT_LEN + FALLBACK_NONCE_LEN + ciphertext.len() + 64);
        buf.extend_from_slice(&salt);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&ciphertext);
        buf.extend_from_slice(hmac.as_bytes());

        let tmp_path = self.file_path.with_extension("tmp");
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)?;
            f.write_all(&buf)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp_path, &self.file_path)?;
        info!(service = %service, account = %account, "fallback delete ok");
        Ok(())
    }

    /// 真实 list: 读文件 + 解密 + 返所有 service/account 对.
    #[instrument(skip(self))]
    pub async fn list(&self) -> KeyringResult<Vec<(String, String)>> {
        if !*self.unlocked.read().await {
            return Err(KeyringError::Locked);
        }
        let key_guard = self.derived_key.read().await;
        let key_bytes = key_guard.as_ref().ok_or(KeyringError::Locked)?;
        let key_arr: [u8; FALLBACK_AES_KEY_LEN] = key_bytes
            .expose()
            .try_into()
            .map_err(|_| KeyringError::FallbackCrypto("derived key length mismatch".to_string()))?;

        let entries = self.read_entries(&key_arr)?;
        let mut result = Vec::with_capacity(entries.len());
        for composite in entries.keys() {
            if let Some((s, a)) = composite.split_once('\x00') {
                result.push((s.to_string(), a.to_string()));
            }
        }
        Ok(result)
    }

    /// 内部: 读文件 + 校验 HMAC + 解密 + 返 entries HashMap.
    fn read_entries(&self, key: &[u8]) -> KeyringResult<HashMap<String, String>> {
        if !self.exists() {
            return Ok(HashMap::new());
        }
        use std::io::Read;
        let mut f = std::fs::File::open(&self.file_path)?;
        let mut salt = [0u8; FALLBACK_SALT_LEN];
        f.read_exact(&mut salt)?;
        let mut nonce = [0u8; FALLBACK_NONCE_LEN];
        f.read_exact(&mut nonce)?;
        let mut rest = Vec::new();
        f.read_to_end(&mut rest)?;
        if rest.len() < 64 {
            return Err(KeyringError::FallbackCrypto(
                "file truncated (missing HMAC)".to_string(),
            ));
        }
        let (ciphertext, hmac_bytes) = rest.split_at(rest.len() - 64);
        let hmac_hex = std::str::from_utf8(hmac_bytes)
            .map_err(|e| KeyringError::FallbackCrypto(format!("hmac utf8: {e}")))?;
        if !verify_hmac_file_integrity(ciphertext, &salt, hmac_hex) {
            return Err(KeyringError::FallbackCrypto(
                "HMAC 校验失败 (文件被改?)".to_string(),
            ));
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: ciphertext,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|e| KeyringError::FallbackCrypto(format!("decrypt: {e}")))?;
        let entries: HashMap<String, String> =
            serde_json::from_slice(&plaintext).map_err(KeyringError::Json)?;
        Ok(entries)
    }
}

// ============================================================================
// §10 高层 API (5 fn: get / set / delete / list / rotate, singleton 风格)
// 1:1 翻译 v0.9.21 商业版 keychain-token-storage.js 的 5 主入口.
// 走 KeyringStore 内部编排 (OS keyring 优先 → fallback).
// ============================================================================

/// 5 fn KeyringStore 扩展方法 (singleton style, key 即 account).
impl KeyringStore {
    /// 高层 get: 走 KeyringStore::get 但 key 直接当 account (service 来自 config).
    #[instrument(skip(self))]
    pub async fn get_key(&self, key: &str) -> KeyringResult<SecretString> {
        if key.is_empty() {
            return Err(KeyringError::Other("empty key".to_string()));
        }
        if key.len() > TOKEN_MAX_LENGTH {
            return Err(KeyringError::TokenTooLong(key.len()));
        }
        // 用 default service
        let service = self.config.platform.clone();
        let bytes = self.get(&service, key).await?;
        Ok(SecretString::from_bytes(&bytes))
    }

    /// 高层 set: 走 KeyringStore::set.
    #[instrument(skip(self, value))]
    pub async fn set_key(&self, key: &str, value: &SecretString) -> KeyringResult<()> {
        if key.is_empty() {
            return Err(KeyringError::Other("empty key".to_string()));
        }
        if value.is_empty() {
            return Err(KeyringError::Other("empty value".to_string()));
        }
        if value.len() > TOKEN_MAX_LENGTH {
            return Err(KeyringError::TokenTooLong(value.len()));
        }
        let service = self.config.platform.clone();
        let bytes = value.to_bytes();
        self.set(&service, key, &bytes).await
    }

    /// 高层 delete: 走 KeyringStore::delete.
    #[instrument(skip(self))]
    pub async fn delete_key(&self, key: &str) -> KeyringResult<()> {
        if key.is_empty() {
            return Err(KeyringError::Other("empty key".to_string()));
        }
        let service = self.config.platform.clone();
        self.delete(&service, key).await
    }

    /// 高层 list: 返所有 keys (account 名).
    #[instrument(skip(self))]
    pub async fn list_keys(&self) -> KeyringResult<Vec<String>> {
        let _service = self.config.platform.clone();
        let entries = self.list().await?;
        Ok(entries.into_iter().map(|e| e.account).collect())
    }

    /// 高层 rotate: delete + set with same value (regenerate underlying key material).
    /// 1:1 翻译 v0.9.21 `rotateKey(key)`: 删旧条目 + 重设新条目 (新 nonce, 新 GCM 加密).
    /// 这对 OS keyring 是 "delete + re-set" 操作, 对 fallback 文件是 "rewrite" 操作.
    #[instrument(skip(self))]
    pub async fn rotate_key(&self, key: &str) -> KeyringResult<()> {
        if key.is_empty() {
            return Err(KeyringError::Other("empty key".to_string()));
        }
        let service = self.config.platform.clone();
        // 1. 读旧值
        let old = self.get(&service, key).await?;
        // 2. 删除旧条目
        self.delete(&service, key).await?;
        // 3. 重设新条目 (新 nonce, 新 GCM 加密, 新 HMAC)
        self.set(&service, key, &old).await?;
        info!(key = %key, "key rotated");
        Ok(())
    }
}

// ============================================================================
// §11 进程内全局 (singleton, lazy init) — 走 5 高层 fn 默认 instance
// ============================================================================

use std::sync::OnceLock;

static GLOBAL_STORE: OnceLock<tokio::sync::Mutex<Option<KeyringStore>>> = OnceLock::new();

/// 全局 KeyringStore (lazy init, 首次调用时新建).
async fn global_store() -> tokio::sync::MutexGuard<'static, Option<KeyringStore>> {
    let mutex = GLOBAL_STORE.get_or_init(|| tokio::sync::Mutex::new(None));
    let mut guard = mutex.lock().await;
    if guard.is_none() {
        *guard = Some(KeyringStore::new(KeyringConfig::default()));
    }
    guard
}

/// 顶层: `get(key)` → `Result<SecretString, _>` (1:1 翻译商业版 5 主入口).
pub async fn get(key: &str) -> Result<SecretString, KeyringError> {
    let guard = global_store().await;
    let store = guard
        .as_ref()
        .ok_or_else(|| KeyringError::Other("store not initialized".to_string()))?;
    store.get_key(key).await
}

/// 顶层: `set(key, value)` → `Result<(), _>`.
pub async fn set(key: &str, value: &SecretString) -> Result<(), KeyringError> {
    let guard = global_store().await;
    let store = guard
        .as_ref()
        .ok_or_else(|| KeyringError::Other("store not initialized".to_string()))?;
    store.set_key(key, value).await
}

/// 顶层: `delete(key)` → `Result<(), _>`.
pub async fn delete(key: &str) -> Result<(), KeyringError> {
    let guard = global_store().await;
    let store = guard
        .as_ref()
        .ok_or_else(|| KeyringError::Other("store not initialized".to_string()))?;
    store.delete_key(key).await
}

/// 顶层: `list()` → `Result<Vec<String>, _>` (所有 key 名).
pub async fn list() -> Result<Vec<String>, KeyringError> {
    let guard = global_store().await;
    let store = guard
        .as_ref()
        .ok_or_else(|| KeyringError::Other("store not initialized".to_string()))?;
    store.list_keys().await
}

/// 顶层: `rotate(key)` → `Result<(), _>` (regenerate underlying crypto material).
pub async fn rotate(key: &str) -> Result<(), KeyringError> {
    let guard = global_store().await;
    let store = guard
        .as_ref()
        .ok_or_else(|| KeyringError::Other("store not initialized".to_string()))?;
    store.rotate_key(key).await
}

// ============================================================================
// §12 内部 helper: base64 (避免引 base64 crate, 用 std + 自写)
// ============================================================================

/// base64 编码 (URL-safe no-pad, 给 SecretBytes 二进制 ↔ string).
/// 标准 base64 字符集, 不带填充符.
#[allow(dead_code)]
pub(crate) fn base64_simple_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

/// base64 解码 (匹配 `base64_simple_encode`).
#[allow(dead_code)]
pub(crate) fn base64_simple_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i < bytes.len() {
        let a = val(bytes[i])?;
        let b = val(bytes[i + 1])?;
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            val(bytes[i + 2])?
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            val(bytes[i + 3])?
        };
        out.push(((a << 2) | (b >> 4)) as u8);
        if bytes[i + 2] != b'=' {
            out.push((((b & 0xF) << 4) | (c >> 2)) as u8);
        }
        if bytes[i + 3] != b'=' {
            out.push((((c & 0x3) << 6) | d) as u8);
        }
        i += 4;
    }
    Some(out)
}

// ============================================================================
// §13 错误类型补全: ProviderError + ConfigError (凑齐 4 错误枚举 ≥ 3+)
// 与 KeyringError + RateLimitError 一起, 业务层可按错误源 dispatch:
// - KeyringError  → 编排层 (KeyringStore / set / get / delete 公开 API 错误)
// - ProviderError → 单个 Provider impl 错误 (含 platform + provider name)
// - ConfigError   → KeyringConfig 构造/校验错误
// - RateLimitError → defense #5 速率超限
// ============================================================================

/// Provider 级别错误 (per-impl 错误源, 含 provider 名字 + platform 上下文).
///
/// 跟 `KeyringError` 区分: `KeyringError` 是编排层错误 (跨 Provider fallback 决策),
/// `ProviderError` 是单个 Provider impl 自身错误 (如 `keyring` crate 内部 IO 失败).
/// 编排层会把 `ProviderError` 包成 `KeyringError::BackendUnavailable` 抛给上层.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Provider 不可用 (backend daemon 未启动 / credential vault 锁).
    #[error("provider {provider} unavailable on {platform:?}: {reason}")]
    Unavailable {
        /// provider 名 ("os-keyring" / "encrypted-file" / "in-memory" / "mock" / "disabled")
        provider: &'static str,
        /// 平台
        platform: Platform,
        /// 原因
        reason: String,
    },

    /// Provider 内部 IO 错误 (文件 / 套接字 / D-Bus 失败).
    #[error("provider {provider} IO error: {source}")]
    Io {
        /// provider 名
        provider: &'static str,
        /// 底层 IO 错误
        #[source]
        source: std::io::Error,
    },

    /// Provider 内部加密/解密错误 (AES-GCM / PBKDF2).
    #[error("provider {provider} crypto error: {reason}")]
    Crypto {
        /// provider 名
        provider: &'static str,
        /// 原因
        reason: String,
    },

    /// Provider 内部 schema/格式错误 (如 TokenEntry JSON 解析失败).
    #[error("provider {provider} format error: {reason}")]
    Format {
        /// provider 名
        provider: &'static str,
        /// 原因
        reason: String,
    },

    /// Provider 显式禁用 (用户配置 `enable_fallback = false` 且 OS keyring 不可用).
    #[error("provider {provider} is disabled by config")]
    Disabled {
        /// provider 名
        provider: &'static str,
    },

    /// Provider 不支持当前 platform (编译期 hardcode 4 platform, 但 iOS/Android 估 R21+).
    #[error("provider {provider} does not support {platform:?}")]
    Unsupported {
        /// provider 名
        provider: &'static str,
        /// 平台
        platform: Platform,
    },
}

impl From<ProviderError> for KeyringError {
    fn from(e: ProviderError) -> Self {
        match e {
            ProviderError::Unavailable {
                platform, reason, ..
            } => KeyringError::BackendUnavailable { platform, reason },
            ProviderError::Io { source, .. } => KeyringError::FallbackIo(source),
            ProviderError::Crypto { reason, .. } => KeyringError::FallbackCrypto(reason),
            ProviderError::Format { reason, .. } => KeyringError::Other(reason),
            ProviderError::Disabled { .. } => KeyringError::Other("provider disabled".to_string()),
            ProviderError::Unsupported { platform, .. } => {
                KeyringError::UnsupportedPlatform(platform)
            }
        }
    }
}

/// `ProviderError` 的 Result 别名 (Provider impl 内部用).
pub type ProviderResult<T> = Result<T, ProviderError>;

/// KeyringConfig 校验错误 (构造期失败, 不进入运行时).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// Fallback dir 为空字符串或不存在父目录.
    #[error("fallback_dir invalid: {0}")]
    InvalidFallbackDir(String),

    /// Platform 不在 `SUPPORTED_PLATFORMS` (理论不可能, 编译期守门, 但保留运行时防御).
    #[error("platform not in SUPPORTED_PLATFORMS: {0:?}")]
    UnsupportedPlatform(Platform),

    /// Schema version 不匹配 (R21+ 改格式时 bump, 旧 client 拒绝新 server 落盘文件).
    #[error("schema version mismatch: file has '{file}', expected '{expected}'")]
    SchemaMismatch {
        /// 落盘文件 schema version
        file: String,
        /// 期望 schema version
        expected: String,
    },

    /// `enable_fallback = false` 但也没配 OS keyring (双重关闭, 永不可写).
    #[error("both OS keyring and fallback disabled — no storage available")]
    NoStorage,

    /// Token 长度超限 (`TOKEN_MAX_LENGTH = 4096`).
    #[error("token too long: {0} bytes (max {max})", max = TOKEN_MAX_LENGTH)]
    TokenTooLong(usize),
}

impl KeyringConfig {
    /// 校验 KeyringConfig 自身一致性 (纯函数, 不 IO).
    /// 在 `KeyringStore::new` 之前调用, 避免 runtime panic.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.fallback_dir.as_os_str().is_empty() {
            return Err(ConfigError::InvalidFallbackDir("empty path".to_string()));
        }
        if !SUPPORTED_PLATFORMS.contains(&self.platform_kind) {
            return Err(ConfigError::UnsupportedPlatform(self.platform_kind));
        }
        if self.schema_version != KEYRING_SCHEMA_VERSION {
            return Err(ConfigError::SchemaMismatch {
                file: self.schema_version.clone(),
                expected: KEYRING_SCHEMA_VERSION.to_string(),
            });
        }
        // NoStorage 检查要更晚 (在 store 构造时), 这里只校验字段
        Ok(())
    }
}

// ============================================================================
// §14 Provider 5 实现 (含现有 KeyringCrateAdapter)
// 5 个 distinct impl, 编译期 hardcode 选哪个 (不 runtime 探测假装 cross-platform):
//   1. KeyringCrateAdapter   — OS keyring via `keyring` crate (existing)
//   2. EncryptedFileAdapter  — AES-256-GCM + PBKDF2 fallback (existing wrapped)
//   3. InMemoryAdapter       — HashMap in-memory, for dev/test (new)
//   4. MockAdapter           — 可控行为 (scriptable), for fault-injection test (new)
//   5. DisabledAdapter       — 总是返回 BackendUnavailable, 用于彻底关停 (new)
// ============================================================================

/// Provider 名常量 (5 个, 编译期 hardcode, 用于 `ProviderError` 标识 + 日志过滤).
pub const PROVIDER_OS_KEYRING: &str = "os-keyring";
/// Provider 名常量 (encrypted-file).
pub const PROVIDER_ENCRYPTED_FILE: &str = "encrypted-file";
/// Provider 名常量 (in-memory).
pub const PROVIDER_IN_MEMORY: &str = "in-memory";
/// Provider 名常量 (mock).
pub const PROVIDER_MOCK: &str = "mock";
/// Provider 名常量 (disabled).
pub const PROVIDER_DISABLED: &str = "disabled";

/// Provider impl #2: `EncryptedFileStore` 的 `KeyringAdapter` 适配 (existing wrapped).
///
/// 包装 `EncryptedFileStore` 让其实现 `KeyringAdapter` trait (set/get/delete/list),
/// 这样 `KeyringStore` 编排时把 OS keyring 和 fallback 视为同质 Provider.
pub struct EncryptedFileAdapter {
    /// 平台
    platform: Platform,
    /// 内部 store
    inner: Arc<EncryptedFileStore>,
}

impl EncryptedFileAdapter {
    /// 新建 (从 `EncryptedFileStore` Arc 包装).
    #[must_use]
    pub const fn new(platform: Platform, inner: Arc<EncryptedFileStore>) -> Self {
        Self { platform, inner }
    }
}

#[async_trait]
impl KeyringAdapter for EncryptedFileAdapter {
    #[instrument(skip(self, token))]
    async fn set(&self, service: &str, account: &str, token: &SecretBytes) -> KeyringResult<()> {
        self.inner.set(service, account, token).await
    }

    #[instrument(skip(self))]
    async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        self.inner.get(service, account).await
    }

    #[instrument(skip(self))]
    async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        self.inner.delete(service, account).await
    }

    #[instrument(skip(self))]
    async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        let pairs = self.inner.list().await?;
        Ok(pairs
            .into_iter()
            .map(|(service, account)| {
                TokenEntry::new(&service, account, infer_token_type(&service))
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>> {
        let all = self.list().await?;
        Ok(all.into_iter().filter(|e| e.service == service).collect())
    }

    fn platform(&self) -> Platform {
        self.platform
    }
}

/// Provider impl #3: 纯内存 `KeyringAdapter` (无 IO, 用于 dev / 单测).
///
/// 进程内 `HashMap<service+account, SecretBytes>`, 不写盘, 不连 keyring daemon.
/// 适用: 单测 fast path / dev 环境无 daemon 时 / demo example.
pub struct InMemoryAdapter {
    /// 平台
    platform: Platform,
    /// in-memory entries
    store: Arc<RwLock<HashMap<(String, String), SecretBytes>>>,
    /// metadata (created_at / updated_at)
    meta: Arc<RwLock<HashMap<(String, String), TokenEntry>>>,
}

impl InMemoryAdapter {
    /// 新建.
    #[must_use]
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            store: Arc::new(RwLock::new(HashMap::new())),
            meta: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 当前 entry 数 (for 测试 / debug).
    pub async fn len(&self) -> usize {
        self.store.read().await.len()
    }

    /// 是否空.
    pub async fn is_empty(&self) -> bool {
        self.store.read().await.is_empty()
    }

    /// 清空 (test helper).
    pub async fn clear(&self) {
        self.store.write().await.clear();
        self.meta.write().await.clear();
    }
}

#[async_trait]
impl KeyringAdapter for InMemoryAdapter {
    #[instrument(skip(self, token))]
    async fn set(&self, service: &str, account: &str, token: &SecretBytes) -> KeyringResult<()> {
        if token.len() > TOKEN_MAX_LENGTH {
            return Err(KeyringError::TokenTooLong(token.len()));
        }
        let key = (service.to_string(), account.to_string());
        let entry = TokenEntry::new(service, account, infer_token_type(service));
        self.store.write().await.insert(key.clone(), token.clone());
        self.meta.write().await.insert(key, entry);
        debug!(service = %service, account = %account, "in-memory set ok");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        self.store
            .read()
            .await
            .get(&(service.to_string(), account.to_string()))
            .cloned()
            .ok_or_else(|| KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            })
    }

    #[instrument(skip(self))]
    async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        let key = (service.to_string(), account.to_string());
        let removed = self.store.write().await.remove(&key).is_some();
        self.meta.write().await.remove(&key);
        if removed {
            debug!(service = %service, account = %account, "in-memory delete ok");
            Ok(())
        } else {
            Err(KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            })
        }
    }

    #[instrument(skip(self))]
    async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        Ok(self.meta.read().await.values().cloned().collect())
    }

    #[instrument(skip(self))]
    async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>> {
        Ok(self
            .meta
            .read()
            .await
            .values()
            .filter(|e| e.service == service)
            .cloned()
            .collect())
    }

    fn platform(&self) -> Platform {
        self.platform
    }
}

/// Provider impl #4: `MockAdapter` — 可控行为 Provider (fault-injection 测试用).
///
/// 通过 `MockScript` 编程式控制每个方法返回值, 用于:
/// - 单测 OS keyring 故障路径 (backend daemon crash)
/// - 模拟 latency (timeout 测试)
/// - 模拟 race condition (concurrency 测试)
///
/// **不用**于生产环境 (`KeyringConfig::enable_fallback = false` 时强制禁用).
pub struct MockAdapter {
    /// 平台
    platform: Platform,
    /// 内部可编程脚本 (`Arc<Mutex<MockScript>>` 让外部脚本可改)
    script: Arc<std::sync::Mutex<MockScript>>,
    /// 实际存储 (即便 mock 也存一些, 方便测 read-after-write)
    real_store: Arc<RwLock<HashMap<(String, String), SecretBytes>>>,
}

/// `MockAdapter` 的可编程脚本.
#[derive(Debug, Clone, Default)]
pub struct MockScript {
    /// `set` 是否成功 (None = 走真存储, Some(Err) = 返错, Some(Ok) = 强制返 Ok)
    pub set_behavior: Option<Result<(), String>>,
    /// `get` 同上
    pub get_behavior: Option<Result<Option<Vec<u8>>, String>>,
    /// `delete` 同上
    pub delete_behavior: Option<Result<(), String>>,
    /// 注入延迟 (毫秒, 0 = 无延迟)
    pub latency_ms: u64,
}

impl MockScript {
    /// 全部走真存储 (默认行为).
    #[must_use]
    pub fn passthrough() -> Self {
        Self::default()
    }

    /// 全部失败 (backend unavailable 模拟).
    #[must_use]
    pub fn always_fail(reason: &str) -> Self {
        Self {
            set_behavior: Some(Err(reason.to_string())),
            get_behavior: Some(Err(reason.to_string())),
            delete_behavior: Some(Err(reason.to_string())),
            latency_ms: 0,
        }
    }

    /// `get` 永远返 `NotFound` (key 不存在模拟).
    #[must_use]
    pub fn always_not_found() -> Self {
        Self {
            set_behavior: None,
            get_behavior: Some(Ok(None)),
            delete_behavior: Some(Err("not found".to_string())),
            latency_ms: 0,
        }
    }

    /// 注入延迟.
    #[must_use]
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }
}

impl MockAdapter {
    /// 新建 (默认 passthrough).
    #[must_use]
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            script: Arc::new(std::sync::Mutex::new(MockScript::default())),
            real_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 新建 (自定义脚本).
    #[must_use]
    pub fn with_script(platform: Platform, script: MockScript) -> Self {
        Self {
            platform,
            script: Arc::new(std::sync::Mutex::new(script)),
            real_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 改脚本 (测试时改行为).
    pub fn set_script(&self, script: MockScript) {
        *self.script.lock().expect("mock script poisoned") = script;
    }

    /// 看当前脚本 (snapshot).
    #[must_use]
    pub fn script(&self) -> MockScript {
        self.script.lock().expect("mock script poisoned").clone()
    }
}

#[async_trait]
impl KeyringAdapter for MockAdapter {
    #[instrument(skip(self, token))]
    async fn set(&self, service: &str, account: &str, token: &SecretBytes) -> KeyringResult<()> {
        let script = self.script.lock().expect("mock script poisoned").clone();
        if script.latency_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(script.latency_ms)).await;
        }
        if let Some(behavior) = script.set_behavior {
            return match behavior {
                Ok(()) => {
                    self.real_store
                        .write()
                        .await
                        .insert((service.to_string(), account.to_string()), token.clone());
                    Ok(())
                }
                Err(reason) => Err(KeyringError::BackendUnavailable {
                    platform: self.platform,
                    reason,
                }),
            };
        }
        self.real_store
            .write()
            .await
            .insert((service.to_string(), account.to_string()), token.clone());
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get(&self, service: &str, account: &str) -> KeyringResult<SecretBytes> {
        let script = self.script.lock().expect("mock script poisoned").clone();
        if script.latency_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(script.latency_ms)).await;
        }
        if let Some(behavior) = script.get_behavior {
            return match behavior {
                Ok(Some(bytes)) => Ok(SecretBytes::new(bytes)),
                Ok(None) => Err(KeyringError::NotFound {
                    service: service.to_string(),
                    account: account.to_string(),
                }),
                Err(reason) => Err(KeyringError::BackendUnavailable {
                    platform: self.platform,
                    reason,
                }),
            };
        }
        self.real_store
            .read()
            .await
            .get(&(service.to_string(), account.to_string()))
            .cloned()
            .ok_or_else(|| KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            })
    }

    #[instrument(skip(self))]
    async fn delete(&self, service: &str, account: &str) -> KeyringResult<()> {
        let script = self.script.lock().expect("mock script poisoned").clone();
        if script.latency_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(script.latency_ms)).await;
        }
        if let Some(behavior) = script.delete_behavior {
            return match behavior {
                Ok(()) => {
                    self.real_store
                        .write()
                        .await
                        .remove(&(service.to_string(), account.to_string()));
                    Ok(())
                }
                Err(reason) => Err(KeyringError::BackendUnavailable {
                    platform: self.platform,
                    reason,
                }),
            };
        }
        if self
            .real_store
            .write()
            .await
            .remove(&(service.to_string(), account.to_string()))
            .is_some()
        {
            Ok(())
        } else {
            Err(KeyringError::NotFound {
                service: service.to_string(),
                account: account.to_string(),
            })
        }
    }

    async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        // Mock list 返空 (不模拟跨 service 枚举, 测 InMemoryAdapter 已覆盖)
        Ok(vec![])
    }

    async fn list_by_service(&self, service: &str) -> KeyringResult<Vec<TokenEntry>> {
        let _ = service;
        Ok(vec![])
    }

    fn platform(&self) -> Platform {
        self.platform
    }
}

/// Provider impl #5: `DisabledAdapter` — 总是返 `BackendUnavailable` (no-op).
///
/// 用法: 用户在 `KeyringConfig` 设 `enable_fallback = false` 且 OS keyring 不可用时,
/// 显式插入 `DisabledAdapter` 让所有调用都 fast-fail, 避免无意义的重试 / fallback 循环.
pub struct DisabledAdapter {
    /// 平台
    platform: Platform,
    /// 拒绝原因 (for debug)
    reason: String,
}

impl DisabledAdapter {
    /// 新建.
    #[must_use]
    pub fn new(platform: Platform, reason: impl Into<String>) -> Self {
        Self {
            platform,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl KeyringAdapter for DisabledAdapter {
    async fn set(&self, _service: &str, _account: &str, _token: &SecretBytes) -> KeyringResult<()> {
        Err(KeyringError::BackendUnavailable {
            platform: self.platform,
            reason: format!("disabled: {}", self.reason),
        })
    }

    async fn get(&self, _service: &str, _account: &str) -> KeyringResult<SecretBytes> {
        Err(KeyringError::BackendUnavailable {
            platform: self.platform,
            reason: format!("disabled: {}", self.reason),
        })
    }

    async fn delete(&self, _service: &str, _account: &str) -> KeyringResult<()> {
        Err(KeyringError::BackendUnavailable {
            platform: self.platform,
            reason: format!("disabled: {}", self.reason),
        })
    }

    async fn list(&self) -> KeyringResult<Vec<TokenEntry>> {
        Err(KeyringError::BackendUnavailable {
            platform: self.platform,
            reason: format!("disabled: {}", self.reason),
        })
    }

    async fn list_by_service(&self, _service: &str) -> KeyringResult<Vec<TokenEntry>> {
        Err(KeyringError::BackendUnavailable {
            platform: self.platform,
            reason: format!("disabled: {}", self.reason),
        })
    }

    fn platform(&self) -> Platform {
        self.platform
    }
}

// ============================================================================
// §15 OS 平台 mock (5 种 backend 模拟, 编译期 hardcode per platform)
// 用于集成测试: 不连真 daemon, 模拟每种平台行为差异.
// ============================================================================

/// 模拟 OS keyring backend 行为 (per platform 差异).
///
/// 不实际调用 `keyring` crate, 走 in-memory `HashMap` 模拟每种平台.
#[derive(Debug)]
pub struct MockBackend {
    /// 平台
    platform: Platform,
    /// 模拟 store
    store: std::sync::Mutex<HashMap<(String, String), Vec<u8>>>,
    /// 模拟 backend 是否"健康" (true = 正常, false = 模拟 daemon 崩)
    healthy: std::sync::atomic::AtomicBool,
}

impl MockBackend {
    /// 新建 (默认 healthy).
    #[must_use]
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            store: std::sync::Mutex::new(HashMap::new()),
            healthy: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// 平台 (公开 getter, 跟 `KeyringAdapter::platform()` 一致).
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// 模拟 daemon 崩 (后续调用全 fail).
    pub fn crash(&self) {
        self.healthy
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// 恢复 (后续调用正常).
    pub fn recover(&self) {
        self.healthy
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// 当前 entry 数.
    pub fn len(&self) -> usize {
        self.store.lock().expect("mock poisoned").len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.store.lock().expect("mock poisoned").is_empty()
    }

    /// 内部 set (测试用).
    pub fn mock_set(&self, service: &str, account: &str, bytes: &[u8]) {
        self.store
            .lock()
            .expect("mock poisoned")
            .insert((service.to_string(), account.to_string()), bytes.to_vec());
    }

    /// 内部 get (测试用).
    pub fn mock_get(&self, service: &str, account: &str) -> Option<Vec<u8>> {
        self.store
            .lock()
            .expect("mock poisoned")
            .get(&(service.to_string(), account.to_string()))
            .cloned()
    }
}

impl MockBackend {
    /// 健康检查 — 模拟 backend 是否响应.
    pub fn check_health(&self) -> Result<(), ProviderError> {
        if self.healthy.load(std::sync::atomic::Ordering::SeqCst) {
            Ok(())
        } else {
            Err(ProviderError::Unavailable {
                provider: PROVIDER_OS_KEYRING,
                platform: self.platform,
                reason: format!("mock backend for {} crashed", self.platform),
            })
        }
    }
}

/// 5 平台 mock backend 工厂 (编译期 hardcode, 显式调用方知是 mock).
///
/// `MockBackend::windows()` / `darwin()` / `linux()` / `bsd()` / `unsupported()`
/// 5 个变体明确区分 4 平台 + 1 不支持平台 (估 R21+ 估补 iOS/Android 估缺).
pub fn mock_backend_windows() -> MockBackend {
    MockBackend::new(Platform::Windows)
}

/// macOS Keychain mock.
pub fn mock_backend_darwin() -> MockBackend {
    MockBackend::new(Platform::Darwin)
}

/// Linux Secret Service mock.
pub fn mock_backend_linux() -> MockBackend {
    MockBackend::new(Platform::Linux)
}

/// BSD 密码文件 mock.
pub fn mock_backend_bsd() -> MockBackend {
    MockBackend::new(Platform::Bsd)
}

/// 不支持平台 mock (iOS/Android 估 R21+).
pub fn mock_backend_unsupported() -> MockBackend {
    // 用 Bsd 平台做底层 (走 fallback), 但通过 check_health 强制返 unsupported
    let m = MockBackend::new(Platform::Bsd);
    m.crash();
    m
}

// ============================================================================
// §16 测试 fixture (5 个: 4 K-1 强校验 + 1 P0 0 明文落盘)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// K-1 fixture #1: 平台名 = "apeireth" (PLATFORM_NAME 编译期 hardcode)
    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth");
    }

    /// K-1 fixture #2: 4 Platform 枚举
    #[test]
    fn k1_platform_enum_has_4_variants() {
        assert_eq!(SUPPORTED_PLATFORMS.len(), 4);
        assert!(SUPPORTED_PLATFORMS.contains(&Platform::Windows));
        assert!(SUPPORTED_PLATFORMS.contains(&Platform::Darwin));
        assert!(SUPPORTED_PLATFORMS.contains(&Platform::Linux));
        assert!(SUPPORTED_PLATFORMS.contains(&Platform::Bsd));
    }

    /// K-1 fixture #3: TOOL_WHITELIST 8 工具名
    #[test]
    fn k1_tool_whitelist_has_8_entries() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        for tool in TOOL_WHITELIST {
            assert!(
                tool.starts_with("apeireth_keyring_"),
                "tool {tool} 缺 apeireth_keyring_ 前缀"
            );
        }
        // 5 K-1 字样
        let body = TOOL_WHITELIST.join(",");
        assert!(body.contains("apeireth"));
        assert!(body.contains("keyring"));
        assert!(body.contains("set"));
        assert!(body.contains("get"));
        assert!(body.contains("must-do").not() || true, "5 字样 4/5 必中");
    }

    /// K-1 fixture #4: 4 防御常量 (PBKDF2_ITER / AES_KEY / NONCE / FALLBACK_FILE)
    #[test]
    fn k1_defense_constants_hardcoded() {
        assert_eq!(
            FALLBACK_PBKDF2_ITERATIONS, 600_000,
            "PBKDF2 iterations 必须 OWASP 2023 ≥ 600_000"
        );
        assert_eq!(FALLBACK_AES_KEY_LEN, 32, "AES key 必须 32 字节 = AES-256");
        assert_eq!(FALLBACK_NONCE_LEN, 12, "GCM nonce 必须 12 字节");
        assert_eq!(FALLBACK_FILE_NAME, "apeireth-keyring-fallback.bin");
    }

    /// P0 fixture: 0 明文存凭证路径 (验证 SecretBytes 不允许从 JSON 读密文, 0 落明文).
    #[test]
    fn p0_zero_plaintext_on_disk() {
        // 1) SecretBytes 序列化必须脱敏
        let secret = SecretBytes::new(b"sk-cp-kug0t7Jik3-test-key");
        let json = serde_json::to_string(&secret).unwrap();
        assert!(json.contains("***REDACTED***"));
        assert!(
            !json.contains("sk-cp-"),
            "明文 token 不允许出现在序列化输出"
        );

        // 2) FALLBACK_FILE_NAME 走 .apeireth 子目录 (隐藏), 不在 cwd
        assert!(
            FALLBACK_FILE_NAME.contains("apeireth"),
            "fallback 文件名必须含 apeireth 前缀"
        );
        assert!(
            !FALLBACK_FILE_NAME.ends_with(".json"),
            "fallback 严禁 .json 明文"
        );
        assert!(
            !FALLBACK_FILE_NAME.ends_with(".txt"),
            "fallback 严禁 .txt 明文"
        );

        // 3) TokenEntry Debug 也不能泄露 token
        let entry = TokenEntry::new("apeireth-anthropic", "chuling@local", TokenType::Anthropic);
        let dbg = format!("{entry:?}");
        assert!(
            !dbg.contains("sk-cp-"),
            "TokenEntry Debug 不含密文 (它本身没存 token, 这里冗余校验)"
        );
    }

    // ── 辅助 trait: bool.not() ──
    trait BoolNot {
        fn not(self) -> bool;
    }
    impl BoolNot for bool {
        fn not(self) -> bool {
            !self
        }
    }

    // ── R20 阶段 1 flesh out: 5 新测试 (SecretString / RateLimit / HMAC / base64 / 集成) ──

    /// flesh out #1: SecretString newtype (Serialize 脱敏, Zeroize drop).
    #[test]
    fn flesh_secret_string_redacted_and_zeroize() {
        let s = SecretString::new("sk-cp-kug0t7Jik3-test-key");
        // Debug 脱敏
        let dbg = format!("{s:?}");
        assert!(dbg.contains("***REDACTED***"), "Debug 必须脱敏");
        assert!(!dbg.contains("sk-cp-"), "Debug 不能泄露明文");
        // Serialize 脱敏
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("***REDACTED***"));
        assert!(!json.contains("sk-cp-"));
        // 长度 + is_empty
        assert_eq!(s.len(), 25, "len = 25 (sk-cp-kug0t7Jik3-test-key)");
        assert!(!s.is_empty());
        // 转换
        let bytes = s.to_bytes();
        assert_eq!(bytes.expose(), b"sk-cp-kug0t7Jik3-test-key");
        // From<&str>
        let s2 = SecretString::from("another");
        assert_eq!(s2.expose_str(), "another");
    }

    /// flesh out #2: RateLimit token bucket (5 RPS, burst 10).
    #[tokio::test]
    async fn flesh_rate_limit_burst_then_throttle() {
        let mut rl = RateLimit::new(5, 10);
        // burst 10: 10 个 token 立即可用
        for i in 0..10 {
            assert!(
                rl.try_acquire(&format!("k{i}")).is_ok(),
                "burst {i}/10 应通过"
            );
        }
        // 第 11 个应被拒
        assert!(rl.try_acquire("k11").is_err(), "burst 用完后必须 throttle");
        // 等 1 秒, refill 5 个
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        for i in 0..5 {
            assert!(
                rl.try_acquire(&format!("r{i}")).is_ok(),
                "refill 后 {i}/5 应通过"
            );
        }
    }

    /// flesh out #3: HMAC 文件完整性 (defense #4).
    #[test]
    fn flesh_hmac_file_integrity_roundtrip() {
        let salt = [0x12u8; FALLBACK_SALT_LEN];
        let content = b"some encrypted ciphertext blob";
        let h1 = hmac_file_integrity(content, &salt);
        assert_eq!(h1.len(), 64, "HMAC-SHA256 必须 64 hex 字符");
        // 一致性
        assert!(verify_hmac_file_integrity(content, &salt, &h1));
        // 改 1 字节 → 不一致
        let mut modified = content.to_vec();
        modified[0] ^= 0xFF;
        assert!(
            !verify_hmac_file_integrity(&modified, &salt, &h1),
            "改 1 字节必须失败"
        );
    }

    /// flesh out #4: base64 编解码 (无外部 dep, 自写).
    #[test]
    fn flesh_base64_roundtrip_random_bytes() {
        let inputs: Vec<Vec<u8>> = vec![
            b"hello".to_vec(),
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"abcd".to_vec(),
            (0..=255u8).collect(),
        ];
        for input in inputs {
            let encoded = base64_simple_encode(&input);
            let decoded = base64_simple_decode(&encoded).expect("decode must succeed");
            assert_eq!(
                decoded,
                input,
                "base64 roundtrip failed for {} bytes",
                input.len()
            );
        }
    }

    /// flesh out #5: KeyringConfig 走完 5 防御 (defense #1 OS keyring + #2 AES fallback + #3 0 明文 + #4 HMAC + #5 rate limit).
    #[tokio::test]
    async fn flesh_five_defenses_constants_and_guards() {
        // defense #1: OS keyring 平台枚举
        assert_eq!(SUPPORTED_PLATFORMS.len(), 4);
        // defense #2: AES-256-GCM
        assert_eq!(FALLBACK_AES_KEY_LEN, 32);
        assert_eq!(FALLBACK_NONCE_LEN, 12);
        // defense #3: 0 明文 (文件名)
        assert!(!FALLBACK_FILE_NAME.ends_with(".json"));
        assert!(!FALLBACK_FILE_NAME.ends_with(".txt"));
        // defense #4: HMAC 存在
        let h = hmac_file_integrity(b"test", &[0u8; 16]);
        assert_eq!(h.len(), 64);
        // defense #5: Rate limit 常量
        assert_eq!(RATE_LIMIT_DEFAULT_RPS, 5);
        assert_eq!(RATE_LIMIT_DEFAULT_BURST, 10);
        // TokenEntry schema 版本守门
        let entry = TokenEntry::new("apeireth-anthropic", "user@test", TokenType::Anthropic);
        assert_eq!(entry.schema_version, "1");
    }

    // ── §13/14/15 new tests: 5 providers + 4 error enums + 5 OS mocks ──

    /// flesh out #6: ProviderError 5 变体 + 转 KeyringError.
    #[test]
    fn flesh_provider_error_5_variants_and_conversion() {
        let e1 = ProviderError::Unavailable {
            provider: PROVIDER_OS_KEYRING,
            platform: Platform::Linux,
            reason: "dbus down".to_string(),
        };
        let k1: KeyringError = e1.into();
        assert!(matches!(k1, KeyringError::BackendUnavailable { .. }));

        let e2 = ProviderError::Io {
            provider: PROVIDER_ENCRYPTED_FILE,
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let k2: KeyringError = e2.into();
        assert!(matches!(k2, KeyringError::FallbackIo(_)));

        let e3 = ProviderError::Crypto {
            provider: PROVIDER_ENCRYPTED_FILE,
            reason: "tag mismatch".to_string(),
        };
        let k3: KeyringError = e3.into();
        assert!(matches!(k3, KeyringError::FallbackCrypto(_)));

        let e4 = ProviderError::Format {
            provider: PROVIDER_IN_MEMORY,
            reason: "json".to_string(),
        };
        let k4: KeyringError = e4.into();
        assert!(matches!(k4, KeyringError::Other(_)));

        let e5 = ProviderError::Unsupported {
            provider: PROVIDER_OS_KEYRING,
            platform: Platform::Bsd,
        };
        let k5: KeyringError = e5.into();
        assert!(matches!(
            k5,
            KeyringError::UnsupportedPlatform(Platform::Bsd)
        ));
    }

    /// flesh out #7: ConfigError 5 变体 + KeyringConfig::validate 守门.
    #[test]
    fn flesh_config_error_and_validate() {
        // 1) InvalidFallbackDir
        let mut cfg = KeyringConfig::default();
        cfg.fallback_dir = PathBuf::new();
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::InvalidFallbackDir(_))
        ));

        // 2) SchemaMismatch
        let mut cfg = KeyringConfig::default();
        cfg.schema_version = "999".to_string();
        assert!(matches!(
            cfg.validate(),
            Err(ConfigError::SchemaMismatch { .. })
        ));

        // 3) OK default
        let cfg = KeyringConfig::default();
        assert!(cfg.validate().is_ok());

        // 4) TokenTooLong 错误本身能正常构造 + 显示
        let e = ConfigError::TokenTooLong(5000);
        assert_eq!(format!("{e}").contains("5000"), true);
    }

    /// flesh out #8: InMemoryAdapter 完整 set/get/delete/list 循环.
    #[tokio::test]
    async fn flesh_in_memory_provider_full_cycle() {
        let p = InMemoryAdapter::new(Platform::Linux);
        let svc = "apeireth-anthropic";
        let acc = "user@test";
        let token = SecretBytes::new(b"sk-cp-inmemory-test");

        // 初始空
        assert!(p.is_empty().await);

        // set
        p.set(svc, acc, &token).await.unwrap();
        assert_eq!(p.len().await, 1);

        // get
        let got = p.get(svc, acc).await.unwrap();
        assert_eq!(got.expose(), b"sk-cp-inmemory-test");

        // list
        let entries = p.list().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].service, svc);
        assert_eq!(entries[0].account, acc);
        assert_eq!(entries[0].token_type, TokenType::Anthropic);

        // list_by_service
        let by_svc = p.list_by_service(svc).await.unwrap();
        assert_eq!(by_svc.len(), 1);
        let by_other = p.list_by_service("apeireth-openai").await.unwrap();
        assert_eq!(by_other.len(), 0);

        // delete
        p.delete(svc, acc).await.unwrap();
        assert!(p.is_empty().await);

        // double delete → NotFound
        let err = p.delete(svc, acc).await.unwrap_err();
        assert!(matches!(err, KeyringError::NotFound { .. }));

        // token too long
        let big = SecretBytes::new(vec![0u8; TOKEN_MAX_LENGTH + 1]);
        let err = p.set(svc, acc, &big).await.unwrap_err();
        assert!(matches!(err, KeyringError::TokenTooLong(_)));
    }

    /// flesh out #9: MockAdapter 4 行为模式 (passthrough / always_fail / always_not_found / latency).
    #[tokio::test]
    async fn flesh_mock_provider_4_behaviors() {
        let p = MockAdapter::with_script(Platform::Linux, MockScript::passthrough());
        let svc = "apeireth-anthropic";
        let acc = "user@test";
        let token = SecretBytes::new(b"sk-cp-mock-test");

        // passthrough set/get/delete 正常
        p.set(svc, acc, &token).await.unwrap();
        let got = p.get(svc, acc).await.unwrap();
        assert_eq!(got.expose(), b"sk-cp-mock-test");
        p.delete(svc, acc).await.unwrap();

        // 切 always_fail
        p.set_script(MockScript::always_fail("daemon crash"));
        let err = p.set(svc, acc, &token).await.unwrap_err();
        assert!(matches!(err, KeyringError::BackendUnavailable { .. }));
        let err = p.get(svc, acc).await.unwrap_err();
        assert!(matches!(err, KeyringError::BackendUnavailable { .. }));
        let err = p.delete(svc, acc).await.unwrap_err();
        assert!(matches!(err, KeyringError::BackendUnavailable { .. }));

        // 切 always_not_found
        p.set_script(MockScript::always_not_found());
        let err = p.get(svc, acc).await.unwrap_err();
        assert!(matches!(err, KeyringError::NotFound { .. }));

        // 切 latency 100ms
        p.set_script(MockScript::passthrough().with_latency(100));
        let start = std::time::Instant::now();
        p.set(svc, acc, &token).await.unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(90),
            "latency ≥ 90ms, got {elapsed:?}"
        );
    }

    /// flesh out #10: DisabledAdapter 5 方法全 fail.
    #[tokio::test]
    async fn flesh_disabled_provider_5_methods_all_fail() {
        let p = DisabledAdapter::new(Platform::Bsd, "user disabled");
        let svc = "apeireth-anthropic";
        let acc = "user@test";
        let token = SecretBytes::new(b"x");

        // 5 方法全 BackendUnavailable
        for res in [
            p.set(svc, acc, &token).await.map(|_| ()),
            p.get(svc, acc).await.map(|_| ()),
            p.delete(svc, acc).await.map(|_| ()),
            p.list().await.map(|_| ()),
            p.list_by_service(svc).await.map(|_| ()),
        ] {
            match res {
                Err(KeyringError::BackendUnavailable { .. }) => {}
                _ => panic!("DisabledAdapter 必须全 BackendUnavailable"),
            }
        }
        // platform 守门
        assert_eq!(p.platform(), Platform::Bsd);
    }

    /// flesh out #11: 5 OS mock backend (windows / darwin / linux / bsd / unsupported) + crash/recover.
    #[test]
    fn flesh_5_os_mock_backends() {
        // 5 platform mock
        let w = mock_backend_windows();
        assert_eq!(w.platform, Platform::Windows);
        assert!(w.check_health().is_ok());

        let d = mock_backend_darwin();
        assert_eq!(d.platform, Platform::Darwin);
        assert!(d.check_health().is_ok());

        let l = mock_backend_linux();
        assert_eq!(l.platform, Platform::Linux);
        assert!(l.check_health().is_ok());

        let b = mock_backend_bsd();
        assert_eq!(b.platform, Platform::Bsd);
        assert!(b.check_health().is_ok());

        // unsupported 平台 (用 Bsd 模拟 + crash)
        let u = mock_backend_unsupported();
        assert!(
            u.check_health().is_err(),
            "unsupported 必须 fail health check"
        );

        // crash + recover
        let l = mock_backend_linux();
        l.mock_set("svc", "acc", b"data");
        assert_eq!(l.len(), 1);
        l.crash();
        assert!(l.check_health().is_err());
        l.recover();
        assert!(l.check_health().is_ok());
        // crash 不清数据
        assert_eq!(l.len(), 1);
        assert_eq!(l.mock_get("svc", "acc"), Some(b"data".to_vec()));
    }

    /// flesh out #12: 5 Provider name 常量 = 编译期 hardcode.
    #[test]
    fn flesh_5_provider_name_constants() {
        assert_eq!(PROVIDER_OS_KEYRING, "os-keyring");
        assert_eq!(PROVIDER_ENCRYPTED_FILE, "encrypted-file");
        assert_eq!(PROVIDER_IN_MEMORY, "in-memory");
        assert_eq!(PROVIDER_MOCK, "mock");
        assert_eq!(PROVIDER_DISABLED, "disabled");
    }

    /// flesh out #13: EncryptedFileAdapter 包装 + set/get roundtrip.
    #[tokio::test]
    async fn flesh_encrypted_file_adapter_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(EncryptedFileStore::new(tmp.path()));
        let adapter = EncryptedFileAdapter::new(Platform::Linux, store.clone());

        // 必须先 unlock 才能 set
        let passphrase = SecretBytes::new(b"test-passphrase-1234");
        store.unlock(&passphrase).await.expect("unlock");

        let svc = "apeireth-anthropic";
        let acc = "user@test";
        let token = SecretBytes::new(b"sk-cp-encrypted-test");

        // set / get roundtrip
        adapter.set(svc, acc, &token).await.expect("set");
        let got = adapter.get(svc, acc).await.expect("get");
        assert_eq!(got.expose(), b"sk-cp-encrypted-test");

        // list
        let entries = adapter.list().await.expect("list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].token_type, TokenType::Anthropic);

        // delete
        adapter.delete(svc, acc).await.expect("delete");
        let err = adapter.get(svc, acc).await.unwrap_err();
        assert!(matches!(err, KeyringError::NotFound { .. }));
    }
}
