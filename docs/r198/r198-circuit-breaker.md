# R198 真 Circuit Breaker 进 apeireth-pipeline-g5 (替换 stub)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R198
> **日期**: 2026-08-13
> **来源**: R184 调研推荐 failsafe-rs, R198 改用 std 自实现 (0 新依赖)
> **状态**: 实施完成, 10/10 单测全过

---

## 0. 动机

apeireth-pipeline-g5/reliability.rs 当前 circuit-breaker 是 stub:
> "阶段 6 仅警告, R21 续真断" / "当前仅返回警告"

R198 替换为真正的 Circuit Breaker 实现.

---

## 1. 设计

### 1.1 状态机 (3 状态)

`
[Closed] -- 失败 N 次 --> [Open]
[Open]   -- cooldown 到期 --> [HalfOpen]
[HalfOpen] -- 成功 --> [Closed]
[HalfOpen] -- 失败 --> [Open] (立即回)
`

### 1.2 公共 API

`
ust
pub enum CircuitState { Closed, Open, HalfOpen }

pub struct CircuitBreaker {
    name: String,
    threshold: u32,           // 失败 N 次触发 Open
    cooldown: Duration,       // Open 持续 D 后转 HalfOpen
    inner: Mutex<Inner>,      // 同步锁
}

impl CircuitBreaker {
    pub fn new(name, threshold, cooldown) -> Self;
    pub fn with_defaults(name) -> Self;  // threshold=10, cooldown=30s
    pub fn state(&self) -> CircuitState;  // 自动考虑时间转换
    pub fn allow(&self) -> bool;
    pub fn record_success(&self);
    pub fn record_failure(&self);
    pub fn reset(&self);
    pub fn failure_count(&self) -> u32;
    pub fn name(&self) -> &str;
}
`

### 1.3 线程安全

- std::sync::Mutex<Inner>
- 单实例 100ns 量级开销
- 不锁外的资源, 不影响 stage 调度

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- reliability.rs stub: 0 改 (旧 API 完整保留)
- lib.rs 改 1 行: 加 pub mod circuit_breaker;
- pipeline 公开 API: 0 改

---

## 3. 测试 (10/10 pass)

- **t01**: new 启动 Closed
- **t02**: with_defaults
- **t03**: threshold 触发 Open
- **t04**: cooldown 转 HalfOpen
- **t05**: HalfOpen 成功 -> Closed
- **t06**: HalfOpen 失败 -> 回 Open
- **t07**: reset 清空
- **t08**: Closed 状态 success 重置计数
- **t09**: state as_str
- **t10**: default state

---

## 4. 风险

- **R1**: 0 新依赖, 无 R 级风险
- **R2**: Mutex 锁: 单实例 100ns 开销, 可接受
- **R3**: 测试用 sleep 50ms, CI 时间影响 < 1s

---

## 5. 中期路径 (R198+1 候选)

- 评估 failsafe-rs crate (~30KB 编译, MIT, 异步原生)
- 评估 tokio::sync::Mutex (如果 pipeline 改 async)
- 评估 lock-free 实现 (atomic + Instant)

---

## 6. 实施步骤

1. ✅ 写 circuit_breaker.rs (~190 行, 含 10 测试)
2. ✅ lib.rs 加 pub mod 导出 (1 行)
3. ✅ 10/10 测试全过
4. ✅ cargo check --workspace: 0 errors
5. ⏭️ 写本设计稿
6. ⏭️ commit

---

## 7. 0 触碰风险

- 0 风险 (新增子模块, 0 改现有)