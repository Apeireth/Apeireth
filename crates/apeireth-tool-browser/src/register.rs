//! N17 工具装配 (TP2): tool-browser 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `EnhancedBrowser::dispatch_cli` (CLI 命令派发), 不自写调用方式.
//!
//! **JSON 约定**:
//! `{"op": "navigate", "url": <str>}` / `{"op": "snapshot", "kind"?: "full"|"text"|"refs"}`
//! / `{"op": "click", "ref_id": <str>}` / `{"op": "type", "ref_id": <str>, "text": <str>}`
//! / `{"op": "extract"}` / `{"op": "help"}`
//! **0 装 PASS**: 未接 CDP/MCP 通道 (引擎侧能力), 装配层只暴露 CLI 派发路.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::cli::{CliCommand, SnapshotKind};
use crate::enhanced::{DispatchResult, EnhancedBrowser};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "EnhancedBrowser";

/// Tool trait 适配器: 持 EnhancedBrowser (默认 FetchBrowser 内核).
pub struct EnhancedBrowserTool {
    browser: EnhancedBrowser,
}

impl EnhancedBrowserTool {
    /// 从已构造引擎装配 (测试/自定义 Browser 实现用).
    pub fn new(browser: EnhancedBrowser) -> Self {
        Self { browser }
    }
}

#[async_trait]
impl Tool for EnhancedBrowserTool {
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
            transport: TransportAxis::Network,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        let cmd = match op {
            "navigate" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `url`".to_string())?;
                CliCommand::Navigate(url.to_string())
            }
            "snapshot" => {
                let kind = match args.get("kind").and_then(Value::as_str).unwrap_or("full") {
                    "text" => SnapshotKind::Text,
                    "refs" => SnapshotKind::Refs,
                    _ => SnapshotKind::Full,
                };
                CliCommand::Snapshot(kind)
            }
            "click" => {
                let ref_id = args
                    .get("ref_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `ref_id`".to_string())?;
                CliCommand::Click(ref_id.to_string())
            }
            "type" => {
                let ref_id = args
                    .get("ref_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `ref_id`".to_string())?;
                let text = args
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `text`".to_string())?;
                CliCommand::Type {
                    ref_id: ref_id.to_string(),
                    text: text.to_string(),
                }
            }
            "extract" => CliCommand::Extract,
            "help" => CliCommand::Help,
            _ => return Err(format!("unknown op `{op}` (expected navigate|snapshot|click|type|extract|help)")),
        };
        let result = self
            .browser
            .dispatch_cli(cmd)
            .await
            .map_err(|e| e.to_string())?;
        Ok(match result {
            DispatchResult::Snapshot(snap) => json!({
                "op": op,
                "url": snap.url,
                "title": snap.title,
                "snapshot": snap.accessibility.to_snapshot(),
            }),
            DispatchResult::Text(text) => json!({ "op": op, "text": text }),
        })
    }
}

/// 统一注册进 registry (§10 铁边界③). 默认 FetchBrowser 内核.
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    let browser = EnhancedBrowser::from_fetch().map_err(|e| format!("EnhancedBrowser::from_fetch: {e}"))?;
    registry.register(TOOL_NAME.to_string(), Arc::new(EnhancedBrowserTool::new(browser)));
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
    async fn unknown_op_rejected() {
        let browser = EnhancedBrowser::from_fetch().expect("browser");
        let tool = EnhancedBrowserTool::new(browser);
        let e = tool.call(json!({"op": "fly"})).await.unwrap_err();
        assert!(e.contains("unknown op"));
    }

    #[tokio::test]
    async fn navigate_missing_url_rejected() {
        let browser = EnhancedBrowser::from_fetch().expect("browser");
        let tool = EnhancedBrowserTool::new(browser);
        let e = tool.call(json!({"op": "navigate"})).await.unwrap_err();
        assert!(e.contains("url"));
    }

    #[tokio::test]
    async fn help_returns_text() {
        let browser = EnhancedBrowser::from_fetch().expect("browser");
        let tool = EnhancedBrowserTool::new(browser);
        let r = tool.call(json!({"op": "help"})).await.expect("help");
        assert!(r["text"].as_str().map(str::len).unwrap_or(0) > 0, "help 应返文本");
    }
}
