//! `apeireth-rate-limiter` 滑动窗口算法。
//!
//! 4 参数: `window_size` / `slide_interval` / `max_requests` / `precision`。
//!
//! **算法**:
//! - `Log` 模式: 每次请求记时间戳到 `VecDeque<Instant>`, 过期弹出, O(requests) 内存, 精确
//! - `Counter` 模式: 把窗口切 N 个 sub-bucket（按 `slide_interval` 切分）, 每桶计数, O(N) 内存
//!
//! **解决边界突刺**: 滑窗连续滚动, 不会在窗口边界处双倍通过
//!
//! **8 项不修改承诺**: 同 token_bucket。

use crate::config::SlidingWindowPrecision;
use crate::error::{RateLimiterError, Result};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 滑动窗口 — 单 key 状态。
#[derive(Debug)]
pub struct SlidingWindow {
    /// 窗口大小。
    window_size: Duration,
    /// slide 步长（Counter 模式用, Log 模式可忽略）。
    slide_interval: Duration,
    /// 窗口内最大请求数。
    max_requests: u32,
    /// 精度模式。
    precision: SlidingWindowPrecision,
    /// Counter 模式的 epoch（构造时刻）— sub-bucket 对齐基准。
    epoch: Instant,

    // Log 模式状态
    log: VecDeque<Instant>,

    // Counter 模式状态: (sub_bucket_start, count) 按时间升序
    counter: VecDeque<SubBucket>,
}

/// Counter 模式的子桶。
#[derive(Debug, Clone, Copy)]
struct SubBucket {
    start: Instant,
    count: u32,
}

impl SlidingWindow {
    /// 新建滑动窗口。
    pub fn new(
        window_size: Duration,
        slide_interval: Duration,
        max_requests: u32,
        precision: SlidingWindowPrecision,
    ) -> Result<Self> {
        if window_size.is_zero() {
            return Err(RateLimiterError::ZeroWindowSize);
        }
        let slide = if slide_interval.is_zero() {
            // Counter 模式默认把窗口切 10 段
            window_size / 10
        } else {
            slide_interval
        };
        Ok(Self {
            window_size,
            slide_interval: slide,
            max_requests,
            precision,
            epoch: Instant::now(),
            log: VecDeque::new(),
            counter: VecDeque::new(),
        })
    }

    /// 非阻塞尝试占一个名额。
    pub fn try_acquire(&mut self) -> bool {
        match self.precision {
            SlidingWindowPrecision::Log => self.try_acquire_log(),
            SlidingWindowPrecision::Counter => self.try_acquire_counter(),
        }
    }

    /// 重置（清空所有计数）。
    pub fn reset(&mut self) {
        self.log.clear();
        self.counter.clear();
    }

    /// 当前窗口内请求数。
    pub fn current_count(&mut self) -> u32 {
        match self.precision {
            SlidingWindowPrecision::Log => {
                self.prune_log();
                self.log.len() as u32
            }
            SlidingWindowPrecision::Counter => {
                self.prune_counter();
                self.counter.iter().map(|b| b.count).sum()
            }
        }
    }

    /// 窗口内剩余名额。
    pub fn remaining(&mut self) -> u32 {
        self.max_requests.saturating_sub(self.current_count())
    }

    /// 窗口大小。
    pub fn window_size(&self) -> Duration {
        self.window_size
    }

    /// slide 步长。
    pub fn slide_interval(&self) -> Duration {
        self.slide_interval
    }

    // ---- Log 模式 ----

    fn try_acquire_log(&mut self) -> bool {
        self.prune_log();
        if (self.log.len() as u32) < self.max_requests {
            self.log.push_back(Instant::now());
            true
        } else {
            false
        }
    }

    fn prune_log(&mut self) {
        let cutoff = Instant::now() - self.window_size;
        while let Some(&front) = self.log.front() {
            if front < cutoff {
                self.log.pop_front();
            } else {
                break;
            }
        }
    }

    // ---- Counter 模式 ----

    fn try_acquire_counter(&mut self) -> bool {
        self.prune_counter();
        let total: u32 = self.counter.iter().map(|b| b.count).sum();
        if total < self.max_requests {
            let now = Instant::now();
            let bucket_start = self.bucket_start(now);
            // 找或建当前 sub-bucket
            if let Some(last) = self.counter.back_mut() {
                if last.start == bucket_start {
                    last.count += 1;
                    return true;
                }
            }
            self.counter.push_back(SubBucket {
                start: bucket_start,
                count: 1,
            });
            true
        } else {
            false
        }
    }

    fn prune_counter(&mut self) {
        let cutoff = Instant::now() - self.window_size;
        while let Some(&front) = self.counter.front() {
            if front.start < cutoff {
                self.counter.pop_front();
            } else {
                break;
            }
        }
    }

    fn bucket_start(&self, now: Instant) -> Instant {
        // 把 now 向下对齐到 slide_interval 边界（基于 epoch）
        let slide_nanos = self.slide_interval.as_nanos().max(1) as u64;
        let since_epoch = now.duration_since(self.epoch).as_nanos() as u64;
        let bucket_index = since_epoch / slide_nanos;
        self.epoch + Duration::from_nanos(bucket_index * slide_nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_zero_window() {
        let r = SlidingWindow::new(
            Duration::ZERO,
            Duration::from_millis(10),
            10,
            SlidingWindowPrecision::Log,
        );
        assert!(matches!(r, Err(RateLimiterError::ZeroWindowSize)));
    }

    #[test]
    fn log_mode_counts_within_window() {
        let mut sw = SlidingWindow::new(
            Duration::from_secs(10),
            Duration::ZERO,
            3,
            SlidingWindowPrecision::Log,
        )
        .unwrap();
        assert!(sw.try_acquire());
        assert!(sw.try_acquire());
        assert!(sw.try_acquire());
        assert!(!sw.try_acquire());
    }

    #[test]
    fn log_mode_drops_expired_entries() {
        let mut sw = SlidingWindow::new(
            Duration::from_millis(50),
            Duration::ZERO,
            2,
            SlidingWindowPrecision::Log,
        )
        .unwrap();
        assert!(sw.try_acquire());
        assert!(sw.try_acquire());
        assert!(!sw.try_acquire());
        std::thread::sleep(Duration::from_millis(60));
        assert!(sw.try_acquire());
    }

    #[test]
    fn counter_mode_counts_within_window() {
        let mut sw = SlidingWindow::new(
            Duration::from_secs(10),
            Duration::from_millis(100),
            3,
            SlidingWindowPrecision::Counter,
        )
        .unwrap();
        assert!(sw.try_acquire());
        assert!(sw.try_acquire());
        assert!(sw.try_acquire());
        assert!(!sw.try_acquire());
    }

    #[test]
    fn reset_clears_state() {
        let mut sw = SlidingWindow::new(
            Duration::from_secs(10),
            Duration::from_millis(10),
            2,
            SlidingWindowPrecision::Log,
        )
        .unwrap();
        let _ = sw.try_acquire();
        let _ = sw.try_acquire();
        assert!(!sw.try_acquire());
        sw.reset();
        assert!(sw.try_acquire());
    }

    #[test]
    fn remaining_decreases() {
        let mut sw = SlidingWindow::new(
            Duration::from_secs(10),
            Duration::from_millis(10),
            3,
            SlidingWindowPrecision::Log,
        )
        .unwrap();
        assert_eq!(sw.remaining(), 3);
        let _ = sw.try_acquire();
        assert_eq!(sw.remaining(), 2);
    }
}
