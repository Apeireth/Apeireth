//! 价值优先级排序模块 — ValuePrioritization trait.
//!
//! 公开 API:
//! - `prioritize_values` — 多候选按 (E-pass, motivation_score, priority_kind.weight, stability) 排序
//! - `ValueRank` — 排序后的单个候选条目 (含原 candidate ID + 综合分 + 排名)

use uuid::Uuid;

use crate::{
    evaluation::{evaluate_value, ValueEvaluationReport, DEFAULT_THRESHOLD},
    ValueCandidate, ValueComparison, ValueError, ValuePriorityKind, ValueResult,
};

use serde::{Deserialize, Serialize};

/// ValuePrioritization trait — 价值器官的排序契约。
///
/// 排序规则 (从主到次)：
/// 1. **E 层**: 有 E 层冲突的候选直接被压到队尾 (硬门槛)
/// 2. **硬门槛**: 通过 motivation_score ≥ threshold 的优先于未通过的
/// 3. **动机分**: motivation_score 高 → 排名靠前
/// 4. **优先级类别**: weight 高 → 排名靠前 (即时 > 短期 > 长期 > 地平线)
/// 5. **稳定性**: value_stability 高 → 排名靠前 (并列时)
pub trait ValuePrioritization {
    /// 排序一组候选 — 返回按优先级从高到低排列的 rank 列表。
    fn prioritize(&self, candidates: &[ValueCandidate]) -> ValueResult<Vec<ValueRank>>;

    /// 两候选的直接比较 (用于单元测试和增量排序).
    fn compare(&self, a: &ValueCandidate, b: &ValueCandidate) -> ValueComparison;
}

/// 排序后的单个条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueRank {
    /// 排名 (1-based, 1 = 最高优先级).
    pub rank: usize,
    /// 关联候选 ID.
    pub candidate_id: Uuid,
    /// 综合分 (motivation_score, 用于 UI 展示).
    pub score: f64,
    /// 关联评估报告 (来自 ValueEvaluation)。
    pub report: ValueEvaluationReport,
}

/// 默认排序器 — 实现上述 5 级排序规则。
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultPrioritizer;

