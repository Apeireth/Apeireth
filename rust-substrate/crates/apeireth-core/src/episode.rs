//! Episode — append-only raw event (主人 12:14 永恒身份, 不可改)
//!
//! 借鉴:
//! - Zep / Graphiti: Episode provenance (https://github.com/getzep/graphiti)
//! - claude-mem: raw observations (https://github.com/thedotmack/claude-mem)
//! - MemoryOS-Rust: episodes 不可变

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Actor {
    Master,
    Apeireth,
    Tool,
    System,
}

impl Actor {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "master" => Actor::Master,
            "apeireth" => Actor::Apeireth,
            "tool" => Actor::Tool,
            "system" => Actor::System,
            _ => Actor::System,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EpisodeKind {
    Utterance,
    ToolCall,
    Observation,
    Kickoff,
    Reflection,
    Consolidation,
}

impl EpisodeKind {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "utterance" => EpisodeKind::Utterance,
            "tool_call" => EpisodeKind::ToolCall,
            "observation" => EpisodeKind::Observation,
            "kickoff" => EpisodeKind::Kickoff,
            "reflection" => EpisodeKind::Reflection,
            "consolidation" => EpisodeKind::Consolidation,
            _ => EpisodeKind::Utterance,
        }
    }
}

/// Episode = 一次互动事件的不可变记录
///
/// 主人 12:14 "中央 AI 是永恒身份" → Episode 必须可追溯
/// 主人 12:27 "LLM 没历史就从主人学" → Episode 永远保留 (除非主动 Forget)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// 唯一 ID (UUID v4, 16 chars)
    pub eid: String,
    /// 谁发出 (master / apeireth / tool / system)
    pub actor: Actor,
    /// 原始文本
    pub content: String,
    /// 当时上下文 (filename / URL / situation)
    pub context: String,
    /// 时间戳
    pub ts: DateTime<Utc>,
    /// 类型
    pub kind: EpisodeKind,
    /// 触发时的 IdentityCard hash (主人 12:14 "每次互动属哪个身份")
    pub linked_identity_hash: String,
    /// Tier 标签 (stm / mtm / ltm) — 借鉴 MemoryOS-Rust 3-tier
    pub tier: String,
    /// De-dup fingerprint (SHA256 of content + actor + ts, 16 chars)
    pub fingerprint: String,
}

impl Episode {
    /// 新建 Episode (自动算 fingerprint)
    pub fn new(
        actor: impl Into<Actor>,
        content: impl Into<String>,
        context: impl Into<String>,
        kind: impl Into<EpisodeKind>,
        linked_identity_hash: impl Into<String>,
        tier: impl Into<String>,
    ) -> Self {
        let actor = actor.into();
        let content = content.into();
        let context = context.into();
        let kind = kind.into();
        let linked_identity_hash = linked_identity_hash.into();
        let tier = tier.into();
        let ts = Utc::now();

        let fingerprint = Self::compute_fingerprint(&actor, &content, &context);

        Self {
            eid: Uuid::new_v4().simple().to_string()[..16].to_string(),
            actor,
            content,
            context,
            ts,
            kind,
            linked_identity_hash,
            tier,
            fingerprint,
        }
    }

    /// SHA256 fingerprint for de-dup
    fn compute_fingerprint(actor: &Actor, content: &str, context: &str) -> String {
        let mut h = Sha256::new();
        h.update(format!("{:?}", actor).as_bytes());
        h.update(content.as_bytes());
        h.update(b"|");
        h.update(context.as_bytes());
        let result = h.finalize();
        let hex = format!("{:x}", result);
        hex[..16].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_immutable_fingerprint() {
        let e1 = Episode::new(
            Actor::Master,
            "中央 AI 必须有 Memory",
            "test",
            EpisodeKind::Utterance,
            "abc123",
            "stm",
        );
        let e2 = e1.clone();
        assert_eq!(e1.fingerprint, e2.fingerprint);
        assert_eq!(e1.eid.len(), 16);
        assert!(!e1.content.is_empty());
    }

    #[test]
    fn test_episode_dedup_different_content() {
        let e1 = Episode::new(Actor::Master, "content A", "ctx", EpisodeKind::Utterance, "x", "stm");
        let e2 = Episode::new(Actor::Master, "content B", "ctx", EpisodeKind::Utterance, "x", "stm");
        assert_ne!(e1.fingerprint, e2.fingerprint);
    }
}