//! CliPanelData adapter: session mapping, trace/audit archives, tool catalog,
//! and the memory surface (list/append/protect/forget/graph).

use std::sync::{Arc, Mutex};

use apeireth_cli::gateway_panels::CliPanelData;
use apeireth_core::kernel::{Clock, HistoryEntry, SessionId, StreamKind, Timestamp, VirtualClock};
use apeireth_core::Episode;
use apeireth_gateway::{PanelData, TraceSpanDto};
use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_plugin::memory_backend::{BackendKind, CapabilityResult, MemoryBackend};
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::canonical::{InMemorySessionStore, Session, SessionStore};

/// In-memory test backend mirroring the production trait surface.
struct FakeMemory {
    episodes: Mutex<Vec<Episode>>,
}

impl FakeMemory {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            episodes: Mutex::new(Vec::new()),
        })
    }
}

impl MemoryBackend for FakeMemory {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }

    fn put_episode(&self, ep: &Episode) -> CapabilityResult<()> {
        self.episodes.lock().unwrap().push(ep.clone());
        Ok(())
    }

    fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>> {
        Ok(self
            .episodes
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned())
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> CapabilityResult<Vec<Episode>> {
        let mut episodes: Vec<Episode> = self
            .episodes
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect();
        episodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        episodes.truncate(n);
        Ok(episodes)
    }

    fn append_stream(&self, _kind: StreamKind, _entry: HistoryEntry) -> CapabilityResult<()> {
        Ok(())
    }

    fn list_stream(
        &self,
        _kind: StreamKind,
        _session_id: &str,
        _n: usize,
    ) -> CapabilityResult<Vec<HistoryEntry>> {
        Ok(Vec::new())
    }
}

fn clock_pair() -> (VirtualClock, Arc<dyn Clock>) {
    let virtual_clock = VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    );
    let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
    (virtual_clock, clock)
}

/// Shared test policy mirroring the production grants.
fn test_policy() -> Arc<Mutex<PermissionPolicy>> {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    policy.grant(Permission::ExecuteTool("tool.filesystem".to_string()));
    Arc::new(Mutex::new(policy))
}

#[tokio::test]
async fn sessions_map_to_contract_summaries() {
    let store = Arc::new(InMemorySessionStore::new());
    let (virtual_clock, clock) = clock_pair();
    let mut older = Session::new(SessionId::new(), clock.as_ref());
    older.append(
        NormalizedMessage::user("第一条消息, 用来当标题"),
        clock.as_ref(),
    );
    virtual_clock.advance(chrono::Duration::seconds(30));
    let mut newer = Session::new(SessionId::new(), clock.as_ref());
    newer.append(NormalizedMessage::user("second"), clock.as_ref());
    store.save(&older).await.unwrap();
    store.save(&newer).await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let panels = CliPanelData::new(
        store,
        FakeMemory::new(),
        test_policy(),
        true,
        dir.path().to_path_buf(),
    );
    let sessions = panels.list_sessions().await.unwrap();

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id, newer.id.to_string(), "newest first");
    assert_eq!(sessions[0].title.as_deref(), Some("second"));
    assert_eq!(sessions[0].message_count, 1);
    assert_eq!(sessions[1].title.as_deref(), Some("第一条消息, 用来当标题"));
    assert_eq!(sessions[1].revision, 1);
}

