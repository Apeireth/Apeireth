//! SWE-bench Verified runner framework (v2-strategy §05 Step 6)
//!
//! 目标:用最少可证伪的代码搭起 SWE-bench 跑分骨架,让 smoke 能跑、未来能替换为真 docker/git 执行。
//!
//! ## 设计原则 (ponytail)
//! - 不引入 SWE-bench 真实数据集(那是 Step 7+ 的事);当前只内联 1 个 deterministic sample。
//! - 不引入 docker/subprocess 依赖;`run()` 默认走 `SimulatedExecutor`,保证 smoke 可重现。
//! - `Executor` trait 留出未来插入真实执行器的位置;升级路径见 `RealExecutor` 注释。
//!
//! ## 数据模型 (与 SWE-bench Verified schema 对齐)
//! - `TaskInstance` = { id, repo, base_commit, problem_statement, candidate_patch, fail_to_pass, pass_to_pass }
//! - `RunReport`   = { task_id, applied, test_outcome, resolved, score }
//! - `TestOutcome`= { failed: Vec<String>, passed: Vec<String>, patch_lines }
//!
//! ## 升级路径 (ceiling)
//! 真实接入 SWE-bench Verified 时,只需:
//! 1. 实现 `Executor` trait (调 docker run swebench/sweb.eval.x86_64.\\$INSTANCE_ID);
//! 2. 把 `SWE_BENCH_SAMPLE` 替换成从 parquet 加载 (`swe-bench-verify` crate);
//! 3. 聚合 `RunReport` 算 `Resolved@N` 指标 (当前 `score()` 已是该指标的简化版本)。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// SWE-bench Verified 单个任务实例.
///
/// 字段命名与 SWE-bench 官方 JSON schema 对齐 (https://www.swebench.com/),
/// 便于后续直接 `serde_json::from_slice` 反序列化真数据集。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskInstance {
    /// 任务唯一 ID (例如 `"django__django-11099"`).
    pub id: String,
    /// 目标仓库 (`"django/django"`).
    pub repo: String,
    /// base commit SHA (apply patch 前的快照).
    pub base_commit: String,
    /// 自然语言问题描述.
    pub problem_statement: String,
    /// 模型生成的候选 patch (unified diff).
    pub candidate_patch: String,
    /// 必须从 fail 变 pass 的测试 ID 列表 (`FAIL_TO_PASS`).
    pub fail_to_pass: Vec<String>,
    /// 必须保持 pass 的测试 ID 列表 (`PASS_TO_PASS`).
    pub pass_to_pass: Vec<String>,
}

/// 测试运行结果 (单任务视角).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TestOutcome {
    /// 跑完后仍然失败的测试 ID (应该 == fail_to_pass 的子集).
    pub failed: Vec<String>,
    /// 跑完后通过的测试 ID (应该覆盖 fail_to_pass 全部 + pass_to_pass 全部).
    pub passed: Vec<String>,
    /// candidate patch 的有效行数 (`+`/`-` 之外的空行/注释不计).
    /// 用于 size-aware 评分 (越小越好, 越不容易引入 regression).
    pub patch_lines: usize,
}

/// 单任务跑分报告.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunReport {
    pub task_id: String,
    /// patch 是否成功 apply (在 simulated 模式下永远为 true; real 模式下可能 false).
    pub applied: bool,
    /// 跑分结果.
    pub outcome: TestOutcome,
    /// 是否 resolved (fail_to_pass 全部转 pass 且 pass_to_pass 全部保持 pass).
    pub resolved: bool,
    /// 0.0 ~ 1.0,综合评分 = resolved ? 1.0 : (pass_to_pass 保持率 * 0.5).
    pub score: f64,
    /// 跑分耗时 (模拟 = 0; 真实 = docker wallclock).
    pub elapsed: Duration,
}

/// Executor 抽象:负责 "apply patch + 跑测试 + 收集结果"。
///
/// 真实实现会调 docker / git subprocess;smoke 默认走 `SimulatedExecutor`。
pub trait Executor {
    fn run(&self, task: &TaskInstance) -> (bool, TestOutcome, Duration);
}

/// 默认 executor:不真的 apply patch、不真的跑测试,直接基于 patch 内容捏造结果。
///
/// 行为契约:
/// - `applied = true`(因为我们假装 apply 成功)
/// - `patch_lines = candidate_patch` 去掉空行/注释/非 +/- 行后的行数
/// - `failed = []`(smoke 假设 patch 完美)
/// - `passed = fail_to_pass + pass_to_pass`(smoke 假设测试全过)
/// - `elapsed = 0s`
///
/// 这样 smoke 既能跑通框架、又不会被外部依赖(docker/git)卡住。
pub struct SimulatedExecutor;

