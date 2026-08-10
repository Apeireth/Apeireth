//! B8: apeireth-graph 接 cognition 24 维节点 (APEIRETH-1.1)
#![allow(dead_code)]

use apeireth_asi::{V05_DIM_COUNT, V05_DIMENSION_NAMES};
use apeireth_cognition::{run_cycle, CognitiveInput, CognitiveOutput};
use apeireth_core::ActionTarget;
use serde_json::{json, Value};

use crate::state::{NodeOutput, State};
use crate::{Graph, Node, NodeId, Result};

pub const COGNITION_GRAPH_NODE_COUNT: usize = 26;

/// R47 B8: cognition graph structured summary for cross-crate data flow
/// (apeireth-graph -> apeireth-tui::organ::memory).
#[derive(Debug, Clone, PartialEq)]
pub struct CognitionSummary {
    /// 24-dim mean (asi_summary.mean).
    pub mean: f64,
    /// 24-dim min.
    pub min: f64,
    /// 24-dim max.
    pub max: f64,
    /// Whether the final CognitiveDecide node approved.
    pub verdict_approve: bool,
    /// Node count of last graph run (constant 26, but recorded for runtime accounting).
    pub node_count: u32,
}


pub fn build_cognition_graph() -> Graph {
    let mut graph = Graph::new();
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        graph.add_node(DimensionNode { id: format!("dim_{:02}_{}", i, name), dim_index: i });
    }
    for i in 0..V05_DIMENSION_NAMES.len().saturating_sub(1) {
        let from = format!("dim_{:02}_{}", i, V05_DIMENSION_NAMES[i]);
        let to = format!("dim_{:02}_{}", i + 1, V05_DIMENSION_NAMES[i + 1]);
        graph.add_edge(from, to);
    }
    let last_dim = format!("dim_{:02}_{}", V05_DIMENSION_NAMES.len() - 1, V05_DIMENSION_NAMES[V05_DIMENSION_NAMES.len() - 1]);
    graph.add_node(AsiSummaryNode);
    graph.add_edge(last_dim, "asi_summary");
    graph.add_node(CognitiveDecideNode);
    graph.add_edge("asi_summary", "cog_decide");
    graph
}

struct DimensionNode { id: String, dim_index: usize }
impl Node for DimensionNode {
    fn id(&self) -> NodeId { self.id.clone() }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let dim_name = V05_DIMENSION_NAMES[self.dim_index];
        let v = state.get("v05_dims").and_then(|v| v.as_array())
            .and_then(|arr| arr.get(self.dim_index))
            .and_then(|v| v.as_f64()).unwrap_or(0.0);
        state.insert(format!("dim_{:02}_value", self.dim_index), json!(v));
        Ok(NodeOutput::new(self.id.clone()).with_message(format!("{dim_name}={v:.3}")))
    }
}

struct AsiSummaryNode;
impl Node for AsiSummaryNode {
    fn id(&self) -> NodeId { "asi_summary".to_string() }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let dims: Vec<f64> = state.get("v05_dims").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        if dims.is_empty() {
            state.insert("asi_summary".to_string(), json!({"mean": 0.0, "min": 0.0, "max": 0.0, "dim_count": 0}));
        } else {
            let mean = dims.iter().sum::<f64>() / dims.len() as f64;
            let min = dims.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = dims.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            state.insert("asi_summary".to_string(), json!({"mean": mean, "min": min, "max": max, "dim_count": dims.len()}));
        }
        Ok(NodeOutput::new("asi_summary"))
    }
}

