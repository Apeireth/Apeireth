# v2.0.0-rc.1 前架构重构批次 (O-6 重构批次, 2026-08-27 启动)

> **现状 (2026-08-27)**：v2.0.0-alpha.1 trait 边界落地后立即发现**O-6 哲学锚** 提示的"位置不对"问题: 多 instance LLM / 凭证 / 记忆等 trait 摆在了 engine/adapter 而**不在 foundation**——架构上违反"trait 抽象在 foundation / impl 在 engine/adapter"原则。本批次是 v2.0.0-rc.1 启动前的**结构性整理** (在接真 backend 前最后一次低成本窗口)。
> 哲学锚依据: [philosophy.md §O-6](../01-architecture/philosophy.md) —— "**等以后做"是借口; 现在"足够好"= 下一版更难改**。
> 接手人: 必读, 6 项必做 + 5 项选做. 0 触碰 LOCKED.

```
[Document-Meta]
Document:        docs/04-internal/v2-arch-refactor-batch.md
Version:         Design-1.0
Last-Modified:   2026-08-27
Status:          🟡 实施中 (O-6 重构批次, v2.0.0-rc.1 之前必完)
```

---

## 0. 为什么现在做, 不等 rc 之后

| 时机 | 改 trait 位置 | 改 backend 绑定 | 改 integration tests | 风险 |
|---|---|---|---|---|
| **现在 (alpha → rc 临界)** | ✅ 容易 (0 装 trait 没人用) | 0 改动 | 0 改动 | **低** |
| rc 阶段 | 难 (impl 已经在接 backend, 改位置 = 改 backend 绑定) | 大量改动 | 测试要重写 | **高** |
| rc 之后 (v2.0.0) | 极难 (用户已依赖) | 大量改动 | 破坏 v1.0 / rc 用户 | **极高** |

**结论**: alpha 阶段锁位置 = **O-6 兑现最低成本时刻**, 必须做.

---

## 1. 三阶审查 (O-6 落地点)

每项改动都过这 3 阶 (写到 commit message):

1. **总体最优**: 在 v2 整体语境里这是不是最优? (ROADMAP / scene-d / 13 键 / 文档体系)
2. **系统最优**: 在 subsystem 依赖图里位置对不对? (trait 在 foundation, impl 在 engine/adapter, 单向依赖)
3. **架构最优**: 引入后整个 workspace 边界是不是更清晰? (单一事实源? 抽象层不重复? 入口语义不歧义?)

## 2. 必做 5 项 (按依赖 + 风险排)

### Refactor-1: `MemoryBackend` trait → `apeireth-plugin` (foundation)

**位置迁移**: `crates/engine/memory/src/backend/mod.rs` → `crates/foundation/plugin/src/memory_backend.rs`

**为什么**: MemoryBackend 是 **capability backend 抽象**, 与 `ToolCapability` / `ProviderCapability` 同级. 应该统一在 plugin crate (trait 抽象层). impl 仍在 memory (engine) - 不动 SqliteBackend / InMemoryBackend / FileBackend 代码.

**变更**:
- 新文件: `crates/foundation/plugin/src/memory_backend.rs` (trait + BackendKind enum)
- `apeireth-memory/src/backend/mod.rs` → `pub use apeireth_plugin::memory_backend::*;` (re-export 保持向后兼容)
- 三个 impl 迁到 `apeireth-memory/src/backend/{sqlite,file,in_memory}.rs`, 都 `use apeireth_plugin::MemoryBackend`
- 删 `MemoryError::Io` 重复 (plugin crate 已有, memory 留 *crate*-specific errors)

**3 阶审查**:
1. 总体: 把"什么算 memory backend"集中到 foundation, 与 Tool/Provider 三件套对齐 = 总体最优
2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin/Provider/Tool 一致) = 系统最优
3. 架构: backend registry 与 plugin registry 同一抽象层 = 架构最优

**回滚**: git revert, 5 分钟.

**风险**: 12 个 v1 consumer 不变 (他们用 apeireth_plugin::* re-export) - **0 破**.

**Acceptance**:
- `cargo test -p apeireth-plugin --lib memory_backend` 通过
- `cargo test -p apeireth-memory --lib backend` 通过
- 现有 `use apeireth_plugin::MemoryBackend` 仍可编译 (zero breaking)

### Refactor-2: `Experience` trait → `apeireth-plugin` (foundation)

**位置迁移**: `crates/engine/memory/src/experience.rs` → `crates/foundation/plugin/src/experience.rs`

**为什么**: Experience trait (Wiki/KG/Association) 是 **capability abstraction**, 同 MemoryBackend 性质.

**变更**:
- 新文件: `crates/foundation/plugin/src/experience.rs` (5 traits + 5 dataclass)
- `apeireth-memory::experience` → `pub use apeireth_plugin::experience::*;` (向后兼容 re-export)
- `extract_experience_from_episode` 0 装仍保留在 memory (impl 边界)

