//! Incremental dream-candidate selection (donor `DreamScheduler::tick` filter).
//!
//! v2 already owns:
//! - [`crate::dreaming::DreamEngine`] — 6-stage cognitive dream SM
//! - [`crate::lightmemo::DreamSubsystem`] — pair-merge callback
//! - [`crate::lightmemo::SleepCycle`] — quiet-threshold trigger
//!
//! Donor companion `dream.rs` additionally **filters** what enters a cycle:
//! skip `mem-dream-*` (prevents summary-of-summary nesting) and skip items
//! older than `last_cycle_at` (incremental night). Pair-merge and write-back
//! stay with lightmemo / the caller. Default-off; not production-wired.

/// Prefix of dream-cycle products (and thought-inventory rows).
pub const DREAM_ID_PREFIX: &str = "mem-dream-";

/// One episode-shaped row for candidate selection. Callers map from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DreamSource {
    pub id: String,
    pub timestamp: i64,
    pub content: String,
}

impl DreamSource {
    pub fn new(id: impl Into<String>, timestamp: i64, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            timestamp,
            content: content.into(),
        }
    }
}

/// Select contents for a dream cycle.
///
/// - drops ids starting with [`DREAM_ID_PREFIX`]
/// - keeps `timestamp >= last_cycle_at` (unix seconds)
///
/// Does not pair-merge; [`crate::lightmemo::DreamSubsystem::dream_cycle`] does.
pub fn select_dream_candidates(sources: &[DreamSource], last_cycle_at: i64) -> Vec<String> {
    sources
        .iter()
        .filter(|e| !e.id.starts_with(DREAM_ID_PREFIX))
        .filter(|e| e.timestamp >= last_cycle_at)
        .map(|e| e.content.clone())
        .collect()
}

/// Pair-merge contents with the donor joiner `" ◆ "` (honest string concat).
/// Odd tail is left unpaired (same as `DreamSubsystem`).
pub fn pair_merge(items: &[String]) -> Vec<String> {
    items
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .map(|pair| format!("{} ◆ {}", pair[0], pair[1]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<DreamSource> {
        ["线代: 特征值最后一题卡住", "高数: 换元忘换 dx", "明天交线代作业", "council bug: advisor 低频误报"]
            .iter()
            .enumerate()
            .map(|(i, c)| DreamSource::new(format!("mem-{i}"), 1 + i as i64, *c))
            .collect()
    }

    #[test]
    fn first_night_pairs_four_items() {
        let items = select_dream_candidates(&seed(), 0);
        assert_eq!(items.len(), 4);
        let merged = pair_merge(&items);
        assert_eq!(merged.len(), 2);
        assert!(merged[0].contains("◆"));
    }

    #[test]
    fn skips_old_dream_results() {
        let mut sources = seed();
        sources.push(DreamSource::new(
            "mem-dream-1",
            10,
            "【做梦整合】线代 ◆ 高数",
        ));
        sources.push(DreamSource::new(
            "mem-dream-thought-1",
            11,
            "【思维链盘点】cluster-a:2 篇",
        ));
        let items = select_dream_candidates(&sources, 0);
        assert_eq!(items.len(), 4, "dream products must not re-enter");
        assert!(!items.iter().any(|c| c.contains("【做梦整合】")));
    }

    #[test]
    fn incremental_window_drops_pre_cycle_items() {
        let sources = seed();
        let items = select_dream_candidates(&sources, 3);
        assert_eq!(items.len(), 2, "timestamp >= last_cycle_at");
        assert_eq!(items[0], "明天交线代作业");
        assert_eq!(pair_merge(&items).len(), 1);
    }

    #[test]
    fn empty_or_odd_tail() {
        assert!(select_dream_candidates(&[], 0).is_empty());
        assert!(pair_merge(&["only".into()]).is_empty());
        assert_eq!(pair_merge(&["a".into(), "b".into(), "c".into()]).len(), 1);
    }
}
