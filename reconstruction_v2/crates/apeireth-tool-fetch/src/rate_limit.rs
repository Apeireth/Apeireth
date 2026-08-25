//! **R230 — apeireth-tool-fetch 速率限制 (per-host sliding window)**
//!
//! **设计**: 借鉴 token bucket + sliding window, 0 引外部 dep.
//! - 每个 host 独立计数 (HashMap<host, VecDeque<Instant>>)
//! - `check(host)` — 当前时间窗口内请求数 < limit 返 true, 否则 false
//! - `wait_time(host)` — 距离下一次允许的时间 (用于 backoff)
//! - `record(host)` — 记录一次请求
//! - `clear()` — 清空 (测试用)
//!
//! **不假装**: 0 引 external crate (governor 等). std + VecDeque + Instant.
//!   单进程内存计数, 跨进程不共享 (per-host 真实限速应放在 HTTP client 层面).

#![allow(missing_docs)]

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// **Per-host rate limiter** (sliding window)
pub struct RateLimiter {
    /// 最大请求数 / window
    max_requests: usize,
    /// 窗口时长
    window: Duration,
    /// host → 请求时间戳 deque (按时间升序)
    history: HashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    /// 默认构造 — 60 请求 / 60 秒
    pub fn new() -> Self {
        Self::with_limit(60, Duration::from_secs(60))
    }

    /// 自定义 (max_requests / window)
    pub fn with_limit(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            history: HashMap::new(),
        }
    }

    /// **check** — 此刻 host 是否允许请求 (true = 允许, false = 已超)
    pub fn check(&self, host: &str) -> bool {
        let now = Instant::now();
        match self.history.get(host) {
            None => true,
            Some(dq) => {
                // 清除窗口外的旧记录 (sliding window)
                let active = dq
                    .iter()
                    .filter(|&&t| now.duration_since(t) <= self.window)
                    .count();
                active < self.max_requests
            }
        }
    }

    /// **record** — 记录一次请求
    pub fn record(&mut self, host: &str) {
        let now = Instant::now();
        let dq = self
            .history
            .entry(host.to_string())
            .or_insert_with(VecDeque::new);
        // 清掉窗口外的
        while let Some(&front) = dq.front() {
            if now.duration_since(front) > self.window {
                dq.pop_front();
            } else {
                break;
            }
        }
        dq.push_back(now);
    }

    /// **wait_time** — 距离下次允许的等待时长 (None = 现在可发)
    pub fn wait_time(&self, host: &str) -> Option<Duration> {
        let now = Instant::now();
        let dq = self.history.get(host)?;
        if dq.len() < self.max_requests {
            return None;
        }
        // 最早一条记录 + window = 何时腾出位置
        let oldest = dq.front()?;
        let elapsed = now.duration_since(*oldest);
        if elapsed >= self.window {
            None
        } else {
            Some(self.window - elapsed)
        }
    }

    /// **hosts** — 当前跟踪的 host 数
    pub fn hosts(&self) -> usize {
        self.history.len()
    }

    /// **count** — 当前 host 在窗口内的请求数
    pub fn count(&self, host: &str) -> usize {
        let now = Instant::now();
        match self.history.get(host) {
            None => 0,
            Some(dq) => dq
                .iter()
                .filter(|&&t| now.duration_since(t) <= self.window)
                .count(),
        }
    }

    /// **clear** — 清空所有 host 计数
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// **clear_host** — 清空单个 host
    pub fn clear_host(&mut self, host: &str) -> bool {
        self.history.remove(host).is_some()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// **共享 RateLimiter 工厂** — 用 Arc<Mutex<RateLimiter>>
pub fn shared_rate_limiter() -> std::sync::Arc<std::sync::Mutex<RateLimiter>> {
    std::sync::Arc::new(std::sync::Mutex::new(RateLimiter::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_limiter_empty() {
        let rl = RateLimiter::new();
        assert!(rl.check("example.com"));
        assert_eq!(rl.count("example.com"), 0);
        assert_eq!(rl.hosts(), 0);
    }

    #[test]
    fn record_and_check() {
        let mut rl = RateLimiter::with_limit(3, Duration::from_secs(60));
        rl.record("a.com");
        rl.record("a.com");
        rl.record("a.com");
        assert_eq!(rl.count("a.com"), 3);
        assert!(!rl.check("a.com"), "第 4 个应被拒");
    }

    #[test]
    fn different_hosts_independent() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_secs(60));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        assert!(rl.check("b.com"), "不同 host 独立");
    }

    #[test]
    fn wait_time_none_when_allowed() {
        let mut rl = RateLimiter::with_limit(2, Duration::from_secs(60));
        rl.record("a.com");
        assert_eq!(rl.wait_time("a.com"), None);
    }

    #[test]
    fn wait_time_when_at_limit() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_millis(100));
        rl.record("a.com");
        let w = rl.wait_time("a.com");
        assert!(w.is_some());
        assert!(w.unwrap() <= Duration::from_millis(100));
    }

    #[test]
    fn sliding_window_expires() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_millis(50));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        std::thread::sleep(Duration::from_millis(60));
        assert!(rl.check("a.com"), "50ms 后应过期");
    }

    #[test]
    fn clear_resets() {
        let mut rl = RateLimiter::with_limit(1, Duration::from_secs(60));
        rl.record("a.com");
        assert!(!rl.check("a.com"));
        rl.clear();
        assert!(rl.check("a.com"));
        assert_eq!(rl.hosts(), 0);
    }

    #[test]
    fn clear_host_removes_single() {
        let mut rl = RateLimiter::with_limit(2, Duration::from_secs(60));
        rl.record("a.com");
        rl.record("a.com");
        rl.record("b.com");
        assert_eq!(rl.hosts(), 2);
        assert!(rl.clear_host("a.com"));
        assert_eq!(rl.hosts(), 1);
        // a 已清, 可重新发; b 仍限速但还有 1 个名额
        assert!(rl.check("a.com"), "a.com clear 后应可发");
        assert!(rl.check("b.com"), "b.com 还没超限 (limit=2, 已用 1)");
    }

    #[test]
    fn max_requests_constant() {
        let rl = RateLimiter::with_limit(42, Duration::from_secs(60));
        assert_eq!(rl.max_requests, 42);
        assert_eq!(rl.window, Duration::from_secs(60));
    }
}
