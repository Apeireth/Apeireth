//! C-D1 记忆**对象级授权**（`tool.object_grant`）的行为证据套件。
//!
//! 钉死的行为面:
//!
//! * 未授权读不到 / 授权后可读 / 授权后**写不了**;
//! * 撤销**立刻**读不到（无缓存、无轮询）;
//! * 域外对象授权**需人批**且**留审计**（跨分区披露 = R4 硬门）;
//! * 读被授对象**零审批**（读面 R0）;
//! * **坏账本行按不存在处理**;
//! * **域级隔离未被突破**（别的域的对象仍读不到 · 域可见语义不变）;
//! * **对抗**：伪造授权（对象 id 不存在 / 主体为空 / 已被撤销）⇒ 一律拒绝。
//!
//! 语义口径（lead 拍板，**不得改**）：**叠加 ≠ 替代** ——
//! `读可见 = 域可见 ∨ (存在有效对象 grant)`。

use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::kernel::{
    AgentId, CapabilityId, Clock, NamespaceId, PluginId, ProfileId, SessionId, SystemClock, TraceId,
};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_memory::object_grant::{
    GrantAuditKind, GrantStatus, GrantVisibility, ObjectGrantLedger, ObjectGrantRegistry,
};
use apeireth_memory::{MemoryCoordinator, MemoryProvenance, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::agents::AgentPrincipal;
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::{
    ToolAuthorization, ToolCaller, ToolCancellation, ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_runtime_assembly::canonical::object_grant::{
    object_grant_registry, MemoryGrantResolver, ObjectGrantApprovalHook, ObjectGrantService,
    ObjectGrantTool, ACTION_CHECK, ACTION_GRANT, ACTION_LIST, ACTION_REVOKE,
    OBJECT_GRANT_CAPABILITY_ID, OBJECT_GRANT_TOOL_NAME,
};
use serde_json::{json, Value};

/// 两个互不可见的域（私有会话域）。
const ALICE: &str = "ns-alice";
const BOB: &str = "ns-bob";

struct Harness {
    coordinator: Arc<MemoryCoordinator>,
    registry: Arc<ObjectGrantRegistry>,
    service: Arc<ObjectGrantService>,
    tool: ObjectGrantTool,
}

fn seed(coordinator: &MemoryCoordinator, id: &str, owner: &str) {
    coordinator
        .writeback_scoped_episode(
            &apeireth_core::kernel::Episode {
                id: id.to_string(),
                timestamp: SystemClock.now().timestamp(),
                role: "user".to_string(),
                content: format!("memory owned by {owner}"),
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

fn harness() -> Harness {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator =
        Arc::new(MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone()));
    seed(&coordinator, "ep-alice-1", ALICE);
    seed(&coordinator, "ep-alice-2", ALICE);
    seed(&coordinator, "ep-bob-1", BOB);
    let resolver = Arc::new(MemoryGrantResolver::new(coordinator.clone()));
    let registry = object_grant_registry(None, resolver).expect("in-memory ledger");
    let service = Arc::new(ObjectGrantService::with_default_clock(registry.clone()));
    let tool = ObjectGrantTool::new(service.clone());
    Harness {
        coordinator,
        registry,
        service,
        tool,
    }
}

fn execution_context(call_id: &str) -> ToolExecutionContext {
    ToolExecutionContext {
        agent: AgentId::default(),
        principal: Some(AgentPrincipal {
            profile: ProfileId::new(),
            namespace: NamespaceId::new(),
        }),
        session: None,
        turn: None,
        call_id: call_id.to_string(),
        caller: ToolCaller::Host,
        module: PluginId::new("test.object_grant").expect("valid plugin id"),
        module_instance: None,
        internal_review: None,
        message_support: false,
        message_route: None,
        delegated_tools: None,
        delegation: None,
        tool_scope: None,
        authorization: ToolAuthorization::default(),
        cancellation: ToolCancellation::default(),
        deadline: Instant::now() + Duration::from_secs(30),
        trace: None,
        module_tasks: ModuleTaskScope::default(),
        resource_lease: None,
    }
}

fn call(action: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: format!("call_{action}"),
        name: OBJECT_GRANT_TOOL_NAME.to_string(),
        arguments,
    }
}

fn payload(result: &ToolResult) -> Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("object_grant call failed: {message}"),
    }
}

// ---------------------------------------------------------------------------
// 读可见性 = 域可见 ∨ 有效对象授权（叠加 ≠ 替代）
// ---------------------------------------------------------------------------

#[test]
fn ungranted_object_is_not_readable_across_domains() {
    let h = harness();

    let view = h.service.check_read(BOB, "ep-alice-1");

    assert!(!view.domain_visible, "域级本就不可见（域隔离）");
    assert!(!view.object_grant, "无授权 ⇒ 无对象可见性");
    assert!(!view.visible, "未授权读不到");
    assert!(
        !h.registry.read_grant_allows(BOB, "ep-alice-1"),
        "登记处也判不可读"
    );
}

#[test]
fn approved_grant_makes_exactly_one_object_readable() {
    let h = harness();

    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, Some("交接一条记忆"))
        .expect("grant request registered");
    assert_eq!(entry.status, GrantStatus::Pending);
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "请求本身**不生效**（逐次人批前不得披露）"
    );

    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");

    let view = h.service.check_read(BOB, "ep-alice-1");
    assert!(view.object_grant, "有效对象授权 ⇒ 对象可见");
    assert!(view.visible, "授权后可读");
    assert!(
        !view.domain_visible,
        "叠加**不改**域级语义：域级仍不可见（可见性来自对象授权）"
    );
    assert!(
        !h.service.check_read(BOB, "ep-alice-2").visible,
        "只授权那一条对象 ⇒ 同域的其它对象仍读不到"
    );
    assert!(
        h.service.check_read(ALICE, "ep-alice-1").domain_visible,
        "域内主体照旧域可见（域级语义逐字节不变）"
    );
}

