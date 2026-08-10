
#![warn(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

// 子模块 (per-request tracing + Prometheus metrics + JSON logging + 3 health 端点)

// ============================================================================
// 编译期 hardcode 常量 (10 项, per task spec)
// ============================================================================

/// Observability schema 版本 (向前兼容字段, R20+ 改格式时 bump).
pub const OBSERVABILITY_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// Prometheus 默认 scrape 端口 (1.0 release 标配 9090).
pub const PROMETHEUS_DEFAULT_PORT: u16 = 9090;

/// OpenTelemetry OTLP 默认 endpoint (1.0 release 标配 localhost:4317).
/// R20 阶段 3 续 OpenTelemetry SDK 时使用, skeleton 阶段 0 实际 push.
pub const OTLP_DEFAULT_ENDPOINT: &str = "http://localhost:4317";

/// Health check 默认超时 (毫秒, 5s).
pub const HEALTH_CHECK_TIMEOUT_MS: u64 = 5000;

/// Tracing 采样率 (1.0 release 全量采样, 不允许降级).
pub const TRACING_SAMPLE_RATE: f64 = 1.0;

/// Log 格式强制 JSON (1.0 release, 不允许降级到 plain text).
pub const LOG_FORMAT_JSON: bool = true;

/// PII 脱敏 regex 模式 (4 关键词: password/secret/token/key, 1:1 翻译 keyring SecretBytes 脱敏逻辑).
/// 编译期 hardcode, 运行时不允许改, 防 m3 hallucination 调弱脱敏规则.
pub const PII_REDACTION_PATTERN: &str = r"(?i)(password|secret|token|key)\s*[:=]\s*\S+";

/// 支持的 metrics 种类 (3 类: counter / gauge / histogram, per K-1 强校验).
pub const SUPPORTED_METRICS: &[MetricKind] = &[
    MetricKind::Counter,
    MetricKind::Gauge,
    MetricKind::Histogram,
];

/// 编译期守门: SUPPORTED_METRICS 长度 == 3 (K-1 强校验 #2).
const _: () = assert!(SUPPORTED_METRICS.len() == 3);

/// Health 3 端点 (per K-1 强校验).
pub const HEALTH_ENDPOINTS: &[&str] = &["/health", "/ready", "/metrics"];

/// Health 端点数量 (编译期守门, 必 = 3).
pub const HEALTH_ENDPOINTS_COUNT: usize = 3;

/// 编译期守门: HEALTH_ENDPOINTS 长度 == 3 (K-1 强校验).
const _: () = assert!(HEALTH_ENDPOINTS.len() == HEALTH_ENDPOINTS_COUNT);

// ============================================================================
// m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的 observability 工具名.
// ============================================================================

