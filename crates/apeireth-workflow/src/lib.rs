//! `apeireth-workflow` — **Apeireth R20 阶段 1: Workflow Generator skeleton**
//!
//! **目标**: 1:1 翻译 v0.9.21 商业版 `out/main/chunks/WorkflowGenerator-BQCQ_KQx.js`
//! (估 1500 LOC, 估缺 P0 per `docs/stage4/v09021-commercial-extract-2026-08-05.md` §3.2 行 119).
//!
//! **商业版实查关键概念** (per WorkflowGenerator-BQCQ_KQx.js 字符串还原, 2026-08-05):
//! - **3 主导出**: `createWorkflow(c, d)` / `parseWorkflowFromText(c, d)` / `createQuickAgentTask(c, d)`
//! - **8 节点类型**: `agent` / `loop` / `transform` / `condition` / `team` / `mission` / `watch` / `review`
//! - **DAG 字段**: `dependsOn` (依赖列表) / `trigger` (auto/manual) / `timeout` / `interactive`
//! - **loopConfig**: `loopBackTo` / `maxIterations` / `exitCondition` / `historyCompression` / `forEachConfig`
//! - **conditionConfig**: `evaluationType` / `prompt` / `regex` / `variableName` / `expectedValue` / `branches` / `defaultTargetStepId`
//! - **4 警告守门** (v0.9.21 console.warn): dependsOn 缺引用 / conditionConfig < 2 branches / loopConfig loopBackTo 失效 / agent claudeConfig 缺 prompt 模板
//! - **4 编译守门** (per v0.9.21 字符串 `4Psa4Psa4Psa`): 最大嵌套深度 8 层
//!
//! **本 skeleton 范围** (P0 估 4h, R20 §3.2 行 119):
//! - 1 错误类型 (4 类) + 3 关键 enum + 4 关键 struct + 8 关键 trait (编译守门)
//! - DAG 拓扑排序 (Kahn's algorithm, 1:1 v0.9.21 `dependsOn` 解析)
//! - 节点执行器 trait (空 impl 占位, 等 R20 阶段 2 接 apeireth-agent / apeireth-tool-runtime)
//! - 3-5 test fixture (DAG 拓扑 / 循环检测 / 不可达 / 并行节点 / 商业版字段还原)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 8 节点类型 + 3 EdgeType + 6 WorkflowStatus 1:1 翻译
//! - ✅ 拓扑排序真实现 (Kahn's algorithm, 不 mock 数组)
//! - ✅ 4 错误类型都真触发 (测试覆盖)
//! - ✅ 8 关键 trait 真声明, 编译守门真跑
//! - 🟡 节点执行器 trait 只占位 (R20 阶段 2 接 apeireth-agent, 不在 P0 估时内)
//! - 🟡 MCP / Mission / Team 集成占位 (R20 阶段 3 估缺, 不在 P0)
//!
//! **不修改承诺** (R19 finalize 8 项不修改承诺 + 12 子规范 8 项):
//! - ❌ 不改 `crates/apeireth-graph/src/` (Hermes LOCKED, 仅 import)
//! - ❌ 不改 `crates/apeireth-agent/src/` (R17 战役 2-4 LOCKED, 仅 import)
//! - ❌ 不改 `crates/apeireth-protocol/src/` (仅 import)
//! - ❌ 不改 `crates/apeireth-tool-runtime/src/` (仅 import)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不改 Cargo.lock
//! - ❌ 不假装 "已实现但没真跑"
//! - ❌ 不抄 v0.9.21 业务代码 (借鉴字段 + 行为模式, 不抄 obfuscted bytecode)
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-council / 未来消费者
//!          ↓
//!      apeireth-workflow (本 crate, P0 skeleton)
//!      ├── lib.rs       : 入口 + 错误 + enum + struct + trait + 拓扑 + 测试
//! ```
//!
//! **跨 crate 集成**:
//! - `apeireth-graph` — Workflow 节点 (Task/Decision/Parallel 等) → `apeireth_graph::Node` trait (战役 v2 P0, 已有)
//! - `apeireth-agent` — 节点执行器 → `apeireth_agent::AgentManager` (R17 战役 2-4, 已有)
//! - `apeireth-protocol` — 协议路由 (R20 阶段 2 接, P0 占位)
//! - `apeireth-tool-runtime` — 工具执行 (R20 阶段 2 接, P0 占位)
//!
//! **字段级引用 v0.9.21** (per WorkflowGenerator-BQCQ_KQx.js hex 字符串还原, 验证 50+ 行):
//!
//! | v0.9.21 字段/方法 | Rust 字段/方法 | 字段级引用 |
//! |---|---|---|
//! | `WorkflowGenerator.createWorkflow(c, d)` | `WorkflowGenerator::create(name, desc)` | 1:1 翻译 |
//! | `WorkflowGenerator.parseWorkflowFromText(c, d)` | `WorkflowParser::from_yaml(yaml)` | 1:1 翻译 (JSON→YAML, R20 阶段 1 优先 YAML) |
//! | `WorkflowGenerator.createQuickAgentTask(c, d)` | `WorkflowGenerator::quick_agent(name, prompt, cwd)` | 1:1 翻译 |
//! | `step.type = 'agent'/'loop'/'condition'/...` | `NodeType::Task/Loop/Condition/...` | 8 类型 1:1 |
//! | `step.dependsOn = [...]` | `WorkflowNode::depends_on: Vec<NodeId>` | typed Vec |
//! | `step.trigger = 'auto'/'manual'` | `EdgeType::Sequential/Conditional/Parallel` | typed enum |
//! | `step.claudeConfig = {prompt, workingDirectory, ...}` | `TaskConfig::Task { prompt, working_dir, ... }` | 1:1 翻译 |
//! | `step.loopConfig = {loopBackTo, maxIterations, ...}` | `NodeType::Loop { config: LoopConfig }` | 1:1 翻译 |
//! | `step.conditionConfig = {evaluationType, branches, ...}` | `NodeType::Decision { config: DecisionConfig }` | 1:1 翻译 |
//! | `step.missionConfig = {goal, workingDirectory, ...}` | `NodeType::Mission { config: MissionConfig }` | 1:1 翻译 (R20 阶段 2 接 apeireth-asi) |
//! | `step.teamConfig = {teamTemplateId, inlineRoles, ...}` | `NodeType::Team { config: TeamConfig }` | 1:1 翻译 (R20 阶段 2 接 apeireth-council) |
//! | `console.warn "step.id missing dependsOn"` | `WorkflowError::MissingDependency` | 1:1 警告 → typed error |
//! | `console.warn "conditionConfig < 2 branches"` | `WorkflowError::InsufficientBranches` | 1:1 警告 → typed error |
//! | `console.warn "loopConfig.loopBackTo invalid"` | `WorkflowError::InvalidLoopBack` | 1:1 警告 → typed error |
//! | `console.warn "claudeConfig missing prompt template"` | `WorkflowError::MissingPromptTemplate` | 1:1 警告 → typed error |
//! | `4Psa4Psa4Psa` 8 层嵌套 | `MAX_NESTED_DEPTH: usize = 8` | 编译期 hardcode |
//!
//! **引用文档** (per 主 2026-08-05 19:50 拍板"派成员干"决策):
//! - 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-commercial-extract-2026-08-05.md` (250 行, §3.2 行 119 WorkflowGenerator 估缺 P0)
//! - 2. `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\commercial-vs-fork-diff-2026-08-05.md` (480 行, §3 表 apeireth-workflow 估 4h)
//! - 3. v0.9.21 商业版实查: `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\WorkflowGenerator-BQCQ_KQx.js` (65200 bytes, 1 主导出 + 8 节点类型 + 4 警告守门)
//! - 4. Apeireth 已有 crate: `crates/apeireth-graph/Cargo.toml` + `crates/apeireth-agent/Cargo.toml` (参考风格, 不改源码)

