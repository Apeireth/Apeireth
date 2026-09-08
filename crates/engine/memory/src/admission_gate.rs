//! B2 · RA-15 P1-A: 记忆准入控制（Research 前缀，默认关闭）。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**（吸收自 arXiv:2603.04549 A-MAC）: 记忆准入是弱规范、弱控制的
//!   组件——要么积累大量幻觉/过时内容，要么依赖不透明的全 LLM 策略。
//! - **对手框架**: 把记忆价值分解为五个可解释因子——未来效用、事实置信、
//!   语义新颖度、时间新近度、内容类型先验；规则化特征 + 单次 LLM 辅助效用
//!   评估；策略经交叉验证优化。
//! - **工程版本**: 纯规则打分器（LLM 效用评估留 trait 口，0 装）——
//!   `AdmissionSignal` 五因子 → 加权分 → `Admit` / `PendingReview`；
//!   来源类型先验复用 `bitemporal_graph::TrustWeights`（RA-2 §5.2 既有 w 表）；
//!   所有准入决定写 append-only 审计事件（`research_lineage_events`）。
//! - **与 StackPin 的分层**: 准入管"该不该记"，StackPin 管"记了之后带哪些进
//!   考场"——两层正交（P1 论文 related work 差异点）。
//! - **默认关闭（铁律 1）**: 不挂任何生产写入路径；旧写入语义零改变。
//! - **引用**: arXiv:2603.04549；逐行对照见
//!   `docs/03-reference/absorption-2026-09.md` §P1-A。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::bitemporal_graph::{FactProvenance, TrustWeights};
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 准入五因子信号（A-MAC 分解的工程映射）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdmissionSignal {
    /// 内容类型先验（来源类型，复用 RA-2 w 表）。
    pub provenance: FactProvenance,
    /// 事实置信/grounding 信号 [0,1]（幻觉信号 ⇒ 低值）。
    pub confidence: f32,
    /// 语义新颖度 [0,1]（与既有记忆的重复度反向）。
    pub novelty: f32,
    /// 未来效用 [0,1]（调用方估计；LLM 效用评估留 trait 口）。
    pub utility: f32,
    /// 时间新近度 [0,1]（越新越接近 1）。
    pub recency: f32,
}

/// 准入策略（可配置；默认值 = 五因子等权 + 0.5 阈值）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionPolicy {
    pub weights: AdmissionWeights,
    /// score >= threshold ⇒ Admit；否则 PendingReview。
    pub threshold: f32,
}

/// 五因子权重。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdmissionWeights {
    pub w_type_prior: f32,
    pub w_confidence: f32,
    pub w_novelty: f32,
    pub w_utility: f32,
    pub w_recency: f32,
    /// 与既有事实冲突度的惩罚权重（A-MAC 未显式列为正因子，
    /// 我们作为冲突减分项并入——冲突内容新颖但不可靠）。
    pub w_conflict_penalty: f32,
}

impl Default for AdmissionWeights {
    fn default() -> Self {
        Self {
            w_type_prior: 1.0,
            w_confidence: 1.0,
            w_novelty: 1.0,
            w_utility: 1.0,
            w_recency: 1.0,
            w_conflict_penalty: 1.0,
        }
    }
}

impl Default for AdmissionPolicy {
    fn default() -> Self {
        Self {
            weights: AdmissionWeights::default(),
            threshold: 0.5,
        }
    }
}

/// 准入决定（全部进审计日志，含分因子明细）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdmissionDecision {
    Admit,
    PendingReview,
}

/// 一次准入判决的完整记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionVerdict {
    pub decision: AdmissionDecision,
    /// 加权总分（五因子 + 冲突惩罚）。
    pub score: f32,
    /// 分因子贡献（审计可见，A-MAC 可解释性）。
    pub breakdown: HashMap<String, f32>,
}

/// 记忆准入门（纯规则打分器；默认关闭，不挂生产路径）。
#[derive(Debug, Clone, Default)]
pub struct ResearchAdmissionGate {
    pub policy: AdmissionPolicy,
    pub trust: TrustWeights,
}

impl ResearchAdmissionGate {
    pub fn new(policy: AdmissionPolicy, trust: TrustWeights) -> Self {
        Self { policy, trust }
    }

    /// 准入判决：score = w·[type_prior·confidence, novelty, utility, recency] − conflict·w_conflict_penalty
    /// （type_prior 与 confidence 按 A-MAC"内容类型先验是最可靠因子"结论相乘，
    /// 其余因子线性加权；conflict 作为惩罚项）。
    pub fn adjudicate(&self, signal: &AdmissionSignal, conflict: f32) -> AdmissionVerdict {
        let w = self.policy.weights;
        let type_prior = self.trust.w(signal.provenance);
        let score = w.w_type_prior * type_prior * signal.confidence
            + w.w_novelty * signal.novelty
            + w.w_utility * signal.utility
            + w.w_recency * signal.recency
            - w.w_conflict_penalty * conflict;
        let mut breakdown = HashMap::new();
        breakdown.insert("type_prior".into(), type_prior);
        breakdown.insert("confidence".into(), signal.confidence);
        breakdown.insert("novelty".into(), signal.novelty);
        breakdown.insert("utility".into(), signal.utility);
        breakdown.insert("recency".into(), signal.recency);
        breakdown.insert("conflict".into(), conflict);
        let decision = if score >= self.policy.threshold {
            AdmissionDecision::Admit
        } else {
            AdmissionDecision::PendingReview
        };
        AdmissionVerdict {
            decision,
            score,
            breakdown,
        }
    }

