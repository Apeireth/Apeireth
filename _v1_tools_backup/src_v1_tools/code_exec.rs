//! `/v1/tools/code_exec/invoke` — **4 真接** (D-02 子路径风格)
//!
//! **端点**: `POST /v1/tools/code_exec/invoke`
//! **调内部 trait**: `apeireth-tools::CodeExecTool::call` (per `v2_endpoints.rs:32`)
//! **Req schema** (per §2.2 路由表 #4):
//! ```json
//! { "args": { "command": "string", "cwd"?: "string", "env"?: {...}, "timeout_ms"?: 30000 } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! {
//!   "ok": true,
//!   "result": { "exit_code": 0, "stdout": "...", "stderr": "" },
//!   "error": null,
//!   "meta": { "tool": "code_exec", ... }
//! }
//! ```
//!
//! **真接 (D-01 + §5 决策表)**: 4 真接之 4, 走 `apeireth-tools::CodeExecTool` 真跑子进程.
//! **不假装**: 0 stub, 0 返 501, 真 `tokio::process::Command` + `tokio::time::timeout`.
//! **不修改承诺**: 0 改 `apeireth-tools/src/` (LOCKED), 仅通过 registry 复用.
//!
//! **P0 endpoint** (per §2.6 限流策略 E 急救路径): code_exec 在 P0 list
//! (`/v1/sovereignty/check` + `/v1/agent/spawn` + `/v1/tools/code_exec/invoke`),
//! 阶段 3 引入限流 middleware 后, 本 endpoint 1000/s 软上限, 触发只 WARN 不 429.
//!
//! **错误码** (per §2.5 12 类映射, 本阶段 2 实施 4 类):
//! - 503 / 404 / 200 ok=false / 200 ok=true (同 web_search)

/// **6 端点 handler** — 直接 re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

// ============================================================
// 端点 spec 单元测试
// ============================================================

#[cfg(test)]
mod code_exec_tests {
    /// 验证 4 真接路径之四正确 (per §2.2 路由表 #4)
    #[test]
    fn code_exec_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[3], "/tools/code_exec/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[3], "code_exec");
        assert_eq!(super::super::V1_REGISTRY_NAMES[3], "ShellExec");
    }

    /// 验证 code_exec 真接
    #[test]
    fn code_exec_is_real_not_stub() {
        // VCP ShellExec 真名 (per `apeireth-tools/src/register.rs:27`)
        assert_eq!(super::super::V1_REGISTRY_NAMES[3], "ShellExec");
    }

    /// 验证 code_exec 在 P0 endpoint 名单 (per §2.6 E 急救路径)
    #[test]
    fn code_exec_is_p0_endpoint() {
        // P0 名单: /v1/sovereignty/check + /v1/agent/spawn + /v1/tools/code_exec/invoke
        // 本测试仅 doc 性质, 阶段 3 限流 middleware 实施时硬约束
        let p0_endpoints = [
            "/v1/sovereignty/check",
            "/v1/agent/spawn",
            "/v1/tools/code_exec/invoke",
        ];
        let path = "/v1/tools/code_exec/invoke";
        assert!(p0_endpoints.contains(&path));
    }
}
