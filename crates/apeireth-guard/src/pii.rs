//! PII 检测 — 邮箱 / 电话 / SSN / 信用卡 / IP / 密钥 token / env 赋值行 等敏感信息识别.
//!
//! 借鉴 VCP PrivacyGuard (字段级: 5 类 PII + 自定义 regex).
//!
//! 任务 ae12d9eb 增量补强 (对齐 VCP toolResultPrivacyGuard.js):
//! - SecretToken: 7 类高置信密钥前缀 (sk-/sk-proj-/xox*/ghp_/github_pat_/glpat-/AKIA)
//! - EnvSecret: 敏感键名 env 赋值行 (KEY=VALUE / KEY: VALUE) 的值部
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
    /// 高置信密钥 token (sk- / ghp_ / AKIA... 等前缀模式)
    SecretToken,
    /// env 赋值行敏感值 (TOKEN/SECRET/KEY/PASSWORD 等键名的 KEY=VALUE / KEY: VALUE)
    EnvSecret,
}

impl PiiKind {
    pub const ALL: [PiiKind; 8] = [
        Self::Email,
        Self::Phone,
        Self::Ssn,
        Self::CreditCard,
        Self::IpAddress,
        Self::UrlWithCredentials,
        Self::SecretToken,
        Self::EnvSecret,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Ssn => "ssn",
            Self::CreditCard => "credit_card",
            Self::IpAddress => "ip_address",
            Self::UrlWithCredentials => "url_with_credentials",
            Self::SecretToken => "secret_token",
            Self::EnvSecret => "env_secret",
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

// ============================================
// ae12d9eb 增量: 高置信密钥 token + env 赋值行
// (借鉴 VCP toolResultPrivacyGuard.js HIGH_CONFIDENCE_TOKEN_PATTERNS /
//  ENV_ASSIGNMENT_PATTERN / SENSITIVE_KEY_PATTERN, 长度阈值与 tool-runtime/privacy.rs 同源)
// ============================================

/// 高置信密钥 token: 7 类前缀模式 (词边界 + 最小长度, 控制误报).
/// - sk- / sk-proj- (OpenAI, ≥24)
/// - xoxb/xoxp/xoxa/xoxr- (Slack, ≥24)
/// - ghp_ (GitHub PAT, ≥30) / github_pat_ (≥40)
/// - glpat- (GitLab, ≥20)
/// - AKIA + 16 位大写字母数字 (AWS access key id)
static SECRET_TOKEN_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(concat!(
        r"\b(?:",
        r"sk-proj-[A-Za-z0-9_-]{24,}",
        r"|sk-[A-Za-z0-9_-]{24,}",
        r"|(?:xoxb|xoxp|xoxa|xoxr)-[A-Za-z0-9-]{24,}",
        r"|ghp_[A-Za-z0-9]{30,}",
        r"|github_pat_[A-Za-z0-9_]{40,}",
        r"|glpat-[A-Za-z0-9_-]{20,}",
        r"|AKIA[0-9A-Z]{16}",
        r")\b"
    ))
    .expect("secret token regex must compile")
});

/// env 赋值行: `[export] KEY=VALUE` / `[export] KEY: VALUE` (多行模式).
/// value 组只匹配单 token 值 (不含引号/空白), 引号由可选段吃掉.
static ENV_ASSIGN_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(concat!(
        r"(?m)^[ \t]*(?:export[ \t]+)?",
        r"(?P<key>[A-Za-z_][A-Za-z0-9_.\-]*)[ \t]*[=:][ \t]*",
        r#"['"]?(?P<value>[^\s'"\r\n]+)['"]?[ \t\r]*(?:#.*)?$"#
    ))
    .expect("env assignment regex must compile")
});

/// 敏感键名模式 (与 tool-runtime/privacy.rs sensitive_key_pattern 同源):
/// api_key / secret / token / password / credential / private_key / ... 等 16 类,
/// 词边界锚定 (前后须为边界或 _-\s.), 防止 monkey/keyword 之类子串误报.
static SENSITIVE_KEY_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(concat!(
        r"(?i)(?:^|[_\-\s.])",
        r"(?:api[_\-\s]?key|apikey|secret|token|access[_\-\s]?token|refresh[_\-\s]?token",
        r"|auth[_\-\s]?token|bearer|password|passwd|pwd|credential|credentials",
        r"|private[_\-\s]?key|client[_\-\s]?secret|webhook[_\-\s]?secret)",
        r"(?:$|[_\-\s.])"
    ))
    .expect("sensitive key regex must compile")
});

