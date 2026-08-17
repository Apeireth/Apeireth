# R210 unified query cache (TTL-bounded QueryCache + CachedUnifiedIntelligence facade)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R210
> **日期**: 2026-08-13
> **来源**: R202 unified facade + 主人指示 "全做全做全补弱 + 一体化优美"
> **状态**: 实施完成, 10/10 单测全过 (累计 77/77)

---

## 0. 动机

R202 unified query (6 维 code intelligence facade) 每次调用都重扫文件系统/调 ast-grep 进程. R210 加 TTL cache:
- 避免重复 (kind, pattern, path, lang) 的重扫
- 给 agent 端到端的 "这次问了, 下次不要再扫" 行为
- LRU 上限避免内存无限增长
- index_file 改动后失效 cache (粗粒度, 安全优先)

---

## 1. 设计

### 1.1 公共 API

```
pub struct QueryCacheStats { pub size: usize, pub hits: u64, pub misses: u64, pub evictions: u64 }

pub struct QueryCache { /* Mutex<HashMap<CacheKey, CacheEntry>>, ttl, max_entries */ }
impl QueryCache {
    pub fn new(ttl_ms: u64, max_entries: usize) -> Self;
    pub fn with_defaults() -> Self;  // 60s TTL, 1000 entries
    pub fn get(&self, q: &UnifiedQuery) -> Option<Vec<IntelligenceHit>>;
    pub fn put(&self, q: &UnifiedQuery, hits: Vec<IntelligenceHit>);
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool;
    pub fn clear(&self);
    pub fn stats(&self) -> QueryCacheStats;
}

pub struct CachedUnifiedIntelligence {
    cache: QueryCache,
    inner: UnifiedCodeIntelligence,
}
impl CachedUnifiedIntelligence {
    pub fn new(ttl_ms: u64, max_entries: usize) -> Self;
    pub fn with_defaults() -> Self;
    pub fn query(&self, q: &UnifiedQuery) -> Result<Vec<IntelligenceHit>, UnifiedError>;
    pub fn index_file(&self, path: &str) -> Result<(), UnifiedError>;  // 失效全部 cache
    pub fn stats(&self) -> QueryCacheStats;
    pub fn clear(&self);
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool;
}
```

### 1.2 Cache key

`(IntelligenceKind, pattern, path, Option<lang>)` 四元组哈希 — 4 维全部变化才视为不同 query.

### 1.3 LRU 策略

简化策略: `len >= max_entries` 时清掉 HashMap 前 1/4 keys (无排序, O(n) 但常数小).
- 默认 max=1000, 每次 evict 250 个
- 真实 LRU 留给后续 R213 streaming/batch 一起优化

### 1.4 Cache 失效

- **TTL 过期**: `Instant::now() >= expires` 时 get 返回 None + evictions++
- **index_file**: cache 全清 (保守, 因为索引改动影响所有 query 结果)
- **显式 invalidate(q)**: 单 key 失效

### 1.5 线程安全

- `Mutex<HashMap>` 包裹内部状态
- `AtomicU64` 计 hits/misses/evictions (无锁读)

---

## 2. 测试覆盖 (10 cases)

| ID | 用例 | 覆盖点 |
|---|---|---|
| t01 | new_defaults | 构造 + 初始 stats=0 |
| t02 | get_miss_returns_none | miss 路径 |
| t03 | put_get_hit | hit 路径 |
| t04 | miss_increments | misses++ |
| t05 | invalidate | 单 key 失效 |
| t06 | clear | 全清 |
| t07 | ttl_expiry | TTL 过期返回 None |
| t08 | lru_eviction | 超 max_entries 时清 25% |
| t09 | stats_atomic | AtomicU64 正确 |
| t10 | cached_unified_query_hit | facade 端到端 |

累计 `cargo test -p apeireth-tool-codesearch --lib`: 77 passed, 0 failed.

---

## 3. 0 触碰守门

- `apeireth-tool-codesearch::unified::UnifiedCodeIntelligence` 0 改 — QueryCache 是 wrapper, 不动 inner
- `unified::UnifiedQuery / IntelligenceHit / IntelligenceKind / UnifiedError` 0 改
- 3 不可变脊柱 (sovereignty self_disable / physical_multisig / verdict_cache) 0 触碰
- workspace.version 1.2.0 0 改
- 0 新增 Cargo.toml 依赖

---

## 4. 路线意义

R193 -> R201 -> R202 -> R203 -> R210 这条线已经把 `tool-codesearch` 从 "grep 工具" 升级成:
- 12 MCP 工具 (R201 ast_grep_search)
- 13 MCP 工具 (R203 unified_query)
- + QueryCache facade (R210)

距离 Tier 1 工具全栈完成还差 R149 `apeireth-tool-fetch` (UrlFetch + Tavily + AnySearch + VSearch + ...).

---

## 5. 下一步

按 R205/R207 路线:
- **R211** consciousness emotion engine 集成 Plutchik (1 day, 中-高 ROI)
- **R212** council deliberation checkpoint (3-5 days, 高 ROI)
- **R213** tool-codesearch streaming/batch distance + 真 LRU (1-2 days)
- **R217** Kani 1 proof 演示 (2-3 hours, 高 ROI)
