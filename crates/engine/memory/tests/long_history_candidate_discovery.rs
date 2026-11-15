//! 长历史关键事实：情节发现「候选阶段」的回归测试（三条候选发现路径同口径）。
//!
//! # 缺陷形状（要钉住的那条行为）
//!
//! 情节发现的候选阶段若**只按最新时间**截断候选 ⇒ ⇒ 早期确立的关键事实
//! （姓名 / 住址 / 长期约定）在进入正式排序与评分**之前**就被近期记录挤掉
//! ⇒ ⇒ 后面的检索层再强也救不回来（它根本不在候选池里）。
//!
//! # 用例形状
//!
//! 一条早期关键事实 + 大量近期无关记录。候选预算 = `(limit * 8).max(64)`；
//! 近期记录数 > 预算 ⇒ 关键事实若只走「最新优先」必然被挤掉。
//!
//! # 三条路径都要钉
//!
//! 候选发现有三条实现（`backend/sqlite.rs` · `src/lib.rs` · `backend/in_memory.rs`）
//! ⇒ ⇒ 只修一条 ⇒ ⇒ 落在另外两条上的检索**照样**丢长历史关键事实
//! ⇒ ⇒ 本文件对三条路径跑**同一组断言**（同一用例、同一断言、同一预算）。
//!
//! # 分两层钉（每层各钉各的接线）
//!
//! * **存储层**（`query_candidates` 直接调用）：作用域 / 治理状态 / `as_of` / 预算
//!   四条不变量就落在这条查询上 ⇒ ⇒ 断言直接对候选池取证。
//! * **检索层**（`MemoryCoordinator::recall` 整条正式路径）：用户可见的结果。
//!
//! ⚠️ 检索层的三条不变量用例是**双层守卫**（存储层过滤 + coordinator 过滤）
//! ⇒ ⇒ 单点摘除任一层都**不会**让它们变红 ⇒ ⇒ 它们的单点反证在存储层用例里。
//! 本文件每条断言的反证记录见随本批交付的核查报告。

use std::sync::Arc;

