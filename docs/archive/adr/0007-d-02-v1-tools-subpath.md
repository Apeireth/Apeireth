# ADR 0007: D-02 6 工具各 1 URL 子路径 `/v1/tools/{name}/invoke`

> **状态**: 🟢 Accepted (主人 2026-08-05 20:53 拍板 A 推荐)
> **commit 锚**: `b2b9ec8e` (feat(api): R20 阶段 2 — 6 工具 v1 子路径 endpoint) + `crates/apeireth-api/src/server.rs` 实施
> **最后更新**: 2026-08-05 22:13
> **原版 ADR**: [`archive/r20-pre-renumber/0016-d-02-v1-tools-subpath.md`](archive/r20-pre-renumber/0016-d-02-v1-tools-subpath.md) (v0; v1 本 ADR 引用新编号 0006/0008)

---

## 1. 背景 (Context)

Apeireth v1 6 工具 (calendar / message / contact / task / search / drive) 走 HTTP 端点。

**问题**: 每个工具 1 个独立 URL, 还是 6 个工具走同一 URL 模板?

### 1.1 选项对比

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

### 2.2 6 工具路由表 (1 行, 1 次挂载)

| 工具 | 路径 |
|---|---|
| calendar | `POST /v1/tools/calendar/invoke` |
| message | `POST /v1/tools/message/invoke` |
| contact | `POST /v1/tools/contact/invoke` |
| task | `POST /v1/tools/task/invoke` |
| search | `POST /v1/tools/search/invoke` |
| drive | `POST /v1/tools/drive/invoke` |

### 2.3 axum 实施 (1 行, 1 次挂载)

```rust
// crates/apeireth-api/src/server.rs (commit b2b9ec8e, 实施)
.route("/v1/tools/:name/invoke", post(tools_invoke))  // 1 行

async fn tools_invoke(
    State(state): State<AppState>,
    Path(name): Path<String>,                     // 工具名: calendar / message / ...
    Extension(auth): Extension<AuthContext>,      // 鉴权 (per D-03 链接 token)
    Json(req): Json<ToolInvokeRequest>,           // body
) -> Result<Json<ToolInvokeResponse>, ApiError> {
    // 1. 限流 (per D-04 token bucket)
    state.rate_limiter.check(&auth, &name).await?;
    // 2. 派发到 6 工具 (per [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md))
    let result = state.tools.invoke(&name, &req).await?;
    // 3. 审计 (5 守门 #4)
    state.audit.log_invoke(&auth, &name, &req, &result).await?;
    Ok(Json(result))
}
```

### 2.4 选 A (本决策) vs 选 B/C/D 对比

| 维度 | A (本决策) | B 每工具 1 路径 | C 1 通用路径 | D GraphQL |
|---|---|---|---|---|
| 路由表 | 1 行 | 6 行 | 1 行 | 1 行 |
| 中间件挂载 | 1 次 | 6 次 | 1 次 | 复杂 (schema + resolver) |
| 工具扩展 | 加 1 行 enum | 加 6 行路由 | 加 1 行 enum | 加 1 个 GraphQL type |
| 团队学习成本 | 1 URL 模板 | 6 URL 模板 | 1 URL 模板 | GraphQL 概念 |
| 类型安全 | ✅ path param | ✅ path param | ❌ body 字符串 | ✅ schema |
| RESTful | ✅ 是 | ✅ 是 | ⚠️ 弱 | ❌ 否 |
| 业界常见 | ✅ GitHub API `/repos/:owner/:repo/issues/:number` | ⚠️ 偶发 | ⚠️ 弱 | ⚠️ GraphQL 1 派 |

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **1 行路由表**: 加工具 = 加 enum, 不加路由
- ✅ **1 次中间件挂载**: 鉴权 / 限流 / 审计 1 处挂载
- ✅ **RESTful**: 符合 REST 设计原则
- ✅ **类型安全**: path param + body schema 双层校验
- ✅ **团队学习成本低**: 1 URL 模板, 1 body schema
- ✅ **TUI / Tauri / 第三方客户端复用**: 1 套 HTTP client (per [0011-tui-as-thin-client.md](0011-tui-as-thin-client.md))

### 3.2 负面

- ⚠️ **URL 含 path param**: 需 URL 编码 (calendar 名字简单, 无问题)
- ⚠️ **action 在 body**: 不如 URL 直观, 但鉴权 / 限流 / 审计 统一
- ⚠️ **Swagger / OpenAPI 文档需动态生成**: 6 工具 1 模板, schema 6 套

### 3.3 风险

- 6 工具 action 命名冲突: 1.0 release 约定 `tool_action` 格式 (e.g. `list_events` / `send_message`), R21 估补统一 schema 校验
- 路径爆炸: 6 工具估补 50+ action, URL 仍 1 模板, 风险低

---

## 4. 备选 (Alternatives Considered)

### A. 统一子路径 `/v1/tools/{name}/invoke` (本决策)
- 优点: 1 行 + 1 次挂载 + RESTful + 类型安全
- 拍板: 主人 20:53 按 A 推荐拍板, commit `b2b9ec8e` 落地

### B. 每工具 1 路径 `/v1/{tool}/{action}`
- 优点: URL 直观
- 否决: 6 行路由表, 6 次中间件挂载, 加工具 = 6 处改; 团队学习成本 × 6

### C. 1 通用路径 `/v1/invoke` + body 里 tool+action
- 优点: 1 行
- 否决: action 在 body 字符串, 类型安全弱; Swagger 难生成; 业界少见

