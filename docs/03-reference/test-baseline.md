# 测试基线（单一真源 / Single Source of Truth）

> **用途**：全仓测试数字的唯一真源。其他文档（INSTALL / SECURITY / ROADMAP / README 草稿 /
> capabilities-matrix 等）**只引用本文件、不抄写数字**；数字刷新只改本文件一处。
> **维护**：每次全量实测收卷后更新本表，并在「历史口径」追加旧行（带日期保留，不追溯改写）。
> **维护权/执行权（2026-10-21 起）**：由效率线（全量门禁执行线）持有；全量唯一执行器 =
> `pwsh scripts/gate_full.ps1 -FailIfBusy`（不自拼 cargo test；锁/失败分类由 R6-A 执行器负责）。
> **性质**：本节为「落差清账·文本可信度批」（审计修 A-03 / B-01 / C-07）落点。

---

## 1. 现行基线

| 项 | 值 |
|---|---|
| **命令** | `pwsh scripts/gate_full.ps1 -FailIfBusy`（R6-A 唯一全量执行器：main workspace `cargo test --workspace --all-targets --locked --no-fail-fast --exclude <图像家族 11 crate>` + 图像家族独立同命令；等价于全 workspace 口径） |
| **结果** | **341 套件 / 8570 passed / 0 failed / 51 ignored**（main 264 套件 / 6263 passed / 19 ignored + image 家族 77 套件 / 2307 passed / 32 ignored；首轮环境敏感失败口径已退役，见 §3） |
| **日期** | 2026-10-21 |
| **工具链** | rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30) / rustup 1.29.1 |
| **构建环境（Windows 实测）** | MSVC 14.44.35207（VS Build Tools 2022，C++ workload）+ Windows SDK 10.0.26100.0 |
| **来源** | 由全量门禁执行线（效率线）执行（接管日期 2026-10-21；Wave 4 收口时由架构线以 R6-A 执行器代跑的事实保留于此）：`pwsh scripts/gate_full.ps1 -FailIfBusy` 实测 2026-10-21、26.8 分钟、exit 0、HEAD `f2dad300`；日志 `artifacts/wave4-close-full-2026-10-21.log` 与 `logs/gate-full.log` |