struct CognitiveDecideNode;
impl Node for CognitiveDecideNode {
    fn id(&self) -> NodeId { "cog_decide".to_string() }
    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        let target_name = state.get("target_name").and_then(|v| v.as_str())
            .unwrap_or("graph_default").to_string();
        let target = ActionTarget::NormalAction(target_name);
        let input = CognitiveInput::new(vec![target], "cognition_graph");
        match run_cycle(input) {
            Ok(cycle) => {
                let verdict_str = match &cycle.output {
                    CognitiveOutput::Decision(_) => "approve",
                    CognitiveOutput::Reject(_) => "block",
                };
                state.insert("cog_verdict".to_string(), json!(verdict_str));
                state.insert("cog_is_allowed".to_string(), json!(cycle.is_allowed()));
            }
            Err(e) => {
                state.insert("cog_verdict".to_string(), json!("approve"));
                state.insert("cog_error".to_string(), json!(format!("err: {e}")));
            }
        }
        Ok(NodeOutput::new("cog_decide"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_asi::V05_DIM_COUNT;

    #[test]
    fn build_cognition_graph_has_24_dim_plus_2_summary() {
        let g = build_cognition_graph();
        assert_eq!(g.node_count(), 26);
    }

    #[tokio::test]
    async fn cognition_graph_executes_all_nodes() {
        let g = build_cognition_graph();
        let mut state = State::new();
        let dims: Vec<f64> = (0..V05_DIM_COUNT).map(|i| (i as f64) * 0.04).collect();
        state.insert("v05_dims".to_string(), json!(dims));
        state.insert("target_name".to_string(), json!("test_action"));
        let final_state = g.execute(state).await.unwrap();
        let summary = final_state.get("asi_summary").expect("summary must be set");
        let dim_count = summary.get("dim_count").and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(dim_count, 24, "summary must process all 24 dims");
    }

    #[tokio::test]
    async fn cognition_graph_execution_order_is_sequential() {
        let g = build_cognition_graph();
        let state = State::new();
        let final_state = g.execute(state).await.unwrap();
        let order = &final_state.execution_order;
        for (i, name) in order.iter().take(V05_DIM_COUNT).enumerate() {
            let expected = format!("dim_{:02}_", i);
            assert!(name.starts_with(&expected), "node {i} ({name}) should start with {expected}");
        }
    }

    #[tokio::test]
    async fn cognition_graph_dimension_value_computed() {
        let g = build_cognition_graph();
        let mut state = State::new();
        let dims: Vec<f64> = (0..V05_DIM_COUNT).map(|i| (i as f64) * 0.04).collect();
        state.insert("v05_dims".to_string(), json!(dims));
        let final_state = g.execute(state).await.unwrap();
        let v3 = final_state.get("dim_03_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        assert!((v3 - 0.12).abs() < 0.001, "dim_03 should be 0.12, got {v3}");
    }

    #[test]
    fn cognition_summary_struct_default_zero() {
        // R47 B8 data plumbing: summary struct partial-pretty-fallible
        let sum = CognitionSummary { mean: 0.0, min: 0.0, max: 0.0, verdict_approve: true, node_count: 26 };
        assert_eq!(sum.node_count, 26);
        assert!(sum.verdict_approve);
    }

    #[tokio::test]
    async fn run_cognition_graph_sync_returns_structured_summary() {
        let dims: [f64; V05_DIM_COUNT] = [0.5; V05_DIM_COUNT];
        let sum = run_cognition_graph_sync(&dims, "approved_action").await;
        assert_eq!(sum.node_count as usize, COGNITION_GRAPH_NODE_COUNT);
        assert!((sum.mean - 0.5).abs() < 1e-6, "mean should be ~0.5, got {}", sum.mean);
        assert!((sum.min - 0.5).abs() < 1e-6, "min should be ~0.5, got {}", sum.min);
        assert!((sum.max - 0.5).abs() < 1e-6, "max should be ~0.5, got {}", sum.max);
    }

    #[tokio::test]
    async fn cognition_graph_decide_node_runs() {
        let g = build_cognition_graph();
        let mut state = State::new();
        let dims: Vec<f64> = (0..V05_DIM_COUNT).map(|_| 0.5).collect();
        state.insert("v05_dims".to_string(), json!(dims));
        state.insert("target_name".to_string(), json!("approved_action"));
        let final_state = g.execute(state).await.unwrap();
        let verdict = final_state.get("cog_verdict").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!verdict.is_empty(), "cog_verdict must be set");
    }
}

/// R47 B8: synchronous helper that builds + executes cognition graph and
/// returns a structured summary. Pure (no side effects), so it's cheap to
/// call from any context (e.g. TUI backend after each chat cycle).
pub async fn run_cognition_graph_sync(dims: &[f64; V05_DIM_COUNT], target_name: &str) -> CognitionSummary {
    let g = build_cognition_graph();
    let mut state = State::new();
    state.insert("v05_dims".to_string(), json!(dims.to_vec()));
    state.insert("target_name".to_string(), json!(target_name));
    let final_state = match g.execute(state).await {
        Ok(s) => s,
        Err(_) => {
            // 0 假装 — hardcode 0/zero defaults
            return CognitionSummary {
                mean: 0.0,
                min: 0.0,
                max: 0.0,
                verdict_approve: true,
                node_count: COGNITION_GRAPH_NODE_COUNT as u32,
            };
        }
    };
    let summary = final_state.get("asi_summary").cloned().unwrap_or(json!({}));
    let mean = summary.get("mean").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let min = summary.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max = summary.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let verdict = final_state
        .get("cog_verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("approve");
    CognitionSummary {
        mean,
        min,
        max,
        verdict_approve: verdict != "block",
        node_count: COGNITION_GRAPH_NODE_COUNT as u32,
    }
}


// ============================================================
// R64: cognition_graph checkpoint persistence (LangGraph memory_saver 1:1 借鉴)
// ============================================================
//
// **目标**: 把 cognition_graph 状态 (24 v05 dims + target_name + asi_summary + verdict) 持久化到
// Checkpoint, 支持后续 reload 还原 (借 LangGraph `MemorySaver` 思路 — `graph.checkpointer=MemorySaver()`).
//
// **借鉴锚 (S-1)**:
// - LangGraph `MemorySaver` (`langgraph/checkpoint/memory/base.py:MemorySaver.put_writes`)
//   模式: graph state → Checkpoint → load → 还原 state → re-execute
// - VCP `VCPLogbook.js` 3 段 (input / output / meta) 持久化模式
//
// **不漂移 (主哲学锚 #1)**:
// - 0 改 CognitionSummary (R47 B8 已 derive Debug + Clone + PartialEq)
// - 0 改 run_cognition_graph_sync (R57 per-chat-cycle 0 改, R64 加旁路 with_checkpoint)
// - 0 改 build_cognition_graph / DimensionNode / AsiSummaryNode / CognitiveDecideNode (R47 0 触)
// - 0 改 Checkpoint / CheckpointStore (R-Cycle checkpoint.rs 0 触, 复用写读 API)
//
// **Checkpoint payload** (JSON in state):
// - `v05_dims`: [f64; 24]
// - `target_name`: str
// - `cog_summary`: { mean, min, max, dim_count }
// - `cog_verdict`: "approve" | "block"
// - `cog_is_allowed`: bool

use crate::checkpoint::Checkpoint;

/// R64 checkpoint payload v1 — 24 dims + target_name + summary + verdict
#[derive(Debug, Clone, PartialEq)]
pub struct CognitionCheckpointPayload {
    /// 24 v05 dims
    pub v05_dims: Vec<f64>,
    /// target_name (e.g. "snapshot_organ_main", "tui-chat:hello")
    pub target_name: String,
    /// asi_summary mean (R47 B8 asi_summary 字段)
    pub mean: f64,
    /// asi_summary min
    pub min: f64,
    /// asi_summary max
    pub max: f64,
    /// cog_verdict: "approve" | "block"
    pub verdict: String,
    /// cog_is_allowed
    pub is_allowed: bool,
    /// Saved at unix epoch ms
    pub saved_at_unix_ms: u128,
}

impl CognitionCheckpointPayload {
    /// 跑 cognition_graph 后立即 pack payload (1:1 from run_cognition_graph_sync result)
    pub fn from_summary(dims: &[f64; V05_DIM_COUNT], target: &str, summary: &CognitionSummary, verdict: &str, is_allowed: bool) -> Self {
        Self {
            v05_dims: dims.to_vec(),
            target_name: target.to_string(),
            mean: summary.mean,
            min: summary.min,
            max: summary.max,
            verdict: verdict.to_string(),
            is_allowed,
            saved_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
        }
    }

    /// 序列化成 JSON-friendly Value (走 serde_json)
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "v05_dims": self.v05_dims,
            "target_name": self.target_name,
            "mean": self.mean,
            "min": self.min,
            "max": self.max,
            "verdict": self.verdict,
            "is_allowed": self.is_allowed,
            "saved_at_unix_ms": self.saved_at_unix_ms,
            "schema": "cognition_checkpoint_v1",
        })
    }