#[test]
fn the_overlay_primitive_is_wired_at_the_domain_filter_point() {
    let h = harness();
    let alice_object = MemoryScope::Session {
        session_id: ALICE.to_string(),
    };
    let bob_scopes = vec![MemoryScope::Session {
        session_id: BOB.to_string(),
    }];
    let alice_scopes = vec![MemoryScope::Session {
        session_id: ALICE.to_string(),
    }];

    // 既有域级过滤点：`scope.is_visible_in(&visible_scopes)`。
    let overlay = GrantVisibility::new(&h.registry, BOB);
    assert!(
        !overlay.allows(&alice_object, &bob_scopes, "ep-alice-1"),
        "域不可见 + 无授权 ⇒ 读不到"
    );

    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");

    assert!(
        overlay.allows(&alice_object, &bob_scopes, "ep-alice-1"),
        "域不可见 + 有效授权 ⇒ 叠加放行"
    );
    assert!(
        !overlay.allows(&alice_object, &bob_scopes, "ep-alice-2"),
        "叠加只对被授的那一条对象生效"
    );
    assert!(
        overlay.allows(&alice_object, &alice_scopes, "ep-alice-1"),
        "域可见路径逐字节不变（不需授权）"
    );
    assert_eq!(
        overlay.granted_object_ids(),
        vec!["ep-alice-1".to_string()],
        "候选生成侧原语：只给被授的那一条对象"
    );
}

#[test]
fn a_grant_never_opens_the_write_path() {
    let h = harness();

    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");

    assert!(
        h.service.check_read(BOB, "ep-alice-1").visible,
        "读可见（只读授予成立）"
    );
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").write_allowed,
        "只读授予 ⇒ 写路径不放行"
    );
    assert!(
        !h.registry.allows_write(BOB, "ep-alice-1"),
        "登记处的写判定恒 false"
    );
    let snapshot = h.service.snapshot();
    let active = snapshot
        .entries
        .iter()
        .find(|entry| entry.status == GrantStatus::Active)
        .expect("active grant row");
    assert_eq!(
        active.effect,
        apeireth_memory::object_grant::GrantEffect::ReadOnly,
        "生效范围恒为只读"
    );
}