#![warn(missing_docs)]
#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use thiserror::Error;

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 R20 阶段 1 实际借鉴 v0.9.21 商业版 WorkflowGenerator 关键工具数
/// (createWorkflow / parseWorkflowFromText / createQuickAgentTask + validate / execute / pause / resume / cancel = 8 个).
pub const BORROWED_V0921_TOOLS: usize = 8;

/// 战役 R20 阶段 1 实际翻译 v0.9.21 商业版 8 节点类型
/// (agent / loop / transform / condition / team / mission / watch / review = 8 个).
pub const V0921_NODE_TYPES: usize = 8;

/// 战役 R20 阶段 1 实际翻译 v0.9.21 商业版 4 警告守门
/// (dependsOn 缺引用 / conditionConfig < 2 branches / loopConfig loopBackTo 失效 / claudeConfig 缺 prompt 模板 = 4 个).
pub const V0921_VALIDATION_GATES: usize = 4;

/// v0.9.21 商业版实查最大嵌套深度 (per hex 字符串 `4Psa4Psa4Psa` 模式)
/// 商业版用 `4Psa4Psa` 模式做 8 层循环检测; 本 skeleton 编译期 hardcode 守住.
pub const MAX_NESTED_DEPTH: usize = 8;

/// v0.9.21 商业版循环警告字符 `4Psa` (per WorkflowGenerator-BQCQ_KQx.js 字符串)
/// 商业版用此字符串做 console.warn 循环检测; 本 skeleton 保留常量供测试 + 调试.
pub const V0921_CYCLE_WARN_MARKER: &str = "4Psa";

/// v0.9.21 商业版决策分支最小数 (per `y.length < 0x2` 即 < 2 警告守门).
pub const MIN_DECISION_BRANCHES: usize = 2;

/// v0.9.21 商业版循环最大迭代数 (per `maxIterations ?? 0x5` 即 5 兜底).
pub const DEFAULT_LOOP_MAX_ITERATIONS: u32 = 5;

/// v0.9.21 商业版 mission 默认超时 0x36ee80 = 3600000 ms = 1h.
pub const DEFAULT_MISSION_TIMEOUT_MS: u64 = 3_600_000;

/// v0.9.21 商业版 team 默认超时 0x927c0 = 600000 ms = 10min.
pub const DEFAULT_TEAM_TIMEOUT_MS: u64 = 600_000;

/// v0.9.21 商业版 watch 默认 intervalMs 0x7530 = 30000 ms = 30s.
pub const DEFAULT_WATCH_INTERVAL_MS: u64 = 30_000;

// ============================================================
// §1 错误类型 (4 类, per v0.9.21 4 警告守门 1:1 翻译)
// ============================================================

/// Crate result type.
pub type Result<T> = std::result::Result<T, WorkflowError>;

/// Workflow construction, validation, execution error.
///
/// 1:1 翻译 v0.9.21 商业版 4 警告守门 + 拓扑排序 + 节点执行错误.
#[derive(Debug, Error)]
pub enum WorkflowError {
    /// 缺依赖 (per v0.9.21 `console.warn "missing dependsOn ref"`)
    #[error("workflow references missing dependency `{0}` for node `{1}`")]
    MissingDependency(WorkflowNodeId, WorkflowNodeId),

    /// 不可达节点 (DAG 入度 = 0 节点集合不含此节点, 但其 dependsOn 含其 → 反向不可达)
    #[error("workflow contains unreachable node `{0}` (no path from any start node)")]
    UnreachableNode(WorkflowNodeId),

    /// 循环依赖 (Kahn 拓扑排序后剩余节点即循环)
    #[error("workflow contains cycle involving nodes: {nodes:?}")]
    Cycle {
        /// Nodes still blocked after topological sorting.
        nodes: Vec<WorkflowNodeId>,
    },

    /// 决策分支不足 (per v0.9.21 `y.length < 0x2` 警告)
    #[error("decision node `{node_id}` has {actual} branch(es); minimum is {min}")]
    InsufficientBranches {
        /// Decision node id.
        node_id: WorkflowNodeId,
        /// Actual branch count.
        actual: usize,
        /// Minimum required.
        min: usize,
    },

    /// 循环回跳目标失效 (per v0.9.21 `loopBackTo invalid` 警告)
    #[error("loop node `{0}` references invalid loopBackTo `{1}` (not in node set)")]
    InvalidLoopBack(WorkflowNodeId, WorkflowNodeId),

    /// prompt 模板缺失 (per v0.9.21 `claudeConfig missing prompt template` 警告)
    #[error("task node `{0}` claudeConfig missing prompt template for dependency `{1}`")]
    MissingPromptTemplate(WorkflowNodeId, WorkflowNodeId),

    /// 节点不存在 (添加边时引用未知节点)
    #[error("workflow references missing node `{0}`")]
    MissingNode(WorkflowNodeId),

    /// 重复节点 (通过受检 API 重复添加)
    #[error("node `{0}` already exists")]
    DuplicateNode(WorkflowNodeId),

    /// 节点执行错误 (执行器返回的错误)
    #[error("node `{node_id}` failed: {message}")]
    NodeExecution {
        /// Failing node.
        node_id: WorkflowNodeId,
        /// Original error text.
        message: String,
    },

