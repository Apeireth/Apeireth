//! 「主体 → 会话」归属映射的**写侧**测试（2026-11-01 安全审计 P1 收口）。
//!
//! # 钉住什么
//!
//! 1. **幂等**：同一 `(principal, session)` 重复登记不报错、不重复写。
//! 2. **空主体 / 空会话 ⇒ 拒**（fail-closed：空主体会把所有人的 episode 归到同一主体）。
//! 3. ⭐ **端到端**：登记归属 ⇒ 「被遗忘权」在 **episode 维度真的生效**
//!    （`UniversalForgetOutcome.episode` 由**实测**从 `false` 变 `true`）。
//!    —— 这是本批的**目的**：不是「写进去了就算」，而是「**被遗忘权真的生效了**」。

use apeireth_memory::principal_sessions::record_principal_session_on_pool;
use apeireth_memory::{run_migrations_on_pool, UniversalForgetFacade};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

async fn pool_with_migrations() -> (tempfile::TempDir, SqliteConnectionPool) {
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("principal-sessions.sqlite");
    let pool = SqliteConnectionPool::open(&path).await.expect("打开连接池");
    run_migrations_on_pool(&pool).await.expect("跑迁移");
    (dir, pool)
}

#[tokio::test]
async fn recording_a_mapping_is_idempotent() {
    let (_dir, pool) = pool_with_migrations().await;
    assert!(
        record_principal_session_on_pool(&pool, "user-a", "session-a", 1)
            .await
            .expect("首次登记"),
        "首次登记应当新写入"
    );
    assert!(
        !record_principal_session_on_pool(&pool, "user-a", "session-a", 2)
            .await
            .expect("重复登记"),
        "重复登记应当幂等命中（不新写入、不报错）"
    );
    let count = pool
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM memory_principal_sessions WHERE principal_id='user-a'",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .expect("查映射数");
    assert_eq!(count, 1, "同一会话只应有一条映射");
}

#[tokio::test]
async fn empty_principal_or_session_is_refused() {
    let (_dir, pool) = pool_with_migrations().await;
    assert!(
        record_principal_session_on_pool(&pool, "", "session-a", 1)
            .await
            .is_err(),
        "空主体必须被拒（否则会把所有人的 episode 归到同一主体）"
    );
    assert!(
        record_principal_session_on_pool(&pool, "user-a", "  ", 2)
            .await
            .is_err(),
        "空会话必须被拒（没有会话就没有归属连接点）"
    );
    let count = pool
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM memory_principal_sessions",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .expect("查映射数");
    assert_eq!(count, 0, "被拒的登记不得落库");
}

#[tokio::test]
async fn a_mapped_session_makes_the_episode_dimension_of_forget_work() {
    let (_dir, pool) = pool_with_migrations().await;
    pool.write(|conn| {
        conn.execute(
            "INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) \
             VALUES ('ep-a','user-a','session-a',1,'user','要忘掉的内容')",
            [],
        )?;
        Ok(())
    })
    .await
    .expect("写 episode");

    // ① 未登记归属 ⇒ 归属**不可判定** ⇒ episode 维度**不生效**（生产旧形态）。
    let before = UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 100, None)
        .expect("主体遗忘");
    assert!(
        !before.episode,
        "未登记归属时 episode 维度不得声称已擦除（实测口径）"
    );
    assert!(
        before.attribution.unattributable(),
        "未登记归属 ⇒ 归属不可判定：{before:?}"
    );

    // ② 登记归属 ⇒ 同一个主体、同一个 episode ⇒ **被遗忘权真的生效**。
    record_principal_session_on_pool(&pool, "user-a", "session-a", 1)
        .await
        .expect("登记归属");
    let after = UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 200, None)
        .expect("主体遗忘");
    assert!(
        after.episode,
        "登记归属后 episode 维度必须真的生效（这是本批的目的）：{after:?}"
    );
    assert!(
        after.counts.episodes > 0,
        "应当真的擦到 episode 行：{after:?}"
    );
    assert_eq!(
        after.attribution.sessions_mapped, 1,
        "登记后映射数应为 1：{after:?}"
    );
    assert_eq!(
        after.attribution.episodes_attributable, 1,
        "登记后 episode 应当能归到主体：{after:?}"
    );
}
