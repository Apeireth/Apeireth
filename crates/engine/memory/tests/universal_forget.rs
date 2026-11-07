use apeireth_memory::{run_migrations_on_pool, UniversalForgetFacade};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

#[tokio::test]
async fn principal_forget_is_atomic_and_preserves_append_only_provenance() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget.sqlite");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();
    run_migrations_on_pool(&pool).await.unwrap();

    pool.write(|conn| {
        conn.execute("INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-a','session-a',1)", [])?;
        conn.execute("INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-a','user-a','session-a',1,'user','private')", [])?;
        conn.execute("INSERT INTO commitments (id,subject_id,statement,source_episode_id,created_at_ms,updated_at_ms) VALUES ('c-a','user-a','todo','ep-a',1,1)", [])?;
        conn.execute("INSERT INTO temporal_graph_facts (id,relation_key,subject_id,predicate,object_id,valid_from_ms,believed_at_ms,source_episode_id,created_at_ms) VALUES ('f-a','rel-a','user-a','lives_in','Wuhan',1,1,'ep-a',1)", [])?;
        conn.execute("INSERT INTO persona_profiles (persona_id,subject_id,revision,created_at_ms,updated_at_ms) VALUES ('persona-a','user-a',1,1,1)", [])?;
        conn.execute("INSERT INTO persona_profile_governance (persona_id,subject_id) VALUES ('persona-a','user-a')", [])?;
        Ok(())
    }).await.unwrap();

    let result =
        UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 100, Some("user request"));
    assert!(result.is_ok(), "{result:?}");
    let observed = pool.read(|conn| {
        let episode_status: String = conn.query_row("SELECT status FROM episode_governance WHERE episode_id='ep-a'", [], |row| row.get(0))?;
        let commitment_status: String = conn.query_row("SELECT status FROM commitments WHERE id='c-a'", [], |row| row.get(0))?;
        let commitment_events: i64 = conn.query_row("SELECT COUNT(*) FROM commitment_events WHERE commitment_id='c-a'", [], |row| row.get(0))?;
        let retracted: i64 = conn.query_row("SELECT COUNT(*) FROM temporal_graph_facts WHERE relation_key='rel-a' AND retracted_at_ms IS NOT NULL", [], |row| row.get(0))?;
        let tombstoned: i64 = conn.query_row("SELECT COUNT(*) FROM persona_profile_governance WHERE persona_id='persona-a' AND tombstoned_at_ms IS NOT NULL", [], |row| row.get(0))?;
        Ok((episode_status, commitment_status, commitment_events, retracted, tombstoned))
    }).unwrap();
    assert_eq!(observed, ("forgotten".into(), "cancelled".into(), 1, 1, 1));
    assert_eq!(
        pool.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM persona_profiles WHERE persona_id='persona-a'",
            [],
            |row| row.get::<_, i64>(0),
        )?))
        .unwrap(),
        1
    );
}

#[tokio::test]
async fn principal_forget_does_not_touch_unowned_principal() {
    let pool = SqliteConnectionPool::in_memory().await.unwrap();
    run_migrations_on_pool(&pool).await.unwrap();
    pool.write(|conn| {
        conn.execute("INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-b','session-b',1)", [])?;
        conn.execute("INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-b','user-b','session-b',1,'user','keep')", [])?;
        Ok(())
    }).await.unwrap();
    UniversalForgetFacade::forget_principal_on_pool(&pool, "user-a", 100, None).unwrap();
    assert!(
        pool.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM episode_governance WHERE episode_id='ep-b'",
            [],
            |row| row.get::<_, i64>(0),
        )?))
        .unwrap()
            == 0
    );
}

/// **生产路径**的按主体全量遗忘。
///
/// 此前只有 pool 版有测试，而生产用的是 [`apeireth_memory::SqliteMemoryStore`]
/// （持有 `Mutex<Connection>`，不是 pool）⇒ 那个入口**生产上根本调不到**。
/// 本测试钉住「store 版也能用」，否则「被遗忘权」在生产上仍是缺口。
#[test]
fn principal_forget_works_through_the_production_store() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget-store.sqlite");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open store");

    {
        let conn = store.conn().expect("conn");
        conn.execute("INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-a','session-a',1)", []).expect("principal session");
        conn.execute("INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-a','user-a','session-a',1,'user','private')", []).expect("episode");
        conn.execute("INSERT INTO commitments (id,subject_id,statement,source_episode_id,created_at_ms,updated_at_ms) VALUES ('c-a','user-a','todo','ep-a',1,1)", []).expect("commitment");
    }

    let result =
        UniversalForgetFacade::forget_principal(&store, "user-a", 100, Some("user removed"));
    assert!(result.is_ok(), "{result:?}");
    let outcome = result.expect("should forget");
    assert!(
        outcome.episode && outcome.commitment,
        "两账都要清: {outcome:?}"
    );

    let conn = store.conn().expect("conn");
    let episode_status: String = conn
        .query_row(
            "SELECT status FROM episode_governance WHERE episode_id='ep-a'",
            [],
            |row| row.get(0),
        )
        .expect("episode governance row");
    let commitment_status: String = conn
        .query_row("SELECT status FROM commitments WHERE id='c-a'", [], |row| {
            row.get(0)
        })
        .expect("commitment row");
    assert_eq!(episode_status, "forgotten");
    assert_eq!(commitment_status, "cancelled");
}

