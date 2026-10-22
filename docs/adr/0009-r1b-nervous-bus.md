# 0009. R1b 神经系统——同步信号总线 / 开关单入口 / 台账可校验

- **Status**: Accepted（owner 2026-10-20 授权 Wave 4 按架构线建议推进；本文为 ADR-0002 已 Accepted 的 R1b 方向的实施形状裁决，owner 如反对具体条款可在 R1b-1 收口前驳回）
- **Date**: 2026-10-20
- **Owners**: 架构线出 spec/验收；R1b-1 结构内核架构线先行，R1b-2 机制迁移交接生物线，R1b-3 缺失信号源生物线主笔
- **Supersedes**: —
- **契约影响**: `docs/architecture/contract.toml` §meta.target_architecture.waves（追加 ADR-0009 R1b，Wave 4）；
  `[extension_points]` 不新增登记（总线为 runtime-assembly 内部私有面，非扩展点）；R1b-4 收口时更新 §meta.wave_note
- **例外关联**: —（R1b 全程不新增依赖边/开关/crate，不需要 EXC-*）
- **预算净额**: 依赖边 ±0 / 开关 ±0 / crate ±0 / 文件尺寸 +（新文件 `nervous_bus.rs` ≈600-800 行 + `production.rs` 接线 ≈20 行 + `nervous_snapshot.rs` 校验 ≈80 行）；新增文件在 R1b-1 收口时登记 size 基线条目（只增该类新文件条目，不抬既有预算）
- **证据包**: [r1b-inventory-2026-10-20.md](../architecture/r1b-inventory-2026-10-20.md)（HEAD `7e98744d` 实测）

## 1. 背景与问题

R1a 已交付神经系统**只读聚合**（`capabilities.nervous_system`，schema v1，14 机制台账），但 R1b 要解决的
三件事实无一存在（证据与命令见证据包 §2-§6）：

1. **没有跨机制信号总线**：`git grep "pub struct NervousSignal"` / `NervousBus` = 0；唯一有序观测点是
   模块钩子派发环 `crates/engine/runtime/src/canonical/execute.rs:2447-2460`，各模块各自 `on_hook`、
   各自过滤、各自消化。新增机制必须重写一遍「钩子→状态」适配；机制之间无法通信（例如仪式完成无法
   唤醒关怀候选、调质反馈无法驱动修复策略）。
2. **开关没有单入口**：9 个生物开关 + 1 个时区偏置参数，在 **17 个 src 文件**中被解析，且大量发生在
   模块安装函数内部（`BioAssemblySwitches::from_env()` / `bio_tone_wiring_from_env` /
   `std::env::var(BIO_LIFECYCLE_ENABLE_ENV)` / `bio_self_status_enabled_from_env_value(...)`）。
   单点审计/回滚不可能；新增开关极易读法漂移（`"1"` vs `"true"`）。
3. **台账手写且已漂移 3 行**：SG-18 的 lifecycle 模块已在 `production.rs:1521` 注册（真实缺口只剩
   probe 未注册，`bio_lifecycle.rs:76-78/:825-826`）；SG-22 的 W3.7 工具面 `tool.ritual_done` /
   `tool.ritual_skip` 已交付（`7e98744d`，未改台账）；SG-26 的「14 格只注册 1 格」在 W3.6 后已变为
   最多 7 格（`bio_production_wiring.rs:390-403`）。→ 手写台账必然再漂，自述会对 owner 说假话（0 装反向）。

同时，生产事件面已经存在四套不同语义（`RuntimeEventSink` 外部观测 / gateway `EventBus` async broadcast /
`ToneState::apply_feedback` 状态直写 / 四份无生产调用方的历史反馈接口），**都不能**直接当作神经系统总线：
前两套是外部观测域（依赖方向 adapters 在下），第三套是状态而非信号，第四套 src 无生产者。

**为什么现在决策**：ADR-0002 已把 R1b 列为 Wave 4 出口第一条；生物线当前 WIP 收口中，R1b-1 的结构内核
（新增文件、零行为变化）与其零重叠，可以先行；若等所有机制批次收口再启动，Wave 4 将空转，且后续机制
批次会继续按「各自 observe」写，迁移成本单调上升。

## 2. 决策

**一句话**：在 `apeireth-runtime-assembly` 内新增**内部私有、同步**的神经系统信号总线 + 单一桥模块 +
装配根开关表；适配器只 publish 信号，机制只订阅信号，台账改为可校验视图。不新增 crate、不新增外部依赖、
不改任何开关默认值、不做公共 ABI。

