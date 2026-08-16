//! R177 naming-v05 organ Kani proofs (W6)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_nam_01_class_count_4() {
    assert_eq!(CRATE_CLASS_COUNT, 4);
}

#[test]
fn r177_nam_02_dimension_count_6() {
    assert_eq!(CRATE_DIMENSION_COUNT, 6);
}

#[test]
fn r177_nam_03_total_dims_24() {
    assert_eq!(CRATE_V05_TOTAL_DIMS, 24);
}

#[test]
fn r177_nam_04_prefix() {
    assert_eq!(CRATE_PREFIX, "apeireth:");
}

#[test]
fn r177_nam_05_naming_result() {
    let r: NamingResult<u32> = Ok(1);
    assert_eq!(r.unwrap(), 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_nam_kani_01_total_dims() {
    assert_eq!(CRATE_CLASS_COUNT * CRATE_DIMENSION_COUNT, 24);
}

#[cfg(kani)]
#[kani::proof]
fn r177_nam_kani_02_variant_count() {
    assert!(NAMING_ERROR_VARIANT_COUNT >= 1);
}
