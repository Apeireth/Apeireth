//! IdentityCard 跨载体存储 (D2 §4 主体连续性).
//!
//! - 物理唯一性: `continuity_id` UNIQUE 约束, 跨载体去重
//! - 物理保护: 硬 DELETE 被 trigger 拒绝; 软删除 = 设置 `tombstoned_at`
//! - 物理允许: UPDATE 允许 (用于追加 migration_history / 改变 carriers)
//! - API 风格: `IdentityCardStore` trait + `SqliteMemoryStore` 默认实现
//!
//! 与 `apeireth_core::IdentityCard` 关系:
//! - `IdentityCardRecord` 是存储层结构 (含 subject_rev, created_at, tombstoned_at 等)
//! - `IdentityCard` 是核心层公开类型 (continuity_id / birth_time / carriers / migration_history)
//! - `from_core` / `into_core` 双向转换

use rusqlite::{params, ErrorCode, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use apeireth_core::{IdentityCard, Migration};

use crate::append_only::now_unix;
use crate::{MemoryError, MemoryResult};

/// 冲突错误: continuity_id 已存在 / 不存在.
#[derive(Debug, Error)]
pub enum IdentityConflict {
    /// continuity_id 已存在 (D2 §4 跨载体唯一).
    #[error("continuity_id `{0}` already exists (UNIQUE constraint)")]
    AlreadyExists(String),
    /// continuity_id 未找到.
    #[error("continuity_id `{0}` not found")]
    NotFound(String),
}

/// IdentityCard 存储层记录 (含审计字段).
///
/// ⚠️ 与 `apeireth_core::IdentityCard` 不是同一个类型:
/// - core 类型 = 业务对象 (4 字段)
/// - record 类型 = 存储结构 (8 字段, 含 subject_rev / 时间戳 / tombstone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCardRecord {
    /// 跨载体唯一 ID.
    pub continuity_id: String,
    /// 诞生时间戳.
    pub birth_time: i64,
    /// 当前所在载体.
    pub carriers: Vec<String>,
    /// 跨载体迁移历史.
    pub migration_history: Vec<Migration>,
    /// 主体版本号 (D2 §4.3).
    pub subject_rev: i64,
    /// 创建时间.
    pub created_at: i64,
    /// 最后更新时间.
    pub updated_at: i64,
    /// 软删除时间 (`None` = 未删除).
    pub tombstoned_at: Option<i64>,
    /// 软删除原因.
    pub tombstoned_reason: Option<String>,
}

impl IdentityCardRecord {
    /// 从 `apeireth_core::IdentityCard` 构造存储记录.
    pub fn from_core(card: &IdentityCard) -> Self {
        let now = now_unix();
        Self {
            continuity_id: card.continuity_id.clone(),
            birth_time: card.birth_time,
            carriers: card.carriers.clone(),
            migration_history: card.migration_history.clone(),
            subject_rev: 0,
            created_at: now,
            updated_at: now,
            tombstoned_at: None,
            tombstoned_reason: None,
        }
    }

    /// 转回 `apeireth_core::IdentityCard` (丢弃审计字段).
    pub fn into_core(&self) -> IdentityCard {
        IdentityCard {
            continuity_id: self.continuity_id.clone(),
            birth_time: self.birth_time,
            carriers: self.carriers.clone(),
            migration_history: self.migration_history.clone(),
        }
    }
}

/// IdentityCard 存储 trait.
pub trait IdentityCardStore {
    /// 创建新 IdentityCard. `continuity_id` 已存在时报 `IdentityConflict::AlreadyExists`.
    fn create(&self, card: &IdentityCard) -> MemoryResult<IdentityCardRecord>;
    /// 按 `continuity_id` 取记录 (含 tombstoned).
    fn get(&self, continuity_id: &str) -> MemoryResult<Option<IdentityCardRecord>>;
    /// 快速存在性检查.
    fn exists(&self, continuity_id: &str) -> MemoryResult<bool>;
    /// 列出全部 (默认排除 tombstoned).
    fn list(&self, include_tombstoned: bool) -> MemoryResult<Vec<IdentityCardRecord>>;
    /// 按 carrier 查找 (主体在哪些载体上).
    fn list_by_carrier(&self, carrier: &str) -> MemoryResult<Vec<IdentityCardRecord>>;
    /// 追加一次跨载体迁移 (更新 carriers + migration_history).
    fn record_migration(
        &self,
        continuity_id: &str,
        migration: &Migration,
    ) -> MemoryResult<IdentityCardRecord>;
    /// 软删除 (设置 `tombstoned_at` + `tombstoned_reason`).
    fn tombstone(&self, continuity_id: &str, at: i64, reason: &str) -> MemoryResult<()>;
    /// 累计已 tombstone 的 record 数量 (用于监控/审计).
    fn count_tombstoned(&self) -> MemoryResult<i64>;
}

