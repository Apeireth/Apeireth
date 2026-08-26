//! Integration tests for apeireth-guard (post-1.0.0)

#![allow(missing_docs)]

use apeireth_guard::{detect_pii, redact_one, redact_text, PiiKind, PiiMatch, RedactionStrategy};

#[test]
fn detect_pii_email() {
    let r = detect_pii("contact me at user@example.com please");
    assert!(r
        .iter()
        .any(|m| m.kind == PiiKind::Email && m.value == "user@example.com"));
}

#[test]
fn detect_pii_phone() {
    let r = detect_pii("call me at 555-123-4567 today");
    assert!(r.iter().any(|m| m.kind == PiiKind::Phone));
}

#[test]
fn detect_pii_ssn() {
    let r = detect_pii("SSN 123-45-6789");
    assert!(r.iter().any(|m| m.kind == PiiKind::Ssn));
}

#[test]
fn detect_pii_credit_card() {
    let r = detect_pii("card 4111111111111111 here");
    assert!(r.iter().any(|m| m.kind == PiiKind::CreditCard));
}

#[test]
fn detect_pii_ip_address() {
    let r = detect_pii("server at 192.168.1.1");
    assert!(r
        .iter()
        .any(|m| m.kind == PiiKind::IpAddress && m.value == "192.168.1.1"));
}

#[test]
fn detect_pii_url_with_credentials() {
    let r = detect_pii("see https://user:pass@example.com/path");
    assert!(r.iter().any(|m| m.kind == PiiKind::UrlWithCredentials));
}

#[test]
fn detect_pii_high_entropy_token_7_prefixes() {
    let fake_prefixes = [
        "FAKE-type1-1234567890abcdefxyz",
        "FAKE-type2-1234567890abcdefxyzABCD",
        "FAKE-type3-1234567890abcdefxyz",
        "FAKE-type4-1234567890abcdefxyzABCD",
        "FAKE-type5-1234567890abcdefxyzABCD12",
        "FAKE-type6-1234567890abcdefxyz",
        "FAKE-type7-EXAMPLE00000000",
    ];
    for token in fake_prefixes {
        let r = detect_pii(&format!("key {token} here"));
        assert!(
            !r.iter().any(|m| m.kind == PiiKind::SecretToken),
            "fake token 不应误报 SecretToken: {token}: r={r:?}"
        );
    }
}

#[test]
fn detect_pii_env_assignment() {
    for line in [
        "apikey=FAKEPLACEHOLDER12345",
        "apikey: FAKEPLACEHOLDER00",
        "apikey=FAKEhunter2hunter",
    ] {
        let r = detect_pii(line);
        assert!(
            r.iter().any(|m| m.kind == PiiKind::EnvSecret),
            "EnvSecret 应匹配 {line}: r={r:?}"
        );
    }
}

#[test]
fn detect_pii_text_without_pii_returns_empty() {
    let r = detect_pii("this text has no pii, just some lorem ipsum dolor sit amet");
    assert!(r.is_empty());
}

#[test]
fn detect_pii_multiple_types_in_one_text() {
    let r = detect_pii("email a@b.com or call 555-123-4567");
    let has_email = r.iter().any(|m| m.kind == PiiKind::Email);
    let has_phone = r.iter().any(|m| m.kind == PiiKind::Phone);
    assert!(has_email && has_phone, "2 类 PII 应各自归类");
}

#[test]
fn detect_pii_offset_is_byte_accurate() {
    let r = detect_pii("a@b.com");
    let m = r.iter().find(|m| m.kind == PiiKind::Email).unwrap();
    assert_eq!(&"a@b.com"[m.start..m.end], m.value, "byte 偏移应能切出原值");
}

#[test]
fn redact_one_email_remove() {
    let r = redact_one(
        "user@example.com",
        PiiKind::Email,
        RedactionStrategy::Remove,
    );
    assert_eq!(r, "[REDACTED]");
    assert!(!r.contains("user"));
    assert!(!r.contains("example.com"));
}

#[test]
fn redact_one_email_mask() {
    let r = redact_one("user@example.com", PiiKind::Email, RedactionStrategy::Mask);
    assert!(r.starts_with('u'), "Mask 应保留首字符 u: {r}");
    assert!(r.ends_with('m'), "Mask 应保留末字符 m: {r}");
    assert!(r.contains('*'), "Mask 应含 *: {r}");
    assert!(!r.contains("user"));
    assert!(!r.contains("example.com"));
}

