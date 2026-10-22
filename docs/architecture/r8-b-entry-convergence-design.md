# R8-B 入口收敛设计候选（未实施）

> 状态：设计候选 **+ R8-B1 已交付（2026-10-21，§7）**；R8-B2/B3 与 R8-C/D 仍 NOT IMPLEMENTED。
> 目的：把「治理入口 3 → 1（`GovernanceChain::run`）」这个目标拆成可验证的批次，
> 并如实登记**不能机械替换**的结构原因，供安全线（R8-B/C/D 的既有归属）或架构线
> 接续实施时直接使用。
> 依据：ADR-0010 §4 R8-B、ADR-0010 §9；证据命令见 §6。

## 1. 现状（2026-10-21 实测）

### 1.1 已存在的两级治理面

| 级 | 载体 | 粒度 | 现状 |
| --- | --- | --- | --- |
| 回合级骨架 | `crates/foundation/governance/src/chain.rs` `GovernanceChain::run` | 一次 run = 六段（Gate/Deliberation/HumanAuthority/ApprovalClosure/AuditAppend/Reflection/SelfDisable） | R8-A 已交付（`fd74a669`），**生产零调用** |
| 动作级策略 | `crates/foundation/governance/src/lib.rs` `GovernancePipeline`（:534-604） | 每个动作（能力分发/模型补全）一次 `evaluate` | PRODUCTION WIRED（runtime 内） |

`GovernancePipeline` 已实现 `GovernanceHook`（`name() = "pipeline"`，`evaluate_verbose`
顺序执行 hooks、首个非 allow 短路）。因此 `GovernanceChain::new(Arc::new(pipeline))`
**今天就能编译**——R8-B 不需要改 governance crate 的 Gate 面。

### 1.2 三条入口/消费面（实测锚点）

1. **CLI 装配根**（`crates/adapters/cli/src/lib.rs`）：
   - `:1188 fn build_production_governance_parts_from_env() -> (GovernancePipeline,
     Arc<Mutex<PermissionPolicy>>, Arc<BehaviorChainGuardHook>)`；
   - `:1550` 取三件；`:1559-1564`
     `PermissionPresetGovernanceHook::new(Arc::new(governance), sessions.store())`
     包一层后 `builder.with_governance(Arc::new(preset_governance))`；
   - `guard_hook` 另作副作用注入（`:1555` 免疫记忆、`:1638-1641` dataset recorder、
     `:1709` `.with_guard(...)`），**不是** runtime governance 本体；
   - 构造器共 5 个（`:566` / `:835` / `:870` / `:1108` / `:1188`），测试直接消费
     `build_production_governance_from_env()`（如 `tests/production_governance.rs`）。
2. **gateway 审批归属面**（`crates/adapters/gateway/src/canonical_entry.rs:1139`）：
   `may_resolve_approval(identity.role, &identity.user_id, subject)` —— 这是**策略函数**
   （角色×归属闸），不是 pipeline 评估；对应 `gateway.user_action` 审计。
3. **IM 审批审计对**（`crates/adapters/gateway/src/im_approval.rs`）：
   `commit_approval_audit_pair` / `ApprovalAuditPair` / `ApprovalAuditSink`
   （`:23` imports、`:218/:245/:250` commit、`:360/:362`、`:493` 构造 pair）——
   自有一段审计闭环，未经过 `AuditHashChain`。

### 1.3 runtime 内的动作级调用

`crates/engine/runtime/src/canonical/runtime.rs`：字段 `governance: Arc<dyn GovernanceHook>`
（`:330`）、getter（`:387-389`）、`with_governance`（`:801`）、默认
`DenyUnconfigured`（`:750`）；调用点在 guard 路径
`crates/engine/runtime/src/canonical/module.rs:710` 与
`crates/engine/runtime/src/canonical/subloop.rs:260/:460`。**粒度 = 每个动作一次**。

## 2. 为什么不能机械替换

1. **粒度不一致**：chain 的 `run` 是回合/请求级；runtime 现在是动作级。
   若直接把 runtime 的 `governance` 换成 chain 适配器，审计/反思次数会从
   「每回合 1 条」变成「每个动作 1 条」，属于可观测行为变化，须显式裁决。
2. **预设包装层**：`PermissionPresetGovernanceHook`（`permission_preset.rs:129`）
   在 pipeline 外再叠会话预设；收敛点是它，不是原始 pipeline。
3. **gateway 的两段不是 pipeline**：`may_resolve_approval` 是策略函数，
   IM 审计对是另一条审计链。把它们并入 chain 需要 `AuditSink` 抽象先落地
   （R8-A 已留 `AuditSink` 端口，但生产持久实现未接线）。
4. **`DenyUnconfigured` fail-closed 默认**：任何替换都必须保持「未配置 = 拒绝能力」
   的现有语义（`runtime.rs:750`，测试 `default_governance_is_fail_closed_for_capabilities`）。

## 3. 候选方案

### 方案 A（最小风险）：入口链 + 保留动作级 pipeline

- 在 CLI/gateway 的回合入口先跑 `chain.run`（Gate = pipeline），runtime 继续持有
  pipeline 做动作级判定。
- 优：行为面几乎不变；审计可先只记回合级。
- 劣：pipeline 被评估两次（入口一次 + 每个动作一次），且「单点」只对入口成立；
  guard 副作用（免疫记忆/dataset）与两处审计可能重复。**不推荐**作为 R8-B 终态。

### 方案 B（推荐）：chain 作为 runtime 的唯一治理面（动作级语义不变）

