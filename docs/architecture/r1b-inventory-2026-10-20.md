# R1b 取证包（2026-10-20）

> **归属**：架构线取证 / spec（ADR-0002 §R1b：机制改订阅统一信号总线、开关收敛单入口、补缺失信号源）。
> **实施线**：生物线主笔（ADR-0002 主笔线口径）；R1b-1 结构内核与生物线在途 WIP 无交集，可由架构线先行落。
> **快照**：HEAD `7e98744d`（W3.7 仪式完成事件源，2026-10-20）。工作树另有生物线文档 3 个 M
> （`CHANGELOG.md` / `docs/03-reference/capabilities-matrix.md` /
> `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md`），未纳入证据。
> **口径**：所有数字可由 §9 命令复核；四级口径含义见
> [system-capabilities.md §0](../01-architecture/system-capabilities.md)；本文不发布测试数字（归
> `docs/03-reference/test-baseline.md`）。

## 1. 结论摘要

1. **没有跨机制信号总线**：`git grep "pub struct NervousSignal"` = 0、`... NervousBus` = 0；
   唯一有序观测点是模块钩子派发环 `crates/engine/runtime/src/canonical/execute.rs:2447-2460`
   （`for module in &self.modules { ... module.on_hook(hook, &context).await ... }`），各模块各自过滤、
   各自消化，无共享信号类型、无订阅注册、无背压/计数语义。
2. **开关未收敛**：9 个生物开关 + 1 个时区偏置参数，均在模块安装函数内直接读 env；bio env 常量命中
   **17 个 src 文件**（§3）。
3. **台账手写且已漂移**：R1a 台账 14 行中至少 3 行落后于生物线 W3.5/W3.6/W3.7 实况
   （SG-18 / SG-22 / SG-26，§2.3 实证）。
4. **适配器只有一条真实事件注入路径**：CLI 审批面 `canonical_approval.rs` 在审批落账后旁路推进
   `ToneState`；其余四份反馈接口（self_tuning / value_cases / online_calibration / orchestrator）
   在 src 无生产调用方。
5. **R1b 不新增 crate、不新增外部依赖**：`apeireth-runtime-assembly` 已具备 `parking_lot` / `tokio` /
   `chrono`。总线建议同步实现（`parking_lot::Mutex` + 订阅者表），不进 async 通道，避免把钩子派发
   语义复制成第二套异步生命周期。

## 2. R1a 台账现状（唯一现存的「机制 × 信号」矩阵）

### 2.1 形状与计数

`crates/engine/runtime-assembly/src/canonical/nervous_snapshot.rs`（647 行）：

| 项 | 值 | 位置 |
|---|---|---|
| 快照挂面 | `capabilities.nervous_system`（`capabilities` 节内第二个只读子对象，与 `bio_mechanisms` 并列） | `:47-52` `CAPABILITIES_SECTION` / `NERVOUS_SYSTEM_SUBOBJECT` |
| schema 版本 | `NERVOUS_SNAPSHOT_VERSION = 1` | `:54` |
| 机制总数 | `NERVOUS_MECHANISM_TOTAL = 14`（SG-13…SG-26 闭合词表） | `:56` |
| 行结构 | `MechanismSpec { id, level, enabled_env, parser, wiring, signals, gaps }` | `MECHANISMS: [MechanismSpec; 14]` |
| 级别枚举 | `MechanismLevel { ProductionWired, Half, Implemented }` | 台账内部 |
| 信号态枚举 | `SignalState { Wired, Partial, NotWired }` | 台账内部 |
| 快照结构 | `NervousSnapshot { version, summary, mechanisms }`；`summary = {total, production_wired, half, unwired, observed}` | 台账内部 |
| `observed` 口径 | 当前 `bio_mechanisms` 帧中 `wired: true` 的格数（真实注册且读数成功的 probe 数） | 只读，不伪造 |
| `last_observation` | 恒 `null`（本模块不承载读数；读数在 `capabilities.bio_mechanisms`） | 0 装诚实 |
| 开关关 | 不读 env、不构造快照、不插键；单测断言 `reads = 0` | `nervous_snapshot.rs` tests |

