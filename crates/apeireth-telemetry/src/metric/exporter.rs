//! # Exporter — 5 exporter 实现
//!
//! 1:1 翻译 v0.9.21 @anthropic-ai/metrics 5 exporter:
//!
//! | # | Exporter          | 实现状态        | 输出                        |
//! |---|-------------------|-----------------|-----------------------------|
//! | 1 | Prometheus        | ✅ 完整         | text exposition format      |
//! | 2 | Pushgateway       | 🚧 stub         | HTTP POST (R20 估缺)        |
//! | 3 | OTLP              | 🚧 stub         | gRPC (R20 估缺)             |
//! | 4 | StatsD            | 🚧 stub         | UDP packet (R20 估缺)       |
//! | 5 | Stdout            | ✅ 完整         | println! 打印               |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::sync::Arc;

use async_trait::async_trait;

use super::config::ExporterKind;
use super::encoder;
use super::error::{MetricsError, MetricsResult};
use super::registry::{MetricsRegistry, RegisteredMetric};

// ============================================================================
// §1 Exporter trait
// ============================================================================

/// Exporter 通用 trait (5 exporter 共享接口).
#[async_trait]
pub trait Exporter: Send + Sync {
    /// 导出 name (e.g. "prometheus", "pushgateway").
    fn name(&self) -> &'static str;

    /// Exporter 类型.
    fn kind(&self) -> ExporterKind;

    /// 导出 registry 全部 metric.
    async fn export(&self, registry: &MetricsRegistry) -> MetricsResult<String>;

    /// 是否已实现 (false → stub 返 Err).
    fn is_implemented(&self) -> bool;
}

// ============================================================================
// §2 PrometheusExporter — 完整实现
// ============================================================================

/// Prometheus text exposition format exporter.
///
/// 输出 Prometheus 0.0.4 规范的文本格式, 可直接被 Prometheus server scrape.
pub struct PrometheusExporter {
    /// namespace 前缀 (e.g. "apeireth").
    namespace: String,
    /// subsystem 前缀 (e.g. "agent").
    subsystem: String,
}

impl PrometheusExporter {
    /// 构造 Prometheus exporter.
    pub fn new(namespace: impl Into<String>, subsystem: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            subsystem: subsystem.into(),
        }
    }
}

#[async_trait]
impl Exporter for PrometheusExporter {
    fn name(&self) -> &'static str {
        "prometheus"
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Prometheus
    }

    fn is_implemented(&self) -> bool {
        true
    }

    async fn export(&self, registry: &MetricsRegistry) -> MetricsResult<String> {
        let mut out = String::with_capacity(2048);
        for metric in registry.list() {
            let full_name = format!("{}_{}_{}", self.namespace, self.subsystem, metric.name());
            match &metric {
                RegisteredMetric::Counter(c) => {
                    out.push_str(&encoder::encode_counter(&full_name, metric.help(), c)?);
                }
                RegisteredMetric::Gauge(g) => {
                    out.push_str(&encoder::encode_gauge(&full_name, metric.help(), g)?);
                }
                RegisteredMetric::Histogram(h) => {
                    out.push_str(&encoder::encode_histogram_full(
                        &full_name,
                        metric.help(),
                        h,
                    )?);
                }
                RegisteredMetric::Summary(s) => {
                    out.push_str(&encoder::encode_summary_full(&full_name, metric.help(), s)?);
                }
            }
        }
        Ok(out)
    }
}

// ============================================================================
// §3 PushgatewayExporter — stub
// ============================================================================

/// Pushgateway exporter (stub).
///
/// R20 阶段 6 估缺: 仅保留接口和 K-1 守门, 实际 push 留 R21 续接.
pub struct PushgatewayExporter {
    /// Pushgateway URL (e.g. "http://localhost:9091").
    url: String,
    /// Job label.
    job: String,
}

impl PushgatewayExporter {
    /// 构造 (K-1 强校验: 实际 push 返 NotImplemented, 构造本身不拒).
    pub fn new(url: impl Into<String>, job: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            job: job.into(),
        }
    }

    /// Pushgateway URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Job label.
    pub fn job(&self) -> &str {
        &self.job
    }
}

