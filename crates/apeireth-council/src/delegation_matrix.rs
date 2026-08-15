
//! R176 Agent delegation 7×7=49 paths test matrix
//!
//! **\u80cc\u666f**: 7 AdvisorDomain (\u5b89\u5168/\u6027\u80fd/\u54f2\u5b66/\u5386\u53f2/\u7b56\u7565/\u4f26\u7406/\u6cd5\u5f8b) \u53ef\u4ee5\u4e92\u76f8\u59d4\u6258.
//! \u672c\u6a21\u5757\u9a8c\u8bc1 7×7=49 \u59d4\u6258\u8def\u5f84\u90fd\u80fd\u6784\u9020\u5e76\u8fd0\u884c (\u4e0d\u5fc5\u90fd\u6709\u4e1a\u52a1\u542b\u4e49, \u4f46\u8bed\u4e49\u4e0a\u5e94\u53ef\u884c).
//!
//! **\u4e0d\u6f02\u79fb**:
//! - 0 \u6539 AdvisorDomain enum (R10 LOCKED)
//! - 0 \u52a8 workspace.version
//!
//! **\u72b6\u6001**: R176 (2026-08-15) \u521d\u59cb\u7248, 49 paths + 7 self-delegation + \u4e92\u9001\u68c0\u67e5.

#![allow(missing_docs)]

use crate::advisor::AdvisorDomain;

/// \u4e00\u4e2a\u59d4\u6258\u8def\u5f84: (from, to)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DelegationPath(pub AdvisorDomain, pub AdvisorDomain);

impl DelegationPath {
    pub fn from(&self) -> AdvisorDomain { self.0 }
    pub fn to(&self) -> AdvisorDomain { self.1 }
    pub fn name(&self) -> String {
        format!("{:?} → {:?}", self.0, self.1)
    }
}

/// 7×7=49 \u59d4\u6258\u8def\u5f84 (\u542b\u81ea\u59d4\u6258)
pub const DELEGATION_PATHS: [(AdvisorDomain, AdvisorDomain); 49] = [
    // Safety \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Safety, AdvisorDomain::Safety),
    (AdvisorDomain::Safety, AdvisorDomain::Performance),
    (AdvisorDomain::Safety, AdvisorDomain::Philosophy),
    (AdvisorDomain::Safety, AdvisorDomain::History),
    (AdvisorDomain::Safety, AdvisorDomain::Strategy),
    (AdvisorDomain::Safety, AdvisorDomain::Ethics),
    (AdvisorDomain::Safety, AdvisorDomain::Legal),
    // Performance \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Performance, AdvisorDomain::Safety),
    (AdvisorDomain::Performance, AdvisorDomain::Performance),
    (AdvisorDomain::Performance, AdvisorDomain::Philosophy),
    (AdvisorDomain::Performance, AdvisorDomain::History),
    (AdvisorDomain::Performance, AdvisorDomain::Strategy),
    (AdvisorDomain::Performance, AdvisorDomain::Ethics),
    (AdvisorDomain::Performance, AdvisorDomain::Legal),
    // Philosophy \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Philosophy, AdvisorDomain::Safety),
    (AdvisorDomain::Philosophy, AdvisorDomain::Performance),
    (AdvisorDomain::Philosophy, AdvisorDomain::Philosophy),
    (AdvisorDomain::Philosophy, AdvisorDomain::History),
    (AdvisorDomain::Philosophy, AdvisorDomain::Strategy),
    (AdvisorDomain::Philosophy, AdvisorDomain::Ethics),
    (AdvisorDomain::Philosophy, AdvisorDomain::Legal),
    // History \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::History, AdvisorDomain::Safety),
    (AdvisorDomain::History, AdvisorDomain::Performance),
    (AdvisorDomain::History, AdvisorDomain::Philosophy),
    (AdvisorDomain::History, AdvisorDomain::History),
    (AdvisorDomain::History, AdvisorDomain::Strategy),
    (AdvisorDomain::History, AdvisorDomain::Ethics),
    (AdvisorDomain::History, AdvisorDomain::Legal),
    // Strategy \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Strategy, AdvisorDomain::Safety),
    (AdvisorDomain::Strategy, AdvisorDomain::Performance),
    (AdvisorDomain::Strategy, AdvisorDomain::Philosophy),
    (AdvisorDomain::Strategy, AdvisorDomain::History),
    (AdvisorDomain::Strategy, AdvisorDomain::Strategy),
    (AdvisorDomain::Strategy, AdvisorDomain::Ethics),
    (AdvisorDomain::Strategy, AdvisorDomain::Legal),
    // Ethics \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Ethics, AdvisorDomain::Safety),
    (AdvisorDomain::Ethics, AdvisorDomain::Performance),
    (AdvisorDomain::Ethics, AdvisorDomain::Philosophy),
    (AdvisorDomain::Ethics, AdvisorDomain::History),
    (AdvisorDomain::Ethics, AdvisorDomain::Strategy),
    (AdvisorDomain::Ethics, AdvisorDomain::Ethics),
    (AdvisorDomain::Ethics, AdvisorDomain::Legal),
    // Legal \u59d4\u6258 7 \u4e2a
    (AdvisorDomain::Legal, AdvisorDomain::Safety),
    (AdvisorDomain::Legal, AdvisorDomain::Performance),
    (AdvisorDomain::Legal, AdvisorDomain::Philosophy),
    (AdvisorDomain::Legal, AdvisorDomain::History),
    (AdvisorDomain::Legal, AdvisorDomain::Strategy),
    (AdvisorDomain::Legal, AdvisorDomain::Ethics),
    (AdvisorDomain::Legal, AdvisorDomain::Legal),
];

