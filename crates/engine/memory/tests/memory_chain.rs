//! 记忆层 D0 哈希链的集成测试（B1-c / 缺口 G6 + G7）。
//!
//! 覆盖：
//! - 链的链接关系（首尾相连、链头推进、幂等重写不推进）；
//! - 篡改检出（改内容 / 改链字段 / 删中间一条）**并定位到具体第几条**；
//! - 主体隔离（两个主体各自独立成链，互不影响）；
//! - 「无链记录 = 未纳管 ≠ 坏链」的语义；
//! - 确定性（同一库连续两次校验逐字相等）与汇总计数；
//! - 老库兼容（没有链列的既有库：初始化不崩，如实体现「未纳管」）；
//! - 两条写入路径真的接上了链（RC-1 后端 + v1 存储显式入口）。
//!
//! **测试 schema 刻意不带 append-only 触发器**：真实库的 `episodes` 是 append-only
//! （UPDATE/DELETE 直接 ABORT）⇒ 篡改检测验的是「绕过应用层直接改库」的场景
//! （磁盘损坏 / 越权写入 / 外部工具），所以这里用一张可改的表来制造损坏。

use std::sync::Arc;

use apeireth_core::hash_chain::GENESIS_PREVIOUS_HASH;
use apeireth_core::kernel::memory::Episode;
use apeireth_memory::backend::MemoryBackend;
use apeireth_memory::memory_chain::{
    ensure_chain_schema, entry_digest, put_chained_episode, summarize, verify_all_chains,
    verify_subject_chain, ChainEntry, ChainSummary,
};
use rusqlite::Connection;

/// 建一个「可篡改」的 `episodes` 表（6 列 + 2 个链列，无 append-only 触发器）。
fn open_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE episodes (
             id TEXT PRIMARY KEY,
             continuity_id TEXT NOT NULL,
             session_id TEXT NOT NULL,
             timestamp INTEGER NOT NULL,
             role TEXT NOT NULL,
             content TEXT NOT NULL,
             chain_hash TEXT,
             chain_prev_hash TEXT
         );",
    )
    .expect("create episodes");
    conn
}

/// 建一个**没有链列**的旧表（模拟 V4 迁移前 / 别的后端 schema 的既有库）。
fn open_legacy_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE episodes (
             id TEXT PRIMARY KEY,
             continuity_id TEXT NOT NULL,
             session_id TEXT NOT NULL,
             timestamp INTEGER NOT NULL,
             role TEXT NOT NULL,
             content TEXT NOT NULL
         );",
    )
    .expect("create legacy episodes");
    conn
}

/// 建一个带 V4 元数据列的表（`valid_from_ms` / `valid_until_ms` / `created_ms` /
/// `provenance`）—— 摘要覆盖这 4 列 ⇒ 改它们必须破链。
fn open_db_with_v4() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE episodes (
             id TEXT PRIMARY KEY,
             continuity_id TEXT NOT NULL,
             session_id TEXT NOT NULL,
             timestamp INTEGER NOT NULL,
             role TEXT NOT NULL,
             content TEXT NOT NULL,
             valid_from_ms INTEGER,
             valid_until_ms INTEGER,
             created_ms INTEGER,
             provenance TEXT,
             chain_hash TEXT,
             chain_prev_hash TEXT
         );",
    )
    .expect("create episodes with v4");
    conn
}

/// 造一条参与链摘要的 episode 快照。
fn entry(id: &str, timestamp: i64, content: &str) -> ChainEntry {
    ChainEntry {
        id: id.to_string(),
        content: content.to_string(),
        timestamp,
        role: "user".to_string(),
        session_id: "sess-1".to_string(),
        valid_from_ms: None,
        valid_until_ms: None,
        created_ms: None,
        provenance: None,
    }
}

/// 写一条带链记录（返回写入结果）。
fn write(conn: &Connection, subject: &str, id: &str, timestamp: i64, content: &str) -> ChainEntry {
    let item = entry(id, timestamp, content);
    put_chained_episode(conn, subject, &item).expect("chained write");
    item
}

