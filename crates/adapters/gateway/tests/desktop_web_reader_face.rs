use std::sync::Arc;

use apeireth_gateway::model_connection::{
    managed_operator_access, ModelConnectionError, ModelConnectionPatch, ModelConnectionService,
    ModelConnectionView,
};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices,
    GatewayTokenGate,
};
use apeireth_runtime::Runtime;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const PATH: &str = "/v1/admin/desktop-web-reader";
const TOKEN: &str = "desktop-web-reader-test-fixture-nonsecret";

struct ManagedModel;

impl ModelConnectionService for ManagedModel {
    fn read(&self) -> Result<ModelConnectionView, ModelConnectionError> {
        panic!("reader may not access model connection settings")
    }

    fn save(&self, _: ModelConnectionPatch) -> Result<ModelConnectionView, ModelConnectionError> {
        panic!("reader may not alter model connection settings")
    }
}

async fn router(managed: bool) -> Router {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut services = GatewayServices::default();
    if managed {
        services.model_connection = Some(Arc::new(ManagedModel));
    }
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    state.user_access = managed_operator_access();
    canonical_router_with_state(state)
}

async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: &str,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let request = request.body(Body::from(body.to_owned())).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 32 * 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-secret"));
    (status, value)
}

#[tokio::test]
async fn web_reader_is_post_only_and_requires_authenticated_managed_operator() {
    let managed = router(true).await;
    let outside_scope = router(false).await;
    let request = json!({"url":"https://127.0.0.1/?secret=fixture-secret"}).to_string();
    let (status, error) = send(&outside_scope, "POST", PATH, Some(TOKEN), &request).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"]["code"], "single_operator_required");
    for token in [None, Some("bad-token")] {
        assert_eq!(
            send(&managed, "POST", PATH, token, &request).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        send(&managed, "GET", PATH, Some(TOKEN), "").await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    let response = send(&managed, "POST", PATH, Some(TOKEN), &request).await;
    assert_eq!(response.0, StatusCode::FORBIDDEN);
    assert_eq!(response.1["error"]["code"], "web_destination_denied");
}

#[tokio::test]
async fn web_reader_rejects_unknown_fields_queries_and_local_or_non_https_sources() {
    let managed = router(true).await;
    let token = Some(TOKEN);
    let url = "https://127.0.0.1/?secret=fixture-secret";
    for body in [
        json!({"url":url,"headers":{"Cookie":"fixture-secret"}}),
        json!({"url":url,"method":"POST"}),
        json!({"url":false}),
        json!({"url":null}),
    ] {
        let (status, response) = send(&managed, "POST", PATH, token, &body.to_string()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert_eq!(response["error"]["code"], "invalid_request");
    }
    let query_uri = format!("{PATH}?ignored=true");
    assert_eq!(
        send(
            &managed,
            "POST",
            &query_uri,
            token,
            &json!({"url":url}).to_string()
        )
        .await
        .1["error"]["code"],
        "invalid_request"
    );
    let over_limit = "x".repeat(4097);
    for url in [
        "file:///fixture-secret",
        "http://example.org/fixture-secret",
        "https://reader:fixture-secret@example.org/",
        &over_limit,
    ] {
        let body = json!({"url":url}).to_string();
        let (status, response) = send(&managed, "POST", PATH, token, &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert_eq!(response["error"]["code"], "invalid_url");
    }
    for url in [
        "https://[::1]/",
        "https://10.80.50.10/",
        "https://localhost/",
        "https://example.org:8443/",
    ] {
        let body = json!({"url":url}).to_string();
        let (status, response) = send(&managed, "POST", PATH, token, &body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{response}");
        assert_eq!(response["error"]["code"], "web_destination_denied");
    }
    let too_large = json!({"url":"x".repeat(32 * 1024)}).to_string();
    let (status, response) = send(&managed, "POST", PATH, token, &too_large).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{response}");
    assert_eq!(response["error"]["code"], "reader_request_too_large");
}
