//! apeireth-vector: 向量检索子系统 (V2 P1 战区 4)
//!
//! 目标:让 R14 记忆系统具备语义检索能力,而不破坏现有 apeireth-memory (BM25) 链路.
//!
//! 设计要点:
//! 1. **trait 抽象**: `VectorStore` 是唯一契约,业务侧只依赖 trait,不绑死后端.
//! 2. **backend 可替换**: 现在默认 `SqliteVecBackend` (R19 P2: 真接 `sqlite-vec` C 扩展 vec0 虚拟表);
//!    后续可平滑换为 `lancedb-rs` / `pgvector`,调用方零改动.
//! 3. **与 apeireth-memory 共存**: 各自打开自己的 .db 文件,SQLite WAL 不互锁;
//!    类型层通过 `Uuid`/`String` 标签对齐,不共享 trait.
//!
//! 禁止:
//! - ❌ 引入 ORM
//! - ❌ 触碰 `apeireth-memory` 任何现有表 / migration
//!
//! 关于 unsafe_code:
//! - crate 级仍 `deny(unsafe_code)`, 保持 safe-first 哲学
//! - 例外: FFI 调 `sqlite-vec` 的 `sqlite3_vec_init` (R19 P2 必需) 由 fn-level
//!   `#[allow(unsafe_code)]` 显式 opt-in, 范围收窄到 1 个 fn, 0 业务代码影响
//!
//! ponytail ceiling:
//! - vec0 虚拟表在 10w 条 × 768 维 KNN P99 < 50ms (sqlite-vec 官方 benchmark)
//! - 真上 100w+ 时换 `lancedb-rs`, trait 不动, 仅替换 backend

#![deny(unsafe_code)]

mod error;
mod sqlite_backend;
mod traits;
// R150 P1 #6: Qdrant HTTP 协议兼容层 (借鉴 qdrant/qdrant REST API v1.7+)
pub mod qdrant_compat;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod distance;  // R206: vector distance utilities (std + auto-vectorization, 0 新依赖)

pub use error::VectorError;
pub use sqlite_backend::SqliteVecBackend;
pub use traits::{ScoredId, SearchHit, Vector, VectorStore};
pub use qdrant_compat::{QdrantClient, QdrantDistance, QdrantError, ScoredPoint};
