# 7. 跨 Agent 群聊 — O-1 安全优先

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/07-group-chat.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`GroupChat` 房间抽象:
- 3 角色: `Host` / `Agent` / `Observer`
- 3 调度: `RoundRobin` / `Free` / `HostDriven`
- 3 状态: `Open` / `Closed` / `Archived`
- `ChatMessage` 含 SHA-256 `content_hash` (与仲裁对齐)

## 借鉴 vs 上升

| VCP | 我们 |
|---|---|
| AgentAssistant 描述"围炉夜话", 治理模糊 | 3 角色 + 3 调度 + 3 状态 显式 |
| 群聊事件无统一存储 | content_hash 对齐 arbitration |
| IRC 风格管理 | RoundRobin / HostDriven 显式 |

## 哲学基础

**O-1 安全优先**: 群聊必须有治理, 否则 agent spam / loop / 攻陷.
**O-2 走在前人**: 3 角色映射 IRC/Discord 已知模式, 不重造.

## 治理规则

- `HostDriven` 模式下, 非 host 不能连发
- `RoomClosed` 拒绝任何 post
- `GroupChat` 事件全部 SHA-256 哈希, 可入 arbitration (per [`03-arbitration.md`](03-arbitration.md))
- `Observer` 角色只读, 不能 `post`

## 局限性

- 单进程房间, 跨进程需要仲裁层
- 0 持久化 (R146+ 接 SQLite)
- 0 加密 (私密对话 R146+ 考虑 e2ee)

## 借鉴

VCP v1.1 "AgentAssistant / VCPGroupChat 与其它 Agent 朋友们围炉夜话".

## 内部参考

- 实现: [`crates/apeireth-council/src/group_chat.rs`](../../crates/apeireth-council/src/group_chat.rs)
- 7 顾问: [`crates/apeireth-council/src/advisors/`](../../crates/apeireth-council/src/advisors/)
- 索引: [`README.md`](README.md)