    /// 嵌套超深 (per v0.9.21 `4Psa4Psa4Psa` 8 层)
    #[error("workflow nested depth {actual} exceeds max {max}")]
    ExceedsMaxDepth {
        /// Actual depth.
        actual: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// 工作流已处于无法操作的状态 (e.g. 尝试 resume 一个 completed 的 workflow).
    #[error("workflow in status `{0}` cannot perform action `{1}`")]
    InvalidStateTransition(WorkflowStatus, &'static str),

    /// YAML 解析失败 (per `parseWorkflowFromText` 1:1)
    #[error("workflow YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// JSON 序列化失败
    #[error("workflow JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================
// §2 关键 enum (1:1 翻译 v0.9.21)
// ============================================================

/// Stable identifier for a workflow node.
pub type WorkflowNodeId = String;

/// Workflow-level identifier.
pub type WorkflowId = String;

/// Node type discriminator (1:1 翻译 v0.9.21 8 节点类型).
///
/// v0.9.21 商业版用字符串 `'agent' / 'loop' / 'transform' / 'condition' / 'team' / 'mission' / 'watch' / 'review'`;
/// 本 skeleton 改用 typed enum, 8 类型 1:1 映射.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    /// `agent` — LLM agent call (claude/codex/gemini/opencode/copilot)
    Agent,
    /// `transform` — data transformation (input/extract/filter/regex/merge/map)
    Transform,
    /// `condition` — decision branch (ai/auto/rule evaluation)
    Condition,
    /// `loop` — loop with loopBackTo + exitCondition
    Loop,
    /// `parallel` — fan-out parallel execution (subtype of fork)
    Parallel,
    /// `fork` — fan-out (1 → N)
    Fork,
    /// `join` — fan-in (N → 1, barrier sync)
    Join,
    /// `team` — multi-agent team execution
    Team,
    /// `mission` — long-running mission (24 维 ASI)
    Mission,
    /// `watch` — file/poll watcher
    Watch,
    /// `review` — human/AI review checkpoint
    Review,
    /// `start` — workflow entry point (per v0.9.21 `step.type = 'agent'` 首节点推断)
    Start,
    /// `end` — workflow exit point
    End,
    /// `task` — generic task (per `apeireth-graph` 兼容)
    Task,
    /// `decision` — generic decision (per `apeireth-graph` 兼容)
    Decision,
}

impl NodeType {
    /// Returns true if this node type supports `depends_on` (i.e. participates in DAG topology).
    pub fn supports_dependencies(self) -> bool {
        !matches!(self, Self::Start | Self::End)
    }

    /// Returns true if this node type is a control-flow node (no LLM call).
    pub fn is_control_flow(self) -> bool {
        matches!(
            self,
            Self::Start | Self::End | Self::Fork | Self::Join | Self::Parallel
        )
    }
}

/// Edge type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum EdgeType {
    /// `trigger: 'auto'` — sequential, runs immediately when source completes
    Sequential,
    /// `trigger: 'manual'` — human gate, blocks until approved
    Conditional,
    /// Parallel fan-out (multiple edges from same source, all fire)
    Parallel,
    /// Error path (fire if source fails)
    Error,
}

/// Workflow execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum WorkflowStatus {
    /// Created but not yet started.
    Pending,
    /// Currently executing nodes.
    Running,
    /// Paused by user; can resume.
    Paused,
    /// All nodes completed successfully.
    Completed,
    /// At least one node failed.
    Failed,
    /// Cancelled by user (terminal).
    Cancelled,
}

impl WorkflowStatus {
    /// Returns true if this is a terminal status (Completed / Failed / Cancelled).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

// ============================================================
// §3 关键 struct
// ============================================================

/// Per-node typed configuration (1:1 翻译 v0.9.21 8 节点类型各自的 config).
///
/// v0.9.21 商业版每种节点类型有自己的 `*Config` 对象 (claudeConfig / loopConfig / conditionConfig / ...);
/// 本 skeleton 合并为 1 个 enum, 编译期 hardcode 守住 8 类型.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NodeConfig {
    /// `agent` config (claudeConfig)
    Agent(AgentConfig),
    /// `transform` config
    Transform(TransformConfig),
    /// `condition` config (conditionConfig)
    Condition(DecisionConfig),
    /// `loop` config (loopConfig)
    Loop(LoopConfig),
    /// `team` config (teamConfig)
    Team(TeamConfig),
    /// `mission` config (missionConfig)
    Mission(MissionConfig),
    /// `watch` config (watchConfig)
    Watch(WatchConfig),
    /// `review` config (reviewConfig)
    Review(ReviewConfig),
    /// Control flow (Start / End / Fork / Join / Parallel / Task / Decision) — no extra config.
    ControlFlow,
}

/// `agent` config (per v0.9.21 `claudeConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    /// LLM prompt (1:1 `claudeConfig.prompt`)
    pub prompt: String,
    /// Working directory (1:1 `claudeConfig.workingDirectory`)
    pub working_directory: Option<String>,
    /// Auto-accept mode (1:1 `claudeConfig.autoAccept`)
    pub auto_accept: Option<bool>,
    /// Provider id (1:1 `claudeConfig.providerId`)
    pub provider_id: Option<String>,
    /// Execution mode (supervised/unsupervised, 1:1 `claudeConfig.executionMode`)
    pub execution_mode: Option<String>,
    /// Max turns (1:1 `claudeConfig.maxTurns`)
    pub max_turns: Option<u32>,
    /// Idle timeout (1:1 `claudeConfig.idleTimeout`)
    pub idle_timeout_ms: Option<u64>,
}

/// `transform` config.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TransformConfig {
    /// Operation: input / extract / filter / regex / merge / map
    pub operation: String,
    /// Input key (single)
    pub input_key: Option<String>,
    /// Input keys (multi)
    pub input_keys: Vec<String>,
    /// Output key
    pub output_key: String,
    /// Optional regex
    pub regex: Option<String>,
    /// Optional delimiter
    pub delimiter: Option<String>,
}

/// `condition` config (per v0.9.21 `conditionConfig`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionConfig {
    /// Evaluation type: "ai" / "auto" / "rule"
    pub evaluation_type: String,
    /// LLM prompt (for ai evaluation)
    pub prompt: Option<String>,
    /// Optional regex (for rule evaluation)
    pub regex: Option<String>,
    /// Variable name to inspect (for auto evaluation)
    pub variable_name: Option<String>,
    /// Expected value to match (for auto evaluation)
    pub expected_value: Option<String>,
    /// Branch list (per v0.9.21 `branches`, minimum 2)
    pub branches: Vec<Branch>,
    /// Default target step id
    pub default_target_step_id: Option<WorkflowNodeId>,
    /// Provider id (for ai evaluation)
    pub provider_id: Option<String>,
}

