//! Schema migrations for `apeireth-memory`.
//!
//! 按主人偏好: 直接 SQL, 不引入 ORM. 每次新 schema 改动都追加一条新 entry
//! 到 [`MIGRATIONS`], 不可修改历史 (migrations are append-only too).
//!
//! 命名规范: `V<version>__<short_description>`.

use rusqlite::{params, Connection, Transaction};

use crate::MemoryError;
use crate::MemoryResult;

/// 单条 schema migration.
pub struct Migration {
    /// 顺序版本号 (单调递增).
    pub version: i64,
    /// 人类可读名称.
    pub name: &'static str,
    /// 该 migration 实际执行的 SQL (可在事务中批量执行).
    pub sql: &'static str,
}

/// 全部已实装 migrations.
///
/// ⚠️ Append-only: 不要修改既有 entry, 只能追加新 entry.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "V1__init_six_history_streams",
        sql: INIT_SQL,
    },
    // R179 P1-10: Hallway 表 (wing 内 entity-pair co-occurrence)
    // 跟 6 历史流不一样: 不加 append-only trigger, 允许 recompute 时 UPSERT
    // (mempalace "preserve L7 dynamics" — strength/stability 必须跨 recompute 保留).
    Migration {
        version: 2,
        name: "V2__hallways_for_wings",
        sql: HALLWAYS_SQL,
    },
    // M5 (记忆调研批⭐): notes 时间有效性 valid_from/valid_until.
    // 向后兼容铁律: 两列均 NULLable, 存量行 ALTER 后自动 NULL = 永久有效, 零数据迁移.
    // SQLite ALTER TABLE 一次只能加一列 → 两条语句. 不加索引: 过滤条件含
    // "IS NULL OR" 析取本身非 sargable, notes 表规模下全扫足够 (升级路径: 表大后
    // 可加 valid_until 部分索引, 见 docs/backlog.md M5 记录).
    Migration {
        version: 3,
        name: "V3__notes_validity_window",
        sql: "ALTER TABLE notes ADD COLUMN valid_from INTEGER;\n\
              ALTER TABLE notes ADD COLUMN valid_until INTEGER;",
    },
    // TP24 (M5 + N25): episodes 表加 4 列元数据 — 来源链 (provenance) + 时间窗 (ms 精度).
    // 加列而非改列 (per task 纪律): 存量行 4 列均为 NULL, 读取时按 normalize_meta 兜底:
    //   provenance NULL → Manual, valid_from NULL → created_ms, valid_until NULL → None (永久),
    //   created_ms NULL → timestamp * 1000 (s → ms 兜).
    // 不加索引: V4 查询条件含 "IS NULL OR" 析取本身非 sargable, episodes 表规模下全扫足够.
    // 升级路径: 表大后可加 created_ms 部分索引 / 表达式索引, 见 docs/backlog.md M5 记录.
    // SQLite ALTER TABLE 一次一列 → 4 条语句.
    Migration {
        version: 4,
        name: "V4__episodes_provenance_and_timing",
        sql: "ALTER TABLE episodes ADD COLUMN valid_from_ms INTEGER;\n\
              ALTER TABLE episodes ADD COLUMN valid_until_ms INTEGER;\n\
              ALTER TABLE episodes ADD COLUMN created_ms INTEGER;\n\
              ALTER TABLE episodes ADD COLUMN provenance TEXT;\n\
              CREATE INDEX IF NOT EXISTS idx_episodes_created_ms ON episodes(created_ms);",
    },
];

const HALLWAYS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS hallways (
    id                 TEXT PRIMARY KEY,
    wing               TEXT NOT NULL,
    entity_a           TEXT NOT NULL,
    entity_b           TEXT NOT NULL,
    co_occurrence_count INTEGER NOT NULL,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    -- L7 dynamics (mempalace preserve-on-recompute)
    strength           REAL NOT NULL DEFAULT 1.0,
    stability          REAL NOT NULL DEFAULT 1.0,
    last_activated     INTEGER,
    access_count       INTEGER NOT NULL DEFAULT 0,
    tombstoned_at      INTEGER
);

CREATE INDEX IF NOT EXISTS idx_hallways_wing ON hallways(wing);
CREATE INDEX IF NOT EXISTS idx_hallways_pair ON hallways(entity_a, entity_b);
CREATE INDEX IF NOT EXISTS idx_hallways_entity_a ON hallways(entity_a);
CREATE INDEX IF NOT EXISTS idx_hallways_entity_b ON hallways(entity_b);
"#;

const INIT_SQL: &str = r#"
-- === A4 主目标: 6 历史流 Append-only Log ===
-- 表命名严格按 StreamKind::table_name() 一一对应, 6 张独立表 (非 1 张混合表).

