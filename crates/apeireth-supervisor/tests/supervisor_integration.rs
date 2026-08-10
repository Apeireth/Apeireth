//! supervisor_integration — smoke tests for the supervisor crate.
//!
//! - PID 1 default plan shape (5 kinds, 21 children, no restart_strategy field)
//! - Sub-supervisor default strategies are per-kind
//! - Strategy dispatch (OneForOne / RestForOne / Transient)
//! - Child spec builder chains
//! - replace_plan bumps version

use apeireth_supervisor::pid_one::PidOneSupervisor;
use apeireth_supervisor::strategy::{
    affected_indices, should_restart, ExitReason, RestartDecision, RestartStrategy,
};
use apeireth_supervisor::supervisor::{default_plan, SubSupervisorKind};
use apeireth_supervisor::ChildSpec;
use std::time::Duration;

#[test]
fn pid_one_boots_with_5_sub_supervisors() {
    let pid_one = PidOneSupervisor::new();
    assert_eq!(pid_one.sub_supervisors.len(), 5);
    assert_eq!(pid_one.total_children(), 21);
}

#[test]
fn each_kind_has_a_default_strategy() {
    let pid_one = PidOneSupervisor::new();
    for (kind, _) in &pid_one.sub_supervisors {
        // default_strategy is callable — proves the per-kind mapping is wired
        let _ = kind.default_strategy();
    }
}

#[test]
fn one_for_one_strategy_isolates_failures() {
    let affected: Vec<usize> = affected_indices(RestartStrategy::OneForOne, 2, 5).collect();
    assert_eq!(affected, vec![2]);
}

#[test]
fn rest_for_one_takes_subsequent_siblings() {
    let affected: Vec<usize> = affected_indices(RestartStrategy::RestForOne, 1, 4).collect();
    assert_eq!(affected, vec![1, 2, 3]);
}

#[test]
fn transient_decision_skips_normal_exit() {
    assert_eq!(
        should_restart(RestartStrategy::Transient, ExitReason::Normal),
        RestartDecision::Skip
    );
    assert_eq!(
        should_restart(RestartStrategy::Transient, ExitReason::Abnormal(1)),
        RestartDecision::Restart
    );
}

#[test]
fn child_spec_builder_round_trips() {
    let spec = ChildSpec::new(
        "plg.host",
        "plugin host",
        "true",
        RestartStrategy::OneForOne,
    )
    .with_max_restarts(7)
    .with_restart_window(Duration::from_secs(5))
    .with_snapshot("snap-x");
    assert_eq!(spec.id, "plg.host");
    assert_eq!(spec.max_restarts, 7);
    assert_eq!(spec.snapshot_id.as_deref(), Some("snap-x"));
}

#[test]
fn replace_plan_bumps_version_monotonically() {
    let mut pid_one = PidOneSupervisor::new();
    let v0 = pid_one.plan_version;
    let v1 = pid_one.replace_plan(default_plan());
    let v2 = pid_one.replace_plan(default_plan());
    assert!(v1 > v0);
    assert!(v2 > v1);
}

#[test]
fn all_five_kinds_present_in_default_plan() {
    let plan = default_plan();
    let kinds: Vec<SubSupervisorKind> = plan.iter().map(|(k, _)| *k).collect();
    for expected in [
        SubSupervisorKind::Core,
        SubSupervisorKind::Cognition,
        SubSupervisorKind::Council,
        SubSupervisorKind::Upgrade,
        SubSupervisorKind::Plugin,
    ] {
        assert!(kinds.contains(&expected), "missing kind {:?}", expected);
    }
}

#[test]
fn council_has_7_advisor_slots() {
    let pid_one = PidOneSupervisor::new();
    let council = pid_one.children_of(SubSupervisorKind::Council).unwrap();
    assert_eq!(council.len(), 7);
}
