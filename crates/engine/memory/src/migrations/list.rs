//! 迁移**列表**（从 `migrations.rs` 拆出）。
//!
//! # 为什么拆
//!
//! 尺寸棘轮**只减不增**（`scripts/arch-baseline.txt`）⇒ 而迁移列表**每加一条迁移
//! 就要长几行** ⇒ 与「结构/运行器」代码混在一个文件里 ⇒ 那个文件**只涨不跌**
//! ⇒ 迟早撞棘轮 ⇒ 拆开 ⇒ 两个文件各自都能保持可读与**可缩**。
//!
//! 本文件只放**清单**（哪一版迁移做什么）；DDL 常量与运行器仍在 `migrations.rs`。

use super::{
    v19_memory_chain_schema, Migration, HALLWAYS_SQL, INIT_SQL, V13_SQL, V14_SQL, V16_SQL, V17_SQL,
};

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "V1__init_six_history_streams",
        sql: INIT_SQL,
        idempotent: None,
    },
    // R179 P1-10: Hallway 表 (wing 内 entity-pair co-occurrence)
    // 跟 6 历史流不一样: 不加 append-only trigger, 允许 recompute 时 UPSERT
    // (公开设计 "preserve L7 dynamics" — strength/stability 必须跨 recompute 保留).
    Migration {
        version: 2,
        name: "V2__hallways_for_wings",
        sql: HALLWAYS_SQL,
        idempotent: None,
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
        idempotent: None,
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
        idempotent: None,
    },
    // Core Capability Expansion Phase 2: sessions 表生命周期扩展列.
    // 向后兼容铁律: 全部新增列 NULLable / 有默认值, 存量行 ALTER 后自动取默认语义
    // (title NULL→"未命名", state NULL→active, revision NULL→0, scope NULL→global).
    // 不改既有 sessions 列定义, 不动既有 SessionNoteStore trait 行为 (旧 upsert 仍只写 4 列).
    // SQLite ALTER TABLE 一次一列 → 多条语句.
    Migration {
        version: 5,
        name: "V5__sessions_lifecycle",
        sql: "ALTER TABLE sessions ADD COLUMN title TEXT;\n\
              ALTER TABLE sessions ADD COLUMN scope TEXT;\n\
              ALTER TABLE sessions ADD COLUMN project_id TEXT;\n\
              ALTER TABLE sessions ADD COLUMN state TEXT;\n\
              ALTER TABLE sessions ADD COLUMN metadata_json TEXT;\n\
              ALTER TABLE sessions ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;\n\
              ALTER TABLE sessions ADD COLUMN archived_at INTEGER;\n\
              ALTER TABLE sessions ADD COLUMN updated_at INTEGER;\n\
              CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);\n\
              CREATE INDEX IF NOT EXISTS idx_sessions_scope ON sessions(scope);",
        idempotent: None,
    },
    // Core Capability Expansion Phase 3: 记忆治理 — episode governance 表.
    // episodes 表是 append-only (trigger 拒绝 UPDATE/DELETE), 不能直接改其内容/状态.
    // 治理层用独立 sidecar 表记录可变元数据: forgotten (软删, 从检索排除) /
    // protected (防自动遗忘) / content_override (修订内容) / revision (乐观并发).
    // 原始 episode 行不动 → provenance 完整 + 审计可追溯. 存量行无 governance 记录 =
    // 默认 active/unprotected (LEFT JOIN NULL 语义), 零数据迁移.
    Migration {
        version: 6,
        name: "V6__episode_governance",
        sql: "CREATE TABLE IF NOT EXISTS episode_governance (\n\
              episode_id    TEXT PRIMARY KEY,\n\
              status        TEXT NOT NULL DEFAULT 'active',\n\
              protected     INTEGER NOT NULL DEFAULT 0,\n\
              content_override TEXT,\n\
              revision      INTEGER NOT NULL DEFAULT 0,\n\
              updated_at    INTEGER,\n\
              updated_by    TEXT,\n\
              reason        TEXT,\n\
              forgotten_at  INTEGER\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_episode_governance_status ON episode_governance(status);\n\
              CREATE INDEX IF NOT EXISTS idx_episode_governance_protected ON episode_governance(protected);",
        idempotent: None,
    },
    // Core Capability Expansion Phase 5: Agent 执行轨迹 (structured trace).
    // 一次用户请求 → trace_id; Commander/Worker/Tool/Memory 各为 span (parent_span_id 关联).
    // append-only (每 span 一行, 终态时写 ended_at/status). 属性 attributes_json 已 redaction.
    // **严禁**存储模型内部原始 Chain-of-Thought; 只存 safe user-facing summary.
    // trace_id/span_id 用 16-hex (与 telemetry W3C span 同形态, 便于未来打通).
    Migration {
        version: 7,
        name: "V7__agent_traces",
        sql: "CREATE TABLE IF NOT EXISTS agent_traces (\n\
              span_id        TEXT PRIMARY KEY,\n\
              trace_id       TEXT NOT NULL,\n\
              parent_span_id TEXT,\n\
              kind           TEXT NOT NULL,\n\
              actor          TEXT NOT NULL,\n\
              status         TEXT NOT NULL DEFAULT 'running',\n\
              summary        TEXT,\n\
              attributes_json TEXT NOT NULL DEFAULT '{}',\n\
              started_at     INTEGER NOT NULL,\n\
              ended_at       INTEGER,\n\
              session_id     TEXT\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_agent_traces_trace ON agent_traces(trace_id, started_at);\n\
              CREATE INDEX IF NOT EXISTS idx_agent_traces_session ON agent_traces(session_id, started_at);\n\
              CREATE INDEX IF NOT EXISTS idx_agent_traces_started ON agent_traces(started_at DESC);",
        idempotent: None,
    },
    // B2 · Phase 1 (research, 默认关闭): 派生记忆血缘 + append-only 血缘审计事件.
    // - research_derived_from: RA-1 A.4.1 schema (主键 = 全列去重, 幂等登记).
    // - research_lineage_events: A8 不可变审计事件 (只 INSERT, 无 UPDATE/DELETE 路径).
    // 纯新增表: 不影响任何既有表/查询语义; 生产路径零触碰.
    Migration {
        version: 8,
        name: "V8__research_derived_lineage",
        sql: "CREATE TABLE IF NOT EXISTS research_derived_from (\n\
              derived_kind TEXT NOT NULL,\n\
              derived_id   TEXT NOT NULL,\n\
              source_kind  TEXT NOT NULL,\n\
              source_id    TEXT NOT NULL,\n\
              span         TEXT,\n\
              ts           INTEGER NOT NULL,\n\
              PRIMARY KEY (derived_kind, derived_id, source_kind, source_id)\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_research_derived_src ON research_derived_from(source_kind, source_id);\n\
              CREATE INDEX IF NOT EXISTS idx_research_derived_der ON research_derived_from(derived_kind, derived_id);\n\
              CREATE TABLE IF NOT EXISTS research_lineage_events (\n\
              seq              INTEGER PRIMARY KEY AUTOINCREMENT,\n\
              op               TEXT NOT NULL,\n\
              actor            TEXT,\n\
              ts               INTEGER NOT NULL,\n\
              reason           TEXT,\n\
              subject          TEXT NOT NULL,\n\
              revision_before  INTEGER,\n\
              revision_after   INTEGER,\n\
              detail_json      TEXT NOT NULL DEFAULT '{}'\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_research_lineage_events_ts ON research_lineage_events(ts);",
        idempotent: None,
    },
    // B2 · RA-15 P0-A (research, 默认关闭): 执行态遗忘登记表.
    // 吸收自 arXiv:2609.04875 (execution-state unlearning):
    // 遗忘闭包除派生记忆面外, 还要审计执行态 (会话 token 跨度 / 挂起工具计划 /
    // 在途 summary / prompt 缓存片段). 纯登记表: 只 INSERT/UPSERT + 查询, 无删除路径,
    // 生产路径零触碰.
    Migration {
        version: 9,
        name: "V9__research_execution_state",
        sql: "CREATE TABLE IF NOT EXISTS research_execution_state (\n\
              kind           TEXT NOT NULL,\n\
              ref_id         TEXT NOT NULL,\n\
              injection_step INTEGER,\n\
              taint_kind     TEXT NOT NULL,\n\
              taint_id       TEXT NOT NULL,\n\
              note           TEXT NOT NULL DEFAULT '',\n\
              ts             INTEGER NOT NULL,\n\
              PRIMARY KEY (kind, ref_id)\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_research_exec_state_taint ON research_execution_state(taint_kind, taint_id);",
        idempotent: None,
    },
    // B2 · RA-15 P0-A 生产接线 (2026-09-08): 持久化遗忘集.
    // 治理语义的 forgotten set 从"调用方内存集"升级为 store 内持久事实
    // (per docs/01-architecture/forget-three-phase-production-spec.md);
    // GovernedRecall::from_store_persisted 读它; P0-B 修复再发布清标记.
    // 纯新增表, 生产路径零触碰.
    Migration {
        version: 10,
        name: "V10__research_forgotten_artifacts",
        sql: "CREATE TABLE IF NOT EXISTS research_forgotten_artifacts (\n\
              kind        TEXT NOT NULL,\n\
              id          TEXT NOT NULL,\n\
              reason      TEXT NOT NULL DEFAULT '',\n\
              approval_id TEXT NOT NULL DEFAULT '',\n\
              ts          INTEGER NOT NULL,\n\
              PRIMARY KEY (kind, id)\n\
              );\n\
              CREATE INDEX IF NOT EXISTS idx_research_forgotten_ts ON research_forgotten_artifacts(ts);",
        idempotent: None,
    },
    // Memory 2.1: additive scope/provenance metadata sidecar. Episodes stay
    // append-only; existing rows without metadata are interpreted as
    // session-scoped legacy records by the coordinator.
    // Renumbered from experimental V9 because current main already shipped
    // V9/V10 research tables after the shared V8 merge-base.
    Migration {
        version: 11,
        name: "V11__episode_memory_scope_metadata",
        sql: r#"
CREATE TABLE IF NOT EXISTS episode_memory_metadata (
    episode_id TEXT PRIMARY KEY,
    metadata_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_episode_memory_metadata_scope
    ON episode_memory_metadata(json_extract(metadata_json, '$.scope.scope'));
"#,
        idempotent: None,
    },
    // Memory 2.2: additive indices for scoped cross-session candidate queries.
    // Renumbered from experimental V10.
    Migration {
        version: 12,
        name: "V12__episode_metadata_scope_indices",
        sql: r#"
CREATE INDEX IF NOT EXISTS idx_episode_metadata_scope_project
    ON episode_memory_metadata(
        json_extract(metadata_json, '$.scope.scope'),
        json_extract(metadata_json, '$.scope.project_id')
    );
CREATE INDEX IF NOT EXISTS idx_episode_metadata_scope_session
    ON episode_memory_metadata(
        json_extract(metadata_json, '$.scope.scope'),
        json_extract(metadata_json, '$.scope.session_id')
    );
CREATE INDEX IF NOT EXISTS idx_episode_metadata_scope_user
    ON episode_memory_metadata(
        json_extract(metadata_json, '$.scope.scope'),
        json_extract(metadata_json, '$.scope.user_id')
    );
CREATE INDEX IF NOT EXISTS idx_episode_metadata_scope_persona
    ON episode_memory_metadata(
        json_extract(metadata_json, '$.scope.scope'),
        json_extract(metadata_json, '$.scope.persona_id'),
        json_extract(metadata_json, '$.scope.user_id')
    );
"#,
        idempotent: None,
    },
    // Memory 2.2: durable temporal facts, access telemetry, commitments,
    // persona revisions, lineage, consolidation records, and proactive recall.
    // All structures are additive and every statement is safe to replay.
    // Renumbered from experimental V11.
    Migration {
        version: 13,
        name: "V13__memory_2_2_durable_tables",
        sql: V13_SQL,
        idempotent: None,
    },
    // Memory 2.2 governance completion: explicit principal ownership and
    // durable persona tombstones. Legacy rows are intentionally not backfilled.
    // Renumbered from experimental V12.
    Migration {
        version: 14,
        name: "V14__principal_ownership_and_persona_governance",
        sql: V14_SQL,
        idempotent: None,
    },
    // M27 / L 组 (2026-09 安全审计): 把运行时懒建表 `dedup_fingerprints`
    // (dedup.rs `ensure_dedup_table`) 收编进迁移列表 — schema 快照与迁移
    // 列表不再漂移. DDL 与 dedup.rs 的懒建版本逐字一致 (幂等, 老库里两边
    // 谁先跑都得到同一张表). `episode_memory_metadata` 已于 V11 收录.
    Migration {
        version: 15,
        name: "V15__dedup_fingerprints",
        sql: r#"
CREATE TABLE IF NOT EXISTS dedup_fingerprints (
    namespace    TEXT NOT NULL,
    fingerprint  TEXT NOT NULL,
    last_seen_ms INTEGER NOT NULL,
    PRIMARY KEY (namespace, fingerprint)
);
CREATE INDEX IF NOT EXISTS idx_dedup_last_seen
    ON dedup_fingerprints(last_seen_ms);
"#,
        idempotent: None,
    },
    // V4-a 人格记忆独立成层: 「我是谁」与「关于用户」分表 (episodes/偏好画像 =
    // 用户事实层; persona_memory_* = 人格层). 状态面语义 (非 append-only 流):
    // 变更经合并策略 (同类合并 / 矛盾标注张力, 不静默覆盖), 见 persona_layer.rs.
    Migration {
        version: 16,
        name: "V16__persona_memory_layer",
        sql: V16_SQL,
        idempotent: None,
    },
    // V4-b 记忆保鲜回路 (保鲜三件套): 散列账 source_hash (同源同散列去重) +
    // 待摄取清单 ingest_queue (重启续摄 / 失败留队列) + 折入段账
    // memory_fold_segments (增量折入: 同主题追加不重写, UPDATE/DELETE 触发器
    // 机器钉死 append-only)。迁移 append-only: 只追加新版本, 老版本不改。
    Migration {
        version: 17,
        name: "V17__memory_freshness_loop",
        sql: V17_SQL,
        idempotent: None,
    },
    Migration {
        version: 18,
        name: "V18__lesson_object",
        sql: crate::lesson::LESSON_SCHEMA_SQL,
        idempotent: None,
    },
    // V19 (W-10): 把 W-4 的**运行时懒建**收编进迁移列表 ——
    // 建 `memory_chain_heads` + 给 `episodes` 加 `chain_hash` / `chain_prev_hash`
    // （此前只有运行时幂等 ALTER ⇒ `schema_migrations` 簿记里没这一版 ⇒
    //   schema 快照与迁移列表漂移，且违反「懒建路径不得新增列」的纪律）。
    // **DDL 不写第二份**: 单一真源是 `memory_chain::ensure_chain_schema()`
    // （写入路径也复用它 ⇒ 表结构与链代码不会漂移）⇒ `sql` 为空, DDL 挂 `idempotent`.
    // 之所以需要钩子而不是静态 SQL: SQLite 的 `ALTER TABLE ... ADD COLUMN`
    // **没有 `IF NOT EXISTS` ⇒ 「列已存在」的库上重跑迁移必然报 duplicate column
    // ⇒ 加列必须先探测列集（`PRAGMA table_info`）再决定是否 ALTER。
    //
    // **不引入任何触发器**: `episodes` 的 append-only 触发器来自 V1, 本迁移
    // 只加列、不改既有触发器 ⇒ append-only 语义不被削弱。
    //
    // **口径**: 迁移 = **IMPLEMENTED**（已登记进 `MIGRATIONS`, 有幂等/空库测试
    // 覆盖）；生效方式 = **DEFAULT ENABLED**（启动期迁移必跑, 无 feature 开关）。
    Migration {
        version: 19,
        name: "V19__memory_chain_schema",
        sql: "",
        idempotent: Some(v19_memory_chain_schema),
    },
];