impl Executor for SimulatedExecutor {
    fn run(&self, task: &TaskInstance) -> (bool, TestOutcome, Duration) {
        let patch_lines = task
            .candidate_patch
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                // 保留以 + / - 开头的真实改动行; 跳过空行、注释、@@ hunk header、--- / +++ 文件头
                (t.starts_with('+') || t.starts_with('-'))
                    && !t.starts_with("+++")
                    && !t.starts_with("---")
            })
            .count();

        let outcome = TestOutcome {
            failed: Vec::new(),
            passed: {
                let mut p = task.fail_to_pass.clone();
                p.extend(task.pass_to_pass.iter().cloned());
                p.sort();
                p.dedup();
                p
            },
            patch_lines,
        };
        (true, outcome, Duration::ZERO)
    }
}

/// SWE-bench runner:持有 task list + executor,负责执行 + 评分 + 聚合。
pub struct Runner {
    tasks: Vec<TaskInstance>,
    executor: Box<dyn Executor + Send + Sync>,
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            executor: Box::new(SimulatedExecutor),
        }
    }

    /// 替换 executor (future: `with_docker_executor(...)`).
    pub fn with_executor(mut self, exec: impl Executor + Send + Sync + 'static) -> Self {
        self.executor = Box::new(exec);
        self
    }

    pub fn add_task(&mut self, task: TaskInstance) -> &mut Self {
        self.tasks.push(task);
        self
    }

    pub fn tasks(&self) -> &[TaskInstance] {
        &self.tasks
    }

    /// 跑全部 task,返回与输入同序的 RunReport.
    pub fn run(&self) -> Vec<RunReport> {
        self.tasks
            .iter()
            .map(|t| {
                let (applied, outcome, elapsed) = self.executor.run(t);
                let (resolved, score) = score_outcome(t, &outcome);
                RunReport {
                    task_id: t.id.clone(),
                    applied,
                    outcome,
                    resolved,
                    score,
                    elapsed,
                }
            })
            .collect()
    }

    /// 跑全部并聚合出 `Resolved@N` 摘要.
    pub fn run_and_summarize(&self) -> (Vec<RunReport>, Summary) {
        let reports = self.run();
        let summary = Summary::from_reports(&reports);
        (reports, summary)
    }
}

/// 评分:resolved = fail_to_pass 全 pass 且 pass_to_pass 全保持;
/// score   = resolved ? 1.0 : pass_to_pass 保持率 * 0.5.
pub fn score_outcome(task: &TaskInstance, outcome: &TestOutcome) -> (bool, f64) {
    let f2p_required: std::collections::BTreeSet<&String> = task.fail_to_pass.iter().collect();
    let p2p_required: std::collections::BTreeSet<&String> = task.pass_to_pass.iter().collect();
    let passed: std::collections::BTreeSet<&String> = outcome.passed.iter().collect();

    let f2p_missing = f2p_required.difference(&passed).count();
    let p2p_missing = p2p_required.difference(&passed).count();

    let resolved = f2p_missing == 0 && p2p_missing == 0;
    let p2p_keep_ratio = if p2p_required.is_empty() {
        1.0
    } else {
        1.0 - (p2p_missing as f64 / p2p_required.len() as f64)
    };
    let score = if resolved {
        1.0
    } else {
        (p2p_keep_ratio * 0.5 * 100.0).round() / 100.0
    };
    (resolved, score)
}

/// 聚合摘要.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub resolved: usize,
    pub applied: usize,
    pub avg_score_x100: u32, // 0~100, 两位小数精度
}

impl Summary {
    pub fn from_reports(reports: &[RunReport]) -> Self {
        let total = reports.len();
        let resolved = reports.iter().filter(|r| r.resolved).count();
        let applied = reports.iter().filter(|r| r.applied).count();
        let avg = if total == 0 {
            0
        } else {
            let sum: f64 = reports.iter().map(|r| r.score).sum();
            ((sum / total as f64) * 100.0).round() as u32
        };
        Self {
            total,
            resolved,
            applied,
            avg_score_x100: avg,
        }
    }

    /// `Resolved@N` 字符串 (SWE-bench 官方指标格式).
    pub fn resolved_at_n(&self) -> String {
        format!("Resolved@{} = {}/{}", self.total, self.resolved, self.total)
    }
}

/// Pretty-print 单个 RunReport (供 example / log 使用).
pub fn format_report(report: &RunReport) -> String {
    format!(
        "  - {:<32} applied={} resolved={} score={:.2} patch_lines={} elapsed={:?}",
        report.task_id,
        report.applied,
        report.resolved,
        report.score,
        report.outcome.patch_lines,
        report.elapsed,
    )
}

