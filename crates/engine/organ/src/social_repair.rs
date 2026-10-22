//! social_repair —— SG-20 交互修复内核 v0（生物机制族 W1 批；机制内核，未接线）。
//!
//! # 生物原型
//!
//! 对话质量的大头不是「一次说对」，而是**错了之后能不能修**：
//!
//! - **心智理论（Theory of Mind）**：对方持有独立信念，「我说了什么」不等于「对方理解了什么」。
//!   一旦双方信念出现偏差，继续堆信息只会放大偏差，先修对齐才划算。
//! - **共同注意（Joint Attention）**：双方指向同一个「它」（同一块屏幕区域、同一份文档、
//!   同一个任务、同一件产物）。共同注意是协作的前置条件——没确认「我们看的是不是同一个东西」，
//!   后续所有澄清都可能落在不同对象上。
//! - **误解后的修复动作**：澄清（clarify）、复述（restate）、确认（confirm）、
//!   给回退路径（rollback）。修复成本远低于重新解释整段。
//! - **轮流与打断（turn-taking / barge-in）**：本模块只吸收其中一条可测约束——
//!   **一次最多问一个问题**；发言轮次的调度属于既有 speech arbiter 面，不在本文件。
//!
//! # 工程边界（明确不做什么）
//!
//! - **不模拟真实心智理论**：不建「对方信念」状态机、不做信念推理、不推断动机或情绪。
//!   本模块只做**不确定性信号 → 修复动作的确定性映射**（固定规则表，见 [`plan_repair`]）。
//! - **不生成对话文本**：动作是固定枚举 [`RepairMove`]，没有自由文本载荷；
//!   [`RepairPlan::rationale`] 只给人工复核看的说明，且**不拼接**
//!   [`UncertaintySignal::detail`] 与 [`JointAttentionTarget`] 的 label 原文——
//!   避免把用户原话回灌成指责/质问文本。
//! - **不读系统时钟、不用随机数、不依赖 HashMap 迭代序**：同输入必得同输出
//!   （排序用 `f32::total_cmp`，NaN 也有全序）。
//! - **不做「问句清单」**：硬上限 [`MAX_REPAIR_MOVES`] = 2 个动作，且问句类动作
//!   （[`RepairMove::AskClarifying`] / [`RepairMove::RequestConfirmation`]）
//!   全计划至多 [`MAX_QUESTIONS_PER_PLAN`] = 1 个。
//!
//! # 能力级别（四级口径）：IMPLEMENTED（已实现、未接线）
//!
//! 不是 PRODUCTION WIRED、不是 DEFAULT ENABLED、不是 HARDWARE VALIDATED：
//!
//! - **未接 gateway 会话面**：没有任何会话事件驱动 [`plan_repair`]；
//! - **未进 prompt 组装**：`prompt_assembly` 不引用本模块，动作不会被写进提示词；
//! - **无 `self_status` 节**：本模块既不产出也不消费自述状态；
//! - 无持久化、无工具面、无遥测、无日志副作用。
//!
//! 接线点（W2 候选，不在本批）：会话纠正/矛盾事件 → [`UncertaintySignal`]；
//! 焦点变化 → [`JointAttention`]；[`RepairPlan::moves`] → prompt 组装中的一段修复指令；
//! [`RepairPlan::ask_user`] → 本轮是否允许向用户发问。

use serde::{Deserialize, Serialize};

/// 单个修复计划允许的最大动作数（硬不变量：「最多 2 个动作」）。
pub const MAX_REPAIR_MOVES: usize = 2;

/// 单个修复计划允许的最大问句数（硬不变量：「不许堆问句清单」）。
pub const MAX_QUESTIONS_PER_PLAN: usize = 1;