/// env 值是否值得脱敏 (对齐 tool-runtime should_mask_value 的误报控制):
/// 长度 ≥ 8, 且不是布尔/null 字面量或纯数字.
fn env_value_maskable(value: &str) -> bool {
    if value.len() < 8 {
        return false;
    }
    let lower = value.to_lowercase();
    if matches!(lower.as_str(), "true" | "false" | "null" | "undefined" | "none") {
        return false;
    }
    value.parse::<f64>().is_err()
}

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
    // ae12d9eb 增量: 高置信密钥 token
    for cap in SECRET_TOKEN_RE.find_iter(text) {
        matches.push(PiiMatch {
            kind: PiiKind::SecretToken,
            value: cap.as_str().to_string(),
            start: cap.start(),
            end: cap.end(),
        });
    }
    // ae12d9eb 增量: env 赋值行 — 敏感键名 + 值可脱敏 → 只报 value 段 (KEY= 前缀保留)
    for cap in ENV_ASSIGN_RE.captures_iter(text) {
        let key = cap.name("key").map(|m| m.as_str()).unwrap_or("");
        let Some(value_m) = cap.name("value") else { continue };
        if !SENSITIVE_KEY_RE.is_match(key) {
            continue;
        }
        if !env_value_maskable(value_m.as_str()) {
            continue;
        }
        matches.push(PiiMatch {
            kind: PiiKind::EnvSecret,
            value: value_m.as_str().to_string(),
            start: value_m.start(),
            end: value_m.end(),
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

    // ============================================
    // ae12d9eb 增量: SecretToken 检测 (7 类前缀)
    // ============================================

    #[test]
    fn detect_secret_token_sk() {
        let m = detect_pii("key is sk-1234567890abcdefghijklmnopqrstuv ok");
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken));
    }

    #[test]
    fn detect_secret_token_sk_proj() {
        let m = detect_pii("k=sk-proj-abcdefghijklmnopqrstuvwxyz1234");
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken));
    }

    #[test]
    fn detect_secret_token_ghp_and_github_pat() {
        let m = detect_pii("ghp_abcdefghijklmnopqrstuvwxyz1234");
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken));
        let m2 = detect_pii("github_pat_abcdefghijklmnopqrstuvwxyz12345678901234");
        assert!(m2.iter().any(|x| x.kind == PiiKind::SecretToken));
    }

    #[test]
    fn detect_secret_token_aws_akia() {
        let m = detect_pii("aws key AKIAIOSFODNN7EXAMPLE here");
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken && x.value == "AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn detect_secret_token_slack_and_gitlab() {
        let m = detect_pii("xoxb-123456789012345678901234");
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken));
        let m2 = detect_pii("glpat-abcdefghijklmnopqrst");
        assert!(m2.iter().any(|x| x.kind == PiiKind::SecretToken));
    }

    #[test]
    fn secret_token_short_not_detected() {
        // 短 token (低于最小长度阈值) 不检测 — 误报控制
        assert!(detect_pii("sk-abc").is_empty());
        assert!(detect_pii("sk-ab 短 token 保留").iter().all(|x| x.kind != PiiKind::SecretToken));
    }

    // ============================================
    // ae12d9eb 增量: EnvSecret 检测 (KEY=VALUE / KEY: VALUE)
    // ============================================

    #[test]
    fn detect_env_secret_equals() {
        let m = detect_pii("API_KEY=supersecretvalue123");
        let env: Vec<_> = m.iter().filter(|x| x.kind == PiiKind::EnvSecret).collect();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].value, "supersecretvalue123");
    }

    #[test]
    fn detect_env_secret_colon_form() {
        let m = detect_pii("AUTH_TOKEN: myverylongtoken123");
        assert!(m.iter().any(|x| x.kind == PiiKind::EnvSecret && x.value == "myverylongtoken123"));
    }

    #[test]
    fn detect_env_secret_export_quoted_comment() {
        let m = detect_pii("export DB_PASSWORD='hunter2hunter2' # 注释");
        let env: Vec<_> = m.iter().filter(|x| x.kind == PiiKind::EnvSecret).collect();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].value, "hunter2hunter2");
    }

    #[test]
    fn detect_env_secret_multiline_block() {
        let text = "LOG_LEVEL=debug\nDATABASE_PASSWORD=supersecret99\nHOME=/home/user\n";
        let m = detect_pii(text);
        let env: Vec<_> = m.iter().filter(|x| x.kind == PiiKind::EnvSecret).collect();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].value, "supersecret99");
    }

    #[test]
    fn env_secret_non_sensitive_key_not_detected() {
        // 非敏感键名不误伤
        assert!(detect_pii("HOME=/usr/local/bin").iter().all(|x| x.kind != PiiKind::EnvSecret));
        assert!(detect_pii("PATH=/usr/bin:/bin").iter().all(|x| x.kind != PiiKind::EnvSecret));
        assert!(detect_pii("monkey=abc12345678").iter().all(|x| x.kind != PiiKind::EnvSecret));
        assert!(detect_pii("keyword=abcdefgh").iter().all(|x| x.kind != PiiKind::EnvSecret));
    }

    #[test]
    fn env_secret_short_numeric_bool_not_detected() {
        // 值过短 / 纯数字 / 布尔字面量 不检测 — 误报控制
        assert!(detect_pii("API_KEY=abc").iter().all(|x| x.kind != PiiKind::EnvSecret));
        assert!(detect_pii("TOKEN_COUNT=42").iter().all(|x| x.kind != PiiKind::EnvSecret));
        assert!(detect_pii("PASSWORD=123456789").iter().all(|x| x.kind != PiiKind::EnvSecret));
    }

    #[test]
    fn clean_text_with_sk_substring_not_detected() {
        // 正常文本中 sk- 子串不误伤 (词边界控制)
        let m = detect_pii("flask-mode and risk-taking are fine workflows");
        assert!(m.iter().all(|x| x.kind != PiiKind::SecretToken));
    }

    #[test]
    fn env_value_with_token_both_detected_sorted() {
        // 嵌套场景: env 值本身是高置信 token → 两类都检出且按 start 有序
        let text = "API_KEY=sk-1234567890abcdefghijklmnopqrstuv";
        let m = detect_pii(text);
        assert!(m.iter().any(|x| x.kind == PiiKind::EnvSecret));
        assert!(m.iter().any(|x| x.kind == PiiKind::SecretToken));
        for i in 1..m.len() {
            assert!(m[i - 1].start <= m[i].start);
        }
    }
}
