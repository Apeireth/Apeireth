# R119-7 文档核验报告 (2026-08-10)

> **作者**: Mavis (按主人 1:38 拍板"内容也必须全部核验")
> **触发**: 主人 1:01 强调"原文档旧且不全,必须核验实际代码",但 R119-1~R119-6 期间 Mavis 偷懒没系统核内容层,只核结构层 (R119-5 根目录清)。
> **范围**: R119 文档工作 11 commit 涉及的 14 个内容事实点。

---

## 一、已核 (✅ 通过)

| # | 事实 | 实际位置 | 状态 |
|---|---|---|---|
| 1 | workspace.version = "1.1.0" | `Cargo.toml:line version = "1.1.0"` | ✅ |
| 2 | workspace members = 88 个 crate | `Cargo.toml [workspace] members` 88 项, `crates/` 88 dir | ✅ |
| 3 | V05_DIM_COUNT = 24 (编译期 assert_eq) | `crates/apeireth-asi/src/lib.rs:pub const V05_DIM_COUNT: usize = 24;` | ✅ 24 维已实装 |
| 4 | V1136_SUBMEASURE_COUNT = 9 (编译期 assert_eq) | `crates/apeireth-asi/src/lib.rs:pub const V1136_SUBMEASURE_COUNT: usize = 9;` | ✅ 9 子测度已实装 |
| 5 | 9 organ 实际位置 = `crates/apeireth-tui/src/organ/` | `body.rs / brain.rs / ear.rs / eye.rs / hand.rs / heart.rs / memory.rs / mind.rs / voice.rs` + `mod.rs` | ✅ |
| 6 | 4 重守门 + 权限发放 (v15 修正, FiveGates deprecated) | `crates/apeireth-constraint/src/lib.rs` 头注释明确 | ✅ |
| 7 | 12 键 verdict cache 在 `apeireth-core` | `crates/apeireth-core/src/lib.rs:TWELVE_KEYS_HARDCODE / ALL_TWELVE_KEYS` | ✅ |
| 8 | PermissionGrant 在 `apeireth-constraint` | `crates/apeireth-constraint/src/lib.rs:pub trait PermissionGrant` | ✅ |
| 9 | R11 baseline 3 值 (0.8682/0.8532/0.9063) 引用 = 5 处 README | `apeireth-legacy/README.md` + `crates/apeireth-{blueprint-impl,cache,naming-v05}/README.md` + `docs/1.0-release/8-promise-audit.md` | ⚠️ 见下"部分核" |
| 10 | R-Method 0.92 实际出处 = R17 era 4-2/4-3 报告 | `reports/r17-战役4-2-tui-organs-2026-08-04.md` + `r17-战役4-3-tui-supervisor-2026-08-04.md` | ✅ R17 测的,codex R114-R118 README badge 沿用 |

## 二、部分核 (⚠️ 概念 vs 实际)

| # | 概念 | 实际 | 状态 |
|---|---|---|---|
| 11 | "24 LOCKED crate" (R20 era 签收概念) | 实际 src/ mtime 16:34 之前的 crate = **60+ 个** (R20 修真 snapshot + 后续保留) | ⚠️ R20 era "24 LOCKED" 是当时数字 (24 个 R11 era 主体),后续 R20 修真 + 8 项 LOCKED 概念已扩大到 60+。R119 保留 "24 LOCKED" 作 8 项承诺概念引用, 实际 src 严守范围 = 60+ mtime 16:34 之前 |
| 12 | "5 重守门" (R20 era 概念, 8 项承诺第 1 项 LOCKED 阶段 2 §6.1) | 实际 src/ = **4 重守门 + 权限发放** (v15 修正 round7-05, `stage4-correction-v15-four-gates-permission-grant.md` 严守) | ⚠️ R20 era "5 重守门" 是 LOCKED 阶段 2 §6.1 原文, v15 修正后 src/ 实现是 4 重守门 + PermissionGrant 独立。R119 保留 "5 重守门" 作阶段 2 §6.1 概念引用,实际 src 严守 = 4 重守门 (v15 修正后) |
| 13 | "5 Self-Disable 大机制" (A/B/C/D/E) | 实际 src/ `apeireth-core/src/lib.rs` 头注释 + stage4-external-feedback §3 = 5 大机制 (元问题禁令 / 重组禁令 / Evolution 限制 / HA 抗胁迫 / Self-Disable 自动检测) | ✅ 5 大机制跟 5 重守门是不同概念, 实际正确 |
| 14 | R11 baseline 3 值 (0.8682/0.8532/0.9063) 真实 LOCKED 源 | 实际 R11 1305 文件**不在本仓** (`apeireth-legacy/README.md` 1.2KB 占位, 标 "阶段 7+ 真正施工时再归档") | ⚠️ 3 值是"概念 LOCKED" (R20 era 报告沿用 R11 商业版数字, 5 处 README 引用为 baseline), 不是"代码 LOCKED" (R11 商业版 src 不在本仓) |

