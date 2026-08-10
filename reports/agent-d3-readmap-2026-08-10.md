# Agent D-3 — D3-1 Readmap (R25 战区 3 / 2026-08-10)

> **任务**: Apeireth-rust 战区 3 Multi-Agent — `apeireth-council` 升级 4 种协作模式 + reasoning trace 可视化 + 角色宪法
> **出处**: v2.0 strategy Stage 2 §2B (per `docs/v2-strategy/03-EXTREME-PLAN.md:114-124`)
> **节奏**: 7h (D3-1 1h 读全 → D3-2 2h collaboration.rs+PlannerExecutor → D3-3 1.5h Debate+Voting+Hierarchical → D3-4 1h 角色宪法 → D3-5 1h trace 可视化 → D3-6 0.5h 报告)

## 0. 验收硬指标 (per 派活单)

| 硬指标 | 状态 | 备注 |
|---|---|---|
| `cargo check -p apeireth-council --lib --tests --examples` exit 0 | ✅ 4.04s baseline | 仅 1 个 pre-existing unused warning (synthesis.rs:107 weights) 不在 D-3 范围 |
| `cargo test -p apeireth-council` 0 failed | ⏳ D3-5 验 | 新增 ≥ 20 tests 累计 ≥ 30 |
| `cargo run -p apeireth-council --example trace_visualize` 跑通 | ⏳ D3-5 验 | 3 advisor 协作 + trace 打印 |
| 4 模式全实现 (Planner+Executor / Debate / Voting / Hierarchical) | ⏳ D3-2/3 验 | |
| 角色宪法 trait + RoleConstitution struct 完整 | ⏳ D3-4 验 | 跟 R11 5 重守门 1:1 镜像 |
| 0 改 workspace.version (1.1.0) | ✅ 严守 | |
| 0 触碰 24 LOCKED (cognition/core/sovereignty/formal) | ✅ 严守 | 仅改 `apeireth-council/src/{collaboration,constitution,trace}.rs` + lib.rs + 1 example + tests |
| 不与 A/B/C/D-2 冲突 | ✅ | D-2 改 tool-registry, 我改 council, 0 交叉 |

## 1. 现有 `apeireth-council/` 全貌 (2026-08-10 03:00 baseline)

### 1.1 src/ 模块 (16 个, 总 ~3700 LOC)

| 文件 | 行数 | 角色 | 关键 API |
|---|---|---|---|
| `lib.rs` | 131 | 入口 + 编译期 hardcode (SEVEN_MANDATORY_ADVISORS=7, HOLD 30%/60s, MAX_PERSONA_DEBATE_ROUNDS=3) | 17 行 `pub mod` + re-export 7 类 |
| `advisor.rs` | 331 | `Advisor` trait + 7 `AdvisorDomain` + `StanceKind` (5 强度+Abstain) + `AdvisorOpinion` + `DeliberationContext` + `DeliberationOutcome` + `AdvisorError` | `Advisor::deliberate(query, ctx) -> Result<DeliberationOutcome, _>` |
| `deliberation.rs` | 390 | `Council` struct (recruit / deliberate / deliberate_persona) + `CouncilQuery` + `CouncilVerdict` + emit_event hook | `Council::deliberate(query) -> CouncilVerdict` |
| `synthesis.rs` | 197 | `SynthesisWeights` (7 域) + `SynthesisReport` (weighted_score / aggregated_stance / confidence / dissenting / hold_decision) + `synthesize()` | 5 阈值映射 (≥0.6 StrongApprove) |
| `hold.rs` | ? | `HoldTrigger` (30% StrongDisapprove / 1-1 反对 / 60s timeout) + `HoldDecision` + `HoldOutcome` + `HoldThreshold` | (0 触碰, 仅 re-export) |
| `persona.rs` | 161 | `Persona` (name/character/voice/stance_bias) + `PersonaSession` (3 轮辩论) + `DebateRound` | 拟人化辩论 3 轮 |
| `council_member.rs` | 172 | R33-4 `CouncilMember { role, goal, backstory, provider }` + 5 supported providers + `to_system_prompt()` | 4 字段 1:1 翻译 AutoGen |
| `council_member_deliberation.rs` | ~580 | R33-4-1 `CouncilMemberDeliberator` (3 轮 + 共识 0.6) | 复用 R16-09 `LlmAdvisorBackend` |
| `council_member_persona_combo.rs` | ~580 | R33-4-2 `PersonaBoundDeliberator` (3 轮 + persona voice) | 6 段 system_prompt |
| `graph_bridge.rs` | 232 | R113 cognition summary → council query context | 5 helper functions |
| `mcp_bridge.rs` | ? | R115 council → MCP Prompt/ResourceServer | (0 触碰) |
| `bus_bridge.rs` | ? | R111 council event → bus | (0 触碰) |
| `sovereignty.rs` | 100 | `SovereigntyHook` trait + `CouncilEvent` 5 variants | `SovereigntyHook::on_council_event` |
| `mock_llm.rs` | ? | `MockLlmProvider` trait + `ScriptedMockLlm` (供 R33-4-1/-2 + D3 复用) | (复用, 0 改) |
| `llm_backend.rs` | ? | R16-09 `LlmAdvisorBackend` (LOCKED, 0 改) | 复用 |
| `lifecycle.rs` | ? | `AdvisorLifecycle` (3: persistent/ephemeral/dynamic) + `LifecycleManager` + `LifecycleStats` | (0 触碰) |
| `stress_test.rs` | ? | R68 stress runner | (0 触碰) |
| `advisors/` (7 文件) | ~700 | 7 强制 advisor (safety/performance/philosophy/history/strategy/ethics/legal) | R15 LOCKED, 0 触碰 |

