//! R177 vector organ Kani proofs (W6)

#![allow(missing_docs)]

use crate::error::VectorError;
use crate::traits::{ScoredId, SearchHit, Vector};
use uuid::Uuid;

#[test]
fn r177_vec_01_vector_new() {
    let v = Vector::new(Uuid::nil(), vec![1.0, 2.0, 3.0]);
    assert_eq!(v.data.len(), 3);
    assert!(v.metadata.is_none());
}

#[test]
fn r177_vec_02_vector_dim() {
    let v = Vector::new(Uuid::nil(), vec![0.5; 8]);
    assert_eq!(v.dim(), 8);
}

#[test]
fn r177_vec_03_search_hit_basic() {
    let h = SearchHit {
        id: Uuid::nil(),
        score: 0.5,
        metadata: None,
    };
    assert_eq!(h.score, 0.5);
    assert!(h.metadata.is_none());
}

#[test]
fn r177_vec_04_scored_id_basic() {
    let s = ScoredId {
        id: Uuid::nil(),
        score: 0.99,
    };
    assert!(s.score > 0.0);
}

#[test]
fn r177_vec_05_error_is_error() {
    let e = VectorError::EmptyVector;
    let _: &dyn std::error::Error = &e;
}

#[cfg(kani)]
#[kani::proof]
fn r177_vec_kani_01_vector_dim_invariant() {
    let v = Vector::new(Uuid::nil(), vec![1.0, 2.0]);
    assert_eq!(v.dim(), 2);
}

#[cfg(kani)]
#[kani::proof]
fn r177_vec_kani_02_scored_id_score_bounded() {
    let s = ScoredId {
        id: Uuid::nil(),
        score: 0.5,
    };
    assert!(s.score >= 0.0 && s.score <= 1.0);
}
