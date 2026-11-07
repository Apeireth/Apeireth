# apeireth-memory

> Apeireth 规范记忆面 —— Episode / Note / Session 的持久化、词法与向量检索、以及记忆治理。

`apeireth-memory` 拥有记忆条目的语义：可见范围、溯源、排序、合并与遗忘。它按边界契约
**不构造 HTTP 客户端、不读凭据**——具体实现由装配根注入。

## 主入口

- `src/lib.rs` — 记忆面入口与顶层 re-export：`SqliteMemoryStore`（默认 SQLite 后端，实现
  `EpisodeStore`（`src/episode.rs`）、`NoteStore` / `SessionNoteStore`（`src/session_note.rs`）、
  `HistoryStream`（`src/append_only.rs`）等存储 trait）、`Provenance` / `EpisodeMeta`
  （`src/provenance.rs`）、`HybridSearchEngine` / `Bm25Index`（`src/hybrid_search.rs`）与
  `PersistentVectorIndex`（`src/persistent_vector.rs`）。
- `src/scope.rs` — 边界契约：`MemoryScope`（全局 / 用户 / 人格 / 项目 / 会话五级可见范围）、
  `EmbeddingProvider` trait、`NoEmbeddingProvider`、`MemoryCandidate`、`MemoryRankingConfig`。
- `src/canonical/` — 规范记忆子系统的领域模型与查询基础设施：`domain` / `error` / `graph` /
  `repository` / `retrieval` / `sqlite` / `vector`；取回入口是 `retrieve()` 与 `act_r_activation()`。
- `src/retrieval_pipeline.rs` — `HybridRetrievalPipeline`：合并词法候选源（`Bm25LexicalCandidateSource`）与向量候选源（`StaticVectorCandidateSource`）。
- `src/backend/` — 四个存储后端：`sqlite` / `file`（明文 JSON Lines）/ `file_encrypted`
  （AES-256-GCM，opt-in）/ `in_memory`（仅测试用）；`MemoryBackend` trait 定义在 `apeireth-plugin`。
- `src/facade.rs` — `MemoryMutationFacade`：承诺候选与人格增量的最小持久化边界。

## 测试

```
cargo test -p apeireth-memory --locked
```

集成测试在 `crates/engine/memory/tests/`（15 个测试文件），`src/` 内另有单元测试。测试数量以
[docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md) 为单一真源。

## 已知边界

- 记忆面自身不碰网络与凭据：向量阶段的 HTTP 传输在 provider 侧
  （`apeireth-provider::embeddings`），需装配根注入 `EmbeddingProvider`；不注入时走
  `NoEmbeddingProvider`，召回静默退回词法路径。
- 语义向量需 `APEIRETH_EMBEDDING_URL` 与 `APEIRETH_EMBEDDING_MODEL` 才被装配，缺省时没有
  向量阶段；feature 面为空（`default = []`），本 crate 不做能力门控。
- 加密后端 `file_encrypted`（AES-256-GCM）是 opt-in，明文 `file` 后端与它并存。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
