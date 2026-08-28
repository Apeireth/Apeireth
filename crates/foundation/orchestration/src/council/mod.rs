//! Council adapters and compatibility exports.
//!
//! The typed Council implementation lives at the crate root so the runtime
//! and legacy callers share one aggregation contract. This module exposes the
//! real LLM advisor slots and keeps the historical `council::Council` path.

use std::time::Duration;

pub mod advisors_llm;

pub use crate::Council;
pub use advisors_llm::{default_seven_advisors, seven_system_prompts, LlmAdvisor};

/// Default overall deadline for the compatibility LLM Council path.
pub const DEFAULT_COUNCIL_TIMEOUT: Duration = Duration::from_secs(60);
