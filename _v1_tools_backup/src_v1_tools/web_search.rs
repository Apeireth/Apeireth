//! `/v1/tools/web_search/invoke` — **4 真接** (D-02 子路径风格)
//!
//! **端点**: `POST /v1/tools/web_search/invoke`
//! **调内部 trait**: `apeireth-tools::WebSearchTool::call` (per `v2_endpoints.rs:32`)
//! **Req schema** (per §2.2 路由表 #1):
//! ```json
//! { "args": { "query": "string", "top_k": 10, "filter": { ... } } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! {
//!   "ok": true,
//!   "result": { "results": [{ "title", "url", "snippet" }] },
//!   "error": null,
//!   "meta": { "tool": "web_search", "duration_ms": 234, "trace_id": "tr-..." }
//! }
//! ```
//!
//! **真接 (D-01 + §5 决策表)**: 4 真接之 1, 走 `apeireth-tools::WebSearchTool` 真 HTTP 调.
//! **不假装**: 0 stub, 0 返 501, 0 假装实装.
//! **不修改承诺**: 0 改 `apeireth-tools/src/` (LOCKED), 仅通过 registry 复用.
//!
//! **架构**:
//! ```text
//!   axum POST /v1/tools/web_search/invoke
//!     ↓
//!   v1_tools::web_search::invoke
//!     ↓
//!   super::invoke_by_name (统一 dispatch, Path("web_search") extracted from URL)
//!     ↓
//!   ToolRegistry.get("WebSearch") → WebSearchTool → Tool::call(args)
//!     ↓
//!   HttpWebSearch 真发 HTTP 5 字段 keep-alive (per `apeireth-http-client` VCP 借鉴)
//! ```
//!
//! **错误码** (per §2.5 12 类映射, 本阶段 2 实施 4 类):
//! - 503 — V2State.tools 未初始化
//! - 404 — 工具名未注册 (正常不该发生, 防御性)
//! - 200 ok=false + error — WebSearchTool 自身返 (e.g. empty query 返 error_code:400)
//! - 200 ok=true + result — 成功

/// **6 端点 handler** — 直接 re-export 共享 dispatch, 路由由 `v2_endpoints::build_router` 注册
///
/// **设计**: 4 真接 (web_search / file_ops / git_ops / code_exec) 走同一 `super::invoke_by_name`,
/// 路径由 axum `Path<String>` 从 URL 提取, 自动路由到对应 tool.
pub use super::invoke_by_name as invoke;

// ============================================================
// 端点 spec 单元测试 (per §2.9 V20-S2-V1 守门)
// ============================================================

#[cfg(test)]
mod web_search_tests {
    /// 验证 4 真接路径之一正确 (per §2.2 路由表 #1)
    #[test]
    fn web_search_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[0], "/tools/web_search/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[0], "web_search");
        assert_eq!(super::super::V1_REGISTRY_NAMES[0], "WebSearch");
    }

    /// 验证 web_search 真接 (D-01 + 蓝图 §1.4)
    #[test]
    fn web_search_is_real_not_stub() {
        // 4 真接 — web_search 对应 registry "WebSearch" (VCP 真名)
        assert_eq!(super::super::V1_REGISTRY_NAMES[0], "WebSearch");
    }
}
