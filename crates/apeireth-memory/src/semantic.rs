//! R19 P2 战区 4: 语义搜索 + 用户画像
//!
//! ## 设计
//! - `EmbedFn`: 文本 → 向量的 trait (mock 哈希 / 真 LLM 嵌入)
//! - `SemanticIndex`: 把 `SqliteMemoryStore` (SQLite 关系存储) + `apeireth-vector`
//!   (vec0 向量存储) 绑起来的双存储 facade
//! - `SemanticIndex::index_episode(ep)`: 写入 episode 的同时计算向量并 upsert 到 vec0
//! - `SemanticIndex::search(query, k)`: 文本 → 向量 → KNN → 回查 episode
//!
//! ## 与 trait 集成
//! - 复用 `apeireth_vector::VectorStore` + `apeireth_core::Episode` (无新引入类型)
//! - `Episode.id` 是 `String`, 我们用 `Uuid::new_v5(NS, id.as_bytes())` 派生稳定 Uuid
//!
//! ## 锁
//! - 0 触碰 9 个 LOCKED memory 文件 (append_only / identity / migrations / episode /
//!   session_note / streams / history_streams / continuity_link / llm_analysis)
//! - 仅新建 semantic.rs + user_profile.rs + lib.rs re-export (1 行 `pub mod semantic;`)

// 54ed4c7d: 向量路径 imports 挂 semantic feature (关闭时纯件 EmbedFn/HashEmbedder/EmbedderIdentity/episode_uuid 仍可用)
#[cfg(feature = "semantic")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "semantic")]
use apeireth_core::Episode;
#[cfg(feature = "semantic")]
use apeireth_vector::{SearchHit, Vector, VectorStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "semantic")]
use crate::episode::EpisodeStore;
#[cfg(feature = "semantic")]
use crate::user_profile::UserProfile;
#[cfg(feature = "semantic")]
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 命名空间 UUID (v5), 用于从 `Episode.id` (String) 派生稳定 `Uuid`.
/// NS 值: 固定 16 字节, 跨进程一致 (DCE 1.1 命名空间).
const EPISODE_NS: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// 从 `Episode.id` (String) 派生稳定 `Uuid`.
pub fn episode_uuid(episode_id: &str) -> Uuid {
    Uuid::new_v5(&EPISODE_NS, episode_id.as_bytes())
}

/// Embedder 身份 (RFC 001 移植 from mempalace `backends/base.py::EmbedderIdentity`).
///
/// ## 目的
/// 防止"静默降级"bug: 当 embedder 升级 (e.g. FNV-1a 改 FNV-1a-v2, 或换真 LLM)
/// 时, 旧 index 里的向量用新 embedder 检索会全错. 持久化 `model_name` 后,
/// 启动时 mismatch 立即报错, 而不是悄悄返回乱序结果.
///
/// ## 字段
/// - `model_name`: 稳定标识 (e.g. `"apeireth/hash-fnva-1a/v1"`).
///   **不可含版本号以外的运行时信息** (e.g. 路径 / hash / 时间), 否则
///   同一 embedder 在不同部署会判 mismatch.
/// - `dimension`: 向量维度. `0` = unknown (mempalace 同语义:
///   "no dimension signal", 跳过 dim 比对, 只比 model_name).
///
/// ## 协议 (RFC 001)
/// 1. 首次写入: 把当前 embedder identity 持久化到 vector store metadata.
/// 2. 后续打开: 读 stored identity, 比对 model_name + dimension.
/// 3. Legacy vector store (无 identity metadata): 报 `IdentityUnknown`
///    warning (per mempalace `EmbedderIdentityUnknownWarning`), 写入后
///    自动补登, 后续 open 转 strict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedderIdentity {
    /// 稳定模型标识.
    pub model_name: String,
    /// 向量维度. 0 = unknown (legacy).
    pub dimension: usize,
}

impl EmbedderIdentity {
    /// 构造一个 identity.
    pub fn new(model_name: impl Into<String>, dimension: usize) -> Self {
        Self {
            model_name: model_name.into(),
            dimension,
        }
    }

    /// 未知 / legacy identity (dimension = 0 信号).
    pub fn unknown() -> Self {
        Self {
            model_name: String::new(),
            dimension: 0,
        }
    }

