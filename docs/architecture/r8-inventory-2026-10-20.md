# R8 取证包（2026-10-20）

> **归属**：架构线取证 / spec（ADR-0002 §R8：单一治理管线——同一治理语义只有一个执行入口）。
> **实施线**：安全线主笔（ADR-0002 主笔线口径：R8 = 安全线）；架构线只定契约与边界。
> **快照**：HEAD `d87ea36f`（ADR-0009 提交后，2026-10-20）。工作树另有并行 SG-25 工具生态线
> 在途改动（`canonical.rs` / `bio_production_wiring.rs` / `production.rs` 及新增 probe 文件），
> 未纳入本证据；本线新增未跟踪文件仅 `nervous_*.rs` 三件。
> **口径**：所有数字可由 §9 命令复核；四级口径含义见
> [system-capabilities.md §0](../01-architecture/system-capabilities.md)；本文不发布测试数字（归
> `docs/03-reference/test-baseline.md`）。
> **前置证据**：[governance-chain-drill-2026-10-20.md](../04-internal/governance-chain-drill-2026-10-20.md)
> （安全线 B7 合练，commit `1f65852b`）——本文 §3 的现状表直接引用该演练结论并逐条标注生产调用面。

## 1. 结论摘要

1. **六段治理语义各自能跑，但没有单一编排**：AND 门（gate）→ 审议（rubric）→ 人类权威多签
   （onion）→ 审批审计对（approval_closure）→ 审计哈希链（audit）→ 自停用审计（core::SelfDisableAudit）
   六段都有实现与库内测试，合练集成测试
   `crates/foundation/governance/tests/governance_chain_drill.rs` 三路径全通，但**库内不存在把六段串起来的
   编排函数**——串接由调用方（CLI 装配、gateway 审批面）各自完成（§3）。
2. **生产装配面是分叉的**：`GovernancePipeline` 的唯一 hook 序列定义在
   `crates/foundation/governance/src/lib.rs:522`（`evaluate` 顺序执行、首个非 allow 即停），
   但它被三处分别组装：`crates/adapters/cli/src/lib.rs:566`（`build_production_governance`）、
   `:1108`（`from_env`）、`:1021`（preset 加成），运行期注入点 `:1564`
   `builder.with_governance(...)`；gateway 审批面走另一条解析路径
   `crates/adapters/gateway/src/canonical_entry.rs:1139` `may_resolve_approval(...)`，
   IM 审批落账走 `crates/adapters/gateway/src/im_approval.rs:250/:362`
   `commit_approval_audit_pair(...)`。**没有任何一个 crate 拥有「治理一轮完整生命周期」的函数**。
3. **多签生产调用为 0**：`multi_sign` 字段全库生产调用点为 `None`
   （`crates/foundation/core/src/onion.rs:83`、`MultiSignPolicy` `:176`），
   `HumanAuthority::verify_multisig`（`crates/foundation/core/src/onion.rs:94`）只在库内测试与
   `onion_gate.rs:733` 自证里出现；physical/cryptographic 多签明确留 v2.1
   （`crates/foundation/core/src/onion_gate.rs:16` 注释）。
4. **反思期没有常驻调度**：`HOLD_DELIBERATION_TIMEOUT_MS = 60_000` 由调用方注入（drill §5-⑥），
   库内无计时器/调度器；非 v2.1 autopilot 面不默认运行。
5. **审计链是内存链**：`AuditHashChain`（`crates/foundation/governance/src/audit.rs:104`）持有
   `Vec` 内存链；生产级持久 `ApprovalAuditSink` 只在 gateway IM 审批路径接线（§3），
   runtime 侧审批不落链。
6. **席位矩阵与 MEWG 权重仍是注释态**：风险→席位矩阵 7/5/3/1/0
   （`crates/foundation/core/src/gate.rs:50-68`）与 MEWG 加权
   （`crates/foundation/governance/src/rubric.rs`，自述"不接循环"）未进入任何生产判定循环。

## 2. 治理面资产清单

