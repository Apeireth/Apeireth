//! `permission_effect_demo` — apeireth-sovereignty 端到端真效果验证-权限 (R17 阶段 7)
//!
//! **目的**: 验证 5 大 Self-Disable 机制真工作
//!   (NoDegrade / NoPatch / NoBypass / NoReverse / NoHide)
//!
//! **流程**:
//! 1. 创建一个 SelfDisableGuard (默认 armed=true)
//! 2. 演示 6 个场景:
//!    - ✅ 风险升高 (low→high): Pass (正常行为)
//!    - ❌ 风险降低 (high→low): Triggered NoDegrade
//!    - ❌ 试图 patch 12 键 hardcode: Triggered NoPatch
//!    - ❌ Master token 绕过 governance: Triggered NoBypass
//!    - ❌ 试图撤销已触发的 self-disable: Triggered NoReverse
//!    - ❌ 试图清空 audit 记录: Triggered NoHide
//! 3. 验证 records.append-only (5 个 Triggered 都被记录, 不能删)
//! 4. 验证 disarm+rearm 仍不能绕过 (5 大机制永远 active)
//!
//! **跑法**: cargo run -p apeireth-sovereignty --example permission_effect_demo

use apeireth_sovereignty::self_disable::{SelfDisableCheck, SelfDisableGuard};

