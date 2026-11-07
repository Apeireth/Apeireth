//! Schema migrations for `apeireth-memory`.
//!
//! 按主人偏好: 直接 SQL, 不引入 ORM. 每次新 schema 改动都追加一条新 entry
//! 到 [`MIGRATIONS`], 不可修改历史 (migrations are append-only too).
//!
//! 命名规范: `V<version>__<short_description>`.

use std::time::{Duration, Instant};

use rusqlite::{params, Connection, Transaction, TransactionBehavior};

use apeireth_storage::{SqliteConnectionPool, StorageError};

use crate::MemoryError;
use crate::MemoryResult;

/// 迁移**列表**（拆到独立文件 ⇒ 见该文件的模块文档说明理由）。
mod list;
pub use list::MIGRATIONS;
/// 单条 schema migration.
pub struct Migration {
    /// 顺序版本号 (单调递增).
    pub version: i64,
    /// 人类可读名称.
    pub name: &'static str,
    /// 该 migration 实际执行的 SQL (可在事务中批量执行).
    ///
    /// **允许是空串**: 静态 SQL 表达不了的迁移把 DDL 挂到 [`Migration::idempotent`]
    /// 钩子上 ⇒ 表/列的定义只在**一处**（不写第二份 ⇒ 两份口径漂移是本仓反复吃过的亏）.
    pub sql: &'static str,
    /// 可选的「幂等 DDL 钩子」—— 静态 SQL 表达不了的迁移用它.
    ///
    /// **为什么需要**: SQLite 的 `ALTER TABLE … ADD COLUMN` 没有 `IF NOT EXISTS`
    /// ⇒ 纯静态 SQL 写不出幂等的加列迁移 ⇒ 只能先探测列集再 ALTER（需要活连接）.
    /// **语义**: 与 `sql` 同事务、`sql` 之后执行; 失败 ⇒ 回滚并如实报错
    /// (绝不静默跳过: 静默跳过 = 老库悄悄失去链保护).
    pub idempotent: Option<fn(&Connection) -> MemoryResult<()>>,
}

/// 全部已实装 migrations.
///
/// ⚠️ Append-only: 不要修改既有 entry, 只能追加新 entry.
/// V19 的幂等 DDL: 复用 `memory_chain::ensure_chain_schema()`（单一真源）。
///
/// **为什么复用而不是抄一份 SQL**: 建表 + 加列的 DDL 若在迁移表和链模块各写
/// 一份, 将来改一处忘另一处 ⇒ 老库和新库结构分叉（两份口径漂移）。
///
/// **失败语义**: 加列失败一律如实报错（整个迁移事务回滚）—— 静默跳过 =
/// 老库悄悄失去链保护, 比没有链更糟（它会让人以为数据受保护）。
fn v19_memory_chain_schema(conn: &Connection) -> MemoryResult<()> {
    crate::memory_chain::ensure_chain_schema(conn)
}

pub(super) const V17_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS source_hash (
    source        TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    first_seen_ms INTEGER NOT NULL,
    last_seen_ms  INTEGER NOT NULL,
    ingest_count  INTEGER NOT NULL DEFAULT 0,
    skip_count    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (source, content_hash)
);
CREATE INDEX IF NOT EXISTS idx_source_hash_last_seen
    ON source_hash(last_seen_ms);
CREATE TRIGGER IF NOT EXISTS source_hash_no_delete
BEFORE DELETE ON source_hash BEGIN
    SELECT RAISE(ABORT, 'source_hash: hard DELETE forbidden (ledger append-only)');
END;