CREATE TABLE IF NOT EXISTS thought_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

CREATE TABLE IF NOT EXISTS proposal_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

CREATE TABLE IF NOT EXISTS action_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

CREATE TABLE IF NOT EXISTS relation_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

CREATE TABLE IF NOT EXISTS evolution_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

CREATE TABLE IF NOT EXISTS reflection_stream (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    subject_rev  INTEGER NOT NULL,
    session_id   TEXT,
    created_at   INTEGER NOT NULL,
    payload      TEXT NOT NULL,
    source       TEXT NOT NULL,
    tags         TEXT NOT NULL DEFAULT '[]',
    tombstoned_at INTEGER
);

-- 6 张表共享的索引: (subject_id, created_at) 便于按主体按时间窗查询.
CREATE INDEX IF NOT EXISTS idx_thought_subject      ON thought_stream(subject_id, created_at);
CREATE INDEX IF NOT EXISTS idx_proposal_subject     ON proposal_stream(subject_id, created_at);
CREATE INDEX IF NOT EXISTS idx_action_subject       ON action_stream(subject_id, created_at);
CREATE INDEX IF NOT EXISTS idx_relation_subject     ON relation_stream(subject_id, created_at);
CREATE INDEX IF NOT EXISTS idx_evolution_subject    ON evolution_stream(subject_id, created_at);
CREATE INDEX IF NOT EXISTS idx_reflection_subject   ON reflection_stream(subject_id, created_at);

-- === IdentityCard 跨载体唯一 (D2 §4 主体连续性) ===
-- 物理上允许 UPDATE (用于: 跨载体迁移追加 migration_history、软删除设置 tombstoned_at)
-- 物理上禁止硬 DELETE (主体连续性记录不可抹除, 只能 tombstone)
CREATE TABLE IF NOT EXISTS identity_cards (
    continuity_id    TEXT PRIMARY KEY,        -- 跨载体唯一
    birth_time       INTEGER NOT NULL,
    carriers_json    TEXT NOT NULL DEFAULT '[]',
    migration_history_json TEXT NOT NULL DEFAULT '[]',
    subject_rev      INTEGER NOT NULL DEFAULT 0,  -- D2 §4.3 主体版本号
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    tombstoned_at    INTEGER,                       -- 软删除标记
    tombstoned_reason TEXT
);
CREATE INDEX IF NOT EXISTS idx_identity_cards_birth  ON identity_cards(birth_time);
CREATE INDEX IF NOT EXISTS idx_identity_cards_tomb   ON identity_cards(tombstoned_at);
CREATE TRIGGER IF NOT EXISTS identity_cards_no_delete
BEFORE DELETE ON identity_cards
BEGIN
    SELECT RAISE(ABORT, 'identity_cards: hard DELETE forbidden, use tombstone() instead');
END;