### 2.2 14 行现状（级别 / 未接或半程信号源）

| 行 | 级别 | 未接（NotWired）/ 半程（Partial）信号源（reason 原文节选） |
|---|---|---|
| SG-13.neuromodulation | ProductionWired | `tone_feedback` NotWired「真实反馈事件源未接（半程）」 |
| SG-14.homeostasis | ProductionWired | `vital_sign_inputs` Partial「仅 ContextPressure 实测」 |
| SG-15.immune_memory | ProductionWired | — |
| SG-16.nociception | ProductionWired | — |
| SG-17.recovery | ProductionWired | `scar_ledger_learn` NotWired；gap「ScarLedger::learn 未接」 |
| SG-18.lifecycle | Implemented | `tool_lifecycle_ledger` NotWired「production.rs 装配根未注册（本批禁改，待 Lead）」 |
| SG-19.attachment | ProductionWired | — |
| SG-20.social_repair | ProductionWired | `session_event_source` NotWired「会话事件源未接（仅显式注入 RepairPlan）」 |
| SG-21.play | ProductionWired | —（gap：无 IM 卡 / 无持久化 / 沙箱非 OS 隔离） |
| SG-22.ritual | Half | `ritual_producer` NotWired「仪式生产者未接 ⇒ 0 候选（如实）」 |
| SG-23.circadian | ProductionWired | `sleep_pressure` NotWired「门内压力恒 0」 |
| SG-24.quorum | ProductionWired | — |
| SG-25.ecosystem_health | Implemented（未接线） | `ecosystem_health_kernel` NotWired「内核在场但无装配入口/开关」 |
| SG-26.capacity | ProductionWired | —（gap 文字见 §2.3，已过期） |

分布：ProductionWired 11 / Half 1（SG-22）/ Implemented 2（SG-18、SG-25）。

### 2.3 已实证的台账漂移（R1b 必须先修的三处）

| 行 | 台账原文 | 实况（HEAD `7e98744d`） | 证据 |
|---|---|---|---|
| SG-18.lifecycle | `wiring = "assembly-entry-ready::root-registration-pending"`；gap「production.rs 装配根注册待 Lead（本批禁改）」 | **模块已注册**：`production.rs:1521` 调 `bio_production_wiring::install_bio_modules(...)`，lifecycle 模块进 module 表并回传句柄（`:1594-1595`）。真实缺口只剩 **probe 未注册**：`bio_lifecycle.rs:76-78` / `:825-826` 明写「由 Lead 在装配根 `register_bio_probe(BioMechanismKind::Lifecycle, ...)`」「本批未注册（production.rs 禁改）」 | `production.rs:1521,1594`；`bio_lifecycle.rs:76-78,825-826` |
| SG-22.ritual | `ritual_producer` NotWired「仪式生产者未接」 | W3.7 `7e98744d` 已交付 `tool.ritual_done` / `tool.ritual_skip` 工具面（`ritual_tools.rs` 698 行），该 commit **未改** `nervous_snapshot.rs` → 台账仍按旧态 | `git show --stat 7e98744d`（6 文件，含 `ritual_tools.rs`，不含 `nervous_snapshot.rs`） |
| SG-26.capacity | gap「bio_mechanisms 14 格当前只注册 1 格（其余 wired:false）」 | W3.6 后 `install_bio_self_status_with` 在开关开时注册 **7 格 probe**（capacity + play + 5 条迟绑定：immune / attachment / quorum / nociception / recovery，`bio_production_wiring.rs:390-403`）；可读性取决于 `fill_*` 句柄（§4.3） | `bio_production_wiring.rs:366-404` |

