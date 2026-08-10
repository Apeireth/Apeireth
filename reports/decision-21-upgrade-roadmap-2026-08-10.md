# Decision #21 — R125+ 升级路线图 (基于 R124-1/2/3 + R122-10 + R123 调研)

**Date**: 2026-08-10 16:25
**Author**: Mavis (root session, 主人 16:23 "制定后续的升级计划, 有需要借鉴源码的就把项目下载下来" 明确 go-ahead)
**Status**: ✅ 决策登记 + 路线图定稿 + Top 10 借鉴源码 git clone 启动 (background)

---

## 0. 触发事件

主人 16:23 拍板:
- "那些调研你都看一下" — R124-1/2/3 + R122-10 + R123 + R120 全看
- "制定后续的升级计划" — 写 R125+ 路线图
- "有需要借鉴源码的就把项目下载下来" — git clone Top 借鉴项目

**Mavis 授权**: 主人 14:56 "你拍" 仍有效, 0 触碰 8 硬墙前提下, 路线图派活 + 下载 + 实施自主决策.

---

## 1. 调研综合 (R122-10 + R123 + R124 全 5 报告)

| 报告 | 大小 | 候选 | 借鉴 | ID | Top ROI |
|---|---:|---:|---:|---:|---|
| **R122-10** (Mavis 自干 refactor scan) | 7.7KB | 15 大文件 | 5 重复模式 | 10 TODO 优先级 | 4 协议 handler trait 抽象 (R123-2 已做 1.0 骨架) |
| **R123-2/3/4** (实施类) | 17.5KB | — | — | — | trait + MCP + 多模态 — R125+ 续真接 |
| **R124-1** (战区 1-2: TUI/LLM Gateway 8 模块) | 41.7KB | 28 | 30 | 22 | LiteLLM provider registry **-209.6KB (-65%)** |
| **R124-2** (战区 3: Multi-Agent 13 模块) | 47.0KB | 20+ | 39 | 42+ | OpenCog ECAN + aGLM PODA + Chidori journal |
| **R124-3** (战区 4-5+L0+跨战区 16 模块) | 49.2KB | 64 | 68 | 77 | MCP servers / PyO3 / NVIDIA Guardrails / Kani / sqlite-vec |
| **总计** | **163KB** | **127+** | **142+** | **151+** | **Top 5 借鉴 ROI 总计 -209.6KB 代码 + 认知架构 30 年沉淀** |

---

## 2. R125+ 升级路线图 (5 阶段, Mavis 派活)

### Phase 1: 借鉴源码下载 (16:25-16:35, 10 min, Mavis 自干)

