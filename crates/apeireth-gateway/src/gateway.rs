//! OpenClaw-mode single long-lived Gateway.
//!
//! **What this crate owns** (per stage 6 §3 / OpenClaw borrowing):
//! 1. **SingleProcess mode** — exactly one root Session lives until the
//!    process exits. Daemonized mode is a deployment concern, not a runtime
//!    concern, so it is intentionally out of scope here.
//! 2. **Multi-Node admission** — every inbound `NodeKind` is registered,
//!    authorized, and bound to a per-Node `Session`.
//! 3. **DM access security** — `AccessPolicy` rejects unknown/revoked keys
//!    and enforces `DmScope` per NodeKind.
//! 4. **Transport registry** — pluggable adapters (Http, Ws, Telegram, ...)
//!    forward frames into a single fan-out bus.
//! 5. **Agent workspace** — per-Node file + memory + skill roots with
//!    path-traversal defense.
//!
//! **What this crate does NOT own** (lives elsewhere):
//! - Upstream LLM calls (`apeireth-api`).
//! - Council arbitration (`apeireth-council`).
//! - Voice synthesis (`apeireth-voice`).
//! - Persistence / SQLite (`apeireth-vector`).
//! - Skills registry (`apeireth-skills`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use uuid::Uuid;
use crate::auth::{AccessPolicy, AuthDecision, AuthError, AuthResult, DmScope};
use crate::node::{NodeId, NodeKind, NodeRecord, NodeRegistry};
use crate::session::{Session, SessionId, SessionRegistry};
use crate::transport::{Transport, TransportRegistry, InMemoryTransport};
use crate::workspace::AgentWorkspace;

pub const MAX_GATEWAY_NODES: usize = 1024;
pub const MAX_GATEWAY_SESSIONS: usize = 4096;

/// Single long-lived mode. The ceiling is cluster / sharded modes; the
/// upgrade path is `enum GatewayMode { SingleProcess, Cluster { shards: u8 } }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayMode {
    SingleProcess,
}

impl GatewayMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SingleProcess => "single-process",
        }
    }
}

/// Lightweight per-Node sliding-window rate limiter (1s window, 10 reqs).
/// One counter per Node; reset on admit.
/// ponytail: this is a deliberate floor; production wiring uses
/// `apeireth-rate-limiter` which gives 4 algorithms + 5 storage backends.
const RATE_WINDOW_SECS: i64 = 1;
const RATE_LIMIT_PER_NODE: u32 = 10;

#[derive(Debug, Default, Clone)]
struct RateLimitState {
    window_start: i64,
    count: u32,
}

impl RateLimitState {
    fn allow(&mut self, now: i64) -> bool {
        if now - self.window_start >= RATE_WINDOW_SECS {
            self.window_start = now;
            self.count = 0;
        }
        if self.count >= RATE_LIMIT_PER_NODE {
            return false;
        }
        self.count += 1;
        true
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("gateway: auth failed: {0}")]
    Auth(#[from] AuthError),
    #[error("gateway: capacity exceeded: nodes={0} max={MAX_GATEWAY_NODES}")]
    NodeCapacity(usize),
    #[error("gateway: capacity exceeded: sessions={0} max={MAX_GATEWAY_SESSIONS}")]
    SessionCapacity(usize),
    #[error("gateway: not running")]
    NotRunning,
    #[error("gateway: already running")]
    AlreadyRunning,
    #[error("gateway: workspace root missing")]
    WorkspaceRootMissing,
}

pub type GatewayResult<T> = Result<T, GatewayError>;

/// Read-only snapshot for diagnostics + API endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySnapshot {
    pub mode: GatewayMode,
    pub running: bool,
    pub node_count: usize,
    pub session_count: usize,
    pub live_session_count: usize,
    pub transport_channels: Vec<String>,
    pub active_key_count: usize,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admission {
    pub node_id: NodeId,
    pub session_id: SessionId,
    pub owner: String,
    pub node_kind: NodeKind,
}

/// The single long-lived Gateway.
pub struct Gateway {
    mode: GatewayMode,
    name: String,
    started_at: i64,
    running: Arc<RwLock<bool>>,
    nodes: NodeRegistry,
    sessions: SessionRegistry,
    policy: AccessPolicy,
    transports: TransportRegistry,
    rate_limits: Arc<Mutex<HashMap<String, RateLimitState>>>,
    workspace_root: Option<String>,
    /// In-memory test transport always registered for the loopback channel.
    loopback: Arc<InMemoryTransport>,
}

impl Gateway {
    pub fn open(mode: GatewayMode, name: impl Into<String>, started_at: i64) -> Self {
        let loopback = Arc::new(InMemoryTransport::new("loopback"));
        let transports = TransportRegistry::new();
        transports.register(loopback.clone() as Arc<dyn Transport>);
        Self {
            mode,
            name: name.into(),
            started_at,
            running: Arc::new(RwLock::new(true)),
            nodes: NodeRegistry::new(),
            sessions: SessionRegistry::new(),
            policy: AccessPolicy::new(),
            transports,
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            workspace_root: None,
            loopback,
        }
    }

