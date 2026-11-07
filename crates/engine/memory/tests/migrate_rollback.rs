//! `migrate_rollback` 的测试：把「迁移回滚」的三条纪律**钉死**。
//!
//! 设计稿 §8 **B4**（脱机归档 + **迁移回滚** + handover 演练）+ §2 **P1**
//! （数据保真优先 · 原始证据永久保留）。
//!
//! 重点不在「能删掉东西」，而在**纪律不许漂**：
//!
//! * **只删副本**（`mig-` 前缀）⇒ 原记录**一条都不许少**（P1 底线）；
//! * **有账可查** ⇒ 副本缺失 / 被篡改 / 没链字段 ⇒ **不许删**（`Mismatch`）；
//! * **原记录不在了 ⇒ 绝不许回滚**（`SourceMissing`）；
//! * **失败即失败** ⇒ 任一步不符 ⇒ **一个副本都不删**；
//! * **回到原点** ⇒ 回滚后的**内容哈希**必须与迁移前**逐字相等**（核心断言）。
//!
//! ⚠️ **端到端纪律**：副本一律由**真实的** `continuity_link::migrate_subject` 造
//! （不许用假 INSERT 糊弄）⇒ 因此测试库是**文件库**（演练与真迁移都需要同库第二个连接）。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::continuity_link::{migrate_subject, MIGRATED_ID_PREFIX};
use apeireth_memory::migrate_rollback::{
    drill_rollback, prepare_ledger, rollback_migration, verify_migration, MigrationLedger,
    RollbackVerdict,
};
use apeireth_memory::offline_archive::{export_archive, verify_archive, ExportConfig};
use apeireth_memory::SqliteMemoryStore;
use rusqlite::{params, Connection};
use tempfile::TempDir;

/// 导出/建账时刻（**注入** ⇒ 确定性）。
const CREATED_AT_MS: i64 = 1_760_000_000_000;

/// 源锚点（**会话锚点** ⇒ 与 `migrate_subject` 的取源口径同源）。
const FROM: &str = "anchor-a";

/// 目标锚点。
const TO: &str = "anchor-b";

/// 原记录的主体（迁移**不会**动它 ⇒ 回滚后必须原样还在）。
const SUBJECT: &str = "subject-x";

/// 一次性的**文件库**（真实 schema ⇒ 跑过真实迁移 ⇒ append-only 触发器齐备）。
///
/// **为什么不用内存库**：`drill_rollback` 与真迁移都要在**同库第二个连接**上跑
/// （`SqliteMemoryStore` 持自己的 `Mutex<Connection>` ⇒ 不能塞进现成的连接）
/// ⇒ 纯内存库没法重开 ⇒ 退化成文件库。
struct Fixture {
    /// 临时目录（**必须活得比库久** ⇒ 字段名带下划线免得被误删）。
    _dir: TempDir,
    /// 应用层 store（真迁移走它 ⇒ 生产同款写路径）。
    store: SqliteMemoryStore,
    /// 第二个连接（核对 / 回滚 / 导出走它）。
    conn: Connection,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("建临时目录");
        let path = dir.path().join("memory.sqlite3");
        let store = SqliteMemoryStore::open(&path).expect("开库");
        let conn = Connection::open(&path).expect("开第二个连接");
        Self {
            _dir: dir,
            store,
            conn,
        }
    }

    /// 写入 3 条**带链**的原记录（生产默认写路径 ⇒ 自动入链）。
    fn seed(&self) {
        for (id, ts, content) in [
            ("ep-a", 100_i64, "事实一"),
            ("ep-b", 101_i64, "事实二\n带换行与\"引号\""),
            ("ep-c", 102_i64, "事实三"),
        ] {
            let episode = Episode {
                id: id.to_string(),
                timestamp: ts,
                role: "user".to_string(),
                content: content.to_string(),
                session_id: FROM.to_string(),
            };
            self.store
                .put_episode_for_subject(&episode, SUBJECT)
                .expect("写入原记录");
        }
    }

    /// 当前库的内容哈希（回滚前后对比用）。
    fn digest(&self) -> String {
        export_archive(&self.conn, &ExportConfig::new(), CREATED_AT_MS)
            .expect("导出")
            .manifest
            .content_digest
    }

    /// 当前库里的记录 id（**有序**）。
    fn ids(&self) -> Vec<String> {
        let archive =
            export_archive(&self.conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
        archive.episodes.iter().map(|e| e.id.clone()).collect()
    }
}

