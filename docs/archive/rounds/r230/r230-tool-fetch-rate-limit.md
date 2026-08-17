# R230 — apeireth-tool-fetch Per-host Rate Limit

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R230
> **日期**: 2026-08-13
> **状态**: 1 commit, 9 测试 +9, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱 + 一体化优美" + "继续全做完"

## 1. 设计

apeireth-tool-fetch R149 已整合 7 个 fetch 类工具 (UrlFetch / TavilySearch /
AnySearch / FlashDeepSearch / BilibiliFetch / AnimeFinder / WebReadFile).
但缺 per-host rate limit, 调用方 (AnySearch / Bilibili / Bangumi 等外部 API)
可能因为高频调用 ban 掉 IP / API key.

R230 落地 RateLimiter: per-host sliding window.

### 1.1 RateLimiter

```rust
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    history: HashMap<String, VecDeque<Instant>>,
}
```

**字段**:
- `max_requests` — 窗口内最大请求数 (默认 60)
- `window` — 窗口时长 (默认 60s)
- `history` — host → 时间戳 deque (sliding)

**9 工具方法**:
- `check(host) -> bool` — 此刻 host 是否可发
- `record(host)` — 记录一次
- `wait_time(host) -> Option<Duration>` — 距离下次允许的等待
- `count(host) -> usize` — 当前窗口内请求数
- `hosts() -> usize` — 跟踪的 host 数
- `clear() / clear_host(host)`

**默认构造**: 60 req/60s, 适用大多数 API.
**自定义**: `with_limit(max, window)`.

### 1.2 不引外部 dep

借鉴 token bucket + sliding window, std + VecDeque + Instant.
不引 `governor` 等 crate — 0 引外部 dep.
单进程内存计数, 跨进程不共享 (per-host 真实限速应放在 HTTP client 层面).

## 2. 测试 (9 cases)

| 测试 | 验证 |
|---|---|
| new_limiter_empty | 默认构造空, check 返 true |
| record_and_check | 记录 N 次后第 N+1 check 返 false |
| different_hosts_independent | 不同 host 独立计数 |
| wait_time_none_when_allowed | 未达限时 wait_time None |
| wait_time_when_at_limit | 达限后 wait_time Some(Duration) |
| sliding_window_expires | 窗口过期后允许 |
| clear_resets | clear 后可重新发 |
| clear_host_removes_single | 单 host clear 不影响其他 |
| max_requests_constant | 字段暴露给测试 |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 58 → 67 (+9)

## 4. 战区意义

apeireth-tool-fetch 补 per-host rate limit, 适用:
- **API 调用限速** — AnySearch / Bilibili / Tavily / Bangumi 等外部 API
- **防止 ban** — 高频请求触发反爬
- **测试场景** — 用 record + check 避免 sleep 实际等待

## 5. 下一步候选

- **R231** RateLimiter 集成到 FetchEngine (fetch 前 check, 超限 await wait_time)
- **R232** council collect_opinions API (per-advisor visibility)
- **R233** consciousness temporal emotion decay per-event
- **R234** tool-codesearch ast-grep in-process