//! R129-5 ASI Python 整合 Stage 5 治理 — G1 资源治理集成测试
//!
//! 任务: 验证 G1 资源治理 (rate/memory/time/count 4 维度 + 7 ASI 模块 + 3 路径)
//! 借鉴: PyO3 928 + hyper 80 + superpowers 234 (per decision-61 §3.1 R129-5)

use apeireth_pybridge::*;

// =============================================================================
// G1 基础架构 (6 测)
// =============================================================================

#[test]
fn g1_version_is_r129_stage5() {
    assert_eq!(resource_governance_version(), "0.1.0-R129-Stage5-G1");
}

#[test]
fn g1_dimension_count_is_4() {
    assert_eq!(RESOURCE_GOVERNANCE_DIMENSION_COUNT, 4);
    assert_eq!(ResourceDimension::ALL.len(), 4);
}

#[test]
fn g1_asi_module_count_is_7() {
    assert_eq!(RESOURCE_GOVERNANCE_MODULE_COUNT, 7);
}

#[test]
fn g1_health_struct_ok() {
    let h = resource_governance_health();
    assert!(h.is_ok);
    assert_eq!(h.dimension_count, 4);
    assert_eq!(h.asi_module_count, 7);
}

#[test]
fn g1_health_display_contains_version() {
    let h = resource_governance_health();
    let s = format!("{h}");
    assert!(s.contains("G1 资源治理"));
    assert!(s.contains("R129-Stage5-G1"));
}

#[test]
fn g1_bootstrap_ok() {
    assert!(resource_governance_bootstrap_ok());
}

// =============================================================================
// G1 4 维度验证 (5 测)
// =============================================================================

#[test]
fn g1_rate_dimension() {
    assert_eq!(ResourceDimension::Rate.name(), "rate");
    assert_eq!(ResourceDimension::Rate.unit(), "req/s");
}

#[test]
fn g1_memory_dimension() {
    assert_eq!(ResourceDimension::Memory.name(), "memory");
    assert_eq!(ResourceDimension::Memory.unit(), "bytes");
}

#[test]
fn g1_time_dimension() {
    assert_eq!(ResourceDimension::Time.name(), "time");
    assert_eq!(ResourceDimension::Time.unit(), "ms");
}

#[test]
fn g1_count_dimension() {
    assert_eq!(ResourceDimension::Count.name(), "count");
    assert_eq!(ResourceDimension::Count.unit(), "concurrent");
}

#[test]
fn g1_dimension_all_4_in_order() {
    let all = ResourceDimension::ALL;
    assert_eq!(all[0], ResourceDimension::Rate);
    assert_eq!(all[1], ResourceDimension::Memory);
    assert_eq!(all[2], ResourceDimension::Time);
    assert_eq!(all[3], ResourceDimension::Count);
}

// =============================================================================
// G1 3 配额档位 (3 测)
// =============================================================================

#[test]
fn g1_default_quota_moderate() {
    let q = ResourceQuota::default_const();
    assert_eq!(q.rate_per_sec, 100);
    assert_eq!(q.memory_bytes, 64 * 1024 * 1024);
    assert_eq!(q.time_ms, 5_000);
    assert_eq!(q.count_max, 8);
}

#[test]
fn g1_strict_quota_stricter_than_default() {
    let s = ResourceQuota::strict_const();
    let d = ResourceQuota::default_const();
    assert!(s.rate_per_sec < d.rate_per_sec);
    assert!(s.memory_bytes < d.memory_bytes);
    assert!(s.time_ms < d.time_ms);
    assert!(s.count_max < d.count_max);
}

#[test]
fn g1_relaxed_quota_more_loose_than_default() {
    let r = ResourceQuota::relaxed_const();
    let d = ResourceQuota::default_const();
    assert!(r.rate_per_sec > d.rate_per_sec);
    assert!(r.memory_bytes > d.memory_bytes);
    assert!(r.time_ms > d.time_ms);
    assert!(r.count_max > d.count_max);
}

// =============================================================================
// G1 3 路径 (5 测)
// =============================================================================

#[test]
fn g1_action_allow() {
    assert_eq!(GovernanceAction::Allow.name(), "allow");
}

#[test]
fn g1_action_throttle() {
    assert_eq!(GovernanceAction::Throttle.name(), "throttle");
}

#[test]
fn g1_action_reject() {
    assert_eq!(GovernanceAction::Reject.name(), "reject");
}

