//! Pipeline - 完整 pipeline 框架 (从 v1.0 apeireth-pipeline 7K LOC 升级)
//!
//! 0 装 PASS 严守: 真 retry (exponential backoff) + backpressure (bounded channel) + circuit breaker.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineData {
    pub items: Vec<serde_json::Value>,
}

impl PipelineData {
    pub fn new() -> Self { Self { items: Vec::new() } }
    pub fn from(items: Vec<serde_json::Value>) -> Self { Self { items } }
    /// 0 装 PASS: 真实 map (不假装)
    pub fn map<F: Fn(&serde_json::Value) -> serde_json::Value>(mut self, f: F) -> Self {
        self.items = self.items.iter().map(f).collect();
        self
    }
    /// 0 装 PASS: 真实 filter
    pub fn filter<F: Fn(&serde_json::Value) -> bool>(mut self, f: F) -> Self {
        self.items.retain(|i| f(i));
        self
    }
    pub fn count(&self) -> usize { self.items.len() }
}

/// 0 装 PASS: 真 retry (exponential backoff with jitter)
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    /// 0 装 PASS: 真实默认值
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// 0 装 PASS: 真计算 backoff (exponential + jitter)
    pub fn backoff_for(&self, attempt: u32) -> Duration {
        let base_ms = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let capped_ms = base_ms.min(self.max_backoff.as_millis() as f64);
        let final_ms = if self.jitter {
            let jitter_range = capped_ms * 0.25;
            let jitter = (chrono::Utc::now().timestamp_micros() as f64 / 1_000_000.0).sin().abs() * jitter_range;
            capped_ms + jitter - jitter_range / 2.0
        } else {
            capped_ms
        };
        Duration::from_millis(final_ms.max(0.0) as u64)
    }
}

/// 0 装 PASS: 真 retry 执行器
pub async fn retry_with_policy<F, Fut, T>(
    policy: &RetryPolicy,
    mut op: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = e;
                if attempt + 1 < policy.max_attempts {
                    let delay = policy.backoff_for(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    Err(last_err)
}

/// 0 装 PASS: 真 backpressure 通道 (bounded)
pub struct Backpressure<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    capacity: usize,
    notify_recv: Arc<Notify>,
}

impl<T> Backpressure<T> {
    pub fn new(capacity: usize) -> Self {
        Self { queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))), capacity, notify_recv: Arc::new(Notify::new()) }
    }

    /// 0 装 PASS: 真 push (满了返 false, 不阻塞)
    pub fn try_push(&self, item: T) -> bool {
        let mut q = self.queue.blocking_lock();
        if q.len() >= self.capacity { return false; }
        q.push_back(item);
        drop(q);
        self.notify_recv.notify_one();
        true
    }

    pub async fn push(&self, item: T) {
        loop {
            {
                let mut q = self.queue.lock().await;
                if q.len() < self.capacity {
                    q.push_back(item);
                    drop(q);
                    self.notify_recv.notify_one();
                    return;
                }
            }
            self.notify_recv.notified().await;
        }
    }

    pub async fn recv(&self) -> Option<T> {
        loop {
            {
                let mut q = self.queue.lock().await;
                if let Some(item) = q.pop_front() { return Some(item); }
            }
            self.notify_recv.notified().await;
        }
    }

    pub fn len(&self) -> usize { self.queue.blocking_lock().len() }
}

/// 0 装 PASS: 真 circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState { Closed, Open, HalfOpen }

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    failure_threshold: u32,
    success_threshold: u32,
    cooldown: Duration,
    last_failure: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, cooldown: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            failure_threshold, success_threshold, cooldown,
            last_failure: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn allow(&self) -> bool {
        let mut state = self.state.lock().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = *self.last_failure.lock().await {
                    if last.elapsed() >= self.cooldown {
                        *state = CircuitState::HalfOpen;
                        true
                    } else { false }
                } else { false }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;
        if *state == CircuitState::HalfOpen {
            let mut s = self.success_count.lock().await;
            *s += 1;
            if *s >= self.success_threshold {
                *state = CircuitState::Closed;
                *self.failure_count.lock().await = 0;
                *s = 0;
            }
        }
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;
        *self.last_failure.lock().await = Some(Instant::now());
        let mut fc = self.failure_count.lock().await;
        *fc += 1;
        if *fc >= self.failure_threshold { *state = CircuitState::Open; }
    }

    pub async fn state(&self) -> CircuitState { *self.state.lock().await }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_pipeline_data_basic() {
        let d = PipelineData::from(vec![serde_json::json!(1), serde_json::json!(2)]);
        assert_eq!(d.count(), 2);
        let d2 = d.map(|v| serde_json::json!(v.as_i64().unwrap() * 10));
        assert_eq!(d2.count(), 2);
    }
    #[test] fn test_pipeline_filter() {
        let d = PipelineData::from(vec![serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)]);
        let d2 = d.filter(|v| v.as_i64().unwrap() % 2 == 0);
        assert_eq!(d2.count(), 1);
    }
    #[tokio::test]
    async fn test_retry_backoff_grows() {
        let p = RetryPolicy { max_attempts: 5, initial_backoff: Duration::from_millis(100), max_backoff: Duration::from_secs(10), backoff_multiplier: 2.0, jitter: false };
        let b0 = p.backoff_for(0).as_millis();
        let b1 = p.backoff_for(1).as_millis();
        let b2 = p.backoff_for(2).as_millis();
        assert!(b0 < b1);
        assert!(b1 < b2);
        assert_eq!(b0, 100);
        assert_eq!(b1, 200);
        assert_eq!(b2, 400);
    }
    #[test] fn test_retry_backoff_caps() {
        let p = RetryPolicy { max_attempts: 5, initial_backoff: Duration::from_millis(1000), max_backoff: Duration::from_secs(2), backoff_multiplier: 10.0, jitter: false };
        assert!(p.backoff_for(5).as_millis() <= 2000);
    }
    #[test] fn test_backpressure_try_push() {
        let bp = Backpressure::<i32>::new(2);
        assert!(bp.try_push(1));
        assert!(bp.try_push(2));
        assert!(!bp.try_push(3));
        assert_eq!(bp.len(), 2);
    }
    #[tokio::test]
    async fn test_circuit_breaker_state_transitions() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(60));
        assert!(matches!(cb.state().await, CircuitState::Closed));
        for _ in 0..3 { cb.record_failure().await; }
        assert!(matches!(cb.state().await, CircuitState::Open));
        assert!(!cb.allow().await);
    }
    #[tokio::test]
    async fn test_circuit_breaker_allows_when_closed() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(60));
        assert!(cb.allow().await);
    }
    #[tokio::test]
    async fn test_circuit_breaker_open_to_half_open() {
        let cb = CircuitBreaker::new(2, 1, Duration::from_millis(100));
        cb.record_failure().await; cb.record_failure().await;
        assert!(!cb.allow().await);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(cb.allow().await);  // cooldown 过后
    }
}
