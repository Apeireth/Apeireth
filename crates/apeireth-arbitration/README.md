# apeireth-arbitration

**R145 VCP 终极差距补弱** — HASH-SQL 仲裁 + 唯一事实时间线

## 职责

跨前端 / 群聊 / 邮箱 / Agent 通讯统一通过本 crate 仲裁, 构建唯一事实时间线.

## 核心类型

- `ArbitrationLog` (SQLite): append-only 事件日志
- `ArbitrationEvent`: 事件 + SHA-256 content_hash
- `EventSource`: 6 类 (Frontend/GroupChat/Email/AgentComm/System/External)
- canonical order = `(timestamp_ms ASC, content_hash ASC, seq ASC)`

## 借鉴

VCP v1.1 "纯 HASH-SQL 仲裁所有前端/群聊/邮箱/Agent 间通讯, 构建唯一事实时间线".

## 上升

- 编译期 `EventSource::COUNT = 6` 守门
- SHA-256 Rust 标准库实现, 0 外部 PHP 依赖
- 真正的 SQLite WAL 持久化, drop+reopen 拿同一序列

## 跨模块

- `apeireth-council::group_chat::ChatMessage` 用同 SHA-256 字段
- `apeireth-tool-registry::async_task::TaskRecord` 可序列化进仲裁 log
- `apeireth-bus::ChanneledBus` 事件可路由进仲裁

## 0 假装

✅ SQLite 真持久化 (rusqlite 0.32 bundled) | ✅ SHA-256 真哈希 | ✅ 9 单元测试