#[tokio::test]
async fn trace_and_audit_archives_round_trip_across_reopen() {
    let store = Arc::new(InMemorySessionStore::new());
    let dir = tempfile::tempdir().unwrap();
    let panels = CliPanelData::new(
        store.clone(),
        FakeMemory::new(),
        test_policy(),
        false,
        dir.path().to_path_buf(),
    );

    panels
        .append_trace(
            "t1",
            vec![TraceSpanDto {
                span_id: "t1-0".into(),
                parent_span_id: None,
                kind: "turn".into(),
                actor: "runtime".into(),
                status: "ok".into(),
                summary: None,
                started_at: 5,
                ended_at: None,
                session_id: None,
            }],
        )
        .await;
    panels
        .append_audit("chat.turn.completed", Some("session=x"))
        .await;

    let traces = panels.list_traces(10).await.unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].trace_id, "t1");
    assert_eq!(traces[0].span_count, 1);
    assert_eq!(traces[0].started_at, 5);

    let detail = panels.trace_detail("t1").await.unwrap().unwrap();
    assert_eq!(detail.spans[0].kind, "turn");
    assert!(panels.trace_detail("nope").await.unwrap().is_none());

    let audit = panels.list_audit(10).await.unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].event, "chat.turn.completed");
    assert_eq!(audit[0].service, "gateway");
    assert_eq!(audit[0].detail.as_deref(), Some("session=x"));

    // A fresh instance over the same dir must reload both archives.
    let reopened = CliPanelData::new(
        store,
        FakeMemory::new(),
        test_policy(),
        false,
        dir.path().to_path_buf(),
    );
    assert_eq!(reopened.list_traces(10).await.unwrap().len(), 1);
    assert_eq!(reopened.list_audit(10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn tool_catalog_follows_local_read_flag() {
    let store = Arc::new(InMemorySessionStore::new());
    let dir = tempfile::tempdir().unwrap();

    let off = CliPanelData::new(
        store.clone(),
        FakeMemory::new(),
        test_policy(),
        false,
        dir.path().to_path_buf(),
    );
    let tools = off.list_tools().await.unwrap();
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[0].name, "tool.repo");
    assert_eq!(tools[0].permission, "granted");
    assert!(tools[0].available);
    assert!(!tools[1].available, "local read tools off by default");
    assert_eq!(tools[1].permission, "none");

    let on = CliPanelData::new(
        store,
        FakeMemory::new(),
        test_policy(),
        true,
        dir.path().to_path_buf(),
    );
    let tools = on.list_tools().await.unwrap();
    assert!(tools[1].available);
    assert_eq!(tools[1].permission, "granted");
    assert!(tools[2].available);
}

#[tokio::test]
async fn real_sqlite_backend_persists_episodes_over_file_pool() {
    // Mirrors the production composition: file pool + run_migrations +
    // SqliteBackend over the same pool. Regression guard for the schema
    // mismatch that once made every put_episode vanish silently
    // (INSERT OR IGNORE vs NOT NULL continuity_id).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cognitive.sqlite3");
    let pool = Arc::new(
        apeireth_storage::SqliteConnectionPool::open(&path)
            .await
            .unwrap(),
    );
    pool.write(|conn| {
        apeireth_memory::run_migrations(conn).map_err(|e| {
            apeireth_storage::StorageError::Migration {
                version: 0,
                name: "cognitive_memory",
                message: e.to_string(),
            }
        })
    })
    .await
    .unwrap();

    let backend = apeireth_memory::backend::sqlite::SqliteBackend::from_arc(pool.clone());
    let episode = Episode {
        id: "ep-persist".into(),
        timestamp: 123,
        role: "user".into(),
        content: "persisted?".into(),
        session_id: "sess-persist".into(),
    };
    backend.put_episode(&episode).unwrap();

    let recent = backend.recent_episodes("sess-persist", 10).unwrap();
    assert_eq!(
        recent.len(),
        1,
        "file-pool put must survive into recent reads"
    );
    assert_eq!(recent[0].content, "persisted?");

    // Independent raw connection must see the row on disk.
    let raw = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = raw
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "the row must be durable on disk");
}