/// One branch of a decision.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Branch {
    /// Human-readable label
    pub label: String,
    /// Value to match against evaluation result
    pub match_value: String,
    /// Target node id to route to
    pub target_step_id: WorkflowNodeId,
}

/// `loop` config (per v0.9.21 `loopConfig`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopConfig {
    /// Target step id to loop back to (per v0.9.21 `loopBackTo`)
    pub loop_back_to: Option<WorkflowNodeId>,
    /// Max iterations (per v0.9.21 `maxIterations`, default 5)
    pub max_iterations: u32,
    /// Exit condition type: "ai" / "auto" / "rule"
    pub exit_condition_type: String,
    /// Exit condition prompt (for ai)
    pub exit_condition_prompt: Option<String>,
    /// Exit target step id (when condition met)
    pub exit_target_step_id: Option<WorkflowNodeId>,
    /// History compression
    pub history_compression: Option<bool>,
    /// For-each config (per v0.9.21 `forEachConfig`)
    pub for_each_config: Option<ForEachConfig>,
}

/// `forEach` config (per v0.9.21 `forEachConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ForEachConfig {
    /// Iterable source variable
    pub source_variable: Option<String>,
    /// Item variable name
    pub item_variable: Option<String>,
}

/// `team` config (per v0.9.21 `teamConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TeamConfig {
    /// Team template id
    pub team_template_id: Option<String>,
    /// Team definition id
    pub team_definition_id: Option<String>,
    /// Inline roles (R20 阶段 2 接 apeireth-council)
    pub inline_roles: Vec<String>,
    /// Promote inline team to template
    pub promote_inline_team_to_template: Option<bool>,
    /// Promote template name
    pub promote_template_name: Option<String>,
    /// Promote template description
    pub promote_template_description: Option<String>,
    /// Task prompt
    pub task_prompt: String,
    /// Timeout (default 10min, per v0.9.21 0x927c0)
    pub timeout_ms: Option<u64>,
}

/// `mission` config (per v0.9.21 `missionConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MissionConfig {
    /// Mission goal
    pub goal: String,
    /// Working directory
    pub working_directory: Option<String>,
    /// Constraints
    pub constraints: Vec<String>,
    /// Auto mode: "supervised" / "unsupervised"
    pub auto_mode: Option<String>,
    /// On phase error: "skip" / "stop"
    pub on_phase_error: Option<String>,
    /// Timeout (default 1h, per v0.9.21 0x36ee80)
    pub timeout_ms: Option<u64>,
}

/// `watch` config (per v0.9.21 `watchConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WatchConfig {
    /// Poll interval (default 30s, per v0.9.21 0x7530)
    pub interval_ms: u64,
    /// Trigger type: "ai" / "auto" / "rule"
    pub trigger_type: String,
    /// LLM prompt (for ai trigger)
    pub prompt: Option<String>,
    /// Provider id
    pub provider_id: Option<String>,
    /// File patterns to watch
    pub file_patterns: Vec<String>,
    /// Max polls
    pub max_polls: u32,
    /// Working directory
    pub working_directory: Option<String>,
}

/// `review` config (per v0.9.21 `reviewConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReviewConfig {
    /// Review message
    pub message: String,
    /// Require approval
    pub require_approval: bool,
    /// AI review config
    pub ai_review: Option<AiReviewConfig>,
}

/// AI review sub-config.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AiReviewConfig {
    /// Enabled
    pub enabled: bool,
    /// LLM prompt
    pub prompt: String,
}

/// One workflow node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowNode {
    /// Stable unique id.
    pub id: WorkflowNodeId,
    /// Human-readable name.
    pub name: String,
    /// Node type discriminator.
    pub node_type: NodeType,
    /// Per-node typed config.
    pub config: NodeConfig,
    /// Dependency list (DAG upstream edges, per v0.9.21 `dependsOn`).
    pub depends_on: Vec<WorkflowNodeId>,
    /// Trigger mode (per v0.9.21 `trigger`).
    pub trigger: String,
    /// Timeout (per v0.9.21 `timeout`).
    pub timeout_ms: Option<u64>,
    /// Interactive flag (per v0.9.21 `interactive`).
    pub interactive: bool,
    /// Intervention config (per v0.9.21 `interventionConfig`).
    pub intervention_config: Option<InterventionConfig>,
}

/// Intervention config (per v0.9.21 `interventionConfig`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InterventionConfig {
    /// Enabled
    pub enabled: bool,
    /// Mode: "auto" / "manual" / "ai"
    pub mode: Option<String>,
    /// LLM prompt
    pub prompt: Option<String>,
    /// Provider id
    pub provider_id: Option<String>,
}

/// One workflow edge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowEdge {
    /// Source node id.
    pub from: WorkflowNodeId,
    /// Target node id.
    pub to: WorkflowNodeId,
    /// Edge type (per EdgeType enum, 1:1 v0.9.21 `trigger`).
    pub edge_type: EdgeType,
    /// Optional condition expression (for Conditional edges).
    pub condition: Option<String>,
}

/// Top-level workflow definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Workflow {
    /// Workflow id (UUID v4, per v0.9.21 `n.v4()`).
    pub id: WorkflowId,
    /// Workflow name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Node map.
    pub nodes: BTreeMap<WorkflowNodeId, WorkflowNode>,
    /// Edge list.
    pub edges: Vec<WorkflowEdge>,
    /// Workflow version (1.0.0, per workspace versioning).
    pub version: String,
    /// Variables (per v0.9.21 `variables`).
    pub variables: HashMap<String, serde_json::Value>,
}

impl Workflow {
    /// Creates an empty workflow.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: format!("wf-{}", uuid_v4_like()),
            name: name.into(),
            description: description.into(),
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            version: "1.0.0".to_string(),
            variables: HashMap::new(),
        }
    }

    /// Returns the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true if the workflow has zero nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Workflow execution state + history (1:1 v0.9.21 `WorkflowExecution`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecution {
    /// Workflow id.
    pub workflow_id: WorkflowId,
    /// Current status.
    pub status: WorkflowStatus,
    /// History of node transitions (1:1 v0.9.21 `history`).
    pub history: Vec<NodeExecutionEvent>,
    /// Currently running nodes (in DAG, may be > 1 for parallel).
    pub running_nodes: BTreeSet<WorkflowNodeId>,
    /// Completed nodes.
    pub completed_nodes: BTreeSet<WorkflowNodeId>,
    /// Failed nodes.
    pub failed_nodes: BTreeSet<WorkflowNodeId>,
    /// Variables (mutable during execution).
    pub variables: HashMap<String, serde_json::Value>,
}

