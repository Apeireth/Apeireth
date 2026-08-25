//! Apeireth action organ (A11.1 landing — R14 stage 4).
//!
//! **Responsibility**: changing environment + tool execution + expression + silence
//! ("not-acting is also acting").
//!
//! **Architecture position**: stage-4 §3 main path 17-crate A11.1 organ
//! (after apeireth-cognition, before apeireth-motivation/value).
//!
//! **Current state**: A11.1 minimum viable landing (5+ pub fn, 5+ tests, 1+ integration test,
//! examples). Full tool bridge / sandbox-validator remain for A14/A19 deepening.
//!
//! **Honest registration**: per `leader-handover-final-2026-08-01` §B simplified implementation.
//! Complete action organ (12-key hardcode rejection, Mutex tx_log, four trait surfaces) deferred
//! to stage 5.
//!
//! **Prohibitions**:
//! - do NOT modify apeireth-core installed type signatures
//! - do NOT touch R11 baseline three values
//! - do NOT touch apeireth-legacy/

#![deny(unsafe_code)]

use thiserror::Error;

mod execution;
mod expression;
mod silence;

pub use execution::{ActionAtom, ActionEngine, ActionPlan, ExecutionResult, RollbackResult, TxId};
pub use expression::{ActionIntent, ExpressionChannel, StructuredOutput};
pub use silence::SilenceReason;

/// Top-level error: fallback for all action subsystems.
#[derive(Debug, Error)]
pub enum ActionError {
    /// Invalid input.
    #[error("invalid action input: {0}")]
    InvalidInput(String),
    /// Serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Unified result type.
pub type ActionResult<T> = Result<T, ActionError>;

/// Action execution trait (core trait 1/3 — stage-4 §3.3 Action + Execution merged).
///
/// Any concrete implementation that "changes the environment" must implement this trait.
/// Scheduler drives via `execute_plan` / `dispatch_atom` / `rollback_tx`.
pub trait ActionExecution: Send + Sync + 'static {
    /// Atomically execute an ActionPlan.
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult;
    /// Atomically execute a finer-grained ActionAtom.
    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult;
    /// Roll back by transaction ID (PHL-02b not_undo enforcement — rollback only applies to
    /// "the future"; landed side-effects are not undone).
    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult;
}

/// Action expression trait (core trait 2/3 — stage-4 §3.3 Expression).
///
/// Project internal intent to external channel (text / voice / multi-modal / structured).
pub trait ActionExpression: Send + Sync + 'static {
    /// Project intent to target channel.
    fn express(&self, intent: &ActionIntent, channel: ExpressionChannel) -> StructuredOutput;
    /// Convenience: default text channel.
    fn express_text(&self, intent: &ActionIntent) -> String {
        self.express(intent, ExpressionChannel::Text).text_payload()
    }
}

/// Action silence trait (core trait 3/3 — stage-4 §3.3 Silence).
///
/// **Not acting is also a legal action**. This trait explicitly acknowledges that silence is a
/// legal output of the action organ.
pub trait ActionSilence: Send + Sync + 'static {
    /// Determine whether the current intent should be silenced.
    fn should_silence(&self, intent: &ActionIntent) -> bool;
    /// Give a silence reason.
    fn reason_for_silence(&self, intent: &ActionIntent) -> SilenceReason;
}

/// Default aggregate entry: compose execution / expression / silence into one object.
///
/// Ponytail: no trait object factory — just provide a `DefaultActionEngine` struct that
/// satisfies "need a working instance" without unnecessary abstraction.
#[derive(Debug, Default)]
pub struct DefaultActionEngine {
    inner: ActionEngine,
}

impl DefaultActionEngine {
    /// Construct a default engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the inner engine reference.
    pub fn engine(&self) -> &ActionEngine {
        &self.inner
    }
}

impl ActionExecution for DefaultActionEngine {
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult {
        self.inner.execute_plan(plan)
    }
    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult {
        self.inner.dispatch_atom(atom)
    }
    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult {
        self.inner.rollback_tx(tx_id)
    }
}

impl ActionExpression for DefaultActionEngine {
    fn express(&self, intent: &ActionIntent, channel: ExpressionChannel) -> StructuredOutput {
        self.inner.express(intent, channel)
    }
}

impl ActionSilence for DefaultActionEngine {
    fn should_silence(&self, intent: &ActionIntent) -> bool {
        self.inner.should_silence(intent)
    }
    fn reason_for_silence(&self, intent: &ActionIntent) -> SilenceReason {
        self.inner.reason_for_silence(intent)
    }
}

/// Top-level convenience: execute a plan, returning ActionError on failure.
pub fn run_execute(
    engine: &dyn ActionExecution,
    plan: &ActionPlan,
) -> ActionResult<ExecutionResult> {
    plan.validate().map_err(ActionError::InvalidInput)?;
    Ok(engine.execute_plan(plan))
}