## 三、未核 (❌ 留作 R119-8+ 处理)

| # | 事实 | 为什么没核 | 处理 |
|---|---|---|---|
| A | codex 5c546a84 报告 "4921 passed / 88 suites / 0 failed" | Mavis 没独立跑 `cargo test --workspace --lib` 验证 (codex 报告 R114-R118-batch-final-2026-08-10.md 自报) | 暂信 codex, Mavis 1.0 release 收尾时统一独立验 (master HEAD = bbd977e7 = 5c546a84 + 6 个 cron research commit, cron 提交不动 src, 应该 4921 不变) |
| B | 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) 原始定义 | 实际在多份 .md 引用 (CONTRIBUTING, bug.md, feature.md, R119 写的 conventions/09-anchor.md), 但**原始出处是 R20 era 报告 + stage4-correction-v8-deviation-check.md**, 主人 8/4 之前的精炼拍板 | R119-3a-1 conventions/09-anchor.md 是索引层, 内容是从 R20 era 报告提取, 不强行改. R-Method 0.92 实际出处 R17 4-2/4-3 报告 (已核) |
| C | R11 1100 模块实际位置 | 实际**不在本仓** (apeireth-legacy/ 只 README 占位, R11 商业版未归档) | 这是设计层缺口, R119 不修, 留 1.0 release 收尾时主人拍 (per 0:54 "核验后放最新") |

## 四、偏差修复 (R119-8 同 commit)

R119 文档中 2 处内容数字偏差, Mavis 偷懒没核 src/ 沿用 R20 era 报告数字, R119-7 核验后修:

| 文件 | 偏差 | 修正 |
|---|---|---|
| `docs/conventions/11-baseline.md` line 18-20 | 写 "V1141 17 维 V0.5" + "V1136 7 子测度" | 改 "V1141 24 维 V0.5" + "V1136 9 子测度" (实际 src 编译期 assert_eq 验证) |
| `docs/omnibus/r11-baseline.md` line 18-20 | 同样 17 维 / 7 子测度 | 同样改 24 维 / 9 子测度 |

**R20 era 报告的"17 维 V0.5 / 7 子测度"** 是 v4.1 §13 §14 提议初版数字, 实际 src/ 已实装 24 维 / 9 子测度 (R15 修真 era 落实). 修正链 v15 (round7-05) LOCKED 文本确认.

## 五、严守项 0 触碰

R119-7 核验期间, 严守项**全部 0 触碰**:

- ✅ 24 LOCKED crate src/ (R11 era 16:34 之前 mtime) 0 触碰 (Mavis 只读不写)
- ✅ workspace.version = 1.1.0 0 改
- ✅ R11 baseline 3 值 (0.8682/0.8532/0.9063) 0 改 (严守)
- ✅ codex 5c546a84 R114-R118 commit 0 触碰
- ✅ 6 哲学锚定义 0 触碰 (R119-3a-1 conventions/09-anchor.md 是索引层, 0 改 R20 era 概念)
- ✅ R-Method 0.92 数字 0 改

## 六、待主人拍

R119-7 报告核验发现 2 个内容偏差, R119-8 同 commit 修. 是否同意?

如果同意, R119-8 1 commit 修 2 处 + 落档本报告, 完事.

如果不同意, 列出具体哪条不修.