#[tokio::test]
async fn memory_surface_append_list_protect_forget_graph() {
    let (_, clock) = clock_pair();
    let store = Arc::new(InMemorySessionStore::new());
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("会话标题"), clock.as_ref());
    store.save(&session).await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let panels = CliPanelData::new(
        store.clone(),
        FakeMemory::new(),
        test_policy(),
        false,
        dir.path().to_path_buf(),
    );

    // append via the panel surface
    let appended = panels
        .append_episode(&session.id.to_string(), "user", "主人喜欢古风")
        .await
        .unwrap();
    assert_eq!(appended.session_id, session.id.to_string());
    assert_eq!(appended.protected, Some(false));
    assert_eq!(appended.status.as_deref(), Some("active"));
    assert_eq!(appended.timestamp % 1_000, 0, "contract is epoch ms");

    // list: session-scoped + query filter + newest first
    panels
        .append_episode(&session.id.to_string(), "assistant", "第二条记忆")
        .await
        .unwrap();
    let listed = panels
        .list_episodes(Some(&session.id.to_string()), None, 10)
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed[0].timestamp >= listed[1].timestamp);
    let searched = panels
        .list_episodes(Some(&session.id.to_string()), Some("古风"), 10)
        .await
        .unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].id, appended.id);

    // protect with rev check, then conflict
    let protected = panels.protect_episode(&appended.id, 0).await.unwrap();
    assert!(protected.ok);
    assert_eq!(protected.rev, 1);
    assert!(
        panels.protect_episode(&appended.id, 0).await.is_err(),
        "stale rev must conflict"
    );
    let listed = panels
        .list_episodes(Some(&session.id.to_string()), None, 10)
        .await
        .unwrap();
    let protected_episode = listed.iter().find(|e| e.id == appended.id).unwrap();
    assert_eq!(protected_episode.protected, Some(true));

    // unprotect then forget
    let unprotected = panels.unprotect_episode(&appended.id, 1).await.unwrap();
    assert_eq!(unprotected.rev, 2);
    let forgotten = panels
        .forget_episode(&appended.id, 2, Some("测试遗忘"))
        .await
        .unwrap();
    assert_eq!(forgotten.rev, 3);
    let listed = panels
        .list_episodes(Some(&session.id.to_string()), None, 10)
        .await
        .unwrap();
    assert!(
        listed.iter().all(|e| e.id != appended.id),
        "forgotten episode must disappear from lists"
    );

    // graph: session node + remaining episode node + containment edge
    let graph = panels.memory_graph().await.unwrap();
    assert!(graph.nodes.iter().any(|n| n.kind == "session"));
    assert!(graph.nodes.iter().any(|n| n.kind == "episode"));
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from, format!("session:{}", session.id));
    assert!(
        !graph.edges.iter().any(|e| e.to == appended.id),
        "forgotten episode must not be in the graph"
    );

    // append to a brand-new session creates it in the ledger, so the global
    // (session-less) list can reach the episode too
    let ghost_id = SessionId::new();
    panels
        .append_episode(&ghost_id.to_string(), "user", "幽灵会话的记忆")
        .await
        .unwrap();
    let global = panels.list_episodes(None, None, 10).await.unwrap();
    assert!(global.iter().any(|e| e.session_id == ghost_id.to_string()));
    assert!(panels
        .list_sessions()
        .await
        .unwrap()
        .iter()
        .any(|s| s.id == ghost_id.to_string()));

    // flags survive reopen
    let reopened = CliPanelData::new(
        store,
        FakeMemory::new(),
        test_policy(),
        false,
        dir.path().to_path_buf(),
    );
    assert!(reopened.forget_episode(&appended.id, 3, None).await.is_ok());
}

#[tokio::test]
async fn grants_list_revoke_and_organ_catalog() {
    let store = Arc::new(InMemorySessionStore::new());
    let dir = tempfile::tempdir().unwrap();
    let policy = test_policy();
    let panels = CliPanelData::new(
        store,
        FakeMemory::new(),
        policy.clone(),
        false,
        dir.path().to_path_buf(),
    );

    // grants mirror the shared policy in deterministic order
    let grants = panels.list_grants().await.unwrap();
    assert_eq!(grants.len(), 2);
    assert!(grants.iter().any(|g| g.capability == "tool.repo"));
    assert!(grants.iter().any(|g| g.capability == "tool.filesystem"));
    assert_eq!(grants[0].permission, "execute_tool:tool.filesystem"); // BTreeSet order

    // hot revoke mutates the SAME policy the hook would evaluate
    let revoked = panels.revoke_grant("tool.repo").await.unwrap();
    assert!(revoked.ok);
    let grants = panels.list_grants().await.unwrap();
    assert!(!grants.iter().any(|g| g.capability == "tool.repo"));
    assert!(!policy
        .lock()
        .unwrap()
        .has(&Permission::ExecuteTool("tool.repo".to_string())));

    // revoking again reports not-present honestly
    let again = panels.revoke_grant("tool.repo").await.unwrap();
    assert!(!again.ok);

    // organ catalog: 9 canonical organs, all disabled by production default
    let organs = panels.list_organs().await.unwrap();
    assert_eq!(organs.len(), 9);
    assert_eq!(organs[0].id, "W1");
    assert!(organs.iter().all(|o| !o.enabled));
    assert!(organs.iter().any(|o| o.id == "Memory"));
}
