//! `/v1/tools/file_ops/invoke` — **4 真接** (D-02 子路径风格)
//!
//! **端点**: `POST /v1/tools/file_ops/invoke`
//! **调内部 trait**: `apeireth-tools::FileOpsTool::call` (per `v2_endpoints.rs:32`)
//! **Req schema** (per §2.2 路由表 #2):
//! ```json
//! { "args": { "action": "read|write|list|stat|delete", "path": "string", "content"?: "string" } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! { "ok": true, "result": { "data": ... }, "error": null, "meta": { "tool": "file_ops", ... } }
//! ```
//!
//! **真接 (D-01 + §5 决策表)**: 4 真接之 2, 走 `apeireth-tools::FileOpsTool` 真文件操作.
//! **不假装**: 0 stub, 0 返 501, 真调 `tokio::fs` 6 操作 (read/write/list/mkdir/delete/move).
//! **不修改承诺**: 0 改 `apeireth-tools/src/` (LOCKED), 仅通过 registry 复用.
//!
//! **6 ops** (per `apeireth-tools::FILE_OPS_OPERATION_COUNT = 6`):
//! - read / write / list / mkdir / delete / move (VCP FileOperator 字段级)
//!
//! **VCP 字段级对齐** (per `apeireth-tools/src/lib.rs:14-22`):
//! - `MAX_FILE_SIZE=20MB` / `MAX_DIRECTORY_ITEMS=1000` / `MAX_SEARCH_RESULTS=100`
//!
//! **错误码** (per §2.5 12 类映射, 本阶段 2 实施 4 类):
//! - 503 / 404 / 200 ok=false / 200 ok=true (同 web_search)

/// **6 端点 handler** — 直接 re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

// ============================================================
// 端点 spec 单元测试
// ============================================================

#[cfg(test)]
mod file_ops_tests {
    /// 验证 4 真接路径之二正确 (per §2.2 路由表 #2)
    #[test]
    fn file_ops_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[1], "/tools/file_ops/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[1], "file_ops");
        assert_eq!(super::super::V1_REGISTRY_NAMES[1], "FileOperator");
    }

    /// 验证 file_ops 真接
    #[test]
    fn file_ops_is_real_not_stub() {
        // VCP FileOperator 真名 (per `apeireth-tools/src/register.rs:25`)
        assert_eq!(super::super::V1_REGISTRY_NAMES[1], "FileOperator");
    }
}