#[async_trait]
impl Exporter for PushgatewayExporter {
    fn name(&self) -> &'static str {
        "pushgateway"
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Pushgateway
    }

    fn is_implemented(&self) -> bool {
        false
    }

    async fn export(&self, _registry: &MetricsRegistry) -> MetricsResult<String> {
        Err(MetricsError::ExporterNotImplemented(
            ExporterKind::Pushgateway.as_str().to_string(),
        ))
    }
}

// ============================================================================
// §4 OtlpExporter — stub
// ============================================================================

/// OTLP (OpenTelemetry Protocol) exporter (stub).
///
/// R20 阶段 6 估缺: 实际 gRPC push 留 R21 续接.
pub struct OtlpExporter {
    /// OTLP endpoint (e.g. "http://localhost:4317").
    endpoint: String,
    /// Service name resource attribute.
    service_name: String,
}

impl OtlpExporter {
    /// 构造.
    pub fn new(endpoint: impl Into<String>, service_name: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            service_name: service_name.into(),
        }
    }

    /// OTLP endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

#[async_trait]
impl Exporter for OtlpExporter {
    fn name(&self) -> &'static str {
        "otlp"
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Otlp
    }

    fn is_implemented(&self) -> bool {
        false
    }

    async fn export(&self, _registry: &MetricsRegistry) -> MetricsResult<String> {
        Err(MetricsError::ExporterNotImplemented(
            ExporterKind::Otlp.as_str().to_string(),
        ))
    }
}

// ============================================================================
// §5 StatsdExporter — stub
// ============================================================================

/// StatsD (UDP) exporter (stub).
///
/// R20 阶段 6 估缺: 实际 UDP packet 发送留 R21 续接.
pub struct StatsdExporter {
    /// StatsD server host (e.g. "127.0.0.1").
    host: String,
    /// StatsD server port (e.g. 8125).
    port: u16,
}

impl StatsdExporter {
    /// 构造.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Host.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[async_trait]
impl Exporter for StatsdExporter {
    fn name(&self) -> &'static str {
        "statsd"
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Statsd
    }

    fn is_implemented(&self) -> bool {
        false
    }

    async fn export(&self, _registry: &MetricsRegistry) -> MetricsResult<String> {
        Err(MetricsError::ExporterNotImplemented(
            ExporterKind::Statsd.as_str().to_string(),
        ))
    }
}

// ============================================================================
// §6 StdoutExporter — 完整实现
// ============================================================================

/// Stdout exporter — 直接 println! 到标准输出.
///
/// 用于本地开发 / debug, 不依赖网络.
pub struct StdoutExporter;

impl StdoutExporter {
    /// 构造.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdoutExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Exporter for StdoutExporter {
    fn name(&self) -> &'static str {
        "stdout"
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Stdout
    }

    fn is_implemented(&self) -> bool {
        true
    }

    async fn export(&self, registry: &MetricsRegistry) -> MetricsResult<String> {
        // 复用 PrometheusExporter 的逻辑 (text format), 区别只是出口 (println vs HTTP)
        let p = PrometheusExporter::new("apeireth", "agent");
        let body = p.export(registry).await?;
        // 写到 stdout
        println!("{body}");
        Ok(body)
    }
}

// ============================================================================
// §7 Factory
// ============================================================================

/// 按 ExporterKind 构造对应 Exporter.
pub fn build_exporter(
    kind: ExporterKind,
    namespace: &str,
    subsystem: &str,
) -> MetricsResult<Arc<dyn Exporter>> {
    kind.check_implemented()?;
    let exporter: Arc<dyn Exporter> = match kind {
        ExporterKind::Prometheus => Arc::new(PrometheusExporter::new(namespace, subsystem)),
        ExporterKind::Stdout => Arc::new(StdoutExporter::new()),
        ExporterKind::Pushgateway => {
            return Err(MetricsError::ExporterNotImplemented(
                ExporterKind::Pushgateway.as_str().to_string(),
            ));
        }
        ExporterKind::Otlp => {
            return Err(MetricsError::ExporterNotImplemented(
                ExporterKind::Otlp.as_str().to_string(),
            ));
        }
        ExporterKind::Statsd => {
            return Err(MetricsError::ExporterNotImplemented(
                ExporterKind::Statsd.as_str().to_string(),
            ));
        }
    };
    Ok(exporter)
}

