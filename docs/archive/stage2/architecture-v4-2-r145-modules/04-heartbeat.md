# 4. AI 自驱心跳 — O-3 干到底

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/04-heartbeat.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`HeartbeatScheduler` 后台 tick 循环:
- 5 类 `WakeupSource`: `Time` / `Event` / `Agent` / `User` / `Async`
- 5 类 `HeartbeatPriority`: `Background` / `Low` / `Normal` / `High` / `Critical`
- `Schedule { interval, jitter }` 周期 + 抖动

## 为什么 5 类 WakeupSource

VCP OneRing 偏"时间驱动", 偏"持续意识". 我们扩到 5 类:
- Time: 周期性主动调研
- Event: 响应 bus 事件
- Agent: 跨 Agent 协作
- User: 用户主动触发
- Async: 异步任务完成

## 哲学基础

**O-3 干到底**: 主动循环 ≠ 反应循环. agent 不能单纯等用户发指令, 必须有自驱 heartbeat.
**S-1 北极星**: heartbeat 的目的是"自演化", 不是"被动服务".

## 反对意见

心跳过密 = 浪费计算. 我们用 `decay_rate` + `priority` 控制 — 不是每 tick 都跑全量.

## 局限性

- 单调度器, 跨进程需要分布式时钟
- priority 仅调度顺序, 不能抢占正在跑的 heartbeat

## 借鉴

VCP v1.1 "AI 自己决定下一次心跳 (OneRing + FlowInvite)".

## 内部参考

- 实现: [`crates/apeireth-supervisor/src/heartbeat.rs`](../../crates/apeireth-supervisor/src/heartbeat.rs)
- 监督树: [`crates/apeireth-supervisor/src/supervisor.rs`](../../crates/apeireth-supervisor/src/supervisor.rs)
- 索引: [`README.md`](README.md)
