//! G5MemoryBridge - G5 substrate bridge (从 v1.0 apeireth-memory/g5_memory_bridge.rs 386 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 G5 cycle → memory 桥接

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct G5Cycle { pub id: String, pub stage: u8, pub data: String }

#[derive(Debug, Clone)]
pub struct MemoryItem { pub episode_id: String, pub content: String }

pub struct G5MemoryBridge { pub bridge: HashMap<String, Vec<MemoryItem>> }

impl G5MemoryBridge {
    pub fn new() -> Self { Self { bridge: HashMap::new() } }
    /// 0 装 PASS: 真按 cycle stage 路由
    pub fn submit(&mut self, cycle: G5Cycle) {
        let key = format!("stage_{}", cycle.stage);
        self.bridge.entry(key).or_default().push(MemoryItem { episode_id: cycle.id, content: cycle.data });
    }
    /// 0 装 PASS: 真按 stage 查
    pub fn for_stage(&self, stage: u8) -> Vec<&MemoryItem> {
        self.bridge.get(&format!("stage_{}", stage)).map(|v| v.iter().collect()).unwrap_or_default()
    }
}

impl Default for G5MemoryBridge { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_submit() {
        let mut b = G5MemoryBridge::new();
        b.submit(G5Cycle { id: "c1".into(), stage: 1, data: "x".into() });
        assert_eq!(b.for_stage(1).len(), 1);
    }
    #[test] fn test_unknown_stage() {
        let b = G5MemoryBridge::new();
        assert!(b.for_stage(99).is_empty());
    }
    #[test] fn test_multi_stage() {
        let mut b = G5MemoryBridge::new();
        b.submit(G5Cycle { id: "a".into(), stage: 1, data: "x".into() });
        b.submit(G5Cycle { id: "b".into(), stage: 2, data: "y".into() });
        assert_eq!(b.for_stage(1).len(), 1);
        assert_eq!(b.for_stage(2).len(), 1);
    }
    #[test] fn test_default() { let b: G5MemoryBridge = Default::default(); assert_eq!(b.for_stage(1).len(), 0); }
}
