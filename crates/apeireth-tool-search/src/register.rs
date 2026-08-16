//! N17 工具装配 (TP2): tool-search 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `SearchEngine` (倒排索引 + TF 评分 + 聚合), 不自写调用方式.
//!
//! **JSON 约定**:
//! `{"op": "index", "source": <str>, "topic": <str>, "body": <str>}` → `{"doc_id": <u64>}`
//! `{"op": "search", "query": <str>, "limit"?: <u64>}` → `{"hits": [{id, source, topic, score}]}`
//! `{"op": "remove", "id": <u64>}` / `{"op": "len"}`

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::{Document, SearchEngine};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "VSearch";

/// Tool trait 适配器: 持 SearchEngine (内存倒排索引).
pub struct VSearchTool {
    engine: Arc<SearchEngine>,
}

impl VSearchTool {
    /// 从已构造引擎装配 (可与 pipeline 共享同一索引实例).
    pub fn new(engine: Arc<SearchEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Tool for VSearchTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Cached,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        match op {
            "index" => {
                let source = args
                    .get("source")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `source`".to_string())?;
                let topic = args
                    .get("topic")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `topic`".to_string())?;
                let body = args
                    .get("body")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `body`".to_string())?;
                let doc_id = self.engine.index(Document::new(0, source, topic, body));
                Ok(json!({ "op": "index", "doc_id": doc_id }))
            }
            "search" => {
                let query = args
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `query`".to_string())?;
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                let hits = self
                    .engine
                    .search(query, limit)
                    .map_err(|e| e.to_string())?;
                Ok(json!({
                    "op": "search",
                    "count": hits.len(),
                    "hits": hits.iter().map(|r| json!({
                        "id": r.doc.id,
                        "source": r.doc.source,
                        "topic": r.doc.topic,
                        "score": r.score,
                    })).collect::<Vec<_>>(),
                }))
            }
            "remove" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing `id`".to_string())?;
                self.engine.remove(id).map_err(|e| e.to_string())?;
                Ok(json!({ "op": "remove", "id": id }))
            }
            "len" => Ok(json!({ "op": "len", "len": self.engine.len() })),
            _ => Err(format!("unknown op `{op}` (expected index|search|remove|len)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③). 默认新建独立索引.
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(VSearchTool::new(Arc::new(SearchEngine::new()))),
    );
    Ok(())
}

/// 卸载真清理 (§5.6 插件规范, 0 残留).
pub fn unregister(registry: &ToolRegistry) -> bool {
    registry.unregister(TOOL_NAME).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_adds_and_unregister_cleans() {
        let registry = ToolRegistry::new();
        register(&registry).expect("register");
        assert!(registry.get(TOOL_NAME).is_some());
        let before = registry.len();
        assert!(unregister(&registry));
        assert!(registry.get(TOOL_NAME).is_none(), "卸载后 0 残留");
        assert_eq!(registry.len(), before - 1);
    }

    #[tokio::test]
    async fn index_then_search_roundtrip() {
        let tool = VSearchTool::new(Arc::new(SearchEngine::new()));
        let r = tool
            .call(json!({"op": "index", "source": "note", "topic": "n17", "body": "装配引擎验收标记"}))
            .await
            .expect("index");
        assert!(r["doc_id"].as_u64().is_some());
        let r = tool
            .call(json!({"op": "search", "query": "装配引擎验收标记"}))
            .await
            .expect("search");
        assert_eq!(r["count"], 1, "应命中刚索引的文档: {r}");
        assert_eq!(r["hits"][0]["topic"], "n17");
    }

    #[tokio::test]
    async fn remove_missing_doc_errors() {
        let tool = VSearchTool::new(Arc::new(SearchEngine::new()));
        let e = tool.call(json!({"op": "remove", "id": 99999})).await.unwrap_err();
        assert!(!e.is_empty(), "删除不存在的文档应报错");
    }

    #[tokio::test]
    async fn empty_query_errors() {
        let tool = VSearchTool::new(Arc::new(SearchEngine::new()));
        let e = tool.call(json!({"op": "search", "query": ""})).await.unwrap_err();
        assert!(!e.is_empty(), "空查询应报错");
    }
}
