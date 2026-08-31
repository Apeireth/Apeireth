//! Score aggregation recovered from `legacy/donor/apeireth-eval`.
//!
//! Pure stdlib statistics: arithmetic / weighted mean, sample standard
//! deviation, linear-interpolation percentile, and a SWE-bench-style pass-rate
//! summarizer. This is **governance-of-eval** math, not a task runner and not
//! a second agent loop. LLM smoke / MCP bridge / live cross-model HTTP paths
//! from the donor are discarded.

use serde::{Deserialize, Serialize};

/// One named score in `[0, 1]`. Non-finite or out-of-range values are invalid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalScore {
    pub dimension: String,
    pub value: f64,
}

impl EvalScore {
    pub fn new(dimension: impl Into<String>, value: f64) -> Self {
        Self {
            dimension: dimension.into(),
            value,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.value.is_finite() && (0.0..=1.0).contains(&self.value)
    }
}

/// Arithmetic mean of valid scores. `None` when every input is invalid.
pub fn mean(scores: &[EvalScore]) -> Option<f64> {
    let valid: Vec<f64> = scores
        .iter()
        .filter(|score| score.is_valid())
        .map(|score| score.value)
        .collect();
    if valid.is_empty() {
        return None;
    }
    Some(valid.iter().sum::<f64>() / valid.len() as f64)
}

/// Weighted mean. Length mismatch, invalid score, negative / non-finite
/// weight, or zero weight-sum → `None`.
pub fn weighted_mean(scores: &[EvalScore], weights: &[f64]) -> Option<f64> {
    if scores.len() != weights.len() {
        return None;
    }
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (score, weight) in scores.iter().zip(weights) {
        if !score.is_valid() || !weight.is_finite() || *weight < 0.0 {
            return None;
        }
        total += score.value * weight;
        weight_sum += weight;
    }
    if weight_sum == 0.0 {
        return None;
    }
    Some(total / weight_sum)
}

/// Sample standard deviation (Bessel correction). Fewer than two valid
/// scores → `0.0`.
pub fn stddev(scores: &[EvalScore]) -> f64 {
    let valid: Vec<f64> = scores
        .iter()
        .filter(|score| score.is_valid())
        .map(|score| score.value)
        .collect();
    if valid.len() < 2 {
        return 0.0;
    }
    let mean = valid.iter().sum::<f64>() / valid.len() as f64;
    let variance = valid
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (valid.len() - 1) as f64;
    variance.sqrt()
}

/// Linear-interpolation percentile. `p` must be in `[0, 1]`.
pub fn percentile(scores: &[EvalScore], p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) {
        return None;
    }
    let mut valid: Vec<f64> = scores
        .iter()
        .filter(|score| score.is_valid())
        .map(|score| score.value)
        .collect();
    if valid.is_empty() {
        return None;
    }
    valid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if valid.len() == 1 {
        return Some(valid[0]);
    }
    let rank = p * (valid.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        return Some(valid[lo]);
    }
    let frac = rank - lo as f64;
    Some(valid[lo] * (1.0 - frac) + valid[hi] * frac)
}

pub fn is_valid_percentile(p: f64) -> bool {
    p.is_finite() && (0.0..=1.0).contains(&p)
}

/// One SWE-bench-style result. The executor itself is not ported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub category: String,
    pub passed: bool,
    pub score: f64,
}

/// Per-category pass-rate row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryBreakdown {
    pub category: String,
    pub total: usize,
    pub passed: usize,
    pub pass_rate: f64,
}

/// Aggregated pass-rate report. Empty input uses pass_rate / mean_score = 1.0
/// (donor convention: "no tasks" is not a failure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub mean_score: f64,
    pub per_category: Vec<CategoryBreakdown>,
}

