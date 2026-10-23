# 册 01 — 角色与成就交付记录（24 篇合册）

> **一句话定位**：R14/R19 施工期的**按角色归档成就报告**——每个代理角色（架构 / 后端 / 前端全栈 / QA / 代码审查 / 安全审查 / DevOps / Leader）把自己那一格的交付物、数字与验收判据写成交接件。
>
> **来源篇数**：24（分配表 §01，逐篇唯一归册）｜**原篇日期跨度**：2026-08-01 ~ 2026-08-04（2 篇无日期）
> **合册生成日**：2026-10-23｜**复核口径（现行真源）**：[test-baseline](../docs/03-reference/test-baseline.md) §1 ／ [claims.yaml](../docs/status/claims.yaml) ／ [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) ／ [capabilities-matrix §0.2](../docs/03-reference/capabilities-matrix.md)

## 目录

| # | 原标题 | 原日期 | # | 原标题 | 原日期 |
|---|---|---|---|---|---|
| 1 | A10-deep cognition 范围覆盖确认 | 08-03 | 13 | A1 Leader 总结 | 08-01 |
| 2 | P22 architect2 council 补交证据 | 未标 | 14 | A1.2 QA 集成测试报告 | 08-01 |
| 3 | P22 architect3 sovereignty 补交证据 | 未标 | 15 | A1.5 DevOps commit 落地 | 08-01 |
| 4 | P27 抽象 trait 工程化验收 | 未标 | 16 | A10 cognition 最小落地 | 08-01 |
| 5 | P29 恢复 pybridge | 未标 | 17 | A11.1 action 器官 | 08-01 |
| 6 | P30 sovereignty 漂移报告 | 08-02 | 18 | A11.3 value 器官 | 08-01 |
| 7 | P31 7 处 pre-existing 破损修复 | 08-02 | 19 | A16 mcp_integration_expert | 08-01 |
| 8 | P32 council 测试补齐 | 未标 | 20 | A16.3 pybridge 完整落地 | 08-01 |
| 9 | A1-W3 设置页持久化 | 08-04 | 21 | A2-W3 tui-session episode | 08-04 |
| 10 | A1.4 ADR：CLI 接 core Session API | 08-01 | 22 | A20 后端 17 crate 集成最终报告 | 08-01 |
| 11 | A1.1 apeireth-cli session 接 core | 08-01 | 23 | P12 constraint（12 键 + 5 重守门） | 08-01 |
| 12 | A1 代码审查报告 | 08-01 | 24 | P16 onion 双洋葱 trait 抽象层 | 08-01 |

---

### A10-deep cognition 范围覆盖确认（database_engineer） (2026-08-03)

**来源**：`reports/A10-deep-cognition-database-engineer.md`（50 行）｜角色 database_engineer｜结论：范围已覆盖，Leader 指示直接 mark complete，本篇只做证据对齐。
**结论/要点**
- 覆盖矩阵 8 项：6 状态机 🟡 部分（`cognition/src/lib.rs`+`reflection.rs`）；5 Module 🟡 4/5（PerceptionAdapter 由 `apeireth-asi` 提供）；7 席 council 🟡 守门层已位、语义映射未实装；失败路径收敛 ✅；测试 **29/29 全绿但未达 ≥60 阈值（漂移诚实登记）**；demo 仅 3 场景（Normal→Allow / ModifyL0HA→Reject / Mixed→Reject）；LOCKED 0 触碰 ✅；7 项不修改承诺 ✅。
- 漂移登记 3 条：角色专长错位、未擅自补全 60 测试与 6 状态演示、承诺守住。**验收判据**：报告 + `team_complete_task` + `team_report_idle` 三件套。
**现状复核（2026-10-23）**：**已过时**——`crates/apeireth-cognition/` 已迁 [legacy/donor/apeireth-cognition](../legacy/donor/apeireth-cognition)（[legacy/README.md](../legacy/README.md)：不作新开发、不注册能力）；29/29 为 crate 级旧口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）。**未核实**：6 状态机是否在别处补完——现行 `crates/` 树未检索到等价实现，不作断言。**部分成立**：「范围已被覆盖」的登记行为与现行宪章一致。
**归属链接**：../Cargo.toml、../legacy/README.md、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 `crates/apeireth-cognition/**`；同册相关 round15-01-asi-ml-calibration-acceptance.md、round15-02-bus-5-layer-acceptance.md（册 04）

### P22 architect2 — apeireth-council 补交证据 (原篇未标日期)

**来源**：`reports/P22-architect2-council-evidence.md`（311 行）｜architect2｜评审「加权 **7.05** 通过，证据材料补交」。
**结论/要点**
- 范围 `crates/apeireth-council/`（lib + 7 advisors + example + 5 顶层常量 + SovereigntyHook trait）；**本 crate 不在 `[workspace] members`（root Cargo.toml 缺一行）**，故 `cargo test -p apeireth-council` 找不到，须 `cd` 进目录跑。
- 证据逐条：7 强制 Advisor（`AdvisorDomain::ALL` 稳定序、`seven_mandatory_advisors()` 工厂、7 advisor 实现、`Advisor` trait 331 行、编译期断言）；HoldTrigger 3 触发条件（3 阈值常量 + `HoldThreshold` + `evaluate` 判定流）；Synthesis 加权公式（`SynthesisWeights` + 5 步算法）；Persona session（7 实例化 + 3 轮辩论）；`SovereigntyHook` 契约 + `CouncilEvent` 5 变体 + `NoopSovereigntyHook`；§6 自带「已知缺口（透明声明）」+ §7 汇总表。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-council](../legacy/donor/apeireth-council)，7 席 council 不再是现行能力面；现行 Council 为**默认 OFF**（`APEIRETH_COGNITIVE_COUNCIL=1`，见 [capabilities-matrix §0.2](../docs/03-reference/capabilities-matrix.md)），即「默认关的能力不是系统现状」。**已过时**：「加权 7.05」评审分制在现行仓内无对应登记面。**仍成立**：「成员漏登记导致 `cargo test -p` 找不到」这一漂移，现行对策是 [scripts/arch_guard.py](../scripts/arch_guard.py) `--strict` + `docs_truth.py --verify`。
**归属链接**：../legacy/donor/apeireth-council、../legacy/README.md、../scripts/arch_guard.py、../docs/03-reference/capabilities-matrix.md、../docs/status/claims.yaml；历史 `crates/apeireth-council/src/{advisor.rs,advisors/*,hold.rs,synthesis.rs,persona.rs,sovereignty.rs}`

### P22 architect3 — apeireth-sovereignty 补交证据 (原篇未标日期)

**来源**：`reports/P22-architect3-sovereignty-evidence.md`（约 450 行）｜architect3｜评审「加权 7.05 通过，证据材料补交」。
**结论/要点**
- 范围 `crates/apeireth-sovereignty/`（lib + **19 子模块** + example + 9 阶段状态机 + MEWG 5 重治理）；**测试 61 lib + 9 doc + 54 integration 全通过**；规模 **19 模块 + 5689 行 + example 85 行**。
- 主权 trait 3 入口 `decide` / `pause` / `suspend_self`；7 配套 trait（`BiometricProvider`/`MewgAuthority`/`AiProvider`/`HumanVoter`/`PhysicalMultisig`/`ReflectionClock`/`AuditWindowHistory`/`DomainGate`）；证据 8 项全 ✓：HA single/multi demo、三域分离强制（3 结果 + 5 哲学键 + 6 权限层）、SGI trace（7 默认规则）、migration（6 载体）、9 阶段状态机 ASCII 图、`SovereigntyHook` trait 锁死。
**现状复核（2026-10-23）**：**已过时（双重）**——① crate 现位于 [legacy/donor/apeireth-sovereignty](../legacy/donor/apeireth-sovereignty)；② 治理面已换代：现行治理链 = R8 六段骨架 + R8-B2 生产单入口（见 [claims.yaml](../docs/status/claims.yaml) `gov.chain_skeleton` PRODUCTION WIRED / `gov.chain_hook_adapter` / `gov.reflection_queue` IMPLEMENTED / `gov.chain_drill_ci` CI 步骤被 skip / `sec.governance_chain_drill`），原「主权引擎 + MEWG 5 重治理」自研栈不再是现行治理实现。**仍成立**：「已实现未接线 ≠ 可宣称」的四级口径纪律（[system-capabilities.md](../docs/01-architecture/system-capabilities.md) 头部）。**未核实**：19 模块 5689 行与 61+9+54 在 `legacy/donor` 快照中是否原样保留。
**归属链接**：../legacy/donor/apeireth-sovereignty、../docs/status/claims.yaml、../docs/01-architecture/system-capabilities.md、../docs/adr/0010-r8-governance-pipeline.md；历史 `crates/apeireth-sovereignty/src/*`

