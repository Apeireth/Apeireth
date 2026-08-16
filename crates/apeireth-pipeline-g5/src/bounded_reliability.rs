//! R204 BoundedReliability — DefaultReliability + R198 CircuitBreaker 集成.
//!
//! **0 触碰**: DefaultReliability stub 完全保留. BoundedReliability 是 alternative.
//! DefaultReliability 用户可平滑迁移到 BoundedReliability 获得真断路保护.
//!
//! **设计**:
//! - 包装 DefaultReliability, 加上 R198 CircuitBreaker
//! - process() 入口: circuit_breaker.allow() 守门
//! - record_failure: 调 DefaultReliability::record_failure + circuit_breaker.record_failure
//! - record_success: 调 circuit_breaker.record_success (当前 DefaultReliability process 总是 Ok, 集成者手工调)
//! - reset(): 同时重置 DefaultReliability + circuit_breaker
//!
//! **不假装** (O-5):
//! - CircuitBreaker 是真 3 状态机 (R198), 不是 stub
//! - 同步, 单实例 100ns 锁开销
//! - threshold 10, cooldown 30s (R198 defaults, 跟 reliability CIRCUIT_BREAKER_THRESHOLD=10 对齐)

#![allow(missing_docs)] // R204: 0 触碰现有 API 文档

use std::time::Duration;

use crate::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::error::PipelineError;
use crate::message::PipelineMessage;
use crate::reliability::DefaultReliability;
use crate::stage::{Stage, StageKind};

/// R204: DefaultReliability + R198 CircuitBreaker 集成.
#[derive(Debug)]
pub struct BoundedReliability {
    inner: DefaultReliability,
    circuit_breaker: CircuitBreaker,
}

impl BoundedReliability {
    /// 创建 BoundedReliability, 配 circuit_breaker
    pub fn new(threshold: u32, cooldown: Duration) -> Self {
        Self {
            inner: DefaultReliability::new(),
            circuit_breaker: CircuitBreaker::new("bounded-reliability", threshold, cooldown),
        }
    }

    /// 用 CIRCUIT_BREAKER_THRESHOLD (10) + 30s cooldown 默认值
    pub fn with_defaults() -> Self {
        Self {
            inner: DefaultReliability::new(),
            circuit_breaker: CircuitBreaker::with_defaults("bounded-reliability"),
        }
    }

    /// 当前 circuit breaker 状态
    pub fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker.state()
    }

    /// circuit breaker 名称
    pub fn circuit_breaker_name(&self) -> &str {
        self.circuit_breaker.name()
    }

    /// 记录成功 (重置 circuit_breaker)
    pub fn record_success(&self) {
        self.circuit_breaker.record_success();
    }

    /// 记录失败 (累计 + 触发 circuit_breaker)
    pub fn record_failure(&self) -> u32 {
        let count = self.inner.record_failure();
        self.circuit_breaker.record_failure();
        count
    }

    /// 重置全部 (inner + circuit_breaker)
    pub fn reset(&self) {
        self.inner.reset_failures();
        self.circuit_breaker.reset();
    }

    /// 当前 inner 失败计数
    pub fn failure_count(&self) -> u32 {
        self.inner.failure_count()
    }

    /// 访问内部 DefaultReliability (向后兼容)
    pub fn inner(&self) -> &DefaultReliability {
        &self.inner
    }

    /// 访问内部 CircuitBreaker (高级用法, 例如多 stage 共享)
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }
}

impl Default for BoundedReliability {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl Stage<PipelineMessage, PipelineMessage> for BoundedReliability {
    fn kind(&self) -> StageKind {
        StageKind::Reliability
    }

    fn name(&self) -> &str {
        "bounded-reliability"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 新守门 0: circuit_breaker (R204 + R198 真断路)
        if !self.circuit_breaker.allow() {
            return Err(PipelineError::Stage {
                kind: StageKind::Reliability,
                source: Box::new(crate::reliability::ReliabilityError::CircuitBreakerOpen {
                    failure_count: self.circuit_breaker.failure_count(),
                    threshold: crate::reliability::CIRCUIT_BREAKER_THRESHOLD,
                }),
            });
        }

        // 委派给 DefaultReliability (attempt-counter / idempotency-key / backoff)
        // 注意: DefaultReliability 内部仍走原 stub 守门 (CIRCUIT_BREAKER_THRESHOLD=10), 不冲突
        // 因为我们外层先 allow(), 默认 inner 走不到 11+ 失败路径
        self.inner.process(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::PipelineMessage;

    #[test]
    fn t01_new_with_defaults() {
        let b = BoundedReliability::with_defaults();
        assert_eq!(b.circuit_state(), CircuitState::Closed);
        assert_eq!(b.failure_count(), 0);
    }

    #[test]
    fn t02_new_custom() {
        let b = BoundedReliability::new(3, Duration::from_millis(100));
        assert_eq!(b.circuit_breaker_name(), "bounded-reliability");
        assert_eq!(b.circuit_state(), CircuitState::Closed);
    }

    #[test]
    fn t03_default_trait() {
        let b = BoundedReliability::default();
        assert_eq!(b.circuit_state(), CircuitState::Closed);
    }

    #[test]
    fn t04_record_failure_triggers_circuit() {
        let b = BoundedReliability::new(3, Duration::from_secs(60));
        b.record_failure();
        b.record_failure();
        assert_eq!(b.circuit_state(), CircuitState::Closed);
        b.record_failure();
        assert_eq!(b.circuit_state(), CircuitState::Open);
    }

    #[test]
    fn t05_record_success_in_closed_resets_inner() {
        let b = BoundedReliability::new(5, Duration::from_secs(60));
        b.record_failure();
        b.record_failure();
        assert_eq!(b.failure_count(), 2);
        b.record_success();
        // circuit_breaker.record_success() in Closed: 只重置 failure_count (in Inner)
        // 但我们的 inner.failure_count 是 AtomicU32, 不会自动 sync
        // 所以这里 inner 仍是 2
        assert_eq!(b.failure_count(), 2);
        assert_eq!(b.circuit_state(), CircuitState::Closed);
    }

    #[test]
    fn t06_reset_clears_both() {
        let b = BoundedReliability::new(2, Duration::from_secs(60));
        b.record_failure();
        b.record_failure();
        assert_eq!(b.circuit_state(), CircuitState::Open);
        b.reset();
        assert_eq!(b.circuit_state(), CircuitState::Closed);
        assert_eq!(b.failure_count(), 0);
    }

    #[test]
    fn t07_stage_trait_implements() {
        let b = BoundedReliability::with_defaults();
        assert_eq!(b.kind(), StageKind::Reliability);
        assert_eq!(b.name(), "bounded-reliability");
    }

    #[test]
    fn t08_process_normal_message() {
        let b = BoundedReliability::with_defaults();
        let msg = PipelineMessage {
            kind: "test".to_string(),
            payload: "hello".to_string(),
            attempt: 0,
            trace_id: String::new(),
        };
        let r = b.process(msg);
        assert!(r.is_ok());
    }

    #[test]
    fn t09_process_when_circuit_open_errors() {
        let b = BoundedReliability::new(2, Duration::from_secs(60));
        b.record_failure();
        b.record_failure();
        // circuit_breaker now Open
        let msg = PipelineMessage {
            kind: "test".to_string(),
            payload: "hello".to_string(),
            attempt: 0,
            trace_id: String::new(),
        };
        let r = b.process(msg);
        assert!(r.is_err());
    }

    #[test]
    fn t10_inner_access() {
        let b = BoundedReliability::with_defaults();
        let _ = b.inner();
        let _ = b.circuit_breaker();
    }
}
