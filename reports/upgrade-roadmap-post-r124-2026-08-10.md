# R125+ 升级路线图 — 基于 R124 + R123 + R122-10 + R120 全调研综合

**Date**: 2026-08-10 16:30
**Author**: Mavis (root session, 主人 16:23 "制定后续的升级计划, 有需要借鉴源码的就把项目下载下来" 明确授权)
**关联决策**: `reports/decision-21-upgrade-roadmap-2026-08-10.md`
**关联报告**: R120 (4 团队) + R121-retry (5 任务) + R122 (12 任务) + R123 (4 任务) + R124-1/2/3 (3 调研) + R122-10 (refactor scan) = 28 子任务 / 19945+50+ baseline / 0 越界 8 硬墙

---

## 0. 概述 (TL;DR)

整合 #3 节点 (17:30 截止) 后, **Apeireth-rust 1.0 release** 前还剩 **3 阶段升级**:

| 阶段 | 时间 | 任务 | 产出 | ROI |
|---|---|---|---|---|
| **R125** (本月内) | 16:30-23:59 | 14 借鉴实施 + 5 抽象 + 1 拆 crate | -250KB 代码 + 5 抽象 + 1 删除 | 极高 |
| **R126** (Q4 2026) | 9-10 月 | 4 拆 crate + 真接 4 协议 handler + StateGraph 抽象 | 主仓拆分 + 状态机成熟 | 高 |
| **R127** (1.0 release 前) | 11-12 月 | ASI 24 维 + 评估 + Skill 化 + 集成测试 | 1.0 release 候选 | 极高 |

**核心思想**: "**借鉴 + 抽象 + 拆分**" 三步走 — 借鉴站巨人肩上, 抽象为长期可维护, 拆分为 1.0 release 铺路.

---

## 1. 调研综合 (5 报告, 28 子任务, 19945+50+ tests)

### 1.1 报告一览

| 报告 | 作者 | 大小 | 产出 | 8 硬墙 |
|---|---|---:|---|---|
| **R120** (4 团队) | A/B/C/D × 2 = 8 agent | 4 commits | Memory/Vector 真接 (128/128) + API 健壮性 (360/360) + 工具 9 类分类器 (90/90) + CI 矩阵化 (18 yml) + .github 完善 (1 改 + 3 新建 + 1 重写) | ✅ |
| **R121-retry** (5 任务) | 1 团队 5 sub-agent | 1 commit | SSE 流式 7 test + Redis stub 8 test + jitter/eviction 12 test + dependabot yml no-op + 1 failed 修 | ✅ |
| **R122** (12 任务) | 7 团队 12 sub-agent | 1 commit (df6dfb69) + 1 override (95ac8e4f) | Response Replay Cache + 角色划分 + tiktoken 精确计数 + 4 TODO 续 + 语义模型路由 + 运维快赢 + 日志回放 + 多语言 SDK + Kani 5 + 重复扫描 | ✅ |
| **R123** (4 任务) | 4 团队 4 sub-agent | 1 commit 拍板中 | 4 协议 handler trait 抽象 (R123-2) + 浏览器 MCP (R123-3) + 多模态 MCP (R123-4) + clippy/doc 详细清 (R123-1 进行中) | ✅ |
| **R124** (3 调研) | 3 团队 3 sub-agent | 3 报告 (138KB) | 战区 1-2 (TUI/LLM Gateway 8 模块) + 战区 3 (Multi-Agent 13 模块) + 战区 4-5+L0 (16 模块) | ✅ 0 触碰 src |
| **R122-10** (refactor scan) | Mavis 自干 (task 工具挂) | 1 报告 (7.7KB) | 15 大文件 + 5 重复模式 + 5 dead code + 91 TODO/FIXME | ✅ 0 触碰 src |
| **总计** | 28 子任务 | 7 commits | **19945 + 50+ tests, 0 failed** | **8 硬墙全守** |

### 1.2 借鉴统计 (R124 主调研)

