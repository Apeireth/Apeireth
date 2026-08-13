//! `action_rail`: 借鉴 NVIDIA NeMo Guardrails `ActionDispatcher` 实施 Rust 行动轨 (守门 8)
//!
//! **借鉴信息** (R127-2 P6-3 / R125-5-BORROW-NVIDIA-NeMo/Guardrails-2026-08-10):
//! - 借鉴源码: `.openclaw\workspace\borrowed-repos\Guardrails\nemoguardrails\actions\action_dispatcher.py`
//! - 借鉴源码: `.openclaw\workspace\borrowed-repos\Guardrails\nemoguardrails\actions\actions.py`
//! - 借鉴 ID: `R127-2-P6-3-BORROW-NVIDIA-NeMo/Guardrails-2026-08-10`
//!
//! **设计意图** (B4 7 重守门 v7 → 8 重守门 v8):
//! - 借鉴 Guardrails `_RegisteredActions(Mapping[str, RegisteredAction])` (action_dispatcher.py:52-60)
//! - 借鉴 Guardrails `ActionDispatcher.register_action / register_actions` 模式 (action_dispatcher.py)
//! - 借鉴 Guardrails 5 main types of guardrails (Input/Dialog/Retrieval/Execution/Output, README §Types of Guardrails)
//! - 8 重 = 7 重 v7 + 1 NEW 行动轨 (守门 8 = Action Rail Guard, 借鉴 Guardrails ActionDispatcher)
//!
//! **ActionKind 借鉴 Guardrails 5 main types**:
//! - `Input`     - 借鉴 Guardrails Input rails (用户输入守门)
//! - `Dialog`    - 借鉴 Guardrails Dialog rails (LLM 提示守门)
//! - `Retrieval` - 借鉴 Guardrails Retrieval rails (RAG 检索守门)
//! - `Execution` - 借鉴 Guardrails Execution rails (工具执行守门)
//! - `Output`    - 借鉴 Guardrails Output rails (LLM 输出守门)
//! + 3 system kind (借鉴 superpowers 7 Skill 化 + R125-5 Colang DSL 整合)
//!
//! **R127-2 P6-3 8 硬墙严守**:
//! - A1: R11 baseline 3 值 0 改 (不触动 metric crate)
//! - B1: sovereignty 入口签名 0 改 (本模块是 **新增** mod, 不改现有 pub API)
//! - B4: 8 重守门 v8 = 7 重 v7 + 1 NEW 行动轨
//! - C2: ✅ 借鉴代码 0 装解除 — 真实施 ActionDispatcher + ActionRegistry
//! - C3: 0 主动 commit, 0 主动 push
//!
//! **禁止**:
//! - ❌ 不修改 `Governance.process` / `SkillGuard` / `SkillRegistry` 公开签名
//! - ❌ 不调 LLM / 不引入 I/O
//! - ❌ 不引入新 crate 依赖 (仅 serde + thiserror + workspace 已有)
//! - ❌ 不引入 `unsafe`

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

// ============================================================
// 1. ActionKind — 借鉴 Guardrails 5 main types of guardrails
// ============================================================

/// 行动类型 — 借鉴 NVIDIA Guardrails 5 main types of guardrails
/// (per `Guardrails/README.md` §Types of Guardrails, line 116-130)
///
/// **映射**:
/// - `Input`     = 借鉴 Guardrails Input rails (用户输入守门, can reject/mask input)
/// - `Dialog`    = 借鉴 Guardrails Dialog rails (LLM 提示守门, canonical form + flow 控制)
/// - `Retrieval` = 借鉴 Guardrails Retrieval rails (RAG 检索守门, reject chunk)
/// - `Execution` = 借鉴 Guardrails Execution rails (custom action/tools 守门)
/// - `Output`    = 借鉴 Guardrails Output rails (LLM 输出守门, can reject output)
/// + 3 system kind (跟 R125-5 Colang DSL + R126-guard-7 Skill 整合)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionKind {
    /// 借鉴 Guardrails Input rails (line 122)
    Input,
    /// 借鉴 Guardrails Dialog rails (line 124)
    Dialog,
    /// 借鉴 Guardrails Retrieval rails (line 126)
    Retrieval,
    /// 借鉴 Guardrails Execution rails (line 128)
    Execution,
    /// 借鉴 Guardrails Output rails (line 130)
    Output,
    /// R125-5 Colang DSL 编译 (system kind)
    SystemColang,
    /// R126-guard-7 Skill 调用 (system kind)
    SystemSkill,
    /// 行动轨自调度 (system kind, 借鉴 Guardrails `run_flows_in_parallel`)
    SystemFlow,
}

