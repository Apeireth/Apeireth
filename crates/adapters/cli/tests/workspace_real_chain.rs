//! 工作区合同面的真实链路 (A2 批): 真 `InMemoryWorkspaceRegistry` + 真宿主受管根
//! + 真 CLI 审计落盘 + 真 HTTP 路由。
//!
//! 这里钉四件事 (与面契约测试分工: 面形状在 gateway 侧, 本文件钉**真数据面**):
//! 1. **宿主受管根** —— 宿主把受管根绑成一个工作区, 读面只给叶子名, 响应体里
//!    **没有**绝对根路径;
//! 2. **作用域宿主确认** —— 客户端请求 `approval_write`, 宿主按上限截断为
//!    `write` (请求值 ≠ 生效值);
//! 3. **真乐观并发** —— 陈旧修订号 = 409 `workspace_conflict` 且工作区逐字段不变;
//!    吊销后再选 = 403 `workspace_revoked`;
//! 4. **可检索审计** —— 建/选/关/被拒都落 `gateway.workspace_action` 审计行,
//!    `GET /v1/panel/audit` 查得到, 详情里没有绝对路径。

use std::sync::{Arc, Mutex};

use apeireth_cli::gateway_panels::{bind_managed_workspace, CliPanelData, CliWorkspacePanel};
use apeireth_core::kernel::{HistoryEntry, StreamKind};
use apeireth_core::Episode;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, AuditQuery,
    GatewayServices, GatewayTokenGate, WorkspacePanel,
};
use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_plugin::memory_backend::{BackendKind, CapabilityResult, MemoryBackend};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 内存记忆后端 (只为把真 `CliPanelData` 装起来: 审计落盘是真链路的一部分)。
struct FakeMemory {
    episodes: Mutex<Vec<Episode>>,
}

impl FakeMemory {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            episodes: Mutex::new(Vec::new()),
        })
    }
}

impl MemoryBackend for FakeMemory {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }

    fn put_episode(&self, ep: &Episode) -> CapabilityResult<()> {
        self.episodes.lock().expect("锁").push(ep.clone());
        Ok(())
    }

    fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>> {
        Ok(self
            .episodes
            .lock()
            .expect("锁")
            .iter()
            .find(|episode| episode.id == id)
            .cloned())
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> CapabilityResult<Vec<Episode>> {
        let mut episodes: Vec<Episode> = self
            .episodes
            .lock()
            .expect("锁")
            .iter()
            .filter(|episode| episode.session_id == session_id)
            .cloned()
            .collect();
        episodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        episodes.truncate(n);
        Ok(episodes)
    }

    fn append_stream(&self, _kind: StreamKind, _entry: HistoryEntry) -> CapabilityResult<()> {
        Ok(())
    }

    fn list_stream(
        &self,
        _kind: StreamKind,
        _session_id: &str,
        _n: usize,
    ) -> CapabilityResult<Vec<HistoryEntry>> {
        Ok(Vec::new())
    }
}

fn test_policy() -> Arc<Mutex<PermissionPolicy>> {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    Arc::new(Mutex::new(policy))
}

