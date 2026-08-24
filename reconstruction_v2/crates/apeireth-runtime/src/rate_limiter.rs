//! RateLimiter - 限流 (从 v1.0 apeireth-rate-limiter 3,714 LOC 收敛)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LimitStrategy {
    TokenBucket { rate_per_sec: f64, burst: u32 },
    FixedWindow { max_per_window: u32, window: Duration },
}

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, BucketState>>>,
}

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
    window_count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new() -> Self { Self { buckets: Arc::new(RwLock::new(HashMap::new())) } }

    pub async fn try_acquire(&self, key: &str, strategy: LimitStrategy) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let entry = buckets.entry(key.to_string()).or_insert_with(|| BucketState {
            tokens: match strategy { LimitStrategy::TokenBucket { burst, .. } => burst as f64, _ => 0.0 },
            last_refill: now, window_count: 0, window_start: now,
        });
        match strategy {
            LimitStrategy::TokenBucket { rate_per_sec, burst } => {
                let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
                let refill = elapsed * rate_per_sec;
                entry.tokens = (entry.tokens + refill).min(burst as f64);
                entry.last_refill = now;
                if entry.tokens >= 1.0 { entry.tokens -= 1.0; true } else { false }
            }
            LimitStrategy::FixedWindow { max_per_window, window } => {
                if now.duration_since(entry.window_start) >= window {
                    entry.window_count = 0; entry.window_start = now;
                }
                if entry.window_count < max_per_window { entry.window_count += 1; true } else { false }
            }
        }
    }
}

impl Default for RateLimiter { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*; use std::thread::sleep;
    #[tokio::test] async fn test_token_bucket_basic() {
        let rl = RateLimiter::new();
        let s = LimitStrategy::TokenBucket { rate_per_sec: 1.0, burst: 3 };
        assert!(rl.try_acquire("k", s).await);
        assert!(rl.try_acquire("k", s).await);
        assert!(rl.try_acquire("k", s).await);
        assert!(!rl.try_acquire("k", s).await);
    }
    #[tokio::test] async fn test_fixed_window() {
        let rl = RateLimiter::new();
        let s = LimitStrategy::FixedWindow { max_per_window: 2, window: Duration::from_millis(100) };
        assert!(rl.try_acquire("k", s).await);
        assert!(rl.try_acquire("k", s).await);
        assert!(!rl.try_acquire("k", s).await);
        sleep(Duration::from_millis(150));
        assert!(rl.try_acquire("k", s).await);
    }
}
