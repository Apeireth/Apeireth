# R231 — tool-fetch FetchEngine Rate Limit 集成

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R231
> **日期**: 2026-08-13
> **状态**: 1 commit, 5 测试 +5, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱" + "继续全做完"

## 1. 设计

R230 RateLimiter 已落地, 但未集成到 FetchEngine. R231 接入:

### 1.1 FetchEngine 改造

**新字段**:
```rust
pub struct FetchEngine {
    cfg: FetchConfig,
    rate_limiter: Option<Arc<Mutex<RateLimiter>>>,
}
```

**新方法**:
- `with_rate_limit()` — 启用 (默认 60 req/60s)
- `with_rate_limit_config(max, window_ms)` — 自定义
- `rate_limiter()` — 拿 Arc<Mutex<RateLimiter>> 引用

**fetch 路径改造**:
```rust
pub async fn fetch(&self, req: &FetchRequest) -> FetchResult<FetchResponse> {
    if req.url.trim().is_empty() { return Err(FetchError::EmptyUrl); }
    let parsed = url::Url::parse(&req.url).map_err(|_| FetchError::InvalidUrl(req.url.clone()))?;
    // R231: per-host rate limit
    if let Some(rl) = &self.rate_limiter {
        let host = parsed.host_str().unwrap_or("").to_string();
        if !host.is_empty() {
            if !rl.lock().check(&host) {
                return Err(FetchError::RateLimited(host));
            }
            rl.lock().record(&host);
        }
    }
    let fetcher = HttpFetcher::new();
    fetcher.fetch(req, &self.cfg).await
}
```

### 1.2 不假装

- record 在 HTTP 调用前 — 即使后续失败也占用配额 (防 retry 风暴)
- 超限返语义化错误 `RateLimited(host)`, 不假装"已经发了"
- 0 触碰既有 FetchEngine::fetch (除新加 check + record)

## 2. 测试 (5 cases)

| 测试 | 验证 |
|---|---|
| r231_01_engine_no_rate_limit_by_default | new() 不启用 rate limiter |
| r231_02_engine_with_rate_limit_enabled | with_rate_limit() 启用 |
| r231_03_engine_with_rate_limit_custom | with_rate_limit_config 自定义参数 |
| r231_04_engine_record_via_rl_accessor | 通过 rate_limiter() accessor record |
| r231_05_rate_limited_error_includes_host | FetchError::RateLimited 显示 host |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 67 → 72 (+5)

## 4. 战区意义

tool-fetch 完整 rate limit 闭环:
- 配置 (with_rate_limit_config)
- 启用 (with_rate_limit)
- 真实 gate (fetch 自动 check)
- 错误语义化 (RateLimited(host))

## 5. 下一步候选

- **R232** council collect_opinions (per-advisor visibility, 不经 synthesis)
- **R233** consciousness temporal emotion decay per-event
- **R234** tool-codesearch ast-grep in-process (no CLI dep)
- **R235+** protocol Arrow / DataFusion (大项目, 最后)