> 复现提示：先 `cargo build --workspace --locked`（约 4 min，CARGO_BUILD_JOBS=8），再跑上表命令
> （约 11 min）；数字随批次增长属正常，**以最近一次全量实测为准**，逐批交付文档只报自己批的增量。
>
> **复测注记（2026-10-16，落差清账·文本可信度批校验）**：同命令（加 `-j 2`）在当前工作树
> 复测 = **191 套件 / 5390 passed / 0 failed / 18 ignored**，exit 0。与上表差异来自树间在途批次
> 增量（例数 +30、ignored -4）；上表为外部审计基线口径，对外引用保持上表，现场以复测类实数为准。
>
> **复测注记（2026-10-17，大总装接线批校验）**：同命令（加 `-j 2`）复测 = **208 套件 /
> 5619 passed / 0 failed / 20 ignored**，exit 0（「套件」= 测试二进制目标计数；本批 +1 =
> `runtime-assembly/tests/persona_constant_injection.rs` 恒定注入两态装配测试）。本批增量 =
> self_status 十四节合同（persona 节四面 + 缺节即红断言）、`self_status_persona` 探测口、
> TurnStart 恒定注入接线、capabilities-matrix §0.2 开关行；上表为外部审计基线口径，对外引用
> 保持上表，现场以复测类实数为准。
>
> **复测注记（2026-10-17，心跳宿主两步接线批校验）**：同命令（加 `-j 2`）复测 = **224 套件 /
> 5898 passed / 0 failed / 20 ignored**，exit 0（「套件」= 测试二进制目标计数；本批 +1 =
> `cli/tests/care_heartbeat_host.rs` 心跳宿主接线套件）。本批增量 = 心跳宿主两步接线
> （gateway presence 装配级读取口 + 宿主三注入点：闸经 `FnInitiativeGate` 注入 `CareEngine`
> / dispatch 接 `Runtime::execute` / `care_status_probe_with_heartbeat` 注自述关怀节，
> 双闸齐开才起）+ 自述关怀节心跳两列 null→真值 + capabilities-matrix §0.2 `retrieval_tuning`
> 开关行与名册互钉（新增测试 6 例）；接线对照表与测试证据见
> `artifacts/care-heartbeat-host-wiring-report.md`，日志
> `artifacts/care-heartbeat-host-wiring-workspace-test.log`。上表为外部审计基线口径，对外引用
> 保持上表，现场以复测类实数为准。
>
> **复测注记（2026-10-18，许可搬运线第一批·图像核心）**：同命令（加 `-j 2`）**按排除他线在途编译红
> crate 的口径**（`--exclude apeireth-tools-canonical`；红面归口 W1 线自检收口，全量复验待收口后
> 回填）复测 = **287 套件 / 7580 passed / 0 failed / 48 ignored**，exit 0（隔离 target 目录
> `target/licport` 出数，同命令面同 `--locked`；共享 target 与他线并发构建互撞 LNK1104 已在
> `docs/04-internal/license-port/gate4-integration.md` §6 登记）。本批增量 = 图像核心 vendored
> 家族 10 crate（迁移测试 **77 套件 / 2307 passed / 0 failed / 32 ignored**）+ 接口层
> `apeireth-image-core` 冒烟 **9 例**；四关质检卷宗 `docs/04-internal/license-port/`，日志
> `artifacts/licport-workspace-test-isolated.log`。上表为外部审计基线口径，对外引用保持上表，
> 现场以复测类实数为准。
>
> **复测注记（2026-10-18，四批大收卷·总装批校验）**：同命令（加 `-j 2`）**全量口径**复测 =
> **322 套件 / 8509 passed / 0 failed / 52 ignored**，exit 0（**首次全量回填**：许可搬运线第一批
> 当时的 `--exclude apeireth-tools-canonical` 287 套件口径已收口 —— 换核批把该 crate 接回树内，
> 全量含其全部目标）。本波四批增量 = ①K 轴 L3/L4 设备审批卡→IM 推送链（+17 例）②K 轴 L2 提醒
> 闭环 + L5 守护骨架（+22 例）③创作面补齐 xlsx 读入 + DOCX 写面（+15 例）④图像操作换创作内核 +
> 词表 11→13（+11 例）；同批工程收口（凭据绊线 JWT/Bearer 两处空槽、认知调度器严格 FIFO、体验
> 旋钮测试减敏、deny 治理对账、只读预设补设备执行面/关怀规则面）不单独计套件。日志
> `artifacts/wave-1018-final-workspace-test.log`。
>
> **复测注记（2026-10-18，小批总装批校验）**：同命令（加 `-j 2`）全量复测 =
> **322 套件 / 8515 passed / 0 failed / 52 ignored**，exit 0。较上一口径（8509 passed）+6 例：
> ①`docx_from_markdown` 补 `numbering.xml` +1 例（六件包 + 正文引用 ⊆ numbering 定义交叉校验）
> ②`sheet_ops` 新增 `export_xlsx` +4 例（往返逐格 / 五件包 XML 检视 / 受控面同族 / 坏输入拒）
> ③其余 +1 为同批词表钉与自述节计数调整（`tools` lib 562→567；`--test sheet_ops` 23 passed）；
> 前端独立 workspace `pnpm test` 82/82 suites 不并入本表。日志
> `artifacts/wave2-final-workspace-test.log`；clippy `--workspace -D warnings` 0 警告、`cargo fmt --check` 0 diff 同批亲跑。
> **复测注记（2026-10-19，生物机制族 W1 总装批校验）**：同命令（加 `-j 2`）全量复测 =
> **322 套件 / 8652 passed / 0 failed / 52 ignored**，exit 0（上口径 8515 → **+137**：
> organ +79 / guard +18 / orchestration +40，全部为生物机制族 W1 九条内核的机制单测）。
> 本批九条（SG-13/14/15/16/17/19/20/23/24）均为 **IMPLEMENTED 库级、未接线**（生产 0 调用点）；
> 三 crate scoped clippy 0 警告、`cargo fmt --check` 0 diff 同批亲跑。日志
> `artifacts/wave-bio-w1-workspace-test.log`。
> **复测注记（2026-10-19，生物机制族 W3.1 总装批校验）**：同命令（加 `-j 2`）全量复测 =
> **322 套件 / 8820 passed / 0 failed / 52 ignored**，exit 0（上口径 8652 → **+77**：guard +10 /
> organ +57（care 24 + context/circadian 14 + tone 10 + prompt 9）/ orchestration +10，全部为
> W3.1 六条 crate 内接线单测）。同批 W2 全量曾出现 1 次环境敏感测试失败
> （process::appcontainer::tests::probe_appcontainer_launch_succeeds_with_step_attribution，
> CreateProcessW(AppContainer): 系统找不到指定的文件）；单独跑与单 crate lib 复跑均通过，
> 本批全量亦通过 —— 已登记 **D25**（待专项处置），不当作本波回归。日志
> `artifacts/wave-bio-w31-workspace-test.log`。
> **复测注记（2026-10-19，生物机制族 W3.2 总装批校验）**：全量复测 =
> **326 套件 / 8889 passed / 0 failed / 52 ignored**（上口径 8820 → **+69**：W3.2 新增装配面
> 与观测面测试，含 4 个新装配测试文件 = 套件数 322 → 326）。**口径差异如实说明**：标准
> -j 2 全量本轮多次触发 **D26 环境问题**（Windows administered 保留端口区间 50000-50059
> 被 mock server 临时端口分配偶发命中 → WSAEACCES 10013，5–12 个 socket 测试套件偶发失败；
> 单套件复跑均绿，产品面 0 失败）；最终权威绿跑 = **同命令 + -j 1 + RUST_TEST_THREADS=4
> 降并发规避**，全 326 套件 0 失败。D26 记录于 00-backlog，彻底修需管理员移除保留区间
> 或测试侧加 bind 重试。日志 `artifacts/wave-bio-w32f-workspace-test.log`。
> **复测注记（2026-10-20，第三线权威复跑 / D27 证据链补全）**：生物机制族 W3.4 首轮全量日志
> `artifacts/wave-bio-w34-workspace-test.log` 汇总 = 336 套件 / 9075 passed / **1 failed** / 52 ignored，
> 失败帧 = `apeireth-core --lib` 的 `storage_atomic::tests::write_atomic_refuses_preexisting_tmp_name`
> （D27 环境敏感；首轮 §1 曾以「产品面 0 失败」口径记 0 failed 但缺绿跑日志背书）。2026-10-20 第三线
> 按同口径（`-j 1` + `RUST_TEST_THREADS=4`，`--no-fail-fast --all-targets --locked`）权威复跑 =
> **336 套件 / 9076 passed / 0 failed / 52 ignored，exit 0**（该测试 `... ok`；+1 = 失败测试转绿）。
> §1 改为绿跑口径 9076；首轮 9075 口径移入 §3 历史备查。日志
> `artifacts/wave-dts-authoritative-full-2026-10-20.log`。

