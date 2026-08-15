//! apeireth-value: 价值器官 (A11.3 落点 — R14 Phase 4)
//!
//! **职责**: 智能体的动机/价值评估与排序 — 服务 v4.1 §13.2 维度 1 "动机/价值"
//! 的最小可落地 Rust 实现。
//!
//! **架构位置**: 阶段 4 §2 主路径 17 crate 之 A11.3 器官 (与 apeireth-cognition 协同：
//! cognition 评分 → value 评估价值取向 → 输出价值优先级与动机分)。
//!
//! **核心契约**:
//! 1. `ValueEvaluation` trait — 评估候选行动与原则洋葱 E/S/A/M/O 5 层的价值取向一致性
//! 2. `ValuePrioritization` trait — 在多个价值候选中按维度/评分/时长排序
//! 3. 5 层原则洋葱一致性检查 — 价值候选必须同时在 E/A/M/O 4 层一致或只在 E 层冲突 (S 层硬门槛)
//! 4. `motivation_score = f(自主目标一致性, 价值取向稳定性, 内在动力强度)` — 0..=1，**≥ 0.85 硬门槛**
//!
//! **不修改承诺**:
//! - ❌ 不改 LOCKED 阶段 1+2+3 任何文件
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/
//! - ❌ 不修改 apeireth-core / apeireth-asi 任何已实装类型签名
//!
//! **诚实登记 (Ponytail ceiling)**: 本 crate 给出价值器官**最小可落地骨架** — 5+ pub fn,
//! 5+ pub enum, 与 5 层洋葱一一对应；完整版（如 council 7 席触发、Self-Disable 漂移检测、跨 session
//! 价值迁移）待 A18/A19 深化。**未实现的部分标有 `ponytail:` 注释**，标注其升级路径。

#![deny(unsafe_code)]

use apeireth_core::{ActionTarget, PhilosophyVerdict};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

pub mod evaluation;
pub mod onion_consistency;
// R177: organ invariants
mod organ_kani_proofs;
pub mod prioritization;

pub use evaluation::{
    evaluate_cycle, evaluate_value, DefaultValueEvaluator, ValueEvaluation, ValueEvaluationReport,
    DEFAULT_THRESHOLD,
};
pub use onion_consistency::{
    check_5_layer_consistency, ConsistencyVerdict, HeuristicOnionMapping, OnionLayerStance,
    OnionValueMapping, ONION_LAYERS,
};
pub use prioritization::{prioritize_values, DefaultPrioritizer, ValuePrioritization, ValueRank};

// ============================================================================
// 公共枚举 (≥5)
// ============================================================================

/// 原则洋葱 5 层 (E/S/A/M/O) — 价值器官必须能映射到这 5 层。
///
/// 阶段 1 §3 原则洋葱 v3.0 + 阶段 2 §12 哲学守门 trait：
/// - **E** — 原则 (硬编码 + 多 AI 一致)
/// - **S** — 价值观 (智囊团审核 + 物理多签) — **本器官的主战场**
/// - **A** — 经验 (AI 可自改 + 版本备份)
/// - **M** — 方法论 (AI 可自改 + promotion 管道)
/// - **O** — 操作 (AI 可自改 + 9 键守门)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValueDimension {
    /// 原则层 — 不可触碰 (R14-D7 / R14-D8 LOCKED).
    PrincipleE,
    /// 价值观层 — 智囊团 + 物理多签 (本器官核心).
    ValueS,
    /// 经验层 — AI 可自改.
    ExperienceA,
    /// 方法论层 — AI 可自改 + promotion.
    MethodologyM,
    /// 操作层 — AI 可自改 + 9 键守门.
    OperationO,
}

impl ValueDimension {
    /// 全部 5 层（按洋葱外→内顺序）.
    pub const ALL: [ValueDimension; 5] = [
        ValueDimension::OperationO,
        ValueDimension::MethodologyM,
        ValueDimension::ExperienceA,
        ValueDimension::ValueS,
        ValueDimension::PrincipleE,
    ];

