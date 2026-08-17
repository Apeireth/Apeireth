# message 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/message.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: send / list / get 真接；mark_read 501 stub

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `message` |
| **version** | 1.0.0 |
| **scope** | `message:send` / `message:read` |
| **rate_limit** | capacity=20, refill=5/s（成本高，per `apeireth-constraint/tools.toml`） |
| **上游** | SMTP / Lark webhook / 飞书 API |
| **存储** | `apeireth-memory` SQLite（消息历史） |

---

## 2. Actions

### 2.1 `send`

**功能**: 发送消息（邮件 / 飞书）

**scope 要求**: `message:send`

**请求**:
```json
{
  "tool": "message",
  "action": "send",
  "params": {
    "channel": "email",
    "to": ["alice@apeireth.dev"],
    "subject": "项目周报",
    "body": "本周完成...",
    "cc": [],
    "bcc": [],
    "attachments": [],
    "is_html": false
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `channel` | enum | ✅ | `email` / `lark` / `sms`(R21) |
| `to` | string[] | ✅ | 收件人 |
| `subject` | string | 🟡 (lark 无需) | 主题 |
| `body` | string | ✅ | 内容（plain text 或 HTML） |
| `cc` / `bcc` | string[] | 🟡 | 抄送 / 密送 |
| `attachments` | object[] | 🟡 | `[{"drive_id": "drive-uuid", "name": "report.pdf"}]` |
| `is_html` | bool | 🟡 默认 false | body 是否 HTML |

**响应**:
```json
{
  "result": {
    "message_id": "msg-uuid-1",
    "sent_at": "2026-08-05T15:00:00Z",
    "channel": "email",
    "recipients_delivered": 1,
    "recipients_failed": 0
  }
}
```

**错误**:
- `UPSTREAM_TIMEOUT` (SMTP 超时)
- `UPSTREAM_AUTH_FAILED` (SMTP 凭据错)
- `VALIDATION_FAILED` (channel 不支持)

---

### 2.2 `list`

**功能**: 列举已发送/已接收消息历史

**scope 要求**: `message:read`

**请求**:
```json
{
  "tool": "message",
  "action": "list",
  "params": {
    "direction": "sent",
    "start": "2026-08-01T00:00:00Z",
    "end": "2026-08-05T23:59:59Z",
    "max_results": 50
  }
}
```

**响应**:
```json
{
  "result": {
    "messages": [
      {
        "message_id": "msg-uuid-1",
        "channel": "email",
        "direction": "sent",
        "to": ["alice@apeireth.dev"],
        "subject": "项目周报",
        "preview": "本周完成...",
        "sent_at": "2026-08-05T15:00:00Z",
        "status": "delivered"
      }
    ]
  }
}
```

---

### 2.3 `get`

**功能**: 取单条消息详情（含完整 body）

**请求**:
```json
{
  "tool": "message",
  "action": "get",
  "params": { "message_id": "msg-uuid-1" }
}
```

**响应**: 单条消息对象（含完整 body + headers）

---

### 2.4 `mark_read` (501 STUB)

**scope**: `message:read`

**错误**: `TOOL_NOT_IMPLEMENTED` (R21 实装)

---

## 3. SDK 用法

```rust
// 发送邮件
client
    .tool("message")
    .action("send")
    .params(json!({
        "channel": "email",
        "to": ["alice@apeireth.dev"],
        "subject": "周报",
        "body": "本周完成..."
    }))
    .invoke::<SendResult>()
    .await?;

// 发送飞书
client
    .tool("message")
    .action("send")
    .params(json!({
        "channel": "lark",
        "to": ["chat_id_xxx"],
        "body": "**加粗** Markdown 内容"
    }))
    .invoke::<SendResult>()
    .await?;
```

---

## 4. 附件上传

附件走 drive 工具先上传，再 message 引用：

```bash
# 1. 上传到 drive
DRIVE_ID=$(curl -X POST .../v1/tools/drive/invoke -d '{"tool":"drive","action":"upload","params":{"content":"...","name":"report.pdf"}}' | jq -r .result.drive_id)

# 2. 引用发送
curl -X POST .../v1/tools/message/invoke -d "{
  \"tool\":\"message\",
  \"action\":\"send\",
  \"params\":{
    \"channel\":\"email\",
    \"to\":[\"alice@apeireth.dev\"],
    \"subject\":\"附件\",
    \"body\":\"见附件\",
    \"attachments\":[{\"drive_id\":\"$DRIVE_ID\",\"name\":\"report.pdf\"}]
  }
}"
```

---

## 5. 不假装

- ✅ send 真接 SMTP / Lark
- ✅ list / get 真接本地存储
- ❌ mark_read 1.0 返 501

---

## 6. 相关

- 实现: `crates/apeireth-api/src/v1_tools/message.rs`
- Lark SDK: `crates/apeireth-lark/` (STUB, 8 NotImplemented)
- 决策: D-01 真接（消息发送是核心，写消息历史真接）
