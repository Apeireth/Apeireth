//! R177 tool-approval organ Kani proofs (W3)

#![allow(missing_docs)]

use crate::ApprovalDecision;

#[test]
fn r177_app_01_allow_helpers() {
    let a = ApprovalDecision::Allow;
    assert!(a.is_allow());
    assert!(!a.is_require_approval());
    assert!(!a.is_deny());
    assert!(!a.is_no_match());
    assert!(a.is_terminal());
}

#[test]
fn r177_app_02_require_approval_helpers() {
    let r = ApprovalDecision::RequireApproval { timeout_ms: 300_000 };
    assert!(r.is_require_approval());
    assert!(!r.is_allow());
    assert!(!r.is_deny());
    assert!(r.is_terminal());
}

#[test]
fn r177_app_03_deny_helpers() {
    let d = ApprovalDecision::Deny {
        reason: "test".into(),
        silent: false,
    };
    assert!(d.is_deny());
    assert!(!d.is_allow());
    assert!(d.is_terminal());
}

#[test]
fn r177_app_04_no_match_helpers() {
    let n = ApprovalDecision::NoMatch;
    assert!(n.is_no_match());
    assert!(!n.is_terminal());
    assert!(!n.is_allow());
    assert!(!n.is_deny());
    assert!(!n.is_require_approval());
}

#[test]
fn r177_app_05_decisions_mutually_exclusive() {
    let a = ApprovalDecision::Allow;
    let r = ApprovalDecision::RequireApproval { timeout_ms: 0 };
    let d = ApprovalDecision::Deny { reason: "x".into(), silent: true };
    let n = ApprovalDecision::NoMatch;

    assert_ne!(a, r);
    assert_ne!(a, d);
    assert_ne!(a, n);
    assert_ne!(r, d);
    assert_ne!(r, n);
    assert_ne!(d, n);
}

#[test]
fn r177_app_06_as_str_values() {
    assert_eq!(ApprovalDecision::Allow.as_str(), "allow");
    assert_eq!(
        ApprovalDecision::RequireApproval { timeout_ms: 0 }.as_str(),
        "require_approval"
    );
    assert_eq!(
        ApprovalDecision::Deny { reason: "x".into(), silent: false }.as_str(),
        "deny"
    );
    assert_eq!(
        ApprovalDecision::Deny { reason: "x".into(), silent: true }.as_str(),
        "deny_silent"
    );
    assert_eq!(ApprovalDecision::NoMatch.as_str(), "no_match");
}

#[test]
fn r177_app_07_5min_default_timeout() {
    let r = ApprovalDecision::RequireApproval { timeout_ms: 5 * 60 * 1000 };
    assert_eq!(r.as_str(), "require_approval");
    if let ApprovalDecision::RequireApproval { timeout_ms } = r {
        assert_eq!(timeout_ms, 300_000);
    } else {
        panic!("应为 RequireApproval");
    }
}

#[test]
fn r177_app_08_deny_silent_field() {
    let loud = ApprovalDecision::Deny { reason: "x".into(), silent: false };
    let silent = ApprovalDecision::Deny { reason: "x".into(), silent: true };
    assert_ne!(loud, silent);
    assert_eq!(loud.as_str(), "deny");
    assert_eq!(silent.as_str(), "deny_silent");
}

#[test]
fn r177_app_09_clone_eq() {
    let r1 = ApprovalDecision::RequireApproval { timeout_ms: 1000 };
    let r2 = r1.clone();
    assert_eq!(r1, r2);
}

#[test]
fn r177_app_10_terminal_count() {
    let decisions = vec![
        ApprovalDecision::Allow,
        ApprovalDecision::RequireApproval { timeout_ms: 0 },
        ApprovalDecision::Deny { reason: "x".into(), silent: false },
    ];
    let terminal_count = decisions.iter().filter(|d| d.is_terminal()).count();
    assert_eq!(terminal_count, 3);
}

#[cfg(kani)]
#[kani::proof]
fn r177_app_kani_01_helpers_consistent() {
    let a = ApprovalDecision::Allow;
    assert!(a.is_allow());
    assert!(a.is_terminal());
}

#[cfg(kani)]
#[kani::proof]
fn r177_app_kani_02_no_match_not_terminal() {
    let n = ApprovalDecision::NoMatch;
    assert!(n.is_no_match());
    assert!(!n.is_terminal());
}