    /// 把准入决定写入 append-only 审计事件（`research_lineage_events`）。
    /// 低分待审 = 仍登记待审区（写入事件，不写产品表——0 装）。
    pub fn record_decision(
        &self,
        store: &SqliteMemoryStore,
        subject_id: &str,
        verdict: &AdmissionVerdict,
        actor: Option<&str>,
    ) -> MemoryResult<i64> {
        let detail = serde_json::json!({
            "subject_id": subject_id,
            "decision": match verdict.decision {
                AdmissionDecision::Admit => "admit",
                AdmissionDecision::PendingReview => "pending_review",
            },
            "score": verdict.score,
            "breakdown": verdict.breakdown,
        });
        store.research_write_event("admission", actor, None, subject_id, &detail)
    }
}

// 冲突度 trait 口（0 装）：与既有事实的冲突度由部署层供给
// （候选实现: bitemporal_graph 事实匹配 + 语义轴距离）。
pub trait ConflictScorer: Send + Sync {
    fn conflict(&self, candidate_text: &str, existing_facts: &[&str]) -> f32;
}

/// 确定性冲突打分 stub（测试/基准用）: 与任一既有事实的共享关键词比例。
#[derive(Debug, Clone, Default)]
pub struct KeywordConflictScorer;

impl ConflictScorer for KeywordConflictScorer {
    fn conflict(&self, candidate_text: &str, existing_facts: &[&str]) -> f32 {
        if existing_facts.is_empty() {
            return 0.0;
        }
        let tokens: Vec<&str> = candidate_text.split_whitespace().collect();
        if tokens.is_empty() {
            return 0.0;
        }
        existing_facts
            .iter()
            .map(|fact| {
                let ft: Vec<&str> = fact.split_whitespace().collect();
                let overlap = ft.iter().filter(|t| tokens.contains(t)).count();
                overlap as f32 / ft.len().max(1) as f32
            })
            .fold(0.0f32, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> ResearchAdmissionGate {
        ResearchAdmissionGate::new(AdmissionPolicy::default(), TrustWeights::default())
    }

    /// 高可信人工注入 + 无冲突 ⇒ 准入.
    #[test]
    fn high_confidence_manual_admitted() {
        let g = gate();
        let v = g.adjudicate(
            &AdmissionSignal {
                provenance: FactProvenance::Manual,
                confidence: 1.0,
                novelty: 1.0,
                utility: 0.8,
                recency: 1.0,
            },
            0.0,
        );
        assert_eq!(v.decision, AdmissionDecision::Admit);
        assert!(v.score > 0.5);
    }

    /// 低置信对话提取 + 高冲突 + 低新颖/低效用 ⇒ 待审 (A-MAC 内容类型先验最可靠).
    #[test]
    fn low_confidence_dialog_with_conflict_pending_review() {
        let g = gate();
        let v = g.adjudicate(
            &AdmissionSignal {
                provenance: FactProvenance::Dialog,
                confidence: 0.3,
                novelty: 0.1,
                utility: 0.1,
                recency: 0.5,
            },
            0.9,
        );
        assert_eq!(v.decision, AdmissionDecision::PendingReview);
        assert!(v.score < 0.5);
    }

    /// 来源类型先验起作用: 同信号下 reflection 比分低于 manual.
    #[test]
    fn type_prior_penalizes_reflection() {
        let g = gate();
        let sig = |p| AdmissionSignal {
            provenance: p,
            confidence: 1.0,
            novelty: 1.0,
            utility: 1.0,
            recency: 1.0,
        };
        let manual = g.adjudicate(&sig(FactProvenance::Manual), 0.0);
        let refl = g.adjudicate(&sig(FactProvenance::Reflection), 0.0);
        assert!(manual.score > refl.score);
    }

    /// 决定写入审计事件 (append-only, seq 递增).
    #[test]
    fn decision_recorded_to_audit_log() {
        let s = SqliteMemoryStore::open_in_memory().unwrap();
        let g = gate();
        let v = g.adjudicate(
            &AdmissionSignal {
                provenance: FactProvenance::Tool,
                confidence: 0.9,
                novelty: 0.5,
                utility: 0.7,
                recency: 0.8,
            },
            0.1,
        );
        let seq1 = g.record_decision(&s, "fact-1", &v, Some("p1a")).unwrap();
        let seq2 = g.record_decision(&s, "fact-2", &v, Some("p1a")).unwrap();
        assert!(seq2 > seq1, "审计事件必须 append-only 递增");
        // 表只 INSERT: 再次重放同决定产生新事件 (0 装: 不幂等覆盖审计).
        let seq3 = g.record_decision(&s, "fact-1", &v, Some("p1a")).unwrap();
        assert!(seq3 > seq2);
    }

    /// 冲突打分 stub: 共享关键词比例.
    #[test]
    fn keyword_conflict_scorer_ratio() {
        let scorer = KeywordConflictScorer;
        assert!(
            (scorer.conflict("the sky is blue", &["the sky is red"]) - 0.75).abs() < 1e-6,
            "共享 3/4 关键词 ⇒ 0.75"
        );
        assert_eq!(
            scorer.conflict("the sky is blue", &["ocean waves crash"]),
            0.0
        );
    }
}
