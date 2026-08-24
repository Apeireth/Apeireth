//! 15s retry suppression window (VCP §6.2.2 #19).

use std::time::{Duration, Instant as TimeInstant};

pub const DEFAULT_SUPPRESSION_WINDOW_MS: u64 = 15_000;

#[derive(Debug, Clone)]
pub struct RetrySuppression {
    pub window: Duration,
    pub last_failure: Option<TimeInstant>,
}

impl RetrySuppression {
    pub fn with_chat_default() -> Self {
        Self { window: Duration::from_millis(DEFAULT_SUPPRESSION_WINDOW_MS), last_failure: None }
    }

    pub fn new(window: Duration) -> Self {
        Self { window, last_failure: None }
    }

    pub fn record_failure(&mut self) {
        self.last_failure = Some(TimeInstant::now());
    }

    pub fn allow(&self) -> bool {
        match self.last_failure {
            None => true,
            Some(t) => t.elapsed() >= self.window,
        }
    }

    pub fn remaining_ms(&self) -> u64 {
        match self.last_failure {
            None => 0,
            Some(t) => {
                let elapsed = t.elapsed();
                if elapsed >= self.window { 0 } else { (self.window - elapsed).as_millis() as u64 }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allow() {
        let s = RetrySuppression::with_chat_default();
        assert!(s.allow());
        assert_eq!(s.remaining_ms(), 0);
    }

    #[test]
    fn after_failure_suppressed() {
        let mut s = RetrySuppression::with_chat_default();
        s.record_failure();
        assert!(!s.allow());
        assert!(s.remaining_ms() > 0);
    }

    #[test]
    fn constant_value() {
        assert_eq!(DEFAULT_SUPPRESSION_WINDOW_MS, 15_000);
    }
}