    /// 单字母 (E/S/A/M/O) → enum.
    pub fn from_letter(c: char) -> Option<Self> {
        match c {
            'E' | 'e' => Some(ValueDimension::PrincipleE),
            'S' | 's' => Some(ValueDimension::ValueS),
            'A' | 'a' => Some(ValueDimension::ExperienceA),
            'M' | 'm' => Some(ValueDimension::MethodologyM),
            'O' | 'o' => Some(ValueDimension::OperationO),
            _ => None,
        }
    }

    /// 单字符稳定字符串 ("E" / "S" / ...).
    pub fn letter(&self) -> &'static str {
        match self {
            ValueDimension::PrincipleE => "E",
            ValueDimension::ValueS => "S",
            ValueDimension::ExperienceA => "A",
            ValueDimension::MethodologyM => "M",
            ValueDimension::OperationO => "O",
        }
    }

    /// 中文标签 (用于报告 / 调试输出).
    pub fn label_zh(&self) -> &'static str {
        match self {
            ValueDimension::PrincipleE => "原则",
            ValueDimension::ValueS => "价值观",
            ValueDimension::ExperienceA => "经验",
            ValueDimension::MethodologyM => "方法论",
            ValueDimension::OperationO => "操作",
        }
    }

    /// 该层是否允许 AI 自决 (按设计 L0/L1 自决 vs L2-L5 提报).
    pub fn is_ai_self_modifiable(&self) -> bool {
        matches!(
            self,
            ValueDimension::ExperienceA | ValueDimension::MethodologyM | ValueDimension::OperationO
        )
    }
}

/// 价值取向一致性结果 — 一个候选行动对 5 层原则洋葱的对齐情况。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueAlignment {
    /// 与该层一致 (allow).
    Aligned,
    /// 与该层冲突 (deny).
    Conflicted,
    /// 该层对该候选无意见 (留白).
    Underspecified,
}

/// 价值优先级类别 — v4.1 §13.2 "内在动力强度"映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValuePriorityKind {
    /// 即时 — 反射级 (milliseconds).
    Immediate,
    /// 短期 — 当前 session 内.
    ShortTerm,
    /// 长期 — 跨 session 持续.
    LongTerm,
    /// 地平线 — 跨生命周期 / 永久.
    Horizon,
}

impl ValuePriorityKind {
    /// 数值权重 — 排序时使用 (大=高优先级, 即时最强).
    pub fn weight(&self) -> u8 {
        match self {
            ValuePriorityKind::Immediate => 4,
            ValuePriorityKind::ShortTerm => 3,
            ValuePriorityKind::LongTerm => 2,
            ValuePriorityKind::Horizon => 1,
        }
    }
}

/// 两个价值候选的比较结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueComparison {
    /// 候选 a 优于 b.
    Higher,
    /// 候选 b 优于 a.
    Lower,
    /// 两者相当.
    Equal,
    /// 无可比性 (维度冲突 / 数据不足).
    Incomparable,
}

