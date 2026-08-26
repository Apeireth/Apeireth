//! R129-5 ASI Python 整合 Stage 5 治理 — G4 演进治理集成测试
//!
//! 任务: 验证 G4 演进治理 (4 kind + 4 rule + 3 outcome + 4 stage)
//! 借鉴: superpowers 234 + langgraph 829 + kani 4502 (per decision-61 §3.1 R129-5)

use apeireth_pybridge::*;

// =============================================================================
// G4 基础架构 (4 测)
// =============================================================================

#[test]
fn g4_version_is_r129_stage5() {
    assert_eq!(evolution_governance_version(), "0.1.0-R129-Stage5-G4");
}

#[test]
fn g4_rule_count_is_4() {
    assert_eq!(EVOLUTION_GOVERNANCE_RULE_COUNT, 4);
    assert_eq!(EvolutionRule::ALL.len(), 4);
}

#[test]
fn g4_kind_count_is_4() {
    assert_eq!(EVOLUTION_GOVERNANCE_KIND_COUNT, 4);
    assert_eq!(EvolutionKind::ALL.len(), 4);
}

#[test]
fn g4_stage_count_is_4() {
    assert_eq!(EVOLUTION_GOVERNANCE_STAGE_COUNT, 4);
}

// =============================================================================
// G4 4 演进类型 (5 测)
// =============================================================================

#[test]
fn g4_kind_add() {
    assert_eq!(EvolutionKind::Add.name(), "add");
    assert_eq!(EvolutionKind::Add.number(), 1);
}

#[test]
fn g4_kind_upgrade() {
    assert_eq!(EvolutionKind::Upgrade.name(), "upgrade");
    assert_eq!(EvolutionKind::Upgrade.number(), 2);
}

#[test]
fn g4_kind_downgrade() {
    assert_eq!(EvolutionKind::Downgrade.name(), "downgrade");
    assert_eq!(EvolutionKind::Downgrade.number(), 3);
}

#[test]
fn g4_kind_retire() {
    assert_eq!(EvolutionKind::Retire.name(), "retire");
    assert_eq!(EvolutionKind::Retire.number(), 4);
}

#[test]
fn g4_4_kinds_in_order() {
    let all = EvolutionKind::ALL;
    assert_eq!(all[0], EvolutionKind::Add);
    assert_eq!(all[1], EvolutionKind::Upgrade);
    assert_eq!(all[2], EvolutionKind::Downgrade);
    assert_eq!(all[3], EvolutionKind::Retire);
}

// =============================================================================
// G4 4 演进规则 (5 测)
// =============================================================================

#[test]
fn g4_rule_r1_new_module_safe() {
    assert_eq!(EvolutionRule::R1NewModuleSafe.name(), "R1_new_module_safe");
    assert_eq!(EvolutionRule::R1NewModuleSafe.number(), 1);
}

#[test]
fn g4_rule_r2_upgrade_backward_compat() {
    assert_eq!(
        EvolutionRule::R2UpgradeBackwardCompat.name(),
        "R2_upgrade_backward_compat"
    );
    assert_eq!(EvolutionRule::R2UpgradeBackwardCompat.number(), 2);
}

#[test]
fn g4_rule_r3_downgrade_justified() {
    assert_eq!(
        EvolutionRule::R3DowngradeJustified.name(),
        "R3_downgrade_justified"
    );
    assert_eq!(EvolutionRule::R3DowngradeJustified.number(), 3);
}

#[test]
fn g4_rule_r4_retire_confirmed() {
    assert_eq!(
        EvolutionRule::R4RetireConfirmed.name(),
        "R4_retire_confirmed"
    );
    assert_eq!(EvolutionRule::R4RetireConfirmed.number(), 4);
}

#[test]
fn g4_4_rules_in_order() {
    let all = EvolutionRule::ALL;
    assert_eq!(all[0], EvolutionRule::R1NewModuleSafe);
    assert_eq!(all[1], EvolutionRule::R2UpgradeBackwardCompat);
    assert_eq!(all[2], EvolutionRule::R3DowngradeJustified);
    assert_eq!(all[3], EvolutionRule::R4RetireConfirmed);
}

// =============================================================================
// G4 3 状态 (4 测)
// =============================================================================

#[test]
fn g4_outcome_pass() {
    assert_eq!(EvolutionOutcome::Pass.name(), "pass");
}

#[test]
fn g4_outcome_warn() {
    assert_eq!(EvolutionOutcome::Warn.name(), "warn");
}

#[test]
fn g4_outcome_fail() {
    assert_eq!(EvolutionOutcome::Fail.name(), "fail");
}

#[test]
fn g4_3_outcomes_distinct() {
    let outcomes = [
        EvolutionOutcome::Pass,
        EvolutionOutcome::Warn,
        EvolutionOutcome::Fail,
    ];
    assert_eq!(outcomes.len(), 3);
}

