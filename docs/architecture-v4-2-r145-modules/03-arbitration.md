# 3. HASH-SQL 仲裁 — S-2 实事求是

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/03-arbitration.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`ArbitrationLog` SQLite append-only:
- `events` 表 (no UPDATE, INSERT only)
- 6 类 `EventSource`: `Frontend` / `GroupChat` / `Email` / `AgentComm` / `System` / `External`
- SHA-256 `content_hash`
- canonical order = `(timestamp_ms ASC, content_hash ASC, seq ASC)` 三元组

## 为什么 hash + SQL (非纯 hash chain / 非 vector clock)

VCP HASH-SQL 仲裁 = "用 hash 决次序, SQL 决存储". 我们上升:
- 物理时钟 + 哈希次序硬冲突 (不假装分布式共识)
- 6 类 EventSource 字段级对应 VCP 5 类 + 1 类 (External) 我们加
- 索引 (timestamp_ms, content_hash, source) 加速查询

## 哲学基础

**S-2 实事求是**: 唯一事实时间线 = "事实" 的最高要求.
**O-5 不假装**: 我们不假装"分布式共识" — 这是单进程 SQLite. 跨进程需要 raft/paxos, R146+ 考虑.

## 局限性

- 时钟回拨会导致同时间戳同 hash 冲突 (用 seq 兜底)
- 单进程 SQLite 不跨机器
- 跨 agent 群聊需要 IPC 同步

## 借鉴

VCP v1.1 "纯 HASH-SQL 仲裁所有前端/群聊/邮箱/Agent 间通讯, 构建唯一事实时间线".

## 内部参考

- 实现: [`crates/apeireth-arbitration/src/lib.rs`](../../crates/apeireth-arbitration/src/lib.rs)
- 群聊对齐: [`docs/architecture-v4-2-r145-modules/07-group-chat.md`](07-group-chat.md)
- 索引: [`README.md`](README.md)