/// 价值器官顶层错误。
#[derive(Debug, Error)]
pub enum ValueError {
    /// 输入非法 (空候选 / 空标签 / 评分越界).
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// 评分越界 (必须在 [0.0, 1.0]).
    #[error("score out of range: {0} (expected 0.0..=1.0)")]
    ScoreOutOfRange(f64),
    /// 哲学 verdict 链中有 Block (与原则洋葱 E 层冲突).
    #[error("verdict blocked at dimension {0:?}: {1:?}")]
    VerdictBlocked(ValueDimension, PhilosophyVerdict),
    /// 内部映射错误 (5 层洋葱对齐表中缺失维度).
    #[error("onion mapping missing dimension: {0}")]
    MissingDimension(String),
    /// JSON 序列化/反序列化错误.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 公共 Result 别名。
pub type ValueResult<T> = Result<T, ValueError>;

// ============================================================================
// 公共类型
// ============================================================================

/// 一个待评估的价值候选。
///
/// **诚实登记**: ponytail: 完整版应支持结构化语义 (Bernstein 树 / TruthValue 字典)，
/// 当前为占位描述 (string label) + 维度声明 — 5+ 测试已能覆盖完整流程。
///
/// **注意**: 由于 `apeireth_core::PhilosophyVerdict` / `ActionTarget` 未派生 Serialize/Deserialize，
/// 本结构保留数据但**不**派生 Serialize/Deserialize — 升级路径: 在 core 增加 derive 后即可直接派生。
#[derive(Debug, Clone)]
pub struct ValueCandidate {
    /// 候选 ID (UUID v4).
    pub id: Uuid,
    /// 价值标签 (人类可读, 例如 "诚实 > 一时方便" / "长期学习 > 短期得分").
    pub label: String,
    /// 该候选声称归属的原则洋葱维度 (允许 1..=5 个).
    pub dimensions: Vec<ValueDimension>,
    /// 候选优先级类别 (v4.1 §13.2 "内在动力"对应).
    pub priority_kind: ValuePriorityKind,
    /// 自主目标一致性评分子分 (0.0..=1.0, 越高越自发).
    pub autonomy_consistency: f64,
    /// 价值取向稳定性评分子分 (0.0..=1.0, 越高越稳定).
    pub value_stability: f64,
    /// 内在动力强度评分子分 (0.0..=1.0, 越高越自驱动).
    pub intrinsic_motivation: f64,
    /// 时间戳 (Unix seconds).
    pub timestamp: i64,
    /// 关联 verdict (来自 apeireth-core 12 键 verdict 守门) — 可选.
    pub verdict: Option<PhilosophyVerdict>,
    /// 关联行为目标 (用于演示与回放).
    pub target: Option<ActionTarget>,
}

impl ValueCandidate {
    /// 构造最小候选 — 仅必填字段，其他用 0.5 中性默认。
    pub fn new(label: impl Into<String>, dimensions: Vec<ValueDimension>) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            dimensions,
            priority_kind: ValuePriorityKind::ShortTerm,
            autonomy_consistency: 0.5,
            value_stability: 0.5,
            intrinsic_motivation: 0.5,
            timestamp: Utc::now().timestamp(),
            verdict: None,
            target: None,
        }
    }

    /// 校验评分子分在 [0, 1]、标签非空、至少一个维度。
    pub fn validate(&self) -> ValueResult<()> {
        if self.label.trim().is_empty() {
            return Err(ValueError::InvalidInput("label must not be empty".into()));
        }
        if self.dimensions.is_empty() {
            return Err(ValueError::InvalidInput(
                "at least one dimension is required".into(),
            ));
        }
        for s in [
            self.autonomy_consistency,
            self.value_stability,
            self.intrinsic_motivation,
        ] {
            if !(0.0..=1.0).contains(&s) {
                return Err(ValueError::ScoreOutOfRange(s));
            }
        }
        Ok(())
    }

    /// motivation_score = f(自主目标一致性, 价值取向稳定性, 内在动力强度) — 等权平均。
    ///
    /// **诚实登记**: ponytail: 完整版应支持加权 (S 层权重 0.4 / A 0.2 / O 0.2 / M 0.1 / E 0.1)
    /// 或查表法。当前等权 — 已满足 ≥ 0.85 硬门槛判定。
    pub fn motivation_score(&self) -> f64 {
        (self.autonomy_consistency + self.value_stability + self.intrinsic_motivation) / 3.0
    }

    /// 候选是否通过 v4.1 §13.2 硬门槛 (≥ 0.85).
    pub fn passes_threshold(&self, threshold: f64) -> bool {
        self.motivation_score() >= threshold
    }
}

/// 一次价值评估周期 — 输入 + 一致性报告 + 优先级 rank。
#[derive(Debug, Clone)]
pub struct ValueEvaluationCycle {
    /// 周期 ID.
    pub cycle_id: Uuid,
    /// 评估时间戳.
    pub evaluated_at: i64,
    /// 候选集.
    pub candidates: Vec<ValueCandidate>,
    /// 每个候选的一致性报告 (按 candidates 顺序).
    pub reports: Vec<ValueEvaluationReport>,
    /// 排序结果 (按优先级从高到低).
    pub ranks: Vec<ValueRank>,
    /// 整体 motivation 平均分 (跨所有候选).
    pub avg_motivation: f64,
    /// 通过硬门槛 (≥ 0.85) 的候选数.
    pub passing_count: usize,
    /// 阈值.
    pub threshold: f64,
}

