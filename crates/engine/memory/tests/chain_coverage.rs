//! 记忆层哈希链的**链覆盖率**集成测试（W-13）。
//!
//! # 这一批收编了什么
//!
//! 上一批（W-10）把 v1 存储的四条默认写入路径收编进哈希链，但**还有两条路径绕过
//! `EpisodeStore` 直接 raw SQL INSERT** ⇒ 产出**没有链字段**的记录 ⇒ 体检报告
//! （`memory_chain::summarize`）把它们算成「未纳管」⇒ 链覆盖率有缺口：
//!
//! 1. `continuity_link::migrate_subject`（生产路径：主体锚点迁移的副本写入）——
//!    **已入链**（链键 = 副本的 `continuity_id` = 新锚点）。
//! 2. `lesson.rs` 里的那条 raw INSERT —— **结论：它写的是 `episodes` 表**
//!    （`lesson` 模块的 `#[cfg(test)]` 夹具 `seed_episode`）⇒ 已改为入链；
//!    而 `lesson` 模块的**生产**写路径（`append_lesson_candidate` 等）写的是
//!    `lessons` 表 ⇒ **本来就不该入链**（链只管 `episodes`）—— 见
//!    `lesson_path_does_not_touch_the_chain`。
//!
//! # 口径（诚实标注，四级口径）
//!
//! - 库级能力（迁移副本入链 + 夹具入链 + 体检）：**IMPLEMENTED**
//!   （本文件覆盖）。
//! - 生产接线：`migrate_subject` **未接线** —— 全仓**零生产调用点**（只有 `pub`
//!   API 与测试调用）⇒ 只能说库级能力已实现，不能说「生产已受保护」。
//! - `lessons` 表：**不在链的范围内**（链只管 `episodes` 的权威记忆记录）；
//!   `lessons` 的可追溯性由它自己的 `revision` CAS + `source_episodes` 外键
//!   语义承担，**不假装**它有哈希链保护。
//!
//! # ⚠️ 存量无链行**无法回填**（append-only 的硬约束）
//!
//! `episodes` 有 append-only 触发器（`BEFORE UPDATE` / `BEFORE DELETE` 直接
//! ABORT）⇒ 链字段只能在 **INSERT 时**一次写好 ⇒ **历史上已经写进去的无链行
//! 永远无法补上链字段**（改回去会被触发器拒绝）。
//! ⇒ 链覆盖率只能**向前**提高，不能事后回填 —— 见
//! `legacy_unmanaged_rows_cannot_be_backfilled_afterwards`。

use apeireth_core::hash_chain::GENESIS_PREVIOUS_HASH;
use apeireth_core::kernel::memory::Episode;
use apeireth_memory::lesson::{LessonScope, NewLesson};
use apeireth_memory::memory_chain::{summarize, verify_subject_chain};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use rusqlite::Connection;

/// v1 存储 `put_episode` 的既有占位链键（沿用既有口径，不改口径）。
const PLACEHOLDER_SUBJECT: &str = "default";
/// 迁移的目标锚点（= 副本的 `continuity_id` = 副本所在链的键）。
const TARGET_SUBJECT: &str = "c-main";
/// 迁移的源锚点（`migrate_subject` 按 **session_id** 选源记录）。
const SOURCE_SESSION: &str = "me";

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

/// 造两条源经历（session = `me`），走默认写入路径（它们本身已入链）。
fn seed_sources(store: &SqliteMemoryStore) {
    store
        .put_episode(&episode("ep-0", SOURCE_SESSION, 100, "user"))
        .expect("seed ep-0");
    store
        .put_episode(&episode("ep-1", SOURCE_SESSION, 101, "assistant"))
        .expect("seed ep-1");
}

/// 读一条记录的链列。
fn chain_columns(conn: &Connection, id: &str) -> (Option<String>, Option<String>) {
    conn.query_row(
        "SELECT chain_hash, chain_prev_hash FROM episodes WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("读链列")
}

/// 数某主体下**没有链字段**的行数（未纳管）。
fn unmanaged_rows(conn: &Connection, subject: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM episodes WHERE continuity_id = ?1 AND chain_hash IS NULL",
        [subject],
        |row| row.get(0),
    )
    .expect("数未纳管行")
}

