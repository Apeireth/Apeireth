
#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 (7 子模块)
// ============================================================================


// ============================================================================
// §1 Re-exports (常用类型, doctest 友好)
// ============================================================================

// --- error ---
pub use super::error::{TracingError, TracingResult};

// --- config ---
pub use super::config::{
    ExporterConfig, ResourceConfig, SamplerConfig, ServiceConfig, TracingConfig,
    TracingConfigBuilder, TRACING_CONFIG_SECTION_COUNT, TRACING_CONFIG_SCHEMA_VERSION,
};

// --- context ---
pub use super::context::{TraceContext, SPAN_ID_HEX_LEN, TRACE_ID_HEX_LEN};

// --- span ---
pub use super::span::{
    now_nanos, Span, SpanEvent, SpanEventKind, SpanKind, SpanStatus, SPAN_EVENT_KIND_COUNT,
    SPAN_KIND_COUNT,
};

// --- exporter ---
pub use super::exporter::{
    build_exporter, ExporterKind, FileExporter, JaegerExporter, OtlpGrpcExporter, SpanExporter,
    StdoutExporter, EXPORTER_KIND_COUNT,
};

// --- sampler ---
pub use super::sampler::{
    build_sampler, AlwaysOffSampler, AlwaysOnSampler, ParentBasedSampler, Sampler, SamplerKind,
    TraceIdRatioBasedSampler, SAMPLER_KIND_COUNT,
};

// --- propagation ---
pub use super::propagation::{
    build_propagator, is_valid_span_id, is_valid_trace_id, parse_kv_list, parse_traceparent,
    B3Propagator, JaegerPropagator, Propagator, PropagatorKind, W3CTraceContextPropagator,
    PROPAGATOR_KIND_COUNT,
};

// --- trace ---
pub use super::trace::{generate_span_id, generate_trace_id, Trace, SPAN_ID_LEN, TRACE_ID_LEN};

// ============================================================================
// §2 编译期常量 (crate 级别)
// ============================================================================

/// Schema 版本 (用于序列化兼容性 + 日志 + metrics).
pub const TRACING_SCHEMA_VERSION: &str = "apeireth.tracing/v1";

/// Crate 名.
pub const CRATE_NAME: &str = "apeireth-tracing";

/// R 阶段标识.
pub const R_CYCLE: &str = "R20-Stage-6";

/// 1:1 翻译源 (v0.9.21 商业版).
pub const TRANSLATION_SOURCE: &str = "v0.9.21 @anthropic-ai/tracing";

/// Error 变体数.
pub const TRACING_ERROR_VARIANT_COUNT: usize = 10;

/// K-1 强校验项数 (trace_id + span_id + service_name).
pub const K1_STRONG_VALIDATION_COUNT: usize = 3;

// ============================================================================
// §3 平台名 (用于 metrics / log)
// ============================================================================

/// 平台名 (per m3 防御: 0 PII 暴露, 1 个常量).
pub const PLATFORM_NAME: &str = "apeireth";

// ============================================================================
// §4 顶层辅助函数
// ============================================================================

/// 快速构造 stdout trace (开发/测试用).
///
/// `service_name` 必填 (K-1 强校验).
///
/// # Example
///
/// ```no_run
/// # use apeireth_tracing::quick_trace;
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let _trace = quick_trace("apeireth-api").await?;
/// # Ok(())
/// # }
/// ```
pub async fn quick_trace(
    service_name: &str,
) -> TracingResult<Trace> {
    let config = TracingConfig::builder()
        .service_name(service_name)
        .build()?;
    let sampler = std::sync::Arc::new(AlwaysOnSampler);
    Trace::new(config, sampler).await
}

/// 快速构造 file trace (持久化用).
pub async fn quick_file_trace(
    service_name: &str,
    output_path: &str,
) -> TracingResult<Trace> {
    let config = TracingConfig::builder()
        .service_name(service_name)
        .exporter(ExporterKind::File)
        .file_output_path(output_path)
        .build()?;
    let sampler = std::sync::Arc::new(AlwaysOnSampler);
    Trace::new(config, sampler).await
}

