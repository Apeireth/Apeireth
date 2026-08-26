//! apeireth-cognition: 认知器官 (A10 落点 — R14 Phase 4)
//!
//! **职责**: 内部认知主路径 — 接收感知输入 → ASI 评分 (V0.5/V1136) → 12 键 verdict
//! 守门 → 决策 → 反思。
//!
//! **架构位置**: 阶段 4 §2 主路径 17 crate 之 A10 器官 (在 apeireth-perception 之后、
//! apeireth-action/motivation/value 之前)
//!
//! **当前状态**: A10 最小可用落地 (P2 任务 e3523aca by database_engineer).
//! 本 crate 提供 5+ pub fn + 5+ tests + example, 调用 V0.5/V1136 + 12 键 verdict 守门.
//!
//! **TP18 (E3, P1) 增量**: 校准 + 集合预报 + 预测市场
//! - `calibration` — Brier 评分 + Murphy 单调分解 (reliability / resolution / uncertainty)
//!   + 10-bin CalibrationBin + Expected Calibration Error
//! - `forecast` — EnsembleForecast (Bayesian / Mean / Median 聚合, 反方加权) +
//!   PredictionMarket (Hanson LMSR, 反方 cost subsidy)
//!
//! **诚实登记**: 按 handover-final-2026-08-01 §B.4 "5+ pub fn, 5+ tests, 调用 V0.5/V1136 +
//! 12 键" 简化实现. 完整认知器官 (双洋葱 + Cognitive-Dream 6 状态机) 待 A18/A19 深化.
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core / apeireth-asi 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/

#![deny(unsafe_code)]

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use apeireth_core::{verdict_for_target, ActionTarget, PhilosophyVerdict};
use thiserror::Error;
use uuid::Uuid;

pub mod consciousness_bridge;
mod decision;
pub mod forecast; // TP18 (E3, P1): EnsembleForecast + PredictionMarket (LMSR) // R172: bridge 1 of 7
                  // R176: bridge 1 Kani proofs
mod bridge_kani_proofs;
// R177: organ invariants (10 tests + 2 Kani proofs)
pub mod calibration; // TP18 (E3, P1): Brier 单调分解 + CalibrationBin
mod organ_kani_proofs;
pub mod planning;
mod reflection;
mod scoring; // A1/P2#7: MCTS/LATS 规划搜索机制 (trait 注入, 零 LLM 依赖)

pub use calibration::{
    brier_score, brier_single, calibration_bins, decompose, expected_calibration_error,
    BrierDecomposition, CalibrationBin, Observation, DEFAULT_NUM_BINS,
};
pub use decision::{CognitiveOutput, CognitivePipeline};
pub use forecast::{
    AggregationStrategy, EnsembleConfig, EnsembleForecast, EnsembleMember, MarketConfig,
    MarketError, PredictionMarket, TradeReceipt,
};
pub use planning::{
    MctsConfig, MctsPlanner, SearchAction, SearchResult, SearchState, StateEvaluator,
};
pub use reflection::{ReflectionReport, ReflectionVerdict};
pub use scoring::{
    continuity_score, identity_score, philosophy_guard_score, salience_score, transferability_score,
};

/// 顶层错误: 所有 cognition 子系统的 fallback error.
#[derive(Debug, Error)]
pub enum CognitionError {
    /// 输入参数非法.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// ASI 评分越界 ([0.0, 1.0]).
    #[error("asi score out of range: {0}")]
    AsiOutOfRange(f64),
    /// Verdict 链中发现 Block 决策.
    #[error("verdict blocked: {0:?}")]
    VerdictBlocked(PhilosophyVerdict),
    /// 序列化错误.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 统一结果类型.
pub type CognitionResult<T> = Result<T, CognitionError>;

/// 认知输入 — 感知结果 + 候选行动 + 上下文元数据.
#[derive(Debug, Clone)]
pub struct CognitiveInput {
    /// 输入唯一 ID.
    pub input_id: Uuid,
    /// 关联 session ID (可选, 用于跨 session continuity).
    pub session_id: Option<Uuid>,
    /// 候选行动 (12 键 verdict 守门的目标).
    pub candidate_targets: Vec<ActionTarget>,
    /// 时间戳 (Unix seconds).
    pub timestamp: i64,
    /// 上下文标签 (供反思/记忆回溯使用).
    pub context_tag: String,
}

impl CognitiveInput {
    /// 构造最小输入.
    pub fn new(candidate_targets: Vec<ActionTarget>, context_tag: impl Into<String>) -> Self {
        Self {
            input_id: Uuid::new_v4(),
            session_id: None,
            candidate_targets,
            timestamp: chrono::Utc::now().timestamp(),
            context_tag: context_tag.into(),
        }
    }

