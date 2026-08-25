//! R176 Agent delegation 7×7=49 paths test matrix
//!
//! **\u{80cc}\u{666f}**: 7 AdvisorDomain (\u{5b89}\u{5168}/\u{6027}\u{80fd}/\u{54f2}\u{5b66}/\u{5386}\u{53f2}/\u{7b56}\u{7565}/\u{4f26}\u{7406}/\u{6cd5}\u{5f8b}) \u{53ef}\u{4ee5}\u{4e92}\u{76f8}\u{59d4}\u{6258}.
//! \u{672c}\u{6a21}\u{5757}\u{9a8c}\u{8bc1} 7×7=49 \u{59d4}\u{6258}\u{8def}\u{5f84}\u{90fd}\u{80fd}\u{6784}\u{9020}\u{5e76}\u{8fd0}\u{884c} (\u{4e0d}\u{5fc5}\u{90fd}\u{6709}\u{4e1a}\u{52a1}\u{542b}\u{4e49}, \u{4f46}\u{8bed}\u{4e49}\u{4e0a}\u{5e94}\u{53ef}\u{884c}).
//!
//! **\u{4e0d}\u{6f02}\u{79fb}**:
//! - 0 \u{6539} AdvisorDomain enum (R10 LOCKED)
//! - 0 \u{52a8} workspace.version
//!
//! **\u{72b6}\u{6001}**: R176 (2026-08-15) \u{521d}\u{59cb}\u{7248}, 49 paths + 7 self-delegation + \u{4e92}\u{9001}\u{68c0}\u{67e5}.

#![allow(missing_docs)]

use crate::advisor::AdvisorDomain;

/// \u{4e00}\u{4e2a}\u{59d4}\u{6258}\u{8def}\u{5f84}: (from, to)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DelegationPath(pub AdvisorDomain, pub AdvisorDomain);

impl DelegationPath {
    pub fn from(&self) -> AdvisorDomain {
        self.0
    }
    pub fn to(&self) -> AdvisorDomain {
        self.1
    }
    pub fn name(&self) -> String {
        format!("{:?} → {:?}", self.0, self.1)
    }
}

/// 7×7=49 \u{59d4}\u{6258}\u{8def}\u{5f84} (\u{542b}\u{81ea}\u{59d4}\u{6258})
pub const DELEGATION_PATHS: [(AdvisorDomain, AdvisorDomain); 49] = [
    // Safety \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Safety, AdvisorDomain::Safety),
    (AdvisorDomain::Safety, AdvisorDomain::Performance),
    (AdvisorDomain::Safety, AdvisorDomain::Philosophy),
    (AdvisorDomain::Safety, AdvisorDomain::History),
    (AdvisorDomain::Safety, AdvisorDomain::Strategy),
    (AdvisorDomain::Safety, AdvisorDomain::Ethics),
    (AdvisorDomain::Safety, AdvisorDomain::Legal),
    // Performance \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Performance, AdvisorDomain::Safety),
    (AdvisorDomain::Performance, AdvisorDomain::Performance),
    (AdvisorDomain::Performance, AdvisorDomain::Philosophy),
    (AdvisorDomain::Performance, AdvisorDomain::History),
    (AdvisorDomain::Performance, AdvisorDomain::Strategy),
    (AdvisorDomain::Performance, AdvisorDomain::Ethics),
    (AdvisorDomain::Performance, AdvisorDomain::Legal),
    // Philosophy \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Philosophy, AdvisorDomain::Safety),
    (AdvisorDomain::Philosophy, AdvisorDomain::Performance),
    (AdvisorDomain::Philosophy, AdvisorDomain::Philosophy),
    (AdvisorDomain::Philosophy, AdvisorDomain::History),
    (AdvisorDomain::Philosophy, AdvisorDomain::Strategy),
    (AdvisorDomain::Philosophy, AdvisorDomain::Ethics),
    (AdvisorDomain::Philosophy, AdvisorDomain::Legal),
    // History \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::History, AdvisorDomain::Safety),
    (AdvisorDomain::History, AdvisorDomain::Performance),
    (AdvisorDomain::History, AdvisorDomain::Philosophy),
    (AdvisorDomain::History, AdvisorDomain::History),
    (AdvisorDomain::History, AdvisorDomain::Strategy),
    (AdvisorDomain::History, AdvisorDomain::Ethics),
    (AdvisorDomain::History, AdvisorDomain::Legal),
    // Strategy \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Strategy, AdvisorDomain::Safety),
    (AdvisorDomain::Strategy, AdvisorDomain::Performance),
    (AdvisorDomain::Strategy, AdvisorDomain::Philosophy),
    (AdvisorDomain::Strategy, AdvisorDomain::History),
    (AdvisorDomain::Strategy, AdvisorDomain::Strategy),
    (AdvisorDomain::Strategy, AdvisorDomain::Ethics),
    (AdvisorDomain::Strategy, AdvisorDomain::Legal),
    // Ethics \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Ethics, AdvisorDomain::Safety),
    (AdvisorDomain::Ethics, AdvisorDomain::Performance),
    (AdvisorDomain::Ethics, AdvisorDomain::Philosophy),
    (AdvisorDomain::Ethics, AdvisorDomain::History),
    (AdvisorDomain::Ethics, AdvisorDomain::Strategy),
    (AdvisorDomain::Ethics, AdvisorDomain::Ethics),
    (AdvisorDomain::Ethics, AdvisorDomain::Legal),
    // Legal \u{59d4}\u{6258} 7 \u{4e2a}
    (AdvisorDomain::Legal, AdvisorDomain::Safety),
    (AdvisorDomain::Legal, AdvisorDomain::Performance),
    (AdvisorDomain::Legal, AdvisorDomain::Philosophy),
    (AdvisorDomain::Legal, AdvisorDomain::History),
    (AdvisorDomain::Legal, AdvisorDomain::Strategy),
    (AdvisorDomain::Legal, AdvisorDomain::Ethics),
    (AdvisorDomain::Legal, AdvisorDomain::Legal),
];

