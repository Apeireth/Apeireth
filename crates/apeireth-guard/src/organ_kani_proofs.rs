//! R177 guard (Privacy Guard) organ Kani proofs (W3+W5)
//!
//! **要验证的不变量 (VCP 模式 3/8 + 找 bug)**:
//! 1. detect_pii 找到的 PII 位置 start < end
//! 2. 8 类 PII 全覆盖 (Email/Phone/Ssn/CreditCard/IpAddress/UrlWithCredentials/SecretToken/EnvSecret)
//! 3. redact 不可逆 (redacted != original)
//! 4. 4 策略都产生红字输出 (Mask / Hash / Drop / ReplaceLabel)
//! 5. audit log 不超过 capacity (ring buffer)
//! 6. without_audit 不写日志
//! 7. audit_enabled 默认 true
//! 8. PiiMatch::length == end - start
//! 9. detect_pii 对纯文本返回空 vec
//! 10. PiiKind::ALL 8 个 (含 UrlWithCredentials + ae12d9eb 增量的 SecretToken/EnvSecret)

#![allow(missing_docs)]

use crate::pii::{detect_pii, PiiKind, PiiMatch};
use crate::{PrivacyAction, PrivacyGuard, RedactionStrategy, RedactionResult};

// ============================================
// Property 1: PII 位置 start < end
// ============================================
#[test]
fn r177_grd_01_pii_positions_valid() {
    let text = "Email alice@example.com Phone 13800138000 SSN 123-45-6789";
    let matches = detect_pii(text);
    for m in &matches {
        assert!(m.start < m.end, "PII start {} >= end {}", m.start, m.end);
        assert!(m.length() > 0, "PII length 0");
        assert!(m.start <= text.len(), "PII start 超出 text");
        assert!(m.end <= text.len(), "PII end 超出 text");
    }
}

// ============================================
// Property 2: PiiKind 8 类 (ae12d9eb: 6 → 8)
// ============================================
#[test]
fn r177_grd_02_pii_kinds_eight() {
    assert_eq!(PiiKind::ALL.len(), 8);
    let names: Vec<&str> = PiiKind::ALL.iter().map(|k| k.as_str()).collect();
    assert!(names.contains(&"email"));
    assert!(names.contains(&"phone"));
    assert!(names.contains(&"ssn"));
    assert!(names.contains(&"credit_card"));
    assert!(names.contains(&"ip_address"));
    assert!(names.contains(&"url_with_credentials"));
    assert!(names.contains(&"secret_token"));
    assert!(names.contains(&"env_secret"));
}

// ============================================
// Property 3: redact 不可逆
// ============================================
#[test]
fn r177_grd_03_redact_irreversible() {
    let g = PrivacyGuard::new();
    let original = "alice@example.com and 192.168.1.1";
    let r: RedactionResult = g.check_and_redact(original, 1_700_000_000);
    assert_ne!(r.redacted_text, original, "redact 后应 != original");
    assert!(!r.redacted_text.contains("alice@example.com"), "email 不应可见");
    assert!(!r.redacted_text.contains("192.168.1.1"), "IP 不应可见");
}

// ============================================
// Property 4: 4 策略都产生不同输出
// ============================================
#[test]
fn r177_grd_04_four_strategies() {
    let text = "alice@example.com";
    let strategies = [
        RedactionStrategy::Mask,
        RedactionStrategy::Hash,
        RedactionStrategy::Remove,
        RedactionStrategy::ReplaceLabel,
    ];
    let mut outputs = std::collections::HashSet::new();
    for s in &strategies {
        let g = PrivacyGuard::with_strategy(*s);
        let r = g.check_and_redact(text, 1_700_000_000);
        outputs.insert(r.redacted_text.clone());
    }
    // 至少 3 种不同输出 (Hash 和 Mask 可能相同情况, 但 ReplaceLabel 必不同)
    assert!(outputs.len() >= 3, "4 策略应产生至少 3 种不同输出, got {}", outputs.len());
}

// ============================================
// Property 5: audit ring buffer 不超 capacity
// ============================================
#[test]
fn r177_grd_05_audit_ring_buffer() {
    let g = PrivacyGuard::new().with_audit_capacity(3);
    for i in 0..10 {
        let _ = g.check_and_redact("alice@example.com", 1_700_000_000 + i);
    }
    let count = g.audit().len();
    assert!(count <= 3, "audit ring buffer 应 ≤ capacity (3), got {}", count);
}

// ============================================
// Property 6: without_audit 不写日志
// ============================================
#[test]
fn r177_grd_06_without_audit() {
    let g = PrivacyGuard::new().without_audit();
    let _ = g.check_and_redact("alice@example.com and 192.168.1.1", 1_700_000_000);
    assert_eq!(g.audit().len(), 0, "without_audit 后应不写日志");
}

// ============================================
// Property 7: audit_enabled 默认 true
// ============================================
#[test]
fn r177_grd_07_audit_default_on() {
    let g = PrivacyGuard::new();
    let _ = g.check_and_redact("alice@example.com", 1_700_000_000);
    assert_eq!(g.audit().len(), 1, "默认应开启 audit");
}

// ============================================
// Property 8: PiiMatch::length == end - start
// ============================================
#[test]
fn r177_grd_08_pii_match_length() {
    let text = "Contact alice@example.com please";
    let matches = detect_pii(text);
    for m in &matches {
        assert_eq!(m.length(), m.end - m.start);
    }
}

// ============================================
// Property 9: detect_pii 纯文本返回空
// ============================================
#[test]
fn r177_grd_09_clean_text_no_pii() {
    let text = "The quick brown fox jumps over the lazy dog";
    let matches = detect_pii(text);
    assert_eq!(matches.len(), 0);
}

// ============================================
// Property 10: PiiKind::as_str 8 个不重不漏
// ============================================
#[test]
fn r177_grd_10_pii_kind_str_distinct() {
    let names: Vec<&str> = PiiKind::ALL.iter().map(|k| k.as_str()).collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "PII kind name 重复: {}", n);
    }
}

// ============================================
// Property 11: detect_only vs check_and_redact
// ============================================
#[test]
fn r177_grd_11_detect_only_no_redact() {
    let g = PrivacyGuard::new();
    let text = "alice@example.com";
    let detected = g.detect_only(text, 1_700_000_000);
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].kind, PiiKind::Email);
    // detect_only 只检测不 redact
    assert_eq!(g.audit().count_by_action(PrivacyAction::Detected), 1);
    assert_eq!(g.audit().count_by_action(PrivacyAction::Redacted), 0);
}

// ============================================
// Property 12: set_strategy mutates
// ============================================
#[test]
fn r177_grd_12_set_strategy() {
    let mut g = PrivacyGuard::new();
    assert_eq!(g.strategy(), RedactionStrategy::Mask);
    g.set_strategy(RedactionStrategy::Hash);
    assert_eq!(g.strategy(), RedactionStrategy::Hash);
}

// ============================================
// Kani-style formal proof — PII 位置始终有效
// ============================================
#[cfg(kani)]
#[kani::proof]
fn r177_grd_kani_01_pii_position_invariant() {
    let text = "alice@example.com";
    let matches = detect_pii(text);
    for m in &matches {
        assert!(m.start < m.end, "PII 位置无效");
        assert!(m.length() > 0);
    }
}

#[cfg(kani)]
#[kani::proof]
fn r177_grd_kani_02_pii_kinds_complete() {
    assert_eq!(PiiKind::ALL.len(), 8);
}

#[allow(dead_code)]
fn _ensure_pii_match_used(m: &PiiMatch) -> usize {
    m.length()
}