#[test]
fn revoke_takes_effect_immediately() {
    let h = harness();
    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");
    assert!(h.service.check_read(BOB, "ep-alice-1").visible);

    h.service
        .revoke_grant("owner", &entry.grant_id, Some("交接结束"))
        .expect("revoke");

    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "撤销后**立刻**读不到（无缓存 / 无轮询）"
    );
    assert!(
        !h.registry.read_grant_allows(BOB, "ep-alice-1"),
        "登记处同样立刻判不可读"
    );
    assert!(
        h.service
            .snapshot()
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantRevoked),
        "撤销留审计"
    );
}

// ---------------------------------------------------------------------------
// 治理：逐次人批 + 审计
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cross_partition_grant_needs_human_approval_and_leaves_audit_trail() {
    let h = harness();

    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, Some("跨分区披露"))
        .expect("grant request");
    assert!(
        entry.cross_partition,
        "对象分区 ≠ 主体分区 ⇒ 跨分区披露（R4 硬门）"
    );
    assert_eq!(entry.status, GrantStatus::Pending);
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "人批前不得披露（域级隔离此刻未被突破）"
    );

    let snapshot = h.service.snapshot();
    assert!(
        snapshot
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantRequested
                && row.object_id == "ep-alice-1"
                && row.subject == BOB),
        "授予请求留审计"
    );

    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");
    let snapshot = h.service.snapshot();
    assert!(
        snapshot
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantApproved
                && row.actor == "owner"
                && row.object_id == "ep-alice-1"),
        "人批留审计（含人批执行者）"
    );
    assert!(h.service.check_read(BOB, "ep-alice-1").visible);
}

#[tokio::test]
async fn write_face_stops_at_human_approval_and_read_face_needs_none() {
    let hook = ObjectGrantApprovalHook::new();
    let capability = CapabilityId::new(OBJECT_GRANT_CAPABILITY_ID).expect("capability id");

    for action in [ACTION_GRANT, ACTION_REVOKE] {
        let arguments = json!({"action": action, "object_id": "ep-alice-1", "subject": BOB});
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let decision = hook.evaluate(&request).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "{action} = 写面 ⇒ 逐次人批（无自动放行档）"
        );
    }

    for action in [ACTION_LIST, ACTION_CHECK] {
        let arguments = json!({"action": action});
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let decision = hook.evaluate(&request).await;
        assert!(
            decision.is_allowed(),
            "{action} = 只读面 ⇒ 零审批（读取被授对象不需要审批）"
        );
    }

    // 未知动作 fail-closed 停审批。
    let arguments = json!({"action": "unknown_action"});
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    assert!(
        matches!(
            hook.evaluate(&request).await,
            Decision::RequireApproval { .. }
        ),
        "未知动作 fail-closed 停人工审批"
    );
}

// ---------------------------------------------------------------------------
// 工具面端到端
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_tool_face_registers_a_pending_request_and_never_grants_by_itself() {
    let h = harness();

    let result = h
        .tool
        .invoke_with_context(
            &call(
                ACTION_GRANT,
                json!({"action": ACTION_GRANT, "object_id": "ep-alice-1", "subject": BOB, "reason": "交接"}),
            ),
            None,
            &execution_context("call_grant"),
        )
        .await;
    let frame = payload(&result);
    assert_eq!(frame["ok"], json!(true));
    assert_eq!(frame["status"], json!("pending"));
    assert_eq!(frame["requires_human_approval"], json!(true));
    assert_eq!(frame["grant"]["effect"], json!("read_only"));
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "模型面登记请求**不产生**可读性（逐次人批前）"
    );

    let grant_id = frame["grant"]["grant_id"]
        .as_str()
        .expect("grant id")
        .to_string();
    h.service
        .approve_grant("owner", &grant_id)
        .expect("human approval");
    assert!(h.service.check_read(BOB, "ep-alice-1").visible);

    // 撤销面：即时生效。
    let result = h
        .tool
        .invoke_with_context(
            &call(
                ACTION_REVOKE,
                json!({"action": ACTION_REVOKE, "grant_id": grant_id}),
            ),
            None,
            &execution_context("call_revoke"),
        )
        .await;
    let frame = payload(&result);
    assert_eq!(frame["ok"], json!(true));
    assert_eq!(frame["immediate"], json!(true));
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "工具面撤销后立刻读不到"
    );
}

