//! R32-3: Eval smoke test — 真接 1 个 task
//!
//! **目标**: apeireth-eval 借 R32-2 `tool_loop` + R33-1 `conventions_scanner` 真跑 1 个
//! smoke task, 验证:
//! - 启动时扫描 workspace Cargo.toml, 抽 edition / lints / deps
//! - 借 `run_tool_loop` 状态机走"有 tool → 解析 → 最终"3 步 (stub F, 0 真 LLM)
//! - 输出 7 阶段 metric (R33-1 system prompt block 注入验证)
//!
//! **7 阶段 metric** (借鉴 OpenAI Evals 7 阶段 + Anthropic Evals 7 阶段 + 实战 R23 6 module):
//! 1. `setup_ok` — scan workspace root + parse Cargo.toml (0 error)
//! 2. `prompt_built` — Aider-style system block 真拼成 (含 edition / rust-version / lints / deps)
//! 3. `tool_loop_init` — R32-2 `ToolLoopState` 构造成功 (max_turns=3, 跟 R30 兼容)
//! 4. `tool_call_dispatched` — stub F 返 tool call, `run_tool_loop` 状态机正确走第 1 轮
//! 5. `tool_result_digested` — 第 2 轮 stub F 返 final_answer, `should_continue` 判定停
//! 6. `final_reply` — `state.last_reply` = 期望内容
//! 7. `no_regression` — 0 panic, 0 error, 7/7 metric 都过
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 引入网络/LLM 依赖 (stub F 内置, 0 LLM 真接)
//! - 0 改 R32-2 tool_loop / R33-1 conventions_scanner (只 import 复用)
//! - 0 触碰 R23 6 module (EvalScore / mean / stddev / percentile / weighted_mean / is_valid_percentile)

use crate::{mean, percentile, stddev, weighted_mean, EvalScore};
use apeireth_pipeline::tool_loop::{
    run_tool_loop, LlmStepResult, ToolLoopMessage, ToolLoopState, DEFAULT_MAX_TOOL_TURNS,
};
use apeireth_tools::ProjectConventions;
use std::path::Path;

/// 7 阶段 metric
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SmokeReport {
    pub setup_ok: bool,
    pub prompt_built: bool,
    pub tool_loop_init: bool,
    pub tool_call_dispatched: bool,
    pub tool_result_digested: bool,
    pub final_reply_correct: bool,
    pub no_regression: bool,
    /// 拼好的 system prompt block (Aider-style, R33-1)
    pub system_block: String,
    /// 抽到的 conventions (R33-1)
    pub conventions: Option<ProjectConventions>,
    /// tool loop 末态
    pub final_reply: String,
    /// 7 阶段 score (1.0 = pass, 0.0 = fail), 给 R23 6 module eval aggregation 用
    pub phase_scores: Vec<EvalScore>,
}

impl SmokeReport {
    /// 7 阶段全 pass?
    pub fn all_pass(&self) -> bool {
        self.setup_ok
            && self.prompt_built
            && self.tool_loop_init
            && self.tool_call_dispatched
            && self.tool_result_digested
            && self.final_reply_correct
            && self.no_regression
    }
    /// 整体 pass rate (7 阶段算 mean)
    pub fn pass_rate(&self) -> f64 {
        let n = 7.0;
        let pass = [
            self.setup_ok,
            self.prompt_built,
            self.tool_loop_init,
            self.tool_call_dispatched,
            self.tool_result_digested,
            self.final_reply_correct,
            self.no_regression,
        ]
        .iter()
        .filter(|x| **x)
        .count() as f64;
        pass / n
    }
    /// 跟 R23 6 module aggregation 接口对齐
    pub fn to_eval_scores(&self) -> Vec<EvalScore> {
        self.phase_scores.clone()
    }
}

