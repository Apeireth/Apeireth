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

    /// Executes live Git worktree isolated engineering task
    pub async fn run_live_worktree_pipeline(
        repo_path: &str,
        task_id: &str,
        files_to_write: std::collections::HashMap<String, String>,
        test_cmd: Option<&str>,
    ) -> Result<(PatchSet, ValidationResult), String> {
        let branch = format!("factory/{}", task_id);
        let temp_dir = std::env::temp_dir().join(format!("apeireth_wt_{}", task_id));

        if temp_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        }

        // 1. git worktree add
        let add_out = tokio::process::Command::new("git")
            .current_dir(repo_path)
            .args(&["worktree", "add", "-B", &branch, temp_dir.to_str().unwrap(), "HEAD"])
            .output()
            .await
            .map_err(|e| format!("Failed to spawn git worktree add: {}", e))?;

        if !add_out.status.success() {
            let stderr = String::from_utf8_lossy(&add_out.stderr);
            return Err(format!("git worktree add failed: {}", stderr));
        }

        // 2. Write synthesized files
        let mut changed_files = Vec::new();
        for (rel_path, content) in files_to_write {
            let target_path = temp_dir.join(&rel_path);
            if let Some(parent) = target_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            let _ = tokio::fs::write(&target_path, content).await;
            changed_files.push(rel_path);
        }

        // 3. Run validation test command
        let val_result = if let Some(cmd) = test_cmd {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                let test_proc = tokio::process::Command::new(parts[0])
                    .current_dir(&temp_dir)
                    .args(&parts[1..])
                    .output()
                    .await;
                match test_proc {
                    Ok(out) => {
                        let stdout_str = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
                        Self::evaluate_test_output(&stdout_str, out.status.code().unwrap_or(-1))
                    }
                    Err(e) => Self::evaluate_test_output(&format!("Failed to run test command: {}", e), -1),
                }
            } else {
                Self::evaluate_test_output("no test command executed", 0)
            }
        } else {
            Self::evaluate_test_output("no test command requested", 0)
        };

        // 4. Extract real git diff
        let diff_out = tokio::process::Command::new("git")
            .current_dir(&temp_dir)
            .args(&["diff"])
            .output()
            .await;

        let diff_str = match diff_out {
            Ok(d) => String::from_utf8_lossy(&d.stdout).to_string(),
            Err(_) => String::new(),
        };

        // 5. Clean up worktree directory
        let _ = tokio::process::Command::new("git")
            .current_dir(repo_path)
            .args(&["worktree", "remove", temp_dir.to_str().unwrap(), "--force"])
            .output()
            .await;

        let patch_set = Self::create_patch_set(&branch, changed_files, diff_str, None);
        Ok((patch_set, val_result))
    }

    /// Applies a live patch onto the repository
    pub async fn apply_live_patch(repo_path: &str, diff_content: &str) -> Result<String, String> {
        let temp_patch = std::env::temp_dir().join(format!("patch_{}.diff", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_patch, diff_content).await.map_err(|e| format!("Failed to write patch file: {}", e))?;

        let apply_out = tokio::process::Command::new("git")
            .current_dir(repo_path)
            .args(&["apply", temp_patch.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| format!("Failed to run git apply: {}", e))?;

        let _ = tokio::fs::remove_file(&temp_patch).await;

        if apply_out.status.success() {
            Ok("Patch applied cleanly to repository".into())
        } else {
            Err(format!("git apply failed: {}", String::from_utf8_lossy(&apply_out.stderr)))
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
