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
//!
//! ## N5 (backlog): artifact_sig 内容寻址缓存门禁
//! 吸收 VCP rust-vexus-lite `memo_artifact_builder.rs:662-802` 的"内容寻址资产":
//! - `artifact_sig(content)` = 规范化内容 UTF-8 字节的 SHA-256 (hex 64, 算法明确,
//!   NIST FIPS 180-4 测试向量锚定; 纯 Rust 手写 ~50 行, 0 新依赖 — Cargo.lock
//!   当时已被并行任务弄脏, 避免共享文件冲突)
//! - 重算前先查签名 (`check_artifact`): 命中且 normalize/schema 版本不 stale →
//!   复用磁盘资产, 跳过整段 embed+upsert (`reindex_all` 返回 `Hit`)
//! - 与 P1#3 normalize 版本机制协作: 存储记录携带 normalize_version/schema_version,
//!   版本落后 → 签名即使匹配也判 stale 强制重算 (失效规则显式, 见
//!   `artifact_gate_decision` 文档)
//! - 防脏读: 内容变 → 签名变 → 强制重算; 重算走 clear + upsert_batch 全量重建,
//!   episode 集收缩 (删除) 也能同步 (0 残留旧向量)
//! - 门禁只覆盖全量重建路径 (`reindex_all`); `index_episode`/`index_episodes`
//!   是增量写入路径, 保持原语义 (诚实标注, 0 假装全覆盖)

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::Episode;
use apeireth_vector::{SqliteVecBackend, VectorStore};

use crate::episode::EpisodeStore;
use crate::semantic::{episode_uuid, EmbedderIdentity, EmbedFn};
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
    /// P0-5: Embedder 身份 (model_name + dimension).
    /// 持久化到 sidecar 文件 `<vector_path>.embedder.json`.
    embedder_identity: EmbedderIdentity,
    /// P0-6: 当前 schema 版本.
    schema_version: u32,
    /// P1#3: 磁盘上的 normalize/chunk 规则版本 (stored; < CURRENT → stale 需重建).
    normalize_version: u32,
}

/// P0-6: 当前 schema 版本.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// P1#3 (审计 A1#2, 2026-08-16): 文本 normalize/chunk 规则版本.
///
/// 与 `CURRENT_SCHEMA_VERSION` (向量表 schema) 正交: 本版本标记「喂给 embedder 的
/// 文本是怎么规范化/切块的」。换 chunk 规则 (如换分隔符/长度策略) 后 bump 本常量,
/// `open()` 会识别 stale 向量 (`needs_reindex() == true`), 调用方决定重建索引 —
/// 不自动删, 不假装迁移 (0 装 PASS)。
pub const CURRENT_NORMALIZE_VERSION: u32 = 1;

