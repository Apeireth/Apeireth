# 记忆 (Memory) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-memory` + `apeireth-vector` (24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 记 / **i18n 解剖名词**: 记忆

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | memory (记 / 记忆) |
| **6 command** | recall / store / forget / index / search / purge |
| **关键 dep** | tokio 1.40 / rusqlite 0.32 (硬锁) / tantivy 0.22 / serde 1.0 |
| **状态** | ✅ 24 LOCKED 之一 |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `recall` | 召回 (per 相似度检索) | 召回 |
| `store` | 存储 (per kv + embedding) | 存储 |
| `forget` | 遗忘 (per 隐私 / GDPR) | 遗忘 |
| `index` | 索引 (per tantivy 全文) | 索引 |
| `search` | 搜索 (per 全文 + 向量) | 搜索 |
| `purge` | 清理 (per 过期数据) | 清理 |

---

## 2. API 调用

```rust
use apeireth_memory::organ::memory::{Memory, MemoryRecord, RecallQuery};

let memory = Memory::new();

// store
memory.store(MemoryRecord {
    key: "doc-1",
    content: "Apeireth 是长程 AI 成长平台",
    embedding: vec![/* 768 dim */],
    metadata: HashMap::from([("source", "intro.md")]),
}).await?;

// recall (相似度检索)
let results = memory.recall(RecallQuery {
    query: "Apeireth 1.0 release",
    top_k: 10,
    metric: SimilarityMetric::Cosine,
}).await?;
// Vec<MemoryRecord> 按相似度降序
```

---

## 3. 5 存储后端 (per `apeireth-vector`)

| 后端 | 1.0 状态 | 用途 |
|------|---------|------|
| **SQLite** (rusqlite 0.32) | ✅ | kv + metadata |
| **Tantivy** (0.22) | ✅ | 全文索引 (FST + BM25) |
| **Vector** (768 dim) | ✅ | 相似度 (cosine / euclidean / dot) |
| **Hybrid** (SQLite + Tantivy + Vector) | ✅ | 全文 + 向量混合检索 |
| **LRU Cache** (0.12) | ✅ | 热数据缓存 |

---

## 4. 5 K-1 强校验 (per `apeireth-vector`)

| K-1 | 校验 |
|-----|------|
| **key** | 长度 ≤ 256 + 字符白名单 (alphanumeric + `_-./`) |
| **content** | 长度 ≤ 1 MB (per memory record) |
| **embedding** | 维度 = 768 (固定, per `apeireth-provider-opencode`) |
| **metric** | 3 metric 白名单 (cosine / euclidean / dot) |
| **top_k** | 1 ≤ k ≤ 1000 |

---

## 5. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/memory.rs
impl Command for MemoryCommand {
    fn name(&self) -> &str { "memory" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* recall / store / etc */ }
}
```

---

## 6. 相关

- 实现: `crates/apeireth-memory/` + `crates/apeireth-vector/`
- 1:1 翻译源: v0.9.21 SpectrAI memory organ
- 决策: 整合 #3 C-1 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
