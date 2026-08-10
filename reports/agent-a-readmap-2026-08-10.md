# Agent-A 战区 4 (Memory) — Readmap 报告
**日期**: 2026-08-10  
**作者**: Mavis 派 — Agent A (Apeireth-rust 后端升级)  
**任务**: 实现 `apeireth-vector` 真接 `sqlite-vec` + `apeireth-memory::semantic_search` + `extract_user_profile`  
**阶段**: A1 读图 (0-2h)

---

## 1. 现有 vector 子系统 (战区 4 核心)

### 1.1 文件结构 (`crates/apeireth-vector/`)
```
Cargo.toml          1.1KB  — rusqlite 0.32 (workspace), serde, uuid, anyhow
src/lib.rs          1.3KB  — 公共 re-export + 0 unsafe_code deny
src/error.rs        1.3KB  — VectorError enum (Sqlite/Json/Io/Dim/Empty/NonFinite/Other)
src/traits.rs       3.5KB  — VectorStore trait + Vector/SearchHit/ScoredId 类型
src/sqlite_backend.rs 13.8KB — SqliteVecBackend (rusqlite + BLOB + Rust 余弦)
tests/store.rs      4.8KB  — 13 integration tests
examples/semantic_smoke.rs 6.9KB — 1000 条 mock 向量 smoke test
```

### 1.2 关键现状 ✅
- **VectorStore trait 公共 API 完整**: `set_dimension` / `dimension` / `len` / `is_empty` / `upsert` / `upsert_batch` / `search` / `delete` / `clear` — 9 个方法,形状符合 R18 P2 第 1 项验收。
- **SqliteVecBackend 已实现**: 表 `vec_items(id BLOB PK, dim INTEGER, vec BLOB, metadata TEXT)`, `vec_meta` 存 dim;WAL + synchronous=NORMAL;事务化 batch;元数据 JSON 序列化;dim/NaN/Inf 校验。
- **13 个 integration test 全过**: `vector_new_*` / `vector_with_metadata` / `backend_*` (8 个) — 涵盖 open、dim、upsert、overwrite、search、k_limit、delete、clear、path。
- **6 个 unit test (src/sqlite_backend.rs)**: dim 设置、upsert/search/delete、dim mismatch、NaN 拒绝、query dim mismatch、batch 事务。
- **semantic_smoke example**: 1000 条 × 256 维写入 + 50 次查询,打印 p50/p99/max 延迟 + top-1 category 命中率,monkey-test 走通。
- **Cargo.toml 已有 `[lints] workspace = true`**,符合 R19 第 0 阶段。

### 1.3 关键缺什么 ❌ (本次要补)
1. **不接 sqlite-vec**: 纯 BLOB + Rust 内 brute-force 余弦,>10w 条性能不够。
2. **没有 cosine/l2 距离度量切换**: 当前 hardcode 余弦;vec0 扩展原生支持 `distance_metric=cosine|l2`。
3. **没有 metadata 过滤列**: vec0 扩展支持 `text` / `int` 辅助列 + `WHERE x = ?` 预过滤,目前 metadata 只是 JSON blob。
4. **没有 distance 字段返回**: 当前 `SearchHit.score` 是手动算的相似度;vec0 扩展直接返 `distance`,需要转回。
5. **没接 `apeireth-memory`**: vector 不知道 episode 是什么,得有一个 facade 把 memory.upsert_episode + vector.upsert 绑起来。

---

## 2. 现有 memory 子系统 (战区 4 关联)