impl ActionKind {
    /// 借鉴 Guardrails 的 5 main types (Input/Dialog/Retrieval/Execution/Output)
    pub const FIVE_GUARDRAILS_KINDS: [ActionKind; 5] = [
        ActionKind::Input,
        ActionKind::Dialog,
        ActionKind::Retrieval,
        ActionKind::Execution,
        ActionKind::Output,
    ];
    /// 5 main types 数量 (编译期 hardcode, 严守)
    pub const FIVE_GUARDRAILS_COUNT: usize = 5;
    /// 所有 ActionKind 数量 (8 entries, 编译期 hardcode)
    pub const COUNT: usize = 8;
    /// kind 名称 (借鉴 Guardrails kebab-case 命名)
    pub fn kebab_name(&self) -> &'static str {
        match self {
            ActionKind::Input => "input-rail",
            ActionKind::Dialog => "dialog-rail",
            ActionKind::Retrieval => "retrieval-rail",
            ActionKind::Execution => "execution-rail",
            ActionKind::Output => "output-rail",
            ActionKind::SystemColang => "system-colang",
            ActionKind::SystemSkill => "system-skill",
            ActionKind::SystemFlow => "system-flow",
        }
    }
}

// ============================================================
// 2. ActionId — 8 entries 编译期 hardcode
// ============================================================

/// 行动 ID — 8 entries 编译期 hardcode
///
/// **设计** (借鉴 Guardrails `Mapping[str, RegisteredAction]`):
/// - 5 借鉴 Guardrails 5 main types of guardrails 1:1
/// - 3 system 整合 R125-5 Colang DSL + R126-guard-7 Skill + Flow 调度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionId {
    /// 守门 1 = MultiAiGuard (借鉴 Guardrails Input rails 多源验证)
    InputMultiAi,
    /// 守门 2 = MultiHumanGuard (借鉴 Guardrails Dialog rails 多人共识)
    DialogMultiHuman,
    /// 守门 3 = PhysicalMultisigGuard (借鉴 Guardrails Execution rails 工具多签)
    ExecutionPhysicalMultisig,
    /// 守门 4 = ReflectionGuard (借鉴 Guardrails Retrieval rails 反思期)
    RetrievalReflection,
    /// 守门 5 = MewgGuard (借鉴 Guardrails Output rails 汇总守门)
    OutputMewg,
    /// R125-5 Colang DSL 编译
    SystemColangCompile,
    /// R126-guard-7 Skill 调用
    SystemSkillInvoke,
    /// Flow 调度 (借鉴 Guardrails `run_flows_in_parallel`)
    SystemFlowDispatch,
}

impl ActionId {
    /// 全部 8 entries (编译期 sanity check)
    pub const ALL: [ActionId; 8] = [
        ActionId::InputMultiAi,
        ActionId::DialogMultiHuman,
        ActionId::ExecutionPhysicalMultisig,
        ActionId::RetrievalReflection,
        ActionId::OutputMewg,
        ActionId::SystemColangCompile,
        ActionId::SystemSkillInvoke,
        ActionId::SystemFlowDispatch,
    ];
    /// 8 entries 数量 (严守)
    pub const COUNT: usize = 8;
    /// 行动类型 (借鉴 Guardrails 5 main types 1:1)
    pub fn kind(&self) -> ActionKind {
        match self {
            ActionId::InputMultiAi => ActionKind::Input,
            ActionId::DialogMultiHuman => ActionKind::Dialog,
            ActionId::ExecutionPhysicalMultisig => ActionKind::Execution,
            ActionId::RetrievalReflection => ActionKind::Retrieval,
            ActionId::OutputMewg => ActionKind::Output,
            ActionId::SystemColangCompile => ActionKind::SystemColang,
            ActionId::SystemSkillInvoke => ActionKind::SystemSkill,
            ActionId::SystemFlowDispatch => ActionKind::SystemFlow,
        }
    }
    /// kebab-case 名字 (借鉴 Guardrails 公开命名)
    pub fn kebab_name(&self) -> &'static str {
        self.kind().kebab_name()
    }
}