**Top 10 借鉴项目 git clone** 到 `.openclaw\workspace\borrowed-repos\` (主仓外, 0 污染):

| # | owner/repo | 借鉴对应 | 协议 | 估大小 |
|---|---|---|---|---|
| 1 | BerriAI/litellm | R124-1 API-1/2 + R125-1 核心 | MIT | ~50MB |
| 2 | langchain-ai/langgraph | R124-1 PIPELINE-1/3 + R124-2 B-001/B-007/B-011 | MIT | ~30MB |
| 3 | sst/opencode | R124-1 TUI-1/3 子代理模式 | MIT | ~80MB |
| 4 | modelcontextprotocol/servers | R124-3 Top 1 + R125-4 MCP 协议 | MIT | ~30MB |
| 5 | PyO3/PyO3 | R124-3 Top 2 + R125-9 pybridge 重构 | Apache-2.0/MIT | ~20MB |
| 6 | NVIDIA-NeMo/Guardrails | R124-3 Top 3 + R125-5 Colang DSL | Apache-2.0 | ~50MB |
| 7 | model-checking/kani | R124-3 Top 4 + R125-10 形式化验证 | Apache-2.0/MIT | ~10MB |
| 8 | asg017/sqlite-vec | R124-3 Top 5 + R125-11 vector 降级 | MIT | ~5MB |
| 9 | opencog/opencog | R124-2 B-028 Atomspace ECAN | AGPL-3.0 ⚠️ | ~200MB |
| 10 | ThousandBirdsInc/chidori | R124-2 B-006 host-call journal | MIT | ~30MB |

**注**: opencog 是 AGPL-3.0, 仅 reference 不集成 (避免传染). 其余 MIT/Apache 0 顾虑.

**git clone 命令** (background, --depth 1):
```bash
cd .openclaw\workspace\borrowed-repos
git clone --depth 1 https://github.com/BerriAI/litellm.git
git clone --depth 1 https://github.com/langchain-ai/langgraph.git
git clone --depth 1 https://github.com/sst/opencode.git
git clone --depth 1 https://github.com/modelcontextprotocol/servers.git
git clone --depth 1 https://github.com/PyO3/PyO3.git
git clone --depth 1 https://github.com/NVIDIA-NeMo/Guardrails.git
git clone --depth 1 https://github.com/model-checking/kani.git
git clone --depth 1 https://github.com/asg017/sqlite-vec.git
git clone --depth 1 https://github.com/opencog/opencog.git
git clone --depth 1 https://github.com/ThousandBirdsInc/chidori.git
```

### Phase 2: R125-1 实施 (16:25-17:30, 65 min, Mavis 调度派)

**任务**: LiteLLM style Provider Registry 骨架
- **位置**: `crates/apeireth-pipeline/src/provider_registry.rs` (NEW mod, 50 min)
- **借鉴 ID**: `R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10`
- **8 硬墙全守**: workspace.version 1.1.0, R11 baseline 3 值, 24 LOCKED, 6 哲学锚, 9 organ, 11 公共 API, 0 装, 0 主动 commit
- **8 unit test 必过**: register / dispatch / 4 协议 / Send+Sync / semantic_router 0 漂移
- **整合 R122-5**: semantic_router 0 替换, 作为 ProviderRegistry 路由器上层
- **17:30 截止**: 50 min 骨架 + 1 provider (openai) + 8 test, 完整 4+ provider 留 R125-2/3 续

### Phase 3: R125-2 ~ R125-11 续 (17:30 后, 主人 14:56 "你拍" 授权持续)

按 ROI 优先级派 10 个独立 sub-agent 续实施, 每个 1 借鉴 + 8 硬墙 + 17:30 后分批:

| 任务 | 借鉴 ID | 目标 | ROI |
|---|---|---|---|
| **R125-2** | `R124-1-BORROW-clap-rs/clap-4f7a2c1-2026-08-10` | `apeireth-cli/commands.rs` 26.5KB → 12KB (-55%) | 高 / 4-6 h |
| **R125-3** | `R124-1-BORROW-hyperium/hyper-util-2e9d4b6-2026-08-10` | `apeireth-http-client` 池复用 | 中 / 1 天 |
| **R125-4** | `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10` | `apeireth-mcp` 协议对齐 | 高 / 1-2 天 |
| **R125-5** | `R124-3-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` | `apeireth-sovereignty` Colang DSL 借鉴 | 极高 / 2-3 天 |
| **R125-6** | `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` | `apeireth-cognition` Atomspace hypergraph | 极高 / 1 周 |
| **R125-7** | `R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10` | `apeireth-evolution` PODA cycle | 极高 / 3-5 天 |
| **R125-8** | `R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10` | `apeireth-supervisor` host-call journal | 极高 / 1 周 |
| **R125-9** | `R124-3-BORROW-PyO3/PyO3-2026-08-10` | `apeireth-pybridge` 重构 | 中 / 1-2 天 |
| **R125-10** | `R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10` | `apeireth-formal` 24 LOCKED 全覆盖 | 高 / 2-3 天 |
| **R125-11** | `R124-3-BORROW-asg017/sqlite-vec-v0.1.0-2026-08-10` | `apeireth-vector` 单文件降级 | 中 / 1 天 |
| **R125-12** | `R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` | `apeireth-tui` 子代理拆 9 器官 (199KB → 120KB, -40%) | 极高 / 3-5 天 |
| **R125-13** | `R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10` | `apeireth-graph` StateGraph 抽象 | 极高 / 1 周 |
| **R125-14** | `R124-2-BORROW-obra/superpowers-2026-05-2026-08-10` | `apeireth-central` Skill trait | 高 / 1-2 天 |

**总 ROI 估算**: 14 R125 任务 2-3 周完成, 代码减少 -250KB+, 认知架构 (aGLM/OpenCog/Chidori) 30 年沉淀落地, 主人 1.0 release 路线图完整.

### Phase 4: R122-10 抽象与拆 crate (R125 末 / R126 衔接)

按 R122-10 refactor scan §2 5 重复模式:
1. **4 协议 handler trait 真接** (R123-2 trait 骨架 + R125-1/13 真接 4 handler impl) — R125 末
2. **5 Evictor macro** (`macro_rules! impl_policy_label`) — R125-15, 25 行
3. **4 auth provider trait** (AuthProvider + dispatch_by_platform) — R125-16
4. **5 stage macro** (`Stage::new(name)` 工厂 + `Pipeline<Stage>` 通用驱动) — R125-17
5. **5 拆 crate 候选** (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive) — R126 路线图, 主人拍板

**1 真死** (apeireth-test placeholder) — 物理删除, R125-18 任务.

### Phase 5: 整合 #3 final commit (R125 末 / 17:30 节点)

- 整合 #3 commit 收尾 (R125 各任务 src + 报告 + 决策)
- 0 越界 8 硬墙
- 主人 "你拍" 授权持续到主人收回
- 17:30 节点写 R123+R124+R125 final report + 拍板 commit

---

## 3. 时间表 (16:25-17:30 紧节奏)

| 时间 | 动作 | 状态 |
|---|---|---|
| **16:25-16:35** (10 min) | Phase 1: Top 10 借鉴源码 git clone (background) | 🔵 启动 |
| **16:25-16:45** (20 min) | Phase 2 spec: decision-21 + upgrade-roadmap + R125-1 派活 spec | 🟢 在写 |
| **16:30-17:30** (60 min) | Phase 2 实施: R125-1 LiteLLM Provider Registry 骨架 | 🟡 Mavis 调度下个 tick 派 |
| **17:30** | R123+R124+R125 final report + 拍板 commit | 🟡 计划 |

---

## 4. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 借鉴源码 git clone 慢/失败 | 10 min 风险 → Phase 2 17:30 截止紧 | background, --depth 1, 顺序 5+5 项目, 失败重试 1 次 |
| R125-1 实施 50 min 紧 | 17:30 截止风险 | R125-1 spec 锁定 (LiteLLM style + 1 provider + 8 test), 0 范围扩散 |
| 借鉴 8+ 项目大 | 5-15 min 网络 + 1-2 GB 硬盘 | 顺序 5 + 5, 失败可中断, 主仓外 0 污染 |
| R125 派活 task 工具挂 | R122-11 教训 | Mavis 自干 (decision-21 + R125-1 spec) 备 0 阻塞 |
| 主人 GitHub remote 未配 | R125 派活 commit 难 | 0 主动 commit, 留主人 1.0 release 前配 remote |

---

## 5. 0 拍板 R125-1 + 借鉴 clone 启动

- [x] 写本决策文件
- [x] git clone 启动 (background)
- [ ] Mavis 调度下个 tick 派 R125-1 (LiteLLM Provider Registry 骨架)
- [ ] 17:30 写 R123+R124+R125 final report + 拍板 commit
- [ ] 主人决定 17:30 后是否继续 R125-2 ~ R125-14 (14 续任务)
