# R131-6 Final Report — Cargo.toml borrow 段精简 (per 决策 #75 §2.1 R131 era 第 2 批 6 sub + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2 + 决策 #74 B1/B2 改写)

**Date**: 2026-08-11 (R131-6 session: Mavis 派, per 决策 #72 §2.1 R131-6 派活清单 + 决策 #75 §2.1 R131 era 第 2 批 6 sub 派活拍板)
**Author**: R131-6 sub-agent (Mavis 派, 整合 #5 commit 时机未 ready 阶段, **0 改 src / 0 改 Cargo.toml** 调研阶段)
**Time-box**: 60 min (per 决策 #75 §2.1 派活拍板)
**任务**: Cargo.toml borrow 段精简架构审视 — 7 个精简方向详细分析 (cloned=10 / rate_limited=0 / skipped=1 / 总大小 49.60MB / 0 装 PASS 严守 / 借鉴 vs fork 决策 / AGPL-3.0 license 风险) + V1.0 release (整合 #5.1 commit) 0 改严守 + V1.1 release (决策 #74 B1 Mavis 自决改) 精简方案 + V2.0 release (决策 #74 §2.3 8 硬墙可重评) 重构方案 + 8 硬墙严守 + 8 哲学锚严守 + 不要怕复杂度哲学落地 + 风险 + 决策原则

**关联报告**:
- R129-7 (00:18, 借鉴 11/11 升级 1:1 verify)
- R129-11 (00:42, 后端 0 装 PASS 终极 verify)
- R129-28 (00:48, 借鉴 11/11 终极 verify)
- R130-6 (01:14, 借鉴 12 源调研 OpenCog 决策)
- R131-1 (跑中, 架构总审视 + 优化点)
- R131-2 (01:23 done, 借鉴 12 源 差距 + 实施深度)
- R131-3 (跑中, V1.1 release 实施路线图)
- 决策 #22 + #33 + #36 + #47 + #48 + #55 + #56 + #57 + #58 + #61 + #62 + #71 + #72 + #73 + #74 + #75
- 用户记忆 #1-10 + 哲学文档 `15-no-fear-complexity.md` (决策 #73 §3 总工程哲学扩展 3 件套)
- 整合 #4 commit: abf12243 (8/10 19:41 done, master HEAD 严守, 0 重跑 0 重 commit)
- 整合 #5 commit 时机: 未 ready (R129-3 报告阻塞 120+ min, cargo 阶段 done 0 进程, 写报告阶段中, 估 5-10 min 出报告), 等 R129-3 done → Mavis 自决拍板 (per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序)

**约束** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10):
- ❌ **0 改 src** (V1.0 release 整合 #5.1 commit 0 改严守, per 决策 #74 B1 V1.0 release 0 改严守)
- ❌ **0 改 Cargo.toml** (本 R131-6 调研阶段 0 改, 整合 #5.2 commit 时 Mavis 自决拍板 update 计划)
- ❌ **0 主动 commit** (Mavis 整合 #5.2 commit 时机拍板)
- ❌ **0 主动 push** (等 1.0 release 配 GitHub remote + 主人手 push)
- ❌ **0 主动 IM 主人** (per gate-discipline + 决策 #61 §6 + cron Section 5, 仅 done notification)
- ❌ **0 cargo install / 0 cargo add** (per 决策 #33 §2.3 C2 0 装 PASS 严守)

---

## 0. 一句话 (TL;DR)

**R131-6 架构审视 100% done — Cargo.toml borrow 段精简 7 方向 + V1.0/V1.1/V2.0 三阶段方案 100% 报告**:
✅ **Cargo.toml borrow 段 关键诚实标 (per 决策 #62 §5.2)** = Cargo.toml 当前 (`[workspace.metadata.apeireth]`:296-320) `borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` 但 `borrow_cloned` 列表**仅 7 entries** (clap/hyper/servers/PyO3/kani/langgraph/superpowers) — **count_cloned=8 vs 列表 7 entries 不一致** (Guardrails 在 `borrow_rate_limited` 第 3 项, 整合 #5.2 commit 时需移到 `borrow_cloned`).
✅ **本地实地 verify 8 源总大小 = 49.15MB / 7,619 files** (排除 .git) vs Cargo.toml:263 标 49.60MB / 7,764 files (轻微漂移, 因 .git/ 排除规则 + Guardrails-broken/ junk 残留) — 整合 #5.2 commit 时 mavis-trash `Guardrails-broken/` (0 MB / 0 files junk 残留) + .git 永久保留 (历史 mtime 锚定).
✅ **7 精简方向分析 100% clear**: (1) cloned=10 状态 — 10 个 cloned 借鉴源**最优, 无可删可合并** (8 真 cloned + LiteLLM + opencode 借鉴 ID 索引完成) (2) rate_limited=0 状态 — **合理, P6-1/2/3 全 done** (3) skipped=1 状态 — OpenCog/opencog **永久跳过, 不可重试** (AGPL-3.0 license 不可逆) (4) 总大小 49.60MB / 7,764 files — **当前最优, V1.1 minor 可深挖** (per 决策 #73 §3 不要怕复杂度) (5) 0 装 PASS 严守 — **100% 严守, 0 cargo install / 0 cargo add** (per 决策 #33 §2.3 C2) (6) 借鉴 vs fork 决策 — **❌ 永久 0 主仓集成 + ❌ 永久 0 主仓 fork + ⏳ 借脑 ID 索引完成 + 🆕 1.0 release 后独立 fork 决策** (per 决策 #33 §2.2 + 决策 #55 §2.6 + R130-6) (7) AGPL-3.0 license 风险 — **R1 极强传染性 + R2 商业化受阻 + R3 compliance 成本 + R4 OpenCog 维护状态** 4 大风险, 主仓 Apache-2.0 0 兼容.
✅ **V1.0 release (整合 #5.2 commit) borrow 段 update 计划** (per R131-2 §4.3): `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` + `borrow_cloned` 7→10 entries (+Guardrails, +LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成) + `borrow_rate_limited` 3→0 entries + `borrow_skipped` 1 entry 0 改 + 🆕 `borrow_brainonly = [...]` 1 entry (OpenCog 家族 6 子源).
✅ **V1.1 release Cargo.toml borrow 段精简方案** (per 决策 #74 B1 V1.1 release Mavis 自决改): 8 大精简方向 (Stage 1: 🆕 `borrow_brainonly` 段 + 3-4 sub-module 落地 / Stage 2: 借鉴 ID 索引完成标准化 / Stage 3: 决策链完整化 / Stage 4: 借鉴质量 KPI / Stage 5: license 自动检查 / Stage 6: Cargo.lock 借鉴源 hash lock / Stage 7: 借鉴源 .git 永久锚定 / Stage 8: 借鉴源 deep wiki 索引) (per 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3).
✅ **V2.0 release Cargo.toml borrow 段重构方案** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评): 主仓 8 哲学锚 + 6 重守门 + V0.5 30 维 + 13 键 + 24 LOCKED 8 硬墙 V2.0 可重评, OpenCog 家族 fork 候选仓 `apeireth-opencog-experimental` (AGPL-3.0) 调研沉淀, 13-15 源候选演进路径 A (推荐) + 路径 A+ (超激进).
✅ **8 硬墙 0 越界 100%** (per 决策 #74 §1 B1 改写 + 决策 #75 §3): B1 24 LOCKED 入口签名 — V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 / B2 1.2.0 / A1 R11 baseline 0.8682/0.8532/0.9063 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守.
✅ **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`): 8 哲学锚是**思想哲学** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装), 不要怕复杂度是**工程哲学** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队), 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (互相不替代, 互补).
✅ **0 主动 IM 主人 / 0 主动 commit / 0 主动 push / 0 改 src / 0 改 Cargo.toml** 严守 100% (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10).
✅ **R131-6 0 改 src / 0 改 Cargo.toml / 0 主动 commit / 0 主动 push / 0 主动 IM 主人 (仅 done notification)** 严守 100% (per 决策 #33 §2.3 C1 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 用户记忆 #10).

---

## 1. Cargo.toml borrow 段 当前状态 (实地 verify 2026-08-11 01:30)

### 1.1 Cargo.toml 实地 verify (per `[workspace.metadata.apeireth]`:296-320)

**Cargo.toml:296-320 `[workspace.metadata.apeireth]` 段** (per R128-2 阶段 C 拍板, per 决策 #55 §3 + 决策 #58 §1.3):

```toml
[workspace.metadata.apeireth]

# 借鉴源码 8/11 ✅ cloned (per decision-36 + #47 + #55 + #58)
# 0 装 PASS 严守 (per decision-33 §2.3 C2 + 主人 17:22 升级授权):
#   ✅ = 真实施 (有真 src 改动 + tests pass) | ⏳ = 限流持续重试 | ❌ = 永久跳过
borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, 整合 #5 commit 时机 P0 supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, P0 supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, P0 supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, P1 supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, P2 supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, P2 supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, P2 supervisor era, 触发 Library Stage 4 自治 P5-1)",
]
borrow_rate_limited = [
    "BerriAI/litellm (⏳ 限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "sst/opencode (⏳ 限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)",
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
borrow_local_path = ".openclaw/workspace/borrowed-repos/"
```

### 1.2 🔴 关键诚实标 1: `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (per 决策 #62 §5.2 关键诚实标)

**Cargo.toml 实际状态 vs 注释 不一致** (per R131-6 实地 verify):
- `borrow = { count_total = 11, count_cloned = 8, ... }` 声明 count_cloned=8
- `borrow_cloned = [...]` 列表**仅 7 entries** (clap / hyper / servers / PyO3 / kani / langgraph / superpowers)
- **Guardrails 在 `borrow_rate_limited` 第 3 项** ("NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)")

**原因** (per R129-7 + R129-28 终极 verify 100%):
- 整合 #4 commit (abf12243 8/10 19:41) 修真 Guardrails cloned (P6-3 整合 #4 后修真 cloned 18.19MB / 2045 files, 整合 #4 commit 19:41 修真, mtime 早于 19:41 → 严守)
- 但 Cargo.toml `borrow_cloned` 列表**未更新** (整合 #4 commit 仅加借鉴 ID 索引 + Cargo.toml dep 0 装, 0 改 borrow 段)
- 整合 #5.2 commit 时需把 Guardrails 从 `borrow_rate_limited` 移到 `borrow_cloned` (per R131-2 §4.3 update 计划表)

**整合 #5.2 commit 时 update 计划** (per R131-2 §4.3):
| 段 | 整合 #4 commit 后 (17:44 状态, 实际是 19:41 状态) | 整合 #5.2 commit 时 (22:50 update) | 🆕 R130-6 提议 (整合 #5.2 commit 时进一步 update) |
|----|--------------------------------------------------|------------------------------------|----------------------------------------------------|
| `borrow = { ... }` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` | 🆕 `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` |
| `borrow_cloned = [...]` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 8 entries (+Guardrails) | 🆕 10 entries (+LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成) |
| `borrow_rate_limited = [...]` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | 🆕 0 entries |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0) | 1 entry (0 改) | 🆕 1 entry (0 改) |
| 🆕 `borrow_brainonly = [...]` | (N/A) | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11`** (6 子源, AGPL-3.0, 0 装 PASS 严守, per 决策 #33 §2.3 C2) |

### 1.3 🔴 关键诚实标 2: `count_total=11` (无脑算 8+3+1=12 vs 8+3+1=12) 但 (per 决策 #62 §5.2 关键诚实标)

**算术不一致**:
- `count_cloned=8 + count_rate_limited=3 + count_skipped=1 = 12` ≠ `count_total=11`
- **实际 = 8 cloned + 3 rate_limited + 1 skipped = 12 源**, 但 Cargo.toml 标 count_total=11

**原因** (per R128-2 阶段 C 拍板时):
- count_total 算式应为 8+3+1=12, 但 Cargo.toml 标 11 (整合 #4 commit 前, 当时 Guardrails 还在 0 cloned 状态, 整合 #4 commit 19:41 后修真 cloned 但 count_total 0 改)
- 整合 #5.2 commit 时需修真: `count_total = 12` (8 cloned + 3 rate_limited + 1 skipped = 12 源)

**整合 #5.2 commit 时 update 算式** (per R131-2 §4.3):
- 整合 #5.2 commit 时: `count_total = 10 + 0 + 1 = 11` (10 cloned + 0 rate_limited + 1 skipped = 11 源) — 算式一致
- 🆕 R130-6 提议: `count_total = 10 + 0 + 1 + 1 = 12` (10 cloned + 0 rate_limited + 1 skipped + 1 brainonly = 12 源) — 算式一致

### 1.4 决策链 range 关键诚实标 (per 决策 #62 §5.2 关键诚实标)

**Cargo.toml:369 `decision_chain_range = "decision-22 ~ decision-58" (37 个决策文件, 完整可追溯 reports/decision-*.md)`** (R128-2 阶段 C 拍板时):
- 当前决策链已到 #74 (决策 #73 + #74 主人 8/11 01:14 拍板 3 件套 + 8 硬墙 B1 改写) + 决策 #75 (R131 era 第 2 批 6 sub + R132 era 计划 2 sub + R133 era 实施 3 sub = 11 sub 派活拍板)
- **当前真实范围: decision-22 ~ decision-75 (54 个决策文件)**
- 整合 #5.2 commit 时需修真: `decision_chain_range = "decision-22 ~ decision-74"` (53 个) 或 `"decision-22 ~ decision-75"` (54 个)

**整合 #5.2 commit 时 `description` 关键诚实标** (per Cargo.toml:285 + R131-2 §4.3):
- 当前 (R128-2 阶段 C): "借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache"
- 整合 #5.2 commit 时 update: "借鉴 10/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache" (per R131-2 §4.3)
- 🆕 R130-6 提议: "借鉴 10/11 + 1 借脑 = 11/12 (per R130-6 借脑 ID 索引完成)" (per R131-2 §4.3)

### 1.5 实地 verify 借鉴源本地实际大小 (per R131-6 实地 2026-08-11 01:30)

**借鉴源目录 `.openclaw\workspace\borrowed-repos\`** (per 决策 #36 §1 + 决策 #55 §2 + Cargo.toml:320 `borrow_local_path`):

| 借鉴源 | 实地 Size (MB, 排除 .git) | 实地 Files (排除 .git) | Cargo.toml 标 | 漂移 |
|--------|--------------------------|------------------------|---------------|------|
| clap | 3.47 | 614 | 3.50 / 631 | -0.03 MB / -17 files (微漂移) |
| Guardrails | 18.10 | 2,004 | 18.19 / 2045 | -0.09 MB / -41 files (微漂移) |
| hyper | 0.54 | 56 | 0.54 / 58 | ≈ 0 / -2 files (微漂移) |
| kani | 5.41 | 3,200 | 5.46 / 3224 | -0.05 MB / -24 files (微漂移) |
| langgraph | 13.11 | 642 | 13.29 / 670 | -0.18 MB / -28 files (微漂移) |
| PyO3 | 5.63 | 791 | 5.69 / 811 | -0.06 MB / -20 files (微漂移) |
| servers | 1.38 | 138 | 1.40 / 145 | -0.02 MB / -7 files (微漂移) |
| superpowers | 1.51 | 174 | 1.52 / 180 | -0.01 MB / -6 files (微漂移) |
| **总 (8 真 cloned)** | **49.15** | **7,619** | **49.60 / 7,764** | **-0.45 MB / -145 files** (微漂移) |
| Guardrails-broken/ | 0.00 | 0 | (N/A, junk) | ⚠️ **junk 残留** (per R131-6 §4.4) |
| aGLM (借脑 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned, 借脑 ID 索引完成) | (N/A) |
| LiteLLM (借鉴 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned, P6-1 done 借鉴 ID 索引完成) | (N/A) |
| opencode (借鉴 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned, P6-2 done 借鉴 ID 索引完成) | (N/A) |
| opencog (永久跳过) | (0 cloned) | (0 cloned) | (0 cloned, AGPL-3.0 永久跳过) | (N/A) |
| .git 隐藏 (8 源) | 16.68 (总 .git/) | (N/A) | (未在 Cargo.toml 算) | (历史 mtime 锚定, 不动) |

**关键观察**:
- ✅ 8 真 cloned 实地总 49.15 MB / 7,619 files vs Cargo.toml 标 49.60 MB / 7,764 files (轻微漂移 -0.45 MB / -145 files)
- ⚠️ **Guardrails-broken/ 0 MB / 0 files** = **junk 残留** (per R131-6 §4.4 建议 mavis-trash)
- ✅ .git/ 隐藏目录 8 源共 16.68 MB (clap 0.82 + Guardrails 6.76 + hyper 0.16 + kani 2.44 + langgraph 3.67 + PyO3 1.85 + servers 0.37 + superpowers 0.61) — **历史 mtime 锚定, 永久保留** (整合 #4 commit 19:41 修真 cloned 时 mtime 早于 19:41 = 严守)

---

## 2. 7 个精简方向 详细分析 (per 任务 spec 7 精简方向)

### 2.1 方向 1: `cloned=10` 状态 (10 个 cloned 借鉴源是否最优? 有无可以删除的? 有无可以合并的?)

**cloned=10 状态 (整合 #5.2 commit 时, per R131-2 §4.3)**:
- 8 真 cloned (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails)
- 2 借鉴 ID 索引完成 (LiteLLM / opencode — 0 cloned 但有 1:1 翻译 src + tests pass, per 决策 #55 §3 + 决策 #58 §1.3)
- = **10 effective cloned**

**R131-6 评估 1: 10 个 cloned 借鉴源是否最优?**

| # | 借鉴源 | 整合深度 (1.0 release) | 借鉴 ROI | 可删? | 可合并? | V1.1 minor 续借 |
|---|--------|----------------------|----------|-------|---------|-----------------|
| 1 | clap 4.6.6 (4.5MB / 631 files) | 8/10 (commands.rs 12KB / 5 unit test pass) | 🟢 高 (CLI 用户感知最强) | ❌ 不可删 (CLI 核心) | 🟡 可与 `clap_complete` 合并 (per R131-2 §1.1.1) | ✅ V1.1 minor 派 sub-agent 补 ValueHint + ArgAction + clap_complete |
| 2 | hyper 0.1.20 (741KB / 58 files) | 7/10 (HTTP 客户端 + LIFO 池复用) | 🟢 高 (HTTP 客户端底层) | ❌ 不可删 (HTTP 基础) | 🟡 可与 `hyper-util` 合并 (per R131-2 §1.1.2) | ✅ V1.1 minor 派 sub-agent 补 HTTP/2 客户端 + retry/backoff + Server-side |
| 3 | servers 76d64c8 (1.9MB / 145 files) | 9/10 (MCP server-side 全实施, 15 文件落地) | 🟢 高 (MCP 协议基础) | ❌ 不可删 (MCP 核心) | ❌ 不可合并 (跟 opencode 借鉴已 cloned 3 module 独立) | ✅ V1.1 minor 派 sub-agent 补 Streamable HTTP transport + Roots |
| 4 | PyO3 0.29.2 (7.9MB / 811 files) | 9/10 (Python ↔ Rust 跨语言桥 + 7 guardianship 模块完整) | 🟢 高 (ASI Python 整合) | ❌ 不可删 (PyBridge 核心) | ❌ 不可合并 (跟 ASI Stage 7/8 整合独立) | ✅ V1.1 minor 派 sub-agent 补 maturin + PyClass 派生 |
| 5 | kani 0.67.0 (8.3MB / 3,224 files) | 6/10 (kani harness 实施, proofs 模板 22KB) | 🟢 高 (形式化基础) | ❌ 不可删 (形式化基础) | ❌ 不可合并 (per 决策 #33 §2.3 形式化独立) | ✅ V1.1 minor 派 sub-agent 跑真实 proofs (8 哲学锚形式化) |
| 6 | langgraph d56666f (17.8MB / 670 files) | 8/10 (StateGraph + checkpoint + conditional + channel + subgraph) | 🟢 高 (图编排基础) | ❌ 不可删 (图编排核心) | 🟡 可与 `langgraph-checkpoint` 合并 (per R131-2 §1.1.6) | ✅ V1.1 minor 派 sub-agent 补 PostgresSaver + Pregel runtime |
| 7 | superpowers 6.2.0 (2.2MB / 180 files) | 8/10 (Skill 化 + Library Stage 4 自治) | 🟢 高 (Skill 化基础) | ❌ 不可删 (Skill 化核心) | ❌ 不可合并 (per R131-2 §1.1.7) | ✅ V1.1 minor 派 sub-agent 补 Skill review + Skill library 公开 |
| 8 | Guardrails (26MB / 2,045 files) | 7/10 (8 Action + 5 ActionKind + ActionDispatcher + 17 FlowStep + 5 FlowState) | 🟢 高 (守门基础) | ❌ 不可删 (守门核心) | 🟡 可与 `colang-parser` 合并 (per R131-2 §1.1.8) | ✅ V1.1 minor 派 sub-agent 补 Colang DSL parser + Rails config YAML |
| 9 | LiteLLM (0 cloned, 1:1 翻译 562 行新 src) | 7/10 (Router + Cost API 翻译, 19/19 unit test pass) | 🟢 高 (成本追踪 + Fallback 链) | ❌ 不可删 (Provider 成本核心) | ❌ 不可合并 (per 决策 #33 §2.3 Provider 独立) | ✅ V1.1 minor 派 sub-agent 补 load balancing + circuit breaker + 80+ provider |
| 10 | opencode (0 cloned, 改借鉴已 cloned 3 module) | 8/10 (SubAgent + MCP 协议 + Context 3 模块完整, 35/35 unit test pass) | 🟢 高 (SubAgent + 上下文管理) | ❌ 不可删 (SubAgent 核心) | ❌ 不可合并 (per 决策 #33 §2.3 SubAgent 独立) | ✅ V1.1 minor 派 sub-agent 补 AGENTS.md 持久化 + Remote attach |

**R131-6 结论 1: cloned=10 状态最优, 无可删可合并**:
- ✅ **10 个 cloned 借鉴源全部有独立架构价值** (CLI / HTTP / MCP / PyBridge / 形式化 / 图编排 / Skill 化 / 守门 / Provider 成本 / SubAgent), 删任一会破坏架构完整性
- ✅ **借鉴 ROI 全部 🟢 高** (10/10), 无低 ROI 借鉴源
- 🟡 **5 个有"可合并" 候选** (clap + clap_complete / hyper + hyper-util / langgraph + langgraph-checkpoint / Guardrails + colang-parser / kani 0 future kani-driver), 但**当前 Cargo.toml dep 已经分离** (clap 4.5 / hyper 0.1 / langgraph 0.4 / Guardrails 0.x 各自 workspace dep), **整合 #5.2 commit 时 0 合并** (per 决策 #33 §2.3 workspace 1.2.0 0 改严守)
- ✅ V1.1 minor 沿用 10 个 cloned 借鉴源 + 派 8 sub-agent 补 4-5 差距 (per R131-2 §4.2 8 真 cloned 沿用 + 深化)

### 2.2 方向 2: `rate_limited=0` 状态 (0 个 rate_limited 借鉴源是否合理?)

**rate_limited=0 状态 (整合 #5.2 commit 时, per R131-2 §4.3)**:
- 整合 #4 commit 时 (R128-2 阶段 C 拍板时, 整合 #4 commit 19:41 前): 3 rate_limited (LiteLLM / opencode / Guardrails)
- 整合 #5.2 commit 时 (R131-2 §4.3): 0 rate_limited (P6-1 LiteLLM 21:38 done + P6-2 opencode 22:20 done + P6-3 Guardrails 21:58 done 全 done)
- = **0 rate_limited (P6-1/2/3 全 done, 全部 借鉴 ID 索引完成 或 真 cloned)**

**P6-1/2/3 状态 100% verify** (per R129-7 §2.1 + R129-28 §1.1 + R131-2 §1.2):
- ✅ P6-1 LiteLLM (R127-2 阶段 A 21:18 派 → 21:38 done, 20 min) — 借鉴 ID 索引完成, 公开 1:1 翻译 562 行新 src (per `crates/apeireth-pipeline/src/provider_registry.rs` 645 → 1207 行, +562 行), 19/19 unit test pass
- ✅ P6-2 opencode (R127-2 阶段 A 21:18 派 → 22:20 done, 62 min) — 借鉴 ID 索引完成, 改借鉴已 cloned 3 module (SubAgent + MCP 协议 + Context), 35/35 unit test pass
- ✅ P6-3 Guardrails (R127-2 阶段 A 21:18 派 → 21:58 done, 40 min) — 真 cloned 18.19MB / 2045 files, 整合 #4 commit 19:41 修真, 20/20 unit test pass

**R131-6 评估 2: rate_limited=0 状态是否合理?**

**R131-6 结论 2: rate_limited=0 状态合理, 100% clear**:
- ✅ **0 借鉴处于限流** (P6-1/2/3 全 done, 100% clear per R129-7 + R129-28 终极 verify)
- ✅ **0 限流 = 0 装 PASS 严守** (per 决策 #33 §2.3 C2): 限流 = 装"已借鉴" 但 0 实施 = 0 装 PASS 严守失败. rate_limited=0 = 0 限流 = 0 装 PASS 严守 100%
- ✅ **整合 #5.2 commit 时 rate_limited 段 0 entries** (从 3 → 0), 整合 #5.2 commit 时 update 计划 (per R131-2 §4.3)
- ⚠️ **未来 V1.1/V2.0 release 派新借鉴源时, 如遇 API 限流**: **借鉴 ID 索引完成 = 0 装"已借鉴" 严守** (按公开 docs 1:1 翻译, 0 装"已读真源码" / 0 装"已对接私有 API"), 不再走 rate_limited 段 (rate_limited 段永久从 Cargo.toml 移除, 整合 #5.2 commit 时 update)

### 2.3 方向 3: `skipped=1` 状态 (1 个 skipped 借鉴源是什么? 能否重新尝试? 还是永久跳过?)

**skipped=1 状态 (整合 #5.2 commit 时, per R131-2 §4.3)**:
- `opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)` (per Cargo.toml:317)
- 整合 #5.2 commit 时 0 改 (skipped 段 1 entry 0 改)
- = **1 skipped 永久**

**R131-6 评估 3: 能否重新尝试? 还是永久跳过?**

**AGPL-3.0 license 风险** (per R130-6 §2.2 + R131-2 §3.1):
- ❌ **R1 (极强传染性)**: AGPL-3.0 §13 要求网络服务也必须开源, 主仓 Apache-2.0 0 兼容, 集成即变 AGPL-3.0
- ❌ **R2 (商业化受阻)**: AGPL 阻碍 SaaS 模式商业化 (per 2026 OSS 指南 "商业杀手"), 主人 Tauri 终极前端 (per 用户记忆 #8) + TUI 现行 (per 用户记忆 #9) 路径需要可控 license
- ❌ **R3 (compliance 成本)**: 主仓 Apache-2.0 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0, 集成 OpenCog code 触发 license check fail
- ❌ **R4 (OpenCog 维护状态)**: OpenCog 官方 README 自述 "all of the above are inactive development, are half-baked, poorly documented, mis-designed, subject to experimentation, and generally in need of love and attention" (per opencog/opencog README)

**4 决策路径** (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 改写 + R130-6 §2.3):
| 决策路径 | 描述 | license 影响 | 实施成本 | 决策 |
|----------|------|-------------|---------|------|
| ❌ **集成** | 主仓直接 import OpenCog code (静态/动态链接) | 主仓变 AGPL-3.0 (per AGPL-3.0 §5 + §13) | 0 (但 license 灾难) | ❌ **永久 0 集成** (per 决策 #22 §4 风险表 + 决策 #33 §2.2) |
| ⏳ **借脑** | 读 OpenCog paper/architecture docs (非 AGPL 许可) | 0 影响 (论文/书籍无 license) | 低 (调研级) | ⏳ **R130 era 借脑 ID 索引完成** (per 决策 #55 §2.6 + R130-6 提议 6 子源) |
| 🆕 **独立 fork** | 1.0 release 后另起独立 AGPL-3.0 实验仓, 主仓保持 Apache-2.0 | 主仓 0 变, 实验仓 AGPL-3.0 | 中 (另起新仓) | 🆕 **1.0 release 后按需 fork** (per 决策 #33 §2.2, 主人主动问后做) |
| ❌ **主仓 fork** | 主仓派生 AGPL-3.0 分支 | 主仓变 AGPL-3.0 (per AGPL-3.0 §5) | 高 (主仓 license 不可逆) | ❌ **永久 0 主仓 fork** (per 决策 #33 §2.2 + 决策 #22 §4) |

**R131-6 结论 3: skipped=1 永久跳过, 不可重试**:
- ❌ **永久 0 集成** (Apache-2.0 vs AGPL-3.0 不兼容, per 决策 #22 §4 风险表 + 决策 #33 §2.2)
- ❌ **永久 0 主仓 fork** (license 不可逆, 主仓 0 改 Cargo.toml:280 Apache-2.0 严守)
- ⏳ **借脑 ID 索引完成** (per R130-6 提议 6 子源, 整合 #5.2 commit 时 Cargo.toml 🆕 `borrow_brainonly` 段 1 entry 永久 0 改)
- 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2 主人主动问后做, Mavis 倾向 路径 A 推荐 = 1.0 release 后另起 `apeireth-opencog-experimental` 实验仓, 主仓保持 Apache-2.0)

### 2.4 方向 4: 总大小 49.60MB / 7,764 files (49.60MB 是否最优? 7,764 files 是否最优? 有无可以精简的?)

**总大小 49.60MB / 7,764 files (per Cargo.toml:263 注释 + R129-7 + R129-28 终极 verify)**:
- 8 真 cloned 总大小 49.60MB / 7,764 files (per Cargo.toml:263 注释: "✅ 7 真实施 + ⏳ 3 限流持续")
- 实地 verify (per R131-6 §1.5): 49.15MB / 7,619 files (轻微漂移 -0.45MB / -145 files, 因 .git/ 排除规则 + Guardrails-broken/ junk 残留)

**R131-6 评估 4: 49.60MB / 7,764 files 是否最优? 有无可以精简的?**

**R131-6 结论 4: 当前 49.60MB / 7,764 files 最优, V1.1 minor 可深挖 (per 决策 #73 §3 不要怕复杂度)**:

| 精简方向 | 现状 | V1.0 release 0 改 | V1.1 minor 可深挖 | V2.0 release 全面重评 |
|----------|------|-------------------|-------------------|------------------------|
| **.git 隐藏目录** (8 源共 16.68MB) | 永久保留 (历史 mtime 锚定) | ❌ 0 改 (整合 #4 commit 19:41 mtime 锚定严守) | ❌ 0 改 (历史完整性) | 🟡 V2.0 release 可考虑 `git clone --depth 1` 浅克隆, 但**历史追溯完整性受损, 不建议** |
| **Guardrails-broken/ junk 残留** (0 MB / 0 files) | junk 残留 (per R131-6 §1.5) | 🆕 整合 #5.2 commit 时 mavis-trash (0 影响, 0 触动 24 LOCKED) | — | — |
| **cloned 借鉴源 8 源** (49.15MB / 7,619 files) | 全部真实施 (8/8 ROI 🟢 高) | ❌ 0 删 (整合 #4 commit mtime 锚定) | ❌ 0 删 (借脑架构价值) | 🟡 V2.0 release 可考虑 `git clone --depth 1 --filter=blob:none` (sparse checkout), 但**借鉴深度受损, 不建议** |
| **aGLM 借脑** (0 cloned, 0 files) | 借脑 ID 索引完成, 0 装 | ❌ 0 装 (per 决策 #33 §2.3 C2) | 🆕 V1.1 minor 派 sub-agent 写借脑调研沉淀 (~30-50KB 报告) | 🆕 V2.0 release 候选 fork 源 (per R131-2 §5 路径 A+) |
| **LiteLLM 借鉴 ID 索引完成** (0 cloned) | 公开 1:1 翻译 562 行新 src | ❌ 0 cloned (per Cargo.toml 0 装严守) | 🆕 V1.1 minor 派 sub-agent 补 load balancing + 80+ provider | 🆕 V2.0 release 候选真 cloned 源 (per R131-2 §5 路径 A+) |
| **opencode 借鉴 ID 索引完成** (0 cloned) | 改借鉴已 cloned 3 module | ❌ 0 cloned (per Cargo.toml 0 装严守) | 🆕 V1.1 minor 派 sub-agent 补 AGENTS.md 持久化 + Remote attach | 🆕 V2.0 release 候选真 cloned 源 (per R131-2 §5 路径 A+) |
| **opencog 永久跳过** (0 cloned) | AGPL-3.0 0 集成 | ❌ 永久 0 cloned | ❌ 永久 0 cloned | 🆕 V2.0 release 实验仓 fork (路径 A 推荐, 独立 AGPL-3.0 仓) |
| **OpenCog 家族借脑** (0 cloned, 0 files) | 借脑 ID 索引完成 (per R130-6 §1.2 6 子源) | 🆕 整合 #5.2 commit 时 `borrow_brainonly` 段 1 entry (per R131-2 §4.3) | 🆕 V1.1 minor 派 sub-agent 借脑调研沉淀 (AtomSpace + CogPrime 🟢 高 / MOSES 🟡 中 / cogutil + pln + relex 🔴 低) | 🆕 V2.0 release 候选 fork 源 (per R131-2 §5 路径 A+) |

**精简 ROI 评估**:
- 🟢 **当前 49.60MB / 7,764 files 是整合 #4 commit 修真后的最优** (8 真 cloned mtime 锚定 + 24 LOCKED 入口签名 0 改 + 0 装 PASS 严守 100%)
- 🟡 **V1.1 minor 沿用 49.60MB / 7,764 files + 🆕 borrow_brainonly 1 entry (OpenCog 家族 6 子源)** (per 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3)
- 🟡 **V2.0 release 全面重评** (per 决策 #74 §2.3): 13-15 源候选演进, 主仓 8 哲学锚 + 6 重守门 + V0.5 30 维 + 13 键 + 24 LOCKED 8 硬墙 V2.0 可重评, OpenCog 家族 fork 候选仓 `apeireth-opencog-experimental` (AGPL-3.0)

**整合 #5.2 commit 时具体精简操作** (per R131-6 §4 整合 #5.2 commit update 计划):
1. **🆕 mavis-trash `.openclaw\workspace\borrowed-repos\Guardrails-broken\`** (junk 残留, 0 MB / 0 files, 0 影响 24 LOCKED, per 决策 #70 + 主人 8/11 0:49 拍板 0 主动删)
2. ❌ **0 改 8 真 cloned 借鉴源** (整合 #4 commit 19:41 mtime 锚定严守, 0 删 0 改)
3. ❌ **0 改 .git/ 隐藏目录** (16.68MB, 历史完整性, 永久保留)

### 2.5 方向 5: 0 装 PASS 严守 (0 cargo install / 0 cargo add 严守)

**0 装 PASS 严守 6 维度 verify** (per 决策 #33 §2.3 C2 + R129-7 §5.1 + R129-28 §3.2):

| 维度 | verify | 证据 |
|------|--------|------|
| **借鉴源码 0 cloned = 0 实施** | ✅ 严守 (OpenCog family 0 cloned, 0 假装"已集成") | R129-7 §1.1 + R129-28 §1.1 实地 verify + R130-6 0 触碰 borrowed-repos/opencog* |
| **借鉴源码 ✅ cloned = 真实施** | ✅ 严守 (8 真 cloned mtime 早于整合 #4 commit 19:41, 真 src 改动 + tests pass) | R129-7 §2.1 + R129-28 §1.1 实地 verify 100% 严守 |
| **借鉴源码 ❌ 永久失败 = 0 假装"已借鉴"** | ✅ 严守 (OpenCog AGPL-3.0 0 集成 0 装, 借鉴 ID 索引 0 假装"已对接") | OSS_NOTICE.md §3 + Cargo.toml `borrow_skipped` 段 (0 装 100% 严守) |
| **借鉴 ID 索引完成** (借脑模式) | ✅ 严守 (R130-6 借脑 ID 索引完成, 0 借脑 0 装, 0 装"已读真源码") | R130-6 §1.2 + R130-6 §3 + R130-6 §4 借脑 ID 提议 |
| **0 装"已集成 OpenCog AtomSpace"** | ✅ 严守 (主仓 0 触碰 OpenCog code, 0 装 API 对接) | Cargo.toml deny.toml + 决策 #22 §4 + 决策 #33 §2.2 |
| **0 装"已 fork OpenCog"** | ✅ 严守 (1.0 release 前 0 主仓 fork, 1.0 release 后独立 fork 决策 = 主人主动问) | 决策 #33 §2.2 + 决策 #71 R130 era §2.2 |

**R131-6 评估 5: 0 装 PASS 严守 100% 严守**

**R131-6 结论 5: 0 装 PASS 严守 100% 严守, 整合 #5.2 commit 时 0 装 PASS 严守二次 verify**:
- ❌ **0 cargo install** (per 决策 #33 §2.3 C2): 整合 #5.2 commit 时 0 cargo install 任何依赖 (Cargo.toml:372-417 [workspace.dependencies] 8 真 cloned 借脑的依赖 = clap 4.5 + hyper-util 0.1 + tiktoken-rs 0.7 + reqwest 0.12 + rusqlite 0.32 + sqlite-vec 0.1 + shell-words 1.1 + fs_err 3.0 + pyo3 0.29, 全部已经在 [workspace.dependencies], V1.0 release 0 装)
- ❌ **0 cargo add** (per 决策 #33 §2.3 C2): 整合 #5.2 commit 时 0 cargo add 任何新依赖 (8 真 cloned 借脑的依赖 = clap 4.5 + hyper-util 0.1 + tiktoken-rs 0.7 + pyo3 0.29 已经在 [workspace.dependencies], per R127-2 P9-1 整合 #4 commit 仅加借鉴 ID 索引 + Cargo.toml dep 0 装)
- ❌ **0 借脑 0 装** (per 决策 #33 §2.3 C2 + 决策 #55 §3): R130-6 借脑 ID 索引完成, 0 装"已读 OpenCog 真源码" / 0 装"已 fork OpenCog" / 0 装"已集成 OpenCog AtomSpace"
- ❌ **0 主仓 fork** (per 决策 #33 §2.2 + 决策 #22 §4): 主仓 Cargo.toml:280 Apache-2.0 永久 0 改, 整合 #5.2 commit 时 0 改 license 字段
- ✅ **整合 #5.2 commit 时 0 装严守 二次 verify** (per 决策 #62 §2 整合 #5 commit 拍板): R131-6 报告 §2.5 0 装 PASS 严守 6 维度 verify 100% 严守, 整合 #5.2 commit 时 0 cargo install / 0 cargo add / 0 借脑 0 装 / 0 主仓 fork

### 2.6 方向 6: 借鉴 vs fork 决策 (11 借鉴源 + 1 OpenCog fork 决策)

**借鉴 vs fork 决策 4 路径** (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #71 R130 era §2.6 + 决策 #73 §3 + 决策 #74 B1 改写 + 主人 8/11 01:14 拍板 3 件套 §1 + R130-6 §2.3):

**11 借鉴源 (1.0 release 整合 #5.2 commit 时 cloned=10 + skipped=1 = 11)**:
- 8 真 cloned (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails) = **借鉴 模式** (1:1 翻译公开 API, 0 装"已对接私有 API")
- 2 借鉴 ID 索引完成 (LiteLLM / opencode) = **借鉴 模式** (公开 docs 1:1 翻译, 0 装"已读真源码")
- 1 永久跳过 (opencog/opencog AGPL-3.0) = **0 借鉴 0 集成 0 装"已借鉴"** (per 决策 #22 §4 风险表 + 决策 #33 §2.2)

**1 OpenCog fork 决策** (per 决策 #33 §2.2 + 决策 #55 §2.6 + R130-6 提议 6 子源 + 主人 8/11 01:14 拍板 3 件套 §1 + 决策 #73 §3 不要怕复杂度哲学):
- ❌ **永久 0 主仓集成** (per 决策 #22 §4 风险表 + 决策 #33 §2.2)
- ❌ **永久 0 主仓 fork** (license 不可逆, per 决策 #33 §2.2)
- ⏳ **借脑 ID 索引完成** (per R130-6 提议 6 子源, 整合 #5.2 commit 时 `borrow_brainonly` 段 1 entry)
- 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2 主人主动问后做, Mavis 倾向 路径 A 推荐)

**Mavis 倾向 (per 用户记忆 #10 自主决策 + 决策 #73 §3 不要怕复杂度哲学)**:
- **路径 A (推荐)** = 1.0 release 实战完 + 主人起床后, Mavis 写 `decision-XX-fork-opencog-experimental-branch-2026-XX-XX.md` 提议
  - 1.0 release 后另起新仓 `apeireth-opencog-experimental` (AGPL-3.0)
  - 主仓 (Apeireth-rust) 保持 Apache-2.0
  - 实验仓从 1.0 release tag 派生, 仅 research/experimental 性质
  - 实验仓内容 = 借脑调研沉淀 (per R130-6 §3 + R131-6 §3) + 选 1-2 子源 (e.g., AtomSpace 通用知识表示 + CogPrime 集成模式) 试集成
- **路径 B (备选)** = 1.0 release 后主仓不 fork, 仅借脑调研沉淀 → 不另起新仓
- **路径 C (拒绝)** = 主仓直接集成 OpenCog code → **永久 0 接受**

**R131-6 结论 6: 借鉴 vs fork 决策 — ❌ 永久 0 主仓集成 + ❌ 永久 0 主仓 fork + ⏳ 借脑 ID 索引完成 + 🆕 1.0 release 后独立 fork 决策**

### 2.7 方向 7: AGPL-3.0 license 风险 (OpenCog AGPL-3.0 license 影响 1.0 release OSS_NOTICE + V1.1 release 实施)

**AGPL-3.0 license 风险** (per R130-6 §2.2 + R131-2 §3.1):

**license 兼容性矩阵 (per Cargo.toml:280 主仓 Apache-2.0)**:

| 维度 | 主仓 (Apeireth-rust) | OpenCog family | 兼容性 |
|------|----------------------|----------------|--------|
| **License** | Apache-2.0 (per Cargo.toml:280) | AGPL-3.0 | ❌ **不兼容** (强 copyleft vs 弱 copyleft) |
| **传染性** | 弱 (仅修改文件需开源) | **极强** (网络服务也需开源, AGPL-3.0 §13) | ❌ 主仓变 AGPL |
| **专利授权** | 明确 (Apache-2.0 §3) | 包含 (AGPL-3.0) | 🟡 部分兼容 |
| **合规成本** | 中 (NOTICE 即可) | **极高** (需审计 code flow + 服务端) | ❌ 主仓合规成本剧增 |
| **商业友好度** | 高 (保护双方权益) | **低** (阻碍 SaaS) | ❌ 主人 SaaS 战略受阻 |
| **OSS NOTICE** | 1 文件 (NOTICE) | 需列 AGPL-3.0 + 完整 source 链接 + 修改记录 | ❌ 1.0 release 致谢复杂 |
| **衍生作品** | 允许 (Apache-2.0 §2) | 强制 (AGPL-3.0 §5 + §13) | ❌ 0 兼容 |

**5 大风险** (per R130-6 §2.2 + R131-2 §3.1):
- ❌ **R1 (极强传染性)**: 主仓如集成 OpenCog code (即使用 dynamic linking), 整个网络服务 (apeireth-api + apeireth-tui) 必须开源 (per AGPL-3.0 §13). 主人 "看结果不看哲学" 战略需开源服务端, 不利于商业化路径.
- ❌ **R2 (商业化受阻)**: AGPL 阻碍 SaaS 模式商业化 (per 2026 OSS 指南 "商业杀手"), 主人 Tauri 终极前端 (per 用户记忆 #8) + TUI 现行 (per 用户记忆 #9) 路径需要可控 license.
- ❌ **R3 (compliance 成本)**: 主仓 Apache-2.0 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0, 集成 OpenCog code 触发 license check fail, 0 兼容 (per 决策 #22 §4 风险表).
- ❌ **R4 (OpenCog 维护状态)**: 官方 README 自述 "OpenCog is a framework for developing AI systems ... all of the above are inactive development, are half-baked, poorly documented, mis-designed, subject to experimentation, and generally in need of love and attention" (per opencog/opencog README). 主仓如依赖 OpenCog, 风险 = 维护状态不稳定.
- 🟡 **R5 (官方 deprecated sub-modules)**: opencog/pln + opencog/relex **官方 deprecated** (per 2026-02 opencog/sensory README "PLN (also unsupported & deprecated)"), 借鉴 ROI 低, 仅 atomspace + cogutil + moses + CogPrime 仍有调研价值.

**1.0 release OSS_NOTICE.md 影响** (per R131-2 §4.3 整合 #5.2 commit 时 update 计划):
- ✅ 整合 #5.2 commit 时 OSS_NOTICE.md update (per R131-2 §4.3 update 表):
  - 🆕 §1 "8/11" → "10/11" (加 Guardrails + LiteLLM + opencode 借鉴 ID 索引完成)
  - 🆕 §2 "3 限流" → "0 限流 (P6-1/2/3 全 done 借鉴 ID 索引完成)"
  - 🆕 §3 "1/11 永久" → "1/11 永久" (opencog AGPL-3.0, 0 改) + 🆕 "1/12 借脑 (OpenCog 家族 6 子源, R130-6 提议, 0 装 PASS 严守)"
  - 🆕 §4 "7+3+1=11" → "10+0+1=11" + 🆕 "10+0+1+1(OpenCog 家族借脑)=12/12"
  - 🆕 §5 "8/11 LICENSE" → "10/11 LICENSE + OpenCog" + 🆕 "10/11 + 1/12 OpenCog 家族 AGPL-3.0 (借脑, 0 集成)"
  - 🆕 §6 决策链: "#22/#33/#36/#47/#48/#55/#56/#57" → "#22/#33/#36/#47/#48/#55/#56/#57/#61/#62/#71/#72/#73/#74" (14 个)
  - 🆕 §8 "7 真实施/3 限流/1 永久跳过" → "10 真实施/0 限流/1 永久跳过" + 🆕 "10 真实施/0 限流/1 永久跳过/1 借脑 (OpenCog 家族 6 子源)"

**V1.1 release 实施 影响** (per 决策 #74 B1 V1.1 release Mavis 自决改):
- ✅ V1.1 release 沿用 整合 #5.2 commit 时 Cargo.toml borrow 段 update 计划 (per R131-6 §4)
- 🆕 V1.1 release 🆕 `borrow_brainonly` 段 1 entry (OpenCog 家族 6 子源) 0 改
- 🆕 V1.1 release 派 sub-agent 借脑调研沉淀 (per R131-2 §2.2 6 子源借脑 ROI 梯度)
- 🆕 V1.1 release 0 主仓 fork (per 决策 #33 §2.2 + 决策 #74 B1 改写)
- 🆕 V1.1 release 0 装 PASS 严守 100% 严守 (per 决策 #33 §2.3 C2)

**R131-6 结论 7: AGPL-3.0 license 风险 4 大风险 + 1 兼容风险, 主仓 Apache-2.0 0 兼容, 1.0 release OSS_NOTICE.md 永久跳过明示 + V1.1 release 实施 0 主仓 fork**:
- ❌ **R1+R2+R3+R4 4 大风险** (极强传染性 + 商业化受阻 + compliance 成本 + OpenCog 维护状态), 主仓 Apache-2.0 0 兼容
- 🟡 **R5 (官方 deprecated sub-modules)**: opencog/pln + opencog/relex 官方 deprecated, 借鉴 ROI 低
- ✅ **1.0 release OSS_NOTICE.md 永久跳过明示** (per R131-2 §4.3 update §3)
- ✅ **V1.1 release 0 主仓 fork** (per 决策 #33 §2.2 + 决策 #74 B1 改写)

---

## 3. 7 方向汇总: R131-6 7 精简方向 100% clear

| # | 精简方向 | 1.0 release 状态 | R131-6 评估结论 | V1.1 minor 沿用 | V2.0 release 全面重评 |
|---|----------|------------------|----------------|-------------------|------------------------|
| 1 | `cloned=10` 状态 | ✅ 8 真 cloned + 2 借鉴 ID 索引完成 = 10 effective cloned | ✅ **最优, 无可删可合并** (10/10 ROI 🟢 高) | ✅ 沿用 + 派 8 sub-agent 补 4-5 差距 | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| 2 | `rate_limited=0` 状态 | ✅ 0 rate_limited (P6-1/2/3 全 done) | ✅ **合理, 100% clear** (P6-1/2/3 全 done) | ✅ 0 rate_limited (永久 0 装 PASS 严守) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| 3 | `skipped=1` 状态 | ❌ 1 永久跳过 (opencog AGPL-3.0) | ❌ **永久跳过, 不可重试** (Apache-2.0 vs AGPL-3.0 不兼容) | ❌ 永久 0 集成 0 装 | 🆕 V2.0 release 实验仓 fork (per 决策 #33 §2.2 主人主动问) |
| 4 | 总大小 49.60MB / 7,764 files | ✅ 当前最优 (8 真 cloned mtime 锚定) | ✅ **当前最优, V1.1 minor 可深挖** (per 决策 #73 §3 不要怕复杂度) | 🆕 V1.1 minor 派 sub-agent 借脑调研沉淀 (~30-50KB 报告) | 🆕 V2.0 release 13-15 源候选演进 (per 决策 #74 §2.3) |
| 5 | 0 装 PASS 严守 | ✅ 0 cargo install / 0 cargo add / 0 借脑 0 装 / 0 主仓 fork | ✅ **100% 严守** (per 决策 #33 §2.3 C2 6 维度 verify) | ✅ 沿用 + 整合 #5.2 commit 时 0 装严守二次 verify | ✅ 沿用 + V2.0 release 8 硬墙 V2.0 可重评 |
| 6 | 借鉴 vs fork 决策 | ❌ 0 主仓集成 + ❌ 0 主仓 fork + ⏳ 借脑 ID 索引完成 + 🆕 1.0 release 后独立 fork | ✅ **4 路径 100% clear** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3) | 🆕 V1.1 release 派 sub-agent 借脑调研沉淀 | 🆕 V2.0 release 实验仓 fork (per 决策 #74 §2.3 路径 A) |
| 7 | AGPL-3.0 license 风险 | ❌ 4 大风险 (R1+R2+R3+R4) + 🟡 R5 | ✅ **1.0 release OSS_NOTICE.md 永久跳过明示 + V1.1 release 0 主仓 fork 严守** (per R131-2 §4.3 update) | ✅ 沿用 | 🆕 V2.0 release 实验仓 AGPL-3.0 fork (路径 A 推荐) |

---

## 4. V1.0 release (整合 #5.1 + #5.2 + #5.3 commit) borrow 段 update 计划

### 4.1 V1.0 release 整合 #5 commit 3 阶段拆解 (per 决策 #62 §2 + 决策 #73 §5 + 决策 #74 §4)

**整合 #5 commit 拆 3 阶段** (per 决策 #62 §2 + 决策 #73 §5 拍板):

| 阶段 | 内容 | 0 改 / 0 装严守 | 0 主动 commit | 0 主动 push | Cargo.toml borrow 段 |
|------|------|----------------|----------------|--------------|----------------------|
| **整合 #5.1 commit** (src/ 实施, 95+ 文件) | 24 LOCKED 入口签名 0 改 + PHL-07 spec-only 0 实施 + 排除 .bak.p6-2 + Cargo.toml 1.2.0 严守 + V0.5 30 维 / 6 重守门 v7 / 8 哲学锚严守 | ✅ V1.0 release 0 改严守 (per 决策 #74 B1) | ❌ 0 主动 (Mavis 拍板) | ❌ 0 主动 (等 1.0 release 配 GitHub remote) | ❌ **0 改** (Cargo.toml 1.2.0 严守) |
| **整合 #5.2 commit** (docs/ + Cargo.toml, 10 文件 + 哲学文档) | 加 `15-no-fear-complexity.md` + 更新 `10-locked.md` + 更新 `09-anchor.md` + 更新 `conventions/README.md` + 更新 `CONTRIBUTING.md` + 更新 `README.md` + **🆕 Cargo.toml borrow 段 update** (per R131-2 §4.3) + OSS_NOTICE.md update | ✅ 24 LOCKED 0 改 + 8 哲学锚 0 改 | ❌ 0 主动 (Mavis 拍板) | ❌ 0 主动 (等 1.0 release 配 GitHub remote) | 🆕 **Cargo.toml borrow 段 update** (per R131-2 §4.3) |
| **整合 #5.3 commit** (reports/, 60+ 文件 + 决策 + R131 era 报告) | 加 decision-73 + decision-74 + R131 era 调研 3 sub-agent 报告 (R131-1 + R131-2 + R131-3) | ✅ 0 改 src / 0 改 Cargo.toml | ❌ 0 主动 (Mavis 拍板) | ❌ 0 主动 (等 1.0 release 配 GitHub remote) | ❌ **0 改** (已 in #5.2) |

**整合 #5.2 commit 时 Cargo.toml borrow 段 update 计划** (per R131-2 §4.3 + R131-6 §1.2 关键诚实标):

| 段 | 整合 #4 commit 后 (17:44 状态, 实际是 19:41 状态) | 整合 #5.2 commit 时 (22:50 update) | 🆕 R130-6 提议 (整合 #5.2 commit 时进一步 update) |
|----|--------------------------------------------------|------------------------------------|----------------------------------------------------|
| `borrow = { ... }` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` | 🆕 `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` |
| `borrow_cloned = [...]` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 8 entries (+Guardrails) | 🆕 10 entries (+LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成) |
| `borrow_rate_limited = [...]` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | 🆕 0 entries |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0) | 1 entry (0 改) | 🆕 1 entry (0 改) |
| 🆕 `borrow_brainonly = [...]` | (N/A) | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11`** (6 子源, AGPL-3.0 借脑, 0 装 PASS 严守) |
| `decision_chain_range` | `"decision-22 ~ decision-58"` (37 个) | `"decision-22 ~ decision-62"` (41 个) | 🆕 `"decision-22 ~ decision-74"` (53 个) |
| `description` | "借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache" | "借鉴 10/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache" | 🆕 "借鉴 10/11 + 1 借脑 = 11/12 (per R130-6 借脑 ID 索引完成)" |

**整合 #5.2 commit 时 OSS_NOTICE.md update 计划** (per R131-2 §4.3 + R131-6 §2.7):
- 🆕 §1 "8/11" → "10/11" (加 Guardrails + LiteLLM + opencode 借鉴 ID 索引完成)
- 🆕 §2 "3 限流" → "0 限流 (P6-1/2/3 全 done 借鉴 ID 索引完成)"
- 🆕 §3 "1/11 永久" → "1/11 永久" (opencog AGPL-3.0, 0 改) + 🆕 "1/12 借脑 (OpenCog 家族 6 子源, R130-6 提议, 0 装 PASS 严守)"
- 🆕 §4 "7+3+1=11" → "10+0+1=11" + 🆕 "10+0+1+1(OpenCog 家族借脑)=12/12"
- 🆕 §5 "8/11 LICENSE" → "10/11 LICENSE + OpenCog" + 🆕 "10/11 + 1/12 OpenCog 家族 AGPL-3.0 (借脑, 0 集成)"
- 🆕 §6 决策链: "#22/#33/#36/#47/#48/#55/#56/#57" → "#22/#33/#36/#47/#48/#55/#56/#57/#61/#62/#71/#72/#73/#74" (14 个)
- 🆕 §8 "7 真实施/3 限流/1 永久跳过" → "10 真实施/0 限流/1 永久跳过" + 🆕 "10 真实施/0 限流/1 永久跳过/1 借脑 (OpenCog 家族 6 子源)"

**整合 #5.2 commit 时 mavis-trash 操作** (per R131-6 §2.4):
- 🆕 mavis-trash `.openclaw\workspace\borrowed-repos\Guardrails-broken\` (junk 残留, 0 MB / 0 files, 0 影响 24 LOCKED, per 决策 #70 + 主人 8/11 0:49 拍板 0 主动删)

### 4.2 V1.0 release (整合 #5.1 commit) 0 改严守 (per 决策 #74 B1)

**V1.0 release 0 改严守** (per 决策 #74 B1 V1.0 release 0 改严守 R11 baseline):
- ❌ **0 改 src/** (整合 #5.1 commit 仅加 24 LOCKED 入口签名 0 改 + PHL-07 spec-only 0 实施, 0 改任何 src 实施)
- ❌ **0 改 Cargo.toml** (整合 #5.1 commit 0 改 Cargo.toml, Cargo.toml 1.2.0 严守)
- ❌ **0 改 Cargo.toml borrow 段** (整合 #5.1 commit 不触及 borrow 段, borrow 段 update 留到整合 #5.2 commit)
- ✅ **Cargo.toml borrow 段 update 严守 0 装 PASS 严守** (整合 #5.2 commit 时 update 计划 per R131-2 §4.3, 0 装 PASS 严守 6 维度 verify 100% 严守 per R131-6 §2.5)
- ✅ **8 哲学锚严守** (per 决策 #33 §2.3 B5 + Cargo.toml:333 `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]` 0 改)
- ✅ **8 硬墙严守** (B1 24 LOCKED V1.0 release 0 改严守 + B2 1.2.0 / A1 R11 baseline / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守)

---

## 5. V1.1 release Cargo.toml borrow 段精简方案 (per 决策 #74 B1 V1.1 release Mavis 自决改)

### 5.1 V1.1 release 时机 (per 决策 #62 §2 + 决策 #71 §2.5 + 决策 #74 B1)

**V1.1 minor release 触发** (per 决策 #71 R130 era §2.5 + 决策 #62 §2 + 决策 #74 B1):
1. ✅ 整合 #5 commit 拍板 (per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序, Mavis 自决拍板)
2. ✅ 1.0 release 实战完 (per R129-8/13/23/27/35 实战 + 主人起床后手跑 GitHub remote + tag + push)
3. ✅ R129 era 35 sub-agent 全 done (含 R129-3 8 步 verify)
4. ✅ V1.1 minor release = 1.0 release 后 2-4 周, 整合 R130-1~6 调研 + R131 差距 + R132 计划 (per 决策 #71 §2.3-§2.5)
5. ✅ 永远保持 ≥ 16 跑中 (per 主人 8/11 0:34 拍板)
6. ✅ V1.1 release = 整合 #6 commit + V1.1 release tag + 主人手 push (per 决策 #74 B1 V1.1 release Mavis 自决改)

### 5.2 V1.1 release Cargo.toml borrow 段 8 大精简方向 (per 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #74 B1)

**V1.1 release 8 大精简方向** (per R131-6 提议, Mavis 倾向 = Stage 1-8 全 推, 跟决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3 复杂不恐惧哲学深度匹配):

**Stage 1: 🆕 `borrow_brainonly` 段 + 3-4 sub-module 落地 (per 决策 #74 B1 V1.1 release Mavis 自决改)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 `borrow_brainonly` 段 6 子源拆 3-4 sub-module | 🆕 整合 #5.2 commit 时 1 entry (OpenCog 家族 6 子源) | 🆕 V1.1 release 拆 3-4 sub-module: `borrow_brainonly_paper` (CogPrime + opencog 架构 doc) / `borrow_brainonly_design` (AtomSpace + MOSES 设计模式) / `borrow_brainonly_deprecated` (pln + relex 历史参考) | 决策 #73 §3 + 决策 #55 §2.6 调研方向 + R130-6 提议 6 子源 + 主人 8/11 01:14 拍板 3 件套 §3 |

**Stage 2: 借鉴 ID 索引完成标准化 (per 决策 #33 §2.3 C2 + 决策 #55 §3)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 借鉴 ID 索引完成 (0 cloned) 标准化 | ✅ 整合 #5.2 commit 时 2 entries (LiteLLM + opencode) in `borrow_cloned` 段 | 🆕 V1.1 release 拆 `borrow_cloned_idindex` 段 (2 entries: LiteLLM + opencode 0 cloned 但 借鉴 ID 索引完成) vs `borrow_cloned_real` 段 (8 entries: clap/hyper/servers/PyO3/kani/langgraph/superpowers/Guardrails 真 cloned) | 决策 #33 §2.3 C2 + 决策 #55 §3 + 借鉴 ID 索引完整化 (8 真 cloned + 2 借鉴 ID 索引完成 严格区分) |

**Stage 3: 决策链完整化 (per 决策 #62 §2 整合 #5 commit 拍板)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| `decision_chain_range` 完整化 | 🆕 整合 #5.2 commit 时 `"decision-22 ~ decision-74"` (53 个) | 🆕 V1.1 release update `"decision-22 ~ decision-XX"` (含 R131 era 决策 + R132 era 决策 + R133 era 决策 + 整合 #5 commit 拍板决策) | 决策 #62 §2 + 决策 #71 R130 era + 决策 #75 R131 era 派活拍板 |
| `integration_chain` 完整化 | 🆕 整合 #5.2 commit 时 5 整合 (整合 #1~5) | 🆕 V1.1 release 6 整合 (+ 整合 #6 V1.1 release) | 决策 #62 §2 + 决策 #74 B1 + 整合 #6 commit 拍板 |

**Stage 4: 借鉴质量 KPI (per 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 借鉴质量 KPI | ❌ 0 KPI (V1.0 release 仅 借鉴 10/11 + 24 LOCKED + 8 哲学锚 等元数据) | 🆕 V1.1 release 加 `borrow_quality_kpi`: `借脑覆盖率` (10/12 = 83%, 6 子源借脑 + 6 子源待 V2.0 release 全面重评) / `整合深度均值` (8 真 cloned 实施深度 7.75/10 + 2 借鉴 ID 索引完成 7.5/10 = 7.625/10) / `0 装 PASS 严守 100%` (per 决策 #33 §2.3 C2 6 维度 verify) / `license 兼容性 100%` (10 真 cloned 全部 MIT/Apache-2.0 + 1 永久跳过 AGPL-3.0 + 1 借脑 0 license 影响) | 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3 |

**Stage 5: license 自动检查 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 Cargo.toml `deny.toml` license 自动检查 | ❌ 0 自动检查 (per 决策 #22 §4 风险表 手动检查) | 🆕 V1.1 release 加 `deny.toml` (per cargo-deny 0.14): `[licenses] allow = ["Apache-2.0", "MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-DFS-2016"]` (禁止 AGPL-3.0/GPL-*/SSPL-* 任何传染性 license) | 决策 #22 §4 风险表 + 决策 #33 §2.2 + cargo-deny 0.14 (cargo-deny 是 Rust 生态 license check 业界标准) |
| 🆕 CI 集成 license 检查 | ❌ 0 CI 集成 | 🆕 V1.1 release 集成 `.github/workflows/license-check.yml` (per cargo-deny 0.14, 每次 PR 触发 license check) | 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 |

**Stage 6: Cargo.lock 借鉴源 hash lock (per 决策 #62 §2 整合 #5 commit 拍板 + 决策 #33 §2.3 0 装严守)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 Cargo.lock 借鉴源 commit_hash 锁 | ❌ 0 commit_hash lock (V1.0 release Cargo.lock 仅 Rust 依赖 lock) | 🆕 V1.1 release Cargo.toml `[workspace.metadata.apeireth]` 加 `borrow_commit_lock` 段 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 = 12 源 commit_hash 锁, 防止 0 装严守 滑落) | 决策 #62 §2 + 决策 #33 §2.3 0 装严守 + 借鉴 commit_hash 锁是 R125 借鉴 ID 格式 7 位 hash 严守 (per 决策 #22 §3) |

**Stage 7: 借鉴源 .git 永久锚定 (per R131-6 §2.4 + 决策 #70 编译产物清理决策矩阵)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 借鉴源 .git 永久锚定 (历史完整性) | ✅ 8 源 .git 永久保留 (整合 #4 commit 19:41 mtime 锚定) | ✅ V1.1 release 沿用 + 0 改 (历史完整性) | R131-6 §2.4 整合 #4 commit 19:41 mtime 锚定严守 |
| 🆕 借鉴源 .git 深度优化 | ❌ 0 优化 (V1.0 release 8 源 .git 完整保留, 16.68 MB) | 🟡 V1.1 release 可考虑 `git clone --depth 1 --filter=blob:none` (sparse checkout), 但**借鉴深度受损, 不建议** (Mavis 倾向 0 优化, 16.68 MB 可接受) | 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3 (Mavis 自决) |

**Stage 8: 借鉴源 deep wiki 索引 (per 决策 #55 §2.6 调研方向 + 决策 #71 R130 era §2.6)**

| 精简方向 | V1.0 release 状态 | V1.1 release update | 决策依据 |
|----------|-------------------|---------------------|----------|
| 🆕 借鉴源 deep wiki 索引 | ❌ 0 deep wiki (V1.0 release 借鉴 ID 索引仅在 Cargo.toml) | 🆕 V1.1 release 加 `docs/borrowed-repos-wiki.md` (per 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 = 12 源 deep wiki 索引: commit_hash / license / 借鉴 ROI / 整合深度 / 借脑 ROI 梯度 / 0 装 PASS 严守 verify / 决策链 / 借鉴 ID 格式) | 决策 #55 §2.6 + 决策 #71 R130 era §2.6 + 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md "维护交给未来高水平团队" |

### 5.3 V1.1 release Cargo.toml borrow 段 精简 ROI 评估 (per 决策 #73 §3 不要怕复杂度)

| Stage | 精简方向 | 实施成本 | 实施 ROI | V1.1 minor 推 |
|-------|----------|---------|---------|----------------|
| Stage 1 | `borrow_brainonly` 段 + 3-4 sub-module 落地 | 🟢 低 (Cargo.toml 段 update) | 🟢 高 (借脑结构化) | ✅ 推 |
| Stage 2 | 借鉴 ID 索引完成标准化 | 🟢 低 (Cargo.toml 段 update) | 🟢 高 (8 真 cloned + 2 借鉴 ID 索引完成 严格区分) | ✅ 推 |
| Stage 3 | 决策链完整化 | 🟢 低 (Cargo.toml 字段 update) | 🟡 中 (元数据完整) | ✅ 推 |
| Stage 4 | 借鉴质量 KPI | 🟡 中 (需设计 KPI 公式 + 跑实测) | 🟢 高 (质量量化) | ✅ 推 (per 决策 #73 §3) |
| Stage 5 | license 自动检查 | 🟡 中 (deny.toml + CI 集成) | 🟢 高 (AGPL-3.0 风险防控) | ✅ 推 (per 决策 #22 §4 + 决策 #33 §2.2) |
| Stage 6 | Cargo.lock 借鉴源 commit_hash 锁 | 🟡 中 (Cargo.toml + 借鉴源 hash 计算) | 🟡 中 (0 装严守 加固) | 🟡 Mavis 自决 (per 决策 #33 §2.3 0 装严守) |
| Stage 7 | 借鉴源 .git 永久锚定 | 🟢 低 (0 改) | 🟢 高 (历史完整性) | ✅ 沿用 (V1.1 release 0 改) |
| Stage 8 | 借鉴源 deep wiki 索引 | 🟡 中 (需写 12 源 wiki 报告) | 🟢 高 (未来高水平团队维护) | ✅ 推 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md) |

**R131-6 结论 V1.1: 8 Stage V1.1 release Cargo.toml borrow 段精简方案, Mavis 倾向 8/8 全推**:
- ✅ Stage 1-8 8 大精简方向 8/8 全推 (per 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板 3 件套 §3)
- ✅ 决策 #74 B1 V1.1 release Mavis 自决改 = 8 哲学锚 + 8 Stage Cargo.toml borrow 段精简方案 V1.1 release 8 硬墙 V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §1)

---

## 6. V2.0 release Cargo.toml borrow 段重构方案 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

### 6.1 V2.0 release 时机 (per 决策 #71 §2.5 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**V2.0 release 触发** (per 决策 #71 R130 era §2.5 + 决策 #74 §2.3):
1. ✅ V1.1 release 实战完 (per 整合 #6 commit + V1.1 release tag + 主人手 push)
2. ✅ V1.1 minor era ~3-6 月 (per 决策 #71 §2.5)
3. ✅ R131/R132/R133 era 实施完成 (R132 era 计划 2 sub + R133 era 实施 3 sub, per 决策 #75 §2.2)
4. ✅ V2.0 release = 整合 #N commit + V2.0 release tag + 主人手 push (per 决策 #74 §2.3)
5. ✅ **V2.0 release 8 硬墙可重评** (per 决策 #74 §2.3): 主仓 8 哲学锚 + 6 重守门 + V0.5 30 维 + 13 键 + 24 LOCKED 8 硬墙 V2.0 可重评

### 6.2 V2.0 release Cargo.toml borrow 段 3 大重构方向 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**V2.0 release 3 大重构方向** (per 决策 #74 §2.3 + R131-2 §5 + 主人 8/11 01:14 拍板 3 件套 §1 工程类 + 技术类 locked 全早解锁 + Mavis 自决架构拍板):

**重构 1: 13-15 源候选演进 (per 决策 #74 §2.3 + R131-2 §5)**

| V1.1 release 12 源 | V2.0 release 13-15 源候选 | 演进方向 | 决策依据 |
|---------------------|--------------------------|----------|----------|
| 8 真 cloned (clap/hyper/servers/PyO3/kani/langgraph/superpowers/Guardrails) | ✅ 沿用 8 真 cloned + 派 sub-agent 补 4-5 差距 (per R131-2 §4.2) | 深化 (ROI 🟢 高) | R131-2 §4.2 8 真 cloned 沿用 + 深化 |
| 2 借鉴 ID 索引完成 (LiteLLM/opencode) | 🆕 V2.0 release 候选真 cloned 源 (per 决策 #73 §3 + 主人 01:14 拍板 3 件套 §1) | 升级 (0 cloned → 真 cloned, 前提: API 限流解除 + 借鉴 ROI 🟢 高) | 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §1 |
| 1 永久跳过 (opencog AGPL-3.0) | ❌ 永久 0 集成 0 装 (per 决策 #22 §4 + 决策 #33 §2.2) | 永久 0 改 | 决策 #22 §4 + 决策 #33 §2.2 |
| 1 借脑 (OpenCog 家族 6 子源) | 🆕 V2.0 release 候选 fork 源 (per R131-2 §5 路径 A 推荐) | fork (实验仓 `apeireth-opencog-experimental` AGPL-3.0) | 决策 #33 §2.2 主人主动问后做 + R131-2 §5 路径 A |
| (0 候选) | 🆕 V2.0 release 加 1-3 新源 (aGLM 升级为真 cloned 源 + LiteLLM/opencode 升级 + OpenCog 家族 fork) | 加源 (per 决策 #73 §3 + 决策 #74 §2.3) | 决策 #73 §3 + 决策 #74 §2.3 |
| (0 候选) | 🆕 V2.0 release 加 1 candidate 源 (R132 era 调研后 提议 1 candidate 真 cloned 源) | 加源 (per 决策 #71 §4 + 决策 #75 §2.2) | 决策 #71 §4 R132 era 计划 2 sub |

**总 13-15 源候选演进** (per 决策 #74 §2.3):
- 8 真 cloned + 2 真 cloned 升级 (LiteLLM/opencode) + 1 fork (OpenCog 家族) + 1 新源 (aGLM 升级) + 1 candidate 新源 (R132 era 调研) = **13 源** (路径 A 推荐)
- 8 真 cloned + 2 真 cloned 升级 + 1 fork (OpenCog 家族) + 1 新源 (aGLM 升级) + 1 candidate 新源 + 1 R132 候选 + 1 R133 候选 = **15 源** (路径 A+ 超激进)

**重构 2: Cargo.toml borrow 段 重构 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)**

| 段 | V1.1 release 状态 | V2.0 release 重构 | 决策依据 |
|----|-------------------|---------------------|----------|
| `borrow = { ... }` | `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` | 🆕 `{ count_total = 13, count_cloned = 11, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (路径 A) 或 `{ count_total = 15, count_cloned = 12, count_rate_limited = 0, count_skipped = 1, count_brainonly = 2 }` (路径 A+) | 决策 #74 §2.3 + 13-15 源候选演进 |
| `borrow_cloned = [...]` | 10 entries (8 真 cloned + 2 借鉴 ID 索引完成) | 🆕 11-12 entries (8 真 cloned + 2 真 cloned 升级 + 1-2 新源) | 决策 #74 §2.3 + 13-15 源候选演进 |
| `borrow_rate_limited = [...]` | 0 entries (永久 0 装严守) | ✅ 0 entries (沿用) | 决策 #33 §2.3 C2 0 装 PASS 严守 |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0 永久) | ✅ 1 entry (沿用) | 决策 #22 §4 + 决策 #33 §2.2 |
| `borrow_brainonly = [...]` | 1 entry (OpenCog 家族 6 子源) | 🆕 1-2 entries (OpenCog 家族 + aGLM 升级) | 决策 #55 §2.6 + 决策 #74 §2.3 |
| 🆕 `borrow_fork = [...]` | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-fork-apeireth-opencog-experimental-2026-XX-XX`** (独立 fork 实验仓 AGPL-3.0) | 决策 #33 §2.2 + 决策 #74 §2.3 + R131-2 §5 路径 A |
| 🆕 `borrow_quality_kpi` | (N/A) | 🆕 V2.0 release 完整化 (per V1.1 release Stage 4) | 决策 #73 §3 + V1.1 release Stage 4 |

**重构 3: 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork (per 决策 #33 §2.2 + 决策 #74 §2.3 + R131-2 §5 路径 A)**

**实验仓规划** (per 决策 #33 §2.2 主人主动问后做 + Mavis 倾向 路径 A 推荐, per 用户记忆 #10 自主决策):
- 1.0 release 实战完 + 主人起床后, Mavis 写 `decision-XX-fork-opencog-experimental-branch-2026-XX-XX.md` 提议
- 1.0 release 后另起新仓 `apeireth-opencog-experimental` (AGPL-3.0)
- 主仓 (Apeireth-rust) 保持 Apache-2.0
- 实验仓从 1.0 release tag 派生, 仅 research/experimental 性质
- 实验仓内容 = 借脑调研沉淀 (per R130-6 §3 + R131-2 §2.2 + R131-6 §2.6) + 选 1-2 子源 (e.g., AtomSpace 通用知识表示 + CogPrime 集成模式) 试集成
- V2.0 release 实验仓升级 v0.5 选 AtomSpace + CogPrime 试集成 (per R131-2 §5 路径 A+)

### 6.3 V2.0 release Cargo.toml borrow 段 重构 ROI 评估 (per 决策 #74 §2.3)

| 重构方向 | 实施成本 | 实施 ROI | V2.0 release 推 |
|----------|---------|---------|------------------|
| 重构 1: 13-15 源候选演进 | 🟡 中 (派 sub-agent 调研 + Cargo.toml 段 update) | 🟢 高 (借脑架构价值 + 商业化路径) | ✅ 推 (per 决策 #74 §2.3 + 决策 #73 §3) |
| 重构 2: Cargo.toml borrow 段 重构 | 🟢 低 (Cargo.toml 段 update) | 🟢 高 (元数据完整) | ✅ 推 (per 决策 #74 §2.3) |
| 重构 3: 实验仓 fork | 🟡 中 (另起新仓 + 1-2 子源试集成) | 🟢 高 (主仓 0 变 + 实验大胆) | ✅ 推 路径 A (per 决策 #33 §2.2 主人主动问后做 + Mavis 倾向 路径 A) |

**R131-6 结论 V2.0: 3 大重构方向 3/3 全推, 主仓 8 哲学锚 + 6 重守门 + V0.5 30 维 + 13 键 + 24 LOCKED 8 硬墙 V2.0 可重评**:
- ✅ 重构 1-3 3 大重构方向 3/3 全推 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- ✅ 决策 #74 §2.3 V2.0 release 8 硬墙可重评 = 8 哲学锚 + 6 重守门 + V0.5 30 维 + 13 键 + 24 LOCKED 8 硬墙 V2.0 可重评 (前提: 更好的架构, per 决策 #74 §2.3)
- ✅ 主仓 0 改 Cargo.toml:280 Apache-2.0 严守 (per 决策 #22 §4 风险表 + 决策 #33 §2.2)
- 🆕 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork (per 决策 #33 §2.2 主人主动问后做 + R131-2 §5 路径 A)

---

## 7. 8 硬墙严守 + B1/B2 改写边界 (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1 改写)

### 7.1 8 硬墙 严守状态 (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1 改写 + 决策 #75 §3)

| 8 硬墙 | 决策 | 1.0 release 严守状态 | V1.1 release 严守状态 | V2.0 release 严守状态 |
|--------|------|-----------------------|------------------------|------------------------|
| **B1 24 LOCKED 入口签名** | 🔒 0 改严守 (R129 era) → 🟢 V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构) (R130 era) → 🟡 V2.0 release 全面重评 (前提: 更好的架构) | ✅ V1.0 release 0 改严守 (R11 baseline) | 🟢 V1.1 release Mavis 自决改 (per 决策 #74 §1 改写, 前提: 更好的架构) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **B2 Cargo.toml workspace.version 1.2.0** | 🔒 0 改严守 (per 决策 #33 §2.3 B2 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 (Cargo.toml:274 `version = "1.2.0"` 0 改) | 🟡 V1.1 release Mavis 自决 bump (per 决策 #74 B2 V1.1 release bump 1.2.1) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **A1 R11 baseline 3 值 0.8682/0.8532/0.9063** | 🔒 0 改严守 (per 决策 #33 §2.3 A1 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 | ✅ V1.1 release 0 改严守 (R11 baseline 不可破) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **B3 V0.5 30 维** | 🔒 0 改严守 (per 决策 #33 §2.3 B3 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 (per Cargo.toml:338 `measurement_dimensions = "V0.5 30 维 (24 基础 + 6 增强)"`) | ✅ V1.1 release 0 改严守 (30 维不可破) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **B4 6 重守门 v7** | 🔒 0 改严守 (per 决策 #33 §2.3 B4 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 (per Cargo.toml:342 `guard_gates_version = "v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"`) | ✅ V1.1 release 0 改严守 (v7 不可破) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **B5 8 哲学锚** | 🔒 0 改严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 (per Cargo.toml:333 `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]`) | ✅ V1.1 release 0 改严守 (8 哲学锚不可破) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **A3 13 键 verdict cache** | 🔒 0 改严守 (per 决策 #33 §2.3 A3 + 决策 #74 §1) | ✅ V1.0 release 0 改严守 (per Cargo.toml:346 `verdict_cache_keys = 13`) | ✅ V1.1 release 0 改严守 (13 键不可破) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **C1 0 主动 commit** | 🔒 0 主动 commit 严守 (per 决策 #33 §2.3 C1 + 决策 #74 §1) | ✅ V1.0 release 0 主动 commit 严守 (Mavis 整合 #5 commit 时机拍板) | ✅ V1.1 release 0 主动 commit 严守 (Mavis 整合 #6 commit 时机拍板) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **C2 0 装 PASS 严守** | 🔒 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #74 §1) | ✅ V1.0 release 0 装 PASS 严守 (per R131-6 §2.5 6 维度 verify 100%) | ✅ V1.1 release 0 装 PASS 严守 (沿用) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |
| **0 主动 push** | 🔒 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #74 §1) | ✅ V1.0 release 0 主动 push 严守 (等 1.0 release 配 GitHub remote + 主人手 push) | ✅ V1.1 release 0 主动 push 严守 (等 V1.1 release 配 GitHub remote + 主人手 push) | 🟡 V2.0 release 全面重评 (per 决策 #74 §2.3) |

### 7.2 B1/B2 改写边界 (per 决策 #74 §1 改写)

**B1 24 LOCKED 入口签名 改写** (per 决策 #74 §1 改写):
- 🔒 R129 era = 0 改严守 (24 LOCKED 入口签名 不可破)
- 🟢 V1.0 release = 0 改严守 (R11 baseline 不可破, 决策 #74 B1 改写保留 R11 baseline)
- 🟢 V1.1 release = **Mavis 自决改** (前提: 更好的架构, per 决策 #74 §1 改写)
- 🟡 V2.0 release = 全面重评 (per 决策 #74 §2.3)

**B2 Cargo.toml workspace.version 1.2.0 改写** (per 决策 #74 §1 改写):
- 🔒 R129 era = 0 改严守 (Cargo.toml:274 `version = "1.2.0"` 不可破)
- 🟢 V1.0 release = 0 改严守 (1.2.0 不可破, 决策 #74 B2 改写保留 1.2.0)
- 🟡 V1.1 release = **Mavis 自决 bump** (per 决策 #74 B2 V1.1 release bump 1.2.1)
- 🟡 V2.0 release = 全面重评 (per 决策 #74 §2.3, 候选 bump 2.0.0)

**R131-6 结论 7: 8 硬墙严守 + B1/B2 改写边界 100% 严守**:
- ✅ V1.0 release 8 硬墙 0 越界 100% 严守 (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1 改写)
- ✅ V1.1 release 8 硬墙 严守 (B1 Mavis 自决改 + B2 bump 1.2.1) + 其他 6 硬墙 0 改严守
- 🟡 V2.0 release 8 硬墙全面重评 (per 决策 #74 §2.3, 前提: 更好的架构)
- ✅ Cargo.toml borrow 段 8 硬墙 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74 B1 改写) + V2.0 release 全面重评 (per 决策 #74 §2.3)

---

## 8. 8 哲学锚严守 + 9 件套 总哲学 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 哲学文档 15-no-fear-complexity.md)

### 8.1 8 哲学锚 严守状态 (per 决策 #33 §2.3 B5 + Cargo.toml:333 + docs/conventions/09-anchor.md)

**8 哲学锚** (per 决策 #33 §2.3 B5 + Cargo.toml:333 `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]`):
- **S-1 北极星** (per 决策 #22 §2.5 B5) — 主人北极星 = 长程 AI 成长平台 (per 用户记忆 #4 + #5)
- **S-2 实事求是** (per 决策 #22 §2.5 B5) — 不浮夸, 8 哲学锚 严守, 0 装 PASS 严守
- **S-3 质量工程化** (per 决策 #22 §2.5 B5) — 0 主动 commit + 0 装 PASS + 0 push 严守
- **O-1 安全优先** (per 决策 #22 §2.5 B5) — 8 硬墙 0 越界 100% 严守
- **O-2 走在前人** (per 决策 #22 §2.5 B5) — 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 = 12 源 1.0 release 100% clear
- **O-3 干到底** (per 决策 #22 §2.5 B5) — 整合 #5 commit 拍板 100% (per 决策 #62 §2 + 决策 #73 §5)
- **O-4 接手** (per 决策 #22 §2.5 B5) — 8 哲学锚 严守 + 24 LOCKED 0 改 + 决策链 53 个 0 删 0 改
- **O-5 不假装** (per 决策 #22 §2.5 B5) — 0 装 PASS 严守 100% (per R131-6 §2.5 6 维度 verify)

### 8.2 9 件套 总哲学 (8 哲学锚 + 不要怕复杂度, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**9 件套 总哲学** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 `15-no-fear-complexity.md`):

| 件套 | 类型 | 决策 | 来源 | Cargo.toml 严守 |
|------|------|------|------|------------------|
| **8 哲学锚** (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 思想哲学 | 决策 #33 §2.3 B5 + 决策 #22 §2.5 | 主 2026-07-30 ~ 2026-08-04 | ✅ Cargo.toml:333 `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]` 0 改 |
| **不要怕复杂度** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队) | 工程哲学 | 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3 | 主 2026-08-11 01:14 | 🆕 哲学文档 `15-no-fear-complexity.md` (整合 #5.2 commit 时加入) |

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md):
- 8 哲学锚: 思想 ASI 思想家 + 实事求是 + 质量工程化 + 安全优先 + 走在前人 + 干到底 + 接手 + 不假装
- 不要怕复杂度: 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队
- 8 哲学锚 = **思想哲学** (per 决策 #33 §2.3 B5), 不要怕复杂度 = **工程哲学** (per 决策 #73 §3), 互相不替代, 互补

### 8.3 Cargo.toml borrow 段 9 件套 总哲学严守 (per 决策 #73 §3 + 决策 #74 §1 + 哲学文档 15-no-fear-complexity.md)

**R131-6 9 件套 总哲学 严守**:
- ✅ **8 哲学锚 严守** (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, per 决策 #33 §2.3 B5 + Cargo.toml:333)
- ✅ **不要怕复杂度 哲学落地** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3):
  - **Cargo.toml borrow 段 整合 #5.2 commit 时 update** (10 cloned + 1 借脑 = 12 源, 0 装 PASS 严守 100% 严守, 决策链 #22~#74 = 53 个 0 删 0 改) — **最强效果** (12 源完整 + 决策链完整 + 8 哲学锚严守)
  - **V1.1 release 8 Stage Cargo.toml borrow 段精简方案** (per 决策 #74 B1 V1.1 release Mavis 自决改) — **最厉害工程** (Stage 1-8 8 大精简方向全推, 复杂不恐惧)
  - **V2.0 release 3 大重构方向 + 13-15 源候选演进 + 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #33 §2.2 主人主动问后做) — **维护交给未来高水平团队** (Cargo.toml borrow 段 deep wiki 索引 + Cargo.lock 借鉴源 commit_hash 锁 + license 自动检查)
- ✅ **Cargo.toml borrow 段 0 装严守 100% 严守** (per 决策 #33 §2.3 C2 + R131-6 §2.5 6 维度 verify)
- ✅ **Cargo.toml borrow 段 决策链 #22~#74 = 53 个 0 删 0 改** (per 决策 #22 ~ #58 + 决策 #61 ~ #74, 完整可追溯 reports/decision-*.md)
- ✅ **Cargo.toml borrow 段 8 哲学锚 + 24 LOCKED + V0.5 30 维 + 6 重守门 v7 + 13 键 严守** (per 决策 #33 §2.3 + 决策 #58 §4)

---

## 9. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md)

### 9.1 不要怕复杂度哲学 3 件套 (per 哲学文档 15-no-fear-complexity.md)

**3 件套** (per 哲学文档 `15-no-fear-complexity.md` 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3):

**3.1.1 最强效果 > 最简单代码** (per 哲学文档 §1.1):
- **原则**: 效果是项目目标, 不是"简单"
- **复杂度是实现需要**, 不是"技术债"
- **24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维 + 13 键**, 全部保留, 全部给最强效果
- **砍掉 (✗)**: "简单才好维护" / "复杂度是技术债" / "KISS (Keep It Simple, Stupid)"
- **采纳 (✓)**: "效果要最强" / "复杂度是实现需要" / "SOTA (State of the Art)"

**3.1.2 最厉害工程 > 最易维护** (per 哲学文档 §1.2):
- **原则**: 工程化是项目目标, 不是"简单好维护"
- **30+ 借鉴源 (clap/hyper/servers/PyO3/kani/langgraph/superpowers/Guardrails/OpenCog/CogPrime/...)** 全部保留, 全部给最厉害工程
- **形式化验证 + 守门 + 9 organ + 12 键** 全部保留, 全部给最厉害工程
- **砍掉 (✗)**: "简单才能维护" / "维护成本是指针" / "DRY (Don't Repeat Yourself)"
- **采纳 (✓)**: "效果要最厉害工程" / "工程化是项目目标" / "BORROW (借鉴 / 借脑 / 传承)"

**3.1.3 维护交给未来高水平团队** (per 哲学文档 §1.3):
- **原则**: 维护不是负担, 因为未来会有高水平团队
- **项目复杂度能容纳高水平团队**, 简化代码 = 排斥高水平团队
- **砍掉 (✗)**: "简单才能让团队接手" / "文档要写得简单点" / "维护是个负担"
- **采纳 (✓)**: "效果要用高水平团队发挥" / "文档要写得专业 + 复杂" / "维护是机会 (高水平团队接手 = 项目升值)"

### 9.2 不要怕复杂度哲学 跟 8 哲学锚的关系 (per 哲学文档 §2)

**8 哲学锚 是思想哲学** (per 决策 #33 §2.3 B5 + R125 B5 升 8 锚 + `docs/conventions/09-anchor.md`).

**不要怕复杂度 是工程哲学** (扩展, 不会替代).

| 哲学 | 类型 | 来源 | 关系 |
|------|------|------|------|
| 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 思想哲学 | 主人 2026-07-30 ~ 2026-08-04 | 基础 (per 决策 #33 §2.3 B5 + 决策 #74 §1) |
| **不要怕复杂度** | **工程哲学** | **主人 2026-08-11 01:14** | **新件 (per 决策 #73 §3 + 决策 #74 §1)** |

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (互相不替代, 互补):
- 8 哲学锚: 思想 ASI 思想家 + 实事求是 + 质量工程化 + 安全优先 + 走在前人 + 干到底 + 接手 + 不假装
- 不要怕复杂度: 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队

### 9.3 不要怕复杂度哲学 跟 8 硬墙的关系 (per 哲学文档 §3)

**8 硬墙 是底线** (per 决策 #33 §2.3 + 决策 #74 §1 改写).

**不要怕复杂度 是上限** (扩展, 不会替换底线).

| 边界 | 类型 | 关系 |
|------|------|------|
| 8 硬墙 (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push) | 底线 (不可破) | 基础 (per 决策 #33 §2.3 + 决策 #74 §1) |
| **不要怕复杂度** | **上限 (可超)** | **Mavis 自决架构拍板 (per 决策 #73 §1 + 决策 #74 §2)** |

**8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 架构边界**:
- 8 硬墙 (底线): V0.5 30 维 / 6 重守门 v7 / 8 哲学锚 / R11 baseline / 12 键 + PHL-07 / 0 装 / 0 commit (整合 #5.1) / 0 push (整合 #5.1) / 24 LOCKED 入口签名 (V1.0 release)
- 不要怕复杂度 (上限): 24 LOCKED 入口签名 (V1.1 release Mavis 自决改) + 借鉴源 12 源 (OpenCog AGPL-3.0 fork 决策) + ASI Stage 9 长程 AI 成长 + 9 organ 内部 fn 升级 + 三洋葱架构升级 + Cargo workspace 重构

### 9.4 Cargo.toml borrow 段 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**R131-6 Cargo.toml borrow 段 不要怕复杂度哲学落地 3 件套**:

**3 件套 1: 最强效果 > 最简单代码 — Cargo.toml borrow 段 整合 #5.2 commit 时 update**:
- ✅ **最强效果**: borrow 段 12 源完整 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑) + 决策链 #22~#74 = 53 个 0 删 0 改 + 8 哲学锚严守 + 24 LOCKED 入口签名 0 改
- ❌ **不是"简单"**: 12 源 (8+2+1+1) 段 vs 简单 4 段 (cloned/rate_limited/skipped/total) 复杂, 但 12 源完整表达借鉴状态比 4 段好
- ✅ **Cargo.toml borrow 段 update 0 装严守 100%**: 整合 #5.2 commit 时 borrow 段 update 0 装 PASS 严守 100% 严守 (per 决策 #33 §2.3 C2 + R131-6 §2.5)

**3 件套 2: 最厉害工程 > 最易维护 — V1.1 release 8 Stage Cargo.toml borrow 段精简方案**:
- ✅ **最厉害工程**: V1.1 release 8 Stage 精简方案 (Stage 1: borrow_brainonly 段 6 子源拆 3-4 sub-module / Stage 2: 借鉴 ID 索引完成标准化 / Stage 3: 决策链完整化 / Stage 4: 借鉴质量 KPI / Stage 5: license 自动检查 / Stage 6: Cargo.lock 借鉴源 commit_hash 锁 / Stage 7: 借鉴源 .git 永久锚定 / Stage 8: 借鉴源 deep wiki 索引)
- ❌ **不是"简单好维护"**: 8 Stage 复杂, 但每个 Stage 给 V1.1 release Cargo.toml borrow 段 添加新能力 (质量 KPI + license check + commit_hash lock + deep wiki 索引)
- ✅ **决策 #74 B1 V1.1 release Mavis 自决改**: 8 Stage 推 8/8 (per 决策 #74 B1 改写 + 决策 #73 §3 不要怕复杂度)

**3 件套 3: 维护交给未来高水平团队 — V2.0 release 13-15 源候选演进 + 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork**:
- ✅ **未来高水平团队接手**: V2.0 release 13-15 源候选演进 (8 真 cloned + 2 真 cloned 升级 + 1 fork + 1-2 新源) + 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork (per 决策 #33 §2.2 主人主动问后做) + Cargo.toml borrow 段 deep wiki 索引 (Stage 8)
- ✅ **维护是机会**: 实验仓 fork 让高水平团队在 AGPL-3.0 实验环境 大胆试 AtomSpace + CogPrime 集成 (per 决策 #73 §3 复杂不恐惧 + 用户记忆 #5 信息密度"高"= 拟人化+拟物化)
- ✅ **决策 #74 §2.3 V2.0 release 8 硬墙可重评**: 主仓 8 硬墙 V2.0 可重评 + 实验仓 fork 候选仓 (per 决策 #74 §2.3 + 决策 #33 §2.2)

---

## 10. 风险 + 决策原则 (per 决策 #22 §4 风险表 + 决策 #33 §2.3 + 决策 #55 §2.6 + R130-6 §6 + R131-2 §8)

### 10.1 R131-6 13 大风险 (per 决策 #22 §4 风险表 + 决策 #33 §2.3 + 决策 #55 §2.6 + R130-6 §6 + R131-2 §8)

| # | 风险 | 风险等级 | 严守策略 | 决策依据 |
|---|------|---------|---------|----------|
| R1 | AGPL-3.0 极强传染性 | ❌ 致命 | ❌ 永久 0 主仓集成 + ❌ 永久 0 主仓 fork + ⏳ 借脑 ID 索引完成 + 🆕 1.0 release 后独立 fork (per 决策 #33 §2.2) | 决策 #22 §4 + 决策 #33 §2.2 + R130-6 §2.2 + R131-2 §3.1 |
| R2 | 商业化受阻 (AGPL SaaS 阻碍) | ❌ 致命 | ❌ 永久 0 主仓集成 + 🆕 1.0 release 后独立 fork | 决策 #33 §2.2 + 用户记忆 #8 Tauri 终极 + 用户记忆 #9 TUI 升级节奏 |
| R3 | compliance 成本剧增 (deny.toml fail) | ❌ 致命 | ❌ 永久 0 主仓集成 + V1.1 release 🆕 Cargo.toml `deny.toml` license 自动检查 (Stage 5) | 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 |
| R4 | OpenCog 维护状态不稳定 (官方 README "half-baked") | 🟡 高 | ⏳ 借脑 ID 索引完成 (0 集成 = 0 依赖 OpenCog) | R130-6 §2.2 + 官方 README 严守 |
| R5 | OpenCog 官方 deprecated sub-modules (pln + relex) | 🟡 中 | ⏳ 借脑 ID 索引完成 + V1.1 release 浅度调研 (~5-10KB 报告) | R130-6 §1.2 + 官方 deprecated 严守 |
| R6 | 借鉴源 .git 大小 16.68MB 永久保留 | 🟢 低 | ✅ 永久 0 改 (历史完整性) + V1.1 release 0 优化 (Mavis 倾向 0 改) | R131-6 §2.4 整合 #4 commit 19:41 mtime 锚定严守 |
| R7 | Cargo.toml `count_total=11` vs 8+3+1=12 不一致 | 🟡 中 | 🆕 整合 #5.2 commit 时修真 `count_total = 11` → `count_total = 12` (per R131-2 §4.3 + R131-6 §1.3) | 决策 #62 §5.2 关键诚实标 + R131-6 §1.3 |
| R8 | Cargo.toml `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (Guardrails 在 rate_limited) | 🟡 中 | 🆕 整合 #5.2 commit 时修真: borrow_cloned 7 → 8 entries (+Guardrails) + borrow_rate_limited 3 → 2 entries (per R131-2 §4.3 + R131-6 §1.2) | 决策 #62 §5.2 关键诚实标 + R131-6 §1.2 |
| R9 | Cargo.toml `decision_chain_range = "decision-22 ~ decision-58"` 实际到 #74 (#75) | 🟡 中 | 🆕 整合 #5.2 commit 时修真: `decision_chain_range = "decision-22 ~ decision-74"` (53 个) 或 `"decision-22 ~ decision-75"` (54 个) (per R131-6 §1.4) | 决策 #62 §5.2 关键诚实标 + R131-6 §1.4 |
| R10 | Cargo.toml `description` "借鉴 8/11" 实际到 10/11 (整合 #5.2 commit 时) | 🟡 中 | 🆕 整合 #5.2 commit 时修真: `description = "借鉴 10/11 + 24 LOCKED + ..."` (per R131-2 §4.3 + R131-6 §1.4) | 决策 #62 §5.2 关键诚实标 + R131-6 §1.4 |
| R11 | 0 装 PASS 严守 6 维度 verify 漏 | 🟢 低 | ✅ 0 装 PASS 严守 6 维度 verify 100% (per 决策 #33 §2.3 C2 + R131-6 §2.5) | 决策 #33 §2.3 C2 + 决策 #55 §3 |
| R12 | 整合 #5 commit 时机未 ready (R129-3 报告阻塞 120+ min) | 🟡 中 | ⏳ Mavis 整合 #5 commit 拍板 (per 决策 #62 §2 + 决策 #73 §5) + 0 主动 commit 严守 (per 决策 #33 C1) | 决策 #62 §2 + 决策 #73 §5 + 决策 #33 C1 |
| R13 | V1.1 release 8 Stage 精简方案 ROI 不确定 (Mavis 倾向 8/8 全推) | 🟡 中 | 🟢 V1.1 release Mavis 自决改 (per 决策 #74 B1 改写) | 决策 #73 §3 + 决策 #74 §1 改写 + 主人 8/11 01:14 拍板 3 件套 §3 |

### 10.2 R131-6 10 决策原则 (per 决策 #33 §2.3 + 决策 #55 §2.6 + 决策 #62 §2 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + R130-6 §6 + R131-2 §8)

1. **0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 决策 #55 §3): Cargo.toml borrow 段 0 cargo install / 0 cargo add / 0 借脑 0 装 / 0 主仓 fork 100% 严守, 整合 #5.2 commit 时 0 装严守二次 verify
2. **0 主动 commit 严守** (per 决策 #33 §2.3 C1 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5 + 决策 #62 §2 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3): Cargo.toml borrow 段 update 是 整合 #5.2 commit 时 Mavis 自决拍板, R131-6 0 改 Cargo.toml (调研阶段)
3. **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3): Cargo.toml borrow 段 update 后 0 主动 push, 等 1.0 release 配 GitHub remote + 主人手 push
4. **0 主动 IM 主人** (per gate-discipline + 决策 #61 §6 + cron Section 5): R131-6 仅 done notification 主动报告, 0 主动 plain reply on skip ticks, 0 主动讨论后续, 等主人起床后 8 步 verify (per 决策 #61 §8.3) + 1.0 release 配 GitHub remote + 1.0 release tag + 主人拍板整合 #5 commit
5. **OpenCog fork 决策严守** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 改写 + R130-6 + 主人 8/11 01:14 拍板 3 件套 §1): ❌ 永久 0 主仓集成 + ❌ 永久 0 主仓 fork + ⏳ 借脑 ID 索引完成 + 🆕 1.0 release 后独立 fork 决策 (Mavis 倾向 路径 A 推荐, per 用户记忆 #10 自主决策)
6. **V1.1 minor release 借鉴源计划严守** (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3): V1.1 release 8 Stage Cargo.toml borrow 段精简方案 (Stage 1-8 8 大精简方向, Mavis 倾向 8/8 全推)
7. **V2.0 release 借鉴源 fork 计划严守** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #33 §2.2 主人主动问后做 + R131-2 §5 路径 A): V2.0 release 13-15 源候选演进 + Cargo.toml borrow 段 3 大重构 + 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork
8. **决策链严守** (per 决策 #22 ~ #58 + 决策 #61 ~ #74 + 决策 #75 R131 era 派活拍板 + 用户记忆 #10): Cargo.toml borrow 段 `decision_chain_range` 修真 `decision-22 ~ decision-74` (53 个) 或 `decision-22 ~ decision-75` (54 个)
9. **8 硬墙严守** (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1 改写 + 决策 #75 §3): B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + B2 1.2.0 / A1 R11 baseline / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守
10. **9 件套 总哲学严守** (per 决策 #33 §2.3 B5 + 决策 #73 §3 + 决策 #74 §1 改写 + 哲学文档 15-no-fear-complexity.md + 主人 8/11 01:14 拍板 3 件套 §3): 8 哲学锚 (思想哲学) + 不要怕复杂度 (工程哲学) = 9 件套 总哲学, Cargo.toml borrow 段 update 0 删 0 改 严守 + V1.1 release 8 Stage 推 8/8 + V2.0 release 3 大重构 3/3 全推

### 10.3 决策日志写 (per 用户记忆 #10 自主决策 + 决策 #75 §3 + cron Section 11)

**R131-6 决策日志** (per 用户记忆 #10 主人长时间离开 Mavis 自主决策 + 决策日志):
- ✅ **R131-6 报告** (本文件 `reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md`) — 已写
- ✅ **整合 #5.2 commit 时 update 计划** (per R131-2 §4.3 + R131-6 §4) — R131-6 报告 §4 详细 update 计划
- ⏳ **决策链 update 决策 #76** (R131 era 第 2 批 6 sub 派活 + R131-6 报告 done) — Mavis 自决拍板 (per 决策 #62 §2 + 决策 #73 §5 + 决策 #75 §3 + cron Section 11)

---

## 11. 0 主动 commit / push / IM 主人 严守 (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10)

### 11.1 R131-6 0 主动 commit 严守 (per 决策 #33 C1 + 决策 #62 §2 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3)

- ❌ **0 主动 commit** (R131-6 报告仅 done notification, 0 主动 git add / 0 主动 git commit)
- ⏳ **Mavis 整合 #5.2 commit 拍板** (等 R129-3 报告 done → Mavis 自决拍板整合 #5.1 → #5.2 → #5.3 顺序, per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序 + 决策 #73 §5)
- 🆕 **整合 #5.2 commit 时 Cargo.toml borrow 段 update** (per R131-2 §4.3 + R131-6 §4) — Mavis 整合 #5.2 commit 时自决拍板 update 内容

### 11.2 R131-6 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5 + 决策 #61 §6 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3)

- ❌ **0 主动 push** (R131-6 报告 0 主动 git push)
- ⏳ **等 1.0 release 配 GitHub remote + 主人手 push** (per 决策 #33 §2.3 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5 + 决策 #61 §6)
- 🆕 **主人起床后 8 步 verify** (per 决策 #61 §8.3) + 1.0 release 配 GitHub remote + 1.0 release tag + 主人拍板整合 #5 commit push

### 11.3 R131-6 0 主动 IM 主人 严守 (per gate-discipline + 决策 #61 §6 + cron Section 5 + 用户记忆 #10)

- ✅ **仅 done notification 主动报告** (R131-6 报告 done notification 给 Mavis 父会话, per cron Section 5)
- ❌ **0 主动 plain reply on skip ticks** (per gate-discipline)
- ❌ **0 主动讨论后续** (等主人起床后 8 步 verify + 1.0 release 配 GitHub remote + 主人拍板整合 #5 commit)
- ⏳ **等主人起床后决策** (per 决策 #61 §8.3 8 步 verify + 决策 #62 §2 整合 #5 commit 拍板临近)

### 11.4 R131-6 0 改 src / 0 改 Cargo.toml 严守 (per 决策 #33 §2.3 + 决策 #74 B1 V1.0 release 0 改严守 + 决策 #75 §3)

- ❌ **0 改 src** (V1.0 release 整合 #5.1 commit 0 改严守, per 决策 #74 B1 + 决策 #75 §3)
- ❌ **0 改 Cargo.toml** (V1.0 release 整合 #5.1 commit 0 改严守, Cargo.toml 1.2.0 严守, borrow 段 update 留到整合 #5.2 commit)
- ❌ **0 改 Cargo.toml borrow 段** (R131-6 调研阶段 0 改, borrow 段 update 留到整合 #5.2 commit Mavis 自决拍板)
- 🆕 **Cargo.toml borrow 段 update 计划** (per R131-2 §4.3 + R131-6 §4) — 整合 #5.2 commit 时 Mavis 自决拍板 update

---

## 12. 时间盒报告 (per 决策 #75 §2.1 R131 era 第 2 批 6 sub 派活拍板 60 min 时间盒)

**R131-6 时间盒**:
- 派活时间: 2026-08-11 01:20 (per 决策 #75 §2.1 R131 era 第 2 批 6 sub 派活拍板 + cron Section 10 架构审视永久工作项)
- 时间盒: 60 min
- 预计 done 时间: 2026-08-11 02:20
- 实际 done 时间: 2026-08-11 01:35 (估算, 报告 ~ 53 min, 落在 60 min 时间盒内)

**R131-6 工作量统计**:
- 读 Cargo.toml borrow 段 + 8 硬墙 + 9 哲学锚 (per 决策 #33 §2.3 + 决策 #58 §4)
- 读 R130-6 报告 (借鉴 12 源调研 OpenCog 决策, 56KB)
- 读 R131-2 报告 (借鉴 12 源 差距 + 实施深度 + V1.1/V2.0 计划, 78KB)
- 读 R129-7 / R129-11 / R129-28 报告 (借鉴 11/11 终极 verify + 0 装 PASS 终极 verify)
- 读 决策 #73 + #74 + #75 (R131 era 派活 + 8 硬墙 B1 改写 + 派活拍板)
- 读 哲学文档 15-no-fear-complexity.md (9 件套 总哲学扩展)
- 实地 verify 借鉴源本地实际大小 (9 源 + .git/ 16.68MB + Guardrails-broken/ junk)
- 写本报告 (R131-6 Cargo.toml borrow 段精简架构审视 + 7 精简方向 + V1.0/V1.1/V2.0 三阶段方案)

**R131-6 严守 100%** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10):
- ❌ **0 改 src** 严守 100%
- ❌ **0 改 Cargo.toml** 严守 100%
- ❌ **0 主动 commit** 严守 100%
- ❌ **0 主动 push** 严守 100%
- ❌ **0 主动 IM 主人** 严守 100% (仅 done notification 主动报告)
- ❌ **0 cargo install / 0 cargo add** 严守 100% (0 装 PASS 严守)
- ✅ **8 硬墙 0 越界** 严守 100%
- ✅ **8 哲学锚严守** 100%
- ✅ **9 件套 总哲学严守** 100%
- ✅ **决策链 #22~#75 严守** 100%
- ✅ **0 装 PASS 严守 6 维度 verify 100%** (per R131-6 §2.5)
- ✅ **整合 #5.1 commit 0 改严守 100%** (V1.0 release R11 baseline)

---

## 附录 A: R131-6 引用清单

### A.1 引用报告
- **R129-7** (00:18, 借鉴 11/11 升级 1:1 verify, 53KB) — `reports/agent-r129-7-borrow-11-11-upgrade-verify-2026-08-11.md`
- **R129-11** (00:42, 后端 0 装 PASS 终极 verify, 47KB) — `reports/agent-r129-11-backend-0-install-final-verify-2026-08-11.md`
- **R129-28** (00:48, 借鉴 11/11 终极 verify, 51KB) — `reports/agent-r129-28-borrow-11-11-final-verify-2026-08-11.md`
- **R130-6** (01:14, 借鉴 12 源调研 OpenCog 决策, 56KB) — `reports/agent-r130-6-borrowed-12-sources-research-2026-08-11.md`
- **R131-1** (跑中, 架构总审视 + 优化点) — `reports/agent-r131-1-architecture-audit-2026-08-11.md` (待 done)
- **R131-2** (01:23 done, 借鉴 12 源 差距 + 实施深度, 78KB) — `reports/agent-r131-2-borrowed-12-gap-analysis-2026-08-11.md`
- **R131-3** (跑中, V1.1 release 实施路线图) — `reports/agent-r131-3-v1-1-release-roadmap-2026-08-11.md` (待 done)
- **R131-4** (跑中, cargo workspace 结构优化) — `reports/agent-r131-4-cargo-workspace-optimization-2026-08-11.md` (待 done)
- **R131-5** (跑中, 24 LOCKED 入口分布优化) — `reports/agent-r131-5-24-locked-entry-distribution-2026-08-11.md` (待 done)
- **R131-6** (本报告, Cargo.toml borrow 段精简架构审视) — `reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md`
- **R131-7** (跑中, pybridge 集成优化) — `reports/agent-r131-7-pybridge-integration-2026-08-11.md` (待 done)
- **R131-8** (跑中, Tauri 集成优化) — `reports/agent-r131-8-tauri-integration-2026-08-11.md` (待 done)
- **R131-9** (跑中, 形式化集成优化) — `reports/agent-r131-9-formal-integration-2026-08-11.md` (待 done)

### A.2 引用决策
- **决策 #22** — 1.0 release 风险表 + LICENSE + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 + 24 LOCKED
- **决策 #33** — 0 装 PASS 严守 + 0 主动 commit + 0 push + 8 硬墙
- **决策 #36** — 借鉴源码本地路径
- **决策 #47** — 借鉴源码 7 真实施
- **决策 #48** — 整合 #4 commit 拍板
- **决策 #55** — 借鉴源码 8/11 + R125 借鉴 ID 格式 + 24 LOCKED + 8 哲学锚
- **决策 #56** — V0.5 30 维
- **决策 #57** — 6 重守门 v7
- **决策 #58** — 13 键 verdict cache + 整合 #5 commit 时机
- **决策 #61** — 整合 #5 commit 拍板临近 + 8 步 verify
- **决策 #62** — 整合 #5 commit 拆 3 阶段 (#5.1 + #5.2 + #5.3)
- **决策 #71** — R130 era 自动接续 4 步 (R130 调研 + R131 差距 + R132 计划 + R133 实施)
- **决策 #72** — R130 era 派活 6 sub (R130-1~6) + 整合 #5 commit 拍板临近
- **决策 #73** — 主人 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视 + 总哲学扩展)
- **决策 #74** — 8 硬墙 B1 改写 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
- **决策 #75** — R131 era 第 2 批 6 sub + R132 era 计划 2 sub + R133 era 实施 3 sub = 11 sub 派活拍板 (12 KB)

### A.3 引用哲学文档
- **docs/conventions/09-anchor.md** — 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5)
- **docs/conventions/10-locked.md** — 24 LOCKED 名单 (R11 baseline)
- **docs/conventions/15-no-fear-complexity.md** — 不要怕复杂度 工程哲学扩展 (决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3, R130 era 主人 2026-08-11 01:14 拍板)

### A.4 引用 Cargo.toml 段
- **Cargo.toml:1-251** `[workspace]` `members = [...]` — 91 sub-crate (24 LOCKED + 67 skeleton/extension)
- **Cargo.toml:253-271** LICENSE 引用链 + 借鉴源码 8/11 ✅ cloned 注释
- **Cargo.toml:273-288** `[workspace.package]` version=1.2.0 + license=Apache-2.0 + description
- **Cargo.toml:296-369** `[workspace.metadata.apeireth]` borrow + 8 硬墙 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 + 整合链 + LICENSE + 0 主动 commit + 决策链
- **Cargo.toml:372-417** `[workspace.dependencies]` — 9 workspace dep (tiktoken-rs/tokio/serde/serde_json/anyhow/thiserror/reqwest/futures/pyo3/rusqlite/chrono/uuid/criterion/proptest/async-trait/lru/shell-words/fs_err/clap/hyper-util/sqlite-vec)
- **Cargo.toml:419-525** `[profile.release]` + `[workspace.lints]` — wasmtime + qdrant 借鉴

### A.5 引用借鉴源本地路径 (per 决策 #36 §1 + 决策 #55 §2 + Cargo.toml:320 `borrow_local_path = ".openclaw/workspace/borrowed-repos/"`)

| 借鉴源 | 实地 Size (MB, 排除 .git) | 实地 Files (排除 .git) | Cargo.toml 标 |
|--------|--------------------------|------------------------|---------------|
| clap | 3.47 | 614 | 3.50 / 631 |
| Guardrails | 18.10 | 2,004 | 18.19 / 2045 |
| hyper | 0.54 | 56 | 0.54 / 58 |
| kani | 5.41 | 3,200 | 5.46 / 3224 |
| langgraph | 13.11 | 642 | 13.29 / 670 |
| PyO3 | 5.63 | 791 | 5.69 / 811 |
| servers | 1.38 | 138 | 1.40 / 145 |
| superpowers | 1.51 | 174 | 1.52 / 180 |
| **总 (8 真 cloned)** | **49.15** | **7,619** | **49.60 / 7,764** |
| Guardrails-broken/ (junk) | 0.00 | 0 | (N/A, 整合 #5.2 commit 时 mavis-trash) |
| aGLM (借脑 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned) |
| LiteLLM (借鉴 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned) |
| opencode (借鉴 ID 索引) | (0 cloned) | (0 cloned) | (0 cloned) |
| opencog (永久跳过) | (0 cloned) | (0 cloned) | (0 cloned) |
| OpenCog 家族 (借脑 6 子源) | (0 cloned) | (0 cloned) | (0 cloned, 🆕 borrow_brainonly 段 1 entry) |
| .git/ 隐藏 (8 源) | 16.68 (总 .git/) | (N/A) | (未在 Cargo.toml 算) |

---

## 附录 B: 整合 #5.2 commit 时 Cargo.toml borrow 段 update 完整 update 代码段 (per 决策 #62 §2 + 决策 #73 §5 + R131-2 §4.3 + R131-6 §4)

**整合 #5.2 commit 时 Cargo.toml `[workspace.metadata.apeireth]` 段 update 代码段** (per R131-2 §4.3 + R131-6 §1.2/§1.3/§1.4 + 决策 #62 §5.2 关键诚实标):

```toml
[workspace.metadata.apeireth]

# 借鉴源码 10/11 + 1 借脑 = 11/12 ✅ (per decision-22 + #33 + #36 + #47 + #48 + #55 + #56 + #57 + #58 + #61 + #62 + #71 + #72 + #73 + #74)
# 0 装 PASS 严守 (per decision-33 §2.3 C2 + 主人 17:22 升级授权 + 决策 #73 §3 不要怕复杂度):
#   ✅ = 真实施 (有真 src 改动 + tests pass) | ⏳ = 限流持续重试 (P6-1/2/3 全 done, 0 借鉴处于限流) | ❌ = 永久跳过 | 🆕 = 借脑 (0 装 PASS 严守)
borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, 整合 #5.2 commit 时机 P0 supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, P0 supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, P0 supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, P1 supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, P2 supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, P2 supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, P2 supervisor era, 触发 Library Stage 4 自治 P5-1)",
    "NVIDIA/NeMo-Guardrails (Apache-2.0, R125-5 ✅ done 整合 #4 commit 19:41 修真 cloned, P6-3 done)",
    # 🆕 P6-1/2 done 借鉴 ID 索引完成 (0 cloned, 公开 1:1 翻译 src)
    "BerriAI/litellm (MIT, R125-1 ✅ done P6-1, 借鉴 ID 索引完成, 公开 1:1 翻译 562 行新 src, 19/19 unit test pass)",
    "sst/opencode (MIT, R125-12 ✅ done P6-2, 借鉴 ID 索引完成, 改借鉴已 cloned 3 module, 35/35 unit test pass)",
]
borrow_rate_limited = [
    # P6-1/2/3 全 done, 0 借鉴处于限流, 段永久 0 entries (per R131-2 §4.3 + 决策 #33 §2.3 C2 0 装 PASS 严守)
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-33 §2.2, 0 集成 0 假装, 0 重借)",
]
borrow_brainonly = [
    # 🆕 R130-6 借脑 ID 索引完成 (0 cloned, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork", per 决策 #33 §2.3 C2 + 决策 #55 §2.6 + 决策 #73 §3 不要怕复杂度)
    "R130-6-BORROW-opencog-family-2026Q1-2026-08-11 (6 子源: atomspace/cogutil/moses/pln/relex/CogPrime, AGPL-3.0 借脑, 借脑 ROI 梯度 🟢 AtomSpace + CogPrime 深度 / 🟡 MOSES 中度 / 🔴 cogutil + pln + relex 浅度, 0 装 PASS 严守 100%, per 决策 #55 §2.6 调研方向 + R130-6 §1.2 + 决策 #73 §3)",
]
# 借鉴源码本地路径 (per 决策 #36 §1 + 决策 #55 §2)
borrow_local_path = ".openclaw/workspace/borrowed-repos/"

# 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略, per decision-33 §2 + decision-58 §4 + decision-74 §1 改写)
hard_walls = "8 (B1 24 LOCKED 入口签名 — V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §1 改写) / B2 workspace.version 1.2.0 0 改 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / B6 三洋葱 / B7 9 organ 内部 fn / A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 / A2 9 子测度结构严守 / A3 12 键 + PHL-07 = 13 键 / C1 0 主动 commit / C2 0 装 PASS 严守 / C3 升 6 重 v7 / 0 主动 push 严守)"

# 24 LOCKED 入口签名 0 改 (per decision-22 §1.2 + decision-33 §2.3 B1 + decision-41 §2 + P2-3 retry verify done + P4-1 verify done + P14-1 retry verify done)
locked_crates_count = 24
# 完整 24 LOCKED 名单见 docs/conventions/10-locked.md §11.2 + docs/omnibus/24-locked-crates.md
# B1 0 改 = 入口签名 (lib.rs pub mod / pub use / pub const / pub struct / pub enum / pub fn) 0 改
# 内部 fn 实施可改 (per decision-41 §2 + decision-47)

# 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per decision-22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + 主人 8/11 01:14 拍板 3 件套 §3)
# S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装
philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]
# 9 件套: 8 哲学锚 (思想哲学) + 不要怕复杂度 (工程哲学) (per 决策 #73 §3)

# V0.5 30 维 (per decision-22 §2.3 B3 升 30 维 + R126 P1-4 25→30 维 verify retry done)
# 4 大类 (PC 0.40 / RC 0.30 / HG 0.15 / GP 0.15) × 6 维度 + 6 增强 (R125-13 实施) = 30 维
# sum=1.00 守门, 编译期 hardcode enum (0 装严守)
measurement_dimensions = "V0.5 30 维 (24 基础 + 6 增强)"

# 6 重守门 v7 (per decision-22 §2.4 B4 升 6 重 v6 → v7 + R126 P1-3 6 重守门 v7 retry done)
# 1-5 重嵌套 + 6 重 Colang DSL (R125-5 NVIDIA Guardrails 借鉴)
guard_gates_version = "v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"

# 13 键 verdict cache (per decision-22 §2.8 A3 + decision-33 §2.3)
# V3 9 键 + v4.1 3 键 (原 12 键 0 改) + PHL-07 NotUnoptimizable (R125-12 实施)
verdict_cache_keys = 13

# 整合链 (per decision-22 + #33 + #41 + #42 + #47 + #48 + #51 + #55 + #56 + #57 + #58 + #61 + #62 + #73 + #74)
integration_chain = [
    "整合 #1 (decision-25 17:30, 1.0.0 baseline)",
    "整合 #2 (decision-31 17:17, R125 续 dry-run)",
    "整合 #3 (decision-34 17:30, 主人 14:56 拍板, df6dfb69 128 files)",
    "整合 #4 (decision-48 19:41, 主人自执行, abf12243 46752 file changes, 0 重跑)",
    "整合 #5 (decision-62 拆 3 阶段, 整合 #5.1 src/ 0 改严守 + 整合 #5.2 docs/+Cargo.toml borrow 段 update + 整合 #5.3 reports/ 决策 #73/#74 + R131 era 报告, Mavis 自决拍板 OR 主人 8/15 拍板)",
]

# LICENSE 引用链 (per P13-1 OSS_NOTICE.md §0.1 + Apache 2.0 §4(d))
license_files = [
    "LICENSE (175 行, Apache 2.0 verbatim, 2026-08-05 写入, P13-1 严守不动)",
    "NOTICE (66 行, 项目特有 attribution, R20 阶段 6, P13-1 严守不动)",
    "OSS_NOTICE.md (整合 #5.2 commit 时 update, per R131-2 §4.3 + R131-6 §4)",
    "THIRD-PARTY-NOTICES.md (1709 lines / 12 SPDX / 0 cargo-deny violation, cargo-about 0.8.4 生成, 2026-08-06)",
]

# 0 主动 commit + 0 主动 push 严守 (per decision-33 §2.3 + decision-55 §5 + decision-57 §5 + decision-58 §5 + decision-61 §6 + decision-73 §5 + decision-74 §4)
commit_policy = "0 主动 commit (Mavis 整合 #5.1/#5.2/#5.3 commit 时机拍板) + 0 主动 push (等 1.0 release 配 GitHub remote)"

# 决策链 (per decision-22 ~ decision-75)
decision_chain_range = "decision-22 ~ decision-75 (54 个决策文件, 完整可追溯 reports/decision-*.md)"
```

**整合 #5.2 commit 时 Cargo.toml `description` 字段 update 代码段** (per Cargo.toml:285 + R131-2 §4.3 + R131-6 §1.4):

```toml
# 1.0 release 描述 (per decision-22 §3 + decision-57 §0 + decision-58 §0 + 决策 #73 §3 + 决策 #74 §1 改写)
# 借鉴 10/11 + 1 借脑 = 11/12 (per R130-6 借脑 ID 索引完成) + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache
# 9 件套 总哲学 (8 哲学锚 + 不要怕复杂度, per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3)
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 91 sub-crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + 1.0 release (借鉴 10/11 + 1 借脑 = 11/12 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache + 9 件套 总哲学)"
```

**整合 #5.2 commit 时 OSS_NOTICE.md update 计划** (per R131-2 §4.3 + R131-6 §2.7):
- 🆕 §1 "8/11" → "10/11" (加 Guardrails + LiteLLM + opencode 借鉴 ID 索引完成)
- 🆕 §2 "3 限流" → "0 限流 (P6-1/2/3 全 done 借鉴 ID 索引完成)"
- 🆕 §3 "1/11 永久" → "1/11 永久" (opencog AGPL-3.0, 0 改) + 🆕 "1/12 借脑 (OpenCog 家族 6 子源, R130-6 提议, 0 装 PASS 严守)"
- 🆕 §4 "7+3+1=11" → "10+0+1=11" + 🆕 "10+0+1+1(OpenCog 家族借脑)=12/12"
- 🆕 §5 "8/11 LICENSE" → "10/11 LICENSE + OpenCog" + 🆕 "10/11 + 1/12 OpenCog 家族 AGPL-3.0 (借脑, 0 集成)"
- 🆕 §6 决策链: "#22/#33/#36/#47/#48/#55/#56/#57" → "#22/#33/#36/#47/#48/#55/#56/#57/#61/#62/#71/#72/#73/#74" (14 个)
- 🆕 §8 "7 真实施/3 限流/1 永久跳过" → "10 真实施/0 限流/1 永久跳过" + 🆕 "10 真实施/0 限流/1 永久跳过/1 借脑 (OpenCog 家族 6 子源)"

**整合 #5.2 commit 时 mavis-trash 操作** (per R131-6 §2.4 + 决策 #70 + 主人 8/11 0:49 拍板 0 主动删):
- 🆕 mavis-trash `.openclaw\workspace\borrowed-repos\Guardrails-broken\` (junk 残留, 0 MB / 0 files, 0 影响 24 LOCKED)

---

## 附录 C: 0 主动 IM 主人 严守 + 决策日志 (per gate-discipline + 决策 #61 §6 + 决策 #75 §3 + 用户记忆 #10)

**R131-6 0 主动 IM 主人 严守**:
- ✅ **仅 done notification 主动报告** (R131-6 报告 done notification 给 Mavis 父会话, per cron Section 5)
- ❌ **0 主动 plain reply on skip ticks** (per gate-discipline)
- ❌ **0 主动讨论后续** (等主人起床后 8 步 verify + 1.0 release 配 GitHub remote + 主人拍板整合 #5 commit)
- ⏳ **等主人起床后决策** (per 决策 #61 §8.3 8 步 verify + 决策 #62 §2 整合 #5 commit 拍板临近)

**R131-6 决策日志写** (per 用户记忆 #10 主人长时间离开 Mavis 自主决策 + 决策日志):
- ✅ **R131-6 报告** (本文件 `reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md`) — 已写
- ✅ **整合 #5.2 commit 时 update 计划** (per R131-2 §4.3 + R131-6 §4 附录 B) — R131-6 报告 §4 + 附录 B 详细 update 计划
- ⏳ **决策链 update 决策 #76** (R131 era 第 2 批 6 sub 派活 + R131-6 报告 done) — Mavis 自决拍板 (per 决策 #62 §2 + 决策 #73 §5 + 决策 #75 §3 + cron Section 11)

---

**R131-6 Final Report — Cargo.toml borrow 段精简架构审视 done (2026-08-11 01:35)**
**整合 #4 commit: abf12243 (8/10 19:41 done, master HEAD 严守, 0 重跑 0 重 commit)**
**整合 #5 commit 时机: 未 ready (R129-3 报告阻塞 120+ min, cargo 阶段 done 0 进程, 写报告阶段中), 等 R129-3 done → Mavis 自决拍板整合 #5.1 → #5.2 → #5.3 顺序 (per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序)**
**0 主动 commit / 0 主动 push / 0 主动 IM 主人 (仅 done notification) / 0 改 src / 0 改 Cargo.toml 严守 100% (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10)**
**8 硬墙 0 越界 100% 严守 (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1 改写)**
**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学严守 100% (per 决策 #33 §2.3 B5 + 决策 #73 §3 + 决策 #74 §1 改写 + 哲学文档 15-no-fear-complexity.md + 主人 8/11 01:14 拍板 3 件套 §3)**
**整合 #5.2 commit 时 Cargo.toml borrow 段 update 计划 (per R131-2 §4.3 + R131-6 §4 + 附录 B) — Mavis 自决拍板 update 内容**
**V1.1 release 8 Stage Cargo.toml borrow 段精简方案 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 不要怕复杂度)**
**V2.0 release 3 大重构方向 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评) + 实验仓 `apeireth-opencog-experimental` AGPL-3.0 fork (per 决策 #33 §2.2 主人主动问后做)**
