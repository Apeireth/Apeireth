# R131.7 pipeline vs pipeline-g5 重复 audit

> 2026-08-12 R131.7 — 两个 crate 名字都含 "pipeline", 阶段数都是 5, **是否真重复?**

## 1. 维度对比

| 维度 | `apeireth-pipeline` | `apeireth-pipeline-g5` |
|---|---|---|
| 阶段数 | 5 步 | 5 阶段 |
| 阶段名 | 解析 placeholder → token 预算 → Force-Translate → 协议归一 → HTTP | Dispatch → Normalize → Policy → Reliability → Throttle |
| 核心抽象 | 5 步处理流 (输入消息 → HTTP 响应) | `Pipeline<T, I, O>` 通用框架 (任意 T, I, O) |
| 用途 | chat 主路径 (实际生产) | 通用执行框架 (13 集成测试) |
| 借鉴源 | VCP `chatCompletionHandler.js` | Golutra v0.1.0 `chat_db/pipeline` |
| 模块数 | 11 | 9 |
| 代码量 | 217KB | 81KB |
| 生产调用 | 被 `apeireth-api/serve.rs` 真调 (R17 战役 1-3) | 0 生产调用 (纯框架, 13 unit test 覆盖) |
| Pipeline 类型 | 无显式 Pipeline<T, I, O> generic | `Pipeline<T, I, O>` 是核心 API |

## 2. 命名相似度分析

**为什么阶段数都是 5 是巧合**:
- pipeline 借鉴 VCP 5 步 chat 模式 (实战经验)
- pipeline-g5 借鉴 Golutra 5 阶段 (通用化抽象)
- 阶段名完全不一样 (解析 placeholder ≠ Dispatch)
- 抽象层级不一样 (处理流 vs 通用框架)

**真正的"5 阶段"重叠度**:
- pipeline 的 "协议归一" ≈ pipeline-g5 的 "Normalize" (语义相近)
- pipeline 的 "HTTP 调用" ≈ pipeline-g5 的 "Dispatch" (都是发起外部动作)
- 但其余 3 阶段完全不同 (pipeline 的 token 预算/Force-Translate vs pipeline-g5 的 Policy/Reliability/Throttle)

## 3. 是否真重复?

**结论: 不是真重复, 是设计意图不同**:
- pipeline = **chat 专用处理流**, 已经被 api/serve 真接, **不能删**
- pipeline-g5 = **通用执行框架**, Pipeline<T, I, O> 可被任意业务复用, **不能删**

## 4. 互引用情况

- pipeline 0 import pipeline-g5
- pipeline-g5 0 import pipeline
- 两者完全独立, 0 协作
- 意味着: pipeline-g5 的 Pipeline<T, I, O> **理论上**可以包装 pipeline 的 chat 处理流为 `ChatPipeline`, 但目前**0 这么用**

## 5. R132-R133 续填建议

### Option A: 不动 (推荐)

两者设计意图不同, 强行合并会破坏 pipeline 的生产稳定性 + pipeline-g5 的通用抽象。

### Option B: 整合示例 (1 周工作量)

写 `apeireth-pipeline/src/g5_bridge.rs` 把 pipeline 的 5 步封装为 `pipeline_g5::Pipeline<ChatPipeline, ChatInput, ChatOutput>` specialization, 展示两者协作模式。但**不强制替换**——serve 仍可用 raw pipeline。

### Option C: 合并 (不推荐)

把 pipeline 5 步改为 Pipeline<T, I, O> generic, 风险高 (改 217KB 生产代码), 收益低 (chat 处理流跟通用框架的语义边界已模糊)。

## 6. 决策

**R131.7 决策**: 选 Option A — **不合并, 0 改**. 理由:
- pipeline 是生产路径, 改 = risk
- pipeline-g5 是通用抽象, 改 = 破坏通用性
- 两者并行存在 = 设计分层, 跟 VCP / Golutra 借鉴源同构
- R132+ 真有需求时再 Option B (写整合示例)

## 7. 当前状态 (无改动)

```
apeireth-pipeline:    11 modules, 217KB, 生产路径, 真接 minimaxi
apeireth-pipeline-g5:  9 modules,  81KB, 通用框架, 13 集成测试
两者独立存在, 0 互引用
```