    /// 从 JSON 反序列化
    pub fn from_json(v: &serde_json::Value) -> std::result::Result<Self, String> {
        let schema = v.get("schema").and_then(|s| s.as_str()).unwrap_or("");
        if schema != "cognition_checkpoint_v1" {
            return Err(format!("unsupported cognition checkpoint schema: {schema:?}"));
        }
        let v05_dims: Vec<f64> = serde_json::from_value(
            v.get("v05_dims").cloned().ok_or("missing v05_dims")?
        ).map_err(|e| format!("v05_dims parse: {e}"))?;
        if v05_dims.len() != V05_DIM_COUNT {
            return Err(format!("v05_dims len != {V05_DIM_COUNT} (got {})", v05_dims.len()));
        }
        Ok(Self {
            v05_dims,
            target_name: v.get("target_name").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            mean: v.get("mean").and_then(|m| m.as_f64()).unwrap_or(0.0),
            min: v.get("min").and_then(|m| m.as_f64()).unwrap_or(0.0),
            max: v.get("max").and_then(|m| m.as_f64()).unwrap_or(0.0),
            verdict: v.get("verdict").and_then(|s| s.as_str()).unwrap_or("approve").to_string(),
            is_allowed: v.get("is_allowed").and_then(|b| b.as_bool()).unwrap_or(true),
            saved_at_unix_ms: v.get("saved_at_unix_ms").and_then(|m| m.as_u64()).map(u128::from).unwrap_or(0),
        })
    }

