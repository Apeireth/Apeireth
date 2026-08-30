//! Deterministic integration tests for the M1A canonical storage foundation.

use apeireth_storage::{
    current_version, run_migrations, SqliteConfig, SqliteConnectionPool, StorageError,
    LATEST_SCHEMA_VERSION,
};

fn temp_db_path(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "apeireth_storage_test_{}_{}_{}.db",
        std::process::id(),
        name,
        nanos
    ))
}

#[tokio::test]
async fn open_file_backed_pool_and_roundtrip_write_read() {
    let path = temp_db_path("roundtrip");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    pool.write(|conn| {
        conn.execute_batch("CREATE TABLE test_items (id INTEGER PRIMARY KEY, value TEXT)")?;
        conn.execute("INSERT INTO test_items (value) VALUES (?1)", ["hello"])?;
        Ok(())
    })
    .await
    .unwrap();

    let value: String = pool
        .read(|conn| {
            Ok(
                conn.query_row("SELECT value FROM test_items WHERE id = 1", [], |row| {
                    row.get(0)
                })?,
            )
        })
        .unwrap();

    assert_eq!(value, "hello");
    drop(pool);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn multiple_reads_see_committed_writes() {
    let path = temp_db_path("multi_read");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    pool.write(|conn| {
        conn.execute_batch("CREATE TABLE test_reads (id INTEGER PRIMARY KEY, value TEXT)")?;
        conn.execute("INSERT INTO test_reads (id, value) VALUES (1, 'a')", [])?;
        conn.execute("INSERT INTO test_reads (id, value) VALUES (2, 'b')", [])?;
        Ok(())
    })
    .await
    .unwrap();

    let count: i64 = pool
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM test_reads", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(count, 2);

    let first: String = pool
        .read(|conn| {
            Ok(
                conn.query_row("SELECT value FROM test_reads WHERE id = 1", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(first, "a");

    drop(pool);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn concurrent_writes_are_serialized_without_corruption() {
    let path = temp_db_path("concurrent_writes");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    pool.write(|conn| {
        conn.execute_batch("CREATE TABLE test_concurrent (id INTEGER PRIMARY KEY, value TEXT)")?;
        Ok(())
    })
    .await
    .unwrap();

    let mut handles = Vec::new();
    for i in 0..40 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            p.write(move |conn| {
                conn.execute(
                    "INSERT INTO test_concurrent (id, value) VALUES (?1, ?2)",
                    (&i, &format!("value-{i}")),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let count: i64 = pool
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM test_concurrent", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(count, 40);

    let unique_values: i64 = pool
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT count(DISTINCT value) FROM test_concurrent",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(unique_values, 40);

    drop(pool);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn wal_configuration_is_applied_to_connections() {
    let path = temp_db_path("wal_config");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    let journal_mode: String = pool
        .read(|conn| Ok(conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?))
        .unwrap();
    let foreign_keys: i64 = pool
        .read(|conn| Ok(conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
        .unwrap();
    let busy_timeout: i64 = pool
        .read(|conn| Ok(conn.query_row("PRAGMA busy_timeout", [], |r| r.get(0))?))
        .unwrap();

    assert_eq!(journal_mode.to_lowercase(), "wal");
    assert_eq!(foreign_keys, 1);
    assert_eq!(busy_timeout, 5000);

    drop(pool);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn shared_in_memory_pool_is_visible_across_connections() {
    let pool = SqliteConnectionPool::in_memory().await.unwrap();

    pool.write(|conn| {
        conn.execute_batch("CREATE TABLE test_memory (id INTEGER PRIMARY KEY, value TEXT)")?;
        conn.execute("INSERT INTO test_memory (value) VALUES ('shared')", [])?;
        Ok(())
    })
    .await
    .unwrap();

    let value: String = pool
        .read(|conn| Ok(conn.query_row("SELECT value FROM test_memory", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(value, "shared");
}

#[tokio::test]
async fn write_sql_failure_surfaces_as_storage_error() {
    let pool = SqliteConnectionPool::in_memory().await.unwrap();

    let result = pool
        .write(|conn| {
            conn.execute("INSERT INTO missing_table (id) VALUES (1)", [])?;
            Ok(())
        })
        .await;

    match result {
        Err(StorageError::Db(_)) => {}
        other => panic!("expected StorageError::Db, got {other:?}"),
    }
}

#[tokio::test]
async fn migrations_are_versioned_and_idempotent_on_file_db() {
    let path = temp_db_path("migrations");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    pool.write(|conn| run_migrations(conn)).await.unwrap();
    let version = pool.read(current_version).unwrap();
    assert_eq!(version, LATEST_SCHEMA_VERSION);

    pool.write(|conn| run_migrations(conn)).await.unwrap();
    let version_again = pool.read(current_version).unwrap();
    assert_eq!(version_again, LATEST_SCHEMA_VERSION);

    let facts_table_exists: bool = pool
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'facts')",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert!(facts_table_exists);

    drop(pool);

    // Re-opening an already-migrated database must succeed and stay at v1.
    let reopened = SqliteConnectionPool::open(&path).await.unwrap();
    reopened.write(|conn| run_migrations(conn)).await.unwrap();
    let reopened_version = reopened.read(current_version).unwrap();
    assert_eq!(reopened_version, LATEST_SCHEMA_VERSION);

    drop(reopened);
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn config_with_zero_connections_is_rejected() {
    let path = temp_db_path("zero_connections");
    let config = SqliteConfig::new().with_max_connections(0);
    let result = SqliteConnectionPool::open_with_config(&path, config).await;
    match result {
        Err(StorageError::InvalidConfiguration(_)) => {}
        other => panic!("expected InvalidConfiguration, got {other:?}"),
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn rate_limiter_token_bucket_burst_and_refill() {
    use apeireth_storage::rate_limit::{token_bucket_in_memory, RateLimiterStats};
    use std::time::{Duration, Instant};

    let l = token_bucket_in_memory(100.0, 5, None).unwrap();
    let now = Instant::now();
    for _ in 0..5 {
        assert!(l.try_acquire_at("k", 1, now).unwrap());
    }
    assert!(!l.try_acquire_at("k", 1, now).unwrap());
    assert!(l.try_acquire_at("k", 1, now + Duration::from_millis(20)).unwrap());
    let s: RateLimiterStats = l.stats();
    assert_eq!(s.hits, 6);
    assert_eq!(s.misses, 1);
}

#[test]
fn rate_limiter_retry_after_overrides_backoff() {
    use apeireth_storage::rate_limit::{decide, ConstantBackoff, RetryAfter, RetryOutcome};
    use std::time::Duration;

    let b = ConstantBackoff::new(Duration::from_millis(100), 0);
    let outcome = decide(&b, 0, Some(RetryAfter::Seconds(5)), Duration::ZERO, 0);
    assert_eq!(outcome, RetryOutcome::Retry(Duration::from_secs(5)));
}