| 报告 | 候选项目 | 借鉴机会 | 唯一 ID | Top ROI |
|---|---:|---:|---:|---|
| **R124-1** (战区 1-2: 8 模块) | 28 | 30 | 22 | LiteLLM provider registry **-209.6KB (-65%)** |
| **R124-2** (战区 3: 13 模块) | 41 | 39 | 55 + 5 跨模块观察 | superpowers / chidori / aGLM / SWE-bench / OpenCog |
| **R124-3** (战区 4-5+L0: 16 模块) | 64 | 68 | 77 | MCP servers / PyO3 / NVIDIA Guardrails / Kani / sqlite-vec |
| **总计** | **133** | **137** | **154+** | **Top 15 借鉴 ROI 总计 -250KB+ 代码 + 认知架构 30 年沉淀** |

### 1.3 当前状态 (16:30 整合 #3 节点)

- **代码量**: 92 crate, 1.05MB Rust 源码 (Top 15 大文件), workspace.version 1.1.0
- **测试**: 19945 + 50+ = 19995+ tests, 0 failed
- **commit 链**: 95ac8e4f ← df6dfb69 ← 075d7d3d ← 0b576568 ← ... (R120 至今 7 commit)
- **8 硬墙全守**: workspace.version / R11 baseline 3 值 / 24 LOCKED / 6 哲学锚 / 9 organ / 11 公共 API / 0 装 / 0 主动 commit
- **R123-1 还在跑**: clippy 9 批 / doc 1 批 (清 apeireth-eval/telemetry 中)
- **借鉴源码下载**: Top 10 git clone background 跑中 (.openclaw\workspace\borrowed-repos\)

---

## 2. Top 15 借鉴优先级 (R124 综合, 按 ROI 排序)

### Tier 1: ROI 极高, 2 周内可完成 (R125 主线)

| # | 借鉴 ID | 项目 | 目标 crate | 预计 ROI | 周期 |
|---|---|---|---|---|---|
| 1 | `R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10` | **LiteLLM provider registry** | apeireth-pipeline + api + protocol | -209.6KB (-65%) | 3-5 天 |
| 2 | `R124-2-BORROW-obra/superpowers-2026-05-2026-08-10` | **obra/superpowers Skill 化** | apeireth-central | 2025-2026 AI 工程化标准 | 1-2 天 |
| 3 | `R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10` | **Chidori host-call journal + replay** | apeireth-supervisor | durable execution 字节级一致 | 1 周 |
| 4 | `R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10` | **aGLM PODA cycle** | apeireth-evolution | AI 自主成长循环 | 3-5 天 |
| 5 | `R124-2-BORROW-SWE-bench-OpenAI-Verified-2024-08-2026-08-10` | **SWE-bench Verified 范式** | apeireth-asi + eval | FAIL_TO_PASS / PASS_TO_PASS 评估 | 1-2 周 |
| 6 | `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` | **OpenCog Atomspace + ECAN** | apeireth-cognition + consciousness | AGI 认知架构 30 年沉淀 | 1-2 周 |
| 7 | `R124-3-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` | **NVIDIA Guardrails Colang DSL** | apeireth-sovereignty | 守门 DSL 范式 | 2-3 天 |
| 8 | `R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` | **OpenCode 子代理 + AGENTS.md 持久化** | apeireth-tui | 199KB → 120KB (-40%) | 3-5 天 |
| 9 | `R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10` | **LangGraph StateGraph 抽象** | apeireth-graph | 状态机成熟 | 1 周 |
| 10 | `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10` | **MCP servers 协议对齐** | apeireth-mcp | 89.4k ⭐ 协议参考 | 1-2 天 |

### Tier 2: ROI 高, R125 末 / R126 续

| # | 借鉴 ID | 项目 | 目标 | ROI | 周期 |
|---|---|---|---|---|---|
| 11 | `R124-1-BORROW-clap-rs/clap-4f7a2c1-2026-08-10` | **clap derive** | apeireth-cli/commands.rs 26.5KB → 12KB (-55%) | 高 | 4-6 h |
| 12 | `R124-1-BORROW-hyperium/hyper-util-2e9d4b6-2026-08-10` | **hyper 池复用** | apeireth-http-client | 中 | 1 天 |
| 13 | `R124-3-BORROW-PyO3/PyO3-2026-08-10` | **PyO3 重构** | apeireth-pybridge | 中 | 1-2 天 |
| 14 | `R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10` | **Kani 形式化验证** | apeireth-formal 24 LOCKED 全覆盖 | 高 | 2-3 天 |
| 15 | `R124-3-BORROW-asg017/sqlite-vec-v0.1.0-2026-08-10` | **sqlite-vec 单文件降级** | apeireth-vector | 中 (R120 A 已真接) | 1 天 |

