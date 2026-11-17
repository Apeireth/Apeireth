//! 工作区持久化的真链路 (A3 批): 真 `FileWorkspaceRegistry` (落盘) + 真 HTTP 路由。
//!
//! 这里钉四件事:
//! 1. **重启后仍列出** —— 换一个进程内的新注册表打开同一个数据根,
//!    `GET /v1/panel/workspaces` 仍列出同一批工作区, 且逐字段一致;
//! 2. **revision 跨重启单调** —— 吊销把 revision 推到 1, 重启后仍是 1:
//!    陈旧 revision (0) 的写面被拒 (409 `workspace_conflict`), 正确 revision (1)
//!    命中吊销 (403 `workspace_revoked`);
//! 3. **读面不含绝对根, 落盘含绝对根** —— 响应体里没有根路径, 落盘记录里有;
//! 4. **重启不膨胀** —— 宿主绑定的受管根不会每次重启都新铸一个身份。
//!
//! 数据根是**临时目录**, 不碰真实的 `~/.apeireth`。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_cli::gateway_panels::{bind_managed_workspace, CliWorkspacePanel};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices,
    GatewayTokenGate, WorkspacePanel,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 装一条真链路: 落盘工作区注册表 (同一个数据根) + 真网关路由。
///
/// 每次调用都**新开**一个注册表 —— 这就是「重启」: 进程内的旧状态一个都不留,
/// 能读回来的只有落盘的事实。
async fn router_over(root: PathBuf, data_dir: PathBuf) -> axum::Router {
    let workspaces =
        Arc::new(CliWorkspacePanel::new_persistent(root, data_dir).expect("打开落盘工作区注册表"));
    bind_managed_workspace(&workspaces, "默认工作区").expect("绑定宿主受管根");
    let services = GatewayServices {
        workspaces: Some(workspaces as Arc<dyn WorkspacePanel>),
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

/// 一个临时数据根 + 一个临时受管根 (互不重叠, 都不碰真实用户目录)。
fn roots() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_root = dir.path().join("managed-root");
    std::fs::create_dir_all(&managed_root).expect("managed root");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    (dir, managed_root, data_dir)
}

/// 重启后读面仍列出同一批工作区, 且逐字段一致。
#[tokio::test]
async fn the_panel_still_lists_the_same_workspaces_after_a_restart() {
    let (_dir, managed_root, data_dir) = roots();

    let (status, first) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "GET",
        "/v1/panel/workspaces",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={first}");
    let first_list = first["workspaces"].as_array().expect("rows").clone();
    assert_eq!(first_list.len(), 1, "宿主受管根应绑定成一个工作区: {first}");
    assert_eq!(first_list[0]["root_display"], "managed-root");
    assert_eq!(first_list[0]["status"], "ready");

    // 重启: 新注册表, 同一个数据根。
    let (status, second) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "GET",
        "/v1/panel/workspaces",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={second}");
    let second_list = second["workspaces"].as_array().expect("rows").clone();
    assert_eq!(
        first_list, second_list,
        "重启后列表必须逐字段一致, 且不得凭空多出工作区"
    );
    assert_eq!(
        second_list.len(),
        1,
        "宿主绑定的受管根不得每次重启都新铸一个身份"
    );
}

/// 重启后 revision 单调: 落盘里的 revision 继续用, 不重置; 吊销状态也在。
#[tokio::test]
async fn the_revision_is_monotonic_across_a_restart() {
    let (_dir, managed_root, data_dir) = roots();

    // 第一个会话: 建一个工作区 (revision 0)。
    let (status, created) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "POST",
        "/v1/workspaces",
        Some(serde_json::json!({
            "display_name": "会话一",
            "root_kind": "managed",
            "permission_scope": "write",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={created}");
    let workspace_id = created["workspace"]["workspace_id"]
        .as_str()
        .expect("id")
        .to_string();
    assert_eq!(created["workspace"]["revision"], 0);

    // 第二个会话 (新注册表, 同一数据根 = 重启): 身份与修订号都得在。
    let (status, after) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "GET",
        &format!("/v1/workspaces/{workspace_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={after}");
    assert_eq!(
        after["workspace"]["revision"], 0,
        "重启后 revision 必须仍是建档时的 0, 不得被重算"
    );
    assert_eq!(after["workspace"]["status"], "ready");

    // 重启后比对 revision 的写面仍生效: 陈旧修订号 = 409。
    let (status, conflict) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 99})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body={conflict}");
    assert_eq!(conflict["error"]["code"], "workspace_conflict");

    // 正确修订号 (落盘里那个) = 真转移 —— 若 revision 被重置过, 这里会 409。
    let (status, selected) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={selected}");
    assert_eq!(selected["workspace"]["revision"], 0);

    // 第三个会话: 吊销 (revision +1), 再重启, 吊销与新修订号都还在。
    let (status, closed) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "POST",
        &format!("/v1/workspaces/{workspace_id}/close"),
        Some(serde_json::json!({"expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={closed}");
    assert_eq!(closed["workspace"]["revision"], 1, "吊销恰好 +1");
    assert_eq!(closed["workspace"]["status"], "revoked");

    let (status, after_revoke) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "GET",
        &format!("/v1/workspaces/{workspace_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={after_revoke}");
    assert_eq!(
        after_revoke["workspace"]["revision"], 1,
        "重启后 revision 必须仍是 1, 不得重置为 0"
    );
    assert_eq!(after_revoke["workspace"]["status"], "revoked");

    // 吊销不因重启而失效: 写面 403 (合同面: 吊销检查在 revision 检查之前)。
    let (status, revoked) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "POST",
        &format!("/v1/workspaces/{workspace_id}/select"),
        Some(serde_json::json!({"expected_revision": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={revoked}");
    assert_eq!(revoked["error"]["code"], "workspace_revoked");
}

/// 读面不含绝对根, 落盘记录含绝对根。
#[tokio::test]
async fn the_read_face_hides_the_root_while_the_record_keeps_it() {
    let (_dir, managed_root, data_dir) = roots();

    let (status, body) = call(
        router_over(managed_root.clone(), data_dir.clone()).await,
        "GET",
        "/v1/panel/workspaces",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    let text = body.to_string();
    assert!(
        !text.contains('\\') && !text.contains(":/"),
        "读面不得出现绝对路径: {text}"
    );
    assert!(
        !text.contains(&managed_root.display().to_string()),
        "读面不得回显宿主根路径: {text}"
    );

    // 落盘侧: 受管根在记录里 (否则重启后无从解析工作目录)。
    // 注意: JSON 里反斜杠是转义的 ⇒ 必须解析字段再比对, 不能拿裸串去 contains。
    let records_dir = data_dir.join("workspaces");
    let expected_root = managed_root.to_string_lossy().to_string();
    let mut found = false;
    for entry in std::fs::read_dir(&records_dir).expect("records dir") {
        let entry = entry.expect("entry");
        let record = entry.path().join("workspace.json");
        let text = std::fs::read_to_string(&record).expect("record");
        let doc: serde_json::Value = serde_json::from_str(&text).expect("record json");
        let recorded = doc["body"]["managed_root"].as_str().expect("managed_root");
        if recorded == expected_root {
            found = true;
        }
    }
    assert!(
        found,
        "落盘记录必须带着受管根 (数据根: {})",
        records_dir.display()
    );
}
