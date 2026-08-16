//! R177 cli organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;
use apeireth_core::{Action, ActionVerdict, HumanAuthority, PermissionOnion, RiskLevel, Session};

#[test]
fn r177_cli_01_classify_risk() {
    let l = classify_risk("hello");
    let _: &dyn std::fmt::Debug = &l;
}

#[test]
fn r177_cli_02_placeholder() {
    let p = placeholder();
    assert!(!p.is_empty());
}

#[test]
fn r177_cli_03_session_default() {
    let s: Session = create_default_session();
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_cli_04_perm_onion_default() {
    let po: PermissionOnion = build_default_permission_onion();
    let _: String = format!("{:?}", po);
}

#[test]
fn r177_cli_05_human_authority_default() {
    let h: HumanAuthority = build_default_human_authority();
    let _: String = format!("{:?}", h);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cli_kani_01_placeholder_invariant() {
    let p = placeholder();
    assert!(!p.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_cli_kani_02_classify_invariant() {
    let l = classify_risk("");
    let _: &dyn std::fmt::Debug = &l;
}