-- === Episode 表 (按 session_id / time range / continuity_id 索引查询) ===
CREATE TABLE IF NOT EXISTS episodes (
    id           TEXT PRIMARY KEY,
    continuity_id TEXT NOT NULL,               -- 跨载体主体引用
    session_id   TEXT NOT NULL,
    timestamp    INTEGER NOT NULL,
    role         TEXT NOT NULL,
    content      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_episodes_session   ON episodes(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_episodes_subject   ON episodes(continuity_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_episodes_time      ON episodes(timestamp);

-- Episode 同样 append-only: 写入即不可变.
CREATE TRIGGER IF NOT EXISTS episodes_no_update
BEFORE UPDATE ON episodes
BEGIN
    SELECT RAISE(ABORT, 'episodes is append-only: cannot mutate historical events');
END;
CREATE TRIGGER IF NOT EXISTS episodes_no_delete
BEFORE DELETE ON episodes
BEGIN
    SELECT RAISE(ABORT, 'episodes is append-only: tombstone via reflection_stream instead');
END;

-- === Session + Note (A1 兼容 + Note 可更新) ===
CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    started_at    INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    closed_at     INTEGER
);

CREATE TABLE IF NOT EXISTS notes (
    id                 TEXT PRIMARY KEY,
    timestamp          INTEGER NOT NULL,
    content            TEXT NOT NULL,
    source_episode_ids_json TEXT NOT NULL DEFAULT '[]',
    confidence         REAL NOT NULL DEFAULT 0.5,
    tags_json          TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_notes_time ON notes(timestamp);
CREATE INDEX IF NOT EXISTS idx_notes_conf ON notes(confidence);
-- Note 可被遗忘/合并 (apeireth_core::Note 文档明确), 不强制 append-only.

-- === Migration tracking ===
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  INTEGER NOT NULL
);
"#;

const APPEND_ONLY_TRIGGERS: &[(&str, &str)] = &[
    (
        "thought_stream",
        "thought_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
    (
        "proposal_stream",
        "proposal_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
    (
        "action_stream",
        "action_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
    (
        "relation_stream",
        "relation_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
    (
        "evolution_stream",
        "evolution_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
    (
        "reflection_stream",
        "reflection_stream is append-only: D2 §5.3 #1 prohibits in-place mutation",
    ),
];

/// 应用全部未执行的 migrations.
///
/// 事务保护: 单条 migration 内所有 DDL 在一个事务内执行, 失败回滚.
pub fn run_migrations(conn: &mut Connection) -> MemoryResult<()> {
    // 1. 建表总是先跑一次 (CREATE IF NOT EXISTS) — 后续 migration 再叠加 trigger.
    let tx = conn.transaction()?;
    tx.execute_batch(INIT_SQL)?;
    tx.commit()?;

    // 2. 注册 6 流 append-only triggers (幂等).
    // 规则: 禁止任何原地修改; 唯一例外是 "软删除" (tombstoned_at 从 NULL 变为非 NULL).
    // 软删除也是不可逆的 (OLD.tombstoned_at 必须是 NULL).
    for (table, reason) in APPEND_ONLY_TRIGGERS {
        let update_trigger = format!(
            "CREATE TRIGGER IF NOT EXISTS {table}_no_inplace_update
             BEFORE UPDATE ON {table}
             FOR EACH ROW
             WHEN NOT (NEW.tombstoned_at IS NOT NULL AND OLD.tombstoned_at IS NULL)
             BEGIN
                 SELECT RAISE(ABORT, '{reason}');
             END;"
        );
        let delete_trigger = format!(
            "CREATE TRIGGER IF NOT EXISTS {table}_no_delete
             BEFORE DELETE ON {table}
             BEGIN
                 SELECT RAISE(ABORT, '{reason}');
             END;"
        );
        conn.execute_batch(&update_trigger)?;
        conn.execute_batch(&delete_trigger)?;
    }

    // 3. 记录已应用的 migration.
    for m in MIGRATIONS {
        if migration_applied(conn, m.version)? {
            continue;
        }
        let tx: Transaction<'_> = conn.transaction()?;
        tx.execute_batch(m.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![m.version, m.name, now_unix()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

fn migration_applied(conn: &Connection, version: i64) -> MemoryResult<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![version],
            |row| row.get(0),
        )
        .map_err(MemoryError::from)?;
    Ok(count > 0)
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    #[test]
    fn migrations_apply_idempotently() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let first = store.applied_migrations().unwrap();
        assert!(first.contains(&1), "expected V1 applied");
        // 二次 open: 不应重复插入.
        let conn = store.conn().expect("store conn");
        let mut guard = conn;
        run_migrations(&mut guard).unwrap();
        let second = SqliteMemoryStore::open_in_memory()
            .unwrap()
            .applied_migrations()
            .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn append_only_triggers_reject_inplace_update_and_delete() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().expect("store conn");
        // 直接 raw SQL 验证: 写入一行后, UPDATE 必失败.
        conn.execute(
            "INSERT INTO thought_stream (id, subject_id, subject_rev, created_at, payload, source)
             VALUES ('t1', 'subj', 1, 0, 'p', 'unit')",
            [],
        )
        .unwrap();
        // 改 payload 应被 trigger 拒绝
        let update_err = conn
            .execute(
                "UPDATE thought_stream SET payload = 'x' WHERE id = 't1'",
                [],
            )
            .unwrap_err();
        assert!(
            update_err.to_string().contains("append-only"),
            "expected append-only error, got {update_err}"
        );
        // 改 subject_id 也应被拒绝
        let subj_err = conn
            .execute(
                "UPDATE thought_stream SET subject_id = 'new' WHERE id = 't1'",
                [],
            )
            .unwrap_err();
        assert!(
            subj_err.to_string().contains("append-only"),
            "got {subj_err}"
        );
        // 软删除 (tombstoned_at 从 NULL → 非 NULL) 应被允许
        conn.execute(
            "UPDATE thought_stream SET tombstoned_at = 1000 WHERE id = 't1'",
            [],
        )
        .expect("soft delete should be allowed");
        // 二次软删除 (tombstoned_at 已非 NULL) 应被拒绝
        let re_tomb = conn
            .execute(
                "UPDATE thought_stream SET tombstoned_at = 2000 WHERE id = 't1'",
                [],
            )
            .unwrap_err();
        assert!(re_tomb.to_string().contains("append-only"), "got {re_tomb}");
        // 物理 DELETE 仍被拒绝
        let delete_err = conn
            .execute("DELETE FROM thought_stream WHERE id = 't1'", [])
            .unwrap_err();
        assert!(delete_err.to_string().contains("append-only"));
    }
}