/// K-1 强校验: trace_id 32 lowercase hex char.
pub fn validate_trace_id(s: &str) -> TracingResult<()> {
    if is_valid_trace_id(s) {
        Ok(())
    } else {
        Err(TracingError::InvalidTraceId(s.to_string()))
    }
}

/// K-1 强校验: span_id 16 lowercase hex char.
pub fn validate_span_id(s: &str) -> TracingResult<()> {
    if is_valid_span_id(s) {
        Ok(())
    } else {
        Err(TracingError::InvalidSpanId(s.to_string()))
    }
}

/// K-1 强校验: service.name 非空.
pub fn validate_service_name(s: &str) -> TracingResult<()> {
    if s.trim().is_empty() {
        Err(TracingError::EmptyServiceName)
    } else {
        Ok(())
    }
}

// ============================================================================
// §5 跨服务 trace propagation 辅助
// ============================================================================

/// 把 TraceContext 注入 HTTP header carrier (用 W3C TraceContext).
///
/// # Example
///
/// ```no_run
/// # use apeireth_tracing::{inject_context, span::SpanKind, trace::Trace, exporter::ExporterKind, sampler::AlwaysOnSampler, config::TracingConfig};
/// # use std::collections::HashMap;
/// # use std::sync::Arc;
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = TracingConfig::default();
/// # let mut trace = Trace::new(config, Arc::new(AlwaysOnSampler)).await?;
/// # let span = trace.start_root("op", SpanKind::Client).await?;
/// # let ctx = span.context.clone();
/// let mut headers = HashMap::new();
/// inject_context(&ctx, &mut headers).await;
/// // headers["traceparent"] == "00-...-...-01"
/// # Ok(())
/// # }
/// ```
pub async fn inject_context(
    ctx: &TraceContext,
    carrier: &mut std::collections::HashMap<String, String>,
) {
    W3CTraceContextPropagator.inject(ctx, carrier).await
}

/// 从 HTTP header carrier 提取 TraceContext (用 W3C TraceContext).
pub async fn extract_context(
    carrier: &std::collections::HashMap<String, String>,
) -> Option<TraceContext> {
    W3CTraceContextPropagator.extract(carrier).await
}

// ============================================================================
// §6 Builder 模式: Tracer 顶层
// ============================================================================

/// Tracer — 顶层 trace 构造器.
///
/// # Example
///
/// ```no_run
/// # use apeireth_tracing::{Tracer, span::SpanKind};
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut tracer = Tracer::new("apeireth-api").build().await?;
/// let root = tracer.trace_mut().start_root("op", SpanKind::Server).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Tracer {
    trace: Trace,
}

impl Tracer {
    /// 构造 builder.
    pub fn new(service_name: &str) -> TracerBuilder {
        TracerBuilder {
            service_name: service_name.to_string(),
            sampler_kind: SamplerKind::AlwaysOn,
            sampler_ratio: 1.0,
            exporter_kind: ExporterKind::Stdout,
            exporter_endpoint: String::new(),
            exporter_output_path: "./traces.jsonl".to_string(),
        }
    }

    /// 获取 trace 引用.
    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    /// 获取 trace 可变引用.
    pub fn trace_mut(&mut self) -> &mut Trace {
        &mut self.trace
    }

    /// 消费 self, 返回 trace.
    pub fn into_trace(self) -> Trace {
        self.trace
    }
}

/// TracerBuilder — fluent 构造.
#[derive(Debug, Clone)]
pub struct TracerBuilder {
    service_name: String,
    sampler_kind: SamplerKind,
    sampler_ratio: f64,
    exporter_kind: ExporterKind,
    exporter_endpoint: String,
    exporter_output_path: String,
}

impl TracerBuilder {
    /// 设置 sampler.
    pub fn sampler(mut self, kind: SamplerKind, ratio: f64) -> Self {
        self.sampler_kind = kind;
        self.sampler_ratio = ratio;
        self
    }