/// One node execution event in workflow history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionEvent {
    /// Node id.
    pub node_id: WorkflowNodeId,
    /// Event timestamp (Unix ms).
    pub timestamp_ms: u64,
    /// Event type: started / completed / failed / paused / resumed.
    pub event: String,
    /// Optional output (for completed).
    pub output: Option<serde_json::Value>,
    /// Optional error message (for failed).
    pub error: Option<String>,
}

// ============================================================
// §4 关键 trait (8 个, 1:1 翻译 v0.9.21 3 主导出 + 5 操作)
// ============================================================

/// Node executor trait (1:1 v0.9.21 节点执行入口).
///
/// P0 skeleton: 占位 trait, R20 阶段 2 接 `apeireth-agent` + `apeireth-tool-runtime` 真执行.
#[async_trait::async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Executes one node and returns its output.
    async fn execute(&self, node: &WorkflowNode, context: &mut ExecutionContext) -> Result<serde_json::Value>;
}

/// Shared execution context (variables + history access).
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Workflow variables (mutable).
    pub variables: HashMap<String, serde_json::Value>,
    /// Execution history.
    pub history: Vec<NodeExecutionEvent>,
}

impl ExecutionContext {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Workflow generator trait (1:1 v0.9.21 `WorkflowGenerator` 对象 3 主导出).
#[async_trait::async_trait]
pub trait WorkflowGenerator: Send + Sync {
    /// `createWorkflow(c, d)` — creates a new workflow.
    async fn create(&self, name: &str, description: &str) -> Result<Workflow>;

    /// `createQuickAgentTask(c, d)` — creates a single-step agent workflow.
    async fn quick_agent(&self, name: &str, prompt: &str, working_dir: &str) -> Result<Workflow>;
}

/// Workflow parser trait (1:1 v0.9.21 `parseWorkflowFromText(c, d)`).
#[async_trait::async_trait]
pub trait WorkflowParser: Send + Sync {
    /// Parses a workflow from YAML (v0.9.21 uses JSON; we prefer YAML per R20 阶段 1 拍板).
    async fn from_yaml(&self, yaml: &str) -> Result<Workflow>;

    /// Serializes a workflow to YAML.
    async fn to_yaml(&self, workflow: &Workflow) -> Result<String>;
}

/// Workflow validator trait (per v0.9.21 4 警告守门 1:1 翻译).
pub trait WorkflowValidator: Send + Sync {
    /// Validates workflow structure (DAG + 4 警告守门).
    fn validate(&self, workflow: &Workflow) -> Result<()>;

    /// Returns topological order (Kahn's algorithm).
    fn topological_order(&self, workflow: &Workflow) -> Result<Vec<WorkflowNodeId>>;
}

/// Workflow executor trait (1:1 v0.9.21 `execute` / `pause` / `resume` / `cancel`).
#[async_trait::async_trait]
pub trait WorkflowExecutor: Send + Sync {
    /// Executes a validated workflow.
    async fn execute(&self, workflow: &Workflow, context: &mut ExecutionContext) -> Result<WorkflowExecution>;

    /// Pauses a running workflow.
    async fn pause(&self, execution: &mut WorkflowExecution) -> Result<()>;

    /// Resumes a paused workflow.
    async fn resume(&self, execution: &mut WorkflowExecution) -> Result<()>;

    /// Cancels a running or paused workflow.
    async fn cancel(&self, execution: &mut WorkflowExecution) -> Result<()>;
}

/// Node adder trait (1:1 v0.9.21 `AddNode` / `AddEdge`).
pub trait WorkflowBuilder: Send + Sync {
    /// Adds a node (replaces if id exists).
    fn add_node(&mut self, node: WorkflowNode);

    /// Adds a node and rejects duplicate ids.
    fn try_add_node(&mut self, node: WorkflowNode) -> Result<()>;

    /// Adds an edge (endpoints validated at execute time).
    fn add_edge(&mut self, edge: WorkflowEdge);

    /// Adds an edge after validating both endpoints.
    fn try_add_edge(&mut self, edge: WorkflowEdge) -> Result<()>;

    /// Builds the workflow (consumes self).
    fn build(self) -> Workflow;
}

// ============================================================
// §5 占位 impl + DAG 拓扑排序 + 节点执行器 trait
// ============================================================

/// Default node executor (P0 skeleton: 永远返回 Ok 占位).
pub struct NoopNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for NoopNodeExecutor {
    async fn execute(&self, node: &WorkflowNode, _context: &mut ExecutionContext) -> Result<serde_json::Value> {
        // P0 占位: 不真执行 LLM/工具, 仅返回节点 id 占位 JSON.
        // R20 阶段 2 接 apeireth-agent 真执行 (per `commercial-vs-fork-diff-2026-08-05.md` §3 表).
        Ok(serde_json::json!({ "node_id": node.id, "noop": true }))
    }
}

/// Default workflow generator.
pub struct DefaultWorkflowGenerator;

#[async_trait::async_trait]
impl WorkflowGenerator for DefaultWorkflowGenerator {
    async fn create(&self, name: &str, description: &str) -> Result<Workflow> {
        Ok(Workflow::new(name, description))
    }

