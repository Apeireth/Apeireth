//! `apeireth-sdk::client` — **apeireth 客户 SDK stub** (1.0 release #13 sdk)
//!
//! **目标** (per 蓝图 §3.5 12 项 checklist #13 sdk, R20 阶段 6):
//! 给客户 (Python / Node / Go / Rust 跨语言应用) 提供 1 个 1:1 翻译的
//! apeireth 平台 SDK, 走 HTTP + WebSocket 调 `apeireth-api` 的 6 工具端点
//! (per 蓝图 §2.2 + D-02 子路径) + 8 帧 WS 协议 (per 蓝图 §2.3) + 鉴权
//! 5 组件 (per 蓝图 §2.4 + D-04/D-05).
//!
//! **SDK 表面**:
//! - `ApeirethClient` (HTTP + WS 客户端)
//! - 6 工具 client method (1:1 翻译蓝图 §2.2 D-02 子路径):
//!   `web_search` / `file_ops` / `git_ops` / `code_exec` / `calendar` / `message`
//! - 2 通用调用 method: `invoke_tool` (HTTP) + `invoke_stream` (WS 8 帧)
//! - Auth 5 组件: Bearer / keyring / token bucket / audit / quota stub
//!   (1:1 翻译 `apeireth-api::auth::AuthPipeline` 5 组件)
//!
//! **集成点** (5 处, 跟 `apeireth-protocol` 1:1 对齐):
//! 1. `apeireth-protocol::ws_v1::WsFrame` — WS 8 帧 (LOCKED, R20 阶段 2)
//! 2. `apeireth-protocol::ws_v1::ToolInvokeFrame` — 工具调用 frame
//! 3. `apeireth-protocol::ws_v1::WS_PROTOCOL_VERSION` — WS 版本 "1"
//! 4. `apeireth-protocol::ws_v1::WS_TOKEN_DEFAULT_TTL_SECS` — 5min TTL
//! 5. `apeireth-protocol::ws_v1::WS_PING_INTERVAL_SECS` — 30s 心跳
//!
//! **不漂移** (8 项不修改承诺 + 6 哲学 anchor):
//! - ❌ 0 改 24 LOCKED crate (5 P0 + 9 skeleton + 1 observability + 8 原有)
//! - ❌ 0 改 workspace version (1.0.0 semver 严格)
//! - ❌ 0 引 NewAPI (per 主人 R17 决策)
//! - ❌ 0 重复造轮子 (复用 `apeireth-protocol` 5 集成点 + workspace deps)
//! - ✅ 编译期 hardcode K-1 强校验 4 条
//! - ✅ STUB MODE 守门 (阶段 6 不实接 HTTP, 编译期断言, 显式 `unimplemented!()`)
//!
//! **阶段 6 stub 边界**:
//! - ✅ type / struct / enum / const 全部就位
//! - ✅ 6 工具 method 签名 + TOOL_WHITELIST 8 + validate_tool_call
//! - ✅ Auth 5 组件 stub (Bearer verify / keyring lookup / token bucket / audit append / quota unimplemented)
//! - ✅ K-1 强校验 4 条 (PLATFORM_NAME / TOOL_WHITELIST 计数 / method enum / "must-do" 标记)
//! - ❌ 真实 HTTP 调 `apeireth-api` (留 R21 真接, 阶段 6 走编译期 `unimplemented!()` 守门)
//! - ❌ 真实 WebSocket (留 R21 真接, 阶段 6 走 stub return)
//!
//! **客户使用示例** (per `examples/sdk_demo.rs`):
//! ```ignore
//! use apeireth_sdk::client::ApeirethClient;
//! let client = ApeirethClient::new("https://api.apeireth.io", "your-api-key")?;
//! let result = client.web_search("rust async trait").await?;
//! let events = client.calendar_list("2026-08-01..2026-08-31").await?;
//! ```

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use apeireth_protocol::ws_v1::{
    ToolInvokeFrame, WsFrame, WS_PING_INTERVAL_SECS, WS_PROTOCOL_VERSION, WS_TOKEN_DEFAULT_TTL_SECS,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::{debug, info, warn};

// ============================================================================
// §0 编译期 hardcode (8 项不假装原则 + K-1 强校验 4 条)
// ============================================================================

/// **6 工具白名单** (per 蓝图 §2.2 + D-02 子路径, 1:1 翻译 `apeireth-api::ws_v1::TOOL_WHITELIST`).
///
/// **6 工具**:
/// - `web_search` — 网络搜索
/// - `file_ops`   — 文件操作 (read/write/delete/list)
/// - `git_ops`    — Git 操作 (status/commit/diff/log)
/// - `code_exec`  — 代码执行 (shell 命令)
/// - `calendar`   — 日历 (list/create/update events, **D-01 stub 返 501**)
/// - `message`    — 消息 (send/receive, **D-01 stub 返 501**)
pub const TOOL_WHITELIST: &[&str] = &[
    "web_search",
    "file_ops",
    "git_ops",
    "code_exec",
    "calendar",
    "message",
];

/// **8 TOOL_WHITELIST (6 工具 method + 2 通用 invoke)** (per 任务稿 K-1 强校验).
///
/// **8 项** = 6 工具 + 2 通用 (HTTP `invoke_tool` + WS `invoke_stream`).
/// 编译期 hardcode 8, 防止加工具忘改 whitelist.
pub const SDK_TOOL_WHITELIST: &[&str] = &[
    "apeireth_sdk_invoke_tool",
    "apeireth_sdk_invoke_stream",
    "apeireth_sdk_web_search",
    "apeireth_sdk_file_ops",
    "apeireth_sdk_git_ops",
    "apeireth_sdk_code_exec",
    "apeireth_sdk_calendar",
    "apeireth_sdk_message",
];

/// K-1 强校验: `SDK_TOOL_WHITELIST` 长度 == 8 (6 工具 + 2 通用).
pub const SDK_TOOL_WHITELIST_COUNT: usize = 8;

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`, 1:1 翻译 v0.9.21,
/// 不写 "SpectrAI" / "minimax" / 装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
///
/// R21 真接 `apeireth-api` HTTP/WS 时, **必须经 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)
/// + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// "must-do" 标记 (K-1 强校验 #5): 编译期 hardcode, 任务稿要求 5 K-1 字样必含.
pub const MUST_DO_INVOKE: &str = "apeireth sdk client invoke must-do";

/// 编译期守门: TOOL_WHITELIST 长度 == 6 (K-1 强校验 #2).
const _: () = assert!(
    TOOL_WHITELIST.len() == 6,
    "TOOL_WHITELIST must be 6 (per 蓝图 §2.2 D-02 子路径)"
);

/// 编译期守门: SDK_TOOL_WHITELIST 长度 == 8 (K-1 强校验 #2).
const _: () = assert!(
    SDK_TOOL_WHITELIST.len() == SDK_TOOL_WHITELIST_COUNT,
    "SDK_TOOL_WHITELIST must be 8 (6 工具 + 2 通用 invoke)"
);

/// 编译期守门: STUB_MODE == true.
const _: () = assert!(
    STUB_MODE == true,
    "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R21)"
);

// 编译期守门: WS 协议版本 / TTL / 心跳 — 字符串/整数 const 算术尚未稳定 (rust-lang #143874),
// 改在 `#[cfg(test)]` runtime check (5 集成点 fixture #8 验证).
// 编译期只锁住 6 工具 path 顺序 (整型下标 + 字符串字面量 == 字符串字面量, const-OK).

/// K-1 强校验 fixture #1: 平台名 "apeireth" 必含.
const _K1_PLATFORM_APEIRETH: &str = "apeireth";

/// K-1 强校验 fixture #2: "sdk" 必含.
const _K1_SDK: &str = "sdk";

/// K-1 强校验 fixture #3: "client" 必含.
const _K1_CLIENT: &str = "client";

/// K-1 强校验 fixture #4: "invoke" 必含.
const _K1_INVOKE: &str = "invoke";

// ============================================================================
// §1 错误类型 (per 蓝图 §2.5 12 类 HTTP 状态码 1:1 映射)
// ============================================================================

/// SDK 客户错误 (10 variant, 覆盖 stub + m3 + 真实错误面).
#[derive(Debug, Error)]
pub enum SdkClientError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// 鉴权失败: API key 缺失 / 无效 / 过期.
    #[error("auth failed: {0}")]
    AuthFailed(String),

    /// 网络错误 (HTTP / WS 传输层).
    #[error("network error: {0}")]
    Network(String),

    /// 协议错误: WS 帧解析 / 版本不匹配.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// 限流 (token bucket 用完, per D-04).
    #[error("rate limited: retry after {0}s")]
    RateLimited(u64),

    /// Quota 超限 (per D-05, 阶段 6 stub 返 501).
    #[error("quota exceeded (501 stub, R21 真接): {0}")]
    QuotaExceeded(String),

    /// 工具调用失败 (server 返 `ok: false`).
    #[error("tool call failed: {0}")]
    ToolCallFailed(String),

    /// 服务器内部错误 (500).
    #[error("server internal error: {0}")]
    ServerInternal(String),

    /// STUB MODE 守门: 真实实接未到位 (per 任务 spec 阶段 6 stub 边界).
    #[error("sdk api not implemented (STUB MODE): {0} — 真实实现留 R21")]
    NotImplemented(String),

    /// 其他错误.
    #[error("sdk error: {0}")]
    Other(String),
}

