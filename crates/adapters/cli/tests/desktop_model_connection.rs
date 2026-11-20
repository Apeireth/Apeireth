//! Real role, HTTP, canonical provider and artifact-store integration.
//! Only fixture credentials and a loopback provider are used.

use apeireth_core::kernel::SessionId;
use apeireth_gateway::model_connection::{
    managed_operator_access, CredentialState, ModelConnectionError, ModelConnectionPatch,
    ModelConnectionService, ModelConnectionView,
};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices, GatewayState,
    GatewayTokenGate, UserAccess,
};
use apeireth_governance::{AllowAll, UserRecord, UserTable};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const TOKEN: &str = "managed-fixture-operator-token-0123456789abcdef";

#[derive(Default)]
struct Config(Mutex<Option<(String, String)>>);
impl ModelConnectionService for Config {
    fn read(&self) -> Result<ModelConnectionView, ModelConnectionError> {
        let configured = self.0.lock().unwrap().clone();
        Ok(ModelConnectionView {
            schema_version: 1,
            provider: "openai-compatible",
            base_url: configured
                .as_ref()
                .map(|c| c.0.clone())
                .unwrap_or_else(|| "https://model.example/v1".into()),
            model: configured
                .as_ref()
                .map(|c| c.1.clone())
                .unwrap_or_else(|| "model-a".into()),
            credential_state: if configured.is_some() {
                CredentialState::Available
            } else {
                CredentialState::Missing
            },
            storage: "windows-credential-manager",
            endpoint_bound: configured.is_some(),
            restart_required: configured.is_some(),
        })
    }
    fn save(
        &self,
        patch: ModelConnectionPatch,
    ) -> Result<ModelConnectionView, ModelConnectionError> {
        *self.0.lock().unwrap() = Some((patch.base_url, patch.model));
        self.read()
    }
}

async fn state() -> (tempfile::TempDir, GatewayState) {
    let root = tempfile::tempdir().unwrap();
    let artifacts =
        apeireth_runtime_assembly::sqlite_artifacts::SqliteArtifactService::open(root.path())
            .await
            .unwrap();
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        model_connection: Some(Arc::new(Config::default())),
        artifacts: Some(Arc::new(artifacts)),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    state.user_access = managed_operator_access();
    (root, state)
}