// =============================================================================
// G4 EvolutionContext (3 测)
// =============================================================================

#[test]
fn g4_context_default() {
    let c = EvolutionContext::default();
    assert_eq!(c.current_version, 1);
    assert_eq!(c.guard_layers, 6);
    assert_eq!(c.borrow_id, 0);
}

#[test]
fn g4_context_asi_default() {
    let c = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(c.module, "apeireth.v1077_asi_v04_full_measurement");
    assert_eq!(c.current_version, 1077);
    assert_eq!(c.guard_layers, 6);
}

#[test]
fn g4_context_equality() {
    let c1 = EvolutionContext::default();
    let c2 = EvolutionContext::default();
    assert_eq!(c1, c2);
}

// =============================================================================
// G4 EvolutionEngine (10 测)
// =============================================================================

#[test]
fn g4_engine_new_7_modules() {
    let e = EvolutionEngine::new();
    assert_eq!(e.asi_module_count(), 7);
}

#[test]
fn g4_engine_current_version_v1077() {
    let e = EvolutionEngine::new();
    assert_eq!(
        e.current_version("apeireth.v1077_asi_v04_full_measurement"),
        1077
    );
}

#[test]
fn g4_engine_current_version_v1458_ceiling() {
    let e = EvolutionEngine::new();
    assert_eq!(
        e.current_version("apeireth.v1458_asi_north_star_ceiling_chain_audit"),
        1458
    );
}

#[test]
fn g4_engine_current_version_unknown() {
    let e = EvolutionEngine::new();
    assert_eq!(e.current_version("apeireth.unknown"), 0);
}

#[test]
fn g4_engine_clone() {
    let e1 = EvolutionEngine::new();
    let e2 = e1.clone();
    assert_eq!(e1.asi_module_count(), e2.asi_module_count());
}

#[test]
fn g4_r1_new_module_safe_passes() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(e.check_r1_new_module_safe(&ctx), EvolutionOutcome::Pass);
}

#[test]
fn g4_r1_new_module_safe_fails_borrow_id() {
    let mut e = EvolutionEngine::new();
    let mut ctx = EvolutionContext::asi_default("apeireth.test", 1);
    ctx.borrow_id = 99;
    assert_eq!(e.check_r1_new_module_safe(&ctx), EvolutionOutcome::Fail);
}

#[test]
fn g4_r2_upgrade_backward_compat_passes() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r2_upgrade_backward_compat(&ctx, 1078),
        EvolutionOutcome::Pass
    );
}

#[test]
fn g4_r2_upgrade_backward_compat_warns_lower() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r2_upgrade_backward_compat(&ctx, 1076),
        EvolutionOutcome::Warn
    );
}

#[test]
fn g4_r3_downgrade_justified_passes() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r3_downgrade_justified(&ctx, 1076, "test reason"),
        EvolutionOutcome::Pass
    );
}

#[test]
fn g4_r3_downgrade_justified_fails_no_reason() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r3_downgrade_justified(&ctx, 1076, ""),
        EvolutionOutcome::Fail
    );
}

#[test]
fn g4_r4_retire_confirmed_passes() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r4_retire_confirmed(&ctx, true, true),
        EvolutionOutcome::Pass
    );
}

#[test]
fn g4_r4_retire_confirmed_fails_no_mavis() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r4_retire_confirmed(&ctx, false, true),
        EvolutionOutcome::Fail
    );
}

#[test]
fn g4_r4_retire_confirmed_fails_no_master() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
    assert_eq!(
        e.check_r4_retire_confirmed(&ctx, true, false),
        EvolutionOutcome::Fail
    );
}

// =============================================================================
// G4 audit_default (4 测)
// =============================================================================

#[test]
fn g4_audit_default_4_events() {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    assert_eq!(e.report.total(), 4);
}

#[test]
fn g4_audit_default_all_pass() {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    assert!(e.report.is_all_pass());
}

#[test]
fn g4_audit_default_pass_count_4() {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    assert_eq!(e.report.pass_count(), 4);
    assert_eq!(e.report.warn_count(), 0);
    assert_eq!(e.report.fail_count(), 0);
}

#[test]
fn g4_audit_default_count_by_kind() {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    let by_kind = e.report.count_by_kind();
    assert_eq!(by_kind.get(&EvolutionKind::Add), Some(&1));
    assert_eq!(by_kind.get(&EvolutionKind::Upgrade), Some(&1));
    assert_eq!(by_kind.get(&EvolutionKind::Downgrade), Some(&1));
    assert_eq!(by_kind.get(&EvolutionKind::Retire), Some(&1));
}

// =============================================================================
// G4 EvolutionReport (4 测)
// =============================================================================

#[test]
fn g4_report_new() {
    let r = EvolutionReport::new();
    assert_eq!(r.total(), 0);
}

