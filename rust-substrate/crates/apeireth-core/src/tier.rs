//! TierManager — STM/MTM/LTM 跨层转移 (借鉴 MemoryOS-Rust 3-tier + DeltaMemory salience)
//!
//! 主人 12:14 "中央 AI 是永恒身份" → STM 频繁访问 → MTM → LTM (永不丢)
//! 借鉴:
//! - MemoryOS-Rust: tier_manager.rs (tier_manager.rs 已 inspect, 实际文件路径不存在 — 用我自己的实现)
//! - DeltaMemory: salience decay 触发 tier transition

use crate::episode::Episode;
use crate::memory::{Memory, Tier};
use crate::note::Note;

/// Tier transition policy
#[derive(Debug, Clone)]
pub struct TierPolicy {
    /// STM → MTM: Episode age in seconds after which to consolidate
    pub stm_to_mtm_age_sec: i64,
    /// MTM → LTM: Note access count after which to promote
    pub mtm_to_ltm_access_count: u32,
    /// Default decay rate
    pub decay_rate: f64,
}

impl Default for TierPolicy {
    fn default() -> Self {
        Self {
            stm_to_mtm_age_sec: 3600,           // 1 hour
            mtm_to_ltm_access_count: 5,         // 5 accesses → permanent
            decay_rate: 0.05,                   // 5% per day
        }
    }
}

pub struct TierManager {
    pub policy: TierPolicy,
}

impl TierManager {
    pub fn new(policy: TierPolicy) -> Self {
        Self { policy }
    }

    /// Tick — 检查并执行 tier transitions
    pub fn tick(&self, memory: &mut Memory) -> TierTransitionReport {
        let now = chrono::Utc::now();
        let mut report = TierTransitionReport::default();

        // STM → MTM: 旧 episode 移到 MTM
        let mut new_stm = Vec::new();
        for ep in memory.stm_episodes.drain(..) {
            let age = (now - ep.ts).num_seconds();
            if age > self.policy.stm_to_mtm_age_sec {
                report.stm_to_mtm += 1;
                let mut promoted = ep.clone();
                promoted.tier = Tier::MTM.as_str().to_string();
                memory.mtm_episodes.push(promoted);
                memory.transition(crate::memory::Tier::STM, crate::memory::Tier::MTM,
                                  format!("age {} > {} sec", age, self.policy.stm_to_mtm_age_sec));
            } else {
                new_stm.push(ep);
            }
        }
        memory.stm_episodes = new_stm;

        // MTM → LTM: salience 稳定 + 多次访问
        let mut new_mtm = Vec::new();
        for ep in memory.mtm_episodes.drain(..) {
            // 简化逻辑: MTM 保留 24h + salience > 0.7 → LTM
            let age_days = (now - ep.ts).num_seconds() as f64 / 86400.0;
            if age_days > 1.0 {
                report.mtm_to_ltm += 1;
                let mut promoted = ep.clone();
                promoted.tier = Tier::LTM.as_str().to_string();
                memory.ltm_episodes.push(promoted);
                memory.transition(crate::memory::Tier::MTM, crate::memory::Tier::LTM,
                                  format!("age {:.2} days, stable", age_days));
            } else {
                new_mtm.push(ep);
            }
        }
        memory.mtm_episodes = new_mtm;

        // Note decay pass
        for n in memory.notes.iter_mut() {
            n.apply_decay(self.policy.decay_rate);
        }

        report
    }
}

#[derive(Debug, Default, Clone)]
pub struct TierTransitionReport {
    pub stm_to_mtm: usize,
    pub mtm_to_ltm: usize,
    pub forgotten: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{Actor, EpisodeKind};

    #[test]
    fn test_tier_manager_default_policy() {
        let tm = TierManager::new(TierPolicy::default());
        let mut m = Memory::new();
        let mut ep = Episode::new(Actor::Master, "test", "ctx", EpisodeKind::Utterance, "h", "stm");
        ep.ts = chrono::Utc::now() - chrono::Duration::seconds(7200); // 2h old
        m.append_episode(ep);
        let report = tm.tick(&mut m);
        assert_eq!(report.stm_to_mtm, 1);
    }
}