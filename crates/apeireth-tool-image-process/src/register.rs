//! N17 工具装配 (TP2): tool-image-process 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `EnhancedImageProcess::process` (hash/exif/ocr/thumbnail 路由), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "hash"|"exif"|"ocr"|"thumbnail", "data_base64"?: <str>, "path"?: <str>, "lang"?: <str>}`
//! (data_base64 与 path 二选一; path 走本地文件读取)
//! **0 装 PASS**: OCR 真依赖缺失时引擎返错, 装配层如实透传.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::enhanced::EnhancedImageProcess;
use crate::router::ProcessOp;

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "ImageProcess";

/// Tool trait 适配器: 持 EnhancedImageProcess.
pub struct ImageProcessTool {
    engine: EnhancedImageProcess,
}

impl ImageProcessTool {
    /// 从已构造引擎装配.
    pub fn new(engine: EnhancedImageProcess) -> Self {
        Self { engine }
    }
}

fn parse_op(s: &str) -> Result<ProcessOp, String> {
    Ok(match s {
        "hash" => ProcessOp::Hash,
        "exif" => ProcessOp::Exif,
        "ocr" => ProcessOp::Ocr,
        "thumbnail" => ProcessOp::Thumbnail,
        _ => return Err(format!("unknown op `{s}` (expected hash|exif|ocr|thumbnail)")),
    })
}

#[async_trait]
impl Tool for ImageProcessTool {
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
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op_s = args.get("op").and_then(Value::as_str).unwrap_or("");
        let op = parse_op(op_s)?;
        let data: Vec<u8> = if let Some(b64) = args.get("data_base64").and_then(Value::as_str) {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| format!("invalid base64: {e}"))?
        } else if let Some(path) = args.get("path").and_then(Value::as_str) {
            tokio::fs::read(path)
                .await
                .map_err(|e| format!("read `{path}`: {e}"))?
        } else {
            return Err("missing `data_base64` or `path`".to_string());
        };
        let lang = args.get("lang").and_then(Value::as_str);
        let out = self
            .engine
            .process(op, &data, lang)
            .map_err(|e| e.to_string())?;
        Ok(json!({ "op": op_s, "result": out }))
    }
}

/// 统一注册进 registry (§10 铁边界③).
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(ImageProcessTool::new(EnhancedImageProcess::new())),
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
    async fn unknown_op_rejected() {
        let tool = ImageProcessTool::new(EnhancedImageProcess::new());
        let e = tool.call(json!({"op": "fly"})).await.unwrap_err();
        assert!(e.contains("unknown op"));
    }

    #[tokio::test]
    async fn missing_data_rejected() {
        let tool = ImageProcessTool::new(EnhancedImageProcess::new());
        let e = tool.call(json!({"op": "hash"})).await.unwrap_err();
        assert!(e.contains("data_base64"));
    }

    #[tokio::test]
    async fn invalid_base64_rejected() {
        let tool = ImageProcessTool::new(EnhancedImageProcess::new());
        let e = tool
            .call(json!({"op": "hash", "data_base64": "!!not-base64!!"}))
            .await
            .unwrap_err();
        assert!(e.contains("base64"));
    }
}