> **复测注记（2026-10-20，生物机制族 W3.5 总装批校验）**：本批全量实测 = **338 套件 / 8997 passed /
> 0 failed / 51 ignored**（命令加 `-j 1` + `RUST_TEST_THREADS=4` 规避 D26）。**口径提示**：该轮跑时
> 另一条施工线（R4b「SDK 三岛归档」）的 staged 改动在途（`legacy/archived/sdk-islands/` 重命名已入
> index），工作树测试面与 §1 现行口径（336 套件 / 9076 passed）不同——主要差异 = SDK 归档岛测试尚未
> 计入，故 passed 数低于 §1。**本数字不替换 §1 单一真源**，仅作本批交付证据；待 R4b 落地后由基线线
> 统一复跑并刷新 §1。本批新增测试证据（scoped，全部绿）：assembly lib 214、`bio_lifecycle_wiring` 11、
> `bio_lifecycle_production` 4、`bio_tone_feedback` 8、`bio_mechanisms_self_status` 22、`nervous_snapshot`
> 11、`self_status_runtime_e2e` 12、`startup_instrumentation` 9、cli lib 56、`care_heartbeat_host` 4；
> `arch_guard --strict` VIOLATIONS 0（STALE 11 = 其它线在途缩表项）。日志
> `artifacts/wave-bio-w35-workspace-test.log`。

