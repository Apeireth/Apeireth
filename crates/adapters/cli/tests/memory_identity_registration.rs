//! `apeireth memory migrate-subject` 的**主体登记**接线测试 —— `identity_cards` 不再永远是空表。
//!
//! # 这个接线解决什么缺口
//!
//! 生产里 `identity_cards` 表**永远是空的** ⇒ ⇒ `ensure_identity` /
//! `record_carrier_migration` **零生产调用** ⇒ ⇒ `continuity_id` 的**物理唯一性**
//! （`identity_cards` 表的 UNIQUE 约束）**从未生效**。
//!
//! # 为什么「主体迁移」是真消费点（不是「凑数」）
//!
//! `apeireth memory migrate-subject` 是**生产面唯一**会把记录挂到**新主体**
//! 名下的入口 ⇒ ⇒ 迁移产出的副本带 `continuity_id = to`
//! ⇒ ⇒ 那个主体必须在 `identity_cards` 里**在册**，否则：
//!
//! * 物理唯一性约束永远不生效（表是空的 ⇒ 约束不会触发）；
//! * 迁移完的主体是「黑户」⇒ `record_carrier_migration` 直接报 `NotFound`。
//!
//! # 本文件钉死的行为差异
//!
//! * **接线前**：`identity_cards` **0 行**；`record_carrier_migration` 报 `NotFound`；
//! * **接线后**：`identity_cards` **2 行**（源主体 + 目标主体）；载体迁移**成功**；
//!   再登记一次同一 `continuity_id` ⇒ `AlreadyExists`（**物理唯一性真的生效**）。

use std::path::Path;
use std::process::Command;

use apeireth_core::kernel::memory::{Episode, IdentityCard};
use apeireth_memory::{EpisodeStore, IdentityCardStore, SqliteMemoryStore};

/// 源锚点。
const FROM: &str = "identity-anchor-a";

/// 目标锚点（迁移后记录挂在这个主体名下）。
const TO: &str = "identity-anchor-b";

/// 真实 CLI 二进制路径（与同目录其它 CLI 测试同一口径）。
fn bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("current test binary path");
    path.pop(); // 退出测试可执行文件
    if path.ends_with("deps") {
        path.pop(); // 退出 deps 目录 → target/debug
    }
    path.push(if cfg!(windows) {
        "apeireth.exe"
    } else {
        "apeireth"
    });
    path
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// 造一个文件库并写入 2 条原记录（挂在**源**会话锚点下）。
fn seed_database(db_path: &Path) {
    let store = SqliteMemoryStore::open(db_path).expect("开库");
    for (id, ts, content) in [("ep-a", 100_i64, "事实一"), ("ep-b", 101_i64, "事实二")] {
        store
            .put_episode_for_subject(
                &Episode {
                    id: id.to_string(),
                    timestamp: ts,
                    role: "user".to_string(),
                    content: content.to_string(),
                    session_id: FROM.to_string(),
                },
                "identity-subject-x",
            )
            .expect("写入原记录");
    }
}

/// 库里 `identity_cards` 的 `continuity_id`（**有序**）。
fn registered_subjects(db_path: &Path) -> Vec<String> {
    let store = SqliteMemoryStore::open(db_path).expect("开库");
    let conn = store.conn().expect("拿连接");
    let mut stmt = conn
        .prepare("SELECT continuity_id FROM identity_cards ORDER BY continuity_id ASC")
        .expect("准备查询");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("执行查询");
    let mut out = Vec::new();
    for row in rows {
        out.push(row.expect("读一行"));
    }
    out
}

