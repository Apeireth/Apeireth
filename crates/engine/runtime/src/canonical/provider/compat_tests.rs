//! Retained Host entry points must preserve projection, egress and accounting.
use super::*;
use apeireth_core::kernel::{system_clock, ModelId, RequestId, SessionId};
use apeireth_protocol::canonical::{ModelDescriptor, ModelFeature, NormalizedMessage};
use async_trait::async_trait;
use std::sync::Mutex;

struct RecordingProvider {
    id: CapabilityId,
    tools: bool,
    fail: bool,
    requests: Mutex<Vec<NormalizedRequest>>,
}
impl RecordingProvider {
    fn new(id: &str, tools: bool, fail: bool) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            tools,
            fail,
            requests: Mutex::new(vec![]),
        })
    }
}
#[async_trait]
impl ProviderCapability for RecordingProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        let model = ModelDescriptor::new(ModelId::new("compat-model").unwrap(), self.id.clone());
        vec![if self.tools {
            model.with_feature(ModelFeature::ToolCalls)
        } else {
            model
        }]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        if self.fail {
            return Err(ProviderError::RateLimited {
                provider: self.id.to_string(),
                retry_after_ms: 1,
            });
        }
        Ok(NormalizedResponse::text(
            "compat-response",
            &request.model,
            "answer",
        ))
    }
}
fn request() -> NormalizedRequest {
    NormalizedRequest::new("compat-model", vec![NormalizedMessage::user("hello")])
}

#[tokio::test]
async fn host_tool_completion_filters_each_fallback_without_mutating_the_request() {
    let plain = RecordingProvider::new("provider.plain", false, true);
    let capable = RecordingProvider::new("provider.tools", true, false);
    let router = ProviderRouter::new(vec![plain.clone(), capable.clone()], system_clock());
    let tools = vec![NormalizedTool::new("lookup")];
    let mut request = request();
    request.tools = vec![NormalizedTool::new("stale")];
    let response = router.complete_with_tools(&request, &tools).await.unwrap();
    assert_eq!(response.served_by, capable.id);
    assert_eq!(response.failed_attempts.len(), 1);
    assert!(plain.requests.lock().unwrap()[0].tools.is_empty());
    assert_eq!(capable.requests.lock().unwrap()[0].tools[0].name, "lookup");
    assert_eq!(request.tools[0].name, "stale");
}

#[tokio::test]
async fn host_streaming_fallback_preserves_full_response_and_filters_tools() {
    let provider = RecordingProvider::new("provider.plain", false, false);
    let router = ProviderRouter::new(vec![provider.clone()], system_clock());
    let deltas = Arc::new(Mutex::new(vec![]));
    let seen = deltas.clone();
    let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |s| seen.lock().unwrap().push(s));
    let response = router
        .complete_with_tools_streaming(&request(), &[NormalizedTool::new("lookup")], &sink)
        .await
        .unwrap();
    assert_eq!(response.response.content, "answer");
    assert_eq!(*deltas.lock().unwrap(), vec!["answer"]);
    assert!(provider.requests.lock().unwrap()[0].tools.is_empty());
}

#[tokio::test]
async fn host_session_completion_records_identity_and_egress_denial_never_sends() {
    use super::super::egress_gate::{EgressGate, EgressVerdict};
    struct Deny;
    impl EgressGate for Deny {
        fn vet(&self, _: &NormalizedRequest) -> EgressVerdict {
            EgressVerdict::Deny {
                reason: "host policy".into(),
            }
        }
    }
    let provider = RecordingProvider::new("provider.session", true, false);
    let router = ProviderRouter::new(vec![provider.clone()], system_clock());
    let session = SessionId::new();
    let request_id = RequestId::new();
    router
        .complete_for_session(session, request_id, &request(), None, None)
        .await
        .unwrap();
    let facts = router.accounting.recent_requests(session);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].request, request_id);
    assert_eq!(facts[0].provider, provider.id);
    assert!(!facts[0].failed);
    assert!(router
        .accounting
        .recent_requests(SessionId::new())
        .is_empty());

    let router = router.with_egress_gate(Arc::new(Deny));
    let error = router
        .complete_for_session(session, RequestId::new(), &request(), None, None)
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::Denied { .. }));
    assert_eq!(provider.requests.lock().unwrap().len(), 1);
    assert_eq!(router.accounting.recent_requests(session).len(), 1);
}