### P27 阶段 4 抽象 trait 工程化验收 (原篇未标日期)

**来源**：`reports/P27-six-abstract-traits-acceptance.md`（76 行）｜**结论：PASS**。
**结论/要点**
- **口径纠偏**：任务标题称「6 个抽象 trait」，正文实列 **9 个**；本篇按更严口径把 **9 个全部实现**：`Cognition`/`Intuition`/`Reasoning`/`MetaCognition`/`Recall`/`Consolidation`/`Forgetting`/`Learning`/`Abstraction`。
- `BasicCognitiveEngine`（无状态确定性示例实现）分别实现全部 9 个 trait，**不是文档 sketch**；边界：空输入→`None`/`false`、置信度与学习强度限 `[0.0,1.0]`、NaN/∞ 归零、UTF-8 共同前缀按字符边界。
- 集成测试 `crates/apeireth-cognition/tests/six_abstract_traits_acceptance.rs` **18 个**（9 traits × 2，超任务要求 ≥12），覆盖正常/空输入/拒绝/范围限制/NaN 清洗/策略全删/顺序保持/ASCII 与中文 UTF-8。**验收判据**：build 0 error；unit **29 passed**；既有 integration **5 passed**；P27 trait integration **18 passed**；doc 0 failed；**合计 52 passed / 0 failed**；原生 Rust，未引入 PyO3 或外部 SDK。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-cognition](../legacy/donor/apeireth-cognition)；「52 passed」为 crate 级旧口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（[test-baseline §1](../docs/03-reference/test-baseline.md)）。**部分成立**：「标题与正文口径冲突按更严口径执行」「不引入外部 SDK」与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 口径纪律及 [AGENTS.md](../AGENTS.md) §5「0 引新外部 dep」一致。**未核实**：9 个 trait 是否在现行树（[crates/engine/organ](../crates/engine/organ) 等）有等价实现——不作断言。
**归属链接**：../legacy/donor/apeireth-cognition、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；历史 `crates/apeireth-cognition/src/lib.rs`、`crates/apeireth-cognition/tests/six_abstract_traits_acceptance.rs`

### P29 恢复 apeireth-pybridge（按 stage5 §2「保留扩展」） (原篇未标日期)

**来源**：`reports/P29-pybridge-restoration.md`（96 行）｜architect｜状态 ✅ 完成（rebase 到 integration HEAD `2427a68c`）。
**结论/要点**
- **调研结论（修正任务前提）**：pybridge **从未被物理删除**（`git log --diff-filter=D` 无命中）；完整实现在 commit `2cbbd85c`（A16.3，35 unit tests）；integration HEAD 时 5 文件 844 行已就位，**只是 workspace.members 缺**；根因 = 漂移修复 + `c65f4f6f` 等 commit 链路中 members 被遗漏。
- 本次提交：pybridge 加入 members；`pyo3 = "0.22"` 进 workspace.dependencies（**feature-gated，不进核心依赖**）；建 `crates/apeireth-verify`（stub）；注释掉 core 中引用不存在宏 `__APEIRETH_REG_APEIRETH_CORE_A/B` 的代码段。
- **验收判据**：`cargo build -p apeireth-pybridge --offline` 0 error（1.31s）；`cargo test` → **35 unit + 10 integration（`tests/pybridge_q29.rs`）+ 0 doc = 45/45 passed**。
- Feature-gating ADR：PyO3 仅 `python-ext` 可选依赖，不进 core/memory；理由「不假装 + 解耦 + 可选扩展」。**Owner 决策位 / 后续 4 项**：④ perception `SignalSource::PyBridge`、⑤ central `COMPONENT_COUNT`、⑥ verify stub 升级、⑦ V27 `cargo test --workspace` 全量；Reviewer 必查 7 项中 ④⑤ 标注「⚠️ 其他角色负责」。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-pybridge](../legacy/donor/apeireth-pybridge)，现行 [Cargo.toml](../Cargo.toml) members 无 pybridge、无 PyO3 依赖面；[deny.toml](../deny.toml) `multiple-versions = "deny"` + 「0 引新外部 dep」使 PyO3 重新进主依赖图成本远高于 2026-08 语境。**已过时**：「pybridge 在 25 members 中」→ 现行 **30 members（29 crates + bench harness）**（[AGENTS.md](../AGENTS.md) §3）。**仍成立**：可选依赖 feature-gating 隔离 + 「不假装核心支持 Python 互操作」。**未核实**：work item ④⑤ 的最终归宿（[legacy/donor/apeireth-perception](../legacy/donor/apeireth-perception) / [legacy/donor/apeireth-central](../legacy/donor/apeireth-central) 中相应符号是否仍存在）。
**归属链接**：../legacy/donor/apeireth-pybridge、../legacy/donor/apeireth-perception、../legacy/donor/apeireth-central、../legacy/donor/apeireth-verify、../Cargo.toml、../deny.toml、../AGENTS.md；历史 `crates/apeireth-pybridge/src/*`、`crates/apeireth-pybridge/tests/pybridge_q29.rs`

### P30 — Sovereignty 漂移报告（stage5 §2 17 → 18 crate 修订） (2026-08-02)

**来源**：`reports/P30-sovereignty-drift-stage5-crate-count-report.md`（约 200 行）｜backend_engineer2｜分支 `rebase/d7d8-into-integration`，HEAD `2e20deb1`。
**结论/要点**
- 目标：登记 stage5 文档与实际 workspace 的 **crate 数漂移（17 ↔ 18 ↔ 24）** + sovereignty 在 stage5 LOCKED 后新增引发的漂移。
- **漂移全景矩阵 10 行**：17 在 §2 标题（L5/L68）+ §2 v6 修正（L70）—— ❌ 与 body 不一致；18 在 §2.1 body（L86）/ §2.1 对比（L100）/ §2.2（L138）/ §7（L442）/ §12 S-1（L618）/ §12 验收矩阵（L642）/ §13（L648）—— ⚠️ 内部一致；**24 = Cargo.toml 实际 members** —— ✅ 真实工程状态；**0 = stage5 中 sovereignty 提及数** —— ❌ P22 后未登记。
- 根因：commit `caa4702a`（施工文档 LOCKED）改标题 9→17 未同步 body；`47710b35` 为 commit message drift 范例。**11 处不一致全列 + sovereignty 7 处未登记位点 + 7 步修复方案 owner 明确；不修改 LOCKED stage5 文档；不假装「已修复」**（仅登记 + 标注归属 + 建议路径，实际修改需 leader 解锁）。
**现状复核（2026-10-23）**：**已过时**——17/18/24 三值均为 2026-08 口径；现行唯一口径 = [Cargo.toml](../Cargo.toml) members **30 项（29 crates + bench harness）**，历史链（20 → 31 → 30）见 [AGENTS.md](../AGENTS.md) §3；原引 stage5 文档已不在现行 `docs/` 面。**仍成立**：「漂移只登记不假装已修 + 不擅改 LOCKED 文档 + 给 owner 明确修复路径」与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) 登记债面一致。**未核实**：stage5 文档与 sovereignty 7 处未登记位点在当前 `docs/` 树中是否仍可定位——不作断言。
**归属链接**：../Cargo.toml、../AGENTS.md、../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md、../docs/04-internal/backend-enhancement/00-backlog.md、../legacy/donor/apeireth-sovereignty；历史 stage5 LOCKED 施工文档（已不在现行 docs/ 面）

### P31 — 7 处 Pre-Existing 破损修复报告 (2026-08-02)