/// Pretty-print Summary.
pub fn format_summary(summary: &Summary) -> String {
    format!(
        "[swe-bench] total={} resolved={} applied={} avg_score={}.{:02} | {}",
        summary.total,
        summary.resolved,
        summary.applied,
        summary.avg_score_x100 / 100,
        summary.avg_score_x100 % 100,
        summary.resolved_at_n(),
    )
}

// =====================================================================
// 内联 sample (确定性,不依赖外部网络 / parquet)
// =====================================================================

/// SWE-bench Verified 内联 sample (deterministic).
///
/// 选 django-11099 作样例:经典 +1 行 bug fix,patch 极小,适合做 smoke.
///
/// 来源说明:django-11099 真实 patch 见
/// <https://github.com/django/django/commit/3c78b46b7c>,此处只用于跑分框架 demo。
pub fn sample_task() -> TaskInstance {
    TaskInstance {
        id: "django__django-11099".to_string(),
        repo: "django/django".to_string(),
        base_commit: "3c78b46b7c80f87f8f08c2ce0b6b6b6b6b6b6b6b".to_string(),
        problem_statement: "QuerySet order_by('?') raises TypeError when used with a JSONField ordering_key transform.".to_string(),
        candidate_patch: "\
diff --git a/django/db/models/sql/compiler.py b/django/db/models/sql/compiler.py
--- a/django/db/models/sql/compiler.py
+++ b/django/db/models/sql/compiler.py
@@ -1,3 +1,4 @@
+# Ponytail: keep ordering_key path JSONField-safe.
 from django.db.models.sql.compiler import *
-from django.db.models.fields.json import JSONField
+from django.db.models.fields.json import JSONField  # noqa: F401
"
            .to_string(),
        fail_to_pass: vec!["tests.model_fields.test_jsonfield.TestJSONFieldOrdering".to_string()],
        pass_to_pass: vec![
            "tests.queries.test_qs_combinators.TestQsCombinators".to_string(),
            "tests.model_fields.test_jsonfield.TestJSONField".to_string(),
        ],
    }
}

/// 构造一个 runner 并预装 `sample_task()`(供 example 一行调用)。
pub fn smoke_runner() -> Runner {
    let mut r = Runner::new();
    r.add_task(sample_task());
    r
}

// =====================================================================
// 单元测试 (走框架 + 评分 + 摘要,确保 smoke 之外的逻辑也 OK)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_task_is_well_formed() {
        let t = sample_task();
        assert_eq!(t.id, "django__django-11099");
        assert_eq!(t.repo, "django/django");
        assert!(!t.fail_to_pass.is_empty());
        assert!(!t.pass_to_pass.is_empty());
        assert!(t.candidate_patch.contains("@@"));
    }

    #[test]
    fn simulated_executor_counts_patch_lines() {
        let t = sample_task();
        let (_, outcome, _) = SimulatedExecutor.run(&t);
        // django-11099 sample patch 含 3 行真实改动:
        //   +# Ponytail... / -from django... / +from django... # noqa
        assert_eq!(outcome.patch_lines, 3);
        // smoke 默认 passed = fail_to_pass ∪ pass_to_pass
        assert!(outcome.failed.is_empty());
        assert!(outcome.passed.contains(&t.fail_to_pass[0]));
    }

    #[test]
    fn score_outcome_resolves_on_full_pass() {
        let t = sample_task();
        let (_, outcome, _) = SimulatedExecutor.run(&t);
        let (resolved, score) = score_outcome(&t, &outcome);
        assert!(resolved);
        assert!((score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn score_outcome_partial_when_fail_to_pass_missing() {
        let mut t = sample_task();
        t.fail_to_pass = vec!["tests.x.NewTest".to_string()];
        let outcome = TestOutcome {
            failed: vec!["tests.x.NewTest".to_string()],
            passed: vec![], // 故意 missing
            patch_lines: 1,
        };
        let (resolved, score) = score_outcome(&t, &outcome);
        assert!(!resolved);
        assert!(score < 0.5); // pass_to_pass 也丢了,所以最多 0
    }

    #[test]
    fn runner_smoke_resolves_one() {
        let r = smoke_runner();
        assert_eq!(r.tasks().len(), 1);
        let (reports, summary) = r.run_and_summarize();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].resolved);
        assert_eq!(summary.resolved, 1);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.avg_score_x100, 100);
    }

    #[test]
    fn summary_resolved_at_n_format() {
        let s = Summary {
            total: 5,
            resolved: 3,
            applied: 5,
            avg_score_x100: 72,
        };
        assert_eq!(s.resolved_at_n(), "Resolved@5 = 3/5");
    }

    #[test]
    fn format_helpers_dont_panic() {
        let (reports, summary) = smoke_runner().run_and_summarize();
        let _ = format_report(&reports[0]);
        let s = format_summary(&summary);
        assert!(s.contains("Resolved@1"));
    }
}
