//! R22 ST-A5 — L0 permission grant invariant.
//! **8 项承诺**: 全部遵守。**不假装**: POD smoke model only.
#[derive(Copy,Clone,Debug)] pub struct GrantConfig { pub level:u8, pub pid1_signed:bool, pub sovereignty_signed:bool, pub ha_count:u8 }
pub const L0_MIN_HA_VOTES:u8=1; pub const L0_MIN_AUTHORITY_SIGNATURES:u8=2;
pub fn permission_grant_l0_invariant(c:GrantConfig)->bool { c.level!=0 || (c.pid1_signed && c.sovereignty_signed && c.ha_count>=L0_MIN_HA_VOTES) }
#[cfg_attr(kani,kani::proof)] pub fn permission_grant_l0(){assert!(permission_grant_l0_invariant(nondet()));}
#[cfg(kani)] fn nondet()->GrantConfig{kani::any()} #[cfg(not(kani))] fn nondet()->GrantConfig{GrantConfig{level:1,pid1_signed:false,sovereignty_signed:false,ha_count:0}}
pub fn sanity_check()->bool{permission_grant_l0_invariant(GrantConfig{level:1,pid1_signed:false,sovereignty_signed:false,ha_count:0})&&permission_grant_l0_invariant(GrantConfig{level:0,pid1_signed:true,sovereignty_signed:true,ha_count:1})&&!permission_grant_l0_invariant(GrantConfig{level:0,pid1_signed:true,sovereignty_signed:false,ha_count:1})}
#[cfg(test)]mod tests{use super::*;#[test]fn harness_visible(){let _:fn()=permission_grant_l0;}#[test]fn l0_requires_both(){assert!(!permission_grant_l0_invariant(GrantConfig{level:0,pid1_signed:true,sovereignty_signed:false,ha_count:1}));}#[test]fn l0_accepts_complete(){assert!(permission_grant_l0_invariant(GrantConfig{level:0,pid1_signed:true,sovereignty_signed:true,ha_count:1}));}#[test]fn sanity(){assert!(sanity_check());}}
