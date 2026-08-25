//! R177 tool-image-gen organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_ig_01_r141_deliverables() {
    assert_eq!(R141_DELIVERABLES, 7);
}

#[test]
fn r177_ig_02_provider_count() {
    assert_eq!(PROVIDER_COUNT, 13);
}

#[test]
fn r177_ig_03_provider_kind() {
    let k = ProviderKind::OpenAiDallE;
    let _: String = format!("{:?}", k);
}

#[test]
fn r177_ig_04_image_size() {
    let s = ImageSize::Small;
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_ig_05_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_ig_kani_01_deliverables() {
    assert_eq!(R141_DELIVERABLES, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ig_kani_02_provider_count() {
    assert_eq!(PROVIDER_COUNT, 13);
}
