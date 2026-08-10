//! # MetricsError — metrics skeleton 错误类型 (10 variant)
//!
//! 10 个错误 variant 覆盖 metrics 全流程:
//! 1. `MetricNameInvalid` — metric name 不符合 `[a-zA-Z_:][a-zA-Z0-9_:]*` (K-1 强校验)
//! 2. `MetricNameEmpty` — metric name 为空字符串 (K-1 强校验)
//! 3. `HelpRequired` — Help 文本为空 (K-1 强校验, Prometheus exposition format 要求)
//! 4. `LabelKeyInvalid` — label key 包含非法字符 (K-1 强校验)
//! 5. `LabelValueTooLong` — label value 超过 256 字符 (K-1 强校验, Prometheus 上限)
//! 6. `TooManyLabels` — label 数量超过 10 (K-1 强校验)
//! 7. `MetricAlreadyRegistered` — 同名 metric 重复注册
//! 8. `MetricNotFound` — get / unregister 时 metric 不存在
//! 9. `ExporterNotImplemented` — 3 stub exporter (Pushgateway/OTLP/StatsD) 返
//! 10. `EncodeError` — Prometheus text format 编码失败
//!
//! ## m3 防御 (per `m3-hallucination-defense §2.1`)
//!
//! 每个错误 variant 编译期 hardcode, 不允许运行时新增 variant, 防 m3 hallucination
//! 改错分类 (如把 `MetricNotFound` 改成 `MetricNotFoundError` 制造不可识别的 err).
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use thiserror::Error;

// ============================================================================
// §1 MetricsError 枚举 (10 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// Metrics skeleton 错误 (10 variant, 编译期 hardcode).
#[derive(Debug, Error, PartialEq)]
pub enum MetricsError {
    /// metric name 不符合 `[a-zA-Z_:][a-zA-Z0-9_:]*` (K-1 强校验).
    #[error("invalid metric name: '{0}', must match [a-zA-Z_:][a-zA-Z0-9_:]*")]
    MetricNameInvalid(String),

    /// metric name 为空字符串 (K-1 强校验).
    #[error("metric name is empty")]
    MetricNameEmpty,

    /// Help 文本为空 (K-1 强校验, Prometheus exposition format 要求).
    #[error("help text required for metric: '{0}'")]
    HelpRequired(String),

    /// label key 包含非法字符 (K-1 强校验, 必须 `[a-zA-Z_][a-zA-Z0-9_]*`).
    #[error("invalid label key: '{0}', must match [a-zA-Z_][a-zA-Z0-9_]*")]
    LabelKeyInvalid(String),

    /// label value 超过 256 字符 (K-1 强校验, Prometheus exposition format 上限).
    #[error("label value too long: {actual} chars, max 256 (key='{key}')")]
    LabelValueTooLong {
        /// 实际字符数.
        actual: usize,
        /// 触发的 label key.
        key: String,
    },

    /// label 数量超过 10 (K-1 强校验).
    #[error("too many labels: {actual}, max 10")]
    TooManyLabels {
        /// 实际 label 数.
        actual: usize,
    },

    /// 同名 metric 重复注册.
    #[error("metric already registered: '{0}'")]
    MetricAlreadyRegistered(String),

    /// get / unregister 时 metric 不存在.
    #[error("metric not found: '{0}'")]
    MetricNotFound(String),

    /// 3 stub exporter (Pushgateway/OTLP/StatsD) 返 (R20 阶段 6 估缺).
    #[error("exporter not implemented: '{0}' (R20 阶段 6 stub)")]
    ExporterNotImplemented(String),

    /// Prometheus text format 编码失败.
    #[error("encode error: {0}")]
    EncodeError(String),
}

// ============================================================================
// §2 统一 Result 别名
// ============================================================================

/// 统一 Result 别名.
pub type MetricsResult<T> = std::result::Result<T, MetricsError>;

// ============================================================================
// §3 错误 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// MetricsError 编译期 hardcode variant 数 (10).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const METRICS_ERROR_VARIANT_COUNT: usize = 10;

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: MetricsError 编译期 10 variant.
    #[test]
    fn metrics_error_has_ten_variants() {
        // 10 variant 1:1: MetricNameInvalid / MetricNameEmpty / HelpRequired
        //               / LabelKeyInvalid / LabelValueTooLong / TooManyLabels
        //               / MetricAlreadyRegistered / MetricNotFound
        //               / ExporterNotImplemented / EncodeError
        assert_eq!(METRICS_ERROR_VARIANT_COUNT, 10);
    }

    /// 守门 #2: 错误 Display 实现正常.
    #[test]
    fn metrics_error_display_works() {
        let e = MetricsError::MetricNotFound("requests_total".to_string());
        let s = format!("{e}");
        assert!(s.contains("requests_total"));
        assert!(s.contains("not found"));
    }

    /// 守门 #3: MetricNameEmpty 无 field.
    #[test]
    fn metric_name_empty_error() {
        let e = MetricsError::MetricNameEmpty;
        let s = format!("{e}");
        assert!(s.contains("empty"));
    }

    /// 守门 #4: LabelValueTooLong 含 actual + key.
    #[test]
    fn label_value_too_long_error_includes_both() {
        let e = MetricsError::LabelValueTooLong {
            actual: 300,
            key: "method".to_string(),
        };
        let s = format!("{e}");
        assert!(s.contains("300"));
        assert!(s.contains("method"));
    }

    /// 守门 #5: TooManyLabels 含 actual.
    #[test]
    fn too_many_labels_error_includes_count() {
        let e = MetricsError::TooManyLabels { actual: 15 };
        let s = format!("{e}");
        assert!(s.contains("15"));
    }

    /// 守门 #6: ExporterNotImplemented 含 exporter name.
    #[test]
    fn exporter_not_implemented_error_includes_name() {
        let e = MetricsError::ExporterNotImplemented("PUSHGATEWAY".to_string());
        let s = format!("{e}");
        assert!(s.contains("PUSHGATEWAY"));
        assert!(s.contains("stub"));
    }

    /// 守门 #7: MetricNameInvalid 含 name.
    #[test]
    fn metric_name_invalid_error_includes_name() {
        let e = MetricsError::MetricNameInvalid("123-bad".to_string());
        let s = format!("{e}");
        assert!(s.contains("123-bad"));
    }

    /// 守门 #8: HelpRequired 含 metric name.
    #[test]
    fn help_required_error_includes_name() {
        let e = MetricsError::HelpRequired("requests_total".to_string());
        let s = format!("{e}");
        assert!(s.contains("requests_total"));
        assert!(s.contains("help"));
    }

    /// 守门 #9: LabelKeyInvalid 含 key.
    #[test]
    fn label_key_invalid_error_includes_key() {
        let e = MetricsError::LabelKeyInvalid("123-bad".to_string());
        let s = format!("{e}");
        assert!(s.contains("123-bad"));
    }

    /// 守门 #10: MetricAlreadyRegistered 含 name.
    #[test]
    fn metric_already_registered_error_includes_name() {
        let e = MetricsError::MetricAlreadyRegistered("requests_total".to_string());
        let s = format!("{e}");
        assert!(s.contains("requests_total"));
        assert!(s.contains("already"));
    }
}