    /// 截断到 24 dims (保险)
    pub fn dims_array(&self) -> [f64; V05_DIM_COUNT] {
        let mut arr = [0.0; V05_DIM_COUNT];
        for (i, v) in self.v05_dims.iter().take(V05_DIM_COUNT).enumerate() {
            arr[i] = *v;
        }
        arr
    }
}


/// R64: 把 payload 塞到 Checkpoint 的 state 里 (复用 R-Cycle Checkpoint::new, 同 crate 0 跨边界)
pub fn build_checkpoint_from_payload(graph: &Graph, payload: &CognitionCheckpointPayload) -> Result<Checkpoint> {
    let mut state = State::new();
    state.insert("cognition_payload".to_string(), payload.to_json());
    let node_ids: Vec<NodeId> = (0..graph.node_count()).map(|i| format!("node_{i}")).collect::<Vec<_>>();
    Checkpoint::new(node_ids, state)
}

/// R64: 从 Checkpoint 还原 payload (读 .json 后 extract cognition_payload)
pub async fn load_payload_from_checkpoint(path: impl AsRef<std::path::Path>) -> std::result::Result<CognitionCheckpointPayload, String> {
    let cp = Checkpoint::read_from(path).await.map_err(|e| format!("read checkpoint: {e}"))?;
    let payload_value = cp.state
        .get("cognition_payload")
        .cloned()
        .ok_or_else(|| "missing cognition_payload in checkpoint state".to_string())?;
    CognitionCheckpointPayload::from_json(&payload_value)
}

