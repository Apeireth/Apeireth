//! 脱敏策略 — 把 PII 替换为安全表示.
//!
//! 借鉴 VCP PrivacyGuard redaction strategies.
//!
//! 策略:
//! - Mask: 保留前 N + 后 N, 中间替换为 * (eg. "alice@example.com" -> "ali*******.com")
//! - Hash: SHA256 哈希前 16 字符 (eg. "alice@example.com" -> "a1b2c3d4e5f6g7h8")
//! - Remove: 直接删除 (eg. "alice@example.com" -> "[REDACTED]")
//! - ReplaceLabel: 替换为类型标签 (eg. "alice@example.com" -> "[EMAIL]")

#![deny(unsafe_code)]

use sha2::{Digest, Sha256};

use crate::pii::{PiiKind, PiiMatch};

/// 脱敏策略.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedactionStrategy {
    /// 部分掩码 (保留前 1 + 后 1, 中间 *)
    Mask,
    /// SHA256 哈希前 16 字符
    Hash,
    /// 删除并替换为 [REDACTED]
    Remove,
    /// 替换为类型标签 (e.g. [EMAIL])
    ReplaceLabel,
}

/// 应用单个匹配处的脱敏.
pub fn redact_one(value: &str, kind: PiiKind, strategy: RedactionStrategy) -> String {
    match strategy {
        RedactionStrategy::Mask => mask_value(value),
        RedactionStrategy::Hash => hash_value(value),
        RedactionStrategy::Remove => "[REDACTED]".to_string(),
        RedactionStrategy::ReplaceLabel => format!("[{}]", kind.as_str().to_uppercase()),
    }
}

/// 部分掩码: 保留前 1 + 后 1, 中间替换为 *.
/// 短于 4 字符全部替换为 *.
fn mask_value(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    let mut out = String::with_capacity(value.len());
    out.push(chars[0]);
    for _ in 1..(chars.len() - 1) {
        out.push('*');
    }
    out.push(*chars.last().unwrap());
    out
}

/// SHA256 哈希前 16 字符 (大写 hex).
fn hash_value(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    let hex = format!("{:x}", result);
    hex.chars().take(16).collect()
}

/// 批量脱敏文本,按 PiiMatch 列表 (必须按 start 升序).
pub fn redact_text(text: &str, matches: &[PiiMatch], strategy: RedactionStrategy) -> String {
    if matches.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for m in matches {
        if m.start >= text.len() || m.end > text.len() {
            continue;
        }
        // ae12d9eb: 跳过与已脱敏区间重叠的匹配 (如 EnvSecret 值内嵌 SecretToken/Email),
        // 否则会在 cursor 之前重复拼接脱敏内容造成输出错乱
        if m.start < cursor || m.end <= cursor {
            continue;
        }
        // 追加未匹配部分
        if cursor < m.start {
            out.push_str(&text[cursor..m.start]);
        }
        // 追加脱敏后内容
        out.push_str(&redact_one(&text[m.start..m.end], m.kind, strategy));
        cursor = m.end;
    }
    // 收尾
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_short_value() {
        assert_eq!(mask_value("abc"), "***");
        assert_eq!(mask_value("abcdef"), "a****f");
        assert_eq!(mask_value("abcdefgh"), "a******h");
    }

    #[test]
    fn hash_deterministic() {
        let h1 = hash_value("alice@example.com");
        let h2 = hash_value("alice@example.com");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn hash_changes_per_input() {
        let h1 = hash_value("alice@example.com");
        let h2 = hash_value("bob@example.com");
        assert_ne!(h1, h2);
    }

    #[test]
    fn redact_one_strategies() {
        let v = "alice@example.com";
        assert!(redact_one(v, PiiKind::Email, RedactionStrategy::Mask).contains('*'));
        assert_eq!(
            redact_one(v, PiiKind::Email, RedactionStrategy::Remove),
            "[REDACTED]"
        );
        assert_eq!(
            redact_one(v, PiiKind::Email, RedactionStrategy::ReplaceLabel),
            "[EMAIL]"
        );
    }

    #[test]
    fn redact_text_replaces_matches() {
        let text = "Email alice@example.com today";
        let matches = vec![PiiMatch {
            kind: PiiKind::Email,
            value: "alice@example.com".to_string(),
            start: 6,
            end: 23,
        }];
        let out = redact_text(text, &matches, RedactionStrategy::ReplaceLabel);
        assert_eq!(out, "Email [EMAIL] today");
    }

    #[test]
    fn redact_text_no_matches_passthrough() {
        let text = "no pii here";
        let out = redact_text(text, &[], RedactionStrategy::Remove);
        assert_eq!(out, text);
    }

    #[test]
    fn redact_text_overlapping_matches_no_corruption() {
        // ae12d9eb: EnvSecret 值内嵌 SecretToken (重叠匹配) — 只脱一次, 输出不错乱
        use crate::pii::detect_pii;
        let text = "API_KEY=sk-1234567890abcdefghijklmnopqrstuv";
        let matches = detect_pii(text);
        assert!(matches.len() >= 2, "应同时检出 EnvSecret + SecretToken");
        let out = redact_text(text, &matches, RedactionStrategy::Mask);
        assert!(
            out.starts_with("API_KEY=s"),
            "KEY= 前缀保留, 值部首字符保留: {}",
            out
        );
        assert!(out.ends_with('v'), "值部尾字符保留: {}", out);
        assert!(!out.contains("1234567890"), "token 主体不应可见: {}", out);
        assert!(
            !out.contains("sk-1234"),
            "token 前缀+主体不应连续可见: {}",
            out
        );
    }

    #[test]
    fn redact_text_multiple_matches() {
        let text = "a@b.com and 192.168.1.1";
        let matches = vec![
            PiiMatch {
                kind: PiiKind::Email,
                value: "a@b.com".to_string(),
                start: 0,
                end: 7,
            },
            PiiMatch {
                kind: PiiKind::IpAddress,
                value: "192.168.1.1".to_string(),
                start: 12,
                end: 23,
            },
        ];
        let out = redact_text(text, &matches, RedactionStrategy::ReplaceLabel);
        assert_eq!(out, "[EMAIL] and [IP_ADDRESS]");
    }
}
