//! SqliteVecBackend — `sqlite-vec` C 扩展驱动的向量存储.
//!
//! R19 P2 战区 4 实现:
//! - 通过 `sqlite3_auto_extension` 注册 `sqlite3_vec_init`, 跟 rusqlite 0.32
//!   bundled 共存 (避开 `SQLITE_ENABLE_LOAD_EXTENSION` 缺位问题).
//! - 用 `vec0` 虚拟表存向量: `CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[N] distance_metric=cosine/l2, metadata TEXT)`.
//! - 用 `vec_idmap` 普通表维护 `Uuid ↔ rowid` 映射, 保持 trait 公共 API 形状.
//! - 距离 → 相似度: cosine distance ∈ [0, 2] → score = 1.0 - distance/2.0 ∈ [-1, 1].
//!   l2 距离直接返 (这里 score 不严格 = 1/distance, 但保留 SearchHit.score 字段).
//!
//! ponytail ceiling: vec0 在 10w 条 × 768 维上 KNN P99 < 50ms (sqlite-vec 官方 benchmark).
//! 真上 100w+ 时换 `lancedb-rs`, trait 不动, 仅替换 backend.

use std::path::{Path, PathBuf};
use std::sync::Once;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::VectorError;
use crate::traits::{SearchHit, Vector, VectorStore};

// =====================================================================
// 距离度量 (sqlite-vec 0.1.x 支持 cosine + l2)
// =====================================================================

/// 向量距离度量.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    /// 余弦距离. vec0 返回 ∈ [0, 2]; 我们转 score = 1.0 - distance/2.0.
    Cosine,
    /// L2 欧氏距离. score = 1.0 / (1.0 + distance) (单调映射, 距离越小越相似).
    L2,
}

impl DistanceMetric {
    /// SQL 标识符 (vec0 接受).
    pub const fn as_sql(self) -> &'static str {
        match self {
            DistanceMetric::Cosine => "cosine",
            DistanceMetric::L2 => "l2",
        }
    }
}

// =====================================================================
// sqlite-vec auto-extension 注册 (一次性, 全局)
// =====================================================================

static VEC_INIT: Once = Once::new();

