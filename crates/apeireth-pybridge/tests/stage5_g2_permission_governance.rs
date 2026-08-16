//! R129-5 ASI Python 整合 Stage 5 治理 — G2 权限治理集成测试
//!
//! 任务: 验证 G2 权限治理 (6 重守门 v7 + 3 状态 + 4 Stage)
//! 借鉴: superpowers 234 + langgraph 829 + PyO3 928 (per decision-61 §3.1 R129-5)

use apeireth_pybridge::*;

// =============================================================================
// G2 基础架构 (4 测)
// =============================================================================

#[test]
fn g2_version_is_r129_stage5() {
    assert_eq!(permission_governance_version(), "0.1.0-R129-Stage5-G2");
}

#[test]
fn g2_layer_count_is_6_v7() {
    // 1:1 跟 B4 6 重 v7 严守
    assert_eq!(PERMISSION_GOVERNANCE_LAYER_COUNT, 6);
    assert_eq!(permission_governance_layer_count(), 6);
    assert_eq!(PermissionLayer::ALL.len(), 6);
}

#[test]
fn g2_stage_count_is_4() {
    assert_eq!(PERMISSION_GOVERNANCE_STAGE_COUNT, 4);
}

#[test]
fn g2_health_struct_ok() {
    let h = permission_governance_health();
    assert!(h.is_ok);
    assert_eq!(h.layer_count, 6);
    assert_eq!(h.stage_count, 4);
}

// =============================================================================
// G2 6 重守门 (8 测)
// =============================================================================

#[test]
fn g2_l1_type_check() {
    assert_eq!(PermissionLayer::L1TypeCheck.number(), 1);
    assert_eq!(PermissionLayer::L1TypeCheck.name(), "L1_type_check");
}

#[test]
fn g2_l2_scope_check() {
    assert_eq!(PermissionLayer::L2ScopeCheck.number(), 2);
    assert_eq!(PermissionLayer::L2ScopeCheck.name(), "L2_scope_check");
}

#[test]
fn g2_l3_rate_check() {
    assert_eq!(PermissionLayer::L3RateCheck.number(), 3);
    assert_eq!(PermissionLayer::L3RateCheck.name(), "L3_rate_check");
}

#[test]
fn g2_l4_guard_check() {
    assert_eq!(PermissionLayer::L4GuardCheck.number(), 4);
    assert_eq!(PermissionLayer::L4GuardCheck.name(), "L4_guard_check");
}

#[test]
fn g2_l5_audit_check() {
    assert_eq!(PermissionLayer::L5AuditCheck.number(), 5);
    assert_eq!(PermissionLayer::L5AuditCheck.name(), "L5_audit_check");
}

#[test]
fn g2_l6_provenance_check() {
    assert_eq!(PermissionLayer::L6ProvenanceCheck.number(), 6);
    assert_eq!(
        PermissionLayer::L6ProvenanceCheck.name(),
        "L6_provenance_check"
    );
}

#[test]
fn g2_6_layers_in_order() {
    let all = PermissionLayer::ALL;
    assert_eq!(all[0], PermissionLayer::L1TypeCheck);
    assert_eq!(all[1], PermissionLayer::L2ScopeCheck);
    assert_eq!(all[2], PermissionLayer::L3RateCheck);
    assert_eq!(all[3], PermissionLayer::L4GuardCheck);
    assert_eq!(all[4], PermissionLayer::L5AuditCheck);
    assert_eq!(all[5], PermissionLayer::L6ProvenanceCheck);
}

#[test]
fn g2_six_fold_v7_gate_verified() {
    // 1:1 跟 B4 6 重 v7 严守
    assert_eq!(permission_governance_layer_count(), 6);
}

// =============================================================================
// G2 3 状态 (4 测)
// =============================================================================

#[test]
fn g2_decision_allow() {
    assert_eq!(PermissionDecision::Allow.name(), "allow");
}

#[test]
fn g2_decision_deny() {
    assert_eq!(PermissionDecision::Deny.name(), "deny");
}

#[test]
fn g2_decision_audit_required() {
    assert_eq!(PermissionDecision::AuditRequired.name(), "audit_required");
}

