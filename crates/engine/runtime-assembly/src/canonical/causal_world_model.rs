//! Causal World Model (因果世界模型) — Speculative Execution, CoW State Branching & SAGA Compensation.
//!
//! # Mathematical & Architectural Foundations
//!
//! Prevents destructive action side-effects in long-range agent loops:
//! - **Copy-On-Write (CoW) State Forking**: Before attempting risky actions, forks a state snapshot
//!   $S_{\text{branch}} = \text{fork}(S_0)$;
//! - **Speculative Hypothesis Validation**: Actions execute in speculative sandbox; evaluated by
//!   verification assertions. If passed, changes are fast-forward committed into the main world $S_0 \to S_1$;
//! - **SAGA Compensating Action Protocol**: For irreversible external tool effects (cloud provisioning,
//!   external API updates), maintains an inverted compensation transaction stack $\mathcal{T} = \langle A_i, A_i^{-1} \rangle$.
//!   On failure, rolls back the transaction chain deterministically in reverse order $A_{k-1}^{-1}, \dots, A_1^{-1}$.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Environmental state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub timestamp_secs: u64,
    pub file_checksums: HashMap<String, String>,
    pub environment_variables: HashMap<String, String>,
    pub state_flags: HashMap<String, bool>,
}

/// SAGA Compensating action definition for irreversible external operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SagaCompensatingAction {
    pub action_id: String,
    pub forward_action_name: String,
    pub compensation_action_name: String,
    pub payload: HashMap<String, String>,
}

/// Speculative hypothesis branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HypothesisBranch {
    pub branch_id: String,
    pub base_snapshot_id: String,
    pub modified_files: HashMap<String, String>,
    pub proposed_state_flags: HashMap<String, bool>,
    pub saga_stack: Vec<SagaCompensatingAction>,
    pub is_committed: bool,
    pub is_pruned: bool,
}

/// Verification result of speculative hypothesis execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchEvaluationOutcome {
    pub success: bool,
    pub confidence_score: f32,
    pub validation_message: String,
}

/// Causal World Model Manager.
#[derive(Debug, Clone)]
pub struct CausalWorldModel {
    snapshots: HashMap<String, WorldStateSnapshot>,
    branches: HashMap<String, HypothesisBranch>,
    active_main_snapshot_id: String,
}

impl CausalWorldModel {
    pub fn new(initial_snapshot_id: &str) -> Self {
        let root = WorldStateSnapshot {
            snapshot_id: initial_snapshot_id.to_string(),
            parent_snapshot_id: None,
            timestamp_secs: 0,
            file_checksums: HashMap::new(),
            environment_variables: HashMap::new(),
            state_flags: HashMap::new(),
        };

        let mut snapshots = HashMap::new();
        snapshots.insert(initial_snapshot_id.to_string(), root);

        Self {
            snapshots,
            branches: HashMap::new(),
            active_main_snapshot_id: initial_snapshot_id.to_string(),
        }
    }

    /// Gets the active main world snapshot.
    pub fn current_snapshot(&self) -> Option<&WorldStateSnapshot> {
        self.snapshots.get(&self.active_main_snapshot_id)
    }

    /// Forks a new speculative hypothesis branch from the current snapshot.
    pub fn fork_branch(&mut self, branch_id: &str) -> Result<&mut HypothesisBranch, String> {
        if self.branches.contains_key(branch_id) {
            return Err(format!("Branch '{branch_id}' already exists"));
        }

        let branch = HypothesisBranch {
            branch_id: branch_id.to_string(),
            base_snapshot_id: self.active_main_snapshot_id.clone(),
            modified_files: HashMap::new(),
            proposed_state_flags: HashMap::new(),
            saga_stack: Vec::new(),
            is_committed: false,
            is_pruned: false,
        };

        self.branches.insert(branch_id.to_string(), branch);
        Ok(self.branches.get_mut(branch_id).unwrap())
    }

    /// Records a speculative change in a hypothesis branch.
    pub fn record_speculative_write(
        &mut self,
        branch_id: &str,
        path: &str,
        checksum: &str,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch '{branch_id}' not found"))?;

        if branch.is_committed || branch.is_pruned {
            return Err(format!("Branch '{branch_id}' is closed"));
        }

        branch
            .modified_files
            .insert(path.to_string(), checksum.to_string());
        Ok(())
    }

    /// Registers a compensating action on the branch's SAGA stack.
    pub fn push_saga_compensation(
        &mut self,
        branch_id: &str,
        action: SagaCompensatingAction,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch '{branch_id}' not found"))?;

        branch.saga_stack.push(action);
        Ok(())
    }

