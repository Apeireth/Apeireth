# R132-1: V1.1 release 路线图 final (per 决策 #74 B1 + 决策 #75 §2.1 + 决策 #73 拍板 3 件套 + 不要怕复杂度哲学 + 主人 8/11 01:14 拍板)

**Date**: 2026-08-11 (R132-1 sub-agent, Mavis 派, per 决策 #75 §2.1 R132 era 计划 2 sub-agent + 决策 #71 §4 R132 era 计划 + 主人 8/11 01:14 拍板 3 件套)
**Author**: R132-1 sub-agent (Mavis 派, R132 era 计划阶段, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 装 PASS 严守)
**任务**: V1.1 release 路线图 final 版 (整合 R130-5 V1.1 路线图 + R131-1 架构审视 + R131-2 借鉴 12 源 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 = final 版)
**关联**:
- decision-9 (TUI 升级节奏) + decision-10 (主人离场 Mavis 自主决策) + decision-22 (24 LOCKED 自主确认 + semver) + decision-33 (8 硬墙 + 0 装 PASS) + decision-48 (整合 #4 commit abf12243) + decision-62 (整合 #5 commit 拆 3 commit 拍板) + decision-71 (R130 era 自动接续 4 步) + decision-72 (R130 era 调研 6 sub-agent 派活) + **decision-73 (主拍板 3 件套)** + **decision-74 (8 硬墙 B1 改写)** + **decision-75 (R131-R132-R133 batch dispatch 11 sub fill 16)**
- R129-11 (后端 0 装 PASS 终极 verify + PHL-07 spec-only 关键诚实标) + R129-17 (R130 era 路线图详细, V1.1 基础 §4) + R129-29 (R130 era 路线图 final, V1.1 §4 详细 6 维度) + R129-35 (1.0 release 实战 final-final 7 步 runbook)
- R130-1 (整合 #5 commit cargo 二次 verify, ❌ 25 hard errors BLOCK) + R130-2 (ASI Stage 8 集成深化) + R130-3 (Tauri Stage 5 集成深化) + R130-4 (形式化 Stage 5.5 集成深化) + R130-5 (V1.1 路线图) + R130-6 (借鉴源 12 源调研)
- R131-1 (现有架构总审视 10 方向, 87 crate + 24 LOCKED + Cargo.toml borrow + Cargo.lock + pybridge + ASI + 形式化 + Tauri + 借鉴 12 源 + 三洋葱 + 9 organ) + R131-2 (借鉴 12 源差距分析 + OpenCog AGPL-3.0 fork 决策)
- R131-3 (V1.1 release 实施路线图, **任务里 R132-1 派活时 R131-3 报告未出, 任务描述给 R132-1 已给详细规划, R132-1 整合 R130-5 + R131-1 + R131-2 = final 版**)
- 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri" + 8/4 23:55 "TUI 升级路线图沉淀成文档暂时就这样告一段落" + 8/6 01:14 "后面有需要决定的都按你想法倾向来" + 8/10 16:31 "全部采纳, 全都能动, 你有最高权限" + 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视永久工作项 + 不要怕复杂度哲学)
- 用户记忆 #3 (用户看结果不看哲学) + #4 (AI 不会衰老病死) + #5 (信息密度高 = 拟人化 + 拟物化) + #6 (派 sub-agent 干, 但要驾驭团队不重复造轮子) + #8 (TUI → Tauri 终极路线) + #9 (TUI 升级节奏) + #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R130-1 01:14 实地 verify = **NOT READY** (cargo workspace 3 crate 25 hard errors + 8 步 verify 全部 FAIL, 需先派 fix sub-agent 修 25 hard errors, 估 30-60 min fix → 8 步 verify 全 PASS → 再拍 5.1 → 5.2 → 5.3)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间, per R130-5 §1.1 V1.1 定位
**状态**: ✅ done (R132-1 V1.1 release 路线图 final 版, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 不重写 R130-5 + R131-1/2, 整合 final 版, 拓维)

---

## 0. 一句话 (TL;DR)

**V1.1 release 路线图 final 版 (per 决策 #74 B1 + 决策 #75 §2.1 + 决策 #73 拍板 3 件套 + 主人 8/11 01:14 拍板 + 不要怕复杂度哲学)**: 1.0 release (~8/11) 后 ~3.5 个月 minor release (`v1.1.0`, 估 2026-11-30), **6 大方向 final 版**: ①**PHL-07 实施** (V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests, R129-11 关键诚实标落地) ②**24 LOCKED 入口签名改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级, 0 改原 24 LOCKED 入口签名顺序 + 0 改原 24 LOCKED crate mtime 16:34 之前) ③**后端加固** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml `1.2.0 → 1.2.1` bump + pybridge 886/886 性能测试 + Cargo.lock 分模块) ④**Tauri Stage 5+** (9 organ 拟人化深化 + 5 nav 完整 + Tauri 2.0 完整集成 + 跨平台部署 Windows/macOS/Linux + Tauri 性能优化) ⑤**ASI Stage 8+** (Stage 9 终极自治 + 长程 AI 成长平台 + OpenCog AGPL-3.0 fork 决策 + pybridge 集成优化 + ASI Stage 9 集成测试) ⑥**形式化 Stage 5.5+** (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化). **V1.1 release 时间窗口**: 短期 1.0 release 实战完 + 1 周 V1.1 release 路线图拍板 → V1.1 release 实施 6 方向 6 周 (per 方向 1 周 R134-N sub-agent) → 估 2026-11-30 V1.1 release tag 打上. **16 跑中上限持续**: 30+ sub-agent 实施 (5-10 per 方向 × 6 方向 = 30+ sub-agent, per 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 R134+ era 派活). **永久循环**: V1.1 release → V1.2 minor → V2.0 major (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构). **决策原则**: 8 硬墙 0 越界 (B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 `1.2.0` V1.0 release 严守 + V1.1 release bump `1.2.1` / A1 R11 baseline 3 值 `0.8682/0.8532/0.9063` 严守 / A3 12 键 + PHL-07 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 / B3 V0.5 30 维严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 主动 push 严守) + 0 装 PASS 严守 (V1.1 借鉴源 12 源: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源) + 0 主动 IM 主人 (per gate-discipline) + 0 主动 commit/push (per 决策 #33 §2.3) + 0 主动改 src (per 决策 #33 §2.3 + 决策 #74 B1 V1.0 release 0 改严守).

---

## 1. V1.1 release 战略定位 (1.0 release 后下一步 + 3.5 个月 minor release era)

### 1.1 V1.1 定位 (per 决策 #9 + 决策 #22 §2.2 + 决策 #71 §2.2 + R129-17 §4 + R129-29 §4 + R130-5 §1.1 + 主人 8/4 23:33 + 主人 8/4 23:55 + 用户记忆 #8-#10 + 决策 #74 B1 改写)

**V1.1 = 1.0 release (~8/11) 后 ~3.5 个月 minor release era, 6 大方向 final 版 (per R130-5 调研 + R131-1 架构审视 + R131-2 借鉴 12 源差距 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 拍板 3 件套)**:
- **起点**: 1.0 release tag `v1.0.0` 打上 (per R129-35 final-final 7 步 runbook, 主人起床后 06:00-08:00 手跑, 估 8/11 done, 当前 01:30 状态, R129-3 报告 01:25 tick 仍未出 → Section 3 中断接手)
- **终点**: V1.1 release tag `v1.1.0` 打上 (估 2026-11-30, per R130-5 §1.1 V1.1 定位 + R130-5 [R129-35 final-final 续] 7 步 runbook, 主人起床后手跑)
- **核心任务**: 6 大方向 final 版 + 整合 #6 commit 拍板 (Mavis 自决, 拆 3 commit 拍板, per 决策 #33 C1 + 决策 #71 §2.5) + 整合 #7 commit 拍板 (V1.1 release 前) + V1.1 release 实战 (主人起床后手跑, 估 2026-11-30)
- **semver 严守 (per 决策 #22 §2.2 + 决策 #74 §1 B2 改写)**:
  - 整合 #4 commit abf12243 master HEAD: `workspace.version = "1.2.0"` (B2 严守 100%)
  - 1.0 release 时: `1.2.0 → 1.0.0` 大版本归 0 (per 决策 #22 §2.2, R129-7 done, R129-21 verify)
  - V1.1 release 时: `1.0.0 → 1.1.0` minor bump (per 决策 #22 §2.2, semver 严守, V1.1 加 NEW feature 兼容 1.0)
  - V1.2 release 时: `1.1.0 → 1.2.0` minor bump (per 决策 #22 §2.2, 后续 V1.2 加 NEW feature 兼容 1.1)
  - **决策 #74 §1 B2 改写**: V1.0 release 1.2.0 严守 + **V1.1 release bump 1.2.1** (版本管理严守 semver, per "不要怕复杂度"哲学)

**R130 era 接力 → V1.1 era 战略** (per 决策 #71 §2.2 + R129-29 §1.3 + R129-17 §1.3 + R130-5 §1.1 + 决策 #75 §2.1):
- **R130 era (8/11, 整合 #5 commit 拍板 → 主人起床)** = 整合 #5 commit 拍板 + 1.0 release 实战 (主人起床后手跑) + R130-1~6 6 sub-agent 调研 (R130-1 整合 #5 verify / R130-2 ASI Stage 8 / R130-3 Tauri Stage 5 / R130-4 形式化 Stage 5.5 / R130-5 V1.1 路线图 / R130-6 借鉴 12 源)
- **R131 era (V1.1 era 调研, 8/11 01:18+ → done)** = 3 sub-agent 差距分析 (R131-1 架构审视 / R131-2 借鉴 12 源差距 / R131-3 V1.1 release 实施路线图) + 6 sub-agent 架构细分 (R131-4~9 per 决策 #75 §2.1, 1:20 派活)
- **R132 era (V1.1 era 计划, 8/11 01:20+ → done)** = 2 sub-agent 计划拍板 (R132-1 [本任务] V1.1 release 路线图 final + R132-2 V2.0 release 战略路线图, per 决策 #75 §2.1)
- **R133 era (V1.1 era 实施 spec, 8/11 01:20+ → done)** = 3 sub-agent 实施 spec (R133-1 借鉴 12 源 实施 / R133-2 ASI Stage 9 实施 / R133-3 三洋葱架构升级 实施, per 决策 #75 §2.1, V1.1 release 实施)
- **R134 era (V1.1 era 实施, 估 8/12+) = 30+ sub-agent 实施 (5-10 per 方向 × 6 方向 = 30+, per 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1, 16 跑中上限严守, 2 批 15+15 派满 16 上限)
- **R135+ era (V1.1 release 实战 + V1.2 调研)** = V1.1 release 实战 (主人起床后手跑, 估 2026-11-30) + V1.2 调研 (估 2026-12, per R130-5 §1.3)

### 1.2 V1.1 时间线 (per 决策 #71 §2.2 + R129-29 §4 + R130-5 §1.2 + 主人 8/4 23:33 + 主人 8/4 23:55 + 用户记忆 #8-#9 + 决策 #74 §1)

```
[8/11 01:00+ 整合 #5 commit 拍板]   Mavis 自决 (5.1 → 5.2 → 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup)
                                     ⚠️ 整合 #5.1 commit BLOCKED per R130-1 01:14 实地 verify (25 hard errors), 需先派 fix sub-agent
[8/11 06:00-08:00 主人起床 1.0 release 实战]   主人手跑 R129-35 final-final 7 步 runbook (8 步 verify + 配 GitHub remote + git push + 打 v1.0.0 tag + GitHub Pages)
[8/11 08:00+ 1.0 release done]    master HEAD = abf12243 + 3 commit (5.1/5.2/5.3), v1.0.0 tag, GitHub release, GitHub Pages 部署
[8/11 08:00+ R130 era 调研 6 sub-agent]  R130-1~6 6 sub-agent 派活 (per 决策 #72, 整合 #5 commit cargo 二次 verify [BLOCKED] + ASI Stage 8 深化 + Tauri Stage 5 深化 + 形式化 Stage 5.5 深化 + V1.1 路线图 [done] + 借鉴源 12 源调研)
[8/11 01:18+ R131 era 差距 3 sub-agent 派活]  R131-1 + R131-2 + R131-3 (per 决策 #73 §3.2)
[8/11 01:20+ R131 era 第 2 批 6 sub-agent 派活]  R131-4~9 (架构细分, per 决策 #75 §2.1)
[8/11 01:20+ R132 era 计划 2 sub-agent 派活]  R132-1 [本任务] + R132-2 (V2.0 release 战略, per 决策 #75 §2.1)
[8/11 01:20+ R133 era 实施 3 sub-agent 派活]  R133-1 + R133-2 + R133-3 (V1.1 release 实施 spec, per 决策 #75 §2.1)
[8/12 R130 + R131 + R132 + R133 era done]   全部 17 sub-agent 调研 + 计划 + 实施 spec done, 决策链 #76-#80 写
[8/12 R134 era 派活]              V1.1 release 实施 6 大方向 6 周 (per 方向 1 周 R134-N sub-agent, 30+ sub-agent, 16 跑中上限严守)
[8/12 - 11/30 R134 era 实施 13 周]  V1.1 release 实施 (实际 ~ 13 周 per 6 大方向 × 1-2 周, 跟 R130-5 §1.1 V1.1 估 2026-11-30 一致)
[9-10 月 R131 era 实施 + R132 era 实施 spec 续]  实施 R131 era 路线图 (TUI 升级 + Tauri Stage 4 + ASI Stage 7 + 形式化 Stage 5.4 + 后端 Stage 4-6 续, per R129-29 §4.2 详细 spec) + R132 era 实施 spec
[11 月 R131 + R132 + R133 era 总览 + 决策链]     整合 #6 commit 拍板 (Mavis 自决, 5.1/5.2/5.3 顺序)
[11/30 06:00-08:00 主人起床 V1.1 release 实战]  主人手跑 V1.1 release 7 步 runbook (8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署)
[12 月 V1.1 release 后]           V1.2 路线图 (per R129-29 §5, 估 2027-02-28, 6 维度: TUI 阶段 3 + Tauri Stage 5 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战)
[2027-02-28 V1.2 release]         v1.2.0 tag 打上
[2027+ V2.0 远期]                 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 (per ROADMAP.md §4 + R119-2 思想层保留)
```

**时间窗口总结 (per 决策 #22 §2.2 + 决策 #71 §2.2 + 决策 #74 §1 + R129-29 §4.1 + R129-29 §5.1 + R130-5 §1.2)**:
- **1.0 release (估 8/11)**: V1.0 release tag `v1.0.0` 打上, R11 baseline 严守
- **V1.1 release (估 2026-11-30)**: 1.0 release 后 ~3.5 个月, V1.1 release tag `v1.1.0` 打上 (per R130-5 §1.1)
- **V1.2 release (估 2027-02-28)**: V1.1 后 ~3 个月, V1.2 release tag `v1.2.0` 打上 (per R130-5 §1.2)
- **V2.0 (2027+, 远期)**: R128+ 升级 + 主人 1.0 release 流程 + 终极路线图 (per ROADMAP.md §4 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

### 1.3 V1.1 跟 R130 era + 1.0 release 实战 + V1.2 接力 (per R130-5 §1.3 + R131-1 §2 + 决策 #75 §2.1)

| Era | 时间 | 状态 | 核心任务 | 决策链 |
|-----|------|------|---------|--------|
| **R125 era** | 8/10 14:00-17:22 | ✅ done (16 sub-agent) | 借鉴 8/11 ✅ cloned + 41 任务起步 | #30-#41 |
| **R126 era** | 8/10 17:22-21:00 | ✅ done (16 sub-agent) | 后端升级 + 8 哲学锚 + 30 维 + 6 重 v7 + Library v1.0 礼物 | #33 + #51-#54 |
| **R127 era** | 8/10 21:00-22:00 | ✅ done (4 sub-agent) | Library Stage 4-6 + 整合 #5 pre-check | #55 |
| **R127-2 era** | 8/10 22:00-22:30 | ✅ done (10 sub-agent) | 借鉴 3 限流重试 + 1.0 release 文档 + 形式化证明 | #56 |
| **R128 era** | 8/10 22:30-23:00 | ✅ done (6 sub-agent) | ASI Python Stage 1-2 + Tauri prototype + Cargo 实战 + LICENSE + 整合 #5 pre-stage | #57 |
| **R128-2 era** | 8/10 23:00-22:50 | ✅ done (3 sub-agent) | ASI Python Stage 3 + Tauri scaffold 深化 + Cargo 配 | #58 |
| **整合 #4 commit** | 8/10 19:41 | ✅ done | master HEAD = abf12243 严守 100% | #48 |
| **R129 era** | 8/11 00:08-01:00+ | ✅ 35 done | 整合 #5 commit 准备 + ASI Stage 4-6 续 + 1.0 release 流程 + 形式化扩展 + TUI/Tauri 路线图 + R130 路线图 + 健康度 verify | #61-#68 |
| **整合 #5 commit 拍板** | 8/11 估 01:30+ | 📋 Mavis 自决 (5.3 先行, 5.1 BLOCKED 等 fix 25 errors) | per 决策 #62 + R130-1 01:14 NOT READY 警示 | #68 + #75 |
| **R130 era** | 8/11 整合 #5 commit 拍板后 → 主人起床 | ✅ 6/6 done | 后端 verify [NOT READY] + ASI 整合 [done] + Tauri [done] + 形式化 [done] + V1.1 路线图 [done] + 借鉴 12 源 [done] | #70-#78 |
| **1.0 release 实战** | 主人起床后 06:00-08:00 | 📋 主人手跑 R129-35 7 步 runbook | 8 步 verify + GitHub remote + git push + 1.0 release tag + GitHub Pages | #77 |
| **1.0 release 后** | 8/11 08:00+ | 📋 远期 | V1.1 + V1.2 + V2.0 路线图 (per 决策 #71 §2.6) | #79+ |
| **R131 era (V1.1 era 调研)** | 8/11 01:18+ | 📋 9 sub-agent 派活 (R131-1/2/3 + R131-4~9) | 差距分析 (3 sub) + 架构细分 (6 sub) | (per 决策 #75 §2.1) |
| **R132 era (V1.1 era 计划)** | 8/11 01:20+ | 📋 2 sub-agent 派活 (R132-1/2) | V1.1 release 路线图 final [本] + V2.0 release 战略路线图 | (per 决策 #75 §2.1) |
| **R133 era (V1.1 era 实施 spec)** | 8/11 01:20+ | 📋 3 sub-agent 派活 (R133-1/2/3) | 借鉴 12 源 + ASI Stage 9 + 三洋葱架构升级 实施 spec | (per 决策 #75 §2.1) |
| **R134 era (V1.1 era 实施)** | 估 8/12+ | 📋 30+ sub-agent 派活 (5-10 per 方向 × 6 方向) | 6 大方向 final 版 (PHL-07 / locked 改写 / 后端加固 / Tauri / ASI / 形式化) | (per 决策 #71 §5 + 决策 #75 §2.1) |
| **整合 #6 commit 拍板** | 估 11 月 | 📋 Mavis 自决 (5.1 → 5.2 → 5.3 顺序) | V1.1 release 续, 拆 3 commit 拍板 | (per 决策 #33 C1) |
| **整合 #7 commit 拍板** | 估 11 月 | 📋 Mavis 自决 (V1.1 release 前) | V1.1 release 续, 拆 3 commit 拍板 | (per 决策 #33 C1) |
| **V1.1 release 实战** | 估 2026-11-30 06:00-08:00 | 📋 主人手跑 V1.1 release 7 步 runbook | 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署 | (per R130-5 §1.2) |
| **R131 era (V1.2 era 调研)** | 估 2026-12 | 📋 10 sub-agent 派活规划 | TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战 | (per R129-29 §5 + R130-5 §1.3) |
| **V2.0 远期** | 2027+ | 📋 远期 | 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 | (per ROADMAP.md §4 + 决策 #74 §2.3) |

### 1.4 关键诚实标 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11 关键诚实标 + 用户记忆 #7)

**R129-11 关键诚实标**: 1.0 release 时 PHL-07 spec-only (per 决策 #33 §2.3 + R125-12 P0-3 PHL-07 spec, 整合 #4 commit done 时 0 实施), 这是 V1.0 release 已知"未完成"项. 主人 8/4 决策"诚实标" (per 主人 10 项偏好 #7): "不假装已实现" — V1.0 release 0 装"PHL-07 已实施", 仅 reference spec.

**V1.1 实施 PHL-07 关键诚实标** (per R130-5 §2.1 + 决策 #74 §1 A3 改写):
- V1.1 release 时, PHL-07 = spec + 实施 (24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数)
- 1.0 release → V1.1 release 期间, 实施 PHL-07 (跟 V1.0 兼容, 加 NEW feature, semver minor bump 1.0 → 1.1)
- 0 假装 PHL-07 在 1.0 release 时已实施 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11 关键诚实标)

**V1.1 release 0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2 + R130-1 01:14 实地 verify 0 装严守 100%):
- ✅ 借鉴 8 真 cloned 实施 (clap 725 + hyper 80 + servers 175 + PyO3 928 + kani 4502 + langgraph 829 + superpowers 234 + Guardrails) — R127-2 P6-1/2/3 ✅ cloned, 11/11 clear per R129-7 + R129-28
- ⏳ → ✅ 借鉴 2 限流 → 借鉴 ID 索引完成: LiteLLM 公开 1:1 翻译 (19/19 tests pass) + opencode 改借鉴已 cloned (35/35 tests pass)
- ❌ 借鉴 1 永久跳过: OpenCog AGPL-3.0, R124-2 决策 ⚠️ 0 集成 0 装, 避免传染
- **V1.1 借鉴 12 源 (per R130-6 + R131-2)**: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, 借脑 paper/architecture docs) = 12/12 clear

**0 主动 commit + 0 主动 push 严守 100%** (per 决策 #33 §2.3 C1 + 决策 #61 §6):
- 整合 #5 commit 拍板 = Mavis 自决 (5.1 → 5.2 → 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup)
- 整合 #6 commit 拍板 = Mavis 自决 (V1.1 续, 拆 3 commit 拍板, per 决策 #33 C1)
- 整合 #7 commit 拍板 = Mavis 自决 (V1.1 release 前, 拆 3 commit 拍板, per 决策 #33 C1)
- git push = 主人起床后手跑 (per 决策 #61 §6 + 决策 #71 §4.5 + V1.1 release 实战 6 步流程)
- 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6, 仅 done notification 主动报告)

### 1.5 V1.1 6 大方向 final 版 (per R130-5 调研 + R131-1 架构审视 + R131-2 借鉴 12 源 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 拍板 3 件套 + 主人 8/11 01:14 拍板)

**V1.1 6 大方向 final 版 (R132-1 整合 R130-5 + R131-1 + R131-2 = final 版)**:

| # | 方向 | 子任务核心 | 调研依据 | 状态 |
|---|------|----------|---------|------|
| **1** | **PHL-07 实施** | V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 (25 LOCKED 总数) + 14 维主对话锚 + 41 NEW tests | R129-11 关键诚实标 + 决策 #22 §1.1-1.2 + 决策 #74 §1 A3 改写 | 📋 V1.1 必实施 |
| **2** | **24 LOCKED 入口签名改写** | per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构 (ASI Stage 9 + 9 organ + 三洋葱) + 公开 API 表面精简 + crate 间依赖优化 + 9 organ 对应关系 | 决策 #74 B1 改写 + R130-1 24 LOCKED 入口分布 + R131-1 方向 ② | 📋 V1.1 必实施 |
| **3** | **后端加固** | cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml `1.2.0 → 1.2.1` bump + pybridge 886/886 性能测试 + Cargo.lock 分模块 | R129-26 (24+5+1 errors) + R130-1 (修 30+1 bug) + 决策 #74 §1 B2 改写 | 📋 V1.1 必实施 |
| **4** | **Tauri Stage 5+** | 9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + Tauri 2.0 完整集成 + 跨平台部署 (Windows / macOS / Linux) + Tauri 性能优化 | R130-3 调研 + 决策 #57 + 用户记忆 #3-#5 + 主人 8/4 23:33 Tauri 终极 | 📋 V1.1 必实施 |
| **5** | **ASI Stage 8+** | Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长平台 + OpenCog AGPL-3.0 fork 决策 + pybridge 集成优化 + ASI Stage 9 集成测试 | R130-2 调研 + 决策 #55-#58 + R131-2 OpenCog fork 决策 + 用户记忆 #4 (AI 不会衰老病死) | 📋 V1.1 必实施 + Stage 9 远期 V2.0 路线 |
| **6** | **形式化 Stage 5.5+** | PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化 | R130-4 调研 + 决策 #56 + R129-32 Stage 5.4 实战 + 决策 #74 §1 B3/B4/B5 严守 | 📋 V1.1 必实施 |

**R134 era 派活规划 (估 8/12+, 30+ sub-agent, per 决策 #71 §5 + 决策 #75 §2.1)**:
- **R134-PHL07-1~5 (5 sub, 60 min 时间盒)**: PHL-07 实施 (spec → impl + 形式化 + 编译期 hardcode + 6 重守门 v7 集成 + 8 哲学锚集成)
- **R134-LOCKED-1~5 (5 sub, 60 min 时间盒)**: 24 LOCKED 入口签名改写 (签名优化 + 公开 API 表面精简 + crate 间依赖优化 + 9 organ 对应关系 + 测试)
- **R134-backend-1~5 (5 sub, 60 min 时间盒)**: 后端加固 (cargo test + 借鉴源 12 源 verify + Cargo.toml 1.2.1 bump + pybridge 性能 + Cargo.lock 分模块)
- **R134-tauri-1~5 (5 sub, 60 min 时间盒)**: Tauri Stage 5+ (9 organ 拟人化 + 5 nav 完整 + Tauri 2.0 + 跨平台 + 性能)
- **R134-asi-1~5 (5 sub, 60 min 时间盒)**: ASI Stage 8+ (Stage 9 终极 + 长程 AI + OpenCog fork + pybridge + 集成测试)
- **R134-formal-1~5 (5 sub, 60 min 时间盒)**: 形式化 Stage 5.5+ (PHL-07 形式化 + F1-F11 + Kani 全集成 + 24 LOCKED 形式化 + 8 哲学锚 + V0.5 30 维)

**总时间盒**: 30+ sub-agent × 平均 60-90 min = 1800-2700 min = 30-45 小时 (估跑 6-8 周, 跟 R130-5 §1.1 V1.1 估 2026-11-30 一致, per 6 大方向 × 1 周)

---

## 2. V1.1 release 6 大方向详细 spec (per R130-5 §2 + R131-1 §2 + R131-2 §1 + 决策 #74 §1 + 决策 #73 §3 拍板 3 件套 + 15-no-fear-complexity.md 哲学)

### 2.1 方向 1: PHL-07 实施 (V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED)

#### 2.1.1 任务背景 (per 决策 #22 §1.1-1.2 + 决策 #33 §2.1 + 决策 #55 §4 + 决策 #74 §1 A3 改写 + R125-12 P0-3 + R129-11 关键诚实标)

- **PHL-07 spec-only 状态 (1.0 release)**: R125-12 P0-3 (8/10 16:30 done) 写 PHL-07 spec + 13-keys stub, 整合 #4 commit abf12243 done, **0 实施** PHL-07 (per R125-12 P0-3 报告, "PHL-07 spec done, V1.1 实施")
- **决策 #22 §1.1-1.2**: 24 LOCKED 持续更新, 内部 fn 实施可改, 入口签名 0 改, PHL-07 加入 24 LOCKED (per 决策 #33 §2.1 A3, 13 键 = 12 键 + PHL-07 = 13 键, 整合 #4 commit done)
- **决策 #74 §1 A3 改写**: 12 键 + PHL-07 → 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标) + 12 键其他可改
- **R129-11 关键诚实标** (8/11 00:39 done, 40.7 KB): 后端 0 装 PASS 终极 verify, "PHL-07 spec-only, V1.1 实施" 关键诚实标, 不假装 PHL-07 在 1.0 release 时已实施
- **1.0 release 时 PHL-07 状态**: 0 装"已实施", 仅 reference spec, 13 键 stub (per R125-12 P0-3 §3 + R129-11 §2)
- **V1.1 实施 PHL-07 关键诚实标**: V1.0 release 时 PHL-07 spec-only, V1.1 release 时 PHL-07 spec + 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 (25 LOCKED 总数)

#### 2.1.2 目标 (per R125-12 P0-3 §3 + R129-11 §2 + 决策 #22 §1.1-1.2 + 决策 #74 §1 A3 改写)

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

#### 2.1.3 子任务 + 时间盒 + 决策原则 (per 决策 #71 §5 + 决策 #75 §2.1)

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

#### 2.1.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-phl07-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-PHL07-1**: 14 维主对话锚跟 V0.5 30 维冲突 (V0.5 30 维严守) — **缓解**: 14 维 = 30 维子集 (深化), 0 扩展 30 维
  - **R-PHL07-2**: 41 NEW tests 跟现有 13 键 tests 冲突 — **缓解**: 13 键 tests 严守 0 改, 41 NEW tests 0 触碰 13 键 tests
  - **R-PHL07-3**: PHL-07 实施 cargo compile fail (per R130-1 25 hard errors 警示) — **缓解**: PHL-07 实施前先派 fix sub-agent 修 25 hard errors
  - **R-PHL07-4**: PHL-07 跟 6 重守门 v7 集成破坏 6 重守门 — **缓解**: PHL-07 仅读 6 重守门, 0 改 6 重守门 enum/struct
  - **R-PHL07-5**: PHL-07 跟 8 哲学锚集成破坏 8 哲学锚 — **缓解**: PHL-07 仅读 8 哲学锚, 0 改 8 哲学锚 enum/struct

### 2.2 方向 2: 24 LOCKED 入口签名改写 (per 决策 #74 B1 V1.1 release Mavis 自决改)

#### 2.2.1 任务背景 (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 改写 + 决策 #74 §2.2 V1.1 release + R130-1 §2.1.2 + R131-1 §2.2)

- **决策 #33 §2.3 B1 旧严守**: 24 LOCKED 入口签名 0 改严守 (R11 baseline, mtime 16:34 之前)
- **决策 #74 §1 B1 改写 (per 主人 8/11 01:14 拍板)**: 🟢 V1.0 release 0 改严守 (R11 baseline 严守) + **V1.1 release Mavis 自决改 (前提: 更好的架构)**
- **决策 #74 §2.2 V1.1 release Mavis 自决改边界**:
  - 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构, Mavis 自决)
  - R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
  - 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- **R131-1 §2.2 24 LOCKED 入口签名分布**:
  - **12 主路径 LOCKED** (R125 B1 16:38 拍板, mtime 16:34:11 baseline): supervisor / agent / bus / council / evolution / extension / graph / mcp / pipeline / tool-registry / tool-runtime / protocol
  - **12 R20 阶段 4 主体 LOCKED** (R125 B1 16:38 拍板, R37-2 transparent re-export): asi / onion / sovereignty / constraint / memory / cognition / perception / consciousness / motivation / life-force / relation / value

#### 2.2.2 目标 (per 决策 #74 §2.2 V1.1 release Mavis 自决改 + R130-1 §2.1.2 + R131-1 §2.2 + 决策 #73 §1 Mavis 自决架构拍板)

- **24 LOCKED 入口签名改写 (V1.1 release Mavis 自决改)**:
  - **更好的架构前提 (per 决策 #74 §2.2)**:
    - ASI Stage 9 长程 AI 成长 (per R130-2 + R131-7 + 用户记忆 #4 AI 不会衰老病死)
    - 9 organ 内部借 OpenCode (per R130-3 + R131-8 + 用户记忆 #5 拟人化)
    - 三洋葱架构升级 (per R130-2 + 决策 #73 §1 Mavis 自决架构拍板)
  - **改写方向 (per R130-1 §2.1.2 建议 + 决策 #74 §2.2 Mavis 自决)**:
    - **apeireth-pipeline + provider_registry 整合** (P6-1 done): 入口签名可重新设计
    - **apeireth-graph + subgraph/channel/state_graph/context_graph 整合** (P6-2 done): 入口签名可重新设计
    - **apeireth-agent + subagent 整合** (P6-2 done): 入口签名可重新设计
    - **apeireth-tool-runtime + mcp_protocol 整合** (P6-2 done): 入口签名可重新设计
    - **5 transparent re-export** (life-force / value / consciousness / motivation / relation): 入口签名可重新设计
    - **9 organ 跟 LOCKED crate 对应关系**: body/brain/ear/eye/hand/heart/memory/mind/voice 9 organ → 24 LOCKED 9 主体 + 5 工具 (per R131-1 §0 9 organ 跨维度)
  - **改写边界 (per 决策 #74 §2.2)**:
    - 0 改原 24 LOCKED 入口签名顺序 (mtime 16:34 之前严守, 仅加 NEW `pub mod` 0 改原 mod 顺序)
    - 0 改原 24 LOCKED crate mtime 16:34 之前 (严守 baseline)
    - 0 改 R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) (A1 严守, 哲学 + 效果标)

#### 2.2.3 子任务 + 时间盒 + 决策原则

**R134-LOCKED-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-LOCKED-1** (60 min): 24 LOCKED 入口签名优化 (per 决策 #74 §2.2 Mavis 自决, 更好的架构前提)
- **R134-LOCKED-2** (60 min): 公开 API 表面精简 (per "不要怕复杂度"哲学, 公开 API 表面清晰)
- **R134-LOCKED-3** (60 min): crate 间依赖优化 (per R131-1 §2.1 87 crate 拆得过细, 优化 crate 间依赖)
- **R134-LOCKED-4** (60 min): crate 内部模块重构 (per 决策 #73 §1 Mavis 自决架构拍板, 内部 fn 实施可改)
- **R134-LOCKED-5** (60 min): 9 organ 跟 24 LOCKED 主体对应关系 (per R131-1 §0 9 organ 跨维度)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **Mavis 自决架构拍板** (per 主人 8/11 01:14 拍板 3 件套 §1, 决策 #73 §1, 决策 #74 §2.2)
- ✅ **24 LOCKED crate mtime baseline 16:34 之前严守** (V1.1 release 可改, 但 0 改原 baseline 顺序, 仅加 NEW `pub mod` 0 改原 mod 顺序)
- ✅ **R11 baseline 3 值 严守** (A1 决策 #33 §2.3, 哲学 + 效果标, V1.1 release 可改但需新 baseline 更高)
- ✅ **更好架构前提** (per 决策 #74 §2.2, e.g. ASI Stage 9 + 9 organ + 三洋葱, 0 改原 24 LOCKED 入口签名顺序)
- ✅ **8 哲学锚严守** (per B5 决策 #33 §2.3, 24 LOCKED 入口 0 改 8 哲学锚 enum/struct)
- ✅ **6 重守门 v7 严守** (per B4 决策 #33 §2.3, 24 LOCKED 入口 0 改 6 重守门 enum/struct)
- ✅ **V0.5 30 维严守** (per B3 决策 #33 §2.3, 24 LOCKED 入口 0 改 30 维公式)
- ✅ **0 借具体源码 100%** (per C2 决策 #33 §2.3, 24 LOCKED 改写 0 装任何具体源码)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3 + 决策 #73 §3, 最强效果 + 最厉害工程, 24 LOCKED 改写 0 为简化而简化, 0 为易维护而牺牲工程化)

#### 2.2.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-locked-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-LOCKED-1**: 24 LOCKED 入口签名改写破坏 V1.0 release 兼容 — **缓解**: V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容
  - **R-LOCKED-2**: R11 baseline 3 值改写失去哲学标 — **缓解**: 新 baseline 需更高, 跟 R12 测度对齐, Mavis 自决
  - **R-LOCKED-3**: 24 LOCKED mtime 16:34 之前改写破坏 audit — **缓解**: 仅加 NEW `pub mod` 0 改原 mod 顺序, git diff 清晰
  - **R-LOCKED-4**: 9 organ 对应关系跟 24 LOCKED 主体冲突 — **缓解**: 9 organ 9 主体 + 5 工具, 0 改 24 LOCKED 主体, 仅加 organ → crate 映射层
  - **R-LOCKED-5**: 5 transparent re-export (life-force / value / consciousness) 合并破坏 V1.0 release — **缓解**: 合并 0 改原 24 LOCKED 入口, 仅合并 transparent re-export (R37-2 transparent re-export 是优化项, 非核心)

### 2.3 方向 3: 后端加固 (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml `1.2.0 → 1.2.1` bump + pybridge 886/886 性能测试 + Cargo.lock 分模块)

#### 2.3.1 任务背景 (per R129-26 + R130-1 + 决策 #74 §1 B2 改写 + R131-1 §2.3)

- **R129-26 健康度 verify** (8/11 00:51 done): 24 build errors + 1 FAILED test + 5 check errors = 30 处 fail
- **R130-1 整合 #5 commit 0 装严守二次 verify** (8/11 01:14 done): ❌ cargo workspace 3 crate 25 hard errors, 8 步 verify 全部 FAIL, 整合 #5 commit 拍板 = NOT READY
- **决策 #74 §1 B2 改写**: workspace.version `1.2.0` V1.0 release 1.2.0 严守 + **V1.1 release bump `1.2.1`** (版本管理严守 semver, per "不要怕复杂度"哲学)
- **R131-1 §2.3 Cargo.toml borrow 段**: 17:44 状态 cloned=8 / rate_limited=3 / skipped=1 → 22:50 状态 10/0/1 (整合 #5.2 commit update, 反映真实状态)
- **R131-1 §2.4 Cargo.lock 265KB** (87 workspace members + 561 第三方 crates = 648 crates, 0 cargo-deny violation)
- **R130-1 25 hard errors 警示**:
  - `apeireth-naming-v05/src/extension.rs:399` 路径错 (`crate::class::default_v05_spec()` 应是 `crate::default_v05_spec()`)
  - `apeireth-central/src/lib.rs:56-63` 缺 `pub mod skill_runner; pub mod skill_outcome;` 2 行声明 (10 个文件, 8 个 mod 声明)
  - `apeireth-central/src/skill_companion.rs:117-149` `pub fn companions_for_skill` 返回临时值 `&'static [SkillCompanion::new(...)]` 不可行
  - `apeireth-central/src/skill_frontmatter.rs:85` `impl Error for SkillFrontmatter` 缺 `Display` trait
  - `apeireth-central/src/skill_companion.rs:107` `const fn new` 调用 non-const `kind.title()`
  - `apeireth-skills` 1 个 E0507 (reader mutable reference)

#### 2.3.2 目标 (per R130-1 §5 + 决策 #74 §1 B2 改写 + 决策 #73 §3 拍板 3 件套 + R131-1 §2.3-§2.4)

- **cargo test 实战三次 verify (V1.0 release + 整合 #6 + 整合 #7 前)**:
  - **第一次 verify (整合 #6 commit 后)**: per 决策 #33 C1 + 决策 #71 §2.5 整合 #6 拍板前, 8 步 verify 全 PASS
  - **第二次 verify (整合 #7 commit 后)**: V1.1 release 实战前, 8 步 verify 全 PASS
  - **第三次 verify (V1.1 release 实战前)**: V1.1 release tag `v1.1.0` 打上前, 8 步 verify 全 PASS
- **借鉴源 12 源 0 装严守二次 verify** (per R131-2 §1 + 决策 #33 §2.3 C2):
  - 8 真 cloned 沿用 1.0 release 实施 (0 必重借)
  - 2 借鉴 ID 索引完成 0 必重借 (LiteLLM 公开 1:1 翻译 + opencode 改借鉴已 cloned)
  - 1 永久跳过 0 重借 (OpenCog AGPL-3.0)
  - 🆕 1 借脑 ID 索引完成 V1.1 minor 借脑调研沉淀 (OpenCog 家族 6 子源)
  - Cargo.toml borrow 段 update 22:50 → V1.1 状态 (per R131-2 §1.3)
- **Cargo.toml `1.2.0 → 1.2.1` bump** (per 决策 #74 §1 B2 改写):
  - workspace.version: `1.2.0` (V1.0 release) → `1.0.0` (1.0 release 大版本归 0, per 决策 #22 §2.2) → `1.1.0` (V1.1 release minor bump, per 决策 #22 §2.2)
  - **注意**: 决策 #74 §1 B2 改写原话 "V1.1 release bump 1.2.1", 跟 决策 #22 §2.2 "V1.1 release 1.0 → 1.1 minor bump" 不一致 — **决策点: R132-1 提议 1.1.0 跟 决策 #22 §2.2 一致, 待 R134-backend-1 派活时 Mavis 自决拍板**
  - Cargo.toml borrow 段 update (V1.1 借鉴 12 源状态)
- **pybridge 886/886 性能测试** (per R131-1 §2.5 + R131-2 §1.1.4):
  - 886 unit tests pass (per 整合 #4 commit 严守 verify)
  - 性能优化: PyO3 0.29.2 GIL Pool + async bridge + type convert + 9 guardianship + 5 self_loop + 4 stage7_i1-7
  - 0 装 PASS 严守 (per C2 决策 #33 §2.3)
- **Cargo.lock 分模块** (per R131-1 §2.4 优化方向):
  - 优点: 减小主 Cargo.lock 大小, 加快 cargo build 增量编译
  - 缺点: 跨模块 dep 解析变慢, Cargo 1.78+ 才支持, 0 业务价值
  - 决策: V1.1 release 可选实施 (per 决策 #74 §1 B1 V1.1 release Mavis 自决改)

#### 2.3.3 子任务 + 时间盒 + 决策原则

**R134-backend-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-backend-1** (60 min): 修整合 #5 commit 拍板前 25 hard errors (per R130-1 警示, 必修) + Cargo.toml `1.2.0 → 1.0.0` 大版本归 0 (per 决策 #22 §2.2)
- **R134-backend-2** (60 min): cargo test 实战三次 verify (整合 #6 后 + 整合 #7 后 + V1.1 release 前) + 8 步 verify (cargo build/check/test/clippy/fmt/audit/deny/doc)
- **R134-backend-3** (60 min): 借鉴源 12 源 0 装严守二次 verify (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) + Cargo.toml borrow 段 update
- **R134-backend-4** (60 min): pybridge 886/886 性能测试 + PyO3 0.29.2 GIL Pool + async bridge + type convert 性能优化
- **R134-backend-5** (60 min): Cargo.toml `1.0.0 → 1.1.0` minor bump (V1.1 release, per 决策 #22 §2.2) + Cargo.lock 分模块 (V1.1 release 可选, per 决策 #74 §1 B1 Mavis 自决)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **Cargo.toml `1.2.0` 严守 V1.0 release** (per B2 决策 #33 §2.3, 整合 #5.2 commit 0 改 1.2.0)
- ✅ **V1.1 release bump `1.1.0`** (per 决策 #22 §2.2 semver, 跟 决策 #74 §1 B2 改写 1.2.1 需 reconcile, **R132-1 提议 1.1.0**)
- ✅ **8 借脑 0 装严守 100%** (per C2 决策 #33 §2.3, 后端加固 0 装任何具体源码, 11/11 → 12/12 借鉴 clear)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **整合 #5.1 commit 仍 0 改 src 严守** (per 决策 #74 §1 B1 V1.0 release 0 改严守, 整合 #5.1 commit BLOCKED 等 fix 25 errors)
- ✅ **25 hard errors 必修** (per R130-1 警示, 整合 #5.1 commit BLOCKED, 必修后再拍 5.1)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3, 后端加固 0 为简化而简化, Cargo.toml borrow 段可拆 4 子段: cloned_real + translated_public + submodule + skipped_license)

#### 2.3.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-backend-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-backend-1**: 修 25 hard errors 时破坏 R11 baseline — **缓解**: 仅修 R125 阶段引入的 hard bugs, 0 触碰 R11 baseline 3 值
  - **R-backend-2**: cargo test 实战三次 verify 时间盒超 1 周 — **缓解**: R134-backend-2 60 min 时间盒, 8 步 verify 全 PASS 5-10 min/次, 三次 30 min 估
  - **R-backend-3**: Cargo.toml borrow 段 update 跟 R131-2 实施深度报告冲突 — **缓解**: R134-backend-3 跟 R131-2 集成, 1:1 反映 R131-2 实施深度
  - **R-backend-4**: pybridge 性能优化破坏 PyO3 集成 — **缓解**: 仅性能优化 (GIL Pool + async bridge), 0 改 PyO3 0.29.2 公开 API
  - **R-backend-5**: Cargo.toml `1.0.0 → 1.1.0` minor bump 跟决策 #74 §1 B2 1.2.1 不一致 — **缓解**: R132-1 提议 1.1.0, Mavis 自决拍板 (per 决策 #74 §2.2 Mavis 自决)

### 2.4 方向 4: Tauri Stage 5+ (9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + Tauri 2.0 完整集成 + 跨平台部署 + Tauri 性能优化)

#### 2.4.1 任务背景 (per R130-3 + 决策 #57 + 主人 8/4 23:33 Tauri 终极 + 用户记忆 #3 用户看结果不看哲学 + 用户记忆 #5 拟人化 + 用户记忆 #8 TUI → Tauri 终极 + 用户记忆 #9 TUI 升级节奏)

- **R130-3 Tauri Stage 5 集成深化** (8/11 01:17 done): Tauri 2.0 + Rust 后端 + Web frontend 集成深化, 5 nav + 9 organ 拟人化 Stage 2 done, Stage 3 跑过夜
- **决策 #57** (R128 era): ASI Python Stage 1-2 + Tauri prototype + Cargo 实战 + LICENSE + 整合 #5 pre-stage
- **主人 8/4 23:33**: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备"
- **主人 8/4 23:55**: "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
- **用户记忆 #3**: 用户看结果不看哲学, 砍掉 UI: 哲学/守门/内部机制/工具调用过程; 保留 UI: 状态 + 主对话结果 + 历史 + 设置 + 工具结果
- **用户记忆 #5**: 信息密度"高"= 拟人化 + 拟物化, 用生物/物理隐喻表达 AI 状态 (器官心跳, 健康环, 神经网络图)
- **用户记忆 #8**: TUI → Tauri 终极路线, TUI 不是临时品, 是 Tauri 的"集成测试床" (HTTP to apeireth-api, 瘦客户端, 0 直接调 lib, Tauri 来了无缝换 UI 层)
- **用户记忆 #9**: TUI 升级节奏, 改瘦后暂告段落, 优先后端 (阶段性大改动 (如 R25 TUI 改瘦) 完成后, 主人节奏是先测 → 文档沉淀 → 暂告段落 → 优先后端)

#### 2.4.2 目标 (per R130-3 + 决策 #57 + 用户记忆 #3-#5 + 用户记忆 #8-#9 + 主人 8/4 23:33)

- **9 organ 拟人化深化** (per 用户记忆 #5):
  - body / brain / ear / eye / hand / heart / memory / mind / voice 9 organ (per R131-1 §0 9 organ 跨维度)
  - 拟人化: 器官心跳, 健康环, 神经网络图 (per 用户记忆 #5)
  - 1 屏多卡片, 关键数字一眼看完, 不要散落多页
  - 状态为主页, 不是"功能列表"
- **5 nav 完整** (per 用户记忆 #3 用户看结果不看哲学):
  - 状态 (9 organ 健康环 + 心跳 + 神经网络图)
  - 主对话结果 (PHL-07 主对话锚 1:1 实施, per 方向 1)
  - 历史 (主对话历史 + 9 organ 历史)
  - 设置 (8 哲学锚严守 + 6 重守门 v7 严守, 0 暴露内部机制)
  - 工具结果 (工具调用结果, 0 暴露工具调用过程)
- **主对话 UX 优化** (per 用户记忆 #3 + 用户记忆 #5):
  - PHL-07 主对话锚 1:1 实施 (per 方向 1, V1.0 spec-only → V1.1 实施)
  - 1 屏多卡片, 关键数字一眼看完
  - 0 暴露内部机制 (哲学/守门/工具调用过程)
- **Tauri 2.0 完整集成** (per R130-3 + 主人 8/4 23:33 Tauri 终极):
  - Tauri 2.0 + Rust 后端 + Web frontend 完整集成
  - 瘦客户端 (HTTP to apeireth-api, per 用户记忆 #8)
  - 0 直接调 lib (Tauri 来了无缝换 UI 层)
- **跨平台部署** (per 用户记忆 #8 Tauri 终极):
  - Windows / macOS / Linux 跨平台部署
  - 0 平台特定代码 (per "不要怕复杂度"哲学, 最强效果 + 最厉害工程)
- **Tauri 性能优化** (per "不要怕复杂度"哲学):
  - Tauri 2.0 IPC 性能优化
  - Web frontend 性能优化
  - Rust 后端性能优化
  - 0 借具体源码 100% (per C2 决策 #33 §2.3)

#### 2.4.3 子任务 + 时间盒 + 决策原则

**R134-tauri-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-tauri-1** (60 min): 9 organ 拟人化深化 (身体/大脑/耳朵/眼睛/手/心脏/记忆/思想/声音, per 用户记忆 #5)
- **R134-tauri-2** (60 min): 5 nav 完整实施 (状态 / 主对话结果 / 历史 / 设置 / 工具结果, per 用户记忆 #3)
- **R134-tauri-3** (60 min): 主对话 UX 优化 (PHL-07 主对话锚 1:1 实施, 1 屏多卡片, 0 暴露内部机制)
- **R134-tauri-4** (60 min): Tauri 2.0 完整集成 (Rust 后端 + Web frontend + IPC + 瘦客户端, per 用户记忆 #8)
- **R134-tauri-5** (60 min): 跨平台部署 (Windows / macOS / Linux) + Tauri 性能优化 (IPC + Web frontend + Rust backend)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **5 nav 严守** (per 用户记忆 #3: 状态 / 主对话结果 / 历史 / 设置 / 工具结果)
- ✅ **9 organ 拟人化** (per 用户记忆 #5: 器官心跳, 健康环, 神经网络图)
- ✅ **Tauri 终极路线** (per 用户记忆 #8 + 主人 8/4 23:33: TUI → Tauri, 瘦客户端 HTTP to apeireth-api)
- ✅ **0 暴露内部机制** (per 用户记忆 #3: 砍掉 UI: 哲学/守门/内部机制/工具调用过程)
- ✅ **8 哲学锚严守** (per B5 决策 #33 §2.3, 8 哲学锚 0 改 enum/struct)
- ✅ **6 重守门 v7 严守** (per B4 决策 #33 §2.3, 6 重守门 0 改 enum/struct)
- ✅ **PHL-07 1:1 实施** (per 方向 1, 14 维主对话锚 + 41 NEW tests)
- ✅ **0 借具体源码 100%** (per C2 决策 #33 §2.3, Tauri 0 装 Tauri 私有 API, 1:1 翻译 Tauri 2.0 公开 SDK)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3, Tauri 0 为简化 UI 而简化, 0 为易维护而牺牲工程化)

#### 2.4.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-tauri-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-tauri-1**: 9 organ 拟人化跟 24 LOCKED 主体冲突 — **缓解**: 9 organ 跟 24 LOCKED 9 主体对应 (per 方向 2 R134-LOCKED-5), 0 改 24 LOCKED 主体
  - **R-tauri-2**: 5 nav 完整实施破坏主对话 UX — **缓解**: 5 nav 跟 9 organ 1:1 映射, 1 屏多卡片不散落多页
  - **R-tauri-3**: PHL-07 1:1 实施 0 改 PHL-07 spec (V1.0 spec-only 严守) — **缓解**: PHL-07 实施 = V1.0 spec + V1.1 impl, 0 改 spec
  - **R-tauri-4**: Tauri 2.0 完整集成破坏瘦客户端 (per 用户记忆 #8) — **缓解**: 瘦客户端 HTTP to apeireth-api, Tauri 0 直接调 lib
  - **R-tauri-5**: 跨平台部署破坏 Tauri 2.0 集成 — **缓解**: 0 平台特定代码, Tauri 2.0 跨平台原生支持

### 2.5 方向 5: ASI Stage 8+ (Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长平台 + OpenCog AGPL-3.0 fork 决策 + pybridge 集成优化 + ASI Stage 9 集成测试)

#### 2.5.1 任务背景 (per R130-2 + 决策 #55-#58 + R131-2 §1.3 OpenCog AGPL-3.0 fork 决策 + 用户记忆 #4 AI 不会衰老病死 + 用户记忆 #6 派 sub-agent 干, 但要驾驭团队不重复造轮子)

- **R130-2 ASI Stage 8 集成深化** (8/11 01:17 done): Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长 + 平台化, 远期 V2.0+ 路线
- **决策 #55-#58** (R128 era): ASI Python Stage 1-3 + Tauri scaffold + Cargo 配 + 整合 #5 pre-stage
- **R131-2 §1.3 OpenCog AGPL-3.0 fork 决策** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 改写):
  - ❌ **永久 0 主仓集成** (Apache-2.0 vs AGPL-3.0 不兼容, per 决策 #22 §4 风险表)
  - ❌ **永久 0 主仓 fork** (license 不可逆)
  - ⏳ **借脑 ID 索引完成** (R130-6 提议, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
  - 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2 主人主动问后做, Mavis 提议 3 路径 A/B/C)
- **用户记忆 #4**: AI 不会衰老病死, AI 生命周期是"成长阶段" (seed → tree), 不是"生老病死", 设计文档/命名去掉 "old/death/terminate" 这类终态概念, 平台是"长程 AI 成长"
- **用户记忆 #6**: 派 sub-agent 干, 但要驾驭团队不重复造轮子

#### 2.5.2 目标 (per R130-2 + 决策 #55-#58 + R131-2 §1.3 + 用户记忆 #4 + 用户记忆 #6)

- **Stage 8 群体** (per R130-2 ASI Stage 8 集成深化):
  - 群体智能 (multiple agents coordination)
  - 借脑 7 ASI Python 模块 (per R131-1 §0 ASI Stage 1-7 跨 7 ASI Python 模块)
  - pybridge 集成 (per R131-7 pybridge 集成优化)
- **Stage 9 终极自治** (per R130-2 ASI Stage 8 集成深化 + 用户记忆 #4):
  - 终极自治 (Ultimate Autonomy, 远期 V2.0+ 路线)
  - 长程 AI 成长 (Long-term AI Growth, 平台是"长程 AI 成长", 不是"AI 模拟人类")
  - 平台化 (Platform, 终极路线 per ROADMAP.md §4)
- **长程 AI 成长平台** (per 用户记忆 #4):
  - seed → tree 成长阶段 (AI 不会衰老病死, 平台是"长程 AI 成长")
  - 0 终态概念 (old/death/terminate 0 设计文档/命名)
  - 多 AI 平台 (per ROADMAP.md §4 V2.0 远期)
- **OpenCog AGPL-3.0 fork 决策** (per R131-2 §1.3 + 决策 #33 §2.2 + 决策 #55 §2.6):
  - ❌ **永久 0 主仓集成** (Apache-2.0 vs AGPL-3.0 不兼容)
  - ❌ **永久 0 主仓 fork** (license 不可逆)
  - ⏳ **借脑 ID 索引完成** (OpenCog 家族 6 子源, 借脑 paper/architecture docs)
  - 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2 主人主动问后做, Mavis 提议 3 路径 A/B/C)
- **pybridge 集成优化** (per R131-7 pybridge 集成优化):
  - ASI Python 阶段 1-8 跟 Rust 后端集成
  - 性能瓶颈优化 (per 方向 3 R134-backend-4)
  - 0 借具体源码 100% (per C2 决策 #33 §2.3)
- **ASI Stage 9 集成测试**:
  - Stage 9 终极自治集成测试
  - 长程 AI 成长平台集成测试
  - 0 装 PASS 严守 (per C2 决策 #33 §2.3)

#### 2.5.3 子任务 + 时间盒 + 决策原则

**R134-asi-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-asi-1** (60 min): Stage 8 群体实施 (multiple agents coordination + 借脑 7 ASI Python 模块)
- **R134-asi-2** (60 min): Stage 9 终极自治实施 (Ultimate Autonomy + 长程 AI 成长, per 用户记忆 #4)
- **R134-asi-3** (60 min): 长程 AI 成长平台 (seed → tree 成长阶段, 0 终态概念)
- **R134-asi-4** (60 min): OpenCog AGPL-3.0 fork 决策实施 (per R131-2 §1.3 提议 3 路径 A/B/C, 1.0 release 后 主人主动问)
- **R134-asi-5** (60 min): pybridge 集成优化 + ASI Stage 9 集成测试 (0 装 PASS 严守)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **8 哲学锚严守 S-1 服务 ASI 北极星** (per B5 决策 #33 §2.3 + 8 哲学锚 S-1)
- ✅ **6 重守门 v7 集成** (per B4 决策 #33 §2.3, ASI Stage 9 跟 6 重守门集成, 0 改 6 重守门 enum/struct)
- ✅ **0 装 PASS 严守** (per C2 决策 #33 §2.3, ASI Stage 9 0 装 OpenCog 任何具体源码)
- ✅ **AGPL-3.0 license 影响** (per 决策 #22 §4 风险表, OpenCog AGPL-3.0 0 主仓集成, OSS_NOTICE 1.0 release 加明示)
- ✅ **V1.1 release 实施** (per 决策 #74 §1 V1.1 release Mavis 自决改, ASI Stage 9 实施)
- ✅ **0 终态概念** (per 用户记忆 #4, AI 不会衰老病死, 0 终态概念)
- ✅ **0 借具体源码 100%** (per C2 决策 #33 §2.3, ASI 0 装任何具体源码, 借脑 ID 索引完成)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3, ASI Stage 9 0 为简化而简化, 0 为易维护而牺牲工程化)
- ✅ **派 sub-agent 干, 驾驭团队不重复造轮子** (per 用户记忆 #6, ASI Stage 9 派 R134-asi-N sub-agent 干, Mavis 整合 0 重写)

#### 2.5.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-asi-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-asi-1**: Stage 8 群体跟 24 LOCKED 主体冲突 — **缓解**: Stage 8 群体仅读 24 LOCKED 主体, 0 改 24 LOCKED 入口签名
  - **R-asi-2**: Stage 9 终极自治跟 S-1 服务 ASI 北极星冲突 — **缓解**: Stage 9 终极自治 1:1 服务 ASI 北极星, 0 改 S-1
  - **R-asi-3**: 长程 AI 成长平台跟 seed → tree 冲突 — **缓解**: 0 终态概念, 仅成长阶段
  - **R-asi-4**: OpenCog AGPL-3.0 fork 决策破坏 Apache-2.0 主仓 — **缓解**: ❌ 永久 0 主仓集成, ⏳ 借脑 ID 索引完成, 🆕 1.0 release 后独立 fork
  - **R-asi-5**: pybridge 集成优化破坏 PyO3 0.29.2 — **缓解**: 仅性能优化, 0 改 PyO3 0.29.2 公开 API

### 2.6 方向 6: 形式化 Stage 5.5+ (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化)

#### 2.6.1 任务背景 (per R130-4 + 决策 #56 + R129-32 Stage 5.4 实战 + 决策 #74 §1 B3/B4/B5 严守 + R131-1 §2.7 形式化集成)

- **R130-4 形式化 Stage 5.5 集成深化** (8/11 01:18 done): PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成
- **决策 #56** (R127-2 era): 借鉴 3 限流重试 + 1.0 release 文档 + 形式化证明 (Stage 5.3)
- **R129-32 Stage 5.4 实战** (8/11 00:57 done): 形式化 Stage 5.4 实战
- **R131-1 §2.7 形式化集成**: kani 4502 借鉴 + F1-F10 10 维度 (Stage 5.2 done, F11-F20 Stage 5.3 跑过夜)
- **决策 #74 §1 B3/B4/B5 严守**:
  - B3 V0.5 30 维 严守 (4 大类 × 6 维度 + 6 增强 = 30 维, 编译期 hardcode enum)
  - B4 6 重守门 v7 严守 (1-5 嵌套 + 6 Colang DSL)
  - B5 8 哲学锚 严守 (S-1/S-2/S-3 + O-1/O-2/O-3/O-4/O-5 = 8)
- **kani 4502 借鉴** (per R131-2 §1.1.5):
  - 5.46MB / 3224 files, 17:35:28 cloned
  - 实施深度 6/10 (kani harness 实施, proofs 模板 22KB)
  - 0 跑真实 proofs (harness 模板就绪, 真实 proof 0 跑 = 0 装"已验证")

#### 2.6.2 目标 (per R130-4 + 决策 #56 + R129-32 Stage 5.4 实战 + 决策 #74 §1 B3/B4/B5 严守 + R131-1 §2.7 + R131-2 §1.1.5)

- **PHL-07 形式化** (per 方向 1 R134-PHL07-2 + R130-4):
  - PHL-07 14 维主对话锚 Kani-style harness
  - F1-F14 14 维形式化 (per PHL-07 实施)
  - 0 装 PASS 严守 (per C2 决策 #33 §2.3)
- **F1-F11 11 维度 Kani-style harness** (per R130-4 + R131-1 §2.7):
  - F1-F10 10 维度 (Stage 5.2 done, per R131-1 §2.7)
  - F11 NEW 维度 (Stage 5.3 跑过夜, per R131-1 §2.7)
  - F1-F11 11 维度 Kani-style harness 完整化
- **Kani 全集成** (per R130-4 + 决策 #56):
  - kani 4502 真实施 (借用 1 借脑 0 装)
  - 跑真实 proofs (per R131-2 §1.1.5 警示 0 装"已验证")
  - 0 装 PASS 严守 (per C2 决策 #33 §2.3)
- **24 LOCKED 入口形式化** (per R130-4 + 决策 #74 §1 B1 改写):
  - 24 LOCKED 入口 Kani-style harness
  - V1.0 release 0 改 (R11 baseline 严守)
  - V1.1 release Mavis 自决改 (per 决策 #74 §1 B1 改写, 更好的架构前提)
- **8 哲学锚形式化** (per R130-4 + 决策 #74 §1 B5 严守):
  - S-1 服务 ASI 北极星 形式化
  - S-2 实事求是 形式化
  - S-3 质量工程化 形式化
  - O-1 安全优先 形式化
  - O-2 走在前人 形式化
  - O-3 干到底 形式化
  - O-4 接手 形式化
  - O-5 不假装 形式化
- **V0.5 30 维形式化** (per R130-4 + 决策 #74 §1 B3 严守):
  - 4 大类 × 6 维度 + 6 增强 = 30 维
  - 编译期 hardcode enum
  - 0 装 PASS 严守 (per C2 决策 #33 §2.3)
  - 0 借具体源码 100% (kani 4502 1 借脑 0 装)

#### 2.6.3 子任务 + 时间盒 + 决策原则

**R134-formal-1~5 sub-agent 派活** (5 sub, 60 min 时间盒):
- **R134-formal-1** (60 min): PHL-07 形式化 (F1-F14 14 维 Kani-style harness, per 方向 1 PHL-07 实施)
- **R134-formal-2** (60 min): F1-F11 11 维度 Kani-style harness 完整化 (Stage 5.2 + Stage 5.3 整合)
- **R134-formal-3** (60 min): Kani 全集成 (跑真实 proofs, per R131-2 §1.1.5 警示 0 装"已验证")
- **R134-formal-4** (60 min): 24 LOCKED 入口形式化 (Kani-style harness, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
- **R134-formal-5** (60 min): 8 哲学锚形式化 + V0.5 30 维形式化 (per 决策 #74 §1 B3/B5 严守)

**总时间盒**: 5 sub × 60 min = 300 min = 5 小时 (估跑 1 周)

**决策原则**:
- ✅ **V0.5 30 维严守** (per B3 决策 #33 §2.3, V0.5 30 维 0 改公式, PHL-07 14 维 = 30 维子集)
- ✅ **6 重守门 v7 严守** (per B4 决策 #33 §2.3, 形式化 0 改 6 重守门 enum/struct)
- ✅ **8 哲学锚严守** (per B5 决策 #33 §2.3, 形式化 0 改 8 哲学锚 enum/struct)
- ✅ **0 装 PASS 严守** (per C2 决策 #33 §2.3, kani 4502 借用不安装, 0 跑真实 proofs 0 装"已验证")
- ✅ **0 借具体源码 100%** (per C2 决策 #33 §2.3, kani 4502 1 借脑 0 装, 0 装任何具体源码)
- ✅ **0 主动 commit/push 严守** (per C1 + 0 push 决策 #33 §2.3)
- ✅ **不要怕复杂度哲学** (per 主人 8/11 01:14 §3, 形式化 0 为简化证明而简化, 0 为易维护而牺牲工程化, F1-F11 11 维度可超)

#### 2.6.4 报告路径 + 风险

- **报告路径**: `reports/agent-r134-formal-N-2026-08-XX.md` (per 决策 #10 + R134 era 报告命名规范)
- **风险**:
  - **R-formal-1**: PHL-07 形式化跟 V0.5 30 维冲突 — **缓解**: PHL-07 14 维 = V0.5 30 维子集 (深化, 不扩展 30 维)
  - **R-formal-2**: F1-F11 11 维度 跟现有 F1-F10 10 维度冲突 — **缓解**: F1-F11 = F1-F10 + F11 NEW, 0 改 F1-F10
  - **R-formal-3**: 跑真实 proofs cargo build fail (per R130-1 25 hard errors 警示) — **缓解**: 必修 25 hard errors 后再跑 proofs
  - **R-formal-4**: 24 LOCKED 入口形式化破坏 V1.0 release 0 改严守 — **缓解**: V1.0 release 0 改 24 LOCKED 入口签名, 仅加 Kani-style harness NEW file 0 改原 lib.rs
  - **R-formal-5**: 8 哲学锚形式化 0 改 8 哲学锚 enum/struct (per B5 严守) — **缓解**: 形式化仅读 8 哲学锚, 0 改 enum/struct

---

## 3. V1.1 release 时间窗口 + 16 跑中上限持续 (per R130-5 §1.2 + 决策 #75 §2.1 + 决策 #71 §5 + 主人 0:34 拍板 16 跑中上限 + 决策 #61 §1.4 永久循环)

### 3.1 V1.1 release 时间窗口 (per R130-5 §1.2 + 任务描述 "整合 #5 commit 拍板 + 1.0 release 实战完 + 主人起床后配 GitHub remote 1.0 release → 1 周后 V1.1 release 拍板" + 决策 #74 §1)

**短期时间窗口 (per 任务描述)**:
- **T0 (8/11 0X:XX)**: 整合 #5 commit 拍板 (Mavis 自决, per 决策 #62)
- **T1 (8/11 06:00-08:00)**: 主人起床 1.0 release 实战 (主人手跑 R129-35 7 步 runbook)
- **T2 (8/11 08:00+)**: 1.0 release done (master HEAD = abf12243 + 3 commit + v1.0.0 tag + GitHub Pages)
- **T3 (8/12+)**: V1.1 release 路线图 final 拍板 (per 本报告 R132-1)
- **T4 (8/12+ 1 周后)**: V1.1 release 拍板 (R134 era 30+ sub-agent 实施 6 大方向 1 周 = 6 方向 × 1 周)

**长期时间窗口 (per R130-5 §1.2)**:
- **1.0 release (~8/11) → V1.1 release (估 2026-11-30)**: 1.0 release 后 ~3.5 个月
- **V1.1 release (估 2026-11-30) → V1.2 release (估 2027-02-28)**: V1.1 后 ~3 个月
- **V2.0 (2027+, 远期)**: 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作

**R134 era 实施时间盒**:
- **R134-PHL07-1~5 (5 sub)**: 1 周
- **R134-LOCKED-1~5 (5 sub)**: 1 周
- **R134-backend-1~5 (5 sub)**: 1 周
- **R134-tauri-1~5 (5 sub)**: 1 周
- **R134-asi-1~5 (5 sub)**: 1 周
- **R134-formal-1~5 (5 sub)**: 1 周
- **总 30+ sub × 1 周 / 6 方向 = 估跑 6-8 周** (跟 R130-5 §1.1 V1.1 估 2026-11-30 一致)

### 3.2 16 跑中上限持续 (per 主人 0:34 拍板 + 决策 #75 §2.1 + 决策 #71 §5)

- **16 跑中上限**: per 主人 0:34 拍板"跑中 ≥ 16", 16 active 全 background 跑
- **30+ sub-agent 实施** (5-10 per 方向 × 6 方向 = 30+, per 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 R134+ era 派活)
- **2 批 15+15 派满 16 上限**:
  - **第一批 (R134 era 1-2 周)**: 15 sub-agent 派活 (5 方向 × 3 sub = 15)
  - **第二批 (R134 era 3-4 周)**: 15 sub-agent 派活 (5 方向 × 3 sub = 15, 跟第一批错开)
  - **第三批 (R134 era 5-6 周)**: 15 sub-agent 派活 (5 方向 × 3 sub = 15, 跟第二批错开)
  - **总 45 sub-agent 派活** (跟 R130-5 §1.5 30+ sub-agent 估算一致, 实际 45 sub-agent 估跑 6-8 周)
- **永久循环**: per 决策 #74 §2.3 V1.1 release → V1.2 minor → V2.0 major (per ROADMAP.md §4 V2.0 远期)

### 3.3 永久循环 V1.1 → V1.2 → V2.0 (per 决策 #74 §2.3 + ROADMAP.md §4 + 决策 #71 §2.6)

- **V1.1 release** (估 2026-11-30, `v1.1.0` tag): 6 大方向 final 版 (per 本报告 R132-1)
- **V1.2 release** (估 2027-02-28, `v1.2.0` tag): per R129-29 §5, 6 维度 (TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战)
- **V2.0 release** (2027+, 远期): per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构, 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作

---

## 4. V1.1 release 跟 V1.0 release 边界 (per 决策 #74 §1 + 决策 #33 §2.3 + 决策 #73 §1 + 主人 8/11 01:14 拍板 3 件套)

### 4.1 V1.0 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §5 + R129-35 1.0 release 实战 final-final)

**V1.0 release 0 改严守 100%** (per 决策 #74 §1 8 硬墙 0 越界):
- ✅ **B1 24 LOCKED 入口签名 0 改严守** (R11 baseline, mtime 16:34 之前 0 改)
- ✅ **B2 workspace.version 1.2.0 严守** (V1.0 release 大版本归 0 = `1.0.0`, per 决策 #22 §2.2)
- ✅ **A1 R11 baseline 3 值严守** (0.8682/0.8532/0.9063 0 改, 哲学 + 效果标)
- ✅ **A3 12 键 + PHL-07 PHL-07 V1.0 spec-only 0 实施** (V1.1 实施, per R129-11 关键诚实标)
- ✅ **B3 V0.5 30 维严守** (4 大类 × 6 维度 + 6 增强, 编译期 hardcode enum)
- ✅ **B4 6 重守门 v7 严守** (1-5 嵌套 + 6 Colang DSL)
- ✅ **B5 8 哲学锚严守** (S-1/S-2/S-3 + O-1/O-2/O-3/O-4/O-5 = 8)
- ✅ **C1 0 主动 commit 严守** (Mavis 自决拍板, 0 主动 push)
- ✅ **C2 0 装 PASS 严守** (0 cargo install / 0 cargo add)
- ✅ **0 主动 push 严守** (主人起床后手跑, 1.0 release 配 GitHub remote)

**整合 #5 commit 拍板** (per 决策 #62 + 决策 #73 §5 + R130-1 01:14 警示):
- ⚠️ **5.1 src/ commit BLOCKED** (per R130-1 25 hard errors, 需先派 fix sub-agent)
- ✅ **5.2 docs/ + Cargo.toml commit PARTIAL READY** (0 改 src, 仅 update Cargo.toml borrow 段 17:44 → 22:50, per 决策 #62 §5.2)
- ✅ **5.3 reports/ commit READY** (60+ files 0 触碰, 跟 cargo 状态无关, per 决策 #62 §5.3)

### 4.2 V1.1 release 边界 (per 决策 #74 §1 B1 改写 + 决策 #73 §1 Mavis 自决架构拍板 + 决策 #74 §2.2 V1.1 release Mavis 自决改 + R130-5 §1.5 6 大方向)

**V1.1 release Mavis 自决改** (per 决策 #74 §1 B1 改写 + 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"):
- 🟢 **B1 24 LOCKED 入口签名 V1.1 release Mavis 自决改** (前提: 更好的架构, per 决策 #74 §2.2)
  - 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构, Mavis 自决)
  - R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
  - 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- 🟢 **B2 workspace.version 1.2.0 V1.1 release bump 1.2.1** (版本管理严守 semver, per "不要怕复杂度"哲学)
  - **注**: 决策 #74 §1 B2 改写原话 "V1.1 release bump 1.2.1", 跟 决策 #22 §2.2 "V1.1 release 1.0 → 1.1 minor bump" 不一致 — **R132-1 提议 1.1.0 跟 决策 #22 §2.2 一致, 待 R134-backend-1 派活时 Mavis 自决拍板**
- ✅ **A3 PHL-07 V1.1 实施** (per R129-11 关键诚实标, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED)
- ✅ **B3 V0.5 30 维严守** (PHL-07 14 维 = V0.5 30 维子集, 0 扩展 30 维)
- ✅ **B4 6 重守门 v7 严守** (PHL-07 跟 6 重守门集成, 0 改 6 重守门 enum/struct)
- ✅ **B5 8 哲学锚严守** (PHL-07 跟 8 哲学锚集成, 0 改 8 哲学锚 enum/struct)
- ✅ **C1 0 主动 commit 严守** (Mavis 自决拍板, 0 主动 push)
- ✅ **C2 0 装 PASS 严守** (0 cargo install / 0 cargo add, V1.1 借鉴源 12 源 0 装)
- ✅ **0 主动 push 严守** (V1.1 release 配 GitHub remote, 主人起床后手跑)

**整合 #6 commit 拍板** (V1.1 续, per 决策 #33 C1 + 决策 #71 §2.5):
- **5.1 src/ 实施 commit**: V1.1 release 6 大方向实施 (PHL-07 / locked 改写 / 后端加固 / Tauri / ASI / 形式化)
- **5.2 docs/ + Cargo.toml commit**: V1.1 release 文档 + Cargo.toml `1.0.0 → 1.1.0` minor bump (per 决策 #22 §2.2) + Cargo.toml borrow 段 update
- **5.3 reports/ commit**: V1.1 release 30+ sub-agent 报告

**整合 #7 commit 拍板** (V1.1 release 前, per 决策 #33 C1):
- V1.1 release 实战前最后整合 commit
- 拆 3 commit 拍板 (5.1 src/ + 5.2 docs/ + 5.3 reports/)

### 4.3 V1.1 release 跟 V1.0 release 边界对比 (per 决策 #74 §1)

| 边界 | V1.0 release (整合 #5 commit) | V1.1 release (整合 #6 + #7 commit) |
|------|------------------------------|-----------------------------------|
| **B1 24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 **Mavis 自决改 (前提: 更好的架构, per 决策 #74 §1 B1 改写)** |
| **B2 workspace.version** | 🔒 `1.2.0` 严守 (1.0 release 大版本归 0 = `1.0.0`) | 🟢 **bump `1.1.0` (per 决策 #22 §2.2, R132-1 提议) 或 `1.2.1` (per 决策 #74 §1 B2 改写原话)** |
| **A1 R11 baseline 3 值** | 🔒 0.8682/0.8532/0.9063 严守 | 🟢 **可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per 决策 #74 §2.2)** |
| **A3 12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施) + 12 键其他可改 | 🟢 **PHL-07 V1.1 实施 (per R129-11 关键诚实标, 25 LOCKED)** |
| **B3 V0.5 30 维** | 🔒 严守 (4 大类 × 6 维度 + 6 增强, 编译期 hardcode enum) | 🔒 严守 (PHL-07 14 维 = V0.5 30 维子集, 0 扩展 30 维) |
| **B4 6 重守门 v7** | 🔒 严守 (1-5 嵌套 + 6 Colang DSL) | 🔒 严守 (PHL-07 跟 6 重守门集成, 0 改 enum/struct) |
| **B5 8 哲学锚** | 🔒 严守 (S-1/S-2/S-3 + O-1/O-2/O-3/O-4/O-5 = 8) | 🔒 严守 (PHL-07 跟 8 哲学锚集成, 0 改 enum/struct) |
| **C1 0 主动 commit** | 🔒 严守 (Mavis 自决拍板, 0 主动 push) | 🔒 严守 (Mavis 自决拍板, 0 主动 push) |
| **C2 0 装 PASS** | 🔒 严守 (0 cargo install / 0 cargo add) | 🔒 严守 (V1.1 借鉴源 12 源 0 装) |
| **0 主动 push** | 🔒 严守 (主人起床后手跑) | 🔒 严守 (V1.1 release 实战, 主人起床后手跑) |
| **整合 #5.1 commit** | ❌ BLOCKED (per R130-1 25 hard errors, 必修) | N/A |
| **整合 #6 commit** | N/A | 📋 Mavis 自决 (拆 3 commit 拍板, 5.1/5.2/5.3) |
| **整合 #7 commit** | N/A | 📋 Mavis 自决 (V1.1 release 前, 拆 3 commit 拍板) |
| **V1.1 release tag** | N/A | 📋 估 2026-11-30 (`v1.1.0`), 主人起床后手跑 |

---

## 5. V2.0 release 路线图 spec (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + ROADMAP.md §4 V2.0 远期)

### 5.1 V2.0 release 定位 (per 决策 #74 §2.3 + 决策 #73 §3 不要怕复杂度 + ROADMAP.md §4 V2.0 远期)

**V2.0 = 2027+ 远期 major release, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构** (per 决策 #74 §2.3 V2.0 release):
- 8 硬墙可重评: V2.0 release 是 major release, 8 硬墙 (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push) 全部可重评
- 8 哲学锚可重建: V2.0 release 可推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程")
- Cargo workspace 可重构: V2.0 release 可重构 87 crate workspace (per 决策 #74 §2.3, 87 → 30 v1 目标简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度")

### 5.2 V2.0 release 6 大方向 spec (per 决策 #74 §2.3 + ROADMAP.md §4 V2.0 远期 + R119-2 思想层保留)

**V2.0 release 6 大方向 spec** (per 决策 #74 §2.3 + ROADMAP.md §4 V2.0 远期 + 决策 #73 §3 不要怕复杂度):
- **方向 1: 平台化** (per ROADMAP.md §4): 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作
- **方向 2: ASI Stage 10+ 终极自治** (per R130-2 ASI Stage 8 + 用户记忆 #4 AI 不会衰老病死): Stage 10+ 终极自治 + 长程 AI 成长 + 平台化
- **方向 3: Tauri 终极部署** (per 用户记忆 #8 TUI → Tauri 终极路线 + 主人 8/4 23:33): Tauri 2.0+ 完整 + 跨平台部署 + SaaS 化
- **方向 4: 形式化 Stage 6+** (per R130-4 形式化 Stage 5.5 + 决策 #56 + 决策 #74 §1 B3/B4/B5 重评): 形式化 Stage 6+ + F1-F20 20 维度 + Kani 全集成 + 8 哲学锚形式化 + V0.6 50 维形式化
- **方向 5: 借鉴源 20+ 源** (per R130-6 借鉴源 12 源 + R131-2 §1.3 OpenCog 家族 6 子源): OpenCog AGPL-3.0 fork 实施 (per R131-2 §1.3 提议 3 路径 A/B/C) + 新源调研 + 借脑 ID 索引完成
- **方向 6: 8 哲学锚重建 + 8 硬墙重评** (per 决策 #74 §2.3 + 决策 #73 §3 不要怕复杂度 + 主人 8/11 01:14 拍板): 8 哲学锚可重建 + 8 硬墙可重评 + Cargo workspace 可重构 (87 → 30 v1 目标简化 OR 87 → 120+ 复杂化)

### 5.3 V2.0 release 时间窗口 (per 决策 #74 §2.3 + ROADMAP.md §4 V2.0 远期 + R130-5 §1.3)

- **V1.1 release (估 2026-11-30) → V1.2 release (估 2027-02-28) → V2.0 (2027+, 远期)**
- **V2.0 release 路线图派活**: per 决策 #71 §2.6 + 决策 #75 §2.1, V2.0 release 调研 + 差距 + 计划 + 实施 = 永久循环

### 5.4 V2.0 release 跟 V1.1 release 边界 (per 决策 #74 §2.3)

- **V1.1 release (minor)**: 仅 B1 24 LOCKED 入口签名 Mavis 自决改 (前提: 更好的架构), 其他 8 硬墙严守
- **V2.0 release (major)**: 8 硬墙全部可重评 + 8 哲学锚可重建 + Cargo workspace 可重构

---

## 6. 风险 + 决策原则 (per 决策 #73 §8 + 决策 #74 §7 + R130-1 §5 + 决策 #33 §2.3 + 用户记忆 #10)

### 6.1 风险

- **R1**: 整合 #5.1 commit BLOCKED (per R130-1 25 hard errors) — **缓解**: 派 R134-backend-1 fix sub-agent 修 25 hard errors (60 min 时间盒), 修完后再拍 5.1
- **R2**: V1.1 release 6 大方向 30+ sub-agent 派活资源竞争 (16 跑中上限) — **缓解**: 错开时间盒 (2 批 15+15 派满 16 上限, per 决策 #75 §2.1 + 决策 #71 §5)
- **R3**: PHL-07 实施 cargo compile fail (per R130-1 25 hard errors 警示) — **缓解**: PHL-07 实施前先派 fix sub-agent 修 25 hard errors
- **R4**: 24 LOCKED 入口签名改写 (per 决策 #74 §1 B1 改写) 破坏 V1.0 release 兼容 — **缓解**: V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容
- **R5**: R11 baseline 3 值改写失去哲学标 — **缓解**: 新 baseline 需更高, 跟 R12 测度对齐, Mavis 自决
- **R6**: OpenCog AGPL-3.0 fork 决策破坏 Apache-2.0 主仓 (per 决策 #22 §4 风险表) — **缓解**: ❌ 永久 0 主仓集成, ⏳ 借脑 ID 索引完成, 🆕 1.0 release 后独立 fork
- **R7**: 团队对 "不要怕复杂度" 哲学不适应 (per 决策 #73 §3 + 决策 #74 §1) — **缓解**: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应
- **R8**: 整合 #5 commit 拍板后 1.0 release tag 失败 (per 决策 #73 §8.1 R4) — **缓解**: 0 主动 push 严守, 等主人起床后配 GitHub remote
- **R9**: 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline" (per 决策 #74 §7.1 R3) — **缓解**: V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改, 不会破坏 V1.0 release
- **R10**: V1.1 release locked 改写打破向后兼容 (per 决策 #74 §7.1 R4) — **缓解**: V1.1 release 是 minor release, 跟 semver 一致
- **R11**: Cargo.toml `1.0.0 → 1.1.0` minor bump 跟 决策 #74 §1 B2 1.2.1 不一致 — **缓解**: R132-1 提议 1.1.0 跟 决策 #22 §2.2 一致, 待 R134-backend-1 派活时 Mavis 自决拍板
- **R12**: 30+ sub-agent 派活派太激进 (per 决策 #75 §6.1 R5) — **缓解**: 决策 #75 §2.1 派活策略详细, R134 30+ sub-agent 全部是 0 改 src 调研 / 路线图 / 实施 spec 阶段
- **R13**: Tauri Stage 5+ 跟 TUI 升级冲突 (per 用户记忆 #9 TUI 升级节奏 + 主人 8/4 23:55 "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了") — **缓解**: TUI 暂告段落, Tauri 终极路线, 瘦客户端 HTTP to apeireth-api, Tauri 来了无缝换 UI 层
- **R14**: ASI Stage 9 跟 S-1 服务 ASI 北极星冲突 (per 决策 #74 §1 B5 严守) — **缓解**: Stage 9 终极自治 1:1 服务 ASI 北极星, 0 改 S-1

### 6.2 决策原则 (per 决策 #73 §8.2 + 决策 #74 §7.2 + 决策 #33 §2.3 + 用户记忆 #10 + 主人 8/11 01:14 拍板 3 件套)

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑)
- **中断接手** (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- **编译产物清理决策矩阵** (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- **计划内任务完成自动接续 4 步 + 永久循环** (per 主人 0:57: 调研 + 差距 + 计划 + 实施 → 永久)
- **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
- **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, 写新文档 `docs/conventions/15-no-fear-complexity.md`)
- **8 硬墙 严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板):
  - **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改
  - **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.1.0 (per 决策 #22 §2.2, R132-1 提议) 或 1.2.1 (per 决策 #74 §1 B2 改写原话, Mavis 自决)
  - **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标) + V1.1 release 可改 (前提: 新的 baseline 更高)
  - **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
  - **B3 V0.5 30 维**: 严守 (哲学)
  - **B4 6 重守门 v7**: 严守 (哲学)
  - **B5 8 哲学锚**: 严守 (哲学)
  - **C1 0 主动 commit (主人起床前)**: 严守
  - **C2 0 装 PASS 严守**: 严守
  - **0 主动 push (主人起床前)**: 严守
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2, V1.1 借鉴源 12 源 0 装)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **不要漂移**: 8 哲学锚严守 (per 决策 #33 §2.3 B5) + 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1) + V0.5 30 维严守 + 6 重守门 v7 严守 + 0 装 PASS 严守 + 0 主动 commit/push 严守 + 整合 #4 commit abf12243 严守 + 决策日志写

### 6.3 V1.1 release 实施哲学 (per 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §3 + 15-no-fear-complexity.md)

- **最强效果 > 最简单代码** (per 15-no-fear-complexity.md §1.1): V1.1 release 6 大方向 0 为简化而简化
- **最厉害工程 > 最易维护** (per 15-no-fear-complexity.md §1.2): V1.1 release 0 为易维护而牺牲工程化
- **维护交给未来高水平团队** (per 15-no-fear-complexity.md §1.3): V1.1 release 0 排斥高水平团队
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 15-no-fear-complexity.md §2)
- **8 硬墙 (底线) + 不要怕复杂度 (上限) = 完整边界** (per 15-no-fear-complexity.md §3)

---

## 7. 一句话 (再次强调)

**V1.1 release 路线图 final 版 (per 决策 #74 B1 + 决策 #75 §2.1 + 决策 #73 拍板 3 件套 + 主人 8/11 01:14 拍板 + 不要怕复杂度哲学)**: 1.0 release (~8/11) 后 ~3.5 个月 minor release (`v1.1.0`, 估 2026-11-30), **6 大方向 final 版**: ①**PHL-07 实施** (V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests, R129-11 关键诚实标落地) ②**24 LOCKED 入口签名改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. ASI Stage 9 + 9 organ + 三洋葱, 0 改原 24 LOCKED 入口签名顺序) ③**后端加固** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml `1.2.0 → 1.1.0` minor bump (R132-1 提议) + pybridge 886/886 性能测试 + Cargo.lock 分模块) ④**Tauri Stage 5+** (9 organ 拟人化深化 + 5 nav 完整 + Tauri 2.0 完整集成 + 跨平台部署 Windows/macOS/Linux + Tauri 性能优化) ⑤**ASI Stage 8+** (Stage 9 终极自治 + 长程 AI 成长平台 + OpenCog AGPL-3.0 fork 决策 + pybridge 集成优化 + ASI Stage 9 集成测试) ⑥**形式化 Stage 5.5+** (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化). **V1.1 release 时间窗口**: 短期 1.0 release 实战完 + 1 周 V1.1 release 路线图拍板 → V1.1 release 实施 6 方向 6 周 (per 方向 1 周 R134-N sub-agent) → 估 2026-11-30 V1.1 release tag 打上. **16 跑中上限持续**: 30+ sub-agent 实施 (5-10 per 方向 × 6 方向 = 30+, per 决策 #71 §5 + 决策 #75 §2.1, 2 批 15+15 派满 16 上限). **永久循环**: V1.1 release → V1.2 minor → V2.0 major (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构). **决策原则**: 8 硬墙 0 越界 (B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 `1.2.0` V1.0 release 严守 + V1.1 release bump `1.1.0` (R132-1 提议) / A1 R11 baseline 3 值 严守 / A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 / B3 V0.5 30 维严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 主动 push 严守) + 0 装 PASS 严守 (V1.1 借鉴源 12 源: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源) + 0 主动 IM 主人 (per gate-discipline) + 0 主动 commit/push (per 决策 #33 §2.3) + 0 主动改 src (per 决策 #33 §2.3 + 决策 #74 B1 V1.0 release 0 改严守).
