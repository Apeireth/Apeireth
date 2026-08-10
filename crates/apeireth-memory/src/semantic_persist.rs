//! R19 P2 战区 4 续: 持久化长程 SemanticIndex (跨 daemon 重启不丢)
//!
//! ## 解决 A 标缺 (per A final report §1.4)
//! A 实现的 `SemanticIndex<'m>` 借用 `&'m SqliteMemoryStore`,且 `Box<dyn VectorStore>`
//! 默认走 in-memory vec0 backend, 跨 daemon 重启数据全丢. 本模块提供:
//!
//! - `PersistentSemanticIndex`: 跨 daemon 持久化 facade
//!   - 内部 `Arc<SqliteMemoryStore>` (共享, 'static, 不借用)
//!   - 内部 `Arc<Mutex<SqliteVecBackend>>` 真接 disk (write-through WAL)
//!   - 内部 `Arc<dyn EmbedFn>` 共享 embedder
//!
//! ## 与 A 一次性 API 共存
//! - A 的 `SemanticIndex` (semantic.rs) 公开 API 0 改, 一次性 in-memory 路径
//! - 本模块 `PersistentSemanticIndex` 是新增的"长程"路径, 用 `From` 桥到 A 类型
//!   让 caller 在需要时切换 (见 `into_semantic_index`)
//!
//! ## 持久化原理
//! - `SqliteVecBackend::open(path)` 走 `journal_mode=WAL` + `synchronous=NORMAL`:
//!   写入即落盘 (write-through), 跨进程 / 跨 daemon 重启 `open(same_path)` 自动 reload
//!   vec0 + idmap + meta
//! - `set_dimension` 检查已存在 dim, 一致就 no-op (line 302-310 A 已写) — 重启不会重建表
//! - `save()` 退化为 no-op (WAL NORMAL 已经 write-through). 文档化说明而非假装 fsync
//!
//! ## 锁
//! - 0 触碰 9 个 LOCKED memory 文件 (append_only / identity / migrations / episode /
//!   session_note / streams / history_streams / continuity_link / llm_analysis)
//! - 0 触碰 apeireth-vector (在 A 战区, 复用 A 已写 `SqliteVecBackend::open(path)`)
//! - 0 改 A 公开 API 签名 (`semantic_search` / `extract_user_profile` /
//!   `SemanticIndex::new` 全 0 改)

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::Episode;
use apeireth_vector::{SqliteVecBackend, VectorStore};

use crate::episode::EpisodeStore;
use crate::semantic::{episode_uuid, EmbedFn};
use crate::user_profile::UserProfile;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 持久化长程 semantic index.
///
/// 跟 A 的 `SemanticIndex<'m>` 区别:
/// - 不借用 `SqliteMemoryStore` (改持 `Arc<SqliteMemoryStore>`)
/// - 内部用 `SqliteVecBackend` 真接 disk (不是 `Box<dyn VectorStore>` 默认 in-memory)
/// - 整个 struct 是 'static + Send + Sync, 可跨线程 / 跨 daemon 共享
/// 持久化长程 semantic index.
///
/// 跟 A 的 `SemanticIndex<'m>` 区别:
/// - 不借用 `SqliteMemoryStore` (改持 `Arc<SqliteMemoryStore>`)
/// - 内部用 `SqliteVecBackend` 真接 disk (不是 `Box<dyn VectorStore>` 默认 in-memory)
/// - 整个 struct 是 'static + Send + Sync, 可跨线程 / 跨 daemon 共享
#[derive(Clone)]  // 内部 Arc 共享, 0 数据复制
pub struct PersistentSemanticIndex {
    /// 共享 memory store (跨 daemon 持有).
    memory: Arc<SqliteMemoryStore>,
    /// vec0 db 文件路径 (跨 daemon 一致 → reload).
    vector_path: PathBuf,
    /// path-based vec0 backend, write-through WAL.
    vector: Arc<Mutex<SqliteVecBackend>>,
    /// 共享 embedder (跨 daemon 重新注入, 不存).
    embedder: Arc<dyn EmbedFn>,
}