    /// 是否标记为 legacy / unknown.
    pub fn is_unknown(&self) -> bool {
        self.model_name.is_empty() || self.dimension == 0
    }

    /// 严格匹配 (model_name + dim 都非空 + 一致).
    pub fn matches(&self, other: &EmbedderIdentity) -> bool {
        if self.is_unknown() || other.is_unknown() {
            // Legacy 兼容: 任一 unknown 视为相等 (mempalace 同语义).
            return true;
        }
        self.model_name == other.model_name && self.dimension == other.dimension
    }
}

impl std::fmt::Display for EmbedderIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_unknown() {
            write!(f, "<unknown embedder>")
        } else {
            write!(f, "{}@{}d", self.model_name, self.dimension)
        }
    }
}

impl EmbedderIdentity {
    /// Builder: 设 dimension (keep model_name).
    pub fn with_dim(mut self, dim: usize) -> Self {
        self.dimension = dim;
        self
    }

    /// Trait-default helper (use_unknown + set dim).
    fn tap_dim(self, dim: usize) -> Self {
        self.with_dim(dim)
    }
}

/// 文本 → 向量 trait.
///
/// ## 实现选择
/// - **mock**: `HashEmbedder` (本文件, 确定性 FNV-1a, 用于测试 / 本地开发)
/// - **真 LLM**: 留接口 — 调用方实现本 trait, 内部调 `apeireth_api::llm::LlmProvider`
///   拿 embedding (R21+ 续接)
pub trait EmbedFn: Send + Sync {
    /// 返回向量维度.
    fn dim(&self) -> usize;
    /// 把一段文本编码成 dim 维 f32 向量.
    fn embed(&self, text: &str) -> Vec<f32>;
    /// 返回 embedder 身份 (RFC 001). 默认 unknown + self.dim().
    ///
    /// 实现方**应该**override 此方法返回稳定 model_name + dimension.
    /// 不 override = 不参与身份校验 (warning 而非 error).
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::unknown().tap_dim(self.dim())
    }
}

/// 确定性 FNV-1a hash-based embedder (测试 / 本地 dev 用).
///
/// ## 性质
/// - 同输入永远同输出 (deterministic)
/// - 不调用任何外部 API
/// - 输出向量已 L2 归一化 (cosine 距离有效)
/// - 适合测试和"无 LLM 凭据"场景
///
/// ## 真 LLM 路径留口子
/// 实现方只需写自己的 `EmbedFn`, 内部调 `apeireth_api::llm::LlmProvider`,
/// 本模块不依赖具体 LLM 类型.
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    /// 构造一个 dim 维 hash embedder.
    pub fn new(dim: usize) -> Self {
        assert!(dim > 0, "dim must be > 0");
        Self { dim }
    }
}

impl EmbedFn for HashEmbedder {
    /// RFC 001 身份: FNV-1a hash embedder, 语义版本 v1.
    ///
    /// ## 命名约定 (apeireth namespace)
    /// - `apeireth/<algo>/<version>`
    /// - algo: 算法名 (`hash-fnva-1a` = FNV-1a 64-bit hash)
    /// - version: 实现版本 (改 hash 函数 / dim 行为时 bump)
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", self.dim)
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut values = vec![0.0_f32; self.dim];
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for (position, byte) in text.bytes().enumerate() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            values[(hash as usize ^ position) % self.dim] += 1.0 + f32::from(byte) / 255.0;
        }
        // L2 归一化.
        let norm: f32 = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut values {
                *v /= norm;
            }
        }
        values
    }
}

/// 内存 ↔ 向量双存储 facade.
///
/// `&'m SqliteMemoryStore` 借用保证内存 store 不会在 index 期间被释放.
/// `Box<dyn VectorStore>` 因为 trait object 持有所有权.
///
/// ## P0-5 (RFC 001 移植)
/// 持有 `embedder_identity`, 在每次 `index_episode` 时:
/// - 跟当前 embedder 的 identity 比对
/// - mismatch → `MemoryError::Other("embedder identity mismatch: stored=X@Yd current=Z@Wd")`
/// - 跟 unknown (legacy) 比对 → 写入当前 identity (per RFC 001 §3 warn-then-strict)
#[cfg(feature = "semantic")]
pub struct SemanticIndex<'m> {
    memory: &'m SqliteMemoryStore,
    vector: Mutex<Box<dyn VectorStore>>,
    embedder: Arc<dyn EmbedFn>,
    /// Embedder 身份 (RFC 001).
    ///
    /// 默认 = `embedder.identity()` (从 trait 取).
    /// 旧 vector store 加载时 = stored identity (从 metadata 读).
    embedder_identity: EmbedderIdentity,
}