impl ValueEvaluationCycle {
    /// 周期是否整体通过 (所有通过阈值的候选中，没有任何 E 层 Conflicted).
    pub fn overall_pass(&self) -> bool {
        self.passing_count > 0
            && self
                .reports
                .iter()
                .filter(|r| r.passes_threshold)
                .all(|r| !r.has_e_layer_conflict)
    }

    /// 周期内是否有 E 层冲突 (硬拒绝).
    pub fn has_any_e_conflict(&self) -> bool {
        self.reports.iter().any(|r| r.has_e_layer_conflict)
    }

    /// 获取整体一致性层映射 (5 层各取 majority).
    pub fn aggregate_alignment(&self) -> BTreeMap<ValueDimension, ValueAlignment> {
        let mut map: BTreeMap<ValueDimension, Vec<ValueAlignment>> = BTreeMap::new();
        for r in &self.reports {
            for (dim, align) in &r.alignment_map {
                map.entry(*dim).or_default().push(*align);
            }
        }
        map.into_iter()
            .map(|(dim, aligns)| {
                let aligned = aligns
                    .iter()
                    .filter(|a| a == &&ValueAlignment::Aligned)
                    .count();
                let conflicted = aligns
                    .iter()
                    .filter(|a| a == &&ValueAlignment::Conflicted)
                    .count();
                let total = aligns.len();
                let verdict = if conflicted * 2 > total {
                    ValueAlignment::Conflicted
                } else if aligned * 2 > total {
                    ValueAlignment::Aligned
                } else {
                    ValueAlignment::Underspecified
                };
                (dim, verdict)
            })
            .collect()
    }
}