/// 链首尾相连：第二条的前驱 == 第一条的摘要，第一条的前驱 == 创世值。
#[test]
fn chain_links_records_head_to_tail() {
    let conn = open_db();
    let first = put_chained_episode(&conn, "alice", &entry("ep-1", 1000, "hello")).unwrap();
    let second = put_chained_episode(&conn, "alice", &entry("ep-2", 2000, "world")).unwrap();

    assert_eq!(first.chain_prev_hash, GENESIS_PREVIOUS_HASH);
    assert_eq!(second.chain_prev_hash, first.chain_hash);
    assert!(first.chained && second.chained);
    assert_eq!(first.entry_count, 1);
    assert_eq!(second.entry_count, 2);

    // 落库的列与返回值一致（链不是只活在内存里）。
    let (hash, previous): (String, String) = conn
        .query_row(
            "SELECT chain_hash, chain_prev_hash FROM episodes WHERE id = 'ep-2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(hash, second.chain_hash);
    assert_eq!(previous, first.chain_hash);

    // 链头表也推进了。
    let (last, count): (String, i64) = conn
        .query_row(
            "SELECT last_hash, entry_count FROM memory_chain_heads WHERE subject_id = 'alice'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(last, second.chain_hash);
    assert_eq!(count, 2);
}

/// 完好链：校验通过、定位字段为空。
#[test]
fn verification_passes_on_intact_chain() {
    let conn = open_db();
    let first = put_chained_episode(&conn, "alice", &entry("ep-1", 1000, "hello")).unwrap();
    let second = put_chained_episode(&conn, "alice", &entry("ep-2", 2000, "world")).unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok, "完好链必须通过: {report:?}");
    assert_eq!(report.first_bad_index, None);
    assert_eq!(report.first_bad_id, None);
    assert_eq!(report.entry_count, 2);
    assert_eq!(report.head_hash, second.chain_hash);
    assert_eq!(report.unmanaged_entries, 0);
    assert_eq!(report.broken_entries, 0);
    assert_ne!(report.head_hash, first.chain_hash);
}

/// 篡改内容 ⇒ 破链，**并指出是哪一条**。
#[test]
fn tampered_content_is_detected_and_located() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");
    write(&conn, "alice", "ep-2", 2000, "world");
    write(&conn, "alice", "ep-3", 3000, "again");

    conn.execute(
        "UPDATE episodes SET content = 'tampered' WHERE id = 'ep-2'",
        [],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "内容被改必须检出");
    assert_eq!(report.first_bad_index, Some(1), "第 2 条（第 1 基）出问题");
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-2"));
    assert_eq!(report.broken_entries, 1);
}

/// 篡改链字段（`chain_hash`）⇒ 检出。
#[test]
fn tampered_chain_field_is_detected() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");
    write(&conn, "alice", "ep-2", 2000, "world");

    conn.execute(
        "UPDATE episodes SET chain_hash = 'deadbeef' WHERE id = 'ep-2'",
        [],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "链字段被改必须检出");
    assert_eq!(report.first_bad_index, Some(1));
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-2"));
}

/// 删除中间一条 ⇒ 断链检出。
///
/// 被删的记录没有 id 可指 ⇒ 定位给的是**断口位置**（第 2 条的位置）+ 链上第一条
/// 「再也接不上」的记录 id。
#[test]
fn deleted_middle_record_breaks_the_chain() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");
    write(&conn, "alice", "ep-2", 2000, "world");
    write(&conn, "alice", "ep-3", 3000, "again");

    conn.execute("DELETE FROM episodes WHERE id = 'ep-2'", [])
        .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "中间少一条必须检出断链");
    assert_eq!(report.first_bad_index, Some(1), "断口在第 2 条的位置");
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-3"));
    assert_eq!(report.entry_count, 2, "链上只剩 2 条可走的记录");
}

/// 删除**最后一条** ⇒ 靠链头账实相符抓到（链本身走通了，链头却还记着 3 条）。
#[test]
fn deleted_tail_record_is_detected_via_head_mismatch() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");
    write(&conn, "alice", "ep-2", 2000, "world");
    write(&conn, "alice", "ep-3", 3000, "again");

    conn.execute("DELETE FROM episodes WHERE id = 'ep-3'", [])
        .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "尾部少一条必须检出（链头账实不符）");
    assert_eq!(report.entry_count, 2);
    assert_eq!(report.first_bad_index, Some(2), "缺的是第 3 条");
    assert_eq!(report.first_bad_id, None, "被删的记录没有 id 可指");
}

