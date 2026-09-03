//! Retention policy + sweep (salvage of context-ledger prune, layered-memo decay, governance protect).
//!
//! Combines three donor / v2 primitives that previously lived in isolation:
//! - **Count cap** (Context ledger `max_records` per subject) — keep the newest N.
//! - **Age TTL** — forget episodes older than `max_age_secs`.
//! - **Decay threshold** — forget when Ebbinghaus strength drops below `min_strength`.
//!
//! Sweep never hard-deletes episode rows (append-only). It writes governance
//! sidecar `forgotten` via [`crate::MemoryGovernanceStore::forget_episode`].
//! Protected episodes are skipped. Already-forgotten rows are skipped.
//!
//! ContextLedger's own rolling DELETE stays inside [`crate::context_ledger`] because a
//! ledger is a recent-window, not an archive.

use crate::memory_governance::{
    MemoryGovernanceStatus, MemoryGovernanceStore,
};
use crate::{EpisodeQuery, EpisodeStore, MemoryResult, SqliteMemoryStore};

/// Ebbinghaus strength at `now_unix` relative to `last_unix` (both epoch seconds).
///
/// `strength = 0.5 ^ (elapsed_hours / half_life_hours)`. Computed against the
/// sweep clock so tests (and scheduled jobs) are deterministic — donor
/// `DecayEngine::strength` always reads `SystemClock::now()`, which would make
/// a sweep at a caller-supplied `now_unix` wall-clock-dependent.
pub fn decay_strength(last_unix: i64, now_unix: i64, half_life_hours: f64) -> f32 {
    let half = half_life_hours.max(0.001);
    let elapsed_hours = (now_unix.saturating_sub(last_unix) as f64) / 3600.0;
    0.5f64.powf(elapsed_hours / half) as f32
}

/// Combined retention policy. All fields optional; a `None` / zero value
/// disables that axis. At least one axis must be active for a sweep to do work.
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Keep at most this many active (non-forgotten, unprotected) episodes
    /// per `session_id`. Oldest beyond the cap are forgotten. `None` = no cap.
    pub max_count: Option<usize>,
    /// Forget episodes whose `timestamp` is older than `now_unix - max_age_secs`.
    pub max_age_secs: Option<i64>,
    /// Forget when decay strength (half-life hours) falls below this (0, 1].
    pub min_strength: Option<f32>,
    /// Half-life used when `min_strength` is set. Default 24h.
    pub half_life_hours: f64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_count: None,
            max_age_secs: None,
            min_strength: None,
            half_life_hours: 24.0,
        }
    }
}

impl RetentionPolicy {
    pub fn with_max_count(mut self, n: usize) -> Self {
        self.max_count = Some(n.max(1));
        self
    }

    pub fn with_max_age_secs(mut self, secs: i64) -> Self {
        self.max_age_secs = Some(secs.max(1));
        self
    }

    pub fn with_min_strength(mut self, strength: f32, half_life_hours: f64) -> Self {
        self.min_strength = Some(strength.clamp(0.0, 1.0));
        self.half_life_hours = half_life_hours.max(0.001);
        self
    }

    pub fn is_active(&self) -> bool {
        self.max_count.is_some() || self.max_age_secs.is_some() || self.min_strength.is_some()
    }
}

/// Result of one sweep.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetentionSweepReport {
    pub scanned: usize,
    pub forgotten: usize,
    pub skipped_protected: usize,
    pub skipped_already_forgotten: usize,
}