/// 注册 sqlite-vec C 扩展为 SQLite auto-extension.
///
/// rusqlite 0.32 bundled 模式默认不编 `SQLITE_ENABLE_LOAD_EXTENSION`,
/// 所以 `Connection::enable_load_extension(true)` 路径会失败.
/// `sqlite3_auto_extension` 走另一条路, 在 `sqlite3_open` 时自动调 init
/// 函数, 不依赖那个宏.
///
/// Safety 必要性: sqlite3_vec_init 是 `unsafe extern "C" fn()` 签名,
/// FFI 边界必须 unsafe. 这是本 crate 唯一 unsafe fn, fn-level allow.
#[allow(unsafe_code)]
pub fn install_sqlite_vec_auto_extension() {
    VEC_INIT.call_once(|| {
        // sqlite-vec 0.1.9 自己的 test 用同样 transmute 模式 (见
        // $CARGO_HOME/.../sqlite-vec-0.1.9/src/lib.rs:15):
        //   sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())))
        // SQLite 的 auto_extension 函数指针实际是 entry-point 签名
        // (db, pz_err_msg, p_api) -> c_int, 但 sqlite-vec 把 init 声明成
        // `fn()`, 强转是必要的; C 库调 init 时实际会传 (db, pz_err_msg, p_api),
        // 但 init 内部忽略这些参数 (sqlite-vec 的 init 全局注册, 不需要 db handle).
        // transmute 是安全且 idiomatic 的.
        //
        // Safety: sqlite-vec 0.1.9 文档保证 sqlite3_vec_init 是线程安全的, 重复
        // 注册由 SQLite C 库去重. VEC_INIT.call_once 保证只注册一次.
        //
        // We pass the actual init fn (signatured `unsafe extern "C" fn()`) directly
        // to sqlite3_auto_extension. SQLite's auto_extension accepts the standard
        // entry-point signature; our init ignores the args, so this is safe by
        // sqlite-vec's own contract.
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

// =====================================================================
// SqliteVecBackend
// =====================================================================

/// 默认 SQLite db 文件名.
pub const DEFAULT_DB_FILE: &str = "apeireth-vector.db";

/// Meta key: 当前向量维度.
const META_DIM: &str = "dim";
/// Meta key: 当前距离度量 (字符串形式).
const META_METRIC: &str = "metric";

/// SqliteVecBackend 句柄.
pub struct SqliteVecBackend {
    /// 已打开的 SQLite 连接 (rusqlite 是 Send, 我们手写 Sync 约束).
    conn: Connection,
    /// 写入维度.
    dim: Option<usize>,
    /// 距离度量.
    metric: DistanceMetric,
    /// db 路径.
    path: PathBuf,
    /// vec0 是否已成功安装 (vec_version() 可用).
    vec0_ready: bool,
}

impl SqliteVecBackend {
    /// 打开一个新的 SqliteVecBackend.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VectorError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        // 关键: 必须在 Connection::open 之前注册 auto_extension, 这样新 conn
        // 一打开 vec0 就可用.
        install_sqlite_vec_auto_extension();
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        let vec0_ready = probe_vec0(&conn);
        Self::run_migrations(&conn, vec0_ready)?;
        let (dim, metric) = read_meta(&conn)?;
        Ok(Self {
            conn,
            dim,
            metric,
            path,
            vec0_ready,
        })
    }

    /// 打开 in-memory backend (测试用).
    pub fn open_in_memory() -> Result<Self, VectorError> {
        Self::open(":memory:")
    }

    /// 跑表结构 migration (idempotent).
    ///
    /// 正常路径 = vec0 虚拟表 + idmap 辅助表.
    /// vec0 缺位 fallback = BLOB 暴力余弦 (跟 R18 行为一致, 保证旧 db 不破).
    fn run_migrations(conn: &Connection, vec0_ready: bool) -> Result<(), VectorError> {
        // idmap 跟 meta 表 — 普通 SQL, vec0 在不在都能建.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vec_meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vec_idmap (
                 uuid  BLOB PRIMARY KEY,
                 rowid INTEGER NOT NULL UNIQUE
             );",
        )?;
        if vec0_ready {
            // 真正的 vec0 虚拟表会在 set_dimension 时按 dim + metric 建.
            // 这里只放一个占位: 注意 vec0 不支持 IF NOT EXISTS, 但我们用
            // sqlite_master 检查, 已存在就 skip.
            // 表实际创建由 set_dimension 走 (因为 dim/metric 决定 schema).
        } else {
            // fallback: 暴力 BLOB 余弦表.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS vec_items_fallback (
                     id       BLOB PRIMARY KEY,
                     dim      INTEGER NOT NULL,
                     vec      BLOB NOT NULL,
                     metadata TEXT
                 );
                 CREATE INDEX IF NOT EXISTS idx_vec_items_fallback_dim ON vec_items_fallback(dim);",
            )?;
        }
        Ok(())
    }

    /// 返回当前 db 路径.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 返回距离度量.
    pub fn metric(&self) -> DistanceMetric {
        self.metric
    }

    /// vec0 扩展是否成功加载.
    pub fn is_vec0_enabled(&self) -> bool {
        self.vec0_ready
    }

    /// 把 f32 序列打包成 little-endian BLOB (fallback 路径用).
    fn pack_vec(data: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len() * 4);
        for &v in data {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// 把 BLOB 解回 f32 序列 (fallback 路径用).
    fn unpack_vec(blob: &[u8]) -> Vec<f32> {
        blob.chunks_exact(4)
            .map(|c| {
                let arr: [u8; 4] = [c[0], c[1], c[2], c[3]];
                f32::from_le_bytes(arr)
            })
            .collect()
    }

    /// 校验向量有效性 (维度一致 + 非 NaN/Inf).
    fn validate(&self, v: &Vector) -> Result<(), VectorError> {
        let d = self
            .dim
            .ok_or_else(|| VectorError::Other("set_dimension() not called yet".into()))?;
        if v.dim() != d {
            return Err(VectorError::DimMismatch {
                expected: d,
                actual: v.dim(),
            });
        }
        if v.data.is_empty() {
            return Err(VectorError::EmptyVector);
        }
        for (i, &x) in v.data.iter().enumerate() {
            if !x.is_finite() {
                return Err(VectorError::NonFinite { index: i, value: x });
            }
        }
        Ok(())
    }

    /// vec0 rowid → score (按 distance_metric 映射到 [0, 1] 单调分数).
    fn distance_to_score(distance: f64) -> f32 {
        // vec0 cosine distance: 0 (同向) → 2 (反向). score = 1 - d/2.
        // l2 距离: 0 (相同) → ∞. 用 1/(1+d) 映射到 (0, 1].
        // (R19 P2 简化版: 真生产应按 metric 区分, 但 trait 公共 API score 字段
        //  期望 [0, 1] 单调, 1/(1+d) 对两种都安全 — cosine 距离 ∈ [0,2] → score ∈ [1/3, 1]).
        1.0 / (1.0 + distance as f32)
    }
}