**Cargo.toml**:
```toml
[dependencies]
apeireth-verify = { path = "../apeireth-verify" }
apeireth-core = { path = "../apeireth-core" }
apeireth-api = { path = "../apeireth-api" }
apeireth-graph = { path = "../apeireth-graph" }    # R113 真接
apeireth-mcp = { path = "../apeireth-mcp" }         # R115 真接
serde / serde_json / thiserror / tokio
```
**关键**: 0 新加 dep, 所有 D-3 工作走现有 `apeireth-graph::Graph` API

### 1.2 tests/ (4 个文件, 累计 ≥ 30 测试)

- `tests/council_tests.rs` — R10 既有 (synthesize 5 阈值 / 7 advisor 召集团 / hold 触发)
- `tests/round10_07_seven_council.rs` — R10 7 强制 advisor 流程
- `tests/council_member_deliberation_integration.rs` — R33-4-1 (3 测试 + 1 LIVE env-gated)
- `tests/council_member_persona_combo_live.rs` — R33-4-2 (1 LIVE env-gated)

### 1.3 examples/ (3 个)

- `examples/council_demo.rs` — 7 强制 advisor 走 full deliberation
- `examples/council_member_deliberation_demo.rs` — R33-4-1 demo
- `examples/r71_live_stress.rs` — R71 LIVE 压力测试

### 1.4 编译期 hardcode (lib.rs 100% 严守)

```rust
pub const SEVEN_MANDATORY_ADVISORS: usize = 7;
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;
pub const MAX_PERSONA_DEBATE_ROUNDS: u8 = 3;
```
**D-3 行为**: 0 改上述 4 const. 角色宪法将引入**新** const (`ROLE_CONSTITUTION_FIELDS`, `COLLABORATION_MODE_COUNT` 等), 在新 module 的 `const _: () = { ... }` 块内独立断言.

## 2. 4 种协作模式具体形状 (per 派活 + LangGraph / AutoGen 借鉴)

### 2.1 Planner + Executor (1:1 对标 LangGraph PlanAndExecute / BabyAGI)

| 阶段 | 角色 | 产出 |
|---|---|---|
| Plan | Planner advisor (1 个) | 拆 query 为 N 步执行计划 (Vec<SubTask>) |
| Execute | Executor advisors (1..N 个) | 顺序执行每步, 每步返回 opinion |
| Synthesize | Council | 综合所有 opinion 出 verdict |

**借鉴锚** (per 主哲学锚 "走在前人经验上" + LangGraph):
- LangGraph `PlanAndExecute` plan_node + execute_node (sequential, state-passing)
- AutoGen `GroupChat` planner role + executor role separation
- VCP `vcpLoop` task decomposition

