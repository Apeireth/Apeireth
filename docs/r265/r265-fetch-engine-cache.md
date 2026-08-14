# R265: FetchEngine TTL cache 集成

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 把已有 `FetchCache` (TTL HashMap) 真接到 FetchEngine.fetch(), 加 cache hit/miss metrics

---

## §1 背景

R149 创建了 `apeireth-tool-fetch::cache::FetchCache` (TTL HashMap, RwLock),
R231 加 rate limit, 但 `FetchEngine.fetch()` 完全没用 cache — 每次都走 HTTP.

R258 GitHub 调研 Tier A 候选: 统一 cache layer = ★★★★ (与 FetchEngine 集成是价值最高的子项).

---

## §2 设计

### 2.1 FetchEngine struct 加 cache 字段

```rust
pub struct FetchEngine {
    cfg: FetchConfig,
    rate_limiter: Option<Arc<Mutex<RateLimiter>>>,
    pub fetch_metrics: FetchMetrics,
    cache: Option<Arc<FetchCache>>,  // R265: None = disabled (默认, 向后兼容)
}
```

### 2.2 Builder methods (5)

```rust
pub fn with_cache(mut self) -> Self                  // 用 cfg.cache_ttl_ms
pub fn with_cache_ttl(mut self, ttl_ms: u64) -> Self
pub fn cache(&self) -> Option<&Arc<FetchCache>>      // 拿引用
pub fn cache_stats(&self) -> Option<CacheStats>      // 拿 stats (size/hits/misses/evictions)
pub fn cache_invalidate(&self, url: &str) -> bool    // 单 key invalidate
pub fn cache_clear(&self)                            // 清全部
```

### 2.3 fetch() 集成 (lookup before http + write on success)

```rust
pub async fn fetch(&self, req: &FetchRequest) -> FetchResult<FetchResponse> {
    let started = std::time::Instant::now();
    // ... URL validation ...
    
    // R265: cache lookup
    if let Some(cache) = &self.cache {
        if let Some(cached_json) = cache.get(&req.url) {
            match serde_json::from_str::<FetchResponse>(&cached_json) {
                Ok(resp) => {
                    self.fetch_metrics.record_cache_hit(started.elapsed().as_secs_f64() * 1000.0);
                    return Ok(resp);
                }
                Err(_) => cache.invalidate(&req.url),  // corrupted: invalidate + fall through
            }
        } else {
            self.fetch_metrics.record_cache_miss();
        }
    }
    // ... rate_limit check ...
    let fetcher = HttpFetcher::new();
    let result = fetcher.fetch(req, &self.cfg).await;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    match &result {
        Ok(resp) => {
            self.fetch_metrics.record_success(latency_ms);
            if let Some(cache) = &self.cache {
                if let Ok(json) = serde_json::to_string(resp) {
                    cache.put(&req.url, json);
                }
            }
        }
        Err(_) => self.fetch_metrics.record_error(latency_ms),
    }
    result
}
```

### 2.4 FetchMetrics 加 cache 字段

```rust
pub struct FetchMetrics {
    // ... 4 旧字段 ...
    pub fetch_cache_hits: AtomicU64,      // R265
    pub fetch_cache_misses: AtomicU64,    // R265
}

impl FetchMetrics {
    pub fn record_cache_hit(&self, latency_ms: f64);   // hits++, total++, max 更新
    pub fn record_cache_miss(&self);                    // misses++, 0 增加 total (实际 fetch 后再加)
}
```

**关键设计**: cache hit 也算 fetch_total + 算 latency (cache lookup 极快 <1ms), cache miss 只 +misses, 实际 fetch 走完后 record_success/error 才 +total.

### 2.5 metrics_text 输出

```
fetch_total=N fetch_errors_total=M fetch_latency_mean_ms=X.X fetch_latency_max_ms=Y fetch_cache_hits=H fetch_cache_misses=K
```

---

## §3 测试 (13 新 cases)

- r265_01..04: FetchMetrics cache counter (initial / hit++ / miss 不增加 total / text 含 cache fields)
- r265_05..07: builder (with_cache / with_cache_ttl / 默认无 cache)
- r265_08..09: invalidate / clear 在无 cache 时安全 (no panic)
- r265_10..12: 完整 put/get + invalidate + clear (含 stats 验证)
- r265_13: corrupted JSON deserialize 失败 → invalidate + 走 HTTP (fall-through)

**103 tests pass** (90 R231 旧 + 13 R265 新).

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借鉴 TTL cache 模式 (Redis-style), 自实现 0 引 moka/dashmap
- **S-2 实事求是**: 默认无 cache (向后兼容 R231 既有用法), opt-in via with_cache()
- **O-1 安全优先**: corrupted cache 自动 invalidate, 避免永久坏数据
- **O-3 干到底**: 5 builders + 6 accessors + 13 tests + 真接 fetch() 入口
- **O-5 不假装**: cache hit 也 record metrics (有 latency, 即便 0.x ms), miss 区分记录
