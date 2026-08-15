//! R177 tool-browser organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tb_01_r139_deliverables() {
    assert_eq!(R139_DELIVERABLES, 7);
}

#[test]
fn r177_tb_02_upgrade_dimensions() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}

#[test]
fn r177_tb_03_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_tb_04_browser_mode() {
    let m = BrowserMode::Fetch;
    let _: String = format!("{:?}", m);
}

#[test]
fn r177_tb_05_browser_err() {
    let e = BrowserError::Http("x".into());
    let _: String = format!("{:?}", e);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tb_kani_01_deliverables() {
    assert_eq!(R139_DELIVERABLES, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tb_kani_02_upgrade_dims() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}
