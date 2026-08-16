//! R177 config organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_cfg_01_entry_new() {
    let e = ConfigEntry::new("k", "v", true);
    assert_eq!(e.key, "k");
}

#[test]
fn r177_cfg_02_entry_validate() {
    let e = ConfigEntry::new("k", "v", false);
    assert!(e.validate().is_ok());
}

#[test]
fn r177_cfg_03_entry_required_empty() {
    let e = ConfigEntry::new("k", "", true);
    assert!(e.validate().is_err());
}

#[test]
fn r177_cfg_04_validate_all() {
    let v = vec![ConfigEntry::new("a", "1", false)];
    assert!(validate_all(&v).is_ok());
}

#[test]
fn r177_cfg_05_lookup() {
    let v = vec![ConfigEntry::new("a", "1", false)];
    assert_eq!(lookup(&v, "a"), Some("1"));
}

#[cfg(kani)]
#[kani::proof]
fn r177_cfg_kani_01_key_valid() {
    assert!(key_is_valid("a.b.c"));
}

#[cfg(kani)]
#[kani::proof]
fn r177_cfg_kani_02_key_invalid() {
    assert!(!key_is_valid(""));
}
