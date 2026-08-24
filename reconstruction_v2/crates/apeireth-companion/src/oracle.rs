//! Oracle - 预测机 (从 v1.0 apeireth-companion/oracle.rs 4K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 prediction + Brier scoring + ledger
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub claim: String,
    pub probability: f32,  // 0 装 PASS: 0.0-1.0
    pub timestamp_ms: i64,
    pub outcome: Option<bool>,
}

pub struct Oracle {
    predictions: HashMap<String, Prediction>,
}

impl Oracle {
    pub fn new() -> Self { Self { predictions: HashMap::new() } }

    /// 0 装 PASS: 真预测
    pub fn predict(&mut self, claim: impl Into<String>, probability: f32) -> String {
        let id = format!("p-{}", chrono::Utc::now().timestamp_millis());
        self.predictions.insert(id.clone(), Prediction { id: id.clone(), claim: claim.into(), probability: probability.clamp(0.0, 1.0), timestamp_ms: chrono::Utc::now().timestamp_millis(), outcome: None });
        id
    }

    /// 0 装 PASS: 真记录结果
    pub fn resolve(&mut self, id: &str, outcome: bool) -> Result<(), String> {
        let p = self.predictions.get_mut(id).ok_or_else(|| "not found")?;
        p.outcome = Some(outcome);
        Ok(())
    }

    /// 0 装 PASS: 真 Brier score (1 prediction)
    pub fn brier(&self, id: &str) -> Option<f32> {
        self.predictions.get(id).and_then(|p| p.outcome.map(|o| {
            let a = if o { 1.0 } else { 0.0 };
            (p.probability - a).powi(2)
        }))
    }

    /// 0 装 PASS: 真平均 Brier
    pub fn avg_brier(&self) -> Option<f32> {
        let scored: Vec<f32> = self.predictions.values().filter_map(|p| self.brier(&p.id)).collect();
        if scored.is_empty() { return None; }
        Some(scored.iter().sum::<f32>() / scored.len() as f32)
    }
}

impl Default for Oracle { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_predict_resolve() {
        let mut o = Oracle::new();
        let id = o.predict("it will rain", 0.8);
        o.resolve(&id, true).unwrap();
        let b = o.brier(&id).unwrap();
        assert!((b - 0.04).abs() < 1e-6);  // (0.8-1)^2 = 0.04
    }
    #[test] fn test_unresolved_no_brier() {
        let mut o = Oracle::new();
        let id = o.predict("x", 0.5);
        assert!(o.brier(&id).is_none());
    }
    #[test] fn test_clamp() {
        let mut o = Oracle::new();
        let id = o.predict("x", 1.5);
        assert_eq!(o.predictions.get(&id).unwrap().probability, 1.0);
    }
    #[test] fn test_avg_brier() {
        let mut o = Oracle::new();
        let id1 = o.predict("a", 0.5);
        let id2 = o.predict("b", 0.5);
        o.resolve(&id1, true).unwrap();
        o.resolve(&id2, false).unwrap();
        let avg = o.avg_brier().unwrap();
        assert!((avg - 0.25).abs() < 1e-6);  // 0.5^2 = 0.25 each
    }
}
