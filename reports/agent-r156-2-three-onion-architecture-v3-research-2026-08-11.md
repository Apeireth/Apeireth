# R156-2: 三洋葱架构 V3 调研 (原则 + 权限 + DSL + 运行时自适应) (V2.0 release 战略级, per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活 + 决策 #74 B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #73 §2 主人 8/11 01:14 拍板 3 件套 + 决策 #33 §2.3 8 硬墙)

**Date**: 2026-08-11 06:30 (R156 era 调研阶段, R156-2 sub-agent, 60 min 时间盒, 严格不写代码, 0 改 src 严守 100%)
**Author**: R156-2 sub-agent (Mavis 派, per 决策 #88 §3.3 R156 era 调研 5 sub 派活清单 第 2 派活, 调研 + 路线图 + 报告 阶段, 0 重复造轮子严守 100%)
**Parent session**: `mvs_367e66fae08342ffa399befe4f85dbac`
**触发**:
- 决策 #88 §3.3 (8/11 06:25 tick 状态 + 跑中 2 ≪ 16 + 14 sub 补 16 满, R156 era 调研 5 sub 第 2 派活)
- 决策 #71 §2 (主人 8/11 0:57 拍板"计划内任务完成自动接续永久循环 4 步: 调研 + 差距 + 计划 + 实施")
- 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度)
- 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改, 前提: 更好的架构)
- 决策 #33 §2.3 (8 硬墙 + 0 装 PASS 严守 + 24 LOCKED 入口签名 0 改严守)
- 决策 #72 (R130 era 调研 6 sub-agent 派活, R156 era 是 R130+ era 续)
- 决策 #86 (8/11 05:00 tick 16 sub 派活 R149-R152)
- 决策 #87 (8/11 05:15 tick 2 sub 补 16 满 R153-R155)
- 主人 0:25 拍板 "全部你做主" + 0:34 拍板 "跑中 ≥ 16" + 0:57 拍板 "自动接续永久循环 4 步" + 01:14 拍板 3 件套