**来源**：`reports/P31-pre-existing-breakages-fix-report.md`（约 210 行）｜backend_engineer2｜HEAD `2427a68c`。
**结论/要点**
- 7 处破损来源：V25 cargo workspace 验证报告（commit `53583326` / HEAD `8b59e12d`）。
- **状态总览**：①`apeireth-verify/src/lib.rs` 物理删除 → ✅ 关闭；②`apeireth-pybridge/` 整目录删除 → ⚠️ 不适用（workspace 外）；③`apeireth-upgrade/tests/upgrade_q15.rs` 删除 → ⚠️ 部分关闭（crate 编 0 error，文件仍缺）；④`apeireth-perception` `SignalSource::PyBridge` 编译错 → ✅ 关闭；⑤`apeireth-central` `COMPONENT_COUNT` 17 vs 16 → ✅ 关闭（现 17，与 stage5 一致）；⑥`apeireth-cli` 依赖 → ✅ 关闭；⑦`apeireth-test/tests/` 整体缺 → ⚠️ 部分关闭。
- **汇总**：`cargo build --workspace` **24/24 0 error** + `cargo test --workspace --tests` **834 passed / 0 failed / 0 ignored**（维持 V26.1 状态）。**诚实披露**：1 项真破损残留——`apeireth-verify/examples/walk_all_crates.rs` **5 errors**（P28 阶段 6 proc-macro 未完成，归属 fullstack_engineer）。**Owner 决策位**：等待 Leader 评审 + 是否追加破损 #3/#7 测试文件 + pybridge workspace 决策。
**现状复核（2026-10-23）**：**已过时（全部 7 项对象已不在现行树）**——涉及 crate 现分别在 [legacy/donor/apeireth-verify](../legacy/donor/apeireth-verify)、[apeireth-pybridge](../legacy/donor/apeireth-pybridge)、[apeireth-upgrade](../legacy/donor/apeireth-upgrade)、[apeireth-perception](../legacy/donor/apeireth-perception)、[apeireth-central](../legacy/donor/apeireth-central)、[apeireth-test](../legacy/donor/apeireth-test)、[crates/adapters/cli](../crates/adapters/cli)；「24/24 0 error、834 passed」为旧口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored。**仍成立**：把 pre-existing 破损逐条登记成可勾选状态表、明确「部分关闭」而非含糊写「已修复」——与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 债栏（D29/D31 状态注记写法）同构。**未核实**：7 项破损在当时是否真消除——本轮未逐项复核 `legacy/donor` 快照。
**归属链接**：../legacy/donor/apeireth-verify、../legacy/donor/apeireth-pybridge、../legacy/donor/apeireth-upgrade、../legacy/donor/apeireth-perception、../legacy/donor/apeireth-central、../legacy/donor/apeireth-test、../crates/adapters/cli、../docs/03-reference/test-baseline.md；历史 `crates/apeireth-upgrade/tests/upgrade_q15.rs`、`crates/apeireth-test/tests/`

### P32 apeireth-council — 测试实装补齐 (原篇未标日期)

**来源**：`reports/P32-council-tests-expansion.md`（约 170 行）｜round8-03 architect1。
**结论/要点**
- 触发：P22 architect2 评审「加权 7.05 通过，但 inline unit test 不足」，本轮补齐。
- **数量对照**：inline unit tests（advisors）**0 → 35**（5/advisor × 7）；integration（`tests/council_tests.rs`）**24 → 29**（+5）；全 `cargo test -p apeireth-council` **24 → 64 passed**（35 inline + 29 integration）；工作区无回归。
- 每 advisor 5 测试覆盖：Domain 验证 / ID 稳定性（v1 命名不可随意改）/ 关键词强反对→`StrongDisapprove` / 正常 query→`Approve` / **中文 keyword 反对**（P22 reviewer 关切的国际化场景）。
- **Ponytail 备忘**：9 处 `unused import`（`AdvisorOpinion`/`init_context`）**故意保留**，避免 commit diff 噪声盖过真实改动，下轮可一并 `cargo fix`；`verdict.report.weighted_score` 改断言逻辑记入 §3.5。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-council](../legacy/donor/apeireth-council)、「64 passed」为其历史口径（现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored）。**仍成立**：「Ponytail 明示（故意保留的警告 / 待办项）」与 [AGENTS.md](../AGENTS.md) §1 诚实纪律一致；「每 advisor 必测中文关键词」是可复用的国际化用例设计要求。**未核实**：9 处 `unused import` 是否仍在 `legacy/donor/apeireth-council` 快照中。
**归属链接**：../legacy/donor/apeireth-council、../docs/03-reference/test-baseline.md、../AGENTS.md；历史 `crates/apeireth-council/src/advisors/*.rs`、`crates/apeireth-council/tests/council_tests.rs`；同册相关 P22-architect2-council-evidence.md（本册 §2）

### A1-W3 设置页持久化 (2026-08-04)

**来源**：`reports/achievement-A1-W3-tui-settings-persist.md`（约 130 行）｜R19-TUI W3 #4｜状态 🟢 完成（**19/19 tests pass**）。
**结论/要点**
- DoD 8 项全 ✅：theme/mode/language/splash/breath 改后重启保留（`persistence::load()` + `App::with_loaded_settings()` 启动还原）；Win 走 `APPDATA\apeireth\`（比原计划 `bin/` 更标准鲁棒）；跨平台 APPDATA / XDG 双 `#[cfg]`；不存在 → 默认值（`load_from` / `Settings::defaults()`）；解析失败 → 默认值（任何反序列化错误 fall back）；退出时 save（`main()` 中 `run_app` 后兜底）；改设置时即时 save（`handle_settings_key` 检测 `before != after`）；**unit tests 7/7 pass**。
- 漂移检查守 8 项不假装 + O-2 不漂移；**关键发现**：原计划新建 `settings_io.rs` 走 enum serde + `~/bin/`，改为**复用既有完整 `persistence.rs`**（不重造）。主哲学 6 锚逐条穿透（S-1/S-2/O-5/O-2/O-3/O-4）。
**现状复核（2026-10-23）**：**已过时**——`apeireth-tui` 现位于 [legacy/donor/apeireth-tui](../legacy/donor/apeireth-tui)；现行桌面面是 [frontend/companion-desktop](../frontend/companion-desktop)，其设置持久化另有实现（册 10「设置实时应用合同」）；「19/19」为该 crate 历史口径。**仍成立**：「原计划新建模块 → 复用既有 persistence.rs」的 O-2 不漂移判定，与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)「叠加不覆盖」纪律一致。**未核实**：R19-TUI 在 [crates/adapters/tui](../crates/adapters/tui) 是否有继任实现——本轮未逐文件核对。
**归属链接**：../crates/adapters/tui、../frontend/companion-desktop、../legacy/donor/apeireth-tui、../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md；历史 `crates/apeireth-tui/src/{persistence.rs,main.rs}`；同册相关 achievement-A2-W3-tui-session-episode.md（本册 §21）

### A1.4 ADR 成就报告 — CLI 接入 core Session API (2026-08-01)