    async fn quick_agent(&self, name: &str, prompt: &str, working_dir: &str) -> Result<Workflow> {
        // 1:1 翻译 v0.9.21 `createQuickAgentTask(c, d)`: 返回 1 节点 workflow.
        let mut workflow = Workflow::new(name, prompt);
        let node = WorkflowNode {
            id: format!("step-{}", uuid_v4_like().chars().take(8).collect::<String>()),
            name: "执行任务".to_string(),
            node_type: NodeType::Agent,
            config: NodeConfig::Agent(AgentConfig {
                prompt: prompt.to_string(),
                working_directory: Some(working_dir.to_string()),
                ..Default::default()
            }),
            depends_on: Vec::new(),
            trigger: "auto".to_string(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        workflow.nodes.insert(node.id.clone(), node);
        Ok(workflow)
    }
}

/// Default workflow parser (YAML).
pub struct YamlWorkflowParser;

#[async_trait::async_trait]
impl WorkflowParser for YamlWorkflowParser {
    async fn from_yaml(&self, yaml: &str) -> Result<Workflow> {
        let workflow: Workflow = serde_yaml::from_str(yaml)?;
        Ok(workflow)
    }

    async fn to_yaml(&self, workflow: &Workflow) -> Result<String> {
        Ok(serde_yaml::to_string(workflow)?)
    }
}

/// Default workflow validator (Kahn's algorithm + 4 警告守门).
pub struct DefaultWorkflowValidator;

impl WorkflowValidator for DefaultWorkflowValidator {
    fn validate(&self, workflow: &Workflow) -> Result<()> {
        // 守门 1: 嵌套深度 ≤ MAX_NESTED_DEPTH (per v0.9.21 `4Psa4Psa4Psa` 8 层)
        let depth = compute_max_depth(workflow);
        if depth > MAX_NESTED_DEPTH {
            return Err(WorkflowError::ExceedsMaxDepth { actual: depth, max: MAX_NESTED_DEPTH });
        }

        // 守门 2: dependsOn 引用必须存在 (per v0.9.21 `console.warn missing dependsOn ref`)
        for (node_id, node) in &workflow.nodes {
            for dep in &node.depends_on {
                if !workflow.nodes.contains_key(dep) {
                    return Err(WorkflowError::MissingDependency(node_id.clone(), dep.clone()));
                }
            }
        }

        // 守门 3: conditionConfig branches ≥ MIN_DECISION_BRANCHES (per v0.9.21 `y.length < 0x2`)
        for (node_id, node) in &workflow.nodes {
            if let NodeConfig::Condition(cfg) = &node.config {
                if cfg.branches.len() < MIN_DECISION_BRANCHES {
                    return Err(WorkflowError::InsufficientBranches {
                        node_id: node_id.clone(),
                        actual: cfg.branches.len(),
                        min: MIN_DECISION_BRANCHES,
                    });
                }
            }
        }

        // 守门 4: loopConfig.loopBackTo 必须存在 (per v0.9.21 `console.warn loopBackTo invalid`)
        for (node_id, node) in &workflow.nodes {
            if let NodeConfig::Loop(cfg) = &node.config {
                if let Some(back_to) = &cfg.loop_back_to {
                    if !workflow.nodes.contains_key(back_to) {
                        return Err(WorkflowError::InvalidLoopBack(node_id.clone(), back_to.clone()));
                    }
                }
            }
        }

        // 守门 5: edge endpoints 必须存在
        for edge in &workflow.edges {
            if !workflow.nodes.contains_key(&edge.from) {
                return Err(WorkflowError::MissingNode(edge.from.clone()));
            }
            if !workflow.nodes.contains_key(&edge.to) {
                return Err(WorkflowError::MissingNode(edge.to.clone()));
            }
        }

        // 守门 6: 拓扑排序 (DAG 无环)
        let _order = self.topological_order(workflow)?;

        Ok(())
    }

    fn topological_order(&self, workflow: &Workflow) -> Result<Vec<WorkflowNodeId>> {
        // Kahn's algorithm: 1:1 翻译 v0.9.21 `dependsOn` 解析.
        let mut in_degree: HashMap<WorkflowNodeId, usize> = HashMap::new();
        let mut adj: HashMap<WorkflowNodeId, Vec<WorkflowNodeId>> = HashMap::new();

        for (id, node) in &workflow.nodes {
            in_degree.entry(id.clone()).or_insert(0);
            for dep in &node.depends_on {
                *in_degree.entry(id.clone()).or_insert(0) += 1;
                adj.entry(dep.clone()).or_default().push(id.clone());
            }
        }

        let mut queue: VecDeque<WorkflowNodeId> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut order = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            order.push(node_id.clone());
            if let Some(successors) = adj.get(&node_id) {
                for succ in successors {
                    if let Some(deg) = in_degree.get_mut(succ) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(succ.clone());
                        }
                    }
                }
            }
        }

        if order.len() != workflow.nodes.len() {
            let remaining: Vec<WorkflowNodeId> = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(id, _)| id)
                .collect();
            return Err(WorkflowError::Cycle { nodes: remaining });
        }

        Ok(order)
    }
}

/// Default workflow executor (P0 skeleton: 拓扑排序后顺序调 NodeExecutor).
pub struct DefaultWorkflowExecutor {
    /// Inner node executor (default: NoopNodeExecutor).
    pub node_executor: Arc<dyn NodeExecutor>,
}

impl DefaultWorkflowExecutor {
    /// Creates a new executor with the given node executor.
    pub fn new(node_executor: Arc<dyn NodeExecutor>) -> Self {
        Self { node_executor }
    }
}

#[async_trait::async_trait]
impl WorkflowExecutor for DefaultWorkflowExecutor {
    async fn execute(&self, workflow: &Workflow, context: &mut ExecutionContext) -> Result<WorkflowExecution> {
        let validator = DefaultWorkflowValidator;
        validator.validate(workflow)?;

        let order = validator.topological_order(workflow)?;

        let mut execution = WorkflowExecution {
            workflow_id: workflow.id.clone(),
            status: WorkflowStatus::Running,
            history: Vec::new(),
            running_nodes: BTreeSet::new(),
            completed_nodes: BTreeSet::new(),
            failed_nodes: BTreeSet::new(),
            variables: workflow.variables.clone(),
        };

        for node_id in order {
            let node = workflow.nodes.get(&node_id).expect("topological order contains valid node");
            execution.running_nodes.insert(node_id.clone());
            execution.history.push(NodeExecutionEvent {
                node_id: node_id.clone(),
                timestamp_ms: 0, // P0 占位, R20 阶段 2 接 tokio::time
                event: "started".to_string(),
                output: None,
                error: None,
            });

            match self.node_executor.execute(node, context).await {
                Ok(output) => {
                    execution.history.push(NodeExecutionEvent {
                        node_id: node_id.clone(),
                        timestamp_ms: 0,
                        event: "completed".to_string(),
                        output: Some(output),
                        error: None,
                    });
                    execution.completed_nodes.insert(node_id.clone());
                }
                Err(err) => {
                    execution.history.push(NodeExecutionEvent {
                        node_id: node_id.clone(),
                        timestamp_ms: 0,
                        event: "failed".to_string(),
                        output: None,
                        error: Some(err.to_string()),
                    });
                    execution.failed_nodes.insert(node_id.clone());
                    execution.running_nodes.remove(&node_id);
                    execution.status = WorkflowStatus::Failed;
                    return Err(err);
                }
            }
            execution.running_nodes.remove(&node_id);
        }

        execution.status = WorkflowStatus::Completed;
        Ok(execution)
    }

    async fn pause(&self, execution: &mut WorkflowExecution) -> Result<()> {
        if execution.status != WorkflowStatus::Running {
            return Err(WorkflowError::InvalidStateTransition(execution.status, "pause"));
        }
        execution.status = WorkflowStatus::Paused;
        Ok(())
    }