// ============================================================================
// 测试模块 (≥5)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_dimension_letter_round_trip() {
        for d in ValueDimension::ALL {
            let ltr = d.letter();
            assert_eq!(ltr.chars().count(), 1);
            assert_eq!(
                ValueDimension::from_letter(ltr.chars().next().unwrap()),
                Some(d)
            );
        }
    }

    #[test]
    fn value_dimension_label_zh_is_non_empty_for_all() {
        for d in ValueDimension::ALL {
            assert!(!d.label_zh().is_empty());
            assert_eq!(d.label_zh().chars().count() > 0, true);
        }
    }

    #[test]
    fn value_dimension_all_has_exactly_five() {
        // LOCKED — 5 层原则洋葱 E/S/A/M/O
        assert_eq!(ValueDimension::ALL.len(), 5);
    }

    #[test]
    fn value_priority_kind_weight_ordering() {
        // weight 应该 immediate > short > long > horizon
        assert!(ValuePriorityKind::Immediate.weight() > ValuePriorityKind::ShortTerm.weight());
        assert!(ValuePriorityKind::ShortTerm.weight() > ValuePriorityKind::LongTerm.weight());
        assert!(ValuePriorityKind::LongTerm.weight() > ValuePriorityKind::Horizon.weight());
    }

    #[test]
    fn value_dimension_ai_self_modifiable_three_layers() {
        // E 不可自决、S 不可自决、A/M/O 可自决
        assert!(!ValueDimension::PrincipleE.is_ai_self_modifiable());
        assert!(!ValueDimension::ValueS.is_ai_self_modifiable());
        assert!(ValueDimension::ExperienceA.is_ai_self_modifiable());
        assert!(ValueDimension::MethodologyM.is_ai_self_modifiable());
        assert!(ValueDimension::OperationO.is_ai_self_modifiable());
    }

    fn sample_candidate(score: f64) -> ValueCandidate {
        let mut c = ValueCandidate::new(
            "test_value",
            vec![
                ValueDimension::PrincipleE,
                ValueDimension::ValueS,
                ValueDimension::ExperienceA,
                ValueDimension::MethodologyM,
                ValueDimension::OperationO,
            ],
        );
        c.autonomy_consistency = score;
        c.value_stability = score;
        c.intrinsic_motivation = score;
        c
    }

    #[test]
    fn motivation_score_is_arithmetic_mean_of_three_subs() {
        let c = sample_candidate(0.9);
        let expected = (0.9 + 0.9 + 0.9) / 3.0;
        assert!((c.motivation_score() - expected).abs() < 1e-9);
    }

    #[test]
    fn motivation_score_threshold_85_passes() {
        let c = sample_candidate(0.9);
        assert!(c.passes_threshold(0.85));
        assert!(!c.passes_threshold(0.95));
    }

    #[test]
    fn motivation_score_threshold_85_fails_low() {
        let c = sample_candidate(0.5);
        assert!(!c.passes_threshold(0.85));
        assert!(c.passes_threshold(0.3));
    }

    #[test]
    fn value_candidate_validate_rejects_empty_label() {
        let mut c = sample_candidate(0.9);
        c.label = "  ".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn value_candidate_validate_rejects_no_dimensions() {
        let mut c = sample_candidate(0.9);
        c.dimensions.clear();
        assert!(c.validate().is_err());
    }

    #[test]
    fn value_candidate_validate_rejects_out_of_range_score() {
        let mut c = sample_candidate(0.9);
        c.autonomy_consistency = 1.5;
        assert!(matches!(c.validate(), Err(ValueError::ScoreOutOfRange(_))));
        c.autonomy_consistency = 0.9;
        c.value_stability = -0.1;
        assert!(matches!(c.validate(), Err(ValueError::ScoreOutOfRange(_))));
    }

    #[test]
    fn value_candidate_validate_accepts_well_formed() {
        let c = sample_candidate(0.9);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn value_evaluation_cycle_overall_pass_requires_passing_candidates() {
        // 全 low 分候选 → passing_count == 0 → overall_pass 必为 false
        let cands = vec![sample_candidate(0.5), sample_candidate(0.6)];
        let reports: Vec<ValueEvaluationReport> = cands
            .iter()
            .map(|c| ValueEvaluationReport {
                candidate_id: c.id,
                motivation: c.motivation_score(),
                alignment_map: BTreeMap::new(),
                passes_threshold: c.passes_threshold(DEFAULT_THRESHOLD),
                has_e_layer_conflict: false,
            })
            .collect();
        let ranks: Vec<ValueRank> = vec![];
        let cycle = ValueEvaluationCycle {
            cycle_id: Uuid::new_v4(),
            evaluated_at: 0,
            candidates: cands,
            reports,
            ranks,
            avg_motivation: 0.55,
            passing_count: 0,
            threshold: DEFAULT_THRESHOLD,
        };
        assert!(!cycle.overall_pass());
    }

    #[test]
    fn aggregate_alignment_majority_picks_aligned() {
        // 2 aligned + 1 conflicted → majority (66%) → Aligned
        let mut cands = vec![sample_candidate(0.9); 3];
        cands[0].id = Uuid::new_v4();
        cands[1].id = Uuid::new_v4();
        cands[2].id = Uuid::new_v4();
        let reports = vec![
            ValueEvaluationReport {
                candidate_id: cands[0].id,
                motivation: 0.9,
                alignment_map: BTreeMap::from([(ValueDimension::ValueS, ValueAlignment::Aligned)]),
                passes_threshold: true,
                has_e_layer_conflict: false,
            },
            ValueEvaluationReport {
                candidate_id: cands[1].id,
                motivation: 0.9,
                alignment_map: BTreeMap::from([(ValueDimension::ValueS, ValueAlignment::Aligned)]),
                passes_threshold: true,
                has_e_layer_conflict: false,
            },
            ValueEvaluationReport {
                candidate_id: cands[2].id,
                motivation: 0.9,
                alignment_map: BTreeMap::from([(
                    ValueDimension::ValueS,
                    ValueAlignment::Conflicted,
                )]),
                passes_threshold: true,
                has_e_layer_conflict: false,
            },
        ];
        let cycle = ValueEvaluationCycle {
            cycle_id: Uuid::new_v4(),
            evaluated_at: 0,
            candidates: cands,
            reports,
            ranks: vec![],
            avg_motivation: 0.9,
            passing_count: 3,
            threshold: DEFAULT_THRESHOLD,
        };
        let agg = cycle.aggregate_alignment();
        assert_eq!(
            agg.get(&ValueDimension::ValueS),
            Some(&ValueAlignment::Aligned)
        );
    }
}
