//! Memory — STM / MTM / LTM 三层记忆
//!
//! 主人 12:14 "中央 AI 是永恒身份" → LTM 必须永不丢
//! 主人 13:47 "记忆是我关心的" → 借鉴 MemoryOS-Rust 3-tier 范式
//!
//! 借鉴:
//! - MemoryOS-Rust (TelivANT): STM/MTM/LTM 三层 (Apache-2.0)
//! - DeltaMemory: salience decay + 跨层 transition
//! - claude-mem: progressive disclosure

use serde::{Deserialize, Serialize};
use crate::episode::Episode;
use crate::note::Note;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Tier {
    /// 短期记忆 — 最近对话,频繁更新
    STM,
    /// 中期记忆 — 主题聚合,定期总结
    MTM,
    /// 长期记忆 — 持久事实,永不丢 (主人 12:14 永恒身份)
    LTM,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::STM => "stm",
            Tier::MTM => "mtm",
            Tier::LTM => "ltm",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stm" => Tier::STM,
            "mtm" => Tier::MTM,
            "ltm" => Tier::LTM,
            _ => Tier::STM,
        }
    }
}

/// Tier transition 事件 (借鉴 DeltaMemory "memories fade")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierTransition {
    pub from: Tier,
    pub to: Tier,
    pub reason: String,
    pub ts: chrono::DateTime<chrono::Utc>,
}

/// Memory = Episode + Note 的容器 (按 tier 分类)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub stm_episodes: Vec<Episode>,
    pub mtm_episodes: Vec<Episode>,
    pub ltm_episodes: Vec<Episode>,
    pub notes: Vec<Note>,
    pub transitions: Vec<TierTransition>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            stm_episodes: Vec::new(),
            mtm_episodes: Vec::new(),
            ltm_episodes: Vec::new(),
            notes: Vec::new(),
            transitions: Vec::new(),
        }
    }

    pub fn append_episode(&mut self, ep: Episode) {
        let tier = Tier::from_str(&ep.tier);
        match tier {
            Tier::STM => self.stm_episodes.push(ep),
            Tier::MTM => self.mtm_episodes.push(ep),
            Tier::LTM => self.ltm_episodes.push(ep),
        }
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    /// Tier transition (借鉴 DeltaMemory "frequently accessed → permanent")
    pub fn transition(&mut self, from: Tier, to: Tier, reason: impl Into<String>) {
        self.transitions.push(TierTransition {
            from,
            to,
            reason: reason.into(),
            ts: chrono::Utc::now(),
        });
    }

    pub fn episode_count(&self) -> (usize, usize, usize) {
        (self.stm_episodes.len(), self.mtm_episodes.len(), self.ltm_episodes.len())
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{Actor, EpisodeKind};

    #[test]
    fn test_memory_tier_separation() {
        let mut m = Memory::new();
        let ep_stm = Episode::new(Actor::Master, "recent", "ctx", EpisodeKind::Utterance, "h", "stm");
        let ep_ltm = Episode::new(Actor::Master, "eternal", "ctx", EpisodeKind::Utterance, "h", "ltm");
        m.append_episode(ep_stm);
        m.append_episode(ep_ltm);
        let (stm, _, ltm) = m.episode_count();
        assert_eq!(stm, 1);
        assert_eq!(ltm, 1);
    }
}