### D1 信号类型与总线（R1b-1）

- 新文件 `crates/engine/runtime-assembly/src/canonical/nervous_bus.rs`：
  - `NervousSignalKind`（首版 2 类）：`ToolOutcome` / `ErrorObserved`——只承载**已发生的事实**
    （§8 变更记录：首版收窄，理由见下条）。
  - `NervousSignal`（闭合 enum，首版 2 变体）：`ToolOutcome { ok, tool: Option<String>, retryable }`、
    `ErrorObserved { message }`。信号**只承载事实，不承载机制状态、不带时间戳**（需要时间的订阅者自取时钟；
    总线不做时钟）。其余信号（审批社交事实 / 调质反馈 / 生命周期迁移 / 仪式完成与跳过 / 睡眠压力 /
    疤痕学习 / 会话事件 / 能力读数）**不预定义**：闭合枚举按「生产者与订阅者同批落地」additive 扩展，
    由 R1b-2 / R1b-3 逐批加入。0 装口径：不定义当前无生产者的信号，避免「类型存在 = 事实存在」的假象。
  - `NervousBus`（`Clone + Default`，内部 `Arc` 共享）：`subscribe(kind, Arc<dyn Fn(&NervousSignal) + Send + Sync>)
    -> usize`（订阅票号）、`publish(&NervousSignal) -> usize`（投递数）、`subscriber_count(kind)`、
    `published_count(kind)`、`delivered_total()`、`is_idle()`、`snapshot() -> NervousBusSnapshot
    { subscribers, published, delivered }`；订阅表 `parking_lot::RwLock<BTreeMap<kind, Vec<..>>>`，
    publish 时按订阅顺序**同步**调用，回调在锁外执行（不持锁调用外部代码）；总线**无持久化、无背压、
    不跨进程**。
  - 订阅者约定（文档 + 评审约束）：同步、不得阻塞、不返回错误（订阅者自身状态失败写自己的 reason，
    不上抛成回合失败）。闭包形状替代 trait 形状：订阅者只需一个回调，闭包免去为每种订阅者建类型；
    §8 记录该实施期形状变更。
- 位置约束：模块内部私有（`pub(crate)`/`pub(super)` 面），**不**挂到 `apeireth_runtime` 公共 ABI，
  **不**在 `contract.toml [extension_points]` 登记；R1b-4 收口后如真实订阅者稳定再单开 ADR 讨论 ABI。

### D2 单一桥模块（R1b-1 骨架，R1b-2 迁移完成）

- 新增 `NervousBridgeModule`（实现 `BehaviorModule`，`manifest().id = "nervous.bridge"`），**注册在模块表首位**；
  它是唯一把 `HookPoint` 派发映射为 `NervousSignal` 的点。R1b-1 首版只映射**有确定事实源**的两个钩子：
  `AfterToolResult` → `ToolOutcome`（ok / tool / retryable）、`OnError` → `ErrorObserved`（message）；
  其余钩子返回 `None`（`TurnStart` 会话事件、`AfterTurn` 睡眠压力等要等 R1b-2/R1b-3 的生产者落地后
  再在同一桥里 additive 加映射，避免桥先编造事实）。桥恒返回 `ModuleOutcome::continue_()`——
  发布信号不改变回合结果、不阻塞、不上抛错误。
- R1b-1 只落桥 + 总线 + 计数（不迁移既有机制行为：既有模块仍各自 observe，桥只新增一路无人订阅的
  publish，默认关 = 零构造）；R1b-2 再把 `bio_tone` / `bio_lifecycle` / `care_heartbeat` /
  `bio_circadian` / `play` / `quorum` 的观测改成订阅总线，删除各自重复过滤。
- 迁移完成的判据：`bio_tone.rs` 不再从 `HookPoint` 直接调 `apply_feedback`（改为订阅 `ToneFeedback`）；
  `bio_lifecycle.rs` 不再自己过滤 `AfterToolResult`；每模块默认关测试逐字节不变。

### D3 开关单入口：`NervousSwitchboard`（R1b-1）

- 新增 `NervousSwitchboard`（值类型、`Clone + Copy` 可空），由装配根**一次**解析 9 开关 + 1 参数：
  `care_candidates / circadian / context_policy / lifecycle / pain / play / repair_guidance /
  self_status / tone / circadian_tz_offset_min`。
