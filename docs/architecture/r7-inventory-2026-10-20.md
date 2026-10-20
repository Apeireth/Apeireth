# R7-1 现状清单（ADR-0007 预研，2026-10-20）

> 目的：为 owner 裁决 ADR-0007 提供**可复核证据**，本清单不改代码。
> 口径：静态 `git grep` + 定义/接线点核对（**未做运行时 E2E**）；"有引用"≠"活跃生产路径"。

## 1. Scheduler（5 个）

| Scheduler | 定义 | 接线证据 | 判定 |
|---|---|---|---|
| `CallScheduler` | `foundation/orchestration/src/call_scheduler.rs:272` | runtime `execute.rs:2014/3671` 使用；`runtime.rs:355/733/1032` 字段+`with_call_scheduler` | **active**（请求期调度） |
| `HeartbeatScheduler` | `runtime/src/canonical/heartbeat.rs:67` | **被 `HeartbeatProducer` 内嵌**：字段 `:268`（`scheduler: Arc<Mutex<HeartbeatScheduler>>`）、构造 `:284`、访问器 `:310`、run loop 参数 `:359`；R7-4 复核（2026-10-20）发现早先"仅测试构造"结论有误（此前 grep 被截断到前 6 行） | **active（保留）** |
| `NightwatchIdleScheduler` | `runtime-assembly/src/canonical/nightwatch.rs:334` | CLI `dispatch_nightwatch`（`cli/src/lib.rs:2714+`）内构造 `:2811` | **active**（CLI 显式命令路径） |
| `RecalibrationScheduler` | `memory/src/online_calibration.rs:418` | 仅 memory `lib.rs:199` re-export，无其他 src 构造/使用 | **dormant（export-only）** |
| `CognitiveQuotaScheduler` | `orchestration/src/cognitive_quota_scheduler.rs:198` | 仅 orchestration `lib.rs:132` re-export，无其他 src 构造/使用 | **dormant（export-only）** |

**结论**：活跃路径 = **3**（CallScheduler + NightwatchIdleScheduler + HeartbeatScheduler）；但 R7 目标口径为"dormant 清零、活跃面可复核"，Heartbeat 是 Producer 内嵌调度缝（非独立注册）→ 保留；dormant 2 个（Recalibration/CognitiveQuota）已于 R7-4b-1/2 归档。

## 2. PromptCacheStabilizer

| 项 | 证据 | 判定 |
|---|---|---|
| 定义/实现 | `orchestration/src/prompt_stabilizer.rs:44/46` | — |
| 导出 | `orchestration/src/lib.rs:189` | export-only |
| 生产消费 | 无（仅 organ `context_assembly.rs:13/330` 注释引用） | **dormant** |
| 测试 | 同文件 `:168/:208` | 测试覆盖存在 |

**结论**：按 ADR-0007 §2.3 二选一——接入 canonical prompt 组装路径（须新增 cache-prefix 测试+生产消费点）或删除；**不允许保留 export-only**。

## 3. SessionStore（3 个 trait）

| 位置 | 实现者 | 说明 |
|---|---|---|
| `runtime/src/canonical/session.rs:802` | `InMemorySessionStore`(:859)、`SqliteSessionStore`(assembly `sqlite_session.rs:92`)、测试 `RecordingStore` | **canonical 会话契约**（ADR-0007 §2.2 保留为唯一公共名） |
| `memory/src/session_lifecycle.rs:259` | `SqliteMemoryStore`(:312) | 域内生命周期 store → **已改名 `SessionLifecycleStore`**（`1cbaee95`） |
| `memory/src/session_note.rs:54` | `SqliteMemoryStore`(:76) | 域内 note store → **已改名 `SessionNoteStore`**（`1cbaee95`） |

**结论**：3 个 trait 实为"1 公共契约 + 2 域 store"同名冲突；ADR-0007 §2.2 的"重命名+文档说明"成立，**不需要合并语义**（避免把域职责塞进公共契约）。

**R7-3 执行状态（2026-10-20，已完成）**：memory 侧已改名（commit `1cbaee95`，trait+impl+导出+集成测试+注释同步）；memory 内 `\bSessionStore\b` 残留 **0**、全仓 `memory::SessionStore` 引用 **0**；依赖方 `apeireth-runtime-assembly` / `apeireth-gateway` 的 `--all-targets` 编译 exit 0；runtime canonical `SessionStore` 保持唯一公共契约。

## 4. ContextBlock / ContextAssembler

| 概念 | canonical | debt | 使用面（src 文件） |
|---|---|---|---|
| `ContextBlock` | `orchestration/src/context_budget.rs:34` | `organ/src/context_assembly.rs:45` | organ 3 文件、runtime 3 文件、orchestration 2 文件、runtime-assembly `cognitive.rs` |
| `ContextAssembler` | `orchestration/src/context_budget.rs`（contract） | `organ/src/context_assembly.rs:184` | organ `lib.rs:79`/`prompt_assembly.rs` 导出与使用 |

**结论**：按 ADR-0007 §2.1——类型收敛到 orchestration、装配器留 organ、删 runtime `execute.rs:83` re-export；迁移批 R7-2 的 9 个 src 文件为触面。

