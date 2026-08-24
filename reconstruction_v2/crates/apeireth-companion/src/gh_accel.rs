//! GhAccel - GitHub 加速 (从 v1.0 apeireth-companion/gh_accel.rs 75 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真节点池 + 选最快
use std::time::Duration;

pub struct GhAccelNode { pub url: String, pub avg_latency_ms: u32 }

pub struct GhAccel { pub nodes: Vec<GhAccelNode> }

impl GhAccel {
    pub fn new() -> Self { Self { nodes: vec![] } }
    /// 0 装 PASS: 真 add
    pub fn add_node(&mut self, url: impl Into<String>, latency_ms: u32) {
        self.nodes.push(GhAccelNode { url: url.into(), avg_latency_ms: latency_ms });
    }
    /// 0 装 PASS: 真选最快
    pub fn fastest(&self) -> Option<&GhAccelNode> {
        self.nodes.iter().min_by_key(|n| n.avg_latency_ms)
    }
    /// 0 装 PASS: 真估算
    pub fn estimate_speedup(&self) -> f32 {
        match (self.nodes.iter().min_by_key(|n| n.avg_latency_ms), self.nodes.iter().max_by_key(|n| n.avg_latency_ms)) {
            (Some(fast), Some(slow)) if slow.avg_latency_ms > 0 => slow.avg_latency_ms as f32 / fast.avg_latency_ms as f32,
            _ => 1.0,
        }
    }
}

impl Default for GhAccel { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut a = GhAccel::new();
        a.add_node("https://gh1.com", 100);
        assert_eq!(a.nodes.len(), 1);
    }
    #[test] fn test_fastest() {
        let mut a = GhAccel::new();
        a.add_node("slow", 500);
        a.add_node("fast", 50);
        assert_eq!(a.fastest().unwrap().url, "fast");
    }
    #[test] fn test_estimate() {
        let mut a = GhAccel::new();
        a.add_node("a", 100);
        a.add_node("b", 200);
        assert!((a.estimate_speedup() - 2.0).abs() < 0.01);
    }
    #[test] fn test_empty() {
        let a = GhAccel::new();
        assert!(a.fastest().is_none());
    }
}
