//! Bench - 性能测试 harness (从 v1.0 apeireth-bench 3.8K LOC 收敛)
//!
//! 0 装 PASS: 简化 benchmark (avg / min / max 计时), 完整 v1.0 era (criterion, flamegraph) 不做.

use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct BenchResult {
    pub name: &'static str,
    pub iterations: u32,
    pub total_ms: u64,
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
}

impl BenchResult {
    pub fn print(&self) -> String {
        format!("[{}] iter={} avg={:.3}ms min={}ms max={}ms total={}ms", self.name, self.iterations, self.avg_ms, self.min_ms, self.max_ms, self.total_ms)
    }
}

pub fn bench<F: FnMut()>(name: &'static str, iters: u32, mut f: F) -> BenchResult {
    let start = Instant::now();
    let mut min = u64::MAX;
    let mut max = 0u64;
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        let dt = t0.elapsed().as_millis() as u64;
        if dt < min { min = dt; }
        if dt > max { max = dt; }
    }
    let total = start.elapsed().as_millis() as u64;
    BenchResult { name, iterations: iters, total_ms: total, avg_ms: total as f64 / iters as f64, min_ms: min, max_ms: max }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bench_basic() {
        let r = bench("noop", 5, || { let _ = 1+1; });
        assert_eq!(r.iterations, 5);
        assert!(r.avg_ms >= 0.0);
    }
}
