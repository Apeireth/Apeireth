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

// ===========================================================================
// W2 §4.3 (2026-10-10): WorktreeSandboxedOrchestrator — worktree 隔离调度装饰器
// ===========================================================================

use crate::{Orchestrator, OrchestratorError, SubagentOutcome, SubagentSpec};

/// worktree 命令执行器 (生产 = 真 `git`; 测试 = 记录型假执行器, 0 git 依赖)。
pub type CommandRunner = std::sync::Arc<dyn Fn(&[String]) -> Result<(), String> + Send + Sync>;

/// **worktree 隔离调度装饰器** (W2 §4.3 落点 = subagent/Orchestrator):
/// 包任意 [`Orchestrator`] ——
/// - `dispatch` 前建独立 git worktree (物理目录级隔离, 防并发/长程 subagent 污染
///   主工作区);
/// - 把 worktree 路径注入 `spec.payload["worktree"]` (subagent 以该目录为工作区,
///   非破坏性扩展字段);
/// - 完成后移除 worktree。**清理失败只降级不枪毙**: outcome 主体不受影响, 仅在
///   output 的 `_worktree_cleanup_failed` 留痕; 失败路径同样清理 (残骸不留)。
///
/// **0 假装边界**: `Orchestrator` 生产实现与调用方是独立工作项 (trait 就绪, 本装饰器
/// 把隔离层接好后等主角 —— 见台账 W2 §4.3); `TddStateMachine` / `RateLimitBackoff`
/// 为驱动侧库工具 (状态机自证), 未接 dispatch 协议 (协议扩展属后续)。
pub struct WorktreeSandboxedOrchestrator<O: Orchestrator> {
    inner: O,
    repo_root: std::path::PathBuf,
    runner: CommandRunner,
}

impl<O: Orchestrator> WorktreeSandboxedOrchestrator<O> {
    /// 包装 inner orchestrator; `runner` 执行 `git <args...>` 形态命令。
    pub fn new(inner: O, repo_root: impl Into<std::path::PathBuf>, runner: CommandRunner) -> Self {
        Self {
            inner,
            repo_root: repo_root.into(),
            runner,
        }
    }

    fn run(&self, args: &[String]) -> Result<(), String> {
        (self.runner)(args)
    }
}

#[async_trait::async_trait]
impl<O: Orchestrator> Orchestrator for WorktreeSandboxedOrchestrator<O> {
    async fn start(&mut self) -> Result<(), OrchestratorError> {
        self.inner.start().await
    }

    async fn stop(&mut self) -> Result<(), OrchestratorError> {
        self.inner.stop().await
    }