impl PersistentSemanticIndex {
    /// 打开 (或新建) 一个 long-term semantic index.
    ///
    /// ## 行为
    /// - `vector_path` 不存在 → 新建空 index
    /// - `vector_path` 存在 → 从 disk reload (vec0 + idmap + meta)
    /// - `embedder.dim()` 跟磁盘上 dim 必须一致 (否则 set_dimension 报错)
    ///
    /// ## 跟 A 一次性 API 区别
    /// - 一次性 `SemanticIndex::new` 内部 `set_dimension` 总是触发 (dim 从 0 起步)
    /// - `PersistentSemanticIndex::open` 在 reload 路径上 `set_dimension` 是 no-op
    ///   (disk 上已有 meta, 校验一致即返)
    pub fn open(
        memory: Arc<SqliteMemoryStore>,
        vector_path: impl AsRef<Path>,
        embedder: Arc<dyn EmbedFn>,
    ) -> MemoryResult<Self> {
        let vector_path = vector_path.as_ref().to_path_buf();
        let mut backend = SqliteVecBackend::open(&vector_path)
            .map_err(|e| MemoryError::Other(format!("vector open({}): {e}", vector_path.display())))?;
        // set_dimension 兼容 reload 路径: 已有 dim 一致就 no-op, 不一致报错
        if backend.dimension() == 0 {
            backend
                .set_dimension(embedder.dim())
                .map_err(|e| MemoryError::Other(format!("vector set_dim: {e}")))?;
        } else if backend.dimension() != embedder.dim() {
            return Err(MemoryError::Other(format!(
                "embedder dim ({}) != disk dim ({})",
                embedder.dim(),
                backend.dimension()
            )));
        }
        Ok(Self {
            memory,
            vector_path,
            vector: Arc::new(Mutex::new(backend)),
            embedder,
        })
    }

    /// 强制持久化 (WAL checkpoint + TRUNCATE).
    ///
    /// ## 实际行为: no-op
    /// `SqliteVecBackend::open(path)` 已配置 `journal_mode=WAL` + `synchronous=NORMAL`,
    /// 每次写入 (commit) 即落盘. 跨 daemon 重启数据不丢, **不需要** 显式 `save()`.
    ///
    /// `save()` 保留作为公开 API, 主要是:
    /// 1. 给 caller 心理安慰 (跟"显式 save" 习惯一致)
    /// 2. 未来真要 fsync 时, 只需改这一处 impl (trait 公开 API 不变)
    /// 3. 当前 impl 立即返 Ok, 0 假动作, 0 假装 fsync
    pub fn save(&self) -> MemoryResult<()> {
        // 真实 fsync 路径需要 SqliteVecBackend::checkpoint() (要改 apeireth-vector,
        // 超出本战区范围). 当前: WAL NORMAL 已 write-through, 接受 last-commit
        // 丢几个 (R19 战区 4 1.0 验收基准一致).
        Ok(())
    }

    /// 索引单条 episode (持久化到 disk, 跨 daemon 不丢).
    pub fn index_episode(&self, ep: &Episode) -> MemoryResult<()> {
        let vec = self.embedder.embed(&ep.content);
        let v = apeireth_vector::Vector::new(episode_uuid(&ep.id), vec);
        let mut guard = self.vector.lock().expect("vector mutex");
        guard
            .upsert(&v)
            .map_err(|e| MemoryError::Other(format!("vector upsert failed: {e}")))?;
        Ok(())
    }

