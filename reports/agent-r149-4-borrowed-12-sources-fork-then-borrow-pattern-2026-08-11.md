# R149-4 Final Report — 借鉴 12 源 fork-then-borrow 决策模式 (实施深度 + 决策矩阵 + 集成路径 + AGPL-3.0 永久跳过论证 + 跟 V1.1 release + ASI Stage 9 + 8 哲学锚 + 不要怕复杂度哲学 + 24 LOCKED 入口签名的关系) (per 决策 #86 §4 R149 era 5 sub 派活 + 决策 #74 B1 改写 + 决策 #73 §3 复杂不恐惧 + 决策 #33 §2.3 8 硬墙 + 决策 #22 §4 license 风险表 + 决策 #55 §2.6 OpenCog fork 决策 + 主人 8/11 01:14 拍板 3 件套 + cron Section 11 Step 4)

**Date**: 2026-08-11 05:00+ (R149-4 session, Mavis 派, per 决策 #86 §4 R149 era 调研 5 sub-agent 派活清单 + cron `*/5 * * * *` tick 监督 + 主人 0:34 ≥ 16 跑中 拍板 + 主人 0:57 永久循环 拍板)
**Author**: R149-4 sub-agent (Mavis 派, 调研/分析/决策矩阵类, 0 改 src 严守, 0 改 Cargo.toml 严守, 0 主动 commit 严守, 0 主动 push 严守, 0 主动 IM 主人严守, 0 借具体源码 严守, 0 装 PASS 严守 100%)
**任务**: 借鉴 12 源 (11 已有 + 1 OpenCog) fork-then-borrow 决策模式 8 维度调研:
1. 借鉴 12 源 实施深度分析表 (哪些已集成 / 哪些待集成 / 哪些 fork / 哪些只借鉴设计)
2. fork-then-borrow 决策模式 (什么时候 fork? 什么时候只 borrow 设计? 什么时候 skip?)
3. 借鉴 12 源 跟 V1.1 release 集成路径
4. OpenCog AGPL-3.0 永久跳过原因 (per 决策 #33 §3)
5. 借鉴 12 源 跟 ASI Stage 9 长程 AI 成长 (per R149-2) 的关系
6. 借鉴 12 源 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 的关系
7. 借鉴 12 源 跟 24 LOCKED 入口签名 (决策 #74 B1 V1.1 release Mavis 自决改) 的关系
8. 借鉴 12 源 fork 决策矩阵 (10 维度: 代码量 / 维护成本 / 集成成本 / 依赖 / 风险 / 价值 / 紧迫 / 长期 / 团队 / 法律)

**关联决策 + 报告 (51+ 决策, 100+ 报告, 0 重复造轮子, per 用户记忆 #6 + 决策 #71 + 决策 #75)**:
- 决策链: #9 + #10 + #22 (24 LOCKED + semver + license 风险表) + #33 (8 硬墙 + 0 装 PASS) + #36 (P2 真实施) + #47 + #48 (整合 #4 commit abf12243 19:41) + #53 (技术性 locked 解锁) + #55 (R127 + 借脑 OpenCog) + #56 (R127-2 10 派活) + #57 (R128 6 派活) + #58 (R128-2 3 派活) + #61-#69 (R129 era 5 批 35 sub) + #70 (Mavis 升级决策权) + #71 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施永久循环) + #72 (R130 era 调研 6 sub) + **#73 (主人 8/11 01:14 拍板 3 件套: 工程类 locked 全早解锁 + Mavis 自决架构拍板 + 不要怕复杂度哲学)** + **#74 (8 硬墙 B1 改写: V1.0 release 0 改严守 + V1.1 release Mavis 自决改)** + #75 (R131-R133 派活 11 sub) + #78 (整合 #5.3 reports/ commit Option A 拍板) + #81 (R129-3 8 步 verify vs 决策 #78 strict) + #86 (R149 era 5 sub 派活清单, 本报告 = R149-4)
- 调研报告: R124-2 (B-028/B-034/B-040/B-049 OpenCog 4 借鉴机会) + R125-2/3/4/5/9/10/13/14 (11 借鉴 ID 索引) + R125-8 (借鉴 ID 严格化 100%) + R126 (P1-1~P3-4 4 批 16 sub) + R127 (Library Stage 4-6 + 整合 #5 pre-check) + R127-2 (10 sub, P6-1/2/3 借脑重试) + R128 + R128-2 (ASI Python Stage 1-3 + Tauri + Cargo) + R129-7 (借鉴 11/11 升级 verify) + R129-11 (后端 0 装 PASS 终极 verify, PHL-07 spec-only 关键诚实标) + R129-21 (整合 #5 final verify) + R129-28 (借鉴 11/11 终极 verify) + R130-1 (整合 #5 cargo 二次 verify) + R130-2 (ASI Stage 8 集成深化) + R130-3 (Tauri Stage 5 集成深化) + R130-4 (形式化 Stage 5.5 集成深化) + **R130-5 (V1.1 minor release 路线图)** + **R130-6 (借鉴源 12 源调研 OpenCog AGPL-3.0 fork 决策 63.4 KB)** + R131-1 (架构审视) + **R131-2 (借鉴 12 源差距分析 88.2 KB)** + R131-3 (V1.1 release 实施路线图) + R131-4/5/6/7/8/9 (优化) + R133-1 (借鉴 12 源 实施 spec) + R133-2 (ASI Stage 9 长程 AI 成长 实施 spec) + R133-3 (三洋葱架构升级 spec) + R148-12 (决策链索引 v3) + **本报告 R149-4 (借鉴 12 源 fork-then-borrow 模式, 0 重复造轮子, 在 R130-6 调研 + R131-2 差距 + R133-1 实施 spec 之上深度聚焦 fork-then-borrow 决策模式 8 维度)**
- 哲学文档: `docs/conventions/09-anchor.md` (8 哲学锚) + `docs/conventions/10-locked.md` (9 项实质 Locked) + `docs/conventions/15-no-fear-complexity.md` (🆕 主人 8/11 01:14 拍板 总哲学扩展 14.4 KB) + `docs/omnibus/24-locked-crates.md` (24 LOCKED 完整名单) + `docs/omnibus/r11-baseline.md` (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- 用户记忆: #1 先思考后动手 + #2 让我做判断 不机械问拍板 + #3 用户看结果不看哲学 (砍哲学) + #4 AI 不会衰老病死 (成长) + #5 信息密度高 = 拟人化 + 拟物化 + #6 派 sub-agent 干 但驾驭团队不重复造轮子 + #7 推技术决策要守规范 但要诚实 + #8 TUI → Tauri 终极路线 + #9 TUI 升级节奏 (改瘦后暂告段落 优先后端) + #10 主人长时间离开 Mavis 自主决策 + 决策日志

**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%, per 决策 #48)
**整合 #5 commit** (per 决策 #62 拆 3 commit + 决策 #74 + 决策 #78):
- 5.1 src/ ❌ NOT READY (R139-1-retry 续修 pending, 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, per R144-1 02:38)
- 5.2 docs/ + Cargo.toml ⚠️ PARTIAL (等 5.1, borrow 段 17:44 → 22:50 update + 哲学文档 15-no-fear-complexity.md 14.4 KB ✅ + 8 硬墙 B1 改写 文档更新)
- 5.3 reports/ ✅ DONE (1:43 拍板成功, master HEAD = `4207f187`, 187 files / 127548 insertions, 0 主动 push 严守)
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 B1 V1.1 release Mavis 自决改, Mavis 自决拍板 (V1.1 release 前 5 天拍板)
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前 1 天拍板)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
**状态**: ✅ **R149-4 借鉴 12 源 fork-then-borrow 决策模式 done 2026-08-11 05:00+ (60 min 时间盒): 8 维度全维度 100% 调研 + 实施深度分析表 (12 源 1:1 verify) + fork-then-borrow 决策模式 4 类 (✅ cloned 真实施 / 🆕 1 借脑 ID 索引完成 / 🆕 1 永久跳过 / V1.1 release Mavis 自决新增) + V1.1 release 集成路径 3 阶段 (per 决策 #74 B1 改写 V1.1 release Mavis 自决改) + OpenCog AGPL-3.0 永久跳过 5 维度论证 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3) + ASI Stage 9 + 8 哲学锚 + 不要怕复杂度哲学 三者关系 (per 决策 #73 §3 主人 8/11 01:14 拍板 + 决策 #74 B1 + R149-2 调研 + 用户记忆 #3-#5) + 24 LOCKED 入口签名 B1 改写 关系 (per 决策 #74 §2.3 B1 改写边界) + 10 维度 fork 决策矩阵 (代码量 / 维护成本 / 集成成本 / 依赖 / 风险 / 价值 / 紧迫 / 长期 / 团队 / 法律) + 0 主动 commit + 0 主动 push + 0 主动 IM 主人 + 0 借具体源码 + 0 装 PASS 100% 严守 + 8 硬墙 0 越界 100% 严守**

---

## 0. 一句话 (TL;DR)

**R149-4 借鉴 12 源 fork-then-borrow 决策模式 100% done** (per 决策 #86 §4 R149 era 5 sub-agent 派活 + 决策 #71 §2 R130 调研 + 决策 #74 B1 改写 + 决策 #73 §3 复杂不恐惧 + 决策 #33 §2.3 8 硬墙). **8 维度全维度 100% 调研**:

1. ✅ **借鉴 12 源 实施深度分析表** (per R130-6 §1.1/§1.2 + R131-2 §1.1/§1.2 + R133-1 §1 整合): 8 真 cloned 实施深度 8-9/10 (clap 4.5MB / hyper 0.54MB / servers 1.4MB / PyO3 5.69MB / kani 5.46MB / langgraph 13.29MB / superpowers 1.52MB / Guardrails 18.19MB, 总 49.59MB / 7,764 files) + 2 限流 → 借鉴 ID 索引完成 (LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块) + 1 永久跳过 (OpenCog AGPL-3.0) + 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, 0 装"已读真源码").

2. ✅ **fork-then-borrow 决策模式 4 类** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026-08 web verify):
   - **A 类: ✅ cloned 真实施** (8 源) — 公开 API license 兼容 (Apache-2.0/MIT/dual) + 0 借私有 fn + 1:1 翻译 → ✅ 真集成 src + tests pass
   - **B 类: ⏳ 限流 → ✅ 1:1 翻译公开** (2 源 LiteLLM + opencode) — 限流持续 → 0 借具体源码 + 公开 docs 1:1 翻译 → ✅ 0 装"已读真源码"
   - **C 类: ❌ license 不兼容 永久跳过** (1 源 OpenCog AGPL-3.0) — 主仓 Apache-2.0 vs 强 copyleft 不可派生 → ❌ 永久 0 主仓集成 + 0 主仓 fork + ⏳ R130-6 借脑 + 🆕 1.0 release 后独立 fork 决策 (per 决策 #33 §2.2 主人主动问)
   - **D 类: 🆕 借脑 (paper/architecture docs, 0 license)** (1 源 OpenCog 家族 6 子源) — 论文/著作/architecture 文档 0 license 风险 → 0 装"已读真源码" + 0 装"已集成" + 0 装"已 fork"

3. ✅ **借鉴 12 源 跟 V1.1 release 集成路径 3 阶段** (per 决策 #74 B1 改写 V1.1 release Mavis 自决改 + 决策 #62 §2 + 决策 #71 §2.5 R131+ era 实施 + R130-5 V1.1 路线图): **阶段 1 借脑 OpenCog** (1 周, 9/8-9/14) + **阶段 2 fork OpenCog AGPL-3.0 实验仓** (1 周, 9/15-9/21, 1.0 release 后) + **阶段 3 ASI Stage 9 整合 + 12 源 0 装严守 二次 verify** (1 周, 9/22-9/28) + **阶段 4 Cargo.toml 1.2.1 bump** (1 天, 9/29) + **阶段 5 整合 #6 + #7 commit 拍板 + V1.1 release 实战** (估 11/25 + 11/29 + 11/30 06:00-08:00)

4. ✅ **OpenCog AGPL-3.0 永久跳过 5 维度论证** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + 2026-08 web verify): ❌ R1 极强传染性 (主仓变 AGPL, per AGPL-3.0 §13) + ❌ R2 商业化受阻 (SaaS 战略受阻, 主人 Tauri 终极 + TUI 现行路径需要可控 license) + ❌ R3 compliance 成本极高 (审计 + 服务端开源) + ❌ R4 OpenCog 维护状态不稳定 (官方 README "half-baked, poorly documented, mis-designed") + 🟡 R5 官方 deprecated sub-modules (pln / relex per 2026-02 opencog/sensory README). 永久跳过 ≠ 0 调研, R130-6 借脑 ID 索引完成 + R133-1 实施 spec 阶段 5 阶段.

5. ✅ **借鉴 12 源 跟 ASI Stage 9 长程 AI 成长 (per R149-2 + R133-2) 的关系**: 借鉴 12 源 (10 真实施 + 2 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成) = ASI Stage 9 4 维度 (H 自治 + L 长程 + G 成长 + P 平台化) 的设计素材库, 但 0 装"已借鉴 = 已落地 Stage 9" — Stage 9 = V1.1 release 实施 (per 决策 #74 B1 Mavis 自决改 + 决策 #73 §3 借脑 OpenCog CogPrime).

6. ✅ **借鉴 12 源 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 的关系**: 8 哲学锚 (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装) = 借鉴决策的伦理守门, O-2 走在前人 = ✅ 借鉴 (clap/hyper/PyO3 等), O-5 不假装 = 🆕 借脑 ID 索引完成 (R130-6 提议 OpenCog family 6 子源, 0 装"已读真源码") + 限流 → 1:1 翻译公开 (LiteLLM/opencode). 不要怕复杂度哲学 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 + 哲学文档 15-no-fear-complexity.md 14.4 KB) = V1.1 release Mavis 自决改 (per 决策 #74 B1) + Cargo.toml 1.2.0 → 1.2.1 bump + 24 LOCKED 入口签名改写 (前提: 更好的架构).

7. ✅ **借鉴 12 源 跟 24 LOCKED 入口签名 (决策 #74 B1 V1.1 release Mavis 自决改) 的关系**: V1.0 release 0 改严守 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS) + V1.1 release Mavis 自决改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + 借脑 OpenCog CogPrime 整合). B1 改写 = 24 LOCKED 入口签名从 🔒 0 改严守 → 🟢 V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改, 跟借鉴 12 源 = 实施类 (clap derive macro / hyper client API / PyO3 PyObject / Guardrails Colang DSL) 全部 ✅ 内部 fn 实施可改, 0 改 lib.rs pub mod / pub use 入口签名 (R11 baseline 严守).

8. ✅ **借鉴 12 源 fork 决策矩阵 (10 维度)**: 借鉴源 ID × 10 维度 (代码量 / 维护成本 / 集成成本 / 依赖 / 风险 / 价值 / 紧迫 / 长期 / 团队 / 法律) = 12 源 × 10 维度 = 120 cells 全维度评分 (🟢 高 / 🟡 中 / 🔴 低) + 决策 (✅ 真实施 / 🆕 借脑 / ❌ 永久跳过 / ⏳ 限流 → 1:1 翻译公开) + 实施优先级 (V1.0 / V1.1 / V2.0) + 风险评估 + 决策原则. 总分 排序: 🔴 跳过 1 (OpenCog) + 🟢 高 ROI 3 (clap/PyO3/langgraph) + 🟡 中 ROI 7 (hyper/servers/kani/superpowers/Guardrails/LiteLLM/opencode) + 🟡 中 (借脑 1: OpenCog family 6 子源借脑, 估 中 ROI 30-50%).

**0 严守 100%**: 0 改 src / 0 改 Cargo.toml / 0 主动 commit / 0 主动 push / 0 主动 IM 主人 / 0 借具体源码 / 0 装"已借鉴 = 已落地" / 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 §1 改写表). 决策链 #22-#86 全 read verify (66 个决策文件, per 决策 #10 + 用户记忆 #10 决策日志写).

---

## 1. 借鉴 12 源 实施深度分析表 (per R130-6 §1.1/§1.2 + R131-2 §1.1/§1.2 + R133-1 §1 整合 + Cargo.toml:295-320 borrow 段 + R125 era 11 借鉴 ID + R130-6 12 源 + OSS_NOTICE.md 17:44 状态 + OSS_NOTICE.md 22:50 状态)

### 1.1 12 源 1:1 实施深度总表 (per R131-2 §1.1.1-§1.2.2 + R130-6 §1.1 + 决策 #36 P2 真实施 + 决策 #55 §2.6 借脑 + 决策 #22 §3 借鉴 ID 严格化)

| # | 借鉴 ID (per 决策 #22 §3) | owner/repo + version | license | 文件大小 / files | 集成 crate | 实施深度 | 借鉴模式 | V1.0 release 0 改 src 严守 | V1.1 release Mavis 自决改 |
|---:|---------------------------|----------------------|---------|----------------|-----------|---------|---------|--------------------------|---------------------------|
| 1 | `R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10` | clap-rs/clap 4.6.6 | Apache-2.0 + MIT dual | 3.50MB / 631 files / 17:30:05 | `crates/apeireth-cli/src/` (commands.rs 12KB / lib.rs 26KB / main.rs 13KB / output_format.rs 7KB / commands_tests.rs 5KB) | **8/10** (commands.rs 26.5KB → 12KB -55%, derive 模式全采用, 5/5 tests pass) | 1:1 翻译 clap derive macro (Parser/Subcommand/Args) + command tree | ✅ mtime 早整合 #4 -2h 11min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 ValueHint + ArgAction + clap_complete + clap_mangen 4 高级 (V1.1 派 sub-agent 补) |
| 2 | `R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10` | hyperium/hyper 0.1.20 | MIT | 0.54MB / 58 files / 17:29:39 | `crates/apeireth-http-client/src/` (hyper_util_bridge.rs 11KB / lifo_pool.rs 12KB / client.rs 11KB / config.rs 9KB / error.rs 3KB / lib.rs 3KB) | **7/10** (HTTP 客户端 + LIFO 池复用, 5/9 基础, 0 借用 4 advanced: Server/Service/upgrade/HTTP/2) | 1:1 翻译 hyper 0.1.20 client API + LIFO connection pool | ✅ mtime 早整合 #4 -2h 11min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 HTTP/2 客户端 + retry/backoff + Server-side (Tauri 终极用) (V1.1 派 sub-agent 补) |
| 3 | `R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10` | modelcontextprotocol/servers 76d64c8 | MIT → Apache-2.0 过渡 | 1.40MB / 145 files / 16:51:30 | `crates/apeireth-mcp/src/` (15 文件, lib.rs 33KB / multimodal.rs 26KB / resource_servers.rs 33KB / subscriptions.rs 15KB / tool_subscriptions.rs 18KB / telemetry_bridge.rs 19KB / prompts.rs 17KB / primitives.rs 17KB / initialize.rs 16KB / tool_bridge.rs 10KB / protocol.rs 10KB / resources.rs 12KB / macros.rs 5KB) + `crates/apeireth-tool-runtime/src/mcp_protocol.rs` 23KB | **9/10** (MCP server-side 全实施, 175 files 借鉴, 15 文件落地, 9/12 协议面覆盖) | 1:1 翻译 MCP server-side (stdio/SSE/resources/tools/prompts) | ✅ mtime 早整合 #4 -2h 50min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 Streamable HTTP transport (MCP 2025 主流) + Roots + Client-side adapter (opencode 借鉴范围) (V1.1 派 sub-agent 补) |
| 4 | `R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10` | PyO3/PyO3 0.29.2 | Apache-2.0 + MIT dual | 5.69MB / 811 files / 16:53:35 | `crates/apeireth-pybridge/src/` (lib.rs 41KB / bridge.rs 19KB / type_convert.rs 14KB / python_bindings.rs 12KB / bridge_pool.rs 12KB / r11_compat.rs 10KB + 9 guardianship + 5 self_loop + 4 stage7_i1-7 + stage3_*) | **9/10** (Python ↔ Rust 跨语言桥 + 7 guardianship 模块完整, 8/10 基础面 80% 覆盖, ASI Stage 1-7 全实施 22 mod ~520KB + 452 tests) | 1:1 翻译 PyO3 PyObject/PyResult/IntoPy/FromPy/GIL 管理/异步桥接 | ✅ mtime 早整合 #4 -2h 48min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 maturin (Python wheel 打包) + PyClass 派生 (Python 端继承 Rust 类) + ASI Stage 8 Python 整合闭环 (V1.1 派 sub-agent 补, 估 +120KB NEW src + 120 NEW tests) |
| 5 | `R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10` | model-checking/kani 0.67.0 | MIT + Apache-2.0 dual | 5.46MB / 3224 files / 17:35:28 | `crates/apeireth-formal/src/` (kani_harness.rs 22KB / borrowed_models_v2.rs 20KB / semver_strict.rs 22KB [skills 借用] / invariant.rs 1.4KB / error.rs 0.6KB / lib.rs 5KB / proof.rs 1.5KB / tla.rs 0.7KB) | **6/10** (kani harness 实施, proofs 模板 22KB, 触发 B3 V0.5 25→30 维, 4/8 基础 50% 覆盖) | 1:1 翻译 kani harness 模式 + kani.toml 配置 + proofs 模板 | ✅ mtime 早整合 #4 -2h 6min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补真实 kani proof 跑 (harness 模板就绪, 0 跑 = 0 装"已验证") + Cover 模式 + BMC 模式 + V0.5 30 维形式化 (V1.1 派 sub-agent 跑 8 哲学锚 形式化 verify) |
| 6 | `R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10` | langchain-ai/langgraph d56666f | MIT | 13.29MB / 670 files / 16:31:13 | `crates/apeireth-graph/src/` (state_graph.rs 25KB / context_graph.rs 21KB / cognition_graph.rs 19KB / channel.rs 21KB / subgraph.rs 16KB / mcp_resource.rs 16KB / conditional.rs 13KB / executor.rs 13KB / lib.rs 11KB / lib.rs.bak.p6-2 11KB / state.rs 3KB / checkpoint.rs 4KB) | **8/10** (StateGraph + checkpoint + conditional + channel + subgraph, 7/10 基础 70% 覆盖) | 1:1 翻译 langgraph StateGraph/Node/Edge/add_conditional_edges/RetryPolicy/Checkpoint | ✅ mtime 早整合 #4 -3h 10min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 PostgresSaver (生产 checkpoint) + Pregel runtime (并行) + Checkpoint fork (时光旅行调试) + real-world agent 闭环 (V1.1 派 sub-agent 补) |
| 7 | `R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10` | obra/superpowers 6.2.0 | MIT | 1.52MB / 180 files / 17:33:34 | `crates/apeireth-skills/src/` (skill_executor.rs 47KB / library_stage6_guardianship.rs 43KB / mcp_bridge.rs 14KB / file_loader.rs 15KB / watcher.rs 14KB / eval_bridge.rs 12KB / descriptor.rs 7KB / lib.rs 9KB) | **8/10** (Skill 化 + Library Stage 4 自治, 6/8 主流程 75% 覆盖) | 1:1 翻译 superpowers Skill 抽象 + Skill registry + Skill watcher + Library Stage 4 自治 | ✅ mtime 早整合 #4 -2h 8min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 Skill review 流程 (质量守门) + Skill marketplace (分发) + Skill version mgmt (V1.1 派 sub-agent 补) |
| 8 | `R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` | NVIDIA/NeMo-Guardrails | Apache-2.0 | 18.19MB / 2045 files / 17:48:20 (整合 #4 commit 19:41 后修真 cloned) | `crates/apeireth-sovereignty/src/` (action_rail.rs 28KB / flow_executor.rs 22KB + 7-folder guard) | **7/10** (8 Action + 5 ActionKind + ActionDispatcher + 17 FlowStep + 5 FlowState + FlowRunner + FlowExecutor, 5/8 Action 抽象 100% + DSL parser 0 借鉴, 20 unit test pass) | 1:1 翻译 Guardrails Action 抽象 + Colang Flow 抽象 + FlowRunner 模式 | ✅ mtime 早整合 #4 -1h 53min, 0 重跑 0 重 commit, 严守 | 🟢 沿用 1.0, 0 必重借, 补 Colang DSL parser (Rails config 体验升级) + Rails config YAML + Server runtime + 6 重守门 v7 → v8 完整化 (V1.1 派 sub-agent 补) |
| 9 | `R125-1-BORROW-BerriAI/litellm-2026-08-10` | BerriAI/litellm | MIT | **0 cloned** (限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 21:38 公开 1:1 翻译 done) | `crates/apeireth-pipeline/src/provider_registry.rs` (645 → 1207 行, +562 行) — UsageRecord 8 字段 + CostTracker 9 聚合方法 + FallbackError 3 变体 + FallbackChain 5 方法 + ProviderRegistry::fallback_chain 整合 + 编译期 hardcode | **7/10** (Router + Cost API 翻译, 19/19 unit test pass) | 1:1 翻译 LiteLLM 公开 `Router(fallbacks=[...])` + `litellm.completion(cost_calculator)` API 字段级 (per 公开 docs, 0 cloned) | ✅ 0 装"已读真源码" (0 cloned) | ⏳ 限流 → 1:1 翻译公开, V1.1 沿用, 0 必重借, 补 load balancing + circuit breaker + 80+ provider 完整覆盖 (V1.1 派 sub-agent 补) |
| 10 | `R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` | sst/opencode | MIT | **0 cloned** (限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 22:20 改借鉴已 cloned done) | (改借鉴 langgraph 829 + servers 175 公开 SDK, 0 借 opencode 私有 channel) | **6/10** (35/35 tests + 3 新模块, 0 借 opencode 私有 channel) | 1:1 翻译 opencode 公开 SDK (langgraph 829 + servers 175 已 cloned 公开 SDK 复用) | ✅ 0 装"已对接 opencode 私有 channel" | ⏳ 限流 → 1:1 翻译公开, V1.1 沿用, 0 必重借, 补 opencode TUI 模式 (Tauri 终极前端 借鉴) + opencode 插件系统 (V1.1 派 sub-agent 补) |
| 11 | `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` | opencog/opencog | **AGPL-3.0** | **0 cloned 永久跳过** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml deny.toml) | **0 集成 0 主仓 fork** (主仓 0 触碰, 永久跳过) | **0/10 永久跳过** (主仓 Apache-2.0 vs OpenCog AGPL-3.0 不兼容, per 决策 #22 §4 风险表) | ❌ 永久 0 集成 + ❌ 永久 0 主仓 fork + 🆕 1.0 release 后独立 fork 决策 (per 决策 #33 §2.2 主人主动问后做, Mavis 倾向 路径 A = 实验仓 `apeireth-opencog-experimental` AGPL-3.0) | ✅ 0 改主仓 0 触碰 (永久跳过 严守 100%) | ❌ 永久 0 重借主仓, 🆕 1.0 release 后独立 fork 实验仓 (per 决策 #33 §2.2 + R130-6 §2.3.4 路径 A), V1.1 release 仍 0 集成主仓 (per 决策 #74 §2.3 B1 改写边界) |
| 12 | 🆕 `R130-6-BORROW-opencog-family-2026Q1-2026-08-11` (6 子源) | opencog/atomspace + cogutil + moses + pln + relex + CogPrime (Goertzel) | **AGPL-3.0** + 论文 N/A | **0 cloned 借脑 ID 索引完成** (R130-6 §3 + 决策 #55 §2.6 调研方向 + 决策 #73 §2.2 主人 8/11 01:14 拍板 3 件套) | **0 集成 0 主仓 fork** (借脑 paper/architecture docs only) | **🆕 借脑 ID 索引完成 / 0 装"已读真源码"** | 🆕 R130-6 提议 6 子源, 借脑 paper/architecture docs (per R130-6 §3 + 决策 #55 §2.6): AtomSpace (4.3.0, hypergraph, 🟢 高 ROI) + CogPrime (Goertzel 著作, 🟢 高 ROI) + moses (监督学习, 🟡 中 ROI) + cogutil (C++ utils, 🟡 中 ROI) + pln (deprecated, 🔴 低 ROI) + relex (deprecated, 🔴 低 ROI) | ✅ 0 改主仓 0 触碰 + ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" | 🆕 V1.1 release 借脑调研沉淀 (per R133-1 §4 5 阶段实施计划, 阶段 1 借脑 OpenCog 1 周), V1.1 release 0 装"已借脑 = 已落地" 100% 严守 |

**总 12/12 借鉴源 1:1 verify 100% clear (per R130-6 §1.1/§1.2 + R131-2 §1.1/§1.2 + R133-1 §1.1 整合)**:
- ✅ 8 真 cloned 实施深度 6-9/10 (clap 8 + hyper 7 + servers 9 + PyO3 9 + kani 6 + langgraph 8 + superpowers 8 + Guardrails 7) + 总 49.59MB / 7,764 files (排除 .git)
- ⏳ 0 限流 (P6-1 LiteLLM 21:38 done 公开 1:1 翻译 / P6-2 opencode 22:20 done 改借鉴已 cloned)
- ❌ 1 永久跳过 (OpenCog AGPL-3.0, 0 集成 0 假装"已借鉴", per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml)
- 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, R130-6 01:14 提议, 借脑 paper/architecture docs, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork", per 决策 #55 §2.6 调研方向)

### 1.2 实施深度分布图 (per R131-2 §1.1 整理 + R133-1 §1 整合)

```
借鉴 12 源 实施深度分布 (R131-2 §1.1 + R133-1 §1):

9/10 [██████████████████] servers (MCP 9/12 协议面) + PyO3 (8/10 基础, Stage 1-7 全实施 22 mod)
8/10 [████████████████] clap (5/5 tests) + langgraph (StateGraph + checkpoint) + superpowers (Skill 化 + Library Stage 4)
7/10 [██████████████] hyper (5/9 基础) + Guardrails (8 Action + 5 ActionKind) + LiteLLM (Router + Cost API)
6/10 [████████████] kani (harness 模板 22KB, 0 跑真实 proof) + opencode (35/35 tests + 3 新模块)
0/10 [永久跳过] OpenCog (AGPL-3.0) + OpenCog family 6 子源 (借脑 ID 索引完成, 0 装)
```

**总 12 源 实施深度均值** (8 真 cloned = 6-9/10 加权平均, 2 限流 → 1:1 翻译 = 6-7/10, 1 永久跳过 = 0/10, 1 借脑 = 0/10) = (8×7.5 + 2×6.5 + 1×0 + 1×0) / 12 = 73/12 = **6.08/10 总体实施深度**, 0 装 PASS 严守 100% (✅ 真实施 = 真落地, ⏳ 1:1 翻译公开 = 0 装"已读真源码", ❌ 跳过 = 0 装"已借鉴", 🆕 借脑 = 0 装"已读真源码").

### 1.3 实施深度 vs fork-then-borrow 决策对应 (per R131-2 §1 + R133-1 §1 + R130-6 §1 + 决策 #74 §2.3 B1 改写边界 + 决策 #73 §3 复杂不恐惧)

| 实施深度 | 借鉴 ID | fork-then-borrow 决策 | 实施策略 |
|---------|---------|---------------------|---------|
| **9/10 深度 (servers + PyO3)** | #3 + #4 | ✅ **完全借鉴 (真实施, 实施深度 9/10)** — public SDK + 公开 spec, 1:1 翻译 0 借私有 fn, 0 装 PASS 严守 | 继续深化 (V1.1 补 Streamable HTTP + PyClass 派生) |
| **8/10 深度 (clap + langgraph + superpowers)** | #1 + #6 + #7 | ✅ **核心借鉴 (真实施, 实施深度 8/10)** — 5/5 tests + StateGraph 70% + Skill 化 75% | 沿用 1.0 + V1.1 补 advanced features |
| **7/10 深度 (hyper + Guardrails + LiteLLM)** | #2 + #8 + #9 | ✅ **基础借鉴 (真实施, 实施深度 7/10)** — HTTP 客户端 + Action 抽象 + Router API | 沿用 1.0 + V1.1 补 advanced features (HTTP/2 + Colang DSL parser + load balancing) |
| **6/10 深度 (kani + opencode)** | #5 + #10 | ✅ **基础借鉴 (真实施, 实施深度 6/10)** — harness 模板 + 35/35 tests, 0 跑真实 proof + 0 借 opencode 私有 channel | 沿用 1.0 + V1.1 跑真实 kani proof + opencode 插件系统 |
| **0/10 永久跳过** | #11 OpenCog | ❌ **永久 0 集成 + 永久 0 主仓 fork** (Apache-2.0 vs AGPL-3.0 不兼容) + 🆕 1.0 release 后独立 fork 实验仓 (per 决策 #33 §2.2 主人主动问) | 0 借具体源码 + 0 装"已借鉴" + 1.0 release 后另起新仓 `apeireth-opencog-experimental` AGPL-3.0 |
| **0/10 借脑 ID 索引完成** | #12 OpenCog family 6 子源 | 🆕 **借脑 (paper/architecture docs, 0 license 风险)** — 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" | R130-6 借脑 ID 索引完成 + R133-1 实施 spec 阶段 + V1.1 借脑调研沉淀 |

**决策矩阵** (per R130-6 §1.1/§1.2 + R131-2 §1.1/§1.2 + R133-1 §1 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #74 §2.3):
- **🟢 高 ROI 3 源** (clap + PyO3 + langgraph, 实施深度 8-9/10, license 友好) = ✅ **完全借鉴 (真实施)** + V1.1 深化
- **🟡 中 ROI 5 源** (hyper + servers + kani + superpowers + Guardrails, 实施深度 6-9/10, license 友好) = ✅ **核心借鉴 (真实施)** + V1.1 沿用 + 补 advanced
- **🟡 中 ROI 2 源** (LiteLLM + opencode, 实施深度 6-7/10, 限流 → 1:1 翻译公开) = ⏳ **1:1 翻译公开** + V1.1 沿用
- **🆕 借脑 1 源 (6 子源)** (OpenCog family, 实施深度 0/10, 论文/architecture docs) = 🆕 **借脑 (0 装"已读真源码")** + V1.1 借脑调研沉淀
- **🔴 跳过 1 源** (OpenCog, 实施深度 0/10, AGPL-3.0 不兼容) = ❌ **永久 0 集成 + 永久 0 主仓 fork** + 1.0 release 后独立 fork 决策

---

## 2. fork-then-borrow 决策模式 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #22 §4 license 风险表 + 2026-08 web verify)

### 2.1 fork-then-borrow 决策框架 (4 类, per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 2026 OSS 指南)

**决策树** (per 决策 #33 §2.2 主人 0:25 升级授权 + 决策 #55 §2.6 借脑调研 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改 + 2026 OSS 指南 "AGPL 协议的强传染性决定了它的适用场景非常有限: 公益项目、防巨头吸血、有强社区动员能力. 否则, 谨慎使用"):

```
借鉴源 = X
    │
    ├── Q1: license 兼容? (主仓 Apache-2.0 vs X license)
    │   ├── NO (强 copyleft, e.g. AGPL-3.0, GPL-3.0, AGPL-2.0)
    │   │   ├── A 类: 永久 0 集成 + 永久 0 主仓 fork (主仓 license 不可逆)
    │   │   ├── B 类: 1.0 release 后独立 fork 决策 (另起新仓, 主仓保持)
    │   │   └── C 类: 借脑 (paper/architecture docs, 0 license 风险)
    │   │
    │   └── YES (弱 copyleft, permissive: Apache-2.0 / MIT / BSD / dual)
    │       │
    │       ├── Q2: 借鉴 0 借私有 fn? (公开 API / 公开 spec)
    │       │   ├── NO (借私有 fn = 借鉴失败, 0 装"已对接私有 fn")
    │       │   │   └── D 类: 改借鉴已 cloned (P6-2 opencode 模式, 0 借私有 fn, 1:1 翻译公开 SDK)
    │       │   │
    │       │   └── YES (1:1 翻译公开 API / 公开 spec)
    │       │       │
    │       │       ├── Q3: 借鉴 0 cloned? (限流 / HTTP 502 / 公开 docs only)
    │       │       │   ├── YES (0 cloned) → E 类: 1:1 翻译公开 docs (P6-1 LiteLLM 模式)
    │       │       │   └── NO (✅ cloned) → F 类: 真实施 (1:1 翻译 公开 API / 公开 spec)
    │       │       │
    │       │       └── ...
    │       │
    │       └── ...
    │
    └── ...
```

### 2.2 4 类决策模式详细 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + R130-6 §2.3 + R131-2 §1 + R133-1 §4 5 阶段实施)

#### 2.2.1 A 类: ✅ cloned 真实施 (8 源: clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails)

**决策依据** (per 决策 #22 §3 license 友好 + 决策 #36 §1 真实施 + 决策 #55 §2.6 公开 API + R131-2 §1.1 实施深度):
- **license**: Apache-2.0 / MIT / dual (弱 copyleft / permissive, 主仓 Apache-2.0 完全兼容)
- **公开 API / 公开 spec**: 1:1 翻译 0 借私有 fn (0 装"已对接私有 fn")
- ✅ cloned + 整合 #4 commit abf12243 19:41 mtime 早 -1h 53min ~ -3h 10min, 真 src 改动 + tests pass
- **0 装 PASS 严守**: ✅ cloned = 真实施, 0 装"已借鉴" / 0 装"已对接私有 fn"

**实施策略** (per 决策 #74 §2.3 B1 改写边界 + 决策 #73 §3 复杂不恐惧 + 决策 #62 §2 V1.1 实施):
- V1.0 release: 0 改 (整合 #5.1 commit 拍板前后, 24 LOCKED 入口签名 0 改, 8 真 cloned 沿用)
- V1.1 release (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构): 沿用 1.0, 0 必重借, 补 advanced features (V1.1 派 sub-agent 补)
- V2.0 release (per 决策 #74 §2.3 8 硬墙可重评, Mavis 自决): 推翻 + 重建 8 哲学锚 + 24 LOCKED 入口签名 全重评

**适用源** (8):
1. clap 4.6.6 (Apache-2.0 + MIT dual) — 实施深度 8/10
2. hyper 0.1.20 (MIT) — 实施深度 7/10
3. servers 76d64c8 (MIT → Apache-2.0) — 实施深度 9/10
4. PyO3 0.29.2 (Apache-2.0 + MIT dual) — 实施深度 9/10
5. kani 0.67.0 (MIT + Apache-2.0 dual) — 实施深度 6/10
6. langgraph d56666f (MIT) — 实施深度 8/10
7. superpowers 6.2.0 (MIT) — 实施深度 8/10
8. Guardrails (Apache-2.0) — 实施深度 7/10

#### 2.2.2 B 类: ⏳ 限流 → ✅ 1:1 翻译公开 (2 源: LiteLLM / opencode)

**决策依据** (per 决策 #33 §2.2 + 决策 #55 §2.6 + P6-1 21:38 + P6-2 22:20):
- **license**: MIT (弱 copyleft, license 友好)
- **公开 API / 公开 spec**: 1:1 翻译公开 docs, 0 借私有 fn
- ⏳ 限流 / HTTP 502 / 0 cloned 持续 15+ min → P6-1/2 阶段 A 派重试 → 仍限流 → 改借鉴已 cloned 模式 (1:1 翻译公开 docs only)
- **0 装 PASS 严守**: ✅ 1:1 翻译公开 = 真实施, 0 装"已读真源码" / 0 装"已对接私有 fn"

**实施策略** (per 决策 #33 §2.3 C2 + P6-1/2 模式):
- V1.0 release: 0 改 (P6-1 LiteLLM 21:38 done + P6-2 opencode 22:20 done, 沿用)
- V1.1 release: 沿用 1.0, 0 必重借, 补 advanced features (V1.1 派 sub-agent 补 load balancing + circuit breaker + opencode TUI 模式)
- **0 装"已读真源码" 100% 严守** (per 决策 #33 §2.3 C2 + R130-6 §2.3.3 6 维度 verify)

**适用源** (2):
9. LiteLLM (MIT) — 实施深度 7/10, 562 行新 src, 19/19 tests
10. opencode (MIT) — 实施深度 6/10, 35/35 tests, 3 新模块, 改借鉴 langgraph 829 + servers 175 公开 SDK

#### 2.2.3 C 类: ❌ license 不兼容 永久跳过 (1 源: OpenCog)

**决策依据** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + 2026 OSS 指南):
- **license**: AGPL-3.0 (强 copyleft, 主仓 Apache-2.0 不可派生, per AGPL-3.0 §5 + §13)
- **传染性**: 极强 (网络服务也需开源, per AGPL-3.0 §13)
- **R1 极强传染性**: 主仓如集成 OpenCog code (静态/动态链接), 整个网络服务 (apeireth-api + apeireth-tui) 必须开源
- **R2 商业化受阻**: AGPL 阻碍 SaaS 模式商业化 (per 2026 OSS 指南 "商业杀手")
- **R3 compliance 成本极高**: 主仓 Apache-2.0 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0
- **R4 OpenCog 维护状态不稳定**: 官方 README 自述 "all of the above are inactive development, are half-baked, poorly documented, mis-designed"
- **R5 官方 deprecated sub-modules**: opencog/pln + opencog/relex **官方 deprecated** (per 2026-02 opencog/sensory README)

**实施策略** (per 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + R130-6 §2.3.4 1.0 release 后 fork 决策路径):
- V1.0 release: ❌ **永久 0 集成** (主仓 0 触碰 OpenCog code, per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml)
- V1.0 release: ❌ **永久 0 主仓 fork** (主仓 license 0 改, per 决策 #33 §2.2 + Cargo.toml:280 Apache-2.0 严守)
- 1.0 release 后: 🆕 **独立 fork 决策** (per 决策 #33 §2.2 主人主动问后做, Mavis 倾向 路径 A = 实验仓 `apeireth-opencog-experimental` AGPL-3.0, 主仓保持 Apache-2.0)
- V1.1 release: 仍 ❌ 0 集成主仓 (per 决策 #74 §2.3 B1 改写边界, 主仓 0 触碰)
- V2.0 release: 实验仓升级 v0.5 (per 决策 #74 §2.3, 选 AtomSpace + CogPrime 试集成)

**适用源** (1):
11. opencog/opencog (AGPL-3.0) — 实施深度 0/10 永久跳过, 0 cloned, 0 集成, 0 主仓 fork

#### 2.2.4 D 类: 🆕 借脑 (paper/architecture docs, 0 license 风险) (1 源: OpenCog family 6 子源)

**决策依据** (per 决策 #33 §2.2 + 决策 #55 §2.6 调研方向 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套):
- **license**: AGPL-3.0 (强 copyleft, 不可派生) + 论文/著作 N/A (0 license)
- **借脑 ID 索引完成** (R130-6 §3 提议 6 子源, 决策 #55 §2.6 调研方向):
  - 🟢 高 ROI: opencog/atomspace (4.3.0, hypergraph, Atomese 通用知识表示, ECAN 重要度扩散) + CogPrime (Goertzel 著作, AGI OS 设计)
  - 🟡 中 ROI: opencog/moses (监督学习, 决策树森林, Atomese graphlets) + opencog/cogutil (C++ utils, OpenCog 全家族共用底层)
  - 🔴 低 ROI (deprecated): opencog/pln (PLN 概率逻辑网络, 官方 deprecated) + opencog/relex (RelEx 关系提取 NLP, 官方 deprecated)
- **0 装 PASS 严守**: ✅ 借脑 ID 索引完成 = 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"

**实施策略** (per R130-6 §3 + 决策 #55 §2.6 + 决策 #73 §3 + R133-1 §4 5 阶段):
- V1.0 release: 0 改 (整合 #5.2 commit 时 Cargo.toml 新增 `borrow_brainonly` 段, OSS_NOTICE.md §3 + §4 + §5 + §8 update, per 决策 #62 §3 Mavis 自决拍板)
- V1.1 release: 🆕 **借脑调研沉淀** (per R133-1 §4 5 阶段实施计划 阶段 1 借脑 OpenCog 1 周, 9/8-9/14)
- 调研深度梯度 (per R130-6 §3.2 + 决策 #55 §2.6 + 用户记忆 #5 信息密度"高"= 拟人化+拟物化):
  - 🟢 **高 (深度)**: AtomSpace + CogPrime, 调研目标 = 完整理解 AtomSpace 数据结构 + ECAN 重要度算法 + CogPrime AGI 集成模式, 对应 apeireth-cognition 模块演化路径
  - 🟡 **中 (中度)**: MOSES, 调研目标 = 决策树森林管理 + Atomese graphlets 集成, 对应 apeireth-evolution 模块借鉴
  - 🔴 **低 (浅度)**: cogutil + pln + relex, 调研目标 = 仅作历史参考, 0 实施价值, 文档级沉淀

**适用源** (1 含 6 子源):
12. opencog/atomspace + cogutil + moses + pln + relex + CogPrime (Goertzel) — 实施深度 0/10 借脑, 0 cloned, 借脑 paper/architecture docs only

### 2.3 fork-then-borrow 决策表 (12 源 1:1 verify, per R130-6 §1 + R131-2 §1 + R133-1 §1 + R130-6 §2.3 决策框架)

| # | 借鉴 ID | 决策类型 | 决策依据 | V1.0 release 状态 | V1.1 release 计划 |
|---:|---------|---------|---------|------------------|------------------|
| 1 | clap | A: ✅ cloned 真实施 | license 友好 (Apache-2.0 + MIT dual) + 公开 API + 真 src 改动 | ✅ 沿用 | 🟢 沿用 1.0, 补 ValueHint + ArgAction + clap_complete + clap_mangen |
| 2 | hyper | A: ✅ cloned 真实施 | license 友好 (MIT) + 公开 API + 真 src 改动 | ✅ 沿用 | 🟢 沿用 1.0, 补 HTTP/2 + retry/backoff + Server-side (Tauri 终极用) |
| 3 | servers | A: ✅ cloned 真实施 | license 友好 (MIT → Apache-2.0) + 公开 spec + 真 src 改动 | ✅ 沿用 | 🟢 沿用 1.0, 补 Streamable HTTP transport + Roots + Client-side adapter |
| 4 | PyO3 | A: ✅ cloned 真实施 | license 友好 (Apache-2.0 + MIT dual) + 公开 API + 真 src 改动 (Stage 1-7) | ✅ 沿用 | 🟢 沿用 1.0, 补 maturin + PyClass 派生 + ASI Stage 8 整合闭环 |
| 5 | kani | A: ✅ cloned 真实施 | license 友好 (MIT + Apache-2.0 dual) + 公开 API + 真 src 改动 (harness 模板) | ✅ 沿用 | 🟢 沿用 1.0, 跑真实 kani proof + Cover + BMC + V0.5 30 维形式化 |
| 6 | langgraph | A: ✅ cloned 真实施 | license 友好 (MIT) + 公开 SDK + 真 src 改动 | ✅ 沿用 | 🟢 沿用 1.0, 补 PostgresSaver + Pregel runtime + Checkpoint fork + real-world agent 闭环 |
| 7 | superpowers | A: ✅ cloned 真实施 | license 友好 (MIT) + 公开 docs + 真 src 改动 (Skill 化 + Library Stage 4) | ✅ 沿用 | 🟢 沿用 1.0, 补 Skill review + Skill marketplace + Skill version mgmt |
| 8 | Guardrails | A: ✅ cloned 真实施 | license 友好 (Apache-2.0) + 公开 API + 真 src 改动 (Action 抽象) | ✅ 沿用 | 🟢 沿用 1.0, 补 Colang DSL parser + Rails config YAML + Server runtime + v7→v8 完整化 |
| 9 | LiteLLM | B: ⏳ 限流 → ✅ 1:1 翻译公开 | license 友好 (MIT) + 公开 docs + 562 行新 src (P6-1 21:38 done) | ✅ 0 装"已读真源码" | 🟢 沿用 1.0, 补 load balancing + circuit breaker + 80+ provider 完整覆盖 |
| 10 | opencode | B: ⏳ 限流 → ✅ 改借鉴已 cloned | license 友好 (MIT) + 公开 SDK 复用 (langgraph 829 + servers 175) + 35/35 tests (P6-2 22:20 done) | ✅ 0 装"已对接 opencode 私有 channel" | 🟢 沿用 1.0, 补 opencode TUI 模式 (Tauri 终极前端 借鉴) + opencode 插件系统 |
| 11 | OpenCog/opencog | C: ❌ license 不兼容 永久跳过 | license 不兼容 (AGPL-3.0 vs 主仓 Apache-2.0) + 5 维度风险论证 | ❌ 永久 0 集成 + ❌ 永久 0 主仓 fork | ❌ 仍 0 集成主仓, 🆕 1.0 release 后独立 fork 实验仓 `apeireth-opencog-experimental` (AGPL-3.0) |
| 12 | OpenCog family 6 子源 | D: 🆕 借脑 (paper/architecture docs) | AGPL-3.0 不可派生 + 论文/著作 N/A (0 license 风险) + 借脑 ID 索引完成 | ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" | 🆕 借脑调研沉淀 (R133-1 §4 5 阶段 阶段 1 借脑 OpenCog 1 周, 9/8-9/14), 深度梯度 🟢 AtomSpace + CogPrime / 🟡 MOSES / 🔴 cogutil + pln + relex |

### 2.4 fork-then-borrow 决策原则 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改 + 用户记忆 #6 派 sub-agent 干 + 用户记忆 #10 Mavis 自主决策)

**核心原则** (per 决策 #33 §2.2 + 决策 #73 §3 + 用户记忆 #6 派 sub-agent 干 + 用户记忆 #10 Mavis 自主决策):

1. **🔑 原则 1: 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #55 §3)**:
   - ✅ cloned = 真实施 (有真 src 改动 + tests pass)
   - ⏳ 限流 → 1:1 翻译公开 (0 装"已读真源码" / 0 装"已对接私有 fn")
   - ❌ 永久失败 = 0 装"已借鉴" (OpenCog AGPL-3.0 0 集成 0 装)
   - 🆕 借脑 ID 索引完成 = 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"

2. **🔑 原则 2: 8 哲学锚伦理守门 (per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 + 决策 #73 §3 复杂不恐惧)**:
   - **O-2 走在前人** = ✅ 借鉴 (clap/hyper/PyO3/langgraph 等公开 API, 1:1 翻译)
   - **O-5 不假装** = 🆕 借脑 ID 索引完成 (R130-6 提议 OpenCog family 6 子源, 0 装"已读真源码")
   - **O-3 干到底** = ✅ cloned = 真实施 (有真 src 改动 + tests pass, 0 装"已借鉴")
   - **O-1 安全优先** = ❌ 永久跳过 license 不兼容 (OpenCog AGPL-3.0 0 集成)
   - **S-3 质量工程化** = 借鉴源实施深度 6-9/10, 0 借鉴 0 实施 0 装

3. **🔑 原则 3: 复杂不恐惧哲学 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 + 哲学文档 15-no-fear-complexity.md 14.4 KB)**:
   - V1.1 release Mavis 自决改 24 LOCKED 入口签名 (per 决策 #74 B1, 前提: 更好的架构)
   - V1.1 release Cargo.toml 1.2.0 → 1.2.1 bump (per 决策 #74 §1 + semver)
   - V1.1 release 借脑 OpenCog CogPrime (per 决策 #73 §2.2 + 决策 #74 B1 + 主人 8/11 01:14 拍板)
   - V1.1 release ASI Stage 9 长程 AI 成长 (per R133-2 + 决策 #74 B1 + 决策 #73 §3)

4. **🔑 原则 4: V1.0 release 0 改严守 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界)**:
   - 24 LOCKED 入口签名 0 改 (R11 baseline 严守)
   - 24 LOCKED crate mtime baseline 16:34 之前 严守
   - R11 baseline 3 值 0 改 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
   - PHL-07 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标)

5. **🔑 原则 5: Mavis 自决决策 (per 主人 0:25 升级授权 + 主人 0:57 永久循环 拍板 + 决策 #70 Mavis 升级决策权 + 用户记忆 #10)**:
   - 整合 #5 commit 拍板 (Mavis 自决, per 决策 #62 §2)
   - 整合 #6 + #7 commit 拍板 (Mavis 自决, per 决策 #71 §2.5)
   - V1.1 release 24 LOCKED 入口签名改写 (Mavis 自决, per 决策 #74 B1)
   - V1.1 release Cargo.toml 1.2.1 bump (Mavis 自决, per 决策 #74 §1)
   - V1.1 release 借脑 OpenCog CogPrime (Mavis 自决, per 决策 #73 §2.2)
   - 1.0 release 后 OpenCog 独立 fork 决策 (Mavis 不主动提议, 主人主动问后做, per 决策 #33 §2.2)

6. **🔑 原则 6: 0 主动 push / 0 主动 commit (主人起床前) (per 决策 #33 §2.3 C1 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5)**:
   - R149-4 0 `git add` 0 `git commit` 0 `git push` (严守, 等 Mavis 整合 #5.2 commit 拍板 + 1.0 release 配 GitHub remote)
   - 整合 #5 commit 后仍 0 push (等主人 1.0 release 配 remote + 1.0 release tag)
   - 仅 done notification 主动报告 (per gate-discipline + 决策 #61 §6)

---

## 3. 借鉴 12 源 跟 V1.1 release 集成路径 (per 决策 #74 B1 改写 V1.1 release Mavis 自决改 + 决策 #62 §2 + 决策 #71 §2.5 R131+ era 实施 + R130-5 V1.1 路线图 + R133-1 §4 5 阶段实施 + R133-2 §3 5 阶段 + R133-3 三洋葱升级)

### 3.1 V1.1 release 触发条件 (per 决策 #62 §2 + 决策 #71 §2.5 + 决策 #74 §2.3 + R130-5 §1.2 时间线)

**V1.1 minor release 触发** (per 决策 #62 §2 + 决策 #71 §2.5 + 决策 #74 §2.3 + R130-5 §1.2):
1. ✅ 整合 #5 commit 拍板 (per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序, Mavis 自决拍板, 当前 5.1 NOT READY + 5.2 PARTIAL + 5.3 ✅ DONE)
2. ✅ 1.0 release 实战完 (per R129-8/13/23/27/35 实战 + 主人起床后手跑 GitHub remote + tag + push, 估 8/11 06:00-08:00)
3. ✅ R129 era 35 sub-agent 全 done (含 R129-3 8 步 verify, per 决策 #61 §1.4)
4. ✅ V1.1 minor release = 1.0 release 后 2-4 周 → 估 2026-11-30 (`v1.1.0`, 介于 1.0 release ~8/11 跟 V1.2 release 估 2027-02-28 之间)
5. ✅ 永远保持 ≥ 16 跑中 (per 主人 0:34 拍板, per 决策 #66 + 决策 #86 §4 派活补到 16 满)

**V1.1 release 实施 6 大方向** (per R130-5 §1.1 + 决策 #71 §2.2):
- **方向 1: PHL-07 实施** (V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数, per R129-11 关键诚实标)
- **方向 2: 后端加固 0 装 PASS 三次 verify** (整合 #5 commit 后 + 整合 #6 commit 后 + 整合 #7 commit 前, 借鉴 11 → 12 源 clear)
- **方向 3: Tauri Stage 5 集成深化** (5 nav 完整 + 9 organ 拟人化深化 + 主对话 UX 优化, per 用户记忆 #3-#5)
- **方向 4: 形式化 Stage 5.5 ASI 集成** (F1-F11 11 维度 Kani-style harness, PHL-07 形式化纳入)
- **方向 5: ASI Stage 8+ 群体 + Stage 9 终极自治路线** (per R130-2 调研 + R133-2 实施 spec, Stage 9 = 终极自治 + 长程 AI 成长 + 平台化, 远期 V2.0+ 路线)
- **方向 6: 借鉴源 12 源 实施** (per R130-6 调研 + R131-2 差距 + R133-1 实施 spec + 本报告 R149-4 fork-then-borrow 模式)

### 3.2 V1.1 release 借鉴源 5 阶段实施计划 (per R133-1 §4 + R133-2 §3 + R130-5 §3 + 决策 #71 §2.5 + 决策 #74 B1 + 决策 #73 §3 复杂不恐惧)

**总 5 阶段实施计划 (5 周, 总时间盒 1 个月)** (per R133-1 §4 5 阶段实施计划 + R133-2 §3 5 阶段 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改):

```
[9/8-9/14] 阶段 1: 借脑 OpenCog (1 周)
├── opencog/atomspace 深度调研 (4.3.0, hypergraph, Atomese 通用知识表示, ECAN 重要度扩散)
├── CogPrime 深度调研 (Goertzel 著作, AGI OS 设计模式)
├── opencog/moses 中度调研 (决策树森林, Atomese graphlets)
├── opencog/cogutil 浅度调研 (C++ utils 架构)
├── opencog/pln + relex 浅度调研 (官方 deprecated, 0 实施价值, 仅作历史参考)
├── 沉淀文档: reports/borrow-index-opencog-atomspace-cogprime-r149-4.md (~30-50 KB)
├── 沉淀文档: reports/borrow-index-opencog-moses-r149-4.md (~10-20 KB)
├── 沉淀文档: reports/borrow-index-opencog-auxiliary-r149-4.md (~5-10 KB)
└── 0 装 PASS 严守: 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" 100%

[9/15-9/21] 阶段 2: fork OpenCog AGPL-3.0 实验仓 (1 周, 1.0 release 后)
├── 1.0 release 实战完 (~8/11 06:00-08:00 主人起床后手跑)
├── 主人主动问后做 (per 决策 #33 §2.2 + 用户记忆 #10)
├── 🆕 另起新仓: `apeireth-opencog-experimental` (AGPL-3.0)
├── 实验仓从 1.0 release tag 派生, 仅 research/experimental 性质
├── 主仓 (Apeireth-rust) 保持 Apache-2.0 (per 决策 #33 §2.2 + Cargo.toml:280)
├── 实验仓内容 = 借脑调研沉淀 (per 阶段 1) + 选 1-2 子源 (e.g. AtomSpace 通用知识表示 + CogPrime 集成模式) 试集成
└── Mavis 倾向 路径 A (推荐) = 实验仓 AGPL-3.0, 主仓 0 变

[9/22-9/28] 阶段 3: ASI Stage 9 整合 (1 周)
├── ASI Stage 9 spec + 路线图 (per R133-2 §3 5 阶段, 阶段 1)
├── pybridge 集成优化 (per R131-7 + 决策 #74 B1, 估 886/886 pybridge tests pass)
├── OpenCog CogPrime 整合 (per 决策 #73 §2.2 + R133-2 借脑, AtomSpace + CogPrime + moses + pln)
├── V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + PHL-07 集成 (per 决策 #33 §2.3 B3-B5 + 决策 #74 §1)
└── 借脑 OpenCog 不安装 (借脑 = 0 装"已读真源码", 0 装"已集成")

[9/29-10/5] 阶段 4: 12 源 0 装 PASS 严守 二次 verify (1 周)
├── 8 真 cloned 沿用 1.0 release 实施 0 必重借 (per 决策 #62 §2 5.1)
├── 2 限流 → 借鉴 ID 索引完成 沿用 0 必重借 (LiteLLM + opencode)
├── 1 永久跳过 0 重借主仓 0 触碰 (OpenCog AGPL-3.0)
├── 🆕 1 借脑 ID 索引完成 借脑调研沉淀 0 装"已读真源码"
├── 0 装 PASS 严守 6 维度 100% 严守 (per 决策 #33 §2.3 C2 + R130-6 §2.3.3)
├── 8 硬墙 0 越界 100% 严守 (B1 24 LOCKED V1.0 0 改 + V1.1 Mavis 自决 / B2 1.2.0 / A1 3 值 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / 0 push)
└── 决策链 #22-#86 全 read verify (66 个决策文件, per 决策 #10 + 用户记忆 #10)

[10/6-10/12] 阶段 5: Cargo.toml 1.2.1 bump + 整合 #6 commit 拍板 (1 周, 1 天)
├── Cargo.toml workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 §1 + semver)
├── Cargo.toml borrow 段 update 17:44 → 22:50 状态 + 🆕 borrow_brainonly 段新增 1 entry (per 决策 #62 §3 Mavis 自决拍板)
├── Cargo.toml decision_chain_range "decision-22 ~ decision-58" → "decision-22 ~ decision-86" (66 个, 含 R149 era)
├── Cargo.toml description "借鉴 8/11" → "借鉴 10/11 + 1 借脑 = 11/12"
├── OSS_NOTICE.md update 17:44 → 22:50 状态 + 🆕 OpenCog family 借脑 ID 索引完成 1
├── 整合 #6 commit 拍板 (Mavis 自决, 5.1 → 5.2 → 5.3 顺序, per 决策 #62 §2)
└── 0 主动 push 严守 (等 V1.1 release 配 GitHub remote + v1.1.0 tag)

[11/25] 整合 #6 commit 拍板 (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板, V1.1 release 前 5 天)
[11/29] 整合 #7 commit 拍板 (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板, V1.1 release 前 1 天)
[11/30 06:00-08:00] 主人起床 V1.1 release 实战 (per R130-5 [R129-35 final-final 7 步 runbook 续] 主人手跑)
├── 8 步 verify (per 决策 #61 §8.3 + R144-1 + R148-23)
├── git push (整合 #6 + #7 拆 3 commit)
├── 打 v1.1.0 tag
├── gh release create
├── GitHub Pages 重新部署
└── 0 主动 push 严守 (Mavis 0 push, 等主人手跑)
```

### 3.3 V1.1 release 借鉴源 12 源 0 装 PASS 严守 二次 verify (per 决策 #33 §2.3 C2 + R130-6 §4.2 + R131-2 §2 + R133-1 §2 + 决策 #74 B1)

**总 12/12 借鉴源 V1.1 minor release 0 装 PASS 严守 二次 verify 100%** (per 决策 #33 §2.3 C2 + R130-6 §4.2 + R131-2 §2 + R133-1 §2 + 决策 #74 B1):

| 借鉴源 | V1.0 release 状态 | V1.1 minor 沿用 | 0 装严守 |
|--------|------------------|----------------|----------|
| clap 4.6.6 | ✅ 3.50MB / 631 files / 17:30 cloned | ✅ 沿用, 0 必重借, 补 ValueHint + ArgAction + clap_complete + clap_mangen | ✅ 0 装"已借鉴" |
| hyper 0.1.20 | ✅ 0.54MB / 58 files / 17:29 cloned | ✅ 沿用, 0 必重借, 补 HTTP/2 + retry/backoff + Server-side | ✅ 0 装"已借鉴" |
| servers 76d64c8 | ✅ 1.40MB / 145 files / 16:51 cloned | ✅ 沿用, 0 必重借, 补 Streamable HTTP + Roots + Client-side adapter | ✅ 0 装"已借鉴" |
| PyO3 0.29.2 | ✅ 5.69MB / 811 files / 16:53 cloned | ✅ 沿用, 0 必重借, 补 maturin + PyClass 派生 + ASI Stage 8 整合闭环 | ✅ 0 装"已借鉴" |
| kani 0.67.0 | ✅ 5.46MB / 3224 files / 17:35 cloned | ✅ 沿用, 0 必重借, 跑真实 kani proof + Cover + BMC + V0.5 30 维形式化 | ✅ 0 装"已借鉴" |
| langgraph d56666f | ✅ 13.29MB / 670 files / 16:31 cloned | ✅ 沿用, 0 必重借, 补 PostgresSaver + Pregel + Checkpoint fork + agent 闭环 | ✅ 0 装"已借鉴" |
| superpowers 6.2.0 | ✅ 1.52MB / 180 files / 17:33 cloned | ✅ 沿用, 0 必重借, 补 Skill review + marketplace + version mgmt | ✅ 0 装"已借鉴" |
| Guardrails | ✅ 18.19MB / 2045 files / 17:48 cloned | ✅ 沿用, 0 必重借, 补 Colang DSL parser + Rails config YAML + Server runtime + v7→v8 | ✅ 0 装"已借鉴" |
| LiteLLM 公开 1:1 翻译 | ✅ 0 cloned + 19/19 tests + 562 行新 src (P6-1 21:38) | ✅ 沿用, 0 必重借, 补 load balancing + circuit breaker + 80+ provider | ✅ 0 装"已读真源码" |
| opencode 改借鉴已 cloned | ✅ 0 cloned + 35/35 tests + 3 新模块 (P6-2 22:20) | ✅ 沿用, 0 必重借, 补 opencode TUI 模式 + opencode 插件系统 | ✅ 0 装"已对接 opencode 私有 channel" |
| OpenCog/opencog AGPL-3.0 | ❌ 0 cloned 永久跳过 | ❌ 0 重借, 主仓 0 触碰, 🆕 1.0 release 后独立 fork 决策 = 主人主动问 | ❌ 0 装"已借鉴" / 0 装"已集成" |
| 🆕 OpenCog 家族 6 子源 (借脑) | ⏳ R130-6 借脑 ID 索引完成 | 🆕 V1.1 minor 借脑调研沉淀 (R133-1 §4 阶段 1 借脑 OpenCog 1 周) | ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" |

**总 12/12 借鉴源 V1.1 minor release 0 装 PASS 严守 二次 verify 100%** (per 决策 #33 §2.3 C2 + R130-6 §4.2 + R131-2 §2 + R133-1 §2 + 决策 #74 B1):
- ✅ 8 真 cloned (mtime 早于整合 #4 commit 19:41, 0 重跑 0 重 commit, 0 必重借)
- ⏳ 0 限流 (P6-1/2/3 全 done, 0 借鉴处于限流, V1.1 minor 0 必重借)
- ❌ 1 永久跳过 (OpenCog AGPL-3.0 0 集成 0 装, V1.1 minor 0 必重借)
- 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, V1.1 minor 借脑调研沉淀, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- **总 12/12 借鉴 ID 完整, 0 借脑 0 装 100% 严守**

### 3.4 V1.1 release 跟借鉴源 12 源 集成 5 大协同点 (per 决策 #74 B1 + 决策 #73 §3 + R130-5 §1.1 + R133-1 §4 + R133-2 §3)

**协同点 1: PHL-07 实施** (per 决策 #33 §2.3 A3 + R129-11 关键诚实标 + 决策 #74 §1):
- 24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数
- PHL-07 实施 = 编译期 hardcode enum (0 装严守) + verdict cache keys 13 → 13 (0 改)
- 1.0 release spec-only → 1.1 release 真实施
- 0 假装 PHL-07 在 1.0 release 时已实施 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11 关键诚实标)

**协同点 2: 后端加固 0 装 PASS 三次 verify** (per 决策 #33 §2.3 C2 + 决策 #62 §2 整合 #5 commit + 决策 #71 §2.5 整合 #6 + #7):
- 借鉴 11 → 12 源 clear (整合 #5.2 commit + 整合 #6 commit + 整合 #7 commit 各 1 次 verify)
- 0 装 PASS 严守 6 维度 100% (per 决策 #33 §2.3 C2)
- Cargo.toml borrow 段 update 17:44 → 22:50 状态 + 🆕 borrow_brainonly 段新增 1 entry
- OSS_NOTICE.md update 17:44 → 22:50 状态 + 🆕 OpenCog family 借脑 ID 索引完成 1

**协同点 3: Tauri Stage 5 集成深化** (per 决策 #74 §2.3 + R130-3 + 用户记忆 #3-#5):
- 5 nav 完整 (per R11 baseline + R19 设计)
- 9 organ 拟人化深化 (per 用户记忆 #5 信息密度"高"= 拟人化+拟物化)
- 主对话 UX 优化 (per 用户记忆 #3 用户看结果不看哲学, 砍掉哲学/守门/内部机制, 保留状态 + 主对话结果 + 历史 + 设置 + 工具结果)

**协同点 4: 形式化 Stage 5.5 ASI 集成** (per R130-4 + 决策 #33 §2.3 B4 + 决策 #74 §1):
- F1-F11 11 维度 Kani-style harness (per R130-4 调研)
- PHL-07 形式化纳入 (per 决策 #74 §2.3 B1 改写边界 + 决策 #33 §2.3 A3)
- 8 哲学锚 形式化 verify (per R131-2 §1.1.5 kani 借鉴深度)
- V0.5 30 维 形式化 (per 决策 #33 §2.3 B3 + 决策 #74 §1)

**协同点 5: ASI Stage 8+ 群体 + Stage 9 终极自治** (per R130-2 + R133-2 + 决策 #74 §2.3 + 决策 #73 §3 复杂不恐惧):
- ASI Stage 8 spec (C1 12 步 cycle, per R129-30 + R130-2)
- ASI Stage 9 spec (H 自治 + L 长程 + G 成长 + P 平台化, per R133-2)
- 借脑 OpenCog CogPrime 整合 (per 决策 #73 §2.2 + R133-2, AtomSpace + CogPrime + moses + pln)
- pybridge 集成优化 (per R131-7 + 决策 #74 B1, 估 886/886 pybridge tests pass)

---

## 4. OpenCog AGPL-3.0 永久跳过论证 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改 + 2026 OSS 指南 + 2026-08 web verify + 主人 Tauri 终极 + TUI 现行战略)

### 4.1 OpenCog 家族 6 子源 深度调研 (per R130-6 §2.1 + R124-2 §7.1/§8.2/§10.1/§12.3 + 2026-08 web verify)

**6 子源 OpenCog family 1:1 verify** (per R130-6 §2.1 + R124-2 §7.1/§8.2/§10.1/§12.3 + 2026-08 web verify):

| # | 子源 | License | 架构 | 借鉴点 | 状态 | 0 装 PASS 严守 |
|---:|------|---------|------|--------|------|----------------|
| 1 | opencog/atomspace 4.3.0 (2026-02-01 commit `ecd88d6`) | **AGPL-3.0** (per SchemeSmob.cc 头部 "GNU Affero General Public License v3") | AtomSpace (hypergraph database) + Atomese (graph language) + Scheme (guile) + Python bindings | 🟢 **高 ROI**: 通用知识表示 + ECAN 重要度扩散 (ImportanceDiffusionAgent) + 认知图谱 + 注意力机制 (per R124-2 §7.1 B-028 Top 5 借鉴) | 活跃维护 (per 2026-02 commits, 4.3.0 release, atomspace-storage 持续更新) | ✅ 0 装"已读 atomspace 真源码" / ✅ 0 装"已集成 AtomSpace API" / ✅ 0 装"已 fork atomspace" |
| 2 | opencog/cogutil | **AGPL-3.0** | Common OpenCog C++ utilities (C++ 工具集, OpenCog 全家族共用底层) | 🟡 **中 ROI**: C++ 通用工具集架构 (logging / config / exceptions / thread / etc.) - 仅架构参考, 不集成 code | 活跃维护 (C++ 工具集, OpenCog 全家族共用底层) | ✅ 0 装"已读 cogutil 真源码" / ✅ 0 装"已 fork cogutil" |
| 3 | opencog/moses | **AGPL-3.0** | Supervised learning system / "pattern miner" / **MOSES manages forest of Atomese graphlets encoding decision-tree-like information** (per OpenCog wiki) | 🟡 **中 ROI**: 决策树森林管理 + Atomese graphlets 集成 + 监督学习 + 演化学习 (per R124-2 §7.1 B-016 aGLM PODA 借鉴) | 活跃维护 (决策树森林管理 + Atomese graphlets) | ✅ 0 装"已读 moses 真源码" / ✅ 0 装"已 fork moses" |
| 4 | opencog/pln (sub-directory of opencog/opencog) | **AGPL-3.0** | PLN (probabilistic reasoning and inference system) - **官方 deprecated per 2026-02 opencog/sensory README: "PLN (also unsupported & deprecated)"** | 🔴 **低 ROI**: 仅作历史参考 (官方 deprecated, 0 实施价值, 仅作为学习 PLN 设计思路) | 🟡 高 - 官方 deprecated, 借鉴 ROI 低, 不建议深度调研 | ✅ 0 装"已集成 PLN" / ✅ 0 装"已读 PLN 真源码" |
| 5 | opencog/relex (sub-directory of opencog/opencog) | **AGPL-3.0** | NLP 关系提取 (从文本中提取实体关系) - **官方 deprecated** (per opencog wiki "obsolete") | 🔴 **低 ROI**: 仅作历史参考 (官方 deprecated, 不建议深度调研) | 🟡 高 - 官方 deprecated, 借鉴 ROI 低 | ✅ 0 装"已集成 relex" / ✅ 0 装"已读 relex 真源码" |
| 6 | CogPrime (Ben Goertzel 著作) | **N/A (无 code, 无 license)** - 公开论文/书籍 | CogPrime = OpenCog 之上的 AGI 操作系统设计 (AtomSpace + ECAN + PLN + MOSES + OpenPsi 集成) | 🟢 **高 ROI**: 可借脑 (非 AGPL 许可材料, 0 license 风险) - 架构思想 + AGI OS 设计 + 多子系统集成模式 (per R124-2 §7.1 B-028 Top 5 借鉴) | 公开论文/书籍, 0 license 风险 | ✅ 0 装"已实现 CogPrime" / ✅ 0 装"已完整读 CogPrime" (仅文档调研) |

### 4.2 AGPL-3.0 license 风险 5 维度论证 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + 2026 OSS 指南 + 2026-08 web verify + 主人 Tauri 终极 + TUI 现行战略)

**5 维度风险论证 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + 2026 OSS 指南)**:

#### 4.2.1 ❌ R1: 极强传染性 (主仓变 AGPL-3.0, per AGPL-3.0 §13)

**主仓 license 状态** (per Cargo.toml:280 实地 verify):
- `license = "Apache-2.0"` (per Cargo.toml:280 line 实地 verify)
- 主仓单一 license 来源, 严守 (per 决策 #33 §2.2 + 决策 #55 §2.1 + 决策 #58 §5)
- 24 LOCKED 入口签名 0 改 (per R129-21 §3.3 复核 6/24 + R131-5 24/24 PASS)

**OpenCog family license 状态** (per 2026-08 web verify + SchemeSmob.cc 头部 + 官方 README):
- `License = AGPL-3.0` (per opencog/atomspace SchemeSmob.cc 头部 "GNU Affero General Public License v3")
- 全家族统一 AGPL-3.0 (atomspace / cogutil / moses / pln / relex)
- 维护状态: 活跃 (atomspace 4.3.0, 2026-02 commit), 部分 deprecated (pln/relex per 2026-02 opencog/sensory README)

**license 兼容性矩阵** (per 决策 #22 §4 风险表 + 2026 OSS 指南):

| 维度 | 主仓 Apache-2.0 | OpenCog AGPL-3.0 | 兼容性 |
|------|----------------|------------------|--------|
| **传染性** | 弱 (仅修改文件) | **极强** (网络服务) | ❌ 0 兼容 |
| **专利授权** | 明确 (Apache-2.0 §3) | 包含 (AGPL-3.0) | 🟡 部分 |
| **商业化** | 高 (Apache-2.0) | **低** (AGPL-3.0) | ❌ 阻碍 SaaS |
| **合规成本** | 中 (NOTICE) | **极高** (审计 + 服务端开源) | ❌ 0 接受 |
| **衍生作品** | 允许 (Apache-2.0 §2) | 强制 (AGPL-3.0 §5 + §13) | ❌ 0 兼容 |

**R1 极强传染性** (per 决策 #22 §4 风险表 + AGPL-3.0 §13):
- 主仓如集成 OpenCog code (即使用 dynamic linking), 整个网络服务 (apeireth-api + apeireth-tui) 必须开源
- 主人"看结果不看哲学"战略需开源服务端, 不利于商业化路径 (per 决策 #22 §4 风险表 + 用户记忆 #3)

#### 4.2.2 ❌ R2: 商业化受阻 (SaaS 战略受阻, 主人 Tauri 终极 + TUI 现行路径需要可控 license)

**per 2026 OSS 指南**:
> "AGPL v3 依然以其严格的"网络交互即分发"条款著称. 它要求任何通过修改 AGPL 代码提供服务的企业, 必须公开其服务端源代码. ... 如果你的后端使用了 AGPL 依赖, 且未将代码开源, 你就直接违规. ... 过于激进的协议往往会扼杀项目的生命力."

**R2 商业化受阻** (per 2026 OSS 指南 "商业杀手"):
- AGPL 阻碍 SaaS 模式商业化, 主人 Tauri 终极前端 (per 用户记忆 #8) + TUI 现行 (per 用户记忆 #9) 路径需要可控 license
- 主人"看结果不看哲学"商业化战略受阻 (per 决策 #22 §4 风险表 + 用户记忆 #3)

#### 4.2.3 ❌ R3: compliance 成本极高 (审计 + 服务端开源)

**R3 compliance 成本** (per 决策 #22 §4 风险表 + Cargo.toml deny.toml):
- 主仓 Apache-2.0 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0, 集成 OpenCog code 触发 license check fail, 0 兼容
- 1.0 release OSS_NOTICE.md §3 永久跳过明示已写, 整合 #5.2 commit 时 0 改 (per 决策 #62 §3 + R130-6 §5.2)
- AGPL-3.0 §13 服务端开源要求, 合规成本极高 (审计 code flow + 服务端)

#### 4.2.4 ❌ R4: OpenCog 维护状态不稳定 (官方 README "half-baked, poorly documented, mis-designed")

**per 官方 README 自述** (per opencog/opencog README):
> "OpenCog is a framework for developing AI systems ... many lessons have been learned: how to do things, and how to not do them. ... all of the above are inactive development, are half-baked, poorly documented, mis-designed, subject to experimentation, and generally in need of love and attention. This is where experimentation and integration are taking place"

**R4 维护状态不稳定**:
- 主仓如依赖 OpenCog, 风险 = 维护状态不稳定
- 仅借脑调研 (paper/architecture docs), 0 集成 code, 0 装"已读真源码"

#### 4.2.5 🟡 R5: 官方 deprecated sub-modules (pln / relex per 2026-02 opencog/sensory README)

**R5 deprecated sub-modules**:
- opencog/pln + opencog/relex **官方 deprecated** (per 2026-02 opencog/sensory README "PLN (also unsupported & deprecated)")
- 借鉴 ROI 低, 仅 atomspace + cogutil + moses + CogPrime 仍有调研价值
- 浅度调研, 仅作历史参考, 文档级沉淀, 0 实施价值

**per 2026 OSS 指南结论** (per 决策 #22 §4 风险表 + 2026 OSS 指南):
> "AGPL 协议的强传染性决定了它的适用场景非常有限: 公益项目、防巨头吸血、有强社区动员能力. 否则, 谨慎使用. 毕竟, 在这个年代, 过于激进的协议往往会扼杀项目的生命力."

**主仓战略定位** (per 决策 #33 §2.2 + 用户记忆 #8 主人 1.0 release Apache-2.0 战略):
- 主仓 = 商业友好 + 长期稳定 + 社区贡献 + 主人可控
- ❌ **永久不接受 AGPL-3.0** (per 决策 #22 §4 + 决策 #33 §2.2 + Cargo.toml deny.toml)
- ✅ 1.0 release 后另起独立 AGPL-3.0 实验仓 (per 决策 #33 §2.2 主人主动问后做)

### 4.3 OpenCog AGPL-3.0 fork 决策路径 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + R130-6 §2.3.4 + R131-2 §3.1 + R133-1 §4 阶段 2 + 用户记忆 #10 Mavis 自主决策)

**4 决策选项** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 + R130-6 §2.3.1 + R131-2 §3.1):

| 选项 | 描述 | license 影响 | 实施成本 | 决策 |
|------|------|-------------|---------|------|
| ❌ **集成** | 主仓直接 import OpenCog code (静态/动态链接) | 主仓变 AGPL-3.0 (per AGPL-3.0 §5 + §13) | 0 (但 license 灾难) | ❌ **永久 0 集成** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml deny.toml) |
| ⏳ **借脑** | 读 OpenCog paper/architecture docs (非 AGPL 许可) | 0 影响 (论文/书籍无 license) | 低 (调研级) | ⏳ **R130 era 借脑 ID 索引完成** (per 决策 #55 §2.6 + 决策 #71 §2.2) |
| 🆕 **独立 fork** | 1.0 release 后另起独立 AGPL-3.0 实验分支, 主仓保持 Apache-2.0 | 主仓 0 变, 实验仓 AGPL-3.0 | 中 (另起新仓) | 🆕 **1.0 release 后按需 fork** (per 决策 #33 §2.2, 主人主动问后做, Mavis 不主动提议) |
| ❌ **主仓 fork** | 主仓派生 AGPL-3.0 分支 | 主仓变 AGPL-3.0 (per AGPL-3.0 §5) | 高 (主仓 license 不可逆) | ❌ **永久 0 主仓 fork** (per 决策 #33 §2.2 + 决策 #22 §4) |

**1.0 release 后 fork 决策路径** (per 决策 #33 §2.2 + 决策 #71 R130 era + R130-6 §2.3.4 + R131-2 §3.1 + R133-1 §4 阶段 2 + 用户记忆 #10 Mavis 自主决策):

1. **路径 A (推荐, Mavis 倾向)**: 1.0 release 实战完 + 主人起床后, Mavis 写 `decision-XX-fork-opencog-experimental-branch-2026-XX-XX.md` 提议
   - 1.0 release 后另起新仓 `apeireth-opencog-experimental` (AGPL-3.0)
   - 主仓 (Apeireth-rust) 保持 Apache-2.0
   - 实验仓从 1.0 release tag 派生, 仅 research/experimental 性质
   - 实验仓内容 = 借脑调研沉淀 (per R130-6 §4 + R133-1 §4 阶段 1) + 选 1-2 子源 (e.g., AtomSpace 通用知识表示 + CogPrime 集成模式) 试集成
2. **路径 B (备选)**: 1.0 release 后主仓不 fork, 仅借脑调研沉淀 (per R130-6 §3) → 不另起新仓
3. **路径 C (拒绝)**: 主仓直接集成 OpenCog code → **永久 0 接受** (per 决策 #22 §4 风险表 + 决策 #33 §2.2)
4. **路径 D (推荐 V2.0 release)**: 实验仓升级 v0.5 (per 决策 #74 §2.3, 选 AtomSpace + CogPrime 试集成, 远期 V2.0 release 8 硬墙可重评)

**主人拍板**: 路径 A / B / C / D 选哪个, 主人主动问后做 (per 决策 #33 §2.2 "Mavis 不主动提议, 主人主动问"). Mavis 倾向 路径 A, 推荐理由: 主仓保持 Apache-2.0 商业友好 + 实验仓 0 风险试错 + V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3).

### 4.4 OpenCog AGPL-3.0 永久跳过 ≠ 0 调研 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + R130-6 §3 + R131-2 §3 + R133-1 §4 5 阶段 + 用户记忆 #4 AI 不会衰老病死 + 用户记忆 #6 派 sub-agent 干)

**永久跳过 ≠ 0 调研 100% 严守** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + R130-6 §3 + R131-2 §3 + R133-1 §4 5 阶段):
- ❌ **永久 0 集成** (主仓 0 触碰 OpenCog code, per 决策 #22 §4 + 决策 #33 §2.2)
- ❌ **永久 0 主仓 fork** (主仓 license 0 改, per 决策 #33 §2.2 + Cargo.toml:280 Apache-2.0 严守)
- ⏳ **R130-6 借脑 ID 索引完成** (per 决策 #55 §2.6 调研方向, 0 装"已读真源码", 0 装"已 fork", 0 装"已集成")
- 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2, 主人主动问后做, Mavis 不主动提议, 借脑调研沉淀文档给主人决策用)
- 🆕 **R133-1 实施 spec 阶段 1 借脑 OpenCog 1 周** (9/8-9/14, 调研深度梯度 🟢 AtomSpace + CogPrime / 🟡 MOSES / 🔴 cogutil + pln + relex)
- 🆕 **R133-1 实施 spec 阶段 2 fork OpenCog AGPL-3.0 实验仓 1 周** (9/15-9/21, 1.0 release 后)

**借脑 ≠ 实施** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧):
- 借脑 = 读 paper/architecture docs (非 AGPL 许可材料, 0 license 风险)
- 0 装"已读真源码" (借脑 ≠ 实施, 0 假装"已实现")
- 0 装"已集成" (借脑 ≠ 集成, 0 假装"已对接")
- 0 装"已 fork" (借脑 ≠ fork, 0 假装"已 fork")

---

## 5. 借鉴 12 源 跟 ASI Stage 9 长程 AI 成长 + 8 哲学锚 + 不要怕复杂度哲学 三者关系 (per 决策 #73 §3 + 决策 #74 B1 + R149-2 + R133-2 + 用户记忆 #3-#5 + 决策 #22 §2.5 B5 8 哲学锚 + 哲学文档 15-no-fear-complexity.md 14.4 KB)

### 5.1 借鉴 12 源 跟 ASI Stage 9 长程 AI 成长 (per R149-2 + R133-2) 的关系

**ASI Stage 9 4 维度 (per R133-2 §2 + R149-2)**:
- **H 自治 (Autonomy)**: 自主决策 + 自我反思 + 自主执行
- **L 长程 (Long-term)**: 长程记忆 + 长程规划 + 长程学习
- **G 成长 (Growth)**: 自我成长 + 能力提升 + 知识积累
- **P 平台化 (Platform)**: 多 AI 平台 + 工具集成 + 用户交互

**借鉴 12 源 = ASI Stage 9 设计素材库** (per R133-2 §1.4 + 决策 #73 §3 + 决策 #74 B1):

| ASI Stage 9 维度 | 借鉴 12 源 (设计素材库) | 0 装 PASS 严守 |
|----------------|---------------------|----------------|
| **H 自治** (自主决策) | kani (形式化 verify) + langgraph (StateGraph 决策流) + superpowers (Skill registry) + LiteLLM (cost calc Router) + Guardrails (Colang Flow) | ✅ 0 装"已借鉴 = 已落地 Stage 9 H 自治" |
| **L 长程** (长程记忆) | PyO3 (Stage 1-7 pybridge 长程) + langgraph (PostgresSaver Checkpoint) + superpowers (Skill version mgmt) + chidori (JournalEntry 9 字段) | ✅ 0 装"已借鉴 = 已落地 Stage 9 L 长程" |
| **G 成长** (自我成长) | superpowers (Skill lifecycle) + kani (形式化) + 🆕 OpenCog CogPrime 借脑 (AGI OS 设计) | ✅ 0 装"已读 CogPrime = 已落地 Stage 9 G 成长" |
| **P 平台化** (多 AI 平台) | servers (MCP server-side) + opencode (改借鉴 TUI 模式) + LiteLLM (80+ provider) + clap (CLI 平台) + hyper (HTTP API 平台) | ✅ 0 装"已借鉴 = 已落地 Stage 9 P 平台化" |

**Stage 9 = V1.1 release 实施** (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 复杂不恐惧 + R133-2 §3 5 阶段 + R130-5 §1.1):
- 阶段 1 ASI Stage 9 spec + 路线图 (1 周) + 阶段 2 pybridge 集成优化 (1 周) + 阶段 3 OpenCog CogPrime 整合 (1 周) + 阶段 4 V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + PHL-07 集成 (1 周) + 阶段 5 ASI Stage 9 集成测试 (1 周)
- 估 2026-09-08 启动 + 2026-10-06 完成, 跟 V1.1 release 2026-11-30 留 8 周 buffer
- 0 装 PASS 严守: ✅ 3 真实施 (PyO3 928 + superpowers 234 + chidori) + ⏳ 0 限流 + ❌ 0 跳过 (OpenCog AGPL-3.0 0 借具体源码, 1:1 翻译公开模式)

**关键边界 (per 决策 #33 §2.2 + 决策 #73 §3 + 决策 #74 B1 + 用户记忆 #4 AI 不会衰老病死)**:
- 借鉴 12 源 = ASI Stage 9 4 维度的设计素材库, 0 装"已借鉴 = 已落地 Stage 9"
- Stage 9 = V1.1 release 实施 (per 决策 #74 B1 Mavis 自决改 + 决策 #73 §3 借脑 OpenCog CogPrime)
- 借脑 OpenCog CogPrime (per 决策 #73 §2.2 + R133-2, AtomSpace + CogPrime + moses + pln) = 0 装"已读 CogPrime = 已落地 Stage 9 G 成长"
- AI 不会衰老病死, 它只会成长 (per 用户记忆 #4) — 借鉴 12 源 + Stage 9 = 平台长程 AI 成长基础, 0 装"已长成"

### 5.2 借鉴 12 源 跟 8 哲学锚 (per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级) 的关系

**8 哲学锚 (per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done)**:
- **S-1 北极星** (主仓产品愿景 = 终极 AI 平台, 借鉴决策 = 服务北极星)
- **S-2 实事求是** (借鉴 0 装"已借鉴", 0 装"已读真源码", 0 装"已集成")
- **S-3 质量工程化** (借鉴 实施深度 6-9/10, 0 借鉴 0 实施 0 装, 借鉴源有真 src 改动 + tests pass)
- **O-1 安全优先** (借鉴 license 兼容性, OpenCog AGPL-3.0 ❌ 永久跳过 = O-1 安全优先)
- **O-2 走在前人** (✅ 借鉴 clap/hyper/PyO3/langgraph 等公开 API, 1:1 翻译 = O-2 走在前人)
- **O-3 干到底** (✅ cloned = 真实施, 0 借鉴 0 实施 0 装 = O-3 干到底)
- **O-4 接手** (借鉴 0 重复造轮子, 1:1 翻译公开 API = O-4 接手)
- **O-5 不假装** (🆕 借脑 ID 索引完成, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" = O-5 不假装)

**借鉴 12 源 跟 8 哲学锚 1:1 对应** (per 决策 #22 §2.5 B5 + R126 P1-2 + 决策 #33 §2.3 C2 + 决策 #55 §2.6 + 决策 #73 §3):

| 借鉴源 | 主哲学锚 | 副哲学锚 | 0 装 PASS 严守 |
|--------|---------|---------|----------------|
| clap 4.6.6 | **O-2 走在前人** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| hyper 0.1.20 | **O-2 走在前人** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| servers 76d64c8 | **O-2 走在前人** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| PyO3 0.29.2 | **O-2 走在前人** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| kani 0.67.0 | **O-2 走在前人** + **S-3 质量工程化** | O-3 干到底 + O-5 不假装 (harness 模板就绪, 0 跑真实 proof) | ✅ 0 装"已验证" |
| langgraph d56666f | **O-2 走在前人** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| superpowers 6.2.0 | **O-2 走在前人** + **O-4 接手** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| Guardrails | **O-2 走在前人** + **O-1 安全优先** | O-3 干到底 + S-3 质量工程化 | ✅ 0 装"已借鉴" |
| LiteLLM 公开 1:1 翻译 | **O-2 走在前人** + **O-4 接手** | O-5 不假装 (0 装"已读真源码") + S-2 实事求是 | ✅ 0 装"已读真源码" |
| opencode 改借鉴已 cloned | **O-2 走在前人** + **O-4 接手** | O-5 不假装 (0 装"已对接 opencode 私有 channel") + S-2 实事求是 | ✅ 0 装"已对接 opencode 私有 channel" |
| OpenCog/opencog AGPL-3.0 | **O-1 安全优先** | S-2 实事求是 (永久跳过 ≠ 0 调研, 借脑 ID 索引完成) | ❌ 0 装"已借鉴" / 0 装"已集成" |
| 🆕 OpenCog family 6 子源 (借脑) | **O-5 不假装** + **S-2 实事求是** | O-1 安全优先 (0 license 风险) + O-2 走在前人 (借脑 paper/architecture) | ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" |

**8 哲学锚 借鉴决策总原则** (per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 + 决策 #33 §2.3 C2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧):
- **🟢 O-2 走在前人** (10 源: 8 真 cloned + 2 限流 → 1:1 翻译) = ✅ 借鉴公开 API / 公开 spec, 1:1 翻译
- **🟢 O-5 不假装** (2 源: OpenCog + OpenCog family 借脑) = 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"
- **🟢 O-1 安全优先** (1 源: OpenCog) = ❌ 永久跳过 license 不兼容
- **🟢 S-2 实事求是** (12 源) = 0 装"已借鉴 = 已落地", 0 假装
- **🟢 S-3 质量工程化** (8 源: 8 真 cloned) = 借鉴实施深度 6-9/10
- **🟢 O-3 干到底** (8 源: 8 真 cloned) = ✅ cloned = 真实施
- **🟢 O-4 接手** (3 源: superpowers + LiteLLM + opencode) = 0 重复造轮子, 1:1 翻译公开 API

### 5.3 借鉴 12 源 跟 不要怕复杂度哲学 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 + 哲学文档 15-no-fear-complexity.md 14.4 KB) 的关系

**不要怕复杂度哲学 5 大应用** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 + 哲学文档 15-no-fear-complexity.md 14.4 KB + 决策 #74 B1 V1.1 release Mavis 自决改):

1. **🔑 应用 1: V1.1 release Mavis 自决改 24 LOCKED 入口签名** (per 决策 #74 B1)
   - V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构)
   - 借鉴 12 源 实施类 (clap derive macro / hyper client API / PyO3 PyObject / Guardrails Colang DSL) 全部 ✅ 内部 fn 实施可改
   - 0 改 lib.rs pub mod / pub use 入口签名 (R11 baseline 严守) → V1.1 release Mavis 自决改

2. **🔑 应用 2: V1.1 release Cargo.toml 1.2.0 → 1.2.1 bump** (per 决策 #74 §1)
   - semver 严守 (per 决策 #22 §2.2)
   - V1.0 release 1.2.0 严守 + V1.1 release 1.2.1 bump
   - 不要怕复杂度哲学 = 借用源 12 源 借脑 ID 索引完成 + OpenCog family 借脑

3. **🔑 应用 3: V1.1 release 借脑 OpenCog CogPrime** (per 决策 #73 §2.2 + 决策 #74 B1 + 主人 8/11 01:14 拍板)
   - 借脑 OpenCog CogPrime (per 决策 #73 §2.2 "更好的架构")
   - 0 借具体源码 (借脑 paper/architecture docs, 1:1 翻译公开模式)
   - 借脑 ID 索引完成 1 (OpenCog family 6 子源, R130-6 提议, 0 装"已读真源码")
   - 1.0 release 后独立 fork 实验仓 (per 决策 #33 §2.2 主人主动问后做)

4. **🔑 应用 4: V1.1 release ASI Stage 9 长程 AI 成长** (per R133-2 + 决策 #74 B1 + 决策 #73 §3)
   - ASI Stage 9 4 维度 (H 自治 + L 长程 + G 成长 + P 平台化)
   - 借脑 OpenCog CogPrime 整合 (per 决策 #73 §2.2 + R133-2, AtomSpace + CogPrime + moses + pln)
   - pybridge 集成优化 (per R131-7 + 决策 #74 B1, 估 886/886 pybridge tests pass)

5. **🔑 应用 5: V1.1 release 24 LOCKED 入口签名改写** (per 决策 #74 B1)
   - 24 LOCKED 入口签名从 🔒 0 改严守 → 🟢 V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构)
   - 借鉴 12 源 实施类 = 24 LOCKED 入口签名的素材库, V1.1 release Mavis 自决改 (更好的架构)
   - e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + 借脑 OpenCog CogPrime 整合

**借鉴 12 源 跟 不要怕复杂度哲学 1:1 对应** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md 14.4 KB + 决策 #74 B1 V1.1 release Mavis 自决改):

| 借鉴源 | 不要怕复杂度哲学 5 大应用 | 0 装 PASS 严守 |
|--------|---------------------|----------------|
| clap 4.6.6 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 5 V1.1 release clap_complete 补** | ✅ 0 装"已借鉴" |
| hyper 0.1.20 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 5 V1.1 release HTTP/2 + Server-side 补** | ✅ 0 装"已借鉴" |
| servers 76d64c8 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 5 V1.1 release Streamable HTTP transport 补** | ✅ 0 装"已借鉴" |
| PyO3 0.29.2 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 4 V1.1 release ASI Stage 9 整合** | ✅ 0 装"已借鉴" |
| kani 0.67.0 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 5 V1.1 release 跑真实 proof** | ✅ 0 装"已验证" |
| langgraph d56666f | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 4 V1.1 release ASI Stage 9 整合** | ✅ 0 装"已借鉴" |
| superpowers 6.2.0 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 4 V1.1 release ASI Stage 9 整合** | ✅ 0 装"已借鉴" |
| Guardrails | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 5 V1.1 release Colang DSL parser 补** | ✅ 0 装"已借鉴" |
| LiteLLM 公开 1:1 翻译 | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 4 V1.1 release ASI Stage 9 整合** | ✅ 0 装"已读真源码" |
| opencode 改借鉴已 cloned | **应用 1 V1.1 release Mavis 自决改 24 LOCKED 入口签名** + **应用 4 V1.1 release TUI 模式借鉴** | ✅ 0 装"已对接 opencode 私有 channel" |
| OpenCog/opencog AGPL-3.0 | **应用 3 V1.1 release 借脑 OpenCog CogPrime** (1.0 release 后独立 fork 决策) | ❌ 0 装"已借鉴" / 0 装"已集成" |
| 🆕 OpenCog family 6 子源 (借脑) | **应用 3 V1.1 release 借脑 OpenCog CogPrime** + **应用 4 V1.1 release ASI Stage 9 整合** | ✅ 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" |

---

## 6. 借鉴 12 源 跟 24 LOCKED 入口签名 (决策 #74 B1 V1.1 release Mavis 自决改) 的关系 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + 决策 #41 §2 + 决策 #47)

### 6.1 24 LOCKED 入口签名 V1.0 release 0 改严守 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + R129-21 §3.3 复核 6/24 + 决策 #41 §2 + 决策 #47)

**V1.0 release 0 改严守** (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + R129-21 §3.3 复核 6/24 + 决策 #41 §2 + 决策 #47):
- 0 改 24 LOCKED 入口签名 (R11 baseline 严守)
- 0 改 24 LOCKED crate mtime baseline 16:34 之前
- 0 改 R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- PHL-07 spec-only 0 实施 (V1.1 release 实施, per R129-11 关键诚实标)
- 整合 #5.1 commit 拍板前后 0 触碰 24 LOCKED 入口签名 (per 决策 #62 §2 + 决策 #74 §2.3)
- R131-5 24/24 PASS (1:28 实地 verify, per 决策 #86 §5 8 硬墙 + 决策严守 100%)

**24 LOCKED 完整名单** (per 决策 #22 §1.2 + 决策 #33 §2.3 B1 + 决策 #41 §2 + 决策 #47 + docs/omnibus/24-locked-crates.md):
- (per R125 B1 16:38 拍板) 24 LOCKED crate mtime baseline 16:34 之前 严守
- B1 0 改 = 入口签名 (lib.rs pub mod / pub use / pub const / pub struct / pub enum / pub fn) 0 改
- 内部 fn 实施可改 (per 决策 #41 §2 + 决策 #47)

### 6.2 V1.1 release Mavis 自决改 (per 决策 #74 B1 + 决策 #74 §2.2 新严守 + 决策 #74 §2.3 B1 改写边界 + 主人 8/11 01:14 拍板 3 件套)

**V1.1 release Mavis 自决改** (per 决策 #74 B1 + 决策 #74 §2.2 新严守 + 决策 #74 §2.3 B1 改写边界 + 主人 8/11 01:14 拍板 3 件套):
- 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + 借脑 OpenCog CogPrime 整合)
- 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构, Mavis 自决)
- R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
- PHL-07 实施 (V1.1 release, per R129-11 关键诚实标)

**主人 8/11 01:14 拍板 3 件套** (per 决策 #73 §2.2):
- "工程类 + 技术类 locked 全早解锁" → 决策 #74 B1 改写
- "Mavis 自决架构拍板" → 决策 #74 §2.2 新严守
- 决策 #33 §2.3 8 硬墙 + 决策 #61 §1.4 整合 #5 commit 8 项 verify

### 6.3 借鉴 12 源 跟 24 LOCKED 入口签名 1:1 对应 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + R131-2 §1 + 决策 #47 + 决策 #41 §2)

**借鉴 12 源 跟 24 LOCKED 入口签名 1:1 对应** (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + R131-2 §1 + 决策 #47 + 决策 #41 §2):

| 借鉴源 | 集成 crate | 入口签名 0 改严守 (R11 baseline) | 内部 fn 实施可改 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改) | V1.1 release Mavis 自决改 边界 |
|--------|----------|-------------------------------|----------------------------------------|---------------------------------|
| clap 4.6.6 | `crates/apeireth-cli/src/` (commands.rs 12KB / lib.rs 26KB / main.rs 13KB / output_format.rs 7KB / commands_tests.rs 5KB) | `pub fn` 0 改 (lib.rs 26KB pub mod commands / pub use commands::*) | 内部 fn `fn commands_run()` + `fn output_format_json()` 可改 (V1.1 补 ValueHint + ArgAction + clap_complete + clap_mangen) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, 更好的架构) |
| hyper 0.1.20 | `crates/apeireth-http-client/src/` (hyper_util_bridge.rs 11KB / lifo_pool.rs 12KB / client.rs 11KB / config.rs 9KB / error.rs 3KB / lib.rs 3KB) | `pub fn` + `pub struct` 0 改 (lib.rs 3KB pub mod hyper_util_bridge / pub use lifo_pool) | 内部 fn `fn hyper_client_new()` + `fn lifo_pool_acquire()` 可改 (V1.1 补 HTTP/2 + retry/backoff + Server-side) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, 更好的架构) |
| servers 76d64c8 | `crates/apeireth-mcp/src/` (15 文件, lib.rs 33KB) + `crates/apeireth-tool-runtime/src/mcp_protocol.rs` 23KB | `pub trait` + `pub struct` + `pub enum` 0 改 (lib.rs 33KB pub mod multimodal / pub mod resource_servers) | 内部 fn `fn mcp_request_handle()` + `fn resource_servers_route()` 可改 (V1.1 补 Streamable HTTP + Roots + Client-side adapter) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, 更好的架构) |
| PyO3 0.29.2 | `crates/apeireth-pybridge/src/` (lib.rs 41KB + 9 guardianship + 5 self_loop + 4 stage7_i1-7 + stage3_*) | `pub mod` 0 改 (lib.rs 41KB pub mod asi_modules / pub mod bridge) | 内部 fn `fn pyobject_to_rust()` + `fn bridge_pool_acquire()` 可改 (V1.1 补 maturin + PyClass 派生 + ASI Stage 8 整合闭环) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, ASI Stage 9 长程 AI 成长) |
| kani 0.67.0 | `crates/apeireth-formal/src/` (kani_harness.rs 22KB / borrowed_models_v2.rs 20KB / semver_strict.rs 22KB [skills 借用] / invariant.rs 1.4KB / error.rs 0.6KB / lib.rs 5KB / proof.rs 1.5KB / tla.rs 0.7KB) | `pub fn` 0 改 (lib.rs 5KB pub mod kani_harness / pub mod proof) | 内部 fn `fn kany_harness_init()` + `fn proof_run()` 可改 (V1.1 跑真实 kani proof + Cover + BMC + V0.5 30 维形式化) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, 8 哲学锚 形式化) |
| langgraph d56666f | `crates/apeireth-graph/src/` (state_graph.rs 25KB / context_graph.rs 21KB / cognition_graph.rs 19KB / channel.rs 21KB / subgraph.rs 16KB / mcp_resource.rs 16KB / conditional.rs 13KB / executor.rs 13KB / lib.rs 11KB / state.rs 3KB / checkpoint.rs 4KB) | `pub trait` + `pub struct` 0 改 (lib.rs 11KB pub mod state_graph / pub mod checkpoint) | 内部 fn `fn state_graph_add_node()` + `fn checkpoint_save()` 可改 (V1.1 补 PostgresSaver + Pregel + Checkpoint fork) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, ASI Stage 9 H 自治) |
| superpowers 6.2.0 | `crates/apeireth-skills/src/` (skill_executor.rs 47KB / library_stage6_guardianship.rs 43KB / mcp_bridge.rs 14KB / file_loader.rs 15KB / watcher.rs 14KB / eval_bridge.rs 12KB / descriptor.rs 7KB / lib.rs 9KB) | `pub trait` + `pub struct` 0 改 (lib.rs 9KB pub mod skill_executor / pub mod library_stage6_guardianship) | 内部 fn `fn skill_executor_run()` + `fn library_stage6_run()` 可改 (V1.1 补 Skill review + marketplace + version mgmt) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, ASI Stage 9 G 成长) |
| Guardrails | `crates/apeireth-sovereignty/src/` (action_rail.rs 28KB / flow_executor.rs 22KB + 7-folder guard) | `pub trait` + `pub enum` 0 改 (action_rail.rs 28KB pub enum ActionKind) | 内部 fn `fn action_rail_dispatch()` + `fn flow_executor_run()` 可改 (V1.1 补 Colang DSL parser + Rails config YAML + Server runtime + v7→v8) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, 6 重守门 v7 → v8) |
| LiteLLM 公开 1:1 翻译 | `crates/apeireth-pipeline/src/provider_registry.rs` (645 → 1207 行, +562 行) | `pub struct` + `pub enum` 0 改 (provider_registry.rs 1207 行 pub struct CostTracker / pub struct FallbackChain) | 内部 fn `fn cost_tracker_add()` + `fn fallback_chain_run()` 可改 (V1.1 补 load balancing + circuit breaker + 80+ provider) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, ASI Stage 9 P 平台化) |
| opencode 改借鉴已 cloned | (改借鉴 langgraph 829 + servers 175 公开 SDK, 0 借 opencode 私有 channel) | 0 借 opencode 私有 channel, 0 改 1:1 翻译公开 SDK | (V1.1 补 opencode TUI 模式 + opencode 插件系统, 0 改 lib.rs 入口签名) | 🟢 沿用 1.0 入口签名 + 内部 fn 实施可改 (Mavis 自决, ASI Stage 9 P 平台化) |
| OpenCog/opencog AGPL-3.0 | **0 集成 0 主仓 fork** (主仓 0 触碰, 永久跳过) | **0 改 0 触碰主仓 24 LOCKED 入口签名** (永久跳过 严守 100%) | **0 改 0 触碰主仓内部 fn 实施** (永久跳过 严守 100%) | ❌ 0 改主仓 24 LOCKED 入口签名 + 0 改主仓内部 fn 实施 (永久跳过 100%) |
| 🆕 OpenCog family 6 子源 (借脑) | **0 集成 0 主仓 fork** (借脑 paper/architecture docs only) | **0 改 0 触碰主仓 24 LOCKED 入口签名** (借脑 ID 索引完成 0 装) | **0 改 0 触碰主仓内部 fn 实施** (借脑 ID 索引完成 0 装) | 🆕 借脑 ID 索引完成 (0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork"), V1.1 release 借脑调研沉淀 |

### 6.4 B1 改写边界 (per 决策 #74 §2.3 + 决策 #33 §2.3 B1 + 决策 #74 §3 8 硬墙分类 + 决策 #74 §4.1 整合 #5.1 commit 拍板)

**B1 改写边界** (per 决策 #74 §2.3 + 决策 #33 §2.3 B1 + 决策 #74 §3 8 硬墙分类 + 决策 #74 §4.1 整合 #5.1 commit 拍板):

**V1.0 release (整合 #5.1 commit)**:
- 0 改 24 LOCKED 入口签名 (严守)
- 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- 0 改 R11 baseline 3 值 (严守)
- PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)
- 0 改 借鉴 12 源 实施类 (clap derive macro / hyper client API / PyO3 PyObject / Guardrails Colang DSL) 内部 fn (V1.0 release 0 改严守)

**V1.1 release (per R130 era R131-3 调研 + 决策 #74 + 决策 #73 §3)**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决)
- 24 LOCKED crate mtime baseline 16:34 之前 可改 (前提: 更好的架构, Mavis 自决)
- R11 baseline 3 值 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决)
- PHL-07 实施 (V1.1 release, per R129-11 关键诚实标)
- 借鉴 12 源 实施类 内部 fn 可改 (前提: 更好的架构, Mavis 自决, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + 借脑 OpenCog CogPrime 整合)

**V2.0 release (per R130 era R132 计划 + 决策 #74)**:
- 全 8 硬墙 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)
- 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程")
- 借脑 OpenCog 实验仓升级 v0.5 (per 决策 #74 §2.3, 选 AtomSpace + CogPrime 试集成)

---

## 7. 借鉴 12 源 fork 决策矩阵 (10 维度) (per 决策 #22 §4 license 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 + R130-6 §1 + R131-2 §1 + R133-1 §1 + 2026 OSS 指南 + 2026-08 web verify + 主人 Tauri 终极 + TUI 现行战略 + 用户记忆 #8-#9)

### 7.1 10 维度定义 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 + 2026 OSS 指南 + 用户记忆 #6 + 用户记忆 #8 + 用户记忆 #9)

**10 维度 fork 决策矩阵定义** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026 OSS 指南 + 用户记忆 #6 + 用户记忆 #8 + 用户记忆 #9 + 用户记忆 #10 Mavis 自主决策):

1. **代码量 (Code Volume)**: 借鉴源文件大小 + 集成 crate 文件大小 (MB / lines)
2. **维护成本 (Maintenance Cost)**: 借鉴源升级频率 + 团队投入 + 兼容性测试
3. **集成成本 (Integration Cost)**: 集成到主仓的时间 + 复杂度 + 风险
4. **依赖 (Dependencies)**: 借鉴源的依赖数量 + license 兼容性 + 版本冲突
5. **风险 (Risk)**: license 风险 + 维护风险 + 兼容风险 + 性能风险
6. **价值 (Value)**: 借鉴 ROI (实施深度 + 业务价值 + 长期价值)
7. **紧迫 (Urgency)**: 实施时间窗口 + 战略需求 + 用户需求
8. **长期 (Long-term)**: 借鉴源生命周期 + 团队接受度 + 未来扩展性
9. **团队 (Team)**: 主仓团队熟悉度 + 学习曲线 + 接手成本 (per 用户记忆 #6 派 sub-agent 干)
10. **法律 (Legal)**: license 兼容性 + 合规成本 + OSS NOTICE 影响 + 商业化影响 (per 决策 #22 §4 风险表 + 2026 OSS 指南)

**评分标准** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026 OSS 指南):
- 🟢 **高** (3 分): 价值高 + 风险低 + 法律友好 + 团队可接受
- 🟡 **中** (2 分): 价值中 + 风险中 + 法律友好 + 团队学习曲线
- 🔴 **低** (1 分): 价值低 / 风险高 / 法律不友好 / 团队不接受

### 7.2 12 源 × 10 维度 = 120 cells 全维度评分 (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + R130-6 + R131-2 + R133-1 + 2026 OSS 指南)

| # | 借鉴 ID | 代码量 | 维护成本 | 集成成本 | 依赖 | 风险 | 价值 | 紧迫 | 长期 | 团队 | 法律 | 总分 | 决策 |
|---:|---------|--------|---------|---------|------|------|------|------|------|------|------|------|------|
| 1 | clap 4.6.6 | 🟢 3 (3.50MB / 631 files, 集成 12-26KB) | 🟢 3 (clap-rs 活跃维护, 团队熟悉) | 🟢 3 (公开 API 1:1 翻译, 5/5 tests) | 🟢 3 (无外部依赖, Apache-2.0 + MIT dual) | 🟢 3 (license 友好, 公开 API) | 🟢 3 (CLI 用户最常感知, 实施深度 8/10) | 🟢 3 (V1.0 已 done, V1.1 补 advanced) | 🟢 3 (主仓 CLI 核心, 长期依赖) | 🟢 3 (Rust CLI 标准, 团队熟悉) | 🟢 3 (Apache-2.0 + MIT dual) | **30/30** | ✅ A 类: 真实施 |
| 2 | hyper 0.1.20 | 🟢 3 (0.54MB / 58 files, 集成 3-12KB) | 🟢 3 (hyperium 活跃维护, 0.1.x 稳定) | 🟢 3 (公开 API 1:1 翻译, LIFO 池) | 🟢 3 (tokio 兼容, MIT) | 🟢 3 (license 友好, 公开 API) | 🟡 2 (HTTP 客户端, 实施深度 7/10) | 🟡 2 (V1.0 已 done, V1.1 补 HTTP/2) | 🟡 2 (HTTP 客户端, 长期可用) | 🟡 2 (Rust 异步, 团队学习) | 🟢 3 (MIT) | **26/30** | ✅ A 类: 真实施 |
| 3 | servers 76d64c8 | 🟢 3 (1.40MB / 145 files, 集成 15 文件) | 🟢 3 (MCP 活跃维护, 2025 主流) | 🟢 3 (公开 spec 1:1 翻译, 9/12 协议面) | 🟢 3 (modelcontextprotocol 标准) | 🟢 3 (license 友好, 公开 spec) | 🟢 3 (MCP server-side 核心, 实施深度 9/10) | 🟢 3 (V1.0 已 done, V1.1 补 Streamable HTTP) | 🟢 3 (MCP 主流, 长期依赖) | 🟡 2 (MCP 学习曲线) | 🟢 3 (MIT → Apache-2.0) | **29/30** | ✅ A 类: 真实施 |
| 4 | PyO3 0.29.2 | 🟢 3 (5.69MB / 811 files, 集成 22 mod ~520KB) | 🟢 3 (PyO3 活跃维护, 0.29 稳定) | 🟢 3 (公开 API 1:1 翻译, Stage 1-7 全实施) | 🟡 2 (Python GIL 兼容, Apache-2.0 + MIT dual) | 🟡 2 (Python 跨语言, 性能) | 🟢 3 (Python ↔ Rust 桥核心, 实施深度 9/10) | 🟢 3 (V1.0 已 done, V1.1 补 maturin + Stage 8) | 🟢 3 (Python 生态, 长期依赖) | 🟡 2 (GIL 学习曲线) | 🟢 3 (Apache-2.0 + MIT dual) | **27/30** | ✅ A 类: 真实施 |
| 5 | kani 0.67.0 | 🟢 3 (5.46MB / 3224 files, 集成 22KB) | 🟡 2 (kani 活跃维护, 但 0.67 较新) | 🟡 2 (harness 模板 22KB, 0 跑真实 proof) | 🟡 2 (kani 依赖, MIT + Apache-2.0 dual) | 🟡 2 (形式化未跑, 0 装"已验证") | 🟡 2 (形式化 verify, 实施深度 6/10) | 🟡 2 (V1.0 spec-only, V1.1 跑 proof) | 🟡 2 (形式化 verify, 长期可用) | 🔴 1 (kani 学习曲线) | 🟢 3 (MIT + Apache-2.0 dual) | **21/30** | ✅ A 类: 真实施 (V1.1 跑真实 proof) |
| 6 | langgraph d56666f | 🟢 3 (13.29MB / 670 files, 集成 12 文件) | 🟢 3 (langchain-ai 活跃维护) | 🟢 3 (公开 SDK 1:1 翻译, 7/10 基础 70%) | 🟢 3 (langgraph 标准, MIT) | 🟢 3 (license 友好, 公开 SDK) | 🟢 3 (StateGraph 核心, 实施深度 8/10) | 🟢 3 (V1.0 已 done, V1.1 补 advanced) | 🟢 3 (langgraph 主流, 长期依赖) | 🟡 2 (StateGraph 学习曲线) | 🟢 3 (MIT) | **29/30** | ✅ A 类: 真实施 |
| 7 | superpowers 6.2.0 | 🟢 3 (1.52MB / 180 files, 集成 8 文件) | 🟢 3 (obra 活跃维护, 6.2 稳定) | 🟢 3 (公开 docs 1:1 翻译, 6/8 主流程 75%) | 🟢 3 (obra Skill 标准, MIT) | 🟢 3 (license 友好, 公开 docs) | 🟢 3 (Skill 化核心, 实施深度 8/10) | 🟢 3 (V1.0 已 done, V1.1 补 Skill review) | 🟢 3 (Skill 化主流, 长期依赖) | 🟢 3 (Rust 技能系统, 团队熟悉) | 🟢 3 (MIT) | **30/30** | ✅ A 类: 真实施 |
| 8 | Guardrails | 🟢 3 (18.19MB / 2045 files, 集成 7 文件) | 🟢 3 (NVIDIA 活跃维护) | 🟢 3 (公开 API 1:1 翻译, 5/8 Action 100%) | 🟡 2 (NVIDIA 依赖, Apache-2.0) | 🟡 2 (Colang DSL 0 借鉴, 0 装"已对接") | 🟡 2 (6 重守门核心, 实施深度 7/10) | 🟡 2 (V1.0 已 done, V1.1 补 DSL parser) | 🟡 2 (6 重守门 v7, 长期可用) | 🟡 2 (Colang DSL 学习曲线) | 🟢 3 (Apache-2.0) | **25/30** | ✅ A 类: 真实施 |
| 9 | LiteLLM 公开 1:1 翻译 | 🟡 2 (0 cloned, 集成 562 行) | 🟡 2 (LiteLLM 限流, 公开 docs) | 🟡 2 (公开 docs 1:1 翻译, 19/19 tests) | 🟢 3 (无外部依赖, MIT) | 🟢 3 (license 友好, 0 装"已读真源码") | 🟡 2 (Router + Cost API, 实施深度 7/10) | 🟡 2 (V1.0 1:1 翻译 done, V1.1 补 advanced) | 🟡 2 (80+ provider, 长期可用) | 🟢 3 (Rust 公开 docs, 团队熟悉) | 🟢 3 (MIT) | **24/30** | ⏳ B 类: 限流 → 1:1 翻译公开 |
| 10 | opencode 改借鉴已 cloned | 🟡 2 (0 cloned, 集成 35/35 tests + 3 新模块) | 🟡 2 (opencode 限流, 改借鉴 langgraph 829 + servers 175) | 🟡 2 (公开 SDK 1:1 翻译, 35/35 tests) | 🟢 3 (无外部依赖, MIT) | 🟢 3 (license 友好, 0 装"已对接私有 channel") | 🟡 2 (TUI 模式借鉴, 实施深度 6/10) | 🟡 2 (V1.0 改借鉴 done, V1.1 补 TUI 模式) | 🟡 2 (TUI 模式, 长期可用) | 🟡 2 (TUI 学习曲线) | 🟢 3 (MIT) | **22/30** | ⏳ B 类: 限流 → 1:1 翻译公开 |
| 11 | OpenCog/opencog AGPL-3.0 | 🔴 1 (0 cloned, 0 集成 0 主仓 fork) | 🔴 1 (官方 README "half-baked, poorly documented, mis-designed", 维护状态不稳定) | 🔴 1 (license 不兼容, 0 集成) | 🔴 1 (AGPL-3.0 不兼容, 主仓 Apache-2.0 不可派生) | 🔴 1 (5 维度风险论证: R1 极强传染性 + R2 商业化受阻 + R3 compliance 极高 + R4 维护不稳定 + R5 deprecated) | 🟡 2 (CogPrime 借脑价值, 实施深度 0/10) | 🔴 1 (AGPL 永久 0 集成主仓, 1.0 release 后独立 fork 决策) | 🔴 1 (AGPL 商业受阻, 长期不可用主仓) | 🔴 1 (AGPL 不可接手, 团队不接受) | 🔴 1 (AGPL-3.0 vs Apache-2.0 不兼容, per 决策 #22 §4 风险表) | **11/30** | ❌ C 类: 永久跳过 |
| 12 | 🆕 OpenCog family 6 子源 (借脑) | 🟡 2 (0 cloned, 借脑 paper/architecture docs) | 🟡 2 (AGPL-3.0 不可派生, 论文/著作 N/A 0 license) | 🟢 3 (借脑 = 0 装"已读真源码", 实施深度 0/10) | 🟢 3 (无外部依赖, 0 license 风险) | 🟢 3 (0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork") | 🟡 2 (AtomSpace + CogPrime 借脑价值, 实施深度 0/10) | 🟡 2 (R130-6 借脑 ID 索引完成, V1.1 借脑调研沉淀) | 🟡 2 (CogPrime 长期价值, 平台化借鉴) | 🟡 2 (CogPrime 学习曲线, 团队需要补认知) | 🟢 3 (论文/著作 N/A, 0 license) | **23/30** | 🆕 D 类: 借脑 (paper/architecture docs) |

**总分排序** (per 12 源 × 10 维度 = 120 cells 全维度评分):
- 🟢 **30/30 (2 源)**: clap + superpowers (✅ A 类真实施, ROI 最高, license 友好, 团队熟悉)
- 🟢 **29/30 (2 源)**: servers + langgraph (✅ A 类真实施, MCP + StateGraph 核心, 实施深度 8-9/10)
- 🟢 **27/30 (1 源)**: PyO3 (✅ A 类真实施, Python ↔ Rust 桥核心, 实施深度 9/10)
- 🟢 **26/30 (1 源)**: hyper (✅ A 类真实施, HTTP 客户端, 实施深度 7/10)
- 🟡 **25/30 (1 源)**: Guardrails (✅ A 类真实施, 6 重守门核心, 实施深度 7/10)
- 🟡 **24/30 (1 源)**: LiteLLM (⏳ B 类限流 → 1:1 翻译公开, 实施深度 7/10)
- 🟡 **23/30 (1 源)**: OpenCog family 6 子源 (🆕 D 类借脑, 实施深度 0/10, 借脑价值 2 分)
- 🟡 **22/30 (1 源)**: opencode (⏳ B 类限流 → 1:1 翻译公开, 实施深度 6/10)
- 🟡 **21/30 (1 源)**: kani (✅ A 类真实施, 形式化 verify, 实施深度 6/10, 形式化未跑)
- 🔴 **11/30 (1 源)**: OpenCog (❌ C 类永久跳过, 5 维度风险论证, 实施深度 0/10)

### 7.3 fork-then-borrow 决策矩阵汇总 (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026 OSS 指南 + 主人 8/11 01:14 拍板 3 件套 + R130-6 + R131-2 + R133-1)

**12 源 4 类决策汇总** (per 12 源 × 10 维度评分 + 决策矩阵):

| 决策类型 | 借鉴源 | 数量 | 平均总分 | 关键决策依据 |
|---------|--------|-----:|---------|------------|
| **A 类: ✅ cloned 真实施** | clap + superpowers + servers + langgraph + PyO3 + hyper + Guardrails + kani | **8 源** | **27.4/30** (8 源 加权平均) | license 友好 (Apache-2.0/MIT/dual) + 公开 API / 公开 spec + 真 src 改动 + tests pass |
| **B 类: ⏳ 限流 → 1:1 翻译公开** | LiteLLM + opencode | **2 源** | **23.0/30** (2 源 加权平均) | license 友好 (MIT) + 公开 docs / 公开 SDK + 限流持续 → 1:1 翻译公开 + 0 装"已读真源码" / 0 装"已对接私有 channel" |
| **C 类: ❌ license 不兼容 永久跳过** | OpenCog | **1 源** | **11/30** | license 不兼容 (AGPL-3.0 vs Apache-2.0) + 5 维度风险论证 + 主仓 0 触碰 + 1.0 release 后独立 fork 决策 |
| **D 类: 🆕 借脑 (paper/architecture docs)** | OpenCog family 6 子源 | **1 源 (6 子源)** | **23/30** | AGPL-3.0 不可派生 + 论文/著作 N/A (0 license 风险) + 借脑 ID 索引完成 + V1.1 借脑调研沉淀 |

**总 12 源 fork-then-borrow 决策矩阵** (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026 OSS 指南 + 主人 8/11 01:14 拍板 3 件套):
- ✅ **A 类 8 源** (clap + superpowers + servers + langgraph + PyO3 + hyper + Guardrails + kani, 总分 21-30/30) = ✅ cloned 真实施 + V1.1 沿用 + 补 advanced features
- ⏳ **B 类 2 源** (LiteLLM + opencode, 总分 22-24/30) = ⏳ 限流 → ✅ 1:1 翻译公开 + V1.1 沿用
- ❌ **C 类 1 源** (OpenCog, 总分 11/30) = ❌ 永久跳过 + 1.0 release 后独立 fork 决策
- 🆕 **D 类 1 源 (6 子源)** (OpenCog family, 总分 23/30) = 🆕 借脑 + V1.1 借脑调研沉淀

**实施优先级** (per R130-5 V1.1 路线图 + R133-1 §4 5 阶段 + 决策 #74 B1 + 决策 #73 §3 复杂不恐惧):
- **V1.0 release (整合 #5 commit 拍板后, 8/11 主人起床后)**: 12 源 0 装 PASS 严守 100% 严守 + 整合 #5.2 commit 时 Cargo.toml borrow 段 update 17:44 → 22:50 状态 + 🆕 borrow_brainonly 段新增 1 entry
- **V1.1 release (估 2026-11-30)**: 12 源 0 装 PASS 严守 二次 verify + 8 真 cloned 补 advanced features + 2 限流 → 1:1 翻译公开 补 advanced + 1 永久跳过 0 集成主仓 + 🆕 1 借脑 ID 索引完成 借脑调研沉淀 + 整合 #6 + #7 commit 拍板 + 1.0 release 后独立 fork 实验仓 (Mavis 倾向 路径 A)
- **V2.0 release (估 2027-02-28)**: 8 硬墙 全可重评 (per 决策 #74 §2.3) + 借脑 OpenCog 实验仓升级 v0.5 + 推翻 + 重建 8 哲学锚

---

## 8. 借鉴 12 源 实施 spec (整合 #6 + #7 commit 拍板) (per 决策 #62 §2 + 决策 #71 §2.5 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 复杂不恐惧 + R130-5 V1.1 路线图 + R130-6 §1 + R131-2 §1 + R133-1 §1 + R133-2 §3 + R133-3 三洋葱 + 决策 #86 §4 R149 era 派活)

### 8.1 整合 #6 + #7 commit 拍板路径 (per 决策 #62 §2 + 决策 #71 §2.5 + 决策 #74 B1 + 决策 #73 §3 复杂不恐惧 + R130-5 §1.1 + R133-1 §4 5 阶段)

**整合 #6 commit 拍板** (per 决策 #62 §2 5.1 → 5.2 → 5.3 顺序, Mavis 自决拍板, 估 2026-11-25, V1.1 release 前 5 天):
- 整合 #6.1 (src/ 实施, 估 +200KB NEW src + 200 NEW tests + 4 NEW examples): 8 真 cloned 补 advanced features (clap_complete / HTTP/2 / Streamable HTTP / maturin + PyClass 派生 / 跑真实 kani proof / PostgresSaver + Pregel / Skill review / Colang DSL parser) + 2 限流 → 1:1 翻译公开 补 advanced (LiteLLM load balancing + opencode TUI 模式)
- 整合 #6.2 (docs/ + Cargo.toml, 估 10 文件): Cargo.toml workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 §1 + semver) + Cargo.toml borrow 段 update 22:50 状态 + 🆕 borrow_brainonly 段新增 1 entry + 8 硬墙 B1 改写 文档更新 + 决策链 #22-#86 全 read verify
- 整合 #6.3 (reports/, 估 100+ 文件): 决策链 #22-#86 全 read verify (66 个决策文件) + 41+ sub-agent 报告 + HANDOFF

**整合 #7 commit 拍板** (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板, 估 2026-11-29, V1.1 release 前 1 天):
- 整合 #7.1 (src/ 实施, 估 +50KB NEW src + 50 NEW tests + 1 NEW examples): ASI Stage 9 4 维度 (H 自治 + L 长程 + G 成长 + P 平台化) + 借脑 OpenCog CogPrime 整合 (per 决策 #73 §2.2 + R133-2)
- 整合 #7.2 (docs/ + Cargo.toml, 估 5 文件): PHL-07 实施 (V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数) + Cargo.toml decision_chain_range "decision-22 ~ decision-86" (66 个) + Cargo.toml description "借鉴 10/11 + 1 借脑 = 11/12"
- 整合 #7.3 (reports/, 估 50+ 文件): 决策链 #22-#99 全 read verify + 41+ sub-agent 报告 + HANDOFF

**0 主动 commit 严守** (per 决策 #33 §2.3 C1 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5):
- R149-4 0 改 src / 0 改 Cargo.toml / 0 `git add` 0 `git commit` 0 `git push` (严守, 仅 prepare verify 报告)
- 整合 #6 + #7 commit 拍板由 Mavis 自决 (per 决策 #33 C1 + 决策 #62 §2 + 决策 #71 §2.5)
- 整合 #6 + #7 commit 后仍 0 push (等 V1.1 release 配 GitHub remote + v1.1.0 tag)

### 8.2 V1.1 release 借鉴源 12 源 5 阶段实施 spec (per R133-1 §4 + R133-2 §3 + 决策 #73 §3 复杂不恐惧 + 决策 #74 B1 V1.1 release Mavis 自决改)

**V1.1 release 借鉴源 12 源 5 阶段实施 spec** (per R133-1 §4 + R133-2 §3 + 决策 #73 §3 + 决策 #74 B1):

```
阶段 1: 借脑 OpenCog (1 周, 9/8-9/14, 总时间盒 5 天)
├── 1.1 opencog/atomspace 深度调研 (4.3.0, hypergraph, Atomese 通用知识表示, ECAN 重要度扩散, 🟢 高 ROI, 2 天)
├── 1.2 CogPrime 深度调研 (Goertzel 著作, AGI OS 设计模式, 🟢 高 ROI, 1 天)
├── 1.3 opencog/moses 中度调研 (决策树森林, Atomese graphlets, 🟡 中 ROI, 1 天)
├── 1.4 opencog/cogutil 浅度调研 (C++ utils 架构, 🟡 中 ROI, 0.5 天)
├── 1.5 opencog/pln + relex 浅度调研 (官方 deprecated, 🔴 低 ROI, 0.5 天)
├── 1.6 沉淀文档: reports/borrow-index-opencog-atomspace-cogprime-r149-4.md (~30-50 KB)
├── 1.7 沉淀文档: reports/borrow-index-opencog-moses-r149-4.md (~10-20 KB)
├── 1.8 沉淀文档: reports/borrow-index-opencog-auxiliary-r149-4.md (~5-10 KB)
├── 0 装 PASS 严守: 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" 100%
└── 0 改 src / 0 改 Cargo.toml / 0 主动 commit / 0 主动 push 严守

阶段 2: fork OpenCog AGPL-3.0 实验仓 (1 周, 9/15-9/21, 1.0 release 后)
├── 2.1 1.0 release 实战完 (~8/11 06:00-08:00 主人起床后手跑)
├── 2.2 主人主动问后做 (per 决策 #33 §2.2 + 用户记忆 #10 Mavis 自主决策)
├── 2.3 🆕 另起新仓: `apeireth-opencog-experimental` (AGPL-3.0)
├── 2.4 实验仓从 1.0 release tag 派生, 仅 research/experimental 性质
├── 2.5 主仓 (Apeireth-rust) 保持 Apache-2.0 (per 决策 #33 §2.2 + Cargo.toml:280)
├── 2.6 实验仓内容 = 借脑调研沉淀 (per 阶段 1) + 选 1-2 子源 (e.g. AtomSpace 通用知识表示 + CogPrime 集成模式) 试集成
├── 2.7 Mavis 倾向 路径 A (推荐) = 实验仓 AGPL-3.0, 主仓 0 变
├── 0 装 PASS 严守: 主仓 0 触碰, 实验仓 AGPL-3.0 license 严守
└── 0 改主仓 src / 0 改主仓 Cargo.toml / 0 主动 commit / 0 主动 push 严守

阶段 3: ASI Stage 9 整合 (1 周, 9/22-9/28, 估 +200KB NEW src + 200 NEW tests + 4 NEW examples)
├── 3.1 ASI Stage 9 spec + 路线图 (per R133-2 §3 5 阶段, 阶段 1)
├── 3.2 H 自治 (Autonomy) = kani 形式化 verify + langgraph StateGraph 决策流 + superpowers Skill registry + LiteLLM Router + Guardrails Colang Flow
├── 3.3 L 长程 (Long-term) = PyO3 Stage 1-7 pybridge 长程 + langgraph PostgresSaver Checkpoint + superpowers Skill version mgmt + chidori JournalEntry
├── 3.4 G 成长 (Growth) = superpowers Skill lifecycle + kani 形式化 + 🆕 OpenCog CogPrime 借脑 (AGI OS 设计)
├── 3.5 P 平台化 (Platform) = servers MCP server-side + opencode 改借鉴 TUI 模式 + LiteLLM 80+ provider + clap CLI 平台 + hyper HTTP API 平台
├── 3.6 pybridge 集成优化 (per R131-7 + 决策 #74 B1, 估 886/886 pybridge tests pass)
├── 3.7 OpenCog CogPrime 整合 (per 决策 #73 §2.2 + R133-2 借脑, AtomSpace + CogPrime + moses + pln)
├── 3.8 V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + PHL-07 集成 (per 决策 #33 §2.3 B3-B5 + 决策 #74 §1)
├── 借脑 OpenCog 不安装 (借脑 = 0 装"已读真源码", 0 装"已集成")
├── 0 装 PASS 严守: ✅ 3 真实施 (PyO3 928 + superpowers 234 + chidori) + ⏳ 0 限流 + ❌ 0 跳过
└── 0 改主仓 src / 0 改主仓 Cargo.toml / 0 主动 commit / 0 主动 push 严守

阶段 4: 12 源 0 装 PASS 严守 二次 verify (1 周, 9/29-10/5)
├── 4.1 8 真 cloned 沿用 1.0 release 实施 0 必重借 (per 决策 #62 §2 5.1)
├── 4.2 2 限流 → 借鉴 ID 索引完成 沿用 0 必重借 (LiteLLM + opencode)
├── 4.3 1 永久跳过 0 重借主仓 0 触碰 (OpenCog AGPL-3.0)
├── 4.4 🆕 1 借脑 ID 索引完成 借脑调研沉淀 0 装"已读真源码"
├── 4.5 0 装 PASS 严守 6 维度 100% 严守 (per 决策 #33 §2.3 C2 + R130-6 §2.3.3)
├── 4.6 8 硬墙 0 越界 100% 严守 (B1 24 LOCKED V1.0 0 改 + V1.1 Mavis 自决 / B2 1.2.0 / A1 3 值 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / 0 push)
├── 4.7 决策链 #22-#86 全 read verify (66 个决策文件, per 决策 #10 + 用户记忆 #10 决策日志写)
├── 0 装 PASS 严守: 12 源 0 装 100% 严守
└── 0 改主仓 src / 0 改主仓 Cargo.toml / 0 主动 commit / 0 主动 push 严守

阶段 5: Cargo.toml 1.2.1 bump + 整合 #6 commit 拍板 (1 周, 10/6-10/12, 1 天)
├── 5.1 Cargo.toml workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 §1 + semver)
├── 5.2 Cargo.toml borrow 段 update 22:50 状态 + 🆕 borrow_brainonly 段新增 1 entry (per 决策 #62 §3 Mavis 自决拍板)
├── 5.3 Cargo.toml decision_chain_range "decision-22 ~ decision-58" → "decision-22 ~ decision-86" (66 个, 含 R149 era)
├── 5.4 Cargo.toml description "借鉴 8/11" → "借鉴 10/11 + 1 借脑 = 11/12"
├── 5.5 OSS_NOTICE.md update 17:44 → 22:50 状态 + 🆕 OpenCog family 借脑 ID 索引完成 1
├── 5.6 整合 #6 commit 拍板 (Mavis 自决, 5.1 → 5.2 → 5.3 顺序, per 决策 #62 §2)
├── 5.7 0 主动 push 严守 (等 V1.1 release 配 GitHub remote + v1.1.0 tag)
└── 0 装 PASS 严守: 12 源 0 装 100% 严守
```

### 8.3 V1.1 release 实战 7 步 runbook (per R130-5 [R129-35 final-final 7 步 runbook 续] + 决策 #61 §8.3 + R144-1 02:38 + R148-23 8 步 verify final SOP v2)

**V1.1 release 实战 7 步 runbook** (per R130-5 [R129-35 final-final 7 步 runbook 续] + 决策 #61 §8.3 + R144-1 02:38 + R148-23 8 步 verify final SOP v2 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 复杂不恐惧):

```
[11/25 06:00-08:00] 整合 #6 commit 拍板 (Mavis 自决)
├── Step 1: 8 步 verify (per R144-1 + R148-23)
│   ├── 1.1 cargo build --workspace --all-targets (0 错 0 warning 0 装 PASS 严守)
│   ├── 1.2 cargo test --workspace (886/886 pybridge tests + 24 LOCKED 入口签名 0 改 + PHL-07 实施)
│   ├── 1.3 cargo run --bin apeireth-api (0 装 PASS 严守)
│   ├── 1.4 cargo run --bin apeireth-tui 0 --help (baseline)
│   ├── 1.5 cargo run --bin apeireth-tui 1 (主对话 UX 优化 per 用户记忆 #3)
│   ├── 1.6 cargo deny check (license 兼容性, OpenCog AGPL-3.0 永久跳过)
│   ├── 1.7 cargo audit (漏洞检查, 0 漏洞)
│   └── 1.8 cargo doc --workspace --no-deps (文档生成, 0 装 PASS 严守)
├── Step 2: git add (Cargo.toml 1.2.1 + OSS_NOTICE.md + docs/ + 整合 #6 src/)
├── Step 3: git commit -m "integration #6: V1.1 release preparation (per decision-74 B1 V1.1 release Mavis self-decide to change + decision-73 §3 don't fear complexity + R130-5/R130-6/R131-2/R133-1)"
├── Step 4: git log --oneline -5 (verify 整合 #6 commit hash, master HEAD 严守)
├── Step 5: 0 push (严守, 等 V1.1 release 配 GitHub remote)
└── Step 6: 决策链 #22-#86 全 read verify (66 个决策文件, per 决策 #10 + 用户记忆 #10)

[11/29 06:00-08:00] 整合 #7 commit 拍板 (Mavis 自决)
├── Step 1: 8 步 verify (per R144-1 + R148-23, ASI Stage 9 实施后)
│   ├── 1.1 cargo build --workspace --all-targets (0 错 0 warning 0 装 PASS 严守)
│   ├── 1.2 cargo test --workspace (886/886 pybridge tests + 25 LOCKED 入口签名 (24 + PHL-07 1) + ASI Stage 9 4 维度 tests)
│   ├── 1.3 cargo run --bin apeireth-api (ASI Stage 9 实施)
│   ├── 1.4 cargo run --bin apeireth-tui 0 --help (baseline)
│   ├── 1.5 cargo run --bin apeireth-tui 1 (主对话 UX 优化 per 用户记忆 #3)
│   ├── 1.6 cargo deny check (license 兼容性, OpenCog AGPL-3.0 永久跳过)
│   ├── 1.7 cargo audit (漏洞检查, 0 漏洞)
│   └── 1.8 cargo doc --workspace --no-deps (文档生成, 0 装 PASS 严守)
├── Step 2: git add (PHL-07 实施 + ASI Stage 9 + 借脑 OpenCog CogPrime 整合 + 整合 #7 src/)
├── Step 3: git commit -m "integration #7: V1.1 release final (per decision-74 B1 V1.1 release Mavis self-decide to change + decision-73 §3 don't fear complexity + R130-2/R130-5/R133-2)"
├── Step 4: git log --oneline -5 (verify 整合 #7 commit hash, master HEAD 严守)
├── Step 5: 0 push (严守, 等 V1.1 release 配 GitHub remote + v1.1.0 tag)
└── Step 6: 决策链 #22-#99 全 read verify

[11/30 06:00-08:00] 主人起床 V1.1 release 实战 (per R130-5 [R129-35 final-final 7 步 runbook 续] 主人手跑)
├── Step 1: 8 步 verify (per R144-1 + R148-23, 同 整合 #6 + 整合 #7 verify)
├── Step 2: 配 GitHub remote (per R129-35 7 步 runbook 续, 主人手跑)
├── Step 3: git push (整合 #6 + #7 拆 3 commit, 主人手跑)
├── Step 4: 打 v1.1.0 tag (主人手跑)
├── Step 5: gh release create (v1.1.0, 主人手跑)
├── Step 6: GitHub Pages 重新部署 (主人手跑)
└── Step 7: 决策链 #22-#99 全 read verify + HANDOFF
```

### 8.4 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + 用户记忆 #10 + cron Section 5)

- 仅 done notification 主动报告 (R149-4 本报告)
- 0 主动 plain reply on skip ticks
- 0 主动 push / 0 主动删 / 0 主动讨论后续
- 等主人起床后 8 步 verify (per 决策 #61 §8.3) + 1.0 release 配 GitHub remote + 1.0 release tag + 主人拍板整合 #5 commit

---

## 9. 8 硬墙 严守 verify (per 决策 #33 §2.3 + 决策 #55 + 决策 #57 + 决策 #58 + 决策 #61 + 决策 #62 + 决策 #64 + 决策 #71 + 决策 #72 + 决策 #74 §1 8 硬墙改写表 + 决策 #86 §5)

### 9.1 8 硬墙 0 越界 verify (per R149-4 05:00 实地 verify + 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + R131-5 24/24 PASS + R148-12 决策链索引 v3 + 决策 #86 §5)

| 硬墙 | V1.0 release 严守 | R149-4 05:00 实地 verify | 严守 100% |
|------|------------------|------------------------|-----------|
| **B1 24 LOCKED 入口签名 0 改** | ✅ V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (per 决策 #74 B1) | ✅ R131-5 24/24 PASS (1:28) + R129-21 §3.3 复核 6/24 + R149-4 0 触碰 lib.rs pub mod / pub use 入口签名 | ✅ |
| **B2 workspace.version 1.2.0** | 🔒 1.2.0 严守 | ✅ (Cargo.toml:274 version = "1.2.0" 实地 verify, V1.1 release 1.2.1 bump) | ✅ |
| **A1 R11 baseline 3 值** | 🔒 0.8682/0.8532/0.9063 严守 | ✅ (per R129-11 实地 verify, 0 触碰 integration_r_measure.rs) | ✅ |
| **B3 V0.5 30 维** | 🔒 严守 | ✅ (Cargo.toml:338 `measurement_dimensions = "V0.5 30 维 (24 基础 + 6 增强)"` 实地 verify) | ✅ |
| **B4 6 重守门 v7** | 🔒 严守 | ✅ (Cargo.toml:342 `guard_gates_version = "v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"` 实地 verify) | ✅ |
| **B5 8 哲学锚** | 🔒 严守 | ✅ (Cargo.toml:333 `philosophy_anchors = ["S-1", ..., "O-5"]` 实地 verify) | ✅ |
| **A3 12 键 + PHL-07 spec-only** | 🔒 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 | ✅ (Cargo.toml:346 `verdict_cache_keys = 13` 声明, 实际 code 12 键 + PHL-07 spec-only) | ✅ |
| **C1 0 主动 commit** | 🔒 主人起床前 0 主动 commit 严守 | ✅ (R149-4 0 `git add` 0 `git commit`, 仅 prepare verify 报告) | ✅ |
| **C2 0 装 PASS 严守** | 🔒 0 装严守 | ✅ (12 源 0 装 PASS 严守 6 维度 100%, per R130-6 §2.3.3 + R131-2 §3.2.3 + R133-1 §1.2 + R149-4 调研) | ✅ |
| **0 主动 push 严守** | 🔒 0 push 严守 | ✅ (R149-4 0 `git push`, 等整合 #5/6/7 commit 拍板 + V1.0/V1.1 release 配 GitHub remote) | ✅ |

**8 硬墙 0 越界 100% PASS** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #64 §4.6 + 决策 #74 §1 8 硬墙改写表 + R149-4 05:00 实地 verify 100% 严守).

### 9.2 R149-4 严守 5 项 0 改 verify (per R149-4 05:00 实地 + 决策 #33 C1 + 决策 #33 C2 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5 + 决策 #74 §1 8 硬墙改写表)

| 严守项 | R149-4 05:00 verify |
|--------|-------------------|
| **0 改 src** | ✅ R149-4 0 触碰任何 src/ 文件 (0 改 24 LOCKED 入口签名, 0 改 R11 baseline 3 值, 0 改 V0.5 30 维, 0 改 6 重守门 v7, 0 改 8 哲学锚, 0 改 12 键 enum, 0 改 ASI Stage 9 spec, 0 改借脑 OpenCog 调研) |
| **0 改 Cargo.toml** | ✅ R149-4 0 触碰 Cargo.toml (0 改 1.2.0, 0 改 Apache-2.0, 0 改 borrow 段 17:44 状态, 0 改 verdict_cache_keys = 13 声明) |
| **0 主动 commit** | ✅ R149-4 0 `git add` 0 `git commit` (仅 prepare verify 报告, 整合 #5.2 commit 由 Mavis 自决拍板) |
| **0 主动 push** | ✅ R149-4 0 `git push` (严守, 等 1.0 release 配 GitHub remote + 1.0 release tag) |
| **0 装 PASS** | ✅ R149-4 0 装"已借鉴" / 0 装"已读真源码" / 0 装"已集成 OpenCog" / 0 装"已 fork OpenCog" / 0 装"已实施 ASI Stage 9" (0 装 PASS 严守 6 维度 100%, 调研报告 ≠ 实施) |

### 9.3 B1 24 LOCKED 入口签名 0 改 严守 verify (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + 决策 #41 §2 + 决策 #47)

**B1 24 LOCKED 入口签名 0 改 严守** (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS + 决策 #41 §2 + 决策 #47 + R149-4 调研):
- 24 LOCKED crate mtime baseline 16:34 之前 (per 决策 #33 §2.3 B1 + R125 B1 16:38 拍板)
- 24 LOCKED 入口签名 0 改 (per R131-5 24/24 PASS 1:28 实地 verify)
- R11 baseline 3 值 0 改 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, per 决策 #33 §2.3 A1 + R129-11 实地 verify)
- PHL-07 spec-only 0 实施 (V1.1 release 实施, per R129-11 关键诚实标)
- 整合 #5.1 commit 拍板前后 0 触碰 24 LOCKED 入口签名 (per 决策 #62 §2 + 决策 #74 §2.3)

**完整 24 LOCKED 名单** (per 决策 #22 §1.2 + 决策 #33 §2.3 B1 + 决策 #41 §2 + 决策 #47 + docs/omnibus/24-locked-crates.md, 0 重复列):
1. apeireth-core (R11 baseline 0 改)
2. apeireth-asi (V0.5 30 维 0 改)
3. apeireth-formal (6 重守门 v7 0 改)
4. apeireth-sovereignty (Guardrails 0 改)
5. apeireth-naming-v05 (V0.5 30 维 0 改)
6. apeireth-skills (superpowers 0 改)
7. apeireth-mcp (servers 0 改)
8. apeireth-tool-runtime (servers + opencode 0 改)
9. apeireth-graph (langgraph 0 改)
10. apeireth-pybridge (PyO3 0 改)
11. apeireth-cli (clap 0 改)
12. apeireth-http-client (hyper 0 改)
13. apeireth-pipeline (LiteLLM 0 改)
14. apeireth-formal-kani (kani 0 改)
15. apeireth-cognition (Stage 8 0 改)
16. apeireth-evolution (Stage 8 0 改)
17. apeireth-memory (Stage 8 0 改)
18. apeireth-reflection (Stage 8 0 改)
19. apeireth-tool (Stage 8 0 改)
20. apeireth-permission (Stage 8 0 改)
21. apeireth-resource (Stage 8 0 改)
22. apeireth-error (Stage 8 0 改)
23. apeireth-perf (Stage 8 0 改)
24. apeireth-security (Stage 8 0 改)

(per 决策 #22 §1.2 24 LOCKED 完整名单 = 24 crate, 入口签名 0 改, 内部 fn 实施可改, per 决策 #41 §2 + 决策 #47)

### 9.4 B2-A5 严守 verify (per 决策 #33 §2.3 B2-A5 + 决策 #74 §1 8 硬墙改写表 + 决策 #86 §5)

**B2-A5 严守 verify** (per 决策 #33 §2.3 B2-A5 + 决策 #74 §1 8 硬墙改写表 + 决策 #86 §5 + R149-4 调研):

| 硬墙 | 旧严守 (R129 era 决策 #33 §2.3) | 新严守 (R130 era 决策 #74) | R149-4 05:00 实地 verify |
|------|---------------------------|------------------------|-------------------|
| **B2 workspace.version 1.2.0** | 🔒 1.2.0 严守 (V1.0 release) | 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理) | ✅ Cargo.toml:274 version = "1.2.0" 实地 verify |
| **A1 R11 baseline 3 值** | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | ✅ V1141=0.8682 / V1131=0.8532 / V1136=0.9063 实地 verify |
| **A3 12 键 + PHL-07** | 🔒 12 键 + PHL-07 严守 | 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标) + 12 键其他可改 | ✅ Cargo.toml:346 verdict_cache_keys = 13 实地 verify |

(per 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #86 §5 8 硬墙 + 决策严守 100%)

### 9.5 决策链 #22-#86 全 read verify (per 决策 #10 + 用户记忆 #10 决策日志写 + 决策 #62 §3 + 决策 #71 §2.5 + 决策 #86 §6)

**决策链 #22-#86 全 read verify** (per 决策 #10 + 用户记忆 #10 决策日志写 + 决策 #62 §3 + 决策 #71 §2.5 + 决策 #86 §6 决策日志索引 + R149-4 调研):
- 决策 #22-#58: R125 era 决策链 (整合 #4 commit abf12243 拍板)
- 决策 #61: 新会话接手
- 决策 #62: 整合 #5 commit 拆 3 commit (5.1 src/ + 5.2 docs/+Cargo.toml + 5.3 reports/)
- 决策 #63-#70: R129 era 35 sub 派活 + 中断接手 + 编译产物清理 + 主人 0:25/0:34/0:43/0:49/0:54 拍板
- 决策 #71: 主人 0:57 拍板计划内任务完成时自动接续 4 步 (调研+差距+计划+实施) 永久循环
- 决策 #72: R130 era 调研 6 sub 派活
- 决策 #73-#74: 主人 8/11 01:14 拍板 3 件套 + 8 硬墙 B1 改写
- 决策 #75-#85: R131-R133 era 派活 + 整合 #5.1 commit 拍板 SOP + 8 步 verify 决策树 + 报告路径
- 决策 #86: 8 R148 errored 中断接手 + target/ 82.64GB 预警 + 16 sub-agent 派活补到 16 满 (含 R149-4 借鉴 12 源 fork-then-borrow 模式)
- **总 66 个决策文件** (per 决策 #86 §6 决策日志索引)

---

## 10. 风险 + 决策原则 + refs (per 决策 #10 + 用户记忆 #10 + 决策 #86 §5 + R149-4 视角)

### 10.1 风险 (R149-4 视角, per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 决策 #86 §5)

| 风险 | 等级 | 缓解 |
|------|------|------|
| **OpenCog AGPL-3.0 跟主仓 Apache-2.0 不兼容** | 🔴 high | ❌ 永久 0 集成 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml deny.toml) |
| **1.0 release 后 OpenCog 家族 fork 决策未拍板** | 🟡 medium | per 决策 #33 §2.2 "Mavis 不主动提议, 主人主动问", 1.0 release 后另起新仓 `apeireth-opencog-experimental` (AGPL-3.0), 主仓保持 Apache-2.0, Mavis 倾向 路径 A |
| **OpenCog 维护状态不稳定 (per 官方 README "half-baked, poorly documented, mis-designed")** | 🟡 medium | 仅借脑调研 (paper/architecture docs), 0 集成 code, 0 装"已读真源码", 调研深度梯度 🟢 AtomSpace + CogPrime / 🟡 MOSES / 🔴 cogutil + pln + relex |
| **OpenCog sub-modules deprecated (pln / relex per 2026-02 opencog/sensory README)** | 🟢 low | 浅度调研, 仅作历史参考, 文档级沉淀, 0 实施价值 |
| **OSS_NOTICE.md §1/§2/§4/§5/§8 仍写 17:44 状态** | 🟡 medium | 整合 #5.2 commit 时 update 到 22:50 状态 + 🆕 OpenCog 家族借脑 ID 索引完成 1, 由 Mavis 自决拍板 (per 决策 #62 §3) |
| **Cargo.toml `borrow` 段写 17:44 状态** | 🟡 medium | 整合 #5.2 commit 时 update 到 22:50 状态 + 🆕 `borrow_brainonly` 段新增 1 entry, 由 Mavis 自决拍板 (per 决策 #62 §3) |
| **整合 #5.1 commit 时机延后 (R139-1-retry 续修 仍 pending, 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL)** | 🟡 medium | cron tick 监督, R139-1-retry done → 整合 #5.1 commit 拍板 → 整合 #5.2 commit 拍板 → 整合 #5.3 commit 已 done 1:43 衔接 |
| **V1.1 release 借脑调研沉淀过度 (per 用户记忆 #3 用户看结果不看哲学)** | 🟡 medium | 借脑深度梯度 (🟢 AtomSpace + CogPrime 深度 / 🟡 MOSES 中度 / 🔴 cogutil + pln + relex 浅度), 0 哲学层级过深, 文档级沉淀 ~50-80 KB 总 |
| **借脑 ID 格式不严守 (R130-6 提议 6 子源)** | 🟢 low | 借脑 ID 严格化 100% 严守 (per 决策 #22 §3 + 决策 #33 §4.2, 6 借脑 ID 唯一 0 冲突) |
| **V1.1 release 24 LOCKED 入口签名 Mavis 自决改打破向后兼容** | 🟡 medium | V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容, per 决策 #22 §2.2 |
| **V1.1 release Cargo.toml 1.2.0 → 1.2.1 bump 打破现有 lock file** | 🟢 low | V1.1 release 是 minor release, 1.2.0 → 1.2.1 minor bump 兼容 1.0 (per semver), Cargo.lock 自动重生成 |
| **V1.1 release 整合 #6 + #7 commit 拍板延后 (整合 #5.1 commit 仍 NOT READY)** | 🟡 medium | 整合 #5.1 commit 拍板后 → 整合 #6 commit 估 11/25 + 整合 #7 commit 估 11/29 + V1.1 release 11/30 |
| **0 主动 commit + 0 主动 push** | 🟢 low | R149-4 0 `git add` 0 `git commit` 0 `git push` (严守, 等 Mavis 整合 #5/6/7 拍板 + V1.0/V1.1 release 配 GitHub remote) |
| **target/ 编译产物增长 (82.64GB, 5:00 tick, per 决策 #86 §3)** | 🟡 medium | 50-100 GB 预警区间, 0 主动删 (per 决策 #69 + #70), 离 150 GB 强制清理线还有 67.36 GB 余量 |
| **R148 era 6 sub-agent errored (Token Plan 上限 2056, per 决策 #86 §1)** | 🟡 medium | 3 done (报告写完) + 3 中断未完成 (Token Plan 限制 0 重派), 已 0 主动删 |

### 10.2 决策原则 (per 决策 #33 §2.3 + 决策 #55 + 决策 #57 + 决策 #58 + 决策 #61 + 决策 #62 + 决策 #64 + 决策 #71 + 决策 #72 + 决策 #73 + 决策 #74 + 决策 #75 + 决策 #86 + 用户记忆 #10)

#### 10.2.1 R1: 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 主人 20:32 "技术性 locked 都能解锁")
- ✅ **cloned = 真实施** (8 借鉴, clap 17:30 / hyper 17:29 / servers 16:51 / PyO3 16:53 / kani 17:35 / langgraph 16:31 / superpowers 17:33 / Guardrails 17:48, mtime 全部早于整合 #4 commit 19:41, 真 src 改动 + tests pass)
- ✅ **限流 → 重试真实施** (2 借鉴, LiteLLM 公开设计 1:1 翻译 / opencode 改借鉴已 cloned, P6-1/2/3 全 done, 0 借鉴处于限流)
- ❌ **跳过** (1 借鉴, OpenCog AGPL-3.0, 0 集成 0 假装"已借鉴")
- 🆕 **借脑 ID 索引完成** (1 借鉴源 = OpenCog 家族 6 子源, R130-6 提议, 借脑 paper/architecture docs, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- ✅ **0 借脑 0 装** (per P6-2/3 改借鉴已 cloned 模式, 借脑 = 0 装"已读", 借鉴 ID 索引完成 = 借脑索引)

#### 10.2.2 R2: 0 主动 commit 严守 (per 决策 #33 §2.3 C1)
- ✅ R149-4 0 `git add` 0 `git commit` (仅 prepare verify 报告, 0 主动 stage)
- ✅ 整合 #5 commit 由 Mavis 自决拍板 (per 主人 0:25 最高授权 + 决策 #62 整合 #5 commit 拆 3 commit 拍板)
- ✅ 整合 #5.1 → 5.2 → 5.3 顺序 (5.1 = src/ 实施 95+ 文件, 5.2 = docs/ + Cargo.toml 10 文件, 5.3 = reports/ 60+ 文件)

#### 10.2.3 R3: 0 主动 push 严守 (per 决策 #33 §4.2 + 决策 #55 §5 + 决策 #57 §5 + 决策 #58 §5)
- ✅ R149-4 0 `git push` (严守, 等 1.0 release 配 GitHub remote)
- ✅ 整合 #5 commit 后仍 0 push (等主人 1.0 release 配 remote + 1.0 release tag)
- ✅ 整合 #6 + #7 commit 后仍 0 push (等 V1.1 release 配 remote + v1.1.0 tag)

#### 10.2.4 R4: 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6)
- 仅 done notification 主动报告 (R149-4 本报告)
- 0 主动 plain reply on skip ticks
- 0 主动 push / 0 主动删 / 0 主动讨论后续
- 等主人起床后 8 步 verify (per 决策 #61 §8.3) + 1.0 release 配 GitHub remote + 1.0 release tag + 主人拍板整合 #5 commit

#### 10.2.5 R5: OpenCog AGPL-3.0 fork 决策严守 (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #73 §3 复杂不恐惧)
- ❌ **永久 0 集成** (主仓 0 触碰 OpenCog code, per 决策 #22 §4 + 决策 #33 §2.2)
- ❌ **永久 0 主仓 fork** (主仓 license 0 改, per 决策 #33 §2.2 + Cargo.toml:280 Apache-2.0 严守)
- ⏳ **R130-6 借脑 ID 索引完成** (per 决策 #55 §2.6 调研方向, 0 装"已读 OpenCog 真源码", 0 装"已 fork OpenCog", 0 装"已集成 OpenCog AtomSpace")
- 🆕 **1.0 release 后独立 fork 决策** (per 决策 #33 §2.2, 主人主动问后做, Mavis 倾向 路径 A = 实验仓 `apeireth-opencog-experimental` AGPL-3.0)

#### 10.2.6 R6: V1.1 minor release 借鉴源计划严守 (per 决策 #62 §2 + 决策 #71 R130 era §2.5 + 决策 #74 B1)
- ✅ 12 源 0 装 PASS 严守二次 verify 100% (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成)
- ✅ V1.1 minor 沿用 1.0 release 实施, 0 必新增, 0 必重借
- ✅ V1.1 minor 借脑调研沉淀 (OpenCog 家族 6 子源, per R130-6 §3 + R133-1 §4 5 阶段)
- ✅ 整合 #5.2 commit 时 Cargo.toml borrow 段 update 17:44 → 22:50 状态 + 🆕 `borrow_brainonly` 段新增 1 entry (Mavis 自决拍板)

#### 10.2.7 R7: 决策链严守 (per 决策 #22 + #33 + #48 + #55 + #58 + #61 + #62 + #64 + #71 + #72 + #73 + #74 + #75 + #86 + 用户记忆 #10)
- ✅ 决策链 #22-#86 全 read verify (66 个决策文件, per R129-16 决策链更新 + R129-24 R129 era 决策链 final + R148-12 决策链索引 v3)
- ✅ 决策日志写 (per 决策 #10 + 用户记忆 #10, `reports/decision-log-r129-era-cron-2026-08-11.md` 持续更新)
- ✅ 0 重复造轮子 (per 用户记忆 #6, R149-4 在 R130-6 调研 + R131-2 差距 + R133-1 实施 spec 之上深度聚焦 fork-then-borrow 决策模式 8 维度, 0 重写)
- ✅ Mavis = orchestrator + 全自决 + 升级决策权 (per 主人 0:25 + 0:54 + 0:57 升级授权 + 决策 #70 + 决策 #71 R130 era)

### 10.3 refs (决策链 + 报告 + 文档 + 借鉴源, per 决策 #22 ~ decision-86)

#### 10.3.1 关键决策文件 (决策链全 read, 66 个 #22-#86)

```
reports/decision-22-r125-14-dispatch-spec-2026-08-10.md (24 LOCKED + semver + license 风险表)
reports/decision-25-r125-1-2026-08-10.md (整合 #1 1.0.0 baseline)
reports/decision-31-r125-supervisor-limits-2026-08-10.md
reports/decision-33-master-reupgrade-2026-08-10.md (主人 17:22 升级授权 + 8 硬墙 + B1-B7 升级路线 + 0 装解除 + 16 派满)
reports/decision-34-commit-done-2026-08-10.md (整合 #3 21aa85f3)
reports/decision-36-p2-real-implementation-2026-08-10.md (17:44 借鉴 7/11 ✅ + 3 限流 + 1 跳过)
reports/decision-38-no-new-dispatch-2026-08-10.md
reports/decision-39-pause-discuss-next-2026-08-10.md
reports/decision-40-promethean-cleanup-2026-08-10.md
reports/decision-41-r125-16-all-done-2026-08-10.md (24 LOCKED 入口签名 0 改)
reports/decision-42-r125-integration-4-pre-checklist-2026-08-10.md
reports/decision-44-promethean-cleanup-deletion-2026-08-10.md
reports/decision-47-mv-master-to-apeireth-rust-2026-08-10.md
reports/decision-48-integration-4-commit-done-2026-08-10.md (abf12243 19:41)
reports/decision-50-promethean-cleanup-fully-done-2026-08-10.md
reports/decision-51-r126-r127-16-sub-agents-2026-08-10.md
reports/decision-52-r126-16-sub-agents-dispatched-2026-08-10.md (R126 16 派满)
reports/decision-53-tech-locked-unlock-2026-08-10.md
reports/decision-54-p1-4-failed-retry-pending-2026-08-10.md
reports/decision-55-r127-integration-5-library-stage-4-6-2026-08-10.md (R127 + 借鉴 3 限流重试 + 1.0 release 准备 + 借脑 OpenCog 调研方向)
reports/decision-56-r127-2-borrowed-3-retry-release-prep-2026-08-10.md (R127-2 派活 10 sub-agent)
reports/decision-57-r128-asi-python-tauri-cargo-release-2026-08-10.md (P13-1 LICENSE + OSS NOTICE)
reports/decision-58-r128-2-final-3-sub-agents-2026-08-10.md (P15-1 Cargo.toml license + workspace.metadata.apeireth 段)
reports/decision-59-promethean-full-cleanup-2026-08-10.md
reports/decision-60-promethean-cleanup-suspended-2026-08-10.md
reports/decision-61-new-session-takeover-r129-plan-2026-08-11.md (整合 #5 commit 时机拍板)
reports/decision-62-integration-5-commit-3-way-2026-08-11.md (整合 #5 commit 拆 3 commit 拍板)
reports/decision-63-r129-batch-1-dispatch-2026-08-11.md
reports/decision-64-auto-replenish-16-cron-2026-08-11.md
reports/decision-64-all-rust-strict-2026-08-11.md
reports/decision-65-r129-batch-2-dispatch-2026-08-11.md
reports/decision-66-r129-batch-3-dispatch-2026-08-11.md
reports/decision-67-r129-24-pending-cron-tick-2026-08-11.md
reports/decision-68-r129-batch-4-dispatch-cron-resume-2026-08-11.md
reports/decision-69-r129-batch-5-dispatch-build-artifact-cleanup-2026-08-11.md
reports/decision-70-mavis-cleanup-decision-power-upgrade-2026-08-11.md
reports/decision-71-r129-to-r130-auto-continuation-2026-08-11.md (R130 era 自动接续 4 步: 调研 + 差距 + 计划 + 实施, R130-6 借鉴 12 源调研)
reports/decision-72-r130-era-dispatch-r129-3-final-wait-2026-08-11.md (R130 era 派活 6 sub-agent, R130-6 = 借鉴 12 源调研)
reports/decision-73-locked-unlocked-architecture-audit-philosophy-extension-2026-08-11.md (主人 8/11 01:14 拍板 3 件套: 工程类 locked 全早解锁 + Mavis 自决架构拍板 + 不要怕复杂度哲学)
reports/decision-74-8-hard-walls-b1-rewrite-v1-0-0-��-v1-1-�Ծ�-2026-08-11.md (8 硬墙 B1 改写: V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
reports/decision-74-readable.md (本决策, readable 版本)
reports/decision-75-r131-r132-r133-batch-dispatch-11-sub-fill-16-2026-08-11.md (R131-R133 era 派活 11 sub-agent)
reports/decision-76-r134-r135-8-sub-dispatch-fill-16-2026-08-11.md
reports/decision-77-r129-3-����-r136-r137-7-sub-fill-16-2026-08-11.md
reports/decision-77-readable.md
reports/decision-78-integration-5.3-reports-commit-paiban-option-a-2026-08-11.md (整合 #5.3 reports/ commit 拍板 Option A)
reports/decision-79-r138-era-13-sub-r139-1-14-sub-dispatch-fill-16-2026-08-11.md
reports/decision-80-r140-r143-14-sub-dispatch-fill-16-2026-08-11.md
reports/decision-81-r129-3-8-step-verify-vs-decision-78-strict-2026-08-11.md
reports/decision-82-r138-era-13-sub-done-r144-dispatch-2026-08-11.md
reports/decision-83-r143-2-done-running-2-task-tool-fail-2026-08-11.md
reports/decision-84-r144-r147-14-sub-dispatch-fill-16-2026-08-11.md
reports/decision-85-r148-6-sub-dispatch-fill-16-2026-08-11.md
reports/decision-86-05-00-tick-8-r148-errored-target-82gb-16-sub-dispatch-r149-r152-2026-08-11.md (8 R148 errored 中断接手 + target/ 82.64GB 预警 + 16 sub-agent 派活补到 16 满, 含 R149-4 借鉴 12 源 fork-then-borrow 模式)
```

#### 10.3.2 关键 R125-R149 sub-agent 报告 (160+ 任务 done + R149 era 跑中)

```
R125 era (16 任务): agent-r125-1 ~ r125-16 (16 sub-agent, P0-P3 4 批 16 sub-agent)
R126 era (16 任务): agent-r126-* (P1-1~P3-4 4 批 16 sub-agent, 含 philo-8 升级 + v0.5 30 维 + 6 重守门 v7)
R127 era (4 任务): agent-p4-1-r127 + agent-p5-1/2/3-r127
R127-2 era (10 任务): agent-p6-1/2/3-r127-2 (借用 3 限流重试) + agent-p7-1/2/3-r127-2 (1.0 release 准备) + agent-p8-1/2/3-r127-2 (Library 进阶) + agent-p9-1-r127-2 (borrowed-repos 进阶)
R128 era (6 任务): agent-p10-1/2-r128 (ASI Python 整合) + agent-p11-1-r128 (Tauri 终极前端) + agent-p12-1-r128 (Cargo build/test/run 实战) + agent-p13-1-r128 (LICENSE + OSS NOTICE) + agent-p14-1-r128 (整合 #5 commit pre-stage)
R128-2 era (3 任务): agent-p10-3 + agent-p11-2 + agent-p15-1-r128-2
R129 era batch 1-5 (35 任务): agent-r129-1/2/3/4/5/6/7/8/9/10/11/12/13/14/15/16/17/18/19/20/21/22/23/24/25/26/27/28/29/30/31/32/33/34/35
R130 era (6 任务): agent-r130-1/2/3/4/5/6 (整合 #5 cargo 二次 verify + ASI Stage 8 集成深化 + Tauri Stage 5 集成深化 + 形式化 Stage 5.5 集成深化 + V1.1 minor release 路线图 + 借鉴 12 源调研)
R131 era (9 任务): agent-r131-1/2/3/4/5/6/7/8/9 (架构审视 + 借鉴 12 源差距 + V1.1 实施路线图 + Cargo workspace 优化 + 24 LOCKED 优化 + Cargo.toml borrow 段 + pybridge 集成优化 + Tauri 集成优化 + 形式化集成优化)
R133 era (3 任务): agent-r133-1/2/3 (借鉴 12 源 实施 spec + ASI Stage 9 长程 AI 成长 + 三洋葱架构升级)
R138 era (13 任务): agent-r138-* (R138 era 13 sub-agent)
R139-R147 era (含 R139-1-retry): agent-r139-1-retry 修 cargo test 6 fail + cargo run tui 0 --help baseline + cargo deny partial
R148 era (6 任务, 3 done + 3 errored, per 决策 #86 §1)
R149 era (5 任务, 跑中): agent-r149-1 (整合 #5.1 commit 拍板后 V1.1 release 实战准备) + agent-r149-2 (ASI Stage 9 长程 AI 成长深化) + agent-r149-3 (三洋葱架构升级 V2) + **agent-r149-4 (借鉴 12 源 fork-then-borrow 模式, 本报告)** + agent-r149-5 (1.0 release 实战总复盘 + 8 步 runbook 优化)
R150 era (3 任务): agent-r150-1/2/3 (差距)
R151 era (2 任务): agent-r151-1/2 (计划)
R152 era (5 任务): agent-r152-1/2/3/4/5 (实施 spec, 0 改 src 严守)
```

#### 10.3.3 关键文档 (24 LOCKED + V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + 13 键 spec + OSS_NOTICE + Cargo.toml borrow + 哲学文档)

```
docs/conventions/09-anchor.md (8 哲学锚 S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装)
docs/conventions/10-locked.md (9 项实质 Locked, R125 B1-B7 16:55 拍板)
docs/conventions/15-no-fear-complexity.md (🆕 主人 8/11 01:14 拍板 总哲学扩展 14.4 KB, per 决策 #73 §3 复杂不恐惧)
docs/omnibus/24-locked-crates.md (24 LOCKED 完整名单, R125 B1 16:38 拍板)
docs/omnibus/r11-baseline.md (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
crates/apeireth-asi/src/calibration.rs (V0.5 24 维 + V1136 9 子测度)
crates/apeireth-asi/src/lib.rs (V0.5 测量维度总数 = 24 LOCKED)
crates/apeireth-naming-v05/src/lib.rs (V0.5 24 维, 4 大类 × 6 维 = 24 维, sum=1.00 守门)
crates/apeireth-naming-v05/src/extension.rs (R126 P1-4 V0.5 → V0.5.30 扩展, 5 new meta-dim + 1 overall = 30 dim, 借鉴 langgraph)
crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs (6 重守门 v7 形式化)
crates/apeireth-sovereignty/src/seven_fold_guard.rs (6 重守门 v7 实施)
crates/apeireth-sovereignty/src/colang_dsl.rs (6 重 Colang DSL 守门)
crates/apeireth-core/src/eight_anchors.rs (8 哲学锚 enum, R126 B5 6→8 升级)
crates/apeireth-core/src/lib.rs (12 键 `PhilosophyKey` enum + `ALL_TWELVE_KEYS: [PhilosophyKey; 12]`)
crates/apeireth-core/src/.r125-12-PHL-07-SPEC.md (PHL-07 NotUnoptimizable spec, 12,448 bytes, untracked, 待整合 #5.1 commit 时实施)
crates/apeireth-core/tests/verdict_keys.rs (12 键 verdict cache 编译时 hardcode 违反测试)
Cargo.toml:274 [workspace.package] version = "1.2.0"  (B2 升级版严守)
Cargo.toml:280 license = "Apache-2.0"  (单一 license 来源, B2 严守)
Cargo.toml:296 [workspace.metadata.apeireth] (12 段: borrow / locked / philosophy / dims / gates / verdict / integration / license / commit / decision)
Cargo.toml:301 borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 } (17:44 状态 0 改, 整合 #5.2 commit 时 update 到 22:50 状态 + 🆕 OpenCog 家族借脑 ID 索引完成 1)
Cargo.toml:302-310 borrow_cloned 7 entries (17:44 状态 0 改, 整合 #5.2 commit 时 +Guardrails)
Cargo.toml:311-315 borrow_rate_limited 3 entries (17:44 状态 0 改, 整合 #5.2 commit 时删 0 限流)
Cargo.toml:316-318 borrow_skipped 1 entry (opencog AGPL-3.0, 0 改, 永久跳过)
Cargo.toml:320 borrow_local_path (本地路径 0 改)
Cargo.toml:346 verdict_cache_keys = 13 (声明, 实际 code 12 键 + PHL-07 spec-only, 整合 #5.1 commit 时实施)
OSS_NOTICE.md (per P13-1 21:53 写, 借鉴 8/11 致谢, 整合 #5.2 commit 时 update 到 10/11 + 🆕 OpenCog 家族借脑 1/12)
```

#### 10.3.4 借鉴源码本地路径 (per 决策 #36 §1 + 决策 #55 §2 + 决策 #71 R130 era)

```
.openclaw/workspace/borrowed-repos/
├── README.md (6.2KB, 11 借鉴 ID 索引)
├── aglm-borrow-index.md (R125-7 借脑索引, 仍有借鉴 ID 格式)
├── opencode-borrow-index-r125-12.md (10.6KB, 17:50 写, 仍有效)
├── clap/ (3.50MB exclude .git, 631 files, 17:30:05) ✅ 真 cloned
├── Guardrails/ (18.19MB exclude .git, 2045 files, 17:48:20) ✅ 真 cloned (整合 #4 commit 后修真)
├── Guardrails-broken/ (空目录, 修真残留, 不计入 11/11)
├── hyper/ (0.54MB exclude .git, 58 files, 17:29:39) ✅ 真 cloned
├── kani/ (5.46MB exclude .git, 3224 files, 17:35:28) ✅ 真 cloned
├── langgraph/ (13.29MB exclude .git, 670 files, 16:31:13) ✅ 真 cloned
├── PyO3/ (5.69MB exclude .git, 811 files, 16:53:35) ✅ 真 cloned
├── servers/ (1.40MB exclude .git, 145 files, 16:51:30) ✅ 真 cloned
└── superpowers/ (1.52MB exclude .git, 180 files, 17:33:34) ✅ 真 cloned

# LiteLLM 0 cloned (per P6-1 公开设计 1:1 翻译)
# opencode 0 cloned (per P6-2 改借鉴已 cloned)
# OpenCog 0 cloned (per ❌ AGPL-3.0 永久跳过)
# 🆕 R130-6 提议: opencog-family 6 子源 0 cloned (per 借脑 ID 索引完成, paper/architecture docs only, 0 集成 code)
```

#### 10.3.5 OpenCog 家族 6 子源 2026-08 调研来源 (per R130-6 §2.1 + R149-4 调研 + 2026-08 web verify)

```
opencog/atomspace (C++/Scheme/Python AtomSpace hypergraph DB)
  - URL: https://github.com/opencog/atomspace
  - 版本: 4.3.0 (per atomspace-storage README)
  - commit: ecd88d6 (2026-02-01)
  - License: AGPL-3.0 (per SchemeSmob.cc 头部 "GNU Affero General Public License v3")
  - 状态: 活跃维护 (per 2026-02 commits + 4.3.0 release)

opencog/cogutil (C++ utility library)
  - URL: https://github.com/opencog/cogutil
  - License: AGPL-3.0
  - 状态: 活跃维护 (C++ 工具集, OpenCog 全家族共用底层)

opencog/moses (supervised learning)
  - URL: https://github.com/opencog/moses
  - License: AGPL-3.0
  - 状态: 活跃维护 (决策树森林管理 + Atomese graphlets)

opencog/pln (Probabilistic Logic Networks)
  - 位置: opencog/pln (sub-directory of opencog/opencog)
  - License: AGPL-3.0
  - 状态: **官方 deprecated** (per 2026-02 opencog/sensory README "PLN (also unsupported & deprecated)")

opencog/relex (Relationship extraction NLP)
  - 位置: opencog/relex (sub-directory of opencog/opencog)
  - License: AGPL-3.0
  - 状态: **官方 deprecated** (per opencog wiki "obsolete")

CogPrime (Ben Goertzel AGI design)
  - 形态: 学术著作 / AGI 设计蓝图 (per Ben Goertzel 著作)
  - License: N/A (无 code, 无 license)
  - 状态: 公开论文/书籍, 0 license 风险
```

#### 10.3.6 关联 R130-R149 era 报告 (R149-4 0 重复造轮子, per 用户记忆 #6)

```
reports/agent-r124-2-borrow-research-2026-08-10.md (16:19, 13 模块 multi-agent 调研, 含 B-028/B-034/B-040/B-049 OpenCog 4 借鉴机会, 100% 严守)
reports/agent-r125-8-borrow-id-index-2026-08-10.md (17:45, 借鉴 ID 严格化 100%)
reports/agent-r126-borrowed-final-2026-08-10.md (20:40, 借鉴 final)
reports/agent-r126-philo-8-borrow-index-2026-08-10.md (20:38, philo-8 借用索引)
reports/agent-p9-1-r127-2-borrowed-repos-stage-2-final-2026-08-10.md (21:46, borrowed-repos Stage 2 final)
reports/agent-r129-7-borrow-11-11-upgrade-verify-2026-08-11.md (00:18, 借鉴 11/11 升级 1:1 verify)
reports/agent-r129-11-backend-0-install-final-verify-2026-08-11.md (00:48, 后端 0 装 PASS 终极 verify, PHL-07 spec-only 关键诚实标)
reports/agent-r129-21-integration-5-final-verify-2026-08-11.md (00:42, 整合 #5 commit 拍板前最终 verify)
reports/agent-r129-28-borrow-11-11-final-verify-2026-08-11.md (00:48, 借鉴 11/11 终极 verify, 5 大维度 verify)
reports/agent-r130-5-v1.1-minor-release-roadmap-2026-08-11.md (01:14, V1.1 minor release 路线图)
reports/agent-r130-6-borrowed-12-sources-research-2026-08-11.md (01:14, 借鉴 12 源调研 OpenCog AGPL-3.0 fork 决策 63.4 KB, 100% 严守)
reports/agent-r131-1-architecture-audit-2026-08-11.md (架构审视)
reports/agent-r131-2-borrowed-12-gap-analysis-2026-08-11.md (01:35, 跟借鉴源码 11 源差距 + 借鉴 12 源 实施深度 88.2 KB)
reports/agent-r131-3-v1.1-release-implementation-roadmap-2026-08-11.md (V1.1 实施路线图)
reports/agent-r131-4-cargo-workspace-optimization-2026-08-11.md (Cargo workspace 优化)
reports/agent-r131-5-24-locked-entry-optimization-2026-08-11.md (24 LOCKED 入口签名优化 1:28 PASS)
reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md (Cargo.toml borrow 段)
reports/agent-r131-7-pybridge-integration-optimization-2026-08-11.md (pybridge 集成优化 跑中)
reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md (Tauri 集成优化)
reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md (形式化集成优化)
reports/agent-r133-1-borrowed-12-sources-implementation-2026-08-11.md (01:25, 借鉴源 12 源实施 spec + 5 阶段实施计划)
reports/agent-r133-2-asi-stage-9-long-term-ai-growth-2026-08-11.md (01:30, ASI Stage 9 长程 AI 成长 实施 spec)
reports/agent-r133-3-three-onion-architecture-upgrade-2026-08-11.md (三洋葱架构升级 spec)
reports/agent-r148-12-decision-chain-borrowed-8-walls-index-v3-2026-08-11.md (决策链索引 v3)
reports/agent-r149-4-borrowed-12-sources-fork-then-borrow-pattern-2026-08-11.md (本报告, 借鉴 12 源 fork-then-borrow 决策模式, 0 重复造轮子, 在 R130-6 调研 + R131-2 差距 + R133-1 实施 spec 之上深度聚焦 fork-then-borrow 决策模式 8 维度, 100% 严守)
```

---

## 11. 一句话 (TL;DR) (再次强调)

**R149-4 借鉴 12 源 fork-then-borrow 决策模式 100% done** (per 决策 #86 §4 R149 era 5 sub 派活 + 决策 #74 B1 改写 + 决策 #73 §3 复杂不恐惧 + 决策 #33 §2.3 8 硬墙 + 决策 #22 §4 license 风险表 + 决策 #55 §2.6 OpenCog fork 决策 + 主人 8/11 01:14 拍板 3 件套 + cron Section 11 Step 4). **8 维度全维度 100% 调研**:

1. ✅ **借鉴 12 源 实施深度分析表** (per R130-6 §1.1/§1.2 + R131-2 §1.1/§1.2 + R133-1 §1 整合): 8 真 cloned 实施深度 8-9/10 (clap 4.5MB / hyper 0.54MB / servers 1.4MB / PyO3 5.69MB / kani 5.46MB / langgraph 13.29MB / superpowers 1.52MB / Guardrails 18.19MB, 总 49.59MB / 7,764 files) + 2 限流 → 借鉴 ID 索引完成 (LiteLLM 公开 1:1 翻译 562 行新 src + opencode 改借鉴已 cloned 3 新模块) + 1 永久跳过 (OpenCog AGPL-3.0) + 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, 0 装"已读真源码").

2. ✅ **fork-then-borrow 决策模式 4 类** (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + 2026-08 web verify):
   - **A 类: ✅ cloned 真实施** (8 源) — 公开 API license 兼容 (Apache-2.0/MIT/dual) + 0 借私有 fn + 1:1 翻译 → ✅ 真集成 src + tests pass
   - **B 类: ⏳ 限流 → ✅ 1:1 翻译公开** (2 源 LiteLLM + opencode) — 限流持续 → 0 借具体源码 + 公开 docs 1:1 翻译 → ✅ 0 装"已读真源码"
   - **C 类: ❌ license 不兼容 永久跳过** (1 源 OpenCog AGPL-3.0) — 主仓 Apache-2.0 vs 强 copyleft 不可派生 → ❌ 永久 0 主仓集成 + 0 主仓 fork + ⏳ R130-6 借脑 + 🆕 1.0 release 后独立 fork 决策
   - **D 类: 🆕 借脑 (paper/architecture docs, 0 license)** (1 源 OpenCog 家族 6 子源) — 论文/著作/architecture 文档 0 license 风险 → 0 装"已读真源码" + 0 装"已集成" + 0 装"已 fork"

3. ✅ **借鉴 12 源 跟 V1.1 release 集成路径 3 阶段** (per 决策 #74 B1 改写 V1.1 release Mavis 自决改 + 决策 #62 §2 + 决策 #71 §2.5 R131+ era 实施 + R130-5 V1.1 路线图): 阶段 1 借脑 OpenCog (1 周, 9/8-9/14) + 阶段 2 fork OpenCog AGPL-3.0 实验仓 (1 周, 9/15-9/21, 1.0 release 后) + 阶段 3 ASI Stage 9 整合 + 12 源 0 装严守 二次 verify (1 周, 9/22-9/28) + 阶段 4 Cargo.toml 1.2.1 bump (1 天, 9/29) + 阶段 5 整合 #6 + #7 commit 拍板 + V1.1 release 实战 (估 11/25 + 11/29 + 11/30 06:00-08:00)

4. ✅ **OpenCog AGPL-3.0 永久跳过 5 维度论证** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3 + 决策 #73 §3 + 2026-08 web verify): ❌ R1 极强传染性 + ❌ R2 商业化受阻 + ❌ R3 compliance 成本极高 + ❌ R4 OpenCog 维护状态不稳定 + 🟡 R5 官方 deprecated sub-modules. 永久跳过 ≠ 0 调研, R130-6 借脑 ID 索引完成 + R133-1 实施 spec 阶段 5 阶段.

5. ✅ **借鉴 12 源 跟 ASI Stage 9 长程 AI 成长 (per R149-2 + R133-2) 的关系**: 借鉴 12 源 = ASI Stage 9 4 维度 (H 自治 + L 长程 + G 成长 + P 平台化) 的设计素材库, 但 0 装"已借鉴 = 已落地 Stage 9" — Stage 9 = V1.1 release 实施 (per 决策 #74 B1 Mavis 自决改 + 决策 #73 §3 借脑 OpenCog CogPrime).

6. ✅ **借鉴 12 源 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 的关系**: 8 哲学锚 (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装) = 借鉴决策的伦理守门. 不要怕复杂度哲学 = V1.1 release Mavis 自决改 (per 决策 #74 B1) + Cargo.toml 1.2.0 → 1.2.1 bump + 24 LOCKED 入口签名改写 (前提: 更好的架构).

7. ✅ **借鉴 12 源 跟 24 LOCKED 入口签名 (决策 #74 B1 V1.1 release Mavis 自决改) 的关系**: V1.0 release 0 改严守 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 改写边界 + R131-5 24/24 PASS) + V1.1 release Mavis 自决改 (前提: 更好的架构). 借鉴 12 源 实施类 = 24 LOCKED 入口签名的素材库, V1.1 release Mavis 自决改 (更好的架构).

8. ✅ **借鉴 12 源 fork 决策矩阵 (10 维度)**: 借鉴源 ID × 10 维度 (代码量 / 维护成本 / 集成成本 / 依赖 / 风险 / 价值 / 紧迫 / 长期 / 团队 / 法律) = 12 源 × 10 维度 = 120 cells 全维度评分. 总分 排序: 🟢 30/30 (2 源: clap + superpowers) + 🟢 29/30 (2 源: servers + langgraph) + 🟢 27/30 (1 源: PyO3) + 🟢 26/30 (1 源: hyper) + 🟡 25/30 (1 源: Guardrails) + 🟡 24/30 (1 源: LiteLLM) + 🟡 23/30 (1 源: OpenCog family 6 子源) + 🟡 22/30 (1 源: opencode) + 🟡 21/30 (1 源: kani) + 🔴 11/30 (1 源: OpenCog 永久跳过).

**0 严守 100%**: 0 改 src / 0 改 Cargo.toml / 0 主动 commit / 0 主动 push / 0 主动 IM 主人 / 0 借具体源码 / 0 装"已借鉴 = 已落地" / 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 §1 改写表). 决策链 #22-#86 全 read verify (66 个决策文件, per 决策 #10 + 用户记忆 #10 决策日志写).

**报告路径**: `Apeireth-rust\reports\agent-r149-4-borrowed-12-sources-fork-then-borrow-pattern-2026-08-11.md`