/// ⭐ **接线前不可用 / 接线后可用**：跑一次**真实 CLI** 的主体迁移
/// ⇒ ⇒ `identity_cards` 从 **0 行**变成 **2 行**（源主体 + 目标主体都在册）。
#[test]
fn migrate_subject_registers_both_subjects_in_identity_cards() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);

    // **接线前**：表是空的（这正是要修的缺口）⇒ 先把「接线前」的事实钉住。
    assert!(
        registered_subjects(&db_path).is_empty(),
        "接线前 `identity_cards` 应该是空的（缺口本体）"
    );

    let bin = bin_path();
    assert!(bin.exists(), "CLI 二进制应存在：{}", bin.display());
    let output = Command::new(&bin)
        .args(args(&[
            "memory",
            "migrate-subject",
            "--from",
            FROM,
            "--to",
            TO,
            "--db",
            &db_path.display().to_string(),
        ]))
        .output()
        .expect("执行 CLI");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "迁移应成功\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("主体已登记"),
        "登记必须**可观测**（不许静默写库）：\n{stdout}"
    );

    assert_eq!(
        registered_subjects(&db_path),
        vec![FROM.to_string(), TO.to_string()],
        "**接线后**源主体与目标主体都必须在 `identity_cards` 在册"
    );
}

/// ⭐ **物理唯一性真的生效**：主体在册之后 ⇒ 再登记同一个 `continuity_id`
/// ⇒ ⇒ 被 UNIQUE 约束挡下（`AlreadyExists`）⇒ ⇒ 「物理唯一性从未生效」这个缺口被补上。
#[test]
fn identity_physical_uniqueness_is_enforced_after_registration() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);

    apeireth_cli::memory_migrate_command::run_migrate(&args(&[
        "--from",
        FROM,
        "--to",
        TO,
        "--db",
        &db_path.display().to_string(),
    ]));
    assert_eq!(
        registered_subjects(&db_path),
        vec![FROM.to_string(), TO.to_string()],
        "迁移后两个主体都应在册"
    );

    // 再登记一次同一个主体 ⇒ **必须**被 UNIQUE 约束挡下。
    let store = SqliteMemoryStore::open(&db_path).expect("开库");
    let err = <SqliteMemoryStore as IdentityCardStore>::create(
        &store,
        &IdentityCard {
            continuity_id: TO.to_string(),
            birth_time: 1,
            carriers: vec!["another-carrier".to_string()],
            migration_history: Vec::new(),
        },
    )
    .expect_err("同一 continuity_id 不许登记两次");
    assert!(
        err.to_string().contains("already exists"),
        "期望 UNIQUE 约束的 AlreadyExists，实际：{err}"
    );
}

/// ⭐ **迁移完的主体不再是「黑户」**：载体迁移（`record_carrier_migration`）
/// 需要那张卡存在 ⇒ ⇒ 接线前报 `NotFound`，接线后成功。
#[test]
fn migrated_subject_accepts_carrier_migration_after_registration() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);

    apeireth_cli::memory_migrate_command::run_migrate(&args(&[
        "--from",
        FROM,
        "--to",
        TO,
        "--db",
        &db_path.display().to_string(),
    ]));

    let store = SqliteMemoryStore::open(&db_path).expect("开库");
    apeireth_memory::continuity_link::record_carrier_migration(
        &store,
        TO,
        "disk-a",
        "disk-b",
        1_700_000_500,
    )
    .expect("迁移后的主体必须能记录载体迁移（接线前是 NotFound）");
    let card = <SqliteMemoryStore as IdentityCardStore>::get(&store, TO)
        .expect("读卡")
        .expect("卡必须存在");
    assert!(
        card.carriers.iter().any(|carrier| carrier == "disk-b"),
        "载体迁移必须落到卡上：{:?}",
        card.carriers
    );
    assert_eq!(card.migration_history.len(), 1);
}

/// **幂等**：重复迁移（含重复登记）⇒ 不许报错、不许重复建卡。
#[test]
fn registration_is_idempotent_across_repeated_migrations() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);
    let cli_args = args(&[
        "--from",
        FROM,
        "--to",
        TO,
        "--db",
        &db_path.display().to_string(),
    ]);

    assert_eq!(
        apeireth_cli::memory_migrate_command::run_migrate(&cli_args),
        0,
        "第一次迁移应成功"
    );
    assert_eq!(
        apeireth_cli::memory_migrate_command::run_migrate(&cli_args),
        0,
        "第二次迁移应成功（**幂等** ⇒ 登记不许把 AlreadyExists 冒泡成失败）"
    );
    assert_eq!(
        registered_subjects(&db_path),
        vec![FROM.to_string(), TO.to_string()],
        "重复登记不许多建卡"
    );
}
