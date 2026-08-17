# R204 BoundedReliability — DefaultReliability + R198 CircuitBreaker 集成

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R204
> **日期**: 2026-08-13
> **来源**: R184 调研 + R198 真 CircuitBreaker 实施
> **状态**: 实施完成, 21/21 单测全过 (累计)

---

## 0. 动机

R198 写了真 CircuitBreaker (3 状态机, std 实现, 0 新依赖). R204 把 CircuitBreaker 真正集成进 pipeline reliability stage, 让用户可选 BoundedReliability 替代 DefaultReliability 获得真断路保护.

---

## 1. 设计

### 1.1 BoundedReliability struct

`
ust
pub struct BoundedReliability {
    inner: DefaultReliability,    // 原有 stub
    circuit_breaker: CircuitBreaker,  // R198 真 3 状态
}

impl BoundedReliability {
    pub fn new(threshold: u32, cooldown: Duration) -> Self;
    pub fn with_defaults() -> Self;  // 10 / 30s, 跟 reliability CIRCUIT_BREAKER_THRESHOLD 对齐
    pub fn circuit_state(&self) -> CircuitState;
    pub fn record_success(&self);
    pub fn record_failure(&self) -> u32;  // 同步调 inner + circuit_breaker
    pub fn reset(&self);
    pub fn failure_count(&self) -> u32;
    pub fn inner(&self) -> &DefaultReliability;
    pub fn circuit_breaker(&self) -> &CircuitBreaker;
}
`

### 1.2 Stage trait impl

`
ust
impl Stage<PipelineMessage, PipelineMessage> for BoundedReliability {
    fn process(&self, input) -> Result<...> {
        // 守门 0: circuit_breaker.allow() (R204 + R198 真断路)
        if !self.circuit_breaker.allow() {
            return Err(CircuitBreakerOpen { ... });
        }
        // 委派给 DefaultReliability (attempt/idempotency/backoff)
        self.inner.process(input)
    }
}
`

### 1.3 0 触碰承诺

- DefaultReliability 公开 API: 0 改
- reliability.rs stub 注释 (R198 已说"留 R21 续真断"): 仍可用
- BoundedReliability 是 alternative, 不是 replacement

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- DefaultReliability 公开 API: 0 改
- reliability.rs stub: 0 改
- lib.rs 改 1 行: pub mod bounded_reliability

---

## 3. 测试 (10/10 pass, 累计 21/21)

- t01: with_defaults
- t02: new custom
- t03: Default trait
- t04: record_failure 触发 circuit
- t05: record_success 在 Closed
- t06: reset 清空 both
- t07: Stage trait 实现
- t08: process 正常 message
- t09: process circuit open 错误
- t10: inner + circuit_breaker 访问

---

## 4. 风险

- 0 新依赖
- DefaultReliability stub 行为不变 (向后兼容)
- BoundedReliability 是 additive (用户自选)

---

## 5. 中期路径 (R204+1 候选)

- integration_test: 5 阶段 pipeline 跑通, BoundedReliability 在 Reliability slot
- 替换 DefaultReliability (如果测试通过, R204+2)
- 评估多 stage 共享 circuit_breaker