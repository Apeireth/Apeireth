//! Bounded reliability combining circuit breaker + retries.

use std::time::Duration;
use crate::circuit_breaker::CircuitBreaker;
use crate::reliability::DefaultReliability;

pub struct BoundedReliability {
    pub reliability: DefaultReliability,
    pub breaker: CircuitBreaker,
}

impl BoundedReliability {
    pub fn new() -> Self {
        Self {
            reliability: DefaultReliability::new(),
            breaker: CircuitBreaker::new(10, Duration::from_secs(30)),
        }
    }

    pub fn can_proceed(&self) -> bool {
        !self.breaker.is_open()
    }

    pub fn record_success(&self) { self.breaker.record_success(); }
    pub fn record_failure(&self) { self.breaker.record_failure(); }
}

impl Default for BoundedReliability {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_proceed_default() {
        let r = BoundedReliability::new();
        assert!(r.can_proceed());
    }

    #[test]
    fn breaker_integrated() {
        let r = BoundedReliability::new();
        for _ in 0..10 { r.record_failure(); }
        assert!(!r.can_proceed());
        r.record_success();
        assert!(r.can_proceed());
    }
}