**结论**：R1a 台账是手写常量，生物线每批交付不会自动改它。R1b 若继续手写，Wave 4 收口时必然再漂。
→ 台账必须变成**可校验视图**：至少对每行校验 ①`enabled_env` 已在 `env-registry.toml` 登记；
②`wiring` 指向的生产调用点存在（可用 R3b `docs/status/wiring.yaml` 的 `production_refs` 或源码扫描）；
③`NotWired` 必须带 reason（现结构已强制）。校验放 `cargo test -p apeireth-runtime-assembly --lib`
（快、必跑），失败即红，不新增脚本依赖。

## 3. 开关面（9 开关 + 1 参数，解析散布 17 文件）

### 3.1 登记表实况（R3a `docs/architecture/env-registry.toml`，`APEIRETH_ENABLE_BIO_*` 9 条 + `APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN` 1 条 = 10 条）

| env 名 | 默认 | layer / owner | 登记 level |
|---|---|---|---|
| `APEIRETH_ENABLE_BIO_CARE_CANDIDATES` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_CIRCADIAN` | off | engine / runtime-assembly | PRODUCTION WIRED |
| `APEIRETH_ENABLE_BIO_CONTEXT_POLICY` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_LIFECYCLE` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_PAIN` | off | adapters / cli | PRODUCTION WIRED |
| `APEIRETH_ENABLE_BIO_PLAY` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_SELF_STATUS` | off | engine / runtime-assembly | UNCLASSIFIED |
| `APEIRETH_ENABLE_BIO_TONE` | off | engine / runtime-assembly | PRODUCTION WIRED |
| `APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN` | 参数（非开关） | engine / runtime-assembly | —（偏置分钟） |

`level` 口径不齐（3 条 PRODUCTION WIRED / 6 条 UNCLASSIFIED）是 R3b 填实遗留，不属 R1b 行为缺陷，
但 R1b 开关单入口落地后应同步刷新（开关唯一入口 + registry 唯一登记）。

### 3.2 解析点散布

bio env 常量在 **17 个 src 文件**中出现（`Select-String -List` 实测，§9-②）：

- `crates/adapters/cli/src/`：`lib.rs`、`care_heartbeat.rs`、`canonical_approval.rs`
- `crates/engine/runtime-assembly/src/canonical/`：`production.rs`、`bio_production_wiring.rs`、
  `bio_tone.rs`、`bio_lifecycle.rs`、`bio_circadian.rs`、`bio_mechanism_probes.rs`、
  `care_heartbeat.rs`、`care_reminder.rs`、`cognitive.rs`、`nervous_snapshot.rs`、`play_tools.rs`、
  `ritual_state_store.rs`、`ritual_tools.rs`、`self_status_source.rs`

安装函数内直接解析的三个代表点：

| 位置 | 代码 | 说明 |
|---|---|---|
| `bio_production_wiring.rs` `install_bio_modules` | `BioAssemblySwitches::from_env()` | 装配块（repair guidance + tone + lifecycle）开关 |
| `bio_production_wiring.rs` `install_bio_modules` | `bio_tone_wiring_from_env(...)` | 调质子开关 |
| `bio_production_wiring.rs` `install_bio_modules` | `std::env::var(BIO_LIFECYCLE_ENABLE_ENV)` | 生命周期存储目录 + 开关 |
| `bio_production_wiring.rs` `install_bio_self_status` | `bio_self_status_enabled_from_env_value(std::env::var(BIO_SELF_STATUS_ENABLE_ENV).ok()...)` | 自述开关 |
| `bio_tone.rs` / `bio_circadian.rs` / … | `*_enabled_from_env_value` | 机制模块自身仍读 env（R1a 台账解析复用同一批函数） |

**后果**：①单点审计/回滚不可能（关一个开关要改 N 处语义）；②新增开关必须在 17 个文件的读法之间
保持一致（现有 failure mode：某模块读 `"1"`、别的读 `"true"`）；③`arch_guard` 只能守"新 env 必须
登记"，守不住"同一开关多入口语义不一致"。

## 4. 装配根现状

### 4.1 生产装配根（`crates/engine/runtime-assembly/src/canonical/production.rs`，1,683 行）