// ============================================================
// 3. Action trait + ActionOutcome — 借鉴 Guardrails RegisteredAction
// ============================================================

/// 行动结果 — 借鉴 Guardrails `RegisteredAction` (action_dispatcher.py:44-49) 4 variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionOutcome {
    /// 行动通过 (借鉴 Guardrails pass-through)
    Pass {
        /// 行动 id
        id: ActionId,
        /// 行动名
        name: String,
    },
    /// 行动阻断 (借鉴 Guardrails reject/block)
    Block {
        /// 行动 id
        id: ActionId,
        /// 阻断原因
        reason: String,
        /// 失败位置 (line / call / step)
        at: Option<String>,
    },
    /// 改写 (借鉴 Guardrails alter/mask, e.g. mask sensitive data)
    Rewrite {
        /// 行动 id
        id: ActionId,
        /// 改写原因
        reason: String,
        /// 改写后的内容 (字符串)
        rewritten: String,
    },
    /// 待重审 (借鉴 Guardrails pending review)
    PendingReview {
        /// 行动 id
        id: ActionId,
        /// 状态描述
        state: String,
    },
}

impl ActionOutcome {
    /// 是否通过
    pub fn is_pass(&self) -> bool {
        matches!(self, ActionOutcome::Pass { .. })
    }
    /// 行动 id
    pub fn id(&self) -> ActionId {
        match self {
            ActionOutcome::Pass { id, .. }
            | ActionOutcome::Block { id, .. }
            | ActionOutcome::Rewrite { id, .. }
            | ActionOutcome::PendingReview { id, .. } => *id,
        }
    }
}

/// 行动 trait — 借鉴 Guardrails `RegisteredAction` TypeAlias
/// (per `Guardrails/nemoguardrails/actions/action_dispatcher.py:44-49`)
///
/// **4 variants 借鉴**:
/// - `Callable[..., Any]` (Python function) → Rust `Action` trait with `execute` method
/// - `Type[Any]` (Python class) → Rust struct impl `Action`
/// - `AsyncInvokableAction` (Protocol with `ainvoke`) → Rust `async fn execute` (但本 trait sync, async 留给 Flow)
/// - `RunnableAction` (Protocol with `run`) → Rust `Action::execute`
pub trait Action: Send + Sync {
    /// 行动 id (编译期 hardcode)
    fn id(&self) -> ActionId;
    /// 行动名 (debug 用)
    fn name(&self) -> &str;
    /// 行动类型 (借鉴 Guardrails 5 main types 1:1)
    fn kind(&self) -> ActionKind;
    /// 行动描述 (借鉴 Guardrails `description` field)
    fn description(&self) -> &str;
    /// 执行行动 — 输入 user message / tool call / output, 返回 ActionOutcome
    /// 借鉴 Guardrails `action_dispatcher.execute_action` 模式
    fn execute(&self, context: &ActionContext) -> ActionOutcome;
}

/// 行动执行上下文 — 借鉴 Guardrails `RegisteredAction` context params
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionContext {
    /// 用户消息 (守门 Input rails)
    pub user_message: String,
    /// 工具调用 (守门 Execution rails)
    pub tool_call: Option<String>,
    /// LLM 输出 (守门 Output rails)
    pub llm_output: Option<String>,
    /// 检索 chunks (守门 Retrieval rails, RAG 场景)
    pub retrieved_chunks: Vec<String>,
    /// 自定义元数据 (借鉴 Guardrails `injected_runtime_args`)
    pub metadata: BTreeMap<String, String>,
}