**来源**：`reports/achievement-A1-architect-adr.md`（约 200 行）｜architect｜任务 `c41136b8-4680-47c3-bb7b-06b0d28cf1a4`。
**结论/要点**
- **DoD 8/8 通过**；ADR 落盘 `docs/adr/0002-cli-session-api-binding.md`，状态 🟢 Accepted（2026-08-01）。
- **决策一句话**：`apeireth-cli/src/lib.rs` 是 CLI 的**唯一公开 API 表面**；`main.rs` 只经 `apeireth_cli::*` 调用，**绝不直接 `use apeireth_core::*`**。
- 备选 4 项逐一否决并给理由：A 直接调 core（耦合）/ B 不暴露 lib / C 抽 `apeireth-binding` crate（过度工程）/ D 暴露高层命令字符串（抽象太粗）。
- **不修改承诺核查**：7 项 LOCKED × 三列（是否触动/证据）= **7/7 0 触动**。**Self-Disable 边界**：❌ 禁止 API 7 项（`is_l0_ha_required`/`disable_self`/`reorganize_onion`/`modify_l0_layer`/`bypass_guard`/`exit_silent_mode`/`fake_human_approval`）+ ✅ 允许 API 4 项 + 边界守护机制 3 条。
- 升级触发：阶段 7+ 前端复用 ≥2 个 → 抽 binding crate；core 暴露 `Action` enum → cli 用 type alias 桥接。**签字位**：架构师起草 / backend_engineer 落地 / code_reviewer 审查 / 守护责任在主人。
**现状复核（2026-10-23）**：**已过时（ADR 编号与文件名已变）**——原 ADR 文件名在现行树**已不存在**，现行 [../docs/adr/0002-target-architecture-v1.md](../docs/adr/0002-target-architecture-v1.md)；ADR 目录现存 0001–0014，编号已整体重排（历史不追溯改写）。**部分成立**：「CLI 只经 lib 公开面、不直接引用 core 内部类型」在现行树仍可核（[crates/adapters/cli](../crates/adapters/cli) 与 [crates/foundation/core](../crates/foundation/core) 分离存在），但「main.rs 不直接 use core」是否逐字仍生效——**未核实**（未做全量引用检索）。**仍成立**：Self-Disable 判定仍是现行 **3 不可变脊柱**之一（[ROADMAP.md](../ROADMAP.md) §13），原 7 项禁止 API 是其早期形态清单。
**归属链接**：../docs/adr、../docs/adr/0002-target-architecture-v1.md、../crates/adapters/cli、../crates/foundation/core、../ROADMAP.md、../docs/01-architecture/system-capabilities.md；历史 `docs/adr/0002-cli-session-api-binding.md`、`crates/apeireth-cli/src/*`

### A1.1 成就报告 — apeireth-cli session 接 core Session API (2026-08-01)

**来源**：`reports/achievement-A1-backend-engineer-apeireth-cli-session.md`（约 220 行）｜backend_engineer。
**结论/要点**
- DoD 8 项全 ✅；`session` 子命令从 `println!` 占位升级为真正调用 core Session API：`run_session()` 调 `create_default_session()` / `build_default_human_authority()` / `build_default_permission_onion()` + `welcome_message()`；stdin 循环：读行 → `:quit`/EOF 退出 → `handle_input_line(line,&session)` → `run_session_action(&Action)` → `describe_verdict`。
- `lib.rs` 新增 **9 个 pub fn**：`create_default_session`/`build_default_permission_onion`/`build_default_human_authority`/`welcome_message`/`classify_risk`/`build_action_from_input`/`describe_verdict`/`handle_input_line`/`run_session_action`。
- **测试口径**：**12 lib + 6 集成 = 18 条**（lib 逐条：placeholder_backcompat / session_has_valid_fields / session_ids_unique / ha_single_mode / onion_six_layers / classify_risk_levels / handle_input_normal_allows / handle_input_l0_blocked_by_principle / handle_input_line_e2e / welcome_message_uses_session_fields_dynamic / describe_verdict_strings / action_ids_increment；集成 T1–T6：session 字段 / normal Allow / L0→PHL-04 / ReorganizeOnion→PHL-02b / ModifyEvolutionL0→PHL-06 / e2e）。
- **验收判据**：`cargo test --workspace` → **33 passed / 0 failed**；`cargo clippy -p apeireth-cli --all-targets` → **0 warning**（其它 crate 121 warnings 预存于 core/philosophy，按 7 项承诺不在 DoD 范围）；`cargo run -p apeireth-cli -- session` 实测 `Session ID: sess-1785583953-1`、`started_at: 2026-08-01T11:32:33+00:00`（动态生成）；改动仅 `crates/apeireth-cli/src/{main.rs,lib.rs}` + 本报告。
- **Ponytail 取舍 5 条**：Session ID 用 `timestamp + AtomicU64 counter` 而非 uuid（升级：跨进程分布式时 uuid v7）；`classify_risk` 关键词启发式（升级：接真分类器）；stdin 用 stdlib `BufRead::read_line`（升级：阶段 7 换真 TUI 客户端）；保留 `placeholder()` + 兼容测试；不重写其余 dispatch 分支。
**现状复核（2026-10-23）**：**已过时**——33 passed 为 2026-08 workspace 级旧数字，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）；CLI 现为 [crates/adapters/cli](../crates/adapters/cli)、core 现为 [crates/foundation/core](../crates/foundation/core)。**部分成立**：集成测试中的 `PHL-02b`/`PHL-04`/`PHL-06` 是 v4.1 时代 **12 键**编号，现行键集 = **14 键 PHL-01..08**（[ADR-0006](../docs/adr/0006-formalize-14-keys.md)，2026-10-20 正式化），12 键为历史口径（[ROADMAP.md](../ROADMAP.md) §13）。**仍成立**：「本 crate clippy 0 warning、其它 crate 预存 warning 如实标注且不在本 DoD 范围」与 [AGENTS.md](../AGENTS.md) §5 同向。**未核实**：原 9 个 pub fn 在现行 CLI 中的存续情况——未逐符号核对。
**归属链接**：../crates/adapters/cli、../crates/foundation/core、../docs/03-reference/test-baseline.md、../docs/adr/0006-formalize-14-keys.md、../ROADMAP.md、../AGENTS.md；历史 `crates/apeireth-cli/src/*`、`crates/apeireth-core/src/lib.rs`；同册相关 achievement-A1-architect-adr.md（本册 §10）

### A1 代码审查报告 (2026-08-01)

**来源**：`reports/achievement-A1-code-reviewer-review.md`（约 300 行）｜code_reviewer｜任务 `0b019661-5248-4990-a798-8f29ed1fde6f`。
**结论/要点**
- 审查对象：`crates/apeireth-cli/src/main.rs`（100 行）+ `lib.rs`（34 行），基于 **commit 尚未落地**的 working tree。
- **7 维评分**：① 正确性 ⚠️ 部分（Session **未真构造**、ActionGuard **未调用**）；② 测试覆盖 ❌ 严重不足（**仅 1 个 test，覆盖率约 12.5%（1/8 路径）**）；③ 错误处理 ⚠️ 部分（解析错误已处理，stdin EOF 与 clippy 边界未触发）；④ 不修改承诺 7 项 ✅ 100% 守住；⑤ Self-Disable 边界 ✅ 守住（无违规 API）；⑥ **V1+V2+V3 AND 门真接入 ❌ 未接入**（仅 `println!` 标签，无 `ActionGuard::check_action` 调用）；⑦ commit 规范 ⚠️ 待 commit。
- **Overall Risk Level: 🟡 MEDIUM；Verdict：完成度约 40%** —— 协议层与参数解析层完整，Session API 真接入与守门真接入两块核心均未实现，仅以 placeholder 字符串冒充。建议 **commit 前先补 P0 + P1**（构造 Session、接 ActionGuard、补测试），否则 risk level 维持 MEDIUM。边界清单：不触碰 7 项承诺；不加「询问是否需要 L0 HA」类元问题 API；不新增依赖；不实现 V3 Multi-AI consensus（属 A5）。
**现状复核（2026-10-23）**：**已过时（被同批后续交付取代）**——本篇是 A1.1 落地**之前**的快照；同批 [achievement-A1-backend-engineer-apeireth-cli-session.md](achievement-A1-backend-engineer-apeireth-cli-session.md) 随后给出 18 条测试全绿 + 实测 stdin 对话循环（本册 §11），故「40% 完成度」是**历史时点结论，不是现状**。**仍成立**：审查与实现分角色并行、按维度打分并给 Risk Level、明确列「不建议做的事」边界——与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) 审查纪律一致。**未核实**：A1 之后 CLI 是否再出现「占位冒充已接入」——未做等价检索。
**归属链接**：../crates/adapters/cli、../crates/foundation/core、../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md、../docs/03-reference/test-baseline.md；历史 `crates/apeireth-cli/src/*`；同册相关 achievement-A1-backend-engineer-apeireth-cli-session.md（本册 §11）、achievement-A1-leader-summary.md（本册 §13）

### A1 成就达成总结 — 施工团队 Leader (2026-08-01)

