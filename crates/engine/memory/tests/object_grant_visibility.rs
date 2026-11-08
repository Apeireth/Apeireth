//! C-D1 记忆**对象级授权**的读路径并联 + 账本读端加固的行为证据套件。
//!
//! # 语义口径（lead 拍板，**不得改**）：**叠加 ≠ 替代**
//!
//! ```text
//! 读可见 = 域可见 ∨ (存在有效对象 grant)
//! ```
//!
//! 钉死的行为面：
//!
//! * **读路径并联**：被授对象**能进候选集**、**能被检索到**（域外的那几条）；
//! * **零变化**：无授权时读路径**逐字节同行为**（老判定退化）；
//! * **不扩域**：授权只对**那一个 object id** 生效；
//! * **撤销即时生效**（现读账本，无缓存 / 无 TTL / 无轮询）；
//! * **人批前零披露**（`pending` 不生效）；
//! * **账本读端加固**：重放旧行 / 伪造 active 行 / 非法 version ⇒ 按不存在处理。
//!
//! ⚠️ 本套件只覆盖**记忆 crate 内**的读路径与账本；治理档（逐次人批）与模型面
//! 工具在 `crates/engine/runtime-assembly/src/canonical/object_grant.rs`。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{Clock, Episode, SystemClock};
use apeireth_memory::backend::in_memory::InMemoryBackend;
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::object_grant::{
    GrantAuditKind, GrantStatus, MemoryObjectResolver, ObjectGrantLedger, ObjectGrantRegistry,
};
use apeireth_memory::scope::{
    sanitize_granted_object_ids, ScopedMemoryBackend, MAX_GRANTED_OBJECT_IDS,
};
use apeireth_memory::{
    MemoryCandidateQuery, MemoryCoordinator, MemoryProvenance, MemoryRecallQuery, MemoryScope,
    SqliteMemoryStore,
};
use apeireth_plugin::memory_backend::MemoryBackend;

const ALICE: &str = "ns-alice";
const BOB: &str = "ns-bob";

/// 生产解析面的等价物：以**记忆存储**为真源回答「对象存在吗 / 在哪个分区」。
struct StoreResolver {
    store: Arc<SqliteMemoryStore>,
}

impl MemoryObjectResolver for StoreResolver {
    fn object_exists(&self, object_id: &str) -> bool {
        self.store.get_episode(object_id).ok().flatten().is_some()
    }

    fn object_partition(&self, object_id: &str) -> Option<String> {
        let episode = self.store.get_episode(object_id).ok().flatten()?;
        let metadata = self
            .store
            .get_episode_metadata(&episode.id)
            .ok()
            .flatten()?;
        let scope = metadata
            .get("scope")
            .and_then(|value| serde_json::from_value::<MemoryScope>(value.clone()).ok())?;
        Some(scope.to_string())
    }

    fn subject_partition(&self, subject: &str) -> Option<String> {
        Some(
            MemoryScope::Session {
                session_id: subject.to_string(),
            }
            .to_string(),
        )
    }
}

/// 夹具解析面（账本加固用例用：只关心「对象存在 + 分区名」）。
struct FixtureResolver {
    objects: BTreeMap<String, String>,
}

impl MemoryObjectResolver for FixtureResolver {
    fn object_exists(&self, object_id: &str) -> bool {
        self.objects.contains_key(object_id)
    }

    fn object_partition(&self, object_id: &str) -> Option<String> {
        self.objects.get(object_id).cloned()
    }

    fn subject_partition(&self, subject: &str) -> Option<String> {
        Some(format!("session:{subject}"))
    }
}

