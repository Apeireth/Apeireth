//! 生产路径 (memory 首开) 的 SQLITE_BUSY 有界退避重试: 确定性取证。
//!
//! **被钉的根因** (上一批只在**测试侧**重试, 生产路径仍会直接失败):
//! - `run_migrations` 用 `BEGIN IMMEDIATE` 从一开始就抢写锁
//!   (`crates/engine/memory/src/migrations.rs`), 赢方在**同一个事务**里装订整套
//!   schema (`FRESH_TABLES_SQL` + `INIT_SQL` + 触发器 + V2..V19 的 DDL)
//!   ⇒ 写锁一直持有到 `tx.commit()`。
//! - 输方 (并发首开) 的全部预算 = 连接既有的 `busy_timeout` (5s,
//!   `crates/engine/memory/src/lib.rs:601`) ⇒ 赢方耗时 > 预算时, 输方拿到
//!   `SQLITE_BUSY` ⇒ 旧写法**直接失败** (上一批的测试侧重试只是「替生产扛」)。
//! - 本批把等待策略**下沉到生产侧**: 重试发生在 `BEGIN` 处 ⇒ **事务体一次都
//!   不重复执行** ⇒ 无重复装订、无重复插版本簿记。
//!
//! **确定性手法** (不靠时间窗口赌概率):
//! - 外部写者用**另一个** `rusqlite::Connection` 对同一个库文件显式
//!   `BEGIN IMMEDIATE` 并**持住写锁**;
//! - 持锁方拿到写锁后**先发信号**, 主线程收到信号才发起生产调用 ⇒ 「对方已持锁」
//!   是**前置条件**而不是概率事件;
//! - 放锁时机由测试**显式控制** (发信号) 或由显式持锁时长决定 ⇒ 两个场景都可复现。
//!
//! **场景**:
//! - ① 对方持锁 < 预算 ⇒ 生产调用**应当成功** (旧写法必失败);
//! - ② 对方持锁 > 预算 ⇒ 生产调用**应当失败**, 且**耗时有界** (不是无限挂起),
//!   且错误**仍是 busy 类** (错误码不变) + 看得出「等过、仍被锁」。

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use apeireth_memory::{run_migrations, MemoryError, MIGRATIONS};
use rusqlite::Connection;

/// 生产配置 (`SqliteMemoryStore::configure`): WAL + `busy_timeout` 5s。
const PRODUCTION_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
/// 场景 ①: 对方持锁时长 —— 必须**超过**既有 `busy_timeout` (5s) 才能证伪旧写法
/// (旧写法在 5s 处就 `SQLITE_BUSY` 失败), 又必须**小于**总预算 (5s + 3s)。
const HOLD_INSIDE_BUDGET: Duration = Duration::from_millis(5500);
/// 场景 ③: 收紧既有 `busy_timeout` 后的**名义**总等待 = 0.2s + 额外预算 3s
/// + 一个 busy 窗口 250ms = **3.45s** ⇒ 用它把「有界」钉得**紧**。
const TIGHT_BUSY_TIMEOUT: Duration = Duration::from_millis(200);
/// 场景 ③ 的断言上界 (名义 3.45s × ~4) ⇒ 钉住「额外预算确实有界」。
const TIGHT_WAIT_CEILING: Duration = Duration::from_secs(15);
/// 生产配置下失败路径的断言上界。
///
/// **名义**总等待 = 既有 `busy_timeout` 5s + 额外退避预算 3s + 一个 busy 窗口
/// 250ms = **8.25s** (实测失败路径稳定在 8.25s-8.28s)。
///
/// 断言上界放宽到 30s: SQLite 的 busy handler 是**睡眠预算**, 宿主被压满时
/// (实测 load average > 100) 睡眠截止点不会准时唤醒 ⇒ 墙钟被拉长 (实测
/// 成功路径最差 22s)。**无争用**的迁移正文只花 ~18ms ⇒ 多出来的时间全在
/// 「等锁」上。断言要钉的是「**有界、不是无限挂起**」。
const TOTAL_WAIT_CEILING: Duration = Duration::from_secs(30);
/// 成功路径的断言上界 (比失败路径更松: 它含持锁时长 + 正文体)。
const SUCCESS_WAIT_CEILING: Duration = Duration::from_secs(60);

