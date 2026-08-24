//! R179 P1-10: Hallway — wing 内 entity-pair 跨位置走廊 (借鉴 mempalace hallways.py).
//!
//! ## 模型
//! - **wing**: 项目 / 主题 (类似 mempalace 的 wing, 不是 apeireth 的 wing 机构术语)
//! - **drawer**: 一条 Note (含 tags / content / source_episode_ids)
//! - **entity**: 一个 tag (例外 wing 自己)
//! - **hallway**: wing 内两个 entity 的连接, 共现次数 >= min_count
//!
//! ## 算法 (借鉴 mempalace compute_hallways_for_wing)
//! 1. 拉所有 Note (filter by min_confidence)
//! 2. 按 wing 分组: note.tags[0] = wing (约定)
//!    — 允许 caller 自定义 wing_of(note) (默认用 tags[0])
//! 3. 对每个 wing:
//!    - 对每条 Note, 取非 wing 的 tags → entity set
//!    - 对每个 entity pair (sorted), 累计 co_occurrence
//!    - co_occurrence >= min_count → materialize Hallway
//!
//! ## apeireth 适配
//! - 存到 SQLite `hallways` 表 (V2 migration)
//! - 不加 append-only trigger (计算后 UPSERT 保留 L7 dynamics)
//! - id = sha256(wing::a::b)[..12], 对称 (与 mempalace 一致)
//! - dynamics: strength / stability / last_activated / access_count
//!   recompute 时保留旧值 (与 mempalace "preserve L7" 一致)
//! - 访问一个 hallway → access_count + 1, last_activated = now
//!   (同 mempalace dynamics layer 语义, 但限于 in-memory mutate;
//!    走 persistence 在 caller 调 `touch_hallway` 时同步落盘)
//!
//! ## 用法
//! ```rust,no_run
//! use apeireth_memory::hallways::{compute_hallways_for_wing, list_hallways};
//! use apeireth_memory::SqliteMemoryStore;
//! use std::sync::Arc;
//!
//! let store = Arc::new(SqliteMemoryStore::open_in_memory()?);
//! let created = compute_hallways_for_wing(&store, "memory-palace", 2)?;
//! println!("created {} new hallways", created.len());
//! let all = list_hallways(&store, None)?;
//! println!("total hallways: {}", all.len());
//! # Ok::<(), apeireth_memory::MemoryError>(())
//! ```

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::session_note::{NoteRecord, NoteStore};
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// Default minimum co-occurrence count required to materialize a hallway
/// (matches mempalace `min_count=2`).
pub fn default_min_count() -> usize {
    2
}

/// Hallway record (persisted to `hallways` table).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hallway {
    /// Deterministic id: sha256("wing::a::b")[..12] (对称).
    pub id: String,
    /// Wing (项目 / 主题).
    pub wing: String,
    /// 两个 entity 中较小的一个 (在 sorted 后).
    pub entity_a: String,
    /// 两个 entity 中较大的一个.
    pub entity_b: String,
    /// 共现次数 (跨 Note).
    pub co_occurrence_count: i64,
    /// 创建时间 (epoch seconds).
    pub created_at: i64,
    /// 最近重算时间.
    pub updated_at: i64,
    /// L7 dynamics: 连接强度 (默认 1.0).
    pub strength: f64,
    /// L7 dynamics: 稳定性 (默认 1.0).
    pub stability: f64,
    /// L7 dynamics: 最近访问时间 (None = 从未).
    pub last_activated: Option<i64>,
    /// L7 dynamics: 累计访问次数.
    pub access_count: i64,
    /// 软删除时间 (None = 活跃).
    pub tombstoned_at: Option<i64>,
}

impl Hallway {
    /// Canonicalize (a, b) → sorted, 保证对称 id 生成.
    pub fn canonical_pair(a: &str, b: &str) -> (String, String) {
        if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }

    /// Compute deterministic id for (wing, a, b).
    pub fn make_id(wing: &str, a: &str, b: &str) -> String {
        let (a, b) = Self::canonical_pair(a, b);
        let key = format!("{wing}::{a}::{b}");
        let mut hasher = Sha1_12::new();
        hasher.update(key.as_bytes());
        let hex = hasher.finalize_hex();
        format!("hallway_{wing}_{a}_{b}_{hex}")
    }

