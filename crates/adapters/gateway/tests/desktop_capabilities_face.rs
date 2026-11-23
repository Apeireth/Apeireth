use std::sync::{Arc, Mutex};

use apeireth_gateway::desktop_capabilities::{
    DesktopCapabilityError, DesktopCapabilityPatch, DesktopCapabilityService,
    DesktopCapabilitySettings, DesktopCapabilityView,
};
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
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PATH: &str = "/v1/admin/desktop-capabilities";
const TOKEN: &str = "desktop-capability-test-fixture-nonsecret";

struct ManagedModel;

impl ModelConnectionService for ManagedModel {
    fn read(&self) -> Result<ModelConnectionView, ModelConnectionError> {
        Err(ModelConnectionError::CredentialUnavailable)
    }

    fn save(&self, _: ModelConnectionPatch) -> Result<ModelConnectionView, ModelConnectionError> {
        Err(ModelConnectionError::CredentialUnavailable)
    }
}

struct Capabilities(Mutex<DesktopCapabilityView>);

impl DesktopCapabilityService for Capabilities {
    fn read(&self) -> Result<DesktopCapabilityView, DesktopCapabilityError> {
        Ok(*self.0.lock().unwrap())
    }

    fn save(
        &self,
        patch: DesktopCapabilityPatch,
    ) -> Result<DesktopCapabilityView, DesktopCapabilityError> {
        let mut view = self.0.lock().unwrap();
        if patch.expected_revision != view.revision {
            return Err(DesktopCapabilityError::Conflict);
        }
        view.revision += 1;
        view.configured = patch.configured;
        view.restart_required = view.configured != view.effective;
        Ok(*view)
    }
}

async fn request(router: &Router, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(PATH);
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => request
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn managed_operator_contract_accepts_only_the_three_versioned_switches() {
    let defaults = DesktopCapabilitySettings::default();
    let capabilities = Arc::new(Capabilities(Mutex::new(DesktopCapabilityView {
        schema_version: 1,
        revision: 0,
        configured: defaults,
        effective: defaults,
        restart_required: false,
    })));
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        desktop_capabilities: Some(capabilities.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.to_string());
    state.user_access = managed_operator_access();
    let ordinary = canonical_router_with_state(state.clone());
    assert_eq!(
        request(&ordinary, Some(TOKEN), None).await.0,
        StatusCode::FORBIDDEN
    );
    state.services.model_connection = Some(Arc::new(ManagedModel));
    let managed = canonical_router_with_state(state);
    assert_eq!(
        request(&managed, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&managed, Some("wrong"), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (_, initial) = request(&managed, Some(TOKEN), None).await;
    assert_eq!(initial["revision"], 0);
    assert_eq!(initial["configured"], initial["effective"]);
    assert_eq!(initial["effective"]["skill_search"], false);

    let desired =
        json!({"project_file_tools":false,"preference_learning":false,"skill_search":true});
    for invalid in [
        json!({"configured":desired}),
        json!({"expected_revision":0,"configured":{"skill_search":true}}),
        json!({"expected_revision":0,"configured":{"project_file_tools":false,"preference_learning":false,"skill_search":true,"shell":true}}),
        json!({"expected_revision":0,"configured":desired,"effective":desired}),
    ] {
        assert_eq!(
            request(&managed, Some(TOKEN), Some(invalid)).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(capabilities.read().unwrap().revision, 0);
    let patch = json!({"expected_revision":0,"configured":desired});
    let (status, saved) = request(&managed, Some(TOKEN), Some(patch.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["configured"], desired);
    assert_eq!(saved["effective"], initial["effective"]);
    assert_eq!(saved["restart_required"], true);
    assert_eq!(saved["revision"], 1);
    let (status, stale) = request(&managed, Some(TOKEN), Some(patch)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale["error"]["code"], "capability_revision_conflict");
    assert_eq!(request(&managed, Some(TOKEN), None).await.1, saved);
}