### 2.1 文件结构 (`crates/apeireth-memory/`)
```
Cargo.toml          2.3KB  — apeireth-core, apeireth-api, rusqlite 0.32, semantic feature
src/lib.rs         15.0KB  — 顶层 SqliteMemoryStore + StreamKind + MemoryError
src/append_only.rs 12.1KB  — 6 历史流 Append-only Log 实现 + 触发器
src/episode.rs     13.7KB  — EpisodeStore (按 session/continuity_id/time range)
src/identity.rs    19.9KB  — IdentityCardStore (continuity_id UNIQUE)
src/migrations.rs  13.1KB  — 6 流 schema + 6 触发器 + schema_migrations 表
src/session_note.rs 23.5KB — SessionStore + NoteStore
src/streams.rs     20.0KB  — 11 个流类型 (Thought/Proposal/Action/...)
src/history_streams.rs 6.5KB — 6 历史流深度 API
src/continuity_link.rs 6.5KB — 跨载体 link
src/llm_analysis.rs 3.8KB  — analyze_episode (LlmProvider 调用, 4 种 AnalysisKind)
src/three_layer.rs 20.5KB  — ThreeLayerMemory (R30 U9 claude-mem 3 层 facade)
extensions/        子 crate — InMemoryProvider / FileProvider / MongoDbProvider
benches/v2-memory-vector-bench.rs 5.1KB — **已经引用 `apeireth_memory::semantic::{EmbedFn, SemanticIndex}`**
tests/integration_six_streams.rs 12.7KB
tests/sqlite.rs 1.7KB
```

### 2.2 关键现状 ✅
- **SqliteMemoryStore 完整**: WAL + foreign_keys + synchronous=NORMAL;6 流 append-only (trigger 拒绝 UPDATE/DELETE);IdentityCard UNIQUE 跨载体;Episode 按 session/continuity_id 索引。
- **`semantic` feature 已声明**: `apeireth-vector = { optional = true }` + `semantic = ["apeireth-vector"]`,但 `src/semantic.rs` **不存在**!
- **benches 已写好但编译不过**: `v2-memory-vector-bench.rs` 第 15 行 import `apeireth_memory::semantic::{EmbedFn, SemanticIndex}`,但 `pub mod semantic;` 在 lib.rs 缺席 — bench 当前 0 编译通过状态。
- **`analyze_episode` LLM 调用已就绪**: 走 `Arc<dyn LlmProvider>`, 4 种 AnalysisKind (Summary/Keywords/RiskFlag/PhilosophyGate), 写好系统 prompt。
- **re-export 已就位**: `pub use apeireth_core;` / `pub use apeireth_core::Episode as CoreEpisode;` / `pub use apeireth_life_force::*;` — 给下游 1 行调用就行。
- **LOCKED 9 文件标注** (per lib.rs 注释): `append_only / identity / migrations / episode / session_note / streams / history_streams / continuity_link / llm_analysis` — 这 9 个**我 0 触碰**。

### 2.3 关键缺什么 ❌ (本次要补)
1. **`pub mod semantic;`**: 必须在 `lib.rs` 加上,然后新建 `src/semantic.rs` (或 `src/semantic/mod.rs`)。
2. **`EmbedFn` trait**: bench 已要求 `dim() -> usize` + `embed(&str) -> Vec<f32>`,我在 semantic.rs 里写。
3. **`SemanticIndex` struct + 3 个核心方法**: `new(memory, vector, embedder)` / `index_episode(ep)` / `search(query, k) -> Vec<Episode>` — bench 还要 `&mut SemanticIndex` 借用,意味着内部 `Box<dyn VectorStore>` 或 `&mut dyn VectorStore`。
4. **Memory::semantic_search(query, k)** 方法: 在 SqliteMemoryStore 上加 (面向 memory 用户的便捷 API),内部转给 SemanticIndex。
5. **UserProfile 类型 + extract_user_profile()**: 用户画像 (preferences/topics/style/recurring_questions),从 episodes + notes 聚合;**mock LLM call 写签名,真 LLM 集成留 trait 扩展**。

---

## 3. sqlite-vec 集成方案 (关键技术决策)

### 3.1 库选型: `sqlite-vec = "0.1"` 锁定
- **crates.io**: `sqlite-vec 0.1.9` (2026-07-05 发布)
- **依赖**: rusqlite ^0.31.0 (dev), cc ^1.0 (build) — API 应该向后兼容到 0.32
- **公开 API**: 仅一个 `unsafe fn sqlite3_vec_init()` — 注册成 SQLite auto-extension
- **License**: MIT OR Apache-2.0 (跟 workspace Apache-2.0 兼容)
- **跟现有栈完美契合**: 跟 workspace 锁的 rusqlite 0.32 (bundled) 共存,build.rs 用 cc 编译 C 源,不依赖系统 SQLite