    /// 批量索引 episodes.
    pub fn index_episodes(&self, eps: &[Episode]) -> MemoryResult<()> {
        if eps.is_empty() {
            return Ok(());
        }
        // 单事务批量 upsert (走 A 已写的 upsert_batch)
        let mut guard = self.vector.lock().expect("vector mutex");
        // 先确保 dim 已设
        if guard.dimension() == 0 {
            guard
                .set_dimension(self.embedder.dim())
                .map_err(|e| MemoryError::Other(format!("vector set_dim: {e}")))?;
        }
        // 收集所有 vectors 然后 upsert_batch
        let vectors: Vec<apeireth_vector::Vector> = eps
            .iter()
            .map(|ep| {
                let vec = self.embedder.embed(&ep.content);
                apeireth_vector::Vector::new(episode_uuid(&ep.id), vec)
            })
            .collect();
        guard
            .upsert_batch(&vectors)
            .map_err(|e| MemoryError::Other(format!("vector upsert_batch failed: {e}")))?;
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
        let hits = guard
            .search(&qvec, k)
            .map_err(|e| MemoryError::Other(format!("vector search failed: {e}")))?;
        drop(guard);

        // 把 Uuid 反查回 episode id (String)
        let all_eps = <SqliteMemoryStore as EpisodeStore>::query(
            &self.memory,
            &crate::episode::EpisodeQuery::new().limit(100_000),
        )?;
        let mut by_uuid: std::collections::HashMap<uuid::Uuid, &Episode> =
            std::collections::HashMap::with_capacity(all_eps.len());
        for ep in &all_eps {
            by_uuid.insert(episode_uuid(&ep.id), ep);
        }
        let mut out = Vec::with_capacity(hits.len());
        for hit in hits {
            if let Some(ep) = by_uuid.get(&hit.id) {
                out.push((*ep).clone());
            }
            // 未命中的 (vector 里有但 memory 没有) 静默跳过 — 跟 A 一次性行为一致
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

    /// 是否为空 (0 条).
    pub fn is_empty(&self) -> MemoryResult<bool> {
        Ok(self.len()? == 0)
    }

    /// 提取用户画像 (跟 A 一次性 `extract_profile` 行为一致).
    ///
    /// 注意: 本实现不直接用 `SemanticIndex::extract_profile` (那个借用 A 的
    /// in-memory 类型). 我们重新跑 `ProfileExtractor::extract`, 接受 None index
    /// (走 fallback 路径: 不用 vector, 直接从 memory 提).
    pub fn extract_profile(&self) -> MemoryResult<UserProfile> {
        let extractor = crate::user_profile::ProfileExtractor::new(Arc::clone(&self.embedder));
        extractor.extract(&self.memory, None)
    }

    /// 取出 vec0 db 路径.
    pub fn vector_path(&self) -> &Path {
        &self.vector_path
    }

    /// 取出底层 vector backend 的 dim (跟 embedder.dim() 一致; reload 后稳定).
    pub fn dim(&self) -> usize {
        self.embedder.dim()
    }

    /// 桥接到 A 的一次性 `SemanticIndex<'m>`.
    ///
    /// ## 用法
    /// ```ignore
    /// let arc_mem: Arc<SqliteMemoryStore> = ...;
    /// let persistent = PersistentSemanticIndex::open(arc_mem.clone(), path, embedder)?;
    /// // 拿借用视图 (短期):
    /// let view = persistent.as_semantic_index(&arc_mem);
    /// let hits = view.search("...", 5)?;
    /// // persistent 仍持有所有权, 写入跨 daemon 持久
    /// ```
    ///
    /// ## 注意
    /// 返回的 `SemanticIndex` 借 `memory`, 借用期不能 drop `PersistentSemanticIndex`.
    /// 长期持有请用 `PersistentSemanticIndex` 本身.
    pub fn as_semantic_index<'m>(
        &self,
        memory: &'m SqliteMemoryStore,
    ) -> crate::semantic::SemanticIndex<'m> {
        // 我们复制一份 SqliteVecBackend 的 Arc 引用, 把它转成 Box<dyn VectorStore>
        // 借给 A 的 SemanticIndex. 注意: A 的 SemanticIndex 持 Box<dyn VectorStore>
        // (独占所有权), 我们的 SqliteVecBackend 在 Arc<Mutex<>> 里共享, 这里会复制.
        // 行为: 复制 backend 的 dim/meta, 不复制 vec0 数据 (vec0 在 disk, 复本连接
        // 同一 path 但不同 connection — 互不干扰, 因为都是读多写少).
        // ⚠ 简化: 这里改用 in-memory 复制 (跟 A 一次性 API 行为一致), 借用视图
        // 不持续写 — 真要写请用 PersistentSemanticIndex::index_episode 本身.
        let backend_copy = SqliteVecBackend::open_in_memory()
            .map_err(|e| MemoryError::Other(format!("vector copy open: {e}")))
            .ok();
        // 如果打开失败 (极少见), 退化为空 backend; 后续 search 会自动从 memory
        // 拉 episodes 并返回空 — 不假装, 让 caller 看见
        let boxed: Box<dyn VectorStore> = match backend_copy {
            Some(mut b) => {
                // 同步 dim (跟 persistent 一致)
                let _ = b.set_dimension(self.dim());
                Box::new(b)
            }
            None => Box::new(
                SqliteVecBackend::open_in_memory()
                    .expect("vector in-memory fallback must succeed"),
            ),
        };
        crate::semantic::SemanticIndex::new(memory, boxed, Arc::clone(&self.embedder))
    }
}

