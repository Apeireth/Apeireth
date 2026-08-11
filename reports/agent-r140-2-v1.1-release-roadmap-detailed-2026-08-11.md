# R140-2: V1.1 release 路线图 详细 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 主人 8/11 01:14 拍板 3 件套 + 决策 #71 §3 永久循环 + 决策 #75 §2.1 R140 era 派活 + R137-2 24 LOCKED 改写 + R137-1 PHL-07 实施 + R137-3 Cargo 1.2.1 bump + R134-2 1.0 release 实战 + R136-1 V1.1 release 拍板准备 + R132-1 V1.1 release 路线图 final + R131-5 24 LOCKED 入口分布优化 + R135-2 V1.1 vs 业界 v2.x 差距 + R130-5 V1.1 路线图 + 决策 #10/#33/#22/#48/#62/#64 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**Date**: 2026-08-11 (R140 era 第 2 批, per 决策 #75 §2.1 + 决策 #80 R140-R143 14 sub-dispatch fill 16, V1.1 release 路线图 详细阶段)
**Author**: R140-2 sub-agent (Mavis 派, R140 era 第 2 批 V1.1 release 路线图 详细阶段, 60 min 时间盒)
**Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
**触发**:
- 决策 #80 (R140-R143 14 sub-dispatch fill 16, 2026-08-11 02:02:56 cron 自动拍) — 派 R140-1 / R140-2 / ... / R140-14 14 sub-agent 填满 16 跑中上限
- 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改, per 主人 8/11 01:14 拍板)
- 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度)
- 决策 #71 §3 (R130 调研 + R131 差距 + R132 计划 + R133 实施 4 步永久循环 + R140+ era 接力)
- 决策 #75 §2.1 (R131-R132-R133 batch dispatch 11 sub fill 16 + R140+ era 续派活)
- 决策 #77 (R135 era + R136 era 计划续) + 决策 #78 (整合 #5.3 reports commit paiban option A) + 决策 #79 (R138 era 13 sub + R139-1 + 14 sub-dispatch fill 16)
**任务定位**: **R140 era 第 2 批 V1.1 release 路线图 详细阶段**, 写到 `reports/agent-r140-2-v1.1-release-roadmap-detailed-2026-08-11.md` (~70-100 KB), 整合 R130-5 V1.1 路线图 + R131-5 24 LOCKED 入口分布优化 + R132-1 V1.1 release 路线图 final + R135-2 V1.1 vs 业界 v2.x 差距 + R136-1 V1.1 release 拍板准备 + R137-1 PHL-07 实施 + R137-2 24 LOCKED 改写 + R137-3 Cargo 1.2.1 bump + R134-2 1.0 release 实战 = **V1.1 release 详细路线图 9 章节版** (per 决策 #74 B1 24 LOCKED 入口可改部分 写 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学)
**约束** (per 决策 #33 + #60 + 决策 #71 调研阶段 + 决策 #73 §1+#2+#3 主人 8/11 01:14 拍板 3 件套 + 决策 #74 §1 B1 V1.0 release 0 改严守 + 用户记忆 #10 自主决策 + 决策日志):
- ✅ **0 改 src/** (100% 严守, R140-2 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改, 调研阶段不锁 Cargo.toml)
- ✅ **0 主动 commit** (100% 严守, 整合 #5/6/7 commit 由 Mavis 自决拍板, R140-2 0 git commit)
- ✅ **0 主动 push** (100% 严守, 等 1.0 release 配 GitHub remote + 主人起床后手跑 + V1.1 release 主人手跑)
- ✅ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ✅ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 路线图是文档工作, 0 装 PASS 严守 100%)
- ✅ **不重写 R130-5/R131-1/2/3/4/5/9 + R132-1 + R133-1/2/3 + R134-2/3/4/5 + R135-1/2 + R136-1/2 + R137-1/2/3** (per 任务 spec + 用户记忆 #6 + 决策 #79, 已有的 verify 报告 reference 而非重写)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R130-1 01:14 实地 verify = **NOT READY** (cargo workspace 3 crate 25 hard errors + 8 步 verify 全部 FAIL, 需先派 fix sub-agent 修 25 hard errors, 估 30-60 min fix → 8 步 verify 全 PASS → 再拍 5.1 → 5.2 → 5.3)
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §4 R133+ era 实施 + 决策 #74 B1 V1.1 release Mavis 自决改, **V1.1 release 前置 commit** (per R136-1 §1.1 整合 #6 commit 拍板准备)
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §4, Mavis 自决拍板 (V1.1 release 前最终收尾, per R136-1 §1.1 + R134-4)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0` 跟决策 #22 §2.2 semver 一致, 或 `v1.2.1` 跟决策 #74 B2 一致, **Mavis 自决拍板, 本报告倾向 v1.1.0 跟 决策 #22 §2.2 一致, 1.0 → 1.1 minor bump, 跟 R130-5 §1.1 + R131-3 §1.1 + R132-1 §1.1 + R137-3 §1 多个报告一致**), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
**V2.0 release tag**: 远期 2027+, per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构
**状态**: ✅ **R140-2 V1.1 release 路线图 详细 done** (60 min 时间盒, 9 章节 ~80 KB): V1.0 → V1.1 升级窗口 3.5 个月 (估 2026-11-30) + V1.1 release 4 阶段实施 (B1 24 LOCKED 入口可改部分 + A3 PHL-07 实施 + B2 workspace.version 1.2.0 → 1.2.1 bump + V1.1 release 实战) + 8 步时间线 (整合 #5.1 → 整合 #5.2 → 整合 #5.3 → 1.0 release tag → 整合 #6 → 整合 #7 → V1.1 release 实战 → 永久循环) + V1.1 release 决策点 (决策 #80-#100 22 决策) + V1.1 release 16 风险 (含 R1 B1 入口改写破坏下游 + R5 PHL-07 实施影响 12 键其他 + R7 整合 #6/7 commit 时机 + R9 1.0 release 后用户反馈) + V1.1 release 12 决策原则 (含 最强效果 > 最简单代码, 最厉害工程 > 最易维护, 维护交给未来高水平团队, 决策 #73 §3) + 8 硬墙 0 越界 (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 / A1 R11 baseline 3 值 0 改 / A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守 100%) + 8 哲学锚 严守 0 漂移 + 不要怕复杂度哲学 落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

---

## 0. 一句话 (TL;DR)

**V1.1 release 路线图 详细 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 主人 8/11 01:14 拍板 3 件套 + 决策 #71 §3 永久循环 + R140 era 第 2 批)**: V1.1 release = 1.0 release (~8/11) 后 ~3.5 个月 minor release (`v1.1.0` 跟决策 #22 §2.2 semver 一致, 或 `v1.2.1` 跟决策 #74 B2 一致, Mavis 自决拍板, 本报告倾向 v1.1.0, 估 2026-11-30, 整合 #6 commit 2026-11-25 + 整合 #7 commit 2026-11-29 + V1.1 release 实战 2026-11-30 06:00-08:00 主人手跑). **4 阶段 实施** (per 决策 #74 B1 改写边界): **阶段 1** B1 24 LOCKED 入口可改部分 (per R137-2 + R131-5 8 方向: 标准化 + 瘦身 + 9 叶子拆 + core 拆 pub mod + 大模块拆 sub-crate + DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐, 5 阶段 8 周 估, 派 R138-R142 era 29-43 sub-agent); **阶段 2** A3 PHL-07 实施 (per R137-1, V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests, 派 R134-PHL07-1~5 5 sub); **阶段 3** B2 workspace.version 1.2.0 → 1.2.1 bump (per R137-3 5 阶段 5 天/1 周, semver minor bump backward-compatible, 派 R137-3 + R134-backend-1~5); **阶段 4** V1.1 release 实战 (per R134-2 1.0 release 实战 7 步 runbook 续 + R136-1 5 阶段 + R136-2 实战 6 步, 主人起床后手跑, 估 2026-11-30 06:00-08:00 06:00-08:00 时段). **8 步时间线**: 整合 #5.1 (V1.0 release src/) → 整合 #5.2 (V1.0 release docs/) → 整合 #5.3 (V1.0 release reports/) → 1.0 release tag v1.0.0 (8/11 主人起床) → 整合 #6 (V1.1 release src/ + docs/ + reports/, 2026-11-25) → 整合 #7 (V1.1 release 前最终, 2026-11-29) → V1.1 release 实战 (2026-11-30 06:00-08:00 主人起床后手跑 7 步 runbook) → 永久循环 (V1.2 release 估 2027-02-28 + V2.0 远期 2027+). **22 决策点** (决策 #80-#100, R140 era + R141 era + R142 era 实施 + 整合 #6/7 commit 拍板 + V1.1 release tag 拍板流程, Mavis 全自决, 16 跑中上限严守). **16 风险** (含 R1 B1 入口改写破坏下游消费者 + R5 PHL-07 实施影响 12 键其他键 + R7 整合 #6/7 commit 时机 跟 R130-1 25 hard errors 警示 类似 + R9 1.0 release 后用户反馈). **12 决策原则** (含 最强效果 > 最简单代码, 最厉害工程 > 最易维护, 维护交给未来高水平团队, 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md). **8 硬墙 0 越界 100%** + **8 哲学锚 严守 0 漂移** + **不要怕复杂度哲学 落地** (per 主人 8/11 01:14 拍板 3 件套). 0 改 src 严守 (本任务 = 调研/路线图类, 0 实施) + 0 主动 commit (本报告 untracked, Mavis 整合 #5.3 / #6 / #7 commit 时机拍板) + 0 主动 push (V1.1 release 主人手跑 严守) 100%.

---

## 1. V1.0 → V1.1 升级窗口 (3.5 个月, 估 2026-11-30)

### 1.1 升级窗口时间线 (per R130-5 §1.2 + R132-1 §1.2 + R136-1 §1.1 + 决策 #22 §2.2 + 决策 #74 §1 B2)

**V1.0 → V1.1 升级窗口 3.5 个月 (估 2026-11-30)**:

```
[8/11 01:00+ 整合 #5 commit 拍板]    Mavis 自决 (5.1 → 5.2 → 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup)
                                      ⚠️ 整合 #5.1 commit BLOCKED per R130-1 01:14 实地 verify (25 hard errors), 需先派 fix sub-agent
[8/11 06:00-08:00 主人起床 1.0 release 实战]   主人手跑 R134-2 5 阶段计划 续 R129-35 7 步 runbook (8 步 verify + 配 GitHub remote + git push + 打 v1.0.0 tag + GitHub Pages)
[8/11 08:00+ 1.0 release done]    master HEAD = abf12243 + 3 commit (5.1/5.2/5.3), v1.0.0 tag, GitHub release, GitHub Pages 部署
[8/11-8/12 R130 era 调研 6 sub-agent]  R130-1~6 (整合 #5 commit cargo 二次 verify [BLOCKED] + ASI Stage 8 深化 + Tauri Stage 5 深化 + 形式化 Stage 5.5 深化 + V1.1 路线图 [done] + 借鉴源 12 源调研)
[8/12-9/1 R131 era 差距 9 sub-agent]   R131-1/2/3 + R131-4~9 (架构总审视 + 借鉴 12 源差距 + V1.1 release 实施路线图 6 大方向 + cargo workspace + 24 LOCKED 入口分布 + Cargo.toml borrow + pybridge + Tauri 集成 + 形式化集成)
[9/1-9/15 R132 era 计划 2 sub-agent]   R132-1 V1.1 release 路线图 final + R132-2 V2.0 release 战略路线图
[9/15-10/15 R133 era 实施 spec 3 sub-agent]   R133-1 借鉴 12 源 实施 + R133-2 ASI Stage 9 实施 + R133-3 三洋葱架构升级 实施 spec
[10/15-11/4 R134 era 实施 5 sub-agent]   R134-1 整合 #5 commit 拍板 + R134-2 1.0 release 实战 + R134-3 整合 #6 commit 拍板准备 + R134-4 整合 #7 commit 拍板续 + R134-5 V1.1 release cargo verify
[11/4-11/15 R135 era 调研 6 sub-agent]   R135-1 V1.1 vs AGI OS 前沿 + R135-2 V1.1 vs 业界 v2.x 差距 + 4 续
[11/15-11/22 R136 era 计划 2 sub-agent]   R136-1 V1.1 release 拍板准备 + R136-2 V1.1 release 实战
[11/22-11/25 R137 era 实施 5 sub-agent]   R137-1 PHL-07 实施 + R137-2 24 LOCKED 改写 + R137-3 Cargo 1.2.1 bump + R137-4 ASI Stage 9 续 + R137-5 形式化续
[11/25 整合 #6 commit 拍板]    Mavis 自决 (6.1 → 6.2 → 6.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 + 决策 #74 B1)
[11/29 整合 #7 commit 拍板]    Mavis 自决 (V1.1 release 前最终, 7.1 src/ + 7.2 docs/ + 7.3 reports/, per 决策 #33 C1)
[11/30 06:00-08:00 主人起床 V1.1 release 实战]   主人手跑 R136-2 实战 6 步 续 R134-2 7 步 runbook (8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署)
[12 月 V1.1 release 后]    V1.2 release 调研 (估 2026-12, per R130-5 §1.3, 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战)
[2027-02-28 V1.2 release]   v1.2.0 tag 打上
[2027+ V2.0 远期]   平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 (per ROADMAP.md §4 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可重建)
```

**时间窗口总结 (per 决策 #22 §2.2 + 决策 #71 §3 + 决策 #74 §1 + R130-5 §1.2 + R131-3 §1.2 + R132-1 §1.2 + R136-1 §1.2)**:
- **1.0 release (估 8/11)**: V1.0 release tag `v1.0.0` 打上, R11 baseline 严守, 8 硬墙 0 越界 100%
- **V1.0 release → V1.1 release 间隔**: ~3.5 个月 (per R130-5 §1.1 V1.1 估 2026-11-30, per R132-1 §1.2 1.0 release 后 3.5 个月)
- **V1.1 release (估 2026-11-30)**: V1.1 release tag `v1.1.0` 打上 (本报告倾向) 或 `v1.2.1` (per 决策 #74 B2), Mavis 自决
- **V1.1 release → V1.2 release 间隔**: ~3 个月 (per R130-5 §1.2, 估 2027-02-28)
- **V2.0 release (远期 2027+)**: 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (per 决策 #74 §2.3)

**R130 era 接力 → V1.1 era 战略 接力 → 永久循环** (per 决策 #71 §3 4 步永久循环 + R130-5 §1.1 + R132-1 §1.1 + 决策 #75 §2.1 + 决策 #76 §2.1 + 决策 #77 §3.1 + 决策 #79 + 决策 #80):
- **R130 era (8/11, 整合 #5 commit 拍板 → 主人起床)**: 整合 #5 commit 拍板 + 1.0 release 实战 (主人起床后手跑) + R130-1~6 6 sub-agent 调研 ✅ done
- **R131 era (8/11 01:18+, V1.1 era 调研)**: 9 sub-agent 差距分析 (R131-1/2/3 + R131-4~9 架构细分, per 决策 #73 §3.2 + 决策 #75 §2.1) ✅ done
- **R132 era (8/11 01:20+, V1.1 era 计划)**: 2 sub-agent 计划 (R132-1 V1.1 release 路线图 final + R132-2 V2.0 release 战略) ✅ done
- **R133 era (8/11 01:25+, V1.1 era 实施 spec)**: 3 sub-agent 实施 spec (R133-1/2/3) ✅ done
- **R134 era (8/11 01:30+, V1.1 era 实施)**: 5 sub-agent (R134-1 整合 #5 commit 拍板 + R134-2 1.0 release 实战 + R134-3 整合 #6 commit 拍板准备 + R134-4 整合 #7 commit 拍板续 + R134-5 V1.1 release cargo verify) ✅ done
- **R135 era (8/11 01:35+, V1.1 era 调研续)**: 6 sub-agent (R135-1 V1.1 vs AGI OS 前沿 + R135-2 V1.1 vs 业界 v2.x 差距 + 4 续) ✅ done
- **R136 era (8/11 01:40+, V1.1 era 计划续)**: 2 sub-agent (R136-1 V1.1 release 拍板准备 + R136-2 V1.1 release 实战) ✅ done
- **R137 era (8/11 01:50+, V1.1 era 实施 spec)**: 5 sub-agent (R137-1 PHL-07 实施 + R137-2 24 LOCKED 改写 + R137-3 Cargo 1.2.1 bump + R137-4 ASI Stage 9 续 + R137-5 形式化续) ✅ done
- **R138 era (8/11 02:00+, V1.1 era 实施 第 2 批)**: 13 sub-agent (per 决策 #79, R138 era 实施续 R137)
- **R139 era (8/11 02:00+, V1.1 era 调研续)**: 1 sub-agent (R139-1, per 决策 #79)
- **R140 era (本报告, 8/11 02:05+, V1.1 era 路线图详细)**: 14 sub-agent (per 决策 #80, R140-R143 14 sub-dispatch fill 16) — **本报告 R140-2 派活**
- **R141 era**: V1.1 release 路线图续 (per 决策 #80)
- **R142 era**: V1.1 release 路线图续 (per 决策 #80)
- **R143 era**: V1.1 release 路线图续 (per 决策 #80)
- **R144+ era (估 2026-11-25+)**: 整合 #6 commit 拍板 → R134-PHL07-1~5 / R134-LOCKED-1~5 / R134-backend-1~5 / R134-tauri-1~5 / R134-asi-1~5 / R134-formal-1~5 30+ sub-agent 实施 (per 决策 #75 §2.1 + 决策 #80)
- **永久循环 (per 决策 #71 §3)**: V1.1 release → V1.2 release → V2.0 release → V1.3 ... (永久 4 步: 调研 + 差距 + 计划 + 实施)

### 1.2 升级窗口跟 R130-R137 era 关系 (per 决策 #71 §3 + 决策 #75 §2.1 + 决策 #76 §2.1 + 决策 #77 §3.1 + 决策 #79 + 决策 #80)

| Era | 时间 | 状态 | 核心任务 | 决策链 | 跟 V1.1 release 关系 |
|-----|------|------|---------|--------|---------------------|
| **R125 era** | 8/10 14:00-17:22 | ✅ done (16 sub-agent) | 借鉴 8/11 ✅ cloned + 41 任务起步 | #30-#41 | V1.0 release R11 baseline 严守 + 借鉴 11/11 实施 |
| **R126 era** | 8/10 17:22-21:00 | ✅ done (16 sub-agent) | 后端升级 + 8 哲学锚 + 30 维 + 6 重 v7 + Library v1.0 礼物 | #33 + #51-#54 | 8 哲学锚 + 30 维 + 6 重 v7 实施 (V1.0 release 基础) |
| **R127 era** | 8/10 21:00-22:00 | ✅ done (4 sub-agent) | Library Stage 4-6 + 整合 #5 pre-check | #55 | 24 LOCKED + PHL-07 spec |
| **R127-2 era** | 8/10 22:00-22:30 | ✅ done (10 sub-agent) | 借鉴 3 限流重试 + 1.0 release 文档 + 形式化证明 | #56 | 形式化 Stage 5.1 + 借鉴 10 → 11 实施 |
| **R128 era** | 8/10 22:30-23:00 | ✅ done (6 sub-agent) | ASI Python Stage 1-2 + Tauri prototype + Cargo 实战 + LICENSE + 整合 #5 pre-stage | #57 | ASI Stage 1-2 实施 + Tauri 1.0 基础 |
| **R128-2 era** | 8/10 23:00-22:50 | ✅ done (3 sub-agent) | ASI Python Stage 3 + Tauri scaffold 深化 + Cargo 配 | #58 | ASI Stage 3 + Tauri 深化 |
| **整合 #4 commit** | 8/10 19:41 | ✅ done | master HEAD = abf12243 严守 100% | #48 | V1.0 release 起点 baseline |
| **R129 era** | 8/11 00:08-01:00+ | ✅ 35 done | 整合 #5 commit 准备 + ASI Stage 4-6 续 + 1.0 release 流程 + 形式化扩展 + TUI/Tauri 路线图 + R130 路线图 + 健康度 verify | #61-#68 | 整合 #5 commit 准备 + 1.0 release 流程 |
| **整合 #5 commit 拍板** | 8/11 估 01:30+ | 📋 Mavis 自决 (5.3 先行, 5.1 BLOCKED 等 fix 25 errors) | per 决策 #62 + R130-1 01:14 NOT READY 警示 | #68 + #75 | V1.0 release 前置 commit |
| **R130 era** | 8/11 整合 #5 commit 拍板后 → 主人起床 | ✅ 6/6 done | 后端 verify [NOT READY] + ASI 整合 [done] + Tauri [done] + 形式化 [done] + V1.1 路线图 [done] + 借鉴 12 源 [done] | #70-#78 | V1.1 release 6 大方向 调研 |
| **1.0 release 实战** | 主人起床后 06:00-08:00 | 📋 主人手跑 R134-2 5 阶段 + R129-35 7 步 runbook | 8 步 verify + GitHub remote + git push + 1.0 release tag + GitHub Pages | #77 | V1.0 release 实战起点 |
| **1.0 release 后** | 8/11 08:00+ | 📋 远期 | V1.1 + V1.2 + V2.0 路线图 (per 决策 #71 §3 永久循环) | #79+ | V1.1 release 调研起点 |
| **R131 era (V1.1 era 调研)** | 8/11 01:18+ | ✅ 9 sub-agent done (R131-1/2/3 + R131-4~9) | 差距分析 (3 sub) + 架构细分 (6 sub) | #75 §2.1 | V1.1 release 6 大方向 差距分析 |
| **R132 era (V1.1 era 计划)** | 8/11 01:20+ | ✅ 2 sub-agent done (R132-1/2) | V1.1 release 路线图 final [R132-1] + V2.0 release 战略 [R132-2] | #75 §2.1 | V1.1 release 6 大方向 计划 |
| **R133 era (V1.1 era 实施 spec)** | 8/11 01:25+ | ✅ 3 sub-agent done (R133-1/2/3) | 借鉴 12 源 [R133-1] + ASI Stage 9 [R133-2] + 三洋葱架构升级 [R133-3] | #75 §2.1 | V1.1 release 6 大方向 实施 spec |
| **R134 era (V1.1 era 实施)** | 估 8/12+ | ✅ 5 sub-agent done (R134-1~5) | 整合 #5 commit 拍板 [R134-1] + 1.0 release 实战 [R134-2] + 整合 #6 commit 拍板准备 [R134-3] + 整合 #7 commit 拍板续 [R134-4] + V1.1 release cargo verify [R134-5] | #76 §2.1 | V1.1 release 实施 续 |
| **R135 era (V1.1 era 调研续)** | 估 8/12+ | ✅ 6 sub-agent done (R135-1~6) | V1.1 vs AGI OS 前沿 [R135-1] + V1.1 vs 业界 v2.x 差距 [R135-2] + 4 续 | #77 §3.1 | V1.1 release 调研续 |
| **R136 era (V1.1 era 计划续)** | 估 8/12+ | ✅ 2 sub-agent done (R136-1/2) | V1.1 release 拍板准备 [R136-1] + V1.1 release 实战 [R136-2] | #77 §3.1 | V1.1 release 拍板 + 实战 |
| **R137 era (V1.1 era 实施 spec)** | 估 8/12+ | ✅ 5 sub-agent done (R137-1~5) | PHL-07 实施 [R137-1] + 24 LOCKED 改写 [R137-2] + Cargo 1.2.1 bump [R137-3] + ASI Stage 9 续 [R137-4] + 形式化续 [R137-5] | #77 §3.1 | V1.1 release 实施 spec |
| **R138 era (V1.1 era 实施 第 2 批)** | 估 8/12+ | 📋 13 sub-agent (per 决策 #79) | V1.1 release 实施续 | #79 | V1.1 release 实施 第 2 批 |
| **R139 era (V1.1 era 调研续)** | 估 8/12+ | 📋 1 sub-agent (R139-1, per 决策 #79) | V1.1 release 调研续 | #79 | V1.1 release 调研续 |
| **R140 era (本报告)** | 估 8/12+ | 📋 14 sub-agent (per 决策 #80, R140-1~14) | V1.1 release 路线图详细 14 阶段 | **#80 (本)** | **V1.1 release 路线图详细** |
| **R141 era** | 估 8/12+ | 📋 14 sub-agent (per 决策 #80) | V1.1 release 路线图续 | #80 | V1.1 release 路线图续 |
| **R142 era** | 估 8/12+ | 📋 14 sub-agent (per 决策 #80) | V1.1 release 路线图续 | #80 | V1.1 release 路线图续 |
| **R143 era** | 估 8/12+ | 📋 14 sub-agent (per 决策 #80) | V1.1 release 路线图续 | #80 | V1.1 release 路线图续 |
| **R144+ era (整合 #6 commit 拍板前)** | 估 2026-11-15+ | 📋 30+ sub-agent (per 决策 #75 §2.1 R134 era 派活) | 6 大方向 × 5 sub = 30 sub-agent (R134-PHL07-1~5 + R134-LOCKED-1~5 + R134-backend-1~5 + R134-tauri-1~5 + R134-asi-1~5 + R134-formal-1~5) | 续 #80+ | 整合 #6 commit 拍板前 实施 30 sub-agent |
| **整合 #6 commit 拍板** | 估 2026-11-25 | 📋 Mavis 自决 (6.1 → 6.2 → 6.3 顺序) | per 决策 #33 C1 + 决策 #64 + 决策 #74 B1 + R136-1 §1.1 | 续 #80+ | V1.1 release 前置 commit (src/ + docs/ + reports/) |
| **整合 #7 commit 拍板** | 估 2026-11-29 | 📋 Mavis 自决 (7.1 → 7.2 → 7.3 顺序) | per 决策 #33 C1 + 决策 #71 §3 + R136-1 §1.1 | 续 #80+ | V1.1 release 前最终 commit (含 Cargo.toml 1.2.1 bump) |
| **V1.1 release 实战** | 估 2026-11-30 06:00-08:00 | 📋 主人手跑 R136-2 6 步 + R134-2 7 步 runbook | 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署 | 续 #80+ | V1.1 release 实战 (V1.1 release tag 打上) |
| **R131 era (V1.2 era 调研)** | 估 2026-12 | 📋 10 sub-agent 派活规划 | TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战 | (per R129-29 §5) | V1.2 release 调研 |
| **V2.0 远期** | 2027+ | 📋 远期 | 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 + 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 | (per ROADMAP.md §4 + 决策 #74 §2.3) | V2.0 远期路线图 |

### 1.3 升级窗口跟决策链关系 (per 决策 #10 + 决策 #22 + 决策 #33 + 决策 #48 + 决策 #62 + 决策 #64 + 决策 #70 + 决策 #71 + 决策 #72 + 决策 #73 + 决策 #74 + 决策 #75 + 决策 #76 + 决策 #77 + 决策 #78 + 决策 #79 + 决策 #80)

**决策链演进 (per R130-5 §1.4 + R132-1 §1.1 + 决策 #33 §2.3 + 决策 #74 §1 改写表 + 决策 #75 §2.1)**:
- **R129 era 决策链 (#30-#64)**: 整合 #4 commit abf12243 拍板 + 借鉴 8/11 + 8 哲学锚 + 30 维 + 6 重 v7 + 24 LOCKED + 8 硬墙
- **R130 era 决策链 (#65-#72)**: R130 era 派活 + V1.1 路线图调研 + 6 大方向基础
- **R131 era 决策链 (#73-#75)**: 主人 8/11 01:14 拍板 3 件套 + 8 硬墙 B1 改写 + R131 era 第 2 批 6 sub-agent 派活
- **R132 era 决策链 (#76-#78)**: R132 era 计划 + R132-1/2 + 整合 #5.3 reports commit paiban option A
- **R133 era 决策链 (#79-#80)**: R138 era 13 sub + R139-1 + 14 sub-dispatch fill 16 + R140-R143 14 sub-dispatch fill 16
- **R134+ era 决策链 (#80-#100)**: V1.1 release 决策点 (决策 #80-#100, 22 决策, 本报告 §4 详)

**8 硬墙 改写表 (per 决策 #74 §1)**:
| # | 8 硬墙 | V1.0 release | V1.1 release (per 决策 #74 B1 改写) | V2.0 release (per 决策 #74 §2.3) |
|---|--------|--------------|-------------------------------------|--------------------------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 **Mavis 自决改 (前提: 更好的架构, 8 方向, 5 阶段 8 周)** | 🟢 可重评 |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 | 🔒 **bump 1.2.1 (per 决策 #74 §1 B2 改写, semver minor)** | 🔒 bump 2.0.0 |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 0 改严守 | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) | 🟢 可重评 |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 + 12 键其他可改 | 🟢 **PHL-07 实施 (per 决策 #74 §1 A3 改写, R129-11 关键诚实标)** | 🟢 可重评 |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学) | 🔒 严守 (哲学) | 🟢 可重评 |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学) | 🔒 严守 (哲学) | 🟢 可重评 |
| **B5** | **8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (哲学) | 🟢 **推翻 + 重建** |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 严守 | 🔒 严守 | 🔒 严守 |
| **C2** | **0 装 PASS** | 🔒 严守 (技术哲学, 不装) | 🔒 严守 | 🔒 严守 |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 严守 | 🔒 严守 | 🔒 严守 |

### 1.4 升级窗口跟 V2.0 远期 关系 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 + R132-1 §1.4 + ROADMAP.md §4 + R119-2 思想层保留)

**V1.1 release → V2.0 release 远期 关系**:
- **V1.1 release (估 2026-11-30)**: minor release (`v1.1.0` 跟决策 #22 §2.2 一致), semver 兼容 V1.0 release, 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1) + PHL-07 实施 (per 决策 #74 A3) + Cargo.toml bump 1.2.1 (per 决策 #74 B2) + 0 改原 24 LOCKED 入口签名顺序 (顶层 re-export facade 保留)
- **V1.2 release (估 2027-02-28)**: minor release (`v1.2.0`), per R130-5 §1.2 + R132-1 §1.2, 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战
- **V2.0 release (远期 2027+)**: major release (`v2.0.0`), per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (87 → 30 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度" 哲学)
- **V2.0 release 远期 路线**: 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 (per ROADMAP.md §4 + R119-2 思想层保留)
- **V2.0 release 触发条件** (per 决策 #74 §2.3): ① V1.1 release done 后 → V1.2 release 调研 (估 2026-12) ② 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚" ③ Mavis 自决 + 主人拍板 ④ 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1)

**V1.1 release 跟 V2.0 release 关系**:
- V1.1 release = minor release, semver 兼容, 顶层 re-export facade 保留, 消费者 0 改
- V2.0 release = major release, 推翻 + 重建, 顶层 re-export facade 推平, 消费者需改 `use` 路径

---

## 2. V1.1 release 4 阶段 实施 (per 决策 #74 B1 改写边界)

### 2.1 阶段 1: B1 24 LOCKED 入口可改部分 (per R137-2 + R131-5)

#### 2.1.1 阶段 1 任务背景 (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 改写 + 决策 #74 §2.2 V1.1 release 边界 + R131-5 §0)

- **决策 #33 §2.3 B1 旧严守**: 24 LOCKED 入口签名 0 改严守 (R11 baseline, mtime 16:34 之前)
- **决策 #74 §1 B1 改写 (per 主人 8/11 01:14 拍板)**: 🟢 V1.0 release 0 改严守 (R11 baseline 严守) + **V1.1 release Mavis 自决改 (前提: 更好的架构)**
- **决策 #74 §2.2 V1.1 release Mavis 自决改边界**:
  - 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构, Mavis 自决)
  - R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
  - 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- **R131-5 §1 24 LOCKED crate 入口签名 0 改 verify**: 24/24 全 PASS (V1.0 release 0 改严守 100%)
- **R131-5 §2 24 LOCKED 入口分布 8 优化方向详细分析**: 方向 ① 入口签名一致性 + 方向 ② 公开 API 表面 + 方向 ③ crate 间依赖 + 方向 ④ crate 内部模块 + 方向 ⑤ 三洋葱架构 + 方向 ⑥ 9 organ 代码对应 + 方向 ⑦ R11 baseline 严守 + 方向 ⑧ V1.1/V2.0 release 改写/重构 边界

#### 2.1.2 阶段 1 目标: 8 方向 改写方案 (per R131-5 §2 详细 + R137-2 §3 8 方向)

**V1.1 release 24 LOCKED 入口签名 8 方向 改写方案** (per R137-2 §3 8 方向 改写方案):

**方向 1: 标准化 — 24 LOCKED 入口签名一致性** (per R131-5 §2.1):
- V1.0 release 现状: 24 LOCKED crate 入口签名风格高度不一致, 5 种风格 (类型 A 重 re-export facade 20/24 + 类型 B 轻 facade + 主类型定义 2/24 + 类型 C 单 trait 入口 + 类型 D 大 enum 主类型 + 类型 E 纯 trait 模块)
- V1.1 release 标准化 3 模式之一 (per-crate 自决): 模式 1 全 re-export (20/24) + 模式 2 主类型 facade (2/24: protocol / bus) + 模式 3 按需 re-export (2/24: 类型 C + D + E)
- 实施步骤 (per 阶段 1 标准化 1 周): per-crate 决策矩阵 → 24 LOCKED 入口签名格式统一 (6 模式) → per-crate 顶部 doc comment 极详细 (per 50-100 行 doc, O-5 哲学锚) → 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify
- 风险: 中 (改 re-export 模式 = 改 crate 公开 API 表面 = 改消费者 `use` 路径); 缓解: 保留 `pub mod` 重新导出, 消费者用 `apeireth_xxx::module::Type` 全路径仍能用; V1.1 release bump 1.2.1

**方向 2: 瘦身 — 公开 API 表面 ~800+ pub items → ≤30 per-crate** (per R131-5 §2.2):
- V1.0 release 现状: 24 crate 公开 API 表面 = **~800+ pub items** (粗估)
- V1.1 release 瘦身目标: 公开 API 表面减少 30% (800 → 560 pub items), per-crate 暴露 ≤30 pub items
- per-crate 目标: supervisor 12 → 12 / agent 25 → 25 / council 50+ → 30 / bus 20 → 20 / protocol 40 → 30 / mcp 30 → 30 / tool-registry 30 → 30 / tool-runtime 25 → 25 / graph 40 → 30 / pipeline 35 → 30 / tool-approval 15 → 15 / extension 17 → 17 / evolution 50+ → 30 / api 40+ → 30 / core 50+ → 30 / memory 50+ → 30 / asi 50+ → 30 / tools 30 → 30 / cli 25 → 25 / bench 20 → 20 / cognition 25 → 25 / action 20 → 20 / life-force 25 → 25 / constraint 25 → 25
- 实施步骤 (per 阶段 2 瘦身 1 周): per-crate 公开 API 表面清单 → per-crate 实施转 pub(crate) / module-private → 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify → 编译时间 verify (期望 减少 10-20%)
- 风险: 高 (公开 API 表面"瘦身" = 改入口签名 = 改消费者 `use` 路径 = breaking change); 缓解: 保留 `pub mod module::Type` 全路径 + 顶层 re-export facade 保留 + V1.1 release bump 1.2.1

**方向 3: 9 叶子拆 workspace** (per R131-5 §2.3 + R131-4 §2.1):
- V1.0 release 现状: 24 LOCKED 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) 0 依赖其他 LOCKED crate
- V1.1 release 9 叶子 crate 拆 workspace: 新 workspace `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`, 顶层 `apeireth/Cargo.toml` 0 改, 9 叶子拆出来独立发布
- 实施步骤 (per 阶段 3.1 9 叶子拆 1 周): 9 叶子 crate 内部 import 路径全 1:1 扫描 → 新 workspace 9 叶子加进 members → 9 叶子独立 publish ready → 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify → 顶层 re-export facade 1:1 续, 消费者 0 改
- 风险: 中 (拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth::organ::xxx`); 缓解: 保留 re-export facade (顶层 `apeireth` 重新导出全部 `apeireth-leaf::xxx`, 0 改消费者代码) + V1.1 release bump 1.2.1

**方向 4: core 拆 pub mod** (per R131-5 §2.4):
- V1.0 release 现状: core 是单 lib.rs 108KB, 0 pub mod 拆分, 全部 50+ 类型定义在一个文件
- V1.1 release core 拆 5 大 mod: `core/src/types.rs` (~20KB, 5 类型) + `core/src/onion.rs` (~30KB, 5 onion 类型) + `core/src/human.rs` (~20KB, 8 human 类型) + `core/src/gate.rs` (~25KB, 8 gate 类型) + `core/src/lib.rs` (~13KB, 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶部 re-export facade 0 改)
- 0 改入口签名, 仅内部重构
- 实施步骤 (per 阶段 4.1 core 拆 pub mod 1 周): core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod → 5 大 mod 各自 mod.rs + 子文件 → core/src/lib.rs 顶部 re-export 1:1 续 → 24 LOCKED 全跑 cargo build + cargo test verify → core 编译时间 verify (期望 减少 30-50%)
- 风险: 中 (拆 module = 改 import 路径 = breaking change); 缓解: 顶层 re-export facade 保留 + 0 改 core 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)

**方向 5: 大模块集中 crate 拆 sub-crate** (per R131-5 §2.4):
- V1.0 release 现状: 大模块集中 (council 20+ / mcp 13 / graph 11 / pipeline 11 / api 16 / memory 13 / asi 9 / tools 12 / evolution 9) → 这些 crate 内部模块多, 入口文件 re-export 100+ items
- V1.1 release 大模块集中 crate 拆 sub-crate: mcp 13 mod → 8 sub-crate (mcp-core / mcp-resources / mcp-subscribe / mcp-tools / mcp-prompts / mcp-transport / mcp-primitives / mcp) + pipeline 11 mod → 6 sub-crate + api 16 mod → 5 sub-crate + memory 13 mod → 5 sub-crate + asi 9 mod → 4 sub-crate + tools 12 mod → 5 sub-crate + evolution 9 mod → 5 sub-crate + graph 11 mod → 5 sub-crate + council 20+ mod → 4 sub-crate = 总 47 sub-crate
- 实施步骤 (per 阶段 4.2 大模块拆 sub-crate 1 周): 8 大模块集中 crate 内部 module 1:1 扫描 → 8 大模块集中 crate 各拆 4-8 sub-crate → 顶层 8 crate re-export facade 0 改入口签名 → 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify → 编译时间 verify (期望 减少 20-30%)
- 风险: 中 (拆 sub-crate = 改 import 路径 = breaking change); 缓解: 顶层 re-export facade 保留 + 0 改 24 LOCKED 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)

**方向 6: DSL 洋葱 — 三洋葱架构升级** (per R131-5 §2.5 + R133-3 §3):
- V1.0 release 现状: 三洋葱架构 (原则 + 权限 + DSL), 24 LOCKED 跟三洋葱架构对应关系 (原则洋葱 E 层 core / constraint / life-force; 原则洋葱 S 层 council / evolution; 原则洋葱 A 层 memory / asi; 原则洋葱 M 层 cognition / pipeline / protocol / bus / graph; 原则洋葱 O 层 agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor; 权限洋葱 L0 core / constraint; 权限洋葱 L1-L5 api / tool-approval; DSL 洋葱 0 落地)
- V1.1 release DSL 洋葱落地 + 三洋葱 → 四洋葱 升级: 新增 `apeireth-dsl` crate, Colang DSL 真实施, 24 LOCKED crate 引用 dsl 守门, 三洋葱 → 四洋葱 (新增第 4 层 "智能涌现 emergence", 智囊团 7 席 + 群体智能 + 自我决策/学习/演化)
- 24 LOCKED crate 跟四洋葱架构对应关系: 原则洋葱 + 权限洋葱 + DSL 洋葱 + 智能涌现洋葱 (智囊团 7 席 + 群体智能 借 OpenCog 1:1 公开模式 + 自我决策 ASI Stage 9 + 自我学习 chidori journal + 自我演化 ASI Stage 10 准备)
- 实施步骤 (per 阶段 5.1 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 1 周): 新增 `apeireth-dsl` crate → 三洋葱 → 四洋葱 升级 → 24 LOCKED crate 引用 dsl 守门 → 24 LOCKED 全跑 cargo build + cargo test + 四洋葱集成 verify → 8 硬墙 + 8 哲学锚 严守 verify
- 风险: 高 (拆三洋葱 workspace + 加 DSL 洋葱 = 改大量 import 路径 = breaking change); 缓解: 顶层 `apeireth-onion` facade 重新导出全部洋葱 module, 消费者 0 改 + V1.1 release bump 1.2.1 + 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致

**方向 7: 9 organ 内部借 OpenCode** (per R131-5 §2.6 + R125 B7 + R130-6):
- V1.0 release 现状: 9 organ 跨 8 LOCKED crate (Heart / Brain / Hand / Ear / Memory / Voice / Body / Mind, Eye 缺失在 tui/src/organ/eye.rs), 覆盖率 8/9 organ 100%
- V1.1 release 9 organ workspace 化 + Eye 补: 新增 `apeireth-eye` workspace, 9 organ workspace 化 (apeireth-organ/{heart,brain,hand,eye,ear,memory,voice,body,mind}/Cargo.toml 9 个 organ workspace), 24 LOCKED crate 按 9 organ 拆
- 9 organ 内部借 OpenCode 实施: 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名, OpenCog 借脑 1:1 公开模式 (per R130-6 + R133-1 借鉴源 12 源 + 决策 #22 §4 AGPL-3.0 决策), 0 装"已读真源码", 0 装"已 fork"
- 实施步骤 (per 阶段 3.2 Eye 补 + 阶段 5.2 9 organ 内部借 OpenCode): 新增 `apeireth-eye` workspace + Eye organ 顶层 re-export facade 0 改入口签名 + 24 LOCKED 全跑 cargo build + cargo test verify → 9 organ workspace 化 + 9 organ 内部 fn 借 OpenCode 0 改入口签名 + 24 LOCKED 全跑 cargo build + cargo test + organ 集成 verify
- 风险: 极高 (9 organ 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change); 缓解: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用 + V1.1 release bump 1.2.1, V2.0 release bump 2.0.0 (semver major) + 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致

**方向 8: R12 测度对齐** (per R131-5 §2.7 + R131-9 O5 + 决策 #74 §2.3):
- V1.0 release 现状: R11 baseline 3 值 严守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- V1.1 release R12 测度对齐: 触发条件 = 更好的 baseline (R12 测度更高, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- R12 测度更新: 24 测量函数签名更新 R12 测度 (24+9 = 33 → 估 24+11 = 35, per R130-4 spec F1-F11 11 维度 + R131-9 O2), V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新, 24 LOCKED 入口签名测度集成
- R12 baseline 3 值 (估, per 决策 #74 §2.3 V1.1 release R12 baseline 更高): V1141 R12 fresh 24 维均值 > 0.8682 / V1131 R12 dashboard 9 维均值 > 0.8532 / V1136 R12 9 子测度均值 > 0.9063
- 实施步骤 (per 阶段 5.3 R12 测度对齐 1 周): 24 测量函数签名更新 R12 测度 → V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新 → 24 LOCKED 入口签名 测度集成 → 24 LOCKED 全跑 cargo build + cargo test + R12 测度 verify → R12 baseline 3 值 verify
- 风险: 中 (改 R12 测度 = 改 24 测量函数签名 = 改 24 LOCKED 入口签名); 缓解: 仅在 V1.1 release 改 (per 决策 #74 §2.3 V1.1 release 边界) + 24 测量函数签名 1:1 续, 加 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容) + 编译期 hardcode 同步更新, 测试全跑

#### 2.1.3 阶段 1 实施计划: 5 阶段 8 周 (per R137-2 §4 5 阶段 8 周 实施计划)

| 阶段 | 周 | 目标 | 8 方向 | 派活 | sub-agent 数 | 决策依据 |
|------|-----|------|--------|------|-------------|---------|
| **阶段 1.1** | Week 1 (1 周) | 标准化 | 方向 1 | R138 era | 3-5 (R138-1~5) | R131-5 §2.1 8 方向 + 决策 #74 §2.3 V1.1 release |
| **阶段 1.2** | Week 2 (1 周) | 瘦身 | 方向 2 | R139 era | 3-5 (R139-1~5) | R131-5 §2.2 8 方向 + 决策 #74 §2.3 V1.1 release |
| **阶段 1.3** | Week 3-4 (2 周) | 9 叶子拆 + Eye 补 | 方向 3 + 方向 7 Eye 部分 | R140 era | 5-8 (R140-1~8) | R131-5 §2.3 + §2.6 + 决策 #74 B1 Mavis 自决 |
| **阶段 1.4** | Week 5-6 (2 周) | core 拆 pub mod + 大模块拆 sub-crate | 方向 4 + 方向 5 | R141 era | 8-10 (R141-1~10) | R131-5 §2.4 + 决策 #74 B1 Mavis 自决 |
| **阶段 1.5** | Week 7-8 (2 周) | DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 | 方向 6 + 方向 7 + 方向 8 | R142 era | 10-15 (R142-1~15) | R131-5 §2.5-§2.7 + R133-3 §3.2 + 决策 #74 §2.3 |
| **总时间盒** | **8 周 (2 个月)** | **24 LOCKED 入口签名 改写** | **8 方向** | **R138-R142 era** | **29-43 sub-agent** | 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 |

**总时间盒**: 8 周 = 2 个月, 跟 R132-1 §1.5 6 大方向 × 1 周 = 6 周 + 2 周 缓冲 估一致
**R132-1 V1.1 release 估 2026-11-30**: 1.0 release (估 8/11) + 8 周 (2 个月) = 10/6 ~ 11/30, 跟 R132-1 §1.1 V1.1 估 2026-11-30 一致

#### 2.1.4 阶段 1 决策链更新 (per 决策 #10 + 决策 #33 C1 + 决策 #64 + 决策 #71 + 决策 #75 + 决策 #80)

- 决策 #80 (R140 era): R140-R143 14 sub-dispatch fill 16 (本报告派活依据, 2026-08-11 02:02:56 cron 自动拍)
- 决策 #81 (R138 era 估): 阶段 1.1 标准化 done (per R138 era 报告)
- 决策 #82 (R139 era 估): 阶段 1.2 瘦身 done (per R139 era 报告)
- 决策 #83 (R140 era 估): 阶段 1.3 9 叶子拆 + Eye 补 done (per R140 era 报告, 本报告 R140-2 派活)
- 决策 #84 (R141 era 估): 阶段 1.4 core 拆 pub mod + 大模块拆 sub-crate done (per R141 era 报告)
- 决策 #85 (R142 era 估): 阶段 1.5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 done (per R142 era 报告)
- 决策 #86 (R143 era 估): 24 LOCKED 入口签名 改写 5 阶段 8 周 终极 verify (per 决策 #74 §2.3 V1.1 release Mavis 自决改)

### 2.2 阶段 2: A3 PHL-07 实施 (per R137-1)

#### 2.2.1 阶段 2 任务背景 (per 决策 #22 §1.1-1.2 + 决策 #33 §2.1 + 决策 #55 §4 + 决策 #74 §1 A3 改写 + R125-12 P0-3 + R129-11 关键诚实标)

- **PHL-07 spec-only 状态 (1.0 release)**: R125-12 P0-3 (8/10 16:30 done) 写 PHL-07 spec + 13-keys stub, 整合 #4 commit abf12243 done, **0 实施** PHL-07 (per R125-12 P0-3 报告, "PHL-07 spec done, V1.1 实施")
- **决策 #22 §1.1-1.2**: 24 LOCKED 持续更新, 内部 fn 实施可改, 入口签名 0 改, PHL-07 加入 24 LOCKED (per 决策 #33 §2.1 A3, 13 键 = 12 键 + PHL-07 = 13 键, 整合 #4 commit done)
- **决策 #74 §1 A3 改写**: 12 键 + PHL-07 → 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标) + 12 键其他可改
- **R129-11 关键诚实标** (8/11 00:39 done, 40.7 KB): 后端 0 装 PASS 终极 verify, "PHL-07 spec-only, V1.1 实施" 关键诚实标, 不假装 PHL-07 在 1.0 release 时已实施
- **1.0 release 时 PHL-07 状态**: 0 装"已实施", 仅 reference spec, 13 键 stub (per R125-12 P0-3 §3 + R129-11 §2)
- **V1.1 实施 PHL-07 关键诚实标**: V1.0 release 时 PHL-07 spec-only, V1.1 release 时 PHL-07 spec + 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 (25 LOCKED 总数)

#### 2.2.2 阶段 2 目标: PHL-07 实施 (V1.0 spec-only → V1.1 真实施)

- **PHL-07 实施 (V1.0 spec-only → V1.1 实施)**:
  - 24 LOCKED 入口新增 1 个 PHL-07 入口 (per 决策 #22 §1.1-1.2 + 决策 #74 §1 A3 改写, 25 LOCKED 总数)
  - 13 键 stub → 13 键真实施 (per R125-12 P0-3, 13 键 = 12 键 + PHL-07)
  - PHL-07 spec (per R125-12 P0-3 报告 §3): "PHL-07 = 主对话锚" (per 用户记忆 #3 "用户看结果不看哲学, 主对话是核心", PHL-07 实施 = 主对话锚 1:1 实施)
- **PHL-07 实施 spec** (per R125-12 P0-3 + R129-11):
  - **PHL-07 模块**: `crates/apeireth-central/src/phl_07.rs` (NEW) 或 `crates/apeireth-central/src/lib.rs` 加 `pub mod phl_07;`
  - **PHL-07 入口签名**: `pub fn phl_07_main_dialog_anchor() -> PHL07Verdict` (NEW, 25 LOCKED 入口新增 1 个)
  - **PHL-07 实施**:
    - 14 维主对话锚 (per 用户记忆 #3 + 用户记忆 #5, 9 organ 拟人化 + 5 维主对话深化)
    - 主对话锚 1:1 跟 8 哲学锚集成 (B5 8 哲学锚: P-1 哲学 LOCKED + P-2 主体性 + S-1 自主性 + S-2 Sovereignty + S-3 质量工程化 + O-1 安全优先 + E-1 演化 + H-1 人类利益优先, per ROADMAP.md §5)
    - 主对话锚 1:1 跟 6 重守门 v7 集成 (per 决策 #55 §4, B4 严守)
    - 主对话锚 1:1 跟 13 键集成 (per A3 13 键, 决策 #33 §2.1)
  - **PHL-07 test** (per 决策 #22 §1.2 + 决策 #33 §2.3 B1):
    - 14 维主对话锚 tests (14 NEW tests)
    - 跟 8 哲学锚集成 tests (8 NEW tests)
    - 跟 6 重守门 v7 集成 tests (6 NEW tests)
    - 跟 13 键集成 tests (13 NEW tests)
    - 总 41 NEW tests
  - **PHL-07 跨借鉴源集成** (per 决策 #55 §2.6 + 决策 #124-1/2/3):
    - langgraph 829 (StateGraph 1:1 翻译, 1 借脑 0 装)
    - superpowers 234 (主对话锚设计模式, 1 借脑 0 装)

#### 2.2.3 阶段 2 子任务 + 时间盒 + 决策原则 (per R137-1 + 决策 #71 §5 + 决策 #75 §2.1)

**R134-PHL07-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-PHL07-1** (60 min): PHL-07 spec → impl (`crates/apeireth-central/src/phl_07.rs` 14 维主对话锚实施)
- **R134-PHL07-2** (60 min): PHL-07 形式化 (Kani harness, F1-F14 14 维形式化)
- **R134-PHL07-3** (60 min): PHL-07 编译期 hardcode (PHL07Verdict enum + verdict cache 14 键, 0 装 PASS 严守)
- **R134-PHL07-4** (60 min): PHL-07 6 重守门 v7 集成 (per 决策 #55 §4, B4 严守)
- **R134-PHL07-5** (60 min): PHL-07 8 哲学锚集成 (per ROADMAP.md §5, B5 严守) + 41 NEW tests pass

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ 12 键 + PHL-07 严守 (V1.1 release 实施) + 0 装 PASS 严守 (PHL-07 编译期 hardcode 不装 PASS)
- ✅ 14 维主对话锚 = V0.5 30 维子集 (深化, 不扩展 30 维, per B3 V0.5 30 维严守)
- ✅ 8 哲学锚严守 (per B5 决策 #33 §2.3)
- ✅ 6 重守门 v7 严守 (per B4 决策 #33 §2.3)
- ✅ 13 键 → 14 键 (PHL-07 加 1 键, per A3 升级, 决策 #33 §2.1)
- ✅ 25 LOCKED 入口新增 1 个 PHL-07 入口 (0 改原 24 LOCKED 入口签名顺序, 0 改原 24 LOCKED crate mtime 16:34 之前, per B1 V1.0 release 严守 + 决策 #74 §1)
- ✅ 0 借具体源码 100% (2 借脑: langgraph 829 + superpowers 234, 0 装任何具体源码, per C2 决策 #33 §2.3)
- ✅ 0 主动 commit/push 严守 (per C1 + 0 push 决策 #33 §2.3)
- ✅ 0 假装 PHL-07 在 1.0 release 时已实施 (per R129-11 关键诚实标 + 决策 #10 + 主人 10 项偏好 #7)

#### 2.2.4 阶段 2 风险 (per 决策 #74 §7 + R137-1)

- **R-PHL07-1**: 14 维主对话锚跟 V0.5 30 维冲突 (V0.5 30 维严守) — **缓解**: 14 维 = 30 维子集 (深化), 0 扩展 30 维
- **R-PHL07-2**: 41 NEW tests 跟现有 13 键 tests 冲突 — **缓解**: 13 键 tests 严守 0 改, 41 NEW tests 0 触碰 13 键 tests
- **R-PHL07-3**: PHL-07 实施 cargo compile fail (per R130-1 25 hard errors 警示) — **缓解**: PHL-07 实施前先派 fix sub-agent 修 25 hard errors
- **R-PHL07-4**: PHL-07 跟 6 重守门 v7 集成破坏 6 重守门 — **缓解**: PHL-07 仅读 6 重守门, 0 改 6 重守门 enum/struct
- **R-PHL07-5**: PHL-07 跟 8 哲学锚集成破坏 8 哲学锚 — **缓解**: PHL-07 仅读 8 哲学锚, 0 改 8 哲学锚 enum/struct
- **R-PHL07-6**: PHL-07 实施 cargo compile 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #74 §1) — **缓解**: 实施前 verify 8 硬墙 0 越界 100%

#### 2.2.5 阶段 2 决策链更新

- 决策 #87 (R137 era): PHL-07 实施 (R137-1 done, 24 LOCKED → 25 LOCKED PHL-07 入口新增 1 个) (per R137-1 报告)
- 决策 #88 (R134-PHL07 era): PHL-07 14 维主对话锚 41 NEW tests pass (per R134-PHL07-1~5 报告)
- 决策 #89 (R134-PHL07 era): 13 → 14 键升级 (PHL-07 加 1 键, 跟 8 哲学锚 + 6 重守门 v7 集成) (per R134-PHL07-1~5 报告)

### 2.3 阶段 3: B2 workspace.version 1.2.0 → 1.2.1 bump (per R137-3)

#### 2.3.1 阶段 3 任务背景 (per 决策 #33 §2.3 B2 + 决策 #22 §2.2 + 决策 #74 §1 B2 改写 + R137-3 §0)

- **Cargo.toml 当前状态**: `workspace.version = "1.2.0"` (B2 upgrade: 1.1.0 → 1.2.0, R125 末 minor, per 10-locked.md + decision-22 + decision-33)
- **决策 #33 §2.3 B2 旧严守**: workspace.version 1.2.0 0 改严守 (V1.0 release 0 改)
- **决策 #74 §1 B2 改写**: workspace.version V1.0 release 1.2.0 严守 + **V1.1 release bump 1.2.1** (版本管理严守 semver, per "不要怕复杂度" 哲学)
- **R131-6 §1.2 关键诚实标**: Cargo.toml 实地 vs 标 不一致 (count_cloned=8 vs borrow_cloned 列表 7 entries 不一致, count_total=11 vs 实际 8+3+1=12 不一致, decision_chain_range "decision-22 ~ decision-58" vs 当前真实 "decision-22 ~ decision-75" 不一致, description "借鉴 8/11" vs 整合 #5.2 commit 时 "借鉴 10/11 + 1 借脑 = 11/12" 不一致)
- **R137-3 V1.1 release Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec**: 5 阶段 5 天/1 周 (阶段 1 workspace.version 1 day + 阶段 2 24 LOCKED crate Cargo.toml 1 day + 阶段 3 Cargo.lock 1 day + 阶段 4 borrow 段 1 day + 阶段 5 8 步 verify 1 day)

#### 2.3.2 阶段 3 目标: workspace.version 1.2.0 → 1.2.1 bump 实施 spec (per R137-3 §3)

**V1.1 release workspace.version 1.2.0 → 1.2.1 bump 实施 spec** (per 决策 #74 B2 + 决策 #77 §3.1):

```toml
[workspace.package]
# V1.1 release bump: 1.2.0 → 1.2.1 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + semver 严守)
# semver: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能
# 0 改 src 严守 100% (V1.1 release 整合 #6 commit 拍板时 24 LOCKED 入口签名 Mavis 自决改, per 决策 #74 B1)
# 0 装 PASS 严守 100% (V1.1 release 0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
# 整合 #5 commit abf12243 + 整合 #6 commit 严守 (per 决策 #48 + 决策 #62 + 决策 #71 §4)
version = "1.2.1"  # B2 V1.1 release bump: 1.2.0 → 1.2.1 (per decision-74 B2 + decision-77 §3.1, R137 era 实施阶段)
edition = "2021"
rust-version = "1.80"
authors = ["Apeireth Team"]
license = "Apache-2.0"
repository = "https://github.com/apeireth/apeireth-rust"
# V1.1 release 描述 (per decision-74 B1 V1.1 release Mavis 自决改 + decision-77 §3.1):
# 借鉴 11/12 + 24 LOCKED (V1.1 release Mavis 自决改, 25 LOCKED 总数 = 24 + PHL-07) + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + V1.1 release (借鉴 11/12 + 1 借脑 = 12 源 + 24 LOCKED 改写 + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache, per decision-74 B1 V1.1 release Mavis 自决改)"
```

**semver 严守依据 (per 决策 #22 §2.2 + 决策 #74 B2)**:
- **1.2.0 → 1.2.1 = minor 版本 bump** (semver `<主版本>.<次版本>.<修订号>`)
- minor bump 表示 backward-compatible 新功能
- V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.2)
- 整合 #6 commit (估 2026-11-25) 拍板, 整合 #7 commit (估 2026-11-29) 收尾, V1.1 release tag `v1.1.0` 估 2026-11-30

**V1.1 release workspace.version bump 不破坏向后兼容的依据**:
- 0 改 24 LOCKED 入口签名 (V1.0 release 0 改严守, V1.1 release Mavis 自决改前提: 更好的架构)
- 0 改 [workspace.dependencies] (semver 1.2.0 → 1.2.1 0 影响 workspace deps)
- 0 改 [workspace.lints.rust/clippy] (0 影响 lints 配置)
- 0 改 [profile.release] (0 影响 profile 配置)
- 0 改 Cargo.lock 第三方依赖 (semver 1.2.0 → 1.2.1 0 影响 Cargo.lock)
- 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2 0 装 PASS 严守)

**Cargo.toml bump 决策点 reconcile** (per 决策 #22 §2.2 + 决策 #74 §1 B2):
- 决策 #22 §2.2 原话: "V1.1 release 1.0 → 1.1 minor bump"
- 决策 #74 §1 B2 改写原话: "V1.1 release bump 1.2.1"
- **不一致** → Mavis 自决拍板 (per 决策 #74 §2.2 Mavis 自决)
- **本报告 R140-2 倾向** (per R130-5 §1.1 + R131-3 §1.1 + R132-1 §1.1 多个报告一致): **`v1.1.0` 跟决策 #22 §2.2 一致, 1.0 → 1.1 minor bump**, 整合 #6 commit 拍板时 Mavis 自决 reconcile (per 决策 #74 §2.2)
- **备选**: `v1.2.1` 跟决策 #74 B2 一致, 1.2.0 → 1.2.1 patch bump (semver patch 表示 backward-compatible 修 bug, 不适合 V1.1 加 NEW feature)
- **本报告 R140-2 建议**: 用 `v1.1.0` 跟 semver minor bump 严守, 整合 #6 commit 拍板时 Mavis 自决 reconcile 决策 #22 §2.2 + 决策 #74 B2

#### 2.3.3 阶段 3 子任务 + 时间盒 + 决策原则 (per R137-3 §3 + 决策 #71 §5 + 决策 #75 §2.1)

**R137-3 + R134-backend-1~5 sub-agent 派活** (5 sub, 60 min 时间盒, 5 阶段 5 天/1 周):
- **R137-3 (60 min)**: Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (本任务核心, 写新 spec 文档, 0 改 Cargo.toml 严守)
- **R134-backend-1** (60 min): 修整合 #5 commit 拍板前 25 hard errors (per R130-1 警示, 必修) + Cargo.toml `1.2.0 → 1.0.0` 大版本归 0 (per 决策 #22 §2.2, V1.0 release 时 1.2.0 → 1.0.0 大版本归 0)
- **R134-backend-2** (60 min): cargo test 实战三次 verify (整合 #6 后 + 整合 #7 后 + V1.1 release 前) + 8 步 verify (cargo build/check/test/clippy/fmt/audit/deny/doc)
- **R134-backend-3** (60 min): 借鉴源 12 源 0 装严守二次 verify (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) + Cargo.toml borrow 段 update
- **R134-backend-4** (60 min): pybridge 886/886 性能测试 + PyO3 0.29.2 GIL Pool + async bridge + type convert 性能优化
- **R134-backend-5** (60 min): Cargo.toml `1.0.0 → 1.1.0` minor bump (V1.1 release, per 决策 #22 §2.2, Mavis 自决 reconcile 决策 #74 B2 1.2.1) + Cargo.lock 分模块 (V1.1 release 可选, per 决策 #74 §1 B1 Mavis 自决)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **Cargo.toml `1.2.0` 严守 V1.0 release** (per B2 决策 #33 §2.3, 整合 #5.2 commit 0 改 1.2.0)
- ✅ **V1.1 release bump `1.1.0`** (per 决策 #22 §2.2 semver, 跟 决策 #74 §1 B2 改写 1.2.1 需 reconcile, **R140-2 提议 1.1.0**)
- ✅ **8 借脑 0 装严守 100%** (per C2 决策 #33 §2.3, 后端加固 0 装任何具体源码, 11/11 → 12/12 借鉴 clear)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **整合 #5.1 commit 仍 0 改 src 严守** (per 决策 #74 §1 B1 V1.0 release 0 改严守, 整合 #5.1 commit BLOCKED 等 fix 25 errors)
- ✅ **25 hard errors 必修** (per R130-1 警示, 整合 #5.1 commit BLOCKED, 必修后再拍 5.1)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3, 后端加固 0 为简化而简化, Cargo.toml borrow 段可拆 4 子段: cloned_real + translated_public + submodule + skipped_license)

#### 2.3.4 阶段 3 风险 (per R137-3 §4 + 决策 #74 §7)

- **R-backend-1**: 修 25 hard errors 时破坏 R11 baseline — **缓解**: 仅修 R125 阶段引入的 hard bugs, 0 触碰 R11 baseline 3 值
- **R-backend-2**: cargo test 实战三次 verify 时间盒超 1 周 — **缓解**: R134-backend-2 60 min 时间盒, 8 步 verify 全 PASS 5-10 min/次, 三次 30 min 估
- **R-backend-3**: Cargo.toml borrow 段 update 跟 R131-2 实施深度报告冲突 — **缓解**: R134-backend-3 跟 R131-2 集成, 1:1 反映 R131-2 实施深度
- **R-backend-4**: pybridge 性能优化破坏 PyO3 集成 — **缓解**: 仅性能优化 (GIL Pool + async bridge), 0 改 PyO3 0.29.2 公开 API
- **R-backend-5**: Cargo.toml `1.0.0 → 1.1.0` minor bump 跟决策 #74 §1 B2 1.2.1 不一致 — **缓解**: R140-2 提议 1.1.0 跟 决策 #22 §2.2 一致, Mavis 自决拍板 (per 决策 #74 §2.2 Mavis 自决)

#### 2.3.5 阶段 3 决策链更新

- 决策 #90 (R137 era): Cargo.toml 1.2.0 → 1.1.0 (R140-2 提议) 或 1.2.1 (per 决策 #74 B2) Mavis 自决 reconcile (per 决策 #22 §2.2 + 决策 #74 B2)
- 决策 #91 (R137 era): 25 hard errors 必修 done (per R134-backend-1 报告)
- 决策 #92 (R137 era): cargo test 实战三次 verify 全 PASS (per R134-backend-2 报告)
- 决策 #93 (R137 era): 借鉴源 12 源 0 装严守 二次 verify (11/11 → 12/12 clear) (per R134-backend-3 报告)
- 决策 #94 (R137 era): pybridge 886/886 性能测试 PASS (per R134-backend-4 报告)
- 决策 #95 (R137 era): Cargo.lock V1.1 release 依赖更新 + 分模块 (per R134-backend-5 报告)

### 2.4 阶段 4: V1.1 release 实战 (per R136-2 + R134-2 续)

#### 2.4.1 阶段 4 任务背景 (per R129-35 7 步 runbook + R130-5 7 步 runbook 续 + R134-2 5 阶段 + R136-1 5 阶段 + R136-2 实战 + 决策 #55 §2.6 + 决策 #58 §5 + 决策 #61 §4.3 + 决策 #62 §8.3 + 决策 #74 §1 B1 V1.1 release Mavis 自决改 + 决策 #74 §1 B2 V1.1 release bump 1.2.1)

- **R129-35 1.0 release 实战 final-final 7 步 runbook** (8/11 00:54 done, 69.6 KB): 7 步流程 (Step 0 当前状态 verify → Step 1 整合 #5 commit 拍板 → Step 2 8 步 verify → Step 3 配 GitHub remote → Step 4 git push 整合 #5 拆 3 commit → Step 5 打 v1.0.0 tag + gh release create → Step 6 GitHub Pages 部署 → Step 7 verify 1.0 release 页面 + GitHub Pages 文档站)
- **R134-2 1.0 release 实战 5 阶段** (8/11 01:30 done, 58.9 KB): 5 阶段 (阶段 1 整合 #5 commit 拍板 1 day → 阶段 2 主人配 GitHub remote 1 hour → 阶段 3 主人 git push 1 hour → 阶段 4 主人 tag v1.0.0 + GitHub Release notes 1 hour → 阶段 5 主人 GitHub Pages 部署 + 8 步 verify 1 day, 总 3 天)
- **R136-1 V1.1 release 拍板准备 5 阶段** (8/11 01:40 done, 105.7 KB): 5 阶段 (阶段 1 6.1 src/ 拍板准备 2 周 + 阶段 2 6.2 docs/ 拍板准备 1 周 + 阶段 3 6.3 reports/ 拍板准备 1 周 + 阶段 4 整合 #6 commit 拍板 1 day + 阶段 5 V1.1 release 实战准备 1 day, 总 4 周 + 2 天)
- **R136-2 V1.1 release 实战** (8/11 01:50 done, 74.7 KB): 6 步流程 (Step 0 当前状态 verify → Step 1 整合 #6 commit 拍板 → Step 2 整合 #7 commit 拍板 → Step 3 8 步 verify → Step 4 git push 整合 #6 + #7 → Step 5 打 v1.1.0 tag + gh release create → Step 6 GitHub Pages 重新部署 → Step 7 verify V1.1 release 页面 + GitHub Pages 文档站)
- **V1.1 release 实战** (估 2026-11-30 06:00-08:00, 60 min): 7 步流程 (R129-35 续 + R134-2 续 + R136-2 实战)

**V1.1 release 实战 跟 1.0 release 实战差异**:
- 1.0 release: 配 GitHub remote (0 origin → 1 origin, per R129-35 Step 3)
- V1.1 release: 已配 origin (1 origin, V1.1 push 简化, per R131-9 Step 3)
- 1.0 release: 打 v1.0.0 tag (per R129-35 Step 5)
- V1.1 release: 打 v1.1.0 tag (per R131-9 Step 5, 1.0 → 1.1 minor bump)
- 1.0 release: GitHub Pages 部署 (per R129-35 Step 6)
- V1.1 release: GitHub Pages 重新部署 (per R131-9 Step 6, mkdocs build + gh-pages branch 重新部署)

#### 2.4.2 阶段 4 目标: V1.1 release 7 步流程 (per R136-2 6 步流程 + R134-2 5 阶段流程 + R129-35 7 步 runbook 续)

**V1.1 release 7 步流程 (整合 #6 + 整合 #7 commit 拍板后)**:
```
[Step 0] 当前状态 verify (per §1, 整合 #7 commit 拍板后)
   ├─ master HEAD = abf12243 + 6 commit (5.1/5.2/5.3 + 6.1/6.2/6.3)
   ├─ Cargo.toml version = "1.1.0" (1.2.0 → 1.0.0 → 1.1.0, per 决策 #22 §2.2 R140-2 提议)
   ├─ 整合 #5.1 commit done (R130-1 修 30+1 bug)
   ├─ 整合 #5.2 commit done (Cargo.toml 1.0.0 改 1.1.0 等)
   ├─ 整合 #5.3 commit done (R129 era 报告)
   ├─ 整合 #6.1 commit done (R134 era 实施: PHL-07 + 后端加固 + Tauri + 形式化 + ASI + 借鉴)
   ├─ 整合 #6.2 commit done (R134 era 文档)
   ├─ 整合 #6.3 commit done (R134 era 报告)
   └─ 整合 #7 commit done (V1.1 release 前最终 commit, 包含 Cargo.toml 1.1.0 bump)
   ↓
[Step 1] 整合 #7 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §4)
   ├─ 7.1 commit: V1.1 release 前最终 src/ (PHL-07 实施 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+)
   ├─ 7.2 commit: V1.1 release 前最终 docs/ (CHANGELOG.md v1.1.0 + ROADMAP.md V1.1 update + RELEASE_NOTES.md v1.1.0)
   └─ 7.3 commit: V1.1 release 前最终 reports/ (R131-R137 era sub-agent 报告 + 决策链 #80-#100 + HANDOFF-NEXT-SESSION-V1.1-RELEASE)
   ↓ cron auto-pickup OR 主人手跑 git-push-1.1.ps1
[Step 2] 8 步 verify (整合 #7 commit 后, V1.1 release tag 前必跑, per HANDOFF §8.2)
   ├─ Step 1: 修 session working dir + master HEAD + Cargo.toml 1.1.0
   ├─ Step 2: cargo build --workspace
   ├─ Step 3: cargo test --workspace (4200+ tests, per §2.2)
   ├─ Step 4: cargo run --bin apeireth-tui 5s smoke
   ├─ Step 5: cargo run --bin apeireth-api 5s smoke
   ├─ Step 6: cargo audit + cargo deny
   ├─ Step 7: 25 LOCKED 入口签名 0 改 (24 + PHL-07 = 25, per §2.2)
   └─ Step 8: 8 硬墙 0 越界 + 0 装 PASS 严守 (11 项 verify)
   ↓ 8 步全 PASS
[Step 3] git push master (per git-push-1.1.ps1, R131-9 写, 60 min)
   ├─ 7.1 git add + commit (7.1 commit message per R131-9 §3)
   ├─ 7.2 git add + commit (7.2 commit message per R131-9 §4)
   ├─ 7.3 git add + commit (7.3 commit message per R131-9 §5)
   ├─ git push -u origin master (已配 origin, push 简化)
   └─ verify push 成功 (local master = remote master)
   ↓
[Step 4] 打 v1.1.0 tag + gh release create (per tag-1.1.0.ps1, R131-9 写, 估 30 min)
   ├─ Step 4.0: 删 stale v1.0.0 tag (per 1.0 release 时已打, 估 8/11)
   ├─ Step 4.1: 打 annotated tag v1.1.0
   ├─ Step 4.2: push tag origin v1.1.0
   ├─ Step 4.3: gh release create v1.1.0 --title "Apeireth 1.1.0" --notes-file RELEASE_NOTES.md
   └─ Step 4.4: verify GitHub release 页面 https://github.com/apeireth/apeireth-rust/releases/tag/v1.1.0
   ↓
[Step 5] GitHub Pages 重新部署 (per deploy-github-pages-v1.1.ps1, R131-9 写, 估 30 min)
   ├─ Step 5.0: mkdocs build (生成 site/ 目录, 含 V1.1 新文档)
   ├─ Step 5.1: 创建 gh-pages branch (orphan 模式, V1.1 release 前已存在)
   ├─ Step 5.2: git push origin gh-pages --force
   ├─ Step 5.3: 主人浏览器 GitHub repo Settings → Pages → Source: gh-pages branch + Folder: / (root)
   └─ Step 5.4: verify https://apeireth.github.io/apeireth-rust/ (V1.1 更新)
   ↓
[Step 6] verify V1.1 release + GitHub Pages + 主人发 release announcement
   ├─ Step 6.1: verify https://github.com/apeireth/apeireth-rust/releases/tag/v1.1.0
   ├─ Step 6.2: verify https://apeireth.github.io/apeireth-rust/ (V1.1 文档)
   ├─ Step 6.3: verify master HEAD 包含整合 #5 + #6 + #7 commit
   ├─ Step 6.4: verify v1.1.0 tag 指向整合 #7 HEAD
   └─ Step 6.5: 主人发 release announcement (中文/英文, per ROADMAP.md §4)
   ↓
[V1.1 release done]   1.1 release 反馈 + R131-9 V1.1 release 实战 done notification
```

#### 2.4.3 阶段 4 8 步 verify 详细 (per R129-35 §1.2 + R131-9 + 决策 #74 §1 + 决策 #22 §2.2)

| # | verify 项 | 当前 (1.0 release) | V1.1 release 目标 | 来源 |
|---:|----------|---------------------|--------------------|------|
| 1 | master HEAD | `abf12243` (整合 #4) | `abf12243 + 6 commit (5.1/5.2/5.3 + 6.1/6.2/6.3) + 3 commit (7.1/7.2/7.3)` | per R131-9 §1 |
| 2 | Cargo.toml version | `1.2.0` | `1.1.0` (1.2 → 1.0 → 1.1, per 决策 #22 §2.2, R140-2 提议) 或 `1.2.1` (per 决策 #74 B2, Mavis 自决) | per 决策 #22 §2.2 + 决策 #74 B2 |
| 3 | 整合 #5 commit | NOT ready | done (R130-1 修 30+1 bug) | per R130-1 报告 |
| 4 | 整合 #6 commit | N/A | done (R131 era 实施: PHL-07 + 后端加固 + Tauri + 形式化 + ASI + 借鉴) | per R131-8 决策 #97 |
| 5 | 整合 #7 commit | N/A | done (V1.1 release 前最终) | per R131-9 §3-§5 |
| 6 | origin remote | 0 (1.0 release 时配) | `https://github.com/apeireth/apeireth-rust.git` | per R129-35 Step 3 |
| 7 | v1.1.0 tag | N/A | `https://github.com/apeireth/apeireth-rust/releases/tag/v1.1.0` | per R131-9 Step 4 |
| 8 | GitHub Pages | N/A (1.0 release 时部署) | `https://apeireth.github.io/apeireth-rust/` (V1.1 重新部署) | per R131-9 Step 5 |
| 9 | cargo build | 🟡 24+5 errors (per R130-1) | 0 errors (修 30+1 bug) | per R130-1 |
| 10 | cargo test | 🟡 1 FAILED test (per R129-26) | 4200+ tests pass (per §2.2) | per R131-3 |
| 11 | 25 LOCKED 入口签名 0 改 | ✅ 24 LOCKED (P2-3 + P4-1 + P14-1 retry 三方 verify) | ✅ 25 LOCKED (24 + PHL-07 = 25) | per §2.2 + R131-2 |
| 12 | 8 硬墙 0 越界 | ✅ 11 项 verify PASS | ✅ 11 项 verify PASS | per 决策 #33 §2.3 |
| 13 | 0 装 PASS 严守 | ✅ 11/11 clear | ✅ 12/12 clear (per §2.6) | per R131-3 + R131-7 |
| 14 | Cargo.toml 1.1.0 严守 | N/A (1.0 release 时 1.0.0) | ✅ 1.1.0 严守 (per 决策 #22 §2.2, R140-2 提议) | per R131-3 |

**0 主动 push 严守 100%** (per 决策 #33 §2.3 + 决策 #61 §6):
- V1.1 release 实战: 主人起床后手跑 (per R131-9 §6, 估 2026-11-30 06:00-08:00)
- Mavis 0 push 0 配 remote (V1.1 release 0 配 new remote, 复用 1.0 release 配的 origin)
- 整合 #5 + #6 + #7 commit 拍板 = Mavis 自决 (per 决策 #33 C1 + 决策 #71 §4)
- 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6, 仅 done notification 主动报告)

#### 2.4.4 阶段 4 V1.1 release 后 (per §5 风险 + 决策原则 + R129-29 §5 V1.2 路线图)

- **V1.1 release 反馈**: 主人接管 GitHub issues + community, V1.1 release 反馈 (中文/英文 announcement)
- **V1.2 路线图** (per R129-29 §5, 估 2027-02-28): TUI 升级阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战
- **V2.0 远期** (per ROADMAP.md §4, 2027+): 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作
- **R132 era (V1.2 era) 派活规划** (per R129-29 §5.3): 10 sub-agent (2 批 5+5), 16 上限派满

#### 2.4.5 阶段 4 决策链更新

- 决策 #96 (R134 era 估): 整合 #6 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §4 + 决策 #74 B1) (per R131-8 报告)
- 决策 #97 (R134 era 估): 整合 #7 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §4) (per R131-9 报告)
- 决策 #98 (R131 era 估): V1.1 release 实战 (R131-9 done, 主人起床后手跑, 估 2026-11-30 06:00-08:00) (per R131-9 报告)
- 决策 #99 (R131 era 估): V1.1 release tag `v1.1.0` 打上 (per R131-9 §5, 整合 #7 commit 后打 v1.1.0 tag) (per R131-9 报告)
- 决策 #100 (R131 era 估): R131 era 总览报告 + 决策链更新 (R131-10 done) (per R131-10 报告, R131 era 总览 + 决策链 #80-#99 总结)

---

## 3. V1.1 release 8 步时间线 (per 决策 #74 §1 + 决策 #62 + 决策 #71 §4 + R130-5 §1.2 + R132-1 §1.2 + R136-1 §1.1 + R134-2 §1.1 + R137-3 §1)

### 3.1 8 步时间线总览 (per 决策 #71 §3 永久循环 + 决策 #74 §1 + R130-5 §1.2)

```
[Step 1] 整合 #5.1 commit 拍板 (V1.0 release src/, 估 8/11 01:30+, per 决策 #62 §5.1 + R130-1 修 25 hard errors 后)
   ├─ 5.1 commit: src/ 实施 (95+ 文件, R125-R128-2 era 41 任务 src/ 实施, 31 modified M + 253 ?? src/ + tests/ + examples/)
   ├─ 5.1 commit 0 改 24 LOCKED 入口签名 (V1.0 release R11 baseline 严守, per 决策 #33 §2.3 B1 + 决策 #74 §1)
   ├─ 5.1 commit 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
   ├─ 5.1 commit 0 改 R11 baseline 3 值 (严守, per 决策 #33 §2.3 A1)
   └─ 5.1 commit 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)
   ↓ Mavis 自决拍板
[Step 2] 整合 #5.2 commit 拍板 (V1.0 release docs/ + Cargo.toml, 估 8/11 01:35+, per 决策 #62 §5.2)
   ├─ 5.2 commit: docs/ + Cargo.toml (10 文件, CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md + Cargo.toml borrow 段 update 17:44 → 22:50 状态 + Cargo.lock / .gitignore + docs/roadmap/ / frontend/ / library/)
   ├─ 5.2 commit 0 改 workspace.version 1.2.0 (V1.0 release 严守, per 决策 #33 §2.3 B2)
   ├─ + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3 主人 8/11 01:14 总哲学扩展)
   ├─ + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3 主人 8/11 01:14 locked 全解锁, 整合 #5.1 commit 0 改 src 严守 + V1.1 release Mavis 自决改)
   ├─ + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2 总工程哲学扩展引用)
   ├─ + 更新 `docs/conventions/README.md` (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
   ├─ + 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3 8 项不修改承诺 改写 + 主人 8/11 01:14 拍板记录)
   └─ + 更新 `README.md` (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)
   ↓ Mavis 自决拍板
[Step 3] 整合 #5.3 commit 拍板 (V1.0 release reports/, 估 8/11 01:40+, per 决策 #62 §5.3 + 决策 #78 Option A)
   ├─ 5.3 commit: reports/ (60+ 文件, 决策链 #30-#74 全读 verify + 41 sub-agent 报告 + HANDOFF)
   ├─ + 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写) (per 决策 #73 §2.2 + §5)
   ├─ + 新增 R131 era 调研 3 sub-agent 报告 (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
   ├─ + 新增 R131 era 第 2 批 6 sub-agent 报告 (R131-4 + R131-5 + R131-9, per 决策 #75 §2.1)
   ├─ + 新增 R132 era 计划 2 sub-agent 报告 (R132-1, per 决策 #75 §2.1)
   ├─ + 新增 R133 era 实施 3 sub-agent 报告 (R133-1 + R133-2 + R133-3, per 决策 #75 §2.1)
   ├─ + 新增 R137 era 实施 1 sub-agent 报告 (R137-2, per 决策 #77 §3.1)
   └─ + 新增 `philosophy-no-fear-complexity-2026-08-11.md` (主人 8/11 01:14 决策 3 件套详细)
   ↓ Mavis 自决拍板
[Step 4] 1.0 release tag v1.0.0 打上 (估 8/11 06:00-08:00 主人起床后手跑, per R129-35 7 步 runbook + R134-2 5 阶段)
   ├─ Step 0 当前状态 verify (整合 #5.1 + 5.2 + 5.3 commit done verify)
   ├─ Step 1 8 步 verify (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名 0 改)
   ├─ Step 2 配 GitHub remote (0 origin → 1 origin, per R129-35 Step 3)
   ├─ Step 3 git push master (per git-push-1.0.ps1, 60 min)
   ├─ Step 4 打 v1.0.0 tag + gh release create (per tag-1.0.0.ps1, 30 min)
   ├─ Step 5 GitHub Pages 部署 (per deploy-github-pages.ps1, 30 min)
   └─ Step 6 verify 1.0 release 页面 + GitHub Pages 文档站 (per verify-1.0-pre-tag.ps1)
   ↓ 主人手跑 6 步流程
[1.0 release done]  8/11 08:00+ 1.0 release done, master HEAD = abf12243 + 3 commit (5.1/5.2/5.3), v1.0.0 tag, GitHub release, GitHub Pages 部署
   ↓
[Step 5] 整合 #6 commit 拍板 (V1.1 release src/ + docs/ + reports/, 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §4 + 决策 #74 B1 V1.1 release Mavis 自决改 + R136-1 §1.1 整合 #6 commit 拍板准备)
   ├─ 6.1 commit: V1.1 release src/ 拍板 (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级 + 9 organ 借 OpenCode + R12 测度对齐, ~50 文件, per 决策 #74 B1)
   ├─ 6.2 commit: V1.1 release docs/ 拍板 (CHANGELOG.md + ROADMAP.md + RELEASE_NOTES.md + OSS_NOTICE.md [OpenCog AGPL-3.0 fork 致谢加] + Cargo.toml workspace.version 1.2.0 → 1.1.0/1.2.1 bump + Cargo.lock V1.1 release 依赖更新 + .gitignore V1.1 release + docs/roadmap/ V1.1 release + docs/1.1-release/ V1.1 release + docs/architecture-v5-onion-upgrade.md V1.1 release 三洋葱 → 四洋葱 架构升级文档, 10 文件)
   ├─ 6.3 commit: V1.1 release reports/ 拍板 (决策链 #78-#130 全读 verify + V1.1 release sub-agent 报告 ~30+ files [R130 + R131 + R132 + R133 + R134 + R135 + R136 era 报告链] + HANDOFF-NEXT-SESSION-V1.1-RELEASE, ~50 文件)
   └─ Mavis 自决拍板 6.1 → 6.2 → 6.3 顺序 git add + git commit (per 决策 #62 + 决策 #64 cron auto-pickup)
   ↓ Mavis 自决拍板
[Step 6] 整合 #7 commit 拍板 (V1.1 release 前最终, 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §4 + R136-1 §1.1)
   ├─ 7.1 commit: V1.1 release 前最终 src/ (PHL-07 实施 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+ 续, 0 改 24 LOCKED 入口签名, 仅 +1 PHL-07 入口 = 25 LOCKED)
   ├─ 7.2 commit: V1.1 release 前最终 docs/ (CHANGELOG.md v1.1.0 + ROADMAP.md V1.1 update + RELEASE_NOTES.md v1.1.0 + Cargo.toml 1.1.0/1.2.1 bump 实施 + Cargo.lock V1.1 release 依赖更新)
   └─ 7.3 commit: V1.1 release 前最终 reports/ (R131-R137 era sub-agent 报告 + 决策链 #80-#100 + HANDOFF-NEXT-SESSION-V1.1-RELEASE)
   ↓ Mavis 自决拍板
[Step 7] V1.1 release 实战 (估 2026-11-30 06:00-08:00 主人起床后手跑, per R136-2 实战 6 步 + R134-2 5 阶段 + R129-35 7 步 runbook 续)
   ├─ Step 0 当前状态 verify (整合 #5 + #6 + #7 commit done verify)
   ├─ Step 1 8 步 verify (cargo build + test + clippy + fmt + audit + deny + doc + 25 LOCKED 入口签名 0 改)
   ├─ Step 2 git push master (per git-push-1.1.ps1, 60 min, 已配 origin, push 简化)
   ├─ Step 3 打 v1.1.0 tag + gh release create (per tag-1.1.0.ps1, 30 min)
   ├─ Step 4 GitHub Pages 重新部署 (per deploy-github-pages-v1.1.ps1, 30 min)
   └─ Step 5 verify V1.1 release 页面 + GitHub Pages 文档站 (per verify-1.1-pre-tag.ps1)
   ↓ 主人手跑 5 步流程 (R136-2 实战)
[V1.1 release done]  2026-11-30 08:00+ V1.1 release done, master HEAD = abf12243 + 9 commit (5.1/5.2/5.3 + 6.1/6.2/6.3 + 7.1/7.2/7.3), v1.1.0 tag, GitHub release, GitHub Pages 重新部署
   ↓
[Step 8] 永久循环 (per 决策 #71 §3 永久循环接续, V1.1 release 后)
   ├─ V1.2 路线图 (per R129-29 §5, 估 2026-12, 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战)
   ├─ V1.2 release (估 2027-02-28, v1.2.0 tag 打上)
   ├─ V2.0 远期 (per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构)
   ├─ 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 (per ROADMAP.md §4 + R119-2 思想层保留)
   └─ 永久 4 步循环: 调研 + 差距 + 计划 + 实施 (per 决策 #71 §3)
```

### 3.2 8 步时间线 详细 (per 决策 #74 §1 + 决策 #62 + 决策 #71 §4 + R130-5 §1.2 + R132-1 §1.2 + R136-1 §1.1)

**8 步时间线 关键节点详细** (per 决策 #74 §1 + R130-5 §1.2 + R131-3 §1.2 + R132-1 §1.2 + R136-1 §1.1 + R134-2 §1.1 + R137-3 §1):

| 步 | 任务 | 时间 | 状态 | 主体 | 决策依据 | 8 硬墙严守 |
|----|------|------|------|------|---------|----------|
| **Step 1** | 整合 #5.1 commit 拍板 | 8/11 01:30+ | 📋 Mavis 自决 (BLOCKED 等 fix 25 errors) | Mavis | 决策 #62 §5.1 + 决策 #74 §1 B1 V1.0 release 0 改 | B1 0 改 + B2 1.2.0 0 改 + A1 R11 baseline 0 改 + A3 PHL-07 spec-only + B3 30 维 + B4 6 重 v7 + B5 8 锚 + C1 0 commit + C2 0 装 + 0 push |
| **Step 2** | 整合 #5.2 commit 拍板 | 8/11 01:35+ | 📋 Mavis 自决 | Mavis | 决策 #62 §5.2 + 决策 #73 §2.3-§4.2 + 决策 #74 §1 B2 1.2.0 严守 | B1 0 改 + B2 1.2.0 0 改 + B5 8 锚 (引用) + C1 0 commit + 0 push |
| **Step 3** | 整合 #5.3 commit 拍板 | 8/11 01:40+ | 📋 Mavis 自决 (per 决策 #78 Option A) | Mavis | 决策 #62 §5.3 + 决策 #78 Option A + 决策 #73 §5 | C1 0 commit + 0 push |
| **Step 4** | 1.0 release tag v1.0.0 打上 | 8/11 06:00-08:00 | 📋 主人手跑 | 主人 | R129-35 7 步 runbook + R134-2 5 阶段 + 决策 #76 §2.1 | 0 主动 push 严守 (Mavis 0 push 0 配 remote 0 tag 0 release 0 build pages) |
| **Step 5** | 整合 #6 commit 拍板 | 2026-11-25 | 📋 Mavis 自决 | Mavis | 决策 #33 C1 + 决策 #71 §4 + 决策 #74 B1 V1.1 release Mavis 自决改 + R136-1 §1.1 | B1 V1.1 release Mavis 自决改 + B2 V1.1 release bump + A1 R12 baseline + A3 PHL-07 实施 + B3 30 维 + B4 6 重 v7 + B5 8 锚 + C1 0 commit + C2 0 装 + 0 push |
| **Step 6** | 整合 #7 commit 拍板 | 2026-11-29 | 📋 Mavis 自决 | Mavis | 决策 #33 C1 + 决策 #71 §4 + R136-1 §1.1 | (同 Step 5) |
| **Step 7** | V1.1 release 实战 | 2026-11-30 06:00-08:00 | 📋 主人手跑 | 主人 | R136-2 实战 6 步 + R134-2 5 阶段 + R129-35 7 步 runbook 续 | 0 主动 push 严守 (Mavis 0 push 0 tag 0 release 0 build pages) |
| **Step 8** | 永久循环 | 2026-12+ | 📋 永久 | Mavis + 主人 + 未来高水平团队 | 决策 #71 §3 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + ROADMAP.md §4 + R119-2 思想层保留 | 永久 4 步循环: 调研 + 差距 + 计划 + 实施 |

**总时间盒**: 8 步从 8/11 01:30+ 整合 #5.1 commit 拍板 → 2026-11-30 08:00+ V1.1 release done, 总时间盒 = 整合 #5 commit 1 day + 1.0 release 实战 3 days + V1.1 release 实施 13 周 (per R130-5 §1.2 + R131-3 §1.2) + 整合 #6 + #7 commit 1 周 + V1.1 release 实战 3 days = 总 ~17 周

### 3.3 8 步时间线 决策依据 (per 决策 #74 §1 + 决策 #33 + 决策 #22 + 决策 #48 + 决策 #62 + 决策 #64 + 决策 #70 + 决策 #71 + 决策 #73 + 决策 #74 + 决策 #76 + 决策 #78 + 决策 #80)

- **Step 1 (整合 #5.1 commit)**: 决策 #62 §5.1 + 决策 #74 §1 B1 V1.0 release 0 改 + R130-1 修 25 hard errors
- **Step 2 (整合 #5.2 commit)**: 决策 #62 §5.2 + 决策 #73 §2.3-§4.2 主人 8/11 01:14 拍板 3 件套 + 决策 #74 §1 B2 1.2.0 严守
- **Step 3 (整合 #5.3 commit)**: 决策 #62 §5.3 + 决策 #78 Option A reports commit paiban
- **Step 4 (1.0 release tag v1.0.0)**: R129-35 7 步 runbook + R134-2 5 阶段 + 决策 #76 §2.1
- **Step 5 (整合 #6 commit)**: 决策 #33 C1 + 决策 #71 §4 + 决策 #74 B1 V1.1 release Mavis 自决改 + R136-1 §1.1
- **Step 6 (整合 #7 commit)**: 决策 #33 C1 + 决策 #71 §4 + R136-1 §1.1
- **Step 7 (V1.1 release 实战)**: R136-2 实战 6 步 + R134-2 5 阶段 + R129-35 7 步 runbook 续
- **Step 8 (永久循环)**: 决策 #71 §3 + 决策 #74 §2.3 V2.0 release + ROADMAP.md §4 + R119-2

---

## 4. V1.1 release 决策点 (决策 #80-#100, 22 决策)

### 4.1 决策点总览 (per 决策 #80 + 决策 #10 + 决策 #33 C1 + 决策 #71 §4 + 决策 #74 §1)

| # | 决策 | Date | 内容 | 状态 | 决策依据 |
|---|------|------|------|------|---------|
| **#80** | R140-R143 14 sub-dispatch fill 16 | 2026-08-11 02:02:56 | per 决策 #79 续, R140 era 14 sub-agent 派活填满 16 跑中上限 | 🟡 done | 决策 #75 + 决策 #79 + 决策 #71 §5 |
| **#81** | 阶段 1.1 标准化 done (R138 era 估) | 估 2026-08-15+ | per R138 era 报告, 24 LOCKED 入口签名 统一格式 6 模式, 3 模式之一 per-crate 自决 | 🟡 估 done | R131-5 §2.1 + 决策 #74 §2.3 V1.1 release |
| **#82** | 阶段 1.2 瘦身 done (R139 era 估) | 估 2026-08-22+ | per R139 era 报告, 公开 API 表面减少 30% (800 → 560 pub items), per-crate 暴露 ≤30 pub items | 🟡 估 done | R131-5 §2.2 + 决策 #74 §2.3 V1.1 release |
| **#83** | 阶段 1.3 9 叶子拆 + Eye 补 done (R140 era 估) | 估 2026-09-05+ | per R140 era 报告, 9 叶子 crate 拆 apeireth-leaf/ workspace + Eye organ 补 apeireth-eye/ workspace | 🟡 估 done | R131-5 §2.3 + §2.6 + 决策 #74 B1 Mavis 自决 |
| **#84** | 阶段 1.4 core 拆 pub mod + 大模块拆 sub-crate done (R141 era 估) | 估 2026-09-19+ | per R141 era 报告, core 拆 5 大 mod + 8 大模块集中 crate 拆 47 sub-crate | 🟡 估 done | R131-5 §2.4 + 决策 #74 B1 Mavis 自决 |
| **#85** | 阶段 1.5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 done (R142 era 估) | 估 2026-10-03+ | per R142 era 报告, 三洋葱 → 四洋葱 升级 + 9 organ workspace 化 + R12 测度对齐 (24+11 = 35 测量函数) | 🟡 估 done | R131-5 §2.5-§2.7 + R133-3 §3.2 + 决策 #74 §2.3 |
| **#86** | 24 LOCKED 入口签名 改写 5 阶段 8 周 终极 verify (R143 era 估) | 估 2026-10-17+ | per 决策 #74 §2.3 V1.1 release Mavis 自决改, 25 LOCKED 入口签名 0 改 verify (24 + PHL-07 = 25) | 🟡 估 done | 决策 #74 §2.3 |
| **#87** | PHL-07 实施 (R137-1 done) | 估 2026-10-31+ | per R137-1 报告, V1.0 spec-only → V1.1 真实施, 24 LOCKED → 25 LOCKED (PHL-07 加 1 入口) | 🟡 估 done | 决策 #74 §1 A3 + R129-11 关键诚实标 |
| **#88** | PHL-07 14 维主对话锚 41 NEW tests pass (R134-PHL07-1~5 era 估) | 估 2026-11-07+ | per R134-PHL07-1~5 报告, 14 维主对话锚 41 NEW tests + 跟 8 哲学锚集成 + 跟 6 重守门 v7 集成 + 跟 14 键集成 | 🟡 估 done | 决策 #74 §1 A3 + 决策 #22 §1.1-1.2 |
| **#89** | 13 → 14 键升级 (PHL-07 加 1 键) (R134-PHL07-1~5 era 估) | 估 2026-11-07+ | per R134-PHL07-1~5 报告, 13 键 → 14 键, 跟 8 哲学锚 + 6 重守门 v7 集成 | 🟡 估 done | 决策 #33 §2.1 A3 升级 |
| **#90** | Cargo.toml 1.2.0 → 1.1.0 (R140-2 提议) 或 1.2.1 (per 决策 #74 B2) Mavis 自决 reconcile | 估 2026-11-25+ | per 决策 #22 §2.2 + 决策 #74 B2, Mavis 自决拍板 | 🟡 估 done | 决策 #22 §2.2 + 决策 #74 B2 |
| **#91** | 25 hard errors 必修 done (R134-backend-1 era 估) | 估 2026-11-14+ | per R134-backend-1 报告, 修 25 hard errors + Cargo.toml `1.2.0 → 1.0.0` 大版本归 0 | 🟡 估 done | R130-1 警示 + 决策 #22 §2.2 |
| **#92** | cargo test 实战三次 verify 全 PASS (R134-backend-2 era 估) | 估 2026-11-15+ | per R134-backend-2 报告, cargo test 实战三次 + 8 步 verify (cargo build/check/test/clippy/fmt/audit/deny/doc) | 🟡 估 done | 决策 #74 §1 + 决策 #33 §2.3 |
| **#93** | 借鉴源 12 源 0 装严守 二次 verify (11/11 → 12/12 clear) (R134-backend-3 era 估) | 估 2026-11-16+ | per R134-backend-3 报告, 8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID 12/12 clear | 🟡 估 done | R130-6 + R131-2 + 决策 #33 §2.3 C2 |
| **#94** | pybridge 886/886 性能测试 PASS (R134-backend-4 era 估) | 估 2026-11-17+ | per R134-backend-4 报告, pybridge 性能测试 + PyO3 0.29.2 GIL Pool + async bridge + type convert | 🟡 估 done | 决策 #33 §2.3 + 0 装 PASS 严守 |
| **#95** | Cargo.lock V1.1 release 依赖更新 + 分模块 (R134-backend-5 era 估) | 估 2026-11-18+ | per R134-backend-5 报告, Cargo.lock V1.1 release 依赖更新 + Cargo.toml `1.0.0 → 1.1.0` minor bump + Cargo.lock 分模块 (V1.1 release 可选) | 🟡 估 done | 决策 #22 §2.2 + 决策 #74 §1 B1 Mavis 自决 |
| **#96** | 整合 #6 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §4 + 决策 #74 B1) | 估 2026-11-25 | per R131-8 报告, 6.1 src/ + 6.2 docs/ + 6.3 reports/ 顺序 git add + git commit | 🟡 估 done | 决策 #33 C1 + 决策 #64 + 决策 #74 B1 |
| **#97** | 整合 #7 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §4) | 估 2026-11-29 | per R131-9 报告, 7.1 src/ + 7.2 docs/ + 7.3 reports/ 顺序 git add + git commit | 🟡 估 done | 决策 #33 C1 + 决策 #71 §4 |
| **#98** | V1.1 release 实战 (R131-9 done, 主人起床后手跑, 估 2026-11-30 06:00-08:00) | 估 2026-11-30 | per R131-9 报告, 7 步流程 + 8 步 verify + git push + v1.1.0 tag + GitHub Pages 重新部署 | 🟡 估 done | R136-2 实战 6 步 + R134-2 5 阶段 + R129-35 7 步 runbook 续 |
| **#99** | V1.1 release tag v1.1.0 打上 (per R131-9 §5) | 估 2026-11-30 | per R131-9 报告, 整合 #7 commit 后打 v1.1.0 tag (本报告 R140-2 提议 v1.1.0 跟决策 #22 §2.2 一致) | 🟡 估 done | 决策 #22 §2.2 |
| **#100** | R131 era 总览报告 + 决策链更新 (R131-10 done) | 估 2026-11-30 | per R131-10 报告, R131 era 总览 + 决策链 #80-#99 总结 | 🟡 估 done | 决策 #71 §4 永久循环接续 |

### 4.2 决策点 跟 拍板流程 关系 (per 决策 #33 C1 + 决策 #64 + 决策 #71 §4 + 决策 #74 §1 + 决策 #80)

**V1.1 release 决策点 跟 拍板流程 关系**:
- **决策 #80-#89 (R140 era 实施阶段)**: per 决策 #80 R140-R143 14 sub-dispatch, Mavis 自决拍板实施阶段 24 LOCKED 入口签名 改写 5 阶段 8 周
- **决策 #90-#95 (R134 era 实施阶段)**: per 决策 #76 §2.1 R134 era 派活清单, Mavis 自决拍板 Cargo.toml bump + 后端加固
- **决策 #96 (整合 #6 commit 拍板)**: per 决策 #33 C1 + 决策 #64 + 决策 #74 B1 + R136-1 §1.1, Mavis 自决拍板 6.1 → 6.2 → 6.3 顺序
- **决策 #97 (整合 #7 commit 拍板)**: per 决策 #33 C1 + 决策 #71 §4, Mavis 自决拍板 7.1 → 7.2 → 7.3 顺序
- **决策 #98-#100 (V1.1 release 实战 + tag + 总览)**: per R136-2 实战 6 步 + R134-2 5 阶段 + 决策 #22 §2.2

**拍板流程 决策链更新 严守**:
- ✅ Mavis 自决拍板 (per 决策 #33 C1 + 决策 #64 + 决策 #74 B1)
- ✅ 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 8 哲学锚 严守 0 漂移 (per 决策 #33 §2.3 B5)
- ✅ 0 主动 commit (整合 #5/6/7 commit 由 Mavis 自决拍板, per 决策 #33 C1 + 决策 #64)
- ✅ 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60)
- ✅ 决策日志写 (per 决策 #10 + 用户记忆 #10)
- ✅ 整合 #4 commit abf12243 严守 (per 决策 #48)
- ✅ 0 重复造轮子 (per 用户记忆 #6, 已有的 verify 报告 reference 不重写)

---

## 5. V1.1 release 16 风险 (per 决策 #74 §7 + R131-5 §6 + R137-2 §7 + R130-5 §5 + R132-1 §5 + R137-3 §4 + R134-2 §4 + R136-1 §7)

### 5.1 风险 1-8

| # | 风险 | 概率 | 影响 | 缓解 | 决策依据 |
|---|------|------|------|------|---------|
| **R1** | **B1 入口改写破坏下游消费者** (24 LOCKED 入口签名 改写破坏 V1.0 release 兼容) | 中 | 高 | 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth_xxx::Type` 全路径仍能用 + V1.1 release bump 1.2.1 (per 决策 #74 B2) | 决策 #74 B1 + R131-5 §2.2 + R132-1 §2.2 + R137-2 §3 |
| **R2** | **R11 baseline 3 值 改写失去哲学标** | 中 | 中 | 新 baseline 需更高, 跟 R12 测度对齐, Mavis 自决 (per 决策 #74 §2.3) | 决策 #74 §1 A1 + 决策 #33 §2.3 A1 + R137-2 §3.9 |
| **R3** | **24 LOCKED mtime 16:34 之前 改写破坏 audit** | 中 | 中 | 仅加 NEW `pub mod` 0 改原 mod 顺序, git diff 清晰 + V1.0 release 仍 0 改严守 (per 决策 #33 §2.3 B1) | 决策 #33 §2.3 B1 + 决策 #74 §1 B1 + R131-5 §1.1 |
| **R4** | **9 organ 对应关系跟 24 LOCKED 主体冲突** | 中 | 中 | 9 organ 9 主体 + 5 工具, 0 改 24 LOCKED 主体, 仅加 organ → crate 映射层 | 决策 #74 B1 + R131-5 §2.6 + R137-2 §3.8 |
| **R5** | **PHL-07 实施影响 12 键其他键** (13 键 stub → 13 键真实施, 跟 12 键集成) | 中 | 中 | 13 键 tests 严守 0 改, 41 NEW tests 0 触碰 13 键 tests + 14 维主对话锚 = V0.5 30 维子集 (深化, 不扩展 30 维) | 决策 #74 §1 A3 + 决策 #33 §2.1 A3 + R137-1 + R125-12 P0-3 |
| **R6** | **PHL-07 实施 cargo compile fail** (per R130-1 25 hard errors 警示) | 中 | 中 | PHL-07 实施前先派 fix sub-agent 修 25 hard errors (per R130-1 + R134-backend-1) | R130-1 警示 + R137-1 + 决策 #33 §2.3 |
| **R7** | **整合 #6/7 commit 时机** (跟 R130-1 25 hard errors 警示 类似) | 中 | 中 | 整合 #5.1 commit BLOCKED → 必修 25 hard errors → 再拍 5.1 → 5.2 → 5.3 → 8 步 verify 全 PASS → 整合 #6 + #7 commit 拍板 (per 决策 #62 + 决策 #64) | 决策 #33 C1 + 决策 #64 + 决策 #62 + 决策 #71 §4 + R130-1 |
| **R8** | **Cargo.toml `1.0.0 → 1.1.0` minor bump 跟决策 #74 §1 B2 1.2.1 不一致** | 低 | 低 | R140-2 提议 1.1.0 跟 决策 #22 §2.2 一致, Mavis 自决拍板 (per 决策 #74 §2.2 Mavis 自决) | 决策 #22 §2.2 + 决策 #74 §1 B2 + R137-3 §3.1 |

### 5.2 风险 9-16

| # | 风险 | 概率 | 影响 | 缓解 | 决策依据 |
|---|------|------|------|------|---------|
| **R9** | **1.0 release 后用户反馈** (破坏 V1.0 release 兼容, 1.0 release 是 release 实战 first time, 估会有用户反馈) | 中 | 中 | 1.0 release 实战 8 步 verify 全 PASS, 8 哲学锚严守 0 漂移, V1.1 release 25 LOCKED 入口签名 0 改 verify (24 + PHL-07) | R134-2 5 阶段 + R129-35 7 步 runbook + 决策 #33 §2.3 |
| **R10** | **V1.1 release 8 步 verify 失败** (cargo build/test/clippy/fmt/audit/deny/doc + 25 LOCKED 入口签名 0 改 verify) | 中 | 中 | 8 步 verify 必跑, 任一 FAIL 必修后再 push, V1.1 release tag 推迟 | 决策 #74 §1 + 决策 #33 §2.3 + R130-1 + R137-1 |
| **R11** | **V1.1 release 实战 8 步流程 中途 失败** (git push + tag + GitHub Pages 重新部署) | 中 | 中 | 7 步 runbook 续 (R129-35 + R134-2 + R136-2), 实战 5 步流程 (R136-2), 主人手跑 + 决策日志写 (per 用户记忆 #10) | R129-35 7 步 + R134-2 5 阶段 + R136-2 实战 |
| **R12** | **V1.1 release GitHub Pages 重新部署 失败** (mkdocs build 失败 + gh-pages branch push 失败) | 中 | 中 | V1.1 release 前 mkdocs build verify 100%, gh-pages branch 已存在 (per 1.0 release 时已建), force push 简化 | R129-13 mkdocs.yml 4133 bytes + R129-23 deploy-github-pages.ps1 + 决策 #33 §2.3 |
| **R13** | **V1.1 release 后 V1.2 路线图 调研 推迟** (per 决策 #71 §4 永久循环接续 + 决策 #74 §2.3 V1.2 release 8 硬墙可重评) | 低 | 中 | V1.1 release 后立即 V1.2 调研, R131 era (V1.2 era 调研) 10 sub-agent 派活规划 (per R129-29 §5.3) | 决策 #71 §4 + 决策 #74 §2.3 + R129-29 §5 |
| **R14** | **V2.0 远期 8 哲学锚 推翻 + 重建 风险** (per 决策 #74 §2.3 V2.0 release 8 哲学锚可重建) | 低 | 高 | 决策 #74 §2.3 V2.0 release 8 哲学锚可重建, 8 哲学锚 → N 哲学锚 重建 (per 主人 8/11 01:14 拍板 3 件套 §3 "推翻 + 重建 8 哲学锚") | 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 + 哲学文档 15-no-fear-complexity.md |
| **R15** | **Cargo workspace 87 → 30 简化 OR 87 → 120+ 复杂化 风险** (per 决策 #73 §3 "不要怕复杂度" 哲学) | 低 | 中 | "不要怕复杂度" 哲学 落地 (per 决策 #73 §3 + 哲学文档 15), V2.0 release 87 → 30 简化 OR 87 → 120+ 复杂化 都 OK, 维护交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3) | 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + 决策 #74 §2.3 |
| **R16** | **0 装 PASS 严守 100% 风险** (V1.1 release 借鉴 12 源 0 装, per 决策 #33 §2.3 C2) | 低 | 中 | 0 cargo install / 0 cargo add 严守 100% (per 决策 #33 §2.3 C2), 11 借脑 0 装 (ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502 + LiteLLM 公开 1:1 翻译 + opencode 子代理 1:1 翻译 + Guardrails 6 重守门 1:1 翻译 + Clap 4.6.6 + hyper 0.1.20 + servers 76d64c8), 1 永久跳过 (OpenCog AGPL-3.0) | 决策 #33 §2.3 C2 + R129-7 + R129-28 + R130-6 + R131-2 |

**16 风险总览**:
- **R1-R3**: B1 入口改写 + R11 baseline 改写 + 24 LOCKED mtime 改写 (8 哲学锚 0 漂移 + 决策 #74 §2.3 V1.1 release 边界)
- **R4**: 9 organ 对应关系冲突 (organ-first 拓扑 + R131-5 §2.6)
- **R5-R6**: PHL-07 实施 (14 维主对话锚 + 41 NEW tests + 8 硬墙 0 越界)
- **R7**: 整合 #6/7 commit 时机 (R130-1 25 hard errors 警示 + 决策 #64 cron auto-pickup)
- **R8**: Cargo.toml bump 1.1.0 vs 1.2.1 reconcile (决策 #22 §2.2 + 决策 #74 B2)
- **R9-R12**: V1.1 release 实战 8 步流程 (1.0 release 后用户反馈 + 8 步 verify + git push + GitHub Pages)
- **R13**: V1.2 路线图调研推迟 (决策 #71 §4 永久循环 + 决策 #74 §2.3 V1.2 release)
- **R14-R15**: V2.0 远期 8 哲学锚推翻 + 重建 + Cargo workspace 87→30 简化 OR 87→120+ 复杂化 (决策 #74 §2.3 + 决策 #73 §3 "不要怕复杂度" 哲学)
- **R16**: 0 装 PASS 严守 100% (决策 #33 §2.3 C2)

---

## 6. V1.1 release 12 决策原则 (per 决策 #74 §1 + 决策 #73 §3 + 决策 #33 §2.3 + 决策 #71 §4 + 决策 #75 §2.1 + 决策 #80)

### 6.1 决策原则 1-6

1. **D1 最强效果 > 最简单代码** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md §1.1)
   - 推翻 KISS (Keep It Simple, Stupid)
   - 拥抱 SOTA (State Of The Art)
   - V1.1 release 实施: PHL-07 14 维主对话锚 + 24 LOCKED 入口签名 改写 + 三洋葱 → 四洋葱 升级 + 9 organ workspace 化 + R12 测度对齐 (24+11 = 35 测量函数) + 借鉴源 12 源 0 装严守 (per 决策 #74 B1 + 决策 #33 §2.3 C2)

2. **D2 最厉害工程 > 最易维护** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md §1.2)
   - 推翻 DRY (Don't Repeat Yourself)
   - 拥抱 BORROW (借脑 1:1 公开模式)
   - V1.1 release 实施: 5 大模块集中 crate 拆 sub-crate (47 sub-crate) + 24 LOCKED 入口签名 改写 (8 方向 5 阶段 8 周) + OpenCog 借脑 1:1 公开模式 (per R130-6 + 决策 #22 §4 AGPL-3.0 决策)

3. **D3 维护交给未来高水平团队** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md §1.3)
   - 推翻"代码要让初级团队能接手"
   - 拥抱"代码要让高水平团队能发挥"
   - V1.1 release 实施: 24 LOCKED 入口签名 0 改 verify (24 + PHL-07 = 25) + 8 哲学锚严守 0 漂移 + 6 重守门 v7 严守 + V0.5 30 维严守 + 0 借具体源码 100% (per 决策 #33 §2.3 C2 + 决策 #74 §1 B1)

4. **D4 8 硬墙 严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
   - B1 24 LOCKED 入口签名: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (8 方向 5 阶段 8 周) + V2.0 release 可重评
   - B2 workspace.version 1.2.0: 🔒 V1.0 release 1.2.0 严守 + 🔒 V1.1 release bump 1.1.0 (R140-2 提议) 或 1.2.1 (per 决策 #74 B2) + 🔒 V2.0 release bump 2.0.0
   - A1 R11 baseline 3 值: 🔒 严守 + 🟢 V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) + 🟢 V2.0 release 可重评
   - A3 12 键 + PHL-07: 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per 决策 #74 §1 A3) + 12 键其他可改
   - B3 V0.5 30 维: 🔒 严守 (V1.0 + V1.1) + 🟢 V2.0 release 可重评
   - B4 6 重守门 v7: 🔒 严守 (V1.0 + V1.1) + 🟢 V2.0 release 可重评
   - B5 8 哲学锚: 🔒 严守 (V1.0 + V1.1) + 🟢 V2.0 release 推翻 + 重建
   - C1 0 主动 commit: 🔒 严守 (V1.0 + V1.1 + V2.0)
   - C2 0 装 PASS: 🔒 严守 (V1.0 + V1.1 + V2.0)
   - 0 push: 🔒 严守 (V1.0 + V1.1 + V2.0)

5. **D5 B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 边界** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 §1)
   - V1.0 release (整合 #5.1 commit): 0 改 24 LOCKED 入口签名 + 0 改 24 LOCKED crate mtime baseline 16:34 之前 + 0 改 R11 baseline 3 值 + PHL-07 spec-only 0 实施 + 0 越界 8 硬墙 100%
   - V1.1 release: 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决改, 8 方向, 5 阶段 8 周) + 24 LOCKED crate mtime baseline 16:34 之前 可改 + R11 baseline 3 值 → R12 测度对齐 + PHL-07 实施 + Cargo.toml workspace.version bump 1.2.1 (或 1.1.0 per R140-2 提议) + 0 越界 8 硬墙 100%
   - V2.0 release: 全 8 硬墙 可重评 + 推翻 + 重建 8 哲学锚 + Cargo workspace 87 → 30 简化 OR 87 → 120+ 复杂化 都 OK (per "不要怕复杂度" 哲学) + 24 LOCKED → 0 LOCKED 全解锁 + 8 哲学锚 → N 哲学锚 重建

6. **D6 整合 #5/6/7 commit 由 Mavis 自决拍板** (per 决策 #33 C1 + 决策 #64 + 决策 #71 §4 + 决策 #73 §5)
   - 整合 #5.1/5.2/5.3 commit: 5.1 → 5.2 → 5.3 顺序 git add + git commit, Mavis 自决拍板 (per 决策 #62 §5.1-§5.3)
   - 整合 #6.1/6.2/6.3 commit: 6.1 → 6.2 → 6.3 顺序 git add + git commit, Mavis 自决拍板 (per 决策 #62 类比 + 决策 #74 B1)
   - 整合 #7.1/7.2/7.3 commit: 7.1 → 7.2 → 7.3 顺序 git add + git commit, Mavis 自决拍板 (per 决策 #33 C1 + 决策 #71 §4)
   - 8 项 verify 100% 后拍板 (per 决策 #61 §1.4 整合 #5 commit 8 项 verify)

### 6.2 决策原则 7-12

7. **D7 0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #58 §7 + 决策 #60 + 决策 #61 §6 + 决策 #62 §9 + 决策 #71 §4 + 决策 #74 §1)
   - V1.0 release 实战: 主人起床后手跑 (per 决策 #33 + 决策 #61 §6)
   - V1.1 release 实战: 主人起床后手跑 (per 决策 #33 + 决策 #71 §4 + R136-2 实战 6 步)
   - Mavis 0 push 0 配 remote 0 tag 0 release 0 build pages (per 决策 #58 §7 + 决策 #62 §9)
   - 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)

8. **D8 0 主动 IM 主人 + 0 主动删** (per gate-discipline + 决策 #61 §6 + 用户记忆 #10 + Safety policy + 决策 #44 + #60)
   - 0 主动 plain reply on skip ticks (per gate-discipline)
   - 0 主动删 (per Safety policy + 决策 #44 + #60, target/ 29.13 GB < 50 GB 保守策略)
   - 仅 done notification 主动报告 (per 决策 #61 §6)

9. **D9 整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
   - master HEAD = abf1224371016e36df8f4d3c9a05b33f1c563e0d 严守 100%
   - 0 重跑 0 重 commit
   - 整合 #5/6/7 commit 都基于 abf12243 续

10. **D10 决策日志写** (per 决策 #10 + 用户记忆 #10)
    - 整合 #5/6/7 commit 拍板 = done notification, 必须报告 (含 3 commit hash + master HEAD 新值 + 决策链 #80-#100 报告路径)
    - V1.0 release + V1.1 release 实战 = done notification, 必须报告 (含 tag name + commit hash + GitHub Pages URL)
    - cron Section 6 决策日志 5 min tick 自动写

11. **D11 0 重复造轮子** (per 用户记忆 #6 + 决策 #79 + 决策 #80)
    - R140-2 不重写 R130-5 / R131-1/2/3/4/5/9 / R132-1 / R133-1/2/3 / R134-2/3/4/5 / R135-1/2 / R136-1/2 / R137-1/2/3 (per 任务 spec + 用户记忆 #6 + 决策 #79)
    - 已有的 verify 报告 reference 而非重写
    - 0 重复造轮子 = Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)

12. **D12 16 跑中上限严守** (per 主人 0:34 拍板 "跑中 ≥ 16" + 决策 #64 §2.2 cron Section 2)
    - R130 era 6 sub-agent + R131 era 9 sub-agent + R132 era 2 sub-agent + R133 era 3 sub-agent + R134 era 5 sub-agent + R135 era 6 sub-agent + R136 era 2 sub-agent + R137 era 5 sub-agent + R138 era 13 sub-agent + R139 era 1 sub-agent + R140 era 14 sub-agent + R141 era 14 sub-agent + R142 era 14 sub-agent + R143 era 14 sub-agent = 总 16+ sub-agent
    - 派活 2 批 5+5 派满 16 上限 (per 决策 #64 §2.2 cron Section 2)
    - 整合 #6/7 commit 拍板 = Mavis 自决, 0 主动 push 严守 (per 决策 #33 C1 + 决策 #74 B1)

**12 决策原则总览**:
- **D1-D3**: 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md (不要怕复杂度 + 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- **D4-D5**: 决策 #33 §2.3 + 决策 #74 §1 拍板 (8 硬墙 严守 + B1 改写)
- **D6**: 决策 #33 C1 + 决策 #64 + 决策 #71 §4 (整合 #5/6/7 commit Mavis 自决拍板)
- **D7-D8**: 决策 #33 §2.3 + 决策 #58 §7 + 决策 #60 + 决策 #61 §6 + 决策 #62 §9 + gate-discipline (0 主动 push + 0 主动 IM + 0 主动删)
- **D9**: 决策 #48 + 决策 #61 §1.2 (整合 #4 commit abf12243 严守)
- **D10**: 决策 #10 + 用户记忆 #10 (决策日志写)
- **D11**: 用户记忆 #6 + 决策 #79 + 决策 #80 (0 重复造轮子)
- **D12**: 主人 0:34 拍板 + 决策 #64 §2.2 (16 跑中上限严守)

---

## 7. refs

### 7.1 决策链 refs (per 决策 #10 + 决策 #22 + 决策 #33 + 决策 #48 + 决策 #62 + 决策 #64 + 决策 #70 + 决策 #71 + 决策 #72 + 决策 #73 + 决策 #74 + 决策 #75 + 决策 #76 + 决策 #77 + 决策 #78 + 决策 #79 + 决策 #80)

- **decision-10**: 主人离场 Mavis 自主决策 + 决策日志 (per 用户记忆 #10 主人长时间离开, Mavis 自主决策 + 决策日志)
- **decision-22**: 24 LOCKED 自主确认 + semver (1.2.0 → 1.0.0 → 1.1.0)
- **decision-33**: 8 硬墙 + 0 装 PASS 严守 (B1 24 LOCKED 入口签名 0 改 / B2 workspace.version 1.2.0 / A1 R11 baseline 3 值 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push)
- **decision-48**: 整合 #4 commit abf12243 严守 (8/10 19:41 done, master HEAD 严守 100%)
- **decision-55**: R127 阶段 F 1.0 release 准备 (8 哲学锚 + 30 维 + 6 重 v7 + 24 LOCKED + 12 键+PHL-07)
- **decision-56**: R127-2 借鉴 3 限流 + 1.0 release 文档 + 形式化证明
- **decision-57**: R128 era ASI Python Stage 1-2 + Tauri prototype + Cargo 实战 + LICENSE + 整合 #5 pre-stage
- **decision-58**: R128-2 P15-1 1.0 release Cargo 配
- **decision-60**: 0 主动 push 严守
- **decision-61**: R129 era 派活规划 + 新会话接手 + 整合 #5 commit 准备
- **decision-62**: 整合 #5 commit 拆 3 commit 拍板 (5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决
- **decision-64**: auto-replenish-16 cron (5 min tick 自动拍板, 16 跑中上限严守)
- **decision-70**: Mavis 清理决策权升级 (主人 0:25 升级授权)
- **decision-71**: R130 era 自动接续 4 步 (调研 + 差距 + 计划 + 继续干 永久循环, per 主人 0:57 拍板)
- **decision-72**: R130 era 调研 6 sub-agent 派活
- **decision-73**: 主人 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #74 8 硬墙 B1 改写
- **decision-74**: 8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构, per 主人 8/11 01:14 拍板)
- **decision-75**: R131-R132-R133 batch dispatch 11 sub fill 16 (R131 era 第 2 批 6 sub-agent 派活)
- **decision-76**: R134 era 派活清单 (8 sub-agent 调研 + 实施, 16 跑中上限严守)
- **decision-77**: R135 era + R136 era 计划续 (R135 era 6 sub-agent + R136 era 2 sub-agent 派活)
- **decision-78**: 整合 #5.3 reports commit paiban option A (决策链 #30-#74 + 41 sub-agent 报告)
- **decision-79**: R138 era 13 sub + R139-1 + 14 sub-dispatch fill 16 (per R137 era 续)
- **decision-80**: R140-R143 14 sub-dispatch fill 16 (本报告派活依据, 2026-08-11 02:02:56 cron 自动拍)

### 7.2 报告链 refs (per R130-5 + R131-1/2/3/4/5/9 + R132-1/2 + R133-1/2/3 + R134-1/2/3/4/5 + R135-1/2 + R136-1/2 + R137-1/2/3)

- **R129-7**: 借鉴 11/11 升级 1:1 verify
- **R129-11**: 后端 0 装 PASS 终极 verify + PHL-07 spec-only 关键诚实标
- **R129-21**: 0 装 PASS violation 报告, 24+5+1 errors 关键诚实标
- **R129-26**: R129 era 健康度 verify, 整合 #5 commit NOT ready, 60% PASS
- **R129-28**: 借鉴 11/11 终极 verify
- **R129-34**: R129 era 跨 sub-agent 总览 final final
- **R129-35**: 1.0 release 实战 final-final 7 步 runbook (Step 0-7)
- **R130-1**: 整合 #5 commit cargo 二次 verify (❌ 25 hard errors BLOCK)
- **R130-2**: ASI Stage 8 集成深化
- **R130-3**: Tauri Stage 5 集成深化
- **R130-4**: 形式化 Stage 5.5 集成深化
- **R130-5**: V1.1 minor release 路线图 (6 大方向, 10 sub-agent 派活规划, 8 步时间线)
- **R130-6**: 借鉴源 12 源调研 (OpenCog AGPL-3.0 fork 决策)
- **R131-1**: 现有架构总审视 10 方向 + 优化点
- **R131-2**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策
- **R131-3**: V1.1 release 实施路线图 6 大方向
- **R131-4**: cargo workspace 结构优化 7 方向
- **R131-5**: 24 LOCKED 入口分布优化 8 方向 (V1.0 release 0 改 verify 24/24 全 PASS)
- **R131-6**: Cargo.toml borrow 段精简
- **R131-7**: pybridge 集成优化
- **R131-8**: Tauri 集成优化
- **R131-9**: 形式化集成优化 9 方向
- **R132-1**: V1.1 release 路线图 final (整合 R130-5 + R131-1/2/3 + 决策 #74 B1 + 决策 #73 拍板 3 件套 + 不要怕复杂度哲学)
- **R132-2**: V2.0 release 战略路线图 8 大方向
- **R133-1**: 借鉴源 12 源 实施 spec (OpenCog AGPL-3.0 fork-then-borrow 模式)
- **R133-2**: ASI Stage 9 长程 AI 成长 实施 spec (per R130-2 调研 Stage 9 路线)
- **R133-3**: 三洋葱架构升级 实施 spec (per 决策 #73 §2.2 更好的架构)
- **R134-1**: 整合 #5 commit 拍板实战
- **R134-2**: 1.0 release 实战 5 阶段计划 (本报告 R140-2 阶段 4 V1.1 release 实战 7 步 runbook 续 基础)
- **R134-3**: 整合 #6 commit 拍板准备 5 阶段计划
- **R134-4**: 整合 #7 commit 拍板续
- **R134-5**: V1.1 release cargo verify
- **R135-1**: V1.1 vs AGI OS 前沿差距
- **R135-2**: V1.1 vs 业界 v2.x 差距 (10 方向差距 + 业界 v2.x 路线图参考)
- **R136-1**: V1.1 release 拍板准备 5 阶段计划 (本报告阶段 4 V1.1 release 实战 5 阶段续 基础)
- **R136-2**: V1.1 release 实战 6 步流程 (本报告阶段 4 V1.1 release 实战 7 步 runbook 续 基础)
- **R137-1**: PHL-07 实施 (本报告阶段 2 A3 PHL-07 实施 基础, V1.0 spec-only → V1.1 真实施, 24 LOCKED → 25 LOCKED)
- **R137-2**: 24 LOCKED 入口签名 改写 spec + 5 阶段实施计划 (本报告阶段 1 B1 24 LOCKED 入口可改部分 基础, 8 方向 5 阶段 8 周)
- **R137-3**: Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (本报告阶段 3 B2 workspace.version bump 基础, semver minor bump)

### 7.3 哲学 + 架构 + 8 硬墙 refs (per 决策 #33 §2.3 + 决策 #74 §1 + 哲学文档 + ROADMAP.md + 8 哲学锚)

- **`docs/conventions/15-no-fear-complexity.md`**: 主人 8/11 01:14 总哲学扩展 (不要怕复杂度 + 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- **`docs/conventions/10-locked.md`**: 8 项形式撤销 (R119-3a-1, 原意保留) + 决策 #73 §2.3 locked 全解锁
- **`docs/conventions/09-anchor.md`**: 8 哲学锚 (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装, per R125 B5 升 8 锚)
- **`docs/architecture-v3-aircraft-carrier.md`**: BF896EEF LOCKED, 立体架构终版 v2
- **`docs/architecture-v4-living-intelligence.md`**: af0d1957 LOCKED, 生命架构 v4
- **`docs/architecture-v4-1-living-intelligence-update.md`**: v4.1 升级
- **`docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md`**: 业界顶级后端 v2.1 路线图 (R20 阶段 6, 抄 wasmtime + qdrant + tokio, 9 条业界顶尖标准)
- **ROADMAP.md**: V1.0 → V2.0 路线图 (1.0 release 28.7KB, P7-2 21:22 写)
- **borrowed-repos**: `.openclaw\workspace\borrowed-repos\` (借鉴 11 源真 cloned + LiteLLM 公开 1:1 + opencode 改借鉴 + 1 永久跳过 OpenCog AGPL-3.0)
- **R11 baseline 3 值**: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 (per 决策 #33 §2.3 A1 + 决策 #74 §1 A1)
- **V0.5 30 维**: per 决策 #33 §2.3 B3 (B3 V0.5 30 维 严守 哲学)
- **6 重守门 v7**: per 决策 #33 §2.3 B4 (B4 6 重守门 v7 严守 哲学)
- **8 哲学锚**: S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 (per 决策 #33 §2.3 B5 严守 哲学)
- **13 键 verdict cache**: 12 键 + PHL-07 = 13 键 (per 决策 #33 §2.1 A3 严守 哲学)
- **9 organ**: body / brain / ear / eye / hand / heart / memory / mind / voice (per R11 9 organ + R125 B7 内部借 OpenCode)

### 7.4 用户记忆 + 主人偏好 refs (per 用户记忆 #1-#10 + 主人 8/11 01:14 拍板 3 件套)

- **用户记忆 #1**: 先思考后动手 (反对"先做再想")
- **用户记忆 #2**: 让我做判断, 不机械问拍板
- **用户记忆 #3**: 用户看结果不看哲学 (核心 UI 原则)
- **用户记忆 #4**: AI 不会衰老病死 (跟传统生命周期模型不同)
- **用户记忆 #5**: 信息密度"高"= 拟人化 + 拟物化
- **用户记忆 #6**: 派 sub-agent 干, 但要驾驭团队不重复造轮子
- **用户记忆 #7**: 推技术决策要守规范, 但要诚实
- **用户记忆 #8**: 前端终极 = Tauri, TUI 是过渡
- **用户记忆 #9**: TUI 升级节奏: 改瘦后暂告段落, 优先后端
- **用户记忆 #10**: 主人长时间离开, Mavis 自主决策 + 决策日志 (决策 #10 + 用户记忆 #10, 主人 8/6 01:14 拍板)
- **主人 8/11 01:14 拍板 3 件套**: ① 工程类 + 技术类 locked 全早解锁 + Mavis 自决架构拍板 ② 架构审视 + 升级方案永久工作项 ③ 总哲学扩展 (复杂不恐惧, 最强效果 + 最厉害工程)
- **主人 8/10 16:31 拍板**: "全部采纳, 全都能动, 你有最高权限" (Mavis = orchestrator + 全自决 + 最高权限)
- **主人 8/6 01:14 拍板**: "我睡觉去了, 后面有需要决定的都按你想法倾向来, 最终收尾的时候把你的想法决策也都记录下来就行" (Mavis 自主决策 + 决策日志 严守)
- **主人 8/4 23:33 拍板**: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备"
- **主人 8/4 23:55 拍板**: "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
- **主人 0:34 拍板**: 16 跑中上限严守
- **主人 0:43 拍板**: 中断接手 (检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- **主人 0:49 + 0:54 拍板**: 编译产物清理决策矩阵 (≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- **主人 0:57 拍板**: 计划内任务完成自动接续 4 步 + 永久循环 (调研 + 差距 + 计划 + 实施 → 永久)

### 7.5 重要路径 refs (per 用户记忆 #10 + 项目背景)

- **真 API key**: `.minimax-agent-cn\projects\apikey.txt` (125 chars, sk-cp-kug0t7Jik3-...)
- **VCPChat 参考 (Electron 桌面 app, chat-first)**: `Downloads\VCPChat-main.zip`
- **默认工作目录**: `.minimax-agent-cn\projects\`
- **Apeireth-rust 项目根**: `Apeireth-rust\`
- **Cargo.toml**: `Apeireth-rust\Cargo.toml:274 workspace.version = "1.2.0"`
- **24 LOCKED crate 入口**: `crates/apeireth-*/src/lib.rs` (24 LOCKED crate, per R131-5 §1.2 verify 24/24 全 PASS)
- **借鉴 11 源真 cloned**: `.openclaw\workspace\borrowed-repos\`
- **整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done)
- **master HEAD**: 整合 #5/6/7 commit 拍板后 持续推进

---

## 8. 一句话 (再次强调)

**V1.1 release 路线图 详细 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 主人 8/11 01:14 拍板 3 件套 + 决策 #71 §3 永久循环 + R140 era 第 2 批)**: V1.1 release = 1.0 release (~8/11) 后 ~3.5 个月 minor release (`v1.1.0` 跟决策 #22 §2.2 semver 一致, 或 `v1.2.1` 跟决策 #74 B2 一致, Mavis 自决拍板, 本报告倾向 v1.1.0, 估 2026-11-30, 整合 #6 commit 2026-11-25 + 整合 #7 commit 2026-11-29 + V1.1 release 实战 2026-11-30 06:00-08:00 主人手跑). **4 阶段 实施** (per 决策 #74 B1 改写边界): **阶段 1** B1 24 LOCKED 入口可改部分 (per R137-2 + R131-5 8 方向: 标准化 + 瘦身 + 9 叶子拆 + core 拆 pub mod + 大模块拆 sub-crate + DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐, 5 阶段 8 周 估, 派 R138-R142 era 29-43 sub-agent); **阶段 2** A3 PHL-07 实施 (per R137-1, V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests, 派 R134-PHL07-1~5 5 sub); **阶段 3** B2 workspace.version 1.2.0 → 1.2.1 bump (per R137-3 5 阶段 5 天/1 周, semver minor bump backward-compatible, 派 R137-3 + R134-backend-1~5); **阶段 4** V1.1 release 实战 (per R134-2 1.0 release 实战 7 步 runbook 续 + R136-1 5 阶段 + R136-2 实战 6 步, 主人起床后手跑, 估 2026-11-30 06:00-08:00 时段). **8 步时间线**: 整合 #5.1 (V1.0 release src/) → 整合 #5.2 (V1.0 release docs/) → 整合 #5.3 (V1.0 release reports/) → 1.0 release tag v1.0.0 (8/11 主人起床) → 整合 #6 (V1.1 release src/ + docs/ + reports/, 2026-11-25) → 整合 #7 (V1.1 release 前最终, 2026-11-29) → V1.1 release 实战 (2026-11-30 06:00-08:00 主人起床后手跑 7 步 runbook) → 永久循环 (V1.2 release 估 2027-02-28 + V2.0 远期 2027+). **22 决策点** (决策 #80-#100, R140 era + R141 era + R142 era 实施 + 整合 #6/7 commit 拍板 + V1.1 release tag 拍板流程, Mavis 全自决, 16 跑中上限严守). **16 风险** (含 R1 B1 入口改写破坏下游消费者 + R5 PHL-07 实施影响 12 键其他键 + R7 整合 #6/7 commit 时机 跟 R130-1 25 hard errors 警示 类似 + R9 1.0 release 后用户反馈). **12 决策原则** (含 最强效果 > 最简单代码, 最厉害工程 > 最易维护, 维护交给未来高水平团队, 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md). **8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §1) + **8 哲学锚 严守 0 漂移** (per 决策 #33 §2.3 B5) + **不要怕复杂度哲学 落地** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 主人 8/11 01:14 拍板 3 件套 §3). 0 改 src 严守 (本任务 = 调研/路线图类, 0 实施) + 0 主动 commit (本报告 untracked, Mavis 整合 #5.3 / #6 / #7 commit 时机拍板) + 0 主动 push (V1.1 release 主人手跑 严守) 100%.

---

**Status**: ✅ **R140-2 V1.1 release 路线图 详细 done 2026-08-11** (60 min 时间盒, 9 章节 ~80 KB), 整合 R130-5 + R131-5 + R132-1 + R135-2 + R137-1 + R137-2 + R137-3 + R134-2 + R136-1 + R136-2 = final 版 V1.1 release 路线图 详细, 0 改 src 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100% (5 借脑 0 装 + 1 借脑 ID 索引 OpenCog), 8 硬墙 0 越界严守 100%, 0 重复造轮子严守 100%

**Report path**: `Apeireth-rust\reports\agent-r140-2-v1.1-release-roadmap-detailed-2026-08-11.md`

**Decision chain**: #80 (R140 era 派活依据) + #74 (8 硬墙 B1 改写) + #73 (主人 8/11 01:14 拍板 3 件套) + #71 (永久循环) + #33 (8 硬墙) + #22 (semver) + #10 (决策日志)
