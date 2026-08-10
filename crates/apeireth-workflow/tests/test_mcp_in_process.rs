//! Fixture 5: in-process Workflow 验证调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `Workflow::new` 构造 + 空状态 (node_count=0, is_empty=true)
//! 2. `DefaultWorkflowValidator::validate` 接受空 workflow
//! 3. `DefaultWorkflowValidator::topological_order` 接受空 workflow
//!
//! 注: workflow crate 自含 `DefaultWorkflowValidator` (m3 防御模式已在, 跳过 TOOL_WHITELIST 嵌入).
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_workflow::{DefaultWorkflowValidator, Workflow, WorkflowValidator};

#[test]
fn test_workflow_new_creates_empty_state() {
    let wf = Workflow::new("test-wf", "fixture 5 test workflow");
    assert_eq!(wf.node_count(), 0, "新构造 workflow 应无节点");
    assert_eq!(wf.edge_count(), 0, "新构造 workflow 应无边");
    assert!(wf.is_empty(), "新构造 workflow 应 is_empty");
    assert_eq!(wf.name, "test-wf");
    assert_eq!(wf.description, "fixture 5 test workflow");
}

#[test]
fn test_workflow_validator_accepts_empty_workflow() {
    let wf = Workflow::new("empty", "no nodes");
    let validator = DefaultWorkflowValidator;
    let result = validator.validate(&wf);
    assert!(result.is_ok(), "空 workflow 应通过 validate: {result:?}");
}

#[test]
fn test_workflow_validator_topological_order_on_empty() {
    let wf = Workflow::new("empty", "no nodes");
    let validator = DefaultWorkflowValidator;
    let order = validator
        .topological_order(&wf)
        .expect("空 workflow 应返回空拓扑序");
    assert!(order.is_empty(), "空 workflow 拓扑序应为空 vec");
}
