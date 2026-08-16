//! N17 工具装配 (TP2): tool-fetch 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `FetchEngine` (限速/缓存/指标), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "fetch", "url": <str>, "method"?: <str>, "body"?: <str>, "extract_text_only"?: <bool>}`
//! **0 装 PASS**: 未接 anysearch/bilibili/deep 子能力 (引擎侧能力, 装配层只暴露通用 fetch).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::{FetchEngine, FetchRequest};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "FetchEngine";

/// Tool trait 适配器: 持 FetchEngine.
pub struct FetchEngineTool {
    engine: FetchEngine,
}

impl FetchEngineTool {
    /// 从已构造引擎装配.
    pub fn new(engine: FetchEngine) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Tool for FetchEngineTool {
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
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Network,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        match op {
            "fetch" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `url`".to_string())?;
                let mut req = FetchRequest::get(url);
                if let Some(m) = args.get("method").and_then(Value::as_str) {
                    req.method = Some(m.to_string());
                }
                if let Some(b) = args.get("body").and_then(Value::as_str) {
                    req.body = Some(b.to_string());
                }
                if let Some(t) = args.get("extract_text_only").and_then(Value::as_bool) {
                    req.extract_text_only = t;
                }
                let resp = self
                    .engine
                    .fetch(&req)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({
                    "op": "fetch",
                    "url": resp.url,
                    "final_url": resp.final_url,
                    "status": resp.status,
                    "content_type": resp.content_type,
                    "bytes_received": resp.bytes_received,
                    "elapsed_ms": resp.elapsed_ms,
                    "body": resp.body,
                }))
            }
            _ => Err(format!("unknown op `{op}` (expected fetch)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③).
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(FetchEngineTool::new(FetchEngine::new())),
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
    async fn invalid_url_errors_without_panic() {
        let tool = FetchEngineTool::new(FetchEngine::new());
        let r = tool.call(json!({"op": "fetch", "url": "not a valid url"})).await;
        assert!(r.is_err(), "非法 URL 应报错");
    }

    #[tokio::test]
    async fn missing_url_rejected() {
        let tool = FetchEngineTool::new(FetchEngine::new());
        let e = tool.call(json!({"op": "fetch"})).await.unwrap_err();
        assert!(e.contains("url"));
    }

    #[tokio::test]
    async fn unknown_op_rejected() {
        let tool = FetchEngineTool::new(FetchEngine::new());
        let e = tool.call(json!({"op": "fly"})).await.unwrap_err();
        assert!(e.contains("unknown op"));
    }
}