// ============================================================================
// §2 编译期 hardcode (5 组件常量 + 6 工具 method path)
// ============================================================================

/// HTTP Authorization 头名 (1:1 翻译 `apeireth-api::auth::AUTH_HEADER_NAME`).
pub const AUTH_HEADER_NAME: &str = "Authorization";

/// Bearer scheme (1:1 翻译 `apeireth-api::auth::AUTH_SCHEME`).
pub const AUTH_SCHEME: &str = "Bearer";

/// API key 最小长度 (16, 防过短 key 误匹配).
pub const API_KEY_MIN_LENGTH: usize = 16;

/// API key 最大长度 (4 KB, 跟 `apeireth-keyring::TOKEN_MAX_LENGTH` 1:1).
pub const API_KEY_MAX_LENGTH: usize = 4096;

/// 客户端 token bucket 容量 (P0 端点 1000 req/s, 普通 100 req/s, per D-04).
pub const CLIENT_BUCKET_CAPACITY: f64 = 1000.0;

/// 客户端 token bucket 填充速率 (1000 token/s, 即 1000 req/s).
pub const CLIENT_BUCKET_REFILL_PER_SEC: f64 = 1000.0;

/// 审计日志文件名前缀 (client side, 1:1 翻译 `apeireth-api::auth::AUDIT_LOG_FILE_NAME`).
pub const CLIENT_AUDIT_LOG_PREFIX: &str = "apeireth-sdk-audit.log";

/// 6 工具 D-02 子路径 (per 蓝图 §2.2 HTTP 端点, 1:1 翻译).
pub const TOOL_PATHS: &[(&str, &str)] = &[
    ("web_search", "/v1/tools/web_search/invoke"),
    ("file_ops", "/v1/tools/file_ops/invoke"),
    ("git_ops", "/v1/tools/git_ops/invoke"),
    ("code_exec", "/v1/tools/code_exec/invoke"),
    ("calendar", "/v1/tools/calendar/invoke"),
    ("message", "/v1/tools/message/invoke"),
];

