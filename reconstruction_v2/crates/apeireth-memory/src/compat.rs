//! v1-style compat types for apeireth-memory (v2 core has different shapes).
//!
//! v1 Episode:    { id: String, timestamp: i64, role: String, content: String, session_id: String }
//! v2 Episode:    { id: Uuid, session_id: Uuid, kind: EpisodeKind, content, importance, timestamp: DateTime<Utc> }
//! -> incompatible -> we define local v1-style structs in this module.
//!
//! v1 Note:       { id: String, timestamp: i64, content, source_episode_ids, confidence, tags }
//! v2 Note:       { id: Uuid, title, content, tags, created_at } -> incompatible
//!
//! v1 Session:    { id: String, started_at: i64, last_active_at: i64 }
//! v2 Session:    { id: Uuid, title, episodes, created_at } -> incompatible
//!
//! IdentityCard + Migration: v2 core IdentityCard already has v1 fields
//! (continuity_id/birth_time/carriers/migration_history via #[serde(default)]),
//! so we re-export those unchanged.

use serde::{Deserialize, Serialize};

/// v1-style Episode (String id, i64 timestamp, role/content/session_id).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Episode {
    pub id: String,
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub session_id: String,
}

/// v1-style Note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub timestamp: i64,
    pub content: String,
    pub source_episode_ids: Vec<String>,
    pub confidence: f64,
    pub tags: Vec<String>,
}

/// v1-style Session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub started_at: i64,
    pub last_active_at: i64,
}

// IdentityCard + Migration: re-export from v2 core (field-compatible).
pub use apeireth_core::{IdentityCard, Migration};
