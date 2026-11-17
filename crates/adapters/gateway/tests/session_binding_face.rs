//! B3b-1 会话 agent 绑定合同面的契约测试。
//!
//! 端点：`GET /v1/sessions/{session_id}/agent-binding` 与
//! `POST /v1/sessions/{session_id}/agent-binding`。
//!
//! 钉住的事：
//! 1. 读面只回显已存事实：未绑定 ⇒ `bound=false` 且 `binding=null`；已绑定 ⇒
//!    9 个字段齐全且与注册表里的事实一致。
//! 2. 写面必须带 `expected_revision`：缺失 ⇒ 400 且**不碰会话**；不符 ⇒ 409。
//! 3. 解绑要显式 `contact_id: null`；字段缺失 ⇒ 400。
//! 4. 绑定要过注册表：已归档的联系人 ⇒ 403。
//! 5. allow / deny 都留痕（事件 `gateway.session_binding_action`）。
//! 6. 回合闸拒绝 ⇒ HTTP 409 + `agent_binding_rejected`。
//!
//! 每条断言都对着可观测事实（状态码 / 错误码 / 账本内容 / 审计行），不用被测
//! 代码自己算出来的值当答案；每条断言都做过反向验伪（见交付说明）。

mod common;

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use apeireth_core::agent_contact::{
    Contact, ContactCreateRequest, ContactRegistry, ContractMemoryScope, InMemoryContactRegistry,
    PersonaText,
};
use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::SessionId;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, GatewayServices,
};
use apeireth_runtime::canonical::agent_binding::{resolve_agent_binding, AgentBindingRequest};
use apeireth_runtime::canonical::workspace::{
    InMemoryWorkspaceRegistry, Workspace, WorkspaceCreateRequest, WorkspacePermissionScope,
    WorkspaceRegistry,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, Session, SessionStore};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const NOW_MS: u64 = 1_700_000_000_000;

/// 审计落点（allow / deny 都往这里留一行）。
struct StubAudit {
    rows: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl AuditCommand for StubAudit {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.rows
            .lock()
            .expect("audit rows lock")
            .push((event.to_string(), detail.unwrap_or_default().to_string()));
    }
}

/// 一套夹具：运行时（带注册表）+ 会话存储 + 审计落点。
struct Harness {
    runtime: Arc<Runtime>,
    store: Arc<dyn SessionStore>,
    contacts: Arc<InMemoryContactRegistry>,
    workspaces: Arc<InMemoryWorkspaceRegistry>,
    audit: Arc<StubAudit>,
    clock: Arc<VirtualClock>,
}

async fn harness() -> Harness {
    let clock = common::test_clock();
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let provider = common::FakeProvider::new();
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let runtime = Runtime::builder()
        .with_clock(clock.clone())
        .with_session_store(store.clone())
        .with_governance(common::allow_all())
        .with_plugin(common::TestPlugin::provider(provider))
        .with_plugin(common::TestPlugin::calculator(tool_calls))
        .with_default_model(common::MODEL)
        .with_contact_registry(Some(contacts.clone() as Arc<dyn ContactRegistry>))
        .with_workspace_registry(Some(workspaces.clone() as Arc<dyn WorkspaceRegistry>))
        .build()
        .await
        .expect("runtime builds");
    let audit = Arc::new(StubAudit {
        rows: Mutex::new(Vec::new()),
    });
    Harness {
        runtime: Arc::new(runtime),
        store,
        contacts,
        workspaces,
        audit,
        clock,
    }
}

fn create_contact(
    registry: &InMemoryContactRegistry,
    display_name: &str,
    persona: Option<&str>,
) -> Contact {
    registry
        .create(ContactCreateRequest {
            display_name: display_name.to_string(),
            subtitle: None,
            avatar_ref: None,
            persona: persona.map(|raw| PersonaText::new(raw).expect("persona text is valid")),
            memory_scope: ContractMemoryScope::Session,
            now_ms: NOW_MS,
        })
        .expect("create contact")
}

fn create_workspace(registry: &InMemoryWorkspaceRegistry, root: &str) -> Workspace {
    registry
        .create(WorkspaceCreateRequest::managed(
            root,
            root,
            WorkspacePermissionScope::Read,
            NOW_MS,
        ))
        .expect("create workspace")
}

