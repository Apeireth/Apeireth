//! 5 节点 DAG workflow demo: Start → A → (B || C) → Join → End
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main/chunks/WorkflowGenerator-BQCQ_KQx.js` 的
//! `createWorkflow` + 节点编排 + 拓扑执行.
//!
//! `cargo run -p apeireth-workflow --example workflow_demo`

use apeireth_workflow::{
    AgentConfig, Branch, DecisionConfig, DefaultWorkflowExecutor, DefaultWorkflowGenerator,
    DefaultWorkflowValidator, ExecutionContext, LoopConfig, NodeConfig, NodeExecutor,
    NodeType, Result, Workflow, WorkflowEdge, WorkflowExecutor, WorkflowGenerator,
    WorkflowNode, WorkflowNodeId, WorkflowParser, WorkflowStatus, WorkflowValidator,
    YamlWorkflowParser,
};
use async_trait::async_trait;
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("=== apeireth-workflow 5 节点 DAG demo ===\n");

    // 1) 构造 5 节点 DAG: Start → A → (B || C) → Join → End
    let mut wf = Workflow::new("parallel-demo", "Start → A → (B || C) → Join → End");

    let start = make_node("start", "Start", NodeType::Start, vec![], NodeConfig::ControlFlow);
    let a = make_node(
        "a",
        "A (Agent)",
        NodeType::Agent,
        vec!["start".into()],
        NodeConfig::Agent(AgentConfig {
            prompt: "do task A".into(),
            ..Default::default()
        }),
    );
    let b = make_node(
        "b",
        "B (Agent, parallel)",
        NodeType::Agent,
        vec!["a".into()],
        NodeConfig::Agent(AgentConfig {
            prompt: "do task B in parallel".into(),
            ..Default::default()
        }),
    );
    let c = make_node(
        "c",
        "C (Agent, parallel)",
        NodeType::Agent,
        vec!["a".into()],
        NodeConfig::Agent(AgentConfig {
            prompt: "do task C in parallel".into(),
            ..Default::default()
        }),
    );
    let join = make_node(
        "join",
        "Join (barrier)",
        NodeType::Join,
        vec!["b".into(), "c".into()],
        NodeConfig::ControlFlow,
    );
    let end = make_node(
        "end",
        "End",
        NodeType::End,
        vec!["join".into()],
        NodeConfig::ControlFlow,
    );

    for n in [start, a, b, c, join, end] {
        wf.nodes.insert(n.id.clone(), n);
    }
    wf.edges.push(WorkflowEdge {
        from: "start".into(),
        to: "a".into(),
        edge_type: apeireth_workflow::EdgeType::Sequential,
        condition: None,
    });
    wf.edges.push(WorkflowEdge {
        from: "a".into(),
        to: "b".into(),
        edge_type: apeireth_workflow::EdgeType::Parallel,
        condition: None,
    });
    wf.edges.push(WorkflowEdge {
        from: "a".into(),
        to: "c".into(),
        edge_type: apeireth_workflow::EdgeType::Parallel,
        condition: None,
    });
    wf.edges.push(WorkflowEdge {
        from: "b".into(),
        to: "join".into(),
        edge_type: apeireth_workflow::EdgeType::Sequential,
        condition: None,
    });
    wf.edges.push(WorkflowEdge {
        from: "c".into(),
        to: "join".into(),
        edge_type: apeireth_workflow::EdgeType::Sequential,
        condition: None,
    });
    wf.edges.push(WorkflowEdge {
        from: "join".into(),
        to: "end".into(),
        edge_type: apeireth_workflow::EdgeType::Sequential,
        condition: None,
    });

    println!("[1] 创建 workflow: {} ({} 节点, {} 边)", wf.name, wf.node_count(), wf.edge_count());

    // 2) 验证 workflow
    let validator = DefaultWorkflowValidator;
    validator.validate(&wf)?;
    println!("[2] 验证通过: 4 警告守门 + 拓扑排序 + 嵌套深度");

    // 3) 打印拓扑顺序
    let order = validator.topological_order(&wf)?;
    println!("[3] 拓扑顺序: {:?}", order);

    // 4) 用真 NoopNodeExecutor 执行
    let executor = DefaultWorkflowExecutor::new(Arc::new(NoopNodeExecutor));
    let mut ctx = ExecutionContext::new();
    let exec = executor.execute(&wf, &mut ctx).await?;
    println!("[4] 执行完成: status = {:?}, 完成 {} 节点", exec.status, exec.completed_nodes.len());

    assert_eq!(exec.status, WorkflowStatus::Completed);
    assert_eq!(exec.completed_nodes.len(), wf.node_count());

    // 5) YAML round-trip (1:1 v0.9.21 parseWorkflowFromText → to_yaml)
    let parser = YamlWorkflowParser;
    let yaml = parser.to_yaml(&wf).await?;
    let restored = parser.from_yaml(&yaml).await?;
    println!("[5] YAML round-trip: {} → {} 节点", restored.id, restored.node_count());
    assert_eq!(restored.node_count(), wf.node_count());

    // 6) quick_agent 演示 (1:1 v0.9.21 createQuickAgentTask)
    let gen = DefaultWorkflowGenerator;
    let quick = gen.quick_agent("quick-demo", "explain this code", ".").await?;
    println!(
        "[6] quick_agent: {} ({} 节点, type = {:?})",
        quick.name,
        quick.node_count(),
        quick.nodes.values().next().unwrap().node_type
    );
    assert_eq!(quick.node_count(), 1);

    println!("\n=== 5 节点 DAG demo 完成 ===");
    Ok(())
}

/// Helper: 构造一个 WorkflowNode.
fn make_node(
    id: &str,
    name: &str,
    node_type: NodeType,
    depends_on: Vec<WorkflowNodeId>,
    config: NodeConfig,
) -> WorkflowNode {
    WorkflowNode {
        id: id.into(),
        name: name.into(),
        node_type,
        config,
        depends_on,
        trigger: "auto".into(),
        timeout_ms: None,
        interactive: false,
        intervention_config: None,
    }
}

/// NoopNodeExecutor (P0 占位, R20 阶段 2 接 apeireth-agent).
pub struct NoopNodeExecutor;

#[async_trait]
impl NodeExecutor for NoopNodeExecutor {
    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &mut ExecutionContext,
    ) -> Result<serde_json::Value> {
        println!("    → executing node: {} ({:?})", node.id, node.node_type);
        Ok(serde_json::json!({ "node_id": node.id, "noop": true }))
    }
}

// 故意让 `Branch` / `DecisionConfig` / `LoopConfig` 留作未来 demo 引用, 不触发 unused 警告.
#[allow(dead_code)]
fn _ensure_types_compile() {
    let _ = std::any::type_name::<Branch>();
    let _ = std::any::type_name::<DecisionConfig>();
    let _ = std::any::type_name::<LoopConfig>();
}
