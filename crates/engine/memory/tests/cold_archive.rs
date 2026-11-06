//! `cold_archive` 的库级测试：**单条冷层归档**（归档 ≠ 删除）。
//!
//! # 这批测试在守什么
//!
//! 1. **不删数据**（P1）⇒ 归档前后 `episodes` 表**逐列逐行相等**（核心断言）；
//! 2. **不消失**（保真可达）⇒ 归档件**仍能被真检索召回**（只是权重降、排序靠后）；
//! 3. **账实相符可查** ⇒ 账本里的记录若在 `episodes` 里没了，**必须能查出来**；
//! 4. **不许 0 权重** ⇒ 0 = 过滤掉 ⇒ 违反「归档件不消失」⇒ 构造期就拒。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线；库级测试，非生产接线）

use apeireth_memory::cold_archive::{
    archive_episode, read_ledger, render_cold_report, unarchive_episode, verify_ledger, weight_for,
    ArchiveLedger, ColdArchiveConfig, COLD_ARCHIVE_TABLE, DEFAULT_WEIGHT_FACTOR,
};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, Connection};
use std::sync::MutexGuard;

// ============================================================
// 夹具
// ============================================================

/// 造一条记忆（**字段与生产写入路径一致** ⇒ 链/双时间列照常落库）。
fn episode(id: &str, timestamp: i64, content: &str) -> CoreEpisode {
    CoreEpisode {
        id: id.to_string(),
        timestamp,
        role: "user".to_string(),
        content: content.to_string(),
        session_id: "sess-cold".to_string(),
    }
}

/// 建一个装了 3 条记忆的库（真实迁移 ⇒ 真实 schema + append-only 触发器）。
fn seeded_store() -> SqliteMemoryStore {
    let store = SqliteMemoryStore::open_in_memory().expect("建库");
    store
        .put_episode(&episode("ep-a", 1_000, "第一条记忆：关于冷层归档的说明"))
        .expect("写 ep-a");
    store
        .put_episode(&episode("ep-b", 2_000, "第二条记忆：归档不等于删除"))
        .expect("写 ep-b");
    store
        .put_episode(&episode("ep-c", 3_000, "第三条记忆：保真可达的机器证明"))
        .expect("写 ep-c");
    store
}

/// 拿连接（store 必须活着 ⇒ 返回 guard 而不是裸引用）。
fn conn_of(store: &SqliteMemoryStore) -> MutexGuard<'_, Connection> {
    store.conn().expect("拿连接")
}

/// `ValueRef` ⇒ `Value`（**显式逐型转** ⇒ 不依赖 rusqlite 的 From 实现细节）。
fn value_of(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(inner) => Value::Integer(inner),
        ValueRef::Real(inner) => Value::Real(inner),
        ValueRef::Text(inner) => Value::Text(String::from_utf8_lossy(inner).into_owned()),
        ValueRef::Blob(inner) => Value::Blob(inner.to_vec()),
    }
}

/// **`episodes` 全表快照**（**所有列**、所有行 ⇒ 动态读列名，不假设列集）。
///
/// **为什么动态读列名**：`episodes` 的列集随迁移版本变（V4 加 4 列、V19 加 2 列）
/// ⇒ 写死列名就是在赌 schema（本仓吃过「SQL 假设了不存在的列」的亏）。
fn snapshot_episodes(conn: &Connection) -> Vec<Vec<Value>> {
    let mut statement = conn
        .prepare("SELECT * FROM episodes ORDER BY id ASC")
        .expect("准备快照 SQL");
    let column_count = statement.column_count();
    let rows = statement
        .query_map([], |row| {
            let mut cells = Vec::with_capacity(column_count);
            for index in 0..column_count {
                cells.push(value_of(row.get_ref(index)?));
            }
            Ok(cells)
        })
        .expect("读快照行");
    rows.map(|row| row.expect("快照行")).collect()
}