/// 模拟篡改者：摘掉 append-only 的 UPDATE 触发器再改正文。
///
/// **为什么要摘触发器**：`episodes` 的 append-only 触发器会挡住一切原地修改
/// ⇒ 想造「内容被改过」的现场，只能像真攻击者那样先拆保护。
fn tamper_content(conn: &Connection, id: &str, content: &str) {
    conn.execute_batch("DROP TRIGGER IF EXISTS episodes_no_update")
        .expect("摘掉更新触发器");
    conn.execute(
        "UPDATE episodes SET content = ?2 WHERE id = ?1",
        params![id, content],
    )
    .expect("篡改正文");
}

/// 模拟数据丢失：摘掉 append-only 的 DELETE 触发器再删原记录。
fn delete_record(conn: &Connection, id: &str) {
    conn.execute_batch("DROP TRIGGER IF EXISTS episodes_no_delete")
        .expect("摘掉删除触发器");
    conn.execute("DELETE FROM episodes WHERE id = ?1", params![id])
        .expect("删除记录");
}

// ============================================================
// prepare_ledger：账的内容 + 纯读
// ============================================================

#[test]
fn prepare_ledger_records_every_source_id_in_order() {
    let mut fixture = Fixture::new();
    fixture.seed();

    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    assert_eq!(
        snapshot.source_ids,
        vec!["ep-a".to_string(), "ep-b".to_string(), "ep-c".to_string()],
        "源锚点下的**全部**记录 id 都要进账，且**有序**"
    );
    assert_eq!(
        ledger.copied_ids,
        vec![
            "mig-ep-a".to_string(),
            "mig-ep-b".to_string(),
            "mig-ep-c".to_string()
        ],
        "账里的副本承诺 = 源 id 加 `mig-` 前缀（与 migrate_subject 同一套规则）"
    );
    assert_eq!(ledger.from, FROM);
    assert_eq!(ledger.to, TO);
    assert_eq!(ledger.executed_at_ms, CREATED_AT_MS, "迁移时刻是**注入**的");
}

#[test]
fn prepare_ledger_snapshot_is_not_empty() {
    let mut fixture = Fixture::new();
    fixture.seed();

    let (snapshot, _ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    assert_eq!(
        snapshot.before.manifest.episode_count, 3,
        "迁移前快照必须含全部原记录"
    );
    assert!(
        snapshot.before.manifest.content_digest.len() == 64,
        "快照必须带内容哈希（回滚后拿它对比 ⇒ 证明回到原点）"
    );
    verify_archive(&snapshot.before).expect("迁移前快照必须自洽");
}

#[test]
fn prepare_ledger_does_not_write_to_the_database() {
    let mut fixture = Fixture::new();
    fixture.seed();

    let before_digest = fixture.digest();
    let before_ids = fixture.ids();
    let rows_before: i64 = fixture
        .conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .expect("数记录");

    let _ = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    assert_eq!(
        before_digest,
        fixture.digest(),
        "建账不许改库（内容哈希不变）"
    );
    assert_eq!(before_ids, fixture.ids(), "建账不许改库（记录集合不变）");
    let rows_after: i64 = fixture
        .conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .expect("数记录");
    assert_eq!(rows_before, rows_after, "建账不许写库（条数不变）");
}

#[test]
fn prepare_ledger_rejects_empty_or_same_anchor() {
    let mut fixture = Fixture::new();
    assert!(prepare_ledger(&fixture.conn, "", TO, CREATED_AT_MS).is_err());
    assert!(prepare_ledger(&fixture.conn, FROM, "  ", CREATED_AT_MS).is_err());
    assert!(prepare_ledger(&fixture.conn, FROM, FROM, CREATED_AT_MS).is_err());
}

// ============================================================
// verify_migration：四档判定
// ============================================================

#[test]
fn verify_migration_is_safe_after_real_migration() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    let report = migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");
    assert_eq!(
        report.episodes_copied, 3,
        "真实 migrate_subject 必须复制出 3 条副本"
    );
    assert_eq!(report.episodes_skipped, 0);

    // 端到端：副本**确实**在库里（不是假 INSERT 糊弄出来的）。
    let archive = export_archive(&fixture.conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    for id in &ledger.copied_ids {
        let copy = archive
            .episodes
            .iter()
            .find(|episode| &episode.id == id)
            .unwrap_or_else(|| panic!("副本 {id} 必须真的在库里"));
        assert!(copy.id.starts_with(MIGRATED_ID_PREFIX));
        assert_eq!(copy.continuity_id, TO, "副本必须落在新主体上");
        assert!(copy.chain_hash.is_some(), "副本必须**带链字段**");
    }
    assert!(
        archive
            .episodes
            .iter()
            .any(|episode| episode.id == "ep-a" && episode.content == "事实一"),
        "原记录必须还在（P1）"
    );

    assert_eq!(
        verify_migration(&fixture.conn, &snapshot, &ledger),
        RollbackVerdict::Safe,
        "账实相符 ⇒ 可以回滚"
    );
}

#[test]
fn verify_migration_detects_tampered_copy() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");

    tamper_content(&fixture.conn, "mig-ep-a", "被改过的副本");

    assert_eq!(
        verify_migration(&fixture.conn, &snapshot, &ledger),
        RollbackVerdict::Mismatch,
        "副本内容被改 ⇒ 账实不符 ⇒ 不许删"
    );
}