> **复测注记（2026-10-20，生物机制族 W3.6 总装批校验）**：本批 = probe 迟绑定生产接入（`f7a436a9`，
> 观测面 2/14 → **7/14**）+ 生命周期账本持久化（`8c4fa5c2`），**+31 新测**（probe 12 + 持久化 19）。
> **scoped 全绿证据**：assembly lib 235、`bio_mechanisms_self_status` 22、`nervous_snapshot` 11、
> `self_status_runtime_e2e` 12、`bio_lifecycle_persistence` 10、`bio_lifecycle_production` 4、
> `bio_lifecycle_wiring` 11、cli lib 56、`care_heartbeat_host` 4；`cargo clippy --all-targets -D warnings`
> 0、`cargo fmt --check` 0 diff、`arch_guard --strict` VIOLATIONS 0。
> **全量口径本轮未出数（如实）**：两次降并发（`-j 1` + `RUST_TEST_THREADS=4`）全量均在他线在途改动上
> 编译中断——①`apeireth-memory` (lib) 1 error；②`apeireth-runtime` (lib) E0252 重复导入 3 errors
> （R7 序列 ContextBlock/会话 trait 收敛在途），均非本批文件（本批代码已提交、scoped 绿）。**本批不替换
> §1 单一真源**，仅登记交付证据；待跨线收口稳定后按同命令面统一复跑并刷新 §1。日志
> `artifacts/wave-bio-w36-full-test.log` / `artifacts/wave-bio-w36-full-test2.log`。


> **复测注记（2026-10-20，生物机制族 W3.7 事件源批校验）**：本批 = tone 事件源两档（`53e7aa09`：
> 工具名进程内首见 → Novelty；CLI 审批落账后人批/人拒 → SocialPositive/SocialNegative），tone 事件源
> **5/9**。**scoped 全绿证据**：bio_tone lib 12、`bio_tone_feedback` 10（含本批新增 2）、cli lib 56、
> `canonical_cli_bootstrap` 6、早前 assembly lib 237；`cargo clippy -p apeireth-cli -p
> apeireth-runtime-assembly --all-targets -D warnings` 0、`cargo fmt --check` 0 diff、`arch_guard
> --strict` VIOLATIONS 0（STALE 1 = cli lib 可缩表 3887→3870，归效率线）。**全量本轮未出数（如实）**：
> 他线 R2-4 命名迁移 / R7 归档在途，且本机页文件耗尽使 nextest 编译期 `os error 1455`（
> `logs/verify-changed-w37b.log`）；**不替换 §1**，待跨线收口后按同命令面统一复跑刷新。环境事件登记 D28。

> **复测注记（2026-10-20，生物机制族 W3.7 仪式完成面批校验）**：本批 = SG-22 完成事件源
> （`7e98744d`：`tool.ritual_done`/`tool.ritual_skip`，默认关，复用 bio 候选总闸 + care 数据目录），
> 完成源从「无」→ 工具面。**scoped 全绿证据**：`ritual_tools` 单元 5 passed、`bio_ritual_tools` 集成
> 5 passed（默认关零注册零文件 / 无目录拒注册 / 开时恰好两工具且真实调用写出 StoredDoc v1 /
> 非精确值全关 / 两 id 属 write_or_execute）、`cargo test -p apeireth-runtime-assembly --locked` 全目标
> 0 failed、cli 架构守卫 2 passed；`cargo clippy -p apeireth-cli -p apeireth-runtime-assembly
> --all-targets -D warnings` 0、`cargo fmt --check` 0 diff、`arch_guard --strict` VIOLATIONS 0。
> **全量如实登记**：20:46 启动的首跑出现 1 失败 = `canonical_cli_architecture` 仍以 `53e7aa09`
> 已移走的文档注释为切片终点（`35c4b24a` 修正，非生产缺陷），该跑已中止、数字不可作基线；权威全量
> 待他线 R2-4/R7 收敛后按同命令面（`-j 1` + `RUST_TEST_THREADS=4`）复跑并刷新 §1。**§1 本轮不替换**。

