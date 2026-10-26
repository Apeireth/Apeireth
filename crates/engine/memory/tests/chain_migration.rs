//! 记忆层哈希链的**正式迁移** + v1 存储**默认写入路径入链**的集成测试（W-10）。
//!
//! 覆盖两笔欠账（W-4 留下的）：
//!
//! 1. **正式迁移**：链的 schema（`memory_chain_heads` 表 + `episodes` 的
//!    `chain_hash` / `chain_prev_hash` 两列）此前只有**运行时幂等 ALTER**
//!    （`memory_chain::ensure_chain_schema`）⇒ `schema_migrations` 簿记里没有
//!    这一版 ⇒ schema 快照与迁移列表漂移。本批由 **V19** 收编。
//! 2. **v1 默认写入路径入链**：`EpisodeStore::put_episode` / `put_episode_full`
//!    此前产出的记录**没有链字段** ⇒ 体检报告算「未纳管」⇒ 链覆盖率取决于
//!    调用方。本批让这些路径**默认入链**（写入时一次 INSERT 落链列）。
//!
//! 口径（诚实标注）：
//! - 库级能力（迁移 + 默认写入路径入链 + 体检）：**IMPLEMENTED**（本文件覆盖）。
//! - 链键 = `continuity_id`（沿用既有占位口径，**不改口径**）：v1 存储的
//!   `put_episode` / `put_episode_full` 写的是占位值 `"default"`，真实主体走
//!   `put_episode_for_subject` / `put_episode_full_for_subject`。
//! - CLI 体检命令（`memory verify`）**不在本文件范围**（另属 CLI crate）。
//!
//! ⚠️ 篡改检测那一节**故意先撞 append-only 触发器**：真实库的 `episodes` 是
//! append-only（UPDATE/DELETE 直接 ABORT）⇒ 事后 UPDATE 补链字段根本不可能
//! （这正是「链列必须在 INSERT 时写」的原因）⇒ 篡改场景只能模拟「绕过应用层
//! 直接改库」（越权写入 / 磁盘损坏 / 触发器被摘掉），测的是**检出能力**。

use apeireth_core::hash_chain::GENESIS_PREVIOUS_HASH;
use apeireth_core::kernel::memory::Episode;
use apeireth_memory::memory_chain::{summarize, verify_subject_chain};
use apeireth_memory::{EpisodeMeta, EpisodeStore, Provenance, SqliteMemoryStore};
use rusqlite::Connection;

/// v1 存储 `continuity_id` 的既有占位值（链键口径，测试与实现同源）。
const PLACEHOLDER_SUBJECT: &str = "default";

fn store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("open in-memory store")
}

fn episode(id: &str, session: &str, timestamp: i64, role: &str) -> Episode {
    Episode {
        id: id.into(),
        timestamp,
        role: role.into(),
        content: format!("content of {id}"),
        session_id: session.into(),
    }
}

/// 读一条记录的链列（`chain_hash` / `chain_prev_hash`）。
fn chain_columns(conn: &Connection, id: &str) -> (Option<String>, Option<String>) {
    conn.query_row(
        "SELECT chain_hash, chain_prev_hash FROM episodes WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("读链列")
}

/// `episodes` 的列名集合。
fn episode_columns(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(episodes)")
        .expect("prepare");
    stmt.query_map([], |row| row.get::<_, String>(1))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect()
}

// ===== ① 正式迁移 =====

/// 迁移后：新库有 `memory_chain_heads` 表 + `episodes` 有两个链列。
#[test]
fn fresh_database_has_chain_heads_table_and_chain_columns() {
    let store = store();
    let conn = store.conn().unwrap();

    let heads_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='memory_chain_heads')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(heads_exists, "V19 必须建 memory_chain_heads 表");

    let cols = episode_columns(&conn);
    assert!(
        cols.iter().any(|c| c == "chain_hash"),
        "episodes 必须有 chain_hash 列, 实际列集: {cols:?}"
    );
    assert!(
        cols.iter().any(|c| c == "chain_prev_hash"),
        "episodes 必须有 chain_prev_hash 列, 实际列集: {cols:?}"
    );
}

/// 迁移登记：`schema_migrations` 簿记里**有这一版**（此前只有运行时 ALTER，
/// 簿记缺口 ⇒ 没人知道库该有这两列）。
#[test]
fn chain_schema_migration_is_registered_in_bookkeeping() {
    let store = store();
    let applied = store.applied_migrations().unwrap();
    assert!(
        applied.contains(&19),
        "V19 必须在簿记里, 实际已应用: {applied:?}"
    );

    let conn = store.conn().unwrap();
    let (name,): (String,) = conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 19",
            [],
            |row| Ok((row.get(0)?,)),
        )
        .unwrap();
    assert_eq!(name, "V19__memory_chain_schema");

    // 迁移列表里也必须有这一版（簿记与迁移列表不得漂移）。
    let entry = apeireth_memory::MIGRATIONS
        .iter()
        .find(|m| m.version == 19)
        .expect("MIGRATIONS 必须含 V19");
    assert_eq!(entry.name, "V19__memory_chain_schema");
    assert!(
        entry.idempotent.is_some(),
        "V19 的 DDL 必须挂在幂等钩子上（SQLite 的 ADD COLUMN 无 IF NOT EXISTS）"
    );
}

