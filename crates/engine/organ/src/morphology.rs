//! Query morphology softmax — retrieval-depth classifier (N7 / rivermemo_topology_v3).
//!
//! Canonical implementation module.
//!
//! **Algorithm**: extract deterministic text features (length / entity density /
//! question morphology / clause count / depth cues) → three logits
//! (Shallow / Standard / Deep) → temperature-scaled softmax → argmax mode +
//! expected CRAWL budget in `[1, 6]`.
//!
//! **Honesty**: TopologicalEngine originally used river-network hop / HHI / forward-flow
//! topology. Apeireth has no river graph, so features are hand-tuned text
//! heuristics (not learned, not calibrated). Same mechanism (logits + softmax
//! + bins); different features.
//!
//! Pure function: same query + temperature → same verdict. 0 IO / 0 LLM /
//! 0 RNG. Default-off library primitive — no production wiring.

/// Retrieval-depth bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    /// Short / direct query (CRAWL base budget 1).
    Shallow,
    /// Multi-entity / relational query (CRAWL base budget 3).
    Standard,
    /// Long query + depth cues (CRAWL base budget 6).
    Deep,
}

impl RetrievalMode {
    /// Bin → CRAWL base budget (BFS expansion cap).
    pub fn base_budget(self) -> usize {
        match self {
            RetrievalMode::Shallow => 1,
            RetrievalMode::Standard => 3,
            RetrievalMode::Deep => 6,
        }
    }
}

/// Verdict: argmax bin + softmax distribution `[shallow, standard, deep]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphologyVerdict {
    pub mode: RetrievalMode,
    pub weights: [f64; 3],
}

impl MorphologyVerdict {
    /// Expected budget from the softmax mix, clamped to `[1, 6]`.
    pub fn budget(&self) -> usize {
        let v = self.weights[0] * 1.0 + self.weights[1] * 3.0 + self.weights[2] * 6.0;
        (v.round() as usize).clamp(1, 6)
    }
}

/// Narrative / recall / synthesis cues → raise Deep logit.
const DEPTH_CUES: &[&str] = &[
    "详细",
    "深入",
    "全面",
    "背景",
    "历史",
    "过程",
    "来龙去脉",
    "前因后果",
    "为什么",
    "原因",
    "梳理",
    "总结",
    "回顾",
    "整个",
];

/// Direct-question cues → raise Shallow logit.
const QUESTION_MARKS: &[&str] = &[
    "？",
    "?",
    "吗",
    "呢",
    "怎么",
    "如何",
    "是否",
    "哪些",
    "什么",
    "为什么",
];

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn cue_hits(q: &str, cues: &[&str]) -> f64 {
    cues.iter().map(|c| q.matches(c).count()).sum::<usize>() as f64
}

struct Features {
    length: f64,
    entity: f64,
    question: f64,
    clauses: f64,
    depth: f64,
}

fn extract(q: &str) -> Features {
    let total = q.chars().count();
    let length = clamp01(total as f64 / 60.0);
    let entity = if total == 0 {
        0.0
    } else {
        let dense = q.chars().filter(|c| c.is_alphanumeric()).count();
        dense as f64 / total as f64
    };
    let question = clamp01(cue_hits(q, QUESTION_MARKS) / 2.0);
    let depth = clamp01(cue_hits(q, DEPTH_CUES) / 2.0);
    let segs = q
        .split(|c: char| {
            matches!(
                c,
                '，' | ',' | '。' | '！' | '!' | '？' | '?' | '；' | ';' | '、' | '：' | ':' | '\n'
            )
        })
        .filter(|s| !s.trim().is_empty())
        .count();
    let clauses = clamp01(segs.saturating_sub(1) as f64 / 3.0);
    Features {
        length,
        entity,
        question,
        clauses,
        depth,
    }
}

/// Three logits `[shallow, standard, deep]` (hand-tuned, TopologicalEngine-shaped weights).
fn logits(f: &Features) -> [f64; 3] {
    [
        1.45 * (1.0 - f.length) + 0.9 * f.question - 1.25 * f.depth - 0.65 * f.clauses,
        0.35 + 1.25 * f.clauses + 0.7 * f.entity + 0.35 * f.question - 0.45 * f.depth,
        1.4 * f.length + 1.15 * f.depth + 0.8 * f.clauses + 0.3 * f.entity - 0.65 * f.question,
    ]
}

/// NaN / ≤0 / ∞ → 1.0; finite values clamped to `[0.1, 10.0]` (prevent exp collapse).
pub fn sanitize_temperature(t: f64) -> f64 {
    if !t.is_finite() || t <= 0.0 {
        1.0
    } else {
        t.clamp(0.1, 10.0)
    }
}

