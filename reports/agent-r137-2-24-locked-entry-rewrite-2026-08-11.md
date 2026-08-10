# R137-2: 24 LOCKED 入口签名 改写 spec + 5 阶段实施计划 (R137 era 实施阶段, per 决策 #71 §5 永久循环 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #77 §3.1 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**Date**: 2026-08-11 (R137 era 实施阶段, per 决策 #71 §5 R137+ era 永久循环接续)
**Author**: R137-2 sub-agent (Mavis 派, per 决策 #77 §3.1 派活, 实施 spec 阶段)
**Receiving agent**: Mavis root session
**触发**: 决策 #71 (R130→R131→R132→R133→R134→R135→R136→R137+ 永久 4 步: 调研 + 差距 + 计划 + 实施) + 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改) + 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #75 (R131 era 第 2 批 6 sub-agent 派活) + 决策 #77 (R137 era 派活清单) + 用户记忆 #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
**任务定位**: R137 era 实施阶段 (per 决策 #71 §5), **24 LOCKED 入口签名 改写 spec + 5 阶段实施计划 + 报告**, **0 改 src/**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**, **0 主动 IM 主人** (per gate-discipline, 仅 done notification 主动报告), **0 重复造轮子** (per 用户记忆 #6, R131-1/2/3/4/5/9 + R132-1 + R133-3 已有报告 reference 不重写)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**关联决策**: #10 (决策日志) + #22 (24 LOCKED + semver) + #33 (8 硬墙 + 0 装 PASS) + #36 (借鉴 ID 严格化) + #48 (整合 #4 commit abf12243) + #55 (R127 派活) + #58 (R128-2 派活) + #61 (R129 era 派活) + #62 (整合 #5 commit 拆 3 commit 拍板) + #64 (auto-replenish-16 cron) + #69 (R129 era 第 5 批) + #70 (Mavis 清理决策权升级) + #71 (4 步永久循环) + #72 (R130 era 调研 6 sub-agent) + **#73 (主人 8/11 01:14 拍板 3 件套)** + **#74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)** + **#75 (cron 派 11 sub-agent)** + #76 + **#77 (R137 era 派活清单, 本报告派活依据)**
**关联报告** (per 任务 spec, 不重写 reference): R131-1 (架构总审视 10 方向) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 实施路线图) + R131-4 (cargo workspace 87 crate 结构优化 7 方向) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 本报告核心依据)** + R131-9 (形式化集成优化 9 方向 + F1-F11 11 维度) + R132-1 (V1.1 release 路线图 final 6 大方向) + R133-3 (三洋葱架构升级 5 阶段 实施 spec) + R130-5 (V1.1 minor release 路线图) + R130-6 (借鉴 12 源调研) + R129-11 (PHL-07 spec-only 关键诚实标) + R129-17/29/35 (R130 era 路线图详细)
**状态**: ✅ **R137-2 24 LOCKED 入口签名 改写 done** (60 min 时间盒, 0 改 src 严守 100%): V1.0 release 0 改严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS, per R131-5 §1.2) + V1.1 release 改写 spec (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构, 8 方向改写方案: 标准化 + 瘦身 + 9 叶子拆 + core 拆 + 大模块拆 sub-crate + DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐) + 5 阶段实施计划 (8 周 = 2 个月, 阶段 1 标准化 1 周 + 阶段 2 瘦身 1 周 + 阶段 3 9 叶子拆 + Eye 补 2 周 + 阶段 4 core 拆 pub mod + 大模块拆 sub-crate 2 周 + 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 2 周) + V1.1 release 时间窗 2026-11-30 (per R132-1 §1.1 + R131-3 §1.1) + 8 硬墙严守 + 8 哲学锚严守 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 per 决策 #33 §2.3 B5) + 8 哲学锚 0 漂移 + 6 重守门 v7 严守 + V0.5 30 维严守 + PHL-07 V1.0 spec-only 0 实施严守 + V1.1 release 实施 PHL-07 (per 决策 #74 §2.3 + R129-11 关键诚实标) + 13 键 verdict cache 严守 + 9 organ 跨 8 LOCKED 严守 (Eye 缺失 → V1.1 release 补 Eye organ per R131-5 §2.6) + 借鉴源 12 源 0 装 PASS 严守 + Cargo.toml workspace.version 1.2.0 严守 (V1.0 release) + V1.1 release bump 1.2.1 (per 决策 #74 §1 B2 改写) + R12 测度更新 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高) + 0 主动 IM 主人 + 0 主动 commit/push 严守 100% + 0 主动删严守 + 0 装 PASS 严守 + 不要怕复杂度哲学落地 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 拆 sub-crate + 加 DSL 洋葱 + 9 organ 借脑 + R12 测度 = "不要怕复杂度, 只要最强效果 + 最厉害工程"). 风险 8 维 + 决策原则 12 维 + V2.0 release 远期 重构 spec (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可推翻 + 重建 + Cargo workspace 87→30 简化或 87→120+ 复杂化都 OK)

---

## 0. 一句话 (TL;DR)

