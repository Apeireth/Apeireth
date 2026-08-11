//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I1 D1+G1 工具+资源集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I1 D1+G1 工具+资源 跨 stage 集成
//! **目标**: 验证 D1 工具调用跟 G1 资源治理跨 stage 集成

use apeireth_pybridge::{
    stage7_i1_healthy, stage7_i1_summary, stage7_i1_to_d1_consistency,
    stage7_i1_to_g1_consistency, ToolResourceAuditEvent, ToolResourceBinding,
    ToolResourceCoordinator, ToolResourceMatrix, ToolResourceReport, GovernanceAction,
    ResourceDimension, STAGE7_I1_BINDING_COUNT, STAGE7_I1_DIMENSION_COUNT, STAGE7_I1_VERSION,
};

// 1. I1 编译期常数
#[test]
fn i1_01_version_constants() {
    assert_eq!(STAGE7_I1_VERSION, "0.1.0-R129-Stage7-I1");
    assert_eq!(STAGE7_I1_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I1_BINDING_COUNT, 20);
}

// 2. I1 默认矩阵 5 tool × 4 dim = 20
#[test]
fn i1_02_matrix_size() {
    let m = ToolResourceMatrix::default_matrix();
    assert_eq!(m.len(), 20);
    assert!(!m.is_empty());
}

// 3. I1 tool_ids() 5 个唯一 ID
#[test]
fn i1_03_tool_ids_unique() {
    let m = ToolResourceMatrix::default_matrix();
    let ids = m.tool_ids();
    assert_eq!(ids.len(), 5);
}

// 4. I1 查 get() 存在绑定
#[test]
fn i1_04_get_existing_binding() {
    let m = ToolResourceMatrix::default_matrix();
    let b = m.get("executor", ResourceDimension::Rate);
    assert!(b.is_some());
    assert_eq!(b.unwrap().quota_label, "default");
}

// 5. I1 查 get() 不存在 → None
#[test]
fn i1_05_get_missing_returns_none() {
    let m = ToolResourceMatrix::default_matrix();
    let b = m.get("nonexistent", ResourceDimension::Rate);
    assert!(b.is_none());
}

// 6. I1 4 dim 都查得到
#[test]
fn i1_06_all_4_dims() {
    let m = ToolResourceMatrix::default_matrix();
    for dim in &ResourceDimension::ALL {
        assert!(m.get("executor", *dim).is_some(), "dim {:?} 缺", dim);
    }
}

// 7. I1 Binding 字段对
#[test]
fn i1_07_binding_new() {
    let b = ToolResourceBinding::new("test", ResourceDimension::Memory, "strict");
    assert_eq!(b.tool_id, "test");
    assert_eq!(b.dimension, ResourceDimension::Memory);
    assert_eq!(b.quota_label, "strict");
}

// 8. I1 default_binding
#[test]
fn i1_08_default_binding() {
    let b = ToolResourceBinding::default_binding("test", ResourceDimension::Time);
    assert_eq!(b.quota_label, "default");
}

// 9. I1 协调器 default Allow
#[test]
fn i1_09_coordinator_allow() {
    let mut c = ToolResourceCoordinator::new();
    let a = c.check_and_call(0, "executor", ResourceDimension::Rate);
    assert_eq!(a, GovernanceAction::Allow);
    assert_eq!(c.report.event_count(), 1);
    assert_eq!(c.report.allow_count(), 1);
}

// 10. I1 协调器 unknown Reject
#[test]
fn i1_10_coordinator_reject() {
    let mut c = ToolResourceCoordinator::new();
    let a = c.check_and_call(0, "unknown", ResourceDimension::Rate);
    assert_eq!(a, GovernanceAction::Reject);
    assert_eq!(c.report.reject_count(), 1);
}

// 11. I1 多次调用累加
#[test]
fn i1_11_multi_calls_accumulate() {
    let mut c = ToolResourceCoordinator::new();
    for _ in 0..5 {
        c.check_and_call(0, "executor", ResourceDimension::Rate);
    }
    assert_eq!(c.report.event_count(), 5);
    assert_eq!(c.report.allow_count(), 5);
}

// 12. I1 AuditEvent 字段
#[test]
fn i1_12_audit_event_fields() {
    let e = ToolResourceAuditEvent::new(
        1234,
        "test",
        ResourceDimension::Count,
        GovernanceAction::Throttle,
        "over count",
    );
    assert_eq!(e.timestamp, 1234);
    assert_eq!(e.tool_id, "test");
    assert_eq!(e.action, GovernanceAction::Throttle);
}

// 13. I1 Report 默认
#[test]
fn i1_13_report_default() {
    let r = ToolResourceReport::default();
    assert_eq!(r.event_count(), 0);
}

// 14. I1 public API summary
#[test]
fn i1_14_summary() {
    let s = stage7_i1_summary();
    assert!(s.contains("I1"));
    assert!(s.contains("D1"));
    assert!(s.contains("G1"));
}

// 15. I1 公共 API healthy
#[test]
fn i1_15_healthy() {
    assert!(stage7_i1_healthy());
}

// 16. I1 跟 D1 协同
#[test]
fn i1_16_to_d1() {
    assert!(stage7_i1_to_d1_consistency());
}

// 17. I1 跟 G1 协同
#[test]
fn i1_17_to_g1() {
    assert!(stage7_i1_to_g1_consistency());
}
