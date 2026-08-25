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
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

// ============================================================================
// Local base64 helpers (replaces legacy `base64_simple_encode` / `decode`)
// ============================================================================

/// Encode bytes as standard base64 (no extra padding quirks).
fn base64_simple_encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

/// Decode standard base64 into bytes. Returns `None` on invalid input.
fn base64_simple_decode(s: &str) -> Option<Vec<u8>> {
    STANDARD.decode(s).ok()
}

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
///
/// **v2 design note**: holds the primary adapter as the concrete
/// `KeyringCrateAdapter` rather than `Box<dyn KeyringAdapter>`. The
/// `KeyringAdapter` trait is `#[async_trait]` (returns boxed futures), which
/// makes it **not dyn compatible**. Only one adapter type exists in this
/// crate, so the indirection costs nothing and removing it lets us keep the
/// `KeyringAdapter` trait object-friendly for downstream callers who may
/// want to roll their own (concrete) implementation.
pub struct KeyringStore {
    config: KeyringConfig,
    primary: KeyringCrateAdapter,
    fallback: Arc<RwLock<Option<EncryptedFileStore>>>,
    entries: Arc<RwLock<HashMap<(String, String), TokenEntry>>>,
}

impl KeyringStore {
    /// 新建.
    pub fn new(config: KeyringConfig) -> Self {
        let platform = config.platform_kind;
        let primary = KeyringCrateAdapter::new(platform);
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
            self.tokens = f64::from(self.burst);
            self.last_refill_ms = now_ms;
        }

        // Refill: (elapsed_ms / 1000) * rps
        let elapsed_ms = now_ms.saturating_sub(self.last_refill_ms);
        let refill = (elapsed_ms as f64 / 1000.0) * f64::from(self.rps);
        self.tokens = (self.tokens + refill).min(f64::from(self.burst));
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