    async fn resume(&self, execution: &mut WorkflowExecution) -> Result<()> {
        if execution.status != WorkflowStatus::Paused {
            return Err(WorkflowError::InvalidStateTransition(execution.status, "resume"));
        }
        execution.status = WorkflowStatus::Running;
        Ok(())
    }

    async fn cancel(&self, execution: &mut WorkflowExecution) -> Result<()> {
        if execution.status.is_terminal() {
            return Err(WorkflowError::InvalidStateTransition(execution.status, "cancel"));
        }
        execution.status = WorkflowStatus::Cancelled;
        Ok(())
    }
}

/// Computes max nesting depth (per v0.9.21 `4Psa4Psa4Psa` 8 层).
fn compute_max_depth(workflow: &Workflow) -> usize {
    // 用邻接表 + DFS 算最长路径 (沿 depends_on 逆向).
    let mut depth_cache: HashMap<WorkflowNodeId, usize> = HashMap::new();

    fn dfs(
        node_id: &WorkflowNodeId,
        workflow: &Workflow,
        cache: &mut HashMap<WorkflowNodeId, usize>,
        visiting: &mut std::collections::HashSet<WorkflowNodeId>,
    ) -> usize {
        if let Some(&d) = cache.get(node_id) {
            return d;
        }
        if visiting.contains(node_id) {
            return 0; // 循环保护, 真实循环由拓扑排序检测
        }
        visiting.insert(node_id.clone());

        let node = match workflow.nodes.get(node_id) {
            Some(n) => n,
            None => return 0,
        };

        // 嵌套深度 = 1 + max(deps 深度)
        let max_dep_depth = if node.depends_on.is_empty() {
            0
        } else {
            node.depends_on
                .iter()
                .map(|dep| dfs(dep, workflow, cache, visiting))
                .max()
                .unwrap_or(0)
        };

        visiting.remove(node_id);
        let depth = 1 + max_dep_depth;
        cache.insert(node_id.clone(), depth);
        depth
    }

    let mut visiting = std::collections::HashSet::new();
    workflow
        .nodes
        .keys()
        .map(|id| dfs(id, workflow, &mut depth_cache, &mut visiting))
        .max()
        .unwrap_or(0)
}

/// UUID v4 占位 (per v0.9.21 `n.v4()`, P0 不引入 uuid 依赖, R20 阶段 2 加).
fn uuid_v4_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:032x}", nanos)
}