/// 迁移副本**带链字段**：走 `migrate_subject` 之后，副本的 `chain_hash` 非空。
#[test]
fn migrated_episodes_carry_chain_fields() {
    let store = store();
    seed_sources(&store);

    let report = apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200)
        .expect("迁移必须成功");
    assert_eq!(report.episodes_copied, 2, "两条源记录都应被复制");
    assert_eq!(report.episodes_skipped, 0);

    let conn = store.conn().unwrap();
    for id in ["mig-ep-0", "mig-ep-1"] {
        let (hash, previous) = chain_columns(&conn, id);
        assert!(
            hash.as_deref().is_some_and(|value| !value.is_empty()),
            "迁移副本必须带链字段（此前是裸 INSERT ⇒ 未纳管）: {id}"
        );
        assert!(
            previous.as_deref().is_some_and(|value| !value.is_empty()),
            "迁移副本必须带前驱摘要: {id}"
        );
    }
    // 首条副本的前驱是创世值（副本是新链的第一条）。
    let (_, first_previous) = chain_columns(&conn, "mig-ep-0");
    assert_eq!(
        first_previous.as_deref(),
        Some(GENESIS_PREVIOUS_HASH),
        "新主体链的首条必须是创世前驱"
    );
}

/// 链首尾相连：第二条副本的 `chain_prev_hash` == 第一条的 `chain_hash`。
#[test]
fn migrated_episodes_are_linked_head_to_tail() {
    let store = store();
    seed_sources(&store);
    apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200).expect("迁移");

    let conn = store.conn().unwrap();
    let (first_hash, _) = chain_columns(&conn, "mig-ep-0");
    let (_, second_previous) = chain_columns(&conn, "mig-ep-1");
    let first_hash = first_hash.expect("首条副本必须带摘要");
    assert_eq!(
        second_previous.as_deref(),
        Some(first_hash.as_str()),
        "第二条副本的前驱必须等于第一条的摘要（批量迁移要推进链头）"
    );
}

/// 体检通过：迁移后用 `verify_subject_chain` 校验 ⇒ `ok == true`。
#[test]
fn verify_subject_chain_passes_after_migration() {
    let store = store();
    seed_sources(&store);
    apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200).expect("迁移");

    let conn = store.conn().unwrap();
    let report = verify_subject_chain(&conn, TARGET_SUBJECT).expect("体检");
    assert!(report.ok, "迁移后的链必须通过体检: {report:?}");
    assert_eq!(report.entry_count, 2, "两条副本都应入链");
    assert_eq!(report.unmanaged_entries, 0, "不得留下未纳管副本");
    assert_eq!(report.broken_entries, 0);
    assert_eq!(report.first_bad_id, None);
}

/// 「未纳管」计数**不增加**：迁移前后对比 `summarize`。
///
/// **如实说明**：理想是「减少」，但存量无链行**无法回填**（append-only）⇒ 能做到
/// 的底线是「迁移不制造新的未纳管记录」。
#[test]
fn migration_does_not_increase_unmanaged_entries() {
    let store = store();
    seed_sources(&store);

    let conn = store.conn().unwrap();
    let before = summarize(&conn).expect("迁移前汇总");
    drop(conn);

    apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200).expect("迁移");

    let conn = store.conn().unwrap();
    let after = summarize(&conn).expect("迁移后汇总");
    assert!(
        after.unmanaged_entries <= before.unmanaged_entries,
        "迁移不得制造未纳管记录: 迁移前 {before:?} → 迁移后 {after:?}"
    );
    assert_eq!(after.unmanaged_entries, 0, "本例不该有未纳管记录");
    assert_eq!(
        after.entries,
        before.entries + 2,
        "两条副本必须入链（链上条目数 +2）"
    );
    assert_eq!(after.broken_subjects, 0, "迁移不得造出坏链");
}

