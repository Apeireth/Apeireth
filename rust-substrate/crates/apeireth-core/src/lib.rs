//! # Apeireth Core — Domain
//!
//! 主人 14:52 "最高深度, 最深刻优先, 不计成本"
//! 主人 12:14 "中央 AI 是永恒身份, 不是调度者/思考者"
//!
//! 借鉴:
//! - MemoryOS-Rust (TelivANT, Apache-2.0): STM/MTM/LTM 三层架构 + tier_manager
//! - DeltaMemory: WAL + MemTable + SSTable
//! - Zep / Graphiti: Episode provenance + temporal validity
//! - claude-mem: 3-layer progressive disclosure
//!
//! 模块结构:
//! - `episode` — 不可变 raw 事件 (append-only)
//! - `note` — 从 Episode 抽象的可被 Forget 的知识
//! - `memory` — STM/MTM/LTM 三层记忆 (主人 12:14 永恒身份 = LTM)
//! - `identity` — 中央 AI 多身份 (主人 12:14 "多身份, 像人是一切社会关系的总和")
//! - `relation_graph` — Episodic + Semantic 双图 (借鉴 AriGraph + Graphiti)
//! - `reconsolidate` — 4 paths: boost / flag / align / none (主人 13:47 关心)
//! - `forget` — Salience decay + threshold (借鉴 DeltaMemory exp decay)
//! - `wal` — Write-Ahead Log (借鉴 DeltaMemory CRC32)
//! - `episodes/episode`
//! - `notes/note`
//! - `memory/{stm, mtm, ltm}`
//! - `identity/identity_card`
//! - `relation_graph/{episodic, semantic}`

pub mod episode;
pub mod note;
pub mod memory;
pub mod identity;
pub mod relation_graph;
pub mod reconsolidate;
pub mod forget;
pub mod wal;
pub mod tier;

pub use episode::Episode;
pub use note::Note;
pub use identity::IdentityCard;
pub use memory::{Memory, Tier, TierTransition};

// Re-export tier_manager API
pub use tier::TierManager;
pub mod deliberation;
