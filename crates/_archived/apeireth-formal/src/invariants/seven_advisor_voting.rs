//! R22 ST-A5 — seven-advisor council completeness invariant.
//! **8 项承诺**: 全部遵守。**不假装**: POD smoke model only.
#[derive(Copy,Clone,Debug)]pub struct CouncilConfig{pub advisor_count:u8,pub present_mask:u8} pub const COUNCIL_ADVISOR_COUNT:u8=7;
pub fn seven_advisor_voting_invariant(c:CouncilConfig)->bool{c.advisor_count!=COUNCIL_ADVISOR_COUNT||(c.present_mask&0x7f)==0x7f}
#[cfg_attr(kani,kani::proof)]pub fn seven_advisor_voting(){assert!(seven_advisor_voting_invariant(nondet()));}#[cfg(kani)]fn nondet()->CouncilConfig{kani::any()}#[cfg(not(kani))]fn nondet()->CouncilConfig{CouncilConfig{advisor_count:7,present_mask:0x7f}}
pub fn sanity_check()->bool{seven_advisor_voting_invariant(CouncilConfig{advisor_count:6,present_mask:0})&&seven_advisor_voting_invariant(CouncilConfig{advisor_count:7,present_mask:0x7f})&&!seven_advisor_voting_invariant(CouncilConfig{advisor_count:7,present_mask:0x3f})}
#[cfg(test)]mod tests{use super::*;#[test]fn harness_visible(){let _:fn()=seven_advisor_voting;}#[test]fn exactly_seven_requires_mask(){assert!(!seven_advisor_voting_invariant(CouncilConfig{advisor_count:7,present_mask:0x3f}));}#[test]fn complete_council_passes(){assert!(seven_advisor_voting_invariant(CouncilConfig{advisor_count:7,present_mask:0x7f}));}#[test]fn nonseven_is_not_this_guard(){assert!(seven_advisor_voting_invariant(CouncilConfig{advisor_count:6,present_mask:0}));}}