fn session_id(seed: u128) -> SessionId {
    SessionId::from_uuid(uuid::Uuid::from_u128(seed))
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request builds")
}

fn post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request builds")
}

async fn status_and_body(response: axum::response::Response) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body collects")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn error_code(body: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(body).expect("json body");
    value["error"]["code"]
        .as_str()
        .expect("error code is a string")
        .to_string()
}

// ---------------------------------------------------------------------------
// 组 1：读面 —— 未绑定
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_reports_an_unbound_session_as_bound_false() {
    let harness = harness().await;
    let session = Session::new(session_id(9101), harness.clock.as_ref());
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let response = router
        .oneshot(get(&format!("/v1/sessions/{}/agent-binding", session.id)))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        value["bound"],
        serde_json::json!(false),
        "未绑定 ⇒ bound=false"
    );
    assert!(
        value["binding"].is_null(),
        "未绑定 ⇒ binding=null, body={body}"
    );
    assert_eq!(
        value["session_id"],
        serde_json::json!(session.id.to_string()),
        "回包里带会话身份"
    );
    assert_eq!(
        value["session_revision"],
        serde_json::json!(session.revision),
        "回包里带会话修订号 (写面要拿它做 CAS)"
    );
}

// ---------------------------------------------------------------------------
// 组 2：读面 —— 已绑定 ⇒ 9 个字段齐全
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_reports_every_binding_field_for_a_bound_session() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(
        &request,
        harness.contacts.as_ref(),
        harness.workspaces.as_ref(),
        NOW_MS,
    )
    .expect("binding resolves");

    let mut session = Session::new(session_id(9102), harness.clock.as_ref());
    session.set_agent_binding(binding, harness.clock.as_ref());
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let response = router
        .oneshot(get(&format!("/v1/sessions/{}/agent-binding", session.id)))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        value["bound"],
        serde_json::json!(true),
        "已绑定 ⇒ bound=true"
    );
    let binding = &value["binding"];
    assert_eq!(
        binding["contact_id"],
        serde_json::json!(contact.contact_id().to_string()),
        "绑定字段 contact_id"
    );
    assert_eq!(
        binding["agent_id"],
        serde_json::json!(contact.agent_id().to_string()),
        "绑定字段 agent_id"
    );
    assert_eq!(
        binding["persona_revision"],
        serde_json::json!(contact.persona_revision()),
        "绑定字段 persona_revision"
    );
    assert_eq!(
        binding["workspace_id"],
        serde_json::json!(workspace.workspace_id.to_string()),
        "绑定字段 workspace_id"
    );
    assert_eq!(
        binding["workspace_revision"],
        serde_json::json!(workspace.revision),
        "绑定字段 workspace_revision"
    );
    assert!(
        binding["context_key"].is_null(),
        "context_key 只透传不生成 ⇒ null, body={body}"
    );
    assert!(
        binding["incarnation"].is_null(),
        "incarnation 只透传不生成 ⇒ null, body={body}"
    );
    assert_eq!(
        binding["revision"],
        serde_json::json!(0),
        "绑定字段 revision (绑定自身的修订号)"
    );
    assert_eq!(
        binding["updated_at_ms"],
        serde_json::json!(NOW_MS),
        "绑定字段 updated_at_ms"
    );
    for field in [
        "contact_id",
        "agent_id",
        "persona_revision",
        "workspace_id",
        "workspace_revision",
        "context_key",
        "incarnation",
        "revision",
        "updated_at_ms",
    ] {
        assert!(
            binding.get(field).is_some(),
            "绑定回包缺字段 {field}, body={body}"
        );
    }
}

// ---------------------------------------------------------------------------
// 组 3：写面 —— 缺 expected_revision ⇒ 400 且不写
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_without_expected_revision_is_rejected_and_writes_nothing() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(
        &request,
        harness.contacts.as_ref(),
        harness.workspaces.as_ref(),
        NOW_MS,
    )
    .expect("binding resolves");

    let mut session = Session::new(session_id(9103), harness.clock.as_ref());
    session.set_agent_binding(binding, harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let body = serde_json::json!({
        "contact_id": contact.contact_id().to_string(),
        "agent_id": contact.agent_id().to_string(),
        "persona_revision": contact.persona_revision(),
        "workspace_id": workspace.workspace_id.to_string(),
        "workspace_revision": workspace.revision,
    });
    let response = router
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            body,
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body_text}");
    assert_eq!(
        error_code(&body_text),
        "invalid_request",
        "缺 expected_revision ⇒ 400 invalid_request, body={body_text}"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert_eq!(
        stored.revision, before_revision,
        "缺 expected_revision ⇒ 会话 revision 没动"
    );
    assert!(
        stored.agent_binding().is_some(),
        "缺 expected_revision ⇒ 绑定没被动过"
    );
}

