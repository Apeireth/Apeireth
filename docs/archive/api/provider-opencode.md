# opencode Provider API (R20 阶段 4 估补)

> **性质**: 1.0 release #11 license + 5 Provider 100% (per 整合 #3 E-1)
> **依据**: `crates/apeireth-provider-opencode/src/` + `@opencode-ai/sdk` 1.17.15 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: R20 阶段 4 估补 100% 完成, 0 引 SDK; R21 续真接 SDK

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ 100% 完成度 (per 整合 #3 E-1) |
| **3 ModelKind** | opencode-llm / opencode-embed / opencode-rerank |
| **8 工具** | read_file / write_file / edit / bash / grep / glob / web_fetch / web_search |
| **3 特有能力** | vector embedding / 相似度检索 / rerank |
| **3 similarity metric** | cosine / euclidean / dot_product |
| **1:1 翻译源** | @opencode-ai/sdk 1.17.15 |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | reqwest 0.12 + rusqlite 0.32 (本地 embedding 存储) |

---

## 1. 客户端初始化

```rust
use apeireth_provider_opencode::{Client, OpencodeModel};

let client = Client::new(
    std::env::var("OPENCODE_API_KEY")?,
)
.with_model(OpencodeModel::Llm);
```

---

## 2. 3 ModelKind (跟其他 4 Provider 不同)

```rust
pub enum OpencodeModel {
    Llm,        // LLM chat
    Embed,      // 768 维 embedding
    Rerank,     // rerank (per query)
}
```

**3 similarity metric** (opencode 特有):

```rust
pub enum SimilarityMetric {
    Cosine,         // cos(θ) ∈ [-1, 1]
    Euclidean,      // L2 距离 ∈ [0, ∞)
    DotProduct,     // a · b ∈ (-∞, ∞)
}
```

---

## 3. 4 K-1 强校验

| K-1 | 校验内容 |
|-----|---------|
| **token** | 长度 (40) + 字符 (oc- prefix) |
| **model** | 3 model 白名单 |
| **scope** | 5 scope (read / write / admin / owner / root) |
| **metric** | 3 similarity metric 白名单 |

---

## 4. 8 工具 (跟 claude-code 1:1 镜像, 命名略异)

| 工具 | 调用 | 1:1 翻译源 |
|------|------|------------|
| `read_file(path)` | 读文件 | opencode.fs.read |
| `write_file(path, content)` | 写文件 | opencode.fs.write |
| `edit(path, old, new)` | 编辑 | opencode.fs.edit |
| `bash(cmd)` | shell | opencode.shell |
| `grep(pattern, path)` | grep | opencode.search |
| `glob(pattern)` | glob | opencode.glob |
| `web_fetch(url)` | HTTP GET | opencode.web_fetch |
| `web_search(query)` | Web 搜索 | opencode.web_search |

---

## 5. 5 端点 (HTTP) - 含 3 特有

| 端点 | 方法 | 用途 |
|------|------|------|
| `POST /v1/chat/completions` | POST | LLM chat (SSE) |
| `POST /v1/embeddings` | POST | 768 维 embedding |
| `POST /v1/rerank` | POST | rerank 排序 |
| `POST /v1/search` | POST | 向量相似度检索 |
| `GET /v1/models` | GET | 3 model 列表 |

---

## 6. 3 特有能力 (opencode 独有)

### 6.1 vector embedding (opencode-embed)

```rust
let embedding = client.embed("Apeireth 是长程 AI 成长平台").await?;
// 返 Vec<f32> 长度 768

// 本地存储 (rusqlite 0.32, workspace 硬锁)
client.store_embedding("doc-1", &embedding).await?;
```

### 6.2 相似度检索

```rust
let query_embed = client.embed("1.0 release 进度").await?;
let results = client.search(
    &query_embed,
    SimilarityMetric::Cosine,
    10,  // top-10
).await?;
// 返 Vec<(String, f32)> = (doc_id, similarity)
```

### 6.3 rerank

```rust
let ranked = client.rerank(
    "Apeireth 1.0 release",
    &["doc-1", "doc-2", "doc-3"],
    3,  // top-3
).await?;
// 返 Vec<(String, f32)>
```

---

## 7. 19 unit + 19 wiremock = 38 tests (R20 阶段 4 估补)

| 类别 | 数量 |
|------|----:|
| ModelKind 3 model × 3 case = 9 | 9 |
| Embedding 5 case (空 / 短 / 长 / unicode / 重复) | 5 |
| 相似度检索 3 metric × 3 case = 9 | 9 |
| Rerank 5 case | 5 |
| 5 端点 × 2 case (success / auth fail) = 10 | 10 |
| **总** | **38** |

---

## 8. 5 关键差异 (vs 其他 4 Provider)

| 维度 | opencode | 其他 4 Provider |
|------|----------|---------------|
| **embedding** | ✅ 768 维 + 3 metric | ⚪ |
| **rerank** | ✅ query-aware | ⚪ |
| **本地存储** | ✅ rusqlite 0.32 | ⚪ |
| **chat** | ✅ | ✅ |
| **工具** | 8 (同 4) | 8 |

---

## 9. R21 续真接 SDK 计划

| 项 | R21 估时 |
|----|---------|
| 接入 `@opencode-ai/sdk` 商业版 (Rust 包装) | 1 owner × 1 周 |
| 4 K-1 强校验 → SDK adapter | 0.5 owner × 1 周 |
| 19 wiremock 端到端 → 真实 API | 0.5 owner × 1 周 |
| 本地 embedding 存储 (rusqlite) 完善 | 0.5 owner × 1 周 |
| **总** | **2.5 owner × 1 周 ≈ 2.5 周** |

---

## 10. 相关

- [provider-claude-code.md](provider-claude-code.md)
- [provider-codex.md](provider-codex.md)
- [provider-gemini-cli.md](provider-gemini-cli.md)
- [provider-copilot.md](provider-copilot.md)
- 实现: `crates/apeireth-provider-opencode/`
- 1:1 翻译源: @opencode-ai/sdk 1.17.15
- 决策: 整合 #3 E-1 + E-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