CREATE TABLE IF NOT EXISTS ingest_queue (
    id            TEXT PRIMARY KEY,
    source        TEXT NOT NULL,
    topic         TEXT NOT NULL,
    content       TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    state         TEXT NOT NULL,
    attempts      INTEGER NOT NULL DEFAULT 0,
    last_error    TEXT,
    created_ms    INTEGER NOT NULL,
    updated_ms    INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_ingest_queue_source_hash
    ON ingest_queue(source, content_hash);
CREATE INDEX IF NOT EXISTS idx_ingest_queue_state
    ON ingest_queue(state, created_ms);

CREATE TABLE IF NOT EXISTS memory_fold_segments (
    topic         TEXT NOT NULL,
    seq           INTEGER NOT NULL,
    content       TEXT NOT NULL,
    content_hash  TEXT NOT NULL,
    folded_at_ms  INTEGER NOT NULL,
    PRIMARY KEY (topic, seq)
);
CREATE TRIGGER IF NOT EXISTS memory_fold_segments_no_rewrite
BEFORE UPDATE ON memory_fold_segments BEGIN
    SELECT RAISE(ABORT, 'memory_fold_segments: rewrite forbidden (append-only)');
END;
CREATE TRIGGER IF NOT EXISTS memory_fold_segments_no_delete
BEFORE DELETE ON memory_fold_segments BEGIN
    SELECT RAISE(ABORT, 'memory_fold_segments: hard DELETE forbidden');
END;
"#;

pub(super) const V16_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS persona_memory_entries (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL,
    text         TEXT NOT NULL,
    sources_json TEXT NOT NULL,
    polarity     TEXT NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    protection   TEXT NOT NULL DEFAULT 'on'
);
CREATE INDEX IF NOT EXISTS idx_persona_memory_entries_kind
    ON persona_memory_entries(kind);

CREATE TABLE IF NOT EXISTS persona_memory_tensions (
    id                TEXT PRIMARY KEY,
    kind              TEXT NOT NULL,
    existing_entry_id TEXT NOT NULL,
    existing_text     TEXT NOT NULL,
    incoming_text     TEXT NOT NULL,
    note              TEXT NOT NULL,
    detected_at       INTEGER NOT NULL,
    resolved          INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS persona_memory_state (
    singleton              INTEGER PRIMARY KEY CHECK (singleton = 1),
    self_narrative         TEXT NOT NULL,
    updated_at             INTEGER NOT NULL,
    last_consolidation_at  INTEGER
);
"#;

pub(super) const V14_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memory_principal_sessions (
    principal_id TEXT NOT NULL,
    session_id TEXT NOT NULL PRIMARY KEY,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memory_principal_sessions_principal
    ON memory_principal_sessions(principal_id, session_id);

CREATE TABLE IF NOT EXISTS persona_profile_governance (
    persona_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    tombstoned_at_ms INTEGER,
    protected INTEGER NOT NULL DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER,
    reason TEXT,
    PRIMARY KEY (persona_id, subject_id)
);
CREATE INDEX IF NOT EXISTS idx_persona_profile_governance_visibility
    ON persona_profile_governance(subject_id, tombstoned_at_ms, protected);
CREATE TRIGGER IF NOT EXISTS persona_profile_governance_no_delete
BEFORE DELETE ON persona_profile_governance BEGIN
    SELECT RAISE(ABORT, 'persona_profile_governance: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS persona_profile_governance_no_tombstone_reversal
BEFORE UPDATE ON persona_profile_governance
FOR EACH ROW
WHEN OLD.tombstoned_at_ms IS NOT NULL AND NEW.tombstoned_at_ms IS NULL
BEGIN
    SELECT RAISE(ABORT, 'persona_profile_governance: tombstone reversal forbidden');
END;
"#;

pub(super) const V13_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS temporal_graph_facts (
    id TEXT PRIMARY KEY,
    relation_key TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object_id TEXT NOT NULL,
    valid_from_ms INTEGER NOT NULL,
    valid_until_ms INTEGER,
    believed_at_ms INTEGER NOT NULL,
    source_episode_id TEXT,
    scope_json TEXT NOT NULL DEFAULT '{}',
    provenance_json TEXT NOT NULL DEFAULT '{}',
    confidence REAL NOT NULL DEFAULT 0.5,
    revision INTEGER NOT NULL DEFAULT 0,
    supersedes_id TEXT,
    retracted_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_relation
    ON temporal_graph_facts(relation_key, valid_from_ms);
CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_subject
    ON temporal_graph_facts(subject_id, predicate, valid_from_ms);
CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_object
    ON temporal_graph_facts(object_id, predicate, valid_from_ms);
CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_active
    ON temporal_graph_facts(subject_id, valid_until_ms, retracted_at_ms);
CREATE INDEX IF NOT EXISTS idx_temporal_graph_facts_source
    ON temporal_graph_facts(source_episode_id);

CREATE TABLE IF NOT EXISTS memory_access_events (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    subject_id TEXT,
    session_id TEXT,
    accessed_at_ms INTEGER NOT NULL,
    query TEXT,
    access_kind TEXT NOT NULL DEFAULT 'recall',
    rank INTEGER,
    score REAL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_memory_access_events_memory
    ON memory_access_events(memory_id, accessed_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_memory_access_events_subject_time
    ON memory_access_events(subject_id, accessed_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_memory_access_events_session_time
    ON memory_access_events(session_id, accessed_at_ms DESC);

CREATE TABLE IF NOT EXISTS memory_access_aggregates (
    memory_id TEXT NOT NULL,
    bucket_start_ms INTEGER NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at_ms INTEGER,
    cumulative_score REAL NOT NULL DEFAULT 0.0,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (memory_id, bucket_start_ms)
);
CREATE INDEX IF NOT EXISTS idx_memory_access_aggregates_recent
    ON memory_access_aggregates(memory_id, last_accessed_at_ms DESC);

CREATE TABLE IF NOT EXISTS commitments (
    id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL,
    statement TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    due_at_ms INTEGER,
    scope_json TEXT NOT NULL DEFAULT '{}',
    source_episode_id TEXT,
    provenance_json TEXT NOT NULL DEFAULT '{}',
    confidence REAL NOT NULL DEFAULT 0.5,
    revision INTEGER NOT NULL DEFAULT 0,
    supersedes_id TEXT,
    retracted_at_ms INTEGER,
    completed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_commitments_subject_status
    ON commitments(subject_id, status, due_at_ms);
CREATE INDEX IF NOT EXISTS idx_commitments_due
    ON commitments(status, due_at_ms);
CREATE INDEX IF NOT EXISTS idx_commitments_source
    ON commitments(source_episode_id);

CREATE TABLE IF NOT EXISTS commitment_events (
    id TEXT PRIMARY KEY,
    commitment_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_at_ms INTEGER NOT NULL,
    source_episode_id TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_commitment_events_commitment
    ON commitment_events(commitment_id, event_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_commitment_events_type_time
    ON commitment_events(event_type, event_at_ms DESC);

CREATE TABLE IF NOT EXISTS persona_profiles (
    persona_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    portrait TEXT NOT NULL DEFAULT '',
    traits_json TEXT NOT NULL DEFAULT '[]',
    known_facts_json TEXT NOT NULL DEFAULT '[]',
    shared_experiences_json TEXT NOT NULL DEFAULT '[]',
    revision INTEGER NOT NULL DEFAULT 0,
    provenance_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (persona_id, subject_id)
);
CREATE INDEX IF NOT EXISTS idx_persona_profiles_subject
    ON persona_profiles(subject_id, updated_at_ms DESC);

CREATE TABLE IF NOT EXISTS persona_profile_history (
    id TEXT PRIMARY KEY,
    persona_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    portrait TEXT NOT NULL DEFAULT '',
    traits_json TEXT NOT NULL DEFAULT '[]',
    known_facts_json TEXT NOT NULL DEFAULT '[]',
    shared_experiences_json TEXT NOT NULL DEFAULT '[]',
    provenance_json TEXT NOT NULL DEFAULT '{}',
    changed_at_ms INTEGER NOT NULL,
    change_reason TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_persona_profile_history_revision
    ON persona_profile_history(persona_id, subject_id, revision);
CREATE INDEX IF NOT EXISTS idx_persona_profile_history_subject
    ON persona_profile_history(subject_id, changed_at_ms DESC);

CREATE TABLE IF NOT EXISTS memory_lineage (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    parent_memory_id TEXT,
    relation TEXT NOT NULL,
    source_kind TEXT,
    source_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memory_lineage_memory
    ON memory_lineage(memory_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_memory_lineage_parent
    ON memory_lineage(parent_memory_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_memory_lineage_source
    ON memory_lineage(source_kind, source_id);

CREATE TABLE IF NOT EXISTS memory_consolidation_records (
    id TEXT PRIMARY KEY,
    subject_id TEXT,
    session_id TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    started_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER,
    input_count INTEGER NOT NULL DEFAULT 0,
    output_count INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    provenance_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memory_consolidation_subject
    ON memory_consolidation_records(subject_id, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_memory_consolidation_session
    ON memory_consolidation_records(session_id, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_memory_consolidation_status
    ON memory_consolidation_records(status, started_at_ms DESC);

CREATE TABLE IF NOT EXISTS proactive_recall_candidates (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    scope_json TEXT NOT NULL DEFAULT '{}',
    cue TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0.0,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    expires_at_ms INTEGER,
    consumed_at_ms INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_proactive_recall_pending
    ON proactive_recall_candidates(subject_id, status, expires_at_ms);
CREATE INDEX IF NOT EXISTS idx_proactive_recall_memory
    ON proactive_recall_candidates(memory_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_proactive_recall_score
    ON proactive_recall_candidates(subject_id, score DESC, created_at_ms DESC);

-- Historical/audit records remain durable; callers use retraction/status fields.
CREATE TRIGGER IF NOT EXISTS temporal_graph_facts_no_delete
BEFORE DELETE ON temporal_graph_facts BEGIN
    SELECT RAISE(ABORT, 'temporal_graph_facts: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS memory_access_events_no_delete
BEFORE DELETE ON memory_access_events BEGIN
    SELECT RAISE(ABORT, 'memory_access_events: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS commitment_events_no_delete
BEFORE DELETE ON commitment_events BEGIN
    SELECT RAISE(ABORT, 'commitment_events: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS persona_profile_history_no_delete
BEFORE DELETE ON persona_profile_history BEGIN
    SELECT RAISE(ABORT, 'persona_profile_history: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS memory_lineage_no_delete
BEFORE DELETE ON memory_lineage BEGIN
    SELECT RAISE(ABORT, 'memory_lineage: hard DELETE forbidden');
END;
CREATE TRIGGER IF NOT EXISTS memory_consolidation_records_no_delete
BEFORE DELETE ON memory_consolidation_records BEGIN
    SELECT RAISE(ABORT, 'memory_consolidation_records: hard DELETE forbidden');
END;
"#;

pub(super) const HALLWAYS_SQL: &str = r#"
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

pub(super) const INIT_SQL: &str = r#"
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
-- Note 可被遗忘/合并 (apeireth_core::kernel::memory::Note 文档明确), 不强制 append-only.

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

// Only an empty schema may use these final table definitions. They include all
// column/index effects of V3..V5 without repeatedly reparsing the schema through
// ADD COLUMN. Historical SQL remains authoritative for every existing database.
const FRESH_TABLES_SQL: &str = include_str!("migrations/fresh_tables.sql");

// ============================================================================
// SQLITE_BUSY 有界退避重试 (只包住「能不能开始事务」这一层)
// ============================================================================
//
// 形态照抄仓内既有先例 (`crates/engine/storage/src/pool.rs` 的
// `BusyRetryManager` / `connect_with_busy_retry` / `is_busy`, `:279`-`:354`):
// 指数退避 + 有界预算 + 只认 SQLITE_BUSY / SQLITE_LOCKED。
//
// 为什么**必须**在生产侧 (而不是让调用方重试整个 `run_migrations`):
// - 根因是「并发首开时输方拿不到写锁」: 赢方在**同一个** IMMEDIATE 事务里装订
//   整套 schema (`:645`-`:675`) ⇒ 写锁一直持有到 `tx.commit()` (`:677`);
//   输方全部预算 = 连接既有的 `busy_timeout` (5s, `src/lib.rs:601`)
//   ⇒ 赢方耗时 > 预算时输方拿到 SQLITE_BUSY ⇒ 生产路径**直接失败**。
// - 重试放在 `BEGIN` 处 ⇒ **事务体一次都不重复执行** (无重复写、无重复插版本簿记)
//   ⇒ 且拿到锁后仍在**锁内重检空 schema** (`:639`-`:646`) ⇒ 「首开必须重检」的
//   语义一条没松。
//
// 为什么在 BEGIN 处重试是**安全**的:
// - `BEGIN IMMEDIATE` 失败 ⇒ SQLite **没有**开启事务 ⇒ 连接回到 autocommit 态
//   ⇒ 重试只是「再要一次写锁」, 事务体 (装订 schema / 记版本) 一次都没跑过。
// - 事务体一旦开始 (拿到锁) ⇒ 后面无论成功还是回滚, 都只执行一次。
//
// 预算取值 (生产侧比测试侧保守 —— 生产是**启动路径**, 不能为一把锁等 20s):
// - 第一次尝试**完全沿用**连接既有的 `busy_timeout` (5s) ⇒ 常见路径 (对方 < 5s
//   就提交) 的行为**一字未改**。
// - 第一次失败后**额外**再给 ≤ `BUSY_RETRY_EXTRA_BUDGET` (3s) 的退避重试预算
//   ⇒ 总等待 ≲ 5s + 3s = 8s。
// - 额外轮次把 busy 窗口**收窄**到 `BUSY_RETRY_WINDOW` (250ms) ⇒ 每轮等待有界
//   ⇒ 总等待有界 (不会「预算还剩 10ms 却再等 5s」)。
const BUSY_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(5);
/// SQLITE_BUSY 退避重试: 单次等待上限 (指数退避封顶).
const BUSY_RETRY_MAX_DELAY: Duration = Duration::from_millis(250);
/// 退避重试的**额外**总预算 (叠加在连接既有的 `busy_timeout` 之上).
const BUSY_RETRY_EXTRA_BUDGET: Duration = Duration::from_secs(3);
/// 退避重试**轮次**的 busy 窗口 (收窄 ⇒ 每轮等待有界 ⇒ 总等待有界).
const BUSY_RETRY_WINDOW: Duration = Duration::from_millis(250);

/// 判定一个 sqlite 错误是不是 SQLITE_BUSY / SQLITE_LOCKED。
///
/// 结构化判定 (读 `rusqlite::ErrorCode`), **不**做文案匹配 ⇒ 端口层
/// `TaskQueueError::Store { reason }` 那种「按文案猜」的弱判定不再需要。
fn is_busy_error(error: &rusqlite::Error) -> bool {
    match error {
        rusqlite::Error::SqliteFailure(inner, _) => matches!(
            inner.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        ),
        _ => false,
    }
}

/// 读连接当前的 `busy_timeout` (毫秒)。
fn read_busy_timeout_ms(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))
}

/// 把 `PRAGMA busy_timeout` 的毫秒值还原成 [`Duration`] (负值按 0 处理).
fn busy_timeout_duration(millis: i64) -> Duration {
    Duration::from_millis(u64::try_from(millis).unwrap_or(0))
}

/// 预算耗尽仍是 busy ⇒ **保留原错误码** (结构化判定不变: 仍是
/// `DatabaseBusy` / `DatabaseLocked`), 只在**消息尾部**追加「等过、仍被锁」的说明
/// (原消息前缀保留) ⇒ 既不吞错、也不把 busy 改写成别的错。
fn busy_error_after_wait(
    error: rusqlite::Error,
    attempts: u32,
    elapsed: Duration,
) -> rusqlite::Error {
    match error {
        rusqlite::Error::SqliteFailure(inner, message) => {
            let base = message.unwrap_or_else(|| inner.to_string());
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: inner.code,
                    extended_code: inner.extended_code,
                },
                Some(format!(
                    "{base}; write lock still held after waiting: {attempts} extra attempt(s), \
                     {} ms total (extra budget {} ms)",
                    elapsed.as_millis(),
                    BUSY_RETRY_EXTRA_BUDGET.as_millis(),
                )),
            )
        }
        other => other,
    }
}

/// 起一个 `BEGIN IMMEDIATE` 事务: 撞上 SQLITE_BUSY / SQLITE_LOCKED 时
/// **有界**退避重试。
///
/// - ① 只对 busy 类错误重试; 其它错误**原样上抛**。
/// - ② 预算有界: 第一次尝试沿用连接既有的 `busy_timeout`, 失败后**额外**再给
///   `BUSY_RETRY_EXTRA_BUDGET` (3s) 的指数退避重试 ⇒ 总等待 ≲ 8s。
/// - ③ 重试发生在 `BEGIN` 处 ⇒ **事务体一次都不重复执行**。
/// - ④ 预算耗尽仍是 busy ⇒ 返回原错误 (错误码不变) + 「等过、仍被锁」的说明。
///
/// ⚠️ **为什么用 [`Transaction::new_unchecked`] 而不是
/// [`Connection::transaction_with_behavior`]**: 后者要求 `&mut Connection`,
/// 而「拿到写锁」这个借用必须**活到函数返回** (返回的 `Transaction` 挂在它上面)
/// ⇒ 函数体内无法再碰连接 (收窄 / 恢复 `busy_timeout` 都要碰) ⇒ 借用检查直接拒绝。
/// `Transaction::new` 在 rusqlite 里**就是** `Transaction::new_unchecked`
/// (只有 `&mut` / `&` 的**编译期**嵌套防护不同, 运行时发出的 SQL 完全一样:
/// `BEGIN IMMEDIATE`) ⇒ 换过去**运行时语义一字未改**。
///
/// **嵌套防护仍在**: 调用方传进来的是 `&mut Connection` (这里按共享引用重借用)
/// ⇒ 事务存活期间调用方拿不回那个 `&mut` ⇒ 同一个连接上开嵌套事务仍然编译期被禁。
fn begin_immediate_with_busy_retry<'conn>(
    conn: &'conn Connection,
) -> Result<Transaction<'conn>, rusqlite::Error> {
    // 连接既有的 busy_timeout (毫秒): 收窄 / 恢复都要用 ⇒ 读不出来就**不收窄**
    // (退化成「每次尝试都吃满既有 busy_timeout」, 仍然有界, 只是上界更松)。
    let original_busy_timeout: Option<i64> = read_busy_timeout_ms(conn).ok();
    let started = Instant::now();
    let mut delay = BUSY_RETRY_INITIAL_DELAY;
    let mut extra_attempts: u32 = 0;
    // 额外预算的截止时刻: **第一次失败之后**才起算 (第一次尝试花掉的是连接既有
    // 的 busy_timeout, 那部分不计入「额外预算」)。
    let mut deadline: Option<Instant> = None;
    let mut narrowed = false;

    loop {
        match Transaction::new_unchecked(conn, TransactionBehavior::Immediate) {
            Ok(tx) => {
                if narrowed {
                    if let Some(original) = original_busy_timeout {
                        // 恢复连接既有的 busy_timeout (净效果: 连接配置不变)。
                        tx.busy_timeout(busy_timeout_duration(original))?;
                    }
                }
                return Ok(tx);
            }
            Err(error) => {
                // ① 其它错误一律原样上抛 (不重试、不改写)。
                if !is_busy_error(&error) {
                    if narrowed {
                        if let Some(original) = original_busy_timeout {
                            conn.busy_timeout(busy_timeout_duration(original))?;
                        }
                    }
                    return Err(error);
                }
                // ② 第一次失败 ⇒ 起算额外预算, 并收窄 busy 窗口 (保证总等待有界)。
                let deadline =
                    *deadline.get_or_insert_with(|| Instant::now() + BUSY_RETRY_EXTRA_BUDGET);
                if !narrowed {
                    narrowed = true;
                    // 读不出原值就不收窄 (收窄了却恢复不了 = 连接配置被改坏)。
                    if original_busy_timeout.is_some() {
                        conn.busy_timeout(BUSY_RETRY_WINDOW)?;
                    }
                }
                let now = Instant::now();
                if now >= deadline {
                    if let Some(original) = original_busy_timeout {
                        conn.busy_timeout(busy_timeout_duration(original))?;
                    }
                    // ④ 预算耗尽: 保留原错误码, 追加「等过、仍被锁」的说明。
                    return Err(busy_error_after_wait(
                        error,
                        extra_attempts,
                        started.elapsed(),
                    ));
                }
                std::thread::sleep(delay.min(deadline - now));
                delay = delay.saturating_mul(2).min(BUSY_RETRY_MAX_DELAY);
                extra_attempts += 1;
            }
        }
    }
}

/// 应用全部未执行的 migrations.
///
/// 事务保护: 基表 + 触发器 + 未执行 migration + 版本簿记在**一个** IMMEDIATE
/// 事务内执行, 失败整体回滚 (migration SQL 本身保持逐条幂等, 见 `MIGRATIONS`
/// 注释; 启动期只提交一次, 少一次 WAL 往返)。
///
/// 迁移只在需要时跑 (B-P1 版本检查短路): 版本簿记已是完整最新集的库**不再跑
/// 迁移** —— 不空开事务、不重放幂等 DDL (含建表与触发器); 簿记缺口/落后/无表
/// 时照常走全量路径, 修复语义在需要时仍然生效。列集校验 (M27) 短路前后都跑。
pub fn run_migrations(conn: &mut Connection) -> MemoryResult<()> {
    // 0. M27 (L 组): 迁移前列集校验 — 如果同名表已被**别套 schema 家族**
    // (如 `apeireth-storage` 的 `episodes(id, data)` 最小 layout) 创建过,
    // 必须先报清楚的错. 否则 `CREATE TABLE IF NOT EXISTS` 静默 no-op,
    // 之后 INSERT 才以 "no such column" 这种难懂的错爆炸 (样板:
    // `commitments.rs` 的 `validate_schema_columns`).
    validate_owned_schema(conn)?;

    // 0.5 B-P1: 已迁移库短路 —— 簿记满最新集 = 迁移不再跑。
    if migrations_up_to_date(conn)? {
        return Ok(());
    }

    // 1..3. 建表 (CREATE IF NOT EXISTS) + 6 流 append-only 触发器注册 (幂等)
    //     + 4. 记录已应用的 migration.
    // 规则: 禁止任何原地修改; 唯一例外是 "软删除" (tombstoned_at 从 NULL 变为非 NULL).
    // 软删除也是不可逆的 (OLD.tombstoned_at 必须是 NULL).
    //
    // M21: 用 BEGIN IMMEDIATE 从一开始就拿写锁, 版本重读放在**事务内**
    // (读-判-写同一把写锁), 与 `storage/migrations.rs:109` 同模式 —— 两个
    // 进程同时启动时按写锁串行而非 BUSY_SNAPSHOT. SQL 全部幂等
    // (IF NOT EXISTS), 并发下最坏是 busy_timeout 内等待.
    //
    // B-P2 (SQLITE_BUSY 下沉到生产侧): `BEGIN IMMEDIATE` 撞上对方未提交的写
    // 事务时走**有界退避重试** (见 `begin_immediate_with_busy_retry`) —— 重试
    // 只发生在「能不能开始事务」这一层 ⇒ 事务体一次都不重复执行; 拿到锁后仍在
    // 锁内重检空 schema (下面 `fresh` 判定) ⇒ 「首开必须重检」的语义不变。
    let tx: Transaction<'_> = begin_immediate_with_busy_retry(conn)?;

    // Check under the writer lock: another opener may have initialized the file
    // since the optimistic version check. Any existing object keeps the original
    // upgrade path, including partially initialized or unrelated databases.
    let fresh: bool = tx.query_row(
        "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master)",
        [],
        |row| row.get(0),
    )?;
    if fresh {
        tx.execute_batch(FRESH_TABLES_SQL)?;
    }
    tx.execute_batch(INIT_SQL)?;

    install_history_guards(&tx)?;

    let mut record = tx
        .prepare("INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)")?;
    for m in MIGRATIONS {
        if fresh || !migration_applied(&tx, m.version)? {
            // On a fresh schema V3..V5 have already been applied by the complete
            // table definitions above, including their indices. Record each
            // version in the same transaction as all other migrations.
            // V1 is INIT_SQL, which has also already run above on both paths.
            if m.version != 1 && !(fresh && matches!(m.version, 3..=5)) {
                // 静态 SQL (可为空: DDL 全挂在 `idempotent` 钩子上的迁移).
                if !m.sql.trim().is_empty() {
                    tx.execute_batch(m.sql)?;
                }
                // 幂等 DDL 钩子 (需活连接的迁移, 如 V19 的「先探测列集再 ALTER」).
                // 与静态 SQL 同一事务 ⇒ 失败整体回滚, 如实报错不静默跳过.
                // ⚠️ **fresh 路径也必须跑**: `fresh_tables.sql` 只覆盖 V3..V5 的
                // 列效应, V19 的 `chain_hash` / `chain_prev_hash` 不在其中
                // ⇒ 跳过钩子 = 全新库悄悄失去链保护.
                if let Some(idempotent) = m.idempotent {
                    idempotent(&tx)?;
                }
            }
            record.execute(params![m.version, m.name, now_unix()])?;
        }
    }
    drop(record);
    tx.commit()?;
    Ok(())
}

fn install_history_guards(conn: &Connection) -> MemoryResult<()> {
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

    Ok(())
}

/// memory schema 家族拥有的核心表 → 必需列 (M27 / L 组).
///
/// 与 `apeireth-storage` 的 `episodes(id, data)` / `sessions(id, data)` 等
/// 最小 layout 同名不同义: 两套迁移先后处理同一 .db 文件时,
/// `CREATE TABLE IF NOT EXISTS` 会静默 no-op, 之后 INSERT 因缺列报难懂的错.
const MEMORY_OWNED_TABLES: &[(&str, &[&str])] = &[
    (
        "episodes",
        &[
            "id",
            "continuity_id",
            "session_id",
            "timestamp",
            "role",
            "content",
        ],
    ),
    (
        "agent_traces",
        &["span_id", "trace_id", "kind", "actor", "session_id"],
    ),
    ("sessions", &["id", "started_at", "last_active_at"]),
    ("notes", &["id", "timestamp", "content"]),
];

/// 迁移前列集校验 (M27 / L 组): 已存在的同名表必须已经是 memory layout.
///
/// 借用 `commitments.rs` 的 `validate_schema_columns` 样板: 直接对
/// `sqlite_master` 查列集, 缺列即拒绝启动并给出可操作的信息 (而不是让
/// 后续 INSERT 崩一个 "no such column").
fn validate_owned_schema(conn: &Connection) -> MemoryResult<()> {
    for (table, required) in MEMORY_OWNED_TABLES {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                params![table],
                |row| row.get(0),
            )
            .map_err(MemoryError::from)?;
        if !exists {
            continue;
        }
        // 表名来自本文件静态白名单, 无注入面.
        let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let actual = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let missing: Vec<&str> = required
            .iter()
            .copied()
            .filter(|column| !actual.iter().any(|actual| actual == column))
            .collect();
        if !missing.is_empty() {
            return Err(MemoryError::Invalid(format!(
                "table `{table}` exists with a foreign schema layout (missing memory columns: {}). \
                 The database file is owned by another schema family (e.g. apeireth-storage's \
                 minimal (id, data) layout); memory and storage must not share one .db file — \
                 use separate database paths",
                missing.join(", ")
            )));
        }
    }
    Ok(())
}

/// Applies the memory schema through the pool's serialized writer.
///
/// This is the bootstrap entry point for applications that share one
/// [`SqliteConnectionPool`] across memory backends. It deliberately does not
/// open a connection or run migrations on a reader, so stores can only be
/// constructed after the complete V1-V14 schema is installed.
pub async fn run_migrations_on_pool(pool: &SqliteConnectionPool) -> Result<(), StorageError> {
    pool.write(|conn| {
        run_migrations(conn).map_err(|error| StorageError::Migration {
            version: 0,
            name: "cognitive_memory",
            message: error.to_string(),
        })
    })
    .await
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

/// 已迁移判定 (B-P1 版本检查短路): `schema_migrations` 簿记存在、条数 = 迁移
/// 集条数、且最大版本 = 迁移集最新版本 —— 三条齐备才认定「已迁移, 不再跑」。
/// 簿记表不存在 (全新库)、条数不符 (旧库/半迁移)、最大版本不符 (异常态) 都
/// 判「需要跑」, 走全量路径。
fn migrations_up_to_date(conn: &Connection) -> MemoryResult<bool> {
    let Some(latest) = MIGRATIONS.last().map(|m| m.version) else {
        return Ok(true);
    };
    let table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
            [],
            |row| row.get(0),
        )
        .map_err(MemoryError::from)?;
    if !table_exists {
        return Ok(false);
    }
    let (count, max_version): (i64, Option<i64>) = conn
        .query_row(
            "SELECT COUNT(*), MAX(version) FROM schema_migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(MemoryError::from)?;
    Ok(count == MIGRATIONS.len() as i64 && max_version == Some(latest))
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
