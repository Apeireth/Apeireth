//! # MetricsConfig — metrics 配置 (4 段)
//!
//! 4 段配置 (per task spec §5):
//! 1. `namespace: String` — 命名空间前缀 (e.g. `apeireth`)
//! 2. `subsystem: String` — 子系统前缀 (e.g. `agent`)
//! 3. `global_labels: HashMap<String, String>` — 全局 label, 注册时自动追加
//! 4. `exporter: ExporterKind` — 5 exporter 之一
//!
//! ## 1:1 翻译映射 (v0.9.21 @anthropic-ai/metrics)
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版   | 1:1 |
//! |------------------|--------------------------------|-----|
//! | `MetricsConfig`  | `MetricsConfig`                | ✅  |
//! | `ExporterKind`   | `ExporterKind`                 | ✅  |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::label::{validate_label_key, validate_labels, LABEL_MAX_COUNT};

// ============================================================================
// §1 ExporterKind (5 段, 编译期 hardcode)
// ============================================================================

/// 5 exporter 之一 (per task spec §4).
///
/// - `Prometheus` — text exposition format, `/metrics` HTTP endpoint, 完整
/// - `Pushgateway` — push, R20 阶段 6 stub
/// - `Otlp` — OpenTelemetry Protocol, R20 阶段 6 stub
/// - `Statsd` — UDP, R20 阶段 6 stub
/// - `Stdout` — print, 完整
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExporterKind {
    /// Prometheus text exposition format, 完整.
    Prometheus,
    /// Prometheus Pushgateway, R20 阶段 6 stub.
    Pushgateway,
    /// OpenTelemetry Protocol, R20 阶段 6 stub.
    Otlp,
    /// StatsD (UDP), R20 阶段 6 stub.
    Statsd,
    /// Stdout print, 完整.
    Stdout,
}

impl ExporterKind {
    /// 全部 5 个 variant (K-1 守门).
    pub const ALL: [ExporterKind; 5] = [
        ExporterKind::Prometheus,
        ExporterKind::Pushgateway,
        ExporterKind::Otlp,
        ExporterKind::Statsd,
        ExporterKind::Stdout,
    ];

    /// 字符串表示 (跟 config JSON 兼容).
    pub fn as_str(&self) -> &'static str {
        match self {
            ExporterKind::Prometheus => "PROMETHEUS",
            ExporterKind::Pushgateway => "PUSHGATEWAY",
            ExporterKind::Otlp => "OTLP",
            ExporterKind::Statsd => "STATSD",
            ExporterKind::Stdout => "STDOUT",
        }
    }

    /// 是否已完整实现 (用于 R20 阶段 6 skeleton 守门).
    pub fn is_implemented(&self) -> bool {
        matches!(self, ExporterKind::Prometheus | ExporterKind::Stdout)
    }

    /// K-1 强校验: 未实现的 exporter 返 ExporterNotImplemented.
    pub fn check_implemented(&self) -> super::error::MetricsResult<()> {
        if self.is_implemented() {
            Ok(())
        } else {
            Err(super::error::MetricsError::ExporterNotImplemented(
                self.as_str().to_string(),
            ))
        }
    }
}

impl std::fmt::Display for ExporterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExporterKind {
    type Err = super::error::MetricsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "PROMETHEUS" => Ok(ExporterKind::Prometheus),
            "PUSHGATEWAY" => Ok(ExporterKind::Pushgateway),
            "OTLP" => Ok(ExporterKind::Otlp),
            "STATSD" => Ok(ExporterKind::Statsd),
            "STDOUT" => Ok(ExporterKind::Stdout),
            other => Err(super::error::MetricsError::ExporterNotImplemented(
                other.to_string(),
            )),
        }
    }
}

// ============================================================================
// §2 MetricsConfig 结构 (4 段)
// ============================================================================

/// Metrics 配置 (4 段).
///
/// - `namespace` + `subsystem` 拼成 metric 完整名前缀: `{namespace}_{subsystem}_{name}`
///   (e.g. `apeireth_agent_requests_total`)
/// - `global_labels` 在每次 register 时自动追加 (用于环境/版本/region 等切片)
/// - `exporter` 默认 Prometheus
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// 命名空间前缀 (e.g. "apeireth"), 编译期 hardcode 默认值.
    pub namespace: String,

    /// 子系统前缀 (e.g. "agent"), 编译期 hardcode 默认值.
    pub subsystem: String,

    /// 全局 label, 注册时自动追加 (K-1 强校验: key 字符 + value ≤ 256 + ≤ 10 个).
    pub global_labels: HashMap<String, String>,

    /// 5 exporter 之一.
    pub exporter: ExporterKind,
}

impl MetricsConfig {
    /// 默认配置 (per task spec §5).
    pub fn default_config() -> Self {
        Self {
            namespace: super::DEFAULT_NAMESPACE.to_string(),
            subsystem: super::DEFAULT_SUBSYSTEM.to_string(),
            global_labels: HashMap::new(),
            exporter: super::DEFAULT_EXPORTER,
        }
    }

    /// 拼出 metric 完整名: `{namespace}_{subsystem}_{name}`.
    pub fn full_name(&self, name: &str) -> String {
        format!("{}_{}_{}", self.namespace, self.subsystem, name)
    }

