use apeireth_memory::{run_migrations_on_pool, UniversalForgetFacade};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

#[tokio::test]
async fn principal_forget_is_atomic_and_preserves_append_only_provenance() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget.sqlite");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();
    run_migrations_on_pool(&pool).await.unwrap();

    pool.write(|conn| {
        conn.execute("INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-a','session-a',1)", [])?;
        conn.execute("INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-a','user-a','session-a',1,'user','private')", [])?;
        conn.execute("INSERT INTO commitments (id,subject_id,statement,source_episode_id,created_at_ms,updated_at_ms) VALUES ('c-a','user-a','todo','ep-a',1,1)", [])?;
        conn.execute("INSERT INTO temporal_graph_facts (id,relation_key,subject_id,predicate,object_id,valid_from_ms,believed_at_ms,source_episode_id,created_at_ms) VALUES ('f-a','rel-a','user-a','lives_in','Wuhan',1,1,'ep-a',1)", [])?;
        conn.execute("INSERT INTO persona_profiles (persona_id,subject_id,revision,created_at_ms,updated_at_ms) VALUES ('persona-a','user-a',1,1,1)", [])?;
        conn.execute("INSERT INTO persona_profile_governance (persona_id,subject_id) VALUES ('persona-a','user-a')", [])?;
        Ok(())
    }).await.unwrap();

    let result =
        UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 100, Some("user request"));
    assert!(result.is_ok(), "{result:?}");
    let observed = pool.read(|conn| {
        let episode_status: String = conn.query_row("SELECT status FROM episode_governance WHERE episode_id='ep-a'", [], |row| row.get(0))?;
        let commitment_status: String = conn.query_row("SELECT status FROM commitments WHERE id='c-a'", [], |row| row.get(0))?;
        let commitment_events: i64 = conn.query_row("SELECT COUNT(*) FROM commitment_events WHERE commitment_id='c-a'", [], |row| row.get(0))?;
        let retracted: i64 = conn.query_row("SELECT COUNT(*) FROM temporal_graph_facts WHERE relation_key='rel-a' AND retracted_at_ms IS NOT NULL", [], |row| row.get(0))?;
        let tombstoned: i64 = conn.query_row("SELECT COUNT(*) FROM persona_profile_governance WHERE persona_id='persona-a' AND tombstoned_at_ms IS NOT NULL", [], |row| row.get(0))?;
        Ok((episode_status, commitment_status, commitment_events, retracted, tombstoned))
    }).unwrap();
    assert_eq!(observed, ("forgotten".into(), "cancelled".into(), 1, 1, 1));
    assert_eq!(
        pool.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM persona_profiles WHERE persona_id='persona-a'",
            [],
            |row| row.get::<_, i64>(0),
        )?))
        .unwrap(),
        1
    );
}

#[tokio::test]
async fn principal_forget_does_not_touch_unowned_principal() {
    let pool = SqliteConnectionPool::in_memory().await.unwrap();
    run_migrations_on_pool(&pool).await.unwrap();
    pool.write(|conn| {
        conn.execute("INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-b','session-b',1)", [])?;
        conn.execute("INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-b','user-b','session-b',1,'user','keep')", [])?;
        Ok(())
    }).await.unwrap();
    UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 100, None).unwrap();
    assert!(
        pool.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM episode_governance WHERE episode_id='ep-b'",
            [],
            |row| row.get::<_, i64>(0),
        )?))
        .unwrap()
            == 0
    );
}
