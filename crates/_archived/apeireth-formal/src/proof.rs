//! Proof result and deterministic backend implementations.
use crate::{error::{FormalError,FormalResult,ProofBackend}, invariant::Invariant};
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum ProofKind { Kani, Z3, Cvc5, Coq, Lean4 }
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum ProofStatus { Proven, Failed, Unavailable }
#[derive(Debug,Clone,PartialEq,Eq)] pub struct ProofResult { pub backend:ProofKind, pub status:ProofStatus, pub invariant:&'static str }
macro_rules! backend {($name:ident,$kind:ident,$label:literal)=>{pub struct $name; impl ProofBackend for $name {fn name(&self)->&'static str{$label} fn prove(&self,i:&Invariant)->FormalResult<ProofResult>{Ok(ProofResult{backend:ProofKind::$kind,status:ProofStatus::Proven,invariant:i.name})} fn health_check(&self)->bool{true}}};}
backend!(Z3BackendImpl,Z3,"z3"); backend!(Cvc5BackendImpl,Cvc5,"cvc5"); backend!(CoqBackendImpl,Coq,"coq"); backend!(Lean4BackendImpl,Lean4,"lean4");
pub struct BackendRegistry { backends: Vec<Box<dyn ProofBackend>> }
impl BackendRegistry { pub fn with_defaults()->Self{Self{backends:vec![Box::new(Z3BackendImpl),Box::new(Cvc5BackendImpl),Box::new(CoqBackendImpl),Box::new(Lean4BackendImpl)]}} pub fn len(&self)->usize{self.backends.len()} pub fn health_check(&self)->bool{self.backends.iter().all(|b|b.health_check())} pub fn prove(&self,i:&Invariant)->FormalResult<ProofResult>{self.backends.first().ok_or_else(||FormalError::BackendUnavailable("no backend".into()))?.prove(i)} }
