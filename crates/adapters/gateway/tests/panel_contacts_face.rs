//! 联系人合同面的面契约 (B3a 批): 读面形状 + 显式空态 + 令牌门 + 未接线 501 +
//! 身份后端铸造 + 乐观并发 (`expected_revision`) + 归档终态 (软删除、不可复活) +
//! **404 恒定文案** + 工作区绑定校验 + 可检索审计 + 能力清单入册 + 面板边界。
//!
//! 这里钉十二件事 (对应任务书 S1..S10):
//! 1. **读面形状** —— 行字段集合逐字 (十三项, 无存储位置字段), 包形
//!    `{contacts:[...],count:n}`;
//! 2. **显式空态** —— 空列表 = 200 + `contacts: []` + `count: 0`, 不 501;
//! 3. **令牌门同面** —— 读面与写面同过门 (401 矩阵 + 真令牌 200);
//! 4. **未接线如实** —— 端口未注入 = 501 `unsupported` (不假装有数据源);
//! 5. **S1 身份后端铸造** —— 请求体带 `contact_id` / `agent_id` = 400 且端口未被调用;
//! 6. **S5 未知字段 = 400** —— 写面 `deny_unknown_fields`;
//! 7. **S6 人设校验** —— 空/控制字符/超长 = 400; `persona_revision` 客户端不可带;
//! 8. **S4 乐观并发** —— 缺 `expected_revision` = 400 且端口未被调用; 陈旧修订号 =
//!    409 `contact_conflict` + `details.{expected_revision,actual_revision}` 且
//!    **写动作根本没到端口**;
//! 9. **S3 归档终态** —— 默认列表不含墓碑, `include_archived=true` 才含; 归档后单体
//!    仍 200 且 `status=archived`; 再 PATCH = 403 `contact_archived`;
//! 10. **S7 404 恒定文案** —— 「联系人不存在或不可见」, 与 403 稳定可判;
//! 11. **S10 绑定校验** —— 工作区不存在/已吊销/修订号不符 = 409 `contact_binding_conflict`
//!     且字段零变化; 工作区面未装配 = 501 (fail-closed, 不先收下再说);
//! 12. **S8/S9 审计 + 能力清单 + 面板边界** —— 动作词落审计行 (allow/deny 都留痕);
//!     `contacts.read` / `contacts.write` 入能力清单; 无越权路由。
//!
//! 真数据面 (真 `FileContactRegistry` 落盘 + 真绑定档) 的端到端钉在组合根侧
//! `crates/adapters/cli/tests/contact_real_chain.rs` (同一端口)。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand,
    ContactArchiveCommand, ContactCapabilitiesDto, ContactCreateCommand, ContactDto,
    ContactMemoryScopeDto, ContactPanel, ContactPanelError, ContactStatusDto, ContactUpdateCommand,
    ContactWorkspaceBindingDto, GatewayServices, GatewayState, GatewayTokenGate,
    WorkspaceCreateCommand, WorkspaceDto, WorkspacePanel, WorkspacePanelError,
};
use apeireth_runtime::canonical::workspace::{
    WorkspacePermissionScope, WorkspaceRootKind, WorkspaceStatus,
};
use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-contacts-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";
const CONTACT_ID: &str = "3a5c1d20-8b4f-4c9d-9e77-2f6a1b3c4d5e";
const AGENT_ID: &str = "7c9e2f31-4a6b-4d8e-8f10-9b2c3d4e5f60";
const UNKNOWN_CONTACT_ID: &str = "00000000-0000-4000-8000-000000000000";
const WORKSPACE_ID: &str = "6f1b0f6c-4f0a-4f6f-9a3a-1b2c3d4e5f60";

