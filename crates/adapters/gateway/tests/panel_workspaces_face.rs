//! 工作区合同面的面契约 (A2 批): 读面形状 + 显式空态 + 令牌门 + 未接线 501 +
//! 写面乐观并发 (`expected_revision`) + **读面绝不泄露绝对根路径** +
//! 可检索审计 + 能力清单入册 + 面板边界。
//!
//! 这里钉十件事:
//! 1. **读面形状** —— 行字段集合逐字 (十三项, 无根路径字段), 包形
//!    `{workspaces:[...],count:n}`;
//! 2. **显式空态** —— 空列表 = 200 + `workspaces: []` + `count: 0`, 不 501;
//! 3. **有界列表** —— `?limit=` 真的截断行数;
//! 4. **令牌门同面** —— 读面与写面同过 W2-② 门 (401 矩阵 + 真令牌 200);
//! 5. **未接线如实** —— 端口未注入 = 501 `unsupported` (不假装有数据源);
//! 6. **乐观并发** —— 缺 `expected_revision` = 400 `invalid_request`; 陈旧修订号 =
//!    409 `workspace_conflict` 且**写动作根本没到端口** (工作区逐字段不变);
//! 7. **吊销 ≠ 冲突** —— 已吊销 = 403 `workspace_revoked`;
//! 8. **客户端不能塞根路径** —— 请求体带 `root` = 400 (受管根由宿主决定);
//! 9. **不泄露** —— 读面任何字段都不含绝对路径; 端口交回不安全展示名 = 500 且
//!    **不回显**该值;
//! 10. **可检索审计 + 能力清单 + 面板边界** —— 动作词落审计行; `workspaces.read` /
//!     `workspaces.write` 入能力清单; 无越权路由。
//!
//! 真数据面 (真 `InMemoryWorkspaceRegistry` + 真受管根) 的端到端钉在组合根侧
//! `crates/adapters/cli/tests/workspace_real_chain.rs` (同一端口)。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, GatewayServices,
    GatewayState, GatewayTokenGate, WorkspaceCreateCommand, WorkspaceDto, WorkspacePanel,
    WorkspacePanelError,
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

const TOKEN: &str = "panel-workspaces-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";
const WORKSPACE_ID: &str = "6f1b0f6c-4f0a-4f6f-9a3a-1b2c3d4e5f60";
const UNKNOWN_WORKSPACE_ID: &str = "00000000-0000-4000-8000-000000000000";

/// 端口桩: 回固定行, 记录写动作入参 (面形状与门语义不需要真注册表)。
#[derive(Default)]
struct StubWorkspacePanel {
    rows: Mutex<Vec<WorkspaceDto>>,
    created: Mutex<Vec<WorkspaceCreateCommand>>,
    selected: Mutex<Vec<(String, u64)>>,
    closed: Mutex<Vec<(String, u64)>>,
    select_result: Mutex<Option<Result<WorkspaceDto, WorkspacePanelError>>>,
    close_result: Mutex<Option<Result<WorkspaceDto, WorkspacePanelError>>>,
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
        self.created.lock().expect("锁").push(command.clone());
        Ok(sample_workspace(
            UNKNOWN_WORKSPACE_ID,
            0,
            WorkspaceStatus::Ready,
            &command.display_name,
        ))
    }

    async fn select_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        self.selected
            .lock()
            .expect("锁")
            .push((workspace_id.to_string(), expected_revision));
        let preset = self.select_result.lock().expect("锁").take();
        match preset {
            Some(result) => result,
            None => self.get_workspace(workspace_id).await,
        }
    }

    async fn close_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        self.closed
            .lock()
            .expect("锁")
            .push((workspace_id.to_string(), expected_revision));
        let preset = self.close_result.lock().expect("锁").take();
        match preset {
            Some(result) => result,
            None => self.get_workspace(workspace_id).await,
        }
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
        expires_at: Some(1_800_000_000_000),
        source: "cli".to_string(),
        updated_at_ms: 1_700_000_000_000,
    }
}