**注**: #15 R120 A 已真接 sqlite-vec (1000 条 p99 1ms, 50x 加速), R125-11 任务是"评估单文件降级路径" (是否完全脱离服务端 sqlite).

---

## 3. R125+ 14 任务派活清单 (主人 "你拍" 授权持续)

### 3.1 R125-1 (17:30 截止, 50 min, Mavis 调度下个 tick 派)

**任务**: LiteLLM style Provider Registry 骨架
- **位置**: `crates/apeireth-pipeline/src/provider_registry.rs` (NEW mod)
- **借鉴 ID**: `R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10`
- **核心**:
  - `trait Provider { fn kind() + fn endpoint_url() + async fn dispatch() + fn supports_stream() }`
  - `ProviderRegistry { HashMap<ProviderKind, Arc<dyn Provider>> }`
  - 1 stub provider (openai, 走 protocol_handlers::dispatch 现有 1.0 行为)
  - 8 unit test 必过
- **整合 R122-5**: semantic_router 0 替换, 作为 ProviderRegistry 路由器上层
- **8 硬墙全守**: workspace.version 1.1.0, R11 baseline, 24 LOCKED, 6 哲学锚, 9 organ, 11 公共 API, 0 装, 0 主动 commit

### 3.2 R125-2 ~ R125-14 (R125 末, 17:30 后分批)

按 ROI 优先级 + 借鉴源码 ready 状态分批派 (10 任务):

| 任务 | 借鉴 | 目标 | 估时 | 派活条件 |
|---|---|---|---|---|
| **R125-2** | clap derive | apeireth-cli commands.rs 26.5KB → 12KB | 4-6 h | clap-rs/clap clone ready |
| **R125-3** | hyper 池 | apeireth-http-client LIFO 复用 | 1 天 | hyperium/hyper clone ready |
| **R125-4** | MCP servers 协议 | apeireth-mcp 协议对齐 | 1-2 天 | modelcontextprotocol/servers clone ready |
| **R125-5** | NVIDIA Guardrails Colang DSL | apeireth-sovereignty 守门 DSL | 2-3 天 | NVIDIA-NeMo/Guardrails clone ready |
| **R125-6** | OpenCog Atomspace + ECAN | apeireth-cognition hypergraph | 1-2 周 | opencog/opencog clone ready (AGPL-3.0 ⚠️) |
| **R125-7** | aGLM PODA cycle | apeireth-evolution EvolutionCycle | 3-5 天 | GATERAGE/aglm clone ready |
| **R125-8** | Chidori host-call journal | apeireth-supervisor JournalEntry | 1 周 | ThousandBirdsInc/chidori clone ready |
| **R125-9** | PyO3 重构 | apeireth-pybridge 重构 | 1-2 天 | PyO3/PyO3 clone ready |
| **R125-10** | Kani 形式化 | apeireth-formal 24 LOCKED 全覆盖 | 2-3 天 | model-checking/kani clone ready |
| **R125-11** | sqlite-vec 单文件降级 | apeireth-vector 单文件降级 | 1 天 | asg017/sqlite-vec clone ready |
| **R125-12** | OpenCode 子代理 + AGENTS.md | apeireth-tui 199KB → 120KB | 3-5 天 | sst/opencode clone ready |
| **R125-13** | LangGraph StateGraph | apeireth-graph 状态机 | 1 周 | langchain-ai/langgraph clone ready |
| **R125-14** | obra/superpowers Skill | apeireth-central Skill trait | 1-2 天 | (调研报告足够, 0 需 clone) |

**注**: opencog 是 AGPL-3.0 ⚠️, 仅 reference 不集成, 避免传染主仓. Mavis 0 主动 commit 严守.

**R125 阶段总估时**: 14 任务 2-3 周完成 (并行 4-5 任务 / 周), 借鉴源码已就绪 (Top 10 git clone 跑中).

---

## 4. R122-10 重构机会续 (R125 末 / R126 衔接)