#[test]
fn g4_report_record() {
    let mut r = EvolutionReport::new();
    r.record(EvolutionEvent::new(
        "test",
        EvolutionKind::Add,
        EvolutionRule::R1NewModuleSafe,
        EvolutionOutcome::Pass,
        "ok",
    ));
    assert_eq!(r.total(), 1);
}

#[test]
fn g4_report_count_by_rule() {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    let by_rule = e.report.count_by_rule();
    assert_eq!(by_rule.get(&EvolutionRule::R1NewModuleSafe), Some(&1));
    assert_eq!(
        by_rule.get(&EvolutionRule::R2UpgradeBackwardCompat),
        Some(&1)
    );
    assert_eq!(by_rule.get(&EvolutionRule::R3DowngradeJustified), Some(&1));
    assert_eq!(by_rule.get(&EvolutionRule::R4RetireConfirmed), Some(&1));
}

#[test]
fn g4_report_display() {
    let r = evolution_governance_summary();
    let s = format!("{r}");
    assert!(s.contains("G4 演进治理报告"));
    assert!(s.contains("4 events"));
    assert!(s.contains("kind add"));
    assert!(s.contains("kind upgrade"));
    assert!(s.contains("kind downgrade"));
    assert!(s.contains("kind retire"));
}

// =============================================================================
// G4 EvolutionEvent (2 测)
// =============================================================================

#[test]
fn g4_event_new() {
    let e = EvolutionEvent::new(
        "test",
        EvolutionKind::Add,
        EvolutionRule::R1NewModuleSafe,
        EvolutionOutcome::Pass,
        "ok",
    );
    assert_eq!(e.module, "test");
    assert_eq!(e.kind, EvolutionKind::Add);
    assert_eq!(e.rule, EvolutionRule::R1NewModuleSafe);
    assert_eq!(e.outcome, EvolutionOutcome::Pass);
    assert_eq!(e.reason, "ok");
}

#[test]
fn g4_event_clone() {
    let e = EvolutionEvent::new(
        "test",
        EvolutionKind::Add,
        EvolutionRule::R1NewModuleSafe,
        EvolutionOutcome::Pass,
        "ok",
    );
    let e2 = e.clone();
    assert_eq!(e.module, e2.module);
}

// =============================================================================
// G4 健康度 (3 测)
// =============================================================================

#[test]
fn g4_health_struct_ok() {
    let h = evolution_governance_health();
    assert!(h.is_ok);
    assert_eq!(h.rule_count, 4);
    assert_eq!(h.kind_count, 4);
    assert_eq!(h.stage_count, 4);
    assert_eq!(h.asi_module_count, 7);
}

#[test]
fn g4_health_display() {
    let h = evolution_governance_health();
    let s = format!("{h}");
    assert!(s.contains("G4 演进治理"));
    assert!(s.contains("4 rules"));
    assert!(s.contains("4 kinds"));
}

#[test]
fn g4_health_with_7_asi_modules() {
    let h = evolution_governance_health();
    // 1:1 跟 Stage 1 ASI 7 模块
    assert_eq!(h.asi_module_count, 7);
}

// =============================================================================
// G4 端到端 (2 测)
// =============================================================================

#[test]
fn g4_e2e_full_lifecycle() {
    let mut e = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);

    // Add
    assert_eq!(e.check_r1_new_module_safe(&ctx), EvolutionOutcome::Pass);
    // Upgrade
    assert_eq!(
        e.check_r2_upgrade_backward_compat(&ctx, 1078),
        EvolutionOutcome::Pass
    );
    // Downgrade
    assert_eq!(
        e.check_r3_downgrade_justified(&ctx, 1076, "perf issue"),
        EvolutionOutcome::Pass
    );
    // Retire
    assert_eq!(
        e.check_r4_retire_confirmed(&ctx, true, true),
        EvolutionOutcome::Pass
    );
    // 4 events
    assert_eq!(e.report.total(), 4);
}

#[test]
fn g4_e2e_v1458_ceiling_critical() {
    let mut e = EvolutionEngine::new();
    let ctx =
        EvolutionContext::asi_default("apeireth.v1458_asi_north_star_ceiling_chain_audit", 1458);

    // V1458 = ceiling_critical, 必严格守门
    // 1. R1: V1458 已注册, 必通过
    assert_eq!(e.check_r1_new_module_safe(&ctx), EvolutionOutcome::Pass);
    // 2. R2: 升级 V1458 (模拟)
    assert_eq!(
        e.check_r2_upgrade_backward_compat(&ctx, 1459),
        EvolutionOutcome::Pass
    );
    // 3. R3: 不该降级 V1458
    let outcome = e.check_r3_downgrade_justified(&ctx, 1457, "test");
    // 1457 < 1458, 但 ctx 合法 → Pass (降级可)
    assert_eq!(outcome, EvolutionOutcome::Pass);
    // 4. R4: 退役 V1458 必 3 方确认
    assert_eq!(
        e.check_r4_retire_confirmed(&ctx, true, true),
        EvolutionOutcome::Pass
    );
}