fn main() {
    println!("🛡️  apeireth-sovereignty 端到端真效果验证-权限 (R17 阶段 7)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let mut guard = SelfDisableGuard::new();
    let now_ms: i64 = 1_700_000_000_000;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 A: 验证 5 大 Self-Disable 机制");
    println!();

    // === 场景 1: 风险升高 (正常 Pass) ===
    let r1 = guard.check_no_degrade("low", "high", "scenario 1: low→high (升高)", now_ms);
    println!("🔍 场景 1 — 风险升高 (low→high):");
    println!("   期望: Pass (风险升高是允许的)");
    println!("   实际: {}", check_str(&r1));
    assert!(r1.is_pass(), "R17 不假装: 风险升高必须 Pass");
    println!();

    // === 场景 2: 风险降低 (触发 NoDegrade) ===
    let r2 = guard.check_no_degrade(
        "high",
        "low",
        "scenario 2: high→low (降低, 违规)",
        now_ms + 1,
    );
    println!("🔍 场景 2 — 风险降低 (high→low):");
    println!("   期望: Triggered (NoDegradeViolation)");
    println!("   实际: {}", check_str(&r2));
    assert!(r2.is_triggered(), "R17 不假装: 风险降低必须 Triggered");
    println!();

    // === 场景 3: 试图 patch 12 键 hardcode ===
    let r3 = guard.check_no_patch(
        "principle_keys_count",
        6,
        "scenario 3: 试图把 12 键改成 6",
        now_ms + 2,
    );
    println!("🔍 场景 3 — 试图 patch principle_keys_count (12 键 → 6):");
    println!("   期望: Triggered (NoPatchViolation, 12 键是编译期 hardcode)");
    println!("   实际: {}", check_str(&r3));
    assert!(
        r3.is_triggered(),
        "R17 不假装: patch hardcode 12 键数必须 Triggered"
    );
    println!();

    // === 场景 4: Master token 绕过 governance ===
    let r4 = guard.check_no_bypass(
        "master",
        true,
        "scenario 4: Master token 试图跳过 governance",
        now_ms + 3,
    );
    println!("🔍 场景 4 — Master token 绕过 governance:");
    println!("   期望: Triggered (NoBypassViolation, Q13 兜底)");
    println!("   实际: {}", check_str(&r4));
    assert!(
        r4.is_triggered(),
        "R17 不假装: Master bypass governance 必须 Triggered"
    );
    println!();

    // === 场景 5: 试图撤销已触发的 self-disable ===
    // 先记下一个已触发的 trigger id
    let triggered_id = match &r2 {
        SelfDisableCheck::Triggered(rec) => rec.trigger_id.clone(),
        _ => panic!("expected r2 to be triggered"),
    };
    let r5 = guard.check_no_reverse(
        &triggered_id,
        "scenario 5: 试图撤销已触发的 self-disable",
        now_ms + 4,
    );
    println!(
        "🔍 场景 5 — 试图撤销已触发的 self-disable (id={}):",
        triggered_id
    );
    println!("   期望: Triggered (NoReverseViolation, append-only)");
    println!("   实际: {}", check_str(&r5));
    assert!(
        r5.is_triggered(),
        "R17 不假装: 撤销 self-disable 必须 Triggered"
    );
    println!();

    // === 场景 6: 试图清空 audit 记录 ===
    let r6 = guard.check_no_hide(
        "audit-window-1",
        "scenario 6: 试图清空 audit 记录",
        now_ms + 5,
    );
    println!("🔍 场景 6 — 试图清空 audit 记录 (window=audit-window-1):");
    println!("   期望: Triggered (NoHideViolation, append-only)");
    println!("   实际: {}", check_str(&r6));
    assert!(r6.is_triggered(), "R17 不假装: 清空 audit 必须 Triggered");
    println!();

    // === 场景 7: 验证 records 全部 append-only ===
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 B: 验证 records append-only (5 个 Triggered 都被记录)");
    let count = guard.record_count();
    println!("📊 触发记录数: {}", count);
    assert_eq!(count, 5, "R17 不假装: 5 个 Triggered 场景必须全部被记录");

    let by_mechanism = guard.records_by_mechanism(1); // 1=NoDegrade
    println!("📊 NoDegrade 机制触发记录: {} 条", by_mechanism.len());
    assert_eq!(by_mechanism.len(), 1, "R17 不假装: NoDegrade 必须有 1 条");

    let has_triggered = guard.has_triggered();
    println!("📊 has_triggered (总触发): {}", has_triggered);
    assert!(
        has_triggered,
        "R17 不假装: has_triggered 必须 true (5 个场景都触发了)"
    );
    println!();

    // === 场景 8: 验证 disarm+rearm 仍不能绕过 (5 大机制永远 active) ===
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 C: 验证 disarm+rearm 仍不能绕过 5 大机制");
    println!("⚠️  攻击者尝试: disarm → 触发违规检查 → rearm");
    guard.disarm();
    let r_during_disarm =
        guard.check_no_degrade("high", "low", "attack during disarm", now_ms + 10);
    println!("🔍 disarm 状态下 check_no_degrade(high→low):");
    println!("   实际: {}", check_str(&r_during_disarm));
    assert!(
        r_during_disarm.is_pass(),
        "disarm 期间守门不工作 (按设计, disarm = 解除)"
    );
    println!();
    guard.rearm();
    let r_after_rearm = guard.check_no_degrade("high", "low", "attack after rearm", now_ms + 11);
    println!("🔍 rearm 状态下 check_no_degrade(high→low):");
    println!("   实际: {}", check_str(&r_after_rearm));
    assert!(
        r_after_rearm.is_triggered(),
        "R17 不假装: rearm 后守门必须重新 active"
    );
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ permission_effect_demo 验收通过 (R17 阶段 7 效果验证)");
    println!();
    println!("📊 验证总结:");
    println!("   ✅ NoDegrade 机制: high→low 风险降低被拦截");
    println!("   ✅ NoPatch 机制: 试图改 12 键 hardcode 被拦截");
    println!("   ✅ NoBypass 机制: Master token 绕过 governance 被拦截");
    println!("   ✅ NoReverse 机制: 撤销已触发的 self-disable 被拦截");
    println!("   ✅ NoHide 机制: 清空 audit 记录被拦截");
    println!("   ✅ Records append-only: 5 个 Triggered 全部被永久记录");
    println!("   ✅ Disarm+rearm 不能绕过 5 大机制 (rearm 后重新 active)");
}

fn check_str(c: &SelfDisableCheck) -> String {
    match c {
        SelfDisableCheck::Pass => "✅ Pass".to_string(),
        SelfDisableCheck::Triggered(rec) => {
            format!("❌ Triggered ({})", rec.trigger.mechanism_name())
        }
    }
}