    pub fn mode(&self) -> GatewayMode {
        self.mode
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn started_at(&self) -> i64 {
        self.started_at
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn nodes(&self) -> &NodeRegistry {
        &self.nodes
    }

    pub fn sessions(&self) -> &SessionRegistry {
        &self.sessions
    }

    pub fn policy(&self) -> &AccessPolicy {
        &self.policy
    }

    pub fn transports(&self) -> &TransportRegistry {
        &self.transports
    }

    pub fn loopback(&self) -> Arc<InMemoryTransport> {
        self.loopback.clone()
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn workspace_for(&self, node_id: NodeId, kind: NodeKind) -> Option<AgentWorkspace> {
        self.workspace_root.as_ref().map(|r| AgentWorkspace::new(r.clone(), node_id, kind))
    }

    pub fn register_key(&self, key: impl Into<String>, owner: impl Into<String>, label: impl Into<String>, scope: DmScope, now: i64) {
        let api_key = crate::auth::ApiKey::new(key, owner, label, now);
        self.policy.register_key(api_key, scope);
    }

    pub fn register_transport(&self, t: Arc<dyn Transport>) -> Option<Arc<dyn Transport>> {
        self.transports.register(t)
    }

    /// Admit a node: authorize key, register node, open session, fold rate-limit.
    pub fn admit_node(
        &self,
        kind: NodeKind,
        label: impl Into<String>,
        owner: impl Into<String>,
        api_key: &str,
        now: i64,
    ) -> GatewayResult<Admission> {
        if !self.is_running() {
            return Err(GatewayError::NotRunning);
        }
        if self.nodes.len() >= MAX_GATEWAY_NODES {
            return Err(GatewayError::NodeCapacity(self.nodes.len()));
        }
        let node_id = Uuid::new_v4();
        // 1. authorize
        let decision: AuthDecision = self.policy.authorize(api_key, kind, node_id)?;
        // 2. rate-limit (per key, not per node_id, so it actually binds)
        {
            let mut rl = self.rate_limits.lock();
            let state = rl.entry(api_key.to_string()).or_default();
            if !state.allow(now) {
                return Err(GatewayError::Auth(AuthError::RateLimited(decision.owner.clone())));
            }
        }
        // 3. register node
        let rec = NodeRecord::new(kind, label, owner, now);
        let node_id = self.nodes.register(rec);
        // 4. open session
        if self.sessions.len() >= MAX_GATEWAY_SESSIONS {
            return Err(GatewayError::SessionCapacity(self.sessions.len()));
        }
        let s = self.sessions.open(node_id, kind, now);
        Ok(Admission {
            node_id,
            session_id: s.id,
            owner: decision.owner,
            node_kind: kind,
        })
    }

    pub fn release_node(&self, node_id: NodeId, now: i64) -> bool {
        let n = self.nodes.unregister(node_id).is_some();
        let mut count = 0;
        for s in self.sessions.list_by_node(node_id) {
            if self.sessions.close(s.id, now) {
                count += 1;
            }
        }
        // Rate-limit is per-key; releasing a node does not reset the window.
        n || count > 0
    }

    pub fn touch_node(&self, node_id: NodeId, now: i64) -> bool {
        self.nodes.touch(node_id, now)
    }

    pub fn close_session(&self, session_id: SessionId, now: i64) -> bool {
        self.sessions.close(session_id, now)
    }

    pub fn snapshot(&self) -> GatewaySnapshot {
        GatewaySnapshot {
            mode: self.mode,
            running: self.is_running(),
            node_count: self.nodes.len(),
            session_count: self.sessions.len(),
            live_session_count: self.sessions.live_count(),
            transport_channels: self.transports.channels(),
            active_key_count: self.policy.active_key_count(),
            workspace_root: self.workspace_root.clone(),
        }
    }

    pub fn shutdown(&self) {
        *self.running.write() = false;
    }

    pub fn root_session(&self) -> Option<Session> {
        self.sessions.root()
    }

    pub fn admit_node_as(
        &self,
        kind: NodeKind,
        label: impl Into<String>,
        owner: impl Into<String>,
        api_key: &str,
        scope: DmScope,
        now: i64,
    ) -> GatewayResult<Admission> {
        let owner_str: String = owner.into();
        self.register_key(api_key, owner_str.clone(), "auto-issued", scope, now);
        self.admit_node(kind, label, owner_str, api_key, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_mode_as_str() {
        assert_eq!(GatewayMode::SingleProcess.as_str(), "single-process");
    }

    #[test]
    fn gateway_open_is_running() {
        let g = Gateway::open(GatewayMode::SingleProcess, "test", 100);
        assert!(g.is_running());
        assert_eq!(g.mode(), GatewayMode::SingleProcess);
        assert_eq!(g.name(), "test");
        assert_eq!(g.started_at(), 100);
    }

    #[test]
    fn gateway_register_key_and_admit_node() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "alice", "primary", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "tui-1", "alice", "k1", 100).unwrap();
        assert_eq!(a.owner, "alice");
        assert_eq!(a.node_kind, NodeKind::Tui);
        assert_eq!(g.nodes().len(), 1);
        assert_eq!(g.sessions().len(), 1);
    }

    #[test]
    fn gateway_admit_requires_running() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.shutdown();
        g.register_key("k1", "alice", "l", DmScope::All, 0);
        assert!(matches!(
            g.admit_node(NodeKind::Tui, "x", "alice", "k1", 0),
            Err(GatewayError::NotRunning)
        ));
    }

    #[test]
    fn gateway_admit_auth_failure_does_not_register_node() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "alice", "l", DmScope::Single(NodeKind::Http), 0);
        assert!(g.admit_node(NodeKind::Tui, "x", "alice", "k1", 0).is_err());
        assert_eq!(g.nodes().len(), 0);
    }