/// 分叉（同一个前驱有两条后继）⇒ 检出。
#[test]
fn forked_chain_is_detected() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");

    // 手工塞一条「也指创世前驱」的记录 = 分叉。
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content,
                               chain_hash, chain_prev_hash)
         VALUES ('ep-fork', 'alice', 'sess-1', 2000, 'user', 'forked', 'ff00', ?1)",
        rusqlite::params![GENESIS_PREVIOUS_HASH],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "分叉必须检出");
    assert_eq!(report.first_bad_index, Some(0), "分叉发生在链首位置");
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-fork"));
}

/// 两个主体 ⇒ 各自独立成链，互不影响。
#[test]
fn two_subjects_chain_independently() {
    let conn = open_db();
    let alice = put_chained_episode(&conn, "alice", &entry("a-1", 1000, "a")).unwrap();
    let bob = put_chained_episode(&conn, "bob", &entry("b-1", 1000, "b")).unwrap();

    assert_eq!(alice.chain_prev_hash, GENESIS_PREVIOUS_HASH);
    assert_eq!(bob.chain_prev_hash, GENESIS_PREVIOUS_HASH);
    assert_ne!(alice.chain_hash, bob.chain_hash);

    // 篡改 alice 不影响 bob。
    conn.execute("UPDATE episodes SET content = 'x' WHERE id = 'a-1'", [])
        .unwrap();
    assert!(!verify_subject_chain(&conn, "alice").unwrap().ok);
    assert!(
        verify_subject_chain(&conn, "bob").unwrap().ok,
        "主体之间必须互不影响"
    );
}

/// 空库：没有链 ⇒ 没有可断的地方 ⇒ 通过（且条目数为 0）。
#[test]
fn empty_database_reports_ok_with_zero_entries() {
    let conn = open_db();
    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok);
    assert_eq!(report.entry_count, 0);
    assert_eq!(report.head_hash, GENESIS_PREVIOUS_HASH);
    assert_eq!(report.first_bad_index, None);
    assert!(verify_all_chains(&conn).unwrap().is_empty());

    let summary = summarize(&conn).unwrap();
    assert_eq!(
        summary,
        ChainSummary {
            subjects: 0,
            entries: 0,
            broken_subjects: 0,
            broken_entries: 0,
            unmanaged_entries: 0,
        }
    );
}

/// 无链记录 = **未纳管**，不是坏链（单独计数，不影响 `ok`）。
#[test]
fn records_without_chain_are_unmanaged_not_broken() {
    let conn = open_db();
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES ('legacy-1', 'alice', 'sess-1', 1, 'user', 'old row')",
        [],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok, "没有链字段的记录不算坏链");
    assert_eq!(report.entry_count, 0);
    assert_eq!(report.unmanaged_entries, 1);
    assert_eq!(report.broken_entries, 0);

    let summary = summarize(&conn).unwrap();
    assert_eq!(summary.subjects, 0, "没有链就没有主体");
    assert_eq!(summary.entries, 0);
    assert_eq!(summary.broken_subjects, 0);
    assert_eq!(summary.unmanaged_entries, 1);
}

/// 未纳管记录与坏链可以并存：未纳管不掩盖坏链。
#[test]
fn unmanaged_records_do_not_hide_a_broken_chain() {
    let conn = open_db();
    write(&conn, "alice", "ep-1", 1000, "hello");
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES ('legacy-1', 'alice', 'sess-1', 1, 'user', 'old row')",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE episodes SET content = 'tampered' WHERE id = 'ep-1'",
        [],
    )
    .unwrap();

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok);
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-1"));
    assert_eq!(report.unmanaged_entries, 1);
}

/// 汇总计数正确。
#[test]
fn summary_counts_are_correct() {
    let conn = open_db();
    write(&conn, "alice", "a-1", 1000, "a");
    write(&conn, "alice", "a-2", 2000, "b");
    write(&conn, "bob", "b-1", 1000, "c");
    write(&conn, "bob", "b-2", 2000, "d");
    conn.execute(
        "UPDATE episodes SET content = 'tampered' WHERE id = 'b-1'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES ('legacy-1', 'carol', 'sess-9', 1, 'user', 'old row')",
        [],
    )
    .unwrap();

    let summary = summarize(&conn).unwrap();
    assert_eq!(summary.subjects, 2, "alice + bob");
    assert_eq!(summary.entries, 4, "链上条目总数");
    assert_eq!(summary.broken_subjects, 1, "只有 bob 的链坏了");
    assert_eq!(summary.broken_entries, 1);
    assert_eq!(summary.unmanaged_entries, 1, "carol 那条未纳管");
}