- 给 chain 增加一个 `GovernanceHook` 适配器（新类型，如
  `ChainGovernanceHook`）：`evaluate_verbose` = `chain.run` 后把 `ChainOutcome`
  映射回 `GovernanceVerdict`（Allowed→Allow、Denied→Deny、RequireApproval→
  RequireApproval、NotConfigured→Deny）；`approval_resolved` 转发给 gate。
- runtime 侧继续只认 `Arc<dyn GovernanceHook>`（**零 runtime 改动**）；CLI 装配根把
  `PermissionPresetGovernanceHook(Arc<ChainGovernanceHook>)` 换入。
- 审计：`AuditSink` 用进程内/文件持久实现，一条动作一条审计；默认保持现有
  「只读不落盘」行为直到持久 sink 落地（R8-B3）。
- 优：单点真实成立（每个动作都经过 chain 六段），runtime 接口不变，回滚=换回
  `Arc<GovernancePipeline>`。劣：审计体积增大；每动作 Reflection 需默认关闭
  （`ImmediateReflectionScheduler` 仍为进程内 token，不落盘）。

### 方案 C（分阶段妥协）：入口链 + 持久审计先行

- 先只做 `AuditSink` 持久实现与 IM 审计对迁移，chain 暂时只做回合级入口；
  动作级收敛留 R8-C/D。优：审计单一真源先行，风险小；劣：入口仍非单点。

**建议路线**：B1（`ChainGovernanceHook` 适配器 + 映射测试）→ B2（CLI 装配根切到
`ChainGovernanceHook`，保持预设包装与 guard 副作用；既有 CLI 治理测试不改期望）→
B3（持久 `AuditSink` + IM 审计对经 chain 段；依赖 owner 对审计落盘介质裁决）。
若 owner 要求「先止血」而非收敛，则走 C。

## 4. 批次与验收（建议）

| 批次 | 交付 | 验收 |
| --- | --- | --- |
| R8-B1 | `ChainGovernanceHook`（governance crate，additive）+ 映射/短路/默认失败测试 | `cargo test -p apeireth-governance --lib`；新适配器测试全绿 |
| R8-B2 | CLI 装配根切换（`crates/adapters/cli/src/lib.rs` :1550-1564 一带） | 既有 `tests/production_governance.rs` 等不改期望全绿；`grep` 证据：runtime governance 构造点仅一处经 chain 适配器 |
| R8-B3 | 持久 `AuditSink` + IM 审批审计对并入 | `cargo test -p apeireth-gateway`；审计链可 `verify()`；wire 台账 `--verify` 绿 |
| 文档 | ADR-0010 §6 R8-B 勾选、§9 记录、handbook | `docs_truth.py --verify --check` exit 0 |

风险与回滚：R8-B1 是纯新增类型（不改既有路径），可独立合入并回滚；
R8-B2 行为敏感（CLI 是共享热点，按 line-coordination 与生物线协调后再动），
回滚=恢复 `Arc<GovernancePipeline>` 包装一行。

## 5. 未决问题（需 owner/安全线裁决）

1. 动作级审计体积：每条动作一条审计 vs 仅拒绝/审批事件——默认口径？
2. 持久 `AuditSink` 落盘介质（文件追加 / SQLite / 委托 gateway）与保留期。
3. IM 审计对是否并入 `AuditHashChain`（会改变现有审计表结构）。
4. `ChainConfig.human_authority_signatures` 从哪一层取（env / 会话预设 / 主人确认）。
5. R8-B2 是否允许 CLI 与生物线 R1b-2c（审批面 publish）同窗口施工——按线协调契约。

## 6. 证据命令

```powershell
git grep -n 'GovernancePipeline' -- crates/foundation/governance/src/lib.rs
git grep -n 'build_production_governance' -- crates/adapters/cli/src/lib.rs
git grep -n 'with_governance(' -- crates/engine/runtime/src crates/adapters/cli/src
git grep -n 'may_resolve_approval\|commit_approval_audit_pair' -- crates/adapters/gateway/src
git grep -n 'struct PermissionPresetGovernanceHook' -- crates
git grep -n 'struct BehaviorChainGuardHook' -- crates
```

以上命令 2026-10-21 实测；§1–§5 为设计候选（未写生产代码）；R8-B1 适配器已交付
（§7），R8-B2/B3 与 R8-C/D 仍 NOT IMPLEMENTED。

## 7. 交付记录

### 2026-10-21（R8-B1）

- **交付**：`crates/foundation/governance/src/chain.rs` 新增 `ChainGovernanceHook`
  （`Arc<GovernanceChain>` 适配为 `GovernanceHook`）：`name() = "governance_chain"`；
  `evaluate_verbose` = `chain.run` 后映射 `ChainOutcome` → `GovernanceVerdict`
  （Allowed→Allow、Denied→`gate: <reason>`、RequireApproval→RequireApproval、
  NotConfigured→`not configured at <phase>`）；`approval_resolved` 经新增
  `GovernanceChain::gate()` 访问器转发给 gate。附 `Debug` 实现（只报 gate 名）。
- **性质**：纯新增类型 + 访问器，**零生产接线**（R8-B2 才切装配根）；运行时接口
  不变，回滚 = 删除该类型与一行导出。
- **实测**：`cargo test -p apeireth-governance --lib --locked -j 2` = 209 passed /
  0 failed（新增 5 条适配器测试）；`cargo clippy -p apeireth-governance --all-targets
  --locked -- -D warnings` exit 0；`cargo fmt -p apeireth-governance --check` 0 diff。
- **如实边界**：无生产装配构造 `ChainGovernanceHook`；`Runtime` 仍持
  `Arc<GovernancePipeline>` 包装；R8-B2（CLI 装配根切换）与 R8-B3（持久
  `AuditSink` + IM 审计对）未实施；未跑全 workspace 门禁（本批仅 governance 面）。

