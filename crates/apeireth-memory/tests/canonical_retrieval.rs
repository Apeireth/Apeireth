//! M1B2 canonical memory retrieval semantics tests.
//!
//! Deterministic timestamps only; no sleeps, no wall-clock dependence.

use apeireth_core::kernel::Timestamp;
use apeireth_memory::canonical::{
    act_r_activation, retrieve, MemoryId, MemoryItem, MemoryRepository, RetrievalOptions,
    SqliteMemoryRepository,
};

fn ts(ms: i64) -> Timestamp {
    Timestamp::from_epoch_millis(ms).unwrap()
}

fn make_item(
    id: &str,
    data: &str,
    created_at: Timestamp,
    valid_from: Timestamp,
    valid_until: Option<Timestamp>,
    importance: f64,
    access_times: Vec<Timestamp>,
) -> MemoryItem {
    MemoryItem {
        id: MemoryId::new(id).unwrap(),
        data: data.into(),
        importance,
        access_count: access_times.len() as u32,
        access_times,
        created_at,
        valid_from,
        valid_until,
        is_tombstone: false,
        artifact_sig: None,
    }
}

#[tokio::test]
async fn retrieval_returns_eligible_current_memory() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    // Effective forever from 100_000_000.
    repo.insert(make_item(
        "current",
        "current",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![ts(199_000_000)],
    ))
    .await
    .unwrap();

    // Not effective yet.
    repo.insert(make_item(
        "future",
        "future",
        ts(100_000_000),
        ts(300_000_000),
        None,
        0.9,
        vec![ts(199_000_000)],
    ))
    .await
    .unwrap();

    // Expired before as_of.
    repo.insert(make_item(
        "expired",
        "expired",
        ts(100_000_000),
        ts(100_000_000),
        Some(ts(150_000_000)),
        0.9,
        vec![ts(199_000_000)],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].item.id.as_str(), "current");
}

#[tokio::test]
async fn tombstone_excluded_by_default_and_included_when_requested() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    let mut tomb = make_item(
        "tomb",
        "tomb",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.7,
        vec![],
    );
    tomb.is_tombstone = true;
    repo.insert(tomb).await.unwrap();

    let active = make_item(
        "active",
        "active",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.7,
        vec![],
    );
    repo.insert(active).await.unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].item.id.as_str(), "active");

    let hits_with_tombstones = retrieve(
        &repo,
        &RetrievalOptions::new(as_of).with_include_tombstones(true),
    )
    .await
    .unwrap();
    assert_eq!(hits_with_tombstones.len(), 2);
}

#[tokio::test]
async fn importance_filter_excludes_items_below_threshold() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    repo.insert(make_item(
        "low",
        "low",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.1,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "high",
        "high",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.9,
        vec![],
    ))
    .await
    .unwrap();

    let hits = retrieve(
        &repo,
        &RetrievalOptions::new(as_of).with_minimum_importance(0.5),
    )
    .await
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].item.id.as_str(), "high");
}

#[tokio::test]
async fn more_recent_access_ranks_higher() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    repo.insert(make_item(
        "older",
        "older",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![ts(150_000_000)],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "recent",
        "recent",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![ts(199_000_000)],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].item.id.as_str(), "recent");
    assert_eq!(hits[1].item.id.as_str(), "older");
    assert!(hits[0].score > hits[1].score);
}

#[tokio::test]
async fn more_accesses_rank_higher_for_the_same_recency() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    repo.insert(make_item(
        "single",
        "single",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![ts(199_000_000)],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "double",
        "double",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![ts(199_000_000), ts(198_000_000)],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits[0].item.id.as_str(), "double");
    assert_eq!(hits[1].item.id.as_str(), "single");
}

#[tokio::test]
async fn zero_access_items_rank_by_importance_bonus() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    repo.insert(make_item(
        "low-importance",
        "low",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.2,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "high-importance",
        "high",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.8,
        vec![],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits[0].item.id.as_str(), "high-importance");
    assert_eq!(hits[1].item.id.as_str(), "low-importance");

    // Zero access history -> activation == beta == 0.0 by default.
    let activation = act_r_activation(&[], as_of, 0.5, 0.0).unwrap();
    assert_eq!(activation, 0.0);
}

#[tokio::test]
async fn stable_tie_ordering_uses_created_at_then_id() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    // Same score (zero access, same importance, same created_at) -> id order.
    repo.insert(make_item(
        "b",
        "b",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "a",
        "a",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    assert_eq!(hits[0].item.id.as_str(), "a");
    assert_eq!(hits[1].item.id.as_str(), "b");

    // Same score and same id-like recency, but an earlier created_at wins.
    repo.insert(make_item(
        "older-created",
        "older",
        ts(90_000_000),
        ts(90_000_000),
        None,
        0.4,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "newer-created",
        "newer",
        ts(110_000_000),
        ts(110_000_000),
        None,
        0.4,
        vec![],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of))
        .await
        .unwrap();
    let older_pos = hits
        .iter()
        .position(|h| h.item.id.as_str() == "older-created")
        .unwrap();
    let newer_pos = hits
        .iter()
        .position(|h| h.item.id.as_str() == "newer-created")
        .unwrap();
    assert!(older_pos < newer_pos);
}

#[tokio::test]
async fn invalid_retrieval_parameters_are_rejected() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    let err = retrieve(&repo, &RetrievalOptions::new(as_of).with_act_r(0.0, 0.0))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apeireth_memory::canonical::MemoryError::InvalidData(_)
    ));

    let err = retrieve(
        &repo,
        &RetrievalOptions::new(as_of).with_minimum_importance(f64::NAN),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        apeireth_memory::canonical::MemoryError::InvalidData(_)
    ));
}

#[tokio::test]
async fn limit_is_deterministic_after_ranking() {
    let repo = SqliteMemoryRepository::in_memory().await.unwrap();
    let as_of = ts(200_000_000);

    repo.insert(make_item(
        "m1",
        "m1",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.3,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "m2",
        "m2",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.5,
        vec![],
    ))
    .await
    .unwrap();
    repo.insert(make_item(
        "m3",
        "m3",
        ts(100_000_000),
        ts(100_000_000),
        None,
        0.9,
        vec![],
    ))
    .await
    .unwrap();

    let hits = retrieve(&repo, &RetrievalOptions::new(as_of).with_limit(2))
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].item.id.as_str(), "m3");
    assert_eq!(hits[1].item.id.as_str(), "m2");
}
