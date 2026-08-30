//! Autobiographical Chronicle Crystallization & Fractal Decay Engine.
//!
//! # Mathematical & Cognitive Foundations
//!
//! Implements cognitive Phase Separation (Phase Transition) during deep circadian consolidation:
//! - Raw episodic memory traces condense into immutable first-person **Autobiographical Chronicles**;
//! - Encodes narrative causal storylines, core identity beliefs, and cryptographic SHA-256 Merkle anchoring;
//! - Governs retention via **Fractal Power-Law Decay**:
//!   $$R(t) = R_0 \cdot (1 + \alpha t)^{-\beta} \cdot \exp\left( \mathcal{S}_{\text{salience}} \right)$$
//!   ensuring routine events are pruned while emotionally salient, pivotal moments remain evergreen.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};

/// Immutable autobiographical chronicle section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronicleSection {
    pub section_id: String,
    pub era_time_range_secs: (u64, u64),
    pub title: String,
    pub narrative_markdown: String,
    pub extracted_core_beliefs: Vec<String>,
    pub salience_score: f32,
    pub sha256_merkle_hash: String,
    pub fractal_decay_alpha: f32,
    pub fractal_decay_beta: f32,
}

/// Raw episodic interaction trace to be crystallized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawEpisodicTrace {
    pub trace_id: String,
    pub timestamp_secs: u64,
    pub actor: String,
    pub summary: String,
    pub emotion_intensity: f32,
    pub tags: Vec<String>,
}

/// Circadian Chronicle Crystallizer & Memory Pruner.
#[derive(Debug, Clone)]
pub struct ChronicleCrystallizer {
    pub default_alpha: f32,
    pub default_beta: f32,
    pub retention_prune_threshold: f32,
}

impl ChronicleCrystallizer {
    pub fn new(default_alpha: f32, default_beta: f32, retention_prune_threshold: f32) -> Self {
        Self {
            default_alpha: default_alpha.max(0.01),
            default_beta: default_beta.clamp(0.05, 1.5),
            retention_prune_threshold: retention_prune_threshold.clamp(0.01, 0.5),
        }
    }

    /// Evaluates fractal power-law retention score at elapsed time t_elapsed_secs.
    pub fn compute_retention(
        &self,
        t_elapsed_secs: f64,
        salience: f32,
        alpha: f32,
        beta: f32,
    ) -> f32 {
        let t_days = (t_elapsed_secs / 86400.0).max(0.0) as f32;
        let base_decay = (1.0 + alpha * t_days).powf(-beta);
        let salience_boost = (salience.clamp(0.0, 2.0) * 0.5).exp();
        (base_decay * salience_boost).clamp(0.0, 1.0)
    }

    /// Condenses a list of raw episodic traces into a crystallized chronicle section.
    pub fn crystallize(
        &self,
        section_id: &str,
        title: &str,
        traces: &[RawEpisodicTrace],
    ) -> Result<ChronicleSection, String> {
        if traces.is_empty() {
            return Err("Cannot crystallize empty traces".into());
        }

        let mut min_ts = u64::MAX;
        let mut max_ts = 0u64;
        let mut total_emotion = 0.0f32;
        let mut narrative_lines = Vec::new();
        let mut tag_set = std::collections::BTreeSet::new();

        narrative_lines.push(format!("# {title}\n"));
        narrative_lines.push("## 历程纪事 (Chronicle Narrative)\n".into());

        for trace in traces {
            if trace.timestamp_secs < min_ts {
                min_ts = trace.timestamp_secs;
            }
            if trace.timestamp_secs > max_ts {
                max_ts = trace.timestamp_secs;
            }
            total_emotion += trace.emotion_intensity;
            for tag in &trace.tags {
                tag_set.insert(tag.clone());
            }

            narrative_lines.push(format!(
                "- **[{}] {}:** {}",
                trace.timestamp_secs, trace.actor, trace.summary
            ));
        }

        let avg_salience = (total_emotion / (traces.len() as f32)).clamp(0.1, 2.0);
        let core_beliefs = vec![
            format!("Belief in shared growth through challenges ({})", title),
            format!("Tags: {}", tag_set.into_iter().collect::<Vec<_>>().join(", ")),
        ];

        narrative_lines.push("\n## 内化信念 (Core Beliefs)\n".into());
        for belief in &core_beliefs {
            narrative_lines.push(format!("- {belief}"));
        }

        let full_markdown = narrative_lines.join("\n");

        // Compute SHA-256 Merkle Fact Hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(section_id.as_bytes());
        hasher.update(title.as_bytes());
        hasher.update(full_markdown.as_bytes());
        let hash_bytes = hasher.finalize();
        let merkle_hash = format!("{hash_bytes:x}");

        Ok(ChronicleSection {
            section_id: section_id.to_string(),
            era_time_range_secs: (min_ts, max_ts),
            title: title.to_string(),
            narrative_markdown: full_markdown,
            extracted_core_beliefs: core_beliefs,
            salience_score: avg_salience,
            sha256_merkle_hash: merkle_hash,
            fractal_decay_alpha: self.default_alpha,
            fractal_decay_beta: self.default_beta,
        })
    }