> **复测注记（2026-10-20，生物机制族 W3.7 SG-25 生态位观测面 v0 校验）**：本批 = 仓内策展表
	> （`tool_taxonomy`，40 条工具 id/名 → 13 类标签，静态快照真源）+ `tool_ecosystem_probe` 经 `BioProbeSlots`
	> 迟绑定接入 `APEIRETH_ENABLE_BIO_SELF_STATUS`（不新增 env），观测面 **7/14 → 8/14**（`9133c7bb`）。
	> **scoped 全绿证据**：`tool_taxonomy` 单测 5、`tool_ecosystem_probe` 单测 6、`bio_tool_ecosystem` 集成 3、
	> `cargo test -p apeireth-runtime-assembly --locked` 全目标 0 failed（lib 252 + 全部集成绿）、`bio_ritual_tools`
	> 5；`cargo clippy -p apeireth-runtime-assembly --all-targets -D warnings` 0、`cargo fmt --check` 0 diff、
	> `arch_guard --strict` VIOLATIONS 0。**全量本轮未出数（如实）**：W3.7 首跑因 cli 架构守卫旧切片终点失败
	> （`35c4b24a` 已修，非生产缺陷）且该跑在他线在途改动上启动、已中止不可作基线；权威全量须**单线独占**按
	> 同命令面（`-j 1` + `RUST_TEST_THREADS=4`）复跑（今晚双线并发全量曾致 image-compose OOM，数字不可发布），
	> **今晚全量尝试如实登记**：①21:30 起跑，22:04 进程被环境终止于 image 套件（日志 `artifacts/wave-bio-w37c-workspace-test.log`：139/340 套件、唯一失败 = 已知 D27 环境敏感 `storage_atomic` 用例、无编译错误）；②22:22 起跑在编译期被他线在途 R1b-4a（`nervous_snapshot.rs` schema v2 迁移）签名变更阻断（非本批文件，见 `wave-bio-w37c-workspace-test2.log`）；③22:25 起跑在 owner 收工口令下主动停止。三跑均不作为基线数字，§1 仍待明日单线独占全量刷新后广播。**§1 本轮不替换**。

> **复测注记（2026-10-21，R1b-2a 调质订阅总线批校验）**：本批 = 调质事件面从「模块 `on_hook` 旁路扫
> `AfterToolResult`/`OnError`」收敛到「`nervous.bridge` → `NervousBus` → `BioToneBusSubscriber` 订阅」
> （`404919f6`，默认关）。订阅者与模块共享同一 `Arc<ToneState>` 与同一首见集合（Novelty 语义不变）；
> 开关关 = 不构造订阅者、总线零订阅零发布、语气状态与叠加逐字节零变化。**scoped 全绿证据**：
> `cargo test -p apeireth-runtime-assembly --locked` 全目标 **62 套件 / 676 passed / 0 failed**（lib 281；
> bio_tone 单测 16、`bio_tone_feedback` 10、`bio_tone_wiring` 8）；`cargo clippy -p apeireth-runtime-assembly
> --all-targets -- -D warnings` 0、`cargo fmt --check` 0 diff、`arch_guard --strict` VIOLATIONS 0（STALE 1 =
> `play_tools.rs` 可缩表；EXC-0002/0003 属架构线在途例外）、`docs_truth --verify --check` OK。
> **验收（R1b-2 handoff）**：既有 tone 测试全绿 + 新「总线订阅与旧 hook 观测路径终态等价」测试；
> 开关开时订阅生效、关时总线零订阅；无双重推进。**未做（如实）**：R1b-2b/2c、R1b-3 未开始；
> 权威全量仍未出数。**§1 本轮不替换**。

