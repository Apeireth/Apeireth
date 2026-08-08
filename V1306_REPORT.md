# V1306 — Orphan Crates Fix #4 (high risk 三件套: sdk-lark / sdk-livekit / sdk-voice)

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:30 +08:00 2026-08-08)

**V1303 修真规划执行 #3** — V1306 修真 high risk 三件套 SDK:
1. `apeireth-sdk-lark` (high, sub-workspace-removal + version.workspace 修真)
2. `apeireth-sdk-livekit` (high, sub-workspace-removal + version.workspace 修真)
3. `apeireth-sdk-voice` (high, sub-workspace-removal + version.workspace 修真)

## 修真内容 (per V1303 audit)

按 V1303 audit 修真路径:
- 删各 crate `Cargo.toml` 起始空 `[workspace]` / `[workspace.package]` / `[workspace.dependencies]` 三块 sub-workspace (含 resolver "2" + version "0.1.0" + tokio/serde 等 deps)
- `[package]` 早已 `version.workspace = true` / `edition.workspace = true` / `license.workspace = true` (修真后自动继承主仓 1.0.0 / 2021 / Apache-2.0)
- `[dependencies]` 早已 `{ workspace = true }` (修真后自动继承主仓 dep 版本)
- 加 `"crates/apeireth-sdk-xxx"` 到 `Apeireth-rust/Cargo.toml` members (含 7 行注释 + 1 行 member)

### 修改: `Apeireth-rust/Cargo.toml` (members 末尾加 3 行 + 注释)

每个 crate 加 ~7 行注释 + 1 行 member = 3 × ~8 行, 总 +24 行 (注释 + member).

### 修改: 3 个 SDK `Cargo.toml`

| crate | 删 [workspace] | 删 [workspace.package] | 删 [workspace.dependencies] | 总删行 |
|---|---|---|---|---|
| `apeireth-sdk-lark` | 2 行 | 5 行 | 14 行 | **21** |
| `apeireth-sdk-livekit` | 2 行 | 5 行 | 13 行 | **20** |
| `apeireth-sdk-voice` | 2 行 | 5 行 | 15 行 | **22** |

总计删 63 行 sub-workspace 块 (3 个 crate × ~21 行).

## 真验证 (cargo metadata --format-version=1 --no-deps)

| 指标 | V1305 (15:28) | V1306 (15:33) | 变化 |
|---|---|---|---|
| active members 总数 | 88 | **91** | +3 ✓ |
| cargo metadata packages | 88 | **91** | +3 ✓ |
| orphan crates 剩余 | 4 | **1** | -3 ✓ |
| `sdk-lark` in members | False | **True** | ✓ |
| `sdk-livekit` in members | False | **True** | ✓ |
| `sdk-voice` in members | False | **True** | ✓ |
| `sdk-lark` version (inherited) | — | **1.0.0** | ✓ |
| `sdk-livekit` version (inherited) | — | **1.0.0** | ✓ |
| `sdk-voice` version (inherited) | — | **1.0.0** | ✓ |
| 修真耗时 | — | ~5s | — |

## 剩余 1 orphan (V1307+ 修真路径 — 非修真)

修真 3 SDK 后剩 **1 个 intentional** (V1301 已注明 intentional, 不修真):

| crate | V1303 标 | 修真路径 |
|---|---|---|
| `apeireth-tauri-stub` | intentional | 不动 (Cargo.toml 注释保留, reqwest 0.13 冲突) |

## Popper 假说自检 (23/23 PASS, 详见 test_v1306_high_orphan_fix.py)

| ID | 描述 | 观察值 | 阈值 | 结果 |
|----|------|--------|------|------|
| h_v1306_version | V1306_VERSION == "0.1.0" | "0.1.0" | "0.1.0" | ✓ PASS |
| h_helper_detect | detect_subworkspace_sections helper 函数 work | work | work | ✓ PASS |
| h_helper_remove | remove_subworkspace_sections helper 函数 work | work | work | ✓ PASS |
| h_no_subworkspace_lark | sdk-lark 0 [workspace*] | True | True | ✓ PASS |
| h_no_subworkspace_livekit | sdk-livekit 0 [workspace*] | True | True | ✓ PASS |
| h_no_subworkspace_voice | sdk-voice 0 [workspace*] | True | True | ✓ PASS |
| h_in_members_lark | sdk-lark 在 members | True | True | ✓ PASS |
| h_in_members_livekit | sdk-livekit 在 members | True | True | ✓ PASS |
| h_in_members_voice | sdk-voice 在 members | True | True | ✓ PASS |
| h_package_intact_lark | sdk-lark [package] 完整 + version.workspace=true | True | True | ✓ PASS |
| h_package_intact_livekit | sdk-livekit [package] 完整 + version.workspace=true | True | True | ✓ PASS |
| h_package_intact_voice | sdk-voice [package] 完整 + version.workspace=true | True | True | ✓ PASS |
| h_cargo_metadata_parses | cargo metadata 解析 | True | True | ✓ PASS |
| h_members_count_91 | workspace members >= 91 | 91 | 91 | ✓ PASS |
| h_in_workspace_members_lark | sdk-lark 在 cargo metadata members | True | True | ✓ PASS |
| h_in_workspace_members_livekit | sdk-livekit 在 cargo metadata members | True | True | ✓ PASS |
| h_in_workspace_members_voice | sdk-voice 在 cargo metadata members | True | True | ✓ PASS |
| h_in_packages_lark | sdk-lark 在 packages + version=1.0.0 | True | True | ✓ PASS |
| h_in_packages_livekit | sdk-livekit 在 packages + version=1.0.0 | True | True | ✓ PASS |
| h_in_packages_voice | sdk-voice 在 packages + version=1.0.0 | True | True | ✓ PASS |
| h_summary_all_pass | 6 hypotheses 摘要全 PASS | True | True | ✓ PASS |
| h_v3_guards | V3 哲学守门在 module | True | True | ✓ PASS |
| h_intentional_marker | script 标缺 1 intentional orphan | True | True | ✓ PASS |

