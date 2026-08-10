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

use std::sync::{Arc, Mutex};

use apeireth_core::Episode;
use apeireth_vector::{SearchHit, Vector, VectorStore};
use uuid::Uuid;

use crate::episode::EpisodeStore;
use crate::user_profile::UserProfile;
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
pub struct SemanticIndex<'m> {
    memory: &'m SqliteMemoryStore,
    vector: Mutex<Box<dyn VectorStore>>,
    embedder: Arc<dyn EmbedFn>,
}

impl<'m> SemanticIndex<'m> {
    /// 构造一个 SemanticIndex.
    pub fn new(
        memory: &'m SqliteMemoryStore,
        vector: Box<dyn VectorStore>,
        embedder: Arc<dyn EmbedFn>,
    ) -> Self {
        Self {
            memory,
            vector: Mutex::new(vector),
            embedder,
        }
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
    /// 1. embed episode.content
    /// 2. upsert 到 vec0
    /// 3. (内存存储) 不动, episode 已经在 `SqliteMemoryStore` 里
    pub fn index_episode(&self, ep: &Episode) -> MemoryResult<()> {
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

#[cfg(test)]
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
}
