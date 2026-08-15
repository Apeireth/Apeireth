//! R177 agent organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_agt_01_borrowed_legacy_fields() {
    assert_eq!(BORROWED_LEGACY_FIELDS, 9);
}

#[test]
fn r177_agt_02_agent_field_count() {
    assert_eq!(AGENT_FIELD_COUNT, 6);
}

#[test]
fn r177_agt_03_event_variant_count() {
    assert_eq!(AGENT_EVENT_VARIANT_COUNT, 4);
}

#[test]
fn r177_agt_04_default_cache_size() {
    assert!(DEFAULT_CACHE_SIZE_CONST >= 1);
}

#[test]
fn r177_agt_05_alias_prefix() {
    let p = ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX_CONST; assert!(!p.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_agt_kani_01_field_count_invariant() {
    assert_eq!(AGENT_FIELD_COUNT, 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_agt_kani_02_event_invariant() {
    assert_eq!(AGENT_EVENT_VARIANT_COUNT, 4);
}