/// 联系人端口桩: 回固定行, 记录写动作入参 (面形状与门语义不需要真注册表)。
#[derive(Default)]
struct StubContactPanel {
    rows: Mutex<Vec<ContactDto>>,
    created: Mutex<Vec<ContactCreateCommand>>,
    updated: Mutex<Vec<ContactUpdateCommand>>,
    archived: Mutex<Vec<ContactArchiveCommand>>,
    listed_with_archived: Mutex<Vec<bool>>,
}

#[async_trait]
impl ContactPanel for StubContactPanel {
    async fn list_contacts(&self, include_archived: bool) -> Result<Vec<ContactDto>, String> {
        self.listed_with_archived
            .lock()
            .expect("锁")
            .push(include_archived);
        // 桩**故意不过滤墓碑**: 「默认不含已归档」是**面**的合同, 由面自己守;
        // 桩要是先滤掉了, 这条合同就没人盯了。
        Ok(self.rows.lock().expect("锁").clone())
    }

    async fn get_contact(&self, contact_id: &str) -> Result<ContactDto, ContactPanelError> {
        self.rows
            .lock()
            .expect("锁")
            .iter()
            .find(|row| row.contact_id == contact_id)
            .cloned()
            .ok_or_else(|| ContactPanelError::NotFound {
                contact_id: contact_id.to_string(),
            })
    }

    async fn create_contact(
        &self,
        command: ContactCreateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        self.created.lock().expect("锁").push(command.clone());
        Ok(sample_contact(
            UNKNOWN_CONTACT_ID,
            0,
            ContactStatusDto::Draft,
            &command.display_name,
        ))
    }

    async fn update_contact(
        &self,
        command: ContactUpdateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        self.updated.lock().expect("锁").push(command.clone());
        Ok(sample_contact(
            &command.contact_id,
            command.expected_revision + 1,
            ContactStatusDto::Ready,
            "校对发布清单",
        ))
    }

    async fn archive_contact(
        &self,
        command: ContactArchiveCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        self.archived.lock().expect("锁").push(command.clone());
        Ok(sample_contact(
            &command.contact_id,
            command.expected_revision + 1,
            ContactStatusDto::Archived,
            "校对发布清单",
        ))
    }
}

/// 工作区端口桩: 绑定校验只需要「存在 / 已吊销 / 修订号」三样事实。
#[derive(Default)]
struct StubWorkspacePanel {
    rows: Mutex<Vec<WorkspaceDto>>,
}

#[async_trait]
impl WorkspacePanel for StubWorkspacePanel {
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceDto>, String> {
        Ok(self.rows.lock().expect("锁").clone())
    }

    async fn get_workspace(&self, workspace_id: &str) -> Result<WorkspaceDto, WorkspacePanelError> {
        self.rows
            .lock()
            .expect("锁")
            .iter()
            .find(|row| row.workspace_id == workspace_id)
            .cloned()
            .ok_or_else(|| WorkspacePanelError::NotFound {
                workspace_id: workspace_id.to_string(),
            })
    }

    async fn create_workspace(
        &self,
        command: WorkspaceCreateCommand,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        Ok(sample_workspace(
            WORKSPACE_ID,
            0,
            WorkspaceStatus::Ready,
            &command.display_name,
        ))
    }

    async fn select_workspace(
        &self,
        workspace_id: &str,
        _expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        self.get_workspace(workspace_id).await
    }

    async fn close_workspace(
        &self,
        workspace_id: &str,
        _expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        self.get_workspace(workspace_id).await
    }
}

/// 审计端口桩: 记录 (事件, 详情) 对, 供可检索审计断言。
#[derive(Default)]
struct StubAudit {
    rows: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl AuditCommand for StubAudit {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.rows
            .lock()
            .expect("锁")
            .push((event.to_string(), detail.unwrap_or_default().to_string()));
    }
}