#[test]
fn verify_migration_detects_missing_source_record() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");

    delete_record(&fixture.conn, "ep-b");

    assert_eq!(
        verify_migration(&fixture.conn, &snapshot, &ledger),
        RollbackVerdict::SourceMissing,
        "原记录不在了 ⇒ 绝不许回滚（P1 底线）"
    );
}

#[test]
fn verify_migration_reports_nothing_to_rollback_without_migration() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    assert_eq!(
        verify_migration(&fixture.conn, &snapshot, &ledger),
        RollbackVerdict::NothingToRollback,
        "没迁移过 ⇒ 没有可回滚的"
    );
}

#[test]
fn verify_migration_reports_nothing_to_rollback_for_an_empty_ledger() {
    let mut fixture = Fixture::new();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");

    assert!(ledger.copied_ids.is_empty(), "空库 ⇒ 账里没有副本承诺");
    assert_eq!(
        verify_migration(&fixture.conn, &snapshot, &ledger),
        RollbackVerdict::NothingToRollback
    );
}

// ============================================================
// rollback_migration：只删副本 + 回到原点
// ============================================================

#[test]
fn rollback_removes_copies_and_keeps_every_original() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");

    let outcome = rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚");

    assert_eq!(outcome.verdict, RollbackVerdict::Safe);
    assert_eq!(
        outcome.removed_ids, ledger.copied_ids,
        "只准删账里的副本，一个不多一个不少"
    );
    let ids = fixture.ids();
    for copy_id in &ledger.copied_ids {
        assert!(!ids.contains(copy_id), "副本 {copy_id} 必须被删掉");
    }
    for source_id in &snapshot.source_ids {
        assert!(
            ids.contains(source_id),
            "原记录 {source_id} **一条都不许少**（P1 底线）"
        );
    }
}

#[test]
fn rollback_restores_the_pre_migration_content_digest() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let before_digest = fixture.digest();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");
    assert_ne!(
        before_digest,
        fixture.digest(),
        "迁移后库必须**真的变了**（不然这个断言是空的）"
    );

    let outcome = rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚");

    assert_eq!(
        outcome.after.manifest.content_digest, before_digest,
        "回滚后内容哈希必须与迁移前**逐字相等** ⇒ 数据真的回到了原点"
    );
    assert_eq!(outcome.after.manifest.episode_count, 3);
    verify_archive(&outcome.after).expect("回滚后的归档物必须自洽");
}

