# R2-1 现状清单（ADR-0008 预研，2026-10-20）

> 目的：为 owner 裁决 ADR-0008 提供可复核证据；本清单不改代码。
> 口径：静态 `git grep`；"引用/内部使用"≠"生产外部消费"。

## 1. ABI 与 alias

| 概念 | canonical | alias | alias 定义 |
|---|---|---|---|
| 运行期模块 trait | `BehaviorModule`（`runtime/src/canonical/module.rs:426`） | `AgentModule` | `runtime/src/canonical/module.rs:443` `pub use BehaviorModule as AgentModule;` |
| 模块注册表 | `BehaviorRegistry`（`module.rs`） | `ModuleRegistry` | `module.rs:496` `pub use BehaviorRegistry as ModuleRegistry;` |
| 配置期 provider trait | `Plugin`（`plugin/src/plugin.rs:72`） | — | — |

`AgentModule` 在 src 的分布（9 文件）：`runtime` 3、`memory` 1、`runtime-assembly` 5（**其中 `bio_lifecycle.rs`/`bio_tone.rs` 是生物线在途文件**，改名必须等其收口）。

## 2. 注册表

| 注册表 | 定义 | 构造点（实测） | 判定 |
|---|---|---|---|
| `BehaviorRegistry` | `runtime/src/canonical/module.rs` | `runtime/src/canonical/runtime.rs:1053` | 单装配根持有 |
| `PluginManager` | `plugin/src/manager.rs` | `runtime.rs:1065`（另有测试构造 9 处） | 单装配根持有 |
| `CapabilityRegistry`（canonical） | `runtime/src/canonical/capability.rs:49` | `runtime.rs:1060` | 单装配根持有 |
| `CapabilityRegistry`（重复） | `plugin/src/registry.rs:122` | 无外部构造 | **plugin 内部使用**（`manager.rs:32` 导入，id→owner 索引） |

关键修正（相对 contract 旧描述）：plugin 侧 `CapabilityRegistry` **不是完全无使用**——`PluginManager` 内部用它做 id→owner 索引；R2-3 删除前需把该索引并入共享 `ExtensionDescriptor.capabilities`，属"内部重构"而非纯删。

## 3. 扩展点登记（contract）

6 项（实测）：`module_trait` / `plugin_trait` / `module_registry` / `capability_registry` / `plugin_manager` / `sub_loop`；R2 完成后待更新为单表描述，新增仍须 ADR。

## 4. 批次计划（file-level 范围）

| 批 | 范围 | 冲突注意 |
|---|---|---|
| R2-1 | 本清单（+ 消费方复核） | 不改码 |
| R2-2 | `foundation/plugin` additive：`ExtensionKind/Descriptor/Catalog` | 与 plugin 内部无关，可先做 |
| R2-3 | `PluginManager` 内部索引迁移 + 删 plugin `CapabilityRegistry`；`BehaviorRegistry` 实现 `ExtensionCatalog` | 触 `plugin/src/{manager,registry,lib}.rs` + `runtime/src/canonical/{module,runtime}.rs` |
| R2-4 | 改名：`AgentModule`→`BehaviorModule`（9 文件）、`ModuleRegistry`→`BehaviorRegistry`；删两 alias | **生物线在途的 5 个 runtime-assembly 文件收口后**再动 |
| R2-5 | 契约更新（extension_points/debt/concepts 清零） | Wave 收口 |

## 5. 交给 owner 的问题（裁决 ADR-0008 时）

1. canonical 名定为 `BehaviorModule`/`BehaviorRegistry`（现值契约方向）还是 `AgentModule`（旧名 75 处更熟）？ADR 建议前者（一个概念一个名字，保留 git 历史可查）。
2. 删 plugin `CapabilityRegistry` 是否批准（合并进 `ExtensionDescriptor.capabilities`）？
3. R2-4 改名批次是否等生物线当前批收口后执行（避免 5 文件冲突）？
4. `PluginManager` 是否保留原名（建议保留，作为 ABI 的配置期角色实现名）。

## 6. 证据边界（0 装）

- 静态证据，未跑运行时；plugin `CapabilityRegistry` 的"内部使用"已点名文件行，删除前仍须 `cargo check` 验证。
- 未修改任何 crate 源码。

## 7. R2-3c 漏检与修复（2026-10-20，commit ae1f8779）

R2-3c（`2da976ee`）按"外部 0 调用"grep 删除 `PluginManager::capabilities()`，但
`runtime/src/canonical/runtime.rs:704` 的 `Debug` 实现仍在调用 → `cargo check -p apeireth-runtime
--all-targets` 报 E0599（private field, not a method）。本轮 R7-2 下游验证时发现，改用公开
`records().len()`（语义等价，仅 Debug 计数，无行为影响）。

**教训（写入 R2 取证口径）**：
1. "外部 0 调用"结论必须由编译验证（`cargo check --all-targets`）兜底，不能只凭 grep 计数；
2. fast gate（`verify_changed.py`）未能拦截该回归的原因待查（反向依赖覆盖口径可能未含本次
   trait/impl 面），在 Wave 收口效率线复盘中复核；
3. R2-3 批次计划已列"触 `runtime/src/canonical/runtime.rs`"，但 R2-3c 实施时未按 file-level
   checklist 全量编译 → 后续 R2 批次收口固定跑 `cargo check --all-targets` + 相关测试。