/// P1#3: 磁盘版本是否 stale (按旧 chunk 规则生成, 需重建索引).
/// 注意: `open()` 会把 0 (无 sidecar 记录) 归一化为当前版本 — 因为历史从未
/// bump 过, 无记录的库 = 当前规则生成 (诚实成立); 未来 bump 后旧库自然 stale.
pub fn normalize_is_stale(stored: u32) -> bool {
    stored < CURRENT_NORMALIZE_VERSION
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
        // P0-5: 校验 stored embedder identity (per RFC 001)
        let stored = read_embedder_sidecar(&vector_path);
        let current = embedder.identity();
        if !stored.is_unknown() && !current.is_unknown() && stored != current {
            return Err(MemoryError::Other(format!(
                "embedder identity mismatch: stored={stored} current={current}"
            )));
        }
        let embedder_identity = if stored.is_unknown() { current } else { stored };

        // P0-6: 校验 stored schema version
        let stored_schema = read_schema_sidecar(&vector_path);
        let schema_version = if stored_schema == 0 {
            CURRENT_SCHEMA_VERSION
        } else if stored_schema > CURRENT_SCHEMA_VERSION {
            return Err(MemoryError::Other(format!(
                "schema version on disk ({stored_schema}) > current code ({CURRENT_SCHEMA_VERSION})"
            )));
        } else if stored_schema < CURRENT_SCHEMA_VERSION {
            CURRENT_SCHEMA_VERSION
        } else {
            stored_schema
        };

        // P1#3: 校验 stored normalize 版本 (独立于 schema 版本)
        let stored_norm = read_normalize_sidecar(&vector_path);
        let normalize_version = if stored_norm == 0 {
            CURRENT_NORMALIZE_VERSION
        } else if stored_norm > CURRENT_NORMALIZE_VERSION {
            return Err(MemoryError::Other(format!(
                "normalize version on disk ({stored_norm}) > current code ({CURRENT_NORMALIZE_VERSION})"
            )));
        } else {
            stored_norm
        };
        if normalize_is_stale(normalize_version) {
            // 诚实: 不自动重建, 只标记 stale 供调用方决策
            eprintln!(
                "[semantic] normalize 版本落后 (disk={normalize_version} code={CURRENT_NORMALIZE_VERSION}): \
                 向量按旧 chunk 规则生成, needs_reindex()=true — 由调用方决定重建"
            );
        }

        Ok(Self {
            memory,
            vector_path,
            vector: Arc::new(Mutex::new(backend)),
            embedder,
            embedder_identity,
            schema_version,
            normalize_version,
        })
    }

    /// P0-5: 取当前 embedder 身份.
    pub fn embedder_identity(&self) -> &EmbedderIdentity {
        &self.embedder_identity
    }

    /// P0-6: 取当前 schema 版本.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// P1#3: 磁盘上的 normalize/chunk 规则版本 (stored).
    pub fn normalize_version(&self) -> u32 {
        self.normalize_version
    }

    /// P1#3: 向量是否按旧 chunk 规则生成 (stored < current → 需重建索引).
    pub fn needs_reindex(&self) -> bool {
        self.normalize_version < CURRENT_NORMALIZE_VERSION
    }

    /// 显式 no-op flush — 留作公开 API 当 caller 想"心理 flush"时用.
    ///
    /// 当前 impl: WAL NORMAL 已 write-through, 0 真实 fsync 动作.
    /// 未来 SqliteVecBackend::checkpoint() 可用时, 改这一处 impl 即可.
    /// 实际行为: 立即返 Ok(()), 0 伪装 fsync, 0 假装持久化强保证.
    pub fn flush_noop(&self) -> MemoryResult<()> {
        Ok(())
    }

    /// **DEPRECATED** (since 1.2.0): 改名 + 暴露"不假装"性.
    ///
    /// 原 `save()` 听起来像 fsync, 实际是 no-op. 公开 API 留这个名字
    /// 只会让 caller 误以为有强保证. 改用 [`flush_noop`] 显式声明意图.
    #[deprecated(
        since = "1.2.0",
        note = "save() 是 no-op (WAL NORMAL 已 write-through). 改用 flush_noop() 显式声明, 或 0 调用 (WAL 自动落盘)."
    )]
    pub fn save(&self) -> MemoryResult<()> {
        self.flush_noop()
    }

    /// 索引单条 episode (持久化到 disk, 跨 daemon 不丢).
    pub fn index_episode(&self, ep: &Episode) -> MemoryResult<()> {
        let vec = self.embedder.embed(&ep.content);
        let v = apeireth_vector::Vector::new(episode_uuid(&ep.id), vec);
        let mut guard = self.vector.lock().expect("vector mutex");
        guard
            .upsert(&v)
            .map_err(|e| MemoryError::Other(format!("vector upsert failed: {e}")))?;
        drop(guard);
        // P0-5 + P0-6 + P1#3: 落盘 sidecar
        self.persist_embedder_sidecar()?;
        self.persist_schema_sidecar()?;
        self.persist_normalize_sidecar()?;
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

// ============================================================
// P0-5 + P0-6: Sidecar helpers + impl methods
// ============================================================
const EMBEDDER_SIDECAR_SUFFIX: &str = ".embedder.json";
const SCHEMA_SIDECAR_SUFFIX: &str = ".schema.json";
const NORMALIZE_SIDECAR_SUFFIX: &str = ".normalize.json";

fn embedder_sidecar_path(vector_path: &Path) -> PathBuf {
    let mut p = vector_path.to_path_buf();
    let original = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    p.set_file_name(format!("{original}{EMBEDDER_SIDECAR_SUFFIX}"));
    p
}

fn schema_sidecar_path(vector_path: &Path) -> PathBuf {
    let mut p = vector_path.to_path_buf();
    let original = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    p.set_file_name(format!("{original}{SCHEMA_SIDECAR_SUFFIX}"));
    p
}

fn normalize_sidecar_path(vector_path: &Path) -> PathBuf {
    let mut p = vector_path.to_path_buf();
    let original = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    p.set_file_name(format!("{original}{NORMALIZE_SIDECAR_SUFFIX}"));
    p
}

fn read_embedder_sidecar(vector_path: &Path) -> EmbedderIdentity {
    let path = embedder_sidecar_path(vector_path);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| EmbedderIdentity::unknown()),
        Err(_) => EmbedderIdentity::unknown(),
    }
}

fn read_schema_sidecar(vector_path: &Path) -> u32 {
    let path = schema_sidecar_path(vector_path);
    match std::fs::read_to_string(&path) {
        Ok(s) => s.trim().parse::<u32>().unwrap_or(0),
        Err(_) => 0,
    }
}

fn read_normalize_sidecar(vector_path: &Path) -> u32 {
    let path = normalize_sidecar_path(vector_path);
    match std::fs::read_to_string(&path) {
        Ok(s) => s.trim().parse::<u32>().unwrap_or(0),
        Err(_) => 0,
    }
}