> **复测注记（2026-10-21，R1b-2b 生命周期订阅总线批校验）**：本批 = `bio_lifecycle` 工具结果面从「模块 `on_hook` 扫描 `AfterToolResult`」收敛到「`nervous.bridge` → `NervousBus` → `BioLifecycleBusSubscriber` 订阅」，并接通 `BioMechanismKind::Lifecycle` 观测格（`bio_mechanisms` 观测面 8/14 → 9/14；默认关零注册/零订阅/零 fill）。实测：`cargo test -p apeireth-runtime-assembly --locked` 全目标绿（lib **284** passed；集成套件 0 failed；`bio_lifecycle_wiring` **11** passed、`nervous_snapshot` **28** passed）；`cargo clippy -p apeireth-runtime-assembly --all-targets --locked -- -D warnings` 0；`cargo fmt -p apeireth-runtime-assembly --check` 0；`arch_guard --strict` VIOLATIONS 0（STALE 1 = play_tools.rs 可缩表）；`docs_truth --verify --check` OK。
> **fast-gate（verify_changed.py）未出全链绿，如实登记**：①/② 次 nextest 构建阶段 `link.exe` exit 1102 / 页面文件不足（环境，记 `00-backlog` D28）；③ 次 check 阶段被在途未提交 governance 改动阻断（`error[E0277]` `crates/foundation/governance/src/chain.rs:372` `(dyn ChainClock + 'static)` 未实现 `Debug`）。scoped 门禁在阻断前已跑绿（上列数字）。
> **口径**：全量工作区测试本批未跑（他线在途并发构建，数字不作基线）；**§1 本轮不替换**；等单线独占全量后再统一刷新 §1 + README 徽章。
> **复测注记（2026-10-21，Wave 4 收尾批校验：SG-20 观测面 14/14 + 桌面 CI 执行面）**：本批 = ①SG-20 社会修复
> 观测面（人工拒绝审批 = 事件源，总线订阅 → 内核 `plan_repair` 只读折算；默认关零订阅）→ `bio_mechanisms`
> 观测面 **14/14**；②`companion-desktop-ci.yml` 执行面补齐（tauri-shell `cargo test`、svelte-ui `pnpm test`、
> node 20→22）。**scoped 实测**：`cargo test -p apeireth-runtime-assembly --locked -j 2` → 63 套件 /
> **708 passed** / 0 failed（lib 309；`bio_social_repair` 4、`bio_observation_cells` 3、`nervous_snapshot` 13）；
> 桌面 `cargo test --workspace --all-targets`（src-tauri）→ lib 140 passed / 3 ignored + integration 11；
> `pnpm test` → **82/82 suites**；clippy/fmt 绿。**环境注记（如实）**：本机 `%TEMP%` 对本仓新编译测试进程
> 自 2026-10-21 17:40 前后起拒写（`os error 5`；`cmd`/`python` 可写），本批测试以
> `TEMP/TMP=<repo>\target\tmp-test` 覆盖运行（等价 CI 临时目录语义），非代码回归，登记 `00-backlog` D30。
> **口径**：全量工作区测试本批未跑（效率线执行权）；桌面三组数字为本机 Windows 实测，**CI 远端 ubuntu
> 首跑未验证**；**§1 本轮不替换**。
> **复测注记（2026-10-21，Wave 4 SG-14 homeostasis 行为接线批校验）**：本批 = `context_budget` 单真源
> （`APEIRETH_CONTEXT_BUDGET_CHARS`，cli 与装配根转调同一函数）+ `install_bio_modules` 注入预算 +
> `ObservedBioAssemblyModule` TurnStart 回合观测（只记数字/策略，不含用户原文）+ `BioProbeSlots` 第 7 槽 +
> `BioMechanismKind::Homeostasis` 注册（`bio_mechanisms` 观测面 12/14 → **13/14**，默认关零注册零 fill）。
> **scoped 全绿证据**：`cargo test -p apeireth-runtime-assembly --locked -j 2` → 63 套件 / **704 passed** / 0 failed
> （lib 305；`bio_observation_cells` 3 / `nervous_snapshot` 13 / `bio_lifecycle_wiring` 11 等）；`-p apeireth-cli --lib`
> **57 passed**；两 crate `clippy -D warnings` 0、`cargo fmt --check` 0 diff。
> **fast-gate 未全绿（如实）**：`arch_guard --strict` 唯一违规 = 他线在途未提交 `crates/adapters/cli/src/lib.rs`
> 3878→3885（R8-B2 治理链在途，非本批文件；本批 staged cli 段净 -1 行）；全量测试未跑，**§1 不替换**
> （执行权归效率线 `gate_full.ps1 -FailIfBusy`）。
> **复测注记（2026-10-21，Wave 4 观测补格 + 环境债去抖批校验）**：本批 = ①`bio_extra_probes` 三格自述读数
> （调质 / 昼夜 / 仪式）经迟绑定槽接入 `APEIRETH_ENABLE_BIO_SELF_STATUS`（观测面 9/14 → **12/14**，默认关零注册零 fill）；
> ②`18691532` 环境敏感测试去抖（D25 AppContainer 探针有界重试 + SKIP 显式化、D26 tools crate 10 处 bind 重试 helper、
> D27 storage_atomic 进程唯一后缀）；③D18 取证 = `962f30a8` 已落地、本批 0 代码改动。
> **scoped 全绿证据**：`cargo test -p apeireth-runtime-assembly --locked -j 2` 全目标 0 failed（lib **300**；
> 新集成 `bio_observation_cells` **3**；`bio_lifecycle_wiring` 11 / `nervous_snapshot` 13 / `bio_tool_ecosystem` 3 /
> `bio_ritual_tools` 5）；`cargo test -p apeireth-core --lib` **199**；`-p apeireth-tools-canonical --lib` **595**；
> 集成 `fetch_execution` 22 / `content_feed` 13 / `content_feed_lifecycle` 11；两 crate `clippy -D warnings` 0、fmt 0 diff。
> **fast-gate 未全绿（如实）**：`verify_changed --arch-strict --no-test` 唯一失败 = 他线在途未提交的
> `crates/adapters/cli/src/lib.rs` 尺寸 3878→3886（非本批文件；本批改动 arch 检查 0 违规）。
> **口径**：全量工作区测试本批未跑（他线并发提交中）；**§1 本轮不替换**。全量执行权与 §1 刷新归效率线
> （`7153f5db`，`gate_full.ps1 -FailIfBusy` 唯一执行器；现行 §1 = 341 套件 / 8570 passed / 0 failed / 51 ignored，2026-10-21）。