/// Sweep `session_id` under `policy` at `now_unix` (epoch seconds).
///
/// Empty session_id is rejected. Inactive policy is a no-op (scanned = 0).
pub fn sweep_session(
    store: &SqliteMemoryStore,
    session_id: &str,
    policy: &RetentionPolicy,
    now_unix: i64,
) -> MemoryResult<RetentionSweepReport> {
    if session_id.trim().is_empty() {
        return Err(crate::MemoryError::Invalid(
            "retention sweep session_id is empty".into(),
        ));
    }
    if !policy.is_active() {
        return Ok(RetentionSweepReport::default());
    }

    let episodes = <SqliteMemoryStore as EpisodeStore>::query(
        store,
        &EpisodeQuery::new().for_session(session_id),
    )?;

    let mut report = RetentionSweepReport {
        scanned: episodes.len(),
        ..RetentionSweepReport::default()
    };

    // Age / decay candidates first (any that fail those tests, regardless of count).
    let mut keep: Vec<(String, i64, i64)> = Vec::new(); // (id, timestamp, revision)
    for ep in &episodes {
        let Some(governed) = store.get_governed(&ep.id)? else {
            continue;
        };
        if governed.status == MemoryGovernanceStatus::Forgotten {
            report.skipped_already_forgotten += 1;
            continue;
        }
        if governed.protected {
            report.skipped_protected += 1;
            continue;
        }

        let mut drop_it = false;
        if let Some(max_age) = policy.max_age_secs {
            if now_unix.saturating_sub(ep.timestamp) > max_age {
                drop_it = true;
            }
        }
        if !drop_it {
            if let Some(min_s) = policy.min_strength {
                if decay_strength(ep.timestamp, now_unix, policy.half_life_hours) < min_s {
                    drop_it = true;
                }
            }
        }
        if drop_it {
            match store.forget_episode(&ep.id, Some("retention-sweep"), governed.revision) {
                Ok(_) => report.forgotten += 1,
                Err(crate::memory_governance::MemoryGovernanceError::Protected(_)) => {
                    report.skipped_protected += 1;
                }
                Err(crate::memory_governance::MemoryGovernanceError::AlreadyForgotten(_)) => {
                    report.skipped_already_forgotten += 1;
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            keep.push((ep.id.clone(), ep.timestamp, governed.revision));
        }
    }

    // Count cap: among remaining keepers, drop oldest beyond max_count.
    if let Some(max_count) = policy.max_count {
        if keep.len() > max_count {
            keep.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
            let overflow = keep.len() - max_count;
            let victims: Vec<(String, i64)> = keep
                .iter()
                .take(overflow)
                .map(|(id, _, rev)| (id.clone(), *rev))
                .collect();
            for (id, rev) in victims {
                match store.forget_episode(&id, Some("retention-count-cap"), rev) {
                    Ok(_) => report.forgotten += 1,
                    Err(crate::memory_governance::MemoryGovernanceError::Protected(_)) => {
                        report.skipped_protected += 1;
                    }
                    Err(crate::memory_governance::MemoryGovernanceError::AlreadyForgotten(_)) => {
                        report.skipped_already_forgotten += 1;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EpisodeStore, MemoryGovernanceStore};
    use apeireth_core::kernel::memory::Episode;

    fn put(store: &SqliteMemoryStore, id: &str, session: &str, ts: i64) {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            store,
            &Episode {
                id: id.into(),
                timestamp: ts,
                role: "user".into(),
                content: format!("body-{id}"),
                session_id: session.into(),
            },
        )
        .unwrap();
    }

    #[test]
    fn inactive_policy_is_noop() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        put(&store, "e1", "s", 100);
        let r = sweep_session(&store, "s", &RetentionPolicy::default(), 1_000).unwrap();
        assert_eq!(r, RetentionSweepReport::default());
        assert_eq!(store.governed_recent_episodes("s", 10).unwrap().len(), 1);
    }

    #[test]
    fn age_ttl_forgets_old_keeps_fresh() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        put(&store, "old", "s", 10);
        put(&store, "fresh", "s", 900);
        let policy = RetentionPolicy::default().with_max_age_secs(100);
        let r = sweep_session(&store, "s", &policy, 1_000).unwrap();
        assert_eq!(r.forgotten, 1);
        let live = store.governed_recent_episodes("s", 10).unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].episode.id, "fresh");
    }

    #[test]
    fn count_cap_keeps_newest() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        put(&store, "a", "s", 1);
        put(&store, "b", "s", 2);
        put(&store, "c", "s", 3);
        let policy = RetentionPolicy::default().with_max_count(2);
        let r = sweep_session(&store, "s", &policy, 10).unwrap();
        assert_eq!(r.forgotten, 1);
        let live = store.governed_recent_episodes("s", 10).unwrap();
        let ids: Vec<&str> = live.iter().map(|g| g.episode.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c"]);
    }

    #[test]
    fn protected_episode_is_skipped() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        put(&store, "keep", "s", 1);
        put(&store, "drop", "s", 2);
        store.protect_episode("keep", 0).unwrap();
        let policy = RetentionPolicy::default().with_max_count(1);
        let r = sweep_session(&store, "s", &policy, 10).unwrap();
        assert_eq!(r.skipped_protected, 1);
        // only "drop" is in the keep-set for the cap, so nothing extra forgotten
        // beyond possibly drop if cap applies to unprotected only.
        let live = store.governed_recent_episodes("s", 10).unwrap();
        assert!(live.iter().any(|g| g.episode.id == "keep" && g.protected));
    }

    #[test]
    fn empty_session_rejected() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let policy = RetentionPolicy::default().with_max_count(1);
        assert!(sweep_session(&store, "  ", &policy, 0).is_err());
    }

    #[test]
    fn decay_strength_is_deterministic_against_sweep_clock() {
        assert!((decay_strength(0, 0, 24.0) - 1.0).abs() < 1e-6);
        assert!((decay_strength(0, 24 * 3600, 24.0) - 0.5).abs() < 1e-5);
        assert!(decay_strength(0, 48 * 3600, 24.0) < 0.26);
    }

    #[test]
    fn min_strength_forgets_decayed_keeps_fresh() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        put(&store, "old", "s", 0);
        put(&store, "fresh", "s", 7200);
        let policy = RetentionPolicy::default().with_min_strength(0.5, 1.0);
        let r = sweep_session(&store, "s", &policy, 7200).unwrap();
        assert_eq!(r.forgotten, 1);
        let live = store.governed_recent_episodes("s", 10).unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].episode.id, "fresh");
        // Original rows remain (append-only); forget is governance sidecar.
        assert_eq!(
            <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, "s").unwrap(),
            2
        );
    }
}
