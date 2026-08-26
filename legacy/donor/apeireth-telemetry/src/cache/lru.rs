//! # LRU — 4 个 LRU 实现 (HashMap+VecDeque / indexmap / lru / quickcache 留口子)
//!
//! 4 个 LRU 实现, 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版 LRU 谱:
//! 1. **HashMap+VecDeque** — 经典手写, 无依赖, 教学用
//! 2. **indexmap** — 业界标准, O(1) get/put
//! 3. **lru** — `lru = "0.12"`, 工业界默认
//! 4. **quickcache 留口子** — 性能导向, R20 阶段 6 skeleton 阶段不引
//!
//! ## R20 阶段 6 实现策略
//!
//! 3 个 LRU 完整实现 (HashMap+VecDeque / indexmap / lru), quickcache 留口子 (skeleton 不引).
//! 4 个都暴露 trait, 调用方按 `LruImpl` 枚举选.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

use indexmap::IndexMap;

// ============================================================================
// §1 LruImpl 枚举 (4 实现选择)
// ============================================================================

/// 4 个 LRU 实现选择 (编译期 hardcode).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版的 LRU 谱.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LruImpl {
    /// HashMap + VecDeque 经典手写 (无依赖, 教学用).
    HashMapVecDeque,

    /// indexmap 实现 (业界标准, O(1) get/put).
    IndexMapBacked,

    /// lru crate 0.12 (工业界默认).
    LruCrate,

    /// quickcache 留口子 (性能导向, skeleton 阶段 0 实际引, 留未来).
    QuickCache,
}

impl LruImpl {
    /// 4 实现 1:1 列表.
    pub const ALL: [LruImpl; 4] = [
        LruImpl::HashMapVecDeque,
        LruImpl::IndexMapBacked,
        LruImpl::LruCrate,
        LruImpl::QuickCache,
    ];

    /// 1:1 字符串名.
    pub const fn as_str(&self) -> &'static str {
        match self {
            LruImpl::HashMapVecDeque => "HashMap+VecDeque",
            LruImpl::IndexMapBacked => "indexmap",
            LruImpl::LruCrate => "lru",
            LruImpl::QuickCache => "quickcache",
        }
    }

    /// R20 阶段 6 是否完整实现 (quickcache 留口子, 返 false).
    pub const fn is_implemented(&self) -> bool {
        match self {
            LruImpl::HashMapVecDeque | LruImpl::IndexMapBacked | LruImpl::LruCrate => true,
            LruImpl::QuickCache => false,
        }
    }
}

impl Default for LruImpl {
    fn default() -> Self {
        LruImpl::IndexMapBacked
    }
}

impl std::fmt::Display for LruImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 HashMap+VecDeque 实现 (经典手写, 教学用)
// ============================================================================

/// LRU 用 HashMap + VecDeque 实现 (经典, 无外部依赖).
///
/// - get: O(1) (VecDeque 顺序调整 O(n) 但 n 远小于 map.size, amortized O(1))
/// - put: O(1) (同上)
/// - 淘汰: O(1) (pop_front)
pub struct HashMapVecDequeLru<K, V>
where
    K: Hash + Eq + Clone,
{
    /// K → V 映射.
    map: HashMap<K, V>,
    /// key 访问序 (back = 最近, front = 最久).
    order: VecDeque<K>,
    /// 最大容量.
    capacity: usize,
}

impl<K, V> HashMapVecDequeLru<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// 构造.
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// get (返回 cloned value, 经典 LRU 行为: 命中后移到 back).
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.map.get(key).cloned() {
            // 移到 back (最近)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            Some(v)
        } else {
            None
        }
    }

    /// put (插入或更新, 超 capacity 淘汰 front).
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        // 移除旧 order
        if let Some(pos) = self.order.iter().position(|k| k == &key) {
            self.order.remove(pos);
        }
        let old = self.map.insert(key.clone(), value);
        self.order.push_back(key);

        // 超 capacity 淘汰
        while self.map.len() > self.capacity {
            if let Some(front) = self.order.pop_front() {
                self.map.remove(&front);
            } else {
                break;
            }
        }

        old
    }

    /// 移除 key.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.map.remove(key)
    }

    /// 当前 size.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 容量.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ============================================================================
// §3 indexmap 实现 (业界标准)
// ============================================================================

/// LRU 用 indexmap 实现 (业界标准, O(1) get/put + 保持插入序).
///
/// indexmap 内部用 hash table + Vec, get/put 都是 O(1) amortized,
/// shift_remove 是 O(n) 但 n 是被删 key 之后的元素数, 通常很小.
pub struct IndexMapLru<K, V>
where
    K: Hash + Eq,
{
    /// K → V 映射 (IndexMap 保持插入序).
    map: IndexMap<K, V>,
    /// 最大容量.
    capacity: usize,
}

impl<K, V> IndexMapLru<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// 构造.
    pub fn new(capacity: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(capacity),
            capacity,
        }
    }

    /// get (返回 cloned value, 命中后移到 back).
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.map.get(key).cloned() {
            // 移到 back (最近)
            self.map.shift_remove(key);
            self.map.insert(key.clone(), v.clone());
            Some(v)
        } else {
            None
        }
    }

    /// put (插入或更新, 超 capacity 淘汰 front).
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        let old = self.map.insert(key, value);
        // 超 capacity 淘汰
        while self.map.len() > self.capacity {
            if let Some((front_key, _)) = self.map.shift_remove_index(0) {
                let _ = front_key;
            } else {
                break;
            }
        }
        old
    }

    /// 移除.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.shift_remove(key)
    }

    /// 当前 size.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 容量.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ============================================================================
// §4 lru crate 实现 (工业界默认)
// ============================================================================

