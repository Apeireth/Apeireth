//! Canonical storage foundation (M1A).
//!
//! This crate owns the low-level, architecture-neutral persistence
//! primitives: SQLite connection pool management, SQLite configuration,
//! schema migrations, and storage-level errors.
//!
//! Salvage wave (agent 17) additionally recovers library primitives that
//! belong next to persistence, not as a second memory / credential owner:
//! in-process rate-limiter algorithms, LRU+TTL cache, snapshot quota, and
//! a portable machine-id probe.
//!
//! It deliberately does **not** own memory, vector, graph, session, runtime,
//! gateway, companion, provider, or governance logic. Those layers will be
//! built on top of this foundation in later migration phases.
//!
//! M1A port source: `origin/master:reconstruction_v2/crates/apeireth-storage`.
//! Only `pool.rs` and `migrations.rs` semantics were ported, with the canonical
//! migration SQL kept for on-disk compatibility.

pub mod cache;
pub mod error;
pub mod machine_id;
pub mod migrations;
pub mod pool;
pub mod quota;
pub mod rate_limit;

pub use error::StorageError;
pub use migrations::{current_version, run_migrations, Migration, LATEST_SCHEMA_VERSION};
pub use pool::{JournalMode, SqliteConfig, SqliteConnectionPool, SynchronousMode};