fn seed(coordinator: &MemoryCoordinator, id: &str, owner: &str, content: &str) {
    coordinator
        .writeback_scoped_episode(
            &Episode {
                id: id.to_string(),
                timestamp: SystemClock.now().timestamp(),
                role: "user".to_string(),
                content: content.to_string(),
                session_id: owner.to_string(),
            },
            &MemoryScope::Session {
                session_id: owner.to_string(),
            },
            &MemoryProvenance {
                source_session: Some(owner.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
}

/// 造一套夹具存储：alice 域两条、bob 域一条。
fn fixture_store() -> Arc<SqliteMemoryStore> {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let seeder = MemoryCoordinator::new(store.clone(), store.clone());
    seed(&seeder, "ep-alice-1", ALICE, "alpha orchard one");
    seed(&seeder, "ep-alice-2", ALICE, "alpha orchard two");
    seed(&seeder, "ep-bob-1", BOB, "alpha orchard bob");
    store
}

/// 装了授权面的协调器 + 登记处（域可见集合 = 主体自己的私有域）。
fn wired() -> (Arc<MemoryCoordinator>, Arc<ObjectGrantRegistry>) {
    let store = fixture_store();
    let registry = Arc::new(ObjectGrantRegistry::with_resolver(
        ObjectGrantLedger::in_memory(),
        Arc::new(StoreResolver {
            store: store.clone(),
        }),
    ));
    let coordinator = Arc::new(
        MemoryCoordinator::new(store.clone(), store.clone())
            .with_scoped_backend(store)
            .with_object_grants(registry.clone()),
    );
    (coordinator, registry)
}

/// bob 的召回（域可见 = bob 的私有域）。
fn recall_bob(coordinator: &MemoryCoordinator) -> Vec<String> {
    let mut query = MemoryRecallQuery::new(BOB, "alpha orchard");
    query.visible_scopes = vec![MemoryScope::Session {
        session_id: BOB.to_string(),
    }];
    query.limit = 10;
    // 打分阈值归零 ⇒ 断言只考**可见性**，不考 BM25 分值。
    query.min_score = 0.0;
    coordinator
        .recall(&query)
        .expect("recall")
        .items
        .into_iter()
        .map(|item| item.id)
        .collect()
}

// ---------------------------------------------------------------------------
// 读路径并联
// ---------------------------------------------------------------------------

#[test]
fn without_any_grant_the_recall_is_identical_with_and_without_the_grant_surface() {
    let store = fixture_store();
    let registry = Arc::new(ObjectGrantRegistry::with_resolver(
        ObjectGrantLedger::in_memory(),
        Arc::new(StoreResolver {
            store: store.clone(),
        }),
    ));
    let without = Arc::new(
        MemoryCoordinator::new(store.clone(), store.clone()).with_scoped_backend(store.clone()),
    );
    let with = Arc::new(
        MemoryCoordinator::new(store.clone(), store.clone())
            .with_scoped_backend(store)
            .with_object_grants(registry),
    );

    let a = recall_bob(&without);
    let b = recall_bob(&with);

    assert_eq!(a, b, "空授权面 ⇒ 读结果逐条相同（零变化纪律）");
    assert!(a.contains(&"ep-bob-1".to_string()), "域内对象照旧可见");
    assert!(!a.contains(&"ep-alice-1".to_string()), "域外对象读不到");
}

#[test]
fn an_approved_grant_makes_the_granted_object_retrievable() {
    let (coordinator, registry) = wired();
    assert!(!recall_bob(&coordinator).contains(&"ep-alice-1".to_string()));

    let entry = registry
        .request_grant(ALICE, "ep-alice-1", BOB, None, 1)
        .expect("grant request");
    registry
        .approve_grant("owner", &entry.grant_id, 2)
        .expect("approval");

    let ids = recall_bob(&coordinator);
    assert!(
        ids.contains(&"ep-alice-1".to_string()),
        "授权后被授对象**能被检索到**（能进候选集 ⇒ 不被静默丢掉）"
    );
}

#[test]
fn ungranted_objects_stay_invisible() {
    let (coordinator, registry) = wired();
    let entry = registry
        .request_grant(ALICE, "ep-alice-1", BOB, None, 1)
        .expect("grant request");
    registry
        .approve_grant("owner", &entry.grant_id, 2)
        .expect("approval");

    let ids = recall_bob(&coordinator);
    assert!(
        !ids.contains(&"ep-alice-2".to_string()),
        "未授权对象仍看不到"
    );
}

#[test]
fn a_grant_does_not_widen_the_domain() {
    let (coordinator, registry) = wired();
    let entry = registry
        .request_grant(ALICE, "ep-alice-1", BOB, None, 1)
        .expect("grant request");
    registry
        .approve_grant("owner", &entry.grant_id, 2)
        .expect("approval");

    let ids = recall_bob(&coordinator);
    assert!(ids.contains(&"ep-alice-1".to_string()), "被授的那一条可读");
    assert!(
        !ids.contains(&"ep-alice-2".to_string()),
        "同域的**另一条**仍不可见（授权不扩域）"
    );
}

#[test]
fn revoke_hides_the_object_immediately() {
    let (coordinator, registry) = wired();
    let entry = registry
        .request_grant(ALICE, "ep-alice-1", BOB, None, 1)
        .expect("grant request");
    registry
        .approve_grant("owner", &entry.grant_id, 2)
        .expect("approval");
    assert!(recall_bob(&coordinator).contains(&"ep-alice-1".to_string()));

    registry
        .revoke_grant("owner", &entry.grant_id, None, 3)
        .expect("revoke");

    assert!(
        !recall_bob(&coordinator).contains(&"ep-alice-1".to_string()),
        "撤销后**立刻**读不到（现读账本，无缓存）"
    );
}

#[test]
fn a_pending_grant_discloses_nothing() {
    let (coordinator, registry) = wired();
    let entry = registry
        .request_grant(ALICE, "ep-alice-1", BOB, None, 1)
        .expect("grant request");
    assert_eq!(entry.status, GrantStatus::Pending);

    assert!(
        !recall_bob(&coordinator).contains(&"ep-alice-1".to_string()),
        "人批前零披露（域级隔离此刻未被突破）"
    );
}

#[test]
fn forged_object_ids_are_refused() {
    let (_coordinator, registry) = wired();

    let error = registry
        .request_grant(ALICE, "ep-does-not-exist", BOB, None, 1)
        .expect_err("unknown object must be refused");
    assert!(
        error.to_string().contains("不存在"),
        "伪造对象 id ⇒ 拒绝：{error}"
    );
    assert!(
        !registry.read_grant_allows(BOB, "ep-does-not-exist"),
        "伪造对象 id 不产生任何授权"
    );
}

#[test]
fn the_granted_id_list_is_sanitized() {
    // 空集合 ⇒ 空（零变化）。
    assert!(sanitize_granted_object_ids(Vec::new()).is_empty());
    // 空串 / 重复 / 超长 id ⇒ 丢弃或去重。
    let sanitized = sanitize_granted_object_ids(vec![
        "  ".to_string(),
        "ep-alice-1".to_string(),
        "ep-alice-1".to_string(),
        "x".repeat(300),
    ]);
    assert_eq!(sanitized, vec!["ep-alice-1".to_string()]);
    // 超长列表 ⇒ 截断（不静默放大）。
    let many: Vec<String> = (0..1_000).map(|index| format!("ep-{index}")).collect();
    assert_eq!(
        sanitize_granted_object_ids(many).len(),
        MAX_GRANTED_OBJECT_IDS,
        "超长 id 列表被截断"
    );
}

/// 后端候选查询：**域可见集合 ∪ 被授对象 id 集合**（并联）。
///
/// 覆盖两条被改过的后端路径：
/// * `InMemoryBackend`（`backend/in_memory.rs`）—— 进程内哈希实现；
/// * `SqliteBackend`（`backend/sqlite.rs`）—— 真 SQL 谓词。
fn candidate_ids(backend: &dyn ScopedMemoryBackend, granted: Vec<String>) -> Vec<String> {
    let query = MemoryCandidateQuery {
        source_session: Some(BOB.to_string()),
        visible_scopes: vec![MemoryScope::Session {
            session_id: BOB.to_string(),
        }],
        granted_object_ids: granted,
        limit: 16,
        as_of_ms: None,
    };
    backend
        .query_candidates(&query)
        .expect("query candidates")
        .into_iter()
        .map(|episode| episode.id)
        .collect()
}

#[test]
fn the_in_memory_backend_candidate_query_unions_domain_scopes_with_granted_ids() {
    let backend = InMemoryBackend::new();
    for (id, owner) in [
        ("ep-alice-1", ALICE),
        ("ep-alice-2", ALICE),
        ("ep-bob-1", BOB),
    ] {
        backend
            .put_episode(&Episode {
                id: id.to_string(),
                timestamp: SystemClock.now().timestamp(),
                role: "user".to_string(),
                content: format!("alpha {id}"),
                session_id: owner.to_string(),
            })
            .expect("put episode");
        backend
            .put_episode_metadata(
                id,
                serde_json::json!({
                    "scope": MemoryScope::Session { session_id: owner.to_string() },
                }),
            )
            .expect("put metadata");
    }

    let plain = candidate_ids(&backend, Vec::new());
    assert!(
        plain.contains(&"ep-bob-1".to_string()),
        "域内对象照旧在候选集里"
    );
    assert!(
        !plain.contains(&"ep-alice-1".to_string()),
        "无授权 ⇒ 域外对象不在候选集里"
    );

    let granted = candidate_ids(&backend, vec!["ep-alice-1".to_string()]);
    assert!(
        granted.contains(&"ep-alice-1".to_string()),
        "被授对象进得了候选集（域外）"
    );
    assert!(
        !granted.contains(&"ep-alice-2".to_string()),
        "未授权对象仍不在候选集里（授权不扩域）"
    );
}

#[tokio::test]
async fn the_sqlite_backend_candidate_query_unions_domain_scopes_with_granted_ids() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("candidates.sqlite3");
    // 先跑一次迁移建表（`SqliteBackend` 复用同一套表）。
    let _migrated = SqliteMemoryStore::open(&path).expect("migrate schema");
    let pool = Arc::new(
        apeireth_storage::SqliteConnectionPool::open(&path)
            .await
            .expect("open pool"),
    );
    let backend = SqliteBackend::from_arc(pool);
    for (id, owner) in [
        ("ep-alice-1", ALICE),
        ("ep-alice-2", ALICE),
        ("ep-bob-1", BOB),
    ] {
        backend
            .put_episode(&Episode {
                id: id.to_string(),
                timestamp: SystemClock.now().timestamp(),
                role: "user".to_string(),
                content: format!("alpha {id}"),
                session_id: owner.to_string(),
            })
            .expect("put episode");
        backend
            .put_episode_metadata(
                id,
                serde_json::json!({
                    "scope": MemoryScope::Session { session_id: owner.to_string() },
                }),
            )
            .expect("put metadata");
    }

    let plain = candidate_ids(&backend, Vec::new());
    assert!(
        plain.contains(&"ep-bob-1".to_string()),
        "域内对象照旧在候选集里"
    );
    assert!(
        !plain.contains(&"ep-alice-1".to_string()),
        "无授权 ⇒ 域外对象不在候选集里"
    );

    let granted = candidate_ids(&backend, vec!["ep-alice-1".to_string()]);
    assert!(
        granted.contains(&"ep-alice-1".to_string()),
        "被授对象进得了候选集（域外）"
    );
    assert!(
        !granted.contains(&"ep-alice-2".to_string()),
        "未授权对象仍不在候选集里（授权不扩域）"
    );
}

// ---------------------------------------------------------------------------
// 账本读端加固
// ---------------------------------------------------------------------------

fn grant_line(version: u64, grant_id: &str, status: &str, decided_by: Option<&str>) -> String {
    serde_json::json!({
        "record": "grant",
        "version": version,
        "grant_id": grant_id,
        "object_id": "ep-alice-1",
        "subject": BOB,
        "grantor": ALICE,
        "granted_at_ms": 1,
        "at_ms": 1,
        "effect": "read_only",
        "status": status,
        "partition": format!("session:{ALICE}"),
        "cross_partition": true,
        "reason": null,
        "decided_by": decided_by,
        "decided_at_ms": null,
    })
    .to_string()
}

fn registry_over(path: &std::path::Path) -> Arc<ObjectGrantRegistry> {
    let ledger = ObjectGrantLedger::open(path).expect("open ledger");
    let resolver = FixtureResolver {
        objects: BTreeMap::from([("ep-alice-1".to_string(), format!("session:{ALICE}"))]),
    };
    Arc::new(ObjectGrantRegistry::with_resolver(
        ledger,
        Arc::new(resolver),
    ))
}

#[test]
fn a_replayed_old_active_row_does_not_revive_a_revoked_grant() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = String::new();
    text.push_str(&grant_line(1, "g-1", "pending", None));
    text.push('\n');
    text.push_str(&grant_line(2, "g-1", "active", Some("owner")));
    text.push('\n');
    text.push_str(&grant_line(3, "g-1", "revoked", Some("owner")));
    text.push('\n');
    // 攻击：把**旧的 active 行**追加到文件末尾。
    text.push_str(&grant_line(2, "g-1", "active", Some("owner")));
    text.push('\n');
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "旧 active 行被重放 ⇒ **不复活**（按 version 折算）"
    );
}

