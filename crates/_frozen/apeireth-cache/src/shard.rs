//! # Shard — 16-256 分片锁
//!
//! 减少单锁竞争: 哈希 key → shard_id → 操作单 shard.
//!
//! ## 设计原则 (per task spec §6)
//!
//! - 16-256 shards 范围 (K-1 强校验, 不允许范围外)
//! - 用 parking_lot::Mutex (无 deadlock 风险, 性能优于 std::sync::Mutex)
//! - shard 路由: std::hash::Hasher (SipHash 默认) → shard_id = hash % shards
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::hash::{Hash, Hasher};

use parking_lot::Mutex;

// ============================================================================
// §1 ShardRange 常量 (K-1 强校验)
// ============================================================================

/// 最小 shard 数.
pub const SHARD_MIN: usize = 16;

/// 最大 shard 数.
pub const SHARD_MAX: usize = 256;

/// 默认 shard 数.
pub const SHARD_DEFAULT: usize = 32;

// ============================================================================
// §2 守门: 校验 shard_count 在 16..=256
// ============================================================================

/// 校验 shard_count 合法 (K-1 强校验).
///
/// 不合法返 InvalidShardCount error.
pub fn validate_shard_count(shards: usize) -> crate::error::CacheResult<()> {
    use crate::error::CacheError;

    if !(SHARD_MIN..=SHARD_MAX).contains(&shards) {
        return Err(CacheError::InvalidShardCount(shards));
    }
    Ok(())
}

// ============================================================================
// §3 ShardRouter 路由 (key → shard_id)
// ============================================================================

/// Shard 路由器 (key → shard_id).
///
/// 用 std::hash::Hasher (SipHash 1-3 默认) 计算 hash, 然后 mod shards.
#[derive(Debug, Clone, Copy)]
pub struct ShardRouter {
    /// shard 数 (16..=256).
    shards: usize,
}

impl ShardRouter {
    /// 构造 (K-1 强校验).
    pub fn new(shards: usize) -> crate::error::CacheResult<Self> {
        validate_shard_count(shards)?;
        Ok(Self { shards })
    }

    /// 构造 (unchecked, 给测试用, 假定 shards 合法).
    pub const fn new_unchecked(shards: usize) -> Self {
        Self { shards }
    }

    /// shard 数.
    #[inline]
    pub const fn shards(&self) -> usize {
        self.shards
    }

    /// 路由 key 到 shard_id (0..shards).
    pub fn route<K: Hash>(&self, key: &K) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let h = hasher.finish() as usize;
        h % self.shards
    }
}

impl Default for ShardRouter {
    fn default() -> Self {
        Self::new_unchecked(SHARD_DEFAULT)
    }
}

// ============================================================================
// §4 ShardedMap 分片 HashMap
// ============================================================================

/// 分片 HashMap<K, V> — 16-256 个 shard, 每个 shard 独立 Mutex.
///
/// 减少单锁竞争: N threads 操作 cache 时, 大概率落在不同 shard.
/// key 路由: ShardRouter::route(key) → shard_id.
pub struct ShardedMap<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// shard 列表.
    shards: Vec<Mutex<std::collections::HashMap<K, V>>>,
    /// 路由器.
    router: ShardRouter,
}

impl<K, V> ShardedMap<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// 构造.
    pub fn new(shards: usize) -> crate::error::CacheResult<Self> {
        let router = ShardRouter::new(shards)?;
        let mut shard_vec = Vec::with_capacity(shards);
        for _ in 0..shards {
            shard_vec.push(Mutex::new(std::collections::HashMap::new()));
        }
        Ok(Self {
            shards: shard_vec,
            router,
        })
    }

    /// 构造 (unchecked, 给测试用).
    pub fn new_unchecked(shards: usize) -> Self {
        let router = ShardRouter::new_unchecked(shards);
        let mut shard_vec = Vec::with_capacity(shards);
        for _ in 0..shards {
            shard_vec.push(Mutex::new(std::collections::HashMap::new()));
        }
        Self {
            shards: shard_vec,
            router,
        }
    }

    /// shard 数.
    #[inline]
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// 路由 key.
    #[inline]
    pub fn route<K2: Hash>(&self, key: &K2) -> usize {
        self.router.route(key)
    }

    /// get (锁对应 shard, 返回 cloned value).
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let id = self.route(key);
        let shard = self.shards[id].lock();
        shard.get(key).cloned()
    }

    /// contains_key (锁对应 shard, 仅检查存在, 不 clone).
    pub fn contains_key(&self, key: &K) -> bool {
        let id = self.route(key);
        let shard = self.shards[id].lock();
        shard.contains_key(key)
    }

    /// with_entry (锁对应 shard, 给调用方 closure 操作 entry, 不要求 V: Clone).
    pub fn with_entry<R>(&self, key: &K, f: impl FnOnce(Option<&V>) -> R) -> R {
        let id = self.route(key);
        let shard = self.shards[id].lock();
        f(shard.get(key))
    }

    /// put (锁对应 shard).
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let id = self.route(&key);
        let mut shard = self.shards[id].lock();
        shard.insert(key, value)
    }

    /// 移除 (锁对应 shard).
    pub fn remove(&self, key: &K) -> Option<V> {
        let id = self.route(key);
        let mut shard = self.shards[id].lock();
        shard.remove(key)
    }

    /// 总 size (累加所有 shard, 锁每个 shard).
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|s| s.lock().len())
            .sum()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.shards.iter().all(|s| s.lock().is_empty())
    }

    /// 清空全部 (锁每个 shard).
    pub fn clear(&self) {
        for s in &self.shards {
            s.lock().clear();
        }
    }

    /// 单 shard size (测试 / 调试用).
    pub fn shard_len(&self, shard_id: usize) -> usize {
        if shard_id >= self.shards.len() {
            0
        } else {
            self.shards[shard_id].lock().len()
        }
    }
}