fn ready_row() -> WorkspaceDto {
    sample_workspace(WORKSPACE_ID, 7, WorkspaceStatus::Ready, "校对发布清单")
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

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn stub_services(rows: Vec<WorkspaceDto>) -> (GatewayServices, Arc<StubWorkspacePanel>) {
    let panel = Arc::new(StubWorkspacePanel {
        rows: Mutex::new(rows),
        ..StubWorkspacePanel::default()
    });
    let services = GatewayServices {
        workspaces: Some(panel.clone() as Arc<dyn WorkspacePanel>),
        audit_commands: Some(Arc::new(StubAudit::default()) as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    (services, panel)
}

/// 1. 读面形状: 行字段集合逐字 (十三项) + 包形 `{workspaces, count}`。
#[tokio::test]
async fn workspace_list_shape_is_the_frozen_contract() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", Some(TOKEN)))
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
    assert_eq!(envelope_keys, vec!["count", "workspaces"]);

    let row = &value["workspaces"][0];
    let mut row_keys: Vec<String> = row.as_object().expect("row").keys().cloned().collect();
    row_keys.sort();
    assert_eq!(
        row_keys,
        vec![
            "can_create_temporary",
            "can_select",
            "can_write",
            "display_name",
            "expires_at",
            "permission_scope",
            "revision",
            "root_display",
            "root_kind",
            "source",
            "status",
            "updated_at_ms",
            "workspace_id",
        ],
        "行字段集合必须逐字冻结"
    );
    assert_eq!(row["workspace_id"], WORKSPACE_ID);
    assert_eq!(row["root_kind"], "managed");
    assert_eq!(row["permission_scope"], "write");
    assert_eq!(row["status"], "ready");
    assert_eq!(row["revision"], 7);
    assert_eq!(row["updated_at_ms"], 1_700_000_000_000i64);
    assert_eq!(row["expires_at"], 1_800_000_000_000i64);
    assert_eq!(value["count"], 1);

    // 无过期时间时该键**省略** (Option 走 skip_serializing_if, 不回显 null)。
    let mut without_expiry = ready_row();
    without_expiry.expires_at = None;
    let (services, _panel) = stub_services(vec![without_expiry]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(
        value["workspaces"][0].get("expires_at").is_none(),
        "无过期时间时不得回显该键: {body}"
    );
}

/// 2. 显式空态: 空列表是 200 + 空数组 + count 0 (不 501)。
#[tokio::test]
async fn empty_workspace_list_is_an_explicit_empty_state() {
    let (services, _panel) = stub_services(Vec::new());
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["workspaces"], serde_json::json!([]));
    assert_eq!(value["count"], 0);
}

/// 3. 有界列表: `?limit=` 真的截断行数。
#[tokio::test]
async fn workspace_list_is_bounded_by_the_limit_query() {
    let (services, _panel) = stub_services(vec![ready_row(), ready_row(), ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces?limit=1", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["count"], 1);
    assert_eq!(value["workspaces"].as_array().expect("rows").len(), 1);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces?limit=0", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "invalid_request");
}

/// 4. 令牌门同面: 读面与写面同过 W2-② 门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn workspace_endpoints_sit_behind_the_gateway_token_gate() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "auth_missing_key");
    assert!(!body.contains(TOKEN), "401 帧不得回显令牌");

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", Some(FORGED_TOKEN)))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "auth_invalid_key");
    assert!(!body.contains(FORGED_TOKEN), "401 帧不得回显令牌");

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", Some(TOKEN)))
        .await
        .expect("response");
    let (status, _body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK);

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/select"),
            Some(TOKEN),
            serde_json::json!({"expected_revision": 7}),
        ))
        .await
        .expect("response");
    let (status, _body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK);
}

/// 5. 未接线如实: 端口未注入 = 501 `unsupported`。
#[tokio::test]
async fn workspace_face_degrades_to_501_without_the_port() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "unsupported");
    assert!(
        body.contains("workspaces.read"),
        "降级帧必须点名能力项: body={body}"
    );

    let response = router
        .clone()
        .oneshot(post(
            "/v1/workspaces",
            None,
            serde_json::json!({"display_name": "x"}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "body={body}");
    assert!(body.contains("workspaces.write"), "body={body}");
}

/// 6. 乐观并发 (一): 缺 `expected_revision` = 400 `invalid_request`, 且写动作没到端口。
#[tokio::test]
async fn missing_expected_revision_is_a_400_and_never_reaches_the_port() {
    let (services, panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/select"),
            None,
            serde_json::json!({}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(
        panel.selected.lock().expect("锁").is_empty(),
        "缺修订号时写动作不得到达端口"
    );

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/close"),
            None,
            serde_json::json!({}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert!(panel.closed.lock().expect("锁").is_empty());
}

/// 7. 乐观并发 (二): 陈旧修订号 = 409 `workspace_conflict`, 工作区逐字段不变。
#[tokio::test]
async fn stale_expected_revision_is_a_409_conflict_that_changes_nothing() {
    let (services, panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let before = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (_status, before_body) = status_and_body(before).await;

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/select"),
            None,
            serde_json::json!({"expected_revision": 3}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "workspace_conflict");
    assert_eq!(value["error"]["details"]["expected_revision"], 3);
    assert_eq!(value["error"]["details"]["actual_revision"], 7);
    assert!(
        panel.selected.lock().expect("锁").is_empty(),
        "冲突时写动作不得到达端口"
    );

    let after = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (_status, after_body) = status_and_body(after).await;
    assert_eq!(before_body, after_body, "冲突后工作区必须逐字段不变");
}

/// 8. 吊销 ≠ 冲突: 已吊销 = 403 `workspace_revoked`。
#[tokio::test]
async fn revoked_workspace_is_a_403_not_a_conflict() {
    let mut revoked = ready_row();
    revoked.status = WorkspaceStatus::Revoked;
    revoked.can_select = false;
    revoked.can_create_temporary = false;
    let (services, panel) = stub_services(vec![revoked]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/select"),
            None,
            serde_json::json!({"expected_revision": 7}),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "workspace_revoked");
    assert!(panel.selected.lock().expect("锁").is_empty());
}

/// 9. 客户端不能塞根路径: 请求体带 `root` = 400 (受管根由宿主决定)。
#[tokio::test]
async fn client_cannot_push_a_root_path_into_the_create_body() {
    let (services, panel) = stub_services(Vec::new());
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post(
            "/v1/workspaces",
            None,
            serde_json::json!({
                "display_name": "越权工作区",
                "root": "C:\\Users\\someone\\secret",
            }),
        ))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "invalid_request");
    assert!(
        panel.created.lock().expect("锁").is_empty(),
        "带根路径的请求不得进入端口"
    );
}

/// 10. 不泄露: 读面任何字段都不含绝对路径; 端口交回不安全展示名 = 500 且不回显。
#[tokio::test]
async fn workspace_read_face_never_carries_an_absolute_root_path() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let row = &value["workspaces"][0];
    let root_display = row["root_display"].as_str().expect("root_display");
    assert_eq!(root_display, "release");
    assert!(
        !root_display.contains('/') && !root_display.contains('\\') && !root_display.contains(':'),
        "root_display 只能是叶子名: {root_display}"
    );
    // 整行不得出现盘符/分隔符形态的路径。
    let row_text = serde_json::to_string(row).expect("row json");
    assert!(
        !row_text.contains("\\\\") && !row_text.contains(":/"),
        "读面不得出现绝对路径形态: {row_text}"
    );
    assert!(
        row.get("root").is_none(),
        "读面不得有根路径字段: {row_text}"
    );

    // 端口交回不安全展示名 ⇒ 500 internal 且不回显该值。
    let mut hostile = ready_row();
    hostile.root_display = "C:\\Users\\someone\\secret".to_string();
    let (services, _panel) = stub_services(vec![hostile]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let response = router
        .clone()
        .oneshot(get("/v1/panel/workspaces", None))
        .await
        .expect("response");
    let (status, body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "body={body}");
    assert!(
        !body.contains("secret"),
        "不安全展示名不得回显: body={body}"
    );
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(value["error"]["code"], "internal");
}

/// 11. 可检索审计: 创建/选择/关闭/被拒都落审计行, 详情不含绝对路径。
#[tokio::test]
async fn workspace_actions_are_audited_with_the_contract_action_words() {
    let audit = Arc::new(StubAudit::default());
    let panel = Arc::new(StubWorkspacePanel {
        rows: Mutex::new(vec![ready_row()]),
        ..StubWorkspacePanel::default()
    });
    let services = GatewayServices {
        workspaces: Some(panel.clone() as Arc<dyn WorkspacePanel>),
        audit_commands: Some(audit.clone() as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    let response = router
        .clone()
        .oneshot(post(
            "/v1/workspaces",
            None,
            serde_json::json!({"display_name": "校对发布清单"}),
        ))
        .await
        .expect("response");
    let (status, _body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CREATED, "创建必须是 201");

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/select"),
            None,
            serde_json::json!({"expected_revision": 7}),
        ))
        .await
        .expect("response");
    let (status, _body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::OK);

    let response = router
        .clone()
        .oneshot(post(
            &format!("/v1/workspaces/{WORKSPACE_ID}/close"),
            None,
            serde_json::json!({"expected_revision": 3}),
        ))
        .await
        .expect("response");
    let (status, _body) = status_and_body(response).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let rows = audit.rows.lock().expect("锁").clone();
    let events: Vec<&str> = rows.iter().map(|(event, _)| event.as_str()).collect();
    assert!(
        events
            .iter()
            .all(|event| *event == "gateway.workspace_action"),
        "审计事件名必须唯一: {events:?}"
    );
    let details: Vec<&str> = rows.iter().map(|(_, detail)| detail.as_str()).collect();
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.create")),
        "创建动作必须落审计: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.select")),
        "选择动作必须落审计: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.deny")),
        "被拒动作必须落审计: {details:?}"
    );
    for detail in &details {
        assert!(
            !detail.contains("://") && !detail.contains('\\'),
            "审计详情不得含绝对路径: {detail}"
        );
    }
}

