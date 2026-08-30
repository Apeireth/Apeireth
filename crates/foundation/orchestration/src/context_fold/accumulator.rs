//! Cross-session token accumulator.
//!
//! Honest: [`approx_tokens`] is `chars / 4` (no tiktoken). The map is a
//! [`BTreeMap`] so snapshots serialize in a stable key order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Approximate token count: chars / 4 (no tiktoken dep, per honest scope).
pub fn approx_tokens(s: &str) -> usize {
    s.chars().count() / 4
}

/// Point-in-time tally of recorded sessions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccumulatorSnapshot {
    /// Distinct session keys recorded.
    pub session_count: usize,
    /// Sum of recorded token counts.
    pub total_tokens: usize,
    /// Per-session totals (stable key order).
    pub per_session: BTreeMap<String, usize>,
}

/// Running cross-session token tally.
#[derive(Debug, Clone, Default)]
pub struct TokenAccumulator {
    snapshot: AccumulatorSnapshot,
}

impl TokenAccumulator {
    /// Empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record tokens for a session (additive; same id accumulates).
    pub fn record_session(&mut self, session_id: &str, tokens: usize) {
        let entry = self
            .snapshot
            .per_session
            .entry(session_id.to_string())
            .or_insert(0);
        *entry += tokens;
        self.snapshot.total_tokens += tokens;
        self.snapshot.session_count = self.snapshot.per_session.len();
    }

    /// Record tokens for an anonymous session (auto-id `anon-{count}`).
    pub fn record_anonymous(&mut self, tokens: usize) {
        let id = format!("anon-{}", self.snapshot.session_count);
        self.record_session(&id, tokens);
    }

    /// Borrow the current snapshot.
    pub fn snapshot(&self) -> &AccumulatorSnapshot {
        &self.snapshot
    }

    /// Total recorded tokens.
    pub fn total_tokens(&self) -> usize {
        self.snapshot.total_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_tokens_basic() {
        assert_eq!(approx_tokens(""), 0);
        assert_eq!(approx_tokens("abcd"), 1);
        assert_eq!(approx_tokens("abcde"), 1);
        assert_eq!(approx_tokens("abcdefgh"), 2);
    }

    #[test]
    fn approx_tokens_chinese() {
        assert_eq!(approx_tokens("你好"), 0);
        assert_eq!(approx_tokens("你好世界你好世界"), 2);
    }

    #[test]
    fn accumulator_initial_empty() {
        let a = TokenAccumulator::new();
        assert_eq!(a.total_tokens(), 0);
        assert_eq!(a.snapshot().session_count, 0);
    }

    #[test]
    fn accumulator_record_session() {
        let mut a = TokenAccumulator::new();
        a.record_session("s1", 100);
        a.record_session("s1", 50);
        a.record_session("s2", 200);
        assert_eq!(a.total_tokens(), 350);
        assert_eq!(a.snapshot().session_count, 2);
        assert_eq!(a.snapshot().per_session.get("s1"), Some(&150));
        assert_eq!(a.snapshot().per_session.get("s2"), Some(&200));
    }

    #[test]
    fn accumulator_anonymous() {
        let mut a = TokenAccumulator::new();
        a.record_anonymous(10);
        a.record_anonymous(20);
        assert_eq!(a.total_tokens(), 30);
        assert_eq!(a.snapshot().session_count, 2);
    }

    #[test]
    fn snapshot_key_order_is_stable() {
        let mut a = TokenAccumulator::new();
        a.record_session("z", 1);
        a.record_session("a", 2);
        let keys: Vec<_> = a.snapshot().per_session.keys().cloned().collect();
        assert_eq!(keys, vec!["a".to_string(), "z".to_string()]);
    }
}