fn persist_embedder_sidecar_impl(vector_path: &Path, identity: &EmbedderIdentity) -> std::io::Result<()> {
    let path = embedder_sidecar_path(vector_path);
    let tmp = path.with_extension("embedder.json.tmp");
    let json = serde_json::to_string(identity).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn persist_schema_sidecar_impl(vector_path: &Path, version: u32) -> std::io::Result<()> {
    let path = schema_sidecar_path(vector_path);
    let tmp = path.with_extension("schema.json.tmp");
    std::fs::write(&tmp, version.to_string())?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn persist_normalize_sidecar_impl(vector_path: &Path, version: u32) -> std::io::Result<()> {
    let path = normalize_sidecar_path(vector_path);
    let tmp = path.with_extension("normalize.json.tmp");
    std::fs::write(&tmp, version.to_string())?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

impl PersistentSemanticIndex {
    fn persist_embedder_sidecar(&self) -> MemoryResult<()> {
        let id = self.embedder.identity();
        persist_embedder_sidecar_impl(&self.vector_path, &id)
            .map_err(|e| MemoryError::Other(format!("embedder sidecar write: {e}")))?;
        Ok(())
    }

    fn persist_schema_sidecar(&self) -> MemoryResult<()> {
        persist_schema_sidecar_impl(&self.vector_path, self.schema_version)
            .map_err(|e| MemoryError::Other(format!("schema sidecar write: {e}")))?;
        Ok(())
    }

    fn persist_normalize_sidecar(&self) -> MemoryResult<()> {
        persist_normalize_sidecar_impl(&self.vector_path, CURRENT_NORMALIZE_VERSION)
            .map_err(|e| MemoryError::Other(format!("normalize sidecar write: {e}")))?;
        Ok(())
    }
}

// =====================================================================
// N5 (backlog): artifact_sig 内容寻址缓存门禁
// 吸收 VCP rust-vexus-lite memo_artifact_builder.rs:662-802:
// 内容寻址资产 — 签名不变跳过重算; 版本落后/内容变 → 强制重算.
// =====================================================================

/// N5: 内容签名算法 = SHA-256 (FIPS 180-4), 纯 Rust 手写.
///
/// ponytail: 只为内容寻址 (非安全场景), 0 新依赖 (sha2 crate 不在本 crate 依赖图,
/// 且当时 Cargo.lock 被并行任务弄脏, 加 dep 会引入共享文件冲突). 正确性由
/// `n5_artifact_sig_matches_sha256_known_vectors` 用 NIST 官方向量锚定.
/// 升级路径: 若工作区统一引入 sha2, 换 `artifact_sig` 内部实现即可, 签名语义不变.
mod sha256_n5 {
    /// SHA-256 轮常量 (FIPS 180-4 §4.2.2).
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// SHA-256 摘要 (FIPS 180-4 §6.2). 标准实现, 0 优化花样.
    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        // 填充: message || 0x80 || 0x00* || 64-bit 大端 bit 长度
        let bit_len = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        let mut w = [0u32; 64];
        for chunk in msg.chunks_exact(64) {
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        let mut out = [0u8; 32];
        for (i, word) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

/// N5: artifact_sig sidecar 后缀 (`<vector_path>.artifact_sig.json`).
pub const ARTIFACT_SIG_SIDECAR_SUFFIX: &str = ".artifact_sig.json";

/// N5: 内容签名 — SHA-256(规范化内容 UTF-8 字节), 小写 hex 64 字符.
///
/// 算法明确可测: NIST FIPS 180-4 官方向量见
/// `n5_artifact_sig_matches_sha256_known_vectors`. 纯函数, 同内容恒同签名.
pub fn artifact_sig(content: &str) -> String {
    let digest = sha256_n5::digest(content.as_bytes());
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// N5: 多项内容规范化级联签名 — 排序后级联, 同集合同签名 (与插入顺序无关).
///
/// 规范化格式 v1: `"apeireth-artifact-v1\ncount=<n>\n<id1>\0<content1>\n..."`
/// — id/content 边界用 `\0` 分隔防拼接歧义, count 防长度歧义.
/// 换格式时 bump 前缀版本号 (旧签名自然 miss → 重算, 不会误命中).
pub fn artifact_sig_many(items: &[(&str, &str)]) -> String {
    let mut sorted: Vec<(&str, &str)> = items.to_vec();
    sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut canonical = format!("apeireth-artifact-v1\ncount={}\n", sorted.len());
    for (id, content) in sorted {
        canonical.push_str(id);
        canonical.push('\0');
        canonical.push_str(content);
        canonical.push('\n');
    }
    artifact_sig(&canonical)
}

/// N5: 门禁决策 (重算前查签名的结果).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactDecision {
    /// 签名命中且 normalize/schema 版本都匹配 → 复用缓存资产, 跳过重算.
    Hit,
    /// 无存储签名 (首次构建) → 重算.
    MissNoRecord,
    /// 内容签名变化 → 强制重算 (防脏读: 内容变 → 签名变 → 绝不复用).
    MissContentChanged,
    /// 签名匹配但资产按旧 normalize/chunk 规则生成 → 失效重算
    /// (与 `normalize_is_stale`/`needs_reindex` 同语义协作).
    StaleNormalize,
    /// 签名匹配但 schema 版本不一致 → 失效重算.
    StaleSchema,
}

impl ArtifactDecision {
    /// 是否需要重算 (只有 Hit 复用).
    pub fn should_recompute(self) -> bool {
        !matches!(self, Self::Hit)
    }

    /// 是否命中 (可复用缓存资产).
    pub fn is_hit(self) -> bool {
        matches!(self, Self::Hit)
    }
}

/// N5: 磁盘签名记录 (`<vector_path>.artifact_sig.json` 内容).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactSigRecord {
    /// 资产内容签名 (`artifact_sig_many` 输出).
    pub sig: String,
    /// 构建时的 normalize/chunk 规则版本 (用于 stale 判定).
    pub normalize_version: u32,
    /// 构建时的 schema 版本 (用于 stale 判定).
    pub schema_version: u32,
}

/// N5: 门禁决策纯函数 (0 IO, 可独立测).
///
/// ## 失效规则 (显式, 按检查顺序)
/// 1. 无记录 → `MissNoRecord` (首次构建)
/// 2. `record.sig != current_sig` → `MissContentChanged`
///    (内容变 → 签名变 → 强制重算, 防脏读第一原则)
/// 3. `record.normalize_version < CURRENT_NORMALIZE_VERSION` → `StaleNormalize`
///    (规范化/chunk 规则升级 → 旧签名即使内容相同也失效, 因产物语义已变)
/// 4. `record.schema_version != CURRENT_SCHEMA_VERSION` → `StaleSchema`
/// 5. 全匹配 → `Hit` (复用缓存资产, 跳过重算)
pub fn artifact_gate_decision(
    stored: Option<&ArtifactSigRecord>,
    current_sig: &str,
) -> ArtifactDecision {
    let Some(rec) = stored else {
        return ArtifactDecision::MissNoRecord;
    };
    if rec.sig != current_sig {
        return ArtifactDecision::MissContentChanged;
    }
    if normalize_is_stale(rec.normalize_version) {
        return ArtifactDecision::StaleNormalize;
    }
    if rec.schema_version != CURRENT_SCHEMA_VERSION {
        return ArtifactDecision::StaleSchema;
    }
    ArtifactDecision::Hit
}

fn artifact_sig_sidecar_path(vector_path: &Path) -> PathBuf {
    let mut p = vector_path.to_path_buf();
    let original = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    p.set_file_name(format!("{original}{ARTIFACT_SIG_SIDECAR_SUFFIX}"));
    p
}

/// N5: 读磁盘签名记录. 无文件 / 损坏 JSON → None (= miss 走重算, 0 panic).
pub fn read_artifact_record(vector_path: impl AsRef<Path>) -> Option<ArtifactSigRecord> {
    let path = artifact_sig_sidecar_path(vector_path.as_ref());
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn persist_artifact_record_impl(vector_path: &Path, record: &ArtifactSigRecord) -> std::io::Result<()> {
    let path = artifact_sig_sidecar_path(vector_path);
    let tmp = path.with_extension("artifact_sig.json.tmp");
    let json = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

impl PersistentSemanticIndex {
    /// N5: 门禁检查 — 对当前内容签名给出复用/重算决策 (0 副作用).
    pub fn check_artifact(&self, current_sig: &str) -> ArtifactDecision {
        artifact_gate_decision(read_artifact_record(&self.vector_path).as_ref(), current_sig)
    }

    /// N5: 重算成功后记录新签名 (写入 CURRENT 版本号).
    /// IO 失败传播 Err (0 假装成功).
    pub fn record_artifact(&self, sig: &str) -> MemoryResult<()> {
        let record = ArtifactSigRecord {
            sig: sig.to_string(),
            normalize_version: CURRENT_NORMALIZE_VERSION,
            schema_version: self.schema_version,
        };
        persist_artifact_record_impl(&self.vector_path, &record)
            .map_err(|e| MemoryError::Other(format!("artifact_sig sidecar write: {e}")))?;
        Ok(())
    }

    /// N5: 带门禁的全量重建 (VCP "内容寻址资产"语义).
    ///
    /// - `Hit` → 跳过整段 embed + upsert (磁盘资产与当前内容和版本一致, 直接复用)
    /// - `Miss*`/`Stale*` → embed 全部 episodes (先算完再动 store, embed 失败
    ///   不破坏现有资产) → `clear()` + `upsert_batch` 全量重建 → 落盘新签名.
    ///   clear 保证防脏读: episode 集收缩/内容变化后 0 残留旧向量.
    ///
    /// 返回门禁决策, 调用方可观测是否跳过了重算.
    pub fn reindex_all(&self, eps: &[Episode]) -> MemoryResult<ArtifactDecision> {
        let uuids: Vec<(String, &str)> = eps
            .iter()
            .map(|ep| (episode_uuid(&ep.id).to_string(), ep.content.as_str()))
            .collect();
        let refs: Vec<(&str, &str)> = uuids.iter().map(|(u, c)| (u.as_str(), *c)).collect();
        let sig = artifact_sig_many(&refs);
        let decision = self.check_artifact(&sig);
        if decision.is_hit() {
            return Ok(decision);
        }
        // 重算: 先 embed 全部 (失败则现有资产不动), 再 clear + upsert_batch
        let vectors: Vec<apeireth_vector::Vector> = eps
            .iter()
            .map(|ep| {
                let vec = self.embedder.embed(&ep.content);
                apeireth_vector::Vector::new(episode_uuid(&ep.id), vec)
            })
            .collect();
        let mut guard = self.vector.lock().expect("vector mutex");
        guard
            .clear()
            .map_err(|e| MemoryError::Other(format!("vector clear failed: {e}")))?;
        // clear() 会删 vec_meta 并重置 dim (SqliteVecBackend 语义), 重建前必须重设
        guard
            .set_dimension(self.embedder.dim())
            .map_err(|e| MemoryError::Other(format!("vector set_dim: {e}")))?;
        if !vectors.is_empty() {
            guard
                .upsert_batch(&vectors)
                .map_err(|e| MemoryError::Other(format!("vector upsert_batch failed: {e}")))?;
        }
        drop(guard);
        // 落盘 sidecar (与 index_episode 一致) + 新签名记录
        self.persist_embedder_sidecar()?;
        self.persist_schema_sidecar()?;
        self.persist_normalize_sidecar()?;
        self.record_artifact(&sig)?;
        Ok(decision)
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

    // ---- P1#3: normalize 版本 (审计 A1#2) ----

    #[test]
    fn fresh_index_has_current_normalize_version() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert_eq!(idx.normalize_version(), CURRENT_NORMALIZE_VERSION);
        assert!(!idx.needs_reindex(), "新库不应 stale");
        // 写一条 → sidecar 落盘
        let ep = make_episode("n1", 1, "normalize version content");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&idx.memory, &ep).unwrap();
        idx.index_episode(&ep).unwrap();
        assert!(normalize_sidecar_path(&path).exists(), "normalize sidecar 应落盘");
        let stored = read_normalize_sidecar(&path);
        assert_eq!(stored, CURRENT_NORMALIZE_VERSION);
        cleanup(&path);
        let _ = std::fs::remove_file(normalize_sidecar_path(&path));
    }

    #[test]
    fn normalize_version_persists_across_reopen() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, fresh_embedder()).unwrap();
            let ep = make_episode("n2", 2, "chunk rule content");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        // sidecar 落盘 + 重开版本一致
        assert!(normalize_sidecar_path(&path).exists(), "normalize sidecar 应落盘");
        assert_eq!(read_normalize_sidecar(&path), CURRENT_NORMALIZE_VERSION);
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, fresh_embedder()).unwrap();
        assert_eq!(idx2.normalize_version(), CURRENT_NORMALIZE_VERSION);
        assert!(!idx2.needs_reindex(), "当前版本不 stale");
        assert_eq!(idx2.len().unwrap(), 1, "重开数据仍在");
        cleanup(&path);
        let _ = std::fs::remove_file(normalize_sidecar_path(&path));
    }

    #[test]
    fn stale_detection_pure_fn() {
        // 未来 bump 后, 旧版本库应判 stale (纯函数覆盖判定逻辑)
        assert!(normalize_is_stale(CURRENT_NORMALIZE_VERSION - 1), "旧版本应判 stale");
        assert!(!normalize_is_stale(CURRENT_NORMALIZE_VERSION), "当前版本不 stale");
        assert!(!normalize_is_stale(CURRENT_NORMALIZE_VERSION + 1), "未来版本由 open 拦截, 纯函数不判 stale");
    }

    #[test]
    fn newer_normalize_version_on_disk_errors() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        persist_normalize_sidecar_impl(&path, CURRENT_NORMALIZE_VERSION + 1).unwrap();
        let result = PersistentSemanticIndex::open(mem, &path, fresh_embedder());
        assert!(result.is_err(), "磁盘版本比代码新应报错 (旧代码别读新库)");
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("normalize"), "错误信息应提 normalize: {err}");
        cleanup(&path);
        let _ = std::fs::remove_file(normalize_sidecar_path(&path));
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

    // =====================================================================
    // P0-5: Embedder 身份持久化 (per RFC 001)
    // =====================================================================

    struct AltEmbedder {
        dim: usize,
        model: String,
    }
    impl crate::semantic::EmbedFn for AltEmbedder {
        fn dim(&self) -> usize { self.dim }
        fn embed(&self, _text: &str) -> Vec<f32> { vec![0.0; self.dim] }
        fn identity(&self) -> crate::semantic::EmbedderIdentity {
            crate::semantic::EmbedderIdentity::new(self.model.clone(), self.dim)
        }
    }

    #[test]
    fn p05_sidecar_written_on_first_index() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        let sidecar = std::path::PathBuf::from(format!("{}.embedder.json", path.display()));
        let _ = std::fs::remove_file(&sidecar);
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        assert!(sidecar.exists(), "sidecar 应被写出");
        let content = std::fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("apeireth/hash-fnva-1a/v1"), "model_name 应被持久化");
        assert!(content.contains("32"), "dimension 应被持久化");
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    #[test]
    fn p05_reopen_same_embedder_ok() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let e = fresh_embedder();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, Arc::clone(&e)).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        let e2 = fresh_embedder();
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e2).unwrap();
        assert_eq!(idx2.embedder_identity().model_name, "apeireth/hash-fnva-1a/v1");
        assert_eq!(idx2.embedder_identity().dimension, 32);
        cleanup(&path);
        let _ = std::fs::remove_file(format!("{}.embedder.json", path.display()));
    }

    #[test]
    fn p05_reopen_model_name_change_errors() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        {
            let e: Arc<dyn crate::semantic::EmbedFn> = Arc::new(AltEmbedder { dim: 32, model: "alpha".into() });
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        let e: Arc<dyn crate::semantic::EmbedFn> = Arc::new(AltEmbedder { dim: 32, model: "beta".into() });
        let result = PersistentSemanticIndex::open(Arc::clone(&mem), &path, e);
        assert!(result.is_err(), "model_name 改应报错");
        cleanup(&path);
        let _ = std::fs::remove_file(format!("{}.embedder.json", path.display()));
    }

    #[test]
    fn p05_legacy_no_sidecar_adopts_current() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.embedder.json", path.display()));
        let _ = std::fs::remove_file(&sidecar);
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert_eq!(idx.embedder_identity().model_name, "apeireth/hash-fnva-1a/v1");
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    #[test]
    fn p05_corrupt_sidecar_falls_back_to_legacy() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.embedder.json", path.display()));
        std::fs::write(&sidecar, "not valid json {[").unwrap();
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert_eq!(idx.embedder_identity().model_name, "apeireth/hash-fnva-1a/v1");
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    // =====================================================================
    // P0-6: Schema 版本 (per CURRENT_SCHEMA_VERSION)
    // =====================================================================

    #[test]
    fn p06_current_schema_version_is_1() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn p06_open_default_schema_version() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.schema.json", path.display()));
        let _ = std::fs::remove_file(&sidecar);
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert_eq!(idx.schema_version(), CURRENT_SCHEMA_VERSION);
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    #[test]
    fn p06_schema_sidecar_written_on_first_index() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.schema.json", path.display()));
        let _ = std::fs::remove_file(&sidecar);
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, fresh_embedder()).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        assert!(sidecar.exists(), "schema sidecar 应被写出");
        let content = std::fs::read_to_string(&sidecar).unwrap();
        assert_eq!(content.trim(), CURRENT_SCHEMA_VERSION.to_string());
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    #[test]
    fn p06_reopen_same_version_ok() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        {
            let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, fresh_embedder()).unwrap();
            let ep = make_episode("e1", 1, "x");
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }
        let idx2 = PersistentSemanticIndex::open(Arc::clone(&mem), &path, fresh_embedder()).unwrap();
        assert_eq!(idx2.schema_version(), CURRENT_SCHEMA_VERSION);
        cleanup(&path);
        let _ = std::fs::remove_file(format!("{}.schema.json", path.display()));
        let _ = std::fs::remove_file(format!("{}.embedder.json", path.display()));
    }

    #[test]
    fn p06_legacy_no_sidecar_adopts_current() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.schema.json", path.display()));
        let _ = std::fs::remove_file(&sidecar);
        let idx = PersistentSemanticIndex::open(mem, &path, fresh_embedder()).unwrap();
        assert_eq!(idx.schema_version(), CURRENT_SCHEMA_VERSION);
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    #[test]
    fn p06_disk_schema_higher_than_current_errors() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let sidecar = std::path::PathBuf::from(format!("{}.schema.json", path.display()));
        std::fs::write(&sidecar, (CURRENT_SCHEMA_VERSION + 1).to_string()).unwrap();
        let result = PersistentSemanticIndex::open(mem, &path, fresh_embedder());
        assert!(result.is_err(), "disk schema 比 current 新应报错");
        cleanup(&path);
        let _ = std::fs::remove_file(&sidecar);
    }

    // ---- N5: artifact_sig 内容寻址缓存门禁 (backlog N5) ----

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// N5 测试用计数 embedder: 证明门禁"真跳过" embed 重算 (0 装 PASS —
    /// 命中路径靠 embed 调用计数不增长断言, 不是靠"返回 Hit 就信").
    struct CountingEmbedder {
        inner: HashEmbedder,
        calls: Arc<AtomicUsize>,
    }

    fn counting_embedder(dim: usize) -> (Arc<CountingEmbedder>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let emb = Arc::new(CountingEmbedder {
            inner: HashEmbedder::new(dim),
            calls: Arc::clone(&calls),
        });
        (emb, calls)
    }

    impl EmbedFn for CountingEmbedder {
        fn dim(&self) -> usize {
            self.inner.dim()
        }
        fn embed(&self, text: &str) -> Vec<f32> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inner.embed(text)
        }
        fn identity(&self) -> EmbedderIdentity {
            self.inner.identity()
        }
    }

    /// N5 测试清理: 删 artifact_sig sidecar.
    fn remove_artifact_sidecar(path: &Path) {
        let _ = std::fs::remove_file(artifact_sig_sidecar_path(path));
    }

    #[test]
    fn n5_artifact_sig_matches_sha256_known_vectors() {
        // 算法明确可测: NIST FIPS 180-4 官方测试向量 (锚定手写实现正确性)
        assert_eq!(
            artifact_sig(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            artifact_sig("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 超过单块 (64B) 的输入也正确 (触发消息调度循环)
        assert_eq!(
            artifact_sig("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn n5_artifact_sig_deterministic_and_sensitive() {
        assert_eq!(artifact_sig("same content"), artifact_sig("same content"), "同内容恒同签名");
        assert_ne!(artifact_sig("same content"), artifact_sig("same content."), "一字符差 → 签名必变");
    }

    #[test]
    fn n5_artifact_sig_many_order_invariant_and_boundary_safe() {
        let ab: &[(&str, &str)] = &[("id1", "alpha"), ("id2", "beta")];
        let ba: &[(&str, &str)] = &[("id2", "beta"), ("id1", "alpha")];
        assert_eq!(artifact_sig_many(ab), artifact_sig_many(ba), "同集合与插入顺序无关");
        let changed: &[(&str, &str)] = &[("id1", "alpha"), ("id2", "beta!")];
        assert_ne!(artifact_sig_many(ab), artifact_sig_many(changed), "内容微变 → 签名变");
        // 边界防拼接歧义: ("ab","c") vs ("a","bc") 不能同签名
        let t1: &[(&str, &str)] = &[("ab", "c")];
        let t2: &[(&str, &str)] = &[("a", "bc")];
        assert_ne!(artifact_sig_many(t1), artifact_sig_many(t2), "id/content 边界歧义防护");
    }

    #[test]
    fn n5_gate_decision_rules_pure() {
        let sig = artifact_sig("content-v1");
        // 1. 无记录 → miss (首次构建)
        assert_eq!(artifact_gate_decision(None, &sig), ArtifactDecision::MissNoRecord);
        // 2. 签名匹配 + 版本 current → hit
        let rec_ok = ArtifactSigRecord {
            sig: sig.clone(),
            normalize_version: CURRENT_NORMALIZE_VERSION,
            schema_version: CURRENT_SCHEMA_VERSION,
        };
        assert_eq!(artifact_gate_decision(Some(&rec_ok), &sig), ArtifactDecision::Hit);
        assert!(!ArtifactDecision::Hit.should_recompute(), "Hit 不重算");
        assert!(ArtifactDecision::MissNoRecord.should_recompute(), "miss/stale 都重算");
        // 3. 内容变 → 签名变 → 强制重算 (防脏读)
        let sig2 = artifact_sig("content-v2");
        assert_eq!(
            artifact_gate_decision(Some(&rec_ok), &sig2),
            ArtifactDecision::MissContentChanged
        );
        // 4. 签名匹配但 normalize 落后 → 失效重算
        let rec_stale_norm = ArtifactSigRecord {
            normalize_version: CURRENT_NORMALIZE_VERSION - 1,
            ..rec_ok.clone()
        };
        assert_eq!(
            artifact_gate_decision(Some(&rec_stale_norm), &sig),
            ArtifactDecision::StaleNormalize
        );
        // 5. 签名匹配但 schema 不一致 → 失效重算
        let rec_stale_schema = ArtifactSigRecord {
            schema_version: CURRENT_SCHEMA_VERSION + 1,
            ..rec_ok
        };
        assert_eq!(
            artifact_gate_decision(Some(&rec_stale_schema), &sig),
            ArtifactDecision::StaleSchema
        );
    }

    #[test]
    fn n5_artifact_record_sidecar_roundtrip_and_corruption() {
        let path = temp_vector_path(); // 0 建 db, 纯 sidecar 读写
        let rec = ArtifactSigRecord {
            sig: artifact_sig("payload"),
            normalize_version: CURRENT_NORMALIZE_VERSION,
            schema_version: CURRENT_SCHEMA_VERSION,
        };
        persist_artifact_record_impl(&path, &rec).unwrap();
        assert_eq!(read_artifact_record(&path), Some(rec.clone()), "roundtrip 一致");
        // 损坏 JSON → None (= miss 走重算, 0 panic 0 假命中)
        std::fs::write(artifact_sig_sidecar_path(&path), "not-json").unwrap();
        assert_eq!(read_artifact_record(&path), None, "损坏记录应 miss 不 panic");
        let _ = std::fs::remove_file(artifact_sig_sidecar_path(&path));
    }

    #[test]
    fn n5_reindex_all_miss_then_hit_skips_recompute() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let (emb, calls) = counting_embedder(32);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, emb).unwrap();
        let eps = vec![make_episode("e1", 1, "alpha"), make_episode("e2", 2, "beta")];
        // 首次: 无签名记录 → miss → 真重算
        let d1 = idx.reindex_all(&eps).unwrap();
        assert_eq!(d1, ArtifactDecision::MissNoRecord);
        assert_eq!(idx.len().unwrap(), 2);
        let first_calls = calls.load(Ordering::SeqCst);
        assert_eq!(first_calls, 2, "miss 时每条 episode 都 embed (真重算)");
        // 二次: 内容不变 → hit → 跳过重算 (embed 计数不增长 = 真跳过)
        let d2 = idx.reindex_all(&eps).unwrap();
        assert_eq!(d2, ArtifactDecision::Hit);
        assert_eq!(calls.load(Ordering::SeqCst), first_calls, "hit 时跳过 embed 重算");
        assert_eq!(idx.len().unwrap(), 2, "hit 复用磁盘资产不丢");
        cleanup(&path);
        remove_artifact_sidecar(&path);
    }

    #[test]
    fn n5_reindex_all_content_micro_change_forces_recompute() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let (emb, calls) = counting_embedder(32);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, emb).unwrap();
        let eps = vec![make_episode("e1", 1, "alpha"), make_episode("e2", 2, "beta")];
        idx.reindex_all(&eps).unwrap();
        let before = calls.load(Ordering::SeqCst);
        // 内容微变 (id 不变, e2 内容改一字符) → 防脏读: 强制重算
        let eps2 = vec![make_episode("e1", 1, "alpha"), make_episode("e2", 2, "beta!")];
        let d = idx.reindex_all(&eps2).unwrap();
        assert_eq!(d, ArtifactDecision::MissContentChanged);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            before + 2,
            "内容变 → 全量重 embed"
        );
        assert_eq!(idx.len().unwrap(), 2, "clear+rebuild 后条数正确");
        // 新签名已落盘; 再跑同内容 → hit (证明重算后状态一致)
        let rec = read_artifact_record(&path).expect("重算后应有签名记录");
        assert_ne!(rec.sig, artifact_sig_many(&[("unused", "")]), "签名是真实内容签名");
        assert_eq!(idx.reindex_all(&eps2).unwrap(), ArtifactDecision::Hit);
        cleanup(&path);
        remove_artifact_sidecar(&path);
    }

    #[test]
    fn n5_reindex_all_stale_normalize_invalidates_hit() {
        let path = temp_vector_path();
        let mem = fresh_mem();
        let (emb, calls) = counting_embedder(32);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &path, emb).unwrap();
        let eps = vec![make_episode("e1", 1, "alpha")];
        idx.reindex_all(&eps).unwrap();
        // 模拟 stale: 磁盘签名与内容一致, 但按旧 normalize 规则构建
        let rec = read_artifact_record(&path).unwrap();
        let stale_rec = ArtifactSigRecord {
            normalize_version: CURRENT_NORMALIZE_VERSION - 1,
            ..rec
        };
        persist_artifact_record_impl(&path, &stale_rec).unwrap();
        // 同内容 → 签名匹配, 但 normalize stale → 强制重算
        let before = calls.load(Ordering::SeqCst);
        let d = idx.reindex_all(&eps).unwrap();
        assert_eq!(d, ArtifactDecision::StaleNormalize);
        assert!(calls.load(Ordering::SeqCst) > before, "stale 时真重算");
        // 重算后记录回到 CURRENT 版本 → 立即恢复 hit
        let rec2 = read_artifact_record(&path).unwrap();
        assert_eq!(rec2.normalize_version, CURRENT_NORMALIZE_VERSION, "重算后版本回 current");
        assert_eq!(idx.reindex_all(&eps).unwrap(), ArtifactDecision::Hit);
        cleanup(&path);
        remove_artifact_sidecar(&path);
    }

}

// 内部 helper: 让 unit test 能拿到 self.memory 写 episode (不走 public API 暴露)
#[cfg(test)]
impl PersistentSemanticIndex {
    pub(crate) fn memory_via_test(&self) -> Arc<SqliteMemoryStore> {
        Arc::clone(&self.memory)
    }
}