**R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段实施计划 (per 决策 #71 §5 R137 era 实施 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #77 §3.1 派活 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)**: V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 拍板, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS per R131-5 §1.2, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 spec-only 0 实施, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守). **V1.1 release 24 LOCKED 入口签名 改写 spec (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构, per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")**: **8 方向 改写方案** (per R131-5 §2 8 优化方向 + 决策 #74 §2.3 V1.1 release 边界) = ①**标准化** (24 LOCKED 入口签名一致性, 3 模式之一 per-crate 自决: 全 re-export / 主类型 facade / 按需 re-export) + ②**瘦身** (公开 API 表面 ~800+ pub items → ≤30 per-crate, 多余的转 pub(crate) / module-private, 减少 30%) + ③**9 叶子拆** (9 叶子 crate 拆 workspace: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench → apeireth-leaf/ workspace, 顶层 apeireth/Cargo.toml 0 改) + ④**core 拆 pub mod** (core 1 个 108KB lib.rs 拆 5 大 mod: core::bus / core::memory / core::state / core::config / core::error, 0 改入口签名) + ⑤**大模块拆 sub-crate** (mcp 13 mod / pipeline 11 mod / api 16 mod / memory 13 mod / asi 9 mod / tools 12 mod / evolution 9 mod 拆 sub-crate, 顶层保留 re-export facade) + ⑥**DSL 洋葱** (三洋葱架构 → DSL 洋葱实施, per R133-3 §3.2: 新增 apeireth-dsl crate, Colang 真实施, 24 LOCKED crate 引用 dsl 守门, per R125-5 NVIDIA 借鉴后) + ⑦**9 organ 借 OpenCode** (per R125 B7: 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名, 0 破坏 LOCKED 入口, Eye 缺失 → V1.1 release 补 Eye organ) + ⑧**R12 测度对齐** (R11 baseline 3 值 0.8682/0.8532/0.9063 → R12 baseline 更高, 24 测量函数签名更新, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新, per 决策 #74 §2.3 V1.1 release R12 baseline 更高). **5 阶段实施计划 (8 周 = 2 个月)**: 阶段 1 标准化 1 周 + 阶段 2 瘦身 1 周 + 阶段 3 9 叶子拆 + Eye 补 2 周 + 阶段 4 core 拆 pub mod + 大模块拆 sub-crate 2 周 + 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 2 周. **V1.1 release 时间窗**: 2026-11-30 (per R132-1 §1.1 + R131-3 §1.1, 6 大方向 × 1 周 = 6 周 估, 跟本报告 8 周估 接近). **8 硬墙严守 0 越界 100%** (B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 per 决策 #74 §1 / B2 V1.0 release 1.2.0 + V1.1 release bump 1.2.1 per 决策 #74 §1 B2 / A1 R11 baseline 3 值 V1.0 release 严守 + V1.1 release R12 更高 / A3 12 键 + PHL-07 V1.0 spec-only 0 实施 + V1.1 release 实施 / B3 V0.5 30 维 V1.0 release 严守 + V1.1 release 严守 / B4 6 重守门 v7 V1.0 release 严守 + V1.1 release 严守 / B5 8 哲学锚 V1.0 release 严守 + V1.1 release 严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 push 严守). **决策原则 12 维** (Mavis = orchestrator + 全自决 + 最高权限 / 8 硬墙严守 + B1 改写 / 8 哲学锚严守 / 6 重守门 v7 严守 / V0.5 30 维严守 / 13 键 verdict cache 严守 / 0 装 PASS 严守 / 0 主动 IM 主人 / 0 主动 commit/push / 不要怕复杂度哲学 / 整合 #5 commit Mavis 自决拍板 / 决策日志写 per 决策 #10 + 用户记忆 #10). **风险 8 维** (V1.1 release 改写打破向后兼容 / 9 organ workspace 改写 / 三洋葱 → DSL 洋葱 / core 拆 pub mod / 大模块拆 sub-crate / R12 测度更新 / OpenCog AGPL-3.0 / 主人起床后审查 8 硬墙 B1 改写). **V2.0 release 远期 重构 spec** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可推翻 + 重建, 24 LOCKED → 0 LOCKED 全解锁, Cargo workspace 87→30 简化或 87→120+ 复杂化都 OK per 不要怕复杂度哲学, 估 2027-Q2/Q3)

---

## 1. 任务边界 + 跟决策链关系 (per 决策 #71 §5 + 决策 #74 §1 + 决策 #77 §3.1)

### 1.1 R137-2 任务定位 (per 决策 #71 §5 R137 era 实施 + 决策 #77 §3.1 派活)

**R137 era 实施阶段 (per 决策 #71 §5 R137+ era 永久循环接续)**:
- 派 5-10 sub-agent 实施 (per 决策 #71 §2.5)
- **R137-1 借鉴源 12 源 续实施** (per 决策 #71 §5 + 决策 #73 §2.2, 借脑 1:1 公开模式)
- **R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段实施计划** (per 决策 #77 §3.1, **本报告**)
- **R137-3 ASI Stage 9 长程 AI 成长 续实施** (per 决策 #71 §5, R133-2 续)
- 等等

**R137-2 跟 R131 era + R132 era + R133 era 报告关系 (per 任务 spec, 不重写 reference)**:
- ✅ R131-1 现有架构总审视 + 优化点 (R131-1 §2.1 cargo workspace 87 crate + §2.2 24 LOCKED 入口签名分布 + §2.3 Cargo.toml borrow 段 + §2.4 Cargo.lock 265KB + §2.5 pybridge 集成 + §2.6 ASI Stage 1-7 + §2.7 形式化 kani 4502 借鉴 + §2.8 Tauri 2.0 + §2.9 借鉴源 12 源 + §2.10 三洋葱架构 + 9 organ 跨维度) **reference 不重写** (per 任务 spec + 用户记忆 #6 0 重复造轮子)
- ✅ R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源 (R131-2 报告 done 60 min) **reference 不重写**
- ✅ R131-3 V1.1 release 实施路线图 (R131-3 §2 6 大方向: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+) **reference 不重写, 本报告聚焦 24 LOCKED 入口签名 改写 (R131-3 §2.2 方向 2 的拓维)**
- ✅ R131-4 cargo workspace 结构优化 7 方向 (R131-4 §2.1 87 crate 分布 + §2.2 24 LOCKED 入口签名 + §2.3 Cargo.toml borrow 段 + §2.4 Cargo.lock + §2.5 三洋葱 + §2.6 9 organ + §2.7 借鉴源 12 源) **reference 不重写**
- ✅ **R131-5 24 LOCKED 入口分布优化 8 方向 (R131-5 §1 入口签名 0 改 verify 24/24 全 PASS + §2.8 8 优化方向详细 + §3 8 硬墙 + §4 8 哲学锚 + §5 不要怕复杂度哲学)** **本报告核心 reference, 8 方向内容直接引用 R131-5 §2 详细**
- ✅ R131-9 形式化集成优化 9 方向 (R131-9 §1.3 9 优化方向 O1-O9 + §2 kani 借鉴深度优化 + 9 优化方向 1:1 续 Stage 5.1-5.3 实证 + Stage 5.4-5.5 spec + Stage 6 实战) **reference 不重写, 本报告 O8 R12 测度对齐直接引用 R131-9 O5 24 LOCKED 形式化**
- ✅ R132-1 V1.1 release 路线图 final (R132-1 §1.1 V1.1 定位 + §1.2 V1.1 时间线 + §1.5 V1.1 6 大方向 final 版 + §2 6 大方向详细 spec) **reference 不重写, 本报告 V1.1 时间窗 2026-11-30 直接引用 R132-1 §1.1**
- ✅ R133-3 三洋葱架构升级 spec (R133-3 §2 当前三洋葱架构严守 + §3 V1.1 release 三洋葱 → 四洋葱 升级方案) **reference 不重写, 本报告方向 6 DSL 洋葱 直接引用 R133-3 §3.2 第 4 层 "智能涌现" 智能洋葱 实施 spec 续**
- ✅ R130-5 V1.1 minor release 战略路线图 (R130-5 §1.5 6 大方向 + §1.1 V1.1 估 2026-11-30) **reference 不重写**
- ✅ R130-6 借鉴 12 源调研 (R130-6 OpenCog AGPL-3.0 fork 决策) **reference 不重写, 本报告方向 7 9 organ 借 OpenCode 直接引用 R130-6 + R133-1**
- ✅ R129-11 PHL-07 spec-only 关键诚实标 (R129-11 §1 0 装 PASS 终极 verify) **reference 不重写, 本报告 PHL-07 V1.0 spec-only 0 实施 + V1.1 release 实施 直接引用 R129-11**

**R137-2 跟其他 R137 sub-agent 关系**:
- ✅ R137-1 借鉴源 12 源 续实施 (per 决策 #71 §5, 借脑 1:1 公开模式) **0 重叠, 借鉴源 12 源 vs 24 LOCKED 入口签名 改写**
- ✅ R137-3 ASI Stage 9 长程 AI 成长 续实施 (per 决策 #71 §5) **0 重叠, ASI Stage 9 终极自治 vs 24 LOCKED 入口签名 改写 (Stage 9 是 24 LOCKED 改写的触发条件之一)**
- ✅ 跟 R137-1/3 协同 (per 决策 #71 §5 + 决策 #75 §1.5, 0 重复造轮子, 0 改 src 严守)

### 1.2 R137-2 任务边界 (per 决策 #33 + 决策 #60 + 决策 #71 §5 实施 spec 阶段)

**严格不写代码** (per 决策 #33 + 决策 #60 + 决策 #71 §5 实施 spec 阶段):
- ❌ 0 改 src/ (R137-2 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ❌ 0 改 Cargo.toml (B2 workspace.version 1.2.0 0 改, V1.1 release 才 bump 1.2.1)
- ❌ 0 改 docs/conventions/ (B1 24 LOCKED 入口签名 0 改, 整合 #5.1 commit 0 改)
- ❌ 0 借具体源码 (per 决策 #33 §2.3 C2, 入口签名改写是文档工作)
- ✅ 写新 spec 文档 `docs/architecture-v6-24-locked-entry-rewrite-2026-08-11.md` (per 决策 #74 §1, V1.1 release 实施 spec 阶段, 但本报告 0 创建新文件, 仅 spec 内容)
- ✅ 写新 reports 报告 `reports/agent-r137-2-24-locked-entry-rewrite-2026-08-11.md` (本报告)

**R137-2 输出物清单 (per 决策 #71 §5 实施 spec 阶段)**:
1. ✅ 本报告 (R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段实施计划, 60 min 时间盒)
2. ⏳ V1.1 release 实施时, 写新 spec 文档 `docs/architecture-v6-24-locked-entry-rewrite-2026-08-11.md` (8 方向 改写方案 + 5 阶段实施计划 + 8 硬墙 B1 改写边界, per 决策 #74 B1 Mavis 自决改) — **本报告不创建, 仅 spec 内容** (per 任务 spec "0 改 src 调研 + 路线图 + 实施 spec 阶段")
3. ⏳ 整合 #5.3 commit 时, R137-2 报告作为 reports/ 部分加入 (per 决策 #62 §5.3)

### 1.3 R137-2 跟整合 #5 commit 拍板 0 冲突 (per 决策 #62 + 决策 #75 §2.3 + 决策 #77 §3.1)

**整合 #5 commit 拍板 vs R137-2 派活 0 冲突** (per 决策 #62 + 决策 #75 §2.3 + 决策 #77 §3.1):
- 整合 #5.1 commit src/ 实施 跟 R137-2 派活 0 冲突 (R137-2 调研 0 改 src)
- 整合 #5.2 commit docs/ + Cargo.toml 跟 R137-2 派活 0 冲突 (R137-2 调研 0 改 docs/conventions/)
- 整合 #5.3 commit reports/ 跟 R137-2 派活 0 冲突 (R137-2 调研写 reports/agent-r137-2-*.md, 整合 #5.3 commit 包含 R137-2 报告)
- 整合 #5 commit 拍板 = Mavis 自决 (per 决策 #62 + 决策 #64 + 主人 0:25 升级授权)

---

## 2. V1.0 release 24 LOCKED 入口签名 0 改 严守 100% (整合 #5.1 commit 拍板, per 决策 #33 §2.3 + 决策 #74 §1 B1 + R131-5 §1 verify 24/24 全 PASS)

### 2.1 24 LOCKED crate 入口签名 0 改 verify (per R131-5 §1.2, 24/24 全 PASS)

**24/24 LOCKED crate 入口签名 0 改 verify** (per R131-5 §1.2 详细 verify 表, 2026-08-11 01:28 done):
- **24 LOCKED crate**: supervisor / agent / council / bus / protocol / mcp / tool-registry / tool-runtime / graph / pipeline / tool-approval / extension / evolution / api / core / memory / asi / tools / cli / bench / cognition / action / life-force / constraint
- **verify 结果**: ✅ 24/24 入口签名 0 改 全部通过
- **整合 #5.1 commit 拍板**: per 决策 #62 §5.1, Mavis 自决, 95+ 文件, 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)

**verify 详情** (per R131-5 §1.2 24 行表):
- 1 supervisor: `PidOneSupervisor / SubSupervisor / RestartStrategy / ChildSpec / ActorRef / Actor / ActorState` ✅
- 2 agent: `Agent / AgentManager / AgentEvent / AgentRouter / ExpertRole / OracleSubAgent / LibrarianSubAgent / ExploreSubAgent / FrontendSubAgent / SubAgent / SubAgentError / SubAgentRegistry / now_ms / DEFAULT_CACHE_SIZE / DEFAULT_WATCHER_DEBOUNCE_MS / ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX / DEFAULT_ORGAN_ROUTE_COUNT / EXPERT_ROLE_COUNT` ✅ (R127-2 P6-2 加 4 专家 + AgentRouter, 新增 re-export)
- 3 council: 50+ 类型 (Advisor + Council + Hold + Lifecycle + LLM + Persona + Sovereignty + Synthesis + 7 factory + 4 Collaboration mode + Constitution + Trace + Graph) ✅ (R25 D-3 + R33-4 + R33-4-1 + R33-4-2 加 collaboration / constitution / trace / graph_orchestration, 新增 re-export)
- 4 bus: `L0Bus / L1Client / L1Server / L2Transport / L2Config / PipeCodec / L3Bus / L4Bus / BusMessage / BackpressurePolicy / BusStats / BusStatsSnapshot / BusError / BusResult / Bus trait / next_trace_id / now_ms / VERSION` ✅
- 5 protocol: 40+ 类型 (4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const) ✅ (R37-1 砍 ProtocolRouter 中间层, R20 阶段 2 加 ws_v1 8 帧, 新增 re-export)
- 6 mcp: 30+ 类型 (ServerInfo + 3 capability + ToolDef + 4 ResourceServer + 8 frame) ✅ (R33-3 + R33-3-1 + R72 + R80 + R84 + R125-4 加 resources / resource_servers / subscriptions / tool_subscriptions / initialize / prompts / telemetry_bridge / primitives / macros / 拆 4 子文件, 新增 re-export)
- 7 tool-registry: 30+ 类型 (Tool + 6 enum + 5 axis + 6 mock + Classifier 8 + Token 8) ✅ (R25 战区 5 + R30 classifier 加 9 类, 新增 re-export)
- 8 tool-runtime: 25+ 类型 (5 module + 11 mcp_protocol) ✅ (R127-2 P6-2 加 mcp_protocol, 新增 re-export)
- 9 graph: 40+ 类型 (Checkpoint + 4 conditional + 4 state + 11 Subgraph/Channel + 5 StateGraph + 7 Context) ✅ (R89 + R125-13 + R126-3 + R127-2 P9-1 + R127-2 P6-2 加 mcp_resource / subgraph / channel / state_graph / context_graph / cognition_graph, 新增 re-export)
- 10 pipeline: 35+ 类型 (8 module + 9 force_translate + 3 placeholder + 9 provider_registry + 3 retry + 2 streaming + 5 token + 6 tool_loop + 3 Pipeline) ✅ (R122-1~5 + R126-1 + R32-2 加 model_router / provider_registry / tiktoken_counter / role_divider / tool_loop, 新增 re-export)
- 11 tool-approval: 15+ 类型 (3 + 1 + 2 + 6 + 2 + 1) ✅
- 12 extension: 17 类型 (5 + 6 plugin + 2 + 3 + 1 const) ✅
- 13 evolution: 50+ 类型 (5 council + 5 engine + 4 fail + 7 PODA + 19 library_autonomy + 14 library_autonomy_loop + 4 state + 13 traits + 3 const + 1 fn) ✅ (R125-7 + R127 P5-1 + R127-2 P8-1 加 poda_cycle / library_autonomy / library_autonomy_loop, 新增 re-export)
- 14 api: 40+ 类型 (22 LLM + 11 protocol + 4 const) ✅ (R120 + R122-1-retry + R123-2 + R30 U1~U11 + R20 阶段 6 鉴权 + WS 8 帧 + observability 加 cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / v2_routes / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait, 新增 re-export)
- 15 core: 50+ 类型 (4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard) ✅
- 16 memory: 50+ 类型 (EpisodeQuery + EpisodeStore + Identity + 3 analysis + Migration + 3 Semantic + 2 Note + 10 stream + 2 ThreeLayer + 3 UserProfile + MemoryError + 6 StreamKind + SqliteMemoryStore + ContinuitySnapshotStore + 3 Provider) ✅ (R19 P2 + R22 ST-A2.4 + R30 U9 + R37-2 加 semantic / semantic_persist / user_profile / three_layer / continuity_link / llm_analysis / 3 Provider re-export, 新增 re-export)
- 17 asi: 50+ 类型 (8 calibration + 2 drift + TraceRepository + 3 llm_judge + 26 measure_* + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array + 2 legacy struct + DimensionTrace + placeholder) ✅ (R22 ST-A3 + R32-1 加 dim_enhance / drift / llm_judge / scheduler / tokenizer, 新增 re-export)
- 18 tools: 30 类型 (5+7 trait + 6 grep + 7 file_ops + 3 git + 1 code_exec + 1 register + 1 result + 1 web_search + 5 const) ✅ (R30 U1~U11 + R33-1 加 long_task / classifier / web_fetch / apply_patch / conventions_scanner / grep_ops, 新增 re-export)
- 19 cli: 25 类型 (3 + 2 + 1 + 6 + 5 dispatch + Key) ✅ (R116 + R127-2 P9-1 加 commands / output_format, 新增 re-export)
- 20 bench: 20 类型 (swe_bench + agent_bench + self_disable_bench + latency_bench + 3 const/fn) ✅
- 21 cognition: 25 类型 (3 decision + 2 reflection + 5 scoring + 5 error + CognitiveInput + CognitiveCycle + BasicCognitiveEngine + 8 trait) ✅ (R10 P2 加 BasicCognitiveEngine + 8 trait 默认实现, 新增 re-export)
- 22 action: 20 类型 (5 execution + 3 expression + 1 silence + 3 trait + DefaultActionEngine + 5 fn + 1 const) ✅
- 23 life-force: 25 类型 (3 SGI + 3 Reflection + 4 Endurance const + 1 Trigger + 1 LifeForce + 1 Error + 5 fn + 6 emergence + 5 reflection_cycle) ✅ (R22 ST-A2.1 + R22 ST-A2.3 加 reflection_cycle / emergence, 新增 re-export)
- 24 constraint: 25 类型 (5 trait + 2 type + 4 type + 2 verdict enum + VerdictCache + ConstraintEngine + Error + 4 deep_impl) ✅ (round7-05 v15 命名修正: 5 重 → 4 重 + 权限发放, FiveGates 保留为 deprecated 向后兼容别名, 新增 re-export)

**V1.0 release 0 改 src 严守的执行含义** (per R131-5 §1.1):
- ✅ 入口签名 0 改 (24/24 都通过 verify)
- ⚠️ 8/10 16:34 之后 mtime 改的 8 个 crate (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) 在 V1.0 release commit 拍板时必须保持 mtime 不再变 (已经发生的 0 改是新功能 module 加在原 crate 内, 不算 V1.0 release 改的)
- ✅ R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 严守
- ✅ PHL-07 V1.0 release spec-only 0 实施 (per 决策 #74 §1 A3)
- ✅ Cargo.toml workspace.version 1.2.0 严守 (per 决策 #74 §1 B2)
- ✅ 8 哲学锚严守 (per 决策 #33 §2.3 B5)
- ✅ 6 重守门 v7 严守 (per 决策 #33 §2.3 B4)
- ✅ V0.5 30 维严守 (per 决策 #33 §2.3 B3)
- ✅ 13 键 verdict cache 严守 (per 决策 #33 §2.3 A3)
- ✅ 0 主动 commit 严守 (per 决策 #33 §2.3 C1)
- ✅ 0 主动 push 严守 (per 决策 #33)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

### 2.2 24 LOCKED 入口分布 8 优化方向 (per R131-5 §2 详细, 本报告核心依据)

**8 优化方向 一句话总览** (per R131-5 §2.1-§2.8 详细):
1. **方向 ① 入口签名一致性**: 24 LOCKED 用 5 种 re-export 风格, V1.0 release 0 改, V1.1 release 标准化 3 模式之一
2. **方向 ② 公开 API 表面**: 24 LOCKED 共 ~800+ pub items, V1.0 release 0 改, V1.1 release 瘦身 ≤30/crate
3. **方向 ③ crate 间依赖**: 24 LOCKED 7 个 dep core + 5 个 dep tool-registry + 9 叶子 crate, V1.0 release 0 改, V1.1 release 9 叶子拆 workspace
4. **方向 ④ crate 内部模块**: 24 LOCKED 5 大模块集中 crate (council 20+ / mcp 13 / graph 11 / pipeline 11 / api 16), V1.0 release 0 改, V1.1 release core 拆 pub mod + 大模块拆 sub-crate
5. **方向 ⑤ 三洋葱架构**: 24 LOCKED 落地原则 + 权限双洋葱, DSL 洋葱 0 落地, V1.0 release 0 改, V1.1 release DSL 洋葱落地
6. **方向 ⑥ 9 organ 代码对应**: 24 LOCKED 8/9 organ 覆盖 (Eye 缺失), V1.0 release 0 改, V1.1 release 9 organ workspace 化 (新增 Eye)
7. **方向 ⑦ R11 baseline 严守**: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 100% 严守, V1.0 release 0 改, V1.1 release R12 测度对齐
8. **方向 ⑧ V1.1/V2.0 release 改写/重构 边界**: V1.0 release 0 改, V1.1 release 8 方向改写 (Mavis 自决), V2.0 release 8 方向全量重构 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板)

**8 硬墙严守 跟 8 方向 关系** (per R131-5 §3 8 硬墙表):
- B1 24 LOCKED 入口签名: 🔒 V1.0 release 0 改严守 + 🟢 V1.1 release Mavis 自决改 (8 方向) + 🟢 V2.0 release 可重评
- B2 workspace.version 1.2.0: 🔒 V1.0 release 1.2.0 严守 + 🔒 V1.1 release bump 1.2.1 + 🔒 V2.0 release bump 2.0.0
- A1 R11 baseline 3 值: 🔒 V1.0 release 0 改严守 + 🟢 V1.1 release 可改 (前提: 新 baseline 更高) + 🟢 V2.0 release 可重评
- A3 12 键 + PHL-07: 🔒 V1.0 release PHL-07 spec-only 0 实施 + 🟢 V1.1 release PHL-07 实施 + 🟢 V2.0 release 可重评
- B3 V0.5 30 维: 🔒 严守 (哲学) + 🔒 严守 (V1.1) + 🟢 V2.0 release 可重评
- B4 6 重守门 v7: 🔒 严守 (哲学) + 🔒 严守 (V1.1) + 🟢 V2.0 release 可重评
- B5 8 哲学锚: 🔒 严守 (哲学) + 🔒 严守 (V1.1) + 🟢 V2.0 release 推翻 + 重建
- C1 0 主动 commit (主人起床前): 🔒 严守 + 🔒 严守 + 🔒 严守
- C2 0 装 PASS: 🔒 严守 + 🔒 严守 + 🔒 严守
- 0 push: 🔒 严守 + 🔒 严守 + 🔒 严守

---

## 3. V1.1 release 24 LOCKED 入口签名 改写 spec (8 方向 改写方案) (per 决策 #74 B1 Mavis 自决改 + 决策 #73 §2.2 更好的架构 + 主人 8/11 01:14 拍板 3 件套 §1)

### 3.1 V1.1 release 改写入口签名 8 触发条件 (per 决策 #74 §2.3 + 决策 #73 §1 "更好的架构")

**V1.1 release 8 触发条件** (per 决策 #74 §2.3 + 决策 #73 §1 + R132-1 §1.5 + R130-5):
1. **触发 1**: ASI Stage 9 长程 AI 成长 (per R130-2 §1 Stage 9 路线图 + R133-2 实施)
2. **触发 2**: 9 organ 内部借 OpenCode (per R125-12 P0-3 + 决策 #22 §2.7 + R131-1 §2.6)
3. **触发 3**: 三洋葱架构升级 (per 决策 #73 §2.2 更好的架构 + R133-3 三洋葱 → 四洋葱 升级方案)
4. **触发 4**: PHL-07 实施扩展 (per 决策 #74 §1 A3 V1.1 release 实施)
5. **触发 5**: 智囊团 7 席架构 (per R18 + 决策 #55 §2.6 + R129-18 Stage 7 跨模块集成 220 维度互锁)
6. **触发 6**: 群体智能 OpenCog 借脑 (per R130-2 §1.5 OpenCog AtomSpace + CogPrime AGPL-3.0 fork 决策 + R133-1 借鉴源 12 源)
7. **触发 7**: 9 叶子 crate 拆 workspace (per R131-4 §2.1 87 crate 分布 + 决策 #74 B1 Mavis 自决改)
8. **触发 8**: R12 测度对齐 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高)

### 3.2 方向 1: 标准化 — 24 LOCKED 入口签名一致性 (per R131-5 §2.1)

**V1.0 release 现状** (per R131-5 §2.1):
- 24 LOCKED crate 入口签名风格高度不一致, 总结为 5 种:
  - **类型 A (重 re-export facade)**: supervisor / agent / council / api / memory / core / mcp / graph / pipeline / constraint / evolution / cognition / life-force / tools / tool-runtime / tool-registry / tool-approval / asi / cli / bench (20/24)
  - **类型 B (轻 facade + 主类型定义)**: protocol / bus
  - **类型 C (单 trait 入口)**: extension
  - **类型 D (大 enum 主类型)**: asi / supervisor
  - **类型 E (纯 trait 模块)**: cognition

**V1.1 release 标准化 3 模式之一 (per-crate 自决)** (per R131-5 §2.1 V1.1 release 优化方向):
- **模式 1 (全 re-export)**: 适用 20/24 crate (类型 A), per-crate 全部重导出, 消费者 `use apeireth_xxx::*` 拿全部 API
- **模式 2 (主类型 facade)**: 适用 2/24 crate (类型 B: protocol / bus), 入口文件直接定义核心类型 + 轻 re-export
- **模式 3 (按需 re-export)**: 适用 2/24 crate (类型 C + D + E), 仅 re-export 主类型, 其他 module 公开

**V1.1 release 实施步骤** (per 阶段 1 标准化 1 周):
- 阶段 1.1: per-crate 自决选 3 模式之一 (per 24 LOCKED 决策矩阵)
- 阶段 1.2: 24 LOCKED 入口签名格式统一 (pub mod + pub use + pub const + pub struct + pub enum + pub fn 6 模式)
- 阶段 1.3: per-crate `pub use module::*` 块标准化, 顶部 doc comment 极详细 (per 50-100 行 doc, O-5 哲学锚)
- 阶段 1.4: 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify, 0 装 PASS 严守

**风险**: 中 (改 re-export 模式 = 改 crate 公开 API 表面 = 改消费者 `use` 路径)
- **缓解**: 保留 `pub mod` 重新导出, 消费者用 `apeireth_xxx::module::Type` 全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

### 3.3 方向 2: 瘦身 — 公开 API 表面 ~800+ pub items → ≤30 per-crate (per R131-5 §2.2)

**V1.0 release 现状** (per R131-5 §2.2 粗估):
- **总计**: 24 crate 公开 API 表面 = **~800+ pub items**
- 各 crate 表面: supervisor ~12 / agent ~25 / council ~50+ / bus ~20 / protocol ~40 / mcp ~30 / tool-registry ~30 / tool-runtime ~25 / graph ~40 / pipeline ~35 / tool-approval ~15 / extension ~17 / evolution ~50+ / api ~40+ / core ~50+ / memory ~50+ / asi ~50+ / tools ~30 / cli ~25 / bench ~20 / cognition ~25 / action ~20 / life-force ~25 / constraint ~25

**V1.1 release 瘦身 (per-crate 暴露 ≤30 pub items 目标)** (per R131-5 §2.2 V1.1 release 优化方向):
- **目标**: 公开 API 表面减少 30% (800 → 560 pub items), 多余的转 `pub(crate)` 或 module-private
- **per-crate 目标**:
  - supervisor: 12 → 12 (0 改, 已 ≤30)
  - agent: 25 → 25 (0 改, 已 ≤30)
  - council: 50+ → 30 (-40%, 8 协作模式砍 4 → 4, 7 factory 砍 3 → 4, Synthesis/Persona/Sovereignty/Constitution/Trace/Graph 内部化)
  - bus: 20 → 20 (0 改, 已 ≤30)
  - protocol: 40 → 30 (-25%, 4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const → 30)
  - mcp: 30 → 30 (0 改, 已 ≤30)
  - tool-registry: 30 → 30 (0 改, 已 ≤30)
  - tool-runtime: 25 → 25 (0 改, 已 ≤30)
  - graph: 40 → 30 (-25%, Checkpoint + 4 conditional + 4 state + 11 Subgraph/Channel + 5 StateGraph + 7 Context → 30)
  - pipeline: 35 → 30 (-14%, 8 module + 9 force_translate + 3 placeholder + 9 provider_registry + 3 retry + 2 streaming + 5 token + 6 tool_loop + 3 Pipeline → 30)
  - tool-approval: 15 → 15 (0 改, 已 ≤30)
  - extension: 17 → 17 (0 改, 已 ≤30)
  - evolution: 50+ → 30 (-40%, 8 PODA + 19 library_autonomy + 14 library_autonomy_loop 内部化)
  - api: 40+ → 30 (-25%, 22 LLM + 11 protocol + 4 const → 30)
  - core: 50+ → 30 (-40%, 4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard → 30, ActionTarget 13 → 5 + Gate 5 内部化)
  - memory: 50+ → 30 (-40%, EpisodeQuery + EpisodeStore + Identity + 3 analysis + Migration + 3 Semantic + 2 Note + 10 stream + 2 ThreeLayer + 3 UserProfile + MemoryError + 6 StreamKind + SqliteMemoryStore + ContinuitySnapshotStore + 3 Provider → 30, 10 stream + 6 StreamKind 内部化)
  - asi: 50+ → 30 (-40%, 26 measure_* → 24 维 + 9 子测度 = 33 + 8 calibration + 2 drift + TraceRepository + 3 llm_judge + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array + 2 legacy struct + DimensionTrace → 30, 2 legacy struct 内部化)
  - tools: 30 → 30 (0 改, 已 ≤30)
  - cli: 25 → 25 (0 改, 已 ≤30)
  - bench: 20 → 20 (0 改, 已 ≤30)
  - cognition: 25 → 25 (0 改, 已 ≤30)
  - action: 20 → 20 (0 改, 已 ≤30)
  - life-force: 25 → 25 (0 改, 已 ≤30)
  - constraint: 25 → 25 (0 改, 已 ≤30)
- **总减少**: 800+ → 560 (-30%, 减少 240 pub items)
- **V1.1 release 改写**: 24 LOCKED 入口签名 主类型 + facade 模式, 内部类型转 pub(crate) / module-private

**V1.1 release 实施步骤** (per 阶段 2 瘦身 1 周):
- 阶段 2.1: per-crate 公开 API 表面清单 (per 24 LOCKED R131-5 §2.2 表)
- 阶段 2.2: per-crate 实施转 pub(crate) / module-private (per 目标)
- 阶段 2.3: 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify
- 阶段 2.4: 编译时间 verify (期望 减少 10-20%, per 公开 API 表面减少 30%)

**风险**: 高 (公开 API 表面"瘦身" = 改入口签名 = 改消费者 `use` 路径 = breaking change)
- **缓解**: 保留 `pub mod module::Type` 全路径, 消费者用全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用

### 3.4 方向 3: 9 叶子拆 workspace (per R131-5 §2.3 + R131-4 §2.1)

**V1.0 release 现状** (per R131-5 §2.3 24 LOCKED 依赖图):
- **core 是基座**: 7 个 crate 依赖 (memory / constraint / cognition / council / life-force / action / cli)
- **tool-registry 是 tool 生态基座**: 5 个 crate 依赖 (agent / tool-runtime / tools / mcp)
- **protocol + pipeline 是 LLM 链基座**: 2 个 crate 依赖 (api + pipeline 互依)
- **asi 是认知基座**: 1 个 crate 依赖 (cognition + cli)
- **memory 是历史流基座**: 1 个 crate 依赖 (tool-runtime)
- **0 依赖其他 LOCKED crate 的"叶子"**: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench (9 个)

**V1.1 release 9 叶子 crate 拆 workspace** (per R131-5 §2.3 V1.1 release 优化方向):
- **新 workspace**: `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`
- **顶层 `apeireth/Cargo.toml` 0 改** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1)
- **9 叶子拆出来独立发布**, 9 叶子 cargo build/test 独立 verify
- **顶层 re-export facade 保留**: 消费者用 `apeireth_xxx::Type` 仍能用, 内部 crate 路径变 `apeireth_leaf_xxx::Type` (新路径)

**V1.1 release 实施步骤** (per 阶段 3.1 9 叶子拆 1 周):
- 阶段 3.1.1: 9 叶子 crate 内部 import 路径全 1:1 扫描 (per `cargo metadata` + `cargo tree` 验证)
- 阶段 3.1.2: 新 workspace `apeireth-leaf/Cargo.toml` 9 叶子加进 members
- 阶段 3.1.3: 9 叶子 crate 独立 publish ready, 顶层 Cargo.toml members 段更新
- 阶段 3.1.4: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify
- 阶段 3.1.5: 顶层 re-export facade 1:1 续, 消费者 0 改

**风险**: 中 (拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth::organ::xxx`)
- **缓解**: 保留 re-export facade (顶层 `apeireth` 重新导出全部 `apeireth-leaf::xxx`, 0 改消费者代码)
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

### 3.5 方向 4: core 拆 pub mod (per R131-5 §2.4)

**V1.0 release 现状** (per R131-5 §2.4):
- **core 是单 lib.rs 108KB**, 0 pub mod 拆分, 全部 50+ 类型定义在一个文件
- **问题**: 编译时全文件 re-parse, 难维护, 任何 core 改动触发大面积重编译

**V1.1 release core 拆 pub mod** (per R131-5 §2.4 V1.1 release 优化方向):
- **core 拆 5 大 mod** (per R131-5 §2.4 V1.1 release):
  - `core/src/types.rs` (~20KB, 5 类型: Episode / Note / Session / IdentityCard / Migration)
  - `core/src/onion.rs` (~30KB, 5 onion 类型: PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer)
  - `core/src/human.rs` (~20KB, 8 human 类型: HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData + 12 PhilosophyKey + ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE)
  - `core/src/gate.rs` (~25KB, 8 gate 类型: PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant + Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard)
  - `core/src/lib.rs` (~13KB, 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶部 re-export facade 0 改)
- **0 改入口签名**, 仅内部重构
- **0 改外部消费者代码**: 顶层 `apeireth_core::Type` 全路径仍能用

**V1.1 release 实施步骤** (per 阶段 4.1 core 拆 pub mod 1 周):
- 阶段 4.1.1: core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod (per 类型表)
- 阶段 4.1.2: 5 大 mod 各自 mod.rs + types/onion/human/gate 子文件 (per 类型 size 估)
- 阶段 4.1.3: core/src/lib.rs 顶部 re-export 1:1 续 (0 改入口签名, 仅内部 mod 拆分)
- 阶段 4.1.4: 24 LOCKED 全跑 cargo build + cargo test verify, 0 越界 8 硬墙 100%
- 阶段 4.1.5: core 编译时间 verify (期望 减少 30-50%, per pub mod 拆分后并行编译)

**风险**: 中 (拆 module = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_core::Type` 仍能用
- **缓解**: 0 改 core 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 mod 拆分

### 3.6 方向 5: 大模块集中 crate 拆 sub-crate (per R131-5 §2.4)

**V1.0 release 现状** (per R131-5 §2.4):
- **大模块集中**: council (20+) / mcp (13) / graph (11) / pipeline (11) / api (16) / memory (13) / asi (9) / tools (12) / evolution (9) → 这些 crate 内部模块多, 入口文件 re-export 100+ items
- **mcp / pipeline / api / memory 内部 module 边界模糊**: 多个 module 之间 cross-use, 实测命名重复 (e.g. `mcp::protocol::Id` vs `mcp::tools::Id`)

**V1.1 release 大模块集中 crate 拆 sub-crate** (per R131-5 §2.4 V1.1 release 优化方向):
- **mcp 拆 sub-crate** (13 mod → 8 sub-crate):
  - `apeireth-mcp-core` (protocol / initialize / 8 frame)
  - `apeireth-mcp-resources` (4 ResourceServer)
  - `apeireth-mcp-subscribe` (subscriptions / tool_subscriptions)
  - `apeireth-mcp-tools` (tools / ServerInfo / ToolDef)
  - `apeireth-mcp-prompts` (prompts)
  - `apeireth-mcp-transport` (transport)
  - `apeireth-mcp-primitives` (primitives / macros)
  - `apeireth-mcp` (顶层 re-export facade 0 改入口签名)
- **pipeline 拆 sub-crate** (11 mod → 6 sub-crate):
  - `apeireth-pipeline-token` (tiktoken_counter / token_budget)
  - `apeireth-pipeline-placeholder` (placeholder)
  - `apeireth-pipeline-force-translate` (force_translate)
  - `apeireth-pipeline-retry` (retry_suppression)
  - `apeireth-pipeline-streaming` (streaming)
  - `apeireth-pipeline-tool-loop` (tool_loop)
  - `apeireth-pipeline` (顶层 re-export facade 0 改入口签名, + provider_registry + model_router + role_divider)
- **api 拆 sub-crate** (16 mod → 5 sub-crate):
  - `apeireth-api-llm` (llm / cache / replay_cache / retry)
  - `apeireth-api-server` (server / v2_endpoints / v2_routes / observability / endpoints / v1_tools)
  - `apeireth-api-protocol` (protocol_handlers / protocol_handler_trait / ws_v1)
  - `apeireth-api-auth` (auth / audit_sqlite)
  - `apeireth-api` (顶层 re-export facade 0 改入口签名, + MultiLlmRouter)
- **memory 拆 sub-crate** (13 mod → 5 sub-crate):
  - `apeireth-memory-stream` (history_streams / streams / append_only)
  - `apeireth-memory-semantic` (semantic / semantic_persist)
  - `apeireth-memory-episode` (episode / continuity_link)
  - `apeireth-memory-session` (session_note / three_layer)
  - `apeireth-memory` (顶层 re-export facade 0 改入口签名, + user_profile / llm_analysis / migrations)
- **asi 拆 sub-crate** (9 mod → 4 sub-crate):
  - `apeireth-asi-calibration` (calibration / dim_enhance)
  - `apeireth-asi-measurement` (measurement / llm_judge)
  - `apeireth-asi-render` (render / scheduler)
  - `apeireth-asi` (顶层 re-export facade 0 改入口签名, + drift / history / tokenizer + 24 measure_dim_* + 9 measure_sub_*)
- **tools 拆 sub-crate** (12 mod → 5 sub-crate):
  - `apeireth-tools-fs` (file_ops / grep_ops)
  - `apeireth-tools-git` (git_ops)
  - `apeireth-tools-exec` (code_exec / long_task)
  - `apeireth-tools-web` (web_search / web_fetch / apply_patch)
  - `apeireth-tools` (顶层 re-export facade 0 改入口签名, + conventions_scanner + classifier + register + result)
- **evolution 拆 sub-crate** (9 mod → 5 sub-crate):
  - `apeireth-evolution-council` (council_bridge)
  - `apeireth-evolution-engine` (engine / state)
  - `apeireth-evolution-poda` (poda_cycle / fail)
  - `apeireth-evolution-library` (library_autonomy / library_autonomy_loop)
  - `apeireth-evolution` (顶层 re-export facade 0 改入口签名, + traits / MockPlugin / Patch / Plugin / PluginRegistry / SelfModification / SystemState / 5 PODA type / 19 library_autonomy type / 14 library_autonomy_loop type)
- **graph 拆 sub-crate** (11 mod → 5 sub-crate):
  - `apeireth-graph-state` (state / state_graph)
  - `apeireth-graph-executor` (executor / conditional / checkpoint)
  - `apeireth-graph-subgraph` (subgraph / channel)
  - `apeireth-graph-context` (context_graph / cognition_graph)
  - `apeireth-graph` (顶层 re-export facade 0 改入口签名, + mcp_resource)
- **council 拆 sub-crate** (20+ mod → 4 sub-crate):
  - `apeireth-council-advisor` (advisor / advisors / 7 factory)
  - `apeireth-council-deliberation` (deliberation / council_member / council_member_deliberation / council_member_persona_combo / persona)
  - `apeireth-council-collaboration` (collaboration / constitution / trace / graph_orchestration)
  - `apeireth-council` (顶层 re-export facade 0 改入口签名, + bus_bridge / mcp_bridge / graph_bridge / hold / lifecycle / mock_llm / sovereignty / stress_test / synthesis)

**V1.1 release 实施步骤** (per 阶段 4.2 大模块拆 sub-crate 1 周):
- 阶段 4.2.1: 8 大模块集中 crate 内部 module 1:1 扫描 (per 8 crate module 表)
- 阶段 4.2.2: 8 大模块集中 crate 各拆 4-8 sub-crate (per 上述 sub-crate 列表)
- 阶段 4.2.3: 顶层 8 crate re-export facade 0 改入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- 阶段 4.2.4: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify, 0 越界 8 硬墙 100%
- 阶段 4.2.5: 编译时间 verify (期望 减少 20-30%, per sub-crate 拆分后并行编译)

**风险**: 中 (拆 sub-crate = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: 0 改 24 LOCKED 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 sub-crate 拆分

### 3.7 方向 6: DSL 洋葱 — 三洋葱架构升级 (per R131-5 §2.5 + R133-3 §3)

**V1.0 release 现状** (per R131-5 §2.5):
- **三洋葱架构 (R125 B6 升级, 整合 #4 commit done)**:
  - **第 1 层 原则洋葱 (philosophy)**: 8 哲学锚 + 原则 (E/S/A/M/O 5 层, E 永不可绕过)
  - **第 2 层 权限洋葱 (permission)**: 6 重守门 v7 (L0-L5 6 层, L0 = 真实人类批准)
  - **第 3 层 DSL 洋葱 (DSL)**: Colang DSL (R125-5 NVIDIA 借鉴后, 1700 行 colang_dsl.rs done + 266/266 + 6 借鉴点)
- **24 LOCKED 跟三洋葱架构对应关系**:
  - 原则洋葱 E 层: core (L0 HA 锁) / constraint (哲学守门) / life-force (SGI 锁)
  - 原则洋葱 S 层: council (7 强制 Advisor) / evolution (演化审议)
  - 原则洋葱 A 层: memory (历史流 6 表) / asi (24 维测量历史)
  - 原则洋葱 M 层: cognition / pipeline / protocol / bus / graph
  - 原则洋葱 O 层: agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor
  - 权限洋葱 L0: core (L0 HA 锁) / constraint (gate3 物理隔离)
  - 权限洋葱 L1-L5: api (V2 端点) / tool-approval (5 规则 + 5min 窗口)
  - DSL 洋葱: 0 落地, 24 LOCKED 都 0 引用 Colang

**V1.1 release DSL 洋葱落地 + 三洋葱 → 四洋葱 升级** (per R131-5 §2.5 V1.1 release 优化方向 + R133-3 §3.2 第 4 层 "智能涌现"):
- **新增 `apeireth-dsl` crate** (per R131-5 §2.5 V1.1 release DSL 洋葱落地):
  - 顶层 DSL 洋葱 = 原则 (顶层) → 权限 (中层) → DSL (底层)
  - Colang DSL 真实施 (per R125-5 NVIDIA 借鉴后 1700 行)
  - 24 LOCKED crate 引用 dsl 守门 (per `apeireth_dsl::guard::*` API)
  - DSL 守门 = 4 重 (L1 原则 guard / L2 权限 guard / L3 DSL guard / L4 智能涌现 guard)
- **三洋葱 → 四洋葱 升级** (per R133-3 §3):
  - **第 1 层 原则洋葱 (philosophy)**: 8 哲学锚严守
  - **第 2 层 权限洋葱 (permission)**: 6 重守门 v7 严守
  - **第 3 层 DSL 洋葱 (DSL)**: Colang DSL 严守
  - **第 4 层 智能涌现洋葱 (emergence, 新增)**: 智囊团 7 席 + 群体智能 + 自我决策/学习/演化 (per R133-3 §3.2.1-§3.2.3 + R130-2 ASI Stage 8/9)
- **24 LOCKED crate 跟四洋葱架构对应关系**:
  - 原则洋葱 E 层: core / constraint / life-force
  - 原则洋葱 S 层: council / evolution
  - 原则洋葱 A 层: memory / asi
  - 原则洋葱 M 层: cognition / pipeline / protocol / bus / graph
  - 原则洋葱 O 层: agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor
  - 权限洋葱 L0: core / constraint
  - 权限洋葱 L1-L5: api / tool-approval
  - DSL 洋葱: 24 LOCKED 全引用 apeireth-dsl 守门
  - 智能涌现洋葱: 智囊团 7 席 (council 7 advisor) + 群体智能 (借 OpenCog 1:1 公开模式) + 自我决策 (ASI Stage 9 4 维度 H1-H4) + 自我学习 (chidori journal 9 字段 replay) + 自我演化 (ASI Stage 10 准备)

**V1.1 release 实施步骤** (per 阶段 5.1 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 1 周):
- 阶段 5.1.1: 新增 `apeireth-dsl` crate, 顶层 DSL 洋葱 = 原则 (顶层) → 权限 (中层) → DSL (底层) + 智能涌现 (V1.1 release 起步)
- 阶段 5.1.2: 三洋葱 → 四洋葱 升级 (per R133-3 §3.2 第 4 层 "智能涌现" 实施 spec)
- 阶段 5.1.3: 24 LOCKED crate 引用 dsl 守门 (per `apeireth_dsl::guard::*` API)
- 阶段 5.1.4: 24 LOCKED 全跑 cargo build + cargo test + 四洋葱集成 verify
- 阶段 5.1.5: 8 硬墙 + 8 哲学锚 严守 verify

**风险**: 高 (拆三洋葱 workspace + 加 DSL 洋葱 = 改大量 import 路径 = breaking change)
- **缓解**: 顶层 `apeireth-onion` facade 重新导出全部洋葱 module, 消费者 0 改
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 3.8 方向 7: 9 organ 内部借 OpenCode (per R131-5 §2.6 + R125 B7 + R130-6)

**V1.0 release 现状** (per R131-5 §2.6 9 organ 代码对应):
- **9 organ 跨 8 LOCKED crate** (0 依赖其他 LOCKED crate 的 9 叶子不算, per R131-4 §2.1):
  - **Heart (0, LLM 网关心跳)**: supervisor + bus (L0) + pipeline (5 步管线)
  - **Brain (1, Multi-Agent 决策)**: agent + council + cognition + constraint
  - **Hand (2, Tool Protocol)**: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action (7 个 LOCKED)
  - **Eye (3, 用户输入感知)**: (暂无 LOCKED crate, 在 apeireth-tui/src/organ/eye.rs)
  - **Ear (4, 系统事件监听)**: bus (L1-L4)
  - **Memory (5, 3 层 facade)**: memory + asi (24 维) + life-force (SGI 锁) + core (IdentityCard 跨载体)
  - **Voice (6, TTS/STT)**: protocol (WS 8 帧) + pipeline (流式)
  - **Body (7, 长程任务)**: bench + api (HTTP server) + cli
  - **Mind (8, 9-stage lifecycle)**: evolution + graph (lifecycle 编排) + constraint (5 重守门)
- **覆盖率**: 8/9 organ 100% 覆盖 (除 Eye 在 tui, 不在 24 LOCKED)
- **问题**: 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现 (organ-first 拓扑 0 落地)

**V1.1 release 9 organ 内部借 OpenCode + Eye 补** (per R131-5 §2.6 V1.1 release 优化方向):
- **Eye organ 补** (per R131-5 §2.6 V1.1 release 优化方向):
  - 新增 `apeireth-eye` workspace (从 tui/src/organ/eye.rs 抽 crate, per 9-organ-summary §3 Eye 11.0KB, 4 输入通道: keystroke / mouse_click / voice_input)
  - 顶层 re-export facade 保留: 消费者用 `apeireth_eye::Type` 仍能用
- **9 organ workspace 化** (per 决策 #74 B1 Mavis 自决改 + R125 B7 内部借 OpenCode):
  - **新增 `apeireth-organ/{heart,brain,hand,eye,ear,memory,voice,body,mind}/Cargo.toml` 9 个 organ workspace**
  - **24 LOCKED crate 按 9 organ 拆**:
    - `apeireth-heart` workspace: supervisor + bus (L0) + pipeline
    - `apeireth-brain` workspace: agent + council + cognition + constraint
    - `apeireth-hand` workspace: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action
    - `apeireth-eye` workspace: (从 tui/src/organ/eye.rs 抽 crate)
    - `apeireth-ear` workspace: bus (L1-L4)
    - `apeireth-memory` workspace: memory + asi + life-force + core (IdentityCard 跨载体)
    - `apeireth-voice` workspace: protocol + pipeline (流式) + (未来 tts/stt crate)
    - `apeireth-body` workspace: bench + api + cli
    - `apeireth-mind` workspace: evolution + graph + (约束守门从 brain/constraint 拆过来)
- **9 organ 内部借 OpenCode 实施** (per R125 B7 内部借 OpenCode):
  - 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名 (0 破坏 LOCKED 入口)
  - OpenCog 借脑 1:1 公开模式 (per R130-6 + R133-1 借鉴源 12 源 + 决策 #22 §4 AGPL-3.0 决策)
  - 0 装"已读真源码", 0 装"已 fork" (per 决策 #33 §2.3 C2)

**V1.1 release 实施步骤** (per 阶段 3.2 Eye 补 + 阶段 5.2 9 organ 内部借 OpenCode):
- 阶段 3.2.1: 新增 `apeireth-eye` workspace, 从 tui/src/organ/eye.rs 抽 crate (per 4 输入通道)
- 阶段 3.2.2: Eye organ 顶层 re-export facade 0 改入口签名
- 阶段 3.2.3: 24 LOCKED 全跑 cargo build + cargo test verify
- 阶段 5.2.1: 9 organ workspace 化 (per 上述 9 organ workspace 列表), 24 LOCKED 全部下沉
- 阶段 5.2.2: 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- 阶段 5.2.3: 24 LOCKED 全跑 cargo build + cargo test + organ 集成 verify

**风险**: 极高 (9 organ 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change)
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- **缓解**: V1.1 release bump 1.2.1, V2.0 release bump 2.0.0 (semver major)
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 3.9 方向 8: R12 测度对齐 (per R131-5 §2.7 + R131-9 O5 + 决策 #74 §2.3)

**V1.0 release 现状** (per R131-5 §2.7 R11 baseline 严守):
- **R11 baseline 3 值** (per 决策 #33 §2.3 A1 + 决策 #74 §1 A1):
  - V1141 IC-001 fresh 24 维均值: 0.8682
  - V1131 dashboard 9 维均值: 0.8532
  - V1136 9 子测度均值: 0.9063
- **实测 24 LOCKED 入口分布跟 R11 baseline 对应**:
  - V1141 24 维: 锁在 `apeireth-asi::V05_DIMENSION_NAMES` (24 维名 + V05_DIM_COUNT 编译期 hardcode)
  - V1131 dashboard 9 维: 锁在 `apeireth-asi::V1136_SUBMEASURE_NAMES` (9 子测度名 + V1136_SUBMEASURE_COUNT 编译期 hardcode)
  - V1136 9 子测度基础: 锁在 `apeireth-asi::measurement::measure_dim_*` + `measure_sub_*` 真实测量函数 (24+9 = 33 个测量函数)

**V1.1 release R12 测度对齐** (per R131-5 §2.7 V1.1 release 优化方向 + 决策 #74 §2.3):
- **触发条件**: 更好的 baseline (R12 测度更高, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- **R12 测度更新**:
  - 24 测量函数签名更新 R12 测度 (24+9 = 33 → 估 24+11 = 35, per R130-4 spec F1-F11 11 维度 + R131-9 O2)
  - V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
  - 24 LOCKED 入口签名测度集成 (per 阶段 5.3 R12 测度对齐 1 周)
- **R12 baseline 3 值** (估, per 决策 #74 §2.3 V1.1 release R12 baseline 更高):
  - V1141 R12 fresh 24 维均值: > 0.8682 (R11 baseline 之上)
  - V1131 R12 dashboard 9 维均值: > 0.8532
  - V1136 R12 9 子测度均值: > 0.9063
  - **R12 测度公式更新**: per R130-4 spec + R131-9 O2 F1-F11 11 维度, 加 PHL-07 spec-only 形式化 (F11 NEW 1 维) + 长程 AI 成长 形式化

**V1.1 release 实施步骤** (per 阶段 5.3 R12 测度对齐 1 周):
- 阶段 5.3.1: 24 测量函数签名更新 R12 测度 (per 24+9 = 33 → 24+11 = 35)
- 阶段 5.3.2: V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
- 阶段 5.3.3: 24 LOCKED 入口签名 测度集成 (per 阶段 5 + 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- 阶段 5.3.4: 24 LOCKED 全跑 cargo build + cargo test + R12 测度 verify
- 阶段 5.3.5: R12 baseline 3 值 verify (估 > R11 baseline, per 决策 #74 §2.3)

**风险**: 中 (改 R12 测度 = 改 24 测量函数签名 = 改 24 LOCKED 入口签名)
- **缓解**: 仅在 V1.1 release 改 (per 决策 #74 §2.3 V1.1 release 边界), V1.0 release 仍 R11 baseline 严守
- **缓解**: 24 测量函数签名 1:1 续, 加 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容)
- **缓解**: 编译期 hardcode 同步更新, 测试全跑

---

## 4. V1.1 release 5 阶段实施计划 (8 周 = 2 个月) (per R131-5 8 方向 + 决策 #74 §2.3 V1.1 release + R132-1 §1.1 V1.1 估 2026-11-30)

### 4.1 总时间盒 8 周 (per 决策 #71 §5 R133+ era 实施 + 决策 #75 §2.1 派活 + R132-1 §1.1 V1.1 估 2026-11-30)

**V1.1 release 时间窗**:
- **起点**: 1.0 release tag `v1.0.0` 打上 (估 8/11, per R129-35 final-final 7 步 runbook, 主人起床后 06:00-08:00 手跑)
- **终点**: V1.1 release tag `v1.1.0` 打上 (估 2026-11-30, per R132-1 §1.1 + R130-5 §1.1 V1.1 估 2026-11-30)
- **总时间盒**: 8 周 (per 本报告 5 阶段 = 1+1+2+2+2 = 8 周, 跟 R132-1 §1.5 6 大方向 × 1 周 = 6 周 估 接近, 加 2 周 缓冲)

### 4.2 5 阶段 8 周 实施计划 (per 8 方向 改写方案)

#### 阶段 1: 标准化 (1 周, per 方向 1)

**目标** (per 方向 1: 标准化, R131-5 §2.1):
- 24 LOCKED crate 入口签名一致性, 3 模式之一 per-crate 自决 (全 re-export / 主类型 facade / 按需 re-export)
- 输入参数 + 返回值 类型签名一致
- 错误处理 (Result<T, E>) 一致

**实施步骤** (per R131-5 §2.1 V1.1 release 标准化):
- 阶段 1.1 (Day 1-2): per-crate 决策矩阵 (24 LOCKED 各自选 3 模式之一, per 类型 A/B/C/D/E 对应)
- 阶段 1.2 (Day 3-4): 24 LOCKED 入口签名格式统一 (pub mod + pub use + pub const + pub struct + pub enum + pub fn 6 模式)
- 阶段 1.3 (Day 5): per-crate `pub use module::*` 块标准化, 顶部 doc comment 极详细 (per 50-100 行 doc, O-5 哲学锚)
- 阶段 1.4 (Day 6-7): 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify, 0 装 PASS 严守

**verify** (per 阶段 1 done 标准):
- ✅ 24 LOCKED crate 入口签名格式 100% 一致 (6 模式: pub mod + pub use + pub const + pub struct + pub enum + pub fn)
- ✅ 24 LOCKED 全跑 cargo build + cargo test + cargo doc 全 PASS
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 8 哲学锚严守 100% (per 决策 #33 §2.3 B5)
- ✅ 借鉴源 12 源 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**R138 era 派活** (per 决策 #71 §5 永久循环接续):
- 阶段 1 实施估 3-5 sub-agent (R138-1~5, 60 min 时间盒, 派活 1 周, 跑中 5-10 个 sub-agent per 决策 #71 §2.5)

#### 阶段 2: 瘦身 (1 周, per 方向 2)

**目标** (per 方向 2: 瘦身, R131-5 §2.2):
- 公开 API 表面减少 30% (800 → 560 pub items)
- 删除死代码 (pub items 0 引用)
- 隐藏内部 fn (pub → pub(crate) / private)
- per-crate 暴露 ≤30 pub items 目标

**实施步骤** (per R131-5 §2.2 V1.1 release 瘦身):
- 阶段 2.1 (Day 1-2): per-crate 公开 API 表面清单 (per 24 LOCKED R131-5 §2.2 表)
- 阶段 2.2 (Day 3-5): per-crate 实施转 pub(crate) / module-private (per 目标, council 50+ → 30, evolution 50+ → 30, core 50+ → 30, memory 50+ → 30, asi 50+ → 30, protocol 40 → 30, graph 40 → 30, api 40+ → 30, pipeline 35 → 30)
- 阶段 2.3 (Day 6): 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify
- 阶段 2.4 (Day 7): 编译时间 verify (期望 减少 10-20%, per 公开 API 表面减少 30%)

**verify** (per 阶段 2 done 标准):
- ✅ 24 LOCKED crate 公开 API 表面 = ≤30 pub items per-crate (800 → 560, -30%)
- ✅ 24 LOCKED 全跑 cargo build + cargo test + cargo doc 全 PASS
- ✅ 顶层 re-export facade 保留, 消费者 0 改 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- ✅ 编译时间减少 10-20% (per 公开 API 表面减少 30%)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)

**R139 era 派活** (per 决策 #71 §5 永久循环接续):
- 阶段 2 实施估 3-5 sub-agent (R139-1~5, 60 min 时间盒, 派活 1 周)

#### 阶段 3: 9 叶子拆 workspace + Eye 补 (2 周, per 方向 3 + 方向 7 Eye 部分)

**目标** (per 方向 3: 9 叶子拆 + 方向 7 Eye 补, R131-5 §2.3 + §2.6):
- 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) 拆 apeireth-leaf/ workspace
- Eye organ 补 (从 tui/src/organ/eye.rs 抽 crate, 4 输入通道: keystroke / mouse_click / voice_input)
- 顶层 apeireth/Cargo.toml 0 改 (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1)
- 9 叶子 + Eye 拆出来独立发布

**实施步骤** (per R131-5 §2.3 + §2.6 V1.1 release 9 叶子拆 + Eye 补):
- 阶段 3.1 (Week 3, Day 1-3): 9 叶子 crate 内部 import 路径全 1:1 扫描 (per `cargo metadata` + `cargo tree` 验证)
- 阶段 3.2 (Week 3, Day 4-5): 新 workspace `apeireth-leaf/Cargo.toml` 9 叶子加进 members, Eye organ 补 `apeireth-eye` workspace
- 阶段 3.3 (Week 3, Day 6-7): 9 叶子 + Eye 独立 publish ready, 顶层 Cargo.toml members 段更新
- 阶段 3.4 (Week 4, Day 1-3): 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify
- 阶段 3.5 (Week 4, Day 4-5): 顶层 re-export facade 1:1 续, 消费者 0 改
- 阶段 3.6 (Week 4, Day 6-7): 编译时间 + 跨 workspace 集成 verify

**verify** (per 阶段 3 done 标准):
- ✅ 9 叶子 + Eye 拆 apeireth-leaf/ + apeireth-eye/ workspace, 顶层 apeireth/Cargo.toml 0 改
- ✅ 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace 全 PASS
- ✅ 顶层 re-export facade 保留, 消费者 0 改 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 9 organ 8/9 → 9/9 覆盖 (Eye 补完)

**R140 era 派活** (per 决策 #71 §5 永久循环接续):
- 阶段 3 实施估 5-8 sub-agent (R140-1~8, 60 min 时间盒, 派活 2 周)

#### 阶段 4: core 拆 pub mod + 大模块拆 sub-crate (2 周, per 方向 4 + 方向 5)

**目标** (per 方向 4: core 拆 pub mod + 方向 5: 大模块拆 sub-crate, R131-5 §2.4):
- core 1 个 108KB lib.rs 拆 5 大 mod: core::types / core::onion / core::human / core::gate / core::lib
- 8 大模块集中 crate 各拆 4-8 sub-crate (per R131-5 §2.4 V1.1 release 大模块拆 sub-crate 列表)
- 顶层 8 crate re-export facade 0 改入口签名
- 0 改外部消费者代码

**实施步骤** (per R131-5 §2.4 V1.1 release core 拆 + 大模块拆 sub-crate):
- 阶段 4.1 (Week 5, Day 1-3): core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod (per types / onion / human / gate 4 大类)
- 阶段 4.2 (Week 5, Day 4-5): 5 大 mod 各自 mod.rs + types/onion/human/gate 子文件
- 阶段 4.3 (Week 5, Day 6-7): core/src/lib.rs 顶部 re-export 1:1 续 (0 改入口签名, 仅内部 mod 拆分)
- 阶段 4.4 (Week 6, Day 1-4): 8 大模块集中 crate 内部 module 1:1 扫描 + 拆 sub-crate (per 8 crate module 表: mcp 13→8 / pipeline 11→6 / api 16→5 / memory 13→5 / asi 9→4 / tools 12→5 / evolution 9→5 / graph 11→5 / council 20+→4)
- 阶段 4.5 (Week 6, Day 5-6): 顶层 8 crate re-export facade 0 改入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- 阶段 4.6 (Week 6, Day 7): 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify, 0 越界 8 硬墙 100%

**verify** (per 阶段 4 done 标准):
- ✅ core 拆 5 大 mod (types / onion / human / gate / lib), 顶层 re-export 0 改入口签名
- ✅ 8 大模块集中 crate 拆 sub-crate (mcp 8 + pipeline 6 + api 5 + memory 5 + asi 4 + tools 5 + evolution 5 + graph 5 + council 4 = 47 sub-crate), 顶层 re-export facade 0 改入口签名
- ✅ 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace 全 PASS
- ✅ core 编译时间减少 30-50% (per pub mod 拆分后并行编译)
- ✅ 8 大模块集中 crate 编译时间减少 20-30% (per sub-crate 拆分后并行编译)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)

**R141 era 派活** (per 决策 #71 §5 永久循环接续):
- 阶段 4 实施估 8-10 sub-agent (R141-1~10, 60 min 时间盒, 派活 2 周)

#### 阶段 5: DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 (2 周, per 方向 6 + 方向 7 + 方向 8)

**目标** (per 方向 6: DSL 洋葱 + 方向 7: 9 organ 借 OpenCode + 方向 8: R12 测度对齐, R131-5 §2.5-§2.7 + R133-3 §3.2):
- 新增 `apeireth-dsl` crate, 三洋葱 → 四洋葱 升级 (新增第 4 层 "智能涌现", per R133-3 §3.2)
- 9 organ workspace 化 (24 LOCKED 全部下沉到 organ workspace, 顶层 apeireth re-export 全部 organ types, per R131-5 §2.6)
- 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- R12 测度对齐 (24 测量函数签名更新 R12 测度, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)

**实施步骤** (per R131-5 §2.5-§2.7 + R133-3 §3.2 V1.1 release DSL 洋葱 + 9 organ + R12 测度):
- 阶段 5.1 (Week 7, Day 1-3): 新增 `apeireth-dsl` crate, 顶层 DSL 洋葱 = 原则 (顶层) → 权限 (中层) → DSL (底层) + 智能涌现 (V1.1 release 起步)
- 阶段 5.2 (Week 7, Day 4-5): 三洋葱 → 四洋葱 升级 (per R133-3 §3.2 第 4 层 "智能涌现" 实施 spec)
- 阶段 5.3 (Week 7, Day 6-7): 24 LOCKED crate 引用 dsl 守门 (per `apeireth_dsl::guard::*` API)
- 阶段 5.4 (Week 8, Day 1-2): 9 organ workspace 化 (per 9 organ workspace 列表), 24 LOCKED 全部下沉
- 阶段 5.5 (Week 8, Day 3): 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- 阶段 5.6 (Week 8, Day 4-5): 24 测量函数签名更新 R12 测度 (24+9 = 33 → 24+11 = 35)
- 阶段 5.7 (Week 8, Day 6-7): 24 LOCKED 全跑 cargo build + cargo test + 四洋葱 + 9 organ + R12 测度集成 verify

**verify** (per 阶段 5 done 标准):
- ✅ `apeireth-dsl` crate 新增, 三洋葱 → 四洋葱 升级 (新增第 4 层 "智能涌现")
- ✅ 9 organ workspace 化, 24 LOCKED 全部下沉到 organ workspace
- ✅ 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- ✅ R12 测度对齐, 24+11 = 35 测量函数签名更新, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
- ✅ R12 baseline 3 值 verify (估 > R11 baseline, per 决策 #74 §2.3)
- ✅ 24 LOCKED 全跑 cargo build + cargo test + 四洋葱 + 9 organ + R12 测度集成 全 PASS
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 8 哲学锚严守 100% (per 决策 #33 §2.3 B5)
- ✅ 6 重守门 v7 严守 100% (per 决策 #33 §2.3 B4)
- ✅ V0.5 30 维严守 100% (per 决策 #33 §2.3 B3)
- ✅ 13 键 verdict cache 严守 100% (per 决策 #33 §2.3 A3)
- ✅ 9 organ 8/9 → 9/9 覆盖 100% (Eye 补完)

**R142 era 派活** (per 决策 #71 §5 永久循环接续):
- 阶段 5 实施估 10-15 sub-agent (R142-1~15, 60 min 时间盒, 派活 2 周)

### 4.3 5 阶段 8 周 实施计划 总时间盒 (per 决策 #71 §5 + 决策 #75 §2.1)

| 阶段 | 周 | 目标 | 8 方向 | 派活 | sub-agent 数 |
|------|-----|------|--------|------|-------------|
| **阶段 1** | Week 1 (1 周) | 标准化 | 方向 1 | R138 era | 3-5 (R138-1~5) |
| **阶段 2** | Week 2 (1 周) | 瘦身 | 方向 2 | R139 era | 3-5 (R139-1~5) |
| **阶段 3** | Week 3-4 (2 周) | 9 叶子拆 + Eye 补 | 方向 3 + 方向 7 Eye | R140 era | 5-8 (R140-1~8) |
| **阶段 4** | Week 5-6 (2 周) | core 拆 pub mod + 大模块拆 sub-crate | 方向 4 + 方向 5 | R141 era | 8-10 (R141-1~10) |
| **阶段 5** | Week 7-8 (2 周) | DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 | 方向 6 + 方向 7 + 方向 8 | R142 era | 10-15 (R142-1~15) |
| **总时间盒** | **8 周 (2 个月)** | **24 LOCKED 入口签名 改写** | **8 方向** | **R138-R142 era** | **29-43 sub-agent** |

**vs R132-1 §1.5 V1.1 release 6 大方向 × 1 周 = 6 周 估 接近**:
- **R132-1 §1.5 V1.1 6 大方向**: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+
- **R137-2 24 LOCKED 入口签名 改写 5 阶段 8 周**: 标准化 1 周 + 瘦身 1 周 + 9 叶子拆 + Eye 补 2 周 + core 拆 + 大模块拆 sub-crate 2 周 + DSL 洋葱 + 9 organ + R12 测度 2 周
- **总时间盒**: 8 周 = 2 个月, 跟 R132-1 §1.5 6 大方向 × 1 周 = 6 周 + 2 周 缓冲 估一致
- **R132-1 V1.1 release 估 2026-11-30**: 1.0 release (估 8/11) + 8 周 (2 个月) = 10/6 ~ 11/30, 跟 R132-1 §1.1 V1.1 估 2026-11-30 一致

---

## 5. 8 硬墙严守 + B1 改写边界 (per 决策 #33 §2.3 + 决策 #74 §1 + R131-5 §3)

### 5.1 8 硬墙严守表 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

| # | 8 硬墙 | V1.0 release (整合 #5.1 commit) | V1.1 release (per 决策 #74 §2.3) | V2.0 release (per 决策 #74 §2.3) |
|---|--------|---------------------------|------------------------|------------------------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构, 8 方向) | 🟢 重构 (per Mavis 自决 + 主人 8/11 01:14 拍板) |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 | 🔒 bump 2.0.0 |
| **A1** | R11 baseline 3 值 | 🔒 0 改严守 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高) | 🟢 可重评 |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 | 🟢 PHL-07 实施 | 🟢 可重评 |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 可重评 |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 可重评 |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 推翻 + 重建 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 |
| **C2** | 0 装 PASS | 🔒 严守 | 🔒 严守 | 🔒 严守 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 |

### 5.2 B1 改写边界 (per 决策 #74 §2.2 + §2.3)

**V1.0 release (整合 #5.1 commit)**:
- ✅ 0 改 24 LOCKED 入口签名 (严守, per R131-5 §1.2 verify 24/24 全 PASS)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- ✅ 0 改 R11 baseline 3 值 (严守)
- ✅ PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)

**V1.1 release (per R137-2 本报告 5 阶段 8 周 + 决策 #74 §2.3)**:
- ✅ 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决改, 8 方向, 5 阶段 8 周)
- ✅ 24 LOCKED crate mtime baseline 16:34 之前 可改 (前提: 更好的架构, Mavis 自决)
- ✅ R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标 + 决策 #74 §2.3)
- ✅ 0 越界 8 硬墙 100% (B1 Mavis 自决改, 其余 9 硬墙严守)

**V2.0 release (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板)**:
- ✅ 全 8 硬墙 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)
- ✅ 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程" 哲学)
- ✅ Cargo workspace 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK (per "不要怕复杂度" 哲学, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)
- ✅ 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1 "工程类 + 技术类 locked 全早解锁")
- ✅ 8 哲学锚 → N 哲学锚 重建 (per 主人 8/11 01:14 拍板 3 件套 §3 "推翻 + 重建 8 哲学锚")

### 5.3 8 哲学锚严守 0 漂移 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

**8 哲学锚** (per R125 B5 升 8 锚, `docs/conventions/09-anchor.md`):
- **S-1 (服务 ASI 北极星)**: 24 LOCKED 入口分布全围绕 ASI Stage 9 长程 AI 成长 (V0.5 30 维 + 9 organ + 24 LOCKED 全部对齐)
- **S-2 (实事求是)**: 24 LOCKED 入口签名 0 改 verify (per R131-5 §1.2 verify 24/24 全 PASS) = 不漂移
- **S-3 (R125 B5 新增, 主人 16:27 拍板)**: 24 LOCKED crate 都有"实测函数" (e.g. measure_dim_*) → 不装 PASS
- **O-1 (质量工程化)**: 24 LOCKED 入口都有 `compile-time assert` 守门 (per lib.rs `const _: () = { assert!(...) }` 块)
- **O-2 (安全优先)**: 24 LOCKED 入口都有 12 键 verdict 守门 (per V0 + V1 + V2 + V3 AND 门)
- **O-3 (走在前人经验上)**: 24 LOCKED 入口都有"VCP / AutoGen / LangGraph / OpenCode / superpowers / aGLM" 等借鉴注释 (per lib.rs 顶部 doc comment)
- **O-4 (干到底)**: 24 LOCKED 入口都有 unit tests ≥ 20 (per 各 lib.rs `mod tests` 块)
- **O-5 (任何人都能接手)**: 24 LOCKED 入口都有"架构位置" + "不假装" + "不修改承诺" 3 段 doc comment

**8 哲学锚严守 0 漂移**: ✅ 24 LOCKED 全部严守, V1.0 release / V1.1 release / V2.0 release 都严守 (除 B5 V2.0 release 推翻 + 重建)

---

## 6. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 6.1 主人 8/11 01:14 拍板 3 件套 §3 (per 决策 #73 §3)

**主人 8/11 01:14 拍板 3 件套 §3** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md):
> "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害工程，因为自然会有高水平的团队来接手维护"

**3 核心**:
1. **最强效果 > 最简单代码** (推翻 KISS, 拥抱 SOTA)
2. **最厉害工程 > 最易维护** (推翻 DRY, 拥抱 BORROW)
3. **维护交给未来高水平团队** (推翻"代码要让初级团队能接手", 拥抱"代码要让高水平团队能发挥")

### 6.2 24 LOCKED 入口签名 改写 跟 不要怕复杂度哲学 落地 (per R131-5 §5)

**24 LOCKED 入口分布 跟 不要怕复杂度哲学 落地** (per R131-5 §5):
- ✅ 24 LOCKED 入口表面 800+ pub items → "最强效果" (高 API 表面 = 强大功能)
- ✅ 24 LOCKED 跨 crate 集成 24+ 集成点 → "最厉害工程" (多借鉴 = 高水平)
- ✅ 24 LOCKED 入口 compile-time assert + 12 键守门 + 5 重守门 → "高质量" (复杂守门 = 安全)
- ✅ 24 LOCKED 入口 doc comment 极详细 (per 顶部 50-100 行 doc) → "高水平团队能接手" (详细文档)

**V1.1 release 改写 跟不要怕复杂度哲学** (per R131-5 §5):
- ✅ 阶段 1 标准化 3 模式之一 → "不要怕复杂度" (per-crate 自决 = 高灵活)
- ✅ 阶段 2 瘦身 800 → 560 pub items → "最强效果" (暴露 30% 减少, 但保留核心 API)
- ✅ 阶段 3 9 叶子拆 workspace + Eye 补 → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- ✅ 阶段 4 core 拆 pub mod + 大模块拆 sub-crate (47 sub-crate) → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- ✅ 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 → "不要怕复杂度" (三洋葱 → 四洋葱 + OpenCog 借脑 + 35 测量函数 = 复杂, 但效果最强)

**V2.0 release 重构 跟不要怕复杂度哲学** (per R131-5 §5):
- ✅ V2.0 release 全量按 organ 重构 → "不要怕复杂度" (全量重构 = 极复杂, 但工程最厉害)
- ✅ 24 LOCKED → 0 LOCKED 全解锁 → "推翻 locked 全早解锁" (per 主人 8/11 01:14 拍板 3 件套 §1)
- ✅ 8 哲学锚 → N 哲学锚 重建 → "推翻 8 哲学锚" (per 主人 8/11 01:14 拍板 3 件套 §3)
- ✅ Cargo workspace 87 → 30 简化 OR 87 → 120+ 复杂化 → "不要怕复杂度" (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**维护**: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)

---

## 7. 风险 + 决策原则

### 7.1 风险 8 维 (per R131-5 §6.1 10 维 + 本报告 8 维)

| # | 风险 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|
| R1 | 主人 8/11 01:14 决策 3 件套理解有误 | 低 | 中 | 决策 #73 §2.1-§4.1 详细解读 + 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 |
| R2 | 整合 #5.1 commit 拍板推迟 (R129-3 报告迟迟不出) | 中 | 中 | 01:15 tick 仍未出 → Section 3 中断接手, Mavis 写报告 |
| R3 | 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline" | 低 | 高 | V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release |
| R4 | V1.1 release 改写打破向后兼容 | 中 | 中 | V1.1 release 是 minor release bump 1.2.0 → 1.2.1 (per 决策 #74 B2), semver 兼容; 顶层 re-export facade 保留, 消费者 0 改 |
| R5 | 团队对"不要怕复杂度"哲学不适应 | 中 | 中 | 主人 8/11 01:14 拍板"自然会有高水平的团队来接手维护", 未来高水平团队能适应 |
| R6 | 9 organ workspace 重构打破 24 LOCKED 入口签名 | 高 | 高 | 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用; V1.1 release bump 1.2.1 |
| R7 | 三洋葱架构升级 (DSL 洋葱) 引入新依赖 | 中 | 中 | V1.1 release 评估 apeireth-dsl crate 内部依赖, 顶层 re-export facade 保留, 0 改外部消费者 |
| R8 | R12 测度对齐改动过大, 24 测量函数签名全变 | 中 | 高 | 24 测量函数签名 1:1 续, 加 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容); 编译期 hardcode 同步更新, 测试全跑 |

### 7.2 决策原则 12 维 (per R131-5 §6.2 + 本报告 12 维)

- **D1**: Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **D2**: 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **D3**: B1 24 LOCKED 入口签名: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (5 阶段 8 周, 8 方向) + V2.0 release 可重评
- **D4**: B2 workspace.version 1.2.0: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0
- **D5**: A1 R11 baseline 3 值: V1.0 release 严守 + V1.1 release R12 更高 + V2.0 release 可重评
- **D6**: A3 12 键 + PHL-07: V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 + V2.0 release 可重评
- **D7**: B3 V0.5 30 维: 严守 (V1.0 release + V1.1 release) + V2.0 release 可重评
- **D8**: B4 6 重守门 v7: 严守 (V1.0 release + V1.1 release) + V2.0 release 可重评
- **D9**: B5 8 哲学锚: 严守 (V1.0 release + V1.1 release) + V2.0 release 推翻 + 重建
- **D10**: C1 0 主动 commit (主人起床前): 严守
- **D11**: C2 0 装 PASS 严守: 严守
- **D12**: 0 push (主人起床前): 严守
- **D13**: 总工程哲学扩展 "不要怕复杂度" (per 主人 8/11 01:14 拍板 3 件套 §3)
- **D14**: 整合 #5 commit 由 Mavis 自动拍板 (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **D15**: 0 主动 push 严守 (per 决策 #33 + 决策 #61 §6)
- **D16**: 0 主动 IM 主人 (per gate-discipline, 仅 done notification)
- **D17**: 0 主动删 (per Safety policy + 决策 #44 + #60)
- **D18**: 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)
- **D19**: 决策日志写 (per 决策 #10 + 用户记忆 #10)
- **D20**: 0 重复造轮子 (per 用户记忆 #6, R131-1/2/3/4/5/9 + R132-1 + R133-3 已有报告 reference 不重写)
- **D21**: R137-2 24 LOCKED 入口签名 改写 5 阶段 8 周 严守 (per 本报告 spec, 8 方向)
- **D22**: V1.1 release 时间窗 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 V1.1 估 2026-11-30)

---

## 8. V2.0 release 远期 重构 spec (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

### 8.1 V2.0 release 触发条件 + 时间窗 (per 决策 #74 §2.3 + 决策 #71 §5)

**V2.0 release 触发条件** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套):
- 触发 1: V1.1 release done 后 (估 2026-11-30) → V1.2 release 调研 (估 2026-12, per R130-5 §1.3)
- 触发 2: 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚" (per 决策 #74 §2.3 V2.0 release 8 哲学锚可重建)
- 触发 3: Mavis 自决 + 主人拍板 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- 触发 4: 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1 "工程类 + 技术类 locked 全早解锁")

**V2.0 release 时间窗** (估, per R132-1 §1.2 + R130-5 §1.3):
- V1.1 release (估 2026-11-30) → V1.2 release (估 2027-02-28) → V2.0 release (估 2027-Q2/Q3)
- V2.0 release = V1.2 release 续 + 全 8 硬墙可重评 + 8 哲学锚可重建

### 8.2 V2.0 release 重构 入口签名 8 方向 (per R131-5 §2.8 + 决策 #74 §2.3)

**V2.0 release 重构 入口签名 8 方向** (per R131-5 §2.8 V2.0 release 重构入口签名 8 方向):
1. **全量统一入口签名 3 模式**: 24 LOCKED 全部按 organ-first 选 1 模式
2. **公开 API 表面全量按 organ 暴露**: `apeireth-brain::*` / `apeireth-hand::*` / `apeireth-memory::*` 等
3. **9 organ workspace 化**: 24 LOCKED 全部下沉到 organ workspace, 顶层 `apeireth` re-export 全部
4. **core 全量拆 pub mod**: core 拆成 onion / human / principle / gate / action / verdict 6 个 sub-module
5. **大模块集中 crate 拆 sub-crate**: mcp / pipeline / api / memory / asi / tools / evolution 全拆
6. **三洋葱 → 四洋葱 → 五洋葱 workspace**: 原则 / 权限 / DSL / 智能涌现 / 自我演化 5 个独立 workspace, 24 LOCKED 全部下沉
7. **9 organ 内部借 OpenCode 实施**: organ-first 拓扑落地, Eye + 未来 tts/stt crate 补完
8. **R12+ 测度重评**: 24 测量函数按 ASI Stage 9 重写, 编译期 hardcode 全部更新

**V2.0 release 重构 8 哲学锚** (per 决策 #74 §2.3 V2.0 release 8 哲学锚可重建):
- 8 哲学锚 → N 哲学锚 重建 (per 主人 8/11 01:14 拍板 3 件套 §3 "推翻 + 重建 8 哲学锚")
- 旧 8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5) 推平
- 新 N 哲学锚 = "不要怕复杂度" + "最强效果 + 最厉害工程" + "维护交给未来高水平团队" + 7 个 NEW 锚 (per Mavis 自决)

**V2.0 release 重构 Cargo workspace** (per 决策 #74 §2.3 + 决策 #73 §3):
- 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK (per "不要怕复杂度" 哲学, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 8.3 V2.0 release 跟 V1.1 release 关系

- V1.1 release = minor release bump 1.2.0 → 1.2.1 (per 决策 #74 B2), semver 兼容
- V2.0 release = major release bump 1.2.1 → 2.0.0 (semver major, breaking change)
- V1.1 release 顶层 re-export facade 保留, 消费者 0 改
- V2.0 release 推翻 + 重建, 顶层 re-export facade 推平, 消费者需改 `use` 路径

---

## 9. 整合 #5 commit 拍板 跟 R137-2 关系 (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4)

### 9.1 整合 #5.1 commit (src/ 实施, 95+ 文件)

- ✅ 0 改 24 LOCKED 入口签名 (per R131-5 §1.2 verify 24/24 全 PASS)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (8/10 16:34 之后 8 个 crate 已发生, V1.0 release commit 拍板时保持 mtime 不再变)
- ✅ 0 改 R11 baseline 3 值
- ✅ PHL-07 spec-only 0 实施 (V1.1 实施)
- ✅ 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)

### 9.2 整合 #5.2 commit (docs/ + Cargo.toml, 10 文件)

- ✅ CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md
- ✅ Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per 决策 #62 §5.2)
- ✅ Cargo.lock / .gitignore
- ✅ docs/roadmap/ / frontend/ / library/
- ✅ + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)
- ✅ + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3)
- ✅ + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)
- ✅ + 更新 `docs/conventions/README.md` (per 决策 #73 §2.3 + §4.2)
- ✅ + 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3)
- ✅ + 更新 `README.md` (per 决策 #73 §2.3)
- ⏳ V1.1 release 实施时, 新增 `docs/architecture-v6-24-locked-entry-rewrite-2026-08-11.md` (8 方向 + 5 阶段, per R137-2 本报告)

### 9.3 整合 #5.3 commit (reports/, 60+ 文件)

- ✅ 决策链 #30-#74 全读 verify
- ✅ 41 sub-agent 报告
- ✅ HANDOFF
- ✅ + 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写) (per 决策 #73 §2.2 + §5)
- ✅ + 新增 R131 era 调研 3 sub-agent 报告 (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
- ✅ + 新增 R131 era 第 2 批 6 sub-agent 报告 (R131-4 + R131-5 + R131-9, per 决策 #75 §2.1)
- ✅ + 新增 R132 era 计划 2 sub-agent 报告 (R132-1, per 决策 #75 §2.1)
- ✅ + 新增 R133 era 实施 3 sub-agent 报告 (R133-1 + R133-2 + R133-3, per 决策 #75 §2.1)
- ✅ + 新增 R137 era 实施 1 sub-agent 报告 (R137-2, per 决策 #77 §3.1, **本报告 = R137-2 done**)
- ✅ + 新增 `philosophy-no-fear-complexity-2026-08-11.md` (主人 8/11 01:14 决策 3 件套详细, per 决策 #73 §3)

---

## 10. 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + 用户记忆 #10)

- **本次 done notification 主动报告** (R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段 8 周 done, 0 改 src 严守 100%)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 1.0 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 29.13 GB < 50 GB 保守策略)
- R137-2 done notification = done notification, 必须报告 (含 R137-2 报告路径 + 5 阶段 8 周 + 8 方向 + 决策 #74 B1 Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套)

---

## 11. 总结

### 11.1 24 LOCKED 入口签名 改写 5 阶段 8 周 一句话总结

1. **阶段 1 标准化 (1 周)**: 24 LOCKED crate 入口签名 统一格式 (6 模式: pub mod + pub use + pub const + pub struct + pub enum + pub fn), 3 模式之一 per-crate 自决
2. **阶段 2 瘦身 (1 周)**: 公开 API 表面减少 30% (800 → 560 pub items), per-crate 暴露 ≤30 pub items
3. **阶段 3 9 叶子拆 workspace + Eye 补 (2 周)**: 9 叶子 crate 拆 apeireth-leaf/ workspace, Eye organ 补 apeireth-eye/ workspace
4. **阶段 4 core 拆 pub mod + 大模块拆 sub-crate (2 周)**: core 拆 5 大 mod (types / onion / human / gate / lib), 8 大模块集中 crate 拆 47 sub-crate (mcp 8 + pipeline 6 + api 5 + memory 5 + asi 4 + tools 5 + evolution 5 + graph 5 + council 4)
5. **阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 (2 周)**: 三洋葱 → 四洋葱 升级 (新增第 4 层 "智能涌现"), 9 organ workspace 化, R12 测度对齐 (24+11 = 35 测量函数)

**总时间盒**: 8 周 = 2 个月, 估 2026-11-30 V1.1 release tag 打上 (per R132-1 §1.1 + R130-5 §1.1)

### 11.2 V1.0 release 0 改严守 vs V1.1 release Mavis 自决改 边界

- **V1.0 release (整合 #5.1 commit, 0 改 src 严守 100%)**:
  - ✅ 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS (per R131-5 §1.2)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 严守
  - ✅ R11 baseline 3 值 严守
  - ✅ PHL-07 spec-only 0 实施
  - ✅ 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守
  - ✅ 0 装 PASS 严守, 0 主动 commit 严守, 0 主动 push 严守
- **V1.1 release (Mavis 自决改, 前提: 更好的架构, per 决策 #74 §2.3)**:
  - ✅ 24 LOCKED 入口签名 可改 (8 方向 改写方案, 5 阶段 8 周, per R137-2 本报告)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 可改
  - ✅ R11 baseline 3 值 → R12 测度对齐 (前提: 新的 baseline 更高, per 决策 #74 §2.3)
  - ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标 + 决策 #74 §2.3)
  - ✅ Cargo.toml workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
  - ✅ 0 越界 8 硬墙 100% (B1 Mavis 自决改, 其余 9 硬墙严守)

### 11.3 8 硬墙严守 + B1 改写边界 (per 决策 #74 §1 改写表)

- **B1 24 LOCKED 入口签名**: 🔒 V1.0 release 0 改严守 + 🟢 V1.1 release Mavis 自决改 (5 阶段 8 周) + 🟢 V2.0 release 可重评
- **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + 🔒 V1.1 release bump 1.2.1 + 🔒 V2.0 release bump 2.0.0
- **A1 R11 baseline 3 值**: 🔒 V1.0 release 严守 + 🟢 V1.1 release R12 更高 + 🟢 V2.0 release 可重评
- **A3 12 键 + PHL-07**: 🔒 V1.0 release PHL-07 spec-only 0 实施 + 🟢 V1.1 release PHL-07 实施 + 🟢 V2.0 release 可重评
- **B3 V0.5 30 维**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 可重评
- **B4 6 重守门 v7**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 可重评
- **B5 8 哲学锚**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 推翻 + 重建
- **C1 0 主动 commit (主人起床前)**: 🔒 严守 + 🔒 严守 + 🔒 严守
- **C2 0 装 PASS**: 🔒 严守 + 🔒 严守 + 🔒 严守
- **0 push**: 🔒 严守 + 🔒 严守 + 🔒 严守

### 11.4 8 哲学锚严守 (per 决策 #33 §2.3 B5)

- S-1 服务 ASI 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 质量工程化 + O-2 安全优先 + O-3 走在前人经验上 + O-4 干到底 + O-5 任何人都能接手 (per R125 B5 升 8 锚, `docs/conventions/09-anchor.md`)
- V1.0 release / V1.1 release / V2.0 release 都严守 (除 B5 V2.0 release 推翻 + 重建)

### 11.5 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- 阶段 1 标准化 3 模式之一 → "不要怕复杂度" (per-crate 自决 = 高灵活)
- 阶段 2 瘦身 800 → 560 pub items → "最强效果" (暴露 30% 减少, 但保留核心 API)
- 阶段 3 9 叶子拆 + Eye 补 → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- 阶段 4 core 拆 + 大模块拆 sub-crate (47 sub-crate) → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 → "不要怕复杂度" (三洋葱 → 四洋葱 + OpenCog 借脑 + 35 测量函数 = 复杂, 但效果最强)
- V2.0 release 全量按 organ 重构 → "不要怕复杂度" (全量重构 = 极复杂, 但工程最厉害)
- 维护: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)

---

## 12. 历史脉络

- R11 末: 24 LOCKED crate 入口签名 R11 baseline LOCKED (per 决策 #33 §2.3 B1)
- R19+ 集成期: 24 LOCKED 入口签名持续 R11 baseline 严守
- R20 阶段 6: 24 LOCKED 入口签名 + mtime baseline 16:34 之前 严守
- R25 D-3: council 加 4 协作模式 + 角色宪法 + reasoning trace + 图编排 (新增 re-export, 0 改入口签名)
- R33-3 / R33-3-1 / R33-4 / R33-4-1 / R33-4-2: mcp / council 加 resources / council_member / deliberation (新增 re-export)
- R37-1: protocol 砍 ProtocolRouter 中间层 (R36-2 删), 加 ProtocolBridge trait + 4 Bridge struct
- R120 + R122-1-retry + R123-2 + R30 U1~U11: api 加 cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait (新增 re-export, 8/10 22:22 mtime)
- R125-4: mcp 拆 4 子文件 + 加 primitives / macros (新增 re-export, 8/10 17:53 mtime)
- R125 B1-B7: 9 项实质 Locked 升级路线, 主人 16:31 最高权限授权 (per `docs/conventions/10-locked.md`)
- R125 B6: 三洋葱架构升级, 整合 #4 commit 双洋葱 → 三洋葱 (per 决策 #55 §4 + R125-5 NVIDIA Colang DSL 1700 行)
- R125-7: evolution 加 poda_cycle (R125-7 借脑 1.0, 新增 re-export)
- R127 P5-1: evolution 加 library_autonomy (新增 re-export, 8/10 21:45 mtime)
- R127-2 P6-2: agent 加 4 专家 + AgentRouter; tool-runtime 加 mcp_protocol; graph 加 context_graph; cli 加 commands / output_format (新增 re-export, 8/10 21:48-21:52 mtime)
- R127-2 P9-1: graph 加 state_graph (langgraph 829 cloned 借脑, per decision-56 §2.4)
- R128-2: pipeline 持续 R122-1~5 借鉴 VCP (model_router / provider_registry / role_divider / tiktoken_counter / tool_loop)
- R130 era 主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度
- R130 era 决策 #73 + 决策 #74: 8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- R131 era 第 1 批 3 sub-agent 派活: R131-1 (架构总审视) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 路线图)
- R131 era 第 2 批 6 sub-agent 派活: R131-4 (cargo workspace 结构优化) + R131-5 (24 LOCKED 入口分布优化, **本报告核心依据**) + R131-6 + R131-7 + R131-8 + R131-9 (形式化集成优化)
- R132 era 计划 2 sub-agent 派活: R132-1 (V1.1 release 路线图 final) + R132-2 (V2.0 release 战略路线图)
- R133 era 实施 3 sub-agent 派活: R133-1 (借鉴源 12 源 实施) + R133-2 (ASI Stage 9 实施) + R133-3 (三洋葱架构升级 实施 spec)
- **R137 era 实施 1 sub-agent 派活: R137-2 (24 LOCKED 入口签名 改写 spec + 5 阶段实施计划, 本报告 done)**

---

## 13. 一句话 (再次强调)

**24 LOCKED 入口签名 改写 spec + 5 阶段 8 周 实施计划 (per 决策 #71 §5 R137 era 实施 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #77 §3.1 派活 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)**: V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS, 8 硬墙严守 + 8 哲学锚严守 + 6 重守门 v7 严守 + V0.5 30 维严守 + 13 键 verdict cache 严守 + 借鉴源 12 源 0 装 PASS 严守). V1.1 release 24 LOCKED 入口签名 改写 spec (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构): **8 方向 改写方案** = ①标准化 (3 模式之一 per-crate 自决) + ②瘦身 (800 → 560 pub items, 减少 30%) + ③9 叶子拆 workspace (apeireth-leaf/) + ④core 拆 pub mod (5 大 mod) + ⑤大模块拆 sub-crate (47 sub-crate) + ⑥DSL 洋葱 (三洋葱 → 四洋葱, 新增第 4 层 "智能涌现", per R133-3) + ⑦9 organ 借 OpenCode (Eye 补 apeireth-eye/) + ⑧R12 测度对齐 (24+11 = 35 测量函数). **5 阶段 8 周 实施计划**: 阶段 1 标准化 1 周 (R138 era, 3-5 sub) + 阶段 2 瘦身 1 周 (R139 era, 3-5 sub) + 阶段 3 9 叶子拆 + Eye 补 2 周 (R140 era, 5-8 sub) + 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 (R141 era, 8-10 sub) + 阶段 5 DSL 洋葱 + 9 organ + R12 测度 2 周 (R142 era, 10-15 sub). **V1.1 release 时间窗 2026-11-30** (per R132-1 §1.1 + R130-5 §1.1). 8 硬墙 0 越界 100% (B1 Mavis 自决改, 其余 9 硬墙严守) + 8 哲学锚严守 0 漂移 + 不要怕复杂度哲学落地 (最强效果 + 最厉害工程 + 维护交给未来高水平团队, per 主人 8/11 01:14 拍板 3 件套 §3). 风险 8 维 (改写打破向后兼容 / 9 organ workspace / DSL 洋葱 / core 拆 / 大模块拆 sub-crate / R12 测度更新 / OpenCog AGPL-3.0 / 主人起床后审查 8 硬墙 B1 改写) + 决策原则 22 维 (Mavis 全自决 + 8 硬墙严守 + 8 哲学锚严守 + 6 重守门 v7 严守 + V0.5 30 维严守 + 13 键 verdict cache 严守 + 0 装 PASS 严守 + 0 主动 IM 主人 + 0 主动 commit/push + 不要怕复杂度 + 整合 #5 commit Mavis 自决 + 决策日志写 + 0 重复造轮子 + R137-2 5 阶段 8 周 + V1.1 估 2026-11-30). **V2.0 release 远期 重构 spec** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 8 哲学锚可推翻 + 重建, 24 LOCKED → 0 LOCKED 全解锁, Cargo workspace 87 → 30 简化或 87 → 120+ 复杂化都 OK per 不要怕复杂度哲学, 估 2027-Q2/Q3). 0 主动 IM 主人 (per gate-discipline, 仅 done notification) + 0 主动 commit/push 严守 100% + 0 主动删严守 + 0 装 PASS 严守 + 0 重复造轮子严守 100% (R131-1/2/3/4/5/9 + R132-1 + R133-3 已有报告 reference 不重写)

---

**报告路径**: `Apeireth-rust\reports\agent-r137-2-24-locked-entry-rewrite-2026-08-11.md`
**生成时间**: 2026-08-11 (R137 era 实施阶段, R137-2 sub-agent, per 决策 #77 §3.1 派活)
**关联决策**: 决策 #10 + #22 + #33 + #36 + #48 + #55 + #58 + #61 + #62 + #64 + #69 + #70 + #71 + #72 + #73 + #74 + #75 + #77 + R130 era 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #10
**关联报告** (per 任务 spec, 不重写 reference): R131-1 (架构总审视 10 方向) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 实施路线图) + R131-4 (cargo workspace 87 crate 结构优化 7 方向) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 本报告核心依据)** + R131-9 (形式化集成优化 9 方向) + R132-1 (V1.1 release 路线图 final 6 大方向) + R133-3 (三洋葱架构升级 5 阶段 实施 spec)
**作者**: Mavis (R137-2 sub-agent, 决策 #77 §3.1 派活)