/// 跑 1 个 smoke task: 验证 conventions_scanner + run_tool_loop 集成
///
/// **不真接 LLM**: stub F 内部返 tool call + final_answer, 0 网络, 0 凭证
pub fn run_smoke_conventions_tool_loop(workspace_root: &Path) -> SmokeReport {
    let mut report = SmokeReport::default();

    // 阶段 1: 扫 workspace root
    let conv = ProjectConventions::scan(workspace_root);
    report.conventions = Some(conv.clone());
    report.setup_ok = conv.scan_error.is_none();
    if !report.setup_ok {
        return report;
    }

    // 阶段 2: 拼 Aider-style system block
    let block = conv.to_system_prompt_block();
    report.prompt_built = block.contains("# 项目约定") && block.contains("# 风格提示");
    report.system_block = block;
    if !report.prompt_built {
        return report;
    }

    // 阶段 3: 构造 ToolLoopState
    let initial = ToolLoopState::new(
        "smoke: 跑 1 个 tool_call + final_answer",
        vec![ToolLoopMessage::user("跑 smoke task")],
        DEFAULT_MAX_TOOL_TURNS,
    );
    report.tool_loop_init = initial.turn == 0 && initial.max_turns == 3;
    if !report.tool_loop_init {
        return report;
    }

    // 阶段 4 + 5: 跑 stub F (第一轮 tool call, 第二轮 final answer)
    let expected_final = "smoke task 跑完, conventions 抽到 N 个 deps, tool loop 走 2 轮";
    let final_state = run_tool_loop(initial, |s| {
        if s.turn == 0 {
            // 第一轮: 假 LLM 返 tool call
            LlmStepResult::with_tool_call(
                "我先扫一下项目\n<<<[TOOL_REQUEST]>>>\ntool_name: <<<scan>>>\n<<<[END_TOOL_REQUEST]>>>",
                "[scan OK]\n抽到 12 个 workspace deps\n---",
            )
        } else {
            // 第二轮: 假 LLM 消化 tool 结果, 返 final answer
            LlmStepResult::final_answer(expected_final)
        }
    });
    report.tool_call_dispatched = final_state.turn >= 1;
    report.tool_result_digested = final_state.turn >= 2;
    report.final_reply = final_state.last_reply.clone();
    report.final_reply_correct = final_state.last_reply == expected_final;

    // 阶段 6: 0 panic / 0 error
    report.no_regression = final_state.error.is_none() && report.all_pass_count() >= 6;
    // 阶段 7: 0 退化 (所有 metric 都进 phase_scores)
    report.phase_scores = vec![
        EvalScore::new("setup_ok", if report.setup_ok { 1.0 } else { 0.0 }),
        EvalScore::new("prompt_built", if report.prompt_built { 1.0 } else { 0.0 }),
        EvalScore::new("tool_loop_init", if report.tool_loop_init { 1.0 } else { 0.0 }),
        EvalScore::new("tool_call_dispatched", if report.tool_call_dispatched { 1.0 } else { 0.0 }),
        EvalScore::new("tool_result_digested", if report.tool_result_digested { 1.0 } else { 0.0 }),
        EvalScore::new("final_reply_correct", if report.final_reply_correct { 1.0 } else { 0.0 }),
        EvalScore::new("no_regression", if report.no_regression { 1.0 } else { 0.0 }),
    ];
    report
}