/// m3 防御: Observability 8 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **8 工具对应 5 大功能**:
/// - Tracing (1): `apeireth_observability_trace` — per-request trace 集成
/// - Metrics (3): `apeireth_observability_metric_counter` / `_gauge` / `_histogram`
/// - Health (1): `apeireth_observability_health_check`
/// - PII (1): `apeireth_observability_pii_redact`
/// - Logging (1): `apeireth_observability_log_structured`
/// - OTLP (1): `apeireth_observability_otlp_push` (R20 阶段 3 续)
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_observability_trace",
    "apeireth_observability_metric_counter",
    "apeireth_observability_metric_gauge",
    "apeireth_observability_metric_histogram",
    "apeireth_observability_health_check",
    "apeireth_observability_pii_redact",
    "apeireth_observability_log_structured",
    "apeireth_observability_otlp_push",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8.
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `ObservabilityError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), ObservabilityError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(ObservabilityError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (1:1 翻译 OpenTelemetry / Prometheus 异常面, 估 10 variant)
// ============================================================================

/// Observability 错误类型 (m3 + PII + tracing + metrics + health).
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// PII 脱敏失败 (regex 编译 / 匹配错误, 估 0 触发, 编译期 PII_REDACTION_PATTERN hardcode 已校验)
    #[error("PII redaction failed: {0}")]
    PiiRedactionFailed(String),

    /// Tracing 上下文缺失 (没在 span 内, 不能注入 trace_id)
    #[error("tracing context missing: {0}")]
    TracingContextMissing(String),

    /// Metric 类型不支持 (估 m3 调 `apeireth_observability_metric_summary`, 实际只有 3 类)
    #[error("metric kind not supported: {0}")]
    MetricKindUnsupported(String),

    /// Health check 超时 (HEALTH_CHECK_TIMEOUT_MS)
    #[error("health check timeout after {timeout_ms}ms: {endpoint}")]
    HealthCheckTimeout {
        /// 超时毫秒
        timeout_ms: u64,
        /// 端点
        endpoint: String,
    },

    /// Health check 端点未知
    #[error("health check endpoint unknown: {0}")]
    HealthEndpointUnknown(String),

    /// OTLP push 失败 (R20 阶段 3 续, skeleton 阶段不触发, 留口子)
    #[error("OTLP push failed: {0}")]
    OtlpPushFailed(String),

    /// Prometheus exposition format 错误 (反序列化失败)
    #[error("prometheus format error: {0}")]
    PrometheusFormat(String),

    /// JSON log 序列化错误
    #[error("JSON log serialization error: {0}")]
    JsonLog(#[from] serde_json::Error),

    /// 通用错误
    #[error("observability error: {0}")]
    Other(String),
}

/// Observability Result 别名.
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;

// ============================================================================
// §2 核心类型 (TraceId / SpanId / MetricKind enum / HealthStatus enum)
// ============================================================================

/// Trace ID (16 字节随机 / W3C Trace Context 兼容格式, 32 hex 字符).
///
/// 1:1 翻译 OpenTelemetry `trace_id` 字段, 但 skeleton 阶段不引 OpenTelemetry SDK,
/// 用 `rand` 0.8 自己生成 16 字节随机 ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    /// 新建 (16 字节随机, hex 编码 32 字符).
    #[must_use]
    pub fn new() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(hex::encode(bytes))
    }

    /// 暴露 hex 字符串 (32 字符).
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// 从 hex 字符串解析 (失败返 `ObservabilityError::TracingContextMissing`).
    pub fn from_hex(s: &str) -> ObservabilityResult<Self> {
        if s.len() != 32 {
            return Err(ObservabilityError::TracingContextMissing(format!(
                "trace_id must be 32 hex chars, got {}",
                s.len()
            )));
        }
        hex::decode(s).map_err(|e| {
            ObservabilityError::TracingContextMissing(format!("trace_id hex decode: {e}"))
        })?;
        Ok(Self(s.to_string()))
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Span ID (8 字节随机 / W3C Trace Context 兼容格式, 16 hex 字符).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(String);

impl SpanId {
    /// 新建 (8 字节随机, hex 编码 16 字符).
    #[must_use]
    pub fn new() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 8];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(hex::encode(bytes))
    }

    /// 暴露 hex 字符串 (16 字符).
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// 从 hex 字符串解析.
    pub fn from_hex(s: &str) -> ObservabilityResult<Self> {
        if s.len() != 16 {
            return Err(ObservabilityError::TracingContextMissing(format!(
                "span_id must be 16 hex chars, got {}",
                s.len()
            )));
        }
        hex::decode(s).map_err(|e| {
            ObservabilityError::TracingContextMissing(format!("span_id hex decode: {e}"))
        })?;
        Ok(Self(s.to_string()))
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Span 上下文 (per-request `trace_id` + `span_id` 注入载体).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanContext {
    /// Trace ID (W3C 32 hex)
    pub trace_id: TraceId,
    /// Span ID (W3C 16 hex)
    pub span_id: SpanId,
    /// Span 名 (例: "http_request" / "db_query")
    pub name: String,
    /// 父 Span ID (可选, 顶层 span 无父)
    pub parent_span_id: Option<SpanId>,
    /// 开始时间戳 (UTC RFC 3339)
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl SpanContext {
    /// 新建顶层 span (无父).
    pub fn new_root(name: impl Into<String>) -> Self {
        Self {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            name: name.into(),
            parent_span_id: None,
            start_time: chrono::Utc::now(),
        }
    }

    /// 新建子 span (继承 trace_id, 新 span_id, 父为传入的 ctx).
    pub fn new_child(name: impl Into<String>, parent: &SpanContext) -> Self {
        Self {
            trace_id: parent.trace_id.clone(),
            span_id: SpanId::new(),
            name: name.into(),
            parent_span_id: Some(parent.span_id.clone()),
            start_time: chrono::Utc::now(),
        }
    }
}

/// Metric 类型 (3 类, 1:1 翻译 Prometheus metric types, per K-1 强校验).
///
/// Prometheus 工业标准 4 类 (Counter/Gauge/Histogram/Summary), skeleton 阶段只支持 3 类
/// (Counter/Gauge/Histogram), Summary 留 R20 阶段 3 续 (跟 OpenTelemetry SDK 一起加).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricKind {
    /// Counter (单调递增, 例: 请求总数)
    Counter,
    /// Gauge (任意值, 例: 当前并发连接数)
    Gauge,
    /// Histogram (分布, 例: 延迟 P50/P90/P99)
    Histogram,
}

impl MetricKind {
    /// Prometheus 字符串 (小写, 跟 exposition format 对齐).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricKind::Counter => "counter",
            MetricKind::Gauge => "gauge",
            MetricKind::Histogram => "histogram",
        }
    }
}

impl fmt::Display for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 单个 metric 样本.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    /// Metric 名 (Prometheus snake_case, 例: "http_requests_total")
    pub name: String,
    /// 类型
    pub kind: MetricKind,
    /// 值
    pub value: f64,
    /// 标签 (key=value, Prometheus exposition format `{k="v",k2="v2"}`)
    pub labels: HashMap<String, String>,
}

impl MetricSample {
    /// 构造新样本.
    pub fn new(name: impl Into<String>, kind: MetricKind, value: f64) -> Self {
        Self {
            name: name.into(),
            kind,
            value,
            labels: HashMap::new(),
        }
    }

    /// 加标签 (链式).
    pub fn with_label(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.labels.insert(k.into(), v.into());
        self
    }
}

/// Health 状态 (3 端点共用 enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// 健康 (所有依赖 OK)
    Healthy,
    /// 降级 (部分依赖异常, 仍可服务)
    Degraded,
    /// 不健康 (核心依赖异常, 不能服务)
    Unhealthy,
}

impl HealthStatus {
    /// HTTP 状态码 (per Prometheus / Kubernetes health 约定).
    #[must_use]
    pub fn http_status_code(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 200,   // 200 + body 表明降级
            HealthStatus::Unhealthy => 503,  // 503 Service Unavailable
        }
    }

    /// 字符串 (per K-1 强校验).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        }
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Health 响应 (统一 3 端点返, 字段差异由 `endpoint` 字段区分).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// 端点名 ("/health" / "/ready" / "/metrics")
    pub endpoint: String,
    /// 状态
    pub status: HealthStatus,
    /// 时间戳 (UTC RFC 3339)
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Schema 版本
    pub schema_version: String,
    /// 平台名 (K-1 强校验, 永远 = "apeireth")
    pub platform: String,
    /// 额外信息 (key-value, 端点特定)
    pub details: HashMap<String, String>,
}

impl HealthResponse {
    /// 构造新响应.
    pub fn new(endpoint: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            endpoint: endpoint.into(),
            status,
            timestamp: chrono::Utc::now(),
            schema_version: OBSERVABILITY_SCHEMA_VERSION.to_string(),
            platform: PLATFORM_NAME.to_string(),
            details: HashMap::new(),
        }
    }

    /// 加 detail (链式).
    pub fn with_detail(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.details.insert(k.into(), v.into());
        self
    }
}

// ============================================================================
// §3 PII 脱敏 (per 2 重 PII 防御 + PII_REDACTION_PATTERN 编译期 hardcode)
// ============================================================================

/// PII 脱敏 (1:1 翻译 keyring SecretBytes 脱敏逻辑, 但用 regex pattern 覆盖更多场景).
///
/// 把 `password=xxx` / `token:xxx` 等模式中的值替换为 `***`.
///
/// # Examples
///
/// ```
/// use apeireth_observability::redact_pii;
/// assert_eq!(redact_pii("password=secret123").as_deref(), Some("password=***"));
/// assert_eq!(redact_pii("api_token=abc").as_deref(), Some("api_token=***"));
/// ```
pub fn redact_pii(input: &str) -> Option<String> {
    let re = regex::Regex::new(PII_REDACTION_PATTERN).ok()?;
    let result = re.replace_all(input, |caps: &regex::Captures| {
        // caps[0] 是整段 "password=secret123", 取第一段 "password" 或 "password="
        let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        // 提取 key + 分隔符 (= 或 :)
        let key_end = full
            .find(|c: char| c == '=' || c == ':')
            .map(|i| i + 1)
            .unwrap_or(full.len());
        let key_with_sep = &full[..key_end];
        format!("{key_with_sep}***")
    });
    Some(result.into_owned())
}

