//! gnhf-inspired Autonomous Git Worktree Sandbox & TDD State Machine.
//!
//! # Architecture
//!
//! Provides physical directory-level worktree isolation for concurrent or long-running
//! subagents, preventing workspace pollution. Implements a strict Test-Driven
//! Development (TDD) loop state machine (`Edit -> Test -> Commit on Pass / Hard Reset on Fail`)
//! and exponential backoff rate-limit recovery.
//!
//! Pure Safe Rust (`#![forbid(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Errors related to worktree sandbox operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeError {
    InvalidConfig(String),
    IllegalTransition(TddPhase, TddPhase),
    TestFailed(String),
    Execution(String),
}

impl fmt::Display for WorktreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid worktree configuration: {msg}"),
            Self::IllegalTransition(from, to) => {
                write!(f, "illegal state transition from {from:?} to {to:?}")
            }
            Self::TestFailed(msg) => write!(f, "verification test failed: {msg}"),
            Self::Execution(msg) => write!(f, "worktree execution error: {msg}"),
        }
    }
}

impl std::error::Error for WorktreeError {}

/// TDD Verification Cycle Phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TddPhase {
    /// Worktree initialized and isolated.
    Initialized,
    /// Agent is modifying code.
    Editing,
    /// Automated test/check verification running.
    Testing,
    /// Tests passed successfully.
    Passed,
    /// Tests failed.
    Failed,
    /// Successful commit created on branch.
    Committed,
    /// Hard reset / rollback executed to clean working tree.
    RolledBack,
}

/// Configuration for an isolated worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub repo_root: PathBuf,
    pub worktree_name: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
}

impl WorktreeConfig {
    pub fn new(
        repo_root: impl Into<PathBuf>,
        worktree_name: impl Into<String>,
        branch_name: impl Into<String>,
    ) -> Result<Self, WorktreeError> {
        let repo_root = repo_root.into();
        let worktree_name = worktree_name.into();
        let branch_name = branch_name.into();

        if worktree_name.trim().is_empty() {
            return Err(WorktreeError::InvalidConfig(
                "worktree name cannot be empty".to_string(),
            ));
        }
        if branch_name.trim().is_empty() {
            return Err(WorktreeError::InvalidConfig(
                "branch name cannot be empty".to_string(),
            ));
        }

        let worktree_path = repo_root.join(".worktrees").join(&worktree_name);
        Ok(Self {
            repo_root,
            worktree_name,
            branch_name,
            worktree_path,
        })
    }

    /// Generates the canonical git command arguments to create the worktree.
    pub fn create_command_args(&self) -> Vec<String> {
        vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            self.branch_name.clone(),
            self.worktree_path.to_string_lossy().to_string(),
        ]
    }

    /// Generates the canonical git command arguments to remove the worktree.
    pub fn remove_command_args(&self) -> Vec<String> {
        vec![
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            self.worktree_path.to_string_lossy().to_string(),
        ]
    }
}

/// State machine coordinating TDD verification and fail-safe rollbacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TddStateMachine {
    pub config: WorktreeConfig,
    pub current_phase: TddPhase,
    pub iteration_count: usize,
    pub consecutive_failures: usize,
    pub last_test_output: Option<String>,
}

impl TddStateMachine {
    pub fn new(config: WorktreeConfig) -> Self {
        Self {
            config,
            current_phase: TddPhase::Initialized,
            iteration_count: 0,
            consecutive_failures: 0,
            last_test_output: None,
        }
    }

    /// Starts an edit iteration.
    pub fn begin_edit(&mut self) -> Result<(), WorktreeError> {
        match self.current_phase {
            TddPhase::Initialized | TddPhase::Committed | TddPhase::RolledBack => {
                self.current_phase = TddPhase::Editing;
                self.iteration_count += 1;
                Ok(())
            }
            phase => Err(WorktreeError::IllegalTransition(phase, TddPhase::Editing)),
        }
    }

    /// Transitions from Editing to Testing.
    pub fn begin_testing(&mut self) -> Result<(), WorktreeError> {
        if self.current_phase != TddPhase::Editing {
            return Err(WorktreeError::IllegalTransition(
                self.current_phase,
                TddPhase::Testing,
            ));
        }
        self.current_phase = TddPhase::Testing;
        Ok(())
    }

    /// Records test results. If passed, transitions to Passed; if failed, to Failed.
    pub fn record_test_result(
        &mut self,
        passed: bool,
        output: String,
    ) -> Result<TddPhase, WorktreeError> {
        if self.current_phase != TddPhase::Testing {
            return Err(WorktreeError::IllegalTransition(
                self.current_phase,
                if passed {
                    TddPhase::Passed
                } else {
                    TddPhase::Failed
                },
            ));
        }

        self.last_test_output = Some(output);
        if passed {
            self.current_phase = TddPhase::Passed;
            self.consecutive_failures = 0;
        } else {
            self.current_phase = TddPhase::Failed;
            self.consecutive_failures += 1;
        }
        Ok(self.current_phase)
    }

