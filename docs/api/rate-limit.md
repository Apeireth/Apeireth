# Apeireth API 限流（token bucket via `apeireth-constraint`）

> **性质**: v1 API 限流设计 + 3 档 token bucket
> **依据**: `crates/apeireth-constraint/src/` 实现 + D-04 主人拍板
> **最后更新**: 2026-08-05

---

## 1. 限流目标

| 目标 | 阈值 | 触发动作 |
|---|---|---|
| **保护服务稳定** | 全局 qps 上限 | 429 + `RATE_LIMIT_GLOBAL` |
| **公平分配** | 用户级 qps | 429 + `RATE_LIMIT_PER_USER` |
| **工具隔离** | 工具级 qps | 429 + `RATE_LIMIT_PER_TOOL` |
| **突发友好** | burst bucket | 429 + `RATE_LIMIT_BURST_EXHAUSTED` |

---

## 2. 3 档 token bucket

| 档位 | 容量（burst） | 补充速率 | 适用 |
|---|---|---|---|
| **Global** | 1000 req | 500 req/s | 全局入口 |
| **Per-User** | 100 req | 20 req/s | 单用户 |
| **Per-Tool** | 50 req | 10 req/s | 单工具 |

**算法**: token bucket（golang.org/x/time/rate 同款，Rust 端口在 `apeireth-constraint`）

```
Bucket(capacity, refill_rate):
  tokens: 初始 = capacity
  last_refill: now()
  
  take(n):
    elapsed = now() - last_refill
    tokens = min(capacity, tokens + elapsed * refill_rate)
    if tokens >= n:
      tokens -= n
      return ALLOW
    else:
      return DENY(retry_after = (n - tokens) / refill_rate)
```

---

## 3. 响应头

每个响应带限流相关 header：

| Header | 说明 | 示例 |
|---|---|---|
| `X-RateLimit-Limit` | bucket 容量 | `100` |
| `X-RateLimit-Remaining` | 当前剩余 token | `73` |
| `X-RateLimit-Reset` | 桶完全恢复 Unix epoch | `1754438460` |
| `Retry-After` | 429 时建议等待秒数 | `2` |

**429 响应示例**:
```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1754438460
Retry-After: 2
Content-Type: application/json

{
  "error": {
    "code": "RATE_LIMIT_PER_USER",
    "message": "User qps exceeded (20 req/s)",
    "details": { "user_id": "user-uuid", "limit": 20, "current": 25 },
    "request_id": "req-..."
  }
}
```

---

## 4. 实现（per D-04）

### 4.1 全局限流

```rust
// crates/apeireth-api/src/server.rs
let global_limiter = ConstraintLayer::new(GlobalConfig {
    capacity: 1000,
    refill_rate: 500.0,
    scope: ConstraintScope::Global,
});

let app = Router::new()
    .route("/v1/tools/:name/invoke", post(invoke_tool))
    .layer(global_limiter);
```

### 4.2 用户级限流

```rust
let user_limiter = ConstraintLayer::new(PerUserConfig {
    capacity: 100,
    refill_rate: 20.0,
    scope: ConstraintScope::PerUser,  // key = JWT sub
});
```

### 4.3 工具级限流

```rust
let tool_limiter = ConstraintLayer::new(PerToolConfig {
    capacity: 50,
    refill_rate: 10.0,
    scope: ConstraintScope::PerTool,  // key = tool name
});
```

### 4.4 三层组合

```rust
// 三层串联：先全局 → 再用户 → 再工具
let app = Router::new()
    .route("/v1/tools/:name/invoke", post(invoke_tool))
    .layer(tool_limiter)
    .layer(user_limiter)
    .layer(global_limiter);
```

任一层 deny → 立即 429，不消耗下一层 token。

---

## 5. 工具级详细配额

| 工具 | capacity | refill | 备注 |
|---|---|---|---|
| calendar | 50 | 10/s | 高频读/低频写 |
| message | 20 | 5/s | 发送成本高（上游 SMTP） |
| contact | 100 | 30/s | 读多写少 |
| task | 50 | 10/s | 平衡 |
| search | 100 | 30/s | 查询密集 |
| drive | 30 | 5/s | 带宽敏感 |

配置：`crates/apeireth-constraint/src/tools.toml` 编译期 hardcode。

---

## 6. WebSocket 限流

WS 连接不走 HTTP 层限流，单独按连接计：

| 维度 | 阈值 | 触发 |
|---|---|---|
| 单连接 msg/s | 50 msg/s | `RATE_LIMIT_PER_TOOL` |
| 单连接 frame/s | 200 frame/s | `PROTOCOL_INVALID_FRAME` |
| 全局活跃连接 | 5000 conn | `RATE_LIMIT_GLOBAL` |
| 连接时长 | 24 h | 强制断连 + `PROTOCOL_TIMEOUT` |

---

## 7. 失败策略

**Fail-open**（限流系统不可用时）：
- `apeireth-constraint` 健康检查失败 → 自动降级为 fail-open
- 记录 metric `apeireth_rate_limit_fail_open_total`
- 不阻塞业务（trade-off: 临时过载风险）

---

## 8. 指标（per 1.0 release #8 observability）

| Prometheus 指标 | 类型 | 标签 |
|---|---|---|
| `apeireth_rate_limit_decisions_total` | counter | `scope`, `decision` (allow/deny) |
| `apeireth_rate_limit_tokens_remaining` | gauge | `scope` |
| `apeireth_rate_limit_burst_exhausted_total` | counter | `scope` |
| `apeireth_rate_limit_fail_open_total` | counter | — |

---

## 9. 不修改承诺

- ✅ 编译期 hardcode：3 档 bucket 配置在 `tools.toml` 编译期固定
- ✅ 不改 LOCKED：限流算法沿用 `apeireth-constraint` 既有设计
- ✅ 不假装已实现：所有阈值都标实际值

---

## 10. 相关

- 实现: `crates/apeireth-constraint/` (token bucket)
- API 集成: `crates/apeireth-api/src/server.rs`
- 决策: [`docs/adr/0015-d-04-rate-limit-token-bucket.md`](../adr/0015-d-04-rate-limit-token-bucket.md) (D-04 拍板)
- 1.0 release #8: [observability 12 项 checklist](../1.0-release/checklist.md#8-observability)
