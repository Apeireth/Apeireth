//! W3-① 多户数据隔离 (星群数据层) 的隔离证据套件。
//!
//! 钉死的行为面: 域创建 / 单户兼容采用零迁移 / 三面 (会话·记忆·配额) 跨域读拒
//! 写拒 / 越权不泄露 (拒绝帧恒定) / 域上下文路由 (多域并发互不串) / 配额账本
//! 分域独立 / 自述域节 / 坏域配置拒开 / 单户路径零回归。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::kernel::{system_clock, Episode, SessionId};
use apeireth_core::user_domain::{
    DomainRegistry, DomainRole, DomainUserId, UserDomainError, DEFAULT_DOMAIN_ID,
};
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::SqliteMemoryStore;
use apeireth_runtime::canonical::{Session, SessionManager, SessionStore};
use apeireth_runtime_assembly::{
    domain_status_probe, session_row_count, DefaultDomainStores, DomainDataPlane,
    ProductionSelfStatusSource, SqliteSessionStore,
};

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-domain-isolation-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 造一个「既有单户数据面」: 根上已有一个会话库 (含一条会话) 与记忆库。
async fn legacy_root_with_one_session(tag: &str) -> (PathBuf, SessionId) {
    let root = temp_root(tag);
    let store = SqliteSessionStore::open(root.join("sessions.sqlite3"))
        .await
        .unwrap();
    let clock = system_clock();
    let session = Session::new(SessionId::new(), clock.as_ref());
    let id = session.id;
    store.save(&session).await.unwrap();
    drop(store);
    let memory = SqliteMemoryStore::open(root.join("cognitive.sqlite3")).unwrap();
    drop(memory);
    (root, id)
}

/// 默认域句柄注入 (组合根形态: 与运行时共用同一把会话写闸)。
async fn default_stores(root: &Path) -> DefaultDomainStores {
    let store = SqliteSessionStore::open(root.join("sessions.sqlite3"))
        .await
        .unwrap();
    let sessions = Arc::new(SessionManager::new(Arc::new(store), system_clock()));
    let memory = Arc::new(SqliteMemoryStore::open(root.join("cognitive.sqlite3")).unwrap());
    DefaultDomainStores {
        sessions,
        memory: Arc::clone(&memory),
        memory_backend: Arc::new(SqliteBackend::from_arc(Arc::new(
            apeireth_storage::SqliteConnectionPool::open(root.join("cognitive.sqlite3"))
                .await
                .unwrap(),
        ))),
        sessions_db: root.join("sessions.sqlite3"),
        memory_db: root.join("cognitive.sqlite3"),
    }
}

async fn open_plane(root: &Path) -> DomainDataPlane {
    let registry = DomainRegistry::open(root).unwrap();
    DomainDataPlane::open(registry, default_stores(root).await)
        .await
        .unwrap()
}

fn new_session() -> Session {
    Session::new(SessionId::new(), system_clock().as_ref())
}