- 安装函数签名改为接收 `&NervousSwitchboard`（或显式 bool 参数保持测试友好）；模块内部**不再**
  `std::env::var`。env 读取点从 17 个文件收敛到 1 处（`production.rs` 装配根）+ R1a 只读解析复用。
- 开关关 = 零订阅、零构造、零行为变化（延续 R1a `reads = 0` 测试口径）；解析语义与原各解析函数
  逐字节一致（允许 `"1"` 的开关不得突然接受 `"true"`——R1b-1 用现有 `*_from_env_value` 函数，
  只把「读 env」这一步挪到装配根）。
- **实施期口径（§8）**：R1b-1a 的 switchboard 首版按「只认 `"1"`、其余 fail-closed（视为关）」实现，
  并以单测钉住「恰好查询 9 开关 + 1 参数各一次」。R1b-1b 迁移时必须逐开关对齐原解析语义
  （若某开关原解析接受 `"true"`/其它值，迁移批必须行为等价并补等价测试），**不一致就不删原解析路径**。

### D4 适配器只 publish，不直写机制状态（R1b-2）

- CLI 审批面 `canonical_approval.rs:36` 的 `apply_approval_social_feedback(bio_tone.as_ref(), feedback)`
  改为 `bus.publish(NervousSignal::ApprovalSocial { kind, at_ms })`；`bio_tone` 机制订阅后自行
  `apply_feedback`。适配器不再需要 `Option<Arc<ToneState>>` 句柄（句柄只留在装配根/机制内）。
- 同一规则适用于后续一切适配器事件（gateway 若接入生物机制，也走信号而非句柄直写）。

### D5 台账可校验（R1b-1）

`nervous_snapshot.rs` 单测（随 `cargo test -p apeireth-runtime-assembly --lib` 必跑）：

1. 每行 `enabled_env`（`Some` 时）必须在 `docs/architecture/env-registry.toml` 登记（解析 TOML，
   只读字符串匹配，不新增依赖——rust 侧用 `include_str!` + 人工可读的最小解析或改为在 CI 由
   `wiring_report.py` 覆盖；选型见 D5 实施注：优先 `include_str!` + 简单行扫描，避免在测试里引 toml 依赖）。
2. 每行 `wiring` 必须能在源码中定位到调用点（扫描本 crate `src/` 的字符串，失败即红）；`NotWired`
   必须带非空 `reason`（结构已强制）。
3. 更正 §2.3 三行漂移（SG-18/SG-22/SG-26）；SG-22 信号源改 `Partial`（显式用户动作已有生产者，
   自动候选仍未接），机制级别保持 `Half`；SG-26 gap 文案改为「已注册 7 格 probe，可读性依赖句柄填充」。
4. `wiring.yaml`（R3b 生成物，当前只覆盖 4/14 机制、claims 20 项中 bio.* 4 项）**不**作为 R1b 台账的
   唯一真源：R1b 台账校验只依赖「env-registry + 源码调用点 + NotWired reason」三件；补 claims 覆盖归
   R3b 后续批次（不阻塞 R1b）。此条为 Q4/Q7 裁决。

### D6 缺失信号源与 lifecycle probe（R1b-2/R1b-3）

- R1b-2：装配根注册 `bio_lifecycle_probe(state)`（补 SG-18 唯一真实缺口），台账 SG-18 改
  `wiring = production-assembly::bio_production_wiring.install_bio_modules + probe register`。
- R1b-3：`tool.ritual_done` / `tool.ritual_skip` 的落账点 publish `RitualDone/RitualSkip`；sleep pressure、
  scar learn、session event source 逐条补（补不了的如实保留 `NotWired` + reason，0 装）。
- 不新增开关：上述信号源均挂在既有开关下（ritual 在 care candidates 面，lifecycle 在 lifecycle 开关面）。

### D7 schema 版本

- `nervous_system` schema v1 形状**冻结不变**；R1b-4 只做 additive `v2`：新增 `bus` 子对象
  （`subscribers` / `published` / `delivered` 聚合计数 + `last_observation`）。`last_observation` **仍** 为
  `null`（不填「最近信号时间」，避免第二真源；Q5 裁决）。
- **实施期口径（§8 变更记录）**：R1b-4a 首版只落**聚合计数**（避免按 kind 的 map 变成未经消费的第二计数
  真源）；按 kind 明细若真实订阅方需要，按「生产者+消费者同批 additive」原则另开小版本，不在 v2 预定义。

### 不改什么

