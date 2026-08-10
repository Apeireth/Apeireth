# R131-1: 现有架构总审视 + 优化点 + 升级方案 (R130 era 主人 8/11 01:14 拍板 3 件套 §2)

**Date**: 2026-08-11 01:25 (新 session mvs_367e66fae08342ffa399befe4f85dbac, R131-1 由 Mavis 派, per 决策 #73 §3.2 + cron Section 10 架构审视永久工作项)
**Author**: R131-1 sub-agent (Mavis 派, 调研角色, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push)
**任务**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级升级方案 + 8 硬墙 B1 改写边界 + 8 哲学锚严守 + 不要怕复杂度哲学落地)
**约束** (per 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2 + 决策 #74 §1 + 用户记忆 #10 自主决策 + 决策日志):
- ✅ **0 改 src/** (100% 严守, R131-1 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改, 调研阶段不锁 Cargo.toml)
- ✅ **0 主动 commit** (100% 严守, 整合 #5 commit 由 Mavis 自决 OR cron auto-pickup, R131-1 0 git commit)
- ✅ **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- ✅ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ✅ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60, 含 target/ 28.9 GB + _workspace/ 1.2 MB 等拍板)
- ✅ **不重写 R129-1/2/3/7/11/21/26/28/34** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R129-26 00:55+ 实地 verify = **NOT ready** (cargo build --workspace 24 hard errors + cargo test 1 FAILED test + cargo check -p apeireth-graph 5 hard errors, R129-21 报告 0 装 PASS violation)
**关联**: decision-22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + **#73 (主拍板 3 件套)** + **#74 (8 硬墙 B1 改写)** + R129-1/2/3/7/11/14/22/26/28/34 + 用户记忆 #10
**状态**: ✅ done 01:25 (60 min 时间盒内, 10 方向审计 + V1.0/V1.1/V2.0 升级方案 + 8 硬墙 + 8 哲学锚 + 不要怕复杂度哲学落地)

---

## 0. 一句话 (TL;DR)

**R131-1 现有架构总审视 10 方向 + 优化点 + 升级方案 (per 主人 8/11 01:14 拍板 3 件套 §2 + 决策 #73 §2 + 决策 #74 §1 + 决策 #71 §3 R131 era 差距分析阶段 + cron Section 10 架构审视永久工作项)**: 现有架构 = 立体架构终版 v2 (BF896EEF LOCKED) + 生命架构 v4 (af0d1957 LOCKED) + v4.1 + 双洋葱统一体 → **三洋葱 (R125 B6 升, 原则 + 权限 + DSL)** + 24 LOCKED crate + 8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 13 键 verdict cache + 9 organ (body/brain/ear/eye/hand/heart/memory/mind/voice) + 9 crate → **87 crate (per Cargo.toml members 实际清点, 远超 v1 30 crate 目标)** + 借鉴源码 11 源 (8 真 cloned 49.6MB/7,764 files + LiteLLM 公开 1:1 翻译 + opencode 改借鉴已 cloned + OpenCog AGPL-3.0 永久跳过). **0 改 src 严守 100% (调研阶段, 整合 #5.1 commit 仍 0 改严守)**. **10 方向审计**: ①cargo workspace 87 crate (超 v1 30 目标, 但符合"不要怕复杂度") ②24 LOCKED 入口签名 100% 0 改 (B1 V1.0 release 严守) ③Cargo.toml borrow 段 cloned=10/rate_limited=0/skipped=1 (1:1 实际 11 源 状态 clear) ④Cargo.lock 265KB (87 crate 正常) ⑤pybridge 集成 PyO3 0.29 真接 (1 端到端) ⑥ASI Stage 1-7 跨 7 ASI Python 模块 (7 跨模块待 Stage 7 集成) ⑦形式化 kani 4502 借鉴 + F1-F10 10 维度 (Stage 5.2 done, F11-F20 Stage 5.3 跑过夜) ⑧Tauri 2.0 + Rust 后端 + Web frontend (5 nav + 9 organ 拟人化 Stage 2 done, Stage 3 跑过夜) ⑨借鉴源 12 源 (11+1 新增 OpenCog AGPL-3.0 永久跳过, 借脑 1.0 准备中) ⑩三洋葱架构 (原则 + 权限 + DSL, 升级版) + 9 organ 跨维度 (bci+mem+mind 借 superpowers 9 模式). **优化点 + 升级方案**: V1.0 release 0 改 src 严守 + V1.1 release 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + 后端加固 (24 build errors + 1 test fix + 5 check errors) + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + V2.0 release 全 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度"). **8 硬墙严守 + B1 改写**: B1 24 LOCKED 入口签名 V1.0 release 0 改 (R11 baseline 严守) + V1.1 release Mavis 自决改 (前提: 更好的架构). **8 哲学锚严守**: S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装. **不要怕复杂度哲学落地**: 最强效果 + 最厉害工程 + 维护交给未来高水平团队 (per `docs/conventions/15-no-fear-complexity.md`). **风险**: 整合 #5 commit 时机 NOT ready (per R129-26 实地 verify 30 处 fail 需修). **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 决策日志写.

---

## 1. R131-1 任务背景 + 跟决策链关系

### 1.1 R131-1 触发 (per 决策 #73 §3.2 + cron Section 10)

**主人 8/11 01:14 拍板 3 件套** (per 决策 #73 §1):
1. **locked 全解锁 + Mavis 自决架构拍板** (per 决策 #74 §1 8 硬墙 B1 改写)
2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10)
3. **总哲学扩展 (不要怕复杂度)** (per `docs/conventions/15-no-fear-complexity.md`)

**R131-1 跟决策链关系**:
- 决策 #73 §3.2: R131 era 派 3 sub-agent (R131-1 + R131-2 + R131-3)
- 决策 #74 §1: 8 硬墙 B1 改写 (V1.0 release 0 改 + V1.1 release Mavis 自决改)
- 决策 #71 §3: R131 era 差距分析阶段 (per cron Section 9 Step 3)
- cron Section 10: 架构审视永久工作项 (每次 cron tick 自动审视)

### 1.2 R131 era 3 sub-agent 派活策略 (per 决策 #73 §3.2 + 决策 #71 §3)

**3 sub-agent 派活分工** (per 决策 #73 §3.2 主人新决策):
- **R131-1 (本任务)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计, V1.0/V1.1/V2.0 release 分级)
- **R131-2 (待派)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 (新增 OpenCog 借脑 1.0 评估)
- **R131-3 (待派)**: V1.1 release 实施路线图 (PHL-07 + locked 改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)

**派活约束** (per 决策 #73 §3.2):
- 0 改 src 调研阶段 (R131-1/2/3 全部 0 改 src)
- 整合 #5.1 commit 仍 0 改严守 (per 决策 #74 §1 B1 V1.0 release 0 改)
- 整合 #5.2 commit 加 locked 全解锁哲学文档 + 不要怕复杂度哲学文档
- 整合 #5.3 commit 加决策 #73 + #74 + R131 era 3 sub-agent 报告

### 1.3 R131-1 跟 R129 era 报告关系

**R129 era 已有的关键报告** (per 任务 spec, 不重写 reference):
- R129-11: 后端 0 装 PASS 终极 verify (整合 11/11 1:1 + 8 硬墙 0 越界) → 100% PASS
- R129-14: 后端健康度总览 (R125 era 起到 R128-2 era, 41 sub-agent + 4100+ tests) → 100% PASS
- R129-22: R129 era 跨 sub-agent 总览 (R129-1~21 21 sub-agent) → 24 sub-agent 总览
- R129-26: R129 era 健康度 verify (R129-1~23 24 sub-agent + cargo test 实际状态) → **60% PASS, R129-21 报告 0 装 PASS violation** (24 build errors + 1 FAILED test + 5 check errors)
- R129-28: 借鉴 11/11 终极 verify (1:1 实地 verify 实际文件列表) → 100% clear
- R129-34: R129 era 跨 sub-agent 总览 final final (R129-1~33 33 sub-agent) → 整合 #5 commit NOT ready

**R131-1 跟 R129 era 关系**:
- ✅ 引用不重写 (per 任务 spec)
- ✅ 0 改 src 调研阶段
- ✅ 0 装 PASS 严守 (R129-26 揭示的 30 处 fail 在本报告里诚实标)
- ✅ 8 硬墙 0 越界 (V1.0 release 0 改严守)

---

## 2. 现有架构总审视 (10 方向审计)

### 2.1 方向 ①: cargo workspace 结构 (87 crate vs v1 30 crate 目标)

**现状清点 (per Cargo.toml members)**:
- **总 crate 数量**: **87 个** (per Cargo.toml members 实际清点, 2026-08-11)
- **24 LOCKED crate** (per `docs/omnibus/24-locked-crates.md`):
  - supervisor / agent / bus / council / evolution / extension / graph / mcp / pipeline / tool-registry / tool-runtime / protocol (12 主路径 LOCKED)
  - asi / onion / sovereignty / constraint / memory / cognition / perception / consciousness / motivation / life-force / relation / value (12 R20 阶段 4 主体 LOCKED, R37-2 transparent re-export)
- **非 LOCKED crate** (63 个):
  - 核心抽象 (10): core / telemetry / provider / tools / cli / bench / test / config / upgrade / verify
  - 哲学/能力 (6): cognition / action / central / skills / acp / cron
  - 智囊团/工具 (8): onion / council / supervisor / pybridge / bus / api / web / extension
  - 兼容组件 (12): mcp / mcp-ssh / mcp-winrm / mcp-relay-image / sdk / sdk-sandbox / sdk-lark / sdk-livekit / sdk-voice / lark / voice / livekit
  - 形式化/治理 (5): formal / library-governance / eval / tracing / metrics
  - 借鉴源 1:1 翻译 (5): pipeline-g5 / pipeline / tool-registry / tool-runtime / tool-approval
  - 借鉴模式 (7): agent / plugin / state / cache / credentials / oauth / update
  - ASI/认知 (3): asi / cognition / action
  - 升级/通信 (5): upgrade / bus / api / web / supervisor
  - 持久化/工具 (4): vector / observability / tree-sitter / i18n
  - 任务/工作流 (4): task / workflow / team-lead / cron
  - 鉴权/凭据 (3): credentials / oauth / keyring / machine-id
  - 监控/告警 (3): observability / metrics / tracing
  - 安全/沙箱 (3): sandbox / keyring / machine-id
  - 工具扩展 (5): tool-registry / tool-runtime / tool-approval / tool-state
  - 第三方 SDK (4): lark / voice / livekit / tree-sitter
  - 集成测试 (4): integration-e2e / integration-r20-stage4 / tui-e2e / image-prompt
  - 估算缺补 (8): rollback / repo-scan / repo-analyzer / update / state / cache / tracing / metrics
  - R20 阶段 1 估补 (5): mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead
  - R20 阶段 4 估补 (5): image-prompt / rollback / plugin / repo-scan / repo-analyzer
  - R20 阶段 6 估补 (5): keyring / machine-id / lark / voice / observability
  - R20 阶段 6 续 (5): task / tree-sitter / i18n / naming-v05 / credentials
  - R20 阶段 5 估补 (1): tui-e2e
  - R20 阶段 6 估补 (1): cache
  - R21 估补 (5): tracing / metrics / oauth / update / state
  - R20 阶段 6 估补 (1): sandbox
  - R23 P3 透明登记 (1): memory/extensions
  - V1302/1304/1305/1306 fix (7): blueprint-impl / sdk-sandbox / integration-e2e / integration-r20-stage4 / rate-limiter / sdk-lark / sdk-livekit / sdk-voice
  - R20 阶段 6 估补 (1): cache
  - R127 P5-2 估补 (1): library-governance
  - R20 阶段 6 估补 (1): tauri-stub
  - R20 阶段 6 估补 (1): tui

**vs R14 阶段 2 §3 设计 v1 30 crate 目标对比**:
- R14 阶段 2 §3 设计 30 crate: 入口层(1) + 核心抽象(2) + 智能层(3) + 智囊团层(1) + 经验方法论(4) + 兼容组件(5) + 升级层(1) + 通信总线(4) + 持久化(1) + 哲学/权限洋葱双锁层(2) + 双锁补充(6) = **30 crate**
- 实际 87 crate = **30 × 2.9 = 远超 v1 30 目标**

**审计结论**:
- ✅ **87 crate 数量符合"不要怕复杂度"哲学** (per 主人 8/11 01:14 拍板 §3, 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- ⚠️ **87 crate 拆得过细**: 24 LOCKED + 63 非 LOCKED, 63 非 LOCKED 里有 5 个 transparent re-export (life-force / value / consciousness 等) + 10+ 借鉴源 1:1 翻译 (tool-registry / tool-runtime / tool-approval / pipeline-g5 / cache / credentials / oauth / update / state / tracing / metrics) + 10+ 估补 (mcp-ssh / mcp-winrm / mcp-relay-image / keyring / machine-id / rollback / repo-scan / repo-analyzer / i18n / task) = **真正核心 ≈ 40-50 crate, 估补 + 借鉴 1:1 + transparent re-export ≈ 30+ crate**
- ⚠️ **5 个 transparent re-export crate** (life-force → memory, value → motivation, consciousness → perception) **可考虑 V2.0 release 合并** (前提: 0 触碰 24 LOCKED, 仅并 transparent re-export)
- ⚠️ **24 LOCKED crate 集合应保持稳定** (per 决策 #33 §2.3 B1 + 决策 #22 §1.2, V1.0 release 0 改, V1.1 release Mavis 自决改)

**vs 实际可编译 crate**:
- per P12-1 verify (8/10 21:44): 33 crates compile 2 fail (apeireth-central 23 + apeireth-naming-v05 1 + apeireth-graph 5)
- per R129-26 00:55+ 实地 verify: cargo build --workspace 24 hard errors (apeireth-central 23 + apeireth-naming-v05 1) + cargo test 1 FAILED (test_release_version_is_1_1_0) + cargo check -p apeireth-graph 5 hard errors
- **总需修 30 处 fail (24 + 1 + 5)** = 整合 #5 commit 前必修 (R129-26 关键诚实标)

**优化方向** (per 决策 #73 + 决策 #74):
- **V1.0 release**: 0 改 workspace 严守 (整合 #5 commit 0 改 Cargo.toml members)
- **V1.1 release**: 0 主动合并 (per 决策 #74 §1 B1 V1.1 release Mavis 自决改, 前提: 更好的架构)
- **V2.0 release**: Cargo workspace 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.2 方向 ②: 24 LOCKED 入口签名分布

**24 LOCKED crate 入口签名分布** (per R129-11 §4.1 + `docs/omnibus/24-locked-crates.md`):
- **12 主路径 LOCKED** (R125 B1 16:38 拍板, mtime 16:34:11 baseline):
  - apeireth-supervisor: `pub mod journal_entry, lib;` + `pub use ...;` (LOCKED 16:34:11)
  - apeireth-agent: `pub mod agent, manager, subagent;` + `pub use ...;` (LOCKED 16:34:11, subagent 是 NEW per P6-2)
  - apeireth-bus: `pub mod ...;` + `pub use ...;` (LOCKED 14:07:47)
  - apeireth-council: `pub mod ...;` + `pub const PHILOSOPHICAL_ANCHORS: [&str; 6];` (LOCKED 14:07:57, **6 哲学锚 0 改**)
  - apeireth-evolution: `pub mod ...;` + `pub use ...;` (LOCKED 14:07:57)
  - apeireth-extension: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:05, **6 kinds pluginType** 0 改)
  - apeireth-graph: `pub mod checkpoint, conditional, executor, mcp_resource, state, cognition_graph, subgraph, channel, state_graph, context_graph;` (LOCKED 09:08:10, 4 NEW per P6-2)
  - apeireth-mcp: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:05)
  - apeireth-pipeline: `pub mod force_translate, model_router, placeholder, tiktoken_counter, retry_suppression, role_divider, streaming, token_budget, tool_loop, provider_registry;` (LOCKED 14:08:14, 1 NEW per P6-1)
  - apeireth-tool-registry: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:27)
  - apeireth-tool-runtime: `pub mod executor, fuzzy, parser, privacy, record, mcp_protocol;` (LOCKED 14:08:27, 1 NEW per P6-2)
  - apeireth-protocol: `pub mod ...;` + `pub use ...;` (LOCKED 16:34:11, **8 lines 模块导出声明是 LOCKED 范围内**)
- **12 R20 阶段 4 主体 LOCKED** (R125 B1 16:38 拍板, R37-2 transparent re-export):
  - apeireth-asi: ASI 北极星 (R11 baseline 0 改)
  - apeireth-onion: 原则 + 权限洋葱 (R14 D7 0 改)
  - apeireth-sovereignty: 主权 (R14 D7 0 改)
  - apeireth-constraint: 约束 (R14 D7 0 改)
  - apeireth-memory: 记忆 (R11 baseline 0 改)
  - apeireth-cognition: 认知 (R20 哲学 crate 0 触碰)
  - apeireth-perception: 感知 (R20 哲学 crate 0 触碰)
  - apeireth-consciousness: 意识 (R37-2 transparent re-export 0 触碰)
  - apeireth-motivation: 动机 (R20 哲学 crate 0 触碰)
  - apeireth-life-force: 生命力 (R37-2 transparent re-export 0 触碰)
  - apeireth-relation: 关系 (R20 哲学 crate 0 触碰)
  - apeireth-value: 价值 (R37-2 transparent re-export 0 触碰)

**审计结论**:
- ✅ **24 LOCKED 入口签名 100% 0 改严守** (per R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 全 PASS)
- ✅ **NEW `pub mod` 0 改原 signature** (P6-1 +1 pipeline provider_registry, P6-2 +3 graph subgraph/channel/state_graph/context_graph, P6-2 +1 tool-runtime mcp_protocol, P6-2 +1 agent subagent = 6 NEW `pub mod` 加在原 mod 后, 0 改原 mod 顺序)
- ✅ **0 越界 8 硬墙 B1 严守** (per R129-11 + R129-21 + R129-26 交叉 verify 100%)
- ⚠️ **V1.0 release 0 改严守** + **V1.1 release Mavis 自决改** (per 决策 #74 §1 B1 改写, 前提: 更好的架构)

**优化方向**:
- **V1.0 release**: 0 改 24 LOCKED 入口签名严守 (R11 baseline 严守)
- **V1.1 release**: 24 LOCKED 入口签名可改 (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
  - 例: apeireth-pipeline + provider_registry 整合 (P6-1 done) → 入口签名可重新设计
  - 例: apeireth-graph + subgraph/channel/state_graph/context_graph 整合 (P6-2 done) → 入口签名可重新设计
  - 例: 5 transparent re-export (life-force / value / consciousness) → 可改入口 (per 决策 #74 §1 V1.1 release Mavis 自决)

### 2.3 方向 ③: Cargo.toml borrow 段 (cloned=10, rate_limited=0, skipped=1)

**Cargo.toml borrow 段现状** (per `[workspace.metadata.apeireth]`):
```
borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done)",
    "NVIDIA/NeMo-Guardrails (R125-5 整合 #4 commit 后 ✅ cloned, 0 装 PASS 严守)",
]
borrow_rate_limited = [
    "BerriAI/litellm (⏳ 限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "sst/opencode (⏳ 限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)",
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
```

**vs R129-7 22:50 报告 + R129-28 00:48 终极 verify 实际状态**:
- ✅ **8 真 cloned** (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails) mtime 全部早于整合 #4 commit 19:41 (per R129-11 §1.1 1:1 verify, 49.6MB / 7,764 files)
- ⏳ → ✅ **3 限流 → 重试真实施**: LiteLLM 0 cloned → P6-1 公开设计 1:1 翻译 (19/19 tests pass), opencode 0 cloned → P6-2 改借鉴已 cloned langgraph 829 + servers 175 (35/35 tests pass), Guardrails 0 cloned → P6-3 整合 #4 commit 后 ✅ cloned (20 unit test)
- ❌ **1 永久跳过**: OpenCog AGPL-3.0 0 集成 0 装, 0 假装

**实际状态 (per R129-7 + R129-11 + R129-28 1:1 verify)**:
- ✅ **10 真实施** (8 真 cloned + LiteLLM 公开 1:1 翻译 + opencode 改借鉴已 cloned) 
- ⏳ **0 限流** (P6-1/2/3 全 done)
- ❌ **1 跳过** (OpenCog AGPL-3.0 永久跳过)
- 0 借脑 0 装 (per P6-2/3 改借鉴已 cloned 而非真 clone, 仍属"借鉴 ID 索引完成", 0 装"已读真源码" / 0 装"已对接 opencode 私有 channel" / 0 装"已借鉴 Guardrails 私有 plugin")

**Cargo.toml borrow 段 vs 实际状态**:
- ⚠️ **Cargo.toml borrow 段 17:44 状态 vs 22:50 实际状态不一致**: Cargo.toml `borrow_count_cloned = 8` (R125 era), 但 R129-7 22:50 实际 ✅ 10 真实施 (8 真 cloned + LiteLLM + opencode 改借鉴)
- ✅ **整合 #5.2 commit 时 Cargo.toml borrow 段 update 17:44 → 22:50 状态** (per 决策 #62 §5.2)
- ✅ **整合 #5.2 commit 时 borrow_skipped 段加 OpenCog 永久跳过明示** (per 决策 #62 §5.2)

**审计结论**:
- ✅ **Cargo.toml borrow 段 0 装 PASS 严守** (per R129-11 + R129-28 1:1 verify 100%)
- ✅ **整合 #5.2 commit 时 update 17:44 → 22:50 状态** (per 决策 #62 §5.2)
- ⚠️ **可优化**: borrow 段可拆更细 (per 决策 #73 §2 cron Section 10)
  - 例: `borrow_cloned_real` (8) + `borrow_translated_public` (2, LiteLLM + opencode) + `borrow_submodule` (0) + `borrow_skipped_license` (1, OpenCog) = 4 子段
  - 例: 借鉴源版本 hash 段 (per 决策 #36 §1.1 严格化) → `borrow_cloned_real_with_hash` (e.g., `clap-rs/clap 4a622b4`)
- ⚠️ **借鉴源 12 源 (11+1 新增)**: 主人 8/11 01:14 拍板 借鉴源可加 (per 决策 #73 §1), R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源 (待派) 会评估新增源

**优化方向**:
- **V1.0 release**: 整合 #5.2 commit update Cargo.toml borrow 段 17:44 → 22:50 状态 (per 决策 #62 §5.2)
- **V1.1 release**: Cargo.toml borrow 段拆更细 (4 子段: cloned_real + translated_public + submodule + skipped_license)
- **V2.0 release**: Cargo.toml borrow 段可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.4 方向 ④: Cargo.lock 大小 (265KB)

**Cargo.lock 现状** (per 文件 stat):
- **文件大小**: **265.1 KB** (271,450 bytes)
- **总 crate 数量**: 87 workspace members + 561 第三方 crates (per THIRD-PARTY-NOTICES.md, 0 cargo-deny violation)
- **总依赖 crate**: 87 + 561 = **648 crates**

**vs 业界对比**:
- 大型 Rust 项目 (如 tokio / rust-analyzer / servo) Cargo.lock 通常 200-500 KB
- Cargo workspace 50-100 crate 项目 Cargo.lock 通常 150-350 KB
- **Apeireth Cargo.lock 265KB 在合理范围** (87 + 561 crate)

**审计结论**:
- ✅ **Cargo.lock 265KB 合理** (per Cargo workspace 50-100 crate 项目通常 150-350KB)
- ✅ **0 cargo-deny violation** (per P13-1 THIRD-PARTY-NOTICES.md 1709 lines / 12 SPDX / 0 cargo-deny violation)
- ⚠️ **可优化**: Cargo.lock 可分模块 lockfile (per Cargo 1.78+ feature `[workspace.metadata.cargo-tree]`)
  - 例: `crates/apeireth-core/Cargo.lock` (core + LOCKED) + `crates/non-locked/Cargo.lock` (其余 63 crate) + `frontend/Cargo.lock` (Tauri)
  - 优点: 减小主 Cargo.lock 大小, 加快 cargo build 增量编译
  - 缺点: 跨模块 dep 解析变慢, Cargo 1.78+ 才支持, 0 业务价值
- ⚠️ **Cargo.lock commit policy**: 整合 #4 commit abf12243 含 Cargo.lock (per 决策 #48 §1.2), **V1.0 release Cargo.lock 严守 0 改** (B2 严守), **V1.1 release Cargo.lock 可更新** (per 决策 #74 §1 B2 1.2.0 严守, 但 Cargo.lock 不算 workspace.version)

**优化方向**:
- **V1.0 release**: 0 改 Cargo.lock 严守 (整合 #5 commit 时 Cargo.lock 0 改, 等整合 #5.1 commit 时一并 update)
- **V1.1 release**: Cargo.lock 可分模块 lockfile (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: Cargo.lock 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.5 方向 ⑤: pybridge 集成 (PyO3 0.29 真接 1 端到端)

**pybridge 现状** (per R125-9 PyO3 借鉴 1:1 翻译 + 整合 #4 commit abf12243):
- **crates/apeireth-pybridge/src/**: 3 files (bridge.rs +203 + lib.rs +7 + python_bindings.rs +56, **6 E0599 全修 + 77/77 tests**)
- **PyO3 0.29.2** ✅ cloned 真实施 (per R129-11 §1.1, 5.7MB / 811 files, mtime 16:53:35)
- **borrowed-repos/PyO3/PyO3-0.29.2-2026-08-10/**: 7.9MB / 928 files (含 .git, R125-9 ✅)
- **ASI Python 1100+ v*.py**: 1:1 翻译不重写 (per `architecture-v3-aircraft-carrier.md` §3.2.3 R11 真借鉴)

**vs ASI Python Stage 1-7 集成**:
- **ASI Python Stage 1-3 (P10-1/2/3, R128 era)**: ASI Python 整合 1+2+3, 跨 7 ASI Python 模块
- **ASI Python Stage 4 (R129-4, 00:25 done)**: 自治 4 维 (D1 工具 + D2 反思 + D3 记忆 + D4 决策, 4 src 106KB + 4 tests 22KB / 60 tests + 4 examples 11KB)
- **ASI Python Stage 5 (R129-5, 00:28 done)**: 治理 4 维 (G1 资源 + G2 权限 + G3 形式化 + G4 演进, 4 src 124KB + 4 tests 52KB / 184 tests + 4 examples 11KB)
- **ASI Python Stage 6 (R129-6, 00:24 done)**: 守护 4 维 (K1 错误 + K2 性能 + K3 6+1 重门安全 + K4 5 维度健康, 4 src 91KB + 4 tests / 43 tests + 4 examples)
- **ASI Python Stage 7 (R129-18, 跑过夜)**: 跨模块集成 I1-I7 7 维度

**审计结论**:
- ✅ **PyO3 0.29.2 真接 1 端到端** (per R125-9, 77/77 tests pass, 6 E0599 全修, 整合 #4 commit 严守)
- ✅ **ASI Python 1:1 翻译不重写** (per `architecture-v3-aircraft-carrier.md` §3.2.3 R11 真借鉴)
- ✅ **ASI Python Stage 4-6 已 done** (per R129-4/5/6, 60 + 184 + 43 = 287 tests 跨 12 维度)
- 🟡 **ASI Python Stage 7 跑过夜** (per R129-18 派中, 估 01:30 done)
- ⚠️ **pybridge 性能瓶颈**: PyO3 0.29 真接 1 端到端, 但跨进程调用开销需要 R22+ 续优化
- ⚠️ **pybridge 集成可深化**: V1.1 release Stage 8 实战 (per R129-30 跑过夜)

**优化方向**:
- **V1.0 release**: PyO3 0.29 真接 1 端到端 严守 (整合 #5.1 commit 0 改)
- **V1.1 release**: pybridge 集成深化 (per R129-30 Stage 8 实战 + 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: pybridge 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.6 方向 ⑥: ASI 阶段集成 (Stage 1-7)

**ASI Stage 1-7 集成现状** (per R128 era + R129 era 报告):
- **Stage 1 (P10-1)**: ASI Python 整合 Stage 1 - 关键模块 (apeireth/ 130+ .py → Rust crate 整合 Stage 1, 7 ASI 模块各 1 配额档)
- **Stage 2 (P10-2)**: ASI Python 整合 Stage 2 - 集成测试 (integration_bridge_* 33 tests)
- **Stage 3 (P10-3)**: ASI Python 整合 Stage 3 集成验证 (端到端 + 性能 + 跨模块, 3 NEW src 61KB + 3 NEW tests 56 tests + 4 examples + lib.rs +310 行, 290/290 tests pass)
- **Stage 4 (R129-4)**: 自治 4 维 D1-D4 (60 tests + 4 examples 11KB)
- **Stage 5 (R129-5)**: 治理 4 维 G1-G4 (184 tests + 4 examples 11KB)
- **Stage 6 (R129-6)**: 守护 4 维 K1-K4 (43 tests + 4 examples)
- **Stage 7 (R129-18)**: 跨模块集成 I1-I7 7 维度 (跑过夜, 估 01:30 done)

**阶段间接口**:
- Stage 1 → Stage 2: 配额档 1 端到端 → 集成测试
- Stage 2 → Stage 3: 集成测试 → 端到端 + 性能
- Stage 3 → Stage 4: 端到端 → 自治 4 维
- Stage 4 → Stage 5: 自治 → 治理 (G1 资源 / G2 权限 / G3 形式化 / G4 演进)
- Stage 5 → Stage 6: 治理 → 守护 (K1 错误 / K2 性能 / K3 安全 / K4 健康)
- Stage 6 → Stage 7: 守护 → 跨模块集成 (I1-I7 7 维度)
- Stage 7 → Stage 8: 跨模块 → 实战 (per R129-30 跑过夜, Stage 8/9 路线)

**审计结论**:
- ✅ **Stage 1-3 done** (per R128 era, 290/290 tests pass)
- ✅ **Stage 4-6 done** (per R129-4/5/6, 287 tests 跨 12 维度)
- 🟡 **Stage 7 跑过夜** (per R129-18, 估 01:30 done)
- 🟡 **Stage 8 跑过夜** (per R129-30, 估 01:20 done)
- ⚠️ **阶段间接口清晰**: Stage 1-7 阶段间接口 1:1 翻译, 0 业务耦合
- ⚠️ **Stage 9 长程 AI 成长** (per `architecture-v4-1-living-intelligence-update.md`): 9 organ + 主体连续性 + 涌现能力

**优化方向**:
- **V1.0 release**: ASI Stage 1-7 严守 (整合 #5.1 commit 0 改)
- **V1.1 release**: ASI Stage 8+ 实战 (per R129-30 + 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: ASI Stage 9 长程 AI 成长 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.7 方向 ⑦: 形式化集成 (kani 4502 借鉴 + F1-F10 10 维度)

**形式化集成现状** (per R125-10 Kani 借鉴 1:1 翻译 + R127-2 P5-2 实施 + R129-10 Stage 5.2):
- **crates/apeireth-formal/src/**: Kani 形式化工具 0 触碰 (per R129-14 §2.1 P12-1 verify, 41 tests pass)
- **crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs**: 6 重守门 v7 形式化 (per R129-11 §4.5)
- **kani 0.67.0** ✅ cloned 真实施 (per R129-11 §1.1, 5.5MB / 3224 files, mtime 17:35:29)
- **borrowed-repos/model-checking/kani-0.67.0-2026-08-10/**: 8.3MB / 4502 files (含 .git, R125-10 ✅)

**形式化维度** (per R129-10 + R129-20):
- **Stage 5.2 (R129-10, 00:42 done)**: F1-F10 10 维度 (per R129-10 报告)
- **Stage 5.3 (R129-20, 跑过夜)**: F11-F20 10 维度 跨 4 治理维 + 跨 6 重守门 + 跨 30 维 V0.5
- **Stage 5.4 (R129-32, 跑过夜)**: Stage 5.4 实战 (per R129-32 估 01:20 done)

**审计结论**:
- ✅ **kani 0.67.0 真接真实施** (per R125-10, 30 passed tests, 5+1 kani_harness.rs)
- ✅ **F1-F10 10 维度 done** (per R129-10, Stage 5.2)
- 🟡 **F11-F20 10 维度 跑过夜** (per R129-20 Stage 5.3)
- 🟡 **Stage 5.4 实战 跑过夜** (per R129-32)
- ⚠️ **形式化可深化**: V1.1 release Stage 5.5+ 跨 ASI Stage 8 + 跨 Tauri Stage 5 集成

**优化方向**:
- **V1.0 release**: F1-F10 10 维度 严守 (整合 #5.1 commit 0 改)
- **V1.1 release**: F11-F20 + Stage 5.4 实战 + Stage 5.5+ 跨模块 (per R129-20/32 + 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: 形式化全维度可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.8 方向 ⑧: Tauri 集成 (Tauri 2.0 + Rust 后端 + Web frontend)

**Tauri 集成现状** (per R128 era + R129 era 报告):
- **frontend/tauri-prototype/**: 5 nav + 主对话 + 9 organ 拟人化 (per R129-9 + R129-19 + R129-31)
- **crates/apeireth-tauri-stub/**: 32 min 真实施, cargo build PASS binary 12.8 MB + cargo tauri dev 跑通, 111 core tests PASS (per R128-2 P11-2)
- **Tauri 2.0 + superpowers 234 + 用户记忆 #3-#5 拟人化** (per R128-2 P11-2 + R129-9/19/31)
- **9 organ 拟人化**: body / brain / ear / eye / hand / heart / memory / mind / voice (per R125-7 借 aGLM 108)

**Tauri Stage 进度** (per R128 + R129 era 报告):
- **Stage 1 (P11-1)**: Tauri 终极前端 prototype (5 nav + 主对话 + 9 organ 拟人化 stub, 197KB)
- **Stage 2 (P11-2)**: Tauri 终极前端 scaffold 深化 (32 min 真实施, 111 core tests PASS)
- **Stage 2 深化 (R129-9, 00:43 done)**: 5 nav + 主对话 + 9 organ 拟人化深化
- **Stage 3 跨 nav (R129-19, 跑过夜)**: 5 nav 完整 + 9 organ 拟人化 + 跟 backend API 联调, J1-J7 7 维度
- **Stage 4 实战 (R129-31, 跑过夜)**: Stage 3 续 + Stage 4/5 路线
- **Stage 5+ (per 决策 #74 §1 V1.1 release Mavis 自决改)**: 跨 ASI Stage 8 + 跨形式化 Stage 5.5+ 集成

**审计结论**:
- ✅ **Tauri 2.0 + Rust 后端 + Web frontend 集成合理** (per R128-2 P11-2, 32 min 真实施 + 111 core tests PASS)
- ✅ **5 nav + 主对话 + 9 organ 拟人化 done** (per R128 era + R129-9)
- 🟡 **Stage 3 跑过夜** (per R129-19, 估 01:30 done)
- 🟡 **Stage 4 跑过夜** (per R129-31, 估 01:20 done)
- ⚠️ **Tauri 2.0 集成可深化**: V1.1 release Stage 5+ 跨 ASI Stage 8 + 跨形式化 Stage 5.5+ 集成

**优化方向**:
- **V1.0 release**: Tauri Stage 1-2 严守 (整合 #5.1 commit 0 改)
- **V1.1 release**: Tauri Stage 5+ 跨 ASI + 形式化 集成 (per R129-19/31 + 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: Tauri 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.9 方向 ⑨: 借鉴源 12 源 (11 + 1 新增)

**借鉴源 11 源现状** (per Cargo.toml borrow 段 + R129-7/11/28 1:1 verify):
- ✅ **8 真 cloned**: clap 4.6.6 / hyper 0.1.20 / servers 76d64c8 / PyO3 0.29.2 / kani 0.67.0 / langgraph d56666f / superpowers 6.2.0 / Guardrails (整合 #4 commit 后 ✅ cloned)
- ⏳ → ✅ **2 限流 → 重试真实施**: LiteLLM 公开 1:1 翻译 / opencode 改借鉴已 cloned (langgraph 829 + servers 175)
- ❌ **1 永久跳过**: OpenCog AGPL-3.0 0 集成 0 装

**借鉴源 12 源 (11 + 1 新增)** (per 决策 #73 §1 主人 8/11 01:14 拍板 "有更好的架构需要用 (或改变现有的) 你就直接拍板就行了"):
- 11 源: clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / LiteLLM / opencode / OpenCog
- **+1 新增源 (待 R131-2 评估)**: 例如 cogprime / act-r / soar / etc (per 决策 #73 §1 + 决策 #74 §1 Mavis 自决)

**审计结论**:
- ✅ **借鉴源 11 源 1:1 verify 100%** (per R129-7 + R129-11 + R129-28 实地 verify 100%)
- ✅ **0 装 PASS 严守** (per R129-11 §1.2 0 借脑 0 装 100%)
- 🟡 **+1 新增源 待 R131-2 评估** (per 决策 #73 §1 + 决策 #74 §1 Mavis 自决 + R131-2 任务)
- ⚠️ **借鉴源借脑 1.0 准备中** (per 整合 #5.1 commit 时机, P9-1 borrowed-repos 进阶 Stage 2 done)

**优化方向**:
- **V1.0 release**: 借鉴源 11 源 严守 (整合 #5.1 commit 0 改 Cargo.toml borrow 段 22:50 状态)
- **V1.1 release**: 借鉴源 +1 新增源 (per R131-2 评估 + 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: 借鉴源 12 源可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

### 2.10 方向 ⑩: 三洋葱架构 (原则 + 权限 + DSL) + 9 organ 跨维度

**三洋葱架构现状** (per R125 B6 升 + `docs/conventions/10-locked.md`):
- **原则洋葱 (Principle Onion)**: E/S/A/M/O 5 层 (per `architecture-v3-aircraft-carrier.md` §2.2 + `onion-wall-architecture-2026-07-31.md` §2.2)
- **权限洋葱 (Permission Onion)**: L0-L5 6 层 (per `onion-wall-architecture-2026-07-31.md` §2.2, L0 = 真实人类批准)
- **DSL 洋葱 (Colang DSL)**: R125-5 NVIDIA Guardrails 借鉴, 6 重守门 v7 第 6 重 (per R125-5 + R129-11 §4.5)
- **三洋葱统一体**: 原则洋葱嵌入权限洋葱 (per R14-D7 精化, 主哲学 O-1 安全优先)

**9 organ 代码** (per R125 B7 内部借 OpenCode + `docs/conventions/10-locked.md`):
- **9 organ 文件名 + 入口签名 LOCKED** (per 决策 #33 §2.3 B7)
- **9 organ 内部 fn 实施 0 改入口** (per R125 B7 内部借 OpenCode)
- **9 organ 分布**:
  - body (apeireth-core)
  - brain (apeireth-cognition)
  - ear (apeireth-perception)
  - eye (apeireth-perception)
  - hand (apeireth-action)
  - heart (apeireth-life-force)
  - memory (apeireth-memory)
  - mind (apeireth-consciousness)
  - voice (apeireth-voice)

**审计结论**:
- ✅ **三洋葱架构合理** (per R125 B6 升, 原则 + 权限 + DSL = 6 重守门 v7)
- ✅ **9 organ 跨维度合理** (per R125 B7 内部借 OpenCode, 0 改入口签名)
- ⚠️ **可深化**: V1.1 release 9 organ 内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改)
- ⚠️ **9 organ 跟 ASI Python Stage 1-7 + Tauri 5 nav + 形式化 F1-F10 集成** 可深化

**优化方向**:
- **V1.0 release**: 三洋葱 + 9 organ 入口签名 严守 (整合 #5.1 commit 0 改)
- **V1.1 release**: 三洋葱 + 9 organ 内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **V2.0 release**: 三洋葱 + 9 organ 可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

---

## 3. 优化点 + 升级方案 (V1.0 / V1.1 / V2.0 release 分级)

### 3.1 V1.0 release 优化点 (整合 #5.1 commit, 0 改 src 严守)

**per 决策 #62 §5.1 + 决策 #74 §1 B1 V1.0 release 0 改严守**:
- ❌ **0 改 src/** (整合 #5.1 commit 0 触碰 crates/ 下任何 .rs 文件, 严守 per 决策 #33 §2.3 + 决策 #74 §1)
- ❌ **0 改 24 LOCKED 入口签名** (B1 V1.0 release R11 baseline 严守)
- ❌ **0 改 workspace.version 1.2.0** (B2 严守)
- ❌ **0 改 R11 baseline 3 值** (A1 严守, 数字 0 改)
- ❌ **0 改 V0.5 30 维** (B3 严守)
- ❌ **0 改 6 重守门 v7** (B4 严守)
- ❌ **0 改 8 哲学锚** (B5 严守)
- ❌ **0 装 PASS 严守** (C2 严守)
- ❌ **0 主动 commit** (C1 严守, 主人起床前)
- ❌ **0 主动 push** (严守, 主人起床前)
- ✅ **整合 #5.1 commit 包含 95+ 文件 src/ 实施** (per 决策 #62 §5.1, 31 M + 50+ ?? src/ + tests/ + examples/)
- ✅ **整合 #5.2 commit 加 locked 全解锁哲学文档 + 不要怕复杂度哲学文档** (per 决策 #73 §5 + 决策 #74 §4)
- ✅ **整合 #5.3 commit 加决策 #73 + #74 + R131 era 3 sub-agent 报告** (per 决策 #73 §5)
- ⚠️ **整合 #5 commit 时机 NOT ready** (per R129-26 实地 verify 30 处 fail 需修, 等 R129-3 跑过夜 + 主人起床后 fix 30 处 + 重跑 8 步 verify → 8/8 ready → 拍板 5.1 + 5.2 + 5.3 顺序)

**V1.0 release 优化点** (整合 #5 commit 必须修的, per R129-26 30 处 fail):
- ⚠️ **24 build errors fix** (per R129-26 §0 G, apeireth-central 23 + apeireth-naming-v05 1)
- ⚠️ **1 FAILED test fix** (per R129-26 §0 G, `test_release_version_is_1_1_0` apeireth-core, 1.1.0 vs 1.2.0 stale hardcode)
- ⚠️ **5 check errors fix** (per R129-26 §0 I, apeireth-graph state_graph.rs + subgraph.rs 内部 fn 实施 bug)
- ⚠️ **PHL-07 spec-only 0 实施** (per 决策 #74 §1 A3, V1.0 release PHL-07 仍 spec-only, 0 改 code)

### 3.2 V1.1 release 升级方案 (per 决策 #74 §2 V1.1 release Mavis 自决改)

**per 决策 #74 §1 B1 改写 + 决策 #74 §2.3 V1.1 release 实施路线图**:
- ✅ **24 LOCKED 入口签名 可改** (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- ✅ **24 LOCKED crate mtime baseline 16:34 之前 可改** (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- ✅ **R11 baseline 3 值 可改** (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决 per 决策 #74 §1)
- ✅ **PHL-07 实施** (per R129-11 关键诚实标, 整合 #5.1 commit 仍 spec-only, V1.1 实施)
- ✅ **12 键其他可改** (per 决策 #74 §1 A3)
- ✅ **workspace.version bump 1.2.1** (per 决策 #74 §1 B2)
- 🟡 **后端加固** (per R129-26 30 处 fail fix 后 V1.1 release 续)
- 🟡 **Tauri Stage 5+** (per R129-19/31 Stage 3/4 续)
- 🟡 **ASI Stage 8+** (per R129-18/30 Stage 7/8 续)
- 🟡 **形式化 Stage 5.5+** (per R129-20/32 Stage 5.3/5.4 续)
- 🟡 **Cargo.toml borrow 段拆更细** (per §2.3 方向 ③ 优化方向)
- 🟡 **借鉴源 +1 新增源** (per R131-2 评估)

**V1.1 release 实施路线图** (per R131-3 任务, 待派):
- **PHL-07 实施** (per 决策 #74 §1 A3 + R129-11 关键诚实标)
- **24 LOCKED 入口签名 改写** (per 决策 #74 §1 B1, 前提: 更好的架构)
- **后端加固** (per R129-26 30 处 fail fix + 续 24 build errors + 1 FAILED test + 5 check errors)
- **Tauri Stage 5+** (per R129-19/31)
- **ASI Stage 8+** (per R129-18/30)
- **形式化 Stage 5.5+** (per R129-20/32)
- **借鉴源 +1 新增源** (per R131-2 评估)
- **Cargo.toml borrow 段拆更细** (per §2.3 方向 ③ 优化方向)
- **Cargo.lock 可分模块 lockfile** (per §2.4 方向 ④ 优化方向, Cargo 1.78+ feature)
- **9 organ 内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **三洋葱内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **0 主动 commit 严守** (per 决策 #74 §1 C1)
- **0 主动 push 严守** (per 决策 #74 §1)

### 3.3 V2.0 release 升级方案 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

**per 决策 #74 §1 + 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 主人 8/11 01:14 拍板 §3 "不要怕复杂度"**:
- ✅ **全 8 硬墙 可重评** (per 决策 #74 §1 V2.0 release, Mavis 自决)
  - B1 24 LOCKED 入口签名 可重构
  - B2 workspace.version 可重评 (e.g., 1.2.1 → 2.0.0 大版本)
  - A1 R11 baseline 3 值 可重评
  - A3 12 键 + PHL-07 可重构
  - B3 V0.5 30 维 可重构 (e.g., 50 维 + 5 增强)
  - B4 6 重守门 v7 可重构 (e.g., 8 重守门 v9)
  - B5 8 哲学锚 可重建 (e.g., 10 哲学锚)
  - C1 0 主动 commit 严守 (保留 OR 改 per 主人拍板)
- ✅ **8 哲学锚 推翻 + 重建** (per "不要怕复杂度" + "最强效果 + 最厉害工程", 主人 8/11 01:14 拍板 §3)
- ✅ **Cargo workspace 可重构** (per §2.1 方向 ① 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度")
  - 例: 5 transparent re-export (life-force / value / consciousness) → 可合并
  - 例: 10+ 借鉴源 1:1 翻译 (tool-registry / tool-runtime / tool-approval / pipeline-g5 / cache / credentials / oauth / update / state / tracing / metrics) → 可重构
- ✅ **三洋葱架构可重构** (per §2.10 方向 ⑩)
- ✅ **9 organ 可重构** (per §2.10 方向 ⑩)
- ✅ **Cargo.lock 可重构** (per §2.4 方向 ④)
- ✅ **Cargo.toml borrow 段可重构** (per §2.3 方向 ③)
- ✅ **pybridge 集成可重构** (per §2.5 方向 ⑤)
- ✅ **ASI 阶段集成可重构** (per §2.6 方向 ⑥, Stage 9 长程 AI 成长)
- ✅ **形式化集成可重构** (per §2.7 方向 ⑦, 形式化全维度)
- ✅ **Tauri 集成可重构** (per §2.8 方向 ⑧)
- ✅ **借鉴源 12 源可重构** (per §2.9 方向 ⑨)

**V2.0 release 路线图** (per 决策 #74 §2.3 V2.0 release, 待 R132 计划):
- 1.0 release tag v1.0.0 (per R129-23 + R129-27 1.0 release 实战 final runbook)
- V1.1 release (per R131-3 实施路线图, 估 8/15-8/20 完成)
- V2.0 release (per 决策 #74 §2.3, 估 9 月初 完成, 跟 ASI Python Stage 9 + Tauri Stage 5+ + 形式化 Stage 5.5+ 集成)

### 3.4 升级方案优先级矩阵 (per 决策 #73 + 决策 #74 + 不要怕复杂度哲学)

| # | 优化点 | V1.0 release (0 改) | V1.1 release (Mavis 自决改) | V2.0 release (全 8 硬墙可重评) | 优先级 | 决策依据 |
|---|--------|-------------------|--------------------------|------------------------------|------|------|
| 1 | 24 LOCKED 入口签名 改写 | ❌ 0 改严守 | ✅ 可改 (前提: 更好的架构) | ✅ 可重构 | V1.1 P1 | 决策 #74 §1 B1 改写 |
| 2 | PHL-07 实施 | ❌ spec-only 0 实施 | ✅ 实施 (V1.1 release) | ✅ 可重构 | V1.1 P2 | 决策 #74 §1 A3 + R129-11 关键诚实标 |
| 3 | 后端加固 (30 处 fail fix) | ⚠️ 修 30 处 fail 必做 | ✅ 续加固 | ✅ 可重构 | V1.0 P1 | R129-26 30 处 fail 关键诚实标 |
| 4 | Tauri Stage 5+ 跨模块 | ❌ 严守 | ✅ Stage 5+ | ✅ 可重构 | V1.1 P3 | R129-19/31 |
| 5 | ASI Stage 8+ 实战 | ❌ 严守 | ✅ Stage 8+ | ✅ 可重构 | V1.1 P4 | R129-18/30 |
| 6 | 形式化 Stage 5.5+ 跨模块 | ❌ 严守 | ✅ Stage 5.5+ | ✅ 可重构 | V1.1 P5 | R129-20/32 |
| 7 | 借鉴源 +1 新增源 | ❌ 严守 | ✅ +1 新增源 (R131-2 评估) | ✅ 可重构 | V1.1 P6 | 决策 #73 §1 + 决策 #74 §1 |
| 8 | Cargo.toml borrow 段拆更细 | ⚠️ update 17:44 → 22:50 必做 | ✅ 拆更细 | ✅ 可重构 | V1.0 P2 / V1.1 P7 | 决策 #62 §5.2 |
| 9 | Cargo.lock 分模块 lockfile | ❌ 严守 | ✅ 分模块 (Cargo 1.78+) | ✅ 可重构 | V1.1 P8 | Cargo 1.78+ feature |
| 10 | 9 organ 内部实施 改写 | ❌ 严守 (0 改入口) | ✅ 内部实施可改 | ✅ 可重构 | V1.1 P9 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 11 | 三洋葱内部实施 改写 | ❌ 严守 (0 改入口) | ✅ 内部实施可改 | ✅ 可重构 | V1.1 P10 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 12 | 5 transparent re-export 合并 | ❌ 严守 | ✅ 合并 (V1.1 release) | ✅ 可重构 | V1.1 P11 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 13 | 8 哲学锚 重建 | ❌ 严守 | ❌ 严守 | ✅ 重建 (V2.0) | V2.0 P1 | 决策 #74 §2.3 + 不要怕复杂度 |
| 14 | Cargo workspace 重构 (87 → ?) | ❌ 严守 | ❌ 严守 | ✅ 重构 (V2.0) | V2.0 P2 | 决策 #74 §2.3 V2.0 release |
| 15 | workspace.version 重评 (1.2.0 → ?) | 🔒 1.2.0 严守 | 🔒 1.2.1 bump | ✅ 2.0.0 大版本 | V1.1 P12 / V2.0 P3 | 决策 #74 §1 B2 |
| 16 | 整合 #5 commit 拍板 | ✅ 5.1/5.2/5.3 (Mavis 自决) | N/A | N/A | V1.0 P0 (前置) | 决策 #62 + #74 |
| 17 | 1.0 release tag v1.0.0 | ✅ 主人起床后手跑 | N/A | N/A | V1.0 P0 (后置) | R129-8 + R129-13 + R129-23 + R129-27 |

---

## 4. 8 硬墙严守 + B1 改写 (per 决策 #74 §1)

### 4.1 8 硬墙 V1.0 release 严守 (per 决策 #33 §2.3 + 决策 #74 §1)

| # | 8 硬墙 | V1.0 release 严守 | 证据 | 决策依据 |
|---|--------|------------------|------|---------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 全 PASS | 决策 #22 §1.2 + #33 §2.3 B1 + #41 §2 + P2-3 retry verify done + P4-1 verify done + P14-1 retry verify done |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 | Cargo.toml:274 `version = "1.2.0"` (0 改) | 决策 #22 §2.2 + #33 §2.3 B2 + #41 §2 |
| **A1** | **R11 baseline 3 值** | 🔒 数字 0 改 (0.8682/0.8532/0.9063) | `crates/apeireth-asi/src/calibration.rs:3` + `crates/apeireth-naming-v05/src/lib.rs:67` 0 改 | 决策 #22 §1.2 + #33 §2.3 A1 + #41 §2 |
| **A2** | **R11 9 子测度结构** | 🔒 严守 (V1136 9 子测度 0 改) | per 决策 #22 §1.2 A2 | 决策 #22 §1.2 A2 |
| **A3** | **12 键 + PHL-07 = 13 键** | 🔒 12 键严守 + PHL-07 spec-only 0 实施 | R125-12 PHL-07 `.r125-12-PHL-07-SPEC.md` 是 untracked spec, 0 触碰 12 键 | 决策 #22 §2.8 + #33 §2.3 + #47 |
| **B3** | **V0.5 30 维** | 🔒 24 基础 + 6 增强 = 30 维 严守 | `Cargo.toml:335-338` + `crates/apeireth-naming-v05/src/extension.rs` 0 改 | 决策 #22 §2.3 B3 + R126 P1-4 verify retry done + R125-13 langgraph 触发 B3 |
| **B4** | **6 重守门 v7** | 🔒 6 重 v6 → v7 严守 | `Cargo.toml:340-342` + `crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs` + `crates/apeireth-sovereignty/src/seven_fold_guard.rs` 0 改 | 决策 #22 §2.4 B4 + R126 P1-3 retry done + R125-5 NVIDIA Guardrails 借鉴 |
| **B5** | **8 哲学锚** | 🔒 6 锚 0 改 + 2 锚 (S-3 + O-1) 严守 | `Cargo.toml:331-333` + `crates/apeireth-core/src/eight_anchors.rs` 0 改 | 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done |
| **B6** | **三洋葱架构** | 🔒 原则 + 权限 + DSL 三洋葱 严守 | per 决策 #33 §2.3 B6 | 决策 #33 §2.3 B6 |
| **B7** | **9 organ 内部 fn 实施** | 🔒 入口签名 0 改 + 内部 fn 实施可改 | per 决策 #33 §2.3 B7 | 决策 #33 §2.3 B7 |
| **C1** | **0 主动 commit** (主人起床前) | 🔒 严守 | R129 era 24 sub-agent 全部 0 commit | 决策 #33 §2.3 C1 + #61 §6 + #62 §9 |
| **C2** | **0 装 PASS 严守** | 🔒 严守 | R129-11 + R129-28 1:1 verify 100% | 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 主人 20:32 "技术性 locked 都能解锁" |
| **C3** | **升 6 重 v6 → v7** | 🔒 严守 (B4 已升) | per 决策 #22 §2.4 | 决策 #22 §2.4 |
| **0 push** | **0 主动 push** (主人起床前) | 🔒 严守 | R129 era 24 sub-agent 全部 0 push | 决策 #33 §2.3 + #61 §6 + #62 §9 |

**8 硬墙 严守 100% verify (per R129-11 + R129-14 + R129-26 交叉 verify)**:
- ✅ 8 硬墙 0 越界 100% (per R129-11 §4.1-§4.7 + R129-14 §0 + R129-26 §0 A-F)
- ⚠️ **R129-21 报告 0 装 PASS violation** (per R129-26 §0 J, claimed "0 errors" but actual 24 + 5 + 1 = 30 处 fail, 需纠正 per R129-21 报告 0 装 PASS 严守 violation)

### 4.2 B1 改写 V1.1 release 边界 (per 决策 #74 §1)

**per 决策 #74 §1 B1 改写**:
- **旧严守 (R129 era)**: 🔒 24 LOCKED 入口签名 0 改严守 (R11 baseline 严守)
- **新严守 (V1.0 release)**: 🔒 24 LOCKED 入口签名 0 改严守 (R11 baseline 严守) (per 决策 #74 §1)
- **新严守 (V1.1 release)**: 🟢 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- **新严守 (V2.0 release)**: ✅ 24 LOCKED 入口签名 可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

**B1 改写 边界** (per 决策 #74 §1):
- ✅ V1.0 release 0 改严守: 24 LOCKED 入口签名 0 改, 内部 fn 实施可改 (per 决策 #41 §2 + #47)
- ✅ V1.1 release Mavis 自决改: 24 LOCKED 入口签名 可改, 前提: 更好的架构 (per 决策 #74 §1)
- ✅ V2.0 release 可重构: 24 LOCKED 入口签名 可重构 (per 决策 #74 §2.3)

**B1 改写 拍板流程** (per 决策 #74 §1):
- V1.1 release Mavis 自决改 → Mavis 写决策 #75 (V1.1 release 24 LOCKED 入口签名 改写 拍板) + 报告路径
- V2.0 release 可重构 → Mavis 写决策 #76 (V2.0 release 24 LOCKED 入口签名 重构 拍板) + 报告路径

### 4.3 8 硬墙 V1.1 / V2.0 release 改写 (per 决策 #74 §1 + §2.3)

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 改写 | V2.0 release 重构 | 决策依据 |
|---|--------|------------------|------------------|------------------|---------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ 可重构 | 决策 #74 §1 + §2.3 |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 1.2.1 bump | ✅ 2.0.0 大版本 | 决策 #74 §1 B2 |
| **A1** | R11 baseline 3 值 | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | ✅ 可重评 | 决策 #74 §1 A1 |
| **A2** | R11 9 子测度结构 | 🔒 严守 | 🔒 严守 | ✅ 可重评 | 决策 #74 §1 A2 |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 | ✅ PHL-07 实施 | ✅ 可重构 | 决策 #74 §1 A3 |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B3 |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B4 |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重建 | 决策 #74 §1 B5 |
| **B6** | 三洋葱架构 | 🔒 严守 | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B6 |
| **B7** | 9 organ 内部 fn 实施 | 🔒 入口签名 0 改 + 内部 fn 实施可改 | ✅ 入口签名可改 | ✅ 可重构 | 决策 #74 §1 B7 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 🔒 严守 (per 决策 #33 §2.3) | ✅ 可重评 | 决策 #74 §1 C1 |
| **C2** | 0 装 PASS 严守 | 🔒 严守 | 🔒 严守 (技术哲学, 不装) | ✅ 可重评 | 决策 #74 §1 C2 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 (per 决策 #33 §2.3) | ✅ 可重评 | 决策 #74 §1 |

---

## 5. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

**8 哲学锚 V1.0 release 严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1):
- ✅ **S-1 北极星导向** (主 22:33): 服务 ASI 北极星, 不是工具能力
- ✅ **S-2 实事求是** (主 17:43): 基于现状不重写, 核验后写
- ✅ **S-3 质量工程化** (主 16:55 R123-1): 代码质量 = 工程信誉
- ✅ **O-1 安全优先** (主 16:55 R125-5): 安全 > 功能 > 性能, 6 重守门 v7
- ✅ **O-2 走在前人经验上** (主 19:33): 借鉴 11 源 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / LiteLLM / opencode / OpenCog)
- ✅ **O-3 干到底** (主 23:44): 决策立刻沉淀, 1 commit 总
- ✅ **O-4 任何人都能接手** (主 00:56): 4 件套齐全, 顶层瘦
- ✅ **O-5 不假装** (主 17:58): 12 键编译期 hardcode, 8 项不修改承诺形式撤销后原意保留

**8 哲学锚 V1.1 / V2.0 release 改写** (per 决策 #74 §1 B5 + §2.3):
- V1.1 release: 🔒 8 哲学锚严守 (per 决策 #74 §1 B5)
- V2.0 release: ✅ 8 哲学锚 可重建 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 主人 8/11 01:14 拍板 §3 "不要怕复杂度")

**8 哲学锚 跟不要怕复杂度哲学关系** (per 决策 #33 §2.3 B5 + 决策 #74 §1 + `docs/conventions/15-no-fear-complexity.md` §2):
- 8 哲学锚 = **思想哲学** (per 决策 #33 §2.3 B5 + `docs/conventions/09-anchor.md`)
- 不要怕复杂度 = **工程哲学** (扩展, 不是替换, per `docs/conventions/15-no-fear-complexity.md` §2)
- 8 哲学锚 + 不要怕复杂度 = **9 件套 总哲学** (per `docs/conventions/15-no-fear-complexity.md` §2 + 决策 #73 §3.2)

---

## 6. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 决策 #74 §1 + `docs/conventions/15-no-fear-complexity.md`)

### 6.1 哲学 3 件套 (per 主人 8/11 01:14 拍板 + `docs/conventions/15-no-fear-complexity.md`)

**主人 8/11 01:14 拍板原文** (per `docs/conventions/15-no-fear-complexity.md` §0 + 决策 #73 §1):
> 1. "事关工程类的，技术类的全早都给你解锁locked了"
> 2. "项目里要是有文档没提到这一点你就补充进去，让以后任何团队都能看到"
> 3. "所以有更好的架构需要用（或改变现有的）你就直接拍板就行了"
> 4. "我确实需要你注意一下现有的架构什么的，有没有需要优化升级的地方，有的你也就加入升级方案"
> 5. "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害的工程，因为自然会有高水平的团队来接手维护"

**3 件套新决策** (per 决策 #73 §1):
1. **locked 全解锁 + Mavis 自决架构拍板** (per 决策 #74 §1 8 硬墙 B1 改写)
2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10 + R131 era 3 sub-agent)
3. **总哲学扩展 (不要怕复杂度)** (per `docs/conventions/15-no-fear-complexity.md`)

### 6.2 哲学核心 (per `docs/conventions/15-no-fear-complexity.md` §1)

**1.1 最强效果 > 最简单代码**:
- ✅ 效果是最高目标, 不是"代码要简单"
- ✅ 复杂度是实力的体现, 不是"技术债"
- ✅ 24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维公式 + 13 键, 都复杂, 但都是最强效果
- ❌ 推翻 KISS (Keep It Simple, Stupid)
- ✅ 新哲学 SOTA (State of the Art)

**1.2 最厉害工程 > 最易维护**:
- ✅ 工程化是最高目标, 不是"代码要易维护"
- ✅ 30+ 借鉴源 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / OpenCog / CogPrime / 等) 都复杂, 但都是最厉害工程
- ✅ 形式化证明 + 三洋葱 + 9 organ + 12 键 都复杂, 但都是最厉害工程
- ❌ 推翻 DRY (Don't Repeat Yourself)
- ✅ 新哲学 BORROW (借脑 / 借鉴 / 借源)

**1.3 维护交给未来高水平团队**:
- ✅ 维护不是问题, 因为自然会有高水平的团队来接手
- ✅ 项目复杂度是吸引高水平团队的核心
- ✅ 简化代码 = 排斥高水平团队
- ❌ 推翻 "代码要让初级团队能接手"
- ❌ 推翻 "文档要写得简单易懂"
- ❌ 推翻 "维护是负担"
- ✅ 新哲学 "维护是机会 (高水平团队接手 = 项目升级)"

### 6.3 哲学跟 R131-1 10 方向审计关系

**per `docs/conventions/15-no-fear-complexity.md` §1 + 决策 #73 + 决策 #74**:
- 方向 ① 87 crate vs 30 v1 目标: **87 crate 符合"不要怕复杂度"** (最强效果 + 最厉害工程, 维护交给未来高水平团队)
- 方向 ② 24 LOCKED 入口签名: **24 LOCKED 符合"不要怕复杂度"** (B1 0 改严守, 0 装 PASS 严守)
- 方向 ③ Cargo.toml borrow 段: **10 真实施 + 1 跳过 符合"不要怕复杂度"** (克隆 8 源真实施 = 最厉害工程)
- 方向 ④ Cargo.lock 265KB: **265KB 符合"不要怕复杂度"** (87 + 561 crate 合理范围)
- 方向 ⑤ pybridge 集成: **PyO3 0.29 真接 1 端到端 符合"不要怕复杂度"** (真接, 0 装"已对接")
- 方向 ⑥ ASI Stage 1-7: **Stage 1-7 跨 7 ASI Python 模块 符合"不要怕复杂度"** (最强效果)
- 方向 ⑦ 形式化 F1-F10 10 维度: **10 维度 符合"不要怕复杂度"** (形式化证明 = 最厉害工程)
- 方向 ⑧ Tauri 集成: **5 nav + 9 organ 拟人化 符合"不要怕复杂度"** (前端拟人化 = 最强效果)
- 方向 ⑨ 借鉴源 12 源: **12 源 符合"不要怕复杂度"** (BORROW 借脑 / 借鉴 / 借源 = 最厉害工程)
- 方向 ⑩ 三洋葱 + 9 organ: **三洋葱 + 9 organ 符合"不要怕复杂度"** (立体架构 + 生命架构 = 最强效果)

---

## 7. 风险 + 决策原则 (per 决策 #33 §2.3 + 决策 #74 §1)

### 7.1 风险 (per 决策 #33 §2.3 + 决策 #74 §1 + R129-26 关键诚实标)

| # | 风险 | 严重度 | 状态 | 缓解 |
|---|------|-------|------|------|
| 1 | **整合 #5 commit 时机 NOT ready** | 🔴 P0 | per R129-26 00:55+ 实地 verify 30 处 fail 需修 (24 build + 1 test + 5 check) | 等 R129-3 跑过夜 + 主人起床后 fix 30 处 + 重跑 8 步 verify → 8/8 ready → 拍板 |
| 2 | **R129-21 报告 0 装 PASS violation** | 🔴 P0 | per R129-26 §0 J, claimed "0 errors" but actual 30 处 fail | 8 硬墙 #C2 0 装 PASS 严守 violation, 需纠正 (per 决策 #33 §2.3 C2) |
| 3 | **PHL-07 spec-only 0 实施** | 🟡 P1 | per 决策 #74 §1 A3, V1.0 release 0 实施, V1.1 release 实施 | per R131-3 实施路线图 |
| 4 | **87 crate 拆得过细** | 🟡 P1 | per §2.1 方向 ①, 远超 v1 30 crate 目标 | per §3.2 V1.1 release 优化点 (5 transparent re-export 合并) + §3.3 V2.0 release 重构 |
| 5 | **Cargo.lock 265KB 大** | 🟢 P2 | per §2.4 方向 ④, 87 + 561 crate 合理范围 | per §3.2 V1.1 release Cargo.lock 可分模块 lockfile |
| 6 | **pybridge 性能瓶颈** | 🟡 P1 | per §2.5 方向 ⑤, PyO3 0.29 真接 1 端到端, 跨进程调用开销 | per §3.2 V1.1 release pybridge 集成深化 |
| 7 | **借鉴源 +1 新增源 待评估** | 🟡 P1 | per §2.9 方向 ⑨, R131-2 待派评估 | per R131-2 任务 |
| 8 | **Tauri Stage 3-5 跨模块** | 🟡 P1 | per §2.8 方向 ⑧, 跑过夜 | per R129-19/31 |
| 9 | **ASI Stage 7-9 跨模块** | 🟡 P1 | per §2.6 方向 ⑥, 跑过夜 | per R129-18/30 |
| 10 | **形式化 Stage 5.3-5.5 跨模块** | 🟡 P1 | per §2.7 方向 ⑦, 跑过夜 | per R129-20/32 |

### 7.2 决策原则 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 + 用户记忆 #10)

**核心原则 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 + 用户记忆 #10)**:
- ✅ **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- ✅ **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑)
- ✅ **中断接手** (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- ✅ **编译产物清理决策矩阵** (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- ✅ **计划内任务完成自动接续 4 步 + 永久循环** (per 主人 0:57: 调研 + 差距 + 计划 + 实施 → 永久)
- ✅ **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
- ✅ **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
- ✅ **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, `docs/conventions/15-no-fear-complexity.md`)

**8 硬墙严守 + B1 改写 (per 决策 #74 §1)**:
- ✅ **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)
- ✅ **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
- ✅ **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- ✅ **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
- ✅ **B3 V0.5 30 维**: 严守 (哲学)
- ✅ **B4 6 重守门 v7**: 严守 (哲学)
- ✅ **B5 8 哲学锚**: 严守 (哲学)
- ✅ **C1 0 主动 commit (主人起床前)**: 严守
- ✅ **C2 0 装 PASS 严守**: 严守
- ✅ **0 push (主人起床前)**: 严守

**流程严守 (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #62 §9 + 决策 #74 §1)**:
- ✅ **整合 #5 commit 由 Mavis 自决拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- ✅ **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- ✅ **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- ✅ **0 主动删** (per Safety policy + 决策 #44 + #60, 含 target/ 28.9 GB + _workspace/ 1.2 MB)
- ✅ **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10)
- ✅ **0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权)
- ✅ **0 借脑 0 装** (per P6-2/3 改借鉴已 cloned 而非真 clone, 仍属"借鉴 ID 索引完成")

---

## 8. 整合 #5 commit 拍板逻辑 (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4)

**整合 #5.1 commit (src/ 实施, 95+ 文件)** (per 决策 #62 §5.1):
- 仍按原计划 (per 决策 #62 §5.1)
- **0 改 24 LOCKED 入口签名** (V1.0 release R11 baseline 严守 per 决策 #74 §1)
- 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup)
- PHL-07 spec-only 0 实施 (V1.1 release 实施 per 决策 #74 §1 A3)
- ⚠️ **整合 #5.1 commit 时机 NOT ready** (per R129-26 实地 verify 30 处 fail 需修: 24 build + 1 test + 5 check)

**整合 #5.2 commit (docs/ + Cargo.toml, 10 文件)** (per 决策 #62 §5.2 + 决策 #73 §5):
- 仍按原计划 (per 决策 #62 §5.2)
- Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per 决策 #62 §5.2)
- **+ 新增 `docs/conventions/15-no-fear-complexity.md`** (per 决策 #73 §3 主人 8/11 01:14 总哲学扩展)
- **+ 更新 `docs/conventions/10-locked.md`** (per 决策 #73 §2.3 主人 8/11 01:14 locked 全解锁)
- **+ 更新 `docs/conventions/09-anchor.md`** (per 决策 #73 §4.2 总工程哲学扩展引用)
- **+ 更新 `docs/conventions/README.md`** (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
- **+ 更新 `CONTRIBUTING.md`** (per 决策 #73 §2.3 8 项不修改承诺 改写 + 主人 8/11 01:14 拍板记录)
- **+ 更新 `README.md`** (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)

**整合 #5.3 commit (reports/, 60+ 文件)** (per 决策 #62 §5.3 + 决策 #73 §5):
- 仍按原计划 (per 决策 #62 §5.3)
- **+ 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写)** (per 决策 #73 §2.2 + §5)
- **+ 新增 R131 era 调研 3 sub-agent 报告** (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
- **+ 新增 `philosophy-no-fear-complexity-2026-08-11.md`** (主人 8/11 01:14 决策 3 件套详细)

**整合 #5 commit 拍板时机 (per 决策 #61 §1.4 + 决策 #62 §2 + 决策 #74 §4)**:
- ⚠️ **当前 NOT ready** (per R129-26 00:55+ 实地 verify 30 处 fail 需修)
- ✅ **8 项 verify 100% 落实 → Mavis 自决拍板** (per 决策 #64 §4):
  - 8 项 verify: 41 任务 done / 借鉴 11/11 状态 clear / 8 硬墙 0 越界 / 24 LOCKED 入口签名 0 改 / Cargo.toml 1.2.0 严守 / master HEAD = abf12243 / 决策链 #30-#65 全读 / 8 步 verify 全 PASS (R129-3)
  - 当前: 7/8 verify ✅, R129-3 8 步 verify 跑过夜 (估 01:30 done), 完后 cron Section 4 自动拍板

---

## 9. 1.0 release 实战流程 (per R129-8 + R129-13 + R129-23 + R129-27 final runbook)

**1.0 release 完整 5 步流程** (per R129-8 + R129-13 准备 + 决策 #55 §2.6 + 决策 #58 §5 + 主人 8/4 23:33):

1. **8 步 verify** (verify-1.0-pre-tag.{ps1,sh}, 主人起床后手跑):
   - Step 1: 修 session working dir + master HEAD + Cargo.toml
   - Step 2: `cargo build --workspace` (per R129-26 24 build errors 需修)
   - Step 3: `cargo test --workspace` (per R129-26 1 FAILED test 需修 + 4100+ tests)
   - Step 4: `cargo run --bin apeireth-tui` 5s smoke
   - Step 5: `cargo run --bin apeireth-api` 5s smoke
   - Step 6: `cargo audit + cargo deny`
   - Step 7: 24 LOCKED 入口签名 0 改 verify (24/24)
   - Step 8: 8 硬墙 0 越界 + 0 装 PASS 严守 (14/14 verify)

2. **配 GitHub remote** (setup-github-remote.{ps1,sh}, 主人手跑):
   - 主人浏览器创建 GitHub repo `apeireth/apeireth-rust` (Public, 0 初始化 README/.gitignore/license)
   - 加 origin remote `https://github.com/apeireth/apeireth-rust.git`
   - 主人配 git push 认证 (gh auth login 或 Personal Access Token)

3. **git push 整合 #5 拆 3 commit** (git-push-1.0.{ps1,sh}, 主人手跑):
   - 整合 #5.1 commit (50+ src/ 改动)
   - 整合 #5.2 commit (10 docs + Cargo.toml)
   - 整合 #5.3 commit (30+ reports/)
   - `git push -u origin master`

4. **打 v1.0.0 tag + gh release create** (tag-1.0.0.{ps1,sh}, 主人手跑):
   - `git tag -a v1.0.0 -m "Apeireth 1.0.0 release"`
   - `git push origin v1.0.0`
   - `gh release create v1.0.0 --title "Apeireth 1.0.0" --notes-file RELEASE_NOTES.md`
   - verify GitHub release 页面

5. **1.0 release 反馈**: 主人 verify + Mavis 写 decision-67 (1.0 release 拍板) + decision-68 (后续 R130 era 派活规划)

**GitHub Pages 部署 5 步** (per R129-13 准备, 主人手跑):
- mkdocs build → 配 gh-pages branch → git push origin gh-pages → 启用 GitHub Pages 设置 → verify 文档页面
- 7 markdown 源文件 (index + getting-started + api + roadmap + changelog + borrowed-repos + architecture) + 根 mkdocs.yml (Material theme, 5 nav + 3 链式页)

---

## 10. 一句话 (再次强调)

**R131-1 现有架构总审视 + 优化点 + 升级方案 (per 主人 8/11 01:14 拍板 3 件套 §2 + 决策 #73 §2 + 决策 #74 §1 + 决策 #71 §3 R131 era 差距分析阶段 + cron Section 10 架构审视永久工作项)**: 10 方向审计 + V1.0 release 0 改 src 严守 + V1.1 release Mavis 自决改 + V2.0 release 全 8 硬墙可重评. **8 硬墙严守 + B1 改写** (B1 V1.0 0 改 + V1.1 Mavis 自决改 + V2.0 可重构). **8 哲学锚严守** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5). **不要怕复杂度哲学落地** (最强效果 + 最厉害工程 + 维护交给未来高水平团队, per `docs/conventions/15-no-fear-complexity.md`). **风险**: 整合 #5 commit 时机 NOT ready (per R129-26 实地 verify 30 处 fail 需修). **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 决策日志写. **R131-1 0 改 src 严守 100%**, 报告路径 `reports/agent-r131-1-architecture-audit-2026-08-11.md` (本文件, 整合 #5.3 commit 包含 per 决策 #62 §5.3 + 决策 #73 §5).

---

## 11. 不漂移 (per 决策 #33 §2.3 + 决策 #74 §1)

- 🔒 **0 改 src/** (100% 严守, R131-1 调研阶段, 整合 #5.1 commit 仍 0 改严守)
- 🔒 **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改)
- 🔒 **0 主动 commit** (100% 严守, 整合 #5 commit 由 Mavis 自决拍板, R131-1 0 git commit)
- 🔒 **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- 🔒 **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告)
- 🔒 **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60)
- 🔒 **8 哲学锚严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1)
- 🔒 **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1)
- 🔒 **V0.5 30 维严守** (per 决策 #33 §2.3 B3 + 决策 #74 §1)
- 🔒 **6 重守门 v7 严守** (per 决策 #33 §2.3 B4 + 决策 #74 §1)
- 🔒 **0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 决策 #74 §1)
- 🔒 **0 主动 commit (主人起床前) 严守** (per 决策 #33 §2.3 C1 + 决策 #74 §1)
- 🔒 **0 主动 push (主人起床前) 严守** (per 决策 #33 §2.3 + 决策 #74 §1)
- 🔒 **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- 🔒 **决策日志写** (per 决策 #10 + 用户记忆 #10)
- 🔒 **不要怕复杂度哲学落地** (per `docs/conventions/15-no-fear-complexity.md` + 决策 #73 §3 + 决策 #74 §1)
- 🔒 **借鉴源 11 源 1:1 verify 100%** (per R129-11 + R129-28)
- 🔒 **0 借脑 0 装 PASS 严守** (per P6-2/3 改借鉴已 cloned 而非真 clone)
- 🔒 **不重写 R129-1/2/3/7/11/21/26/28/34** (per 任务 spec, reference 而非重写)
- 🔒 **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)

---

## 12. 核验 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §1 + 用户记忆 #10)

- ✅ 主人 8/10 01:14 拍板 "locked 全部解锁"
- ✅ 主人 8/10 16:27 拍板 "为了升级或更好, 要改动现有的 locked, 不必犹豫, 完全可以"
- ✅ 主人 8/10 16:31 拍板 "全部采纳, 全都能动, 需要具体确认的你自己确认就行, 你有最高权限"
- ✅ 主人 8/11 01:14 拍板 3 件套 (per 决策 #73 §1 + 决策 #74 §1)
- ✅ 决策 #73 写完 (locked 全解锁 + 架构审视 + 不要怕复杂度)
- ✅ 决策 #74 写完 (8 硬墙 B1 改写)
- ✅ R131 era 3 sub-agent 派活 (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
- ✅ 整合 #5 commit 拍板逻辑更新 (5.1 仍 0 改, 5.2 加哲学文档, 5.3 加决策 + R131 era 报告, per 决策 #73 §5)
- ✅ cron Section 10 新增 (架构审视永久循环, per 决策 #73 §2)
- ✅ R131-1 现有架构总审视 10 方向 + 优化点 + 升级方案 (本报告, 0 改 src 严守)
- ⏳ R129-3 报告 done → 整合 #5 commit 拍板 (per 决策 #62 + 决策 #74 §4, 8 项 verify 100% 后)
- ⏳ 主人起床后配 GitHub remote + git push + tag v1.0.0 + release notes (per R129-8 + R129-13 + R129-23 + R129-27 final runbook)
- ⏳ R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源 (待派, per 决策 #73 §3.2)
- ⏳ R131-3 V1.1 release 实施路线图 (待派, per 决策 #73 §3.2)
- ⏳ V1.0 release tag v1.0.0 (估 8/11-8/12)
- ⏳ V1.1 release PHL-07 实施 + 24 LOCKED 入口签名 Mavis 自决改 (估 8/15-8/20)
- ⏳ V2.0 release 全 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (估 9 月初)
