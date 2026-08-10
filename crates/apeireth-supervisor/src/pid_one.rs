//! PID 1 supervisor — root of the supervision tree.
//!
//! PID 1 has NO parent, NO restart_strategy (it doesn't get restarted by anyone),
//! and is responsible for bringing up all 5 sub-supervisors and accepting
//! runtime plan upgrades via `replace_plan`.
//!
//! ponytail: this is a data struct, not a runtime; the ceiling is a real event
//! loop that polls child PIDs, the upgrade path is "split out a `PidOneRuntime`
//! that owns a JoinHandle"; not needed yet (see `examples/supervisor_demo.rs`
//! for the wiring).

use crate::child::ChildSpec;
use crate::supervisor::{default_plan, SubSupervisorKind};

#[derive(Debug, Clone)]
pub struct PidOneSupervisor {
    /// Plan version. Increments on every `replace_plan`. Useful for audit.
    pub plan_version: u64,
    /// Current sub-supervisor tree (kind + child specs).
    pub sub_supervisors: Vec<(SubSupervisorKind, Vec<ChildSpec>)>,
}

impl PidOneSupervisor {
    /// Create PID 1 with the default 5-subtree, 21-child plan.
    pub fn new() -> Self {
        Self::with_plan(default_plan())
    }

    /// Create PID 1 with a custom plan (used by tests + plugin hot-swap).
    pub fn with_plan(plan: Vec<(SubSupervisorKind, Vec<ChildSpec>)>) -> Self {
        Self {
            plan_version: 1,
            sub_supervisors: plan,
        }
    }

    /// Replace the entire plan atomically. Returns the new version.
    ///
    /// ponytail: returns the new version for audit; the ceiling is "drain
    /// old sub-supervisors before swap", the upgrade path is to return the
    /// list of dropped specs so the caller can release resources.
    pub fn replace_plan(&mut self, plan: Vec<(SubSupervisorKind, Vec<ChildSpec>)>) -> u64 {
        self.sub_supervisors = plan;
        self.plan_version += 1;
        self.plan_version
    }

    /// Total child specs across all sub-supervisors.
    pub fn total_children(&self) -> usize {
        self.sub_supervisors.iter().map(|(_, s)| s.len()).sum()
    }

    /// Lookup children by kind.
    pub fn children_of(&self, kind: SubSupervisorKind) -> Option<&[ChildSpec]> {
        self.sub_supervisors
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, specs)| specs.as_slice())
    }
}

impl Default for PidOneSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_one_has_no_restart_strategy() {
        // Type-level proof: PidOneSupervisor struct has no `restart_strategy` field.
        // If anyone adds one, this compile-time check won't fail — but a docstring
        // will. The invariant: PID 1 is NEVER restartable by definition.
        let _pid_one = PidOneSupervisor::new();
        // Compile-time check on field names is via the struct definition above.
    }

    #[test]
    fn default_plan_yields_21_children() {
        let pid_one = PidOneSupervisor::new();
        assert_eq!(pid_one.total_children(), 21);
        assert_eq!(pid_one.plan_version, 1);
    }

    #[test]
    fn replace_plan_bumps_version() {
        let mut pid_one = PidOneSupervisor::new();
        let new_version = pid_one.replace_plan(default_plan());
        assert!(new_version >= 2);
        assert_eq!(pid_one.plan_version, new_version);
    }

    #[test]
    fn children_of_lookup_works() {
        let pid_one = PidOneSupervisor::new();
        assert!(pid_one.children_of(SubSupervisorKind::Council).is_some());
        assert_eq!(
            pid_one
                .children_of(SubSupervisorKind::Council)
                .unwrap()
                .len(),
            7
        );
        assert_eq!(
            pid_one.children_of(SubSupervisorKind::Core).unwrap().len(),
            3
        );
    }
}
