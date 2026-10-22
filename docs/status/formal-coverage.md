# 形式化覆盖边界登记（formal-coverage）

> 载体来源：ADR-0011 D6。**这是形式化面的唯一登记表**；新增 `unsafe` 集中 crate 或
> 新形式化目标必须同时更新本表，否则 `docs_truth` 漂移检查应红（本批只落载体与边界，
> 自动漂移检查属 R6-D/E 效率线）。
>
> 口径纪律（0 装）：本表只登记**已配置/已实证**的内容；未运行 = NOT RUN，
> 未覆盖 = NOT COVERED，有界 = 明写边界，不写「全部证明」。

## 1. Kani（有界模型检查，CI 阻塞）

| 项 | 事实 |
|---|---|
| 载体 | `research/verification/kani/` mirror crate（`#[path]` 零复制包含 canonical 源；不复制代码） |
| CI | `.github/workflows/kani.yml`；`push main` / `pull_request` / `workflow_dispatch`；ubuntu runner |
| 阻塞语义 | 汇总门 job `kani-required-gate` = 分支保护 required check；无 `continue-on-error`；触碰已验证面 → 矩阵全绿才放行，未触碰 → 路径豁免秒绿 |
| 版本钉 | Kani 0.68.0（最后一次全绿 run 2026-09-25 实测版本） |
| 本机限制 | Windows 无 Kani 支持（只在 ubuntu/macOS 跑；本机只能 `cargo kani setup` 后跑 Linux runner/容器） |

### 1.1 终局矩阵（10 件；2026-10-13 K18 定稿 11 件，2026-10-22 族 4 归档后 11→10）

| # | harness | 被测面 | 性质 |
|---|---|---|---|
| 1 | `kani_terminal_lock_no_outgoing_transitions` | Phase 5 审批状态机（`crates/engine/runtime/src/canonical/research_approval_sm.rs`） | 终态锁：终态后无出边 |
| 2 | `kani_executed_monotonic_once` | 同上 | executed 单调、至多一次 |
| 3 | `kani_crash_recovery_invariant_c` | 同上 | 崩溃恢复保持 InvC |
| 4 | `kani_panic_free_base64_decode` | `crates/adapters/gateway/src/file_fetcher.rs`（模块私有 `base64_decode`） | panic-freedom 子集 |
| 5 | `kani_panic_free_river_measure_omega` | `crates/engine/memory/src/river_topology.rs` | panic-freedom 子集 |
| 6 | `kani_governance_deny_axis_never_allows` | `crates/foundation/governance/src/risk.rs`/`intent.rs` | 治理单调性 |
| 7 | `kani_governance_default_deny_unrequested_ops` | 同上 | 治理单调性 |
| 8 | `kani_governance_no_degrade_never_weakens` | 同上 | 治理单调性 |
| 9 | `kani_governance_fail_closed_no_bypass` | 同上 | 治理单调性 |
| 10 | `kani_sandbox_validate_rejects_traversal_and_nul` | `crates/capabilities/tools/src/sensitive_path.rs` | 路径沙箱子集 |

- **有界口径**：Phase 5 三 harness 位于 canonical 源内、带 `#[kani::unwind(32)]`
  （仅 `cfg(kani)` 生效）：String 字节缓冲按符号长度建模，Kani 把 `HashMap` SipHash
  `Hasher::write` 当无限展开，`unwind(32)` 是实测可覆盖真实短键的深度上限；
  **超出上界的符号路径不检查**——属有界模型检查，不是全系统证明。
  详见 `research/verification/kani/README.md` §循环展开上界与
  `docs/04-internal/kani-verification-playbook.md`。