R122-10 refactor scan 5 重复模式 + 5 拆 crate + 1 dead code:

### 4.1 5 重复模式抽象 (R125 末)

| # | 模式 | 抽象 | 任务 | 状态 |
|---|---|---|---|---|
| 1 | 4 协议 handler 重复 | trait `ProtocolHandler` + `route_dispatch()` | R123-2 骨架 ✅ + R125-1 真接 4 handler (17:30 末) | 1.0 骨架 done |
| 2 | 5 Evictor | `macro_rules! impl_policy_label` | R125-15 | 1-2 h |
| 3 | 4 auth provider | trait `AuthProvider` + `dispatch_by_platform()` | R125-16 | 1 天 |
| 4 | 5 stage | `Stage::new(name)` 工厂 + `Pipeline<Stage>` 通用驱动 | R125-17 | 1-2 天 |
| 5 | 4 tool category | 0 抽象 (D-2 已 OK) | 0 任务 | done |

### 4.2 5 拆 crate 候选 (R126 路线图, 主人拍板)

| 拆 crate | 父 crate | 估大小 | 理由 |
|---|---|---:|---|
| `apeireth-tui-backend` | apeireth-tui | 100KB | state 持久化, 跟 frontend (渲染) 解耦 |
| `apeireth-keyring-platform-3` | apeireth-keyring | 90KB | macOS/Windows/Linux 三平台 impl 各占 30% |
| `apeireth-constraint-engine` | apeireth-constraint | 50KB | 4 gate + risk level, 跟 permission 拆 |
| `apeireth-classifier-core` | apeireth-tool-registry | 40KB | D-2 Classifier 抽独立 crate |
| `apeireth-pipeline-derive` | apeireth-pipeline | 80KB | 5 stage macro derive 抽独立 proc-macro crate |

### 4.3 1 真死 (R125-18)

- **apeireth-test** placeholder crate — 物理删除整个 crate (per 04 §2.2)

---

## 5. 整合 #3 升级 commit 拍板 (17:30 节点)

### 5.1 17:30 节点交付清单

- **R123-1 完成 + commit 拍板** (Mavis 调度下个 tick, 主人 14:56 "你拍" 授权持续)
- **R125-1 实施** (LiteLLM Provider Registry 骨架, 50 min)
- **R124-1/2/3 调研报告 commit** (138KB 报告, 0 触碰 src, Mavis 拍板)
- **R124-2 task mark done** (报告 47KB 已写完)
- **Top 10 借鉴源码** git clone 完成 (background 跑中)
- **references/borrowed-repos/README.md 索引** (克隆完写)

### 5.2 整合 #3 commit 内容 (17:30 后)

按 Mavis 拍板策略 (per decision-17 + 18 + 19 + 20 + 21), 整合 #3 commit 包含:
- R123-1 完成 src 改动 (clippy 清 + doc 清)
- R125-1 src 改动 (provider_registry.rs)
- 3 调研报告 (R124-1/2/3)
- 5 早 R120 final 报告 (A/B/C/D/D-2 已 commit, 0 重 commit)
- 决策 + 路线图 (decision-19/20/21 + upgrade-roadmap)

### 5.3 17:30 final report

`reports/final-17-30-r123-r124-r125-2026-08-10.md` (per 主人 14:56 "你拍" + 16:23 "制定后续升级计划") 涵盖:
- R123-1 done + 1 commit
- R124-1/2/3 调研 commit
- R125-1 实施 + 1 commit
- 整合 #3 收尾 (1+ commits)
- 17:30 后 R125-2 ~ R125-14 派活清单 + 时间表
- 主人决定 R125 续 11 任务 vs 暂停

---

## 6. 实施时间表 (16:30 → 17:30 → 1.0 release)

### 6.1 17:30 前 (整合 #3 节点)

| 时间 | 动作 | 状态 |
|---|---|---|
| **16:30-16:35** | Top 10 借鉴 git clone (background 跑中) | 🔵 |
| **16:30-16:45** | 写本路线图 + decision-21 + R125-1 spec | 🟢 在写 |
| **16:35-16:45** | Mavis 调度派 R125-1 (LiteLLM Provider Registry 骨架) | 🟡 等下个 tick |
| **16:45-17:30** | R125-1 实施 (50 min, 含 8 unit test) | 🟡 派活后启动 |
| **17:30** | R123-1 done + R125-1 done + 整合 #3 commit | 🟡 计划 |