### 3.2 集成方式 (选 `sqlite3_auto_extension` 路径)
**理由**:
- rusqlite 0.32 bundled 模式默认 **不** 编译 `SQLITE_ENABLE_LOAD_EXTENSION` 宏,所以 `Connection::enable_load_extension(true)` 路径会失败 (返回 "not authorized")。
- `sqlite3_auto_extension(Some(init_fn))` 是 SQLite C API 的另一条加载路径,在 `sqlite3_open` 时自动调,**不**需要 `SQLITE_ENABLE_LOAD_EXTENSION` 宏,bundled 模式下也能工作。
- 这种方式 `sqlite-vec` 的 C 扩展被**静态链接**到我们的可执行文件(通过 build.rs cc 编译),无运行时 .dll/.so 部署依赖。

**实施**:
```rust
// apeireth-vector/src/sqlite_vec_backend.rs (新文件)
use rusqlite::ffi::{sqlite3_auto_extension, SQLITE_EXTENSION_INIT};
use sqlite_vec::sqlite3_vec_init;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn install_auto_extension() {
    INIT.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    });
}
```
- `Once` 保证只在第一次 SqliteVecBackend::open() 时注册一次。
- `transmute` 把 `unsafe extern "C" fn()` 转成 `unsafe extern "C" fn(*mut c_void)` — 这是 SQLite C API 签名要求。

### 3.3 距离度量 (cosine 默认 + l2 可切)
- vec0 扩展 SQL 语法: `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[N] distance_metric=cosine)`
- 支持 `cosine` / `l2` / `l1` (具体看 0.1.9 支持哪些 — cosine + l2 是稳的)
- 我在 SqliteVecBackend 加 `enum DistanceMetric { Cosine, L2 }` + `set_metric(m: DistanceMetric)`,默认 Cosine。
- trait 上不暴露 metric (保持公共 API 形状不变, owner 自己选)。

### 3.4 维度配置 + batch
- 现有 trait 已有 `set_dimension` + `upsert_batch`,**接口不变**,只是在 backend 实现里改 SQL 为 vec0 语法。
- `upsert` 单条: `INSERT OR REPLACE INTO vec_items(rowid, embedding) VALUES (?, ?)`
- `search` top-k: `SELECT rowid, distance FROM vec_items WHERE embedding MATCH ? ORDER BY distance LIMIT k` (distance 字段直接由 vec0 算,不用 Rust 端再算)
- 距离 → 相似度映射: cosine distance ∈ [0, 2],我们转成 `score = 1.0 - distance/2.0` (cosine sim ∈ [-1, 1],但 sqlite-vec 实际返的是 0..2 还是别的我会在测试里验)。

### 3.5 元数据 (从 JSON blob → vec0 辅助列)
- vec0 扩展支持辅助列: `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[N], source TEXT, category INTEGER, +metadata_json TEXT)`
- 优点: `WHERE source = ? AND embedding MATCH ?` 可以走 vec0 内部预过滤,不用全表扫。
- 缺点: 改 schema 是**破坏性**的 — 已经用 BLOB 表的 db 升级不上去。
- **决策**: 保留旧 `vec_items` 表 schema 不变 (BLOB 余弦路径),新增 `vec_items_vec0` (vec0 路径) 通过 `set_metric` 切换;`is_vec0` 在 backend 状态里记录。这样既有数据不丢。
- 等等 — 这过度设计了。**简化方案**: `SqliteVecBackend` 内部**只走 vec0** (假设新装的 db 走 vec0),老的 brute-force 路径**保留为 `LegacyBLOBBackend`** 单独类型 (现在不用,留给老用户)。

但 `SqliteVecBackend` 名字在很多测试和 bench 里被引用。我**不改 trait、不改名字**,只在实现里把"用 BLOB 装 f32"换成"用 vec0 虚拟表",加载 auto_extension 后自动建表、查询 OK。

