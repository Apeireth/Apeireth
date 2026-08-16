//! # Label — metrics label 强校验 (K-1)
//!
//! Label 是 metrics 的 key-value 标注, 用于多维度切片
//! (e.g. `method="GET"`, `status="200"`, `path="/api/v1/users"`).
//!
//! ## K-1 强校验 (per Prometheus exposition format 规范)
//!
//! - **key**: 必须 `[a-zA-Z_][a-zA-Z0-9_]*` (与 metric name 不同: 不允许 `:`)
//! - **value**: 任意 UTF-8 字符, 但 **总长度 ≤ 256 字符**
//! - **数量**: 单 metric 最多 **10 个** label
//!
//! ## 1:1 翻译 v0.9.21 @anthropic-ai/metrics
//!
//! | apeireth-metrics | @anthropic-ai/metrics 商业版    | 1:1 |
//! |------------------|---------------------------------|-----|
//! | `Label`          | `type Label`                    | ✅  |
//! | `validate_key`   | `validateLabelKey`              | ✅  |
//! | `validate_value` | `validateLabelValue`            | ✅  |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::{MetricsError, MetricsResult};

// ============================================================================
// §1 Label 结构
// ============================================================================

/// 单个 label (key-value pair).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/metrics `type Label` 商业版.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label {
    /// key, K-1 强校验: `[a-zA-Z_][a-zA-Z0-9_]*`.
    pub key: String,
    /// value, K-1 强校验: ≤ 256 字符.
    pub value: String,
}

impl Label {
    /// 构造 (K-1 强校验).
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> MetricsResult<Self> {
        let key = key.into();
        let value = value.into();
        validate_label_key(&key)?;
        validate_label_value(&key, &value)?;
        Ok(Self { key, value })
    }

    /// 不校验构造 (用于 from HashMap 等已校验过的场景).
    pub fn new_unchecked(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// key 引用.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// value 引用.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=\"{}\"", self.key, escape_label_value(&self.value))
    }
}

// ============================================================================
// §2 校验函数 (K-1 强校验, 编译期守门)
// ============================================================================

/// 校验 label key (K-1 强校验).
///
/// 必须 `[a-zA-Z_][a-zA-Z0-9_]*` (注意: 不允许 `:`, 与 metric name 不同).
pub fn validate_label_key(key: &str) -> MetricsResult<()> {
    if key.is_empty() {
        return Err(MetricsError::LabelKeyInvalid(key.to_string()));
    }
    let mut chars = key.chars();
    let first = chars.next().expect("non-empty checked above");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(MetricsError::LabelKeyInvalid(key.to_string()));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(MetricsError::LabelKeyInvalid(key.to_string()));
        }
    }
    Ok(())
}

/// 校验 label value (K-1 强校验).
///
/// 总长度 ≤ 256 字符 (按 char count, 非 byte count, 跟 Prometheus 规范一致).
pub fn validate_label_value(key: &str, value: &str) -> MetricsResult<()> {
    let len = value.chars().count();
    if len > LABEL_VALUE_MAX_LEN {
        return Err(MetricsError::LabelValueTooLong {
            actual: len,
            key: key.to_string(),
        });
    }
    Ok(())
}

/// 校验 label 集合 (K-1 强校验).
///
/// - 单 metric 最多 10 label
/// - key 不可重复
/// - 每个 key / value 单独校验
pub fn validate_labels(labels: &HashMap<String, String>) -> MetricsResult<()> {
    if labels.len() > LABEL_MAX_COUNT {
        return Err(MetricsError::TooManyLabels {
            actual: labels.len(),
        });
    }
    for (k, v) in labels {
        validate_label_key(k)?;
        validate_label_value(k, v)?;
    }
    Ok(())
}

/// Label 转 Prometheus exposition format 字符串片段: `key="escaped_value"`.
pub fn label_to_prometheus(label: &Label) -> String {
    format!("{}=\"{}\"", label.key, escape_label_value(&label.value))
}

/// Label 集合按 key 排序后拼接 (Prometheus exposition format 要求 label 按 key 排序).
pub fn labels_to_prometheus_sorted(labels: &HashMap<String, String>) -> String {
    let mut entries: Vec<(&String, &String)> = labels.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
        .iter()
        .map(|(k, v)| format!("{k}=\"{}\"", escape_label_value(v)))
        .collect::<Vec<_>>()
        .join(",")
}

/// 转义 label value 中的特殊字符 (per Prometheus exposition format).
///
/// - `\` → `\\`
/// - `"` → `\"`
/// - `\n` → `\\n`
fn escape_label_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

// ============================================================================
// §3 Crate-level 常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// Label value 最大长度 (Prometheus exposition format 上限).
pub const LABEL_VALUE_MAX_LEN: usize = 256;