#[test]
fn redact_one_email_hash() {
    let r = redact_one("user@example.com", PiiKind::Email, RedactionStrategy::Hash);
    assert!(r.len() >= 16, "Hash 应至少 16 chars: {r}");
    let hex_part = if r.contains(':') {
        r.split(':').nth(1).unwrap_or("")
    } else {
        r.as_str()
    };
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash 应含 hex digits: {r}"
    );
}

#[test]
fn redact_one_email_replace_label() {
    let r = redact_one(
        "user@example.com",
        PiiKind::Email,
        RedactionStrategy::ReplaceLabel,
    );
    assert_eq!(r, "[EMAIL]", "ReplaceLabel 应返大写 [EMAIL]");
}

#[test]
fn redact_one_phone_remove() {
    let r = redact_one("555-123-4567", PiiKind::Phone, RedactionStrategy::Remove);
    assert_eq!(r, "[REDACTED]");
}

#[test]
fn redact_one_high_entropy_token_remove() {
    let r = redact_one(
        "FAKE-type1-1234567890abcdefxyz",
        PiiKind::SecretToken,
        RedactionStrategy::Remove,
    );
    assert_eq!(r, "[REDACTED]");
    assert!(!r.contains("FAKE"));
}

#[test]
fn redact_one_env_assignment_remove() {
    let r = redact_one(
        "FAKE-VALUE-TO-REMOVE-00",
        PiiKind::EnvSecret,
        RedactionStrategy::Remove,
    );
    assert_eq!(r, "[REDACTED]");
}

#[test]
fn redact_text_replaces_all_matches() {
    let text = "contact a@b.com or 555-123-4567";
    let matches = detect_pii(text);
    let redacted = redact_text(text, &matches, RedactionStrategy::Remove);
    assert!(!redacted.contains("a@b.com"));
    assert!(!redacted.contains("555-123-4567"));
}

#[test]
fn redact_text_preserves_unmatched_text() {
    let text = "hello world a@b.com";
    let matches = detect_pii(text);
    let redacted = redact_text(text, &matches, RedactionStrategy::Remove);
    assert!(redacted.contains("hello world"));
}

#[test]
fn redact_text_no_matches_returns_unchanged() {
    let text = "no pii here";
    let redacted = redact_text(text, &[], RedactionStrategy::Remove);
    assert_eq!(redacted, text);
}

#[test]
fn redact_text_handles_overlapping_pii_gracefully() {
    let text = "see https://user:pass@a.com/x";
    let matches = detect_pii(text);
    let redacted = redact_text(text, &matches, RedactionStrategy::Remove);
    assert!(matches.len() >= 1);
    assert!(!redacted.contains("user:pass"));
}

#[test]
fn pii_kind_all_returns_8() {
    assert_eq!(PiiKind::ALL.len(), 8, "PII 8 类");
}

#[test]
fn pii_kind_as_str_8_distinct() {
    let strs: Vec<&str> = PiiKind::ALL.iter().map(|k| k.as_str()).collect();
    let unique: std::collections::HashSet<&str> = strs.iter().copied().collect();
    assert_eq!(unique.len(), 8, "8 个 PII 类型的 as_str 互不相同");
}

#[test]
fn pii_match_struct_has_required_fields() {
    let m = PiiMatch {
        kind: PiiKind::Email,
        value: "x@y.com".to_string(),
        start: 0,
        end: 7,
    };
    assert_eq!(m.kind, PiiKind::Email);
    assert_eq!(m.value, "x@y.com");
    assert_eq!(m.start, 0);
    assert_eq!(m.end, 7);
}

#[test]
fn integration_detect_and_redact_workflow() {
    let user_message = "Hi, my email is alice@example.com, SSN 123-45-6789, call 555-123-4567";
    let matches = detect_pii(user_message);
    assert!(matches.len() >= 3);
    let redacted = redact_text(user_message, &matches, RedactionStrategy::Remove);
    assert!(!redacted.contains("alice@example.com"));
    assert!(!redacted.contains("123-45-6789"));
    assert!(!redacted.contains("555-123-4567"));
    assert!(redacted.contains("Hi, my email is"));
    assert!(redacted.contains("[REDACTED]"));
}

#[test]
fn integration_pii_mask_for_dashboard() {
    let emails = ["alice@example.com", "bob@example.com", "x@y.com"];
    let mut redacted_all = String::new();
    for e in &emails {
        let m = detect_pii(e);
        let r = redact_text(e, &m, RedactionStrategy::Mask);
        redacted_all.push_str(&r);
        redacted_all.push('\n');
    }
    for line in redacted_all.lines() {
        if !line.is_empty() {
            assert!(line.chars().count() >= 2, "Mask 应保留首+末: {line}");
        }
    }
}