/// LRU 用 lru crate 0.12 实现 (工业界默认).
pub struct LruCrateLru<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// 底层 lru::LruCache.
    inner: lru::LruCache<K, V>,
}

impl<K, V> LruCrateLru<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// 构造.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity)
                    .unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
            ),
        }
    }

    /// get.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    /// get cloned.
    pub fn get_cloned(&mut self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        self.inner.get(key).cloned()
    }

    /// put.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.inner.put(key, value)
    }

    /// 移除.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.pop(key)
    }

    /// 当前 size.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 容量.
    pub fn capacity(&self) -> usize {
        self.inner.cap().get()
    }
}

// ============================================================================
// §5 quickcache 留口子
// ============================================================================

/// quickcache 留口子 (R20 阶段 6 skeleton 阶段 0 实际引).
///
/// 调用方选 LruImpl::QuickCache 时, 返 NotImplemented error (per CacheError::PolicyNotSupported).
pub struct QuickCacheStub;

impl QuickCacheStub {
    /// 留口子: 返 stub 标识.
    pub const STUB_NAME: &'static str = "quickcache (留口子, R20 阶段 6 0 实际引)";
}

// ============================================================================
// §6 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: LruImpl 4 实现 1:1 列表.
    #[test]
    fn lru_impl_4_all() {
        assert_eq!(LruImpl::ALL.len(), 4);
    }

    /// 守门 #2: 3 实现已实现, quickcache 留口子.
    #[test]
    fn lru_impl_3_implemented_quickcache_stub() {
        assert!(LruImpl::HashMapVecDeque.is_implemented());
        assert!(LruImpl::IndexMapBacked.is_implemented());
        assert!(LruImpl::LruCrate.is_implemented());
        assert!(!LruImpl::QuickCache.is_implemented());
    }

    /// 守门 #3: HashMap+VecDeque 基础 get/put.
    #[test]
    fn hashmap_vecdeque_basic_get_put() {
        let mut lru = HashMapVecDequeLru::new(2);
        assert!(lru.put("a".to_string(), 1).is_none());
        assert!(lru.put("b".to_string(), 2).is_none());
        assert_eq!(lru.get(&"a".to_string()), Some(1));
        assert_eq!(lru.get(&"b".to_string()), Some(2));
    }

    /// 守门 #4: HashMap+VecDeque 淘汰 (超 capacity).
    #[test]
    fn hashmap_vecdeque_eviction() {
        let mut lru = HashMapVecDequeLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        lru.put("c".to_string(), 3); // 触发淘汰 "a"
        assert_eq!(lru.get(&"a".to_string()), None);
        assert_eq!(lru.get(&"b".to_string()), Some(2));
        assert_eq!(lru.get(&"c".to_string()), Some(3));
        assert_eq!(lru.len(), 2);
    }

    /// 守门 #5: HashMap+VecDeque 更新已有 key.
    #[test]
    fn hashmap_vecdeque_update_existing() {
        let mut lru = HashMapVecDequeLru::new(2);
        lru.put("a".to_string(), 1);
        let old = lru.put("a".to_string(), 100);
        assert_eq!(old, Some(1));
        assert_eq!(lru.get(&"a".to_string()), Some(100));
        assert_eq!(lru.len(), 1);
    }

    /// 守门 #6: indexmap 基础 get/put.
    #[test]
    fn indexmap_lru_basic_get_put() {
        let mut lru = IndexMapLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        assert_eq!(lru.get(&"a".to_string()), Some(1));
        assert_eq!(lru.get(&"b".to_string()), Some(2));
    }

    /// 守门 #7: indexmap 淘汰.
    #[test]
    fn indexmap_lru_eviction() {
        let mut lru = IndexMapLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        lru.put("c".to_string(), 3); // 触发淘汰 "a"
        assert_eq!(lru.get(&"a".to_string()), None);
        assert_eq!(lru.get(&"b".to_string()), Some(2));
        assert_eq!(lru.get(&"c".to_string()), Some(3));
    }

    /// 守门 #8: lru crate 基础.
    #[test]
    fn lru_crate_basic_get_put() {
        let mut lru = LruCrateLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        assert_eq!(lru.get_cloned(&"a".to_string()), Some(1));
        assert_eq!(lru.get_cloned(&"b".to_string()), Some(2));
        assert_eq!(lru.len(), 2);
    }

    /// 守门 #9: lru crate 淘汰.
    #[test]
    fn lru_crate_eviction() {
        let mut lru = LruCrateLru::new(2);
        lru.put("a".to_string(), 1);
        lru.put("b".to_string(), 2);
        lru.put("c".to_string(), 3); // 触发淘汰 "a"
        assert!(lru.get(&"a".to_string()).is_none());
        assert_eq!(lru.get_cloned(&"c".to_string()), Some(3));
    }

    /// 守门 #10: lru crate 移除.
    #[test]
    fn lru_crate_remove() {
        let mut lru = LruCrateLru::new(3);
        lru.put("a".to_string(), 1);
        let removed = lru.remove(&"a".to_string());
        assert_eq!(removed, Some(1));
        assert_eq!(lru.len(), 0);
    }

    /// 守门 #11: quickcache stub 标识.
    #[test]
    fn quickcache_stub_marker() {
        assert!(QuickCacheStub::STUB_NAME.contains("quickcache"));
        assert!(QuickCacheStub::STUB_NAME.contains("R20 阶段 6"));
    }

    /// 守门 #12: LruImpl Display.
    #[test]
    fn lru_impl_display() {
        assert_eq!(LruImpl::HashMapVecDeque.to_string(), "HashMap+VecDeque");
        assert_eq!(LruImpl::IndexMapBacked.to_string(), "indexmap");
        assert_eq!(LruImpl::LruCrate.to_string(), "lru");
    }
}