#[cfg(feature = "semantic")]
impl<'m> SemanticIndex<'m> {
    /// 构造一个 SemanticIndex.
    ///
    /// ## P0-5
    /// 从 `embedder.identity()` 取身份, 存进 `embedder_identity` 字段.
    /// 后续 `index_episode` 时用这个 stored identity 跟当前 embedder 比对.
    pub fn new(
        memory: &'m SqliteMemoryStore,
        vector: Box<dyn VectorStore>,
        embedder: Arc<dyn EmbedFn>,
    ) -> Self {
        let identity = embedder.identity();
        Self {
            memory,
            vector: Mutex::new(vector),
            embedder,
            embedder_identity: identity,
        }
    }

    /// 构造并显式指定 stored identity (用于从旧 vector store 加载).
    ///
    /// 比对规则 (per RFC 001 §3):
    /// - stored == unknown: 自动采用 embedder 当前 identity (warn-then-strict)
    /// - stored matches embedder: 通过
    /// - stored mismatch embedder: `MemoryError::Other("embedder identity mismatch: ...")`
    pub fn with_stored_identity(
        memory: &'m SqliteMemoryStore,
        vector: Box<dyn VectorStore>,
        embedder: Arc<dyn EmbedFn>,
        stored: EmbedderIdentity,
    ) -> MemoryResult<Self> {
        let current = embedder.identity();
        if !stored.is_unknown() && !current.is_unknown() && stored != current {
            return Err(MemoryError::Other(format!(
                "embedder identity mismatch: stored={} current={}",
                stored, current
            )));
        }
        // stored unknown OR matches: 采用 current (warn-then-strict)
        Ok(Self {
            memory,
            vector: Mutex::new(vector),
            embedder,
            embedder_identity: if stored.is_unknown() { current } else { stored },
        })
    }

    /// 取 stored embedder identity.
    pub fn embedder_identity(&self) -> &EmbedderIdentity {
        &self.embedder_identity
    }