| 行 | 调用 | 作用 |
|---|---|---|
| `:1179` | `bio_production_wiring::install_bio_play(...)` | 练习场四工具面 |
| `:1270` | `bio_production_wiring::install_bio_self_status(...)` | 自述 `bio_mechanisms` 挂面 + probe 注册 |
| `:1521` | `bio_production_wiring::install_bio_modules(...)` | 修复指令 + 调质 tone + 生命周期块，返回句柄 |
| `:1575` | `bio_probe_slots.fill_from_organ_module(organ_module.as_ref())` | 审议门 trace 迟绑定 |
| `:1594-1595` | 存 `bio_wiring.tone_state` / `bio_wiring.lifecycle_state` | `ProductionModules` 句柄 |
| `:1662` | `attach_immune_memory(memory)` | `fill_immune_memory` + OrganModule 免疫记忆 |

### 4.2 生物装配扩展点（`bio_production_wiring.rs`，853 行）

| 元素 | 说明 |
|---|---|
| `install_bio_modules(&mut modules, &clock, &capabilities, dir)` | 进 module 表（顺序即钩子派发顺序） |
| `install_bio_play(...)` / `install_bio_self_status(...)` | 工具面 / 自述挂面 |
| `BioProbeSlots` | 5 条迟绑定槽（immune / care / quorum / pain(+recovery) / play 走独立注册） + `arm(enabled)` |
| `BioWiringHandles { tone_state, lifecycle_state }` | 装配根回传句柄 |
| `ProductionModules::bio_tone_state()` / `attach_tone_feedback_sink()` / `bio_lifecycle_state()` / `bio_probe_slots()` | 只读句柄口 |

### 4.3 CLI 组合根（`crates/adapters/cli/src/lib.rs`，3,879 行）

| 行 | 调用 | 说明 |
|---|---|---|
| `:1433` | `build_canonical_runtime_assembly_from_env() -> (parts, BioProbeSlots, Option<Arc<ToneState>>)` | 组合根唯一装配入口 |
| `:1463` | `bio_probe_slots.fill_care_status(care_heartbeat.status_probe())` | 关怀心跳 probe 迟绑定 |
| `:1465` | `bio_tone = cognitive.bio_tone_state()` | 调质句柄取出（供审批面注入） |
| `:1492` | `bind_bio_pain_reader(...)`（`Weak<Runtime>`） | 痛觉 + 再生读取口 |
| `:1555` | `cognitive.attach_immune_memory(...)` | 免疫记忆 → 自述槽 + OrganModule |
| `:1521` | 域伙伴运行时 `fill_care_status(...)` | 逐域自述槽 |
| `canonical_approval.rs:36` | `apply_approval_social_feedback(bio_tone.as_ref(), feedback)` | 审批落账成功后旁路推进 `ToneState`（唯一真实适配器事件源） |

`crates/adapters/gateway/`、`crates/adapters/tui/`、`crates/adapters/im/`：`git grep` 生物机制接线
命中 = 0 —— 生物机制当前**只在 CLI 装配面**（gateway 只承载通用 `RuntimeEventSink`，见 §6）。

### 4.4 probe 注册 vs 填充（现状最脆弱的一环）

- **注册**（开关开时）：capacity（任务队列在场）、play（状态在场）、immune / attachment / quorum /
  nociception / recovery（5 条迟绑定恒注册，槽空 = 单格 `null` + 真因）→ 最多 **7 格可读**。
- **填充**：care（`cli/lib.rs:1463`、`:1521`）、immune（`production.rs:1575` 链中 / `:1662`）、
  quorum（`production.rs:1575`）、nociception + recovery（`cli/lib.rs:1492` 同一 reader）、play
  （装配时 state）、capacity（装配时 queue）。
- **时序依赖**：`install_bio_self_status` 先于 runtime `Arc` 化；pain 读取口必须用 `Weak` 反填
  （`cli/lib.rs:1489-1497`）。任何一处提前/漏填 = 该格永久 `null` 且**没有红灯**（只有 reason 字符串）。