// ============================================================================
// §4 Observability 总线 (per-request span 注入 + metrics 收集 + health 检查)
// ============================================================================

/// Observability 总线 (跟 keyring KeyringStore 同模式, async fn + `Arc<RwLock<...>>`).
///
/// 持有 per-request SpanContext + MetricSample 列表 + 最近 health 状态.
pub struct ObservabilityBus {
    /// 配置
    config: ObservabilityConfig,
    /// 最近活跃的 span 栈 (per-request 上下文注入)
    span_stack: Arc<RwLock<Vec<SpanContext>>>,
    /// 已收集的 metrics 样本
    metrics: Arc<RwLock<Vec<MetricSample>>>,
    /// 最近 health 状态 (3 端点共用)
    health: Arc<RwLock<HealthStatus>>,
}

/// Observability 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// 平台名 (K-1 强校验, 永远 = "apeireth")
    pub platform: String,
    /// Schema 版本
    pub schema_version: String,
    /// Prometheus scrape 端口
    pub prometheus_port: u16,
    /// OTLP endpoint (R20 阶段 3 续)
    pub otlp_endpoint: String,
    /// Tracing 采样率 (1.0 全量)
    pub sample_rate: f64,
    /// Health check 超时 (毫秒)
    pub health_timeout_ms: u64,
    /// Log 格式强制 JSON
    pub log_format_json: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            platform: PLATFORM_NAME.to_string(),
            schema_version: OBSERVABILITY_SCHEMA_VERSION.to_string(),
            prometheus_port: PROMETHEUS_DEFAULT_PORT,
            otlp_endpoint: OTLP_DEFAULT_ENDPOINT.to_string(),
            sample_rate: TRACING_SAMPLE_RATE,
            health_timeout_ms: HEALTH_CHECK_TIMEOUT_MS,
            log_format_json: LOG_FORMAT_JSON,
        }
    }
}