> **复测注记（2026-10-21，R1b-2c 审批社交总线批校验）**：`cargo test -p apeireth-runtime-assembly --locked -j 2`
> → 62 套件 / 0 failed（lib 291 passed）；`cargo test -p apeireth-cli --locked -j 2` → 56 lib + 各集成套件全绿
> （该次全量跑跳过一条他线在途 R1b-3 端到端测试，随后由他线 `0b71cd6f` 提交并复跑绿）；`cargo clippy -p
> apeireth-runtime-assembly -p apeireth-cli --all-targets --locked -- -D warnings` → 0；`cargo fmt --check` 0 diff；
> `python scripts/arch_guard.py --strict` → VIOLATIONS 0；`python scripts/verify_changed.py --files <11 文件>
> --arch-strict --no-test` → FAST GATE PASSED（check 5.3s / clippy 7.6s）。
> **口径**：全量工作区测试本批未跑（他线仍在并发提交）；**§1 本轮不替换**；待单线独占全量后统一刷新 §1 + README 徽章。
> **默认关**：开关关 = 装配根不交付审批社交源 = 零发布 = 总线 `is_idle()`，审批结论 / 权限判定零变化（逐字节测试）。

## 2. 口径定义

| 口径 | 定义 | 用途 |
|---|---|---|
| **运行时通过数**（本表采用） | `cargo test --workspace --all-targets --locked --no-fail-fast` 实际绿数 | 对外引用、徽章、验收 |
| 静态计数 | `#[test]` 标注总数（含平台/feature 未编译项，约 4,101） | 仅作覆盖参考，不对外 |
| ignored | 需真 API key / 真硬件的 live E2E（`#[ignore]`），CI 默认不跑 | 数量单列，不计入 passed |

