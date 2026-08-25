//! action_rail: 8 重守门 v8 行动轨

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionKind {
    Input, Dialog, Retrieval, Execution, Output,
    SystemColang, SystemSkill, SystemFlow,
}

impl ActionKind {
    pub const FIVE_GUARDRAILS_KINDS: [ActionKind; 5] = [Self::Input, Self::Dialog, Self::Retrieval, Self::Execution, Self::Output];
    pub const FIVE_GUARDRAILS_COUNT: usize = 5;
    pub const COUNT: usize = 8;
    pub fn kebab_name(&self) -> &'static str {
        match self {
            Self::Input => "input-rail", Self::Dialog => "dialog-rail", Self::Retrieval => "retrieval-rail",
            Self::Execution => "execution-rail", Self::Output => "output-rail",
            Self::SystemColang => "system-colang", Self::SystemSkill => "system-skill", Self::SystemFlow => "system-flow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionId {
    InputMultiAi, DialogMultiHuman, ExecutionPhysicalMultisig, RetrievalReflection,
    OutputMewg, SystemColangCompile, SystemSkillInvoke, SystemFlowDispatch,
}

impl ActionId {
    pub const ALL: [ActionId; 8] = [
        Self::InputMultiAi, Self::DialogMultiHuman, Self::ExecutionPhysicalMultisig, Self::RetrievalReflection,
        Self::OutputMewg, Self::SystemColangCompile, Self::SystemSkillInvoke, Self::SystemFlowDispatch,
    ];
    pub const COUNT: usize = 8;
    pub fn kind(&self) -> ActionKind {
        match self {
            Self::InputMultiAi => ActionKind::Input, Self::DialogMultiHuman => ActionKind::Dialog,
            Self::ExecutionPhysicalMultisig => ActionKind::Execution, Self::RetrievalReflection => ActionKind::Retrieval,
            Self::OutputMewg => ActionKind::Output, Self::SystemColangCompile => ActionKind::SystemColang,
            Self::SystemSkillInvoke => ActionKind::SystemSkill, Self::SystemFlowDispatch => ActionKind::SystemFlow,
        }
    }
    pub fn kebab_name(&self) -> &'static str { self.kind().kebab_name() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionOutcome {
    Pass { id: ActionId, name: String },
    Block { id: ActionId, reason: String, at: Option<String> },
    Rewrite { id: ActionId, reason: String, rewritten: String },
    PendingReview { id: ActionId, state: String },
}

impl ActionOutcome {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass { .. }) }
    pub fn id(&self) -> ActionId {
        match self {
            Self::Pass { id, .. } | Self::Block { id, .. } | Self::Rewrite { id, .. } | Self::PendingReview { id, .. } => *id,
        }
    }
}