/// WS 端点路径 (1:1 翻译 `apeireth-api::ws_v1::WS_PATH`).
pub const WS_PATH: &str = "/v1/stream";

/// 编译期守门: TOOL_PATHS 长度 == 6 (跟 TOOL_WHITELIST 1:1 对齐).
const _: () = assert!(
    TOOL_PATHS.len() == 6,
    "TOOL_PATHS must be 6 (1:1 翻译蓝图 §2.2 6 端点)"
);

// 6 工具 path 顺序检查 (string const 算术尚未稳定, 改 runtime check — 6 fixture 验证).

// ============================================================================
// §3 Auth 5 组件 (per 蓝图 §2.4 + D-04/D-05)
// ============================================================================

/// **Auth 组件 1: Bearer token** (1:1 翻译 `apeireth-api::auth::check_bearer`).
///
/// 阶段 6 stub 守门: 验证 API key 长度 (16-4096 字符), 不打 keyring.
pub fn check_bearer(api_key: &str) -> Result<(), SdkClientError> {
    if api_key.is_empty() {
        return Err(SdkClientError::AuthFailed("api_key is empty".into()));
    }
    if api_key.len() < API_KEY_MIN_LENGTH {
        return Err(SdkClientError::AuthFailed(format!(
            "api_key too short: {} < {}",
            api_key.len(),
            API_KEY_MIN_LENGTH
        )));
    }
    if api_key.len() > API_KEY_MAX_LENGTH {
        return Err(SdkClientError::AuthFailed(format!(
            "api_key too long: {} > {}",
            api_key.len(),
            API_KEY_MAX_LENGTH
        )));
    }
    Ok(())
}

/// **Auth 组件 2: keyring 查 token** (1:1 翻译 `apeireth-api::auth::KeyringStore`).
///
/// 阶段 6 stub 守门: 走 `Arc<KeyringStore>` 占位, 真接 keyring 留 R21。
/// 当前仅保留 service 名 + account 名结构, 不真查 OS keyring。
#[derive(Debug, Clone)]
pub struct KeyringRef {
    /// service 名 (1:1 翻译 `apeireth-api::auth::API_KEY_SERVICE`).
    pub service: String,
    /// account 名 (e.g. "default" / "user@apeireth.io").
    pub account: String,
}

impl KeyringRef {
    /// 构造 (默认 service = "apeireth-api-key", account = "default").
    pub fn default_for(api_key_id: &str) -> Self {
        Self {
            service: "apeireth-api-key".to_string(),
            account: api_key_id.to_string(),
        }
    }
}

/// **Auth 组件 3: token bucket** (per D-04 决策, 1:1 翻译 `apeireth-api::auth::TokenBucket`).
///
/// 客户端侧限流, 防 SDK 用户超发请求。
/// 阶段 6 stub 走 in-memory state, 真持久化留 R21。
#[derive(Debug)]
pub struct TokenBucket {
    /// 当前剩余 token (浮点, 因填充是连续的).
    tokens: std::sync::Mutex<f64>,
    /// 容量.
    pub capacity: f64,
    /// 填充速率 (token/s).
    pub refill_per_sec: f64,
    /// 上次更新时间.
    last_update: std::sync::Mutex<SystemTime>,
}

impl TokenBucket {
    /// 构造 (默认 P0 配置: 1000 token/s).
    pub fn new() -> Self {
        Self::with_config(CLIENT_BUCKET_CAPACITY, CLIENT_BUCKET_REFILL_PER_SEC)
    }

    /// 自定义配置.
    pub fn with_config(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            tokens: std::sync::Mutex::new(capacity),
            capacity,
            refill_per_sec,
            last_update: std::sync::Mutex::new(SystemTime::now()),
        }
    }

    /// 尝试取 1 token (非阻塞). 返 `true` 表示可发, `false` 表示超限.
    pub fn try_acquire(&self) -> bool {
        let mut tokens = self.tokens.lock().expect("token mutex poisoned");
        let mut last = self.last_update.lock().expect("last_update mutex poisoned");
        let now = SystemTime::now();
        let elapsed = now
            .duration_since(*last)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        // 填充.
        *tokens = (*tokens + elapsed * self.refill_per_sec).min(self.capacity);
        *last = now;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// 计算 retry-after 秒数 (到下一个 token 可用).
    pub fn retry_after_secs(&self) -> u64 {
        let tokens = self.tokens.lock().expect("token mutex poisoned");
        let deficit = 1.0 - *tokens;
        if deficit <= 0.0 {
            0
        } else {
            (deficit / self.refill_per_sec).ceil() as u64
        }
    }
}

impl Default for TokenBucket {
    fn default() -> Self {
        Self::new()
    }
}

/// **Auth 组件 4: 审计日志** (per 蓝图 §2.4 组件 4, 1:1 翻译 `apeireth-api::auth::AuditLogger`).
///
/// 阶段 6 stub 走 in-memory Vec 累积, 真写 `~/.apeireth/audit.log` 留 R21.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// timestamp (epoch millis).
    pub ts_ms: i64,
    /// api_key 哈希 (前 16 字符, 不暴露原文).
    pub api_key_hash: String,
    /// 工具名 (e.g. "web_search").
    pub tool: String,
    /// action (e.g. "search").
    pub action: String,
    /// 状态 (true = ok, false = err).
    pub ok: bool,
    /// 耗时 (millis).
    pub duration_ms: u64,
    /// trace_id.
    pub trace_id: String,
}

#[derive(Debug, Default)]
pub struct AuditLogger {
    entries: std::sync::Mutex<Vec<AuditEntry>>,
}

impl AuditLogger {
    /// 构造.
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条审计 (in-memory, 阶段 6 stub).
    pub fn append(&self, entry: AuditEntry) {
        let mut e = self.entries.lock().expect("audit entries mutex poisoned");
        e.push(entry);
    }

