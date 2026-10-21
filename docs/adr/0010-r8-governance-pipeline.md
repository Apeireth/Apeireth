# 0010. R8 治理单一管线——六段编排 / 单一入口 / 审计可落地

- **Status**: Accepted（owner 2026-10-20 授权 Wave 4 按架构线建议推进；本文为 ADR-0002 已 Accepted 的 R8 方向的实施形状裁决，安全线实施期如发现条款不可行可在 R8-A 开工前提出修订）
- **Date**: 2026-10-20
- **Owners**: 架构线出 spec / 验收标准 / 契约面；R8-A..D 实施 = 安全线（ADR-0002 主笔线口径）
- **Supersedes**: —
- **契约影响**: `docs/architecture/contract.toml` §meta.status_note（追加 ADR-0010 一行）与
  §meta.target_architecture.waves（追加 "ADR-0010 R8（Wave 4）"）；`[extension_points]` 不新增登记
  （治理编排器是 foundation 内部面，不是扩展点）；依赖层边 ±0
- **例外关联**: —（R8-A 只在 `foundation/governance` 内新增文件，不加依赖边/不新增 crate）
- **预算净额**: 依赖边 ±0 / 开关 ±0 / crate ±0 / 文件尺寸 +（`foundation/governance` 新文件约 300-600 行，
  收口时登记 size 条目；不改既有文件预算）
- **证据包**: [r8-inventory-2026-10-20.md](../architecture/r8-inventory-2026-10-20.md)（HEAD `d87ea36f` 实测）
  + [governance-chain-drill-2026-10-20.md](../04-internal/governance-chain-drill-2026-10-20.md)（安全线 B7 合练）

## 1. 背景与问题

治理六段（AND 门 → 审议 → 人类权威多签 → 审批审计对 → 审计哈希链 → 自停用审计）**每段都已有实现，
但不存在把六段串成一轮生命周期的单一编排函数**（证据包 §1-§3）：

1. **编排分叉（G1）**：`GovernancePipeline` 是唯一 hook 序列（`governance/src/lib.rs:522`），但被
   CLI 三处分别组装（`cli/src/lib.rs:566/:1021/:1108`，注入点 `:1564`）；gateway 审批面另走
   `may_resolve_approval`（`gateway/canonical_entry.rs:1139`）与 IM 审计落账
   `commit_approval_audit_pair`（`gateway/im_approval.rs:250/:362`）。同一治理语义在三个入口各自串段，
   漏一段不会报错，新入口必须复制一遍串接。
2. **多签空转（G2）**：`multi_sign` 全库生产调用点为 `None`，`HumanAuthority::verify_multisig`
   （`core/src/onion.rs:94`）零生产调用；物理/密码学多签明确留 v2.1（`core/src/onion_gate.rs:16`）。
3. **反思期缺位（G3）**：无调度实现，`HOLD_DELIBERATION_TIMEOUT_MS = 60_000` 由调用方注入；
   反思期审计默认不运行。
4. **审计落地分叉（G4）**：`ApprovalAuditPair` 只在 gateway IM 路径接线；`AuditHashChain` 是内存链，
   生产级持久 sink 未默认接线。无法回答「这次审批进了哪条链」。
5. **席位/权重注释态（G5）**：风险→席位矩阵（`core/src/gate.rs:50-68`）与 MEWG 加权
   （`governance/src/rubric.rs` 自述"不接循环"）均未进入生产判定。
6. **治理 CI 面缺口（G6）**：29 个 workflow 中无治理链端到端执行面；六段退化只能靠库内单测兜底。

**为什么现在决策**：R8 是 ADR-0002 Wave 4 出口三条之一；六段实现已齐（安全线 B7 合练已跑通三路径），
缺的只是编排与入口收敛——这是纯加法形状，不需要改六段内部语义，风险面最小。若继续让每个入口各自串段，
后续每加一个入口（TUI / IM / 桌面）都会再复制一次并继续漂移。

## 2. 决策

