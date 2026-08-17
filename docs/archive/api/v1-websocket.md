# Apeireth v1 WebSocket API（8 帧）

> **依据**: `crates/apeireth-api/src/ws_v1.rs` 实际实现 + D-03 主人拍板
> **最后更新**: 2026-08-05

---

## 1. 端点

```
wss://api.apeireth.dev/v1/ws?token=<ws_token>
```

**鉴权**: URL query string 携带 ws_token（5 min TTL 单次使用，per D-03 + [`auth.md`](auth.md) §4）

---

## 2. 8 帧总览

| 帧 | 方向 | 用途 | 必填字段 |
|---|---|---|---|
| `auth` | C → S | 鉴权（首次） | `token` |
| `chat` | C → S | 发消息 | `content` |
| `delta` | S → C | 流式 token 增量 | `text`, `index` |
| `tool_call` | S → C | 工具调用请求 | `tool`, `action`, `params`, `call_id` |
| `tool_result` | C → S | 工具调用结果 | `call_id`, `result` |
| `error` | S → C | 错误响应 | `code`, `message` |
| `ping`/`pong` | 双向 | 心跳保活 | `timestamp` |
| `close` | 双向 | 优雅关闭 | `reason` |

---

## 3. 帧通用 schema

```json
{
  "type": "chat",                  // 必填，8 选 1
  "id": "frame-uuid-1",            // 必填，frame 唯一 ID
  "timestamp": 1754438400000,      // 必填，Unix ms
  "session_id": "sess-uuid-1",     // 必填，会话 ID
  "payload": { ... }               // 必填，frame 特定数据
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `type` | enum | ✅ | 8 帧类型 |
| `id` | uuid v4 | ✅ | 帧 ID（去重/重试用） |
| `timestamp` | int | ✅ | 客户端时间（ms） |
| `session_id` | uuid | ✅ | 会话 ID（首帧后保留） |
| `payload` | object | ✅ | 帧特定数据 |

---

## 4. 8 帧详细

### 4.1 `auth`（客户端 → 服务端）

**首帧必发**，携带 ws_token：

```json
{
  "type": "auth",
  "id": "frame-1",
  "timestamp": 1754438400000,
  "session_id": "sess-uuid-1",
  "payload": {
    "token": "ws_token_5min_ttl"
  }
}
```

**响应**: 成功 → 服务端发 `pong`；失败 → `error` + 关闭

### 4.2 `chat`（客户端 → 服务端）

**功能**: 发用户消息

```json
{
  "type": "chat",
  "id": "frame-2",
  "timestamp": 1754438401000,
  "session_id": "sess-uuid-1",
  "payload": {
    "content": "apeireth 1.0 release 进度如何？",
    "attachments": []
  }
}
```

**响应序列**:
1. 服务端 → `delta` (多个) 流式文本
2. 可选: 服务端 → `tool_call` (调用工具)
3. 客户端 → `tool_result`
4. 服务端 → `delta` (继续流式)
5. 最后: 服务端 → `close` 或继续 `delta`

### 4.3 `delta`（服务端 → 客户端）

**功能**: LLM 流式响应（per token）

```json
{
  "type": "delta",
  "id": "frame-3",
  "timestamp": 1754438401500,
  "session_id": "sess-uuid-1",
  "payload": {
    "text": "apeireth ",
    "index": 5,
    "is_final": false
  }
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `text` | string | 增量文本 |
| `index` | int | 累计 token 数 |
| `is_final` | bool | 是否最后一片（true 时结束） |

### 4.4 `tool_call`（服务端 → 客户端）

**功能**: LLM 决定调用工具

```json
{
  "type": "tool_call",
  "id": "frame-10",
  "timestamp": 1754438402000,
  "session_id": "sess-uuid-1",
  "payload": {
    "call_id": "call-uuid-1",
    "tool": "search",
    "action": "query",
    "params": { "query": "1.0 release 进度" }
  }
}
```

### 4.5 `tool_result`（客户端 → 服务端）

**功能**: 工具调用结果回传

```json
{
  "type": "tool_result",
  "id": "frame-11",
  "timestamp": 1754438402500,
  "session_id": "sess-uuid-1",
  "payload": {
    "call_id": "call-uuid-1",
    "result": { "hits": [...], "total": 3 }
  }
}
```

### 4.6 `error`（服务端 → 客户端）

**功能**: 错误响应

```json
{
  "type": "error",
  "id": "frame-99",
  "timestamp": 1754438403000,
  "session_id": "sess-uuid-1",
  "payload": {
    "code": "TOOL_NOT_FOUND",
    "message": "Tool 'foo' not registered",
    "details": { ... },
    "request_id": "req-..."
  }
}
```

### 4.7 `ping` / `pong`（双向）

**功能**: 心跳保活（30 s 间隔）

```json
// ping
{ "type": "ping", "id": "frame-50", "timestamp": ..., "session_id": "sess-...", "payload": {} }

// pong（服务端响应）
{ "type": "pong", "id": "frame-51", "timestamp": ..., "session_id": "sess-...", "payload": {} }
```

**超时**: 60 s 无 ping/pong → 服务端主动关闭（`PROTOCOL_TIMEOUT`）

### 4.8 `close`（双向）

**功能**: 优雅关闭

```json
{
  "type": "close",
  "id": "frame-100",
  "timestamp": 1754438410000,
  "session_id": "sess-uuid-1",
  "payload": {
    "reason": "user_logout",
    "code": 1000
  }
}
```

**close code**:
- 1000: 正常关闭
- 1001: 端点离开
- 1008: 协议违规
- 1011: 服务端错误

---

## 5. 完整会话示例

```
C: { "type":"auth", "payload":{ "token":"..." } }
S: { "type":"pong", ... }

C: { "type":"chat", "payload":{ "content":"apeireth 1.0 release 进度？" } }

S: { "type":"delta", "payload":{ "text":"让我", "index":1 } }
S: { "type":"delta", "payload":{ "text":"查一下", "index":2 } }
S: { "type":"delta", "payload":{ "text":"...", "index":3 } }

S: { "type":"tool_call", "payload":{ "call_id":"c1", "tool":"search", ... } }
C: { "type":"tool_result", "payload":{ "call_id":"c1", "result":{...} } }

S: { "type":"delta", "payload":{ "text":"1.0 release 进度 80%", "index":10 } }
S: { "type":"delta", "payload":{ "text":"...", "index":11, "is_final":true } }

C: { "type":"close", "payload":{ "reason":"done" } }
```

---

## 6. 限流（per [`rate-limit.md`](rate-limit.md) §6）

| 维度 | 阈值 | 触发 |
|---|---|---|
| 单连接 msg/s | 50 | `RATE_LIMIT_PER_TOOL` |
| 单连接 frame/s | 200 | `PROTOCOL_INVALID_FRAME` |
| 全局活跃连接 | 5000 | `RATE_LIMIT_GLOBAL` |
| 连接时长 | 24 h | 强制断连 |

---

## 7. SDK 用法

```rust
use apeireth_sdk::ws::WsClient;
use futures::StreamExt;

let mut ws = WsClient::connect("wss://api.apeireth.dev/v1/ws", ws_token).await?;

// 发送消息
ws.chat("apeireth 1.0 release 进度？").await?;

// 接收 delta 流
while let Some(frame) = ws.next().await {
    match frame? {
        Frame::Delta(d) => print!("{}", d.text),
        Frame::ToolCall(tc) => {
            // 调 tool 后回传
            let result = call_tool(&tc).await?;
            ws.tool_result(&tc.call_id, result).await?;
        }
        Frame::Close(_) => break,
        _ => {}
    }
}
```

---

## 8. 不假装

- ✅ 8 帧全实装（per 蓝图 §1.1）
- ✅ 流式 LLM 走 SSE-over-WS
- ✅ 心跳保活 30s
- ✅ 限流 4 维度

---

## 9. 相关

- 实现: `crates/apeireth-api/src/ws_v1.rs`
- 鉴权: [`auth.md`](auth.md) §4
- 限流: [`rate-limit.md`](rate-limit.md) §6
- 决策: [`docs/adr/0014-d-03-ws-auth-link-token.md`](../adr/0014-d-03-ws-auth-link-token.md) (D-03 拍板)
