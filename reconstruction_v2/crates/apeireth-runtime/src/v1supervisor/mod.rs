//! v1 era apeireth-supervisor transcription (7 files).
//!
//! Source: crates/_archived/v1.0-legacy/apeireth-supervisor/src
//! Files transcribed verbatim:
//!   - actor.rs          (tokio actor mailbox + Actor trait)
//!   - child.rs          (declarative ChildSpec + restart decision)
//!   - pid_one.rs        (root supervisor — never restartable)
//!   - strategy.rs       (RestartStrategy + ExitReason + RestartDecision)
//!   - supervisor.rs     (SubSupervisorKind + default_plan 5-subtree/21-child)
//!   - journal_entry.rs  (host-call journal, chidori 1:1)
//!   - span.rs           (OTel-style SpanTracker, R259)
//!
//! Module graph (interior):
//!   strategy, actor, journal_entry, span   ← leaves (no internal deps)
//!   child                                  ← uses strategy
//!   supervisor, pid_one                    ← uses child + strategy
//!
//! Note: v1 imports `crate::strategy::*` / `crate::child::*` / `crate::supervisor::*`
//! were rewritten to `super::*` for this nested layout (preserves semantics).

pub mod actor;
pub mod child;
pub mod journal_entry;
pub mod pid_one;
pub mod span;
pub mod strategy;
pub mod supervisor;