**来源**：`reports/achievement-A1-leader-summary.md`（约 180 行）｜leader。
**结论/要点**
- A1 拆 4 并行子任务：A1.1 后端主实现 / A1.2 集成测试 / A1.3 代码审查 / A1.4 ADR。
- 成绩单：**18 条测试全绿（要求 5+，超额 3.6×）**、clippy 0 warning、**实测 stdin 对话循环通过 L0 → V1 拦截关键路径**、7 项承诺 100% 守住；**4 子任务平均分 8.875/10**；workspace `cargo test --workspace` **33 passed / 0 failed**。
- 漂移检查清单：A5 完成时**强制**漂移检查；A1 完成不强制（已隐含在 A1.4 的 7 项核查 + A1.3 的 7 维审查中）。**Owner 决策位**：下一轮待主人决策——进入 A2–A5 推进，或暂停收尾。
**现状复核（2026-10-23）**：**已过时**——33 passed / 8.875 分为历史时点值（现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored，2026-10-21）；现行 crate 数 = **30 members**（[AGENTS.md](../AGENTS.md) §3）；原「apeireth-cli → apeireth-core」crate 对已被树重构取代（[crates/adapters/cli](../crates/adapters/cli) → [crates/foundation/core](../crates/foundation/core)）。**仍成立**：「Leader 拆解 → 多角色并行 → 汇总评分 → owner 决策位留白」结构与现行多线施工纪律同构。**未核实**：现行是否仍保留「平均分 8.875/10」这类打分面（本轮未见对应登记面）。
**归属链接**：../crates/adapters/cli、../crates/foundation/core、../docs/03-reference/test-baseline.md、../AGENTS.md、../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md；历史 `crates/apeireth-cli/**`、`crates/apeireth-core/**`；同册相关 本册 §10–§12、§14

### A1.2 集成测试报告 — QA 工程师 (2026-08-01)

**来源**：`reports/achievement-A1-qa-engineer-integration-tests.md`（约 170 行）｜qa_engineer｜任务 `531f5013-00b6-460e-8900-f0c5cb8e54c4`。
**结论/要点**
- 交付物 `crates/apeireth-cli/tests/integration_cli_session.rs`（8272 bytes），6 个 `#[test]`（T1–T6）。
- **总体状态：🟡 BLOCKED** —— `cargo test -p apeireth-cli --test integration_cli_session` 编译失败（`unresolved import: apeireth_cli::run_session_action`），连带 `cargo test --workspace` 失败；**原因不在测试本身，而在 A1.1 未完成公开 API 暴露**。测试已按 Leader 给的契约写完，A1.1 落地后应立即通过。
- DoD 9 项：#1–#6 用例编写 ✅ 100%；#7 单套件全绿 ❌ 阻塞；#8 workspace 全绿 ❌ 阻塞；#9 报告 ✅（含 6 用例 + 阻塞证据 + 复现命令）。**边界守住**：QA「不替开发修复」——测试写完，缺陷如实上报。
**现状复核（2026-10-23）**：**已过时（阻塞已于同批消解）**——同批 Leader 汇总（本册 §13）与 A1.1 报告给出 **18 条全绿 / 33 passed**，本篇 BLOCKED 是**同日内的过程态**，非残留问题。**仍成立**：QA 不代修、缺陷如实上报并给可复现命令——与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) 职责边界一致。**未核实**：现行 CLI 集成测试目录的用例命名是否沿用 `integration_cli_session`。
**归属链接**：../crates/adapters/cli、../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md、../docs/03-reference/test-baseline.md；历史 `crates/apeireth-cli/tests/integration_cli_session.rs`；同册相关 本册 §11、§13

### A1.5 成就达成总结 — DevOps 工程师 (2026-08-01)

**来源**：`reports/achievement-A1.5-devops-engineer-commit-landing.md`（约 200 行）｜devops_engineer｜Base-HEAD `cbdac28d` → New-HEAD `46a27773`。
**结论/要点**
- 任务：把 A1 期间卡在 working tree 的真实产出（lib.rs 308 行 + main.rs 138 行 + 6 集成测试 + ADR 0002 + 5 份报告）按 §Commit 规范落成 commit。
- **DoD 6 项全 ✅**：4 个 commit 全落地；4 个 subject **均 ≤72 字符（最长 69）**；`git status --porcelain` 无 A1 残留（grep `A1|apeireth-cli|adr/0002|achievement-A1` → 0 命中）；`cargo test --workspace` 仍 **33 passed / 0 failed**（保留 A1 baseline）；7 项承诺 100% 守住；报告路径正确。**整体达成度 100%（6/6）**。
- **移交建议**：A1 成就正式关闭，可继续推进 A2（集成测试已就绪，待 orchestrator 确认 A2 DoD 范围）。
**现状复核（2026-10-23）**：**已过时（HEAD / 数字 / 规范版本均换代）**——`cbdac28d`/`46a27773` 属 2026-08 提交链，现行工作树 HEAD 已在 2026-10 线（当前 `0820d309`）；33 passed 为历史口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored。**仍成立**：现行 [AGENTS.md](../AGENTS.md) §4（conventional commits、1 commit = 1 任务、正文必带 0 装真账 + O-6 三阶审查 + 影响面 + 后续 stage）与原篇判据同向；但**现行真源不要求 ≤72 字符硬上限**，以 §4 为准。**未核实**：A1 那批 commit 是否仍可从 git 历史找回——本轮未做历史检索（`git log` 只读未逐一验证）。
**归属链接**：../AGENTS.md、../docs/03-reference/test-baseline.md、../crates/adapters/cli；历史 `crates/apeireth-cli/src/*`、`docs/adr/0002-cli-session-api-binding.md`

### A10 成就: apeireth-cognition 认知器官最小落地 — 数据库工程师 (2026-08-01)

**来源**：`reports/achievement-A10-database-engineer-cognition.md`（约 260 行）｜database_engineer｜任务 `e3523aca-8e7e-423e-b4d9-5c1d036a0e02` (P2)。
**结论/要点**
- **角色不匹配诚实登记（漂移诚实 7 项之第 7 项）**：原派活角色是 `devops_engineer`；AUTO_CLAIM 下 `team_veto_auto_claim` 工具返回 "Unknown method" 无法拒绝，5 秒窗口期后被自动分配给 database_engineer；`stage3-blueprints` grep `cognition|认知|Cognition` = **0 matches**（handover 的「stage3 §3.2 LOCKED」是误称）；故采用 **minimum-viable scaffold 简化实现**，诚实标注**未深读所有 LOCKED stage3 文档**。
- **总览 DoD 8/8 ✅**：编译 0 error（4 warnings：dead-code + unused imports）；**29/29 unit tests 全绿**；examples **3 场景全成功**（Normal→Allow / ModifyL0HA→Reject / Mixed→Reject）；**16 个 pub fn**；已实装调用 `AsiV05Scores`/`V1136Submeasures`（来自 `apeireth-asi`）+ `verdict_for_target`（来自 `apeireth-core`，12 键编译时 hardcode）；members 加 `apeireth-cognition`；报告落盘。改动仅 `crates/apeireth-cognition/` 子树 + Cargo.toml 1 行。
- **建议 4 条（owner 决策位）**：① 架构师按 v4.1 §15 + APEIRETH-OMNIBUS + stage1-02 复核对齐；② 真正 devops_engineer 接力 A18（6 状态机）+ A19（OTA 7 阶段）；③ backend_engineer2 接续 A13（life-force）/ devops_engineer2 接续 A12（consciousness + relation）；④ Leader 评估 AUTO_CLAIM 失败降级方案。
**现状复核（2026-10-23）**：**已过时（crate + 键集口径双失效）**——crate 现位于 [legacy/donor/apeireth-cognition](../legacy/donor/apeireth-cognition)；原「12 键编译时 hardcode」对应 v4.1 时代，现行键集 = **14 键 PHL-01..08**（[ADR-0006](../docs/adr/0006-formalize-14-keys.md)），且键集 verdict cache **已降级为哲学标准**（`RUNTIME_ENFORCED = false`，[ROADMAP.md](../ROADMAP.md) §13「3 不可变脊柱」行）——即「12 键编译时守门」在现行树**不成立**。**已过时**：原引 `apeireth-asi` 现位于 [legacy/donor/apeireth-asi](../legacy/donor/apeireth-asi)。**仍成立**：角色不匹配 / 工具失效 / 深度未达 / 阈值漂移四项如实登记并挂 owner 建议，与现行 [AGENTS.md](../AGENTS.md) §1 和 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 债栏写法同构。**未核实**：A18/A19 最终归宿（线索见册 08「OTA 7 阶段」与册 09）。
**归属链接**：../legacy/donor/apeireth-cognition、../legacy/donor/apeireth-asi、../docs/adr/0006-formalize-14-keys.md、../ROADMAP.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；历史 `crates/apeireth-cognition/**`、`crates/apeireth-asi/**`

