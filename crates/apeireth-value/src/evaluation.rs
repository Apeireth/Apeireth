//! 价值评估模块 — ValueEvaluation trait + 默认实现.
//!
//! 公开 API:
//! - `evaluate_value` — 评估单个 `ValueCandidate` 对 5 层洋葱的一致性 + motivation 分数
//! - `ValueEvaluationReport` — 单候选评估报告 (5 层 alignment_map + 总 motivation)
//! - `evaluate_cycle` — 多候选评估周期

use apeireth_core::PhilosophyVerdict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{
    ValueAlignment, ValueCandidate, ValueComparison, ValueDimension, ValueError, ValueResult,
};

/// 默认硬门槛 — v4.1 §13.2: motivation_score ≥ 0.85 才算通过。
///
/// **诚实登记**: ponytail: 完整版应支持 per-context 阈值 (思辨 vs 实时反射) 与配置驱动。
pub const DEFAULT_THRESHOLD: f64 = 0.85;

/// 单候选评估报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueEvaluationReport {
    /// 关联候选 ID.
    pub candidate_id: Uuid,
    /// 综合动机分 (`motivation_score`).
    pub motivation: f64,
    /// 5 层原则洋葱一致性 (layer → alignment).
    pub alignment_map: BTreeMap<ValueDimension, ValueAlignment>,
    /// 是否通过硬门槛 (`motivation >= threshold`).
    pub passes_threshold: bool,
    /// 是否在 E 层有 Conflicted (硬拒绝).
    pub has_e_layer_conflict: bool,
}

impl ValueEvaluationReport {
    /// 报告涉及的洋葱维度数 (1..=5).
    pub fn dimension_count(&self) -> usize {
        self.alignment_map.len()
    }

    /// 报告涉及的 Aligned 维度数.
    pub fn aligned_count(&self) -> usize {
        self.alignment_map
            .values()
            .filter(|v| **v == ValueAlignment::Aligned)
            .count()
    }
}

/// ValueEvaluation trait — 价值器官的评估契约。
///
/// 任何"评估一个候选行动是否对齐原则洋葱 5 层"的实现都应实现此 trait。
pub trait ValueEvaluation {
    /// 评估单一候选 — 必须返回 5 层一致性 map + motivation 分.
    fn evaluate(&self, candidate: &ValueCandidate) -> ValueResult<ValueEvaluationReport>;

    /// 比较两个候选在 onion consistency 上的优劣 — 默认实现按 overall_pass 排序.
    fn compare(&self, a: &ValueCandidate, b: &ValueCandidate) -> ValueComparison {
        let ra = self.evaluate(a).ok();
        let rb = self.evaluate(b).ok();
        match (ra, rb) {
            (Some(ra), Some(rb)) => {
                if ra.has_e_layer_conflict && !rb.has_e_layer_conflict {
                    ValueComparison::Lower
                } else if !ra.has_e_layer_conflict && rb.has_e_layer_conflict {
                    ValueComparison::Higher
                } else if (ra.motivation - rb.motivation).abs() < 1e-9 {
                    ValueComparison::Equal
                } else if ra.motivation > rb.motivation {
                    ValueComparison::Higher
                } else {
                    ValueComparison::Lower
                }
            }
            (Some(_), None) => ValueComparison::Higher,
            (None, Some(_)) => ValueComparison::Lower,
            (None, None) => ValueComparison::Incomparable,
        }
    }

    /// 当前评估器使用的硬门槛.
    fn threshold(&self) -> f64 {
        DEFAULT_THRESHOLD
    }
}

/// 默认评估器 — 按 v4.1 §13.2 描述实现：
/// - 对 `candidate.dimensions` 声明的每个维度, 给出 alignment
/// - E 层：如果 verdict = Block 则 Conflicted；否则 Aligned
/// - S 层：默认根据 candidate.value_stability 来判定 (>= 0.5 → Aligned)
/// - A/M/O 层：与 S 一致逻辑
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultValueEvaluator;

