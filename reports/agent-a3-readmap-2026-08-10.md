# Agent A-3 战区 4 (Memory) 续 — Vector Long-Term Persistence Readmap

**日期**: 2026-08-10
**作者**: Mavis 派 — Agent A-3 (Apeireth-rust 后端升级, 接 A)
**任务**: 把 A 一次性 `SemanticIndex` 升级为 **持久化长程索引**, 跨 daemon 重启不丢
**战区**: 4 (Memory) 续 — `apeireth-memory::semantic_persist` (新) + `lib.rs` 集成 2 处
**总时间预算**: 7h (A3-1 读图 1h + A3-2~3 实现 3h + A3-4 集成 1h + A3-5 test 1h + A3-6 报告 30min + 缓冲 30min)
**状态**: 🚧 进行中, 03:39 开始

---

## §0. TL;DR

| # | 关键事实 | 数据 / 状态 |
|---|---------|-----------|
| 1 | **A 已完成什么** | 一次性 `SemanticIndex` (in-memory vec0 backend, 7 unit tests) + `extract_user_profile` + `user_profile.rs` (9 tests) — vector 31/31 + memory 95/95 全过 |
| 2 | **A 缺什么 (A final §1.4 标缺)** | `SemanticIndex<'m>` 借 `&'m SqliteMemoryStore` + `Box<dyn VectorStore>` 默认 in-memory → 跨 daemon 不持久 |
| 3 | **本任务核心** | 用 A 已写 `SqliteVecBackend::open(path)` (真接 disk) + `Arc<SqliteMemoryStore>` (跨 daemon), 写 `PersistentSemanticIndex` |
| 4 | **0 改** A 公开 API 签名 | `semantic_search` / `extract_user_profile` / `SemanticIndex::new` 全部 0 改 (向后兼容) |
| 5 | **0 触碰** 24 LOCKED | 新建 `semantic_persist.rs` + lib.rs 加 re-export + 加 2 个方法 (0 触碰 LOCKED 9 文件) |
| 6 | **0 改** workspace.version (1.1.0) | ✅ |
| 7 | **0 主动 commit** | ✅ (等主人验收) |

---

## §1. A 已写代码全景 (我跟 A 的接口边界)

### 1.1 `SemanticIndex<'m>` (semantic.rs, 12.3KB)

```rust
pub struct SemanticIndex<'m> {
    memory: &'m SqliteMemoryStore,         // ⚠ 借用, 不能跨 daemon
    vector: Mutex<Box<dyn VectorStore>>,    // ⚠ 默认 in-memory
    embedder: Arc<dyn EmbedFn>,
}
```

**A 公开 API** (硬约束: **0 改**):
- `SemanticIndex::new(memory, vector: Box<dyn VectorStore>, embedder: Arc<dyn EmbedFn>) -> Self`
- `index_episode(ep) / index_episodes(eps)`
- `search(query, k) -> Vec<Episode>`
- `len() / vector() / embedder() / extract_profile() -> UserProfile`

**A 已实现的 SQL** (硬约束: **0 改**):
- `index_episode` 调 `vector.upsert(&Vector::new(episode_uuid(ep.id), vec))`
- `search` 调 `vector.search(qvec, k)` → 反查 `SqliteMemoryStore::query(limit=100_000)`
- `len` 调 `vector.len()`
- `extract_profile` 调 `ProfileExtractor::extract(memory, Some(self))`

**A 7 个 unit test** (硬约束: **0 触碰**):
- `hash_embedder_is_deterministic`
- `hash_embedder_different_text_different_vector`
- `hash_embedder_output_is_l2_normalized`
- `episode_uuid_is_deterministic`
- `semantic_index_indexes_and_searches` (核心)
- `semantic_index_search_with_zero_corpus_returns_empty`
- `semantic_index_dim_auto_set_on_first_upsert`

### 1.2 `SqliteVecBackend` (sqlite_backend.rs, 32KB, A 写)