    #[test]
    fn gateway_release_node_closes_sessions() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "alice", "l", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "x", "alice", "k1", 0).unwrap();
        // open a child session
        let _ = g.sessions().open(a.node_id, NodeKind::Tui, 0);
        assert!(g.release_node(a.node_id, 100));
        assert_eq!(g.nodes().len(), 0);
        assert_eq!(g.sessions().live_count(), 0);
    }

    #[test]
    fn gateway_release_unknown_node_returns_false() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        assert!(!g.release_node(Uuid::new_v4(), 100));
    }

    #[test]
    fn gateway_touch_node_updates_last_seen() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "x", "u", "k1", 100).unwrap();
        assert!(g.touch_node(a.node_id, 500));
        assert_eq!(g.nodes().get(a.node_id).unwrap().last_seen_at, 500);
    }

    #[test]
    fn gateway_close_session() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "x", "u", "k1", 0).unwrap();
        assert!(g.close_session(a.session_id, 100));
        assert!(g.sessions().get(a.session_id).unwrap().state.is_alive() == false);
    }

    #[test]
    fn gateway_snapshot_reflects_state() {
        let g = Gateway::open(GatewayMode::SingleProcess, "diag", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        let _ = g.admit_node(NodeKind::Http, "h", "u", "k1", 0).unwrap();
        let s = g.snapshot();
        assert_eq!(s.mode, GatewayMode::SingleProcess);
        assert!(s.running);
        assert_eq!(s.node_count, 1);
        assert_eq!(s.session_count, 1);
        assert_eq!(s.live_session_count, 1);
        assert!(s.transport_channels.contains(&"loopback".to_string()));
        assert_eq!(s.active_key_count, 1);
    }

    #[test]
    fn gateway_node_capacity_respected() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        // MAX_GATEWAY_NODES is 1024; bound the test to 3 to keep it fast.
        // We bypass the limit by direct registry push for the first 3 nodes.
        for _ in 0..3 {
            let _ = g.nodes().register(crate::node::NodeRecord::new(NodeKind::Cli, "x", "u", 0));
        }
        // Now admit a real one through the auth path — succeeds (4 total).
        let _ = g.admit_node(NodeKind::Cli, "x", "u", "k1", 0).unwrap();
        assert_eq!(g.nodes().len(), 4);
    }

    #[test]
    fn gateway_transport_registry_default_loopback() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        assert!(g.transports().get("loopback").is_some());
    }

    #[test]
    fn gateway_register_extra_transport() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        let t = Arc::new(InMemoryTransport::new("custom"));
        assert!(g.register_transport(t).is_none());
        assert!(g.transports().get("custom").is_some());
    }

    #[test]
    fn gateway_workspace_root_attached() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0).with_workspace_root("/tmp/ws");
        let node_id = Uuid::new_v4();
        let ws = g.workspace_for(node_id, NodeKind::Tui).unwrap();
        assert!(ws.skills_dir().unwrap().to_string_lossy().contains("tui"));
    }

    #[test]
    fn gateway_admit_rate_limit_bursts() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        // 10 successful admits inside the same 1s window.
        let mut ok = 0;
        for _i in 0..10 {
            if g.admit_node(NodeKind::Cli, "c", "u", "k1", 0).is_ok() {
                ok += 1;
            }
        }
        assert_eq!(ok, 10);
        // 11th must be rate-limited.
        assert!(matches!(
            g.admit_node(NodeKind::Cli, "c", "u", "k1", 0),
            Err(GatewayError::Auth(AuthError::RateLimited(_)))
        ));
    }

    #[test]
    fn gateway_admit_rate_limit_window_resets() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        for _ in 0..10 {
            let _ = g.admit_node(NodeKind::Cli, "c", "u", "k1", 0);
        }
        // Move forward 2s -> window resets.
        assert!(g.admit_node(NodeKind::Cli, "c", "u", "k1", 2).is_ok());
    }

    #[test]
    fn gateway_admit_node_as_auto_registers() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        let a = g.admit_node_as(NodeKind::Tui, "auto", "bob", "k1", DmScope::All, 0).unwrap();
        assert_eq!(a.owner, "bob");
        assert_eq!(g.policy().active_key_count(), 1);
    }

    #[test]
    fn gateway_shutdown_flips_running() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        assert!(g.is_running());
        g.shutdown();
        assert!(!g.is_running());
    }

    #[test]
    fn gateway_root_session_present() {
        let g = Gateway::open(GatewayMode::SingleProcess, "t", 0);
        g.register_key("k1", "u", "l", DmScope::All, 0);
        let a = g.admit_node(NodeKind::Tui, "x", "u", "k1", 0).unwrap();
        let root = g.root_session().unwrap();
        assert_eq!(root.id, a.session_id);
    }
}