- 不改 `contract.toml` 允许层边/预算/开关登记（switches ±0）；不改 `workspace.version` / LOCKED 5 项；
  不改任何开关默认值（全部默认关）；不新增 crate / 外部 dep；不碰 `crates/**` 之外除 docs/baseline 外的面；
  不把总线做成公共 ABI；不在 R1b-1 迁移既有机制行为。

## 3. O-6 三阶审查（各含拒的 alternatives + 拒理由）

- **总体最优**：Wave 4 从 R1b-1 结构内核切入，是「最小可增量 + 最大风险前置」的切点：生物线当前 WIP
  收口前的空窗正好落新增文件；R8（治理单一管线）与 R6（验证架构）不依赖 R1b，可并行后置。
  拒「先 R8/R6」——它们与生物线在途 WIP 无冲突但也不解锁任何 Wave 4 出口；拒「等所有机制批次收口再启动
  R1b」——空转且后续批次继续写各自 observe，迁移成本单调上升；拒「只改台账不建总线」——G1（无总线）
  是 Wave 4 出口条款，台账只是诚实性问题的一半。
- **系统最优**：总线放 `engine/runtime-assembly`（机制层同层）——生物机制的共享状态（ToneState /
  BioLifecycleState / probe 槽）全部已在该 crate；依赖方向不动（无新边、无循环）。
  拒「放 `engine/runtime`」——runtime 是回合编排核心，不应认识生物机制 DTO，否则 bio 依赖拖进核心；
  拒「放 `foundation/orchestration`」——orchestration 是纯内核（无 tokio/parking_lot 依赖面，且
  runtime-assembly 已依赖它，反向会成环）；拒「放 adapters / 复用 gateway EventBus」——engine→adapters
  方向反转，`arch_guard` 直接禁；拒「新建 `apeireth-nervous` crate」——新增 crate 是结构性成本（预算、
  层规则、发布面），当前收益不成立。
- **架构最优**：同步 + 内部私有 + 单一 bridge 三个形状组合最优：同步与钩子派发同一任务、语义确定、
  易测（无 async 生命周期）；内部私有避免过早冻结公共 ABI（R2 刚收口扩展 ABI，神经总线还不是稳定
  扩展点）；单一 bridge 让「钩子→信号」只有一处映射，机制侧只写订阅逻辑。
  拒「tokio broadcast（async）」——第二套异步生命周期、错误/取消语义重复、测试要 runtime；
  拒「复用 `RuntimeEventSink`」——那是外部观测 ABI（turn 级事实），把机制信号塞进去会污染观测面并
  迫使市场面理解内部机制；拒「每机制自建 channel」——回到 G1 现状、无统一计数与校验。

## 4. 迁移计划（分步、每步可验证）

1. **R1b-1a 总线内核 + 桥骨架 + 开关表**（架构线，新增文件为主）
   - 新增 `nervous_bus.rs`（`NervousSignalKind` / `NervousSignal` / `NervousBus`）、
     `nervous_switchboard.rs`（9 开关 + 1 参数解析）、`nervous_bridge.rs`（`NervousBridgeModule`，
     装配根仅在 `NervousSwitchboard::any_enabled()` 为真时注册；全关 = 不注册 = 零行为变化）。
   - `production.rs` 唯一构造点：装配根解析 switchboard → 传给 `install_bio_*`；桥仅在
     `APEIRETH_ENABLE_BIO_SELF_STATUS`（或独立内部条件）打开时注册？→ **裁决：桥不新增开关**，
     与总线一起仅由装配根构造、默认零订阅（无订阅者 = publish 为空操作），不改变任何默认行为。
   - 验证：`cargo test -p apeireth-runtime-assembly --lib` 绿；现有默认关测试不变；`arch_guard` 0。
2. **R1b-1b 开关迁移 + 台账校验**（架构线）
   - 模块安装函数改收 switchboard/显式 bool；`git grep "std::env::var" crates/engine/runtime-assembly/src/canonical/{bio_*,care_*,play_tools,ritual_*,cognitive}.rs` 收敛到装配根 1 处（R1a 只读解析函数保留）。
   - `nervous_snapshot.rs` 加 3 条校验单测 + 更正 3 行漂移；`wiring_report.py --verify` 绿。
   - 验证：`cargo test -p apeireth-runtime-assembly --lib`；`cargo clippy -- -D warnings`；`cargo fmt --check`；
     `python scripts/arch_guard.py --report` VIOLATIONS 0 / STALE 0。
