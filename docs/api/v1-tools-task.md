# task 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/task.rs` + `crates/apeireth-task/` 实际实现
> **最后更新**: 2026-08-05
> **状态**: list / get 真接；create / update / complete / delete 501 stub

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `task` |
| **version** | 1.0.0 |
| **scope** | `task:read` / `task:write` |
| **rate_limit** | capacity=50, refill=10/s |
| **存储** | `apeireth-memory` SQLite（tasks 表） |
| **实现细节** | 1:1 翻译 v0.9.21 商业版 taskTools.js（per RIVAL §2.5.4） |

---

## 2. Actions

### 2.1 `list`

**功能**: 列出任务（按状态过滤）

**scope**: `task:read`

**请求**:
```json
{
  "tool": "task",
  "action": "list",
  "params": {
    "status": "pending",
    "assignee": "user-uuid",
    "max_results": 50
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `status` | enum | 🟡 | `pending` / `in_progress` / `completed` / `cancelled` / 不填 = 全部 |
| `assignee` | string | 🟡 | 按负责人过滤 |
| `max_results` | int | 🟡 默认 50 | 上限 500 |

**响应**:
```json
{
  "result": {
    "tasks": [
      {
        "task_id": "task-uuid-1",
        "title": "完成 1.0 release 文档",
        "description": "补全 docs/api/ docs/sdk/ docs/adr/",
        "status": "in_progress",
        "priority": "P0",
        "assignee": "user-uuid",
        "due_date": "2026-09-30T00:00:00Z",
        "tags": ["1.0", "docs"],
        "created_at": "2026-08-01T10:00:00Z",
        "updated_at": "2026-08-05T12:30:00Z"
      }
    ]
  }
}
```

---

### 2.2 `get`

**功能**: 取单任务详情

**请求**:
```json
{ "tool": "task", "action": "get", "params": { "task_id": "task-uuid-1" } }
```

**响应**: 单任务对象（同 list 中元素结构）

---

### 2.3 `create` (501 STUB)

**scope**: `task:write`

**R21 计划 schema**:
```json
{
  "tool": "task",
  "action": "create",
  "params": {
    "title": "新任务",
    "description": "...",
    "priority": "P1",
    "assignee": "user-uuid",
    "due_date": "2026-12-31T00:00:00Z",
    "tags": ["R21"]
  }
}
```

### 2.4 `update` (501 STUB)
### 2.5 `complete` (501 STUB)
### 2.6 `delete` (501 STUB)

---

## 3. SDK 用法

```rust
// 列待办
let tasks = client
    .tool("task")
    .action("list")
    .params(json!({ "status": "pending", "assignee": me.id }))
    .invoke::<ListTasksResult>()
    .await?;
```

---

## 4. 不假装

- ✅ 读（list / get）真接
- ❌ 写（create / update / complete / delete）1.0 返 501
- 写操作 R21 商业化版估补（per D-05）

---

## 5. 相关

- 实现: `crates/apeireth-api/src/v1_tools/task.rs` + `crates/apeireth-task/`
- 1:1 翻译: v0.9.21 商业版 taskTools.js
- 蓝图: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §2.5.4
