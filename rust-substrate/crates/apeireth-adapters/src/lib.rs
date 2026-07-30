//! # Apeireth Adapters — Concrete implementations
//!
//! 借鉴 MemoryOS-Rust hexagonal architecture:
//! - EpisodeRepository → SqliteEpisodeRepository
//! - NoteRepository → SqliteNoteRepository
//! - VectorIndex → QdrantVectorIndex (or Pinecone / LanceDB)
//! - FullTextIndex → TantivyIndex
//! - WalSink → FileWalSink
//! - LlmClient → OpenAICompatibleLlmClient
//! - AsyncDispatcher → TokioDispatcher (V30, tokio 真生产, 主 12:07)
//!
//! 主人 14:52 "最高深度" → adapter 都做,但 hot path 优先

pub mod sqlite_episode;
pub mod sqlite_note;
pub mod qdrant_vector;
pub mod tantivy_fulltext;
pub mod file_wal;
pub mod openai_llm;
pub mod tokio_dispatcher;

pub use sqlite_episode::SqliteEpisodeRepository;
pub use sqlite_note::SqliteNoteRepository;
pub use qdrant_vector::QdrantVectorIndex;
pub use tantivy_fulltext::TantivyIndex;
pub use file_wal::FileWalSink;
pub use openai_llm::OpenAICompatibleLlmClient;
pub use tokio_dispatcher::TokioDispatcher;