async fn http(
    state: GatewayState,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = if let Some(body) = body {
        request = request.header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    let response = canonical_router_with_state(state)
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if uri == "/v1/admin/model-connection" {
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn explicit_operator_uses_normal_role_gate_and_c2_store() {
    let (_root, state) = state().await;
    let (status, view) = http(
        state.clone(),
        "GET",
        "/v1/admin/model-connection",
        None,
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["credential_state"], "missing");
    assert_eq!(view["provider"], "openai-compatible");
    let (status, caps) = http(
        state.clone(),
        "GET",
        "/v1/artifacts/capabilities",
        None,
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{caps}");
    let (status, receipt) = http(state.clone(),"POST","/v1/artifacts",Some(json!({
        "store_id":caps["store_id"],"operation_id":uuid::Uuid::new_v4().to_string(),"work_id":"work-c3",
        "title":"fixture","media_type":"text/plain","content":"editable fixture","conversation_session_id":null
    })),Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    let (status, ack) = http(state.clone(),"POST","/v1/admin/model-connection",Some(json!({"base_url":"https://model.example/v1","model":"model-b","api_key":"fixture-secret-only"})),Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK, "{ack}");
    assert_eq!(
        ack,
        json!({"schema_version":1,"ok":true,"restart_required":true})
    );
    let (_, view) = http(
        state.clone(),
        "GET",
        "/v1/admin/model-connection",
        None,
        Some(TOKEN),
    )
    .await;
    assert_eq!(view["model"], "model-b");
    assert!(!view.to_string().contains("fixture-secret"));
    let (status, _) = http(
        state,
        "POST",
        "/v1/admin/config",
        Some(json!({"base_url":"https://other.example/v1"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn missing_token_inactive_role_and_expanded_user_scope_stay_closed() {
    let (_root, state) = state().await;
    assert_eq!(
        http(
            state.clone(),
            "GET",
            "/v1/admin/model-connection",
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let mut inactive = state.clone();
    inactive.user_access = UserAccess::inactive();
    assert_eq!(
        http(
            inactive,
            "GET",
            "/v1/admin/model-connection",
            None,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    for mode in 0..4 {
        let mut altered = state.clone();
        let mut table = managed_operator_access().table().unwrap().clone();
        match mode {
            0 => table.users[0].role = "member".into(),
            1 => table.users[0].domain = Some("other-domain".into()),
            2 => table.users.push(UserRecord {
                user_id: "other".into(),
                role: "guest".into(),
                client_names: vec!["other-client".into()],
                domain: None,
            }),
            _ => table.users[0].client_names.push("extra-client".into()),
        }
        altered.user_access = UserAccess::from_table(UserTable::new(table.users));
        let (status, body) = http(
            altered,
            "GET",
            "/v1/artifacts/capabilities",
            None,
            Some(TOKEN),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "mode={mode}: {body}"
        );
    }
    let mut extra_token = state.clone();
    extra_token.client_tokens = apeireth_gateway::ClientTokenTable::from_config(
        apeireth_gateway::ClientTokenConfig::new(vec![apeireth_gateway::ClientTokenSpec {
            name: "extra".into(),
            token: "extra-fixture-token".into(),
            user_id: None,
        }]),
    )
    .unwrap();
    assert_eq!(
        http(
            extra_token,
            "GET",
            "/v1/artifacts/capabilities",
            None,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut unprotected = state;
    unprotected.token_gate = GatewayTokenGate::UnprotectedDefault;
    // Read methods have no role capability requirement. The managed store's
    // own strict operator scope rejects the unprotected/anonymous caller.
    let (status, body) = http(unprotected, "GET", "/v1/artifacts/capabilities", None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "invalid_request");
    assert_eq!(
        body["error"]["details"]["artifact_error"],
        "scope_unavailable"
    );
}

#[tokio::test]
async fn contacts_allow_only_the_managed_operator_shape_with_an_active_user_table() {
    let (_root, state) = state().await;
    let session = SessionId::new();
    let path = format!("/v1/sessions/{session}/contact-session");
    let input = json!({"contact_id": session.to_string(), "context_key": "a".repeat(64),
                        "incarnation": "managed-test"});
    let (status, _) = http(
        state.clone(),
        "POST",
        &path,
        Some(input.clone()),
        Some(TOKEN),
    )
    .await;
    // This fixture deliberately omits the contact registry. Reaching its
    // explicit unsupported response proves the authorized managed scope passed.
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);

    let mut expanded = state;
    let mut users = managed_operator_access().table().unwrap().clone();
    users.users.push(UserRecord {
        user_id: "other".into(),
        role: "guest".into(),
        client_names: vec!["other-client".into()],
        domain: None,
    });
    expanded.user_access = UserAccess::from_table(UserTable::new(users.users));
    let (status, _) = http(expanded, "POST", &path, Some(input), Some(TOKEN)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn strict_config_body_cannot_echo_secret_or_accept_unknown_fields() {
    let (_root, state) = state().await;
    let (status,body)=http(state,"POST","/v1/admin/model-connection",Some(json!({"base_url":"https://model.example/v1","model":"model-a","api_key":"fixture-do-not-echo","unexpected":true})),Some(TOKEN)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.to_string().contains("fixture-do-not-echo"));
}

#[tokio::test]
async fn same_named_vendor_model_is_served_only_by_registered_generic_provider() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0; 4096];
            let count = socket.read(&mut chunk).await.unwrap();
            assert!(count > 0 && request.len() + count <= 16384);
            request.extend_from_slice(&chunk[..count]);
            if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse().ok())
                    })
                    .unwrap();
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let input = String::from_utf8_lossy(&request);
        assert!(input.contains("fixture-provider-key"));
        assert!(input.contains("MiniMax-M3"));
        let body = r#"{"id":"fixture-completion","model":"MiniMax-M3","choices":[{"message":{"role":"assistant","content":"canonical fixture completed"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    });
    let hot = Arc::new(apeireth_credentials::HotCredentialStore::new());
    hot.set_with_endpoints(
        apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY,
        "fixture-provider-key",
        [format!("http://{address}")],
    );
    let plugin =
        apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin::new(
            format!("http://{address}"),
            vec!["MiniMax-M3".into()],
            reqwest::Client::new(),
            2000,
        )
        .unwrap();
    let runtime = Arc::new(
        Runtime::builder()
            .with_governance(Arc::new(AllowAll))
            .with_credentials(hot)
            .with_plugin(Arc::new(plugin))
            .with_default_model("minimax-m3")
            .build()
            .await
            .unwrap(),
    );
    assert_eq!(
        runtime
            .providers()
            .provider_ids()
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["provider.openai-compatible"]
    );
    let (_root, mut state) = state().await;
    state.runtime = runtime;
    let (status, body) = http(
        state,
        "POST",
        "/v1/chat",
        Some(json!({"session":SessionId::new(),"input":"hello","model":"minimax-m3"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["text"], "canonical fixture completed");
    server.await.unwrap();
}
