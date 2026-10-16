//! Integration tests for apeireth-memory (service-based, real SQLite in-memory)
//!
//! **R18 第 2 阶段第 10 项**: 测 SqliteMemoryStore 真实 SQLite (in-memory 模式)

use apeireth_memory::SqliteMemoryStore;
use rusqlite::Connection;

fn store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("open in-memory")
}

#[test]
fn store_opens_in_memory() {
    let _store = store();
}

#[test]
fn store_runs_at_least_one_migration() {
    let s = store();
    let applied = s.applied_migrations().expect("applied_migrations");
    assert!(!applied.is_empty(), "expected at least 1 migration");
}

#[test]
fn store_export_streams_jsonl_empty() {
    let s = store();
    let entries = s.export_streams_jsonl().expect("export_streams_jsonl");
    assert!(
        entries.is_empty(),
        "expected 0 history entries in fresh DB, got {}",
        entries.len()
    );
}

#[test]
fn store_open_in_memory_creates_fresh_db() {
    let s1 = store();
    let applied1 = s1.applied_migrations().expect("applied 1");
    drop(s1);
    let s2 = store();
    let applied2 = s2.applied_migrations().expect("applied 2");
    assert_eq!(applied1.len(), applied2.len());
}

#[test]
fn store_migration_ids_are_positive() {
    let s = store();
    let applied = s.applied_migrations().expect("applied");
    for id in &applied {
        assert!(*id > 0, "migration id should be positive, got {}", id);
    }
}

#[test]
fn store_migration_ids_sorted_ascending() {
    let s = store();
    let mut applied = s.applied_migrations().expect("applied");
    let mut sorted = applied.clone();
    sorted.sort();
    assert_eq!(applied, sorted, "migrations should be in ascending order");
}

#[test]
fn old_v1_memory_fixture_migrates_without_duplicate_versions() {
    let mut conn = Connection::open_in_memory().expect("open sqlite fixture");
    conn.execute_batch(apeireth_memory::MIGRATIONS[0].sql)
        .expect("create legacy V1 schema");
    conn.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, 0)",
        (
            apeireth_memory::MIGRATIONS[0].version,
            apeireth_memory::MIGRATIONS[0].name,
        ),
    )
    .expect("mark V1 applied");
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES ('legacy-episode', 'legacy-subject', 'legacy-session', 7, 'user', 'old fixture')",
        [],
    )
    .expect("insert legacy row");

    apeireth_memory::run_migrations(&mut conn).expect("upgrade legacy fixture");
    apeireth_memory::run_migrations(&mut conn).expect("replay upgrade");

    let versions: Vec<i64> = conn
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    assert_eq!(versions.len(), apeireth_memory::MIGRATIONS.len());
    assert_eq!(
        versions,
        (1..=apeireth_memory::MIGRATIONS.len() as i64).collect::<Vec<_>>()
    );
    let content: String = conn
        .query_row(
            "SELECT content FROM episodes WHERE id = 'legacy-episode'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(content, "old fixture");
}

#[tokio::test]
async fn canonical_validity_query_uses_memory_items_index() {
    let repo = apeireth_memory::canonical::SqliteMemoryRepository::in_memory()
        .await
        .expect("open canonical repository");
    let details: Vec<String> = repo
        .pool()
        .read(|conn| {
            let mut stmt = conn.prepare(
                "EXPLAIN QUERY PLAN SELECT id FROM memory_items
                 WHERE valid_from <= ?1 AND (valid_until IS NULL OR valid_until > ?1)
                   AND is_tombstone = 0
                 ORDER BY created_at ASC, id ASC",
            )?;
            let rows = stmt.query_map([200_i64], |row| row.get::<_, String>(3))?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .expect("explain canonical validity query");
    assert!(
        details.iter().any(|detail| detail.contains("USING INDEX")),
        "expected validity query to use an index, got {details:?}"
    );
    assert!(
        details
            .iter()
            .all(|detail| !detail.contains("SCAN memory_items")),
        "validity query must not fall back to a full table scan, got {details:?}"
    );
}