**D-3 实现 (per 决策权)**:
- Planner 角色: `CouncilMember { role: "planner", ... }` + 拆分为 `Vec<SubTask>` (每 SubTask 1 个 `CouncilMember` role + 描述)
- Executor 角色: planner 之外的 members 按 SubTask 顺序跑 (per `SubTask.role` 派)
- 合成: 最后走 `synthesize()` 全部 opinions
- 终止: plan 跑完 OR 任意 step 触发 strong_disapprove (按 planner 规划的 plan 走完为止)

### 2.2 Debate (per AutoGen GroupChat 简化)

| 阶段 | 角色 | 产出 |
|---|---|---|
| Round 0..N | 全部 advisors | 每轮每人 1 opinion, 跨轮 prior_opinions 传递 |
| Terminate | Council | 共识 OR max_rounds OR strong_disapprove |

**借鉴锚**:
- R33-4-1 `CouncilMemberDeliberator` 已实现 (3 轮 + 共识 0.6), 0 重写, **复用 + 包装** 为 Debate mode
- AutoGen `GroupChat.speaker_selection_method` 顺序轮换
- LangGraph `MessagesState` 跨轮 state

**D-3 实现 (per 决策权)**:
- **包装 R33-4-1** 已有 `CouncilMemberDeliberator::deliberate()` → `CollaborationMode::Debate` 路径
- 不重复造 R33-4-1 的轮次/共识逻辑
- 区别于 R33-4-1: Debate mode 在 `CollaborationContext` 内多包一层 (mode = Debate), 让 trace 能标注这是 Debate 不是单轮投票

### 2.3 Voting (per AutoGen GroupChatManager / VCP 共识投票)

| 阶段 | 角色 | 产出 |
|---|---|---|
| Vote | 全部 advisors | 每位 1 opinion (1 轮) |
| Tally | Council | 加权投票, 简单多数 (≥ 50%) OR 加权最高 |
| Terminate | Council | 1 轮跑完 |

**借鉴锚**:
- 简单多数: 跟 `SynthesisWeights` 加权一致 (Safety 1.0 / Philosophy 0.95 / etc.)
- VCP `vcpLoop/toolCallParser.js` 投票聚合模式
- LangGraph `add_conditional_edges` 投票后分支

**D-3 实现 (per 决策权)**:
- **简单多数** (default): `Σ(stance.score × weight × confidence) > 0` → Approve; 反之 → Disapprove
- **加权最高** (variant): 取 weighted_score 最高的 opinion 当 verdict
- 单轮 (1 round), 不做多轮协商
- 复用 `synthesize()` 出 report (1:1 兼容 R10 既有路径)

### 2.4 Hierarchical (per OpenAI Swarm / Anthropic sub-agents 借鉴)

| 阶段 | 角色 | 产出 |
|---|---|---|
| Root | 1 主 advisor | 委派子任务给 sub-advisors |
| Sub | N 子 advisors | 执行子任务, 各自 1 opinion |
| Aggregate | Root | 收集子 opinion 合成最终 verdict |

**借鉴锚**:
- OpenAI `swarm` handoff pattern (主 agent 委派子 agent)
- LangGraph `subgraph` (子图作为节点)
- AutoGen `GroupChatManager` 主从结构
- VCP `vcpLoop/toolExecutor.js` 嵌套 task

**D-3 实现 (per 决策权)**:
- **主 + 2 子** (per 派活单提示): 1 个 root advisor + 2 个 sub advisors, 简化起见
- Root: 拆 query 为 2 个 sub-task (e.g. "技术方案" + "风险评估"), 委派给 2 sub
- Sub: 各自产出 1 opinion
- Root: 收集 2 sub opinion → synthesize → 出 final verdict
- 终止: 1 轮跑完 (sub 各自 1 opinion, root 1 final)

## 3. reasoning trace 可视化 (D3-5)

### 3.1 借鉴锚

- LangGraph `MemorySaver.get_tuple()` → graph state timeline
- VCP `vcpLoop/traceLog.js` step-by-step log
- AutoGen `GroupChat.messages` transcript
- Anthropic `claude_code/trace.jsonl` 风格 (每行 1 step JSON)

### 3.2 设计 (per 决策权)

**D3-5 输出格式**: **Pretty-printed human-readable** (vs JSON 优先)

