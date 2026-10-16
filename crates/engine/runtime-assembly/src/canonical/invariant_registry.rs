//! Runtime invariant registry — the online guard surface over the audit event
//! stream, exposed at the assembly boundary.
//!
//! The registry, its per-event checks, and the stream consumption point are one
//! shared observation primitive in `apeireth_orchestration::runtime_invariants`;
//! this module surfaces that primitive alongside the turn chain it observes so
//! an assembly host can register, toggle, and run the guards without reaching
//! into another layer's internals.
//!
//! Observation layer only: a violation is a structured record naming the
//! invariant, its module, and a legible detail. Nothing here changes a
//! governance verdict or blocks a turn; fail-fast versus log-only is a policy
//! the call site chooses.

pub use apeireth_orchestration::runtime_invariants::{
    first_batch_auditor, first_batch_invariants, AuditEvent, AuditEventKind, InvariantAuditor,
    InvariantCheck, InvariantMode, InvariantRegistry, InvariantViolation, RuntimeInvariant,
    StreamFacts, INV_A_NO_DOUBLE_SIDE_EFFECT, INV_FORGET_CONFINED_TO_TARGET, INV_FORGET_IDEMPOTENT,
    INV_SWEEP_KEEPS_PROTECTED,
};