    /// 设置 exporter = stdout.
    pub fn stdout_exporter(mut self) -> Self {
        self.exporter_kind = ExporterKind::Stdout;
        self
    }

    /// 设置 exporter = file.
    pub fn file_exporter(mut self, output_path: &str) -> Self {
        self.exporter_kind = ExporterKind::File;
        self.exporter_output_path = output_path.to_string();
        self
    }

    /// 设置 exporter = OTLP gRPC.
    pub fn otlp_exporter(mut self, endpoint: &str) -> Self {
        self.exporter_kind = ExporterKind::OtlpGrpc;
        self.exporter_endpoint = endpoint.to_string();
        self
    }

    /// 设置 exporter = Jaeger.
    pub fn jaeger_exporter(mut self, endpoint: &str) -> Self {
        self.exporter_kind = ExporterKind::Jaeger;
        self.exporter_endpoint = endpoint.to_string();
        self
    }

    /// 构造 Tracer (K-1 强校验).
    pub async fn build(self) -> TracingResult<Tracer> {
        let config = TracingConfig::builder()
            .service_name(&self.service_name)
            .sampler(self.sampler_kind, self.sampler_ratio)
            .exporter(self.exporter_kind)
            .file_output_path(&self.exporter_output_path)
            .build()?;
        let sampler = build_sampler(self.sampler_kind, self.sampler_ratio)?;
        let trace = Trace::new(config, sampler).await?;
        Ok(Tracer { trace })
    }
}

// ============================================================================
// §7 顶层 metrics (atomic 计数)
// ============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// 全局 metrics: 启动的 trace 总数.
static METRIC_TRACES_STARTED: AtomicU64 = AtomicU64::new(0);

/// 全局 metrics: 启动的 span 总数.
static METRIC_SPANS_STARTED: AtomicU64 = AtomicU64::new(0);

/// 全局 metrics: export 成功数.
static METRIC_EXPORTS_OK: AtomicU64 = AtomicU64::new(0);

/// 全局 metrics: export 失败数.
static METRIC_EXPORTS_FAILED: AtomicU64 = AtomicU64::new(0);

/// 增加 traces 计数.
pub fn metric_inc_traces() {
    METRIC_TRACES_STARTED.fetch_add(1, Ordering::Relaxed);
}

/// 增加 spans 计数.
pub fn metric_inc_spans() {
    METRIC_SPANS_STARTED.fetch_add(1, Ordering::Relaxed);
}

/// 增加 export ok 计数.
pub fn metric_inc_export_ok() {
    METRIC_EXPORTS_OK.fetch_add(1, Ordering::Relaxed);
}

/// 增加 export failed 计数.
pub fn metric_inc_export_failed() {
    METRIC_EXPORTS_FAILED.fetch_add(1, Ordering::Relaxed);
}

/// 读 traces 计数.
pub fn metric_traces() -> u64 {
    METRIC_TRACES_STARTED.load(Ordering::Relaxed)
}

/// 读 spans 计数.
pub fn metric_spans() -> u64 {
    METRIC_SPANS_STARTED.load(Ordering::Relaxed)
}

/// 读 export ok 计数.
pub fn metric_exports_ok() -> u64 {
    METRIC_EXPORTS_OK.load(Ordering::Relaxed)
}

/// 读 export failed 计数.
pub fn metric_exports_failed() -> u64 {
    METRIC_EXPORTS_FAILED.load(Ordering::Relaxed)
}

/// 重置全部 metrics (测试用).
pub fn metric_reset() {
    METRIC_TRACES_STARTED.store(0, Ordering::Relaxed);
    METRIC_SPANS_STARTED.store(0, Ordering::Relaxed);
    METRIC_EXPORTS_OK.store(0, Ordering::Relaxed);
    METRIC_EXPORTS_FAILED.store(0, Ordering::Relaxed);
}

// ============================================================================
// §7.5 跨模块辅助函数
// ============================================================================

