# R235 — apeireth-runtime auto_decay 集成

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R235
> **日期**: 2026-08-14
> **状态**: 1 commit, 1 测试 +1, 0 errors / 0 warnings

---

## 0. 主人指示

"继续" (2026-08-14)

## 1. 设计

R234 EmotionEngine::auto_decay 已落地, 但未接入 runtime 主循环.
R235 在 `run_one_cycle()` 开头调一次, 让 emotion engine 自行衰减.

### 1.1 改动

```rust
pub async fn run_one_cycle(&self) -> RuntimeResult<CycleReport> {
    // R235: 每个 cycle 开先调 auto_decay, 让 emotion engine 自行算 elapsed 衰减
    let _decayed_secs = self.emotion.lock().auto_decay();
    let start = now_ms();
    let trace_id = apeireth_bus::next_trace_id();
    // ... existing flow ...
}
```

### 1.2 不假装

- 用 wallclock (auto_decay 内部) — 真 wallclock
- 0 触碰既有 cycle 流程 (dispatch → search → arbitration → group_chat → emotion.apply)
- 不假装"实时衰减" — 每次 cycle 调一次, frequency 由 scheduler tick 决定

## 2. 测试 (1 case)

| 测试 | 验证 |
|---|---|
| t11_runtime_cycle_calls_emotion_auto_decay | cycle 内 last_event_at_ms 推进 |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings**
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **runtime 测试**: 23 → 24 (+1)

## 4. 战区意义

apeireth-runtime 补时间感知 emotion:
- 每个 cycle 自动衰减 emotion (向 baseline)
- 不依赖外部调度 — engine 自身 wallclock 决策
- 上层 0 改

## 5. 累计 (R224-R235, 12 commits / 12 子模块)

| R | 主题 | 战区 |
|---|---|---|
| R224 | mcp JSON-RPC 2.0 §6 Batch | protocol + lib |
| R225 | 修 pre-existing 测试错 | workflow + codesearch |
| R226 | bus BackpressurePolicy +Coalesce +Adaptive | lib |
| R227 | bus topic pattern matching | pattern |
| R228 | L0Bus subscribe_pattern 集成 | l0 |
| R229 | bus event_log / replay | event_log + l0 |
| R230 | tool-fetch RateLimiter | rate_limit |
| R231 | tool-fetch engine rate limit 集成 | engine |
| R232 | council collect_opinions | deliberation |
| R233 | tool-codesearch query_batch | unified |
| R234 | consciousness EmotionEngine auto_decay | emotion |
| R235 | runtime auto_decay 集成 | lib |

## 6. 下一步候选

- **R236** council streaming deliberation (callback API)
- **R237** tool-codesearch ast-grep in-process
- **R238+** protocol Arrow / DataFusion (大项目)