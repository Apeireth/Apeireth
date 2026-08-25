//! flow_executor: 流程执行器

use crate::action_rail::{ActionContext, ActionDispatcher, ActionOutcome};
use crate::colang_dsl::{ColangElementKind, ParsedColangFile};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowState {
    Idle, Running, Paused, Done, Failed,
}

impl FlowState {
    pub fn is_terminal(&self) -> bool { matches!(self, Self::Done | Self::Failed) }
    pub fn is_pending(&self) -> bool { matches!(self, Self::Paused) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowStep {
    UserSay, BotSay, When, ElseWhen, If, Else, Goto, Run, Do, Set,
    Allow, Disallow, Stop, Abort, Return, Pass, Log,
}

impl FlowStep {
    pub fn from_colang_kind(kind: ColangElementKind) -> Option<Self> {
        match kind {
            ColangElementKind::UserSay => Some(FlowStep::UserSay),
            ColangElementKind::BotSay => Some(FlowStep::BotSay),
            ColangElementKind::When => Some(FlowStep::When),
            ColangElementKind::ElseWhen => Some(FlowStep::ElseWhen),
            ColangElementKind::If => Some(FlowStep::If),
            ColangElementKind::Else => Some(FlowStep::Else),
            ColangElementKind::Goto | ColangElementKind::GotoAlias => Some(FlowStep::Goto),
            ColangElementKind::Run => Some(FlowStep::Run),
            ColangElementKind::Do => Some(FlowStep::Do),
            ColangElementKind::Set => Some(FlowStep::Set),
            ColangElementKind::Allow => Some(FlowStep::Allow),
            ColangElementKind::Disallow => Some(FlowStep::Disallow),
            ColangElementKind::Stop => Some(FlowStep::Stop),
            ColangElementKind::Abort => Some(FlowStep::Abort),
            ColangElementKind::Return => Some(FlowStep::Return),
            ColangElementKind::Pass => Some(FlowStep::Pass),
            ColangElementKind::Log => Some(FlowStep::Log),
            _ => None,
        }
    }
    pub const COUNT: usize = 17;
    pub const ALL: [FlowStep; 17] = [
        FlowStep::UserSay, FlowStep::BotSay, FlowStep::When, FlowStep::ElseWhen, FlowStep::If, FlowStep::Else,
        FlowStep::Goto, FlowStep::Run, FlowStep::Do, FlowStep::Set, FlowStep::Allow, FlowStep::Disallow,
        FlowStep::Stop, FlowStep::Abort, FlowStep::Return, FlowStep::Pass, FlowStep::Log,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FlowOutcome {
    Completed { state: FlowState, step_count: usize, action_outcomes: Vec<ActionOutcome> },
    Blocked { at_step: FlowStep, reason: String, steps_executed: usize },
    Paused { at_step: FlowStep, state: String, steps_executed: usize },
    Failed { error: String, steps_executed: usize },
}

impl FlowOutcome {
    pub fn is_completed(&self) -> bool { matches!(self, Self::Completed { .. }) }
    pub fn state(&self) -> FlowState {
        match self {
            Self::Completed { state, .. } => *state,
            Self::Blocked { .. } => FlowState::Failed,
            Self::Paused { .. } => FlowState::Paused,
            Self::Failed { .. } => FlowState::Failed,
        }
    }
}

#[derive(Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum FlowError {
    #[error("Empty flow file: {0}")]
    EmptyFile(String),
    #[error("Unknown flow: {0}")]
    UnknownFlow(String),
    #[error("Step execution failed at {step:?}: {reason}")]
    StepFailed { step: FlowStep, reason: String },
}

pub struct FlowRunner<'a> {
    pub parsed: &'a ParsedColangFile,
    pub dispatcher: &'a ActionDispatcher,
    state: FlowState,
    step_count: usize,
}

impl<'a> FlowRunner<'a> {
    pub fn new(parsed: &'a ParsedColangFile, dispatcher: &'a ActionDispatcher) -> Self {
        Self { parsed, dispatcher, state: FlowState::Idle, step_count: 0 }
    }
    pub fn state(&self) -> FlowState { self.state }
    pub fn step_count(&self) -> usize { self.step_count }

    pub fn run_flow(&mut self, flow_name: &str) -> Result<FlowOutcome, FlowError> {
        let flow_define = self.parsed.flow_defines.iter().find(|(name, _)| name == flow_name)
            .ok_or_else(|| FlowError::UnknownFlow(flow_name.to_string()))?;
        let flow_line = flow_define.1;
        let flow_struct = self.parsed.defines.iter().find(|d| d.line == flow_line && matches!(d.kind, ColangElementKind::DefineFlow))
            .ok_or_else(|| FlowError::UnknownFlow(flow_name.to_string()))?;
        if flow_struct.elements.is_empty() {
            return Err(FlowError::EmptyFile(format!("flow '{}' has no elements", flow_name)));
        }
        self.state = FlowState::Running;
        let mut action_outcomes: Vec<ActionOutcome> = Vec::new();
        for element in &flow_struct.elements {
            let Some(step) = FlowStep::from_colang_kind(element.kind) else { continue; };
            self.step_count += 1;
            match step {
                FlowStep::Stop | FlowStep::Abort => {
                    self.state = FlowState::Failed;
                    return Ok(FlowOutcome::Blocked { at_step: step, reason: format!("Flow aborted at step #{}", self.step_count), steps_executed: self.step_count });
                }
                FlowStep::Allow => continue,
                FlowStep::Disallow => {
                    self.state = FlowState::Failed;
                    return Ok(FlowOutcome::Blocked { at_step: step, reason: format!("Flow disallowed at step #{}", self.step_count), steps_executed: self.step_count });
                }
                FlowStep::Return => {
                    self.state = FlowState::Done;
                    return Ok(FlowOutcome::Completed { state: FlowState::Done, step_count: self.step_count, action_outcomes });
                }
                FlowStep::Pass => continue,
                _ => {}
            }
            if matches!(step, FlowStep::Run | FlowStep::Do) {
                let ctx = ActionContext::new(format!("flow:{}:step#{}", flow_name, self.step_count));
                let outcomes = self.dispatcher.run_five_rails(&ctx);
                action_outcomes.extend(outcomes);
            }
        }
        self.state = FlowState::Done;
        Ok(FlowOutcome::Completed { state: FlowState::Done, step_count: self.step_count, action_outcomes })
    }
}

pub struct FlowExecutor<'a> {
    pub dispatcher: &'a ActionDispatcher,
    flows_executed: usize,
    state: FlowState,
}

impl<'a> FlowExecutor<'a> {
    pub fn new(dispatcher: &'a ActionDispatcher) -> Self {
        Self { dispatcher, flows_executed: 0, state: FlowState::Idle }
    }
    pub fn state(&self) -> FlowState { self.state }
    pub fn flows_executed(&self) -> usize { self.flows_executed }

