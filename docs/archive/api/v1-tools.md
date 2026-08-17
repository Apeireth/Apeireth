# Apeireth v1 Tools API 总览（6 工具 endpoint）

> **性质**: 6 工具 endpoint 总览（per D-02 主人拍板统一子路径）
> **依据**: `crates/apeireth-api/src/v1_tools/` 实际实现
> **最后更新**: 2026-08-05

---

## 1. 统一子路径（per D-02）

**所有 6 工具走同一 URL 模板**: `/v1/tools/{name}/invoke`

| 工具 | 路径 |
|---|---|
| calendar | `POST /v1/tools/calendar/invoke` |
| message | `POST /v1/tools/message/invoke` |
| contact | `POST /v1/tools/contact/invoke` |
| task | `POST /v1/tools/task/invoke` |
| search | `POST /v1/tools/search/invoke` |
| drive | `POST /v1/tools/drive/invoke` |

**为什么走同一子路径**（per 主人拍板 A 推荐）:
- ✅ 跟 R20 §3.3 10 REST 端点 `/v1/organs/{name}/invoke` 模式一致
- ✅ 路由表 1 行 vs 6 行
- ✅ 中间件（auth/rate-limit/trace）一次挂载
- ✅ 工具注册表 vs 路由表双源 → 1 处

---

## 2. 通用请求 schema

```json
{
  "tool": "calendar",                     // 必填，6 选 1
  "action": "list_events",                // 必填，工具内 action
  "params": { ... },                      // 可选，action 参数
  "idempotency_key": "uuid-v4",           // 可选，幂等键
  "trace_id": "client-trace-uuid"         // 可选，客户端 trace ID
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `tool` | enum | ✅ | 6 工具枚举，编译期 hardcode |
| `action` | string | ✅ | 工具内 action（如 `list_events` / `create_event`） |
| `params` | object | 🟡 | action 参数，schema per 工具 |
| `idempotency_key` | uuid v4 | 🟡 | 24h 窗口内去重 |
| `trace_id` | string | 🟡 | 客户端 trace ID（关联 OpenTelemetry） |

**服务端校验流程**:
1. JWT 鉴权 → 查 scope
2. 限流检查（3 档 token bucket）
3. 解析 `tool` → 查 tool registry
4. 查 tool 白名单（编译期 hardcode）
5. 校验 `action` 在 tool 内合法
6. JSON schema 校验 `params`
7. 调 tool 实现
8. 返回响应

---

## 3. 通用响应 schema

**成功**:
```json
{
  "result": { ... },                       // action 返回值
  "metadata": {
    "tool": "calendar",
    "action": "list_events",
    "duration_ms": 142,
    "cache_hit": false,
    "trace_id": "req-..."
  }
}
```

**失败**: 走 [`error-codes.md`](error-codes.md) 12 类 code。

---

## 4. 6 工具 action 总表

### 4.1 calendar
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `list_events` | POST | ✅ | [v1-tools-calendar.md](v1-tools-calendar.md) |
| `get_event` | POST | ✅ | 同上 |
| `create_event` | POST | 🟡 501 | 同上 |
| `update_event` | POST | 🟡 501 | 同上 |
| `delete_event` | POST | 🟡 501 | 同上 |

### 4.2 message
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `send` | POST | ✅ | [v1-tools-message.md](v1-tools-message.md) |
| `list` | POST | ✅ | 同上 |
| `get` | POST | ✅ | 同上 |
| `mark_read` | POST | 🟡 501 | 同上 |

### 4.3 contact
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `lookup` | POST | ✅ | [v1-tools-contact.md](v1-tools-contact.md) |
| `list` | POST | ✅ | 同上 |
| `create` | POST | 🟡 501 | 同上 |
| `update` | POST | 🟡 501 | 同上 |
| `delete` | POST | 🟡 501 | 同上 |

### 4.4 task
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `list` | POST | ✅ | [v1-tools-task.md](v1-tools-task.md) |
| `get` | POST | ✅ | 同上 |
| `create` | POST | 🟡 501 | 同上 |
| `update` | POST | 🟡 501 | 同上 |
| `complete` | POST | 🟡 501 | 同上 |
| `delete` | POST | 🟡 501 | 同上 |

### 4.5 search
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `query` | POST | ✅ | [v1-tools-search.md](v1-tools-search.md) |
| `index` | POST | ✅ | 同上（admin scope） |
| `delete` | POST | ✅ | 同上（admin scope） |

### 4.6 drive
| Action | HTTP | 真接 | 文档 |
|---|---|---|---|
| `upload` | POST | ✅ | [v1-tools-drive.md](v1-tools-drive.md) |
| `download` | POST | ✅ | 同上 |
| `list` | POST | ✅ | 同上 |
| `delete` | POST | ✅ | 同上 |
| `get_metadata` | POST | ✅ | 同上 |

---

## 5. curl 示例

### 5.1 calendar 列表
```bash
curl -X POST https://api.apeireth.dev/v1/tools/calendar/invoke \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "calendar",
    "action": "list_events",
    "params": {
      "start": "2026-08-05T00:00:00Z",
      "end": "2026-08-12T00:00:00Z",
      "max_results": 50
    }
  }'
```

### 5.2 完整 SDK 调用（Rust）
```rust
use apeireth_sdk::Client;

let client = Client::new("https://api.apeireth.dev")
    .with_token(access_token);

let events = client
    .tool("calendar")
    .action("list_events")
    .params(json!({
        "start": "2026-08-05T00:00:00Z",
        "end": "2026-08-12T00:00:00Z"
    }))
    .invoke()
    .await?;

println!("Found {} events", events.len());
```

---

## 6. 6 工具不假装清单

| 工具 | 真接 actions | 501 stub actions |
|---|---|---|
| calendar | list_events, get_event | create_event, update_event, delete_event |
| message | send, list, get | mark_read |
| contact | lookup, list | create, update, delete |
| task | list, get | create, update, complete, delete |
| search | query, index, delete | — |
| drive | upload, download, list, delete, get_metadata | — |

> 501 stub 路径在 R21 商业化版估补（per 1.0 release D-05）。

---

## 7. 不修改承诺

- ✅ 编译期 hardcode：6 工具白名单 + 6 工具 enum 编译期固定
- ✅ 不改 LOCKED：tool 协议 schema 严守向后兼容
- ✅ 不假装已实现：每个 action 状态明确标注
- ✅ 不依赖 NewAPI：自建 tool registry

---

## 8. 相关

- 实现: `crates/apeireth-api/src/v1_tools/{calendar,message,contact,task,search,storage}.rs`
- Tool registry: `crates/apeireth-tool-registry/src/`
- 决策: [`docs/adr/0016-d-02-v1-tools-subpath.md`](../adr/0016-d-02-v1-tools-subpath.md) (D-02 拍板)
