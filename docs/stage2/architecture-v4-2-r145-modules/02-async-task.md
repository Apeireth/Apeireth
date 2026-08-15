# 2. 异步任务推送 — S-3 质量工程化

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/02-async-task.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`AsyncTaskStore` 任务状态机: `Pending → Running → (Completed | Failed | Cancelled)`.

## 借鉴 vs 上升

| VCP | 我们 |
|---|---|
| task_id 字符串, 状态分散 | `TaskId` u64, 状态机 |
| 启动后无状态查询 | `get(task_id)` 任意时刻 |
| 长任务无进度 | `update_progress(task_id, 0.0..1.0)` |
| 失败无原因 | `fail(task_id, error)` 详情 |
| 取消无竞争检测 | `cancel` 拒绝重复 / 拒绝 terminal |

## 哲学基础

**S-3 质量工程化**: 异步不是 fire-and-forget, 状态必须可验证. 5 状态机 + 进度 + 取消 = 工程化的"质量 = 状态可控".
**O-3 干到底**: 跨进程 / 跨重启的 task 重连, 不假装"已经发了".

## 6 类 `ToolKind` 集成

VCP 6 类插件协议 (Sync / Async / Static / Service / MessagePreprocessor / Hybridservice) 中:
- `Async` → 走本 task store
- `Service` → 长生命周期, 不走 task store
- `Hybridservice` → 复合, sync 部分 + async 部分

## 局限性

- 内存存储, 重启丢 (R146+ 接 SQLite)
- 单进程, 跨进程需要 L3/L4 bus 同步

## 内部参考

- 实现: [`crates/apeireth-tool-registry/src/async_task.rs`](../../crates/apeireth-tool-registry/src/async_task.rs)
- 6 类 enum: [`crates/apeireth-tool-registry/src/types.rs`](../../crates/apeireth-tool-registry/src/types.rs)
- 索引: [`README.md`](README.md)