**风险**: 0 破 (同样 re-export pattern).

### Refactor-3: `PerceptionInput/Channel/Attention` trait → `apeireth-plugin` (foundation)

**位置迁移**: `crates/engine/perception/src/lib.rs` (含 5 modality + 3 trait) → `crates/foundation/plugin/src/perception.rs`

**为什么**: 同样 — capability 抽象在 foundation. impl 仍在 perception crate (audio capture / screen capture / attention strategies 属于 input adapter concern, 放 engine/ 或 adapters/).

**变更**:
- 新文件 `crates/foundation/plugin/src/perception.rs` (PerceptionInput/Channel/Attention + 5 modality)
- `apeireth-perception` → `pub use apeireth_plugin::perception::*;` (re-export)
- 5 个 Input impl (Text/Voice/Vision/Tactile/Command) + Channel impl + Attention impl 留在 perception crate
- `Mutex<Option<String>>` 留在 perception (impl detail, 不在 trait 抽象)

**风险**: 0 破.

### Refactor-4: `KeyringCredentialResolver` → `apeireth-plugin` (trait), credentials 留 backend impl

**位置迁移**:
- trait `CredentialResolver` 已在 `apeireth-plugin/src/credentials.rs` (✓) — **0 改**
- impl `KeyringCredentialResolver` 从 `crates/foundation/credentials/src/plugin_bridge.rs` → 改回 `crates/foundation/credentials/src/keyring_resolver.rs` (impl only, 不需 plugin_bridge)
- 删 `plugin_bridge.rs` (迁出) → 加 `KeyringResolver` impl 文件 (只负责 impl)
- credentials crate 不再依赖 plugin (dependency 翻转: 之前 credentials → plugin 写 impl; 之后 plugin → credentials 用 `KeyringResolver` impl 满足 trait)

**等等** — 之前我说"KeyringCredentialResolver 在 credentials 接入 plugin trait"是正向, 现在要不要再翻? **不必**:
- trait 已在 plugin
- impl 在 credentials (这个对)
- credentials → plugin 依赖 OK (credentials 实现了 plugin 的 trait, 需要看 trait 定义)

**实际变更**:
- `crates/foundation/credentials/src/plugin_bridge.rs` 改名为 `keyring_resolver.rs` (impl only, 更准确命名)
- `crates/foundation/credentials/src/lib.rs` `pub mod` 改从 `plugin_bridge` 到 `keyring_resolver`
- 4 个测试保留

**3 阶审查**:
1. 总体: trait 边界清晰, impl 分类准确 (keyring/encrypted_file/in_memory backend)
2. 系统: credentials 依赖 plugin 看 trait, 不反向; 单向
3. 架构: bridge 这个名字是过渡的, 改成 resolver 更准确

### Refactor-5: `core drain` 第二阶段 (kernel 重新定义 + 12 consumer deprecation)

**位置**: `crates/foundation/core/src/kernel/` 模块

**当前** (第一阶段, `f4de51e9`): `pub use crate::memory::Episode as kernel::Episode` — **alias, 同一类型**.

**第二阶段**:
- 在 `kernel/` 模块**重新定义** `Episode / Note / Session / IdentityCard / Migration` (数据字段一致, 但**类型独立**)
- 删 `core/src/memory.rs` / `onion.rs` / `gate.rs` / `lifecycle.rs` / `philosophy.rs` 旧模块 (改用 kernel 的)
- root `pub use` **加 `#[deprecated]`** 指向 `kernel::*`
- 12 consumer (memory, runtime, gateway, cli, sdk, ...) 收到 deprecation warning, 但**不破** (编译过)
- linter: `cargo clippy ... -D warnings` 把 deprecation warn 当 error, 需要分两步:
  - 步骤 5a: 加 `#[allow(deprecated)]` 到所有 12 consumer, 让 build 仍绿
  - 步骤 5b: 后续版本 (rc / v2.0) 删 allow, 强制迁移

**风险**: **高** — 12 consumer 都要动, 即使加 allow 也是侵入式 diff.

**Acceptance**:
- `cargo check --workspace --locked` 通过 (clippy 不开 -D warnings)
- 12 consumer 都加 `#[allow(deprecated)]` on `use apeireth_core::*`
- 新 `Episode` 类型与旧 `Episode` 字段一致, 任何序列化兼容

## 3. 选做 3 项 (v2.0.0-rc 阶段做)

### Refactor-6: orchestrator dispatch trait 拆 runtime concern

**位置**: 现在在 `crates/foundation/orchestration/src/lib.rs` 的 `Orchestrator` trait

**争议**: orchestrate 多 agent 是 **runtime concern** (subagent 调 LLM, runtime 调度), 不是 plugin concern. 现在 trait 在 foundation 偏强, 应该 trait 移到 `crates/engine/runtime/src/orchestrator/`, impl 也 runtime.