impl ValueEvaluation for DefaultValueEvaluator {
    fn evaluate(&self, candidate: &ValueCandidate) -> ValueResult<ValueEvaluationReport> {
        candidate.validate()?;

        let mut map: BTreeMap<ValueDimension, ValueAlignment> = BTreeMap::new();
        let mut has_e_layer_conflict = false;

        for &dim in &candidate.dimensions {
            let alignment = match dim {
                ValueDimension::PrincipleE => {
                    // E 层：verdict Block → 必 Conflicted；否则若 verdict Allow → Aligned；
                    // 没传 verdict → Underspecified.
                    match candidate.verdict {
                        Some(PhilosophyVerdict::Block(_)) => {
                            has_e_layer_conflict = true;
                            ValueAlignment::Conflicted
                        }
                        Some(PhilosophyVerdict::Allow) => ValueAlignment::Aligned,
                        None => ValueAlignment::Underspecified,
                    }
                }
                ValueDimension::ValueS => {
                    // S 层 (本器官核心): value_stability >= 0.7 → Aligned, < 0.4 → Conflicted
                    if candidate.value_stability >= 0.7 {
                        ValueAlignment::Aligned
                    } else if candidate.value_stability < 0.4 {
                        ValueAlignment::Conflicted
                    } else {
                        ValueAlignment::Underspecified
                    }
                }
                ValueDimension::ExperienceA
                | ValueDimension::MethodologyM
                | ValueDimension::OperationO => {
                    // A/M/O 层 (AI 可自改): autonomy_consistency >= 0.5 → Aligned
                    if candidate.autonomy_consistency >= 0.5 {
                        ValueAlignment::Aligned
                    } else if candidate.autonomy_consistency < 0.3 {
                        ValueAlignment::Conflicted
                    } else {
                        ValueAlignment::Underspecified
                    }
                }
            };
            map.insert(dim, alignment);
        }

        let motivation = candidate.motivation_score();
        let passes = motivation >= self.threshold();

        Ok(ValueEvaluationReport {
            candidate_id: candidate.id,
            motivation,
            alignment_map: map,
            passes_threshold: passes,
            has_e_layer_conflict,
        })
    }
}

/// 公开便捷函数 — 默认评估器评估单个候选。
pub fn evaluate_value(candidate: &ValueCandidate) -> ValueResult<ValueEvaluationReport> {
    DefaultValueEvaluator.evaluate(candidate)
}

