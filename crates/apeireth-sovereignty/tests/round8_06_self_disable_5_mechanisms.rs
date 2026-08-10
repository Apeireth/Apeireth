//! round8-06 integration tests — Self-Disable 5 大机制端到端
//!
//! 覆盖 5 大机制在真实决策链中的触发:
//! - No-Degrade: risk_level 被降低 (high → low) 触发
//! - No-Patch: 5 哲学键数被改 触发
//! - No-Bypass: Master token 走 governance 但 governance 被旁路 触发
//! - No-Reverse: 尝试撤销触发记录 触发
//! - No-Hide: 触发后 audit 被清空 触发
//!
//! **守 7 项不修改承诺**: 不修改任何 LOCKED 模块。

use apeireth_sovereignty::{
    SelfDisableCheck, SelfDisableGuard, SelfDisableSignal, SelfDisableTrigger,
};

const NOW: i64 = 1_700_000_000_000;

#[test]
fn integration_no_degrade_blocks_critical_to_low_silently() {
    let mut g = SelfDisableGuard::new();
    // 真实场景: 试图把 nuclear risk 静默降级到 low
    let result = g.check_no_degrade("nuclear", "low", "decision:critical-op", NOW);
    assert!(result.is_triggered());
    let records = g.records();
    assert_eq!(records.len(), 1);
    match &records[0].trigger {
        SelfDisableTrigger::NoDegradeViolation { from, to } => {
            assert_eq!(from, "nuclear");
            assert_eq!(to, "low");
        }
        _ => panic!("应为 NoDegradeViolation"),
    }
    assert_eq!(records[0].context, "decision:critical-op");
}

#[test]
fn integration_no_patch_blocks_philosophy_keys_change() {
    let mut g = SelfDisableGuard::new();
    // 真实场景: 运行时 patch principle_keys_count 从 5 → 3
    let result = g.check_no_patch("principle_keys_count", 3, "patch:proposal_gate", NOW);
    assert!(result.is_triggered());
    assert_eq!(g.records_by_mechanism(2).len(), 1);
}

#[test]
fn integration_no_bypass_blocks_master_with_bypass() {
    let mut g = SelfDisableGuard::new();
    // 真实场景: Master token 试图跳过 governance.process_owner_decision
    let result = g.check_no_bypass("Master", true, "request:owner-bypass-1", NOW);
    assert!(result.is_triggered());
    if let SelfDisableCheck::Triggered(rec) = result {
        assert_eq!(rec.trigger.mechanism_id(), 3);
        assert_eq!(rec.trigger.mechanism_name(), "no_bypass");
        assert_eq!(rec.trigger.chinese_name(), "不可绕过");
    }
}

#[test]
fn integration_no_reverse_blocks_revoke_attempts() {
    let mut g = SelfDisableGuard::new();
    // 先触发一次
    let _ = g.check_no_degrade("high", "low", "first", NOW);
    // 尝试撤销刚才的触发 (clone trigger_id 以避免借用冲突)
    let trigger_id = g.records()[0].trigger_id.clone();
    let revoke_attempt = g.check_no_reverse(&trigger_id, "attempt:undo-1", NOW + 1000);
    assert!(revoke_attempt.is_triggered());
    // 现在有 2 个触发: 原 NoDegrade + 新 NoReverse
    assert_eq!(g.record_count(), 2);
}

#[test]
fn integration_no_hide_blocks_audit_clear() {
    let mut g = SelfDisableGuard::new();
    let result = g.check_no_hide("audit-window-2026-08-01", "session:42", NOW);
    assert!(result.is_triggered());
    assert_eq!(g.records_by_mechanism(5).len(), 1);
    assert!(g.has_triggered());
}

#[test]
fn integration_full_check_routes_all_5_signals() {
    let mut g = SelfDisableGuard::new();
    // 测试 5 个信号路由
    let signals = [
        SelfDisableSignal::NoDegrade {
            original: "high".into(),
            proposed: "low".into(),
            context: "ctx-1".into(),
        },
        SelfDisableSignal::NoPatch {
            rule: "permission_layers_count".into(),
            value: 7,
            context: "ctx-2".into(),
        },
        SelfDisableSignal::NoBypass {
            owner_token: "Master".into(),
            bypassed_governance: true,
            context: "ctx-3".into(),
        },
        SelfDisableSignal::NoReverse {
            trigger_id: "sd-000001".into(),
            context: "ctx-4".into(),
        },
        SelfDisableSignal::NoHide {
            window_id: "window-1".into(),
            context: "ctx-5".into(),
        },
    ];
    let mut triggered_count = 0;
    for (i, signal) in signals.iter().enumerate() {
        let result = g.full_check(signal, NOW + i as i64);
        if result.is_triggered() {
            triggered_count += 1;
        }
    }
    assert_eq!(triggered_count, 5, "5 个信号全部应触发");
    assert_eq!(g.record_count(), 5);
    assert_eq!(g.records_by_mechanism(1).len(), 1);
    assert_eq!(g.records_by_mechanism(2).len(), 1);
    assert_eq!(g.records_by_mechanism(3).len(), 1);
    assert_eq!(g.records_by_mechanism(4).len(), 1);
    assert_eq!(g.records_by_mechanism(5).len(), 1);
}

#[test]
fn integration_5_mechanisms_in_order_realistic_flow() {
    // 真实场景: 决策链路触发多个机制
    let mut g = SelfDisableGuard::new();

    // Step 1: AI 试图把 high 降级到 low → No-Degrade
    let r1 = g.check_no_degrade("high", "low", "step1:risk_degrade", NOW);
    assert!(r1.is_triggered());

    // Step 2: 检测到 Master 试图绕过 governance → No-Bypass
    let r2 = g.check_no_bypass("Master", true, "step2:owner_bypass", NOW + 100);
    assert!(r2.is_triggered());

    // Step 3: 试图撤销 step1 的触发 → No-Reverse
    let step1_id = g.records()[0].trigger_id.clone();
    let r3 = g.check_no_reverse(&step1_id, "step3:undo_step1", NOW + 200);
    assert!(r3.is_triggered());

    // Step 4: 触发后 audit 被清空 → No-Hide
    let r4 = g.check_no_hide("audit-window-001", "step4:hide_audit", NOW + 300);
    assert!(r4.is_triggered());

    // 验证最终状态: 4 个触发记录
    assert_eq!(g.record_count(), 4);
    assert!(g.has_triggered());
}
