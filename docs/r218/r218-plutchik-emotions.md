# R218 Plutchik 8 情绪进 consciousness (R187 调研推荐)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R218
> **日期**: 2026-08-13
> **来源**: R187 cognition 调研提到 Plutchik 情感轮
> **状态**: 实施完成, 12/12 单测全过 (累计 43/43)

---

## 0. 背景

apeireth-consciousness/emotion.rs 当前用 6 基础情绪 (Joy/Sadness/Anger/Fear/Surprise/Disgust, Ekman 模型).

R187 cognition 调研提到 Plutchik 情感轮 (8 基础 + 8 高级 = 16) 是经典情感理论, 与 Ekman 6 情绪并列. R218 加 Plutchik 8 + 8 + 强度分类.

---

## 1. 设计

### 1.1 Plutchik 8 基础情绪

按 Plutchik 1980 情感轮 (顺时针 0-7):
- 0 Joy (喜)
- 1 Trust (信任)
- 2 Fear (恐惧)
- 3 Surprise (惊讶)
- 4 Sadness (悲伤)
- 5 Disgust (厌恶)
- 6 Anger (愤怒)
- 7 Anticipation (期待)

### 1.2 Plutchik 8 高级情绪 (Dyads, 相邻复合)

- Love (Joy+Trust)
- Submission (Trust+Fear)
- Awe (Fear+Surprise)
- Disapproval (Surprise+Sadness)
- Remorse (Sadness+Disgust)
- Contempt (Disgust+Anger)
- Aggressiveness (Anger+Anticipation)
- Optimism (Anticipation+Joy, wrap)

### 1.3 强度 (4 档)

Mild / Moderate / Strong / Extreme

### 1.4 复合 API

`
ust
pub enum PlutchikEmotion {
    Basic(PlutchikBasic, PlutchikIntensity),
    Advanced(PlutchikAdvanced, PlutchikIntensity),
}

PlutchikAdvanced::from_pair(a, b) -> Option<Self>  // 相邻或 wrap 才返回
`

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- emotion.rs 现有 6 基础情绪 API: 0 改 (新 plutchik.rs 是 additive)
- 现有 emotion / transfer_monitor 模块: 0 改
- lib.rs 改 1 行: pub mod plutchik

---

## 3. 测试 (12/12 pass, 累计 43/43)

- t01: basic count
- t02: advanced count
- t03: basic as_str
- t04: wheel positions
- t05: from_pair adjacent (Joy+Trust=Love)
- t06: from_pair not adjacent
- t07: from_pair same emotion
- t08: from_pair opposite (Fear+Anger 对位)
- t09: intensity as_str
- t10: PlutchikEmotion basic
- t11: PlutchikEmotion advanced
- t12: all 8 advanced pairs (含 wrap)

---

## 4. 中期路径 (R218+1 候选)

- 集成进 emotion.rs (把 Plutchik 8 替换/补充 Ekman 6)
- 加 PAD 转换 (Plutchik 8 位置 -> PAD 3 维)
- 暴露给 LLM (让 agent 表达 16 情绪, 不是 6 情绪)