**最终方案**:
- `SqliteVecBackend` 内部改用 vec0 虚拟表: `CREATE VIRTUAL TABLE IF NOT EXISTS vec_items USING vec0(embedding float[N] distance_metric=cosine, metadata TEXT)`
- `id` 字段从 BLOB 改成 INTEGER primary key (vec0 rowid);但 trait `Vector` 用的是 `Uuid` — 我在 backend 内部维护一个 `id_map: HashMap<Uuid, i64>` 持久化在另一张普通表 `vec_idmap(uuid BLOB PK, rowid INTEGER UNIQUE)`,保持 trait 公共 API 形状不变。
- 这样所有 13 个 integration test 应该**不动**就能继续过(只换了底层实现,公共 API 一致)。

但要等 — 这是个大改。**先保底方案**: 14 个原 test 全部不动 + 加 10+ 新 vec0-specific test。底层细节通过 `install_auto_extension()` 透明生效。

### 3.6 性能预期
- vec0 扩展是 KNN 近似,10w 条 × 768 维应该 < 50ms (sqlite-vec 官方 benchmark)。
- 我跑 smoke example (`cargo run --example semantic_smoke`) 应该 p99 < 200ms (跟原版持平或更好)。

---

## 4. 跟 memory 怎么接 (本次 A3 阶段要做)

### 4.1 SemanticIndex 形状 (从 bench 反推)
```rust
// crates/apeireth-memory/src/semantic.rs (新)
use std::sync::Arc;
use apeireth_core::Episode;
use apeireth_vector::{VectorStore, Vector};
use uuid::Uuid;

/// 文本 → 向量 的 trait (mock LLM / 真 LLM 都实现)
pub trait EmbedFn: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// 内存 ↔ 向量 双存储 facade
pub struct SemanticIndex<'m> {
    memory: &'m SqliteMemoryStore,
    vector: Box<dyn VectorStore>,   // Box 因为 VectorStore trait object
    embedder: Arc<dyn EmbedFn>,
}

impl<'m> SemanticIndex<'m> {
    pub fn new(
        memory: &'m SqliteMemoryStore,
        vector: Box<dyn VectorStore>,
        embedder: Arc<dyn EmbedFn>,
    ) -> Self { ... }
    
    pub fn index_episode(&mut self, ep: &Episode) -> Result<(), MemoryError> { ... }
    pub fn search(&mut self, query: &str, k: usize) -> Result<Vec<Episode>, MemoryError> { ... }
}
```

### 4.2 Memory::semantic_search 便捷 API
```rust
impl SqliteMemoryStore {
    pub fn semantic_search(
        &self,
        query: &str,
        k: usize,
        embedder: &dyn EmbedFn,
    ) -> Result<Vec<Episode>, MemoryError> {
        // 1. embed query
        let query_vec = embedder.embed(query);
        // 2. 走临时 vec0 backend (in-memory)
        let mut vector = SqliteVecBackend::open_in_memory()?;
        vector.set_dimension(embedder.dim())?;
        // 3. 重建索引: 拉所有 episodes, embed & insert
        for ep in self.recent_episodes_all()? {
            let v = embedder.embed(&ep.content);
            vector.upsert(&Vector::new(ep_id_to_uuid(ep.id), v))?;
        }
        // 4. top-k 检索 → 回查 episode
        let hits = vector.search(&query_vec, k)?;
        // ... back-lookup episodes by uuid, 返 Vec<Episode>
    }
}
```
- **note**: 上面是简化版。生产里 SemanticIndex 应该**持久化**向量,不能每次都重 embed。我会让用户构造自己的 `SemanticIndex` 拿持久的 vec0 backend,`Memory::semantic_search` 留个一次性便捷版本。

### 4.3 extract_user_profile 形状
```rust
// crates/apeireth-memory/src/user_profile.rs (新)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub preferences: Vec<String>,         // 偏好的话题 / 风格
    pub recurring_topics: Vec<String>,    // 反复出现的主题
    pub communication_style: String,      // "直接"/"详细"/"轻松"...
    pub expertise_areas: Vec<String>,     // 用户擅长的领域
    pub interaction_count: usize,         // 总交互次数
    pub last_active: Option<i64>,         // 最后活跃时间
}

#[derive(Debug, Clone)]
pub struct ProfileExtractor {
    // mock LLM call 留接口;真接 LLM 用 apeireth_api::llm::LlmProvider
    embedder: Arc<dyn EmbedFn>,
    top_k: usize,
}

impl ProfileExtractor {
    pub fn new(embedder: Arc<dyn EmbedFn>) -> Self;
    pub fn extract(&self, memory: &SqliteMemoryStore) -> Result<UserProfile, MemoryError>;
}
```