    /// Commits changes on test pass.
    pub fn commit_on_pass(&mut self, _commit_msg: &str) -> Result<Vec<String>, WorktreeError> {
        if self.current_phase != TddPhase::Passed {
            return Err(WorktreeError::IllegalTransition(
                self.current_phase,
                TddPhase::Committed,
            ));
        }
        self.current_phase = TddPhase::Committed;
        Ok(vec![
            "commit".to_string(),
            "-am".to_string(),
            _commit_msg.to_string(),
        ])
    }

    /// Rolls back working copy to clean state on failure (`git reset --hard`).
    pub fn rollback_on_fail(&mut self) -> Result<Vec<String>, WorktreeError> {
        if self.current_phase != TddPhase::Failed {
            return Err(WorktreeError::IllegalTransition(
                self.current_phase,
                TddPhase::RolledBack,
            ));
        }
        self.current_phase = TddPhase::RolledBack;
        Ok(vec![
            "reset".to_string(),
            "--hard".to_string(),
            "HEAD".to_string(),
        ])
    }
}

/// Exponential backoff rate limit sleep window manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitBackoff {
    pub initial_delay_ms: u64,
    pub multiplier: f64,
    pub max_delay_ms: u64,
    pub current_delay_ms: u64,
    pub retry_count: usize,
}

impl RateLimitBackoff {
    pub fn new(initial_delay_ms: u64, multiplier: f64, max_delay_ms: u64) -> Self {
        Self {
            initial_delay_ms,
            multiplier,
            max_delay_ms,
            current_delay_ms: initial_delay_ms,
            retry_count: 0,
        }
    }

    /// Computes the next sleep duration in milliseconds and advances backoff state.
    pub fn next_delay(&mut self) -> u64 {
        let delay = self.current_delay_ms;
        self.retry_count += 1;
        self.current_delay_ms =
            ((self.current_delay_ms as f64 * self.multiplier) as u64).min(self.max_delay_ms);
        delay
    }

    /// Resets the backoff to initial state on successful request.
    pub fn reset(&mut self) {
        self.current_delay_ms = self.initial_delay_ms;
        self.retry_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_config_commands() {
        let config = WorktreeConfig::new(
            Path::new("/workspace"),
            "agent_feature_x",
            "feature/x_patch",
        )
        .unwrap();

        let add_args = config.create_command_args();
        assert_eq!(add_args[0], "worktree");
        assert_eq!(add_args[1], "add");
        assert_eq!(add_args[2], "-b");
        assert_eq!(add_args[3], "feature/x_patch");

        let rm_args = config.remove_command_args();
        assert_eq!(rm_args[0], "worktree");
        assert_eq!(rm_args[1], "remove");
        assert_eq!(rm_args[2], "--force");
    }

    #[test]
    fn test_tdd_state_machine_success_flow() {
        let config = WorktreeConfig::new(Path::new("/workspace"), "test_wt", "branch_wt").unwrap();

        let mut sm = TddStateMachine::new(config);
        assert_eq!(sm.current_phase, TddPhase::Initialized);

        sm.begin_edit().unwrap();
        assert_eq!(sm.current_phase, TddPhase::Editing);
        assert_eq!(sm.iteration_count, 1);

        sm.begin_testing().unwrap();
        assert_eq!(sm.current_phase, TddPhase::Testing);

        let phase = sm
            .record_test_result(true, "All 10 tests passed".to_string())
            .unwrap();
        assert_eq!(phase, TddPhase::Passed);

        let commit_args = sm.commit_on_pass("feat: complete feature X").unwrap();
        assert_eq!(commit_args[0], "commit");
        assert_eq!(sm.current_phase, TddPhase::Committed);
    }

    #[test]
    fn test_tdd_state_machine_fail_and_rollback_flow() {
        let config = WorktreeConfig::new(Path::new("/workspace"), "test_wt", "branch_wt").unwrap();

        let mut sm = TddStateMachine::new(config);
        sm.begin_edit().unwrap();
        sm.begin_testing().unwrap();

        let phase = sm
            .record_test_result(false, "Syntax error at line 42".to_string())
            .unwrap();
        assert_eq!(phase, TddPhase::Failed);
        assert_eq!(sm.consecutive_failures, 1);

        let reset_args = sm.rollback_on_fail().unwrap();
        assert_eq!(reset_args[0], "reset");
        assert_eq!(reset_args[1], "--hard");
        assert_eq!(sm.current_phase, TddPhase::RolledBack);

        // Can resume next edit from clean rollback state
        sm.begin_edit().unwrap();
        assert_eq!(sm.current_phase, TddPhase::Editing);
        assert_eq!(sm.iteration_count, 2);
    }

    #[test]
    fn test_rate_limit_backoff() {
        let mut backoff = RateLimitBackoff::new(1000, 2.0, 8000);
        assert_eq!(backoff.next_delay(), 1000);
        assert_eq!(backoff.next_delay(), 2000);
        assert_eq!(backoff.next_delay(), 4000);
        assert_eq!(backoff.next_delay(), 8000);
        assert_eq!(backoff.next_delay(), 8000); // capped at max_delay_ms

        backoff.reset();
        assert_eq!(backoff.next_delay(), 1000);
    }
}