/// \u68c0\u67e5\u59d4\u6258\u662f\u5426\u5408\u6cd5\u8bed\u4e49 (\u672c\u8d33 49 \u8def\u5f84\u90fd\u5408\u6cd5)
pub fn is_valid_delegation(path: &DelegationPath) -> bool {
    DELEGATION_PATHS.iter().any(|&(f, t)| f == path.0 && t == path.1)
}

/// 7 \u81ea\u59d4\u6258\u8def\u5f84 (\u4e00\u4e2a advisor \u59d4\u6258\u7ed9\u81ea\u5df1)
pub fn self_delegations() -> Vec<DelegationPath> {
    DELEGATION_PATHS.iter()
        .filter(|(f, t)| f == t)
        .map(|(f, t)| DelegationPath(*f, *t))
        .collect()
}

/// \u4ece\u67d0\u4e2a advisor \u53d1\u51fa\u7684\u59d4\u6258\u5217\u8868
pub fn delegations_from(from: AdvisorDomain) -> Vec<DelegationPath> {
    DELEGATION_PATHS.iter()
        .filter(|(f, _)| *f == from)
        .map(|(f, t)| DelegationPath(*f, *t))
        .collect()
}

/// \u67d0\u4e2a advisor \u63a5\u6536\u7684\u59d4\u6258\u5217\u8868
pub fn delegations_to(to: AdvisorDomain) -> Vec<DelegationPath> {
    DELEGATION_PATHS.iter()
        .filter(|(_, t)| *t == to)
        .map(|(f, t)| DelegationPath(*f, *t))
        .collect()
}

#[cfg(test)]
mod delegation_matrix_tests {
    use super::*;

    #[test]
    fn delegation_matrix_count_is_49() {
        assert_eq!(DELEGATION_PATHS.len(), 49, "7x7=49 paths");
    }

    #[test]
    fn self_delegations_count_is_7() {
        let s = self_delegations();
        assert_eq!(s.len(), 7);
        for d in &s {
            assert_eq!(d.from(), d.to());
        }
    }

    #[test]
    fn delegations_from_returns_7() {
        for from in [AdvisorDomain::Safety, AdvisorDomain::Performance, AdvisorDomain::Philosophy,
                     AdvisorDomain::History, AdvisorDomain::Strategy, AdvisorDomain::Ethics,
                     AdvisorDomain::Legal] {
            let paths = delegations_from(from);
            assert_eq!(paths.len(), 7, "{:?} should have 7 delegations", from);
        }
    }

    #[test]
    fn delegations_to_returns_7() {
        for to in [AdvisorDomain::Safety, AdvisorDomain::Performance, AdvisorDomain::Philosophy,
                   AdvisorDomain::History, AdvisorDomain::Strategy, AdvisorDomain::Ethics,
                   AdvisorDomain::Legal] {
            let paths = delegations_to(to);
            assert_eq!(paths.len(), 7, "{:?} should receive 7 delegations", to);
        }
    }

    #[test]
    fn is_valid_delegation_for_all_49() {
        // Walk all 49 paths
        for i in 0..49 {
            let path = DelegationPath(DELEGATION_PATHS[i].0, DELEGATION_PATHS[i].1);
            assert!(is_valid_delegation(&path), "path {} not valid", path.name());
        }
    }

    #[test]
    fn delegation_path_name_format() {
        let p = DelegationPath(AdvisorDomain::Safety, AdvisorDomain::Legal);
        let n = p.name();
        assert!(n.contains("Safety"));
        assert!(n.contains("Legal"));
        assert!(n.contains("→"));
    }

    #[test]
    fn delegation_path_equality() {
        let p1 = DelegationPath(AdvisorDomain::Safety, AdvisorDomain::Legal);
        let p2 = DelegationPath(AdvisorDomain::Safety, AdvisorDomain::Legal);
        let p3 = DelegationPath(AdvisorDomain::Legal, AdvisorDomain::Safety);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn delegation_matrix_unique_pairs() {
        // No duplicate (from, to) pairs
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for (f, t) in DELEGATION_PATHS.iter() {
            let key = (*f as u8, *t as u8);
            assert!(seen.insert(key), "duplicate delegation: {:?} → {:?}", f, t);
        }
    }

    #[test]
    fn delegation_matrix_covers_all_pairs() {
        // Every (from, to) combination must exist
        use std::collections::HashSet;
        let actual: HashSet<(u8, u8)> = DELEGATION_PATHS.iter().map(|(f, t)| (*f as u8, *t as u8)).collect();
        let expected: HashSet<(u8, u8)> = (0..7).flat_map(|f| (0..7).map(move |t| (f, t))).collect();
        assert_eq!(actual, expected, "matrix should be complete 7x7 grid");
    }
}