/// 外部写者: 显式 `BEGIN IMMEDIATE` 持住写锁。
///
/// - 拿到写锁后发信号 (⇒ 主线程可以确定「对方已持锁」, 无竞争窗口);
/// - 持锁到 `hold` 时长耗尽**或**收到放行信号 (⇒ 放锁时机由测试显式控制)。
struct LockHolder {
    release: Option<mpsc::Sender<()>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl LockHolder {
    /// 起一个外部写者并**等它真的拿到写锁**再返回。
    fn acquire(path: &Path, hold: Duration) -> Self {
        let (locked_tx, locked_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let path = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let conn = Connection::open(&path).expect("外部写者连接");
            conn.busy_timeout(PRODUCTION_BUSY_TIMEOUT)
                .expect("外部写者 busy_timeout");
            conn.execute_batch("BEGIN IMMEDIATE")
                .expect("外部写者拿到写锁");
            locked_tx.send(()).expect("通知「已持锁」");
            let deadline = Instant::now() + hold;
            loop {
                match release_rx.recv_timeout(Duration::from_millis(20)) {
                    // 放行 / 通道断开 ⇒ 收工
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if Instant::now() >= deadline {
                            break;
                        }
                    }
                }
            }
            let _ = conn.execute_batch("COMMIT");
        });
        locked_rx.recv().expect("等待「已持锁」信号");
        Self {
            release: Some(release_tx),
            handle: Some(handle),
        }
    }

    /// 放锁并等持锁线程收工。
    fn release(mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 按生产配置打开连接 (WAL + `busy_timeout` 5s)。
fn open_production_connection(path: &PathBuf) -> Connection {
    let conn = Connection::open(path).expect("打开生产连接");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("WAL");
    conn.busy_timeout(PRODUCTION_BUSY_TIMEOUT)
        .expect("busy_timeout");
    conn
}

/// 场景 ①: 对方持锁 (< 预算) ⇒ 生产首开**应当成功**。
///
/// 旧写法 (只靠既有 `busy_timeout` 5s) 在这里**必失败**: 对方持锁 5.5s > 5s。
#[test]
fn production_migration_succeeds_when_the_external_writer_releases_inside_the_budget() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("memory.db");
    let mut conn = open_production_connection(&path);

    let holder = LockHolder::acquire(&path, HOLD_INSIDE_BUDGET);
    let started = Instant::now();
    let outcome = run_migrations(&mut conn);
    let elapsed = started.elapsed();
    holder.release();

    outcome.expect("对方在预算内提交 ⇒ 生产首开应当成功 (旧写法在这里必失败)");
    eprintln!(
        "[busy-retry] 场景① 持锁 {HOLD_INSIDE_BUDGET:?} (< 预算) ⇒ 成功, 实测耗时 {elapsed:?}"
    );
    // 真的等过对方 (不是「没撞上锁」的空过): 持锁 5.5s ⇒ 至少等 5s。
    assert!(
        elapsed >= Duration::from_secs(5),
        "应当真的等对方提交, 实测耗时 {elapsed:?}"
    );
    // 等待有界 (不是无限挂起)。
    assert!(
        elapsed < SUCCESS_WAIT_CEILING,
        "等待必须有界 (< {SUCCESS_WAIT_CEILING:?}), 实测耗时 {elapsed:?}"
    );

    // 语义没松: 版本簿记**恰好**一条一套 (事务体只跑了一次 ⇒ 没有重复插行)。
    let recorded: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("读版本簿记");
    assert_eq!(
        recorded,
        MIGRATIONS.len() as i64,
        "版本簿记应当恰好一条一套 (事务体只执行一次)"
    );
    // 基表装订完成 (首开路径真的干了活)。
    let tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'episodes'",
            [],
            |row| row.get(0),
        )
        .expect("读 sqlite_master");
    assert_eq!(tables, 1, "首开应当装订基表");
}

