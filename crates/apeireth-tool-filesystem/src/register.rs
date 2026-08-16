//! N17 工具装配 (TP2): tool-filesystem 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `StdEnhancedFileOps` (沙盒 resolve + 原子写), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "read"|"write", "path": <str>, "content"?: <str>}`
//! **沙盒根**: env `APEIRETH_TOOL_FS_ROOTS` (逗号分隔) 覆盖, 默认当前工作目录.
//! **0 装 PASS**: read_with_lock 涉 FileLockGuard 生命周期, 装配层不暴露 (引擎侧能力保留).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::{EnhancedFileOps, StdEnhancedFileOps};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "EnhancedFileOps";

/// 沙盒允许根: env 覆盖, 默认当前工作目录.
pub fn allowed_roots_from_env() -> Vec<PathBuf> {
    if let Ok(v) = std::env::var("APEIRETH_TOOL_FS_ROOTS") {
        let roots: Vec<PathBuf> = v.split(',').map(|s| PathBuf::from(s.trim())).filter(|p| !p.as_os_str().is_empty()).collect();
        if !roots.is_empty() {
            return roots;
        }
    }
    std::env::current_dir().map(|d| vec![d]).unwrap_or_default()
}

/// Tool trait 适配器: 持 StdEnhancedFileOps (沙盒 + 原子写).
pub struct EnhancedFileOpsTool {
    ops: StdEnhancedFileOps,
}

impl EnhancedFileOpsTool {
    /// 从已构造引擎装配 (测试/自定义沙盒根用).
    pub fn new(ops: StdEnhancedFileOps) -> Self {
        Self { ops }
    }
}

#[async_trait]
impl Tool for EnhancedFileOpsTool {
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
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        let path_s = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing `path`".to_string())?;
        let path = std::path::Path::new(path_s);
        match op {
            "read" => {
                let content = self.ops.read_sandboxed(path).await.map_err(|e| e.to_string())?;
                Ok(json!({ "op": "read", "path": path_s, "content": content }))
            }
            "write" => {
                let content = args
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `content`".to_string())?;
                self.ops
                    .write_atomic(path, content.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "op": "write", "path": path_s, "bytes": content.len() }))
            }
            _ => Err(format!("unknown op `{op}` (expected read|write)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③). 沙盒根 env 可配.
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(
        TOOL_NAME.to_string(),
        Arc::new(EnhancedFileOpsTool::new(StdEnhancedFileOps::new(
            allowed_roots_from_env(),
        ))),
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

    fn make_tool(tmp: &std::path::Path) -> EnhancedFileOpsTool {
        EnhancedFileOpsTool::new(StdEnhancedFileOps::new(vec![tmp.to_path_buf()]))
    }

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
    async fn write_then_read_roundtrip_in_sandbox() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = make_tool(tmp.path());
        let p = tmp.path().join("n17.txt");
        let r = tool
            .call(json!({"op": "write", "path": p.to_string_lossy(), "content": "n17-装配"}))
            .await
            .expect("write");
        assert_eq!(r["bytes"], "n17-装配".len());
        let r = tool
            .call(json!({"op": "read", "path": p.to_string_lossy()}))
            .await
            .expect("read");
        assert_eq!(r["content"], "n17-装配");
    }

    #[tokio::test]
    async fn read_outside_sandbox_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = make_tool(tmp.path());
        // 沙盒外路径 (父目录的父目录) 应被沙盒拒绝
        let outside = tmp.path().parent().and_then(|p| p.parent()).unwrap_or(tmp.path());
        let e = tool
            .call(json!({"op": "read", "path": outside.join("definitely-outside.txt").to_string_lossy()}))
            .await
            .unwrap_err();
        assert!(!e.is_empty(), "沙盒外读取应报错");
    }

    #[tokio::test]
    async fn unknown_op_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = make_tool(tmp.path());
        let e = tool.call(json!({"op": "fly", "path": "x"})).await.unwrap_err();
        assert!(e.contains("unknown op"));
    }
}