#[test]
fn rollback_restores_the_chain_head_of_the_target_anchor() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");

    let outcome = rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚");

    assert!(
        !outcome
            .after
            .chain_heads
            .iter()
            .any(|head| head.subject_id == TO),
        "迁移新建的链头必须被退回去（否则链账与链上条数对不上）"
    );
    assert_eq!(
        outcome.after.chain_heads, snapshot.before.chain_heads,
        "链头账本必须与迁移前逐字相等"
    );
}

#[test]
fn rollback_restores_the_append_only_guard() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");

    let _ = rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚");

    let trigger_count: i64 = fixture
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = 'episodes_no_delete'",
            [],
            |row| row.get(0),
        )
        .expect("查触发器");
    assert_eq!(
        trigger_count, 1,
        "append-only 删除触发器必须被复原（不许留裸库）"
    );

    let blocked = fixture
        .conn
        .execute("DELETE FROM episodes WHERE id = 'ep-a'", []);
    assert!(
        blocked.is_err(),
        "append-only 保护必须继续生效（删原记录必须失败）"
    );
}

#[test]
fn rollback_on_mismatch_removes_nothing() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");
    tamper_content(&fixture.conn, "mig-ep-a", "被改过的副本");
    let before_ids = fixture.ids();

    let outcome =
        rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚（判失败，不是崩）");

    assert_eq!(outcome.verdict, RollbackVerdict::Mismatch);
    assert!(
        outcome.removed_ids.is_empty(),
        "账实不符 ⇒ 一个副本都不许删"
    );
    assert!(
        !outcome.detail.is_empty(),
        "Mismatch 必须**说明差在哪**（不许含糊）"
    );
    assert_eq!(before_ids, fixture.ids(), "库里必须一条都没动");
}

#[test]
fn rollback_on_missing_source_removes_nothing() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    migrate_subject(&fixture.store, FROM, TO, CREATED_AT_MS).expect("真迁移");
    delete_record(&fixture.conn, "ep-b");
    let before_ids = fixture.ids();

    let outcome =
        rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚（判失败，不是崩）");

    assert_eq!(outcome.verdict, RollbackVerdict::SourceMissing);
    assert!(outcome.removed_ids.is_empty(), "原记录不在了 ⇒ 绝不许删");
    assert_eq!(before_ids, fixture.ids(), "库里必须一条都没动");
}

#[test]
fn rollback_refuses_to_delete_an_original_record() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, _ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    let before_ids = fixture.ids();

    // 造一个**恶意账**：把「副本承诺」指向原记录（主体也改成原记录的主体 ⇒ 核对会放行）。
    let evil = MigrationLedger {
        from: FROM.to_string(),
        to: SUBJECT.to_string(),
        executed_at_ms: CREATED_AT_MS,
        copied_ids: vec!["ep-a".to_string()],
    };

    let error = rollback_migration(&mut fixture.conn, &snapshot, &evil)
        .expect_err("删原记录必须被拒（P1 底线）");
    assert!(
        error.contains("P1") || error.contains("原记录"),
        "报错必须点明是原记录：{error}"
    );
    assert_eq!(before_ids, fixture.ids(), "被拒的回滚不许动任何一条记录");
}

#[test]
fn rollback_reports_nothing_to_rollback_when_no_migration_happened() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let (snapshot, ledger) = prepare_ledger(&fixture.conn, FROM, TO, CREATED_AT_MS).expect("建账");
    let before_ids = fixture.ids();

    let outcome = rollback_migration(&mut fixture.conn, &snapshot, &ledger).expect("回滚");

    assert_eq!(outcome.verdict, RollbackVerdict::NothingToRollback);
    assert!(outcome.removed_ids.is_empty());
    assert_eq!(before_ids, fixture.ids(), "没迁移过 ⇒ 库里一条都不许动");
}

// ============================================================
// drill_rollback：完整循环 + 机器可证的「回到原点」
// ============================================================

