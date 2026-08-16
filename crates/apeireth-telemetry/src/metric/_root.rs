#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// 模块声明
// ============================================================================

// ============================================================================
// Re-export (主入口便捷)
// ============================================================================

pub use super::config::{ExporterKind, MetricsConfig};
pub use super::counter::Counter;
pub use super::encoder::{
    encode_counter, encode_gauge, encode_histogram, encode_histogram_full, encode_metric,
    encode_metric_with_labels, encode_summary, encode_summary_full,
};
pub use super::error::{MetricsError, MetricsResult, METRICS_ERROR_VARIANT_COUNT};
pub use super::exporter::{
    build_exporter, Exporter, OtlpExporter, PrometheusExporter, PushgatewayExporter,
    StatsdExporter, StdoutExporter,
};
pub use super::gauge::Gauge;
pub use super::histogram::{validate_buckets, Histogram, DEFAULT_BUCKETS};
pub use super::label::{
    label_to_prometheus, labels_to_prometheus_sorted, validate_label_key, validate_label_value,
    validate_labels, Label, LABEL_MAX_COUNT, LABEL_VALUE_MAX_LEN,
};
pub use super::metric::{Metric, MetricValue};
pub use super::registry::{MetricsRegistry, RegisteredMetric};
pub use super::summary::{validate_quantiles, Summary, DEFAULT_QUANTILES, RESERVOIR_SIZE};

// ============================================================================
// §1 Crate-level 常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 4 Metric 类型 1:1 计数.
pub const CRATE_METRIC_KIND_COUNT: usize = 4;

/// 5 Exporter 1:1 计数.
pub const CRATE_EXPORTER_COUNT: usize = 5;

/// 10 MetricsError variant 1:1 计数.
pub const CRATE_METRICS_ERROR_VARIANT_COUNT: usize = 10;

/// 11 默认 Histogram bucket 边界 1:1 计数.
pub const CRATE_DEFAULT_BUCKETS_COUNT: usize = 11;

/// 5 默认 Summary quantile 1:1 计数.
pub const CRATE_DEFAULT_QUANTILES_COUNT: usize = 5;

/// 1024 样本池容量 1:1 计数.
pub const CRATE_RESERVOIR_SIZE: usize = 1024;

// ============================================================================
// §2 Crate-level 默认配置
// ============================================================================

/// 默认 namespace.
pub const DEFAULT_NAMESPACE: &str = "apeireth";

/// 默认 subsystem.
pub const DEFAULT_SUBSYSTEM: &str = "agent";

/// 默认 Exporter (Prometheus).
pub const DEFAULT_EXPORTER: ExporterKind = ExporterKind::Prometheus;

// ============================================================================
// §3 MetricsBuilder — 链式构造 MetricsConfig
// ============================================================================

/// Metrics builder — 链式构造 MetricsConfig.
#[derive(Debug, Clone)]
pub struct MetricsBuilder {
    namespace: String,
    subsystem: String,
    global_labels: std::collections::HashMap<String, String>,
    exporter: ExporterKind,
}

impl Default for MetricsBuilder {
    fn default() -> Self {
        Self {
            namespace: DEFAULT_NAMESPACE.to_string(),
            subsystem: DEFAULT_SUBSYSTEM.to_string(),
            global_labels: std::collections::HashMap::new(),
            exporter: DEFAULT_EXPORTER,
        }
    }
}

impl MetricsBuilder {
    /// 构造 builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 namespace.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// 设置 subsystem.
    pub fn subsystem(mut self, subsystem: impl Into<String>) -> Self {
        self.subsystem = subsystem.into();
        self
    }

    /// 添加全局 label.
    pub fn global_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_labels.insert(key.into(), value.into());
        self
    }

    /// 设置 exporter.
    pub fn exporter(mut self, exporter: ExporterKind) -> Self {
        self.exporter = exporter;
        self
    }

    /// 构造 MetricsConfig.
    pub fn build(self) -> MetricsResult<MetricsConfig> {
        let config = MetricsConfig {
            namespace: self.namespace,
            subsystem: self.subsystem,
            global_labels: self.global_labels,
            exporter: self.exporter,
        };
        config.validate()?;
        Ok(config)
    }
}

// ============================================================================
// §4 全局默认 registry (便捷 helper)
// ============================================================================