**一句话**：在 `foundation/governance` 内新增**单一编排器** `GovernanceChain`，按固定六段顺序执行一轮
治理生命周期；对外部效应（多签验证 / 审计落地 / 反思调度 / 时钟）**只依赖注入 trait**；CLI 与 gateway
的治理入口收敛为"构造 hook 序列 / 调用编排器"。不新建 crate、不新增外部依赖、不改六段内部语义、
不做第二套判定逻辑。

### D1 编排器形状

- 新文件 `crates/foundation/governance/src/chain.rs`：
  - `GovernancePhase`（闭合 enum，顺序即语义）：`Gate` → `Deliberation` → `HumanAuthority` →
    `ApprovalClosure` → `AuditAppend` → `SelfDisable`；
  - `ChainOutcome`：`Allowed(ChainRecord)` / `Denied { phase, reason }` / `RequireApproval(ChainRecord)` /
    `NotConfigured { phase }`——**每段缺注入实现必须显式 `NotConfigured`，不得静默跳过**（0 装）；
  - `ChainRecord`：包含六段各自的判定产物引用/摘要 + `signature_mode`（`Placeholder` / `Cryptographic`，
    见 D6）+ 审计落地标记（是否入链 / 落在哪个 sink）；
  - `GovernanceChain::run(&self, request: &GovernanceRequest, ctx: &TurnSecurityContext) -> ChainOutcome`：
    唯一公共入口；内部每段只调用既有类型/函数（`GovernancePipeline::evaluate`、`rubric`、
    `HumanAuthority`、`commit_approval_audit_pair`、`AuditHashChain`、`SelfDisableAudit`），**不复制**判定逻辑。
- 位置约束：`foundation/governance` 内部；不挂公共 ABI 以外的面（本就属 governance 公共 API，非扩展点）；
  不新增 crate。

### D2 注入 trait（外部效应全部走接口）

| trait | 语义 | 默认实现 | 生产实现归属 |
|---|---|---|---|
| `MultiSignVerifier` | 校验人类权威签名；`Placeholder` 实现包装既有 `HumanAuthority::verify_multisig` | `PlaceholderVerifier`（显式标注占位） | v2.1 真 Ed25519（NOT IMPLEMENTED） |
| `AuditSink` | 审批审计对落地（复用 `ApprovalAuditSink` 语义：落地或整对回滚） | `InMemoryAuditSink`（= 现状） | runtime-assembly / gateway（R8-B） |
| `ReflectionScheduler` | 反思期调度：挂起审批到期、反思审计触发 | `ImmediateScheduler`（不延迟，等价现状） | R8-C：进程内延迟队列 |
| `ChainClock` | 时间源（`now_ms`） | `SystemClock` | 测试用可控时钟 |

- 四个 trait 均为**注入面**，不是第二套逻辑；chain 只依赖 trait，不依赖具体适配器 crate（依赖方向不变）。

### D3 入口收敛（R8-B）

- **唯一流程入口** = `GovernanceChain::run`。
- CLI：`build_production_governance*`（`cli/src/lib.rs:566/:1108`）收敛为**构造 hook 序列的工厂**
  （返回值仍是 `GovernancePipeline` / hook 集合），不再承担流程编排；`builder.with_governance(...)`
  注入点保留（runtime 侧仍只认识 `GovernanceHook`，不需要认识 chain——chain 在装配面先跑，或作为 hook
  的第一段执行，实施期由安全线按最小侵入选择并记录）。
- gateway：`may_resolve_approval` 保留为**策略函数**（身份判定，不是流程段）；IM 审批的
  `commit_approval_audit_pair` 调用改由 chain 的 `ApprovalClosure` 段发起（审计落账不再分叉）；
  入口数量 3 → 1。
- 既有 hook 序列语义不变：`GovernancePipeline` 仍是唯一 hook 顺序定义（`approval_policy.rs:6` /
  `permission.rs:10` 的口径继续成立）。

### D4 审计落地

- `AuditHashChain` 仍是链本体；持久化通过 `AuditSink` 注入实现，**默认仍是内存**（不改变默认行为）。
- 生产持久 sink 的实现放 **runtime-assembly / gateway**（安全线 R8-B 接线）：`AuditHashChain` 追加后
  由 sink 落地一份追加记录；不新引数据库依赖到 `foundation/governance`（跨层依赖会被架构守卫禁）。
