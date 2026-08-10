//! Formal verification errors and backend contracts.
use thiserror::Error;
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FormalError { #[error("unknown invariant: {0}")] UnknownInvariant(String), #[error("backend unavailable: {0}")] BackendUnavailable(String), #[error("proof failed: {0}")] ProofFailed(String), #[error("invalid specification: {0}")] InvalidSpecification(String) }
pub type FormalResult<T> = Result<T, FormalError>;
pub trait ProofBackend: Send + Sync { fn name(&self)->&'static str; fn prove(&self, invariant:&crate::invariant::Invariant)->FormalResult<crate::proof::ProofResult>; fn health_check(&self)->bool; }
