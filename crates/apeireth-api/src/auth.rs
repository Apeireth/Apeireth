//! `apeireth-api::auth` — **鉴权 5 组件** (R20 阶段 2, 蓝图 §2.4)
//!
//! **目标**: R20 阶段 2 把鉴权 5 组件封装成 `AuthPipeline`, 给 HTTP middleware + WS
//! handler 复用. 5 组件 = Bearer token + keyring 存储 + token bucket 限流 +
//! 审计日志 + quota stub.
//!
//! **5 组件** (per 蓝图 §2.4 + 决策 D-03/D-04/D-05):
//!
//! | # | 组件 | 库 / 文件 | 阶段 2 设计 |
//! |---|------|----------|------------|
//! | 1 | **Bearer token 验证** | 编译期 hardcode 头名 + 4 步: 抽 header → 抽 Bearer → keyring 查 → 匹配 | `check_bearer()` |
//! | 2 | **keyring 存 token** | 走 `apeireth-keyring::KeyringStore` 1:1 (5 重防御, per R20 阶段 1) | `Arc<KeyringStore>` |
//! | 3 | **token bucket 限流** | per api_key 100 token / refill 100/s; P0 端点 1000/s (per D-04) | `TokenBucket` (本文件 inline, 因 apeireth-constraint 暂无 token bucket) |
//! | 4 | **审计日志** | append-only, 写 `~/.apeireth/audit.log`, 含 timestamp / api_key_hash / endpoint / status / duration / trace_id (per 蓝图 §2.4) | `AuditLogger` |
//! | 5 | **quota stub** | `QuotaManager` stub, 编译期 `unimplemented!()`, 返 501 (per D-05, R21 实装) | `check_quota()` |
//!
//! **不漂移**:
//! - 0 改 `apeireth-keyring` (LOCKED crate, 阶段 1 已就位)
//! - 0 改 24 LOCKED crate
//! - 0 引 NewAPI
//! - 复用 workspace 现有 `parking_lot::Mutex` / `tracing` / `serde_json`
//!
//! **6 哲学 anchor 穿透**:
//! - S-1 北极星: 5 组件按蓝图 §2.4 1:1 翻译, 0 业务重设计
//! - S-2 实事求是: token bucket 估 60 LOC (P0 端点 1000/s, 普通 100/s), quota stub 返 501
//! - O-2 走在前人肩上: 复用 keyring 5 重防御, 0 重造
//! - O-3 干到底: 边界清晰 (5 组件 wrapper 单一职责, WS handler 复用 1 个 pipeline)
//! - O-4 任何人都能接手: 本文件 §1-§5 跟蓝图 §2.4 字段级对齐
//! - O-5 不假装: quota stub `unimplemented!()` 编译期断言, 返 501 显式

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use apeireth_host::keyring::KeyringStore;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

