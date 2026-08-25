//! Canonical builtin tool capabilities.
//!
//! This crate provides the M2A low-risk builtin tools as canonical
//! [`ToolCapability`] implementations wrapped in a single plugin.
//!
//! # Ownership
//!
//! The tools own their identity, input schema, execution implementation, and
//! structured result. They do **not** own the runtime, the gateway, sessions,
//! providers, governance, or the dispatch loop. The runtime reaches them only
//! through the canonical plugin/capability registry path.

#![deny(unsafe_code)]

pub mod filesystem;
pub mod repo;
pub mod search;

pub use filesystem::{FilesystemError, FilesystemTool};
pub use repo::{RepoError, RepoTool};
pub use search::{SearchError, SearchTool};