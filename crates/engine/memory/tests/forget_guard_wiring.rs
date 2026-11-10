//! 「**先记访问史，再忘记**」守卫的生产接线契约 (G1 收口批).
//!
//! # 这份测试在钉什么
//!
//! 守卫 (`forget_access_guard`) 上一批只交付了类型 + 判定, **没接线** ——
//! 生产 forgets 入口还能绕开它直接销毁。本批把守卫接进三条生产 forgets 路径,
//! 这份测试钉住接线的**行为契约**:
//!
//! * **正向**: 先有**真实**访问事件 ⇒ 销毁成功, 且事件**真的落库**;
//! * **反向**: 没有访问史 ⇒ 销毁**被拒** (`Err`), 记录**仍在**;
//! * **覆盖不全**: 3 条目标只有 2 条有访问史 ⇒ **缺一即拒**, 3 条都在;
//! * **反伪造**: 只有 permit、没有真实访问事件 ⇒ **被拒**
//!   (证明守卫不是靠「有个 permit 就行」);
//! * **三条路径各受约束**: `universal_forget` / `memory_governance` /
//!   `forget_coordinator`;
//! * **兼容面如实标注**: 既有入口保持既有行为, 但「未覆盖也执行了」必须**留痕**
//!   (返回标注 + 审计行), 不许静默降级成「没记也算过」。
//!
//! # 纪律: 测试也必须走真实写入面
//!
//! 访问事件一律用生产同一条写入路径
//! (`SqliteAccessHistoryStore::record_event` / `record_selected_context`) 落库,
//! **不**手搓 `AccessEvent` 冒充访问 (唯一例外是「反伪造」用例 —— 那里恰恰要
//! 证明手搓的事件换来的 permit **不被接受**)。