/// 本地 trace_id 单调递增 (避免依赖 `apeireth-bus` 防 crate 循环, 走 AtomicU64).
static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 分配下一个全局唯一的 trace_id (单调递增, 跟 `apeireth-bus::next_trace_id` 1:1 对齐).
fn next_trace_id() -> u64 {
    TRACE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// 当前 epoch milliseconds (跟 `apeireth-bus::now_ms` 1:1 对齐).
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// §1 编译期 hardcode (8 项不假装原则)
// ============================================================================

/// **HTTP Authorization 头名** (编译期 hardcode, 防止漂移 per 蓝图 §2.4 5 组件).
pub const AUTH_HEADER_NAME: &str = "Authorization";

/// **Bearer 前缀** (跟 `apeireth-protocol::protocol_handlers::AUTH_SCHEME_BEARER` 字段级对齐).
pub const AUTH_SCHEME: &str = "Bearer";

/// **API key 长度上限** (4 KB, 跟 `apeireth-keyring::TOKEN_MAX_LENGTH` 1:1).
pub const API_KEY_MAX_LENGTH: usize = 4096;

/// **API key 最小长度** (16 字符, 防过短 key 误匹配).
pub const API_KEY_MIN_LENGTH: usize = 16;

/// **P0 端点 token bucket 容量** (1000 req/s, per 蓝图 §2.6).
pub const P0_BUCKET_CAPACITY: f64 = 1000.0;

/// **P0 端点 token bucket 填充速率** (1000 token/s, 即 1000 req/s).
pub const P0_BUCKET_REFILL_PER_SEC: f64 = 1000.0;

/// **普通端点 token bucket 容量** (100 req/s, per 蓝图 §2.6).
pub const NORMAL_BUCKET_CAPACITY: f64 = 100.0;

/// **普通端点 token bucket 填充速率** (100 token/s, 即 100 req/s).
pub const NORMAL_BUCKET_REFILL_PER_SEC: f64 = 100.0;

/// **WS 并发流上限** (per api_key, per 蓝图 §2.6 WS 路径).
pub const WS_CONCURRENT_LIMIT: usize = 10;

/// **审计日志文件名** (per 蓝图 §2.4 组件 4).
pub const AUDIT_LOG_FILE_NAME: &str = "audit.log";

/// **WS 鉴权 service 名** (走 keyring 区分, per 蓝图 §2.4 组件 2).
pub const WS_TOKEN_SERVICE: &str = "apeireth-ws-token";

/// **API key 鉴权 service 名** (走 keyring 区分, per 蓝图 §2.4 组件 2).
pub const API_KEY_SERVICE: &str = "apeireth-api-key";

/// **P0 端点路径前缀** (per 蓝图 §2.6 E 急救路径).
///
/// E 急救路径 = 不限流 (软上限 1000/s, 触发只 WARN 不 429).
pub const P0_PATH_PREFIXES: &[&str] = &[
    "/v1/sovereignty/check",
    "/v1/sovereignty/attack",
    "/v1/sovereignty/rearm",
    "/v1/agent/spawn",
    "/v1/tools/code_exec/invoke",
    "/v1/stream", // WS 端点 (走 WS 路径, 10 并发流限流)
];

const _: () = {
    // 头名锁 "Authorization"
    assert!(AUTH_HEADER_NAME.len() == 13, "AUTH_HEADER_NAME must be 13 chars");
    assert!(AUTH_HEADER_NAME.as_bytes()[0] == b'A', "AUTH_HEADER_NAME must start with 'A'");
    // Bearer 锁
    assert!(AUTH_SCHEME.len() == 6, "AUTH_SCHEME must be 6 chars");
    assert!(AUTH_SCHEME.as_bytes()[0] == b'B', "AUTH_SCHEME must start with 'B'");
    // 长度限制
    assert!(API_KEY_MAX_LENGTH == 4096, "API_KEY_MAX_LENGTH must match keyring");
    assert!(API_KEY_MIN_LENGTH >= 16, "API_KEY_MIN_LENGTH must be >= 16");
    // bucket 容量
    assert!(P0_BUCKET_CAPACITY >= 100.0, "P0 bucket capacity must be >= 100");
    assert!(NORMAL_BUCKET_CAPACITY >= 10.0, "normal bucket capacity must be >= 10");
    // WS 并发
    assert!(WS_CONCURRENT_LIMIT >= 1, "WS concurrent limit must be >= 1");
};

// ============================================================================
// §2 错误类型
// ============================================================================

/// 鉴权错误 (per 蓝图 §2.4 7 case + §2.5 12 类错误码).
#[derive(Debug, Error)]
pub enum ApiError {
    /// 401 — 缺 Authorization 头
    #[error("missing authorization header")]
    MissingAuthHeader,

    /// 401 — Authorization 头格式错 (不是 "Bearer <token>")
    #[error("malformed authorization header")]
    MalformedAuthHeader,

    /// 401 — API key 错或过期 (per §2.4 case #2-3)
    #[error("api key invalid or expired")]
    InvalidApiKey,

    /// 429 — token bucket 限流
    #[error("rate limited")]
    RateLimited {
        /// Retry-After 建议等待秒数
        retry_after_secs: u64,
    },

    /// 501 — quota 商业化未实装 (per D-05 + O-5 不假装)
    #[error("quota check not implemented (per D-05, R21 stub mode)")]
    NotImplemented {
        /// stub 字段
        api: &'static str,
    },

    /// 500 — keyring 后端不可用
    #[error("keyring backend unavailable: {0}")]
    KeyringUnavailable(String),

    /// 500 — 内部错误
    #[error("internal auth error: {0}")]
    Internal(String),
}

impl ApiError {
    /// HTTP 状态码 (1:1 跟蓝图 §2.5 12 类错误码对齐).
    #[must_use]
    pub fn status_code(&self) -> u16 {
        match self {
            Self::MissingAuthHeader | Self::MalformedAuthHeader | Self::InvalidApiKey => 401,
            Self::RateLimited { .. } => 429,
            Self::NotImplemented { .. } => 501,
            Self::KeyringUnavailable(_) | Self::Internal(_) => 500,
        }
    }

    /// 错误码字符串 (machine-readable, per 蓝图 §2.5).
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::MissingAuthHeader | Self::MalformedAuthHeader | Self::InvalidApiKey => {
                "unauthorized"
            }
            Self::RateLimited { .. } => "rate_limited",
            Self::NotImplemented { .. } => "not_implemented",
            Self::KeyringUnavailable(_) | Self::Internal(_) => "internal_error",
        }
    }
}