/// 装一条真链路: 真工作区注册表 (受管根) + 真 CLI 审计 + 真网关路由。
async fn router_over(root: std::path::PathBuf, data_dir: std::path::PathBuf) -> axum::Router {
    let panel = Arc::new(CliPanelData::new(
        Arc::new(InMemorySessionStore::new()),
        FakeMemory::new(),
        test_policy(),
        true,
        data_dir,
    ));
    let workspaces = Arc::new(CliWorkspacePanel::new(root));
    bind_managed_workspace(&workspaces, "默认工作区").expect("绑定宿主受管根");
    let services = GatewayServices {
        workspaces: Some(workspaces as Arc<dyn WorkspacePanel>),
        audit: Some(panel.clone() as Arc<dyn AuditQuery>),
        audit_commands: Some(panel as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    canonical_router_with_state(state)
}

async fn call(
    router: axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(match body {
            Some(body) => Body::from(body.to_string()),
            None => Body::empty(),
        })
        .expect("request");
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    let response = router.oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// 宿主受管根: 读面只给叶子名, 响应体里没有绝对根路径。
#[tokio::test]
async fn host_managed_root_is_listed_without_its_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir).await;

    let (status, body) = call(router.clone(), "GET", "/v1/panel/workspaces", None).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let rows = body["workspaces"].as_array().expect("rows");
    assert_eq!(rows.len(), 1, "宿主受管根应绑定成一个工作区: {body}");
    assert_eq!(rows[0]["root_display"], "managed-root");
    assert_eq!(rows[0]["root_kind"], "managed");
    assert_eq!(rows[0]["status"], "ready");
    assert_eq!(rows[0]["source"], "cli");

    let text = body.to_string();
    assert!(
        !text.contains('\\') && !text.contains(":/"),
        "读面不得出现绝对路径: {text}"
    );
    assert!(
        !text.contains(&managed_root.display().to_string()),
        "读面不得回显宿主根路径: {text}"
    );
}

/// 作用域宿主确认: 客户端请求 `approval_write`, 宿主按上限截断为 `write`。
#[tokio::test]
async fn create_uses_the_host_root_and_the_host_confirmed_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir).await;

    let (status, body) = call(
        router,
        "POST",
        "/v1/workspaces",
        Some(serde_json::json!({
            "display_name": "校对发布清单",
            "root_kind": "managed",
            "permission_scope": "approval_write",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let workspace = &body["workspace"];
    assert_eq!(workspace["display_name"], "校对发布清单");
    assert_eq!(workspace["root_display"], "managed-root");
    assert_eq!(
        workspace["permission_scope"], "write",
        "宿主上限是 write: 请求值 approval_write 不得直接生效"
    );
    assert_eq!(workspace["status"], "ready");
    assert_eq!(workspace["revision"], 0);

    let text = body.to_string();
    assert!(
        !text.contains(&managed_root.display().to_string()),
        "响应体不得回显宿主根路径: {text}"
    );
}

/// 客户端不能塞根路径, 也不能要求宿主不装配的根类型。
#[tokio::test]
async fn create_refuses_client_paths_and_unassembled_root_kinds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir).await;

    let (status, body) = call(
        router.clone(),
        "POST",
        "/v1/workspaces",
        Some(serde_json::json!({
            "display_name": "越权工作区",
            "root": "C:\\Users\\someone\\secret",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert_eq!(body["error"]["code"], "invalid_request");

    let (status, body) = call(
        router,
        "POST",
        "/v1/workspaces",
        Some(serde_json::json!({
            "display_name": "临时工作区",
            "root_kind": "temporary",
            "expires_at": 1_800_000_000_000i64,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert_eq!(body["error"]["code"], "invalid_request");
}

/// 真乐观并发: 陈旧修订号 = 409 且工作区逐字段不变; 吊销后再选 = 403。
#[tokio::test]
async fn stale_revision_conflicts_and_revoked_workspaces_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir).await;

    let (_, listed) = call(router.clone(), "GET", "/v1/panel/workspaces", None).await;
    let workspace_id = listed["workspaces"][0]["workspace_id"]
        .as_str()
        .expect("id")
        .to_string();
    let before = listed["workspaces"][0].clone();

    // 缺修订号 = 400。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body={body}");
    assert_eq!(body["error"]["code"], "invalid_request");

    // 陈旧修订号 = 409, 且状态不动。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 99})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body={body}");
    assert_eq!(body["error"]["code"], "workspace_conflict");

    let (_, after_conflict) = call(router.clone(), "GET", "/v1/panel/workspaces", None).await;
    assert_eq!(
        after_conflict["workspaces"][0], before,
        "冲突后工作区必须逐字段不变"
    );

    // 正确修订号 = 真转移。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["workspace"]["status"], "ready");

    // 关闭 = 吊销 (修订号 +1)。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/close"),
        Some(serde_json::json!({"expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["workspace"]["status"], "revoked");
    assert_eq!(body["workspace"]["revision"], 1);
    assert_eq!(body["workspace"]["can_select"], false);

    // 吊销后再选 = 403 (不是冲突)。
    let (status, body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["error"]["code"], "workspace_revoked");
}

/// 可检索审计: 建/选/关/被拒都落审计行, 详情里没有绝对路径。
#[tokio::test]
async fn workspace_actions_are_retrievable_in_the_audit_face() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let router = router_over(managed_root.clone(), data_dir.clone()).await;

    let (status, created) = call(
        router.clone(),
        "POST",
        "/v1/workspaces",
        Some(serde_json::json!({"display_name": "校对发布清单"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={created}");
    let workspace_id = created["workspace"]["workspace_id"]
        .as_str()
        .expect("id")
        .to_string();

    let (status, _body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _body) = call(
        router.clone(),
        "POST",
        &format!("/v1/workspaces/{workspace_id}/close"),
        Some(serde_json::json!({"expected_revision": 99})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, audit) = call(router.clone(), "GET", "/v1/panel/audit", None).await;
    assert_eq!(status, StatusCode::OK, "body={audit}");
    let rows = audit["events"].as_array().expect("audit rows");
    let workspace_rows: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|row| row["event"] == "gateway.workspace_action")
        .collect();
    assert!(
        !workspace_rows.is_empty(),
        "工作区动作必须落审计行: {audit}"
    );
    let details: Vec<String> = workspace_rows
        .iter()
        .map(|row| row["detail"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.create")),
        "创建动作必须可检索: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.select")),
        "选择动作必须可检索: {details:?}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains("action=workspace.deny")),
        "被拒动作必须可检索: {details:?}"
    );
    for detail in &details {
        assert!(
            !detail.contains(&managed_root.display().to_string()),
            "审计详情不得含绝对根路径: {detail}"
        );
    }

    // 审计同时落了 JSONL (真落盘)。
    let audit_file = data_dir.join("daemon-audit.jsonl");
    let text = std::fs::read_to_string(&audit_file).expect("audit jsonl");
    assert!(
        text.contains("gateway.workspace_action"),
        "审计必须落盘: {text}"
    );
}