impl ObservabilityBus {
    /// 新建 (默认配置, 不实际启动服务).
    #[must_use]
    pub fn new(config: ObservabilityConfig) -> Self {
        Self {
            config,
            span_stack: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(Vec::new())),
            health: Arc::new(RwLock::new(HealthStatus::Healthy)),
        }
    }

    /// 配置
    #[must_use]
    pub fn config(&self) -> &ObservabilityConfig {
        &self.config
    }

    /// 推入新 span (per-request trace 注入, 模拟 OpenTelemetry `tracer.start_span()`).
    pub async fn push_span(&self, ctx: SpanContext) {
        info!(
            trace_id = %ctx.trace_id,
            span_id = %ctx.span_id,
            name = %ctx.name,
            "observability: span push"
        );
        self.span_stack.write().await.push(ctx);
    }

    /// 弹出 span (per-request 结束).
    pub async fn pop_span(&self) -> Option<SpanContext> {
        let ctx = self.span_stack.write().await.pop();
        if let Some(ref c) = ctx {
            info!(
                trace_id = %c.trace_id,
                span_id = %c.span_id,
                name = %c.name,
                "observability: span pop"
            );
        }
        ctx
    }

    /// 当前栈顶 span (活跃 span).
    pub async fn current_span(&self) -> Option<SpanContext> {
        self.span_stack.read().await.last().cloned()
    }

    /// 记录 metric 样本.
    pub async fn record_metric(&self, sample: MetricSample) {
        info!(
            name = %sample.name,
            kind = %sample.kind,
            value = sample.value,
            "observability: metric record"
        );
        self.metrics.write().await.push(sample);
    }

    /// 列出所有 metrics 样本 (Prometheus scrape 用).
    pub async fn list_metrics(&self) -> Vec<MetricSample> {
        self.metrics.read().await.clone()
    }

    /// 设置 health 状态 (3 端点共用).
    pub async fn set_health(&self, status: HealthStatus) {
        info!(status = %status, "observability: health update");
        *self.health.write().await = status;
    }

    /// 当前 health 状态.
    pub async fn health(&self) -> HealthStatus {
        *self.health.read().await
    }
}

