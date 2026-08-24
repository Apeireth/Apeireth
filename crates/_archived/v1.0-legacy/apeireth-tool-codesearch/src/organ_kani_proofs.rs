//! R177 tool-codesearch organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_cs_01_r140_deliverables() {
    assert_eq!(R140_DELIVERABLES, 8);
}

#[test]
fn r177_cs_02_supported_langs() {
    assert_eq!(SUPPORTED_LANGS.len(), 5);
}

#[test]
fn r177_cs_03_lang_rust() {
    assert!(SUPPORTED_LANGS.contains(&"rust"));
}

#[test]
fn r177_cs_04_search_kind() {
    let k = SearchKind::Literal;
    let _: String = format!("{:?}", k);
}

#[test]
fn r177_cs_05_search_options() {
    let o = SearchOptions::default();
    let _: String = format!("{:?}", o);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cs_kani_01_deliverables() {
    assert_eq!(R140_DELIVERABLES, 8);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cs_kani_02_langs_count() {
    assert!(SUPPORTED_LANGS.len() >= 1);
}