    /// Commits a successful hypothesis branch into a new main world snapshot.
    pub fn commit_branch(
        &mut self,
        branch_id: &str,
        new_snapshot_id: &str,
        timestamp_secs: u64,
    ) -> Result<WorldStateSnapshot, String> {
        let branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch '{branch_id}' not found"))?;

        if branch.is_committed || branch.is_pruned {
            return Err(format!("Branch '{branch_id}' is not in active state"));
        }

        let base_snap = self
            .snapshots
            .get(&branch.base_snapshot_id)
            .ok_or_else(|| format!("Base snapshot '{}' not found", branch.base_snapshot_id))?
            .clone();

        let mut new_checksums = base_snap.file_checksums;
        for (path, hash) in &branch.modified_files {
            new_checksums.insert(path.clone(), hash.clone());
        }

        let mut new_flags = base_snap.state_flags;
        for (flag, val) in &branch.proposed_state_flags {
            new_flags.insert(flag.clone(), *val);
        }

        let new_snapshot = WorldStateSnapshot {
            snapshot_id: new_snapshot_id.to_string(),
            parent_snapshot_id: Some(base_snap.snapshot_id),
            timestamp_secs,
            file_checksums: new_checksums,
            environment_variables: base_snap.environment_variables,
            state_flags: new_flags,
        };

        self.snapshots
            .insert(new_snapshot_id.to_string(), new_snapshot.clone());
        self.active_main_snapshot_id = new_snapshot_id.to_string();
        branch.is_committed = true;

        Ok(new_snapshot)
    }

    /// Prunes/rolls back a failed branch, executing all SAGA compensations in reverse order.
    pub fn rollback_branch(
        &mut self,
        branch_id: &str,
    ) -> Result<Vec<SagaCompensatingAction>, String> {
        let branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch '{branch_id}' not found"))?;

        if branch.is_committed || branch.is_pruned {
            return Err(format!("Branch '{branch_id}' cannot be rolled back"));
        }

        branch.is_pruned = true;

        // Drain SAGA compensations in reverse (LIFO order)
        let mut compensations = Vec::new();
        while let Some(comp) = branch.saga_stack.pop() {
            compensations.push(comp);
        }

        Ok(compensations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_world_model_branch_commit() {
        let mut world = CausalWorldModel::new("snap_0");

        let branch = world.fork_branch("feature_optim").unwrap();
        assert_eq!(branch.base_snapshot_id, "snap_0");

        world
            .record_speculative_write("feature_optim", "src/main.rs", "hash_abc")
            .unwrap();

        let new_snap = world
            .commit_branch("feature_optim", "snap_1", 1000)
            .expect("Commit failed");

        assert_eq!(new_snap.snapshot_id, "snap_1");
        assert_eq!(new_snap.parent_snapshot_id, Some("snap_0".into()));
        assert_eq!(
            new_snap.file_checksums.get("src/main.rs"),
            Some(&"hash_abc".to_string())
        );
        assert_eq!(world.active_main_snapshot_id, "snap_1");
    }

    #[test]
    fn test_causal_world_model_rollback_and_saga_compensation() {
        let mut world = CausalWorldModel::new("snap_0");
        world.fork_branch("dangerous_action").unwrap();

        world
            .push_saga_compensation(
                "dangerous_action",
                SagaCompensatingAction {
                    action_id: "a1".into(),
                    forward_action_name: "create_bucket".into(),
                    compensation_action_name: "delete_bucket".into(),
                    payload: HashMap::new(),
                },
            )
            .unwrap();

        world
            .push_saga_compensation(
                "dangerous_action",
                SagaCompensatingAction {
                    action_id: "a2".into(),
                    forward_action_name: "insert_db_row".into(),
                    compensation_action_name: "delete_db_row".into(),
                    payload: HashMap::new(),
                },
            )
            .unwrap();

        // Rollback should return compensation in reverse order (a2 -> a1)
        let compensations = world.rollback_branch("dangerous_action").unwrap();
        assert_eq!(compensations.len(), 2);
        assert_eq!(compensations[0].action_id, "a2");
        assert_eq!(compensations[0].compensation_action_name, "delete_db_row");
        assert_eq!(compensations[1].action_id, "a1");
        assert_eq!(compensations[1].compensation_action_name, "delete_bucket");

        // Main snapshot remains intact at snap_0
        assert_eq!(world.active_main_snapshot_id, "snap_0");
    }
}