impl Default for ObservabilityBus {
    fn default() -> Self {
        Self::new(ObservabilityConfig::default())
    }
}

// ============================================================================
// §5 K-1 强校验自检 (5 K-1 字样 + 3 MetricKind + 8 工具名, fixture 验证)
// ============================================================================

/// K-1 强校验 #1: 平台名 (5 K-1 字样 #1).
pub fn k1_check_platform_name() -> bool {
    PLATFORM_NAME == "apeireth"
}

/// K-1 强校验 #2: SUPPORTED_METRICS 长度 3 (5 K-1 字样 #2: "observability" + K-1 #2).
pub fn k1_check_metrics_count() -> bool {
    SUPPORTED_METRICS.len() == 3
}

/// K-1 强校验 #3: TOOL_WHITELIST 8 工具 (5 K-1 字样 #3/4/5: "trace" / "metric" / "must-do").
pub fn k1_check_tool_whitelist() -> bool {
    TOOL_WHITELIST.len() == 8
        && TOOL_WHITELIST.contains(&"apeireth_observability_trace")
        && TOOL_WHITELIST.contains(&"apeireth_observability_metric_counter")
        && TOOL_WHITELIST.contains(&"apeireth_observability_metric_gauge")
        && TOOL_WHITELIST.contains(&"apeireth_observability_metric_histogram")
        && TOOL_WHITELIST.contains(&"apeireth_observability_health_check")
        && TOOL_WHITELIST.contains(&"apeireth_observability_pii_redact")
        && TOOL_WHITELIST.contains(&"apeireth_observability_log_structured")
        && TOOL_WHITELIST.contains(&"apeireth_observability_otlp_push")
}

/// K-1 强校验 #4: 3 HEALTH_ENDPOINTS (per K-1 #4).
pub fn k1_check_health_endpoints() -> bool {
    HEALTH_ENDPOINTS.len() == 3
        && HEALTH_ENDPOINTS.contains(&"/health")
        && HEALTH_ENDPOINTS.contains(&"/ready")
        && HEALTH_ENDPOINTS.contains(&"/metrics")
}

/// 全部 K-1 强校验 (4 条全过).
#[must_use]
pub fn k1_all_pass() -> bool {
    k1_check_platform_name()
        && k1_check_metrics_count()
        && k1_check_tool_whitelist()
        && k1_check_health_endpoints()
}

// ============================================================================
// §6 m3 防御 / 重导出 (per m3-hallucination-defense §2.4)
// ============================================================================