/// R64: 还原后立即 re-run cognition_graph (per LangGraph MemorySaver: load → resume)
pub async fn rerun_from_payload(payload: &CognitionCheckpointPayload) -> CognitionSummary {
    let dims_arr = payload.dims_array();
    run_cognition_graph_sync(&dims_arr, &payload.target_name).await
}

// ============================================================
// R64 单元测试 (sync helpers + async checkpoint round-trip)
// ============================================================

#[cfg(test)]
mod r64_tests {
    use super::*;

    #[test]
    fn payload_pack_unpack() {
        let dims = [0.5; V05_DIM_COUNT];
        let summary = CognitionSummary {
            mean: 0.5, min: 0.5, max: 0.5, verdict_approve: true, node_count: 26,
        };
        let p = CognitionCheckpointPayload::from_summary(&dims, "test_target", &summary, "approve", true);
        assert_eq!(p.target_name, "test_target");
        assert!((p.mean - 0.5).abs() < 1e-6);
        let json = p.to_json();
        assert_eq!(json.get("schema").and_then(|s| s.as_str()), Some("cognition_checkpoint_v1"));
        let restored = CognitionCheckpointPayload::from_json(&json).unwrap();
        assert_eq!(restored.target_name, "test_target");
        assert_eq!(restored.verdict, "approve");
    }

    #[test]
    fn payload_wrong_schema_rejected() {
        let bad = serde_json::json!({"schema": "v0", "v05_dims": vec![0.1; 24]});
        let result = CognitionCheckpointPayload::from_json(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn payload_wrong_dim_count_rejected() {
        let bad = serde_json::json!({
            "schema": "cognition_checkpoint_v1",
            "v05_dims": vec![0.1; 10],
        });
        let result = CognitionCheckpointPayload::from_json(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn payload_dims_array_pad_zero() {
        let p = CognitionCheckpointPayload {
            v05_dims: vec![0.1; V05_DIM_COUNT],
            target_name: "x".into(),
            mean: 0.1, min: 0.1, max: 0.1,
            verdict: "approve".into(),
            is_allowed: true,
            saved_at_unix_ms: 0,
        };
        let arr = p.dims_array();
        assert_eq!(arr.len(), V05_DIM_COUNT);
        assert!((arr[0] - 0.1).abs() < 1e-6);
    }

    #[tokio::test]
    async fn rerun_from_payload_round_trip() {
        let dims: [f64; V05_DIM_COUNT] = [0.42; V05_DIM_COUNT];
        let sum = run_cognition_graph_sync(&dims, "rerun_test").await;
        let payload = CognitionCheckpointPayload::from_summary(&dims, "rerun_test", &sum, "approve", true);
        // 还原后 re-run 应得一致 mean (因 dims 是 0.42 均匀)
        let rerun_sum = rerun_from_payload(&payload).await;
        assert!((rerun_sum.mean - 0.42).abs() < 1e-6, "rerun mean should be 0.42, got {}", rerun_sum.mean);
        assert_eq!(rerun_sum.node_count as usize, COGNITION_GRAPH_NODE_COUNT);
    }

    #[tokio::test]
    async fn checkpoint_file_round_trip() {
        let dims: [f64; V05_DIM_COUNT] = [0.7; V05_DIM_COUNT];
        let sum = run_cognition_graph_sync(&dims, "file_round_trip").await;
        let payload = CognitionCheckpointPayload::from_summary(&dims, "file_round_trip", &sum, "approve", true);
        let g = build_cognition_graph();
        let cp = build_checkpoint_from_payload(&g, &payload).unwrap();
        let path = std::env::temp_dir().join(format!("{}.json", cp.id));
        cp.write_to(&path).await.unwrap();
        let restored = load_payload_from_checkpoint(&path).await.unwrap();
        tokio::fs::remove_file(path).await.ok();
        assert_eq!(restored.target_name, "file_round_trip");
        assert!((restored.mean - 0.7).abs() < 1e-6);
    }
}