use std::path::Path;
use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::forget_access_guard::{missing_access_cover, AccessGuard, ForgetPermit};
use apeireth_memory::universal_forget::{AccessGuardState, UniversalForgetFacade};
use apeireth_memory::{
    AccessEvent, DerivedRef, EpisodeStore, MemoryGovernanceStore, SqliteAccessHistoryStore,
    SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;
use serde_json::json;
use tempfile::tempdir;

/// 打开一个「治理表 + 访问史表同账」的文件库 (生产同构: 一个 cognitive 库).
async fn open_stack(path: &Path) -> (Arc<SqliteMemoryStore>, SqliteAccessHistoryStore) {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open store"));
    let access = SqliteAccessHistoryStore::new(SqliteConnectionPool::open(path).await.unwrap(), 64);
    access.ensure_schema().await.expect("access schema");
    (store, access)
}

/// 真实访问史写入面 (生产同一条路径).
async fn record_access(access: &SqliteAccessHistoryStore, memory_id: &str) {
    access
        .record_selected_context(
            memory_id,
            None,
            Some("session-guard".into()),
            1_000,
            Some("guard test query".into()),
            Some(0),
            Some(1.0),
            json!({"selected": true}),
        )
        .await
        .expect("访问史写入");
}

/// 用**真实落库**的事件换一张 permit (生产调用方的标准姿势).
fn permit_from_real_history(access: &SqliteAccessHistoryStore, memory_id: &str) -> ForgetPermit {
    let events = access
        .latest_accesses(memory_id, 8)
        .expect("读访问史")
        .into_iter()
        .collect::<Vec<AccessEvent>>();
    let mut guard = AccessGuard::new(1_000);
    let receipt = guard.record(events).expect("非空批次才有回执");
    guard.permit(receipt)
}

/// 手搓一张「看起来合法」的 permit (反伪造用例专用): 事件没落库.
fn forged_permit(memory_id: &str) -> ForgetPermit {
    let event = AccessEvent {
        id: "forged".into(),
        memory_id: memory_id.into(),
        subject_id: None,
        session_id: Some("session-guard".into()),
        accessed_at_ms: 1_000,
        query: Some("forged query".into()),
        access_kind: "selected_context".into(),
        rank: Some(0),
        score: Some(1.0),
        metadata: json!({"forged": true}),
    };
    let mut guard = AccessGuard::new(1_000);
    let receipt = guard.record(vec![event]).expect("非空批次才有回执");
    guard.permit(receipt)
}

fn put(store: &Arc<SqliteMemoryStore>, id: &str, session: &str, content: &str) {
    store
        .put_episode(&Episode {
            id: id.into(),
            timestamp: 1_000,
            role: "user".into(),
            content: content.into(),
            session_id: session.into(),
        })
        .expect("写 episode");
}

fn count_events(store: &Arc<SqliteMemoryStore>, memory_id: &str) -> i64 {
    let conn = store.conn().expect("conn");
    conn.query_row(
        "SELECT COUNT(*) FROM memory_access_events WHERE memory_id=?1",
        rusqlite::params![memory_id],
        |row| row.get(0),
    )
    .expect("数事件")
}

/// 治理状态 (active / forgotten).
fn governance_status(store: &Arc<SqliteMemoryStore>, id: &str) -> String {
    store
        .get_governed(id)
        .expect("读治理态")
        .map(|governed| governed.status.as_str().to_string())
        .unwrap_or_else(|| "missing".to_string())
}

/// 审计账里的守卫判定行数 (兼容面「未覆盖也执行了」的留痕).
fn guard_audit_rows(store: &Arc<SqliteMemoryStore>) -> i64 {
    let conn = store.conn().expect("conn");
    conn.query_row(
        "SELECT COUNT(*) FROM research_lineage_events WHERE op='governance_access_guard'",
        [],
        |row| row.get(0),
    )
    .expect("数审计行")
}

/// 正向: 先记真实访问史 ⇒ 带 permit 的销毁成功, 且事件**真的落库**、销毁后仍在.
#[tokio::test]
async fn guarded_forget_succeeds_after_real_access_event() {
    let dir = tempdir().unwrap();
    let (store, access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "被访问过的记忆");
    record_access(&access, "ep-1").await;

    let permit = permit_from_real_history(&access, "ep-1");
    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    let outcome = facade
        .forget_episode_with_permit("ep-1", Some("用户要求"), 0, permit)
        .expect("有访问史 ⇒ 销毁成功");

    assert!(outcome.episode, "episode 账应被清");
    assert_eq!(outcome.access_guard.state, AccessGuardState::Permitted);
    assert_eq!(governance_status(&store, "ep-1"), "forgotten");
    assert_eq!(count_events(&store, "ep-1"), 1, "访问史事件必须真落库");
}

/// 反向: 没有访问史 ⇒ 销毁被拒 (`Err`), 记录**仍在** (没被删).
#[tokio::test]
async fn guarded_forget_refuses_without_access_history() {
    let dir = tempdir().unwrap();
    let (store, _access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "从没被访问过的记忆");

    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    let error = facade
        .forget_episode("ep-1", Some("用户要求"), 0)
        .expect_err("没访问史 ⇒ 必须拒绝销毁");

    assert!(
        error.to_string().contains("访问史"),
        "理由应如实说明: {error}"
    );
    assert_eq!(governance_status(&store, "ep-1"), "active", "记录必须还在");
    assert_eq!(count_events(&store, "ep-1"), 0);
}

/// 覆盖不全: 3 条目标只有 2 条有访问史 ⇒ 缺一即拒, 3 条都在.
#[tokio::test]
async fn partial_coverage_is_refused_and_all_records_survive() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("guard.sqlite");
    let (store, access) = open_stack(&path).await;
    {
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-a','session-a',1)",
            [],
        )
        .expect("主体会话");
    }
    put(&store, "ep-1", "session-a", "一");
    put(&store, "ep-2", "session-a", "二");
    put(&store, "ep-3", "session-a", "三");
    record_access(&access, "ep-1").await;
    record_access(&access, "ep-2").await;

    // permit 覆盖 3 条 (含一条手搓事件) ⇒ 通过 permit 覆盖检查,
    // 但真实访问史只覆盖 2 条 ⇒ 必须被「反查真实访问史」拦下.
    let mut events = access.latest_accesses("ep-1", 8).expect("读访问史");
    events.extend(access.latest_accesses("ep-2", 8).expect("读访问史"));
    events.push(forged_permit("ep-3").events.remove(0));
    let mut guard = AccessGuard::new(1_000);
    let receipt = guard.record(events).expect("回执");
    let permit = guard.permit(receipt);

    let pool = SqliteConnectionPool::open(&path).await.unwrap();
    let error = UniversalForgetFacade::forget_principal_with_permit_on_pool(
        &pool,
        "user-a",
        100,
        Some("用户要求"),
        permit,
    )
    .expect_err("缺一条覆盖 ⇒ 必须拒绝");

    assert!(error.contains("访问史未覆盖"), "理由应指出缺哪条: {error}");
    for id in ["ep-1", "ep-2", "ep-3"] {
        assert_eq!(governance_status(&store, id), "active", "{id} 必须还在");
    }
}