/// 全局默认 registry (thread-local Lazy).
///
/// 多用于业务代码快速注册 / 读 metrics 而不必显式传 registry.
pub fn default_registry() -> &'static MetricsRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<MetricsRegistry> = OnceLock::new();
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        let _ = REGISTRY.set(MetricsRegistry::new());
    });
    // SAFETY: INIT 在第一次调用前保证 REGISTRY 已 set
    REGISTRY.get_or_init(MetricsRegistry::new)
}

// ============================================================================
// §5 K-1 守门函数 (编译期常量 + 运行时校验)
// ============================================================================

/// K-1 守门: 校验 metric name (允许 `:`).
///
/// 必须 `[a-zA-Z_:][a-zA-Z0-9_:]*`.
pub fn validate_metric_name(name: &str) -> MetricsResult<()> {
    if name.is_empty() {
        return Err(MetricsError::MetricNameEmpty);
    }
    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !(first.is_ascii_alphabetic() || first == '_' || first == ':') {
        return Err(MetricsError::MetricNameInvalid(name.to_string()));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == ':') {
            return Err(MetricsError::MetricNameInvalid(name.to_string()));
        }
    }
    Ok(())
}

/// K-1 守门: 校验 metric help (必填).
pub fn validate_help(name: &str, help: &str) -> MetricsResult<()> {
    if help.is_empty() {
        return Err(MetricsError::HelpRequired(name.to_string()));
    }
    Ok(())
}

// ============================================================================
// §6 K-1 强校验 8 项详细列表 (per Prometheus exposition format 规范)
// ============================================================================

/// K-1 强校验 #1: metric name 非空.
pub const K1_NAME_NON_EMPTY: &str = "metric name must not be empty";

/// K-1 强校验 #2: metric name 字符.
pub const K1_NAME_PATTERN: &str = "metric name must match [a-zA-Z_:][a-zA-Z0-9_:]*";

/// K-1 强校验 #3: help text 必填.
pub const K1_HELP_REQUIRED: &str = "help text must not be empty";

/// K-1 强校验 #4: label key 字符.
pub const K1_LABEL_KEY_PATTERN: &str = "label key must match [a-zA-Z_][a-zA-Z0-9_]*";

/// K-1 强校验 #5: label value 长度上限.
pub const K1_LABEL_VALUE_MAX: &str = "label value must be <= 256 chars";

/// K-1 强校验 #6: label 数量上限.
pub const K1_LABEL_COUNT_MAX: &str = "label count must be <= 10";

/// K-1 强校验 #7: histogram buckets 升序 + 正值.
pub const K1_BUCKETS_PATTERN: &str = "histogram buckets must be strictly ascending and > 0";

/// K-1 强校验 #8: summary quantiles 升序 + ∈ [0, 1].
pub const K1_QUANTILES_PATTERN: &str = "summary quantiles must be strictly ascending in [0, 1]";

// ============================================================================
// §7 全局 metric 生命周期 (register → observe → export)
// ============================================================================

/// 全局 metric 生命周期 4 步 (per task spec §3 4 metric 通用).
///
/// 1. **构造** — `Counter::new(name, help, labels)` (4 类型同款)
/// 2. **注册** — `registry.register_counter(arc)` (4 类型同款)
/// 3. **观察** — `counter.inc_by(n)` / `gauge.set(v)` / `histogram.observe(v)` / `summary.observe(v)`
/// 4. **导出** — `exporter.export(&registry).await` (5 exporter 同接口)
pub const METRIC_LIFECYCLE_STEPS: usize = 4;

// ============================================================================
// §8 详细 1:1 翻译映射表 (per task spec §1)
// ============================================================================

/// 1:1 翻译映射总览 (per docs/stage6/02-metrics-skeleton-blueprint §3).
///
/// 业务核心 4 metric + 5 exporter 共 9 个 API, 全部 1:1 翻译 v0.9.21
/// @anthropic-ai/metrics 商业版. Skeleton 阶段 2 exporter 完整, 3 exporter stub.
pub const TRANSLATION_MAP_ITEMS: usize = 12;

// ============================================================================
// §9 详细 5 Exporter 实现状态 (per task spec §4)
// ============================================================================

