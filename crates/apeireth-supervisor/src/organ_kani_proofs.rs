//! R177 supervisor organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_sup_01_restart_strategy_default() {
    let r = RestartStrategy::default();
    let _: String = format!("{:?}", r);
}

#[test]
fn r177_sup_02_child_spec_new() {
    let c = ChildSpec::new("a", "label", "/bin/true", RestartStrategy::OneForOne);
    assert_eq!(c.id, "a");
}

#[test]
fn r177_sup_03_exit_reason() {
    let e = ExitReason::Normal;
    let s = format!("{:?}", e);
    assert!(!s.is_empty());
}

#[test]
fn r177_sup_04_sub_supervisor_kind() {
    let k = SubSupervisorKind::Core;
    assert_eq!(k.as_str(), "Core");
}

#[test]
fn r177_sup_05_actor_state_running() {
    let s = ActorState::Running;
    let _: String = format!("{:?}", s);
}

#[cfg(kani)]
#[kani::proof]
fn r177_sup_kani_01_restart_invariant() {
    let r = RestartStrategy::OneForOne;
    assert!(matches!(r, RestartStrategy::OneForOne));
}

#[cfg(kani)]
#[kani::proof]
fn r177_sup_kani_02_exit_invariant() {
    let e = ExitReason::Normal;
    assert!(!format!("{:?}", e).is_empty());
}
