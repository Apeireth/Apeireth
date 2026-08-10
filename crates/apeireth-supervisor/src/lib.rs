//! apeireth-supervisor: process-level supervision tree
//!
//! Provides:
//! - [`PidOneSupervisor`]: the root supervisor (never restarted by anyone)
//! - [`SubSupervisor`]: 5 sub-supervisors (Core, Cognition, Council, Upgrade, Plugin)
//! - [`RestartStrategy`]: OneForOne / RestForOne / Transient
//! - [`ChildSpec`]: declarative child process spec
//! - [`actor`]: tokio-based actor mailbox + handle trait
//!
//! Hard constraints:
//! - Pure tokio::process::Command — no external scripts
//! - Windows uses `cmd /c exit <code>`; Unix uses `true`/`false`
//! - PID 1 is structurally never restartable (no `restart_strategy` field)

pub mod actor;
pub mod child;
pub mod pid_one;
pub mod strategy;
pub mod supervisor;

pub use actor::{spawn_actor, Actor, ActorRef, ActorState};
pub use child::ChildSpec;
pub use pid_one::PidOneSupervisor;
pub use strategy::{ExitReason, RestartDecision, RestartStrategy};
pub use supervisor::SubSupervisorKind;

// Re-export test helpers used by integration tests
pub use crate::strategy::{affected_indices, should_restart};

// === P28 阶段 6: apeireth-verify 互锁注入 === — disabled V26 to avoid circular
// apeireth_verify::trace_init!(VERIFY_TRACE);
// apeireth_verify::regression_assert!(
//     __APEIRETH_REG_APEIRETH_SUPERVISOR_A,
//     "apeireth-supervisor",
//     "apeireth-supervisor structural invariant — P28 互锁 (assert_in_range)",
//     InRange { name: "apeireth-supervisor::invariant-a", value: 1.0, min: 0.0, max: 1.0 }
// );
// apeireth_verify::regression_assert!(
//     __APEIRETH_REG_APEIRETH_SUPERVISOR_B,
//     "apeireth-supervisor",
//     "apeireth-supervisor regression gate — P28 互锁 (assert_idempotent)",
//     Idempotent { name: "apeireth-supervisor::invariant-b", first: "stable", second: "stable" }
// );

// ============================================================================
// round9-04 (V26.4) — __register_all_asserts no-op stub
//
// V26.2 backend_engineer2 disabled the original `apeireth_verify::register_all_in_crate!` macro
// call to break a circular dependency (core/verify mutually referenced).
// V26.3 DEF-V26.3-002 walk_all_crates example couldn't compile because no __register_all_asserts
// existed. V26.4 fix: provide a no-op stub that walk_all_crates can call. The stub does
// nothing (no regression assertions registered) which is the V26.2 intent (no circular
// dependency, but the symbol exists for example discovery).
//
// Future upgrade path (P28 stage 6 real impl): replace this stub with the real macro
// call once the circular dependency is resolved (e.g., via inventory/ctor or refactor
// apeireth-verify to be a thin facade).
#[allow(missing_docs, dead_code)] // V26.4 stub: walk_all_crates calls this no-op
pub fn __register_all_asserts() {
    // no-op by design
}
