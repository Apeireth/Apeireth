//! Schema migrations for `apeireth-memory`.
//!
//! 按主人偏好: 直接 SQL, 不引入 ORM. 每次新 schema 改动都追加一条新 entry
//! 到 [`MIGRATIONS`], 不可修改历史 (migrations are append-only too).
//!
//! 命名规范: `V<version>__<short_description>`.

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
    let tx: Transaction<'_> = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute_batch(INIT_SQL)?;

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
        tx.execute_batch(&update_trigger)?;
        tx.execute_batch(&delete_trigger)?;
    }

    for m in MIGRATIONS {
        if !migration_applied(&tx, m.version)? {
            // 静态 SQL (可为空: DDL 全挂在 `idempotent` 钩子上的迁移).
            if !m.sql.trim().is_empty() {
                tx.execute_batch(m.sql)?;
            }
            // 幂等 DDL 钩子 (需活连接的迁移, 如 V19 的「先探测列集再 ALTER」).
            // 与静态 SQL 同一事务 ⇒ 失败整体回滚, 如实报错不静默跳过.
            if let Some(idempotent) = m.idempotent {
                idempotent(&tx)?;
            }
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![m.version, m.name, now_unix()],
            )?;
        }
    }
    tx.commit()?;
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
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    #[tokio::test]
    async fn pooled_migrations_are_idempotent_across_restart() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let path = path.path().to_owned();

        let pool = apeireth_storage::SqliteConnectionPool::open(&path)
            .await
            .unwrap();
        run_migrations_on_pool(&pool).await.unwrap();
        run_migrations_on_pool(&pool).await.unwrap();
        let first = pool
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                )
            })
            .unwrap();
        assert_eq!(first, MIGRATIONS.len() as i64);
        drop(pool);

        let reopened = apeireth_storage::SqliteConnectionPool::open(&path)
            .await
            .unwrap();
        run_migrations_on_pool(&reopened).await.unwrap();
        let second = reopened
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                )
            })
            .unwrap();
        assert_eq!(second, first);
    }

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

    // ===== Core Capability Expansion Phase 7: migration integrity =====

    #[test]
    fn all_migrations_v1_to_v7_applied_on_fresh_db() {
        // 全新 in-memory DB: V1-V7 全部应用, schema_migrations 记录完整.
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let applied = store.applied_migrations().unwrap();
        for v in 1..=9 {
            assert!(
                applied.contains(&v),
                "migration V{v} should be applied on fresh db"
            );
        }
        // V5/V6/V7 新表/列存在.
        let conn = store.conn().unwrap();
        // sessions 生命周期列 (V5).
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(sessions)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for c in [
            "title",
            "scope",
            "project_id",
            "state",
            "metadata_json",
            "revision",
            "archived_at",
            "updated_at",
        ] {
            assert!(
                cols.iter().any(|x| x == c),
                "sessions should have column {c}"
            );
        }
        // episode_governance 表 (V6).
        let gov_exists: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='episode_governance')", [], |r| r.get(0))
            .unwrap();
        assert!(gov_exists, "episode_governance table should exist");
        // agent_traces 表 (V7).
        let trace_exists: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_traces')", [], |r| r.get(0))
            .unwrap();
        assert!(trace_exists, "agent_traces table should exist");
    }

    #[test]
    fn migrations_reopen_preserves_data_and_idempotent() {
        // 写入跨 V5/V6/V7 的数据, 重跑 migration, 数据不丢 + 幂等.
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        use crate::agent_trace::{TraceSpan, TraceSpanKind, TraceSpanStatus};
        use crate::episode::EpisodeStore;
        use crate::{MemoryGovernanceStore, SessionLifecycleStore, TraceStore};
        use apeireth_core::kernel::memory::Episode;
        // session lifecycle (V5).
        store
            .create_session(
                "s1",
                Some("持久"),
                crate::session_lifecycle::SessionScope::Global,
                None,
                None,
            )
            .unwrap();
        // episode + governance (V6).
        store
            .put_episode(&Episode {
                id: "ep-1".into(),
                timestamp: 1,
                role: "user".into(),
                content: "x".into(),
                session_id: "s1".into(),
            })
            .unwrap();
        store.protect_episode("ep-1", 0).unwrap();
        // trace (V7).
        let span = TraceSpan {
            span_id: "sp1".into(),
            trace_id: "t1".into(),
            parent_span_id: None,
            kind: TraceSpanKind::Conversation,
            actor: "user".into(),
            status: TraceSpanStatus::Succeeded,
            summary: Some("done".into()),
            attributes: serde_json::json!({}),
            started_at: 1,
            ended_at: Some(2),
            session_id: Some("s1".into()),
        };
        store.put_trace_span(&span).unwrap();
        // 重跑 migration (幂等).
        {
            let guard = store.conn().unwrap();
            let mut g = guard;
            run_migrations(&mut g).unwrap();
        }
        // 数据仍在.
        let s = store.get_session_lifecycle("s1").unwrap().unwrap();
        assert_eq!(s.title.as_deref(), Some("持久"));
        let g = store.get_governed("ep-1").unwrap().unwrap();
        assert!(g.protected);
        let t = store.get_trace_span("sp1").unwrap().unwrap();
        assert_eq!(t.trace_id, "t1");
        // migration 记录无重复.
        // 断言演进 (2026-10, V4-a 人格记忆批): 11 → 12 —— 迁移列表是 append-only
        // 增长面, 每加一条合法迁移本计数 +1 (钉「无重复」语义, 数字随迁移清单走).
        // 断言演进 (2026-10-17, V4-b 记忆保鲜回路批): 12 → 13 —— 同上语义不变,
        // 新增 V17__memory_freshness_loop (保鲜三件套三表)。
        // 断言演进 (2026-10-26, W-10 记忆链正式迁移批): 13 → 14 —— 同上语义不变,
        // 新增 V19__memory_chain_schema (链头表 + episodes 两列)。
        let applied = store.applied_migrations().unwrap();
        assert_eq!(
            applied.iter().filter(|v| **v >= 5).count(),
            15,
            "V5..V19 各一条 (V15 = dedup_fingerprints; V16 = persona_memory_*; \
             V17 = 记忆保鲜 source_hash/ingest_queue/memory_fold_segments; V18 = lessons; \
             V19 = 记忆层哈希链 schema)"
        );
    }

    #[test]
    fn legacy_episode_works_after_v6_governance_added() {
        // 旧 episode (无 governance 行) 在 V6 后仍可正常检索 + 默认 active.
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        use crate::episode::EpisodeStore;
        use crate::MemoryGovernanceStore;
        store
            .put_episode(&apeireth_core::kernel::memory::Episode {
                id: "legacy-ep".into(),
                timestamp: 1,
                role: "user".into(),
                content: "old".into(),
                session_id: "me".into(),
            })
            .unwrap();
        let recent = store.governed_recent_episodes("me", 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent[0].status,
            crate::memory_governance::MemoryGovernanceStatus::Active
        );
        assert!(!recent[0].protected);
    }

    /// M27 (L 组): 别套 schema 家族 (storage 最小 `(id, data)` layout) 占位的
    /// episodes 表必须在迁移前被拒绝 — 否则 `CREATE TABLE IF NOT EXISTS` 静默
    /// no-op, 之后 INSERT 才报 "no such column" 难懂的错.
    #[test]
    fn foreign_episodes_layout_is_rejected_before_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE episodes (id TEXT PRIMARY KEY, data TEXT)")
            .unwrap();
        let err = run_migrations(&mut conn).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("episodes"), "错误必须指名问题表: {msg}");
        assert!(
            msg.contains("foreign schema layout"),
            "错误必须说明是外来 layout: {msg}"
        );
    }

    /// M27: memory 自己的 layout 不得被误伤 (fresh DB 全跑完 + 重跑都通过).
    #[test]
    fn memory_owned_layout_passes_validation() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        {
            let conn = store.conn().unwrap();
            let mut guard = conn;
            run_migrations(&mut guard).unwrap();
        }
    }

    /// M21: 版本重读在 BEGIN IMMEDIATE 事务内 — 已应用的 migration 不再
    /// 重复执行/重复插行 (幂等), 两进程并发按写锁串行而非 BUSY_SNAPSHOT.
    #[test]
    fn migration_version_is_rechecked_inside_immediate_transaction() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let before = store.applied_migrations().unwrap();
        {
            let conn = store.conn().unwrap();
            let mut guard = conn;
            run_migrations(&mut guard).unwrap();
        }
        let after = store.applied_migrations().unwrap();
        assert_eq!(before, after, "Immediate 事务内重读版本, 不得重复插入");
    }

    /// V15 (M27 / L 组): dedup_fingerprints 必须由迁移列表创建 —
    /// 不再只靠 dedup.rs 的懒建表 (schema 快照与迁移列表一致).
    #[test]
    fn v15_creates_dedup_fingerprints() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        assert!(store.applied_migrations().unwrap().contains(&15));
        let conn = store.conn().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'dedup_fingerprints')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "V15 必须创建 dedup_fingerprints");
    }

    /// V19 (W-10): 链的 schema 必须由**迁移列表**提供 (不再是运行时懒建) ——
    /// 迁移列表与簿记不得漂移, 版本号不得跳号。
    #[test]
    fn v19_registers_chain_schema_and_versions_are_contiguous() {
        let versions: Vec<i64> = MIGRATIONS.iter().map(|m| m.version).collect();
        assert_eq!(
            versions,
            (1..=MIGRATIONS.len() as i64).collect::<Vec<_>>(),
            "迁移版本号必须连续无跳号 (跳号 ⇒ 簿记与迁移列表漂移)"
        );
        let v19 = MIGRATIONS
            .iter()
            .find(|m| m.version == 19)
            .expect("MIGRATIONS 必须含 V19");
        assert_eq!(v19.name, "V19__memory_chain_schema");
        assert!(
            v19.idempotent.is_some(),
            "V19 的 DDL 必须走幂等钩子 (SQLite 的 ADD COLUMN 无 IF NOT EXISTS)"
        );

        let store = SqliteMemoryStore::open_in_memory().unwrap();
        assert!(store.applied_migrations().unwrap().contains(&19));
        let conn = store.conn().unwrap();
        let heads: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'memory_chain_heads')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(heads, "V19 必须创建 memory_chain_heads");
        let mut stmt = conn.prepare("PRAGMA table_info(episodes)").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        assert!(
            columns.iter().any(|c| c == "chain_hash"),
            "V19 必须给 episodes 加 chain_hash, 实际列集: {columns:?}"
        );
        assert!(
            columns.iter().any(|c| c == "chain_prev_hash"),
            "V19 必须给 episodes 加 chain_prev_hash, 实际列集: {columns:?}"
        );
    }
}
