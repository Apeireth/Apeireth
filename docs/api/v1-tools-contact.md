# contact 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/contact.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: lookup / list 真接；create / update / delete 501 stub

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `contact` |
| **version** | 1.0.0 |
| **scope** | `contact:read` / `contact:write` |
| **rate_limit** | capacity=100, refill=30/s（读多写少） |
| **存储** | `apeireth-memory` SQLite（contacts 表） |

---

## 2. Actions

### 2.1 `lookup`

**功能**: 按 email / phone / name 查找联系人

**scope 要求**: `contact:read`

**请求**:
```json
{
  "tool": "contact",
  "action": "lookup",
  "params": {
    "query": "alice@apeireth.dev",
    "limit": 10
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `query` | string | ✅ | 模糊匹配 email / phone / name |
| `limit` | int | 🟡 默认 10 | 上限 100 |

**响应**:
```json
{
  "result": {
    "contacts": [
      {
        "contact_id": "ct-uuid-1",
        "name": "Alice",
        "email": "alice@apeireth.dev",
        "phone": "+86-138-0000-0001",
        "organization": "Apeireth",
        "title": "工程师",
        "tags": ["团队", "工程"],
        "created_at": "2026-01-15T10:00:00Z"
      }
    ]
  }
}
```

---

### 2.2 `list`

**功能**: 列出所有联系人（分页）

**请求**:
```json
{
  "tool": "contact",
  "action": "list",
  "params": {
    "page": 1,
    "page_size": 50,
    "tags_filter": ["团队"]
  }
}
```

**响应**: 同 lookup 结构，多 `next_page_token` 字段。

---

### 2.3 `create` (501 STUB)

**scope**: `contact:write`

**错误**: `TOOL_NOT_IMPLEMENTED` (R21 估补)

**R21 计划 schema**:
```json
{
  "tool": "contact",
  "action": "create",
  "params": {
    "name": "Alice",
    "email": "alice@apeireth.dev",
    "phone": "+86-138-0000-0001",
    "organization": "Apeireth",
    "title": "工程师",
    "tags": ["团队"]
  }
}
```

### 2.4 `update` (501 STUB)
### 2.5 `delete` (501 STUB)

---

## 3. SDK 用法

```rust
// 查找
let contacts = client
    .tool("contact")
    .action("lookup")
    .params(json!({ "query": "alice", "limit": 5 }))
    .invoke::<LookupResult>()
    .await?;
```

---

## 4. 不假装

- ✅ 读（lookup / list）真接
- ❌ 写（create / update / delete）1.0 返 501
- 写操作 R21 商业化版估补（per D-05）

---

## 5. 相关

- 实现: `crates/apeireth-api/src/v1_tools/contact.rs`
- 存储: `apeireth-memory` (contacts 表)
