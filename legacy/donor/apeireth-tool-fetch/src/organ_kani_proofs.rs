//! R177 tool-fetch organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tf_01_absorbed_plugins() {
    assert_eq!(ABSORBED_LEGACY_PLUGINS, 6);
}

#[test]
fn r177_tf_02_module_count() {
    assert_eq!(MODULE_COUNT, 9);
}

#[test]
fn r177_tf_03_borrowed_from_vcp() {
    assert!(!BORROWED_FROM_VCP.is_empty());
}

#[test]
fn r177_tf_04_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_tf_05_count_positive() {
    assert!(MODULE_COUNT > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tf_kani_01_modules() {
    assert_eq!(MODULE_COUNT, 9);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tf_kani_02_absorbed() {
    assert_eq!(ABSORBED_LEGACY_PLUGINS, 6);
}
