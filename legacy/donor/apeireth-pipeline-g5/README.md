# apeireth-pipeline-g5

> R20 阶段 6 估补 → R21+ 重建 — 5 阶段 pipeline 框架 (Dispatch / Normalize / Policy / Reliability / Throttle)

apeireth-pipeline-g5 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 状态: 已实装 ✅ (非 placeholder)

之前 README 标 "R21+ 重建", 但 R21 G-2 续补已完成:
- 9 src 模块真实实现 (dispatch / normalize / policy / reliability / throttle / stage / error / message / pipeline + bounded_reliability + circuit_breaker)
- 13 集成测试 (`tests/pipeline_chain.rs`)
- 10 organ_kani_proofs (`src/organ_kani_proofs.rs`)
- 编译期 hardcode 守门 17+ (per K-1 强校验, `STAGE_KIND_COUNT == 5` 等)

## 5 阶段 (per Golutra v0.1.0 chat_db/pipeline 1:1 翻译)

```
Pipeline<I, O>
   ↓
[0] Dispatch      — 入口路由, 决定下一步去向
   ↓
[1] Normalize    — 输入规范化 (JSON / 类型 / schema)
   ↓
[2] Policy       — 鉴权 / 权限校验 / 风控
   ↓
[3] Reliability  — 重试 / 幂等 / Circuit Breaker
   ↓
[4] Throttle     — QPS / 并发限流 / Token Bucket
   ↓
output
```

通用化: 任何 model (chat / task / memory / MCP / ...) 都能挂上 `Pipeline<T, I, O>`.

## 模块

| 模块 | 职责 |
|---|---|
| `pipeline` | `Pipeline<T, I, O>` 主体, Stage enum, Config |
| `stage` | `Stage` trait, `StageKind` enum (5 阶段), `StageEntry` 注册表 |
| `dispatch` | `Dispatch` trait + `DefaultDispatch` |
| `normalize` | `Normalize` trait + `DefaultNormalize` |
| `policy` | `Policy` trait + `DefaultPolicy`, MAX_POLICY_* 常量 |
| `reliability` | `Reliability` trait + `DefaultReliability`, MAX_RETRY_ATTEMPTS / CIRCUIT_BREAKER_THRESHOLD |
| `bounded_reliability` | R204: 集成 CircuitBreaker 的 DefaultReliability (替换 reliability.rs 的 stub) |
| `circuit_breaker` | R198: 真 Circuit Breaker 实现 |
| `throttle` | `Throttle` trait + `DefaultThrottle`, MAX_QPS / MAX_BURST / TOKEN_BUCKET_REFILL_SECS |
| `error` | `PipelineError` + `PipelineErrorKind` (5 variants) |
| `message` | `PipelineMessage` 跨阶段传递结构 |

## 编译期常量 (守门 17+)

```rust
pub const STAGE_KIND_COUNT: usize = 5;       // 阶段数固定
pub const PIPELINE_MIN_STAGES: usize = 1;
pub const PIPELINE_MAX_STAGES: usize = 5;
pub const PIPELINE_STAGE_NAME_MAX_LEN: usize = 32;
pub const MAX_RETRY_ATTEMPTS: u8 = 5;
pub const RETRY_BACKOFF_MS: [u64; 4] = [100, 500, 2000, 5000];
pub const CIRCUIT_BREAKER_THRESHOLD: u32 = 10;
pub const IDEMPOTENCY_KEY_PREFIX: &str = "sandbox-";
pub const MAX_POLICY_ATTEMPTS: u32 = 3;
pub const MAX_POLICY_PAYLOAD_SIZE: usize = 1_048_576; // 1 MB
pub const MAX_QPS: u32 = 100;
pub const MAX_BURST: u32 = 200;
pub const MAX_CONCURRENT: u32 = 50;
pub const TOKEN_BUCKET_REFILL_SECS: u64 = 1;
pub const MAX_KIND_LEN: usize = 64;
pub const MAX_PAYLOAD_LEN: usize = 1_048_576;
pub const MAX_TRACE_ID_LEN: usize = 128;
pub const PIPELINE_ERROR_VARIANT_COUNT: usize = 5;
```

## 借鉴 & 后续

- **借鉴**: Golutra v0.1.0 `chat_db/pipeline` 5 阶段思想 (per BORROW_FROM_GOLUTRA.md §8 P2)
- **后续 (R21+)**: 真接 `apeireth-pipeline` (LOCKED 专用 chat pipeline), g5 作为通用框架并行
- **0 触碰 24 LOCKED crate** — g5 是通用框架, 不与 LOCKED `apeireth-pipeline` (chat 专用) 冲突

## 测试

- 13 集成测试 (`tests/pipeline_chain.rs`)
- 10 unit tests (`src/organ_kani_proofs.rs`)
- 总计: `cargo test -p apeireth-pipeline-g5` → 23+ passed

## 用法

```rust
use apeireth_pipeline_g5::{Pipeline, Stage, StageKind, PipelineMessage};

// 1. 构造 5 阶段 pipeline
let mut p = Pipeline::new("my_pipeline");
p.add_stage(StageKind::Dispatch, DefaultDispatch::new());
p.add_stage(StageKind::Normalize, DefaultNormalize::new());
p.add_stage(StageKind::Policy, DefaultPolicy::new());
p.add_stage(StageKind::Reliability, DefaultReliability::new());
p.add_stage(StageKind::Throttle, DefaultThrottle::new());

// 2. 跑消息
let msg = PipelineMessage::new("trace-001", b"hello".to_vec());
let result = p.run(msg)?;
assert!(result.is_ok());
```