/// 不确定性信号：一次交互里「可能没对齐」的可测证据。
///
/// 每个信号带 `detail`（人类可读来源说明）与 `weight`（调用方给出的强度建议值，
/// 惯例区间 `0.0..=1.0`，本模块**不做 clamp、不做归一化**）。
///
/// `detail` 不会进入 [`RepairPlan::rationale`]，因此用户原话不会以指责口吻回灌。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UncertaintySignal {
    /// 自身置信度低（自评低于阈值，或证据薄弱）。
    LowConfidence {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
    /// 前后陈述互相矛盾（两轮结论冲突、或与已知事实冲突）。
    Contradiction {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
    /// 指代/意图含糊（「那个」「再改一下」这类没有唯一解释的表述）。
    Ambiguity {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
    /// 用户明确纠正（含「不是这个意思」「我说的是……」）。
    UserCorrection {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
    /// 引用对象缺失（提到的文件/字段/步骤不在当前上下文里）。
    ReferenceMissing {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
    /// 极性反转（同一问题的结论从「是」翻成「否」，或要求相反）。
    PolarityFlip {
        /// 来源说明（只读；不进入 rationale）。
        detail: String,
        /// 强度建议值（惯例 `0.0..=1.0`）。
        weight: f32,
    },
}

impl UncertaintySignal {
    /// 稳定种类名（与 serde 标签同值；写 rationale / 日志用）。
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::LowConfidence { .. } => "low_confidence",
            Self::Contradiction { .. } => "contradiction",
            Self::Ambiguity { .. } => "ambiguity",
            Self::UserCorrection { .. } => "user_correction",
            Self::ReferenceMissing { .. } => "reference_missing",
            Self::PolarityFlip { .. } => "polarity_flip",
        }
    }

    /// 来源说明（人类可读；不进入 rationale）。
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::LowConfidence { detail, .. }
            | Self::Contradiction { detail, .. }
            | Self::Ambiguity { detail, .. }
            | Self::UserCorrection { detail, .. }
            | Self::ReferenceMissing { detail, .. }
            | Self::PolarityFlip { detail, .. } => detail,
        }
    }

    /// 强度建议值（原样返回，不 clamp）。
    #[must_use]
    pub fn weight(&self) -> f32 {
        match self {
            Self::LowConfidence { weight, .. }
            | Self::Contradiction { weight, .. }
            | Self::Ambiguity { weight, .. }
            | Self::UserCorrection { weight, .. }
            | Self::ReferenceMissing { weight, .. }
            | Self::PolarityFlip { weight, .. } => *weight,
        }
    }

    /// 固定优先级 rank：越小越先（跨种类只看 rank，权重不能改序）。
    fn priority(&self) -> u8 {
        match self {
            Self::UserCorrection { .. } => 1,
            Self::Contradiction { .. } => 2,
            Self::PolarityFlip { .. } => 3,
            Self::ReferenceMissing { .. } => 4,
            Self::Ambiguity { .. } => 5,
            Self::LowConfidence { .. } => 6,
        }
    }

    /// 该信号排出的候选动作（固定顺序，最多 2 个）。
    fn seed_moves(&self) -> &'static [RepairMove] {
        match self {
            Self::UserCorrection { .. } => &[
                RepairMove::AcknowledgeUncertainty,
                RepairMove::RestateUnderstanding,
            ],
            Self::Contradiction { .. } => {
                &[RepairMove::CiteEvidence, RepairMove::RequestConfirmation]
            }
            Self::PolarityFlip { .. } => &[
                RepairMove::RestateUnderstanding,
                RepairMove::RequestConfirmation,
            ],
            Self::ReferenceMissing { .. } => {
                &[RepairMove::OfferRollback, RepairMove::AskClarifying]
            }
            Self::Ambiguity { .. } => &[RepairMove::AskClarifying],
            Self::LowConfidence { .. } => &[RepairMove::AcknowledgeUncertainty],
        }
    }
}

/// 修复动作：固定枚举，**没有自由文本载荷**。
///
/// 「不生成指责/质问式文本」由结构保证：动作只能取这 6 个值，具体措辞由上层
/// （尚未接线的 prompt 组装）决定，本模块不产出句子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairMove {
    /// 请对方澄清（问句类；全计划至多 1 个问句类动作）。
    AskClarifying,
    /// 复述我方理解，让对方只做「对/不对」的廉价确认。
    RestateUnderstanding,
    /// 给出回退路径（撤回上一版、回到上一个稳定状态）。
    OfferRollback,
    /// 引用证据（说明依据来自哪一步/哪份材料）。
    CiteEvidence,
    /// 承认不确定（先收窄承诺，不装懂）。
    AcknowledgeUncertainty,
    /// 请求确认（问句类；全计划至多 1 个问句类动作）。
    RequestConfirmation,
}