#[test]
fn drill_rollback_reports_back_to_origin() {
    let mut fixture = Fixture::new();
    fixture.seed();
    let before_digest = fixture.digest();

    let report = drill_rollback(&mut fixture.conn, FROM, TO, CREATED_AT_MS).expect("演练");

    assert!(
        report.contains("回到原点"),
        "报告必须给出「回到原点」的结论：\n{report}"
    );
    assert!(
        report.contains(&before_digest),
        "报告必须贴出**迁移前**的内容哈希：\n{report}"
    );
    assert!(
        report.contains("迁移后内容哈希"),
        "报告必须贴出**回滚前**（迁移后）的内容哈希：\n{report}"
    );
    assert!(
        report.contains("回滚后内容哈希"),
        "报告必须贴出**回滚后**的内容哈希：\n{report}"
    );
    assert_eq!(
        fixture.digest(),
        before_digest,
        "演练结束后，库必须真的回到迁移前"
    );
}

#[test]
fn drill_rollback_uses_the_real_migrate_subject() {
    let mut fixture = Fixture::new();
    fixture.seed();

    let report = drill_rollback(&mut fixture.conn, FROM, TO, CREATED_AT_MS).expect("演练");

    assert!(
        report.contains("continuity_link::migrate_subject"),
        "演练必须走**真实**的 migrate_subject 入口：\n{report}"
    );
    assert!(
        report.contains("真实入口"),
        "报告必须如实标注走的是真实入口：\n{report}"
    );
    assert!(
        report.contains("一条未动"),
        "报告必须如实说清原记录一条未动（P1）：\n{report}"
    );
}

#[test]
fn drill_rollback_on_in_memory_library_falls_back_to_the_same_write_path() {
    // 纯内存库**没有文件路径** ⇒ 没法在第二个连接上调用 `migrate_subject`
    // ⇒ 演练退到「同写路径等价实现」⇒ 报告必须**如实标注**（不许假装走了真实入口）。
    let store = SqliteMemoryStore::open_in_memory().expect("开内存库");
    for (id, ts, content) in [("ep-a", 100_i64, "事实一"), ("ep-b", 101_i64, "事实二")] {
        let episode = Episode {
            id: id.to_string(),
            timestamp: ts,
            role: "user".to_string(),
            content: content.to_string(),
            session_id: FROM.to_string(),
        };
        store
            .put_episode_for_subject(&episode, SUBJECT)
            .expect("写入原记录");
    }
    let mut guard = store.conn().expect("拿连接");

    let report = drill_rollback(&mut *guard, FROM, TO, CREATED_AT_MS).expect("演练");

    assert!(
        report.contains("同写路径等价实现"),
        "退化路径必须如实标注：\n{report}"
    );
    assert!(
        !report.contains("真实入口"),
        "退化路径不许自称真实入口：\n{report}"
    );
    assert!(
        report.contains("回到原点"),
        "退化路径也必须证明回到原点：\n{report}"
    );
}

#[test]
fn drill_rollback_leaves_no_copy_behind() {
    let mut fixture = Fixture::new();
    fixture.seed();

    let _ = drill_rollback(&mut fixture.conn, FROM, TO, CREATED_AT_MS).expect("演练");

    let ids = fixture.ids();
    assert!(
        !ids.iter().any(|id| id.starts_with(MIGRATED_ID_PREFIX)),
        "演练结束后不许留下任何 `mig-` 副本：{ids:?}"
    );
    assert_eq!(
        ids,
        vec!["ep-a".to_string(), "ep-b".to_string(), "ep-c".to_string()],
        "演练结束后只剩原记录"
    );
}

#[test]
fn drill_rollback_refuses_to_run_on_a_library_without_source_records() {
    let mut fixture = Fixture::new();
    // 不 seed ⇒ 源锚点下没有记录 ⇒ 没有可演练的东西。

    let error = drill_rollback(&mut fixture.conn, FROM, TO, CREATED_AT_MS)
        .expect_err("没有源记录 ⇒ 演练必须明说失败（不许假装跑过）");
    assert!(!error.is_empty(), "失败必须**如实**报错，不许返回空报告");
}

#[test]
fn drill_rollback_rejects_empty_anchor() {
    let mut fixture = Fixture::new();
    fixture.seed();
    assert!(drill_rollback(&mut fixture.conn, "", TO, CREATED_AT_MS).is_err());
    assert!(drill_rollback(&mut fixture.conn, FROM, FROM, CREATED_AT_MS).is_err());
}