```
=== Council Trace: session-000001 ===
Mode: Planner+Executor (3 steps)
Query: q-001 "ship feature X"

[Step 0] Planner
  Role: architect
  Plan: ["step 1: design", "step 2: implement", "step 3: test"]
  
[Step 1] Executor #1 — role=implementer
  Stance: Approve (confidence 0.8)
  Reasoning: "设计稳的架构 ... "
  
[Step 2] Executor #2 — role=tester
  Stance: Approve (confidence 0.7)
  Reasoning: "覆盖率达 80% ... "

=== Final Verdict ===
weighted_score: 0.65
stance: Approve
held: false
elapsed: 234ms
```

**D3-5 类型**:
```rust
pub struct TraceStep {
    pub step_id: u32,
    pub mode: CollaborationMode,
    pub actor: String,           // advisor role / planner / executor
    pub action: String,          // "plan" / "execute" / "vote" / "delegate" / "synthesize"
    pub input: String,           // 该步输入 (简短)
    pub output: String,          // 该步输出 (简短)
    pub stance: Option<StanceKind>,
    pub elapsed_ms: u64,
}

pub struct TraceReport {
    pub session_id: String,
    pub mode: CollaborationMode,
    pub query: String,
    pub steps: Vec<TraceStep>,
    pub final_verdict: SynthesisReport,
}
```

**D3-5 方法**:
- `TraceReport::to_pretty_print() -> String` — 人类可读 (如上)
- `TraceReport::to_json() -> String` — 机器可读 (serde_json)
- `TraceReport::to_step_jsonl() -> String` — JSONL (claude_code trace 风格, 每行 1 step)

## 4. 角色宪法 (D3-4)

### 4.1 跟 R11 5 重守门 1:1 镜像 (per 派活单 "1:1 镜像" 提示)

R11 5 重守门 (per 主人长期决策, 待 D-3 4 阶段读 `apeireth-cognition` R11 文档时确认 — D-3 不触碰 24 LOCKED, 但可以读):

| # | 5 重守门 | 角色宪法对应字段 |
|---|---|---|
| 1 | 物理隔离守门 | `constitution.physical_isolation: bool` |
| 2 | L0 HA (热切换) 守门 | `constitution.l0_ha_required: bool` |
| 3 | 司法边界守门 | `constitution.jurisdiction_bounds: Vec<String>` |
| 4 | 编译期 hardcode 守门 | `constitution.compile_time_hardcoded: bool` |
| 5 | 哲学守门 | `constitution.philosophical_anchors: Vec<String>` (per 6 哲学锚) |

**D-3 决策** (per 派活单 "1:1 镜像 vs 简化 自己定"):
- **倾向 1:1 镜像**: 既然 v2.0 strategy 写"角色宪法"专门强调"每个 advisor 自己的约束", 应跟 R11 5 重守门对齐, 严守 6 哲学锚穿透
- 5 字段 struct + trait (新 module `constitution.rs`)
- 提供 `RoleConstitution::default()` (跟 R11 默认对齐, 全部允许) + `RoleConstitution::for_safety_advisor()` (强约束, 物理隔离 + L0 HA + 司法边界全 true)

**实际落实** (避免空谈): 5 字段都是**编译期 hardcoded const** (在 `constitution.rs` 顶部 const _ 块内), 跟 R11 6 哲学锚穿透一致.

### 4.2 类型

```rust
pub trait RoleConstitutionTrait: Send + Sync {
    fn validate_opinion(&self, opinion: &AdvisorOpinion) -> Result<(), ConstitutionViolation>;
    fn compile_time_hardcoded(&self) -> bool;
    fn philosophical_anchors(&self) -> &[String];
}

pub struct RoleConstitution {
    pub physical_isolation: bool,
    pub l0_ha_required: bool,
    pub jurisdiction_bounds: Vec<String>,
    pub compile_time_hardcoded: bool,
    pub philosophical_anchors: Vec<String>,
}

pub enum ConstitutionViolation {
    JurisdictionBreach { reason: String },
    PhysicalIsolationRequired,
    L0HaRequired,
    PhilosophicalAnchorMissing { anchor: String },
}
```