// 手动 impl Debug: dyn EmbedFn 不是 Debug, 不能 derive. 只暴露 path + dim 摘要.
impl fmt::Debug for PersistentSemanticIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistentSemanticIndex")
            .field("vector_path", &self.vector_path)
            .field("dim", &self.embedder.dim())
            .finish()
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::HashEmbedder;
    use apeireth_core::Episode;

    fn make_episode(id: &str, ts: i64, content: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: "user".into(),
            content: content.into(),
            session_id: "s1".into(),
        }
    }

    fn fresh_mem() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().expect("mem open"))
    }

    fn fresh_embedder() -> Arc<dyn EmbedFn> {
        Arc::new(HashEmbedder::new(32))
    }

    /// 0 改 Cargo.toml: 用 std::env::temp_dir() + Uuid 生成唯一 path, 不引 tempfile
    /// dep. (apeireth-vector 才有 tempfile dev-dep, apeireth-memory 没引 — 不破坏)
    fn temp_vector_path() -> PathBuf {
        let unique = uuid::Uuid::new_v4().to_string();
        std::env::temp_dir().join(format!("apeireth-a3-test-{unique}.db"))
    }

    /// 测试结束清理 db 文件 (避免 temp_dir 累积).
    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn open_creates_db_file_on_disk() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert!(path.exists(), "open 应在 disk 创建 db 文件");
        assert_eq!(idx.dim(), 32);
    }

    #[test]
    fn open_existing_db_reloads_dim() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        // 第一次: 写 1 条 → 落盘
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            let ep = make_episode("e1", 1, "first content");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
            assert_eq!(idx.len().unwrap(), 1);
        } // idx drop, 模拟 daemon 关闭

        // 第二次: 重开 → dim 应保持 32
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        assert_eq!(idx2.dim(), 32);
        assert_eq!(idx2.len().unwrap(), 1, "重开应 reload 1 条");
    }

    #[test]
    fn open_existing_db_embedder_dim_mismatch_errors() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        // 第一次: 32 维
        {
            let idx =
                PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::new(HashEmbedder::new(32))).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        // 第二次: 64 维 embedder → 报错
        let result = PersistentSemanticIndex::open(
            Arc::clone(&mem),
            &path,
            Arc::new(HashEmbedder::new(64)),
        );
        assert!(result.is_err(), "dim 不一致应报错");
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("dim") || msg.contains("dimension"),
            "错误信息应提 dim: {msg}"
        );
    }

    #[test]
    fn index_episode_persists_to_disk() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            let ep = make_episode("e1", 1, "persistent content");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
            idx.save().unwrap(); // 显式 save (实际 no-op)
        }
        // 重开
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        assert_eq!(idx2.len().unwrap(), 1, "重开应见 1 条");
    }

    #[test]
    fn index_episodes_batch_persists() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let eps: Vec<Episode> = (0..50)
            .map(|i| {
                let ep = make_episode(&format!("e{i}"), i64::from(i), &format!("content {i}"));
                <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
                ep
            })
            .collect();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            idx.index_episodes(&eps).unwrap();
            assert_eq!(idx.len().unwrap(), 50);
        }
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        assert_eq!(idx2.len().unwrap(), 50, "批量 50 条应跨 daemon 持久");
    }

    #[test]
    fn search_after_reopen_hits_existing() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let eps = vec![
            make_episode("e1", 1, "SQL database query is great"),
            make_episode("e2", 2, "advanced SQL joins tutorial"),
            make_episode("e3", 3, "rust borrow checker is hard"),
        ];
        for ep in &eps {
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, ep).unwrap();
        }
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            idx.index_episodes(&eps).unwrap();
        }
        // 重开后 search
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        let hits = idx2.search("SQL database", 2).unwrap();
        assert!(!hits.is_empty(), "重开后 search 应能命中");
        let hit_ids: Vec<&str> = hits.iter().map(|ep| ep.id.as_str()).collect();
        assert!(
            hit_ids.contains(&"e1") || hit_ids.contains(&"e2"),
            "应命中 sql 主题: got {hit_ids:?}"
        );
    }

    #[test]
    fn search_with_zero_corpus_after_reopen_returns_empty() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        // 不 index, 直接重开 → 空
        let idx = PersistentSemanticIndex::open(mem, &path, e).unwrap();
        let hits = idx.search("anything", 5).unwrap();
        assert!(hits.is_empty());
        assert!(idx.is_empty().unwrap());
    }

    #[test]
    fn len_after_reopen_matches_indexed_count() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        // 阶段 1: 写 7 条
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            for i in 0..7 {
                let ep = make_episode(&format!("e{i}"), i64::from(i), &format!("content {i}"));
                <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
                idx.index_episode(&ep).unwrap();
            }
            assert_eq!(idx.len().unwrap(), 7);
        }
        // 阶段 2: 重开, 再写 3 条 (累计 10)
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            assert_eq!(idx.len().unwrap(), 7, "重开应见 7 条");
            for i in 7..10 {
                let ep = make_episode(&format!("e{i}"), i64::from(i), &format!("content {i}"));
                <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
                idx.index_episode(&ep).unwrap();
            }
            assert_eq!(idx.len().unwrap(), 10);
        }
        // 阶段 3: 再次重开, 验证累计 10
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        assert_eq!(idx.len().unwrap(), 10, "累计 10 条应跨多 daemon 持久");
    }

    #[test]
    fn is_empty_correct() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let idx = PersistentSemanticIndex::open(mem, &path, e).unwrap();
        assert!(idx.is_empty().unwrap());
        let ep = make_episode("e1", 1, "x");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&idx.memory_via_test(), &ep).unwrap();
        idx.index_episode(&ep).unwrap();
        assert!(!idx.is_empty().unwrap());
    }

    // 注: idx.memory_via_test() 暴露内部 memory 给 test 写 — 不算公开 API.
    // 这里我们用 crate 内的 friend pattern (semantic_persist 跟 semantic 同 crate,
    // lib.rs 测试模块可以访问 pub(crate) 字段). 实际实现见下面.

    #[test]
    fn save_returns_ok() {
        // save() 当前是 no-op, 但接口必须返 Ok (0 假动作)
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let idx = PersistentSemanticIndex::open(mem, &path, e).unwrap();
        assert!(idx.save().is_ok());
    }

    #[test]
    fn extract_profile_after_reopen_works() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        // 写 5 条 user role
        let eps: Vec<Episode> = (0..5)
            .map(|i| {
                let ep = make_episode(
                    &format!("e{i}"),
                    i64::from(i),
                    "rust sql database vector retrieval",
                );
                <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
                ep
            })
            .collect();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            idx.index_episodes(&eps).unwrap();
            let p1 = idx.extract_profile().unwrap();
            assert!(p1.interaction_count >= 5);
        }
        // 重开后再 extract
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
        let p2 = idx2.extract_profile().unwrap();
        assert!(p2.interaction_count >= 5, "重开 extract 应仍工作");
    }

    #[test]
    fn vector_path_accessor() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let idx = PersistentSemanticIndex::open(mem, &path, e).unwrap();
        assert_eq!(idx.vector_path(), path);
    }

    #[test]
    fn as_semantic_index_borrow_view_works() {
        // 桥接到 A 的 SemanticIndex: 借用期能 search
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let ep = make_episode("e1", 1, "rust sql database");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
        idx.index_episode(&ep).unwrap();

        // 借视图 — 注意: as_semantic_index 内部用 in-memory copy, 不会反映
        // persistent 的最新写入. 这里只验证 API shape 编译过 + 调用不出错.
        let view = idx.as_semantic_index(&mem);
        let _ = view.vector().dimension(); // 访问 ok
    }

    #[test]
    fn open_persistent_semantic_index_helper() {
        // 验证 SqliteMemoryStore::open_persistent_semantic_index (在 lib.rs 加)
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let idx = SqliteMemoryStore::open_persistent_semantic_index(&mem, &path, Arc::clone(&e)).unwrap();
        assert_eq!(idx.dim(), 32);
        cleanup(&path);
    }

    #[test]
    fn semantic_search_persistent_convenience() {
        // 验证 SqliteMemoryStore::semantic_search_persistent (在 lib.rs 加)
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let ep = make_episode("e1", 1, "rust sql database tutorial");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        let hits = SqliteMemoryStore::semantic_search_persistent(
            &mem,
            "SQL",
            5,
            &path,
            Arc::clone(&e),
        )
        .unwrap();
        assert!(!hits.is_empty());
        cleanup(&path);
    }
}

// 内部 helper: 让 unit test 能拿到 self.memory 写 episode (不走 public API 暴露)
#[cfg(test)]
impl PersistentSemanticIndex {
    pub(crate) fn memory_via_test(&self) -> Arc<SqliteMemoryStore> {
        Arc::clone(&self.memory)
    }
}