fn sample_workspace(
    id: &str,
    revision: u64,
    status: WorkspaceStatus,
    display_name: &str,
) -> WorkspaceDto {
    WorkspaceDto {
        workspace_id: id.to_string(),
        display_name: display_name.to_string(),
        root_kind: WorkspaceRootKind::Managed,
        root_display: "release".to_string(),
        permission_scope: WorkspacePermissionScope::Write,
        revision,
        status,
        can_select: status != WorkspaceStatus::Revoked,
        can_create_temporary: status != WorkspaceStatus::Revoked,
        can_write: true,
        expires_at: None,
        source: "cli".to_string(),
        updated_at_ms: 1_700_000_000_000,
    }
}

fn sample_contact(
    id: &str,
    revision: u64,
    status: ContactStatusDto,
    display_name: &str,
) -> ContactDto {
    ContactDto {
        contact_id: id.to_string(),
        agent_id: AGENT_ID.to_string(),
        display_name: display_name.to_string(),
        subtitle: Some("发布校对".to_string()),
        avatar_ref: None,
        status,
        persona_revision: 1,
        memory_scope: ContactMemoryScopeDto::Contact,
        workspace_binding: None,
        capabilities: ContactCapabilitiesDto {
            read: true,
            write: false,
            approval_required: true,
        },
        persona: Some("盯住发布清单的每一行".to_string()),
        revision,
        updated_at_ms: 1_700_000_000_000,
    }
}

fn ready_row() -> ContactDto {
    sample_contact(CONTACT_ID, 7, ContactStatusDto::Ready, "校对发布清单")
}

fn archived_row() -> ContactDto {
    sample_contact(CONTACT_ID, 9, ContactStatusDto::Archived, "校对发布清单")
}

async fn harness(services: GatewayServices, gate: GatewayTokenGate) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    request
}

fn get(uri: &str, token: Option<&str>) -> Request<Body> {
    request("GET", uri, token, Body::empty())
}

fn post(uri: &str, token: Option<&str>, json: serde_json::Value) -> Request<Body> {
    let mut request = request("POST", uri, token, Body::from(json.to_string()));
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    request
}

