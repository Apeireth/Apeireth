# KC-1 内核收束基线（KC-0 冻结）

> `Status: KC-0 完成（2026-10-27）；KC-1/2/3/4 已实施，KC-5 以 gate_full 收口`
> `Baseline: main@4ea9c260（2026-10-25）`
> `Owner: runtime`
> `Scope: apeireth-runtime / runtime-assembly / organ / foundation::orchestration / plugin::observation`
> `0 装口径: 本文件只登记已用源码路径与行号复核过的越界项；未复核项不写成事实。行号为 main@4ea9c260 当时值，本轮改动后自然漂移。`

## 1. 已确认越界与处置

| 编号 | 已确认问题 | 源码证据（main@4ea9c260） | 本轮处置 |
|---|---|---|---|
| KC-1.1 | Runtime 直接持有领域痛觉状态 | `crates/engine/runtime/src/canonical/runtime.rs:319` `pub(super) bio_pain: Mutex<BioPainState>`；`runtime.rs:1092` 构造 | 状态与再生案例账整体迁至 `apeireth-organ::nociception`；`Runtime` 不再持有领域字段 |
| KC-1.2 | 内核在失败收口驱动痛觉/再生领域逻辑 | `crates/engine/runtime/src/canonical/execute.rs:3401-3413`（`if self.config.bio_pain_enabled { ... }`） | 内核改为在 `record_call_completed` 发出一次通用 `ToolCompletionFact`；领域判断在 organ observer 内 |
| KC-1.3 | 内核配置与 builder 暴露领域开关 | `runtime.rs:172` `pub bio_pain_enabled: bool`；`runtime.rs:921` `with_bio_pain` | `RuntimeConfig`/`RuntimeBuilder` 移除领域开关；开关仍在 CLI 装配层（env > 配置 > 默认关） |
| KC-1.4 | 内核提供领域快照访问器 | `runtime.rs:370-388` `bio_pain_snapshot` / `bio_recovery_snapshot` / `bio_pain_guard` | 快照移至 `NociceptionObserver::snapshot`；自述 probe 绑定同一 `Arc` 实例 |
| KC-2.1 | 执行循环硬编码业务工具名 | `execute.rs:3031-3055` `if self.config.plan_mode_enabled && call.name == PLAN_MODE_EXIT_TOOL_NAME` | 注册式 `ControlCommand`；`execute.rs` 只做通用控制查找，不认业务名 |
| KC-2.2 | 声明/实现耦合在执行循环文件 | `execute.rs:114-136` `PLAN_MODE_EXIT_TOOL_NAME` + `plan_mode_exit_declaration` | 迁至 `canonical/plan_mode_control.rs`；`execute.rs` 仅保持稳定 re-export |
| KC-3.1 | foundation 持有业务话术与关怀动作分级 | `crates/foundation/orchestration/src/care_potential_field.rs:112-129`（`WhisperCare` 话术 + `SilentPreparation` / `AmbientGlowPulse` 分级） | 整体迁至 `apeireth-organ::care_potential_field`，算法与行为未改 |
| KC-4.1 | 装配层 probe 依赖 runtime 领域快照路径 | `crates/engine/runtime-assembly/src/canonical/bio_mechanism_probes.rs:27,197` | 改为 `apeireth-organ::nociception::BioPainSnapshot`；读取口绑定 observer |
| KC-4.2 | CLI 直接绑定 runtime 领域快照 | `crates/adapters/cli/src/lib.rs:1491-1498`（`Weak<Runtime>` → `runtime.bio_pain_snapshot()`） | 提取 `cli/src/bio_pain_wiring.rs`；绑定装配时装入的 observer 句柄 |

## 2. foundation/orchestration 职责分类（KC-3）

按“机制 / 可复用领域算法 / 业务策略”三分类复核现有模块。**本轮只迁出确证越界的 C 类**，B 类保留原位并记录为后续候选，避免把 KC-1 变成全项目重构。

| 类别 | 模块（示例） | 本轮动作 |
|---|---|---|
| A. 通用机制 | `call_scheduler`、`capacity`、`context_budget`、`context_overflow`、`context_rot`、`continuation`、`cron`、`durable_*`、`job_board`、`job_ring`、`lineage_spawning`、`plan_mode`、`repetition_advisory`、`runtime_invariants`、`task_queue`、`token_meter`、`untrusted_envelope`、`work_consumption`、`worktree_sandbox` | 保留 foundation |
| B. 可复用领域算法 | `nociception`、`recovery_protocol`、`quorum`、`self_tuning`、`trait_report`、`speech_arbiter`、`research_*` | 保留原位；归口领域模块的迁移留待独立阶段（不在 KC-1 强行搬迁） |
| C. 业务策略 | `care_potential_field`（业务话术 + 关怀动作分级） | 已迁至 `apeireth-organ::care_potential_field` |

