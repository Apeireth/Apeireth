//! Canonical memory subsystem (M1B1).
//!
//! This module is the canonical owner of durable memory on `reconstruct_v2`.
//! It deliberately does not know about runtime, gateway, provider, companion,
//! vector retrieval, graph traversal, or plugin registries.

pub mod domain;
pub mod error;
pub mod repository;
pub mod sqlite;

pub use domain::{MemoryId, MemoryItem};
pub use error::MemoryError;
pub use repository::{MemoryFilter, MemoryRepository};
pub use sqlite::SqliteMemoryRepository;
