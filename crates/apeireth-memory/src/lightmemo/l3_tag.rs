//! L3: Tag inverted index.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::{HashMap, HashSet};

pub struct TagIndex {
    /// tag → set of item ids
    forward: HashMap<String, HashSet<String>>,
    /// item id → set of tags
    reverse: HashMap<String, HashSet<String>>,
}

impl TagIndex {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: &str, tag: &str) {
        self.forward
            .entry(tag.to_string())
            .or_default()
            .insert(id.to_string());
        self.reverse
            .entry(id.to_string())
            .or_default()
            .insert(tag.to_string());
    }

    pub fn remove(&mut self, id: &str) {
        if let Some(tags) = self.reverse.remove(id) {
            for tag in tags {
                if let Some(set) = self.forward.get_mut(&tag) {
                    set.remove(id);
                    if set.is_empty() {
                        self.forward.remove(&tag);
                    }
                }
            }
        }
    }

    pub fn lookup(&self, tag: &str) -> Vec<String> {
        self.forward
            .get(tag)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn tags_of(&self, id: &str) -> Vec<String> {
        self.reverse
            .get(id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn tag_count(&self) -> usize {
        self.forward.len()
    }

    pub fn item_count(&self) -> usize {
        self.reverse.len()
    }
}

impl Default for TagIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_lookup() {
        let mut idx = TagIndex::new();
        idx.add("item1", "rust");
        idx.add("item2", "rust");
        let r = idx.lookup("rust");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn remove_clears_both() {
        let mut idx = TagIndex::new();
        idx.add("item1", "rust");
        idx.add("item1", "async");
        idx.remove("item1");
        assert!(idx.lookup("rust").is_empty());
        assert!(idx.lookup("async").is_empty());
        assert_eq!(idx.item_count(), 0);
    }

    #[test]
    fn tags_of() {
        let mut idx = TagIndex::new();
        idx.add("x", "a");
        idx.add("x", "b");
        let tags = idx.tags_of("x");
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn empty_index() {
        let idx = TagIndex::new();
        assert_eq!(idx.tag_count(), 0);
        assert_eq!(idx.item_count(), 0);
    }

    #[test]
    fn lookup_missing() {
        let idx = TagIndex::new();
        assert!(idx.lookup("nonexistent").is_empty());
    }
}