- SG-18 lifecycle probe `bio_lifecycle_probe(state)`（`bio_lifecycle.rs:829`）**当前无人注册**——
  这是 R1b 必须补的最后一条迟绑定，也是 R1a 台账 SG-18 的唯一真实缺口。

## 5. 缺口清单（R1b 施工对象）

| ID | 缺口 | 证据 | 后果 | R1b 批次 |
|---|---|---|---|---|
| G1 | 无神经系统信号总线：机制各自 observe、各自消化 | `NervousSignal`/`NervousBus` grep = 0；`execute.rs:2447-2460` 唯一派发环 | 新增机制要重复写钩子适配；机制间无法通信；观测/计数口径分裂 | R1b-1 |
| G2 | 开关分散 17 文件、安装函数内读 env | §3.2 | 单点审计/回滚不可能；新增开关易语义漂移 | R1b-1 |
| G3 | 台账手写且 3 行已漂移 | §2.3 | 自述 `nervous_system` 对 owner 说假话（0 装反向） | R1b-1（校验） |
| G4 | probe 注册 / 填充时序分裂，漏填无红灯 | §4.4 | 14 格"可读"取决于组合根调用顺序 | R1b-2/3 |
| G5 | lifecycle probe 未注册 | `bio_lifecycle.rs:76-78,825-826`；`production.rs` 无 `BioMechanismKind::Lifecycle` | SG-18 永远 Implemented | R1b-2 |
| G6 | ritual 工具面已有、无信号广播 | W3.7 `7e98744d` + SG-22 旧态 | 仪式完成/跳过无法驱动其它机制订阅 | R1b-3 |
| G7 | `last_observation` 恒 null，读数分裂在 `bio_mechanisms` | 台账头部注释 + `observed` 口径 | R1b schema v2 需决定「观测时间」是否入总线（建议：总线带 `at_ms` 计数，快照仍只读） | R1b-4 |
| G8 | env-registry level 口径不齐（3 P-W / 6 UNCLASSIFIED） | §3.1 | 记分牌口径不可信；R1b 开关单入口后可一次刷新 | R1b-1（顺带） |

## 6. 已有事件面（不得误当"总线"）

| 面 | 位置 | 语义 | 为什么不能直接当神经系统总线 |
|---|---|---|---|
| `RuntimeEventSink` / `CompositeRuntimeEventSink` / `NoopRuntimeEventSink` / `FnRuntimeEventSink` | `crates/engine/runtime/src/canonical/events.rs:59-150` | turn 级事实（approval/tool/turn failed）单向 sink；src 内实现 4 处（gateway events / gateway presence / runtime-assembly guard_observer / events 自身） | 事件域是"外部观测"，不含机制状态语义；sink 不能反向消费/订阅 |
| gateway `EventBus`（tokio broadcast） | `crates/adapters/gateway/src/events.rs:155`（`subscribe` `:207`，`impl RuntimeEventSink` `:443`） | SSE / 观测 fan-out | 适配器层，async broadcast；机制在 runtime-assembly 内同步消费，跨界会把依赖方向反转（engine → adapters 禁止） |
| `ToneState` 直写 | `bio_tone.rs:295` `apply_feedback`；生产调用方 = `bio_tone.rs:426-467`（hook 映射）+ `canonical_approval.rs:36`（审批） | 调质状态推进 | 状态 ≠ 信号；其它机制无法订阅"调质发生了什么" |
| 四份反馈接口 | `orchestration/src/self_tuning.rs:416 TuningEvent`、`value_cases.rs`、`online_calibration.rs`、`runtime-assembly/src/canonical/orchestrator.rs:1662 OrchestratorFeedback` | 历史设计预留的反馈面 | **src 无生产调用方**（`git grep apply_feedback` 除 bio_tone 自身与测试外无真实事件源） |

**结论**：R1b 新建的是「业务语义层同步总线」，不是复用/扩展上述任一观测面；不复用 `RuntimeEvent`
（避免把外部观测 ABI 拖进机制层），不新增 crate。

