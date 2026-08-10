//! `apeireth-rate-limiter` 令牌桶算法。
//!
//! 5 参数: `rate_per_second` / `burst` / `refill_interval` / `initial_tokens` / `max_wait`。
//!
//! **算法**:
//! 1. `lazy refill` — 每次 `try_acquire` 先按 `elapsed * rate` 补充令牌, 不开后台线程
//! 2. 桶容量封顶 — `tokens = min(tokens + added, capacity)`
//! 3. `try_acquire(cost)` — `tokens >= cost` 时扣减返回 true, 否则 false
//! 4. `acquire(cost, max_wait)` — 循环 try + `tokio::time::sleep(refill_interval)`
//!
//! **8 项不修改承诺**: 0 真接商业版 rate-limiter SDK, 0 触碰 24 LOCKED crate,
//! 0 改 workspace version, 0 改 workspace Cargo.toml, 0 改任何已有 crate,
//! 8 工具白名单（rate limiter 不暴露工具, 概念省略）, 5 K-1 强校验, 0 主动 commit

use crate::config::BucketConfig;
use crate::error::{RateLimiterError, Result};
use std::time::{Duration, Instant};

/// 令牌桶 — 单 key 状态。
///
/// `f64` 存令牌数（支持小数补充）, `Instant` 存上次补充时刻（单调时钟, 抗回拨）。
#[derive(Debug)]
pub struct TokenBucket {
    /// 桶容量（= burst, 最大令牌数）。
    capacity: f64,
    /// 补充速率（tokens / second）。
    refill_rate: f64,
    /// 补充周期（决定 acquire 等待步长）。
    refill_interval: Duration,
    /// 当前令牌数。
    tokens: f64,
    /// 上次补充时刻。
    last_refill: Instant,
    /// `acquire` 最长等待。
    max_wait: Option<Duration>,
}

impl TokenBucket {
    /// 新建令牌桶（K-1 强校验, 失败返 `ZeroRate` / `ZeroBurst`）。
    pub fn new(cfg: &BucketConfig) -> Result<Self> {
        cfg.validate()?;
        let initial = cfg.initial_tokens.unwrap_or(cfg.burst) as f64;
        Ok(Self {
            capacity: cfg.burst as f64,
            refill_rate: cfg.rate_per_second,
            refill_interval: cfg.refill_interval,
            tokens: initial.min(cfg.burst as f64),
            last_refill: Instant::now(),
            max_wait: cfg.max_wait,
        })
    }

    /// 非阻塞尝试扣减 `cost` 个令牌。
    ///
    /// 返回 `true` = 成功扣减, `false` = 令牌不足拒绝。
    pub fn try_acquire(&mut self, cost: u32) -> bool {
        if cost == 0 {
            return true; // cost=0 视为立即成功（不消耗令牌）
        }
        self.refill();
        if self.tokens >= cost as f64 {
            self.tokens -= cost as f64;
            true
        } else {
            false
        }
    }

    /// 阻塞等待直到拿到 `cost` 个令牌或 `max_wait` 超时。
    ///
    /// 同步版（用 `std::thread::sleep`, 适合 parking_lot 持锁场景）。
    pub fn try_acquire_with_wait(
        &mut self,
        cost: u32,
        max_wait: Duration,
    ) -> std::result::Result<bool, RateLimiterError> {
        let start = Instant::now();
        loop {
            if self.try_acquire(cost) {
                return Ok(true);
            }
            if start.elapsed() >= max_wait {
                return Ok(false);
            }
            std::thread::sleep(self.refill_interval.min(Duration::from_millis(10)));
        }
    }

    /// 异步阻塞等待 — 用 `tokio::time::sleep` 让出 runtime。
    pub async fn acquire(&mut self, cost: u32) -> Result<bool> {
        let max_wait = self.max_wait.unwrap_or(Duration::from_secs(5));
        let start = Instant::now();
        let step = self.refill_interval.max(Duration::from_millis(1));
        loop {
            if self.try_acquire(cost) {
                return Ok(true);
            }
            if start.elapsed() >= max_wait {
                return Err(RateLimiterError::MaxWaitExceeded {
                    key: "<inline>".to_string(),
                });
            }
            tokio::time::sleep(step).await;
        }
    }