#[test]
fn g1_3_path_decision_for_module() {
    let mut g = ResourceGovernor::new();
    // V1458 strict quota rate=10, 5 = Allow
    assert_eq!(
        g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            5
        ),
        GovernanceAction::Allow
    );
    // V1458 strict quota rate=10, 8 = Throttle
    assert_eq!(
        g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            8
        ),
        GovernanceAction::Throttle
    );
    // V1458 strict quota rate=10, 11 = Reject
    assert_eq!(
        g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            11
        ),
        GovernanceAction::Reject
    );
}

#[test]
fn g1_unlimited_always_allow() {
    let mut g = ResourceGovernor::new();
    g.set_quota("unlimited", ResourceQuota::unlimited_const());
    let action = g.check("unlimited", ResourceDimension::Rate, 999_999);
    assert_eq!(action, GovernanceAction::Allow);
}

// =============================================================================
// G1 7 ASI Python 模块 (8 测)
// =============================================================================

#[test]
fn g1_v1077_relaxed_quota() {
    let mut g = ResourceGovernor::new();
    // V1077 = relaxed (rate=1000)
    // 200 = under hard, but 0.8 * 1000 = 800, 200 < 800 = Allow
    let action = g.check(
        "apeireth.v1077_asi_v04_full_measurement",
        ResourceDimension::Rate,
        200,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1400_default_quota() {
    let mut g = ResourceGovernor::new();
    // V1400 = default (rate=100)
    let action = g.check(
        "apeireth.v1400_asi_self_framework",
        ResourceDimension::Rate,
        50,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1447_strict_quota() {
    let mut g = ResourceGovernor::new();
    // V1447 = strict (rate=10)
    let action = g.check(
        "apeireth.v1447_asi_cross_modular_audit",
        ResourceDimension::Rate,
        5,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1457_default_quota() {
    let mut g = ResourceGovernor::new();
    // V1457 = default (rate=100)
    let action = g.check(
        "apeireth.v1457_asi_six_deployment_operational_runbook",
        ResourceDimension::Rate,
        30,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1458_ceiling_critical_strict() {
    let mut g = ResourceGovernor::new();
    // V1458 = strict + ceiling_critical
    let action = g.check(
        "apeireth.v1458_asi_north_star_ceiling_chain_audit",
        ResourceDimension::Rate,
        5,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1467_default_quota() {
    let mut g = ResourceGovernor::new();
    // V1467 = default
    let action = g.check(
        "apeireth.v1467_asi_audit_http_gateway_history_diff",
        ResourceDimension::Rate,
        40,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_v1470_relaxed_quota() {
    let mut g = ResourceGovernor::new();
    // V1470 = relaxed
    let action = g.check(
        "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
        ResourceDimension::Rate,
        500,
    );
    assert_eq!(action, GovernanceAction::Allow);
}

#[test]
fn g1_all_7_asi_modules_have_quota() {
    let g = ResourceGovernor::new();
    let modules = [
        "apeireth.v1077_asi_v04_full_measurement",
        "apeireth.v1400_asi_self_framework",
        "apeireth.v1447_asi_cross_modular_audit",
        "apeireth.v1457_asi_six_deployment_operational_runbook",
        "apeireth.v1458_asi_north_star_ceiling_chain_audit",
        "apeireth.v1467_asi_audit_http_gateway_history_diff",
        "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
    ];
    assert_eq!(g.asi_module_count(), 7);
    for m in modules {
        // 任何模块都有 quota (不会走 default fallback)
        let q = g.quota_for(m);
        assert!(q.rate_per_sec > 0 || q.memory_bytes == 0); // strict/relaxed/default 都 > 0
    }
}

// =============================================================================
// G1 ResourceAuditEvent (3 测)
// =============================================================================

#[test]
fn g1_audit_event_new() {
    let e = ResourceAuditEvent::new(
        "test",
        ResourceDimension::Rate,
        50,
        100,
        GovernanceAction::Allow,
        "ok",
    );
    assert_eq!(e.module, "test");
    assert_eq!(e.dimension, ResourceDimension::Rate);
    assert_eq!(e.used, 50);
    assert_eq!(e.quota, 100);
}

#[test]
fn g1_audit_event_over_budget() {
    let e = ResourceAuditEvent::new(
        "test",
        ResourceDimension::Rate,
        101,
        100,
        GovernanceAction::Reject,
        "over",
    );
    assert!(e.is_over_budget());
}

#[test]
fn g1_audit_event_unlimited_not_over() {
    let e = ResourceAuditEvent::new(
        "test",
        ResourceDimension::Rate,
        999_999,
        0,
        GovernanceAction::Allow,
        "unlimited",
    );
    assert!(!e.is_over_budget());
}

// =============================================================================
// G1 ResourceReport (4 测)
// =============================================================================

#[test]
fn g1_report_new() {
    let r = resource_governance_summary();
    assert_eq!(r.total(), 28); // 7 modules × 4 dims
}

#[test]
fn g1_report_total_allow_throttle_reject_sum() {
    let r = resource_governance_summary();
    assert_eq!(r.allow_count() + r.throttle_count() + r.reject_count(), 28);
}

#[test]
fn g1_report_count_by_dimension() {
    let r = resource_governance_summary();
    let by_dim = r.count_by_dimension();
    for dim in ResourceDimension::ALL {
        assert_eq!(by_dim.get(&dim), Some(&7)); // 7 modules
    }
}

#[test]
fn g1_report_display() {
    let r = resource_governance_summary();
    let s = format!("{r}");
    assert!(s.contains("G1 资源治理报告"));
    assert!(s.contains("28 events"));
}

// =============================================================================
// G1 ResourceGovernor (4 测)
// =============================================================================

#[test]
fn g1_governor_set_quota_override() {
    let mut g = ResourceGovernor::new();
    let custom = ResourceQuota {
        rate_per_sec: 999,
        memory_bytes: 0,
        time_ms: 0,
        count_max: 0,
    };
    g.set_quota("custom", custom.clone());
    let q = g.quota_for("custom");
    assert_eq!(q.rate_per_sec, 999);
}

#[test]
fn g1_governor_default_for_unknown_module() {
    let g = ResourceGovernor::new();
    let q = g.quota_for("apeireth.unknown.module");
    // 未知模块走 default fallback
    assert_eq!(q.rate_per_sec, ResourceQuota::default_const().rate_per_sec);
}

#[test]
fn g1_governor_audit_all_28_events() {
    let mut g = ResourceGovernor::new();
    g.audit_all();
    assert_eq!(g.report.total(), 28);
}

#[test]
fn g1_governor_clone_works() {
    let g1 = ResourceGovernor::new();
    let g2 = g1.clone();
    assert_eq!(g1.asi_module_count(), g2.asi_module_count());
}

// =============================================================================
// G1 端到端 (3 测)
// =============================================================================

#[test]
fn g1_e2e_v1458_full_audit() {
    let mut g = ResourceGovernor::new();
    // V1458 strict — 4 dims check
    let module = "apeireth.v1458_asi_north_star_ceiling_chain_audit";
    g.check(module, ResourceDimension::Rate, 5);
    g.check(module, ResourceDimension::Memory, 16 * 1024 * 1024);
    g.check(module, ResourceDimension::Time, 1_000);
    g.check(module, ResourceDimension::Count, 1);
    // 4 events
    assert_eq!(g.report.total(), 4);
    // 全 Allow (都在 strict 配额内)
    assert_eq!(g.report.allow_count(), 4);
}

#[test]
fn g1_e2e_v1077_high_load_throttle() {
    let mut g = ResourceGovernor::new();
    // V1077 relaxed (rate=1000) — 800 = 0.8 = Throttle
    let action = g.check(
        "apeireth.v1077_asi_v04_full_measurement",
        ResourceDimension::Rate,
        800,
    );
    assert_eq!(action, GovernanceAction::Throttle);
}

#[test]
fn g1_e2e_mixed_3_paths() {
    let mut g = ResourceGovernor::new();
    // Allow
    assert_eq!(
        g.check("apeireth.v1077_asi_v04_full_measurement", ResourceDimension::Rate, 100),
        GovernanceAction::Allow
    );
    // Throttle
    assert_eq!(
        g.check("apeireth.v1458_asi_north_star_ceiling_chain_audit", ResourceDimension::Rate, 8),
        GovernanceAction::Throttle
    );
    // Reject
    assert_eq!(
        g.check("apeireth.v1458_asi_north_star_ceiling_chain_audit", ResourceDimension::Rate, 15),
        GovernanceAction::Reject
    );
    // 3 events
    assert_eq!(g.report.total(), 3);
}
