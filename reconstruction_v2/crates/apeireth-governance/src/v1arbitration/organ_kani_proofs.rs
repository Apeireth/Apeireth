//! R177 arbitration organ Kani proofs (W9)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_arb_01_event_source_count() {
    assert_eq!(EventSource::COUNT, 6);
}

#[test]
fn r177_arb_02_event_source_all() {
    assert_eq!(EventSource::ALL.len(), 6);
}

#[test]
fn r177_arb_03_event_source_as_str() {
    assert_eq!(EventSource::Frontend.as_str(), "frontend");
}

#[test]
fn r177_arb_04_in_memory_log() {
    let l = ArbitrationLog::open_in_memory();
    assert!(l.is_ok());
}

#[test]
fn r177_arb_05_log_is_empty() {
    let l = ArbitrationLog::open_in_memory().unwrap();
    assert!(l.is_empty().unwrap());
}

#[cfg(kani)]
#[kani::proof]
fn r177_arb_kani_01_event_source_count_invariant() {
    assert_eq!(EventSource::COUNT, 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_arb_kani_02_now_ms_positive() {
    assert!(ArbitrationLog::now_ms() > 0);
}
