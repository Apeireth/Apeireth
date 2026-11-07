//! 能力接线审计（`capability_audit`）的测试：**真读库** + **不许粉饰**。
//!
//! # 钉住四条纪律（每条都能被「删掉实现」证伪）
//!
//! 1. **覆盖完整**：19 个能力探针一个不许少（少一个 ⇒ 那个能力就从「被审计」
//!    变成「没人看」⇒ 可达率闸门会立刻回到缺口）；
//! 2. **0 条不许说成已接线**：空库上 `ARMED` 必须是 **0**（只允许 `IDLE`）
//!    ⇒ ⇒ 否则审计本身就在粉饰；
//! 3. **有数据就要真的 ARMED**：写一条真记录后，写侧/读侧能力必须从 `IDLE`
//!    翻成 `ARMED`（翻不过去 = 探针没读到真数据 ⇒ 假审计）；
//! 4. **只读 + 确定性**：跑两次逐字相等，且库内行数不变。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::capability_audit::{audit, ProbeState};
use apeireth_memory::handover::render_capability_audit;
use apeireth_memory::SqliteMemoryStore;
use std::path::PathBuf;

/// 审计必须覆盖的能力（与 `capability_audit::audit` 的探针一一对应）。
const EXPECTED_MODULES: [&str; 16] = [
    "agent_trace",
    "arbitration",
    "calibration_critic",
    "cold_archive",
    "confidence",
    "daily_summary",
    "derived_repair",
    "ensemble",
    "forget_coordinator",
    "memory_view",
    "metadata_filter",
    "offline_embed",
    "offline_embedding_provider",
    "session_lifecycle",
    "swap_flow",
    "vector_rebuild",
];

/// 库里的记录时刻（注入 ⇒ 审计摘要的「今日」边界可复现）。
const EPISODE_TS: i64 = 1_760_000_000_000;

/// 开一个**有文件路径**的空库（审计要用 `conn.path()` 重开 store ⇒ 内存库拿不到路径）。
fn fresh_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = dir.path().join("cognitive.sqlite3");
    let _store = SqliteMemoryStore::open(&path).expect("建库");
    (dir, path)
}

/// 往库里写一条**真实**记录（走生产写路径 ⇒ 自动入链）。
fn put(path: &PathBuf, id: &str, subject: &str, ts: i64, content: &str) {
    let store = SqliteMemoryStore::open(path).expect("重开库");
    let episode = Episode {
        id: id.to_string(),
        timestamp: ts,
        role: "user".to_string(),
        content: content.to_string(),
        session_id: "sess-audit".to_string(),
    };
    store
        .put_episode_for_subject(&episode, subject)
        .expect("写记录");
}

fn episode_count(path: &PathBuf) -> i64 {
    let store = SqliteMemoryStore::open(path).expect("重开库");
    let conn = store.conn().expect("连接");
    conn.query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .expect("数条数")
}

#[test]
fn audit_covers_every_capability_module_exactly_once() {
    let (_dir, path) = fresh_db();
    let store = SqliteMemoryStore::open(&path).expect("库");
    let conn = store.conn().expect("连接");
    let report = audit(&conn, EPISODE_TS);

    let mut got: Vec<&str> = report.probes.iter().map(|p| p.module).collect();
    got.sort_unstable();
    let mut expected: Vec<&str> = EXPECTED_MODULES.to_vec();
    expected.sort_unstable();
    assert_eq!(got, expected, "审计必须逐能力各一条（少/多都不行）");

    // 输出确定性：同一库同一时刻 ⇒ 逐字相等。
    let again = audit(&conn, EPISODE_TS).render();
    assert_eq!(report.render(), again, "审计输出必须可复现");
}

#[test]
fn an_empty_store_is_never_reported_as_armed() {
    let (_dir, path) = fresh_db();
    let store = SqliteMemoryStore::open(&path).expect("库");
    let conn = store.conn().expect("连接");
    let report = audit(&conn, EPISODE_TS);
    let (armed, idle, unavailable) = report.counts();
    assert_eq!(
        armed, 0,
        "**空库上不许出现 ARMED**（0 条不许说成已接线）：{:#?}",
        report.probes
    );
    assert_eq!(
        unavailable, 0,
        "有文件路径的库上探针不许失败：{:#?}",
        report.probes
    );
    assert_eq!(idle, report.probes.len(), "空库只能是 IDLE");
}

#[test]
fn a_written_episode_actually_arms_the_read_side_capabilities() {
    let (_dir, path) = fresh_db();
    put(
        &path,
        "ep-audit-1",
        "cid-audit",
        EPISODE_TS,
        "审计用的一段真实正文",
    );
    let store = SqliteMemoryStore::open(&path).expect("库");
    let conn = store.conn().expect("连接");
    let report = audit(&conn, EPISODE_TS);

    let state = |module: &str| {
        report
            .probes
            .iter()
            .find(|p| p.module == module)
            .unwrap_or_else(|| panic!("缺探针 {module}"))
            .state
    };
    for module in [
        "arbitration",
        "daily_summary",
        "memory_view",
        "metadata_filter",
        "offline_embed",
    ] {
        assert_eq!(
            state(module),
            ProbeState::Armed,
            "{module} 有真记录却仍是 IDLE ⇒ 探针没读到真数据（假审计）：{:#?}",
            report.probes
        );
    }
    // **如实**：没有承诺表数据 ⇒ 这两个必须还是 IDLE（不许因为「想好看」而 ARMED）。
    for module in ["calibration_critic", "confidence"] {
        assert_eq!(
            state(module),
            ProbeState::Idle,
            "{module} 没有输入，不许 ARMED"
        );
    }
}

#[test]
fn render_carries_the_honesty_note_and_is_read_only() {
    let (_dir, path) = fresh_db();
    put(&path, "ep-audit-2", "cid-audit", EPISODE_TS, "只读性检查");
    let (before, after, first, second) = {
        let store = SqliteMemoryStore::open(&path).expect("库");
        let conn = store.conn().expect("连接");
        let before = episode_count(&path);
        let first = render_capability_audit(&conn, EPISODE_TS);
        let second = render_capability_audit(&conn, EPISODE_TS);
        drop(conn);
        let after = episode_count(&path);
        (before, after, first, second)
    };
    assert_eq!(first, second, "两次审计必须逐字相等（确定性）");
    assert_eq!(before, after, "审计**不许写库**（行数必须不变）");
    for needle in [
        "能力接线审计",
        "ARMED",
        "IDLE",
        "UNAVAILABLE",
        "= 能力可用但本库/本部署无输入（≠ 未接电）",
    ] {
        assert!(first.contains(needle), "渲染缺「{needle}」：\n{first}");
    }
    for module in EXPECTED_MODULES {
        assert!(first.contains(module), "渲染缺能力行 {module}：\n{first}");
    }
}