/// 单 metric 最多 label 数.
pub const LABEL_MAX_COUNT: usize = 10;

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: valid key 通过校验.
    #[test]
    fn validate_label_key_valid() {
        assert!(validate_label_key("method").is_ok());
        assert!(validate_label_key("status_code").is_ok());
        assert!(validate_label_key("_internal").is_ok());
        assert!(validate_label_key("a").is_ok());
        assert!(validate_label_key("A1").is_ok());
    }

    /// 守门 #2: K-1 invalid key 拒 (数字开头).
    #[test]
    fn validate_label_key_starts_with_digit_rejected() {
        assert!(matches!(
            validate_label_key("1method"),
            Err(MetricsError::LabelKeyInvalid(_))
        ));
    }

    /// 守门 #3: K-1 invalid key 拒 (含 `-`).
    #[test]
    fn validate_label_key_dash_rejected() {
        assert!(matches!(
            validate_label_key("status-code"),
            Err(MetricsError::LabelKeyInvalid(_))
        ));
    }

    /// 守门 #4: K-1 invalid key 拒 (含 `:`).
    #[test]
    fn validate_label_key_colon_rejected() {
        // 注意: metric name 允许 `:`, 但 label key 不允许
        assert!(matches!(
            validate_label_key("foo:bar"),
            Err(MetricsError::LabelKeyInvalid(_))
        ));
    }

    /// 守门 #5: K-1 invalid key 拒 (空字符串).
    #[test]
    fn validate_label_key_empty_rejected() {
        assert!(matches!(
            validate_label_key(""),
            Err(MetricsError::LabelKeyInvalid(_))
        ));
    }

    /// 守门 #6: K-1 invalid key 拒 (含空格).
    #[test]
    fn validate_label_key_space_rejected() {
        assert!(matches!(
            validate_label_key("foo bar"),
            Err(MetricsError::LabelKeyInvalid(_))
        ));
    }

    /// 守门 #7: K-1 value 太长拒 (> 256 字符).
    #[test]
    fn validate_label_value_too_long_rejected() {
        let long = "a".repeat(257);
        assert!(matches!(
            validate_label_value("method", &long),
            Err(MetricsError::LabelValueTooLong { actual: 257, .. })
        ));
    }

    /// 守门 #8: K-1 value 正好 256 字符通过.
    #[test]
    fn validate_label_value_256_ok() {
        let exact = "a".repeat(256);
        assert!(validate_label_value("method", &exact).is_ok());
    }

    /// 守门 #9: 最多 10 label, 第 11 个拒.
    #[test]
    fn validate_labels_max_10() {
        let mut labels = HashMap::new();
        for i in 0..10 {
            labels.insert(format!("k{i}"), format!("v{i}"));
        }
        assert!(validate_labels(&labels).is_ok());
        // 加第 11 个
        labels.insert("k10".to_string(), "v10".to_string());
        assert!(matches!(
            validate_labels(&labels),
            Err(MetricsError::TooManyLabels { actual: 11 })
        ));
    }

    /// 守门 #10: Label::new 强校验.
    #[test]
    fn label_new_validates() {
        assert!(Label::new("method", "GET").is_ok());
        assert!(Label::new("123-bad", "GET").is_err());
    }

    /// 守门 #11: Label::new_unchecked 不校验 (用于内部 HashMap 转换).
    #[test]
    fn label_new_unchecked_skips_validation() {
        let l = Label::new_unchecked("123-bad", "GET");
        assert_eq!(l.key, "123-bad");
    }

    /// 守门 #12: Display 包含 key=value.
    #[test]
    fn label_display() {
        let l = Label::new("method", "GET").unwrap();
        assert_eq!(format!("{l}"), "method=\"GET\"");
    }

    /// 守门 #13: 转义特殊字符.
    #[test]
    fn escape_label_value_works() {
        assert_eq!(escape_label_value("normal"), "normal");
        assert_eq!(escape_label_value("a\"b"), "a\\\"b");
        assert_eq!(escape_label_value("a\\b"), "a\\\\b");
        assert_eq!(escape_label_value("a\nb"), "a\\nb");
    }

    /// 守门 #14: labels_to_prometheus_sorted 按 key 排序.
    #[test]
    fn labels_sorted_by_key() {
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        labels.insert("status".to_string(), "200".to_string());
        labels.insert("path".to_string(), "/x".to_string());
        let s = labels_to_prometheus_sorted(&labels);
        // 按字母序: method < path < status
        assert_eq!(s, r#"method="GET",path="/x",status="200""#);
    }

    /// 守门 #15: LABEL_MAX_COUNT = 10 (K-1 守门).
    #[test]
    fn k1_label_max_count() {
        assert_eq!(LABEL_MAX_COUNT, 10);
    }

    /// 守门 #16: LABEL_VALUE_MAX_LEN = 256 (K-1 守门).
    #[test]
    fn k1_label_value_max_len() {
        assert_eq!(LABEL_VALUE_MAX_LEN, 256);
    }
}