    async fn dispatch(&self, mut spec: SubagentSpec) -> Result<SubagentOutcome, OrchestratorError> {
        let config = WorktreeConfig::new(
            &self.repo_root,
            spec.id.clone(),
            format!("apeireth/worktree-{}", spec.id),
        )
        .map_err(|e| OrchestratorError::Io(format!("worktree config invalid: {e}")))?;

        // ① 建 worktree (失败即拒 —— 隔离层不裸跑)。
        self.run(&config.create_command_args())
            .map_err(|e| OrchestratorError::Io(format!("worktree create failed: {e}")))?;

        // ② 注入 worktree 路径 (payload 非破坏性扩展字段)。
        if let Some(obj) = spec.payload.as_object_mut() {
            obj.insert(
                "worktree".to_string(),
                serde_json::Value::String(config.worktree_path.to_string_lossy().to_string()),
            );
        }

        // ③④ 内层 dispatch + 清理 (成败两路都清; 清理失败只降级)。
        match self.inner.dispatch(spec).await {
            Ok(mut done) => {
                if self.run(&config.remove_command_args()).is_err() {
                    if let Some(obj) = done.output.as_object_mut() {
                        obj.insert(
                            "_worktree_cleanup_failed".to_string(),
                            serde_json::Value::Bool(true),
                        );
                    }
                }
                Ok(done)
            }
            Err(e) => {
                let _ = self.run(&config.remove_command_args());
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod worktree_dispatch_tests {
    use super::*;
    use crate::SubagentRole;

    fn spec(id: &str) -> SubagentSpec {
        SubagentSpec {
            id: id.to_string(),
            role: SubagentRole::Reviewer,
            title: "t".to_string(),
            payload: serde_json::json!({"goal": "demo"}),
            model: None,
            system_prompt: None,
            require_human_approval: false,
        }
    }

    struct FakeOrchestrator {
        seen: std::sync::Mutex<Vec<serde_json::Value>>,
    }

    #[async_trait::async_trait]
    impl Orchestrator for FakeOrchestrator {
        async fn start(&mut self) -> Result<(), OrchestratorError> {
            Ok(())
        }
        async fn stop(&mut self) -> Result<(), OrchestratorError> {
            Ok(())
        }
        async fn dispatch(&self, spec: SubagentSpec) -> Result<SubagentOutcome, OrchestratorError> {
            self.seen
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(spec.payload.clone());
            Ok(SubagentOutcome {
                spec_id: spec.id,
                role: spec.role,
                output: serde_json::json!({"ok": true}),
                success: true,
                error: None,
                completed_at: 0,
            })
        }
    }

    fn recording_runner(
        fail_remove: bool,
    ) -> (CommandRunner, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        let log: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log2 = std::sync::Arc::clone(&log);
        let runner: CommandRunner = std::sync::Arc::new(move |args: &[String]| {
            log2.lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(args.join(" "));
            if fail_remove && args.get(1).map(String::as_str) == Some("remove") {
                return Err("forced cleanup failure".to_string());
            }
            Ok(())
        });
        (runner, log)
    }

    #[tokio::test]
    async fn decorator_creates_injects_and_removes_worktree() {
        // W2 五件验收门③: 效果可见 = create 先行、payload 带 worktree、remove 收尾。
        let inner = FakeOrchestrator {
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let (runner, log) = recording_runner(false);
        let orch = WorktreeSandboxedOrchestrator::new(inner, "/repo", runner);

        let out = orch.dispatch(spec("t1")).await.expect("dispatch");
        assert!(out.success);
        let seen = orch
            .inner
            .seen
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        assert_eq!(seen.len(), 1);
        let worktree = seen[0]["worktree"].as_str().expect("payload 注入 worktree");
        assert!(worktree.contains(".worktrees"), "{worktree}");

        let calls = log.lock().unwrap_or_else(|p| p.into_inner()).clone();
        assert_eq!(calls.len(), 2, "create + remove 各一次: {calls:?}");
        assert!(calls[0].starts_with("worktree add"), "{calls:?}");
        assert!(calls[1].starts_with("worktree remove"), "{calls:?}");
        assert!(
            out.output["_worktree_cleanup_failed"].is_null(),
            "清理成功不误报"
        );
    }

    #[tokio::test]
    async fn create_failure_fails_closed_without_dispatch() {
        // W2 五件验收门② 对偶: 建不出 worktree = 拒绝执行 (不裸跑)。
        let inner = FakeOrchestrator {
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let log: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let runner: CommandRunner =
            std::sync::Arc::new(|_args: &[String]| Err("no git".to_string()));
        let orch = WorktreeSandboxedOrchestrator::new(inner, "/repo", runner);
        assert!(orch.dispatch(spec("t2")).await.is_err());
        assert!(
            orch.inner
                .seen
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .is_empty(),
            "create 失败不得进入 dispatch"
        );
        let _ = log;
    }

    #[tokio::test]
    async fn cleanup_failure_only_degrades() {
        // 只降级不枪毙: 清理失败不改变 outcome 主体, 仅留痕。
        let inner = FakeOrchestrator {
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let (runner, _log) = recording_runner(true);
        let orch = WorktreeSandboxedOrchestrator::new(inner, "/repo", runner);

        let out = orch.dispatch(spec("t3")).await.expect("dispatch");
        assert!(out.success, "清理失败不得枪毙主结果");
        assert_eq!(
            out.output["_worktree_cleanup_failed"],
            serde_json::Value::Bool(true),
            "失败须留痕"
        );
    }
}