#[test]
fn g2_3_states_aggregated() {
    let mut e = PermissionEngine::new();
    // 1 个 safe context → 全 Allow
    e.check(PermissionContext::safe_default());
    // 1 个 invalid → 全 Deny
    e.check(PermissionContext {
        module_id: 99,
        ..PermissionContext::safe_default()
    });
    // 1 个 high resource → AuditRequired
    e.check(PermissionContext {
        resource_used: 85,
        ..PermissionContext::safe_default()
    });
    // 总 18 events, deny + audit 都有
    assert_eq!(e.report.total(), 18);
    assert!(e.report.deny_count() > 0);
    assert!(e.report.audit_count() > 0);
}

// =============================================================================
// G2 PermissionContext (3 测)
// =============================================================================

#[test]
fn g2_context_safe_default() {
    let c = PermissionContext::safe_default();
    assert_eq!(c.asi_stage, 1);
    assert_eq!(c.module_id, 0);
    assert_eq!(c.resource_used, 0);
    assert!(!c.audit_required);
}

#[test]
fn g2_context_strict_default() {
    let c = PermissionContext::strict_default();
    assert_eq!(c.asi_stage, 4);
    assert_eq!(c.module_id, 4);
    assert_eq!(c.resource_used, 90);
    assert!(c.audit_required);
}

#[test]
fn g2_context_default_trait() {
    let c = PermissionContext::default();
    assert_eq!(c.asi_stage, 1);
}

// =============================================================================
// G2 PermissionEngine (10 测)
// =============================================================================

#[test]
fn g2_engine_safe_context_allows() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext::safe_default());
    assert_eq!(d, PermissionDecision::Allow);
}

#[test]
fn g2_engine_invalid_module_id_denies() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext {
        module_id: 99,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::Deny);
}

#[test]
fn g2_engine_invalid_asi_stage_denies() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext {
        asi_stage: 99,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::Deny);
}

#[test]
fn g2_engine_high_resource_audits() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext {
        resource_used: 85,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::AuditRequired);
}

#[test]
fn g2_engine_invalid_source_denies() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext {
        source_id: 99,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::Deny);
}

