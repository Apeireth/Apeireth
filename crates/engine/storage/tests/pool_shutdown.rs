//! 写线程收口的回归证明: 池弃用必须**同步**关掉写连接。
//!
//! 取案 (2026-11-06, 三树同窗取证): `SqliteConnectionPool` 有一条专用写线程
//! 持写连接。旧实现的 `drop(pool)` 只丢 mpsc `Sender` ⇒ 写线程**异步**醒来才
//! 退出并关连接, `drop` **立即返回** ⇒「池已弃」与「连接已关」之间留了一个
//! 窗口 ⇒ 紧接着同路径 reopen 时撞上 WAL 索引重建要取的排他锁 (SQLite 的 WAL
//! 恢复路径直接取排他锁, **不走** busy handler ⇒ `busy_timeout` 兜不住) ⇒
//! `database is locked` (SQLITE_BUSY)。
//!
//! 本文件用两条互补的证明把「窗口已消除」钉死:
//! - [`dropping_the_pool_closes_the_write_connection_before_returning`]:
//!   **确定性**证明 —— 把写线程卡在一次写里, 于是「写线程还活着、还持着写
//!   连接」是**可证**的事实, 不是靠概率; `drop` 返回那一刻必须已经关掉写
//!   连接 (最后一个连接关闭 ⇒ SQLite 删掉 `-wal`/`-shm`)。
//! - [`reopening_the_same_path_right_after_drop_always_succeeds`]: 反复
//!   drop/reopen 同路径, 每次都必须成功 (窗口在, 就会偶发 `database is locked`)。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use apeireth_storage::SqliteConnectionPool;

/// 每个用例一个独立临时目录 (用例之间不得共用库文件)。
fn case_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-pool-shutdown-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 确定性证明: `drop(pool)` 返回时, 写连接**必须已关**。
///
/// 手法: 让写线程卡在一次写里 —— 于是「写线程此刻还活着、还持着写连接」是
/// **可证**的事实, 不是靠概率。写线程只有在这笔写被放行之后才可能退出, 所以
/// 「`drop` 返回时连接已关」这条断言在旧实现下**必然**失败 (旧实现 `drop`
/// 立刻返回, 而写线程还卡在写里 ⇒ 连接还开着 ⇒ `-shm` 还在), 在新实现下
/// **必然**成立 (`drop` 等写线程收口 ⇒ 连接已关 ⇒ `-shm` 已删)。
#[tokio::test]
async fn dropping_the_pool_closes_the_write_connection_before_returning() {
    let dir = case_dir("invariant");
    let path = dir.join("pool.db");
    let shm = dir.join("pool.db-shm");

    let pool = Arc::new(SqliteConnectionPool::open(&path).await.expect("open pool"));

    // 先做一次真实写: 让 WAL 落地 (`-wal`/`-shm` 存在), 后面才有「连接关掉
    // ⇒ `-shm` 被删」这个可观测量。
    pool.write(|conn| {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS t(a); INSERT INTO t VALUES (1);")?;
        Ok(())
    })
    .await
    .expect("seed write");
    assert!(
        shm.exists(),
        "前置条件: WAL 模式下写连接开着时 -shm 必须存在"
    );

    // 把写线程卡在写体里: 它持着写连接, 且在放行前**不可能**退出。
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel::<()>();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer_pool = Arc::clone(&pool);
    let task = tokio::spawn(async move {
        let _ = writer_pool
            .write(move |_conn| {
                let _ = entered_tx.send(());
                let _ = release_rx.recv();
                Ok(())
            })
            .await;
    });
    entered_rx.await.expect("写线程必须进入写体");

    // 放掉写体持有的池引用 ⇒ 主线程手里的 `pool` 成为最后一个引用。
    task.abort();
    let _ = task.await;

    // 由另一个线程稍后放行写体: 写线程只有在这之后才可能退出 (进而关连接)。
    let helper = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let _ = release_tx.send(());
    });

    let before = std::time::Instant::now();
    drop(pool);
    let drop_cost = before.elapsed();
    // 必须在放行写体**之前**取样: 放行之后写线程一定会退出, 届时 `-shm` 一定
    // 已被删掉, 取样就分不出「drop 同步收口」与「drop 异步退出」了。
    let shm_after_drop = shm.exists();
    helper.join().expect("helper thread");

    assert!(
        !shm_after_drop,
        "drop 返回时写连接必须已关闭 (最后一个连接关闭 ⇒ SQLite 删除 -wal/-shm); \
         旧实现只丢 mpsc Sender ⇒ 写线程异步退出 ⇒ drop 返回时连接还活着 ⇒ \
         同路径 reopen 撞 WAL 索引重建的排他锁 ⇒ database is locked"
    );
    assert!(
        drop_cost < Duration::from_secs(30),
        "drop 必须是**有界**等待 (实测 {drop_cost:?}), 不得卡死"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// 行为面证明: 紧接着同路径 reopen 必须成功 (反复跑, 不靠运气)。
///
/// 实测复现率 (2026-11-06, 本机 8 线程负载, drop 与 reopen 之间让出 1µs):
/// 修复前 **3/400** 次 reopen 报 `database is locked`; 修复后 **0/400**。
#[tokio::test]
async fn reopening_the_same_path_right_after_drop_always_succeeds() {
    let dir = case_dir("reopen");
    let path = dir.join("pool.db");
    let rounds = 100u32;

    for round in 0..rounds {
        {
            let pool = SqliteConnectionPool::open(&path).await.expect("open pool");
            pool.write(|conn| {
                conn.execute_batch("CREATE TABLE IF NOT EXISTS t(a); INSERT INTO t VALUES (1);")?;
                Ok(())
            })
            .await
            .expect("write");
            // 池在此被弃用 (最后一个引用释放) ⇒ 写连接必须**已经**关掉。
        }
        // 旧实现: 这里会偶发 `database is locked` (写线程还没退出, WAL 索引
        // 重建撞排他锁)。1µs 的让出点正是复现配方: 让写线程恰好在这时醒来。
        tokio::time::sleep(Duration::from_micros(1)).await;
        let reopened = SqliteConnectionPool::open(&path)
            .await
            .unwrap_or_else(|e| panic!("round {round}: 同路径 reopen 必须成功, 实际: {e}"));
        drop(reopened);
    }

    let _ = std::fs::remove_dir_all(&dir);
}