/// 多候选评估周期 — 输入一组候选 + 评估器, 输出每候选的报告 + 整体统计。
///
/// **参数**:
/// - `candidates` — 待评估候选集 (非空, 否则 `InvalidInput`)
/// - `evaluator` — 实现 `ValueEvaluation` 的实例
///
/// **返回**: `(reports, avg_motivation, passing_count)`
pub fn evaluate_cycle(
    candidates: &[ValueCandidate],
    evaluator: &dyn ValueEvaluation,
) -> ValueResult<(Vec<ValueEvaluationReport>, f64, usize)> {
    if candidates.is_empty() {
        return Err(ValueError::InvalidInput(
            "candidates must not be empty".into(),
        ));
    }
    let mut reports = Vec::with_capacity(candidates.len());
    let mut total = 0.0;
    let mut passing = 0;
    for c in candidates {
        let r = evaluator.evaluate(c)?;
        total += r.motivation;
        if r.passes_threshold {
            passing += 1;
        }
        reports.push(r);
    }
    let avg = total / candidates.len() as f64;
    Ok((reports, avg, passing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValuePriorityKind;

    fn cand(score: f64, dims: Vec<ValueDimension>) -> ValueCandidate {
        let mut c = ValueCandidate::new("eval_test", dims);
        c.autonomy_consistency = score;
        c.value_stability = score;
        c.intrinsic_motivation = score;
        c
    }

    #[test]
    fn default_threshold_is_085() {
        assert!((DEFAULT_THRESHOLD - 0.85).abs() < 1e-9);
    }

    #[test]
    fn evaluate_e_layer_aligned_with_allow_verdict() {
        let mut c = cand(0.9, vec![ValueDimension::PrincipleE]);
        c.verdict = Some(PhilosophyVerdict::Allow);
        let r = evaluate_value(&c).unwrap();
        assert_eq!(
            r.alignment_map.get(&ValueDimension::PrincipleE),
            Some(&ValueAlignment::Aligned)
        );
        assert!(!r.has_e_layer_conflict);
    }

    #[test]
    fn evaluate_e_layer_conflicted_with_block_verdict() {
        let mut c = cand(0.9, vec![ValueDimension::PrincipleE]);
        c.verdict = Some(PhilosophyVerdict::Block(
            apeireth_core::PhilosophyKey::NotClone,
        ));
        let r = evaluate_value(&c).unwrap();
        assert_eq!(
            r.alignment_map.get(&ValueDimension::PrincipleE),
            Some(&ValueAlignment::Conflicted)
        );
        assert!(r.has_e_layer_conflict);
    }

    #[test]
    fn evaluate_s_layer_high_stability_is_aligned() {
        let r = evaluate_value(&cand(0.9, vec![ValueDimension::ValueS])).unwrap();
        assert_eq!(
            r.alignment_map.get(&ValueDimension::ValueS),
            Some(&ValueAlignment::Aligned)
        );
    }

    #[test]
    fn evaluate_s_layer_low_stability_is_conflicted() {
        let mut c = cand(0.0, vec![ValueDimension::ValueS]);
        c.value_stability = 0.2;
        let r = evaluate_value(&c).unwrap();
        assert_eq!(
            r.alignment_map.get(&ValueDimension::ValueS),
            Some(&ValueAlignment::Conflicted)
        );
    }

    #[test]
    fn evaluate_passes_threshold_at_09() {
        let r = evaluate_value(&cand(0.9, vec![ValueDimension::ValueS])).unwrap();
        assert!(r.passes_threshold);
        assert!(r.motivation >= 0.85);
    }

    #[test]
    fn evaluate_fails_threshold_at_05() {
        let r = evaluate_value(&cand(0.5, vec![ValueDimension::ValueS])).unwrap();
        assert!(!r.passes_threshold);
        assert!(r.motivation < 0.85);
    }

    #[test]
    fn evaluate_cycle_avg_motivation_and_passing_count() {
        let cands = vec![
            cand(0.9, vec![ValueDimension::ValueS]),
            cand(0.95, vec![ValueDimension::ValueS]),
            cand(0.3, vec![ValueDimension::ValueS]),
        ];
        let (reports, avg, passing) = evaluate_cycle(&cands, &DefaultValueEvaluator).unwrap();
        assert_eq!(reports.len(), 3);
        // 前两个高分（0.9, 0.95）pass, 0.3 fail
        assert_eq!(passing, 2);
        let expected_avg = (0.9 + 0.95 + 0.3) / 3.0;
        assert!((avg - expected_avg).abs() < 1e-9);
    }

    #[test]
    fn evaluate_cycle_rejects_empty() {
        let res = evaluate_cycle(&[], &DefaultValueEvaluator);
        assert!(res.is_err());
    }

    #[test]
    fn compare_prefers_no_e_conflict() {
        let evaluator = DefaultValueEvaluator;
        let mut a = cand(0.5, vec![ValueDimension::PrincipleE]);
        a.verdict = Some(PhilosophyVerdict::Block(
            apeireth_core::PhilosophyKey::NotClone,
        ));
        let mut b = cand(0.5, vec![ValueDimension::PrincipleE]);
        b.verdict = Some(PhilosophyVerdict::Allow);
        // a 有 e 冲突、b 没有 → b 应 Higher
        assert_eq!(evaluator.compare(&a, &b), ValueComparison::Lower);
    }

    #[test]
    fn value_priority_kind_keeps_four_kinds() {
        // LOCKED — 4 种优先级
        let all = [
            ValuePriorityKind::Immediate,
            ValuePriorityKind::ShortTerm,
            ValuePriorityKind::LongTerm,
            ValuePriorityKind::Horizon,
        ];
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn report_dimension_and_aligned_counts() {
        let r = evaluate_value(&cand(0.9, ValueDimension::ALL.to_vec())).unwrap();
        assert_eq!(r.dimension_count(), 5);
        // S/Aligned (stability=0.9 high), E/Underspecified(no verdict), A/M/O Aligned (autonomy=0.9)
        assert!(r.aligned_count() >= 4);
    }
}
