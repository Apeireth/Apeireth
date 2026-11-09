//! W3 单元 7a · C-D1 记忆**对象级授权**的**生产注册**（含宿主激活口的提供方半边）。
//!
//! 钉死的面：
//!
//! * **存在性即开关**（0 新 env 名）：账本不存在 ⇒ 面**不注册**、服务句柄缺席、
//!   名册报 false ⇒ 逐字节零变化；账本存在 ⇒ 注册 + 名册报 true。
//! * **宿主激活口**：人批 `approve`/`deny` 走 [`ObjectGrantActivation`] 端口 ⇒
//!   模型面**没有** approve/deny 动作（词表闭合）⇒ 模型永远不能自批。
//! * **审批前零披露**：`pending` 的授权**不生效**（`check_read` 如实报
//!   `object_grant = false`）⇒ 人批通过后才生效。
//! * **否决留审计**：否决路径**必须**留下审计行（审计未落 ⇒ Err，不静默放行）。
//! * **解析面 fail-closed**：解析面没绑上协调器 ⇒ 对象**查不出** ⇒ 授权请求被拒
//!   （查不出 = 不可判定 = 不放行）。
//! * **读路径并联（生产侧）**：生产装配把**同一个登记处**注入了记忆协调器 ⇒
//!   被授对象真进候选集（候选集效果由记忆 crate 的
//!   `crates/engine/memory/tests/object_grant_visibility.rs` 钉死；本文件钉死
//!   **生产侧确实接线**：真实 episode 能被解析面看见 ⇒ 解析面已绑上生产协调器）。

use std::sync::Arc;

use apeireth_core::kernel::{Clock, Episode, SystemClock};
use apeireth_memory::object_grant::{GrantAuditKind, GrantStatus};
use apeireth_memory::{MemoryCoordinator, MemoryProvenance, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime::canonical::ObjectGrantActivation;
use apeireth_runtime_assembly::canonical::object_grant::{MemoryGrantResolver, ObjectGrantService};
use apeireth_runtime_assembly::canonical::{
    ObjectGrantConfig, ProductionBackends, ProductionModules, ProductionModulesConfig,
};

/// alice 的私有域（episode 归它）。
const ALICE: &str = "ns-alice";
/// bob（被授主体）：它的私有域里**没有** alice 的 episode ⇒ 域级不可见。
const BOB: &str = "ns-bob";

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

/// 造一份**带真实 episode** 的记忆存储（alice 域一条）。
fn fixture_store() -> Arc<SqliteMemoryStore> {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("store"));
    let seeder = MemoryCoordinator::new(store.clone(), store.clone());
    seeder
        .writeback_scoped_episode(
            &Episode {
                id: "ep-alice-1".to_string(),
                timestamp: SystemClock.now().timestamp(),
                role: "user".to_string(),
                content: "alpha orchard one".to_string(),
                session_id: ALICE.to_string(),
            },
            &MemoryScope::Session {
                session_id: ALICE.to_string(),
            },
            &MemoryProvenance {
                source_session: Some(ALICE.to_string()),
                ..Default::default()
            },
        )
        .expect("seed episode");
    store
}

/// 账本落位（独占目录；文件**存在** ⇒ 开关开）。
fn ledger_at(dir: &std::path::Path) -> std::path::PathBuf {
    let ledger = dir.join("object-grant").join("ledger.jsonl");
    std::fs::create_dir_all(ledger.parent().expect("parent")).expect("dir");
    std::fs::write(&ledger, b"").expect("ledger file");
    ledger
}

fn capability_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect()
}

