//! `/v1/tools/code_exec/invoke` — **4 真接** (D-02 子路径风格)
//! **P0 endpoint** (per §2.6 E 急救路径): code_exec 在 P0 list (1000/s 软上限)

/// 6 端点 handler — re-export 共享 dispatch
pub use super::invoke_by_name as invoke;

#[cfg(test)]
mod code_exec_tests {
    #[test]
    fn code_exec_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_REGISTRY_NAMES[3], "ShellExec");
    }
}