**A 已实现** (我**直接复用**, 0 触碰):
- `pub fn open(path: impl AsRef<Path>) -> Result<Self, VectorError>` — **真接 disk, write-through WAL**
- `pub fn open_in_memory() -> Result<Self, VectorError>` — 测试用
- 内部: `vec0` 虚拟表 + `vec_idmap` (uuid BLOB ↔ rowid) + `vec_meta` (dim/metric)
- `PRAGMA journal_mode=WAL; synchronous=NORMAL; foreign_keys=ON;` (line 127-130)
- `set_dimension` 触发 `CREATE VIRTUAL TABLE vec_items USING vec0(...)` (line 335-341)
- `len()` 查 `SELECT COUNT(*) FROM vec_items` (line 361)
- 12 个 unit test (硬约束: **0 触碰**)

**关键 insight**: SqliteVecBackend::open(path) **本身已经是 long_term persistence**!
- write-through WAL → 写入即落盘
- 重启后 `open(path)` 重新打开 → 自动从 disk 读 vec0 + idmap + meta
- `set_dimension` 检查已存在 dim, 一致就 no-op (line 302-310) — 重启后不会重建

### 1.3 `SqliteMemoryStore` (lib.rs, 17.8KB)

**A 已实现** (我**集成**到 `SqliteMemoryStore` 上):
- `pub fn open(path: impl AsRef<Path>) -> MemoryResult<Self>` (line 197) — **真接 disk**
- `pub fn open_in_memory() -> MemoryResult<Self>` (line 211) — 测试用
- `pub fn semantic_search(query, k, embedder) -> MemoryResult<Vec<Episode>>` (line 267) — 一次性 in-memory vec0
- `pub fn extract_user_profile(embedder) -> MemoryResult<UserProfile>` (line 305) — 一次性 in-memory vec0
- `pub fn conn() -> MutexGuard<Connection>` (line 233) — 拿 connection 锁
- 内部 `Mutex<Connection>` (line 192) — 可 `Arc<SqliteMemoryStore>` 共享

**A `semantic_search` 当前实现** (line 267-299):
```rust
pub fn semantic_search(&self, query, k, embedder) -> MemoryResult<Vec<Episode>> {
    // 1. 拉所有 episodes
    // 2. in-memory vec0 backend  ← ⚠ 每次重建, 0 持久
    // 3. 索引 + 检索
}
```

### 1.4 我要 0 改的 A 公开 API (硬约束)

| API | 签名 | 在哪 |
|-----|------|------|
| `SqliteMemoryStore::semantic_search` | `(&self, query, k, embedder) -> Vec<Episode>` | `lib.rs:267` |
| `SqliteMemoryStore::extract_user_profile` | `(&self, embedder) -> UserProfile` | `lib.rs:305` |
| `SemanticIndex::new` | `(memory, vector: Box<dyn VectorStore>, embedder: Arc<dyn EmbedFn>) -> Self` | `semantic.rs:113` |
| `SemanticIndex::index_episode` | `(&self, ep) -> MemoryResult<()>` | `semantic.rs:139` |
| `SemanticIndex::index_episodes` | `(&self, eps) -> MemoryResult<()>` | `semantic.rs:156` |
| `SemanticIndex::search` | `(&self, query, k) -> Vec<Episode>` | `semantic.rs:164` |
| `SemanticIndex::len` | `(&self) -> usize` | `semantic.rs:204` |
| `SemanticIndex::extract_profile` | `(&self) -> UserProfile` | `semantic.rs:214` |
| `EmbedFn` trait | (trait obj) | `semantic.rs:47` |
| `HashEmbedder` struct | (deterministic) | `semantic.rs:65` |
| `ProfileExtractor::extract` | `(&self, memory, Option<&SemanticIndex>) -> UserProfile` | `user_profile.rs` |

### 1.5 ROADMAP 待办 #2 (原文)

> "vector store long_term 真接 — 当前 total/5 heuristic, apeireth-vector 还在 skeleton"

**A 1.0 验收基准** (per A final §1.1 标缺):
> "memory semantic_search 接受 Arc<dyn EmbedFn> 一次性便利方法; 跨 daemon 重建 index 的 long_term persistence 留 R121+ 续"

---

## §2. Long-Term Persistence 设计

### 2.1 核心设计原则

