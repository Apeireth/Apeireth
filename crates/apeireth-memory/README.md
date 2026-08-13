# apeireth-memory

> **Apeireth 长期记忆子系统** — Episode / Note / Session SQLite 存储 + BM25 检索 + semantic_search + 三层记忆.
> **当前状态**: R128 + R130 实际实施, 4 子系统: append_only / episode / note / streams + identity + user_profile + three_layer + semantic_persist.
> **真接**: minimax end-to-end example (`minimax_memory_roundtrip.rs`).

---

## 公共 API (R128 实际)

### 核心存储

- `SqliteMemoryStore` — SQLite 真持久化 (file-backed, drop+reopen 可恢复)
- `EpisodeStore` trait — `put_episode / get_episode / query / recent_episodes / count_by_session / list_by_subject`
- `EpisodeQuery` — 复合条件查询 (session_id / continuity_id / 时间窗 / role / limit)
- `Episode` — Apeireth 主路径 episode (含 session_id, append-only)

### 子系统

- `AppendOnly<HistoryEntry, Tombstone>` — 仅追加存储 + 墓碑
- `IdentityCardStore` — IdentityCard 跨 session 持久化
- `NoteStore / SessionStore` — note + session 元数据
- `ThreeLayerMemory` — 三层记忆 (SHORT_TERM_WINDOW_SECS + WORKING_CAPACITY, per R30 U9)
- `PersistentSemanticIndex` — semantic index 持久化
- `UserProfile / ProfileExtractor` — 用户画像提取
- `LlmAnalysis` — LLM 辅助 episode 分析 (analyze_episode)

### 检索

- `semantic_search(query, k, embedder)` — top-k 检索
- `HashEmbedder` — FNV-1a mock embedder (no external API)
- `extract_user_profile(episode)` — 用户画像
- `export_streams_jsonl` — 历史流 JSONL 导出

### Migrations

- `run_migrations / MIGRATIONS` — schema migration
- `applied_migrations` — 迁移历史

## 真接 minimax end-to-end (R128)

```rust
// crates/apeireth-integration-e2e/examples/minimax_memory_roundtrip.rs
let provider = AnthropicCompatibleProvider::new(/* minimax */)?;
let resp = provider.complete(req).await?;
let store = SqliteMemoryStore::open(&path)?;
store.put_episode(&Episode { role: "user", content: prompt, ..Default::default() })?;
store.put_episode(&Episode { role: "assistant", content: resp.content, ..Default::default() })?;
drop(store); // 真关闭
let store2 = SqliteMemoryStore::open(&path)?;
let episodes = store2.query(&EpisodeQuery::new().for_session("s1"))?;
// 验证持久化跨连接
```

## 依赖方向

```
apeireth-memory → apeireth-core + rusqlite + serde + tokio + chrono + uuid
apeireth-memory-extensions (sub-crate) → apeireth-life-force + apeireth-asi 等
```

## 验证

- `cargo check -p apeireth-memory` — 0 errors
- `cargo test -p apeireth-memory` — SQLite migration + semantic + user profile
- `cargo run -p apeireth-integration-e2e --example minimax_memory_roundtrip` — 真接 minimax + 真持久化

## See also

- [minimax 真端到端 example](../../reports/minimax-end-to-end-r128-2026-08-12.md)
- [3-layer memory spec (R30 U9)](../../docs/conventions/)

## R163 lint cleanup

232 -> 0 warnings. 23 source files (lightmemo/* + dailynote/* + identity + dream + user_profile). 4 bugs fixed (trivial cast, unused var, pattern unused binding, conditional binding).