/// 确定性：同一库连续两次校验逐字相等（且主体按 id 排序）。
#[test]
fn verification_is_deterministic() {
    let conn = open_db();
    write(&conn, "alice", "a-1", 1000, "a");
    write(&conn, "bob", "b-1", 1000, "b");
    write(&conn, "alice", "a-2", 2000, "b");

    let first = verify_all_chains(&conn).unwrap();
    let second = verify_all_chains(&conn).unwrap();
    assert_eq!(first, second, "同一库两次校验必须逐字相等");
    assert_eq!(
        first
            .iter()
            .map(|r| r.subject_id.as_str())
            .collect::<Vec<_>>(),
        vec!["alice", "bob"],
        "主体顺序必须确定（按 subject_id 排序）"
    );
}

/// 老库兼容：没有链列的既有库 ⇒ 初始化不崩，且如实体现「未纳管」。
#[test]
fn legacy_schema_without_chain_columns_survives_initialization() {
    let conn = open_legacy_db();
    ensure_chain_schema(&conn).expect("初始化不得崩");

    // 链列被补上了（幂等：再调一次也不崩）。
    let chain_columns: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('episodes')
             WHERE name IN ('chain_hash', 'chain_prev_hash')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(chain_columns, 2, "两列都要补上");
    ensure_chain_schema(&conn).expect("重复初始化不得崩");

    // 旧行没有链字段 ⇒ 未纳管（不是坏链）。
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES ('legacy-1', 'alice', 'sess-1', 1, 'user', 'old row')",
        [],
    )
    .unwrap();
    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok);
    assert_eq!(report.entry_count, 0);
    assert_eq!(report.unmanaged_entries, 1);
    assert_eq!(summarize(&conn).unwrap().unmanaged_entries, 1);
}

/// 幂等重写：同 id 再写一次**不重复入链**、链头不推进。
#[test]
fn duplicate_write_does_not_advance_the_chain() {
    let conn = open_db();
    let first = put_chained_episode(&conn, "alice", &entry("ep-1", 1000, "hello")).unwrap();
    let again = put_chained_episode(&conn, "alice", &entry("ep-1", 1000, "hello")).unwrap();

    assert!(first.chained);
    assert!(!again.chained, "同 id 重写不重复入链");
    assert_eq!(again.chain_hash, first.chain_hash);
    assert_eq!(again.entry_count, 1, "链头不推进");

    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok, "幂等重写不得把链写坏");
    assert_eq!(report.entry_count, 1);
}

/// 摘要覆盖每一个字段：改任一字段 ⇒ 摘要不同。
#[test]
fn digest_covers_every_covered_field() {
    let base = entry("ep-1", 1000, "hello");
    let digest = entry_digest(GENESIS_PREVIOUS_HASH, &base);

    let mut variants: Vec<ChainEntry> = Vec::new();
    let mut changed_id = base.clone();
    changed_id.id = "ep-2".into();
    variants.push(changed_id);
    let mut changed_content = base.clone();
    changed_content.content = "hello!".into();
    variants.push(changed_content);
    let mut changed_timestamp = base.clone();
    changed_timestamp.timestamp = 1001;
    variants.push(changed_timestamp);
    let mut changed_role = base.clone();
    changed_role.role = "assistant".into();
    variants.push(changed_role);
    let mut changed_session = base.clone();
    changed_session.session_id = "sess-2".into();
    variants.push(changed_session);
    let mut changed_valid_from = base.clone();
    changed_valid_from.valid_from_ms = Some(1);
    variants.push(changed_valid_from);
    let mut changed_valid_until = base.clone();
    changed_valid_until.valid_until_ms = Some(1);
    variants.push(changed_valid_until);
    let mut changed_created = base.clone();
    changed_created.created_ms = Some(1);
    variants.push(changed_created);
    let mut changed_provenance = base.clone();
    changed_provenance.provenance = Some("tool".into());
    variants.push(changed_provenance);

    for variant in variants {
        assert_ne!(
            entry_digest(GENESIS_PREVIOUS_HASH, &variant),
            digest,
            "字段 {:?} 变了摘要必须变",
            variant
        );
    }

    // 长度前缀消除拼接歧义：`id="ab", content="c"` 与 `id="a", content="bc"`
    // 的字节拼法相同 ⇒ 没有长度前缀就分不出两条记录还是一条。
    let mut left = base.clone();
    left.id = "ab".into();
    left.content = "c".into();
    let mut right = base.clone();
    right.id = "a".into();
    right.content = "bc".into();
    assert_ne!(
        entry_digest(GENESIS_PREVIOUS_HASH, &left),
        entry_digest(GENESIS_PREVIOUS_HASH, &right)
    );
}