/// **真检索**（关键词命中 + 归档权重）⇒ 返回 `(id, 权重)`，按权重降序。
///
/// **为什么自己写一条小检索**：要证的是「归档件**仍被召回**」⇒ 判据必须落在
/// **真实检索结果**上（不许假设）⇒ 这里跑真 SQL + 真排序。
fn retrieve(
    conn: &Connection,
    keyword: &str,
    ledger: &ArchiveLedger,
    config: &ColdArchiveConfig,
) -> Vec<(String, f64)> {
    let mut statement = conn
        .prepare("SELECT id FROM episodes WHERE content LIKE ?1 ORDER BY timestamp ASC, id ASC")
        .expect("准备检索 SQL");
    let ids = statement
        .query_map(params![format!("%{keyword}%")], |row| {
            row.get::<_, String>(0)
        })
        .expect("执行检索 SQL")
        .map(|row| row.expect("检索行"))
        .collect::<Vec<String>>();
    let mut scored: Vec<(String, f64)> = ids
        .into_iter()
        .map(|id| {
            let weight = weight_for(ledger, &id, config);
            (id, weight)
        })
        .collect();
    // 权重降序 ⇒ 同权重按 id 升序（确定性）。
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .expect("权重有限")
            .then_with(|| left.0.cmp(&right.0))
    });
    scored
}

// ============================================================
// 配置：权重不许 0
// ============================================================

#[test]
fn config_rejects_zero_weight_factor() {
    let error = ColdArchiveConfig::new(0.0).expect_err("0 必须被拒");
    assert!(
        error.contains("必须 > 0"),
        "错误必须说明「不许 0」的理由：{error}"
    );
    assert!(
        error.contains("不消失"),
        "错误必须点明 0 = 过滤掉 = 违反「不消失」：{error}"
    );
}

#[test]
fn config_rejects_negative_and_non_finite_factors() {
    assert!(
        ColdArchiveConfig::new(-0.5).is_err(),
        "负系数 = 归档件反超未归档件 ⇒ 同罪 ⇒ 拒"
    );
    assert!(
        ColdArchiveConfig::new(f64::NAN).is_err(),
        "NaN 会让排序口径不可复核 ⇒ 拒"
    );
    assert!(
        ColdArchiveConfig::new(f64::INFINITY).is_err(),
        "Inf 同理 ⇒ 拒"
    );
}

#[test]
fn default_config_uses_the_documented_factor() {
    let config = ColdArchiveConfig::default_config();
    assert_eq!(config.weight_factor, DEFAULT_WEIGHT_FACTOR);
    assert_eq!(config.weight_factor, 0.2);
    assert!(
        config.weight_factor > 0.0,
        "默认权重必须 > 0（0 = 过滤掉 ⇒ 违反「不消失」）"
    );
    let ok = ColdArchiveConfig::new(0.2).expect("0.2 合法");
    assert_eq!(ok, config);
}

// ============================================================
// 归档：幂等 / 不许归档不存在的 / 空 id
// ============================================================

#[test]
fn archiving_an_existing_episode_succeeds() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    let archived = archive_episode(conn, "ep-a", "低频冷记忆", 1_700_000_000_000)
        .expect("归档真实存在的记录应当成功");
    assert!(archived, "首次归档 ⇒ Ok(true)");

    let ledger = read_ledger(conn).expect("读账本");
    assert_eq!(ledger.entries.len(), 1);
    assert_eq!(ledger.entries[0].id, "ep-a");
    assert_eq!(ledger.entries[0].archived_at_ms, 1_700_000_000_000);
    assert_eq!(ledger.entries[0].reason, "低频冷记忆");
    assert!(ledger.is_archived("ep-a"));
    assert!(!ledger.is_archived("ep-b"));
}

#[test]
fn archiving_twice_is_idempotent() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    assert!(archive_episode(conn, "ep-a", "第一次", 1).expect("首次归档"));
    let second = archive_episode(conn, "ep-a", "第二次（应当被忽略）", 2).expect("重复归档");
    assert!(!second, "重复归档 ⇒ Ok(false)（幂等）");

    let ledger = read_ledger(conn).expect("读账本");
    assert_eq!(ledger.entries.len(), 1, "账本不许重复记账");
    assert_eq!(
        ledger.entries[0].archived_at_ms, 1,
        "重复归档不许改写首次归档时刻（幂等 ⇒ 账本稳定）"
    );
}