/// Top-level convenience: express an intent.
pub fn run_express(
    engine: &dyn ActionExpression,
    intent: &ActionIntent,
    channel: ExpressionChannel,
) -> StructuredOutput {
    engine.express(intent, channel)
}

/// Top-level convenience: decide silence and return the reason.
pub fn run_silence(engine: &dyn ActionSilence, intent: &ActionIntent) -> SilenceReason {
    if engine.should_silence(intent) {
        engine.reason_for_silence(intent)
    } else {
        SilenceReason::NotSilent
    }
}

/// Utility: determine whether a plan is executable (non-empty + not 13-key blocked).
///
/// 13-key hardcode rejection (PHL-07 no-pretend): ModifyL0HA / ReorganizeOnion /
/// ModifyEvolutionL0 are never executable.
pub fn is_actionable(plan: &ActionPlan) -> bool {
    !plan.steps.is_empty()
        && !matches!(
            plan.target,
            apeireth_core::ActionTarget::ModifyL0HA
                | apeireth_core::ActionTarget::ReorganizeOnion
                | apeireth_core::ActionTarget::ModifyEvolutionL0
        )
}

/// Utility: allocate a new TxId (UUID-backed).
pub fn new_tx_id() -> TxId {
    TxId(uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::ActionTarget;

    fn safe_target() -> ActionTarget {
        ActionTarget::NormalAction("noop".to_string())
    }

    #[test]
    fn action_plan_validate_rejects_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn action_plan_validate_accepts_non_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["step1".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn is_actionable_rejects_modify_l0_ha() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: ActionTarget::ModifyL0HA,
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_rejects_reorganize_onion() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: ActionTarget::ReorganizeOnion,
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_rejects_modify_evolution_l0() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: ActionTarget::ModifyEvolutionL0,
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_rejects_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_accepts_normal_action_with_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(is_actionable(&plan));
    }

    #[test]
    fn new_tx_id_is_unique() {
        let a = new_tx_id();
        let b = new_tx_id();
        assert_ne!(a, b);
    }

    #[test]
    fn run_execute_returns_invalid_input_for_empty_steps() {
        let engine = DefaultActionEngine::new();
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        let res = run_execute(&engine, &plan);
        assert!(matches!(res, Err(ActionError::InvalidInput(_))));
    }

    #[test]
    fn run_execute_returns_applied_for_valid_plan() {
        let engine = DefaultActionEngine::new();
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        let res = run_execute(&engine, &plan).expect("execute ok");
        assert!(matches!(res, ExecutionResult::Applied(_)));
    }

    #[test]
    fn run_express_returns_structured_output() {
        let engine = DefaultActionEngine::new();
        let intent = ActionIntent::new(safe_target()).with_body_hint("hi");
        let out = run_express(&engine, &intent, ExpressionChannel::Text);
        assert_eq!(out.channel, ExpressionChannel::Text);
        assert_eq!(out.text_payload(), "hi");
    }

    #[test]
    fn run_silence_returns_not_silent_for_normal_action() {
        let engine = DefaultActionEngine::new();
        let intent = ActionIntent::new(safe_target());
        let r = run_silence(&engine, &intent);
        assert_eq!(r, SilenceReason::NotSilent);
    }

    #[test]
    fn run_silence_returns_ethical_for_l0() {
        let engine = DefaultActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::ModifyL0HA);
        let r = run_silence(&engine, &intent);
        assert_eq!(r, SilenceReason::EthicalDoubt);
    }

    #[test]
    fn default_engine_dispatches_through_all_three_traits() {
        let engine = DefaultActionEngine::new();
        // ActionExecution
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "x".to_string(),
        };
        assert!(matches!(
            engine.execute_plan(&plan),
            ExecutionResult::Applied(_)
        ));
        // ActionExpression
        let intent = ActionIntent::new(safe_target());
        let out = engine.express(&intent, ExpressionChannel::Text);
        assert_eq!(out.channel, ExpressionChannel::Text);
        // ActionSilence
        assert!(!engine.should_silence(&intent));
        assert_eq!(engine.reason_for_silence(&intent), SilenceReason::NotSilent);
    }

    #[test]
    fn express_text_convenience_returns_text_payload() {
        let engine = DefaultActionEngine::new();
        let intent = ActionIntent::new(safe_target()).with_body_hint("hello world");
        let s = engine.express_text(&intent);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn default_action_engine_engine_accessor() {
        let engine = DefaultActionEngine::new();
        let _ = engine.engine(); // should not panic
        assert_eq!(engine.engine().tx_count(), 0);
    }

    #[test]
    fn action_error_from_serde_json() {
        let bad_json = "not valid json";
        let err: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
        let action_err: ActionError = err.unwrap_err().into();
        assert!(matches!(action_err, ActionError::Json(_)));
    }
}
