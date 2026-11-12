use std::collections::BTreeSet;
use std::sync::Arc;

use apeireth_core::kernel::{HistoryEntry, StreamKind};
use apeireth_core::Episode;
use apeireth_memory::backend::in_memory::InMemoryBackend;
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::backend::{BackendKind, MemoryBackend};
use apeireth_memory::memory_chain::verify_subject_chain;
use apeireth_memory::{MemoryCoordinator, MemoryLayerKind, MemoryRecallQuery, SqliteMemoryStore};
use apeireth_plugin::memory_backend::{CapabilityResult, EpisodeWriteOutcome};
use apeireth_storage::SqliteConnectionPool;
use rusqlite::{Connection, OptionalExtension};

fn episode(id: impl Into<String>) -> Episode {
    let id = id.into();
    Episode {
        content: format!("项目记忆：共同事实与工作记录 {id}"),
        id,
        timestamp: 1_700_000_000,
        role: "user".into(),
        session_id: "session-consistency".into(),
    }
}

fn working_ids(coordinator: &MemoryCoordinator) -> BTreeSet<String> {
    coordinator
        .recall(
            &MemoryRecallQuery::new("session-consistency", "共同事实")
                .with_layers(vec![MemoryLayerKind::Working])
                .with_limit(64)
                .with_max_chars(100_000),
        )
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.id)
        .collect()
}

fn head(conn: &Connection, subject: &str) -> Option<(String, i64)> {
    conn.query_row(
        "SELECT last_hash, entry_count FROM memory_chain_heads WHERE subject_id=?1",
        [subject],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .unwrap()
}

#[tokio::test]
async fn sqlite_sidecar_failure_rolls_back_evidence_and_head_before_working_publication() {
    for pooled in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.sqlite");
        let store = Arc::new(SqliteMemoryStore::open(&path).unwrap());
        let backend: Arc<dyn MemoryBackend> = if pooled {
            Arc::new(SqliteBackend::new(
                SqliteConnectionPool::open(&path).await.unwrap(),
            ))
        } else {
            store.clone()
        };
        let coordinator = MemoryCoordinator::new(backend.clone(), store.clone());
        let subject = if pooled {
            "session-consistency"
        } else {
            "default"
        };
        for index in 0..30 {
            coordinator
                .writeback_episode(&episode(format!("kept-{index}")))
                .unwrap();
        }
        let before = working_ids(&coordinator);
        assert_eq!(before.len(), 30);
        let before_head = head(&store.conn().unwrap(), subject);
        store.conn().unwrap().execute_batch(
            "CREATE TRIGGER reject_sidecar BEFORE INSERT ON episode_memory_metadata
             WHEN NEW.episode_id='failed' BEGIN SELECT RAISE(ABORT, 'injected sidecar failure'); END;",
        ).unwrap();
        assert!(coordinator.writeback_episode(&episode("failed")).is_err());
        assert!(backend.get_episode("failed").unwrap().is_none());
        assert!(backend.get_episode_metadata("failed").unwrap().is_none());
        assert_eq!(head(&store.conn().unwrap(), subject), before_head);
        assert_eq!(
            working_ids(&coordinator),
            before,
            "failed writes must not evict a committed turn"
        );
        store
            .conn()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_sidecar")
            .unwrap();
        coordinator.writeback_episode(&episode("failed")).unwrap();
        assert!(working_ids(&coordinator).contains("failed"));
        drop(coordinator);
        drop(backend);
        drop(store);

        let reopened = SqliteMemoryStore::open(&path).unwrap();
        let metadata = MemoryBackend::get_episode_metadata(&reopened, "failed")
            .unwrap()
            .unwrap();
        assert_eq!(metadata["scope"]["session_id"], "session-consistency");
        assert_eq!(
            metadata["provenance"]["source_session"],
            "session-consistency"
        );
        let verification = verify_subject_chain(&reopened.conn().unwrap(), subject).unwrap();
        assert!(verification.ok);
        assert_eq!(verification.entry_count, 31);
    }
}

#[tokio::test]
async fn sqlite_stable_retry_preserves_ring_and_derived_sidecar_and_rejects_identity_conflicts() {
    for pooled in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.sqlite");
        let store = Arc::new(SqliteMemoryStore::open(&path).unwrap());
        let backend: Arc<dyn MemoryBackend> = if pooled {
            Arc::new(SqliteBackend::new(
                SqliteConnectionPool::open(&path).await.unwrap(),
            ))
        } else {
            store.clone()
        };
        let coordinator = MemoryCoordinator::new(backend.clone(), store.clone());
        for index in 0..30 {
            coordinator
                .writeback_episode(&episode(format!("stable-{index}")))
                .unwrap();
        }
        let before = working_ids(&coordinator);
        let retry = episode("stable-0");
        let mut metadata = backend.get_episode_metadata(&retry.id).unwrap().unwrap();
        metadata["vector"] = serde_json::json!([0.25, 0.75]);
        metadata["injection_count"] = serde_json::json!(4);
        backend
            .put_episode_metadata(&retry.id, metadata.clone())
            .unwrap();
        let subject = if pooled {
            "session-consistency"
        } else {
            "default"
        };
        let before_head = head(&store.conn().unwrap(), subject);
        for _ in 0..3 {
            coordinator.writeback_episode(&retry).unwrap();
        }
        assert_eq!(working_ids(&coordinator), before);
        assert_eq!(head(&store.conn().unwrap(), subject), before_head);
        assert_eq!(
            backend.get_episode_metadata(&retry.id).unwrap().unwrap(),
            metadata
        );

        for field in ["content", "role", "session", "timestamp"] {
            let mut conflict = retry.clone();
            match field {
                "content" => conflict.content = "不同原文".into(),
                "role" => conflict.role = "assistant".into(),
                "session" => conflict.session_id = "another-session".into(),
                _ => conflict.timestamp += 1,
            }
            assert!(coordinator.writeback_episode(&conflict).is_err(), "{field}");
        }
        for key in ["scope", "provenance", "layer"] {
            let mut conflict = metadata.clone();
            conflict[key] = serde_json::json!("conflicting identity");
            assert!(
                backend.put_episode_with_metadata(&retry, conflict).is_err(),
                "{key}"
            );
        }
        assert_eq!(head(&store.conn().unwrap(), subject), before_head);
        assert_eq!(
            backend.get_episode_metadata(&retry.id).unwrap().unwrap(),
            metadata
        );
        assert_eq!(working_ids(&coordinator), before);

        coordinator
            .update_episode_content(&retry.id, "纠正后的共同事实", Some("user"), 0)
            .unwrap();
        let corrected = backend.get_episode_metadata(&retry.id).unwrap().unwrap();
        assert_ne!(corrected["content_hash"], metadata["content_hash"]);
        assert!(corrected.get("vector").is_none());
        coordinator.writeback_episode(&retry).unwrap();
        assert_eq!(
            backend.get_episode_metadata(&retry.id).unwrap().unwrap(),
            corrected
        );
        assert!(
            verify_subject_chain(&store.conn().unwrap(), subject)
                .unwrap()
                .ok
        );
    }
}