#[test]
fn archiving_a_missing_episode_is_rejected() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    let error = archive_episode(conn, "ep-not-here", "归档不存在的记忆", 1)
        .expect_err("归档不存在的记录必须报错");
    assert!(error.contains("不存在"), "错误必须说明记录不存在：{error}");
    let ledger = read_ledger(conn).expect("读账本");
    assert!(
        ledger.entries.is_empty(),
        "失败的归档不许留下任何账（账实相符从写入侧就守）"
    );
}

#[test]
fn archiving_an_empty_id_is_rejected() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    for empty in ["", "   "] {
        let error = archive_episode(conn, empty, "空 id", 1).expect_err("空 id 必须报错");
        assert!(error.contains("id 为空"), "错误要说清是 id 空：{error}");
    }
}

// ============================================================
// 核心断言：数据没动（归档 ≠ 删除）
// ============================================================

#[test]
fn archiving_does_not_change_any_episode_row() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    let before = snapshot_episodes(conn);
    assert_eq!(before.len(), 3, "夹具应当有 3 条记忆");

    archive_episode(conn, "ep-b", "归档但不许删数据", 42).expect("归档");

    let after = snapshot_episodes(conn);
    assert_eq!(
        before, after,
        "归档前后 `episodes` 的**全部列逐字相等**（归档 ≠ 删除 ⇒ P1）"
    );
}

#[test]
fn unarchiving_also_does_not_change_any_episode_row() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-b", "先归档", 42).expect("归档");
    let before = snapshot_episodes(conn);
    unarchive_episode(conn, "ep-b").expect("取消归档");
    let after = snapshot_episodes(conn);
    assert_eq!(
        before, after,
        "取消归档同样不许动 `episodes`（归档是可逆的标签，不是数据操作）"
    );
}

// ============================================================
// 账本：自建表 / 有序 / 空账本
// ============================================================

#[test]
fn archiving_creates_its_own_table_without_touching_migrations() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    // ⚠️ 直接查簿记表（**不**走 `store.applied_migrations()`）：
    // 那个方法会再拿一次 store 的 `Mutex` ⇒ 而本测试**正拿着**它的 guard
    // ⇒ `std::sync::Mutex` 不可重入 ⇒ 直接死锁（本仓这类坑：守卫 + 再取锁）。
    let migrations_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("读迁移簿记条数（归档前）");

    archive_episode(conn, "ep-a", "自建表", 1).expect("归档");

    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![COLD_ARCHIVE_TABLE],
            |row| row.get(0),
        )
        .expect("查表是否存在");
    assert!(exists, "归档账本表必须自己建（不依赖迁移）");

    let migrations_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("读迁移簿记条数（归档后）");
    assert_eq!(
        migrations_before, migrations_after,
        "本模块**不许**改迁移簿记（迁移列表是 append-only 的历史账）"
    );
    assert!(migrations_before > 0, "夹具应当已跑过真实迁移（簿记非空）");
}

#[test]
fn ledger_is_empty_on_a_fresh_database() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    let ledger = read_ledger(conn).expect("读账本");
    assert!(ledger.entries.is_empty(), "新库 ⇒ 空账本");
    assert!(ledger.archived_ids().is_empty());
    assert!(!ledger.is_archived("ep-a"));
}

#[test]
fn reading_a_ledger_on_a_bare_connection_is_empty_not_an_error() {
    // 裸连接（没有 `episodes`、没有账本表）⇒ 读账本返回**空**（不是错）
    // ⇒ 且**只读**连接也能读（读路径不建表 ⇒ 不写库）。
    let conn = Connection::open_in_memory().expect("裸连接");
    let ledger = read_ledger(&conn).expect("裸库读账本");
    assert!(ledger.entries.is_empty());
}