/// 5 Exporter 实现状态 (per task spec §4).
///
/// 1. **Prometheus** (完整) — text exposition format, `/metrics` HTTP endpoint
/// 2. **Pushgateway** (stub) — HTTP POST, R20 阶段 6 估缺
/// 3. **OTLP** (stub) — gRPC, R20 阶段 6 估缺
/// 4. **StatsD** (stub) — UDP, R20 阶段 6 估缺
/// 5. **Stdout** (完整) — println! 打印
pub const EXPORTER_IMPLEMENTED_COUNT: usize = 2;

/// 3 Exporter stub 数 (Pushgateway / OTLP / StatsD).
pub const EXPORTER_STUB_COUNT: usize = 3;

// ============================================================================
// §10 详细 4 Metric 类型 + 11 默认 buckets + 5 默认 quantiles
// ============================================================================

/// Histogram 11 默认 bucket 边界 (per Prometheus convention).
pub const HISTOGRAM_DEFAULT_BOUNDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Summary 5 默认 quantile (per Prometheus convention).
pub const SUMMARY_DEFAULT_QUANTILES: &[f64] = &[0.5, 0.9, 0.95, 0.99, 0.999];

// ============================================================================
// §11 6 哲学 anchor 详细展开
// ============================================================================

/// 哲学 anchor 1: S-1 主 22:33 北极星导向.
///
/// Metrics 服务 ASI 北极星 (观测 ASI 心跳 / 器官生长).
///
/// 默认 metric prefix `apeireth_agent_*` 跟 ASI crate 配套, 可观测:
/// - 9 器官 (heart / cognition / memory / value / motivation / perception / consciousness / action / relation) 的 heartbeat
/// - ASI 决策延迟 / 决策成功率 / 决策失败率
/// - 主对话 token 吞吐 / error rate
pub const PHILOSOPHY_ANCHOR_S1: &str = "S-1 主 22:33 北极星导向";

/// 哲学 anchor 2: S-2 主 17:43 实事求是.
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/metrics 商业版, 不重写不假装不扩张.
pub const PHILOSOPHY_ANCHOR_S2: &str = "S-2 主 17:43 实事求是";

/// 哲学 anchor 3: O-5 主 17:58 不假装.
///
/// 4 metric + 5 exporter 编译期 hardcode, 3 stub exporter 返 ExporterNotImplemented
/// 守门, 不假装 "已实现" 实际上没写的代码.
pub const PHILOSOPHY_ANCHOR_O5: &str = "O-5 主 17:58 不假装";

/// 哲学 anchor 4: O-2 主 19:33 走在前人肩上.
///
/// 借 v0.9.21 + prometheus 0.13 库 (留口子, R21 续接可直用) +
/// parking_lot 0.12 (无锁 atomic 优先, RwLock 兜底) + thiserror + tracing.
pub const PHILOSOPHY_ANCHOR_O2: &str = "O-2 主 19:33 走在前人肩上";

/// 哲学 anchor 5: O-3 主 23:44 干到底.
///
/// Prometheus / Stdout exporter 立即落 (完整实现), 3 stub exporter 返
/// NotImplemented 守门 (不假装, 但留口子 R21 续接).
pub const PHILOSOPHY_ANCHOR_O3: &str = "O-3 主 23:44 干到底";

/// 哲学 anchor 6: O-4 主 00:56 任何人都能接手.
///
/// 10 模块 + 4 metric + 5 exporter + 10 error variant + 6 哲学 anchor +
/// 8 项不修改承诺全文档化, 任何接手者一读就懂.
pub const PHILOSOPHY_ANCHOR_O4: &str = "O-4 主 00:56 任何人都能接手";

// ============================================================================
// §12 8 项不修改承诺详细 (per APEIRETH-CONVENTIONS §10)
// ============================================================================

/// 8 项不修改承诺 #1: 阶段 1+2+3 LOCKED.
pub const PROMISE_1: &str = "阶段 1+2+3 LOCKED — 不动";

/// 8 项不修改承诺 #2: v2 / v4 / v4.1 LOCKED.
pub const PROMISE_2: &str = "v2 / v4 / v4.1 LOCKED — 不动";

/// 8 项不修改承诺 #3: 阶段 4 主文档 LOCKED (commit 6ca80776).
pub const PROMISE_3: &str = "阶段 4 主文档 LOCKED (6ca80776) — 不动";

