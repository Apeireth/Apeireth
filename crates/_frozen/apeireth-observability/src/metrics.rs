//! # Metrics 集成 (Prometheus exposition format)
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main` observability 集成 (per blueprint §2.5.3).
//! 商业版用 `@opentelemetry/exporter-prometheus` 导出, 我们 skeleton 阶段自写
//! Prometheus text format 渲染 (Counter/Gauge/Histogram 3 类, 不引 OpenTelemetry SDK).
//!
//! ## Prometheus 文本格式 (line protocol)
//!
//! ```text
//! # HELP <metric_name> <help text>
//! # TYPE <metric_name> <counter|gauge|histogram>
//! <metric_name>{label1="v1",label2="v2"} <value> <timestamp_ms>
//! ```
//!
//! Skeleton 阶段只渲染 counter / gauge (单值), histogram bucket 留 R20 阶段 3 续.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{MetricKind, MetricSample, ObservabilityError, ObservabilityResult, PLATFORM_NAME};

/// Metrics 注册表 (in-memory, per-key 累加).
///
/// 1:1 翻译 Prometheus `CollectorRegistry`, skeleton 阶段用 `HashMap<name, MetricSample>`.
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    /// Metric 名 → 累计样本 (相同 name + labels 累加 value, per Prometheus 数据模型)
    samples: Arc<RwLock<HashMap<String, MetricSample>>>,
}

impl MetricsRegistry {
    /// 新建空注册表.
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 记录 Counter (单调递增, 累加).
    pub async fn counter(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        self.record(MetricSample {
            name: name.to_string(),
            kind: MetricKind::Counter,
            value,
            labels,
        })
        .await;
    }

    /// 记录 Gauge (任意值, 覆盖).
    pub async fn gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        self.record(MetricSample {
            name: name.to_string(),
            kind: MetricKind::Gauge,
            value,
            labels,
        })
        .await;
    }

    /// 记录 Histogram (skeleton 阶段只存最后一次值, R20 阶段 3 续 bucket 累加).
    pub async fn histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        self.record(MetricSample {
            name: name.to_string(),
            kind: MetricKind::Histogram,
            value,
            labels,
        })
        .await;
    }

    /// 内部: 累加 (Counter) / 覆盖 (Gauge/Histogram) 样本.
    async fn record(&self, sample: MetricSample) {
        let key = format!(
            "{}{}",
            sample.name,
            if sample.labels.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", render_labels(&sample.labels))
            }
        );

        let mut samples = self.samples.write().await;
        // 第一次: or_insert_with 创建占位 (value=0.0, kind/labels 正确).
        // Counter 累加, 所以 entry.value 必为 0, 然后 += sample.value.
        let entry = samples.entry(key.clone()).or_insert_with(|| MetricSample {
            name: sample.name.clone(),
            kind: sample.kind,
            value: 0.0,
            labels: sample.labels.clone(),
        });

        // Counter 累加, 其他覆盖
        match sample.kind {
            MetricKind::Counter => {
                entry.value += sample.value;
            }
            MetricKind::Gauge | MetricKind::Histogram => {
                entry.value = sample.value;
            }
        }

        info!(
            name = %sample.name,
            kind = %sample.kind,
            value = sample.value,
            "metrics: sample recorded"
        );
    }

    /// 列出所有样本 (snapshot).
    pub async fn snapshot(&self) -> Vec<MetricSample> {
        self.samples.read().await.values().cloned().collect()
    }

    /// 清空 (test 用).
    pub async fn clear(&self) {
        self.samples.write().await.clear();
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 渲染 labels (Prometheus exposition format: `k1="v1",k2="v2"`).
fn render_labels(labels: &HashMap<String, String>) -> String {
    let mut out = String::new();
    for (i, (k, v)) in labels.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        // 转义: `\` → `\\`, `"` → `\"`, 换行 → `\n`
        let escaped_v = v
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        write!(&mut out, "{k}=\"{escaped_v}\"").unwrap_or_else(|_| {
            warn!("metrics: failed to write label");
        });
    }
    out
}