impl TaskSummary {
    pub fn summarize(results: &[TaskResult]) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|result| result.passed).count();
        let failed = total - passed;
        let pass_rate = if total == 0 {
            1.0
        } else {
            passed as f64 / total as f64
        };
        let mean_score = if total == 0 {
            1.0
        } else {
            results.iter().map(|result| result.score).sum::<f64>() / total as f64
        };

        let mut categories: Vec<&str> = results
            .iter()
            .map(|result| result.category.as_str())
            .collect();
        categories.sort_unstable();
        categories.dedup();
        let per_category = categories
            .into_iter()
            .map(|category| {
                let cat_results: Vec<&TaskResult> = results
                    .iter()
                    .filter(|result| result.category == category)
                    .collect();
                let cat_total = cat_results.len();
                let cat_passed = cat_results.iter().filter(|result| result.passed).count();
                let cat_rate = if cat_total == 0 {
                    1.0
                } else {
                    cat_passed as f64 / cat_total as f64
                };
                CategoryBreakdown {
                    category: category.into(),
                    total: cat_total,
                    passed: cat_passed,
                    pass_rate: cat_rate,
                }
            })
            .collect();

        Self {
            total,
            passed,
            failed,
            pass_rate,
            mean_score,
            per_category,
        }
    }

    pub fn to_eval_scores(&self) -> Vec<EvalScore> {
        let mut scores = vec![EvalScore::new("overall_pass_rate", self.pass_rate)];
        for breakdown in &self.per_category {
            scores.push(EvalScore::new(
                format!("category:{}_pass_rate", breakdown.category),
                breakdown.pass_rate,
            ));
        }
        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scores() -> Vec<EvalScore> {
        vec![
            EvalScore::new("a", 0.5),
            EvalScore::new("b", 0.8),
            EvalScore::new("c", 1.0),
            EvalScore::new("d", 0.0),
            EvalScore::new("e", 1.5),
            EvalScore::new("f", f64::NAN),
        ]
    }

    #[test]
    fn valid_score_passes() {
        assert!(EvalScore::new("q", 0.85).is_valid());
        assert!(!EvalScore::new("q", 1.5).is_valid());
        assert!(!EvalScore::new("q", f64::NAN).is_valid());
    }

    #[test]
    fn mean_skips_invalid() {
        let value = mean(&scores()).unwrap();
        assert!((value - 0.575).abs() < 1e-9);
        assert!(mean(&[EvalScore::new("x", 2.0)]).is_none());
    }

    #[test]
    fn weighted_mean_simple() {
        let scores = vec![EvalScore::new("a", 0.5), EvalScore::new("b", 1.0)];
        assert_eq!(weighted_mean(&scores, &[1.0, 1.0]), Some(0.75));
        assert_eq!(weighted_mean(&scores, &[0.0, 0.0]), None);
        assert!(weighted_mean(&scores, &[1.0]).is_none());
    }

    #[test]
    fn stddev_basic() {
        let scores = vec![EvalScore::new("a", 0.1), EvalScore::new("b", 0.3)];
        assert!((stddev(&scores) - 0.02f64.sqrt()).abs() < 1e-9);
        assert_eq!(stddev(&[]), 0.0);
        assert_eq!(stddev(&[EvalScore::new("a", 0.5)]), 0.0);
    }

    #[test]
    fn percentile_median_and_p95() {
        let scores = vec![
            EvalScore::new("a", 0.1),
            EvalScore::new("b", 0.5),
            EvalScore::new("c", 0.9),
        ];
        assert!((percentile(&scores, 0.5).unwrap() - 0.5).abs() < 1e-9);
        let vs: Vec<EvalScore> = (1..=20)
            .map(|i| EvalScore::new("x", f64::from(i) / 20.0))
            .collect();
        let p95 = percentile(&vs, 0.95).unwrap();
        assert!((0.90..=1.0).contains(&p95), "p95 = {p95}");
        assert!(percentile(&scores, 1.5).is_none());
        assert!(percentile(&[], 0.5).is_none());
    }

    #[test]
    fn is_valid_percentile_basic() {
        assert!(is_valid_percentile(0.0));
        assert!(is_valid_percentile(1.0));
        assert!(!is_valid_percentile(1.5));
        assert!(!is_valid_percentile(f64::NAN));
    }

    #[test]
    fn task_summary_mixed_categories() {
        let results = vec![
            TaskResult {
                task_id: "t1".into(),
                category: "bug-fix".into(),
                passed: true,
                score: 1.0,
            },
            TaskResult {
                task_id: "t2".into(),
                category: "bug-fix".into(),
                passed: false,
                score: 0.0,
            },
            TaskResult {
                task_id: "t3".into(),
                category: "feat".into(),
                passed: true,
                score: 1.0,
            },
        ];
        let summary = TaskSummary::summarize(&results);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert!((summary.pass_rate - 2.0 / 3.0).abs() < 1e-9);
        let bug = summary
            .per_category
            .iter()
            .find(|row| row.category == "bug-fix")
            .unwrap();
        assert_eq!(bug.total, 2);
        assert_eq!(bug.passed, 1);
        let scores = summary.to_eval_scores();
        assert!(scores
            .iter()
            .any(|score| score.dimension == "overall_pass_rate"));
        for score in &scores {
            assert!(score.is_valid());
        }
    }

    #[test]
    fn empty_tasks_pass_rate_is_one() {
        let summary = TaskSummary::summarize(&[]);
        assert_eq!(summary.total, 0);
        assert!((summary.pass_rate - 1.0).abs() < 1e-9);
        assert!((summary.mean_score - 1.0).abs() < 1e-9);
    }
}
