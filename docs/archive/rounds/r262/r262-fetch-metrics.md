# R262: FetchMetrics (stdlib-only OTel metrics for FetchEngine.fetch)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: FetchEngine.fetch() 加 4 atomic counter (stdlib-only, 0 引外部 dep)

---

## §1 范围

**新增**:
- `FetchMetrics` struct: 4 atomic counters (fetch_total / fetch_errors_total / fetch_total_duration_ms / fetch_latency_max_ms)
- `FetchEngine.fetch_metrics: FetchMetrics` pub 字段
- `new() / with_config()` 初始化 fetch_metrics
- `fetch()` 入口 timing (validation + rate limit + http), 成功 / 错误 / 5 个早退点都 record

**0 触碰**:
- HttpFetcher / FetchRequest / FetchResponse / FetchConfig / rate_limit (R231)
- FetchCache (R265 单独接)

---

## §2 FetchMetrics API

```rust
pub struct FetchMetrics {
    pub fetch_total: AtomicU64,
    pub fetch_errors_total: AtomicU64,
    pub fetch_total_duration_ms: AtomicU64,
    pub fetch_latency_max_ms: AtomicI64,  // -1 = unset
}

impl FetchMetrics {
    pub fn new() -> Self;
    pub fn record_success(&self, latency_ms: f64);
    pub fn record_error(&self, latency_ms: f64);
    pub fn metrics_text(&self) -> String;  // key=value 格式, 非 Prometheus
}
```

**0 引外部 dep** — 全 std::sync::atomic.

---

## §3 fetch() 集成

```rust
pub async fn fetch(&self, req: &FetchRequest) -> FetchResult<FetchResponse> {
    let started = std::time::Instant::now();
    if req.url.trim().is_empty() {
        self.fetch_metrics.record_error(started.elapsed().as_secs_f64() * 1000.0);
        return Err(FetchError::EmptyUrl);
    }
    let parsed = match url::Url::parse(&req.url) {
        Ok(u) => u,
        Err(_) => {
            self.fetch_metrics.record_error(started.elapsed().as_secs_f64() * 1000.0);
            return Err(FetchError::InvalidUrl(req.url.clone()));
        }
    };
    // ... rate_limit check + record_error on early exits ...
    let fetcher = HttpFetcher::new();
    let result = fetcher.fetch(req, &self.cfg).await;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    match &result {
        Ok(_) => self.fetch_metrics.record_success(latency_ms),
        Err(_) => self.fetch_metrics.record_error(latency_ms),
    }
    result
}
```

**关键设计**: 每个 return Err 路径都 record_error, 成功路径 record_success. mean latency = total_duration / total.

---

## §4 测试 (8 cases, 后扩 13 cache tests)

- r262_01..04: FetchMetrics 单元 (initial zero / record_success / record_error / metrics_text)
- r262_05..07: FetchEngine 集成 (empty URL / success example.com / invalid URL)
- r262_08: metrics_text 完整 format

**103 tests pass** (90 R231 旧 + 13 R265 cache 后续; R262 自身贡献 8).

---

## §5 主哲学锚对齐

- **S-1 北极星**: 借鉴 OTel counter/histogram 概念, 自实现保持 0 引外部 dep
- **S-2 实事求是**: stdlib-only, AtomicU64/AtomicI64 (不需要 metrics crate)
- **O-1 安全优先**: 5 个 early-return 路径都 record, 不漏报
- **O-3 干到底**: fetch_metrics pub 字段暴露, caller 可观测
- **O-5 不假装**: latency 是 Instant::now() 真实 wallclock, 不是估算
