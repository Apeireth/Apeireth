//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I3 D3+G3 记忆+形式化集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I3 D3+G3 记忆+形式化 跨 stage 集成
//! **目标**: 验证 D3 记忆自循环跟 G3 形式化治理跨 stage 集成

use apeireth_pybridge::{
    stage7_i3_healthy, stage7_i3_summary, stage7_i3_to_d3_consistency, stage7_i3_to_g3_consistency,
    MemoryFormalAuditEvent, MemoryFormalBinding, MemoryFormalCoordinator, MemoryFormalMatrix,
    MemoryFormalReport, MemoryKind, STAGE7_I3_BINDING_COUNT, STAGE7_I3_DIMENSION_COUNT,
    STAGE7_I3_HARNESS_COUNT, STAGE7_I3_MEMORY_KIND_COUNT, STAGE7_I3_VERSION,
};

// 1. I3 编译期常数
#[test]
fn i3_01_version_constants() {
    assert_eq!(STAGE7_I3_VERSION, "0.1.0-R129-Stage7-I3");
    assert_eq!(STAGE7_I3_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I3_HARNESS_COUNT, 8);
    assert_eq!(STAGE7_I3_MEMORY_KIND_COUNT, 7);
    assert_eq!(STAGE7_I3_BINDING_COUNT, 56);
}

// 2. I3 默认矩阵 7 × 8 = 56
#[test]
fn i3_02_matrix_size() {
    let m = MemoryFormalMatrix::default_matrix();
    assert_eq!(m.len(), 56);
}

// 3. I3 7 kinds 都有 binding
#[test]
fn i3_03_all_7_kinds() {
    let m = MemoryFormalMatrix::default_matrix();
    let kinds = [
        MemoryKind::ToolInvocation,
        MemoryKind::ToolReflection,
        MemoryKind::ReflectionStep,
        MemoryKind::DecisionMake,
        MemoryKind::DecisionRevisit,
        MemoryKind::ObservationRecord,
        MemoryKind::AuditCheckpoint,
    ];
    for k in &kinds {
        assert!(m.get(*k, 0).is_some(), "kind {k:?} 缺");
    }
}

// 4. I3 8 harnesses 都有 binding
#[test]
fn i3_04_all_8_harnesses() {
    let m = MemoryFormalMatrix::default_matrix();
    for h in 0..8u8 {
        assert!(
            m.get(MemoryKind::ToolInvocation, h).is_some(),
            "harness {h} 缺"
        );
    }
}

// 5. I3 get 缺 → None
#[test]
fn i3_05_get_missing() {
    let m = MemoryFormalMatrix::default_matrix();
    assert!(m.get(MemoryKind::ToolInvocation, 99).is_none());
}

// 6. I3 default binding 8 invariant 名
#[test]
fn i3_06_default_binding_invariants() {
    let expected = [
        "format_intact",
        "deterministic",
        "source_known",
        "version_locked",
        "result_valid",
        "no_oversize",
        "trace_linked",
        "audit_complete",
    ];
    for (h, inv) in expected.iter().enumerate() {
        let b = MemoryFormalBinding::default_binding(MemoryKind::ToolInvocation, h as u8);
        assert_eq!(b.invariant, *inv, "harness {h} invariant 不对");
    }
}

// 7. I3 协调器 verify_memory 全 pass
#[test]
fn i3_07_coordinator_verify_all_pass() {
    let mut c = MemoryFormalCoordinator::new();
    let passed = c.verify_memory(0, MemoryKind::ToolInvocation);
    assert_eq!(passed, 8);
    assert_eq!(c.report.passed_count(), 8);
    assert_eq!(c.report.failed_count(), 0);
}

// 8. I3 协调器 多个 kind
#[test]
fn i3_08_coordinator_multi_kinds() {
    let mut c = MemoryFormalCoordinator::new();
    let p1 = c.verify_memory(0, MemoryKind::ToolInvocation);
    let p2 = c.verify_memory(0, MemoryKind::DecisionMake);
    assert_eq!(p1, 8);
    assert_eq!(p2, 8);
    assert_eq!(c.report.event_count(), 16);
}

// 9. I3 AuditEvent 字段
#[test]
fn i3_09_audit_event_fields() {
    let e =
        MemoryFormalAuditEvent::new(123, MemoryKind::AuditCheckpoint, 7, "audit_complete", true);
    assert_eq!(e.timestamp, 123);
    assert!(e.passed);
}

// 10. I3 Report 默认
#[test]
fn i3_10_report_default() {
    let r = MemoryFormalReport::default();
    assert_eq!(r.event_count(), 0);
}

// 11. I3 公共 API summary
#[test]
fn i3_11_summary() {
    let s = stage7_i3_summary();
    assert!(s.contains("I3"));
    assert!(s.contains("D3"));
    assert!(s.contains("G3"));
}

// 12. I3 公共 API healthy
#[test]
fn i3_12_healthy() {
    assert!(stage7_i3_healthy());
}

// 13. I3 跟 D3 协同
#[test]
fn i3_13_to_d3() {
    assert!(stage7_i3_to_d3_consistency());
}

// 14. I3 跟 G3 协同
#[test]
fn i3_14_to_g3() {
    assert!(stage7_i3_to_g3_consistency());
}