/// 幂等：重跑迁移**不重复入链**、链头不推进（与旧 `INSERT OR IGNORE` 口径一致）。
#[test]
fn migration_is_idempotent_and_does_not_advance_the_chain() {
    let store = store();
    seed_sources(&store);

    let first = apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200)
        .expect("首次迁移");
    assert_eq!(first.episodes_copied, 2);

    let conn = store.conn().unwrap();
    let head_before: String = conn
        .query_row(
            "SELECT last_hash FROM memory_chain_heads WHERE subject_id = ?1",
            [TARGET_SUBJECT],
            |row| row.get(0),
        )
        .expect("读链头");
    drop(conn);

    let second = apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 201)
        .expect("重跑迁移");
    assert_eq!(second.episodes_copied, 0, "重跑不得新增副本");
    assert_eq!(second.episodes_skipped, 2, "两条都应被跳过");

    let conn = store.conn().unwrap();
    let head_after: String = conn
        .query_row(
            "SELECT last_hash FROM memory_chain_heads WHERE subject_id = ?1",
            [TARGET_SUBJECT],
            |row| row.get(0),
        )
        .expect("读链头");
    assert_eq!(head_before, head_after, "幂等重跑不得推进链头");
    let report = verify_subject_chain(&conn, TARGET_SUBJECT).expect("体检");
    assert!(report.ok, "幂等重跑不得把链写坏: {report:?}");
    assert_eq!(report.entry_count, 2, "链上仍是两条");
}

/// 主体隔离：迁移只动**目标主体**那条链，源记录所在的那条链**不受影响**
/// （原记录留在旧主体链上，append-only）。
#[test]
fn migration_leaves_the_source_chain_untouched() {
    let store = store();
    seed_sources(&store);

    let conn = store.conn().unwrap();
    let source_before = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).expect("源链体检");
    drop(conn);

    apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200).expect("迁移");

    let conn = store.conn().unwrap();
    let source_after = verify_subject_chain(&conn, PLACEHOLDER_SUBJECT).expect("源链体检");
    assert_eq!(
        source_after.entry_count, source_before.entry_count,
        "源记录的链不得因迁移而变长"
    );
    assert_eq!(
        source_after.head_hash, source_before.head_hash,
        "源链的链头不得被迁移推进"
    );
    assert!(source_after.ok, "源链必须仍然完好: {source_after:?}");

    // 副本落在**新主体**那条链上（链键 = continuity_id）。
    let target = verify_subject_chain(&conn, TARGET_SUBJECT).expect("目标链体检");
    assert_eq!(target.entry_count, 2);
    assert_eq!(target.unmanaged_entries, 0);
}

/// 失败语义：链写不进去 ⇒ **硬失败**（返回 `Err`）且**不留下无链记录**。
///
/// 造一个缺 PRIMARY KEY 的同名链头表 ⇒ 链头 UPSERT 必失败 ⇒ 整条写入回滚。
#[test]
fn chain_write_failure_is_hard_failure_and_writes_nothing() {
    let store = store();
    seed_sources(&store);
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

    let error = apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200)
        .expect_err("链写不进去必须硬失败（宁可迁移失败，不可留下无链记录）");
    assert!(
        error.to_string().contains("sqlite error"),
        "错误须如实报出, got {error}"
    );

    let conn = store.conn().unwrap();
    let copies: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE continuity_id = ?1",
            [TARGET_SUBJECT],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(copies, 0, "链写失败必须一条副本都不落（不能留下无链记录）");
    assert_eq!(
        unmanaged_rows(&conn, TARGET_SUBJECT),
        0,
        "链写失败不得留下未纳管行"
    );
}

/// 失败语义（批中段）：第 k 条写不进去 ⇒ 返回 `Err`，且**已落地的副本仍全部带链**。
///
/// 用 BEFORE INSERT 触发器拦掉第 2 条副本（模拟中途失败）⇒ 第 1 条已提交。
#[test]
fn mid_batch_chain_failure_leaves_no_unmanaged_row() {
    let store = store();
    seed_sources(&store);
    {
        let conn = store.conn().unwrap();
        conn.execute_batch(
            "CREATE TRIGGER block_second_copy BEFORE INSERT ON episodes
             WHEN NEW.id = 'mig-ep-1'
             BEGIN SELECT RAISE(ABORT, 'blocked by test'); END;",
        )
        .unwrap();
    }

    let error = apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200)
        .expect_err("第 2 条写不进去必须硬失败");
    assert!(
        error.to_string().contains("sqlite error"),
        "错误须如实报出, got {error}"
    );

    let conn = store.conn().unwrap();
    let (hash, previous) = chain_columns(&conn, "mig-ep-0");
    assert!(
        hash.as_deref().is_some_and(|value| !value.is_empty()),
        "已落地的第 1 条副本必须带链字段"
    );
    assert!(previous.is_some(), "已落地的第 1 条副本必须带前驱摘要");
    assert_eq!(
        unmanaged_rows(&conn, TARGET_SUBJECT),
        0,
        "批中段失败也不得留下未纳管行"
    );
    assert_eq!(
        summarize(&conn).expect("汇总").unmanaged_entries,
        0,
        "全库口径：不得留下未纳管记录"
    );
}

