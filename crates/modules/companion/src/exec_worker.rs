//! `apeireth-companion::exec_worker` — 执行体隔离: per-call 子进程 (吸收 NemesisBot Layer 1 思想, 重写).
//!
//! 思想: 高危/有副作用的工具调用剥到每次新起的子进程, 一行 JSON 请求 / 一行 JSON 响应;
//! 子进程零状态、跑完即退、可超时 kill, 崩溃/失控不污染主进程 (daemon).
//!
//! 分界 (MOVE/STAY):
//! - MOVE (走子进程): FileOperator / Git / ShellExec / Grep / ApplyPatch — 文件系统/进程副作用
//! - STAY (留宿主): recall_memory / save_memory / WebSearch / WebFetch — 只读或网络
//!
//! 安全位置: 安全判断 (洋葱门/宪法评审/权限包/路径约束) 全部在**宿主**执行完才 spawn,
//! 子进程只执行「已批准的操作」— 与 NemesisBot 同思路 (安全层在网关, 隔离是兜底).
//!
//! 协议: 每行一个 JSON.
//!   请求: `{"tool": "FileOperator", "args": {...}}`
//!   响应: `{"ok": true, "output": ...}` 或 `{"ok": false, "error": "..."}`
//! 子进程处理**一行请求后立即退出** (per-call 语义).

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolRegistry};
use serde_json::{json, Value};

/// MOVE/STAY 分界: 该工具是否应剥离到子进程执行.
pub fn should_isolate(tool: &str) -> bool {
    let t = tool.to_lowercase();
    t.contains("file")
        || t == "git"
        || t.contains("shell")
        || t == "grep"
        || t.contains("patch")
        || t == "code_exec"
        || t.contains("exec")
}

/// 构造 worker 侧工具注册表 (apeireth-tools 全部真工具).
pub fn worker_registry() -> Arc<ToolRegistry> {
    let registry = Arc::new(ToolRegistry::new());
    let _ = apeireth_tools::register_all(&registry);
    registry
}

/// 处理一行协议请求 (可单测): 解析 → 执行 → 响应 JSON.
pub async fn handle_line(registry: &ToolRegistry, line: &str) -> Value {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": format!("请求不是合法 JSON: {e}")}),
    };
    let tool = req.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let args = req.get("args").cloned().unwrap_or(json!({}));
    if tool.is_empty() {
        return json!({"ok": false, "error": "缺少 tool 字段"});
    }
    let Some(t) = registry.get(tool) else {
        return json!({"ok": false, "error": format!("worker 无此工具: {tool}")});
    };
    match t.call(args).await {
        Ok(out) => json!({"ok": true, "output": out}),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

/// worker 入口: 读一行请求 → 执行 → 输出一行响应 → 退出 (per-call).
/// 主进程 (example/daemon) 在收到 `APEIRETH_EXEC_WORKER=1` 环境变量时短路调用本函数.
pub async fn run() {
    use tokio::io::AsyncBufReadExt;
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let registry = worker_registry();
    while let Ok(Some(line)) = lines.next_line().await {
        let resp = handle_line(&registry, &line).await;
        println!("{}", resp);
        break; // per-call: 一行请求即退
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reg() -> ToolRegistry {
        let r = ToolRegistry::new();
        let _ = apeireth_tools::register_all(&r);
        r
    }

    #[tokio::test]
    async fn protocol_writes_file_through_worker_registry() {
        let r = reg();
        let dir = std::env::temp_dir().join(format!("apeireth-worker-test-{}", std::process::id()));
        let target = dir.join("out.txt");
        let req = json!({
            "tool": "FileOperator",
            "args": {"op": "write", "path": target.to_string_lossy().to_string(), "content": "隔离执行"}
        })
        .to_string();
        let resp = handle_line(&r, &req).await;
        assert_eq!(resp["ok"], json!(true), "resp={resp}");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap_or_default(),
            "隔离执行"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn protocol_rejects_unknown_tool() {
        let r = reg();
        let resp = handle_line(&r, r#"{"tool": "NoSuchTool", "args": {}}"#).await;
        assert_eq!(resp["ok"], json!(false));
        assert!(resp["error"].as_str().unwrap_or("").contains("无此工具"));
    }

    #[tokio::test]
    async fn protocol_rejects_bad_json() {
        let r = reg();
        let resp = handle_line(&r, "not json").await;
        assert_eq!(resp["ok"], json!(false));
    }

    #[test]
    fn move_stay_partition_is_explicit() {
        for t in [
            "FileOperator",
            "Git",
            "ShellExec",
            "Grep",
            "ApplyPatch",
            "code_exec",
        ] {
            assert!(should_isolate(t), "{t} 应为 MOVE (剥离)");
        }
        for t in ["recall_memory", "save_memory", "WebSearch", "WebFetch"] {
            assert!(!should_isolate(t), "{t} 应为 STAY (留宿主)");
        }
    }
}