/// 空库迁移：全新（空文件）库 ⇒ 迁移跑通。
#[test]
fn migrations_run_on_empty_database() {
    let path = tempfile::NamedTempFile::new().expect("temp file");
    let mut conn = Connection::open(path.path()).expect("open empty db");
    // 空库上跑迁移: 不能报错.
    apeireth_memory::run_migrations(&mut conn).expect("空库迁移必须跑通");

    let heads: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='memory_chain_heads')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(heads, "空库迁移后必须有链头表");
    let cols = episode_columns(&conn);
    assert!(cols.iter().any(|c| c == "chain_hash"));
    assert!(cols.iter().any(|c| c == "chain_prev_hash"));
}

/// 迁移幂等：在**已经加过列**的库上再跑一次迁移 ⇒ 不报错、不重复加列。
///
/// 这正是 W-4 欠账的形状：列已被运行时 ALTER 加过，但簿记里没有 V19 ⇒
/// 重跑迁移若用裸 `ALTER TABLE` 必然报 duplicate column。
#[test]
fn chain_migration_is_idempotent_on_already_altered_database() {
    let mut conn = Connection::open_in_memory().unwrap();
    apeireth_memory::run_migrations(&mut conn).expect("首次迁移");
    // 模拟「列已存在、簿记缺这一版」的欠账状态.
    conn.execute("DELETE FROM schema_migrations WHERE version = 19", [])
        .expect("删簿记行");

    // 再跑一次: 不能报错（duplicate column 会在这里炸）.
    apeireth_memory::run_migrations(&mut conn).expect("重复迁移必须幂等");

    // 簿记补回来了.
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 19",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "V19 簿记必须恰好一条");

    // 列没有重复（每列只出现一次）.
    let cols = episode_columns(&conn);
    assert_eq!(
        cols.iter().filter(|c| *c == "chain_hash").count(),
        1,
        "chain_hash 列不得重复, 实际列集: {cols:?}"
    );
    assert_eq!(
        cols.iter().filter(|c| *c == "chain_prev_hash").count(),
        1,
        "chain_prev_hash 列不得重复, 实际列集: {cols:?}"
    );
}

/// 迁移**不得**引入 UPDATE/DELETE 触发器（append-only 语义不被削弱）。
#[test]
fn chain_migration_does_not_weaken_append_only() {
    let store = store();
    let (episode_triggers, head_triggers) = {
        let conn = store.conn().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name='episodes' ORDER BY name",
            )
            .unwrap();
        let triggers: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        let head: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND tbl_name='memory_chain_heads'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (triggers, head)
    };

    // episodes 的触发器集合 = V1 的那两个（迁移只加列，不动触发器）.
    assert_eq!(
        episode_triggers,
        vec![
            "episodes_no_delete".to_string(),
            "episodes_no_update".to_string()
        ],
        "迁移不得新增/删除 episodes 的触发器"
    );
    // 链头表必须**可变**（链头要推进）⇒ 不该有 append-only 触发器.
    assert_eq!(head_triggers, 0, "链头表不得有触发器（链头要能推进）");

    // 追加性仍然生效: UPDATE / DELETE 都被拒.
    store
        .put_episode(&episode("ep-a", "s-1", 1, "user"))
        .unwrap();
    let conn = store.conn().unwrap();
    let update_err = conn
        .execute("UPDATE episodes SET content = 'x' WHERE id = 'ep-a'", [])
        .unwrap_err();
    assert!(
        update_err.to_string().contains("append-only"),
        "UPDATE 仍须被拒, got {update_err}"
    );
    let delete_err = conn
        .execute("DELETE FROM episodes WHERE id = 'ep-a'", [])
        .unwrap_err();
    assert!(
        delete_err.to_string().contains("append-only"),
        "DELETE 仍须被拒, got {delete_err}"
    );
}

// ===== ② v1 默认写入路径入链 =====

