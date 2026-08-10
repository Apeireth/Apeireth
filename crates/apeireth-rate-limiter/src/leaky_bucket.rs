//! `apeireth-rate-limiter` 漏桶算法。
//!
//! 4 参数: `rate` / `capacity` / `drip_interval` / `overflow_policy`。
//!
//! **算法**:
//! 1. 与令牌桶对称 — 请求是「水滴」, 桶以 rate 速度漏水
//! 2. `lazy drip` — 每次 `try_acquire` 先按 `elapsed * rate` 漏水, 不开后台
//! 3. `overflow_policy`:
//!    - `Drop` (默认): 桶满时拒绝
//!    - `Block`: 桶满时阻塞, 等漏水后塞入
//!
//! **与 token bucket 的区别**:
//! - 令牌桶允许 burst（突发消耗满桶后还能补充）
//! - 漏桶严格平滑输出（无论输入多快, 输出恒定 rate）
//!
//! **8 项不修改承诺**: 同 token_bucket。

use crate::config::{BucketConfig, LeakyBucketOverflow};
use crate::error::{RateLimiterError, Result};
use std::time::{Duration, Instant};

/// 漏桶 — 单 key 状态。
#[derive(Debug)]
pub struct LeakyBucket {
    /// 漏出速率（requests / second）。
    drip_rate: f64,
    /// 桶容量。
    capacity: f64,
    /// 漏出周期。
    drip_interval: Duration,
    /// 当前桶内积压（0 = 空桶）。
    level: f64,
    /// 上次漏水时刻。
    last_drip: Instant,
    /// 溢出策略。
    overflow_policy: LeakyBucketOverflow,
    /// 阻塞模式下最长等待。
    max_wait: Option<Duration>,
}

impl LeakyBucket {
    /// 新建漏桶（K-1 强校验）。
    pub fn new(cfg: &BucketConfig, overflow: LeakyBucketOverflow) -> Result<Self> {
        cfg.validate()?;
        Ok(Self {
            drip_rate: cfg.rate_per_second,
            capacity: cfg.burst as f64,
            drip_interval: cfg.refill_interval,
            level: 0.0,
            last_drip: Instant::now(),
            overflow_policy: overflow,
            max_wait: cfg.max_wait,
        })
    }

    /// 非阻塞尝试加入 `cost` 个水滴。
    ///
    /// - `Drop` 策略: 桶满返 false
    /// - `Block` 策略: 桶满返 false（不阻塞, 调用方决定）
    pub fn try_acquire(&mut self, cost: u32) -> bool {
        if cost == 0 {
            return true;
        }
        self.drip();
        if self.level + cost as f64 <= self.capacity {
            self.level += cost as f64;
            true
        } else {
            false
        }
    }

    /// 阻塞等待（仅 `Block` 策略有效）。
    pub async fn acquire(&mut self, cost: u32) -> Result<bool> {
        let max_wait = self.max_wait.unwrap_or(Duration::from_secs(5));
        let start = Instant::now();
        let step = self.drip_interval.max(Duration::from_millis(1));
        loop {
            if self.try_acquire(cost) {
                return Ok(true);
            }
            if self.overflow_policy == LeakyBucketOverflow::Drop {
                return Ok(false);
            }
            if start.elapsed() >= max_wait {
                return Err(RateLimiterError::MaxWaitExceeded {
                    key: "<leaky>".to_string(),
                });
            }
            tokio::time::sleep(step).await;
        }
    }

    /// 同步阻塞（parking_lot + std::thread）— 配 `try_acquire_with_wait` 测 Block 策略。
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
            std::thread::sleep(self.drip_interval.min(Duration::from_millis(10)));
        }
    }

    /// 排出 `cost` 个水滴（permit drop 时调用 — 不常用, 漏桶语义上水滴不应回退）。
    pub fn release(&mut self, cost: u32) {
        if cost == 0 {
            return;
        }
        self.level = (self.level - cost as f64).max(0.0);
    }

    /// 重置桶为空。
    pub fn reset(&mut self) {
        self.level = 0.0;
        self.last_drip = Instant::now();
    }

    /// 当前桶内积压（调试 / stats）。
    pub fn current_level(&mut self) -> f64 {
        self.drip();
        self.level
    }

    /// 桶容量。
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// 漏出速率。
    pub fn drip_rate(&self) -> f64 {
        self.drip_rate
    }

    /// lazy drip — 按 `elapsed * rate` 漏水, 下限 0。
    fn drip(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_drip).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        let leaked = elapsed * self.drip_rate;
        self.level = (self.level - leaked).max(0.0);
        self.last_drip = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(rate: f64, burst: u32) -> BucketConfig {
        BucketConfig {
            rate_per_second: rate,
            burst,
            initial_tokens: Some(0),
            max_wait: Some(Duration::from_millis(500)),
            refill_interval: Duration::from_millis(10),
        }
    }

    #[test]
    fn new_rejects_zero_rate() {
        let r = LeakyBucket::new(&cfg(0.0, 10), LeakyBucketOverflow::Drop);
        assert!(matches!(r, Err(RateLimiterError::ZeroRate)));
    }

    #[test]
    fn new_rejects_zero_burst() {
        let r = LeakyBucket::new(&cfg(10.0, 0), LeakyBucketOverflow::Drop);
        assert!(matches!(r, Err(RateLimiterError::ZeroBurst)));
    }

    #[test]
    fn drops_when_overflow_drop() {
        let mut lb = LeakyBucket::new(&cfg(0.01, 5), LeakyBucketOverflow::Drop).unwrap();
        for _ in 0..5 {
            assert!(lb.try_acquire(1));
        }
        assert!(!lb.try_acquire(1));
    }

    #[test]
    fn drips_over_time_makes_room() {
        let mut lb = LeakyBucket::new(&cfg(100.0, 5), LeakyBucketOverflow::Drop).unwrap();
        for _ in 0..5 {
            assert!(lb.try_acquire(1));
        }
        assert!(!lb.try_acquire(1));
        std::thread::sleep(Duration::from_millis(30)); // 30ms * 100/s = 3 leaked
        assert!(lb.try_acquire(1));
    }

    #[test]
    fn block_policy_eventually_succeeds() {
        // 不用 .await 测试, 用 try_acquire_with_wait 同步阻塞
        let mut lb = LeakyBucket::new(&cfg(100.0, 2), LeakyBucketOverflow::Block).unwrap();
        assert!(lb.try_acquire(2));
        // 桶满, 但 drips at 100/s = 1 every 10ms
        let r = lb.try_acquire_with_wait(1, Duration::from_millis(100));
        assert!(matches!(r, Ok(true)));
    }

    #[test]
    fn reset_clears_level() {
        let mut lb = LeakyBucket::new(&cfg(0.01, 5), LeakyBucketOverflow::Drop).unwrap();
        for _ in 0..5 {
            let _ = lb.try_acquire(1);
        }
        assert!(!lb.try_acquire(1));
        lb.reset();
        assert!(lb.try_acquire(5));
    }

    #[test]
    fn release_drains_level() {
        let mut lb = LeakyBucket::new(&cfg(0.01, 10), LeakyBucketOverflow::Drop).unwrap();
        for _ in 0..5 {
            let _ = lb.try_acquire(1);
        }
        lb.release(3);
        let l = lb.current_level();
        assert!(l <= 2.0 + 0.1);
    }
}
