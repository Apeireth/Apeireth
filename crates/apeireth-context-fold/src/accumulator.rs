//! Cross-session token accumulator.

use std::collections::HashMap;

/// Approximate token count: chars / 4 (no tiktoken dep, per honest scope).
pub fn approx_tokens(s: &str) -> usize {
    s.chars().count() / 4
}

#[derive(Debug, Clone, Default)]
pub struct AccumulatorSnapshot {
    pub session_count: usize,
    pub total_tokens: usize,
    pub per_session: HashMap<String, usize>,
}

pub struct TokenAccumulator {
    snapshot: AccumulatorSnapshot,
}

impl TokenAccumulator {
    pub fn new() -> Self {
        Self { snapshot: AccumulatorSnapshot::default() }
    }

    /// Record tokens for a session.
    pub fn record_session(&mut self, session_id: &str, tokens: usize) {
        let entry = self.snapshot.per_session.entry(session_id.to_string()).or_insert(0);
        *entry += tokens;
        self.snapshot.total_tokens += tokens;
        self.snapshot.session_count = self.snapshot.per_session.len();
    }

    /// Record tokens for an anonymous session (auto-id).
    pub fn record_anonymous(&mut self, tokens: usize) {
        let id = format!("anon-{}", self.snapshot.session_count);
        self.record_session(&id, tokens);
    }

    pub fn snapshot(&self) -> &AccumulatorSnapshot {
        &self.snapshot
    }

    pub fn total_tokens(&self) -> usize {
        self.snapshot.total_tokens
    }
}

impl Default for TokenAccumulator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_tokens_basic() {
        assert_eq!(approx_tokens(""), 0);
        assert_eq!(approx_tokens("abcd"), 1); // 4 chars / 4 = 1
        assert_eq!(approx_tokens("abcde"), 1); // 5 chars / 4 = 1
        assert_eq!(approx_tokens("abcdefgh"), 2); // 8 chars / 4 = 2
    }

    #[test]
    fn approx_tokens_chinese() {
        // Chinese chars are 1 char each but ~1-2 tokens
        assert_eq!(approx_tokens("你好"), 0); // 2 / 4 = 0
        assert_eq!(approx_tokens("你好世界你好世界"), 2); // 8 / 4 = 2
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
}