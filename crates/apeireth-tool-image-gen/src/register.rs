//! N17 工具装配 (TP2): tool-image-gen 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `EnhancedImageGen::generate_mock` (mock provider 真出图字节), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "generate", "prompt": <str>, "params"?: <ImageGenParams JSON>}`
//! **0 装 PASS**: 装配层默认走 mock provider; 真实 API provider (DALL-E/SD 等 12 种) 需 key,
//! 缺 key 时引擎返 NotImplemented/MissingKey 错误, 装配层如实透传不假装.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::enhanced::EnhancedImageGen;
use crate::params::ImageGenParams;

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "ImageGenEnhanced";

/// Tool trait 适配器: 持 EnhancedImageGen.
pub struct ImageGenTool {
    engine: EnhancedImageGen,
}

impl ImageGenTool {
    /// 从已构造引擎装配.
    pub fn new(engine: EnhancedImageGen) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Tool for ImageGenTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Async
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Deferred,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        match op {
            "generate" => {
                let prompt = args
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `prompt`".to_string())?;
                let mut params = ImageGenParams::new(prompt);
                if let Some(p) = args.get("params") {
                    params = serde_json::from_value::<ImageGenParams>(p.clone())
                        .map_err(|e| format!("invalid `params`: {e}"))?;
                }
                let result = self
                    .engine
                    .generate_mock(&params)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({
                    "op": "generate",
                    "provider": result.provider,
                    "model": result.model,
                    "elapsed_ms": result.elapsed_ms,
                    "images": result.images.iter().map(|img| json!({
                        "mime": img.mime,
                        "width": img.width,
                        "bytes": img.data.len(),
                    })).collect::<Vec<_>>(),
                }))
            }
            _ => Err(format!("unknown op `{op}` (expected generate)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③).
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(ImageGenTool::new(EnhancedImageGen::new())),
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
    async fn generate_mock_produces_image_bytes() {
        let tool = ImageGenTool::new(EnhancedImageGen::new());
        let r = tool
            .call(json!({"op": "generate", "prompt": "a cat on the moon"}))
            .await
            .expect("generate");
        assert_eq!(r["provider"], "mock");
        let images = r["images"].as_array().expect("images");
        assert!(!images.is_empty(), "mock 应真出图");
        assert!(images[0]["bytes"].as_u64().unwrap_or(0) > 0);
    }

    #[tokio::test]
    async fn missing_prompt_rejected() {
        let tool = ImageGenTool::new(EnhancedImageGen::new());
        let e = tool.call(json!({"op": "generate"})).await.unwrap_err();
        assert!(e.contains("prompt"));
    }

    #[tokio::test]
    async fn invalid_params_rejected() {
        let tool = ImageGenTool::new(EnhancedImageGen::new());
        let e = tool
            .call(json!({"op": "generate", "prompt": "x", "params": {"count": "not-a-number"}}))
            .await
            .unwrap_err();
        assert!(e.contains("params"));
    }
}