impl SmokeReport {
    fn all_pass_count(&self) -> usize {
        [
            self.setup_ok,
            self.prompt_built,
            self.tool_loop_init,
            self.tool_call_dispatched,
            self.tool_result_digested,
            self.final_reply_correct,
            self.no_regression,
        ]
        .iter()
        .filter(|x| **x)
        .count()
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod smoke_task_tests {
    use super::*;

    /// 跑 smoke task 在当前 workspace root, 验 7 阶段全 pass
    #[test]
    fn smoke_task_workspace_root_7_phase_all_pass() {
        // 向上 2 层: apeireth-eval -> crates -> workspace root
        let pkg_root = std::env::current_dir().unwrap();
        let root = pkg_root.parent().and_then(|p| p.parent()).unwrap_or(&pkg_root);
        let report = run_smoke_conventions_tool_loop(root);

        // 7 阶段期望全 pass (conventions 抽到 + tool loop 2 轮 + final reply 匹配)
        assert!(report.setup_ok, "setup_ok failed: {:?}", report.conventions.as_ref().and_then(|c| c.scan_error.clone()));
        assert!(report.prompt_built, "prompt_built failed: {}", report.system_block.is_empty());
        assert!(report.tool_loop_init);
        assert!(report.tool_call_dispatched, "turn = {}", report.phase_scores.len());
        assert!(report.tool_result_digested);
        assert!(report.final_reply_correct, "final_reply = {}", report.final_reply);
        assert!(report.no_regression);
        assert!(report.all_pass(), "smoke task 7 phase not all pass");
        assert_eq!(report.pass_rate(), 1.0);
    }

    /// 跑 smoke task 在 missing 目录, 验 setup 阶段 fail (scan_error 记录)
    #[test]
    fn smoke_task_missing_dir_fails_at_setup() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_smoke_conventions_tool_loop(tmp.path());
        assert!(!report.setup_ok);
        // scan_error 记录, 后续阶段都因 setup 失败 skip
        assert!(!report.prompt_built);
        assert!(!report.tool_loop_init);
        // 7 阶段 pass_rate 0
        assert_eq!(report.pass_rate(), 0.0);
    }

    /// pass_rate / all_pass_count helper 正确
    #[test]
    fn pass_rate_counts_correctly() {
        let mut r = SmokeReport::default();
        r.setup_ok = true;
        r.prompt_built = true;
        r.tool_loop_init = true;
        // 4 阶段 fail
        assert_eq!(r.all_pass_count(), 3);
        assert_eq!(r.pass_rate(), 3.0 / 7.0);
        assert!(!r.all_pass());

        r.tool_call_dispatched = true;
        r.tool_result_digested = true;
        r.final_reply_correct = true;
        r.no_regression = true;
        assert!(r.all_pass());
        assert_eq!(r.pass_rate(), 1.0);
    }

    /// to_eval_scores 输出 7 维, 跟 R23 6 module aggregation 兼容
    #[test]
    fn to_eval_scores_7_dim_aggregable() {
        let mut r = SmokeReport::default();
        for _b in [true, false, true, true, false, true, true] {
            // 模拟
        }
        r.setup_ok = true;
        r.prompt_built = true;
        r.tool_loop_init = true;
        r.tool_call_dispatched = true;
        r.tool_result_digested = true;
        r.final_reply_correct = true;
        r.no_regression = true;
        // 重跑让 phase_scores 填充
        r.phase_scores = vec![
            EvalScore::new("setup_ok", 1.0),
            EvalScore::new("prompt_built", 1.0),
            EvalScore::new("tool_loop_init", 1.0),
            EvalScore::new("tool_call_dispatched", 1.0),
            EvalScore::new("tool_result_digested", 1.0),
            EvalScore::new("final_reply_correct", 1.0),
            EvalScore::new("no_regression", 1.0),
        ];
        let scores = r.to_eval_scores();
        assert_eq!(scores.len(), 7);
        // R23 mean / weighted_mean / stddev / percentile 都能跑
        assert!(mean(&scores).is_some());
        assert!(stddev(&scores) >= 0.0);
        assert!(percentile(&scores, 0.5).is_some());
        // 全 1.0 → mean = 1.0, stddev = 0.0
        assert!((mean(&scores).unwrap() - 1.0).abs() < 1e-9);
        assert!(stddev(&scores) < 1e-9);
    }

    /// SmokeReport 默认值是 all fail
    #[test]
    fn default_smoke_report_all_false() {
        let r = SmokeReport::default();
        assert!(!r.all_pass());
        assert_eq!(r.pass_rate(), 0.0);
        assert!(r.system_block.is_empty());
        assert!(r.conventions.is_none());
        assert!(r.final_reply.is_empty());
    }

    /// 集成跟 R32-2 tool_loop 真实跑 (0 mock)
    #[test]
    fn integration_with_real_tool_loop() {
        let tmp = tempfile::tempdir().unwrap();
        // 写 1 个临时 Cargo.toml
        let cargo = tmp.path().join("Cargo.toml");
        std::fs::write(&cargo, "[workspace]\nmembers = [\"x\"]\n[workspace.package]\nedition = \"2021\"\n").unwrap();
        let report = run_smoke_conventions_tool_loop(tmp.path());
        assert!(report.setup_ok);
        assert!(report.prompt_built);
        assert!(report.tool_call_dispatched);
        assert!(report.tool_result_digested);
        assert!(report.final_reply_correct);
    }
}