- 「套件」= 测试二进制目标（lib + 各 tests/ 目标）计数，逐批随在途线增长；
  「例」= 用例数（passed/failed/ignored）。
- 前端 `frontend/companion-desktop` 独立 workspace，`pnpm test` 数字不并入本表。

## 3. 历史口径（带日期保留，不追溯改写）

| 日期 | 结果 | 备注 |
|---|---|---|
| 2026-10-21 | 341 套件 / 8570 passed / 0 failed / 51 ignored | Wave 4 收口（gate_full 实测；R4b 四岛归档后首次全量口径：main 6263 + image 2307；较 2026-10-20 的 9076 下降主要来自 R4b 归档移除的测试（SDK 607→92 等）与新增机制的净差） |
| 2026-10-20 | 336 套件 / 9075 passed / 1 failed / 52 ignored（产品面 0 失败） | 生物机制族 W3.4 首轮全量（D27 环境敏感 1 失败；非绿跑口径；第三线权威复跑 9076 绿后移入历史） |
| 2026-10-19 | 326 套件 / 8889 passed / 0 failed / 52 ignored | 生物机制族 W3.3 批（§1 曾用，2026-10-20 W3.4 批刷新后移入历史） |
| 2026-10-19 | 322 套件 / 8820 passed / 0 failed / 52 ignored | 生物机制族 W3.1 批（§1 曾用，2026-10-19 W3.2 批刷新后移入历史） |
| 2026-10-19 | 322 套件 / 8652 passed / 0 failed / 52 ignored | 生物机制族 W1 批（§1 曾用，2026-10-19 W3.1 批刷新后移入历史） |
| 2026-10-18 | 322 套件 / 8515 passed / 0 failed / 52 ignored | 小批总装批（§1 曾用，2026-10-19 生物机制族 W1 批刷新后移入历史） |
| 2026-10-16 | 199 套件 / 5360 passed / 0 failed / 22 ignored | 外部体验审计本机实测（初版 §1 现行基线，2026-10-18 小批总装批刷新后移入历史） |
| 2026-10-13 | 5243 passed / 0 failed / 22 ignored，190 套件 | `cargo test --workspace --locked -j 2` 全量实测 |
| 2026-10-06 | 3418 passed / 0 failed / 19 ignored，129 套件 | W2 接线批后实测（当日复核批口径 3406） |
| 2026-09-30 | 4829 passed / 0 failed，161 套件 | rc.3 批 |
| 2026-09-26 | 3662 passed / 0 failed / 21 ignored，130 套件 | 数字口径统一批 |
| 2026-09-05 | 3120 passed / 0 failed / 13 ignored | 对账批 |
| 2026-08-28 | 1739 passed（A 块后） | v2.0.0-rc.1 收盘期 |

> 冻结口径说明：`README.md` / `README.zh-CN.md` 的徽章数字现行口径 = **8570 passed / 0 failed（2026-10-21 Wave 4 收口口径）**，与 §1 同日同口径同步；历史冻结值（8509 / 3695）已退役，不再适用。
> 2026-10-18 README 换回退役，不再适用。数字刷新时徽章与本表 §1 同日同口径同步。

## 4. 引用方式

- 其他文档写「测试基线见 [test-baseline.md](test-baseline.md)」，
  需要具体数字时写「（2026-10-18 口径，详见 test-baseline.md）」并注明日期。
- 徽章等必须内联数字的场合：数字取本文件 §1，并保持与本文件同步刷新。

_本文档由「落差清账·文本可信度批」创建（审计修 B-01 / C-07，2026-10-16）。_
