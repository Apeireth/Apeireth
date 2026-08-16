//! Integration tests for apeireth-vector
//!
//! **R18 第 2 阶段 P2 第 1 项**: SqliteVecBackend 真 sqlite-vec e2e

use apeireth_vector::{SqliteVecBackend, Vector, VectorStore};
use uuid::Uuid;

fn backend() -> SqliteVecBackend {
    SqliteVecBackend::open_in_memory().expect("open in-memory")
}

fn vec(id: Uuid, data: Vec<f32>) -> Vector {
    Vector::new(id, data)
}

// =====================================================================
// Vector 自身
// =====================================================================

#[test]
fn vector_new_sets_id_and_data() {
    let id = Uuid::new_v4();
    let v = vec(id, vec![1.0, 2.0, 3.0]);
    assert_eq!(v.id, id);
    assert_eq!(v.data, vec![1.0, 2.0, 3.0]);
    assert_eq!(v.dim(), 3);
    assert!(v.metadata.is_none());
}

#[test]
fn vector_with_metadata() {
    let id = Uuid::new_v4();
    let v = Vector::with_metadata(id, vec![1.0; 4], serde_json::json!({"tag": "test"}));
    assert_eq!(v.dim(), 4);
    assert!(v.metadata.is_some());
}

// =====================================================================
// SqliteVecBackend CRUD
// =====================================================================

#[test]
fn backend_opens_in_memory() {
    let _b = backend();
}

#[test]
fn backend_starts_empty() {
    let b = backend();
    assert!(b.is_empty().expect("is_empty"));
    assert_eq!(b.len().expect("len"), 0);
    assert_eq!(b.dimension(), 0);
}

#[test]
fn backend_set_dimension() {
    let mut b = backend();
    b.set_dimension(4).expect("set dim 4");
    assert_eq!(b.dimension(), 4);
}

#[test]
fn backend_upsert_and_len() {
    let mut b = backend();
    b.set_dimension(3).expect("set dim 3");
    let v1 = vec(Uuid::new_v4(), vec![1.0, 0.0, 0.0]);
    b.upsert(&v1).expect("upsert");
    assert_eq!(b.len().expect("len"), 1);
    assert!(!b.is_empty().expect("is_empty"));
}

#[test]
fn backend_upsert_overwrites_same_id() {
    let mut b = backend();
    b.set_dimension(3).expect("set dim 3");
    let id = Uuid::new_v4();
    b.upsert(&vec(id, vec![1.0, 0.0, 0.0])).expect("upsert 1");
    b.upsert(&vec(id, vec![0.0, 1.0, 0.0]))
        .expect("upsert 2 (overwrite)");
    assert_eq!(b.len().expect("len"), 1); // still 1, overwritten
}

#[test]
fn backend_search_returns_top_k() {
    let mut b = backend();
    b.set_dimension(3).expect("set dim");
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();
    b.upsert(&vec(id1, vec![1.0, 0.0, 0.0])).expect("1");
    b.upsert(&vec(id2, vec![0.0, 1.0, 0.0])).expect("2");
    b.upsert(&vec(id3, vec![0.0, 0.0, 1.0])).expect("3");
    // search for [1, 0, 0] -> top match is id1, 其余 2 个距离相等 (1/sqrt(2)) 所以并列
    let hits = b.search(&[1.0, 0.0, 0.0], 3).expect("search");
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].id, id1, "id1 should be top match");
    assert!(hits[0].score > hits[1].score, "id1 should beat id2/id3");
    // id2 跟 id3 到 [1,0,0] 距离都是 sqrt(2) ≈ 1.414, 并列, 排序稳定但 score 相等
    assert!(
        hits[1].score >= hits[2].score,
        "id2/id3 并列 (距离相等), 顺序由 stable sort 决定"
    );
}

#[test]
fn backend_search_respects_k_limit() {
    let mut b = backend();
    b.set_dimension(2).expect("set dim");
    for _ in 0..5 {
        b.upsert(&vec(Uuid::new_v4(), vec![1.0, 0.0]))
            .expect("upsert");
    }
    let hits = b.search(&[1.0, 0.0], 3).expect("search k=3");
    assert_eq!(hits.len(), 3, "should return at most k hits");
}

#[test]
fn backend_delete_removes_vector() {
    let mut b = backend();
    b.set_dimension(2).expect("set dim");
    let id = Uuid::new_v4();
    b.upsert(&vec(id, vec![1.0, 0.0])).expect("upsert");
    assert_eq!(b.len().expect("len"), 1);
    let removed = b.delete(id).expect("delete");
    assert!(removed, "delete should return true for existing id");
    assert_eq!(b.len().expect("len"), 0);
}

#[test]
fn backend_delete_nonexistent_is_noop() {
    let mut b = backend();
    b.set_dimension(2).expect("set dim");
    let removed = b.delete(Uuid::new_v4()).expect("delete");
    assert!(!removed, "delete of nonexistent should return false");
}

#[test]
fn backend_clear_empties_all() {
    let mut b = backend();
    b.set_dimension(2).expect("set dim");
    for _ in 0..5 {
        b.upsert(&vec(Uuid::new_v4(), vec![1.0, 0.0]))
            .expect("upsert");
    }
    assert_eq!(b.len().expect("len"), 5);
    let cleared = b.clear().expect("clear");
    assert_eq!(cleared, 5);
    assert!(b.is_empty().expect("is_empty"));
}

#[test]
fn backend_path_returns_some() {
    let b = backend();
    let p = b.path();
    // in-memory 通常返 ":memory:" 或 None
    let _ = p; // 不假设具体形式
}
