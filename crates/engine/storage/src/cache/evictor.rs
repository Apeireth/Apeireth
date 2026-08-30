//! Five eviction policies recovered from frozen apeireth-cache.
//!
//! ARC and TinyLFU are honest approximations (dual-list + 3-bit frequency),
//! not IBM ARC / Caffeine TinyLFU.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Eviction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvictionPolicy {
    /// Least recently used.
    Lru,
    /// Least frequently used.
    Lfu,
    /// Insertion order.
    Fifo,
    /// Simplified adaptive replacement (T1/T2 lists).
    Arc,
    /// Simplified frequency-aware FIFO.
    TinyLfu,
}

impl EvictionPolicy {
    /// All policies.
    pub const ALL: [EvictionPolicy; 5] = [
        EvictionPolicy::Lru,
        EvictionPolicy::Lfu,
        EvictionPolicy::Fifo,
        EvictionPolicy::Arc,
        EvictionPolicy::TinyLfu,
    ];

    /// Canonical name.
    pub const fn as_str(&self) -> &'static str {
        match self {
            EvictionPolicy::Lru => "LRU",
            EvictionPolicy::Lfu => "LFU",
            EvictionPolicy::Fifo => "FIFO",
            EvictionPolicy::Arc => "ARC",
            EvictionPolicy::TinyLfu => "TINY_LFU",
        }
    }
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        EvictionPolicy::Lru
    }
}

impl std::fmt::Display for EvictionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EvictionPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LRU" | "lru" => Ok(EvictionPolicy::Lru),
            "LFU" | "lfu" => Ok(EvictionPolicy::Lfu),
            "FIFO" | "fifo" => Ok(EvictionPolicy::Fifo),
            "ARC" | "arc" => Ok(EvictionPolicy::Arc),
            "TINY_LFU" | "tiny_lfu" | "TinyLFU" | "tinylfu" => Ok(EvictionPolicy::TinyLfu),
            other => Err(format!("unknown eviction policy: '{other}'")),
        }
    }
}

/// Internal eviction tracker.
pub(crate) trait Evictor<K>: Send
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, key: &K);
    fn on_insert(&mut self, key: K);
    fn pick_victim(&mut self) -> Option<K>;
    fn on_remove(&mut self, key: &K);
    fn policy(&self) -> EvictionPolicy;
}

fn rebuild_index<K: Hash + Eq + Clone>(order: &VecDeque<K>) -> HashMap<K, usize> {
    let mut index = HashMap::with_capacity(order.len());
    for (i, k) in order.iter().enumerate() {
        index.insert(k.clone(), i);
    }
    index
}

fn remove_at<K: Hash + Eq + Clone>(
    order: &mut VecDeque<K>,
    index: &mut HashMap<K, usize>,
    key: &K,
) {
    if let Some(&pos) = index.get(key) {
        if pos < order.len() && &order[pos] == key {
            order.remove(pos);
            *index = rebuild_index(order);
            return;
        }
    }
    if let Some(pos) = order.iter().position(|k| k == key) {
        order.remove(pos);
        *index = rebuild_index(order);
    } else {
        index.remove(key);
    }
}

struct LruEvictor<K: Hash + Eq + Clone> {
    order: VecDeque<K>,
    index: HashMap<K, usize>,
}

impl<K: Hash + Eq + Clone> LruEvictor<K> {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            index: HashMap::new(),
        }
    }
}

impl<K> Evictor<K> for LruEvictor<K>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, key: &K) {
        if self.index.contains_key(key) {
            remove_at(&mut self.order, &mut self.index, key);
            self.order.push_back(key.clone());
            self.index = rebuild_index(&self.order);
        }
    }

    fn on_insert(&mut self, key: K) {
        if self.index.contains_key(&key) {
            remove_at(&mut self.order, &mut self.index, &key);
        }
        self.order.push_back(key);
        self.index = rebuild_index(&self.order);
    }

    fn pick_victim(&mut self) -> Option<K> {
        let k = self.order.pop_front()?;
        self.index.remove(&k);
        self.index = rebuild_index(&self.order);
        Some(k)
    }

    fn on_remove(&mut self, key: &K) {
        remove_at(&mut self.order, &mut self.index, key);
    }

    fn policy(&self) -> EvictionPolicy {
        EvictionPolicy::Lru
    }
}

struct LfuEvictor<K: Hash + Eq + Clone> {
    freq: HashMap<K, u32>,
    buckets: HashMap<u32, VecDeque<K>>,
    min_freq: u32,
}

impl<K: Hash + Eq + Clone> LfuEvictor<K> {
    fn new() -> Self {
        Self {
            freq: HashMap::new(),
            buckets: HashMap::new(),
            min_freq: 0,
        }
    }
}

