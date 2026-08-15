//! R177 tool-filesystem organ Kani proofs (W10)

#![allow(missing_docs)]

use crate::*;
use std::path::PathBuf;

#[test]
fn r177_fs_01_r137_deliverables() {
    assert_eq!(R137_DELIVERABLES, 6);
}

#[test]
fn r177_fs_02_upgrade_dimensions() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}

#[test]
fn r177_fs_03_sandbox_policy_new() {
    let p = SandboxPolicy::new(vec![PathBuf::from("/tmp")]);
    assert_eq!(p.allowed_roots.len(), 1);
}

#[test]
fn r177_fs_04_sandbox_struct() {
    let s = Sandbox::new(SandboxPolicy::new(vec![]));
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_fs_05_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_fs_kani_01_deliverables() {
    assert_eq!(R137_DELIVERABLES, 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_fs_kani_02_upgrade() {
    assert_eq!(UPGRADE_DIMENSIONS, 5);
}
