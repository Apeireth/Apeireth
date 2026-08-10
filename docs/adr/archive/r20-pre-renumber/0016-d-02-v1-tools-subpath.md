# ADR 0016: D-02 6 工具各 1 URL 子路径 `/v1/tools/{name}/invoke`

> **状态**: 🟢 Accepted (主人 2026-08-05 20:53 拍板 A 推荐)
> **commit 锚**: `r20-stage-2-3-prep-2026-08-05.md` §3.4 + `crates/apeireth-api/src/server.rs` 实施
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth v1 6 工具 (calendar / message / contact / task / search / drive) 走 HTTP 端点。

**问题**: 每个工具 1 个独立 URL, 还是 6 个工具走同一 URL 模板?

### 选项对比

| 选项 | 描述 | 路由表 | 中间件 |
|---|---|---|---|
| **A** (本 ADR 拍板) | 统一子路径 `/v1/tools/{name}/invoke` | 1 行 | 1 次挂载 |
| B | 每工具 1 路径 `/v1/{tool}/{action}` | 6 行 | 6 次挂载 |
| C | 1 通用路径 `/v1/invoke` + body 里 tool+action | 1 行 | 1 次挂载 |
| D | GraphQL 单一端点 | 1 行 | 复杂 |

---

## 2. 决策 (Decision)

**6 工具走同一 URL 模板: `POST /v1/tools/{name}/invoke`**

### 2.1 URL 模板

```
POST /v1/tools/{name}/invoke
Content-Type: application/json
Authorization: Bearer <token>

{
  "tool": "calendar",
  "action": "list_events",
  "params": { ... },
  "idempotency_key": "...",
  "trace_id": "..."
}
```

| 工具 | 路径 |
|---|---|
| calendar | `POST /v1/tools/calendar/invoke` |
| message | `POST /v1/tools/message/invoke` |
| contact | `POST /v1/tools/contact/invoke` |
| task | `POST /v1/tools/task/invoke` |
| search | `POST /v1/tools/search/invoke` |
| drive | `POST /v1/tools/drive/invoke` |

### 2.2 路由实现

```rust
let app = Router::new()
    .route("/v1/tools/:name/invoke", post(invoke_tool))
    .layer(auth_layer)
    .layer(rate_limit_layer)
    .layer(trace_layer);
```

### 2.3 工具分发

`{name}` 路径参数 → 查 `apeireth-tool-registry` → 找到 tool handler → 调

```rust
async fn invoke_tool(
    Path(name): Path<String>,
    Extension(registry): Extension<Arc<ToolRegistry>>,
    Json(req): Json<InvokeRequest>,
) -> Result<Json<InvokeResponse>, ApiError> {
    let tool = registry.get(&name)
        .ok_or(ApiError::ToolNotFound(name))?;
    
    // 校验 tool 白名单（编译期 hardcode）
    if !WHITELIST.contains(&name.as_str()) {
        return Err(ApiError::ToolNotFound(name));
    }
    
    // 校验 action 在 tool 内合法
    tool.validate_action(&req.action)?;
    
    // JSON schema 校验 params
    tool.validate_params(&req.action, &req.params)?;
    
    // 调 tool 实现
    let result = tool.invoke(&req.action, req.params).await?;
    
    Ok(Json(InvokeResponse { result, metadata: ... }))
}
```

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **路由表 1 行**: vs 选项 B 的 6 行
- ✅ **中间件 1 次挂载**: auth / rate-limit / trace 一次挂
- ✅ **跟 R20 §3.3 10 REST 端点模式一致**: 跟 `/v1/organs/{name}/invoke` 镜像
- ✅ **URL 自描述**: `tools/calendar/invoke` 一眼看出调什么
- ✅ **工具注册表 vs 路由表双源统一**: 1 处注册, 1 处路由
- ✅ **HTTP cache 友好**: GET 类 action 可缓存 (虽然当前都 POST)

### 3.2 负面

- ⚠️ **路径参数 vs body 重复**: URL `{name}` 和 body `tool` 字段都指定, 重复 (但 body 是 source of truth, URL 仅作 routing)
- ⚠️ **HTTP method 限制**: 当前都 POST, 未来 GET 工具 (如简单 lookup) 不适合 POST
- ⚠️ **REST 风格减弱**: 严格 RESTful 应该 `/v1/tools/calendar/events` 而不是 `/v1/tools/calendar/invoke`

### 3.3 风险

- 未来 GET-only 工具 (如简单查询) 需要单独路由, 但 R21 估补
- 严格 RESTful 风格偏离, 但 RPC-over-HTTP 业界也常见 (AWS API, Google Cloud API)

---

## 4. 备选 (Alternatives Considered)

### A. (本决策) 统一子路径
- 优点: 简洁 + 跟 organs 模式一致
- 缺点: REST 风格减弱
- 拍板: 主人 2026-08-05 20:53 选 A

### B. 每工具 1 路径
- 优点: 严格 RESTful
- 缺点: 6 行路由 + 6 次中间件挂载, 维护成本高
- 否决: 1 工具 1 路由不必要, 工具数还会增加

### C. 1 通用路径 + body dispatch
- 优点: 最简洁
- 缺点: URL 不自描述, 难用 curl 测试, OpenAPI 生成复杂
- 否决: 主人拍板 A

### D. GraphQL
- 优点: 客户端灵活
- 缺点: 跟现有 REST API 不一致, 实施复杂
- 否决: 1.0 release 不引入新范式, R22+ 估补

### E. gRPC
- 优点: 强类型, 流式原生
- 缺点: 浏览器支持差 (需 grpc-web), 跟 REST 体系割裂
- 否决: 跟 v1 API 不一致, WS 已覆盖流式

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: 跟 R20 §3.3 `/v1/organs/{name}/invoke` 模式一致
- ✅ **S-2 实事求是**: 6 工具简单, 不需要 6 路由
- ✅ **O-2 用户看结果不看哲学**: 用户只看 URL 通不通
- ✅ **O-3 信息密度"高"**: 1 路由 vs 6 路由
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝每工具 1 路由
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 1 路由已 commit
- ✅ **编译期 hardcode**: 6 工具白名单 `WHITELIST` 编译期固定
- ✅ **不改 LOCKED**: API 协议层 LOCKED
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 tool registry
- ✅ **不重复造轮子**: 沿用 organs 模式
- ✅ **诚实标缺**: REST 风格减弱已说明

---

## 7. 引用

- 决策 ID 体系: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-02)
- 蓝图: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` §3.4
- 实施: `crates/apeireth-api/src/server.rs`
- 文档: [`docs/api/v1-tools.md`](../api/v1-tools.md)