    /// Human-readable label (mempalace style).
    pub fn label(&self) -> String {
        format!(
            "{} ↔ {} (co-occur in {} note{} under wing `{}`)",
            self.entity_a,
            self.entity_b,
            self.co_occurrence_count,
            if self.co_occurrence_count == 1 {
                ""
            } else {
                "s"
            },
            self.wing
        )
    }
}

/// 小型 SHA1 -> 12 位 hex (不拉外部 dep, 就地用 sha1 crate
/// 会在 Cargo.toml 加一行, 这里手撒).
mod sha1_12 {
    pub struct Sha1_12 {
        h: [u32; 5],
        buf: Vec<u8>,
        total: u64,
    }
    impl Sha1_12 {
        pub fn new() -> Self {
            Self {
                h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
                buf: Vec::with_capacity(64),
                total: 0,
            }
        }
        pub fn update(&mut self, data: &[u8]) {
            self.buf.extend_from_slice(data);
            self.total += data.len() as u64;
            while self.buf.len() >= 64 {
                let block: [u8; 64] = self.buf.drain(..64).collect::<Vec<_>>().try_into().unwrap();
                self.process_block(&block);
            }
        }
        pub fn finalize_hex(mut self) -> String {
            let bit_len = self.total * 8;
            self.buf.push(0x80);
            while self.buf.len() % 64 != 56 {
                self.buf.push(0);
            }
            self.buf.extend_from_slice(&bit_len.to_be_bytes());
            while self.buf.len() >= 64 {
                let block: [u8; 64] = self.buf.drain(..64).collect::<Vec<_>>().try_into().unwrap();
                self.process_block(&block);
            }
            let mut out = String::with_capacity(40);
            for v in &self.h {
                out.push_str(&format!("{:08x}", v));
            }
            out
        }
        fn process_block(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    block[i * 4],
                    block[i * 4 + 1],
                    block[i * 4 + 2],
                    block[i * 4 + 3],
                ]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let mut a = self.h[0];
            let mut b = self.h[1];
            let mut c = self.h[2];
            let mut d = self.h[3];
            let mut e = self.h[4];
            for i in 0..80 {
                let (f, k) = if i < 20 {
                    ((b & c) | ((!b) & d), 0x5A827999)
                } else if i < 40 {
                    (b ^ c ^ d, 0x6ED9EBA1)
                } else if i < 60 {
                    ((b & c) | (b & d) | (c & d), 0x8F1BBCDC)
                } else {
                    (b ^ c ^ d, 0xCA62C1D6)
                };
                let temp = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(w[i]);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }
            self.h[0] = self.h[0].wrapping_add(a);
            self.h[1] = self.h[1].wrapping_add(b);
            self.h[2] = self.h[2].wrapping_add(c);
            self.h[3] = self.h[3].wrapping_add(d);
            self.h[4] = self.h[4].wrapping_add(e);
        }
    }
}
use sha1_12::Sha1_12;

/// 默认的 wing 推断: note.tags[0] = wing.
/// 调用者可以自己实现一个 Fn(&NoteRecord) -> Option<String>
/// 传给 `compute_hallways_for_wing_with` 以重定义该趋势.
pub fn default_wing_of(note: &NoteRecord) -> Option<String> {
    note.tags.first().cloned().filter(|s| !s.is_empty())
}

/// 计算一个 wing 内的所有 hallways, 按 co_occurrence_count 降序返回.
/// 不修改调用者提供的状态之外的东西——要落盘
/// 请调 `upsert_hallway` (或 `recompute_all_hallways`).
pub fn compute_hallways(
    notes: &[NoteRecord],
    wing: &str,
    min_count: usize,
    wing_of: &dyn Fn(&NoteRecord) -> Option<String>,
) -> Vec<(String, String, i64)> {
    use std::collections::HashMap;
    let min_count = min_count.max(1);
    let mut pair_counts: HashMap<(String, String), i64> = HashMap::new();
    for note in notes {
        if wing_of(note).as_deref() != Some(wing) {
            continue;
        }
        let mut entities: Vec<String> = note
            .tags
            .iter()
            .filter(|t| t.as_str() != wing)
            .cloned()
            .collect();
        entities.sort();
        entities.dedup();
        if entities.len() < 2 {
            continue;
        }
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let a = entities[i].clone();
                let b = entities[j].clone();
                let (a, b) = Hallway::canonical_pair(&a, &b);
                *pair_counts.entry((a, b)).or_insert(0) += 1;
            }
        }
    }
    let mut out: Vec<(String, String, i64)> = pair_counts
        .into_iter()
        .filter(|(_, c)| *c >= min_count as i64)
        .map(|((a, b), c)| (a, b, c))
        .collect();
    out.sort_by(|x, y| y.2.cmp(&x.2).then_with(|| x.0.cmp(&y.0)));
    out
}