use apeireth_core::kernel::Episode;
use apeireth_memory::backend::in_memory::InMemoryBackend;
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::{
    MemoryCandidateQuery, MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind,
    MemoryProvenance, MemoryRecallQuery, MemoryScope, ScopedMemoryBackend, SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;

const SESSION: &str = "session-long-history";
const OTHER_SESSION: &str = "session-other-scope";
const FACT_ID: &str = "fact-critical-early";
const OUT_OF_SCOPE_FACT_ID: &str = "fact-out-of-scope";
const AFTER_AS_OF_FACT_ID: &str = "fact-after-as-of";
const FACT_TEXT: &str = "用户姓名林知远，约定住址青梧市东湖路。";
const FACT_QUERY: &str = "林知远 青梧市东湖路";
/// 与语料**完全不相交**的问句（字符集与事实文本、近期记录都不相交）⇒ ⇒ 零命中。
const NO_HIT_QUERY: &str = "氢氧氖氩氪氙氚";
const FACT_TS: i64 = 1_600_000_000;
const NOISE_TS: i64 = 1_750_000_000;
const NOISE_COUNT: usize = 100;
const AS_OF_MS: i64 = 1_800_000_000_000;
const QUERY_LIMIT: usize = 5;
const QUERY_MAX_CHARS: usize = 1_000;
/// 候选预算 = `(query.limit * 8).max(64)`（与 coordinator 的口径一致）。
const CANDIDATE_BUDGET: usize = 64;
/// 修法前的候选池 = 最新 `CANDIDATE_BUDGET` 条 ⇒ ⇒ 起点是第 `NOISE_COUNT - 预算` 条。
const FIRST_RECENT_NOISE: usize = NOISE_COUNT - CANDIDATE_BUDGET;
/// 可见历史**未超预算**时的可见行数（远小于 `CANDIDATE_BUDGET`）。
const SMALL_VISIBLE_ROWS: usize = 8;

/// 三条候选发现路径（同一口径的三条实现）。
///
/// # 为什么三条都要跑
///
/// 缺陷是「候选阶段只按最新时间截断」⇒ ⇒ 只要还有一条路径这样，
/// 落在那条路径上的检索就会把长历史关键事实挤掉 ⇒ ⇒ 三条路径必须同口径、同结果。
enum CandidateBackend {
    /// 生产路径（连接池 + 独立 SQL 实现）。
    Sqlite(Arc<SqliteBackend>),
    /// 记忆 crate 质量测试与多条 runtime-assembly 测试喂给 coordinator 的路径。
    Store(Arc<SqliteMemoryStore>),
    /// 内存实现（测试用）。
    Memory(Arc<InMemoryBackend>),
}

impl CandidateBackend {
    /// 路径名（断言消息里标明是哪条路径挂了）。
    fn name(&self) -> &'static str {
        match self {
            CandidateBackend::Sqlite(_) => "SqliteBackend",
            CandidateBackend::Store(_) => "SqliteMemoryStore",
            CandidateBackend::Memory(_) => "InMemoryBackend",
        }
    }

    /// 候选发现入口（三条路径同一 trait）。
    fn scoped(&self) -> &dyn ScopedMemoryBackend {
        match self {
            CandidateBackend::Sqlite(backend) => backend.as_ref(),
            CandidateBackend::Store(backend) => backend.as_ref(),
            CandidateBackend::Memory(backend) => backend.as_ref(),
        }
    }

    /// 把一条记录标成「已遗忘」（治理状态）。
    ///
    /// 内存实现不持有治理行（治理在 `coordinator` 侧）⇒ ⇒ 返回 `false`
    /// ⇒ ⇒ 治理状态用例只跑两条 SQL 路径。
    fn forget(&self, episode_id: &str) -> bool {
        match self {
            CandidateBackend::Sqlite(backend) => {
                backend
                    .forget_episode(episode_id, Some("test-forget"), 0)
                    .unwrap();
                true
            }
            CandidateBackend::Store(backend) => {
                backend
                    .forget_episode(episode_id, Some("test-forget"), 0)
                    .unwrap();
                true
            }
            CandidateBackend::Memory(_) => false,
        }
    }
}

/// 三条路径各跑一遍同一条断言。
macro_rules! across_backends {
    ($assertion:ident) => {{
        let (backend, coordinator) = sqlite_fixture().await;
        $assertion(&backend, &coordinator);
        let (backend, coordinator) = store_fixture();
        $assertion(&backend, &coordinator);
        let (backend, coordinator) = memory_fixture();
        $assertion(&backend, &coordinator);
    }};
}

/// 两条 SQL 路径各跑一遍（治理状态只在有治理行的路径上成立）。
macro_rules! across_sql_backends {
    ($assertion:ident) => {{
        let (backend, coordinator) = sqlite_fixture().await;
        $assertion(&backend, &coordinator);
        let (backend, coordinator) = store_fixture();
        $assertion(&backend, &coordinator);
    }};
}

async fn sqlite_fixture() -> (CandidateBackend, MemoryCoordinator) {
    let pool = Arc::new(SqliteConnectionPool::in_memory().await.unwrap());
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    let backend = Arc::new(SqliteBackend::from_arc(pool.clone()));
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone())
        .with_scoped_backend(backend.clone());
    (CandidateBackend::Sqlite(backend), coordinator)
}

fn store_fixture() -> (CandidateBackend, MemoryCoordinator) {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator =
        MemoryCoordinator::new(store.clone(), store.clone()).with_scoped_backend(store.clone());
    (CandidateBackend::Store(store), coordinator)
}

fn memory_fixture() -> (CandidateBackend, MemoryCoordinator) {
    let backend = Arc::new(InMemoryBackend::new());
    // 内存实现不持有治理行 ⇒ ⇒ 治理面借一份独立 store（治理与候选发现正交）。
    let governance = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator =
        MemoryCoordinator::new(backend.clone(), governance).with_scoped_backend(backend.clone());
    (CandidateBackend::Memory(backend), coordinator)
}