// ---------------------------------------------------------------------------
// 组 4：写面 —— expected_revision 不符 ⇒ 409
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_with_a_stale_expected_revision_is_a_conflict() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(
        &request,
        harness.contacts.as_ref(),
        harness.workspaces.as_ref(),
        NOW_MS,
    )
    .expect("binding resolves");

    let mut session = Session::new(session_id(9104), harness.clock.as_ref());
    session.set_agent_binding(binding, harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let body = serde_json::json!({
        "expected_revision": before_revision + 100,
        "contact_id": contact.contact_id().to_string(),
        "agent_id": contact.agent_id().to_string(),
        "persona_revision": contact.persona_revision(),
        "workspace_id": workspace.workspace_id.to_string(),
        "workspace_revision": workspace.revision,
    });
    let response = router
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            body,
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body_text}");
    assert_eq!(
        error_code(&body_text),
        "session_binding_conflict",
        "CAS 不符 ⇒ 409 session_binding_conflict, body={body_text}"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert_eq!(
        stored.revision, before_revision,
        "CAS 不符 ⇒ 会话 revision 没动"
    );
}

// ---------------------------------------------------------------------------
// 组 5：写面 —— 绑定已归档的联系人 ⇒ 403
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_binding_an_archived_contact_is_forbidden() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    harness
        .contacts
        .archive(&contact.contact_id(), contact.revision(), NOW_MS + 1)
        .expect("archive contact");

    let session = Session::new(session_id(9105), harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let body = serde_json::json!({
        "expected_revision": before_revision,
        "contact_id": contact.contact_id().to_string(),
        "agent_id": contact.agent_id().to_string(),
        "persona_revision": contact.persona_revision(),
        "workspace_id": workspace.workspace_id.to_string(),
        "workspace_revision": workspace.revision,
    });
    let response = router
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            body,
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body_text}");
    assert_eq!(
        error_code(&body_text),
        "contact_archived",
        "已归档联系人 ⇒ 403 contact_archived, body={body_text}"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert!(
        stored.agent_binding().is_none(),
        "绑定失败 ⇒ 账本上不许留半份绑定"
    );
    assert_eq!(
        stored.revision, before_revision,
        "绑定失败 ⇒ 会话 revision 没动"
    );
}

// ---------------------------------------------------------------------------
// 组 6：写面 —— 解绑（显式 null）/ 字段缺失 ⇒ 400
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_with_an_explicit_null_contact_id_unbinds_the_session() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(
        &request,
        harness.contacts.as_ref(),
        harness.workspaces.as_ref(),
        NOW_MS,
    )
    .expect("binding resolves");

    let mut session = Session::new(session_id(9106), harness.clock.as_ref());
    session.set_agent_binding(binding, harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let response = router
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            serde_json::json!({
                "expected_revision": before_revision,
                "contact_id": null,
            }),
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body_text}");
    let value: serde_json::Value = serde_json::from_str(&body_text).expect("json");
    assert_eq!(
        value["bound"],
        serde_json::json!(false),
        "解绑后 ⇒ bound=false"
    );
    assert!(
        value["binding"].is_null(),
        "解绑后 ⇒ binding=null, body={body_text}"
    );
    assert!(
        value["session_revision"].as_u64().expect("revision") > before_revision,
        "解绑推进会话 revision"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert!(stored.agent_binding().is_none(), "解绑后账本上不再有绑定");
}

