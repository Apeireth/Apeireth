//! R177 council organ Kani proofs (W3) — delegation matrix 49 paths

#![allow(missing_docs)]

use crate::advisor::AdvisorDomain;
use crate::delegation_matrix::{
    delegations_from, delegations_to, is_valid_delegation, self_delegations, DelegationPath,
    DELEGATION_PATHS,
};

#[test]
fn r177_cou_01_delegation_paths_49() {
    assert_eq!(DELEGATION_PATHS.len(), 49, "7×7=49 paths");
}

#[test]
fn r177_cou_02_delegation_path_accessors() {
    let p = DelegationPath(AdvisorDomain::Safety, AdvisorDomain::Performance);
    assert_eq!(p.from(), AdvisorDomain::Safety);
    assert_eq!(p.to(), AdvisorDomain::Performance);
    assert!(!p.name().is_empty());
}

#[test]
fn r177_cou_03_is_valid_delegation() {
    assert!(is_valid_delegation(&DelegationPath(
        AdvisorDomain::Safety,
        AdvisorDomain::Performance
    )));
    assert!(is_valid_delegation(&DelegationPath(
        AdvisorDomain::Safety,
        AdvisorDomain::Safety
    )));
}

#[test]
fn r177_cou_04_self_delegations_7() {
    let s = self_delegations();
    assert_eq!(s.len(), 7, "7 个 self-delegations");
    for d in &s {
        assert_eq!(d.from(), d.to(), "self-delegation from==to");
    }
}

#[test]
fn r177_cou_05_delegations_from_count() {
    let paths = delegations_from(AdvisorDomain::Safety);
    assert_eq!(paths.len(), 7, "Safety → 7 个目标 (含 self)");
}

#[test]
fn r177_cou_06_delegations_to_count() {
    let paths = delegations_to(AdvisorDomain::Performance);
    assert_eq!(paths.len(), 7, "→ Performance 共 7 个来源 (含 self)");
}

#[test]
fn r177_cou_07_delegations_from_to_consistent() {
    let p1 = DelegationPath(AdvisorDomain::Ethics, AdvisorDomain::Legal);
    let p2 = DelegationPath(AdvisorDomain::Legal, AdvisorDomain::Ethics);
    assert!(is_valid_delegation(&p1));
    assert!(is_valid_delegation(&p2));
    assert_ne!(p1, p2);
}

#[test]
fn r177_cou_08_delegation_hash_eq() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(DelegationPath(
        AdvisorDomain::Safety,
        AdvisorDomain::Performance,
    ));
    assert!(set.contains(&DelegationPath(
        AdvisorDomain::Safety,
        AdvisorDomain::Performance
    )));
    assert!(!set.contains(&DelegationPath(
        AdvisorDomain::Performance,
        AdvisorDomain::Safety
    )));
}

#[test]
fn r177_cou_09_all_7_domains_in_self_delegations() {
    let s = self_delegations();
    let domains: std::collections::HashSet<_> = s.iter().map(|d| d.from()).collect();
    assert_eq!(domains.len(), 7);
}

#[test]
fn r177_cou_10_no_duplicate_paths() {
    use std::collections::HashSet;
    let mut set: HashSet<DelegationPath> = HashSet::new();
    for p in &DELEGATION_PATHS {
        let path = DelegationPath(p.0, p.1);
        assert!(set.insert(path), "DELEGATION_PATHS 重复");
    }
    assert_eq!(set.len(), 49);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cou_kani_01_path_count() {
    assert_eq!(DELEGATION_PATHS.len(), 49);
}

#[cfg(kani)]
#[kani::proof]
fn r177_cou_kani_02_self_delegation_count() {
    let s = self_delegations();
    assert_eq!(s.len(), 7);
}