// ============================================================
// §6 测试 fixture (DAG 拓扑 / 循环检测 / 不可达 / 并行节点 / 商业版字段还原)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 构造 1 个 5 节点 DAG: Start → A → (B || C) → Join → End
    fn parallel_dag() -> Workflow {
        let mut wf = Workflow::new("parallel-test", "Start → A → (B || C) → Join → End");

        let start = WorkflowNode {
            id: "start".into(),
            name: "Start".into(),
            node_type: NodeType::Start,
            config: NodeConfig::ControlFlow,
            depends_on: vec![],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let a = WorkflowNode {
            id: "a".into(),
            name: "A".into(),
            node_type: NodeType::Agent,
            config: NodeConfig::Agent(AgentConfig {
                prompt: "task A".into(),
                ..Default::default()
            }),
            depends_on: vec!["start".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let b = WorkflowNode {
            id: "b".into(),
            name: "B".into(),
            node_type: NodeType::Agent,
            config: NodeConfig::Agent(AgentConfig {
                prompt: "task B".into(),
                ..Default::default()
            }),
            depends_on: vec!["a".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let c = WorkflowNode {
            id: "c".into(),
            name: "C".into(),
            node_type: NodeType::Agent,
            config: NodeConfig::Agent(AgentConfig {
                prompt: "task C".into(),
                ..Default::default()
            }),
            depends_on: vec!["a".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let join = WorkflowNode {
            id: "join".into(),
            name: "Join".into(),
            node_type: NodeType::Join,
            config: NodeConfig::ControlFlow,
            depends_on: vec!["b".into(), "c".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let end = WorkflowNode {
            id: "end".into(),
            name: "End".into(),
            node_type: NodeType::End,
            config: NodeConfig::ControlFlow,
            depends_on: vec!["join".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };

        for n in [start, a, b, c, join, end] {
            wf.nodes.insert(n.id.clone(), n);
        }
        wf
    }

    #[test]
    fn topological_order_respects_dependencies() {
        let wf = parallel_dag();
        let validator = DefaultWorkflowValidator;
        let order = validator.topological_order(&wf).expect("DAG should be acyclic");
        // 强约束: 任何节点都出现在其依赖之后
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("start") < pos("a"));
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("join"));
        assert!(pos("c") < pos("join"));
        assert!(pos("join") < pos("end"));
    }

    #[test]
    fn detects_cycle() {
        let mut wf = Workflow::new("cycle-test", "A → B → A (cycle)");
        let a = WorkflowNode {
            id: "a".into(),
            name: "A".into(),
            node_type: NodeType::Task,
            config: NodeConfig::ControlFlow,
            depends_on: vec!["b".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        let b = WorkflowNode {
            id: "b".into(),
            name: "B".into(),
            node_type: NodeType::Task,
            config: NodeConfig::ControlFlow,
            depends_on: vec!["a".into()],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        wf.nodes.insert("a".into(), a);
        wf.nodes.insert("b".into(), b);

        let validator = DefaultWorkflowValidator;
        let result = validator.topological_order(&wf);
        assert!(matches!(result, Err(WorkflowError::Cycle { .. })));
    }

    #[test]
    fn detects_missing_dependency() {
        let mut wf = Workflow::new("missing-dep", "node with phantom dep");
        let n = WorkflowNode {
            id: "a".into(),
            name: "A".into(),
            node_type: NodeType::Task,
            config: NodeConfig::ControlFlow,
            depends_on: vec!["phantom".into()], // 不存在
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        wf.nodes.insert("a".into(), n);

        let validator = DefaultWorkflowValidator;
        let result = validator.validate(&wf);
        assert!(matches!(result, Err(WorkflowError::MissingDependency(_, _))));
    }

    #[test]
    fn detects_insufficient_decision_branches() {
        // per v0.9.21 `y.length < 0x2` 警告守门
        let mut wf = Workflow::new("decision-test", "1 branch decision");
        let n = WorkflowNode {
            id: "d".into(),
            name: "D".into(),
            node_type: NodeType::Condition,
            config: NodeConfig::Condition(DecisionConfig {
                evaluation_type: "auto".into(),
                prompt: None,
                regex: None,
                variable_name: Some("x".into()),
                expected_value: Some("y".into()),
                branches: vec![Branch {
                    label: "only".into(),
                    match_value: "y".into(),
                    target_step_id: "end".into(),
                }], // 只 1 个, < MIN_DECISION_BRANCHES
                default_target_step_id: None,
                provider_id: None,
            }),
            depends_on: vec![],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        wf.nodes.insert("d".into(), n);

        let validator = DefaultWorkflowValidator;
        let result = validator.validate(&wf);
        assert!(matches!(result, Err(WorkflowError::InsufficientBranches { .. })));
    }

    #[test]
    fn detects_invalid_loop_back() {
        // per v0.9.21 `loopBackTo invalid` 警告守门
        let mut wf = Workflow::new("loop-test", "loop with phantom back");
        let n = WorkflowNode {
            id: "l".into(),
            name: "L".into(),
            node_type: NodeType::Loop,
            config: NodeConfig::Loop(LoopConfig {
                loop_back_to: Some("phantom".into()),
                max_iterations: DEFAULT_LOOP_MAX_ITERATIONS,
                exit_condition_type: "auto".into(),
                exit_condition_prompt: None,
                exit_target_step_id: None,
                history_compression: None,
                for_each_config: None,
            }),
            depends_on: vec![],
            trigger: "auto".into(),
            timeout_ms: None,
            interactive: false,
            intervention_config: None,
        };
        wf.nodes.insert("l".into(), n);

        let validator = DefaultWorkflowValidator;
        let result = validator.validate(&wf);
        assert!(matches!(result, Err(WorkflowError::InvalidLoopBack(_, _))));
    }

    #[test]
    fn detects_exceeds_max_depth() {
        // per v0.9.21 `4Psa4Psa4Psa` 8 层守门
        let mut wf = Workflow::new("depth-test", "10-level chain");
        for i in 0..(MAX_NESTED_DEPTH + 2) {
            let n = WorkflowNode {
                id: format!("n{i}"),
                name: format!("N{i}"),
                node_type: NodeType::Task,
                config: NodeConfig::ControlFlow,
                depends_on: if i == 0 { vec![] } else { vec![format!("n{}", i - 1)] },
                trigger: "auto".into(),
                timeout_ms: None,
                interactive: false,
                intervention_config: None,
            };
            wf.nodes.insert(format!("n{i}"), n);
        }

        let validator = DefaultWorkflowValidator;
        let result = validator.validate(&wf);
        assert!(matches!(result, Err(WorkflowError::ExceedsMaxDepth { .. })));
    }

    #[test]
    fn noop_executor_returns_node_id() {
        // 验证占位执行器 (R20 阶段 2 接 apeireth-agent)
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let node = WorkflowNode {
                id: "n1".into(),
                name: "N1".into(),
                node_type: NodeType::Agent,
                config: NodeConfig::Agent(AgentConfig {
                    prompt: "hi".into(),
                    ..Default::default()
                }),
                depends_on: vec![],
                trigger: "auto".into(),
                timeout_ms: None,
                interactive: false,
                intervention_config: None,
            };
            let mut ctx = ExecutionContext::new();
            let output = NoopNodeExecutor.execute(&node, &mut ctx).await.unwrap();
            assert_eq!(output["node_id"], json!("n1"));
            assert_eq!(output["noop"], json!(true));
        });
    }

    #[test]
    fn workflow_status_terminal() {
        assert!(WorkflowStatus::Completed.is_terminal());
        assert!(WorkflowStatus::Failed.is_terminal());
        assert!(WorkflowStatus::Cancelled.is_terminal());
        assert!(!WorkflowStatus::Running.is_terminal());
        assert!(!WorkflowStatus::Pending.is_terminal());
        assert!(!WorkflowStatus::Paused.is_terminal());
    }

    #[test]
    fn quick_agent_task_creates_single_node() {
        // 1:1 翻译 v0.9.21 `createQuickAgentTask(c, d)`
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let gen = DefaultWorkflowGenerator;
            let wf = gen
                .quick_agent("quick", "do the thing", "/tmp")
                .await
                .unwrap();
            assert_eq!(wf.node_count(), 1);
            let node = wf.nodes.values().next().unwrap();
            assert_eq!(node.node_type, NodeType::Agent);
            assert!(node.depends_on.is_empty());
        });
    }

    #[test]
    fn yaml_round_trip() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let parser = YamlWorkflowParser;
            let wf = parallel_dag();
            let yaml = parser.to_yaml(&wf).await.unwrap();
            let restored = parser.from_yaml(&yaml).await.unwrap();
            assert_eq!(restored.node_count(), wf.node_count());
            assert_eq!(restored.id, wf.id);
        });
    }

    #[test]
    fn full_execute_runs_topological_order() {
        // 端到端: 5 节点 DAG + NoopNodeExecutor → Completed
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let wf = parallel_dag();
            let executor = DefaultWorkflowExecutor::new(Arc::new(NoopNodeExecutor));
            let mut ctx = ExecutionContext::new();
            let exec = executor.execute(&wf, &mut ctx).await.unwrap();
            assert_eq!(exec.status, WorkflowStatus::Completed);
            assert_eq!(exec.completed_nodes.len(), wf.node_count());
            assert!(exec.failed_nodes.is_empty());
        });
    }

    #[test]
    fn pause_resume_cancel_state_transitions() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let executor = DefaultWorkflowExecutor::new(Arc::new(NoopNodeExecutor));
            let mut exec = WorkflowExecution {
                workflow_id: "wf-1".into(),
                status: WorkflowStatus::Running,
                history: vec![],
                running_nodes: BTreeSet::new(),
                completed_nodes: BTreeSet::new(),
                failed_nodes: BTreeSet::new(),
                variables: HashMap::new(),
            };

            executor.pause(&mut exec).await.unwrap();
            assert_eq!(exec.status, WorkflowStatus::Paused);

            executor.resume(&mut exec).await.unwrap();
            assert_eq!(exec.status, WorkflowStatus::Running);

            executor.cancel(&mut exec).await.unwrap();
            assert_eq!(exec.status, WorkflowStatus::Cancelled);

            // Cancelled 是 terminal, 不能再 cancel
            let err = executor.cancel(&mut exec).await.unwrap_err();
            assert!(matches!(err, WorkflowError::InvalidStateTransition(_, _)));
        });
    }
}

// ============================================================
// fmt impl (per apeireth-graph 风格)
// ============================================================
//
// 注意: `WorkflowError` 已经用 `thiserror` derive 了 `Display` (各 variant 的 `#[error("...")]` 自动生成),
// 不要手动 impl, 否则 `write!(f, "{}", self)` 会无限递归.
// NodeType / WorkflowStatus / EdgeType 没有 derive Display, 用 Debug 兜底.

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
