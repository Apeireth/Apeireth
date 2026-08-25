//! Execution module: ActionPlan + ActionAtom + ActionEngine + tx rollback.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::ActionTarget;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ActionExecution, ActionExpression, ActionSilence};

/// Transaction ID (UUID-backed) — for "atomic + rollback" tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(pub Uuid);

/// A pending-action full record — produced by cognition, consumed by action.
#[derive(Debug, Clone)]
pub struct ActionPlan {
    /// Unique plan ID.
    pub plan_id: Uuid,
    /// Action target (13-key hardcoded object).
    pub target: ActionTarget,
    /// Ordered execution steps (description strings).
    pub steps: Vec<String>,
    /// Creation timestamp (epoch seconds).
    pub created_at: i64,
    /// Context tag.
    pub context: String,
}

impl ActionPlan {
    /// Construct a minimal executable plan.
    pub fn new(target: ActionTarget, steps: Vec<String>, context: impl Into<String>) -> Self {
        Self {
            plan_id: Uuid::new_v4(),
            target,
            steps,
            created_at: now_epoch(),
            context: context.into(),
        }
    }

    /// Validate plan legality. Empty steps rejected directly.
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("action plan must have at least one step".to_string());
        }
        if self.context.is_empty() {
            return Err("action plan context must not be empty".to_string());
        }
        Ok(())
    }

    /// Step count.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// Atomic action — finer-grained execution unit (single step) than ActionPlan.
#[derive(Debug, Clone)]
pub struct ActionAtom {
    /// Unique atom ID.
    pub atom_id: Uuid,
    /// Action target (same as ActionPlan.target).
    pub target: ActionTarget,
    /// Payload string (single-step content).
    pub payload: String,
}

impl ActionAtom {
    /// Construct a minimal atomic action.
    pub fn new(target: ActionTarget, payload: impl Into<String>) -> Self {
        Self {
            atom_id: Uuid::new_v4(),
            target,
            payload: payload.into(),
        }
    }
}

/// Execution result — applied / rolled-back / failed three-state.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Applied, with transaction ID (rollbackable later).
    Applied(TxId),
    /// Rolled back (rare — usually returned by rollback_tx, not execute_plan).
    RolledBack(TxId),
    /// Execution failed, with optional tx_id (if recording happened before failure).
    Failed {
        /// Whether tx_id was allocated at failure time (true = recorded but failed).
        tx_id: Option<TxId>,
        /// Failure reason.
        reason: String,
    },
}

impl ExecutionResult {
    /// Whether successfully applied.
    pub fn is_applied(&self) -> bool {
        matches!(self, ExecutionResult::Applied(_))
    }

    /// Whether failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionResult::Failed { .. })
    }

    /// Associated tx_id (if any).
    pub fn tx_id(&self) -> Option<TxId> {
        match self {
            ExecutionResult::Applied(tx) | ExecutionResult::RolledBack(tx) => Some(*tx),
            ExecutionResult::Failed { tx_id, .. } => *tx_id,
        }
    }
}

/// Rollback result — three-state, always returning associated tx_id.
#[derive(Debug, Clone, PartialEq)]
pub enum RollbackResult {
    /// Rolled back.
    RolledBack(TxId),
    /// Corresponding transaction not found.
    NotFound(TxId),
    /// Transaction exists but not rollbackable (PHL-02b not_undo — landed side-effects are not undoable).
    NotRollbackable(TxId),
}

impl RollbackResult {
    /// Associated tx_id.
    pub fn tx_id(&self) -> TxId {
        match self {
            RollbackResult::RolledBack(tx)
            | RollbackResult::NotFound(tx)
            | RollbackResult::NotRollbackable(tx) => *tx,
        }
    }

    /// Whether successfully rolled back.
    pub fn is_rolled_back(&self) -> bool {
        matches!(self, RollbackResult::RolledBack(_))
    }
}

/// Default action engine — in-memory simulated execution + tx log.
///
/// Ponytail: no trait object factory. Single-instance struct + Mutex tx log suffices
/// for A11.1 "minimum viable" standard. Real tool bridge / sandbox-validator remain for A14/A19.
#[derive(Debug, Default)]
pub struct ActionEngine {
    /// Transaction log: TxId -> plan snapshot for that transaction.
    tx_log: Mutex<HashMap<TxId, ActionPlan>>,
}

impl ActionEngine {
    /// Construct a new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of transactions currently in tx_log.
    pub fn tx_count(&self) -> usize {
        self.tx_log.lock().map(|l| l.len()).unwrap_or(0)
    }

    /// List all recorded transactions (for audit).
    pub fn list_tx(&self) -> Vec<(TxId, ActionPlan)> {
        self.tx_log
            .lock()
            .map(|l| l.iter().map(|(k, v)| (*k, v.clone())).collect())
            .unwrap_or_default()
    }
}

