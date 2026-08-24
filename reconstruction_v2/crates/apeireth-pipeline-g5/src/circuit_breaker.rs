//! Circuit breaker.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct CircuitBreaker {
    failures: AtomicU32,
    last_failure_ms: AtomicU64,
    open: std::sync::atomic::AtomicBool,
    threshold: u32,
    cooldown: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, cooldown: Duration) -> Self {
        Self {
            failures: AtomicU32::new(0),
            last_failure_ms: AtomicU64::new(0),
            open: std::sync::atomic::AtomicBool::new(false),
            threshold,
            cooldown,
        }
    }

    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.open.store(false, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        let n = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if n >= self.threshold {
            self.open.store(true, Ordering::Relaxed);
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            self.last_failure_ms.store(now, Ordering::Relaxed);
        }
    }

    pub fn is_open(&self) -> bool {
        if !self.open.load(Ordering::Relaxed) { return false; }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let last = self.last_failure_ms.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= self.cooldown.as_millis() as u64 {
            self.open.store(false, Ordering::Relaxed);
            return false;
        }
        true
    }

    pub fn failures(&self) -> u32 { self.failures.load(Ordering::Relaxed) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_by_default() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        assert!(!cb.is_open());
    }

    #[test]
    fn opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_open());
        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn success_resets() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failures(), 0);
        assert!(!cb.is_open());
    }
}
