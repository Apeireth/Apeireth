# R209 Plutchik <-> BaseEmotion 6 桥接 (接续 R218)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R209
> **日期**: 2026-08-13
> **来源**: R218 Plutchik 8 情绪实施 + R187 调研
> **状态**: 实施完成, 10/10 单测全过 (累计 53/53)

---

## 0. 动机

R218 写了 Plutchik 8 基础情绪, 但和现有 BaseEmotion 6 情绪是孤立的. R209 加桥接:
- 6 BaseEmotion <-> 8 PlutchikBasic (4 个直接对应, 2 个 Plutchik 独有)
- 8 PlutchikAdvanced -> 6 BaseEmotion (按主导情绪映射)
- PAD 中心互转
- closest_base_emotion / closest_plutchik_basic (找最近)

---

## 1. 设计

### 1.1 公共 API

`
ust
pub fn plutchik_pad_center(basic: PlutchikBasic) -> Pad;
pub fn base_to_plutchik(base: BaseEmotion) -> PlutchikBasic;
pub fn plutchik_to_base(basic: PlutchikBasic) -> Option<BaseEmotion>;
pub fn plutchik_advanced_to_base(advanced: PlutchikAdvanced) -> Option<BaseEmotion>;
pub fn pad_distance(a: Pad, b: Pad) -> f32;
pub fn closest_base_emotion(pad: Pad) -> BaseEmotion;
pub fn closest_plutchik_basic(pad: Pad) -> PlutchikBasic;
`

### 1.2 映射规则

- 4 直接对应: Joy/Fear/Anger/Sadness <-> Plutchik 4
- 2 Plutchik 独有: Trust / Anticipation -> BaseEmotion::None
- 8 PlutchikAdvanced -> 6 BaseEmotion (主导情绪, e.g. Love -> Joy)

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 现有 BaseEmotion 6 情绪 API: 0 改
- emotion.rs / transfer_monitor.rs: 0 改
- R218 plutchik.rs: 0 改 (本 R 仅消费其 API)
- lib.rs 改 1 行: pub mod plutchik_integration

---

## 3. 测试 (10/10 pass, 累计 53/53)

- t01-t02: 4 直接对应 + 2 Plutchik 独有
- t03-t04: 8 -> 6 + 4 roundtrip
- t05: 8 Advanced -> 6 BaseEmotion
- t06-t07: PAD center + distance
- t08-t10: closest_base/plutchik

---

## 4. 中期路径 (R209+1 候选)

- 集成进 EmotionEngine (让 engine 同时支持 6 + 8 表达)
- 暴露给 LLM (16 情绪输出)
- LLM call 加 Plutchik PAD center 引导