/// v1 默认写入：`EpisodeStore::put_episode` ⇒ 记录**带链字段**（`chain_hash` 非空）。
#[test]
fn default_write_path_put_episode_writes_chain_fields() {
    let store = store();
    store
        .put_episode(&episode("ep-1", "s-1", 1000, "user"))
        .unwrap();

    let conn = store.conn().unwrap();
    let (hash, previous) = chain_columns(&conn, "ep-1");
    let hash = hash.expect("默认写入路径必须写 chain_hash（不能再是「未纳管」记录）");
    assert!(!hash.is_empty(), "chain_hash 不得为空串");
    assert_eq!(
        previous.as_deref(),
        Some(GENESIS_PREVIOUS_HASH),
        "首条记录的前驱必须是创世值"
    );
}

/// v1 默认写入：`put_episode_full`（显式元数据入口）⇒ 同样带链字段。
#[test]
fn default_write_path_put_episode_full_writes_chain_fields() {
    let store = store();
    let meta = EpisodeMeta::now(1_000_000, Provenance::Dialog);
    store
        .put_episode_full(&episode("ep-full", "s-1", 1000, "user"), &meta)
        .unwrap();

    let conn = store.conn().unwrap();
    let (hash, previous) = chain_columns(&conn, "ep-full");
    assert!(
        hash.is_some_and(|value| !value.is_empty()),
        "put_episode_full 必须写 chain_hash"
    );
    assert_eq!(previous.as_deref(), Some(GENESIS_PREVIOUS_HASH));
    // 摘要用的元数据 = 落库的元数据（同一口径）⇒ 来源列确实写进去了.
    let provenance: String = conn
        .query_row(
            "SELECT provenance FROM episodes WHERE id = 'ep-full'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(provenance, "dialog");
}

/// 链首尾相连：写两条 ⇒ 第二条的 `chain_prev_hash` == 第一条的 `chain_hash`。
#[test]
fn chain_links_head_to_tail() {
    let store = store();
    store
        .put_episode(&episode("ep-1", "s-1", 1000, "user"))
        .unwrap();
    store
        .put_episode(&episode("ep-2", "s-1", 2000, "assistant"))
        .unwrap();

    let conn = store.conn().unwrap();
    let (first_hash, _) = chain_columns(&conn, "ep-1");
    let (_, second_previous) = chain_columns(&conn, "ep-2");
    let first_hash = first_hash.expect("ep-1 必须有链摘要");
    assert_eq!(
        second_previous.as_deref(),
        Some(first_hash.as_str()),
        "第二条的前驱必须等于第一条的摘要"
    );
}

/// 体检通过：写完用 `verify_subject_chain` 校验 ⇒ `ok == true`。
#[test]
fn verify_subject_chain_passes_after_default_writes() {
    let store = store();
    store
        .put_episode(&episode("ep-1", "s-1", 1000, "user"))
        .unwrap();
    store
        .put_episode(&episode("ep-2", "s-1", 2000, "assistant"))
        .unwrap();

    let conn = store.conn().unwrap();
    let report = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).unwrap();
    assert!(report.ok, "体检必须通过, 报告: {report:?}");
    assert_eq!(report.entry_count, 2, "两条都应入链");
    assert_eq!(
        report.unmanaged_entries, 0,
        "默认写入路径不该留下未纳管记录"
    );
    assert_eq!(report.broken_entries, 0);
    assert_eq!(report.first_bad_id, None);
}

/// 体检口径：默认写入路径写完后，**未纳管数 = 0**（此前是「取决于调用方」）。
#[test]
fn default_write_path_leaves_no_unmanaged_entries() {
    let store = store();
    for index in 0..3 {
        store
            .put_episode(&episode(
                &format!("ep-{index}"),
                "s-1",
                1000 + index,
                "user",
            ))
            .unwrap();
    }
    let meta = EpisodeMeta::now(5_000_000, Provenance::Tool);
    store
        .put_episode_full(&episode("ep-full", "s-1", 5000, "tool"), &meta)
        .unwrap();

    let conn = store.conn().unwrap();
    let summary = summarize(&conn).unwrap();
    assert_eq!(
        summary.unmanaged_entries, 0,
        "默认写入路径不得产出未纳管记录, 汇总: {summary:?}"
    );
    assert_eq!(summary.entries, 4, "四条都应入链");
    assert_eq!(summary.broken_subjects, 0);
}

/// 真实主体的写入路径：链键 = 真实 `continuity_id`（不是占位值）。
#[test]
fn subject_bound_write_paths_chain_under_real_continuity_id() {
    let store = store();
    store
        .put_episode_for_subject(&episode("ep-sub-1", "s-1", 1000, "user"), "cid-real")
        .unwrap();
    let meta = EpisodeMeta::now(2_000_000, Provenance::Dialog);
    store
        .put_episode_full_for_subject(&episode("ep-sub-2", "s-1", 2000, "user"), &meta, "cid-real")
        .unwrap();

    let conn = store.conn().unwrap();
    let report = verify_subject_chain(&conn, "cid-real").unwrap();
    assert!(report.ok, "真实主体的链必须通过, 报告: {report:?}");
    assert_eq!(report.entry_count, 2);
    // 占位主体那条链不存在 ⇒ 不会把两条链混成一条.
    let placeholder = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).unwrap();
    assert_eq!(placeholder.entry_count, 0);
}