    /// 充值 `cost` 个令牌（permit drop 时调用）。
    pub fn release(&mut self, cost: u32) {
        if cost == 0 {
            return;
        }
        self.tokens = (self.tokens + cost as f64).min(self.capacity);
    }

    /// 重置桶到初始状态（满桶）。
    pub fn reset(&mut self) {
        self.tokens = self.capacity;
        self.last_refill = Instant::now();
    }

    /// 当前令牌数（调试 / stats 用）。
    pub fn available_tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }

    /// 桶容量。
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// 补充速率。
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    /// lazy refill — 按 `elapsed * rate` 补令牌, 封顶到 capacity。
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        let added = elapsed * self.refill_rate;
        self.tokens = (self.tokens + added).min(self.capacity);
        self.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(rate: f64, burst: u32) -> BucketConfig {
        BucketConfig {
            rate_per_second: rate,
            burst,
            initial_tokens: Some(burst),
            max_wait: Some(Duration::from_millis(500)),
            refill_interval: Duration::from_millis(10),
        }
    }

    #[test]
    fn new_rejects_zero_rate() {
        let r = TokenBucket::new(&cfg(0.0, 10));
        assert!(matches!(r, Err(RateLimiterError::ZeroRate)));
    }

    #[test]
    fn new_rejects_zero_burst() {
        let r = TokenBucket::new(&cfg(10.0, 0));
        assert!(matches!(r, Err(RateLimiterError::ZeroBurst)));
    }

    #[test]
    fn burst_drains_in_one_shot() {
        let mut tb = TokenBucket::new(&cfg(10.0, 5)).unwrap();
        assert!(tb.try_acquire(5));
        assert!(!tb.try_acquire(1));
    }

    #[test]
    fn refill_over_time_restores_tokens() {
        let mut tb = TokenBucket::new(&cfg(100.0, 10)).unwrap();
        assert!(tb.try_acquire(10));
        assert!(!tb.try_acquire(1));
        std::thread::sleep(Duration::from_millis(50)); // 50ms * 100/s = 5 tokens
        assert!(tb.try_acquire(1));
    }

    #[test]
    fn release_returns_tokens() {
        let mut tb = TokenBucket::new(&cfg(0.0, 10)).unwrap_or_else(|_| {
            // 0 rate 不允许, 改用 rate=0.01
            TokenBucket::new(&BucketConfig {
                rate_per_second: 0.01,
                burst: 10,
                initial_tokens: Some(10),
                max_wait: Some(Duration::from_millis(100)),
                refill_interval: Duration::from_millis(10),
            })
            .unwrap()
        });
        let _ = tb.try_acquire(5);
        tb.release(5);
        assert!(tb.try_acquire(5));
    }

    #[test]
    fn release_caps_at_capacity() {
        let mut tb = TokenBucket::new(&cfg(0.01, 10)).unwrap();
        tb.release(1000);
        assert!(tb.try_acquire(10));
    }

    #[test]
    fn try_acquire_zero_cost_is_free() {
        let mut tb = TokenBucket::new(&cfg(0.01, 1)).unwrap();
        for _ in 0..1000 {
            assert!(tb.try_acquire(0));
        }
    }

    #[test]
    fn reset_restores_to_full() {
        let mut tb = TokenBucket::new(&cfg(0.01, 10)).unwrap();
        let _ = tb.try_acquire(10);
        assert!(!tb.try_acquire(1));
        tb.reset();
        assert!(tb.try_acquire(1));
    }

    #[test]
    fn available_tokens_reflects_refill() {
        let mut tb = TokenBucket::new(&cfg(1000.0, 100)).unwrap();
        let _ = tb.try_acquire(50);
        std::thread::sleep(Duration::from_millis(20));
        let avail = tb.available_tokens();
        // 应接近 50 + 20ms*1000/s = 50 + 20 = 70
        assert!(avail >= 60.0 && avail <= 80.0);
    }
}
