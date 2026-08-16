//! N17 工具装配 (TP2): tool-codesearch 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `UnifiedCodeIntelligence::query` (6 维统一查询), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "query", "kind": "text"|"file"|"symbol"|"graph"|"index"|"ast", "pattern": <str>, "path": <str>, "lang"?: <str>}`
//! **0 装 PASS**: 命中序列化 Text 全字段, 其余变体返 kind 标记 (深层结构调用方可再走各子模块).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::unified::{IntelligenceHit, IntelligenceKind, UnifiedCodeIntelligence, UnifiedQuery};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "CodeIntelligence";

/// Tool trait 适配器: 持 UnifiedCodeIntelligence (内存索引).
pub struct CodeIntelligenceTool {
    engine: UnifiedCodeIntelligence,
}

impl CodeIntelligenceTool {
    /// 从已构造引擎装配 (测试/带 ast-grep 二进制配置用).
    pub fn new(engine: UnifiedCodeIntelligence) -> Self {
        Self { engine }
    }
}

fn parse_kind(s: &str) -> Result<IntelligenceKind, String> {
    Ok(match s {
        "text" => IntelligenceKind::Text,
        "file" => IntelligenceKind::File,
        "symbol" => IntelligenceKind::Symbol,
        "graph" => IntelligenceKind::Graph,
        "index" => IntelligenceKind::Index,
        "ast" => IntelligenceKind::Ast,
        _ => return Err(format!("unknown kind `{s}` (expected text|file|symbol|graph|index|ast)")),
    })
}

fn hit_to_json(hit: &IntelligenceHit) -> Value {
    match hit {
        IntelligenceHit::Text { file, line, column, text } => json!({
            "kind": "text",
            "file": file.to_string_lossy(),
            "line": line,
            "column": column,
            "text": text,
        }),
        other => json!({ "kind": format!("{:?}", other.kind()).to_lowercase() }),
    }
}

#[async_trait]
impl Tool for CodeIntelligenceTool {
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
            "query" => {
                let kind_s = args.get("kind").and_then(Value::as_str).unwrap_or("text");
                let kind = parse_kind(kind_s)?;
                let pattern = args
                    .get("pattern")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `pattern`".to_string())?;
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `path`".to_string())?;
                let mut q = UnifiedQuery::new(kind, pattern, path);
                if let Some(lang) = args.get("lang").and_then(Value::as_str) {
                    q = q.with_lang(lang);
                }
                let hits = self.engine.query(&q).map_err(|e| e.to_string())?;
                let count = hits.len();
                Ok(json!({
                    "op": "query",
                    "count": count,
                    "hits": hits.iter().map(hit_to_json).collect::<Vec<_>>(),
                }))
            }
            _ => Err(format!("unknown op `{op}` (expected query)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③). 默认内存索引.
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(CodeIntelligenceTool::new(UnifiedCodeIntelligence::new_in_memory())),
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
    async fn text_query_finds_pattern_in_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn n17_marker() {}\n").unwrap();
        let tool = CodeIntelligenceTool::new(UnifiedCodeIntelligence::new_in_memory());
        let r = tool
            .call(json!({
                "op": "query",
                "kind": "text",
                "pattern": "n17_marker",
                "path": dir.path().to_string_lossy(),
            }))
            .await
            .expect("query");
        assert!(r["count"].as_u64().unwrap_or(0) >= 1, "应命中临时文件: {r}");
    }

    #[tokio::test]
    async fn unknown_kind_rejected() {
        let tool = CodeIntelligenceTool::new(UnifiedCodeIntelligence::new_in_memory());
        let e = tool
            .call(json!({"op": "query", "kind": "telepathy", "pattern": "x", "path": "."}))
            .await
            .unwrap_err();
        assert!(e.contains("unknown kind"));
    }

    #[tokio::test]
    async fn missing_pattern_rejected() {
        let tool = CodeIntelligenceTool::new(UnifiedCodeIntelligence::new_in_memory());
        let e = tool
            .call(json!({"op": "query", "path": "."}))
            .await
            .unwrap_err();
        assert!(e.contains("pattern"));
    }
}