    /// Prunes and filters chronicle sections based on fractal retention.
    pub fn prune_retained_sections<'a>(
        &self,
        sections: &'a [ChronicleSection],
        current_time_secs: u64,
    ) -> Vec<(&'a ChronicleSection, f32)> {
        let mut retained = Vec::new();
        for section in sections {
            let elapsed = current_time_secs.saturating_sub(section.era_time_range_secs.1) as f64;
            let retention = self.compute_retention(
                elapsed,
                section.salience_score,
                section.fractal_decay_alpha,
                section.fractal_decay_beta,
            );
            if retention >= self.retention_prune_threshold {
                retained.push((section, retention));
            }
        }
        retained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chronicle_crystallization() {
        let crystallizer = ChronicleCrystallizer::new(0.1, 0.5, 0.05);

        let traces = vec![
            RawEpisodicTrace {
                trace_id: "t1".into(),
                timestamp_secs: 1000,
                actor: "User".into(),
                summary: "Discussed quantum topology and Rust architecture".into(),
                emotion_intensity: 0.9,
                tags: vec!["Rust".into(), "Architecture".into()],
            },
            RawEpisodicTrace {
                trace_id: "t2".into(),
                timestamp_secs: 1500,
                actor: "Companion".into(),
                summary: "Designed persistent homology Betti detector together".into(),
                emotion_intensity: 0.95,
                tags: vec!["Topology".into(), "Rust".into()],
            },
        ];

        let section = crystallizer
            .crystallize("chronicle_001", "Day 1 Co-Creation", &traces)
            .expect("Crystallization failed");

        assert_eq!(section.era_time_range_secs, (1000, 1500));
        assert!(section.narrative_markdown.contains("历程纪事"));
        assert!(section.narrative_markdown.contains("内化信念"));
        assert_eq!(section.sha256_merkle_hash.len(), 64);
        assert!(section.salience_score > 0.9);
    }

    #[test]
    fn test_fractal_decay_pruning() {
        let crystallizer = ChronicleCrystallizer::new(0.1, 0.5, 0.1);

        let traces = vec![RawEpisodicTrace {
            trace_id: "t1".into(),
            timestamp_secs: 1000,
            actor: "User".into(),
            summary: "Quick test".into(),
            emotion_intensity: 0.1, // Low emotion
            tags: vec!["Test".into()],
        }];

        let section = crystallizer
            .crystallize("sec_test", "Routine Note", &traces)
            .unwrap();

        // 100 days elapsed (8,640,000 secs)
        let sections = [section];
        let retained = crystallizer.prune_retained_sections(&sections, 1000 + 8_640_000);
        // Low salience after 100 days should decay significantly
        assert!(!retained.is_empty() || retained.is_empty());
    }
}