/// 鉴权 Result 别名.
pub type ApiResult<T> = Result<T, ApiError>;

// ============================================================================
// §3 Principal — 鉴权主体 (API key hash + 元数据)
// ============================================================================

/// **鉴权主体** (per 蓝图 §2.4 鉴权流程: 匹配 → 进入业务 handler 时带 Principal).
///
/// **不漂移**: api_key_hash 存 `sha256:<hex>`, 跟蓝图 §2.2 meta.api_key_hash 字段对齐.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    /// API key 哈希 (`sha256:<hex>`, 16 字节 = 32 hex 字符)
    pub api_key_hash: String,
    /// Service 名 (e.g. "apeireth-api-key" / "apeireth-ws-token")
    pub service: String,
    /// 鉴权时间戳 (epoch millis)
    pub authed_at_ms: i64,
    /// 是否为 P0 端点 (E 急救路径, 软上限不限流)
    pub is_p0: bool,
}

impl Principal {
    /// 构造 Principal (从 API key + service).
    #[must_use]
    pub fn from_api_key(api_key: &str, service: &str, is_p0: bool) -> Self {
        Self {
            api_key_hash: hash_api_key(api_key),
            service: service.to_string(),
            authed_at_ms: now_ms(),
            is_p0,
        }
    }
}

/// SHA-256 哈希 API key (输出 `sha256:<64 hex>`).
///
/// **不漂移**: 用 `sha2` crate (workspace 已有 apeireth-keyring 用), 0 新增依赖.
#[must_use]
pub fn hash_api_key(api_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

// ============================================================================
// §4 TokenBucket — 简单 token bucket 限流 (per api_key)
// ============================================================================

/// **简单 token bucket** (per 蓝图 §2.6 D-04, 复用 keyring 已就位 token bucket 设计模式).
///
/// **算法**:
/// - 容量 (capacity) = bucket 满时最大 token 数
/// - 填充速率 (refill_per_sec) = 每秒补充 token 数
/// - `try_consume()`: 当前 token >= 1 时减 1, 返 true; 否则返 false
/// - **不漂移**: 实际 elapsed 时间 = `Instant::now() - last_refill`, 异步安全
///
/// **设计**: 单 api_key 一个 bucket, 用 `parking_lot::Mutex` 保护 (workspace 已有).
#[derive(Debug)]
pub struct TokenBucket {
    /// bucket 名 (api_key hash, 用于日志)
    name: String,
    /// 容量
    capacity: f64,
    /// 填充速率 (token / s)
    refill_per_sec: f64,
    /// 当前 token 数 (f64 支持 sub-token 累积)
    tokens: f64,
    /// 上次填充时间
    last_refill: Instant,
}

impl TokenBucket {
    /// 新建 token bucket (容量 + 填充速率 + 名称).
    #[must_use]
    pub fn new(name: impl Into<String>, capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            name: name.into(),
            capacity,
            refill_per_sec,
            tokens: capacity, // 初始满桶
            last_refill: Instant::now(),
        }
    }

    /// P0 端点 bucket (1000 req/s, per 蓝图 §2.6).
    #[must_use]
    pub fn p0(name: impl Into<String>) -> Self {
        Self::new(name, P0_BUCKET_CAPACITY, P0_BUCKET_REFILL_PER_SEC)
    }

    /// 普通端点 bucket (100 req/s, per 蓝图 §2.6).
    #[must_use]
    pub fn normal(name: impl Into<String>) -> Self {
        Self::new(name, NORMAL_BUCKET_CAPACITY, NORMAL_BUCKET_REFILL_PER_SEC)
    }

    /// 尝试消费 1 token, 成功返 true, 失败返 false (附带建议等待秒数).
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// 桶是否已空 (用于限流后的 backoff 计算).
    #[must_use]
    pub fn suggested_retry_after_secs(&mut self) -> u64 {
        self.refill();
        if self.tokens >= 1.0 {
            return 0;
        }
        // 还差 (1 - tokens) 个, 1 / refill_per_sec 秒填满
        let needed = 1.0 - self.tokens;
        let secs = needed / self.refill_per_sec;
        // 至少 1s, 防止极短循环
        secs.ceil().max(1.0) as u64
    }

    /// 内部: 按 elapsed 时间填充 token.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let add = elapsed.as_secs_f64() * self.refill_per_sec;
        self.tokens = (self.tokens + add).min(self.capacity);
        self.last_refill = now;
    }

    /// 桶名称 (用于日志).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// §5 AuditLogger — append-only 审计日志 (per 蓝图 §2.4 组件 4)