fn provenance(session: &str) -> MemoryProvenance {
    MemoryProvenance {
        source: "test.long_history".into(),
        source_session: Some(session.to_string()),
        source_request: Some("request-long-history".into()),
        source_trace: Some("trace-long-history".into()),
    }
}

fn episode(id: &str, timestamp: i64, content: &str, session: &str) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp,
        role: "user".into(),
        content: content.to_string(),
        session_id: session.to_string(),
    }
}

/// 近期无关记录：字符集与 `FACT_QUERY` 的查询词**不相交** ⇒ 词法分必然为 0。
fn noise_text(index: usize) -> String {
    format!("例行晨会纪要编号{index:03}：同步周报与排期。")
}

fn write_fact(coordinator: &MemoryCoordinator, session: &str, id: &str, text: &str, ts: i64) {
    coordinator
        .writeback_scoped_episode(
            &episode(id, ts, text, session),
            &MemoryScope::session(session.to_string()),
            &provenance(session),
        )
        .unwrap();
}

/// 写一条早期关键事实 + `NOISE_COUNT` 条近期无关记录。
fn seed_fact_and_noise(coordinator: &MemoryCoordinator) {
    write_fact(coordinator, SESSION, FACT_ID, FACT_TEXT, FACT_TS);
    for index in 0..NOISE_COUNT {
        write_fact(
            coordinator,
            SESSION,
            &format!("noise-{index:03}"),
            &noise_text(index),
            NOISE_TS + index as i64,
        );
    }
}

/// 存储层候选查询（直接打 `query_candidates`）。
///
/// 口径与 coordinator 一致：同一预算、同一可见作用域、同一 `as_of`、同一相关词。
fn candidate_query(text: &str, session: &str) -> MemoryCandidateQuery {
    MemoryCandidateQuery {
        source_session: Some(session.to_string()),
        visible_scopes: vec![MemoryScope::session(session.to_string())],
        limit: CANDIDATE_BUDGET,
        as_of_ms: Some(AS_OF_MS),
        query_terms: apeireth_memory::scope::relevance_terms(text),
    }
}

/// 候选池的 id 列表（存储层）。
fn candidate_ids(backend: &CandidateBackend, query: &MemoryCandidateQuery) -> Vec<String> {
    backend
        .scoped()
        .query_candidates(query)
        .unwrap()
        .into_iter()
        .map(|episode| episode.id)
        .collect()
}

fn recall_query(text: &str, session: &str) -> MemoryRecallQuery {
    let mut query = MemoryRecallQuery::new(session.to_string(), text.to_string());
    query.layers = vec![MemoryLayerKind::Episodic];
    query.visible_scopes = vec![MemoryScope::session(session.to_string())];
    query.exact_scope_authorization = false;
    query.limit = QUERY_LIMIT;
    query.max_chars = QUERY_MAX_CHARS;
    query.min_score = 0.1;
    query.as_of_ms = Some(AS_OF_MS);
    query
}

fn recalled_ids(result: &apeireth_memory::MemoryRecallResult) -> Vec<String> {
    result.items.iter().map(|item| item.id.clone()).collect()
}

// ── 存储层：候选池取证 ───────────────────────────────────────────────────────

/// 存储层主用例：早期关键事实必须进入候选池（候选阶段不得只按最新时间截断）。
fn assert_rescues_early_fact(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);

    let ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));

    assert!(
        ids.iter().any(|id| id == FACT_ID),
        "[{}] 早期关键事实不在候选池里（候选阶段被近期记录挤掉）; candidate ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn candidate_pool_rescues_early_fact_when_recent_records_drown_it() {
    across_backends!(assert_rescues_early_fact);
}

/// 存储层作用域不变量：作用域外的早期事实**不得**因为「相关」被捞进候选池。
fn assert_respects_scope(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    // 同样「相关」的内容，但写在另一个作用域里。
    write_fact(
        coordinator,
        OTHER_SESSION,
        OUT_OF_SCOPE_FACT_ID,
        FACT_TEXT,
        FACT_TS,
    );

    let ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));

    assert!(
        !ids.iter().any(|id| id == OUT_OF_SCOPE_FACT_ID),
        "[{}] 作用域外的早期事实进了候选池（作用域被候选阶段绕过）; candidate ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn candidate_pool_respects_scope() {
    across_backends!(assert_respects_scope);
}

