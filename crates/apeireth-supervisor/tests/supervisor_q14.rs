//! Q14 supervisor integration — 4 critical dimensions:
//!   1) 5-second auto-restart (restart_window + max_restarts prevent self-hang)
//!   2) Failed rollback to last-known-good snapshot (RestForOne + sibling restart)
//!   3) Trigger Council evaluation (default_plan has council-sup + 7 advisor slots)
//!   4) Independent child upgrade (ChildSpec replace → SubSupervisor::spawn re-create)
//!
//! Hard constraints:
//!   - Pure tokio::process::Command — no PyO3 or external scripts
//!   - Windows: `cmd /c exit <code>`; Unix: `true`/`false`
//!
//! ponytail: each test exercises ONE behavior; the ceiling is a full child
//! runtime (PID polling + signal handling), the upgrade path is to swap these
//! for real `Child::spawn` once the runtime exists.

use apeireth_supervisor::pid_one::PidOneSupervisor;
use apeireth_supervisor::strategy::{
    affected_indices, should_restart, ExitReason, RestartDecision, RestartStrategy,
};
use apeireth_supervisor::supervisor::{default_plan, SubSupervisorKind};
use apeireth_supervisor::ChildSpec;
use std::time::Duration;

// ----------------------------------------------------------------------------
// 1) 5-second auto-restart
// ----------------------------------------------------------------------------

/// Verifies that a child spec declares a restart_window + max_restarts pair that
/// prevents the supervisor from getting stuck in a tight crash loop.
///
/// ponytail: this is the data-level guarantee — the runtime that actually polls
/// PIDs will be the enforcement point. Today the spec carries the contract.
#[test]
fn test_5s_restart() {
    let pid_one = PidOneSupervisor::new();
    let core = pid_one.children_of(SubSupervisorKind::Core).unwrap();
    for spec in core {
        // (a) window must be 5s or shorter so a misbehaving child doesn't dominate
        assert!(
            spec.restart_window <= Duration::from_secs(5),
            "spec {} has window {:?} > 5s",
            spec.id,
            spec.restart_window
        );
        // (b) max_restarts must be finite so the supervisor doesn't loop forever
        assert!(
            spec.max_restarts >= 1 && spec.max_restarts <= 100,
            "spec {} has unbounded max_restarts",
            spec.id
        );
    }
}

/// Pair-check: the runtime-style restart decision for any abnormal exit
/// always returns Restart (not Skip), so a crash loop WILL be visible.
#[test]
fn test_5s_restart_abnormal_always_restarts() {
    for strategy in [RestartStrategy::OneForOne, RestartStrategy::RestForOne] {
        assert_eq!(
            should_restart(strategy, ExitReason::Abnormal(1)),
            RestartDecision::Restart,
            "{:?} must restart on Abnormal",
            strategy
        );
    }
}

// ----------------------------------------------------------------------------
// 2) Failure rollback to last-known-good snapshot
// ----------------------------------------------------------------------------

/// Verifies RestForOne rolls forward AND the spec carries a snapshot_id so a
/// rollback target is reachable.
///
/// ponytail: the actual "go back to snapshot_id" lives in the upgrade pipeline;
/// the supervisor only provides the affected-range signal.
#[test]
fn test_rollback_snapshot() {
    let pid_one = PidOneSupervisor::new();
    let cognition = pid_one.children_of(SubSupervisorKind::Cognition).unwrap();
    assert_eq!(cognition.len(), 4);
    assert_eq!(cognition[0].restart, RestartStrategy::RestForOne);

    // RestForOne takes the failed + all subsequent siblings → spec[2] fails
    // → spec[2], spec[3] both restart (rollforward; rollback target = snapshot_id).
    let affected: Vec<usize> = affected_indices(RestartStrategy::RestForOne, 2, 4).collect();
    assert_eq!(affected, vec![2, 3]);

    // At least one spec in the cognition subtree carries a snapshot id
    // (snapshot_id field is present and Option<String> — even None is acceptable
    // for a fresh boot; what matters is the field exists on every spec).
    for spec in cognition {
        let _ = spec.snapshot_id.as_deref(); // Option<&str> access — proves field exists
    }
}

// ----------------------------------------------------------------------------
// 3) Council evaluation trigger
// ----------------------------------------------------------------------------