## 3. runtime-assembly 收束（KC-4）

- 已确认并处理：痛觉/再生 probe 的领域快照类型与读取口不再从 runtime 取；装配层只持有 observer 句柄并绑定读取闭包。
- 未处理（**如实登记，不假装已收口**）：`production.rs`、`cognitive.rs`、`orchestrator.rs` 等大型装配文件的进一步拆分；`organ_module.rs` 的器官编排职责；这些属于既有装配面，本轮不一次性大范围替换。
- 判定依据：装配根允许创建模块、注入依赖、注册能力、建立生命周期/事件连接与装配验证；不允许在 CLI/Gateway 再建第二套业务管线。本轮只迁出已确认越界的实现。

## 4. 守卫与回归（KC-4/KC-5）

- `docs/architecture/contract.toml [kernel]` 新增运行时生产依赖白名单、执行循环禁用字面量、领域类型黑名单。
- `scripts/arch_guard.py` 新增 `check_kernel_boundaries`；`--self-test` 用三类违规 fixture 证明守卫会稳定失败（域依赖 / 硬编码分派 / 领域类型）。
- 回归钉死：`crates/engine/runtime/tests/kernel_boundary.rs`（空内核启动、零业务模块运行、每次执行恰一次完成事实、观察者不改变工具目录、观察者 panic 不破坏已完成调用、审批恢复恰一次投递）。
- 观察者失败隔离：`record_call_completed` 用 `catch_unwind` 包裹观察者调用，**不重试、不补发**；已落账的事件/轨迹与工具结果不受影响。这是进程内隔离，不构成跨崩溃/跨进程的 exactly-once 保证。
- CI 接线：`.github/workflows/arch-guard.yml` 新增 `Arch guard self-test (kernel boundary fixtures)` 步骤，与既有 report 步骤同处灰度 job（`continue-on-error`），不改变 2026-11-03 前 report-only 的既有裁决。
- 静态扫描是预警手段，不是完整架构证明；`--strict` 与 `--self-test` 的具体结果见交付 commit 正文与 `docs/03-reference/test-baseline.md` 收口记录。

## 5. KC-5 全量收口（2026-10-27）

- 执行器：`pwsh scripts/gate_full.ps1`（仓库唯一全量执行器；主 workspace 排除图像家族 + 图像家族独立两段）。
- 结果：**GATE FULL PASSED**，五个阶段全 `OK`（`main.cargo-test` / `main.clippy` / `main.fmt` / `image.cargo-test` / `image.clippy`），日志 `logs/gate-full.log`。
- 实测数字（2026-10-27，本机，日志逐行汇总）：**351 套件 / 8902 passed / 0 failed / 53 ignored**（套件 = 测试二进制目标计数）。
- 工具链：rustc 1.97.1 / cargo 1.97.1（与 `docs/03-reference/test-baseline.md` 现行口径同工具链）。
- 口径说明：`docs/03-reference/test-baseline.md` 维护权归效率线，本批**不代改真源**；其现行 341 套件 / 8570 passed 为 2026-10-21 历史口径，本批实测值如上，对外引用时按 AGENTS.md §3 报「本批实测值 + 日期」。
- 架构验收：`python scripts/arch_guard.py --strict` = VIOLATIONS 0；`--self-test` = 4/4 PASS；`docs_truth.py --verify` = claims=29 fail=0 stale=0；`wiring_report.py --verify` = OK，`--crosscheck --strict` = problems=0；`docs_links.py --check` = 断链 0；`docs_index.py --check` = 问题 0。

## 6. 未完成/后续（如实标注）

- `contract.toml` 的 `[kernel].forbidden_domain_types` 是文本级黑名单，**不覆盖**宏展开、条件编译与动态分派；它不是完整架构证明。
- foundation 的 B 类领域算法（`nociception` 内核、`recovery_protocol`、`quorum` 等）仍在原位，本轮只迁出确证越界的 `care_potential_field`；进一步归属迁移留待独立阶段。
- `runtime-assembly` 的大型装配文件（`production.rs` / `cognitive.rs` / `orchestrator.rs`）未拆分，本轮只迁出已确认越界的 probe 绑定。
- 远程 CI 未在本批验证（本地全量执行器已通过）；`apeireth-mesh-nat-e2e` 等独立工作线未运行。