/// 存储层治理状态不变量：已遗忘的早期事实**不得**因为「相关」被捞进候选池。
fn assert_excludes_forgotten(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    assert!(backend.forget(FACT_ID), "该路径应承载治理状态");

    let ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));

    assert!(
        !ids.iter().any(|id| id == FACT_ID),
        "[{}] 已遗忘的早期事实进了候选池（治理状态被候选阶段绕过）; candidate ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn candidate_pool_excludes_forgotten_episodes() {
    across_sql_backends!(assert_excludes_forgotten);
}

/// 存储层 `as_of` 不变量：晚于 `as_of` 的事实**不得**因为「相关」被捞进候选池。
fn assert_respects_as_of(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    // 同一会话、同样的「相关」内容，但时间晚于 as_of。
    write_fact(
        coordinator,
        SESSION,
        AFTER_AS_OF_FACT_ID,
        FACT_TEXT,
        AS_OF_MS / 1000 + 60,
    );

    let ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));

    assert!(
        !ids.iter().any(|id| id == AFTER_AS_OF_FACT_ID),
        "[{}] 晚于 as_of 的事实进了候选池（时间过滤被候选阶段绕过）; candidate ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn candidate_pool_respects_as_of() {
    across_backends!(assert_respects_as_of);
}

/// 存储层预算不变量：候选池**总量**不得超过预算，且新鲜度切片没被饿死。
fn assert_stays_within_budget(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);

    let ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));

    assert!(
        ids.len() <= CANDIDATE_BUDGET,
        "[{}] 候选池超过预算（候选阶段被放宽）; len = {}, budget = {CANDIDATE_BUDGET}",
        backend.name(),
        ids.len()
    );
    assert!(
        ids.iter().any(|id| id == "noise-099"),
        "[{}] 最新记录不在候选池里（新鲜度切片被饿死）; candidate ids = {ids:?}",
        backend.name()
    );
    assert!(
        ids.iter().any(|id| id == "noise-050"),
        "[{}] 近期窗口中段的记录不在候选池里（新鲜度切片被饿死）; candidate ids = {ids:?}",
        backend.name()
    );
    assert!(
        ids.iter().any(|id| id == FACT_ID),
        "[{}] 早期关键事实不在候选池里（相关度切片没生效）; candidate ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn candidate_pool_stays_within_budget_and_keeps_recent_window() {
    across_backends!(assert_stays_within_budget);
}

/// 零命中 ⇒ 候选池与**修法前**逐条同 id 同序。
///
/// # 为什么这条要钉
///
/// 相关度切片若允许零命中的行进来 ⇒ ⇒ 它们必然是新鲜度切片的**子集**（同一份
/// `WHERE`、同一个排序键、条数更少）⇒ ⇒ 合并阶段按 id 去重时被折叠掉
/// ⇒ ⇒ 候选池**内容**看不出差别，只是白白多取多排一遍。
/// ⇒ ⇒ 这里钉住「零命中时相关度切片为空 ⇒ 候选池与修法前完全一致」。
fn assert_no_hit_pool_matches_pre_fix(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);

    let ids = candidate_ids(backend, &candidate_query(NO_HIT_QUERY, SESSION));
    let expected: Vec<String> = (FIRST_RECENT_NOISE..NOISE_COUNT)
        .map(|index| format!("noise-{index:03}"))
        .collect();

    assert_eq!(
        ids,
        expected,
        "[{}] 零命中时候选池与修法前不一致（相关度切片不应带进零命中行）",
        backend.name()
    );
}

#[tokio::test]
async fn no_hit_candidate_pool_matches_pre_fix_pool() {
    across_backends!(assert_no_hit_pool_matches_pre_fix);
}

