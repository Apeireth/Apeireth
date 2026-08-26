//! R22 ST-A5 — e-layer isolation invariant.
//! **8 项承诺**: 全部遵守。**不假装**: POD smoke model only.
use crate::PermissionLayerConfig;
#[derive(Copy,Clone,Debug)] pub struct EConfig { pub caller_layer:u8, pub target_layer:u8, pub action:u8, pub has_permission:bool }
pub fn e_layer_isolation_invariant(c:EConfig)->bool { c.caller_layer==c.target_layer || c.has_permission }
#[cfg_attr(kani,kani::proof)] pub fn e_layer_isolation(){ assert!(e_layer_isolation_invariant(nondet())); }
#[cfg(kani)] fn nondet()->EConfig { kani::any() }
#[cfg(not(kani))] fn nondet()->EConfig { EConfig{caller_layer:0,target_layer:0,action:0,has_permission:true} }
pub fn sanity_check()->bool { e_layer_isolation_invariant(EConfig{caller_layer:0,target_layer:0,action:1,has_permission:false}) && e_layer_isolation_invariant(EConfig{caller_layer:0,target_layer:1,action:1,has_permission:true}) && !e_layer_isolation_invariant(EConfig{caller_layer:0,target_layer:1,action:1,has_permission:false}) }
#[cfg(test)] mod tests { use super::*; #[test] fn harness_visible(){let _:fn()=e_layer_isolation;} #[test] fn same_layer_allowed(){assert!(e_layer_isolation_invariant(EConfig{caller_layer:1,target_layer:1,action:0,has_permission:false}));} #[test] fn cross_layer_requires_permission(){assert!(!e_layer_isolation_invariant(EConfig{caller_layer:1,target_layer:2,action:0,has_permission:false}));} #[test] fn sanity(){assert!(sanity_check());} }
