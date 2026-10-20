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
  - `NervousSignal`（闭合 enum，首版）：`ToolOutcome { ok, tool, at_ms }`、`ApprovalSocial { kind, at_ms }`、
    `ToneFeedback { kind, at_ms }`、`LifecycleTransition { kind, at_ms }`、`RitualDone { at_ms }`、
    `RitualSkip { at_ms }`、`SleepPressure { value, at_ms }`、`ScarLearn { kind, at_ms }`、
    `SessionEvent { kind, at_ms }`、`CapabilityReading { kind, at_ms }`。信号**只承载事实，不承载机制状态**。
  - `NervousBus`：`subscribe(SubscriberId, Arc<dyn NervousSubscriber>)`、`publish(NervousSignal)`、
    `counters()`（按信号 kind 计数）、`subscriber_count()`；订阅表 `parking_lot::RwLock<Vec<..>>`，
    publish 时按订阅顺序同步调用；订阅者不得阻塞（文档约束）；总线**无持久化、无背压、不跨进程**。
  - `NervousSubscriber` trait：`fn on_signal(&self, signal: &NervousSignal);` —— 同步、无 async、
    不返回错误（订阅者自身状态失败写自己的 reason，不上抛成回合失败）。
- 位置约束：模块内部私有（`pub(crate)`/`pub(super)` 面），**不**挂到 `apeireth_runtime` 公共 ABI，
  **不**在 `contract.toml [extension_points]` 登记；R1b-4 收口后如真实订阅者稳定再单开 ADR 讨论 ABI。

### D2 单一桥模块（R1b-1 骨架，R1b-2 迁移完成）

- 新增 `NervousBridgeModule`（实现 `Module`，`manifest().id = "nervous.bridge"`），**注册在模块表首位**；
  它是唯一把 `HookPoint` 派发映射为 `NervousSignal` 的点：`AfterToolResult`（ok/fail）、`OnError`、
  `TurnStart`（会话事件）、`AfterTurn`（sleep pressure 等）。
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
  （`signals_total` / 按 kind 计数 / 订阅者数）。`last_observation` **仍** 为 `null`（不填「最近信号时间」，
  避免第二真源；Q5 裁决）。

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
   - 新增 `nervous_bus.rs`（`NervousSignal` / `NervousBus` / `NervousSubscriber`）、`nervous_switchboard.rs`
     （10 字段解析）、`nervous_bridge.rs`（`NervousBridgeModule`，默认关不注册）。
   - `production.rs` 唯一构造点：装配根解析 switchboard → 传给 `install_bio_*`；桥仅在
     `APEIRETH_ENABLE_BIO_SELF_STATUS`（或独立内部条件）打开时注册？→ **裁决：桥不新增开关**，
     与总线一起仅由装配根构造、默认零订阅（无订阅者 = publish 为空操作），不改变任何默认行为。
   - 验证：`cargo test -p apeireth-runtime-assembly --lib` 绿；现有默认关测试不变；`arch_guard` 0。
2. **R1b-1b 开关迁移 + 台账校验**（架构线）
   - 模块安装函数改收 switchboard/显式 bool；`git grep "std::env::var" crates/engine/runtime-assembly/src/canonical/{bio_*,care_*,play_tools,ritual_*,cognitive}.rs` 收敛到装配根 1 处（R1a 只读解析函数保留）。
   - `nervous_snapshot.rs` 加 3 条校验单测 + 更正 3 行漂移；`wiring_report.py --verify` 绿。
   - 验证：`cargo test -p apeireth-runtime-assembly --lib`；`cargo clippy -- -D warnings`；`cargo fmt --check`；
     `python scripts/arch_guard.py --report` VIOLATIONS 0 / STALE 0。
3. **R1b-2 机制订阅迁移 + lifecycle probe**（生物线主笔，交接条件：其当前 WIP 已提交且全量绿）
   - bio_tone / bio_lifecycle / care_heartbeat / bio_circadian / play / quorum 改订阅；审批面改 publish；
     lifecycle probe 注册；台账 wiring 字段更新为 bus 路径。
   - 验证：每模块默认关零变化测试；`bio_*` 集成测试；全量门禁由全量线执行并广播。
4. **R1b-3 缺失信号源**（生物线）
   - ritual / sleep pressure / scar learn / session event source；补不了的保留 NotWired + reason。
   - 验证：每个信号源有生产 publish 点 + 订阅测试；台账逐行如实。
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

- [ ] R1b-1：`NervousSignal`/`NervousBus`/`NervousSwitchboard` 落地；总线单测（订阅顺序、重复订阅、
      无订阅者 publish 为空操作、计数、订阅者不可阻塞的文档约束）；`arch_guard --report` VIOLATIONS 0 /
      STALE 0；基线净额 ≤ 0（新增文件条目是唯一增量，且只在收口登记）
- [ ] R1b-1：开关读取点收敛（装配根 1 处；模块内 bio env `std::env::var` = 0）；现有默认关测试不变
- [ ] R1b-1：台账 3 行漂移更正 + 3 条校验单测（env-registry / wiring 调用点 / NotWired reason）
- [ ] R1b-2：机制订阅迁移；lifecycle probe 注册；审批面 publish；默认关零变化
- [ ] R1b-3：缺失信号源如实补齐或如实 NotWired；台账 14/14 逐行有据
- [ ] R1b-4：schema v2 additive；自述渲染；contract wave_note / handbook / integration-plan 同步；
      `docs_truth.py --verify --check` exit 0；`wiring_report.py --verify` exit 0
- [ ] LOCKED 5 项 0 触碰 / 0 新外部 dep / 0 新开关 / 0 新 crate

## 7. 后果（Consequences）

- 正面：机制新增从「重写钩子适配」变成「订阅信号」；开关单点审计/回滚；自述台账不再说假话；
  适配器不再持有机制状态句柄（依赖收窄）；为 R8（治理单一管线）提供统一信号面。
- 负面/接受的代价：同步订阅者若自身慢会拖慢回合（文档 + 评审约束，后续如需要可再 ADR 讨论异步面）；
  总线首版不持久化（重启丢计数，自述 `last_observation` 仍 null）；R1b-2 交接存在排期依赖（生物线
  WIP 收口是前置），交接前 R1b 只到 R1b-1b。
- 后续 stage：Wave 4 其余 = R8（治理单一管线）+ R6（验证架构）+ R7-2 遗留（ContextAssembler 合并，
  已登记 `contract.toml [concepts]` known_debt）；R1b-4 之后再评估是否把总线升为扩展点（需新 ADR）。
