//! PII 检测 — 邮箱 / 电话 / SSN / 信用卡 / IP 等敏感信息识别.
//!
//! 借鉴 VCP PrivacyGuard (字段级: 5 类 PII + 自定义 regex).
//!
//! 不漂移:
//! - 0 依赖 VCP 内部代码, 仅借鉴分类法
//! - 0 副作用, 纯函数检测

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// PII 类型 — 字段级引用 VCP 5 类 (Email / Phone / SSN / CreditCard / IP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PiiKind {
    /// 邮箱
    Email,
    /// 电话
    Phone,
    /// 社会安全号 (美国 SSN 格式 XXX-XX-XXXX)
    Ssn,
    /// 信用卡号 (13-19 位)
    CreditCard,
    /// IP 地址 (v4)
    IpAddress,
    /// URL 含凭证
    UrlWithCredentials,
}

impl PiiKind {
    pub const ALL: [PiiKind; 6] = [
        Self::Email,
        Self::Phone,
        Self::Ssn,
        Self::CreditCard,
        Self::IpAddress,
        Self::UrlWithCredentials,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Ssn => "ssn",
            Self::CreditCard => "credit_card",
            Self::IpAddress => "ip_address",
            Self::UrlWithCredentials => "url_with_credentials",
        }
    }
}

/// PII 匹配结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiMatch {
    /// PII 类型
    pub kind: PiiKind,
    /// 原始值
    pub value: String,
    /// 起始字节位置
    pub start: usize,
    /// 结束字节位置 (exclusive)
    pub end: usize,
}

impl PiiMatch {
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

// ============================================
// 静态 regex 模式 (编译期 hardcode)
// ============================================

/// 邮箱: 简化 RFC 5322 (本地部分 @ 域名).
static EMAIL_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
        .expect("email regex must compile")
});

/// 电话 (宽松): 国际格式 / 北美 / 中国手机.
/// - 国际: +XXX-XXX-XXX-XXXX
/// - 北美: (XXX) XXX-XXXX, XXX-XXX-XXXX
/// - 中国: 1XXXXXXXXXX (11 位)
static PHONE_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?x)
        \+?\d{1,3}[\s\-]?\d{2,4}[\s\-]?\d{3,4}[\s\-]?\d{3,4}
        | \(\d{3}\)\s?\d{3}[\s\-]?\d{4}
        | \d{3}-\d{3}-\d{4}
        | 1[3-9]\d{9}
    ")
    .expect("phone regex must compile")
});

/// SSN: XXX-XX-XXXX.
static SSN_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex must compile")
});

/// 信用卡: 13-19 位连续数字 (粗筛, 实际应做 Luhn 校验).
static CC_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"\b\d{13,19}\b").expect("credit card regex must compile")
});

/// IP v4: X.X.X.X.
static IP_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("ip regex must compile")
});

/// URL 含凭证: protocol://user:password@host.
static URL_CREDS_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"[a-zA-Z][a-zA-Z0-9+\-.]*://[^\s/:@]+:[^\s/:@]+@[^\s/]+")
        .expect("url creds regex must compile")
});

/// 检测文本中所有 PII 匹配.
pub fn detect_pii(text: &str) -> Vec<PiiMatch> {
    let mut matches = Vec::new();
    for cap in EMAIL_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::Email,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    for cap in PHONE_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::Phone,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    for cap in SSN_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::Ssn,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    for cap in CC_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::CreditCard,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    for cap in IP_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::IpAddress,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    for cap in URL_CREDS_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::UrlWithCredentials,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    // 按 start 排序 (后续 redact 需要有序)
    matches.sort_by_key(|m| m.start);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_email_basic() {
        let m = detect_pii("contact alice@example.com today");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].kind, PiiKind::Email);
        assert_eq!(m[0].value, "alice@example.com");
    }

    #[test]
    fn detect_phone_us_format() {
        let m = detect_pii("call (415) 555-1234 now");
        assert!(m.iter().any(|x| x.kind == PiiKind::Phone));
    }

    #[test]
    fn detect_phone_cn_mobile() {
        let m = detect_pii("call 13800138000 please");
        assert!(m.iter().any(|x| x.kind == PiiKind::Phone));
    }

    #[test]
    fn detect_ssn() {
        let m = detect_pii("my ssn is 123-45-6789 ok");
        assert!(m.iter().any(|x| x.kind == PiiKind::Ssn && x.value == "123-45-6789"));
    }

    #[test]
    fn detect_credit_card() {
        let m = detect_pii("card 4111111111111111 here");
        assert!(m.iter().any(|x| x.kind == PiiKind::CreditCard));
    }

    #[test]
    fn detect_ip_v4() {
        let m = detect_pii("server 192.168.1.1 alive");
        assert!(m.iter().any(|x| x.kind == PiiKind::IpAddress && x.value == "192.168.1.1"));
    }

    #[test]
    fn detect_url_with_credentials() {
        let m = detect_pii("db at https://admin:secret@db.example.com");
        assert!(m.iter().any(|x| x.kind == PiiKind::UrlWithCredentials));
    }

    #[test]
    fn no_pii_in_clean_text() {
        let m = detect_pii("the sky is blue today");
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn matches_sorted_by_start() {
        let m = detect_pii("a@b.com and 192.168.1.1");
        assert!(m.len() >= 2);
        for i in 1..m.len() {
            assert!(m[i - 1].start <= m[i].start, "matches must be sorted by start");
        }
    }

    #[test]
    fn pii_kind_as_str_covers_all() {
        for k in PiiKind::ALL {
            assert!(!k.as_str().is_empty());
        }
    }
}
