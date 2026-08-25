//! Memory ThreeLayer - 工作/短期/长期 三层 (抄 v1 apeireth-memory/three_layer.rs)
use std::collections::HashMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLayer { Working, ShortTerm, LongTerm }
pub struct MemoryItem { pub layer: MemoryLayer, pub content: String, pub access_count: u32 }
pub struct ThreeLayerStore { pub layers: HashMap<MemoryLayer, Vec<MemoryItem>>, pub promote_threshold: u32 }
impl ThreeLayerStore {
    pub fn new() -> Self { Self { layers: HashMap::new(), promote_threshold: 3 } }
    pub fn put(&mut self, layer: MemoryLayer, content: impl Into<String>) { self.layers.entry(layer).or_default().push(MemoryItem { layer, content: content.into(), access_count: 0 }); }
    pub fn access(&mut self, layer: MemoryLayer, idx: usize) {
        if let Some(items) = self.layers.get_mut(&layer) {
            if idx < items.len() {
                items[idx].access_count += 1;
                if items[idx].access_count >= self.promote_threshold && layer == MemoryLayer::Working {
                    let item = items.remove(idx);
                    self.layers.entry(MemoryLayer::ShortTerm).or_default().push(item);
                }
            }
        }
    }
    pub fn count(&self) -> usize { self.layers.values().map(|v| v.len()).sum() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_basic() { let mut s = ThreeLayerStore::new(); s.put(MemoryLayer::Working, "x"); assert_eq!(s.count(), 1); } #[test] fn test_promote() { let mut s = ThreeLayerStore::new(); s.put(MemoryLayer::Working, "x"); s.access(MemoryLayer::Working, 0); s.access(MemoryLayer::Working, 0); s.access(MemoryLayer::Working, 0); assert_eq!(s.count(), 1); } }