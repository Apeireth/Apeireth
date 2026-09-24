//! Windowed fingerprint dedup (salvage of companion `observer_capture` LRU + hash).
//!
//! Donor behaviour recovered:
//! - Canonical SHA-256 fingerprint of a payload (first 16 hex chars).
//! - In-memory LRU keyed by `(namespace, fingerprint)` with a time window.
//! - Optional SQLite sidecar so the window survives process restart.
//!
//! This is **not** a second MemoryStore. Persistence reuses
//! [`crate::SqliteMemoryStore::conn`] and a dedicated `dedup_fingerprints`
//! table (created idempotently). Episodes are never polluted with `expc-*`
//! rows — that donor shortcut is discarded.
//!
//! Default window = 24h; default LRU cap = 1024 (donor constants).

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// Default dedup window (24 hours, milliseconds).
pub const DEFAULT_DEDUP_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

/// Default in-memory LRU capacity.
pub const DEFAULT_LRU_CAP: usize = 1024;

/// Character Jaccard threshold used by [`dedup_textual`] (donor dream.rs: 0.8).
pub const TEXTUAL_OVERLAP_THRESHOLD: f64 = 0.8;

/// Minimum normalized length before substring / overlap collapse applies.
pub const TEXTUAL_MIN_LEN: usize = 20;

/// Fingerprint hex length (8 bytes = 16 hex chars; donor `args_hash`).
const FINGERPRINT_HEX_LEN: usize = 16;

/// Configuration for a [`DedupIndex`].
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Time window in milliseconds. Same `(namespace, fingerprint)` inside
    /// this window is treated as a duplicate.
    pub window_ms: i64,
    /// In-memory LRU capacity. Eviction does not delete the SQLite sidecar.
    pub lru_cap: usize,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            window_ms: DEFAULT_DEDUP_WINDOW_MS,
            lru_cap: DEFAULT_LRU_CAP,
        }
    }
}