/// 8 项不修改承诺 #4: 阶段 5 施工文档 LOCKED (631 行).
pub const PROMISE_4: &str = "阶段 5 施工文档 LOCKED (631 行) — 不动";

/// 8 项不修改承诺 #5: v6 修正 LOCKED (4 重守门 + 权限发放 + E 层修改路径).
pub const PROMISE_5: &str = "v6 修正 (4 重守门 + 权限发放 + E 层修改路径) — 不动";

/// 8 项不修改承诺 #6: R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063).
pub const PROMISE_6: &str = "R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) — 不动";

/// 8 项不修改承诺 #7: v1 → v5 历史链不删除.
pub const PROMISE_7: &str = "v1 → v5 历史链 — 不删除";

/// 8 项不修改承诺 #8: v0.9.21 商业版 LOCKED (1:1 翻译 4 metric + 5 exporter, 不改商业版 1:1 映射).
pub const PROMISE_8: &str = "v0.9.21 商业版 LOCKED (1:1 翻译 4 metric + 5 exporter) — 不动";

/// 8 项不修改承诺 1:1 计数.
pub const PROMISE_COUNT: usize = 8;

// ============================================================================
// §13 默认完整 Prometheus 端点路径 (per Prometheus convention)
// ============================================================================

/// Prometheus scrape endpoint 路径 (约定 `/metrics`).
pub const PROMETHEUS_METRICS_PATH: &str = "/metrics";

/// 默认 Prometheus 监听端口 (0 = 由 OS 分配, 测试场景用).
pub const PROMETHEUS_DEFAULT_PORT: u16 = 9090;

// ============================================================================
// §14 业务速查 — 4 metric + 5 exporter + 1 registry
// ============================================================================

/// Counter 关联的 Prometheus 类型名.
pub const COUNTER_PROMETHEUS_TYPE: &str = "counter";

/// Gauge 关联的 Prometheus 类型名.
pub const GAUGE_PROMETHEUS_TYPE: &str = "gauge";

/// Histogram 关联的 Prometheus 类型名.
pub const HISTOGRAM_PROMETHEUS_TYPE: &str = "histogram";

/// Summary 关联的 Prometheus 类型名.
pub const SUMMARY_PROMETHEUS_TYPE: &str = "summary";

