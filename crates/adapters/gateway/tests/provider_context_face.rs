//! Last provider request readback, with no transcript or estimated context.
#![allow(dead_code)]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_gateway::{build_gateway_state, canonical_router_with_state, GatewayTokenGate};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use apeireth_runtime::{Runtime, Session, TurnRequest};
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

const MODEL: &str = "provider-context-model";
const PROVIDER: &str = "provider.context-fixture";
const TOKEN: &str = "provider-context-gateway-fixture-token";

struct Provider {
    id: CapabilityId,
    calls: AtomicUsize,
}

#[async_trait]
impl ProviderCapability for Provider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls)
                .with_context_window(16_384),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let step = self.calls.fetch_add(1, Ordering::SeqCst);
        let actual_model = if step == 2 {
            "unregistered-served-model"
        } else {
            &request.model
        };
        let mut response =
            NormalizedResponse::text(format!("fixture-{step}"), actual_model, "fixture");
        response.usage = NormalizedUsage::new(900 + step as u32, 25);
        match step {
            0 => {
                response.finish_reason = Some(NormalizedFinishReason::ToolCalls);
                response.tool_calls.push(ToolCall {
                    id: "fixture-calculation".into(),
                    name: common::TOOL.into(),
                    arguments: json!({"left":1,"right":1}),
                });
                Ok(response.with_provider_context_input(Some(17)))
            }
            1 => Ok(response.with_provider_context_input(Some(49))),
            2 => Ok(response),
            _ => Err(ProviderError::BadResponse {
                provider: PROVIDER.into(),
                detail: "fixture refusal".into(),
            }),
        }
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Provider>,
}

impl ProviderPlugin {
    fn new() -> Self {
        let id = CapabilityId::new(PROVIDER).unwrap();
        let manifest = PluginManifest::new(
            PluginId::new("test.provider-context").unwrap(),
            "1.0.0",
            "context fixture",
        )
        .declare_capability(id.clone(), CapabilityKind::Provider, "context fixture")
        .unwrap();
        Self {
            manifest,
            provider: Arc::new(Provider {
                id,
                calls: AtomicUsize::new(0),
            }),
        }
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone()]
    }
}

async fn get(router: &Router, path: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_two_round_turn_only_publishes_the_last_invocation_and_unknown_never_falls_back() {
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let runtime = Arc::new(
        Runtime::builder()
            .with_default_model(MODEL)
            .with_governance(common::allow_all())
            .with_plugin(Arc::new(ProviderPlugin::new()))
            .with_plugin(common::TestPlugin::calculator(tool_calls.clone()))
            .build()
            .await
            .unwrap(),
    );
    let mut state = build_gateway_state(runtime.clone(), None);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    let router = canonical_router_with_state(state);
    let session = SessionId::new();
    let path = format!("/v1/sessions/{session}/provider-context");
    let unknown = json!({"schema":1,"session":session,"provider_request":null});
    assert_eq!(get(&router, &path, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        get(&router, &path, Some(TOKEN)).await.0,
        StatusCode::NOT_FOUND
    );
    runtime
        .sessions()
        .save(&Session::new(session, runtime.sessions().clock().as_ref()))
        .await
        .unwrap();
    assert_eq!(
        get(&router, &path, Some(TOKEN)).await,
        (StatusCode::OK, unknown.clone())
    );
    assert_eq!(
        get(&router, &format!("{path}?model={MODEL}"), Some(TOKEN))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );

    let first = runtime
        .execute(TurnRequest::new(session, "first"))
        .await
        .unwrap();
    assert_eq!(first.rounds, 2);
    assert_eq!(tool_calls.load(Ordering::SeqCst), 1);
    let (status, body) = get(&router, &path, Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"schema":1,"session":session,"provider_request":{
            "request":first.request,"trace_id":first.trace.trace,"provider":PROVIDER,"model":MODEL,
            "input_tokens":49,"context_window":16384,
        }})
    );
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    let restarted: Session =
        serde_json::from_value(serde_json::to_value(&stored).unwrap()).unwrap();
    assert_eq!(
        restarted.last_provider_request(),
        stored.last_provider_request()
    );

    let second = runtime
        .execute(TurnRequest::new(session, "without measured input"))
        .await
        .unwrap();
    let (_, missing) = get(&router, &path, Some(TOKEN)).await;
    assert_eq!(
        missing["provider_request"]["request"],
        json!(second.request)
    );
    assert_eq!(
        missing["provider_request"]["trace_id"],
        json!(second.trace.trace)
    );
    assert!(missing["provider_request"]["input_tokens"].is_null());
    assert!(missing["provider_request"]["context_window"].is_null());
    assert!(runtime
        .execute(TurnRequest::new(session, "failed exchange"))
        .await
        .is_err());
    assert_eq!(
        get(&router, &path, Some(TOKEN)).await,
        (StatusCode::OK, unknown)
    );
}