- **mock LLM 路径**: 提取 top_k 个最相似 episodes,按 tag/role/keyword 聚合,生成 UserProfile (preferences = 最高频 tags, recurring_topics = 聚类, style = role 分布, etc.)
- **真 LLM 留接口**: 加一个 `LlmProvider` 字段 + 一个 `build_prompt(&UserProfile) -> String` 函数,真集成留到 R21+。

---

## 5. 硬约束核验 (本次严守)

| 约束 | 现状 | 本次动作 |
|---|---|---|
| 0 改 workspace.version (1.1.0) | Cargo.toml:246 = `"1.1.0"` | **不读不写这一行** |
| 0 改 R11 baseline 3 值 | `apeireth-asi/src/lib.rs` | **不读不写这一行** |
| 0 改 6 哲学锚定义 | `docs/conventions/09-anchor.md` | **不读不写** |
| 0 改 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 / 9 器官 | apeireth-asi / cognition / core | **不触碰** |
| 0 触碰 24 LOCKED | 见 R19 注释 | 24 个 crate 名单内 mtime **不动** |
| 0 主动 commit | git status | 等主人验收 |

`apeireth-vector` 不在 24 LOCKED 列表内 — 我可以自由改。
`apeireth-memory` **部分** LOCKED (9 个 src 文件被标 LOCKED): append_only / identity / migrations / episode / session_note / streams / history_streams / continuity_link / llm_analysis。
- `lib.rs` (本次加 `pub mod semantic;` + `pub use semantic::{EmbedFn, SemanticIndex, UserProfile, ProfileExtractor};`) **不算 LOCKED** (lib.rs 注释里 R23 P3 / R37-2 都改过它 +1 行 re-export,自由度高)。
- `semantic.rs` / `user_profile.rs` (新文件) **不算 LOCKED** (新加)。
- `llm_analysis.rs` 我**不碰** (已有 LLM 接口,我新文件用就行)。

---

## 6. 风险点 + 决策日志

| # | 风险 | 决策 | 备选 |
|---|---|---|---|
| R1 | sqlite-vec 0.1.9 build.rs 在 rusqlite 0.32 bundled 下链接失败 | 先试加 `sqlite-vec = "0.1"` 看 cargo check;失败 2 次就降级: 自己写 `install_auto_extension` + 在 runtime 检测 vec0 是否已注册,没就 fallback 到 BLOB 余弦 | 始终走 fallback (不做) |
| R2 | vec0 距离 → similarity 转换不对 | 写 5 个 unit test 验证 `score = 1.0 - distance/2.0` 跟原 brute-force 结果差 < 1e-5 | 改用 raw distance 当 score (破坏现有 search API) |
| R3 | `id_map` (Uuid ↔ rowid) 写满 vec0 表后 id 唯一性漂移 | 用 INTEGER PRIMARY KEY 自动递增 + UNIQUE 约束 | 让 vec0 接受 TEXT PK (可能要查 vec0 文档) |
| R4 | A2 cargo check 编译 > 2 次失败 | 切方案 R1 fallback | 暂停等 Mavis 拍板 |
| R5 | 主人 10:00 验收时还没全部跑通 | **A4 写清楚哪些完成 / 哪些 TODO**;不假装"complete" | 自己假设 OK |

---

## 7. 本阶段成果

- ✅ 完整摸清 vector / memory / sqlite-vec 现状
- ✅ 写出 1.3+ 3.5+ 13.8 KB 代码,理解每行
- ✅ 锁定 5 大技术决策: 库版本 / 加载方式 / 距离度量 / 维度策略 / 与 memory 集成形态
- ✅ 严守 6 项硬约束
- ✅ 风险表 R1-R5

**A1 阶段完成。开始 A2 阶段: 写代码。**