impl ValuePrioritization for DefaultPrioritizer {
    fn prioritize(&self, candidates: &[ValueCandidate]) -> ValueResult<Vec<ValueRank>> {
        if candidates.is_empty() {
            return Err(ValueError::InvalidInput(
                "candidates must not be empty".into(),
            ));
        }

        // Step 1: 评估所有候选，得到 E-conflict 与 motivation.
        let mut evaluated: Vec<(
            ValueCandidate,
            ValueEvaluationReport,
            ValuePriorityKind,
            f64,
        )> = Vec::with_capacity(candidates.len());
        for c in candidates {
            let r = evaluate_value(c)?;
            evaluated.push((c.clone(), r.clone(), c.priority_kind, c.value_stability));
        }

        // Step 2: 5 级排序键 — 见 trait 注释.
        evaluated.sort_by(|x, y| {
            // Rule 1: E-conflict 压底
            let a_e = x.1.has_e_layer_conflict;
            let b_e = y.1.has_e_layer_conflict;
            if a_e != b_e {
                return a_e.cmp(&b_e); // false (no conflict) < true → not conflicted 先
            }
            // Rule 2: 硬门槛
            let a_pass = x.1.passes_threshold;
            let b_pass = y.1.passes_threshold;
            if a_pass != b_pass {
                return b_pass.cmp(&a_pass); // passing 先
            }
            // Rule 3: motivation_score 高→前
            let a_m = x.1.motivation;
            let b_m = y.1.motivation;
            if (a_m - b_m).abs() > 1e-9 {
                return b_m.partial_cmp(&a_m).unwrap_or(std::cmp::Ordering::Equal);
            }
            // Rule 4: priority_kind.weight 高→前
            let a_k = x.2.weight();
            let b_k = y.2.weight();
            if a_k != b_k {
                return b_k.cmp(&a_k);
            }
            // Rule 5: value_stability 高→前
            b_m.partial_cmp(&a_m).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Step 3: 排名 1-based.
        let ranks: Vec<ValueRank> = evaluated
            .into_iter()
            .enumerate()
            .map(|(i, (c, r, _, _))| ValueRank {
                rank: i + 1,
                candidate_id: c.id,
                score: r.motivation,
                report: r,
            })
            .collect();

        Ok(ranks)
    }

    fn compare(&self, a: &ValueCandidate, b: &ValueCandidate) -> ValueComparison {
        let ranks_a = self.prioritize(&[a.clone()]).ok();
        let ranks_b = self.prioritize(&[b.clone()]).ok();
        match (ranks_a, ranks_b) {
            (Some(_), Some(_)) => {
                // 用评估 + 综合分做单元素比较 — 复用 DefaultValueEvaluator 逻辑
                let ra = evaluate_value(a).ok();
                let rb = evaluate_value(b).ok();
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
                    _ => ValueComparison::Incomparable,
                }
            }
            _ => ValueComparison::Incomparable,
        }
    }
}

/// 公开便捷函数 — 用默认排序器排序一组候选。
pub fn prioritize_values(candidates: &[ValueCandidate]) -> ValueResult<Vec<ValueRank>> {
    DefaultPrioritizer.prioritize(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValueDimension;

    fn cand(score: f64, kind: ValuePriorityKind) -> ValueCandidate {
        let mut c = ValueCandidate::new(
            "prio_test",
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
        c.priority_kind = kind;
        c
    }

    #[test]
    fn prioritize_rejects_empty() {
        let res = prioritize_values(&[]);
        assert!(res.is_err());
    }

    #[test]
    fn prioritize_orders_high_score_first() {
        let cands = vec![
            cand(0.5, ValuePriorityKind::ShortTerm),
            cand(0.95, ValuePriorityKind::ShortTerm),
            cand(0.7, ValuePriorityKind::ShortTerm),
        ];
        let ranks = prioritize_values(&cands).unwrap();
        // 0.95 pass + 0.7 pass + 0.5 not pass (且 hard threshold)
        assert_eq!(ranks[0].rank, 1);
        assert!(ranks[0].score > ranks[1].score);
        assert!(ranks[1].score > ranks[2].score);
    }

    #[test]
    fn prioritize_pushes_e_conflict_to_bottom() {
        let mut a = cand(0.95, ValuePriorityKind::ShortTerm);
        a.verdict = Some(apeireth_core::PhilosophyVerdict::Block(
            apeireth_core::PhilosophyKey::NotClone,
        ));
        let b = cand(0.95, ValuePriorityKind::ShortTerm);
        let ranks = prioritize_values(&[a.clone(), b.clone()]).unwrap();
        // a 有 E 冲突 → 排到最后
        assert_eq!(ranks.last().unwrap().candidate_id, a.id);
        assert_eq!(ranks.first().unwrap().candidate_id, b.id);
    }

    #[test]
    fn prioritize_immediate_outranks_longterm_at_same_score() {
        let imm = cand(0.9, ValuePriorityKind::Immediate);
        let horizon = cand(0.9, ValuePriorityKind::Horizon);
        let ranks = prioritize_values(&[horizon.clone(), imm.clone()]).unwrap();
        assert_eq!(ranks[0].candidate_id, imm.id);
        assert_eq!(ranks[1].candidate_id, horizon.id);
    }

    #[test]
    fn prioritize_ranks_are_one_based_contiguous() {
        let cands = vec![
            cand(0.9, ValuePriorityKind::ShortTerm),
            cand(0.9, ValuePriorityKind::ShortTerm),
            cand(0.9, ValuePriorityKind::ShortTerm),
        ];
        let ranks = prioritize_values(&cands).unwrap();
        for (i, r) in ranks.iter().enumerate() {
            assert_eq!(r.rank, i + 1);
        }
    }

    #[test]
    fn prioritize_threshold_filters_passers() {
        // 0.9 > 0.85 threshold: pass, 0.5 < 0.85: fail.
        let cands = vec![
            cand(0.5, ValuePriorityKind::ShortTerm),
            cand(0.9, ValuePriorityKind::Immediate),
        ];
        let ranks = prioritize_values(&cands).unwrap();
        // 第一个 rank 应该是 passing 的 (0.9)
        assert!(ranks[0].report.passes_threshold);
        assert!(!ranks[1].report.passes_threshold);
    }

    #[test]
    fn default_threshold_constant_in_evaluation_module() {
        // 共享门槛: prioritization 用 evaluation 的 0.85
        assert!((DEFAULT_THRESHOLD - 0.85).abs() < 1e-9);
    }

    #[test]
    fn compare_incomparable_when_evaluate_fails() {
        // 故意构造不合法候选 (label 空) 让 evaluate_value 报错 → 返回 Incomparable
        let mut bad = cand(0.5, ValuePriorityKind::ShortTerm);
        bad.label = "".into();
        let good = cand(0.5, ValuePriorityKind::ShortTerm);
        let c = DefaultPrioritizer.compare(&bad, &good);
        assert_eq!(c, ValueComparison::Incomparable);
    }
}