/// 反伪造: 只有 permit、没有真实访问事件 ⇒ 被拒 (守卫不是靠「有个 permit 就行」).
#[tokio::test]
async fn forged_permit_without_real_events_is_rejected() {
    let dir = tempdir().unwrap();
    let (store, _access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "没被访问过的记忆");

    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    let permit = forged_permit("ep-1");
    let error = facade
        .forget_episode_with_permit("ep-1", Some("用户要求"), 0, permit)
        .expect_err("假 permit 必须被拒");

    assert!(
        error.to_string().contains("访问史未覆盖"),
        "理由应指向真实访问史: {error}"
    );
    assert_eq!(governance_status(&store, "ep-1"), "active", "记录必须还在");
}

/// permit 必须**对准**目标: 拿别的目标的 permit 来忘 ⇒ 拒.
#[tokio::test]
async fn permit_for_another_target_is_rejected() {
    let dir = tempdir().unwrap();
    let (store, access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "一");
    put(&store, "ep-2", "session-guard", "二");
    // 两个目标都有真实访问史 ⇒ 真实覆盖检查能过 ⇒ 拒因必须是「permit 没对准」.
    record_access(&access, "ep-1").await;
    record_access(&access, "ep-2").await;

    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    let permit = permit_from_real_history(&access, "ep-2");
    let error = facade
        .forget_episode_with_permit("ep-1", Some("用户要求"), 0, permit)
        .expect_err("permit 与目标不符 ⇒ 必须被拒");

    assert!(
        error.to_string().contains("permit 未覆盖"),
        "理由应指出 permit 没对准: {error}"
    );
    assert_eq!(governance_status(&store, "ep-1"), "active");
}

/// 幂等: 销毁后再调 ⇒ 行为可预期 (不 panic, 报「已遗忘」).
#[tokio::test]
async fn repeated_guarded_forget_is_idempotent_and_does_not_panic() {
    let dir = tempdir().unwrap();
    let (store, access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "被访问过的记忆");
    record_access(&access, "ep-1").await;

    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    let first = facade
        .forget_episode_with_permit(
            "ep-1",
            Some("用户要求"),
            0,
            permit_from_real_history(&access, "ep-1"),
        )
        .expect("首次销毁成功");
    assert!(first.episode);

    let second = facade.forget_episode_with_permit(
        "ep-1",
        Some("用户要求"),
        1,
        permit_from_real_history(&access, "ep-1"),
    );
    assert!(second.is_err(), "二次销毁必须报错而不是 panic");
    assert_eq!(governance_status(&store, "ep-1"), "forgotten");
}

/// 路径 2: `memory_governance` 的守卫版入口 (permit 必填) —— 缺覆盖即拒.
#[tokio::test]
async fn governance_path_is_guarded() {
    let dir = tempdir().unwrap();
    let (store, access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "被访问过的记忆");

    // 无访问史 ⇒ 拒.
    let error = store
        .forget_episode_guarded("ep-1", Some("用户要求"), 0, forged_permit("ep-1"))
        .expect_err("无访问史 ⇒ 必须拒绝");
    assert!(error.to_string().contains("访问史"), "{error}");
    assert_eq!(governance_status(&store, "ep-1"), "active");

    // 有访问史 ⇒ 过.
    record_access(&access, "ep-1").await;
    let governed = store
        .forget_episode_guarded(
            "ep-1",
            Some("用户要求"),
            0,
            permit_from_real_history(&access, "ep-1"),
        )
        .expect("有访问史 ⇒ 成功");
    assert_eq!(governed.status.as_str(), "forgotten");
}

/// 路径 3: `forget_coordinator` 的三段式协调遗忘 —— 缺访问史即整体拒绝.
#[tokio::test]
async fn coordinator_path_is_guarded() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("guard.sqlite");
    let (store, access) = open_stack(&path).await;
    put(&store, "ep-1", "session-guard", "根事实");

    let request = apeireth_memory::CoordinatedForget {
        roots: vec![DerivedRef::new("episode", "ep-1")],
        reason: "用户要求".into(),
        approval_id: "apv-1".into(),
    };

    // 无访问史 ⇒ 拒, 且一条都不销毁.
    let error = store
        .research_coordinated_forget(&request)
        .expect_err("无访问史 ⇒ 必须拒绝");
    assert!(error.to_string().contains("访问史"), "{error}");
    assert_eq!(
        governance_status(&store, "ep-1"),
        "active",
        "拒绝 ⇒ 不得销毁"
    );

    // 有访问史 ⇒ 过 (三段式全跑完).
    record_access(&access, "ep-1").await;
    let report = store
        .research_coordinated_forget(&request)
        .expect("有访问史 ⇒ 成功");
    assert!(report.behavior_contract_verified, "契约自验必须通过");
    assert_eq!(report.access_guard.state, AccessGuardState::Permitted);
    assert_eq!(governance_status(&store, "ep-1"), "forgotten");
}