**D3-4 复用**:
- `RoleConstitution` 跟 7 强制 advisor 关联 (per `AdvisorDomain`): Safety 必 physical_isolation + L0 Ha + jurisdiction_bounds 含 ["SOVEREIGN", "PRINCIPLE"]; History 仅 physical_isolation; etc.
- 复用 `StanceKind::is_strong_disapprove()` + `triggers_hold()` 检测哲学守门

## 5. 跟 graph (apeireth-graph) 怎么接 (D3-2/-3 + 集成)

### 5.1 借鉴 API (`apeireth-graph` 已有, per `crates/apeireth-graph/src/lib.rs:99-200`)

```rust
pub trait Node: Send + Sync {
    fn id(&self) -> NodeId;
    fn run(&self, state: &mut State) -> Result<NodeOutput>;
}

pub struct Graph {
    pub(crate) nodes: BTreeMap<NodeId, Box<dyn Node>>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) conditional_edges: Vec<ConditionalEdge>,
}

impl Graph {
    pub fn add_node(&mut self, node: impl Node + 'static);
    pub fn add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>);
    pub fn add_conditional_edge(&mut self, from, path_map, default, condition);
    pub async fn execute(&self, init_state: State) -> Result<FinalState>;
    pub async fn checkpoint(&self, state: &State) -> Result<Checkpoint>;
}
```