/// 渲染 Prometheus 文本格式 (整个 registry → 1 字符串, 给 `/metrics` 端点返).
///
/// R20 阶段 3 续 OpenTelemetry SDK 时, 这函数保持兼容 (Prometheus OTLP receiver 直接消费这格式).
pub fn render_prometheus(samples: &[MetricSample]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# platform: {PLATFORM_NAME}\n"));
    out.push_str("# schema_version: 1\n");

    // 按 metric name 分组, 每个 name 渲染一次 HELP + TYPE
    let mut by_name: HashMap<&str, &MetricSample> = HashMap::new();
    for s in samples {
        by_name.entry(s.name.as_str()).or_insert(s);
    }

    for (name, sample) in &by_name {
        out.push_str(&format!("# HELP {name} {name} (apeireth observability)\n"));
        out.push_str(&format!("# TYPE {name} {}\n", sample.kind.as_str()));
    }

    // 逐 sample 渲染
    for s in samples {
        if s.labels.is_empty() {
            out.push_str(&format!("{} {}\n", s.name, s.value));
        } else {
            out.push_str(&format!(
                "{}{{{}}} {}\n",
                s.name,
                render_labels(&s.labels),
                s.value
            ));
        }
    }
    out
}

/// 解析 Prometheus 文本格式 (反序列化, 估 fixture 用).
pub fn parse_prometheus(input: &str) -> ObservabilityResult<Vec<MetricSample>> {
    let mut samples = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // 格式: `<name>{<labels>} <value>` 或 `<name> <value>`
        let (name_and_labels, value_str) = line
            .rsplit_once(' ')
            .ok_or_else(|| ObservabilityError::PrometheusFormat(format!("invalid line: {line}")))?;
        let value: f64 = value_str.parse().map_err(|e| {
            ObservabilityError::PrometheusFormat(format!("value parse: {e}"))
        })?;

        let (name, labels) = if let Some(idx) = name_and_labels.find('{') {
            let name = &name_and_labels[..idx];
            let labels_str = &name_and_labels[idx + 1..name_and_labels.len() - 1];
            let labels = parse_labels(labels_str);
            (name.to_string(), labels)
        } else {
            (name_and_labels.to_string(), HashMap::new())
        };

        samples.push(MetricSample {
            name,
            kind: MetricKind::Gauge, // 解析阶段类型不明, 默认 gauge
            value,
            labels,
        });
    }
    Ok(samples)
}

/// 解析 labels 字符串 (`k1="v1",k2="v2"`).
fn parse_labels(s: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    // 极简 split: 按 `,` 分, 然后 `=` 拆
    for part in s.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            let v = v.trim_matches('"');
            out.insert(k.trim().to_string(), v.to_string());
        }
    }
    out
}

// ============================================================================
// 单元测试 (in-module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn counter_accumulates() {
        let r = MetricsRegistry::new();
        let mut labels = HashMap::new();
        labels.insert("endpoint".to_string(), "/api".to_string());
        r.counter("requests_total", 1.0, labels.clone()).await;
        r.counter("requests_total", 2.0, labels.clone()).await;
        let snap = r.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].value, 3.0);
    }

    #[tokio::test]
    async fn gauge_overwrites() {
        let r = MetricsRegistry::new();
        r.gauge("active_connections", 5.0, HashMap::new()).await;
        r.gauge("active_connections", 10.0, HashMap::new()).await;
        let snap = r.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].value, 10.0);
    }

    #[test]
    fn render_prometheus_format_basic() {
        let samples = vec![
            MetricSample::new("requests_total", MetricKind::Counter, 42.0)
                .with_label("endpoint", "/api/v1"),
            MetricSample::new("active_connections", MetricKind::Gauge, 7.0),
        ];
        let text = render_prometheus(&samples);
        assert!(text.contains("# TYPE requests_total counter"));
        assert!(text.contains("# TYPE active_connections gauge"));
        assert!(text.contains(r#"requests_total{endpoint="/api/v1"} 42"#));
        assert!(text.contains("active_connections 7"));
    }

    #[test]
    fn parse_prometheus_roundtrip() {
        let samples = vec![
            MetricSample::new("requests_total", MetricKind::Counter, 42.0)
                .with_label("endpoint", "/api"),
        ];
        let text = render_prometheus(&samples);
        let parsed = parse_prometheus(&text).expect("parse ok");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "requests_total");
        assert_eq!(parsed[0].value, 42.0);
        assert_eq!(parsed[0].labels.get("endpoint").map(|s| s.as_str()), Some("/api"));
    }

    #[test]
    fn label_escape_quotes() {
        let mut labels = HashMap::new();
        labels.insert("path".to_string(), r#"C:\Users"test""#.to_string());
        let text = render_labels(&labels);
        assert!(text.contains(r#"path="C:\\Users\"test\"""#), "escaped: {text}");
    }
}
