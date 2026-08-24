//! Eval - 评估器 (从 v1.0 apeireth-eval 3,472 LOC 收敛)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSample {
    pub task_id: String,
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub latency_ms: u64,
    pub passed: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    pub total: u32, pub passed: u32, pub failed: u32,
    pub avg_latency_ms: f64, pub p95_latency_ms: u64, pub max_latency_ms: u64,
    pub pass_rate: f64,
}

impl EvalMetrics {
    pub fn from_samples(samples: &[EvalSample]) -> Self {
        let mut m = EvalMetrics::default();
        m.total = samples.len() as u32;
        m.passed = samples.iter().filter(|s| s.passed).count() as u32;
        m.failed = m.total - m.passed;
        if !samples.is_empty() {
            m.avg_latency_ms = samples.iter().map(|s| s.latency_ms as f64).sum::<f64>() / samples.len() as f64;
            let mut sorted: Vec<u64> = samples.iter().map(|s| s.latency_ms).collect();
            sorted.sort();
            m.p95_latency_ms = sorted.get((sorted.len() as f64 * 0.95) as usize).copied().unwrap_or(0);
            m.max_latency_ms = *sorted.last().unwrap_or(&0);
            m.pass_rate = m.passed as f64 / m.total as f64;
        }
        m
    }
}

#[derive(Default)]
pub struct EvalSuite { samples: Vec<EvalSample> }

impl EvalSuite {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, sample: EvalSample) { self.samples.push(sample); }
    pub fn aggregate(&self) -> EvalMetrics { EvalMetrics::from_samples(&self.samples) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mk(id: &str, expected: &str, actual: &str, latency: u64, passed: bool) -> EvalSample {
        EvalSample { task_id: id.into(), input: "?".into(), expected: expected.into(), actual: actual.into(), latency_ms: latency, passed }
    }
    #[test]
    fn test_aggregate_metrics() {
        let mut s = EvalSuite::new();
        s.add(mk("t1", "42", "42", 100, true));
        s.add(mk("t2", "42", "43", 200, false));
        s.add(mk("t3", "42", "42", 300, true));
        let m = s.aggregate();
        assert_eq!(m.total, 3);
        assert_eq!(m.passed, 2);
        assert!((m.pass_rate - 0.6667).abs() < 0.01);
        assert_eq!(m.max_latency_ms, 300);
    }
}
