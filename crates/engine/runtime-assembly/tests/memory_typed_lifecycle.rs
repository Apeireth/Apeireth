use std::sync::Arc;

use apeireth_core::kernel::Timestamp;
use apeireth_memory::{
    Commitment, CommitmentKind, CommitmentStatus, MemoryMaterializationOutcome, MemoryProvenance,
    MemoryScope, MemoryTypedMaterializationSink, PersonaProfileDelta, PersonaProfileStore,
    SqliteCommitmentStore, SqlitePersonaProfileStore, SqliteTemporalGraphStore, TemporalGraphFact,
    TemporalGraphQuery, TypedMemoryRecallSource,
};
use apeireth_runtime_assembly::{CanonicalMemoryTypedSink, SqliteTypedMemoryRecallSource};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

fn commitment(text: &str) -> Commitment {
    Commitment {
        id: format!("commitment-{text}"),
        subject_id: "user-a".into(),
        kind: CommitmentKind::Shared,
        status: CommitmentStatus::Active,
        text: text.into(),
        scope: Some("user:user-a".into()),
        provenance: Some("acceptance-test".into()),
        source_episode: None,
        deadline: Some(1_800_000_000_000),
        confidence: 0.9,
        supersedes_id: None,
        retracted_at_ms: None,
        completed_at_ms: None,
        created_at: 1_700_000_000_000,
        updated_at: 1_700_000_000_000,
        revision: 0,
    }
}

#[tokio::test]
async fn typed_stores_share_file_restart_and_preserve_lifecycle_state() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("typed-lifecycle.sqlite");
    let pool = Arc::new(SqliteConnectionPool::open(&path).await.unwrap());
    let commitments = Arc::new(SqliteCommitmentStore::from_arc(pool.clone()));
    commitments.ensure_schema().await.unwrap();
    let persona = Arc::new(SqlitePersonaProfileStore::from_arc(pool.clone()));
    persona.ensure_schema().await.unwrap();
    let relations = Arc::new(SqliteTemporalGraphStore::from_arc(pool.clone()));
    relations.ensure_schema().await.unwrap();

    let sink = CanonicalMemoryTypedSink::new()
        .with_commitments(commitments.clone())
        .with_persona_store(persona.clone())
        .with_relations(relations.clone())
        .with_identity("persona-a", "user-a");

    let active = commitment("submit report Friday");
    commitments.create(active.clone()).await.unwrap();
    let delta = PersonaProfileDelta {
        facts_add: vec!["prefers concise technical explanations".into()],
        provenance: MemoryProvenance {
            source: "acceptance-test".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sink.materialize_persona(&delta).await.unwrap(),
        MemoryMaterializationOutcome::Applied
    );
    let first = TemporalGraphFact {
        id: "wuhan-fact".into(),
        relation_key: "user-location".into(),
        subject_id: "user-a".into(),
        predicate: "lives_in".into(),
        object_id: "Wuhan".into(),
        valid_from_ms: 1_700_000_000_000,
        valid_until_ms: None,
        believed_at_ms: 1_700_000_000_000,
        source_episode_id: None,
        scope: serde_json::json!({"scope":"user","user_id":"user-a"}),
        provenance: serde_json::json!({"source":"acceptance-test"}),
        confidence: 0.9,
        revision: 0,
        supersedes_id: None,
        retracted_at_ms: None,
        created_at_ms: 1_700_000_000_000,
    };
    let first = relations.append(first).await.unwrap();
    let mut second = first.clone();
    second.id = "shanghai-fact".into();
    second.object_id = "Shanghai".into();
    second.valid_from_ms = 1_800_000_000_000;
    let second = relations.supersede(&first.id, second).await.unwrap();

    commitments
        .transition(
            &active.id,
            active.revision,
            CommitmentStatus::Completed,
            1_800_000_000_001,
            Some("submitted"),
        )
        .await
        .unwrap();

    drop(sink);
    drop(commitments);
    drop(persona);
    drop(relations);
    drop(pool);

    let reopened_pool = Arc::new(SqliteConnectionPool::open(&path).await.unwrap());
    let reopened_commitments = SqliteCommitmentStore::from_arc(reopened_pool.clone());
    let reopened_persona = SqlitePersonaProfileStore::from_arc(reopened_pool.clone());
    let reopened_relations = SqliteTemporalGraphStore::from_arc(reopened_pool);
    assert_eq!(
        reopened_commitments
            .get(&active.id)
            .unwrap()
            .unwrap()
            .status,
        CommitmentStatus::Completed
    );
    let profile = reopened_persona
        .get_profile("persona-a", "user-a")
        .await
        .unwrap()
        .unwrap();
    assert!(profile
        .known_facts
        .iter()
        .any(|fact| fact.contains("concise")));
    let current = reopened_relations
        .query(&TemporalGraphQuery::new(1_900_000_000_000).subject("user-a"))
        .unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].object_id, "Shanghai");
    let history = reopened_relations.history("user-location").unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].object_id, "Wuhan");
    assert_eq!(history[1].id, second.id);

    let typed_source = SqliteTypedMemoryRecallSource::new()
        .with_commitments(Arc::new(reopened_commitments.clone()))
        .with_persona(Arc::new(reopened_persona.clone()))
        .with_profile(profile.clone())
        .with_relations(Arc::new(reopened_relations.clone()));
    let identity = apeireth_memory::TypedRecallIdentity {
        persona_id: "persona-a".into(),
        subject_id: "user-a".into(),
    };
    let query = apeireth_memory::MemoryRecallQuery::new("session", "report Wuhan concise")
        .with_visible_scopes(vec![
            MemoryScope::User {
                user_id: "user-a".into(),
            },
            MemoryScope::Persona {
                persona_id: "persona-a".into(),
                user_id: "user-a".into(),
            },
        ]);
    let candidates = typed_source
        .candidates(&query, &identity, 1_900_000_000_000)
        .unwrap();
    assert!(candidates
        .iter()
        .any(|item| item.id.starts_with("typed:persona:")));
    assert!(candidates
        .iter()
        .any(|item| item.id.starts_with("typed:relation:")));
    assert!(!candidates
        .iter()
        .any(|item| item.id.starts_with("typed:commitment:")));
}

#[tokio::test]
async fn typed_sink_requires_identity_and_persona_isolation_is_explicit() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("persona-identity.sqlite");
    let pool = Arc::new(SqliteConnectionPool::open(&path).await.unwrap());
    let store = Arc::new(SqlitePersonaProfileStore::from_arc(pool));
    store.ensure_schema().await.unwrap();
    let delta = PersonaProfileDelta {
        portrait_replace: Some("detailed technical guide".into()),
        ..Default::default()
    };
    let missing = CanonicalMemoryTypedSink::new().with_persona_store(store.clone());
    assert!(matches!(
        missing.materialize_persona(&delta).await.unwrap(),
        MemoryMaterializationOutcome::Skipped { .. }
    ));
    let user_a = CanonicalMemoryTypedSink::new()
        .with_persona_store(store.clone())
        .with_identity("persona-a", "user-a");
    assert_eq!(
        user_a.materialize_persona(&delta).await.unwrap(),
        MemoryMaterializationOutcome::Applied
    );
    assert!(store
        .get_profile("persona-a", "user-b")
        .await
        .unwrap()
        .is_none());
    assert!(store
        .get_profile("persona-a", "user-a")
        .await
        .unwrap()
        .is_some());
}
