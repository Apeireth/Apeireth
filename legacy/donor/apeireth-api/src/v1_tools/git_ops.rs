//! `/v1/tools/git_ops/invoke` — **4 真接** (D-02 子路径风格)

/// 6 端点 handler — re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

#[cfg(test)]
mod git_ops_tests {
    #[test]
    fn git_ops_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_REGISTRY_NAMES[2], "Git");
    }
}