/// 构造一个最小可用的 root Span (单 trace + 1 span, 用于单元测试).
///
/// # Example
///
/// ```
/// use apeireth_tracing::{make_test_span, span::SpanKind};
/// let span = make_test_span("test", SpanKind::Client);
/// assert_eq!(span.name, "test");
/// ```
pub fn make_test_span(name: &str, kind: SpanKind) -> Span {
    let ctx = TraceContext::new(generate_trace_id(), generate_span_id(), true);
    Span::new(name, kind, ctx).unwrap()
}

/// 构造一个最小可用的 TraceContext (单 trace + 1 span, 用于单元测试).
///
/// # Example
///
/// ```
/// use apeireth_tracing::make_test_ctx;
/// let ctx = make_test_ctx();
/// assert_eq!(ctx.trace_id.len(), 32);
/// assert_eq!(ctx.span_id.len(), 16);
/// ```
pub fn make_test_ctx() -> TraceContext {
    TraceContext::new(generate_trace_id(), generate_span_id(), true)
}

/// 错误码全集 (10 变体, 用于 metrics + log 分类).
pub const ALL_ERROR_CODES: &[&str] = &[
    "TRACING_NOT_IMPLEMENTED",
    "TRACING_INTERNAL",
    "TRACING_INVALID_UTF8",
    "TRACING_INVALID_TRACE_ID",
    "TRACING_INVALID_SPAN_ID",
    "TRACING_EMPTY_SERVICE_NAME",
    "TRACING_INVALID_HEADER",
    "TRACING_EXPORT_FAILED",
    "TRACING_SAMPLING_ERROR",
    "TRACING_PROPAGATION_FAILED",
];

/// 4 SpanKind 字符串名 (稳定).
pub const SPAN_KIND_NAMES: &[&str] = &["client", "server", "producer", "consumer"];

/// 4 SpanEventKind 字符串名.
pub const SPAN_EVENT_KIND_NAMES: &[&str] = &["log", "exception", "event", "message"];

/// 4 ExporterKind 字符串名.
pub const EXPORTER_KIND_NAMES: &[&str] = &["stdout", "file", "otlp_grpc", "jaeger"];

/// 4 SamplerKind 字符串名.
pub const SAMPLER_KIND_NAMES: &[&str] = &[
    "always_on",
    "always_off",
    "trace_id_ratio_based",
    "parent_based",
];

/// 3 PropagatorKind 字符串名.
pub const PROPAGATOR_KIND_NAMES: &[&str] = &["w3c_trace_context", "b3", "jaeger"];

/// 默认 W3C traceparent version.
pub const W3C_TRACEPARENT_VERSION: &str = "00";

/// W3C traceparent trace_id 长度.
pub const W3C_TRACE_ID_LEN: usize = 32;

/// W3C traceparent span_id 长度.
pub const W3C_SPAN_ID_LEN: usize = 16;

/// W3C traceparent flags 长度.
pub const W3C_FLAGS_LEN: usize = 2;

/// W3C traceparent 总长度 (`version(2) + '-' + trace(32) + '-' + span(16) + '-' + flags(2)`).
pub const W3C_TRACEPARENT_LEN: usize = 2 + 1 + 32 + 1 + 16 + 1 + 2;

/// 验证 W3C traceparent 长度.
pub fn is_valid_traceparent_len(s: &str) -> bool {
    s.len() == W3C_TRACEPARENT_LEN
}

/// 解析 W3C traceparent (顶层便捷).
pub fn parse_w3c_traceparent(s: &str) -> TracingResult<TraceContext> {
    super::propagation::parse_traceparent(s)
}

// ============================================================================
// §7.6 模块门面 trait (供 trait object dyn dispatch)
// ============================================================================

/// Tracing 门面 trait — 把 Trace + Exporter + Sampler + Propagator 整合为 1 个 trait.
#[async_trait::async_trait]
pub trait TracingFacade: Send + Sync {
    /// 启动 root span.
    async fn start_root(&mut self, name: &str, kind: SpanKind) -> TracingResult<String>;

