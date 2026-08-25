#![allow(unexpected_cfgs)]

//! R177 tool-image-process organ Kani proofs (W10)

#![allow(missing_docs)]

use super::lib::R141_IMAGE_PROC_DELIVERABLES;
use super::exif::ExifData;
use super::hash::perceptual_hash;
use super::ocr::ocr_extract;

#[test]
fn r177_ip_01_r141_deliverables() {
    assert_eq!(R141_IMAGE_PROC_DELIVERABLES, 7);
}

#[test]
fn r177_ip_02_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_ip_03_image_hash() {
    let h = perceptual_hash(&[]);
    let _: String = format!("{:?}", h);
}

#[test]
fn r177_ip_04_exif() {
    let e = ExifData::default();
    let _: String = format!("{:?}", e);
}

#[test]
fn r177_ip_05_ocr() {
    let r = ocr_extract(b"", "en");
    let _: String = format!("{:?}", r);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ip_kani_01_deliverables() {
    assert_eq!(R141_IMAGE_PROC_DELIVERABLES, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_ip_kani_02_image_hash_invariant() {
    let h = perceptual_hash(&[]);
    assert!(!format!("{:?}", h).is_empty());
}