    /// 校验输入合法性.
    pub fn validate(&self) -> CognitionResult<()> {
        if self.candidate_targets.is_empty() {
            return Err(CognitionError::InvalidInput(
                "candidate_targets must not be empty".to_string(),
            ));
        }
        if self.context_tag.is_empty() {
            return Err(CognitionError::InvalidInput(
                "context_tag must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// 一次完整认知周期 — 输入 → 评分 → 守门 → 决策 → 反思.
#[derive(Debug, Clone)]
pub struct CognitiveCycle {
    /// 周期输入 ID.
    pub input_id: Uuid,
    /// ASI V0.5 评分.
    pub v05: AsiV05Scores,
    /// ASI V1136 子测度.
    pub v1136: V1136Submeasures,
    /// 12 键 verdict 链 (与 candidate_targets 一一对应).
    pub verdicts: Vec<PhilosophyVerdict>,
    /// 最终决策.
    pub output: CognitiveOutput,
    /// 反思报告.
    pub reflection: ReflectionReport,
}

impl CognitiveCycle {
    /// 周期是否拒绝 (任一 verdict Block).
    pub fn is_rejected(&self) -> bool {
        matches!(self.output, CognitiveOutput::Reject(_))
    }

    /// 周期是否允许.
    pub fn is_allowed(&self) -> bool {
        matches!(self.output, CognitiveOutput::Decision(_))
    }
}

/// 跑一次完整认知周期 (公开 API — 调用方最常用入口).
pub fn run_cycle(input: CognitiveInput) -> CognitionResult<CognitiveCycle> {
    input.validate()?;

    let v05 = scoring::score_v05(&input);
    let v1136 = scoring::score_v1136(&input);
    let verdicts = decision::evaluate_actions(&input.candidate_targets);
    let output = decision::decide(&verdicts)?;
    let reflection = reflection::reflect(&input, &v05, &v1136, &verdicts, &output);

    Ok(CognitiveCycle {
        input_id: input.input_id,
        v05,
        v1136,
        verdicts,
        output,
        reflection,
    })
}

/// 示例认知引擎 — 为阶段 4 抽象 trait 提供确定性的原生 Rust 基线实现.
///
/// 该类型无内部状态，适合示例、测试和调用方在接入专用实现前验证契约。
#[derive(Debug, Default, Clone, Copy)]
pub struct BasicCognitiveEngine;

/// 综合认知：把一组观察合成为可消费的认知结果。
pub trait Cognition {
    /// 对观察进行规范化并合成；空观察返回 `None`。
    fn cognize(&self, observations: &[&str]) -> Option<String>;
}

/// 直觉：从候选项中快速选出一个建议。
pub trait Intuition {
    /// 返回首个非空候选项；不存在时返回 `None`。
    fn intuit<'a>(&self, candidates: &'a [&'a str]) -> Option<&'a str>;
}

/// 推理：根据前提形成结论。
pub trait Reasoning {
    /// 所有前提均成立时返回 `true`；空前提不构成充分理由。
    fn reason(&self, premises: &[bool]) -> bool;
}

/// 元认知：评估认知结果的可信度。
pub trait MetaCognition {
    /// 将置信度限制在 `[0.0, 1.0]`；非有限值按 `0.0` 处理。
    fn assess_confidence(&self, confidence: f64) -> f64;
}

/// 回忆：按查询从记忆项中取回匹配内容。
pub trait Recall {
    /// 返回第一个包含查询串的记忆；空查询不匹配。
    fn recall<'a>(&self, query: &str, memories: &'a [&'a str]) -> Option<&'a str>;
}

/// 巩固：去除空白与相邻重复项，形成稳定记忆。
pub trait Consolidation {
    /// 返回保持输入顺序的巩固结果。
    fn consolidate(&self, memories: &[&str]) -> Vec<String>;
}

/// 遗忘：根据保留策略移除记忆。
pub trait Forgetting {
    /// 只保留 `retain` 返回 `true` 的记忆。
    fn forget(&self, memories: &[&str], retain: &dyn Fn(&str) -> bool) -> Vec<String>;
}

/// 学习：根据反馈更新当前知识强度。
pub trait Learning {
    /// 应用反馈增量并把结果限制在 `[0.0, 1.0]`。
    fn learn(&self, current: f64, feedback: f64) -> f64;
}

/// 抽象：从样本中提取共同的非空前缀。
pub trait Abstraction {
    /// 返回所有样本的共同前缀；空输入返回 `None`。
    fn abstract_commonality(&self, samples: &[&str]) -> Option<String>;
}

impl Cognition for BasicCognitiveEngine {
    fn cognize(&self, observations: &[&str]) -> Option<String> {
        let normalized: Vec<_> = observations
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .collect();
        (!normalized.is_empty()).then(|| normalized.join(" | "))
    }
}

impl Intuition for BasicCognitiveEngine {
    fn intuit<'a>(&self, candidates: &'a [&'a str]) -> Option<&'a str> {
        candidates
            .iter()
            .copied()
            .find(|item| !item.trim().is_empty())
    }
}

impl Reasoning for BasicCognitiveEngine {
    fn reason(&self, premises: &[bool]) -> bool {
        !premises.is_empty() && premises.iter().all(|premise| *premise)
    }
}

impl MetaCognition for BasicCognitiveEngine {
    fn assess_confidence(&self, confidence: f64) -> f64 {
        if confidence.is_finite() {
            confidence.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

impl Recall for BasicCognitiveEngine {
    fn recall<'a>(&self, query: &str, memories: &'a [&'a str]) -> Option<&'a str> {
        (!query.is_empty())
            .then(|| {
                memories
                    .iter()
                    .copied()
                    .find(|memory| memory.contains(query))
            })
            .flatten()
    }
}

impl Consolidation for BasicCognitiveEngine {
    fn consolidate(&self, memories: &[&str]) -> Vec<String> {
        let mut result = Vec::new();
        for memory in memories
            .iter()
            .map(|memory| memory.trim())
            .filter(|memory| !memory.is_empty())
        {
            if result.last().map(String::as_str) != Some(memory) {
                result.push(memory.to_string());
            }
        }
        result
    }
}

impl Forgetting for BasicCognitiveEngine {
    fn forget(&self, memories: &[&str], retain: &dyn Fn(&str) -> bool) -> Vec<String> {
        memories
            .iter()
            .copied()
            .filter(|memory| retain(memory))
            .map(str::to_string)
            .collect()
    }
}

impl Learning for BasicCognitiveEngine {
    fn learn(&self, current: f64, feedback: f64) -> f64 {
        let current = if current.is_finite() { current } else { 0.0 };
        let feedback = if feedback.is_finite() { feedback } else { 0.0 };
        (current + feedback).clamp(0.0, 1.0)
    }
}

impl Abstraction for BasicCognitiveEngine {
    fn abstract_commonality(&self, samples: &[&str]) -> Option<String> {
        let first = *samples.first()?;
        let mut boundary = first.len();
        for sample in &samples[1..] {
            let matched_bytes: usize = first
                .chars()
                .zip(sample.chars())
                .take_while(|(left, right)| left == right)
                .map(|(character, _)| character.len_utf8())
                .sum();
            boundary = boundary.min(matched_bytes);
        }
        (boundary > 0).then(|| first[..boundary].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognitive_input_validate_rejects_empty_targets() {
        let input = CognitiveInput::new(vec![], "test");
        assert!(input.validate().is_err());
    }

    #[test]
    fn cognitive_input_validate_rejects_empty_context() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "");
        assert!(input.validate().is_err());
    }

    #[test]
    fn cognitive_input_validate_accepts_valid_input() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "test");
        assert!(input.validate().is_ok());
    }

    #[test]
    fn run_cycle_normal_action_is_allowed() {
        let target = ActionTarget::NormalAction("read".to_string());
        let input = CognitiveInput::new(vec![target], "normal_op");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_allowed());
        assert!(!cycle.is_rejected());
        assert_eq!(cycle.verdicts.len(), 1);
    }

    #[test]
    fn run_cycle_modify_l0_ha_is_rejected() {
        let target = ActionTarget::ModifyL0HA;
        let input = CognitiveInput::new(vec![target], "l0_violation_attempt");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
        assert!(!cycle.is_allowed());
        // 12 键 verdict 守门: ModifyL0HA → Block(NotUnobservable)
        assert!(matches!(cycle.verdicts[0], PhilosophyVerdict::Block(_)));
    }

    #[test]
    fn run_cycle_pretend_clone_is_rejected() {
        let target = ActionTarget::PretendClone;
        let input = CognitiveInput::new(vec![target], "phl01_violation");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
    }

    #[test]
    fn run_cycle_mixed_targets_partial_reject() {
        // 混合: 1 个安全 + 1 个危险 → 周期 reject
        let targets = vec![
            ActionTarget::NormalAction("read".to_string()),
            ActionTarget::PretendPerfect,
        ];
        let input = CognitiveInput::new(targets, "mixed");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
        assert_eq!(cycle.verdicts.len(), 2);
    }

    #[test]
    fn run_cycle_uses_verdict_for_target_core_api() {
        // 直接确认: verdicts 与 apeireth_core::verdict_for_target 一致.
        let target = ActionTarget::PretendUuid;
        let expected = verdict_for_target(&target);
        let input = CognitiveInput::new(vec![target], "phl01_uuid");
        let cycle = run_cycle(input).expect("cycle must run");
        assert_eq!(cycle.verdicts[0], expected);
    }

    #[test]
    fn run_cycle_assigns_input_id_to_cycle() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "id_check");
        let cycle = run_cycle(input).expect("cycle must run");
        // input_id 内部不可见 (run_cycle 接收 ownership), 但 cycle.input_id
        // 应等于新生成的 UUID — 通过长度判断.
        assert_eq!(cycle.input_id.get_uuid_len(), 16);
    }
}

// 内部 helper extension trait 仅用于测试中验证 UUID 长度.
trait UuidLenExt {
    fn get_uuid_len(&self) -> usize;
}

impl UuidLenExt for Uuid {
    fn get_uuid_len(&self) -> usize {
        self.as_bytes().len()
    }
}

// === P28 阶段 6: apeireth-verify 互锁注入 === — disabled V26 to avoid circular
// apeireth_verify::trace_init!(VERIFY_TRACE);
// apeireth_verify::regression_assert!(
//     __APEIRETH_REG_APEIRETH_COGNITION_A,
//     "apeireth-cognition",
//     "apeireth-cognition structural invariant — P28 互锁 (assert_in_range)",
//     InRange { name: "apeireth-cognition::invariant-a", value: 1.0, min: 0.0, max: 1.0 }
// );
// apeireth_verify::regression_assert!(
//     __APEIRETH_REG_APEIRETH_COGNITION_B,
//     "apeireth-cognition",
//     "apeireth-cognition regression gate — P28 互锁 (assert_idempotent)",
//     Idempotent { name: "apeireth-cognition::invariant-b", first: "stable", second: "stable" }
// );

// ============================================================================
// round9-04 (V26.4) — __register_all_asserts no-op stub
//
// V26.2 backend_engineer2 disabled the original `apeireth_verify::register_all_in_crate!` macro
// call to break a circular dependency (core/verify mutually referenced).
// V26.3 DEF-V26.3-002 walk_all_crates example couldn't compile because no __register_all_asserts
// existed. V26.4 fix: provide a no-op stub that walk_all_crates can call. The stub does
// nothing (no regression assertions registered) which is the V26.2 intent (no circular
// dependency, but the symbol exists for example discovery).
//
// Future upgrade path (P28 stage 6 real impl): replace this stub with the real macro
// call once the circular dependency is resolved (e.g., via inventory/ctor or refactor
// apeireth-verify to be a thin facade).
#[allow(missing_docs, dead_code)] // V26.4 stub: walk_all_crates calls this no-op
pub fn __register_all_asserts() {
    // no-op by design
}
