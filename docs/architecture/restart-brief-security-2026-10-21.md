# 安全线重启简报 — 2026-10-21

> 用途：owner 重启安全线后，本文件是**唯一开工入口**。读它与
> [line-coordination.md](line-coordination.md)、[ADR-0010](../../docs/adr/0010-r8-governance-pipeline.md)、
> [r8-b-entry-convergence-design.md](r8-b-entry-convergence-design.md) 即可接续。
> 签发：架构线（lead），2026-10-21。状态：Wave 4 已收口（`5f6ee7da`，`current_wave=5`）。

## 0. 当前仓库基线（开工前先复核）

| 项 | 现值 |
|---|---|
| HEAD | `5f6ee7da`（Wave 4 收口批） |
| 契约守卫 | `wave=5 / crates=30 / edges=111 / env=154 / exceptions=2 / VIOLATIONS 0 / STALE 0` |
| 全量基线 | 341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21，`gate_full`；日志 `artifacts/wave4-close-full-2026-10-21.log`） |
| 治理面已交付 | R8-A `fd74a669`（`GovernanceChain` 六段骨架 + 四注入端口，生产零调用）；R8-B1 `b6d902ce`（`ChainGovernanceHook`，零生产接线）；R8-C `da7e224f`（延迟反思队列，默认零变化）；R8-D 部分 `60627d8e`（演练入 CI） |
| 治理面**未**交付 | R8-B2（CLI 装配根切换）、R8-B3（持久 `AuditSink` + IM 审计对经 chain）；R8-D wiring 面归效率线 |
| 门禁绿 | `docs_truth --verify --check` / `arch_doc --check` / `wiring_report --verify` |

## 1. 本轮任务（按序，1 commit = 1 任务）

### S1. R8-B2：CLI 装配根切换（治理单一入口第一半）
- 依据：`docs/architecture/r8-b-entry-convergence-design.md`（三方案，采纳 B）；ADR-0010 D1/D3。
- 目标：CLI 装配根改为「`GovernanceChain`（gate = 现有 `GovernancePipeline`）→ 包成 `ChainGovernanceHook`
  → `builder.with_governance(...)`」；`PermissionPresetGovernanceHook` 继续作为链内 gate 的包装层，语义不变；
  未配置时仍 `DenyUnconfigured` fail-closed。
- 关键锚点（2026-10-21 已验证存在）：
  - `crates/adapters/cli/src/lib.rs:566` `build_production_governance`；`:835` `build_production_governance_parts`；
    `:1108` `build_production_governance_from_env`；`:1188` `build_production_governance_parts_from_env`；
    `:1550/:1559-1564` 取三件并包 `PermissionPresetGovernanceHook` 后 `builder.with_governance`。
  - `crates/engine/runtime/src/canonical/runtime.rs:330` 治理字段、`:801` `with_governance`、`:750` 默认 `DenyUnconfigured`。
  - `crates/foundation/governance/src/chain.rs` `GovernanceChain` / `ChainGovernanceHook`（B1 已交付，5 测试）。
- 验收：CLI 现有治理/审批测试全绿 + 新增「链入口与旧入口行为等价」测试（Allow / Deny / RequireApproval /
  NotConfigured 四态）；默认路径逐字节行为等价（含 gate 副作用注入点 `:1555/:1638-1641/:1709` 不变）；
  `cargo test -p apeireth-cli --all-targets` 绿；guard VIOLATIONS 0。
- 协调：改 `crates/adapters/cli/src/lib.rs` 前先 `git status` 确认无他线在途（生物线当前只在 runtime-assembly 在途）；
  显式路径 stage。

### S2. R8-B3：持久 `AuditSink` + IM 审批审计对经 chain（治理单一入口第二半）
- 依据：ADR-0010 D2/D4；设计文件 §2 入口面三处。
- 目标：①在 runtime-assembly 或 gateway adapter 落一个**持久** `AuditSink` 实现（禁跨层依赖：不在 governance
  引 adapters）；②`crates/adapters/gateway/src/im_approval.rs` 的 `commit_approval_audit_pair`
  /`ApprovalAuditPair`/`ApprovalAuditRecord` 经 chain 的 AuditAppend 段落账；③`may_resolve_approval`
  保持为策略函数（不进链，ADR-0010 D3）。
- 验收：重启后审计链 `verify()` 仍通过（持久化往返测试）；审计追加失败时 fail-closed（`ChainOutcome::Denied`）；
  gateway IM 审批测试全绿；0 新外部 dep（若必须新增先依 ADR-0001 走例外/ADR）。

### S3. 安全线自有残项（先报 owner 决策，不擅自开工）
来自 `artifacts/security-line-handover-2026-10-20.md` §三，仍在账：
- B6 P1 敏感字段定级（`memory_vectors` 明/密）待记忆线拍板；列级改造未接线；KDF 210k 迭代基线未实测。
- B5 未实测面：TLS 1.2/1.3 矩阵、ECC 材料、证书轮换、生产证书；B2 回环类级口子/公网 allowlist 待续。
- B8 排期四项（P1 签名证书 / P2 可复现构建 / P3 平台校验实测 / P4 分通道）owner 决策。
- 处理方式：先向 owner 报优先级建议，获准后再动；不要与 S1/S2 混在同一 commit。

### S4. 边界（硬约束）
- LOCKED 5 项 0 触碰；`cargo test/clippy/fmt` 三件套按需自跑，**全量数字只认效率线 `gate_full` 结果**（不发布自测全量数字）。
- 不改 `docs/architecture/contract.toml`/`arch-baseline.txt`/`arch_guard.py`（架构线资产）；需要时找架构线。
- 1 commit = 1 任务；显式路径 stage；提交走 `commit.ps1`；灰度期 arch/wiring 守卫 report-only 至 2026-11-03。

## 2. 立即复核命令

```powershell
cd C:\Users\USER_A\Apeireth-rust
git log --oneline -3
git status --short
python scripts/arch_guard.py --report
cargo test -p apeireth-governance --lib --locked -j 2     # 当前 214 passed 基线
cargo test -p apeireth-cli --lib --locked -j 2            # 开工前基线，供等价性对比
```

## 3. 与其它线的接口
- 架构线：契约/ADR/守卫/基线；R8 的 spec 与验收（ADR-0010 §9 记录了架构线代做 A/B1/C/D 部分的边界）。
- 效率线：全量数字执行与 test-baseline 刷新、wiring 台账（R8-D 余项归其线）。
- 生物线：当前在 runtime-assembly `nervous_snapshot.rs` 有在途改动，改共享文件前先看 `git status`。
