//! `/v1/tools/git_ops/invoke` — **4 真接** (D-02 子路径风格)
//!
//! **端点**: `POST /v1/tools/git_ops/invoke`
//! **调内部 trait**: `apeireth-tools::GitOpsTool::call` (per `v2_endpoints.rs:32`)
//! **Req schema** (per §2.2 路由表 #3):
//! ```json
//! { "args": { "action": "status|log|diff|commit|branch|checkout", "args": { ... } } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! { "ok": true, "result": { "data": ... }, "error": null, "meta": { "tool": "git_ops", ... } }
//! ```
//!
//! **真接 (D-01 + §5 决策表)**: 4 真接之 3, 走 `apeireth-tools::GitOpsTool` 真调 git CLI.
//! **不假装**: 0 stub, 0 返 501, 真跑 `git status/log/diff` (3 ops 主, 6 ops 完整).
//! **不修改承诺**: 0 改 `apeireth-tools/src/` (LOCKED), 仅通过 registry 复用.
//!
//! **3 ops** (per `apeireth-tools::GIT_OPS_OP_COUNT = 3`):
//! - status / log / diff (主); commit / branch / checkout 预留 (per §2.2 6 actions)
//!
//! **错误码** (per §2.5 12 类映射, 本阶段 2 实施 4 类):
//! - 503 / 404 / 200 ok=false / 200 ok=true (同 web_search)

/// **6 端点 handler** — 直接 re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

// ============================================================
// 端点 spec 单元测试
// ============================================================

#[cfg(test)]
mod git_ops_tests {
    /// 验证 4 真接路径之三正确 (per §2.2 路由表 #3)
    #[test]
    fn git_ops_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[2], "/tools/git_ops/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[2], "git_ops");
        assert_eq!(super::super::V1_REGISTRY_NAMES[2], "Git");
    }

    /// 验证 git_ops 真接
    #[test]
    fn git_ops_is_real_not_stub() {
        // VCP Git 真名 (per `apeireth-tools/src/register.rs:26`)
        assert_eq!(super::super::V1_REGISTRY_NAMES[2], "Git");
    }
}