impl crate::SqliteMemoryStore {
    fn map_unique_error(e: rusqlite::Error, continuity_id: &str) -> MemoryError {
        if let rusqlite::Error::SqliteFailure(err, msg) = &e {
            if err.code == ErrorCode::ConstraintViolation {
                return MemoryError::Identity(IdentityConflict::AlreadyExists(
                    continuity_id.into(),
                ));
            }
            // 回退: 字符串匹配 (兼容 bundled SQLite 行为)
            let combined = format!("{:?} {}", err, msg.as_deref().unwrap_or(""));
            if combined.contains("UNIQUE") {
                return MemoryError::Identity(IdentityConflict::AlreadyExists(
                    continuity_id.into(),
                ));
            }
        }
        MemoryError::Sqlite(e)
    }
}

impl IdentityCardStore for crate::SqliteMemoryStore {
    fn create(&self, card: &IdentityCard) -> MemoryResult<IdentityCardRecord> {
        validate_continuity_id(&card.continuity_id)?;
        let record = IdentityCardRecord::from_core(card);
        let conn = self.conn()?;
        let carriers_json = serde_json::to_string(&record.carriers)?;
        let history_json = serde_json::to_string(&record.migration_history)?;
        let result = conn.execute(
            "INSERT INTO identity_cards
                (continuity_id, birth_time, carriers_json, migration_history_json,
                 subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL)",
            params![
                record.continuity_id,
                record.birth_time,
                carriers_json,
                history_json,
                record.subject_rev,
                record.created_at,
                record.updated_at,
            ],
        );
        match result {
            Ok(_) => Ok(record),
            Err(e) => Err(Self::map_unique_error(e, &card.continuity_id)),
        }
    }

    fn get(&self, continuity_id: &str) -> MemoryResult<Option<IdentityCardRecord>> {
        if continuity_id.trim().is_empty() {
            return Err(MemoryError::Invalid("continuity_id is empty".into()));
        }
        let conn = self.conn()?;
        let row: Option<RawIdentityRow> = conn
            .query_row(
                "SELECT continuity_id, birth_time, carriers_json, migration_history_json,
                        subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
                 FROM identity_cards WHERE continuity_id = ?1",
                params![continuity_id],
                |row| {
                    Ok(RawIdentityRow {
                        continuity_id: row.get(0)?,
                        birth_time: row.get(1)?,
                        carriers_json: row.get(2)?,
                        history_json: row.get(3)?,
                        subject_rev: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        tombstoned_at: row.get(7)?,
                        tombstoned_reason: row.get(8)?,
                    })
                },
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some(r) => Ok(Some(r.into_record()?)),
        }
    }