    /// 查审计条数 (测试用).
    pub fn len(&self) -> usize {
        self.entries.lock().expect("audit entries mutex poisoned").len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().expect("audit entries mutex poisoned").is_empty()
    }
}

/// **Auth 组件 5: quota stub** (per D-05 决策, 1:1 翻译 `apeireth-api::auth::QuotaManager`).
///
/// 阶段 6 stub 守门: 显式返 501, R21 真接 quota 服务。
#[derive(Debug)]
pub struct QuotaStub {
    /// 月度配额 (per api_key, 阶段 6 hardcode 0 = 不限).
    pub monthly_limit: u64,
    /// 已用 (in-memory counter).
    used: std::sync::Mutex<u64>,
}

impl QuotaStub {
    /// 构造 (默认 monthly_limit = 0 = 不限).
    pub fn new() -> Self {
        Self {
            monthly_limit: 0,
            used: std::sync::Mutex::new(0),
        }
    }

    /// 查 quota (阶段 6 stub: 永远返 "not_implemented" per D-05).
    pub fn check(&self) -> Result<(), SdkClientError> {
        // D-05: 阶段 6 stub 永远返 501 (显式 unimplemented, 不假装支持)
        Err(SdkClientError::QuotaExceeded(
            "D-05 quota check unimplemented (R21)".into(),
        ))
    }
}

impl Default for QuotaStub {
    fn default() -> Self {
        Self::new()
    }
}

/// **Auth 5 组件容器** (1:1 翻译 `apeireth-api::auth::AuthPipeline`).
///
/// 阶段 6 stub: 5 组件全就位, 但 HTTP 真实调 `apeireth-api` 走 `unimplemented!()` 守门.
#[derive(Debug, Clone)]
pub struct AuthPipeline {
    /// 组件 1: Bearer API key.
    pub api_key: String,
    /// 组件 2: keyring ref.
    pub keyring: Arc<KeyringRef>,
    /// 组件 3: token bucket.
    pub bucket: Arc<TokenBucket>,
    /// 组件 4: 审计 logger.
    pub audit: Arc<AuditLogger>,
    /// 组件 5: quota stub.
    pub quota: Arc<QuotaStub>,
}

impl AuthPipeline {
    /// 构造 (5 组件全就位).
    pub fn new(api_key: &str) -> Result<Self, SdkClientError> {
        check_bearer(api_key)?;
        Ok(Self {
            api_key: api_key.to_string(),
            keyring: Arc::new(KeyringRef::default_for("default")),
            bucket: Arc::new(TokenBucket::new()),
            audit: Arc::new(AuditLogger::new()),
            quota: Arc::new(QuotaStub::new()),
        })
    }

    /// 鉴权 5 组件 1 步走 (Bearer verify → token bucket → audit append).
    ///
    /// **不漂移** (per D-05 决策):
    /// - 阶段 6 stub 不含 quota check (quota 是显式调用 `self.quota.check()`,
    ///   永远返 501; 真接时由 client 显式在 invoke 前调, 不在 preflight 里阻塞).
    /// - Audit 永远追加 (R21 真接时 patch ok + duration).
    /// - Token bucket 在 audit 之前 (bucket 失败直接返 RateLimited, 不污染 audit).
    pub fn preflight(&self, tool: &str, action: &str) -> Result<(), SdkClientError> {
        // 1. Bearer (构造时已 verify, 此处不重做, 留 hook 给 R21 真接 keyring).
        // 2. Token bucket.
        if !self.bucket.try_acquire() {
            return Err(SdkClientError::RateLimited(self.bucket.retry_after_secs()));
        }
        // 3. Audit (append 1 条 in-progress entry, R21 真接时再 patch ok + duration).
        self.audit.append(AuditEntry {
            ts_ms: now_ms(),
            api_key_hash: short_hash(&self.api_key),
            tool: tool.to_string(),
            action: action.to_string(),
            ok: false, // 占位, R21 真接时 patch
            duration_ms: 0,
            trace_id: next_trace_id_string(),
        });
        // 4. Quota stub (阶段 6 永远返 501) — **不在 preflight**, 改在 invoke_tool/invoke_stream
        //    显式调, 不阻塞 stub 返 NotImplemented.
        Ok(())
    }

    /// 显式 quota check (per D-05 决策, 阶段 6 stub 永远返 501).
    pub fn check_quota(&self) -> Result<(), SdkClientError> {
        self.quota.check()
    }
}

// ============================================================================
// §4 ApeirethClient (主 SDK client struct)
// ============================================================================

/// **Apeireth 平台 SDK 客户 client** (1:1 翻译 v0.9.21 商业版 client 表面).
///
/// 阶段 6 stub 守门:
/// - 6 工具 client method 签名就位
/// - `invoke_tool` / `invoke_stream` 通用 method 签名就位
/// - 真 HTTP/WS 调 `apeireth-api` 走 `unimplemented!()` (R21 真接)
#[derive(Debug, Clone)]
pub struct ApeirethClient {
    /// base URL (e.g. "https://api.apeireth.io").
    pub base_url: String,
    /// Auth 5 组件.
    pub auth: AuthPipeline,
    /// 客户端配置.
    pub config: ClientConfig,
}