3. **R1b-2 机制订阅迁移 + lifecycle probe**（生物线主笔，交接条件：其当前 WIP 已提交且全量绿；
   交接单：`docs/architecture/r1b-2-handoff-2026-10-20.md`）
   - bio_tone / bio_lifecycle / care_heartbeat / bio_circadian / play / quorum 改订阅；审批面改 publish；
     lifecycle probe 注册；台账 wiring 字段更新为 bus 路径。
   - 验证：每模块默认关零变化测试；`bio_*` 集成测试；全量门禁由全量线执行并广播。
   - 批次拆分（交接单 §3）：R1b-2a bio_tone 订阅 → R1b-2b bio_lifecycle 订阅 + probe → R1b-2c 审批面
     publish；care/circadian/play/quorum 的消费面迁移依赖 R1b-3 信号源，不在本批。
4. **R1b-3 缺失信号源**（生物线；交接单：`docs/architecture/r1b-3-handoff-2026-10-20.md`）
   - **2026-10-21 已由架构线交付（零新增信号变体）**：ritual 不改代码（文件即通道：工具写
     `<care_dir>/ritual_state.json`、care heartbeat 同拍读盘），只补端到端测试与陈旧注释清理；
     sleep pressure / scar learn / session event source 三者如实保持 `NotWired` + 具体 reason。
   - 原计划 ritual / sleep pressure / scar learn / session event source；补不了的保留 NotWired + reason。
   - 主管线复核（2026-10-20）收窄为**零新增信号变体**：ritual 已由 `RitualStateStore` 共享文件打通
     （工具写盘 → CLI care heartbeat 同拍读盘），只补端到端测试与陈旧注释清理；sleep pressure 为模块内
     自消费、scar learn 无生产者/订阅者、session event 的真实消费方需 `UncertaintySignal` 而非回合边界
     ——三者如实保持 `NotWired` + 具体 reason，待真实消费方出现再按"生产者+订阅者同批"原则补批次。
   - 验证：ritual 端到端测试；台账逐行如实；交接单 §4 清单逐项勾；总线无新增变体。
5. **R1b-4 收口**（架构线）
   - schema v2（additive）、自述渲染、contract wave_note / handbook / integration-plan 记分牌、
     基线新增文件条目；`docs_truth.py --verify --check`；Wave 4 收口提交。

**并行线协调**：R1b-1 只新增文件 + `production.rs` 接线（生物线当前 WIP 不在 production.rs，冲突面小）；
若生物线在途改到 `production.rs`，R1b-1 暂缓该文件接线、先落新文件与测试，接线批次后移（不抢占）。

## 5. 回滚

- R1b-1：`git revert <R1b-1 commit>` 即可——新文件整体删除、`production.rs` 接线行还原、无契约/基线
  变更（新增文件条目在收口时才有）；回滚后 CLI 行为与 HEAD 逐字节一致（桥默认零订阅）。
- R1b-2/3：每批独立 commit、可独立 revert；迁移回滚 = 恢复模块内 `on_hook` 过滤 + 审批面直写句柄
  （两处都在同文件，`git revert` 一步）；台账 wiring 字段随批回滚。
- 契约/基线：R1b-1..3 不改 `contract.toml`；R1b-4 收口才更新 wave_note 与 size 条目，回滚按收口 commit
  的 baseline diff 还原。
- 回滚后需重跑：`cargo test -p apeireth-runtime-assembly --lib`、`python scripts/arch_guard.py --report`。

## 6. 验收

- [x] R1b-1（**内核面已交付**，commit `10ec68ae`：bus 5 / switchboard 8 / bridge 9 单测全绿；装配根仅
      任一开关开启时注册桥，当前生产无订阅者）：`NervousSignalKind`/`NervousSignal`/`NervousBus`/
      `NervousSwitchboard`/`NervousBridgeModule` 落地；`arch_guard --report` 仅余已知 W1（R7-2.1 导入 +2 行）
- [x] R1b-1：开关读取点收敛（**已交付** `6837d9b7`：runtime-assembly 四个生物安装函数收
      `NervousSwitchboard`/显式 bool，`BioAssemblySwitches::from_env` 等 6 个 env 读取构造删除；
      模块内生物开关 `std::env::var` = 0；CLI 侧 care/pain/circadian 读取点属 R1b-2/3，如实保留；
      默认关行为零变化，测试改显式布尔入口）
- [x] R1b-1：台账漂移更正（**4 行**：SG-18/SG-25 → PRODUCTION WIRED、SG-22 ritual_producer → Wired、
      SG-26 缺口实况）+ 3 条校验单测（env-registry / wiring 装配根且无 pending / NotWired reason +
      parser fail-closed），`6837d9b7`
