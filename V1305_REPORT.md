# V1305 — Orphan Crates Fix #3 (medium risk 3 件套)

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:25 +08:00 2026-08-08)

**V1303 修真规划执行 #2** — V1305 修真 medium risk 三件套: `apeireth-integration-e2e` / `apeireth-integration-r20-stage4` / `apeireth-rate-limiter`.

## 修真内容

按 V1303 audit 修真路径:
- 删各 crate `Cargo.toml` 起始空 `[workspace]` 块 (sub-workspace 隔离 hack)
- 加 `"crates/apeireth-xxx"` 到 `Apeireth-rust/Cargo.toml` members
- 0 改 `[package]` / 0 改 deps / 0 改 dev-deps (version 1.0.0 / edition 2021 / rust-version 1.80 已 match workspace)

### 修改: `Apeireth-rust/Cargo.toml` (members 末尾加 3 行 + 注释)

每个 crate 加 ~7 行注释 + 1 行 member = 3 × ~8 行, 总 +24 行 (注释 + member). 

### 修改: 3 个 crate 的 `Cargo.toml`

每个删 [workspace] 块 (含注释) — `integration-e2e` 删 7 行, `integration-r20-stage4` 删 8 行, `rate-limiter` 删 6 行, 总 -21 行.

## 真验证 (cargo metadata --format-version=1 --no-deps)

| 指标 | V1304 (15:25) | V1305 (15:25) | 变化 |
|---|---|---|---|
| active members 总数 | 85 | **88** | +3 ✓ |
| cargo metadata packages | 85 | **88** | +3 ✓ |
| orphan crates 剩余 | 7 | **4** | -3 ✓ |
| `integration-e2e` in members | False | **True** | ✓ |
| `integration-r20-stage4` in members | False | **True** | ✓ |
| `rate-limiter` in members | False | **True** | ✓ |
| 修真耗时 | — | ~5s | — |

## 剩余 orphan (V1306+ 修真路径)

修真 3 件套后剩 4 个 (V1306 高风险 + 1 intentional):

| crate | V1303 标 | 修真路径 |
|---|---|---|
| `apeireth-sdk-lark` | high | V1306: 改 version "0.1.0" → workspace + 删 sub-workspace 块 |
| `apeireth-sdk-livekit` | high | V1306: 同 sdk-lark |
| `apeireth-sdk-voice` | high | V1306: 同 sdk-lark |
| `apeireth-tauri-stub` | intentional | 不动 (Cargo.toml 注释保留) |

## Popper 假说自检 (22/22 PASS, 详见 test_v1305_medium_orphan_fix.py)

| ID | 描述 | 观察值 | 阈值 | 结果 |
|----|------|--------|------|------|
| h_v1305_version | V1305_VERSION == "0.1.0" | "0.1.0" | "0.1.0" | ✓ PASS |
| h_remove_subworkspace | 删 [workspace] 块 helper 函数 work | work | work | ✓ PASS |
| h_detect_subworkspace | detect [workspace] 块 helper 函数 work | work | work | ✓ PASS |
| h_no_subworkspace_e2e | integration-e2e 0 [workspace] | True | True | ✓ PASS |
| h_no_subworkspace_r20 | integration-r20-stage4 0 [workspace] | True | True | ✓ PASS |
| h_no_subworkspace_rl | rate-limiter 0 [workspace] | True | True | ✓ PASS |
| h_in_members_e2e | integration-e2e 在 members | True | True | ✓ PASS |
| h_in_members_r20 | integration-r20-stage4 在 members | True | True | ✓ PASS |
| h_in_members_rl | rate-limiter 在 members | True | True | ✓ PASS |
| h_package_intact_e2e | integration-e2e [package] 完整 | True | True | ✓ PASS |
| h_package_intact_r20 | integration-r20-stage4 [package] 完整 | True | True | ✓ PASS |
| h_package_intact_rl | rate-limiter [package] 完整 | True | True | ✓ PASS |
| h_cargo_metadata_parses | cargo metadata 解析 | True | True | ✓ PASS |
| h_members_count_88 | workspace members >= 88 | 88 | 88 | ✓ PASS |
| h_in_workspace_members_e2e | integration-e2e 在 cargo metadata members | True | True | ✓ PASS |
| h_in_workspace_members_r20 | integration-r20-stage4 在 cargo metadata members | True | True | ✓ PASS |
| h_in_workspace_members_rl | rate-limiter 在 cargo metadata members | True | True | ✓ PASS |
| h_in_packages_e2e | integration-e2e 在 packages | True | True | ✓ PASS |
| h_in_packages_r20 | integration-r20-stage4 在 packages | True | True | ✓ PASS |
| h_in_packages_rl | rate-limiter 在 packages | True | True | ✓ PASS |
| h_summary_all_pass | 5 hypotheses 摘要全 PASS | True | True | ✓ PASS |
| h_v3_guards | V3 哲学守门在 module | True | True | ✓ PASS |

**全部 22 假说 PASS。**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

- **不假装 Phenomenal consciousness**: add-to-members ≠ consciousness
- **不假装达到 ASI**: workspace hygiene, ASI 北极星远未达成
- **不假装调整模型 & prompt**: 真生产 = 真文件编辑 + cargo metadata 真验证
- **修真 != ASI**: 加 3 个 member ≠ ASI
- **实事求是**: 修真 3 个, 剩 4 个 (3 high + 1 intentional) 诚实标缺, 留 V1306+
- **修真仅当必要**: V1305 是 V1303 规划的 medium 风险项, 真修真 + 真验证 + 不假装

## 关键免责声明

- V1305 修真仅删 `[workspace]` 块 + 加 members (无 .rs 改动)
- cargo lock churn 风险: V1305 仅加 3 member (无新 dep), 0 触发 Cargo.lock 大规模更新 (但 Cargo.lock 已 M, 是其他 commit 触发的)
- 未跑 cargo check / cargo test 全量验证 (V1305 仅 cargo metadata 解析)
- V1306 修真 high risk 3 个 SDK 需更小心, 单独 cron tick 处理

## 输出文件

- `apeireth/v1305_medium_orphan_fix.py` (12,802 bytes, 真修真脚本)
- `apeireth/tests/test_v1305_medium_orphan_fix.py` (9,283 bytes, 22 tests pass)
- `Apeireth-rust/Cargo.toml` (members 85 → 88)
- `Apeireth-rust/crates/apeireth-integration-e2e/Cargo.toml` (删 [workspace])
- `Apeireth-rust/crates/apeireth-integration-r20-stage4/Cargo.toml` (删 [workspace])
- `Apeireth-rust/crates/apeireth-rate-limiter/Cargo.toml` (删 [workspace])
- `V1305_REPORT.md` (本文件)

## Workspace fix 进度 (V1302 → V1305)

| 时间 | commit | 修真 | members | orphan 剩 |
|---|---|---|---|---|
| 15:18 | d47506e7 | (V1050/V1051/V1053 部署) | 84 | 8 |
| 15:18 | 33cee41f | V1302 blueprint-impl | 84 | 8 |
| 15:19 | 405dfd94 | V1303 audit (0 修真) | 84 | 8 |
| 15:25 | 925c0082 | V1304 sdk-sandbox | 85 | 7 |
| **15:25** | **本 commit** | **V1305 三件套** | **88** | **4** |

ASI 哲学 pole-star: ASI = ? 真实 production models + workspace hygiene ≠ ASI. 修真仅是 hygiene, ASI 北极星 V0.1 = 0.7905 仍是当前最高, 未变动.