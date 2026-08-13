# R229 — apeireth-bus Event Log + Replay

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R229
> **日期**: 2026-08-13
> **状态**: 1 commit, 14 测试 +14, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱 + 一体化优美" + "继续全做完"

## 1. 设计

apeireth-bus R228 已落地 pattern subscribe. R229 加 append-only event log,
让任何时间点可以 replay 历史事件 (按 topic / 时间 / pattern 过滤).

### 1.1 EventLog<T>

**字段**:
- `capacity: usize` — 默认 1024, 满了循环覆盖最旧
- `inner: Mutex<Vec<LoggedEvent<T>>>` — append-only

**LoggedEvent<T>**:
```rust
pub struct LoggedEvent<T: Clone> {
    pub topic: String,
    pub timestamp_ms: i64,
    pub message: BusMessage<T>,
}
```

**9 工具方法**:
- `len` / `is_empty` / `capacity`
- `append(event)` — 满了 pop_front + push_back
- `replay_topic(topic) -> Vec<LoggedEvent<T>>`
- `replay_since(since_ms) -> Vec<LoggedEvent<T>>`
- `replay_pattern(pattern) -> Vec<LoggedEvent<T>>` — 复用 `crate::pattern::TopicPattern`
- `last_n(n) -> Vec<LoggedEvent<T>>` — 最新 N 条,新→旧
- `clear()` / `all()`

### 1.2 不假装

- **0 持久化** — in-memory only, 进程重启即丢. 真持久化是后续范畴 (R229+1 WAL + sqlite).
- **0 引外部 dep** — Vec + Mutex + VecDeque + Instant, 全 std.
- **不假装 fsync** — 不调用任何 syscall, 不假装持久化.

### 1.3 L0Bus 集成

```rust
pub fn with_event_log(self) -> Self  // 默认 capacity 1024
pub fn with_event_log_capacity(self, cap: usize) -> Self
pub fn event_log(&self) -> Option<&Arc<EventLog<T>>>
```

`publish` 在 BackpressurePolicy match + pattern fan-out 后 append 到 log (if enabled).
0 触碰既有 path.

## 2. 测试 (14 cases)

### event_log.rs (8)
- new_log_empty / append_and_query / capacity_overflow_evicts_oldest
- replay_since_filters_by_timestamp / replay_pattern_with_wildcard
- last_n_reverses_order / clear_empties_log / shared_event_log_creates_arc

### r216_tests.rs L0Bus 集成 (6)
- r229_01_publish_records_to_event_log
- r229_02_no_event_log_by_default
- r229_03_replay_topic_returns_matching
- r229_04_replay_pattern_with_wildcard
- r229_05_event_log_overflows_at_capacity
- r229_06_last_n_reverses_order

## 3. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 62 → 76 (+14)

## 4. 战区意义

apeireth-bus 从"实时 pub/sub"升级到"实时 pub/sub + 事件溯源". 这是分布式系统
关键基础设施:

- **Debug 重现**: 任何问题可以 replay 历史事件, 找到 root cause
- **审计**: 谁在什么时间发了什么 (trace_id + topic + payload)
- **回滚**: 重放某窗口的事件, 重建状态
- **监控**: 用 replay API 统计 topic 流量 / 高频事件 / 异常模式

## 5. 下一步候选

- **R230** tool-fetch per-host rate limit
- **R231** bus event_log persistence (WAL + sqlite)
- **R232** council streaming deliberation
- **R233** consciousness temporal emotion decay per-event
- **R234+** protocol Arrow / DataFusion (大项目, 最后)