- [x] R1b-2a/2b（**已交付并复核** 2026-10-21）：`bio_tone` 事件源收敛到 `BioToneBusSubscriber`
      订阅 `ToolOutcome`+`ErrorObserved`（`404919f6`，交付记录 `326e1542`）；`bio_lifecycle` 工具结果面
      收敛到 `BioLifecycleBusSubscriber`（`AfterTurn` 仍走钩子，回合边界信号归 R1b-3），lifecycle
      观测格经迟绑定槽注册（8/14 → 9/14，仅开关全开）（`a71639f9`，交付记录 `0f4ef4dc`）；
      默认关零行为变化；验收记录见 `r1b-2-handoff-2026-10-20.md` §5/§8
- [x] R1b-2c（**2026-10-21 生物线交付、架构线复核**）：`NervousSignalKind::ApprovalSocial` 按 §8「生产者+订阅者同批 additive」新增——审批社交发布路径与调质订阅同批落地，默认关零行为变化；
      证据：代码 `f2dad300` + 交付记录 `815d85e7`；全量门禁 Wave 4 收口批绿（`artifacts/wave4-close-full-2026-10-21.log`，341 套件 / 8570 passed / 0 failed）。
- [x] R1b-3（**2026-10-21 交付，架构线**）：零新增信号变体——ritual 走 `RitualStateStore` 文件通道
      （新增端到端测试 `a_ritual_tool_write_is_visible_to_the_next_heartbeat_beat`：工具写盘 → 同拍读到
      `last_done_ms` → 两周期后产仪式候选）；`sleep_pressure`（门内自消费）/ `scar_ledger_learn`
      （无生产者-订阅方）/ `session_event_source`（消费方需 `UncertaintySignal`）三行如实 `NotWired`
      + 具体 reason；陈旧注释清理；交付记录见 `r1b-3-handoff-2026-10-20.md` §6 / §8 变更记录
- [x] R1b-4a：schema v2 additive（**2026-10-20 提前交付**）：`nervous_system.bus` 报
      subscribers/published/delivered 聚合计数，`last_observation` 恒 null；未绑总线时 `bus=null`；
      v1 字段逐字节不变；装配根把唯一 `NervousBus` 经 `SelfStatusModule::new_with_bus` 接入
- [x] R1b-4b（收口项，**2026-10-21 收口批完成**）：contract wave_note / handbook / integration-plan 记分牌同步；
      `docs_truth.py --verify --check` exit 0；`wiring_report.py --verify` exit 0；全量数字见 test-baseline §1。
- [ ] LOCKED 5 项 0 触碰 / 0 新外部 dep / 0 新开关 / 0 新 crate

## 7. 后果（Consequences）

- 正面：机制新增从「重写钩子适配」变成「订阅信号」；开关单点审计/回滚；自述台账不再说假话；
  适配器不再持有机制状态句柄（依赖收窄）；为 R8（治理单一管线）提供统一信号面。
- 负面/接受的代价：同步订阅者若自身慢会拖慢回合（文档 + 评审约束，后续如需要可再 ADR 讨论异步面）；
  总线首版不持久化（重启丢计数，自述 `last_observation` 仍 null）；R1b-2 交接存在排期依赖（生物线
  WIP 收口是前置），交接前 R1b 只到 R1b-1b。
- 后续 stage：R1b-1a 于 2026-10-20 交付（`10ec68ae`，内核/桥/开关表 + 装配根单入口）；R1b-1b 于同日交付
  （`6837d9b7`，开关读取收敛到装配根 + 台账 4 行更正 + 3 条校验）；**R1b-4a 提前交付**（2026-10-20，
  `nervous_system` schema v2 additive：`bus` 聚合计数观测面，v1 字段不变）；R1b-2a/2b 由生物线交付并复核
  （2026-10-21）；**R1b-2c 于同日由生物线交付并复核**（`ApprovalSocial` 同批 additive）；**R1b-3 于 2026-10-21 由架构线交付（零新增信号变体，见 §8）**；后续
  R1b-4b 于 2026-10-21 收口批完成。
  R7-2 遗留 `ContextAssembler` 已由 R7-2.1 收口（`c8cb34a4`）；Wave 4 其余 = R8 + R6 spec 已 Accepted
  （ADR-0010/0011），实施归安全线/效率线；R1b-4b 之后再评估是否把总线升为扩展点（需新 ADR）。

