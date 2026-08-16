//! R177 host organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::machine_id;
use crate::{derive_id, hash_machine_id};
use uuid::Uuid;

#[test]
fn r177_host_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_host_02_platform_fn() {
    let p = machine_id::platform();
    assert!(!p.as_str().is_empty());
}

#[test]
fn r177_host_03_derive_id() {
    let r = derive_id("test");
    assert!(r.is_ok());
}

#[test]
fn r177_host_04_hash_machine_id() {
    let r = hash_machine_id("test");
    assert!(r.is_ok());
}

#[test]
fn r177_host_05_uuid_namespace() {
    let u = machine_id::uuid_namespace();
    let s = u.to_string();
    assert!(!s.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_host_kani_01_uuid_namespace_invariant() {
    let u = machine_id::uuid_namespace();
    assert!(!u.is_nil());
}

#[cfg(kani)]
#[kani::proof]
fn r177_host_kani_02_validate_uuid() {
    assert!(machine_id::validate_uuid(
        "00000000-0000-0000-0000-000000000000"
    ));
}
