//! Persistent brute-force cosine KNN (salvage of canonical
//! `apeireth-vector::sqlite_backend` **fallback** path).
//!
//! The canonical `vec0` virtual table requires the `sqlite-vec` C extension
//! (unsafe auto-extension + a new crates.io dependency). That path is
//! **deferred**. This module recovers the BLOB table, little-endian pack,
//! metadata JSON, upsert/delete/clear, and full-scan cosine KNN using only
//! `rusqlite` (already in `apeireth-memory`).
//!
//! It is **not** a second [`crate::canonical::vector::VectorIndex`] owner:
//! the in-memory index stays the hybrid-search semantic channel. This type
//! is the durable companion (own `.db` file, same as the canonical) and is
//! default-off — nothing in `hybrid_search` auto-wires it.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::canonical::vector::cosine_similarity;
use crate::metadata_filter::MetadataFilter;
use crate::MemoryError;

/// Default file name used by the canonical backend.
pub const DEFAULT_DB_FILE: &str = "apeireth-vector.db";

const META_DIM: &str = "dim";

/// A query hit: id, cosine score, optional metadata bag.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistentVectorHit {
    pub id: String,
    pub score: f32,
    pub metadata: Option<Value>,
}

/// SQLite-backed brute-force cosine index.
pub struct PersistentVectorIndex {
    conn: Connection,
    dim: Option<usize>,
    path: PathBuf,
}

