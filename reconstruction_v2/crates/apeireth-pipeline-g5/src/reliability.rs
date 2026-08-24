//! Stage 3: Default reliability (retries + idempotency).

use std::time::Duration;

pub const CIRCUIT_BREAKER_THRESHOLD: u32 = 10;
pub const IDEMPOTENCY_KEY_PREFIX: &str = "sandbox-";
pub const MAX_RETRY_ATTEMPTS: u32 = 5;
pub const RETRY_BACKOFF_MS: [u64; 4] = [100, 200, 400, 800];

#[derive(Debug, Default, Clone)]
pub struct DefaultReliability;

impl DefaultReliability {
    pub fn new() -> Self { Self }
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let i = (attempt as usize).saturating_sub(1).min(RETRY_BACKOFF_MS.len() - 1);
        Duration::from_millis(RETRY_BACKOFF_MS[i])
    }
    pub fn idempotency_key(&self, suffix: &str) -> String {
        format!("{IDEMPOTENCY_KEY_PREFIX}{suffix}")
    }
    pub fn should_circuit_break(&self, failures: u32) -> bool {
        failures >= CIRCUIT_BREAKER_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(CIRCUIT_BREAKER_THRESHOLD, 10);
        assert_eq!(MAX_RETRY_ATTEMPTS, 5);
        assert_eq!(IDEMPOTENCY_KEY_PREFIX, "sandbox-");
        assert_eq!(RETRY_BACKOFF_MS.len(), 4);
    }

    #[test]
    fn backoff_progression() {
        let r = DefaultReliability::new();
        assert_eq!(r.backoff_for_attempt(1).as_millis(), 100);
        assert_eq!(r.backoff_for_attempt(2).as_millis(), 200);
        assert_eq!(r.backoff_for_attempt(3).as_millis(), 400);
        assert_eq!(r.backoff_for_attempt(4).as_millis(), 800);
        assert_eq!(r.backoff_for_attempt(100).as_millis(), 800); // clamped
    }

    #[test]
    fn idempotency_key_format() {
        let r = DefaultReliability::new();
        assert_eq!(r.idempotency_key("abc"), "sandbox-abc");
    }

    #[test]
    fn circuit_breaker_threshold() {
        let r = DefaultReliability::new();
        assert!(!r.should_circuit_break(5));
        assert!(r.should_circuit_break(10));
        assert!(r.should_circuit_break(15));
    }
}
