//! R177 environment organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_env_01_backend_all() {
    assert_eq!(BackendKind::ALL.len(), 6);
}

#[test]
fn r177_env_02_backend_as_str() {
    assert_eq!(BackendKind::Local.as_str(), "local");
}

#[test]
fn r177_env_03_backend_is_local() {
    assert!(BackendKind::Local.is_local());
}

#[test]
fn r177_env_04_backend_is_remote() {
    assert!(BackendKind::Ssh.is_remote());
}

#[test]
fn r177_env_05_backend_is_container() {
    assert!(BackendKind::Docker.is_container());
}

#[cfg(kani)]
#[kani::proof]
fn r177_env_kani_01_backend_local() {
    assert!(BackendKind::Local.is_local());
}

#[cfg(kani)]
#[kani::proof]
fn r177_env_kani_02_backend_not_local() {
    assert!(!BackendKind::Docker.is_local());
}

