//! Token accumulator (cross-session).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Default, Clone)]
pub struct AccumulatorSnapshot {
    pub total_tokens: u64,
    pub sessions: HashMap<String, u64>,
}

pub struct TokenAccumulator {
    inner: Arc<RwLock<AccumulatorSnapshot>>,
}

impl Default for TokenAccumulator {
    fn default() -> Self {
        Self { inner: Arc::new(RwLock::new(AccumulatorSnapshot::default())) }
    }
}

impl TokenAccumulator {
    pub fn new() -> Self { Self::default() }

    /// Add tokens (chars / 4) for a session.
    pub fn add(&self, session_id: &str, text: &str) {
        let tokens = ((text.chars().count() as f64) / 4.0).ceil() as u64;
        let mut s = self.inner.write().unwrap();
        s.total_tokens += tokens;
        *s.sessions.entry(session_id.to_string()).or_insert(0) += tokens;
    }

    pub fn snapshot(&self) -> AccumulatorSnapshot {
        self.inner.read().unwrap().clone()
    }

    pub fn total(&self) -> u64 {
        self.inner.read().unwrap().total_tokens
    }

    pub fn for_session(&self, session_id: &str) -> u64 {
        self.inner.read().unwrap().sessions.get(session_id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulate_basic() {
        let a = TokenAccumulator::new();
        a.add("s1", "hello world"); // 11 chars -> 3 tokens
        assert_eq!(a.for_session("s1"), 3);
        assert_eq!(a.total(), 3);
    }

    #[test]
    fn snapshot_clone() {
        let a = TokenAccumulator::new();
        a.add("s1", "abc"); // 3 chars -> 1
        let s = a.snapshot();
        assert_eq!(s.total_tokens, 1);
        assert_eq!(s.sessions.get("s1"), Some(&1));
    }

    #[test]
    fn multiple_sessions() {
        let a = TokenAccumulator::new();
        a.add("s1", "abc");
        a.add("s2", "abcdefgh");
        assert_eq!(a.for_session("s1"), 1);
        assert_eq!(a.for_session("s2"), 2);
        assert_eq!(a.total(), 3);
    }
}