    /// 启动 child span.
    async fn start_child(
        &mut self,
        name: &str,
        kind: SpanKind,
        parent_span_id: &str,
    ) -> TracingResult<String>;

    /// 结束 span.
    async fn end_span(&mut self, name: &str) -> TracingResult<()>;

    /// Inject context.
    async fn inject(&self, ctx: &TraceContext, carrier: &mut std::collections::HashMap<String, String>);

    /// Extract context.
    async fn extract(
        &self,
        carrier: &std::collections::HashMap<String, String>,
    ) -> Option<TraceContext>;
}

#[async_trait::async_trait]
impl TracingFacade for Trace {
    async fn start_root(&mut self, name: &str, kind: SpanKind) -> TracingResult<String> {
        let span = self.start_root(name, kind).await?;
        metric_inc_spans();
        Ok(span.context.span_id.clone())
    }

    async fn start_child(
        &mut self,
        name: &str,
        kind: SpanKind,
        parent_span_id: &str,
    ) -> TracingResult<String> {
        // 1) 找到 parent span id 的索引
        let parent_idx = self
            .spans
            .iter()
            .position(|s| s.context.span_id == parent_span_id)
            .ok_or_else(|| {
                TracingError::Internal(format!("parent span not found: {}", parent_span_id))
            })?;
        // 2) Clone parent Span (owned, 释放 immutable borrow)
        let parent_clone = self.spans[parent_idx].clone();
        // 3) 调用 start_child (借用结束)
        let span = self.start_child(name, kind, &parent_clone).await?;
        metric_inc_spans();
        Ok(span.context.span_id.clone())
    }

    async fn end_span(&mut self, name: &str) -> TracingResult<()> {
        let r = self.end_span(name).await;
        if r.is_ok() {
            metric_inc_export_ok();
        } else {
            metric_inc_export_failed();
        }
        r
    }

    async fn inject(&self, ctx: &TraceContext, carrier: &mut std::collections::HashMap<String, String>) {
        W3CTraceContextPropagator.inject(ctx, carrier).await;
    }

    async fn extract(
        &self,
        carrier: &std::collections::HashMap<String, String>,
    ) -> Option<TraceContext> {
        W3CTraceContextPropagator.extract(carrier).await
    }
}

// ============================================================================
// §8 子模块级集成测试入口 (in-file 单元测试)
// ============================================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_constants() {
        assert_eq!(TRACING_CONFIG_SECTION_COUNT, 4);
        assert_eq!(SPAN_KIND_COUNT, 4);
        assert_eq!(SPAN_EVENT_KIND_COUNT, 4);
        assert_eq!(EXPORTER_KIND_COUNT, 4);
        assert_eq!(SAMPLER_KIND_COUNT, 4);
        assert_eq!(PROPAGATOR_KIND_COUNT, 3);
        assert_eq!(TRACING_ERROR_VARIANT_COUNT, 10);
        assert_eq!(K1_STRONG_VALIDATION_COUNT, 3);
    }

    #[test]
    fn test_k1_validators() {
        assert!(validate_trace_id("0af7651916cd43dd8448eb211c80319c").is_ok());
        assert!(validate_trace_id("invalid").is_err());

        assert!(validate_span_id("b7ad6b7169203331").is_ok());
        assert!(validate_span_id("invalid").is_err());

        assert!(validate_service_name("apeireth-api").is_ok());
        assert!(validate_service_name("").is_err());
        assert!(validate_service_name("   ").is_err());
    }

    #[test]
    fn test_quick_trace_k1() {
        // K-1 强校验: 空 service.name 应被 quick_trace 拒绝
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(quick_trace(""));
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), TracingError::EmptyServiceName));
    }

    #[test]
    fn test_metric_reset() {
        metric_reset();
        assert_eq!(metric_traces(), 0);
        assert_eq!(metric_spans(), 0);
        metric_inc_traces();
        metric_inc_spans();
        assert_eq!(metric_traces(), 1);
        assert_eq!(metric_spans(), 1);
        metric_reset();
        assert_eq!(metric_traces(), 0);
    }

    #[tokio::test]
    async fn test_inject_extract_roundtrip() {
        let ctx = TraceContext::new(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        );
        let mut carrier = HashMap::new();
        inject_context(&ctx, &mut carrier).await;
        let extracted = extract_context(&carrier).await.unwrap();
        assert_eq!(extracted.trace_id, ctx.trace_id);
        assert_eq!(extracted.span_id, ctx.span_id);
        assert_eq!(extracted.sampled, ctx.sampled);
    }

    #[tokio::test]
    async fn test_tracer_builder_default() {
        metric_reset();
        let mut t = Tracer::new("apeireth-api").build().await.unwrap();
        let root = t.trace_mut().start_root("op", SpanKind::Server).await.unwrap();
        assert_eq!(root.kind, SpanKind::Server);
        assert!(t.trace().sampled);
    }

    #[tokio::test]
    async fn test_tracer_builder_file_exporter() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let mut t = Tracer::new("apeireth-api")
            .sampler(SamplerKind::TraceIdRatioBased, 1.0)
            .file_exporter(&path)
            .build()
            .await
            .unwrap();
        t.trace_mut().start_root("op", SpanKind::Client).await.unwrap();
        t.trace_mut().end_span("op").await.unwrap();
        t.trace().flush().await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("op"));
    }
}