#[test]
fn a_forged_active_row_without_a_pending_predecessor_is_absent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    std::fs::write(
        &path,
        format!("{}\n", grant_line(1, "g-1", "active", Some("owner"))),
    )
    .expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "伪造 active 行（无 pending 前身）⇒ 按不存在处理"
    );
    let ledger = ObjectGrantLedger::open(&path).expect("reopen");
    assert!(
        ledger
            .audits()
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantRefused),
        "异常留审计"
    );
}

#[test]
fn a_forged_active_row_without_a_decider_is_absent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = format!("{}\n", grant_line(1, "g-1", "pending", None));
    text.push_str(&grant_line(2, "g-1", "active", None));
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "伪造 active 行（decided_by 空）⇒ 按不存在处理（跳不过人批）"
    );
}

#[test]
fn a_non_increasing_version_is_rejected_and_audited() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = format!("{}\n", grant_line(2, "g-1", "pending", None));
    text.push_str(&grant_line(1, "g-1", "active", Some("owner")));
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "version 非递增 ⇒ 该行按不存在处理"
    );
    let ledger = ObjectGrantLedger::open(&path).expect("reopen");
    assert!(
        ledger
            .audits()
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantRefused && row.detail.contains("非递增")),
        "非递增记审计"
    );
}

#[test]
fn invalid_versions_are_rejected() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = format!("{}\n", grant_line(0, "g-zero", "active", Some("owner")));
    text.push_str(&grant_line(u64::MAX, "g-max", "active", Some("owner")));
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "version=0 / u64::MAX ⇒ 被拒"
    );
    let ledger = ObjectGrantLedger::open(&path).expect("reopen");
    assert!(
        ledger.audits().iter().any(
            |row| row.kind == GrantAuditKind::GrantRefused && row.detail.contains("版本号非法")
        ),
        "非法版本记审计"
    );
}

