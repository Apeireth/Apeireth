# R213 tool-codesearch 真 LRU + streaming + batch (接续 R210)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R213
> **日期**: 2026-08-13
> **来源**: R210 简化版 LRU + 主人"全做全做全补弱 + 一体化优美"
> **状态**: 实施完成, 12/12 单测全过 (累计 89/89)

---

## 0. 动机

R210 QueryCache 用简化 LRU (超 max_entries 时随机清前 25%). R213 升级:
1. **真 LRU** (用 `lru` crate 0.16) — O(1) get/put, 真按访问时间淘汰
2. **streaming_query** — callback-based 流式返回, 内存占用低
3. **batch_query** — 多 query 共享 cache, 重复 query 自动 hit

---

## 1. 设计

### 1.1 公共 API

```rust
pub struct LruQueryCache {
    inner: Mutex<LruCache<CacheKey, CacheEntry>>,
    ttl: Duration,
    hits: AtomicU64, misses: AtomicU64, evictions: AtomicU64,
}
impl LruQueryCache {
    pub fn new(ttl_ms: u64, capacity: usize) -> Self;
    pub fn with_defaults() -> Self;  // 60s TTL, 1000 cap
    pub fn get(&self, q: &UnifiedQuery) -> Option<Vec<IntelligenceHit>>;
    pub fn put(&self, q: &UnifiedQuery, hits: Vec<IntelligenceHit>);
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool;
    pub fn clear(&self);
    pub fn stats(&self) -> LruCacheStats;
}

pub struct LruCacheStats {
    pub size: usize, pub capacity: usize,
    pub hits: u64, pub misses: u64, pub evictions: u64,
}

pub type HitCallback<'a> = dyn FnMut(&IntelligenceHit) -> bool + Send + 'a;

pub fn streaming_query<F>(cache, inner, q, on_hit) -> Result<u64, UnifiedError>
where F: FnMut(&IntelligenceHit) -> bool;

pub fn batch_query(cache, inner, queries: &[UnifiedQuery])
    -> Vec<Result<Vec<IntelligenceHit>, UnifiedError>>;

pub struct CachedUnifiedLru {
    cache: LruQueryCache,
    inner: Arc<UnifiedCodeIntelligence>,
}
impl CachedUnifiedLru {
    pub fn new(ttl_ms: u64, capacity: usize) -> Self;
    pub fn with_defaults() -> Self;
    pub fn query(&self, q: &UnifiedQuery) -> Result<Vec<IntelligenceHit>, UnifiedError>;
    pub fn streaming<F: FnMut(&IntelligenceHit) -> bool>(&self, q, on_hit) -> Result<u64, UnifiedError>;
    pub fn batch(&self, queries: &[UnifiedQuery]) -> Vec<Result<Vec<IntelligenceHit>, UnifiedError>>;
    pub fn stats(&self) -> LruCacheStats;
    pub fn clear(&self);
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool;
    pub fn inner(&self) -> &UnifiedCodeIntelligence;
}
```

### 1.2 真 LRU 行为

用 `lru` crate 0.16:
- `LruCache::push` 满了自动淘汰最久未访问
- `get` 命中后更新访问顺序 (touch)
- O(1) get/put

### 1.3 streaming 模式

callback-based: `FnMut(&IntelligenceHit) -> bool`, 返回 false 提前终止. 用于:
- "匹配成千上万但只要前 100"
- 大结果集的渐进式 UI 渲染
- early-exit 优化

### 1.4 batch 模式

一组 query 顺序处理, 共享 cache. 第二次相同 query 自动 hit.

### 1.5 0 引外部 dep (除 lru)

`lru = { workspace = true }` (workspace deps 已含 `lru = "0.16"`).
未引 `parking_lot` (用 `std::sync::Mutex` 替代, 0 新增).

---

## 2. 测试覆盖 (12 cases)

| ID | 用例 | 覆盖点 |
|---|---|---|
| t01 | new_defaults | 初始 stats + capacity |
| t02 | get_miss_returns_none | miss 路径 |
| t03 | put_get_hit | hit 路径 |
| t04 | miss_increments | misses++ |
| t05 | lru_eviction_order | 真 LRU (访问 p0 后, 淘汰 p1 不是 p0) |
| t06 | invalidate | 单 key 失效 |
| t07 | clear | 全清 |
| t08 | ttl_expiry | 10ms TTL + 20ms sleep → 过期 |
| t09 | streaming_query_calls_callback | 流式 |
| t10 | streaming_early_termination | early-exit |
| t11 | batch_query_shares_cache | 3 个同 query, 共享 cache |
| t12 | cached_unified_lru_facade | facade 端到端 |

累计 `cargo test -p apeireth-tool-codesearch --lib`: 89 passed (77 旧 + 12 新).

---

## 3. 0 触碰守门

- `cache.rs` (R210) 保留 — 只是删了未用的 `use std::path::PathBuf;` 测试 import
- `unified.rs` 0 改
- 3 不可变脊柱 0 触碰
- workspace.version 1.2.0 0 改
- `lru` crate 已存在 workspace deps, 不算新引

---

## 4. 路线意义

R193 → R201 → R202 → R203 → R210 → R213 把 tool-codesearch 从 grep 工具升级为:
- 14 MCP 工具
- 双 cache (R210 简化 LRU + R213 真 LRU)
- streaming + batch + facade 3 种 query 模式
- 89 测试覆盖

---

## 5. 下一步

- **R217** Kani 1 proof 演示 (2-3 hours, 高 ROI)
- **R215** evolution library_autonomy 加 Voyager API (2-3 days)
- **R214** relation petgraph 强化 (1 day)
- **R216** bus 三套通知 (R148 已做) 加测试覆盖 (1 day)
