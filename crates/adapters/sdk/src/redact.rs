//! 日志脱敏原语 (秘密值包装 + 稳定脱敏展示).
//!
//! 协议层 (信令 / 语音流) 会持有访问令牌、API key、签名密钥等秘密值。
//! 本模块提供 [`SecretValue`]: `Debug` / `Display` 一律输出 `[redacted]`,
//! 原值只能通过显式 [`SecretValue::reveal`] 取出 —— 让 `{:?}` / `dbg!` /
//! `tracing` 误打日志时不泄露。
//!
//! 配套:
//! - [`redact_keep_prefix`]: 保留前缀字符做问题定位 (默认只保留 4 个),
//!   剩余部分以 `…[redacted]` 收尾, 长度不回显。
//! - [`secret_debug_len`]: 只暴露"该秘密存在且多长", 供健康检查用。

use serde::{Deserialize, Serialize};

/// 脱敏占位符 (稳定字面量, 测试钉死).
pub const REDACTED_PLACEHOLDER: &str = "[redacted]";

/// 秘密值包装: `Debug` / `Display` 恒脱敏, 原值仅 `reveal()` 可取.
///
/// 等值比较按原值进行 (协议帧回环测试需要), 但绝不经由格式化输出泄露。
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretValue(String);

impl SecretValue {
    /// 包装一个秘密值.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 取出原值 (调用方负责不落日志).
    pub fn reveal(&self) -> &str {
        &self.0
    }

    /// 秘密长度 (只暴露长度, 不暴露内容).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空秘密.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED_PLACEHOLDER)
    }
}

impl std::fmt::Display for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED_PLACEHOLDER)
    }
}

impl From<String> for SecretValue {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SecretValue {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 保留前缀 (至多 `keep` 个字符) 的脱敏展示.
///
/// `keep` 会被钳到 0..=8; 剩余部分统一以 `…[redacted]` 收尾, 不回显原长度。
/// 前缀不足 `keep` 个字符时直接整体脱敏, 防止短秘密被完整展示。
pub fn redact_keep_prefix(value: &str, keep: usize) -> String {
    let keep = keep.min(8);
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= keep {
        return REDACTED_PLACEHOLDER.to_string();
    }
    let prefix: String = chars[..keep].iter().collect();
    format!("{prefix}…{REDACTED_PLACEHOLDER}")
}

/// 只报告"秘密存在且长度为 N", 供健康检查 / 状态上报用.
pub fn secret_debug_len(value: &SecretValue) -> String {
    format!("{REDACTED_PLACEHOLDER} (len={})", value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_value_debug_and_display_are_redacted() {
        let secret = SecretValue::new("super-secret-token-1234567890");
        let dbg = format!("{secret:?}");
        let disp = format!("{secret}");
        assert_eq!(dbg, REDACTED_PLACEHOLDER);
        assert_eq!(disp, REDACTED_PLACEHOLDER);
        assert!(!dbg.contains("super-secret"));
        assert!(!disp.contains("super-secret"));
    }

    #[test]
    fn secret_value_reveal_roundtrip() {
        let secret = SecretValue::new("token-abc".to_string());
        assert_eq!(secret.reveal(), "token-abc");
        assert_eq!(secret.len(), 9);
        assert!(!secret.is_empty());
        assert!(SecretValue::new("").is_empty());
    }

    #[test]
    fn secret_value_equality_uses_inner_value() {
        let a = SecretValue::new("same");
        let b = SecretValue::new("same");
        let c = SecretValue::new("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn secret_value_serde_roundtrip() {
        let secret = SecretValue::new("wire-token");
        let json = serde_json::to_string(&secret).expect("serialize");
        // 序列化是 wire 兼容面 (原值), 但 Debug 永远脱敏
        assert!(json.contains("wire-token"));
        let back: SecretValue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, secret);
        assert_eq!(format!("{back:?}"), REDACTED_PLACEHOLDER);
    }

    #[test]
    fn redact_keep_prefix_keeps_short_prefix_only() {
        let out = redact_keep_prefix("abcdefghijklmnop", 4);
        assert!(out.starts_with("abcd"));
        assert!(out.ends_with(REDACTED_PLACEHOLDER));
        assert!(!out.contains("efgh"));
    }

    #[test]
    fn redact_keep_prefix_fully_redacts_short_values() {
        assert_eq!(redact_keep_prefix("abcd", 4), REDACTED_PLACEHOLDER);
        assert_eq!(redact_keep_prefix("ab", 8), REDACTED_PLACEHOLDER);
    }

    #[test]
    fn redact_keep_prefix_clamps_keep_to_8() {
        let out = redact_keep_prefix("0123456789abcdef", 100);
        assert!(out.starts_with("01234567"));
        assert!(!out.contains("89"));
    }

    #[test]
    fn secret_debug_len_reports_length_only() {
        let secret = SecretValue::new("1234567890");
        let out = secret_debug_len(&secret);
        assert!(out.contains(REDACTED_PLACEHOLDER));
        assert!(out.contains("len=10"));
        assert!(!out.contains("1234567890"));
    }
}