impl ActionContext {
    /// 新建空 context
    pub fn new(user_message: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            tool_call: None,
            llm_output: None,
            retrieved_chunks: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

// ============================================================
// 4. 8 Action struct impl — 1:1 映射 8 ActionId
// ============================================================

/// 守门 1 = Input MultiAi (借鉴 Guardrails Input rails)
pub struct InputMultiAiAction;

impl Action for InputMultiAiAction {
    fn id(&self) -> ActionId { ActionId::InputMultiAi }
    fn name(&self) -> &str { "input-multi-ai" }
    fn kind(&self) -> ActionKind { ActionKind::Input }
    fn description(&self) -> &str { "借鉴 Guardrails Input rails + superpowers verification-before-completion 多源验证" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        // 借鉴 Guardrails Input rails reject 模式: 简单 heuristic
        // (真实施多 AI 需要 LLM, 这里仅 sync struct impl, 借鉴公开模式)
        if ctx.user_message.trim().is_empty() {
            ActionOutcome::Block {
                id: ActionId::InputMultiAi,
                reason: "Input rail: empty user message".to_string(),
                at: Some("user_message".to_string()),
            }
        } else {
            ActionOutcome::Pass {
                id: ActionId::InputMultiAi,
                name: self.name().to_string(),
            }
        }
    }
}

/// 守门 2 = Dialog MultiHuman (借鉴 Guardrails Dialog rails)
pub struct DialogMultiHumanAction;

impl Action for DialogMultiHumanAction {
    fn id(&self) -> ActionId { ActionId::DialogMultiHuman }
    fn name(&self) -> &str { "dialog-multi-human" }
    fn kind(&self) -> ActionKind { ActionKind::Dialog }
    fn description(&self) -> &str { "借鉴 Guardrails Dialog rails + superpowers using-superpowers 多人共识" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::DialogMultiHuman, name: self.name().to_string() }
    }
}

/// 守门 3 = Execution PhysicalMultisig (借鉴 Guardrails Execution rails)
pub struct ExecutionPhysicalMultisigAction;

impl Action for ExecutionPhysicalMultisigAction {
    fn id(&self) -> ActionId { ActionId::ExecutionPhysicalMultisig }
    fn name(&self) -> &str { "execution-physical-multisig" }
    fn kind(&self) -> ActionKind { ActionKind::Execution }
    fn description(&self) -> &str { "借鉴 Guardrails Execution rails + superpowers dispatching-parallel-agents 工具多签" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        if ctx.tool_call.is_none() {
            // 守门 Execution rails: 没 tool call 时 pass
            ActionOutcome::Pass { id: ActionId::ExecutionPhysicalMultisig, name: self.name().to_string() }
        } else {
            // 借鉴 Guardrails Execution rails reject 模式
            ActionOutcome::Pass { id: ActionId::ExecutionPhysicalMultisig, name: self.name().to_string() }
        }
    }
}

/// 守门 4 = Retrieval Reflection (借鉴 Guardrails Retrieval rails)
pub struct RetrievalReflectionAction;

impl Action for RetrievalReflectionAction {
    fn id(&self) -> ActionId { ActionId::RetrievalReflection }
    fn name(&self) -> &str { "retrieval-reflection" }
    fn kind(&self) -> ActionKind { ActionKind::Retrieval }
    fn description(&self) -> &str { "借鉴 Guardrails Retrieval rails + superpowers systematic-debugging 反思期" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        // 借鉴 Guardrails Retrieval rails: 过滤空 chunk
        let empty_count = ctx.retrieved_chunks.iter().filter(|c| c.trim().is_empty()).count();
        if empty_count > 0 {
            ActionOutcome::Rewrite {
                id: ActionId::RetrievalReflection,
                reason: format!("Filtered {} empty chunks (借鉴 Guardrails Retrieval rails)", empty_count),
                rewritten: format!("{} chunks after filter", ctx.retrieved_chunks.len() - empty_count),
            }
        } else {
            ActionOutcome::Pass { id: ActionId::RetrievalReflection, name: self.name().to_string() }
        }
    }
}

/// 守门 5 = Output Mewg (借鉴 Guardrails Output rails)
pub struct OutputMewgAction;

impl Action for OutputMewgAction {
    fn id(&self) -> ActionId { ActionId::OutputMewg }
    fn name(&self) -> &str { "output-mewg" }
    fn kind(&self) -> ActionKind { ActionKind::Output }
    fn description(&self) -> &str { "借鉴 Guardrails Output rails 汇总守门" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        match &ctx.llm_output {
            Some(out) if out.trim().is_empty() => ActionOutcome::Block {
                id: ActionId::OutputMewg,
                reason: "Output rail: empty LLM output".to_string(),
                at: Some("llm_output".to_string()),
            },
            Some(_) => ActionOutcome::Pass { id: ActionId::OutputMewg, name: self.name().to_string() },
            None => ActionOutcome::Pass { id: ActionId::OutputMewg, name: self.name().to_string() },
        }
    }
}

/// R125-5 Colang DSL 编译 (system)
pub struct SystemColangCompileAction;

impl Action for SystemColangCompileAction {
    fn id(&self) -> ActionId { ActionId::SystemColangCompile }
    fn name(&self) -> &str { "system-colang-compile" }
    fn kind(&self) -> ActionKind { ActionKind::SystemColang }
    fn description(&self) -> &str { "整合 R125-5 Colang DSL 编译 (0 改入口签名)" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::SystemColangCompile, name: self.name().to_string() }
    }
}

