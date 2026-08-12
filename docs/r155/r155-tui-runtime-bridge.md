# R155 — TUI × `apeireth-runtime` 集成 (RuntimeBridge)

> **R155 (2026-08-13)**: `apeireth-tui` 加 `runtime_bridge.rs` 模块 (~371 行) — wrap `apeireth-runtime::Runtime` 给 TUI main loop 拉取状态 (cycle 报告 + 异步任务 + 群聊消息 + 情感快照 + 仲裁日志 + 搜索索引). 累计 +17 tests (10 lib unit + 7 integration), 0 errors, 0 触碰 3 不可变脊柱, 0 触碰现有 5 nav 页面渲染. 同时给 `apeireth-tui/Cargo.toml` 加 5 个 deps (apeireth-runtime / apeireth-council / apeireth-arbitration / apeireth-tool-search / apeireth-tool-registry) + parking_lot (Mutex).

---

## 1. 动机

apeireth-runtime R147 已经实现 7 模块端到端 orchestration (HeartbeatScheduler / AsyncTaskStore / ChanneledBus / ArbitrationLog / SearchEngine / GroupChat / EmotionEngine). 但 TUI 没接 — TUI 当前直接调 9 器官 + 后端 30+ crate, runtime 编排层是独立运行的, 状态没回流到 TUI.

主人指示"后端完全做好了再接 tui" — 后端从 R147 起就 OK, R155 正式接.

## 2. 设计决策

### 2.1 Pull-based 桥 (而非 push-based event stream)

```rust
let bridge = RuntimeBridge::new(Arc::new(Runtime::with_config(config)));
// TUI main loop 每 60ms tick:
bridge.refresh_emotion();
let state = bridge.state();
```

不像 Elm/Yew 的 reactive 模型, TUI 是 pull-based (每帧 render). 用 push-based event stream 会需要 Arc<Mutex<>> 包裹每个 state piece + invalidation 逻辑, 复杂度高.

`RuntimeBridge` 提供:
- `refresh_emotion()` — pull 最新 EmotionSnapshot (caller 一帧一调)
- `state()` — snapshot cached BridgeState (cycle count / task ids / chat messages)
- `run_cycle()` / `dispatch_task()` — push events that update cache (when TUI user triggers)
- `snapshot_json()` — 序列化 for telemetry

### 2.2 Bounded-push cache

```rust
fn bounded_push<T>(list: &mut Vec<T>, item: T, max: usize) {
    list.insert(0, item);
    if list.len() > max {
        list.truncate(max);
    }
}
```

TUI render 是 O(n) over lists, 避免无限增长. 默认 32 tasks / 32 arb events / 16 messages.

### 2.3 Participant 显式注册

```rust
bridge.add_participant("test_user", "Test User")?;
bridge.post_message("test_user", "hello".to_string())?;
```

apeireth-council GroupChat 要求 sender 必须是 room participant (R147 设计). `post_message` 不自动注册 — 显式调用保证 audit trail 清晰.

### 2.4 不动 TUI 现有 5 nav 页面

`apeireth-tui/src/main.rs` + `pages/{bridge,dialogue,growth,history,settings}.rs` 0 改动. `runtime_bridge.rs` 是 additive 模块, TUI caller 按需 use.

### 2.5 parking_lot Mutex 引入

按 ponytail ceiling 原则, parking_lot 是 workspace transitive dep (已在 apeireth-runtime 用). 引入 `apeireth-tui` 让 BridgeState 用 parking_lot::Mutex 而非 std::sync::Mutex (后者 async 行为差).

## 3. 模块 API

### 3.1 `BridgeState` — 缓存 state

```rust
pub struct BridgeState {
    pub last_cycle_report: Option<CycleReport>,
    pub cycle_count: u64,
    pub recent_task_ids: Vec<TaskId>,
    pub recent_arbitration_seqs: Vec<i64>,
    pub recent_chat_messages: Vec<String>,
    pub emotion: Option<EmotionSnapshot>,
    pub last_event_topic: Option<String>,
}

impl BridgeState {
    pub const MAX_TRACKED_TASKS: usize = 32;
    pub const MAX_TRACKED_ARBITRATION: usize = 32;
    pub const MAX_TRACKED_MESSAGES: usize = 16;
}
```

### 3.2 `RuntimeBridge` — 主结构

| 方法 | 用途 |
|------|------|
| `new(Arc<Runtime>)` | wrap existing runtime |
| `with_config(RuntimeConfig)` | 新建 runtime + bridge (convenience) |
| `state() -> BridgeState` | 读 cached state snapshot |
| `cycle_count() / last_cycle_report()` | 单项读 |
| `current_emotion() -> EmotionSnapshot` | 实时读 (非缓存) |
| `apply_emotion(EmotionEvent)` | TUI-驱动情感更新 |
| `set_emotion_baseline(Pad)` | 设置 baseline |
| `default_room_id() / participant_count() / recent_messages(n)` | 群聊读 |
| `total_tasks() / total_arbitration_events() / total_indexed_docs()` | 计数读 |
| `run_cycle() -> Option<CycleReport>` | 触发一次心跳 (async) |
| `dispatch_task(tool, params) -> TaskId` | dispatch async task (async) |
| `refresh_emotion()` | pull 最新 emotion snapshot |
| `bootstrap() -> Result<String, String>` | 创建默认 room + 3 participants |
| `add_participant(id, display_name)` | 注册新 participant |
| `post_message(sender, content)` | 发送消息 (需先 add_participant) |
| `snapshot_json() -> serde_json::Value` | 序列化 for telemetry |

## 4. 测试覆盖 (R155 累计)

| 测试类型 | 数量 | 位置 |
|---------|------|------|
| lib unit (10 tests) | 10 | `src/runtime_bridge.rs::tests` |
| integration (7 tests) | 7 | `tests/test_runtime_bridge.rs` |
| pre-existing TUI lib unit | 424 | `src/{app,pages,backend,...}::tests` |
| **Total** | **441 + 0 failed** | |

## 5. 0-touch 声明

按 8 项不修改承诺 + R147 主轴:

- ✓ 0 触碰 `docs/v4/v4.1/v2/V0.5/V1136/9键原始`
- ✓ 0 触碰 `workspace.version` (1.2.0)
- ✓ 0 触碰 R11 baseline 3 values
- ✓ 0 触碰 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- ✓ 0 改 `apeireth-runtime` 签名 (bridge 是单向消费)
- ✓ 0 改 TUI 现有 5 nav 页面 (`pages/{bridge,dialogue,growth,history,settings}.rs` 0 触碰)
- ✓ 0 改 TUI 现有 backend.rs (1138 行 binary 0 触碰)

## 6. 下一步候选 (R156+)

- TUI 调用 `RuntimeBridge::run_cycle()` 接 keyboard event (按主人指示可逐步接入)
- TUI Bridge 状态可视化 (舰桥页显示 cycle_count + emotion snapshot)
- 调研 GitHub 优秀项目 (per master 8/12 指示)
- WebRTC signaling for apeireth-voice (留 R153 已知限制 #1)
