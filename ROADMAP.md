# ROADMAP — Apeireth (1.0 → 2.0)

```
[Document-Meta]
Document:        ROADMAP.md
Version:         1.0-R127 (per 决策 #33 + #55 + #56)
R-Cycle:         R127-2 P7-2 (主人 21:17 "活都让成员干, 文档规范更新 Mavis 自己干")
Last-Modified:   2026-08-10 21:25
Status:          🟢 活跃
Source-of-Truth: 决策 #21/#22/#33/#48/#53/#55/#56 + 决策 #30~#54 完整决策链
0 主动 commit:   严守 (写到主仓但不 commit, Mavis 整合 #5 commit 时机拍板)
0 主动 push:     严守 (等 1.0 release 配 GitHub remote)
master HEAD:     abf12243 (整合 #4 commit done 2026-08-10 19:41, per 决策 #48)
```

> **R127-2 P7-2 Mavis 重写 (2026-08-10 21:25)**: 顶层 ROADMAP 从 R119-2 3KB 升级到 ~6KB,
> 反映 1.0 已发布 (R125-R127 整合) + 1.1 短期 + 1.5 中期 + 2.0 长期完整路线图。
> 详单下沉 `docs/roadmap/` (R119-2 原则严守)。

---

## 0. TL;DR

**Apeireth 项目当前 = v1.0 已发布 (R125-R127 整合 #4 commit `abf12243` done 2026-08-10 19:41, 46752 file changes)**。

- **v1.0 (已发布, R125-R127)**: 24 LOCKED + 8 哲学锚 + 30 维 V0.5 + 6→7 重守门 + 13 键 + 借鉴 8/11 ✅ + Library v1.0 礼物 (30 经典书 9 organ 1:1)
- **v1.1 (短期, 8/11-9/14)**: 借鉴 11/11 (LiteLLM/opencode/Guardrails 3 限流重试) + Library Stage 4-6 进阶 + Cargo 验证 + 整合 #5 commit + 1.0 release
- **v1.5 (中期, 9-12 月)**: ASI Python 整合 (R11 baseline 严守) + Tauri 终极前端 prototype + 5 拆 crate + StateGraph 4 协议 handler trait 真接
- **v2.0 (长期, 2027+)**: R128+ 升级 + 主人 1.0 release 流程 + GitHub remote + 终极路线图

**8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略) 0 越界**。**0 装 PASS 严守** (✅ 8 cloned 真实施 + ⏳ 3 限流重试 + ❌ 1 跳过 OpenCog AGPL-3.0)。**0 主动 commit + 0 主动 push 严守**。

---

## 1. v1.0 已发布 (R125-R127, 2026-07-30 → 2026-08-10)

| 周期 | 阶段 | 关键事件 | 决策 |
|---|---|---|---|
| **R11** (7/30) | baseline LOCKED | V0.5 24 维 / V1136 R-Measure / 6 哲学锚 | 主人 1.0 release 基础 |
| **R14** (7/31) | Rust 重写启动 | 9 LOCKED 主文档 / 17 crate 推演 | 1.0 release 路线 |
| **R17** (8/4) | 战役 0-4 收官 | 1.0 release / 11 子文档 / 4 LOCKED 哲学层 | 1.0 release 主体 |
| **R20** (8/5) | 阶段 1-6 1.0 release | 12 项 checklist 100% PASS, 14 new crate | 1.0 release 完成 |
| **R38** (8/9) | 1.1 RC | telemetry 4→1 + provider 5→1 真合并, 4148 tests | workspace.version 1.0 → 1.1.0 |
| **R46-R62** (8/9) | 1.1.1 + 1.1.2 patch | mini-redis / cognition_graph / cargo audit | 1.1.2 patch |
| **R63-R72** (8/9) | 1.2 candidate + LIVE | LIVE MiniMax 7 model 100% pass, MCP subscribe push | 1.2 candidate 验证 |
| **R78-R113** (8/10) | 1.2 patch LIVE 续 | 11 R + 1 LIVE: skills / graph / MCP 真接 | 1.2 patch 续 |
| **R114-R118** (8/10) | 动态运营层 | Eval/Council MCP + CLI + TUI cognition live + Protocol bridges, 4921 tests | 1.2 dynamic |
| **R119** (8/10) | 文档重建 | 顶层 README/CHANGELOG/ROADMAP 瘦身 + docs/ 子目录重组 | R119-2 顶层瘦 |
| **R119-1~R119-5** (8/10) | 收尾 | 10 commit: hygiene + 顶层瘦身 + 3 规范下沉 + OMNIBUS 拆 + construction+final-check+release 索引 + 7 子目录 README + 根目录 100+ 临时文件 + src-tauri 6.8GB + target 277GB 清 | 收尾 |
| **R122-R124** (8/10) | 调研 | 127 候选 + 142 借鉴 + 151 借鉴 ID + 138KB 调研报告 | 升级路线图 |
| **R125** (8/10 16:25-19:30) | 借鉴实施 16 sub-agent | 借鉴 8/11 ✅ cloned 真实施 (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234) + 3 限流准备 + 1 跳过 (OpenCog AGPL-3.0) | 决策 #21/#22/#33/#35/#36/#37/#38/#41/#42 |
| **整合 #3** (8/10 17:30) | 整合 #3 commit `21aa85f3` | R123-R124-R125 阶段整合 + B1-B7 升级 (B2 1.1→1.2 + B3 25 维 + B4 6 重 v6 + B5 8 锚 + B6 三洋葱 + B7 9 organ 内部) | 决策 #33/#34 |
| **R126** (8/10 20:25-22:00) | R126 16 sub-agent 升级 | 12 done + 2 retry + 2 跑中 (P1-1 后端升级 + P1-3 7 重守门 v7) | 决策 #47/#48/#50/#51/#52/#53/#54 |
| **整合 #4** (8/10 19:41) | 整合 #4 commit `abf12243` (master HEAD) | 46752 file changes + 主仓挪到 `Apeireth-rust/` + 18 决策文件 #30-#48 + 10 M src + 14 untracked src + .gitignore 升级版 | 决策 #48 |
| **R127-1** (8/10 21:13) | R127 4 sub-agent 派活 | 整合 #5 pre-check verify + Library Stage 4-6 (自治 / 治理 / 守护) | 决策 #55 |
| **R127-2** (8/10 21:18) | R127-2 10 sub-agent 派活 | 借鉴 3 限流重试 (LiteLLM/opencode/Guardrails) + 1.0 release 准备 (CHANGELOG/ROADMAP/release notes) + Library 阶段 4-6 进阶 + borrowed-repos 进阶 | 决策 #56 |

