# Apeireth v1 Observability API（3 端点）

> **依据**: `crates/apeireth-observability/src/` + `crates/apeireth-api/src/health.rs` 实际实现
> **最后更新**: 2026-08-05
> **关联**: 1.0 release 12 项 checklist #8（observability）

---

## 1. 3 端点总览

| 端点 | 路径 | 用途 | 鉴权 |
|---|---|---|---|
| **metrics** | `/metrics` | Prometheus 8 指标 | 🟡 可选（IP 白名单） |
| **health** | `/health` | Liveness probe | ❌ 无 |
| **status** | `/v1/status` | 详细运行时状态 | ✅ Bearer |

---

## 2. `/metrics`（Prometheus）

**方法**: `GET /metrics`

**响应**: Prometheus text format（200 OK）

```
# HELP apeireth_qps_total Total requests per second
# TYPE apeireth_qps_total counter
apeireth_qps_total{tool="calendar",action="list_events",status="200"} 1234

# HELP apeireth_request_duration_seconds Request duration
# TYPE apeireth_request_duration_seconds histogram
apeireth_request_duration_seconds_bucket{le="0.005"} 100
apeireth_request_duration_seconds_bucket{le="0.01"} 200
...

# HELP apeireth_active_websocket_connections Active WebSocket connections
# TYPE apeireth_active_websocket_connections gauge
apeireth_active_websocket_connections 42

# HELP apeireth_error_rate_total Error responses
# TYPE apeireth_error_rate_total counter
apeireth_error_rate_total{code="TOOL_NOT_FOUND"} 5

# HELP apeireth_audit_log_size_bytes Audit log size
# TYPE apeireth_audit_log_size_bytes gauge
apeireth_audit_log_size_bytes 1.2e8

# HELP apeireth_asi_score ASI score (24-dim)
# TYPE apeireth_asi_score gauge
apeireth_asi_score 0.85

# HELP apeireth_llm_tokens_total LLM token usage
# TYPE apeireth_llm_tokens_total counter
apeireth_llm_tokens_total{provider="claude-code",model="sonnet"} 1234567

# HELP apeireth_db_pool_size Database connection pool
# TYPE apeireth_db_pool_size gauge
apeireth_db_pool_size 8
```

**8 指标**（per 1.0 release #8）:
1. `apeireth_qps_total` — 请求 qps
2. `apeireth_request_duration_seconds` — 请求时长 histogram
3. `apeireth_active_websocket_connections` — 活跃 WS 连接
4. `apeireth_error_rate_total` — 错误数（按 code）
5. `apeireth_audit_log_size_bytes` — 审计日志大小
6. `apeireth_asi_score` — ASI 评分（24 维）
7. `apeireth_llm_tokens_total` — LLM token 用量
8. `apeireth_db_pool_size` — DB 连接池

**刮取配置**（`prometheus.yml`）:
```yaml
scrape_configs:
  - job_name: 'apeireth'
    scrape_interval: 15s
    static_configs:
      - targets: ['apeireth-api:8080']
```

**Grafana dashboard**: `deploy/grafana/apeireth-dashboard.json`

---

## 3. `/health`（Liveness）

**方法**: `GET /health`

**鉴权**: ❌ 无

**响应** (200):
```json
{
  "status": "ok",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "started_at": "2026-08-04T00:00:00Z"
}
```

**响应** (503, 启动期):
```json
{
  "status": "starting",
  "checks": {
    "database": "ok",
    "keyring": "ok",
    "providers": "loading"
  }
}
```

**K8s 配**:
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10
```

---

## 4. `/v1/status`（详细状态）

**方法**: `GET /v1/status`

**鉴权**: ✅ Bearer (admin scope)

**响应**:
```json
{
  "version": "1.0.0",
  "build": {
    "commit": "8a643778",
    "rust_version": "1.80",
    "build_time": "2026-08-05T10:00:00Z"
  },
  "runtime": {
    "uptime_seconds": 86400,
    "active_websocket_connections": 42,
    "active_http_connections": 8,
    "memory_rss_mb": 256
  },
  "tools": {
    "calendar": { "status": "ok", "upstream": "google_calendar" },
    "message": { "status": "ok", "upstream": "smtp" },
    "contact": { "status": "ok", "upstream": "local" },
    "task": { "status": "ok", "upstream": "local" },
    "search": { "status": "ok", "upstream": "tantivy" },
    "drive": { "status": "ok", "upstream": "minio" }
  },
  "providers": {
    "claude-code": { "status": "ok", "model_count": 3 },
    "gemini-cli": { "status": "stub" },
    "codex": { "status": "stub" },
    "copilot": { "status": "stub" },
    "opencode": { "status": "stub" }
  },
  "database": {
    "engine": "sqlite",
    "version": "3.46",
    "size_mb": 256,
    "tables": 42
  },
  "asi_score": 0.85,
  "release_tag": "v1.0.0-pending"
}
```

---

## 5. Tracing 集成

所有端点自动注入 OpenTelemetry trace：
- `traceparent` header（W3C Trace Context）
- 服务端用 `apeireth-observability::tracing_integration` 包装

**关联查询**:
```
GET /v1/tools/calendar/invoke
X-Trace-Id: 7c5a3b2e9f8d4a1b

(服务端 log)
2026-08-05 15:00:00 INFO request_id=req-7c5a3b2e trace_id=7c5a3b2e tool=calendar action=list_events duration=142ms
```

---

## 6. 不假装

- ✅ `/metrics` 真接 Prometheus exporter
- ✅ `/health` 真接 liveness
- ✅ `/v1/status` 真接（admin scope）
- ✅ 8 指标全部实装（per 1.0 release #8）

---

## 7. 相关

- 实现: `crates/apeireth-observability/src/{metrics,tracing_integration,logging,health}.rs`
- Grafana: `deploy/grafana/apeireth-dashboard.json`
- 决策: 1.0 release checklist #8