- 自述可回答：`signature_mode` / 审计落地标记 / 各段判定摘要（不写"最近时间"，避免第二真源）。

### D5 反思期（R8-C）

- `ReflectionScheduler` 默认实现为**进程内延迟队列**（可控时钟测试）；到期动作 = 审批到期审计 +
  反思审计记录；`HOLD_DELIBERATION_TIMEOUT_MS` 由 chain 构造参数注入（不再散落在调用方）。
- v2.1 autopilot（常驻自主反思）显式 **NOT IMPLEMENTED**：本 ADR 只落接缝与默认关闭实现。

### D6 多签诚实口径

- `MultiSignVerifier::PlaceholderVerifier` 包装既有字符串票 `verify_multisig`；chain 在 `ChainRecord`
  标注 `signature_mode = Placeholder`。真 Ed25519 多签 = v2.1，**NOT IMPLEMENTED**，R8 不伪造签名、
  不把占位写成真多签（0 装反向）。
- `multi_sign` 字段由装配面注入（生产仍可为 `None`；`None` 时 `HumanAuthority` 段按现有语义处理并
  在 `ChainRecord` 标注 `multi_sign = None`）。

### D7 归属与批次

| 批次 | 内容 | 实施线 |
|---|---|---|
| **R8-A** | `chain.rs`（D1+D2 四 trait + 默认实现）+ 库内集成测试（六段各路径） | 安全线 |
| **R8-B** | CLI / gateway 入口收敛；持久 `AuditSink` 接线；入口数 3→1 验证 | 安全线 |
| **R8-C** | `ReflectionScheduler` 进程内实现 + 到期审计；autopilot 保持 NOT IMPLEMENTED | 安全线 |
| **R8-D** | 治理面进 `docs/status/wiring.yaml`；`governance-chain` 集成测试入 CI | 安全线 |

### 不改什么

- 不改六段内部语义与既有公开 API 的语义（只做编排与接缝）；不新建 crate / 外部 dep；不改层边；
  不新增开关；不把治理判定塞进事件总线/观测面；不改 `workspace.version` / LOCKED 5 项；
  不推进 MEWG 加权与席位矩阵进入判定循环（G5 另立 ADR）。

## 3. O-6 三阶审查（各含拒的 alternatives + 拒理由）

- **总体最优**：R8 用「加法编排 + 注入接缝」收口，是六段已齐后的最小改动路径；不重写六段、不动
  LOCKED 面、风险面集中在装配。拒「每个入口继续各自串段」——入口越多漂移越多，G1 已实证；
  拒「把六段合并成一个大函数」——破坏既有库内测试与分段可测性；拒「先做 G5 议事层」——议事层是
  v2.1 语义，先接会冻结未定规则。
- **系统最优**：`foundation/governance` 是唯一 hook 序列的家，编排器放这里不新增依赖边；外部效应
  （多签/审计/调度/时钟）全部走注入 trait，runtime / gateway 只注入实现，依赖方向不变。
  拒「放 runtime」——runtime 不是治理语义的家，放进去会让回合核心反向依赖治理编排；
  拒「新建 `apeireth-governance-chain` crate」——新增 crate 是结构成本，当前收益不成立；
  拒「把编排塞进 `GovernancePipeline` 自身」——pipeline 是 hook 序列不是生命周期，混在一起会让
  六段的注入与测试全部变形。
- **架构最优**：单一入口 + 显式 `NotConfigured` + `signature_mode` 标注三者组合，使"治理做了什么、
  哪段没配、签名是什么强度"三个问题都能从 `ChainRecord` 直接回答；审计落地走既有 atomic pair 语义，
  不造第三份记录。拒「隐式默认跳过缺实现」——0 装；拒「把 chain 做成公共扩展点」——先私有收口，
  稳定后再单开 ADR 讨论 ABI。

## 4. 迁移计划（分步、每步可验证）

1. **R8-A**：`chain.rs` + 四 trait 默认实现 + 六段集成测试（沿用
   `governance/tests/governance_chain_drill.rs` 的三路径，改为走 chain 单入口）。
   验证：`cargo test -p apeireth-governance --locked`；`cargo clippy -p apeireth-governance -- -D warnings`；
   `cargo fmt -p apeireth-governance --check`。