    fn exists(&self, continuity_id: &str) -> MemoryResult<bool> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM identity_cards WHERE continuity_id = ?1",
            params![continuity_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn list(&self, include_tombstoned: bool) -> MemoryResult<Vec<IdentityCardRecord>> {
        let conn = self.conn()?;
        let sql = if include_tombstoned {
            "SELECT continuity_id, birth_time, carriers_json, migration_history_json,
                    subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
             FROM identity_cards ORDER BY birth_time ASC"
        } else {
            "SELECT continuity_id, birth_time, carriers_json, migration_history_json,
                    subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
             FROM identity_cards WHERE tombstoned_at IS NULL
             ORDER BY birth_time ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(RawIdentityRow {
                continuity_id: row.get(0)?,
                birth_time: row.get(1)?,
                carriers_json: row.get(2)?,
                history_json: row.get(3)?,
                subject_rev: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                tombstoned_at: row.get(7)?,
                tombstoned_reason: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?.into_record()?);
        }
        Ok(out)
    }

    fn list_by_carrier(&self, carrier: &str) -> MemoryResult<Vec<IdentityCardRecord>> {
        if carrier.trim().is_empty() {
            return Err(MemoryError::Invalid("carrier is empty".into()));
        }
        let conn = self.conn()?;
        // carriers_json 是 JSON array 字符串, 用 LIKE 匹配 `"<carrier>"` 形式避免部分匹配.
        let needle = format!("\"{}\"", carrier.replace('"', "\\\""));
        let mut stmt = conn.prepare(
            "SELECT continuity_id, birth_time, carriers_json, migration_history_json,
                    subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
             FROM identity_cards
             WHERE tombstoned_at IS NULL AND carriers_json LIKE ?1
             ORDER BY birth_time ASC",
        )?;
        let rows = stmt.query_map(params![format!("%{}%", needle)], |row| {
            Ok(RawIdentityRow {
                continuity_id: row.get(0)?,
                birth_time: row.get(1)?,
                carriers_json: row.get(2)?,
                history_json: row.get(3)?,
                subject_rev: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                tombstoned_at: row.get(7)?,
                tombstoned_reason: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?.into_record()?);
        }
        Ok(out)
    }

    fn record_migration(
        &self,
        continuity_id: &str,
        migration: &Migration,
    ) -> MemoryResult<IdentityCardRecord> {
        // 读出现有 → 修改 → UPDATE
        let mut record = self.get(continuity_id)?.ok_or_else(|| {
            MemoryError::Identity(IdentityConflict::NotFound(continuity_id.into()))
        })?;
        if record.tombstoned_at.is_some() {
            return Err(MemoryError::Invalid(format!(
                "cannot migrate tombstoned identity `{continuity_id}`"
            )));
        }
        if !record.carriers.contains(&migration.to_carrier) {
            record.carriers.push(migration.to_carrier.clone());
        }
        record.migration_history.push(migration.clone());
        record.subject_rev += 1;
        record.updated_at = now_unix();

        let conn = self.conn()?;
        let carriers_json = serde_json::to_string(&record.carriers)?;
        let history_json = serde_json::to_string(&record.migration_history)?;
        conn.execute(
            "UPDATE identity_cards
             SET carriers_json = ?1, migration_history_json = ?2,
                 subject_rev = ?3, updated_at = ?4
             WHERE continuity_id = ?5",
            params![
                carriers_json,
                history_json,
                record.subject_rev,
                record.updated_at,
                record.continuity_id
            ],
        )?;
        Ok(record)
    }

    fn tombstone(&self, continuity_id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        if continuity_id.trim().is_empty() {
            return Err(MemoryError::Invalid("continuity_id is empty".into()));
        }
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE identity_cards
             SET tombstoned_at = ?1, tombstoned_reason = ?2, updated_at = ?1
             WHERE continuity_id = ?3 AND tombstoned_at IS NULL",
            params![at, reason, continuity_id],
        )?;
        if updated == 0 {
            // 不存在 OR 已经 tombstoned. R163: 查询 EXISTS 仅为消 unused warning;
            // 语义保持: tombstone 状态由 updated == 0 + exists == true 隐式表达.
            let _exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM identity_cards WHERE continuity_id = ?1)",
                params![continuity_id],
                |row| row.get(0),
            )?;
            return Err(MemoryError::Identity(IdentityConflict::NotFound(
                continuity_id.to_string(),
            )));
        }
        Ok(())
    }

    fn count_tombstoned(&self) -> MemoryResult<i64> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM identity_cards WHERE tombstoned_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

/// 内部 helper: SQLite 行 → IdentityCardRecord.
struct RawIdentityRow {
    continuity_id: String,
    birth_time: i64,
    carriers_json: String,
    history_json: String,
    subject_rev: i64,
    created_at: i64,
    updated_at: i64,
    tombstoned_at: Option<i64>,
    tombstoned_reason: Option<String>,
}

impl RawIdentityRow {
    fn into_record(self) -> MemoryResult<IdentityCardRecord> {
        let carriers: Vec<String> = serde_json::from_str(&self.carriers_json)?;
        let history: Vec<Migration> = serde_json::from_str(&self.history_json)?;
        Ok(IdentityCardRecord {
            continuity_id: self.continuity_id,
            birth_time: self.birth_time,
            carriers,
            migration_history: history,
            subject_rev: self.subject_rev,
            created_at: self.created_at,
            updated_at: self.updated_at,
            tombstoned_at: self.tombstoned_at,
            tombstoned_reason: self.tombstoned_reason,
        })
    }
}

fn validate_continuity_id(id: &str) -> MemoryResult<()> {
    if id.trim().is_empty() {
        return Err(MemoryError::Invalid("continuity_id is empty".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    fn make_card(id: &str) -> IdentityCard {
        IdentityCard {
            continuity_id: id.into(),
            birth_time: 1_700_000_000,
            carriers: vec!["c1".into()],
            migration_history: vec![],
        }
    }

    #[test]
    fn create_and_get_roundtrip() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let rec =
            <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("cid-1")).unwrap();
        assert_eq!(rec.continuity_id, "cid-1");
        let got = <SqliteMemoryStore as IdentityCardStore>::get(&store, "cid-1")
            .unwrap()
            .unwrap();
        assert_eq!(got.continuity_id, "cid-1");
        assert!(got.tombstoned_at.is_none());
    }

    #[test]
    fn duplicate_create_returns_already_exists_conflict() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("dup")).unwrap();
        let err = <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("dup"))
            .unwrap_err();
        match err {
            MemoryError::Identity(IdentityConflict::AlreadyExists(id)) => {
                assert_eq!(id, "dup")
            }
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    #[test]
    fn record_migration_appends_and_bumps_rev() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("mig")).unwrap();
        let m = Migration {
            from_carrier: "c1".into(),
            to_carrier: "c2".into(),
            timestamp: 1_700_000_500,
        };
        let rec =
            <SqliteMemoryStore as IdentityCardStore>::record_migration(&store, "mig", &m).unwrap();
        assert_eq!(rec.subject_rev, 1);
        assert_eq!(rec.carriers, vec!["c1".to_string(), "c2".to_string()]);
        assert_eq!(rec.migration_history.len(), 1);
    }

