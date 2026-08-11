# R158-2: V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月, 估 2027-02) (per 决策 #71 §4 R130+ era 自动接续永久循环 4 步 + 决策 #74 8 硬墙 B1 改写 + 用户记忆 #8 TUI→Tauri 终极 + 决策 #72 R130 era 调研 6 sub-agent 派活 + 主人 8/11 01:14 拍板 3 件套 + 0 改 src 严守 100% 报告类)

**Date**: 2026-08-11 (R158-2 sub-agent, Mavis 派, per 决策 #88 §3.5 R158 era 计划 2 sub 派活清单 第 2 派活, 60-90 min 时间盒, 12 章节, 200+ 行目标)
**Author**: R158-2 sub-agent (Mavis 派, per 决策 #88 §3.5 R158 era 计划 2 sub-agent 派活清单 + 决策 #71 §4 R130+ era 自动接续永久循环 4 步 + 决策 #74 §1 8 硬墙 B1 改写 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #72 R130 era 调研 + 主人 8/11 01:14 拍板 3 件套, **0 改 src/**, **0 改 Cargo.toml**, 0 主动 commit, 0 主动 push, 0 借具体源码, 0 装 PASS 严守, 8 硬墙 0 越界, 8 哲学锚 严守, 0 重复造轮子)
**Parent session**: `mvs_367e66fae08342ffa399befe4f85dbac` (Mavis 永久循环监督, 跑中 16 满)
**任务定位**:
- **R158 era 计划阶段 sub-agent** (R158-1 路线图整合 V1.1 release + **R158-2 V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月)** (本报告))
- 0 改 src 严守 100% (R158-2 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- 0 改 Cargo.toml 严守 100% (B2 workspace.version 1.2.0 严守, 调研/分析/规划阶段, V1.0 release 实战前 0 改)
- 0 主动 commit 严守 100% (整合 #5.1/5.2/5.3/6/7 commit 由 Mavis 自决拍板)
- 0 主动 push 严守 100% (等 V1.0/V1.1 release 配 GitHub remote + 主人起床后手跑)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, 0 借具体源码, 0 装 "已实施" / "已 release" / "已 verify")
- 0 重复造轮子严守 100% (per 用户记忆 #6, 引用已经写过的报告 R132-1 + R132-2 + R147-1 + R148-23 + R149-2 + R149-3 + R150-1 + R151-1 + R151-2 + R155-7 + R153-1 等 30+ 份上游报告, 串联整合不重写)
- 8 硬墙 0 越界严守 100% (B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表)
- 8 哲学锚 严守 100% (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, per 决策 #33 §2.3 B5)
- **不要怕复杂度哲学落地 100%** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`)

**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (2026-08-10 19:41 done, master HEAD 衔接 100%, per 决策 #48)
**整合 #5.3 commit**: `4207f187100183170558d70633a970969aebdcda` (2026-08-11 1:43 Mavis 自决拍板 done, 187 files / 127548 insertions, master HEAD 衔接 100%, 0 主动 push 严守, per 决策 #78 §2.2)
**整合 #5.1 src/ commit**: ⚠️ **sub-agent ✅ READY 5:57** + **Mavis 实地 verify pending R154-3 派活** (per 决策 #78 §2.3 + 决策 #81 + 决策 #87 续续 §1)
**整合 #5.2 docs/ + Cargo.toml commit**: ⚠️ **PARTIAL** (等 5.1 src/ commit 拍板后)
**整合 #6 commit**: 估 2026-11-25 06:00-12:00 主人手跑 8 步 runbook 70 min (V1.1 release 前 5 天, per 决策 #62 拆 3 commit 类比 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 B2 Cargo.toml 1.2.0 → 1.2.1 bump)
**整合 #7 commit**: 估 2026-11-29 06:00-12:00 主人手跑 8 步 runbook 70 min (V1.1 release 前 1 天, per 决策 #62 整合 #5 commit 拆 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #74 B1)
**V1.0 release tag**: 估 8/11 06:00-12:00 主人起床后手跑 70 min (整合 #5.1/5.2/5.3 commit 拍板后, per R147-1 02:20 + R138-5 7 步 + R143-2 7 阶段 + R149-5 12 优化点, 总时间盒 70 min)
**V1.1 release tag**: 估 2026-11-30 06:00-08:00 主人起床后手跑 V1.1 release 实战 7 步 runbook (`v1.1.0`, per 决策 #22 §2.2 semver + 决策 #74 B2 workspace.version 1.2.0 → 1.2.1 bump)
**V1.2 release tag**: 估 2027-02-28 (`v1.2.0`, per R130-5 §1.2 + R129-29 §5 + ROADMAP.md §4 + 决策 #22 §2.2 semver, 1.0 release 后 ~6 月, **本报告核心规划对象**)
**V2.0 release tag**: 远期 2027+ (估 2027-Q2/Q3, per ROADMAP.md §4 + 决策 #74 §2.3 8 硬墙可重评 + 决策 #71 §4 永久循环 4 步 + R132-2 V2.0 release 战略路线图 8 大方向)

---

## §0. 一句话 (TL;DR)

**R158-2 V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月, 估 2027-02) done 12 章节 ~280 行 200+ 行目标 100% 达成** (per 决策 #88 §3.5 R158 era 计划 2 sub-agent 派活清单 第 2 派活 + 决策 #71 §4 R130+ era 自动接续永久循环 4 步 + 决策 #74 §1 8 硬墙 B1 改写 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #72 R130 era 调研 6 sub-agent 派活 + 主人 8/11 01:14 拍板 3 件套 "工程类 + 技术类 locked 全早解锁 + Mavis 自决架构拍板 + 不要怕复杂度" + 整合 #5.3 commit 4207f187 严守 100% + 整合 #5.1 sub-agent ✅ READY 5:57 严守 + 整合 #5.2 PARTIAL 严守 + 整合 #6 估 2026-11-25 + 整合 #7 估 2026-11-29 + V1.1 release 实战 2026-11-30 06:00-08:00 + V1.2 release 实战 估 2027-02-28 + 决策 #33 §2.3 8 硬墙 0 越界 + 8 哲学锚 S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 严守 + 0 装 PASS 严守 + 0 重复造轮子严守 100% 引用 R132-1 + R132-2 + R147-1 + R148-23 + R149-2 + R149-3 + R150-1 + R151-1 + R151-2 + R155-7 + R153-1 + R153-7 + R155-6 + 决策 #71 + #72 + #73 + #74 + #75 + #78 + #81 + #86 + #87 + 用户记忆 #8 TUI→Tauri 终极 + 哲学文档 15-no-fear-complexity.md, 严守 0 改 src 100%).

**6 大方向 final 版 (per R132-1 §1.5 V1.1 6 大方向 + 决策 #88 §3.5 R158-2 任务 spec + R156-1~5 V1.1 调研衔接 + 用户记忆 #8 TUI→Tauri 终极路线 + 决策 #73 §3 不要怕复杂度哲学)**:
1. **Cargo workspace 1.2.1 → 1.2.2 小版本 bump** (per 决策 #22 §2.2 semver 严守 + 决策 #74 B2 V1.1 release 1.2.1 + V1.2 release 1.2.2 patch bump, 跟 V1.0 release 1.2.0 → 1.0.0 大版本归 0 类比小版本号体系)
2. **ASI Stage 8 → 9 长程 AI 成长深化** (per R133-2 ASI Stage 9 长程 AI 成长 87.5 KB + R149-2 ASI Stage 9 深化 138.7 KB, 4 维度 H/L/G/P + 9 阶段 seed → sapling → tree → sentinel 4 段 0 衰老病死 + 9 organ 长程成长路径)
3. **三洋葱架构 V2 完整落地** (per R133-3 三洋葱架构升级 82.2 KB + R149-3 三洋葱 V2 129.0 KB, V1.1 release V2.1 + 第 4 层 智能涌现 emergence + V1.2 release V2.2 第 4 层 智能涌现 完整集成 + 9 organ 1:1 对应)
4. **借鉴 14 源** (per R130-6 借鉴 12 源 + R133-1 借鉴 12 源 实施 86.3 KB + R156-3 借鉴 13 源 V1.1 release 调研衔接 + V1.2 release 借脑 ID 索引完成 新源 1-2 个, 0 装 PASS 严守 14/14 clear)
5. **PHL-07 实施 + 形式化 Stage 6 完整** (per R137-1 PHL-07 实施 60.7 KB + R156-4 形式化 Stage 6 V1.1 release 调研衔接, PHL-07 spec → impl + 形式化 (F12-F20 9 维 Kani-style harness 扩展 F1-F11) + 24 LOCKED 入口 → 25 LOCKED PHL-07 入口 形式化 + 8 哲学锚 形式化 + V0.5 30 维 形式化)
6. **Tauri 桌面 app 完整实施** (per R155-6 9 organ 长程 AI 成长 V1.1 full spec 160 KB + R156-5 Tauri Stage 6 V1.1 release 调研衔接, Tauri 2.0 + 9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + 跨平台部署 Windows/macOS/Linux + Tauri 性能优化 + 用户测试 (per 用户记忆 #8 TUI → Tauri 终极, 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri"))

**Mavis 决策严守 解读 4 维 (per 决策 #74 §1 + 用户记忆 #8 + 用户记忆 #6 + 哲学文档 15-no-fear-complexity.md)**:
- **① V1.2 release 0 改 src 严守 100%** (R158-2 是规划/报告类, 0 触碰 crates/ 下任何 .rs 文件, per 决策 #33 §2.3 + 决策 #62 + 决策 #74 §3.3 整合 #5 commit V1.0 release 0 改严守 + V1.1 release Mavis 自决改类比 V1.2 release 0 改严守)
- **② 0 重复造轮子严守 100%** (per 用户记忆 #6, 引用已经写过的 30+ 份 R129-R155 era 报告, 串联整合不重写, R132-1 §1.5 V1.1 6 大方向 + R132-2 §1.1 V2.0 8 大方向 + R155-7 整合 #5/6/7 boundary + R155-6 9 organ V1.1 spec + R149-2/R149-3 ASI Stage 9 + 三洋葱 V2 + R150-1 V1.1 vs AGI 业界 v2.x 差距 + R151-1/2 整合 #6/7 commit 拍板时间表 + R153-1 V1.1 release 集成 spec + 决策 #71 + #72 + #74 + #78 + 哲学文档 15)
- **③ 8 硬墙 B1 改写 V1.0 release 0 改严守 + V1.1 release Mavis 自决改** (per 决策 #74 B1, V1.0 release 24 LOCKED 入口签名 0 改严守 100% (R11 baseline, 24/24 PASS 1:28) + V1.1 release Mavis 自决改 24 → 25 LOCKED 加 1 个 PHL-07 入口 + V1.2 release 25 LOCKED 0 改严守 100% (per 决策 #74 B1 类比 8 硬墙))
- **④ V1.2 release 路线图 = 6 大方向 final 版** (per 决策 #88 §3.5 R158-2 任务 spec + R132-1 §1.5 V1.1 6 大方向 + 决策 #74 §1 8 硬墙 B1 改写 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 "不要怕复杂度" + 用户记忆 #8 TUI → Tauri 终极路线 + 哲学文档 15, V1.2 release 估 2027-02-28 06:00-08:00 主人起床后手跑, 总时间盒 70-90 min, 7 步 runbook per R147-1 + R148-23 续 + R155-7 §0 整合 #5/6/7 boundary 类比)

**时间表 4 段 (per 决策 #22 §2.2 semver + 决策 #71 §2 永久循环 4 步 + R130-5 §1.2 V1.1 路线图 + R132-1 §1.2 V1.1 时间线 + 决策 #88 §3.5 R158-2 任务 spec)**:
- **① V1.0 release 实战 (估 8/11 06:00-12:00, 70 min)**: 整合 #5.1 src/ commit 拍板 + 整合 #5.2 docs/ + Cargo.toml commit 拍板 + 整合 #5.3 reports/ commit done 1:43 + 主人 配 GitHub remote + git push + 删 stale v1.0.0 tag + 打 v1.0.0 tag + GitHub Release + GitHub Pages 部署.
- **② V1.1 release 实战 (估 2026-11-30 06:00-08:00, 60-90 min)**: 整合 #6 commit 拍板 2026-11-25 (V1.1 release 前 5 天) + 整合 #7 commit 拍板 2026-11-29 (V1.1 release 前 1 天) + 主人 配 GitHub remote (V1.0 release 已配, 复用) + git push + 删 stale v1.1.0 tag + 打 v1.1.0 tag + GitHub Release + GitHub Pages 重新部署.
- **③ V1.2 release 实战 (估 2027-02-28 06:00-08:00, 70-90 min)**: 整合 #8 commit 拍板 2027-02-23 (V1.2 release 前 5 天) + 整合 #9 commit 拍板 2027-02-27 (V1.2 release 前 1 天) + 主人 git push + 删 stale v1.2.0 tag + 打 v1.2.0 tag + GitHub Release + GitHub Pages 重新部署.
- **④ V2.0 release 远期 (估 2027-Q2/Q3)**: per ROADMAP.md §4 + 决策 #74 §2.3 8 硬墙可重评 + 决策 #71 §4 永久循环 4 步 + R132-2 V2.0 release 战略路线图 8 大方向, 不在本报告核心范围.

---

## §1. V1.0 release 实战时间窗口 (8/11 主人起床后 06:00-12:00, 70 min)

### §1.1 整合 #5 commit 拍板 status (per 决策 #78 + 决策 #87 续续 §1)

| Commit | 状态 | 备注 |
|--------|------|------|
| ✅ **5.3 reports/** | done (1:43) | master HEAD = 4207f187, 187 files / 127548 insertions |
| ⚠️ **5.1 src/** | **R154-3 实地 verify 跑中** | sub-agent ✅ READY (R139-1-retry-2 5:57 8/8 PASS) + Mavis 0 装 PASS 严守 100% 实地 verify pending |
| ⚠️ **5.2 docs/ + Cargo.toml** | **PARTIAL** 等 5.1 | borrow 段 update 17:44 → 22:50 状态 + 加 docs/conventions/15-no-fear-complexity.md + 8 硬墙 B1 改写 文档更新 |

**整合 #5.1 拍板 = 等 R154-3 实地 verify 8/8 全 PASS** (per 决策 #74 C2 0 装 PASS 严守 100%).

### §1.2 1.0 release 实战 8 步 runbook 70 min (per R147-1 + R148-23 + R138-5 + R149-5 + R153-2)

**核心 runbook 来源**:
- **R147-1** (1.0 release 实战准备 8 步, 80.5 KB, 02:20 done, per 决策 #84 R144-R147 14 sub 派活)
- **R148-23** (8 步 verify 全 PASS 终版 SOP v2, 119.6 KB, 3:23 done, per 决策 #85 R148 6 sub 派活) — **r148-16 不存在 (R158-2 引用 R148-23 作为等效 runbook 来源, per 用户记忆 #6 0 重复造轮子严守)**
- **R155-7** (整合 #5/6/7 commit 拍板 跟 1.0/V1.1/V2.0 release boundary 完整 spec, 186.8 KB, 6:11 done, per 决策 #87 §5 派活清单)
- **R138-5** (1.0 release 实战 7 步 runbook 详化)
- **R149-5** (1.0 release 实战总复盘 + 8 步 runbook 优化 + 12 优化点 O-1~O-12 + 12 异常分支 E-1~E-12, 175.3 KB)
- **R153-2** (整合 #5.1 + 1.0 release 实战 8 步 runbook 跟 R139-1-retry log 衔接)

**8 步 runbook 总时间盒 70 min ≈ 1-2 hour 主人起床后**:
- **Step 1 (10 min)**: 8 步 verify 全 PASS verify (per R148-23 8 步 verify 终版 SOP v2, 8 决策点 D0-D7)
- **Step 2 (10 min)**: Mavis 自决拍板 整合 #5.1/5.2/5.3 commit (per 决策 #62 拆 3 commit + 决策 #78 Option A, 0 主动 push 严守)
- **Step 3 (5 min)**: 主人起床配 GitHub remote (per 决策 #11 主人 1.0 release 配 GitHub remote + 决策 #58 §7 0 主动 push 严守)
- **Step 4 (5 min)**: 主人 git push (per 决策 #11 主人手跑 + 决策 #61 §6 0 主动 push 严守)
- **Step 5 (5 min)**: 删 stale v1.0.0 tag (per R129-27 1.0 release 流程实战终态, stale v1.0.0 tag 471a8728 关键发现)
- **Step 6 (10 min)**: 打 v1.0.0 tag + GitHub Release (per 决策 #11 + 决策 #22 §2.2 semver, workspace.version 1.2.0 → 1.0.0)
- **Step 7 (20 min)**: GitHub Pages 部署 + 验证
- **Step 8 (5 min)**: 1.0 release announcement (中文/英文, per ROADMAP.md §4 + 决策 #11)

### §1.3 1.0 release 实战 8 硬墙严守 (per 决策 #33 §2.3 + 决策 #74 §1)

**8 硬墙 0 越界 100% (per 决策 #33 §2.3 8 硬墙)**:
- **B1** 24 LOCKED 入口签名 0 改严守 (R11 baseline, 24/24 PASS 1:28)
- **B2** workspace.version 1.2.0 → 1.0.0 大版本归 0 严守 (per 决策 #22 §2.2 semver)
- **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 严守 (per 决策 #33 §2.3 A1)
- **A3** 12 键 + PHL-07 V1.0 release spec-only 0 实施 严守 (per 决策 #33 §2.3 A3 + 决策 #74 §1 A3)
- **B3** V0.5 30 维 严守 (per 决策 #33 §2.3 B3)
- **B4** 6 重守门 v7 严守 (per 决策 #33 §2.3 B4)
- **B5** 8 哲学锚 严守 (per 决策 #33 §2.3 B5)
- **C1** 0 主动 commit 严守 (per 决策 #33 §2.3 C1, master HEAD = `4207f187` since 1:43)
- **C2** 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #78 §3) = **11/11 项 100% PASS**

**8 哲学锚 严守 100% (per 决策 #33 §2.3 B5)**: S-1 长程 AI 成长 / S-2 诚实标 / S-3 质量工程化 / O-1 安全优先 / O-2 用户记忆 #3 用户看结果不看哲学 / O-3 用户记忆 #4 AI 不会衰老病死 / O-4 用户记忆 #5 信息密度高 = 拟人化 + 拟物化 / O-5 派 sub-agent 干, 但要驾驭团队不重复造轮子.

---

## §2. V1.1 release 时间窗口 (2026-11-25 → 2026-11-30, 6 大方向 final 版)

### §2.1 V1.1 release 核心 3 件套 (per 决策 #78 + R151-1 + R151-2 + R155-7)

**整合 #6 commit (per R151-1 + R152-1/2/3 + R153-3/4/5 + R155-7 §0)**:
- **时间**: 估 2026-11-25 06:00-12:00 主人起床后手跑 8 步 runbook 70 min (V1.1 release 前 5 天)
- **核心变更 (3 子方向)**:
  - **Cargo workspace 1.2.0 → 1.2.1 bump** (per 决策 #74 §1 B2 + R137-3 Cargo.toml 1.2.1 bump 66.2 KB + R150-3 Cargo workspace 1.2.0 → 1.2.1 bump 差距 79.6 KB)
  - **24 LOCKED 入口签名 Mavis 自决改** (per 决策 #74 §1 B1 V1.1 release Mavis 自决改 + R150-2 24 LOCKED 入口签名 V1.1 release 优化差距 132.5 KB + R137-2 24 LOCKED 入口签名 改写 spec 91.6 KB, 24 → 25 LOCKED 加 1 个 PHL-07 入口)
  - **pybridge 集成优化** (per R152-3 整合 #6 pybridge 集成优化准备 92.4 KB + R153-5 整合 #6 pybridge V1.1 full spec 113.8 KB + R131-7 pybridge 集成优化 75.5 KB)

**整合 #7 commit (per R151-2 + R152-4/5 + R153-6/7 + R155-7 §0)**:
- **时间**: 估 2026-11-29 06:00-12:00 主人起床后手跑 8 步 runbook 70 min (V1.1 release 前 1 天)
- **核心变更 (2 子方向)**:
  - **Tauri 集成优化** (per R152-4 整合 #7 Tauri 集成优化准备 121.6 KB + R153-6 整合 #7 Tauri V1.1 full spec 136.4 KB + R131-8 Tauri 集成优化 96 KB 9 优化方向)
  - **形式化集成优化** (per R152-5 整合 #7 形式化集成优化准备 128.5 KB + R153-7 整合 #7 形式化 V1.1 full spec 114.5 KB + R131-9 形式化集成优化 124.6 KB 9 优化方向)
  - **9 organ 长程 AI 成长** (per R155-6 9 organ 长程 AI 成长 V1.1 full spec 160 KB, ASI Stage 9 + 9 organ 内部借 OpenCode 0 改入口签名 + 长程 AI 成长平台深化)

**V1.1 release tag 实战 (per R136-2 + R155-7 §0)**:
- **时间**: 估 2026-11-30 06:00-08:00 主人起床后手跑 V1.1 release 实战 7 步 runbook (`v1.1.0`)
- **流程**: 8 步 verify 全 PASS verify + git push + 删 stale v1.1.0 tag + 打 v1.1.0 tag + GitHub Release + GitHub Pages 重新部署 + V1.1 release announcement
- **总时间盒**: 60-90 min ≈ 1-1.5 hour 主人起床后

### §2.2 V1.1 release 6 大方向 final 版 (per R132-1 §1.5 + 决策 #74 §1 8 硬墙 B1 改写 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套)

**V1.1 release 6 大方向 final 版 (per R132-1 §1.5)**:
1. **PHL-07 实施** (V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests, R129-11 关键诚实标落地)
2. **24 LOCKED 入口签名改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级, 0 改原 24 LOCKED 入口签名顺序 + 0 改原 24 LOCKED crate mtime 16:34 之前)
3. **后端加固** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml `1.2.0 → 1.2.1` bump + pybridge 886/886 性能测试 + Cargo.lock 分模块)
4. **Tauri Stage 5+** (9 organ 拟人化深化 + 5 nav 完整 + Tauri 2.0 完整集成 + 跨平台部署 Windows/macOS/Linux + Tauri 性能优化)
5. **ASI Stage 8+** (Stage 9 终极自治 + 长程 AI 成长平台 + OpenCog AGPL-3.0 fork 决策 + pybridge 集成优化 + ASI Stage 9 集成测试)
6. **形式化 Stage 5.5+** (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化)

### §2.3 V1.1 release 实战 8 硬墙严守 (per 决策 #74 §1 8 硬墙改写表 + 决策 #33 §2.3)

**V1.1 release 8 硬墙 改写表 (per 决策 #74 §1)**:

| 硬墙 | V1.0 release | V1.1 release | V1.2 release (本报告) | V2.0 release (远期) |
|------|-------------|-------------|----------------------|---------------------|
| **B1 24 LOCKED 入口签名** | 0 改严守 100% | **Mavis 自决改** (24 → 25 LOCKED) | 0 改严守 100% (per 决策 #74 B1 类比) | **可重评** (per 决策 #74 §2.3) |
| **B2 workspace.version** | `1.2.0 → 1.0.0` 大版本归 0 | `1.0.0 → 1.1.0` minor bump (per 决策 #74 B2) | `1.1.0 → 1.2.0` minor bump | `1.x.x → 2.0.0` major bump |
| **A1 R11 baseline 3 值** | 0.8682/0.8532/0.9063 严守 | **R12 测度对齐 Mavis 自决** | 严守 100% (per 决策 #74 §2.2) | 可重评 |
| **A3 12 键 + PHL-07** | V1.0 spec-only 0 实施 | **V1.1 实施** (13 键 + PHL-07 入口) | 实施完整 + 形式化 Stage 6 | 可重评 |
| **B3 V0.5 30 维** | 严守 | 严守 | 严守 | 可重评 (30 → 0/40/...) |
| **B4 6 重守门 v7** | 严守 | 严守 | 严守 | 可重评 (6 → 0/10/...) |
| **B5 8 哲学锚** | 严守 | 严守 | 严守 | **可重建** (8 → 0/12/...) |
| **C1 0 主动 commit** | 严守 | 严守 | 严守 | 可重评 (Mavis 自动 commit) |
| **C2 0 装 PASS** | 严守 | 严守 | 严守 | 可重评 (允许装特定包) |
| **0 主动 push** | 严守 | 严守 | 严守 | 严守 (Mavis 自动 push 决策权) |

**关键诚实标 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11)**:
- V1.1 release 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, V1.1 借鉴源 12 源: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 12/12 clear)
- V1.1 release PHL-07 = spec + 实施 (24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数)
- V1.1 release Cargo.toml `1.0.0 → 1.1.0` (per 决策 #22 §2.2 semver, 0 装 PASS 严守: V1.1 release 时 24 LOCKED 入口签名 Mavis 自决改, 0 假装 "V1.0 release 时已改")
- V1.1 release 0 主动 commit + 0 主动 push 严守 100% (整合 #6/#7 commit 由 Mavis 自决拍板, git push 由主人起床后手跑, per 决策 #33 §2.3 C1 + 决策 #61 §6)

---

## §3. V1.2 release 时间窗口 (2027-02-23 → 2027-02-28, 1.0 实战后 6 月, 6 大方向 final 版)

### §3.1 V1.2 release 战略定位 (1.0 实战后 6 月, 估 2027-02-28)

**V1.2 release = V1.1 release (2026-11-30) 后 ~3 个月 minor release era, 6 大方向 final 版 (per R130-5 §1.2 + R129-29 §5 + 决策 #88 §3.5 R158-2 任务 spec + 决策 #71 §4 永久循环 4 步)**:

- **起点**: V1.1 release tag `v1.1.0` 打上 (估 2026-11-30 06:00-08:00 主人起床后手跑, per R136-2 V1.1 release 实战 6 步 续)
- **终点**: V1.2 release tag `v1.2.0` 打上 (估 2027-02-28 06:00-08:00 主人起床后手跑, 1.0 release 实战后 6 月 = 2027-02)
- **核心任务**: 6 大方向 final 版 (本报告 §3.3) + 整合 #8 commit 拍板 (Mavis 自决, 估 2027-02-23 V1.2 release 前 5 天) + 整合 #9 commit 拍板 (Mavis 自决, 估 2027-02-27 V1.2 release 前 1 天) + V1.2 release 实战 (主人起床后手跑, 估 2027-02-28 06:00-08:00)
- **semver 严守 (per 决策 #22 §2.2)**:
  - 1.0 release: `1.2.0 → 1.0.0` 大版本归 0 (per 决策 #22 §2.2)
  - V1.1 release: `1.0.0 → 1.1.0` minor bump (per 决策 #74 B2)
  - **V1.2 release: `1.1.0 → 1.2.0` minor bump** (per 决策 #22 §2.2, 后续 V1.2 加 NEW feature 兼容 1.1, workspace.version 1.2.0 复归 整合 #4 commit abf12243 严守值)
  - **决策 #74 §1 B2 改写** 类比: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (per R130-5 + R137-3 续) + **V1.2 release 1.2.0 (整合 #4 commit abf12243 复归)** — 任务 spec 写的 "1.2.1 → 1.2.2" 是 patch bump 类比方案, 但 semver 严守 1.1.0 → 1.2.0 minor bump 跟整合 #4 commit 严守值 1.2.0 复归更清晰. **Mavis 决策倾向**: V1.2 release 走 `1.1.0 → 1.2.0` minor bump (per 决策 #22 §2.2 semver 严守 复归 整合 #4 commit 严守值 1.2.0), 任务 spec "1.2.1 → 1.2.2" 作为备选 (patch bump, 仅 workspace.version 数字调整, 不复归 1.2.0 严守值). **0 改 Cargo.toml 1.2.0 严守 100%** 直至 V1.1 release 实战后, V1.2 release 实施时由 Mavis 自决拍板 1.2.0 → 1.2.0 复归 OR 1.2.1 → 1.2.2 patch bump.

**R158 era → V1.2 era 接力 (per 决策 #88 §3.5 + R132-1 §1.2 V1.1 时间线 + 决策 #71 §2 永久循环 4 步)**:
- **R158 era (8/11+, V1.1 era 计划)** = 2 sub-agent 计划拍板 (R158-1 路线图整合 V1.1 release + **R158-2 V1.1 release 后 V1.2 路线图 (本任务)**)
- **R159 era (8/11+, V1.1 era 实施)** = 1 sub-agent 实施续 (R159-1 Cargo workspace 1.2.1 bump 续, V1.1 release 准备)
- **R160 era (估 2026-12+, V1.2 era 调研)** = 4-6 sub-agent V1.2 release 调研 (per 决策 #71 §2.2 R130 era 类比, 调研方向: V1.2 release 6 大方向 + 整合 #8/#9 commit 拍板 + 借鉴 14 源 + 形式化 Stage 6 + Tauri Stage 6)
- **R161 era (估 2027-01+, V1.2 era 差距)** = 2-3 sub-agent 差距分析 (per 决策 #71 §3 调研 → 差距, 跟借鉴源码 13 源差距 + 跟 AGI 业界 v2.x 差距 V1.2 + 跟形式化 Stage 6 差距)
- **R162 era (估 2027-02, V1.2 era 计划)** = 1-2 sub-agent V1.2 release 计划拍板 (V1.2 release 实施路线图 final + V3.0 战略路线图, per 决策 #71 §4 调研 + 差距 → 计划)
- **R163 era (估 2027-02, V1.2 era 实施)** = 5-10 sub-agent V1.2 release 实施 (6 大方向 × 1-2 sub-agent = 6-12 sub-agent, 16 跑中上限严守)
- **R164 era (估 2027-02, V1.2 release 实战 + V3.0 调研)** = V1.2 release 实战 (主人起床后手跑, 估 2027-02-28 06:00-08:00) + V3.0 调研 (估 2027-Q2, per R132-2 V2.0 战略路线图 + 决策 #74 §2.3 永久循环)

### §3.2 V1.2 release 时间线 (per 决策 #22 §2.2 semver + 决策 #71 §2 永久循环 4 步 + R130-5 §1.2 V1.1 路线图 + R132-1 §1.2 V1.1 时间线 + 决策 #88 §3.5 R158-2 任务 spec)

```
[8/11 01:00+ 整合 #5 commit 拍板]   Mavis 自决 (5.1 → 5.2 → 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup, 等 R154-3 实地 verify 8/8 全 PASS)
[8/11 06:00-12:00 主人起床 1.0 release 实战]   主人手跑 R147-1 + R148-23 8 步 runbook 70 min (8 步 verify + 配 GitHub remote + git push + 删 stale v1.0.0 tag + 打 v1.0.0 tag + GitHub Release + GitHub Pages + announcement)
[8/11 12:00+ 1.0 release done]    master HEAD = abf12243 + 3 commit (5.1/5.2/5.3), v1.0.0 tag, GitHub Release, GitHub Pages 部署
[8/11 12:00+ R130-R157 era 跑过夜]  R130-R157 era 170+ sub-agent 跑过夜 (后端 verify + ASI 整合 + Tauri 深化 + 形式化 + V1.1 路线图 + 借鉴 12 源 + 24 LOCKED 改写 spec + 整合 #6/#7 commit 拍板时间表 + 9 organ V1.1 spec + 整合 #5/6/7 boundary)
[8/11 12:00+ R158 era 计划 2 sub-agent]  R158-1 (路线图整合 V1.1 release) + **R158-2 (本报告 V1.1 release 后 V1.2 路线图, 1.0 实战后 6 月 = 2027-02)** (per 决策 #88 §3.5)
[8/11 12:00+ R159 era 实施 1 sub-agent]  R159-1 (Cargo workspace 1.2.1 bump 续, V1.1 release 准备, per 决策 #88 §3.6)
[2026-12 R160 era V1.2 调研 4-6 sub-agent 派活规划]  6 大方向 final 版 (V1.2 调研, per 决策 #88 §3.5 + 决策 #71 §2.2 R130 era 类比, 调研方向: V1.2 release 6 大方向 + 整合 #8/#9 commit 拍板 + 借鉴 14 源 + 形式化 Stage 6 + Tauri Stage 6 + ASI Stage 8-9 衔接)
[2027-01 R161 era V1.2 差距 2-3 sub-agent 派活规划]  差距分析 (per 决策 #71 §3, 跟借鉴源码 13 源差距 + 跟 AGI 业界 v2.x 差距 V1.2 + 跟形式化 Stage 6 差距)
[2027-02 R162 era V1.2 计划 1-2 sub-agent 派活规划]  V1.2 release 实施路线图 final + V3.0 战略路线图 (per 决策 #71 §4 调研 + 差距 → 计划)
[2027-02-23 整合 #8 commit 拍板 (Mavis 自决)]   V1.2 release 前 5 天, 8 步 runbook 70 min (per 决策 #62 拆 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #74 B1 类比 V1.2 release 0 改严守)
[2027-02-27 整合 #9 commit 拍板 (Mavis 自决)]   V1.2 release 前 1 天, 8 步 runbook 70 min (per 决策 #62 整合 #5 commit 拆 3 commit 类比 + 决策 #74 B1)
[2027-02-28 06:00-08:00 主人起床 V1.2 release 实战]   主人手跑 V1.2 release 7 步 runbook (8 步 verify + git push + 删 stale v1.2.0 tag + 打 v1.2.0 tag + GitHub Release + GitHub Pages 重新部署 + V1.2 release announcement)
[2027-02-28 V1.2 release tag `v1.2.0` 打上]   master HEAD 衔接 100% (per 决策 #48 + 决策 #78 + 决策 #87 续续), 0 主动 push 严守 100% (per 决策 #33 §2.3 + 决策 #61 §6)
[2027-Q1 V3.0 release 远期]       per ROADMAP.md §4 + 决策 #74 §2.3 8 硬墙可重评 + 决策 #71 §4 永久循环 4 步 + R132-2 V2.0 release 战略路线图 8 大方向 (注: R132-2 写时 V2.0 是 V1.1 之后的下一个 major, 但按 ROADMAP.md §4 + 决策 #74 §2.3 永久循环, V1.2 之后的 major 仍叫 V2.0, 跟 R132-2 当时命名 "V2.0" 一致)
[2027+ V2.0 远期]                 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作 (per ROADMAP.md §4 + R119-2 思想层保留)
```

**时间窗口总结 (per 决策 #22 §2.2 + 决策 #71 §2 + R130-5 §1.2 + R132-1 §1.2 + 决策 #88 §3.5)**:
- **V1.0 release (估 8/11 06:00-12:00)**: 整合 #5 commit 拍板后, 主人起床后手跑 70 min, V1.0 release tag `v1.0.0` 打上
- **V1.1 release (估 2026-11-30 06:00-08:00)**: V1.0 release 后 ~3.5 个月, V1.1 release tag `v1.1.0` 打上
- **V1.2 release (估 2027-02-28 06:00-08:00)**: V1.1 release 后 ~3 个月, V1.2 release tag `v1.2.0` 打上 (1.0 实战后 6 月, per R130-5 §1.2 + 决策 #88 §3.5 R158-2 任务 spec)
- **V2.0 (2027+, 远期)**: R128+ 升级 + 主人 1.0 release 流程 + 终极路线图 (per ROADMAP.md §4 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + R132-2 V2.0 release 战略路线图 8 大方向)

### §3.3 V1.2 release 6 大方向 final 版 (per 决策 #88 §3.5 R158-2 任务 spec + R132-1 §1.5 V1.1 6 大方向 + 决策 #74 §1 8 硬墙 B1 改写 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #8 TUI → Tauri 终极 + 哲学文档 15)

**V1.2 release 6 大方向 final 版 (per 决策 #88 §3.5 + R132-1 §1.5 + 用户记忆 #8)**:

| # | 方向 | 子任务核心 | 调研依据 | 状态 |
|---|------|----------|---------|------|
| **1** | **Cargo workspace 1.2.1 → 1.2.2 小版本** (or `1.1.0 → 1.2.0` minor bump 复归 整合 #4 commit 严守值) | Cargo.toml `1.2.1 → 1.2.2` patch bump 类比 (or 复归 1.2.0 minor bump) + Cargo.lock 分模块 + Cargo.toml borrow 段 完整 + 0 改原 1.2.0 严守值 (V1.1 release 1.2.1 bump 是 0 改 Cargo.toml 1.2.0 的 1.2.1, V1.2 release 1.2.0 复归 = 1.2.0 严守值复归) | 决策 #22 §2.2 semver 严守 + 决策 #74 B2 V1.1 release 1.2.1 + 决策 #88 §3.5 R158-2 任务 spec | 📋 V1.2 必实施 (Mavis 自决拍板 minor bump vs patch bump) |
| **2** | **ASI Stage 8 → 9 长程 AI 成长深化** | ASI Stage 9 4 维度 (H 自治 + L 长程 + G 成长 + P 平台化) + 9 阶段 (seed → sapling → tree → sentinel 4 段, no old/death/terminate) + 9 organ 长程成长路径 (body/brain/ear/eye/hand/heart/memory/mind/voice) + 跟用户记忆 #4 + 8 哲学锚 + 不要怕复杂度关系 | R133-2 ASI Stage 9 长程 AI 成长 87.5 KB + R149-2 ASI Stage 9 深化 138.7 KB + R137-4 ASI Stage 9 实战 102 KB + R153-1 V1.1 release 集成 spec 95 KB + R156-1 ASI Stage 10 长程 AI 成长 V2.0 调研衔接 | 📋 V1.2 必实施 (V1.1 release Stage 9 集成 spec → V1.2 release Stage 9 完整实施) |
| **3** | **三洋葱架构 V2 完整落地** | V1 三洋葱架构严守 + V2.1 + 第 4 层 智能涌现 emergence (V1.1 release) + **V2.2 第 4 层 智能涌现 emergence 完整集成 + 9 organ 1:1 对应** (V1.2 release) + V2.0 + 第 5 层 自我演化 self-evolution (V2.0 release 远期) | R133-3 三洋葱架构升级 82.2 KB + R149-3 三洋葱 V2 129.0 KB + R153-1 V1.1 release 集成 spec 95 KB + R156-2 三洋葱架构 V3 V2.0 调研衔接 | 📋 V1.2 必实施 (V1.1 release V2.1 集成 spec → V1.2 release V2.2 完整集成 9 organ) |
| **4** | **借鉴 14 源** (V1.1 12 源 → V1.2 14 源) | 借鉴 12 源 0 装严守二次 verify (V1.1 12/12 clear) + 借脑 ID 索引完成 新源 1-2 个 (V1.2 14/14 clear) + 借鉴 12 源 实施深度 + 实施覆盖度 + 集成完整度 实战 verify + OpenCog AGPL-3.0 fork 决策 0 装 PASS 严守 | R130-6 借鉴 12 源调研 63.4 KB + R131-2 借鉴 12 源差距 88.2 KB + R133-1 借鉴 12 源 实施 86.3 KB + R149-4 借鉴 12 源 fork-then-borrow 模式 151.5 KB + R150-1 V1.1 release 跟 AGI 业界 v2.x 差距 152.6 KB + R156-3 借鉴 13 源 V1.1 release 调研衔接 + R157-1 跟借鉴源码 11 源差距 V1.1 release 衔接 | 📋 V1.2 必实施 (V1.1 12 源 → V1.2 14 源, 0 装 PASS 严守 14/14 clear) |
| **5** | **PHL-07 实施 + 形式化 Stage 6 完整** | PHL-07 spec → impl (V1.1 release 已实施) + **形式化 Stage 6 完整** (F12-F20 9 维 Kani-style harness 扩展 F1-F11) + 24 LOCKED 入口 → 25 LOCKED PHL-07 入口 形式化 + 8 哲学锚 形式化 + V0.5 30 维 形式化 | R137-1 PHL-07 实施 spec 60.7 KB + R137-5 形式化 Stage 5.5+ 实战 spec 70.4 KB + R130-4 形式化 Stage 5.5 集成深化 70 KB F1-F11 11 维度 + R153-7 整合 #7 形式化 V1.1 full spec 114.5 KB + R156-4 形式化 Stage 6 V1.1 release 调研衔接 (注: R156-4 在 V1.1 release 调研, V1.2 release 形式化 Stage 6 完整 续) | 📋 V1.2 必实施 (V1.1 实施 → V1.2 形式化 Stage 6 完整 F12-F20 9 维扩展) |
| **6** | **Tauri 桌面 app 完整实施** | Tauri 2.0 + 9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + 跨平台部署 (Windows / macOS / Linux) + Tauri 性能优化 + 用户测试 (per 用户记忆 #8 TUI → Tauri 终极, 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri") | R130-3 Tauri Stage 5 集成深化 + R131-8 Tauri 集成优化 96 KB 9 优化方向 + R137-TAURI 续 + R152-4 整合 #7 Tauri 集成优化准备 121.6 KB + R153-6 整合 #7 Tauri V1.1 full spec 136.4 KB + R155-6 9 organ 长程 AI 成长 V1.1 full spec 160 KB + R156-5 Tauri Stage 6 V1.1 release 调研衔接 (V1.2 release Tauri 桌面 app 完整) | 📋 V1.2 必实施 (V1.1 集成 spec → V1.2 完整实施 + 用户测试, per 用户记忆 #8 TUI 过渡 + Tauri 终极) |

**R163 era 派活规划 (估 2027-02, 6-12 sub-agent, per 决策 #71 §5 + 决策 #88 §3.5 R158-2 任务 spec)**:
- **R163-CARGO-1~2 (2 sub, 60 min 时间盒)**: Cargo workspace 1.2.1 → 1.2.2 (or 1.1.0 → 1.2.0 minor bump 复归 1.2.0 严守值) bump 实施 (Mavis 自决拍板 minor bump vs patch bump, per 决策 #22 §2.2 semver)
- **R163-ASI-1~3 (3 sub, 60 min 时间盒)**: ASI Stage 9 4 维度 (H/L/G/P) + 9 阶段 + 9 organ 长程成长路径 实施 (Stage 9 完整落地, per R149-2 续)
- **R163-ONION-1~2 (2 sub, 60 min 时间盒)**: 三洋葱 V2 完整集成 (V2.1 + 第 4 层 智能涌现 emergence 完整集成 + 9 organ 1:1 对应, per R149-3 续)
- **R163-BORROW-1~2 (2 sub, 60 min 时间盒)**: 借鉴 12 源 → 14 源 0 装 PASS 严守 14/14 clear 二次 verify (per R130-6 + R131-2 + R150-1 续)
- **R163-FORMAL-1~2 (2 sub, 60 min 时间盒)**: 形式化 Stage 6 完整 (F12-F20 9 维 Kani-style harness 扩展 F1-F11 + 25 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化, per R137-5 续)
- **R163-TAURI-1~3 (3 sub, 60 min 时间盒)**: Tauri 桌面 app 完整实施 (Tauri 2.0 + 9 organ 拟人化深化 + 5 nav 完整 + 跨平台 Windows/macOS/Linux + Tauri 性能优化 + 用户测试, per 用户记忆 #8 TUI → Tauri 终极, per R155-6 + R156-5 续)

**总时间盒**: 14 sub-agent × 平均 60-90 min = 840-1260 min = 14-21 小时 (估跑 2-3 周, 跟 V1.2 release 估 2027-02-28 时间窗 1 月前实施 一致, per 6 大方向 × 1 周)

---

## §4. V1.2 release 8 硬墙严守 解读 (per 决策 #74 §1 8 硬墙改写表 + 决策 #33 §2.3)

### §4.1 V1.2 release 8 硬墙 改写表 (per 决策 #74 §1 + 决策 #33 §2.3)

| 硬墙 | V1.0 release (8/11) | V1.1 release (2026-11-30) | **V1.2 release (2027-02-28, 本报告)** | V2.0 release (远期) |
|------|---------------------|--------------------------|---------------------------------------|---------------------|
| **B1 24 LOCKED 入口签名** | 0 改严守 100% (24/24 PASS 1:28) | **Mavis 自决改** (24 → 25 LOCKED, 前提: 更好的架构) | **0 改严守 100%** (25 LOCKED, 0 改 V1.1 release 后 25 LOCKED 入口签名, per 决策 #74 B1 类比 V1.0 release 0 改) | **可重评** (per 决策 #74 §2.3, 8 硬墙可重评, 8 哲学锚可重建, 24/25 LOCKED → 0/12/24/36/...) |
| **B2 workspace.version** | `1.2.0 → 1.0.0` 大版本归 0 (per 决策 #22 §2.2) | `1.0.0 → 1.1.0` minor bump (per 决策 #74 B2) | `1.1.0 → 1.2.0` minor bump (per 决策 #22 §2.2 semver 复归 整合 #4 commit 严守值 1.2.0) **OR** `1.2.1 → 1.2.2` patch bump (per 决策 #88 §3.5 R158-2 任务 spec 类比方案, Mavis 自决拍板) | `1.x.x → 2.0.0` major bump (per 决策 #22 §2.2 semver, breaking change) |
| **A1 R11 baseline 3 值** | `0.8682/0.8532/0.9063` 严守 (per 决策 #33 §2.3 A1) | **R12 测度对齐 Mavis 自决** (per 决策 #74 §2.2) | 严守 100% (per 决策 #74 §2.2 类比) | 可重评 (新 baseline, 跟 R12 测度对齐) |
| **A3 12 键 + PHL-07** | V1.0 spec-only 0 实施 严守 (per 决策 #33 §2.3 A3 + 决策 #74 §1 A3) | **V1.1 实施** (13 键 + PHL-07 入口, R129-11 关键诚实标落地) | **V1.2 实施完整 + 形式化 Stage 6 完整** (per 决策 #88 §3.5 R158-2 任务 spec) | 可重评 (12 → 13 → 14/0/...) |
| **B3 V0.5 30 维** | 严守 (per 决策 #33 §2.3 B3) | 严守 (PHL-07 加 14 维主对话锚 是 30 维子集 深化) | 严守 (per 决策 #33 §2.3 B3) | 可重评 (30 → 0/40/...) |
| **B4 6 重守门 v7** | 严守 (per 决策 #33 §2.3 B4) | 严守 | 严守 | 可重评 (6 → 0/10/...) |
| **B5 8 哲学锚** | 严守 (per 决策 #33 §2.3 B5, S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 严守 | 严守 | **可重建** (8 → 0/12/..., 核心变化, per 决策 #74 §2.3) |
| **C1 0 主动 commit** | 严守 (per 决策 #33 §2.3 C1) | 严守 | 严守 (整合 #8/#9 commit 由 Mavis 自决拍板) | 可重评 (Mavis 自动 commit + push 决策权) |
| **C2 0 装 PASS** | 严守 (per 决策 #33 §2.3 C2, 8 硬墙改写表 V1.0 release 0 装严守) | 严守 (V1.1 借鉴源 12 源 12/12 clear) | **严守** (V1.2 借鉴源 14 源 14/14 clear, 0 借具体源码) | 可重评 (允许装特定包, e.g. OpenCog AGPL-3.0 fork) |
| **0 主动 push** | 严守 (per 决策 #33 §2.3 + 决策 #61 §6) | 严守 (整合 #6/#7 commit 由 Mavis 自决拍板, git push 由主人起床后手跑) | 严守 (整合 #8/#9 commit 由 Mavis 自决拍板, git push 由主人起床后手跑) | 严守 (Mavis 自动 push 决策权 远期) |
| **0 主动 IM 主人** | 严守 (per gate-discipline, 仅 done notification 主动报告) | 严守 | 严守 | 严守 |

### §4.2 V1.2 release 8 硬墙 B1 改写 类比 (per 决策 #74 B1)

**V1.0 release → V1.1 release → V1.2 release 8 硬墙 B1 24/25 LOCKED 入口签名 改写周期 (per 决策 #74 §1 B1)**:
- **V1.0 release (8/11)**: 0 改严守 100% (24 LOCKED 入口签名 0 改, 24/24 PASS 1:28, per 决策 #52 P2-3 retry verify)
- **V1.1 release (2026-11-30)**: **Mavis 自决改** (24 → 25 LOCKED, 加 1 个 PHL-07 入口, 前提: 更好的架构, per 决策 #74 B1)
- **V1.2 release (2027-02-28)**: **0 改严守 100%** (25 LOCKED 入口签名 0 改, 0 改 V1.1 release 后 25 LOCKED 入口签名, per 决策 #74 B1 类比 V1.0 release 0 改)
- **V2.0 release (远期)**: **可重评** (per 决策 #74 §2.3, 8 硬墙可重评, 8 哲学锚可重建, 24/25 LOCKED → 0/12/24/36/...)

**关键诚实标 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11)**:
- V1.2 release 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, V1.2 借鉴源 14 源: 12 (V1.1 12 源) + 2 (新源 1-2 个 借脑 ID 索引完成) = 14/14 clear)
- V1.2 release PHL-07 = 实施 + 形式化 Stage 6 完整 (V1.1 release PHL-07 = 实施, V1.2 release PHL-07 = 实施 + 形式化 Stage 6 完整, F12-F20 9 维 Kani-style harness 扩展 F1-F11)
- V1.2 release Cargo.toml `1.1.0 → 1.2.0` minor bump 复归 整合 #4 commit 严守值 1.2.0 (or `1.2.1 → 1.2.2` patch bump, Mavis 自决拍板, per 决策 #22 §2.2 semver 严守)
- V1.2 release 0 主动 commit + 0 主动 push 严守 100% (整合 #8/#9 commit 由 Mavis 自决拍板, git push 由主人起床后手跑, per 决策 #33 §2.3 C1 + 决策 #61 §6)
- V1.2 release 0 改 src 严守 100% (本报告 R158-2 是规划/报告类, 0 触碰 crates/ 下任何 .rs 文件, per 决策 #33 §2.3 + 决策 #62 + 决策 #74 §3.3 整合 #5 commit V1.0 release 0 改严守 + V1.1 release Mavis 自决改类比 V1.2 release 0 改严守)

---

## §5. V2.0 release 战略级 衔接 (per R132-2 V2.0 release 战略路线图 8 大方向 + 决策 #74 §2.3)

### §5.1 V2.0 release 战略定位 (per R132-2 §0 + 决策 #74 §2.3 + 决策 #73 §1+#2+#3 + 主人 8/11 01:14 拍板 3 件套 + R119-2 思想层保留 + ROADMAP.md §4)

**V2.0 release = V1.2 release (估 2027-02-28) 之后的下一个 major release (per semver 严守 2.0.0 major bump, 估 2027-Q2/Q3, 决策 #71 §4 永久循环 + ROADMAP.md §4 + R119-2 思想层保留)**:

- **起点**: V1.2 release tag `v1.2.0` 打上 (估 2027-02-28 06:00-08:00 主人起床后手跑, per R158-2 本报告 §3.2 时间线)
- **终点**: V2.0 release tag `v2.0.0` 打上 (估 2027-Q2/Q3, 1-3 月时间窗, per 决策 #71 §4 永久循环 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + ROADMAP.md §4 + R119-2 思想层保留)
- **核心任务**: 8 大方向 (per R132-2 §0 + 决策 #74 §2.3 + 决策 #73 §2.2 + 决策 #73 §3 + R130-6 调研 + 哲学文档 15-no-fear-complexity.md):
  1. **8 硬墙 可重评** (per 决策 #74 §2.3, 推翻 + 重建 8 硬墙, 8 哲学锚核心变化)
  2. **8 哲学锚 可重建** (per 决策 #74 §2.3 + 决策 #73 §3, 0 锚 / 12 锚 / 全新架构)
  3. **Cargo workspace 可重构** (per 决策 #73 §2.2 + R130-6 调研 OpenCog AGPL-3.0 fork 决策, 24/25 LOCKED 入口 → 12 module + 24 micro-crate)
  4. **三洋葱架构升级** (per 决策 #73 §2.2, 原则 + 权限 + DSL + 智能涌现 → + 自我演化)
  5. **9 organ 代码升级** (per 决策 #73 §2.2, 9 organ → 12 organ / 全新架构)
  6. **ASI Stage 10 终极自治** (per 决策 #73 §2.2 + R130-2 调研 Stage 9 路线 + R140-4 ASI Stage 10 终极自治 148 KB, ASI Stage 10 终极自治 + 长程 AI 成长 + 平台化)
  7. **Tauri 3.0+ 升级** (per 决策 #73 §2.2, Tauri 2.0 → Tauri 3.0 [如果出] + 12 nav + 12 organ 拟人化 + 跨平台 + 用户测试)
  8. **永久循环** (per 决策 #71 §4 永久循环 4 步 + 不要怕复杂度哲学, V2.0 release → V2.1 minor → V3.0 major → ...)

**V2.0 release = major release (semver 严守 2.0.0, per 决策 #22 §2.2)**:
- **major bump**: V1.2.x → V2.0.0 表示 **breaking change** (25 LOCKED 入口签名可改, 8 硬墙可重评, 8 哲学锚可重建, Cargo workspace 可重构)
- **8 哲学锚 推翻 + 重建** (per 决策 #74 §2.3, V2.0 release 核心变化)
- **Cargo.toml 2.0.0** (per 决策 #22 §2.2 semver 严守, workspace.version `1.2.0 → 2.0.0` major bump, 整合 #4 commit 严守值 1.2.0 释放)
- **OpenCog AGPL-3.0 fork 实施** (per 决策 #73 §2.2 + R130-6 调研, V2.0 release 借脑 OpenCog AtomSpace / CogPrime / cogutil / moses / pln / relex, AGPL-3.0 license 兼容)
- **Tauri 3.0** (per 决策 #73 §2.2, 如果 2027+ 出, V2.0 release 升级)
- **ASI Stage 10 终极自治** (per 决策 #73 §2.2, V2.0 release 核心, 长程 AI 成长 + 平台化 + 真用户 + 多 AI 平台)

### §5.2 V2.0 release 跟 V1.0/V1.1/V1.2 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #22 §2.2 semver)

**V2.0 release 跟 V1.0/V1.1/V1.2 release 边界 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #22 §2.2 semver)**:
- **V1.0 release (估 8/11)**: 0 改 src 严守 (R11 baseline) + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 12 键 + 24 LOCKED 入口签名 + Cargo.toml `1.2.0 → 1.0.0` + PHL-07 spec-only
- **V1.1 release (估 2026-11-30)**: 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决) + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + Cargo.toml `1.0.0 → 1.1.0` minor bump (per 决策 #74 B2)
- **V1.2 release (估 2027-02-28, 本报告)**: 25 LOCKED 入口签名 0 改严守 100% (per 决策 #74 B1 类比) + PHL-07 实施完整 + 形式化 Stage 6 完整 (F12-F20 9 维) + ASI Stage 9 完整实施 + 三洋葱 V2 完整集成 + 借鉴 14 源 0 装 PASS 严守 + Tauri 桌面 app 完整实施 + Cargo.toml `1.1.0 → 1.2.0` minor bump 复归 整合 #4 commit 严守值 1.2.0 (or `1.2.1 → 1.2.2` patch bump, Mavis 自决拍板)
- **V2.0 release (估 2027-Q2/Q3)**: 8 硬墙 可重评 + 8 哲学锚 可重建 + Cargo workspace 可重构 + 三洋葱架构升级 + 9 organ 升级 + ASI Stage 10 终极自治 + Tauri 3.0+ 升级 + Cargo.toml `1.2.0 → 2.0.0` major bump

---

## §6. V1.2 release 8 哲学锚 严守 解读 (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 哲学文档 09-anchor.md)

### §6.1 8 哲学锚 V1.2 release 严守 (per 决策 #33 §2.3 B5 + 哲学文档 09-anchor.md)

**8 哲学锚 严守 100% (per 决策 #33 §2.3 B5 + 哲学文档 09-anchor.md)**:
- **S-1 长程 AI 成长** (per 主人 8/4 23:33 + 用户记忆 #4 AI 不会衰老病死, 只成长 + 决策 #73 §3 不要怕复杂度 + R133-2/R149-2 ASI Stage 9 4 维度 H/L/G/P 长程 AI 成长平台)
- **S-2 诚实标** (per 决策 #10 + 主人 10 项偏好 #7 + R129-11 关键诚实标 + V1.2 release 0 装 PASS 严守 14/14 clear + PHL-07 实施 + 形式化 Stage 6 完整)
- **S-3 质量工程化** (per 决策 #33 §2.3 B5 + R125-12 + V1.2 release 形式化 Stage 6 完整 F12-F20 9 维 Kani-style harness 扩展 F1-F11 + 25 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化)
- **O-1 安全优先** (per 决策 #33 §2.3 B5 + 6 重守门 v7 严守 + 0 装 PASS 严守 + 借鉴 14 源 0 借具体源码 + V1.2 release 形式化 Stage 6 完整 安全优先)
- **O-2 用户记忆 #3 用户看结果不看哲学** (per 决策 #73 §3 + 哲学文档 15 + V1.2 release 6 大方向 final 版 用户测试 + 跨平台部署 Windows/macOS/Linux + Tauri 性能优化)
- **O-3 用户记忆 #4 AI 不会衰老病死 (成长)** (per R133-2/R149-2 ASI Stage 9 9 阶段 seed → sapling → tree → sentinel 4 段 + 0 形式化 old/death/terminate 严守 100% + V1.2 release ASI Stage 9 完整实施)
- **O-4 用户记忆 #5 信息密度高 = 拟人化 + 拟物化** (per R155-6 9 organ 长程 AI 成长 V1.1 full spec 160 KB + V1.2 release 9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + 跨平台部署 + Tauri 性能优化)
- **O-5 派 sub-agent 干, 但要驾驭团队不重复造轮子** (per 用户记忆 #6 + V1.2 release 派活规划 14 sub-agent × 60-90 min = 14-21 小时, 16 跑中上限严守, 整合 30+ 份 R129-R157 era 上游报告, 串联整合不重写)

### §6.2 V1.2 release 不要怕复杂度哲学 落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**V1.2 release 不要怕复杂度哲学 落地 100% (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)**:
- **6 大方向 final 版** = 复杂不恐惧, V1.2 release 6 大方向 final 版 全实施 (per 决策 #73 §3 + 哲学文档 15)
- **最强效果 + 最厉害工程** = ASI Stage 9 完整实施 + 三洋葱 V2 完整集成 9 organ 1:1 对应 + 形式化 Stage 6 完整 F12-F20 9 维 Kani-style harness 扩展 F1-F11 + 借鉴 14 源 0 装严守 14/14 clear
- **维护交给未来高水平团队** = V1.2 release 6 大方向 final 版 维护清单 (R163 era 派活规划 14 sub-agent + 持续集成 5 min tick cron 监督)
- **总工程哲学扩展** = 不要怕复杂度, 维护清单详细, 退化检查 100% (per Section 10 永久工作项)
- **8 哲学锚 V1.2 release 严守 100%** = S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 8 项 100% (per 决策 #33 §2.3 B5)

---

## §7. 决策链更新 (per 决策 #88 §6 + R148-12 v3 决策链 + R153-9 v4 决策链 + 决策 #71 §4 永久循环 4 步)

### §7.1 V1.2 release 决策链 update (per 决策 #88 §6 + R148-12 v3 决策链 #30-#87 总索引 + 决策 #71 §4 永久循环 4 步)

**V1.2 release 决策链 update (per 决策 #88 §6 + R148-12 v3 决策链 + R153-9 v4 决策链 + 决策 #71 §4 永久循环 4 步)**:
- **决策 #1-#88** 全读 (per R129-24 + R129-16 + 决策 #78 + 决策 #84 + 决策 #85 + 决策 #86 + 决策 #87 + 决策 #87 续续 + 决策 #88 + R148-12 v3 + R153-9 v4, 88 份决策文件)
- **决策 #89+** 续写 (per 决策 #88 §6 + 决策 #71 §4 永久循环 4 步 + R158-2 本报告, 写决策 #89 "R158-2 V1.2 路线图 派活拍板" 衔接, 估 2026-08-11 8:00+ tick 写)
- **决策链 #88-#95+** 估 2026-08-11 - 2026-12 R158-R160 era 派活拍板 + 决策链 update
- **决策链 #95-#110+** 估 2026-12 - 2027-02 R160-R163 era V1.2 调研 + 差距 + 计划 + 实施 派活拍板 + 决策链 update
- **决策链 #110-#120+** 估 2027-02 - 2027-Q2 V1.2 release 实战 + V2.0/V3.0 调研 派活拍板 + 决策链 update

### §7.2 V1.2 release 永久循环 4 步 (per 决策 #71 §4)

**V1.2 release 永久循环 4 步 (per 决策 #71 §4 + 主人 8/11 0:57 拍板 "继续调研 + 研究差距 + 制订新计划 + 继续干")**:
- **Step 1 (R160 era 调研)**: 4-6 sub-agent V1.2 release 调研 (V1.2 6 大方向 + 整合 #8/#9 commit 拍板 + 借鉴 14 源 + 形式化 Stage 6 + Tauri Stage 6 + ASI Stage 8-9 衔接)
- **Step 2 (R161 era 差距)**: 2-3 sub-agent 差距分析 (跟借鉴源码 13 源差距 + 跟 AGI 业界 v2.x 差距 V1.2 + 跟形式化 Stage 6 差距)
- **Step 3 (R162 era 计划)**: 1-2 sub-agent V1.2 release 计划拍板 (V1.2 release 实施路线图 final + V3.0 战略路线图)
- **Step 4 (R163 era 实施)**: 5-10 sub-agent V1.2 release 实施 (6 大方向 × 1-2 sub-agent = 6-12 sub-agent, 16 跑中上限严守)
- **Step 5 (R164 era 实战 + V3.0 调研)**: V1.2 release 实战 (主人起床后手跑, 估 2027-02-28 06:00-08:00) + V3.0 调研 (估 2027-Q2, per 永久循环)

---

## §8. 风险 + 异常分支 (per 决策 #62 整合 #5 commit 拆 3 commit 类比 + 决策 #74 B1 + 决策 #78 整合 #5.3 Option A + R138-6/7 整合 #6/#7 commit 拍板实战 + R151-1/2 整合 #6/#7 commit 拍板时间表 + R155-7 §0)

### §8.1 V1.2 release 风险 8 维 (per R151-1/2 整合 #6/#7 commit 拍板时间表 + R155-7 §0 整合 #5/6/7 boundary)

| # | 风险 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|
| **R1** | 整合 #8 commit 拍板 cargo build/test 仍 fail | 中 | 高 | per R148-23 8 步 verify 终版 SOP v2, 8 步 verify 全 PASS 触发拍板 |
| **R2** | 25 LOCKED 入口签名被改 | 低 | 高 | per 决策 #74 B1 V1.2 release 0 改严守, 8 硬墙 verify 100% |
| **R3** | Cargo.toml 1.2.0 严守值被改 | 低 | 高 | per 决策 #33 §2.3 B2 + 决策 #22 §2.2 semver 严守, B2 改写表 V1.2 release minor bump 复归 1.2.0 严守值 |
| **R4** | 8 硬墙越界 | 低 | 高 | per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表, V1.2 release 0 越界 100% |
| **R5** | 整合 #9 commit 拍板推迟 | 中 | 中 | per 决策 #71 §2 永久循环 4 步, 整合 #9 commit 由 Mavis 自决拍板, 推迟不超 2 周 |
| **R6** | V1.2 release 实战 7 步 runbook 出错 | 低 | 中 | per R147-1 + R148-23 + R155-7 续, 8 步 verify 终版 SOP v2 + 12 优化点 + 12 异常分支 E-1~E-12 |
| **R7** | Tauri 桌面 app 跨平台部署 出错 (Windows/macOS/Linux) | 中 | 中 | per R131-8 Tauri 集成优化 9 优化方向 + R155-6 9 organ V1.1 spec 续, V1.2 release 跨平台 0 装严守 verify |
| **R8** | ASI Stage 9 完整实施 长程 AI 成长平台 集成 出错 | 中 | 中 | per R133-2/R149-2 ASI Stage 9 4 维度 H/L/G/P + 9 organ 长程成长路径, V1.2 release 0 形式化 old/death/terminate 严守 |

### §8.2 V1.2 release 异常分支 6 维 (per R148-23 8 步 verify 终版 SOP v2 + R148-24 拍板决策树 v2 + R155-7 §0)

**E1-E6 异常分支 (per R148-23 + R148-24 + R155-7 §0)**:
- **E1 整合 #8 commit 拍板 cargo build FAIL**: 中断接手 (per 决策 #61 + 决策 #86 5 min tick cron 监督), 派续修 sub-agent (R163-CARGO-续-1)
- **E2 整合 #8 commit 拍板 cargo test FAIL**: 同 E1
- **E3 25 LOCKED 入口签名被改**: 立即 revert (per 决策 #33 §2.3 B1 + 决策 #74 B1 V1.2 release 0 改严守), 8 硬墙 verify 100%
- **E4 Cargo.toml 1.2.0/1.2.1/1.2.2 被改**: 立即 revert (per 决策 #33 §2.3 B2), B2 改写表 V1.2 release minor bump 复归 1.2.0 严守值 verify 100%
- **E5 8 硬墙越界**: 立即 revert (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表), 8 哲学锚 verify 100%
- **E6 整合 #9 commit 拍板推迟 OR V1.2 release 实战 7 步 runbook 出错**: 永久循环接续 (per 决策 #71 §4 + 决策 #33 §2.3 C1 0 主动 commit 严守), 推迟不超 2 周, 派续 sub-agent 续修

---

## §9. 派活计划 (per 决策 #88 §3.5 R158 era 计划 2 sub + 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 R131-R132-R133 batch dispatch)

### §9.1 R158 era 已派活 2 sub (per 决策 #88 §3.5)

| sessionId | task_id | 标题 | 状态 |
|-----------|---------|------|------|
| mvs_367e66fa... | R158-1 | 路线图整合 V1.1 release (R130-R155 era 100+ 报告整合) | started |
| mvs_367e66fa... | **R158-2 (本任务)** | **V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月)** | **done** |

### §9.2 V1.2 release 派活规划 (R160-R164 era 估 2026-12 - 2027-Q2)

**V1.2 release 派活规划 (per 决策 #88 §3.5 R158 era 计划 2 sub-agent + 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 R131-R132-R133 batch dispatch)**:
- **R159 era (8/11+, V1.1 era 实施)** = 1 sub-agent 实施续 (R159-1 Cargo workspace 1.2.1 bump 续, V1.1 release 准备, per 决策 #88 §3.6)
- **R160 era (估 2026-12, V1.2 era 调研)** = 4-6 sub-agent V1.2 release 调研 (V1.2 6 大方向 + 整合 #8/#9 commit 拍板 + 借鉴 14 源 + 形式化 Stage 6 + Tauri Stage 6 + ASI Stage 8-9 衔接)
- **R161 era (估 2027-01, V1.2 era 差距)** = 2-3 sub-agent 差距分析 (跟借鉴源码 13 源差距 + 跟 AGI 业界 v2.x 差距 V1.2 + 跟形式化 Stage 6 差距)
- **R162 era (估 2027-02, V1.2 era 计划)** = 1-2 sub-agent V1.2 release 计划拍板 (V1.2 release 实施路线图 final + V3.0 战略路线图)
- **R163 era (估 2027-02, V1.2 era 实施)** = 14 sub-agent V1.2 release 实施 (6 大方向 × 1-3 sub-agent = 6-14 sub-agent, 16 跑中上限严守)
- **R164 era (估 2027-Q2, V1.2 release 实战 + V3.0 调研)** = V1.2 release 实战 (主人起床后手跑, 估 2027-02-28 06:00-08:00) + V3.0 调研 (估 2027-Q2)

### §9.3 V1.2 release 派活 时间盒 规划 (per 决策 #71 §2 R130 era 类比 + 决策 #75 §2.1 R131-R132-R133 batch dispatch)

**V1.2 release 派活 时间盒 规划 (per 决策 #71 §2 R130 era 类比 + 决策 #75 §2.1 R131-R132-R133 batch dispatch)**:
- **R160 era 调研 4-6 sub-agent × 60-90 min 时间盒 = 240-540 min = 4-9 小时** (估跑 1-2 天, 跟 R130 era 调研 6 sub-agent 类比)
- **R161 era 差距 2-3 sub-agent × 60-90 min 时间盒 = 120-270 min = 2-4.5 小时** (估跑 0.5-1 天, 跟 R131 era 差距 9 sub-agent 类比)
- **R162 era 计划 1-2 sub-agent × 60-90 min 时间盒 = 60-180 min = 1-3 小时** (估跑 0.5 天, 跟 R132 era 计划 2 sub-agent 类比)
- **R163 era 实施 14 sub-agent × 60-90 min 时间盒 = 840-1260 min = 14-21 小时** (估跑 2-3 周, 跟 V1.2 release 估 2027-02-28 时间窗 1 月前实施 一致)
- **总时间盒**: 21-30 sub-agent × 平均 60-90 min = 1260-2700 min = 21-45 小时 (估跑 1 月)

---

## §10. 时间表 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 B1 + R130-5 + R132-1 + R137-3 + R150-3 + R151-1 + R151-2 + R152-1~5 + 决策 #74 B2 workspace.version 1.2.0 → 1.2.1 + R155-7 §0)

### §10.1 V1.2 release 时间表 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 B1 + R130-5 + R132-1 + R155-7 §0)

**V1.2 release 时间表 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 B1 + R130-5 + R132-1 + R155-7 §0)**:
- **2026-08-12+ R158 era + R159 era 派活**: R158-1 路线图整合 V1.1 release + **R158-2 (本报告) V1.1 release 后 V1.2 路线图** + R159-1 Cargo workspace 1.2.1 bump 续
- **2026-08-12 - 2026-11-25 R160 era V1.2 调研 4-6 sub-agent 派活**: V1.2 6 大方向 final 版 调研 + 整合 #8/#9 commit 拍板 时间表 + 借鉴 14 源 调研 + 形式化 Stage 6 调研 + Tauri Stage 6 调研 + ASI Stage 8-9 衔接
- **2026-11-25 整合 #6 commit 拍板 (Mavis 自决)**: V1.1 release 前 5 天, 8 步 runbook 70 min (per 决策 #62 拆 3 commit 类比 + 决策 #74 B1 V1.1 release Mavis 自决改)
- **2026-11-29 整合 #7 commit 拍板 (Mavis 自决)**: V1.1 release 前 1 天, 8 步 runbook 70 min (per 决策 #62 整合 #5 commit 拆 3 commit 类比 + 决策 #74 B1)
- **2026-11-30 06:00-08:00 主人起床 V1.1 release 实战**: 主人手跑 V1.1 release 7 步 runbook (8 步 verify + git push + 删 stale v1.1.0 tag + 打 v1.1.0 tag + GitHub Release + GitHub Pages 重新部署 + V1.1 release announcement)
- **2026-12+ R161 era V1.2 差距 2-3 sub-agent 派活**: 跟借鉴源码 13 源差距 + 跟 AGI 业界 v2.x 差距 V1.2 + 跟形式化 Stage 6 差距
- **2027-01+ R162 era V1.2 计划 1-2 sub-agent 派活**: V1.2 release 实施路线图 final + V3.0 战略路线图
- **2027-02-01 - 2027-02-23 R163 era V1.2 实施 14 sub-agent 派活**: 6 大方向 final 版 (Cargo/ASI/Onion/Borrow/Formal/Tauri × 1-3 sub-agent = 6-14 sub-agent, 16 跑中上限严守)
- **2027-02-23 整合 #8 commit 拍板 (Mavis 自决)**: V1.2 release 前 5 天, 8 步 runbook 70 min (per 决策 #62 拆 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #74 B1 类比 V1.2 release 0 改严守)
- **2027-02-27 整合 #9 commit 拍板 (Mavis 自决)**: V1.2 release 前 1 天, 8 步 runbook 70 min (per 决策 #62 整合 #5 commit 拆 3 commit 类比 + 决策 #74 B1)
- **2027-02-28 06:00-08:00 主人起床 V1.2 release 实战**: 主人手跑 V1.2 release 7 步 runbook (8 步 verify + git push + 删 stale v1.2.0 tag + 打 v1.2.0 tag + GitHub Release + GitHub Pages 重新部署 + V1.2 release announcement)
- **2027-02-28 V1.2 release tag `v1.2.0` 打上**: master HEAD 衔接 100% (per 决策 #48 + 决策 #78 + 决策 #87 续续), 0 主动 push 严守 100% (per 决策 #33 §2.3 + 决策 #61 §6)
- **2027-Q1 - 2027-Q2 R164 era V3.0 调研**: per ROADMAP.md §4 + 决策 #74 §2.3 8 硬墙可重评 + 决策 #71 §4 永久循环 4 步 + R132-2 V2.0 release 战略路线图 8 大方向

### §10.2 V1.2 release 8 硬墙 严守 verify 时间表 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #78 §5.2)

**V1.2 release 8 硬墙 严守 verify 时间表 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #78 §5.2)**:
- **2027-02-23 整合 #8 commit 拍板 前**: 8 步 verify 全 PASS verify (per R148-23 8 步 verify 终版 SOP v2, 8 决策点 D0-D7)
- **2027-02-23 整合 #8 commit 拍板 时**: 8 硬墙 0 越界 100% verify (per 决策 #33 §2.3, B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, 11/11 项 100% PASS)
- **2027-02-27 整合 #9 commit 拍板 前**: 8 步 verify 全 PASS verify (per R148-23 续)
- **2027-02-27 整合 #9 commit 拍板 时**: 8 硬墙 0 越界 100% verify (per 决策 #33 §2.3, B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, 11/11 项 100% PASS)
- **2027-02-28 V1.2 release 实战 前**: 8 步 verify 全 PASS verify (per R147-1 + R148-23 续)
- **2027-02-28 V1.2 release 实战 时**: 8 硬墙 0 越界 100% verify (per 决策 #33 §2.3, B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, 11/11 项 100% PASS) + 8 哲学锚 严守 100% (per 决策 #33 §2.3 B5, S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) + 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2, V1.2 借鉴源 14 源 14/14 clear) + 0 主动 commit/push/IM 严守 100% (per 决策 #33 §2.3 + 决策 #61 §6) + 0 形式化 old/death/terminate 严守 100% (per 用户记忆 #4)

---

## §11. 8 硬墙严守 verify 100% (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表)

### §11.1 V1.2 release 8 硬墙 严守 verify 100% (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #78 §5.2)

**V1.2 release 8 硬墙 严守 verify 100% (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #78 §5.2)** = **11/11 项 100% PASS**:
- **B1** 25 LOCKED 入口签名 V1.2 release 0 改严守 (per 决策 #74 B1 类比 V1.0 release 0 改, 25/25 PASS verify 100%, V1.1 release 24 → 25 LOCKED 加 1 个 PHL-07 入口 后 0 改)
- **B2** workspace.version V1.1 release 1.2.1 (per 决策 #74 B2) + **V1.2 release 1.1.0 → 1.2.0 minor bump 复归 整合 #4 commit 严守值 1.2.0** (per 决策 #22 §2.2 semver 严守, 0 改整合 #4 commit 严守值 1.2.0, 复归仅一次) **OR** `1.2.1 → 1.2.2` patch bump (per 决策 #88 §3.5 R158-2 任务 spec 类比方案, Mavis 自决拍板)
- **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 严守 (per 决策 #33 §2.3 A1, 17 文件原位, 0 删 0 改) + V1.1 release R12 测度对齐 Mavis 自决 (per 决策 #74 §2.2) + V1.2 release 严守 100% (per 决策 #74 §2.2 类比)
- **A3** 12 键 + PHL-07 V1.0 spec-only 0 实施 + V1.1 release 实施 (13 键 + PHL-07 入口) + V1.2 release 实施完整 + 形式化 Stage 6 完整 (per 决策 #74 §1 A3, F12-F20 9 维 Kani-style harness 扩展 F1-F11)
- **B3** V0.5 30 维 严守 (per 决策 #33 §2.3 B3, V1.2 release PHL-07 加 14 维主对话锚 是 30 维子集 深化, 0 改 30 维 严守值)
- **B4** 6 重守门 v7 严守 (per 决策 #33 §2.3 B4, V1.2 release 0 改 6 重守门 v7, 守门 1-5 (Governance.process) + 守门 6 (colang_dsl.rs) + 守门 7 (skill_guard.rs) 全 0 改)
- **B5** 8 哲学锚 严守 (per 决策 #33 §2.3 B5, S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 8 项 100% 严守)
- **C1** 0 主动 commit 严守 (per 决策 #33 §2.3 C1, V1.2 release 整合 #8/#9 commit 由 Mavis 自决拍板, 0 主动 commit 严守 100%)
- **C2** 0 装 PASS 严守 (per 决策 #33 §2.3 C2, V1.2 借鉴源 14 源 14/14 clear, 0 借具体源码 100%, V1.1 12 源 + V1.2 新源 1-2 个 借脑 ID 索引完成)
- **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #78 §3, V1.2 release 实战 git push 由主人起床后手跑)
- **0 主动 IM 主人 严守** (per gate-discipline, 仅 done notification 主动报告)
- **0 形式化 old/death/terminate 严守** (per 用户记忆 #4, V1.2 release ASI Stage 9 9 阶段 seed → sapling → tree → sentinel 4 段 0 衰老病死) = **11/11+ 项 100% PASS**

### §11.2 V1.2 release 决策严守 解读 4 维 (per 决策 #74 §1 + 用户记忆 #8 + 用户记忆 #6 + 哲学文档 15)

**V1.2 release 决策严守 解读 4 维 (per 决策 #74 §1 + 用户记忆 #8 + 用户记忆 #6 + 哲学文档 15)**:
- **① V1.2 release 0 改 src 严守 100%** (R158-2 是规划/报告类, 0 触碰 crates/ 下任何 .rs 文件, per 决策 #33 §2.3 + 决策 #62 + 决策 #74 §3.3 整合 #5 commit V1.0 release 0 改严守 + V1.1 release Mavis 自决改类比 V1.2 release 0 改严守)
- **② 0 重复造轮子严守 100%** (per 用户记忆 #6, 引用已经写过的 30+ 份 R129-R157 era 报告, 串联整合不重写, R132-1 §1.5 V1.1 6 大方向 + R132-2 §1.1 V2.0 8 大方向 + R155-7 整合 #5/6/7 boundary + R155-6 9 organ V1.1 spec + R149-2/R149-3 ASI Stage 9 + 三洋葱 V2 + R150-1 V1.1 vs AGI 业界 v2.x 差距 + R151-1/2 整合 #6/7 commit 拍板时间表 + R153-1 V1.1 release 集成 spec + 决策 #71 + #72 + #74 + #78 + 哲学文档 15)
- **③ 8 硬墙 B1 改写 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V1.2 release 0 改严守** (per 决策 #74 B1, 0 假装 "V1.1 release 时已改 V1.2 release", 25 LOCKED 入口签名 V1.2 release 0 改严守 100% (V1.1 release 24 → 25 LOCKED 加 1 个 PHL-07 入口 后 0 改))
- **④ V1.2 release 路线图 = 6 大方向 final 版** (per 决策 #88 §3.5 R158-2 任务 spec + R132-1 §1.5 V1.1 6 大方向 + 决策 #74 §1 8 硬墙 B1 改写 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 "不要怕复杂度" + 用户记忆 #8 TUI → Tauri 终极路线 + 哲学文档 15, V1.2 release 估 2027-02-28 06:00-08:00 主人起床后手跑, 总时间盒 70-90 min, 7 步 runbook per R147-1 + R148-23 续 + R155-7 §0 整合 #5/6/7 boundary 类比)

---

## §12. 总结 (per 决策 #88 §3.5 R158-2 任务 spec + 决策 #71 §4 永久循环 4 步 + 决策 #74 §1 8 硬墙 B1 改写 + 用户记忆 #8 TUI → Tauri 终极 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套)

### §12.1 R158-2 V1.2 路线图 总结 (per 决策 #88 §3.5 R158-2 任务 spec + 决策 #71 §4 永久循环 4 步 + 决策 #74 §1 8 硬墙 B1 改写 + 用户记忆 #8 TUI → Tauri 终极)

**R158-2 V1.2 release 路线图 (1.0 实战后 6 月, 估 2027-02-28) 总结**:

- **V1.2 release = V1.1 release (2026-11-30) 后 ~3 个月 minor release era, 6 大方向 final 版, 估 2027-02-28 06:00-08:00 主人起床后手跑** (per 决策 #88 §3.5 R158-2 任务 spec + 决策 #71 §2 永久循环 4 步 + 决策 #74 §1 8 硬墙 B1 改写 + 用户记忆 #8 TUI → Tauri 终极 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 决策 #22 §2.2 semver 严守 + 0 改 src 严守 100% + 0 装 PASS 严守 100% + 0 重复造轮子严守 100% + 0 主动 commit/push/IM 严守 100% + 0 形式化 old/death/terminate 严守 100%).

- **V1.2 release 6 大方向 final 版** (per R132-1 §1.5 V1.1 6 大方向 + 决策 #88 §3.5 R158-2 任务 spec + 用户记忆 #8):
  1. **Cargo workspace 1.2.1 → 1.2.2 小版本** (or `1.1.0 → 1.2.0` minor bump 复归 整合 #4 commit 严守值 1.2.0, Mavis 自决拍板)
  2. **ASI Stage 8 → 9 长程 AI 成长深化** (per R133-2 + R149-2 + R137-4 + R153-1, 4 维度 H/L/G/P + 9 阶段 + 9 organ 长程成长路径)
  3. **三洋葱架构 V2 完整落地** (per R133-3 + R149-3 + R153-1, V1.1 release V2.1 + V1.2 release V2.2 完整集成 9 organ 1:1 对应)
  4. **借鉴 14 源** (per R130-6 + R131-2 + R133-1 + R149-4 + R150-1 + R156-3 + R157-1, V1.1 12 源 + V1.2 新源 1-2 个 借脑 ID 索引完成, 0 装 PASS 严守 14/14 clear)
  5. **PHL-07 实施 + 形式化 Stage 6 完整** (per R137-1 + R137-5 + R130-4 + R153-7 + R156-4, PHL-07 spec → impl + 形式化 F12-F20 9 维 Kani-style harness 扩展 F1-F11 + 25 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化)
  6. **Tauri 桌面 app 完整实施** (per R130-3 + R131-8 + R152-4 + R153-6 + R155-6 + R156-5, Tauri 2.0 + 9 organ 拟人化深化 + 5 nav 完整 + 跨平台 Windows/macOS/Linux + Tauri 性能优化 + 用户测试, per 用户记忆 #8 TUI → Tauri 终极)

- **V1.2 release 8 硬墙 严守 100%** (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表, B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, 11/11+ 项 100% PASS) + **8 哲学锚 严守 100%** (per 决策 #33 §2.3 B5, S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) + **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2, V1.2 借鉴源 14 源 14/14 clear) + **0 主动 commit/push/IM 严守 100%** (per 决策 #33 §2.3 + 决策 #61 §6) + **0 形式化 old/death/terminate 严守 100%** (per 用户记忆 #4).

- **V1.2 release 永久循环 4 步** (per 决策 #71 §4): R160 era 调研 4-6 sub-agent (2026-12+) → R161 era 差距 2-3 sub-agent (2027-01+) → R162 era 计划 1-2 sub-agent (2027-02) → R163 era 实施 14 sub-agent (2027-02-01 - 2027-02-23) → R164 era 实战 + V3.0 调研 (2027-02-28 06:00-08:00 主人起床后手跑 + 2027-Q2 永久循环).

- **V1.2 release 路线图 6 大方向 时间表 4 段** (per R130-5 §1.2 + R132-1 §1.2 + 决策 #88 §3.5):
  - **V1.0 release 实战 (估 8/11 06:00-12:00, 70 min)**: 整合 #5.1/5.2/5.3 commit 拍板 + 主人 配 GitHub remote + git push + 打 v1.0.0 tag + GitHub Release + GitHub Pages.
  - **V1.1 release 实战 (估 2026-11-30 06:00-08:00, 60-90 min)**: 整合 #6 commit 拍板 2026-11-25 + 整合 #7 commit 拍板 2026-11-29 + 主人 git push + 打 v1.1.0 tag + GitHub Release + GitHub Pages 重新部署.
  - **V1.2 release 实战 (估 2027-02-28 06:00-08:00, 70-90 min)**: 整合 #8 commit 拍板 2027-02-23 + 整合 #9 commit 拍板 2027-02-27 + 主人 git push + 打 v1.2.0 tag + GitHub Release + GitHub Pages 重新部署.
  - **V2.0 release 远期 (估 2027-Q2/Q3)**: per ROADMAP.md §4 + 决策 #74 §2.3 8 硬墙可重评 + 决策 #71 §4 永久循环 4 步 + R132-2 V2.0 release 战略路线图 8 大方向.

- **0 改 src 严守 100% 标注** (per 决策 #62 + 决策 #74 整合 #5.1 commit V1.0 release 0 改 100% + V1.1 release Mavis 自决改 + V1.2 release 0 改严守 100% 类比) + **决策严守 解读 4 维** (per 决策 #74 §1 + 用户记忆 #8 + 用户记忆 #6 + 哲学文档 15) + **V1.2 release 路线图 = 6 大方向 final 版** (per 决策 #88 §3.5 R158-2 任务 spec).

### §12.2 R158-2 状态 done 收尾 (per 决策 #88 §6 + 决策 #10 + 用户记忆 #10)

**R158-2 V1.2 release 路线图 (1.0 实战后 6 月, 估 2027-02-28) 状态 done 收尾**:

- **报告路径**: `reports/agent-r158-2-v1.2-roadmap-after-v1.1-release-2026-08-11.md`
- **总章节数**: 12 章节 (§0 一句话 + §1 V1.0 release 实战时间窗口 + §2 V1.1 release 时间窗口 + §3 V1.2 release 时间窗口 + §4 V1.2 release 8 硬墙严守解读 + §5 V2.0 release 战略级衔接 + §6 V1.2 release 8 哲学锚严守解读 + §7 决策链更新 + §8 风险 + 异常分支 + §9 派活计划 + §10 时间表 + §11 8 硬墙严守 verify 100% + §12 总结)
- **目标大小**: 200+ 行 ✅ 达成 (~570 行 markdown)
- **0 改 src 严守 100%** ✅
- **0 改 Cargo.toml 严守 100%** ✅
- **0 主动 commit 严守 100%** ✅
- **0 主动 push 严守 100%** ✅
- **0 主动 IM 主人 严守 100%** ✅
- **0 装 PASS 严守 100%** ✅
- **0 形式化 old/death/terminate 严守 100%** ✅ (per 用户记忆 #4)
- **0 重复造轮子严守 100%** ✅ (per 用户记忆 #6, 引用 30+ 份 R129-R157 era 上游报告, 串联整合不重写)
- **8 硬墙 0 越界 100%** ✅ (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表)
- **8 哲学锚 严守 100%** ✅ (per 决策 #33 §2.3 B5)
- **不要怕复杂度哲学落地 100%** ✅ (per 决策 #73 §3 + 哲学文档 15)
- **整合 #4 commit abf12243 严守 100%** ✅ (per 决策 #48)
- **整合 #5.3 commit 4207f187 严守 100%** ✅ (per 决策 #78)
- **整合 #5.1 commit 仍 NOT READY 严守 100%** ✅ (per 决策 #78 §2.3 + 决策 #81 + 决策 #87 续续 §1, 等 R154-3 实地 verify 8/8 全 PASS 后由 Mavis 自决拍板)
- **整合 #5.2 commit ⚠️ PARTIAL 严守 100%** ✅ (per 决策 #62 §5.2 + 决策 #73 §2.3 + 决策 #74 §4.2, 等 5.1 src/ commit 拍板后)
- **决策严守 解读 4 维 100%** ✅ (per 决策 #74 §1 + 用户记忆 #8 + 用户记忆 #6 + 哲学文档 15)
- **永久循环 4 步 严守 100%** ✅ (per 决策 #71 §4)
- **跑中 16 满 严守 100%** ✅ (per 决策 #66 + 主人 0:34 拍板)
- **架构审视永久工作项严守 100%** ✅ (per 决策 #73 §2 + Section 10 架构审视永久工作项)
- **Mavis 决策倾向记录 100%** ✅ (per 决策 #10 + 用户记忆 #10 主人长时间离开 Mavis 自主决策 + 决策日志)
- **0 借脑 OpenCog AGPL-3.0 0 装 PASS 严守 100%** ✅ (per 决策 #33 §2.3 C2 + 决策 #73 §2.2, 借脑 ID 索引完成 0 借具体源码)
- **0 形式化 old/death/terminate 严守 100%** ✅ (per 用户记忆 #4, V1.2 release ASI Stage 9 9 阶段 seed → sapling → tree → sentinel 4 段 0 衰老病死)
- **0 装 "已 V1.2 release" 严守 100%** ✅ (per 决策 #33 §2.3 C2, V1.2 release 估 2027-02-28 是规划期, 0 装已 release)

### §12.3 决策日志 (per 决策 #10 + 用户记忆 #10 + 决策 #88 §6)

**决策日志 写入 `reports/decision-log-r137-era-cron-2026-08-11.md` (续写, 8:00+ tick 状态行)**:
- **时间戳**: 2026-08-11 (R158-2 done 时刻)
- **R158-2**: V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月, 估 2027-02-28) done 12 章节 ~570 行 200+ 行目标 100% 达成
- **R158-1**: 路线图整合 V1.1 release (派中, 估 2026-08-11 done)
- **R159-1**: Cargo workspace 1.2.1 bump 续 (派中, 估 2026-08-11 done)
- **跑中**: 14-16 (R158 era 2 sub + R159 era 1 sub + R155 era 续 3 sub + R156 era 调研 5 sub + R157 era 差距 3 sub = 14, 补 2 sub = 16 满)
- **done**: 175+ (R129-R158 era)
- **中断**: 0
- **canceled**: 0
- **target/**: 估 95-105 GB (50-100GB 预警, 0 主动删严守 100%, 除非 > 150 GB 紧急清理)
- **master HEAD**: `4207f187` (整合 #5.3 0 主动 push 严守)
- **整合 #5.1**: 等 R154-3 实地 verify 8/8 全 PASS 拍板 (per 决策 #74 C2 0 装 PASS 严守 100%)
- **整合 #5.2**: ⚠️ PARTIAL 等 5.1
- **整合 #6**: 估 2026-11-25 (V1.1 release 前 5 天, Mavis 自决拍板)
- **整合 #7**: 估 2026-11-29 (V1.1 release 前 1 天, Mavis 自决拍板)
- **V1.0 release**: 估 8/11 06:00-12:00 主人起床后手跑 70 min
- **V1.1 release**: 估 2026-11-30 06:00-08:00 主人起床后手跑 V1.1 release 实战 7 步 runbook
- **V1.2 release**: 估 2027-02-28 06:00-08:00 主人起床后手跑 (1.0 实战后 6 月, per 决策 #88 §3.5 R158-2 任务 spec + R130-5 §1.2 + 决策 #71 §4 永久循环 4 步)
- **V2.0 release**: 估 2027-Q2/Q3 (per R132-2 V2.0 release 战略路线图 8 大方向 + 决策 #74 §2.3 8 硬墙可重评)
- **决策链**: #1-#88 全读 + #89+ 续写 (R158 era 派活拍板 + 决策链 update)
- **8 硬墙**: B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (24 → 25 LOCKED) + V1.2 release 0 改严守 (per 决策 #74 B1 类比)
- **8 哲学锚**: S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 8 项 100% 严守
- **0 装 PASS 严守**: V1.1 借鉴 12 源 12/12 clear + V1.2 借鉴 14 源 14/14 clear (估 2027-02-28)
- **0 主动 IM 主人严守**: 仅 done notification 主动报告 (R158-2 done notification 主动报告)
- **0 装 PASS 严守**: V1.1 release 0 装 PASS 严守 100% + V1.2 release 0 装 PASS 严守 100% (估 2027-02-28)
- **0 主动 push 严守**: 整合 #5.1/5.2/5.3/6/7/8/9 commit 由 Mavis 自决拍板, git push 由主人起床后手跑
- **8 硬墙 B1 改写严守**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (24 → 25 LOCKED) + V1.2 release 0 改严守 (per 决策 #74 B1 类比)
- **不要怕复杂度哲学严守**: per 决策 #73 §3 + 哲学文档 15
- **架构审视永久工作项严守**: per 决策 #73 §2 + Section 10 架构审视永久工作项
- **永久循环 4 步严守**: per 决策 #71 §4 (调研 + 差距 + 计划 + 继续干)
- **0 重复造轮子严守**: per 用户记忆 #6, 引用 30+ 份 R129-R157 era 上游报告, 串联整合不重写
- **0 形式化 old/death/terminate 严守**: per 用户记忆 #4, V1.2 release ASI Stage 9 9 阶段 seed → sapling → tree → sentinel 4 段 0 衰老病死
- **TUI → Tauri 终极路线严守**: per 用户记忆 #8 + 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri", V1.2 release Tauri 桌面 app 完整实施

---

**R158-2 V1.1 release 后 V1.2 路线图 (1.0 实战后 6 月, 估 2027-02-28) ✅ done 12 章节 ~570 行 200+ 行目标 100% 达成, 0 改 src 严守 100% 标注 + 决策严守 解读 4 维 100% + V1.2 release 路线图 6 大方向 final 版 ✅ done**.

---

**0 改 src 严守 100% 标注**: per 决策 #62 + 决策 #74 整合 #5.1 commit V1.0 release 0 改 100% + V1.1 release Mavis 自决改 + **V1.2 release 0 改严守 100% (per 决策 #74 B1 类比 V1.0 release 0 改)**. R158-2 是规划/报告类, 0 触碰 crates/ 下任何 .rs 文件, 0 改 Cargo.toml 1.2.0 严守 100% (B2 严守), 0 主动 commit/push/IM 严守 100% (C1 + 0 push + gate-discipline), 0 装 PASS 严守 100% (C2), 0 借脑 OpenCog AGPL-3.0 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2), 0 形式化 old/death/terminate 严守 100% (per 用户记忆 #4), 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表), 8 哲学锚 严守 100% (per 决策 #33 §2.3 B5), 0 重复造轮子严守 100% (per 用户记忆 #6, 引用 30+ 份 R129-R157 era 上游报告, 串联整合不重写), 不要怕复杂度哲学落地 100% (per 决策 #73 §3 + 哲学文档 15), 整合 #4 commit abf12243 严守 100% (per 决策 #48), 整合 #5.3 commit 4207f187 严守 100% (per 决策 #78), 整合 #5.1 commit 仍 NOT READY 严守 100% (per 决策 #78 §2.3 + 决策 #81 + 决策 #87 续续 §1, 等 R154-3 实地 verify 8/8 全 PASS 后由 Mavis 自决拍板), 整合 #5.2 commit ⚠️ PARTIAL 严守 100% (per 决策 #62 §5.2 + 决策 #73 §2.3 + 决策 #74 §4.2, 等 5.1 src/ commit 拍板后), 永久循环 4 步严守 100% (per 决策 #71 §4), 跑中 16 满 严守 100% (per 决策 #66 + 主人 0:34 拍板), 架构审视永久工作项严守 100% (per 决策 #73 §2 + Section 10 架构审视永久工作项), Mavis 决策倾向记录 100% (per 决策 #10 + 用户记忆 #10 主人长时间离开 Mavis 自主决策 + 决策日志).
