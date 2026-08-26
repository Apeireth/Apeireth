//! M1B1 canonical memory repository tests.
//!
//! Deterministic timestamps only: no sleeps, no wall-clock dependence, no
//! network. Every test uses either an in-memory database or a temporary file.

use std::sync::Arc;

use apeireth_core::kernel::Timestamp;
use apeireth_memory::canonical::{
    MemoryError, MemoryFilter, MemoryId, MemoryItem, MemoryRepository, SqliteMemoryRepository,
};

fn ts(ms: i64) -> Timestamp {
    Timestamp::from_epoch_millis(ms).unwrap()
}

fn item(id: &str, data: &str, created_at_ms: i64) -> MemoryItem {
    MemoryItem::new(MemoryId::new(id).unwrap(), data, ts(created_at_ms))
}

#[tokio::test]
async fn insert_and_get_roundtrip() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let item = item("m1", "hello memory", 100);

    repo.insert(item.clone()).await.unwrap();

    let fetched = repo.get(&item.id).await.unwrap().unwrap();
    assert_eq!(fetched, item);
}

#[tokio::test]
async fn insert_duplicate_id_conflicts() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let item = item("m1", "first", 100);

    repo.insert(item.clone()).await.unwrap();
    let err = repo.insert(item).await.unwrap_err();

    assert!(matches!(err, MemoryError::Conflict(_)));
}

#[tokio::test]
async fn update_replaces_existing_and_reports_missing() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let mut stored = item("m1", "before", 100);

    repo.insert(stored.clone()).await.unwrap();

    stored.data = "after".into();
    stored.importance = 0.75;
    repo.update(stored.clone()).await.unwrap();

    let fetched = repo.get(&stored.id).await.unwrap().unwrap();
    assert_eq!(fetched.data, "after");
    assert_eq!(fetched.importance, 0.75);

    let missing = item("missing", "x", 100);
    let err = repo.update(missing).await.unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(_)));
}

#[tokio::test]
async fn query_filters_temporal_validity_deterministically() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();

    // a: valid from 100 forever; b: valid [100, 200); c: future;
    // d: valid [100, 150).
    let mut a = item("a", "alpha", 100);
    a.valid_from = ts(100);
    let mut b = item("b", "beta", 100);
    b.valid_from = ts(100);
    b.valid_until = Some(ts(200));
    let mut c = item("c", "gamma", 100);
    c.valid_from = ts(300);
    let mut d = item("d", "delta", 100);
    d.valid_from = ts(100);
    d.valid_until = Some(ts(150));

    for i in [a, b, c, d] {
        repo.insert(i).await.unwrap();
    }

    // as_of == 100: b and d are at their inclusive lower bound; c is future.
    let at_start = repo.query(&MemoryFilter::new(ts(100))).await.unwrap();
    assert_eq!(ids(&at_start), vec!["a", "b", "d"]);

    // as_of == 150: d's exclusive upper bound excludes it.
    let at_boundary = repo.query(&MemoryFilter::new(ts(150))).await.unwrap();
    assert_eq!(ids(&at_boundary), vec!["a", "b"]);

    // as_of == 200: b's exclusive upper bound excludes it; c is still future.
    let later = repo.query(&MemoryFilter::new(ts(200))).await.unwrap();
    assert_eq!(ids(&later), vec!["a"]);

    // as_of == 300: c becomes effective; b is expired.
    let future = repo.query(&MemoryFilter::new(ts(300))).await.unwrap();
    assert_eq!(ids(&future), vec!["a", "c"]);

    // Before all windows: nothing is effective.
    let before = repo.query(&MemoryFilter::new(ts(99))).await.unwrap();
    assert!(before.is_empty());
}