/// Query → bin + softmax. Pure: same input → same output.
pub fn classify(query: &str, temperature: f64) -> MorphologyVerdict {
    let t = sanitize_temperature(temperature);
    let l = logits(&extract(query));
    let m = l.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exp = l.map(|v| ((v - m) / t).exp());
    let sum: f64 = exp.iter().sum();
    let weights = exp.map(|v| v / sum.max(1e-12));
    let (idx, _) = weights
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    let mode = match idx {
        0 => RetrievalMode::Shallow,
        1 => RetrievalMode::Standard,
        _ => RetrievalMode::Deep,
    };
    MorphologyVerdict { mode, weights }
}

/// Optional env override (`APEIRETH_MORPHOLOGY_TEMPERATURE`). Illegal → 1.0.
/// Not used by production; callers should pass temperature explicitly.
pub fn env_temperature() -> f64 {
    std::env::var("APEIRETH_MORPHOLOGY_TEMPERATURE")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(sanitize_temperature)
        .unwrap_or(1.0)
}

/// Query → CRAWL budget. Temperature is an explicit argument (no hidden env).
pub fn crawl_budget(query: &str, temperature: f64) -> usize {
    classify(query, temperature).budget()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_query_same_mode() {
        for q in [
            "在吗",
            "",
            "帮我详细梳理项目背景和历史过程",
            "项目进度，测试情况，部署安排，分别怎么样了？",
        ] {
            let a = classify(q, 1.0);
            for _ in 0..5 {
                assert_eq!(classify(q, 1.0), a, "同查询同档位: {q}");
            }
        }
    }

    #[test]
    fn short_question_shallow() {
        let v = classify("在吗", 1.0);
        assert_eq!(v.mode, RetrievalMode::Shallow);
        assert!(v.budget() <= 2, "浅扫预算应接近 1: {}", v.budget());
    }

    #[test]
    fn multi_clause_relational_standard() {
        let v = classify("项目进度，测试情况，还有部署安排，分别怎么样了？", 1.0);
        assert_eq!(v.mode, RetrievalMode::Standard);
        assert_eq!(v.budget(), 3);
    }

    #[test]
    fn long_depth_query_deep() {
        let v = classify(
            "帮我详细梳理一下我们之前讨论的项目背景和历史过程，要全面的来龙去脉和原因",
            1.0,
        );
        assert_eq!(v.mode, RetrievalMode::Deep);
        assert!(v.budget() >= 4, "深爬预算应显著高于标准: {}", v.budget());
    }

    #[test]
    fn empty_query_shallow() {
        let v = classify("", 1.0);
        assert_eq!(v.mode, RetrievalMode::Shallow);
        assert!(v.budget() <= 2, "空查询走最浅: {}", v.budget());
    }

    #[test]
    fn huge_query_deep_no_panic() {
        let q = "背景".repeat(5000);
        let v = classify(&q, 1.0);
        assert_eq!(v.mode, RetrievalMode::Deep);
    }

    #[test]
    fn weights_are_valid_distribution() {
        for q in [
            "你好",
            "帮我详细回顾整个历史",
            "进度，风险，资源，时间，分别什么情况？",
        ] {
            let v = classify(q, 1.0);
            let sum: f64 = v.weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "softmax 应归一: {sum}");
            assert!(v.weights.iter().all(|w| (0.0..=1.0).contains(w)));
        }
    }

    #[test]
    fn temperature_affects_sharpness() {
        let q = "帮我详细梳理一下我们之前讨论的项目背景和历史过程，要全面的来龙去脉和原因";
        let cold = classify(q, 0.1);
        let hot = classify(q, 10.0);
        assert_eq!(cold.budget(), RetrievalMode::Deep.base_budget());
        assert!(hot.budget() < cold.budget(), "高温摊平应降低期望预算");
    }

    #[test]
    fn invalid_temperature_falls_back() {
        let base = classify("随便聊聊天", 1.0);
        for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                classify("随便聊聊天", bad),
                base,
                "非法温度应回落 1.0: {bad}"
            );
        }
    }

    #[test]
    fn budget_bounds() {
        let v = |w: [f64; 3]| MorphologyVerdict {
            mode: RetrievalMode::Shallow,
            weights: w,
        };
        assert_eq!(v([1.0, 0.0, 0.0]).budget(), 1);
        assert_eq!(v([0.0, 1.0, 0.0]).budget(), 3);
        assert_eq!(v([0.0, 0.0, 1.0]).budget(), 6);
        assert_eq!(v([1.0 / 3.0; 3]).budget(), 3);
    }

    #[test]
    fn crawl_budget_matches_classify() {
        let q = "项目进度，测试情况，还有部署安排，分别怎么样了？";
        assert_eq!(crawl_budget(q, 1.0), classify(q, 1.0).budget());
    }
}
