# 0008. R2 扩展 ABI 统一（Extension ABI / 单注册表 / 单命名）

- **Status**: **Accepted**（owner 2026-10-20「按建议来」；施工按 R2-1..R2-5 分批）
- **Owner 裁决（2026-10-20，按建议）**: ①canonical 名 = **`BehaviorModule`/`BehaviorRegistry`**（迁移后删 alias）；②**批准删除** plugin 重复 `CapabilityRegistry`（并入 `ExtensionDescriptor.capabilities`，保留 PluginManager 内部索引语义）；③R2-4 改名批次 **等生物线当前批收口后**执行；④`PluginManager` **保留原名**（ABI 配置期角色实现名）。
- **Date**: 2026-10-20
- **Depends on**: 无（R2 是 R1b/R7 的前置；本 ADR 为它们解锁）
- **Owners**: 架构线（spec/验收）· 各线实施（R2-1..R2-4 批）
- **契约影响**: `contract.toml [extension_points]` 收敛为单表；三条 `[[extension_debt]]` 在 R2 完成后清除（契约在波收口由 lead 更新）
- **例外关联**: 无（纯重命名/删除重复实现；不新增扩展点）
- **预算净额**: 净删除（plugin 重复 CapabilityRegistry + 命名 alias），无新增结构

## 1. 背景（2026-10-20 实测，不沿用 2026-10-18 审计数字）

> 逐项清单（alias 矩阵 / 注册表构造点 / 批次 file-level 范围 / 待 owner 问题）见 [docs/architecture/r2-inventory-2026-10-20.md](../architecture/r2-inventory-2026-10-20.md)。

`contract.toml [extension_points]` 已登记 6 个扩展点面，`[[extension_debt]]` 记录三条债务（证据见下）：

| 债务 | 实测证据（2026-10-20） |
|---|---|
| 两套扩展 ABI + 三份注册表 | `pub trait BehaviorModule`（`runtime/src/canonical/module.rs:426`）与 `pub trait Plugin`（`foundation/plugin/src/plugin.rs:72`）；注册表 `PluginManager`（34 处，src 33）、`BehaviorRegistry`（10）、`CapabilityRegistry`（26） |
| foundation/plugin 内同名 `CapabilityRegistry` | **2 个定义**：`runtime/src/canonical/capability.rs:49`（canonical，contract 已登记）与 `foundation/plugin/src/registry.rs:122`（**plugin 内部**被 `PluginManager` 用作 id→owner 索引，`manager.rs:32`；外部 0 消费） |
| 命名漂移 | `AgentModule` 75 处（src **9 文件**；alias 定义 `module.rs:443`）；`ModuleRegistry` 5 处（alias `module.rs:496`）；canonical 应为 `BehaviorModule` / `BehaviorRegistry` |

约束：foundation **不能依赖** engine（`plugin` 是 foundation，`BehaviorRegistry` 在 runtime）→ "物理单注册表"不可行；可行目标是 **单 ABI 家族 + 单命名 + 单 id→owner 索引**，物理注册表按层各一但共享同一套描述/目录契约。

## 2. 决策（Proposed）

### 2.1 单 ABI 家族（R2-2）

- 在 `foundation/plugin` 增加共享 ABI 基础类型（**新增，不破坏**）：
  - `ExtensionKind`（`Plugin` / `BehaviorModule` / `Capability` / `SubLoop`）；
  - `ExtensionDescriptor`（id / kind / owner crate / order / capability 列表）；
  - `ExtensionCatalog` trait（`register/describe/ids/contains`），**由 `PluginManager` 与 `BehaviorRegistry` 各自实现**（foundation/engine 两层，逻辑同一 ABI）；
- 两套 trait（`Plugin` 配置期 provider / `BehaviorModule` 运行期 hook 角色）**保留**——它们是同一 ABI 的两个角色，不强行合并（合并会逼 config-time provider 实现运行期钩子）。`ExtensionKind` 负责对它们统一寻址/描述。