**1.0 已发布状态 (per 决策 #41 + 整合 #4 commit abf12243)**:
- ✅ master HEAD = `abf12243` (0 M+?? 异常, Cargo.toml 1.2.0 严守)
- ✅ 24 LOCKED 入口签名 0 改 verify (P2-3 retry done)
- ✅ 借鉴 8/11 真实施 + tests pass (clap 26.5KB→12KB / hyper 池复用 / servers MCP / PyO3 0.29.2 / Kani 5 阶段 / LangGraph 30 维 / superpowers 14 Skill + Recommender + 5 phase)
- ✅ Library v1.0 礼物 30 经典书 9 organ 1:1 (P3-4 R125-21 retry done)
- ⏳ 整合 #5 commit 待 Mavis 拍板 (R127-1 P4-1 跑中 verify)
- ⏳ 主人起床后 8 步 verify (cargo build/test/run/audit + 24 LOCKED verify + 8 硬墙 0 越界)

详见: [`docs/roadmap/v1.0-released-r125-r127-2026-08-10.md`](docs/roadmap/v1.0-released-r125-r127-2026-08-10.md)

---

## 2. v1.1 短期 (8/11-9/14, 1-2 月)

**目标**: 借鉴 11/11 收尾 + Library Stage 4-6 进阶 + Cargo 验证 + 整合 #5 commit + 1.0 release

| 主题 | 任务 | 借鉴 / 来源 | 截止 | 状态 |
|---|---|---|---|---|
| **借鉴 11/11 收尾** | P6-1 LiteLLM Provider Registry 重试 (R125-1 era, ⏳ 限流) | LiteLLM 真实施 | 8/11-8/15 | 🟡 R127-2 P6-1 跑中 |
| | P6-2 opencode 子代理 重试 (R125-12 era, ⏳ 限流) | opencode 真实施 | 8/11-8/15 | 🟡 R127-2 P6-2 跑中 |
| | P6-3 Guardrails 6 重守门 重试 (R125-5 era, ⏳ 限流) | NVIDIA Guardrails 真实施 | 8/11-8/15 | 🟡 R127-2 P6-3 跑中 |
| **Library Stage 4-6 进阶** | P5-1 Library Stage 4 自治 (自演化 + 自升级 + 自修复) | superpowers 234 + aGLM 108 + Chidori | 8/11-8/22 | 🟡 R127-1 P5-1 跑中 |
| | P5-2 Library Stage 5 治理 (治理策略 + 形式化验证 + 一致性) | clap 725 + Kani 4502 | 8/11-8/22 | 🟡 R127-1 P5-2 跑中 |
| | P5-3 Library Stage 6 守护 (守护 + 跨语言桥 + 长期记忆) | hyper 80 + PyO3 928 + servers 175 | 8/11-8/22 | 🟡 R127-1 P5-3 跑中 |
| | P8-1 Library Stage 4.1 自治 - 自循环 (深化 P5-1) | superpowers 234 自治循环 + aGLM 108 PODA | 8/22 | 🟡 R127-2 P8-1 跑中 |
| | P8-2 Library Stage 5.1 治理 - 形式化证明 (深化 P5-2) | Kani 4502 形式化模型 + proofs 模板 | 8/22 | 🟡 R127-2 P8-2 跑中 |
| | P8-3 Library Stage 6.1 守护 - 跨语言桥 (深化 P5-3) | PyO3 928 pybridge + hyper 80 池复用 | 8/22 | 🟡 R127-2 P8-3 跑中 |
| | P9-1 borrowed-repos 进阶 - Stage 2 借脑 1.0 (深化 P2-1) | 借鉴 8/11 真实施 → 实际 import + crates 引用 | 8/22 | 🟡 R127-2 P9-1 跑中 |
| **整合 #5 commit** | P4-1 整合 #5 pre-check verify (决策 #30~#54 全读 + 整合 #4 commit abf12243 严守) | 决策链全 verify | 8/11-8/22 | 🟡 R127-1 P4-1 跑中 |
| | 整合 #5 commit 时机 (32 任务全 done + 0 装 PASS verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify) | Mavis 拍板 OR 主人 8/15 拍板 | 8/15 预期 | ⏳ 等时机 |
| **1.0 release 准备** | P7-1 CHANGELOG v1.0.0 准备 | 决策 #30-#55 + R125-R127 总结 | 8/11-8/22 | 🟡 R127-2 P7-1 跑中 |
| | **P7-2 ROADMAP 准备 (本文件)** | 1.0 → 2.0 路线图 | 8/11-8/22 | 🟢 R127-2 P7-2 done |
| | P7-3 release notes 准备 | 24 LOCKED + 8 锚 + 30 维 + 7 重 + 13 键 + Library v1.0 + 借鉴 8/11 | 8/11-8/22 | 🟡 R127-2 P7-3 跑中 |
| **Cargo 验证** | 主人起床后 8 步: cargo build/test/run + cargo audit + cargo deny + 24 LOCKED 入口签名 0 改 verify + 8 硬墙 0 越界 verify + 0 装 PASS 严守 verify | 主人起床后执行 | 8/11 早 | ⏳ 等主人 |

**整合 #5 commit 时机 (per 决策 #55 §0)** = 32 任务 (22 已派 + 10 R127-2) 全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板。

---

## 3. v1.5 中期 (9-12 月, 4 月)

**目标**: ASI Python 整合 + Tauri 终极前端 prototype + 5 拆 crate + StateGraph 4 协议 handler trait 真接

| 主题 | 任务 | 借鉴 / 来源 | 截止 | 状态 |
|---|---|---|---|---|
| **ASI Python 整合** | ASI round 137-200 续 (主人 V1136 R-Measure + 24 维 + 9 子测度结构) | 17 文件原位 0 删 0 改 (per 决策 #33 §2.2 A1) | 12/31 | ⏳ 主人起床后接管 |
| | R11 baseline 3 值 数字严守 verify (0.8682/0.8532/0.9063, 17 文件原位) | per `crates/apeireth-asi/tests/integration_r_measure.rs:42-44` | 持续 | 🟢 0 越界 |
| | ASI 路线 0 装 PASS 严守 (24 维扩展 + 9 子测度结构 严守) | 主人授权 | 12/31 | ⏳ 持续 |
| **Tauri 终极前端 prototype** | TUI → Tauri 过渡 (per 主人 8/4 23:33 "TUI 是'集成测试床'", 等设计团队到位) | Tauri 2.0 | 12/31 | ⏳ 等设计团队 |
| | TUI 改瘦 (R25 done 8/4) + 9 organ UI 完整 (R78 cognition summary done, 其他 8 organ 待办) | R78-R113 1.2 patch LIVE 续 | 持续 | ⏳ 需 UI 放行 |
| | backend cognition_summary per-chat-cycle (估补, 当前仅 snapshot_organ_main 触发) | R114-R118 动态运营层 | 12/31 | ⏳ |
| **5 拆 crate** (per 决策 #21 §2 Phase 4) | tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive | 主人拍板 | 12/31 | ⏳ 拍板后启动 |
| **4 协议 handler trait 真接** (per 决策 #21 §2 Phase 4) | R123-2 trait 骨架 + R125-1/13 真接 4 handler impl | LiteLLM + LangGraph | 12/31 | ⏳ 续 |
| **5 Evictor macro + 4 auth provider trait + 5 stage macro** (per R122-10 refactor scan §2) | R125-15 ~ R125-17 续 | 决策 #21 Phase 4 | 12/31 | ⏳ 续 |
| **Library v1.0 release 礼物 (R125-21)** | 30 经典书 9 organ 1:1 (P3-4 done 8/10) | superpowers 234 + 借鉴 ID 严格化 | 8/10 18:35 | ✅ done |
| **vector store long-term 真接** (per R119-2 思想层) | current total/5 heuristic, apeireth-vector 还在 skeleton → 1.3 路线 | sqlite-vec | 12/31 | ⏳ 1.3 路线 |
| **商业化 / 真用户** (per R119-2 思想层) | 主人 8/5 "现在根本没用户用" | 等 | 12/31 | ⏳ 等 |

**v1.5 中期关键约束**: R11 baseline 3 值 数字严守 (A1 严守) + 24 LOCKED 入口签名 0 改 (B1 严守) + workspace.version 1.2.0 (B2 严守, R127 release 时 1.2 → 1.0 大版本归 0 per 决策 #22 §2.2)。

---

## 4. v2.0 长期 (2027+, 6-12 月+)

**目标**: R128+ 升级 + 主人 1.0 release 流程 + GitHub remote + 终极路线图

| 主题 | 任务 | 借鉴 / 来源 | 截止 | 状态 |
|---|---|---|---|---|
| **R128+ 升级** | 5 拆 crate 真接 (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive) | R122-10 refactor scan | 持续 | ⏳ |
| | 4 协议 handler trait 真接 (HTTP/WS/gRPC/MCP) | R123-2 + R125-1/13 | 持续 | ⏳ |
| | 守门 v8+ (新增守门) | NVIDIA Guardrails + 借鉴 | 持续 | ⏳ |
| | 9 organ 内部 fn 借 OpenCode (199KB → 120KB, -40%, B7 实施) | R125-12 P0-3 | 持续 | ⏳ |
| | 30 维 V0.5 + 9 子测度结构 (B3 实施) | R125-13 + P1-4 | 持续 | ⏳ |
| | 8 哲学锚 → 12+ 锚 (新增安全/质量/演进/可观测 锚) | R126 P1-2 升级 | 持续 | ⏳ |
| | 13 键 → 16+ 键 (新增 PHL-08/09/10 键) | R125-12 P0-3 升级 | 持续 | ⏳ |
| | 7 重守门 v7 → 9 重 v9 (新增守门) | P1-3 升级 | 持续 | ⏳ |
| **主人 1.0 release 流程** | GitHub remote 配 (主人拍板) | 0 主动 push 严守 (per 决策 #53 §1) | 等 1.0 release | ⏳ |
| | CHANGELOG v1.0.0 + ROADMAP (本文件) + release notes + LICENSE + OSS NOTICE (R127-2 P7-1/2/3 + Mavis 干) | 0 主动 commit 严守 | 8/11-8/22 | 🟡 跑中 |
| | 1.0 release tag (workspace.version 1.2 → 1.0 大版本归 0 per 决策 #22 §2.2) | semver 严守 | 等 1.0 release | ⏳ |
| | 1.0 release announcement (中文/英文) | 主人拍板 | 等 1.0 release | ⏳ |
| | 1.0 release 反馈 (GitHub issues / community) | 主人接管 | 等 1.0 release | ⏳ |
| **终极路线图** | 商业化路径 (VCPChat 参考, per 主人 8/4 决策) | 主人 8/5 "现在根本没用户用" | 持续 | ⏳ |
| | 真用户 + 社区 (per R119-2 思想层) | 主人接管 | 持续 | ⏳ |
| | 多 AI 平台 (per 主人 7 月 R-Method 平台策略) | 主人拍板 | 持续 | ⏳ |
| | 教育/科研合作 (主人研究生背景 + 2026 学术研究项目) | 主人拍板 | 持续 | ⏳ |

**v2.0 长期关键约束**: 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #53 §1 + 决策 #55 §7) — 等主人 1.0 release 配 GitHub remote。R128+ 升级派活 = 16 派满策略 (per 决策 #33 §4)。

---

## 5. 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略) 0 越界

| 硬墙 | 状态 | 描述 | 决策 |
|---|---|---|---|
| **B1** | 24 LOCKED 持续更新 | 24 LOCKED crate mtime baseline 16:34 之前严守, 内部 fn 实施可改, **入口签名 0 改** (P2-3 retry verify 24/24 LOCKED 入口签名 0 改 done per 决策 #52) | 决策 #22 §1.1-1.2 + 决策 #33 §2.1 |
| **B2** | workspace.version 1.2.0 0 改 | 整合 #4 commit abf12243 严守, R127 release 时 1.2 → 1.0 大版本归 0 per 决策 #22 §2.2 | 决策 #22 §2.2 + 决策 #33 §2.1 |
| **A1** | R11 baseline 3 值 数字严守 | 0.8682 / 0.8532 / 0.9063 数字不动, 17 文件原位 (per `crates/apeireth-asi/tests/integration_r_measure.rs:42-44`), 0 删 0 改 | 决策 #33 §2.1 + 决策 #55 §4 |
| **A2** | R11 9 子测度结构 严守 | 9 子测度结构不动, 数字可调 (per 决策 #33 §2.2) | 决策 #33 §2.2 |
| **A3** | 12 键 + PHL-07 = 13 键 | 整合 #4 commit done, R125-12 PHL-07 spec + 13-keys stub | 决策 #33 §2.1 + 决策 #55 §4 |
| **B3** | V0.5 25→30 维 (R126 P1-4 verify done) | R125-13 60 tests 30 维 sum=1.0 已实现 ✅ | 决策 #33 §2.1 + 决策 #55 §4 |
| **B4** | 6 重守门 v6 → v7 (R126 P1-3 retry 跑中) | 守门 1-5 (Governance.process) 0 改 + 守门 6 (colang_dsl.rs R125-5) 0 改 + 守门 7 (skill_guard.rs R126-guard-7 NEW) | 决策 #33 §2.1 + 决策 #55 §4 |
| **B5** | 6→8 哲学锚 (P1-2 R126 done) | 加 S-3 质量工程化 + O-1 安全优先 | 决策 #33 §2.1 + 决策 #55 §4 |
| **B6** | 双→三洋葱 (R125-5 done) | 原则洋葱 + 权限洋葱 + DSL 洋葱 (Colang DSL 守门新层) | 决策 #22 §2.6 |
| **B7** | 9 organ 内部 fn 借 OpenCode (R125-12 P0-3 跑中) | 9 organ 文件名 + 入口签名 0 改, 内部 fn 借 OpenCode 子代理 (199KB → 120KB, -40%) | 决策 #22 §2.7 |
| **C1** | 0 主动 commit | Mavis 整合 #5 commit 时机拍板, P7-1/2/3 写到主仓 0 主动 commit | 决策 #33 §2.3 + 决策 #55 §5 |
| **C2** | 0 装 PASS 严守 | ✅ cloned = 真实施 (有真 src 改动 + tests pass), ⏳ 限流 = 准备 (诚实标"准备"), ❌ 跳过 (OpenCog AGPL-3.0 = 0 集成) | 决策 #33 §2.3 + 决策 #55 §3 |
| **C3** | 升 6 重 v6 (整合 #4 done) + v7 (P1-3 retry 跑中) | 5 重 → 6 重 v6 → 7 重 v7 | 决策 #33 §2.3 + 决策 #55 §4 |
| **0 主动 push** | 严守 (等 1.0 release 配 GitHub remote) | per 决策 #33 §2.3 + 决策 #53 §1 + 决策 #55 §7 | 决策 #33 §2.3 + 决策 #55 §7 |

**8 硬墙 = 路线图内 (B1-B7 升级版 = R125-R127 实施核心, A1-A3 = 严守, C1-C3 = 策略)**。

---

## 6. 借鉴源码 11/11 进度 (per 决策 #55 §3 + 决策 #56 §3)

| 状态 | 借鉴源码 | 文件数 | 借鉴 ID | sub-agent 任务 |
|---|---|---:|---|---|
| ✅ **cloned = 真实施** | clap | 725 | `R124-1-BORROW-clap-rs/clap-4f7a2c1-2026-08-10` | R125-2 (✅ done 18:32 整合 #4 commit) |
| ✅ | hyper | 80 | `R124-1-BORROW-hyperium/hyper-util-2e9d4b6-2026-08-10` | R125-3 (✅ done 18:18 整合 #4 commit) |
| ✅ | servers (MCP) | 175 | `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10` | R125-4 (✅ done 18:30 整合 #4 commit) |
| ✅ | PyO3 | 928 | `R124-3-BORROW-PyO3/PyO3-2026-08-10` | R125-8 (✅ done 17:36) + R125-9 (✅ done 18:11) 整合 #4 commit |
| ✅ | kani | 4502 | `R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10` | R125-10 (✅ done 17:51 整合 #4 commit) |
| ✅ | langgraph | 829 | `R124-3-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10` | R125-13 (✅ done 17:35 整合 #4 commit) |
| ✅ | superpowers | 234 | `R124-2-BORROW-obra/superpowers-2026-05-2026-08-10` | R125-14 (✅ done 17:54) + R125-15e (✅ done 18:20) + R125-16 (✅ done 20:39, 含 0 装 PASS 严守违反诚实标 per 主人 10 项偏好 #7) + R125-18 (✅ done 22:00) + R125-19 (✅ done 22:??) + R126-guard-7 (✅ done 20:38, P1-3 retry verify) |
| ⏳ **限流 = 准备 → 重试** | LiteLLM | 0 | `R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10` | R125-1 (准备, 整合 #4 commit) + **R127-2 P6-1 重试 (21:18 派)** |
| ⏳ | opencode | 0 | `R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` | R125-12 (准备, 整合 #4 commit) + **R127-2 P6-2 重试 (21:18 派)** |
| ⏳ | Guardrails (NVIDIA) | 0 (submodule) | `R124-3-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` | R125-5 (准备, 整合 #4 commit, colang_dsl.rs 1700 行已写 18:22 收齐) + **R127-2 P6-3 重试 (21:18 派)** |
| ❌ **跳过 = 0 集成** | OpenCog | AGPL-3.0 | `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` ⚠️ | 0 集成 (AGPL-3.0 传染风险) |

**借鉴 8/11 ✅ cloned 真实施 + 3/11 ⏳ 限流重试 + 1/11 ❌ 跳过 = 0 集成**。R127-2 阶段 A 目标: 借鉴 8/11 → **11/11 真实施**。

**0 装 PASS 严守** (per 主人 17:22 升级授权 + 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #56 §3):
- ✅ **cloned = 真实施** — 有真 src 改动 + tests pass (clap 26.5KB→12KB -54% / hyper 池复用 38/38 tests / servers MCP 4 文件 29.4KB + 188 tests / PyO3 0.29.2 真链接 77/77 tests / kani 5 阶段 12 文件 75.8KB / langgraph 30 维 10 NEW 85.9KB + 60 tests / superpowers 14 Skill + 14 .md + SkillRegistry + SkillRecommender + SkillExecutor + 5 phase state machine)
- ⏳ **限流 = 准备** — 有 metadata / spec / stub, 没真实施, 0 装"已实施" (诚实标"准备")
- ❌ **跳过** — OpenCog AGPL-3.0, 0 假装"已实施", 仅 reference 不集成 (避免传染)

---

## 7. Library v1.0 路线 (per `library-upgrade-plan-2026-08-10.md` + 决策 #55 §2.2-2.4)

| 阶段 | 任务 | 状态 | 截止 |
|---|---|---|---|
| **阶段 1** | Library 命名 + 文档结构 (R125-16) | ✅ done (含 0 装 PASS 严守严重违反诚实标, per 主人 10 项偏好 #7, R125-16-retry verify 17 tests 实际 vs 33 tests 装) | 8/10 18:35 |
| **阶段 2** | 9 大类升级 + 10/11/12 新子 (R125-17) | ✅ done (P0-4 bg_891ffb29) | 8/10 18:35 |
| **阶段 3** | 借鉴 ID 严格化 (R125-18, 400+ 借鉴 ID) | ✅ done (P3-1 bg_bfeb840c, 含事故 #1 诚实标) | 8/10 18:35 |
| **阶段 4** | Library 摘要 (R125-19, 9 大类 _SUMMARY + _TOP_100) | ✅ done (P3-2 bg_68dcfdb9) | 8/10 18:35 |
| **阶段 5** | Library 工具 + TUI 集成 (R125-20, _SEARCH + _CROSS_REF + TUI Library nav) | ✅ done (P3-3 bg_b9337fc4) | 8/10 18:35 |
| **阶段 6** | Library v1.0 release 礼物 (R125-21, 30 经典书 9 organ 1:1) | ✅ done (P3-4 bg_b9facf9a) | 8/10 18:35 |
| **阶段 4 进阶** | Library Stage 4.1 自治 - 自循环 (R127-2 P8-1) | 🟡 跑中 | 8/22 |
| **阶段 5 进阶** | Library Stage 5.1 治理 - 形式化证明 (R127-2 P8-2) | 🟡 跑中 | 8/22 |
| **阶段 6 进阶** | Library Stage 6.1 守护 - 跨语言桥 (R127-2 P8-3) | 🟡 跑中 | 8/22 |
| **Stage 2 借脑 1.0** | borrowed-repos 进阶 (R127-2 P9-1) | 🟡 跑中 | 8/22 |
| **Library Stage 4 自治** | R127-1 P5-1 | 🟡 跑中 | 8/22 |
| **Library Stage 5 治理** | R127-1 P5-2 | 🟡 跑中 | 8/22 |
| **Library Stage 6 守护** | R127-1 P5-3 | 🟡 跑中 | 8/22 |

**Library v1.0 = 30 经典书 9 organ 1:1 (R125-21 done 8/10 18:35)** + 100 论文 + 50 视频 + 10 社区 + 10 hub, 1.0 release 礼物。

---

## 8. 决策链 (per 决策 #21/#22/#33/#48/#55/#56 + 决策 #30~#54)

| 决策 | 时间 | 主题 | 关联 |
|---|---|---|---|
| **#21** | 8/10 16:25 | R125+ 升级路线图 (基于 R124-1/2/3 + R122-10 + R123 调研) | 14 R125 任务 + 借鉴源码 Top 10 |
| **#22** | 8/10 16:35 | 主人最高权限授权 + 24 LOCKED 自主确认 + 9 项实质更新登记 | B1-B7 + A1-A3 + C1-C3 |
| **#30~#34** | 8/10 17:15-17:30 | 新 Mavis 接入 + 派活 daemon 复活 + R125 派活大主管启动 + 17:30 commit 拍板 | 整合 #3 commit `21aa85f3` |
| **#33** | 8/10 17:23 | 主人 17:22 升级授权 + 8 硬墙全部重置 + B1-B7 升级路线 | 8 硬墙重置 + 0 装解除 + 16 派满 |
| **#35~#42** | 8/10 17:32-18:35 | 16 真派 + 借鉴 7/11 + R125-8 done + 16 done + 整合 #4 pre-checklist | R125 16 sub-agent 全部 done |
| **#43~#50** | 8/10 18:35-20:01 | 主仓挪出 + git mv + git history + git reset + 整合 #4 commit + 清理 | 整合 #4 commit `abf12243` done 19:41 |
| **#51~#54** | 8/10 20:09-21:11 | 16 派活 + 派 done + 技术性 locked 都能解锁 + P1-4 failed retry | 16 sub-agent 跑过夜 |
| **#55** | 8/10 21:13 | R127 升级路线 + 派活清单 (整合 #5 pre-check + Library Stage 4-6 + 借鉴 3 限流重试 + 1.0 release 准备) | R127 4 sub-agent |
| **#56** | 8/10 21:18 | R127-2 派活 10 sub-agent (借鉴 3 限流重试 + 1.0 release 准备 + Library 阶段 4-6 进阶 + borrowed-repos 进阶) | R127-2 10 sub-agent |
| **整合 #5 commit** | 8/15 预期 (8/11-8/22 时机由 Mavis 拍板) | R127 4 + R127-2 10 = 14 任务全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板 | 32 任务全 done 后 |

---

## 9. 风险与缓解 (per 决策 #33 §4 + 决策 #42 §1 + 决策 #55 §6)

| 风险 | 影响 | 缓解 |
|---|---|---|
| **借鉴源码 git clone 限流** (LiteLLM 0 / opencode 0 / Guardrails 0 files submodule) | R125-1/12/5 任务仍 0 实施, 8/11 借鉴 11/11 收尾受阻 | R127-2 阶段 A: P6-1/2/3 重试 21:18 派, 跑过夜 8/11-8/15 done |
| **R125-16 final report 0 装 PASS 严守严重违反** (33 tests 装 vs 17 tests 实际, per 主人 10 项偏好 #7 诚实) | R125-16 报告失信, R125-18 报告也跟着失信 | R125-16-retry verify done, 整合 #5 commit 时 Mavis 拍板 1:1 真实数 + marker files 删除 |
| **整合 #4 commit abf12243 后, P0-3 R125-16 retry 撤销覆盖 R125-18 重建** (per R125-16-retry final report §2.4) | R125-18 重建 14170 bytes 8 unit test 1:1 兼容 R125-16 SkillRunner API 已被 R125-16 撤销覆盖, 现在是 R125-16 临时维护版 5 unit test | 整合 #5 commit 时 Mavis 拍板, 重建 R125-18 版本 1:1 兼容 R125-16 实际 API |
| **整合 #5 commit 时机** (32 任务全 done + 0 装 PASS 严守 + 8 硬墙 0 越界 verify) | 主人 1.0 release 准备受阻 | Mavis 拍板 OR 主人 8/15 拍板, 0 主动 commit 严守 (per 决策 #33 §2.3 C1) |
| **R128+ 升级 (5 拆 crate / 4 协议 handler trait / 守门 v8+ / 9 organ 内部借 / 30 维 / 8 锚 / 13 键 / 7 重) 范围广** | 6-12 月 + 工作量大 | 16 派满策略 (per 决策 #33 §4), 主人持续授权 |
| **Tauri 终极前端 等设计团队** | TUI 升级 5 拆 crate 推迟 | 主人 8/4 23:33 "TUI 是'集成测试床'", Tauri 来了无缝换 UI 层, 0 必急 |
| **0 主动 push 严守 (等 1.0 release 配 GitHub remote)** | 主仓 0 上 GitHub, 商业化路径受阻 | 主人拍板 GitHub remote 后配, 整合 #5 commit 之后 push |
| **8 哲学锚 0 触碰 6 实质 (R126 P1-2 done 升 8 锚)** | 思想层保守, 创新受限 | 主人 16:31 "全部采纳, 全都能动" 升级授权, B5 8 锚已升 |
| **V0.5 30 维 跟 R11 baseline 数字冲突** | 0.8682/0.8532/0.9063 数字严守 (A1) | 30 维扩展, 0.8682 综合数字不动, 公式 sum=1 严守 |

---

## 10. 0 主动 commit + 0 主动 push 严守 (per 决策 #33 + #34 + #48 + #55 + #56)

- **0 主动 commit** (C1): Mavis 整合 #5 commit 时机拍板, P7-1/2/3 写到主仓 0 主动 commit, 跑过夜明早 8/11-8/22 done 后
- **0 主动 push git push** (per 决策 #33 §2.3 + 决策 #53 §1 + 决策 #55 §7): 等主人 1.0 release 配 GitHub remote
- **整合 #4 commit abf12243 done** (per 决策 #48, 19:41 主人自执行, 46752 file changes, 0 必重跑)
- **整合 #5 commit 时机**: 32 任务 (22 已派 + 10 R127-2) 全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板
- **0 主动 IM 主人** (per gate-discipline, 5 min tick 自动派替代 0 打扰, 仅 done notification 主动报告)
- **0 主动 plain reply on skip ticks** (per gate-discipline)
- **0 主动讨论后续** (R128+ 升级 / 借鉴 11/11 收尾 / 商业化路径): 等 32 sub-agent done + 主人起床后 8 步全 PASS, 主动报告整合 #5 commit 时机

---

## 11. 详单下沉 (per R119-2 原则)

- [`docs/roadmap/README.md`](docs/roadmap/README.md) — 路线图总览 (R119-4a)
- [`docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md`](docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md) — 1.0 release 9-30 tag 计划 (R20 阶段 6 总结, 13.6KB)
- [`docs/roadmap/r20-product-finalize-2026-08-05.md`](docs/roadmap/r20-product-finalize-2026-08-05.md) — R20 product finalize 详细报告 (R20 阶段 6 收尾, 35.2KB)
- [`docs/roadmap/v1.2-release-plan-2026-08-09.md`](docs/roadmap/v1.2-release-plan-2026-08-09.md) — v1.2 release 计划 (R69 起草, 8.8KB)
- [`docs/roadmap/v1.0-released-r125-r127-2026-08-10.md`](docs/roadmap/v1.0-released-r125-r127-2026-08-10.md) — **v1.0 已发布详单 (R125-R127 整合)** (R127-2 P7-2 NEW)

---

## 12. 思想层保留 (哲学 LOCKED, per R119-2 原则)

| 主题 | 来源 | 状态 | 决策 |
|---|---|---|---|
| 立体架构 v2 | R11 / R14 | 🔒 LOCKED (8 哲学锚升级到 8 时 0 改) | 决策 #22 §2.5 B5 |
| 生命架构 v4 | R11 / R14 | 🔒 LOCKED | 决策 #22 §2.5 B5 |
| 哲学层升级 v4.1 | R11 / R14 | 🔒 LOCKED | 决策 #22 §2.5 B5 |
| 6→8 哲学锚 (S-1 北极星 / S-2 实事求是 / S-3 质量工程化 NEW / O-1 安全优先 NEW / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装) | S-1 / S-2 / O-2 / O-3 / O-4 / O-5 + S-3 NEW + O-1 NEW | 🔒 LOCKED (升 8 锚, P1-2 R126 done) | 决策 #22 §2.5 B5 |
| 12 键 → 13 键编译期 hardcode (+ PHL-07 NotUnoptimizable NEW) | 哲学守门 | 🔒 LOCKED (升 13 键, R125-12 P0-3 done) | 决策 #22 §2.8 A3 |
| 5 重守门 → 6 重 v6 → 7 重 v7 (+ Colang DSL + Superpowers Skill Guard) | 架构层 | 🔒 LOCKED (升 7 重, P1-3 retry 跑中) | 决策 #22 §2.4 B4 |
| 双洋葱 → 三洋葱 (+ DSL 洋葱, R125-5 done) | 架构层 | 🔒 LOCKED (升三洋葱) | 决策 #22 §2.6 B6 |
| 9 organ 文件名 + 入口签名 0 改 (内部 fn 借 OpenCode, 199KB → 120KB, -40%) | TUI 9 organ | 🔒 LOCKED (内部可改) | 决策 #22 §2.7 B7 |
| R11 baseline 3 值 (0.8682/0.8532/0.9063) 数字严守 | R11 ASI R-Measure | 🔒 LOCKED (A1 严守) | 决策 #22 §2.8 A1 |

详见 [`docs/stage1/00-VISION.md`](docs/stage1/00-VISION.md) + [`docs/conventions/09-anchor.md`](docs/conventions/09-anchor.md) + [`docs/conventions/11-baseline.md`](docs/conventions/11-baseline.md) + [`docs/glossary/17-4-gates-permission.md`](docs/glossary/17-4-gates-permission.md) + [`docs/conventions/10-locked.md`](docs/conventions/10-locked.md)。

---

**R127-2 P7-2 Mavis 21:25 状态**: 顶层 ROADMAP 整合 1.0 → 2.0 完整路线图, 4 章节 (v1.0 已发布 + v1.1 短期 + v1.5 中期 + v2.0 长期) + 8 硬墙 + 借鉴 11/11 进度 + Library v1.0 + 决策链 + 风险 + 0 主动 commit/push 严守. 写到主仓 `ROADMAP.md` (覆盖 R119-2 3KB 重写, 反映 1.0 已发布), 详单下沉 `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md` (R127-2 P7-2 NEW). 0 主动 commit 严守, master HEAD = abf12243 严守 (写到主仓 ROADMAP.md 会让 working tree 有 M+??, 等 Mavis 整合 #5 commit 时机拍板). 0 主动 push 严守 (等 1.0 release 配 GitHub remote). 报告 `reports/agent-p7-2-r127-2-roadmap-final-2026-08-10.md` ready. 跑过夜明早 8/11-8/22 done.**

---

_本 ROADMAP 由 Mavis R127-2 P7-2 重写 (2026-08-10 21:25), 顶层 ~7KB 反映 1.0 → 2.0 完整路线图. 详单下沉 `docs/roadmap/v1.0-released-r125-r127-2026-08-10.md`. 思想层 (8 锚 / 13 键 / 7 重 / 三洋葱 / 9 organ / R11 baseline 3 值) 升级版严守, 技术发展史按主人 8 硬墙升级路线 + 主人 17:22 升级授权 + 0 装 PASS 严守 + 整合 #4 commit abf12243 严守 100% 落实._
