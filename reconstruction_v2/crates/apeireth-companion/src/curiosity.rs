use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriositySignal {
    pub topic: String,
    pub curiosity_score: f64,
    pub reason: String,
    pub suggested_inquiry: String,
}

pub struct CuriosityEngine {
    pub base_bias: f64,
    pub exploration_threshold: f64,
}

impl Default for CuriosityEngine {
    fn default() -> Self {
        Self {
            base_bias: 0.1,
            exploration_threshold: 0.65,
        }
    }
}

impl CuriosityEngine {
    pub fn new(base_bias: f64, threshold: f64) -> Self {
        Self {
            base_bias,
            exploration_threshold: threshold,
        }
    }

    /// Evaluates curiosity score based on relevance, prediction error (Brier surprise), and topic novelty
    pub fn evaluate(&self, relevance: f64, brier_surprise: f64, novelty: f64) -> f64 {
        let score = self.base_bias + (relevance * 0.4) + (brier_surprise * 0.4) + (novelty * 0.2);
        score.clamp(0.0, 1.0)
    }

    /// Decides whether a topic warrants proactive follow-up inquiry
    pub fn should_proactively_explore(&self, relevance: f64, brier_surprise: f64, novelty: f64) -> bool {
        self.evaluate(relevance, brier_surprise, novelty) >= self.exploration_threshold
    }

    /// Generates a structured curiosity signal when unfamiliar or high-surprise knowledge is encountered
    pub fn generate_signal(&self, topic: &str, relevance: f64, brier_surprise: f64, novelty: f64) -> Option<CuriositySignal> {
        let score = self.evaluate(relevance, brier_surprise, novelty);
        if score >= self.exploration_threshold {
            Some(CuriositySignal {
                topic: topic.to_string(),
                curiosity_score: score,
                reason: format!("High curiosity score ({:.2}): surprise={:.2}, novelty={:.2}", score, brier_surprise, novelty),
                suggested_inquiry: format!("关于'{}'，是否可以深入探讨其背后的机制或具体应用？", topic),
            })
        } else {
            None
        }
    }
}