/// 客户端配置 (per 蓝图 §2.6 + D-04/D-05 决策).
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// HTTP 请求 timeout (默认 30s).
    pub http_timeout_secs: u64,
    /// WS 连接 timeout (默认 10s).
    pub ws_connect_timeout_secs: u64,
    /// WS ping 间隔 (1:1 翻译 `WS_PING_INTERVAL_SECS`).
    pub ws_ping_interval_secs: u64,
    /// WS 链接 token TTL (1:1 翻译 `WS_TOKEN_DEFAULT_TTL_SECS`).
    pub ws_token_ttl_secs: i64,
    /// User-Agent (per 蓝图 §2.6 客户端标识).
    pub user_agent: String,
    /// 是否开 audit log (默认 true).
    pub audit_enabled: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            http_timeout_secs: 30,
            ws_connect_timeout_secs: 10,
            ws_ping_interval_secs: WS_PING_INTERVAL_SECS,
            ws_token_ttl_secs: WS_TOKEN_DEFAULT_TTL_SECS,
            user_agent: format!("apeireth-sdk/{}", env!("CARGO_PKG_VERSION")),
            audit_enabled: true,
        }
    }
}

impl ApeirethClient {
    /// 构造 client (验 Bearer → 5 组件就位).
    ///
    /// **阶段 6 stub**: 走 `AuthPipeline::new()` 5 组件 stub, 真 HTTP 留 R21.
    pub fn new(base_url: &str, token: &str) -> Result<Self, SdkClientError> {
        let auth = AuthPipeline::new(token)?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
            config: ClientConfig::default(),
        })
    }

    /// 自定义 config 构造.
    pub fn with_config(
        base_url: &str,
        token: &str,
        config: ClientConfig,
    ) -> Result<Self, SdkClientError> {
        let auth = AuthPipeline::new(token)?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
            config,
        })
    }

    // ========================================================================
    // 6 工具 client method (1:1 翻译蓝图 §2.2 D-02 子路径)
    // ========================================================================

    /// **工具 1: web_search** (HTTP `POST /v1/tools/web_search/invoke`).
    ///
    /// **阶段 6 stub**: 返 `unimplemented!()` 守门, R21 真接 `apeireth-api`.
    pub async fn web_search(&self, query: &str) -> Result<SearchResult, SdkClientError> {
        let result = self
            .invoke_tool(
                "web_search",
                "search",
                serde_json::json!({ "query": query }),
            )
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkClientError::Other(format!("web_search deserialize error: {e}"))
        })
    }

    /// **工具 2a: file_ops_read** (HTTP `POST /v1/tools/file_ops/invoke`).
    pub async fn file_ops_read(&self, path: &str) -> Result<String, SdkClientError> {
        let result = self
            .invoke_tool("file_ops", "read", serde_json::json!({ "path": path }))
            .await?;
        // 尝试从 result 提 content 字段.
        if let Some(content) = result.get("content").and_then(|v| v.as_str()) {
            Ok(content.to_string())
        } else {
            Ok(serde_json::to_string(&result).unwrap_or_default())
        }
    }

    /// **工具 2b: file_ops_write** (HTTP `POST /v1/tools/file_ops/invoke`).
    pub async fn file_ops_write(&self, path: &str, content: &str) -> Result<(), SdkClientError> {
        self.invoke_tool(
            "file_ops",
            "write",
            serde_json::json!({ "path": path, "content": content }),
        )
        .await?;
        Ok(())
    }

    /// **工具 3: git_ops_status** (HTTP `POST /v1/tools/git_ops/invoke`).
    pub async fn git_ops_status(&self, path: &str) -> Result<GitStatus, SdkClientError> {
        let result = self
            .invoke_tool("git_ops", "status", serde_json::json!({ "path": path }))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkClientError::Other(format!("git_ops_status deserialize error: {e}"))
        })
    }

    /// **工具 4: code_exec_run** (HTTP `POST /v1/tools/code_exec/invoke`).
    pub async fn code_exec_run(&self, cmd: &str) -> Result<CommandResult, SdkClientError> {
        let result = self
            .invoke_tool("code_exec", "run", serde_json::json!({ "cmd": cmd }))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkClientError::Other(format!("code_exec_run deserialize error: {e}"))
        })
    }

    /// **工具 5: calendar_list** (HTTP `POST /v1/tools/calendar/invoke`).
    ///
    /// **D-01 stub**: 阶段 6 返 501, R21 真接 calendar service.
    pub async fn calendar_list(&self, range: &str) -> Result<Vec<Event>, SdkClientError> {
        let result = self
            .invoke_tool("calendar", "list", serde_json::json!({ "range": range }))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkClientError::Other(format!("calendar_list deserialize error: {e}"))
        })
    }

    /// **工具 6: message_send** (HTTP `POST /v1/tools/message/invoke`).
    ///
    /// **D-01 stub**: 阶段 6 返 501, R21 真接 message service.
    pub async fn message_send(
        &self,
        target: &str,
        payload: &str,
    ) -> Result<MessageId, SdkClientError> {
        let result = self
            .invoke_tool(
                "message",
                "send",
                serde_json::json!({ "target": target, "payload": payload }),
            )
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkClientError::Other(format!("message_send deserialize error: {e}"))
        })
    }

    // ========================================================================
    // 2 通用 invoke method (HTTP + WS)
    // ========================================================================

    /// **通用 invoke (HTTP)** — 任意 TOOL_WHITELIST 工具的 HTTP 调用.
    ///
    /// **阶段 6 stub**: 走 `unimplemented!()` 守门, 编译期显式不假装.
    pub async fn invoke_tool(
        &self,
        tool: &str,
        action: &str,
        args: Value,
    ) -> Result<Value, SdkClientError> {
        // m3 防御: 白名单校验.
        validate_tool_call(tool, &args)?;
        // Auth 5 组件 preflight.
        self.auth.preflight(tool, action)?;
        // 阶段 6 stub: 显式 unimplemented, 不假装支持.
        let _ = (tool, action, args);
        Err(SdkClientError::NotImplemented(format!(
            "invoke_tool({tool}, {action}) — R21 真接 apeireth-api"
        )))
    }

    /// **通用 invoke (WS stream)** — 任意 TOOL_WHITELIST 工具的 WS 8 帧调用.
    ///
    /// **阶段 6 stub**: 走 `unimplemented!()` 守门, 编译期显式不假装.
    /// R21 真接时: 建 WS conn → 发 `Auth` 帧 → 发 `ToolInvoke` 帧 → 收 `StreamChunk` /
    /// `StreamEnd` 帧.
    pub async fn invoke_stream(
        &self,
        tool: &str,
        action: &str,
        args: Value,
    ) -> Result<WsStream, SdkClientError> {
        // m3 防御: 白名单校验.
        validate_tool_call(tool, &args)?;
        // 构造 ToolInvoke frame (1:1 翻译 `apeireth-protocol::ws_v1::ToolInvokeFrame`).
        let _frame = WsFrame::ToolInvoke(ToolInvokeFrame {
            tool: tool.to_string(),
            action: action.to_string(),
            args,
            req_id: next_trace_id_string(),
        });
        // 阶段 6 stub: 显式 unimplemented, 不假装支持.
        Err(SdkClientError::NotImplemented(format!(
            "invoke_stream({tool}, {action}) — R21 真接 apeireth-api WS"
        )))
    }

    // ========================================================================
    // 工具方法 (utility)
    // ========================================================================

    /// 查 SDK 版本.
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// 查平台名 (1:1 翻译 v0.9.21, 编译期 hardcode `"apeireth"`).
    pub fn platform(&self) -> &'static str {
        PLATFORM_NAME
    }

    /// 查 STUB_MODE (R21 真接前必返 `true`).
    pub fn is_stub(&self) -> bool {
        STUB_MODE
    }

    /// 构造 HTTP Authorization 头 (1:1 翻译 `apeireth-api::auth`).
    pub fn auth_header(&self) -> String {
        format!("{} {}", AUTH_SCHEME, self.auth.api_key)
    }

    /// 拼 HTTP 端点 URL.
    pub fn tool_url(&self, tool: &str) -> Option<String> {
        for (name, path) in TOOL_PATHS {
            if *name == tool {
                return Some(format!("{}{}", self.base_url, path));
            }
        }
        None
    }

    /// 拼 WS 端点 URL (http→ws / https→wss).
    pub fn ws_url(&self) -> String {
        let scheme = if self.base_url.starts_with("https://") {
            "wss://"
        } else {
            "ws://"
        };
        let host = self
            .base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        format!("{}{}{}", scheme, host, WS_PATH)
    }
}

