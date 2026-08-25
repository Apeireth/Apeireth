//! Memory G5Bridge - G5 阶段路由 (抄 v1 apeireth-memory/g5_memory_bridge.rs)
use std::collections::HashMap;
pub struct G5MemoryBridge { pub stages: HashMap<u8, Vec<String>> }
impl G5MemoryBridge {
    pub fn new() -> Self { Self { stages: HashMap::new() } }
    pub fn submit(&mut self, stage: u8, item: impl Into<String>) {
        self.stages.entry(stage).or_default().push(item.into());
    }
    pub fn for_stage(&self, stage: u8) -> Vec<&String> { self.stages.get(&stage).map(|v| v.iter().collect()).unwrap_or_default() }
    pub fn count(&self) -> usize { self.stages.values().map(|v| v.len()).sum() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_submit() { let mut b = G5MemoryBridge::new(); b.submit(1, "item1"); assert_eq!(b.count(), 1); } #[test] fn test_unknown_stage() { let b = G5MemoryBridge::new(); assert!(b.for_stage(99).is_empty()); } }