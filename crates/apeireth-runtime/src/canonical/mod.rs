//! The canonical Apeireth runtime.
//!
//! # Ownership
//!
//! This module owns **orchestration**: sessions, provider routing, the agent
//! loop, and the composition root that holds them. It owns no translation (that
//! is `apeireth-protocol`), no capability implementations (those are plugins),
//! and no policy (that is `apeireth-governance`).
//!
//! # Layering
//!
//! ```text
//!   apeireth-core        primitives
//!         ^
//!   apeireth-protocol    translation
//!         ^
//!   apeireth-plugin      capabilities        apeireth-governance -> core
//!         ^                                          ^
//!   apeireth-runtime::canonical  ------------------- '
//! ```
//!
//! Nothing below may depend on anything above it.
//!
//! # A transitional impurity, stated plainly
//!
//! These modules depend only on core, protocol, plugin, and governance. The
//! *crate* they live in does not: `apeireth-runtime` still carries the historical
//! seven-module orchestration driver and its ten internal dependencies
//! (`apeireth-council`, `apeireth-consciousness`, `apeireth-arbitration`, and
//! others). The crate-level graph stays acyclic and the canonical code touches
//! none of that, but the crate boundary is not yet clean.
//!
//! This is deliberate and tracked, not overlooked. Evicting the legacy driver is
//! the first item in the migration map. It is recorded here because a comment
//! claiming the boundary is already clean would be the exact failure mode this
//! convergence exists to correct.

pub mod approval;
pub mod error;
pub mod execute;
pub mod provider;
pub mod runtime;
pub mod session;
pub mod trace;

pub use approval::{
    operation_fingerprint, operation_fingerprint_with_invocation, ApprovalDecision, ApprovalStatus,
    PendingApproval, PendingApprovalView,
};
pub use error::{RuntimeError, RuntimeResult};
pub use execute::{ApprovalResolution, TurnOutcome, TurnRequest, TurnResponse};
pub use provider::{ProviderHealth, ProviderRouter, RoutedCompletion};
pub use runtime::{plugin_ids, Runtime, RuntimeBuilder, RuntimeConfig, DEFAULT_MAX_ROUNDS};
pub use session::{
    InMemorySessionStore, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SqliteSessionStore,
};
pub use trace::{ExecutionTrace, TraceEntry, TraceEvent};
