//! Relational practice-memory: EMA verify + promotion threshold.
//!
//! Recovered from `legacy/donor/apeireth-companion/src/experience.rs` (algorithm
//! only). Distinct from v2 `apeireth-plugin::experience` wiki/KG/association
//! extraction: this is **reusable practice** (scene / practice / result /
//! verify_count / EMA score), not episode-derived wiki facts.
//!
//! - No EpisodeStore writes (`exp-*` prefix persistence discarded).
//! - No tool-registry `save_experience` / `verify_experience` tools.
//! - No chrono / uuid: callers inject ids and timestamps.
//!
//! Default-off library primitive.

/// Promotion: `verify_count >= 3` and `score >= 0.7` (donor initial params).
pub const PROMOTE_MIN_VERIFIES: u64 = 3;
pub const PROMOTE_MIN_SCORE: f64 = 0.7;
/// EMA smoothing (aligned with donor capability EMA).
pub const EMA_ALPHA: f64 = 0.7;

/// Practice-memory record (append-only versions share `chain`).
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeExperience {
    pub id: String,
    /// Logical chain id (same experience across revisions).
    pub chain: String,
    /// Monotonic revision (dedup takes max rev).
    pub rev: u64,
    pub scene: String,
    pub practice: String,
    pub result: String,
    /// `success` / `failure` / `partial`.
    pub outcome: String,
    pub verify_count: u64,
    /// EMA score in `[0, 1]`.
    pub score: f64,
    pub ready: bool,
    pub proposed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl PracticeExperience {
    pub fn new(
        id: impl Into<String>,
        scene: impl Into<String>,
        practice: impl Into<String>,
        result: impl Into<String>,
        outcome: impl Into<String>,
        at: i64,
    ) -> Self {
        let id = id.into();
        Self {
            chain: id.clone(),
            id,
            rev: 1,
            scene: scene.into(),
            practice: practice.into(),
            result: result.into(),
            outcome: outcome.into(),
            verify_count: 0,
            score: 0.5,
            ready: false,
            proposed: false,
            created_at: at,
            updated_at: at,
        }
    }
}

/// Outcome value used in the EMA update (donor `verify` table).
fn outcome_value(success: bool, previous_outcome: &str) -> f64 {
    match (success, previous_outcome) {
        (true, _) => 1.0,
        (false, "success") => 0.0,
        (false, "partial") => 0.3,
        (false, _) => 0.0,
    }
}

/// Apply one verification: count++, EMA, ready flag, rev++.
///
/// `new_id` is the append-only version id (chain is preserved). Clock is
/// injected so tests stay deterministic.
pub fn verify_experience(
    exp: &PracticeExperience,
    success: bool,
    new_id: impl Into<String>,
    at: i64,
) -> PracticeExperience {
    let mut next = exp.clone();
    let value = outcome_value(success, &next.outcome);
    next.verify_count += 1;
    next.score = next.score * EMA_ALPHA + value * (1.0 - EMA_ALPHA);
    next.ready = next.verify_count >= PROMOTE_MIN_VERIFIES && next.score >= PROMOTE_MIN_SCORE;
    next.updated_at = at;
    next.rev += 1;
    next.id = new_id.into();
    next
}

/// Mark as already proposed (stops promotion nag). Chain preserved, rev++.
pub fn mark_proposed(exp: &PracticeExperience, new_id: impl Into<String>, at: i64) -> PracticeExperience {
    let mut next = exp.clone();
    next.proposed = true;
    next.updated_at = at;
    next.rev += 1;
    next.id = new_id.into();
    next
}

/// Dedup by chain (keep max rev; later equal-rev wins), then sort by
/// `updated_at` descending. Optional substring scene filter.
pub fn list_latest<'a>(
    records: impl IntoIterator<Item = &'a PracticeExperience>,
    scene: Option<&str>,
) -> Vec<PracticeExperience> {
    let mut by_chain: std::collections::HashMap<String, PracticeExperience> =
        std::collections::HashMap::new();
    for e in records.into_iter().filter(|x| {
        scene.is_none_or(|s| x.scene.contains(s))
    }) {
        match by_chain.get(&e.chain) {
            Some(existing) if existing.rev > e.rev => {}
            _ => {
                by_chain.insert(e.chain.clone(), e.clone());
            }
        }
    }
    let mut out: Vec<PracticeExperience> = by_chain.into_values().collect();
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

/// Ready and not yet proposed.
pub fn ready_for_capability(records: &[PracticeExperience]) -> Vec<&PracticeExperience> {
    records
        .iter()
        .filter(|e| e.ready && !e.proposed)
        .collect()
}

/// Prompt hint for promotion (empty if none ready). Caps at 5 lines.
pub fn build_promotion_hint(ready: &[&PracticeExperience]) -> String {
    if ready.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "【经验晋级提示】以下经验已验证达标, 考虑用 propose_capability 提案为能力:\n",
    );
    for e in ready.iter().take(5) {
        s.push_str(&format!(
            "  • {} (验证 {} 次, 评分 {:.2}) — 做法: {}\n",
            e.scene, e.verify_count, e.score, e.practice
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> PracticeExperience {
        PracticeExperience::new(
            id,
            "主人学习高数换元法",
            "精练8题+套路卡, 每题自检 dx",
            "错题率下降",
            "success",
            1,
        )
    }

    #[test]
    fn three_successes_promote() {
        let mut last = sample("exp-v");
        last = verify_experience(&last, true, "exp-v-2", 2);
        assert!(!last.ready, "1 次验证未达标");
        last = verify_experience(&last, true, "exp-v-3", 3);
        assert!(!last.ready, "2 次验证未达标");
        last = verify_experience(&last, true, "exp-v-4", 4);
        assert!(last.ready, "3 次成功 + score>=0.7 → ready");
        assert_eq!(last.verify_count, 3);
        assert_eq!(last.chain, "exp-v");
        assert_eq!(last.rev, 4);
        let ready = ready_for_capability(std::slice::from_ref(&last));
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn failure_never_promotes() {
        let mut exp = sample("exp-f");
        exp.outcome = "failure".into();
        for i in 0..5 {
            exp = verify_experience(&exp, false, format!("exp-f-{i}"), i + 2);
        }
        assert!(ready_for_capability(std::slice::from_ref(&exp)).is_empty());
        assert!(exp.score < 0.5);
    }

    #[test]
    fn list_latest_dedups_by_chain_max_rev() {
        let a = sample("exp-1");
        let b = verify_experience(&a, true, "exp-1b", 10);
        let mut c = sample("exp-2");
        c.scene = "线代特征值".into();
        let listed = list_latest([&a, &b, &c], None);
        assert_eq!(listed.len(), 2);
        let one = listed.iter().find(|e| e.chain == "exp-1").unwrap();
        assert_eq!(one.id, "exp-1b");
        assert_eq!(one.rev, 2);
        assert_eq!(list_latest([&a, &b, &c], Some("高数")).len(), 1);
        assert_eq!(list_latest([&a, &b, &c], Some("线代")).len(), 1);
        assert_eq!(list_latest([&a, &b, &c], Some("物理")).len(), 0);
    }

    #[test]
    fn mark_proposed_stops_nag() {
        let mut last = sample("exp-v");
        for i in 0..3 {
            last = verify_experience(&last, true, format!("exp-v-{i}"), i + 2);
        }
        assert_eq!(ready_for_capability(std::slice::from_ref(&last)).len(), 1);
        last = mark_proposed(&last, "exp-v-p", 99);
        assert!(ready_for_capability(std::slice::from_ref(&last)).is_empty());
        let hint = build_promotion_hint(&[]);
        assert!(hint.is_empty());
        last.proposed = false;
        let refs = ready_for_capability(std::slice::from_ref(&last));
        let hint = build_promotion_hint(&refs);
        assert!(hint.contains("高数"));
        assert!(hint.contains("验证 3 次"));
    }

    #[test]
    fn ema_partial_failure_uses_0_3() {
        let mut exp = sample("exp-p");
        exp.outcome = "partial".into();
        exp.score = 1.0;
        exp = verify_experience(&exp, false, "exp-p-2", 2);
        let expected = 1.0 * EMA_ALPHA + 0.3 * (1.0 - EMA_ALPHA);
        assert!((exp.score - expected).abs() < 1e-12);
    }
}