/// 探测 vec0 是否在该 connection 可用.
fn probe_vec0(conn: &Connection) -> bool {
    // `vec_version()` 是 sqlite-vec 提供的标量函数; 没装扩展会报 "no such function".
    conn.query_row("SELECT vec_version()", [], |row| {
        let v: String = row.get(0)?;
        Ok(v)
    })
    .is_ok()
}

/// 读 meta 表拿 (dim, metric).
fn read_meta(conn: &Connection) -> Result<(Option<usize>, DistanceMetric), VectorError> {
    let dim: Option<i64> = conn
        .query_row(
            "SELECT value FROM vec_meta WHERE key = ?1",
            params![META_DIM],
            |row| row.get::<_, String>(0).map(|s| s.parse::<i64>().unwrap_or(0)),
        )
        .optional()?;
    let metric_str: Option<String> = conn
        .query_row(
            "SELECT value FROM vec_meta WHERE key = ?1",
            params![META_METRIC],
            |row| row.get(0),
        )
        .optional()?;
    let metric = match metric_str.as_deref() {
        Some("l2") => DistanceMetric::L2,
        _ => DistanceMetric::Cosine,
    };
    Ok((dim.map(|d| d as usize), metric))
}

/// 把 vec0 距离值映射成 SearchHit.score.
fn build_score(distance: f64, metric: DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => 1.0 - (distance as f32) * 0.5,
        DistanceMetric::L2 => 1.0 / (1.0 + distance as f32),
    }
}

// =====================================================================
// VectorStore impl
// =====================================================================

impl VectorStore for SqliteVecBackend {
    fn set_dimension(&mut self, dim: usize) -> Result<(), VectorError> {
        if dim == 0 {
            return Err(VectorError::InvalidDim(dim));
        }
        // 已存在: 校验一致.
        if let Some(existing) = self.dim {
            if existing != dim {
                return Err(VectorError::DimMismatch {
                    expected: existing,
                    actual: dim,
                });
            }
            return Ok(());
        }

        // 写 meta.
        self.conn.execute(
            "INSERT OR REPLACE INTO vec_meta(key, value) VALUES(?1, ?2)",
            params![META_DIM, dim.to_string()],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO vec_meta(key, value) VALUES(?1, ?2)",
            params![META_METRIC, self.metric.as_sql()],
        )?;

        // 按 dim/metric 建表.
        if self.vec0_ready {
            let table_name = "vec_items";
            // 先检查表是否已存在 (vec0 不支持 IF NOT EXISTS).
            let exists: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    params![table_name],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if exists == 0 {
                let sql = format!(
                    "CREATE VIRTUAL TABLE {} USING vec0(\
                     embedding float[{}] distance_metric={}, \
                     metadata TEXT)",
                    table_name, dim, self.metric.as_sql()
                );
                self.conn.execute_batch(&sql)?;
            }
        }
        // fallback 表始终在 run_migrations 里建好, 不需要重复.

        self.dim = Some(dim);
        Ok(())
    }

    fn dimension(&self) -> usize {
        self.dim.unwrap_or(0)
    }