2. **R8-B**：CLI / gateway 接线与入口收敛；持久 `AuditSink` 落地。
   验证：既有 CLI 治理测试（`production_governance.rs` 等）与 gateway 审批测试全绿；入口 grep 证据
   （流程入口 = `GovernanceChain::run` 单点）。
3. **R8-C**：`ReflectionScheduler` 进程内实现 + 到期审计；默认关闭。
   验证：可控时钟到期测试；默认行为逐字节不变（现有测试不改期望）。
4. **R8-D**：wiring 台账 + CI。
   验证：`python scripts/wiring_report.py --verify` exit 0；新增 workflow 绿。

**并行线协调**：R8-A 只新增 `foundation/governance/src/chain.rs` 与测试，不碰在途 SG-25 线文件；
R8-B 触 CLI/gateway 时按 line-coordination 契约与全量线协调（CLI 是共享热点）。

## 5. 回滚

- R8-A：`git revert <commit>` —— 新文件删除、lib.rs 一行导出还原；六段行为不变（chain 未被装配面调用前
  是纯新增）。
- R8-B：入口收敛按 commit 粒度独立 revert；回滚后 CLI/gateway 回到各自串段（现状 G1 复现，但功能不丢）。
- R8-C/D：独立 revert；`ReflectionScheduler` 默认关闭，回滚不影响默认行为。
- 回滚后需重跑：`cargo test -p apeireth-governance`、`cargo test -p apeireth-cli --test production_governance`。

## 6. 验收

- [x] R8-A：`GovernanceChain`/`GovernancePhase`/`ChainOutcome`/`ChainRecord` 落地；四 trait 默认实现；
      六段集成测试（allow / deny / require-approval / not-configured / 审计落地 / 自停用）全绿
- [x] R8-A：`NotConfigured` 显式而非静默跳过；`signature_mode` 标注占位；不新建 crate/依赖
- [ ] R8-B：入口收敛为 `GovernanceChain::run` 单点（grep 证据）；CLI/gateway 既有治理测试全绿
- [ ] R8-B：审计落账不再分叉（gateway IM 与 runtime 审批都经 `ApprovalClosure` 段）；持久 sink 可配置
- [ ] R8-C：到期测试可控时钟；autopilot 保持 NOT IMPLEMENTED；默认关闭零行为变化
- [ ] R8-D：wiring.yaml 覆盖治理面；`governance-chain` 集成测试入 CI
- [x] LOCKED 5 项 0 触碰 / 0 新外部 dep / 0 新开关 / 0 新 crate / 六段语义不变

## 7. 后果（Consequences）

- 正面：治理一轮生命周期只有一个入口与一份 `ChainRecord`；新增入口只调用 chain 不再复制串段；
  审计可回答"进哪条链、签名什么强度、哪段没配"；为 v2.1 真多签/反思期留好接缝。
- 负面/接受的代价：`foundation/governance` 增加编排面（需安全线维护）；R8-B 触 CLI 共享热点，
  需与全量线排期；多签仍是占位、反思期默认不跑（如实标注，不假装）。
- 后续 stage：G5 议事层（席位矩阵/MEWG 加权）另立 ADR；v2.1 真多签替换 `PlaceholderVerifier`；
  R8-D 后评估治理面是否升级为可观测扩展点（需新 ADR）。

## 8. 未决问题（安全线 R8-A 开工前裁决）

| # | 问题 | 本文建议 |
|---|---|---|
| Q1 | chain 在装配面是"先跑再注入 runtime"还是"作为 hook 第一段" | 先跑再注入（最小侵入；runtime 不认识 chain） |
| Q2 | 持久 `AuditSink` 落 sqlite（runtime-assembly）还是文件追加（gateway） | runtime-assembly sqlite（复用既有 memory 基础设施，不引新依赖） |
| Q3 | 反思期调度器进程内 vs 常驻线程 | 进程内延迟队列（默认关闭；常驻留 v2.1） |
| Q4 | gateway `may_resolve_approval` 是否算流程段 | 不算（身份策略函数，保留） |
| Q5 | MEWG 加权是否进 R8 | 不进（G5 另立 ADR） |
| Q6 | 真多签时间点 | v2.1（NOT IMPLEMENTED 至彼时） |

