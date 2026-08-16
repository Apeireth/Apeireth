//! N17 工具装配 (TP2): tool-shell 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `EnhancedShell` (沙盒 + 持久任务), 不自写调用方式.
//!
//! **JSON 约定**: `{"op": "exec"|"exec_persistent", "cmd": <str>, "timeout_ms"?: <u64>}`
//! **0 装 PASS**: 未接 SSH/streaming 多签 (引擎能力, 装配层只暴露 exec 两路).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::{EnhancedShell, ShellError};

/// 注册名 (全局唯一, 不与 apeireth-tools::TOOL_NAMES 9 个撞名)
pub const TOOL_NAME: &str = "EnhancedShell";

/// 默认持久任务库路径 (注册时无参构造用)
fn default_db_path() -> PathBuf {
    std::env::temp_dir().join("apeireth-tool-shell-persist.db")
}

/// Tool trait 适配器: 持 EnhancedShell 引擎.
pub struct EnhancedShellTool {
    shell: EnhancedShell,
}

impl EnhancedShellTool {
    /// 从已构造引擎装配 (测试/自定义沙盒策略用).
    pub fn new(shell: EnhancedShell) -> Self {
        Self { shell }
    }
}

#[async_trait]
impl Tool for EnhancedShellTool {
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
            resident: ResidentAxis::Persistent,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        match op {
            "exec" | "exec_sandboxed" | "exec_persistent" => {
                let cmd = args
                    .get("cmd")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing `cmd`".to_string())?;
                let timeout_ms = args.get("timeout_ms").and_then(Value::as_u64).unwrap_or(30_000);
                let (exit_code, output) = if op == "exec_persistent" {
                    self.shell.exec_persistent(cmd, timeout_ms).await
                } else {
                    self.shell.exec_sandboxed(cmd, timeout_ms).await
                }
                .map_err(|e: ShellError| e.to_string())?;
                Ok(json!({ "op": op, "exit_code": exit_code, "output": output }))
            }
            _ => Err(format!("unknown op `{op}` (expected exec|exec_persistent)")),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③: 每 crate 提供 register 函数).
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    let shell = EnhancedShell::new(default_db_path())
        .map_err(|e| format!("EnhancedShell::new: {e}"))?;
    registry.register(TOOL_NAME.to_string(), Arc::new(EnhancedShellTool::new(shell)));
    Ok(())
}

/// 卸载真清理: 从 registry 移除本工具 (§5.6 插件规范, 0 残留).
pub fn unregister(registry: &ToolRegistry) -> bool {
    registry.unregister(TOOL_NAME).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool() -> (EnhancedShellTool, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().unwrap();
        let shell = EnhancedShell::new(tmp.path().join("t.db")).expect("shell");
        (EnhancedShellTool::new(shell), tmp)
    }

    #[test]
    fn register_adds_and_unregister_cleans() {
        let registry = ToolRegistry::new();
        register(&registry).expect("register");
        assert!(registry.get(TOOL_NAME).is_some(), "register 后应可查到");
        let before = registry.len();
        assert!(unregister(&registry), "unregister 应返 true");
        assert!(registry.get(TOOL_NAME).is_none(), "卸载后 0 残留");
        assert_eq!(registry.len(), before - 1);
        assert!(!unregister(&registry), "重复卸载返 false");
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "Windows echo spawn PATH 限制, 同 register.rs 既有口径")]
    async fn exec_sandboxed_echo_runs() {
        let (tool, _tmp) = make_tool();
        let r = tool.call(json!({"op": "exec", "cmd": "echo n17-shell"})).await.expect("exec");
        assert_eq!(r["exit_code"], 0);
        assert!(r["output"].as_str().unwrap().contains("n17-shell"));
    }

    #[tokio::test]
    async fn unknown_op_rejected() {
        let (tool, _tmp) = make_tool();
        let e = tool.call(json!({"op": "fly"})).await.unwrap_err();
        assert!(e.contains("unknown op"));
    }

    #[tokio::test]
    async fn missing_cmd_rejected() {
        let (tool, _tmp) = make_tool();
        let e = tool.call(json!({"op": "exec"})).await.unwrap_err();
        assert!(e.contains("cmd"));
    }
}