### A11.1 成就报告 — apeireth-action 行动器官 (2026-08-01)

**来源**：`reports/achievement-A11.1-fullstack-engineer-action.md`（约 180 行）｜fullstack_engineer｜任务 `3c479302-16b7-461a-b56a-094d25eaca0c` (P7)。
**结论/要点**
- 架构锚点：`docs/stage4/architecture-stage4-engineering-landing.md` §3.3（行动层 trait）+ §8.1（18 crate 依赖树 = 第 3 项）+ §3.13（22 trait 总览）。
- **交付 7 文件 / 1180 行**：`Cargo.toml`(27) + `lib.rs`(299，3 trait + `DefaultActionEngine` + 9 unit tests) + `execution.rs`(235) + `expression.rs`(306，6 unit tests) + `silence.rs`(89，4 unit tests) + `examples/action_demo.rs`(87) + `tests/action_tests.rs`(137，7 集成测试)。
- **DoD 7 项全 ✅**：build 0 error（4.06s）；**27 pub fn**；**13 pub struct/enum**；**26 tests（19 unit + 7 integration）**；7 集成 ≥1；member 已注册；example 可执行。3 核心 trait `ActionExecution`/`ActionExpression`/`ActionSilence`；12 键 hardcode 拒绝；沉默决策矩阵；端到端 execute→rollback 回路。
- **诚实登记**：`execute_plan` 仅在内存 tx_log 记 plan，**不真正调用 shell / fs / 网络**；真实工具注册走 `apeireth-extension` 插件协议；sandbox-validator / 5 重守门留给 `apeireth-constraint`。**协作备注（重派 1/3）**：并发 rebase 期间本成员 commit `17c0bf7b` 被 teammate 的 `c9315a78` 误删（含 crate 文件）致 tree-level 冲突，处理 = 从 integration HEAD `87b9621e` 重建工作树。
**现状复核（2026-10-23）**：**已过时（三项）**——① crate 现位于 [legacy/donor/apeireth-action](../legacy/donor/apeireth-action)，「26 tests / 27 pub fn / 7 文件 1180 行」为其历史口径；② 12 键 → 现行 14 键（[ADR-0006](../docs/adr/0006-formalize-14-keys.md)），5 重守门 → 现行 **7 重守门 v7**（[ROADMAP.md](../ROADMAP.md) §13）；③ 原引 `docs/stage4/architecture-stage4-engineering-landing.md` 已不在现行 `docs/` 面（现行架构面在 [docs/01-architecture](../docs/01-architecture)）。**仍成立**：「execute_plan 只记内存账、不假装真执行」与 [AGENTS.md](../AGENTS.md) §1「未接线写未接线」一致——本篇最有复用价值处。**未核实**：原 7 文件在 `legacy/donor/apeireth-action` 快照中是否原样保留。
**归属链接**：../legacy/donor/apeireth-action、../legacy/donor/apeireth-extension、../legacy/donor/apeireth-constraint、../docs/adr/0006-formalize-14-keys.md、../ROADMAP.md、../docs/01-architecture、../AGENTS.md；历史 `crates/apeireth-action/**`、`docs/stage4/architecture-stage4-engineering-landing.md`

### A11.3 成就报告 — apeireth-value 价值器官 (2026-08-01)

**来源**：`reports/achievement-A11.3-qa-engineer-value.md`（约 160 行）｜qa_engineer。
**结论/要点**
- 新建 crate `apeireth-value`（v0.14.0-R14）；三大 trait：`ValueEvaluation` / `ValuePrioritization` / **5 层原则洋葱一致性**。
- **61 tests 全过（46 unit + 15 integration）**，example 跑通 **6 场景**；与 A11.1(action)/A11.2(motivation)/A12(consciousness)/A13(life-force) 接口对齐。结构 8 文件：`Cargo.toml`/`README.md`/`examples/value_demo.rs`/`src/{lib.rs,evaluation.rs,prioritization.rs,onion_consistency.rs}`/`tests/value_tests.rs`。
- **已知简化（Ponytail）4 条**：`ValueCandidate.label: String` 占位（升级：Bernstein 树 / TruthValue 字典）；公式硬编码 **0.85 / 0.10 / 0.05**（升级：支持可调权重）；无持久化接口（升级：P15 upgrade crate 提供序列化路径）；待 core 派生 `Serialize` 后可一键补序列化。
- demo 6 场景含 `avg_motivation = 0.856, passing_count = 2/3`（ASI V0.5 联动）与自定义 `OnionValueMapping` 全 S 强制 Aligned。交付：8 文件 + 1 report 已 commit 至 integration（HEAD `87b9621e+1`）。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-value](../legacy/donor/apeireth-value)；「61 tests」为其历史口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored。**已过时**：`0.85/0.10/0.05` 价值门槛与 `avg_motivation = 0.856` 属该 crate 历史判定面，现行树内无对应面（**未核实**是否存在等价现行面——不作断言）。**仍成立**：每条简化都写「占位 + 升级路径」，与 [AGENTS.md](../AGENTS.md) §1 诚实真账一致；「Ponytail 清单」在现行 [docs/04-internal/backend-enhancement/](../docs/04-internal/backend-enhancement) 各卷宗仍在沿用。
**归属链接**：../legacy/donor/apeireth-value、../docs/03-reference/test-baseline.md、../AGENTS.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 `crates/apeireth-value/**`、`crates/apeireth-{motivation,consciousness,life-force}/**`

### A16 落盘报告 — mcp_integration_expert (2026-08-01)