## 9. 变更记录

### 2026-10-21（R8-A 实施归属临时调整 + spec 收窄）

- **归属**：安全线自 2026-10-20 21:00 起零提交（`git log` 实测），owner 先前授权 Wave 4「按建议做到底」；
  R8-A（纯库内骨架，不触 CLI/gateway）由架构线实施。R8-B/C/D（入口收敛、持久 sink、反思运行器）
  仍需安全线接管；接管时以本批 commit 为起点，不需要回滚。
  拒：无限等待（阻塞 Wave 4 收口）；绕开 ADR 直接改 CLI/gateway 热点。
- **spec 收窄 1（签名面）**：`GovernanceChain::run(&self, request: &GovernanceRequest<'_>)` 只收 request。
  `GovernanceRequest` 已携带 `security_context: Option<&TurnSecurityContext>`（§1 类型），再加一个
  并列参数会出现两个真源。`TurnSecurityContext` 参与判定的接线属 R8-B。
- **spec 收窄 2（多签输入）**：R8-A 的签名集合放在 `ChainConfig.human_authority_signatures`
  （无 runtime 审批环可读）；R8-B `run` 增参或改从 request 携带时再迁移，属 additive。
- **已交付（R8-A）**：`crates/foundation/governance/src/chain.rs` — `GovernancePhase`（7 段，含
  `Reflection`）/`ChainOutcome`/`ChainRecord`/`ChainSignatureMode`；四注入端口
  `MultiSignVerifier`/`AuditSink`/`ReflectionScheduler`/`ChainClock` 与默认实现
  （`PlaceholderMultiSignVerifier` 显式 fail-closed / `InMemoryAuditSink` 复用 `AuditHashChain` /
  `ImmediateReflectionScheduler` 进程内记账 / `SystemChainClock`）；`ChainError`；10 条链测试。
- **如实未交付**：Deliberation 段仍为库内 helper 未被 chain 调用（record 里记 `skipped`）；
  `ApprovalClosure` 段只记录 `RequireApproval` 并审计，不消费审批对；Self-disable 段仅记录需要，
  `SelfDisableAudit` 未接线；反思调度无守护进程；真多签 NOT IMPLEMENTED（v2.1）。
- **实测**：`cargo test -p apeireth-governance --lib --locked -j 2` = 204 passed / 0 failed
  （其中 `chain::` 10 passed）；`cargo clippy -p apeireth-governance --all-targets --locked -- -D warnings`
  exit 0；`cargo fmt -p apeireth-governance --check` 0 diff；0 新外部 dep、0 新开关、0 新 crate。

### 2026-10-21（R8-B 设计候选登记，未实施）

- **交付**：`docs/architecture/r8-b-entry-convergence-design.md` —— R8-B 入口收敛设计候选。
  实测确认 `GovernancePipeline` 已实现 `GovernanceHook`（`governance/src/lib.rs:563-604`），
  故 chain 的 Gate 面无需改 governance crate；真正的收敛点在**装配根**：
  CLI `build_production_governance_parts_from_env`（`cli/src/lib.rs:1188`）→
  `PermissionPresetGovernanceHook`（`runtime-assembly/src/canonical/permission_preset.rs:129`）
  → `Runtime::with_governance`（`runtime/src/canonical/runtime.rs:801`）。
- **候选方案**：A 入口链 + 保留动作级 pipeline（不推荐，重复评估）；B 新增
  `GovernanceHook` 适配器把 chain 变成 runtime 唯一治理面（推荐，runtime 零改动、
  回滚=换回 `Arc<GovernancePipeline>`）；C 先落持久审计、收敛延后。
- **如实 IS NOT IMPLEMENTED**：R8-B/C/D 仍未实施；本记录只登记设计候选与证据命令，
  未写任何生产代码。未决问题（动作级审计体积、持久 sink 介质、IM 审计对是否并入
  `AuditHashChain`、多签来源、与生物线 R1b-2c 同窗口施工）待 owner/安全线裁决。
