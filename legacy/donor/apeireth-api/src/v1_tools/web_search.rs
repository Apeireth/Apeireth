//! `/v1/tools/web_search/invoke` — **4 真接** (D-02 子路径风格)
//! **端点**: `POST /tools/web_search/invoke`
//! **真接**: 复用 `apeireth-tools::WebSearchTool` (registry)
//! **不修改承诺**: 0 改 `apeireth-tools/src/` (LOCKED)

/// 6 端点 handler — re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

#[cfg(test)]
mod web_search_tests {
    #[test]
    fn web_search_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_REGISTRY_NAMES[0], "WebSearch");
    }
}
