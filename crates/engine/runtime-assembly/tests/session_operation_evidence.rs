//! Frozen operations share the canonical session store, migration and sealer.
use apeireth_core::kernel::{system_clock, RequestId, SessionId, Timestamp, TraceId};
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{
    canonical::committed_evidence::operation_payload_hash, CommittedInputOrigin,
    CommittedOperationIntent, CommittedUserInput, Session, SessionStore,
};
use apeireth_runtime_assembly::{canonical::session_seal::SessionSealer, SqliteSessionStore};
use apeireth_storage::crypto::{PartitionId, PartitionKey};

fn sealer() -> SessionSealer {
    SessionSealer::new(PartitionKey::from_bytes(
        PartitionId::new("operation-test").unwrap(),
        [19; 32],
    ))
}
fn intent(session: SessionId) -> CommittedOperationIntent {
    let payload =
        serde_json::json!({"target":"private-fact","replacement":"operation-private-canary"});
    CommittedOperationIntent {
        input: CommittedUserInput {
            session,
            request: RequestId::new(),
            trace: TraceId::new(),
            principal: None,
            authorization_epoch: None,
            at: Timestamp::from_epoch_millis(42).unwrap(),
            messages: vec![NormalizedMessage::user("accepted direct input")],
            origin: CommittedInputOrigin::DirectUser,
        },
        operation_id: "test-operation".into(),
        module_id: "module.test".into(),
        payload_hash: operation_payload_hash(&payload).unwrap(),
        payload,
        prepared_at: Timestamp::from_epoch_millis(43).unwrap(),
        sequence: 0,
    }
}

#[tokio::test]
async fn sealed_operation_survives_restart_and_transcript_upsert_without_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.sqlite3");
    let store = SqliteSessionStore::open(&path)
        .await
        .unwrap()
        .with_sealer(sealer());
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    store.save(&session).await.unwrap();
    let original = intent(session.id);
    let saved = store.put_operation_evidence(&original).await.unwrap();
    assert!(saved.sequence > 0);
    assert_eq!(
        saved,
        store.put_operation_evidence(&original).await.unwrap()
    );
    let mut changed = original.clone();
    changed.payload = serde_json::json!({"replacement":"changed"});
    changed.payload_hash = operation_payload_hash(&changed.payload).unwrap();
    assert!(store.put_operation_evidence(&changed).await.is_err());
    let mut corrupt = original.clone();
    corrupt.payload_hash = "wrong".into();
    assert!(store.put_operation_evidence(&corrupt).await.is_err());
    store.save(&session).await.unwrap();
    drop(store);
    let store = SqliteSessionStore::open(&path)
        .await
        .unwrap()
        .with_sealer(sealer());
    assert_eq!(
        store.operation_evidence(&session.id).await.unwrap(),
        vec![saved]
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    let data: String = connection
        .query_row("SELECT data FROM module_operation_evidence", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(!data.contains("operation-private-canary"));
    assert!(connection
        .execute("UPDATE module_operation_evidence SET data='tampered'", [])
        .is_err());
    assert!(store.delete(&session.id).await.unwrap());
    assert!(store
        .operation_evidence(&session.id)
        .await
        .unwrap()
        .is_empty());
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM module_operation_evidence",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn version_three_database_upgrades_without_losing_existing_session() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.sqlite3");
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE sessions (id TEXT PRIMARY KEY, data TEXT);")
        .unwrap();
    connection.pragma_update(None, "user_version", 3).unwrap();
    connection
        .execute(
            "INSERT INTO sessions(id,data) VALUES (?1,?2)",
            rusqlite::params![
                session.id.to_string(),
                serde_json::to_string(&session).unwrap()
            ],
        )
        .unwrap();
    drop(connection);
    let store = SqliteSessionStore::open(&path).await.unwrap();
    assert!(store.load(&session.id).await.unwrap().is_some());
    store
        .put_operation_evidence(&intent(session.id))
        .await
        .unwrap();
    assert_eq!(
        store.operation_evidence(&session.id).await.unwrap().len(),
        1
    );
}
