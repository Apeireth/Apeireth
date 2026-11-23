use super::*;
use apeireth_gateway::model_connection::{
    managed_operator_access, ModelConnectionError, ModelConnectionPatch, ModelConnectionService,
    ModelConnectionView,
};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayTokenGate,
};
use apeireth_runtime::canonical::workspace::{
    FileWorkspaceRegistry, DESKTOP_MANAGED_WORKSPACE_SOURCE,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tower::ServiceExt;

const FIXTURE_TOKEN: &str = "managed-workspace-fixture-operator-token";
const ROOTS_DIR: &str = "managed-workspace-roots";

struct FixtureModelConnection;

impl ModelConnectionService for FixtureModelConnection {
    fn read(&self) -> Result<ModelConnectionView, ModelConnectionError> {
        Err(ModelConnectionError::CredentialUnavailable)
    }

    fn save(&self, _: ModelConnectionPatch) -> Result<ModelConnectionView, ModelConnectionError> {
        Err(ModelConnectionError::CredentialUnavailable)
    }
}

async fn router(root: &Path, data: &Path, managed: bool) -> axum::Router {
    let panel = CliWorkspacePanel::new_persistent(root, data)
        .unwrap()
        .enable_isolated_creation();
    bind_managed_workspace(&panel, "默认工作区").unwrap();
    let services = GatewayServices {
        workspaces: Some(Arc::new(panel)),
        model_connection: managed.then(|| Arc::new(FixtureModelConnection) as Arc<_>),
        ..GatewayServices::default()
    };
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(FIXTURE_TOKEN.to_string());
    if managed {
        state.user_access = managed_operator_access();
    }
    canonical_router_with_state(state)
}

async fn http(router: &axum::Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {FIXTURE_TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn isolated_workspace_creation_persists_distinct_authoritative_roots() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("shared");
    fs::create_dir(&root).unwrap();
    let data = temp.path().join("data");
    let app = router(&root, &data, true).await;
    let (legacy, legacy_body) = http(
        &app,
        "POST",
        "/v1/workspaces",
        json!({"display_name": "Legacy", "permission_scope": "approval_write"}),
    )
    .await;
    assert_eq!(legacy, StatusCode::CREATED);
    assert_eq!(legacy_body["workspace"]["source"], "cli");
    assert_eq!(legacy_body["workspace"]["root_display"], "shared");
    assert_eq!(legacy_body["workspace"]["permission_scope"], "write");

    let opt_in = json!({
        "display_name": "../同名",
        "root_provisioning": "isolated_v1",
        "permission_scope": "approval_write"
    });
    let mut ids = Vec::new();
    for _ in 0..2 {
        let (status, body) = http(&app, "POST", "/v1/workspaces", opt_in.clone()).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let row = &body["workspace"];
        let id = row["workspace_id"].as_str().unwrap();
        assert_eq!(row["root_display"], id);
        assert_eq!(row["source"], DESKTOP_MANAGED_WORKSPACE_SOURCE);
        assert_eq!(row["display_name"], "../同名");
        assert_eq!(row["permission_scope"], "approval_write");
        assert!(!body.to_string().contains(&data.display().to_string()));
        ids.push(id.to_string());
    }
    assert_ne!(ids[0], ids[1]);
    let canonical_parent = data.canonicalize().unwrap().join(ROOTS_DIR);
    fs::write(
        canonical_parent.join(&ids[0]).join("only-in-first.txt"),
        "first",
    )
    .unwrap();
    assert!(canonical_parent.join(&ids[1]).is_dir());
    assert!(!canonical_parent
        .join(&ids[1])
        .join("only-in-first.txt")
        .exists());

    // Open an independent registry so the evidence comes from persisted records.
    let reopened = FileWorkspaceRegistry::open(&data).unwrap();
    for id in &ids {
        let mut executed = false;
        reopened
            .with_desktop_workspace_root(&id.parse().unwrap(), 0, &mut |path, scope| {
                executed = true;
                assert_eq!(path, canonical_parent.join(id));
                assert_eq!(scope, WorkspacePermissionScope::ApprovalWrite);
            })
            .unwrap();
        assert!(executed);
        let (status, fetched) =
            http(&app, "GET", &format!("/v1/workspaces/{id}"), Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            fetched["workspace"]["source"],
            DESKTOP_MANAGED_WORKSPACE_SOURCE
        );
    }
    let (status, listed) = http(&app, "GET", "/v1/panel/workspaces", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        listed["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| { row["source"] == DESKTOP_MANAGED_WORKSPACE_SOURCE })
            .count(),
        2
    );

    let second_id = ids[1].parse().unwrap();
    reopened
        .change_permission_scope(&second_id, 0, WorkspacePermissionScope::Read, 122)
        .unwrap();
    let mut unexpected_operation = |_: &Path, _| panic!("invalid workspace reached operation");
    assert_eq!(
        reopened.with_desktop_workspace_root(&second_id, 0, &mut unexpected_operation),
        Err(WorkspaceError::RevisionConflict {
            expected: 0,
            actual: 1
        })
    );
    reopened
        .rebind_root(&second_id, 1, &canonical_parent.join(&ids[0]), 123)
        .unwrap();
    assert!(matches!(
        reopened.with_desktop_workspace_root(&second_id, 2, &mut unexpected_operation),
        Err(WorkspaceError::InvalidRequest { .. })
    ));

    // A second registry's close wins over the first registry's cached row.
    let id = ids[0].parse().unwrap();
    reopened.close(&id, 0, 123).unwrap();
    let first_registry = crate::binding_registries::workspaces(&data).unwrap();
    assert_eq!(
        first_registry.with_desktop_workspace_root(&id, 0, &mut |_, _| {
            panic!("revoked workspace reached filesystem operation")
        }),
        Err(WorkspaceError::Revoked)
    );
    assert!(canonical_parent
        .join(&ids[0])
        .join("only-in-first.txt")
        .is_file());
}

#[tokio::test]
async fn isolated_creation_fails_closed_on_legacy_hosts_invalid_requests_and_bad_storage() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("shared");
    fs::create_dir(&root).unwrap();
    let data = temp.path().join("data");
    let ordinary = router(&root, &data, false).await;
    let valid = json!({"display_name": "test", "root_provisioning": "isolated_v1"});
    let (status, body) = http(&ordinary, "POST", "/v1/workspaces", valid.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "permission_denied");
    assert!(!data.join(ROOTS_DIR).exists());

    let managed = router(&root, &data, true).await;
    for (field, value) in [
        ("root", json!("some-folder")),
        ("display_name", json!("")),
        ("root_provisioning", json!("unknown_v2")),
        ("permission_scope", json!("write")),
        ("root_kind", json!("preset")),
        ("expires_at", json!(123)),
    ] {
        let mut body = valid.clone();
        body[field] = value;
        let (status, response) = http(&managed, "POST", "/v1/workspaces", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}: {response}");
        assert_eq!(response["error"]["code"], "invalid_request");
    }
    assert!(!data.join(ROOTS_DIR).exists());

    fs::write(data.join(ROOTS_DIR), "folder collision").unwrap();
    let (status, body) = http(&managed, "POST", "/v1/workspaces", valid).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"]["code"], "internal");
    assert_eq!(
        fs::read_to_string(data.join(ROOTS_DIR)).unwrap(),
        "folder collision"
    );
    assert_eq!(FileWorkspaceRegistry::open(&data).unwrap().list().len(), 1);

    fs::remove_file(data.join(ROOTS_DIR)).unwrap();
    let (status, body) = http(
        &managed,
        "POST",
        "/v1/workspaces",
        json!({"display_name": "Read only", "root_provisioning": "isolated_v1"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["workspace"]["permission_scope"], "read");
    assert_eq!(body["workspace"]["can_write"], false);
}