impl PersistentVectorIndex {
    /// Open (or create) a file-backed index.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&path)?;
        Self::from_connection(conn, path)
    }

    /// Open an in-memory index (tests).
    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn, PathBuf::from(":memory:"))
    }

    fn from_connection(conn: Connection, path: PathBuf) -> Result<Self, MemoryError> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS vec_meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vec_items (
                 id       TEXT PRIMARY KEY,
                 dim      INTEGER NOT NULL,
                 vec      BLOB NOT NULL,
                 metadata TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_vec_items_dim ON vec_items(dim);",
        )?;
        let dim = read_dim(&conn)?;
        Ok(Self { conn, dim, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn dimension(&self) -> usize {
        self.dim.unwrap_or(0)
    }

    /// Lock the index to `dim`. Repeating the same dimension is a no-op;
    /// a different dimension is rejected.
    pub fn set_dimension(&mut self, dim: usize) -> Result<(), MemoryError> {
        if dim == 0 {
            return Err(MemoryError::Invalid(
                "vector dimension must be greater than 0".into(),
            ));
        }
        if let Some(existing) = self.dim {
            if existing != dim {
                return Err(MemoryError::Invalid(format!(
                    "vector dimension mismatch: expected {existing}, got {dim}"
                )));
            }
            return Ok(());
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO vec_meta(key, value) VALUES(?1, ?2)",
            params![META_DIM, dim.to_string()],
        )?;
        self.dim = Some(dim);
        Ok(())
    }

    pub fn len(&self) -> Result<usize, MemoryError> {
        if self.dim.is_none() {
            return Ok(0);
        }
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM vec_items", [], |row| row.get(0))?;
        Ok(n as usize)
    }

    pub fn is_empty(&self) -> Result<bool, MemoryError> {
        Ok(self.len()? == 0)
    }

    /// Insert or replace `(id, vector, metadata)`.
    pub fn upsert(
        &mut self,
        id: impl Into<String>,
        vector: &[f32],
        metadata: Option<&Value>,
    ) -> Result<(), MemoryError> {
        let id = id.into();
        if id.is_empty() {
            return Err(MemoryError::Invalid("vector id must not be empty".into()));
        }
        self.validate(vector)?;
        let blob = pack_vec(vector);
        let meta_json = metadata
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        let dim = vector.len() as i64;
        self.conn.execute(
            "INSERT INTO vec_items(id, dim, vec, metadata) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET dim=excluded.dim, vec=excluded.vec, metadata=excluded.metadata",
            params![id, dim, blob, meta_json],
        )?;
        Ok(())
    }

    /// Batch upsert inside a single transaction.
    pub fn upsert_batch(
        &mut self,
        items: &[(String, Vec<f32>, Option<Value>)],
    ) -> Result<(), MemoryError> {
        if items.is_empty() {
            return Ok(());
        }
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), MemoryError> {
            for (id, vector, meta) in items {
                self.upsert(id, vector, meta.as_ref())?;
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

    /// Cosine top-k. Optional `filter` is applied **after** scoring (canonical
    /// metadata is stored but was not used as a KNN predicate; this is the
    /// recovered filter behaviour).
    pub fn search(
        &self,
        query: &[f32],
        top_k: usize,
        filter: Option<&MetadataFilter>,
    ) -> Result<Vec<PersistentVectorHit>, MemoryError> {
        self.validate(query)?;
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let dim = self
            .dim
            .ok_or_else(|| MemoryError::Invalid("set_dimension() not called yet".into()))?
            as i64;
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, vec, metadata FROM vec_items WHERE dim = ?1")?;
        let mut hits: Vec<PersistentVectorHit> = stmt
            .query_map(params![dim], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let meta_str: Option<String> = row.get(2)?;
                Ok((id, blob, meta_str))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, blob, meta_str)| {
                let stored = unpack_vec(&blob);
                if stored.len() != query.len() {
                    return None;
                }
                let metadata = meta_str
                    .filter(|s| !s.is_empty())
                    .and_then(|s| serde_json::from_str(&s).ok());
                if let Some(filter) = filter {
                    if !filter.matches(metadata.as_ref()) {
                        return None;
                    }
                }
                Some(PersistentVectorHit {
                    id,
                    score: cosine_similarity(query, &stored),
                    metadata,
                })
            })
            .collect();

        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        hits.truncate(top_k);
        Ok(hits)
    }

    /// Delete by id. Missing ids return `false`.
    pub fn delete(&mut self, id: &str) -> Result<bool, MemoryError> {
        let n = self
            .conn
            .execute("DELETE FROM vec_items WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Drop all vectors and reset the stored dimension. Returns the number
    /// of rows removed.
    pub fn clear(&mut self) -> Result<usize, MemoryError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM vec_items", [], |row| row.get(0))?;
        self.conn.execute("DELETE FROM vec_items", [])?;
        self.conn.execute("DELETE FROM vec_meta", [])?;
        self.dim = None;
        Ok(n as usize)
    }

    fn validate(&self, vector: &[f32]) -> Result<(), MemoryError> {
        let dim = self
            .dim
            .ok_or_else(|| MemoryError::Invalid("set_dimension() not called yet".into()))?;
        if vector.len() != dim {
            return Err(MemoryError::Invalid(format!(
                "vector dimension mismatch: expected {dim}, got {}",
                vector.len()
            )));
        }
        if vector.is_empty() {
            return Err(MemoryError::Invalid("vector must not be empty".into()));
        }
        if vector.iter().any(|x| !x.is_finite()) {
            return Err(MemoryError::Invalid("vector values must be finite".into()));
        }
        Ok(())
    }
}

fn read_dim(conn: &Connection) -> Result<Option<usize>, MemoryError> {
    let dim: Option<String> = conn
        .query_row(
            "SELECT value FROM vec_meta WHERE key = ?1",
            params![META_DIM],
            |row| row.get(0),
        )
        .optional()?;
    match dim {
        Some(s) => s
            .parse::<usize>()
            .map(Some)
            .map_err(|_| MemoryError::Invalid(format!("corrupt vec_meta dim: {s}"))),
        None => Ok(None),
    }
}

fn pack_vec(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for &v in data {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn unpack_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_filter::MetadataFilter;
    use serde_json::json;

    #[test]
    fn open_set_dim_reject_mismatch() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        assert_eq!(idx.dimension(), 0);
        idx.set_dimension(4).unwrap();
        idx.set_dimension(4).unwrap();
        assert!(idx.set_dimension(8).is_err());
        assert!(idx.set_dimension(0).is_err());
    }

    #[test]
    fn upsert_search_delete_clear() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(3).unwrap();
        idx.upsert("v1", &[1.0, 0.0, 0.0], None).unwrap();
        idx.upsert("v2", &[0.0, 1.0, 0.0], None).unwrap();
        idx.upsert("v3", &[0.9, 0.1, 0.0], None).unwrap();
        assert_eq!(idx.len().unwrap(), 3);

        let hits = idx.search(&[1.0, 0.0, 0.0], 2, None).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "v1");
        assert!((hits[0].score - 1.0).abs() < 0.01);
        assert!(hits[0].score >= hits[1].score);

        assert!(idx.delete("v2").unwrap());
        assert!(!idx.delete("v2").unwrap());
        assert_eq!(idx.len().unwrap(), 2);
        assert_eq!(idx.clear().unwrap(), 2);
        assert_eq!(idx.len().unwrap(), 0);
        assert_eq!(idx.dimension(), 0);
    }

    #[test]
    fn upsert_overwrites_same_id() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(3).unwrap();
        idx.upsert("same", &[1.0, 0.0, 0.0], None).unwrap();
        idx.upsert("same", &[0.0, 1.0, 0.0], None).unwrap();
        assert_eq!(idx.len().unwrap(), 1);
        let hits = idx.search(&[0.0, 1.0, 0.0], 1, None).unwrap();
        assert_eq!(hits[0].id, "same");
        assert!((hits[0].score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn metadata_round_trip_and_filter() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(2).unwrap();
        idx.upsert(
            "alice",
            &[1.0, 0.0],
            Some(&json!({"kind": "agent", "role": "assistant"})),
        )
        .unwrap();
        idx.upsert(
            "bob",
            &[0.9, 0.1],
            Some(&json!({"kind": "agent", "role": "user"})),
        )
        .unwrap();
        idx.upsert("tool", &[0.8, 0.2], Some(&json!({"kind": "tool"})))
            .unwrap();

        let unfiltered = idx.search(&[1.0, 0.0], 10, None).unwrap();
        assert_eq!(unfiltered.len(), 3);
        assert_eq!(
            unfiltered[0].metadata.as_ref().unwrap()["role"],
            "assistant"
        );

        let filter = MetadataFilter::new()
            .kind_eq("agent")
            .property_string("role", "assistant");
        let hits = idx.search(&[1.0, 0.0], 10, Some(&filter)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "alice");
    }

    #[test]
    fn nan_and_dim_mismatch_rejected() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(3).unwrap();
        assert!(idx.upsert("bad", &[1.0, 0.0], None).is_err());
        assert!(idx.upsert("nan", &[1.0, f32::NAN, 0.0], None).is_err());
        assert!(idx.search(&[1.0, 0.0], 5, None).is_err());
        assert!(idx.search(&[1.0, 0.0, 0.0], 0, None).unwrap().is_empty());
    }

    #[test]
    fn ties_are_id_sorted_and_k_may_exceed_corpus() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(1).unwrap();
        idx.upsert("zulu", &[1.0], None).unwrap();
        idx.upsert("alpha", &[1.0], None).unwrap();
        let hits = idx.search(&[1.0], 100, None).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "alpha");
        assert_eq!(hits[1].id, "zulu");
        assert!((hits[0].score - hits[1].score).abs() < 1e-6);
    }

    #[test]
    fn file_reopen_preserves_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vec.db");
        {
            let mut idx = PersistentVectorIndex::open(&path).unwrap();
            idx.set_dimension(2).unwrap();
            idx.upsert("keep", &[1.0, 0.0], Some(&json!({"tag": "x"})))
                .unwrap();
        }
        let idx = PersistentVectorIndex::open(&path).unwrap();
        assert_eq!(idx.dimension(), 2);
        let hits = idx.search(&[1.0, 0.0], 1, None).unwrap();
        assert_eq!(hits[0].id, "keep");
        assert_eq!(hits[0].metadata.as_ref().unwrap()["tag"], "x");
    }

    #[test]
    fn batch_upsert_is_atomic_on_failure() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(2).unwrap();
        let items = vec![
            ("ok".into(), vec![1.0, 0.0], None),
            ("bad".into(), vec![1.0], None),
        ];
        assert!(idx.upsert_batch(&items).is_err());
        assert_eq!(idx.len().unwrap(), 0);
    }

    #[test]
    fn empty_corpus_search_is_empty() {
        let mut idx = PersistentVectorIndex::open_in_memory().unwrap();
        idx.set_dimension(3).unwrap();
        assert!(idx.search(&[1.0, 0.0, 0.0], 5, None).unwrap().is_empty());
    }
}
