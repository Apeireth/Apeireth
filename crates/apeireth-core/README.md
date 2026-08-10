# apeireth-core

> **职责**: 核心抽象 (traits / 错误层级 / 类型 / 配置加载)
> **状态**: R11 占位实现
> **对应文档**: 阶段 2 §3 核心抽象层 + 阶段 2 §2 通信总线 (Bus trait 抽象)

---

## 设计意图

`apeireth-core` 是 Apeireth 的"核心抽象层"crate, 所有其他 crate 都依赖它:

1. **核心 traits** — `Sovereignty` / `Council` / `PrincipleLayer` / `PermissionMatrix` / `ExperienceStore` / `PluginHost` (阶段 2 §16 6 traits)
2. **错误层级** — `ApeirethError` 顶层 + 各子错误
3. **类型定义** — `Message` / `Decision` / `Situation` 等
4. **配置加载** — TOML / env / secret

## 依赖方向

```
所有 crate → apeireth-core (依赖)
apeireth-core → std only (零依赖)
```

## 6 个核心 trait (阶段 2 §16)

```rust
pub trait Sovereignty: Send + Sync { /* 主 AI 决策 */ }
pub trait Council: Send + Sync { /* 智囊团咨询 */ }
pub trait PrincipleLayer: Send + Sync { /* 原则洋葱 5 层 */ }
pub trait PermissionMatrix: Send + Sync { /* 权限矩阵 */ }
pub trait ExperienceStore: Send + Sync { /* 经验沉淀 */ }
pub trait PluginHost: Send + Sync { /* 插件宿主 */ }
```

**不模仿 Hermes** — 按 Apeireth 实际情况设计 (阶段 1 §16 修正)。

---

_主哲学 anchor: 主 19:33 走在前人经验上 (trait 设计借鉴 OpenClaw/Hermes) + 主 17:43 实事求是 (基于实际需求)._