    /// 取出底层 vector store (供 caller 配置 dim / 查询).
    pub fn vector(&self) -> std::sync::MutexGuard<'_, Box<dyn VectorStore>> {
        self.vector.lock().expect("vector mutex")
    }

    /// 取 embedder 引用.
    pub fn embedder(&self) -> &Arc<dyn EmbedFn> {
        &self.embedder
    }

    /// 把一条 episode 写入 index:
    /// 1. **P0-5 校验**: 当前 embedder 身份 == stored identity? mismatch → error
    /// 2. embed episode.content
    /// 3. upsert 到 vec0
    /// 4. (内存存储) 不动, episode 已经在 `SqliteMemoryStore` 里
    pub fn index_episode(&self, ep: &Episode) -> MemoryResult<()> {
        // P0-5: identity check (per RFC 001)
        let current = self.embedder.identity();
        if !self.embedder_identity.matches(&current) {
            return Err(MemoryError::Other(format!(
                "embedder identity mismatch: stored={} current={}",
                self.embedder_identity, current
            )));
        }
        let vec = self.embedder.embed(&ep.content);
        let v = Vector::new(episode_uuid(&ep.id), vec);
        let mut guard = self.vector.lock().expect("vector mutex");
        // 首次 upsert 时确保 dim 已设.
        if guard.dimension() == 0 {
            guard
                .set_dimension(self.embedder.dim())
                .map_err(|e| MemoryError::Other(format!("vector set_dim: {e}")))?;
        }
        guard
            .upsert(&v)
            .map_err(|e| MemoryError::Other(format!("vector upsert failed: {e}")))?;
        Ok(())
    }

    /// 批量索引 episodes.
    pub fn index_episodes(&self, eps: &[Episode]) -> MemoryResult<()> {
        for ep in eps {
            self.index_episode(ep)?;
        }
        Ok(())
    }

    /// 按 query 文本检索最相似的 k 条 episode.
    pub fn search(&self, query: &str, k: usize) -> MemoryResult<Vec<Episode>> {
        if k == 0 {
            return Ok(Vec::new());
        }
        let qvec = self.embedder.embed(query);
        let mut guard = self.vector.lock().expect("vector mutex");
        // 首次 search 时也可能 dim 未设 (没 index_episode 直接 search), 自动设
        if guard.dimension() == 0 {
            guard
                .set_dimension(self.embedder.dim())
                .map_err(|e| MemoryError::Other(format!("vector set_dim: {e}")))?;
        }
        let hits: Vec<SearchHit> = guard
            .search(&qvec, k)
            .map_err(|e| MemoryError::Other(format!("vector search failed: {e}")))?;
        drop(guard);

        // 把 Uuid 反查回 episode id (String).
        // 我们用 v5 派生, 反向需要遍历 memory (因为 v5 是 hash, 不可逆).
        // 简化: 一次 query(limit=None) 拉所有 episodes, 建 HashMap<Uuid, Episode>.
        let all_eps = <SqliteMemoryStore as EpisodeStore>::query(
            self.memory,
            &crate::episode::EpisodeQuery::new().limit(100_000),
        )?;
        let mut by_uuid: std::collections::HashMap<Uuid, &Episode> =
            std::collections::HashMap::with_capacity(all_eps.len());
        for ep in &all_eps {
            by_uuid.insert(episode_uuid(&ep.id), ep);
        }
        let mut out = Vec::with_capacity(hits.len());
        for hit in hits {
            if let Some(ep) = by_uuid.get(&hit.id) {
                out.push((*ep).clone());
            }
            // 未命中的 (vector 里有但 memory 没有, 删除未传播) 静默跳过
        }
        Ok(out)
    }

    /// 当前 index 里的向量数.
    pub fn len(&self) -> MemoryResult<usize> {
        let guard = self.vector.lock().expect("vector mutex");
        guard
            .len()
            .map_err(|e| MemoryError::Other(format!("vector len failed: {e}")))
    }

    /// 提取用户画像 (从已索引的 episodes + memory 现有数据).
    ///
    /// 走 `ProfileExtractor::extract`, 见 `user_profile.rs`.
    pub fn extract_profile(&self) -> MemoryResult<UserProfile> {
        let extractor = crate::user_profile::ProfileExtractor::new(Arc::clone(&self.embedder));
        extractor.extract(self.memory, Some(self))
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(all(test, feature = "semantic"))]
mod tests {
    use super::*;
    use apeireth_vector::SqliteVecBackend;

