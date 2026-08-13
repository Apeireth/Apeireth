# apeireth-pipeline-g5

> **R20 阶段 6 通用 5 阶段 pipeline 框架** — 借鉴 Golutra v0.1.0 `chat_db/pipeline` 5 阶段思想 (Dispatch → Normalize → Policy → Reliability → Throttle).
> **当前状态**: 9 src + 13 集成测试 + 17+ 编译期 hardcode 守门 (K-1 强校验).
> **真接**: 任何 pipeline 类型 (chat / task / memory / MCP) 都挂得上 `Pipeline<T, I, O>`.

---

## 5 阶段 (R21 G-2 真实 enum)

```
0 Dispatch → 1 Normalize → 2 Policy → 3 Reliability → 4 Throttle
```

每阶段都有 trait 抽象 + 编译期 hardcode 守门:

| 阶段 | trait | 守门常量 |
|---|---|---|
| 0 Dispatch | `dispatch::Dispatcher` | `STAGE_KIND_COUNT == 5` |
| 1 Normalize | `normalize::Normalizer` | `MAX_NORMALIZE_RETRIES == 3` |
| 2 Policy | `policy::Policy` | `MAX_POLICY_RULES == 64` |
| 3 Reliability | `reliability::Reliability` | `MAX_RETRY_ATTEMPTS == 5`, `RETRY_BACKOFF_MS [4 步]` |
| 4 Throttle | `throttle::Throttle` | `CIRCUIT_BREAKER_THRESHOLD == 10`, `IDEMPOTENCY_KEY_PREFIX == "sandbox-"` |

## 公共 API

- `pipeline::Pipeline<T, I, O>` — 通用 5 阶段 pipeline (T = marker type, I = input, O = output)
- `stage::Stage` — 5 阶段 enum
- `message::PipelineMessage` — pipeline 内消息类型
- `error::PipelineError` — pipeline 错误
- `reliability::RetryBackoff` — 4 步退避策略
- `reliability::CircuitBreaker` — circuit breaker (10 次阈值)

## 跑

```bash
cargo run -p apeireth-pipeline-g5 --example full_pipeline
# 跑 3 个示例输入: 正常 chat / spam (Policy 拒绝) / 超大 payload (Policy 拒绝 by size)

cargo test -p apeireth-pipeline-g5
# 13 集成测试: test_5_stage_chain_success, test_reliability_backoff, test_circuit_breaker, ...
```

## 跟 `apeireth-pipeline` 关系

- `apeireth-pipeline` — chat 专用管线 (VCP 借鉴 §6.2.2)
- `apeireth-pipeline-g5` — 通用 5 阶段框架 (任何类型)
- 两个不重复, 互补: pipeline 走 chat 业务, pipeline-g5 走通用编排

## 借鉴

- **Golutra v0.1.0** `chat_db/pipeline` 5 阶段思想
- License: 待补 (R20 阶段 6 估补, 借鉴 ID 还没正式登记)

## See also

- [Golutra 借鉴映射](../../reports/)
- [整合 #5.1 commit (src/)](https://github.com/apeireth/apeireth-rust/commit/3598d336)
## R166 public API deep cleanup

(R164/R166 cleanup affected upstream types; pipeline-g5 confirmed healthy post-cleanup).