/// 可见历史**未超预算**时候选池必须等于全部可见行（逐条同 id 同序）。
///
/// # 为什么钉这一条
///
/// 可见历史没超预算 ⇒ ⇒ 新鲜度切片**已经**覆盖了全部可见行 ⇒ ⇒ 相关度切片
/// 只可能返回**已经在池子里**的行 ⇒ ⇒ 跳过它不改变任何一条结果
/// ⇒ ⇒ 池子既与「修法前」逐条一致，也**与问句无关**（命中与否都一个样）。
/// ⇒ ⇒ 这条同时钉住两件事：候选池不被截断、且跳过相关度切片不丢结果。
fn assert_small_history_pool_is_complete(
    backend: &CandidateBackend,
    coordinator: &MemoryCoordinator,
) {
    write_fact(coordinator, SESSION, FACT_ID, FACT_TEXT, FACT_TS);
    for index in 0..SMALL_VISIBLE_ROWS {
        write_fact(
            coordinator,
            SESSION,
            &format!("noise-{index:03}"),
            &noise_text(index),
            NOISE_TS + index as i64,
        );
    }

    let hit_ids = candidate_ids(backend, &candidate_query(FACT_QUERY, SESSION));
    let no_hit_ids = candidate_ids(backend, &candidate_query(NO_HIT_QUERY, SESSION));

    let mut expected = vec![FACT_ID.to_string()];
    expected.extend((0..SMALL_VISIBLE_ROWS).map(|index| format!("noise-{index:03}")));
    assert_eq!(
        hit_ids,
        expected,
        "[{}] 可见历史未超预算时候选池必须等于全部可见行（逐条同 id 同序）",
        backend.name()
    );
    assert_eq!(
        no_hit_ids,
        hit_ids,
        "[{}] 可见历史未超预算时候选池不得随问句变化（相关度切片只可能返回池内已有行）",
        backend.name()
    );
}

#[tokio::test]
async fn small_visible_history_pool_contains_every_visible_row() {
    across_backends!(assert_small_history_pool_is_complete);
}

/// 三条路径必须给出**同一个**候选池（同 id、同序）。
#[tokio::test]
async fn three_backends_agree_on_candidate_pool() {
    let (sqlite, coordinator) = sqlite_fixture().await;
    seed_fact_and_noise(&coordinator);
    let sqlite_ids = candidate_ids(&sqlite, &candidate_query(FACT_QUERY, SESSION));

    let (store, coordinator) = store_fixture();
    seed_fact_and_noise(&coordinator);
    let store_ids = candidate_ids(&store, &candidate_query(FACT_QUERY, SESSION));

    let (memory, coordinator) = memory_fixture();
    seed_fact_and_noise(&coordinator);
    let memory_ids = candidate_ids(&memory, &candidate_query(FACT_QUERY, SESSION));

    assert_eq!(
        sqlite_ids, store_ids,
        "SqliteBackend 与 SqliteMemoryStore 的候选池不一致"
    );
    assert_eq!(
        sqlite_ids, memory_ids,
        "SqliteBackend 与 InMemoryBackend 的候选池不一致"
    );
}

// ── 检索层：正式召回路径 ───────────────────────────────────────────────────

/// 检索层主用例：早期关键事实必须仍然被召回（候选阶段不得只按最新时间截断）。
fn assert_recalls_early_fact(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);

    let query = recall_query(FACT_QUERY, SESSION);
    let result = coordinator.recall(&query).unwrap();
    let ids = recalled_ids(&result);

    assert!(
        ids.iter().any(|id| id == FACT_ID),
        "[{}] 早期关键事实未被召回（候选阶段被近期记录挤掉）; recalled ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn early_critical_fact_is_recalled_when_recent_records_drown_it() {
    across_backends!(assert_recalls_early_fact);
}