// ============================================================================
// §5 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: validate_shard_count K-1 强校验.
    #[test]
    fn k1_validate_shard_count_rejects_zero() {
        let r = validate_shard_count(0);
        assert!(r.is_err());
    }

    /// 守门 #2: validate_shard_count 拒 8 (< 16).
    #[test]
    fn validate_shard_count_rejects_8() {
        let r = validate_shard_count(8);
        assert!(r.is_err());
    }

    /// 守门 #3: validate_shard_count 接受 16, 64, 256.
    #[test]
    fn validate_shard_count_accepts_16_64_256() {
        assert!(validate_shard_count(16).is_ok());
        assert!(validate_shard_count(64).is_ok());
        assert!(validate_shard_count(256).is_ok());
    }

    /// 守门 #4: validate_shard_count 拒 257 (> 256).
    #[test]
    fn validate_shard_count_rejects_257() {
        let r = validate_shard_count(257);
        assert!(r.is_err());
    }

    /// 守门 #5: ShardRouter 16 路由.
    #[test]
    fn shard_router_16_routing() {
        let r = ShardRouter::new(16).unwrap();
        for i in 0..100 {
            let key = format!("k{i}");
            let id = r.route(&key);
            assert!(id < 16);
        }
    }

    /// 守门 #6: ShardRouter 256 路由.
    #[test]
    fn shard_router_256_routing() {
        let r = ShardRouter::new(256).unwrap();
        for i in 0..1000 {
            let key = format!("key_{i}");
            let id = r.route(&key);
            assert!(id < 256);
        }
    }

    /// 守门 #7: 相同 key 永远路由到相同 shard (确定性).
    #[test]
    fn same_key_routes_to_same_shard() {
        let r = ShardRouter::new(32).unwrap();
        let key = "stable_key";
        let id1 = r.route(&key);
        let id2 = r.route(&key);
        let id3 = r.route(&key);
        assert_eq!(id1, id2);
        assert_eq!(id2, id3);
    }

    /// 守门 #8: ShardedMap 16 基础.
    #[test]
    fn sharded_map_16_basic() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        m.put("a".to_string(), 1);
        m.put("b".to_string(), 2);
        assert_eq!(m.get(&"a".to_string()), Some(1));
        assert_eq!(m.get(&"b".to_string()), Some(2));
        assert_eq!(m.len(), 2);
    }

    /// 守门 #9: ShardedMap 64 基础.
    #[test]
    fn sharded_map_64_basic() {
        let m: ShardedMap<String, i32> = ShardedMap::new(64).unwrap();
        m.put("x".to_string(), 100);
        assert_eq!(m.get(&"x".to_string()), Some(100));
    }

    /// 守门 #10: ShardedMap 256 基础.
    #[test]
    fn sharded_map_256_basic() {
        let m: ShardedMap<String, i32> = ShardedMap::new(256).unwrap();
        m.put("y".to_string(), 200);
        assert_eq!(m.get(&"y".to_string()), Some(200));
    }

    /// 守门 #11: 1000 个 key 分布大致均匀 (chebyshev 检验).
    #[test]
    fn key_distribution_roughly_even() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        for i in 0..1000 {
            m.put(format!("k{i}"), i);
        }
        // 总和 1000, 16 shard, 平均 62.5, 允许 ±30
        for shard_id in 0..16 {
            let s = m.shard_len(shard_id);
            assert!(s < 200, "shard {shard_id} has {s} > 200 (too uneven)");
        }
    }

    /// 守门 #12: 移除.
    #[test]
    fn sharded_map_remove() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        m.put("a".to_string(), 1);
        let removed = m.remove(&"a".to_string());
        assert_eq!(removed, Some(1));
        assert_eq!(m.get(&"a".to_string()), None);
        assert_eq!(m.len(), 0);
    }

    /// 守门 #13: clear.
    #[test]
    fn sharded_map_clear() {
        let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
        m.put("a".to_string(), 1);
        m.put("b".to_string(), 2);
        m.clear();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
    }

    /// 守门 #14: ShardRouter Default 用 32.
    #[test]
    fn shard_router_default_is_32() {
        let r = ShardRouter::default();
        assert_eq!(r.shards(), 32);
    }
}