/// Recompute hallways for one wing, preserving existing L7 dynamics
/// (matches mempalace "preserve L7 dynamics" semantics).
pub fn compute_hallways_for_wing(
    store: &SqliteMemoryStore,
    wing: &str,
    min_count: usize,
) -> MemoryResult<Vec<Hallway>> {
    compute_hallways_for_wing_with(store, wing, min_count, &default_wing_of)
}

pub fn compute_hallways_for_wing_with(
    store: &SqliteMemoryStore,
    wing: &str,
    min_count: usize,
    wing_of: &dyn Fn(&NoteRecord) -> Option<String>,
) -> MemoryResult<Vec<Hallway>> {
    if wing.is_empty() {
        return Err(MemoryError::Invalid("wing is empty".into()));
    }
    let notes = <SqliteMemoryStore as NoteStore>::query(
        store,
        &crate::session_note::NoteQuery::new()
            .with_tag(wing)
            .limit(100_000),
    )?;
    let computed = compute_hallways(&notes, wing, min_count, wing_of);
    let now = crate::append_only::now_unix();

    // 保留旧 dynamics (与 mempalace PR #1578 一致)
    let existing = list_hallways_for_wing_internal(&*store.conn()?, wing)?;
    let mut existing_dyn: HashMap<(String, String), (f64, f64, Option<i64>, i64)> = HashMap::new();
    for h in &existing {
        existing_dyn.insert(
            (h.entity_a.clone(), h.entity_b.clone()),
            (h.strength, h.stability, h.last_activated, h.access_count),
        );
    }

    let mut out = Vec::with_capacity(computed.len());
    for (a, b, count) in computed {
        let id = Hallway::make_id(wing, &a, &b);
        let (strength, stability, last_activated, access_count) = existing_dyn
            .get(&(a.clone(), b.clone()))
            .copied()
            .unwrap_or((1.0, 1.0, None, 0));
        let h = Hallway {
            id,
            wing: wing.to_string(),
            entity_a: a,
            entity_b: b,
            co_occurrence_count: count,
            created_at: now,
            updated_at: now,
            strength,
            stability,
            last_activated,
            access_count,
            tombstoned_at: None,
        };
        upsert_hallway_raw(&*store.conn()?, &h)?;
        out.push(h);
    }
    Ok(out)
}

/// UPSERT 一个 hallway (保留 dynamics 只有 caller 明确调用时才会被覆盖).
pub fn upsert_hallway(store: &SqliteMemoryStore, h: &Hallway) -> MemoryResult<()> {
    upsert_hallway_raw(&*store.conn()?, h)
}

fn upsert_hallway_raw(conn: &Connection, h: &Hallway) -> MemoryResult<()> {
    if h.wing.is_empty() {
        return Err(MemoryError::Invalid("hallway.wing is empty".into()));
    }
    if h.entity_a.is_empty() || h.entity_b.is_empty() {
        return Err(MemoryError::Invalid("hallway entity is empty".into()));
    }
    conn.execute(
        "INSERT INTO hallways (
            id, wing, entity_a, entity_b, co_occurrence_count,
            created_at, updated_at, strength, stability,
            last_activated, access_count, tombstoned_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            co_occurrence_count = excluded.co_occurrence_count,
            updated_at = excluded.updated_at,
            tombstoned_at = NULL",
        params![
            h.id,
            h.wing,
            h.entity_a,
            h.entity_b,
            h.co_occurrence_count,
            h.created_at,
            h.updated_at,
            h.strength,
            h.stability,
            h.last_activated,
            h.access_count,
            h.tombstoned_at,
        ],
    )?;
    Ok(())
}

