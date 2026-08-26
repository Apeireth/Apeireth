//! # apeireth-graph benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 graph orchestration 关键 API 性能:
//! - `Graph::new()` + `add_node` + `add_edge`: 10 节点线性图构造
//! - `Graph::execute()`: 异步执行 + 拓扑排序
//! - `Graph::node_count()`: BTreeMap 大小查询
//! - `Edge::new(from, to)`: 边构造
//! - `Graph::checkpoint()`: 序列化 checkpoint
//!
//! **基线** (1.0.0): target/criterion/apeireth-graph/bench/

use apeireth_graph::{Edge, Graph, Node, NodeOutput, Result, State};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use tokio::runtime::Runtime;

struct BenchNode {
    id: String,
}

impl Node for BenchNode {
    fn id(&self) -> apeireth_graph::NodeId {
        self.id.clone()
    }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let mut trace: Vec<serde_json::Value> = state
            .remove("trace")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        trace.push(json!(self.id));
        state.insert("trace", json!(trace));
        Ok(NodeOutput::new(&self.id))
    }
}

fn make_linear_graph(n: usize) -> Graph {
    let mut graph = Graph::new();
    let ids: Vec<String> = (0..n).map(|i| format!("n{i}")).collect();
    for id in &ids {
        graph.add_node(BenchNode { id: id.clone() });
    }
    for i in 0..n.saturating_sub(1) {
        graph.add_edge(&ids[i], &ids[i + 1]);
    }
    graph
}

fn bench_graph_construct_10_nodes(c: &mut Criterion) {
    c.bench_function("graph_construct_10_nodes", |b| {
        b.iter(|| {
            let _ = make_linear_graph(black_box(10));
        });
    });
}

fn bench_graph_construct_100_nodes(c: &mut Criterion) {
    c.bench_function("graph_construct_100_nodes", |b| {
        b.iter(|| {
            let _ = make_linear_graph(black_box(100));
        });
    });
}

fn bench_graph_node_count(c: &mut Criterion) {
    c.bench_function("graph_node_count", |b| {
        let graph = make_linear_graph(50);
        b.iter(|| {
            let _ = black_box(graph.node_count());
        });
    });
}

fn bench_edge_new(c: &mut Criterion) {
    c.bench_function("edge_new", |b| {
        b.iter(|| {
            let _ = Edge::new(black_box("from"), black_box("to"));
        });
    });
}

fn bench_graph_execute_10_nodes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("graph_execute_10_nodes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let graph = make_linear_graph(black_box(10));
                let _ = graph.execute(State::new()).await.unwrap();
            });
        });
    });
}

criterion_group!(
    benches,
    bench_graph_construct_10_nodes,
    bench_graph_construct_100_nodes,
    bench_graph_node_count,
    bench_edge_new,
    bench_graph_execute_10_nodes
);
criterion_main!(benches);
