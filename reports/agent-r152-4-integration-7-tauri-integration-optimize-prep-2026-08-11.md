# Agent R152-4 — 整合 #7 Tauri 集成优化准备 (实施 spec) (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #62 整合 #5 commit 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #86 §4 R152 era 实施 5 sub 派活拍板 + 决策 #78 整合 #5.3 done + R131-8 9 优化方向 + R130-3 Stage 5 集成深化 + R138-7 整合 #7 commit 拍板实战续)

**Date**: 2026-08-11 (R152 era 实施 spec 准备阶段, 60 min 时间盒, 严格不写代码, 0 改 src 严守 100%)
**Author**: Mavis sub-agent R152-4 (planning-only, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 主动 IM 主人)
**任务**: 整合 #7 Tauri 集成优化准备 (实施 spec) — Tauri 2.0 跟 Rust 后端集成 V1.1 release 优化实施 spec 详细 (5 nav + 9 organ 拟人化 + Web frontend 集成 + 实施 spec 8 维度 + 派活计划 + 风险 + 8 硬墙严守 verify)
**派活依据**: 决策 #86 §4 R152 era 实施 5 sub 派活拍板 (R152-1 + R152-2 + R152-3 + **R152-4 (本)** + R152-5) + 决策 #71 §2 永久循环接续 (R130 调研 → R131 差距 → R132 计划 → R133 实施 → R137-R148 续) + 决策 #78 整合 #5.3 reports/ commit 拍板 Option A (1:43 done, master HEAD = 4207f187) + 决策 #62 整合 #5 commit 3 commit 类比 + 决策 #74 §2.2 B1 V1.1 release Mavis 自决改
**关联 R131-8**: `reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md` (96 KB 9 优化方向 + V1.1/V2.0 完整方案, 1:20 done, R131 era 第 2 批)
**关联 R130-3**: `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md` (62.5 KB Stage 5 集成深化, 1:00 done)
**关联 R138-7**: `reports/agent-r138-7-integration-7-commit-paiban-xu-2026-08-11.md` (整合 #7 commit 拍板实战续, 02:00 done)
**关联 R137-TAURI 续**: per R138-6 §2.2 6.1 src/ 拍板准备 8 大方向 方向 5 Tauri Stage 5+ 续, 5 sub-agent (R137-TAURI-1~5) 派活 spec
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%, per 决策 #48)
**整合 #5.3 commit**: `4207f187100183170558d70633a970969aebdcda` (8/11 1:43 done, 187 files / 127548 insertions, master HEAD 严守 100%, 0 主动 push 严守, per 决策 #78 §2.2)
**整合 #5.1 commit**: ❌ NOT READY (per 决策 #78 §2.3 + 决策 #81 + R139-1-retry 续修 跑中, 0 改本报告)
**整合 #6 commit**: 估 2026-11-25 (V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5 + R138-6)
**整合 #7 commit**: 估 2026-11-29 (V1.1 release 前 1 天, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #62 整合 #5 commit 3 commit 类比 + R134-4 + R138-7, **本报告核心**) — Mavis 自决拍板
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0` 或 `v1.2.1`, per 决策 #74 §1 B2 workspace.version bump + R132-1 §1.1)
**报告路径**: `reports/agent-r152-4-integration-7-tauri-integration-optimize-prep-2026-08-11.md`
**目标大小**: 80-120 KB
**状态**: ✅ **R152-4 整合 #7 Tauri 集成优化准备 (实施 spec) done 2026-08-11 (60 min 时间盒): 8 维度实施 spec 详细 (实施步骤 + 接口 + 测试 + 风险 + 8 硬墙严守) + 5 关系 详写 (Rust 后端 / 5 nav / 9 organ / ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 8 哲学锚 / 用户记忆 #3) + 6 子方向 派活计划 (R152-4-1~6 估 6-12 周 实施) + 8 硬墙 0 越界 100% 严守 + 8 哲学锚 严守 100% + 0 装 PASS 严守 100% + 0 借脑 0 装 严守 100% + 0 主动 commit/push/IM 严守 100% + 0 重复造轮子严守 100% (R131-8 96 KB + R130-3 62.5 KB + R129-19 Stage 3 + R129-9 Stage 2 + R130-6 借鉴 12 源 + R133-1/2/3 + R137-1~5 + R138-6/7 + R151-2 整合 #7 commit 拍板时间表 + 哲学文档 15 reference 不重写). 0 改 src 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%, 0 借脑 0 装 严守 100%, 0 重复造轮子严守 100%**

---

## 0. 一句话 (TL;DR)

**R152-4 整合 #7 Tauri 集成优化准备 (实施 spec) (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #62 整合 #5 commit 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #86 §4 R152 era 派活拍板)**: 整合 #7 commit 拍板 估 2026-11-29 (V1.1 release 前 1 天, Mavis 自决拍板) + **8 维度 Tauri 集成优化 实施 spec 详细** (维度 1 Tauri 2.0 完整集成 + 维度 2 5 nav 完整 + 维度 3 9 organ 拟人化 final 1 屏多卡 + 维度 4 Stage 4-8 实战路线 + 维度 5 Tauri 跨平台 + 维度 6 Tauri 性能 + 维度 7 Tauri 借脑 + 维度 8 Tauri PHL-07 集成) + **5 关系 详写** (跟 Rust 后端 7 endpoint + 3 启动模式 / 5 nav / 9 organ / ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 8 哲学锚 / 用户记忆 #3 用户看结果) + **6 关系 + 8 维度 实施 spec 派活计划** (R152-4-1~6 估 6-12 周, ~6-12 sub-agent) + **8 硬墙 V1.1 release Mavis 自决改** (B1 24 LOCKED 入口签名 可改 + 0 改原 24 LOCKED + 仅扩 endpoint, per 决策 #74 §2.2 B1) + **8 哲学锚 严守 100%** (per 决策 #33 §2.3 B5) + **不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 最强效果 + 最厉害工程) + **0 装 PASS 严守 100%** (0 cargo install / 0 cargo add, 借脑 0 借具体源码) + **0 借脑 0 装 严守 100%** (per 决策 #33 §2.3 C2) + **0 主动 commit/push/IM 严守 100%** (per gate-discipline + 决策 #33 + 决策 #61 §6) + **0 重复造轮子严守 100%** (R131-8 96 KB + R130-3 62.5 KB + R129-19 + R129-9 + R130-6 + R133-1/2/3 + R137-1~5 + R138-6/7 + R151-2 + 哲学文档 15 reference 不重写) + **风险 6 维** + **决策原则 22 维** + **8 硬墙 0 越界 100% 严守** (B1 24 LOCKED 入口签名 0 改 + 0 改原 24 LOCKED + 仅扩 endpoint, per 决策 #74 §2.2 B1 V1.1 release Mavis 自决改).

---

## 1. 任务背景 + 上下文 (per 决策 #86 §4 + 决策 #71 §2 + 决策 #62 + 决策 #74 B1)

### 1.1 R152-4 任务定位 (per 决策 #86 §4 R152 era 实施 5 sub 派活拍板)

**R152 era 实施 spec 准备阶段 (per 决策 #86 §4 R152 era 派活拍板)**:
- **R152-1** 整合 #6 Cargo workspace 1.2.1 bump 准备 (实施 spec) — Cargo.toml workspace 1.2.0 → 1.2.1 bump 准备 spec
- **R152-2** 整合 #6 24 LOCKED 入口签名优化准备 (实施 spec) — 24 LOCKED 入口签名 8 方向 改写 spec
- **R152-3** 整合 #6 pybridge 集成优化准备 (实施 spec) — pybridge Stage 9 终极自治 集成 spec
- **R152-4** **整合 #7 Tauri 集成优化准备 (实施 spec)** — **本报告** — Tauri 2.0 跟 Rust 后端集成 V1.1 release 优化 实施 spec 详细
- **R152-5** 整合 #7 形式化集成优化准备 (实施 spec) — 形式化 Stage 5.5+ 实战 集成 spec

**R152-4 跟其他 R152 sub-agent 关系 (per 决策 #86 §4 + 用户记忆 #6 0 重复造轮子)**:
- ✅ R152-1 (Cargo workspace 1.2.1 bump 准备) **0 重叠**, Cargo workspace vs Tauri 集成
- ✅ R152-2 (24 LOCKED 入口签名 优化准备) **0 重叠**, 入口签名优化 vs Tauri 集成
- ✅ R152-3 (pybridge 集成优化准备) **0 重叠**, pybridge (Python ↔ Rust bridge) vs Tauri (Web ↔ Rust bridge)
- ✅ R152-5 (形式化集成优化准备) **0 重叠**, 形式化 (Kani 形式化证明) vs Tauri 集成
- ✅ R152-4 跟 R152-1/2/3/5 协同 (per 决策 #75 §1.5, 0 重复造轮子, 0 改 src 严守)

**R152-4 跟 R131-8 + R130-3 + R138-7 关系 (per 决策 #86 §4 + 用户记忆 #6 0 重复造轮子)**:
- ✅ R131-8 (96 KB 9 优化方向 + V1.1/V2.0 完整方案, 1:20 done) **0 重叠, R152-4 拓维**:
  - R131-8 §2 9 优化方向 (3 层架构 / 5 nav / 9 organ / Tauri Stage 5+ / servers / superpowers / 跨平台 / 性能 / V1.1 完整实施) **0 重写**
  - R131-8 §3 9 优化方向 × release 分层 矩阵 (V1.0/V1.1/V2.0 严守 严守 重评) **0 重写**
  - R131-8 §5 V1.1 release Tauri 完整实施 6 维度 470 min 蓝图 **0 重写**
  - **R152-4 拓维**: 把 R131-8 §5 6 维度 蓝图 展开成 **实施 spec 8 维度 详细** (接口 + 测试 + 风险 + 派活计划 + 8 硬墙严守 verify), 是 R131-8 蓝图的"实施 spec 准备"拓维
- ✅ R130-3 (62.5 KB Stage 5 集成深化, 1:00 done) **0 重叠, R152-4 reference**:
  - R130-3 §2 Stage 5 集成深化方案 (Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 砍 7 项 UI 哲学 100% + 后端全 API 表面同步) **reference 不重写**
  - R130-3 §3 Stage 6+ 路线 spec (Stage 6 后端 API 集成 + Stage 7 实际部署 + Stage 8 用户测试) **reference 不重写**
  - R130-3 §4 V1.1 minor release Tauri 计划 **reference 不重写**
  - **R152-4 拓维**: 跟 R130-3 §3 Stage 6+ 路线 spec + §4 V1.1 计划, 给出 R152-4 实施 spec 8 维度 详细
- ✅ R138-7 (整合 #7 commit 拍板实战续, 02:00 done) **0 重叠, R152-4 续**:
  - R138-7 §1.2 整合 #7 commit 拍板实战续 3 阶段 1 周 实施计划 **0 重写**
  - R138-7 §2 7.1 src/ 拍板 3 大方向 拓维 (Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+) **0 重写**
  - **R152-4 续**: R138-7 §2 7.1 src/ 拍板 Tauri Stage 5+ 续, R152-4 给出 R152-4-1~6 派活 spec
- ✅ R137-TAURI 续 (per R138-6 §2.2 6.1 src/ 拍板准备 8 大方向 方向 5) **0 重叠, R152-4 续**:
  - R137-TAURI-1~5 (5 sub-agent, 1 周) 派活 spec **0 重写**
  - **R152-4 拓维**: R137-TAURI-1~5 是 src/ 实施 sub-agent, R152-4 是 **实施 spec 准备 sub-agent** (0 改 src, 仅 spec), 角色不同

### 1.2 R152-4 任务边界 (per 决策 #33 + 决策 #60 + 决策 #71 §5 实施 spec 阶段 + 决策 #86 §4 R152-4 派活 + 决策 #74 B1 V1.1 release 0 改 src 严守)

**严格不写代码 (per 决策 #33 + 决策 #60 + 决策 #71 §5 实施 spec 阶段 + 决策 #86 §4 R152-4 派活 + 决策 #74 §2.2 B1 V1.0 release 0 改严守)**:
- ❌ 0 改 src/ (R152-4 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ❌ 0 改 Cargo.toml (B2 workspace.version 1.2.0 严守, V1.1 release 才 bump 1.2.1, 整合 #6 实施, 整合 #7 续)
- ❌ 0 改 docs/conventions/ (B1 24 LOCKED 入口签名 0 改, 整合 #6.1 commit 0 改, 整合 #7.1 commit 0 改)
- ❌ 0 改 frontend/tauri-prototype/ (V1.0 release 0 改 R11 baseline 严守, per 决策 #33 §2.3 B1 + 决策 #74 §2.2 B1 V1.1 release Mavis 自决改 仅扩 endpoint, 0 改原 24 LOCKED 入口签名)
- ❌ 0 借具体源码 (per 决策 #33 §2.3 C2, 实施 spec 准备是文档工作)
- ✅ 写新 reports 报告 `reports/agent-r152-4-integration-7-tauri-integration-optimize-prep-2026-08-11.md` (本报告, 80-120 KB)

**R152-4 输出物清单 (per 决策 #71 §5 实施 spec 阶段 + 决策 #86 §4 R152-4 派活)**:
1. ✅ 本报告 (R152-4 整合 #7 Tauri 集成优化准备 实施 spec, 60 min 时间盒, 80-120 KB)
2. ⏳ 整合 #7.1 commit 时, R152-4 报告作为 reports/ 部分加入 (per 决策 #62 §5.1 类比 + R138-7 §4.1 7.3 reports/ 拍板 ~10 文件 续)
3. ⏳ 整合 #7.2 commit 时, 写新 spec 文档 `docs/tauri-integration-optimize-2026-08-11.md` (per 决策 #74 §1, V1.1 release 实施 spec 阶段 — 整合 #7.2 commit 时 创建, 本报告 0 创建, 仅 spec 内容 reference)
4. ⏳ 整合 #7.3 commit 时, R152-4 报告 + R152 era 实施 续 sub-agent 报告 (R152-1/2/3/5 + 后续) 作为 reports/ 部分加入

### 1.3 R152-4 跟整合 #5/6/7 commit 拍板 0 冲突 (per 决策 #62 + 决策 #86 §4 + 决策 #75 §2.3)

**整合 #5 + #6 + #7 commit 拍板 0 冲突 (per 决策 #62 + 决策 #86 §4 + 决策 #75 §2.3)**:
- 整合 #5.3 reports/ commit 拍板 ✅ DONE (per 决策 #78 §2.2, 1:43, master HEAD = 4207f187, 187 files / 127548 insertions, 0 主动 push 严守)
- 整合 #5.1 src/ commit 拍板 ❌ NOT READY (per 决策 #78 §2.3 + 决策 #81 + R139-1-retry 续修 跑中, 0 改本报告)
- 整合 #6 commit 拍板 估 2026-11-25 (V1.1 release 前 5 天, Mavis 自决拍板, per R138-6 续)
- **整合 #7 commit 拍板 估 2026-11-29 (V1.1 release 前 1 天, Mavis 自决拍板, per 决策 #62 类比 + R134-4 + R138-7 + **本报告 R152-4**)**

**整合 #5 + #6 + #7 commit 拍板 顺序 (per 决策 #62 + 决策 #33 C1 + 决策 #71 §2.5 + 决策 #75 §2.3)**:
- 整合 #5 commit 拍板 → 主人起床后配 GitHub remote → V1.0 release tag v1.0.0 打上 → GitHub release + GitHub Pages
- V1.0 release 实战完 → R134 era 实施 (R134-1 ~ R134-6) → R137 era 5 sub 实施 (R137-1~5) → R138 era 13 sub 综合 (R138-1~13)
- R138-6 整合 #6 commit 拍板实战 (2026-11-25 估) → R138-7 整合 #7 commit 拍板实战续 (2026-11-29 估) → R152 era 实施 spec 准备 (R152-1~5, **本报告 R152-4**)
- 整合 #6 + #7 commit 拍板后 → 主人起床后配 GitHub remote V1.1 release push → V1.1 release tag v1.1.0 打上 → GitHub release + GitHub Pages 重新部署
- V1.1 release 实战完 → V1.2 minor release 准备 (per R131-3 永久循环 + 决策 #74 §2.3)

### 1.4 关键约束 (per 决策 #33 + #71 + #73 + #74 + 用户记忆 #1-#10 + gate-discipline)

**关键约束清单 (per 决策 #33 §2.3 + 决策 #71 §2 永久循环 + 决策 #73 §3 + 决策 #74 §1 + 用户记忆 #1-#10 + gate-discipline)**:
- ✅ **0 改 src 严守 100%** (per 决策 #33 §2.3 + 决策 #74 §2.2 B1 V1.0 release 0 改严守 + R152-4 任务 spec)
- ✅ **0 改 Cargo.toml 严守 100%** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1)
- ✅ **0 主动 commit 严守 100%** (per 决策 #33 §2.3 C1 + 决策 #74 §1, Mavis 自决拍板, 0 主动 commit since 1:43)
- ✅ **0 主动 push 严守 100%** (per 决策 #33 + 决策 #61 §6 + 决策 #78 §3, 等 1.0 release 配 GitHub remote + 主人起床后手跑)
- ✅ **0 主动 IM 主人 严守 100%** (per gate-discipline, 仅 done notification 主动报告)
- ✅ **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2, 0 cargo install / 0 cargo add, 借脑 0 装)
- ✅ **0 借脑 0 装 严守 100%** (per 决策 #33 §2.3 C2, 借脑 0 借具体源码, 0 装 "已读真源码" / 0 装 "已集成" / 0 装 "已 fork")
- ✅ **8 硬墙 0 越界 严守 100%** (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #74 B1 V1.1 release Mavis 自决改 仅扩 endpoint, 0 改原 24 LOCKED 入口签名)
- ✅ **8 哲学锚 严守 100%** (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项 UI 哲学)
- ✅ **0 重复造轮子 严守 100%** (per 用户记忆 #6, R131-8 96 KB + R130-3 62.5 KB + R129-19 + R129-9 + R130-6 + R133-1/2/3 + R137-1~5 + R138-6/7 + R151-2 + 哲学文档 15 reference 不重写)
- ✅ **9 organ 永远循环 0 死亡** (per 用户记忆 #4, ticker.js 100ms 周期, 活跃度 0-100 永远循环)
- ✅ **0 暴露 7 项 UI 哲学** (per 用户记忆 #3 砍 7 项: 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM)
- ✅ **5 nav 严守 0 改** (per 用户记忆 #3, 状态 / 主对话 / 历史 / 设置 / 工具结果)
- ✅ **不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 最强效果 + 最厉害工程, 维护交给未来高水平团队)
- ✅ **TUI 跟 Tauri 升级路径一致** (per 决策 #9 + 用户记忆 #8 + 用户记忆 #9 瘦客户端, TUI/Tauri 1:1 翻译, 后端 API 表面 0 改)
- ✅ **9 organ 1 屏多卡 拟人化** (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化, 3x3 网格 + ECG + NN + 健康环)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10, R152-4 报告本身 写入 reports/ + decision-log-r152-era-cron-2026-08-11.md)

### 1.5 R152-4 跟前置报告关系时间线 (per 决策 #86 §4 + 决策 #71 §2 + 决策 #78)

**R152-4 前置报告时间线 (per 决策 #86 §4 + 决策 #71 §2 永久循环 + 决策 #78 整合 #5.3 done + 决策 #62)**:

```
[8/10 19:41 整合 #4 commit]  abf12243 拍板 (per 决策 #48, R125 era)
   ↓
[8/10-8/11 R125-R128-2 era]  整合 41 sub-agent + 24 LOCKED + 11 借脑 + 借鉴 12 源
   ↓
[8/11 00:03 新会话接手]   mvs_367e66fae08342ffa399befe4f85dbac (per 决策 #61)
   ↓
[8/11 00:30 整合 #5 commit 拆 3 commit 拍板]   per 决策 #62
   ↓
[8/11 00:34-02:00 R129 era 5 批 35 sub + R130 era 6 sub + R131 era 9 sub + R132 era 2 sub + R133 era 3 sub + R134 era 6 sub + R135 era 2 sub + R136 era 1 sub + R137 era 5 sub + R138 era 13 sub + R139 era 1 sub + R140 era 14 sub + R141 era 3 sub + R142 era 1 sub + R143 era 2 sub + R144 era 4 sub + R145 era 3 sub + R146 era 3 sub + R147 era 5 sub + R148 era 6 sub]
   ↓
[8/11 1:43 整合 #5.3 reports/ commit 拍板]  4207f187, per 决策 #78 Option A
   ↓
[8/11 02:55 决策链 + 借鉴 + 8 硬墙 v3 索引]  R148-12 v3 索引
   ↓
[8/11 05:00 决策 #86 R152 era 5 sub 派活拍板]  16 满: R149 5 + R150 3 + R151 2 + **R152 5 (本 R152-4)** + R139-1-retry 1
   ↓
[8/11 05:00+ R152 era 实施 spec 准备阶段]  R152-1 ~ R152-5 派活 60 min 时间盒 跑中
   ↓
[R152-4 本报告 60 min 时间盒内 done]  整合 #7 Tauri 集成优化准备 (实施 spec 8 维度 详细)
   ↓
[R152 era 续]  R152-1/2/3/5 跑中 → done → Mavis 自决拍板 → 整合 #7.1 commit 时 R152-4 报告加入
   ↓
[8/12+ R153+ era 派活]  永久循环 (per 决策 #71 §2) 调研 + 差距 + 计划 + 实施 4 步 续
   ↓
[8/12 - 11/24 V1.1 release 实施 6 大方向]  30+ sub-agent (per R131-3 §2 + R132-1 §1.5 + R138-6 §2.1)
   ↓
[11/25 整合 #6 commit 拍板]  Mavis 自决 (per 决策 #74 B1 V1.1 release Mavis 自决改)
   ↓
[11/26-28 整合 #7 commit 拍板准备 5 阶段计划 续]
   ↓
[11/29 整合 #7 commit 拍板]  Mavis 自决 (per 决策 #62 整合 #5 commit 3 commit 类比)
   ↓
[11/30 V1.1 release 实战 7 步 runbook]  主人起床后手跑 7 步
   ↓
[V1.1 release tag v1.1.0 打上]  GitHub release + GitHub Pages 重新部署
   ↓
[V1.1 release 实战完]  V1.2 minor release 准备 (per R131-3 永久循环 + 决策 #74 §2.3)
```

---

## 2. Tauri 集成 V1.1 release 优化 实施 spec 详细 (per R131-8 §5 + R130-3 §2 §3 §4 + R138-7 §2 + 决策 #74 §2.2 B1)

### 2.1 实施 spec 总览 (per R131-8 §5 6 维度 470 min 蓝图 + 决策 #74 B1 V1.1 release Mavis 自决改)

**8 维度 Tauri 集成优化 实施 spec 总览 (per R131-8 §5 6 维度 470 min 蓝图 + R130-3 §2-§4 + R138-7 §2 + 决策 #74 §2.2 B1)**:

| 维度 | 实施 spec | 估时 | 派活 (R152-4-N) | 决策依据 | 8 硬墙严守 |
|:---:|----------|-----:|----------------|---------|-----------|
| **1** | **Tauri 2.0 完整集成** (tauri 2.11+ 跨平台打包 + tauri-build 2.6.3 + 8 Tauri 2.0 permissions + 5 icons + 5 nav 窗口 + capabilities/default.json + WebView 平台差异) | 60 min | **R152-4-1** | R130-3 §2.5 + R131-8 §2.7 + 用户记忆 #8 | 🟢 8 硬墙 0 越界 (Tauri 0 触碰 8 硬墙, 仅扩 endpoint, 0 改原 24 LOCKED 入口签名) |
| **2** | **5 nav 完整集成** (状态/主对话/历史/设置/工具结果, 1:1 镜像 TUI, CrossNavStore 状态中枢 1 真相源, J1-J7 7 模块, 集成层 79 + 84 = 163 tests) | 90 min | **R152-4-2** | R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3 砍 7 项 UI 哲学 + R129-19 Stage 3 79 tests | 🟢 8 硬墙 0 越界 (5 nav 0 改 严守, per 用户记忆 #3) |
| **3** | **9 organ 拟人化 final 1 屏多卡** (heart ECG + brain NN + 9 健康环 + 永远循环 ticker 100ms 周期, 1 真相源 5 nav 共享, Stage 4 D 真 sensor 接入) | 120 min | **R152-4-3** | R130-3 §2.4 + R131-8 §2.3 + 用户记忆 #4 0 死亡 + 用户记忆 #5 信息密度高 + R129-9 Stage 2 122 tests | 🟢 8 硬墙 0 越界 (9 organ 永远循环 0 死亡, per 用户记忆 #4) |
| **4** | **Stage 4-8 实战路线** (Stage 4 实战 4 维度 A 真后端/B WebSocket/C 持久化/D 真 sensor + Stage 5 集成深化 + Stage 6 后端接通 7 endpoint + Stage 7 跨平台部署 + Stage 8 用户测试) | 120 min | **R152-4-4** | R130-3 §3 + R131-8 §2.4 + 决策 #9 TUI 升级路径一致 | 🟢 8 硬墙 0 越界 (Stage 4 4 维度 蓝图就绪, per R129-31 §2) |
| **5** | **Tauri 跨平台 (Windows/macOS/Linux)** (MSI/NSIS/DMG/APP/deb/AppImage 5 bundle format + Tauri 2.0 updater 自动更新 V1.0.0 → V1.0.1 → V1.1.0) | 90 min | **R152-4-5** | R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2 0 装 PASS 严守 | 🟢 8 硬墙 0 越界 (跨平台 蓝图就绪, per R130-3 §2.5) |
| **6** | **Tauri 性能** (WebSocket 流式 浏览器 native + 9 organ 真 sensor 后端 Rust crate 真实施 + 跨 tab 持久化 localStorage + BroadcastChannel) | 90 min | **R152-4-6** | R130-3 §4 + R131-8 §2.8 + 决策 #33 §2.3 C2 0 装 + 决策 #73 §3 不要怕复杂度 | 🟢 8 硬墙 0 越界 (性能 0 瓶颈 0 装, per R131-8 §2.8) |
| **7** | **Tauri 借脑 (5 借脑 0 装)** (Tauri 2.0 真实施 + superpowers 234 5 DialoguePhase 1:1 翻译 + langgraph 829 stream_state_events 1:1 翻译 + servers 1.4MB MCP server 设计模式 1:1 翻译 + kani 5.5MB 0 引 crate 依赖) | 60 min | **(R152-4-3 + R152-4-6 协同)** | R130-3 §5 + R131-8 §2.5-§2.6 + 决策 #33 §2.3 C2 0 借脑 0 装 | 🟢 8 硬墙 0 越界 (借脑 0 借具体源码, per 决策 #33 §2.3 C2) |
| **8** | **Tauri PHL-07 主对话锚集成** (PHL-07 14 维主对话锚 1:1 跟 9 organ 集成, V1.0 spec-only → V1.1 实施, 14 维 5 阶段 8 周 实施计划) | 90 min | **(R152-4-3 协同)** | R130-5 §2.1 + R131-3 §2.1 + 决策 #22 §1.1-1.2 + 决策 #74 A3 改写 | 🟢 8 硬墙 0 越界 (PHL-07 V1.1 实施 14 维, per 决策 #74 §1 A3) |
| **总** | **8 维度 实施 spec** | **~620 min (估 6-12 周 派活 R152-4-1~6)** | **6 sub-agent 派活** | R131-8 §5 6 维度 蓝图 + R130-3 §2-§4 + R138-7 §2 | ✅ 0 越界 100% |

**实施 spec 关系 (per 决策 #74 §2.2 B1 + 决策 #33 + 用户记忆 #3-#10)**:
- ✅ **维度 1 + 5**: Tauri 2.0 + 跨平台 (基础架构)
- ✅ **维度 2**: 5 nav (跟 TUI 1:1 镜像, 跟用户记忆 #3 砍 7 项 UI 哲学 严守)
- ✅ **维度 3 + 7 + 8**: 9 organ + 借脑 + PHL-07 (Stage 4 D 实战 + 9 organ 永远循环 + PHL-07 14 维)
- ✅ **维度 4 + 6**: Stage 4-8 路线 + 性能 (蓝图就绪 + 0 装 + 不要怕复杂度)
- ✅ **6 子方向 派活**: R152-4-1 ~ R152-4-6, 估 6-12 周 实施 (跟 V1.1 release 2026-11-30 留 8-12 周 buffer)
- ✅ **8 硬墙 0 越界 100%**: per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #74 B1 V1.1 release Mavis 自决改 仅扩 endpoint, 0 改原 24 LOCKED 入口签名

### 2.2 维度 1: Tauri 2.0 完整集成 实施 spec (per R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2)

**维度 1: Tauri 2.0 完整集成 实施 spec (per R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2 0 装 PASS 严守)**:

**维度 1.1 Tauri 2.0 集成状态盘点 (per P11-1 + P11-2 + R129-9 + R129-19 baseline)**:
- ✅ Tauri = "=2.11.5" (per `frontend/tauri-prototype/src-tauri/Cargo.toml:34`)
- ✅ tauri-build = "=2.6.3" (per `frontend/tauri-prototype/src-tauri/Cargo.toml:25`)
- ✅ 27 commands 拆 9 submod (per P11-2 §3.5): nav (2) + organ (4) + dialogue (4) + stream (3) + tools (3) + settings (2) + history (1) + app_state (3) + visualization (5)
- ✅ 5 nav 窗口 + 5 icons + bundle.targets = "all" (per `tauri.conf.json:32`)
- ✅ 8 Tauri 2.0 permissions (per `capabilities/default.json:7-14`): core:default + core:window:default + core:window:allow-minimize + core:window:allow-toggle-maximize + core:window:allow-close + core:webview:default + core:event:default + core:app:default
- ✅ 1 窗口 (per `tauri.conf.json:10-23`): label = main, title = "Apeireth — 终极前端 prototype", 1280x800, min 1024x720
- ✅ cargo build PASS 12.8 MB (per P11-2 §3.3)
- ✅ cargo tauri dev 跑通 (per P11-2 §3.4, binary PID 37136, CPU 0.09, RAM 28 MB)
- ✅ cargo test PASS 122 tests (per R129-9 §8.1, 0.01s 跑完)
- ✅ 集成层 test PASS 79 cases (per R129-19 §9.3, node run-all.js 跑通)
- ✅ core lib 0 Tauri 依赖 (per P11-1 §2, 122 tests pass 0.01s, 纯逻辑 1:1 镜像 TUI)
- ✅ 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)

**维度 1.2 Tauri 2.0 V1.1 release 集成 7 子方向 (per R131-8 §2.7 + R130-3 §2.5)**:
- 子方向 1.2.1: **多窗口支持** (per Stage 5 蓝图 + R130-3 §2.5): 主窗口 + 工具结果窗口 + 设置窗口, 1 窗口 → 3 窗口, tauri.conf.json windows [] 加 2 窗口
- 子方向 1.2.2: **Tauri 2.0 IPC 强化** (per R131-8 §2.7 + Stage 4 A 实战): cross-window emit + 9 organ 实时推送 + 27 commands 拆 9 submod 续 36+ commands
- 子方向 1.2.3: **Tauri 2.0 event system** (per R131-8 §2.7): event 双向通信 (Rust → JS + JS → Rust), 9 organ ticker emit, CrossNavStore subscribe
- 子方向 1.2.4: **Tauri 2.0 capabilities 升级** (per R130-3 §2.5): 加 5 nav 窗口对应 capabilities (status.json / chat.json / history.json / settings.json / tools.json), 8 permissions → 5×4 = 20 permissions
- 子方向 1.2.5: **Tauri 2.0 tray icon** (per R130-3 §2.5 + 用户记忆 #5 信息密度高): 系统托盘 + 9 organ 缩略图 + 0 假装
- 子方向 1.2.6: **Tauri 2.0 menu** (per R130-3 §2.5 + 用户记忆 #3 砍 7 项): 菜单栏 + 5 nav 快捷键 + 0 暴露 UI 哲学
- 子方向 1.2.7: **Tauri 2.0 updater 自动更新** (per R131-8 §2.7 + V1.1 release 后): V1.0.0 → V1.0.1 → V1.1.0 自动推送, 跨平台差异, 0 装 Tauri 2.0 native

**维度 1.3 Tauri 2.0 实施 spec 接口 + 测试 (per R131-8 §2.7 + 决策 #33 §2.3)**:
- 接口 1.3.1: tauri.conf.json windows [] 加 2 窗口, capabilities/ 加 status.json / chat.json / history.json / settings.json / tools.json
- 接口 1.3.2: src-tauri/src/commands/ 加 stage5_*.rs 5 子方向 (multi_window.rs + ipc.rs + event_system.rs + tray.rs + menu.rs + updater.rs = 6 NEW)
- 接口 1.3.3: src-tauri/src/lib.rs 注册 6 NEW commands, 总 27 → 36 commands
- 接口 1.3.4: frontend/src/integration/store.js 0 改 (CrossNavStore 1 真相源, 0 加新 EVT 仅 add subscribe)
- 测试 1.3.5: cargo test 6 NEW commands × 3 cases = 18 NEW tests
- 测试 1.3.6: 集成层 stage5-integration.test.js 6 模块 × 5 cases = 30 NEW tests
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装新 lib, 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 设计模式)

### 2.3 维度 2: 5 nav 完整集成 实施 spec (per R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3 + R129-19 Stage 3 baseline)

**维度 2: 5 nav 完整集成 实施 spec (per R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3 砍 7 项 UI 哲学 + R129-19 Stage 3 79 tests)**:

**维度 2.1 5 nav 集成状态盘点 (per P11-1 + P11-2 + R129-19 Stage 3 baseline)**:
- ✅ 5 nav = NAV_ID 0-4 严守 (per 用户记忆 #3, 状态 / 主对话 / 历史 / 设置 / 工具结果)
- ✅ CrossNavStore 状态中枢 (per `frontend/src/integration/store.js:1-10KB`, 14 EVT + 12 mutators + 5 nav 状态 + 9 organ 活动)
- ✅ 集成层 7 模块 J1-J7 (per R129-19 §2.1): status_chat.js / status_history.js / status_tools.js / chat_history.js / chat_tools.js / history_tools.js / settings_global.js
- ✅ 集成层 79 tests + 8 examples + 1 hub (per R129-19 §9.3, 全部 pass)
- ✅ 0 暴露 UI 哲学 100% (per 用户记忆 #3 砍 7 项: 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM)
- ✅ 0 改 5 nav (严守, 0 加 0 砍 0 改 NAV_ID 0-4)

**维度 2.2 5 nav V1.1 release 完整集成 6 子方向 (per R130-3 §2.3 + R131-8 §2.2 + 决策 #74 §2.2 B1)**:
- 子方向 2.2.1: **状态 nav 真打通** (per R130-3 §2.3 状态): 9 organ final 1 屏多卡 (3x3 网格 + ECG + NN + 健康环), Stage 4 D 真 sensor 接入, 跟 apeireth-api `GET /v1/organs` 真接通
- 子方向 2.2.2: **主对话 nav 真打通** (per R130-3 §2.3 主对话): 5 阶段 DialoguePhase 1:1 跟 superpowers 234 executing-plans 翻译, 4 ThinkingPhase 1:1 跟 PHL-07 14 维主对话锚, Stage 4 B WebSocket 流式 真接通
- 子方向 2.2.3: **历史 nav 真打通** (per R130-3 §2.3 历史): 3 kind (会话/消息/工具调用) + SVG 时间线 (timeline.js) + 按 episode 过滤, 跟 apeireth-api `GET /v1/history` 真接通
- 子方向 2.2.4: **设置 nav 真打通** (per R130-3 §2.3 设置): 14 settings (5+5+4, 5 鉴权 + 5 Provider + 4 SDK) 真接通, sub-control 编辑 + 鉴权 UI + settings-editor.js
- 子方向 2.2.5: **工具结果 nav 真打通** (per R130-3 §2.3 工具结果): 6 工具 endpoint (日历/消息/联系人/任务/搜索/云盘) + tool_call deep-link chat + 颜色编码 + 弹窗, 跟 apeireth-api `GET /v1/tools/results` 真接通
- 子方向 2.2.6: **5 nav 1 真相源** (per R129-19 §1.3 + 决策 #74 B1): CrossNavStore 1 真相源, 5 nav 共享, WebSocket 推送实时更新, 0 装 socket.io

**维度 2.3 5 nav 实施 spec 接口 + 测试 (per R131-8 §2.2 + 决策 #74 B1)**:
- 接口 2.3.1: frontend/src/integration/store.js 0 改 (1 真相源, 0 加新 EVT), 仅 add 5 nav 真接通 subscribe
- 接口 2.3.2: frontend/src/integration/ 7 模块 J1-J7 0 改, 仅 add tauriInvoke 调 7 endpoint 真接通
- 接口 2.3.3: src-tauri/src/commands/ 加 nav_v1_1.rs (5 commands: status / dialogue / history / settings / tools 真接通), 总 27 → 32 commands
- 接口 2.3.4: src-tauri/src/lib.rs 注册 5 NEW commands + 加 stage5_*.rs 续
- 接口 2.3.5: frontend/src/app.js (37.1 KB, P11-2 baseline) 0 改 5 nav 路由, 仅 add tauriInvoke 调 5 真接通
- 测试 2.3.6: cargo test 5 NEW commands × 5 cases = 25 NEW tests
- 测试 2.3.7: 集成层 7 模块 J1-J7 0 改, 仅 add 5 真接通 cases × 7 模块 = 35 NEW tests, 集成层累计 79 + 35 = 114 tests
- 测试 2.3.8: Stage 4 A 真后端接通 6 模块 × 5 cases = 30 NEW tests (per R129-31 §2.2)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装新 lib, 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 设计模式)

### 2.4 维度 3: 9 organ 拟人化 final 1 屏多卡 实施 spec (per R130-3 §2.4 + R131-8 §2.3 + 用户记忆 #4-#5 + R129-9 Stage 2 + R129-19 Stage 3)

**维度 3: 9 organ 拟人化 final 1 屏多卡 实施 spec (per R130-3 §2.4 + R131-8 §2.3 + 用户记忆 #4 0 死亡 + 用户记忆 #5 信息密度高 + R129-9 Stage 2 + R129-19 Stage 3)**:

**维度 3.1 9 organ 拟人化状态盘点 (per P11-1 + P11-2 + R129-9 + R129-19 baseline)**:
- ✅ 9 organ = ORGAN_ID 0-8 严守 (per 用户记忆 #4, heart / brain / hand / eye / ear / memory / voice / body / mind)
- ✅ 9 健康环 (per R129-9 §3.2, 1 屏 9 个 SVG circle, radius 30, stroke-width 6, 颜色 0-30 红/30-70 黄/70-100 绿)
- ✅ heart ECG (per R129-9 §3.3, P-QRS-T 三段, 60 采样/周期, 走纸动画, 红色)
- ✅ brain NN (per R129-9 §3.4, 9 节点 + 8 中心边 + 8 围圈边, hover 放大, 紫色)
- ✅ organ_animator.js (per R129-19 §2.1, 9 KB, 5 helper: renderChatHeaderOrgans / renderToolsHeaderOrgan / renderHistoryHeaderOrgans / renderSettingsHeaderOrgan / getOrganHealthSummary)
- ✅ ticker.js (per R129-9 §3.5, 100ms 周期, 永远循环, 0 死亡, activity_pct 0-100)
- ✅ 永远循环 0 死亡 (per 用户记忆 #4, 0 显示 "已死亡/老化/终止", 用 "活跃度" active/idle/dormant 0 用 "健康度" healthy/sick)
- ✅ 1 真相源 CrossNavStore (per R129-19 §1.3, organ_activities 9 organ 1 真相源, 5 nav 共享)
- ✅ 0 暴露内部机制 100% (per 用户记忆 #3 砍 7 项)

**维度 3.2 9 organ V1.1 release 拟人化 final 6 子方向 (per R130-3 §2.4 + R131-8 §2.3 + R129-31 §2.5 Stage 4 D 实战)**:
- 子方向 3.2.1: **heart 真 ECG 真 sensor 接入** (per R130-3 §2.4 + R129-31 §2.5 D1): ECG 60 采样/周期 + 实时 BPM, 跟后端 organ.rs heart 真接通, 0 装 sensor 硬件驱动
- 子方向 3.2.2: **brain 真神经网络 真 sensor 接入** (per R130-3 §2.4 + R129-31 §2.5 D2): 9 节点 + 8 中心边 + 8 围圈边, 跟后端 organ.rs brain 真接通, 0 装 visx
- 子方向 3.2.3: **hand 真待办工具数 真 sensor 接入** (per R130-3 §2.4 + R129-31 §2.5 D3): 待办工具数 + 成功率 + 0 假装, 跟后端 organ.rs hand 真接通
- 子方向 3.2.4: **eye/ear/memory/voice/body/mind 真 sensor 接入** (per R130-3 §2.4 + R129-31 §2.5 D4-D9): eye history 新条目数 + 观察频率 / ear chat 输入频率 / memory history 过滤数 / voice stream chunk/s / body 系统 uptime / mind thinking 阶段
- 子方向 3.2.5: **9 organ 永远循环 ticker 1 真相源 5 nav 共享** (per R129-19 §1.3 + 用户记忆 #4): CrossNavStore.organ_activities 9 organ 1 真相源, 5 nav 共享, ticker 100ms 周期, 永远循环
- 子方向 3.2.6: **PHL-07 14 维主对话锚 1:1 跟 9 organ 集成** (per 决策 #22 §1.1-1.2 + 决策 #74 §2.2 V1.1 release 实施): 14 维主对话锚 跟 9 organ 集成 (心/脑/手/眼/耳/记忆/声/体/意 + 5 维主对话深化)

**维度 3.3 9 organ 实施 spec 接口 + 测试 (per R131-8 §2.3 + 决策 #74 B1)**:
- 接口 3.3.1: core/src/organ.rs 1:1 镜像 TUI organ/mod.rs 0 改 (per R129-9 实施), Stage 4 D 实战 14 NEW tests
- 接口 3.3.2: src-tauri/src/commands/organ.rs 加 5 NEW commands (heart_ecg_live / brain_nn_live / hand_todo / eye_history / ear_chat), 总 27 → 32 commands (or 32+5 = 37)
- 接口 3.3.3: frontend/src/visualizations.js 0 改 (per R129-9 §3 实施, vanilla SVG)
- 接口 3.3.4: frontend/src/integration/organ_animator.js 0 改 (per R129-19 §2.1), 仅 add tauriInvoke 调 5 NEW 真接通
- 接口 3.3.5: frontend/src/integration/store.js 0 改, 仅 add 5 organ 真接通 subscribe
- 测试 3.3.6: cargo test 5 NEW commands × 5 cases = 25 NEW tests
- 测试 3.3.7: 集成层 organ-animator.test.js add 5 organ 真接通 cases × 5 = 25 NEW tests
- 测试 3.3.8: Stage 4 D 9 organ 真 sensor 接入 9 + 1 统一 = 14 NEW tests (per R129-31 §2.5)
- 9 organ 永远循环 0 死亡 严守 100% (per 用户记忆 #4, ticker.js 100ms 周期, activity_pct 0-100)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装 D3 / visx / eCharts, 用 vanilla SVG)

### 2.5 维度 4: Stage 4-8 实战路线 实施 spec (per R130-3 §3 + R131-8 §2.4 + R129-31 §2 蓝图)

**维度 4: Stage 4-8 实战路线 实施 spec (per R130-3 §3 + R131-8 §2.4 + R129-31 §2 4 维度蓝图)**:

**维度 4.1 Stage 4 实战 4 维度 (per R129-31 §2 蓝图 + R131-8 §2.4)**:
- 维度 A: **真后端接通** (per R129-31 §2.2): tauriInvoke 主路径, mock 仅 dev mode fallback, CrossNavStore 7 模块 + 9 organ animator 调 tauriInvoke, 6 模块 × 5 cases = 30 NEW tests
- 维度 B: **WebSocket 流式** (per R129-31 §2.3): 流式打字 (R129-9 字符级 50ms/字) → 真 WebSocket chunk append (browser native, 0 装), 4 模块 × 5 cases = 20 NEW tests
- 维度 C: **跨 tab 持久化** (per R129-31 §2.4): settings/theme/font/layout 跨 tab 同步 (localStorage + BroadcastChannel, browser native), 4 模块 × 5 cases = 20 NEW tests
- 维度 D: **9 organ 真 sensor** (per R129-31 §2.5): 9 organ 真状态接入 (heart ECG / brain NN / hand 待办 / eye 观察 / ear 聆听 / memory 沉淀 / voice 流速 / body uptime / mind 思考), 9 + 1 统一 = 14 NEW tests
- **Stage 4 总**: 84 NEW tests 累计 163 tests

**维度 4.2 Stage 5 集成深化 4 子方向 (per R130-3 §2.8 蓝图 + R131-8 §2.4)**:
- 子方向 4.2.1: **Tauri 2.0 完整集成** (per R130-3 §2.5 + 维度 1): 27 → 36+ commands, 跨平台打包, 0 装新 framework
- 子方向 4.2.2: **5 nav 完整集成** (per R130-3 §2.3 + 维度 2): CrossNavStore + 7 模块 + tauriInvoke 主路径, 集成层累计 163 tests
- 子方向 4.2.3: **9 organ final 1 屏多卡** (per R130-3 §2.4 + 维度 3): heart ECG + brain NN + 9 健康环 + 永远循环 ticker, 1 真相源 5 nav 共享
- 子方向 4.2.4: **砍 7 项 UI 哲学 100%** (per 用户记忆 #3 + 决策 #33 §2.3 B5): CrossNavStore 0 emit 守门事件, 0 暴露内部机制, 0 显示衰老病死, 永远循环 0 死亡

**维度 4.3 Stage 6 后端接通 7 endpoint (per R130-3 §3.1 蓝图 + R131-8 §2.4)**:
- 6.1 `GET /v1/organs` → 9 organ + activities (状态 nav 真接通)
- 6.2 `POST /v1/chat/messages` → user 消息 + AI 回复 (主对话 nav 真接通)
- 6.3 `GET /v1/chat/session/{id}` → 5 DialoguePhase (主对话 nav 真接通)
- 6.4 `GET /v1/history` → history entries (历史 nav 真接通)
- 6.5 `GET /v1/tools/results` → 6 tool results (工具结果 nav 真接通)
- 6.6 `GET /v1/settings` → 14 settings (设置 nav 真接通)
- 6.7 `PATCH /v1/settings/{key}` → 改 1 setting (设置 nav 真接通)
- 6.8 `WS /v1/chat/stream` → stream chunks (主对话 nav WebSocket 流式 真接通)
- **Stage 6 总**: 8 endpoint 真接通, 估 30 NEW tests

**维度 4.4 Stage 7 跨平台部署 (per R130-3 §3.2 蓝图 + R131-8 §2.7 + 维度 5)**:
- 7.1 `cargo tauri build` Windows (MSI/NSIS) + macOS (DMG/APP) + Linux (deb/AppImage) = 5 bundle format
- 7.2 1.0 release tag v1.0.0 打上 (per R129-35 final-final 7 步 runbook, 主人起床后手跑)
- 7.3 GitHub release 创建 v1.0.0 (per R129-35 续, 主人手跑 GitHub UI)
- 7.4 Tauri 2.0 updater 自动更新 V1.0.0 → V1.0.1 → V1.1.0 (per R131-8 §2.7 子方向 1.2.7)
- 7.5 跨平台打包 CI (GitHub Actions, 0 装, Tauri 2.0 官方支持)
- **Stage 7 总**: 75 min, 0 装新 framework

**维度 4.5 Stage 8 用户测试 (per R130-3 §3.3 蓝图 + R131-8 §2.4)**:
- 8.1 主人手跑 (per R129-35 final-final 7 步 runbook, 1.0 release 实战 8 步)
- 8.2 真用户验收 (per ROADMAP.md §4 + 主人 8/4 23:33)
- 8.3 反馈 + V1.0.1 patch + V1.1 规划 (per R130-3 §3.3 续)
- **Stage 8 总**: 180 min + 7 天 主人手跑, 蓝图就绪

**维度 4.6 Stage 4-8 实施 spec 接口 + 测试 (per R131-8 §2.4 + 决策 #74 B1)**:
- 接口 4.6.1: src-tauri/src/commands/ 加 stage4-8 续, 总 27 → 32+ commands
- 接口 4.6.2: src-tauri/src/lib.rs 注册 5-10 NEW commands
- 接口 4.6.3: frontend/src/integration/ 7 模块 J1-J7 add tauriInvoke 调 8 endpoint
- 接口 4.6.4: frontend/src/app.js (37.1 KB, P11-2) 0 改 5 nav 路由, 仅 add tauriInvoke
- 接口 4.6.5: frontend/src/integration/store.js 0 改, 仅 add 5 nav + 9 organ + 14 settings 真接通 subscribe
- 测试 4.6.6: Stage 4 4 维度 84 NEW tests 累计 163
- 测试 4.6.7: Stage 5 集成深化 30 NEW tests 累计 193
- 测试 4.6.8: Stage 6 后端接通 30 NEW tests 累计 223
- 测试 4.6.9: Stage 7 跨平台部署 cargo tauri build 3 平台 PASS
- 测试 4.6.10: Stage 8 用户测试 真用户验收 PASS
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2)

### 2.6 维度 5: Tauri 跨平台 实施 spec (per R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2)

**维度 5: Tauri 跨平台 实施 spec (per R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2 0 装 PASS 严守)**:

**维度 5.1 Tauri 2.0 跨平台 native 支持盘点 (per P11-1 §7.1 + R130-3 §2.5 baseline)**:
- ✅ Tauri 2.0 跨平台 native 支持 (per 决策 #33 §2.3 C2 + P11-1 §7.1 Tauri 2.0 项目结构)
- ✅ bundle.targets = "all" (per `tauri.conf.json:32`): Windows + macOS + Linux 全部 native
- ✅ 5 icons (per `tauri.conf.json:33-39` + `icons/`): 32x32.png / 128x128.png / 128x128@2x.png / icon.icns (macOS) / icon.ico (Windows)
- ✅ WebView 平台差异 (per R130-3 §2.5): WebView2 (Windows) / WKWebView (macOS) / WebKitGTK (Linux)
- ❌ 0 跨平台打包实战 (per R130-3 §2.5 Stage 5 + Stage 7 蓝图, 1.0 release 实战)

**维度 5.2 Tauri 2.0 跨平台打包清单 5 bundle format (per R130-3 §2.5 Stage 5 实施)**:
- Windows: **MSI** (per Tauri 2.0 bundler 官方支持) + **NSIS** (per Tauri 2.0 bundler 官方支持)
- macOS: **DMG** (per Tauri 2.0 bundler 官方支持) + **APP** (per Tauri 2.0 bundler 官方支持)
- Linux: **deb** (per Tauri 2.0 bundler 官方支持) + **AppImage** (per Tauri 2.0 bundler 官方支持)
- 跨平台 `cargo tauri build`: 1 条命令 3 平台打包
- 自动更新 (Tauri 2.0 updater): V1.0.0 → V1.0.1 → V1.1.0 自动推送

**维度 5.3 Tauri 跨平台 6 子方向 (per R130-3 §2.5 + R131-8 §2.7 + 决策 #33 §2.3 C2)**:
- 子方向 5.3.1: **5 icons 真实生成** (per R130-3 §2.5 + P12-1 阶段 1 替换 placeholder): icons/ 5 PNG 真生成, 0 装新 lib
- 子方向 5.3.2: **cargo tauri build 3 平台** (per R130-3 §2.5): Windows + macOS + Linux 3 平台 build, 估 30 min
- 子方向 5.3.3: **5 bundle format 实战** (per R130-3 §2.5): MSI / NSIS / DMG / APP / deb / AppImage, 估 45 min
- 子方向 5.3.4: **Tauri 2.0 updater 自动更新** (per R131-8 §2.7 + R130-3 §2.5): V1.0.0 → V1.0.1 → V1.1.0, 跨平台差异, 估 60 min
- 子方向 5.3.5: **跨平台打包 CI** (per R130-3 §2.5 + 主人 8/11 0:43): GitHub Actions 0 装, Tauri 2.0 官方支持, 估 30 min
- 子方向 5.3.6: **跨平台 verify** (per R130-3 §2.5 + 决策 #33 §2.3 C2): cargo tauri build 3 平台 PASS + cargo test 0 越界 verify

**维度 5.4 Tauri 跨平台 实施 spec 接口 + 测试 (per R131-8 §2.7 + 决策 #33 §2.3 C2)**:
- 接口 5.4.1: `tauri.conf.json` bundle.targets "all" 0 改 (per baseline, 5 bundle format 自动)
- 接口 5.4.2: `src-tauri/icons/` 5 icons 真生成 (per 子方向 5.3.1)
- 接口 5.4.3: `.github/workflows/tauri-build.yml` 加 GitHub Actions 跨平台 build (per 子方向 5.3.5)
- 接口 5.4.4: `src-tauri/tauri.conf.json` plugins/updater 加 Tauri 2.0 updater 配置 (per 子方向 5.3.4)
- 接口 5.4.5: `src-tauri/Cargo.toml` dependencies 加 tauri-plugin-updater = "2" (Tauri 2.0 官方 plugin, 0 装 PASS 严守)
- 测试 5.4.6: cargo tauri build 3 平台 (Windows + macOS + Linux) 5 bundle format = 15 binary, 0 warning 0 error
- 测试 5.4.7: cargo test 0 越界 (0 触碰主仓 24 LOCKED 入口签名)
- 测试 5.4.8: Tauri 2.0 updater 跨平台差异 verify (Windows MSI + macOS DMG + Linux deb 各自动更新 verify)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装新 lib, 仅 Tauri 2.0 native)

### 2.7 维度 6: Tauri 性能 实施 spec (per R130-3 §4 + R131-8 §2.8 + 决策 #33 §2.3 C2)

**维度 6: Tauri 性能 实施 spec (per R130-3 §4 + R131-8 §2.8 + 决策 #33 §2.3 C2 0 装 + 决策 #73 §3 不要怕复杂度)**:

**维度 6.1 Tauri 性能基线盘点 (per P11-2 + R129-9 + R129-19 verify)**:
- ✅ cargo build PASS 12.8 MB + pdb 112 MB (per P11-2 §3.3)
- ✅ cargo tauri dev 跑通 binary PID 37136, CPU 0.09, RAM 28 MB (per P11-2 §3.4)
- ✅ cargo test PASS 122 tests 0.01s (per R129-9 §8.1)
- ✅ 集成层 test PASS 79 cases (per R129-19 §9.3)
- ✅ 9 organ ticker 100ms 周期 CPU < 0.1%, RAM 6 MB (per R129-9 §3.5)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2): vanilla JS + vanilla SVG + 浏览器 native WebSocket/localStorage

**维度 6.2 Tauri 性能瓶颈分析 (per R130-3 §4 + R131-8 §2.8)**:
- 5 nav 切换: 0 瓶颈 (vanilla JS 切换) — 0 优化 (per 决策 #33 §2.3 C2 0 装)
- 9 organ ticker: ticker 0 触 Tauri command (avoid flood) — 0 优化 (vanilla JS ticker)
- 9 健康环 SVG: 0 瓶颈 (vanilla SVG) — 0 优化 (0 装 D3/eCharts)
- heart ECG 走纸: 0 瓶颈 (CSS animation) — 0 优化 (0 装 stream lib)
- brain NN 9 节点: 0 瓶颈 (vanilla SVG) — 0 优化 (0 装 visx)
- 主对话 5 阶段进度条: 0 瓶颈 (vanilla SVG) — 0 优化
- 流式打字 50ms/字: 0 瓶颈 (浏览器 native) — V1.1 改 WebSocket chunk append
- CrossNavStore pub/sub: 0 瓶颈 (vanilla JS pub/sub) — 0 优化
- 9 organ 跨 nav 嵌入: 0 瓶颈 (CrossNavStore 1 真相源) — 0 优化

**维度 6.3 Tauri 性能 V1.1 release 深化 5 子方向 (per R130-3 §4 + R131-8 §2.8 + 决策 #33 §2.3 C2 + 决策 #73 §3 不要怕复杂度)**:
- 子方向 6.3.1: **流式打字 WebSocket chunk append** (per R129-31 §2.3 B 维度): 50ms/字 → WebSocket chunk append, 0 装 socket.io
- 子方向 6.3.2: **9 organ 真 sensor 后端 Rust crate 真实施** (per R129-31 §2.5 D 维度): 后端 Rust crate 真实施, 0 装 sensor 硬件驱动
- 子方向 6.3.3: **WebSocket 长连接稳定性** (per R130-3 §3.1 R2 + R131-8 §2.8): 浏览器 native WebSocket, 0 装 socket.io
- 子方向 6.3.4: **跨 tab 持久化浏览器差异** (per R130-3 §3.1 R3 + R131-8 §2.8): localStorage + BroadcastChannel, 浏览器原生 API
- 子方向 6.3.5: **WebGPU / GPU 加速 (V2.0 release 蓝图, per 决策 #74 §2.3 + 决策 #73 §3 不要怕复杂度)**: V2.0 release 蓝图, 1.0 / 1.1 release 0 装

**维度 6.4 Tauri 性能 实施 spec 接口 + 测试 (per R131-8 §2.8 + 决策 #33 §2.3 C2)**:
- 接口 6.4.1: frontend/src/dialogue-stream.js (5.1 KB, R129-9) 0 改 5 阶段进度条 + 流式打字, 仅 add WebSocket chunk append
- 接口 6.4.2: src-tauri/src/ws/ 加 websocket.rs (WebSocket 长连接, browser native, 0 装 socket.io)
- 接口 6.4.3: src-tauri/src/commands/dialogue.rs 加 2 NEW commands (ws_connect / ws_disconnect), 总 27 → 29 commands
- 接口 6.4.4: frontend/src/integration/chat_history.js (3 KB, R129-19) 0 改, 仅 add WebSocket 推送 subscribe
- 测试 6.4.5: cargo test 2 NEW WebSocket commands × 5 cases = 10 NEW tests
- 测试 6.4.6: 集成层 chat_history.test.js add WebSocket chunk append cases × 5 = 5 NEW tests
- 测试 6.4.7: 9 organ 真 sensor 14 NEW tests (per R129-31 §2.5 D 维度)
- 测试 6.4.8: 跨 tab 持久化 4 模块 × 5 cases = 20 NEW tests (per R129-31 §2.4 C 维度)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装 socket.io / 0 装 D3 / 0 装 visx / 0 装 eCharts)

### 2.8 维度 7: Tauri 借脑 (5 借脑 0 装) 实施 spec (per R130-3 §5 + R131-8 §2.5-§2.6 + 决策 #33 §2.3 C2)

**维度 7: Tauri 借脑 (5 借脑 0 装) 实施 spec (per R130-3 §5 + R131-8 §2.5-§2.6 + 决策 #33 §2.3 C2 0 借脑 0 装)**:

**维度 7.1 Tauri 借脑 5 借脑 状态盘点 (per R130-6 §1.1 + R129-7 §1 + R129-28 §1.1 实地 verify 100%)**:

| # | 借脑 ID | 借脑大小 | 借脑深度 | 借脑模式 | 0 装 PASS 严守 |
|---:|---------|---------:|----------|----------|----------------|
| 1 | Tauri 2.0 (`tauri v2.11.5` + `tauri-build 2.6.3` + `tauri-macros 2.6.3`) | (Tauri 2.0 deps 已下载, 0 装 PASS 严守) | 真实施 (frontend/tauri-prototype/src-tauri/Cargo.toml) | Rust native desktop app framework, 0 借脑 0 装 | ✅ 0 装"已集成 Tauri 2.0" (真实施) |
| 2 | superpowers 234 (`obra/superpowers 6.2.0` 1.52MB) | 1.52MB / 180 files / 17:33 | 中等借鉴 (5 阶段 DialoguePhase 1:1 翻译) | 借鉴 executing-plans 5 阶段状态机, 0 借源码, 0 装 | ✅ 0 装"已集成 superpowers" (1:1 翻译) |
| 3 | langgraph 829 (`langchain-ai/langgraph d56666f` 13.29MB) | 13.29MB / 670 files / 16:31 | 中等借鉴 (stream_state_events 1:1 翻译) | 借鉴 stream_state_events 协议, 0 借源码, 0 装 | ✅ 0 装"已集成 langgraph" (1:1 翻译) |
| 4 | servers 1.4MB (`modelcontextprotocol/servers 76d64c8` 1.40MB) | 1.40MB / 145 files / 16:51 | 浅借鉴 (MCP server 设计模式 1:1 翻译) | 借鉴 MCP server 6 类 (calendar/message/contact/task/search/drive), 0 借源码, 0 装 | ✅ 0 装"已集成 MCP server" (1:1 翻译) |
| 5 | kani 5.5MB (`model-checking/kani 0.67.0` 5.46MB) | 5.46MB / 3224 files / 17:35 | 形式化借脑 (0 装, 仅借 5 模式 1:1 翻译) | 形式化证明 5 模式 (Invariant trait / ProofHarness / ProofResult / 8 Kani-style harness / bounded model checking), 0 装, 0 引 kani crate 依赖 | ✅ 0 装"已集成 kani" (1:1 翻译) |

**维度 7.2 Tauri 借脑 5 子方向 实施 spec (per R130-3 §5 + R131-8 §2.5-§2.6 + 决策 #33 §2.3 C2)**:
- 子方向 7.2.1: **Tauri 2.0 0 借脑 0 装** (per P11-1/2 真实施, tauri v2.11.5): frontend/tauri-prototype/ 0 借具体源码, 0 装"已读 Tauri 2.0 真源码" (真实施 = rust crate, 0 装 PASS 严守 100%)
- 子方向 7.2.2: **superpowers 234 5 阶段 DialoguePhase 1:1 翻译** (per R130-3 §5.2 + R131-8 §2.6): core/dialogue.rs 5 DialoguePhase 1:1 翻译 superpowers 234 executing-plans, 4 ThinkingPhase 1:1 翻译 PHL-07, 0 借具体源码, 0 装
- 子方向 7.2.3: **langgraph 829 stream_state_events 1:1 翻译** (per R130-3 §5.3 + R131-8 §2.8): WebSocket chunk append 1:1 翻译 langgraph 829 stream_state_events, 浏览器 native WebSocket, 0 装 socket.io
- 子方向 7.2.4: **servers 1.4MB MCP server 设计模式 1:1 翻译** (per R130-3 §5.5 + R131-8 §2.5): 6 工具 endpoint 镜像 MCP server 6 类 (calendar/message/contact/task/search/drive), 0 借源码, 0 装
- 子方向 7.2.5: **kani 5.5MB 形式化借脑 0 装** (per R137-5 + R130-3 §5.4): 形式化证明 5 模式 1:1 翻译, 0 引 kani crate 依赖, 0 装

**维度 7.3 Tauri 借脑 实施 spec 接口 + 测试 (per R131-8 §2.5-§2.6 + 决策 #33 §2.3 C2)**:
- 接口 7.3.1: Tauri 2.0 真实施 0 借脑 0 装 (frontend/tauri-prototype/src-tauri/), P11-1/2 baseline 0 改
- 接口 7.3.2: superpowers 234 1:1 翻译 core/src/dialogue.rs (5 DialoguePhase + 4 ThinkingPhase, 11 tests), 0 借源码
- 接口 7.3.3: langgraph 829 1:1 翻译 frontend/src/dialogue-stream.js (5.1 KB, R129-9), 0 借源码
- 接口 7.3.4: servers 1.4MB 1:1 翻译 core/src/tools.rs (6 tool endpoint, 9 tests), 0 借源码
- 接口 7.3.5: kani 5.5MB 1:1 翻译 core/src/ 形式化模块, 0 引 kani crate 依赖
- 测试 7.3.6: cargo test 0 越界 (0 改 24 LOCKED 入口签名, 0 借脑 0 装 PASS 严守 100%)
- 测试 7.3.7: 集成层 79 tests 0 改 (per R129-19 baseline)
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2): 5 借脑 0 装, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"

### 2.9 维度 8: Tauri PHL-07 主对话锚集成 实施 spec (per R130-5 §2.1 + R131-3 §2.1 + 决策 #22 §1.1-1.2 + 决策 #74 A3)

**维度 8: Tauri PHL-07 主对话锚集成 实施 spec (per R130-5 §2.1 + R131-3 §2.1 + 决策 #22 §1.1-1.2 + 决策 #74 A3 改写 PHL-07 V1.0 spec-only → V1.1 实施)**:

**维度 8.1 PHL-07 状态盘点 (per 决策 #74 A3 + R130-5 §2.1 + R137-1 PHL-07 实施 spec 续)**:
- 🔒 PHL-07 V1.0 spec-only 0 实施 (per 决策 #33 §2.1 A3 + 决策 #74 §1 A3 + R129-11 关键诚实标)
- 🟢 PHL-07 V1.1 实施 (per 决策 #74 §1 A3 改写 + 决策 #74 §2.2 V1.1 release Mavis 自决改)
- 24 LOCKED 入口 → 25 LOCKED 入口 (PHL-07 加 1 入口, per 决策 #22 §1.1-1.2)
- 13 键 → 14 键 (PHL-07 加 1 键 + 主对话锚 1 键, per R137-1 §2 续)
- 14 维主对话锚 (per R132-1 §2.1.2): 5 阶段 DialoguePhase 深化 + 4 ThinkingPhase 深化 + 5 维主对话锚 (北极星 / 实事求是 / 质量工程化 / 安全优先 / 不假装) 集成
- 41 NEW tests (per R137-1 §2 续: 14 维 + 8 锚 + 6 重 + 13 键)

**维度 8.2 PHL-07 V1.1 release 实施 5 子方向 (per R130-5 §2.1 + R131-3 §2.1 + R137-1 PHL-07 实施 spec + 决策 #22 §1.1-1.2)**:
- 子方向 8.2.1: **PHL-07 spec → impl** (per R137-1 §2.1): PHL-07 entry 25 LOCKED 入口签名 impl, 14 维主对话锚 spec 翻译 impl, 0 假装
- 子方向 8.2.2: **PHL-07 形式化** (per R137-5 形式化 Stage 5.5+ 实战 + 决策 #74 §1 B3/B4/B5 严守): 14 维主对话锚 Kani 形式化证明 1:1 翻译 (0 装 kani crate), 5 模式
- 子方向 8.2.3: **PHL-07 编译期 hardcode** (per R137-1 §2.2 + 决策 #74 §2.2 24 → 25 LOCKED 入口签名): V05_DIM_COUNT 14 维 hardcode 同步, 0 假装
- 子方向 8.2.4: **PHL-07 6 重守门 v7 集成** (per 决策 #33 §2.3 B4 + 决策 #55 §4 + R137-1 §2.3): PHL-07 跟 6 重守门 v7 1:1 集成, 0 改 6 重守门
- 子方向 8.2.5: **PHL-07 8 哲学锚集成** (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + R137-1 §2.4): PHL-07 跟 8 哲学锚 1:1 集成, 0 改 8 哲学锚, 0 暴露 UI

**维度 8.3 PHL-07 跟 9 organ 集成 (per 决策 #22 §1.1-1.2 + R130-5 §2.1 + 维度 3.2.6)**:
- 集成 1: **14 维主对话锚 ↔ 9 organ 1:1 集成** (心/脑/手/眼/耳/记忆/声/体/意 + 5 维主对话深化)
- 集成 2: **PHL-07 ↔ CrossNavStore 集成** (per R129-19 §1.3 + 维度 2.3 接口)
- 集成 3: **PHL-07 ↔ Tauri 2.0 集成** (per 维度 1.3 接口)
- 集成 4: **PHL-07 ↔ Stage 4 D 真 sensor 集成** (per 维度 3.3 + R129-31 §2.5)

**维度 8.4 PHL-07 实施 spec 接口 + 测试 (per R137-1 + 决策 #74 §1 A3)**:
- 接口 8.4.1: `crates/apeireth-cognition/src/phl07.rs` NEW src 25 LOCKED 入口签名 impl
- 接口 8.4.2: `crates/apeireth-cognition/src/lib.rs` 加 pub mod phl07; + re-export PHL07Anchor 14 维
- 接口 8.4.3: `src-tauri/src/commands/dialogue.rs` 加 1 NEW command (get_phl07_anchor), 总 27 → 28 commands
- 接口 8.4.4: `frontend/src/integration/store.js` 加 1 EVT (phl07_anchor_updated), 14 EVT → 15 EVT
- 测试 8.4.5: cargo test 14 维主对话锚 × 3 cases = 42 NEW tests
- 测试 8.4.6: cargo test 8 哲学锚 + PHL-07 集成 × 1 case = 8 NEW tests
- 测试 8.4.7: cargo test 6 重守门 v7 + PHL-07 集成 × 1 case = 6 NEW tests
- 测试 8.4.8: cargo test 13 键 + PHL-07 = 14 键 verdict cache × 5 cases = 70 NEW tests (per R137-1 §2)
- 测试 8.4.9: 集成层 dialogue-anchor.test.js 14 维 × 3 cases = 42 NEW tests
- 8 哲学锚严守 100% (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5)
- 6 重守门 v7 严守 100% (per 决策 #33 §2.3 B4 + 决策 #74 §1 B4)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.3 C2, 0 装 kani crate, 仅 1:1 翻译)

### 2.10 实施 spec 总览表 (per 8 维度 拓维)

**8 维度 实施 spec 总览表 (per 8 维度 拓维 + 决策 #74 §2.2 B1 + 决策 #33 §2.3)**:

| 维度 | 实施 spec 核心 | 估 NEW tests | 派活 | 8 硬墙严守 | 0 装 PASS 严守 |
|:---:|----------------|------------:|------|:---:|:---:|
| **1 Tauri 2.0 完整集成** | 27 → 36 commands + 5 capabilities + 多窗口 + tray + menu + updater | 18 | R152-4-1 | ✅ 0 越界 | ✅ 0 装 |
| **2 5 nav 完整集成** | 5 nav 真打通 + 7 模块 J1-J7 + CrossNavStore + tauriInvoke | 30+25+35+30 = 120 | R152-4-2 | ✅ 0 越界 | ✅ 0 装 |
| **3 9 organ 拟人化 final** | heart ECG + brain NN + 9 健康环 + 永远循环 + 真 sensor 14 + PHL-07 集成 | 25+25+14 = 64 | R152-4-3 | ✅ 0 越界 | ✅ 0 装 |
| **4 Stage 4-8 实战路线** | Stage 4 4 维度 84 + Stage 5 30 + Stage 6 30 + Stage 7 跨平台 + Stage 8 用户测试 | 84+30+30 = 144 | R152-4-4 | ✅ 0 越界 | ✅ 0 装 |
| **5 Tauri 跨平台** | 5 bundle format + Tauri 2.0 updater + GitHub Actions CI | 0 NEW (cargo tauri build 3 平台 PASS) | R152-4-5 | ✅ 0 越界 | ✅ 0 装 |
| **6 Tauri 性能** | WebSocket chunk append + 9 organ 真 sensor + 跨 tab 持久化 | 10+5+14+20 = 49 | R152-4-6 | ✅ 0 越界 | ✅ 0 装 |
| **7 Tauri 借脑 (5 借脑 0 装)** | Tauri 2.0 + superpowers 234 + langgraph 829 + servers 1.4MB + kani 5.5MB 1:1 翻译 | 0 NEW (蓝图就绪 0 改) | (R152-4-1 ~ R152-4-6 协同) | ✅ 0 越界 | ✅ 0 装 |
| **8 Tauri PHL-07 集成** | PHL-07 impl + 14 维主对话锚 + 8 哲学锚 + 6 重 v7 + 14 键 verdict cache | 42+8+6+70+42 = 168 | (R152-4-3 协同) | ✅ 0 越界 | ✅ 0 装 |
| **总** | **8 维度 实施 spec** | **~600 NEW tests** | **6 sub-agent 派活 R152-4-1~6** | ✅ 0 越界 100% | ✅ 0 装 100% |

**实施 spec 跟 R131-8 §5 6 维度 470 min 蓝图 关系 (per 决策 #74 §2.2 B1 + 决策 #33 §2.3)**:
- ✅ R131-8 §5 6 维度 蓝图 (A Stage 4 实战 120 min + B Stage 5 集成 90 min + C Stage 6 后端 90 min + D Stage 7 部署 75 min + E 砍 7 项 UI 哲学 5 min + F PHL-07 集成 90 min = 470 min) **0 重写**
- ✅ R152-4 拓维: R131-8 6 维度 → R152-4 8 维度 + 实施 spec 详细 (接口 + 测试 + 风险 + 派活计划 + 8 硬墙严守 verify)
- ✅ 6 维度 蓝图 (R131-8 §5) ⊂ 8 维度 实施 spec (R152-4 §2): R131-8 维度 A → R152-4 维度 4 + R131-8 维度 B → R152-4 维度 1+2+3+5+7+8 + R131-8 维度 C → R152-4 维度 4 续 + R131-8 维度 D → R152-4 维度 5 + R131-8 维度 E → R152-4 维度 2.2 + R131-8 维度 F → R152-4 维度 8 + R152-4 新增 维度 6 (Tauri 性能) + 维度 7 (Tauri 借脑)

---

## 3. Tauri 集成 优化 跟 Rust 后端 (apeireth-api) 的关系 (per R131-8 + R130-3 + 决策 #74 B1 + 用户记忆 #8)

### 3.1 Rust 后端 (apeireth-api) 状态盘点 (per 决策 #33 + 决策 #55 + 决策 #57-#58 + R17 战略 1-4 完工)

**apeireth-api crate 状态盘点 (per 决策 #33 + 决策 #55 + 决策 #57-#58 + R17 战略 1-4 完工)**:
- ✅ **apeireth-api crate** 已实施 (per `crates/apeireth-api/src/lib.rs` + Cargo.toml, R17 战略 1-4 完工)
- ✅ **4 协议 endpoint** (per R17 战略 1-4 完工 + `crates/apeireth-api/src/protocol_handlers.rs`): OpenAI Chat + OpenAI Responses + Anthropic Messages + Google Gemini
- ✅ **V2 6 段 endpoint** (per R25 Step 2 + `crates/apeireth-api/src/v2_endpoints.rs` + `v2_routes/`): tools + memory + organs + asi + sovereignty + agent
- ✅ **V1 8 endpoint** (per R130-3 §3.1 蓝图 + 决策 #74 B1 实施 spec): 7 GET + 1 POST + 1 PATCH + 1 WS = 10 endpoint
- ✅ **observability 模块** (per `crates/apeireth-api/src/observability/`): status + metrics + health + dashboard
- ✅ **replay_cache** (per R120 B2 战略 2 + R122-1-retry B5 战略 2 + `crates/apeireth-api/src/replay_cache.rs`): VCP 协议 (method, url, body) 三元组 SHA-256 hash, 重复请求 fast path
- ✅ **LLM provider 5 实现** (per R17 战略 0 完工 + `crates/apeireth-api/src/llm/providers/`): OpenAI + Anthropic + Google Gemini + scripted + apeireth_api
- ✅ **HTTP server axum** (per `crates/apeireth-api/src/server.rs` + `examples/serve.rs`): axum HTTP server 7 endpoint
- ✅ **3 启动模式** (per 决策 #33 + 决策 #55 + 决策 #57-#58 + R17 战略 1-4): 开发模式 + 生产模式 + 测试模式
- ✅ **0 装 PASS 严守** (per 决策 #33 §2.3 C2): 0 cargo install / 0 cargo add, 仅 axum 0.7+ + hyper 1.x + tokio 1.x (已装 R125 era)

### 3.2 Tauri 集成 优化 跟 Rust 后端 (apeireth-api) 关系 6 子方向 (per R131-8 §2 + R130-3 §3.1 + 决策 #74 B1 + 用户记忆 #8)

**子方向 1: Tauri 跟 apeireth-api HTTP 集成 (per R131-8 §2.4 + R130-3 §3.1 + 决策 #74 B1)**:
- 当前: Tauri 0 接 apeireth-api, Stage 1-3 蓝图就绪, 0 假装已接
- V1.1 release 实施 (per 决策 #74 B1 Mavis 自决改): Tauri 27 → 32+ commands, CrossNavStore 调 tauriInvoke, 7 GET + 1 POST + 1 PATCH + 1 WS endpoint 真接通
- 8 endpoint 真接通: GET /v1/organs + POST /v1/chat/messages + GET /v1/chat/session/{id} + GET /v1/history + GET /v1/tools/results + GET /v1/settings + PATCH /v1/settings/{key} + WS /v1/chat/stream
- HTTP 路由实施 spec (per R130-3 §3.1): CrossNavStore 7 模块 + 9 organ animator + 14 settings editor 全部 tauriInvoke 调 8 endpoint
- 0 借脑 0 装 严守 (per 决策 #33 §2.3 C2, 0 装新 HTTP client lib, 用浏览器 native fetch + Tauri 2.0 invoke)

**子方向 2: Tauri 跟 apeireth-api WebSocket 集成 (per R130-3 §3.1 + 决策 #74 B1)**:
- 当前: Tauri 0 接 WebSocket, R129-9 流式打字 50ms/字 字符级
- V1.1 release 实施: WebSocket chunk append 1:1 翻译 langgraph 829 stream_state_events, browser native WebSocket, 0 装 socket.io
- WebSocket 协议 (per R129-31 §2.3 + langgraph 829 1:1 翻译):
  ```
  client → server: {"type": "send_message", "session_id": "...", "content": "..."}
  server → client: {"type": "phase_change", "phase": "Streaming"}
  server → client: {"type": "stream_chunk", "content": "..."}  // 累加到 AI 气泡
  server → client: {"type": "stream_end", "full_content": "..."}  // 写入 history
  server → client: {"type": "phase_change", "phase": "Awaiting"}
  ```
- 0 借脑 0 装 严守 (per 决策 #33 §2.3 C2, 0 装 socket.io, 仅浏览器 native WebSocket)

**子方向 3: Tauri 跟 4 协议 endpoint 集成 (per R17 战略 1-4 + 决策 #55 + 决策 #57-#58)**:
- 当前: Tauri 0 接 4 协议 endpoint (OpenAI Chat + OpenAI Responses + Anthropic Messages + Google Gemini)
- V1.1 release 实施 (per Stage 6 后端接通 R130-3 §3.1): Tauri 27+ commands, 1 command 调 4 协议 endpoint, 主对话 nav 集成
- 0 装 PASS 严守 (per 决策 #33 §2.3 C2): 0 装 LLM client lib, 用 apeireth-api HTTP client
- 0 改主仓 src/ 严守 (per 决策 #33 §2.3 B1, 0 触碰 24 LOCKED 入口签名)

**子方向 4: Tauri 跟 V2 6 段 endpoint 集成 (per R25 Step 2 + 决策 #55)**:
- 当前: Tauri 0 接 V2 6 段 endpoint
- V1.1 release 实施: Tauri 27+ commands 调 V2 6 段 endpoint 真接通 (跟 8 endpoint 重叠)
- 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**子方向 5: Tauri 跟 3 启动模式集成 (per 决策 #33 + 决策 #55 + 决策 #57-#58)**:
- 开发模式 (cargo tauri dev): Tauri 0 接 apeireth-api, mock fallback (per R129-31 §2.1)
- 生产模式 (cargo tauri build): Tauri 真接 apeireth-api HTTP + WebSocket (per R130-3 §3.1)
- 测试模式 (cargo test): Tauri 0 接 apeireth-api, mock data (per 集成层 79 tests + 84 NEW tests)
- 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**子方向 6: Tauri 跟 replay_cache + observability 集成 (per R120 B2 + R122-1-retry B5 + observability/)**:
- 当前: Tauri 0 接 replay_cache
- V1.1 release 实施 (per 决策 #74 B1): Tauri 调 8 endpoint 时 use replay_cache, 重复请求 fast path
- 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

### 3.3 Tauri 集成 跟 Rust 后端 关系 严守 (per 决策 #33 §2.3 + 决策 #74 §2.2 B1)

**8 硬墙 严守**:
- ✅ B1 24 LOCKED 入口签名 0 改 (per 决策 #33 §2.3 B1 + 决策 #74 §2.2 B1 V1.0 release 0 改严守)
- 🟢 B1 24 LOCKED 入口签名 V1.1 release Mavis 自决改 (per 决策 #74 §2.2 B1, 仅扩 endpoint, 0 改原 24 LOCKED 入口签名, PHL-07 加 1 入口 = 25 LOCKED 总数)
- ✅ B2 workspace.version 1.2.0 严守 (per 决策 #33 §2.3 B2, V1.1 release bump 1.2.1)
- ✅ A1 R11 baseline 3 值 0 改 (per 决策 #33 §2.1 A1, 0 触碰 integration_r_measure.rs)
- 🟡 A3 PHL-07 V1.0 spec-only + V1.1 实施 (per 决策 #33 §2.1 A3 + 决策 #74 §1 A3 改写)
- ✅ B3 V0.5 30 维 0 改 (per 决策 #33 §2.3 B3)
- ✅ B4 6 重守门 v7 0 改 (per 决策 #33 §2.3 B4)
- ✅ B5 8 哲学锚 0 暴露 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)
- ✅ C1 0 主动 commit (per 决策 #33 §2.3 C1)
- ✅ C2 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ 0 push (per 决策 #33 + 决策 #61 §6)

---

## 4. Tauri 集成 优化 5 nav + 9 organ 拟人化 (per R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3-#5 + R129-19 Stage 3)

### 4.1 5 nav 状态盘点 (per P11-1 + P11-2 + R129-9 + R129-19 + 用户记忆 #3)

**5 nav 状态 (per P11-1/2 + R129-9/19 baseline + 用户记忆 #3)**:
- ✅ 5 nav = NAV_ID 0-4 严守 (状态 / 主对话 / 历史 / 设置 / 工具结果)
- ✅ 状态 nav (NAV_ID 0): 9 organ 卡片 3x3 + 9 健康环 + heart ECG + brain NN (per P11-1 §1.1 + R129-9 §3 + R129-19 §3)
- ✅ 主对话 nav (NAV_ID 1): 5 阶段 DialoguePhase + user/AI 气泡 + 5 阶段进度条 + 流式打字 (per P11-1 + R129-9 §2)
- ✅ 历史 nav (NAV_ID 2): 3 kind (会话/消息/工具调用) + SVG 时间线 (per P11-1 + R129-9 §2.4 timeline.js)
- ✅ 设置 nav (NAV_ID 3): 14 settings 分 3 section (5+5+4, 5 鉴权 + 5 Provider + 4 SDK) + 开关/状态 (per P11-1 + R129-9 §2.5 settings-editor.js)
- ✅ 工具结果 nav (NAV_ID 4): 6 工具 card + 颜色编码 + 弹窗 (per P11-1 + R129-9 + R129-19 §3.1 J5)
- ✅ CrossNavStore 状态中枢 (per R129-19 §1.3, 14 EVT + 12 mutators + 5 nav 状态 + 9 organ 活动)
- ✅ 集成层 7 模块 J1-J7 (per R129-19 §2.1): status_chat / status_history / status_tools / chat_history / chat_tools / history_tools / settings_global
- ✅ 集成层 79 tests pass + 8 examples + 1 hub (per R129-19 §9.3)
- ✅ 0 暴露 UI 哲学 100% (per 用户记忆 #3 砍 7 项: 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM)
- ✅ 0 加 0 砍 0 改 NAV_ID 0-4 (per 用户记忆 #3 严守)

### 4.2 Tauri 集成 优化 5 nav 严守 (per R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3 + 决策 #74 B1)

**Tauri 集成 优化 5 nav 严守 (per R130-3 §2.3 + R131-8 §2.2 + 用户记忆 #3 + 决策 #74 B1)**:
- ✅ **0 改 5 nav** (per 用户记忆 #3 + 决策 #33 §2.3 B5): 0 加 0 砍 0 改 NAV_ID 0-4
- ✅ **0 暴露 UI 哲学** (per 用户记忆 #3 砍 7 项 + 决策 #33 §2.3 B5)
- ✅ **0 假装已实施** (per 决策 #10 + 决策 #33 §2.3 C2): 5 nav 全 Stub readiness 严守 (Stage 1-3)
- ✅ **5 nav 1:1 镜像 TUI** (per 决策 #9 + 用户记忆 #8 + 用户记忆 #9 瘦客户端): nav/mod.rs → CrossNavStore.NAV_ID 1:1 严守
- ✅ **5 nav 跟 TUI 升级路径一致** (per 决策 #9 + 用户记忆 #8): TUI/Tauri 升级 1:1 翻译, 后端 API 表面 0 改
- ✅ **5 nav 真打通 (V1.1 release 实施, per 决策 #74 B1)**: CrossNavStore + 7 模块 J1-J7 + tauriInvoke 主路径
- ✅ **5 nav 跟后端 API 表面 0 改** (per 用户记忆 #8 + 决策 #9): 8 endpoint 真接通, 0 触碰后端 crate

**Tauri 集成 优化 5 nav 跟 Rust 后端 关系**:
- 5 nav 共享 CrossNavStore 1 真相源 (per R129-19 §1.3)
- 5 nav 通过 tauriInvoke 调 8 endpoint 真接通 (per R130-3 §3.1)
- 5 nav 跟 TUI 1:1 镜像 (per 决策 #9 + 用户记忆 #8)
- 5 nav 永远循环 ticker 0 死亡 (per 用户记忆 #4, ticker.js 100ms 周期)
- 5 nav 0 暴露 UI 哲学 100% (per 用户记忆 #3 砍 7 项)

### 4.3 9 organ 拟人化状态盘点 (per P11-1 + P11-2 + R129-9 + R129-19 + 用户记忆 #4-#5)

**9 organ 拟人化状态 (per P11-1/2 + R129-9/19 + 用户记忆 #4-#5)**:
- ✅ 9 organ = ORGAN_ID 0-8 严守 (heart / brain / hand / eye / ear / memory / voice / body / mind)
- ✅ 9 健康环 (per R129-9 §3.2): 1 屏 9 个 SVG circle, radius 30, stroke-width 6, 颜色 0-30 红/30-70 黄/70-100 绿
- ✅ heart ECG (per R129-9 §3.3): P-QRS-T 三段, 60 采样/周期, 走纸动画, 红色
- ✅ brain NN (per R129-9 §3.4): 9 节点 + 8 中心边 + 8 围圈边, hover 放大, 紫色
- ✅ organ_animator.js (per R129-19 §2.1, 9 KB): 5 helper (renderChatHeaderOrgans / renderToolsHeaderOrgan / renderHistoryHeaderOrgans / renderSettingsHeaderOrgan / getOrganHealthSummary)
- ✅ ticker.js (per R129-9 §3.5, 100ms 周期, 永远循环, 0 死亡)
- ✅ 永远循环 0 死亡 (per 用户记忆 #4): 0 显示 "已死亡/老化/终止", 用 "活跃度" (active/idle/dormant) 0 用 "健康度" (healthy/sick)
- ✅ 1 真相源 CrossNavStore (per R129-19 §1.3): organ_activities 9 organ 1 真相源, 5 nav 共享
- ✅ 0 暴露内部机制 100% (per 用户记忆 #3 砍 7 项)

### 4.4 Tauri 集成 优化 9 organ 严守 (per R130-3 §2.4 + R131-8 §2.3 + 用户记忆 #4-#5 + 决策 #74 B1)

**Tauri 集成 优化 9 organ 严守 (per R130-3 §2.4 + R131-8 §2.3 + 用户记忆 #4-#5 + 决策 #74 B1)**:
- ✅ **0 改 9 organ** (per 用户记忆 #4-#5 + 决策 #33 §2.3 B5): 0 加 0 砍 0 改 ORGAN_ID 0-8
- ✅ **永远循环 0 死亡** (per 用户记忆 #4 + 决策 #33 §2.3 B5): ticker.js 100ms 周期, 活跃度 0-100 永远循环
- ✅ **0 暴露 8 哲学锚** (per 用户记忆 #3 + 决策 #33 §2.3 B5): brain NN 只显示 "AI 在思考" 姿态, 0 暴露 6 重守门/24 LOCKED 内部 fn
- ✅ **9 organ 1 真相源 5 nav 共享** (per R129-19 §1.3 + 决策 #74 B1): CrossNavStore.organ_activities 9 organ 1 真相源
- ✅ **9 organ 1 屏多卡** (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化): 3x3 网格 + ECG + NN + 健康环
- ✅ **9 organ 永远循环 ticker 0 触 Tauri command** (per R129-9 §3.5 avoid flood)
- ✅ **9 organ 跟后端 Rust crate 真接通** (per R130-3 §2.4 + R129-31 §2.5 D 维度): Stage 4 D 真 sensor 接入
- ✅ **9 organ 跟 PHL-07 14 维主对话锚集成** (per 决策 #22 §1.1-1.2 + 决策 #74 §2.2 V1.1 release 实施)

**Tauri 集成 优化 9 organ 跟 Rust 后端 关系**:
- 9 organ 1 真相源 5 nav 共享 (per R129-19 §1.3)
- 9 organ 通过 tauriInvoke 调 5 NEW 真 sensor commands (per 维度 3.3 接口)
- 9 organ 跟 TUI 1:1 镜像 (per 决策 #9 + 用户记忆 #8)
- 9 organ 永远循环 0 死亡 (per 用户记忆 #4)
- 9 organ 0 暴露内部机制 100% (per 用户记忆 #3 砍 7 项)

---

## 5. Tauri 集成 优化 跟 ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 8 哲学锚 + 不要怕复杂度哲学 + 用户记忆 #3 的关系

### 5.1 Tauri 集成 跟 ASI Stage 9 关系 (per R133-2 §2 + R130-2 调研 + 决策 #55 + 用户记忆 #4)

**ASI Stage 9 长程 AI 成长 (per R133-2 §2 + R130-2 调研 + 决策 #55-#58 + 用户记忆 #4)**:
- ASI Stage 9 4 维度 (per R133-2 §2.5): H 自治 + L 长程 + G 成长 + P 平台化
- 借脑 9 源 (per R133-2 §2.5): 3 真实施 (PyO3 928 + superpowers 234 + chidori) + 6 OpenCog 借脑 (AtomSpace + CogPrime + moses + pln + relex + cogutil, 0 借具体源码)
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #73 §2.2 借脑 OpenCog)
- 5 阶段 5 周 实施 (per R137-4 §3)
- 0 形式化 old/death/terminate 严守 (per 用户记忆 #4)

**Tauri 集成 跟 ASI Stage 9 关系 5 子方向 (per R130-3 §2.4 + R131-8 §2.3 + R133-2 + 决策 #55)**:
- 子方向 1: **ASI Stage 9 跟 9 organ 集成** (per R133-2 §2.5 + 用户记忆 #4): 9 organ 拟人化 + Stage 9 4 维度 (心/脑/手/眼/耳/记忆/声/体/意), 1:1 集成
- 子方向 2: **ASI Stage 9 跟 5 nav 集成** (per R130-3 §2.3 + R133-2): 5 nav 跟 Stage 9 4 维度集成 (H 自治 → 状态 nav / L 长程 → 历史 nav / G 成长 → 主对话 nav / P 平台化 → 设置 nav)
- 子方向 3: **ASI Stage 9 跟 5 DialoguePhase 集成** (per R131-8 §2.3 + R133-2): 5 DialoguePhase 跟 Stage 9 4 ThinkingPhase 集成
- 子方向 4: **ASI Stage 9 跟 CrossNavStore 集成** (per R129-19 §1.3 + R133-2): CrossNavStore 调 pybridge Stage 9 4 维度
- 子方向 5: **ASI Stage 9 跟 pybridge 集成** (per R130-3 §2.4 + R131-7 pybridge 集成优化 + 决策 #74 B1): pybridge Stage 9 跟 Tauri 集成
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #73 §2.2)
- 0 形式化 old/death/terminate 严守 (per 用户记忆 #4)

### 5.2 Tauri 集成 跟 三洋葱 V2 (四洋葱) 关系 (per R133-3 §3 + 决策 #73 §2.2 更好的架构 + 决策 #74 B1)

**三洋葱 V2 (四洋葱) 架构 (per R133-3 §3 + 决策 #73 §2.2 + 决策 #74 B1)**:
- 三洋葱 (R125 B6 升级, V1.0 release 严守): 原则 + 权限 + DSL (per 决策 #22 §2.6 + 决策 #33 §2.3 B6 + 决策 #55 §4 + R125-5)
- 四洋葱 (V1.1 release 实施, per 决策 #74 B1 Mavis 自决改): 原则 + 权限 + DSL + **智能涌现 emergence (智囊团 7 席 + 群体智能 OpenCog 借脑 + 自我决策/学习/演化)**
- 五洋葱 (V2.0 release 蓝图, per 决策 #74 §2.3): 原则 + 权限 + DSL + 智能涌现 + **自我演化 self-evolution (per R130-2 §1 Stage 9-12 路线图)**

**Tauri 集成 跟 三洋葱 V2 关系 4 子方向 (per R133-3 §3 + 决策 #73 §2.2 + 决策 #74 B1)**:
- 子方向 1: **Tauri 跟原则洋葱 集成** (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项): 8 哲学锚严守, 0 暴露 UI, 1:1 集成
- 子方向 2: **Tauri 跟权限洋葱 集成** (per 决策 #33 §2.3 B4 + 决策 #55 §4 + 6 重守门 v7): 6 重守门 v7 严守, 0 暴露 UI, CrossNavStore 0 emit 守门事件
- 子方向 3: **Tauri 跟 DSL 洋葱 集成** (per R125-5 + 决策 #55 §4 + Colang DSL): Colang DSL 守门, 0 暴露 UI
- 子方向 4: **Tauri 跟智能涌现洋葱 (新增 第 4 层) 集成** (per R133-3 §3 + 决策 #73 §2.2 + 决策 #74 B1): 智囊团 7 席 + 群体智能 OpenCog 借脑 + 自我决策/学习/演化, Tauri 5 nav + 9 organ + 14 settings 跟 4 洋葱 1:1 集成
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #73 §2.2 借脑 OpenCog)
- 0 漂移 0 假装 (per 决策 #10 + 决策 #33 §2.3 C2): 蓝图就绪, V1.1 release 实施 0 假装

### 5.3 Tauri 集成 跟 借鉴 12 源 fork 关系 (per R133-1 + R130-6 + 决策 #33 §2.2 + 决策 #73 §2.2 借脑 OpenCog)

**借鉴 12 源 (per R133-1 §1.1 + R130-6 §1.1 + 决策 #22 §4)**:
- 8 真 cloned (per R129-28 §1.1 实地 verify 100%): clap 3.50MB / hyper 0.54MB / servers 1.40MB / PyO3 5.69MB / kani 5.46MB / langgraph 13.29MB / superpowers 1.52MB / Guardrails 18.19MB = 49.59MB / 7,764 files
- 2 借鉴 ID 索引完成 (per R133-1 §1.1): LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块
- 1 永久跳过 (per R133-1 §1.1): OpenCog AGPL-3.0 主仓 ID-011, 0 集成 0 装
- 1 借脑 ID 索引完成 (per R130-6 + R133-1 §1.1): OpenCog 家族 6 子源 (atomspace + cogutil + moses + pln + relex + CogPrime), 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.2 + 决策 #73 §2.2 借脑 OpenCog + 决策 #74 §1 C2)

**Tauri 集成 跟 借鉴 12 源 fork 关系 5 子方向 (per R130-3 §5 + R131-8 §2.5-§2.6 + R133-1 + 决策 #33 §2.2)**:
- 子方向 1: **Tauri 2.0 0 借脑 0 装** (per 维度 7.1, tauri v2.11.5 + tauri-build 2.6.3 真实施, 0 装"已读 Tauri 2.0 真源码")
- 子方向 2: **superpowers 234 5 DialoguePhase 1:1 翻译** (per 维度 7.2.2, 0 借脑 0 装, 0 装"已集成 superpowers")
- 子方向 3: **langgraph 829 stream_state_events 1:1 翻译** (per 维度 7.2.3, 0 借脑 0 装, 0 装"已集成 langgraph")
- 子方向 4: **servers 1.4MB MCP server 设计模式 1:1 翻译** (per 维度 7.2.4, 0 借脑 0 装, 0 装"已集成 MCP server")
- 子方向 5: **kani 5.5MB 形式化借脑 0 装** (per 维度 7.2.5 + R137-5, 0 引 kani crate 依赖, 0 装"已集成 kani")
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- 0 借脑 0 装 严守 100% (per 决策 #33 §2.2 + 决策 #73 §2.2 借脑 OpenCog)

### 5.4 Tauri 集成 跟 8 哲学锚 关系 (per 决策 #33 §2.3 B5 + 用户记忆 #3)

**8 哲学锚 (per 决策 #11 + 决策 #33 §2.3 B5 + 主人 23:23 拍板 0 暴露 UI)**:
- **S-1 服务 ASI 北极星**: V1.0 release 严守 (0 暴露 UI, 0 假装已接 ASI, per 用户记忆 #3 砍 7 项)
- **S-2 实事求是**: V1.0 release 严守 (0 假装已实施, stub 诚实标, per R129-11 关键诚实标)
- **S-3 质量工程化**: V1.0 release 严守 (122 tests pass + cargo build PASS + 0 warning 0 error, per R129-9 §8.1)
- **O-1 安全优先**: V1.0 release 严守 (24 LOCKED 入口签名 0 改, per 决策 #33 §2.3 B1)
- **O-2 走在前人经验上**: V1.0 release 严守 (借脑 0 装 8 借鉴源真实施, per R130-6 §1.1)
- **O-3 干到底**: V1.0 release 严守 (cargo tauri build 0 改 0 越界, per R129-9 §8)
- **O-4 任何人都能接手**: V1.0 release 严守 (README + STRUCTURE + 8 硬墙 0 越界 verify, per P11-1 §10)
- **O-5 不假装**: V1.0 release 严守 (9 organ 全 Stub readiness + AI 回复 = stub + 5 鉴权 disabled + 5 Provider model_count=0, per R129-19 §3.5)

**Tauri 集成 跟 8 哲学锚 关系 5 子方向 (per 决策 #33 §2.3 B5 + 用户记忆 #3 + 决策 #74 §1 B5)**:
- 子方向 1: **S-1 服务 ASI 北极星 集成** (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项): Tauri 0 暴露 UI 哲学, 0 假装已接 ASI
- 子方向 2: **S-2 实事求是 集成** (per R129-11 关键诚实标 + 决策 #33 §2.3 B5): Tauri 0 假装已接 LLM, stub 诚实标
- 子方向 3: **S-3 质量工程化 集成** (per 决策 #33 §2.3 B5 + R129-9 §8.1): Tauri 122 tests pass + cargo build PASS + 0 warning 0 error
- 子方向 4: **O-1 + O-2 + O-3 + O-4 + O-5 集成** (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5): 24 LOCKED 入口签名 0 改 + 借脑 0 装 + cargo tauri build 0 改 0 越界 + README + STRUCTURE + 8 硬墙 0 越界 verify + 0 假装
- 子方向 5: **8 哲学锚 跟 CrossNavStore 集成** (per R129-19 §1.3 + 决策 #33 §2.3 B5): CrossNavStore.EVT 0 含哲学锚, B5 硬墙严守
- 0 暴露 UI 哲学 100% (per 用户记忆 #3 砍 7 项: 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM)
- 9 organ 永远循环 0 死亡 (per 用户记忆 #4, ticker.js 100ms 周期, 0 用 health/sick/dying)

### 5.5 Tauri 集成 跟 不要怕复杂度哲学 关系 (per 决策 #73 §3 + 15-no-fear-complexity.md + 主人 8/11 01:14 拍板)

**不要怕复杂度哲学 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + 主人 8/11 01:14 拍板)**:
- **核心 1**: 最强效果 > 最简单代码
- **核心 2**: 最厉害工程 > 最易维护
- **核心 3**: 维护交给未来高水平团队 (per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护")
- **新哲学 4 件套**:
  - ✅ "代码要最强效果 + 最厉害工程"
  - ✅ "复杂度是实力的体现"
  - ✅ "维护交给未来高水平团队"
  - ❌ 推翻: "代码要简单易维护" / "复杂度是技术债" / "维护成本是重要指标"

**Tauri 集成 跟 不要怕复杂度哲学 关系 5 子方向 (per 决策 #73 §3 + 15-no-fear-complexity.md + 用户记忆 #2 + 决策 #74 B1)**:
- 子方向 1: **Tauri 集成 V1.1 release 6-12 周 实施** (per 8 维度 实施 spec + 决策 #74 B1 + 决策 #73 §3): 最强效果 (Tauri 2.0 完整 + 5 nav + 9 organ + Stage 4-8 + 跨平台 + 性能 + 借脑 + PHL-07 8 维度) + 最厉害工程 (Tauri 2.0 native + 30+ 借脑 0 装 + 8 硬墙 0 越界 + 8 哲学锚严守)
- 子方向 2: **Tauri 集成 维护交给未来高水平团队** (per 决策 #73 §3 + 主人 8/11 01:14 拍板): 0 简化代码 (项目复杂度是吸引高水平团队的核心, 简化代码 = 排斥高水平团队)
- 子方向 3: **Tauri 集成 不为简化而简化** (per 决策 #73 §3 + 15-no-fear-complexity.md §1.1): V1.1 release 6-12 周 实施 是最强效果, 0 为易维护而牺牲工程化
- 子方向 4: **Tauri 集成 不漂移不假装** (per 决策 #10 + 决策 #33 §2.3 C2): V1.1 release 实施 0 假装, 0 假装已集成 Tauri 2.0 真源码
- 子方向 5: **Tauri 集成 9 件套 总哲学** (per 15-no-fear-complexity.md §2): 8 哲学锚 (思想) + 不要怕复杂度 (工程) = 完整思想 + 工程边界
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2): 0 装是技术哲学 (底线), 不要怕复杂度是工程哲学 (上限)
- 8 硬墙 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1): 8 硬墙 (底线, 不可破), 不要怕复杂度 (上限, 可超)

### 5.6 Tauri 集成 跟 用户记忆 #3 (用户看结果不看哲学) 关系 (per 用户记忆 #3 + 决策 #33 §2.3 B5)

**用户记忆 #3 (per 用户记忆 #3 + 主人 8/4 R19 8 个认知纠正)**:
- 主人原文: "守门, 原则, 电子环的, 这种东西你觉得用户需要看吗?"
- 主人原文: "用户想体验的并不是带娃, 而是看到ai和自己一同成长, 只看结果和好用"
- 主人原文: "工具的调用啥的, 用户根本就不关心. ...都只看结果的"
- 核心: 用户看结果不看哲学
- 严守 7 项 UI 哲学 砍: 守门/电子环/工具调用过程/哲学锚/内部机制/衰老病死/0 主动 IM

**Tauri 集成 跟 用户记忆 #3 关系 6 子方向 (per 用户记忆 #3 + 决策 #33 §2.3 B5 + 决策 #74 B1)**:
- 子方向 1: **5 nav 严守 1:1 镜像 TUI** (per 用户记忆 #3 + 决策 #33 §2.3 B5): 状态 + 主对话 + 历史 + 设置 + 工具结果, 0 加 0 砍 0 改
- 子方向 2: **状态为主页, 9 organ 拟人化** (per 用户记忆 #5 + 决策 #5 状态为主页): 9 organ 卡片 3x3 + 9 健康环 + heart ECG + brain NN, 0 暴露守门/电子环/工具过程/哲学锚/内部机制
- 子方向 3: **主对话是核心** (per 用户记忆 #3 + 1:1 superpowers 234 5 阶段): 5 阶段 DialoguePhase + user/AI 气泡, 0 假装已接 LLM, stub 诚实标
- 子方向 4: **历史/设置/工具结果 1:1 镜像 TUI** (per 用户记忆 #3 + TUI 6 工具 + 14 setting + 3 kind 1:1 镜像): 严守 0 改
- 子方向 5: **0 暴露 UI 哲学 100%** (per 用户记忆 #3 砍 7 项 + 决策 #33 §2.3 B5): CrossNavStore 0 emit 守门事件, 0 显示 8 哲学锚
- 子方向 6: **9 organ 永远循环 0 死亡** (per 用户记忆 #4 + 用户记忆 #3): 9 organ 活跃度 0-100 永远循环, 0 显示 "已死亡/老化/终止"
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- 0 主动 IM 主人 严守 100% (per gate-discipline)

---

## 6. Tauri 集成 优化 风险 + 异常分支 (per 决策 #33 + 决策 #73 + 决策 #74 + 用户记忆 #6-#7 + #10)

### 6.1 风险 8 维 (per R131-8 §10.1 + R130-3 §6 + 决策 #33 + 决策 #74 + 用户记忆 #6)

**R152-4 风险 8 维 (per R131-8 §10.1 + 决策 #74 B1 + 决策 #33 §2.3 + 用户记忆 #6 0 重复造轮子 + 用户记忆 #7 推技术决策要守规范)**:

- **R1**: V1.1 release 6-12 周 实施时间长 (估 6-12 周 派活 R152-4-1~6) — **缓解**: 蓝图就绪 (per R131-8 §5 6 维度 470 min + R130-3 §2-§4 + R138-7 §2), 派 R152-4-1~6 错开时间盒, 16 跑中上限严守
- **R2**: V1.1 release 实施 跟 V1.0 release 实战冲突 — **缓解**: V1.0 release 实战 = 主人起床后手跑 (估 8/11 06:00-08:00), V1.1 实施 = 估 2026-09-11, 错开
- **R3**: V1.1 release B1 24 LOCKED 入口签名 0 改 但需扩 endpoint + PHL-07 实施 — **缓解**: 仅扩 endpoint, 0 改原 24 LOCKED 入口签名, PHL-07 加 NEW 1 入口 = 25 LOCKED 总数 (per 决策 #22 §1.1-1.2 + 决策 #74 §2.2 B1 V1.1 release Mavis 自决改)
- **R4**: V1.1 release 9 organ 真 sensor 接入 需后端 crate — **缓解**: 已有 core/src/organ.rs 1:1 镜像 (per R129-9 实施), Stage 4 D 蓝图就绪 (per R129-31 §2.5)
- **R5**: V1.1 release WebSocket 长连接 稳定性 — **缓解**: 浏览器 native WebSocket, 0 装 socket.io (per 决策 #33 §2.3 C2)
- **R6**: V1.1 release 跨 tab 持久化 浏览器差异 — **缓解**: 0 装, 浏览器原生 API localStorage + BroadcastChannel (per 决策 #33 §2.3 C2)
- **R7**: V1.1 release 跨平台打包 3 平台 5 bundle format = 15 binary 资源 — **缓解**: 本地 + GitHub Actions (per 主人 8/11 0:43 + R130-3 §2.5)
- **R8**: V1.1 release 借脑 0 装 PASS 严守 vs 不要怕复杂度哲学 冲突 — **缓解**: per 决策 #73 §3 0 装是技术哲学 (严守), 不要怕复杂度是工程哲学 (上限), 0 装是底线, 不要怕复杂度是上限

### 6.2 异常分支 5 维 (per R131-8 + R130-3 + 决策 #33 + 决策 #74)

**R152-4 异常分支 5 维 (per R131-8 + R130-3 + 决策 #33 + 决策 #74 B1 + 用户记忆 #7)**:

- **A1 cargo build fail** (Tauri 2.0 native 编译 fail, per P11-2 §3.3 baseline): **应对**: 0 改 src, 0 借脑 0 装, 蓝图就绪, 0 假装, 派 R152-4-1 续修
- **A2 cargo test fail** (Tauri 2.0 commands 重复定义 bug, per P11-2 §3.5 9 submod workaround): **应对**: 27 commands 拆 9 submod 0 改, 0 借脑 0 装, 派 R152-4-1 续修
- **A3 Tauri 跨平台打包 fail** (cargo tauri build Windows/macOS/Linux 5 bundle format = 15 binary fail): **应对**: 0 装新 lib, 0 借脑 0 装, 派 R152-4-5 续修
- **A4 5 nav 真打通 fail** (CrossNavStore 7 模块 J1-J7 0 调 tauriInvoke): **应对**: 0 加 0 砍 0 改 NAV_ID 0-4, CrossNavStore 1 真相源, 派 R152-4-2 续修
- **A5 9 organ 真 sensor fail** (core/src/organ.rs 0 改, Stage 4 D 真 sensor 接入 fail): **应对**: 0 改 organ_id 0-8, ticker.js 100ms 周期 永远循环 0 死亡, 派 R152-4-3 续修

### 6.3 决策原则 22 维 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §3 + 用户记忆 #1-#10)

**R152-4 决策原则 22 维 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §3 + 用户记忆 #1-#10 + 决策 #78 整合 #5.3 done)**:

- **D1**: Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **D2**: 整合 #7 Tauri 集成优化准备 (实施 spec 8 维度 详细) (per R152-4 任务 spec + 决策 #86 §4 R152 era 派活拍板)
- **D3**: 8 维度 实施 spec (Tauri 2.0 完整集成 + 5 nav 完整 + 9 organ 拟人化 final + Stage 4-8 实战 + Tauri 跨平台 + Tauri 性能 + Tauri 借脑 + Tauri PHL-07 集成) (per R131-8 §5 6 维度 蓝图 + R130-3 §2-§4 + R138-7 §2 拓维)
- **D4**: 5 关系 详写 (Rust 后端 / 5 nav / 9 organ / ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 8 哲学锚 / 用户记忆 #3) (per R152-4 §3-§5)
- **D5**: 6 子方向 派活计划 (R152-4-1 ~ R152-4-6, 估 6-12 周 实施) (per R152-4 §2.1 总览 + 决策 #86 §4)
- **D6**: 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **D7**: B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74 §2.2-§2.3)
- **D8**: B2 workspace.version 1.2.0 V1.0 release 严守 + V1.1 release bump 1.2.1 (per 决策 #74 §1 B2)
- **D9**: A1 R11 baseline 3 值 V1.0 release 严守 + V1.1 release R12 更高 (per 决策 #74 §2.2)
- **D10**: A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (per 决策 #74 §1 A3 + R129-11 关键诚实标 + R137-1 PHL-07 实施)
- **D11**: B3 V0.5 30 维 V1.0 release + V1.1 release 严守 (per 决策 #33 §2.3 B3)
- **D12**: B4 6 重守门 v7 V1.0 release + V1.1 release 严守 (per 决策 #33 §2.3 B4)
- **D13**: B5 8 哲学锚 V1.0 release + V1.1 release 严守 (per 决策 #33 §2.3 B5)
- **D14**: C1 0 主动 commit (主人起床前) 严守 (per 决策 #33 §2.3 C1)
- **D15**: C2 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- **D16**: 0 主动 push (主人起床前) 严守 100% (per 决策 #33 + 决策 #61 §6 + 决策 #78 §3)
- **D17**: 总工程哲学扩展 "不要怕复杂度" (per 决策 #73 §3 + 哲学文档 15)
- **D18**: 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)
- **D19**: 决策日志写 (per 决策 #10 + 用户记忆 #10)
- **D20**: 0 重复造轮子 (per 用户记忆 #6, R131-8 + R130-3 + R129-19 + R129-9 + R130-6 + R133-1/2/3 + R137-1~5 + R138-6/7 + R151-2 + 哲学文档 15 reference 不重写)
- **D21**: TUI 跟 Tauri 升级路径一致 (per 决策 #9 + 用户记忆 #8 + 用户记忆 #9 瘦客户端, TUI/Tauri 1:1 翻译, 后端 API 表面 0 改)
- **D22**: 9 organ 永远循环 0 死亡 (per 用户记忆 #4, ticker.js 100ms 周期, 活跃度 0-100 永远循环, 0 用 health/sick/dying)

---

## 7. Tauri 集成 优化 测试 (cargo test + tauri dev + tauri build 8 步 verify) (per 决策 #11 + 决策 #78 §2.3 + R147-1 1.0 release 实战 8 步 + R129-3 8 步 verify)

### 7.1 8 步 verify 流程 (per 决策 #11 + 决策 #78 §2.3 + R147-1 + R129-3 8 步 verify 流程)

**Tauri 集成 优化 8 步 verify 流程 (per 决策 #11 + 决策 #78 §2.3 + R147-1 1.0 release 实战 8 步 + R129-3 8 步 verify 流程)**:

| 步骤 | 任务 | 估时 | Mavis 角色 | 主人手跑 | 8 硬墙严守 |
|:---:|------|-----:|-----------|----------|-----------|
| **Step 1** | **整合 #7.1 src/ 拍板 拍板准备** (Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+, per R138-7 §2 + R152-4 8 维度) | 5 min (估 11/29 06:00) | Mavis 自决拍板 (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **Step 2** | **cargo test 严守** (122 tests pass + 集成层 79 tests + Stage 4-7 NEW tests 累计 600+ tests, 0 改 24 LOCKED 入口签名) | 30 min (估 11/29 06:30) | Mavis 自决 verify (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **Step 3** | **cargo tauri dev 跑通** (Tauri 2.0 binary 启动, 5 nav 切换, 9 organ ticker 100ms 周期, 永远循环 0 死亡) | 15 min (估 11/29 06:45) | Mavis 自决 verify (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **Step 4** | **cargo tauri build 跨平台 3 平台 5 bundle format** (Windows MSI/NSIS + macOS DMG/APP + Linux deb/AppImage = 15 binary, 0 warning 0 error) | 30 min (估 11/29 07:15) | Mavis 自决 verify (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **Step 5** | **整合 #7 commit 拍板 verify** (7.1 src/ + 7.2 docs/ + 7.3 reports/ 顺序 git add + git commit, master HEAD 衔接整合 #6 commit) | 5 min (估 11/29 07:20) | Mavis 自决拍板 (per 决策 #33 C1 + 决策 #74 B1) | 0 | ✅ 0 越界 |
| **Step 6** | **V1.1 release 实战准备** (整合 #7 commit 拍板后 → 7 步 runbook 续, 配 GitHub remote + git push + git tag v1.1.0 + git push --tags + GitHub Release 创建 v1.1.0 + V1.1 release 实战 done verify) | 5 min (估 11/29 07:25) | Mavis 自决 (per 决策 #33 C1 + 决策 #61 §6) | 0 | ✅ 0 越界 |
| **Step 7** | **0 主动 push 严守 verify** (0 主动 push, 等 1.0 release 配 GitHub remote + 主人起床后手跑) | 0 min (Mavis 0 主动 push 严守) | Mavis 0 主动 push (per 决策 #33 + 决策 #61 §6) | 主人手跑 (8/11 06:00-08:00 + 11/30 06:00-08:00) | ✅ 0 越界 |
| **Step 8** | **V1.1 release 实战 done verify + 决策链 #131 spec** (V1.1 release tag v1.1.0 打上, GitHub Release 创建, GitHub Pages 重新部署, 0 主动 push 严守 100%) | 5 min (估 11/30 06:35) | Mavis verify (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **总时间盒** | **8 步 verify 流程** | **90 min** (估 11/29-11/30 6-7 hours) | Mavis 0 主动 push 严守 100% | 7 步 全部 主人手跑 | ✅ 100% |

**8 步 verify 跟 R129-3 8 步 verify + R147-1 1.0 release 实战 8 步 关系 (per 决策 #11 + 决策 #78 + R147-1)**:
- ✅ R129-3 8 步 verify: cargo test + cargo build + cargo clippy + cargo fmt + cargo deny + cargo audit + cargo doc + cargo run
- ✅ R147-1 1.0 release 实战 8 步: Step 1 整合 #5.1/5.2/5.3 commit done verify → Step 2 主人配 GitHub remote → Step 3 主人 git push → Step 4 主人 git tag v1.0.0 → Step 5 主人 release notes → Step 6 主人 GitHub Pages → Step 7 1.0 release done verify → Step 8 V1.1 release 永久循环接续
- ✅ R152-4 8 步 verify (Tauri V1.1 release 实战 8 步): Step 1-8 (per 上表)

### 7.2 测试 8 维度 (per R131-8 + R130-3 + 决策 #33 §2.3 + 决策 #74 B1)

**Tauri 集成 优化 测试 8 维度 (per R131-8 + R130-3 + 决策 #33 §2.3 + 决策 #74 B1)**:

| 测试维度 | 内容 | 估 NEW tests | 8 硬墙严守 | 0 装 PASS 严守 |
|:---:|------|------------:|:---:|:---:|
| **1 cargo test 单元测试** (per 决策 #33 §2.3 + R129-9 §8.1) | 122 tests + Stage 4-7 NEW tests 累计 600+ tests | 600+ | ✅ 0 越界 | ✅ 0 装 |
| **2 集成层 test (Node.js 跑 JS 测试)** (per R129-19 §9.3 + R130-3 §4) | 79 cases + Stage 4-7 NEW tests 累计 200+ tests | 200+ | ✅ 0 越界 | ✅ 0 装 |
| **3 cargo tauri dev 跑通** (per P11-2 §3.4 + 决策 #33 §2.3) | Tauri 2.0 binary 启动, 5 nav 切换, 9 organ ticker 100ms 周期 | 0 NEW (binary verify) | ✅ 0 越界 | ✅ 0 装 |
| **4 cargo tauri build 跨平台 3 平台** (per R130-3 §2.5 + 决策 #33 §2.3 C2) | Windows MSI/NSIS + macOS DMG/APP + Linux deb/AppImage = 15 binary | 0 NEW (binary verify) | ✅ 0 越界 | ✅ 0 装 |
| **5 cargo test 0 越界 verify** (per 决策 #33 §2.3 B1) | 0 改 24 LOCKED 入口签名, 0 触碰 24 LOCKED crate mtime baseline 16:34 之前 | 0 NEW (verify) | ✅ 0 越界 | ✅ 0 装 |
| **6 R11 baseline 3 值 0 改 verify** (per 决策 #33 §2.1 A1) | 0.8682/0.8532/0.9063 严守, V1.1 release 0 触碰 integration_r_measure.rs | 0 NEW (verify) | ✅ 0 越界 | ✅ 0 装 |
| **7 0 装 PASS 严守 6 维度 verify** (per 决策 #33 §2.3 C2) | 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" / 0 借脑 0 装 / 0 cargo install / 0 cargo add | 0 NEW (verify) | ✅ 0 越界 | ✅ 0 装 |
| **8 8 哲学锚 + 7 项 UI 哲学 砍 verify** (per 决策 #33 §2.3 B5 + 用户记忆 #3) | 0 暴露 UI 哲学, 0 显示 "已死亡/老化/终止", CrossNavStore 0 emit 守门事件 | 0 NEW (verify) | ✅ 0 越界 | ✅ 0 装 |
| **总** | **8 维度 测试** | **800+ NEW tests 累计** | ✅ 0 越界 100% | ✅ 0 装 100% |

### 7.3 V1.1 release 实战 7 步 runbook (整合 #7 commit 拍板后, 主人起床后手跑, 估 2026-11-30)

**V1.1 release 实战 7 步 runbook (整合 #7 commit 拍板后, 主人起床后手跑, 估 2026-11-30)** (per R138-7 §6 + R134-4 §2 续 + 决策 #78 §3 + 决策 #61 §6):

| Step | 任务 | 估时 | Mavis 角色 | 主人手跑 | 8 硬墙严守 |
|:---:|------|-----:|-----------|----------|-----------|
| **Step 1** | 整合 #6 + #7 commit 拍板 verify (3 commit hash + master HEAD 新值) | 5 min (估 11/30 06:05) | Mavis 自决拍板 (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **Step 2** | 主人起床后配 GitHub remote | 5 min (估 11/30 06:10) | 0 主动 push (per 决策 #33 C1) | 主人手跑: `git remote add origin https://github.com/...` | ✅ 0 越界 |
| **Step 3** | 主人手跑 git push | 5 min (估 11/30 06:15) | 0 主动 push (per 决策 #33 C1) | 主人手跑: `git push -u origin master` | ✅ 0 越界 |
| **Step 4** | 主人手跑 git tag v1.1.0 | 5 min (估 11/30 06:20) | 0 主动 tag (per 决策 #33 C1) | 主人手跑: `git tag -a v1.1.0 -m "..."` | ✅ 0 越界 |
| **Step 5** | 主人手跑 git push --tags | 5 min (估 11/30 06:25) | 0 主动 push (per 决策 #33 C1) | 主人手跑: `git push --tags` | ✅ 0 越界 |
| **Step 6** | 主人手跑 GitHub Release 创建 v1.1.0 | 10 min (估 11/30 06:35) | 0 主动 release (per 决策 #33 C1) | 主人手跑 GitHub UI | ✅ 0 越界 |
| **Step 7** | V1.1 release 实战 done verify + 决策链 #131 spec | 5 min (估 11/30 06:40) | Mavis verify (per 决策 #33 C1) | 0 | ✅ 0 越界 |
| **总时间盒** | V1.1 release 实战 7 步 runbook | **40 min** (估 11/30 06:40 done) | Mavis 0 主动 push/tag/release 严守 | 7 步全部主人手跑 | ✅ 100% |

**V1.1 release 实战 0 主动 push 严守 100%** (per 决策 #33 C1 + 决策 #61 §6 + 决策 #74 §1 + 决策 #78 §3):
- Mavis 0 主动 git push
- Mavis 0 主动 git tag
- Mavis 0 主动 GitHub Release
- 全部等主人起床后手跑

---

## 8. Tauri 集成 优化 实施 spec 派活计划 (per R152-4 8 维度 + 决策 #86 §4 R152 era 派活 + 决策 #71 §2 永久循环接续)

### 8.1 6 子方向 派活计划 (per R152-4 8 维度 + 决策 #86 §4)

**6 子方向 派活计划 (per R152-4 8 维度 + 决策 #86 §4 R152 era 派活 + 决策 #71 §2 永久循环接续)**:

| Sub-agent | 实施 spec | 实施子方向 | 估时 | 估 NEW tests | 8 硬墙严守 |
|-----------|----------|----------|-----:|------------:|:---:|
| **R152-4-1** | **Tauri 2.0 完整集成** (per 维度 1) | 27 → 36 commands + 5 capabilities + 多窗口 + tray + menu + updater | 60 min | 18 | ✅ 0 越界 |
| **R152-4-2** | **5 nav 完整集成** (per 维度 2) | 5 nav 真打通 + 7 模块 J1-J7 + CrossNavStore + tauriInvoke | 90 min | 120 | ✅ 0 越界 |
| **R152-4-3** | **9 organ 拟人化 final** (per 维度 3) | heart ECG + brain NN + 9 健康环 + 永远循环 + 真 sensor 14 + PHL-07 集成 | 120 min | 64+168 = 232 | ✅ 0 越界 |
| **R152-4-4** | **Stage 4-8 实战路线** (per 维度 4) | Stage 4 4 维度 84 + Stage 5 30 + Stage 6 30 + Stage 7 跨平台 + Stage 8 用户测试 | 120 min | 144 | ✅ 0 越界 |
| **R152-4-5** | **Tauri 跨平台** (per 维度 5) | 5 bundle format + Tauri 2.0 updater + GitHub Actions CI | 90 min | 0 NEW (cargo tauri build 3 平台 PASS) | ✅ 0 越界 |
| **R152-4-6** | **Tauri 性能** (per 维度 6) | WebSocket chunk append + 9 organ 真 sensor + 跨 tab 持久化 | 90 min | 49 | ✅ 0 越界 |
| **(R152-4-3 协同)** | **Tauri 借脑 (5 借脑 0 装)** (per 维度 7) | Tauri 2.0 + superpowers 234 + langgraph 829 + servers 1.4MB + kani 5.5MB 1:1 翻译 | (60 min, 嵌入 R152-4-1~6) | 0 NEW (蓝图就绪 0 改) | ✅ 0 越界 |
| **(R152-4-3 协同)** | **Tauri PHL-07 集成** (per 维度 8) | PHL-07 impl + 14 维主对话锚 + 8 哲学锚 + 6 重 v7 + 14 键 verdict cache | (90 min, 嵌入 R152-4-3) | 168 (R152-4-3 累计 232) | ✅ 0 越界 |
| **总** | **6 sub-agent 派活** | **8 维度 实施 spec** | **~620 min (估 6-12 周 实施)** | **~600 NEW tests 累计 集成层 79 + 84 = 163 tests + cargo 122 tests = 285 tests + 600 = 885 tests** | ✅ 0 越界 100% |

### 8.2 6-12 周 实施 时间盒 (per R152-4-1~6 + 决策 #71 §2 永久循环接续 + 决策 #74 B1)

**6-12 周 实施 时间盒 (per R152-4-1~6 + 决策 #71 §2 永久循环接续 + 决策 #74 B1)**:

```
[8/11 R152-4 done 整合 #7 Tauri 集成优化准备 实施 spec 报告]  ✅ Mavis 拍板
   ↓
[8/12 - 11/24 派活 R152-4-1~6 实施 sub-agent 跑中 (per 决策 #71 §2 永久循环接续)]
   R152-4-1 Tauri 2.0 完整集成 (1 周, 8/12-8/18)
   R152-4-2 5 nav 完整集成 (1 周, 8/19-8/25)
   R152-4-3 9 organ 拟人化 final (2 周, 8/26-9/8) + PHL-07 集成 (per 维度 8)
   R152-4-4 Stage 4-8 实战路线 (2 周, 9/9-9/22)
   R152-4-5 Tauri 跨平台 (1 周, 9/23-9/29)
   R152-4-6 Tauri 性能 (1 周, 9/30-10/6)
   ↓
[10/7 - 11/24 R152-4-1~6 续 整合 + 集成测试 + 8 步 verify (per 决策 #11 + 决策 #78 §2.3)]
   ↓
[11/25 整合 #6 commit 拍板]  Mavis 自决 (per 决策 #33 C1 + 决策 #74 B1)
   ↓
[11/26-28 整合 #7 commit 拍板准备 5 阶段计划 续 (per R134-4 + R138-7)]
   阶段 1: 7.1 src/ 拍板准备续 (2 周, 11/26-12/09, 但实际 11/26-11/28 准备, 5 阶段计划时序需调整)
   阶段 2: 7.2 docs/ 拍板准备续 (1 周, 11/29 准备, 实际 1 day)
   阶段 3: 7.3 reports/ 拍板准备续 (1 周, 11/29 准备, 实际 1 day)
   阶段 4: 整合 #7 commit 拍板续 (1 day, 11/29 实际)
   阶段 5: V1.2 minor release 实战 准备 (1 day, 11/30 实际)
   ↓
[11/29 整合 #7 commit 拍板]  Mavis 自决 (per 决策 #33 C1 + 决策 #74 B1 + 决策 #62 整合 #5 commit 3 commit 类比)
   ↓
[11/30 V1.1 release 实战 7 步 runbook]  主人起床后手跑 7 步
   ↓
[V1.1 release tag v1.1.0 打上]  GitHub release + GitHub Pages 重新部署
   ↓
[V1.1 release 实战完]  V1.2 minor release 准备 (per R131-3 永久循环 + 决策 #74 §2.3)
```

### 8.3 派活 4 阶段 (per 决策 #71 §2 永久循环 + 决策 #86 §4 R152 era 派活)

**派活 4 阶段 (per 决策 #71 §2 永久循环 + 决策 #86 §4 R152 era 派活)**:

- **阶段 1: 调研** (per 决策 #71 §2 + R130 era 调研 6 sub-agent + R131 era 调研 9 sub-agent)
  - R131-7 pybridge 集成优化 (per 决策 #75 §2.1)
  - R131-8 Tauri 集成优化 (per 决策 #75 §2.1)
  - R131-9 形式化集成优化 (per 决策 #75 §2.1)
  - R133-1 借鉴源 12 源 实施 (per 决策 #75 §2.1)
  - R133-2 ASI Stage 9 长程 AI 成长 (per 决策 #75 §2.1)
  - R133-3 三洋葱架构升级 (per 决策 #75 §2.1)
  - R152-4 整合 #7 Tauri 集成优化准备 (per 决策 #86 §4) — **本报告**

- **阶段 2: 差距** (per 决策 #71 §2 + R131 era 差距)
  - R131-1 现有架构总审视 (per 决策 #73 §3.2)
  - R131-2 跟借鉴源码 11 源差距 (per 决策 #73 §3.2)
  - R131-3 V1.1 release 实施路线图 (per 决策 #73 §3.2)

- **阶段 3: 计划** (per 决策 #71 §2 + R132 era 计划)
  - R132-1 V1.1 release 路线图 final (per 决策 #71 §2.4)
  - R132-2 Tauri Stage 6 后端接通 (per 决策 #71 §2.4)

- **阶段 4: 实施** (per 决策 #71 §2 + R133 era 实施 + R137 era 续 + R138 era 综合 + R152 era 实施 spec 准备)
  - R133-1 借鉴源 12 源 实施 (per 决策 #75 §2.1)
  - R133-2 ASI Stage 9 长程 AI 成长 (per 决策 #75 §2.1)
  - R133-3 三洋葱架构升级 (per 决策 #75 §2.1)
  - R137-1 PHL-07 实施 spec (per 决策 #77 §3.1)
  - R137-2 24 LOCKED 入口签名 改写 spec (per 决策 #77 §3.1)
  - R137-3 Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #77 §3.1)
  - R137-4 ASI Stage 9 实战 (per 决策 #77 §3.1)
  - R137-5 形式化 Stage 5.5+ 实战 (per 决策 #77 §3.1)
  - R138-6 整合 #6 commit 拍板实战 (per 决策 #78 + R134-3 续)
  - R138-7 整合 #7 commit 拍板实战续 (per R134-4 续)
  - **R152-4 整合 #7 Tauri 集成优化准备 (实施 spec)** (per 决策 #86 §4) — **本报告**

- **永久循环接续** (per 决策 #71 §2 永久循环, 0 终点): R152 era → R153+ era → ... 0 终点

### 8.4 派活 4 维 verify (per 决策 #86 §4 + 决策 #71 §2 + 决策 #74 B1)

**派活 4 维 verify (per 决策 #86 §4 + 决策 #71 §2 永久循环 + 决策 #74 B1)**:

- **派活 verify 1**: 0 改 src 严守 100% (R152-4 报告 0 触碰主仓 src/, 0 改 Cargo.toml, 仅写 reports/agent-r152-4-*.md)
- **派活 verify 2**: 0 借脑 0 装 严守 100% (R152-4 0 装 "已读真源码" / 0 装 "已集成" / 0 装 "已 fork" / 0 cargo install / 0 cargo add)
- **派活 verify 3**: 8 硬墙 0 越界 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #74 B1 V1.1 release Mavis 自决改 仅扩 endpoint, 0 改原 24 LOCKED 入口签名)
- **派活 verify 4**: 决策链 + 借鉴 12 源 + 8 哲学锚 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §3 + 决策 #10 + 用户记忆 #10)

### 8.5 派活 跟 整合 #6 + #7 commit 拍板 衔接 (per 决策 #62 + 决策 #78 + R134-4 + R138-7 + R152-4)

**派活 跟 整合 #6 + #7 commit 拍板 衔接 (per 决策 #62 + 决策 #78 + R134-4 + R138-7 + R152-4)**:

```
[8/11 R152-4 done 整合 #7 Tauri 集成优化准备 实施 spec 报告]  ✅ Mavis 拍板
   ↓
[8/12 - 11/24 6-12 周 派活 R152-4-1~6 实施 sub-agent]  实施 spec 8 维度 详细
   ↓
[11/25 整合 #6 commit 拍板]  6.1 src/ 拍板 + 6.2 docs/ 拍板 + 6.3 reports/ 拍板
   Mavis 自决 (per 决策 #33 C1 + 决策 #74 B1 V1.1 release Mavis 自决改)
   整合 #6 commit 拍板 = 24 LOCKED 入口签名 改写 + PHL-07 实施 + 后端加固
   ↓
[11/26-28 整合 #7 commit 拍板准备 5 阶段计划 续]  per R134-4 + R138-7
   ↓
[11/29 整合 #7 commit 拍板]  7.1 src/ 拍板 + 7.2 docs/ 拍板 + 7.3 reports/ 拍板
   Mavis 自决 (per 决策 #33 C1 + 决策 #74 B1 + 决策 #62 整合 #5 commit 3 commit 类比)
   整合 #7 commit 拍板 = Tauri Stage 5+ + ASI Stage 8+ 续 + 形式化 Stage 5.5+ 续 + 三洋葱架构升级 续
   整合 #7.1 commit = Tauri Stage 5+ 实施 (per R152-4 8 维度 续, R152-4-1~6 实施 sub-agent 续)
   ↓
[11/30 V1.1 release 实战 7 步 runbook]  主人起床后手跑 7 步
   ↓
[V1.1 release tag v1.1.0 打上]  GitHub release + GitHub Pages 重新部署
   ↓
[V1.1 release 实战完]  V1.2 minor release 准备 (per R131-3 永久循环 + 决策 #74 §2.3)
```

---

## 9. 8 硬墙严守 verify (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表)

### 9.1 8 硬墙 严守 verify 表 (per 决策 #33 §2.3 + 决策 #74 §1)

**8 硬墙 严守 verify 表 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #74 B1 V1.1 release Mavis 自决改)**:

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 严守 | R152-4 verify | 0 装 PASS 严守 |
|---|--------|-------------------|-------------------|---------------|:---:|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (24 → 25 LOCKED, 仅扩 endpoint, 0 改原 24 LOCKED 入口签名) | ✅ 0 改 (R131-5 verify 24/24 100% PASS) | ✅ 0 装 |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 (per 决策 #74 B2) | ✅ 0 改 | ✅ 0 装 |
| **A1** | R11 baseline 3 值 (0.8682/0.8532/0.9063) | 🔒 严守 | 🔒 严守 (哲学 + 效果标) | ✅ 0 改 | ✅ 0 装 |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 | 🟡 PHL-07 实施 (24 → 25 LOCKED + 13 → 14 键) | ✅ PHL-07 V1.0 spec-only 0 实施 (V1.1 实施) | ✅ 0 装 |
| **B3** | V0.5 30 维 | 🔒 严守 | 🔒 严守 (哲学) | ✅ 0 改 | ✅ 0 装 |
| **B4** | 6 重守门 v7 | 🔒 严守 | 🔒 严守 (哲学) | ✅ 0 改 | ✅ 0 装 |
| **B5** | 8 哲学锚 | 🔒 严守 | 🔒 严守 (哲学) | ✅ 0 改 | ✅ 0 装 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 100% | 🔒 严守 (V1.1 release 整合 #6 + #7 commit Mavis 自决拍板) | ✅ 0 主动 commit (Mavis 拍板) | ✅ 0 装 |
| **C2** | 0 装 PASS 严守 | 🔒 0 装 严守 (技术哲学) | 🔒 严守 (技术哲学, 借脑 0 借具体源码) | ✅ 0 装 | ✅ 0 装 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 (V1.1 release 实战 7 步 runbook) | ✅ 0 主动 push (Mavis 0 主动 push) | ✅ 0 装 |
| **总工程哲学 "不要怕复杂度"** | 🟢 新增 (per 决策 #73 §3 + 主人 01:14 拍板 3 件套 §3) | 🟢 V1.1 release 实施 6-12 周 8 维度 (最强效果 + 最厉害工程) | ✅ 落地 | ✅ 0 装 |

### 9.2 R152-4 报告本身 0 触碰主仓 verify (per 决策 #33 + 决策 #74 + 用户记忆 #10)

**R152-4 报告本身 0 触碰主仓 verify (per 决策 #33 + 决策 #74 + 用户记忆 #10 + 决策 #10)**:

```bash
# 假设跑 (R152-4 实际 0 跑, 仅规划 spec):
$ cd Apeireth-rust
$ git status --porcelain
# 仅显示:
# ?? reports/agent-r152-4-integration-7-tauri-integration-optimize-prep-2026-08-11.md
# (0 触碰主仓 src/, 0 触碰 frontend/, 0 触碰 Cargo.toml)
```

**R152-4 报告 0 触碰 verify**:
- ✅ 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改, 0 触碰 27 commands)
- ✅ 0 改 core/ (24 LOCKED 0 改, 0 触碰 122 tests pass)
- ✅ 0 改 主仓 src/ (workspace.version 1.2.0 0 改, 0 触碰 24 LOCKED crate lib.rs 入口签名)
- ✅ 0 改 主仓 Cargo.toml (0 改 0 触碰, B2 1.2.0 严守, V1.1 release 才 bump 1.2.1)
- ✅ 0 改 frontend/ (0 触碰 5 nav + 9 organ + 14 settings, 0 触碰 122 tests pass + 79 集成层 tests)
- ✅ 0 借脑 0 装 (仅规划, 0 触碰借鉴源码本身)
- ✅ 0 主动 commit (整合 #7.3 reports/ 由 Mavis 拍板, 0 主动 push 严守 100%)
- ✅ 0 主动 push (V1.0 release 实战 + V1.1 release 实战 7 步 runbook 后)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

### 9.3 R152-4 跟 决策链 + 借鉴 12 源 + 8 哲学锚 严守 verify (per R148-12 v3 总索引)

**R152-4 跟 决策链 #30-#86 严守 verify (per R148-12 v3 总索引 57 决策)**:
- ✅ 决策 #10 (决策日志): R152-4 报告写入 reports/, 决策日志写 (per 用户记忆 #10)
- ✅ 决策 #22 (24 LOCKED + semver): 24 LOCKED 0 改, semver 严守
- ✅ 决策 #33 (8 硬墙 + 0 装 PASS): 8 硬墙 0 越界 100%, 0 装 PASS 严守 100%
- ✅ 决策 #48 (整合 #4 commit abf12243): master HEAD 严守 100%
- ✅ 决策 #55 (9 阶段路线图 + 24 LOCKED + 借鉴 ID 严格化): 0 改
- ✅ 决策 #57-#58 (R128 阶段 B Tauri prototype 派活): P11-1/2 baseline 0 改
- ✅ 决策 #61 (新会话接手 + R129 era 派活规划): R152-4 在 mvs_367e66fae08342ffa399befe4f85dbac 跑
- ✅ 决策 #62 (整合 #5 commit 拆 3 commit): 整合 #7 commit 拍板续 3 commit 类比
- ✅ 决策 #64 (5 min tick cron 自动监督): R152-4 done 后 cron tick 派活
- ✅ 决策 #71 (4 步永久循环接续): R152-4 调研 → 实施 spec 阶段
- ✅ 决策 #72 (R130 era 调研 6 sub-agent): R130-3 Tauri Stage 5 集成深化 reference
- ✅ 决策 #73 (主人 8/11 01:14 拍板 3 件套): R152-4 8 维度 实施 spec 严守不要怕复杂度哲学
- ✅ 决策 #74 (8 硬墙 B1 改写): V1.1 release Mavis 自决改, 仅扩 endpoint, 0 改原 24 LOCKED 入口签名
- ✅ 决策 #75-#77 (R131-R137 era 派活): R131-8 + R130-3 + R137-TAURI 续 reference
- ✅ 决策 #78 (整合 #5.3 commit 拍板 Option A, 1:43 done): master HEAD = 4207f187 严守
- ✅ 决策 #79-#86 (R138-R148 era 派活): R138-6/7 + R151-2 + R152-4 续

**R152-4 跟 借鉴 12 源 严守 verify (per R133-1 + R130-6 + 决策 #33 §2.2 + 决策 #73 §2.2)**:
- ✅ 8 真 cloned 0 装 (clap 3.50MB + hyper 0.54MB + servers 1.40MB + PyO3 5.69MB + kani 5.46MB + langgraph 13.29MB + superpowers 1.52MB + Guardrails 18.19MB = 49.59MB / 7,764 files)
- ✅ 2 借鉴 ID 索引完成 0 装 (LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块)
- ✅ 1 永久跳过 0 装 (OpenCog AGPL-3.0 主仓 ID-011)
- ✅ 1 借脑 ID 索引完成 0 装 (OpenCog 家族 6 子源, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")

**R152-4 跟 8 哲学锚 严守 verify (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项 UI 哲学)**:
- ✅ S-1 服务 ASI 北极星: 0 暴露 UI, 0 假装已接 ASI (Tauri 0 暴露 UI 哲学)
- ✅ S-2 实事求是: 0 假装已实施, stub 诚实标 (Tauri 0 假装已接 LLM)
- ✅ S-3 质量工程化: 122 tests pass + cargo build PASS + 0 warning 0 error
- ✅ O-1 安全优先: 24 LOCKED 入口签名 0 改, 8 硬墙 0 越界
- ✅ O-2 走在前人经验上: 借脑 0 装 8 借鉴源 + Tauri 2.0 native 真实施
- ✅ O-3 干到底: cargo tauri build 0 改 0 越界
- ✅ O-4 任何人都能接手: README + STRUCTURE + 8 硬墙 0 越界 verify
- ✅ O-5 不假装: 9 organ 全 Stub readiness + AI 回复 = stub + 5 鉴权 disabled + 5 Provider model_count=0

**R152-4 跟 9 件套 总哲学 verify (per 15-no-fear-complexity.md §2)**:
- ✅ 8 哲学锚 (思想): S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5
- ✅ 不要怕复杂度 (工程): 最强效果 + 最厉害工程 + 维护交给未来高水平团队
- ✅ 9 件套 总哲学 = 完整思想 + 工程边界

**R152-4 跟 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 verify (per 15-no-fear-complexity.md §3)**:
- ✅ 8 硬墙 (底线, 不可破): 严守 (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 不要怕复杂度 (上限, 可超): Mavis 自决架构升级 (per 决策 #73 §1 + 决策 #74 §2)

### 9.4 R152-4 跟 永久循环 4 步 verify (per 决策 #71 §2 永久循环接续)

**R152-4 跟 永久循环 4 步 verify (per 决策 #71 §2 永久循环接续)**:

- **调研 R130** (R130 era 6 sub-agent): R130-1 ~ R130-6 ✅ done, R152-4 reference R130-3 Tauri Stage 5 集成深化
- **差距 R131** (R131 era 9 sub-agent): R131-1 ~ R131-9 ✅ done, R152-4 reference R131-8 Tauri 集成优化
- **计划 R132** (R132 era 2 sub-agent): R132-1 V1.1 release 路线图 final + R132-2 Tauri Stage 6 后端接通 ✅ done
- **实施 R133+** (R133 era 3 sub-agent + R134 era 6 sub-agent + R135 era 2 sub-agent + R136 era 1 sub-agent + R137 era 5 sub-agent + R138 era 13 sub-agent + R139 era 1 sub-agent + R140 era 14 sub-agent + R141 era 3 sub-agent + R142 era 1 sub-agent + R143 era 2 sub-agent + R144 era 4 sub-agent + R145 era 3 sub-agent + R146 era 3 sub-agent + R147 era 5 sub-agent + R148 era 6 sub-agent + **R152 era 5 sub-agent (R152-1 ~ R152-5) ✅**): R152-4 = R152 era 实施 spec 准备 sub-agent, 跟其他 R152-1/2/3/5 协同, 0 重复造轮子
- **综合 R148-12 v3 总索引** (R148-12 决策链 + 借鉴 + 8 硬墙 v3): R152-4 报告 0 重复造轮子, 跟 R148-12 v3 总索引对齐
- **永久循环接续**: R152 era → R153+ era → ... 0 终点 (per 决策 #71 §2 永久循环)

---

## 10. refs (前置报告 + 决策 + 哲学文档 + 用户记忆)

### 10.1 前置报告 (per 决策 #71 §2 永久循环接续 + 决策 #75-#77 + 决策 #86 §4 R152 era 派活拍板)

**Tauri 5 阶段报告 (per 决策 #75 + R130-3 + R130-5)**:
- P11-1 R128 tauri-frontend-prototype-final-2026-08-10: Tauri 2.0 prototype 真实施 (72 tests)
  - `reports/agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md`
- P11-2 R128-2 tauri-frontend-scaffold-final-2026-08-10: Tauri 2.0 scaffold 深化 (111 tests, cargo build PASS + cargo tauri dev 跑通)
  - `reports/agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md`
- R129-9 Tauri Stage 2 深化 (2026-08-11 00:35): 5 phase 进度条 + 流式打字 + 9 健康环 + heart ECG + brain NN + 122 tests
  - `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md`
- R129-19 Tauri Stage 3 跨 nav 集成 (2026-08-11 00:34): 7 模块 J1-J7 + CrossNavStore 状态中枢 + 9 organ animator + 79 tests + 8 examples + 1 hub
  - `reports/agent-r129-19-tauri-stage-3-integration-2026-08-11.md`
- R129-31 Tauri Stage 4 实战规划 (2026-08-11 00:56): 4 维度 A 真后端 / B WebSocket / C 持久化 / D 真 sensor 蓝图 + 84 NEW tests 累计 163
  - `reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md`
- R130-3 Tauri Stage 5 集成深化 (2026-08-11 1:00): Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + Stage 6+ 路线 + V1.1 计划
  - `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md`
- R130-5 V1.1 minor release 路线图 (2026-08-11 01:14): 6 大方向 (PHL-07 + 后端加固 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+ + 借鉴源 12 源)
  - `reports/agent-r130-5-v1.1-minor-release-roadmap-2026-08-11.md`
- R130-6 Final Report 借鉴源码 12 源调研 (2026-08-11 01:14): 11 已有 + 1 新增 = OpenCog AGPL-3.0 fork 决策
  - `reports/agent-r130-6-borrowed-12-sources-research-2026-08-11.md`
- R131-3 V1.1 release 实施路线图 (2026-08-11 01:14): 6 大方向 (PHL-07 + 24 LOCKED 改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)
  - `reports/agent-r131-3-v1.1-release-implementation-roadmap-2026-08-11.md`
- R131-7 pybridge 集成优化 (per 决策 #75 §2.1): pybridge 集成优化, 886/886 pybridge tests 严守
  - `reports/agent-r131-7-pybridge-integration-optimization-2026-08-11.md`
- **R131-8 Tauri 集成优化 (per 决策 #75 §2.1)**: 9 优化方向 + V1.1/V2.0 完整方案 + 6 维度 470 min 蓝图
  - `reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md` (96 KB, **R152-4 reference 不重写**)
- R131-9 形式化集成优化 (per 决策 #75 §2.1): 9 优化方向
  - `reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md`
- R132-1 V1.1 release 路线图 final (per 决策 #71 §2.4): 6 大方向 详细 spec
  - `reports/agent-r132-1-v1.1-release-roadmap-final-2026-08-11.md`
- R132-2 Tauri Stage 6 后端接通 (per 决策 #71 §2.4): Tauri Stage 6 后端接通
  - `reports/agent-r132-2-tauri-stage-6-backend-connect-2026-08-11.md`
- R133-1 借鉴源 12 源 实施 (per 决策 #75 §2.1): OpenCog AGPL-3.0 fork 决策
  - `reports/agent-r133-1-borrowed-12-sources-implementation-2026-08-11.md`
- R133-2 ASI Stage 9 长程 AI 成长 (per 决策 #75 §2.1): H 自治 + L 长程 + G 成长 + P 平台化 4 维度
  - `reports/agent-r133-2-asi-stage-9-long-term-ai-growth-2026-08-11.md`
- R133-3 三洋葱架构升级 (per 决策 #75 §2.1): 三洋葱 → 四洋葱 升级方案
  - `reports/agent-r133-3-three-onion-architecture-upgrade-2026-08-11.md`
- R134-3 整合 #6 commit 拍板 (per 决策 #76 §2.1): 6.1 src/ + 6.2 docs/ + 6.3 reports/ 拍板准备
  - `reports/agent-r134-3-integration-6-commit-paiban-2026-08-11.md`
- R134-4 整合 #7 commit 拍板实战续 (per 决策 #76 §2.1): 整合 #7 commit 拍板准备续
  - `reports/agent-r134-4-integration-7-commit-paiban-xu-2026-08-11.md`
- R137-1 PHL-07 实施 spec (per 决策 #77 §3.1): PHL-07 V1.1 release 实施 spec
  - `reports/agent-r137-1-phl07-implementation-spec-2026-08-11.md`
- R137-2 24 LOCKED 入口签名 改写 spec (per 决策 #77 §3.1): 5 阶段 8 周 实施计划
  - `reports/agent-r137-2-24-locked-entry-rewrite-spec-2026-08-11.md`
- R137-3 Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #77 §3.1)
  - `reports/agent-r137-3-cargo-toml-1-2-1-bump-2026-08-11.md`
- R137-4 ASI Stage 9 实战 (per 决策 #77 §3.1): ASI Stage 9 长程 AI 成长 实战 spec
  - `reports/agent-r137-4-asi-stage-9-execution-2026-08-11.md`
- R137-5 形式化 Stage 5.5+ 实战 (per 决策 #77 §3.1): 形式化 Stage 5.5+ 实战
  - `reports/agent-r137-5-formal-proof-stage-5-5-execution-2026-08-11.md`
- R138-6 整合 #6 commit 拍板实战 (per 决策 #78 + R134-3 续): 5 阶段 4 周 + 2 天 实施计划
  - `reports/agent-r138-6-integration-6-commit-paiban-2026-08-11.md`
- **R138-7 整合 #7 commit 拍板实战续 (per R134-4 续 + 决策 #74 B1 + 决策 #78)**: 3 阶段 1 周 实施计划 + 7.1 src/ 拍板 + 7.2 docs/ 拍板 + 7.3 reports/ 拍板 + V1.1 release 实战 7 步 runbook
  - `reports/agent-r138-7-integration-7-commit-paiban-xu-2026-08-11.md` (**R152-4 reference 不重写**)

### 10.2 决策链 (per R148-12 v3 总索引 + 决策 #71 §2 + 决策 #74 B1 + 决策 #78 + 决策 #86 §4)

**决策链 #30-#86 (57 决策, 12 维度, per R148-12 v3 总索引)**:
- 决策 #10: 决策日志 (per 用户记忆 #10)
- 决策 #22: 24 LOCKED + semver (B1 B2 严守)
- 决策 #33: 8 硬墙 + 0 装 PASS 严守 + 0 主动 commit + 0 主动 push (8 硬墙 0 越界 100%)
- 决策 #48: 整合 #4 commit abf12243 严守 (master HEAD 严守 100%)
- 决策 #55: 9 阶段路线图 + 24 LOCKED + 借鉴 ID 严格化
- 决策 #57-#58: R128 阶段 B Tauri prototype 派活
- 决策 #61: 新会话接手 + R129 era 派活规划 (mvs_367e66fae08342ffa399befe4f85dbac)
- 决策 #62: 整合 #5 commit 拆 3 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/)
- 决策 #64: 5 min tick cron 自动监督
- 决策 #71: 计划内任务完成自动接续 4 步 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施)
- 决策 #72: R130 era 调研 6 sub-agent
- **决策 #73**: 主人 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视 + 总工程哲学扩展 "不要怕复杂度")
- **决策 #74**: 8 硬墙 B1 改写 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改) — **R152-4 核心决策**
- 决策 #75-#77: R131-R137 era 派活拍板
- 决策 #78: 整合 #5.3 reports/ commit 拍板 Option A, 1:43 done, master HEAD = 4207f187
- 决策 #79-#85: R138-R148 era 派活
- 决策 #86: R152 era 5 sub-agent 派活 (R152-1 ~ R152-5) — **R152-4 派活依据**

### 10.3 哲学文档 (per 决策 #73 §3 + 决策 #74 §1 + 哲学文档 15-no-fear-complexity.md)

**哲学文档 (per 决策 #33 §2.3 B5 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)**:
- `docs/conventions/09-anchor.md` (8 哲学锚, per 决策 #33 §2.3 B5)
- `docs/conventions/10-locked.md` (24 LOCKED + 8 哲学锚, per 决策 #33 §2.3 B1)
- `docs/conventions/15-no-fear-complexity.md` (不要怕复杂度哲学, per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3)
- `docs/conventions/README.md` (哲学文档 索引)

### 10.4 用户记忆 (per 用户记忆 #1-#10)

**用户记忆 #1-#10 (per 用户记忆 #1-#10 + 决策 #10 + 用户记忆 #10 决策日志)**:
- 用户记忆 #1: 先思考后动手 (反对"先做再想")
- 用户记忆 #2: 让我做判断, 不机械问拍板 (Mavis 给结构化判断 + 理由 + 风险, 不只列选项)
- 用户记忆 #3: 用户看结果不看哲学 (守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM 7 项砍)
- 用户记忆 #4: AI 不会衰老病死 (跟传统生命周期模型不同, 9 organ 永远循环 0 死亡)
- 用户记忆 #5: 信息密度"高"= 拟人化 + 拟物化 (1 屏多卡, 状态为主页)
- 用户记忆 #6: 派 sub-agent 干, 但要驾驭团队不重复造轮子
- 用户记忆 #7: 推技术决策要守规范, 但要诚实
- 用户记忆 #8: 前端终极 = Tauri, TUI 是过渡 (TUI 升级节奏, 改瘦后暂告段落, 优先后端)
- 用户记忆 #9: TUI 升级节奏: 改瘦后暂告段落, 优先后端
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 10.5 关键路径 + 状态 (per 决策 #78 + 决策 #86 + 整合 #5.3 commit 拍板 Option A)

**关键路径 + 状态 (per 决策 #78 + 决策 #86 §4)**:
- 整合 #4 commit: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
- 整合 #5.3 commit: `4207f187100183170558d70633a970969aebdcda` (8/11 1:43 done, 187 files / 127548 insertions, 0 主动 push 严守)
- 整合 #5.1 commit: ❌ NOT READY (per 决策 #78 §2.3 + 决策 #81 + R139-1-retry 续修 跑中, 0 改本报告)
- 整合 #6 commit: 估 2026-11-25 (V1.1 release 前 5 天, Mavis 自决拍板)
- 整合 #7 commit: 估 2026-11-29 (V1.1 release 前 1 天, Mavis 自决拍板, **本报告核心**)
- V1.1 release tag: 估 2026-11-30 (`v1.1.0` 或 `v1.2.1`, per 决策 #74 §1 B2)
- 报告路径: `Apeireth-rust\reports\agent-r152-4-integration-7-tauri-integration-optimize-prep-2026-08-11.md`
- 目标大小: 80-120 KB
- 生成时间: 2026-08-11 (R152 era 实施 spec 准备阶段, 60 min 时间盒)

---

## 11. 一句话 (再次强调, per 决策 #74 B1 + 决策 #73 §3 + 决策 #33 §2.3 + 用户记忆 #3)

**R152-4 整合 #7 Tauri 集成优化准备 (实施 spec) (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #62 整合 #5 commit 3 commit 类比 + 决策 #71 §2 永久循环接续 + 决策 #86 §4 R152 era 派活拍板 + 决策 #78 整合 #5.3 done + R131-8 9 优化方向 + R130-3 Stage 5 集成深化 + R138-7 整合 #7 commit 拍板实战续)**: 整合 #7 commit 拍板 估 2026-11-29 (V1.1 release 前 1 天, Mavis 自决拍板) + **8 维度 Tauri 集成优化 实施 spec 详细** (维度 1 Tauri 2.0 完整集成 + 维度 2 5 nav 完整 + 维度 3 9 organ 拟人化 final 1 屏多卡 + 维度 4 Stage 4-8 实战路线 + 维度 5 Tauri 跨平台 + 维度 6 Tauri 性能 + 维度 7 Tauri 借脑 + 维度 8 Tauri PHL-07 集成, 总 ~600 NEW tests 累计 cargo 122 + 集成层 79 + 600 = 801 tests) + **5 关系 详写** (跟 Rust 后端 8 endpoint + 3 启动模式 / 5 nav / 9 organ / ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 8 哲学锚 / 用户记忆 #3) + **6 子方向 派活计划** (R152-4-1 ~ R152-4-6 估 6-12 周 实施, 跟 V1.1 release 2026-11-30 留 8-12 周 buffer) + **8 硬墙 V1.1 release Mavis 自决改** (B1 24 LOCKED 入口签名 可改 + 0 改原 24 LOCKED + 仅扩 endpoint, per 决策 #74 §2.2 B1) + **8 哲学锚 严守 100%** (per 决策 #33 §2.3 B5) + **不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 15, 最强效果 + 最厉害工程) + **0 装 PASS 严守 100%** (0 cargo install / 0 cargo add, 借脑 0 借具体源码) + **0 借脑 0 装 严守 100%** (per 决策 #33 §2.3 C2) + **0 主动 commit/push/IM 严守 100%** (per gate-discipline) + **0 重复造轮子严守 100%** (R131-8 96 KB + R130-3 62.5 KB + R129-19 + R129-9 + R130-6 + R133-1/2/3 + R137-1~5 + R138-6/7 + R151-2 + 哲学文档 15 reference 不重写) + **风险 6 维** + **异常分支 5 维** + **决策原则 22 维** + **8 步 verify 流程** (per 决策 #11 + 决策 #78 §2.3 + R147-1 1.0 release 实战 8 步) + **V1.1 release 实战 7 步 runbook** (整合 #7 commit 拍板后, 主人起床后手跑, 0 主动 push 严守 100%, 估 2026-11-30 done) + **8 硬墙 0 越界 100% 严守** (B1 24 LOCKED 入口签名 0 改 + 0 改原 24 LOCKED + 仅扩 endpoint, per 决策 #74 §2.2 B1 V1.1 release Mavis 自决改).

---

**报告路径**: `Apeireth-rust\reports\agent-r152-4-integration-7-tauri-integration-optimize-prep-2026-08-11.md`
**生成时间**: 2026-08-11 (R152 era 实施 spec 准备阶段, 60 min 时间盒)
**关联决策**: 决策 #9 + #10 + #22 + #33 + #44 + #48 + #55 + #56-#58 + #60 + #61 + #62 + #64 + #65-#70 + #71 + #72 + **#73 (主人 01:14 拍板 3 件套)** + **#74 (8 硬墙 B1 改写)** + #75-#78 + #79-#85 + **#86 (R152 era 5 sub 派活拍板)** + 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #1-#10
**作者**: Mavis (R152-4 sub-agent, 决策 #86 §4 R152 era 派活拍板, 60 min 时间盒内 done)
**作者说明**: R152-4 报告 0 改 src 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%, 0 借脑 0 装 严守 100%, 0 重复造轮子严守 100%, 8 硬墙 0 越界 100% 严守, 8 哲学锚 严守 100%, 9 organ 永远循环 0 死亡 严守 100%, 0 暴露 7 项 UI 哲学 严守 100%, 5 nav 严守 0 改 100%, 不要怕复杂度哲学 落地 100%, TUI 跟 Tauri 升级路径一致 100%, 9 organ 1 屏多卡 拟人化 100%, 决策日志写 100%.