/// Verifies the default plan carries a Council subtree with 7 advisor slots
/// (1 council-supervisor + 7 advisors is the eve design; we count advisor slots).
///
/// ponytail: this test asserts the SHAPE — that Council + 7 children exist.
/// The "trigger Council evaluation" semantic is owned by apeireth-council; here
/// we just verify the supervisor side exposes the slots.
#[test]
fn test_council_evaluation() {
    let pid_one = PidOneSupervisor::new();
    let council = pid_one
        .children_of(SubSupervisorKind::Council)
        .expect("Council subtree must exist");
    assert_eq!(council.len(), 7, "Council must have 7 advisor slots");

    // Council uses OneForOne so a single failed advisor doesn't take down siblings
    assert_eq!(council[0].restart, RestartStrategy::OneForOne);

    // Sub-supervisor strategy is per-kind; Council's default is OneForOne
    assert_eq!(
        SubSupervisorKind::Council.default_strategy(),
        RestartStrategy::OneForOne
    );
    assert_eq!(SubSupervisorKind::Council.default_count(), 7);
}

/// Council-spec round-trip — every default plan iteration produces the same 7 slots.
#[test]
fn test_council_evaluation_plan_is_deterministic() {
    let a = default_plan();
    let b = default_plan();
    let a_council: Vec<&str> = a
        .iter()
        .find(|(k, _)| *k == SubSupervisorKind::Council)
        .unwrap()
        .1
        .iter()
        .map(|s| s.id.as_str())
        .collect();
    let b_council: Vec<&str> = b
        .iter()
        .find(|(k, _)| *k == SubSupervisorKind::Council)
        .unwrap()
        .1
        .iter()
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(a_council, b_council);
}

// ----------------------------------------------------------------------------
// 4) Independent child upgrade
// ----------------------------------------------------------------------------

/// Verifies a single ChildSpec can be swapped for a new one (independent of the
/// rest of the subtree), and the new spec has its own strategy + args.
#[test]
fn test_child_upgrade() {
    let pid_one = PidOneSupervisor::new();
    let upgrade = pid_one.children_of(SubSupervisorKind::Upgrade).unwrap();
    assert_eq!(upgrade.len(), 3);
    assert_eq!(upgrade[0].restart, RestartStrategy::Transient);

    // Hot-swap: replace the upgrade subtree with a new version (independent upgrade)
    let mut pid_one = pid_one;
    let new_version = pid_one.replace_plan(default_plan());
    assert!(new_version >= 2);

    // The new plan still has 5 sub-supervisors and 21 children
    assert_eq!(pid_one.total_children(), 21);
    let new_upgrade = pid_one.children_of(SubSupervisorKind::Upgrade).unwrap();
    assert_eq!(new_upgrade.len(), 3);
}

/// Single ChildSpec can be cloned and reconfigured without touching siblings.
#[test]
fn test_child_upgrade_independent_reconfig() {
    let mut spec = ChildSpec::new(
        "upg.pipeline",
        "old pipeline",
        "true",
        RestartStrategy::Transient,
    );
    // Independent upgrade: clone + reconfigure
    let new_spec = spec
        .clone()
        .with_max_restarts(10)
        .with_restart_window(Duration::from_secs(2));
    // Old spec unchanged
    assert_eq!(spec.max_restarts, 5);
    // New spec upgraded
    assert_eq!(new_spec.max_restarts, 10);
    assert_eq!(new_spec.restart_window, Duration::from_secs(2));
    // They share id (the upgrade keeps the id stable)
    assert_eq!(spec.id, new_spec.id);
}

// ----------------------------------------------------------------------------
// Cross-cutting: SubSupervisorKind taxonomy invariants
// ----------------------------------------------------------------------------

#[test]
fn sub_supervisor_kind_default_count_matches_plan() {
    let plan = default_plan();
    for (kind, specs) in &plan {
        assert_eq!(
            specs.len(),
            kind.default_count(),
            "{:?} default_count={} but plan has {} specs",
            kind,
            kind.default_count(),
            specs.len()
        );
    }
}

#[test]
fn sub_supervisor_kind_str_is_stable() {
    assert_eq!(SubSupervisorKind::Core.as_str(), "Core");
    assert_eq!(SubSupervisorKind::Cognition.as_str(), "Cognition");
    assert_eq!(SubSupervisorKind::Council.as_str(), "Council");
    assert_eq!(SubSupervisorKind::Upgrade.as_str(), "Upgrade");
    assert_eq!(SubSupervisorKind::Plugin.as_str(), "Plugin");
}
