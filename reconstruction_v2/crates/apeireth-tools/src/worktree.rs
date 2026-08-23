use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sha2::{Digest, Sha256};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    pub patch_id: String,
    pub worktree_branch: String,
    pub changed_files: Vec<String>,
    pub diff_content: String,
    pub content_sha256: String,
    pub supersedes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub test_count: usize,
    pub failure_count: usize,
    pub stdout: String,
}

pub struct WorktreeSandbox {
    base_dir: PathBuf,
}

impl WorktreeSandbox {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Allocates an isolated task worktree path
    pub fn allocate_task_path(&self, task_id: &str) -> PathBuf {
        self.base_dir.join(format!("worktree_task_{}", task_id))
    }

    /// Computes immutable SHA-256 hash for a patch diff string
    pub fn hash_patch(diff: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(diff.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Packages staged changes into an immutable PatchSet
    pub fn create_patch_set(
        worktree_branch: &str,
        changed_files: Vec<String>,
        diff_content: String,
        supersedes: Option<String>,
    ) -> PatchSet {
        let content_sha256 = Self::hash_patch(&diff_content);
        let patch_id = format!("patch_{}", &content_sha256[..12]);
        PatchSet {
            patch_id,
            worktree_branch: worktree_branch.to_string(),
            changed_files,
            diff_content,
            content_sha256,
            supersedes,
        }
    }

    /// Validates test output and returns ValidationResult
    pub fn evaluate_test_output(test_output: &str, exit_code: i32) -> ValidationResult {
        let passed = exit_code == 0 && !test_output.contains("FAILED") && !test_output.contains("test result: FAILED");
        let test_count = test_output.matches("test ").count().max(1);
        let failure_count = if passed { 0 } else { test_output.matches("FAILED").count().max(1) };

        ValidationResult {
            passed,
            test_count,
            failure_count,
            stdout: test_output.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_patch_set_creation() {
        let diff = "+fn calculate() -> i32 { 42 }\n";
        let patch = WorktreeSandbox::create_patch_set(
            "task/feature-x",
            vec!["src/calc.rs".into()],
            diff.into(),
            None,
        );

        assert!(!patch.patch_id.is_empty());
        assert_eq!(patch.content_sha256.len(), 64);
        assert_eq!(patch.changed_files.len(), 1);

        let val = WorktreeSandbox::evaluate_test_output("test result: ok. 5 passed", 0);
        assert!(val.passed);
        assert_eq!(val.failure_count, 0);
    }
}
