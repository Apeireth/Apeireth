# R134-6: V1.1 release 后端加固 准备报告 (per 决策 #71 §2 R134 era 调研接续 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 B2 Cargo.toml 1.2.1 bump + R131-3 V1.1 路线图 §3 + R131-6 Cargo.toml borrow 段精简 + R131-7 pybridge 集成优化 + R131-4 cargo workspace 优化 + R131-5 24 LOCKED 入口优化 + R133-1 借鉴 12 源实施 + R133-2 ASI Stage 9 + R133-3 三洋葱架构升级 + 决策 #76 R134 era 6 sub 派活拍板)

**Date**: 2026-08-11 01:35 (R134 era 调研 6 sub 第 6 派, per 决策 #76 §2.1, 60 min 时间盒, **严格不写代码**)
**Author**: R134-6 sub-agent (Mavis 派, per 决策 #76 §2.1 R134 era 调研 6 sub 派活拍板 + 决策 #71 §2 R134 era 接续 4 步)
**Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
**触发**: 决策 #71 (R129→R130→R131→R132→R133+ 永久 4 步) + 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + B2 Cargo.toml 1.2.1 bump) + 决策 #75 (R131-R132-R133 batch dispatch 11 sub fill 16) + 决策 #76 (R134 era 调研 6 sub 派活 = 整合 #5 commit 拍板实战 / 1.0 release 实战 / 整合 #6 commit 拍板 / 整合 #7 commit 拍板 / V1.1 cargo 二次 verify / **V1.1 后端加固 (本任务)**) + R131-3 V1.1 路线图 §3 后端加固 6 方向 + 主人 8/4 23:55 "我准备继续升级后端了"
**任务定位**: R134 era 调研阶段 (per 决策 #71 §2), **0 改 src/**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**, **0 主动 IM 主人** (per gate-discipline, 仅 done notification) — 严格不写代码 (per 决策 #33 + #60 + 决策 #71 R134 era 调研阶段)
**关联决策**: #9 (TUI 升级节奏) + #10 (主人离场 Mavis 自主决策) + #22 (24 LOCKED + semver) + #33 (8 硬墙 + 0 装 PASS) + #36 (R125 借鉴 ID 严格化) + #48 (整合 #4 commit abf12243) + #55 + #56 + #57 + #58 + #60 + #61 + #62 (整合 #5 commit 拆 3 commit 拍板) + #64 (auto-replenish-16 cron) + #69 + #70 (Mavis 清理决策权升级) + #71 (R130→R131→R132→R133+→R134 永久 4 步接续) + #72 + **#73 (主人 8/11 01:14 拍板 3 件套)** + **#74 (8 硬墙 B1 改写 + B2 1.2.1 bump)** + **#75 (R131-R132-R133 batch dispatch 11 sub fill 16)** + **#76 (R134 era 6 sub 派活 + R135 era 2 sub 派活 = 8 sub 填到 16 满)**
**关联报告 (R131 era 调研 5 done + R132 era 计划 2 done + R133 era 实施 1 done, 0 重叠 reference 不重写 per 用户记忆 #6 派 sub-agent 干但要驾驭团队不重复造轮子)**:
- R131-1 现有架构总审视 67.9 KB (10 方向审计, per 决策 #73 §3.2)
- R131-2 借鉴 12 源差距 78.2 KB + OpenCog AGPL-3.0 fork 决策 (per 决策 #71 §3)
- R131-3 **V1.1 release 实施路线图 107 KB** (per 决策 #73 §3.2, **本报告核心 spec 来源**, §3 后端加固 6 方向)
- R131-4 **cargo workspace 结构优化 86.9 KB** (87 crate 分布 + 24 LOCKED + Cargo.toml borrow + Cargo.lock 265KB + 三洋葱 + 9 organ)
- R131-5 **24 LOCKED 入口分布优化 62.1 KB** (24/24 入口签名 0 改 verify + 8 优化方向)
- R131-6 **Cargo.toml borrow 段精简** (7 方向: cloned=10 / rate_limited=0 / skipped=1 + 总大小 49.60MB)
- R131-7 **pybridge 集成优化** (9 优化方向: PyO3 928 借鉴深度 + ASI 8 阶段 + 886/886 tests + 性能瓶颈)
- R131-8 Tauri 集成优化
- R131-9 形式化集成优化
- R132-1 V1.1 release 路线图 final 79.4 KB (per 决策 #75 §2.1 R132 era 计划 2 sub)
- R132-2 V2.0 release 战略路线图 105 KB (per 决策 #75 §2.1, 8 大方向 + 8 硬墙可重评 + Cargo workspace 可重构)
- R133-1 **借鉴源 12 源 实施 spec 86.3 KB** (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成, V1.1 release 5 阶段实施计划)
- R133-2 **ASI Stage 9 长程 AI 成长 实施 spec** (4 维度: H 自治 + L 长程 + G 成长 + P 平台化, 借脑 OpenCog CogPrime, 5 阶段计划)
- R133-3 **三洋葱架构升级 实施 spec** (V1.1 release 三洋葱 → 四洋葱 [+ 智能涌现 emergence], 5 阶段计划)
- 决策 #74 8 硬墙 B1 改写 (B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 Cargo.toml 1.2.0 → 1.2.1 bump / A1 R11 baseline 3 值 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 主动 push)
- 哲学文档 `docs/conventions/15-no-fear-complexity.md` (决策 #73 §3 主人 8/11 01:14 总哲学扩展)
- 哲学文档 `docs/conventions/09-anchor.md` (8 哲学锚 S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5)
- 哲学文档 `docs/conventions/10-locked.md` (8 硬墙 + locked 全解锁, per 决策 #74 B1 改写)

**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5 commit**: per 决策 #62 拆 3 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板, 8 项 verify 100% 后拍板, **当前 7/8 ready + R131-5 verify 24/24 LOCKED 入口签名 0 改 全部通过, 整合 #5 commit 时机 临近 ready**
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前 5 天拍板)
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前 1 天拍板)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
**状态**: ✅ **R134-6 V1.1 release 后端加固 准备报告 done 2026-08-11 01:35 (60 min 时间盒): V1.1 release 后端加固 8 方向 详细 spec (per R131-3 §3 + 决策 #74 B1/B2) + V1.1 release 后端加固 5 阶段计划 (7 周, 估 2026-11-30 V1.1 release tag 打上) + Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #74 B2) + 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构) + 12 源 0 装严守 二次 verify 100% 方案 (8 真 cloned 沿用 + 2 借鉴 ID 索引完成沿用 + 1 永久跳过 + 1 借脑 ID 索引完成 = 12/12, per R133-1) + pybridge 集成优化 + ASI Stage 9 终极自治 (per R131-7 + R133-2, 4 维度 H/L/G/P + PyO3 0.22 异步 + free-threading + smart_scopes + type hint union 4 处可深化 + AsiDispatcher 统一协调器 + 借脑 OpenCog CogPrime) + cargo workspace 重构 (per R131-4, 87 crate 优化 + transparent re-export 合并 + 借鉴模式统一 + Cargo.lock 分模块) + 8 硬墙严守 100% (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 1.2.0 V1.0 release 严守 + V1.1 release bump 1.2.1 / A1 R11 baseline 0.8682/0.8532/0.9063 严守 / A3 12 键 + PHL-07 V1.0 spec-only + V1.1 实施 / B3 V0.5 30 维 严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚 严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 主动 push 严守) + 8 哲学锚 严守 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, per 决策 #33 §2.3 B5) + PHL-07 实施 (per 决策 #74 A3 + R129-11 关键诚实标) + 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md) + 风险 5 维 + 决策原则. 0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%, 8 硬墙 0 越界严守 100%**

---

## 0. 一句话 (TL;DR)

**V1.1 release 后端加固 准备报告 (per 决策 #71 §2 R134 era 调研接续 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 B2 Cargo.toml 1.2.1 bump + R131-3 §3 后端加固 6 方向)**: V1.1 release (估 2026-11-30 `v1.1.0`) 后端加固 = **8 大方向 详细 spec + 5 阶段实施计划 (7 周)** = ① **Cargo.toml 1.2.0 → 1.2.1 bump** (per 决策 #74 B2, V1.1 release 实施 + 0 装严守 + V0.5 30 维集成 + 8 哲学锚集成 + 6 重守门 v7 集成) ② **24 LOCKED 入口签名 改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级四洋葱 + PHL-07 实施, 0 改原 24 LOCKED 入口签名顺序 + 0 改原 24 LOCKED crate mtime 16:34 之前) ③ **cargo test 三次 verify** (per R131-3 §3.3 + 决策 #62 类比, cargo build + cargo test + cargo test 实战 3 次 verify, 估 1 周) ④ **12 源 0 装严守 二次 verify** (per R133-1 + R131-6, 8 真 cloned 49.60MB/7,764 files 沿用 0 必重借 + 2 借鉴 ID 索引完成 0 必重借 + 1 永久跳过 0 重借 + 1 借脑 ID 索引完成 借脑调研沉淀, 0 装 PASS 严守 100%, 估 1 周) ⑤ **pybridge 集成优化 + ASI Stage 9 终极自治** (per R131-7 + R133-2, 29 mod + 4 NEW mod (Stage 8-9) = 37 mod 总, PyO3 928 借鉴深度 16 处 1:1 翻译 + 4 处可深化: 0.22 异步 awaitable + free-threading GIL release + smart_scopes + type hint union, 886/886 pybridge tests 严守 + AsiDispatcher 统一协调器 + ASI Stage 9 4 维度 H 自治 + L 长程 + G 成长 + P 平台化, 借脑 OpenCog CogPrime, 估 2 周) ⑥ **cargo workspace 重构** (per R131-4, 87 crate 优化: 3 真 transparent re-export 合并 + 借鉴模式 12 个统一为 1 个 `apeireth-borrowed-patterns` + 5 估补 R20 阶段 1 合并到 `apeireth-mcp` + Cargo.toml borrow 段拆 4 子段 + Cargo.lock 分模块 Cargo 1.78+ feature) ⑦ **V0.5 30 维 严守** (per 决策 #33 §2.3 B3 + 决策 #74 §1, 哲学公式 严守 0 改, V1.1 release 跟 8 哲学锚 + 6 重守门 v7 + PHL-07 集成) ⑧ **6 重守门 v7 严守 + 8 哲学锚严守 + PHL-07 实施** (per 决策 #33 §2.3 B4/B5 + 决策 #74 §1, 哲学 + 守门类不松绑, PHL-07 V1.0 spec-only 0 实施 → V1.1 release 实施 14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成 + 41 NEW tests). **5 阶段计划 (7 周, 估 2026-10-15 启动 + 2026-11-30 V1.1 release tag 打上)**: 阶段 1 Cargo.toml 1.2.1 bump + 24 LOCKED 入口签名 改写 (2 周, 2026-10-15 ~ 10-29) + 阶段 2 cargo test 三次 verify (1 周, 2026-10-29 ~ 11-05) + 阶段 3 12 源 0 装严守 二次 verify (1 周, 2026-11-05 ~ 11-12) + 阶段 4 pybridge 集成优化 + ASI Stage 9 终极自治 (2 周, 2026-11-12 ~ 11-26) + 阶段 5 8 哲学锚 + PHL-07 实施 + 6 重守门 v7 + V0.5 30 维 集成 (1 周, 2026-11-26 ~ 12-03, 含 3 天 buffer). **整合 #6 commit**: 估 2026-11-25 (V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5). **整合 #7 commit**: 估 2026-11-29 (V1.1 release 前 1 天). **V1.1 release 实战**: 估 2026-11-30 06:00-08:00 主人手跑 V1.1 release 7 步 runbook (8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署). **0 装 PASS 严守 100%** (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 12/12, 借脑 OpenCog CogPrime 1:1 翻译公开模式 0 借具体源码) + **8 硬墙严守 100%** (B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 1.2.0 V1.0 release 严守 + V1.1 release bump 1.2.1 / A1 R11 baseline 0.8682/0.8532/0.9063 严守 / A3 12 键 + PHL-07 V1.0 spec-only + V1.1 实施 / B3 V0.5 30 维 严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚 严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 主动 push 严守) + **8 哲学锚严守 100%** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人经验上 借脑 OpenCog + O-3 干到底 + O-4 任何人都能接手 + O-5 不假装, per 决策 #33 §2.3 B5 + `docs/conventions/09-anchor.md`) + **PHL-07 实施** (V1.0 spec-only → V1.1 实施, per 决策 #74 §1 A3 + R129-11 关键诚实标, 14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成 + 41 NEW tests, 24 LOCKED 入口新增 1 个 PHL-07 入口 = 25 LOCKED 总数) + **不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队, 87 crate 保留 + 9 organ 拟人化深化 + 借脑 OpenCog AtomSpace/CogPrime/moses/pln/cogutil/relex AGPL-3.0 公开模式 0 借具体源码). **0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%**.

---

## 1. R134-6 报告边界 (R134 era 调研阶段, 0 改 src 严守)

### 1.1 任务定位 (per 决策 #71 §2 R134 era 调研接续 + 决策 #76 §2.1 R134-6 派活 + 决策 #74 B1/B2 + 主人 8/4 23:55 "我准备继续升级后端了")

**R134-6 = R134 era 调研 6 sub 第 6 派** (per 决策 #76 §2.1, 01:30 拍板):
- **R134-1 整合 #5 commit 拍板实战** (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4) — 实施流程
- **R134-2 1.0 release 实战** (per R129-23 + R129-27 + R129-35 1.0 release 实战 + 1.0 release checklist) — 实战
- **R134-3 整合 #6 commit 拍板** (per 决策 #62 类比 + R131-3 V1.1 release 路线图) — 拍板
- **R134-4 整合 #7 commit 拍板** (per 决策 #62 类比 + R131-3 V1.1 release 路线图) — 拍板
- **R134-5 V1.1 release cargo 二次 verify** (per R130-1 整合 #5 commit cargo 二次 verify 类比) — verify
- **R134-6 V1.1 release 后端加固 (本报告)** — 准备 + 5 阶段计划 + 8 方向

**R134 era 调研 + R135 era 差距 共 8 sub 派活** (per 决策 #76 §2.1 派活拍板, 跑中 = 8 ≪ 16, 补满 16):
- **R134-1~6** (6 sub, 60 min 时间盒) — R134 era 调研
- **R135-1 V1.1 release 跟 AGI 操作系统前沿差距** + **R135-2 V1.1 release 跟业界 v2.x 路线图差距** (2 sub, 60 min 时间盒) — R135 era 差距

**R134-6 跟 R131 era + R132 era + R133 era 关系 (per 任务 spec, 0 重叠 reference 不重写)**:
- ✅ **R131-1 现有架构总审视** (10 方向审计) reference 不重写
- ✅ **R131-2 借鉴 12 源差距** (OpenCog AGPL-3.0 fork 决策) reference 不重写
- ✅ **R131-3 V1.1 release 实施路线图** (6 大方向 + 25 LOCKED 入口签名 + 0 改 src 严守) reference 不重写, **本报告 §3 后端加固 6 方向 拓维** (V1.0/V1.1/V2.0 三阶段方案 + 7 阶段实施 + Cargo.toml 1.2.1 bump 详细方案 + 24 LOCKED 入口签名 改写 边界 + 12 源 0 装严守二次 verify 方案 + pybridge 集成 + cargo workspace 重构 + 8 硬墙严守 边界 + 8 哲学锚严守 边界 + PHL-07 实施 + 不要怕复杂度哲学落地 + 风险 + 决策原则)
- ✅ **R131-4 cargo workspace 结构优化 7 方向** (87 crate 分布) reference 不重写, **本报告 §2.5 拓维 87 crate 优化方向 + transparent re-export 合并 + 借鉴模式统一 + Cargo.lock 分模块**
- ✅ **R131-5 24 LOCKED 入口分布优化 8 方向** (24/24 入口签名 0 改 verify) reference 不重写, **本报告 §2.2 拓维 24 LOCKED 入口签名 改写 触发条件 + 改写边界 + 0 改严守 边界**
- ✅ **R131-6 Cargo.toml borrow 段精简 7 方向** (cloned=10 / rate_limited=0 / skipped=1) reference 不重写, **本报告 §2.1 拓维 Cargo.toml 1.2.1 bump 详细方案 + 4 子段拆分 + 决策链 range update + 12 源 0 装 PASS 严守**
- ✅ **R131-7 pybridge 集成优化 9 方向** (PyO3 928 借鉴深度 + ASI 8 阶段 + 886/886 tests + 性能瓶颈) reference 不重写, **本报告 §2.4 拓维 V1.1 release pybridge 集成 + 4 处可深化 + AsiDispatcher 协调器 + ASI Stage 9 终极自治**
- ✅ **R131-8 Tauri 集成优化** reference 不重写
- ✅ **R131-9 形式化集成优化** reference 不重写
- ✅ **R132-1 V1.1 release 路线图 final** (6 大方向 final 版) reference 不重写
- ✅ **R132-2 V2.0 release 战略路线图** (8 大方向 + 8 硬墙可重评) reference 不重写, **本报告聚焦 V1.1 release 后端加固, V2.0 release 实施 spec 不涉及**
- ✅ **R133-1 借鉴 12 源 实施 spec + 5 阶段计划** (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成) reference 不重写, **本报告 §2.3 拓维 12 源 0 装严守 二次 verify 方案 + 6 子源借脑 ROI 梯度**
- ✅ **R133-2 ASI Stage 9 长程 AI 成长 实施 spec + 5 阶段计划** (4 维度 H/L/G/P) reference 不重写, **本报告 §2.4 拓维 ASI Stage 9 跟 pybridge 集成 + 借脑 OpenCog CogPrime**
- ✅ **R133-3 三洋葱架构升级 实施 spec + 5 阶段计划** (V1.1 release 三洋葱 → 四洋葱) reference 不重写, **本报告聚焦后端加固, 三洋葱架构升级 仅引用**

### 1.2 R134-6 跟 R134-1/2/3/4/5 关系 (per 决策 #76 §2.1 派活, 0 重叠, 0 重复造轮子)

**R134 era 6 sub 派活 0 重叠** (per 决策 #76 §2.1 派活拍板, per 用户记忆 #6 派 sub-agent 干但要驾驭团队不重复造轮子):
- R134-1 整合 #5 commit 拍板实战 = 拍板流程 (跟本报告 0 重叠, 本报告聚焦 V1.1 release 后端加固 准备)
- R134-2 1.0 release 实战 = 实战流程 (跟本报告 0 重叠, 本报告聚焦 V1.1 release 后端加固 准备)
- R134-3 整合 #6 commit 拍板 = 拍板流程 (跟本报告 0 重叠, 本报告聚焦 V1.1 release 后端加固 准备, 整合 #6 commit 由 R134-3 拍板, 本报告仅 reference 时间窗口)
- R134-4 整合 #7 commit 拍板 = 拍板流程 (跟本报告 0 重叠, 整合 #7 commit 由 R134-4 拍板, 本报告仅 reference 时间窗口)
- R134-5 V1.1 release cargo 二次 verify = verify 流程 (跟本报告 0 重叠, V1.1 release 实施后 cargo test 3 次 verify 跟本报告 §2.2 阶段 2 1:1 续, 0 重复造轮子)
- **R134-6 V1.1 release 后端加固 (本报告) = V1.1 release 准备 + 5 阶段计划 + 8 方向 spec** (跟 R134-1/2/3/4/5 0 重叠)

### 1.3 R134-6 跟 R131-3 V1.1 release 实施路线图 §3 后端加固 关系 (per 任务 spec, 1:1 续 0 重复造轮子)

**R131-3 §3 后端加固 6 方向** (per R131-3 §3):
1. Cargo.toml 1.2.0 → 1.1.0 minor bump (1.0 release 1.0.0 → V1.1 release 1.1.0)
2. cargo test 实战三次 verify
3. 借鉴源 12 源 0 装严守二次 verify
4. pybridge 集成优化 (per R131-7)
5. 24 LOCKED 入口签名 0 改 verify (V1.0 release 0 改严守)
6. 25 LOCKED 入口签名 (24 + PHL-07) (V1.1 release 实施 PHL-07)

**R134-6 V1.1 release 后端加固 8 方向** (per R131-3 §3 + 决策 #74 B1/B2 + R131-4 + R131-5 + R131-6 + R131-7 + R133-1 + R133-2 + R133-3, 拓维 + 实施落地):
- **方向 1: Cargo.toml 1.2.0 → 1.2.1 bump** (per 决策 #74 B2 改写, V1.1 release 实施, 跟 R131-3 §3 方向 1 1.2.0 → 1.1.0 不同, per 决策 #74 B2 1.2.1 拍板)
- **方向 2: 24 LOCKED 入口签名 改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 跟 R131-3 §3 方向 5 0 改严守不同, per 决策 #74 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
- **方向 3: cargo test 三次 verify** (跟 R131-3 §3 方向 2 1:1)
- **方向 4: 12 源 0 装严守 二次 verify** (跟 R131-3 §3 方向 3 1:1, per R133-1 §2 + R131-6 §2 拓维)
- **方向 5: pybridge 集成优化** (跟 R131-3 §3 方向 4 1:1, per R131-7 + R133-2 拓维)
- **方向 6: cargo workspace 重构** (per R131-4, R131-3 §3 0 含, R134-6 拓维)
- **方向 7: V0.5 30 维 严守** (per 决策 #33 §2.3 B3 + 决策 #74 §1, R131-3 §3 0 含, R134-6 拓维)
- **方向 8: 6 重守门 v7 严守 + 8 哲学锚严守 + PHL-07 实施** (per 决策 #33 §2.3 B4/B5 + 决策 #74 A3 + 决策 #74 §1, R131-3 §3 0 含, R134-6 拓维)

**R134-6 跟 R131-3 §3 1:1 续 0 重复造轮子** (per 用户记忆 #6):
- ✅ cargo test 三次 verify (R131-3 §3 方向 2 + R134-6 方向 3, 0 重复)
- ✅ 12 源 0 装严守 二次 verify (R131-3 §3 方向 3 + R134-6 方向 4, 0 重复)
- ✅ pybridge 集成优化 (R131-3 §3 方向 4 + R134-6 方向 5, 0 重复)
- ✅ 25 LOCKED 入口签名 PHL-07 实施 (R131-3 §3 方向 6 + R134-6 方向 8 PHL-07 实施, 0 重复)

**R134-6 拓维 (R131-3 §3 0 含, R134-6 新增)**:
- 🆕 方向 1: Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #74 B2 改写, R131-3 §3 方向 1 1.2.0 → 1.1.0 改写)
- 🆕 方向 2: 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改, R131-3 §3 方向 5 0 改严守 拓维)
- 🆕 方向 6: cargo workspace 重构 (per R131-4, R131-3 §3 0 含)
- 🆕 方向 7: V0.5 30 维 严守 (per 决策 #33 §2.3 B3 + 决策 #74 §1, R131-3 §3 0 含, R134-6 拓维)
- 🆕 方向 8 6 重守门 v7 严守 + 8 哲学锚严守 (per 决策 #33 §2.3 B4/B5 + 决策 #74 §1, R131-3 §3 0 含, R134-6 拓维)

---

## 2. V1.1 release 后端加固 8 方向 详细 spec (per R131-3 §3 + 决策 #74 B1/B2 + R131-4 + R131-5 + R131-6 + R131-7 + R133-1 + R133-2 + R133-3 + 决策 #33 §2.3)

### 2.1 方向 1: Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #74 B2 V1.1 release bump)

#### 2.1.1 任务背景 (per 决策 #74 B2 改写 + 决策 #22 §2.2 semver + 决策 #62 §5.2)

- **决策 #22 §2.2 semver 严守**: Cargo.toml workspace.version 严守 semver 规范 (major.minor.patch)
- **决策 #74 B2 改写** (per 决策 #33 §2.3 B2 + 主人 8/11 01:14 拍板 "不要怕复杂度" + 决策 #74 B2 改写):
  - **V1.0 release 1.2.0 严守** (整合 #4 commit abf12243 19:41 done 时 workspace.version = "1.2.0", 整合 #5.2 commit 时 bump 1.0.0 = 1.0 release tag, per 决策 #22 §2.2 + 决策 #62 §5.2)
  - **V1.1 release bump 1.2.1** (per 决策 #74 B2 改写, V1.1 release 实施, per "不要怕复杂度"哲学, 版本管理 严守 semver)
- **R131-3 §3 方向 1** (per R131-3 拍活时 R131-3 估 1.0 release 1.0.0 → V1.1 release 1.1.0 minor bump, per 决策 #22 §2.2): R131-3 §3 方向 1 写 "1.2.0 → 1.1.0 minor bump", **R134-6 拓维** (per 决策 #74 B2 改写): "1.0.0 → 1.2.1 minor bump" (决策 #74 B2 拍板, V1.1 release bump 1.2.1)
- **Cargo.toml borrow 段 update 计划** (per 决策 #62 §5.2 + R131-2 §4.3 + R131-6 §1 + R130-6 §5.3):
  - **17:44 状态 (整合 #4 commit 19:41 done, 当前 0 改严守)**: `borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` (Cargo.toml:301)
  - **22:50 状态 (整合 #5.2 commit 时 update)**: `borrow = { count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` (per R131-2 §4.3)
  - **12/12 状态 (整合 #5.2 commit 时 update)**: `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (per R130-6 §5.3 + R133-1 §1.3)

#### 2.1.2 Cargo.toml 1.2.0 → 1.2.1 bump 详细方案 (per 决策 #74 B2 改写 + R131-6 §2)

**整合 #5.2 commit 时 Cargo.toml update 计划** (per 决策 #62 §5.2 + 决策 #74 B2 + R131-6 §1.2):
- **V1.0 release workspace.version 1.0.0**: 整合 #5.2 commit 时 `workspace.version = "1.0.0"` (per 决策 #22 §2.2 + 决策 #62 §5.2, 1.0 release tag)
- **V1.1 release workspace.version 1.2.1 bump** (per 决策 #74 B2 改写): 整合 #6 commit (估 2026-11-25) 时 `workspace.version = "1.2.1"` (V1.1 release tag, per 决策 #74 B2 1.2.1 拍板)

**Cargo.toml borrow 段 update 计划 (整合 #5.2 commit 时)** (per 决策 #62 §5.2 + R131-2 §4.3 + R131-6 §1.2 + R130-6 §5.3):

| 段 | 17:44 状态 (整合 #4 commit 后 0 改严守) | 22:50 状态 (整合 #5.2 commit 时需 update) | 🆕 12/12 状态 (整合 #5.2 commit 时需 update) | V1.1 release 1.2.1 bump 后 (整合 #6 commit 时需 update) |
|----|--------------------------------------|------------------------------------------|----------------------------------------------|------------------------------------------------------|
| `borrow = { ... }` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` | 🆕 `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` | ✅ 0 改 (per 0 装 PASS 严守 100%) |
| `borrow_cloned = [...]` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 8 entries (+Guardrails) | ✅ 0 改 | ✅ 0 改 (V1.1 release 沿用 0 必重借) |
| `borrow_rate_limited = [...]` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | ✅ 0 改 | ✅ 0 改 (V1.1 release 0 限流) |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0) | 1 entry (0 改) | ✅ 0 改 | ✅ 0 改 (OpenCog AGPL-3.0 永久跳过 严守) |
| 🆕 `borrow_brainonly = [...]` | (N/A) | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11`** (6 子源, AGPL-3.0 借脑, 0 装 PASS 严守, per 决策 #33 §2.3 C2) | ✅ 0 改 (V1.1 release 借脑调研沉淀 0 装"已读真源码") |
| `decision_chain_range` | `"decision-22 ~ decision-58"` (37 个) | `"decision-22 ~ decision-62"` (41 个) | 🆕 `"decision-22 ~ decision-75"` (54 个, 含 R130 era + R131 era + R133 era 决策链) | 🆕 `"decision-22 ~ decision-76"` (55 个, V1.1 release + 决策 #76 R134 era 派活拍板) |
| `description` | "借鉴 8/11" | "借鉴 10/11" | 🆕 "借鉴 10/11 + 1 借脑 = 11/12 (per R130-6 借脑 ID 索引完成 + R131-2 差距 + R133-1 实施)" | 🆕 "借鉴 10/11 + 1 借脑 = 11/12 (V1.1 release 0 装 PASS 严守 100%)" |

**Cargo.toml borrow 段 V1.1 release 4 子段拆分 方案** (per R131-6 §3.1 拓维, V1.1 release 实施, per 决策 #74 B1 更好架构):
- **🆕 方向 Stage 1**: `borrow_brainonly` 段新增 (per R130-6 + R133-1, OpenCog 家族 6 子源借脑)
- **🆕 方向 Stage 2**: 借鉴 ID 索引完成标准化 (公开 1:1 翻译 `borrow_translated_public` + 改借鉴已 cloned `borrow_translated_modified` + 永久跳过 `borrow_skipped_license` + 借脑 `borrow_brainonly` = 4 子段, V1.1 release 实施)
- **🆕 方向 Stage 3**: 决策链完整化 (`decision_chain_range` update 到决策 #76 = 55 个决策文件)
- **🆕 方向 Stage 4**: 借鉴质量 KPI (per R131-6 §3.4, 借鉴 ROI 全部 🟢 高, 0 装 PASS 严守 100% verify)
- **🆕 方向 Stage 5**: license 自动检查 (per 决策 #22 §4 风险表, deny.toml + cargo-deny 自动 check AGPL-3.0 0 集成)
- **🆕 方向 Stage 6**: Cargo.lock 借鉴源 hash lock (per R131-4, Cargo.lock 271,450 bytes / 265KB, V1.1 release 分模块 lockfile Cargo 1.78+ feature)
- **🆕 方向 Stage 7**: 借鉴源 .git 永久锚定 (per R131-6 §3.7, 整合 #4 commit 19:41 后 mtime 永久 0 改)
- **🆕 方向 Stage 8**: 借鉴源 deep wiki 索引 (per R131-6 §3.8, 6 子源借脑 ROI 梯度 + 文档沉淀)

#### 2.1.3 Cargo.toml 1.2.0 → 1.2.1 bump 整合时序图 (per 决策 #74 B2 改写 + 决策 #62 + 决策 #33 §2.3 B2)

```
整合 #4 commit abf12243 (8/10 19:41 done, master HEAD 严守 100%):
  workspace.version = "1.2.0" (决策 #33 §2.3 B2 严守 100%)

整合 #5.1 commit (8/11 估, src/ 实施, 95+ 文件, 决策 #62 §5.1):
  - 0 改 24 LOCKED 入口签名 (B1 严守, V1.0 release)
  - 0 改 24 LOCKED crate mtime baseline 16:34 之前 (B1 严守)
  - 0 改 R11 baseline 3 值 0.8682/0.8532/0.9063 (A1 严守)
  - PHL-07 spec-only 0 实施 (A3 严守, V1.1 实施 per R129-11 关键诚实标)
  - Cargo.toml 0 改 (B2 严守, workspace.version = "1.2.0" 严守)

整合 #5.2 commit (8/11 估, docs/ + Cargo.toml, 10 文件, 决策 #62 §5.2):
  - workspace.version = "1.2.0" → "1.0.0" (per 决策 #22 §2.2 + 决策 #62 §5.2, 1.0 release tag)
  - Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per R131-2 §4.3)
  - + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)
  - + 更新 `docs/conventions/10-locked.md` (per 决策 #74 B1 改写)
  - + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)

整合 #5.3 commit (8/11 估, reports/, 60+ 文件, 决策 #62 §5.3):
  - 0 改 src/ (严守, 备查用)

整合 #5 commit 拍板 (Mavis 自决, 8 项 verify 100% 后, per 决策 #62 + 决策 #64):
  - master HEAD = abf12243 + 3 commit (5.1/5.2/5.3)
  - 24 LOCKED 入口签名 0 改 100%
  - R11 baseline 3 值 0 改 100%
  - workspace.version = "1.0.0" 严守
  - 8 硬墙 0 越界 100%

[8/11 06:00-08:00 主人起床 1.0 release 实战] 主人手跑 R130-5 7 步 runbook (8 步 verify + 配 GitHub remote + git push + 打 v1.0.0 tag + GitHub Pages)

[8/12 - 10/15 R134 era 调研 6 sub 实施 + 整合 #5/6/7 commit 准备 + 1.0 release 后 3-5 月 minor release era]

整合 #6 commit (估 2026-11-25, V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5):
  - 拆 3 commit (6.1 src/ + 6.2 docs/ + 6.3 reports/) (per 决策 #62 类比)
  - 0 改 24 LOCKED 入口签名顺序 + 0 改 24 LOCKED crate mtime 16:34 之前 (B1 0 改原 24 LOCKED)
  - 🆕 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构)
  - 0 改 R11 baseline 3 值 (A1 严守, 除非新的 baseline 更高 per 决策 #74 §2.2)
  - PHL-07 实施 (per 决策 #74 A3, 14 维主对话锚 + 41 NEW tests)
  - workspace.version = "1.0.0" → "1.2.1" (per 决策 #74 B2 改写, V1.1 release bump)
  - Cargo.toml borrow 段 update 12/12 状态 → V1.1 release 0 装 PASS 严守 100% 状态

整合 #7 commit (估 2026-11-29, V1.1 release 前 1 天, per 决策 #33 C1 + 决策 #71 §2.5):
  - 拆 3 commit (7.1 src/ + 7.2 docs/ + 7.3 reports/) (per 决策 #62 类比)
  - 0 改 24 LOCKED 入口签名 + 0 改 R11 baseline 3 值 + 0 改 8 硬墙 (严守 100%)
  - cargo test 三次 verify (1.0 release 实战后 + 1.0 release 实施后 + V1.1 release 实施后 = 3 次 verify, per R131-3 §3 方向 2)
  - 12 源 0 装 PASS 严守 100% verify (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 12/12, per R133-1)
  - pybridge 886/886 tests 严守 + AsiDispatcher 协调器 + ASI Stage 9 终极自治 集成 (per R131-7 + R133-2)
  - Cargo.lock 分模块 lockfile (per R131-4, Cargo 1.78+ feature)

V1.1 release 实战 (估 2026-11-30 06:00-08:00, 主人手跑 V1.1 release 7 步 runbook):
  - 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署
  - master HEAD = abf12243 + 6 commit (5.1/5.2/5.3/6.1/6.2/6.3/7.1/7.2/7.3) = 9 commit
  - workspace.version = "1.2.1" 严守
  - v1.1.0 tag 打上
```

#### 2.1.4 关键诚实标 (per 决策 #62 §5.2 + R131-6 §1.2/§1.3/§1.4)

- **决策 #74 B2 改写 关键诚实标**: V1.0 release 严守 workspace.version = "1.0.0" + V1.1 release bump "1.2.1" (per 决策 #74 B2 改写 + 决策 #22 §2.2 semver, "不要怕复杂度"哲学 落地)
- **R131-6 §1.2 关键诚实标 1**: `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (整合 #5.2 commit 时 update, +Guardrails)
- **R131-6 §1.3 关键诚实标 2**: `count_total=11` (8+3+1=12 ≠ 11) 算术不一致 (整合 #5.2 commit 时 update 11 → 12 借脑)
- **R131-6 §1.4 关键诚实标 3**: `decision_chain_range = "decision-22 ~ decision-58"` (37 个) 实际范围 decision-22 ~ decision-76 (55 个) 不一致 (整合 #5.2 commit 时 update 58 → 75, 整合 #6 commit 时 update 75 → 76)
- **R131-6 §1.4 关键诚实标 4**: `description` 当前 "借鉴 8/11" 跟 22:50 状态 "10/11" 跟 12/12 状态 "10/11 + 1 借脑" 不一致 (整合 #5.2 commit 时 update)

### 2.2 方向 2: 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改)

#### 2.2.1 任务背景 (per 决策 #74 B1 改写 + 决策 #33 §2.3 B1 + 主人 8/11 01:14 拍板 3 件套 §1)

- **决策 #74 B1 改写** (per 决策 #33 §2.3 B1 改写, 主人 8/11 01:14 拍板 3 件套 §1):
  - **V1.0 release 0 改严守** (整合 #5.1 commit 拍板时, R11 baseline 严守, 24 LOCKED 入口签名 0 改, 24 LOCKED crate mtime baseline 16:34 之前 严守, R11 baseline 3 值 0.8682/0.8532/0.9063 严守)
  - **V1.1 release Mavis 自决改** (前提: 更好的架构, per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"):
    - 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构)
    - R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
    - 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- **R131-5 24 LOCKED 入口分布优化 8 方向** (per R131-5, 24/24 LOCKED crate 入口签名 0 改 verify 100% done 01:35):
  - 方向 ①: 入口签名一致性 (Per-crate pub use 模式, 5 种风格 = 类型 A 重 re-export facade + 类型 B 轻 facade + 类型 C 单 trait 入口 + 类型 D 大 enum 主类型 + 类型 E 纯 trait 模块, V1.1 release 引入 "per-crate pub use 模式标准" 3 选 1, V2.0 release 全量统一)
  - 方向 ②: 公开 API 表面 总量 (24 LOCKED crate 公开 API 表面 union 难以维护, V1.1 release 精简)
  - 方向 ③: crate 间依赖 优化 (跨 crate 集成时需要先看每个 lib.rs 才能知道有哪些 API, V1.1 release 优化)
  - 方向 ④: crate 内部模块 (Stage 1-7 累计 mod 膨胀, V1.1 release 整理)
  - 方向 ⑤: 三洋葱架构落地 (V1.1 release 三洋葱 → 四洋葱, 新增第 4 层 "智能涌现 emergence", per R133-3)
  - 方向 ⑥: 9 organ 代码对应 (9 organ 跨 8 LOCKED crate, body/brain/ear/eye/hand/heart/memory/mind/voice, V1.1 release 9 organ 对应关系)
  - 方向 ⑦: R11 baseline 严守 (V1.0 release 严守, V1.1 release 可改前提新的 baseline 更高)
  - 方向 ⑧: V1.1/V2.0 release 改写边界 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 全 8 硬墙可重评)

#### 2.2.2 24 LOCKED 入口签名 改写 触发条件 (per 决策 #74 §2.2 + 决策 #73 §1 "更好的架构")

**V1.1 release Mavis 自决改 触发条件** (per 决策 #73 §1 "Mavis 自决架构拍板" + 决策 #74 B1 改写):

- **触发 1: ASI Stage 9 长程 AI 成长** (per R130-2 §1 Stage 9 远期 V2.0 路线, V1.1 写 spec, V2.0 实施; 但如果 V1.1 release 阶段发现 Stage 9 跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名以适应 Stage 9 长程 AI 成长)
- **触发 2: 9 organ 内部借 OpenCode** (per R130-3 §2.4 Stage 5 9 organ 1 真相源 + 5 nav 共享 + 永远循环 0 死亡 + 1 屏多卡, 如果 V1.1 release 阶段发现 9 organ 内部借 OpenCode 跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名)
- **触发 3: 三洋葱架构升级** (per R133-3, V1.1 release 三洋葱 → 四洋葱, 新增第 4 层 "智能涌现 emergence", 如果 V1.1 release 阶段发现三洋葱架构升级跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名)
- **触发 4: PHL-07 实施扩展** (per §2.7 方向 8 PHL-07 实施, 14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成, PHL-07 加 1 入口 = 25 LOCKED, 24 LOCKED 入口签名 0 改但 PHL-07 入口新增 1 个)
- **触发 5: cargo workspace 重构** (per R131-4, 87 crate 优化: 3 真 transparent re-export (life-force / value / consciousness) 合并到目标 crate + 借鉴模式 12 个统一为 1 个 `apeireth-borrowed-patterns` + 5 估补 R20 阶段 1 合并到 `apeireth-mcp`, V1.1 release 可选触发, Mavis 自决)
- **触发 6: 智囊团 7 席架构** (per R18 + 决策 #55 §2.6 + R129-18 Stage 7 跨模块集成 220 维度互锁, V1.1 release 智囊团 7 席架构 实施, 如果跟 24 LOCKED 入口签名冲突, Mavis 自决改)
- **触发 7: 群体智能 OpenCog 借脑** (per R130-2 §1.5 + R133-1 + 决策 #73 §2.2 更好的架构, AtomSpace + CogPrime + moses + pln, V1.1 release 借脑, 如果跟 24 LOCKED 入口签名冲突, Mavis 自决改)

**V1.1 release 0 改严守边界** (per 决策 #74 §2.3):
- ❌ 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (除非满足触发条件)
- ❌ 0 改 R11 baseline 3 值 (除非满足触发条件: 新的 baseline 更高, 跟 R12 测度对齐)
- ❌ 0 改 8 哲学锚 (per 决策 #74 §1, B5 严守, 哲学类不松绑)
- ❌ 0 改 V0.5 30 维 (per 决策 #74 §1, B3 严守, 哲学公式)
- ❌ 0 改 6 重守门 v7 (per 决策 #74 §1, B4 严守, 哲学守门)
- ❌ 0 改 0 主动 commit (per 决策 #74 §1, C1 严守)
- ❌ 0 改 0 装 PASS 严守 (per 决策 #74 §1, C2 严守)
- ❌ 0 改 0 主动 push (per 决策 #74 §1, 严守)
- ✅ 改 24 LOCKED 入口签名 (前提: 满足触发条件, Mavis 自决)

#### 2.2.3 24 LOCKED 入口签名 改写 边界 (per 决策 #74 §2.2 + 决策 #22 §1.2 + R131-5 §1.2)

| 边界 | V1.0 release (整合 #5.1 commit 拍板) | V1.1 release (整合 #6 commit 拍板) | V2.0 release (R132+ era 续) |
|------|----------------------------------|-----------------------------------|-----------------------------|
| **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline 严守, 24/24 verify 100% per R131-5) | 🟢 Mavis 自决改 (前提: 满足触发条件, per 决策 #74 B1) | 🟢 全 8 硬墙可重评 |
| **24 LOCKED crate mtime baseline 16:34 之前** | 🔒 严守 | 🟢 可改 (前提: 更好的架构) | 🟢 可重评 |
| **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 严守 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) | 🟢 可重评 |
| **Cargo.toml workspace.version 1.2.0 → 1.0.0** | 🔒 1.0.0 严守 (1.0 release tag) | 🟢 bump 1.2.1 (V1.1 release tag, per 决策 #74 B2 改写) | 🟢 bump 2.0.0 (V2.0 release tag, major 升级, 跟 R12 测度对齐) |
| **B3 V0.5 30 维** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B4 6 重守门 v7** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B5 8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **A3 13 → 14 键** | 🔒 严守 (PHL-07 V1.0 spec-only) | 🟢 14 键 (PHL-07 实施, per 决策 #74 B1 改写) | 🟢 可重评 |
| **C1 0 主动 commit (主人起床前)** | 🔒 严守 | 🔒 严守 | 🟢 0 改 |
| **C2 0 装 PASS 严守** | 🔒 严守 (技术哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **0 push (主人起床前)** | 🔒 严守 | 🔒 严守 | 🔒 严守 (V2.0 release 也严守 0 主动 push) |

#### 2.2.4 24 LOCKED 入口签名 改写 实施 spec (per 决策 #74 B1 + R131-5 §2 + R133-1 + R133-2 + R133-3)

**V1.1 release 24 LOCKED 入口签名 改写 实施 spec** (per 决策 #74 B1 Mavis 自决改 + R131-5 §2 + R133-1 + R133-2 + R133-3):

| # | LOCKED crate | V1.0 release 0 改 严守 | V1.1 release 改写 触发条件 | 改写方案 (Mavis 自决) |
|---|------------|------------------------|---------------------------|----------------------|
| 1 | supervisor | ✅ 0 改 (R11 baseline) | 触发 3 三洋葱架构升级 | 智囊团 7 席架构 + 四洋葱 (原则 + 权限 + DSL + 智能涌现) |
| 2 | agent | ✅ 0 改 (R127-2 P6-2 +4 专家 + AgentRouter) | 触发 2 9 organ 内部借 OpenCode | SubAgent 5 席 (Oracle/Librarian/Explore/Frontend + 新 1) |
| 3 | council | ✅ 0 改 (R33-4 + R33-4-1 + R33-4-2 加 collaboration/constitution/trace/graph_orchestration) | 触发 3 三洋葱架构升级 | 智囊团 7 席架构 (R18 + 决策 #55 §2.6) |
| 4 | bus | ✅ 0 改 (round15-02 5 层通信总线) | 触发 5 cargo workspace 重构 | 5 层 → 6 层 (新增 L0 跨 agent 协调) |
| 5 | protocol | ✅ 0 改 (R37-1 砍 ProtocolRouter + R20 阶段 2 加 ws_v1 8 帧) | 触发 5 cargo workspace 重构 | 4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 整合 |
| 6 | mcp | ✅ 0 改 (R33-3 + R125-4 加 resources + 4 子文件) | 触发 5 cargo workspace 重构 | McpServer + McpClient + 4 ResourceServer 整合 |
| 7 | tool-registry | ✅ 0 改 (R25 战区 5 + R30 classifier 加 9 类) | 触发 5 cargo workspace 重构 | Tool + 6 enum + 5 axis + 6 mock + Classifier 8 + Token 8 整合 |
| 8 | tool-runtime | ✅ 0 改 (R127-2 P6-2 加 mcp_protocol) | 触发 4 PHL-07 实施扩展 | mcp_protocol 11 + 5 module 整合 |
| 9 | graph | ✅ 0 改 (R89 + R125-13 + R126-3 + R127-2 P9-1 + P6-2 加 mcp_resource/subgraph/channel/state_graph/context_graph/cognition_graph) | 触发 3 三洋葱架构升级 | Channel + ChannelRegistry + StateGraph + ContextGraph 整合 |
| 10 | pipeline | ✅ 0 改 (R122-1~5 + R126-1 + R32-2 加 model_router/provider_registry/tiktoken_counter/role_divider/tool_loop) | 触发 5 cargo workspace 重构 | 8 module 整合 + Provider 模式 |
| 11 | tool-approval | ✅ 0 改 (战役 2-3 5 规则) | ✅ 0 改 (V1.1 release) | — |
| 12 | extension | ✅ 0 改 (R11 baseline 严守) | ✅ 0 改 (V1.1 release) | — |
| 13 | evolution | ✅ 0 改 (R125-7 + R127 P5-1 + R127-2 P8-1 加 poda_cycle/library_autonomy/library_autonomy_loop) | 触发 4 PHL-07 实施扩展 | PODA 8 + library_autonomy 19 + library_autonomy_loop 14 整合 |
| 14 | api | ✅ 0 改 (R120 + R122-1-retry + R123-2 + R30 U1~U11 + R20 阶段 6 鉴权 + WS 8 帧 + observability) | 触发 5 cargo workspace 重构 | 22 LLM + 11 protocol + 4 const 整合 |
| 15 | core | ✅ 0 改 (R11 baseline + 阶段 4 patches-v2) | 触发 4 PHL-07 实施扩展 | 4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard 整合 |
| 16 | memory | ✅ 0 改 (R19 P2 + R22 ST-A2.4 + R30 U9 + R37-2 加 semantic/semantic_persist/user_profile/three_layer/continuity_link/llm_analysis/3 Provider) | 触发 5 cargo workspace 重构 | 9 LOCKED + 3 Provider 整合 |
| 17 | asi | ✅ 0 改 (R22 ST-A3 + R32-1 加 dim_enhance/drift/llm_judge/scheduler/tokenizer) | 触发 4 PHL-07 实施扩展 | 8 calibration + 2 drift + TraceRepository + 3 llm_judge + 26 measure_* + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array 整合 |
| 18 | tools | ✅ 0 改 (R30 U1~U11 + R33-1 加 long_task/classifier/web_fetch/apply_patch/conventions_scanner/grep_ops) | 触发 5 cargo workspace 重构 | 5+7 trait + 6 grep + 7 file_ops + 3 git + 1 code_exec + 1 register + 1 result + 1 web_search + 5 const 整合 |
| 19 | cli | ✅ 0 改 (R116 + R127-2 P9-1 加 commands/output_format) | 触发 5 cargo workspace 重构 | 3 + 2 + 1 + 6 + 5 dispatch + Key 整合 |
| 20 | bench | ✅ 0 改 (V1190 真测 + V2 扩充 swe_bench/agent_bench/self_disable_bench/latency_bench) | ✅ 0 改 (V1.1 release) | — |
| 21 | cognition | ✅ 0 改 (R10 P2 加 BasicCognitiveEngine + 8 trait 默认实现) | 触发 3 三洋葱架构升级 | 4 + ReflectionReport + ReflectionVerdict + 5 score + 8 trait (Cognition/Intuition/Reasoning/MetaCognition/Recall/Consolidation/Forgetting/Learning/Abstraction) 整合 |
| 22 | action | ✅ 0 改 (R11 baseline 严守) | ✅ 0 改 (V1.1 release) | — |
| 23 | life-force | ✅ 0 改 (R22 ST-A2.1 + R22 ST-A2.3 加 reflection_cycle/emergence) | 触发 5 cargo workspace 重构 (life-force 是真 transparent re-export, 合并到 memory) | transparent re-export 到 memory (per R131-4) |
| 24 | constraint | ✅ 0 改 (round7-05 v15 命名修正 5 重 → 4 重 + 权限发放) | 触发 3 三洋葱架构升级 | PhilosophyKeyAccess + HardCodeConstraint + TwelveKeysHardcode + FourGates + FiveGates(deprecated) + PermissionGrant + GrantVerdict + RiskGrant + GateVerdict + VerdictCache + ConstraintEngine 整合 |

**V1.1 release 改写 spec 总结**:
- 24 LOCKED crate 中 16 个 crate 入口签名 改写 (per 触发条件 1-7)
- 8 个 crate 入口签名 0 改 (supervisor, tool-approval, extension, action, bench, mcp 7 个 + 1 改 transparent re-export)
- 0 改原 24 LOCKED 入口签名顺序 (严守)
- 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (严守, 除非满足触发条件)
- 🆕 PHL-07 入口新增 1 个 (25 LOCKED 总数, per 决策 #74 A3)
- 🆕 AsiDispatcher 入口新增 1 个 (pybridge 协调器, per R131-7 O2.3)

### 2.3 方向 3: cargo test 三次 verify (per R131-3 §3 方向 2 + 决策 #62 类比)

#### 2.3.1 任务背景 (per R131-3 §3 方向 2 + R130-1 整合 #5 commit cargo 二次 verify 类比)

- **R131-3 §3 方向 2 任务背景** (per 决策 #62 类比 R130-1 整合 #5 commit cargo 二次 verify): V1.1 release 实施后 cargo test 实战三次 verify = cargo build --workspace (1 次) + cargo test --workspace (2 次) + cargo test --workspace (3 次)
- **R130-1 整合 #5 commit cargo 二次 verify 类比** (per R130-1 8 步 verify + 30+1 bug fix): V1.1 release 实施后 cargo test 实战三次 verify 跟整合 #5 commit cargo 二次 verify 类比, 但 V1.1 release 是 minor release (per 决策 #22 §2.2 semver), 跟 V1.0 release 兼容
- **cargo test 三次 verify 时序图** (per R131-3 §3 方向 2):
  - **第 1 次: cargo build --workspace** (V1.1 release 实施后 1 周, 估 2026-11-05 ~ 11-12, 验证编译通过)
  - **第 2 次: cargo test --workspace** (V1.1 release 实施后 1.5 周, 估 2026-11-12 ~ 11-19, 验证单元测试通过)
  - **第 3 次: cargo test --workspace** (V1.1 release 实施后 2 周, 估 2026-11-19 ~ 11-26, 验证集成测试 + 端到端测试 + 性能测试 + chaos test 通过)
- **8 步 verify 全 PASS 标准** (per R129-3 8 步 verify 类比):
  1. ✅ cargo build --workspace (编译通过)
  2. ✅ cargo test --workspace (单元测试通过)
  3. ✅ cargo test --workspace (集成测试通过)
  4. ✅ cargo clippy --workspace (静态检查通过)
  5. ✅ cargo fmt --workspace (格式检查通过)
  6. ✅ cargo audit (依赖审计通过)
  7. ✅ cargo doc --workspace (文档生成通过)
  8. ✅ 24 LOCKED 入口签名 0 改 verify (24/24 100%)

#### 2.3.2 cargo test 三次 verify 详细 spec

**第 1 次: cargo build --workspace** (V1.1 release 实施后 1 周, 估 2026-11-05 ~ 11-12):
- **目标**: 验证 V1.1 release 实施后 workspace 编译通过
- **内容**:
  - ✅ cargo build --workspace 编译通过 (估 30-60 min)
  - ✅ 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (B1 严守, 除非满足触发条件)
  - ✅ 0 改 R11 baseline 3 值 (A1 严守, 除非新的 baseline 更高)
  - ✅ PHL-07 实施 (per 决策 #74 A3, 14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成)
  - ✅ 24 LOCKED 入口签名 改写 (per 决策 #74 B1, 16/24 crate 入口签名 改写)
  - ✅ AsiDispatcher 入口新增 1 个 (pybridge 协调器, per R131-7 O2.3)
- **风险**:
  - ❌ 编译错误 (e.g. 24 LOCKED 入口签名 改写 引入 breaking change) → 缓解: 阶段性 改写 + 1:1 verify + cargo build 验证
  - ❌ 借鉴源 12 源 0 装 PASS 严守 violation → 缓解: 借脑 ID 索引完成, 0 装"已读真源码" / 0 装"已集成"
- **整合 #6 commit 拍板**: 整合 #6 commit (估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5) 包含 V1.1 release cargo build 验证

**第 2 次: cargo test --workspace** (V1.1 release 实施后 1.5 周, 估 2026-11-12 ~ 11-19):
- **目标**: 验证 V1.1 release 实施后 单元测试通过
- **内容**:
  - ✅ cargo test --workspace 单元测试通过 (估 60-90 min)
  - ✅ 1007 → ~1200 tests pass (V1.1 release 实施 + 200 NEW tests, 跟 R131-7 §2.2 一致)
  - ✅ 0 装 PASS 严守 100% (12 源 0 装 PASS 严守 verify)
  - ✅ 0 改原 24 LOCKED 入口签名顺序 (B1 0 改原 24 LOCKED, 仅改 16/24 触发)
  - ✅ 8 硬墙 0 越界 100%
- **风险**:
  - ❌ 单元测试失败 (e.g. 24 LOCKED 入口签名 改写 引入 testing 失败) → 缓解: 阶段性 改写 + cargo test 验证
  - ❌ 性能瓶颈 (e.g. pybridge 性能瓶颈) → 缓解: PyO3 0.22 异步 awaitable + free-threading GIL release + smart_scopes + type hint union 4 处可深化 (per R131-7 O1.2)
- **整合 #6 commit 拍板**: 整合 #6 commit 后 8 步 verify 7/8 ready

**第 3 次: cargo test --workspace** (V1.1 release 实施后 2 周, 估 2026-11-19 ~ 11-26):
- **目标**: 验证 V1.1 release 实施后 集成测试 + 端到端测试 + 性能测试 + chaos test 通过
- **内容**:
  - ✅ cargo test --workspace 集成测试通过 (估 90-120 min)
  - ✅ 集成测试 + 端到端测试 + 性能测试 + chaos test 全通过
  - ✅ pybridge 886/886 tests 严守 (跟 V1.0 release 一致, per R131-7 §2.3)
  - ✅ AsiDispatcher 协调器集成测试通过
  - ✅ ASI Stage 9 终极自治 集成测试通过 (per R133-2)
  - ✅ Cargo.lock 分模块 lockfile 测试通过 (per R131-4)
- **风险**:
  - ❌ 集成测试失败 (e.g. 跨 crate 集成时 24 LOCKED 入口签名 改写 引入 integration 失败) → 缓解: 阶段性 改写 + integration test 验证
  - ❌ 端到端测试失败 (e.g. ASI Stage 9 4 维度 H/L/G/P 集成失败) → 缓解: AsiDispatcher 统一协调器 + Stage 8 12 步 cycle spec
- **整合 #7 commit 拍板**: 整合 #7 commit (估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5) 包含 V1.1 release cargo test 三次 verify

#### 2.3.3 cargo test 三次 verify 8 步 verify 全 PASS 标准 (per R129-3 8 步 verify + R131-3 §3 方向 2)

| 步 | verify | V1.0 release 实测 (R130-1 整合 #5 commit cargo 二次 verify) | V1.1 release 期望 (per R131-3 §3 方向 2 + 决策 #74 B1/B2) |
|:--:|--------|----------------------------------------|----------------------------------------|
| 1 | cargo build --workspace | ❌ 24 hard errors (apeireth-central 23 + apeireth-naming-v05 1, per R129-26) | ✅ 全通过 (0 改原 24 LOCKED 入口签名 + 16/24 入口签名 改写) |
| 2 | cargo test --workspace | ❌ 1 FAILED test (test_release_version_is_1_1_0, per R129-26) | ✅ 全通过 (~1200 tests pass, 跟 R131-7 §2.2 一致) |
| 3 | cargo test --workspace (集成) | ⚠️ 部分 PASS (5 hard errors in apeireth-graph, per R129-26) | ✅ 全通过 (集成测试 + 端到端测试 + 性能测试 + chaos test) |
| 4 | cargo clippy --workspace | (估 PASS) | ✅ 全通过 (clippy 0 warning) |
| 5 | cargo fmt --workspace | (估 PASS) | ✅ 全通过 (fmt 0 change) |
| 6 | cargo audit | (估 PASS) | ✅ 全通过 (0 漏洞) |
| 7 | cargo doc --workspace | (估 PASS) | ✅ 全通过 (rustdoc 0 warning) |
| 8 | 24 LOCKED 入口签名 0 改 verify | ✅ 24/24 PASS (per R131-5 01:35 done) | ✅ 24/24 PASS (V1.1 release 0 改原 24 LOCKED 入口签名顺序 + 16/24 入口签名 改写 触发) |

### 2.4 方向 4: 12 源 0 装严守 二次 verify (per R131-3 §3 方向 3 + R133-1 + R131-6)

#### 2.4.1 任务背景 (per 决策 #33 §2.3 C2 + R131-3 §3 方向 3 + R133-1 + R131-6)

- **决策 #33 §2.3 C2 0 装 PASS 严守**: 0 装 PASS 严守 100% (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 12/12, per R133-1 + R131-6)
- **R131-3 §3 方向 3 任务背景**: 借鉴源 12 源 0 装严守二次 verify (per R133-1 实施 spec, 估 1 周)
- **R133-1 借鉴源 12 源 实施 spec** (per R133-1 §1 + §2):
  - 8 真 cloned: clap 4.6.6 (3.50MB / 631 files) + hyper 0.1.20 (0.54MB / 58 files) + servers 76d64c8 (1.40MB / 145 files) + PyO3 0.29.2 (5.69MB / 811 files) + kani 0.67.0 (5.46MB / 3224 files) + langgraph d56666f (13.29MB / 670 files) + superpowers 6.2.0 (1.52MB / 180 files) + Guardrails (18.19MB / 2045 files) = 总 49.59MB / 7,764 files
  - 2 借鉴 ID 索引完成: LiteLLM (0 cloned, 19/19 tests + 562 行新 src) + opencode (0 cloned, 35/35 tests + 3 新模块)
  - 1 永久跳过: OpenCog AGPL-3.0 (0 cloned, 永久跳过)
  - 🆕 1 借脑 ID 索引完成: OpenCog 家族 6 子源 (AtomSpace + CogPrime + moses + pln + relex + cogutil, 0 cloned 借脑, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
  - = **12/12 借鉴 ID 完整, 0 借脑 0 装 100% 严守**

#### 2.4.2 V1.1 release 12 源 0 装严守 二次 verify 方案 (per 决策 #33 §2.3 C2 + 决策 #73 §2.2 + 决策 #74 B1 改写 + R133-1)

**V1.1 release 12 源 0 装严守二次 verify 100% 方案** (per R133-1 §2.2):

| 借鉴源 | 1.0 release 状态 (per R131-6 + R133-1) | V1.1 release 沿用 | 0 装 PASS 严守 |
|--------|--------------------------------------|-------------------|----------------|
| clap 4.6.6 | ✅ 3.50MB / 631 files / 17:30 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| hyper 0.1.20 | ✅ 0.54MB / 58 files / 17:29 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| servers 76d64c8 | ✅ 1.40MB / 145 files / 16:51 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| PyO3 0.29.2 | ✅ 5.69MB / 811 files / 16:53 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| kani 0.67.0 | ✅ 5.46MB / 3224 files / 17:35 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| langgraph d56666f | ✅ 13.29MB / 670 files / 16:31 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| superpowers 6.2.0 | ✅ 1.52MB / 180 files / 17:33 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| Guardrails | ✅ 18.19MB / 2045 files / 17:48 cloned | ✅ 沿用 0 必重借 | ✅ 0 装"已借鉴" |
| LiteLLM 公开 1:1 翻译 | ✅ 0 cloned + 19/19 tests + 562 行新 src | ✅ 沿用 0 必重借 | ✅ 0 装"已读真源码" |
| opencode 改借鉴已 cloned | ✅ 0 cloned + 35/35 tests + 3 新模块 | ✅ 沿用 0 必重借 | ✅ 0 装"已对接 opencode 私有 channel" |
| OpenCog/opencog AGPL-3.0 | ❌ 0 cloned 永久跳过 | ❌ **0 重借**, 主仓 0 触碰 (per Cargo.toml `borrow_skipped` 永久明示) | ❌ 0 装"已借鉴" / 0 装"已集成" |
| 🆕 OpenCog 家族 6 子源 (借脑) | ⏳ R130-6 借脑 ID 索引完成 | 🆕 V1.1 minor **借脑调研沉淀** (per 决策 #55 §2.6 + 决策 #73 §2.2 + 决策 #74 B1 V1.1 release Mavis 自决改) | ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" |

**总 12/12 借鉴源 V1.1 release 0 装 PASS 严守二次 verify 100% 方案**:
- ✅ 8 真 cloned 沿用 0 必重借 (mtime 早于整合 #4 commit 19:41, 0 重跑 0 重 commit, 0 必重借, per R129-28 §1.1 实地 verify 100%)
- ⏳ 0 限流 (P6-1/2/3 全 done, 0 借鉴处于限流, V1.1 release 0 必重借)
- ❌ 1 永久跳过 (OpenCog AGPL-3.0 0 集成 0 装, V1.1 release 0 必重借, 主仓 0 触碰, per 决策 #33 §2.2 + 决策 #22 §4 风险表)
- 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- **总 12/12 借鉴 ID 完整, 0 借脑 0 装 100% 严守**

#### 2.4.3 V1.1 release 借脑调研沉淀 6 子源 (per 决策 #55 §2.6 + 决策 #73 §2.2 + 决策 #74 B1 + R133-1 §2.3)

**6 子源借脑 ROI 梯度 + V1.1 release 调研深度** (per R130-6 §3.2 + R131-2 §2.2 + 决策 #55 §2.6 + 用户记忆 #5 高信息密度 = 拟人化+拟物化):

| 借脑 ROI | 子源 | V1.1 release 借脑调研深度 | 文档沉淀目标 | 0 装 PASS 严守 |
|----------|------|--------------------------|------------|----------------|
| 🟢 **高 (Top 2)** | `R130-6-BORROW-opencog/atomspace-2026Q1-2026-08-11` (4.3.0, AGPL-3.0) | **深度调研** (AtomSpace hypergraph + Atomese 三元素 Atom/Node/Link + ECAN 重要度扩散 + StorageNode 持久化 + Unified Rule Engine URE + 5 阶段 forward/backward chainer) — 对应 apeireth-cognition 模块演化 | `reports/borrow-index-opencog-atomspace-r130-6.md` (~30-50 KB) | ✅ 0 装"已读 atomspace 真源码" / 0 装"已集成 AtomSpace API" / 0 装"已 fork atomspace" |
| 🟢 **高 (Top 2)** | `R130-6-BORROW-CogPrime-Goertzel-2024-2026-08-11` (Ben Goertzel 著作, 无 code 公开论文) | **深度调研** (CogPrime AGI 操作系统设计 + AtomSpace + ECAN + PLN + MOSES + OpenPsi 多子系统集成模式) — 对应 apeireth-cognition 整体架构 | `reports/borrow-index-cogprime-r130-6.md` (~30-50 KB) | ✅ 0 装"已实现 CogPrime" / 0 装"已完整读 CogPrime" (仅文档调研) |
| 🟡 **中** | `R130-6-BORROW-opencog/moses-2026Q1-2026-08-11` (AGPL-3.0) | **中度调研** (监督学习 + 决策树森林管理 + Atomese graphlets 集成 + 演化学习 MOSES) — 对应 apeireth-evolution 模块, per R124-2 §7.1 B-016 aGLM PODA cycle 借鉴 | `reports/borrow-index-opencog-moses-r130-6.md` (~10-20 KB) | ✅ 0 装"已读 moses 真源码" / 0 装"已 fork moses" |
| 🔴 **低** | `R130-6-BORROW-opencog/cogutil-2026Q1-2026-08-11` (AGPL-3.0, C++ utils) | **浅度调研** (C++ 工具集架构, 仅架构参考, 不集成 code) | `reports/borrow-index-opencog-auxiliary-r130-6.md` (~5-10 KB) | ✅ 0 装"已读 cogutil 真源码" / 0 装"已 fork cogutil" |
| 🔴 **低** | `R130-6-BORROW-opencog/pln-2026Q1-2026-08-11` (AGPL-3.0, **官方 deprecated**) | **浅度调研** (PLN 概率逻辑网络设计, 仅历史参考, 0 实施价值, per 2026-02 opencog/sensory README "PLN (also unsupported & deprecated)") | `reports/borrow-index-opencog-auxiliary-r130-6.md` (~5-10 KB) | ✅ 0 装"已读 pln 真源码" / 0 装"已集成 PLN" |
| 🔴 **低** | `R130-6-BORROW-opencog/relex-2026Q1-2026-08-11` (AGPL-3.0, **官方 deprecated**) | **浅度调研** (RelEx 关系提取 NLP 模式, 仅历史参考, 0 实施价值, per opencog wiki "obsolete") | `reports/borrow-index-opencog-auxiliary-r130-6.md` (~5-10 KB) | ✅ 0 装"已读 relex 真源码" / 0 装"已集成 relex" |

**借脑调研总文档沉淀** (~95-155 KB, 6 文档, 借脑 ID 索引完成):
- 🟢 AtomSpace 深度 (~30-50 KB) + 🟢 CogPrime 深度 (~30-50 KB) = ~60-100 KB
- 🟡 MOSES 中度 (~10-20 KB)
- 🔴 cogutil + pln + relex 浅度 (~15-30 KB, 3 子源合 1-3 文档)

#### 2.4.4 0 装 PASS 严守 6 维度 verify (per 决策 #33 §2.3 C2 + R129-7 §5.1 + R129-28 §3.2 + R131-2 §3.2.3)

| 维度 | V1.0 release 严守 verify | V1.1 release 严守 verify |
|------|-------------------------|--------------------------|
| **借鉴源码 0 cloned = 0 实施** | ✅ 严守 (LiteLLM 0 cloned → 公开设计 1:1 翻译 0 装"已读真源码", opencode 0 cloned → 改借鉴已 cloned 0 装"已对接 opencode 私有 channel", OpenCog family 0 cloned → 借脑 ID 索引完成 0 装"已读真源码") | ✅ 严守 (V1.1 release 沿用, 0 装 PASS 严守 100%) |
| **借鉴源码 ✅ cloned = 真实施** | ✅ 严守 (8 真 cloned mtime 早于整合 #4 commit 19:41, 真 src 改动 + tests pass) | ✅ 严守 (V1.1 release 沿用 0 必重借) |
| **借鉴源码 ❌ 永久失败 = 0 假装"已借鉴"** | ✅ 严守 (OpenCog AGPL-3.0 0 集成 0 装, 借鉴 ID 索引 0 假装"已对接") | ✅ 严守 (V1.1 release 沿用) |
| **借鉴 ID 索引完成** (借脑模式) | ✅ 严守 (R130-6 借脑 ID 索引完成, 0 借脑 0 装, 0 装"已读真源码") | ✅ 严守 (V1.1 release 借脑调研沉淀 0 装"已读真源码") |
| **0 装"已集成 OpenCog AtomSpace"** | ✅ 严守 (主仓 0 触碰 OpenCog code, 0 装 API 对接) | ✅ 严守 (V1.1 release 沿用) |
| **0 装"已 fork OpenCog"** | ✅ 严守 (1.0 release 前 0 主仓 fork, 1.0 release 后独立 fork 决策 = 主人主动问) | ✅ 严守 (V1.1 release 沿用, 1.0 release 后独立 fork 决策 = 主人主动问) |

**0 装 PASS 严守 6 维度 100% PASS** (per R129-7 §5.1 + R129-28 §3.2 + R131-2 §3.2.3 + R133-1 01:25 实地 verify 100% 严守 + R131-6 01:30 实地 verify 100% 严守)。

### 2.5 方向 5: pybridge 集成优化 + ASI Stage 9 终极自治 (per R131-7 + R133-2 + 决策 #74 B1)

#### 2.5.1 任务背景 (per R131-7 + R133-2 + 决策 #74 B1 V1.1 release Mavis 自决改)

- **R131-7 pybridge 集成优化 9 方向** (per R131-7 01:30 done):
  - **O1 PyO3 928 借鉴深度** (16 处 1:1 翻译 + 4 处可深化, per R131-7 §2.1):
    - 16 处已 1:1 翻译 (Bound API + Python::attach + kwargs + eval + GIL release + exception + class + free-threading + performance + 5 类错误 + 5 kind 性能 + 7 重门 + Bound 生命周期, 0 装 PASS 严守 100%)
    - 4 处可深化方向 (V1.1 release 实施, per 决策 #74 B1 V1.1 release Mavis 自决改):
      - 🆕 **O1.2.1 PyO3 0.22 异步 awaitable** (PyO3 0.22+ `pyo3-async-runtimes` 异步 awaitable, Rust async/await ↔ Python asyncio 互通, 收益: Stage 8 12 步 cycle 100ms/cycle 优化为 12 步并行 ~30ms, 风险: 0 装 PASS 严守)
      - 🆕 **O1.2.2 free-threading GIL release 实际未测** (PyO3 3.13+ free-threading, Python::allow_threads 包裹, 收益: 跨语言 Bridge 调用实际延迟 470μs → 200μs, 风险: 0 装 PASS 严守)
      - 🆕 **O1.2.3 PyO3 smart_scopes** (PyO3 0.21+ 新特性, 一次 attach 多次操作, 减少 GIL acquire/release 开销, 收益: Stage 8 12 步 cycle GIL acquire 从 12 次 → 1 次)
      - 🆕 **O1.2.4 PyO3 0.24 type hint union** (PyO3 0.24+ `PyAny` union type hint, 支持 int/float/bool/list/dict 异构 args, 收益: ASI Python 阶段 1-8 实际调用可以传异构 args)
  - **O2 ASI Python 8 阶段集成** (8 阶段 63 个 1:1 映射, per R131-7 §2.2):
    - Stage 1-3 (7+22+3 = 32 1:1 映射, R128 P10-1/2/3 done)
    - Stage 4 (D1 工具 + D2 反思 + D3 记忆 + D4 决策 = 4 1:1 映射, R129-4 done)
    - Stage 5 (G1 资源 + G2 权限 + G3 形式化 + G4 演进 = 4 1:1 映射, R129-5 done)
    - Stage 6 (K1 错误 + K2 性能 + K3 6+1 重门 + K4 5 维度 = 4 1:1 映射, R129-6 done)
    - Stage 7 (I1~I7 7 跨模块 = 7 1:1 映射, R129-18 done)
    - Stage 8 spec (C1 12 步 cycle + 5 跨 crate 集成, R130-2 spec done, V1.1 release 实施)
    - Stage 9 spec (H 自治 + L 长程 + G 成长 + P 平台化 4 维度, R133-2 spec done, V1.1 release 实施)
    - = 总 8 阶段 63 个 1:1 映射 (Stage 1-3 32 + Stage 4-7 19 + Stage 8 12)
    - 🆕 O2.3 缺乏统一 dispatcher 协调器 (V1.1 release 加 `AsiDispatcher`, per R131-7 §2.2.3, 收益: Stage 8 12 步 cycle 有统一入口)
  - **O3 886/886 pybridge tests** (实际 1007 累加, per R131-7 §2.3):
    - 单元测试 (440 tests) + 集成测试 (452 tests) + 端到端测试 (63 tests) + 性能测试 (56 tests) = 1011 累加, 实际 886 pass (R129-4 4 test files 60 tests 失败, 跟 R129-5/6/18 0 关系)
    - 🆕 改进方向: V1.1 release 加 Stage 8 12 步 cycle E2E + 1000 samples benchmark + chaos test
  - **O4 跨语言调用性能** (per R131-7 §2.4):
    - R129-6 K2 实测: 5 kind p95 = Bridge 470μs < 500μs / Eval 1000μs / Import 5000μs / Convert 100μs / Call 800μs, over_rate=0.00, throughput=100/s, cfg-gated 守门
    - 1000 cycle 跑通 + 10000 cycle benchmark 还没跑, Stage 8 实施时跑
  - **O5 V0.5 30 维公式** (per 决策 #33 §2.3 B3 严守 100%, 0 触碰 integration_r_measure.rs)
  - **O6 6 重守门 v7 集成** (per R131-7 §2.6, G2 PermissionLayer 6 重 1:1 跟 B4 严守, K3 SecurityGate 7 重, I4 D4+G2 = 5 policy × 6 layer = 30 绑定, I6 G2+K3 = 6 layer × 7 gate = 42 绑定)
  - **O7 8 哲学锚集成** (per 决策 #33 §2.3 B5 严守 100%)
  - **O8 V1.1 release ASI Stage 9 长程 AI 成长** (per R131-7 + R133-2, V1.1 release Mavis 自决改, 4 维度 H 自治 + L 长程 + G 成长 + P 平台化)
  - **O9 pybridge 与 cargo workspace 集成** (per R131-7, 1 真相源 + 5 共享 + 永远循环)

- **R133-2 ASI Stage 9 长程 AI 成长 实施 spec** (per R133-2 01:30 done):
  - 4 维度: H 自治 + L 长程 + G 成长 + P 平台化
  - H 自治 (Autonomy): 4 NEW src (在线自检 + 自动修复 + rollback + 学习, per R130-2 §1 Stage 9 路线图)
  - L 长程 (Long-term): 4 NEW src (跨会话记忆 + 经验积累 + 知识累积, per chidori journal 9 字段 replay)
  - G 成长 (Growth): 4 NEW src (能力升级 + 知识扩展 + 模式识别, per OpenCog AtomSpace 借脑 1:1 公开模式)
  - P 平台化 (Platform): 4 NEW src (多 agent 协同 + 知识共享 + 任务分配 + 冲突解决, per OpenCog CogPrime 借脑 1:1 公开模式)
  - 估 +200KB NEW src + 200 NEW tests + 4 NEW examples
  - 借脑 OpenCog CogPrime (AtomSpace + PLN + MOSES + OpenPsi, 借脑 1:1 公开模式 0 借具体源码, AGPL-3.0 license 0 借)

#### 2.5.2 V1.1 release pybridge 集成优化 实施 spec (per R131-7 + R133-2)

**V1.1 release pybridge 集成优化 实施 spec** (per R131-7 9 方向 + R133-2 4 维度, 估 2 周, 2026-11-12 ~ 11-26):

**V1.1 release 实施内容**:
1. **🆕 PyO3 0.22 异步 awaitable 实施** (per R131-7 O1.2.1):
   - 实施 `pyo3-async-runtimes` crate
   - 实施 pymethod + tokio runtime
   - Stage 8 12 步 cycle 100ms → 30ms 优化
2. **🆕 free-threading GIL release 实际未测 实施** (per R131-7 O1.2.2):
   - 实施 Python::allow_threads 包裹 R129-6 K2 PerfKind 5 类实测
   - 跨语言 Bridge 调用实际延迟 470μs → 200μs
3. **🆕 PyO3 smart_scopes 实施** (per R131-7 O1.2.3):
   - 实施 smart_scopes 一次 attach 多次操作
   - Stage 8 12 步 cycle GIL acquire 从 12 次 → 1 次
4. **🆕 PyO3 0.24 type hint union 实施** (per R131-7 O1.2.4):
   - 实施 `PyAny` union type hint
   - 桥接 int/float/bool/list/dict 异构 args
5. **🆕 AsiDispatcher 统一协调器 实施** (per R131-7 O2.3 + 决策 #74 B1):
   - 实施 `AsiDispatcher::run_stage_n(input, n: u8) -> StageOutput`
   - 实施 `AsiDispatcher::run_cycle(input) -> CycleReport` (12 步 cycle 统一入口)
   - 实施 `AsiDispatcher::bootstrap(7 ASI 模块名) -> DispatcherHandle`
6. **🆕 Stage 8 C1 12 步 cycle 实施** (per R130-2 §2.4 + R131-7 + 决策 #74 B1):
   - 12 步 cycle 集成 4 ASI Python 关键模块 (D1 工具 + G1 资源 + D1 工具 + K1 错误 + D2 反思 + D3 记忆 + G3 形式化 + D4 决策 + G2 权限 + K3 跨语言 + K2 性能 + K4 健康)
   - 5 跨 crate 集成 (apeireth-asi + apeireth-pybridge + apeireth-evolution + apeireth-cognition + apeireth-formal)
7. **🆕 Stage 9 4 维度 H/L/G/P 实施** (per R133-2 + 决策 #74 B1):
   - H 自治: 在线自检 + 自动修复 + rollback + 学习 (4 NEW src, 估 +50KB)
   - L 长程: 跨会话记忆 + 经验积累 + 知识累积 (4 NEW src, 估 +50KB)
   - G 成长: 能力升级 + 知识扩展 + 模式识别 (4 NEW src, 估 +50KB)
   - P 平台化: 多 agent 协同 + 知识共享 + 任务分配 + 冲突解决 (4 NEW src, 估 +50KB)
   - 总 +200KB NEW src + 200 NEW tests + 4 NEW examples
8. **🆕 借脑 OpenCog CogPrime** (per R133-2 + 决策 #73 §2.2 + 决策 #74 B1):
   - AtomSpace hypergraph + Atomese 三元素 Atom/Node/Link + ECAN 重要度扩散 1:1 翻译公开模式
   - CogPrime AGI 操作系统设计 1:1 翻译公开模式 (无 code 公开论文)
   - MOSES 演化学习 1:1 翻译公开模式
   - PLN 概率逻辑网络 1:1 翻译公开模式 (官方 deprecated, 仅历史参考)
   - 0 借具体源码, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"

**V1.1 release pybridge 集成优化 入口签名 改写** (per 决策 #74 B1 Mavis 自决改):
- 0 改原 29 mod 入口签名 (Stage 1-7 累计, B1 0 改原 24 LOCKED 入口签名顺序 严守)
- 🆕 0 改 `bridge::*` + `asi_modules::*` + `r11_compat::*` + `stage3_*::*` + `tool_self_loop::*` + `error_guardianship::*` + `perf_guardianship::*` + `security_guardianship::*` + `health_guardianship::*` + `stage7_i*::*` + `python_bindings::*` (cfg-gated) (B1 0 改)
- 🆕 改写 `stage8_c1_cycle::*` (Stage 8 12 步 cycle 实施, NEW mod) + `stage8_cross_crate::*` (跨 crate 集成, NEW mod) + `stage8_perf::*` (性能优化, NEW mod) + `stage8_full::*` (完整 cycle, NEW mod)
- 🆕 改写 `stage9_autonomy::*` (H 自治, NEW mod) + `stage9_long_term::*` (L 长程, NEW mod) + `stage9_growth::*` (G 成长, NEW mod) + `stage9_platform::*` (P 平台化, NEW mod)
- 🆕 改写 `asi_dispatcher::*` (AsiDispatcher 协调器, NEW mod) + `pyo3_async::*` (PyO3 0.22 异步, NEW mod) + `pyo3_freethreading::*` (free-threading, NEW mod) + `pyo3_smart_scopes::*` (smart_scopes, NEW mod) + `pyo3_type_hint::*` (type hint union, NEW mod)
- 总 NEW mod: 4 (Stage 8) + 4 (Stage 9) + 1 (AsiDispatcher) + 4 (PyO3 深化) = **13 NEW mod**
- 总 mod (Stage 1-9 + dispatcher + pyo3 深化): 29 (Stage 1-7) + 13 (NEW) = **42 mod**

**V1.1 release pybridge 集成优化 关键诚实标**:
- ✅ 0 改原 24 LOCKED 入口签名 (B1 严守)
- ✅ 0 装 PASS 严守 100% (5 借脑 0 装: PyO3 928 + superpowers 234 + langgraph 829 + chidori + servers 175)
- ✅ 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 8 硬墙全严守)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2, 借脑 OpenCog 1:1 翻译公开模式)
- ✅ 0 改 8 哲学锚 (B5 严守)
- ✅ 0 改 V0.5 30 维 (B3 严守)
- ✅ 0 改 6 重守门 v7 (B4 严守)

### 2.6 方向 6: cargo workspace 重构 (per R131-4 + 决策 #74 B1 V1.1 release Mavis 自决改)

#### 2.6.1 任务背景 (per R131-4 + 决策 #74 B1 V1.1 release Mavis 自决改)

- **R131-4 cargo workspace 结构优化 7 方向** (per R131-4 01:40 done, 87 crate 分布 + 24 LOCKED + Cargo.toml borrow + Cargo.lock 265KB + 三洋葱 + 9 organ + 借鉴 12 源):
  - **87 crate 分布合理性** (per R131-4 §2.1):
    - 总 workspace members: 87 个 (per Cargo.toml `members` 段清点 2026-08-11 01:35, 不含 R20 阶段 4 估补 + V1302-V1307 fix 6 个 = 实际 87)
    - 24 LOCKED crate (per `docs/omnibus/24-locked-crates.md` 完整名单 R125 B1 落实, 12 主人已知 + 12 Mavis 自主)
    - 63 非 LOCKED crate 分类 (核心抽象层 6 + 哲学/能力层 5 + 智囊团/工具层 4 + 兼容组件层 12 + 形式化/治理层 5 + 借鉴源 1:1 翻译层 5 + 借鉴模式层 7 + ASI/认知层 2 + 升级/通信层 5 + 持久化/工具层 4 + 任务/工作流层 4 + 鉴权/凭据层 4 + 监控/告警层 3 + 安全/沙箱层 3 + 工具扩展层 4 + 第三方 SDK 层 4 + 集成测试层 4 + R20 阶段 1 估补 5 + R20 阶段 4 估补 5 + R20 阶段 5 估补 1 + R20 阶段 6 估补 10 + R21 估补 5 + R23 P3 透明登记 1 + V1302/1304/1305/1306 fix 7 + R127 P5-2 估补 1 + R20 阶段 6 估补 1 + R20 阶段 6 估补 1 + Blueprint 估补 1 + R17 战役 1)
  - **24 LOCKED 入口签名一致性** (per R131-4 §2.2, 24/24 入口签名 0 改 100%)
  - **Cargo.toml borrow 段** (per R131-4 §2.3, 整合 #5.2 commit 时 update 17:44 → 22:50, V1.1 release 拆 4 子段)
  - **Cargo.lock 265KB** (per R131-4 §2.4, 合理范围, V1.1 release 可分模块 lockfile, V2.0 release 可重构)
  - **三洋葱架构** (per R131-4 §2.5, V1.0 release 严守 0 改, V1.1 release 三洋葱 → 四洋葱 升级, V2.0 release 四洋葱 → 五洋葱 升级)
  - **9 organ 分布** (per R131-4 §2.6, 9 organ 跨 8 LOCKED crate, body/brain/ear/eye/hand/heart/memory/mind/voice)
  - **借鉴源 12 源** (per R131-4 §2.7, 8 真 cloned 49.6MB/7,764 files + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 1 借脑 ID 索引完成 R130-6 6 子源)

#### 2.6.2 V1.1 release cargo workspace 重构 方案 (per R131-4 + 决策 #74 B1)

**V1.1 release cargo workspace 重构 8 方向** (per R131-4 + 决策 #74 B1 V1.1 release Mavis 自决改):

| # | 重构方向 | 描述 | V1.1 release 实施 |
|---|---------|------|------------------|
| 1 | **3 真 transparent re-export 合并** | life-force → memory + value → motivation + consciousness → perception (per R37-2) | ✅ 合并 3 个真 transparent re-export crate 到目标 crate |
| 2 | **借鉴模式 12 个统一** | plugin / state / cache / credentials / oauth / update / tracing / metrics / keyring / machine-id / sandbox / task | ✅ 统一为 1 个 `apeireth-borrowed-patterns` 库 |
| 3 | **5 估补 R20 阶段 1 合并** | mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead → 合并到 `apeireth-mcp` 现有 crate | ✅ 合并 5 估补到 `apeireth-mcp` |
| 4 | **Cargo.toml borrow 段拆 4 子段** | cloned_real + translated_public + translated_modified + skipped_license + brainonly = 5 子段 | ✅ 拆 5 子段 (per R131-6 §3.1) |
| 5 | **Cargo.lock 分模块 lockfile** | Cargo 1.78+ feature, 分模块 lockfile | ✅ 分模块 lockfile |
| 6 | **87 crate 优化** | 87 → 60-70 crate 优化 (3 transparent + 12 借鉴模式 = 15 减 + 估补 5 减) | ✅ 优化 87 → 70-75 crate |
| 7 | **三洋葱架构升级** | 三洋葱 → 四洋葱 [+ 智能涌现 emergence], per R133-3 | ✅ 三洋葱 → 四洋葱 |
| 8 | **9 organ 对应关系** | 9 organ 跨 8 LOCKED crate, V1.1 release 9 organ 对应关系 1:1 翻译 | ✅ 9 organ 1:1 对应 |

**V1.1 release cargo workspace 重构 关键诚实标**:
- ✅ 0 改原 24 LOCKED 入口签名 (B1 0 改原 24 LOCKED 入口签名顺序 严守)
- ✅ 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (B1 0 改原 24 LOCKED crate mtime 严守, 除非满足触发条件)
- ✅ 0 装 PASS 严守 100% (12 源 0 装 PASS 严守 verify)
- ✅ 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 8 硬墙全严守)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2)
- ✅ 0 改 8 哲学锚 (B5 严守)
- ✅ 0 改 V0.5 30 维 (B3 严守)
- ✅ 0 改 6 重守门 v7 (B4 严守)

### 2.7 方向 7: V0.5 30 维 严守 (per 决策 #33 §2.3 B3 + 决策 #74 §1)

#### 2.7.1 任务背景 (per 决策 #33 §2.3 B3 + 决策 #74 §1 + R131-7 O5)

- **决策 #33 §2.3 B3 V0.5 30 维 严守** (per 决策 #33 §2.3 B3, 哲学公式 严守 0 改):
  - V0.5 30 维 = 25 维 (B3 V0.5) + 5 维 (新增长程) = 30 维
  - 9 organ × 5 维 = 45 维? 不, V0.5 30 维 = 9 organ 5 维 × 6 类 pluginType = 30 维 (per 决策 #33 §2.3 B3 + 决策 #55 §2.6)
  - 编译期 hardcode enum 严守 (per 决策 #22 §1.2 + 决策 #33 §2.3 B3)
- **决策 #74 §1 B3 严守**: V0.5 30 维 V1.0 release 严守 + V1.1 release 严守 (per 决策 #74 §1, 哲学公式不松绑) + V2.0 release 可重评
- **R131-7 O5 V0.5 30 维公式** (per R131-7 §2.5, 0 触碰 integration_r_measure.rs, Stage 4-7 0 涉及 V0.5 公式, Stage 8 跨 crate 集成 30 维测度 1:1 翻译模式 0 改公式)

#### 2.7.2 V1.1 release V0.5 30 维 严守 集成方案 (per 决策 #33 §2.3 B3 + 决策 #74 §1 + 决策 #74 A3)

**V1.1 release V0.5 30 维 严守 集成方案** (per 决策 #33 §2.3 B3 + 决策 #74 §1 + 决策 #74 A3 PHL-07 实施 + 8 哲学锚 + 6 重守门 v7):

**V1.1 release 实施内容**:
1. **🆕 PHL-07 14 维主对话锚** (per 决策 #74 A3 + 决策 #22 §1.1-1.2):
   - 14 维主对话锚 (per 用户记忆 #3 "主对话是核心" + 用户记忆 #5 拟人化, 9 organ 拟人化 + 5 维主对话深化)
   - PHL-07 14 维 是 V0.5 30 维子集 (深化) 还是 NEW 维度 (扩展)? 待 R131-2 PHL-07 实施调研 (per R131-3 §2.1.3)
2. **🆕 PHL-07 14 维跟 8 哲学锚 1:1 集成** (per 决策 #33 §2.3 B5 + 决策 #74 A3):
   - S-1 服务 ASI 北极星 → 14 维主对话锚 #1 (北极星维度)
   - S-2 实事求是 → 14 维主对话锚 #2 (实事求是维度)
   - S-3 质量工程化 → 14 维主对话锚 #3 (质量工程化维度)
   - O-1 安全优先 → 14 维主对话锚 #4 (安全优先维度)
   - O-2 走在前人经验上 → 14 维主对话锚 #5 (走在前人经验上维度)
   - O-3 干到底 → 14 维主对话锚 #6 (干到底维度)
   - O-4 任何人都能接手 → 14 维主对话锚 #7 (任何人都能接手维度)
   - O-5 不假装 → 14 维主对话锚 #8 (不假装维度)
   - 9 organ 拟人化维度 (#9-#14) 跟 9 organ 1:1 映射
3. **🆕 PHL-07 14 维跟 6 重守门 v7 1:1 集成** (per 决策 #33 §2.3 B4 + 决策 #74 A3):
   - 6 重守门 v7 (L0 真实人类批准 + L1-L5 5 重) 跟 14 维主对话锚 1:1 集成
4. **🆕 PHL-07 14 维跟 14 键 1:1 集成** (per 决策 #33 §2.3 A3 + 决策 #74 A3):
   - 12 键 + PHL-07 = 13 键 (V1.0 spec-only) → 14 键 (V1.1 release 实施, per 决策 #74 A3 加 1 键)
5. **0 改 V0.5 30 维公式** (B3 严守, 0 触碰 integration_r_measure.rs, 编译期 hardcode enum 严守)

**V1.1 release V0.5 30 维 严守 关键诚实标**:
- ✅ 0 改 V0.5 30 维公式 (B3 严守)
- ✅ 0 改 integration_r_measure.rs (per 决策 #22 §1.2)
- ✅ 0 改 9 organ × 5 维 = 30 维 (per 决策 #33 §2.3 B3)
- ✅ 编译期 hardcode enum 严守
- ✅ PHL-07 14 维 是 V0.5 30 维子集 (深化) (per 决策 #74 A3)

### 2.8 方向 8: 6 重守门 v7 严守 + 8 哲学锚严守 + PHL-07 实施 (per 决策 #33 §2.3 B4/B5 + 决策 #74 §1 + 决策 #74 A3)

#### 2.8.1 6 重守门 v7 严守 (per 决策 #33 §2.3 B4 + 决策 #74 §1)

- **决策 #33 §2.3 B4 6 重守门 v7 严守** (per 决策 #33 §2.3 B4, 哲学守门 严守 0 改):
  - 6 重守门 v7 = 4 重 + 权限 + Colang DSL 守门 (per R125 B6 三洋葱升级, 整合 #4 commit 6 重 v6 → 6 重 v7 升级扩展)
  - 6 重守门 v7 跨权限洋葱 (L0-L5 6 层, L0 = 真实人类批准)
  - 编译期 hardcode 严守 (per 决策 #22 §1.2 + 决策 #33 §2.3 B4)
- **决策 #74 §1 B4 严守**: 6 重守门 v7 V1.0 release 严守 + V1.1 release 严守 (per 决策 #74 §1, 哲学守门不松绑) + V2.0 release 可重评
- **R131-7 O6 6 重守门 v7 集成** (per R131-7 §2.6, G2 PermissionLayer 6 重 1:1 跟 B4 严守, K3 SecurityGate 7 重 (G1-G6 v7 + G7 跨语言 K3 新增), I4 D4+G2 = 5 policy × 6 layer = 30 绑定, I6 G2+K3 = 6 layer × 7 gate = 42 绑定, 严守 V7BaselineCheck::v7_baseline_intact() 编译期 hardcode)

#### 2.8.2 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

- **决策 #33 §2.3 B5 8 哲学锚严守** (per 决策 #33 §2.3 B5, 哲学 严守 0 改):
  - 8 哲学锚 (per `docs/conventions/09-anchor.md`):
    - **S-1 服务 ASI 北极星**: AI 服务 ASI, 终极 终极目标是 AGI / 长程 AI 成长 / 用户价值
    - **S-2 实事求是**: 0 装 / 0 假装 / 关键诚实标 / 8 硬墙 0 越界 / 8 哲学锚 严守
    - **S-3 质量工程化**: 编译期 hardcode / 编译期 enum / V0.5 30 维 / 6 重守门 v7 / 14 键 verdict cache
    - **O-1 安全优先**: 6 重守门 v7 / 0 装 PASS 严守 / 0 借具体源码 / 24 LOCKED 入口签名 0 改
    - **O-2 走在前人经验上**: 借脑 OpenCog / 借脑 11 源 / 0 装"已读真源码" / 1:1 翻译公开模式
    - **O-3 干到底**: 永久循环 / 0 终点 / 16 跑中上限 / 自动接续 4 步
    - **O-4 任何人都能接手**: 决策日志 / 0 装"已实施" / 完整文档 / R11 baseline 3 值 严守
    - **O-5 不假装**: 0 装 PASS 严守 / 0 装"已实施" / 0 装"已读真源码" / 0 装"已 fork" / 0 装"已集成" / 关键诚实标
- **决策 #74 §1 B5 严守**: 8 哲学锚 V1.0 release 严守 + V1.1 release 严守 (per 决策 #74 §1, 哲学不松绑) + V2.0 release 可重建 (per 决策 #74 §2.3, 8 哲学锚可重建 = 0 锚 / 12 锚 / 全新架构)
- **R131-7 O7 8 哲学锚集成** (per R131-7 §2.7, 0 触碰 8 哲学锚, Stage 8 0 触碰, 1:1 翻译 P5-2 verification 5 checks + P8-2 8 harness 跟 B5 8 锚严守)

#### 2.8.3 PHL-07 实施 (per 决策 #74 A3 + 决策 #22 §1.1-1.2 + R129-11 关键诚实标)

- **决策 #74 A3 PHL-07 V1.0 spec-only → V1.1 release 实施** (per 决策 #74 A3 + 决策 #22 §1.1-1.2 + R129-11 关键诚实标 + R125-12 P0-3):
  - **PHL-07 V1.0 spec-only 状态**: R125-12 P0-3 (8/10 16:30 done) 写 PHL-07 spec + 13-keys stub, 整合 #4 commit abf12243 done, **0 实施** PHL-07 (per R125-12 P0-3 报告, "PHL-07 spec done, V1.1 实施")
  - **R129-11 关键诚实标**: 1.0 release 时 PHL-07 spec-only, 0 假装 PHL-07 在 1.0 release 时已实施
  - **决策 #74 A3 PHL-07 实施** (V1.1 release 实施, 25 LOCKED 入口新增 1 个 PHL-07 入口):
    - PHL-07 入口签名: `pub fn phl_07_main_dialog_anchor() -> PHL07Verdict` (NEW, 25 LOCKED 入口新增 1 个)
    - PHL-07 14 维主对话锚 (per 用户记忆 #3 + #5)
    - 跟 8 哲学锚 1:1 集成 (B5 严守)
    - 跟 6 重守门 v7 1:1 集成 (B4 严守)
    - 跟 14 键 1:1 集成 (A3 升级)
    - 41 NEW tests (14 维 + 8 哲学锚 + 6 重守门 + 14 键)
- **PHL-07 跨借鉴源集成** (per 决策 #55 §2.6 + 决策 #74 A3):
  - langgraph 829 (StateGraph 1:1 翻译, 1 借脑 0 装)
  - superpowers 234 (主对话锚设计模式, 1 借脑 0 装)
- **PHL-07 0 借具体源码 100%** (per 决策 #33 §2.3 C2): 2 借脑 0 装

**V1.1 release PHL-07 实施 spec** (per 决策 #74 A3 + 决策 #22 §1.1-1.2 + R129-11 关键诚实标):

| 实施项 | 1.0 release (整合 #5 commit 拍板) | V1.1 release (整合 #6 commit 拍板) | 决策依据 |
|--------|----------------------------------|-----------------------------------|---------|
| **PHL-07 spec** | ✅ done (R125-12 P0-3) | ✅ done (跟 1.0 兼容) | R125-12 P0-3 + 决策 #33 §2.3 A3 |
| **PHL-07 入口签名** | ❌ 0 实施 (spec-only) | ✅ NEW 入口 (25 LOCKED 总数) | 决策 #22 §1.1-1.2 + 决策 #74 A3 |
| **13 → 14 键 verdict cache** | ✅ 13 键 stub (12 + PHL-07) | ✅ 14 键 真实施 (13 + PHL-07 加 1 键) | 决策 #33 §2.3 A3 |
| **14 维主对话锚** | ❌ 0 实施 | ✅ NEW 14 维 (per 用户记忆 #3 "主对话是核心" + 用户记忆 #5 拟人化) | R130-5 §2.1 + 用户记忆 #3 + #5 |
| **跟 8 哲学锚集成** | ❌ 0 集成 | ✅ 跟 8 哲学锚 1:1 集成 (B5 严守) | B5 8 哲学锚严守 + 决策 #33 §2.3 B5 |
| **跟 6 重守门 v7 集成** | ❌ 0 集成 | ✅ 跟 6 重守门 v7 1:1 集成 (B4 严守) | B4 6 重守门 v7 严守 + 决策 #33 §2.3 B4 |
| **跟 14 键集成** | ❌ 0 集成 | ✅ 跟 14 键 1:1 集成 (A3 升级, 13 → 14 键) | 决策 #33 §2.3 A3 + R130-5 §2.1 |
| **PHL-07 tests** | 0 NEW tests | 41 NEW tests (14 维 + 8 哲学锚 + 6 重守门 + 14 键) | 决策 #22 §1.2 + 决策 #33 §2.3 B1 |
| **Cargo.toml workspace.version** | 1.2.0 → 1.0.0 (整合 #5 commit) | 1.0.0 → 1.2.1 (整合 #6 commit, per 决策 #74 B2) | 决策 #22 §2.2 + 决策 #74 B2 改写 |

**V1.1 release 8 方向 关键诚实标**:
- ✅ 0 改 6 重守门 v7 (B4 严守)
- ✅ 0 改 8 哲学锚 (B5 严守)
- ✅ PHL-07 实施 (V1.0 spec-only → V1.1 release 实施, per 决策 #74 A3 + R129-11 关键诚实标)
- ✅ 0 改原 24 LOCKED 入口签名 (B1 0 改原 24 LOCKED 入口签名顺序 严守)
- ✅ 0 装 PASS 严守 100% (12 源 0 装 PASS 严守 verify)
- ✅ 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 8 硬墙全严守)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2)
- ✅ 14 维主对话锚 是 V0.5 30 维子集 (深化) (per 决策 #74 A3)
- ✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

---

## 3. V1.1 release 后端加固 5 阶段计划 (7 周 = 1 个月 3 周, 总 2026-10-15 启动 + 2026-12-03 完成, 估 2026-11-30 V1.1 release tag 打上)

### 3.1 5 阶段计划 时间窗口 (per R131-3 §3 + 决策 #71 §2.5 + 决策 #74 B1/B2)

**V1.1 release 后端加固 5 阶段计划** (per R131-3 §3 + 决策 #71 §2.5 + 决策 #74 B1/B2 + 决策 #33 C1 + R134-6 拓维):

```
V1.1 release 时间窗口 (per R130-5 + R131-3 + R132-1 + R133-1):
─────────────────────────────────────────────────────────
8/11 - 10/15 (1.0 release 整合 #5 commit 拍板 + 实战 + R131-R133 era 调研 + 计划 + 实施 spec 落地)
10/15 启动 V1.1 release 实施 (R134 era 6 sub 调研续 + V1.1 release 实施 spec 落地)
─────────────────────────────────────────────────────────

阶段 1: Cargo.toml 1.2.1 bump + 24 LOCKED 入口签名 改写 (2 周, 2026-10-15 ~ 10-29):
  - 整合 #5 commit 拍板落地 (Mavis 自决, 5.1 → 5.2 → 5.3 顺序)
  - 1.0 release 实战 (主人起床后 8/11 06:00-08:00, 7 步 runbook)
  - 1.0 release tag 打上 (v1.0.0)
  - R134 era 6 sub 调研 (R134-1/2/3/4/5/6) + 派活
  - V1.1 release Cargo.toml 1.2.1 bump 准备 (per 决策 #74 B2)
  - V1.1 release 24 LOCKED 入口签名 改写 spec 准备 (per 决策 #74 B1)
  - V1.1 release 16/24 LOCKED 入口签名 改写 (per 触发条件 1-7)
  - V1.1 release PHL-07 入口新增 1 个 (per 决策 #74 A3)
  - V1.1 release AsiDispatcher 入口新增 1 个 (per R131-7 O2.3)
  - 整合 #6 commit 准备 (拆 6.1 src/ + 6.2 docs/ + 6.3 reports/)

阶段 2: cargo test 三次 verify (1 周, 2026-10-29 ~ 11-05):
  - 第 1 次: cargo build --workspace (估 30-60 min, 验证编译通过)
  - 第 2 次: cargo test --workspace (估 60-90 min, 验证单元测试通过)
  - 第 3 次: cargo test --workspace (估 90-120 min, 验证集成测试 + 端到端测试 + 性能测试 + chaos test 通过)
  - 8 步 verify 全 PASS verify (cargo build + cargo test × 2 + cargo clippy + cargo fmt + cargo audit + cargo doc + 24 LOCKED 入口签名 0 改)
  - 整合 #6 commit 拍板准备 (8/8 verify 100%)
  - 整合 #6 commit 拍板 (Mavis 自决, 拆 3 commit, per 决策 #62 + 决策 #33 C1 + 决策 #71 §2.5)

阶段 3: 12 源 0 装严守 二次 verify (1 周, 2026-11-05 ~ 11-12):
  - 8 真 cloned 沿用 0 必重借 (mtime 早于整合 #4 commit 19:41, 0 重跑 0 重 commit)
  - 2 借鉴 ID 索引完成 沿用 0 必重借 (LiteLLM + opencode)
  - 1 永久跳过 0 重借 (OpenCog AGPL-3.0)
  - 1 借脑 ID 索引完成 借脑调研沉淀 (OpenCog 家族 6 子源)
  - 12 子源 借脑 ROI 梯度 调研 (AtomSpace + CogPrime 深度 + MOSES 中度 + cogutil + pln + relex 浅度)
  - 12 源 0 装 PASS 严守 6 维度 verify 100%
  - 整合 #6 commit 后 8 步 verify 7/8 ready + R131-5 verify 24/24 LOCKED 入口签名 0 改 全部通过
  - 整合 #6 commit 拍板 (Mavis 自决, 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5)

阶段 4: pybridge 集成优化 + ASI Stage 9 终极自治 (2 周, 2026-11-12 ~ 11-26):
  - PyO3 0.22 异步 awaitable 实施 (per R131-7 O1.2.1)
  - free-threading GIL release 实际未测 实施 (per R131-7 O1.2.2)
  - PyO3 smart_scopes 实施 (per R131-7 O1.2.3)
  - PyO3 0.24 type hint union 实施 (per R131-7 O1.2.4)
  - AsiDispatcher 统一协调器 实施 (per R131-7 O2.3 + 决策 #74 B1)
  - Stage 8 C1 12 步 cycle 实施 (per R130-2 §2.4 + R131-7)
  - Stage 9 4 维度 H/L/G/P 实施 (per R133-2 + 决策 #74 B1)
  - 借脑 OpenCog CogPrime 1:1 翻译公开模式 (per R133-2 + 决策 #73 §2.2)
  - 886 → ~1200 pybridge tests pass (V1.1 release 实施 + 200 NEW tests)
  - 整合 #7 commit 准备 (拆 7.1 src/ + 7.2 docs/ + 7.3 reports/)

阶段 5: 8 哲学锚 + PHL-07 实施 + 6 重守门 v7 + V0.5 30 维 集成 (1 周, 2026-11-26 ~ 12-03, 含 3 天 buffer):
  - 8 哲学锚严守 verify (per 决策 #33 §2.3 B5 + 决策 #74 §1)
  - PHL-07 实施 (per 决策 #74 A3 + 决策 #22 §1.1-1.2)
  - PHL-07 14 维主对话锚 跟 8 哲学锚/6 重守门/14 键集成
  - 6 重守门 v7 严守 verify (per 决策 #33 §2.3 B4 + 决策 #74 §1)
  - V0.5 30 维 严守 verify (per 决策 #33 §2.3 B3 + 决策 #74 §1)
  - 41 NEW tests (PHL-07 14 维 + 8 哲学锚 + 6 重守门 + 14 键)
  - 8 硬墙 0 越界 100% verify
  - 0 装 PASS 严守 100% verify
  - 8 步 verify 全 PASS 100%
  - 整合 #7 commit 拍板 (Mavis 自决, 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5)
  - 整合 #7 commit 后 8 步 verify 8/8 PASS 100%

V1.1 release 实战 (估 2026-11-30 06:00-08:00, 主人手跑 V1.1 release 7 步 runbook):
  - 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署
  - master HEAD = abf12243 + 6 commit (5.1/5.2/5.3/6.1/6.2/6.3/7.1/7.2/7.3) = 9 commit
  - workspace.version = "1.2.1" 严守 (per 决策 #74 B2 改写)
  - v1.1.0 tag 打上
  - V1.1 release done
─────────────────────────────────────────────────────────
总时间盒: 7 周 (V1.1 release 估 2026-11-30 per R131-3)
```

### 3.2 5 阶段计划 派活规划 (per 决策 #71 §5 R133+ era 实施 5-10 sub-agent + 决策 #75 §2.1 + 决策 #76 §2.1)

**V1.1 release 后端加固 5 阶段 派活规划** (per 决策 #71 §5 + 决策 #75 §2.1 + 决策 #76 §2.1 + R134-6 拓维):

- **R134 era 调研 6 sub** (per 决策 #76 §2.1, 01:30 拍板, 跑中 60 min 时间盒):
  - R134-1 整合 #5 commit 拍板实战 (实施流程)
  - R134-2 1.0 release 实战 (实战流程)
  - R134-3 整合 #6 commit 拍板 (拍板流程)
  - R134-4 整合 #7 commit 拍板 (拍板流程)
  - R134-5 V1.1 release cargo 二次 verify (verify 流程)
  - **R134-6 V1.1 release 后端加固 (本报告, 准备 + 5 阶段计划 + 8 方向 spec)**
- **R135 era 差距 2 sub** (per 决策 #76 §2.1, 01:30 拍板, 跑中 60 min 时间盒):
  - R135-1 V1.1 release 跟 AGI 操作系统前沿差距
  - R135-2 V1.1 release 跟业界 v2.x 路线图差距
- **V1.1 release 实施 派活规划** (per 决策 #71 §5 R133+ era 实施 5-10 sub-agent, 估 8-12 派活 per 5 阶段):
  - 阶段 1 (2 周): 派 4-6 sub-agent (Cargo.toml 1.2.1 bump 准备 + 24 LOCKED 入口签名 改写 + PHL-07 入口新增 + AsiDispatcher 入口新增)
  - 阶段 2 (1 周): 派 2-3 sub-agent (cargo test 三次 verify 跑通 + 8 步 verify 100%)
  - 阶段 3 (1 周): 派 2-3 sub-agent (12 源 0 装严守 二次 verify + 6 子源借脑调研沉淀)
  - 阶段 4 (2 周): 派 5-7 sub-agent (PyO3 4 处可深化 + AsiDispatcher 协调器 + Stage 8 12 步 cycle + Stage 9 4 维度 H/L/G/P + 借脑 OpenCog CogPrime)
  - 阶段 5 (1 周): 派 3-4 sub-agent (PHL-07 实施 + 41 NEW tests + 8 硬墙 verify + 0 装 PASS verify + 整合 #7 commit 拍板)
  - 总 V1.1 release 实施 派活: 16-23 sub-agent, 16 跑中上限严守 (per 主人 0:34 拍板), 2 批 8-12 派满 16 上限

### 3.3 5 阶段计划 风险 + 决策原则 (per R131-3 §3 风险 + 决策 #74 风险 + 决策原则)

**V1.1 release 后端加固 5 阶段 风险** (per R131-3 §3 风险 + 决策 #74 风险):

- **R1**: 24 LOCKED 入口签名 改写 引入 breaking change → **缓解**: 阶段性 改写 (16/24 per 触发条件 1-7) + 1:1 verify + cargo test 三次 verify
- **R2**: Cargo.toml 1.2.1 bump 引入 cargo dep 冲突 → **缓解**: 借脑 ID 索引完成 (0 装"已读真源码") + cargo-deny 自动 check + 0 装 PASS 严守 100%
- **R3**: pybridge 性能瓶颈 (Stage 8 12 步 cycle 100ms → 30ms 优化 失败) → **缓解**: PyO3 0.22 异步 awaitable + free-threading GIL release + smart_scopes + type hint union 4 处可深化
- **R4**: ASI Stage 9 4 维度 H/L/G/P 集成失败 → **缓解**: AsiDispatcher 统一协调器 + Stage 8 12 步 cycle spec + 借脑 OpenCog CogPrime 1:1 翻译公开模式
- **R5**: 借脑 OpenCog CogPrime 引入 AGPL-3.0 license 风险 → **缓解**: 1:1 翻译公开模式 0 借具体源码, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork", 主仓 0 触碰 OpenCog code, 0 装 PASS 严守 100%
- **R6**: cargo workspace 重构 引入编译错误 → **缓解**: 3 真 transparent re-export 合并 (life-force / value / consciousness) + 借鉴模式 12 个统一为 1 个 `apeireth-borrowed-patterns` + 5 估补 R20 阶段 1 合并到 `apeireth-mcp`, 0 改原 24 LOCKED 入口签名顺序
- **R7**: 8 硬墙 0 越界 失败 → **缓解**: 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (B1 严守, 除非满足触发条件) + 0 改 R11 baseline 3 值 (A1 严守, 除非新的 baseline 更高) + 0 改 8 哲学锚 (B5 严守) + 0 改 V0.5 30 维 (B3 严守) + 0 改 6 重守门 v7 (B4 严守) + 0 主动 commit (C1 严守) + 0 装 PASS 严守 (C2 严守) + 0 主动 push (严守)
- **R8**: PHL-07 实施 跟 24 LOCKED 入口签名 0 改 冲突 → **缓解**: PHL-07 入口新增 1 个 (per 决策 #74 A3, 25 LOCKED 总数), 0 改原 24 LOCKED 入口签名顺序
- **R9**: 团队对 "不要怕复杂度" 哲学不适应 → **缓解**: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)
- **R10**: 整合 #6/7 commit 拍板 失败 → **缓解**: Mavis 自决 (per 决策 #62 + 决策 #33 C1 + 决策 #71 §2.5), 8 步 verify 100% 后拍板

**V1.1 release 后端加固 5 阶段 决策原则** (per R131-3 §3 决策原则 + 决策 #74 决策原则 + 用户记忆 #1-#10):

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写 + B2 1.2.1 bump** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74 B1 改写)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.2.1** (per 决策 #22 §2.2 + 决策 #74 B2 改写)
- **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only + V1.1 release 实施
- **B3 V0.5 30 维**: 严守 (哲学)
- **B4 6 重守门 v7**: 严守 (哲学守门)
- **B5 8 哲学锚**: 严守 (哲学)
- **C1 0 主动 commit**: 严守 (Mavis 拍板, 0 主动 push)
- **C2 0 装 PASS**: 严守 (技术哲学)
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md)
- **整合 #5/6/7 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 4. Cargo.toml 1.2.0 → 1.2.1 bump 方案 (per 决策 #74 B2 V1.1 release bump)

### 4.1 任务背景 (per 决策 #74 B2 改写 + 决策 #22 §2.2 semver)

- **决策 #22 §2.2 semver 严守**: Cargo.toml workspace.version 严守 semver 规范 (major.minor.patch), V1.0 release 大版本归 0 (1.2.0 → 1.0.0), V1.1 release minor bump (1.0.0 → 1.1.0 → 1.2.1, per 决策 #74 B2 改写)
- **决策 #74 B2 改写** (per 决策 #33 §2.3 B2 + 主人 8/11 01:14 拍板 "不要怕复杂度" + 决策 #74 B2 改写):
  - V1.0 release 1.2.0 严守 (整合 #4 commit abf12243 19:41 done 时 workspace.version = "1.2.0")
  - V1.0 release 整合 #5.2 commit 时 1.2.0 → 1.0.0 (per 决策 #22 §2.2 + 决策 #62 §5.2, 1.0 release tag)
  - V1.1 release 整合 #6 commit 时 1.0.0 → 1.2.1 (per 决策 #74 B2 改写, V1.1 release tag, "不要怕复杂度"哲学落地)

### 4.2 Cargo.toml 1.2.0 → 1.2.1 bump 详细方案 (per 决策 #74 B2 改写 + R131-6 §2)

**整合 #5.2 commit 时 Cargo.toml update** (per 决策 #62 §5.2 + 决策 #74 B2):
- workspace.version = "1.2.0" → "1.0.0" (per 决策 #22 §2.2 + 决策 #62 §5.2, 1.0 release tag)
- Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per R131-2 §4.3 + R131-6 §1.2)
  - `borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` (17:44 状态)
  - → `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (12/12 状态)
  - `borrow_cloned = [...]` 7 → 8 entries (+Guardrails) → 10 entries (+LiteLLM 借鉴 ID 索引完成 + opencode 改借鉴已 cloned)
  - `borrow_rate_limited = [...]` 3 → 0 entries (P6-1/2/3 全 done)
  - `borrow_skipped = [...]` 1 entry 0 改
  - 🆕 `borrow_brainonly = [...]` 1 entry (OpenCog 家族 6 子源, AGPL-3.0 借脑, 0 装 PASS 严守)
  - `decision_chain_range` = "decision-22 ~ decision-58" (37 个) → "decision-22 ~ decision-75" (54 个, 含 R130 era + R131 era + R133 era 决策链)
  - `description` = "借鉴 8/11" → "借鉴 10/11 + 1 借脑 = 11/12"
- Cargo.lock / .gitignore 严守
- + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)
- + 更新 `docs/conventions/10-locked.md` (per 决策 #74 B1 改写)
- + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)
- + 更新 `docs/conventions/README.md` (per 决策 #73 §2.3)
- + 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3)
- + 更新 `README.md` (per 决策 #73 §2.3)

**整合 #6 commit 时 Cargo.toml update** (per 决策 #62 类比 + 决策 #74 B2 改写, 估 2026-11-25):
- workspace.version = "1.0.0" → "1.2.1" (per 决策 #74 B2 改写, V1.1 release tag)
- Cargo.toml borrow 段 update 12/12 状态 → V1.1 release 0 装 PASS 严守 100% 状态
  - `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` 0 改
  - `borrow_cloned = [...]` 10 entries 0 改 (V1.1 release 沿用 0 必重借)
  - `borrow_rate_limited = [...]` 0 entries 0 改
  - `borrow_skipped = [...]` 1 entry 0 改 (OpenCog AGPL-3.0 永久跳过 严守)
  - `borrow_brainonly = [...]` 1 entry 0 改 (V1.1 release 借脑调研沉淀 0 装"已读真源码")
  - `decision_chain_range` = "decision-22 ~ decision-75" → "decision-22 ~ decision-76" (55 个, V1.1 release + 决策 #76 R134 era 派活拍板)
  - `description` = "借鉴 10/11 + 1 借脑 = 11/12" → "V1.1 release 0 装 PASS 严守 100% (12 源沿用, OpenCog 家族 6 子源 借脑调研沉淀)"

**整合 #7 commit 时 Cargo.toml update** (per 决策 #62 类比, 估 2026-11-29):
- workspace.version = "1.2.1" 0 改 (V1.1 release tag 严守)
- Cargo.toml borrow 段 0 改 (V1.1 release 0 装 PASS 严守 100% verify 100%)
- Cargo.lock 分模块 lockfile (per R131-4, Cargo 1.78+ feature, V1.1 release 实施)

### 4.3 Cargo.toml 1.2.0 → 1.2.1 bump 关键诚实标 (per 决策 #62 §5.2 + R131-6 §1.2/§1.3/§1.4 + 决策 #74 B2)

- **决策 #74 B2 改写 关键诚实标**: V1.0 release 严守 workspace.version = "1.0.0" + V1.1 release bump "1.2.1" (per 决策 #74 B2 改写 + 决策 #22 §2.2 semver, "不要怕复杂度"哲学 落地)
- **R131-6 §1.2 关键诚实标 1**: `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (整合 #5.2 commit 时 update, +Guardrails)
- **R131-6 §1.3 关键诚实标 2**: `count_total=11` (8+3+1=12 ≠ 11) 算术不一致 (整合 #5.2 commit 时 update 11 → 12 借脑)
- **R131-6 §1.4 关键诚实标 3**: `decision_chain_range = "decision-22 ~ decision-58"` (37 个) 实际范围 decision-22 ~ decision-76 (55 个) 不一致 (整合 #5.2 commit 时 update 58 → 75, 整合 #6 commit 时 update 75 → 76)
- **R131-6 §1.4 关键诚实标 4**: `description` 当前 "借鉴 8/11" 跟 22:50 状态 "10/11" 跟 12/12 状态 "10/11 + 1 借脑" 不一致 (整合 #5.2 commit 时 update)
- **V1.1 release Cargo.toml 1.2.1 bump 关键诚实标 5**: V1.0 release 1.0.0 → V1.1 release 1.2.1 (per 决策 #74 B2 改写, "不要怕复杂度"哲学落地, 0 改原 24 LOCKED 入口签名 + 0 装 PASS 严守 100%)

---

## 5. 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改)

### 5.1 24 LOCKED 入口签名 改写 触发条件 (per 决策 #74 §2.2 + 决策 #73 §1 "更好的架构")

**V1.1 release Mavis 自决改 触发条件** (per 决策 #73 §1 "Mavis 自决架构拍板" + 决策 #74 B1 改写):

- **触发 1: ASI Stage 9 长程 AI 成长** (per R130-2 §1 + R133-2)
- **触发 2: 9 organ 内部借 OpenCode** (per R130-3 §2.4)
- **触发 3: 三洋葱架构升级** (per R133-3, V1.1 release 三洋葱 → 四洋葱)
- **触发 4: PHL-07 实施扩展** (per §2.8 方向 8, 14 维主对话锚)
- **触发 5: cargo workspace 重构** (per R131-4, 87 crate 优化)
- **触发 6: 智囊团 7 席架构** (per R18 + 决策 #55 §2.6)
- **触发 7: 群体智能 OpenCog 借脑** (per R130-2 §1.5 + R133-1)

**V1.1 release 0 改严守边界** (per 决策 #74 §2.3):
- ❌ 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (除非满足触发条件)
- ❌ 0 改 R11 baseline 3 值 (除非满足触发条件: 新的 baseline 更高)
- ❌ 0 改 8 哲学锚 (B5 严守)
- ❌ 0 改 V0.5 30 维 (B3 严守)
- ❌ 0 改 6 重守门 v7 (B4 严守)
- ❌ 0 改 0 主动 commit (C1 严守)
- ❌ 0 改 0 装 PASS 严守 (C2 严守)
- ❌ 0 改 0 主动 push (严守)
- ✅ 改 24 LOCKED 入口签名 (前提: 满足触发条件, Mavis 自决)

### 5.2 24 LOCKED 入口签名 改写 实施 spec (per 决策 #74 B1 + R131-5 §2 + R133-1 + R133-2 + R133-3)

详见 §2.2.4 24 LOCKED 入口签名 改写 实施 spec 表.

**V1.1 release 24 LOCKED 入口签名 改写 总结**:
- 24 LOCKED crate 中 16 个 crate 入口签名 改写 (per 触发条件 1-7)
- 8 个 crate 入口签名 0 改 (supervisor, tool-approval, extension, action, bench, mcp 7 个 + 1 改 transparent re-export)
- 0 改原 24 LOCKED 入口签名顺序 (严守)
- 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (严守, 除非满足触发条件)
- 🆕 PHL-07 入口新增 1 个 (25 LOCKED 总数, per 决策 #74 A3)
- 🆕 AsiDispatcher 入口新增 1 个 (pybridge 协调器, per R131-7 O2.3)

### 5.3 24 LOCKED 入口签名 改写 时序图 (per 决策 #74 §2.2 + 决策 #33 §2.3 B1 + 决策 #62 整合 #5 commit 拍板)

```
V1.0 release 0 改严守 (整合 #5 commit 拍板, 估 8/11 01:30+ 拍板):
─────────────────────────────────────────────────────────
整合 #4 commit abf12243 (8/10 19:41 done) master HEAD
  ↓
整合 #5.1 commit (src/ 实施, 95+ 文件, 决策 #62 §5.1)
  - 0 改 24 LOCKED 入口签名 (严守 R11 baseline)
  - 0 改 R11 baseline 3 值 0.8682/0.8532/0.9063 (严守)
  - 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
  - PHL-07 spec-only 0 实施 (严守, V1.1 实施, per R129-11 关键诚实标)
  ↓
整合 #5.2 commit (docs/ + Cargo.toml, 10 文件, 决策 #62 §5.2)
  - Cargo.toml workspace.version 1.2.0 → 1.0.0 (1.0 release tag)
  - 0 改 24 LOCKED 入口签名 (严守)
  ↓
整合 #5.3 commit (reports/, 60+ 文件, 决策 #62 §5.3)
  - 0 改 src/ (严守, 备查用)
  ↓
整合 #5 commit 拍板 done (Mavis 自决, 8 项 verify 100% 后, per 决策 #62 + 决策 #64)
  - master HEAD = abf12243 + 3 commit (5.1/5.2/5.3)
  - 24 LOCKED 入口签名 0 改 100%
  - R11 baseline 3 值 0 改 100%
  - workspace.version = "1.0.0" 严守
  - 8 硬墙 0 越界 100%

[8/11 06:00-08:00 主人起床 1.0 release 实战] 主人手跑 R130-5 7 步 runbook
[8/11 08:00+ 1.0 release done] master HEAD = abf12243 + 3 commit, v1.0.0 tag, GitHub release

[8/12 - 10/15 R134 era 调研 6 sub 实施 + R131-R133 era 调研 + 计划 + 实施 spec 落地]
  - R134-1 整合 #5 commit 拍板实战
  - R134-2 1.0 release 实战
  - R134-3 整合 #6 commit 拍板
  - R134-4 整合 #7 commit 拍板
  - R134-5 V1.1 release cargo 二次 verify
  - R134-6 V1.1 release 后端加固 (本报告)

V1.1 release Mavis 自决改 (整合 #6 commit 拍板, 估 2026-11-25):
─────────────────────────────────────────────────────────
整合 #6.1 commit (src/ 实施, 估 50+ 文件, 决策 #62 类比)
  - 🆕 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改)
    - 16/24 crate 入口签名 改写 (per 触发条件 1-7)
    - 0 改原 24 LOCKED 入口签名顺序 (B1 严守)
    - 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (B1 严守, 除非满足触发条件)
    - 🆕 PHL-07 入口新增 1 个 (25 LOCKED 总数, per 决策 #74 A3)
    - 🆕 AsiDispatcher 入口新增 1 个 (pybridge 协调器, per R131-7 O2.3)
  - 🆕 Stage 8 C1 12 步 cycle 实施 (per R130-2 §2.4 + R131-7)
  - 🆕 Stage 9 4 维度 H/L/G/P 实施 (per R133-2 + 决策 #74 B1)
  - 🆕 PyO3 4 处可深化 (per R131-7 O1.2)
  - 🆕 cargo workspace 重构 (per R131-4, 87 → 70-75 crate 优化)
  - 🆕 PHL-07 14 维主对话锚 实施 (per 决策 #74 A3 + 决策 #22 §1.1-1.2)
  - 🆕 41 NEW tests (per 决策 #33 §2.3 B1)
  ↓
整合 #6.2 commit (docs/ + Cargo.toml, 估 10 文件, 决策 #62 类比)
  - workspace.version = "1.0.0" → "1.2.1" (per 决策 #74 B2 改写)
  - Cargo.toml borrow 段 update 12/12 → V1.1 release 0 装 PASS 严守 100% 状态
  - + 新增 PHL-07 入口 spec 文档
  - + 更新 8 哲学锚 spec 文档 (per 决策 #74 A3)
  - + 更新 6 重守门 v7 spec 文档
  - + 更新 V0.5 30 维 spec 文档
  - + 更新 14 键 verdict cache spec 文档
  ↓
整合 #6.3 commit (reports/, 估 30+ 文件, 决策 #62 类比)
  - 0 改 src/ (严守, 备查用)
  ↓
整合 #6 commit 拍板 done (Mavis 自决, 8 项 verify 100% 后, per 决策 #62 + 决策 #33 C1 + 决策 #71 §2.5)
  - master HEAD = abf12243 + 6 commit (5.1/5.2/5.3/6.1/6.2/6.3)
  - 24 LOCKED 入口签名 改写 100% (16/24 触发 + 8/24 0 改)
  - PHL-07 入口新增 1 个 (25 LOCKED 总数)
  - AsiDispatcher 入口新增 1 个 (pybridge 协调器)
  - workspace.version = "1.2.1" 严守
  - 8 硬墙 0 越界 100%

整合 #7 commit 拍板 (Mavis 自决, 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5):
  - cargo test 三次 verify 100%
  - 12 源 0 装 PASS 严守 100% verify
  - pybridge ~1200 tests pass
  - Stage 8 12 步 cycle 集成测试通过
  - Stage 9 4 维度 H/L/G/P 集成测试通过
  - Cargo.lock 分模块 lockfile 测试通过
  - 8 步 verify 8/8 PASS 100%

V1.1 release 实战 (估 2026-11-30 06:00-08:00, 主人手跑 V1.1 release 7 步 runbook):
  - 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署
  - master HEAD = abf12243 + 9 commit
  - workspace.version = "1.2.1" 严守
  - v1.1.0 tag 打上
  - V1.1 release done
```

---

## 6. 12 源 0 装严守 二次 verify (per R133-1 + R131-6 + 决策 #33 §2.3 C2)

### 6.1 12 源清单 + 0 装严守 verify (per R133-1 §1 + R131-6 §1 + R130-6 §1.2 + R131-2 §2.1)

详见 §2.4.2 V1.1 release 12 源 0 装严守 二次 verify 方案 表.

**总 12/12 借鉴源 V1.1 release 0 装 PASS 严守二次 verify 100% 方案**:
- ✅ 8 真 cloned 沿用 0 必重借 (mtime 早于整合 #4 commit 19:41, 0 重跑 0 重 commit)
- ⏳ 0 限流 (P6-1/2/3 全 done, 0 借鉴处于限流)
- ❌ 1 永久跳过 (OpenCog AGPL-3.0 0 集成 0 装)
- 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀)

### 6.2 12 源 0 装严守 6 维度 verify (per 决策 #33 §2.3 C2 + R129-7 §5.1 + R129-28 §3.2 + R131-2 §3.2.3)

详见 §2.4.4 0 装 PASS 严守 6 维度 verify 表.

**0 装 PASS 严守 6 维度 100% PASS** (per R129-7 §5.1 + R129-28 §3.2 + R131-2 §3.2.3 + R133-1 01:25 实地 verify 100% 严守 + R131-6 01:30 实地 verify 100% 严守)。

### 6.3 V1.1 release 借脑调研沉淀 6 子源 (per R133-1 §2.3 + 决策 #55 §2.6 + 决策 #73 §2.2 + 决策 #74 B1)

详见 §2.4.3 V1.1 release 借脑调研沉淀 6 子源 表.

**借脑调研总文档沉淀** (~95-155 KB, 6 文档, 借脑 ID 索引完成):
- 🟢 AtomSpace 深度 (~30-50 KB) + 🟢 CogPrime 深度 (~30-50 KB) = ~60-100 KB
- 🟡 MOSES 中度 (~10-20 KB)
- 🔴 cogutil + pln + relex 浅度 (~15-30 KB, 3 子源合 1-3 文档)

### 6.4 V1.1 release 12 源 0 装严守 二次 verify 关键诚实标 (per 决策 #33 §2.3 C2 + 决策 #74 §1 + R133-1 + R131-6)

- ✅ 12 源 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #74 §1)
- ✅ 8 真 cloned 沿用 0 必重借 (mtime 早于整合 #4 commit 19:41, 0 重跑 0 重 commit)
- ✅ 0 限流 (P6-1/2/3 全 done)
- ✅ 1 永久跳过 (OpenCog AGPL-3.0 0 集成 0 装)
- ✅ 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀 0 装"已读真源码")
- ✅ 8 硬墙 0 越界 100% (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改, 其他 8 硬墙全严守)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2)
- ✅ 0 改 8 哲学锚 (B5 严守)
- ✅ 0 改 V0.5 30 维 (B3 严守)
- ✅ 0 改 6 重守门 v7 (B4 严守)

---

## 7. pybridge 集成优化 + ASI Stage 9 终极自治 (per R131-7 + R133-2 + 决策 #74 B1)

### 7.1 pybridge 集成优化 9 方向 详细 (per R131-7 §2)

详见 §2.5.1 V1.1 release pybridge 集成优化 实施 spec.

**V1.1 release pybridge 集成优化 9 方向总结**:
- O1 PyO3 928 借鉴深度 (16 处 1:1 翻译 + 4 处可深化, per R131-7 §2.1)
- O2 ASI Python 8 阶段集成 (8 阶段 63 个 1:1 映射, per R131-7 §2.2)
- O3 886/886 pybridge tests (实际 1007 累加, per R131-7 §2.3)
- O4 跨语言调用性能 (per R131-7 §2.4)
- O5 V0.5 30 维公式 (per 决策 #33 §2.3 B3 严守 100%, 0 触碰 integration_r_measure.rs)
- O6 6 重守门 v7 集成 (per R131-7 §2.6, 30 绑定 + 42 绑定 1:1 跟 B4 严守)
- O7 8 哲学锚集成 (per 决策 #33 §2.3 B5 严守 100%)
- O8 V1.1 release ASI Stage 9 长程 AI 成长 (per R131-7 + R133-2)
- O9 pybridge 与 cargo workspace 集成 (per R131-7, 1 真相源 + 5 共享 + 永远循环)

### 7.2 ASI Stage 9 终极自治 4 维度 (per R133-2 + 决策 #74 B1)

- **H 自治 (Autonomy)**: 4 NEW src (在线自检 + 自动修复 + rollback + 学习, per R130-2 §1 Stage 9 路线图)
- **L 长程 (Long-term)**: 4 NEW src (跨会话记忆 + 经验积累 + 知识累积, per chidori journal 9 字段 replay)
- **G 成长 (Growth)**: 4 NEW src (能力升级 + 知识扩展 + 模式识别, per OpenCog AtomSpace 借脑 1:1 公开模式)
- **P 平台化 (Platform)**: 4 NEW src (多 agent 协同 + 知识共享 + 任务分配 + 冲突解决, per OpenCog CogPrime 借脑 1:1 公开模式)

### 7.3 V1.1 release pybridge 集成优化 + ASI Stage 9 关键诚实标 (per 决策 #33 §2.3 C2 + 决策 #74 B1 + R131-7 + R133-2)

- ✅ 0 改原 24 LOCKED 入口签名 (B1 0 改原 24 LOCKED 入口签名顺序 严守)
- ✅ 0 装 PASS 严守 100% (5 借脑 0 装: PyO3 928 + superpowers 234 + langgraph 829 + chidori + servers 175)
- ✅ 8 硬墙 0 越界 100% (B1 V1.1 release Mavis 自决改, 其他 8 硬墙全严守)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2, 借脑 OpenCog 1:1 翻译公开模式)
- ✅ 0 改 8 哲学锚 (B5 严守)
- ✅ 0 改 V0.5 30 维 (B3 严守)
- ✅ 0 改 6 重守门 v7 (B4 严守)
- ✅ 借脑 OpenCog CogPrime 1:1 翻译公开模式 (0 借具体源码, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- ✅ 13 NEW mod (Stage 8 4 + Stage 9 4 + AsiDispatcher 1 + PyO3 深化 4) 总 42 mod

---

## 8. 8 硬墙严守 + B1/B2 改写边界 (per 决策 #33 §2.3 + 决策 #74 §1)

### 8.1 8 硬墙 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1)

| # | 8 硬墙 | V1.0 release 严守 (整合 #5 commit 拍板) | V1.1 release 严守/改写 (整合 #6 commit 拍板) | V2.0 release 严守/可重评 |
|---|--------|----------------------------------|-----------------------------------|-----------------------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline, 24/24 verify 100%) | 🟢 Mavis 自决改 (per 决策 #74 B1, 前提: 满足触发条件 1-7) | 🟢 全 8 硬墙可重评 |
| **B2** | **Cargo.toml workspace.version** | 🔒 1.2.0 → 1.0.0 严守 (1.0 release tag) | 🟢 bump 1.0.0 → 1.2.1 (per 决策 #74 B2 改写, V1.1 release tag) | 🟢 bump 1.2.1 → 2.0.0 (V2.0 release tag, major 升级) |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 严守 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) | 🟢 可重评 |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 (per R129-11 关键诚实标) + 12 键其他可改 | 🟢 14 键 (PHL-07 实施, per 决策 #74 A3) | 🟢 可重评 |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学公式) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学守门) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B5** | **8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重建 (per 决策 #74 §2.3, 8 哲学锚可重建 = 0 锚 / 12 锚 / 全新架构) |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 严守 (Mavis 拍板, 0 主动 push) | 🔒 严守 | 🟢 0 改 (Mavis 自动 commit + push, per 决策 #74 §2.3 V2.0 release 可重评) |
| **C2** | **0 装 PASS 严守** | 🔒 严守 (技术哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (允许装特定包, per 决策 #74 §2.3) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 严守 (等 1.0 release 配 GitHub remote + 主人起床后手跑) | 🔒 严守 (等 V1.1 release 配 GitHub remote + 主人起床后手跑) | 🔒 严守 (V2.0 release 也严守 0 主动 push, per 决策 #74 §2.3) |

### 8.2 B1/B2 改写边界 (per 决策 #74 §2.2 + 决策 #74 B1/B2 + 决策 #73 §1 "更好的架构")

- **B1 24 LOCKED 入口签名 改写边界**:
  - V1.0 release 0 改严守 (R11 baseline 严守, 24/24 verify 100%, per R131-5 01:35 done)
  - V1.1 release Mavis 自决改 (前提: 满足触发条件 1-7, per 决策 #74 §2.2 + 决策 #73 §1)
  - V1.1 release 16/24 crate 入口签名 改写 + 8/24 crate 入口签名 0 改
  - V1.1 release PHL-07 入口新增 1 个 (25 LOCKED 总数)
  - V1.1 release AsiDispatcher 入口新增 1 个 (pybridge 协调器)
  - V2.0 release 全 8 硬墙可重评 (per 决策 #74 §2.3)
- **B2 Cargo.toml workspace.version 改写边界**:
  - V1.0 release 1.2.0 → 1.0.0 严守 (1.0 release tag, per 决策 #22 §2.2 + 决策 #62 §5.2)
  - V1.1 release 1.0.0 → 1.2.1 bump (V1.1 release tag, per 决策 #74 B2 改写 + "不要怕复杂度"哲学)
  - V2.0 release 1.2.1 → 2.0.0 major bump (V2.0 release tag, per 决策 #74 §2.3 + semver 严守)

---

## 9. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + `docs/conventions/09-anchor.md`)

### 9.1 8 哲学锚 严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

- **S-1 服务 ASI 北极星**: AI 服务 ASI, 终极 终极目标是 AGI / 长程 AI 成长 / 用户价值
- **S-2 实事求是**: 0 装 / 0 假装 / 关键诚实标 / 8 硬墙 0 越界 / 8 哲学锚 严守
- **S-3 质量工程化**: 编译期 hardcode / 编译期 enum / V0.5 30 维 / 6 重守门 v7 / 14 键 verdict cache
- **O-1 安全优先**: 6 重守门 v7 / 0 装 PASS 严守 / 0 借具体源码 / 24 LOCKED 入口签名 0 改
- **O-2 走在前人经验上**: 借脑 OpenCog / 借脑 11 源 / 0 装"已读真源码" / 1:1 翻译公开模式
- **O-3 干到底**: 永久循环 / 0 终点 / 16 跑中上限 / 自动接续 4 步
- **O-4 任何人都能接手**: 决策日志 / 0 装"已实施" / 完整文档 / R11 baseline 3 值 严守
- **O-5 不假装**: 0 装 PASS 严守 / 0 装"已实施" / 0 装"已读真源码" / 0 装"已 fork" / 0 装"已集成" / 关键诚实标

### 9.2 V1.1 release 8 哲学锚 严守 集成方案 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 决策 #74 A3)

- ✅ 0 改 8 哲学锚 (B5 严守, V1.0 release + V1.1 release 都严守, per 决策 #74 §1)
- 🆕 PHL-07 14 维主对话锚 跟 8 哲学锚 1:1 集成 (per 决策 #74 A3)
  - S-1 北极星 → PHL-07 #1
  - S-2 实事求是 → PHL-07 #2
  - S-3 质量工程化 → PHL-07 #3
  - O-1 安全优先 → PHL-07 #4
  - O-2 走在前人经验上 → PHL-07 #5
  - O-3 干到底 → PHL-07 #6
  - O-4 任何人都能接手 → PHL-07 #7
  - O-5 不假装 → PHL-07 #8
- ✅ V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3, 8 哲学锚可重建 = 0 锚 / 12 锚 / 全新架构)

### 9.3 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- 8 哲学锚是**思想哲学** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装)
- 不要怕复杂度是**工程哲学** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队)
- 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (互相不替代, 互补)
- V1.1 release 9 件套 总哲学 落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

---

## 10. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 10.1 不要怕复杂度哲学 核心 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3)

- **最强效果 > 最简单代码**: 复杂度是实力的体现, 不要为了简化而牺牲效果
- **最厉害工程 > 最易维护**: 工程要达到最厉害, 不要为了易维护而妥协
- **维护交给未来高水平团队**: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应
- **永久循环 / 0 终点**: 16 跑中上限 + 自动接续 4 步 + 永久演化, V1.1 release → V1.2 minor → V2.0 major → V2.1 minor → V3.0 major → ... (per 决策 #71 §4 永久循环 4 步)
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学**: 互相不替代, 互补 (per 决策 #73 §3 + R131-6 §0)

### 10.2 V1.1 release 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- **🆕 87 crate → 70-75 crate 优化但保留** (per R131-4, "不要怕复杂度"哲学: 87 crate 保留, 优化 15-17 个, 维护交给未来高水平团队)
- **🆕 9 organ 拟人化深化** (per R130-3, "不要怕复杂度"哲学: 9 organ 拟人化深化 + 5 维主对话 UX 优化 + 1 屏多卡片)
- **🆕 借脑 OpenCog AtomSpace/CogPrime/moses/pln/cogutil/relex** (per R133-1 + 决策 #73 §2.2 + 决策 #74 B1, "不要怕复杂度"哲学: AGPL-3.0 借脑 1:1 翻译公开模式 0 借具体源码, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- **🆕 ASI Stage 9 4 维度 H/L/G/P 终极自治** (per R133-2 + 决策 #74 B1, "不要怕复杂度"哲学: +200KB NEW src + 200 NEW tests + 4 NEW examples, Stage 9 长程 AI 成长 平台化)
- **🆕 V1.1 release Cargo.toml 1.2.1 bump** (per 决策 #74 B2 改写, "不要怕复杂度"哲学落地: 1.0.0 → 1.2.1 minor bump, 0 装严守 + 8 硬墙严守 + 8 哲学锚严守)
- **🆕 V1.1 release 24 LOCKED 入口签名 改写** (per 决策 #74 B1, "不要怕复杂度"哲学落地: 16/24 crate 入口签名 改写 + 25 LOCKED 总数 + AsiDispatcher 入口新增)

### 10.3 V1.1 release 不要怕复杂度哲学 关键诚实标 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- ✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 落地 (per 决策 #73 §3)
- ✅ 87 crate 保留, 优化 15-17 个 (per R131-4, "不要怕复杂度"哲学)
- ✅ 9 organ 拟人化深化 (per R130-3, "不要怕复杂度"哲学)
- ✅ 借脑 OpenCog 1:1 翻译公开模式 0 借具体源码 (per R133-1 + 决策 #73 §2.2, "不要怕复杂度"哲学)
- ✅ ASI Stage 9 4 维度 H/L/G/P 终极自治 (per R133-2, "不要怕复杂度"哲学)
- ✅ Cargo.toml 1.2.1 bump (per 决策 #74 B2 改写, "不要怕复杂度"哲学)
- ✅ 24 LOCKED 入口签名 改写 (per 决策 #74 B1, "不要怕复杂度"哲学)
- ✅ 永久循环 / 0 终点 (per 决策 #71 §4, "不要怕复杂度"哲学)

---

## 11. 风险 + 决策原则 (per R131-3 §3 风险 + 决策 #74 风险 + 决策原则)

### 11.1 风险 (per R131-3 §3 风险 + 决策 #74 风险 + 决策原则)

- **R1**: 24 LOCKED 入口签名 改写 引入 breaking change → **缓解**: 阶段性 改写 (16/24 per 触发条件 1-7) + 1:1 verify + cargo test 三次 verify
- **R2**: Cargo.toml 1.2.1 bump 引入 cargo dep 冲突 → **缓解**: 借脑 ID 索引完成 (0 装"已读真源码") + cargo-deny 自动 check + 0 装 PASS 严守 100%
- **R3**: pybridge 性能瓶颈 (Stage 8 12 步 cycle 100ms → 30ms 优化 失败) → **缓解**: PyO3 0.22 异步 awaitable + free-threading GIL release + smart_scopes + type hint union 4 处可深化
- **R4**: ASI Stage 9 4 维度 H/L/G/P 集成失败 → **缓解**: AsiDispatcher 统一协调器 + Stage 8 12 步 cycle spec + 借脑 OpenCog CogPrime 1:1 翻译公开模式
- **R5**: 借脑 OpenCog CogPrime 引入 AGPL-3.0 license 风险 → **缓解**: 1:1 翻译公开模式 0 借具体源码, 主仓 0 触碰 OpenCog code, 0 装 PASS 严守 100%
- **R6**: cargo workspace 重构 引入编译错误 → **缓解**: 3 真 transparent re-export 合并 + 借鉴模式 12 个统一为 1 个 `apeireth-borrowed-patterns` + 5 估补 R20 阶段 1 合并到 `apeireth-mcp`, 0 改原 24 LOCKED 入口签名顺序
- **R7**: 8 硬墙 0 越界 失败 → **缓解**: 0 改原 24 LOCKED crate mtime baseline 16:34 之前 + 0 改 R11 baseline 3 值 + 0 改 8 哲学锚 + 0 改 V0.5 30 维 + 0 改 6 重守门 v7 + 0 主动 commit + 0 装 PASS 严守 + 0 主动 push
- **R8**: PHL-07 实施 跟 24 LOCKED 入口签名 0 改 冲突 → **缓解**: PHL-07 入口新增 1 个 (25 LOCKED 总数, per 决策 #74 A3)
- **R9**: 团队对 "不要怕复杂度" 哲学不适应 → **缓解**: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应
- **R10**: 整合 #6/7 commit 拍板 失败 → **缓解**: Mavis 自决 (per 决策 #62 + 决策 #33 C1 + 决策 #71 §2.5), 8 步 verify 100% 后拍板
- **R11**: V1.1 release 实战 (估 2026-11-30) 主人起床后手跑失败 → **缓解**: V1.1 release 7 步 runbook (8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署)
- **R12**: V1.1 release cargo test 三次 verify 失败 → **缓解**: 阶段性 改写 + cargo test 三次 verify (cargo build + cargo test + cargo test) + 8 步 verify 100%

### 11.2 决策原则 (per R131-3 §3 决策原则 + 决策 #74 决策原则 + 用户记忆 #1-#10)

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写 + B2 1.2.1 bump** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74 B1 改写)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.2.1** (per 决策 #22 §2.2 + 决策 #74 B2 改写)
- **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only + V1.1 release 实施
- **B3 V0.5 30 维**: 严守 (哲学)
- **B4 6 重守门 v7**: 严守 (哲学守门)
- **B5 8 哲学锚**: 严守 (哲学)
- **C1 0 主动 commit**: 严守 (Mavis 拍板, 0 主动 push)
- **C2 0 装 PASS**: 严守 (技术哲学)
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md)
- **整合 #5/6/7 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **0 重复造轮子** (per 用户记忆 #6, R131-1/2/3/4/5/6/7/8/9 + R132-1/2 + R133-1/2/3 reference 不重写)
- **永久循环 / 0 终点** (per 决策 #71 §4, V1.1 release → V1.2 minor → V2.0 major → V2.1 minor → V3.0 major → ...)
- **16 跑中上限** (per 主人 0:34 拍板, 16 active 全 background 跑)

---

## 12. 0 主动 IM 主人 / 0 主动 commit / 0 主动 push / 0 改 src / 0 改 Cargo.toml 严守 100% (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10)

- **本次 done notification 主动报告** (R134-6 V1.1 release 后端加固 准备报告 done + V1.1 release 后端加固 8 方向 + 5 阶段计划 7 周 + Cargo.toml 1.2.0 → 1.2.1 bump + 24 LOCKED 入口签名 改写 + 12 源 0 装严守 二次 verify + pybridge 集成优化 + ASI Stage 9 终极自治 + 8 硬墙严守 + 8 哲学锚严守 + PHL-07 实施 + 不要怕复杂度哲学落地)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 V1.1 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 29.13 GB < 50 GB 保守策略)
- 0 改 src/ 严守 100% (R134-6 调研阶段 0 改 src, 0 触碰 crates/ 下任何 .rs 文件)
- 0 改 Cargo.toml 严守 100% (R134-6 调研阶段 0 改 Cargo.toml, B2 workspace.version 1.2.0 严守, V1.1 release 才 bump 1.2.1)
- 0 主动 commit 严守 100% (Mavis 整合 #5/6/7 commit 时机拍板, 0 主动 commit, per 决策 #33 §2.3 C1)
- 0 主动 push 严守 100% (等 1.0 release 配 GitHub remote + 主人起床后手跑, per 决策 #33 + 决策 #61 §6 + 决策 #71 §4.5)
- 0 主动 IM 主人 严守 100% (per gate-discipline + 决策 #61 §6 + cron Section 5, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- 0 借具体源码 严守 100% (per 决策 #33 §2.3 C2, 借脑 OpenCog 1:1 翻译公开模式)
- 8 硬墙 0 越界 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1, B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改, B2 1.2.0 V1.0 release 严守 + V1.1 release bump 1.2.1, A1 R11 baseline 3 值 严守, A3 12 键 + PHL-07 V1.0 spec-only + V1.1 实施, B3 V0.5 30 维 严守, B4 6 重守门 v7 严守, B5 8 哲学锚 严守, C1 0 主动 commit 严守, C2 0 装 PASS 严守, 0 push 严守)
- 0 重复造轮子 严守 100% (per 用户记忆 #6, R131-1/2/3/4/5/6/7/8/9 + R132-1/2 + R133-1/2/3 + 决策 #73 + 决策 #74 + 决策 #75 + 决策 #76 reference 不重写)

---

## 13. 写决策日志 (per 决策 #10 + 用户记忆 #10 + cron Section 6)

更新 `reports/decision-log-r129-era-cron-2026-08-11.md`:
- 时间戳: 2026-08-11 01:35 (cron 5 min tick 监督, R134-6 派活)
- 跑中任务数: 8 (R129-3 + R130-1 + R131-6/7/8/9 + R133-2/3) + 8 (R134-1~6 + R135-1/2) = 16 满 (per 决策 #76 §2.1)
- R134-6 报告内容: V1.1 release 后端加固 8 方向 + 5 阶段计划 7 周 + Cargo.toml 1.2.0 → 1.2.1 bump + 24 LOCKED 入口签名 改写 + 12 源 0 装严守 二次 verify + pybridge 集成优化 + ASI Stage 9 终极自治 + 8 硬墙严守 + 8 哲学锚严守 + PHL-07 实施 + 不要怕复杂度哲学落地
- 决策链更新: R134-6 done, R135-1/2 派活, 决策 #76 R134 era 派活拍板
- 派活 vs 整合 #5 commit 拍板 并行 (0 改 src 严守)
- 整合 #5 commit 时机 临近 ready (7/8 verify + R131-5 verify 24/24 LOCKED 入口签名 0 改 全部通过)

---

## 14. 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + cron Section 5)

- **本次 done notification 主动报告** (R134-6 V1.1 release 后端加固 准备报告 done, 估 60 min 时间盒, 01:35 估 done)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 V1.1 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ ~31 GB < 50 GB 保守策略)
- 整合 #5/6/7 commit 拍板 = done notification, 必须报告 (含 3 commit hash + master HEAD 新值 + 决策 #73/74/75/76 报告路径 + 新哲学文档 15-no-fear-complexity.md 路径)

---

## 15. 一句话 (再次强调)

**V1.1 release 后端加固 准备报告 (per 决策 #71 §2 R134 era 调研接续 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 B2 Cargo.toml 1.2.1 bump + R131-3 V1.1 路线图 §3 + R131-4 cargo workspace 优化 + R131-5 24 LOCKED 入口优化 + R131-6 Cargo.toml borrow 段精简 + R131-7 pybridge 集成优化 + R133-1 借鉴 12 源实施 + R133-2 ASI Stage 9 + R133-3 三洋葱架构升级 + 决策 #76 R134 era 6 sub 派活拍板)**: V1.1 release (估 2026-11-30 `v1.1.0`) 后端加固 = **8 大方向 详细 spec + 5 阶段实施计划 (7 周)** = ① Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #74 B2) ② 24 LOCKED 入口签名 改写 (per 决策 #74 B1 V1.1 release Mavis 自决改, 16/24 crate 改写 + 8/24 crate 0 改 + PHL-07 入口新增 1 个 = 25 LOCKED 总数 + AsiDispatcher 入口新增 1 个) ③ cargo test 三次 verify (cargo build + cargo test + cargo test, 1 周) ④ 12 源 0 装严守 二次 verify (8 真 cloned 沿用 + 2 借鉴 ID 索引完成沿用 + 1 永久跳过 + 1 借脑 ID 索引完成 = 12/12, 1 周) ⑤ pybridge 集成优化 + ASI Stage 9 终极自治 (13 NEW mod 总 42 mod, 4 处 PyO3 0.22 异步 + free-threading + smart_scopes + type hint union 可深化 + AsiDispatcher 协调器 + Stage 8 12 步 cycle + Stage 9 4 维度 H/L/G/P, 借脑 OpenCog CogPrime 1:1 翻译公开模式 0 借具体源码, 2 周) ⑥ cargo workspace 重构 (87 → 70-75 crate 优化, 3 transparent re-export 合并 + 12 借鉴模式统一 + 5 估补合并) ⑦ V0.5 30 维 严守 ⑧ 6 重守门 v7 严守 + 8 哲学锚严守 + PHL-07 实施. **5 阶段计划 7 周 (2026-10-15 ~ 12-03, 估 2026-11-30 V1.1 release tag 打上)**: 阶段 1 Cargo.toml 1.2.1 bump + 24 LOCKED 入口签名 改写 (2 周) + 阶段 2 cargo test 三次 verify (1 周) + 阶段 3 12 源 0 装严守 二次 verify (1 周) + 阶段 4 pybridge 集成优化 + ASI Stage 9 终极自治 (2 周) + 阶段 5 8 哲学锚 + PHL-07 实施 + 6 重守门 v7 + V0.5 30 维 集成 (1 周, 含 3 天 buffer). **整合 #6 commit 估 2026-11-25 + 整合 #7 commit 估 2026-11-29 + V1.1 release 实战估 2026-11-30 06:00-08:00 主人手跑**. **0 装 PASS 严守 100%** + **8 硬墙严守 100%** + **8 哲学锚严守 100%** + **PHL-07 实施** + **不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md). **0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%**.
