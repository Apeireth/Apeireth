//! R89: CognitionGraph → MCP `resources` 桥接 (graph state 暴露为 MCP resources)
//!
//! **目标**: 让 apeireth-graph 的 cognition graph state 可以作为 MCP server 暴露,
//! 任意 MCP client (Claude Desktop / IDE / ...) 都能 list/read.
//!
//! **Apeireth 真接 (本 module)**:
//! - `CognitionGraphResourceServer` 实现 `apeireth_mcp::resources::ResourceServer`
//!   - `list()` — 3 个 resources:
//!     - `apeireth://graph/last-summary` — CognitionSummary JSON (24 维 mean/min/max/verdict)
//!     - `apeireth://graph/last-checkpoint` — CognitionCheckpointPayload JSON
//!     - `apeireth://graph/dimensions` — 24 维 names + values 列表
//!   - `read(uri)` — 按 URI 返对应内容 (JSON 文本, mime_type=application/json)
//! - `GraphResourceSnapshot` — 内部快照 struct, 持有 last CognitionSummary / last CognitionCheckpointPayload
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-graph/src/cognition_graph.rs` 已有 CognitionSummary / CognitionCheckpointPayload (R47 + R54 + R64 LOCKED)
//! - 0 改 `apeireth-mcp/src/resources.rs` 已有 Resource / ResourceContent / ResourceServer (R33-3 LOCKED)
//! - 0 引入 I/O / 网络 (snapshot 由 caller 注入, 0 自创 I/O)
//!
//! **借鉴锚 (S-5)**:
//! - MCP spec 2025-03-26 §resources (URI scheme + mimeType 1:1)
//! - LangGraph `MemorySaver.list()` + `get_tuple()` (graph state 暴露为可查对象)
//! - VCP `vcptoolbox/modules` JSON 路由 (URI pattern 风格)

use std::sync::Arc;

use apeireth_asi::{V05_DIMENSION_NAMES, V05_DIM_COUNT};
use apeireth_mcp::protocol::JsonRpcError;
use apeireth_mcp::resources::{Resource, ResourceContent, ResourceServer, RESOURCE_NOT_FOUND};
use serde_json::{json, Value};
use std::sync::Mutex;

use crate::cognition_graph::{CognitionCheckpointPayload, CognitionSummary};

// ============================================================
// URI 常量
// ============================================================

/// **apeireth://graph/last-summary** — 最近一次 cognition graph 跑的 summary
pub const URI_LAST_SUMMARY: &str = "apeireth://graph/last-summary";
/// **apeireth://graph/last-checkpoint** — 最近一次 cognition graph 的 checkpoint payload
pub const URI_LAST_CHECKPOINT: &str = "apeireth://graph/last-checkpoint";
/// **apeireth://graph/dimensions** — 24 维 names + values 列表
pub const URI_DIMENSIONS: &str = "apeireth://graph/dimensions";

/// **URI 匹配 helper**
pub fn is_apeireth_graph_uri(uri: &str) -> bool {
    uri.starts_with("apeireth://graph/")
}

// ============================================================
// 内部快照 (caller 注入, 0 I/O)
// ============================================================

/// **cognition graph 状态快照** — 持有最近一次跑的 summary + checkpoint + dims
///
/// caller (TUI / pipeline) 在 graph 跑完后 `replace(...)` 更新, MCP client
/// `read(uri)` 拿到的就是这个快照.
#[derive(Debug, Default, Clone)]
pub struct GraphResourceSnapshot {
    /// 最近一次 CognitionSummary
    pub last_summary: Option<CognitionSummary>,
    /// 最近一次 CognitionCheckpointPayload
    pub last_checkpoint: Option<CognitionCheckpointPayload>,
    /// 最近一次 24 维 values (用于 apeireth://graph/dimensions 资源)
    pub last_dims: Option<[f64; V05_DIM_COUNT]>,
}

impl GraphResourceSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// 整批更新 (caller 在 graph 跑完后调)
    pub fn replace(
        &mut self,
        summary: Option<CognitionSummary>,
        checkpoint: Option<CognitionCheckpointPayload>,
        dims: Option<[f64; V05_DIM_COUNT]>,
    ) {
        self.last_summary = summary;
        self.last_checkpoint = checkpoint;
        self.last_dims = dims;
    }

    /// 只更新 summary (其它保持)
    pub fn set_summary(&mut self, s: CognitionSummary) {
        self.last_summary = Some(s);
    }

    /// 只更新 checkpoint
    pub fn set_checkpoint(&mut self, c: CognitionCheckpointPayload) {
        self.last_checkpoint = Some(c);
    }

    /// 只更新 dims
    pub fn set_dims(&mut self, d: [f64; V05_DIM_COUNT]) {
        self.last_dims = Some(d);
    }
}

// ============================================================
// CognitionGraphResourceServer
// ============================================================

/// **CognitionGraph MCP ResourceServer** — 把 graph state 暴露为 MCP resources
///
/// 用 `Arc<Mutex<GraphResourceSnapshot>>` 共享 snapshot (TUI 和 MCP client 可同时访问).
/// 0 改 cognition_graph 已有类型, 0 引入 I/O.
pub struct CognitionGraphResourceServer {
    snapshot: Arc<Mutex<GraphResourceSnapshot>>,
}

impl std::fmt::Debug for CognitionGraphResourceServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .snapshot
            .lock()
            .expect("apeireth-graph mcp_resource snapshot mutex poisoned");
        f.debug_struct("CognitionGraphResourceServer")
            .field("has_summary", &s.last_summary.is_some())
            .field("has_checkpoint", &s.last_checkpoint.is_some())
            .field("has_dims", &s.last_dims.is_some())
            .finish()
    }
}

impl CognitionGraphResourceServer {
    /// 构造 + 共享 snapshot
    pub fn new(snapshot: Arc<Mutex<GraphResourceSnapshot>>) -> Self {
        Self { snapshot }
    }

    /// 构造新 snapshot (caller 独占)
    pub fn with_empty_snapshot() -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(GraphResourceSnapshot::new())),
        }
    }

    /// 拿 snapshot 引用 (给 caller 用来 update)
    pub fn snapshot_handle(&self) -> Arc<Mutex<GraphResourceSnapshot>> {
        Arc::clone(&self.snapshot)
    }
}

// ============================================================
// ResourceServer trait impl
// ============================================================

impl ResourceServer for CognitionGraphResourceServer {
    fn list(&self) -> Vec<Resource> {
        vec![
            Resource::new(URI_LAST_SUMMARY, "last-summary")
                .with_description("Cognition graph 最近一次跑的 24 维 summary (mean/min/max/verdict)")
                .with_mime_type("application/json"),
            Resource::new(URI_LAST_CHECKPOINT, "last-checkpoint")
                .with_description("Cognition graph 最近一次的 checkpoint payload (full 24 维 dims + summary + verdict)")
                .with_mime_type("application/json"),
            Resource::new(URI_DIMENSIONS, "dimensions")
                .with_description("Apeireth 24 维 V0.5 names + 最近一次 values 列表")
                .with_mime_type("application/json"),
        ]
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let snapshot = self
            .snapshot
            .lock()
            .expect("apeireth-graph mcp_resource snapshot mutex poisoned");
        match uri {
            URI_LAST_SUMMARY => {
                let summary = snapshot.last_summary.as_ref().ok_or_else(|| {
                    JsonRpcError::new(
                        RESOURCE_NOT_FOUND,
                        "no cognition graph summary yet (call replace(...) after a graph run)",
                    )
                })?;
                let v = json!({
                    "mean": summary.mean,
                    "min": summary.min,
                    "max": summary.max,
                    "verdict_approve": summary.verdict_approve,
                    "node_count": summary.node_count,
                });
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            URI_LAST_CHECKPOINT => {
                let cp = snapshot.last_checkpoint.as_ref().ok_or_else(|| {
                    JsonRpcError::new(
                        RESOURCE_NOT_FOUND,
                        "no cognition graph checkpoint yet (call replace(...) after a graph run)",
                    )
                })?;
                let v = cp.to_json();
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            URI_DIMENSIONS => {
                let dims = snapshot.last_dims.as_ref().ok_or_else(|| {
                    JsonRpcError::new(
                        RESOURCE_NOT_FOUND,
                        "no dimensions recorded yet (call replace(...) after a graph run)",
                    )
                })?;
                let arr: Vec<Value> = (0..V05_DIM_COUNT)
                    .map(|i| {
                        json!({
                            "index": i,
                            "name": V05_DIMENSION_NAMES[i],
                            "value": dims[i],
                        })
                    })
                    .collect();
                let v = json!({ "count": V05_DIM_COUNT, "items": arr });
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            _ => Err(JsonRpcError::new(
                RESOURCE_NOT_FOUND,
                format!("unknown apeireth://graph/ URI: {}", uri),
            )),
        }
    }
}

// ============================================================
// pipe helper (替代 `.pipe(Ok)` 长链)
// ============================================================

trait Pipe: Sized {
    fn pipe<U, F: FnOnce(Self) -> U>(self, f: F) -> U {
        f(self)
    }
}

impl<T> Pipe for T {}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_mcp::resources::Resource;

    fn make_summary(mean: f64, min: f64, max: f64, approve: bool) -> CognitionSummary {
        CognitionSummary {
            mean,
            min,
            max,
            verdict_approve: approve,
            node_count: 26,
        }
    }

    fn make_dims(values: &[f64]) -> [f64; V05_DIM_COUNT] {
        let mut arr = [0.0; V05_DIM_COUNT];
        let n = values.len().min(V05_DIM_COUNT);
        for i in 0..n {
            arr[i] = values[i];
        }
        arr
    }

    #[test]
    fn is_apeireth_graph_uri_basic() {
        assert!(is_apeireth_graph_uri(URI_LAST_SUMMARY));
        assert!(is_apeireth_graph_uri(URI_LAST_CHECKPOINT));
        assert!(is_apeireth_graph_uri(URI_DIMENSIONS));
        assert!(!is_apeireth_graph_uri("file:///etc/passwd"));
        assert!(!is_apeireth_graph_uri("apeireth://other/x"));
    }

    #[test]
    fn snapshot_default_is_empty() {
        let s = GraphResourceSnapshot::new();
        assert!(s.last_summary.is_none());
        assert!(s.last_checkpoint.is_none());
        assert!(s.last_dims.is_none());
    }

    #[test]
    fn snapshot_replace_all_fields() {
        let mut s = GraphResourceSnapshot::new();
        let summary = make_summary(0.5, 0.0, 1.0, true);
        let dims = make_dims(&[0.1; V05_DIM_COUNT]);
        s.replace(Some(summary.clone()), None, Some(dims));
        assert!(s.last_summary.is_some());
        assert_eq!(s.last_summary.as_ref().unwrap().mean, 0.5);
        assert!(s.last_dims.is_some());
    }

    #[test]
    fn snapshot_set_individual() {
        let mut s = GraphResourceSnapshot::new();
        s.set_summary(make_summary(0.7, 0.1, 0.9, true));
        s.set_dims(make_dims(&[0.5; V05_DIM_COUNT]));
        assert!(s.last_summary.is_some());
        assert!(s.last_dims.is_some());
        assert!(s.last_checkpoint.is_none());
    }

    #[test]
    fn resource_server_list_returns_three() {
        let s = CognitionGraphResourceServer::with_empty_snapshot();
        let list = s.list();
        assert_eq!(list.len(), 3);
        let uris: Vec<&str> = list.iter().map(|r: &Resource| r.uri.as_str()).collect();
        assert!(uris.contains(&URI_LAST_SUMMARY));
        assert!(uris.contains(&URI_LAST_CHECKPOINT));
        assert!(uris.contains(&URI_DIMENSIONS));
    }

    #[test]
    fn resource_server_list_resources_have_mime_json() {
        let s = CognitionGraphResourceServer::with_empty_snapshot();
        for r in s.list() {
            assert_eq!(r.mime_type.as_deref(), Some("application/json"));
        }
    }

    #[test]
    fn read_unknown_uri_errors_not_found() {
        let s = CognitionGraphResourceServer::with_empty_snapshot();
        let err = s.read("apeireth://graph/nonexistent").unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn read_non_apeireth_uri_errors_not_found() {
        let s = CognitionGraphResourceServer::with_empty_snapshot();
        let err = s.read("file:///etc/passwd").unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn read_last_summary_when_empty_errors() {
        let s = CognitionGraphResourceServer::with_empty_snapshot();
        let err = s.read(URI_LAST_SUMMARY).unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn read_last_summary_with_data() {
        let server = CognitionGraphResourceServer::with_empty_snapshot();
        {
            let handle = server.snapshot_handle();
            let mut snap = handle
                .lock()
                .expect("apeireth-graph mcp_resource snapshot mutex poisoned");
            snap.set_summary(make_summary(0.42, 0.1, 0.9, true));
        }
        let c = server.read(URI_LAST_SUMMARY).unwrap();
        assert_eq!(c.uri, URI_LAST_SUMMARY);
        assert_eq!(c.mime_type.as_deref(), Some("application/json"));
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["mean"], 0.42);
        assert_eq!(v["verdict_approve"], true);
        assert_eq!(v["node_count"], 26);
    }

    #[test]
    fn read_last_checkpoint_when_empty_errors() {
        let server = CognitionGraphResourceServer::with_empty_snapshot();
        let err = server.read(URI_LAST_CHECKPOINT).unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn read_dimensions_with_data() {
        let server = CognitionGraphResourceServer::with_empty_snapshot();
        let dims = make_dims(&[0.5; V05_DIM_COUNT]);
        {
            let handle = server.snapshot_handle();
            let mut snap = handle
                .lock()
                .expect("apeireth-graph mcp_resource snapshot mutex poisoned");
            snap.set_dims(dims);
        }
        let c = server.read(URI_DIMENSIONS).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["count"], V05_DIM_COUNT);
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), V05_DIM_COUNT);
        assert_eq!(items[0]["name"], V05_DIMENSION_NAMES[0]);
        assert_eq!(items[0]["index"], 0);
        assert_eq!(items[0]["value"], 0.5);
    }

    #[test]
    fn read_dimensions_when_empty_errors() {
        let server = CognitionGraphResourceServer::with_empty_snapshot();
        let err = server.read(URI_DIMENSIONS).unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn debug_impl_works() {
        let server = CognitionGraphResourceServer::with_empty_snapshot();
        let s = format!("{:?}", server);
        assert!(s.contains("CognitionGraphResourceServer"));
    }

    #[test]
    fn shared_snapshot_between_clones() {
        let server1 = CognitionGraphResourceServer::with_empty_snapshot();
        let handle = server1.snapshot_handle();
        {
            let mut s = handle
                .lock()
                .expect("apeireth-graph mcp_resource snapshot mutex poisoned");
            s.set_summary(make_summary(0.8, 0.0, 1.0, true));
        }
        // server1 直接 read (own snapshot)
        let c = server1.read(URI_LAST_SUMMARY).unwrap();
        assert!(c.text.contains("0.8"));
    }
}
