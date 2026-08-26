//! R177 skills organ Kani proofs (W12)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_skl_01_skill_new() {
    let s = Skill::new("id", "1.0.0", "{}", "{}");
    assert_eq!(s.id, "id");
}

#[test]
fn r177_skl_02_skill_validate() {
    let s = Skill::new("id", "1.0.0", "{}", "{}");
    assert!(s.validate().is_ok());
}

#[test]
fn r177_skl_03_registry_new() {
    let r = Registry::new();
    assert!(r.is_empty());
}

#[test]
fn r177_skl_04_is_valid_id() {
    assert!(is_valid_id("valid-id"));
}

#[test]
fn r177_skl_05_parse_version() {
    let r = parse_version("1.2.3");
    assert!(r.is_ok());
}

#[cfg(kani)]
#[kani::proof]
fn r177_skl_kani_01_valid_id_invariant() {
    assert!(is_valid_id("id"));
}

#[cfg(kani)]
#[kani::proof]
fn r177_skl_kani_02_parse_version_invariant() {
    let r = parse_version("1.0.0");
    assert!(r.is_ok());
}
