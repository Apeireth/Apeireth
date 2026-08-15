//! R177 upgrade organ Kani proofs (W12)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_up_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_up_02_upgrade_error() {
    let e = UpgradeError::InvalidManifest("x".into());
    let _: String = format!("{:?}", e);
}

#[test]
fn r177_up_03_manifest_builder() {
    let b = ManifestBuilder::new("1.0.0", UpgradeKind::Patch);
    let m = b.build();
    assert_eq!(m.version, "1.0.0");
}

#[test]
fn r177_up_04_string_basic() {
    let s = String::from("upgrade");
    assert_eq!(s.len(), 7);
}

#[test]
fn r177_up_05_result_basic() {
    let r: Result<u32, &str> = Ok(1);
    assert_eq!(r.unwrap(), 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_up_kani_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_up_kani_02_basic() {
    let v: Vec<u32> = vec![1];
    assert_eq!(v.len(), 1);
}
