# R211 ExtendedEmotionEngine (Plutchik emotion engine 集成)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R211
> **日期**: 2026-08-13
> **来源**: R218 Plutchik 8 情绪 + R209 桥接 + R187 认知层调研 + 主人"全做全补弱"
> **状态**: 实施完成, 14/14 单测全过 (累计 67/67)

---

## 0. 动机

现状 (R209):
- `emotion.rs` EmotionEngine 用 6 Ekman 基础情绪 + PAD + 12 events (向后兼容, R22 老接口)
- `plutchik.rs` 8 基础 + 8 高级 + 4 强度 (无 engine, 纯类型定义)
- `plutchik_integration.rs` 6 Ekman <-> 8 Plutchik 桥接 (函数式)

缺口: Plutchik 有类型, 没有 **engine** — 没有"应用事件"的方法, 没有"维护当前 Plutchik 状态"的概念.

R211 给 Plutchik 配 engine: `ExtendedEmotionEngine`, 同时持有:
1. 内部 EmotionEngine (6 Ekman, 用于向后兼容 6 维推断)
2. Plutchik state (current_basic 8 + current_advanced 8 + current_intensity 4)
3. 14 PlutchikEvent (8 基础 + 6 高级)
4. Plutchik 专属 history

---

## 1. 设计

### 1.1 公共 API

```rust
pub enum PlutchikEvent {  // 14
    Joy, Trust, Fear, Surprise, Sadness, Disgust, Anger, Anticipation,  // 8 基础
    Love, Optimism, Remorse, Contempt, Awe, Aggressiveness,  // 6 高级
}

pub struct ExtendedEmotionEngine {
    ekman: EmotionEngine,        // 内部包装, 0 触碰 emotion.rs
    current_basic: PlutchikBasic,
    current_advanced: Option<PlutchikAdvanced>,
    current_intensity: PlutchikIntensity,
    history: VecDeque<PlutchikEmotion>,
    history_capacity: usize,
    intensity_decay_rate: f32,
    event_count: u64,
}

impl ExtendedEmotionEngine {
    pub fn new() -> Self;
    pub fn with_capacity(self, cap: usize) -> Self;
    pub fn with_decay_rate(self, rate: f32) -> Self;

    pub fn apply(&mut self, event: PlutchikEvent) -> ExtResult<()>;
    pub fn decay(&mut self, dt_secs: f32);

    pub fn current_emotion(&self) -> PlutchikEmotion;
    pub fn current_basic(&self) -> PlutchikBasic;
    pub fn current_advanced(&self) -> Option<PlutchikAdvanced>;
    pub fn current_intensity(&self) -> PlutchikIntensity;
    pub fn event_count(&self) -> u64;

    pub fn closest_basic_from_pad(&self, pad: Pad) -> PlutchikBasic;
    pub fn closest_ekman(&self) -> BaseEmotion;  // 通过内部 ekman 推断 6 Ekman

    pub fn ekman_engine(&self) -> &EmotionEngine;
    pub fn ekman_engine_mut(&mut self) -> &mut EmotionEngine;
    pub fn history(&self) -> Vec<PlutchikEmotion>;
}
```

### 1.2 PlutchikEvent -> PlutchikEmotion 映射

`PlutchikEvent::emotion()` 是 `const fn` — 返回 `PlutchikEmotion::Basic/Advanced` 带 Moderate 强度.
调用方可以用 `apply_with_intensity` 覆盖强度, 但 R211 默认 Moderate.

### 1.3 高级情绪回写基础

`Love` (Joy+Trust) → `current_basic = Joy` (主导) + `current_advanced = Some(Love)`.
这样 `current_emotion()` 优先返回高级, 但 6 Ekman 推断仍能从 `current_basic = Joy` 拿到正确主导.

### 1.4 intensity 升级规则

- Mild + resonance > 0.8 → Moderate
- Moderate + resonance > 0.7 → Strong
- Strong + resonance > 0.85 → Extreme

decay 降级: 每次 decay(dt > 0.5s) 降一档 (Extreme → Strong → Moderate → Mild → Mild).

### 1.5 双引擎同步

`apply(PlutchikEvent)` 同步:
1. 更新 Plutchik state
2. 转发到 `ekman` (如果 `to_ekman_event()` 返回 Some)
   - Trust / Anticipation / 高级 (部分) → None (不污染 Ekman)
   - Joy → UserPraise, Anger → UserCritique 等

### 1.6 14 PlutchikEvent::to_ekman_event() 映射

| PlutchikEvent | Ekman EmotionEvent |
|---|---|
| Joy | UserPraise |
| Fear | Intense |
| Surprise | Novelty |
| Sadness | Silence |
| Anger | UserCritique |
| Disgust | UserCritique |
| Trust, Anticipation | (None — Plutchik 独有) |
| Love | DeepTalk |
| Optimism | TaskSuccess |
| Remorse | TaskFailure |
| Contempt | AgentConflict |
| Awe | Novelty |
| Aggressiveness | AgentConflict |

---

## 2. 测试覆盖 (14 cases)

| ID | 用例 | 覆盖点 |
|---|---|---|
| t01 | event_count | 14 events 编译期守门 |
| t02 | new_defaults | 初始 state = Joy / Mild / count=0 |
| t03 | joy_event | 基础情绪 apply |
| t04 | trust_event_plutchik_only | Trust 不污染 Ekman |
| t05 | anticipation_event_plutchik_only | Anticipation 不污染 Ekman |
| t06 | love_advanced | 高级情绪 + 回写 + 触发 Ekman |
| t07 | optimism_advanced | wrap pair (Anticipation+Joy) |
| t08 | awe_advanced | Fear 主导 + Awe 高级 |
| t09 | intensity_escalation | Mild → Moderate → Strong |
| t10 | decay_reduces_intensity | Strong → Mild 降级 |
| t11 | closest_basic_from_pad_neutral | PAD 推断 |
| t12 | history_capacity | VecDeque 容量 |
| t13 | event_count_tracking | 计数 |
| t14 | ekman_dual_inference | Ekman + Plutchik 同步 |

累计 `cargo test -p apeireth-consciousness --lib`: 67 passed (53 旧 + 14 新).

---

## 3. 0 触碰守门

- `emotion.rs` EmotionEngine / BaseEmotion / EmotionEvent / Pad 0 改
- `plutchik.rs` PlutchikBasic / PlutchikAdvanced / PlutchikIntensity / PlutchikEmotion 0 改
- `plutchik_integration.rs` 6 个桥接函数 0 改
- `lib.rs` 只加 1 行 `pub mod plutchik_engine;`
- 3 不可变脊柱 0 触碰
- workspace.version 1.2.0 0 改
- 0 新增 Cargo.toml 依赖 (用 thiserror / serde 已存在)

---

## 4. 路线意义

R218 (Plutchik 类型) → R209 (桥接) → R211 (Engine) → R212+ (集成到 LLM tone 选择 / pipeline).

R211 完成后, consciousness 战区:
- 5 源文件 (lib + emotion + plutchik + plutchik_integration + plutchik_engine + transfer_monitor)
- 67 测试 (含 6 状态机 + 12 emotion + 12 plutchik + 10 integration + 14 engine + 13 transfer)
- 真正的 6 维 Ekman + 8 维 Plutchik + 4 维 intensity 联合情感模型

---

## 5. 下一步

- **R212** council deliberation checkpoint (3-5 days, 高 ROI)
- **R213** tool-codesearch streaming/batch + 真 LRU (1-2 days)
- **R217** Kani 1 proof 演示 (2-3 hours)
- **R149** apeireth-tool-fetch (Tier 1 唯一缺项)