impl RepairMove {
    /// 稳定动作名（与 serde 标签同值；写 rationale 用）。
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AskClarifying => "ask_clarifying",
            Self::RestateUnderstanding => "restate_understanding",
            Self::OfferRollback => "offer_rollback",
            Self::CiteEvidence => "cite_evidence",
            Self::AcknowledgeUncertainty => "acknowledge_uncertainty",
            Self::RequestConfirmation => "request_confirmation",
        }
    }

    /// 是否问句类动作（计入 [`MAX_QUESTIONS_PER_PLAN`] 上限）。
    #[must_use]
    pub fn is_question(&self) -> bool {
        matches!(self, Self::AskClarifying | Self::RequestConfirmation)
    }
}

/// 一次修复的确定性计划（纯数据，无副作用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairPlan {
    /// 按优先级排好的修复动作，长度不超过 `max_moves`。
    pub moves: Vec<RepairMove>,
    /// 本轮是否向用户发问（`true` 当且仅当 [`Self::moves`] 含问句类动作）。
    pub ask_user: bool,
    /// 给人复核的决策说明；不含信号 `detail` 与注意力 label 原文，不含指责/质问语气。
    pub rationale: String,
    /// 本计划生效的动作上限（恒为 [`MAX_REPAIR_MOVES`]，便于调用方断言）。
    pub max_moves: usize,
}

/// 共同注意对象：双方此刻指向的那个「它」。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JointAttentionTarget {
    /// 屏幕区域（某个面板、某个坐标带）。
    ScreenRegion {
        /// 人类可读标识（不进入 rationale）。
        label: String,
    },
    /// 文档（某份规格、某段正文）。
    Document {
        /// 人类可读标识（不进入 rationale）。
        label: String,
    },
    /// 任务（当前协作的那一件待办）。
    Task {
        /// 人类可读标识（不进入 rationale）。
        label: String,
    },
    /// 产物（构建结果、文件、报告）。
    Artifact {
        /// 人类可读标识（不进入 rationale）。
        label: String,
    },
}

impl JointAttentionTarget {
    /// 稳定种类名（与 serde 标签同值）。
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::ScreenRegion { .. } => "screen_region",
            Self::Document { .. } => "document",
            Self::Task { .. } => "task",
            Self::Artifact { .. } => "artifact",
        }
    }

    /// 对象标识（人类可读；不进入 rationale）。
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::ScreenRegion { label }
            | Self::Document { label }
            | Self::Task { label }
            | Self::Artifact { label } => label,
        }
    }
}

/// 共同注意快照：双方此刻指向的对象，以及是否确认指向同一个对象。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointAttention {
    /// 双方共同指向的对象（焦点）。
    pub target: JointAttentionTarget,
    /// 双方是否确认指向同一个对象（`false` = 注意力不一致，需要先对齐对象）。
    pub agree: bool,
}

