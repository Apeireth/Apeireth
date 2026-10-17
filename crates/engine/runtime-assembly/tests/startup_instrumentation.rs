//! B-P1 冷启动冲刺 · 启动路径测试证据:
//! - 分段计时可测 (计时器注入): 脚本计时器下逐段耗时精确断言 + 接缝分段落账;
//! - 迁移短路: 已迁移库不再跑迁移 (黑盒断言: 删表后重开不被重建);
//! - 池复用: 同路径重复打开复用同一组底层句柄 (句柄数断言在 storage 套件)。

use apeireth_core::kernel::{system_clock, SessionId};
use apeireth_memory::SqliteMemoryStore;
use apeireth_runtime::canonical::{Session, SessionStore};
use apeireth_runtime_assembly::startup::{ScriptedTimer, StartupPhase, StartupTimeline};
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, JudgeConfig, ModuleFamily, ProductionBackends,
    ProductionModules, ProductionModulesConfig, SqliteSessionStore,
};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-startup-instrument-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[tokio::test]
async fn session_store_open_records_pool_and_migration_segments() {
    let dir = temp_dir("segments");
    let path = dir.join("sessions.sqlite3");
    let timer = ScriptedTimer::new();
    let timeline = StartupTimeline::new(timer.clone());

    let store = SqliteSessionStore::open_with_timeline(&path, Some(&timeline))
        .await
        .expect("open session store with timeline");

    // 分段落账: 连接池建立段 + 迁移段, 首尾相接、顺序固定。
    let spans = timeline.spans();
    assert_eq!(spans.len(), 2, "会话库打开应落两段账: 池建立 + 迁移");
    assert_eq!(spans[0].phase, StartupPhase::SessionPoolOpen);
    assert_eq!(spans[1].phase, StartupPhase::SessionMigrations);

    // 打开语义不变: 存储可用。
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    store.save(&session).await.expect("save session");
    let loaded = store.load(&session.id).await.expect("load session");
    assert!(loaded.is_some(), "分段计时不得破坏存储语义");

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn injected_timer_makes_segment_durations_exactly_assertable_through_the_seam() {
    // 计时器注入的另一面: 同一接缝换脚本计时器, 两段都读到确定读数 (游标不推进)。
    let dir = temp_dir("segments-scripted");
    let path = dir.join("sessions.sqlite3");
    let timer = ScriptedTimer::new();
    timer.set_ns(1_000);
    let timeline = StartupTimeline::new(timer.clone());

    let store = SqliteSessionStore::open_with_timeline(&path, Some(&timeline))
        .await
        .expect("open with scripted timer");
    drop(store);

    let spans = timeline.spans();
    assert_eq!(spans.len(), 2);
    // 脚本计时器读数全等 → 两段耗时确定为 0, 证明读数来自**注入**计时器。
    assert_eq!(spans[0].duration_ns, 0);
    assert_eq!(spans[1].duration_ns, 0);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn session_store_open_without_timeline_stays_behaviorally_identical() {
    let dir = temp_dir("no-timeline");
    let path = dir.join("sessions.sqlite3");

    let store = SqliteSessionStore::open(&path).await.expect("open");
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    store.save(&session).await.expect("save");
    assert!(store.load(&session.id).await.expect("load").is_some());
    assert!(store.contains(&session.id).await.expect("contains"));
    assert!(store.delete(&session.id).await.expect("delete"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migrated_memory_db_reopens_without_rerunning_migration_ddl() {
    // 迁移短路 (已迁移库不再跑迁移) 的黑盒断言: 首开跑完迁移后, 删掉一张
    // 迁移建出的表再重开 —— 若迁移真的不再跑, 该表**不会**被重建
    // (全量路径的 CREATE TABLE IF NOT EXISTS 会把它建回来)。
    let dir = temp_dir("memory-short-circuit");
    let path = dir.join("cognitive.sqlite3");

    drop(SqliteMemoryStore::open(&path).expect("首开: 迁移全量跑"));
    {
        let conn = rusqlite::Connection::open(&path).expect("raw open");
        conn.execute_batch("DROP TABLE notes;").expect("drop notes");
    }
    drop(SqliteMemoryStore::open(&path).expect("重开: 已迁移库短路"));

    let conn = rusqlite::Connection::open(&path).expect("raw reopen");
    let notes_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'notes')",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert!(
        !notes_exists,
        "已迁移库重开不得再跑迁移 DDL (notes 删后被重建 = 迁移又跑了)"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fresh_memory_db_still_runs_migrations_and_installs_schema() {
    // 短路不得吞掉真迁移: 全新库照常建表 (反向断言, 与上一测成对)。
    let dir = temp_dir("memory-migrates");
    let path = dir.join("cognitive.sqlite3");

    drop(SqliteMemoryStore::open(&path).expect("首开"));

    let conn = rusqlite::Connection::open(&path).expect("raw open");
    for table in ["notes", "episodes", "schema_migrations"] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .expect("query sqlite_master");
        assert!(exists, "全新库迁移必须建出 {table}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn session_store_reopens_share_one_pool_handle_set() {
    // 池复用的组合根口径: 同路径重复打开的会话存储共享同一组底层句柄
    // (句柄数断言在 storage 套件; 本测钉共享口径的读写一致性)。
    let dir = temp_dir("pool-reuse");
    let path = dir.join("sessions.sqlite3");

    let first = SqliteSessionStore::open(&path).await.expect("first open");
    let second = SqliteSessionStore::open(&path).await.expect("second open");

    // 同一把池: 经一个存储写, 另一个立刻可见 (同库同句柄组)。
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    first.save(&session).await.expect("save via first");
    assert!(
        second
            .load(&session.id)
            .await
            .expect("load via second")
            .is_some(),
        "共享池下两个存储句柄读写同一账本"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---- 懒加载语义 (未启用模块零构造断言, B-P1) ----

/// 全开关关的配置: 默认关族与默认开族一律显式关。
fn all_switches_off() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        judge: JudgeConfig {
            enabled: false,
            ..JudgeConfig::default()
        },
        filesystem: false,
        search: false,
        repo: false,
        self_status: false,
        ..ProductionModulesConfig::default()
    }
}

#[test]
fn default_off_module_families_are_never_constructed() {
    // 懒加载语义钉死: 开关全关时, **零个**模块族走到构造点 (探针零上报),
    // 装配结果也零模块零能力 (未启用 = 不构造 = 不注册)。
    let probe = CountingBuildProbe::new();
    let instrument = BuildInstrumentation::default().with_probe(probe.clone());

    let modules = ProductionModules::build_instrumented(
        all_switches_off(),
        ProductionBackends::default(),
        system_clock(),
        &instrument,
    )
    .expect("全关配置必须能装配");

    assert_eq!(
        probe.total(),
        0,
        "未启用模块族必须零构造 (探针有任何上报 = 懒加载被绕开)"
    );
    assert!(modules.ids().is_empty(), "未启用 = 不注册");
    assert!(modules.capabilities().is_empty(), "未启用 = 不注册");
}

#[test]
fn enabled_families_construct_exactly_once_while_disabled_families_stay_zero() {
    // 开关开 = 恰好构造一次; 未开族仍零构造 (逐族计数断言)。
    let probe = CountingBuildProbe::new();
    let instrument = BuildInstrumentation::default().with_probe(probe.clone());

    let config = ProductionModulesConfig {
        education: true,
        organs: true,
        absorption_insight: true,
        judge: JudgeConfig {
            enabled: true,
            ..JudgeConfig::default()
        },
        shell: Some(apeireth_tools_canonical::TrustedShellConfig::new(
            std::env::temp_dir(),
        )),
        fetch: Some(apeireth_tools_canonical::FetchConfig::public_internet_only()),
        ..all_switches_off()
    };
    ProductionModules::build_instrumented(
        config,
        ProductionBackends::default(),
        system_clock(),
        &instrument,
    )
    .expect("开启的族必须装配成功");

    for family in [
        ModuleFamily::Education,
        ModuleFamily::Organs,
        ModuleFamily::AbsorptionInsight,
        ModuleFamily::Judge,
        ModuleFamily::Shell,
        ModuleFamily::Fetch,
    ] {
        assert_eq!(probe.count(family), 1, "{family:?} 开关开 = 恰好构造一次");
    }
    for family in [
        ModuleFamily::MemoryRecall,
        ModuleFamily::MemoryWriteback,
        ModuleFamily::Council,
        ModuleFamily::Mcp,
        ModuleFamily::TaskBoard,
        ModuleFamily::ContentFeed,
        ModuleFamily::CareSchedule,
        ModuleFamily::BackupExport,
        ModuleFamily::VoiceSpeak,
        ModuleFamily::ExposureAudit,
        ModuleFamily::Reflexion,
        ModuleFamily::PartnerBond,
    ] {
        assert_eq!(probe.count(family), 0, "{family:?} 未启用 = 零构造");
    }
}

#[test]
fn modules_build_segment_is_recorded_by_the_injected_timeline() {
    // 分段计时可测 (计时器注入): 模块构造段用脚本计时器精确断言。
    let timer = ScriptedTimer::new();
    let timeline = StartupTimeline::new(timer.clone());
    let instrument = BuildInstrumentation::default().with_timeline(timeline.clone());

    ProductionModules::build_instrumented(
        all_switches_off(),
        ProductionBackends::default(),
        system_clock(),
        &instrument,
    )
    .expect("装配成功");

    let spans = timeline.spans();
    assert_eq!(spans.len(), 1, "模块构造段恰落一笔账");
    assert_eq!(spans[0].phase, StartupPhase::ModulesBuild);
    assert_eq!(
        spans[0].duration_ns, 0,
        "脚本计时器读数全等 → 段耗时确定为 0"
    );
    assert_eq!(timeline.duration_ns(StartupPhase::ModulesBuild), 0);
}