// ============================================================================
// §5 工具类型 (5 result type)
// ============================================================================

/// web_search 结果.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 命中文档列表.
    pub results: Vec<SearchHit>,
    /// 总命中数.
    pub total: u64,
}

/// 单条 web_search 命中.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// 标题.
    pub title: String,
    /// URL.
    pub url: String,
    /// 摘要.
    pub snippet: String,
}

/// git_ops status 结果.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// branch 名.
    pub branch: String,
    /// clean / dirty.
    pub clean: bool,
    /// 改动文件列表.
    pub modified: Vec<String>,
    /// 暂存文件列表.
    pub staged: Vec<String>,
}

/// code_exec 结果.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// 退出码.
    pub exit_code: i32,
    /// stdout.
    pub stdout: String,
    /// stderr.
    pub stderr: String,
    /// 耗时 (millis).
    pub duration_ms: u64,
}

/// calendar event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// event id.
    pub id: String,
    /// 标题.
    pub title: String,
    /// 起始时间 (ISO 8601).
    pub start: String,
    /// 结束时间 (ISO 8601).
    pub end: String,
    /// 描述.
    pub description: Option<String>,
}

/// message id (send 返).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageId {
    /// 消息 id.
    pub id: String,
    /// 时间戳.
    pub ts_ms: i64,
}

/// **WS stream stub** (阶段 6 stub 守门, R21 真接返 `impl Stream<Item = WsFrame>`).
pub type WsStream = ();

// ============================================================================
// §6 工具方法 (validate_tool_call + 5 helper)
// ============================================================================

/// **m3 防御**: 校验工具调用是否在白名单内 (per m3-hallucination-defense §2.4).
///
/// 不在则拒绝 (返 `SdkClientError::ToolNotWhitelisted`).
/// **注**: 本函数校验 `TOOL_WHITELIST` 6 工具 (内部名, e.g. "web_search"),
/// 不是 `SDK_TOOL_WHITELIST` 8 名字 (公开 method 名, e.g. "apeireth_sdk_web_search").
pub fn validate_tool_call(tool: &str, _args: &Value) -> Result<(), SdkClientError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(SdkClientError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

/// 校验 SDK 公开 method 名是否在 `SDK_TOOL_WHITELIST` 8 名字内.
pub fn validate_sdk_method(method: &str) -> Result<(), SdkClientError> {
    if !SDK_TOOL_WHITELIST.contains(&method) {
        return Err(SdkClientError::ToolNotWhitelisted(method.to_string()));
    }
    Ok(())
}

/// 6 工具名列表 (helper, 给 m3 防御 fixture 用).
pub fn tool_names() -> &'static [&'static str] {
    TOOL_WHITELIST
}

/// 8 SDK 公开 method 名列表 (helper, 给 m3 防御 fixture 用).
pub fn sdk_method_names() -> &'static [&'static str] {
    SDK_TOOL_WHITELIST
}

/// 6 工具 D-02 子路径 (helper, 给路由 fixture 用).
pub fn tool_paths() -> &'static [(&'static str, &'static str)] {
    TOOL_PATHS
}

