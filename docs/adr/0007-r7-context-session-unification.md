# 0007. R7 上下文/提示/会话单一化

- **Status**: Proposed（架构线草案 2026-10-20；待 owner 裁决后 Accepted 并进 Wave 3 施工）
- **Date**: 2026-10-20
- **Depends on**: R2 扩展 ABI（ADR-0008 草案）——**R2 未 Accepted 前不施工**；R7 的 trait/类型收敛需要 R2 的扩展点边界先定
- **Owners**: 架构线（spec/验收）· 各线实施（分 R7-1..R7-4 批）
- **契约影响**: 类型/职责归属变更（foundation↔engine），无新依赖边；新增仅删除/重命名
- **例外关联**: 删除类（PromptCacheStabilizer / 闲置 scheduler）必须先清单+归档，不允许静默删
- **预算净额**: 0（净删除重复实现/重命名；无新增结构）

## 1. 背景（2026-10-20 实测，不沿用 2026-10-18 审计数字）

> 逐项清单（scheduler 接线点 / SessionStore trait 实现 / ContextBlock 使用面 / 待 owner 问题）见 [docs/architecture/r7-inventory-2026-10-20.md](../architecture/r7-inventory-2026-10-20.md)。

ADR-0002 R7 目标："`ContextAssembler`/`ContextBlock`/`assemble_tiered` 各收敛为一份；`SessionStore` 3 份收敛；调度器 4 份→1-2 条活路径；`PromptCacheStabilizer` 要么接线要么删"。

当前实测（`git grep`，2026-10-20）：

| 面 | 现状 | 证据 |
|---|---|---|
| Context 类型 | **2 个 `ContextBlock`**：`engine/organ/src/context_assembly.rs:45` 与 `foundation/orchestration/src/context_budget.rs:34`；`runtime/src/canonical/execute.rs:83` re-export orchestration 那个，`organ/src/lib.rs:79` 导出自己那个 | 双路径 |
| Context 装配 | `ContextAssembler`/`assemble_tiered` 只在 organ/context_assembly.rs（已收敛）；orchestration 侧是 budget/assemble_messages 另一条路 | 待合并 |
| 会话 | **3 个 `SessionStore` trait**：`engine/runtime/src/canonical/session.rs:802`、`engine/memory/src/session_lifecycle.rs:259`、`engine/memory/src/session_note.rs:54`；24 个 src 文件引用 | 同名三义 |
| 调度器 | **5 个 `*Scheduler` struct**：`CallScheduler`（orchestration，4 src 引用）、`HeartbeatScheduler`（runtime，4）、`NightwatchIdleScheduler`（runtime-assembly，3）、`RecalibrationScheduler`（memory，2）、`CognitiveQuotaScheduler`（orchestration，2）——均有 src 引用，**不代表都是活跃生产路径**，需 R7-1 逐条取证 | 待定活跃面 |
| PromptCacheStabilizer | 仅定义（orchestration/prompt_stabilizer.rs:44）+ lib 导出 + organ 注释引用；**无生产消费点**（grep src 仅 3 文件且 2 为注释/导出） | 未接线 |

## 2. 决策（Proposed）

### 2.1 Context 单一化（R7-2）

- **单一类型家**：`ContextBlock`/`ContextBudget` 收敛到 **foundation/orchestration**（最底层共享契约；避免 runtime→organ 反向依赖）；
- **单一装配器**：`ContextAssembler`（tiered/budgeted 两种策略合并为一个入口，策略枚举）保留在 `engine/organ`，消费 orchestration 的类型；删除 organ 自己的 `ContextBlock` 定义；
- 删除 `runtime/src/canonical/execute.rs:83` 的 re-export，调用方直接引 orchestration；
- 验收：`git grep 'struct ContextBlock' -- crates/` = **1**；`assemble_tiered` 公开入口 = 1；organ golden 测试逐字节不变。

### 2.2 会话单一化（R7-3）

- **唯一公共 `SessionStore`** = `engine/runtime/src/canonical/session.rs`（canonical 会话契约）；
- memory 两个同名 trait **重命名**为域内名：`SessionLifecycleStore`（session_lifecycle.rs）/`SessionNoteStore`（session_note.rs），保留职责差异并文档说明；二者不再是 "SessionStore" 同名三义；
- `SqliteSessionStore` 只实现 runtime 的 `SessionStore`（+ 域 store 若需要）；
- 迁移 24 个引用分两批（先新增别名并存 → 再迁调用方 → 最后删旧名）；
- 验收：`git grep 'trait SessionStore' -- crates/` = **1**；域 store 名各 1；无同名歧义；现有 session 测试全绿。

### 2.3 调度器与提示缓存（R7-4）

- **R7-1 先出"活跃路径清单"**：5 个 scheduler 逐条给证据（生产触发点/装配根注册/遥测或测试），判定 `active` / `dormant`；
- 目标：**活跃路径 ≤ 2**（预期 CallScheduler + HeartbeatScheduler），其余 dormant 二选一：①接到真实触发点（须有测试+观测证据）②归档到 `legacy/archived/schedulers/<name>/` + MANIFEST（可逆）；不允许"留着但不接线"；
- **PromptCacheStabilizer**：二选一——①接入 canonical prompt 组装路径（须有 cache-prefix 稳定性测试+生产消费点证据）②删除（git rm + 记录"未接线无消费者"）；**禁止仅保留导出**；
- 验收：`git grep 'struct .*Scheduler' -- crates/` 中 active 路径 ≤2 且清单可复核；PromptCacheStabilizer 有生产消费点或不存在。

## 3. 迁移批次（每批 1 commit）

| 批 | 内容 | 验证 |
|---|---|---|
| R7-1 | 活跃路径/消费点清单（不改代码） | 每项证据（文件:行/装配根注册） |
| R7-2 | Context 类型收敛（orchestration 家 + organ 装配器 + 删 re-export） | ContextBlock 定义=1、organ golden 逐字节 |
| R7-3 | SessionStore 命名收敛（别名并存→迁调用方→删旧名） | trait 定义=1、session 测试绿 |
| R7-4 | scheduler 活跃面 ≤2 + PromptCache 接线或删 | 清单可复核、无"留而不接" |

## 4. 风险与回滚

- R7 触面广（24 个 session 引用 + runtime/organ/orchestration 公共类型）→ 每批可独立 revert；删除类先归档（scheduler/PromptCache 进 legacy/archived + MANIFEST）；
- Context 类型搬家可能引起 organ/runtime 循环依赖 → 只允许"organ→orchestration"方向；若需 runtime 直接用类型，经 foundation 契约，不得反向；
- 不承诺行为变化：默认路径输出逐字节不变（golden 测试守门），0 默认开关变化。

## 5. O-6 三阶审查

- **总体最优**：R7 消除"同名三义/双类型/五调度器"，是后续所有上下文类改动的返工率来源；拒 ①只清注释不改结构（歧义仍在）②等 R2 全完成再设计（Wave 3 空转）——故本 ADR 可先 Proposed、施工在 R2 Accepted 后。
- **系统最优**：类型收敛到最底共享层，装配器留在 organ（职责=认知装配），会话契约留在 runtime（职责=会话生命周期）；拒 ①全部搬 runtime（organ 反向依赖）②全部搬 organ（runtime 反向依赖）。
- **架构最优**：先取证活跃面再删（dormant≠死），删除可逆归档；拒 ①按旧审计数字直接删（可能误删在用）②保留"留而不接"（假接线）。