## 8. 变更记录

- **2026-10-20（实施期收窄，R1b-1a 施工中）**：D1 的信号面从「10 变体 + 时间戳 + `NervousSubscriber`
  trait」收窄为「2 变体事实信号（`ToolOutcome` / `ErrorObserved`）+ 闭包订阅 + 计数快照」；D2 的桥首版
  只映射 `AfterToolResult` / `OnError`。理由：①0 装——不定义当前无生产者的信号（其余 8 类要到
  R1b-2/R1b-3 才有发布点，预定义会让「类型存在」被误读为「事实存在」）；②时间戳应由需要时间的订阅者
  自取，总线不做时钟可少一条注入面；③闭包比 trait 少一层类型样板，订阅者只需一个回调。其余信号按
  「生产者与订阅者同批 additive」扩展，扩展时 D1 的「闭合 enum」约束与 §6 验收同步更新。
- **2026-10-20（D3 增补）**：switchboard 首版只认 `"1"`、其余 fail-closed；R1b-1b 迁移时逐开关对齐原
  解析语义（行为等价 + 等价测试），不一致就不删原解析路径。
- **2026-10-20（R1b-1a 交付）**：`nervous_bus.rs` / `nervous_switchboard.rs` / `nervous_bridge.rs` 三个新文件
  与 `production.rs`/`canonical.rs` 装配根接线落地（commit `10ec68ae`，5 文件 +1048 行）；桥仅当任一生物开关
  开启时注册为模块表首位，全关零行为变化；`ProductionModules` 暴露只读句柄供 R1b-2/3 使用。实测：
  `cargo check`/`test -p apeireth-runtime-assembly --lib` 274 passed、clippy/fmt 绿。**如实边界**：生产仍无任何
  信号订阅者，桥尚未被真实工具链路触发；信号源与消费方接线属 R1b-2/3，未做。
- **2026-10-20（R1b-1b 交付）**：开关读取收敛到装配根单入口（commit `6837d9b7`，13 文件 +257/−222）。
  `install_bio_modules`/`install_bio_play`/`install_bio_ritual_tools`/`install_bio_self_status` 改收
  `&NervousSwitchboard`/显式 `bool`，删除 `BioAssemblySwitches::from_env`、`BioAssemblyModule::from_env`、
  `bio_tone_wiring_from_env`、`bio_play_wiring_from_env`、`bio_ritual_wiring_from_env`、
  `bio_lifecycle_wiring_from_env` 六个 env 读取构造；`production.rs` 四个调用点传 switchboard 解析值。
  台账更正 4 行（SG-18/SG-25 → PRODUCTION WIRED、SG-22 ritual_producer → Wired、SG-26 缺口实况），
  新增 3 条结构校验单测。实测：`cargo check -p apeireth-runtime-assembly --all-targets` exit 0；
  `cargo test -p apeireth-runtime-assembly --locked -j 2` 62 目标全 ok、lib 277 passed、nervous_snapshot
  11 passed；clippy/fmt 绿；`docs_truth.py --verify --check` exit 0。**如实边界**：CLI 侧 care 心跳、痛觉/
  再生、节律门仍各自读 env（R1b-2/3 收敛）；`NervousSwitchboard` 的 CareCandidates/Pain/Circadian 三格
  本批只解析记录、暂无新消费方；arch_guard 仍余已知 W1（cognitive.rs 4757→4763，Wave 4 收口重钉）。
- **2026-10-20（R1b-4a 提前交付）**：`nervous_system` schema v1→v2（additive）：新增 `bus` 子对象
  （`subscribers` / `published` / `delivered` 聚合计数 + `last_observation` 恒 `null`）。生产路径：
  `production.rs` 把装配根唯一 `NervousBus` 传给 `SelfStatusModule::new_with_bus`，帧内
  `capabilities.nervous_system.bus` 现取计数；未绑总线（如普通工具构造/测试）时 `bus = null`，不编造。
  `tool_modules.rs` 新增 `new_with_bus` 并把原构造体抽成私有 `with_tool`（+12 行，登记 `EXC-0003`
  size 类 weight=1，expires_wave=6）；v1 三字段形状逐字节不变。实测：`cargo check
  -p apeireth-runtime-assembly --all-targets` exit 0；lib 277 passed；`tests/nervous_snapshot.rs`
  13 passed（新增 bus 计数/int 测试）；clippy/fmt 绿；`arch_guard --report` VIOLATIONS 0
  （EXC-0002/0003 抑制 NOTE）+ STALE 1（play_tools 1414，收口缩表）。**如实边界**：生产仍无信号订阅者
  （R1b-2 才接），故计数实际为 0；按 kind 明细未落（见 D7 实施期口径）。

