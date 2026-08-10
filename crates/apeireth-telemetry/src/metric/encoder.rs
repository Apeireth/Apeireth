//! # Encoder — Prometheus text exposition format 编码
//!
//! Prometheus text exposition format spec:
//! <https://github.com/prometheus/docs/blob/main/content/docs/instrumenting/exposition_formats.md>
//!
//! ## 格式示例
//!
//! ```text
//! # HELP apeireth_agent_requests_total Total HTTP requests
//! # TYPE apeireth_agent_requests_total counter
//! apeireth_agent_requests_total{method="GET",status="200"} 1027
//!
//! # HELP apeireth_agent_memory_bytes Memory used in bytes
//! # TYPE apeireth_agent_memory_bytes gauge
//! apeireth_agent_memory_bytes 1.234e9
//! ```
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `encode_metrics` | `encodeMetrics`                | ✅  |
//! | `format_help`    | `formatHelp`                   | ✅  |
//! | `format_type`    | `formatType`                   | ✅  |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::fmt::Write;
use std::sync::Arc;

use super::counter::Counter;
use super::gauge::Gauge;
use super::histogram::Histogram;
use super::label::labels_to_prometheus_sorted;
use super::summary::Summary;
use super::error::{MetricsError, MetricsResult};
use super::{Metric, MetricValue};

// ============================================================================
// §1 编码核心 API
// ============================================================================

