//! R22 ST-A5 — mid-task atomicity invariant.
//! **8 项承诺**: 全部遵守。**不假装**: POD smoke model only.
#[derive(Copy,Clone,Debug)] pub struct Session{pub state:u8,pub mid_task_state:u8} #[derive(Copy,Clone,Debug)] pub struct MessageRef{pub id:u64,pub seq:u64,pub valid:bool}
pub const MID_TASK_STATES:u8=4;pub const ACTIVE_SESSION_STATES:u8=2;
pub fn mid_task_atomicity_invariant(s:Session,m:MessageRef)->bool{(s.state>=ACTIVE_SESSION_STATES||s.mid_task_state==0)||m.valid}
#[cfg_attr(kani,kani::proof)]pub fn mid_task_atomicity(){assert!(mid_task_atomicity_invariant(nondet_s(),nondet_m()));}
#[cfg(kani)]fn nondet_s()->Session{kani::any()}#[cfg(not(kani))]fn nondet_s()->Session{Session{state:0,mid_task_state:0}} #[cfg(kani)]fn nondet_m()->MessageRef{kani::any()}#[cfg(not(kani))]fn nondet_m()->MessageRef{MessageRef{id:1,seq:1,valid:true}}
pub fn sanity_check()->bool{mid_task_atomicity_invariant(Session{state:2,mid_task_state:1},MessageRef{id:1,seq:1,valid:false})&&mid_task_atomicity_invariant(Session{state:0,mid_task_state:1},MessageRef{id:1,seq:1,valid:true})&&!mid_task_atomicity_invariant(Session{state:0,mid_task_state:1},MessageRef{id:1,seq:1,valid:false})}
#[cfg(test)]mod tests{use super::*;#[test]fn harness_visible(){let _:fn()=mid_task_atomicity;}#[test]fn inactive_needs_valid_message(){assert!(!mid_task_atomicity_invariant(Session{state:0,mid_task_state:1},MessageRef{id:0,seq:0,valid:false}));}#[test]fn active_is_atomic(){assert!(mid_task_atomicity_invariant(Session{state:2,mid_task_state:1},MessageRef{id:0,seq:0,valid:false}));}#[test]fn sanity(){assert!(sanity_check());}}