    #[test]
    fn tombstone_marks_and_excludes_from_list_by_default() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("t1")).unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("t2")).unwrap();
        <SqliteMemoryStore as IdentityCardStore>::tombstone(&store, "t1", 1_700_000_900, "retired")
            .unwrap();
        let live = <SqliteMemoryStore as IdentityCardStore>::list(&store, false).unwrap();
        assert_eq!(live.len(), 1);
        let all = <SqliteMemoryStore as IdentityCardStore>::list(&store, true).unwrap();
        assert_eq!(all.len(), 2);
        let count = <SqliteMemoryStore as IdentityCardStore>::count_tombstoned(&store).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn hard_delete_blocked_by_trigger() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("hd")).unwrap();
        let conn = store.conn().unwrap();
        let err = conn
            .execute("DELETE FROM identity_cards WHERE continuity_id = 'hd'", [])
            .unwrap_err();
        assert!(
            err.to_string().contains("identity_cards"),
            "expected trigger to abort, got {err}"
        );
    }

    #[test]
    fn list_by_carrier_finds_matching_records() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("k1")).unwrap();
        let mut c2 = make_card("k2");
        c2.carriers = vec!["c2".into()];
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &c2).unwrap();
        let rows = <SqliteMemoryStore as IdentityCardStore>::list_by_carrier(&store, "c2").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].continuity_id, "k2");
    }

    #[test]
    fn record_migration_rejects_tombstoned() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &make_card("z")).unwrap();
        <SqliteMemoryStore as IdentityCardStore>::tombstone(&store, "z", 100, "x").unwrap();
        let m = Migration {
            from_carrier: "c1".into(),
            to_carrier: "c9".into(),
            timestamp: 200,
        };
        let err = <SqliteMemoryStore as IdentityCardStore>::record_migration(&store, "z", &m)
            .unwrap_err();
        assert!(
            err.to_string().contains("tombstoned"),
            "expected tombstoned guard, got {err}"
        );
    }
}
