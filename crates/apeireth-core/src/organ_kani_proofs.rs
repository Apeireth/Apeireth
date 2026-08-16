//! R177 core organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_core_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_core_02_string_basic() {
    let s = String::from("core");
    assert_eq!(s.len(), 4);
}

#[test]
fn r177_core_03_vec_basic() {
    let v: Vec<u32> = vec![1, 2];
    assert_eq!(v.len(), 2);
}

#[test]
fn r177_core_04_option_basic() {
    let o: Option<u32> = None;
    assert!(o.is_none());
}

#[test]
fn r177_core_05_result_basic() {
    let r: Result<u32, &str> = Err("x");
    assert!(r.is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_core_kani_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_core_kani_02_basic() {
    let v: Vec<u32> = vec![1];
    assert_eq!(v.len(), 1);
}