#[tokio::test]
async fn the_tool_face_reports_the_overlay_decomposition_on_check() {
    let h = harness();
    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");

    let result = h
        .tool
        .invoke_with_context(
            &call(
                ACTION_CHECK,
                json!({"action": ACTION_CHECK, "subject": BOB, "object_id": "ep-alice-1"}),
            ),
            None,
            &execution_context("call_check"),
        )
        .await;
    let frame = payload(&result);
    let visibility = &frame["visibility"];
    assert_eq!(visibility["domain_visible"], json!(false));
    assert_eq!(visibility["object_grant"], json!(true));
    assert_eq!(visibility["visible"], json!(true));
    assert_eq!(visibility["write_allowed"], json!(false));
}

// ---------------------------------------------------------------------------
// 账本健壮性 + 对抗
// ---------------------------------------------------------------------------

#[test]
fn malformed_ledger_lines_are_treated_as_absent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("object_grants.jsonl");
    std::fs::write(
        &path,
        concat!(
            "this-is-not-json\n",
            "{\"record\":\"grant\",\"version\":1,\"grant_id\":\"g-bad\",\"object_id\":\"\",\"subject\":\"ns-bob\",\"grantor\":\"ns-alice\",\"granted_at_ms\":1,\"at_ms\":1,\"effect\":\"read_only\",\"status\":\"active\",\"partition\":\"session:ns-alice\",\"cross_partition\":true}\n",
            "\n",
            "{\"record\":\"grant\",\"version\":2,\"grant_id\":\"g-bad2\",\"object_id\":\"ep-alice-1\",\"subject\":\"\",\"grantor\":\"ns-alice\",\"granted_at_ms\":1,\"at_ms\":1,\"effect\":\"read_only\",\"status\":\"active\",\"partition\":\"session:ns-alice\",\"cross_partition\":true}\n",
        ),
    )
    .expect("write ledger");

    let ledger = ObjectGrantLedger::open(&path).expect("open ledger");

    assert!(
        ledger.entries().is_empty(),
        "坏行（解析失败 / 主体为空）按不存在处理 ⇒ 一条都不入账"
    );
}