#[tokio::test]
async fn tombstone_hides_from_normal_reads() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let item = item("m1", "delete me", 100);

    repo.insert(item.clone()).await.unwrap();
    repo.tombstone(&item.id).await.unwrap();

    // Normal read by id does not surface a tombstone.
    assert!(repo.get(&item.id).await.unwrap().is_none());

    // Normal query does not surface a tombstone.
    let active = repo.query(&MemoryFilter::new(ts(100))).await.unwrap();
    assert!(active.is_empty());

    // Explicit historical query does.
    let historical = repo
        .query(&MemoryFilter::new(ts(100)).with_include_tombstones(true))
        .await
        .unwrap();
    assert_eq!(historical.len(), 1);
    assert!(historical[0].is_tombstone);

    // Tombstoning is idempotent.
    repo.tombstone(&item.id).await.unwrap();

    // Missing ids are an explicit error, not a silent success.
    let missing = MemoryId::new("missing").unwrap();
    let err = repo.tombstone(&missing).await.unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(_)));
}

#[tokio::test]
async fn full_entity_roundtrip_preserves_all_domain_fields() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let item = MemoryItem {
        id: MemoryId::new("m-full").unwrap(),
        data: "full payload".into(),
        importance: 0.875,
        access_count: 42,
        access_times: vec![ts(1000), ts(2000), ts(3000)],
        created_at: ts(500),
        valid_from: ts(600),
        valid_until: Some(ts(900)),
        is_tombstone: false,
        artifact_sig: Some("sha256:abcdef".into()),
    };

    repo.insert(item.clone()).await.unwrap();
    let fetched = repo.get(&item.id).await.unwrap().unwrap();

    assert_eq!(fetched, item);
    assert_eq!(fetched.access_count, 42);
    assert_eq!(fetched.access_times, vec![ts(1000), ts(2000), ts(3000)]);
    assert_eq!(fetched.artifact_sig.as_deref(), Some("sha256:abcdef"));
}

#[tokio::test]
async fn persistent_reopen_preserves_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");

    let item = item("durable", "still here", 100);

    {
        let repo = SqliteMemoryRepository::open(&path).await.unwrap();
        repo.insert(item.clone()).await.unwrap();
    }

    {
        let repo = SqliteMemoryRepository::open(&path).await.unwrap();
        let fetched = repo.get(&item.id).await.unwrap().unwrap();
        assert_eq!(fetched, item);
    }
}

#[tokio::test]
async fn parallel_inserts_and_reads_share_the_canonical_pool() {
    let repo = Arc::new(SqliteMemoryRepository::in_memory().await.unwrap());

    let mut handles = Vec::new();
    for i in 0..20 {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let item = item(&format!("p{i}"), &format!("payload-{i}"), 100);
            repo.insert(item).await.unwrap();
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }

    let all = repo.query(&MemoryFilter::new(ts(100))).await.unwrap();
    assert_eq!(all.len(), 20);

    // The deterministic order is by created_at and then id. All created_at
    // values are equal, so ids are the stable tie-breaker.
    let first = repo
        .get(&MemoryId::new("p0").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.data, "payload-0");
}

#[tokio::test]
async fn invalid_domain_data_is_rejected_before_write() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();

    let empty_data = item("bad-empty", "", 100);
    assert!(matches!(
        repo.insert(empty_data).await,
        Err(MemoryError::InvalidData(_))
    ));

    let nan_importance = MemoryItem {
        importance: f64::NAN,
        ..item("bad-nan", "x", 100)
    };
    assert!(matches!(
        repo.insert(nan_importance).await,
        Err(MemoryError::InvalidData(_))
    ));

    let backwards_window = MemoryItem {
        valid_from: ts(200),
        valid_until: Some(ts(100)),
        ..item("bad-window", "x", 100)
    };
    assert!(matches!(
        repo.insert(backwards_window).await,
        Err(MemoryError::InvalidData(_))
    ));

    assert!(MemoryId::new("").is_err());
}

#[tokio::test]
async fn storage_failure_propagates_as_memory_persistence_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("not_a_db.db");
    std::fs::write(&path, b"not a sqlite database").unwrap();

    let err = SqliteMemoryRepository::open(&path).await.unwrap_err();
    assert!(matches!(err, MemoryError::Persistence(_)));
}

fn ids(items: &[MemoryItem]) -> Vec<&str> {
    items.iter().map(|i| i.id.as_str()).collect()
}