// ============================================================================
// §8 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use super::super::counter::Counter;
    use super::super::gauge::Gauge;
    use super::super::histogram::Histogram;
    use super::super::summary::Summary;

    fn sample_registry() -> MetricsRegistry {
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
        r
    }

    /// 守门 #1: PrometheusExporter 完整实现.
    #[tokio::test]
    async fn prometheus_exporter_full() {
        let r = sample_registry();
        let e = PrometheusExporter::new("apeireth", "agent");
        assert!(e.is_implemented());
        let body = e.export(&r).await.unwrap();
        assert!(body.contains("# TYPE apeireth_agent_requests_total counter"));
        assert!(body.contains("# TYPE apeireth_agent_memory_bytes gauge"));
        assert!(body.contains("# TYPE apeireth_agent_latency histogram"));
        assert!(body.contains("# TYPE apeireth_agent_rpc_duration summary"));
    }

    /// 守门 #2: PushgatewayExporter stub.
    #[tokio::test]
    async fn pushgateway_exporter_not_implemented() {
        let r = sample_registry();
        let e = PushgatewayExporter::new("http://localhost:9091", "myjob");
        assert!(!e.is_implemented());
        assert!(matches!(
            e.export(&r).await,
            Err(MetricsError::ExporterNotImplemented(_))
        ));
    }

    /// 守门 #3: OtlpExporter stub.
    #[tokio::test]
    async fn otlp_exporter_not_implemented() {
        let r = sample_registry();
        let e = OtlpExporter::new("http://localhost:4317", "apeireth");
        assert!(!e.is_implemented());
        assert!(matches!(
            e.export(&r).await,
            Err(MetricsError::ExporterNotImplemented(_))
        ));
    }

    /// 守门 #4: StatsdExporter stub.
    #[tokio::test]
    async fn statsd_exporter_not_implemented() {
        let r = sample_registry();
        let e = StatsdExporter::new("127.0.0.1", 8125);
        assert!(!e.is_implemented());
        assert!(matches!(
            e.export(&r).await,
            Err(MetricsError::ExporterNotImplemented(_))
        ));
    }

    /// 守门 #5: StdoutExporter 完整 (不 panic).
    #[tokio::test]
    async fn stdout_exporter_write() {
        let r = sample_registry();
        let e = StdoutExporter::new();
        assert!(e.is_implemented());
        let body = e.export(&r).await.unwrap();
        assert!(body.contains("requests_total"));
        assert!(body.contains("memory_bytes"));
    }

    /// 守门 #6: build_exporter 完整.
    #[test]
    fn build_exporter_prometheus() {
        let e = build_exporter(ExporterKind::Prometheus, "apeireth", "agent").unwrap();
        assert_eq!(e.name(), "prometheus");
    }

    /// 守门 #7: build_exporter stub 拒.
    #[test]
    fn build_exporter_stub_rejected() {
        assert!(build_exporter(ExporterKind::Pushgateway, "a", "b").is_err());
        assert!(build_exporter(ExporterKind::Otlp, "a", "b").is_err());
        assert!(build_exporter(ExporterKind::Statsd, "a", "b").is_err());
    }

    /// 守门 #8: 5 Exporter kind 都正确返.
    #[test]
    fn all_5_kinds_kind() {
        assert_eq!(
            PrometheusExporter::new("a", "b").kind(),
            ExporterKind::Prometheus
        );
        assert_eq!(
            PushgatewayExporter::new("u", "j").kind(),
            ExporterKind::Pushgateway
        );
        assert_eq!(OtlpExporter::new("e", "s").kind(), ExporterKind::Otlp);
        assert_eq!(StatsdExporter::new("h", 1234).kind(), ExporterKind::Statsd);
        assert_eq!(StdoutExporter::new().kind(), ExporterKind::Stdout);
    }

    /// 守门 #9: 5 Exporter name 唯一.
    #[test]
    fn all_5_kinds_name() {
        assert_eq!(PrometheusExporter::new("a", "b").name(), "prometheus");
        assert_eq!(PushgatewayExporter::new("u", "j").name(), "pushgateway");
        assert_eq!(OtlpExporter::new("e", "s").name(), "otlp");
        assert_eq!(StatsdExporter::new("h", 1234).name(), "statsd");
        assert_eq!(StdoutExporter::new().name(), "stdout");
    }
}