/// `lesson` 模块的**生产**写路径写的是 `lessons` 表 ⇒ **不该入链**（链只管
/// `episodes`）⇒ 它不改变链上的记录数。
///
/// **说明性测试**（防止后人误以为这里漏了）：`lessons` 的行进了 `lessons` 表，
/// `episodes` 的行数与链上条目数都**不变**。
#[test]
fn lesson_path_does_not_touch_the_chain() {
    let store = store();
    let source = episode("ep-src", "s-1", 100, "user");
    store.put_episode(&source).expect("写入源经历");

    let conn = store.conn().unwrap();
    let before = summarize(&conn).expect("写入前汇总");
    let episodes_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    drop(conn);

    let lesson = store
        .append_lesson_candidate(NewLesson {
            statement: "专注时不要提醒我喝水，结束后再提醒".to_string(),
            scope: LessonScope::Interaction,
            source_episodes: vec![source.id.clone()],
            evidence: "用户原话：我专注时别提醒我喝水".to_string(),
        })
        .expect("追加候选教训");

    let conn = store.conn().unwrap();
    let after = summarize(&conn).expect("写入后汇总");
    let episodes_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        episodes_after, episodes_before,
        "教训走 `lessons` 表 ⇒ `episodes` 行数不变"
    );
    assert_eq!(
        after.entries, before.entries,
        "链只管 `episodes` ⇒ 教训不进链（不是漏了，是范围外）"
    );
    assert_eq!(after.unmanaged_entries, before.unmanaged_entries);

    // 教训行确实落在 `lessons` 表里（不是漏写，而是写到另一张表）。
    let lessons: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM lessons WHERE id = ?1",
            [&lesson.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lessons, 1, "教训必须落在 `lessons` 表");
}

/// 存量无链行**无法回填**（append-only 的硬约束）⇒ 链覆盖率只能向前提高。
///
/// 这是本批的**已知边界**（不假装能修）：历史写进去的裸 INSERT 行没有链字段，
/// 而 `episodes` 的 append-only 触发器**拒绝 UPDATE** ⇒ 补链字段不可能。
#[test]
fn legacy_unmanaged_rows_cannot_be_backfilled_afterwards() {
    let store = store();
    seed_sources(&store);
    {
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
             VALUES ('legacy-1', 'default', 'me', 50, 'user', 'legacy row')",
            [],
        )
        .unwrap();
    }

    let conn = store.conn().unwrap();
    assert_eq!(
        unmanaged_rows(&conn, PLACEHOLDER_SUBJECT),
        1,
        "老行如实计为未纳管"
    );
    // 补链字段在物理上不可能：append-only 触发器拦住 UPDATE。
    let blocked = conn
        .execute(
            "UPDATE episodes SET chain_hash = 'x' WHERE id = 'legacy-1'",
            [],
        )
        .expect_err("append-only 必须拦住补链字段的 UPDATE");
    assert!(
        blocked.to_string().contains("append-only"),
        "补链字段必须被 append-only 拦下, got {blocked}"
    );
    drop(conn);

    // 迁移只保证**新副本**入链；存量无链行仍然如实计入「未纳管」。
    apeireth_memory::migrate_subject(&store, SOURCE_SESSION, TARGET_SUBJECT, 200).expect("迁移");
    let conn = store.conn().unwrap();
    let summary = summarize(&conn).expect("汇总");
    assert_eq!(
        summary.unmanaged_entries, 1,
        "存量无链行仍如实计入未纳管（不可回填，不假装）"
    );
    assert_eq!(
        summary.entries, 5,
        "两条源 + 三条副本（老行也被复制了一份，副本入链）都在链上"
    );
    assert_eq!(summary.broken_subjects, 0);
}