#[test]
fn g2_engine_audit_required_audits() {
    let mut e = PermissionEngine::new();
    let d = e.check(PermissionContext {
        audit_required: true,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::AuditRequired);
}

#[test]
fn g2_engine_stage4_strict_requires_audit() {
    let mut e = PermissionEngine::new().with_stage4_strict();
    let d = e.check(PermissionContext {
        asi_stage: 4,
        audit_required: false,
        ..PermissionContext::safe_default()
    });
    assert_eq!(d, PermissionDecision::AuditRequired);
}

#[test]
fn g2_engine_6_events_per_check() {
    let mut e = PermissionEngine::new();
    e.check(PermissionContext::safe_default());
    assert_eq!(e.report.total(), 6);
}

#[test]
fn g2_engine_layer_order_preserved() {
    let mut e = PermissionEngine::new();
    e.check(PermissionContext::safe_default());
    let events = &e.report.events;
    assert_eq!(events[0].layer, PermissionLayer::L1TypeCheck);
    assert_eq!(events[1].layer, PermissionLayer::L2ScopeCheck);
    assert_eq!(events[2].layer, PermissionLayer::L3RateCheck);
    assert_eq!(events[3].layer, PermissionLayer::L4GuardCheck);
    assert_eq!(events[4].layer, PermissionLayer::L5AuditCheck);
    assert_eq!(events[5].layer, PermissionLayer::L6ProvenanceCheck);
}

#[test]
fn g2_engine_clone_works() {
    let e1 = PermissionEngine::new();
    let e2 = e1.clone();
    assert_eq!(e1.report.total(), e2.report.total());
}

// =============================================================================
// G2 audit_all_stages (5 测)
// =============================================================================

#[test]
fn g2_audit_all_4_stages_24_events() {
    let mut e = PermissionEngine::new();
    e.audit_all_stages();
    assert_eq!(e.report.total(), 24); // 4 stages × 6 layers
}

#[test]
fn g2_audit_all_no_deny() {
    let mut e = PermissionEngine::new();
    e.audit_all_stages();
    // 4 stages default ctx 都合法 (asi_stage 1..4, module_id 0, source_id 0, resource 50)
    // 但 stage 4 audit_required=true → L5 audit
    assert_eq!(e.report.deny_count(), 0);
}

#[test]
fn g2_audit_all_stage4_has_audit() {
    let mut e = PermissionEngine::new();
    e.audit_all_stages();
    // Stage 4 audit_required=true → L5 AuditRequired (6 layer × 1 stage = 1 event)
    // 但 L3 (resource=50 < 80) = Allow
    // 所以 stage 4 = 5 Allow + 1 AuditRequired = 1 audit event
    assert!(e.report.audit_count() >= 1);
}

#[test]
fn g2_audit_all_count_by_layer() {
    let mut e = PermissionEngine::new();
    e.audit_all_stages();
    let by_layer = e.report.count_by_layer();
    for layer in PermissionLayer::ALL {
        assert_eq!(by_layer.get(&layer), Some(&4));
    }
}

#[test]
fn g2_audit_all_stages_1_2_3_allow() {
    // Stage 1-3 都 Allow, Stage 4 AuditRequired
    let r = permission_governance_summary();
    // 18 (3 stages × 6 layers Allow) + 5 (stage 4: 5 Allow) + 1 (stage 4: 1 Audit) = 24
    assert_eq!(r.allow_count() + r.audit_count(), 24);
}

// =============================================================================
// G2 PermissionReport (3 测)
// =============================================================================

#[test]
fn g2_report_total() {
    let r = permission_governance_summary();
    assert_eq!(r.total(), 24);
}

#[test]
fn g2_report_is_all_allowed() {
    let r = permission_governance_summary();
    // Stage 4 audit → not all allowed
    assert!(!r.is_all_allowed());
}

#[test]
fn g2_report_display() {
    let r = permission_governance_summary();
    let s = format!("{r}");
    assert!(s.contains("G2 权限治理报告"));
    assert!(s.contains("24 events"));
    assert!(s.contains("L1_type_check"));
    assert!(s.contains("L6_provenance_check"));
}

// =============================================================================
// G2 健康度 (3 测)
// =============================================================================

#[test]
fn g2_health_display() {
    let h = permission_governance_health();
    let s = format!("{h}");
    assert!(s.contains("G2 权限治理"));
    assert!(s.contains("6 layers"));
    assert!(s.contains("6-fold v7"));
}

#[test]
fn g2_decision_event_new() {
    let e = PermissionDecisionEvent::new(
        PermissionLayer::L1TypeCheck,
        PermissionDecision::Allow,
        PermissionContext::safe_default(),
        "test",
    );
    assert_eq!(e.layer, PermissionLayer::L1TypeCheck);
    assert_eq!(e.decision, PermissionDecision::Allow);
    assert_eq!(e.reason, "test");
}

#[test]
fn g2_with_stage4_strict_returns_self() {
    let e = PermissionEngine::new();
    let e2 = e.with_stage4_strict();
    assert!(e2.stage4_strict);
}

// =============================================================================
// G2 端到端 (2 测)
// =============================================================================

#[test]
fn g2_e2e_6_layer_full_audit_for_stage1() {
    let mut e = PermissionEngine::new();
    // Stage 1 + V1077 (module_id=0) + resource 30 (low) + 0 audit
    let ctx = PermissionContext {
        asi_stage: 1,
        module_id: 0,
        resource_used: 30,
        audit_required: false,
        ..PermissionContext::safe_default()
    };
    let d = e.check(ctx);
    assert_eq!(d, PermissionDecision::Allow);
    assert_eq!(e.report.total(), 6);
}

#[test]
fn g2_e2e_6_layer_full_audit_for_stage4() {
    let mut e = PermissionEngine::new().with_stage4_strict();
    let ctx = PermissionContext {
        asi_stage: 4,
        module_id: 4, // V1458
        resource_used: 50,
        audit_required: true,
        ..PermissionContext::safe_default()
    };
    let d = e.check(ctx);
    // L5 = AuditRequired, 所以最终 = AuditRequired
    assert_eq!(d, PermissionDecision::AuditRequired);
}