/// 访问史是 append-only: 销毁**之后**事件仍在 (事后可查「当初为什么被降权」).
#[tokio::test]
async fn access_history_survives_the_forget() {
    let dir = tempdir().unwrap();
    let (store, access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "被访问过的记忆");
    record_access(&access, "ep-1").await;

    let facade = UniversalForgetFacade::new(Arc::clone(&store));
    facade
        .forget_episode_with_permit(
            "ep-1",
            Some("用户要求"),
            0,
            permit_from_real_history(&access, "ep-1"),
        )
        .expect("销毁成功");

    assert_eq!(count_events(&store, "ep-1"), 1, "销毁不许带走访问史");
    assert_eq!(
        access.latest_accesses("ep-1", 8).expect("读访问史").len(),
        1
    );
}

/// 兼容面: 既有主体遗忘入口保持既有行为, 但**如实标注**「未覆盖」+ 留审计痕.
#[tokio::test]
async fn legacy_principal_path_reports_uncovered_truthfully() {
    let dir = tempdir().unwrap();
    let (store, _access) = open_stack(&dir.path().join("guard.sqlite")).await;
    {
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO memory_principal_sessions (principal_id, session_id, created_at_ms) VALUES ('user-a','session-a',1)",
            [],
        )
        .expect("主体会话");
    }
    put(&store, "ep-1", "session-a", "没被访问过");

    let outcome = UniversalForgetFacade::forget_principal(&store, "user-a", 100, Some("用户要求"))
        .expect("既有行为保持: 主体遗忘仍可用");
    assert!(
        outcome.episode && outcome.commitment,
        "两账都要清: {outcome:?}"
    );
    assert_eq!(
        outcome.access_guard.state,
        AccessGuardState::Uncovered,
        "未覆盖必须如实标注 (不许静默降级)"
    );
    assert_eq!(outcome.access_guard.missing, vec!["ep-1".to_string()]);
    assert!(guard_audit_rows(&store) >= 1, "未覆盖也执行 ⇒ 必须留审计痕");
}

/// 兼容面: 既有治理入口 (retention / coordinator / CLI 面板用) 保持既有行为 + 留痕.
#[tokio::test]
async fn legacy_governance_entry_keeps_working_and_leaves_audit_trail() {
    let dir = tempdir().unwrap();
    let (store, _access) = open_stack(&dir.path().join("guard.sqlite")).await;
    put(&store, "ep-1", "session-guard", "没被访问过");

    let governed = store
        .forget_episode("ep-1", Some("用户要求"), 0)
        .expect("既有入口行为不变");
    assert_eq!(governed.status.as_str(), "forgotten");
    assert!(guard_audit_rows(&store) >= 1, "守卫判定必须留痕");
}

/// 守卫本体不变式: 空批次 ⇒ 没有回执 (没访问过就没得记) ⇒ 拿不到 permit.
#[test]
fn empty_access_batch_yields_no_permit() {
    let mut guard = AccessGuard::new(1_000);
    assert!(guard.record(Vec::new()).is_none(), "空批次不得给回执");
    assert_eq!(guard.recorded_count(), 0);
}

/// 覆盖判定的既有语义: 缺哪些 id 逐个报出 (缺一即不覆盖).
#[test]
fn missing_access_cover_reports_every_missing_id() {
    let events = vec![AccessEvent {
        id: "e1".into(),
        memory_id: "ep-1".into(),
        subject_id: None,
        session_id: None,
        accessed_at_ms: 1_000,
        query: None,
        access_kind: "selected_context".into(),
        rank: Some(0),
        score: Some(1.0),
        metadata: json!({}),
    }];
    let targets = vec!["ep-1".to_string(), "ep-2".to_string(), "ep-3".to_string()];
    assert_eq!(
        missing_access_cover(&events, &targets),
        vec!["ep-2".to_string(), "ep-3".to_string()],
        "缺一个就不算覆盖, 且逐个报出"
    );
}