#[tokio::test]
async fn post_without_a_contact_id_field_is_rejected() {
    let harness = harness().await;
    let session = Session::new(session_id(9107), harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let response = router
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            serde_json::json!({ "expected_revision": before_revision }),
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body_text}");
    assert_eq!(
        error_code(&body_text),
        "invalid_request",
        "字段缺失 ⇒ 400 (不猜客户端意图), body={body_text}"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert_eq!(
        stored.revision, before_revision,
        "字段缺失 ⇒ 会话 revision 没动"
    );
}

// ---------------------------------------------------------------------------
// 组 7：审计 —— allow 与 deny 都留痕
// ---------------------------------------------------------------------------

#[tokio::test]
async fn binding_actions_are_audited_on_both_allow_and_deny() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let session = Session::new(session_id(9108), harness.clock.as_ref());
    let revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let mut services = GatewayServices::default();
    services.audit_commands = Some(harness.audit.clone() as Arc<dyn AuditCommand>);
    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        services,
    ));

    // allow: 一次成功的绑定。
    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            serde_json::json!({
                "expected_revision": revision,
                "contact_id": contact.contact_id().to_string(),
                "agent_id": contact.agent_id().to_string(),
                "persona_revision": contact.persona_revision(),
                "workspace_id": workspace.workspace_id.to_string(),
                "workspace_revision": workspace.revision,
            }),
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body_text}");

    // deny: 一次缺 expected_revision 的绑定。
    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/sessions/{}/agent-binding", session.id),
            serde_json::json!({
                "contact_id": contact.contact_id().to_string(),
                "agent_id": contact.agent_id().to_string(),
                "persona_revision": contact.persona_revision(),
                "workspace_id": workspace.workspace_id.to_string(),
                "workspace_revision": workspace.revision,
            }),
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body_text}");

    let rows = harness.audit.rows.lock().expect("audit rows lock").clone();
    let binding_rows: Vec<&(String, String)> = rows
        .iter()
        .filter(|(event, _)| event == "gateway.session_binding_action")
        .collect();
    assert_eq!(
        binding_rows.len(),
        2,
        "allow 与 deny 各留一行, rows={rows:?}"
    );
    let allow = binding_rows
        .iter()
        .find(|(_, detail)| detail.contains("decision=allow"))
        .expect("allow 那一行");
    assert!(
        allow.1.contains("action=session_binding.create"),
        "allow 行带动作词, detail={}",
        allow.1
    );
    assert!(
        allow.1.contains(&format!("session={}", session.id)),
        "allow 行带会话身份, detail={}",
        allow.1
    );
    let deny = binding_rows
        .iter()
        .find(|(_, detail)| detail.contains("decision=deny"))
        .expect("deny 那一行");
    assert!(
        deny.1.contains("reason=missing_expected_revision"),
        "deny 行带拒绝原因, detail={}",
        deny.1
    );
}

// ---------------------------------------------------------------------------
// 组 8：回合闸拒绝 ⇒ HTTP 409 + agent_binding_rejected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_turn_rejected_by_the_gate_is_a_conflict_with_agent_binding_rejected() {
    let harness = harness().await;
    let contact = create_contact(&harness.contacts, "face-contact", Some("你是面访者。"));
    let workspace = create_workspace(&harness.workspaces, "face-workspace");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(
        &request,
        harness.contacts.as_ref(),
        harness.workspaces.as_ref(),
        NOW_MS,
    )
    .expect("binding resolves");

    // 绑定之后联系人被归档 ⇒ 回合闸必须拒。
    harness
        .contacts
        .archive(&contact.contact_id(), contact.revision(), NOW_MS + 1)
        .expect("archive contact");

    let mut session = Session::new(session_id(9109), harness.clock.as_ref());
    session.set_agent_binding(binding, harness.clock.as_ref());
    let before_revision = session.revision;
    harness.store.save(&session).await.expect("session saved");

    let router = canonical_router_with_state(build_gateway_state_with_services(
        Arc::clone(&harness.runtime),
        GatewayServices::default(),
    ));
    let response = router
        .oneshot(post(
            "/v1/chat",
            serde_json::json!({
                "session": session.id.to_string(),
                "input": "hello gate",
            }),
        ))
        .await
        .expect("response");
    let (status, body_text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body_text}");
    assert_eq!(
        error_code(&body_text),
        "agent_binding_rejected",
        "回合闸拒绝 ⇒ 409 agent_binding_rejected, body={body_text}"
    );

    let stored = harness
        .store
        .load(&session.id)
        .await
        .expect("session loads")
        .expect("session exists");
    assert_eq!(
        stored.revision, before_revision,
        "回合被拒 ⇒ 账本零写入 (revision 不动)"
    );
    assert!(
        stored.messages.is_empty(),
        "回合被拒 ⇒ 用户输入没落盘, messages={:?}",
        stored.messages.len()
    );
}
