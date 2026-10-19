//! Budgeted context-block assembler (relational prompt injection).
//!
//! Scope: the block
//! pipeline only — rot scoring already lives in
//! `apeireth-orchestration::context_rot` and is a different algorithm.
//!
//! - Ordered named blocks with a total character budget.
//! - Core blocks (persona / identity / essential story) are never truncated.
//! - Non-core overflow is cut greedily from the longest remaining block.
//! - Per-block `cap_chars` applies before the total budget.
//!
//! Default-off library primitive. Not a second session, transcript, or
//! prompt-cache owner (`PromptCacheStabilizer` remains the cache-prefix owner).
//!
//! # W3.1 组装策略（默认关，crate 内接线）
//!
//! [`AssemblyPolicy`] 把异稳态 [`RegulationPlan`](crate::homeostasis::RegulationPlan)
//! 映射到本块的字符预算管线。默认策略 = [`AssemblyPolicy::default`]（全 `false`），
//! 既有输出逐字节不变；只有显式 [`ContextAssembler::with_policy`] 才启用。
//!
//! | 调节动作 | `AssemblyPolicy` 字段 | 组装效果 |
//! |---|---|---|
//! | `ConsolidateBeforeOverflow` | `consolidate_before_overflow` | 非核心块按单块 cap 后**确定性截半**折叠；有效预算减半 |
//! | `ReduceRecallDepth` | `reduce_recall_depth` | 只注入最前面的 2 个非核心块（深检索层丢弃）；有效预算 ×3/4 |
//! | `ThrottleProactiveMessages` | `throttle_proactive_messages` | 仅暴露标志，由调用方消费；组装器不自行发消息 |
//! | `RestoreBudget` | `restore_budget` | 仅暴露标志，由调用方消费 |
//! | `EscalateToHuman` | `escalate_to_human` | 仅暴露标志，由调用方消费 |
//! | `None` / 空计划 | 全部 `false` | 默认策略，与今天行为一致 |
//!
//! 「折叠」是确定性前缀截断，**不是**语义摘要：本模块不调 LLM、不压缩语义，
//! 只是把低优先级（非核心）块在预算压力下提前变短。核心块永不折叠、不参与
//! 预算裁剪（空 / 纯空白块仍按既有规则过滤）。
//!
//! # 能力级别（四级口径）：IMPLEMENTED（crate 内已实现、默认关、未接线）
//!
//! 不是 PRODUCTION WIRED、不是 DEFAULT ENABLED、不是 HARDWARE VALIDATED：
//! 没有 runtime-assembly 采集 `VitalSignSet` 并构造 `RegulationPlan` 注入本策略
//! （W3.2 装配点）。`from_plan` 只是纯映射，不读时钟、不做 IO。

use crate::homeostasis::{RegulationAction, RegulationPlan};
use serde::{Deserialize, Serialize};

/// Named injection block.
#[derive(Debug, Clone)]
pub struct ContextBlock {
    /// Block name (debug / budget report).
    pub name: &'static str,
    /// Block body (may contain newlines).
    pub content: String,
    /// Core block: never truncated by the total budget.
    pub core: bool,
    /// Per-block cap (`None` = unlimited, still subject to total budget).
    pub cap_chars: Option<usize>,
}

impl ContextBlock {
    pub fn new(name: &'static str, content: impl Into<String>) -> Self {
        Self {
            name,
            content: content.into(),
            core: false,
            cap_chars: None,
        }
    }

    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }

    pub fn with_cap(mut self, cap: usize) -> Self {
        self.cap_chars = Some(cap);
        self
    }
}