/// SHA-256 fingerprint of raw bytes, truncated to 16 hex chars.
pub fn fingerprint_bytes(payload: &[u8]) -> String {
    let digest = Sha256::digest(payload);
    let mut out = String::with_capacity(FINGERPRINT_HEX_LEN);
    for b in digest.iter().take(FINGERPRINT_HEX_LEN / 2) {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// SHA-256 fingerprint of a JSON value (canonical `serde_json::to_string`).
pub fn fingerprint_json(value: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(value).unwrap_or_default();
    fingerprint_bytes(canonical.as_bytes())
}

/// SHA-256 fingerprint of an episode's role + normalized content.
///
/// Normalization: trim, collapse interior whitespace, lowercase. Two
/// utterances that differ only in spacing/case share a fingerprint.
pub fn episode_fingerprint(role: &str, content: &str) -> String {
    let mut normalized = String::with_capacity(role.len() + 1 + content.len());
    normalized.push_str(role.trim());
    normalized.push('\n');
    let mut last_space = false;
    for ch in content.trim().chars() {
        if ch.is_whitespace() {
            if !last_space {
                normalized.push(' ');
                last_space = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                normalized.push(lower);
            }
            last_space = false;
        }
    }
    fingerprint_bytes(normalized.as_bytes())
}

/// Strip whitespace and lowercase (donor `dream::dedup_textual` normalizer).
pub fn normalize_for_dedup(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Character-set Jaccard overlap of two already-normalized strings.
pub fn overlap_ratio(a: &str, b: &str) -> f64 {
    let sa: HashSet<char> = a.chars().collect();
    let sb: HashSet<char> = b.chars().collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// In-place textual near-dup collapse. Keeps the longer item.
///
/// Two items are duplicates when, after [`normalize_for_dedup`]:
/// - they are equal, or
/// - both are at least [`TEXTUAL_MIN_LEN`] and one contains the other, or
/// - both are at least [`TEXTUAL_MIN_LEN`] and Jaccard overlap exceeds
///   [`TEXTUAL_OVERLAP_THRESHOLD`].
///
/// Donor `dream::dedup_textual` incremented `i` after replacing the current
/// item, skipping the replacement. This port stays on `i` so the new occupant
/// is compared against the remainder.
pub fn dedup_textual(items: &mut Vec<String>) {
    let mut i = 0;
    while i < items.len() {
        let a = normalize_for_dedup(&items[i]);
        let mut j = i + 1;
        let mut removed_i = false;
        while j < items.len() {
            let b = normalize_for_dedup(&items[j]);
            let long_enough = a.len() >= TEXTUAL_MIN_LEN && b.len() >= TEXTUAL_MIN_LEN;
            let dup = a == b
                || (long_enough
                    && (a.contains(&b)
                        || b.contains(&a)
                        || overlap_ratio(&a, &b) > TEXTUAL_OVERLAP_THRESHOLD));
            if dup {
                if items[i].chars().count() >= items[j].chars().count() {
                    items.remove(j);
                } else {
                    items.remove(i);
                    removed_i = true;
                    break;
                }
            } else {
                j += 1;
            }
        }
        if !removed_i {
            i += 1;
        }
    }
}

/// Windowed fingerprint index (in-memory LRU + optional SQLite sidecar).
pub struct DedupIndex {
    inner: Mutex<Inner>,
    window_ms: i64,
}

struct Inner {
    /// `(namespace, fingerprint)` → last accepted timestamp (ms).
    lru: HashMap<(String, String), i64>,
    /// Insertion order for LRU eviction.
    order: VecDeque<(String, String)>,
    lru_cap: usize,
}

impl DedupIndex {
    /// Empty in-memory index with default window / cap.
    pub fn new() -> Self {
        Self::with_config(DedupConfig::default())
    }

    /// Fully configured in-memory index.
    pub fn with_config(cfg: DedupConfig) -> Self {
        Self {
            inner: Mutex::new(Inner {
                lru: HashMap::new(),
                order: VecDeque::new(),
                lru_cap: cfg.lru_cap.max(1),
            }),
            window_ms: cfg.window_ms.max(1),
        }
    }

    /// Window in milliseconds.
    pub fn window_ms(&self) -> i64 {
        self.window_ms
    }

    /// `true` = first sighting (or window expired) → accept.
    /// `false` = duplicate inside the window → reject.
    pub fn accept(&self, namespace: &str, fingerprint: &str, now_ms: i64) -> bool {
        let key = (namespace.to_string(), fingerprint.to_string());
        let mut inner = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(&prev_ts) = inner.lru.get(&key) {
            if now_ms.saturating_sub(prev_ts) < self.window_ms {
                return false;
            }
        }
        Self::touch_lru(&mut inner, key, now_ms);
        true
    }

    /// Same as [`Self::accept`] but consults / writes the SQLite sidecar so
    /// the window survives restart. LRU miss falls back to the table.
    pub fn accept_persisted(
        &self,
        store: &SqliteMemoryStore,
        namespace: &str,
        fingerprint: &str,
        now_ms: i64,
    ) -> MemoryResult<bool> {
        if namespace.trim().is_empty() || fingerprint.trim().is_empty() {
            return Err(MemoryError::Invalid(
                "dedup namespace/fingerprint must not be empty".into(),
            ));
        }
        if !self.accept(namespace, fingerprint, now_ms) {
            return Ok(false);
        }
        // LRU accepted. Confirm against sqlite (covers restart / LRU eviction).
        // V15 (M27) 起 dedup_fingerprints 已由迁移列表创建; 懒建保留作
        // pre-V15 老库的 self-heal (幂等).
        ensure_dedup_table(store)?;
        let mut conn = store.conn()?;
        // M23: SELECT + INSERT/UPSERT 必须在同一个 BEGIN IMMEDIATE 事务内.
        // 同进程内 `store.conn()` 的 MutexGuard 已串行化, 但**两个进程**同时
        // 打开同一 DB 时, 事务外的 read-check-write 会让双方都读到无 prev →
        // 都写 → 窗口内重复条目被双双接受 (去重语义被破坏). IMMEDIATE 一
        // 开始就拿写锁, 读-判-写原子 (与 migrations.rs M21 同模式).
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let prev: Option<i64> = tx
            .query_row(
                "SELECT last_seen_ms FROM dedup_fingerprints
                 WHERE namespace = ?1 AND fingerprint = ?2",
                params![namespace, fingerprint],
                |row| row.get(0),
            )
            .optional_row()?;
        if let Some(prev_ts) = prev {
            if now_ms.saturating_sub(prev_ts) < self.window_ms {
                // Roll back the in-memory accept so LRU matches sqlite.
                // M22: 回滚必须走 `touch_lru` — 旧实现 `inner.lru.insert(key,
                // prev_ts)` 只插 lru 不进 `order` 队列, 该 key 永远不被驱逐,
                // `lru_cap` 形同虚设 (每个 "LRU 未命中但 sqlite 窗口命中" 的
                // 重复 key 泄漏一个条目, 高流量重复请求下 lru 无限增长).
                let key = (namespace.to_string(), fingerprint.to_string());
                drop(tx); // 只读事务, 先回滚再锁 inner (避免持DB事务时锁内存)
                let mut inner = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                Self::touch_lru(&mut inner, key, prev_ts);
                return Ok(false);
            }
        }
        tx.execute(
            "INSERT INTO dedup_fingerprints (namespace, fingerprint, last_seen_ms)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(namespace, fingerprint) DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
            params![namespace, fingerprint, now_ms],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Current in-memory LRU size (debug / tests).
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).lru.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn touch_lru(inner: &mut Inner, key: (String, String), now_ms: i64) {
        if inner.lru.contains_key(&key) {
            inner.lru.insert(key, now_ms);
            return;
        }
        if inner.order.len() >= inner.lru_cap {
            if let Some(evicted) = inner.order.pop_front() {
                inner.lru.remove(&evicted);
            }
        }
        inner.order.push_back(key.clone());
        inner.lru.insert(key, now_ms);
    }
}

impl Default for DedupIndex {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_dedup_table(store: &SqliteMemoryStore) -> MemoryResult<()> {
    // V15 (M27) 起 canonical 定义在 migrations 列表; 这里是 pre-V15 老库的
    // self-heal 路径 (幂等, 与 V15 DDL 逐字一致).
    let conn = store.conn()?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS dedup_fingerprints (
            namespace    TEXT NOT NULL,
            fingerprint  TEXT NOT NULL,
            last_seen_ms INTEGER NOT NULL,
            PRIMARY KEY (namespace, fingerprint)
         );
         CREATE INDEX IF NOT EXISTS idx_dedup_last_seen
            ON dedup_fingerprints(last_seen_ms);",
    )?;
    Ok(())
}

/// Tiny helper so `.optional()` works without importing rusqlite::OptionalExtension
/// at every call site (the trait is used via this inherent-style wrapper).
trait OptionalRow<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fingerprint_is_stable_and_truncated() {
        let a = fingerprint_bytes(b"{\"x\":1}");
        let b = fingerprint_bytes(b"{\"x\":1}");
        let c = fingerprint_bytes(b"{\"x\":2}");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn fingerprint_json_matches_bytes_of_canonical() {
        let v = json!({"x": 1});
        let expected = fingerprint_bytes(serde_json::to_string(&v).unwrap().as_bytes());
        assert_eq!(fingerprint_json(&v), expected);
    }

    #[test]
    fn episode_fingerprint_collapses_whitespace_and_case() {
        let a = episode_fingerprint("user", "Hello   World");
        let b = episode_fingerprint("user", "hello world");
        let c = episode_fingerprint("assistant", "hello world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn accept_suppresses_duplicate_inside_window() {
        let idx = DedupIndex::new();
        assert!(idx.accept("tool:t", "h", 1_000_000));
        assert!(!idx.accept("tool:t", "h", 1_000_000 + 1000));
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn accept_allows_after_window_expires() {
        let idx = DedupIndex::with_config(DedupConfig {
            window_ms: 100,
            lru_cap: 8,
        });
        assert!(idx.accept("ns", "fp", 0));
        assert!(!idx.accept("ns", "fp", 99));
        assert!(idx.accept("ns", "fp", 100));
    }

    #[test]
    fn different_namespace_or_fingerprint_not_deduped() {
        let idx = DedupIndex::new();
        assert!(idx.accept("a", "h", 0));
        assert!(idx.accept("b", "h", 0));
        assert!(idx.accept("a", "g", 0));
        assert_eq!(idx.len(), 3);
    }

    #[test]
    fn lru_evicts_oldest_when_cap_exceeded() {
        let idx = DedupIndex::with_config(DedupConfig {
            window_ms: 10_000,
            lru_cap: 2,
        });
        assert!(idx.accept("n", "1", 1));
        assert!(idx.accept("n", "2", 2));
        assert!(idx.accept("n", "3", 3));
        assert_eq!(idx.len(), 2);
        // "1" was evicted from LRU so a re-accept inside the window succeeds
        // in-memory (sqlite sidecar is the restart-safe authority).
        assert!(idx.accept("n", "1", 4));
    }

    #[test]
    fn persisted_window_survives_new_index() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let a = DedupIndex::with_config(DedupConfig {
            window_ms: 1_000,
            lru_cap: 8,
        });
        assert!(a.accept_persisted(&store, "ns", "fp", 100).unwrap());
        let b = DedupIndex::with_config(DedupConfig {
            window_ms: 1_000,
            lru_cap: 8,
        });
        assert!(
            !b.accept_persisted(&store, "ns", "fp", 500).unwrap(),
            "fresh index must still hit sqlite window"
        );
        assert!(b.accept_persisted(&store, "ns", "fp", 100 + 1_000).unwrap());
    }

    #[test]
    fn persisted_rejects_empty_keys() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let idx = DedupIndex::new();
        assert!(idx.accept_persisted(&store, " ", "fp", 0).is_err());
        assert!(idx.accept_persisted(&store, "ns", "", 0).is_err());
    }

    /// M22: sqlite 窗口命中后的 LRU 回滚必须走 `touch_lru` (lru/order 双
    /// 结构不变式). 旧实现直接 `lru.insert` 不进 order → 该 key 永远不被
    /// 驱逐, lru_cap 形同虚设 (幽灵条目无限泄漏).
    #[test]
    fn persisted_rollback_does_not_leak_lru_entries() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // 先在 sqlite 侧记录 "a" (窗口内).
        let first = DedupIndex::with_config(DedupConfig {
            window_ms: 10_000,
            lru_cap: 8,
        });
        assert!(first.accept_persisted(&store, "ns", "a", 0).unwrap());

        // 新 index (LRU 空, cap=2): b 占一格, 再撞 a 的 sqlite 窗口 → 回滚路径.
        let second = DedupIndex::with_config(DedupConfig {
            window_ms: 10_000,
            lru_cap: 2,
        });
        assert!(second.accept_persisted(&store, "ns", "b", 0).unwrap());
        assert!(
            !second.accept_persisted(&store, "ns", "a", 1).unwrap(),
            "sqlite 窗口内必须拒绝 (触发回滚)"
        );

        // 回滚插入的 "a" 必须可被正常驱逐: 再插两个新 key 把 cap=2 挤爆.
        assert!(second.accept_persisted(&store, "ns", "c", 2).unwrap());
        assert!(second.accept_persisted(&store, "ns", "d", 3).unwrap());
        assert!(
            second.len() <= 2,
            "lru_cap 必须生效 (回滚 key 也进 order 队列), got {}",
            second.len()
        );
    }

    /// M23: accept_persisted 的 SELECT + UPSERT 在同一个 BEGIN IMMEDIATE
    /// 事务内 — 同 identity 跨进程并发都读到无 prev 时, 写锁串行化保证
    /// 只有一个写入者能插入, 另一个看到 prev 后拒绝 (不双重接受).
    /// 这里用同一 store 上多个新 index 的顺序写入固化"窗口内单接受"语义
    /// (回归锚点; 每个新 index 的 LRU 都空, 必须靠 sqlite 行拒绝).
    #[test]
    fn persisted_accepts_once_per_window_across_indexes() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let seed = DedupIndex::with_config(DedupConfig {
            window_ms: 1_000,
            lru_cap: 8,
        });
        assert!(seed.accept_persisted(&store, "ns", "fp", 0).unwrap());
        for now in [0_i64, 10, 20, 30] {
            let idx = DedupIndex::with_config(DedupConfig {
                window_ms: 1_000,
                lru_cap: 8,
            });
            assert!(
                !idx.accept_persisted(&store, "ns", "fp", now).unwrap(),
                "窗口内每个新 index 都必须被 sqlite 拒绝 (now={now})"
            );
        }
        let idx = DedupIndex::with_config(DedupConfig {
            window_ms: 1_000,
            lru_cap: 8,
        });
        assert!(
            idx.accept_persisted(&store, "ns", "fp", 1_000).unwrap(),
            "窗口过期后重新接受"
        );
    }

    #[test]
    fn reaccept_existing_key_does_not_grow_order() {
        let idx = DedupIndex::with_config(DedupConfig {
            window_ms: 10,
            lru_cap: 2,
        });
        assert!(idx.accept("n", "1", 0));
        assert!(idx.accept("n", "1", 20)); // window expired, same key
        assert_eq!(idx.len(), 1);
        assert!(idx.accept("n", "2", 21));
        assert_eq!(idx.len(), 2);
        // cap still 2: inserting "3" evicts "1", not a leaked duplicate of "1"
        assert!(idx.accept("n", "3", 22));
        assert_eq!(idx.len(), 2);
        assert!(idx.accept("n", "1", 23), "1 was the real LRU victim");
    }

    #[test]
    fn overlap_ratio_jaccard() {
        assert_eq!(overlap_ratio("", "abc"), 0.0);
        assert!((overlap_ratio("abc", "abc") - 1.0).abs() < 1e-9);
        let r = overlap_ratio("abcd", "abce");
        assert!((r - 0.6).abs() < 1e-9, "got {r}");
    }

    #[test]
    fn dedup_textual_exact_and_substring() {
        let mut items = vec![
            "Hello   World".into(),
            "hello world".into(),
            "unique fact".into(),
        ];
        dedup_textual(&mut items);
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|s| s == "unique fact"));

        let mut long = vec![
            "abcdefghijklmnopqrstuvwxyz".into(),
            "abcdefghijklmnopqrstuvwxyz extra".into(),
        ];
        dedup_textual(&mut long);
        assert_eq!(long.len(), 1);
        assert!(long[0].contains("extra"), "keep the longer occupant");
    }

    #[test]
    fn dedup_textual_high_overlap_keeps_longer() {
        let a = "the quick brown fox jumps over the lazy dog and then some";
        let b = "the quick brown fox jumps over the lazy dog plus extra words";
        let mut items = vec![a.to_string(), b.to_string()];
        dedup_textual(&mut items);
        assert_eq!(items.len(), 1);
        assert!(items[0].chars().count() >= a.chars().count());
        assert!(items[0].chars().count() >= b.chars().count());
    }

    #[test]
    fn dedup_textual_replacement_is_recompared() {
        // A is short-dup of B (B longer, so A is removed). B then dups C.
        // Donor incremented past the replacement; we must collapse all three.
        let mut items = vec![
            "abcdefghijklmnopqrst".into(),             // 20 chars
            "abcdefghijklmnopqrstuvwxyz".into(),       // longer, contains first
            "abcdefghijklmnopqrstuvwxyz012345".into(), // even longer, contains second
        ];
        dedup_textual(&mut items);
        assert_eq!(items.len(), 1);
        assert!(items[0].ends_with("012345"));
    }
}
