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
| **结果** | **199 套件 / 5360 passed / 0 failed / 22 ignored** |
| **日期** | 2026-10-16 |
| **工具链** | rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30) / rustup 1.29.1 |
| **构建环境（Windows 实测）** | MSVC 14.44.35207（VS Build Tools 2022，C++ workload）+ Windows SDK 10.0.26100.0 |
| **来源** | 外部体验审计本机实测（2026-10-16，附录「审计者实测基线」，可按同命令复现） |

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
| 2026-10-13 | 5243 passed / 0 failed / 22 ignored，190 套件 | `cargo test --workspace --locked -j 2` 全量实测 |
| 2026-10-06 | 3418 passed / 0 failed / 19 ignored，129 套件 | W2 接线批后实测（当日复核批口径 3406） |
| 2026-09-30 | 4829 passed / 0 failed，161 套件 | rc.3 批 |
| 2026-09-26 | 3662 passed / 0 failed / 21 ignored，130 套件 | 数字口径统一批 |
| 2026-09-05 | 3120 passed / 0 failed / 13 ignored | 对账批 |
| 2026-08-28 | 1739 passed（A 块后） | v2.0.0-rc.1 收盘期 |

> 冻结口径说明：`README.md` / `README.zh-CN.md` 的徽章数字（3695）为 owner 冻结口径，
> 本文件不代改、也不以冻结值覆盖实测值。

## 4. 引用方式

- 其他文档写「测试基线见 [docs/03-reference/test-baseline.md](test-baseline.md)」，
  需要具体数字时写「（2026-10-16 口径，详见 test-baseline.md）」并注明日期。
- 徽章等必须内联数字的场合：数字取本文件 §1，并保持与本文件同步刷新。

_本文档由「落差清账·文本可信度批」创建（审计修 B-01 / C-07，2026-10-16）。_