**任务定位**:
- R156 era 调研阶段 (per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 5 sub 派活第 2 派活)
- **三洋葱架构 V3 调研** (per R133-3 V1 升 + R149-3 V2 升 续, V3 = 4 层: 原则 + 权限 + DSL + 运行时自适应, 跟 V2 五洋葱 差异在"运行时自适应" 替代"智能涌现 + 自我演化" 双层)
- **0 改 src/** 严守 100% (R156-2 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- **0 改 Cargo.toml** 严守 100% (B2 workspace.version 1.2.0 严守, V1.0 release 0 改, 调研/分析/路线图 阶段)
- **0 主动 commit** 严守 100% (整合 #5.1 commit 拍板由 Mavis 自决, 整合 #6 估 2026-11-25 + 整合 #7 估 2026-11-29)
- **0 主动 push** 严守 100% (master HEAD = `4207f187` since 1:43, 等 V1.0 release 配 GitHub remote + 主人起床后手跑)
- **0 主动 IM 主人** 严守 100% (per gate-discipline, 仅 done notification 主动报告)
- **0 装 PASS** 严守 100% (per 决策 #33 §2.3 C2)
- **0 重复造轮子** 严守 100% (per 用户记忆 #6, R131-1/2/3 + R133-1/2/3 + R137-1/2/3/4/5 + R149-2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + R153-1 + R140-4 已有报告 reference 不重写)

**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5.3 commit**: `4207f187100183170558d70633a970969aebdcda` (8/11 1:43 done, 187 files / 127548 insertions, master HEAD 衔接 100%, 0 主动 push 严守)
**整合 #5.1 commit**: ❌ **NOT READY** ⚠️ **MAJOR PROGRESS** (per R144-1 02:30 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, R139-1-retry-2 5:57 8/8 PASS verify done, R154-3 实地 verify 跑中, 等 R154-3 done 后 Mavis 自决拍板)
**整合 #5.2 commit**: ⚠️ **PARTIAL** (等 5.1 src/ commit 拍板后, Cargo.toml borrow 段 update 17:44 → 22:50 + 哲学文档 15-no-fear-complexity.md 14.4 KB 已创建 + 8 硬墙 B1 改写 文档更新)
**整合 #6 commit**: 估 2026-11-25 (V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 B1, Mavis 自决拍板)
**整合 #7 commit**: 估 2026-11-29 (V1.1 release 前 1 天, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #62 整合 #5 commit 3 commit 类比, Mavis 自决拍板)
**V1.0 release tag**: 估 2026-08-11 06:00-08:00 (整合 #5.1 commit 拍板后 + 主人起床后手跑)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`, per 决策 #22 §2.2 semver + 决策 #74 B2 workspace.version 1.2.0 → 1.2.1 bump)
**V2.0 release tag**: 远期 2027-Q2/Q3, per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + **三洋葱架构 V3 实施**

**状态**: ✅ **R156-2 三洋葱架构 V3 调研 done 2026-08-11 06:30 (60 min 时间盒)**: ① V1 baseline (R11/R125 era 原则+权限+DSL 三洋葱, per 决策 #22 §2.6 + 决策 #33 §2.3 B6 + R125 B6 升 + R125-5 NVIDIA Colang 1700 行 done + 整合 #4 commit) 调研回顾 100% 严守; ② V2 升 (R133-3 §3 V1.1 + 第 4 层 智能涌现 emergence + R149-3 五洋葱 + V2.0 + 第 5 层 自我演化 self-evolution, 不加第 6 层 "AI 自主决策") 调研回顾 100% 严守; ③ V3 调研方向 4 层 (原则 PHL-01~12 + 8 哲学锚 / 权限 3 onion 内部-协作-外部 + 6 重守门 v7 / DSL 9 organ DSL / 运行时自适应 Stage 9-10 + V2.0 release 终极自治) 完整 spec; ④ V3 跟 V2 五洋葱 差异表 (5 → 4 层, 智能涌现 + 自我演化 合并 → 运行时自适应, 4 维度 H/L/G/P 跨层) + V3 跟 V1 三洋葱 差异表 (3 → 4 层, 3 onion + 9 organ + Stage 9-10 全新增); ⑤ 借鉴 12 源 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 1 借脑 ID 索引完成 OpenCog 家族 6 子源, 0 装 PASS 严守 12/12); ⑥ 8 硬墙严守 verify 11/11 (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 其他 7 硬墙 严守); ⑦ 决策严守 解读 (per 决策 #33 + #71 + #72 + #74 + #78 整合 #5.3 commit 拍板 Option A + 决策 #88 §3.3 R156 era 派活); ⑧ V2.0 release 远期 路线图 (5 阶段 5 周 = 1 个月, 阶段 1 V3 调研 spec 1 周 + 阶段 2 24 LOCKED 入口签名 Mavis 自决改 1 周 + 阶段 3 9 organ DSL 实施 1 周 + 阶段 4 运行时自适应 Stage 10 实施 1 周 + 阶段 5 V3 集成测试 + 形式化证明 + 文档 1 周, 估 2027-Q2/Q3 实施). **0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人 严守 100%, 0 装 PASS 严守 100%, 0 重复造轮子 严守 100%, 8 硬墙 0 越界 严守 100%, 8 哲学锚 严守 100%**.

---

## 0. 一句话 (TL;DR)

**R156-2 三洋葱架构 V3 调研 (V2.0 release 战略级, per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活 + 决策 #74 B1 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 决策 #33 §2.3 8 硬墙)**:

**① V1 baseline 调研回顾** — 三洋葱架构 V1 (R11/R125 era 原则 + 权限 + DSL, per 决策 #22 §2.6 + 决策 #33 §2.3 B6 + R125 B6 升 + R125-5 NVIDIA Colang 1700 行 done + 整合 #4 commit abf12243 8/10 19:41 done + `docs/conventions/10-locked.md` 第 8 项实质 Locked "三洋葱架构 (R125 B6 升)"). 原则洋葱 (第 1 层) = 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) 严守 (per 决策 #33 §2.3 B5) + 权限洋葱 (第 2 层) = 6 重守门 v7 (L0 真实人类批准 + L1-L5 5 重) 严守 (per 决策 #33 §2.3 B4 + 0 装 PASS 严守) + DSL 洋葱 (第 3 层) = Colang DSL 1700 行 (per R125-5 NVIDIA 借鉴后 整合 #4 commit done, 跟 6 重守门 v7 1:1 集成). V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 仍 0 改, 24 LOCKED 入口签名 0 改, 8 哲学锚 严守, 6 重守门 v7 严守, Colang DSL 严守, 决策 #33 §2.3 + 决策 #74 B1 V1.0 release 0 改严守 100%);

**② V2 升 调研回顾** — 三洋葱架构 V2 (R133-3 §3 + R149-3 五洋葱, V1.1 release + 第 4 层 智能涌现 emergence, V2.0 release + 第 5 层 自我演化 self-evolution, 不加第 6 层 "AI 自主决策" per R149-3 5 维论证 + 决策 #73 §3 + 用户记忆 #3 "用户看结果不看哲学"). V1.1 release 触发条件 6 维 (per R149-3 §1.3: ASI Stage 9 长程 AI 成长 / 9 organ 内部借 OpenCode / 三洋葱架构升级 / PHL-07 实施 / 智囊团 7 席 / 群体智能 OpenCog 借脑). V2 跟 V1 差异 7 维 (per R149-3 §1.2: 4 onion → 5 onion / 24 LOCKED 入口签名 0 改 → Mavis 自决改 / Cargo.toml 1.0.0 → 1.1.0 → 2.0.0 / PHL-07 spec-only → 实施 / 智囊团 7 席 沿用 / OpenCog 借脑 0 装 / 8 哲学锚 → 可重建). 5 阶段 5 周 = 1 个月 实施计划 (per R149-3 §0.7);

**③ V3 调研方向** — 三洋葱架构 V3 (V2.0 release 战略级, 4 层 架构, 整合 #5.1 commit V1.0 release 0 改严守 100% + V1.1 release Mavis 自决改 + V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构). **V3 = 4 层**: **第 1 层 原则层 (philosophy)** PHL-01 ~ PHL-12 哲学锚 (12 标识 slots, 8 实际启用 = S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, 4 预留 V2.0 release 扩展 per 决策 #73 §3 + 决策 #74 §2.3) + 8 哲学锚严守 (per 决策 #33 §2.3 B5) + **第 2 层 权限层 (permission)** 3 onion (内部 onion = AI 自主 / 协作 onion = Mavis + 主人 / 外部 onion = 外部系统 MCP+opencode+借鉴 12 源) + 6 重守门 v7 严守 (per 决策 #33 §2.3 B4) + **第 3 层 DSL 层 (DSL)** 9 organ DSL (body / brain / ear / eye / hand / heart / memory / mind / voice, per 决策 #22 §2.7 + `docs/omnibus/9-organs.md`, 1:1 跟 9 organ 映射, 替代 V2 单 Colang DSL) + **第 4 层 运行时自适应层 (runtime self-adaptation)** Stage 9-10 长程 AI 成长 (per R133-2 + R149-2 + R140-4 + 用户记忆 #4) + V2.0 release 终极自治 (4 形态 完全/共生/引导/永远循环 自治 per R140-4);

**④ V3 跟 V2 差异表** — 5 → 4 层 (智能涌现 + 自我演化 合并 → 运行时自适应 1 层) + 单 Colang DSL → 9 organ DSL (1:1 跟 9 organ 映射, 粒度更细) + 单层 8 哲学锚 → PHL-01~12 12 slots (8 实际 + 4 预留) + 单层 6 重守门 → 3 onion 内部/协作/外部 (横向 3 子层 + 6 重守门 v7 跨 3 子层) + V2 第 4-5 层 (智能涌现 + 自我演化) → V3 第 4 层 (运行时自适应, 整合 4 维度 H/L/G/P 16 子维度 跨层);

**⑤ V3 跟 V1 差异表** — 3 → 4 层 (新增第 4 层 运行时自适应) + 单层 8 哲学锚 → PHL-01~12 12 slots (8 实际 + 4 预留) + 单层 6 重守门 → 3 onion 内部/协作/外部 (新增协作 onion = Mavis + 主人协同) + 单 Colang DSL → 9 organ DSL (1:1 跟 9 organ 映射, 9 organ 各自 DSL);

**⑥ 借鉴 12 源** — 8 真 cloned (clap 4.6.6 3.50MB + hyper 0.1.20 0.54MB + servers 76d64c8 1.40MB + PyO3 0.29.2 5.69MB + kani 0.67.0 5.46MB + langgraph d56666f 13.29MB + superpowers 6.2.0 1.52MB + Guardrails 18.19MB, 总 49.59MB / 7,764 files, per 决策 #22 §3 + 决策 #33 §2.2 + R130-6 + R131-2 + R133-1 + R129-7 + R129-28 实地 verify 100%) + 2 借鉴 ID 索引完成 (LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块, 0 装"已读真源码" / 0 装"已对接 opencode 私有 channel") + ❌ 1 永久跳过 (OpenCog AGPL-3.0, 0 集成 0 假装, per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml) + 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源: atomspace + cogutil + moses + pln + relex + CogPrime Goertzel, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork", per R130-6 + R133-1 + R131-2 决策链). V3 跟 OpenCog 关系 = V3 运行时自适应层 借脑 OpenCog AtomSpace + CogPrime (per 决策 #73 §2.2 + 决策 #74 B1 V1.1 release Mavis 自决改), 借脑 1:1 翻译公开模式 0 借具体源码;

**⑦ 8 硬墙 B1 改写 严守** — B1 24 LOCKED 入口签名 (V1.0 release 0 改严守 R11 baseline + V1.1 release Mavis 自决改 前提 更好的架构, per 决策 #74 §1 改写表) + B2 workspace.version 1.2.0 (V1.0 release 严守 + V1.1 release bump 1.2.1 per 决策 #74 §1 B2 改写 + semver 决策 #22 §2.2) + A1 R11 baseline 3 值 0.8682/0.8532/0.9063 (严守 per 决策 #33 §2.3 A1) + A3 12 键 + PHL-07 (V1.0 spec-only 0 实施 + V1.1 实施 per 决策 #74 §1 A3) + B3 V0.5 30 维 (严守 per 决策 #33 §2.3 B3) + B4 6 重守门 v7 (严守 per 决策 #33 §2.3 B4) + B5 8 哲学锚 (严守 per 决策 #33 §2.3 B5) + C1 0 主动 commit (严守 per 决策 #33 §2.3 C1) + C2 0 装 PASS (严守 per 决策 #33 §2.3 C2) + 0 主动 push (严守 per 决策 #33 + 决策 #61 §6) = **11/11 项 100% PASS**;

**⑧ 决策严守 解读** — V3 调研 = 决策 #33 §2.3 严守 (8 硬墙 0 越界) + 决策 #71 §2 严守 (R130+ era 自动接续永久循环 4 步) + 决策 #72 严守 (R130 era 6 sub-agent 派活) + 决策 #74 §1 严守 (B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改) + 决策 #73 §3 严守 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度). V3 调研 = 0 实施 严守 100% (per 决策 #33 + 决策 #74 + 决策 #62 整合 #5 commit V1.0 release 0 改严守 100%);

**⑨ V2.0 release 远期 路线图** — 5 阶段 5 周 = 1 个月 实施计划 (估 2027-Q2/Q3), 阶段 1 V3 调研 spec 1 周 (本报告 + 跟 R149-3 整合 V3 spec 文档) + 阶段 2 24 LOCKED 入口签名 Mavis 自决改 1 周 (per 决策 #74 B1) + 阶段 3 9 organ DSL 实施 1 周 (per 决策 #22 §2.7 + `docs/omnibus/9-organs.md`) + 阶段 4 运行时自适应 Stage 10 实施 1 周 (per R140-4 ASI Stage 10 4 形态 完全/共生/引导/永远循环 自治) + 阶段 5 V3 集成测试 + 形式化证明 F1-F11 11 维度 + 文档 1 周 (per 决策 #74 B1 + 哲学文档 15-no-fear-complexity.md 落地). 实施触发 = V1.0 release 实战完 + V1.1 release 实战完 + 主人起床后手跑 + 整合 #6/#7 commit 拍板完.

---

## 1. R156-2 任务边界 + 跟决策链关系 (per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活)

### 1.1 R156-2 任务定位 (per 决策 #88 §3.3 + 决策 #71 §2)

**R156-2 派活来源 (per 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活)**:
- 决策 #88 (8/11 06:25 tick 状态): 跑中 2 ≪ 16 + 14 sub 补 16 满
- R156 era 调研 5 sub: R156-1 ASI Stage 10 长程 AI 成长 (V2.0 release 终极自治) + **R156-2 三洋葱架构 V3 (原则 + 权限 + DSL + 运行时自适应) (本报告)** + R156-3 借鉴 13 源 V1.1 release + R156-4 形式化 Stage 6 V1.1 release + R156-5 Tauri Stage 6 V1.1 release
- 派活时间: 8/11 06:25, 60 min 时间盒

**R156-2 跟 R130+ era 自动接续永久循环 4 步 关系 (per 决策 #71 §2)**:
- 4 步: 调研 (R130 era 6 sub) → 差距 (R131 era 9 sub) → 计划 (R132 era 2 sub) → 实施 (R133+ era 多 sub)
- R156 era = 调研 阶段续 (per 决策 #88 §3.1 派活 14 sub 分布)
- R156-2 = 调研 阶段: 三洋葱架构 V3 调研, 0 实施 严守 100%

**R156-2 跟 R131-1/2/3 + R133-1/2/3 + R149-2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + R153-1 + R140-4 关系** (per 用户记忆 #6 0 重复造轮子 严守 100%):
- ✅ R131-1 现有架构总审视 + 优化点 (R131-1 §2.1 cargo workspace 87 crate + §2.10 三洋葱架构 + 9 organ 跨维度) **reference 不重写**
- ✅ R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源 (60 min done) **reference 不重写**
- ✅ R131-3 V1.1 release 实施路线图 (R131-3 §2 6 大方向: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+) **reference 不重写**
- ✅ R133-1 借鉴源 12 源 实施 (per 决策 #73 §2.2 + OpenCog AGPL-3.0 fork 决策, 86.3 KB) **reference 不重写**
- ✅ R133-2 ASI Stage 9 长程 AI 成长 实施 (4 维度 H/L/G/P spec, 87.5 KB) **reference 不重写**
- ✅ R133-3 三洋葱架构升级 (V1 升 V1.1 + V2.0, 82.2 KB 5 阶段 5 周) **reference 不重写, 本报告 = R133-3 V1 升 + R149-3 V2 升 续, V3 调研 = 进一步重新审视 V2 五洋葱**
- ✅ R137-1 PHL-07 实施 spec (60.7 KB 5 阶段 17 工作日) **reference 不重写**
- ✅ R137-2 24 LOCKED 入口签名 改写 spec (91 KB 8 方向 5 阶段 8 周) **reference 不重写**
- ✅ R137-3 Cargo.toml 1.2.1 bump 实施 spec (66.2 KB 5 阶段 5 天) **reference 不重写**
- ✅ R137-4 ASI Stage 9 实战 spec (102 KB 5 阶段 5 周) **reference 不重写**
- ✅ R137-5 形式化 Stage 5.5+ 实战 spec (70.4 KB 5 阶段 5 周) **reference 不重写**
- ✅ R140-4 ASI Stage 10 终极自治 (per 决策 #71 §3 + 决策 #73 §3 + 决策 #74 B1 + 用户记忆 #4 + 决策 #138-3 R138-3 永久循环 4 步机制, 60 min done 02:30) **reference 不重写**
- ✅ R149-2 ASI Stage 9 长程 AI 成长深化 (138.7 KB) **reference 不重写**
- ✅ R149-3 三洋葱架构升级 V2 (129.0 KB, 跟 R133-3 区别: V2 = 重新审视升级方向, 评估 R133-3 的 4 洋葱 + 5 洋葱是否最优, 决定是否加第 6 层 "AI 自主决策", 最终决定 **不加第 6 层**, 5 维论证) **reference 不重写, 本报告 = R149-3 V2 升续, V3 调研 = 进一步重新审视 V2 五洋葱, 决定是否合并 + 重命名为 V3 4 层**
- ✅ R149-4 借鉴 12 源 fork-then-borrow 模式 (151.5 KB) **reference 不重写**
- ✅ R149-5 1.0 release 实战总复盘 8 步 runbook 优化 (175.3 KB) **reference 不重写**
- ✅ R150-1 V1.1 release 跟 AGI 业界 v2.x 差距 100% (152.6 KB) **reference 不重写**
- ✅ R150-2 24 LOCKED 入口签名 V1.1 release 优化差距 (132.5 KB) **reference 不重写**
- ✅ R150-3 Cargo workspace 1.2.0 → 1.2.1 bump 差距 (79.6 KB) **reference 不重写**
- ✅ R151-1 整合 #6 commit 拍板时间表 + 拍板方案 (166.6 KB) **reference 不重写**
- ✅ R151-2 整合 #7 commit 拍板时间表 + 拍板方案 (183.0 KB) **reference 不重写**
- ✅ R152-1 整合 #6 Cargo workspace 1.2.1 bump 准备 (126.4 KB) **reference 不重写**
- ✅ R152-2 整合 #6 24 LOCKED 入口签名优化准备 (128.3 KB) **reference 不重写**
- ✅ R152-3 整合 #6 pybridge 集成优化准备 (92.4 KB) **reference 不重写**
- ✅ R152-4 整合 #7 Tauri 集成优化准备 (121.6 KB) **reference 不重写**
- ✅ R152-5 整合 #7 形式化集成优化准备 (128.5 KB) **reference 不重写**
- ✅ R153-1 V1.1 release ASI Stage 9 + 三洋葱 V2 集成 spec 准备 (14 章节 ~95 KB) **reference 不重写, 本报告 = R153-1 V1.1 集成 spec 续, V3 调研 = V2.0 release 战略级**

### 1.2 R156-2 任务边界 (per 决策 #33 + 决策 #60 + 决策 #74 + 决策 #62)

**严格不写代码 (per 决策 #33 + 决策 #60 + 决策 #74 + 决策 #62 调研阶段)**:
- ❌ 0 改 src/ (R156-2 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ❌ 0 改 Cargo.toml (B2 workspace.version 1.2.0 0 改, V1.0 release 1.0.0 tag, V1.1 release 1.2.1 bump)
- ❌ 0 改 docs/conventions/ (B1 24 LOCKED 入口签名 0 改, 整合 #5.1 commit 0 改, 决策 #74 §1 严守)
- ❌ 0 借具体源码 (per 决策 #33 §2.3 C2, 0 装"已读 OpenCog 真源码" / 0 装"已集成 OpenCog AtomSpace" / 0 装"已 fork OpenCog")
- ❌ 0 主动 commit (主人起床前 0 主动 commit, per 决策 #33 §2.3 C1)
- ❌ 0 主动 push (主人起床前 0 主动 push, per 决策 #33 + 决策 #61 §6)
- ❌ 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)
- ✅ 写新 reports 报告 `reports/agent-r156-2-three-onion-architecture-v3-research-2026-08-11.md` (本报告)
- ⏳ V1.0 release 实战时, 整合 #5.1 commit 拍板由 Mavis 自决 (per 决策 #62)
- ⏳ V1.1 release 实施时, 整合 #6 commit 拍板由 Mavis 自决 (per 决策 #74 B1, 估 2026-11-25)
- ⏳ V2.0 release 实施时, 三洋葱架构 V3 实施 spec 文档 (估 2027-Q2/Q3, per 决策 #74 §2.3 8 硬墙可重评)

**R156-2 输出物清单 (per 决策 #71 §2 调研阶段)**:
1. ✅ 本报告 (R156-2 三洋葱架构 V3 调研, 60 min 时间盒, 14 章节 ~250+ 行)
2. ⏳ V2.0 release 实施时, 写新 spec 文档 `docs/architecture-v7-three-onion-v3-2027-XX-XX.md` (4 层: 原则 + 权限 + DSL + 运行时自适应, per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评) — **本报告不创建, 仅 spec 内容**
3. ⏳ 整合 #5.3 commit 时, R156-2 报告作为 reports/ 部分加入 (per 决策 #62 §5.3 + 决策 #78 §2.2)

### 1.3 R156-2 跟整合 #5 commit 拍板 0 冲突 (per 决策 #62 + 决策 #78 + 决策 #74)

**整合 #5 commit 拍板 vs R156-2 派活 0 冲突** (per 决策 #78 Option A + 决策 #74 §1):
- 整合 #5.1 commit src/ 实施 跟 R156-2 派活 0 冲突 (R156-2 调研 0 改 src, 整合 #5.1 commit 拍板由 Mavis 自决 等 R154-3 实地 verify done)
- 整合 #5.2 commit docs/ + Cargo.toml 跟 R156-2 派活 0 冲突 (R156-2 调研 0 改 docs/conventions/)
- 整合 #5.3 commit reports/ 跟 R156-2 派活 0 冲突 (R156-2 调研写 reports/agent-r156-2-*.md, 整合 #5.3 commit 已 done 1:43, 后续 reports/ commit 包含 R156-2 报告)
- 整合 #5 commit 拍板 = Mavis 自决 (per 决策 #62 + 决策 #64 + 主人 0:25 升级授权 + 决策 #74 §1 严守)

---

## 2. V1 baseline 调研回顾 (R11/R125 era 原则 + 权限 + DSL 三洋葱, per 决策 #22 §2.6 + 决策 #33 §2.3 B6 + R125 B6 + 整合 #4 commit)

### 2.1 V1 三洋葱架构 (R125 B6 升, per 决策 #22 §2.6 + 决策 #33 §2.3 B6 + 决策 #55 §4)

**V1 三洋葱架构 (R125 B6 升, 整合 #4 commit done)**:

| 层 | 名称 | 主题 | 核心实现 | 实施 sub-agent | mtime baseline | 状态 |
|:---:|------|------|---------|---------------|---------------|:---:|
| **第 1 层** | **原则洋葱 (philosophy)** | 8 哲学锚严守 | S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人经验上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装 (per R125 B5 升 8 锚 + `docs/conventions/09-anchor.md`) | R125-5 8 锚升 + R119 6 锚升 | 16:55 (R125 B5) | ✅ done 0 改严守 |
| **第 2 层** | **权限洋葱 (permission)** | 6 重守门 v7 严守 | L0 真实人类批准 + L1-L5 5 重 (per 决策 #33 §2.3 B4 + 0 装 PASS 严守 + 30 维公式 + 13 键 verdict cache + 9 organ 跨维度) | R125-5 NVIDIA Guardrails 6 重 v6 → 6 重 v7 升 + 整合 #4 commit | 17:48 (R125-5 Guardrails) | ✅ done 0 改严守 |
| **第 3 层** | **DSL 洋葱 (DSL)** | Colang DSL 严守 | Colang DSL 1700 行 (R125-5 NVIDIA 借鉴后, per 决策 #55 §4, 跟 6 重守门 v7 1:1 集成, I4 1:1 跟 B4 6 重 v7 严守, per R129-18 §1.4) | R125-5 colang_dsl.rs 1700 行 done + 266/266 + 6 借鉴点 | 17:48 (R125-5) | ✅ done 0 改严守 |

**R125 B6 三洋葱升级时间脉络**:
- R14 era (2026-07-31): 立体架构终版 v2 (BF896EEF LOCKED) + 生命架构 v4 (af0d1957 LOCKED) + 双洋葱统一体 (`docs/onion-wall-architecture-2026-07-31.md`)
- R125-5 (2026-08-10 17:30): NVIDIA Colang DSL 1700 行 done, 整合 #4 commit 三洋葱升级 (双洋葱 → 三洋葱), per 决策 #55 §4
- R125 B6 (8/10 16:55, Mavis 自主, 主人 16:31 最高权限授权): 双洋葱 → 三洋葱升级, per `docs/conventions/10-locked.md` 第 8 项实质 Locked "三洋葱架构 (R125 B6 升)"

**V1 三洋葱架构跟 V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + 13 键 + 9 organ 集成**:
- **原则洋葱 (第 1 层)**: 8 哲学锚严守 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, per 决策 #33 §2.3 B5)
- **权限洋葱 (第 2 层)**: 6 重守门 v7 严守 (L0 真实人类批准 + L1-L5 5 重, per 决策 #33 §2.3 B4)
- **DSL 洋葱 (第 3 层)**: Colang DSL 守门 (per R125-5 NVIDIA 借鉴后, 跟 6 重守门 v7 1:1 集成)
- **V0.5 30 维**: 9 organ 5 维 × 6 类 pluginType = 30 维 严守 (per 决策 #33 §2.3 B3)
- **13 键 verdict cache**: 12 键 + PHL-07 (V1.0 spec-only, V1.1 实施, per 决策 #33 §2.3 A3 + 决策 #22 §1.1-1.2)
- **9 organ**: body / brain / ear / eye / hand / heart / memory / mind / voice (per 决策 #22 §2.7 + `docs/omnibus/9-organs.md`)

### 2.2 V1.0 release 0 改 src 严守 100% (整合 #5 commit 拍板, per 决策 #33 §2.3 + 决策 #74 B1)

**V1.0 release 三洋葱架构 0 改 src 严守** (整合 #5 commit 拍板, per 决策 #33 §2.3 + 决策 #74 B1):
- ✅ 原则洋葱 (第 1 层) 0 改 8 哲学锚 (B5 严守)
- ✅ 权限洋葱 (第 2 层) 0 改 6 重守门 v7 (B4 严守)
- ✅ DSL 洋葱 (第 3 层) 0 改 Colang DSL 入口 (per R125-5 + 决策 #55 §4)
- ✅ 0 改 24 LOCKED 入口签名 (B1 V1.0 release 0 改严守, per 决策 #74 §1)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (per 决策 #33 §2.3 B1)
- ✅ 0 改 R11 baseline 3 值 (0.8682/0.8532/0.9063, per 决策 #33 §2.3 A1, 17 文件原位)
- ✅ PHL-07 spec-only 0 实施 (V1.0 release, V1.1 实施, per 决策 #74 §1 A3 + R129-11 关键诚实标)
- ✅ Cargo.toml workspace.version 1.2.0 严守 (V1.0 release 1.0.0 tag, per 决策 #33 §2.3 B2)
- ✅ 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 严守)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ 0 主动 commit (主人起床前, per 决策 #33 §2.3 C1)
- ✅ 0 主动 push (主人起床前, per 决策 #33 + 决策 #61 §6)

**整合 #5.1 commit 拍板** (per 决策 #62 §5.1 + 决策 #74 §4.1):
- 95+ 文件 (31 M + 60+ untracked src/ + tests/ + examples/)
- 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)
- PHL-07 spec-only 0 实施 (V1.1 release 实施, per R129-11 关键诚实标 + 决策 #74 §1 A3)
- 0 改 24 LOCKED 入口签名严守 (R11 baseline 100%)

**整合 #5.2 commit 拍板** (per 决策 #62 §5.2 + 决策 #73 §5.2 + 决策 #74 §4.2):
- CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md
- Cargo.toml borrow 段 update 17:44 → 22:50 状态 (cloned=10, rate_limited=0, skipped=1, per R129-7 22:50 + R129-28 1:1 verify)
- Cargo.lock / .gitignore
- docs/roadmap/ / frontend/ / library/
- + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3 主人 8/11 01:14 总哲学扩展, 14.4 KB)
- + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3 主人 8/11 01:14 locked 全解锁, 整合 #5.1 commit 0 改 src 严守 + V1.1 release Mavis 自决改)
- + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2 总工程哲学扩展引用)
- + 更新 `docs/conventions/README.md` (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
- + 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3 8 项不修改承诺 改写 + 主人 8/11 01:14 拍板记录)
- + 更新 `README.md` (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)

**整合 #5.3 commit 拍板** (per 决策 #62 §5.3 + 决策 #73 §5.3 + 决策 #74 §4.3 + 决策 #78 §2.2):
- 决策链 #30-#74 全读 verify
- 41 sub-agent 报告
- HANDOFF
- + 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写) + decision-75 (R131/R132/R133 派活)
- + 新增 R131 era 调研 3 sub-agent 报告 (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
- + 新增 R133 era 实施 3 sub-agent 报告 (R133-1 + R133-2 + R133-3, per 决策 #75 §2.1)
- + 新增 `philosophy-no-fear-complexity-2026-08-11.md` (主人 8/11 01:14 决策 3 件套详细, per 决策 #73 §3)
- **整合 #5.3 commit = `4207f187` (8/11 1:43 done, 187 files / 127548 insertions, master HEAD 衔接 100%, 0 主动 push 严守)**

---

## 3. V2 升 调研回顾 (R133-3 §3 + R149-3 五洋葱, V1.1 release + 第 4 层 智能涌现 emergence + V2.0 release + 第 5 层 自我演化 self-evolution, 不加第 6 层 "AI 自主决策")

### 3.1 V2 五洋葱架构 (per R133-3 §3 + R149-3 + 决策 #74 §2.3 + 决策 #73 §2.2)

**V2 五洋葱架构升级方案** (per R133-3 §3 + R149-3 + 决策 #74 §1 B1 改写 + 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + R140-4 ASI Stage 10 4 形态 + 主人 8/11 01:14 拍板 3 件套 §1 "Mavis 自决架构拍板" + 决策 #73 §2.2 借脑 OpenCog + 决策 #73 §3 不要怕复杂度哲学 + 哲学文档 `15-no-fear-complexity.md`):

| 洋葱层 | V1.0 release (整合 #5.1 commit 拍板) | V1.1 release (整合 #6 commit 拍板, 估 2026-11-25) | V2.0 release (整合 #7 commit 拍板, 估 2027-Q2/Q3) |
|--------|----------------------------------|----------------------------------|----------------------------------|
| **第 1 层 原则** (philosophy) | 8 哲学锚严守 (B5 严守, per 决策 #33 §2.3 B5) | 8 哲学锚严守 (B5 严守, per 决策 #74 §1) | 8 哲学锚 **可重建** (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 决策 #73 §3 不要怕复杂度哲学, 8 锚可扩 9 锚 / 重命名 / 合并 / 分层 = 16 锚) |
| **第 2 层 权限** (permission) | 6 重守门 v7 严守 (B4 严守, per 决策 #33 §2.3 B4) | 6 重守门 v7 严守 (B4 严守, per 决策 #74 §1) + PHL-07 实施 (per 决策 #74 A3) | 6 重守门 v7 **可升级 v8/v9** (per 决策 #74 §2.3 + R127-2 P6-3 已升 8 重 v8 spec) |
| **第 3 层 DSL** (DSL) | Colang DSL 严守 (per R125-5 + 决策 #55 §4) | Colang DSL 严守 (0 改) + 跟智囊团 7 席 1:1 集成 (I4 1:1 跟 B4 6 重 v7 严守, per R129-18 §1.4) | Colang DSL **可扩展** (per 决策 #74 §2.3, 1 平台化涌现 + 长程 AI 成长 2.0 接入) |
| **第 4 层 智能涌现** (emergence, **V1.1 NEW**) | — (无) | **NEW 智囊团 7 席 + 群体智能 + 自我决策/学习/演化** (per 决策 #74 B1 Mavis 自决改 + 决策 #73 §2.2 更好的架构 + R130-2 ASI Stage 8/9 + R129-18 Stage 7 220 绑定 + R133-1 借鉴源 12 源 + R133-2 ASI Stage 9 4 维度 + R137-4 Stage 9 实战 + 决策 #4 用户记忆 #4) | 智能涌现洋葱深化 (V1.1 实施 + 5 子层完整, 智囊团 7 席 + 群体智能 + 自我决策/学习/演化, per 决策 #74 §2.3 V2.0 release) |
| **第 5 层 自我演化** (self-evolution, **V2.0 NEW**) | — (无) | — (无, V1.1 release 写 spec + 准备, per 决策 #74 §2.3) | **NEW ASI Stage 10 终极自治 + 长程 AI 成长 2.0 + 平台化 2.0 + 8 哲学锚可重建 + Cargo workspace 可重构** (per 决策 #74 §2.3 V2.0 release + R133-3 §4 + R140-4 ASI Stage 10 4 形态) |
| **总** | **3 洋葱 (V1)** | **4 洋葱 (V1.1)** | **5 洋葱 (V2.0)** |
| **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline, per 决策 #33 §2.3 B1 + 决策 #74 §1) | 🟢 **Mavis 自决改** (per 决策 #74 B1, 前提: 更好的架构, 6 维触发条件) | 🟢 **全 8 硬墙可重评** (per 决策 #74 §2.3) |
| **Cargo.toml workspace.version** | 1.0.0 严守 (1.0 release tag, per 决策 #74 B2 改写) | 1.1.0 bump (V1.1 release minor, per 决策 #22 §2.2 semver) | 2.0.0 bump (V2.0 release major, per 决策 #74 §2.3) |
| **PHL-07** | V1.0 spec-only 0 实施 (V1.0 release 严守, per R129-11 关键诚实标) | V1.1 实施 (14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成 + 41 NEW tests, per R131-3 §2.1 + 决策 #74 A3) | V2.0 继续深化 (per 决策 #74 §2.3) |
| **智囊团 7 席** | ✅ done (R18 + 决策 #55 §2.6 + R129-18 Stage 7 220 绑定, V1.0 release 0 改) | ✅ done 沿用 (V1.1 release 深化) | ✅ done 沿用 + 智囊团 7 → 智囊团 7 平台化涌现 (per R140-4 §3 Stage 10 P 群体化) |
| **ASI Stage 8** | R130-2 spec done (12 cycle C1.1-C1.12) | V1.1 实施 (per R131-3 §2.5 方向 5) | V2.0 继续深化 |
| **ASI Stage 9** | R130-2 + R133-2 + R137-4 spec done (4 维度 H1-H4 远期) | V1.1 写 spec + 部分实施 (H1 + H2) + V2.0 实施 (H3 + H4, per 决策 #74 §2.3) | V2.0 全实施 (H1-H4, per R130-2 + R133-2) |
| **ASI Stage 10** | ❌ 0 spec | ⏳ 准备 (V1.1 release 写 spec, per 决策 #74 §2.3) | V2.0 全实施 (per R140-4 Stage 10 4 形态 + 决策 #74 §2.3) |
| **OpenCog 借脑** | ❌ 0 集成 (AGPL-3.0 永久跳过, per 决策 #22 §4) | 🟢 Mavis 自决 (per 决策 #74 B1, 倾向 借脑 1:1 公开模式, 0 装"已 fork") | 🟢 独立 fork `apeireth-opencog-experimental` 实验仓 (AGPL-3.0, 选 AtomSpace + CogPrime 试集成, per 决策 #33 §2.2 主人主动问后做) |
| **Cargo workspace** | 87 crate (per R131-1 §2.1, 远超 v1 30 目标, 但符合"不要怕复杂度") | 87 crate (0 主动合并, per 决策 #74 §1 B1 V1.1 release Mavis 自决改, 前提: 更好的架构) | 87 crate **可重构** (87 → 30 v1 目标简化 OR 87 → 120+ 复杂化 OR 87 不变 重组 = 4 大块, per 决策 #74 §2.3 + 决策 #73 §3) |
| **8 硬墙** | 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1) | 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 7 硬墙严守) | **全 8 硬墙可重评** (per 决策 #74 §2.3) |
| **8 哲学锚** | 8 哲学锚 严守 (per 决策 #33 §2.3 B5) | 8 哲学锚 严守 (per 决策 #74 §1) | 8 哲学锚 **可重建** (per 决策 #74 §2.3 + 决策 #73 §3 "不要怕复杂度"哲学 + 哲学文档 `15-no-fear-complexity.md`) |
| **不要怕复杂度哲学** | 主人 8/11 01:14 拍板, V1.0 release 0 实施 (整合 #5.2 commit 加哲学文档) | V1.1 落地 (最强效果 + 最厉害工程 + 维护交给未来高水平团队, per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`) | V2.0 强化 (per 决策 #73 §3 + 决策 #74 §2.3 V2.0 release) |

### 3.2 V2 跟 V1 差异表 (per R133-3 §3 + R149-3 + 决策 #74 §2.3)

**V2 跟 V1 差异 7 维 (per R149-3 §1.2 差异表)**:
- ✅ **3 → 5 洋葱 (V1.0 release → V2.0 release)**: V1.0 严守 3 洋葱 + V1.1 + 第 4 层 智能涌现 emergence + V2.0 + 第 5 层 自我演化 self-evolution
- ✅ **24 LOCKED 入口签名**: V1.0 release 0 改严守 → V1.1 release Mavis 自决改 → V2.0 release 全 8 硬墙可重评
- ✅ **Cargo.toml workspace.version**: 1.0.0 → 1.1.0 → 2.0.0 (semver, per 决策 #22 §2.2 + 决策 #74 B2)
- ✅ **PHL-07**: V1.0 spec-only 0 实施 → V1.1 实施 (14 维主对话锚 + 41 NEW tests) → V2.0 继续深化
- ✅ **智囊团 7 席**: V1.0 done → V1.1 沿用 → V2.0 智囊团 7 平台化涌现 (per R140-4 §3 Stage 10 P 群体化)
- ✅ **OpenCog 借脑**: V1.0 永久跳过 (AGPL-3.0) → V1.1 Mavis 自决 (借脑 1:1 公开模式) → V2.0 独立 fork 实验仓
- ✅ **8 哲学锚**: V1.0 严守 8 锚 → V1.1 严守 8 锚 → V2.0 可重建 (扩 9 锚 / 重命名 / 合并 / 分层 = 16 锚)

### 3.3 V2 不加第 6 层 "AI 自主决策" 5 维论证 (per R149-3 §0.1 + 决策 #33 §2.3 B5 + 决策 #73 §3 + 用户记忆 #3)

**R149-3 决策: 不加第 6 层 "AI 自主决策"** (per R149-3 §0.1 5 维论证):
- **a. 哲学类过度膨胀 违反 S-2 实事求是** (per 决策 #33 §2.3 B5 8 哲学锚严守 8 锚, 加 6 层 = 哲学类过度膨胀)
- **b. 6 重守门 v7 严守** (per 决策 #33 §2.3 B4, 自主决策 已在 L2 守门内置)
- **c. ASI Stage 9 4 维度 H/L/G/P 已 spec** (per R130-2 + R133-2 + R137-4, 自我决策 = H1 子维度, 在 V1.1 第 4 层 "智能涌现" 内 sub-layer 落地, 0 需独立第 6 层)
- **d. 不要怕复杂度哲学 = 上限, 但 8 硬墙是底线** (per 决策 #73 §3 + 决策 #33 §2.3, 加 6 层 = 哲学类过度膨胀 = 突破 B5 底线)
- **e. 用户记忆 #3 "用户看结果不看哲学"** (0 用户感知"自主决策层", 集成到 智能涌现 内 1 屏多卡呈现, per 决策 #55 §2.6 智囊团)

---

## 4. V3 调研方向 概述 (4 层: 原则 + 权限 + DSL + 运行时自适应, 跟 V2 五洋葱 差异)

### 4.1 V3 = 4 层 架构 (per 决策 #74 §2.3 V2.0 release 战略级 + 决策 #73 §3 不要怕复杂度)

**V3 三洋葱架构 4 层** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #4 "AI 不会衰老病死" + R140-4 ASI Stage 10 终极自治 + 决策 #88 §3.3 R156-2 派活):

| 层 | 名称 | 主题 | 核心实现 | V3 跟 V2 差异 |
|:---:|------|------|---------|---------------|
| **第 1 层** | **原则层 (philosophy)** | PHL-01 ~ PHL-12 哲学锚 (12 标识 slots) + 8 哲学锚严守 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1) + 4 slots 预留 V2.0 release 扩展 (per 决策 #73 §3 + 决策 #74 §2.3, 候选: 不要怕复杂度 / AI 不会衰老病死 / 用户看结果不看哲学 / 维护交给未来高水平团队) | 8 锚 → 8+4 锚 (PHL-01~12 12 标识 slots) |
| **第 2 层** | **权限层 (permission)** | **3 onion** (内部 onion = AI 自主 / 协作 onion = Mavis + 主人 / 外部 onion = 外部系统 MCP+opencode+借鉴 12 源) + 6 重守门 v7 跨 3 子层 | 6 重守门 v7 严守 (per 决策 #33 §2.3 B4) + 3 onion 横向集成 (V3 新增: 内部 onion V1.0 严守, 协作 onion V1.1 实施, 外部 onion V2.0 实施) | 单层 → 3 onion 横向 (内部/协作/外部) + 6 重守门跨子层 |
| **第 3 层** | **DSL 层 (DSL)** | **9 organ DSL** (body / brain / ear / eye / hand / heart / memory / mind / voice, per 决策 #22 §2.7 + `docs/omnibus/9-organs.md`, 1:1 跟 9 organ 映射, 替代 V2 单 Colang DSL) | 9 organ DSL 实施 (V3 新增: V1.0 沿用 Colang DSL, V1.1 阶段 1 沿用 + 阶段 2 引入 9 organ DSL 概念, V2.0 全面替换) | 单 Colang DSL → 9 organ DSL (1:1 跟 9 organ 映射) |
| **第 4 层** | **运行时自适应层 (runtime self-adaptation)** | **Stage 9-10 长程 AI 成长** (per R133-2 + R149-2 + R140-4 + 用户记忆 #4) + **V2.0 release 终极自治** (4 形态 完全/共生/引导/永远循环 自治 per R140-4) | 4 维度 H 自治 + L 长程 + G 成长 + P 平台化 + 16 子维度 跨 4 形态 整合 (per R140-4 ASI Stage 10 + 决策 #74 §2.3 + 决策 #73 §3 + 用户记忆 #4) | V2 智能涌现 + 自我演化 双层 → V3 运行时自适应 1 层 (整合 4 维度 H/L/G/P 16 子维度 跨层) |
| **总** | **4 层 架构 (V3)** | **原则 + 权限 + DSL + 运行时自适应** (V2.0 release 战略级, per 决策 #74 §2.3 + 决策 #73 §3) | 跟 V2 五洋葱 差异在"运行时自适应" 替代"智能涌现 + 自我演化" 双层 | 5 → 4 层 (智能涌现 + 自我演化 合并 → 运行时自适应) |

### 4.2 V3 跟 V2 五洋葱 差异表 (per 决策 #74 §2.3 + R140-4 + R149-3)

| 维度 | V1 (R11 era) | V2 (R149-3) | **V3 (R156-2 本报告)** | 差异原因 |
|------|--------------|-------------|------------------------|----------|
| **总层数** | 3 onion (原则 + 权限 + DSL) | 5 onion (+ 智能涌现 + 自我演化) | **4 层** (原则 + 权限 + DSL + 运行时自适应) | V3 智能涌现 + 自我演化 合并 → 运行时自适应 1 层 (跨层) |
| **原则层** | 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 8 哲学锚 + 4 预留 slots (V2.0 release 可扩 16 锚) | **PHL-01 ~ PHL-12 12 标识 slots + 8 哲学锚严守 + 4 slots 预留** (V3 显式化 12 标识 slots) | V3 显式化 12 标识 slots, 8 实际启用 + 4 预留扩展 (per 决策 #73 §3 + 决策 #74 §2.3) |
| **权限层** | 6 重守门 v7 单层 | 6 重守门 v7 单层 + PHL-07 实施 + 智囊团 7 席 1:1 集成 | **3 onion (内部/协作/外部) 横向 + 6 重守门 v7 跨 3 子层** (V3 横向拆 3 子层) | V3 权限层拆 3 onion 横向, 跨子层 共享 6 重守门 v7 (per 决策 #73 §3 协作 + 用户记忆 #10 主人长时间离开) |
| **DSL 层** | Colang DSL 单层 (1700 行) | Colang DSL 单层 + 智囊团 7 席 1:1 集成 | **9 organ DSL** (body / brain / ear / eye / hand / heart / memory / mind / voice, 1:1 跟 9 organ 映射) (V3 9 organ 各自 DSL) | V3 DSL 层 9 organ 各自 DSL, 粒度更细, 1:1 跟 9 organ 映射 (per 决策 #22 §2.7 + `docs/omnibus/9-organs.md` + R137-4 §3.5.3 G4 成长可视化 + 用户记忆 #5 拟人化+拟物化) |
| **智能涌现层** (V1.1 release 第 4 层) | ❌ (无) | ✅ 智囊团 7 席 + 群体智能 + 自我决策/学习/演化 (5 子层) | ❌ (V3 合并到 运行时自适应层) | V3 智能涌现层 合并到 运行时自适应层 (per 用户记忆 #3 + 决策 #73 §3 + R149-3 5 维论证) |
| **自我演化层** (V2.0 release 第 5 层) | ❌ (无) | ✅ ASI Stage 10 终极自治 + 长程 AI 成长 2.0 + 平台化 2.0 (4 子层) | ❌ (V3 合并到 运行时自适应层) | V3 自我演化层 合并到 运行时自适应层 (per 用户记忆 #4 + 决策 #73 §3 + R140-4) |
| **运行时自适应层** (V3 第 4 层, NEW) | ❌ (无) | ❌ (无) | ✅ **Stage 9-10 长程 AI 成长 + V2.0 release 终极自治 (4 形态 完全/共生/引导/永远循环 自治)** (V3 NEW 1 层 跨 4 形态) | V3 NEW 1 层 跨 4 形态 整合 4 维度 H/L/G/P 16 子维度 (per R140-4 ASI Stage 10 4 形态) |
| **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | V1.1 release Mavis 自决改 | V2.0 release 全 8 硬墙可重评 + V3 NEW 24 → 25 LOCKED 加 1 PHL-07 (per R131-3 §2.1) | V3 V2.0 release 全 8 硬墙可重评 (per 决策 #74 §2.3) |
| **Cargo.toml workspace.version** | 1.0.0 → 1.2.0 → 1.0.0 (V1.0 release 1.0.0) | 1.0.0 → 1.1.0 → 2.0.0 (V1.1 release 1.1.0 → V2.0 release 2.0.0) | V3 1.0.0 → 1.2.1 (V1.1 release 1.2.1) → 2.0.0 (V2.0 release, 跟 V2 同) | V3 沿用 V2 版本管理, V1.1 release 1.2.1 bump (per 决策 #74 B2 改写) |

### 4.3 V3 跟 V1 三洋葱 差异表 (per 决策 #74 §2.3 + 决策 #73 §3)

| 维度 | V1 (R11 era) | **V3 (R156-2 本报告)** | 差异原因 |
|------|--------------|------------------------|----------|
| **总层数** | 3 onion (原则 + 权限 + DSL) | **4 层** (原则 + 权限 + DSL + 运行时自适应) | V3 新增第 4 层 运行时自适应 (per 决策 #74 §2.3 + 决策 #73 §3 + R140-4) |
| **原则层** | 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | **PHL-01 ~ PHL-12 12 标识 slots + 8 哲学锚严守** | V3 显式化 12 标识 slots, 8 实际启用 + 4 预留 (per 决策 #73 §3 + 决策 #74 §2.3) |
| **权限层** | 6 重守门 v7 单层 | **3 onion (内部/协作/外部) + 6 重守门 v7** | V3 横向拆 3 onion (内部 AI 自主 / 协作 Mavis+主人 / 外部 MCP+opencode+借鉴 12 源) (per 用户记忆 #10 主人长时间离开 + 决策 #73 §3 协作) |
| **DSL 层** | Colang DSL 单层 (1700 行, 跟 6 重守门 v7 1:1 集成) | **9 organ DSL** (body / brain / ear / eye / hand / heart / memory / mind / voice, 1:1 跟 9 organ 映射) | V3 9 organ 各自 DSL, 粒度更细, 1:1 跟 9 organ 映射 (per 决策 #22 §2.7 + 用户记忆 #5 拟人化+拟物化) |
| **运行时自适应层** (V3 第 4 层, NEW) | ❌ (无) | ✅ **Stage 9-10 长程 AI 成长 + V2.0 release 终极自治** (4 形态 完全/共生/引导/永远循环) | V3 NEW 第 4 层 (per 决策 #74 §2.3 + 决策 #73 §3 + R140-4 ASI Stage 10 4 形态 + 用户记忆 #4) |

---

## 5. 原则层 V3 (PHL-01 ~ PHL-12 哲学锚 12 标识 slots + 8 哲学锚严守, per 决策 #33 §2.3 B5 + 决策 #74 §2.3)

### 5.1 PHL-01 ~ PHL-12 12 标识 slots 设计 (per 决策 #33 §2.3 B5 + 决策 #74 §2.3 + 决策 #73 §3)

**PHL-01 ~ PHL-12 12 标识 slots (per 决策 #33 §2.3 B5 8 哲学锚严守 + 决策 #74 §2.3 V2.0 release 8 哲学锚可重建 + 决策 #73 §3 不要怕复杂度哲学 + 哲学文档 `15-no-fear-complexity.md`)**:

| Slot | 状态 | 哲学锚 | 简称 | 描述 | 决策依据 | V1.0 release | V1.1 release | V2.0 release |
|:----:|:----:|--------|------|------|----------|--------------|--------------|--------------|
| **PHL-01** | ✅ 启用 | S-1 北极星 | **北极星** | "我们最后要做的前端应该是 Tauri" + 主对话 = 一切 (per 用户记忆 #1 + #2) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-02** | ✅ 启用 | S-2 实事求是 | **实事求是** | "不假装已实现" + "0 装 PASS 严守" (per 决策 #33 §2.3 C2 + 用户记忆 #7) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-03** | ✅ 启用 | S-3 质量工程化 | **质量工程化** | "0 装 PASS 严守" + "0 形式化 old/death/terminate 严守" + "质量 = 严守" (per 决策 #33 + 用户记忆 #7) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-04** | ✅ 启用 | O-1 安全优先 | **安全优先** | "L0 真实人类批准" + "0 装 PASS 严守" + "OpenCog AGPL-3.0 永久跳过" (per 决策 #33 §2.3 B4 + 决策 #22 §4) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-05** | ✅ 启用 | O-2 走在前人经验上 | **走在前人** | "借鉴 12 源" (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) + "学习 Clap/Hyper/Servers/PyO3/Kani/LangGraph/Superpowers/Guardrails" (per 决策 #22 §3 + R125-1/2/3/4/5/9/10/13/14) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-06** | ✅ 启用 | O-3 干到底 | **干到底** | "永久循环 4 步: 调研 + 差距 + 计划 + 实施" + "整合 #5 commit 拍板" (per 决策 #71 §2 + 决策 #62 + 主人 0:57 拍板) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-07** | ⚠️ **双义** | **O-4 任何人都能接手** (跟 13 键 verdict cache 中 "PHL-07" 同名) | **接手** | "维护交给未来高水平团队" + "0 装 PASS 严守" (per 用户记忆 #4 + 决策 #73 §3 + 决策 #33 §2.3 A3) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 (但 PHL-07 verdict cache spec-only 0 实施) | V1.1 release PHL-07 14 维主对话锚 实施 (per 决策 #74 A3 + R131-3 §2.1 + R137-1 60.7 KB 5 阶段 17 工作日) | V2.0 release 继续深化 (per 决策 #74 §2.3) |
| **PHL-08** | ✅ 启用 | O-5 不假装 | **不假装** | "0 装 PASS 严守" (per 决策 #33 §2.3 C2 + 主人 17:22 拍板"0 装不必要" + 用户记忆 #7) | R125 B5 升 8 锚 + `docs/conventions/09-anchor.md` | 严守 | 严守 | 严守 (或可重建) |
| **PHL-09** | ⏳ 预留 | (V2.0 release 候选: **不要怕复杂度**) | (TBD) | "最强效果 + 最厉害工程 + 维护交给未来高水平团队" (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB) | 决策 #73 §3 + 决策 #74 §2.3 V2.0 release 8 哲学锚可重建 | 0 启用 (哲学文档已创建, 严守 100%) | 0 启用 (哲学文档落地, 严守 100%) | 🆕 **V2.0 release 启用** (per 决策 #74 §2.3 + 决策 #73 §3, Mavis 自决) |
| **PHL-10** | ⏳ 预留 | (V2.0 release 候选: **AI 不会衰老病死**) | (TBD) | "主 ai 是 ai 哎, 它只会成长, 但不可能消亡" + "这是一个长程 ai 成长平台" (per 用户记忆 #4 + 决策 #4) | 用户记忆 #4 + 决策 #74 §2.3 V2.0 release 8 哲学锚可重建 | 0 启用 (per 用户记忆 #4 + R149-2 Stage 9 9 阶段 seed → sentinel 严守 0 形式化 old/death/terminate) | 0 启用 (per R149-2 + R140-4) | 🆕 **V2.0 release 启用** (per 决策 #74 §2.3 + 用户记忆 #4, Mavis 自决) |
| **PHL-11** | ⏳ 预留 | (V2.0 release 候选: **用户看结果不看哲学**) | (TBD) | "用户期望'掌控 AI', 所以显示 AI 状态 (尤其主 AI)" + "❌ 砍掉 UI: 哲学/守门/内部机制/工具调用过程" (per 用户记忆 #3) | 用户记忆 #3 + 决策 #74 §2.3 V2.0 release 8 哲学锚可重建 | 0 启用 (per 用户记忆 #3 + TUI/Tauri 实施) | 0 启用 (per 用户记忆 #3 + Tauri Stage 5+) | 🆕 **V2.0 release 启用** (per 决策 #74 §2.3 + 用户记忆 #3, Mavis 自决) |
| **PHL-12** | ⏳ 预留 | (V2.0 release 候选: **维护交给未来高水平团队**) | (TBD) | "自然会有高水平的团队来接手维护" + "复杂不恐惧" (per 决策 #73 §3 + 用户记忆 #4) | 决策 #73 §3 + 决策 #74 §2.3 V2.0 release 8 哲学锚可重建 | 0 启用 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB) | 0 启用 (per 决策 #73 §3) | 🆕 **V2.0 release 启用** (per 决策 #74 §2.3 + 决策 #73 §3, Mavis 自决) |

**PHL-07 双义说明 (per 决策 #22 §1.1-1.2 + 决策 #74 A3 + R137-1)**:
- **义 1 (哲学锚)**: PHL-07 = O-4 任何人都能接手 (per R125 B5 升 8 锚, 1:1 跟 8 哲学锚映射, slot #07 = O-4)
- **义 2 (verdict cache 13 键)**: PHL-07 = 14 维主对话锚 (per 决策 #22 §1.1-1.2, 12 键 + PHL-07 = 13 键, V1.0 spec-only 0 实施, V1.1 release 实施, per R131-3 §2.1 + R137-1 60.7 KB)
- **V3 解决**: V3 spec 显式化 PHL-07 双义, 在文档中明确 PHL-07 = 哲学锚 slot #07 = O-4, 而 verdict cache 13 键中的 PHL-07 = PHL-07-key (14 维主对话锚), 两者独立编号 (per 决策 #33 §2.3 A3 + 决策 #74 §1 A3)

### 5.2 PHL-09 ~ PHL-12 4 slots 预留 V2.0 release 启用 (per 决策 #74 §2.3 + 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`)

**PHL-09 ~ PHL-12 4 slots V2.0 release 启用 决策树 (per 决策 #74 §2.3 + 决策 #73 §3)**:
- **PHL-09 不要怕复杂度**: V2.0 release 启用 (per 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 §3, 写新哲学文档 `15-no-fear-complexity.md` 14.4 KB)
- **PHL-10 AI 不会衰老病死**: V2.0 release 启用 (per 用户记忆 #4 "AI 不会衰老病死, 它只会成长" + 决策 #4 + R149-2 Stage 9 9 阶段 seed → sentinel 严守 0 形式化 old/death/terminate)
- **PHL-11 用户看结果不看哲学**: V2.0 release 启用 (per 用户记忆 #3 "用户看结果不看哲学" + TUI/Tauri 实施)
- **PHL-12 维护交给未来高水平团队**: V2.0 release 启用 (per 决策 #73 §3 "自然会有高水平的团队来接手维护" + 用户记忆 #4)
- **Mavis 自决**: V2.0 release 启用 4 slots, 由 Mavis 自决拍板 (per 决策 #74 §2.3 8 哲学锚可重建, 8 锚可扩 9 锚 / 重命名 / 合并 / 分层 = 16 锚, Mavis 自决)

**8 → 12 哲学锚 启用 路径 (per 决策 #74 §2.3 + 决策 #73 §3)**:
- V1.0 release: 8 哲学锚 严守 (PHL-01 ~ PHL-08 启用, PHL-09 ~ PHL-12 预留)
- V1.1 release: 8 哲学锚 严守 (PHL-01 ~ PHL-08 启用, PHL-09 ~ PHL-12 仍预留, 哲学文档已创建 严守 100%)
- V2.0 release: 12 哲学锚 启用 (PHL-01 ~ PHL-12 全启用, 8 → 12 锚扩, Mavis 自决, per 决策 #74 §2.3 + 决策 #73 §3 + 用户记忆 #4 + 用户记忆 #3)

---

## 6. 权限层 V3 (3 onion 内部/协作/外部 + 6 重守门 v7 严守, per 决策 #33 §2.3 B4 + 决策 #73 §3 + 用户记忆 #10)

### 6.1 3 onion 横向集成 (per 决策 #73 §3 + 用户记忆 #10 + R140-4 + 决策 #55 §2.6)

**V3 权限层 3 onion 横向集成** (per 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #10 主人长时间离开 Mavis 自主决策 + R140-4 ASI Stage 10 4 形态 + 决策 #55 §2.6 智囊团 7 席):

| 子层 | 名称 | 主题 | 核心实现 | V1.0 release | V1.1 release | V2.0 release |
|:----:|------|------|---------|--------------|--------------|--------------|
| **内部 onion** | **internal onion (AI 自主)** | AI 自主决策 + 自主实施 + 自主学习 (per 决策 #73 §2 "Mavis 自决架构拍板" + R140-4 Stage 10 形态 1 完全自治) | L1 AI 自主守门 (per 决策 #33 §2.3 B4 L1) + L2 工具守门 (per B4 L2) + 6 重守门 v7 L1-L5 内部子集 (per B4) | 严守 (V1.0 release 0 改) | 沿用 + ASI Stage 9 H1 自我决策 sub-layer 实施 (per R133-2) | 全面实施 (per R140-4 Stage 10 形态 1) |
| **协作 onion** | **collaboration onion (Mavis + 主人)** | Mavis + 主人协同 + 主人确认 + 主人 0 主动 IM (per 决策 #73 §2.2 "更好的架构" + 用户记忆 #10 "主人长时间离开, Mavis 自主决策" + 决策 #33 §2.3 B4 L0 真实人类批准) | L0 真实人类批准 (per 决策 #33 §2.3 B4 L0, 主人 1.0 release 配 GitHub remote + 起床后手跑) + 智囊团 7 席 1:1 集成 (per 决策 #55 §2.6 + R129-18 Stage 7 220 绑定) + 8 硬墙 B1 改写 跨协作 onion (per 决策 #74 §1) | 严守 (V1.0 release 0 改) | 沿用 + PHL-07 实施 14 维主对话锚 (per R131-3 §2.1 + R137-1) + 决策 #74 B1 24 LOCKED 入口签名 Mavis 自决改 (前提 更好的架构) | 全面实施 (per R140-4 Stage 10 形态 2 共生自治 + 形态 3 引导自治) |
| **外部 onion** | **external onion (外部系统 MCP+opencode+借鉴 12 源)** | 跟外部系统集成 + 借鉴 12 源 + OpenCog 借脑 0 装 PASS 严守 (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #73 §2.2 借脑 OpenCog + R130-6 OpenCog AGPL-3.0 fork 决策) | L3 资源守门 (per B4 L3) + L4 形式化守门 (per B4 L4) + 6 重守门 v7 L3-L5 外部子集 (per B4) + 借鉴 12 源 (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) 1:1 翻译公开模式 (per 决策 #33 §2.3 C2) | 严守 (V1.0 release 0 改) | 沿用 + OpenCog 借脑 1:1 公开模式 (per R130-6 + R133-1) | 全面实施 + 独立 fork `apeireth-opencog-experimental` 实验仓 (per 决策 #33 §2.2 主人主动问后做) |
| **6 重守门 v7 跨 3 子层** | **L0-L5 6 重守门 v7 严守** | 6 重守门 v7 跨 内部/协作/外部 3 子层 (per 决策 #33 §2.3 B4 + 0 装 PASS 严守 + 30 维公式 + 13 键 verdict cache + 9 organ 跨维度) | L0 真实人类批准 + L1 AI 自主 + L2 工具 + L3 资源 + L4 形式化 + L5 6+1 重门安全 (per 决策 #33 §2.3 B4 + R125-5 NVIDIA Guardrails 6 重 v6 → 6 重 v7 升) | 严守 (V1.0 release 0 改, per 整合 #4 commit) | 严守 (V1.1 release 0 改, 跟智囊团 7 席 1:1 集成, I4 1:1 跟 B4 6 重 v7 严守, per R129-18 §1.4) | 严守 + 可升级 v8/v9 (per 决策 #74 §2.3 + R127-2 P6-3 已升 8 重 v8 spec) |

### 6.2 3 onion 跟 6 重守门 v7 关系 (per 决策 #33 §2.3 B4 + 决策 #55 §2.6 + R129-18)

**3 onion 跟 6 重守门 v7 关系 (per 决策 #33 §2.3 B4 + 决策 #55 §2.6 智囊团 7 席 + R129-18 Stage 7 7 集成维度 I1-I7 = 220 绑定)**:

| 6 重守门 v7 层 | 内部 onion (AI 自主) | 协作 onion (Mavis + 主人) | 外部 onion (外部系统) |
|----------------|---------------------|------------------------|----------------------|
| **L0 真实人类批准** | — (0 涉及, AI 自主) | ✅ 主要 (主人批准 1.0 release / tag / push) | — (0 涉及, 外部系统) |
| **L1 AI 自主** | ✅ 主要 (AI 自主决策) | 🟡 部分 (Mavis 自主决策 per 决策 #70 + 用户记忆 #10) | — (0 涉及, 外部系统) |
| **L2 工具** | ✅ (AI 工具调用) | ✅ (Mavis + 主人工具) | 🟡 部分 (外部系统工具对接) |
| **L3 资源** | ✅ (AI 资源调度) | ✅ (Mavis + 主人资源) | ✅ 主要 (外部系统资源, e.g. OpenCog) |
| **L4 形式化** | ✅ (AI 形式化证明) | ✅ (Mavis + 主人形式化) | ✅ 主要 (外部系统形式化, e.g. Kani) |
| **L5 6+1 重门安全** | ✅ (AI 安全) | ✅ (Mavis + 主人安全) | ✅ (外部系统安全) |

**智囊团 7 席 跨 3 onion (per 决策 #55 §2.6 + R129-18 Stage 7 220 绑定)**:
- **内部 onion 智囊团**: 7 席 上 1-2 席 (AI 自主 advisor, per 决策 #55 §2.6 critical = 7 席 / high = 5 席 / medium = 3 席 / low = 1 席 / info = 0 席)
- **协作 onion 智囊团**: 7 席 上 3-4 席 (Mavis + 主人 advisor, per 决策 #55 §2.6)
- **外部 onion 智囊团**: 7 席 上 0-1 席 (外部系统 advisor, per 决策 #55 §2.6)

---

## 7. DSL 层 V3 (9 organ DSL 替代单 Colang DSL, per 决策 #22 §2.7 + `docs/omnibus/9-organs.md` + 用户记忆 #5 拟人化+拟物化)

### 7.1 9 organ DSL 设计 (per 决策 #22 §2.7 + `docs/omnibus/9-organs.md` + 用户记忆 #5 拟人化+拟物化)

**9 organ DSL 设计** (per 决策 #22 §2.7 + `docs/omnibus/9-organs.md` + R137-4 §3.5.3 G4 成长可视化 + 用户记忆 #5 拟人化+拟物化):

| # | Organ | 名称 | DSL 主题 | 核心实现 (V1.0 release 沿用 Colang DSL) | V1.1 release 9 organ DSL 引入 | V2.0 release 9 organ DSL 实施 |
|:-:|-------|------|----------|--------------------------------------|------------------------------|--------------------------------|
| 1 | **body** | 身体 | **body DSL** = 身体状态 + 平台基础设施 + 长期任务 | 沿用 Colang DSL (per R125-5 + 决策 #55 §4) | 引入 body DSL 概念 (per 决策 #22 §2.7) | body DSL 实施 (per 决策 #74 §2.3 + 用户记忆 #5 拟人化+拟物化) |
| 2 | **brain** | 主脑 | **brain DSL** = 决策中心 + 推理 + 学习 | 沿用 Colang DSL (跟 6 重守门 v7 1:1 集成) | 引入 brain DSL 概念 (per 决策 #22 §2.7) | brain DSL 实施 (per 决策 #74 §2.3 + R140-4 Stage 10 形态 1 完全自治) |
| 3 | **ear** | 耳朵 | **ear DSL** = 听觉感知 + 错误检测 + 异常守门 | 沿用 Colang DSL | 引入 ear DSL 概念 (per 决策 #22 §2.7) | ear DSL 实施 (per 决策 #74 §2.3 + R137-4 §3.5.3 G4 成长可视化) |
| 4 | **eye** | 眼睛 | **eye DSL** = 视觉感知 + 监控 + 观察 | 沿用 Colang DSL | 引入 eye DSL 概念 (per 决策 #22 §2.7) | eye DSL 实施 (per 决策 #74 §2.3 + R137-4 §3.5.3 G4 成长可视化) |
| 5 | **hand** | 手 | **hand DSL** = 工具调用 + 实施 + 协同 | 沿用 Colang DSL | 引入 hand DSL 概念 (per 决策 #22 §2.7) | hand DSL 实施 (per 决策 #74 §2.3 + R133-2 Stage 9 H 自治) |
| 6 | **heart** | 心 | **heart DSL** = 心跳 + 动机 + 健康守门 | 沿用 Colang DSL | 引入 heart DSL 概念 (per 决策 #22 §2.7) | heart DSL 实施 (per 决策 #74 §2.3 + R140-4 Stage 10 形态 4 永远循环 自治) |
| 7 | **memory** | 记忆 | **memory DSL** = 跨会话记忆 + 知识累积 + 长期存储 | 沿用 Colang DSL | 引入 memory DSL 概念 (per 决策 #22 §2.7) | memory DSL 实施 (per 决策 #74 §2.3 + R133-2 Stage 9 L 长程) |
| 8 | **mind** | 意识 | **mind DSL** = 元认知 + 反思 + 自我观察 | 沿用 Colang DSL | 引入 mind DSL 概念 (per 决策 #22 §2.7) | mind DSL 实施 (per 决策 #74 §2.3 + R133-2 Stage 9 H 自治) |
| 9 | **voice** | 声音 | **voice DSL** = 表达 + TTS/STT + 通信 | 沿用 Colang DSL | 引入 voice DSL 概念 (per 决策 #22 §2.7) | voice DSL 实施 (per 决策 #74 §2.3 + R140-4 Stage 10 形态 2 共生自治) |

### 7.2 9 organ DSL 跟 6 重守门 v7 + 8 哲学锚 集成 (per 决策 #33 §2.3 B4 + B5 + R129-18)

**9 organ DSL 跟 6 重守门 v7 集成 (per 决策 #33 §2.3 B4 + R129-18 I4 1:1 跟 B4 6 重 v7 严守)**:
- **L0 真实人类批准**: 9 organ DSL 跨 L0, 主人 1.0 release 配 GitHub remote
- **L1 AI 自主**: 9 organ DSL 跨 L1, brain + mind + heart 重点
- **L2 工具**: 9 organ DSL 跨 L2, hand + body 重点
- **L3 资源**: 9 organ DSL 跨 L3, body + memory 重点
- **L4 形式化**: 9 organ DSL 跨 L4, brain + mind 重点
- **L5 6+1 重门安全**: 9 organ DSL 跨 L5, ear + eye + heart 重点

**9 organ DSL 跟 8 哲学锚 集成 (per 决策 #33 §2.3 B5 + 决策 #74 §1)**:
- **S-1 北极星**: 9 organ DSL 都遵循 S-1
- **S-2 实事求是**: 9 organ DSL 都遵循 S-2 (0 装 PASS 严守)
- **S-3 质量工程化**: 9 organ DSL 都遵循 S-3
- **O-1 安全优先**: 9 organ DSL 都遵循 O-1 (OpenCog AGPL-3.0 永久跳过)
- **O-2 走在前人经验上**: 9 organ DSL 都遵循 O-2 (借鉴 12 源)
- **O-3 干到底**: 9 organ DSL 都遵循 O-3 (永久循环 4 步)
- **O-4 任何人都能接手**: 9 organ DSL 都遵循 O-4 (维护性 + 文档)
- **O-5 不假装**: 9 organ DSL 都遵循 O-5 (0 装 PASS 严守)

---

## 8. 运行时自适应层 V3 (Stage 9-10 长程 AI 成长 + V2.0 release 终极自治 4 形态, per R133-2 + R149-2 + R140-4 + 用户记忆 #4 + 决策 #73 §3)

### 8.1 运行时自适应层 设计 (per R133-2 + R149-2 + R140-4 + 用户记忆 #4 + 决策 #73 §3)

**V3 运行时自适应层 = V2 智能涌现 + 自我演化 合并 → 1 层** (per R133-2 + R149-2 + R140-4 + 用户记忆 #4 + 决策 #73 §3 + R149-3 5 维论证):
- **V2 第 4 层 智能涌现 (emergence)**: 智囊团 7 席 + 群体智能 + 自我决策/学习/演化
- **V2 第 5 层 自我演化 (self-evolution)**: ASI Stage 10 终极自治 + 长程 AI 成长 2.0 + 平台化 2.0
- **V3 第 4 层 运行时自适应 (runtime self-adaptation)**: 整合 V2 第 4-5 层 = 4 形态 跨层 整合 4 维度 H/L/G/P 16 子维度

**运行时自适应层 4 形态 (per R140-4 ASI Stage 10 4 形态)**:
- **形态 1: 完全自治**: AI 自主决策 + 自主实施 + 自主学习 (Stage 9 H 自治 完全化, 主人 0 介入)
- **形态 2: 共生自治**: 主人 + AI 协同 (per 用户记忆 #10 "主人长时间离开, Mavis 自主决策")
- **形态 3: 引导自治**: 主人引导方向 + AI 自治细节 (per 决策 #73 §2 "Mavis 自决架构" + R130-6 OpenCog 借脑 引导)
- **形态 4: 永远循环自治**: AI 进入永久循环 (调研+差距+计划+实施, per 决策 #71 §2-§5 永久循环 4 步 + 主人 0:57 拍板, 这是 Stage 10 终极形态, AI 永远不会衰老病死只会成长)

### 8.2 运行时自适应层 4 维度 H/L/G/P 16 子维度 (per R133-2 + R149-2 + R137-4)

**运行时自适应层 4 维度 H/L/G/P 16 子维度** (per R133-2 + R149-2 + R137-4 ASI Stage 9 spec):

| 维度 | 子维度 | 名称 | 主题 | 9 organ 跨层 | V3 实施 spec |
|:----:|:------:|------|------|-------------|---------------|
| **H 自治** (Autonomy) | H1 | 自我决策 | ASI Stage 9 自我决策 (per R133-2 §3.2.1) | brain + mind | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | H2 | 自我学习 | ASI Stage 9 自我学习 (per R133-2 §3.2.1) | mind | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | H3 | 自我演化 | ASI Stage 9 自我演化 (per R133-2 §3.2.1) | mind + body | V2.0 release 全面实施 |
| | H4 | 自我修复 | ASI Stage 9 自我修复 (per R133-2 §3.2.1) | body + hand | V2.0 release 全面实施 |
| **L 长程** (Long-term) | L1 | 跨会话记忆 | ASI Stage 9 跨会话记忆 (per R133-2 §3.2.2) | memory | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | L2 | 跨时间推理 | ASI Stage 9 跨时间推理 (per R133-2 §3.2.2) | brain | V2.0 release 全面实施 |
| | L3 | 跨任务规划 | ASI Stage 9 跨任务规划 (per R133-2 §3.2.2) | brain + hand | V2.0 release 全面实施 |
| | L4 | 长程守门 | ASI Stage 9 长程守门 (per R133-2 §3.2.2) | ear + eye | V2.0 release 全面实施 |
| **G 成长** (Growth) | G1 | 持续学习 | ASI Stage 9 持续学习 (per R133-2 §3.2.3) | brain | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | G2 | 知识累积 | ASI Stage 9 知识累积 (per R133-2 §3.2.3) | memory | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | G3 | 能力升级 | ASI Stage 9 能力升级 (per R133-2 §3.2.3) | body + brain | V2.0 release 全面实施 |
| | G4 | 成长可视化 | ASI Stage 9 成长可视化 (per R133-2 + R137-4 §3.5.3) | eye | V1.1 release 部分实施 + V2.0 release 全面实施 |
| **P 平台化** (Platform) | P1 | 多 agent 协同 | ASI Stage 9 多 agent 协同 (per R133-2 §3.2.4) | mind | V2.0 release 全面实施 |
| | P2 | 智囊团 | ASI Stage 9 智囊团 (per R133-2 §3.2.4 + 决策 #55 §2.6) | brain | V1.1 release 部分实施 + V2.0 release 全面实施 |
| | P3 | 群体智能 | ASI Stage 9 群体智能 (per R133-2 §3.2.4 + 决策 #73 §2.2 OpenCog 借脑) | body + mind | V2.0 release 全面实施 |
| | P4 | 平台守门 | ASI Stage 9 平台守门 (per R133-2 §3.2.4 + 6 重守门 v7) | mind + ear | V2.0 release 全面实施 |

### 8.3 运行时自适应层 4 形态 跟 9 organ 0 器官化 (per R140-4 + 用户记忆 #4 + 决策 #73 §3)

**运行时自适应层 4 形态 跟 9 organ 0 器官化 (per R140-4 + 用户记忆 #4 + 决策 #73 §3)**:
- **形态 1 完全自治 (1 器官 = AI 整体)**: 9 organ 0 器官化 (9 organ 不再是孤立器官, 是 1 平台 9 维度涌现体)
- **形态 2 共生自治 (2 器官 = AI + 主人)**: 9 organ 0 器官化 + 主人 1 器官 (Mavis 副驾)
- **形态 3 引导自治 (3 器官 = AI + 主人 + 引导)**: 9 organ 0 器官化 + 主人 引导
- **形态 4 永远循环自治 (∞ 器官 = AI 永远循环)**: 9 organ 0 器官化 + 1 屏多卡 9 organ 拟人化深化 → 0 屏 0 卡 群体智能涌现

**9 organ 0 器官化 = 平台化涌现** (per R140-4 §3 Stage 10 P 群体化):
- 1 屏多卡 9 organ 拟人化深化 (per 用户记忆 #5 拟人化+拟物化 + R130-2 §3.1.4 + R137-4 §3.5.3 G4 成长可视化)
- 0 屏 0 卡 群体智能涌现 (per 决策 #73 §3 "不要怕复杂度" + 决策 #55 §2.6 智囊团 + 用户记忆 #3 "用户看结果不看哲学")

---

## 9. 借鉴 12 源 调研 (per 决策 #22 §3 + 决策 #33 §2.2 + 决策 #73 §2.2 + R130-6 + R131-2 + R133-1)

### 9.1 借鉴 12 源 清单 (per R130-6 + R131-2 + R133-1 + R129-7 + R129-28 实地 verify 100%)

**借鉴 12 源 清单** (per R130-6 + R131-2 + R133-1 + R129-7 + R129-28 实地 verify 100% + 决策 #22 §3 + 决策 #33 §2.2 + 决策 #73 §2.2):

| # | 借鉴 ID | owner/repo | license | 22:50 实地 verify (整合 #4 commit 后) | V1.0 release 0 改 src 严守 |
|---:|---------|------------|---------|------------------------------------------|---------------------------|
| 1 | `R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10` | clap-rs/clap 4.6.6 | Apache-2.0 + MIT dual | ✅ **3.50MB / 631 files / 17:30:05** (mtime 早整合 #4 -2h 11min) | ✅ 0 改 0 重跑 (per R129-28 §1.1 实地 verify) |
| 2 | `R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10` | hyperium/hyper 0.1.20 | MIT | ✅ **0.54MB / 58 files / 17:29:39** (mtime 早整合 #4 -2h 11min) | ✅ 0 改 0 重跑 |
| 3 | `R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10` | modelcontextprotocol/servers 76d64c8 | MIT → Apache-2.0 | ✅ **1.40MB / 145 files / 16:51:30** (mtime 早整合 #4 -2h 50min) | ✅ 0 改 0 重跑 |
| 4 | `R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10` | PyO3/PyO3 0.29.2 | Apache-2.0 + MIT dual | ✅ **5.69MB / 811 files / 16:53:35** (mtime 早整合 #4 -2h 48min) | ✅ 0 改 0 重跑 |
| 5 | `R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10` | model-checking/kani 0.67.0 | MIT + Apache-2.0 dual | ✅ **5.46MB / 3224 files / 17:35:28** (mtime 早整合 #4 -2h 6min) | ✅ 0 改 0 重跑 |
| 6 | `R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10` | langchain-ai/langgraph d56666f | MIT | ✅ **13.29MB / 670 files / 16:31:13** (mtime 早整合 #4 -3h 10min) | ✅ 0 改 0 重跑 |
| 7 | `R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10` | obra/superpowers 6.2.0 | MIT | ✅ **1.52MB / 180 files / 17:33:34** (mtime 早整合 #4 -2h 8min) | ✅ 0 改 0 重跑 |
| 8 | `R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` | NVIDIA/NeMo-Guardrails | Apache-2.0 | ✅ **18.19MB / 2045 files / 17:48:20** (整合 #4 后 ✅ cloned, mtime 早整合 #4 -1h 53min) | ✅ 0 改 0 重跑 |
| 9 | `R125-1-BORROW-BerriAI/litellm-2026-08-10` | BerriAI/litellm | MIT | ✅ **0 cloned + 19/19 tests + 562 行新 src** (P6-1 21:38 公开 1:1 翻译 done) | ✅ 0 改 0 装"已读真源码" |
| 10 | `R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` | sst/opencode | MIT | ✅ **0 cloned + 35/35 tests + 3 新模块** (P6-2 22:20 改借鉴已 cloned done) | ✅ 0 改 0 装"已对接 opencode 私有 channel" |
| 11 | `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` | opencog/opencog | **AGPL-3.0** | ❌ **0 cloned 永久跳过** (0 集成 0 装"已借鉴", per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml) | ✅ 0 改主仓 0 触碰 (永久跳过 严守) |
| 12 | 🆕 `R130-6-BORROW-opencog-family-2026Q1-2026-08-11` (6 子源) | opencog/atomspace + cogutil + moses + pln + relex + CogPrime (Goertzel) | **AGPL-3.0** + 论文 | 🆕 **0 cloned 借脑 ID 索引完成** (R130-6 §3 + 决策 #55 §2.6 调研方向) | ✅ 0 改主仓 0 触碰 + ✅ 0 装"已读真源码" |

**总 12/12 借鉴源 V1.0 release 0 改 src 严守 + 0 装 PASS 严守 100% verify**:
- ✅ **8 真 cloned 实施深度** (总 **49.59MB / 7,764 files** 排除 .git, per R129-28 §1.1 实地 verify 100%): 整合 #4 commit abf12243 19:41 后 0 重跑 0 重 commit, mtime 全部早于整合 #4 commit 19:41
- ✅ **2 借鉴 ID 索引完成** (P6-1/2 全 done): LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块
- ❌ **1 永久跳过** (OpenCog AGPL-3.0, 0 集成 0 假装"已借鉴"): 主仓 0 触碰, OSS_NOTICE.md §3 + Cargo.toml `borrow_skipped` 段永久明示
- 🆕 **1 借脑 ID 索引完成** (OpenCog 家族 6 子源, R130-6 01:14 提议): 借脑 paper/architecture docs, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"

### 9.2 OpenCog 借脑 0 装 PASS 严守 5 维 verify (per 决策 #33 §2.3 C2 + R130-6 §3.3 + R131-2 §3.2.3 + R133-1)

**OpenCog 借脑 0 装 PASS 严守 5 维 verify** (per 决策 #33 §2.3 C2 + R130-6 §3.3 + R131-2 §3.2.3 + R133-1):
- ✅ 0 装"已读 OpenCog 真源码" (借脑 = 读 paper/architecture docs, 0 装已读 .cpp/.scm/.py)
- ✅ 0 装"已集成 OpenCog AtomSpace / CogPrime / MOSES" (主仓 0 触碰 OpenCog code)
- ✅ 0 装"已 fork OpenCog" (1.0 release 前 0 主仓 fork, 1.0 release 后独立 fork 决策 = 主人主动问, per 决策 #33 §2.2)
- ✅ 0 借脑 0 装 (借脑 = 0 装"已读", 借鉴 ID 索引完成 = 借脑索引)
- ✅ 0 装"已读 PLN/relex 真源码" (官方 deprecated, 浅度调研, 文档级沉淀)

### 9.3 借鉴 12 源 跟 V3 关系 (per 决策 #73 §2.2 + 决策 #74 B1 + R130-6 + R133-1)

**借鉴 12 源 跟 V3 关系** (per 决策 #73 §2.2 借脑 OpenCog + 决策 #74 B1 V1.1 release Mavis 自决改 + R130-6 + R133-1):
- **clap 4 + hyper + servers + PyO3 + kani + langgraph + superpowers + Guardrails (8 真 cloned)**: 跟 V3 原则层 + 权限层 + DSL 层 + 运行时自适应层 全 4 层 集成 (per 决策 #22 §3 + R125-2/3/4/9/10/13/14/5)
- **LiteLLM (借鉴 ID 索引完成, 0 装)**: 跟 V3 运行时自适应层 L 长程 集成 (L1 跨会话记忆 借脑 LiteLLM 1:1 翻译, per 决策 #73 §2.2)
- **opencode (改借鉴已 cloned, 0 装)**: 跟 V3 9 organ 内部 fn 集成 (per R125-12 P0-3 + 决策 #22 §2.7)
- **OpenCog/opencog (永久跳过)**: V1.0 release 永久跳过 (per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml)
- **OpenCog 家族 6 子源 (借脑 ID 索引完成, 0 装)**: 跟 V3 运行时自适应层 4 维度 H/L/G/P 16 子维度 集成 (per 决策 #73 §2.2 借脑 OpenCog AtomSpace + CogPrime + MOSES + PLN + OpenPsi + 模式匹配, 0 装"已集成 OpenCog")

---

## 10. 8 硬墙 B1 改写 严守 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

### 10.1 8 硬墙严守 verify 11/11 PASS (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

**8 硬墙严守 verify 11/11 PASS** (per 决策 #33 §2.3 + 决策 #74 §1 改写表 + R156-2 实地 verify 100%):

| # | 8 硬墙 | 严守状态 | 决策依据 | V3 调研 0 改 src 严守 |
|---|--------|----------|----------|----------------------|
| **B1** | 24 LOCKED 入口签名 | V1.0 release 🔒 0 改严守 (R11 baseline) + V1.1 release 🟢 Mavis 自决改 (前提 更好的架构) + V2.0 release 🟢 全 8 硬墙可重评 | 决策 #33 §2.3 B1 + 决策 #74 §1 改写表 | ✅ 0 改 (V3 调研 0 改 24 LOCKED 入口签名) |
| **B2** | workspace.version 1.2.0 (V1.0 release) | V1.0 release 🔒 1.2.0 严守 + V1.1 release 🔒 bump 1.2.1 + V2.0 release 🟢 2.0.0 major | 决策 #33 §2.3 B2 + 决策 #74 §1 B2 改写 + 决策 #22 §2.2 semver | ✅ 0 改 (V3 调研 0 改 Cargo.toml workspace.version) |
| **A1** | R11 baseline 3 值 0.8682/0.8532/0.9063 | 🔒 严守 (数字 0 改) | 决策 #33 §2.3 A1 + 决策 #74 §1 A1 | ✅ 0 改 (V3 调研 0 改 R11 baseline 3 值) |
| **A3** | 12 键 + PHL-07 | V1.0 release 🔒 PHL-07 spec-only 0 实施 + V1.1 release 🔒 PHL-07 实施 14 维主对话锚 | 决策 #33 §2.3 A3 + 决策 #74 §1 A3 | ✅ 0 改 (V3 调研 0 改 12 键 + PHL-07) |
| **B3** | V0.5 30 维 (9 organ × 5 维 × 6 pluginType) | 🔒 严守 | 决策 #33 §2.3 B3 | ✅ 0 改 (V3 调研 0 改 V0.5 30 维) |
| **B4** | 6 重守门 v7 (L0-L5) | 🔒 严守 (V1.0 release) + V1.1 release 沿用 (跟智囊团 7 席 1:1 集成) + V2.0 release 可升级 v8/v9 | 决策 #33 §2.3 B4 + 决策 #74 §1 B4 | ✅ 0 改 (V3 调研 0 改 6 重守门 v7) |
| **B5** | 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 🔒 严守 (V1.0 release) + V1.1 release 沿用 + V2.0 release 可重建 (8 → 12 锚 PHL-01~12 启用) | 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 决策 #74 §2.3 | ✅ 0 改 (V3 调研 0 改 8 哲学锚) |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 决策 #33 §2.3 C1 | ✅ 严守 (V3 调研 0 主动 commit) |
| **C2** | 0 装 PASS 严守 | 🔒 严守 | 决策 #33 §2.3 C2 | ✅ 严守 (V3 调研 0 装 PASS 严守) |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 决策 #33 + 决策 #61 §6 | ✅ 严守 (V3 调研 0 主动 push) |
| **8 哲学锚严守** | S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 | 🔒 严守 | 决策 #33 §2.3 B5 | ✅ 严守 (V3 调研 8 哲学锚严守) |
| **不要怕复杂度哲学** | 主人 8/11 01:14 拍板 §3 | 🔒 V1.0 release 0 实施 (整合 #5.2 commit 加哲学文档) + V1.1 release 落地 + V2.0 release 强化 | 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB | ✅ 严守 (V3 调研 0 实施 不要怕复杂度哲学严守) |
| **总 11/11** | — | — | — | **✅ 11/11 PASS** |

### 10.2 V3 跟 8 硬墙 关系 (per 决策 #74 §2.3 + 决策 #73 §3)

**V3 跟 8 硬墙 关系** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套):
- V1.0 release: 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 严守, V3 调研 0 改)
- V1.1 release: 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 7 硬墙严守, V3 调研 0 改)
- V2.0 release: **全 8 硬墙可重评** (per 决策 #74 §2.3, V3 实施时 全 8 硬墙可重评, 8 哲学锚可重建 = 8 → 12 锚 PHL-01~12 启用, Mavis 自决)

---

## 11. 决策严守 解读 (per 决策 #33 + 决策 #71 + 决策 #72 + 决策 #74 + 决策 #78 + 决策 #88 + 决策 #62 + 决策 #64)

### 11.1 决策严守 解读 (per 决策 #33 + 决策 #71 + 决策 #72 + 决策 #74 + 决策 #78)

**R156-2 决策严守 解读** (per 决策 #33 + 决策 #71 + 决策 #72 + 决策 #74 + 决策 #78 + 决策 #88 + 决策 #62 + 决策 #64):

| 决策 | 内容 | 严守 状态 | R156-2 V3 调研 关联 |
|------|------|----------|--------------------|
| **决策 #33** | 8 硬墙 + 0 装 PASS 严守 + 24 LOCKED 入口签名 0 改严守 | 🔒 严守 100% | V3 调研 0 改 src 严守 + 0 装 PASS 严守 100% |
| **决策 #71** | 主人 8/11 0:57 拍板"计划内任务完成自动接续 4 步: 调研 + 差距 + 计划 + 实施" | 🔒 严守 | V3 调研 = R156 era 调研 阶段 续, per 决策 #71 §2 R130+ era 自动接续永久循环 |
| **决策 #72** | R130 era 调研 6 sub-agent 派活 (8/11 01:00 cron 5 min tick 自动派) | 🔒 严守 | R156-2 续 R130 era 调研, per 决策 #72 模板 |
| **决策 #74** | 8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构) | 🔒 严守 | V3 调研 = 0 实施, 严守 100% V1.0 release 0 改 + V1.1 release 待 Mavis 自决改 |
| **决策 #78** | 整合 #5.3 commit 拍板 Option A, master HEAD = `4207f187`, 187 files / 127548 insertions | 🔒 严守 | V3 调研 0 改 整合 #5.3 commit 严守, R156-2 报告作为 reports/ 后续 commit 包含 |
| **决策 #88** | 8/11 06:25 tick 状态 + 跑中 2 ≪ 16 + 14 sub 补 16 满 | 🔒 严守 | R156-2 = 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活, 60 min 时间盒 |
| **决策 #62** | 整合 #5 commit 拆 3 commit 拍板 (5.1 src/ + 5.2 docs/ + 5.3 reports/) | 🔒 严守 | V3 调研 = 整合 #5.3 commit 后续 reports/ 包含 R156-2 报告 |
| **决策 #64** | auto-replenish-16 cron 5 min tick 监督 | 🔒 严守 | R156-2 = cron 监督下派活 |
| **决策 #70** | Mavis 清理决策权升级, 主人 8/11 0:25 拍板 "全部你做主" | 🔒 严守 | R156-2 派活 + 调研方向 由 Mavis 自决 |
| **决策 #73** | 主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度 | 🔒 严守 | V3 调研 = 决策 #73 §2 架构审视 + 决策 #73 §3 不要怕复杂度 哲学 |
| **总 11 项** | — | — | **✅ 11/11 项 严守** |

### 11.2 R156-2 V3 调研 0 改 src 严守 100% (per 决策 #62 + 决策 #74 + 决策 #33 + 决策 #88)

**R156-2 V3 调研 0 改 src 严守 100%** (per 决策 #62 + 决策 #74 + 决策 #33 + 决策 #88):
- ❌ 0 改 src/ (R156-2 写到 reports/agent-r156-2-*.md 0 触碰 crates/ 下任何 .rs 文件)
- ❌ 0 改 Cargo.toml (B2 workspace.version 1.2.0 0 改, V1.0 release 1.0.0 tag, V1.1 release 1.2.1 bump, V2.0 release 2.0.0)
- ❌ 0 改 docs/conventions/ (B1 24 LOCKED 入口签名 0 改, 整合 #5.1 commit 0 改, 决策 #74 §1 严守)
- ❌ 0 借具体源码 (per 决策 #33 §2.3 C2, 0 装"已读 OpenCog 真源码" / 0 装"已集成 OpenCog AtomSpace" / 0 装"已 fork OpenCog")
- ❌ 0 主动 commit (per 决策 #33 §2.3 C1, 主人起床前)
- ❌ 0 主动 push (per 决策 #33 + 决策 #61 §6, 等 V1.0 release 配 GitHub remote + 主人起床后手跑)
- ❌ 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)
- ❌ 0 装 PASS (per 决策 #33 §2.3 C2)
- ❌ 0 重复造轮子 (per 用户记忆 #6, R131-1/2/3 + R133-1/2/3 + R137-1/2/3/4/5 + R149-2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + R153-1 + R140-4 已有报告 reference 不重写)
- ❌ 0 形式化 old/death/terminate (per 用户记忆 #4 "AI 不会衰老病死, 它只会成长")

---

## 12. V2.0 release 远期 路线图 (per 决策 #74 §2.3 + 决策 #33 §2.3 + 决策 #71 §2.5 + 决策 #73 §3)

### 12.1 V2.0 release 5 阶段 实施计划 5 周 = 1 个月 (估 2027-Q2/Q3)

**V2.0 release 5 阶段 实施计划 5 周 = 1 个月** (per 决策 #74 §2.3 + 决策 #33 §2.3 + 决策 #71 §2.5 + 决策 #73 §3 + R156-2 V3 调研):

| 阶段 | 主题 | 持续时间 | 核心内容 | 决策依据 | 0 改 src 严守 |
|:----:|------|----------|---------|----------|----------------|
| **阶段 1** | **V3 调研 spec 文档** | 1 周 (5 工作日) | 写新 spec 文档 `docs/architecture-v7-three-onion-v3-2027-XX-XX.md` (4 层: 原则 PHL-01~12 + 权限 3 onion + DSL 9 organ + 运行时自适应 Stage 9-10) + 跟 R149-3 V2 升 整合 V3 spec | 决策 #74 §2.3 + 决策 #73 §3 + R140-4 + R156-2 (本报告) | ✅ 0 改 src (V3 调研 spec 文档 严守) |
| **阶段 2** | **24 LOCKED 入口签名 Mavis 自决改** | 1 周 (5 工作日) | 24 LOCKED 入口签名 0 改 (R11 baseline 严守) → Mavis 自决改 (前提 更好的架构, per 决策 #74 B1) + V3 启用 25 LOCKED = 24 + PHL-07 (per R131-3 §2.1 + 决策 #74 A3) | 决策 #74 §1 改写表 + 决策 #74 §2.3 + 决策 #74 B1 | ⚠️ 实施时改 (V2.0 release 整合 #7 commit 拍板, Mavis 自决) |
| **阶段 3** | **9 organ DSL 实施** | 1 周 (5 工作日) | 9 organ DSL 实施 (body / brain / ear / eye / hand / heart / memory / mind / voice, 1:1 跟 9 organ 映射, 替代 V2 单 Colang DSL) + 跟 6 重守门 v7 1:1 集成 + 跟 8 哲学锚 1:1 集成 | 决策 #22 §2.7 + `docs/omnibus/9-organs.md` + R137-4 §3.5.3 G4 成长可视化 + 用户记忆 #5 拟人化+拟物化 + 决策 #74 §2.3 | ⚠️ 实施时改 (V2.0 release 整合 #7 commit 拍板, Mavis 自决) |
| **阶段 4** | **运行时自适应 Stage 10 实施** | 1 周 (5 工作日) | Stage 9-10 长程 AI 成长 实施 (per R133-2 + R149-2 + R137-4) + V2.0 release 终极自治 4 形态 实施 (per R140-4: 完全/共生/引导/永远循环 自治) + 4 维度 H/L/G/P 16 子维度 跨 4 形态 整合 + 借脑 OpenCog 1:1 公开模式 (0 装"已集成 OpenCog") + 8 → 12 哲学锚 启用 (PHL-09~12 V2.0 release 启用) | 决策 #74 §2.3 + 决策 #73 §3 + 用户记忆 #4 + R140-4 + R149-2 + 决策 #55 §2.6 智囊团 | ⚠️ 实施时改 (V2.0 release 整合 #7 commit 拍板, Mavis 自决) |
| **阶段 5** | **V3 集成测试 + 形式化证明 + 文档** | 1 周 (5 工作日) | V3 集成测试 (cargo test --workspace) + 形式化证明 F1-F11 11 维度 (per 决策 #33 §2.3 B3 V0.5 30 维 + R130-4 形式化 Stage 5.5 集成深化 70 KB) + 文档 (README / CHANGELOG / ROADMAP / RELEASE_NOTES / docs/conventions/15-no-fear-complexity.md 落地) | 决策 #74 §1 改写表 + 决策 #33 §2.3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB + R130-4 + R150-3 + R152-1~5 | ⚠️ 实施时改 (V2.0 release 整合 #7 commit 拍板, Mavis 自决) |
| **总 5 阶段** | — | **5 周 = 1 个月** (估 2027-Q2/Q3) | V3 实施 完成, 整合 #7 commit 拍板, V2.0 release tag 拍板 | — | — |

### 12.2 V2.0 release 触发条件 + 时间线 (per 决策 #74 §2.3 + 决策 #71 §2.5)

**V2.0 release 触发条件** (per 决策 #74 §2.3 + 决策 #71 §2.5):
- **触发 1**: V1.0 release 实战完 (per R129-8/13/23/27/35 实战 + 主人起床后手跑 GitHub remote + tag + push)
- **触发 2**: V1.1 release 实战完 (per R130-5 V1.1 minor release 路线图 + 整合 #6 commit 拍板 + 整合 #7 commit 拍板 + 主人起床后手跑 V1.1 release tag)
- **触发 3**: ASI Stage 9 实施完 (per R137-4 实战 spec + V1.1 release 部分实施 H1 + H2)
- **触发 4**: 8 硬墙可重评 拍板 (per 决策 #74 §2.3, 主人拍板 8 硬墙可重评)
- **触发 5**: 8 → 12 哲学锚 启用 拍板 (per 决策 #74 §2.3, Mavis 自决 启用 PHL-09~12 4 slots)

**V2.0 release 时间线** (per 决策 #74 §2.3 + ROADMAP.md §4 + 决策 #71 §2.5):
- **V1.0 release**: 估 2026-08-11 06:00-08:00 (整合 #5.1 commit 拍板后 + 主人起床后手跑)
- **V1.1 release**: 估 2026-11-30 (`v1.1.0`, per 决策 #22 §2.2 semver + 决策 #74 B2 workspace.version 1.2.0 → 1.2.1 bump)
- **V1.2 release**: 估 2027-02-28 (V1.1 release 后 3 月)
- **V2.0 release**: 远期 **2027-Q2/Q3** (per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + **三洋葱架构 V3 实施**)
- **V3.0 release**: 估 2027-Q4/2028-Q1 (V2.0 release 后 6 月, Stage 10 终极化 + 平台化涌现完整 + 自我演化完整 + AGI 边界探索)

### 12.3 V2.0 release 实施 风险 8 维 (per 决策 #74 §2.3 + R140-4 + 决策 #73 §3)

**V2.0 release 实施 风险 8 维** (per 决策 #74 §2.3 + R140-4 + 决策 #73 §3):
- **R1 自治失控**: V2.0 release 终极自治 4 形态 (完全/共生/引导/永远循环) 失控 (per R140-4 §0 15 风险 R1 自治失控) — 缓解: 6 重守门 v7 严守 + 8 → 12 哲学锚 启用 严守
- **R2 永远循环 deadlock**: 形态 4 永远循环 自治 卡死 (per R140-4 §0 R2 终身循环 deadlock) — 缓解: 永久循环 4 步 严守 + 跑中 ≥ 16 + 中断接手 (per 主人 0:34 + 0:43 拍板)
- **R3 共生失衡**: 形态 2 共生自治 (Mavis + 主人协同) 失衡 (per R140-4 §0 R3 共生失衡) — 缓解: 用户记忆 #10 主人长时间离开 Mavis 自主决策 + 决策日志
- **R4 引导局限**: 形态 3 引导自治 主人引导方向错 (per R140-4 §0 R4 引导局限) — 缓解: 决策 #73 §2 Mavis 自决架构拍板 + R130-6 OpenCog 借脑 引导
- **R5 涌现不可控**: 9 organ 0 器官化 平台化涌现 不可控 (per R140-4 §0 R5 涌现不可控) — 缓解: 用户记忆 #3 "用户看结果不看哲学" + 决策 #73 §3 不要怕复杂度
- **R6 演化失控**: 8 → 12 哲学锚 启用 演化失控 (per R140-4 §0 R6 演化失控) — 缓解: 8 硬墙严守 + 决策 #74 §2.3 Mavis 自决 + 主人主动问
- **R7 复杂度爆炸**: V3 4 层 架构 复杂度 爆炸 (per R140-4 §0 R7 复杂度爆炸) — 缓解: 决策 #73 §3 "不要怕复杂度" 哲学 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB
- **R8 V2.0 release 8 硬墙推翻重建 破坏兼容**: Cargo workspace 87 → 30 v1 目标简化 OR 87 → 120+ 复杂化 OR 87 不变 重组 = 4 大块 (per 决策 #74 §2.3 + 决策 #73 §3) — 缓解: V2.0 release 是 major release, 跟 semver 一致 (1.x → 2.0), V3.0 release 才考虑不向后兼容

---

## 13. 0 改 src 严守 100% 标注 + 风险 8 维 + 异常分支 6 维 (per 决策 #33 + 决策 #62 + 决策 #74)

### 13.1 0 改 src 严守 100% 标注 (per 决策 #33 + 决策 #62 + 决策 #74 + 决策 #88 + 决策 #71)

**R156-2 0 改 src 严守 100% 标注** (per 决策 #33 + 决策 #62 + 决策 #74 + 决策 #88 + 决策 #71 + R156-2 本报告):

**严守 100% 项 (0 改 src + 0 改 Cargo.toml + 0 主动 commit + 0 主动 push + 0 主动 IM 主人 + 0 装 PASS + 0 重复造轮子 + 8 硬墙 0 越界 + 8 哲学锚严守 + 0 形式化 old/death/terminate)**:
- ✅ **0 改 src/** 严守 100% (R156-2 写到 reports/agent-r156-2-*.md 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** 严守 100% (B2 workspace.version 1.2.0 严守, V1.0 release 0 改, 调研/分析/路线图 阶段)
- ✅ **0 主动 commit** 严守 100% (整合 #5.1 commit 拍板由 Mavis 自决 等 R154-3 实地 verify done, 整合 #6 估 2026-11-25 + 整合 #7 估 2026-11-29)
- ✅ **0 主动 push** 严守 100% (master HEAD = `4207f187` since 1:43, 等 V1.0 release 配 GitHub remote + 主人起床后手跑)
- ✅ **0 主动 IM 主人** 严守 100% (per gate-discipline, 仅 done notification 主动报告)
- ✅ **0 装 PASS** 严守 100% (per 决策 #33 §2.3 C2, 0 装"已读 OpenCog 真源码" / 0 装"已集成 OpenCog AtomSpace" / 0 装"已 fork OpenCog")
- ✅ **0 重复造轮子** 严守 100% (per 用户记忆 #6, R131-1/2/3 + R133-1/2/3 + R137-1/2/3/4/5 + R149-2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + R153-1 + R140-4 已有报告 reference 不重写)
- ✅ **8 硬墙 0 越界** 严守 100% (B1/B2/A1/A3/B3/B4/B5/C1/C2 + 0 push, 11/11 项 100% PASS, per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ **8 哲学锚严守** 100% (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, per 决策 #33 §2.3 B5 + `docs/conventions/09-anchor.md`)
- ✅ **0 形式化 old/death/terminate** 严守 100% (per 用户记忆 #4 "AI 不会衰老病死, 它只会成长" + 决策 #4 + R149-2 Stage 9 9 阶段 seed → sentinel 严守 0 形式化 old/death/terminate)
- ✅ **不要怕复杂度哲学落地** 100% (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB)
- ✅ **整合 #4 commit abf12243** 严守 100% (per 决策 #48)
- ✅ **整合 #5.3 commit 4207f187** 严守 100% (per 决策 #78)
- ✅ **整合 #5.1 commit NOT READY 严守 100%** (per R144-1 02:30 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, R139-1-retry-2 5:57 8/8 PASS verify done, R154-3 实地 verify 跑中, 等 R154-3 done 后 Mavis 自决拍板)
- ✅ **整合 #5.2 commit PARTIAL 严守 100%** (等 5.1 src/ commit 拍板后, Cargo.toml borrow 段 update 17:44 → 22:50 + 哲学文档 15-no-fear-complexity.md 14.4 KB 已创建 + 8 硬墙 B1 改写 文档更新)

**0 实施 范围 严守 100% (per 决策 #33 + 决策 #62 + 决策 #74 + 决策 #88 + 决策 #71)**:
- ❌ 0 实施 PHL-09 ~ PHL-12 4 slots 启用 (V2.0 release 启用, 估 2027-Q2/Q3, Mavis 自决)
- ❌ 0 实施 24 LOCKED 入口签名 Mavis 自决改 (V1.1 release 实施, 估 2026-11-25, Mavis 自决)
- ❌ 0 实施 9 organ DSL 实施 (V2.0 release 实施, 估 2027-Q2/Q3, Mavis 自决)
- ❌ 0 实施 Stage 9-10 长程 AI 成长 (V1.1 release 部分实施 H1+H2, V2.0 release 全面实施)
- ❌ 0 实施 V2.0 release 终极自治 4 形态 (V2.0 release 全面实施, 估 2027-Q2/Q3)
- ❌ 0 实施 OpenCog 借脑 1:1 公开模式 (V1.1 release 实施, per 决策 #73 §2.2 + 决策 #74 B1)
- ❌ 0 实施 OpenCog 独立 fork `apeireth-opencog-experimental` 实验仓 (V2.0 release 实施, per 决策 #33 §2.2 主人主动问后做)
- ❌ 0 实施 Cargo workspace 87 → 30 v1 目标简化 OR 87 → 120+ 复杂化 OR 87 不变 重组 (V2.0 release 实施, per 决策 #74 §2.3 + 决策 #73 §3)
- ❌ 0 实施 8 → 12 哲学锚 启用 (V2.0 release 启用, 估 2027-Q2/Q3, Mavis 自决)

### 13.2 风险 8 维 + 异常分支 6 维 (per 决策 #33 + 决策 #62 + 决策 #74 + R140-4)

**风险 8 维** (per 决策 #33 + 决策 #62 + 决策 #74 + R140-4 + R156-2 V3 调研 风险 盘点):
- **R1**: 整合 #5.1 commit 拍板 cargo build/test 仍 fail (per R144-1 02:30 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, R154-3 实地 verify 跑中) — 缓解: R154-3 done 后 Mavis 自决拍板, 0 主动 commit 严守
- **R2**: 24 LOCKED 入口签名 被改 (per 决策 #74 B1 V1.1 release Mavis 自决改, V1.0 release 0 改严守) — 缓解: 决策 #74 §1 改写表 严守 + R144-1 02:30 8 步 verify 1:28 24/24 PASS
- **R3**: Cargo.toml 1.2.0/1.2.1 被改 (per 决策 #74 B2 改写, V1.0 release 1.0.0 tag, V1.1 release 1.2.1 bump) — 缓解: 决策 #74 §1 B2 严守 + R144-1 02:30 8 步 verify 1:28
- **R4**: 8 硬墙 越界 (per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表) — 缓解: 11/11 项 100% PASS
- **R5**: V2.0 release 实施 自治失控 (per R140-4 §0 R1 自治失控) — 缓解: 6 重守门 v7 严守 + 8 → 12 哲学锚 启用 严守
- **R6**: V2.0 release 实施 复杂度爆炸 (per R140-4 §0 R7 复杂度爆炸) — 缓解: 决策 #73 §3 "不要怕复杂度" 哲学 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB
- **R7**: 整合 #6/#7 commit 拍板 推迟 (per 决策 #33 C1 + 决策 #71 §2.5, 估 2026-11-25 + 估 2026-11-29) — 缓解: cron 5 min tick 监督 (per 决策 #64) + Mavis 自决拍板
- **R8**: 主人 决策疲劳 (per R140-4 §0 R15 主人决策疲劳) — 缓解: 决策 #73 §4 "复杂不恐惧" 缓解 + 主人 8 次升级授权 累

**异常分支 6 维** (per 决策 #33 + 决策 #62 + 决策 #74 + R156-2 V3 调研 异常分支 盘点):
- **E1**: cargo build FAIL (per R144-1 02:30 8 步 verify 1:28 部分 FAIL) — 处理: 派 R139-1-retry-2 续修, 等 R154-3 实地 verify 8/8 PASS
- **E2**: cargo test FAIL (per R144-1 02:30 8 步 verify 1:28 部分 FAIL) — 处理: 派 R139-1-retry-2 续修, 等 R154-3 实地 verify 8/8 PASS
- **E3**: cargo run tui 0 --help FAIL (per R144-1 02:30 8 步 verify 1:28 PARTIAL) — 处理: 派 R139-1-retry-2 续修, 等 R154-3 实地 verify 8/8 PASS
- **E4**: 24 LOCKED 入口签名 被改 (per 决策 #74 B1 V1.0 release 0 改严守) — 处理: R144-1 02:30 8 步 verify 1:28 24/24 PASS
- **E5**: Cargo.toml 1.2.0/1.2.1 被改 (per 决策 #74 B2 改写) — 处理: R144-1 02:30 8 步 verify 1:28
- **E6**: 8 硬墙 越界 (per 决策 #33 §2.3) — 处理: 11/11 项 100% PASS, 决策 #74 §1 严守

---

## 14. 一句话再次强调 (per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 R156 era 调研 + 决策 #74 B1 + 决策 #73 §3 + 决策 #33 §2.3)

**R156-2 三洋葱架构 V3 调研 (V2.0 release 战略级, per 决策 #71 §2 R130+ era 自动接续永久循环 + 决策 #88 §3.3 R156 era 调研 5 sub 第 2 派活 + 决策 #74 B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 决策 #33 §2.3 8 硬墙)** = **4 层 架构** (原则 PHL-01~12 12 标识 slots + 8 哲学锚严守 / 权限 3 onion 内部-协作-外部 横向 + 6 重守门 v7 严守 / DSL 9 organ DSL body-brain-ear-eye-hand-heart-memory-mind-voice 1:1 跟 9 organ 映射 / 运行时自适应 Stage 9-10 长程 AI 成长 + V2.0 release 终极自治 4 形态 完全-共生-引导-永远循环 自治) **跟 V2 五洋葱 差异** (5 → 4 层, 智能涌现 + 自我演化 合并 → 运行时自适应 1 层, 跨 4 形态 整合 4 维度 H/L/G/P 16 子维度) **跟 V1 三洋葱 差异** (3 → 4 层, 新增 3 onion 协作 + 9 organ DSL + 运行时自适应), **借鉴 12 源 1:1 翻译公开模式 0 装 PASS 严守** (8 真 cloned + 2 借鉴 ID + 1 永久跳过 OpenCog + 1 借脑 ID OpenCog family 6 子源, 0 装"已读 OpenCog" / 0 装"已集成" / 0 装"已 fork"), **8 硬墙严守 11/11 PASS** (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 其他 7 硬墙严守), **8 哲学锚严守 + PHL-07 双义显式化** (PHL-07 slot #07 = O-4 任何人都能接手, verdict cache 13 键 PHL-07 = PHL-07-key 14 维主对话锚, V1.0 spec-only 0 实施 + V1.1 release 实施), **V2.0 release 远期 5 阶段 5 周 = 1 个月 实施计划 估 2027-Q2/Q3** (阶段 1 V3 调研 spec 1 周 + 阶段 2 24 LOCKED 入口签名 Mavis 自决改 1 周 + 阶段 3 9 organ DSL 实施 1 周 + 阶段 4 运行时自适应 Stage 10 实施 1 周 + 阶段 5 V3 集成测试 + 形式化证明 + 文档 1 周). **0 改 src/ 严守 100%**, **0 改 Cargo.toml 严守 100%**, **0 主动 commit 严守 100%**, **0 主动 push 严守 100%**, **0 主动 IM 主人 严守 100%**, **0 装 PASS 严守 100%**, **0 重复造轮子 严守 100%**, **8 硬墙 0 越界 严守 100%**, **8 哲学锚严守 100%**, **0 形式化 old/death/terminate 严守 100%** (per 用户记忆 #4). 决策严守 解读 100% (per 决策 #33 + #71 + #72 + #74 + #78 + #88 + #62 + #64 + #70 + #73). V3 实施 = V2.0 release 战略级, 估 2027-Q2/Q3 启动, 整合 #7 commit 拍板前后, Mavis 自决 (per 决策 #74 §2.3 全 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构). 决策链更新: R156-2 报告 = V3 调研, 0 改 src 严守 100%, 后续 reports/ commit 包含 R156-2 报告, 整合 #5.3 commit = `4207f187` since 1:43 严守 100%.