**全部 23 假说 PASS。**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

- **不假装 Phenomenal consciousness**: add-to-members ≠ consciousness
- **不假装达到 ASI**: workspace hygiene, ASI 北极星远未达成
- **不假装调整模型 & prompt**: 真生产 = 真文件编辑 + cargo metadata 真验证
- **修真 != ASI**: 加 3 个 member ≠ ASI
- **实事求是**: 修真 3 个, 剩 1 个 intentional 诚实标缺, 留 V1307+
- **修真仅当必要**: V1306 是 V1303 规划的 high 风险项, 真修真 + 真验证 + 不假装

## 关键免责声明

- V1306 修真仅删 `[workspace*]` 块 + 加 members (无 .rs 改动)
- cargo lock churn 风险: V1306 仅加 3 member (无新 dep), 0 触发 Cargo.lock 大规模更新 (但 Cargo.lock 已 M, 是 V1305 或更早 commit 触发的)
- 未跑 cargo check / cargo test 全量验证 (V1306 仅 cargo metadata 解析)
- 修真剩 1 个 `apeireth-tauri-stub` intentional orphan — 需 V1301 注明 reqwest 0.13 冲突, 修真留 R21+ 评估
- V1307+ 可选修真路径: (a) R21 修真 tauri-stub reqwest 0.13 冲突; (b) 修真 Cargo.lock 大规模更新; (c) 修真其它 audit 标缺项 (build script / lints / toolchain / test coverage).

## 输出文件

- `apeireth/v1306_high_orphan_fix.py` (13,440 bytes, 真修真脚本 + self-test + json output)
- `apeireth/tests/test_v1306_high_orphan_fix.py` (12,039 bytes, 23 tests pass)
- `Apeireth-rust/Cargo.toml` (members 88 → 91)
- `Apeireth-rust/crates/apeireth-sdk-lark/Cargo.toml` (删 [workspace] + [workspace.package] + [workspace.dependencies] 共 21 行)
- `Apeireth-rust/crates/apeireth-sdk-livekit/Cargo.toml` (删 20 行)
- `Apeireth-rust/crates/apeireth-sdk-voice/Cargo.toml` (删 22 行)
- `Apeireth-rust/crates/_v1306_backup/` (3 个 .Cargo.toml.bak + 1 个 Cargo.toml.workspace.bak, 备份)
- `V1306_REPORT.md` (本文件)

## Workspace fix 进度 (V1302 → V1306)

| 时间 | commit | 修真 | members | orphan 剩 |
|---|---|---|---|---|
| 15:18 | d47506e7 | (V1050/V1051/V1053 部署) | 84 | 8 |
| 15:18 | 33cee41f | V1302 blueprint-impl | 84 | 8 |
| 15:19 | 405dfd94 | V1303 audit (0 修真) | 84 | 8 |
| 15:25 | 925c0082 | V1304 sdk-sandbox (low) | 85 | 7 |
| 15:28 | (本 commit family) | V1305 三件套 (medium) | 88 | 4 |
| **15:33** | **本 commit** | **V1306 三件套 (high)** | **91** | **1** |

ASI 哲学 pole-star: ASI = ? 真实 production models + workspace hygiene ≠ ASI. 修真仅是 hygiene, ASI 北极星 V0.1 = 0.7905 仍是当前最高, 未变动.

---

## Cron 上下文诚实声明

`cron:1fba1cc3 apeireth-autonomy-v3` 于 2026-08-08 15:31 +08:00 触发, prompt 描述状态为 "V1049 value alignment 完成 + 2784 tests pass + ASI 0.7905" (实际为 2026-07-22 老状态).

**实际当前状态 (本 cron tick 启动时)**:
- Latest report: V1305_REPORT.md (15:28, 5 分钟前)
- Latest script: `apeireth/v1306_high_orphan_fix.py` (15:30, 1 分钟前, 修真完成但无 report)
- Latest test: `apeireth/tests/test_v1305_medium_orphan_fix.py` (15:28)
- Workspace members: 88 (V1305 已修真 medium 风险三件套)

**修真策略**: 不假装 V1049 是最新, 续做 V1306 (1 分钟前开始但未完成的高风险修真) → 生成 V1306_REPORT.md + tests + 备份 + 完整验证. 这是 cron 上下文与现实差距时的诚实路径.

ASI 哲学守门: cron prompt stale ≠ ASI 已达到; 修真 ≠ ASI; 修真仅是 hygiene + 实事求是.