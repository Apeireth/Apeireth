use super::*;
use crate::SqliteMemoryStore;

// Build the historical path without the empty-schema specialization. Keep this
// fixture derived from the append-only migration list, rather than a copy of the
// optimized definitions it is meant to validate.
fn historical_database(through: usize) -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let tx = conn.transaction().unwrap();
    tx.execute_batch(INIT_SQL).unwrap();
    install_history_guards(&tx).unwrap();
    for migration in &MIGRATIONS[..through] {
        tx.execute_batch(migration.sql).unwrap();
        tx.execute(
            "INSERT INTO schema_migrations VALUES (?1, ?2, 0)",
            params![migration.version, migration.name],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    conn
}

fn schema_objects(conn: &Connection) -> Vec<(String, String, String, Option<String>)> {
    conn.prepare("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap()
        .query_map([], |row| {
            let kind: String = row.get(0)?;
            let sql: Option<String> = row.get(3)?;
            // ADD COLUMN preserves comments/spacing from the original CREATE.
            // Table column metadata (including exact defaults) is checked below;
            // index expressions and trigger bodies are compared verbatim.
            let sql = if kind == "table" {
                sql.map(|sql| {
                    sql.lines()
                        .map(|line| line.split("--").next().unwrap())
                        .flat_map(str::split_whitespace)
                        .collect()
                })
            } else {
                sql
            };
            Ok((kind, row.get(1)?, row.get(2)?, sql))
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn assert_same_schema(left: &Connection, right: &Connection) {
    assert_eq!(schema_objects(left), schema_objects(right));
    let tables: Vec<String> = left
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    for table in tables {
        for pragma in ["table_xinfo", "foreign_key_list"] {
            let metadata = |conn: &Connection| {
                let mut stmt = conn.prepare(&format!("PRAGMA {pragma}({table})")).unwrap();
                let columns = stmt.column_count();
                stmt.query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
            };
            assert_eq!(metadata(left), metadata(right), "{pragma}({table})");
        }
    }
    let ledger = |conn: &Connection| {
        conn.prepare("SELECT version, name FROM schema_migrations ORDER BY version")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    assert_eq!(ledger(left), ledger(right));
}

#[test]
fn fresh_schema_matches_every_historical_migration() {
    let expected = historical_database(MIGRATIONS.len());
    let fresh = SqliteMemoryStore::open_in_memory().unwrap();
    assert_same_schema(&fresh.conn().unwrap(), &expected);
}

#[test]
fn every_historical_version_upgrades_without_changing_data() {
    let fresh = SqliteMemoryStore::open_in_memory().unwrap();
    for through in 1..=MIGRATIONS.len() {
        let mut conn = historical_database(through);
        conn.execute_batch(
            "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
             VALUES ('ep', 'subject', 'session', 7, 'user', 'preserved');
             INSERT INTO sessions (id, started_at, last_active_at) VALUES ('session', 7, 8);
             INSERT INTO notes (id, timestamp, content) VALUES ('note', 7, 'preserved');",
        )
        .unwrap();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_same_schema(&conn, &fresh.conn().unwrap());
        let row: (String, Option<i64>, Option<String>) = conn
            .query_row(
                "SELECT content, created_ms, provenance FROM episodes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, ("preserved".into(), None, None), "V{through}");
        let revision: i64 = conn
            .query_row("SELECT revision FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(revision, 0, "V{through}");
        let note: (String, Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT content, valid_from, valid_until FROM notes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(note, ("preserved".into(), None, None), "V{through}");
        assert!(conn
            .execute("UPDATE episodes SET content = 'mutated'", [])
            .is_err());
        assert!(conn.execute("DELETE FROM episodes", []).is_err());
    }
}

#[test]
fn fresh_schema_failure_rolls_back_all_objects_and_versions() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "max_page_count", 2).unwrap();
    assert!(run_migrations(&mut conn).is_err());
    assert!(conn.is_autocommit());
    assert!(schema_objects(&conn).is_empty());
    conn.pragma_update(None, "max_page_count", 1_073_741_823)
        .unwrap();
    run_migrations(&mut conn).unwrap();
    assert_same_schema(&conn, &historical_database(MIGRATIONS.len()));
}

#[test]
fn concurrent_first_openers_recheck_empty_schema_under_writer_lock() {
    use std::sync::{Arc, Barrier};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    drop(conn);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(path).unwrap();
                conn.busy_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                barrier.wait();
                run_migrations(&mut conn).unwrap();
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_same_schema(
        &Connection::open(path).unwrap(),
        &historical_database(MIGRATIONS.len()),
    );
}

#[tokio::test]
async fn pooled_migrations_are_idempotent_across_restart() {
    let path = tempfile::NamedTempFile::new().unwrap();
    let path = path.path().to_owned();

    let pool = apeireth_storage::SqliteConnectionPool::open(&path)
        .await
        .unwrap();
    run_migrations_on_pool(&pool).await.unwrap();
    run_migrations_on_pool(&pool).await.unwrap();
    let first = pool
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(first, MIGRATIONS.len() as i64);
    drop(pool);

    let reopened = apeireth_storage::SqliteConnectionPool::open(&path)
        .await
        .unwrap();
    run_migrations_on_pool(&reopened).await.unwrap();
    let second = reopened
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(second, first);
}

#[test]
fn migrations_apply_idempotently() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let first = store.applied_migrations().unwrap();
    assert!(first.contains(&1), "expected V1 applied");
    // 二次 open: 不应重复插入.
    let conn = store.conn().expect("store conn");
    let mut guard = conn;
    run_migrations(&mut guard).unwrap();
    let second = SqliteMemoryStore::open_in_memory()
        .unwrap()
        .applied_migrations()
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn append_only_triggers_reject_inplace_update_and_delete() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let conn = store.conn().expect("store conn");
    // 直接 raw SQL 验证: 写入一行后, UPDATE 必失败.
    conn.execute(
        "INSERT INTO thought_stream (id, subject_id, subject_rev, created_at, payload, source)
         VALUES ('t1', 'subj', 1, 0, 'p', 'unit')",
        [],
    )
    .unwrap();
    // 改 payload 应被 trigger 拒绝
    let update_err = conn
        .execute(
            "UPDATE thought_stream SET payload = 'x' WHERE id = 't1'",
            [],
        )
        .unwrap_err();
    assert!(
        update_err.to_string().contains("append-only"),
        "expected append-only error, got {update_err}"
    );
    // 改 subject_id 也应被拒绝
    let subj_err = conn
        .execute(
            "UPDATE thought_stream SET subject_id = 'new' WHERE id = 't1'",
            [],
        )
        .unwrap_err();
    assert!(
        subj_err.to_string().contains("append-only"),
        "got {subj_err}"
    );
    // 软删除 (tombstoned_at 从 NULL → 非 NULL) 应被允许
    conn.execute(
        "UPDATE thought_stream SET tombstoned_at = 1000 WHERE id = 't1'",
        [],
    )
    .expect("soft delete should be allowed");
    // 二次软删除 (tombstoned_at 已非 NULL) 应被拒绝
    let re_tomb = conn
        .execute(
            "UPDATE thought_stream SET tombstoned_at = 2000 WHERE id = 't1'",
            [],
        )
        .unwrap_err();
    assert!(re_tomb.to_string().contains("append-only"), "got {re_tomb}");
    // 物理 DELETE 仍被拒绝
    let delete_err = conn
        .execute("DELETE FROM thought_stream WHERE id = 't1'", [])
        .unwrap_err();
    assert!(delete_err.to_string().contains("append-only"));
}

// ===== Core Capability Expansion Phase 7: migration integrity =====

#[test]
fn all_migrations_v1_to_v7_applied_on_fresh_db() {
    // 全新 in-memory DB: V1-V7 全部应用, schema_migrations 记录完整.
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let applied = store.applied_migrations().unwrap();
    for v in 1..=9 {
        assert!(
            applied.contains(&v),
            "migration V{v} should be applied on fresh db"
        );
    }
    // V5/V6/V7 新表/列存在.
    let conn = store.conn().unwrap();
    // sessions 生命周期列 (V5).
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(sessions)")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    for c in [
        "title",
        "scope",
        "project_id",
        "state",
        "metadata_json",
        "revision",
        "archived_at",
        "updated_at",
    ] {
        assert!(
            cols.iter().any(|x| x == c),
            "sessions should have column {c}"
        );
    }
    // episode_governance 表 (V6).
    let gov_exists: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='episode_governance')", [], |r| r.get(0))
        .unwrap();
    assert!(gov_exists, "episode_governance table should exist");
    // agent_traces 表 (V7).
    let trace_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_traces')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(trace_exists, "agent_traces table should exist");
}

#[test]
fn migrations_reopen_preserves_data_and_idempotent() {
    // 写入跨 V5/V6/V7 的数据, 重跑 migration, 数据不丢 + 幂等.
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    use crate::agent_trace::{TraceSpan, TraceSpanKind, TraceSpanStatus};
    use crate::episode::EpisodeStore;
    use crate::{MemoryGovernanceStore, SessionLifecycleStore, TraceStore};
    use apeireth_core::kernel::memory::Episode;
    // session lifecycle (V5).
    store
        .create_session(
            "s1",
            Some("持久"),
            crate::session_lifecycle::SessionScope::Global,
            None,
            None,
        )
        .unwrap();
    // episode + governance (V6).
    store
        .put_episode(&Episode {
            id: "ep-1".into(),
            timestamp: 1,
            role: "user".into(),
            content: "x".into(),
            session_id: "s1".into(),
        })
        .unwrap();
    store.protect_episode("ep-1", 0).unwrap();
    // trace (V7).
    let span = TraceSpan {
        span_id: "sp1".into(),
        trace_id: "t1".into(),
        parent_span_id: None,
        kind: TraceSpanKind::Conversation,
        actor: "user".into(),
        status: TraceSpanStatus::Succeeded,
        summary: Some("done".into()),
        attributes: serde_json::json!({}),
        started_at: 1,
        ended_at: Some(2),
        session_id: Some("s1".into()),
    };
    store.put_trace_span(&span).unwrap();
    // 重跑 migration (幂等).
    {
        let guard = store.conn().unwrap();
        let mut g = guard;
        run_migrations(&mut g).unwrap();
    }
    // 数据仍在.
    let s = store.get_session_lifecycle("s1").unwrap().unwrap();
    assert_eq!(s.title.as_deref(), Some("持久"));
    let g = store.get_governed("ep-1").unwrap().unwrap();
    assert!(g.protected);
    let t = store.get_trace_span("sp1").unwrap().unwrap();
    assert_eq!(t.trace_id, "t1");
    // migration 记录无重复.
    // 断言演进 (2026-10, V4-a 人格记忆批): 11 → 12 —— 迁移列表是 append-only
    // 增长面, 每加一条合法迁移本计数 +1 (钉「无重复」语义, 数字随迁移清单走).
    // 断言演进 (2026-10-17, V4-b 记忆保鲜回路批): 12 → 13 —— 同上语义不变,
    // 新增 V17__memory_freshness_loop (保鲜三件套三表)。
    let applied = store.applied_migrations().unwrap();
    assert_eq!(
        applied.iter().filter(|v| **v >= 5).count(),
        14,
        "V5..V18 各一条 (V15 = dedup_fingerprints; V16 = persona_memory_*; \
         V17 = 记忆保鲜 source_hash/ingest_queue/memory_fold_segments; V18 = lessons)"
    );
}

#[test]
fn legacy_episode_works_after_v6_governance_added() {
    // 旧 episode (无 governance 行) 在 V6 后仍可正常检索 + 默认 active.
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    use crate::episode::EpisodeStore;
    use crate::MemoryGovernanceStore;
    store
        .put_episode(&apeireth_core::kernel::memory::Episode {
            id: "legacy-ep".into(),
            timestamp: 1,
            role: "user".into(),
            content: "old".into(),
            session_id: "me".into(),
        })
        .unwrap();
    let recent = store.governed_recent_episodes("me", 10).unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(
        recent[0].status,
        crate::memory_governance::MemoryGovernanceStatus::Active
    );
    assert!(!recent[0].protected);
}

/// M27 (L 组): 别套 schema 家族 (storage 最小 `(id, data)` layout) 占位的
/// episodes 表必须在迁移前被拒绝 — 否则 `CREATE TABLE IF NOT EXISTS` 静默
/// no-op, 之后 INSERT 才报 "no such column" 难懂的错.
#[test]
fn foreign_episodes_layout_is_rejected_before_migration() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE episodes (id TEXT PRIMARY KEY, data TEXT)")
        .unwrap();
    let err = run_migrations(&mut conn).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("episodes"), "错误必须指名问题表: {msg}");
    assert!(
        msg.contains("foreign schema layout"),
        "错误必须说明是外来 layout: {msg}"
    );
}

/// M27: memory 自己的 layout 不得被误伤 (fresh DB 全跑完 + 重跑都通过).
#[test]
fn memory_owned_layout_passes_validation() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    {
        let conn = store.conn().unwrap();
        let mut guard = conn;
        run_migrations(&mut guard).unwrap();
    }
}

/// M21: 版本重读在 BEGIN IMMEDIATE 事务内 — 已应用的 migration 不再
/// 重复执行/重复插行 (幂等), 两进程并发按写锁串行而非 BUSY_SNAPSHOT.
#[test]
fn migration_version_is_rechecked_inside_immediate_transaction() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let before = store.applied_migrations().unwrap();
    {
        let conn = store.conn().unwrap();
        let mut guard = conn;
        run_migrations(&mut guard).unwrap();
    }
    let after = store.applied_migrations().unwrap();
    assert_eq!(before, after, "Immediate 事务内重读版本, 不得重复插入");
}

/// V15 (M27 / L 组): dedup_fingerprints 必须由迁移列表创建 —
/// 不再只靠 dedup.rs 的懒建表 (schema 快照与迁移列表一致).
#[test]
fn v15_creates_dedup_fingerprints() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    assert!(store.applied_migrations().unwrap().contains(&15));
    let conn = store.conn().unwrap();
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'dedup_fingerprints')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(exists, "V15 必须创建 dedup_fingerprints");
}