fn new_episode(id: &str, session_id: &str) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp: 1_728_000_000_000,
        role: "user".to_string(),
        content: format!("memory of {session_id}"),
        session_id: session_id.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 域创建
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_created_domain_gets_its_own_three_store_files_under_its_data_dir() {
    let root = temp_root("create");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Member).unwrap();
    let plane = open_plane(&root).await;

    let alice = plane.resolve(Some("alice")).unwrap();
    let handles = plane.handles(&alice).unwrap();
    let session = new_session();
    handles.sessions.save(&session).await.unwrap();
    handles
        .memory
        .backend
        .put_episode(&new_episode("ep-1", "s-1"))
        .unwrap();

    // 三面分库: 该域自己的库文件在自己的数据目录下。
    assert_eq!(
        handles.sessions_db,
        root.join("users/alice/data/sessions.sqlite3"),
        "会话库 (含三维配额账本) = 域自己的库文件"
    );
    assert!(root.join("users/alice/data/sessions.sqlite3").is_file());
    assert!(root.join("users/alice/data/cognitive.sqlite3").is_file());
    assert!(root.join("users/bob/data/sessions.sqlite3").is_file());
    assert_eq!(session_row_count(handles.sessions_db).unwrap(), 1);
    assert!(handles
        .memory
        .backend
        .get_episode("ep-1")
        .unwrap()
        .is_some());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 兼容迁移 (采用): 现有数据入 default 域零丢失
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_legacy_data_root_is_adopted_in_place_and_served_unchanged() {
    let (root, legacy_id) = legacy_root_with_one_session("adopt").await;
    let before_sessions = std::fs::read(root.join("sessions.sqlite3")).unwrap();
    let before_memory = std::fs::read(root.join("cognitive.sqlite3")).unwrap();

    let plane = open_plane(&root).await;

    // 对抗断言: 存量库文件原位、逐字节不变 (零迁移零丢失)。
    assert_eq!(
        std::fs::read(root.join("sessions.sqlite3")).unwrap(),
        before_sessions,
        "会话库不得被移动/改写"
    );
    assert_eq!(
        std::fs::read(root.join("cognitive.sqlite3")).unwrap(),
        before_memory,
        "记忆库不得被移动/改写"
    );

    // 默认域 = 原位数据面: 既有会话在默认域句柄下原样可读 (内容不丢)。
    let default = plane.resolve(None).unwrap();
    assert!(default.is_default());
    let handles = plane.handles(&default).unwrap();
    let loaded = handles
        .sessions
        .load(&legacy_id)
        .await
        .unwrap()
        .expect("既有会话必须在默认域原样可见");
    assert_eq!(loaded.id, legacy_id);
    assert_eq!(
        handles.sessions_db,
        root.join("sessions.sqlite3"),
        "默认域数据面 = 数据根原位"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 三面跨域读拒 / 写拒
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cross_domain_reads_and_writes_are_refused_on_all_three_planes() {
    let root = temp_root("cross");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Owner).unwrap();
    let plane = Arc::new(
        DomainDataPlane::open(registry, default_stores(&root).await)
            .await
            .unwrap(),
    );
    let alice = plane.resolve(Some("alice")).unwrap();
    let bob = plane.resolve(Some("bob")).unwrap();

    // bob 的三面数据 (会话 + 配额记账 + 记忆)。
    let bob_handles = plane.handles(&bob).unwrap();
    let mut bob_session = new_session();
    let bob_id = bob_session.id;
    bob_session.quota.consume_tokens(120);
    bob_handles.sessions.save(&bob_session).await.unwrap();
    bob_handles
        .memory
        .backend
        .put_episode(&new_episode("ep-bob", "bob-s"))
        .unwrap();
    let bob_row_before = std::fs::read(bob_handles.sessions_db).unwrap();

    // 会话面·跨域读拒: 本域句柄查他域会话 = 不存在 (拒绝即帧)。
    let alice_handles = plane.handles(&alice).unwrap();
    assert!(
        alice_handles
            .sessions
            .load(&bob_id)
            .await
            .unwrap()
            .is_none(),
        "他域会话按本域句柄必须读作不存在"
    );
    // 记忆面·跨域读拒。
    assert!(
        alice_handles
            .memory
            .backend
            .get_episode("ep-bob")
            .unwrap()
            .is_none(),
        "他域记忆按本域句柄必须读作不存在"
    );
    // 配额面·跨域读拒: 本域配额探测读不到他域账本。
    let alice_probe =
        apeireth_runtime_assembly::session_quota_probe(alice_handles.sessions_db.to_path_buf());
    assert!(
        alice_probe(bob_id).is_err(),
        "他域会话的配额账本按本域探测口必须读作不存在"
    );

    // 显式点名他域 = 域级越权 → Forbidden (存在与否同帧)。
    for caller in [&alice, &bob] {
        let target = if caller.user_id == alice.user_id {
            &bob.user_id
        } else {
            &alice.user_id
        };
        let err = plane.handles_for(caller, target).unwrap_err();
        assert!(matches!(err, UserDomainError::Forbidden(_)), "{err}");
    }

    // 写拒: alice 用同 id 写自己的库, 他域账本零改动 (对抗断言)。
    let mut impersonated = new_session();
    impersonated.id = bob_id;
    impersonated.quota.consume_tokens(9_999);
    alice_handles.sessions.save(&impersonated).await.unwrap();
    assert_eq!(
        std::fs::read(bob_handles.sessions_db).unwrap(),
        bob_row_before,
        "跨域写必须被隔离在他域账本之外 (逐字节零改动)"
    );
    // alice 自己的库里确有这条 (隔离语义: 写发生在本域, 不是静默丢弃)。
    let alice_copy = alice_handles
        .sessions
        .load(&bob_id)
        .await
        .unwrap()
        .expect("本域写入应落在本域");
    assert_eq!(alice_copy.quota.consumed_tokens, 9_999);
    // bob 的账本读数不受影响 (配额面·跨域写拒)。
    let bob_copy = bob_handles.sessions.load(&bob_id).await.unwrap().unwrap();
    assert_eq!(bob_copy.quota.consumed_tokens, 120);
    // 记忆面·跨域写拒: alice 写同 id 记忆只落本域。
    alice_handles
        .memory
        .backend
        .put_episode(&new_episode("ep-bob", "alice-s"))
        .unwrap();
    assert!(
        bob_handles
            .memory
            .backend
            .get_episode("ep-bob")
            .unwrap()
            .is_some(),
        "bob 自己的记忆仍在 (未被覆盖)"
    );
    let bob_episode = bob_handles
        .memory
        .backend
        .get_episode("ep-bob")
        .unwrap()
        .unwrap();
    assert_eq!(bob_episode.session_id, "bob-s", "他域记忆内容零改动");
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 越权不泄露 (拒绝帧恒定)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cross_domain_refusals_hide_target_existence_behind_one_frame() {
    let root = temp_root("no-leak");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Owner).unwrap();
    let plane = open_plane(&root).await;
    let alice = plane.resolve(Some("alice")).unwrap();

    let existing = DomainUserId::new("bob").unwrap();
    let missing = DomainUserId::new("nobody").unwrap();
    let refused_existing = plane.handles_for(&alice, &existing).unwrap_err();
    let refused_missing = plane.handles_for(&alice, &missing).unwrap_err();

    // 404/403 语义钉死 + 存在性不泄露: 目标存在与否不得改变帧形态或帧文本。
    assert_eq!(refused_existing.http_status(), 403);
    assert_eq!(refused_missing.http_status(), 403);
    assert_eq!(
        refused_existing.to_string(),
        refused_missing.to_string(),
        "越权帧必须恒定: 他域存在与否不得外泄"
    );
    // 资源级 (他域资源) 读作不存在 → 404 语义。
    let err = UserDomainError::NotFound("invisible".to_string());
    assert_eq!(err.http_status(), 404);
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 配额账本分域独立
// ---------------------------------------------------------------------------

#[tokio::test]
async fn quota_ledgers_stay_independent_per_domain_for_the_same_session_id() {
    let root = temp_root("quota");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Owner).unwrap();
    let plane = open_plane(&root).await;
    let alice = plane.resolve(Some("alice")).unwrap();
    let bob = plane.resolve(Some("bob")).unwrap();
    let shared_id = SessionId::new();

    // 同一 session id 在两域各有一份账本; 只在 alice 侧记账。
    let alice_handles = plane.handles(&alice).unwrap();
    let mut in_alice = new_session();
    in_alice.id = shared_id;
    in_alice.quota.consume_tokens(120);
    in_alice.quota.consume_tool_step();
    in_alice.quota.consume_cost_micros(2_000);
    alice_handles.sessions.save(&in_alice).await.unwrap();

    let bob_handles = plane.handles(&bob).unwrap();
    let mut in_bob = new_session();
    in_bob.id = shared_id;
    bob_handles.sessions.save(&in_bob).await.unwrap();

    // 分域独立: 各域探测口只读到本域账本。
    let alice_probe =
        apeireth_runtime_assembly::session_quota_probe(alice_handles.sessions_db.to_path_buf());
    let bob_probe =
        apeireth_runtime_assembly::session_quota_probe(bob_handles.sessions_db.to_path_buf());
    let alice_reading = alice_probe(shared_id).unwrap();
    assert_eq!(alice_reading.consumed_tokens, 120);
    assert_eq!(alice_reading.consumed_tool_steps, 1);
    assert_eq!(alice_reading.consumed_cost_micros, 2_000);
    let bob_reading = bob_probe(shared_id).unwrap();
    assert_eq!(bob_reading.consumed_tokens, 0, "他域记账绝不串账");
    assert_eq!(bob_reading.consumed_tool_steps, 0);
    assert_eq!(bob_reading.consumed_cost_micros, 0);
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 域上下文路由 (多域并发互不串)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_domain_traffic_never_cross_talks_between_domains() {
    let root = temp_root("routing");
    let registry = DomainRegistry::open(&root).unwrap();
    for (user_id, name) in [("alice", "Alice"), ("bob", "Bob"), ("carol", "Carol")] {
        registry.create(user_id, name, DomainRole::Owner).unwrap();
    }
    let plane = Arc::new(
        DomainDataPlane::open(registry, default_stores(&root).await)
            .await
            .unwrap(),
    );

    let mut tasks = Vec::new();
    for user_id in ["alice", "bob", "carol"] {
        let plane = Arc::clone(&plane);
        tasks.push(tokio::spawn(async move {
            let domain = plane.resolve(Some(user_id)).unwrap();
            let handles = plane.handles(&domain).unwrap();
            for round in 0..8usize {
                let mut session = new_session();
                session.quota.consume_tokens(round + 1);
                let id = session.id;
                handles.sessions.save(&session).await.unwrap();
                handles
                    .memory
                    .backend
                    .put_episode(&new_episode(
                        &format!("{user_id}-ep-{round}"),
                        &id.to_string(),
                    ))
                    .unwrap();
            }
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }

    // 互不串: 每域恰 8 条会话, 且谁也读不到别人的东西。
    for user_id in ["alice", "bob", "carol"] {
        let domain = plane.resolve(Some(user_id)).unwrap();
        let handles = plane.handles(&domain).unwrap();
        assert_eq!(
            session_row_count(handles.sessions_db).unwrap(),
            8,
            "{user_id} 域内应恰 8 条会话"
        );
        // 域内记忆只在本域可见。
        assert!(handles
            .memory
            .backend
            .get_episode(&format!("{user_id}-ep-0"))
            .unwrap()
            .is_some());
    }
    // 跨域读: 用本域句柄查不到别域的记忆 id。
    let alice = plane.resolve(Some("alice")).unwrap();
    let alice_handles = plane.handles(&alice).unwrap();
    assert!(alice_handles
        .memory
        .backend
        .get_episode("bob-ep-0")
        .unwrap()
        .is_none());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 自述域节
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_self_status_domain_section_reports_the_current_domain_truthfully() {
    let root = temp_root("self-status");
    let registry = Arc::new(DomainRegistry::open(&root).unwrap());
    registry
        .create("alice", "Alice", DomainRole::Member)
        .unwrap();
    let plane = open_plane(&root).await;
    let alice = plane.resolve(Some("alice")).unwrap();
    let created_at = alice.created_at;
    let handles = plane.handles(&alice).unwrap();
    for _ in 0..2 {
        handles.sessions.save(&new_session()).await.unwrap();
    }
    handles
        .memory
        .backend
        .put_episode(&new_episode("ep-a", "s-a"))
        .unwrap();

    let probe = domain_status_probe(
        registry,
        DomainUserId::new("alice").unwrap(),
        handles.sessions_db.to_path_buf(),
        Arc::clone(&handles.memory.store),
    );
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None).with_domain_probe(probe);
    let snapshot = apeireth_tools_canonical::SelfStatusSource::snapshot(&source);
    let domain = snapshot.domain.expect("域节必须在场");
    assert_eq!(domain.user_id, "alice");
    assert_eq!(domain.display_name, "Alice");
    assert_eq!(domain.role, "member");
    assert_eq!(domain.created_at, created_at);
    assert_eq!(domain.counts.sessions, Some(2), "域内会话数如实");
    assert_eq!(domain.counts.memories, Some(1), "域内记忆数如实");
    assert_eq!(domain.counts.protected, Some(0));
    assert_eq!(domain.counts.reason, None);

    // 未接探测口 = 显式 null + 原因 (不编域)。
    let bare = ProductionSelfStatusSource::new(Vec::new(), false, None);
    let bare_snapshot = apeireth_tools_canonical::SelfStatusSource::snapshot(&bare);
    assert!(bare_snapshot.domain.is_none());
    assert!(bare_snapshot.domain_reason.is_some());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 坏域配置拒开
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_bad_domain_config_refuses_only_its_own_plane_and_touches_nothing() {
    let root = temp_root("bad-config");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Owner).unwrap();
    // 坏域配置 (畸形标记) 在平面打开前出现: 该域拒开, 其余照常。
    std::fs::write(root.join("users/alice/domain.json"), b"{ not json").unwrap();
    let marker_before = std::fs::read(root.join("users/alice/domain.json")).unwrap();

    let plane = open_plane(&root).await;
    let err = plane.resolve(Some("alice")).unwrap_err();
    assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
    assert_eq!(err.http_status(), 403);
    let bob = plane.resolve(Some("bob")).unwrap();
    let bob_handles = plane.handles(&bob).unwrap();
    bob_handles.sessions.save(&new_session()).await.unwrap();
    assert_eq!(
        std::fs::read(root.join("users/alice/domain.json")).unwrap(),
        marker_before,
        "拒开路径不得改动坏配置文件"
    );
    // alice 的坏域不产生任何数据面文件。
    assert!(!root.join("users/alice/data/sessions.sqlite3").exists());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 单户路径零回归
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_single_user_composition_stays_on_the_legacy_paths_without_any_domain_setup() {
    let (root, legacy_id) = legacy_root_with_one_session("single-user").await;
    let plane = open_plane(&root).await;

    // 无用户 = 默认域; 未开域的绑定用户也留默认域 (兼容延伸)。
    assert!(plane.resolve(None).unwrap().is_default());
    assert!(plane.resolve(Some("stranger")).unwrap().is_default());
    let default = plane.resolve(None).unwrap();
    let handles = plane.handles(&default).unwrap();

    // 会话照旧落在既有路径 (APEIRETH_SESSION_DB 语义逐字不变)。
    let mut session = new_session();
    session.quota.consume_tokens(7);
    handles.sessions.save(&session).await.unwrap();
    assert_eq!(
        session_row_count(&root.join("sessions.sqlite3")).unwrap(),
        2
    );
    assert!(
        handles.sessions.load(&legacy_id).await.unwrap().is_some(),
        "既有会话零回归"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 域解析: 坏绑定拒开 (不猜)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_invalid_bound_user_id_is_refused_even_when_everything_else_is_fine() {
    let root = temp_root("bad-binding");
    let plane = open_plane(&root).await;
    let err = plane.resolve(Some("../evil")).unwrap_err();
    assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
    let err = plane.resolve(Some("A/B")).unwrap_err();
    assert!(matches!(err, UserDomainError::Refused(_)), "{err}");
    // 默认域唯一常量与解析一致 (口径唯一)。
    let default = plane.resolve(Some(DEFAULT_DOMAIN_ID)).unwrap();
    assert_eq!(default.user_id.as_str(), DEFAULT_DOMAIN_ID);
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// W3-③ 逐域运行时隔离 (星群正身): 模型 / 记忆 / 配额 / 写闸 四面
// ---------------------------------------------------------------------------

#[tokio::test]
async fn per_domain_runtime_isolation_holds_on_all_four_faces() {
    use apeireth_runtime::canonical::{DomainRuntimeContext, DomainRuntimePlane, Runtime};
    use std::collections::BTreeMap;
    use std::time::Duration;

    let root = temp_root("rt-four-faces");
    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry.create("bob", "Bob", DomainRole::Member).unwrap();
    let plane = open_plane(&root).await;

    // 伙伴实例面: 每域自己的运行时实例 (默认域 = 注入的现运行时)。
    let default_runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let alice_runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let bob_runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut instances = BTreeMap::new();
    let alice_id = DomainUserId::new("alice").unwrap();
    let bob_id = DomainUserId::new("bob").unwrap();
    instances.insert(alice_id.clone(), alice_runtime.clone());
    instances.insert(bob_id.clone(), bob_runtime.clone());
    let runtime_plane = DomainRuntimePlane::new(
        plane.sessions_plane().clone(),
        default_runtime.clone(),
        instances,
    );

    let alice = plane.resolve(Some("alice")).unwrap();
    let bob = plane.resolve(Some("bob")).unwrap();
    let default = plane.resolve(None).unwrap();
    let registry = plane.sessions_plane().registry().clone();

    // 面① 模型: 域覆盖各归各, 对抗改写互不渗透。
    registry
        .set_runtime_overrides(
            &alice_id,
            Some("Alice 的专属伙伴".to_string()),
            Some("model-alice".to_string()),
        )
        .unwrap();
    registry
        .set_runtime_overrides(&bob_id, None, Some("model-bob".to_string()))
        .unwrap();
    assert_eq!(
        runtime_plane.context_for(&alice).unwrap(),
        DomainRuntimeContext {
            model: Some("model-alice".to_string()),
            persona_text: Some("Alice 的专属伙伴".to_string()),
        }
    );
    assert_eq!(
        runtime_plane.context_for(&bob).unwrap().model.as_deref(),
        Some("model-bob")
    );
    // 对抗: 连改 alice 的模型, bob 与默认域的解析逐字不变。
    let bob_context_before = runtime_plane.context_for(&bob).unwrap();
    let default_context_before = runtime_plane.context_for(&default).unwrap();
    for model in ["model-alice-2", "model-alice-3"] {
        registry
            .set_runtime_overrides(&alice_id, None, Some(model.to_string()))
            .unwrap();
        assert_eq!(
            runtime_plane.context_for(&bob).unwrap(),
            bob_context_before,
            "域 A 的模型配置不得影响域 B"
        );
        assert_eq!(
            runtime_plane.context_for(&default).unwrap(),
            default_context_before,
            "域 A 的模型配置不得影响默认域"
        );
    }

    // 面② 记忆: 两域记忆库互不可见 (他域记忆按本域句柄读作不存在)。
    let alice_handles = plane.handles(&alice).unwrap();
    let bob_handles = plane.handles(&bob).unwrap();
    alice_handles
        .memory
        .backend
        .put_episode(&new_episode("ep-a", "s-a"))
        .unwrap();
    assert!(alice_handles
        .memory
        .backend
        .get_episode("ep-a")
        .unwrap()
        .is_some());
    assert!(
        bob_handles
            .memory
            .backend
            .get_episode("ep-a")
            .unwrap()
            .is_none(),
        "他域记忆必须读作不存在 (分库隔离)"
    );

    // 面③ 配额: 同一 session id, 两域账本互不串。
    let sid = SessionId::new();
    let mut in_alice = Session::new(sid, system_clock().as_ref());
    in_alice.quota.consume_tokens(120);
    in_alice.quota.consume_tool_step();
    alice_handles.sessions.save(&in_alice).await.unwrap();
    let in_bob = Session::new(sid, system_clock().as_ref());
    bob_handles.sessions.save(&in_bob).await.unwrap();
    let alice_copy = alice_handles.sessions.load(&sid).await.unwrap().unwrap();
    let bob_copy = bob_handles.sessions.load(&sid).await.unwrap().unwrap();
    assert_eq!(alice_copy.quota.consumed_tokens, 120);
    assert_eq!(alice_copy.quota.consumed_tool_steps, 1);
    assert_eq!(bob_copy.quota.consumed_tokens, 0);
    assert_eq!(bob_copy.quota.consumed_tool_steps, 0);

    // 面④ 写闸: 每域恰一把 (实例互不相同, 也不与默认域共用);
    // 本域持证不阻塞他域写 (域间并发, 域内串行由 W2-③ 套件钉住)。
    assert!(!Arc::ptr_eq(alice_handles.sessions, bob_handles.sessions));
    assert!(!Arc::ptr_eq(
        alice_handles.sessions,
        plane.handles(&default).unwrap().sessions
    ));
    let alice_permit = alice_handles.sessions.write_permit(sid).await;
    let bob_permit = tokio::time::timeout(
        Duration::from_millis(250),
        bob_handles.sessions.write_permit(SessionId::new()),
    )
    .await;
    assert!(bob_permit.is_ok(), "本域持证不得阻塞他域写闸 (跨域互不锁)");
    drop(alice_permit);
    let _ = std::fs::remove_dir_all(&root);
}