/// 12. 能力清单入册: `workspaces.read` / `workspaces.write`; 未接线时 supported=false。
#[tokio::test]
async fn workspace_capabilities_are_registered_with_wiring_truth() {
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
        .find(|group| group["name"] == "workspaces")
        .expect("workspaces 分组必须入册");
    let caps = group["capabilities"].as_array().expect("caps");
    let read = caps
        .iter()
        .find(|cap| cap["id"] == "workspaces.read")
        .expect("workspaces.read");
    assert_eq!(read["read"], true);
    assert_eq!(read["write"], false);
    assert_eq!(read["supported"], true);
    assert_eq!(read["available"], true);
    assert_eq!(read["operations"], serde_json::json!(["list", "get"]));
    let write = caps
        .iter()
        .find(|cap| cap["id"] == "workspaces.write")
        .expect("workspaces.write");
    assert_eq!(write["read"], false);
    assert_eq!(write["write"], true);
    assert_eq!(write["supported"], true);
    assert_eq!(write["available"], true);
    assert_eq!(
        write["operations"],
        serde_json::json!(["create", "select", "close"])
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
        .find(|group| group["name"] == "workspaces")
        .expect("workspaces 分组必须入册");
    for cap in group["capabilities"].as_array().expect("caps") {
        assert_eq!(cap["supported"], false, "cap={cap}");
        assert_eq!(cap["available"], false, "cap={cap}");
        assert!(cap["reason"].is_string(), "cap={cap}");
    }
}

/// 13. 面板边界: 无删除/改派/删除根路由 (写面只有建/选/关三条)。
#[tokio::test]
async fn workspace_face_exposes_no_delete_or_rebind_routes() {
    let (services, _panel) = stub_services(vec![ready_row()]);
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    for (method, uri) in [
        ("DELETE", format!("/v1/workspaces/{WORKSPACE_ID}")),
        ("PUT", format!("/v1/workspaces/{WORKSPACE_ID}")),
        ("PATCH", format!("/v1/workspaces/{WORKSPACE_ID}")),
        ("GET", format!("/v1/workspaces/{WORKSPACE_ID}/select")),
        ("GET", format!("/v1/workspaces/{WORKSPACE_ID}/close")),
        ("POST", format!("/v1/workspaces/{WORKSPACE_ID}/rebind-root")),
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
