//! 16–256 shard map. Uses `std::sync::Mutex` (no parking_lot).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

/// Minimum shard count.
pub const SHARD_MIN: usize = 16;
/// Maximum shard count.
pub const SHARD_MAX: usize = 256;
/// Default shard count.
pub const SHARD_DEFAULT: usize = 32;

/// Reject counts outside 16..=256.
pub fn validate_shard_count(shards: usize) -> Result<(), usize> {
    if (SHARD_MIN..=SHARD_MAX).contains(&shards) {
        Ok(())
    } else {
        Err(shards)
    }
}

/// Hash key → shard id.
#[derive(Debug, Clone, Copy)]
pub struct ShardRouter {
    shards: usize,
}

impl ShardRouter {
    /// Validated constructor.
    pub fn new(shards: usize) -> Result<Self, usize> {
        validate_shard_count(shards)?;
        Ok(Self { shards })
    }

    /// Skip validation (tests).
    pub const fn new_unchecked(shards: usize) -> Self {
        Self { shards }
    }

    /// Shard count.
    pub const fn shards(&self) -> usize {
        self.shards
    }

    /// Route `key` to `0..shards`.
    pub fn route<K: Hash>(&self, key: &K) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shards
    }
}

impl Default for ShardRouter {
    fn default() -> Self {
        Self::new_unchecked(SHARD_DEFAULT)
    }
}

/// Sharded `HashMap` with one mutex per shard.
pub struct ShardedMap<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    shards: Vec<Mutex<HashMap<K, V>>>,
    router: ShardRouter,
}

impl<K, V> ShardedMap<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// Validated constructor.
    pub fn new(shards: usize) -> Result<Self, usize> {
        let router = ShardRouter::new(shards)?;
        Ok(Self::from_router(router))
    }

    /// Skip validation (tests).
    pub fn new_unchecked(shards: usize) -> Self {
        Self::from_router(ShardRouter::new_unchecked(shards))
    }

    fn from_router(router: ShardRouter) -> Self {
        let mut shard_vec = Vec::with_capacity(router.shards());
        for _ in 0..router.shards() {
            shard_vec.push(Mutex::new(HashMap::new()));
        }
        Self {
            shards: shard_vec,
            router,
        }
    }

    /// Shard count.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Route a key.
    pub fn route<K2: Hash>(&self, key: &K2) -> usize {
        self.router.route(key)
    }

    /// Cloned get.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let id = self.route(key);
        self.shards[id].lock().ok()?.get(key).cloned()
    }

    /// Existence check.
    pub fn contains_key(&self, key: &K) -> bool {
        let id = self.route(key);
        self.shards
            .get(id)
            .and_then(|s| s.lock().ok())
            .map(|shard| shard.contains_key(key))
            .unwrap_or(false)
    }

    /// Insert, returning the previous value.
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let id = self.route(&key);
        self.shards
            .get(id)
            .and_then(|s| s.lock().ok())
            .and_then(|mut shard| shard.insert(key, value))
    }

    /// Remove.
    pub fn remove(&self, key: &K) -> Option<V> {
        let id = self.route(key);
        self.shards
            .get(id)
            .and_then(|s| s.lock().ok())
            .and_then(|mut shard| shard.remove(key))
    }

    /// Sum of shard lengths.
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .filter_map(|s| s.lock().ok().map(|g| g.len()))
            .sum()
    }

    /// All shards empty?
    pub fn is_empty(&self) -> bool {
        self.shards
            .iter()
            .all(|s| s.lock().map(|g| g.is_empty()).unwrap_or(true))
    }

    /// Clear every shard.
    pub fn clear(&self) {
        for s in &self.shards {
            if let Ok(mut g) = s.lock() {
                g.clear();
            }
        }
    }

    /// Per-shard length (debug).
    pub fn shard_len(&self, shard_id: usize) -> usize {
        self.shards
            .get(shard_id)
            .and_then(|s| s.lock().ok().map(|g| g.len()))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_shard_count_bounds() {
        assert!(validate_shard_count(0).is_err());
        assert!(validate_shard_count(8).is_err());
        assert!(validate_shard_count(16).is_ok());
        assert!(validate_shard_count(64).is_ok());
        assert!(validate_shard_count(256).is_ok());
        assert!(validate_shard_count(257).is_err());
    }

    #[test]
    fn shard_router_stays_in_range() {
        let r = ShardRouter::new(16).unwrap();
        for i in 0..100 {
            let id = r.route(&format!("k{i}"));
            assert!(id < 16);
        }
    }

    #[test]
    fn same_key_routes_to_same_shard() {
        let r = ShardRouter::new(32).unwrap();
        let key = "stable_key";
        assert_eq!(r.route(&key), r.route(&key));
    }

    #[test]
    fn sharded_map_basic() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        m.put("a".to_string(), 1);
        m.put("b".to_string(), 2);
        assert_eq!(m.get(&"a".to_string()), Some(1));
        assert_eq!(m.get(&"b".to_string()), Some(2));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn key_distribution_roughly_even() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        for i in 0..1000 {
            m.put(format!("k{i}"), i);
        }
        for shard_id in 0..16 {
            let s = m.shard_len(shard_id);
            assert!(s < 200, "shard {shard_id} has {s} > 200");
        }
    }

    #[test]
    fn sharded_map_remove_and_clear() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        m.put("a".to_string(), 1);
        assert_eq!(m.remove(&"a".to_string()), Some(1));
        m.put("b".to_string(), 2);
        m.clear();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
    }

    #[test]
    fn shard_router_default_is_32() {
        assert_eq!(ShardRouter::default().shards(), 32);
    }
}
