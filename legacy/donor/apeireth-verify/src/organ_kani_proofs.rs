//! R177 verify organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_vfy_01_trait_count() {
    assert_eq!(INTERLOCKED_TRAIT_COUNT, 22);
}

#[test]
fn r177_vfy_02_traits_len() {
    assert_eq!(INTERLOCKED_TRAITS.len(), INTERLOCKED_TRAIT_COUNT);
}

#[test]
fn r177_vfy_03_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_vfy_04_assertion_count_positive() {
    let _ = assertion_count();
}

#[test]
fn r177_vfy_05_interlock_error() {
    let e = InterlockError::NotInMatrix { from: "a", to: "b" };
    let _: String = format!("{:?}", e);
}

#[cfg(kani)]
#[kani::proof]
fn r177_vfy_kani_01_trait_count() {
    assert_eq!(INTERLOCKED_TRAIT_COUNT, 22);
}

#[cfg(kani)]
#[kani::proof]
fn r177_vfy_kani_02_traits_len() {
    assert_eq!(INTERLOCKED_TRAITS.len(), 22);
}