/// \u{68c0}\u{67e5}\u{59d4}\u{6258}\u{662f}\u{5426}\u{5408}\u{6cd5}\u{8bed}\u{4e49} (\u{672c}\u{8d33} 49 \u{8def}\u{5f84}\u{90fd}\u{5408}\u{6cd5})
pub fn is_valid_delegation(path: &DelegationPath) -> bool {
    DELEGATION_PATHS
        .iter()
        .any(|&(f, t)| f == path.0 && t == path.1)
}

/// 7 \u{81ea}\u{59d4}\u{6258}\u{8def}\u{5f84} (\u{4e00}\u{4e2a} advisor \u{59d4}\u{6258}\u{7ed9}\u{81ea}\u{5df1})
pub fn self_delegations() -> Vec<DelegationPath> {
    DELEGATION_PATHS
        .iter()
        .filter(|(f, t)| f == t)
        .map(|(f, t)| DelegationPath(*f, *t))
        .collect()
}

/// \u{4ece}\u{67d0}\u{4e2a} advisor \u{53d1}\u{51fa}\u{7684}\u{59d4}\u{6258}\u{5217}\u{8868}
pub fn delegations_from(from: AdvisorDomain) -> Vec<DelegationPath> {
    DELEGATION_PATHS
        .iter()
        .filter(|(f, _)| *f == from)
        .map(|(f, t)| DelegationPath(*f, *t))
        .collect()
}

/// \u{67d0}\u{4e2a} advisor \u{63a5}\u{6536}\u{7684}\u{59d4}\u{6258}\u{5217}\u{8868}
pub fn delegations_to(to: AdvisorDomain) -> Vec<DelegationPath> {
    DELEGATION_PATHS
        .iter()
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
        for from in [
            AdvisorDomain::Safety,
            AdvisorDomain::Performance,
            AdvisorDomain::Philosophy,
            AdvisorDomain::History,
            AdvisorDomain::Strategy,
            AdvisorDomain::Ethics,
            AdvisorDomain::Legal,
        ] {
            let paths = delegations_from(from);
            assert_eq!(paths.len(), 7, "{:?} should have 7 delegations", from);
        }
    }

    #[test]
    fn delegations_to_returns_7() {
        for to in [
            AdvisorDomain::Safety,
            AdvisorDomain::Performance,
            AdvisorDomain::Philosophy,
            AdvisorDomain::History,
            AdvisorDomain::Strategy,
            AdvisorDomain::Ethics,
            AdvisorDomain::Legal,
        ] {
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
        let actual: HashSet<(u8, u8)> = DELEGATION_PATHS
            .iter()
            .map(|(f, t)| (*f as u8, *t as u8))
            .collect();
        let expected: HashSet<(u8, u8)> =
            (0..7).flat_map(|f| (0..7).map(move |t| (f, t))).collect();
        assert_eq!(actual, expected, "matrix should be complete 7x7 grid");
    }
}