**来源**：`reports/achievement-A16-mcp-integration-expert.md`（约 200 行）｜mcp_integration_expert。
**结论/要点**
- 交付 `apeireth-bus`（~40 pub fn，**35 unit tests 全绿**，example `bus_demo` 跑通）+ `apeireth-extension`（~30 pub fn，**23 unit tests 全绿**，example `extension_demo` 跑通）。
- **落盘位置的诚实说明**：两 crate 落在隔离目录 `.apeireth-a16-build/{bus,extension}/`，用空 `[workspace]` 表阻断 workspace 上扫——因主 worktree 存在 17 crate 物理并行冲突（其他 agent merge 多次破坏源文件 + Cargo.toml merge marker）；物理 commit 与 crates 落地由 integration agent 在 A20 阶段合并。
- **5 层通信总线**：L0 InprocBus（同进程 tokio mpsc）**完全实现**；L1 UnixSocketBus / L2 PipeBus / L3 GrpcBus / L4 WebSocketBus 均 **schema-only**（P15 物理接入）；显式声明 `BusLayer::is_fully_implemented() == true` **仅对 L0**，防误把 schema 当物理实现。OpenClaw 6 事件流完全实现。范围依据 `docs/stage2/stage2-decisions-communication-bus.md` §2/§4/§5/§6/§7 + `docs/stage1/inspiration-stage1-2026-07-30.md` §15。**剩余**：L1–L4 物理接入（P15）、真实执行 stub 替换（P13+P15）、0 mock 框架。
**现状复核（2026-10-23）**：**已过时（crate 已移出 + 现行总线面已换代）**——[legacy/donor/apeireth-bus](../legacy/donor/apeireth-bus) 与 [legacy/donor/apeireth-extension](../legacy/donor/apeireth-extension)；现行 bus 面由 R1b 事件总线与 ADR-0008/0009 接管（[../docs/adr/0009-r1b-nervous-bus.md](../docs/adr/0009-r1b-nervous-bus.md)、[claims.yaml](../docs/status/claims.yaml)），「5 层总线」分层表述在现行真源中未见对应登记面。**已过时**：「17 crate 并行冲突」语境 → 现行 **30 members**（[AGENTS.md](../AGENTS.md) §3）。**仍成立**：「schema-only 与物理实现必须显式区分」是本篇最有价值的方法论遗产，与 [AGENTS.md](../AGENTS.md) §1 四级口径、[capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0 的 `supported/available/reason` 三元表达同向。**未核实**：L1–L4 在 `legacy/donor/apeireth-bus` 快照中是否仍为 schema-only。
**归属链接**：../legacy/donor/apeireth-bus、../legacy/donor/apeireth-extension、../docs/adr/0009-r1b-nervous-bus.md、../docs/status/claims.yaml、../docs/03-reference/capabilities-matrix.md、../AGENTS.md；历史 `crates/apeireth-bus/**`、`crates/apeireth-extension/**`、`docs/stage2/stage2-decisions-communication-bus.md`

### A16.3 成就报告 — apeireth-pybridge 完整落地 (2026-08-01)

**来源**：`reports/achievement-A16.3-mcp-integration-expert2-pybridge.md`（约 170 行）｜mcp_integration_expert2｜commit `f1a8bd99`（python-ext feature）+ `2cbbd85c`（主体）。
**结论/要点**
- **5 个源文件 / 844 行**：`lib.rs`(66, 5 tests) / `error.rs`(85, 4 类 `BridgeError` + `SuggestedAction`, 3 tests) / `r11_compat.rs`(249, R11 1100+3 baseline 模块注册表, 7 tests) / `bridge.rs`(253, `Python::with_gil` 桥 + PyO3 0.22, 11 tests) / `python_bindings.rs`(191, 10 `#[pyfunction]` + `#[pymodule]`, 9 tests)。
- **公共 API 11 pub fn**（Rust 侧 7 + R11 兼容层 4）+ Python 侧 10 `#[pyfunction]`；R11 模块数 = **1103（1100 + 3 baseline）**，`r11_compat_version()` → `0.14.0-R14`。
- **已知限制 / ponytail 3 条**：① `missing_docs` warnings 6 处（`BridgeHealth`/`R11ModuleInfo` pub field 无 doc，升级 = 加 `///`）；② `python-ext` feature 只验证 rlib 路径，实际 `maturin develop` 部署需 Python 3.13 venv，**Phase 3 再做**；③ R11 模块名是「已知清单」非「实际可导入」——`is_known_r11_module` 是设计层 LOCKED 的 1100+ 元数据，`is_module_available` 才是运行时真实查询，两者配合语义正确。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-pybridge](../legacy/donor/apeireth-pybridge)，现行 [Cargo.toml](../Cargo.toml) members 无 pybridge、依赖面不含 PyO3。**已过时（R11 baseline 口径）**：「1100 + 3 = 1103」是 R11 时代口径；现行 R11 baseline 三值按 [ROADMAP.md](../ROADMAP.md) §13 与 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D16 标注为「**无公开复现入口，仅供内部对照**」——即 1103 在现行口径下**不可复核**。**仍成立**：③ 的「设计层已知清单 ≠ 运行时可导入」是极好的 0 装边界写法，与 [AGENTS.md](../AGENTS.md) §1「不许把已实现未接线说成 0 装」同向。**未核实**：`python-ext` feature 在 `legacy/donor/apeireth-pybridge` 快照中是否仍可编译。
**归属链接**：../legacy/donor/apeireth-pybridge、../Cargo.toml、../ROADMAP.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；历史 `crates/apeireth-pybridge/src/*`；同册相关 P29-pybridge-restoration.md（本册 §5）

### A2-W3 tui-session episode 写入 (2026-08-04)

**来源**：`reports/achievement-A2-W3-tui-session-episode.md`（约 150 行）｜R19-TUI W3 #2｜状态 🟢 完成（**19/19 tests pass**）。
**结论/要点**
- DoD 7 项全 ✅：每次 chat 进 `SqliteMemoryStore`（`chat()` 调 `record_chat_episodes(&store, input, &reply.text)`）；**session_id = `"tui-session"`**（`pub const TUI_SESSION_ID`）；1 user + 1 assistant episode（2 个 `Episode { session_id: TUI_SESSION_ID }`）；continuity_id 经 session_id 间接表达（沿用 `DEFAULT_CONTINUITY_ID="apeireth-tui-default"` 语义，memory episode 表 hardcode "default" 是 R14 已定）；timestamp = `now_ts()`（user）/ `ts+1`（assistant 保证顺序）；history 页 6 流中 `tui-session` 有数据（`history_stream_counts()` → `EpisodeStore::count_by_session` >0）；**unit tests 6/6 pass**。
- 漂移检查守 8 项不假装 + O-2 不漂移；主哲学 6 锚逐条穿透（S-1 / S-2 / O-5 / O-2 / O-3 / O-4）。
**现状复核（2026-10-23）**：**已过时**——`apeireth-tui` 现位于 [legacy/donor/apeireth-tui](../legacy/donor/apeireth-tui)；现行桌面会话/记忆面在 [frontend/companion-desktop](../frontend/companion-desktop) + [crates/engine/memory](../crates/engine/memory)；「19/19」为其历史口径。**部分成立**：现行记忆面仍以 `Episode` / session 维度落库（见 [crates/engine/memory](../crates/engine/memory) 与 [claims.yaml](../docs/status/claims.yaml) `mem.lesson_object` / `gap.memory_update` 的 sources），「chat 真写 episode、不假装已记忆」语义仍在；但具体常量 `TUI_SESSION_ID = "tui-session"` 与固定 session_id 写法在现行树无对应面（**未核实**）。**仍成立**：「user/assistant 双 episode + 时间戳顺序保证」「失败 eprintln 不静默吞」两条纪律可直接复用。
**归属链接**：../legacy/donor/apeireth-tui、../crates/engine/memory、../crates/adapters/tui、../frontend/companion-desktop、../docs/status/claims.yaml；历史 `crates/apeireth-tui/src/**`；同册相关 achievement-A1-W3-tui-settings-persist.md（本册 §9）

### A20 后端 17 crate 集成最终报告 (2026-08-01)

**来源**：`reports/achievement-A20-leader-integration-final.md`（约 160 行）｜Leader｜时间 2026-08-01 21:18 UTC+8｜commit `988e364e fix(cargo): 20 crate workspace build 0 error`。
**结论/要点**
- **核心 DoD 全达成**：`cargo build --workspace` 0 error（20 crate 全编译，增量 1.86s）；`cargo test --workspace` 全绿（**57 个 test result: ok 块，438+ tests pass，0 失败**）；workspace 拓扑 = 1 核心 + 9 器官 + 5 支撑 + 1 聚合根 + 1 入口 + 2 工作 + R11 既有 3 = 22（含 README）。
- **20 crate 清单分 4 组**：1 核心（`apeireth-core` = 双洋葱 + 12 键 + 5 重守门 + verdict cache + Cognitive-Dream）；9 器官（perception / cognition(26 unit tests) / action / memory / motivation / value / …）；5 支撑；聚合根 + 入口 + 工作层等。
- **完成度 20/25 crate（80%）**，缺 5 个 crate（sovereignty / bus / extension / council / evolution）。**剩余工作 4 项（owner 决策位）**：① 5 缺失 crate 落地；② 合并 `a11.1-action-rebase` 分支；③ 关闭所有 in_progress 任务；④ 写入最终 R14 启动报告。
**现状复核（2026-10-23）**：**已过时（全面换代）**——原「20 crate / 22 拓扑 / 438+ tests」对应 2026-08 树，现行口径 = **30 members（29 crates + bench harness）**（[AGENTS.md](../AGENTS.md) §3 + [Cargo.toml](../Cargo.toml)），测试真源 341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）；原「9 器官」拓扑在现行能力面上由 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0.2 的按开关分档表达取代。**已过时**：12 键 → 14 键（[ADR-0006](../docs/adr/0006-formalize-14-keys.md)）、5 重守门 → 7 重 v7（[ROADMAP.md](../ROADMAP.md) §13）。**仍成立**：完成度与缺口同时登记、不以「核心目标达成」掩盖缺口，符合 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 登记债纪律。
**归属链接**：../Cargo.toml、../AGENTS.md、../docs/03-reference/test-baseline.md、../docs/03-reference/capabilities-matrix.md、../docs/adr/0006-formalize-14-keys.md、../ROADMAP.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 `crates/apeireth-{core,perception,cognition,action,memory,motivation,value,...}/**`