#[test]
fn a_well_formed_active_row_still_takes_effect() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = format!("{}\n", grant_line(1, "g-1", "pending", None));
    text.push_str(&grant_line(2, "g-1", "active", Some("owner")));
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        registry.read_grant_allows(BOB, "ep-alice-1"),
        "正常 active（有 pending 前身 + decided_by）⇒ 仍生效（别把正常路径堵死）"
    );
}

#[test]
fn pending_rows_still_disclose_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    std::fs::write(
        &path,
        format!("{}\n", grant_line(1, "g-1", "pending", None)),
    )
    .expect("write ledger");

    let registry = registry_over(&path);

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "pending 仍零披露"
    );
}

#[test]
fn a_cross_process_revoke_is_picked_up_on_the_next_decision() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ledger.jsonl");
    let mut text = format!("{}\n", grant_line(1, "g-1", "pending", None));
    text.push_str(&grant_line(2, "g-1", "active", Some("owner")));
    std::fs::write(&path, text).expect("write ledger");

    let registry = registry_over(&path);
    assert!(registry.read_grant_allows(BOB, "ep-alice-1"));

    // 另一个进程追加撤销行（长度变化 ⇒ 指纹变化 ⇒ 下一次判定重载）。
    let mut revoked = std::fs::read_to_string(&path).expect("read ledger");
    revoked.push_str(&grant_line(3, "g-1", "revoked", Some("owner")));
    revoked.push('\n');
    std::fs::write(&path, revoked).expect("write revoke");

    assert!(
        !registry.read_grant_allows(BOB, "ep-alice-1"),
        "跨进程撤销 ⇒ 下一次判定即生效（按文件指纹重载）"
    );
}