**3 阶审查**:
1. 总体: orchestrator 与 runtime 紧耦合 (dispatch subagent = 启 LLM factory instance), 放 runtime 更合
2. 系统: foundation 保留 7 Advisor + Council + SelfAssessmentCache (高层 policy), runtime 保留 Orchestrator (调度)
3. 架构: 1 阶清晰 (policy 在 foundation, mechanism 在 runtime)

**争议**: foundation 与 runtime 的边界. v1 时代是 `apeireth-orchestrator` (separate crate), v2 alpha 我合并到 foundation. **真 arch-optimal 是 runtime**.

**决定**: alpha 已发, 不动. **rc 阶段看 reviewer 用法**: 如果 orchestrator 0 装 impl 与 runtime 集成紧, 移 runtime; 如果主要与 policy/Council 集成紧, 保留 foundation. 推迟到 rc.

### Refactor-7: experience WikiEntry 注入策略

**v1 借鉴**: 3-layer progressive disclosure (目录 + 摘要 + 全文). v2 0 装是抽取后不注入. rc 后实现 memory 集成时一起做.

**位置**: `crates/engine/memory/src/experience.rs` 已经有 trait; rc 实现时加 `WikiStore::recall_for_context()` 返回 top-N, runtime `agent_loop` 调用注入 transcript.

**不做单独** — 跟 RC-2 (Experience SQLite impl) 一起做.

### Refactor-8: Perception 真 modality (Voice/Vision/Tactile)

v2 alpha 0 装, v2.0.0-rc.1 接真 backend. 跟 RC-7 一起.

## 4. 不做 1 项

### Refactor-X: 把所有 trait 集中到 "apeireth-traits" mega-crate

**为什么不做**: 过度抽象. 7 个 trait (Tool/Provider/CredentialResolver/MemoryBackend/Experience/PerceptionInput/SubagentRole) 各有不同语义边界, 集中到 1 crate 反而模糊. 现状 "每个语义 1 trait, 找对位置" 正确.

## 5. 总体时间表

| 任务 | 估计 | 前置 | 风险 |
|---|---|---|---|
| Refactor-1 (MemoryBackend → plugin) | 0.5 天 | - | 低 (re-export pattern 验证过) |
| Refactor-2 (Experience → plugin) | 0.5 天 | - | 低 |
| Refactor-3 (PerceptionInput → plugin) | 0.5 天 | - | 低 |
| Refactor-4 (KeyringCredentialResolver 重命名) | 0.1 天 | - | 低 (仅 rename + pub mod 改) |
| Refactor-5 (core drain 真正重定义) | 1.5-2 天 | Refactor-1-4 完成 | **高** (12 consumer 需加 allow) |
| **总计** | **3-4 天 (1 人)** | - | - |

**完成条件**: 5 项 Refactor 全部 done + workspace test pass + clippy 0 警告 + 远端 main 推上去.

**之后**: v2.0.0-rc 启动, 接真 backend (RC-1 到 RC-10 路线图).

## 6. O-6 落地的可检查信号

每项 Refactor 完成后必过:

- [ ] `cargo check --workspace --locked` 通过
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告
- [ ] 现有测试 0 失败 (alpha 1476+ 测试)
- [ ] commit message 写明"3 阶审查的具体回答"
- [ ] O-6 锚的"可检查信号" 5 条全过 (clippy + 0 静默失败 + commit 0 in-progress + push + CI 0 red)
- [ ] 推上去后 CI 23 个 workflow 全过
- [ ] 0 触碰 LOCKED (9 哲学锚 + 13 键 + 3 项不可变脊柱 + workspace.version + R11 baseline)

## 7. 与 ROADMAP / 文档同步

每项完成同步:
- `ROADMAP.md` §4 加完成行 (新 P-arch task)
- `CHANGELOG.md` [Unreleased] 加行
- `v2-unabsorbed-features.md` §P-arch 状态
- `v2.0.0-rc-roadmap.md` §1 范围 (确认 5 项已不需 rc 阶段重做)

## 8. 一句话总结

O-6 锚兑现 = 1 batch (5 项), 3-4 天, 0 触碰 LOCKED. v2.0.0-rc.1 启动前必完. "等以后做"是借口——alpha 是 0 装, 现在改 0 成本.

---

_本文 O-6 重构批次计划首发 (2026-08-27): v2.0.0-alpha.1 trait 边界 + 0 装占位完成后立即启动. 哲学锚 #9 "永远追求最优" 第一次兑现 = 把 alpha 阶段选错位置 (engine / adapter) 的 trait 搬到正确位置 (foundation plugin). 接手人按 §2 顺序 5 项, 每项 commit message 必含 §1 三阶审查的具体回答. v2.0.0-rc.1 启动时这 5 项必须完成. 0 触碰 LOCKED: 8 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline. alpha 已经 "足够好" 的借口 = v2 rc 时 trait 改位置成本爆炸, 现在改 0 成本 (impl 是 0 装)._
