# V1304 — Orphan Crates Fix #2 (apeireth-sdk-sandbox)

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:25 +08:00 2026-08-08)

**V1303 修真规划执行 #1** — V1304 修真 low 风险 #1: `apeireth-sdk-sandbox`.

## 修真内容

**加 `apeireth-sdk-sandbox` 到 workspace members** (V1303 audit 标 low risk 之 1).

### 修改: `Apeireth-rust/Cargo.toml`

在 members 列表末尾加:
```toml
    "crates/apeireth-blueprint-impl",
    # V1304 fix (R-Cycle v2-strategy / V1303 audit low 风险修真): 加 apeireth-sdk-sandbox 到 workspace members.
    # 修真仅 +1 行 (V1303 audit 标 low risk, 单 sub-agent 整合步骤, 0 sub-workspace 块 / 0 version 冲突).
    # 现状: version.workspace = true / edition.workspace = true / deps 用 { workspace = true } 全 OK.
    # 修真后: cargo metadata 应能解析, apeireth-sdk-sandbox 应出现在 members 列表.
    # V1305+ 修真 medium/high risk crates (integration-e2e / integration-r20-stage4 / rate-limiter / sdk-lark / sdk-livekit / sdk-voice).
    "crates/apeireth-sdk-sandbox",
```

无其他 Cargo.toml 修改 — `apeireth-sdk-sandbox/Cargo.toml` 已用 `version.workspace = true` / `edition.workspace = true` / deps `{ workspace = true }`, 完全 ready.

## 真验证 (V1302 audit re-run, cargo metadata 真跑)

| 指标 | V1302 (15:18) | V1304 (15:25) | 变化 |
|---|---|---|---|
| active members 总数 | 84 | **85** | +1 ✓ |
| cargo metadata packages | 84 | **85** | +1 ✓ |
| orphan crates 剩余 | 8 | **7** | -1 ✓ |
| sdk-sandbox in members | False | **True** | ✓ |
| 修真耗时 | — | ~30s | — |

## 剩余 orphan (V1305+ 修真路径)

修真 sdk-sandbox 后剩 7 个:

| crate | V1303 标 | 修真路径 |
|---|---|---|
| `apeireth-integration-e2e` | medium | V1305: 删 sub-workspace 块 + 加 members |
| `apeireth-integration-r20-stage4` | medium | V1305: 删 sub-workspace 块 + 加 members |
| `apeireth-rate-limiter` | medium | V1305: 删 sub-workspace 块 + 加 members |
| `apeireth-sdk-lark` | high | V1306: 改 version "0.1.0" → workspace + 删 sub-workspace 块 |
| `apeireth-sdk-livekit` | high | V1306: 同 sdk-lark |
| `apeireth-sdk-voice` | high | V1306: 同 sdk-lark |
| `apeireth-tauri-stub` | intentional | 不动 (Cargo.toml 注释保留) |

## Popper 假说自检

| ID | 描述 | 观察值 | 阈值 | 结果 |
|----|------|--------|------|------|
| h_v1304_fixed | apeireth-sdk-sandbox 在 workspace members | True | True | ✓ PASS |
| h_cargo_metadata_parses | cargo metadata 解析成功 | True | True | ✓ PASS |
| h_sandbox_in_metadata | apeireth-sdk-sandbox 在 cargo metadata packages | True | True | ✓ PASS |
| h_member_count_increased | workspace members >= 85 | 85 | 85 | ✓ PASS |
| h_orphan_count_decreased | orphan crates <= 7 | 7 | 7 | ✓ PASS |
| h_no_lock_churn | 仅 cargo metadata --no-deps | True | True | ✓ PASS |

**全部 6 假说 PASS。**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

- **不假装 Phenomenal consciousness**: add-to-members ≠ consciousness
- **不假装达到 ASI**: workspace hygiene, ASI 北极星远未达成
- **不假装调整模型 & prompt**: 真生产 = 真文件编辑 + cargo metadata 真验证
- **修真 != ASI**: 加 1 个 member ≠ ASI
- **实事求是**: 修真仅 1 个, 剩 7 个 (6 medium/high + 1 intentional) 诚实标缺, 留 V1305+
- **修真仅当必要**: V1304 是 V1303 规划的低风险项, 无脑修真 = 不修真

## 关键免责声明

- V1304 仅改 Cargo.toml members + 1 行 (无 .rs 改动)
- cargo lock churn 风险: V1304 仅加 1 member (无新 dep), 0 触发 Cargo.lock 大规模更新
- 未跑 cargo build 全量验证 (V1304 仅 cargo metadata 解析)
- V1305+ 修真 medium/high risk crate 需更小心, 单独 cron tick 处理

## 输出文件

- `Apeireth-rust/Cargo.toml` (modified, +5 lines V1304 fix)
- `V1304_REPORT.md` (本文件)

---

_Last update: 2026-08-08 15:25+08, by 楚零 (cron:1fba1cc3 apeireth-autonomy-v3). V1304 orphan crate 修真 #2/8 完成 (sdk-sandbox), 6/6 假说 PASS, VCP 深读 #20._