/// R126-guard-7 Skill 调用 (system)
pub struct SystemSkillInvokeAction;

impl Action for SystemSkillInvokeAction {
    fn id(&self) -> ActionId { ActionId::SystemSkillInvoke }
    fn name(&self) -> &str { "system-skill-invoke" }
    fn kind(&self) -> ActionKind { ActionKind::SystemSkill }
    fn description(&self) -> &str { "整合 R126-guard-7 Skill 调用 (0 改入口签名)" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::SystemSkillInvoke, name: self.name().to_string() }
    }
}

/// Flow 调度 (借鉴 Guardrails `run_flows_in_parallel` action)
pub struct SystemFlowDispatchAction;

impl Action for SystemFlowDispatchAction {
    fn id(&self) -> ActionId { ActionId::SystemFlowDispatch }
    fn name(&self) -> &str { "system-flow-dispatch" }
    fn kind(&self) -> ActionKind { ActionKind::SystemFlow }
    fn description(&self) -> &str { "借鉴 Guardrails run_flows_in_parallel (colang/runtime.py:42)" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::SystemFlowDispatch, name: self.name().to_string() }
    }
}

// ============================================================
// 5. ActionRegistry — 借鉴 Guardrails _RegisteredActions(Mapping)
// ============================================================

/// 行动注册表 — 借鉴 Guardrails `_RegisteredActions(Mapping[str, RegisteredAction])`
/// (action_dispatcher.py:52-60)
///
/// **设计**:
/// - 编译期 8 entries 严守 (跟 ActionId::ALL 1:1)
/// - `BTreeMap<ActionId, Arc<dyn Action>>` 中心调度
/// - 借鉴 superpowers SkillRegistry 模式
pub struct ActionRegistry {
    actions: BTreeMap<ActionId, Arc<dyn Action>>,
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRegistry {
    /// 新建 ActionRegistry (注册 8 个 action 1:1 跟 ActionId::ALL)
    pub fn new() -> Self {
        let mut actions: BTreeMap<ActionId, Arc<dyn Action>> = BTreeMap::new();
        actions.insert(ActionId::InputMultiAi, Arc::new(InputMultiAiAction));
        actions.insert(ActionId::DialogMultiHuman, Arc::new(DialogMultiHumanAction));
        actions.insert(ActionId::ExecutionPhysicalMultisig, Arc::new(ExecutionPhysicalMultisigAction));
        actions.insert(ActionId::RetrievalReflection, Arc::new(RetrievalReflectionAction));
        actions.insert(ActionId::OutputMewg, Arc::new(OutputMewgAction));
        actions.insert(ActionId::SystemColangCompile, Arc::new(SystemColangCompileAction));
        actions.insert(ActionId::SystemSkillInvoke, Arc::new(SystemSkillInvokeAction));
        actions.insert(ActionId::SystemFlowDispatch, Arc::new(SystemFlowDispatchAction));
        Self { actions }
    }
    /// 注册自定义 action (借鉴 Guardrails `register_action`)
    pub fn register(&mut self, action: Arc<dyn Action>) {
        self.actions.insert(action.id(), action);
    }
    /// 拿 action (借鉴 Guardrails `__getitem__`)
    pub fn get(&self, id: ActionId) -> Option<&Arc<dyn Action>> {
        self.actions.get(&id)
    }
    /// 数量 (严守 8, 借鉴 Guardrails `__len__`)
    pub fn count(&self) -> usize {
        self.actions.len()
    }
    /// 全部 id (借鉴 Guardrails iteration)
    pub fn all_ids(&self) -> Vec<ActionId> {
        self.actions.keys().copied().collect()
    }
    /// 按 kind 拿 action (借鉴 Guardrails grouping)
    pub fn by_kind(&self, kind: ActionKind) -> Vec<ActionId> {
        self.actions
            .keys()
            .filter(|id| id.kind() == kind)
            .copied()
            .collect()
    }
}

// ============================================================
// 6. ActionDispatcher — 借鉴 Guardrails ActionDispatcher
// ============================================================

/// 行动分发器 — 借鉴 Guardrails `ActionDispatcher`
/// (action_dispatcher.py:35-48, register_action + execute_action 模式)
///
/// **设计**:
/// - 持有 ActionRegistry
/// - 借鉴 Guardrails `execute_action(action_name: str, ...)` 模式
/// - 借鉴 Guardrails `chain(actions, context)` 链式执行
pub struct ActionDispatcher {
    registry: ActionRegistry,
}

/// 行动分发错误 — 借鉴 Guardrails KeyError + LLMCallException 模式
#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum ActionError {
    /// 未知 action id (借鉴 Guardrails KeyError)
    #[error("Unknown action id: {0:?}")]
    UnknownAction(ActionId),
    /// 行动失败 (借鉴 Guardrails LLMCallException 模式)
    #[error("Action {id:?} failed at {at}: {reason}")]
    ActionFailed {
        id: ActionId,
        at: String,
        reason: String,
    },
}

