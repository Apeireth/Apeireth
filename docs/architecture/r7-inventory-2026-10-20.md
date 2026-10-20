# R7-1 现状清单（ADR-0007 预研，2026-10-20）

> 目的：为 owner 裁决 ADR-0007 提供**可复核证据**，本清单不改代码。
> 口径：静态 `git grep` + 定义/接线点核对（**未做运行时 E2E**）；"有引用"≠"活跃生产路径"。

## 1. Scheduler（5 个）

| Scheduler | 定义 | 接线证据 | 判定 |
|---|---|---|---|
| `CallScheduler` | `foundation/orchestration/src/call_scheduler.rs:272` | runtime `execute.rs:2014/3671` 使用；`runtime.rs:355/733/1032` 字段+`with_call_scheduler` | **active**（请求期调度） |
| `HeartbeatScheduler` | `runtime/src/canonical/heartbeat.rs:67` | 仅同文件（构造点 `:284/:460` 疑为测试）；`care_heartbeat.rs:9` 仅注释提及 | **dormant（待复核 producer 是否内嵌）** |
| `NightwatchIdleScheduler` | `runtime-assembly/src/canonical/nightwatch.rs:334` | CLI `dispatch_nightwatch`（`cli/src/lib.rs:2714+`）内构造 `:2811` | **active**（CLI 显式命令路径） |
| `RecalibrationScheduler` | `memory/src/online_calibration.rs:418` | 仅 memory `lib.rs:199` re-export，无其他 src 构造/使用 | **dormant（export-only）** |
| `CognitiveQuotaScheduler` | `orchestration/src/cognitive_quota_scheduler.rs:198` | 仅 orchestration `lib.rs:132` re-export，无其他 src 构造/使用 | **dormant（export-only）** |

**结论**：活跃路径 = **2**（CallScheduler + NightwatchIdleScheduler），已满足 ADR-0007 "≤2" 目标；dormant 3 个待 R7-1 决策（接线取证 or 归档 `legacy/archived/schedulers/`）。

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
| `memory/src/session_lifecycle.rs:259` | `SqliteMemoryStore`(:312) | 域内生命周期 store → 拟改名 `SessionLifecycleStore` |
| `memory/src/session_note.rs:54` | `SqliteMemoryStore`(:76) | 域内 note store → 拟改名 `SessionNoteStore` |

**结论**：3 个 trait 实为"1 公共契约 + 2 域 store"同名冲突；ADR-0007 §2.2 的"重命名+文档说明"成立，**不需要合并语义**（避免把域职责塞进公共契约）。

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