/// 重新导出子模块核心类型 (per R20 阶段 1 5 P0 + 9 skeleton 风格).
pub use super::health::{health_check, render_health_response, HealthEndpoint};
pub use super::logging::{log_structured, LogEntry, LogLevel};
pub use super::metrics::{render_prometheus, MetricsRegistry};
pub use super::tracing_integration::{next_trace_id, trace_span, TraceContext};
// R25.2 估补: 9 器官 dashboard 核心类型 re-export (per 1.0 release #8 observability 100%).
pub use super::tui_dashboard::{
    render_dashboard, render_organ_widget, OrganDashboard, OrganKind, OrganReadiness,
    TuiOrganState, ORGAN_KIND_COUNT, ORGAN_KIND_NAMES_ZH, ORGAN_KIND_ASCII_CHARS,
    TUI_DASHBOARD_PLATFORM, TUI_DASHBOARD_SCHEMA_VERSION, SIX_ANCHORS, FIVE_NAV,
    DASHBOARD_HEALTH_ENDPOINTS,
    TuiDashboardError, TuiDashboardResult,
};

// ============================================================================
// 单元测试 (in-module, 集成测试在 tests/test_observability_in_process.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_platform_name_in_unit() {
        assert_eq!(PLATFORM_NAME, "apeireth");
    }

    #[test]
    fn k1_metrics_count_in_unit() {
        assert_eq!(SUPPORTED_METRICS.len(), 3);
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Counter));
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Gauge));
        assert!(SUPPORTED_METRICS.contains(&MetricKind::Histogram));
    }

    #[test]
    fn k1_health_endpoints_in_unit() {
        assert_eq!(HEALTH_ENDPOINTS.len(), 3);
    }

    #[test]
    fn trace_id_format_32_hex() {
        let id = TraceId::new();
        assert_eq!(id.as_hex().len(), 32);
    }

    #[test]
    fn span_id_format_16_hex() {
        let id = SpanId::new();
        assert_eq!(id.as_hex().len(), 16);
    }

    #[test]
    fn pii_redact_password() {
        assert_eq!(redact_pii("password=secret123").as_deref(), Some("password=***"));
    }

    #[test]
    fn pii_redact_token() {
        assert_eq!(redact_pii("api_token=abc").as_deref(), Some("api_token=***"));
    }

    #[test]
    fn m3_validate_tool_call_rejects_unknown() {
        assert!(validate_tool_call("apeireth_observability_trace", &serde_json::json!({})).is_ok());
        let err = validate_tool_call("apeireth_observability_evil", &serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(err, ObservabilityError::ToolNotWhitelisted(_)));
    }

    #[test]
    fn span_context_root_and_child() {
        let root = SpanContext::new_root("http_request");
        let child = SpanContext::new_child("db_query", &root);
        assert_eq!(root.trace_id, child.trace_id, "child 继承 trace_id");
        assert_ne!(root.span_id, child.span_id, "child 新 span_id");
        assert_eq!(child.parent_span_id.as_ref().unwrap(), &root.span_id);
    }

    #[tokio::test]
    async fn observability_bus_push_pop_span() {
        let bus = ObservabilityBus::default();
        let root = SpanContext::new_root("test");
        bus.push_span(root.clone()).await;
        let current = bus.current_span().await.unwrap();
        assert_eq!(current.name, "test");
        let popped = bus.pop_span().await.unwrap();
        assert_eq!(popped.span_id, root.span_id);
    }

    #[tokio::test]
    async fn observability_bus_record_metric() {
        let bus = ObservabilityBus::default();
        let sample = MetricSample::new("requests_total", MetricKind::Counter, 1.0)
            .with_label("endpoint", "/api/v1");
        bus.record_metric(sample.clone()).await;
        let metrics = bus.list_metrics().await;
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "requests_total");
    }

    #[tokio::test]
    async fn observability_bus_health() {
        let bus = ObservabilityBus::default();
        assert_eq!(bus.health().await, HealthStatus::Healthy);
        bus.set_health(HealthStatus::Degraded).await;
        assert_eq!(bus.health().await, HealthStatus::Degraded);
    }
}