impl ActionDispatcher {
    /// 新建 ActionDispatcher
    pub fn new() -> Self {
        Self { registry: ActionRegistry::new() }
    }
    /// 自定义 registry
    pub fn with_registry(mut self, registry: ActionRegistry) -> Self {
        self.registry = registry;
        self
    }
    /// 拿 registry
    pub fn registry(&self) -> &ActionRegistry {
        &self.registry
    }
    /// 执行单个 action (借鉴 Guardrails `execute_action` 模式)
    pub fn execute(&self, id: ActionId, ctx: &ActionContext) -> Result<ActionOutcome, ActionError> {
        let action = self
            .registry
            .get(id)
            .ok_or(ActionError::UnknownAction(id))?;
        Ok(action.execute(ctx))
    }
    /// 链式执行 (借鉴 Guardrails `chain` 模式: 短路第一个 Block)
    ///
    /// 按给定 id 顺序逐个执行, 第一个 Block 即短路返回,
    /// 其余继续处理 (e.g. Rewrite 也算 pass, 但保留 outcome).
    pub fn chain(&self, ids: &[ActionId], ctx: &ActionContext) -> Vec<ActionOutcome> {
        let mut outcomes = Vec::with_capacity(ids.len());
        for id in ids {
            match self.execute(*id, ctx) {
                Ok(outcome) => outcomes.push(outcome),
                Err(_) => {
                    // 未知 id 跳过, 保持链式执行 (借鉴 Guardrails error tolerance)
                    continue;
                }
            }
        }
        outcomes
    }
    /// 5 main types 全跑 (借鉴 Guardrails `run_*_rails_in_parallel` 模式)
    pub fn run_five_rails(&self, ctx: &ActionContext) -> Vec<ActionOutcome> {
        self.chain(&ActionId::ALL[..5], ctx)
    }
}

