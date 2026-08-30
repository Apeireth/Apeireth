//! Hand-written HashMap + VecDeque LRU. No extra crates.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Classic LRU: `HashMap` for lookup, `VecDeque` for recency (back = most recent).
pub struct HashMapVecDequeLru<K, V>
where
    K: Hash + Eq + Clone,
{
    map: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K, V> HashMapVecDequeLru<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Construct with a fixed capacity. Capacity 0 is stored as 0 and never
    /// holds items.
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Get and promote to most-recent.
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.map.get(key).cloned() {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            Some(v)
        } else {
            None
        }
    }

    /// Insert or update. Evicts the least-recent item when over capacity.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        if self.capacity == 0 {
            return None;
        }
        if let Some(pos) = self.order.iter().position(|k| k == &key) {
            self.order.remove(pos);
        }
        let old = self.map.insert(key.clone(), value);
        self.order.push_back(key);
        while self.map.len() > self.capacity {
            if let Some(front) = self.order.pop_front() {
                self.map.remove(&front);
            } else {
                break;
            }
        }
        old
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.map.remove(key)
    }

    /// Live size.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashmap_vecdeque_basic_get_put() {
        let mut lru = HashMapVecDequeLru::new(2);
        assert!(lru.put("a".to_string(), 1).is_none());
        assert!(lru.put("b".to_string(), 2).is_none());
        assert_eq!(lru.get(&"a".to_string()), Some(1));
        assert_eq!(lru.get(&"b".to_string()), Some(2));
    }

    #[test]
    fn hashmap_vecdeque_eviction() {
        let mut lru = HashMapVecDequeLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        lru.put("c".to_string(), 3);
        assert_eq!(lru.get(&"a".to_string()), None);
        assert_eq!(lru.get(&"b".to_string()), Some(2));
        assert_eq!(lru.get(&"c".to_string()), Some(3));
        assert_eq!(lru.len(), 2);
    }

    #[test]
    fn hashmap_vecdeque_update_existing() {
        let mut lru = HashMapVecDequeLru::new(2);
        lru.put("a".to_string(), 1);
        let old = lru.put("a".to_string(), 100);
        assert_eq!(old, Some(1));
        assert_eq!(lru.get(&"a".to_string()), Some(100));
        assert_eq!(lru.len(), 1);
    }

    #[test]
    fn get_promotes_so_older_key_is_evicted() {
        let mut lru = HashMapVecDequeLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        assert_eq!(lru.get(&"a".to_string()), Some(1));
        lru.put("c".to_string(), 3);
        assert_eq!(lru.get(&"b".to_string()), None);
        assert_eq!(lru.get(&"a".to_string()), Some(1));
        assert_eq!(lru.get(&"c".to_string()), Some(3));
    }
}