/// 场景 ②: 对方一直持锁 (> 预算) ⇒ 生产首开**应当失败**, 且**耗时有界**。
#[test]
fn production_migration_fails_within_a_bounded_wait_when_the_writer_never_releases() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("memory.db");
    let mut conn = open_production_connection(&path);

    // 持锁到测试显式放行为止 (⇒ 一定超过总预算)。
    let holder = LockHolder::acquire(&path, Duration::from_secs(3600));
    let started = Instant::now();
    let outcome = run_migrations(&mut conn);
    let elapsed = started.elapsed();
    holder.release();

    eprintln!("[busy-retry] 场景② 持锁不放 (> 预算) ⇒ 有界失败, 实测耗时 {elapsed:?}");
    let error: MemoryError = outcome.expect_err("预算耗尽 ⇒ 必须失败 (不能无限挂起)");
    // 错误**仍是 busy 类** (错误码原样保留 ⇒ 结构化判定不变)。
    match &error {
        MemoryError::Sqlite(rusqlite::Error::SqliteFailure(inner, _)) => assert!(
            matches!(
                inner.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            ),
            "错误码应当仍是 busy 类, 实测: {:?}",
            inner.code
        ),
        other => panic!("错误应当是 sqlite 失败, 实测: {other:?}"),
    }
    // 错误信息看得出「等过、仍被锁」。
    let text = error.to_string();
    assert!(
        text.contains("database is locked"),
        "错误信息应当保留 busy 原文, 实测: {text}"
    );
    assert!(
        text.contains("write lock still held after waiting"),
        "错误信息应当看得出「等过、仍被锁」, 实测: {text}"
    );
    // 耗时有界: 至少等满了既有 `busy_timeout` (5s), 又不超过上界。
    assert!(
        elapsed >= PRODUCTION_BUSY_TIMEOUT,
        "应当至少等满既有 busy_timeout, 实测耗时 {elapsed:?}"
    );
    assert!(
        elapsed < TOTAL_WAIT_CEILING,
        "失败等待必须有界 (< {TOTAL_WAIT_CEILING:?}), 实测耗时 {elapsed:?}"
    );
}

/// 场景 ③: 收紧既有 `busy_timeout` 后, 失败路径的耗时被**紧**地钉住。
///
/// 这一条把「额外退避预算确实有界 (不是无限等)」钉成**紧**断言:
/// 名义总等待 = 0.2s (既有 busy_timeout) + 3s (额外预算) + 0.25s (busy 窗口)
/// = 3.45s ⇒ 断言 < 15s。
#[test]
fn production_migration_wait_stays_tightly_bounded_with_a_small_busy_timeout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("memory.db");
    let mut conn = open_production_connection(&path);
    conn.busy_timeout(TIGHT_BUSY_TIMEOUT).expect("busy_timeout");

    let holder = LockHolder::acquire(&path, Duration::from_secs(3600));
    let started = Instant::now();
    let outcome = run_migrations(&mut conn);
    let elapsed = started.elapsed();
    holder.release();

    eprintln!("[busy-retry] 场景③ 紧预算 (busy_timeout 200ms) ⇒ 有界失败, 实测耗时 {elapsed:?}");
    let error: MemoryError = outcome.expect_err("预算耗尽 ⇒ 必须失败 (不能无限挂起)");
    assert!(
        error
            .to_string()
            .contains("write lock still held after waiting"),
        "错误信息应当看得出「等过、仍被锁」, 实测: {error}"
    );
    // 至少等满了既有 `busy_timeout`。
    assert!(
        elapsed >= TIGHT_BUSY_TIMEOUT,
        "应当至少等满既有 busy_timeout, 实测耗时 {elapsed:?}"
    );
    // 紧的上界: 名义 3.45s ⇒ 断言 < 15s。
    assert!(
        elapsed < TIGHT_WAIT_CEILING,
        "紧预算下等待仍必须有界 (< {TIGHT_WAIT_CEILING:?}), 实测耗时 {elapsed:?}"
    );
}