/// 自述名册里该面的登记行（真调 `tool.self_status` ⇒ 与生产同一条投影路径）。
async fn roster_switch(modules: &ProductionModules, name: &str) -> Option<bool> {
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status must be registered");
    let call = ToolCall {
        id: "call_roster".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: serde_json::Value =
        serde_json::from_str(&result.render()).expect("structured self-report json");
    report["capabilities"][name].as_bool()
}

/// ① 存在性即开关：账本不存在 ⇒ 不注册（逐字节零变化）。
#[tokio::test]
async fn the_face_stays_unregistered_without_a_ledger() {
    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    let ids = capability_ids(&modules);
    assert!(
        !ids.iter().any(|id| id == "tool.object_grant"),
        "账本不存在 ⇒ 不得注册: {ids:?}"
    );
    assert!(
        modules.object_grant_service().is_none(),
        "未装 ⇒ 宿主激活口无路可走 (不是默认放行)"
    );
    assert_eq!(roster_switch(&modules, "object_grant").await, Some(false));
}

/// ② 存在性即开关：账本存在 ⇒ 注册 + 服务句柄 + 名册如实。
#[tokio::test]
async fn the_face_is_registered_when_the_ledger_exists() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ledger = ledger_at(temp.path());
    let mut config = base_config();
    config.object_grant = Some(ObjectGrantConfig::new(&ledger));

    let modules =
        ProductionModules::build(config, ProductionBackends::default(), clock()).expect("build");
    let ids = capability_ids(&modules);
    assert!(
        ids.iter().any(|id| id == "tool.object_grant"),
        "账本存在 ⇒ 必须注册: {ids:?}"
    );
    assert!(modules.object_grant_service().is_some());
    assert_eq!(roster_switch(&modules, "object_grant").await, Some(true));

    // 声明名与能力 id 对得上（模型面工具名 = `object_grant`）。
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.object_grant")
        .expect("tool.object_grant");
    assert_eq!(tool.declaration().name, "object_grant");
}

/// ③ 模型面**没有** approve/deny：词表闭合 ⇒ 模型不能自批。
#[tokio::test]
async fn the_model_face_cannot_approve_or_deny() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ledger = ledger_at(temp.path());
    let mut config = base_config();
    config.object_grant = Some(ObjectGrantConfig::new(&ledger));
    let modules =
        ProductionModules::build(config, ProductionBackends::default(), clock()).expect("build");

    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.object_grant")
        .expect("tool.object_grant");
    let actions = tool.declaration().parameters["properties"]["action"]["enum"]
        .as_array()
        .expect("动作词表")
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(actions, ["grant", "revoke", "list", "check"]);
    for forbidden in ["approve", "deny", "active"] {
        assert!(
            !actions.iter().any(|action| action == forbidden),
            "模型面不得有 {forbidden:?} 动作: {actions:?}"
        );
    }
}

/// ④ 审批前零披露 ⇒ 宿主人批通过后才生效（端口契约）。
#[tokio::test]
async fn a_grant_takes_effect_only_after_a_host_approval() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ledger = ledger_at(temp.path());
    let store = fixture_store();
    let mut config = base_config();
    config.object_grant = Some(ObjectGrantConfig::new(&ledger));
    // 记忆召回开 ⇒ 协调器被造出来 ⇒ 解析面绑得上（读路径并联的生产前提）。
    config.memory_recall = true;
    let backends = ProductionBackends {
        memory: Some(store.clone()),
        memory_governance: Some(store),
        ..ProductionBackends::default()
    };

    let modules = ProductionModules::build(config, backends, clock()).expect("build");
    let service = modules.object_grant_service().expect("宿主激活口");

    // 域级不可见（bob 的私有域里没有 alice 的 episode）⇒ 可见性只能来自对象授权。
    let before = service.check_read(BOB, "ep-alice-1");
    assert!(!before.domain_visible, "域级本不可见");
    assert!(!before.object_grant, "尚无授权");
    assert!(!before.visible, "人批前不可见 (审批前零披露)");

    // 模型面登记一条**请求**（pending）。
    let entry = service
        .request_grant(ALICE, "ep-alice-1", BOB, Some("生产接线自证"))
        .expect("请求登记");
    assert_eq!(entry.status, GrantStatus::Pending);

    // pending **不生效** ⇒ 仍读不到。
    let pending = service.check_read(BOB, "ep-alice-1");
    assert!(!pending.object_grant, "pending 不得生效");
    assert!(!pending.visible, "pending 不得披露");

    // 宿主（人）通过 ⇒ 授权生效（只读）。
    let activation: &dyn ObjectGrantActivation = service.as_ref();
    activation
        .approve(&entry.grant_id, "host-operator")
        .expect("人批通过");

    let after = service.check_read(BOB, "ep-alice-1");
    assert!(after.object_grant, "人批通过后授权生效");
    assert!(after.visible, "人批通过后可读");
    assert!(!after.write_allowed, "对象授权**只读** ⇒ 写路径恒关");

    // 审计行在场（授予 + 人批都留痕）。
    let snapshot = service.snapshot();
    assert!(
        snapshot
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantApproved),
        "人批必须留审计行: {:?}",
        snapshot.audits
    );
}