#[test]
fn forged_grants_are_rejected_in_every_shape() {
    let h = harness();

    // ① 伪造对象 id（对象不存在）⇒ 登记拒绝 + 审计留痕。
    let error = h
        .service
        .request_grant(ALICE, "ep-does-not-exist", BOB, None)
        .expect_err("unknown object must be refused");
    assert!(
        matches!(
            error,
            apeireth_memory::object_grant::GrantError::UnknownObject(_)
        ),
        "对象 id 不存在 ⇒ 拒绝"
    );
    assert!(
        h.service
            .snapshot()
            .audits
            .iter()
            .any(|row| row.kind == GrantAuditKind::GrantRefused),
        "拒绝也留审计"
    );
    assert!(!h.service.check_read(BOB, "ep-does-not-exist").visible);

    // ② 主体为空 ⇒ 拒绝。
    let error = h
        .service
        .request_grant(ALICE, "ep-alice-1", "   ", None)
        .expect_err("empty subject must be refused");
    assert!(
        matches!(
            error,
            apeireth_memory::object_grant::GrantError::EmptySubject
        ),
        "主体为空 ⇒ 拒绝"
    );
    assert!(!h.service.check_read("", "ep-alice-1").visible);

    // ③ 已被撤销的授权 ⇒ 拒绝（撤销即时生效）。
    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");
    assert!(h.service.check_read(BOB, "ep-alice-1").visible);
    h.service
        .revoke_grant("owner", &entry.grant_id, None)
        .expect("revoke");
    assert!(
        !h.service.check_read(BOB, "ep-alice-1").visible,
        "已被撤销 ⇒ 拒绝"
    );

    // ④ 伪造账本行（对象不存在）⇒ 绑定校验失败 ⇒ 不生效。
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("forged.jsonl");
    std::fs::write(
        &path,
        "{\"record\":\"grant\",\"version\":1,\"grant_id\":\"forged\",\"object_id\":\"ep-forged\",\"subject\":\"ns-bob\",\"grantor\":\"ns-alice\",\"granted_at_ms\":1,\"at_ms\":1,\"effect\":\"read_only\",\"status\":\"active\",\"partition\":\"session:ns-alice\",\"cross_partition\":true}\n",
    )
    .expect("write forged ledger");
    let ledger = ObjectGrantLedger::open(&path).expect("open forged ledger");
    assert_eq!(ledger.entries().len(), 1, "结构完好的行入账");
    let resolver = Arc::new(MemoryGrantResolver::new(h.coordinator.clone()));
    let registry = ObjectGrantRegistry::with_resolver(ledger, resolver);
    assert!(
        !registry.read_grant_allows(BOB, "ep-forged"),
        "对象不存在 ⇒ 伪造授权不生效（绑定校验 fail-closed）"
    );
}

#[tokio::test]
async fn the_list_face_is_scoped_to_the_caller_and_leaks_no_other_domain() {
    let h = harness();

    // 与 bob 无关的第三方授权。
    let carol = h
        .service
        .request_grant(ALICE, "ep-alice-1", "ns-carol", None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &carol.grant_id)
        .expect("human approval");
    // bob 自己的授权。
    let bob = h
        .service
        .request_grant(ALICE, "ep-alice-2", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &bob.grant_id)
        .expect("human approval");

    let scoped = h.service.snapshot_for(BOB);
    assert!(
        scoped
            .entries
            .iter()
            .all(|entry| entry.subject == BOB || entry.grantor == BOB),
        "按主体过滤 ⇒ 只给与该主体有关的行"
    );
    assert!(
        !scoped
            .entries
            .iter()
            .any(|entry| entry.subject == "ns-carol"),
        "别的分区的授权元数据不外泄"
    );

    // 工具面：与该行无关的调用主体看不到任何行。
    let result = h
        .tool
        .invoke_with_context(
            &call(ACTION_LIST, json!({"action": ACTION_LIST})),
            None,
            &execution_context("call_list"),
        )
        .await;
    let frame = payload(&result);
    assert_eq!(
        frame["entries"].as_array().map(Vec::len),
        Some(0),
        "无关调用主体列不到别人的授权行"
    );
}

#[test]
fn domain_isolation_is_not_breached_by_a_grant() {
    let h = harness();

    let entry = h
        .service
        .request_grant(ALICE, "ep-alice-1", BOB, None)
        .expect("grant request");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("human approval");

    // 别的域的对象仍读不到（授权只对那一条对象生效）。
    assert!(!h.service.check_read(BOB, "ep-alice-2").visible);
    // 域可见的语义没被改：域内主体照旧可见，域外主体照旧不可见。
    assert!(h.service.check_read(ALICE, "ep-alice-2").domain_visible);
    assert!(!h.service.check_read(ALICE, "ep-bob-1").domain_visible);
    // 主体自己的私有域不受影响。
    assert!(h.service.check_read(BOB, "ep-bob-1").domain_visible);
    assert!(h.service.check_read(BOB, "ep-bob-1").visible);
}
