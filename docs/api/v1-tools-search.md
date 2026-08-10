# search 工具详细 API

> **依据**: `crates/apeireth-api/src/v1_tools/search.rs` + `crates/apeireth-vector/` 实际实现
> **最后更新**: 2026-08-05
> **状态**: query / index / delete 全部真接

---

## 1. 工具元信息

| 字段 | 值 |
|---|---|
| **name** | `search` |
| **version** | 1.0.0 |
| **scope** | `search:query` / `search:admin` |
| **rate_limit** | capacity=100, refill=30/s（查询密集） |
| **后端** | tantivy 全文索引 + sqlite-vec 向量检索（per `apeireth-vector`） |

---

## 2. Actions

### 2.1 `query`

**功能**: 混合检索（keyword + vector）

**scope**: `search:query`

**请求**:
```json
{
  "tool": "search",
  "action": "query",
  "params": {
    "query": "apeireth 1.0 release",
    "mode": "hybrid",
    "limit": 10,
    "filters": {
      "source": "calendar",
      "date_range": {
        "start": "2026-08-01T00:00:00Z",
        "end": "2026-08-05T23:59:59Z"
      }
    }
  }
}
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `query` | string | ✅ | 搜索关键词 |
| `mode` | enum | 🟡 默认 `hybrid` | `keyword` / `vector` / `hybrid` |
| `limit` | int | 🟡 默认 10 | 上限 100 |
| `filters` | object | 🟡 | 过滤条件 |

**响应**:
```json
{
  "result": {
    "hits": [
      {
        "doc_id": "doc-uuid-1",
        "score": 0.92,
        "source": "calendar",
        "title": "1.0 release 计划",
        "snippet": "...apeireth 1.0 release 准备...",
        "metadata": {
          "created_at": "2026-08-01T10:00:00Z",
          "url": "calendar://evt-uuid-1"
        }
      }
    ],
    "total": 1,
    "duration_ms": 18
  }
}
```

---

### 2.2 `index`

**功能**: 索引文档

**scope**: `search:admin`

**请求**:
```json
{
  "tool": "search",
  "action": "index",
  "params": {
    "doc_id": "doc-uuid-new",
    "source": "drive",
    "title": "Q3 OKR",
    "content": "Q3 目标：完成 1.0 release...",
    "metadata": { "url": "drive://file-uuid" }
  }
}
```

**响应**:
```json
{
  "result": { "indexed_at": "2026-08-05T15:30:00Z", "doc_id": "doc-uuid-new" }
}
```

---

### 2.3 `delete`

**功能**: 从索引删除文档

**scope**: `search:admin`

**请求**:
```json
{ "tool": "search", "action": "delete", "params": { "doc_id": "doc-uuid-1" } }
```

**响应**:
```json
{ "result": { "deleted": true } }
```

---

## 3. 检索模式

| 模式 | 算法 | 适用 |
|---|---|---|
| `keyword` | tantivy BM25 | 精确关键词 |
| `vector` | sqlite-vec 余弦相似度 | 语义检索 |
| `hybrid` | BM25 + 向量 加权融合 | 默认（推荐） |

**hybrid 权重**（per `apeireth-vector/src/rerank.rs`）:
- keyword_score × 0.4
- vector_score × 0.6
- 总分 = 加权和

---

## 4. SDK 用法

```rust
let results = client
    .tool("search")
    .action("query")
    .params(json!({
        "query": "1.0 release",
        "mode": "hybrid",
        "limit": 5
    }))
    .invoke::<SearchResult>()
    .await?;
```

---

## 5. 不假装

- ✅ query / index / delete 全真接（唯一全 action 真接的工具）
- ✅ 性能：hybrid 查询 P99 < 200ms（per `reports/v2-memory-vector-perf-2026-08-05.md`）

---

## 6. 相关

- 实现: `crates/apeireth-api/src/v1_tools/search.rs` + `crates/apeireth-vector/`
- 性能: `reports/v2-memory-vector-perf-2026-08-05.md`
- 协议: `crates/apeireth-protocol` (Search 协议)