- **已退出矩阵的 harness（源码保留为复验入口，不在 CI 阻塞）**：`kani.yml` 注释记录的
  退出分组为 fold×3 / 数值×4 / memory×3 / quota×2 / saga×2 / sandbox×4 / cache_key。
  退出理由 = 带全部刀法（分支定长/值域表/零 `format!`/静态切片）仍 900s+ 超时，
  按协议退出；panic-freedom 由单测覆盖，属**未覆盖**而非已证明。
  **实测校准（2026-10-22）**：mirror crate `src/` 现有 25 个 `#[kani::proof]`
  （governance 4 / panic-freedom 11 / saga 2 / sandbox 5 / memory 3；原 quota 3 件随
  ADR-0007 归档于 2026-10-22 移除），
  canonical 文件内另有 3 个（README 记录）；其中只有矩阵里的 10 件进 CI。
  workflow 注释的退出分组与当前源码分组**不逐一对应**（历史口径未同步）——
  本表按「未入矩阵 = 未覆盖、不阻塞」登记，不逐件断言是否可证。
- **旧口径订正**：`.github/workflows/kani.yml` 头部与汇总门旧注释写「22 件」
  （2026-10-10 并行化改造时口径），与 2026-10-13 K18 终局矩阵 11 件不符；
  2026-10-21 已把 workflow 注释与汇总门文案改为 11 件并注明历史口径；
  **2026-10-22 再订正为 10 件**（族 4 配额调度随 ADR-0007/R7-4b-2 `d5a4c537` 归档，mirror 与
  harness 同步移除；本表与 kani.yml 已一致）。
  `research/verification/kani/README.md` 的「3 个 `#[kani::proof]` / 3/3」
  是 2026-09-05 Phase 5 单点实证的历史口径，当前矩阵以本表与 `kani.yml` 为准。

## 2. TLA+/TLC（本机验证，不入 CI）

| 文件 | 主题 | 状态 |
|---|---|---|
| `research/verification/tla/ApprovalSM.tla` + `.cfg`、`ApprovalSM3.*` | 审批状态机模型（与 Kani 同源命题） | 本机 TLC 验证，历史记录在 `research/verification/tla/README.md`；**不入 CI**（NOT IN CI） |
| `research/verification/tla/ProtectForget.tla` + `.cfg`、`ProtectForget3.*` | 保护/遗忘状态机 | 同上 |
| `research/verification/tla/QuotaSchedule.tla` + `.cfg`、`QuotaSchedule3.*` | 配额调度模型（实现 `CognitiveQuotaScheduler` 已按 ADR-0007 归档，模型留档） | 同上 |

- 与 Kani 的关系：互补——TLC 对有限状态模型做穷举探索，Kani 对有界实现做符号执行；
  两者都**不是**「对本仓库全部代码的形式化证明」。

## 3. Miri（UB 集中面，CI）

| 项 | 事实 |
|---|---|
| 载体 | `.github/workflows/miri.yml`（nightly toolchain；unsafe 集中 crate 矩阵） |
| 阻塞语义 | 失败即红（无 `continue-on-error`）；只在 nightly 上跑 |
| 覆盖面 | unsafe 集中 crate（矩阵见 workflow）；**不覆盖全 workspace** |
| 本机限制 | 需 nightly + miri component；本机未配置 → 本表不登记本机结果 |

## 4. 明确未覆盖（NOT COVERED / NOT RUN）

- 全系统功能正确性：无形式化证明（不假装）。
- 并发/分布式：仅 TLA 有限状态模型，无实现级并发证明。
- 内存安全：只有 Miri 覆盖 unsafe 集中 crate，非全 workspace。
- 性能/基准：属四层验收第 2 层（`verification-ledger.yaml`，本批未建条目）。
- Kani/Miri 的 CI 运行结果：**本表登记配置与边界，不登记「最近一次绿」**；
  最近运行以 GitHub Actions 历史为准（本仓库未 push 时无 CI 运行记录 = NOT RUN）。

## 5. 维护规则

1. 新 `#[kani::proof]`：先加进 `kani.yml` matrix 与「已验证面」路径列表，再更新本表；
   新增/退出必须在同 commit 内同时改 matrix、汇总门文案与本表（三者一致）。
2. 新 unsafe 集中 crate：必须检查是否纳入 `miri.yml` 矩阵并更新 §3。
3. 形式化目标退出/降级：须在 ADR-0011 §9 或新 ADR 记录，禁止静默删除。
4. 本表与 `kani.yml`/`miri.yml` 冲突时以 workflow 实况为准，并在本表标记订正日期
   （本批 2026-10-21 已按此订正 22→11 旧口径）。