/// 编码单个 metric 为 Prometheus exposition format 字符串 (含 HELP / TYPE).
///
/// ## K-1 守门
///
/// - help 必填 (空字符串返 HelpRequired)
/// - name 必填 (空字符串返 MetricNameEmpty)
pub fn encode_metric(name: &str, help: &str, value: &MetricValue) -> MetricsResult<String> {
    if name.is_empty() {
        return Err(MetricsError::MetricNameEmpty);
    }
    if help.is_empty() {
        return Err(MetricsError::HelpRequired(name.to_string()));
    }
    let mut out = String::with_capacity(256);
    // HELP 行
    writeln!(out, "# HELP {} {}", name, escape_help(help))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    // TYPE 行
    let type_name = value.type_name();
    writeln!(out, "# TYPE {} {}", name, type_name)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    // 数据行
    match value {
        MetricValue::Counter(v) => {
            writeln!(out, "{} {}", name, v)
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
        MetricValue::Gauge(v) => {
            writeln!(out, "{} {}", name, format_float(*v))
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
        MetricValue::Histogram {
            buckets,
            sum,
            count,
        } => {
            // 累加 bucket: bucket(le=X) 表示 ≤ X 的累计次数
            let mut cumulative: u64 = 0;
            for (le, count) in buckets {
                cumulative = cumulative.saturating_add(*count);
                let le_str = if le.is_infinite() {
                    "+Inf".to_string()
                } else {
                    format_float(*le)
                };
                writeln!(out, "{}_bucket{{le=\"{}\"}} {}", name, le_str, cumulative)
                    .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
            }
            // +Inf bucket (总样本)
            writeln!(out, "{}_bucket{{le=\"+Inf\"}} {}", name, count)
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
            // sum + count
            writeln!(out, "{}_sum {}", name, format_float(*sum))
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
            writeln!(out, "{}_count {}", name, count)
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
        MetricValue::Summary {
            quantiles,
            sum,
            count,
        } => {
            for (q, v) in quantiles {
                writeln!(out, "{{{}=\"{}\"}} {}", name, q, format_float(*v))
                    .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
            }
            writeln!(out, "{}_sum {}", name, format_float(*sum))
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
            writeln!(out, "{}_count {}", name, count)
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
    }
    Ok(out)
}

/// 编码带 label 的 metric.
pub fn encode_metric_with_labels(
    name: &str,
    help: &str,
    labels: &std::collections::HashMap<String, String>,
    value: &MetricValue,
) -> MetricsResult<String> {
    if name.is_empty() {
        return Err(MetricsError::MetricNameEmpty);
    }
    if help.is_empty() {
        return Err(MetricsError::HelpRequired(name.to_string()));
    }
    let mut out = String::with_capacity(256);
    writeln!(out, "# HELP {} {}", name, escape_help(help))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    let type_name = value.type_name();
    writeln!(out, "# TYPE {} {}", name, type_name)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    let label_str = if labels.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", labels_to_prometheus_sorted(labels))
    };
    match value {
        MetricValue::Counter(v) => {
            writeln!(out, "{}{} {}", name, label_str, v)
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
        MetricValue::Gauge(v) => {
            writeln!(out, "{}{} {}", name, label_str, format_float(*v))
                .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
        }
        MetricValue::Histogram { .. } | MetricValue::Summary { .. } => {
            // 带 labels 的 histogram / summary 走更细的 encode_histogram / encode_summary
            return Err(MetricsError::EncodeError(
                "histogram/summary with labels: use encode_histogram / encode_summary"
                    .to_string(),
            ));
        }
    }
    Ok(out)
}

/// 编码 histogram 完整 (带 label).
pub fn encode_histogram(
    name: &str,
    help: &str,
    labels: &std::collections::HashMap<String, String>,
    buckets: &[(f64, u64)],
    sum: f64,
    count: u64,
) -> MetricsResult<String> {
    if name.is_empty() {
        return Err(MetricsError::MetricNameEmpty);
    }
    if help.is_empty() {
        return Err(MetricsError::HelpRequired(name.to_string()));
    }
    let mut out = String::with_capacity(512);
    writeln!(out, "# HELP {} {}", name, escape_help(help))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    writeln!(out, "# TYPE {} histogram", name)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;

    let base_labels = labels.clone();
    let mut cumulative: u64 = 0;
    for (le, c) in buckets {
        cumulative = cumulative.saturating_add(*c);
        let le_str = if le.is_infinite() {
            "+Inf".to_string()
        } else {
            format_float(*le)
        };
        let mut bucket_labels = base_labels.clone();
        bucket_labels.insert("le".to_string(), le_str);
        writeln!(
            out,
            "{}_bucket{{{}}} {}",
            name,
            labels_to_prometheus_sorted(&bucket_labels),
            cumulative
        )
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    }
    // +Inf bucket
    let mut inf_labels = base_labels.clone();
    inf_labels.insert("le".to_string(), "+Inf".to_string());
    writeln!(
        out,
        "{}_bucket{{{}}} {}",
        name,
        labels_to_prometheus_sorted(&inf_labels),
        count
    )
    .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    // sum + count
    let label_str = if base_labels.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", labels_to_prometheus_sorted(&base_labels))
    };
    writeln!(out, "{}_sum{} {}", name, label_str, format_float(sum))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    writeln!(out, "{}_count{} {}", name, label_str, count)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    Ok(out)
}

/// 编码 summary 完整 (带 label).
pub fn encode_summary(
    name: &str,
    help: &str,
    labels: &std::collections::HashMap<String, String>,
    quantiles: &[(f64, f64)],
    sum: f64,
    count: u64,
) -> MetricsResult<String> {
    if name.is_empty() {
        return Err(MetricsError::MetricNameEmpty);
    }
    if help.is_empty() {
        return Err(MetricsError::HelpRequired(name.to_string()));
    }
    let mut out = String::with_capacity(512);
    writeln!(out, "# HELP {} {}", name, escape_help(help))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    writeln!(out, "# TYPE {} summary", name)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;

    for (q, v) in quantiles {
        let mut q_labels = labels.clone();
        q_labels.insert("quantile".to_string(), format_float(*q));
        writeln!(
            out,
            "{}{{{}}} {}",
            name,
            labels_to_prometheus_sorted(&q_labels),
            format_float(*v)
        )
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    }
    let label_str = if labels.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", labels_to_prometheus_sorted(labels))
    };
    writeln!(out, "{}_sum{} {}", name, label_str, format_float(sum))
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    writeln!(out, "{}_count{} {}", name, label_str, count)
        .map_err(|e| MetricsError::EncodeError(e.to_string()))?;
    Ok(out)
}

// ============================================================================
// §2 辅助函数
// ============================================================================

/// 转义 help 文本中的 `\n` 和 `\\` (per Prometheus exposition format).
fn escape_help(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

/// 格式化浮点数 (NaN → "NaN", Inf → "+Inf"/"-Inf", 否则 %g 或科学计数法).
fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "+Inf".to_string() } else { "-Inf".to_string() };
    }
    if f == 0.0 {
        return "0".to_string();
    }
    // 用 %g, 6 位有效数字, 跟 Prometheus convention 一致
    format!("{:.6}", f)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

// ============================================================================
// §3 便捷 encode Counter / Gauge / Histogram / Summary (借用 Arc 共享)
// ============================================================================

/// 编码 Arc<Counter> 为完整 exposition 行 (含 HELP / TYPE).
///
/// 注意: 如果 counter 自带 label, 会走 `encode_metric_with_labels`.
pub fn encode_counter(name: &str, help: &str, counter: &Arc<Counter>) -> MetricsResult<String> {
    let v = counter.value();
    let labels = counter.labels();
    if labels.is_empty() {
        encode_metric(name, help, &v)
    } else {
        encode_metric_with_labels(name, help, labels, &v)
    }
}

/// 编码 Arc<Gauge> 为完整 exposition 行.
///
/// 注意: 如果 gauge 自带 label, 会走 `encode_metric_with_labels`.
pub fn encode_gauge(name: &str, help: &str, gauge: &Arc<Gauge>) -> MetricsResult<String> {
    let v = gauge.value();
    let labels = gauge.labels();
    if labels.is_empty() {
        encode_metric(name, help, &v)
    } else {
        encode_metric_with_labels(name, help, labels, &v)
    }
}

/// 编码 Arc<Histogram> 为完整 exposition 行 (含 buckets / sum / count).
pub fn encode_histogram_full(
    name: &str,
    help: &str,
    hist: &Arc<Histogram>,
) -> MetricsResult<String> {
    let v = hist.value();
    if let MetricValue::Histogram {
        buckets,
        sum,
        count,
    } = v
    {
        let labels = hist.labels().clone();
        encode_histogram(name, help, &labels, &buckets, sum, count)
    } else {
        Err(MetricsError::EncodeError(
            "histogram value is not Histogram variant".to_string(),
        ))
    }
}

/// 编码 Arc<Summary> 为完整 exposition 行 (含 quantiles / sum / count).
pub fn encode_summary_full(
    name: &str,
    help: &str,
    summary: &Arc<Summary>,
) -> MetricsResult<String> {
    let v = summary.value();
    if let MetricValue::Summary {
        quantiles,
        sum,
        count,
    } = v
    {
        let labels = summary.labels().clone();
        encode_summary(name, help, &labels, &quantiles, sum, count)
    } else {
        Err(MetricsError::EncodeError(
            "summary value is not Summary variant".to_string(),
        ))
    }
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: encode Counter 包含 HELP / TYPE / value.
    #[test]
    fn encode_counter_basic() {
        let v = MetricValue::Counter(42);
        let s = encode_metric("requests_total", "Total requests", &v).unwrap();
        assert!(s.contains("# HELP requests_total Total requests"));
        assert!(s.contains("# TYPE requests_total counter"));
        assert!(s.contains("requests_total 42"));
    }

    /// 守门 #2: encode Gauge 包含 HELP / TYPE / value.
    #[test]
    fn encode_gauge_basic() {
        let v = MetricValue::Gauge(1.5);
        let s = encode_metric("memory_bytes", "Memory used", &v).unwrap();
        assert!(s.contains("# TYPE memory_bytes gauge"));
        assert!(s.contains("memory_bytes 1.5"));
    }

    /// 守门 #3: K-1 empty name 拒.
    #[test]
    fn k1_empty_name_rejected() {
        let v = MetricValue::Counter(0);
        assert!(matches!(
            encode_metric("", "help", &v),
            Err(MetricsError::MetricNameEmpty)
        ));
    }

    /// 守门 #4: K-1 empty help 拒.
    #[test]
    fn k1_empty_help_rejected() {
        let v = MetricValue::Counter(0);
        assert!(matches!(
            encode_metric("name", "", &v),
            Err(MetricsError::HelpRequired(_))
        ));
    }

    /// 守门 #5: format_float NaN.
    #[test]
    fn format_float_nan() {
        assert_eq!(format_float(f64::NAN), "NaN");
    }

    /// 守门 #6: format_float Inf.
    #[test]
    fn format_float_inf() {
        assert_eq!(format_float(f64::INFINITY), "+Inf");
        assert_eq!(format_float(f64::NEG_INFINITY), "-Inf");
    }

    /// 守门 #7: format_float 0.
    #[test]
    fn format_float_zero() {
        assert_eq!(format_float(0.0), "0");
    }

    /// 守门 #8: format_float 普通值.
    #[test]
    fn format_float_normal() {
        assert_eq!(format_float(1.5), "1.5");
        assert_eq!(format_float(2.0), "2");
    }

    /// 守门 #9: escape_help 转义.
    #[test]
    fn escape_help_works() {
        assert_eq!(escape_help("normal"), "normal");
        assert_eq!(escape_help("a\\b"), "a\\\\b");
        assert_eq!(escape_help("a\nb"), "a\\nb");
    }

    /// 守门 #10: encode histogram 含 buckets.
    #[test]
    fn encode_histogram_buckets() {
        let buckets = vec![(0.1, 1), (0.5, 2), (1.0, 1)];
        let s = encode_histogram(
            "latency",
            "Request latency",
            &std::collections::HashMap::new(),
            &buckets,
            0.75,
            4,
        )
        .unwrap();
        assert!(s.contains("# TYPE latency histogram"));
        assert!(s.contains("latency_bucket{le=\"0.1\"} 1"));
        assert!(s.contains("latency_bucket{le=\"+Inf\"} 4"));
        assert!(s.contains("latency_sum"));
        assert!(s.contains("latency_count 4"));
    }

    /// 守门 #11: encode summary 含 quantiles.
    #[test]
    fn encode_summary_quantiles() {
        let quantiles = vec![(0.5, 0.1), (0.9, 0.5), (0.99, 1.0)];
        let s = encode_summary(
            "rpc_duration",
            "RPC duration",
            &std::collections::HashMap::new(),
            &quantiles,
            100.0,
            50,
        )
        .unwrap();
        assert!(s.contains("# TYPE rpc_duration summary"));
        assert!(s.contains("rpc_duration{quantile=\"0.5\"}"));
        assert!(s.contains("rpc_duration_count 50"));
    }
}