/// 检索层预算不变量：候选预算与交付预算都**不得**因为候选阶段变宽而放宽。
fn assert_recall_budget(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);

    let query = recall_query(FACT_QUERY, SESSION);
    let result = coordinator.recall(&query).unwrap();

    assert!(
        result.total_candidates <= CANDIDATE_BUDGET,
        "[{}] 候选池超过预算（候选阶段被放宽）; total_candidates = {}",
        backend.name(),
        result.total_candidates
    );
    assert!(
        result.items.len() <= QUERY_LIMIT,
        "[{}] 交付条数超过预算; items = {}",
        backend.name(),
        result.items.len()
    );
    assert!(
        result.total_chars <= QUERY_MAX_CHARS,
        "[{}] 交付字数超过预算; total_chars = {}",
        backend.name(),
        result.total_chars
    );
}

#[tokio::test]
async fn recall_budget_is_not_widened() {
    across_backends!(assert_recall_budget);
}

/// 检索层：近期命中记录仍走「最新优先」这条切片（修法不得把近期上下文挤掉）。
fn assert_recent_record_still_recalled(
    backend: &CandidateBackend,
    coordinator: &MemoryCoordinator,
) {
    seed_fact_and_noise(coordinator);

    let query = recall_query("编号042", SESSION);
    let result = coordinator.recall(&query).unwrap();
    let ids = recalled_ids(&result);

    assert!(
        ids.iter().any(|id| id == "noise-042"),
        "[{}] 近期命中记录未被召回; recalled ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn recent_record_matching_the_query_is_still_recalled() {
    across_backends!(assert_recent_record_still_recalled);
}

/// 检索层治理状态不变量：已遗忘的早期事实**不得**因为「相关」被捞回来。
fn assert_forgotten_not_rescued(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    assert!(backend.forget(FACT_ID), "该路径应承载治理状态");

    let query = recall_query(FACT_QUERY, SESSION);
    let result = coordinator.recall(&query).unwrap();
    let ids = recalled_ids(&result);

    assert!(
        !ids.iter().any(|id| id == FACT_ID),
        "[{}] 已遗忘的早期事实被捞回来了（治理状态被候选阶段绕过）; recalled ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn forgotten_early_fact_is_not_rescued() {
    across_sql_backends!(assert_forgotten_not_rescued);
}

/// 检索层作用域不变量：作用域外的早期事实**不得**因为「相关」被捞进来。
fn assert_out_of_scope_not_rescued(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    // 同样「相关」的内容，但写在另一个作用域里。
    write_fact(
        coordinator,
        OTHER_SESSION,
        OUT_OF_SCOPE_FACT_ID,
        FACT_TEXT,
        FACT_TS,
    );

    let query = recall_query(FACT_QUERY, SESSION);
    let result = coordinator.recall(&query).unwrap();
    let ids = recalled_ids(&result);

    assert!(
        !ids.iter().any(|id| id == OUT_OF_SCOPE_FACT_ID),
        "[{}] 作用域外的早期事实被捞进来了（作用域被候选阶段绕过）; recalled ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn out_of_scope_early_fact_is_not_rescued() {
    across_backends!(assert_out_of_scope_not_rescued);
}

/// 检索层 `as_of` 不变量：晚于 `as_of` 的事实**不得**因为「相关」被捞进来。
fn assert_after_as_of_not_rescued(backend: &CandidateBackend, coordinator: &MemoryCoordinator) {
    seed_fact_and_noise(coordinator);
    // 同一会话、同样的「相关」内容，但时间晚于 as_of。
    write_fact(
        coordinator,
        SESSION,
        AFTER_AS_OF_FACT_ID,
        FACT_TEXT,
        AS_OF_MS / 1000 + 60,
    );

    let query = recall_query(FACT_QUERY, SESSION);
    let result = coordinator.recall(&query).unwrap();
    let ids = recalled_ids(&result);

    assert!(
        !ids.iter().any(|id| id == AFTER_AS_OF_FACT_ID),
        "[{}] 晚于 as_of 的事实被捞回来了（时间过滤被候选阶段绕过）; recalled ids = {ids:?}",
        backend.name()
    );
}

#[tokio::test]
async fn fact_after_as_of_is_not_rescued() {
    across_backends!(assert_after_as_of_not_rescued);
}