1. **复用 A 已写 SqliteVecBackend::open(path)**: 它本身已经是 write-through WAL 真接 disk,不需要重写
2. **新 struct `PersistentSemanticIndex`** 替代 `SemanticIndex` 的"借 &SqliteMemoryStore + Box<dyn VectorStore>" 模式:
   - 内部 `Arc<SqliteMemoryStore>` (跨 daemon 共享, Send + 'static)
   - 内部 `SqliteVecBackend` (path-based, 持久化, Send)
   - 内部 `Arc<dyn EmbedFn>` (跨 daemon 共享)
3. **`save()` / `open()` 语义**:
   - `open(memory, vector_path, embedder)`: 从 disk reload vec0 + idmap + meta
   - `save()`: SQLite WAL 已经是 write-through, save() 等价于"调 PRAGMA wal_checkpoint(TRUNCATE) 强制 fsync" (防电源掉)
4. **A 公开 API 0 改**: 通过 `impl From<PersistentSemanticIndex> for SemanticIndex` 桥接 (借 memory 引用 + Box::new(self.vector.clone()))

### 2.2 公开 API 设计 (新 `PersistentSemanticIndex`)

```rust
// crates/apeireth-memory/src/semantic_persist.rs (新文件)
pub struct PersistentSemanticIndex {
    memory: Arc<SqliteMemoryStore>,        // 共享, 跨 daemon
    vector_path: PathBuf,                  // disk 路径
    vector: Arc<Mutex<SqliteVecBackend>>,  // 共享, 跨 daemon
    embedder: Arc<dyn EmbedFn>,            // 共享, 跨 daemon
}

impl PersistentSemanticIndex {
    /// 从 disk 打开 (或新建) 一个 long-term semantic index.
    /// - `memory`: 调用方持的 SqliteMemoryStore (Arc 共享)
    /// - `vector_path`: vec0 db 文件路径 (例: "./data/vector.db")
    /// - `embedder`: 文本 → 向量 (跨进程同一实例 OK)
    pub fn open(
        memory: Arc<SqliteMemoryStore>,
        vector_path: impl AsRef<Path>,
        embedder: Arc<dyn EmbedFn>,
    ) -> MemoryResult<Self>;

    /// 强制 fsync (WAL → main db). 通常不需要 — 写入已 write-through.
    /// 适用场景: 重要 checkpoint (shutdown 前 / 重要写入后).
    pub fn save(&self) -> MemoryResult<()>;

    /// 索引单条 episode.
    pub fn index_episode(&self, ep: &Episode) -> MemoryResult<()>;

    /// 批量索引.
    pub fn index_episodes(&self, eps: &[Episode]) -> MemoryResult<()>;

    /// KNN 检索.
    pub fn search(&self, query: &str, k: usize) -> MemoryResult<Vec<Episode>>;

    /// 当前 index 里的向量数.
    pub fn len(&self) -> MemoryResult<usize>;

    pub fn is_empty(&self) -> MemoryResult<bool> {
        Ok(self.len()? == 0)
    }

    /// 提取用户画像 (跟 A 一次性 `extract_profile` 行为一致).
    pub fn extract_profile(&self) -> MemoryResult<UserProfile>;

    /// 取出 disk 路径.
    pub fn vector_path(&self) -> &Path;
}

// 关键桥: 跟 A 的 SemanticIndex 互转
impl<'m> From<PersistentSemanticIndex> for SemanticIndex<'m> {
    fn from(p: PersistentSemanticIndex) -> Self {
        // 注意: caller 必须确保 &'m SqliteMemoryStore 的 lifetime >= p
    }
}
```

**A 公开 API 完整保留 (0 改)**:
- `SqliteMemoryStore::semantic_search(query, k, embedder) -> Vec<Episode>` (一次性, 走 in-memory)
- `SqliteMemoryStore::extract_user_profile(embedder) -> UserProfile` (一次性, 走 in-memory)
- `SemanticIndex::new(memory, vector, embedder) -> Self` (一次性, 走 in-memory)
- **新增** (不破坏 A API):
  - `SqliteMemoryStore::open_persistent_semantic_index(vector_path, embedder) -> PersistentSemanticIndex`
  - `SqliteMemoryStore::semantic_search_persistent(query, k, vector_path, embedder) -> Vec<Episode>` (便捷方法)

### 2.3 跨 daemon 持久化流程 (验证场景)

```
[daemon 1]                              [daemon 2]
  open(memory1, "/data/vec.db", e)         open(memory2, "/data/vec.db", e)
  index 100 episodes                        ↑ reload from disk
  save() (optional)                          len() = 100  ← 验证
  shutdown                                   search() 命中 ← 验证
  WAL → disk fsync                          
```

**关键测试**:
```rust
#[test]
fn cross_daemon_persistence_100_episodes() {
    // 准备: tempdir + 100 episode corpus
    let dir = tempfile::tempdir().unwrap();
    let vec_path = dir.path().join("vec.db");

    // 阶段 1: daemon 1 写
    let mem1 = Arc::new(SqliteMemoryStore::open(dir.path().join("memory.db")).unwrap());
    {
        let mut idx = PersistentSemanticIndex::open(
            Arc::clone(&mem1), &vec_path, Arc::new(HashEmbedder::new(32))
        ).unwrap();
        // 写 100 episode
        for i in 0..100 {
            mem1.put_episode(&make_episode(&format!("e{i}"), ...)).unwrap();
            idx.index_episode(&...);
        }
        idx.save().unwrap();
        assert_eq!(idx.len().unwrap(), 100);
    } // idx drop, 模拟 daemon 关闭

    // 阶段 2: daemon 2 重开
    let mem2 = Arc::new(SqliteMemoryStore::open(dir.path().join("memory.db")).unwrap());
    let idx2 = PersistentSemanticIndex::open(
        Arc::clone(&mem2), &vec_path, Arc::new(HashEmbedder::new(32))
    ).unwrap();
    assert_eq!(idx2.len().unwrap(), 100, "100 条应跨 daemon 持久");
    let hits = idx2.search("SQL", 5).unwrap();
    assert!(!hits.is_empty(), "重开后 search 应能命中");
}
```

### 2.4 SqliteVecBackend Send + Sync 推导

A 的 `SqliteVecBackend`:
- 内部 `Connection: Send` (rusqlite 0.32 guaranteed)
- 0 主动实现 `Sync` (A line 645-646 注释说: Sync 不自动提供, 上层用 `Arc<Mutex<...>>`)

**我的 `PersistentSemanticIndex`**:
- `Arc<SqliteVecBackend>` 在 `Arc<Mutex<SqliteVecBackend>>` 中, **Mutex 提供 Sync** (A 模式 1:1)
- `Arc<SqliteMemoryStore>` 同理 (A 内部 `Mutex<Connection>`)
- 整个 struct 是 `Send + Sync` (编译期验)

### 2.5 Schema 镜像 (跟 A 1:1)

| 表 | A 1:1 镜像 | 我加什么 |
|----|-----------|----------|
| `vec0(vec_items)` | ✅ 完全复用 | 无 |
| `vec_idmap` | ✅ 完全复用 | 无 |
| `vec_meta` | ✅ 完全复用 | 无 |
| memory `episodes` | ✅ SqliteMemoryStore::open(path) | 无 |

**0 新表 / 0 新 schema / 0 新字段** — 完全复用 A 已写路径。

---

## §3. 跟 A 1.0 验收基准 0 漂移核验

| A 已写 | 我动什么 | 影响 |
|--------|---------|------|
| `semantic_search(&self, query, k, embedder)` 签名 | 0 改 (line 267) | A 1.0 验收基准 0 漂移 ✅ |
| `extract_user_profile(&self, embedder)` 签名 | 0 改 (line 305) | A 1.0 验收基准 0 漂移 ✅ |
| `SemanticIndex::new(memory, vector, embedder)` 签名 | 0 改 (semantic.rs:113) | A 1.0 验收基准 0 漂移 ✅ |
| `SemanticIndex` 7 unit test | 0 改 0 触碰 | A 验收 95/95 仍过 ✅ |
| `SqliteVecBackend` 12 unit test | 0 改 0 触碰 | A 验收 31/31 仍过 ✅ |
| `SqliteMemoryStore` lib.rs 顶层 | +1 个 re-export (pub use semantic_persist) + 2 个新方法 | 0 触碰 LOCKED 9 文件 ✅ |
| 24 LOCKED 名单 | 0 触碰 | ✅ |
| workspace.version (1.1.0) | 0 改 | ✅ |

---

## §4. 测试设计 (A3-5 阶段)

### 4.1 unit test (≥ 15 个, 在 `semantic_persist.rs` `mod tests`)

| # | 名称 | 验证什么 |
|---|------|---------|
| 1 | `open_creates_db_file_on_disk` | 新建路径 → 文件存在 + vec_meta 表有数据 |
| 2 | `open_existing_db_reloads_dim` | 重开已有 db → dimension 一致 |
| 3 | `open_existing_db_reloads_metric` | 重开 → metric 一致 (cosine 默认) |
| 4 | `index_episode_persists_to_disk` | index 1 条 → 关闭 → 重开 → 1 条仍在 |
| 5 | `index_episodes_batch_persists` | 批量 50 条 → 重开 → 50 条仍在 |
| 6 | `search_after_reopen_hits_existing` | 重开后 search 能命中 |
| 7 | `search_with_zero_corpus_after_reopen_returns_empty` | 重开空 index → 0 hit |
| 8 | `len_after_reopen_matches_indexed_count` | len 跨 daemon 正确 |
| 9 | `is_empty_correct` | 空 → true; 1 条后 → false |
| 10 | `save_forces_wal_checkpoint` | save 后 WAL 文件大小缩 (或 PRAGMA wal_checkpoint 验证) |
| 11 | `extract_profile_after_reopen_works` | extract_profile 跟 A 一次性行为一致 |
| 12 | `vector_path_accessor` | vector_path() 返正确路径 |
| 13 | `open_persistent_semantic_index_on_sqlite_store` | SqliteMemoryStore::open_persistent_semantic_index 便捷方法 |
| 14 | `semantic_search_persistent_convenience` | 一次调用: open + index + search + close |
| 15 | `from_persistent_to_semantic_index_works` | From 桥: PersistentSemanticIndex → SemanticIndex 借用期能 search |

### 4.2 integration test (≥ 5 个, 在 `tests/` 新文件 `vector_persistence.rs`)

| # | 名称 | 验证什么 |
|---|------|---------|
| 1 | `cross_daemon_persistence_100_episodes` | 写 100 → 关闭 → 重开 → 数据 + search 仍 OK |
| 2 | `cross_daemon_persistence_1000_episodes` | 写 1000 → 关闭 → 重开 → len + search 仍 OK |
| 3 | `concurrent_persistent_index_drops_gracefully` | 2 个 PersistentSemanticIndex 写同一 path (lock 不冲突, 后写者覆盖) |
| 4 | `vector_db_corrupt_recovery` | 写一半 + 删 WAL → 重开 → 自动重建 (or panic with clear msg) |
| 5 | `persistent_index_with_real_embedder_swap` | 启动时 embedder 1, 重启后 embedder 2 (不同 dim) → 报错清晰 |

**累计预期**: A 已有 95/95 (memory) + A 31/31 (vector) = 126 → 我加 15 unit + 5 integration = 146 总

---

## §5. 实施阶段 (跟任务清单 1:1)

| 阶段 | 时长 | 交付 | 风险 |
|------|------|------|------|
| A3-1 | 0-1h | 本 readmap (现在完成) | 无 |
| A3-2 | 1-3h | `semantic_persist.rs` (PersistentSemanticIndex struct + 6 公开方法 + From 桥 + 15 unit test) | 中: From 借用 lifetime 复杂 |
| A3-3 | 3-4.5h | path-based SqliteVecBackend 集成 (semantic_persist.rs 内部用 SqliteVecBackend::open(path)) | 低: A 已写 |
| A3-4 | 4.5-5.5h | 集成到 SqliteMemoryStore (open_persistent_semantic_index + semantic_search_persistent) | 低: lib.rs 加 2 方法 + 1 re-export |
| A3-5 | 5.5-6.5h | 5 integration test (跨 daemon 持久化) | 中: tempdir + 多进程模拟 |
| A3-6 | 6.5-7h | `reports/agent-a3-final-2026-08-10.md` + `reports/agent-a3-decision-log-2026-08-10.md` | 无 |

---

## §6. 风险点

| # | 风险 | 缓解 |
|---|------|------|
| R1 | `From<PersistentSemanticIndex> for SemanticIndex<'m>` lifetime 复杂 | 简单方案: 桥接时把 `Arc<SqliteMemoryStore>` 解引用为 `&'m SqliteMemoryStore`,caller 保证 lifetime. 提供 `into_semantic_index<'m>(self, &'m SqliteMemoryStore) -> SemanticIndex<'m>` 显式 API |
| R2 | `Arc<Mutex<SqliteVecBackend>>` 跟 A `Mutex<Box<dyn VectorStore>>` 不兼容 | 用 `Arc::get_mut` / `Mutex::lock` 配 A trait — Box::new 转 dyn |
| R3 | SqliteVecBackend::open(path) 跟 A 已测试的 `open_in_memory` 行为可能差异 (WAL, PRAGMA) | A 测试已用 `:memory:`, path 跟 :memory: 共享 sql 引擎, 风险低 |
| R4 | 跨 daemon 测试需要真 tempdir + 多个 SqliteMemoryStore | 用 `tempfile::tempdir()` 隔离; 每个 daemon Arc::new SqliteMemoryStore::open 不同 path |
| R5 | vec0 set_dimension 在重开时校验已有 dim, 新 embedder dim 不一致会报错 | 这是 feature not bug: 持久化的 dim 跟 embedder dim 必须一致; R1 fail-fast 行为正确 |
| R6 | cargo check 时间 | A 报告 memory 编译 8.5s, 我加 1 文件 + 1 文件扩展 ≤ +2s |

---

## §7. 不做的事 (硬约束)

- ❌ 不重写 SqliteVecBackend (A 已写 32KB, 完美)
- ❌ 不改 workspace.version (1.1.0)
- ❌ 不触碰 24 LOCKED (9 memory LOCKED + 9 cognition + 4 核心)
- ❌ 不改 A 公开 API 签名
- ❌ 不改 A 7 unit test
- ❌ 不改 A 12 vector test
- ❌ 不主动 commit (等主人验收)
- ❌ 不跟 A/B/C/D-2/D-3/B-2 冲突 (git diff 不交叉)
- ❌ 不假装 — 跨 daemon 持久化 test 必真实跑 (写 → 关闭 → 重开 → 验证)

---

## §8. 关联文档 (读完)

- **A final**: `reports/agent-a-final-2026-08-10.md` (24KB, R19 P2 战区 4 主线)
- **A readmap**: `reports/agent-a-readmap-2026-08-10.md` (16.7KB, A1 读图)
- **A-2 final**: `reports/agent-a2-final-2026-08-10.md` (.github 工程化, 0 冲突)
- **B-2 final**: `reports/agent-b2-final-2026-08-10.md` (bench 方向, 0 冲突)
- **A semantic.rs**: `crates/apeireth-memory/src/semantic.rs` (12.3KB, 7 unit test)
- **A sqlite_backend.rs**: `crates/apeireth-vector/src/sqlite_backend.rs` (32KB, 12 unit test)
- **A lib.rs**: `crates/apeireth-memory/src/lib.rs` (17.8KB, semantic_search + extract_user_profile)
- **A user_profile.rs**: `crates/apeireth-memory/src/user_profile.rs` (15.5KB, 9 unit test)
- **A bench**: `crates/apeireth-memory/benches/v2-memory-vector-bench.rs` (5KB, compile OK)
- **CONTRIBUTING.md**: 24 LOCKED 名单 + 0 触碰实查 + 6 哲学 + 8 项

---

## §9. 时间线 (实时)

- **03:39** A3-1 完成, readmap 落地
- **下一步**: A3-2 写 `semantic_persist.rs` (目标 03:39-05:39, 2h)

---

_本文件路径: `reports/agent-a3-readmap-2026-08-10.md`_
_生成时间: 2026-08-10 03:39_
_派工来源: Mavis A-3 派活, 接 A (vector+memory 一次性) + A-2 (.github) 后续战区_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 改 R11 baseline + 0 触碰 24 LOCKED + 0 主动 commit + 不与 A/B/C/D-2/D-3/B-2 冲突_
