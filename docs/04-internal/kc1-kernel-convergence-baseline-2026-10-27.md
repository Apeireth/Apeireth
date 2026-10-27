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
- 回归钉死：`crates/engine/runtime/tests/kernel_boundary.rs`（空内核启动、零业务模块运行、每次执行恰一次完成事实、观察者不改变工具目录）。
- 静态扫描是预警手段，不是完整架构证明；`--strict` 与 `--self-test` 的具体结果见交付 commit 正文与 `docs/03-reference/test-baseline.md` 收口记录。