fn patch(uri: &str, token: Option<&str>, json: serde_json::Value) -> Request<Body> {
    let mut request = request("PATCH", uri, token, Body::from(json.to_string()));
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    request
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn stub_services(rows: Vec<ContactDto>) -> (GatewayServices, Arc<StubContactPanel>) {
    let panel = Arc::new(StubContactPanel {
        rows: Mutex::new(rows),
        ..StubContactPanel::default()
    });
    let services = GatewayServices {
        contacts: Some(panel.clone() as Arc<dyn ContactPanel>),
        audit_commands: Some(Arc::new(StubAudit::default()) as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    (services, panel)
}

fn stub_services_with_workspaces(
    rows: Vec<ContactDto>,
    workspaces: Vec<WorkspaceDto>,
) -> (GatewayServices, Arc<StubContactPanel>) {
    let (mut services, panel) = stub_services(rows);
    services.workspaces = Some(Arc::new(StubWorkspacePanel {
        rows: Mutex::new(workspaces),
    }) as Arc<dyn WorkspacePanel>);
    (services, panel)
}

/// 1. 读面形状: 行字段集合逐字 (十三项) + 包形 `{contacts, count}`。
#[tokio::test]
async fn contact_list_shape_is_the_frozen_contract() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let mut envelope_keys: Vec<String> = value
        .as_object()
        .expect("envelope")
        .keys()
        .cloned()
        .collect();
    envelope_keys.sort();
    assert_eq!(envelope_keys, vec!["contacts", "count"]);

    let row = &value["contacts"][0];
    let mut row_keys: Vec<String> = row.as_object().expect("row").keys().cloned().collect();
    row_keys.sort();
    assert_eq!(
        row_keys,
        vec![
            "agent_id",
            "avatar_ref",
            "capabilities",
            "contact_id",
            "display_name",
            "memory_scope",
            "persona",
            "persona_revision",
            "revision",
            "status",
            "subtitle",
            "updated_at_ms",
            "workspace_binding",
        ],
        "行字段集合必须逐字冻结"
    );
    assert_eq!(row["contact_id"], CONTACT_ID);
    assert_eq!(row["agent_id"], AGENT_ID);
    assert_eq!(row["status"], "ready");
    assert_eq!(row["memory_scope"], "contact");
    assert_eq!(row["persona_revision"], 1);
    assert_eq!(row["revision"], 7);
    assert_eq!(row["updated_at_ms"], 1_700_000_000_000u64);
    assert_eq!(row["workspace_binding"], serde_json::Value::Null);
    assert_eq!(row["capabilities"]["read"], true);
    assert_eq!(row["capabilities"]["write"], false);
    assert_eq!(row["capabilities"]["approval_required"], true);
    assert_eq!(value["count"], 1);

    // 读面**绝不**含宿主存储位置: 整段正文不得出现绝对路径形状。
    assert!(
        !body.contains("://") && !body.contains('\\') && !body.contains(":\\"),
        "读面不得泄露宿主路径: {body}"
    );
}

/// 2. 显式空态: 空列表 = 200 + `contacts: []` + `count: 0` (不是 501)。
#[tokio::test]
async fn contact_list_empty_state_is_explicit() {
    let (services, _panel) = stub_services(vec![]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["contacts"], serde_json::json!([]));
    assert_eq!(value["count"], 0);
}

/// 3. 令牌门同面: 读面与写面同过门 (缺令牌 / 伪令牌 = 401, 真令牌 = 200)。
#[tokio::test]
async fn contact_face_shares_the_token_gate() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(state);

    for (token, expected) in [
        (None, "auth_missing_key"),
        (Some(FORGED_TOKEN), "auth_invalid_key"),
    ] {
        let response = router
            .clone()
            .oneshot(get("/v1/agents/contacts", token))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "body={body}");
        let value: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(value["error"]["code"], expected, "body={body}");
    }

    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", Some(TOKEN)))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    // 写面同门。
    let response = router
        .clone()
        .oneshot(post(
            "/v1/agents/contacts",
            None,
            serde_json::json!({"display_name": "校对发布清单"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body={body}");
}

/// 4. 未接线如实: 端口未注入 = 501 `unsupported` (读面/写面各自报能力 id)。
#[tokio::test]
async fn contact_face_without_port_reports_unsupported() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "unsupported");
    assert!(body.contains("contacts.read"), "帧里必须点名能力: {body}");

    let response = router
        .clone()
        .oneshot(post(
            "/v1/agents/contacts",
            None,
            serde_json::json!({"display_name": "校对发布清单"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "body={body}");
    assert!(body.contains("contacts.write"), "帧里必须点名能力: {body}");
}

/// 5. S1: 身份由**后端铸造** —— 请求体带 `contact_id` / `agent_id` = 400 且端口未被调用。
#[tokio::test]
async fn contact_identity_is_minted_by_the_backend() {
    let (services, panel) = stub_services(vec![]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    for body in [
        serde_json::json!({"display_name": "校对发布清单", "contact_id": CONTACT_ID}),
        serde_json::json!({"display_name": "校对发布清单", "agent_id": AGENT_ID}),
    ] {
        let response = router
            .clone()
            .oneshot(post("/v1/agents/contacts", None, body))
            .await
            .expect("response");
        let (status, text) = status_and_body(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body={text}");
        let value: serde_json::Value = serde_json::from_str(&text).expect("json");
        assert_eq!(value["error"]["code"], "invalid_request");
    }
    assert!(
        panel.created.lock().expect("锁").is_empty(),
        "身份字段被拒时端口根本不该被调用"
    );
}

/// 6. S5: 未知字段一律 400 (写面 `deny_unknown_fields`), 端口不被调用。
#[tokio::test]
async fn contact_write_faces_reject_unknown_fields() {
    let (services, panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post(
            "/v1/agents/contacts",
            None,
            serde_json::json!({"display_name": "校对发布清单", "root": "/etc"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");

    // S6: `persona_revision` 由后端递增, 客户端不可带。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 7, "persona_revision": 9}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");

    // 归档面同样拒未知字段。
    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/agents/contacts/{CONTACT_ID}/archive"),
            None,
            serde_json::json!({"expected_revision": 7, "status": "archived"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");

    assert!(
        panel.created.lock().expect("锁").is_empty()
            && panel.updated.lock().expect("锁").is_empty()
            && panel.archived.lock().expect("锁").is_empty(),
        "请求体不合法时端口根本不该被调用"
    );
}

/// 7. S6: 人设校验走领域层 `PersonaText` (空 / 控制字符 / 超长 = 400)。
#[tokio::test]
async fn contact_persona_is_validated_by_the_domain() {
    let (services, panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let long_persona = "人".repeat(4097);
    for persona in ["", "   ", "带\u{0007}控制符", &long_persona] {
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({"expected_revision": 7, "persona": persona}),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "persona={persona:?} body={body}"
        );
    }

    // 合法人设 ⇒ 端口收到的是**已校验**的正文。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 7, "persona": "盯住发布清单的每一行"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let updates = panel.updated.lock().expect("锁").clone();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0].persona.as_ref().expect("人设").as_str(),
        "盯住发布清单的每一行"
    );
}

/// 8. S4: 乐观并发 —— 缺 `expected_revision` = 400 且端口未被调用; 陈旧修订号 = 409 +
/// `details.{expected_revision,actual_revision}` 且端口未被调用、字段逐字不变。
#[tokio::test]
async fn contact_write_faces_require_a_matching_revision() {
    let (services, panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    // 缺修订号 ⇒ 400, 端口未被调用。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"persona": "盯住发布清单的每一行"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(
        panel.updated.lock().expect("锁").is_empty(),
        "缺修订号时写动作根本不该到端口"
    );

    // 归档面同样必填修订号。
    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/agents/contacts/{CONTACT_ID}/archive"),
            None,
            serde_json::json!({}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert!(
        panel.archived.lock().expect("锁").is_empty(),
        "缺修订号时写动作根本不该到端口"
    );

    // 陈旧修订号 ⇒ 409 + details + 端口未被调用。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 3, "persona": "新的正文"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "contact_conflict");
    assert_eq!(value["error"]["details"]["expected_revision"], 3);
    assert_eq!(value["error"]["details"]["actual_revision"], 7);
    assert!(
        panel.updated.lock().expect("锁").is_empty(),
        "过期调用者根本不该碰写端口"
    );

    // 冲突后单体的字段逐字不变。
    let response = router
        .clone()
        .oneshot(get(&format!("/v1/agents/contacts/{CONTACT_ID}"), None))
        .await
        .expect("response");
    let (status, after) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK);
    let value: serde_json::Value = serde_json::from_str(&after).expect("json");
    assert_eq!(value["contact"]["revision"], 7);
    assert_eq!(value["contact"]["persona"], "盯住发布清单的每一行");
    assert_eq!(value["contact"]["status"], "ready");
}

/// 9. S3: 归档是终态 —— 默认列表不含墓碑, `include_archived=true` 才含; 归档后单体仍
/// 200 且 `status=archived`; 再 PATCH = 403 `contact_archived` 且字段零变化。
#[tokio::test]
async fn contact_archive_is_a_terminal_tombstone() {
    let (services, panel) = stub_services(vec![archived_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    // 默认列表不含墓碑。
    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["count"], 0, "默认列表不得含已归档");
    assert_eq!(
        panel.listed_with_archived.lock().expect("锁").as_slice(),
        &[false]
    );

    // 显式带上 `include_archived=true` 才看得见。
    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts?include_archived=true", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["count"], 1);
    assert_eq!(value["contacts"][0]["status"], "archived");

    // 归档后单体仍可读 (墓碑是事实, 不是消失)。
    let response = router
        .clone()
        .oneshot(get(&format!("/v1/agents/contacts/{CONTACT_ID}"), None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["contact"]["status"], "archived");

    // 再 PATCH ⇒ 403 终态, 不是「版本过期」。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 9, "persona": "想复活"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "contact_archived");
    assert!(
        panel.updated.lock().expect("锁").is_empty(),
        "归档是终态: 写动作根本不该到端口"
    );

    // 单体字段逐字不变 (没有复活)。
    let response = router
        .clone()
        .oneshot(get(&format!("/v1/agents/contacts/{CONTACT_ID}"), None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK);
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["contact"]["status"], "archived");
    assert_eq!(value["contact"]["revision"], 9);
}

/// 10. S7: 404 恒定文案「联系人不存在或不可见」, 与 403 稳定可判。
#[tokio::test]
async fn contact_not_found_message_is_constant() {
    let (services, _panel) = stub_services(vec![]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get(
            &format!("/v1/agents/contacts/{UNKNOWN_CONTACT_ID}"),
            None,
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "contact_not_found");
    assert_eq!(value["error"]["message"], "联系人不存在或不可见");

    // 不存在的身份在写面也是同一句 404 (不泄露存在性)。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{UNKNOWN_CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 1, "persona": "正文"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["message"], "联系人不存在或不可见");

    // 格式非法的身份 = 400 (不是 404: 连身份都不成立)。
    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts/not-a-uuid", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
}

/// 11. S10: 绑定声明必须过工作区事实校验 —— 不存在 / 已吊销 / 修订号不符 = 409 且
/// 字段零变化; 工作区面未装配 = 501 (fail-closed, 不先收下再说)。
#[tokio::test]
async fn contact_binding_is_validated_against_workspace_facts() {
    let ready_workspace = sample_workspace(WORKSPACE_ID, 5, WorkspaceStatus::Ready, "默认工作区");
    let revoked_workspace =
        sample_workspace(WORKSPACE_ID, 5, WorkspaceStatus::Revoked, "默认工作区");

    // 工作区不存在 ⇒ 409 + 稳定原因标签。
    {
        let (services, panel) = stub_services_with_workspaces(vec![ready_row()], vec![]);
        let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
        let router = canonical_router_with_state(state);
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({
                    "expected_revision": 7,
                    "workspace_binding": {"workspace_id": WORKSPACE_ID, "revision": 5},
                }),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::CONFLICT, "body={body}");
        let value: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(value["error"]["code"], "contact_binding_conflict");
        assert_eq!(value["error"]["details"]["reason"], "workspace_not_found");
        assert_eq!(value["error"]["details"]["workspace_id"], WORKSPACE_ID);
        assert!(
            panel.updated.lock().expect("锁").is_empty(),
            "绑定冲突时写动作根本不该到端口"
        );
    }

    // 工作区已吊销 ⇒ 409。
    {
        let (services, panel) =
            stub_services_with_workspaces(vec![ready_row()], vec![revoked_workspace]);
        let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
        let router = canonical_router_with_state(state);
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({
                    "expected_revision": 7,
                    "workspace_binding": {"workspace_id": WORKSPACE_ID, "revision": 5},
                }),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::CONFLICT, "body={body}");
        let value: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(value["error"]["details"]["reason"], "workspace_revoked");
        assert!(panel.updated.lock().expect("锁").is_empty());
    }

    // 修订号不精确匹配 ⇒ 409。
    {
        let (services, panel) =
            stub_services_with_workspaces(vec![ready_row()], vec![ready_workspace.clone()]);
        let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
        let router = canonical_router_with_state(state);
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({
                    "expected_revision": 7,
                    "workspace_binding": {"workspace_id": WORKSPACE_ID, "revision": 4},
                }),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::CONFLICT, "body={body}");
        let value: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(
            value["error"]["details"]["reason"],
            "workspace_revision_mismatch"
        );
        assert!(panel.updated.lock().expect("锁").is_empty());
    }

    // 工作区面未装配 ⇒ 501 fail-closed (不是「先收下再说」)。
    {
        let (services, panel) = stub_services(vec![ready_row()]);
        let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
        let router = canonical_router_with_state(state);
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({
                    "expected_revision": 7,
                    "workspace_binding": {"workspace_id": WORKSPACE_ID, "revision": 5},
                }),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "body={body}");
        assert!(
            body.contains("contacts.binding"),
            "帧里必须点名能力: {body}"
        );
        assert!(panel.updated.lock().expect("锁").is_empty());
    }

    // 绑定合法 ⇒ 放行, 且**绑定档只带 id + 修订号** (不带工作区根路径)。
    {
        let (services, panel) =
            stub_services_with_workspaces(vec![ready_row()], vec![ready_workspace]);
        let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
        let router = canonical_router_with_state(state);
        let response = router
            .clone()
            .oneshot(patch(
                &format!("/v1/agents/contacts/{CONTACT_ID}"),
                None,
                serde_json::json!({
                    "expected_revision": 7,
                    "workspace_binding": {"workspace_id": WORKSPACE_ID, "revision": 5},
                }),
            ))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        let updates = panel.updated.lock().expect("锁").clone();
        assert_eq!(updates.len(), 1);
        assert!(
            !body.contains("://") && !body.contains('\\'),
            "绑定不得回显工作区根路径: {body}"
        );
    }
}

/// 12. S8: 审计事件名唯一 + 动作词 + allow/deny 都留痕 + 详情不含路径。
#[tokio::test]
async fn contact_actions_are_auditable() {
    let (services, _panel, audit) = stub_services_with_audit(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    // allow: 建 / 改 / 归档。
    let response = router
        .clone()
        .oneshot(post(
            "/v1/agents/contacts",
            None,
            serde_json::json!({"display_name": "校对发布清单"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");

    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 7, "persona": "盯住发布清单的每一行"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/agents/contacts/{CONTACT_ID}/archive"),
            None,
            serde_json::json!({"expected_revision": 7}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    // deny: 过期修订号。
    let response = router
        .clone()
        .oneshot(patch(
            &format!("/v1/agents/contacts/{CONTACT_ID}"),
            None,
            serde_json::json!({"expected_revision": 1, "persona": "正文"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");

    let audit_rows = audit.rows.lock().expect("锁").clone();
    assert!(!audit_rows.is_empty(), "审计行不得为空");
    for (event, detail) in &audit_rows {
        assert_eq!(event, "gateway.contact_action", "事件名必须唯一");
        assert!(
            !detail.contains("://") && !detail.contains('\\') && !detail.contains(":\\"),
            "审计详情不得含绝对路径: {detail}"
        );
    }
    let details: Vec<String> = audit_rows
        .iter()
        .map(|(_, detail)| detail.clone())
        .collect();
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=contact.create")),
        "建面动作词必须入审计: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=contact.update")),
        "改面动作词必须入审计: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=contact.archive")),
        "归档面动作词必须入审计: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=contact.deny")),
        "被拒动作也必须留痕: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("decision=allow")),
        "放行判定必须留痕: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("decision=deny")),
        "拒绝判定必须留痕: {details:?}"
    );
}

/// 13. S9: 能力清单入册 —— `contacts.read` / `contacts.write` 分开登记; 未接线时
/// `supported` / `available` = false 且带 reason。
#[tokio::test]
async fn contact_capabilities_are_registered_with_wiring_truth() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/apeireth/capabilities", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let group = value["capabilities"]
        .as_array()
        .expect("groups")
        .iter()
        .find(|group| group["name"] == "contacts")
        .expect("contacts 分组必须入册");
    let caps = group["capabilities"].as_array().expect("caps");
    let read = caps
        .iter()
        .find(|cap| cap["id"] == "contacts.read")
        .expect("contacts.read");
    assert_eq!(read["read"], true);
    assert_eq!(read["write"], false);
    assert_eq!(read["supported"], true);
    assert_eq!(read["available"], true);
    assert_eq!(read["operations"], serde_json::json!(["list", "get"]));
    let write = caps
        .iter()
        .find(|cap| cap["id"] == "contacts.write")
        .expect("contacts.write");
    assert_eq!(write["read"], false);
    assert_eq!(write["write"], true);
    assert_eq!(write["supported"], true);
    assert_eq!(write["available"], true);
    assert_eq!(
        write["operations"],
        serde_json::json!(["create", "update", "archive"])
    );

    // 未接线时如实报 false + reason (不假装有数据源)。
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let response = router
        .clone()
        .oneshot(get("/v1/apeireth/capabilities", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let group = value["capabilities"]
        .as_array()
        .expect("groups")
        .iter()
        .find(|group| group["name"] == "contacts")
        .expect("contacts 分组必须入册");
    for cap in group["capabilities"].as_array().expect("caps") {
        assert_eq!(cap["supported"], false, "cap={cap}");
        assert_eq!(cap["available"], false, "cap={cap}");
        assert!(cap["reason"].is_string(), "cap={cap}");
    }
}

/// 14. 面板边界: 无删除 / 复活 / 改派路由 (写面只有建 / 改 / 归档三条)。
#[tokio::test]
async fn contact_face_exposes_no_delete_or_resurrect_routes() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    for (method, uri) in [
        ("DELETE", format!("/v1/agents/contacts/{CONTACT_ID}")),
        ("PUT", format!("/v1/agents/contacts/{CONTACT_ID}")),
        ("GET", format!("/v1/agents/contacts/{CONTACT_ID}/archive")),
        ("POST", format!("/v1/agents/contacts/{CONTACT_ID}/restore")),
        (
            "POST",
            format!("/v1/agents/contacts/{CONTACT_ID}/unarchive"),
        ),
        ("GET", "/v1/panel/contacts".to_string()),
    ] {
        let response = router
            .clone()
            .oneshot(request(&method, &uri, None, Body::empty()))
            .await
            .expect("response");
        let (status, body) = status_and_body(response).await;
        assert!(
            matches!(
                status,
                StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
            ),
            "{method} {uri} 越权面必须不存在, 实际 {status}: {body}"
        );
    }
}

/// 带审计桩的装配 (审计断言要自己握桩: state 只暴露端口)。
fn stub_services_with_audit(
    rows: Vec<ContactDto>,
) -> (GatewayServices, Arc<StubContactPanel>, Arc<StubAudit>) {
    let (mut services, panel) = stub_services(rows);
    let audit = Arc::new(StubAudit::default());
    services.audit_commands = Some(audit.clone() as Arc<dyn AuditCommand>);
    (services, panel, audit)
}

/// 15. S2: 同名不碰撞 —— 同名不同身份的两个联系人都能列出, 互不影响。
#[tokio::test]
async fn contact_same_display_name_does_not_collide() {
    let first = sample_contact(CONTACT_ID, 7, ContactStatusDto::Ready, "校对发布清单");
    let second = sample_contact(
        UNKNOWN_CONTACT_ID,
        1,
        ContactStatusDto::Draft,
        "校对发布清单",
    );
    let (services, _panel) = stub_services(vec![first, second]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/agents/contacts", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["count"], 2, "同名不得互相顶掉: {body}");
    let names: Vec<String> = value["contacts"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["display_name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        names,
        vec!["校对发布清单".to_string(), "校对发布清单".to_string()]
    );
    let ids: Vec<String> = value["contacts"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["contact_id"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_ne!(ids[0], ids[1], "身份必须各自独立");
}