## 7. 设计约束（写 ADR-0009 的硬边界）

1. **不新增 crate / 外部依赖**：总线放 `apeireth-runtime-assembly` 内部（`canonical/nervous_bus.rs`），
   用现依赖 `parking_lot`；不引 `crossbeam`/`tokio::sync`（钩子派发在 async 上下文，但机制消费必须同步，
   避免第二套异步生命周期）。
2. **总线不是第二真源**：只传信号（含 `at_ms` / 来源 / 计数），机制状态仍归各机制；订阅者 consume
   同步、不得阻塞；总线不做持久化。
3. **开关单入口**：新增 `NervousSwitchboard`（装配根构造一次，解析 9+1 env），通过句柄/参数注入
   模块安装函数；`bio_tone.rs` 等不再直接读 env。开关关 = 零订阅、零构造、零行为变化（延续 R1a
   `reads = 0` 测试口径）。
4. **台账可校验**：`nervous_snapshot.rs` 单测校验 ①enabled_env 在 `env-registry.toml` 中；
   ②`wiring` 指向的调用点存在（源码扫描或 `wiring.yaml` production_refs）；③`SignalState::NotWired`
   必须带 reason。禁止再手写"已接/未接"而不被检查。
5. **兼容**：`nervous_system` schema v1 形状冻结，R1b 只做 additive（v2 增加 bus 计数 / 每机制
   `signal_counts`）；自述 tool 渲染（`tools-canonical::self_status`）按新键增补，旧键不动。
6. **LOCKED 5 项 0 触碰**；不新增默认开开关；不改变现有开关默认值。
7. **实施线边界**：R1b-1（新增文件 + 装配根接线）可由架构线先行；R1b-2（机制迁移）必须等生物线
   当前 WIP 合入后交接（其 WIP 涉及 `canonical.rs` / `bio_production_wiring.rs` / `permission_preset.rs`
   / `production.rs`）；R1b-3（缺失信号源）生物线主笔，架构线出验收。

## 8. 迁移批次与验收

| 批次 | 内容 | 验收 |
|---|---|---|
| R1b-1 | `NervousSignal`（Closed enum：ToolOutcome / ApprovalSocial / ToneFeedback / LifecycleTransition / RitualDone / RitualSkip / SleepPressure / ScarLearn / SessionEvent / CapabilityReading 等）+ `NervousBus`（订阅表 + publish + 计数）+ `NervousSwitchboard`（9+1 解析）；台账校验单测；`production.rs` 唯一构造点；模块不再读 env 的第一批（tone/lifecycle/self_status/play） | 新单测（订阅顺序、开关关零订阅、重复订阅、计数、NotWired reason 强制）+ 原默认关测试逐字节不变；`cargo test -p apeireth-runtime-assembly --lib` 绿；guard 0 违规 |
| R1b-2 | 机制迁移：bio_tone / bio_lifecycle / care_heartbeat / bio_circadian / play / quorum 改为「bridge 单点观测 → bus 分发」；CLI 审批面改 `publish(ApprovalSocial)`；lifecycle probe 注册（补 G5）；台账 wiring 字段改 bus 路径 | 每模块默认关零变化；bio 集成测试绿；台账 3 行漂移清零；`observed` 计数不下降 |
| R1b-3 | 缺失信号源：ritual producer（`tool.ritual_done/skip` → bus）、sleep_pressure、scar learn、session event source；capacity/14 格可读性补齐或如实 NotWired | 各信号源有生产发布点 + 订阅测试；台账 14/14 逐行如实；0 装说明 |
| R1b-4 | 收口：schema v2（additive）、自述渲染、docs/contract 同步、记分牌 | `arch_guard` 0 / `docs_truth` 0 / `wiring_report --verify` 0；Wave 4 收口提交 |

## 9. 未决问题（ADR-0009 需裁决）