/// ⑤ 否决 ⇒ 授权**永不生效**，且否决路径**留审计行**（审计守卫）。
#[tokio::test]
async fn a_denied_grant_never_takes_effect_and_leaves_an_audit_row() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ledger = ledger_at(temp.path());
    let store = fixture_store();
    let mut config = base_config();
    config.object_grant = Some(ObjectGrantConfig::new(&ledger));
    config.memory_recall = true;
    let backends = ProductionBackends {
        memory: Some(store.clone()),
        memory_governance: Some(store),
        ..ProductionBackends::default()
    };

    let modules = ProductionModules::build(config, backends, clock()).expect("build");
    let service = modules.object_grant_service().expect("宿主激活口");
    let entry = service
        .request_grant(ALICE, "ep-alice-1", BOB, Some("待否决"))
        .expect("请求登记");

    let activation: &dyn ObjectGrantActivation = service.as_ref();
    activation
        .deny(&entry.grant_id, "host-operator", Some("不该给"))
        .expect("人批否决");

    let after = service.check_read(BOB, "ep-alice-1");
    assert!(!after.object_grant, "否决 ⇒ 授权永不生效");
    assert!(!after.visible, "否决 ⇒ 不可见");

    // 否决路径的审计行**在场**（审计未落 ⇒ Err，不静默放行）。
    let snapshot = service.snapshot();
    assert!(
        snapshot
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantDenied),
        "否决必须留审计行: {:?}",
        snapshot.audits
    );
}

/// ⑥ 解析面 fail-closed：没绑上协调器 ⇒ 对象**查不出** ⇒ 授权请求被拒。
#[test]
fn the_resolver_is_fail_closed_until_a_coordinator_is_bound() {
    let store = fixture_store();
    let resolver = Arc::new(MemoryGrantResolver::deferred());

    // 未绑定 ⇒ 查不出 ⇒ 对象「不存在」⇒ 授权请求被拒（不放行）。
    let registry = apeireth_memory::object_grant::ObjectGrantRegistry::with_resolver(
        apeireth_memory::object_grant::ObjectGrantLedger::in_memory(),
        {
            let resolver: Arc<dyn apeireth_memory::object_grant::MemoryObjectResolver> =
                resolver.clone();
            resolver
        },
    );
    let service = ObjectGrantService::new(Arc::new(registry), clock());
    let refused = service.request_grant(ALICE, "ep-alice-1", BOB, None);
    assert!(
        refused.is_err(),
        "解析面没绑 ⇒ 对象查不出 ⇒ 授权请求必须被拒: {refused:?}"
    );

    // 绑定之后 ⇒ 看得见真实 episode ⇒ 请求通过。
    let coordinator = Arc::new(MemoryCoordinator::new(store.clone(), store));
    resolver.bind(&coordinator);
    let allowed = service.request_grant(ALICE, "ep-alice-1", BOB, None);
    assert!(allowed.is_ok(), "绑定后请求必须通过: {allowed:?}");
}
