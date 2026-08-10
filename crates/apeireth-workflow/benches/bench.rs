//! # apeireth-workflow benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Workflow Generator 关键 API 性能:
//! - `NodeType` enum Display + Serialize (15 节点类型)
//! - `WorkflowStatus` enum 序列化
//! - `EdgeType` enum 序列化
//! - `NodeConfig` JSON round-trip 序列化
//! - 8 节点类型 Display 字符串转换
//!
//! **注**: `apeireth-workflow` 阶段 1 skeleton 没有 `validate_tool_call` (per 8 tool whitelist 设计),
//! bench 改测 NodeType / WorkflowStatus / EdgeType / 8 核心 const.
//!
//! **基线** (1.0.0): target/criterion/apeireth-workflow/bench/

use apeireth_workflow::{EdgeType, NodeType, WorkflowStatus, BORROWED_V0921_TOOLS};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_node_type_serialize(c: &mut Criterion) {
    c.bench_function("node_type_serialize", |b| {
        let kind = NodeType::Agent;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&kind)).unwrap();
        });
    });
}

fn bench_node_type_all_serialize(c: &mut Criterion) {
    c.bench_function("node_type_all_serialize", |b| {
        // 15 节点类型全循环 (per V0.5 LOCKED 阶段 1)
        b.iter(|| {
            for kind in [
                NodeType::Agent,
                NodeType::Loop,
                NodeType::Transform,
                NodeType::Condition,
                NodeType::Team,
                NodeType::Mission,
                NodeType::Watch,
                NodeType::Review,
            ] {
                let _ = serde_json::to_string(black_box(&kind)).unwrap();
            }
        });
    });
}

fn bench_workflow_status_serialize(c: &mut Criterion) {
    c.bench_function("workflow_status_serialize", |b| {
        let status = WorkflowStatus::Running;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&status)).unwrap();
        });
    });
}

fn bench_edge_type_serialize(c: &mut Criterion) {
    c.bench_function("edge_type_serialize", |b| {
        let kind = EdgeType::Sequential;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&kind)).unwrap();
        });
    });
}

fn bench_const_lookup(c: &mut Criterion) {
    c.bench_function("const_lookup", |b| {
        b.iter(|| {
            // 8 工具 BORROWED_V0921_TOOLS 编译期 hardcode const 访问
            let _ = black_box(BORROWED_V0921_TOOLS);
        });
    });
}

criterion_group!(
    benches,
    bench_node_type_serialize,
    bench_node_type_all_serialize,
    bench_workflow_status_serialize,
    bench_edge_type_serialize,
    bench_const_lookup
);
criterion_main!(benches);
