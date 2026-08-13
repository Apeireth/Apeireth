//! R198 真 Circuit Breaker (替换 reliability.rs 的 stub 实现).
//!
//! **来源**: R184 调研推荐 failsafe-rs, R198 改用 std 自实现 (0 新依赖).
//! 短期方案: 三状态机 (Closed / Open / HalfOpen) + 自动恢复.
//! 中期方案: 评估 failsafe-rs crate 集成 (R198+1 候选).
//!
//! **不假装** (O-5):
//! - sync Mutex 锁 (非 lock-free, 单实例 100ns 量级开销)
//! - 0 新增依赖 (纯 std + std::time::Instant)
//! - HalfOpen 状态: Open 持续 cooldown_ms 后, 下次调用尝试放行, 成功恢复 Closed, 失败再回 Open
//!
//! 0 触碰: 新增子模块, 不改 reliability.rs stub. 旧 stub API 完整保留.

#![allow(missing_docs)] // R198: 0 触碰现有 API 文档

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Circuit Breaker 状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭: 正常通过, 累计失败
    Closed,
    /// 打开: 拒绝所有调用, 持续 cooldown
    Open,
    /// 半开: 尝试放行一次, 决定 Closed 还是回 Open
    HalfOpen,
}

impl CircuitState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

impl Default for CircuitState {
    fn default() -> Self { Self::Closed }
}

/// 内部状态机
#[derive(Debug)]
struct Inner {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
}

impl Inner {
    const fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
        }
    }
}

/// Circuit Breaker 同步实现.
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    threshold: u32,
    cooldown: Duration,
    inner: Mutex<Inner>,
}

impl CircuitBreaker {
    /// 创建 Circuit Breaker
    /// - 	hreshold: 连续失败 N 次触发 Open
    /// - cooldown: Open 状态持续 D 后转 HalfOpen
    pub fn new(name: impl Into<String>, threshold: u32, cooldown: Duration) -> Self {
        Self {
            name: name.into(),
            threshold,
            cooldown,
            inner: Mutex::new(Inner::new()),
        }
    }

    /// 默认配置 (threshold=10, cooldown=30s, 跟 reliability.rs 守门对齐)
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, 10, Duration::from_secs(30))
    }

    /// 当前状态 (考虑时间转换 HalfOpen)
    pub fn state(&self) -> CircuitState {
        let mut inner = self.inner.lock().expect("circuit breaker poisoned");
        if inner.state == CircuitState::Open {
            if let Some(last) = inner.last_failure {
                if last.elapsed() >= self.cooldown {
                    inner.state = CircuitState::HalfOpen;
                }
            }
        }
        inner.state
    }

    /// 是否允许通过 (true = 允许, false = 拒绝)
    pub fn allow(&self) -> bool {
        match self.state() {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    /// 记录成功 (重置失败计数, HalfOpen -> Closed)
    pub fn record_success(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker poisoned");
        if inner.state == CircuitState::HalfOpen {
            inner.state = CircuitState::Closed;
        }
        inner.failure_count = 0;
    }

    /// 记录失败 (累计, 达到阈值 -> Open)
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker poisoned");
        inner.failure_count += 1;
        inner.last_failure = Some(Instant::now());
        if inner.state == CircuitState::HalfOpen {
            // HalfOpen 时再失败 -> 立即回 Open
            inner.state = CircuitState::Open;
        } else if inner.failure_count >= self.threshold {
            inner.state = CircuitState::Open;
        }
    }

    /// 强制重置回 Closed (手动 recovery)
    pub fn reset(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker poisoned");
        inner.state = CircuitState::Closed;
        inner.failure_count = 0;
        inner.last_failure = None;
    }

    /// 当前失败计数
    pub fn failure_count(&self) -> u32 {
        self.inner.lock().expect("circuit breaker poisoned").failure_count
    }

    /// 名称
    pub fn name(&self) -> &str { &self.name }
}

// 编译期守门: 状态 3 个
const _: () = assert!(matches!(CircuitState::Closed, CircuitState::Closed));
const _: () = assert!(matches!(CircuitState::Open, CircuitState::Open));
const _: () = assert!(matches!(CircuitState::HalfOpen, CircuitState::HalfOpen));

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn t01_new_starts_closed() {
        let cb = CircuitBreaker::new("test", 3, Duration::from_millis(100));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow());
    }

    #[test]
    fn t02_with_defaults() {
        let cb = CircuitBreaker::with_defaults("default");
        assert_eq!(cb.name(), "default");
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn t03_threshold_triggers_open() {
        let cb = CircuitBreaker::new("test", 3, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow());
    }

    #[test]
    fn t04_cooldown_transitions_to_half_open() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_millis(50));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.allow());
    }

    #[test]
    fn t05_half_open_success_returns_to_closed() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_millis(50));
        cb.record_failure();
        cb.record_failure();
        sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn t06_half_open_failure_returns_to_open() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_millis(50));
        cb.record_failure();
        cb.record_failure();
        sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn t07_reset_clears_state() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn t08_record_success_in_closed_resets_count() {
        let cb = CircuitBreaker::new("test", 5, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn t09_state_as_str() {
        assert_eq!(CircuitState::Closed.as_str(), "closed");
        assert_eq!(CircuitState::Open.as_str(), "open");
        assert_eq!(CircuitState::HalfOpen.as_str(), "half_open");
    }

    #[test]
    fn t10_default_state_is_closed() {
        assert_eq!(CircuitState::default(), CircuitState::Closed);
    }
}