/// Assembly policy derived from a homeostasis regulation plan (default off).
///
/// The default value (all flags `false`) is the legacy pipeline: no folding,
/// no layer dropping, no budget tightening. [`Self::from_plan`] is set-based —
/// predicted and observed steps map identically, and [`RegulationAction::None`]
/// is ignored.
///
/// | action | field | assembly effect |
/// |---|---|---|
/// | `ConsolidateBeforeOverflow` | [`Self::consolidate_before_overflow`] | non-core blocks fold to half their capped length; effective budget halves |
/// | `ReduceRecallDepth` | [`Self::reduce_recall_depth`] | only the first [`Self::REDUCED_RECALL_MAX_LAYERS`] non-core blocks are injected; effective budget ×3/4 |
/// | `ThrottleProactiveMessages` | [`Self::throttle_proactive_messages`] | caller-facing flag only |
/// | `RestoreBudget` | [`Self::restore_budget`] | caller-facing flag only |
/// | `EscalateToHuman` | [`Self::escalate_to_human`] | caller-facing flag only |
///
/// The three caller-facing flags never change assembly output: the assembler
/// does not send messages, spend budget, or escalate on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct AssemblyPolicy {
    /// Fold non-core blocks and halve the effective budget (`ConsolidateBeforeOverflow`).
    pub consolidate_before_overflow: bool,
    /// Keep only the first [`Self::REDUCED_RECALL_MAX_LAYERS`] non-core blocks and
    /// shrink the effective budget to 3/4 (`ReduceRecallDepth`).
    pub reduce_recall_depth: bool,
    /// Caller-facing: suppress proactive messages this turn (`ThrottleProactiveMessages`).
    pub throttle_proactive_messages: bool,
    /// Caller-facing: budget restoration requested (`RestoreBudget`).
    pub restore_budget: bool,
    /// Caller-facing: escalate to human (`EscalateToHuman`).
    pub escalate_to_human: bool,
}

impl AssemblyPolicy {
    /// Effective-budget divisor under `ConsolidateBeforeOverflow` (`budget / 2`).
    pub const CONSOLIDATION_BUDGET_DIVISOR: usize = 2;

    /// Non-core fold divisor under `ConsolidateBeforeOverflow` (keep `len / 2`).
    pub const CONSOLIDATION_FOLD_DIVISOR: usize = 2;

    /// Maximum non-core blocks injected under `ReduceRecallDepth` (deeper layers drop).
    pub const REDUCED_RECALL_MAX_LAYERS: usize = 2;

    /// Effective-budget multiplier numerator under `ReduceRecallDepth` (`×3/4`).
    pub const REDUCED_RECALL_BUDGET_NUMERATOR: usize = 3;

    /// Effective-budget multiplier denominator under `ReduceRecallDepth` (`×3/4`).
    pub const REDUCED_RECALL_BUDGET_DENOMINATOR: usize = 4;

    /// Whether no policy is active (the legacy path, byte-identical output).
    pub fn is_default(self) -> bool {
        self == Self::default()
    }

    /// Effective total char budget for a configured budget.
    ///
    /// Composition order is fixed: consolidation halves first, then reduced
    /// recall takes 3/4 of the result. Integer division floors; the result can
    /// fall below [`ContextAssembler::new`]'s 100-char floor (the floor only
    /// applies to the configured budget, not to a stress policy).
    pub fn effective_budget_chars(self, configured_budget_chars: usize) -> usize {
        let mut budget = configured_budget_chars;
        if self.consolidate_before_overflow {
            budget /= Self::CONSOLIDATION_BUDGET_DIVISOR;
        }
        if self.reduce_recall_depth {
            budget = budget.saturating_mul(Self::REDUCED_RECALL_BUDGET_NUMERATOR)
                / Self::REDUCED_RECALL_BUDGET_DENOMINATOR;
        }
        budget
    }

    /// Map a regulation plan onto the block pipeline (pure, no side effects).
    ///
    /// Action presence is what matters: a step marked `predicted` maps exactly
    /// like an observed step, and a plan containing only
    /// [`RegulationAction::None`] (or no steps) yields [`Self::default`].
    pub fn from_plan(plan: &RegulationPlan) -> Self {
        let mut policy = Self::default();
        for step in &plan.steps {
            match step.action {
                RegulationAction::ConsolidateBeforeOverflow => {
                    policy.consolidate_before_overflow = true;
                }
                RegulationAction::ReduceRecallDepth => {
                    policy.reduce_recall_depth = true;
                }
                RegulationAction::ThrottleProactiveMessages => {
                    policy.throttle_proactive_messages = true;
                }
                RegulationAction::RestoreBudget => {
                    policy.restore_budget = true;
                }
                RegulationAction::EscalateToHuman => {
                    policy.escalate_to_human = true;
                }
                RegulationAction::None => {}
            }
        }
        policy
    }
}