    pub fn run_flows(&mut self, parsed: &ParsedColangFile, flow_names: &[&str]) -> Vec<FlowOutcome> {
        self.state = FlowState::Running;
        let mut outcomes = Vec::with_capacity(flow_names.len());
        for name in flow_names {
            let mut runner = FlowRunner::new(parsed, self.dispatcher);
            match runner.run_flow(name) {
                Ok(outcome) => { self.flows_executed += 1; outcomes.push(outcome); }
                Err(_) => {}
            }
        }
        if outcomes.iter().all(|o| o.is_completed()) { self.state = FlowState::Done; }
        else if outcomes.iter().any(|o| o.state() == FlowState::Paused) { self.state = FlowState::Paused; }
        else { self.state = FlowState::Failed; }
        outcomes
    }

    pub fn run_all_flows(&mut self, parsed: &ParsedColangFile) -> Vec<FlowOutcome> {
        let flow_names: Vec<&str> = parsed.flow_defines.iter().map(|(name, _)| name.as_str()).collect();
        self.run_flows(parsed, &flow_names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_rail::{ActionId, ActionRegistry};
    use crate::colang_dsl::ColangParser;

    #[test] fn flow_state_terminal_predicate() {
        assert!(FlowState::Done.is_terminal());
        assert!(FlowState::Failed.is_terminal());
        assert!(!FlowState::Running.is_terminal());
        assert!(FlowState::Paused.is_pending());
    }
    #[test] fn flow_step_count_17() {
        assert_eq!(FlowStep::COUNT, 17);
        assert_eq!(FlowStep::ALL.len(), 17);
    }
    #[test] fn flow_step_from_colang_kind() {
        assert_eq!(FlowStep::from_colang_kind(ColangElementKind::UserSay), Some(FlowStep::UserSay));
        assert_eq!(FlowStep::from_colang_kind(ColangElementKind::DefineUser), None);
    }
    #[test] fn simple_colang_flow_runs() {
        let src = "\ndefine user express greeting\n  \"hello\"\n  \"hi\"\n\ndefine flow greeting\n  user express greeting\n  bot express greeting\n  allow\n";
        let parsed = ColangParser::new("t.co", src).parse().unwrap();
        let d = ActionDispatcher::new();
        let mut r = FlowRunner::new(&parsed, &d);
        let out = r.run_flow("greeting").unwrap();
        assert!(out.is_completed());
        assert_eq!(r.step_count(), 3);
    }
    #[test] fn abort_terminates() {
        let src = "\ndefine flow abort_test\n  user express greeting\n  abort\n  bot express greeting\n";
        let parsed = ColangParser::new("t.co", src).parse().unwrap();
        let d = ActionDispatcher::new();
        let mut r = FlowRunner::new(&parsed, &d);
        let out = r.run_flow("abort_test").unwrap();
        assert!(matches!(out, FlowOutcome::Blocked { .. }));
        assert_eq!(out.state(), FlowState::Failed);
    }
    #[test] fn unknown_flow_error() {
        let src = "\ndefine user express greeting\n  \"hello\"\n";
        let parsed = ColangParser::new("t.co", src).parse().unwrap();
        let d = ActionDispatcher::new();
        let mut r = FlowRunner::new(&parsed, &d);
        assert!(matches!(r.run_flow("nope"), Err(FlowError::UnknownFlow(_))));
    }
    #[test] fn flow_executor_runs_multiple() {
        let src = "\ndefine user express greeting\n  \"hello\"\n\ndefine flow greeting\n  user express greeting\n  bot express greeting\n  allow\n\ndefine flow farewell\n  user say goodbye\n  bot say goodbye\n  allow\n";
        let parsed = ColangParser::new("t.co", src).parse().unwrap();
        let d = ActionDispatcher::new();
        let mut ex = FlowExecutor::new(&d);
        let out = ex.run_flows(&parsed, &["greeting", "farewell"]);
        assert_eq!(out.len(), 2);
        assert_eq!(ex.flows_executed(), 2);
    }
    #[test] fn flow_executor_run_all() {
        let src = "\ndefine user express greeting\n  \"hello\"\n\ndefine flow greeting\n  user express greeting\n  allow\n\ndefine flow farewell\n  bot say goodbye\n  allow\n";
        let parsed = ColangParser::new("t.co", src).parse().unwrap();
        let d = ActionDispatcher::new();
        let mut ex = FlowExecutor::new(&d);
        let out = ex.run_all_flows(&parsed);
        assert_eq!(out.len(), 2);
    }
    #[test] fn v8_complete() {
        assert_eq!(ActionId::ALL.len(), 8);
        assert_eq!(FlowStep::ALL.len(), 17);
        let _states = [FlowState::Idle, FlowState::Running, FlowState::Paused, FlowState::Done, FlowState::Failed];
        assert_eq!(_states.len(), 5);
    }
}