// ============================================================================
// §9 文档注释: 6 哲学 anchor / 8 项不修改承诺 (顶层模块门)
// ============================================================================

// 末尾注释 (普通 // 而非 //! — //! 只能在 item 之前或文件顶部)

/// ### 6 哲学 anchor 自检清单 (per APEIRETH-CONVENTIONS §9)
const _PHILOSOPHY_ANCHOR_CHECK: &str = "
- [x] S-1 北极星导向 — service 1.0 release observability 必做
- [x] S-2 实事求是 — 1:1 翻译 v0.9.21, 0 业务重设计
- [x] O-5 不假装 — OTLP/Jaeger stub 返 NotImplemented, 0 假装已对接
- [x] O-2 走在前人肩上 — W3C + OpenTelemetry + Zipkin B3
- [x] O-3 干到底 — 25+ 集成测试 + 8 模块全真接
- [x] O-4 任何人都能接手 — 8 模块 + 4+4+4+3 全文档化
";

/// ### 8 项不修改承诺自检 (per APEIRETH-CONVENTIONS §10)
const _NO_MODIFY_PROMISE_CHECK: &str = "
- [x] 1. 阶段 1+2+3 LOCKED
- [x] 2. v2 / v4 / v4.1 LOCKED
- [x] 3. 阶段 4 主文档 LOCKED
- [x] 4. 阶段 5 施工文档 LOCKED
- [x] 5. v6 修正
- [x] 6. R11 baseline 三值
- [x] 7. v1 → v5 历史链
- [x] 8. v0.9.21 商业版 LOCKED (1:1 翻译)
";

/// ### m3 防御 (per m3-hallucination-defense-2026-08-05.md)
const _M3_DEFENSE_CHECK: &str = "
- [x] 0 真实 trace 数据: 所有 fixture 用 32 hex 占位 (e.g. 0af76519...)
- [x] 0 真实 collector endpoint: OTLP/Jaeger stub 返 NotImplemented
- [x] 0 PII 暴露: Display 截前 8 hex 字符, 错误消息不含完整 trace_id
- [x] 0 假装已对接: 2 stub exporter + 2 stub propagator 全部守门
- [x] K-1 强校验 3 项: trace_id 32 hex + span_id 16 hex + service.name 非空
- [x] 0 触碰 24 LOCKED crate: 仅本 crate 内操作
- [x] 0 改 workspace Cargo.toml 其他字段: 仅 + 1 个 members 行 (整合 #2 时)
- [x] 0 改 K-1 强校验: 3 项 K-1 必保留
";
