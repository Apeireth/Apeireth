//! Round-limit and pending-close frames at the real HTTP entry.
//!
//! The wire contract pinned here: a turn that genuinely diverges fails with a
//! `turn_not_converged` frame carrying the real budget numbers (limit, rounds
//! consumed, and the approval-frozen tool name when one existed), while a
//! pending-approval close — including one that lands in the round where the
//! budget is exhausted — is reported as the pending state (202) and never as
//! `turn_not_converged`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_gateway::canonical_router;
use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{parse_turn_round_limit, Runtime};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const MODEL: &str = "fake-model-1";

enum Step {
    Say(&'static str),
    ToolCalls(Vec<ToolCall>),
}

struct ScriptedProvider {
    id: CapabilityId,
    steps: Vec<Step>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let step = self
            .steps
            .get(index)
            .unwrap_or_else(|| panic!("provider script exhausted at call {index}"));
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        Ok(match step {
            Step::Say(text) => NormalizedResponse {
                content: (*text).to_string(),
                ..base
            },
            Step::ToolCalls(calls) => NormalizedResponse {
                content: String::new(),
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                tool_calls: calls.clone(),
                ..base
            },
        })
    }
}

struct CountingTool {
    id: CapabilityId,
    name: &'static str,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for CountingTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.into(),
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({ "ran": true }))
    }
}

struct TestPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<ScriptedProvider>>,
    invocations: Option<Arc<AtomicUsize>>,
}

impl TestPlugin {
    fn provider(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.scripted_provider").unwrap(),
                "1.0.0",
                "scripted provider",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "scripted provider",
            )
            .unwrap(),
            provider: Some(provider),
            invocations: None,
        })
    }

    fn tools(invocations: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.tools").unwrap(),
                "1.0.0",
                "counting tools",
            )
            .declare_capability(
                CapabilityId::new("tool.demo").unwrap(),
                CapabilityKind::Tool,
                "counts invocations",
            )
            .unwrap()
            .declare_capability(
                CapabilityId::new("tool.note").unwrap(),
                CapabilityKind::Tool,
                "counts invocations",
            )
            .unwrap(),
            provider: None,
            invocations: Some(invocations),
        })
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.invocations
            .as_ref()
            .map(|invocations| {
                vec![
                    Arc::new(CountingTool {
                        id: CapabilityId::new("tool.demo").unwrap(),
                        name: "demo",
                        invocations: Arc::clone(invocations),
                    }) as Arc<dyn ToolCapability>,
                    Arc::new(CountingTool {
                        id: CapabilityId::new("tool.note").unwrap(),
                        name: "note",
                        invocations: Arc::clone(invocations),
                    }) as Arc<dyn ToolCapability>,
                ]
            })
            .unwrap_or_default()
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.provider
            .as_ref()
            .map(|provider| vec![Arc::clone(provider) as Arc<dyn ProviderCapability>])
            .unwrap_or_default()
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn gated_tool_call(id: &str, command: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "demo".into(),
        arguments: serde_json::json!({ "command": command }),
    }
}

fn note_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "note".into(),
        arguments: serde_json::json!({ "text": "spin" }),
    }
}

async fn build_runtime(
    provider: Arc<ScriptedProvider>,
    invocations: Arc<AtomicUsize>,
    max_rounds: u32,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.demo".into()));
    policy.grant(Permission::ExecuteTool("tool.note".into()));
    policy.require_approval_for("tool.demo");
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations))
        .with_plugin(TestPlugin::provider(provider))
        .with_default_model(MODEL)
        .with_max_rounds(max_rounds)
        .build()
        .await
        .unwrap()
}

fn chat_request(session: SessionId, input: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/chat")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "session": session, "input": input })).unwrap(),
        ))
        .unwrap()
}

fn resolve_request(session: SessionId, approval: &str, decision: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/approvals/resolve")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "session": session,
                "approval": approval,
                "decision": decision,
            }))
            .unwrap(),
        ))
        .unwrap()
}

async fn send(router: axum::Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

/// ⑤⑥: 调小轮数预算后, 真实上限帧带 limit / 已耗轮数 / 待批工具名。
#[tokio::test]
async fn a_real_round_limit_frame_carries_the_budget_numbers() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls(vec![gated_tool_call("call-1", "echo hi")]),
        Step::ToolCalls(vec![note_call("call-2")]),
        Step::ToolCalls(vec![note_call("call-3")]),
    ]);
    // 预算旋钮经解析器生效: 调小轮数, 触发真实上限帧。
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        parse_turn_round_limit(Some("2")),
    )
    .await;
    let router = canonical_router(Arc::new(runtime));
    let session = SessionId::new();

    let (status, pending) = send(router.clone(), chat_request(session, "run it")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{pending}");
    assert_eq!(pending["tool_name"], "demo");
    assert_eq!(pending["rounds_used"], 1, "待批准态带真实轮耗");
    assert!(
        pending["generated_text"].is_string(),
        "待批准态保留已生成文本"
    );
    let approval_id = pending["approval_id"].as_str().unwrap().to_string();

    let (status, frame) = send(router, resolve_request(session, &approval_id, "reject")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{frame}");
    assert_eq!(frame["error"]["code"], "turn_not_converged");
    assert!(
        frame["error"]["message"]
            .as_str()
            .unwrap()
            .contains("turn did not converge within 2 rounds"),
        "{}",
        frame["error"]["message"]
    );
    assert_eq!(frame["error"]["details"]["limit"], 2, "真实 limit");
    assert_eq!(frame["error"]["details"]["rounds"], 2, "真实已耗轮数");
    assert_eq!(
        frame["error"]["details"]["pending_tool"], "demo",
        "当时待批准工具名"
    );
    assert_eq!(provider.call_count(), 2, "两轮真实 provider 调用后触顶");
}

/// ⑦: 待批准与轮上限同轮时上报待批准 —— 错误帧不再是 turn_not_converged。
#[tokio::test]
async fn a_pending_close_is_reported_instead_of_the_round_limit_frame() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls(vec![
        gated_tool_call("call-a", "echo a"),
        gated_tool_call("call-b", "echo b"),
    ])]);
    let runtime = build_runtime(provider.clone(), invocations.clone(), 1).await;
    let router = canonical_router(Arc::new(runtime));
    let session = SessionId::new();

    // 轮预算已耗尽的那一轮仍然上报待批准 (202), 不是 422。
    let (status, first) = send(router.clone(), chat_request(session, "run both")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{first}");
    let first_id = first["approval_id"].as_str().unwrap().to_string();

    // 同轮的第二个冻结同样上报待批准: 错误帧不再是 turn_not_converged。
    let (status, second) = send(
        router.clone(),
        resolve_request(session, &first_id, "approve"),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{second}");
    assert_eq!(second["tool_name"], "demo");
    let second_id = second["approval_id"].as_str().unwrap().to_string();

    // 续跑剩余流程走到真实上限: 帧带真实数字 (而不是空转丢信息)。
    let (status, frame) = send(router, resolve_request(session, &second_id, "approve")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{frame}");
    assert_eq!(frame["error"]["code"], "turn_not_converged");
    assert_eq!(frame["error"]["details"]["limit"], 1);
    assert_eq!(frame["error"]["details"]["rounds"], 1);
    assert_eq!(frame["error"]["details"]["pending_tool"], "demo");
    assert_eq!(invocations.load(Ordering::SeqCst), 2, "一审批一动作");
}