/// 空主体 id 必须拒绝（不然会「删空」—— 那是全库清空）。
#[test]
fn principal_forget_rejects_empty_principal() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget-empty.sqlite");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open store");
    let error = UniversalForgetFacade::forget_principal(&store, "  ", 100, None)
        .expect_err("空主体必须拒绝");
    assert!(error.contains("empty"), "理由应可读: {error}");
}

// ============================================================================
// 2026-10-30 · 静态安全审计复核：`universal_forget` 的「虚假保证」修复
// ============================================================================
//
// 审计（`/root/security-audit-skill/apeireth/`）指出：主体级遗忘的返回体
// `episode/commitment/temporal/persona` 是**写死的常量** ⇒ 与真实效果脱钩。
//
// 本组测试钉住**修复后的口径**：
//   ① **归属不可判定**（生产默认：无 `memory_principal_sessions` 映射）
//      ⇒ `episode == false`（**不谎称已擦除**）+ `attribution.unattributable() == true`
//      ⇒ 且**实测行数为 0**（确实什么都没擦）。
//   ② **有归属映射** ⇒ `episode == true` + `counts.episodes == 1` + 真落 forgotten 行。
//   ③ **幂等重跑** ⇒ 仍 `episode == true`（后置状态口径），但 `counts.episodes == 0`
//      （本次没有新增 tombstone）。
//   ④ `commitment` / `temporal` / `persona` 三账按 `subject_id` 归属
//      ⇒ **与归属映射无关** ⇒ 生产环境**真的**会撤回。

/// 生产默认形态：**没有归属映射** ⇒ 不得谎称「episode 已擦除」。
#[test]
fn principal_forget_without_ownership_mapping_does_not_claim_success() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget-unmapped.sqlite");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open store");

    // 只写 episode，**不写** `memory_principal_sessions` ⇒ 生产真实形态。
    {
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-x','user-x','session-x',1,'user','private')",
            [],
        )
        .expect("episode");
    }

    let outcome = UniversalForgetFacade::forget_principal(&store, "user-x", 100, Some("用户要求"))
        .expect("不应报错");
    assert!(
        !outcome.episode,
        "归属不可判定 ⇒ 不得报告 episode 已擦除（旧实现会写死 true）: {outcome:?}"
    );
    assert_eq!(outcome.counts.episodes, 0, "确实一个 episode 都没擦");
    assert!(
        outcome.attribution.unattributable(),
        "必须把「归属不可判定」显式带出来: {:?}",
        outcome.attribution
    );
    assert_eq!(outcome.attribution.sessions_mapped, 0);
    assert_eq!(outcome.attribution.episodes_attributable, 0);
}

/// 有归属映射 ⇒ `episode == true` 且**实测行数为 1**（不是常量）。
#[test]
fn principal_forget_with_ownership_mapping_reports_measured_counts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget-mapped.sqlite");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open store");

    {
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-y','session-y',1)",
            [],
        )
        .expect("principal session");
        conn.execute(
            "INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-y','user-y','session-y',1,'user','private')",
            [],
        )
        .expect("episode");
        conn.execute(
            "INSERT INTO commitments (id,subject_id,statement,source_episode_id,created_at_ms,updated_at_ms) VALUES ('c-y','user-y','todo','ep-y',1,1)",
            [],
        )
        .expect("commitment");
    }

    let outcome = UniversalForgetFacade::forget_principal(&store, "user-y", 100, Some("用户要求"))
        .expect("不应报错");
    assert!(outcome.episode, "有归属且已清空 ⇒ 应为真: {outcome:?}");
    assert_eq!(outcome.counts.episodes, 1, "实测：tombstone 1 条");
    assert_eq!(outcome.counts.commitments, 1, "实测：撤回 1 条 commitment");
    assert_eq!(outcome.attribution.sessions_mapped, 1);
    assert_eq!(outcome.attribution.episodes_attributable, 1);
    assert_eq!(outcome.attribution.episodes_outstanding, 0);
    assert!(!outcome.attribution.unattributable());
    assert!(
        outcome.commitment,
        "commitment 按 subject_id 归属 ⇒ 确实撤回"
    );
}

/// 幂等重跑：后置状态口径 ⇒ 仍为 `true`，但本次新增 tombstone 为 0。
#[test]
fn principal_forget_is_idempotent_and_reports_zero_delta_on_second_run() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("universal-forget-idempotent.sqlite");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open store");

    {
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-z','session-z',1)",
            [],
        )
        .expect("principal session");
        conn.execute(
            "INSERT INTO episodes (id,continuity_id,session_id,timestamp,role,content) VALUES ('ep-z','user-z','session-z',1,'user','private')",
            [],
        )
        .expect("episode");
    }

    let first =
        UniversalForgetFacade::forget_principal(&store, "user-z", 100, Some("第一次")).expect("ok");
    assert!(first.episode);
    assert_eq!(first.counts.episodes, 1);

    let second =
        UniversalForgetFacade::forget_principal(&store, "user-z", 200, Some("第二次")).expect("ok");
    assert!(
        second.episode,
        "后置状态口径 ⇒ 幂等重跑仍为 true: {second:?}"
    );
    assert_eq!(second.counts.episodes, 0, "本次没有新增 tombstone ⇒ 应为 0");
    assert_eq!(second.attribution.episodes_outstanding, 0, "无残留");
}
