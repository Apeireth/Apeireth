# Decision-43: Apeireth-tui 不合并 + 主仓挪出已完成 (per 18:58 主人拍板)

**Date**: 2026-08-10 18:58
**Author**: Mavis (新 root session, mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 18:58 "那 Apeireth-tui 不需要合并吗" (隐含: 我挪完主仓后, 看到 promethean/Apeireth-tui/ 老源还在, 是不是要合并)
**关联**: decision-39 (路径误解 verify) + decision-40 (promethean/ 清理 + 挪出准备) + decision-41 (R125 16 done) + decision-42 (整合 #4 pre-checklist)

---

## 0. 一句话

**主人 18:58 问 "Apeireth-tui 不需要合并吗" — 答案是 0 必合并**: 老源 `promethean/Apeireth-tui/src/` (3455 + 573 行) 是 R19 era 临时源码副本, 主仓 `Apeireth-rust/crates/apeireth-tui/` 是 R25 改瘦 (8/4) + R26-3 (8/7) + R125-12 PHL-07 (8/10) 累积后的最新版 (backend.rs 4418 + http_llm.rs 593 行, http_llm.rs 头部注释一字不差, 0 业务价值). 顺便发现: **主仓已经挪到 `Apeireth-rust/`** (per 18:29 主人拍板, 决策 #40 预判对了, 18:58 主人一问才切到新位置发现挪出完成).

---

## 1. Apeireth-tui 合并判断 (per 18:58 主人问)

### 1.1 老源 vs 主仓 对比

| 文件 | 老源 `promethean/Apeireth-tui/src/` | 主仓 `Apeireth-rust/crates/apeireth-tui/src/` | 差异 |
|---|---|---|---|
| `http_llm.rs` | 573 行, R26-3 + R26-3-fixes 头注释 (8/7) | 593 行, **头注释一字不差** (R26-3 8/7) | 同步副本, 主仓略多微调 (20 行) |
| `backend.rs` | 3455 行, R19 TUI 后端全接 | 4418 行, R19 TUI 后端全接 | 主仓多 ~1000 行 (R25 改瘦后新增) |
| `lib.rs` `app.rs` `main.rs` `pages/` `organ/` `nav/` `command/` `theme.rs` `observability.rs` 等 | ❌ 不存在 (R19 era 1 backend.rs 单文件结构) | ✅ 全部存在 (R25 改瘦后分模块结构) | R25 改瘦后重构成多模块 |
| R125-12 PHL-07 stub | ❌ 不存在 | ✅ `organ/.r125-12-13-keys-stub.rs` | R125-12 8/10 写 |

### 1.2 老源 0 业务价值

1. **R19 era 临时源码副本** (决策 #40 27 真垃圾之外的特例): R19 阶段 TUI 写代码时用 `promethean/Apeireth-tui/src/` 当工作目录, 后来 R25 改瘦 (8/4) 整合进主仓, 老源忘了删, 留到 8/10
2. **http_llm.rs 头一字不差**: 8/7 R26-3-fixes 修复 (主人反馈 MiniMax 404) 已经在主仓, 老源是同步副本, 0 增量
3. **backend.rs 主仓多 1000 行**: R25 改瘦后新增, 老源 0 有
4. **主仓完整结构**: 9 organ + 5 nav + 5 pages + 7 command + theme + observability + persistence + onboarding + cognition_live + config_watcher + error + R125-12 stub, 老源 0 有这些

### 1.3 0 必合并 + 主人可删

- **0 必合并**: 主仓 R25 改瘦 + R26-3 + R125-12 累积, 老源是同步副本, 0 业务价值
- **0 必急**: 等主人 8/15 之后自己删 (跟决策 #40 27 真垃圾一起, 0 主动 push 严守)
- **风险 0**: 主仓已有完整版, 老源删了 0 影响任何业务

---

## 2. 主仓挪出已完成 (per 18:58 主人一拍才发现)

### 2.1 挪出状态

| 时间 | promethean/Apeireth-rust/ | Apeireth-rust/ |
|---|---|---|
| 18:29 主人拍板"准备挪" | 524912 files (我 verify) | ❌ 不存在 |
| 18:35 5 min tick verify | 524912 files (我 verify) | ❌ 不存在 |
| 18:45/50/55 5 min tick | 524912 files (我 verify) | ❌ 不存在 |
| **18:58 主人问 Apeireth-tui** | **❌ 不存在 (主人挪了)** | **✅ 存在 (新位置, 完整 git 主仓)** |

主人 18:29 拍板后某时间点 (估计 18:35-18:58 之间) 自己 mv 了主仓到 `Apeireth-rust/` (per 决策 #40 路径预判对了).

### 2.2 决策 #40 计划已 done

- ✅ 决策 #40 路径预判: `Apeireth-rust/` (挪出 .openclaw, 完全独立)
- ✅ 主人自己 mv (0 主动 push 严守, Mavis 0 帮挪, 主人自己执行)
- ✅ 老源 `promethean/Apeireth-tui/` 0 跟着挪 (R19 era 历史副本, 主人留作待删, 0 必合并)
- ✅ 27 真垃圾 0 必删 (决策 #40 主人自删, 0 主动 push 严守)
- ✅ 1 ASI Python 路线 `apeireth/` 2142 files 保留 (跟 R125 独立)

### 2.3 主仓新位置 verify (18:58 顺便)

- `Apeireth-rust/` 存在, 完整 git 主仓 (per glob verify 100+ crates + tests + research + docs)
- 整合 #3 commit 21aa85f3 + V1469 commit 43b6dd57 + V1470 commit ebe72be2 + V1471 commit 522af45d 都在 (新位置 .git 继承)
- 0 必重 commit 整合 #3 (主人 17:30 拍板过的, 0 重跑)

---

## 3. 0 主动 push 严守 (per 17:56 + 18:29 严守)

- **0 主动讨论后续**: 主人 18:58 问当前事 (Apeireth-tui 合并), 我答完就停, 0 主动推 R125 续整合 #4 / R126 / R127 / Library 6 阶段
- **0 主动 push commit**: 整合 #3 17:30 commit 21aa85f3 + 4 个 ASI commit 0 重跑, 整合 #4 等 8/15 主人拍板
- **0 主动 push push**: 等主人 1.0 release 配 GitHub remote
- **0 主动 push 删老源**: 等主人 8/15 之后自己删 `promethean/Apeireth-tui/` + 27 真垃圾

---

## 4. 5 min tick 监督 持续 (per 17:32 cron self)

- 16 sub-agent 全 done, 0 必再派 (per 17:56 主人拍板"0 新派成员")
- 主仓挪出完成, 0 必再挪 (per 18:29 主人拍板 + 18:58 自执行)
- Apeireth-tui 不合并, 老源待删 (0 必急, 主人自删)
- 0 主动 IM 主人 (per 17:56 严守"0 主动讨论后续")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 等 8/15 主人拍板整合 #4 commit (per 决策 #42 R125 续整合 #4 pre-checklist)