    /// 校验 (K-1 强校验: namespace/subsystem 非空 + global_labels).
    pub fn validate(&self) -> super::error::MetricsResult<()> {
        if self.namespace.is_empty() {
            return Err(super::error::MetricsError::MetricNameInvalid(
                "namespace".to_string(),
            ));
        }
        if self.subsystem.is_empty() {
            return Err(super::error::MetricsError::MetricNameInvalid(
                "subsystem".to_string(),
            ));
        }
        // namespace + subsystem 作为 metric name 组成部分, 也需符合 `[a-zA-Z_][a-zA-Z0-9_]*`
        validate_label_key(&self.namespace)?;
        validate_label_key(&self.subsystem)?;
        // global_labels 数量校验
        if self.global_labels.len() > LABEL_MAX_COUNT {
            return Err(super::error::MetricsError::TooManyLabels {
                actual: self.global_labels.len(),
            });
        }
        validate_labels(&self.global_labels)?;
        Ok(())
    }
}



impl Default for MetricsConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: MetricsConfig 4 段.
    #[test]
    fn metrics_config_4_fields() {
        let c = MetricsConfig {
            namespace: "apeireth".to_string(),
            subsystem: "agent".to_string(),
            global_labels: HashMap::new(),
            exporter: ExporterKind::Prometheus,
        };
        assert_eq!(c.namespace, "apeireth");
        assert_eq!(c.subsystem, "agent");
        assert!(c.global_labels.is_empty());
        assert_eq!(c.exporter, ExporterKind::Prometheus);
    }

    /// 守门 #2: 默认配置合法.
    #[test]
    fn metrics_config_default_validates() {
        let c = MetricsConfig::default_config();
        assert!(c.validate().is_ok());
    }

    /// 守门 #3: K-1 empty namespace 拒.
    #[test]
    fn k1_empty_namespace_rejected() {
        let mut c = MetricsConfig::default_config();
        c.namespace = String::new();
        assert!(c.validate().is_err());
    }

    /// 守门 #4: K-1 empty subsystem 拒.
    #[test]
    fn k1_empty_subsystem_rejected() {
        let mut c = MetricsConfig::default_config();
        c.subsystem = String::new();
        assert!(c.validate().is_err());
    }

    /// 守门 #5: K-1 invalid namespace 字符拒.
    #[test]
    fn k1_namespace_invalid_chars_rejected() {
        let mut c = MetricsConfig::default_config();
        c.namespace = "123-bad".to_string();
        assert!(c.validate().is_err());
    }

    /// 守门 #6: K-1 global_label 数 > 10 拒.
    #[test]
    fn k1_too_many_global_labels_rejected() {
        let mut c = MetricsConfig::default_config();
        for i in 0..11 {
            c.global_labels.insert(format!("k{i}"), format!("v{i}"));
        }
        assert!(c.validate().is_err());
    }

    /// 守门 #7: full_name 拼接.
    #[test]
    fn full_name_concat() {
        let c = MetricsConfig {
            namespace: "apeireth".to_string(),
            subsystem: "agent".to_string(),
            global_labels: HashMap::new(),
            exporter: ExporterKind::Prometheus,
        };
        assert_eq!(c.full_name("requests_total"), "apeireth_agent_requests_total");
    }

    /// 守门 #8: 5 ExporterKind 都能作为 config.exporter.
    #[test]
    fn all_5_exporters() {
        for k in ExporterKind::ALL {
            let c = MetricsConfig {
                namespace: "apeireth".to_string(),
                subsystem: "agent".to_string(),
                global_labels: HashMap::new(),
                exporter: k,
            };
            assert!(c.validate().is_ok(), "exporter {k} validate 失败");
        }
    }

    /// 守门 #9: 2 完整, 3 stub.
    #[test]
    fn is_implemented_2_complete_3_stub() {
        assert!(ExporterKind::Prometheus.is_implemented());
        assert!(ExporterKind::Stdout.is_implemented());
        assert!(!ExporterKind::Pushgateway.is_implemented());
        assert!(!ExporterKind::Otlp.is_implemented());
        assert!(!ExporterKind::Statsd.is_implemented());
    }

    /// 守门 #10: Serialize + Deserialize roundtrip.
    #[test]
    fn metrics_config_serde_roundtrip() {
        let mut c = MetricsConfig::default_config();
        c.global_labels.insert("env".to_string(), "test".to_string());
        c.exporter = ExporterKind::Stdout;
        let s = serde_json::to_string(&c).unwrap();
        let parsed: MetricsConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(c, parsed);
    }

    /// 守门 #11: ExporterKind FromStr.
    #[test]
    fn exporter_kind_from_str() {
        assert_eq!("PROMETHEUS".parse::<ExporterKind>().unwrap(), ExporterKind::Prometheus);
        assert_eq!("prometheus".parse::<ExporterKind>().unwrap(), ExporterKind::Prometheus);
        assert_eq!("STDOUT".parse::<ExporterKind>().unwrap(), ExporterKind::Stdout);
        assert!("UNKNOWN".parse::<ExporterKind>().is_err());
    }

    /// 守门 #12: Default = default_config.
    #[test]
    fn metrics_config_default() {
        let c: MetricsConfig = Default::default();
        assert_eq!(c, MetricsConfig::default_config());
    }
}
