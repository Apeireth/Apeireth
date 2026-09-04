//! Generative-Agents-style memory ranking + Mem0 tombstone filter.
//!
//! Recovered from `legacy/donor/apeireth-companion/src/memory_extractor.rs`:
//! - `parse_importance` / `【imp:N】` prefix
//! - `rank_memory_entries`: importance × 3 + access × 0.3 + group bonus + recency
//! - `active_episodes` tombstone filter (`tomb-*` + `【已废弃】{id}`)
//!
//! Not a second extractor engine. LLM extract / `MemoryExtractionService::apply`
//! stays with organ `MemoryMergerOrgan`. Clock is injectable (`now_unix`) so
//! recency is deterministic. Default-off; not production-wired.

use std::collections::{HashMap, HashSet};

/// Importance prefix written by the donor extractor (`【imp:N】content`).
pub const IMP_PREFIX: &str = "【imp:";

/// Tombstone body prefix (`【已废弃】{target_id}`).
pub const TOMBSTONE_PREFIX: &str = "【已废弃】";

/// Rankable memory row. Callers map from episodes; this crate does not write them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankableMemory {
    pub id: String,
    pub content: String,
    /// Event time (unix seconds).
    pub timestamp: i64,
}

impl RankableMemory {
    pub fn new(id: impl Into<String>, content: impl Into<String>, timestamp: i64) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            timestamp,
        }
    }
}

/// Parse `【imp:N】` prefix; missing / malformed → 5 (donor default).
pub fn parse_importance(content: &str) -> u8 {
    if let Some(rest) = content.strip_prefix(IMP_PREFIX) {
        if let Some(end) = rest.find('】') {
            if let Ok(n) = rest[..end].parse::<u8>() {
                return n.clamp(1, 10);
            }
        }
    }
    5
}

/// Group bonus: dream/pref = 4, extracted/reflect = 2, else 0.
pub fn group_bonus(id: &str) -> f64 {
    if id.starts_with("mem-dream-") || id.starts_with("pref-") {
        4.0
    } else if id.starts_with("mem-ex-") || id.starts_with("reflect-") {
        2.0
    } else {
        0.0
    }
}

/// Linear recency in `[0, 1]` over the last 7 days. Older → 0.
pub fn recency_score(timestamp: i64, now_unix: i64) -> f64 {
    let age_days = (now_unix - timestamp) as f64 / 86400.0;
    if age_days < 7.0 {
        (7.0 - age_days) / 7.0
    } else {
        0.0
    }
}

/// Score = importance×3 + access_count×0.3 + group_bonus + recency×2.
pub fn memory_score(item: &RankableMemory, access_count: u64, now_unix: i64) -> f64 {
    let importance = f64::from(parse_importance(&item.content));
    importance * 3.0
        + access_count as f64 * 0.3
        + group_bonus(&item.id)
        + recency_score(item.timestamp, now_unix) * 2.0
}

/// Rank by score descending; ties keep input order. Returns at most `budget`
/// `(id, content)` pairs.
///
/// `access` maps id → `(access_count, last_access_unix)`; last-access is
/// recorded for callers but unused in the donor score (count only).
pub fn rank_memory_entries(
    items: &[RankableMemory],
    access: &HashMap<String, (u64, i64)>,
    budget: usize,
    now_unix: i64,
) -> Vec<(String, String)> {
    let mut ranked: Vec<(usize, &RankableMemory, f64)> = items
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (count, _) = access.get(&e.id).copied().unwrap_or((0, 0));
            (i, e, memory_score(e, count, now_unix))
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked
        .iter()
        .take(budget)
        .map(|(_, e, _)| (e.id.clone(), e.content.clone()))
        .collect()
}

/// Drop `tomb-*` rows and any id listed in a tombstone body.
pub fn filter_active_memories(items: &[RankableMemory], n: usize) -> Vec<RankableMemory> {
    let tombed: HashSet<String> = items
        .iter()
        .filter(|e| e.id.starts_with("tomb-"))
        .filter_map(|e| {
            e.content
                .strip_prefix(TOMBSTONE_PREFIX)
                .map(|s| s.trim().to_string())
        })
        .collect();
    items
        .iter()
        .filter(|e| !e.id.starts_with("tomb-"))
        .filter(|e| !tombed.contains(&e.id))
        .take(n)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_importance_prefix_and_default() {
        assert_eq!(parse_importance("【imp:9】主人周五考高数期中"), 9);
        assert_eq!(parse_importance("【imp:1】low"), 1);
        assert_eq!(parse_importance("【imp:10】high"), 10);
        assert_eq!(parse_importance("【imp:99】clamp"), 10);
        assert_eq!(parse_importance("主人偏好: x"), 5);
        assert_eq!(parse_importance("【imp:x】bad"), 5);
        assert_eq!(parse_importance(""), 5);
    }

    #[test]
    fn rank_prefers_importance_group_and_recency() {
        let now = 7 * 86400;
        let items = vec![
            RankableMemory::new("other-1", "【imp:9】plain high", 0),
            RankableMemory::new("pref-1", "【imp:5】preference", 0),
            RankableMemory::new("mem-ex-1", "【imp:5】extracted", now),
        ];
        let ranked = rank_memory_entries(&items, &HashMap::new(), 3, now);
        // pref: imp5*3 + bonus4 + recency0 = 19
        // mem-ex recent: imp5*3 + bonus2 + recency2 = 19; later in input after pref
        // other: imp9*3 + 0 + recency0 = 27 → first
        assert_eq!(ranked[0].0, "other-1");
        assert_eq!(ranked[1].0, "pref-1");
        assert_eq!(ranked[2].0, "mem-ex-1");
    }

    #[test]
    fn rank_access_count_breaks_ties_and_budget_truncates() {
        let now = 0;
        let items = vec![
            RankableMemory::new("a", "【imp:5】x", 0),
            RankableMemory::new("b", "【imp:5】y", 0),
        ];
        let mut access = HashMap::new();
        access.insert("b".into(), (10, 0));
        let ranked = rank_memory_entries(&items, &access, 1, now);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].0, "b");
    }

    #[test]
    fn filter_active_drops_tombs_and_targets() {
        let items = vec![
            RankableMemory::new("mem-ex-old", "【imp:5】旧事实", 1),
            RankableMemory::new("tomb-1", "【已废弃】mem-ex-old", 2),
            RankableMemory::new("mem-ex-new", "【imp:8】新事实", 3),
        ];
        let active = filter_active_memories(&items, 50);
        assert!(!active.iter().any(|e| e.id == "mem-ex-old"));
        assert!(!active.iter().any(|e| e.id.starts_with("tomb-")));
        assert!(active.iter().any(|e| e.content.contains("新事实")));
    }

    #[test]
    fn filter_active_respects_n() {
        let items: Vec<_> = (0..5)
            .map(|i| RankableMemory::new(format!("mem-ex-{i}"), "x", i))
            .collect();
        assert_eq!(filter_active_memories(&items, 2).len(), 2);
    }
}