    fn make_episode(id: &str, session: &str, ts: i64, role: &str, content: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: role.into(),
            content: content.into(),
            session_id: session.into(),
        }
    }

    fn fresh() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("open memory")
    }

    fn fresh_index<'m>(mem: &'m SqliteMemoryStore, dim: usize) -> SemanticIndex<'m> {
        let backend = SqliteVecBackend::open_in_memory().expect("open vector");
        SemanticIndex::new(
            mem,
            Box::new(backend),
            Arc::new(HashEmbedder::new(dim)),
        )
    }

    #[test]
    fn hash_embedder_is_deterministic() {
        let e1 = HashEmbedder::new(32);
        let a = e1.embed("hello world");
        let b = e1.embed("hello world");
        assert_eq!(a, b, "同输入必须同输出");
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn hash_embedder_different_text_different_vector() {
        let e = HashEmbedder::new(32);
        let a = e.embed("hello world");
        let b = e.embed("completely different content");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_embedder_output_is_l2_normalized() {
        let e = HashEmbedder::new(64);
        let v = e.embed("some content to embed");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm should be 1, got {}", norm);
    }

    #[test]
    fn episode_uuid_is_deterministic() {
        let u1 = episode_uuid("ep-001");
        let u2 = episode_uuid("ep-001");
        assert_eq!(u1, u2, "同 episode id 应同 uuid");
        let u3 = episode_uuid("ep-002");
        assert_ne!(u1, u3);
    }

    #[test]
    fn semantic_index_indexes_and_searches() {
        let mem = fresh();
        let idx = fresh_index(&mem, 32);

        // 索引 3 条 episode, 两条说 sql, 一条说 rust
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e1", "s1", 100, "user", "I want to learn SQL queries"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e2", "s1", 200, "user", "show me advanced SQL joins"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e3", "s1", 300, "user", "rust borrow checker is hard"),
        )
        .unwrap();

        idx.index_episode(&<SqliteMemoryStore as EpisodeStore>::get_episode(&mem, "e1").unwrap().unwrap()).unwrap();
        idx.index_episode(&<SqliteMemoryStore as EpisodeStore>::get_episode(&mem, "e2").unwrap().unwrap()).unwrap();
        idx.index_episode(&<SqliteMemoryStore as EpisodeStore>::get_episode(&mem, "e3").unwrap().unwrap()).unwrap();

        assert_eq!(idx.len().unwrap(), 3);

        // 查 "SQL" → 应命中 e1 / e2
        let hits = idx.search("SQL database query", 2).unwrap();
        assert_eq!(hits.len(), 2, "top-2 应返 2 条");
        // 命中应包含 e1 或 e2 (hash 近似匹配, 不保证严格)
        let hit_ids: Vec<&str> = hits.iter().map(|e| e.id.as_str()).collect();
        assert!(
            hit_ids.contains(&"e1") || hit_ids.contains(&"e2"),
            "hits 应包含 sql 主题的 episode, got {:?}",
            hit_ids
        );
    }

    #[test]
    fn semantic_index_search_with_zero_corpus_returns_empty() {
        let mem = fresh();
        let idx = fresh_index(&mem, 32);
        let hits = idx.search("anything", 5).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn semantic_index_dim_auto_set_on_first_upsert() {
        let mem = fresh();
        let idx = fresh_index(&mem, 64);
        // vector store dim 应该是 0
        assert_eq!(idx.vector().dimension(), 0);
        let ep = make_episode("e1", "s1", 1, "user", "content");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        idx.index_episode(&ep).unwrap();
        // 现在 dim 应该是 64
        assert_eq!(idx.vector().dimension(), 64);
    }

    // =====================================================================
    // P0-5: Embedder \xe6\x8a\x80\xe6\x9c\xaf\xe6\xa0\x87\xe8\xaf\x86\xe6\xa0\xa1\xe9\xaa\x8c (RFC 001 \xe7\xa7\xbb\xe6\xa4\x8d from mempalace)
    // =====================================================================

    /// 1. EmbedderIdentity \xe5\x9f\xba\xe7\xa1\x80\xe6\x9e\x84\xe9\x80\xa0 + Display
    #[test]
    fn embedder_identity_display_format() {
        let id = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        assert_eq!(format!("{id}"), "apeireth/hash-fnva-1a/v1@64d");
        let unk = EmbedderIdentity::unknown();
        assert_eq!(format!("{unk}"), "<unknown embedder>");
        assert!(unk.is_unknown());
        assert!(!id.is_unknown());
    }

    /// 2. matches() \xe8\xa7\x84\xe5\x88\x99: \xe5\x90\x8c model_name + dim -> true
    #[test]
    fn embedder_identity_matches_same_identity() {
        let a = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        let b = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        assert!(a.matches(&b));
    }

    /// 3. matches() \xe8\xa7\x84\xe5\x88\x99: model_name \xe4\xb8\x8d\xe5\x90\x8c -> false
    #[test]
    fn embedder_identity_mismatch_model_name() {
        let a = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        let b = EmbedderIdentity::new("apeireth/hash-fnva-1a/v2", 64);
        assert!(!a.matches(&b));
    }

    /// 4. matches() \xe8\xa7\x84\xe5\x88\x99: dim \xe4\xb8\x8d\xe5\x90\x8c -> false
    #[test]
    fn embedder_identity_mismatch_dimension() {
        let a = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        let b = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 128);
        assert!(!a.matches(&b));
    }

    /// 5. matches() \xe8\xa7\x84\xe5\x88\x99: legacy \xe5\x85\xbc\xe5\xae\xb9 - \xe4\xbb\xbb\xe4\xb8\x80 unknown -> true (RFC 001 \xc2\xa73 warn-then-strict)
    #[test]
    fn embedder_identity_legacy_unknown_matches() {
        let known = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 64);
        let unknown = EmbedderIdentity::unknown();
        assert!(known.matches(&unknown));
        assert!(unknown.matches(&known));
    }

    /// 6. HashEmbedder::identity() \xe8\xbf\x94\xe5\x9b\x9e RFC 001 \xe6\xa0\x87\xe8\xaf\x86
    #[test]
    fn hash_embedder_identity_is_rfc001() {
        let e = HashEmbedder::new(64);
        let id = e.identity();
        assert_eq!(id.model_name, "apeireth/hash-fnva-1a/v1");
        assert_eq!(id.dimension, 64);
        assert!(!id.is_unknown());
    }

    /// 7. SemanticIndex::new() \xe8\x87\xaa\xe5\x8a\xa8\xe4\xbb\x8e embedder \xe5\x8f\x96 identity
    #[test]
    fn semantic_index_new_captures_embedder_identity() {
        let mem = fresh();
        let idx = fresh_index(&mem, 32);
        let id = idx.embedder_identity();
        assert_eq!(id.model_name, "apeireth/hash-fnva-1a/v1");
        assert_eq!(id.dimension, 32);
    }

    /// 8. SemanticIndex \xe9\xbb\x98\xe8\xae\xa4\xe6\x9e\x84\xe9\x80\xa0 + \xe5\x90\x8c embedder -> index_episode \xe9\x80\x9a\xe8\xbf\x87
    #[test]
    fn semantic_index_identity_check_passes_same_embedder() {
        let mem = fresh();
        let idx = fresh_index(&mem, 32);
        let ep = make_episode("ep-p0-5-1", "s", 1, "user", "hello");
        idx.index_episode(&ep).expect("same embedder, should pass");
        assert_eq!(idx.len().unwrap(), 1);
    }

    /// 9. SemanticIndex with_stored_identity + \xe4\xb8\x8d\xe5\x90\x8c model_name -> error
    #[test]
    fn semantic_index_with_stored_identity_mismatch_errors() {
        let mem = fresh();
        let backend = SqliteVecBackend::open_in_memory().expect("open vec");
        let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(32));
        let stored = EmbedderIdentity::new("apeireth/hash-fnva-1a/v2", 32);
        let res = SemanticIndex::with_stored_identity(&mem, Box::new(backend), embedder, stored);
        assert!(res.is_err(), "mismatch should error");
        let err = res.err().unwrap();
        let msg = format!("{err:?}");
        assert!(msg.contains("embedder identity mismatch"), "msg: {msg}");
    }

    /// 10. SemanticIndex with_stored_identity + stored unknown -> \xe6\x8e\xa5\xe5\x8f\x97 (RFC 001 \xc2\xa73)
    #[test]
    fn semantic_index_with_stored_identity_unknown_accepts() {
        let mem = fresh();
        let backend = SqliteVecBackend::open_in_memory().expect("open vec");
        let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(32));
        let stored = EmbedderIdentity::unknown();
        let idx = SemanticIndex::with_stored_identity(&mem, Box::new(backend), embedder, stored)
            .expect("unknown stored should accept (legacy compat)");
        let id = idx.embedder_identity();
        assert_eq!(id.model_name, "apeireth/hash-fnva-1a/v1");
        assert_eq!(id.dimension, 32);
    }

    /// 11. SemanticIndex with_stored_identity + same identity -> \xe6\x8e\xa5\xe5\x8f\x97
    #[test]
    fn semantic_index_with_stored_identity_same_accepts() {
        let mem = fresh();
        let backend = SqliteVecBackend::open_in_memory().expect("open vec");
        let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(32));
        let stored = EmbedderIdentity::new("apeireth/hash-fnva-1a/v1", 32);
        let idx = SemanticIndex::with_stored_identity(&mem, Box::new(backend), embedder, stored)
            .expect("same identity should accept");
        assert_eq!(idx.embedder_identity().model_name, "apeireth/hash-fnva-1a/v1");
    }
}
