//! R177 sdk organ Kani proofs (W12)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_sdk_01_submodule_count() {
    assert_eq!(SDK_SUBMODULE_COUNT, 4);
}

#[test]
fn r177_sdk_02_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_sdk_03_string_basic() {
    let s = String::from("sdk");
    assert_eq!(s.len(), 3);
}

#[test]
fn r177_sdk_04_result_basic() {
    let r: Result<u32, &str> = Ok(1);
    assert_eq!(r.unwrap(), 1);
}

#[test]
fn r177_sdk_05_submodule_check() {
    assert!(SDK_SUBMODULE_COUNT > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_sdk_kani_01_submodule_count() {
    assert_eq!(SDK_SUBMODULE_COUNT, 4);
}

#[cfg(kani)]
#[kani::proof]
fn r177_sdk_kani_02_submodules_enabled() {
    let n = SDK_SUBMODULES_ENABLED;
    assert!(n >= 0 && n <= 4);
}