/// Ordered block pipeline: total budget + core protection + greedy truncate.
///
/// [`Self::policy`] defaults to [`AssemblyPolicy::default`], which keeps the
/// legacy behavior byte-for-byte. [`Self::with_policy`] is the only opt-in switch.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
    policy: AssemblyPolicy,
}

impl ContextAssembler {
    /// Total budget in Unicode scalar values (minimum 100).
    pub fn new(total_budget_chars: usize) -> Self {
        Self {
            blocks: Vec::new(),
            total_budget_chars: total_budget_chars.max(100),
            policy: AssemblyPolicy::default(),
        }
    }

    /// Enable an assembly policy (chainable). Default = legacy behavior.
    pub fn with_policy(mut self, policy: AssemblyPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Current policy (default = all flags off).
    pub fn policy(&self) -> AssemblyPolicy {
        self.policy
    }

    /// Configured total budget (constructor value, before policy tightening).
    pub fn total_budget_chars(&self) -> usize {
        self.total_budget_chars
    }

    /// Effective total budget after policy tightening.
    pub fn effective_total_budget_chars(&self) -> usize {
        self.policy.effective_budget_chars(self.total_budget_chars)
    }

    /// Register a block (order preserved; core blocks should be registered first).
    pub fn push(mut self, block: ContextBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// Diagnostic: `(name, char_count)` per registered block (pre-policy, pre-budget).
    pub fn budget_report(&self) -> Vec<(String, usize)> {
        self.blocks
            .iter()
            .map(|b| (b.name.to_string(), b.content.chars().count()))
            .collect()
    }

    /// Budgeted assembly: truncated contents, empty / whitespace-only dropped.
    pub fn assemble_budgeted(&self) -> Vec<String> {
        self.assemble_budgeted_blocks()
            .into_iter()
            .map(|b| b.content)
            .collect()
    }

    /// Budgeted assembly keeping name / core / cap metadata.
    pub fn assemble_budgeted_blocks(&self) -> Vec<ContextBlock> {
        if self.policy.is_default() {
            return assemble_budgeted_blocks_raw(&self.blocks, self.total_budget_chars, false);
        }
        let blocks = self.policy_blocks();
        let budget = self.policy.effective_budget_chars(self.total_budget_chars);
        assemble_budgeted_blocks_raw(&blocks, budget, self.policy.consolidate_before_overflow)
    }

    /// Block view after recall-depth layer selection (core blocks always stay).
    fn policy_blocks(&self) -> Vec<ContextBlock> {
        if !self.policy.reduce_recall_depth {
            return self.blocks.clone();
        }
        let mut kept = Vec::with_capacity(self.blocks.len());
        let mut non_core_layers = 0_usize;
        for block in &self.blocks {
            if block.core {
                kept.push(block.clone());
            } else if non_core_layers < AssemblyPolicy::REDUCED_RECALL_MAX_LAYERS {
                non_core_layers += 1;
                kept.push(block.clone());
            }
        }
        kept
    }
}

/// Shared block pipeline: cap → optional non-core fold → total-budget greedy cut.
///
/// The default path passes `fold_non_core = false` and the configured budget,
/// so the legacy output is preserved byte-for-byte.
fn assemble_budgeted_blocks_raw(
    blocks: &[ContextBlock],
    total_budget_chars: usize,
    fold_non_core: bool,
) -> Vec<ContextBlock> {
    let mut capped: Vec<String> = blocks
        .iter()
        .map(|b| {
            let capped: String = b
                .content
                .chars()
                .take(b.cap_chars.unwrap_or(usize::MAX))
                .collect();
            if fold_non_core && !b.core {
                let folded_len =
                    capped.chars().count() / AssemblyPolicy::CONSOLIDATION_FOLD_DIVISOR;
                capped.chars().take(folded_len).collect()
            } else {
                capped
            }
        })
        .collect();
    let mut total: usize = capped.iter().map(|s| s.chars().count()).sum();
    if total > total_budget_chars {
        let mut order: Vec<usize> = (0..blocks.len()).filter(|&i| !blocks[i].core).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(capped[i].chars().count()));
        for i in order {
            if total <= total_budget_chars {
                break;
            }
            let len = capped[i].chars().count();
            if len == 0 {
                continue;
            }
            let over = total - total_budget_chars;
            let cut = len.min(over);
            capped[i] = capped[i].chars().take(len - cut).collect();
            total -= cut;
        }
    }
    blocks
        .iter()
        .zip(capped)
        .filter(|(_, s)| !s.trim().is_empty())
        .map(|(b, s)| ContextBlock {
            name: b.name,
            content: s,
            core: b.core,
            cap_chars: b.cap_chars,
        })
        .collect()
}

/// Hydra-style tiered concatenation: smaller tier numbers sort first.
/// Complementary to `PromptCacheStabilizer` (cache prefix) — this only orders
/// system-prompt *parts*, it does not own the message list.
pub fn assemble_tiered(parts: &[(u8, &str)]) -> String {
    let mut sorted = parts.to_vec();
    sorted.sort_by_key(|(tier, _)| *tier);
    let mut s = String::new();
    for (_, content) in sorted {
        s.push_str(content);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::{
        HomeostasisRegulator, RegulationStep, Severity, VitalSign, VitalSignReading, VitalSignSet,
    };

    const GOLDEN_BUDGET: usize = 100;

    fn golden_assembler() -> ContextAssembler {
        ContextAssembler::new(GOLDEN_BUDGET)
            .push(ContextBlock::new("core", "CORE").core(true))
            .push(ContextBlock::new("a", "a".repeat(150)))
            .push(ContextBlock::new("b", "b".repeat(50)))
    }

    fn plan_with(actions: &[RegulationAction]) -> RegulationPlan {
        RegulationPlan {
            steps: actions
                .iter()
                .map(|&action| RegulationStep {
                    action,
                    sign: None,
                    severity: Severity::Warn,
                    predicted: false,
                    reason: String::new(),
                })
                .collect(),
            severity: Severity::Warn,
            summary: String::new(),
        }
    }

    #[test]
    fn core_blocks_never_truncated() {
        let a = ContextAssembler::new(300)
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", "记忆内容".repeat(50)))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));
        let out = a.assemble_budgeted();
        let total: usize = out.iter().map(|s| s.chars().count()).sum();
        assert!(total <= 300, "总预算应约束 (核心保护下 total=300)");
        assert!(out[0].contains("核心人格内容"), "核心块应完整保留");
        assert!(
            out[1].chars().count() < "记忆内容".repeat(50).chars().count(),
            "非核心块应被截断"
        );
        assert_eq!(out[1].chars().count(), 40, "mem 200 → 砍 160 → 留 40");
    }

    #[test]
    fn per_block_cap() {
        let a = ContextAssembler::new(100_000)
            .push(ContextBlock::new("x", "abc".repeat(10)).with_cap(12));
        let out = a.assemble_budgeted();
        assert_eq!(out[0], "abcabcabcabc".to_string());
    }

    #[test]
    fn empty_blocks_filtered() {
        let a = ContextAssembler::new(1000)
            .push(ContextBlock::new("a", "hello"))
            .push(ContextBlock::new("b", "   "));
        let out = a.assemble_budgeted();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "hello");
    }

    #[test]
    fn budget_floor_is_100() {
        let a = ContextAssembler::new(10);
        assert_eq!(a.total_budget_chars(), 100);
    }

    #[test]
    fn budget_report_is_pre_truncate() {
        let a = ContextAssembler::new(10)
            .push(ContextBlock::new("a", "abcdef"))
            .push(ContextBlock::new("b", "xyz"));
        assert_eq!(a.budget_report(), vec![("a".into(), 6), ("b".into(), 3)]);
    }

    #[test]
    fn assemble_tiered_orders_by_tier() {
        let s = assemble_tiered(&[
            (100, "工具指引\n"),
            (0, "身份: 阿佩瑞斯\n"),
            (50, "记忆证据\n"),
        ]);
        let i0 = s.find("身份").unwrap();
        let i1 = s.find("记忆").unwrap();
        let i2 = s.find("工具").unwrap();
        assert!(i0 < i1 && i1 < i2, "tier 0 身份应最前: {s}");
    }

    #[test]
    fn core_blocks_can_exceed_budget_if_they_alone_overflow() {
        let a = ContextAssembler::new(100)
            .push(ContextBlock::new("persona", "核".repeat(150)).core(true))
            .push(ContextBlock::new("mem", "记".repeat(50)));
        let blocks = a.assemble_budgeted_blocks();
        assert_eq!(blocks[0].name, "persona");
        assert_eq!(blocks[0].content.chars().count(), 150);
        assert!(
            !blocks.iter().any(|b| b.name == "mem"),
            "非核心块应被砍空并过滤"
        );
    }

    #[test]
    fn policy_maps_every_regulation_action() {
        let cases = [
            (
                RegulationAction::ConsolidateBeforeOverflow,
                AssemblyPolicy {
                    consolidate_before_overflow: true,
                    ..AssemblyPolicy::default()
                },
            ),
            (
                RegulationAction::ReduceRecallDepth,
                AssemblyPolicy {
                    reduce_recall_depth: true,
                    ..AssemblyPolicy::default()
                },
            ),
            (
                RegulationAction::ThrottleProactiveMessages,
                AssemblyPolicy {
                    throttle_proactive_messages: true,
                    ..AssemblyPolicy::default()
                },
            ),
            (
                RegulationAction::RestoreBudget,
                AssemblyPolicy {
                    restore_budget: true,
                    ..AssemblyPolicy::default()
                },
            ),
            (
                RegulationAction::EscalateToHuman,
                AssemblyPolicy {
                    escalate_to_human: true,
                    ..AssemblyPolicy::default()
                },
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(
                AssemblyPolicy::from_plan(&plan_with(&[action])),
                expected,
                "{action:?} 映射错误"
            );
        }

        // 预测步与观测步映射一致（策略只看动作集合，不看 predicted）。
        let mut predicted = plan_with(&[RegulationAction::ReduceRecallDepth]);
        predicted.steps[0].predicted = true;
        assert_eq!(
            AssemblyPolicy::from_plan(&predicted),
            AssemblyPolicy {
                reduce_recall_depth: true,
                ..AssemblyPolicy::default()
            }
        );

        // None 哨兵不产生任何标志。
        assert!(AssemblyPolicy::from_plan(&plan_with(&[RegulationAction::None])).is_default());
    }

    #[test]
    fn empty_and_noop_plans_map_to_default_policy() {
        let empty = RegulationPlan {
            steps: Vec::new(),
            severity: Severity::Normal,
            summary: String::new(),
        };
        assert!(AssemblyPolicy::from_plan(&empty).is_default());

        let noop = HomeostasisRegulator::new().regulate(&VitalSignSet::healthy());
        assert!(noop.is_noop());
        assert!(AssemblyPolicy::from_plan(&noop).is_default());
    }

    #[test]
    fn critical_vitals_map_to_all_five_flags() {
        let vitals = VitalSignSet::healthy()
            .with(
                VitalSign::ContextPressure,
                VitalSignReading::defaults_for(VitalSign::ContextPressure, 0.95),
            )
            .with(
                VitalSign::BudgetRemaining,
                VitalSignReading::defaults_for(VitalSign::BudgetRemaining, 0.01),
            );
        let plan = HomeostasisRegulator::new().regulate(&vitals);
        let policy = AssemblyPolicy::from_plan(&plan);
        assert!(policy.consolidate_before_overflow);
        assert!(policy.reduce_recall_depth);
        assert!(policy.throttle_proactive_messages);
        assert!(policy.restore_budget);
        assert!(policy.escalate_to_human);
        assert!(!policy.is_default());
    }

    #[test]
    fn default_policy_is_byte_identical_to_legacy_assembly() {
        // 冻结的旧行为：204 chars → 预算 100；a 150 砍 104 留 46。
        let golden = vec!["CORE".to_string(), "a".repeat(46), "b".repeat(50)];

        let legacy = golden_assembler().assemble_budgeted();
        assert_eq!(legacy, golden);

        let explicit_default = golden_assembler()
            .with_policy(AssemblyPolicy::default())
            .assemble_budgeted();
        assert_eq!(explicit_default, golden);
        assert_eq!(
            golden_assembler()
                .with_policy(AssemblyPolicy::default())
                .effective_total_budget_chars(),
            GOLDEN_BUDGET
        );

        let noop_policy = AssemblyPolicy::from_plan(
            &HomeostasisRegulator::new().regulate(&VitalSignSet::healthy()),
        );
        assert_eq!(
            golden_assembler()
                .with_policy(noop_policy)
                .assemble_budgeted(),
            golden
        );
    }

    #[test]
    fn consolidate_policy_folds_non_core_blocks_and_halves_budget() {
        let policy = AssemblyPolicy {
            consolidate_before_overflow: true,
            ..AssemblyPolicy::default()
        };
        let assembler = ContextAssembler::new(400)
            .push(ContextBlock::new("core", "CORE").core(true))
            .push(ContextBlock::new("a", "a".repeat(100)))
            .push(ContextBlock::new("b", "b".repeat(100)))
            .with_policy(policy);
        assert_eq!(assembler.effective_total_budget_chars(), 200);
        let blocks = assembler.assemble_budgeted_blocks();
        assert_eq!(blocks[0].content, "CORE");
        assert_eq!(blocks[1].content, "a".repeat(50));
        assert_eq!(blocks[2].content, "b".repeat(50));

        // 顺序固定：先单块 cap，再折叠（40 → cap 20 → 折 10）。
        let capped = ContextAssembler::new(100_000)
            .push(ContextBlock::new("x", "x".repeat(40)).with_cap(20))
            .with_policy(policy)
            .assemble_budgeted();
        assert_eq!(capped, vec!["x".repeat(10)]);
    }

    #[test]
    fn reduce_recall_policy_keeps_two_non_core_layers_and_core() {
        let policy = AssemblyPolicy {
            reduce_recall_depth: true,
            ..AssemblyPolicy::default()
        };
        let assembler = ContextAssembler::new(1000)
            .push(ContextBlock::new("layer0", "0"))
            .push(ContextBlock::new("core", "CORE").core(true))
            .push(ContextBlock::new("layer1", "1"))
            .push(ContextBlock::new("layer2", "2"))
            .push(ContextBlock::new("layer3", "3"))
            .with_policy(policy);
        assert_eq!(assembler.effective_total_budget_chars(), 750);
        let blocks = assembler.assemble_budgeted_blocks();
        let names: Vec<&str> = blocks.iter().map(|b| b.name).collect();
        assert_eq!(names, vec!["layer0", "core", "layer1"]);
    }

    #[test]
    fn caller_flags_do_not_change_assembly_output() {
        let flags = AssemblyPolicy {
            throttle_proactive_messages: true,
            restore_budget: true,
            escalate_to_human: true,
            ..AssemblyPolicy::default()
        };
        assert!(!flags.is_default());

        let legacy = golden_assembler().assemble_budgeted();
        let flagged = golden_assembler().with_policy(flags).assemble_budgeted();
        assert_eq!(legacy, flagged);
        assert_eq!(
            golden_assembler()
                .with_policy(flags)
                .effective_total_budget_chars(),
            GOLDEN_BUDGET
        );
    }

    #[test]
    fn policy_effects_compose_and_are_deterministic() {
        let policy = AssemblyPolicy {
            consolidate_before_overflow: true,
            reduce_recall_depth: true,
            ..AssemblyPolicy::default()
        };
        let assembler = ContextAssembler::new(400)
            .push(ContextBlock::new("n0", "x".repeat(40)))
            .push(ContextBlock::new("n1", "x".repeat(40)))
            .push(ContextBlock::new("n2", "x".repeat(40)))
            .with_policy(policy);
        // 400 / 2 = 200；200 × 3/4 = 150。
        assert_eq!(assembler.effective_total_budget_chars(), 150);

        let first = assembler.assemble_budgeted();
        assert_eq!(first.len(), 2, "深检索层 n2 应被丢弃");
        assert!(first.iter().all(|s| s.chars().count() == 20));
        assert_eq!(first, assembler.assemble_budgeted());
    }
}