/// 查询 (wing 可选) 的全部 hallways, 过滤 tombstoned.
pub fn list_hallways(store: &SqliteMemoryStore, wing: Option<&str>) -> MemoryResult<Vec<Hallway>> {
    let conn = &*store.conn()?;
    let mut sql = String::from(
        "SELECT id, wing, entity_a, entity_b, co_occurrence_count,
                created_at, updated_at, strength, stability,
                last_activated, access_count, tombstoned_at
         FROM hallways WHERE tombstoned_at IS NULL",
    );
    if wing.is_some() {
        sql.push_str(" AND wing = ?1");
    }
    sql.push_str(" ORDER BY wing ASC, co_occurrence_count DESC, entity_a ASC");
    let mut stmt = conn.prepare(&sql)?;
    let mapper = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Hallway> {
        Ok(Hallway {
            id: row.get(0)?,
            wing: row.get(1)?,
            entity_a: row.get(2)?,
            entity_b: row.get(3)?,
            co_occurrence_count: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            strength: row.get(7)?,
            stability: row.get(8)?,
            last_activated: row.get(9)?,
            access_count: row.get(10)?,
            tombstoned_at: row.get(11)?,
        })
    };
    let rows: Vec<Hallway> = if let Some(w) = wing {
        stmt.query_map(params![w], mapper)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        stmt.query_map([], mapper)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    Ok(rows)
}

fn list_hallways_for_wing_internal(conn: &Connection, wing: &str) -> MemoryResult<Vec<Hallway>> {
    let mut stmt = conn.prepare(
        "SELECT id, wing, entity_a, entity_b, co_occurrence_count,
                created_at, updated_at, strength, stability,
                last_activated, access_count, tombstoned_at
         FROM hallways WHERE wing = ?1",
    )?;
    let rows = stmt
        .query_map(params![wing], |row| {
            Ok(Hallway {
                id: row.get(0)?,
                wing: row.get(1)?,
                entity_a: row.get(2)?,
                entity_b: row.get(3)?,
                co_occurrence_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                strength: row.get(7)?,
                stability: row.get(8)?,
                last_activated: row.get(9)?,
                access_count: row.get(10)?,
                tombstoned_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// 查找包含某个 entity 的全部 hallways (in/out, 跨 wing).
pub fn find_hallways_for_entity(
    store: &SqliteMemoryStore,
    entity: &str,
) -> MemoryResult<Vec<Hallway>> {
    let conn = &*store.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, wing, entity_a, entity_b, co_occurrence_count,
                created_at, updated_at, strength, stability,
                last_activated, access_count, tombstoned_at
         FROM hallways
         WHERE tombstoned_at IS NULL
           AND (entity_a = ?1 OR entity_b = ?1)
         ORDER BY co_occurrence_count DESC, updated_at DESC",
    )?;
    let rows = stmt
        .query_map(params![entity], |row| {
            Ok(Hallway {
                id: row.get(0)?,
                wing: row.get(1)?,
                entity_a: row.get(2)?,
                entity_b: row.get(3)?,
                co_occurrence_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                strength: row.get(7)?,
                stability: row.get(8)?,
                last_activated: row.get(9)?,
                access_count: row.get(10)?,
                tombstoned_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Soft-delete (tombstone) 一个 hallway by id.
pub fn delete_hallway(store: &SqliteMemoryStore, id: &str) -> MemoryResult<bool> {
    let conn = &*store.conn()?;
    let now = crate::append_only::now_unix();
    let updated = conn.execute(
        "UPDATE hallways SET tombstoned_at = ?1
         WHERE id = ?2 AND tombstoned_at IS NULL",
        params![now, id],
    )?;
    Ok(updated > 0)
}

/// 访问一个 hallway: access_count + 1, last_activated = now
/// (同 mempalace dynamics 语义).
pub fn touch_hallway(store: &SqliteMemoryStore, id: &str) -> MemoryResult<()> {
    let conn = &*store.conn()?;
    let now = crate::append_only::now_unix();
    let updated = conn.execute(
        "UPDATE hallways
         SET access_count = access_count + 1, last_activated = ?1
         WHERE id = ?2 AND tombstoned_at IS NULL",
        params![now, id],
    )?;
    if updated == 0 {
        return Err(MemoryError::Invalid(format!("hallway `{id}` not found")));
    }
    Ok(())
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::Note;

    fn make_note(id: &str, ts: i64, content: &str, tags: &[&str]) -> Note {
        Note {
            id: id.into(),
            timestamp: ts,
            content: content.into(),
            source_episode_ids: vec![],
            confidence: 1.0,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn fresh_store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("open memory")
    }

    fn put(store: &SqliteMemoryStore, note: Note) {
        <SqliteMemoryStore as NoteStore>::put_note(store, &note).unwrap();
    }

    /// 1. canonical_pair 对称
    #[test]
    fn canonical_pair_is_symmetric() {
        let (a1, b1) = Hallway::canonical_pair("aya", "lumi");
        let (a2, b2) = Hallway::canonical_pair("lumi", "aya");
        assert_eq!(a1, a2);
        assert_eq!(b1, b2);
        assert_eq!(a1, "aya");
        assert_eq!(b1, "lumi");
    }

    /// 2. make_id 对称: (a, b) 和 (b, a) 同 id
    #[test]
    fn make_id_is_symmetric() {
        let id1 = Hallway::make_id("wing-x", "alpha", "beta");
        let id2 = Hallway::make_id("wing-x", "beta", "alpha");
        assert_eq!(id1, id2);
    }

    /// 3. compute_hallways: 空 notes 返空
    #[test]
    fn compute_empty_returns_empty() {
        let out = compute_hallways(&[], "wing-x", 2, &default_wing_of);
        assert!(out.is_empty());
    }

    /// 4. compute_hallways: 单 entity 不生 pair
    #[test]
    fn compute_single_entity_no_pair() {
        let notes = vec![NoteRecord::from_core(&make_note(
            "n1",
            1,
            "content",
            &["wing-x", "alpha"],
        ))];
        let out = compute_hallways(&notes, "wing-x", 2, &default_wing_of);
        assert!(out.is_empty());
    }

    /// 5. compute_hallways: 2 定主题 pair 共现 → 出 1 个 hallway
    #[test]
    fn compute_pair_appears() {
        let notes = vec![
            NoteRecord::from_core(&make_note("n1", 1, "a", &["wing-x", "alpha", "beta"])),
            NoteRecord::from_core(&make_note("n2", 2, "b", &["wing-x", "alpha", "beta"])),
        ];
        let out = compute_hallways(&notes, "wing-x", 2, &default_wing_of);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "alpha");
        assert_eq!(out[0].1, "beta");
        assert_eq!(out[0].2, 2);
    }

    /// 6. compute_hallways: min_count 越严 越少
    #[test]
    fn compute_min_count_filters_low() {
        let notes = vec![
            NoteRecord::from_core(&make_note("n1", 1, "a", &["wing-x", "alpha", "beta"])),
            NoteRecord::from_core(&make_note("n2", 2, "b", &["wing-x", "alpha", "gamma"])),
        ];
        // alpha-beta 只出现 1 次 → min_count=2 过滤
        let out = compute_hallways(&notes, "wing-x", 2, &default_wing_of);
        assert!(out.is_empty());
        // min_count=1: 2 个 pair (alpha-beta, alpha-gamma) 各 co-occurs 1 次
        let out = compute_hallways(&notes, "wing-x", 1, &default_wing_of);
        assert_eq!(out.len(), 2);
    }

    /// 7. compute_hallways: 跨 wing 不混
    #[test]
    fn compute_filters_by_wing() {
        let notes = vec![
            NoteRecord::from_core(&make_note("n1", 1, "a", &["wing-x", "alpha", "beta"])),
            NoteRecord::from_core(&make_note("n2", 2, "b", &["wing-y", "alpha", "beta"])),
        ];
        // min_count=1: 每个 wing 各自 1 个 pair
        let out_x = compute_hallways(&notes, "wing-x", 1, &default_wing_of);
        let out_y = compute_hallways(&notes, "wing-y", 1, &default_wing_of);
        assert_eq!(out_x.len(), 1);
        assert_eq!(out_y.len(), 1);
        // min_count=2: 各 wing 只 1 note, 0 pair
        let out_x2 = compute_hallways(&notes, "wing-x", 2, &default_wing_of);
        assert_eq!(out_x2.len(), 0);
    }

    /// 8. compute_hallways_for_wing: 真存进 SQLite
    #[test]
    fn compute_for_wing_persists() {
        let store = fresh_store();
        put(
            &store,
            make_note("n1", 1, "a", &["wing-x", "alpha", "beta"]),
        );
        put(
            &store,
            make_note("n2", 2, "b", &["wing-x", "alpha", "beta"]),
        );
        let out = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        assert_eq!(out.len(), 1);
        let id = out[0].id.clone();

        // 重运行 → 应是 update, 不增加行
        let out2 = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        assert_eq!(out2.len(), 1);
        assert_eq!(out2[0].id, id);
        let all = list_hallways(&store, None).unwrap();
        assert_eq!(all.len(), 1);
    }

    /// 9. compute_hallways_for_wing: 保留 dynamics (同 mempalace PR #1578)
    #[test]
    fn compute_preserves_dynamics() {
        let store = fresh_store();
        put(
            &store,
            make_note("n1", 1, "a", &["wing-x", "alpha", "beta"]),
        );
        put(
            &store,
            make_note("n2", 2, "b", &["wing-x", "alpha", "beta"]),
        );
        let out = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        let id = out[0].id.clone();

        // 人为 touch 一下
        touch_hallway(&store, &id).unwrap();
        touch_hallway(&store, &id).unwrap();

        // 重算 → 保留 access_count = 2
        let _ = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        let h = list_hallways(&store, None).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].access_count, 2, "dynamics must survive recompute");
        assert!(h[0].last_activated.is_some());
    }

    /// 10. delete_hallway: 软删除
    #[test]
    fn delete_soft() {
        let store = fresh_store();
        put(
            &store,
            make_note("n1", 1, "a", &["wing-x", "alpha", "beta"]),
        );
        put(
            &store,
            make_note("n2", 2, "b", &["wing-x", "alpha", "beta"]),
        );
        let out = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        let id = out[0].id.clone();
        let ok = delete_hallway(&store, &id).unwrap();
        assert!(ok);
        // list 过滤 tombstoned
        let all = list_hallways(&store, None).unwrap();
        assert!(all.is_empty());
    }

    /// 11. find_hallways_for_entity
    #[test]
    fn find_by_entity() {
        let store = fresh_store();
        put(
            &store,
            make_note("n1", 1, "a", &["wing-x", "alpha", "beta"]),
        );
        put(
            &store,
            make_note("n2", 2, "b", &["wing-x", "alpha", "beta"]),
        );
        put(
            &store,
            make_note("n3", 3, "c", &["wing-x", "alpha", "gamma"]),
        );
        let _ = compute_hallways_for_wing(&store, "wing-x", 2).unwrap();
        let alpha = find_hallways_for_entity(&store, "alpha").unwrap();
        // alpha 出现在 alpha-beta (计 2) + alpha-gamma (计 1, 过滤)
        assert_eq!(alpha.len(), 1);
        assert!(alpha[0].entity_a == "alpha" || alpha[0].entity_b == "alpha");
    }

    /// 12. empty wing 报错
    #[test]
    fn empty_wing_errors() {
        let store = fresh_store();
        assert!(compute_hallways_for_wing(&store, "", 2).is_err());
    }

    /// 13. label 文本生成
    #[test]
    fn label_format() {
        let h = Hallway {
            id: "hallway_x_a_b_xx".into(),
            wing: "wing-x".into(),
            entity_a: "alpha".into(),
            entity_b: "beta".into(),
            co_occurrence_count: 3,
            created_at: 0,
            updated_at: 0,
            strength: 1.0,
            stability: 1.0,
            last_activated: None,
            access_count: 0,
            tombstoned_at: None,
        };
        let lbl = h.label();
        assert!(lbl.contains("alpha"));
        assert!(lbl.contains("beta"));
        assert!(lbl.contains("3 notes"));
    }
}
