# 6. OpenHer 情感引擎 — O-4 接手

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/06-openher.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`EmotionEngine` PAD 3 维情感:
- `Pad { p: f32, a: f32, d: f32 }` (Pleasure-Arousal-Dominance, -1.0..1.0)
- 6 基础情绪 (Joy / Sadness / Anger / Fear / Surprise / Disgust)
- 12 事件触发 (UserPraise / UserCritique / TaskSuccess / TaskFailure / ToolError / ToolOk / Novelty / Intense / Silence / DeepTalk / AgentCoop / AgentConflict)
- 线性衰减 (`decay_rate`, 向 baseline 收拢)
- `resonance` 强度 (0.0..1.0)
- `ResponseStyle` enum (Warm / Friendly / Gentle / Cautious / Diplomatic / Curious / Professional) — LLM tone 指南

## 为什么 PAD 而非 Ekman

VCP OpenHer 描述"情感引擎调动当日记忆, 提供身份依赖与情绪共振", 没明说模型. 我们选 PAD:
- 3 维连续空间, 经典心理学
- 6 基础情绪 (Ekman) 作离散标注
- 12 事件触发, 线性衰减

不选 Ekman 6 情绪的原因: 6 离散不够表达细腻度, 缺 dominant / submissive 维度.

## 哲学基础

**O-4 接手**: 任何人接手都能调"情感参数" — 这才是诚实的"情感引擎". 论状态机可参数化.
**S-2 实事求是**: 情感不是 LLM call, 是状态机. 状态机可验证, LLM call 不可验证.

## 伦理边界

情感引擎只影响 response style, **不影响 decision making**. 决策仍由 13 键 verdict cache 守门 (per [`docs/conventions/09-anchor.md`](../conventions/09-anchor.md)).

## 局限性

- 0 真感知 (无 LLM-driven 情感更新)
- 12 事件硬编码 (R146+ 可学习)
- 跨 session 情感连续性需持久化

## 借鉴

VCP v1.1 "OpenHer 情感引擎调动当日记忆, 提供身份依赖与情绪共振".

## 内部参考

- 实现: [`crates/apeireth-consciousness/src/emotion.rs`](../../crates/apeireth-consciousness/src/emotion.rs)
- 6 状态机: [`crates/apeireth-consciousness/src/lib.rs`](../../crates/apeireth-consciousness/src/lib.rs)
- 索引: [`README.md`](README.md)