| 层 | 资产 | 位置 | 归属 |
|---|---|---|---|
| AND 门 | `ActionGuard::check_action` / `check_ha` | `crates/foundation/core/src/gate.rs:126-198` | foundation/core |
| 审议 | `Ballot` / `synthesize` / `hold` | `crates/foundation/governance/src/rubric.rs` | foundation/governance |
| 人类权威 | `HumanAuthority::verify_multisig` / `MultiSignPolicy` | `crates/foundation/core/src/onion.rs:83-176` | foundation/core |
| 审批审计对 | `ApprovalAuditPair` / `commit_approval_audit_pair` / `ApprovalAuditSink` | `crates/foundation/governance/src/approval_closure.rs:227-390` | foundation/governance |
| 审计链 | `AuditHashChain::append/verify` | `crates/foundation/governance/src/audit.rs:104-169` | foundation/governance |
| 自停用审计 | `SelfDisableAudit` | `crates/foundation/core/src/lib.rs:1601+` | foundation/core |
| 唯一 hook 序列 | `GovernancePipeline`（`new/with/evaluate`） | `crates/foundation/governance/src/lib.rs:522-588` | foundation/governance |
| 审批状态机 | `ApprovalDecision` / `ApprovalStatus` | `crates/engine/runtime/src/canonical/approval.rs:24-80` | engine/runtime |
| 审批消费者 | `may_resolve_approval` | `crates/foundation/governance/src/user_access.rs:254`；生产调用 `gateway/canonical_entry.rs:1139` | adapters/gateway |
| 审批落账 | `commit_approval_audit_pair` 生产调用 | `crates/adapters/gateway/src/im_approval.rs:250/:362` | adapters/gateway |
| 权限钩子 | 各工具 `*ApprovalHook: impl GovernanceHook` | `apply_patch_tool.rs:458`、`git_write.rs:1239`、`doc_ops.rs:4338`、`worktree_apply.rs:782`、`skill/distill.rs:505`、`sheet_ops.rs:3139`、`device_control.rs:1709`、`content_feed/mod.rs:103`、`organ/care/tool.rs:93`、`runtime-assembly/{task_board_tool.rs:128,play_tools.rs:1059,permission_preset.rs:255,onion_layer.rs:76}` | 各工具 crate |

## 3. 六段现状台账（实现 / 生产调用 / 测试）

| # | 段 | 实现 | 生产调用面 | 库内测试 | 判定 |
|---|---|---|---|---|---|
| 1 | AND 门（V1+V2+V3） | `ActionGuard::check_action` | 由 CLI/gateway 装配面调用；runtime 默认 `DenyUnconfigured`（`runtime.rs:750`） | 有 | **PRODUCTION WIRED**（分叉装配） |
| 2 | 审议（rubric） | `Ballot/synthesize/hold` | 无生产调用循环；drill 显式串接 | 有 | **IMPLEMENTED** |
| 3 | 人类权威（多签） | `verify_multisig` 字符串票占位 | `multi_sign` 恒 `None`，零生产调用 | 有 | **IMPLEMENTED**（v2.1 真多签 NOT IMPLEMENTED） |
| 4 | 审批审计对 | `commit_approval_audit_pair` | gateway IM 审批：`im_approval.rs:250/:362`；runtime 审批不落对 | 有 | **PRODUCTION WIRED**（仅 IM 路径） |
| 5 | 审计哈希链 | `AuditHashChain` | 仅内存链消费；无持久 sink 默认接线 | 有 | **IMPLEMENTED**（持久化 NOT IMPLEMENTED） |
| 6 | 自停用审计 | `SelfDisableAudit` | 装配面未见生产消费（drill 三路径在测试内验证） | 有 | **IMPLEMENTED** |
| — | 反思期 | 无调度实现 | 无 | — | **NOT IMPLEMENTED** |

## 4. 缺口（G1–G6）

| 缺口 | 事实 | 影响 |
|---|---|---|
| **G1 无单一编排** | 六段串接由调用方各自完成；CLI 三处组装 pipeline，gateway 另起审批解析路径 | 同一治理语义在三个入口语义漂移；新增入口必须重复串接，漏一段不报错 |
| **G2 多签空转** | `multi_sign` 全库 `None`、`verify_multisig` 零生产调用 | 高风险动作的人审强度实际只有"字符串票"占位；v2.1 前必须显式标注 |
| **G3 反思期缺位** | 无调度器、`HOLD_DELIBERATION_TIMEOUT_MS` 由调用方注入 | 审批挂起无自动到期；反思期审计默认不运行 |
| **G4 审计落地分叉** | `ApprovalAuditPair` 仅 IM 路径落；`AuditHashChain` 仅内存 | 无法回答"这次审批进了哪条链"；跨入口审计不可复核 |
| **G5 席位/权重注释态** | 风险→席位矩阵、MEWG 加权均注释态 | 议事规则无法从文档变成运行时判定 |
| **G6 治理 CI 面缺口** | 29 个 workflow 中无"治理链端到端"执行面（drill 是测试不是管线） | 六段任一退化只能靠库内单测兜底 |

## 5. 可复用资产与边界（避免第二套治理）

