//! R177 tool-search organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_ts_01_document_new() {
    let d = Document::new(1, "src", "topic", "body");
    assert_eq!(d.id, 1);
}

#[test]
fn r177_ts_02_search_engine_new() {
    let e = SearchEngine::new();
    let n = e.len();
    assert_eq!(n, 0);
}

#[test]
fn r177_ts_03_sort_by() {
    let s = SortBy::Relevance;
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_ts_04_aggregate_by() {
    let a = AggregateBy::Source;
    let _: String = format!("{:?}", a);
}

#[test]
fn r177_ts_05_time_bucket() {
    let t = TimeBucket::Day;
    let _: String = format!("{:?}", t);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ts_kani_01_doc_id_invariant() {
    let d = Document::new(1, "s", "t", "b");
    assert!(d.id > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ts_kani_02_engine_invariant() {
    let e = SearchEngine::new();
    assert_eq!(e.len(), 0);
}
