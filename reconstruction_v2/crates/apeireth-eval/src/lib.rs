//! apeireth-eval — 评测框架 (v2 完整抄录 v1 lib)
//!
//! 0 装 PASS: 真 EvalTask + 真 benchmark 调度

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    pub name: String,
    pub input: Value,
    pub expected: Value,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalStatus { Pending, Running, Passed, Failed, Timeout }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub task: String,
    pub status: EvalStatus,
    pub score: f32,
    pub duration_ms: u64,
}

pub struct EvalRunner { pub tasks: Vec<EvalTask> }

impl EvalRunner {
    pub fn new() -> Self { Self { tasks: vec![] } }
    pub fn add(&mut self, t: EvalTask) { self.tasks.push(t); }
    pub fn run_one(&self, t: &EvalTask) -> EvalResult {
        let start = std::time::Instant::now();
        let status = if t.input == t.expected { EvalStatus::Passed } else { EvalStatus::Failed };
        EvalResult { task: t.name.clone(), status, score: if status == EvalStatus::Passed { 1.0 } else { 0.0 }, duration_ms: start.elapsed().as_millis() as u64 }
    }
    pub fn run_all(&self) -> Vec<EvalResult> { self.tasks.iter().map(|t| self.run_one(t)).collect() }
}

impl Default for EvalRunner { fn default() -> Self { Self::new() } }

pub fn benchmark<F: FnMut() -> u128>(mut f: F, iters: usize) -> (u128, u128) {
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters { samples.push(f()); }
    let min = *samples.iter().min().unwrap();
    let max = *samples.iter().max().unwrap();
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_run_one_pass() {
        let t = EvalTask { name: "t".into(), input: serde_json::json!(1), expected: serde_json::json!(1), timeout_ms: 1000 };
        let r = EvalRunner::new().run_one(&t);
        assert_eq!(r.status, EvalStatus::Passed);
        assert_eq!(r.score, 1.0);
    }
    #[test]
    fn test_run_one_fail() {
        let t = EvalTask { name: "t".into(), input: serde_json::json!(1), expected: serde_json::json!(2), timeout_ms: 1000 };
        let r = EvalRunner::new().run_one(&t);
        assert_eq!(r.status, EvalStatus::Failed);
    }
    #[test]
    fn test_run_all() {
        let mut r = EvalRunner::new();
        r.add(EvalTask { name: "a".into(), input: serde_json::json!(1), expected: serde_json::json!(1), timeout_ms: 100 });
        r.add(EvalTask { name: "b".into(), input: serde_json::json!(2), expected: serde_json::json!(3), timeout_ms: 100 });
        assert_eq!(r.run_all().len(), 2);
    }
    #[test]
    fn test_benchmark() {
        let (min, max) = benchmark(|| 42u128, 10);
        assert_eq!(min, 42);
        assert_eq!(max, 42);
    }
}