#[test]
fn ledger_is_ordered_by_archived_time_then_id() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    // 故意乱序写入（时刻倒着来）⇒ 读出来必须按 (时刻, id) 升序。
    archive_episode(conn, "ep-c", "第三条", 2_000).expect("归档 c");
    archive_episode(conn, "ep-a", "第一条", 1_000).expect("归档 a");
    // 与 ep-a **同一时刻** ⇒ 平局按 id 升序（确定性）。
    archive_episode(conn, "ep-b", "第二条", 1_000).expect("归档 b");

    let ledger = read_ledger(conn).expect("读账本");
    assert_eq!(
        ledger.archived_ids(),
        vec!["ep-a".to_string(), "ep-b".to_string(), "ep-c".to_string()],
        "账本按 (归档时刻, id) 升序 ⇒ 同库两次读逐字相等"
    );
    assert_eq!(ledger.entries[0].id, "ep-a");
    assert_eq!(ledger.entries[2].id, "ep-c");
}

// ============================================================
// 取消归档：可逆
// ============================================================

#[test]
fn unarchiving_removes_the_entry_from_the_ledger() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "先归档", 1).expect("归档");
    assert!(read_ledger(conn).expect("读账本").is_archived("ep-a"));

    let removed = unarchive_episode(conn, "ep-a").expect("取消归档");
    assert!(removed, "取消已归档的 ⇒ Ok(true)");

    let ledger = read_ledger(conn).expect("读账本");
    assert!(!ledger.is_archived("ep-a"), "账本里不该再有它");
    assert!(ledger.entries.is_empty());
}

#[test]
fn unarchieving_an_unarchived_episode_reports_false() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    let removed = unarchive_episode(conn, "ep-a").expect("取消未归档的");
    assert!(!removed, "没归档过 ⇒ 无可取消 ⇒ Ok(false)");

    let empty = unarchive_episode(conn, "   ").expect_err("空 id ⇒ Err");
    assert!(empty.contains("id 为空"), "错误要说清是 id 空：{empty}");
}

// ============================================================
// 独立校验：账实相符
// ============================================================

#[test]
fn verify_ledger_passes_when_the_ledger_matches_the_data() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    archive_episode(conn, "ep-c", "归档", 2).expect("归档");
    verify_ledger(conn).expect("账实相符 ⇒ Ok");
}

#[test]
fn verify_ledger_detects_a_deleted_episode() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    verify_ledger(conn).expect("删之前相符");

    // ⚠️ 模拟「绕过 append-only 触发器的数据丢失」：`episodes` 上的
    // `episodes_no_delete` 触发器平时会拦下 DELETE ⇒ 本测试**故意**先把它摘掉
    // ⇒ 制造出「账本指着一条已经不在的记忆」的异常态 ⇒ 校验必须抓得住。
    conn.execute("DROP TRIGGER episodes_no_delete", [])
        .expect("摘掉 append-only 触发器（模拟外部数据丢失）");
    conn.execute("DELETE FROM episodes WHERE id = ?1", params!["ep-a"])
        .expect("制造数据丢失");

    let error = verify_ledger(conn).expect_err("账实不符必须报错");
    assert!(
        error.contains("ep-a"),
        "错误必须指名是哪一条对不上：{error}"
    );
    assert!(
        error.contains("账实不符"),
        "错误必须说清是账实不符：{error}"
    );
}

#[test]
fn verify_ledger_detects_a_missing_episodes_table() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    conn.execute("DROP TABLE episodes", [])
        .expect("删掉 episodes 表（模拟原始证据整体丢失）");

    let error = verify_ledger(conn).expect_err("表没了必须报错");
    assert!(
        error.contains("账实不符"),
        "错误必须说清是账实不符：{error}"
    );
}

// ============================================================
// 权重：不归档 = 1.0；归档 = weight_factor（**不是 0**）
// ============================================================

#[test]
fn weight_is_one_when_not_archived_and_the_factor_when_archived() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;
    let config = ColdArchiveConfig::default_config();

    let ledger = read_ledger(conn).expect("读账本");
    assert_eq!(
        weight_for(&ledger, "ep-a", &config),
        1.0,
        "未归档 ⇒ 权重原样 1.0"
    );

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    let ledger = read_ledger(conn).expect("读账本");
    let weight = weight_for(&ledger, "ep-a", &config);
    assert_eq!(weight, config.weight_factor);
    assert!(
        weight > 0.0,
        "归档件权重必须 > 0（0 = 过滤掉 ⇒ 违反「不消失」）"
    );
    assert_eq!(weight_for(&ledger, "ep-b", &config), 1.0);
}