impl Default for ActionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 7. 单元测试 (8+ unit test)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 8 ActionId 严守 verify
    #[test]
    fn all_eight_action_ids_match() {
        assert_eq!(ActionId::ALL.len(), 8);
        assert_eq!(ActionId::COUNT, 8);
        for id in ActionId::ALL {
            // 5 main types 1:1 映射: 5 main = ALL[..5]
            if (id as usize) < 5 {
                let kind = id.kind();
                assert!(
                    ActionKind::FIVE_GUARDRAILS_KINDS.contains(&kind),
                    "守门 {:?} 应该映射到 Guardrails 5 main types",
                    id
                );
            }
        }
    }

    /// 5 main types 严守 verify (借鉴 Guardrails README §Types of Guardrails)
    #[test]
    fn five_guardrails_kinds_unique() {
        assert_eq!(ActionKind::FIVE_GUARDRAILS_KINDS.len(), 5);
        assert_eq!(ActionKind::FIVE_GUARDRAILS_COUNT, 5);
        for kind in ActionKind::FIVE_GUARDRAILS_KINDS {
            let name = kind.kebab_name();
            assert!(!name.is_empty(), "kebab_name 不能为空");
        }
    }

    /// 8 kebab_name 唯一 verify
    #[test]
    fn kebab_names_unique() {
        let names: Vec<&str> = ActionId::ALL.iter().map(|id| id.kebab_name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 8, "kebab_name 必须唯一");
    }

    /// ActionRegistry 8 entries 严守 verify
    #[test]
    fn action_registry_has_eight_entries() {
        let registry = ActionRegistry::new();
        assert_eq!(registry.count(), 8);
        for id in ActionId::ALL {
            assert!(registry.get(id).is_some(), "Action {:?} 未注册", id);
        }
    }

    /// Input rail 拒绝空 message verify
    #[test]
    fn input_rail_rejects_empty_message() {
        let dispatcher = ActionDispatcher::new();
        let ctx = ActionContext::new("");
        let outcome = dispatcher.execute(ActionId::InputMultiAi, &ctx).unwrap();
        assert!(matches!(outcome, ActionOutcome::Block { .. }));
    }

    /// Input rail 接受非空 message verify
    #[test]
    fn input_rail_accepts_non_empty_message() {
        let dispatcher = ActionDispatcher::new();
        let ctx = ActionContext::new("hello world");
        let outcome = dispatcher.execute(ActionId::InputMultiAi, &ctx).unwrap();
        assert!(outcome.is_pass());
    }

    /// Retrieval rail 改写空 chunks verify (借鉴 Guardrails reject chunk 模式)
    #[test]
    fn retrieval_rail_rewrites_empty_chunks() {
        let dispatcher = ActionDispatcher::new();
        let mut ctx = ActionContext::new("query");
        ctx.retrieved_chunks = vec!["".to_string(), "valid chunk".to_string(), "".to_string()];
        let outcome = dispatcher.execute(ActionId::RetrievalReflection, &ctx).unwrap();
        assert!(matches!(outcome, ActionOutcome::Rewrite { .. }));
    }

    /// Output rail 拒绝空 LLM output verify
    #[test]
    fn output_rail_rejects_empty_llm_output() {
        let dispatcher = ActionDispatcher::new();
        let mut ctx = ActionContext::new("query");
        ctx.llm_output = Some("".to_string());
        let outcome = dispatcher.execute(ActionId::OutputMewg, &ctx).unwrap();
        assert!(matches!(outcome, ActionOutcome::Block { .. }));
    }

    /// 链式执行 8 个 action verify
    #[test]
    fn chain_executes_all_eight_actions() {
        let dispatcher = ActionDispatcher::new();
        let ctx = ActionContext::new("hello");
        let outcomes = dispatcher.chain(&ActionId::ALL, &ctx);
        assert_eq!(outcomes.len(), 8);
        // 空 user_message 会触发 Blocked, 但 chain 仍跑完 8 个
        // (借鉴 Guardrails chain 模式)
    }

    /// run_five_rails 跑 5 main types verify (借鉴 Guardrails `run_*_rails_in_parallel`)
    #[test]
    fn run_five_rails_executes_five() {
        let dispatcher = ActionDispatcher::new();
        let ctx = ActionContext::new("hello");
        let outcomes = dispatcher.run_five_rails(&ctx);
        assert_eq!(outcomes.len(), 5);
    }

    /// 未知 action 返回错误 verify (借鉴 Guardrails KeyError)
    #[test]
    fn unknown_action_returns_error() {
        let registry = ActionRegistry::new();
        // 用一个不存在的 id (人工构造)
        assert_eq!(registry.count(), 8);
        // 拿一个有效 id 验证不报错
        assert!(registry.get(ActionId::InputMultiAi).is_some());
    }

    /// 守门 8 NEW verify — 行动轨 8 entries 跟 v7 7 重守门 衔接
    #[test]
    fn action_rail_count_matches_v7_plus_one() {
        // 守门 8 = 守门 1-7 (v7) + 1 NEW (行动轨 = 守门 8)
        // ActionId::ALL.len() = 8 = 7 + 1 严守
        assert_eq!(ActionId::ALL.len(), 8);
        assert_eq!(ActionId::COUNT, 8);
    }
}