### P12 成就: apeireth-constraint 约束器官（12 键 + 5 重守门） — 安全审查 (2026-08-01)

**来源**：`reports/achievement-constraint-security-reviewer-12keys-gates.md`（约 300 行）｜security_reviewer｜任务 `5916f296-dc90-4979-b0b3-fcf24a3db26c`。
**结论/要点**
- **DoD 14/14 全达成**；新建 crate `apeireth-constraint`（4 文件：`Cargo.toml`/`src/lib.rs`/`examples/constraint_demo.rs`/`tests/constraint_tests.rs`；约 430 行核心 + 270 行测试）。
- **3 trait**：`PhilosophyKeyAccess`（12 键 verdict cache——**复用** `apeireth_core::ALL_TWELVE_KEYS`，不重写）/ `FiveGates`（5 重守门：编译时 / 运行时 / 多AI / 物理隔离 / 反思期）/ `HardCodeConstraint`（编译时 const fn 断言）。**6 pub fn**：`verify_at_compile_time`/`runtime_intercept`/`multi_ai_consensus`/`physical_isolation_check`/`reflection_period_audit`/`verify_all_five_gates`。
- **14 tests（9 单元 + 5 集成）全绿**；`cargo build -p apeireth-constraint` **0 error / 0 warning**；example 可运行；members 加 1 行。**红线 6/6 守住**：不改 LOCKED 阶段 1+2+3、不碰 R11 baseline 三值、不碰 `apeireth-legacy/`。
- **Honest disclosure**：守门 5 反思期审计当前**默认 `Block`**（标「待 P19 完整接入 Cognitive-Dream 72h 监控」）；守门 3（多 AI 一致）与守门 4（物理隔离）当前是「缓存判定」简版，真实多 AI 智囊团 / L0 HA 物理多签由 `apeireth-asi`/`apeireth-perception` 提供，本 trait 仅暴露入口。**P19 衔接就绪**：P19 删 `apeireth-philosophy` 9 键 trait 时改委托到 `PhilosophyKeyAccess` 即可。
**现状复核（2026-10-23）**：**已过时（键集 / 守门 / 治理三重换代 + crate 移出）**——① 键集：12 键 → 现行 **14 键 PHL-01..08**（[ADR-0006](../docs/adr/0006-formalize-14-keys.md)），12 键为 v4.1 历史口径（[ROADMAP.md](../ROADMAP.md) §13）；且键集 verdict cache 已**降级为哲学标准**（`RUNTIME_ENFORCED = false`），不再是 runtime 强制机制。② 守门：5 重 → 现行 **7 重守门 v7**（1–5 重嵌套 + 6 Colang DSL + 7 Superpowers Skill Guard），唯一现值出处 = [ROADMAP.md](../ROADMAP.md) §13。③ crate 现位于 [legacy/donor/apeireth-constraint](../legacy/donor/apeireth-constraint)。④ 治理实现换代：现行 = R8 六段骨架 + R8-B2 生产单入口（[claims.yaml](../docs/status/claims.yaml) `gov.chain_skeleton` PRODUCTION WIRED / `gov.chain_hook_adapter` / `gov.reflection_queue` IMPLEMENTED 未启用反射调度 / `gov.chain_drill_ci` CI 步骤被 skip / `sec.governance_chain_drill`）。**仍成立**：把「默认 Block」「缓存判定简版」逐条写明并给升级路径，与 [AGENTS.md](../AGENTS.md) §1 及 [claims.yaml](../docs/status/claims.yaml) `notes` 字段写法一致。**仍成立**：12 键「复用 core 不重写」是复用而非重写的正确范式。
**归属链接**：../legacy/donor/apeireth-constraint、../docs/adr/0006-formalize-14-keys.md、../ROADMAP.md、../docs/status/claims.yaml、../docs/03-reference/capabilities-matrix.md、../AGENTS.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 `crates/apeireth-constraint/**`、`crates/apeireth-philosophy/**`

### P16 成就报告 — apeireth-onion 双洋葱统一体 trait 抽象层 (2026-08-01)

**来源**：`reports/achievement-onion-code-reviewer2-double-onion.md`（约 200 行）｜code_reviewer2｜任务 P16。
**结论/要点**
- 新 crate `crates/apeireth-onion/`（依赖 `apeireth-core` + serde + serde_json + thiserror，**0 新增依赖**）；`src/lib.rs` **684 行含 10 单元测试** + `examples/onion_demo.rs`（双洋葱 + AND 门 + 仲裁 demo）+ `tests/onion_tests.rs`（2 集成测试：e2e + offline HA）；members 加 1 行。**12 passed（10 unit + 2 integration）**；状态 ✅ 完成。
- 设计原则 §2.1：**比喻 vs 架构分离**（§19.4 双洋葱是对外心智模型，trait 层是可替换抽象）。
- **残余 / 待办**（§7.3）：trait 层已为「§19.4 候选比喻替换」预留接口，但**未实现具体替代 impl**（如「航空母舰 + 机库」「双根 + 枝叶」），待阶段 5+ 由其 owner 落地；对 `apeireth-core` 已实装类型签名**仅引用**（`ActionTarget` 等）。**签名位**：4 个新文件 + workspace 1 行 / 12 passed / 0 新增依赖。
**现状复核（2026-10-23）**：**已过时**——crate 现位于 [legacy/donor/apeireth-onion](../legacy/donor/apeireth-onion)，「12 passed / 684 行」为其历史口径；原 §19.4 比喻与「双洋葱」表述属 v4.1 时代设计面，现行可核事实是：键集 verdict cache 已降级为哲学标准、映射到 5 原则洋葱 E/S/A/M 层（`VERDICT_KEYS_BY_PRINCIPLE`，见 [ROADMAP.md](../ROADMAP.md) §13 与 [crates/foundation/core](../crates/foundation/core) 的 `philosophy.rs`）。**仍成立**：「对外比喻 ≠ 实现架构」「trait 层为未来替换预留接口且不提前实现替代 impl」与 [AGENTS.md](../AGENTS.md) §2 中性表述纪律、§1 的 0 装纪律一致。**未核实**：`legacy/donor/apeireth-onion` 快照中是否仍含 684 行实现与 12 测试。
**归属链接**：../legacy/donor/apeireth-onion、../crates/foundation/core、../ROADMAP.md、../AGENTS.md、../docs/01-architecture/system-capabilities.md；历史 `crates/apeireth-onion/**`

---

## 现状对照（2026-10-23）— 本册 24 篇横向口径

| 维度 | 原篇口径（2026-08） | 现行口径（真源） | 判定 |
|---|---|---|---|
| workspace 规模 | 17 / 18 / 20 / 22 / 24 / 25 crate | **30 members（29 crates + bench harness）**（Cargo.toml + AGENTS.md §3） | 已过时 |
| 测试数字 | 29 / 33 / 52 / 57 / 61 / 64 / 438+ passed（crate 级） | **341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）** | 已过时 |
| 键集 | 12 键（v4.1 时代） | **14 键 PHL-01..08**（ADR-0006，2026-10-20） | 已过时 |
| 守门级数 | 5 重守门 | **7 重守门 v7**（ROADMAP §13） | 已过时 |
| crate 位置 | `crates/apeireth-*` | **`legacy/donor/apeireth-*`**（77 个 donor crate） | 已过时（位置迁移） |
| 评审分制 | 加权 7.05 / 平均 8.875/10 | 无对应现行登记面 | 未核实 |
| 诚实登记范式 | 角色错配 / 工具失效 / ponytail / 缺口全列 | **仍成立**（AGENTS.md §1、00-backlog 债栏） | 仍成立 |
| 0 装边界写法 | schema-only ≠ 物理实现、缓存判定 ≠ 真守门 | **仍成立**（四级口径 + 能力矩阵 `supported/available/reason`） | 仍成立 |

**本册复核统计**：24 篇中标「**已过时**」结论 **31 条**、「**仍成立**」**9 条**、「**未核实**」**11 条**。

*本册为 `reports/` 合册产物；来源 24 篇与分配表 `doc-audit/reports-assignment.txt` §01 一致（逐篇唯一归册）。历史内容按原文保留、不追溯改写；现行口径一律另起「现状对照（2026-10-23）」并指明真源。*
