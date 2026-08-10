# calendar 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/calendar.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: list_events / get_event 真接；create/update/delete 501 stub（R21 商业化实装）

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `calendar` |
| **version** | 1.0.0 |
| **scope** | `calendar:read` / `calendar:write` |
| **rate_limit** | capacity=50, refill=10/s |
| **上游** | iCal/Google Calendar API（per `apeireth-protocol`） |
| **持久化** | `apeireth-memory` SQLite（read 缓存） |

---

## 2. Actions

### 2.1 `list_events`

**功能**: 列举时间区间内的事件

**scope 要求**: `calendar:read`

**请求**:
```json
{
  "tool": "calendar",
  "action": "list_events",
  "params": {
    "start": "2026-08-05T00:00:00Z",
    "end": "2026-08-12T00:00:00Z",
    "max_results": 50,
    "calendar_id": "primary"
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `start` | RFC 3339 datetime | ✅ | 区间起点 |
| `end` | RFC 3339 datetime | ✅ | 区间终点（必须 > start） |
| `max_results` | int | 🟡 默认 50 | 上限 500 |
| `calendar_id` | string | 🟡 默认 `primary` | 日历 ID |

**响应**:
```json
{
  "result": {
    "events": [
      {
        "id": "evt-uuid-1",
        "summary": "团队周会",
        "description": "周会议程",
        "start": "2026-08-05T14:00:00Z",
        "end": "2026-08-05T15:00:00Z",
        "location": "线上 Zoom",
        "attendees": ["alice@apeireth.dev", "bob@apeireth.dev"],
        "status": "confirmed",
        "created_at": "2026-08-01T10:00:00Z",
        "updated_at": "2026-08-03T12:30:00Z"
      }
    ],
    "next_page_token": null
  },
  "metadata": { "duration_ms": 142, "cache_hit": false }
}
```

**错误**: `VALIDATION_OUT_OF_RANGE` (max_results > 500) / `RESOURCE_NOT_FOUND` (calendar_id)

---

### 2.2 `get_event`

**功能**: 取单个事件详情

**请求**:
```json
{
  "tool": "calendar",
  "action": "get_event",
  "params": {
    "event_id": "evt-uuid-1"
  }
}
```

**响应**: 单个事件对象（同 list_events 中元素结构）

**错误**: `RESOURCE_NOT_FOUND` (event_id 不存在)

---

### 2.3 `create_event` (501 STUB)

**scope 要求**: `calendar:write`

**请求**:
```json
{
  "tool": "calendar",
  "action": "create_event",
  "params": {
    "summary": "新事件",
    "start": "2026-08-10T10:00:00Z",
    "end": "2026-08-10T11:00:00Z"
  }
}
```

**响应** (501):
```json
{
  "error": {
    "code": "TOOL_NOT_IMPLEMENTED",
    "message": "calendar.create_event is not implemented in 1.0 release; planned for R21",
    "details": { "tool": "calendar", "action": "create_event", "tracking": "R21" },
    "request_id": "req-..."
  }
}
```

**R21 估补**: 主人 2026-08-05 拍板"商业化才实装"

---

### 2.4 `update_event` (501 STUB)

**scope**: `calendar:write`

**错误**: 同 2.3

### 2.5 `delete_event` (501 STUB)

**scope**: `calendar:write`

**错误**: 同 2.3

---

## 3. SDK 用法

```rust
use apeireth_sdk::Client;

let client = Client::new("https://api.apeireth.dev").with_token(token);

// list
let events = client
    .tool("calendar")
    .action("list_events")
    .params(json!({
        "start": "2026-08-05T00:00:00Z",
        "end": "2026-08-12T00:00:00Z"
    }))
    .invoke::<ListEventsResult>()
    .await?;

// get
let event = client
    .tool("calendar")
    .action("get_event")
    .params(json!({ "event_id": "evt-uuid-1" }))
    .invoke::<Event>()
    .await?;
```

---

## 4. 不假装

- ✅ 读操作（list/get）真接 iCal/Google Calendar
- ❌ 写操作（create/update/delete）1.0 返 501
- 🟡 R21 商业化版估补

---

## 5. 相关

- 实现: `crates/apeireth-api/src/v1_tools/calendar.rs`
- 上游协议: `crates/apeireth-protocol/src/lib.rs` (Calendar 协议)
- 决策: [`docs/adr/0017-d-01-tool-endpoint-real.md`](../adr/0017-d-01-tool-endpoint-real.md) (D-01 真接)
