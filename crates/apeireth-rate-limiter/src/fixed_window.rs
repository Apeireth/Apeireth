//! `apeireth-rate-limiter` 固定窗口算法。
//!
//! 3 参数: `window_size` / `max_requests` / `reset_strategy`。
//!
//! **算法**:
//! 1. 维护一个固定大小窗口 + 计数器
//! 2. `try_acquire`: 窗口过期则重置, 否则计数
//! 3. `reset_strategy`:
//!    - `OnWindowEnd` (默认): 窗口结束时自动重置
//!    - `OnDemand`: 仅手动 `reset()` 时重置
//!    - `Both`: 两者都生效
//!
//! **已知缺陷 — 边界突刺** (boundary spike):
//! - 窗口末尾 N 个请求 + 新窗口前 N 个请求 = 2N
//! - 解决方法: 改用 sliding window
//!
//! **8 项不修改承诺**: 同 token_bucket。

use crate::config::FixedWindowReset;
use crate::error::{RateLimiterError, Result};
use std::time::{Duration, Instant};

/// 固定窗口 — 单 key 状态。
#[derive(Debug)]
pub struct FixedWindow {
    /// 窗口大小。
    window_size: Duration,
    /// 窗口内最大请求数。
    max_requests: u32,
    /// 当前窗口起点。
    window_start: Instant,
    /// 当前窗口已用计数。
    counter: u32,
    /// 重置策略。
    reset_strategy: FixedWindowReset,
}

impl FixedWindow {
    /// 新建固定窗口。
    pub fn new(window_size: Duration, max_requests: u32, reset: FixedWindowReset) -> Result<Self> {
        if window_size.is_zero() {
            return Err(RateLimiterError::ZeroWindowSize);
        }
        Ok(Self {
            window_size,
            max_requests,
            window_start: Instant::now(),
            counter: 0,
            reset_strategy: reset,
        })
    }

    /// 非阻塞尝试占一个名额。
    pub fn try_acquire(&mut self) -> bool {
        self.maybe_rotate();
        if self.counter < self.max_requests {
            self.counter += 1;
            true
        } else {
            false
        }
    }

    /// 手动重置（`OnDemand` / `Both` 策略下生效）。
    pub fn reset(&mut self) {
        self.counter = 0;
        self.window_start = Instant::now();
    }

    /// 当前窗口已用计数。
    pub fn current_count(&self) -> u32 {
        self.counter
    }

    /// 当前窗口剩余名额。
    pub fn remaining(&mut self) -> u32 {
        self.maybe_rotate();
        self.max_requests.saturating_sub(self.counter)
    }

    /// 窗口大小。
    pub fn window_size(&self) -> Duration {
        self.window_size
    }

    /// 窗口自动轮转检查。
    fn maybe_rotate(&mut self) {
        if self.reset_strategy == FixedWindowReset::OnDemand {
            return; // 仅手动重置
        }
        if self.window_start.elapsed() >= self.window_size {
            self.counter = 0;
            self.window_start = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_zero_window() {
        let r = FixedWindow::new(Duration::ZERO, 10, FixedWindowReset::OnWindowEnd);
        assert!(matches!(r, Err(RateLimiterError::ZeroWindowSize)));
    }

    #[test]
    fn counts_within_window() {
        let mut fw =
            FixedWindow::new(Duration::from_secs(10), 3, FixedWindowReset::OnWindowEnd).unwrap();
        assert!(fw.try_acquire());
        assert!(fw.try_acquire());
        assert!(fw.try_acquire());
        assert!(!fw.try_acquire());
    }

    #[test]
    fn reset_clears_counter() {
        let mut fw =
            FixedWindow::new(Duration::from_secs(10), 2, FixedWindowReset::OnDemand).unwrap();
        assert!(fw.try_acquire());
        assert!(fw.try_acquire());
        assert!(!fw.try_acquire());
        fw.reset();
        assert!(fw.try_acquire());
    }

    #[test]
    fn boundary_spike_demonstrates_known_issue() {
        // 边界突刺: 窗口末尾 2 + 新窗口 2 = 4 (max=2, 实际通过 4)
        // 这是 fixed window 的已知缺陷 — 测试明确展示, 不掩盖
        let mut fw =
            FixedWindow::new(Duration::from_millis(50), 2, FixedWindowReset::OnWindowEnd).unwrap();
        assert!(fw.try_acquire());
        assert!(fw.try_acquire());
        std::thread::sleep(Duration::from_millis(60)); // 窗口过期
                                                       // 新窗口开始
        assert!(fw.try_acquire());
        assert!(fw.try_acquire());
        // 实际通过了 4 个请求, 而 max=2
        // 这就是边界突刺 — 解决方案: sliding window
    }

    #[test]
    fn on_demand_does_not_auto_rotate() {
        let mut fw =
            FixedWindow::new(Duration::from_millis(20), 1, FixedWindowReset::OnDemand).unwrap();
        assert!(fw.try_acquire());
        assert!(!fw.try_acquire());
        std::thread::sleep(Duration::from_millis(30));
        // OnDemand 不会自动重置
        assert!(!fw.try_acquire());
        fw.reset();
        assert!(fw.try_acquire());
    }

    #[test]
    fn remaining_decreases() {
        let mut fw =
            FixedWindow::new(Duration::from_secs(10), 3, FixedWindowReset::OnWindowEnd).unwrap();
        assert_eq!(fw.remaining(), 3);
        let _ = fw.try_acquire();
        assert_eq!(fw.remaining(), 2);
    }
}