#[test]
fn unarchiving_restores_the_full_weight() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;
    let config = ColdArchiveConfig::default_config();

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    let ledger = read_ledger(conn).expect("读账本");
    assert!(weight_for(&ledger, "ep-a", &config) < 1.0);

    unarchive_episode(conn, "ep-a").expect("取消归档");
    let ledger = read_ledger(conn).expect("读账本");
    assert_eq!(
        weight_for(&ledger, "ep-a", &config),
        1.0,
        "取消归档 ⇒ 权重回到原样（归档可逆）"
    );
}

// ============================================================
// ⭐ 保真可达：归档件**仍能检索到**（真检索 ⇒ 不许假设）
// ============================================================

#[test]
fn archived_episode_is_still_retrieved_by_a_real_query() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;
    let config = ColdArchiveConfig::default_config();

    // 归档前：真检索能查到「记忆」这一族三条。
    let before = retrieve(conn, "记忆", &read_ledger(conn).expect("读账本"), &config);
    assert_eq!(before.len(), 3, "归档前三条都该被检索到");
    assert!(before.iter().all(|(_, weight)| *weight == 1.0));

    // 归档其中一条（ep-b）。
    archive_episode(conn, "ep-b", "归档但不许消失", 1).expect("归档");

    let ledger = read_ledger(conn).expect("读账本");
    let after = retrieve(conn, "记忆", &ledger, &config);

    let ids: Vec<&str> = after.iter().map(|(id, _)| id.as_str()).collect();
    assert!(
        ids.contains(&"ep-b"),
        "⭐ 归档件**仍能检索到**（归档 ≠ 过滤掉）：{ids:?}"
    );
    assert_eq!(after.len(), 3, "归档不许让任何一条从检索结果里消失");
    assert_eq!(
        ids.last().copied(),
        Some("ep-b"),
        "归档件权重降 ⇒ 排序靠后（但**仍在结果里**）：{ids:?}"
    );
    let archived_weight = after
        .iter()
        .find(|(id, _)| id == "ep-b")
        .map(|(_, weight)| *weight)
        .expect("归档件在结果里");
    assert!(
        archived_weight > 0.0,
        "归档件权重必须 > 0 ⇒ 明确需要时仍会被召回"
    );
}

// ============================================================
// 人话报告
// ============================================================

#[test]
fn render_cold_report_contains_entry_count_and_verdict() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "低频冷记忆", 1_700_000_000_000).expect("归档");
    archive_episode(conn, "ep-c", "重复内容", 1_700_000_001_000).expect("归档");

    let ledger = read_ledger(conn).expect("读账本");
    let report = render_cold_report(&ledger, verify_ledger(conn));
    assert!(
        report.contains("归档条目数：2"),
        "报告必须给出条目数：\n{report}"
    );
    assert!(report.contains("相符"), "报告必须给出校验结论：\n{report}");
    assert!(
        report.contains("ep-a") && report.contains("低频冷记忆"),
        "报告必须逐条列出（含原因 ⇒ 留痕）：\n{report}"
    );
    assert!(
        report.contains("归档 ≠ 删除"),
        "报告必须带上「归档 ≠ 删除」的口径提醒：\n{report}"
    );
    assert!(
        report.contains("NOT IMPLEMENTED"),
        "报告必须如实标出状态（新增未接线）：\n{report}"
    );
}

#[test]
fn render_cold_report_shows_the_mismatch_reason() {
    let store = seeded_store();
    let guard = conn_of(&store);
    let conn: &Connection = &guard;

    archive_episode(conn, "ep-a", "归档", 1).expect("归档");
    conn.execute("DROP TRIGGER episodes_no_delete", [])
        .expect("摘掉触发器");
    conn.execute("DELETE FROM episodes WHERE id = ?1", params!["ep-a"])
        .expect("制造数据丢失");

    let ledger = read_ledger(conn).expect("读账本");
    let report = render_cold_report(&ledger, verify_ledger(conn));
    assert!(
        report.contains("不符"),
        "报告必须照实报「不符」：\n{report}"
    );
    assert!(
        report.contains("ep-a"),
        "报告必须点名对不上的那一条：\n{report}"
    );
}