pub trait Action: Send + Sync {
    fn id(&self) -> ActionId;
    fn name(&self) -> &str;
    fn kind(&self) -> ActionKind;
    fn description(&self) -> &str;
    fn execute(&self, context: &ActionContext) -> ActionOutcome;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionContext {
    pub user_message: String,
    pub tool_call: Option<String>,
    pub llm_output: Option<String>,
    pub retrieved_chunks: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ActionContext {
    pub fn new(user_message: impl Into<String>) -> Self {
        Self { user_message: user_message.into(), tool_call: None, llm_output: None, retrieved_chunks: Vec::new(), metadata: BTreeMap::new() }
    }
}

pub struct InputMultiAiAction;
impl Action for InputMultiAiAction {
    fn id(&self) -> ActionId { ActionId::InputMultiAi }
    fn name(&self) -> &str { "input-multi-ai" }
    fn kind(&self) -> ActionKind { ActionKind::Input }
    fn description(&self) -> &str { "Input rail" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        if ctx.user_message.trim().is_empty() {
            ActionOutcome::Block { id: ActionId::InputMultiAi, reason: "Input rail: empty user message".into(), at: Some("user_message".into()) }
        } else {
            ActionOutcome::Pass { id: ActionId::InputMultiAi, name: self.name().to_string() }
        }
    }
}
pub struct DialogMultiHumanAction;
impl Action for DialogMultiHumanAction {
    fn id(&self) -> ActionId { ActionId::DialogMultiHuman }
    fn name(&self) -> &str { "dialog-multi-human" }
    fn kind(&self) -> ActionKind { ActionKind::Dialog }
    fn description(&self) -> &str { "Dialog rail" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::DialogMultiHuman, name: self.name().to_string() }
    }
}
pub struct ExecutionPhysicalMultisigAction;
impl Action for ExecutionPhysicalMultisigAction {
    fn id(&self) -> ActionId { ActionId::ExecutionPhysicalMultisig }
    fn name(&self) -> &str { "execution-physical-multisig" }
    fn kind(&self) -> ActionKind { ActionKind::Execution }
    fn description(&self) -> &str { "Execution rail" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome {
        ActionOutcome::Pass { id: ActionId::ExecutionPhysicalMultisig, name: self.name().to_string() }
    }
}
pub struct RetrievalReflectionAction;
impl Action for RetrievalReflectionAction {
    fn id(&self) -> ActionId { ActionId::RetrievalReflection }
    fn name(&self) -> &str { "retrieval-reflection" }
    fn kind(&self) -> ActionKind { ActionKind::Retrieval }
    fn description(&self) -> &str { "Retrieval rail" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        let empty_count = ctx.retrieved_chunks.iter().filter(|c| c.trim().is_empty()).count();
        if empty_count > 0 {
            ActionOutcome::Rewrite { id: ActionId::RetrievalReflection, reason: format!("Filtered {} empty chunks", empty_count), rewritten: format!("{} chunks after filter", ctx.retrieved_chunks.len() - empty_count) }
        } else {
            ActionOutcome::Pass { id: ActionId::RetrievalReflection, name: self.name().to_string() }
        }
    }
}
pub struct OutputMewgAction;
impl Action for OutputMewgAction {
    fn id(&self) -> ActionId { ActionId::OutputMewg }
    fn name(&self) -> &str { "output-mewg" }
    fn kind(&self) -> ActionKind { ActionKind::Output }
    fn description(&self) -> &str { "Output rail" }
    fn execute(&self, ctx: &ActionContext) -> ActionOutcome {
        match &ctx.llm_output {
            Some(out) if out.trim().is_empty() => ActionOutcome::Block { id: ActionId::OutputMewg, reason: "Output rail: empty LLM output".into(), at: Some("llm_output".into()) },
            _ => ActionOutcome::Pass { id: ActionId::OutputMewg, name: self.name().to_string() },
        }
    }
}
pub struct SystemColangCompileAction;
impl Action for SystemColangCompileAction {
    fn id(&self) -> ActionId { ActionId::SystemColangCompile }
    fn name(&self) -> &str { "system-colang-compile" }
    fn kind(&self) -> ActionKind { ActionKind::SystemColang }
    fn description(&self) -> &str { "Colang compile" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome { ActionOutcome::Pass { id: ActionId::SystemColangCompile, name: self.name().to_string() } }
}
pub struct SystemSkillInvokeAction;
impl Action for SystemSkillInvokeAction {
    fn id(&self) -> ActionId { ActionId::SystemSkillInvoke }
    fn name(&self) -> &str { "system-skill-invoke" }
    fn kind(&self) -> ActionKind { ActionKind::SystemSkill }
    fn description(&self) -> &str { "Skill invoke" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome { ActionOutcome::Pass { id: ActionId::SystemSkillInvoke, name: self.name().to_string() } }
}
pub struct SystemFlowDispatchAction;
impl Action for SystemFlowDispatchAction {
    fn id(&self) -> ActionId { ActionId::SystemFlowDispatch }
    fn name(&self) -> &str { "system-flow-dispatch" }
    fn kind(&self) -> ActionKind { ActionKind::SystemFlow }
    fn description(&self) -> &str { "Flow dispatch" }
    fn execute(&self, _ctx: &ActionContext) -> ActionOutcome { ActionOutcome::Pass { id: ActionId::SystemFlowDispatch, name: self.name().to_string() } }
}

pub struct ActionRegistry { actions: BTreeMap<ActionId, Arc<dyn Action>> }

impl Default for ActionRegistry { fn default() -> Self { Self::new() } }

impl ActionRegistry {
    pub fn new() -> Self {
        let mut a: BTreeMap<ActionId, Arc<dyn Action>> = BTreeMap::new();
        a.insert(ActionId::InputMultiAi, Arc::new(InputMultiAiAction));
        a.insert(ActionId::DialogMultiHuman, Arc::new(DialogMultiHumanAction));
        a.insert(ActionId::ExecutionPhysicalMultisig, Arc::new(ExecutionPhysicalMultisigAction));
        a.insert(ActionId::RetrievalReflection, Arc::new(RetrievalReflectionAction));
        a.insert(ActionId::OutputMewg, Arc::new(OutputMewgAction));
        a.insert(ActionId::SystemColangCompile, Arc::new(SystemColangCompileAction));
        a.insert(ActionId::SystemSkillInvoke, Arc::new(SystemSkillInvokeAction));
        a.insert(ActionId::SystemFlowDispatch, Arc::new(SystemFlowDispatchAction));
        Self { actions: a }
    }
    pub fn register(&mut self, action: Arc<dyn Action>) { self.actions.insert(action.id(), action); }
    pub fn get(&self, id: ActionId) -> Option<&Arc<dyn Action>> { self.actions.get(&id) }
    pub fn count(&self) -> usize { self.actions.len() }
    pub fn all_ids(&self) -> Vec<ActionId> { self.actions.keys().copied().collect() }
    pub fn by_kind(&self, kind: ActionKind) -> Vec<ActionId> {
        self.actions.keys().filter(|id| id.kind() == kind).copied().collect()
    }
}

#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum ActionError {
    #[error("Unknown action id: {0:?}")]
    UnknownAction(ActionId),
}

pub struct ActionDispatcher { registry: ActionRegistry }
impl Default for ActionDispatcher { fn default() -> Self { Self::new() } }

impl ActionDispatcher {
    pub fn new() -> Self { Self { registry: ActionRegistry::new() } }
    pub fn with_registry(mut self, registry: ActionRegistry) -> Self { self.registry = registry; self }
    pub fn registry(&self) -> &ActionRegistry { &self.registry }
    pub fn execute(&self, id: ActionId, ctx: &ActionContext) -> Result<ActionOutcome, ActionError> {
        let action = self.registry.get(id).ok_or(ActionError::UnknownAction(id))?;
        Ok(action.execute(ctx))
    }
    pub fn chain(&self, ids: &[ActionId], ctx: &ActionContext) -> Vec<ActionOutcome> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids { if let Ok(o) = self.execute(*id, ctx) { out.push(o); } }
        out
    }
    pub fn run_five_rails(&self, ctx: &ActionContext) -> Vec<ActionOutcome> {
        self.chain(&ActionId::ALL[..5], ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_eight_action_ids_match() {
        assert_eq!(ActionId::ALL.len(), 8);
        assert_eq!(ActionId::COUNT, 8);
        for id in ActionId::ALL {
            if (id as usize) < 5 {
                assert!(ActionKind::FIVE_GUARDRAILS_KINDS.contains(&id.kind()));
            }
        }
    }
    #[test] fn five_guardrails_kinds_unique() {
        assert_eq!(ActionKind::FIVE_GUARDRAILS_KINDS.len(), 5);
        assert_eq!(ActionKind::FIVE_GUARDRAILS_COUNT, 5);
    }
    #[test] fn kebab_names_unique() {
        let names: Vec<&str> = ActionId::ALL.iter().map(|id| id.kebab_name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 8);
    }
    #[test] fn action_registry_has_eight() {
        let r = ActionRegistry::new();
        assert_eq!(r.count(), 8);
        for id in ActionId::ALL { assert!(r.get(id).is_some()); }
    }
    #[test] fn input_rail_rejects_empty() {
        let d = ActionDispatcher::new();
        let ctx = ActionContext::new("");
        assert!(matches!(d.execute(ActionId::InputMultiAi, &ctx).unwrap(), ActionOutcome::Block { .. }));
    }
    #[test] fn input_rail_accepts_non_empty() {
        let d = ActionDispatcher::new();
        let ctx = ActionContext::new("hello");
        assert!(d.execute(ActionId::InputMultiAi, &ctx).unwrap().is_pass());
    }
    #[test] fn retrieval_rail_rewrites_empty() {
        let d = ActionDispatcher::new();
        let mut ctx = ActionContext::new("q");
        ctx.retrieved_chunks = vec!["".into(), "valid".into(), "".into()];
        assert!(matches!(d.execute(ActionId::RetrievalReflection, &ctx).unwrap(), ActionOutcome::Rewrite { .. }));
    }
    #[test] fn output_rail_rejects_empty_llm() {
        let d = ActionDispatcher::new();
        let mut ctx = ActionContext::new("q");
        ctx.llm_output = Some("".into());
        assert!(matches!(d.execute(ActionId::OutputMewg, &ctx).unwrap(), ActionOutcome::Block { .. }));
    }
    #[test] fn chain_executes_all_eight() {
        let d = ActionDispatcher::new();
        let ctx = ActionContext::new("hello");
        let out = d.chain(&ActionId::ALL, &ctx);
        assert_eq!(out.len(), 8);
    }
    #[test] fn run_five_rails_executes_five() {
        let d = ActionDispatcher::new();
        let ctx = ActionContext::new("hi");
        let out = d.run_five_rails(&ctx);
        assert_eq!(out.len(), 5);
    }
    #[test] fn by_kind_filters() {
        let r = ActionRegistry::new();
        let v = r.by_kind(ActionKind::Input);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], ActionId::InputMultiAi);
    }
    #[test] fn outcome_id_extraction() {
        let p = ActionOutcome::Pass { id: ActionId::InputMultiAi, name: "x".into() };
        assert_eq!(p.id(), ActionId::InputMultiAi);
    }
}