### 2.2 单注册表口径（R2-3）

- `CapabilityRegistry` 唯一 canonical = `runtime/src/canonical/capability.rs`；**删除** `foundation/plugin/src/registry.rs` 的重复 `CapabilityRegistry`/`CapabilityRecord`（其 id→owner 索引并入 `ExtensionDescriptor.capabilities`）；
- `PluginManager` 内部 `PluginRegistry` 保留但**私有化**（不对外导出的第二真源）；对外只经 `ExtensionCatalog`。

### 2.3 单命名（R2-4）

- canonical：`BehaviorModule` / `BehaviorRegistry`；删除 `AgentModule` alias（75 处迁移，src 35 文件）与 `ModuleRegistry` 旧名（5 处）；
- `Plugin` / `PluginManager` 作为 **ABI 角色名**保留（layer 内实现），文档统一称 "Extension ABI"；
- 迁移顺序：别名并存（canonical 名导出）→ 分批迁调用方 → 删 alias/旧名（每批 1 commit，可独立 revert）。

### 2.4 扩展点登记（R2-5）

- `contract.toml [extension_points]` 收敛为单表（保留 6 个登记项，去掉重复语义）；**新增扩展点/注册表仍必须 ADR**；
- 三条 `[[extension_debt]]` 在 R2 完成后清除；`[[concepts]] CapabilityRegistry known_debt` 收敛为 0。

## 3. 迁移批次与验收

| 批 | 内容 | 验收（硬指标） |
|---|---|---|
| R2-1 | 清单/命名矩阵（不改码）：Alias 分布、注册表调用方、plugin CapabilityRegistry 消费方 | 清单可复核 |
| R2-2 | 共享 ABI 基础类型（additive） | 编译绿、0 行为变化、catalog trait 2 实现 |
| R2-3 | 迁移注册面（PluginManager/BehaviorRegistry 实现 catalog；删 plugin CapabilityRegistry） | `CapabilityRegistry` 定义=**1**；plugin 无重复索引 |
| R2-4 | 命名迁移（AgentModule→BehaviorModule；ModuleRegistry→BehaviorRegistry；删 alias） | `git grep '\bAgentModule\b' -- crates/` = **0**；`ModuleRegistry` = 0 |
| R2-5 | 契约登记更新（extension_points/debt/concepts） | `arch_guard` 0 违规；extension_debt 清空 |

**通用门**：每批只做一件事、可独立 revert；golden/回归测试绿；默认行为 0 变化；0 新外部依赖。

## 4. 风险与回滚

- `AgentModule` 75 处（跨 35 src 文件）改名触面大、与并行线冲突风险高 → 必须等生物线批次收口后再动，且分批（每批 ≤20 文件），逐批 `cargo check`；
- 删除 plugin `CapabilityRegistry` 前先 grep 全仓消费方（contract 称未生产外部使用，需 R2-1 复核）；
- 回滚：每批 `git revert`；删除类先归档（`legacy/archived/` + MANIFEST）或保留 git 历史即可（纯代码删除，历史可查）。

## 5. O-6 三阶审查

- **总体最优**：只统一"描述/命名/索引"，不强行合并两个角色 trait——收益（一个概念一个名字、可枚举的扩展面）大、风险（配置期 provider 被逼实现运行期钩子）小；拒 ①合并为单 trait（职责错配）②只改文档不改命名（alias 继续漂移）。
- **系统最优**：共享 ABI 类型放 foundation/plugin（最底层可依赖方向），runtime 的 BehaviorRegistry 实现同一 catalog；拒 ①把 catalog 放 runtime（foundation 反向依赖）②新建第 4 个注册表（违反单注册表目标）。
- **架构最优**：先清单→再加 additive ABI→再迁注册面→最后改名删 alias，每步可验证可回滚；拒 ①一步全改（75 处 rename 无法审）②先改名后加 ABI（中间态双口径更久）。