/// 链写入失败 ⇒ **硬失败**（宁可拒写，不可无链写入）。
///
/// 造一个缺 PRIMARY KEY 的同名链头表 ⇒ 链头 UPSERT 必失败 ⇒ 整条写入回滚。
#[test]
fn chain_failure_is_hard_failure_and_writes_nothing() {
    let store = store();
    {
        let conn = store.conn().unwrap();
        conn.execute_batch(
            "DROP TABLE memory_chain_heads;
             CREATE TABLE memory_chain_heads (
                 subject_id    TEXT,
                 last_hash     TEXT,
                 entry_count   INTEGER,
                 updated_at_ms INTEGER
             );",
        )
        .unwrap();
    }

    let error = store
        .put_episode(&episode("ep-fail", "s-1", 1000, "user"))
        .expect_err("链写失败必须硬失败");
    assert!(
        error.to_string().contains("sqlite error"),
        "错误须如实报出, got {error}"
    );

    let conn = store.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE id = 'ep-fail'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "链写失败必须拒写（不能留下无链记录）");
}

/// 篡改内容能被检出 ⇒ 校验失败**且指出是哪一条**。
///
/// 先如实记录：append-only 触发器**拦住** UPDATE（这正是「链列只能在 INSERT 时
/// 写」的根据）⇒ 篡改场景改为「绕过应用层直接改库」（摘掉触发器 = 模拟越权
/// 写入 / 底层文件被直接改）。
#[test]
fn tampered_content_is_detected_and_located() {
    let store = store();
    store
        .put_episode(&episode("ep-1", "s-1", 1000, "user"))
        .unwrap();
    store
        .put_episode(&episode("ep-2", "s-1", 2000, "assistant"))
        .unwrap();

    let conn = store.conn().unwrap();
    // ① append-only 触发器确实拦住 UPDATE（先如实说明，再绕过它测检出）.
    let blocked = conn
        .execute(
            "UPDATE episodes SET content = 'tampered' WHERE id = 'ep-1'",
            [],
        )
        .unwrap_err();
    assert!(
        blocked.to_string().contains("append-only"),
        "append-only 必须先拦住 UPDATE, got {blocked}"
    );

    // ② 绕过应用层直接改库 ⇒ 内容与摘要对不上.
    conn.execute_batch("DROP TRIGGER episodes_no_update")
        .unwrap();
    conn.execute(
        "UPDATE episodes SET content = 'tampered' WHERE id = 'ep-1'",
        [],
    )
    .unwrap();

    // ③ 体检必须失败, 且指出是哪一条（第 0 条 = ep-1）.
    let report = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).unwrap();
    assert!(!report.ok, "被篡改的链不得通过体检, 报告: {report:?}");
    assert_eq!(
        report.first_bad_id.as_deref(),
        Some("ep-1"),
        "必须定位到具体记录, 报告: {report:?}"
    );
    assert_eq!(report.first_bad_index, Some(0));
    assert_eq!(report.broken_entries, 1, "只坏一条");
}

/// 摘要覆盖元数据列：改 `provenance` 同样破链（出处也在链的保护范围内）。
#[test]
fn tampered_provenance_is_detected() {
    let store = store();
    let meta = EpisodeMeta::now(1_000_000, Provenance::Tool);
    store
        .put_episode_full(&episode("ep-meta", "s-1", 1000, "tool"), &meta)
        .unwrap();

    let conn = store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER episodes_no_update")
        .unwrap();
    conn.execute(
        "UPDATE episodes SET provenance = 'manual' WHERE id = 'ep-meta'",
        [],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).unwrap();
    assert!(!report.ok, "改来源必须破链, 报告: {report:?}");
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-meta"));
}

/// 老库记录（没有链字段）= **未纳管**，**不算坏链**（口径不被夸大）。
#[test]
fn legacy_rows_are_unmanaged_not_broken() {
    let store = store();
    store
        .put_episode(&episode("ep-new", "s-1", 1000, "user"))
        .unwrap();
    {
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
             VALUES ('ep-legacy', 'default', 's-1', 2000, 'user', 'legacy row')",
            [],
        )
        .unwrap();
    }

    let conn = store.conn().unwrap();
    let report = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).unwrap();
    assert!(report.ok, "未纳管记录不算坏链, 报告: {report:?}");
    assert_eq!(report.entry_count, 1, "只有新记录在链上");
    assert_eq!(report.unmanaged_entries, 1, "老记录如实计为未纳管");
    assert_eq!(report.broken_entries, 0);
}
