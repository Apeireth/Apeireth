//! Minimal TLA+ expression/spec representation for exported proofs.
use crate::error::{FormalError,FormalResult};
#[derive(Debug,Clone,PartialEq,Eq)] pub struct TlaExpr(pub String); impl TlaExpr { pub fn atom(value:impl Into<String>)->Self{Self(value.into())} }
#[derive(Debug,Clone,PartialEq,Eq)] pub struct TlaSpec { pub name:String, pub variables:Vec<String>, pub invariant:String }
impl TlaSpec { pub fn validate(&self)->FormalResult<()> {if self.name.trim().is_empty(){Err(FormalError::InvalidSpecification("name is empty".into()))}else{Ok(())}} pub fn render(&self)->String{format!("---- MODULE {} ----\nVARIABLES {}\nInv == {}\n====",self.name,self.variables.join(", "),self.invariant)} }
