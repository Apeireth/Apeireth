//! R177 tool-shell organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tsh_01_r138_deliverables() {
    assert_eq!(R138_DELIVERABLES, 7);
}

#[test]
fn r177_tsh_02_upgrade_dimensions() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}

#[test]
fn r177_tsh_03_sandbox_policy() {
    let p = SandboxPolicy::default();
    let _: String = format!("{:?}", p);
}

#[test]
fn r177_tsh_04_sandbox_mode() {
    let m = SandboxMode::None;
    let _: String = format!("{:?}", m);
}

#[test]
fn r177_tsh_05_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_tsh_kani_01_deliverables() {
    assert_eq!(R138_DELIVERABLES, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tsh_kani_02_upgrade() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}
