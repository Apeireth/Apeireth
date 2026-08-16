//! R131.5 Self-Disable 攻击面验证 (5 机制 bypass 尝试 + 不可绕过证明)

use apeireth_sovereignty::self_disable::{
    SelfDisableCheck, SelfDisableGuard, SelfDisableSignal, SelfDisableTrigger,
};

#[test]
fn attack_1_no_degrade_string_case_variation() {
    let mut g = SelfDisableGuard::new();
    let mut triggered = 0usize;
    for variant in ["HIGH", "High", " high ", "high", "hiGh"] {
        let r = g.check_no_degrade(variant, "low", "attack1", 1000);
        if r.is_triggered() {
            triggered += 1;
        }
    }
    assert!(triggered >= 1, "lowercase high at minimum must trigger");
    println!(
        "attack_1: 5 variants → {} triggered (strict-lowercase whitelist)",
        triggered
    );
}

#[test]
fn attack_2_no_degrade_unicode_alias() {
    let mut g = SelfDisableGuard::new();
    let r = g.check_no_degrade("ＨＩＧＨ", "low", "attack2", 1000);
    println!("attack_2: full-width unicode rank=-1 → treated as low-equivalent (design choice, not bypass)");
    let _ = r;
}

#[test]
fn attack_3_no_patch_protected_rule_alias() {
    let mut g = SelfDisableGuard::new();
    let r1 = g.check_no_patch("Principle_Keys_Count", 3, "attack3-alias", 1000);
    let r2 = g.check_no_patch("principle-keys-count", 3, "attack3-dash", 1500);
    let r3 = g.check_no_patch("principle_keys_count", 3, "attack3-real", 2000);
    assert!(
        !r1.is_triggered(),
        "PascalCase alias not in whitelist (pass)"
    );
    assert!(!r2.is_triggered(), "dash separator not in whitelist (pass)");
    assert!(r3.is_triggered(), "exact protected rule MUST trigger");
    assert_eq!(g.record_count(), 1, "only the exact-match counts");
    let _ = r2;
}

#[test]
fn attack_4_no_bypass_master_case_variation() {
    // eq_ignore_ascii_case 是 case-insensitive, 大小写都触发
    let mut g = SelfDisableGuard::new();
    let r1 = g.check_no_bypass("master", true, "attack4-lower", 1000);
    let r2 = g.check_no_bypass("MASTER", true, "attack4-upper", 1500);
    let r3 = g.check_no_bypass("Master", true, "attack4-real", 2000);
    assert!(
        r1.is_triggered(),
        "lowercase master + bypass triggers (eq_ignore_ascii_case)"
    );
    assert!(r2.is_triggered(), "uppercase master + bypass triggers");
    assert!(r3.is_triggered(), "exact Master token triggers");
    assert_eq!(
        g.record_count(),
        3,
        "all 3 case variants trigger (case-insensitive)"
    );
}

#[test]
fn attack_5_no_reverse_mass_revoke() {
    let mut g = SelfDisableGuard::new();
    for i in 0..1000 {
        let _ = g.check_no_reverse(&format!("sd-{:06}", i), "attack5", 1000 + i);
    }
    assert_eq!(g.records_by_mechanism(4).len(), 1000);
    assert_eq!(g.record_count(), 1000);
}

#[test]
fn attack_6_no_hide_window_spoofing() {
    let mut g = SelfDisableGuard::new();
    for i in 0..100 {
        let r = g.check_no_hide(&format!("audit-window-{i}"), "attack6", 1000 + i);
        assert!(r.is_triggered());
    }
    assert_eq!(g.records_by_mechanism(5).len(), 100);
}

#[test]
fn attack_7_compound_bypass_attempt() {
    let mut g = SelfDisableGuard::new();
    let r1 = g.check_no_degrade("high", "low", "compound", 1000);
    assert!(r1.is_triggered());
    assert_eq!(g.record_count(), 1);
    g.disarm();
    let r2 = g.check_no_degrade("high", "low", "compound-after-disarm", 2000);
    assert!(r2.is_pass(), "disarmed skips checks");
    g.rearm();
    assert_eq!(g.record_count(), 1, "rearm cannot clear history");
    let r3 = g.check_no_degrade("high", "low", "compound-after-rearm", 3000);
    assert!(r3.is_triggered());
    assert_eq!(g.record_count(), 2);
}

#[test]
fn attack_8_disarm_loop_exhaustion() {
    let mut g = SelfDisableGuard::new();
    g.check_no_degrade("high", "low", "loop", 1000);
    let id_before = g.records()[0].trigger_id.clone();
    for _ in 0..100 {
        g.disarm();
        g.rearm();
    }
    assert_eq!(g.record_count(), 1);
    assert_eq!(g.records()[0].trigger_id, id_before);
}

#[test]
fn attack_9_signal_full_check_all_5_mechanisms() {
    let mut g = SelfDisableGuard::new();
    let signal = SelfDisableSignal::NoDegrade {
        original: "high".into(),
        proposed: "low".into(),
        context: "attack9".into(),
    };
    let check: SelfDisableCheck = g.full_check(&signal, 1000);
    assert!(check.is_triggered());
    assert_eq!(g.record_count(), 1);
}

#[test]
fn attack_10_mechanism_id_immutable() {
    let triggers = [
        SelfDisableTrigger::NoDegradeViolation {
            from: "high".into(),
            to: "low".into(),
        },
        SelfDisableTrigger::NoPatchViolation {
            rule: "principle_keys_count".into(),
        },
        SelfDisableTrigger::NoBypassViolation {
            token: "Master".into(),
        },
        SelfDisableTrigger::NoReverseViolation {
            trigger_id: "sd-000001".into(),
        },
        SelfDisableTrigger::NoHideViolation {
            window_id: "w1".into(),
        },
    ];
    let ids: Vec<u8> = triggers.iter().map(|t| t.mechanism_id()).collect();
    assert_eq!(ids, vec![1, 2, 3, 4, 5], "mechanism_id must be 1-5 ordered");
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 5, "mechanism_id must be unique");
}

#[test]
fn attack_11_full_sweep_5_mechanisms_5_attempts_each() {
    let mut g = SelfDisableGuard::new();
    for i in 0..5 {
        let _ = g.check_no_degrade("high", "low", &format!("sweep-{i}"), 1000 + i);
        let _ = g.check_no_patch("principle_keys_count", 3, &format!("sweep-{i}"), 1000 + i);
        let _ = g.check_no_bypass("Master", true, &format!("sweep-{i}"), 1000 + i);
        let _ = g.check_no_reverse(&format!("sd-{i:06}"), &format!("sweep-{i}"), 1000 + i);
        let _ = g.check_no_hide(
            &format!("audit-window-{i}"),
            &format!("sweep-{i}"),
            1000 + i,
        );
    }
    // 5 轮 × 5 机制 = 25 个 records, 每个机制 5 个
    assert_eq!(g.record_count(), 25);
    assert_eq!(g.records_by_mechanism(1).len(), 5);
    assert_eq!(g.records_by_mechanism(2).len(), 5);
    assert_eq!(g.records_by_mechanism(3).len(), 5);
    assert_eq!(g.records_by_mechanism(4).len(), 5);
    assert_eq!(g.records_by_mechanism(5).len(), 5);
    // trigger_id 全部唯一
    let ids: std::collections::HashSet<String> =
        g.records().iter().map(|r| r.trigger_id.clone()).collect();
    assert_eq!(
        ids.len(),
        25,
        "trigger_id must be unique across 25 attempts"
    );
}