### D. GraphQL 单一端点
- 优点: 1 行
- 否决: GraphQL 概念 + schema + resolver 复杂; 1.0 release 不必; 团队学习成本高

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: RESTful 业界标准; GitHub API `/repos/:owner/:repo/issues/:number` 同模式
- ✅ **S-2 实事求是**: 6 工具 + 1 模板已 commit `b2b9ec8e`, 不凭想象
- ✅ **O-2 用户看结果不看哲学**: 用户只看 6 工具能不能用, 不看 URL 模板
- ✅ **O-3 信息密度"高"**: §2.1 URL 模板 + §2.2 6 工具路由表 + §2.4 4 选项对比
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"每工具 1 路径"重复, 拒绝"GraphQL 复杂"
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 6 工具 endpoint 已 commit `b2b9ec8e` 落地, 写操作 R21 估补 (per [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md))
- ✅ **编译期 hardcode**: 6 工具 enum 编译期固定 (per `apeireth-tools` LOCKED)
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰 (`apeireth-tools` 仅复用)
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 6 工具 endpoint 自建, 0 引 NewAPI
- ✅ **不重复造轮子**: 沿用 axum (router + middleware) / serde / tokio 业界标准
- ✅ **诚实标缺**: 6 工具写操作 R21 估补 (per [0006](0006-d-01-tool-endpoint-real.md) §2.2 501 stub 范围); Swagger 动态生成 R21 估补

---

## 7. 引用

- 决策 ID 体系: [`docs/stage4/pending-decisions-overview-2026-08-05.md`](../../docs/stage4/pending-decisions-overview-2026-08-05.md) (D-02)
- D-01 6 工具真接: [`0006-d-01-tool-endpoint-real.md`](0006-d-01-tool-endpoint-real.md)
- D-03 WS 鉴权: [`docs/adr/0014-d-03-ws-auth-link-token.md`](0014-d-03-ws-auth-link-token.md)
- D-04 限流: [`docs/adr/0015-d-04-rate-limit-token-bucket.md`](0015-d-04-rate-limit-token-bucket.md)
- 6 工具 endpoint 实施: `crates/apeireth-api/src/server.rs` + `crates/apeireth-api/src/v1_tools/` (commit `b2b9ec8e`)
- TUI 集成: [`0011-tui-as-thin-client.md`](0011-tui-as-thin-client.md) (TUI 调 6 工具 via HTTP)
- 1.0 release 总览: [`0001-apeireth-rust-1.0.md`](0001-apeireth-rust-1.0.md)
- 1:1 翻译原则: [`0012-spectrAI-reverse-engineering.md`](0012-spectrAI-reverse-engineering.md)
- 原版 ADR v0: [`archive/r20-pre-renumber/0016-d-02-v1-tools-subpath.md`](archive/r20-pre-renumber/0016-d-02-v1-tools-subpath.md)

---

## 8. 附录

### 8.1 URL 模板细节 + idempotency_key 用法

```
POST /v1/tools/{name}/invoke
  ↓
Path param: {name} = calendar | message | contact | task | search | drive
  ↓
Headers:
  - Authorization: Bearer <token>           # 鉴权 (per D-03 链接 token)
  - Content-Type: application/json
  - X-Idempotency-Key: <uuid>              # 幂等键, 写操作必填
  - X-Trace-Id: <uuid>                     # 链路追踪, 可选
  ↓
Body:
{
  "tool": "<name>",                        # 必填, 跟 path param 一致
  "action": "<tool_action>",               # 必填, e.g. "list_events" / "send"
  "params": { ... },                       # 工具参数, 必填
  "idempotency_key": "<uuid>",             # 写操作必填
  "trace_id": "<uuid>"                     # 链路追踪, 可选
}
```

### 8.2 6 工具 action 命名约定

| 工具 | 读 actions | 写 actions (1.0 release 状态) |
|---|---|---|
| calendar | list_events, get_event | create_event (501), update_event (501), delete_event (501) |
| message | list_messages, get_message | send_message ✅, mark_read (501) |
| contact | lookup_contact, list_contacts | create_contact (501), update_contact (501), delete_contact (501) |
| task | list_tasks, get_task | create_task (501), update_task (501), complete_task (501), delete_task (501) |
| search | query, index, delete | (全真接) |
| drive | list_files, get_metadata, download | upload, delete |

### 8.3 4 选项对比细节

| 维度 | A `/v1/tools/{name}/invoke` (本决策) | B `/v1/{tool}/{action}` | C `/v1/invoke` | D GraphQL |
|---|---|---|---|---|
| URL 数量 | 1 模板 | 6 路径 | 1 路径 | 1 端点 |
| 中间件挂载 | 1 次 | 6 次 | 1 次 | 复杂 |
| 工具扩展 | 加 enum | 加 6 行路由 | 加 enum | 加 GraphQL type |
| 团队学习 | 1 URL 模板 | 6 URL 模板 | 1 URL + body 字符串 | GraphQL 概念 |
| 类型安全 | ✅ path param | ✅ path param | ❌ body 字符串 | ✅ schema |
| RESTful | ✅ 是 | ✅ 是 | ⚠️ 弱 | ❌ 否 |
| 业界常见 | ✅ GitHub API 模式 | ⚠️ 偶发 | ⚠️ 弱 | ⚠️ GraphQL 1 派 |
| 文档生成 | 1 OpenAPI spec + 6 schema | 6 OpenAPI spec | 1 OpenAPI spec + 1 schema | GraphQL schema |
| Swagger UI | ✅ 简洁 | ⚠️ 6 endpoint 列出 | ⚠️ 1 endpoint 列出 | ❌ 不适用 |
