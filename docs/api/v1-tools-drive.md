# drive 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/storage.rs` + `file_ops.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: upload / download / list / delete / get_metadata 全部真接

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `drive` |
| **version** | 1.0.0 |
| **scope** | `drive:upload` / `drive:download` / `drive:list` |
| **rate_limit** | capacity=30, refill=5/s（带宽敏感） |
| **后端** | S3-compatible（默认 MinIO）/ 本地 filesystem |
| **聚合** | file_ops + storage 两个子模块对外统一 `drive` 入口 |

---

## 2. Actions

### 2.1 `upload`

**scope**: `drive:upload`

**请求** (multipart/form-data 或 base64 JSON):
```json
{
  "tool": "drive",
  "action": "upload",
  "params": {
    "name": "report.pdf",
    "content_base64": "JVBERi0xLjQK...",
    "mime_type": "application/pdf",
    "folder": "/projects/2026",
    "metadata": { "tag": "weekly" }
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | ✅ | 文件名 |
| `content_base64` | string | ✅ (二选一) | base64 编码（≤ 50 MB） |
| `content_url` | string | ✅ (二选一) | 流式上传预签名 URL |
| `mime_type` | string | 🟡 默认 `application/octet-stream` | MIME |
| `folder` | string | 🟡 默认 `/` | 路径 |
| `metadata` | object | 🟡 | 自定义元数据 |

**响应**:
```json
{
  "result": {
    "drive_id": "drive-uuid-1",
    "name": "report.pdf",
    "size": 245678,
    "mime_type": "application/pdf",
    "sha256": "abc123...",
    "uploaded_at": "2026-08-05T15:00:00Z",
    "url": "https://api.apeireth.dev/v1/drive/drive-uuid-1"
  }
}
```

---

### 2.2 `download`

**scope**: `drive:download`

**请求**:
```json
{
  "tool": "drive",
  "action": "download",
  "params": { "drive_id": "drive-uuid-1" }
}
```

**响应** (二进制流):
```
Content-Type: application/pdf
Content-Disposition: attachment; filename="report.pdf"
X-Drive-SHA256: abc123...

[binary content]
```

或 base64 JSON:
```json
{
  "result": {
    "drive_id": "drive-uuid-1",
    "name": "report.pdf",
    "content_base64": "JVBERi0xLjQK...",
    "sha256": "abc123..."
  }
}
```

---

### 2.3 `list`

**scope**: `drive:list`

**请求**:
```json
{
  "tool": "drive",
  "action": "list",
  "params": {
    "folder": "/projects/2026",
    "page": 1,
    "page_size": 50
  }
}
```

**响应**:
```json
{
  "result": {
    "files": [
      {
        "drive_id": "drive-uuid-1",
        "name": "report.pdf",
        "size": 245678,
        "mime_type": "application/pdf",
        "folder": "/projects/2026",
        "uploaded_at": "2026-08-05T15:00:00Z"
      }
    ],
    "next_page_token": null
  }
}
```

---

### 2.4 `delete`

**scope**: `drive:upload`（写权限）

**请求**:
```json
{ "tool": "drive", "action": "delete", "params": { "drive_id": "drive-uuid-1" } }
```

**响应**:
```json
{ "result": { "deleted": true, "soft_delete_days": 7 } }
```

> **软删除**: 7 天内可恢复（per 1.0 release 软删策略）

### 2.5 `get_metadata`

**scope**: `drive:list`

**请求**:
```json
{ "tool": "drive", "action": "get_metadata", "params": { "drive_id": "drive-uuid-1" } }
```

**响应**: 文件元数据（不含 content）

---

## 3. SDK 用法

```rust
// 上传
let drive_id = client
    .tool("drive")
    .action("upload")
    .params(json!({
        "name": "report.pdf",
        "content_base64": base64::encode(&pdf_bytes),
        "mime_type": "application/pdf"
    }))
    .invoke::<UploadResult>()
    .await?
    .drive_id;

// 下载
let bytes = client
    .tool("drive")
    .action("download")
    .params(json!({ "drive_id": drive_id }))
    .invoke_raw()
    .await?;
```

---

## 4. 不假装

- ✅ 5 actions 全真接（file_ops + storage 子模块都真接）
- ✅ 软删除 + 7 天可恢复
- ✅ SHA256 去重（per `apeireth-rollback` 4 重防御）

---

## 5. 相关

- 实现: `crates/apeireth-api/src/v1_tools/{storage,file_ops}.rs`
- 备份: `crates/apeireth-rollback` (4 重防御: TTL 7d / 单影子 100MB / 总 2GB / 3 清理钩子)