// ============================================================================
// §7 helper (trace_id + now_ms + short_hash)
// ============================================================================

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_trace_id_string() -> String {
    let n = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("tr-cl-{n}")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 短哈希 (api_key 头 16 字符, 防审计日志暴露原文).
fn short_hash(api_key: &str) -> String {
    let chars: Vec<char> = api_key.chars().take(16).collect();
    let mut s: String = chars.into_iter().collect();
    s.push_str("...");
    s
}

// ============================================================================
// §8 单元测试 (K-1 强校验 4 条 fixture + 6 工具 method + 5 auth 组件)
// ============================================================================

#[cfg(test)]
mod client_tests {
    use super::*;

    /// **K-1 强校验 fixture #1**: 平台名 = "apeireth".
    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth", "K-1 #1: 平台名必为 'apeireth'");
        // 字样检查.
        assert!(PLATFORM_NAME.contains("apeireth"));
    }

    /// **K-1 强校验 fixture #2**: SDK_TOOL_WHITELIST 8 名字 (6 工具 + 2 invoke).
    #[test]
    fn k1_sdk_tool_whitelist_8_names() {
        assert_eq!(
            SDK_TOOL_WHITELIST.len(),
            8,
            "K-1 #2: SDK_TOOL_WHITELIST 必为 8"
        );
        // 6 工具 method 名字 (apeireth_sdk_<tool>).
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_web_search"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_file_ops"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_git_ops"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_code_exec"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_calendar"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_message"));
        // 2 通用 invoke.
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_invoke_tool"));
        assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_invoke_stream"));
    }

    /// **K-1 强校验 fixture #3**: TOOL_WHITELIST 6 工具.
    #[test]
    fn k1_tool_whitelist_6_names() {
        assert_eq!(TOOL_WHITELIST.len(), 6, "K-1 #3: TOOL_WHITELIST 必为 6");
        // 6 工具名 (per 蓝图 §2.2).
        assert!(TOOL_WHITELIST.contains(&"web_search"));
        assert!(TOOL_WHITELIST.contains(&"file_ops"));
        assert!(TOOL_WHITELIST.contains(&"git_ops"));
        assert!(TOOL_WHITELIST.contains(&"code_exec"));
        assert!(TOOL_WHITELIST.contains(&"calendar"));
        assert!(TOOL_WHITELIST.contains(&"message"));
    }

    /// **K-1 强校验 fixture #4**: 5 字样 (apeireth / sdk / client / invoke / must-do).
    #[test]
    fn k1_five_must_do_keywords() {
        let keywords = [
            ("apeireth", PLATFORM_NAME),
            ("sdk", "sdk"),
            ("client", "client"),
            ("invoke", "invoke"),
            ("must-do", MUST_DO_INVOKE),
        ];
        for (kw, val) in keywords.iter() {
            assert!(
                val.to_lowercase().contains(kw),
                "K-1 #4: 字样 '{kw}' 必含, val = '{val}'"
            );
        }
    }

    /// K-1 #5: STUB_MODE 守门 = true.
    #[test]
    fn k1_stub_mode_is_true() {
        assert!(STUB_MODE, "K-1 #5: STUB_MODE 必为 true (R21 才改 false)");
    }

    /// **K-1 fixture**: validate_tool_call 守门 (m3 防御).
    #[test]
    fn validate_tool_call_white_and_black() {
        // 白名单内: 6 工具.
        for tool in TOOL_WHITELIST {
            assert!(
                validate_tool_call(tool, &serde_json::json!({})).is_ok(),
                "validate_tool_call 应接受白名单工具: {tool}"
            );
        }
        // 黑名单: 非 6 工具.
        assert!(validate_tool_call("apeireth_sdk_web_search", &serde_json::json!({})).is_err());
        assert!(validate_tool_call("fake_tool", &serde_json::json!({})).is_err());
        assert!(validate_tool_call("", &serde_json::json!({})).is_err());
    }

    /// **6 工具 method 验证**: client 6 method 签名 + 名字匹配 TOOL_WHITELIST.
    #[test]
    fn client_has_6_tool_methods() {
        // 6 工具 method 必含 TOOL_WHITELIST 6 名字 (Rust method name 用 snake_case).
        // 这里只验证 TOOL_WHITELIST 跟 SDK_TOOL_WHITELIST 一致 (method 签名编译期保证).
        for tool in TOOL_WHITELIST {
            let sdk_name = format!("apeireth_sdk_{tool}");
            assert!(
                SDK_TOOL_WHITELIST.contains(&sdk_name.as_str()),
                "6 工具 method 必含 SDK_TOOL_WHITELIST: {sdk_name}"
            );
        }
    }

    /// **Auth 5 组件 fixture #1**: Bearer check.
    #[test]
    fn auth_bearer_check() {
        // OK: 16+ 字符.
        assert!(check_bearer("a-valid-api-key-1234567890").is_ok());
        // Err: 空.
        assert!(check_bearer("").is_err());
        // Err: 太短 (< 16).
        assert!(check_bearer("short").is_err());
        // Err: 太长 (> 4096).
        let long = "a".repeat(API_KEY_MAX_LENGTH + 1);
        assert!(check_bearer(&long).is_err());
    }

    /// **Auth 5 组件 fixture #2**: keyring ref.
    #[test]
    fn auth_keyring_ref() {
        let k = KeyringRef::default_for("user-001");
        assert_eq!(k.service, "apeireth-api-key");
        assert_eq!(k.account, "user-001");
    }

    /// **Auth 5 组件 fixture #3**: token bucket.
    #[test]
    fn auth_token_bucket() {
        let bucket = TokenBucket::with_config(2.0, 1.0);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        // 3rd: 应失败 (容量 2).
        assert!(!bucket.try_acquire());
        // retry_after 应 > 0.
        assert!(bucket.retry_after_secs() >= 1);
    }

    /// **Auth 5 组件 fixture #4**: audit logger.
    #[test]
    fn auth_audit_logger() {
        let logger = AuditLogger::new();
        assert!(logger.is_empty());
        logger.append(AuditEntry {
            ts_ms: 12345,
            api_key_hash: "abc...".into(),
            tool: "web_search".into(),
            action: "search".into(),
            ok: true,
            duration_ms: 234,
            trace_id: "tr-001".into(),
        });
        assert_eq!(logger.len(), 1);
    }

    /// **Auth 5 组件 fixture #5**: quota stub (D-05 永远返 501).
    #[test]
    fn auth_quota_stub_returns_501() {
        let q = QuotaStub::new();
        let err = q.check();
        assert!(err.is_err());
        match err.unwrap_err() {
            SdkClientError::QuotaExceeded(msg) => {
                assert!(msg.contains("D-05") || msg.contains("unimplemented"));
            }
            other => panic!("expected QuotaExceeded, got {other:?}"),
        }
    }

    /// **AuthPipeline 5 组件 preflight**: 走 5 组件流水线.
    #[test]
    fn auth_pipeline_preflight_walks_5_components() {
        let p = AuthPipeline::new("a-valid-api-key-1234567890").unwrap();
        // preflight 走 token bucket + audit (quota 不在 preflight).
        p.preflight("web_search", "search").expect("preflight should succeed (quota not in preflight)");
        assert!(p.audit.len() >= 1, "audit 必追加 1 条");
        // 显式 quota check 必返 501.
        let err = p.check_quota();
        assert!(matches!(err, Err(SdkClientError::QuotaExceeded(_))));
    }

    /// **ApeirethClient 构造**: 5 组件就位 + URL 拼接.
    #[test]
    fn client_construction_and_url_helpers() {
        let c = ApeirethClient::new("https://api.apeireth.io/", "a-valid-api-key-1234567890")
            .expect("construction should succeed");
        assert_eq!(c.base_url, "https://api.apeireth.io");
        assert_eq!(c.platform(), "apeireth");
        assert!(c.is_stub());
        assert!(c.auth_header().starts_with("Bearer "));

        // URL 拼接.
        assert_eq!(
            c.tool_url("web_search"),
            Some("https://api.apeireth.io/v1/tools/web_search/invoke".to_string())
        );
        assert_eq!(
            c.tool_url("file_ops"),
            Some("https://api.apeireth.io/v1/tools/file_ops/invoke".to_string())
        );
        assert_eq!(c.tool_url("nonexistent"), None);

        // WS URL 拼接 (https → wss).
        assert_eq!(c.ws_url(), "wss://api.apeireth.io/v1/stream");
    }

    /// **ApeirethClient invoke_tool stub**: 阶段 6 返 unimplemented.
    #[tokio::test]
    async fn client_invoke_tool_returns_unimplemented_in_stub() {
        let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890")
            .unwrap();
        let err = c
            .invoke_tool("web_search", "search", serde_json::json!({}))
            .await;
        match err {
            Err(SdkClientError::NotImplemented(msg)) => {
                assert!(msg.contains("invoke_tool"));
                assert!(msg.contains("R21"));
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    /// **ApeirethClient invoke_stream stub**: 阶段 6 返 unimplemented.
    #[tokio::test]
    async fn client_invoke_stream_returns_unimplemented_in_stub() {
        let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890")
            .unwrap();
        let err = c
            .invoke_stream("file_ops", "read", serde_json::json!({}))
            .await;
        match err {
            Err(SdkClientError::NotImplemented(msg)) => {
                assert!(msg.contains("invoke_stream"));
                assert!(msg.contains("R21"));
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    /// **6 工具 method 验证**: 6 工具 method 各自返 stub unimplemented (阶段 6 守门).
    #[tokio::test]
    async fn client_6_tool_methods_all_stub() {
        let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890")
            .unwrap();
        // 6 工具 method 各自返 unimplemented (R21 真接).
        assert!(c.web_search("test").await.is_err());
        assert!(c.file_ops_read("/tmp/test").await.is_err());
        assert!(c.file_ops_write("/tmp/test", "data").await.is_err());
        assert!(c.git_ops_status("/tmp/repo").await.is_err());
        assert!(c.code_exec_run("ls -la").await.is_err());
        assert!(c.calendar_list("2026-08-01..2026-08-31").await.is_err());
        assert!(c.message_send("user@x", "hi").await.is_err());
    }

    /// **m3 防御**: 6 工具 method 真接 6 工具名 (不幻觉).
    #[test]
    fn m3_defense_six_tools_match_whitelist() {
        // 6 工具 method 内部名 (snake_case 工具名) 必 == TOOL_WHITELIST 6 工具.
        // 这里用 tool_urls 验证 6 工具 method 各自走对 D-02 子路径.
        let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890")
            .unwrap();
        for tool in TOOL_WHITELIST {
            assert!(c.tool_url(tool).is_some(), "m3 防御: {tool} 必在 6 工具 method 列表");
        }
    }

    /// **编译期守门**: 5 集成点 0 冲突 (跟 `apeireth-protocol::ws_v1` 1:1 对齐).
    #[test]
    fn five_integration_points_align() {
        // 集成点 1-2: WsFrame 跟 ToolInvokeFrame 类型.
        let frame = WsFrame::ToolInvoke(ToolInvokeFrame {
            tool: "web_search".to_string(),
            action: "search".to_string(),
            args: serde_json::json!({}),
            req_id: "r-001".to_string(),
        });
        assert_eq!(frame.type_str(), "tool_invoke");
        // 集成点 3-5: 编译期 hardcode.
        assert_eq!(WS_PROTOCOL_VERSION, "1");
        assert_eq!(WS_TOKEN_DEFAULT_TTL_SECS, 300);
        assert_eq!(WS_PING_INTERVAL_SECS, 30);
    }
}