## 5. 交给 owner 的问题（裁决 ADR-0007 时需要）

1. dormant 3 个 scheduler：**接线**（谁给真实触发点？）还是**归档**？（默认建议：归档，恢复入口写 MANIFEST）
2. `PromptCacheStabilizer`：接入还是删除？（默认建议：删除；若无 cache-prefix 稳定性收益证据）
3. `HeartbeatScheduler`：care_heartbeat 注释称其为"调度缝"——是否被 `HeartbeatProducer` 内嵌使用？（R7-1 复核项；若内嵌则判 active）
4. SessionStore 改名是否接受（`SessionLifecycleStore` / `SessionNoteStore`）——涉及 24 处引用迁移。

## 6. 证据边界（0 装）

- 本清单为**静态证据**，未跑运行时探针；"dormant" 判定允许被运行时证据推翻（推翻即更新本清单）。
- 未修改任何 crate 源码；未新增/删除文件。

## 7. R7-2 执行结果（2026-10-20，已落地）

**R7-2a**（commit `0bdd9bb4`）：organ 删自有多余 `ContextBlock` 定义（原 `organ/src/context_assembly.rs:45`），改为
`#[doc(inline)] pub use apeireth_orchestration::context_budget::ContextBlock;`；适配 organ 内 3 处构造点
（`name: String`）与 1 处 names 收集（`as_str`）；旧路径 `apeireth_organ::ContextBlock` 保留可用。
**R7-2b**（commit `a93fdbb3`）：删 runtime `execute.rs:83` re-export（改私有 `use`）、`canonical/mod.rs:102` 与
`lib.rs:18` re-export；两处调用方（`runtime/tests/context_budget.rs`、`adapters/cli/tests/production_knobs.rs`）
直连 orchestration canonical。
**附带修复**（commit `ae1f8779`）：R2-3c 删 `PluginManager::capabilities()` 后 runtime `Debug` 漏改致 E0599，
改 `records().len()`；详见 `r2-inventory-2026-10-20.md` §7。

**验收（实测，2026-10-20）**：
- `git grep 'pub struct ContextBlock' -- crates/` = **1**（仅 `orchestration/src/context_budget.rs:34`）；
- `cargo test -p apeireth-organ --lib` 377 passed；`cargo test -p apeireth-runtime --test context_budget` 7 passed；
  `cargo test -p apeireth-cli --test production_knobs` 22 passed；
- `cargo check -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-cli --all-targets` exit 0；
  clippy 0 告警、`cargo fmt --check` 0 diff；
- 残留 `runtime::canonical::ContextBlock` 引用 = 0。
- **基线 STALE**：`concept ContextBlock crates/engine/organ/src/context_assembly.rs` 已失效（定义迁走）——
  Wave 3 收口缩表时删除该行；`size crates/foundation/orchestration/src/lib.rs 1432` 同为收口缩表候选
  （R7-4 删代码引起，非本次改动）。

## 8. R7-2.1 执行结果（2026-10-20，已落地）

**R7-2.1**（commit `c8cb34a4`）：ADR-0007 §2.1 遗留项收口——`ContextAssembler` 单一化。

- foundation/orchestration `context_budget.rs` 成为唯一装配管线：新增 `AssemblyPolicyHook`
  （默认恒等的 `effective_budget_chars` / `max_non_core_layers` / `fold_non_core`），管线为
  cap → fold → 非 core 降序贪心切 → spill；无钩子时输出与旧管线逐字节一致。
- engine/organ `context_assembly.rs` 删本地 `ContextAssembler` 实现与 `assemble_budgeted_blocks_raw`
  （约 148 行），改 `#[doc(inline)] pub use` 单一实现 + `OrganAssemblyPolicy`（tiered policy→钩子）+
  `ContextAssemblerPolicyExt::with_policy`（调用形状 `with_policy(policy)` 不变）；`lib.rs` 导出面同步。
- `runtime-assembly/canonical/cognitive.rs` 补 `ContextAssemblerPolicyExt` 导入（调用点 :2919）。
- `docs/architecture/contract.toml`：`[[concepts]].ContextAssembler` canonical 从 organ 改指
  orchestration，`known_debt = []`（重复概念债务清零）。

**验收（实测，2026-10-20，HEAD 207b3e02 工作树）**：

- `cargo test -p apeireth-orchestration -p apeireth-organ --lib --locked` = 440 / 377 passed、0 failed
  （含新增 4 条钩子测试与既有管线 golden 测试）；
- `cargo test -p apeireth-runtime-assembly --lib --locked` = 274 passed / 0 failed（含 R1b-1a 新模块）；
- `cargo clippy -p apeireth-orchestration -p apeireth-organ -p apeireth-runtime-assembly --lib
  -- -D warnings` exit 0；`cargo fmt --check` 0 diff；
- `python scripts/arch_guard.py --report`：VIOLATIONS 1 = 已知 `[W1] size-grow
  crates/engine/runtime-assembly/src/canonical/cognitive.rs 4757 -> 4759`（导入多行 +2；Wave 4 收口
  按实测重钉基线，灰度期 report-only 如实披露）；concept W10 随 canonical 改指清零。
