//! Note — 从 Episode 抽象的稳定知识 (可被 Forget / Reconsolidate)
//!
//! 主人 13:47 "记忆模块" — Note 是记忆的核心抽象
//! 主人 12:14 "中央 AI 是永恒身份" — Note 跨 session 持久化
//!
//! 借鉴:
//! - PersistBench (arxiv 2602.01146): 防 97% sycophancy 失败 → Note 必须可被 Forget
//! - DeltaMemory: salience decay
//! - claude-mem: semantic summaries

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Note = 抽象的稳定知识 (主题 + 主张 + 证据链)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// 唯一 ID (16 chars)
    pub nid: String,
    /// 主题
    pub topic: String,
    /// 主张 (可证伪的命题)
    pub claim: String,
    /// 证据 (Episode eid 列表)
    pub evidence: Vec<String>,
    /// Bayesian 置信度 0-1
    pub confidence: f64,
    /// 重要度 0-10 (影响 Forget threshold)
    pub importance: u8,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最近一次 Reconsolidate 时间
    pub last_consolidated: DateTime<Utc>,
    /// 该 Note 取代的旧 Note nid 列表 (主人 12:14 永恒身份 → 替代链)
    pub supersedes: Vec<String>,
    /// Tier (stm / mtm / ltm)
    pub tier: String,
    /// Salience (DeltaMemory exp decay)
    pub salience: f64,
}

impl Note {
    pub fn new(
        topic: impl Into<String>,
        claim: impl Into<String>,
        evidence: Vec<String>,
        confidence: f64,
        importance: u8,
        tier: impl Into<String>,
    ) -> Self {
        let topic = topic.into();
        let claim = claim.into();
        let tier = tier.into();
        let now = Utc::now();

        Self {
            nid: Uuid::new_v4().simple().to_string()[..16].to_string(),
            topic,
            claim,
            evidence,
            confidence: confidence.clamp(0.0, 1.0),
            importance: importance.min(10),
            created_at: now,
            last_consolidated: now,
            supersedes: Vec::new(),
            tier,
            salience: 1.0,
        }
    }

    /// Salience decay (DeltaMemory formula)
    /// current_salience = stored_salience * e^(-decay_rate * age_days)
    pub fn apply_decay(&mut self, decay_rate: f64) {
        let now = Utc::now();
        let age_seconds = (now - self.last_consolidated).num_seconds() as f64;
        let age_days = age_seconds / 86400.0;
        self.salience = self.salience * (-decay_rate * age_days).exp();
    }

    /// Bayesian update — 新的证据支持时
    pub fn boost_confidence(&mut self, delta: f64) {
        self.confidence = (self.confidence + delta).clamp(0.0, 1.0);
        self.salience = (self.salience * 1.2).min(1.0); // 频繁访问刷新
        self.last_consolidated = Utc::now();
    }

    /// Bayesian update — 新的反证出现时
    pub fn reduce_confidence(&mut self, delta: f64) {
        self.confidence = (self.confidence - delta).max(0.0);
    }

    /// Forget threshold check
    pub fn should_forget(&self, threshold: f64) -> bool {
        (self.confidence * (self.importance as f64) / 10.0) < threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_decay() {
        let mut n = Note::new("test", "claim", vec![], 0.5, 5, "stm");
        n.salience = 1.0;
        n.apply_decay(0.1);
        assert!(n.salience <= 1.0);
    }

    #[test]
    fn test_note_forget_threshold() {
        let n = Note::new("test", "claim", vec![], 0.1, 1, "stm");
        // 0.1 * 1/10 = 0.01 < 0.30 threshold
        assert!(n.should_forget(0.30));
    }

    #[test]
    fn test_note_boost() {
        let mut n = Note::new("test", "claim", vec![], 0.5, 5, "stm");
        n.boost_confidence(0.3);
        assert!(n.confidence > 0.5);
    }
}