/// 把不确定性信号映射成一份**至多 2 个动作**的修复计划（确定性、无副作用）。
///
/// # 规则表
///
/// 优先级只看「信号种类」（rank 小者先），**权重不能跨种类改序**；同一 rank
/// （同一种类多条）按 `weight` 降序，随后由去重吸收。动作按表中固定顺序产出，
/// 去重后取前 [`MAX_REPAIR_MOVES`] 个，问句类动作全程至多
/// [`MAX_QUESTIONS_PER_PLAN`] 个：
///
/// | rank | 信号 | 候选动作（固定顺序） |
/// |---|---|---|
/// | 0 | 共同注意不一致（[`JointAttention::agree`] == `false`） | `RequestConfirmation` |
/// | 1 | [`UncertaintySignal::UserCorrection`] | `AcknowledgeUncertainty` → `RestateUnderstanding` |
/// | 2 | [`UncertaintySignal::Contradiction`] | `CiteEvidence` → `RequestConfirmation` |
/// | 3 | [`UncertaintySignal::PolarityFlip`] | `RestateUnderstanding` → `RequestConfirmation` |
/// | 4 | [`UncertaintySignal::ReferenceMissing`] | `OfferRollback` → `AskClarifying` |
/// | 5 | [`UncertaintySignal::Ambiguity`] | `AskClarifying` |
/// | 6 | [`UncertaintySignal::LowConfidence`] | `AcknowledgeUncertainty` |
///
/// 规则要点：
///
/// - **显式纠正压倒自评**：`UserCorrection` 固定 rank 1，即使 `LowConfidence`
///   权重更高也不会抢先——高权重噪音不许压过用户明说。
/// - **先对齐对象再谈内容**：注意力不一致排 rank 0，双方可能看的是不同对象时先确认对象。
/// - **问句不堆**：若某个信号想要的问句被更高 rank 的动作占满名额，就退化为
///   「先给证据、不追加第二问」，而不是堆一串问句。
/// - **空信号 + 注意力一致/缺失** → [`RepairPlan::moves`] 为空（本轮不发起修复动作）。
///
/// `rationale` 只使用信号种类名、权重数值、动作名与对象种类名，**不拼接** `detail`
/// 与 label 原文；同一输入必得同一份计划（含 rationale 文本）。
#[must_use]
pub fn plan_repair(
    signals: &[UncertaintySignal],
    attention: Option<&JointAttention>,
) -> RepairPlan {
    let attention_mismatch = attention.is_some_and(|snapshot| !snapshot.agree);

    // rank 升序；同 rank 按权重降序（total_cmp 对 NaN 也有全序 → 确定性）。
    let mut ordered: Vec<&UncertaintySignal> = signals.iter().collect();
    ordered.sort_by(|left, right| {
        left.priority()
            .cmp(&right.priority())
            .then_with(|| right.weight().total_cmp(&left.weight()))
    });

    let mut candidates: Vec<(u8, RepairMove)> = Vec::new();
    if attention_mismatch {
        candidates.push((0, RepairMove::RequestConfirmation));
    }
    for signal in &ordered {
        for seed in signal.seed_moves() {
            candidates.push((signal.priority(), *seed));
        }
    }

    let mut moves: Vec<RepairMove> = Vec::new();
    let mut questions = 0usize;
    for (_, candidate) in candidates {
        if moves.len() >= MAX_REPAIR_MOVES {
            break;
        }
        if moves.contains(&candidate) {
            continue;
        }
        if candidate.is_question() && questions >= MAX_QUESTIONS_PER_PLAN {
            continue;
        }
        if candidate.is_question() {
            questions += 1;
        }
        moves.push(candidate);
    }

    let ask_user = moves.iter().any(|repair_move| repair_move.is_question());

    let entries: Vec<(u8, f32, &'static str)> = ordered
        .iter()
        .map(|signal| (signal.priority(), signal.weight(), signal.kind_name()))
        .collect();
    let rationale = build_rationale(&entries, attention, &moves, questions);

    RepairPlan {
        moves,
        ask_user,
        rationale,
        max_moves: MAX_REPAIR_MOVES,
    }
}

/// 生成人读说明：只用种类名/权重/动作名，绝不拼接 `detail` 或 label 原文。
fn build_rationale(
    entries: &[(u8, f32, &'static str)],
    attention: Option<&JointAttention>,
    moves: &[RepairMove],
    questions: usize,
) -> String {
    let signal_summary = if entries.is_empty() {
        "信号 0 条（无）".to_string()
    } else {
        let listed: Vec<String> = entries
            .iter()
            .map(|(rank, weight, kind)| format!("rank{rank} {kind} w={weight:.2}"))
            .collect();
        format!("信号 {} 条（{}）", entries.len(), listed.join(" > "))
    };

    let attention_summary = match attention {
        None => "共同注意 无（未记录焦点）".to_string(),
        Some(snapshot) if snapshot.agree => {
            format!("共同注意 一致（{}）", snapshot.target.kind_name())
        }
        Some(snapshot) => format!("共同注意 不一致（{}）", snapshot.target.kind_name()),
    };

    let move_summary = if moves.is_empty() {
        "动作 0 个（本轮不发起修复动作）".to_string()
    } else {
        let names: Vec<&str> = moves.iter().map(RepairMove::as_str).collect();
        format!("动作 {} 个（{}）", moves.len(), names.join(" → "))
    };

    format!(
        "{signal_summary}；{attention_summary}；{move_summary}；上限 {MAX_REPAIR_MOVES} 个动作、\
         问句至多 {MAX_QUESTIONS_PER_PLAN} 个（本计划 {questions} 个）\
         ；规则=固定优先级表 rank 小者先、同 rank 按权重降序。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定任务焦点（label 只用于存储，不进入 rationale）。
    fn task_attention(agree: bool) -> JointAttention {
        JointAttention {
            target: JointAttentionTarget::Task {
                label: "SG-20 交互修复内核".to_string(),
            },
            agree,
        }
    }

    /// 六种信号各一条（用于穷举子集与顺序无关性测试）。
    fn sample_signals() -> Vec<UncertaintySignal> {
        vec![
            UncertaintySignal::LowConfidence {
                detail: "自评置信度偏低".to_string(),
                weight: 0.30,
            },
            UncertaintySignal::Contradiction {
                detail: "两轮结论互相矛盾".to_string(),
                weight: 0.80,
            },
            UncertaintySignal::Ambiguity {
                detail: "指代对象不唯一".to_string(),
                weight: 0.50,
            },
            UncertaintySignal::UserCorrection {
                detail: "用户指出上一轮结论有误".to_string(),
                weight: 0.90,
            },
            UncertaintySignal::ReferenceMissing {
                detail: "引用的文件不在上下文里".to_string(),
                weight: 0.60,
            },
            UncertaintySignal::PolarityFlip {
                detail: "同一问题的极性反转".to_string(),
                weight: 0.70,
            },
        ]
    }

    fn question_count(plan: &RepairPlan) -> usize {
        plan.moves
            .iter()
            .filter(|repair_move| repair_move.is_question())
            .count()
    }

    /// 硬不变量：≤2 动作、≤1 问句、动作不重复、`ask_user` 与问句一致、rationale 够长。
    fn assert_plan_shape(plan: &RepairPlan) {
        assert!(
            plan.moves.len() <= MAX_REPAIR_MOVES,
            "动作超过硬上限: {:?}",
            plan.moves
        );
        assert!(plan.moves.len() <= plan.max_moves);
        assert_eq!(plan.max_moves, MAX_REPAIR_MOVES);

        let questions = question_count(plan);
        assert!(
            questions <= MAX_QUESTIONS_PER_PLAN,
            "问句超过硬上限: {:?}",
            plan.moves
        );
        assert_eq!(plan.ask_user, questions > 0);

        for (index, repair_move) in plan.moves.iter().enumerate() {
            assert!(
                !plan.moves[..index].contains(repair_move),
                "动作重复: {:?}",
                plan.moves
            );
        }
        assert!(
            plan.rationale.split_whitespace().count() >= 6,
            "rationale 过短: {}",
            plan.rationale
        );
    }

    #[test]
    fn user_correction_acknowledges_then_restates() {
        let plan = plan_repair(
            &[UncertaintySignal::UserCorrection {
                detail: "用户指出上一轮结论有误".to_string(),
                weight: 0.90,
            }],
            None,
        );
        assert_eq!(
            plan.moves,
            vec![
                RepairMove::AcknowledgeUncertainty,
                RepairMove::RestateUnderstanding
            ]
        );
        assert!(!plan.ask_user, "纠正类修复不应追加问句");
        assert_plan_shape(&plan);
    }

    #[test]
    fn contradiction_cites_evidence_then_requests_confirmation() {
        let plan = plan_repair(
            &[UncertaintySignal::Contradiction {
                detail: "两轮结论互相矛盾".to_string(),
                weight: 0.80,
            }],
            None,
        );
        assert_eq!(
            plan.moves,
            vec![RepairMove::CiteEvidence, RepairMove::RequestConfirmation]
        );
        assert!(plan.ask_user);
        assert_plan_shape(&plan);
    }

    #[test]
    fn ambiguity_asks_one_clarifying_question() {
        let plan = plan_repair(
            &[UncertaintySignal::Ambiguity {
                detail: "指代对象不唯一".to_string(),
                weight: 0.50,
            }],
            None,
        );
        assert_eq!(plan.moves, vec![RepairMove::AskClarifying]);
        assert!(plan.ask_user);
        assert_eq!(question_count(&plan), 1);
        assert_plan_shape(&plan);
    }

    #[test]
    fn reference_missing_offers_rollback_then_clarifies() {
        let plan = plan_repair(
            &[UncertaintySignal::ReferenceMissing {
                detail: "引用的文件不在上下文里".to_string(),
                weight: 0.60,
            }],
            None,
        );
        assert_eq!(
            plan.moves,
            vec![RepairMove::OfferRollback, RepairMove::AskClarifying]
        );
        assert!(plan.ask_user);
        assert_plan_shape(&plan);
    }

    #[test]
    fn polarity_flip_restates_then_requests_confirmation() {
        let plan = plan_repair(
            &[UncertaintySignal::PolarityFlip {
                detail: "同一问题的极性反转".to_string(),
                weight: 0.70,
            }],
            None,
        );
        assert_eq!(
            plan.moves,
            vec![
                RepairMove::RestateUnderstanding,
                RepairMove::RequestConfirmation
            ]
        );
        assert!(plan.ask_user);
        assert_plan_shape(&plan);
    }

    #[test]
    fn low_confidence_acknowledges_without_asking() {
        let plan = plan_repair(
            &[UncertaintySignal::LowConfidence {
                detail: "自评置信度偏低".to_string(),
                weight: 0.30,
            }],
            None,
        );
        assert_eq!(plan.moves, vec![RepairMove::AcknowledgeUncertainty]);
        assert!(!plan.ask_user);
        assert_plan_shape(&plan);
    }

    #[test]
    fn multi_signal_priority_is_rank_first_not_weight_first() {
        let weak_correction = UncertaintySignal::UserCorrection {
            detail: "用户说不是这个意思".to_string(),
            weight: 0.05,
        };
        let strong_contradiction = UncertaintySignal::Contradiction {
            detail: "两轮结论冲突".to_string(),
            weight: 0.99,
        };
        let strong_low_confidence = UncertaintySignal::LowConfidence {
            detail: "自评 0.1".to_string(),
            weight: 0.99,
        };

        let forward = plan_repair(
            &[
                weak_correction.clone(),
                strong_contradiction.clone(),
                strong_low_confidence.clone(),
            ],
            None,
        );
        assert_eq!(
            forward.moves,
            vec![
                RepairMove::AcknowledgeUncertainty,
                RepairMove::RestateUnderstanding
            ],
            "rank 1 的显式纠正必须先于权重更高的矛盾/低置信"
        );
        assert!(!forward.ask_user);

        let backward = plan_repair(
            &[strong_low_confidence, strong_contradiction, weak_correction],
            None,
        );
        assert_eq!(backward, forward, "输入顺序不得改变计划（含 rationale）");
        assert_plan_shape(&forward);
    }

    #[test]
    fn question_budget_is_shared_across_signals() {
        let plan = plan_repair(&sample_signals(), None);
        assert_eq!(
            plan.moves,
            vec![
                RepairMove::AcknowledgeUncertainty,
                RepairMove::RestateUnderstanding
            ],
            "多信号时高 rank 动作先占满 2 个名额，问句被挤掉而不是堆积"
        );
        assert!(!plan.ask_user);
        assert_eq!(question_count(&plan), 0);
        assert_plan_shape(&plan);
    }

    #[test]
    fn attention_disagreement_requests_confirmation_before_content() {
        let disagree = task_attention(false);
        let plan = plan_repair(&[], Some(&disagree));
        assert_eq!(plan.moves, vec![RepairMove::RequestConfirmation]);
        assert!(plan.ask_user);
        assert_plan_shape(&plan);

        let with_low_confidence = plan_repair(
            &[UncertaintySignal::LowConfidence {
                detail: "自评偏低".to_string(),
                weight: 0.90,
            }],
            Some(&disagree),
        );
        assert_eq!(
            with_low_confidence.moves,
            vec![
                RepairMove::RequestConfirmation,
                RepairMove::AcknowledgeUncertainty
            ],
            "注意力不一致排 rank 0，先对齐对象再承认不确定"
        );
        assert_plan_shape(&with_low_confidence);

        let agree = task_attention(true);
        let aligned = plan_repair(&[], Some(&agree));
        assert!(aligned.moves.is_empty(), "注意力一致且无信号时不应发起修复");
        assert!(!aligned.ask_user);
        assert_plan_shape(&aligned);
    }

    #[test]
    fn empty_signals_stay_silent() {
        let plan = plan_repair(&[], None);
        assert!(plan.moves.is_empty());
        assert!(!plan.ask_user);
        assert_eq!(plan.max_moves, MAX_REPAIR_MOVES);
        assert!(plan.rationale.contains("信号 0 条"));
        assert_plan_shape(&plan);
    }

    #[test]
    fn plan_is_deterministic_and_order_invariant() {
        let signals = sample_signals();
        let attention = task_attention(false);
        let baseline = plan_repair(&signals, Some(&attention));
        for _ in 0..16 {
            assert_eq!(
                plan_repair(&signals, Some(&attention)),
                baseline,
                "同输入必须同输出"
            );
        }
        let mut reversed = signals;
        reversed.reverse();
        assert_eq!(
            plan_repair(&reversed, Some(&attention)),
            baseline,
            "信号顺序不得改变计划"
        );
    }

    #[test]
    fn every_subset_respects_two_move_and_one_question_invariants() {
        let pool = sample_signals();
        let agree = task_attention(true);
        let disagree = task_attention(false);
        for mask in 0u8..64 {
            let subset: Vec<UncertaintySignal> = pool
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1u8 << *index) != 0)
                .map(|(_, signal)| signal.clone())
                .collect();
            for attention in [None, Some(&agree), Some(&disagree)] {
                let plan = plan_repair(&subset, attention);
                assert_plan_shape(&plan);
            }
        }
    }

    #[test]
    fn rationale_never_echoes_user_text_or_blames() {
        let hostile_detail = "你错了，你根本不懂我在说什么".to_string();
        let hostile_label = "你错了".to_string();
        let signals = vec![
            UncertaintySignal::UserCorrection {
                detail: hostile_detail.clone(),
                weight: 0.80,
            },
            UncertaintySignal::Ambiguity {
                detail: hostile_detail.clone(),
                weight: 0.70,
            },
        ];
        let attention = JointAttention {
            target: JointAttentionTarget::ScreenRegion {
                label: hostile_label.clone(),
            },
            agree: false,
        };
        let plan = plan_repair(&signals, Some(&attention));

        let forbidden = [
            "你错了",
            "你不懂",
            "你搞错",
            "又不",
            "显然",
            "为什么不",
            "？",
            "?",
        ];
        for needle in forbidden {
            assert!(
                !plan.rationale.contains(needle),
                "rationale 含禁用片段 {needle}: {}",
                plan.rationale
            );
        }
        assert!(
            !plan.rationale.contains(&hostile_detail),
            "rationale 回灌了 detail 原文: {}",
            plan.rationale
        );
        assert!(
            !plan.rationale.contains(&hostile_label),
            "rationale 回灌了 label 原文: {}",
            plan.rationale
        );
        // 输入本身不被改写（只是不进入 rationale）。
        assert_eq!(signals[0].detail(), hostile_detail);
        assert_plan_shape(&plan);
    }

    #[test]
    fn wire_format_roundtrips_and_moves_stay_textless() {
        let signal = UncertaintySignal::ReferenceMissing {
            detail: "文件不在上下文".to_string(),
            weight: 0.60,
        };
        let signal_json = serde_json::to_string(&signal).expect("signal 序列化");
        assert!(signal_json.contains("\"kind\":\"reference_missing\""));
        let signal_back: UncertaintySignal =
            serde_json::from_str(&signal_json).expect("signal 反序列化");
        assert_eq!(signal_back, signal);

        let attention = task_attention(false);
        let attention_json = serde_json::to_string(&attention).expect("attention 序列化");
        let attention_back: JointAttention =
            serde_json::from_str(&attention_json).expect("attention 反序列化");
        assert_eq!(attention_back, attention);

        let plan = plan_repair(&[signal], Some(&attention));
        let plan_json = serde_json::to_string(&plan).expect("plan 序列化");
        let plan_back: RepairPlan = serde_json::from_str(&plan_json).expect("plan 反序列化");
        assert_eq!(plan_back, plan);

        // 动作是固定枚举：unit variant 序列化为纯字符串，没有可塞自由文本的载荷。
        let move_json = serde_json::to_value(RepairMove::AskClarifying).expect("move 序列化");
        assert_eq!(
            move_json,
            serde_json::Value::String("ask_clarifying".to_string())
        );
    }
}