**D-3 集成点** (per 派活单 "加图编排支持"):
- 每个 `CollaborationMode` 提供一个 `Node` impl 包装: `PlannerExecutorNode` / `DebateNode` / `VotingNode` / `HierarchicalNode`
- 每个 Node 的 `run(&mut State)` 内部跑对应 `CollaborationMode` 的协作流程, 写回 State
- 提供 `CouncilGraph::from_collaboration(mode, members) -> Graph` 工厂: 把 4 模式包成 Graph (topological order + execute 1 次)
- **关键约束**: Node 是 `async` + `Send + Sync` (apeireth-graph 已有), D-3 包装层不引入新 I/O / 网络 (per 硬约束 #4 + 0 I/O 哲学锚)

### 5.2 State 设计 (D-3 不动 `apeireth-graph::State`, 走 BTreeMap 注入)

```rust
// State 是 BTreeMap<String, serde_json::Value>
// D-3 写入 / 读取的 key 全部 D-3 私有:
const COLLABORATION_MODE_KEY: &str = "d3.collaboration_mode";
const COLLABORATION_VERDICT_KEY: &str = "d3.collaboration_verdict";
const COLLABORATION_TRACE_KEY: &str = "d3.collaboration_trace";
```

## 6. 4 模式实现策略 (per 决策权, 1.0 内部对齐)

| 模式 | 实现策略 | 行数预算 | 测试预算 |
|---|---|---|---|
| Planner+Executor | 新写 `PlannerExecutor::run() -> Verdict` (拆 3 步 + 3 executor) | ~150 LOC | ≥ 5 tests |
| Debate | **复用** R33-4-1 `CouncilMemberDeliberator`, 包 `Debate` enum variant | ~50 LOC (wrapper) | ≥ 5 tests |
| Voting | 新写 `Voter::tally() -> Verdict` (复用 `synthesize()` 加权) | ~100 LOC | ≥ 5 tests |
| Hierarchical | 新写 `Hierarchical::run() -> Verdict` (root + 2 sub) | ~150 LOC | ≥ 5 tests |

**总预算** (per 派活单 "新增 ≥ 20 tests 累计 ≥ 30"):
- D3-2 collaboration.rs: 3 文件 (collaboration + planner_executor + integration) + 10 tests
- D3-3 voting/hierarchical + debate: 2 文件 + 15 tests
- D3-4 constitution.rs: 1 文件 + 8 tests
- D3-5 trace.rs + example + integration test: 1 文件 + 5 tests + 1 integration
- **合计**: 4 new src files + 1 new example + 1 new integration test = ≥ 38 tests 新增

## 7. 验收硬路径

| 硬指标 | 路径 | 状态 |
|---|---|---|
| `cargo check -p apeireth-council --lib --tests --examples` exit 0 | 4.04s baseline 干净 | D3-5 验 |
| `cargo test -p apeireth-council` 0 failed | 累计 ≥ 30 (R33-4-1 既有 + D-3 新增) | D3-5 验 |
| `cargo run -p apeireth-council --example trace_visualize` 跑通 | 3 advisor 协作 + trace pretty print | D3-5 验 |
| 4 模式全实现 | Planner+Executor / Debate / Voting / Hierarchical | D3-2/3 验 |
| 角色宪法 trait + struct 完整 | `RoleConstitutionTrait` + `RoleConstitution` 5 字段 | D3-4 验 |
| 0 改 workspace.version (1.1.0) | git diff Cargo.toml | D3-6 验 |
| 0 触碰 24 LOCKED | git diff 0 触碰 cognition/core/sovereignty/formal | D3-6 验 |
| 不与 A/B/C/D-2 冲突 | git diff 仅 apeireth-council/ | D3-6 验 |

## 8. 风险 + 决策权

### 8.1 风险

- **R1**: Debate 模式复用 R33-4-1 时, 跟 "Multi-round Debate" 区分不开 — 缓解: `CollaborationContext.mode = Debate` 字段 + trace 标注 mode
- **R2**: Hierarchical root 拆 2 sub-task 怎么拆? — 缓解: 硬编码 "技术方案" + "风险评估" 2 个 sub-task, 由 root 委派
- **R3**: 角色宪法 5 字段跟 R11 5 重守门 1:1 镜像怎么验证? — 缓解: 在 D3-4 加 1 个 `r11_mirror_sanity_check` test, 枚举 7 advisor domain → 5 字段必填规则
- **R4**: graph_bridge 集成 (Node 包装) 引入 async + Send+Sync 边界, 跟现有 sync Council 冲突 — 缓解: graph_bridge 是**新** module, 不动现有 deliberation.rs; Node::run 内部走 `tokio::task::spawn_blocking` 包 sync Council
- **R5**: 7h 预算紧 (1h 已用), D3-2 实现 Planner+Executor (最复杂, 借鉴 LangGraph 最深) 可能超时 — 缓解: D3-2 优先 (2h), 简化 Planner 只拆 1 步 (3 步 → 1 步), 3 executor → 2 executor; 留 R26+ 扩

### 8.2 决策权 (per 派活单)

- 4 模式具体实现策略 ✅ 本 readmap §2 (Planner+Executor = 1 planner + 3 executor / Debate = 3 轮复用 R33-4-1 / Voting = 加权简单多数 / Hierarchical = 1 root + 2 sub)
- reasoning trace 格式 ✅ Pretty-print 主 + JSON + JSONL 3 格式
- 角色宪法跟 5 重守门对齐 ✅ 1:1 镜像 (5 字段 struct + 7 advisor domain 默认值)
- 任何破坏硬约束 → 写 `reports/agent-d3-blocked-2026-08-10.md` (per 派活单严守)

### 8.3 不做的事

- ❌ 0 引入 I/O / 网络 / 外部 LLM HTTP (per 现有 council 哲学锚)
- ❌ 0 改 24 LOCKED crate (cognition/core/sovereignty/formal)
- ❌ 0 改 workspace.version (1.1.0)
- ❌ 0 改 7 强制 advisor (R15 锁定)
- ❌ 0 改 deliberation.rs / council_member.rs / hold.rs / lifecycle.rs / synthesis.rs / mock_llm.rs (R10/R15/R16-09/R19/R33-4 LOCKED)
- ❌ 0 主动 commit
- ❌ 0 重复造轮子 (R33-4-1 多轮协商 1:1 复用, 0 重写)

## 9. 阶段产物清单

- [D3-2] `crates/apeireth-council/src/collaboration.rs` (CollaborationMode enum + CollaborationContext + 4 模式 wrapper)
- [D3-2] `crates/apeireth-council/src/collaboration/planner_executor.rs` (Planner + 3 Executor)
- [D3-2] `crates/apeireth-council/tests/collaboration_integration.rs` (5+ tests)
- [D3-3] `crates/apeireth-council/src/collaboration/debate.rs` (复用 R33-4-1, 包 wrapper)
- [D3-3] `crates/apeireth-council/src/collaboration/voting.rs` (加权多数)
- [D3-3] `crates/apeireth-council/src/collaboration/hierarchical.rs` (1 root + 2 sub)
- [D3-3] `crates/apeireth-council/tests/collaboration_modes_integration.rs` (10+ tests)
- [D3-4] `crates/apeireth-council/src/constitution.rs` (RoleConstitution trait + struct + 7 advisor 默认 + 5 字段 1:1 镜像)
- [D3-4] `crates/apeireth-council/tests/constitution_integration.rs` (5+ tests)
- [D3-5] `crates/apeireth-council/src/trace.rs` (TraceStep + TraceReport + 3 输出格式)
- [D3-5] `crates/apeireth-council/examples/trace_visualize.rs` (3 advisor 协作 + trace pretty print)
- [D3-5] `crates/apeireth-council/tests/trace_integration.rs` (5+ tests)
- [D3-5] `crates/apeireth-council/src/graph_orchestration.rs` (4 模式包成 Graph Node, 加图编排支持)
- [D3-6] `reports/agent-d3-final-2026-08-10.md` + `reports/agent-d3-decision-log-2026-08-10.md`

## 10. 开工时间盒

| 阶段 | 时间 | 累计 | 内容 |
|---|---|---|---|
| D3-1 | 0-1h | 1h | 读全 (本 readmap) |
| D3-2 | 1-3h | 3h | collaboration.rs + Planner+Executor + integration |
| D3-3 | 3-4.5h | 4.5h | Debate + Voting + Hierarchical + integration |
| D3-4 | 4.5-5.5h | 5.5h | constitution.rs (1:1 镜像 5 重守门) |
| D3-5 | 5.5-6.5h | 6.5h | trace.rs + example + graph_orchestration + integration |
| D3-6 | 6.5-7h | 7h | final report + decision log |

## 11. D-3 哲学锚 self-check (per 主人偏好)

- ✅ S-1 走在前人经验上: 4 模式 1:1 对标 LangGraph (PlanAndExecute / MessagesState) + AutoGen (GroupChat / ConversableAgent) + VCP (vcpLoop); 角色宪法 1:1 镜像 R11 5 重守门
- ✅ S-2 实事求是: 复用 R33-4-1 0 重写, 0 假装"debate 模式是新写的"
- ✅ O-2 走在前人肩上: 复用 R16-09 LlmAdvisorBackend + R33-4-1 helpers + R33-4-2 helpers
- ✅ O-3 干到底: 4 模式 + 角色宪法 + trace + graph 集成, 0 砍 v2.0 strategy 2B 任何项
- ✅ O-4 任何人都能接手: 4 mode 全部 doc-comment 写明 LangGraph 真代码 + 借鉴点 + 不漂移清单
- ✅ O-5 不假装: 0 假装 graph_orchestration 真跑 langgraph, 0 假装角色宪法已落地 R11 5 重守门, 0 假装 LLM 真接 (复用 R33-4-1 mock 路径)

## 12. 借鉴锚总览

| 本 R 模块 | LangGraph 真代码 | AutoGen 真代码 | VCP 真代码 | 文件级引用 |
|---|---|---|---|---|
| Planner+Executor | `PlanAndExecute.plan_node` + `execute_node` | `GroupChat` planner/executor 角色分离 | `vcpLoop` task decomposition | `langgraph/prebuilt/plan_execute.py` |
| Debate | `MessagesState` 跨轮 state | `GroupChat.max_round` 终止 | `vcpLoop/state` | `langgraph/graph/message_state.py` |
| Voting | `add_conditional_edges` 投票后分支 | `GroupChatManager` 投票 | `vcpLoop/toolCallParser` | `autogen/agentchat/groupchat.py:GroupChatManager` |
| Hierarchical | `subgraph` 子图作为节点 | `GroupChat` 主从 | `vcpLoop` 嵌套 task | `langgraph/graph/graph.py:add_subgraph` |
| 角色宪法 | (无直接对应, 借鉴 Anthropic constitution) | `ConversableAgent.system_message` 约束 | (无直接对应) | 1:1 镜像 R11 5 重守门 (per 派活单) |
| Reasoning trace | `MemorySaver.get_tuple` 状态时间线 | `GroupChat.messages` transcript | `vcpLoop/traceLog` | `langgraph/checkpoint/memory.py` |
| Graph 集成 | `Graph.add_node` + `Graph.execute` | (0 直接对应) | (0 直接对应) | `langgraph/graph/graph.py:Graph` |