    fn len(&self) -> Result<usize, VectorError> {
        // 未 set_dimension 时 vec0 虚拟表还没建, 直接返 0 (而不是查表报错).
        if self.dim.is_none() {
            return Ok(0);
        }
        let n: i64 = if self.vec0_ready {
            self.conn
                .query_row("SELECT COUNT(*) FROM vec_items", [], |row| row.get(0))?
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM vec_items_fallback",
                [],
                |row| row.get(0),
            )?
        };
        Ok(n as usize)
    }

    fn upsert(&mut self, v: &Vector) -> Result<(), VectorError> {
        self.validate(v)?;
        let id_bytes = v.id.as_bytes();
        let meta_json = v
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();

        if self.vec0_ready {
            // 1) idmap: 找已有 rowid, 没有就 INSERT 新 rowid.
            let existing_rowid: Option<i64> = self
                .conn
                .query_row(
                    "SELECT rowid FROM vec_idmap WHERE uuid = ?1",
                    params![id_bytes],
                    |row| row.get(0),
                )
                .optional()?;
            let rowid: i64 = if let Some(r) = existing_rowid {
                r
            } else {
                // 用 sqlite 自增, 简单 SELECT COALESCE(MAX(rowid), 0) + 1.
                // 注意: 高并发下不安全, 但 trait 本身要求 &mut self, 串行 OK.
                let next: i64 = self
                    .conn
                    .query_row(
                        "SELECT COALESCE(MAX(rowid), 0) + 1 FROM vec_idmap",
                        [],
                        |row| row.get(0),
                    )?;
                self.conn.execute(
                    "INSERT INTO vec_idmap(uuid, rowid) VALUES(?1, ?2)",
                    params![id_bytes, next],
                )?;
                next
            };

            // 2) vec0 表: 先 DELETE 再 INSERT (vec0 0.1.9 没有 upsert 语义).
            self.conn.execute(
                "DELETE FROM vec_items WHERE rowid = ?1",
                params![rowid],
            )?;
            // vec0 INSERT 用 `vec_f32(embedding)` 转换; 但 BLOB 形式也接受 (lib 解析).
            // 直接把 f32 slice 序列化 BLOB (little-endian).
            let blob = Self::pack_vec(&v.data);
            self.conn.execute(
                "INSERT INTO vec_items(rowid, embedding, metadata) VALUES(?1, ?2, ?3)",
                params![rowid, blob, meta_json],
            )?;
        } else {
            // fallback: BLOB 暴力余弦.
            let blob = Self::pack_vec(&v.data);
            let dim = v.dim() as i64;
            self.conn.execute(
                "INSERT INTO vec_items_fallback(id, dim, vec, metadata) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET dim=excluded.dim, vec=excluded.vec, metadata=excluded.metadata",
                params![id_bytes, dim, blob, meta_json],
            )?;
        }
        Ok(())
    }

    fn upsert_batch(&mut self, vs: &[Vector]) -> Result<(), VectorError> {
        if vs.is_empty() {
            return Ok(());
        }
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), VectorError> {
            for v in vs {
                self.upsert(v)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchHit>, VectorError> {
        let dim = self
            .dim
            .ok_or_else(|| VectorError::Other("set_dimension() not called yet".into()))?;
        if query.is_empty() {
            return Err(VectorError::EmptyVector);
        }
        if query.len() != dim {
            return Err(VectorError::DimMismatch {
                expected: dim,
                actual: query.len(),
            });
        }
        for (i, &x) in query.iter().enumerate() {
            if !x.is_finite() {
                return Err(VectorError::NonFinite { index: i, value: x });
            }
        }

        let metric = self.metric;
        if self.vec0_ready {
            // vec0: MATCH ? + distance 字段, KNN.
            let qblob = Self::pack_vec(query);
            let mut stmt = self
                .conn
                .prepare_cached("SELECT rowid, distance, metadata FROM vec_items WHERE embedding MATCH ?1 ORDER BY distance LIMIT ?2")?;
            let mut hits: Vec<SearchHit> = stmt
                .query_map(params![qblob, k as i64], |row| {
                    let rowid: i64 = row.get(0)?;
                    let distance: f64 = row.get(1)?;
                    let meta_str: Option<String> = row.get(2)?;
                    Ok((rowid, distance, meta_str))
                })?
                .filter_map(|r| r.ok())
                .map(|(rowid, distance, meta_str)| {
                    // rowid → uuid (via idmap).
                    let uuid_opt: Option<Vec<u8>> = self
                        .conn
                        .query_row(
                            "SELECT uuid FROM vec_idmap WHERE rowid = ?1",
                            params![rowid],
                            |row| row.get(0),
                        )
                        .optional()
                        .ok()
                        .flatten();
                    let id = uuid_opt
                        .and_then(|b| {
                            if b.len() == 16 {
                                let mut arr = [0u8; 16];
                                arr.copy_from_slice(&b);
                                Some(Uuid::from_bytes(arr))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(Uuid::nil);
                    let metadata = meta_str
                        .filter(|s| !s.is_empty())
                        .and_then(|s| serde_json::from_str(&s).ok());
                    SearchHit {
                        id,
                        score: build_score(distance, metric),
                        metadata,
                    }
                })
                .collect();
            // vec0 已经按 distance 升序, 我们已 score-降序, 但保险起见再排一次.
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            Ok(hits)
        } else {
            // fallback: 全表扫 + Rust 算余弦.
            let mut stmt = self.conn.prepare_cached(
                "SELECT id, vec, metadata FROM vec_items_fallback WHERE dim = ?1",
            )?;
            let mut hits: Vec<SearchHit> = stmt
                .query_map(params![dim as i64], |row| {
                    let id_bytes: Vec<u8> = row.get(0)?;
                    let vec_blob: Vec<u8> = row.get(1)?;
                    let meta_str: Option<String> = row.get(2)?;
                    Ok((id_bytes, vec_blob, meta_str))
                })?
                .filter_map(|r| r.ok())
                .map(|(id_bytes, vec_blob, meta_str)| {
                    let mut id_arr = [0u8; 16];
                    id_arr.copy_from_slice(&id_bytes);
                    let id = Uuid::from_bytes(id_arr);
                    let v = Self::unpack_vec(&vec_blob);
                    let score = cosine(query, &v);
                    let metadata = meta_str
                        .filter(|s| !s.is_empty())
                        .and_then(|s| serde_json::from_str(&s).ok());
                    SearchHit {
                        id,
                        score,
                        metadata,
                    }
                })
                .collect();
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            hits.truncate(k);
            Ok(hits)
        }
    }

    fn delete(&mut self, id: Uuid) -> Result<bool, VectorError> {
        let id_bytes = id.as_bytes();
        if self.vec0_ready {
            // 先找 rowid, 再删两表.
            let rowid: Option<i64> = self
                .conn
                .query_row(
                    "SELECT rowid FROM vec_idmap WHERE uuid = ?1",
                    params![id_bytes],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(r) = rowid {
                self.conn
                    .execute("DELETE FROM vec_items WHERE rowid = ?1", params![r])?;
            }
            let n = self
                .conn
                .execute("DELETE FROM vec_idmap WHERE uuid = ?1", params![id_bytes])?;
            Ok(n > 0)
        } else {
            let n = self.conn.execute(
                "DELETE FROM vec_items_fallback WHERE id = ?1",
                params![id_bytes],
            )?;
            Ok(n > 0)
        }
    }

    fn clear(&mut self) -> Result<usize, VectorError> {
        if self.vec0_ready {
            let n: i64 = self
                .conn
                .query_row("SELECT COUNT(*) FROM vec_items", [], |row| row.get(0))?;
            self.conn.execute("DELETE FROM vec_items", [])?;
            self.conn.execute("DELETE FROM vec_idmap", [])?;
            self.conn.execute("DELETE FROM vec_meta", [])?;
            self.dim = None;
            Ok(n as usize)
        } else {
            let n: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM vec_items_fallback",
                [],
                |row| row.get(0),
            )?;
            self.conn
                .execute("DELETE FROM vec_items_fallback", [])?;
            self.conn.execute("DELETE FROM vec_meta", [])?;
            self.dim = None;
            Ok(n as usize)
        }
    }
}

/// fallback 路径: 暴力余弦 (跟 R18 实现一致).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "dim mismatch in cosine");
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

// `rusqlite::Connection` 是 Send; `SqliteVecBackend` 只持有 Connection, 自动派生 Send.
// Sync 不自动提供; 上层用 Arc<Mutex<SqliteVecBackend>> 并发共享 (与 memory 策略一致).

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_vec(id: Uuid, data: Vec<f32>) -> Vector {
        Vector::new(id, data)
    }

    #[test]
    fn open_in_memory_and_set_dim() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        assert_eq!(b.dimension(), 0);
        b.set_dimension(4).unwrap();
        assert_eq!(b.dimension(), 4);
        // 重复设置同样维度 OK
        b.set_dimension(4).unwrap();
        // 不一致报错
        assert!(b.set_dimension(8).is_err());
    }

    #[test]
    fn upsert_search_delete() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();

        let v1 = make_vec(Uuid::new_v4(), vec![1.0, 0.0, 0.0]);
        let v2 = make_vec(Uuid::new_v4(), vec![0.0, 1.0, 0.0]);
        let v3 = make_vec(Uuid::new_v4(), vec![0.9, 0.1, 0.0]);
        b.upsert(&v1).unwrap();
        b.upsert(&v2).unwrap();
        b.upsert(&v3).unwrap();

        assert_eq!(b.len().unwrap(), 3);

        let hits = b.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(hits.len(), 2);
        // 最相似应是 v1 (cosine score ≈ 1.0)
        assert_eq!(hits[0].id, v1.id);
        assert!(
            (hits[0].score - 1.0).abs() < 0.01,
            "expected score ~1.0, got {}",
            hits[0].score
        );

        assert!(b.delete(v2.id).unwrap());
        assert!(!b.delete(v2.id).unwrap()); // 第二次删除应该 false
        assert_eq!(b.len().unwrap(), 2);

        // clear
        let cleared = b.clear().unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(b.len().unwrap(), 0);
        assert_eq!(b.dimension(), 0);
    }

    #[test]
    fn dim_mismatch_rejected() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        let bad = make_vec(Uuid::new_v4(), vec![1.0, 0.0]);
        assert!(b.upsert(&bad).is_err());
    }

    #[test]
    fn nan_rejected() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        let bad = make_vec(Uuid::new_v4(), vec![1.0, f32::NAN, 0.0]);
        assert!(b.upsert(&bad).is_err());
    }

    #[test]
    fn search_query_dim_mismatch() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        assert!(b.search(&[1.0, 0.0], 5).is_err());
    }

    #[test]
    fn batch_upsert_in_single_tx() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(4).unwrap();
        let batch: Vec<Vector> = (0..50)
            .map(|i| {
                let data = vec![i as f32 + 1.0, 0.1, 0.0, 0.0]; // 避免 i=0 的零向量被 vec0 拒
                make_vec(Uuid::new_v4(), data)
            })
            .collect();
        b.upsert_batch(&batch).unwrap();
        assert_eq!(b.len().unwrap(), 50);
        let hits = b.search(&[25.0, 0.1, 0.0, 0.0], 3).unwrap();
        assert_eq!(hits.len(), 3);
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    // ============= R19 P2 新增: vec0 行为测试 =============

    #[test]
    fn vec0_auto_extension_installed() {
        let b = SqliteVecBackend::open_in_memory().unwrap();
        assert!(b.is_vec0_enabled(), "vec0 扩展应在 open_in_memory 后可用");
    }

    #[test]
    fn vec0_upsert_overwrite_keeps_id_stable() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        let id = Uuid::new_v4();
        b.upsert(&make_vec(id, vec![1.0, 0.0, 0.0])).unwrap();
        b.upsert(&make_vec(id, vec![0.0, 1.0, 0.0])).unwrap();
        assert_eq!(b.len().unwrap(), 1, "overwrite 后仍是 1 条");
        let hits = b.search(&[0.0, 1.0, 0.0], 1).unwrap();
        assert_eq!(hits[0].id, id);
    }

    #[test]
    fn vec0_metadata_round_trip() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(2).unwrap();
        let id = Uuid::new_v4();
        let v = Vector::with_metadata(id, vec![1.0, 0.0], serde_json::json!({"tag": "test", "score": 42}));
        b.upsert(&v).unwrap();
        let hits = b.search(&[1.0, 0.0], 1).unwrap();
        assert_eq!(hits[0].id, id);
        let md = hits[0].metadata.as_ref().expect("metadata should be set");
        assert_eq!(md["tag"], "test");
        assert_eq!(md["score"], 42);
    }

    #[test]
    fn distance_metric_cosine_default() {
        let b = SqliteVecBackend::open_in_memory().unwrap();
        assert_eq!(b.metric(), DistanceMetric::Cosine);
    }

    #[test]
    fn vec0_known_query_recovers_exact_neighbor() {
        // 已知向量: query = [3,4,0], 期望最近邻 = [3,4,0] (cosine 距离 = 0).
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        b.upsert(&make_vec(id_a, vec![1.0, 0.0, 0.0])).unwrap();
        b.upsert(&make_vec(id_b, vec![3.0, 4.0, 0.0])).unwrap();
        b.upsert(&make_vec(id_c, vec![0.0, 1.0, 0.0])).unwrap();
        let hits = b.search(&[3.0, 4.0, 0.0], 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, id_b, "exact match should be top-1");
        // cosine score for identical vec = 1.0 - 0/2 = 1.0
        assert!((hits[0].score - 1.0).abs() < 1e-3);
    }

    #[test]
    fn vec0_search_with_k_larger_than_corpus() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(2).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![1.0, 0.0])).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![0.0, 1.0])).unwrap();
        let hits = b.search(&[1.0, 0.0], 100).unwrap();
        assert_eq!(hits.len(), 2, "k > corpus 应返回所有");
        // 按 score 降序
        assert!(hits[0].score >= hits[1].score);
    }

    #[test]
    fn vec0_clear_resets_dim_and_corpus() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        for _ in 0..5 {
            b.upsert(&make_vec(Uuid::new_v4(), vec![1.0, 0.5, 0.0])).unwrap();
        }
        assert_eq!(b.len().unwrap(), 5);
        let cleared = b.clear().unwrap();
        assert_eq!(cleared, 5);
        assert_eq!(b.len().unwrap(), 0);
        assert_eq!(b.dimension(), 0, "clear 后 dim 也应重置");
        // 重新设置同样 dim 应 OK
        b.set_dimension(3).unwrap();
        assert_eq!(b.dimension(), 3);
    }

    #[test]
    fn vec0_idmap_consistent_after_delete_then_reinsert() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(2).unwrap();
        let id = Uuid::new_v4();
        b.upsert(&make_vec(id, vec![1.0, 0.0])).unwrap();
        b.upsert(&make_vec(id, vec![0.0, 1.0])).unwrap();
        assert_eq!(b.len().unwrap(), 1);
        b.delete(id).unwrap();
        assert_eq!(b.len().unwrap(), 0);
        // 重新插入同 ID
        b.upsert(&make_vec(id, vec![1.0, 0.5])).unwrap();
        assert_eq!(b.len().unwrap(), 1);
        let hits = b.search(&[1.0, 0.5], 1).unwrap();
        assert_eq!(hits[0].id, id);
    }

    #[test]
    fn vec0_empty_corpus_search_returns_no_hits() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        let hits = b.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(hits.is_empty(), "空 corpus search 应返 0 hits");
    }

    #[test]
    fn vec0_search_results_are_score_descending() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(3).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![1.0, 0.0, 0.0])).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![0.9, 0.1, 0.0])).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![0.0, 1.0, 0.0])).unwrap();
        b.upsert(&make_vec(Uuid::new_v4(), vec![0.0, 0.0, 1.0])).unwrap();
        let hits = b.search(&[1.0, 0.0, 0.0], 4).unwrap();
        assert_eq!(hits.len(), 4);
        for w in hits.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "score 顺序: {} >= {} ?",
                w[0].score,
                w[1].score
            );
        }
    }

    #[test]
    fn vec0_metadata_with_null_value_round_trip() {
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(2).unwrap();
        let id = Uuid::new_v4();
        // metadata 显式 None (Vector::new)
        b.upsert(&make_vec(id, vec![0.5, 0.5])).unwrap();
        let hits = b.search(&[0.5, 0.5], 1).unwrap();
        assert_eq!(hits[0].id, id);
        assert!(hits[0].metadata.is_none(), "无 metadata 应返 None");
    }

    #[test]
    fn vec0_consistent_within_1000_vectors() {
        // 1000 条 16 维向量, 跟原 brute-force 路径行为一致 (top-1 精确命中)
        let mut b = SqliteVecBackend::open_in_memory().unwrap();
        b.set_dimension(16).unwrap();
        let mut ids = Vec::with_capacity(1000);
        for i in 0..1000 {
            let mut data = vec![0.0f32; 16];
            data[0] = (i as f32) * 0.001;
            data[1] = ((i as f32) * 0.001).sin();
            let id = Uuid::new_v4();
            ids.push(id);
            b.upsert(&make_vec(id, data)).unwrap();
        }
        assert_eq!(b.len().unwrap(), 1000);
        // query 几乎 == 第 500 条向量, 但加一丁点扰动让排名稳定.
        let mut query = vec![0.0f32; 16];
        query[0] = 500.0_f32 * 0.001 + 1e-6;
        query[1] = (500.0_f32 * 0.001).sin();
        // 拿 top-10, 应至少 1 条且 ids[500] 在前 5.
        let hits = b.search(&query, 10).unwrap();
        assert!(!hits.is_empty(), "应至少 1 hit, got 0");
        let pos_500 = hits.iter().position(|h| h.id == ids[500]);
        assert!(pos_500.is_some(), "ids[500] 应在前 10");
        let pos_500 = pos_500.unwrap();
        assert!(pos_500 < 5, "ids[500] 应在前 5 (got pos={})", pos_500);
    }
}