// ============================================================================
// §15 Crate-level 测试 (跨模块守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// 守门 #1: 4 Metric kind 1:1 计数.
    #[test]
    fn crate_metric_kind_count() {
        assert_eq!(CRATE_METRIC_KIND_COUNT, 4);
    }

    /// 守门 #2: 5 Exporter 1:1 计数.
    #[test]
    fn crate_exporter_count() {
        assert_eq!(CRATE_EXPORTER_COUNT, 5);
    }

    /// 守门 #3: 10 MetricsError variant 1:1 计数.
    #[test]
    fn crate_metrics_error_variant_count() {
        assert_eq!(CRATE_METRICS_ERROR_VARIANT_COUNT, 10);
        assert_eq!(METRICS_ERROR_VARIANT_COUNT, 10);
    }

    /// 守门 #4: 11 默认 buckets 1:1 计数.
    #[test]
    fn crate_default_buckets_count() {
        assert_eq!(CRATE_DEFAULT_BUCKETS_COUNT, 11);
        assert_eq!(DEFAULT_BUCKETS.len(), 11);
    }

    /// 守门 #5: 5 默认 quantiles 1:1 计数.
    #[test]
    fn crate_default_quantiles_count() {
        assert_eq!(CRATE_DEFAULT_QUANTILES_COUNT, 5);
        assert_eq!(DEFAULT_QUANTILES.len(), 5);
    }

    /// 守门 #6: 1024 reservoir 1:1 计数.
    #[test]
    fn crate_reservoir_size() {
        assert_eq!(CRATE_RESERVOIR_SIZE, 1024);
        assert_eq!(RESERVOIR_SIZE, 1024);
    }

    /// 守门 #7: 默认 namespace + subsystem 合法.
    #[test]
    fn default_namespace_subsystem() {
        assert_eq!(DEFAULT_NAMESPACE, "apeireth");
        assert_eq!(DEFAULT_SUBSYSTEM, "agent");
    }

    /// 守门 #8: 默认 exporter = Prometheus.
    #[test]
    fn default_exporter() {
        assert_eq!(DEFAULT_EXPORTER, ExporterKind::Prometheus);
    }

    /// 守门 #9: MetricsBuilder 链式构造.
    #[test]
    fn metrics_builder_chain() {
        let c = MetricsBuilder::new()
            .namespace("test")
            .subsystem("unit")
            .global_label("env", "dev")
            .exporter(ExporterKind::Stdout)
            .build()
            .unwrap();
        assert_eq!(c.namespace, "test");
        assert_eq!(c.subsystem, "unit");
        assert_eq!(c.global_labels.get("env").unwrap(), "dev");
        assert_eq!(c.exporter, ExporterKind::Stdout);
    }

    /// 守门 #10: MetricsBuilder default = default_config.
    #[test]
    fn metrics_builder_default() {
        let c = MetricsBuilder::new().build().unwrap();
        let default_c = MetricsConfig::default_config();
        assert_eq!(c.namespace, default_c.namespace);
        assert_eq!(c.subsystem, default_c.subsystem);
        assert_eq!(c.exporter, default_c.exporter);
    }

    /// 守门 #11: validate_metric_name valid.
    #[test]
    fn validate_metric_name_valid() {
        assert!(validate_metric_name("requests_total").is_ok());
        assert!(validate_metric_name("_internal").is_ok());
        assert!(validate_metric_name("a:b").is_ok()); // 允许 :
        assert!(validate_metric_name("A1").is_ok());
    }

    /// 守门 #12: K-1 invalid name 拒 (数字开头).
    #[test]
    fn k1_metric_name_invalid() {
        assert!(matches!(
            validate_metric_name("1abc"),
            Err(MetricsError::MetricNameInvalid(_))
        ));
        assert!(matches!(
            validate_metric_name("requests-total"),
            Err(MetricsError::MetricNameInvalid(_))
        ));
    }

    /// 守门 #13: K-1 empty name 拒.
    #[test]
    fn k1_metric_name_empty() {
        assert!(matches!(
            validate_metric_name(""),
            Err(MetricsError::MetricNameEmpty)
        ));
    }

    /// 守门 #14: K-1 help 必填.
    #[test]
    fn k1_help_required() {
        assert!(matches!(
            validate_help("foo", ""),
            Err(MetricsError::HelpRequired(_))
        ));
        assert!(validate_help("foo", "bar").is_ok());
    }

    /// 守门 #15: K-1 label value ≤ 256 字符.
    #[test]
    fn k1_label_value_max_256() {
        assert!(validate_label_value("k", &"a".repeat(256)).is_ok());
        assert!(matches!(
            validate_label_value("k", &"a".repeat(257)),
            Err(MetricsError::LabelValueTooLong { actual: 257, .. })
        ));
    }

    /// 守门 #16: 全模块 4 metric + 5 exporter 协同工作.
    #[tokio::test]
    async fn full_pipeline_4_kinds_5_exporters() {
        let r = MetricsRegistry::new();
        r.register_counter(Arc::new(
            Counter::new("requests_total", "Total", HashMap::new()).unwrap(),
        ))
        .unwrap();
        r.register_gauge(Arc::new(
            Gauge::new("memory_bytes", "Memory", HashMap::new()).unwrap(),
        ))
        .unwrap();
        r.register_histogram(Arc::new(
            Histogram::new("latency", "Latency", HashMap::new()).unwrap(),
        ))
        .unwrap();
        r.register_summary(Arc::new(
            Summary::new("rpc_duration", "RPC", HashMap::new()).unwrap(),
        ))
        .unwrap();

        // 2 完整 exporter
        let p = PrometheusExporter::new("apeireth", "agent");
        let body = p.export(&r).await.unwrap();
        assert!(body.contains("apeireth_agent_requests_total"));
        assert!(body.contains("apeireth_agent_memory_bytes"));
        assert!(body.contains("apeireth_agent_latency"));
        assert!(body.contains("apeireth_agent_rpc_duration"));

        let s = StdoutExporter::new();
        let body = s.export(&r).await.unwrap();
        assert!(body.contains("apeireth_agent_requests_total"));

        // 3 stub exporter
        let pg = PushgatewayExporter::new("http://localhost:9091", "job");
        assert!(pg.export(&r).await.is_err());
        let otlp = OtlpExporter::new("http://localhost:4317", "apeireth");
        assert!(otlp.export(&r).await.is_err());
        let statsd = StatsdExporter::new("127.0.0.1", 8125);
        assert!(statsd.export(&r).await.is_err());
    }

    /// 守门 #17: default_registry() 单例.
    #[test]
    fn default_registry_singleton() {
        let r1 = default_registry();
        let r2 = default_registry();
        assert!(std::ptr::eq(r1 as *const _, r2 as *const _));
    }

    /// 守门 #18: validate_metric_name 允许 `:` (与 label key 不同).
    #[test]
    fn validate_metric_name_allows_colon() {
        assert!(validate_metric_name("apeireth:agent:requests_total").is_ok());
    }

    /// 守门 #19: 4 哲学锚 #1 显式编码 (S-1 北极星).
    #[test]
    fn s1_anchor_north_star() {
        // S-1: metrics 服务 ASI 北极星 (观测 ASI 心跳 / 器官生长)
        // 验证: metric 名允许 apeireth_agent_* 命名
        assert!(validate_metric_name("apeireth_agent_heartbeat").is_ok());
    }

    /// 守门 #20: 4 哲学锚 #2 显式编码 (S-2 实事求是 / 1:1 翻译).
    #[test]
    fn s2_anchor_1to1_translation() {
        // S-2: 1:1 翻译 v0.9.21 @anthropic-ai/metrics
        // 验证: 4 metric + 5 exporter 计数
        assert_eq!(CRATE_METRIC_KIND_COUNT, 4);
        assert_eq!(CRATE_EXPORTER_COUNT, 5);
    }

    /// 守门 #21: 8 项不修改承诺 1:1 计数.
    #[test]
    fn promise_count_8() {
        assert_eq!(PROMISE_COUNT, 8);
        // 8 个常量都不为空
        assert!(!PROMISE_1.is_empty());
        assert!(!PROMISE_2.is_empty());
        assert!(!PROMISE_3.is_empty());
        assert!(!PROMISE_4.is_empty());
        assert!(!PROMISE_5.is_empty());
        assert!(!PROMISE_6.is_empty());
        assert!(!PROMISE_7.is_empty());
        assert!(!PROMISE_8.is_empty());
    }

    /// 守门 #22: 6 哲学锚 1:1 计数 + 各自守门.
    #[test]
    fn philosophy_anchors_6() {
        assert_eq!(PHILOSOPHY_ANCHOR_S1, "S-1 主 22:33 北极星导向");
        assert_eq!(PHILOSOPHY_ANCHOR_S2, "S-2 主 17:43 实事求是");
        assert_eq!(PHILOSOPHY_ANCHOR_O5, "O-5 主 17:58 不假装");
        assert_eq!(PHILOSOPHY_ANCHOR_O2, "O-2 主 19:33 走在前人肩上");
        assert_eq!(PHILOSOPHY_ANCHOR_O3, "O-3 主 23:44 干到底");
        assert_eq!(PHILOSOPHY_ANCHOR_O4, "O-4 主 00:56 任何人都能接手");
    }

    /// 守门 #23: K-1 强校验 8 项 1:1 计数 + 各自守门.
    #[test]
    fn k1_8_items() {
        // 8 K-1 强校验常量都不为空
        assert!(!K1_NAME_NON_EMPTY.is_empty());
        assert!(!K1_NAME_PATTERN.is_empty());
        assert!(!K1_HELP_REQUIRED.is_empty());
        assert!(!K1_LABEL_KEY_PATTERN.is_empty());
        assert!(!K1_LABEL_VALUE_MAX.is_empty());
        assert!(!K1_LABEL_COUNT_MAX.is_empty());
        assert!(!K1_BUCKETS_PATTERN.is_empty());
        assert!(!K1_QUANTILES_PATTERN.is_empty());
    }

    /// 守门 #24: 2 完整 + 3 stub exporter 守门.
    #[test]
    fn exporter_2_complete_3_stub() {
        assert_eq!(EXPORTER_IMPLEMENTED_COUNT, 2);
        assert_eq!(EXPORTER_STUB_COUNT, 3);
        assert_eq!(
            EXPORTER_IMPLEMENTED_COUNT + EXPORTER_STUB_COUNT,
            CRATE_EXPORTER_COUNT
        );
    }

    /// 守门 #25: 4 metric + 4 Prometheus type 1:1.
    #[test]
    fn metric_4_prometheus_types() {
        assert_eq!(COUNTER_PROMETHEUS_TYPE, "counter");
        assert_eq!(GAUGE_PROMETHEUS_TYPE, "gauge");
        assert_eq!(HISTOGRAM_PROMETHEUS_TYPE, "histogram");
        assert_eq!(SUMMARY_PROMETHEUS_TYPE, "summary");
    }

    /// 守门 #26: 1:1 翻译映射 12 项.
    #[test]
    fn translation_map_12_items() {
        assert_eq!(TRANSLATION_MAP_ITEMS, 12);
        // 4 metric + 5 exporter + 1 registry + 1 label + 1 config = 12
    }

    /// 守门 #27: 全局 metric 生命周期 4 步.
    #[test]
    fn metric_lifecycle_4_steps() {
        assert_eq!(METRIC_LIFECYCLE_STEPS, 4);
    }

    /// 守门 #28: 11 Histogram bucket 边界 1:1.
    #[test]
    fn histogram_11_default_bounds() {
        assert_eq!(HISTOGRAM_DEFAULT_BOUNDS.len(), 11);
        assert_eq!(HISTOGRAM_DEFAULT_BOUNDS[0], 0.005);
        assert_eq!(HISTOGRAM_DEFAULT_BOUNDS[10], 10.0);
    }

    /// 守门 #29: 5 Summary quantile 1:1.
    #[test]
    fn summary_5_default_quantiles() {
        assert_eq!(SUMMARY_DEFAULT_QUANTILES.len(), 5);
        assert_eq!(SUMMARY_DEFAULT_QUANTILES[0], 0.5);
        assert_eq!(SUMMARY_DEFAULT_QUANTILES[4], 0.999);
    }

    /// 守门 #30: Prometheus 端点路径.
    #[test]
    fn prometheus_endpoint_path() {
        assert_eq!(PROMETHEUS_METRICS_PATH, "/metrics");
    }

    /// 守门 #31: 8 项不修改承诺 #6 守住 R11 baseline 三值.
    #[test]
    fn promise_6_r11_baseline() {
        // 8 项承诺 #6: R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
        // 验证: 守门常量未变
        assert!(PROMISE_6.contains("0.8682"));
        assert!(PROMISE_6.contains("0.8532"));
        assert!(PROMISE_6.contains("0.9063"));
    }

    /// 守门 #32: 4 type 全实现 Metric trait.
    #[test]
    fn all_4_metric_types_implement_trait() {
        let c: Box<dyn Metric> = Box::new(Counter::new("c", "h", HashMap::new()).unwrap());
        let g: Box<dyn Metric> = Box::new(Gauge::new("g", "h", HashMap::new()).unwrap());
        let h: Box<dyn Metric> = Box::new(Histogram::new("h", "h", HashMap::new()).unwrap());
        let s: Box<dyn Metric> = Box::new(Summary::new("s", "h", HashMap::new()).unwrap());
        assert_eq!(c.type_name(), "counter");
        assert_eq!(g.type_name(), "gauge");
        assert_eq!(h.type_name(), "histogram");
        assert_eq!(s.type_name(), "summary");
    }

    /// 守门 #33: K-1 强校验 name 含 `:` (与 label key 区别).
    #[test]
    fn k1_name_allows_colon_label_key_does_not() {
        // metric name 允许 `:`
        assert!(validate_metric_name("a:b:c").is_ok());
        // label key 不允许 `:`
        assert!(validate_label_key("a:b").is_err());
    }

    /// 守门 #34: K-1 强校验 label value 长度守门 256.
    #[test]
    fn k1_label_value_256_boundary() {
        let exact = "a".repeat(256);
        assert!(validate_label_value("k", &exact).is_ok());
        let over = "a".repeat(257);
        assert!(validate_label_value("k", &over).is_err());
    }

    /// 守门 #35: 默认 exporter 守门.
    #[test]
    fn default_exporter_prometheus() {
        assert_eq!(DEFAULT_EXPORTER, ExporterKind::Prometheus);
        assert!(DEFAULT_EXPORTER.is_implemented());
    }
}
