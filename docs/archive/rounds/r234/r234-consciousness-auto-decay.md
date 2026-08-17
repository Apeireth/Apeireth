# R234 — apeireth-consciousness EmotionEngine auto_decay

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R234
> **日期**: 2026-08-14
> **状态**: 1 commit, 6 测试 +6, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱" + "继续" (2026-08-14 02:xx)

## 1. 设计

apeireth-consciousness::EmotionEngine 已有 `decay(dt_secs)`, 但要求 caller 手动算
elapsed. R234 加 `auto_decay` — engine 用 wallclock 自行算 elapsed, 上层只需周期
性调一次.

### 1.1 字段 + API

```rust
pub struct EmotionEngine {
    pad: Pad,
    baseline: Pad,
    history: VecDeque<EmotionSnapshot>,
    decay_rate: f32,
    history_capacity: usize,
    event_count: u64,
    /// R234: 上次事件时间戳 (epoch ms)
    last_event_at_ms: i64,
}

impl EmotionEngine {
    pub fn auto_decay(&mut self) -> f32 { /* wallclock 版本 */ }
    pub fn auto_decay_at(&mut self, now_ms_value: i64) -> f32 { /* 测试友好 */ }
    pub fn last_event_at_ms(&self) -> i64;
}
```

### 1.2 auto_decay 流程

```
now = now_ms()
elapsed_ms = (now - last_event_at_ms).max(0)
elapsed_secs = elapsed_ms / 1000
if elapsed_secs > 0:
    self.decay(elapsed_secs)
    self.last_event_at_ms = now
return elapsed_secs
```

### 1.3 不假装

- 用 wallclock (`std::time::SystemTime`) — 真 wallclock, 不假装内部时钟
- 0 触碰既有 `apply` / `decay` / `snapshot` / `dominant_emotion` / `response_style`
- elapsed = 0 时 no-op, 不强行 decay

## 2. 测试 (6 cases)

| 测试 | 验证 |
|---|---|
| r234_01_last_event_at_ms_init_to_now | init 时记录 now, 之后不变 |
| r234_02_apply_updates_last_event_at_ms | apply 后 last_event_at_ms 更新 |
| r234_03_auto_decay_returns_elapsed_secs | 立即调返非负 elapsed |
| r234_04_auto_decay_at_explicit_time | 10s elapsed 后 pad 衰减 |
| r234_05_auto_decay_at_zero_elapsed_noop | 0s elapsed → pad.p 不变 |
| r234_06_auto_decay_updates_last_event_at_ms | auto_decay_at 后 last_event 推进 |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 67 → 73 (+6)

## 4. 战区意义

apeireth-consciousness 补时间感知衰减:
- **上层 (pipeline / runtime)** — 周期性 `auto_decay()`, engine 自行决定
- **测试友好** — `auto_decay_at(now_ms)` 传显式时间, 0 sleep 等待
- **0 触碰** 既有 API, additive
- **可观测** — 返回 elapsed_secs, 上层可记日志

## 5. 下一步候选

- **R235** consciousness auto_decay 集成到 runtime (HeartbeatScheduler tick)
- **R236** council streaming deliberation (callback API)
- **R237** tool-codesearch ast-grep in-process
- **R238+** protocol Arrow / DataFusion (大项目, 最后)