impl<K> Evictor<K> for LfuEvictor<K>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, key: &K) {
        if let Some(&f) = self.freq.get(key) {
            if let Some(bucket) = self.buckets.get_mut(&f) {
                bucket.retain(|k| k != key);
                if bucket.is_empty() {
                    self.buckets.remove(&f);
                    if self.min_freq == f {
                        self.min_freq = self.buckets.keys().min().copied().unwrap_or(f + 1);
                    }
                }
            }
            let new_f = f + 1;
            self.buckets
                .entry(new_f)
                .or_default()
                .push_back(key.clone());
            self.freq.insert(key.clone(), new_f);
        }
    }

    fn on_insert(&mut self, key: K) {
        self.freq.insert(key.clone(), 1);
        self.buckets.entry(1).or_default().push_back(key);
        self.min_freq = 1;
    }

    fn pick_victim(&mut self) -> Option<K> {
        let bucket = self.buckets.get_mut(&self.min_freq)?;
        let k = bucket.pop_front()?;
        self.freq.remove(&k);
        if bucket.is_empty() {
            self.buckets.remove(&self.min_freq);
            self.min_freq = self.buckets.keys().min().copied().unwrap_or(0);
        }
        Some(k)
    }

    fn on_remove(&mut self, key: &K) {
        if let Some(f) = self.freq.remove(key) {
            if let Some(bucket) = self.buckets.get_mut(&f) {
                bucket.retain(|k| k != key);
                if bucket.is_empty() {
                    self.buckets.remove(&f);
                    if self.min_freq == f {
                        self.min_freq = self.buckets.keys().min().copied().unwrap_or(0);
                    }
                }
            }
        }
    }

    fn policy(&self) -> EvictionPolicy {
        EvictionPolicy::Lfu
    }
}

struct FifoEvictor<K: Hash + Eq + Clone> {
    order: VecDeque<K>,
    index: HashMap<K, usize>,
}

impl<K: Hash + Eq + Clone> FifoEvictor<K> {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            index: HashMap::new(),
        }
    }
}

impl<K> Evictor<K> for FifoEvictor<K>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, _key: &K) {}

    fn on_insert(&mut self, key: K) {
        if self.index.contains_key(&key) {
            return;
        }
        self.order.push_back(key);
        self.index = rebuild_index(&self.order);
    }

    fn pick_victim(&mut self) -> Option<K> {
        let k = self.order.pop_front()?;
        self.index.remove(&k);
        self.index = rebuild_index(&self.order);
        Some(k)
    }

    fn on_remove(&mut self, key: &K) {
        remove_at(&mut self.order, &mut self.index, key);
    }

    fn policy(&self) -> EvictionPolicy {
        EvictionPolicy::Fifo
    }
}

/// Simplified ARC: T1 (recent) + T2 (frequent). Ghost lists are recorded but
/// `p` is not adapted — donor already labelled this as not IBM ARC.
struct ArcEvictor<K: Hash + Eq + Clone> {
    t1: VecDeque<K>,
    t2: VecDeque<K>,
    b1: HashMap<K, ()>,
    b2: HashMap<K, ()>,
    index_t1: HashMap<K, usize>,
    index_t2: HashMap<K, usize>,
}

impl<K: Hash + Eq + Clone> ArcEvictor<K> {
    fn new() -> Self {
        Self {
            t1: VecDeque::new(),
            t2: VecDeque::new(),
            b1: HashMap::new(),
            b2: HashMap::new(),
            index_t1: HashMap::new(),
            index_t2: HashMap::new(),
        }
    }

    fn rebuild(&mut self) {
        self.index_t1 = rebuild_index(&self.t1);
        self.index_t2 = rebuild_index(&self.t2);
    }
}

impl<K> Evictor<K> for ArcEvictor<K>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, key: &K) {
        if self.index_t2.contains_key(key) {
            remove_at(&mut self.t2, &mut self.index_t2, key);
            self.t2.push_back(key.clone());
        } else if self.index_t1.contains_key(key) {
            remove_at(&mut self.t1, &mut self.index_t1, key);
            self.t2.push_back(key.clone());
            self.b1.remove(key);
        } else if self.b2.contains_key(key) {
            self.t2.push_back(key.clone());
            self.b2.remove(key);
        }
        self.rebuild();
    }

    fn on_insert(&mut self, key: K) {
        if self.b1.remove(&key).is_some() {
            self.t2.push_back(key);
        } else if !self.index_t1.contains_key(&key) && !self.index_t2.contains_key(&key) {
            self.t1.push_back(key);
        }
        self.rebuild();
    }

    fn pick_victim(&mut self) -> Option<K> {
        if !self.t1.is_empty() {
            if let Some(k) = self.t1.pop_front() {
                self.b1.insert(k.clone(), ());
                self.rebuild();
                return Some(k);
            }
        }
        if let Some(k) = self.t2.pop_front() {
            self.b2.insert(k.clone(), ());
            self.rebuild();
            return Some(k);
        }
        None
    }

    fn on_remove(&mut self, key: &K) {
        remove_at(&mut self.t1, &mut self.index_t1, key);
        remove_at(&mut self.t2, &mut self.index_t2, key);
        self.b1.remove(key);
        self.b2.remove(key);
        self.rebuild();
    }

    fn policy(&self) -> EvictionPolicy {
        EvictionPolicy::Arc
    }
}