/// V4 元数据列（出处 / 双时间）也在摘要内 ⇒ 改它们必须破链。
#[test]
fn v4_metadata_columns_are_covered_by_the_chain() {
    let conn = open_db_with_v4();
    let mut item = entry("ep-1", 1000, "hello");
    item.valid_from_ms = Some(1_000);
    item.created_ms = Some(1_000);
    item.provenance = Some("tool".into());
    put_chained_episode(&conn, "alice", &item).unwrap();

    assert!(verify_subject_chain(&conn, "alice").unwrap().ok);

    conn.execute(
        "UPDATE episodes SET provenance = 'manual' WHERE id = 'ep-1'",
        [],
    )
    .unwrap();
    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(!report.ok, "改出处必须破链");
    assert_eq!(report.first_bad_index, Some(0));
    assert_eq!(report.first_bad_id.as_deref(), Some("ep-1"));
}

/// 写入路径接上了链：RC-1 后端 `SqliteBackend::put_episode`。
#[tokio::test]
async fn sqlite_backend_write_path_is_chained() {
    let pool = Arc::new(
        apeireth_storage::SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool"),
    );
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .expect("memory migrations");

    let backend = apeireth_memory::backend::sqlite::SqliteBackend::from_arc(Arc::clone(&pool));
    let episode = Episode {
        id: "ep-1".into(),
        timestamp: 1_700_000_000,
        role: "user".into(),
        content: "原文".into(),
        session_id: "sess-1".into(),
    };
    backend
        .put_episode(&episode)
        .expect("chained backend write");

    let report = pool
        .read(|conn| {
            verify_subject_chain(conn, "sess-1")
                .map_err(|error| apeireth_storage::StorageError::Serialization(error.to_string()))
        })
        .expect("pool read");
    assert!(report.ok, "后端写入必须落在链上: {report:?}");
    assert_eq!(report.entry_count, 1);
    assert_eq!(report.unmanaged_entries, 0);
}

/// 写入路径接上了链：v1 存储的显式入口 `put_episode_chained`。
#[test]
fn v1_store_explicit_chained_entry_point() {
    let store = apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap();
    let episode = Episode {
        id: "ep-1".into(),
        timestamp: 1_700_000_000,
        role: "user".into(),
        content: "原文".into(),
        session_id: "sess-1".into(),
    };
    let meta = apeireth_memory::EpisodeMeta {
        valid_from_ms: Some(1_700_000_000_000),
        valid_until_ms: None,
        created_ms: 1_700_000_000_000,
        provenance: apeireth_memory::Provenance::Dialog,
    };

    let outcome = store
        .put_episode_chained(&episode, &meta, "alice")
        .expect("chained write");
    assert!(outcome.chained);
    assert_eq!(outcome.entry_count, 1);
    assert_eq!(outcome.chain_prev_hash, GENESIS_PREVIOUS_HASH);

    let conn = store.conn().unwrap();
    let report = verify_subject_chain(&conn, "alice").unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.entry_count, 1);
    assert_eq!(report.head_hash, outcome.chain_hash);
}

/// 写入失败 ⇒ 硬失败（不静默降级成「无链写入」）。
#[test]
fn chained_write_rejects_invalid_subject() {
    let conn = open_db();
    let error = put_chained_episode(&conn, "  ", &entry("ep-1", 1000, "hello")).unwrap_err();
    assert!(
        error.to_string().contains("subject_id is empty"),
        "错误必须说清原因: {error}"
    );

    let error = put_chained_episode(&conn, "alice", &entry("", 1000, "hello")).unwrap_err();
    assert!(
        error.to_string().contains("id is empty"),
        "错误必须说清原因: {error}"
    );
}
