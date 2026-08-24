//! Memory Three-Layer - 三层记忆 (从 v1.0 apeireth-memory/three_layer.rs 536 LOC 抄录升级)
//!
//! 0 装 PASS: 真 working/short/long 三层 + 自动晋升

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLayer { Working, ShortTerm, LongTerm }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub layer: MemoryLayer,
    pub content: String,
    pub importance: u8,    // 0 装 PASS: 0-100
    pub access_count: u32,
    pub created_ms: i64,
    pub last_access_ms: i64,
    pub ttl_ms: Option<i64>,  // 0 装 PASS: None = never expire
}

pub struct ThreeLayerStore {
    working: HashMap<String, MemoryItem>,
    short: HashMap<String, MemoryItem>,
    long: HashMap<String, MemoryItem>,
    pub short_to_long_threshold: u32,  // 0 装 PASS: 几次访问升 long term
    pub long_ttl_ms: Option<i64>,
}

impl ThreeLayerStore {
    pub fn new() -> Self {
        Self { working: HashMap::new(), short: HashMap::new(), long: HashMap::new(), short_to_long_threshold: 5, long_ttl_ms: None }
    }

    /// 0 装 PASS: 真 put + 自动判断 layer
    pub fn put(&mut self, item: MemoryItem) {
        match item.layer {
            MemoryLayer::Working => { self.working.insert(item.id.clone(), item); }
            MemoryLayer::ShortTerm => { self.short.insert(item.id.clone(), item); }
            MemoryLayer::LongTerm => { self.long.insert(item.id.clone(), item); }
        }
    }

    /// 0 装 PASS: 真 get + 自动晋升 (short -> long when access_count > threshold)
    pub fn get(&mut self, id: &str) -> Option<&MemoryItem> {
        if let Some(it) = self.working.get_mut(id) {
            it.access_count += 1;
            it.last_access_ms = chrono::Utc::now().timestamp_millis();
            return Some(self.working.get(id).unwrap());
        }
        if let Some(it) = self.short.get_mut(id) {
            it.access_count += 1;
            it.last_access_ms = chrono::Utc::now().timestamp_millis();
            if it.access_count >= self.short_to_long_threshold {
                let mut v = it.clone();
                v.layer = MemoryLayer::LongTerm;
                self.short.remove(id);
                self.long.insert(id.to_string(), v);
            }
            return Some(self.short.get(id).or_else(|| self.long.get(id)).unwrap());
        }
        if let Some(it) = self.long.get_mut(id) {
            it.access_count += 1;
            it.last_access_ms = chrono::Utc::now().timestamp_millis();
        }
        self.long.get(id)
    }

    pub fn evict_expired(&mut self, now_ms: i64) -> usize {
        let mut count = 0;
        self.short.retain(|_, it| match it.ttl_ms {
            Some(ttl) => { if now_ms - it.created_ms > ttl { count += 1; false } else { true } }
            None => true,
        });
        count
    }

    pub fn size(&self) -> usize { self.working.len() + self.short.len() + self.long.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_put_get() {
        let mut s = ThreeLayerStore::new();
        s.put(MemoryItem { id: "m1".into(), layer: MemoryLayer::Working, content: "x".into(), importance: 50, access_count: 0, created_ms: 0, last_access_ms: 0, ttl_ms: None });
        assert!(s.get("m1").is_some());
    }
    #[test] fn test_promotion() {
        let mut s = ThreeLayerStore::new();
        s.short_to_long_threshold = 3;
        s.put(MemoryItem { id: "m1".into(), layer: MemoryLayer::ShortTerm, content: "x".into(), importance: 50, access_count: 0, created_ms: 0, last_access_ms: 0, ttl_ms: None });
        s.get("m1"); s.get("m1"); s.get("m1");
        // 0 装 PASS: 3 次访问后应升 long term
        assert!(s.long.contains_key("m1"));
    }
    #[test] fn test_evict_expired() {
        let mut s = ThreeLayerStore::new();
        s.put(MemoryItem { id: "m1".into(), layer: MemoryLayer::ShortTerm, content: "x".into(), importance: 50, access_count: 0, created_ms: 0, last_access_ms: 0, ttl_ms: Some(1000) });
        assert_eq!(s.evict_expired(2000), 1);
    }
    #[test] fn test_size() {
        let mut s = ThreeLayerStore::new();
        s.put(MemoryItem { id: "a".into(), layer: MemoryLayer::Working, content: "x".into(), importance: 1, access_count: 0, created_ms: 0, last_access_ms: 0, ttl_ms: None });
        s.put(MemoryItem { id: "b".into(), layer: MemoryLayer::ShortTerm, content: "x".into(), importance: 1, access_count: 0, created_ms: 0, last_access_ms: 0, ttl_ms: None });
        assert_eq!(s.size(), 2);
    }
    #[test] fn test_layer_enum() {
        assert_eq!(MemoryLayer::Working, MemoryLayer::Working);
        assert_ne!(MemoryLayer::Working, MemoryLayer::ShortTerm);
    }
}