| # | 问题 | 建议 |
|---|---|---|
| Q1 | R1b-1 由架构线先做，R1b-2 交接生物线——是否接受交接时点以「生物线当前 WIP 提交 + 全量绿」为准？ | 接受；交接前架构线只做新增文件，不碰其 WIP 文件 |
| Q2 | 总线同步实现（`parking_lot::Mutex` 订阅表）vs async（tokio broadcast） | 同步；机制消费在钩子派发的同一任务内，语义确定、易测 |
| Q3 | 总线是否成为公共 ABI（`apeireth_runtime` 扩展点）还是 runtime-assembly 内部私有 | 内部私有；等 Wave 4 收口 + 真实订阅者稳定后再谈 ABI（避免过早冻结） |
| Q4 | 台账校验放单测（快）还是 `arch_guard` 新 W 项（跨真源） | 单测为主 + `wiring_report --verify` 已在 CI 兜底；不新增 guard 规则直到 R3c 到期 |
| Q5 | `last_observation` 是否由总线填（含 `at_ms`） | 总线只计数不填观测时间；快照 `last_observation` 仍 null，避免第二真源；如 owner 需要"最近信号时间"再单开 ADR |
| Q6 | ritual 工具面算「生产者已接」→ SG-22 信号源状态改 Partial？ | 是：工具调用是显式用户动作信号源；但"自动候选"仍未接，SG-22 级别保持 Half |

## 10. 证据命令（复核用；2026-10-20，HEAD `7e98744d`）

1. `git log --oneline -4` → `7e98744d` W3.7 ritual / `35c4b24a` cli 切片 / `96ef5c6b` Wave 3 收口 / `d17b6081` bio 文档。
2. `Select-String -Path crates/adapters/cli/src/*.rs,crates/engine/runtime-assembly/src/canonical/*.rs -Pattern 'APEIRETH_ENABLE_BIO_|BIO_[A-Z_]+_ENABLE_ENV|BIO_CIRCADIAN_TZ' -List` → **17 文件**。
3. `(Select-String -Path docs/architecture/env-registry.toml -Pattern '^name = "APEIRETH_ENABLE_BIO_|^name = "APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN"').Count` → **10 条**。
4. `git grep -n "pub struct NervousSignal" -- crates/` / `"pub struct NervousBus"` → **0 命中**。
5. `git grep -ln "impl RuntimeEventSink" -- crates/` → 4 个 src 文件（gateway `events.rs` / `presence.rs` / runtime-assembly `guard_observer.rs` / runtime `events.rs`）+ 1 测试文件。
6. `Select-String docs/status/wiring.yaml -Pattern 'id: "bio\.'` → **4 项**（`bio.tone` / `bio.circadian` / `bio.nociception_pain` / `bio.recovery_protocol`）；`claims_total: 20`。→ R3b wiring ledger 目前**只覆盖 4/14 机制**，R1b 台账校验需补 claims 覆盖或按机制组映射（待 ADR-0009 裁决）。
7. `python -c "..."` 行数：`nervous_snapshot.rs` 647 / `bio_production_wiring.rs` 853 / `bio_tone.rs` 980 / `bio_lifecycle.rs` 972 / `ritual_tools.rs` 698 / `self_status_source.rs` 2,064 / `production.rs` 1,683 / `execute.rs` 4,987 / `cli/lib.rs` 3,879。
8. 钩子派发：`execute.rs:2447-2460` 的 `for module in &self.modules { ... module.on_hook(hook, &context).await ... }`（8 个 `HookPoint` 变体）。
9. probe 注册：`bio_production_wiring.rs:390-403`（capacity + play + 5 迟绑定）；lifecycle probe 未注册证据：`bio_lifecycle.rs:76-78,825-826` + `git grep "BioMechanismKind::Lifecycle" -- crates/engine/runtime-assembly/src/` 仅命中 `bio_lifecycle.rs` 自身。
10. 审批注入：`canonical_approval.rs:36` `apply_approval_social_feedback(bio_tone.as_ref(), feedback)`；`git grep -n "apply_feedback" -- crates/` 生产调用方仅 `bio_tone.rs` 自身映射（`:458/:467`）与 `canonical_approval.rs` 路径。