struct TinyLfuEvictor<K: Hash + Eq + Clone> {
    freq: HashMap<K, u8>,
    order: VecDeque<K>,
    index: HashMap<K, usize>,
}

impl<K: Hash + Eq + Clone> TinyLfuEvictor<K> {
    fn new() -> Self {
        Self {
            freq: HashMap::new(),
            order: VecDeque::new(),
            index: HashMap::new(),
        }
    }
}

impl<K> Evictor<K> for TinyLfuEvictor<K>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn on_access(&mut self, key: &K) {
        if let Some(f) = self.freq.get_mut(key) {
            if *f < 7 {
                *f += 1;
            }
        }
    }

    fn on_insert(&mut self, key: K) {
        self.freq.insert(key.clone(), 1);
        if !self.index.contains_key(&key) {
            self.order.push_back(key);
            self.index = rebuild_index(&self.order);
        }
    }

    fn pick_victim(&mut self) -> Option<K> {
        let mut victim: Option<(K, u8, usize)> = None;
        for (i, k) in self.order.iter().enumerate() {
            let f = self.freq.get(k).copied().unwrap_or(0);
            match &victim {
                None => victim = Some((k.clone(), f, i)),
                Some((_, vf, vi)) => {
                    if f < *vf || (f == *vf && i < *vi) {
                        victim = Some((k.clone(), f, i));
                    }
                }
            }
        }
        if let Some((k, _, _)) = victim {
            self.freq.remove(&k);
            remove_at(&mut self.order, &mut self.index, &k);
            Some(k)
        } else {
            None
        }
    }

    fn on_remove(&mut self, key: &K) {
        self.freq.remove(key);
        remove_at(&mut self.order, &mut self.index, key);
    }

    fn policy(&self) -> EvictionPolicy {
        EvictionPolicy::TinyLfu
    }
}

pub(crate) fn build_evictor<K>(policy: EvictionPolicy) -> Box<dyn Evictor<K>>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    match policy {
        EvictionPolicy::Lru => Box::new(LruEvictor::new()),
        EvictionPolicy::Lfu => Box::new(LfuEvictor::new()),
        EvictionPolicy::Fifo => Box::new(FifoEvictor::new()),
        EvictionPolicy::Arc => Box::new(ArcEvictor::new()),
        EvictionPolicy::TinyLfu => Box::new(TinyLfuEvictor::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(k: &str) -> String {
        k.to_string()
    }

    #[test]
    fn eviction_policy_five() {
        assert_eq!(EvictionPolicy::ALL.len(), 5);
        assert_eq!(EvictionPolicy::Lru.as_str(), "LRU");
        assert_eq!("tiny_lfu".parse::<EvictionPolicy>().unwrap(), EvictionPolicy::TinyLfu);
        assert!("NOPE".parse::<EvictionPolicy>().is_err());
    }

    #[test]
    fn lru_evictor_kicks_lru() {
        let mut e = LruEvictor::new();
        e.on_insert(kv("a"));
        e.on_insert(kv("b"));
        e.on_insert(kv("c"));
        e.on_access(&kv("a"));
        assert_eq!(e.pick_victim(), Some(kv("b")));
    }

    #[test]
    fn lfu_evictor_kicks_least_frequent() {
        let mut e = LfuEvictor::new();
        e.on_insert(kv("a"));
        e.on_insert(kv("b"));
        e.on_access(&kv("a"));
        e.on_access(&kv("a"));
        assert_eq!(e.pick_victim(), Some(kv("b")));
    }

    #[test]
    fn fifo_evictor_kicks_first_inserted() {
        let mut e = FifoEvictor::new();
        e.on_insert(kv("a"));
        e.on_insert(kv("b"));
        e.on_access(&kv("a"));
        assert_eq!(e.pick_victim(), Some(kv("a")));
    }

    #[test]
    fn arc_evictor_promotes_t1_to_t2() {
        let mut e = ArcEvictor::new();
        e.on_insert(kv("a"));
        e.on_access(&kv("a"));
        e.on_insert(kv("b"));
        assert_eq!(e.pick_victim(), Some(kv("b")));
    }

    #[test]
    fn tiny_lfu_evictor_uses_freq_first() {
        let mut e = TinyLfuEvictor::new();
        e.on_insert(kv("a"));
        e.on_insert(kv("b"));
        e.on_access(&kv("a"));
        e.on_access(&kv("a"));
        assert_eq!(e.pick_victim(), Some(kv("b")));
    }

    #[test]
    fn lru_on_remove_clears_index() {
        let mut e = LruEvictor::new();
        e.on_insert(kv("a"));
        e.on_insert(kv("b"));
        e.on_remove(&kv("a"));
        assert_eq!(e.pick_victim(), Some(kv("b")));
        assert_eq!(e.pick_victim(), None);
    }

    #[test]
    fn factory_builds_all_policies() {
        for p in EvictionPolicy::ALL {
            let e: Box<dyn Evictor<String>> = build_evictor(p);
            assert_eq!(e.policy(), p);
        }
    }
}