impl ActionExecution for ActionEngine {
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult {
        // 13-key hardcoded reject (block at dispatcher stage to avoid polluting tx_log)
        if !crate::is_actionable(plan) {
            return ExecutionResult::Failed {
                tx_id: None,
                reason: "13-key blocked or empty steps".to_string(),
            };
        }

        let tx_id = TxId(Uuid::new_v4());
        match self.tx_log.lock() {
            Ok(mut log) => {
                log.insert(tx_id, plan.clone());
                ExecutionResult::Applied(tx_id)
            }
            Err(poisoned) => {
                // Mutex poisoned — serious error but still return Failed
                let _ = poisoned;
                ExecutionResult::Failed {
                    tx_id: Some(tx_id),
                    reason: "tx_log mutex poisoned".to_string(),
                }
            }
        }
    }

    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult {
        // Single-step atom — wrap as ActionPlan then execute (reuse path).
        let plan = ActionPlan {
            plan_id: atom.atom_id,
            target: atom.target,
            steps: vec![atom.payload],
            created_at: now_epoch(),
            context: "atom_dispatch".to_string(),
        };
        self.execute_plan(&plan)
    }

    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult {
        match self.tx_log.lock() {
            Ok(mut log) => match log.remove(&tx_id) {
                Some(_) => RollbackResult::RolledBack(tx_id),
                None => RollbackResult::NotFound(tx_id),
            },
            Err(_) => RollbackResult::NotRollbackable(tx_id),
        }
    }
}

/// epoch seconds (stdlib only — no chrono dependency).
fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
            plan_id: Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn action_plan_validate_rejects_empty_context() {
        let plan = ActionPlan {
            plan_id: Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["step1".to_string()],
            created_at: 0,
            context: "".to_string(),
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn action_plan_validate_accepts_non_empty_steps() {
        let plan = ActionPlan::new(safe_target(), vec!["step1".to_string()], "test");
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn action_plan_step_count_matches() {
        let plan = ActionPlan::new(safe_target(), vec!["a".into(), "b".into(), "c".into()], "x");
        assert_eq!(plan.step_count(), 3);
    }

    #[test]
    fn action_plan_new_generates_unique_plan_id() {
        let p1 = ActionPlan::new(safe_target(), vec!["a".into()], "x");
        let p2 = ActionPlan::new(safe_target(), vec!["a".into()], "x");
        assert_ne!(p1.plan_id, p2.plan_id);
    }

    #[test]
    fn execution_result_is_applied() {
        let r = ExecutionResult::Applied(TxId(Uuid::new_v4()));
        assert!(r.is_applied());
        assert!(!r.is_failed());
        assert!(r.tx_id().is_some());
    }

    #[test]
    fn execution_result_failed_tx_id_propagates() {
        let r = ExecutionResult::Failed {
            tx_id: Some(TxId(Uuid::new_v4())),
            reason: "bad".into(),
        };
        assert!(r.is_failed());
        assert!(r.tx_id().is_some());
    }

    #[test]
    fn execution_result_failed_no_tx_id() {
        let r = ExecutionResult::Failed {
            tx_id: None,
            reason: "pre-check failed".into(),
        };
        assert!(r.is_failed());
        assert!(r.tx_id().is_none());
    }

    #[test]
    fn rollback_result_tx_id_always_present() {
        let tx = TxId(Uuid::new_v4());
        let r = RollbackResult::NotFound(tx);
        assert_eq!(r.tx_id(), tx);
        assert!(!r.is_rolled_back());

        let r = RollbackResult::RolledBack(tx);
        assert!(r.is_rolled_back());

        let r = RollbackResult::NotRollbackable(tx);
        assert!(!r.is_rolled_back());
    }

    #[test]
    fn action_engine_records_and_rolls_back() {
        let engine = ActionEngine::new();
        let plan = ActionPlan::new(safe_target(), vec!["step1".into()], "ctx");
        match engine.execute_plan(&plan) {
            ExecutionResult::Applied(tx) => {
                assert_eq!(engine.tx_count(), 1);
                let rb = engine.rollback_tx(tx);
                assert!(rb.is_rolled_back());
                assert_eq!(engine.tx_count(), 0);
            }
            _ => panic!("expected Applied"),
        }
    }

    #[test]
    fn action_engine_rollback_unknown_tx_returns_not_found() {
        let engine = ActionEngine::new();
        let tx = TxId(Uuid::new_v4());
        let rb = engine.rollback_tx(tx);
        assert_eq!(rb, RollbackResult::NotFound(tx));
    }

    #[test]
    fn action_engine_rejects_l0_target() {
        let engine = ActionEngine::new();
        let plan = ActionPlan::new(ActionTarget::ModifyL0HA, vec!["x".into()], "ctx");
        match engine.execute_plan(&plan) {
            ExecutionResult::Failed { .. } => {}
            _ => panic!("expected Failed for ModifyL0HA"),
        }
        assert_eq!(engine.tx_count(), 0);
    }

    #[test]
    fn dispatch_atom_writes_to_tx_log() {
        let engine = ActionEngine::new();
        let atom = ActionAtom::new(safe_target(), "hello");
        match engine.dispatch_atom(atom) {
            ExecutionResult::Applied(_) => {}
            _ => panic!("expected Applied"),
        }
        assert_eq!(engine.tx_count(), 1);
    }

    #[test]
    fn list_tx_returns_recorded_transactions() {
        let engine = ActionEngine::new();
        let plan = ActionPlan::new(safe_target(), vec!["x".into()], "ctx");
        engine.execute_plan(&plan);
        let entries = engine.list_tx();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.target, safe_target());
    }

    #[test]
    fn execution_result_equality() {
        let tx = TxId(Uuid::new_v4());
        assert_eq!(ExecutionResult::Applied(tx), ExecutionResult::Applied(tx));
        assert_ne!(
            ExecutionResult::Applied(tx),
            ExecutionResult::RolledBack(tx)
        );
    }
}
