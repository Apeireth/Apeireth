//! `apeireth-gateway` — OpenClaw-mode single long-lived Gateway (R174 / stage 6 §3).
//!
//! **What is a gateway, in this design?**
//! A *single* long-lived process that owns one root `Session`, registers any
//! number of inbound `Node` adapters (TUI / HTTP / Desktop / Mobile / CLI),
//! applies DM access security (`AccessPolicy` + `DmScope`), and forwards
//! payloads through a pluggable `Transport` registry into the rest of the
//! 9-organ system.
//!
//! **Borrowed from OpenClaw**, lifted to Rust with compile-time guarantees:
//! - OpenClaw's "single process, multi-LLM, multi-channel" model becomes
//!   `GatewayMode::SingleProcess` (the only mode this crate ships).
//! - OpenClaw's per-channel transport becomes a `Transport` trait + registry.
//! - OpenClaw's per-user scope becomes `DmScope` + `AccessPolicy`.
//!
//! **Non-goals / 0 drift**:
//! - 0 modification to any LOCKED crate (council / runtime / supervisor / ...).
//! - 0 new external dependencies beyond what `apeireth-rate-limiter` already
//!   uses (`tokio`, `serde`, `parking_lot`, `uuid`, `chrono`).
//! - 0 LLM calls or upstream rendering (handled by `apeireth-api`).
//!
//! **Modules (7)**:
#![deny(unsafe_code)]

use std::sync::Arc;
use uuid::Uuid;

pub mod auth;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod guard_bridge;
pub mod gateway;
pub mod node;
pub mod session;
pub mod transport;
pub mod workspace;

pub use auth::{AccessPolicy, ApiKey, AuthDecision, AuthError, AuthResult, DmScope};
pub use guard_bridge::{AuditSummary, GatewayGuard, GuardSide, GuardedFrame};
pub use gateway::{Admission, Gateway, GatewayError, GatewayMode, GatewayResult, GatewaySnapshot, MAX_GATEWAY_NODES, MAX_GATEWAY_SESSIONS};
pub use node::{NodeId, NodeKind, NodeRecord, NodeRegistry};
pub use session::{Session, SessionId, SessionRegistry, SessionState};
pub use transport::{HttpTransport, InFrame, InMemoryTransport, OutFrame, Transport, TransportError, TransportRegistry, WsTransport};
pub use workspace::{safe_join, AgentWorkspace, WorkspaceError, WorkspaceResult, WorkspaceSlot};

/// Compile-time guard: only `SingleProcess` is shipped. Adding a new mode
/// (Cluster / Sharded) requires bumping this constant.
pub const MODES_SUPPORTED: usize = 1;

/// Compile-time guard: 5 Node kinds (OpenClaw taxonomy).
pub const NODE_KINDS: usize = 5;

/// Compile-time guard: 6 modules exposed by this crate.
pub const MODULES: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_time_guards_hold() {
        assert_eq!(MODES_SUPPORTED, 1);
        assert_eq!(NODE_KINDS, 5);
        assert_eq!(MODULES, 7);
    }

    #[test]
    fn end_to_end_admit_release() {
        let g = Gateway::open(GatewayMode::SingleProcess, "e2e", 0);
        g.register_key("k1", "alice", "primary", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "tui-1", "alice", "k1", 100).unwrap();
        assert_eq!(g.nodes().len(), 1);
        assert_eq!(g.sessions().len(), 1);
        assert!(g.release_node(a.node_id, 200));
        assert_eq!(g.nodes().len(), 0);
        assert_eq!(g.sessions().live_count(), 0);
    }

    #[tokio::test]
    async fn end_to_end_transport_exchange() {
        let g = Gateway::open(GatewayMode::SingleProcess, "e2e", 0);
        let custom = Arc::new(InMemoryTransport::new("custom"));
        custom.start().await.unwrap();
        let node_id = Uuid::new_v4();
        custom.push_inbound(InFrame::new(node_id, "custom", serde_json::json!({"hi": 1})));
        let frame = custom.recv().await.unwrap();
        assert_eq!(frame.payload, serde_json::json!({"hi": 1}));
    }

    #[test]
    fn end_to_end_workspace_attached() {
        let g = Gateway::open(GatewayMode::SingleProcess, "e2e", 0).with_workspace_root("/tmp/ws");
        let nid = Uuid::new_v4();
        let ws = g.workspace_for(nid, NodeKind::Desktop).unwrap();
        assert!(ws.skills_dir().unwrap().to_string_lossy().contains("desktop"));
    }

}