### 2026-10-21（R1b-2a/2b 验收复核）

- **复核对象**：`404919f6`（R1b-2a 代码）+`326e1542`（交付记录）、`a71639f9`（R1b-2b 代码）+
  `0f4ef4dc`（交付记录）。
- **结论**：符合 ADR-0009 §4 与 `r1b-2-handoff-2026-10-20.md` §3/§5；默认关零行为变化成立
  （订阅者只在开关开路径构造，桥在全关时不注册）；观测面只有桥一处发布点（两批均未新增
  `NervousSignalKind`）；台账 SG-18/SG-26 与观测面 9/14 文本同步；3 条台账校验单测仍在。
- **未做（如实）**：R1b-2c 审批面 publish 未开工（需新增 `ApprovalSocial` 信号，按 §8
  「生产者+订阅者同批 additive」办）；lifecycle `AfterTurn` 回合收口仍走钩子（总线暂无回合
  边界信号，归 R1b-3）；全 workspace 全量门禁未在复核时由架构线复跑（归 Wave 4 收口批）。
- **复核证据**：`arch_guard --report` VIOLATIONS 0（exceptions=2，STALE 1 = play_tools 待缩表）；
  `docs_truth --verify --check` exit 0；生物线未伪造 `claims.yaml` 的 `verified_at`。

### 2026-10-21（R1b-3 交付，架构线）

- **范围**：零新增信号变体。ritual 生产-消费通道已存在（`tool.ritual_done` / `tool.ritual_skip` 经
  `RitualStateStore` 写 `<care_dir>/ritual_state.json`，CLI care heartbeat 每拍同路径读取）→ 不改代码，
  只补端到端测试 + 清理陈旧注释；`sleep_pressure` / `scar_ledger_learn` / `session_event_source` 三源
  取证后如实保留 `NotWired` + 具体 reason（门内自消费 / 无生产者-订阅方 / 消费方需 `UncertaintySignal`）。
- **验证**：新增 cli 测试 `care_heartbeat::tests::a_ritual_tool_write_is_visible_to_the_next_heartbeat_beat`
  （工具执行写盘 → 同拍读到 `last_done_ms` = 注入时钟值 → 刚完成不重复到点 → 两周期后产仪式候选）；
  `cargo test -p apeireth-runtime-assembly --lib` 291 passed（含生物线在途 WIP 测试）；
  `--test nervous_snapshot` 13 passed；`cargo test -p apeireth-cli --lib` 57 passed；clippy/fmt 绿；
  `arch_guard --report` VIOLATIONS 0（exceptions=2 / STALE 1 不变；cli/lib.rs 尺寸 W1 属生物线 2c 在途）。
- **如实边界**：本批未新增任何 enum 变体/开关/依赖；三源 `NotWired` 是取证结论不是占位；全量门禁数字
  归 Wave 4 收口批。
- **交付记录**：`docs/architecture/r1b-3-handoff-2026-10-20.md` §4/§6。
### 2026-10-21（R1b 收口：2c 复核 + 4b 收口，Wave 4 关闭）

- **R1b-2c 复核**：`NervousSignalKind::ApprovalSocial` 属 §8「生产者+订阅者同批」的 additive 小版本；
  默认关零行为变化成立（生物线 scoped 测试 + 全量门禁同批验证）。
- **R1b-4b 收口**：`contract.toml` wave_note / handbook §3 / `integration-plan §4` 记分牌同步；
  `docs_truth --verify --check`、`wiring_report --verify` 绿；基线缩表 2 条；`current_wave` 4→5。
- **实测**：`pwsh scripts/gate_full.ps1 -FailIfBusy` = main 264 套件 / 6263 passed / 19 ignored +
  image 家族 77 套件 / 2307 passed / 32 ignored → **341 套件 / 8570 passed / 0 failed / 51 ignored**
  （2026-10-21，26.8 min，exit 0，HEAD `f2dad300`，日志 `artifacts/wave4-close-full-2026-10-21.log`）。
- **未做（如实）**：全量数字由架构线以 R6-A 唯一执行器代跑广播，全量门禁执行线接管后以其实测刷新；
  R8-B2/B3、R6-C/D 签核、夜间 full 首次运行仍归他线。

