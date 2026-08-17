# R180 GitHub 优秀项目调研 — council (multi-agent) 模块

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R180
> **日期**: 2026-08-13
> **范围**: apeireth-council 当前实现 (7 Advisor + 4 协作模式 + graph orchestration + MCP bridge) 的可升级方向
> **状态**: 调研为升级预备. council 模块已经实现深度比大多数 multi-agent 框架深 (5 字段宪法 + 按住机制 + 拟人化), 调研目的是找外部 SOTA 借鉴 + 自研强化点.

---

## 0. 现状 — council 已经做了什么

apeireth-council 31 src 文件:
- 7 强制 Advisor: ethics / history / legal / performance / philosophy / safety / strategy
- 4 协作模式: planner_executor / debate / hierarchical / voting
- 按住机制 (30% 强反对 / 一致反对 / 60s 裁决超时)
- 5 字段宪法 (1:1 镜像 R11 5 重守门)
- Multi round deliberation (3 轮) + CouncilMember persona
- GroupChat 图编排 + graph_orchestration
- MCP Prompt/ResourceServer 桥接 (R115)
- bus 真接 (R111) + cognition summary 桥 (R113)
- 3 生命周期: persistent / ephemeral / dynamic
- reasoning trace 可视化 (3 输出格式)
- session_capture 自动捕获 (R150 P1 #10)

已经形成的事实架构: **角色宪法 + 多轮协商 + 按住熔断 + graph orchestration + MCP 桥**. 不止借鉴, 自研成分很高.

---

## 1. Python 生态 SOTA (不集成, 学设计)

### 1.1 LangGraph (langchain-ai/langgraph) — **RECOMMENDED 学习**

- **GitHub**: https://github.com/langchain-ai/langgraph
- **Stars**: 14K+ (2026-08)
- **License**: MIT
- **定位**: Stateful graph-based multi-agent orchestration
- **核心能力**:
  - 有向循环图 (DAG + 循环) 作为一等公民
  - Checkpoint / time-travel / human-in-the-loop
  - Tool calling 集成
  - Streaming + async 一等
  - 状态机 reducers
- **关键设计**:
  - StateGraph = 节点 + 边 + 状态
  - dd_node / dd_edge / dd_conditional_edges
  - 持久化 backend 抽象 (Memory / Postgres / Redis)
  - LangSmith 集成可观测性

**为什么必须学**:
- 业内事实标准, 我们 graph_orchestration 跟它同思路
- Checkpoint + time-travel 设计我们没有
- 持久化 backend 抽象我们没有

**借鉴方案** (草案):
`
ust
// apeireth-council/src/orchestration/checkpoint.rs
pub trait CheckpointBackend: Send + Sync {
    async fn save(&self, state: &GraphState) -> Result<CheckpointId, Error>;
    async fn load(&self, id: CheckpointId) -> Result<GraphState, Error>;
    async fn list(&self, filter: CheckpointFilter) -> Result<Vec<CheckpointId>, Error>;
}

// apeireth-council/src/orchestration/time_travel.rs
pub struct TimeTravel {
    checkpoints: Arc<dyn CheckpointBackend>,
    current: CheckpointId,
}

impl TimeTravel {
    pub async fn rewind_to(&self, id: CheckpointId) -> Result<GraphState, Error>;
    pub async fn fork_from(&self, id: CheckpointId) -> Result<GraphHandle, Error>;
    pub async fn diff(&self, a: CheckpointId, b: CheckpointId) -> Result<StateDiff, Error>;
}
`

### 1.2 AutoGen (microsoft/autogen) — **学习标杆**

- **GitHub**: https://github.com/microsoft/autogen
- **Stars**: 49K+
- **License**: MIT (v0.4+ 改 MIT, 之前 CC-BY-3.0)
- **定位**: Conversable agents + group chat + code execution
- **核心能力**:
  - ConversableAgent / AssistantAgent / UserProxyAgent
  - GroupChat / GroupChatManager (动态决定下一个发言者)
  - Nested chat
  - Code executor sandbox
  - 多 LLM provider 抽象

**为什么必须学**:
- 我们已经借鉴 AutoGen (R33-4), 但只借鉴了 role/goal/backstory/provider
- 继续借鉴 GroupChat 的 \"动态下一个发言者\" 决策算法

**借鉴方案**:
`
ust
// apeireth-council/src/group_chat/next_speaker.rs
pub trait NextSpeakerSelector: Send + Sync {
    async fn select(&self, history: &ChatHistory, advisors: &[Advisor])
        -> Result<AdvisorId, SelectionError>;
}

pub struct LlmDrivenSelector { llm: Arc<dyn LlmProvider> }
pub struct RoundRobinSelector;
pub struct WeightedSelector { weights: HashMap<AdvisorId, f32> }
pub struct ManualSelector;
`

### 1.3 CrewAI (crewAIInc/crewAI) — **学习简化**

- **Stars**: 30K+
- **License**: MIT
- **定位**: 简单上手的 multi-agent, role-based
- **核心能力**:
  - Agent (role + goal + backstory)
  - Task (description + expected_output + agent)
  - Crew (agents + tasks + process)
  - Process: sequential / hierarchical
- **学习点**: API 极简, 易用性

### 1.4 OpenAI Swarm (openai/swarm) — **学习简化**

- **License**: MIT
- **定位**: 极简 multi-agent, handoff 模式
- **学习点**: handoff 比 group_chat 更轻量
- **价值**: 我们 collaboration::Hierarchical 可以借鉴 handoff

### 1.5 Microsoft Magentic-One — **学习**

- **License**: MIT
- **定位**: 通用 multi-agent for web/file/code tasks
- **学习点**: Orchestrator + WebSurfer / FileSurfer / Coder / ComputerTerminal 四 agent 协作
- **价值**: 我们 advisor system + tool integration 的参考架构

### 1.6 kyegomez/swarms (Python) — **学习**

- **License**: Apache 2.0
- **Stars**: 5K+, 5168 commits
- **定位**: Enterprise-grade multi-agent orchestration
- **学习点**: 多种 topology (SpreadSheet / Swarm / GroupChat / AutoBuild)
- **价值**: 我们 graph_orchestration 拓扑可借鉴 SpreadSheet 模式

---

## 2. Rust 生态 multi-agent

### 2.1 swarms-rs (The-Swarm-Corporation/swarms-rs) — **参考架构**

- **GitHub**: https://github.com/The-Swarm-Corporation/swarms-rs
- **License**: Apache 2.0
- **状态**: 162 commits, 活跃
- **定位**: Enterprise-grade Rust multi-agent orchestration
- **核心能力**:
  - 完整 Swarm / GroupChat / Sequential / Hierarchical topology
  - 多 LLM provider (OpenAI / Anthropic / Ollama / Cohere)
  - Tool calling 抽象
  - 状态持久化
  - Async runtime (tokio)

**为什么必须看**:
- 唯一 Rust 生态的企业级 multi-agent 框架
- API 设计与 Python swarms 对齐, 学习资料丰富
- 我们可以借鉴其 Swarm topology 实现

**借鉴方案**:
- 不集成 (避免外部 dep 风险)
- 借鉴其 Swarm / GroupChat / Hierarchical 拓扑实现到 apeireth-council::orchestration
- License 友好 (Apache 2.0), 如果真要 fork 关键文件可标注

### 2.2 LangChain-Rust (abrahmae/rust-langchain-or-llm-chain) — 学习

- 社区维护, 不如 Python 版成熟
- **不选**

### 2.3 Rig (rig-rs/rig) — **学习**

- **License**: MIT
- **定位**: LLM 框架, 包含 agent abstraction
- **学习点**: tool/agent 抽象设计

---

## 3. 横向创新点 (我们没做的)

### 3.1 Constitutional AI (Anthropic)

- 不是项目, 是方法论
- 自我批评 + 自我修订循环
- 我们 5 字段宪法类似, 但 constitutional AI 是 runtime 反馈循环
- **借鉴价值**: 给 council 加 \"宪法一致性审计\" 步骤

### 3.2 Debate-based safety (Irving et al. 2018)

- 让两个 agent 辩论, 第三方裁决
- 我们 debate 模式已经有了
- **强化价值**: 加 \"辩论深度自适应\" (分歧大 -> 多轮)

### 3.3 Recursive Reward Modeling (Leike et al. 2018)

- Agent 给 agent 打分
- 适合我们 \"按住机制\" 的强化
- **借鉴价值**: hold 阈值可自适应

### 3.4 Scalable Oversight (Christiano et al. 2018)

- 弱监督者监督强 agent
- 我们 L0 HA 物理多签就是 scalable oversight 的一种
- **借鉴价值**: 写作 R180+ 哲学依据

---

## 4. 升级方案 (最终阶段执行)

### 4.1 短期 (1-2 days)

1. **Checkpoint / Time Travel**: 借鉴 LangGraph, 给 graph_orchestration 加持久化
2. **Next Speaker Selector**: 借鉴 AutoGen, group_chat 动态决策可插件化
3. **Handoff**: 借鉴 OpenAI Swarm, hierarchical 加 handoff 模式

### 4.2 中期 (3-5 days)

4. **SpreadSheet Topology**: 借鉴 swarms-rs, 多 agent 共享工作区
5. **Constitutional Audit**: 借鉴 Constitutional AI, 每次 deliberation 后做一致性审计
6. **Multi-Provider LlmBackend**: 借鉴 swarms-rs, 抽象 OpenAI / Anthropic / MiniMax provider

### 4.3 长期 (持续)

7. **Recursive Reward Modeling**: 强化按住机制
8. **Self-Organizing Topology**: agent 动态决定协作模式

---

## 5. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| sovereignty | council 通过 SovereigntyHook 集成 |
| cognition (R113) | council deliberation context 接 cognition summary |
| bus (R111) | council deliberation event 真接到 bus |
| mcp_bridge (R115) | CouncilMember -> MCP Prompt/ResourceServer |
| tui | CouncilMember 是 TUI 拟人化基础 |
| pipeline | council 决策可作为 pipeline step |

---

## 6. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- council 公开 API: 0 改 (新功能在子模块内, 旧 trait 不动)

---

## 7. 参考链接

- LangGraph: https://github.com/langchain-ai/langgraph
- AutoGen: https://github.com/microsoft/autogen
- CrewAI: https://github.com/crewAIInc/crewAI
- OpenAI Swarm: https://github.com/openai/swarm
- Magentic-One: https://github.com/microsoft/Magentic-One
- kyegomez/swarms: https://github.com/kyegomez/swarms
- swarms-rs: https://github.com/The-Swarm-Corporation/swarms-rs
- Rig: https://github.com/rig-rs/rig
- Anthropic Constitutional AI: https://www.anthropic.com/news/constitutional-ai-harmlessness-from-ai-feedback
- Irving 2018 Debate: https://arxiv.org/abs/1805.00899
- Christiano 2018 Scalable Oversight: https://arxiv.org/abs/1811.01457