1. `GovernancePipeline` **是**唯一的 hook 序列定义（`governance/src/lib.rs:522`）；R8 **不得**新建
   第二个 pipeline / 第二套 hook 顺序（`approval_policy.rs:6` 与 `permission.rs:10` 已明确该口径）。
2. `ActionGuard` 是**判定函数**不是流程；R8 编排把它当第一段调用，不重写 V1/V2/V3 逻辑。
3. `ApprovalAuditPair` 是**原子双写**语义（落地或整对回滚，`im_approval.rs` 注释）；R8 编排复用，
   不新建第三份审计记录。
4. `RuntimeEventSink` / `EventBus` 是**观测**面，不是治理判定面；R8 不把审批决策塞进事件总线。
5. `ApprovalDecision` / `ApprovalStatus` 是 runtime 侧状态机；R8 编排只消费其状态，不复制枚举。
6. 风险→席位矩阵与 MEWG 是**议事层**；R8-A 只定义接缝（trait），不把加权投票硬编码进判定。

## 6. 设计约束（R8 契约候选的依据）

| # | 约束 |
|---|---|
| C1 | 编排器落在 `foundation/governance`（唯一 hook 序列的家），**不新建 crate**、不新增外部依赖 |
| C2 | 六段以既有类型为输入；对外部效应（多签验证、审计落地、反思调度、时钟）**只依赖 trait**，由 runtime / gateway 注入实现 |
| C3 | 编排器不改变 `GovernancePipeline` 语义，只在其前后串段；`evaluate` 仍是唯一 hook 顺序 |
| C4 | 不许把治理判定塞进事件总线/观测面；审计走 `ApprovalAuditSink`，观测另走既有 sink |
| C5 | v2.1 真多签显式标 NOT IMPLEMENTED；R8 只留 `MultiSignVerifier` trait 接缝，不伪造签名 |
| C6 | 归属：R8 实施 = 安全线；架构线只出 spec + 验收标准；跨线装配改动按 line-coordination 契约登记 |

## 7. 批次与验收（供 ADR-0010 引用）

| 批次 | 内容 | 验收 |
|---|---|---|
| **R8-A** | `foundation/governance` 新增编排器：`GovernanceChain`（阶段枚举 + `run` 单入口 + 4 个注入 trait：`MultiSignVerifier` / `AuditSink`（复用 `ApprovalAuditSink`）/ `ReflectionScheduler` / `ChainClock`） | 库内集成测试串通六段；每种注入缺失时显式 `NotConfigured` 而非静默跳过；不新建 crate/依赖 |
| **R8-B** | runtime / gateway 装配改走编排器：CLI `build_production_governance*` 产出物作为 hook 序列，由编排器第一段调用；gateway 审批解析复用同一编排器 | 现有 CLI/gateway 审批测试全绿；入口数 3→1（`build_production_governance*` 收敛为构造 hook 序列的工厂，不再是流程入口） |
| **R8-C** | 反思期调度接缝落地：`ReflectionScheduler` 有默认实现（进程内延迟队列）+ 到期审计；v2.1 autopilot 保持 NOT IMPLEMENTED | 到期测试可控时钟；默认关闭不改变现行为 |
| **R8-D** | 台账与 CI：治理面状态进 `docs/status/wiring.yaml`；新增 `governance-chain` 集成测试入 CI | wiring drift=0；CI 红即失败 |

## 8. 未决问题（要 owner / 安全线裁决）

| # | 问题 | 默认建议 |
|---|---|---|
| Q1 | 反思期调度器归谁：gateway 常驻进程内 vs runtime 回合内 | 进程内默认实现，接口在 governance；归属安全线 |
| Q2 | 审计持久 sink 默认形态：sqlite vs 文件追加 | 复用 memory 的 sqlite 基础设施（不引新依赖） |
| Q3 | 多签签名来源：v2.1 前是否接入真实 Ed25519 | v2.1 前显式 NOT IMPLEMENTED；只在接口留缝 |
| Q4 | 三入口收敛是否包含 gateway `may_resolve_approval` | 包含（R8-B 的验收标准之一） |
| Q5 | MEWG 加权是否进入 R8 | 不进入（议事层，另立 ADR） |

## 9. 证据命令附录

```powershell
git grep -n "GovernancePipeline" -- crates ':!*/tests/*'
git grep -n "impl GovernanceHook for" -- crates
git grep -n "may_resolve_approval" -- crates
git grep -n "commit_approval_audit_pair" -- crates
git grep -n "verify_multisig" -- crates
git grep -n "multi_sign" -- crates ':!*/tests/*'
git grep -n "build_production_governance" -- crates
Get-ChildItem .github/workflows -Filter *.yml -Name | Measure-Object   # 29
```