struct LegacyMetadataFailure(InMemoryBackend);

#[tokio::test]
async fn replay_of_an_evicted_stable_id_does_not_reenter_or_displace_working_memory() {
    for pooled in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.sqlite");
        let store = Arc::new(SqliteMemoryStore::open(&path).unwrap());
        let backend: Arc<dyn MemoryBackend> = if pooled {
            Arc::new(SqliteBackend::new(
                SqliteConnectionPool::open(&path).await.unwrap(),
            ))
        } else {
            store.clone()
        };
        let coordinator = MemoryCoordinator::new(backend, store.clone());
        for index in 0..31 {
            coordinator
                .writeback_episode(&episode(format!("aged-{index}")))
                .unwrap();
        }
        let before = working_ids(&coordinator);
        assert_eq!(before.len(), 30);
        assert!(
            !before.contains("aged-0"),
            "the first turn must really be evicted"
        );
        let subject = if pooled {
            "session-consistency"
        } else {
            "default"
        };
        let before_head = head(&store.conn().unwrap(), subject);
        for _ in 0..3 {
            coordinator.writeback_episode(&episode("aged-0")).unwrap();
        }
        assert_eq!(
            working_ids(&coordinator),
            before,
            "an existing durable episode must not be promoted to a fresh working turn"
        );
        assert_eq!(head(&store.conn().unwrap(), subject), before_head);
    }
}

impl MemoryBackend for LegacyMetadataFailure {
    fn name(&self) -> &'static str {
        "legacy-metadata-failure"
    }
    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }
    fn put_episode(&self, ep: &Episode) -> CapabilityResult<()> {
        self.0.put_episode(ep)
    }
    fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>> {
        self.0.get_episode(id)
    }
    fn recent_episodes(&self, session: &str, n: usize) -> CapabilityResult<Vec<Episode>> {
        self.0.recent_episodes(session, n)
    }
    fn put_episode_metadata(&self, _: &str, _: serde_json::Value) -> CapabilityResult<()> {
        Err(std::io::Error::other("injected legacy metadata failure").into())
    }
    fn append_stream(&self, kind: StreamKind, entry: HistoryEntry) -> CapabilityResult<()> {
        self.0.append_stream(kind, entry)
    }
    fn list_stream(
        &self,
        kind: StreamKind,
        session: &str,
        n: usize,
    ) -> CapabilityResult<Vec<HistoryEntry>> {
        self.0.list_stream(kind, session, n)
    }
}

#[test]
fn legacy_sequential_failure_remains_explicit_and_does_not_publish_working_memory() {
    let backend = Arc::new(LegacyMetadataFailure(InMemoryBackend::new()));
    let governance = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator = MemoryCoordinator::new(backend.clone(), governance);
    let ep = episode("legacy-partial");
    assert!(coordinator.writeback_episode(&ep).is_err());
    assert!(
        backend.get_episode(&ep.id).unwrap().is_some(),
        "legacy fallback is explicitly non-atomic"
    );
    assert!(working_ids(&coordinator).is_empty());
}

#[test]
fn in_memory_combined_write_is_idempotent_and_preserves_sidecar_on_retry() {
    let backend = InMemoryBackend::new();
    let ep = episode("volatile");
    let metadata =
        serde_json::json!({"scope": "session", "provenance": "user", "layer": "episodic"});
    assert_eq!(
        backend
            .put_episode_with_metadata(&ep, metadata.clone())
            .unwrap(),
        EpisodeWriteOutcome::Inserted
    );
    let mut derived = metadata.clone();
    derived["vector"] = serde_json::json!([1.0]);
    backend
        .put_episode_metadata(&ep.id, derived.clone())
        .unwrap();
    assert_eq!(
        backend.put_episode_with_metadata(&ep, metadata).unwrap(),
        EpisodeWriteOutcome::Existing
    );
    assert_eq!(
        backend.get_episode_metadata(&ep.id).unwrap().unwrap(),
        derived
    );
    let mut conflict = ep.clone();
    conflict.content = "conflicting evidence".into();
    assert!(backend
        .put_episode_with_metadata(&conflict, derived.clone())
        .is_err());
    assert_eq!(
        backend.get_episode(&ep.id).unwrap().unwrap().content,
        ep.content
    );
    assert_eq!(
        backend.get_episode_metadata(&ep.id).unwrap().unwrap(),
        derived
    );
}