### 6.2 17:30 后 (R125 主线, 主人拍板续)

| 周次 | 任务 | 产出 |
|---|---|---|
| **W1** (8/11-8/17) | R125-2 (clap) + R125-4 (MCP) + R125-14 (superpowers) | 3 commit, -50KB |
| **W2** (8/18-8/24) | R125-3 (hyper) + R125-5 (Guardrails) + R125-9 (PyO3) + R125-11 (sqlite-vec) | 4 commit, -30KB |
| **W3** (8/25-8/31) | R125-7 (aGLM) + R125-12 (OpenCode) + R125-15 (Evictor macro) | 3 commit, -80KB |
| **W4** (9/1-9/7) | R125-6 (OpenCog) + R125-8 (Chidori) + R125-13 (LangGraph) | 3 commit, 0 KB 但认知架构 30 年沉淀 |
| **W5-6** (9/8-9/21) | R125-10 (Kani) + R125-16/17 (auth/stage macro) + R125-18 (删 apeireth-test) | 3 commit, -25KB |

### 6.3 R126 (Q4 2026, 9-10 月)

5 拆 crate + 4 协议 handler trait 真接 + StateGraph 抽象 (R125-13 续) + 集成测试.

### 6.4 R127 (1.0 release 前, 11-12 月)

ASI 24 维 + Skill 化 (R125-14 续) + 集成测试 + 文档 + release notes.

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **借鉴源码 git clone 慢/失败** | Top 10 clone 5-10 min 风险, 后续 R125-2~14 派活阻塞 | --depth 1, background, 失败 1 次重试, 主仓外 0 污染 |
| **opencog AGPL-3.0 传染** | 主仓 LICENSE 风险 | 仅 reference 不集成, R125-6 任务标"参考不抄码" |
| **R125 派活 task 工具挂** (R122-11 教训) | R125 续阻塞 | Mavis 自干 spec 备 0 阻塞 (本报告已写) |
| **R125 实施 50 min 紧** | 17:30 截止风险 | R125-1 spec 锁定 (骨架 + 1 provider + 8 test), 0 范围扩散 |
| **8 硬墙破** (R122 协调事故教训) | 整合 #3 失败 | git worktree 隔离, 主分支 0 同时改, 8 硬墙 verify 每个 commit |
| **主人 GitHub remote 未配** | 17:30 后 R125 commit 难 | 0 主动 commit, 留主人 1.0 release 前配 remote, 整合 #3 仍可拍板 |
| **借鉴 ROI 不达预期** (R124 调研假设) | R125 续 ROI 失真 | 每个 R125 任务实施前 Mavis verify 借鉴代码 + 设计, 0 假装 |

---

## 8. 拍板执行 (Mavis 自主, 主人 14:56 "你拍" + 16:23 明确授权)

- [x] 写本路线图文件 `upgrade-roadmap-post-r124-2026-08-10.md`
- [x] 写决策 #21 (派活 + clone 清单 + R125-1 spec)
- [x] 启动 Top 10 借鉴 git clone (background, task `bg_56e2ee14`)
- [ ] Mavis 调度下个 tick 派 R125-1 (LiteLLM Provider Registry 骨架, 17:30 截止)
- [ ] git clone 完写 `references/borrowed-repos/README.md` 索引 (10 项目 + 借鉴 ID 索引)
- [ ] 17:30 写 final-17-30 报告 + 拍板整合 #3 commit
- [ ] 主人决定 17:30 后 R125-2 ~ R125-14 (14 续任务) 派活节奏
- [ ] 主人 GitHub remote 配 URL + token (R125 commit 推)

---

**Mavis 16:30 状态**: 调研完成 + 路线图定稿 + Top 10 借鉴源码下载启动, 17:30 整合 #3 节点准备就绪. 主人 1.0 release 路线图清晰, 3 阶段 14 借鉴任务 + 5 抽象 + 5 拆 crate, 预计 2-3 周完成 R125 + 9-10 月 R126 + 11-12 月 R127.
