//! 生产路径 (sqlite 任务队列) 的 SQLITE_BUSY 有界退避重试: 确定性取证。
//!
//! **被钉的根因** (上一批只在**测试侧**重试, 生产路径仍会直接失败):
//! - `TaskStore::update` 用 `BEGIN IMMEDIATE` 起事务
//!   (`crates/engine/runtime-assembly/src/sqlite_task_queue.rs`), 赢方在事务里做
//!   **整档读改写** (删任务表 + 逐行插回 + 3 行 meta upsert) ⇒ 写锁一直持有到
//!   `commit()`。
//! - 输方的全部预算 = 连接的 `busy_timeout` (5s) ⇒ 赢方耗时 > 预算时, 输方拿到
//!   `SQLITE_BUSY` ⇒ 生产路径**直接失败**。
//! - 端口层的 `TaskQueueError::Store { reason }` 只有**人读 reason** (没有结构化的
//!   sqlite 错误码) ⇒ 上一批只能在**测试侧**按文案猜「是不是 busy」(弱判定)。
//!   本批在**生产侧**直接用 `rusqlite::Error` 的 `ErrorCode` **结构化判定**
//!   (`DatabaseBusy` / `DatabaseLocked`) ⇒ 那套文案匹配不再需要。
//!
//! **确定性手法** (不靠时间窗口赌概率):
//! - 外部写者用**另一个** `rusqlite::Connection` 对同一个库文件显式
//!   `BEGIN IMMEDIATE` 并**持住写锁**;
//! - 持锁方拿到写锁后**先发信号**, 主线程收到信号才发起生产调用 ⇒ 「对方已持锁」
//!   是**前置条件**而不是概率事件;
//! - 放锁时机由测试**显式控制** (发信号) 或由显式持锁时长决定 ⇒ 两个场景都可复现。
//!
//! **场景**:
//! - ① 对方持锁 < 预算 ⇒ 生产 `update` **应当成功** (旧写法必失败);
//! - ② 对方持锁 > 预算 ⇒ 生产 `update` **应当失败**, 且**耗时有界** (不是无限
//!   挂起), 且错误**仍是 busy 类**。

#![cfg(feature = "desktop")]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use apeireth_orchestration::task_queue::{TaskQueueError, TaskQueueState, TaskStore};
use apeireth_runtime_assembly::SqliteTaskStore;
use rusqlite::Connection;

/// 生产配置 (`SqliteTaskStore::open`): `busy_timeout` 5s。
const PRODUCTION_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
/// 场景 ①: 对方持锁时长 —— 必须**超过**既有 `busy_timeout` (5s) 才能证伪旧写法
/// (旧写法在 5s 处就 `SQLITE_BUSY` 失败), 又必须**小于**总预算 (5s + 3s)。
const HOLD_INSIDE_BUDGET: Duration = Duration::from_millis(5500);
/// 生产配置下失败路径的断言上界。
///
/// **名义**总等待 = 既有 `busy_timeout` 5s + 额外退避预算 3s + 一个 busy 窗口
/// 250ms = **8.25s** (实测失败路径稳定在 8.25s-8.28s)。
///
/// 断言上界放宽到 30s: SQLite 的 busy handler 是**睡眠预算**, 宿主被压满时
/// (实测 load average > 100) 睡眠截止点不会准时唤醒 ⇒ 墙钟被拉长 (实测
/// 成功路径最差 22s)。断言要钉的是「**有界、不是无限挂起**」。
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

/// 按生产配置打开存储 (建表 + `busy_timeout` 5s)。**必须在外部写者拿锁之前**开。
///
/// 生产上任务表与**会话账本同库**, 那个库由 `SqliteConnectionPool` 打开
/// (`crates/engine/storage/src/pool.rs` 的 `SqliteConfig::default()`) ⇒
/// **WAL + `busy_timeout` 5000ms**。本测试先把文件置成 WAL (journal mode 是
/// 文件头里的持久属性) ⇒ 存储与外部写者都在 WAL 下 ⇒ 写锁语义与生产一致。
fn open_production_store(path: &PathBuf) -> SqliteTaskStore {
    let seed = Connection::open(path).expect("打开库文件");
    seed.pragma_update(None, "journal_mode", "WAL")
        .expect("WAL");
    drop(seed);
    SqliteTaskStore::open(path).expect("打开生产存储")
}

/// 场景 ①: 对方持锁 (< 预算) ⇒ 生产 `update` **应当成功**。
///
/// 旧写法 (只靠既有 `busy_timeout` 5s) 在这里**必失败**: 对方持锁 5.5s > 5s。
#[test]
fn production_update_succeeds_when_the_external_writer_releases_inside_the_budget() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sessions.sqlite3");
    let store = open_production_store(&path);

    let holder = LockHolder::acquire(&path, HOLD_INSIDE_BUDGET);
    let started = Instant::now();
    let outcome = store.update(&mut |state: &mut TaskQueueState| {
        state.write_seq += 1;
    });
    let elapsed = started.elapsed();
    holder.release();

    outcome.expect("对方在预算内提交 ⇒ 生产 update 应当成功 (旧写法在这里必失败)");
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

    // 事务体**只跑了一次**: `write_seq` 恰好 +1 (重试若重跑了整档读改写, 这里
    // 就会变成 2 ⇒ 重复吃 `write_seq`)。
    let state = store.read().expect("读回整档");
    assert_eq!(
        state.write_seq, 1,
        "事务体应当只执行一次 (write_seq 恰好 +1, 不得重复吃)"
    );
}

/// 场景 ②: 对方一直持锁 (> 预算) ⇒ 生产 `update` **应当失败**, 且**耗时有界**。
#[test]
fn production_update_fails_within_a_bounded_wait_when_the_writer_never_releases() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sessions.sqlite3");
    let store = open_production_store(&path);

    // 持锁到测试显式放行为止 (⇒ 一定超过总预算)。
    let holder = LockHolder::acquire(&path, Duration::from_secs(3600));
    let started = Instant::now();
    let outcome = store.update(&mut |state: &mut TaskQueueState| {
        state.write_seq += 1;
    });
    let elapsed = started.elapsed();
    holder.release();

    eprintln!("[busy-retry] 场景② 持锁不放 (> 预算) ⇒ 有界失败, 实测耗时 {elapsed:?}");
    let error: TaskQueueError = outcome.expect_err("预算耗尽 ⇒ 必须失败 (不能无限挂起)");
    let reason = match &error {
        TaskQueueError::Store { operation, reason } => {
            assert_eq!(*operation, "begin", "失败应当发生在起事务这一步");
            reason.clone()
        }
        other => panic!("错误应当是存储失败, 实测: {other:?}"),
    };
    // 错误**仍是 busy 类** (生产侧按 `ErrorCode` 结构化判定 ⇒ 错误码原样保留)。
    assert!(
        reason.contains("database is locked"),
        "错误信息应当保留 busy 原文, 实测: {reason}"
    );
    // 错误信息看得出「等过、仍被锁」。
    assert!(
        reason.contains("write lock still held after waiting"),
        "错误信息应当看得出「等过、仍被锁」, 实测: {reason}"
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

    // 失败的那次 update **没有留下任何写入** (事务压根没起 ⇒ 整档读改写没跑过)。
    let state = store.read().expect("读回整档");
    assert_eq!(
        state.write_seq, 0,
        "失败的事务体不得留下写入 (write_seq 应当仍是 0)"
    );
}
