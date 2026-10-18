# 测试基线（单一真源 / Single Source of Truth）

> **用途**：全仓测试数字的唯一真源。其他文档（INSTALL / SECURITY / ROADMAP / README 草稿 /
> capabilities-matrix 等）**只引用本文件、不抄写数字**；数字刷新只改本文件一处。
> **维护**：每次全量实测收卷后更新本表，并在「历史口径」追加旧行（带日期保留，不追溯改写）。
> **性质**：本节为「落差清账·文本可信度批」（审计修 A-03 / B-01 / C-07）落点。

---

## 1. 现行基线

| 项 | 值 |
|---|---|
| **命令** | `cargo test --workspace --all-targets --locked --no-fail-fast` |
| **结果** | **322 套件 / 8515 passed / 0 failed / 52 ignored** |
| **日期** | 2026-10-18 |
| **工具链** | rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30) / rustup 1.29.1 |
| **构建环境（Windows 实测）** | MSVC 14.44.35207（VS Build Tools 2022，C++ workload）+ Windows SDK 10.0.26100.0 |
| **来源** | 小批总装批本机全量实测（2026-10-18，同命令可复现；日志 `artifacts/wave2-final-workspace-test.log`） |

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
| 2026-10-16 | 199 套件 / 5360 passed / 0 failed / 22 ignored | 外部体验审计本机实测（初版 §1 现行基线，2026-10-18 小批总装批刷新后移入历史） |
| 2026-10-13 | 5243 passed / 0 failed / 22 ignored，190 套件 | `cargo test --workspace --locked -j 2` 全量实测 |
| 2026-10-06 | 3418 passed / 0 failed / 19 ignored，129 套件 | W2 接线批后实测（当日复核批口径 3406） |
| 2026-09-30 | 4829 passed / 0 failed，161 套件 | rc.3 批 |
| 2026-09-26 | 3662 passed / 0 failed / 21 ignored，130 套件 | 数字口径统一批 |
| 2026-09-05 | 3120 passed / 0 failed / 13 ignored | 对账批 |
| 2026-08-28 | 1739 passed（A 块后） | v2.0.0-rc.1 收盘期 |

> 冻结口径说明：`README.md` / `README.zh-CN.md` 的徽章数字现行口径 = **8509 passed / 0 failed
> （2026-10-18 大波口径，owner 换回 README 时同步）**，本文件不代改 README；历史冻结值 3695 已随
> 2026-10-18 README 换回退役，不再适用。数字刷新时徽章与本表 §1 同日同口径同步。

## 4. 引用方式

- 其他文档写「测试基线见 [docs/03-reference/test-baseline.md](test-baseline.md)」，
  需要具体数字时写「（2026-10-18 口径，详见 test-baseline.md）」并注明日期。
- 徽章等必须内联数字的场合：数字取本文件 §1，并保持与本文件同步刷新。

_本文档由「落差清账·文本可信度批」创建（审计修 B-01 / C-07，2026-10-16）。_