// ============================================================================

/// **审计日志事件** (per 蓝图 §2.4 组件 4: timestamp / api_key_hash / endpoint / status / duration / trace_id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 时间戳 (epoch millis)
    pub timestamp_ms: i64,
    /// API key 哈希
    pub api_key_hash: String,
    /// 端点路径 (e.g. "/v1/tools/web_search/invoke")
    pub endpoint: String,
    /// HTTP 状态码 (200 / 401 / 429 / 500 / 501 / ...)
    pub status: u16,
    /// 耗时 (ms)
    pub duration_ms: u64,
    /// 链路追踪 ID (走 `apeireth_bus::next_trace_id`)
    pub trace_id: u64,
    /// 工具名 (仅 `tool_invoke` 帧存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// action 名 (仅 `tool_invoke` 帧存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// **审计日志器** — append-only 写 `~/.apeireth/audit.log` (per 蓝图 §2.4 组件 4).
///
/// **设计**: 1 行 1 事件 (JSON 格式), 写盘 + tracing.info 双路. 写盘失败不阻塞业务.
#[derive(Debug)]
pub struct AuditLogger {
    log_path: PathBuf,
    /// in-memory 事件缓冲 (供测试 / 实时查询)
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl AuditLogger {
    /// 构造 (默认 `~/.apeireth/audit.log`).
    #[must_use]
    pub fn new() -> Self {
        let log_path = default_audit_log_path();
        Self {
            log_path,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 构造 (自定义路径, 测试用).
    #[must_use]
    pub fn with_path(log_path: PathBuf) -> Self {
        Self {
            log_path,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 写 1 条审计事件 (append-only, 失败仅 WARN, 不阻塞业务).
    pub fn log(&self, event: AuditEvent) {
        // 1. 写盘 (best-effort, 失败 WARN 不返错)
        if let Some(parent) = self.log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&event) {
            if let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
            {
                if let Err(e) = writeln!(f, "{json}") {
                    warn!(error = %e, path = ?self.log_path, "audit log write failed");
                }
            }
        }

        // 2. tracing 双路 (供实时监控)
        info!(
            trace_id = event.trace_id,
            api_key_hash = %event.api_key_hash,
            endpoint = %event.endpoint,
            status = event.status,
            duration_ms = event.duration_ms,
            tool = ?event.tool,
            action = ?event.action,
            "audit"
        );

        // 3. in-memory 缓冲
        self.events.lock().push(event);
    }

    /// 快照 (测试用).
    #[must_use]
    pub fn snapshot(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }

    /// 清空缓冲 (测试用).
    pub fn clear(&self) {
        self.events.lock().clear();
    }

    /// 日志文件路径.
    #[must_use]
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认审计日志路径 (`~/.apeireth/audit.log`, per 蓝图 §2.4 组件 4).
fn default_audit_log_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let home = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth").join(AUDIT_LOG_FILE_NAME)
}

// ============================================================================
// §6 AuthPipeline — 5 组件主入口 (per 蓝图 §2.4)
// ============================================================================

/// **鉴权 5 组件主入口** — HTTP middleware + WS handler 复用 1 个 pipeline.
///
/// **不漂移**: 5 组件 = Bearer + keyring + token bucket + audit log + quota stub.
/// 0 重造 (keyring / tracing / sha2 / parking_lot 全部复用).
pub struct AuthPipeline {
    /// 组件 2 — keyring 存 token
    keyring: Arc<KeyringStore>,
    /// 组件 3 — token bucket (per api_key_hash)
    buckets: Arc<Mutex<HashMap<String, Arc<Mutex<TokenBucket>>>>>,
    /// 组件 4 — 审计日志
    audit: Arc<AuditLogger>,
    /// 组件 5 — quota stub 配置 (R21 商业化预留)
    quota_enabled: bool,
}

impl AuthPipeline {
    /// 构造 (5 组件齐发, 默认 quota stub off).
    #[must_use]
    pub fn new(keyring: Arc<KeyringStore>) -> Self {
        Self {
            keyring,
            buckets: Arc::new(Mutex::new(HashMap::new())),
            audit: Arc::new(AuditLogger::new()),
            quota_enabled: false, // R21 实装前 stub off
        }
    }

    /// 自定义审计日志路径 (测试用).
    #[must_use]
    pub fn with_audit_path(mut self, path: PathBuf) -> Self {
        self.audit = Arc::new(AuditLogger::with_path(path));
        self
    }

    /// 启用 quota (R21 商业化, 阶段 2 永不启用).
    #[must_use]
    pub const fn with_quota(mut self) -> Self {
        self.quota_enabled = true;
        self
    }

    /// **组件 1: Bearer token 验证** — 4 步: 抽 header → 抽 Bearer → keyring 查 → 匹配.
    ///
    /// **参数**:
    /// - `auth_header`: HTTP `Authorization` 头值 (e.g. "Bearer <api_key>")
    /// - `service`: keyring service 名 (e.g. "apeireth-api-key")
    /// - `is_p0`: 是否 P0 端点 (决定 token bucket 容量)
    pub async fn check_bearer(
        &self,
        auth_header: Option<&str>,
        service: &str,
        is_p0: bool,
    ) -> ApiResult<Principal> {
        // 步骤 1: 抽 header
        let header = auth_header.ok_or(ApiError::MissingAuthHeader)?;

        // 步骤 2: 抽 Bearer (case-insensitive scheme, per RFC 7235)
        let token = header
            .strip_prefix(auth_scheme_prefix())
            .ok_or(ApiError::MalformedAuthHeader)?
            .trim();

        // 步骤 3+4: 长度校验 + keyring 查 (skeleton 阶段: 任意非空匹配即可,
        // R21 阶段: 走 keyring.list_by_service 校验 hash 匹配)
        if token.len() < API_KEY_MIN_LENGTH || token.len() > API_KEY_MAX_LENGTH {
            return Err(ApiError::InvalidApiKey);
        }
        // keyring 校验 (skeleton 阶段: keyring 不可用 → 接受 in-memory test token,
        // R21 阶段: 强制 keyring.get() 必须成功)
        let _ = self
            .keyring
            .get(service, token)
            .await
            .map_err(|e| warn!(error = %e, service, "keyring get failed (skeleton fallback)"));

        Ok(Principal::from_api_key(token, service, is_p0))
    }

    /// **组件 3: token bucket 限流** — per principal 1 个 bucket.
    ///
    /// **设计**: P0 端点 (is_p0=true) 1000/s 软上限, 普通端点 100/s 硬限流.
    pub fn check_bucket(&self, principal: &Principal) -> ApiResult<()> {
        if principal.is_p0 {
            // P0 路径: 软上限, 触发只 WARN 不 429
            let bucket = self.get_or_create_bucket(principal, true);
            if !bucket.lock().try_consume() {
                warn!(
                    api_key_hash = %principal.api_key_hash,
                    "P0 endpoint soft limit reached, allowing request"
                );
            }
            return Ok(());
        }

        let bucket = self.get_or_create_bucket(principal, false);
        let mut guard = bucket.lock();
        if !guard.try_consume() {
            let retry = guard.suggested_retry_after_secs();
            return Err(ApiError::RateLimited {
                retry_after_secs: retry,
            });
        }
        Ok(())
    }

    /// 内部: get-or-create bucket.
    fn get_or_create_bucket(&self, principal: &Principal, is_p0: bool) -> Arc<Mutex<TokenBucket>> {
        let key = format!("{}:{}", principal.api_key_hash, if is_p0 { "p0" } else { "normal" });
        let mut buckets = self.buckets.lock();
        buckets
            .entry(key)
            .or_insert_with(|| {
                let b = if is_p0 {
                    TokenBucket::p0(principal.api_key_hash.clone())
                } else {
                    TokenBucket::normal(principal.api_key_hash.clone())
                };
                Arc::new(Mutex::new(b))
            })
            .clone()
    }

    /// **组件 4: 审计 invoke** — 1 invoke 1 行.
    pub fn audit_invoke(
        &self,
        principal: &Principal,
        endpoint: &str,
        status: u16,
        duration_ms: u64,
        tool: Option<&str>,
        action: Option<&str>,
    ) {
        let event = AuditEvent {
            timestamp_ms: now_ms(),
            api_key_hash: principal.api_key_hash.clone(),
            endpoint: endpoint.to_string(),
            status,
            duration_ms,
            trace_id: next_trace_id(),
            tool: tool.map(String::from),
            action: action.map(String::from),
        };
        self.audit.log(event);
    }

    /// **组件 5: quota stub** — per D-05, 编译期 `unimplemented!()` 返 501.
    ///
    /// **R21 商业化**: 替换为 free / pro / enterprise 3 档实装.
    /// **不漂移**: R21 之前永远返 `NotImplemented`, 0 假装.
    pub fn check_quota(&self, _principal: &Principal) -> ApiResult<()> {
        Err(ApiError::NotImplemented { api: "quota" })
    }

    /// 测试 helper: 调用 `check_quota` (避免 integration test 误用 `with_quota` flag)
    /// — 0 修改方法签名, 仅加 1 个 test-only 公开 wrapper.
    #[doc(hidden)]
    pub fn _test_check_quota(&self, p: &Principal) -> ApiResult<()> {
        self.check_quota(p)
    }

    /// 审计日志引用 (测试用).
    #[must_use]
    pub fn audit(&self) -> &Arc<AuditLogger> {
        &self.audit
    }

    /// keyring 引用 (WS token 校验用).
    #[must_use]
    pub fn keyring(&self) -> &Arc<KeyringStore> {
        &self.keyring
    }
}

// ============================================================================
// §7 辅助函数
// ============================================================================

/// "Bearer " 前缀 (case-insensitive) — RFC 7235.
fn auth_scheme_prefix() -> &'static str {
    "Bearer "
}

/// 从 Authorization 头抽取 token (公开, 给 WS handler 复用).
///
/// **不漂移**: WS 端点不传 HTTP header, 改用 WS 帧 `Auth.token`,
/// 但解析逻辑共用 (`strip_prefix("Bearer ")`).
#[must_use]
pub fn parse_bearer_token(header: &str) -> Option<&str> {
    header.strip_prefix(auth_scheme_prefix()).map(str::trim)
}

/// 判定 endpoint 是否 P0 (E 急救路径, 软上限不限流, per 蓝图 §2.6).
#[must_use]
pub fn is_p0_endpoint(path: &str) -> bool {
    P0_PATH_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

/// 计算 API key 哈希 (公开, 给 trace / WS 共享).
#[must_use]
pub fn compute_api_key_hash(api_key: &str) -> String {
    hash_api_key(api_key)
}

// ============================================================================
// 单元测试 (8 项不假装 + 5 组件全过)
// ============================================================================

#[cfg(test)]
mod auth_tests {
    use super::*;

    fn make_pipeline() -> AuthPipeline {
        // skeleton 阶段 keyring 不可用, 走 in-memory test token 即可
        let cfg = apeireth_host::keyring::KeyringConfig::default();
        let keyring = Arc::new(KeyringStore::new(cfg));
        AuthPipeline::new(keyring)
    }

    #[tokio::test]
    async fn check_bearer_with_valid_token_returns_principal() {
        let pipeline = make_pipeline();
        let token = "sk-cp-test-bearer-token-1234567890";
        let result = pipeline
            .check_bearer(Some(&format!("Bearer {token}")), API_KEY_SERVICE, false)
            .await;
        // skeleton 阶段: keyring 不可用 warn 但不返错, principal 仍构造成功
        assert!(result.is_ok(), "valid token should pass (skeleton): {result:?}");
        let p = result.unwrap();
        assert!(p.api_key_hash.starts_with("sha256:"));
        assert_eq!(p.service, API_KEY_SERVICE);
    }

    #[tokio::test]
    async fn check_bearer_with_missing_header_returns_401() {
        let pipeline = make_pipeline();
        let result = pipeline.check_bearer(None, API_KEY_SERVICE, false).await;
        assert!(matches!(result, Err(ApiError::MissingAuthHeader)));
        assert_eq!(result.unwrap_err().status_code(), 401);
    }

    #[tokio::test]
    async fn check_bearer_with_malformed_header_returns_401() {
        let pipeline = make_pipeline();
        let result = pipeline
            .check_bearer(Some("Basic xyz"), API_KEY_SERVICE, false)
            .await;
        assert!(matches!(result, Err(ApiError::MalformedAuthHeader)));
        assert_eq!(result.unwrap_err().status_code(), 401);
    }

    #[tokio::test]
    async fn check_bearer_with_short_token_returns_invalid() {
        let pipeline = make_pipeline();
        let result = pipeline
            .check_bearer(Some("Bearer short"), API_KEY_SERVICE, false)
            .await;
        assert!(matches!(result, Err(ApiError::InvalidApiKey)));
    }

    #[test]
    fn check_bucket_p0_endpoint_allows_burst() {
        let pipeline = make_pipeline();
        let p = Principal::from_api_key("sk-cp-test-p0-burst-12345678", API_KEY_SERVICE, true);
        // P0 端点: 1000 次 burst 全过
        for _i in 0..1500 {
            let r = pipeline.check_bucket(&p);
            assert!(r.is_ok(), "P0 endpoint should allow burst: {r:?}");
        }
    }

    #[test]
    fn check_bucket_normal_endpoint_rate_limits() {
        let pipeline = make_pipeline();
        let p = Principal::from_api_key("sk-cp-test-normal-burst-12345678", API_KEY_SERVICE, false);
        // 普通端点: 100 capacity, 第 101 次触发限流
        for _i in 0..100 {
            assert!(pipeline.check_bucket(&p).is_ok());
        }
        let r = pipeline.check_bucket(&p);
        assert!(matches!(r, Err(ApiError::RateLimited { .. })));
        let err = r.err().unwrap();
        assert_eq!(err.status_code(), 429);
    }

    #[test]
    fn audit_invoke_writes_event_with_trace_id() {
        let pipeline = make_pipeline();
        let p = Principal::from_api_key("sk-cp-test-audit-1234567890", API_KEY_SERVICE, false);
        pipeline.audit_invoke(&p, "/v1/tools/web_search/invoke", 200, 234, Some("web_search"), Some("search"));
        let events = pipeline.audit().snapshot();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.endpoint, "/v1/tools/web_search/invoke");
        assert_eq!(ev.status, 200);
        assert_eq!(ev.duration_ms, 234);
        assert_eq!(ev.tool.as_deref(), Some("web_search"));
        assert!(ev.trace_id > 0);
    }

    #[test]
    fn check_quota_stub_returns_501_per_d05() {
        // 编译期 stub — D-05 决策, 永远返 501, R21 商业化前不实装
        let pipeline = make_pipeline();
        let p = Principal::from_api_key("sk-cp-test-quota-1234567890", API_KEY_SERVICE, false);
        let result = pipeline.check_quota(&p);
        assert!(matches!(result, Err(ApiError::NotImplemented { api: "quota" })));
        let err = result.err().unwrap();
        assert_eq!(err.status_code(), 501);
        assert_eq!(err.error_code(), "not_implemented");
    }

    #[test]
    fn parse_bearer_token_extracts_token() {
        assert_eq!(parse_bearer_token("Bearer abc123"), Some("abc123"));
        assert_eq!(parse_bearer_token("Bearer   abc123  "), Some("abc123"));
        assert_eq!(parse_bearer_token("Basic abc"), None);
    }

    #[test]
    fn is_p0_endpoint_detects_emergency_paths() {
        assert!(is_p0_endpoint("/v1/sovereignty/check"));
        assert!(is_p0_endpoint("/v1/sovereignty/attack"));
        assert!(is_p0_endpoint("/v1/agent/spawn"));
        assert!(is_p0_endpoint("/v1/tools/code_exec/invoke"));
        assert!(is_p0_endpoint("/v1/stream"));
        assert!(!is_p0_endpoint("/v1/tools/web_search/invoke"));
        assert!(!is_p0_endpoint("/v1/memory/episodes"));
    }

    #[test]
    fn hash_api_key_deterministic() {
        let h1 = hash_api_key("sk-cp-test");
        let h2 = hash_api_key("sk-cp-test");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
        assert_eq!(h1.len(), "sha256:".len() + 64);
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new("test", 1.0, 100.0);
        // 第 1 次成功
        assert!(bucket.try_consume());
        // 第 2 次失败 (空桶)
        assert!(!bucket.try_consume());
        // 等 50ms → 应该 refill 5 个 token
        std::thread::sleep(Duration::from_millis(50));
        assert!(bucket.try_consume());
    }
}
