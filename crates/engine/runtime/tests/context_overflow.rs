//! Overflow self-healing wiring: when a provider reports the context window as
//! exceeded, the injected-context budget shrinks and the request is reassembled
//! and resent — bounded by a retry cap and by the progress guard (a retry only
//! happens when the smaller budget produces a different assembly). Normal
//! requests see none of this: one provider call and zero recovery events.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_orchestration::context_overflow::MAX_OVERFLOW_RETRIES;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{
    AgentModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome, Runtime,
    RuntimeError, TraceEvent, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-overflow";
const IDENTITY: &str = "system identity safety preamble";

/// A provider that accepts a request only while its total message text fits
/// `limit_chars`, and reports the context-window-exceeded class otherwise.
/// `always_over` reports it unconditionally.
struct WindowProbe {
    id: CapabilityId,
    requests: Mutex<Vec<NormalizedRequest>>,
    limit_chars: usize,
    always_over: bool,
}

impl WindowProbe {
    fn new(limit_chars: usize, always_over: bool) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            requests: Mutex::new(Vec::new()),
            limit_chars,
            always_over,
        })
    }

    fn calls(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    fn request_text(&self, index: usize) -> String {
        request_text(&self.requests.lock().unwrap()[index])
    }
}

fn request_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect::<Vec<_>>()
        .join("\n---\n")
}

#[async_trait]
impl ProviderCapability for WindowProbe {
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
        self.requests.lock().unwrap().push(request.clone());
        let total_chars: usize = request
            .messages
            .iter()
            .map(|message| ContentPart::join_text(&message.content).chars().count())
            .sum();
        if self.always_over || total_chars > self.limit_chars {
            return Err(ProviderError::BadResponse {
                provider: "provider.fake".into(),
                detail:
                    "vendor returned 400: context length exceeded (request over the context window)"
                        .into(),
            });
        }
        Ok(NormalizedResponse {
            id: "response-0".into(),
            model: request.model.clone(),
            content: "ok".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<WindowProbe>,
}

impl ProviderPlugin {
    fn new(provider: Arc<WindowProbe>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.fake_provider").unwrap(),
                "1.0.0",
                "fake provider",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "fake provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

/// Injects one transient system overlay at TurnStart (the shape used by the
/// memory / organ / lesson modules).
struct InjectOverlays {
    manifest: ModuleManifest,
    overlay: String,
}

#[async_trait]
impl AgentModule for InjectOverlays {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        Ok(ModuleOutcome::continue_().with_system_overlay(self.overlay.clone()))
    }
}

async fn runtime_with(
    provider: Arc<WindowProbe>,
    overlay: String,
    budget: usize,
    spill_root: Option<&std::path::Path>,
) -> Runtime {
    let module = InjectOverlays {
        manifest: ModuleManifest::new("cognitive.inject", "inject"),
        overlay,
    };
    let mut builder = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(Arc::new(module))
        .with_context_budget_chars(budget);
    if let Some(root) = spill_root {
        builder = builder.with_context_spill_root(root);
    }
    builder.build().await.unwrap()
}

/// Window-exceeded shrinks the injected-context budget and resends: three
/// provider calls carry strictly different (smaller) assemblies until the
/// request fits, and the trace records each shrink with budget sizes and
/// attempt counts only.
#[tokio::test]
async fn window_exceeded_shrinks_the_budget_and_reassembles_until_it_fits() {
    let provider = WindowProbe::new(2_400, false);
    let spill_root = tempfile::tempdir().unwrap();
    let runtime = runtime_with(
        Arc::clone(&provider),
        "X".repeat(9_000),
        4_000,
        Some(spill_root.path()),
    )
    .await;

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(IDENTITY))
        .await
        .unwrap();

    assert_eq!(response.text, "ok", "缩到能放下后请求成功");
    assert_eq!(provider.calls(), 3, "1 次原始调用 + 2 次降预算重试");

    // Progress in plain sight: each reassembly is smaller than the last.
    let sizes: Vec<usize> = (0..provider.calls())
        .map(|index| provider.request_text(index).chars().count())
        .collect();
    assert!(
        sizes[1] < sizes[0] && sizes[2] < sizes[1],
        "重组结果必须逐次变小: {sizes:?}"
    );

    // Sanitized log: every retry recorded, budgets monotonically decreasing,
    // and no injected content in the record.
    let shrinks: Vec<(u32, usize, usize)> = response
        .trace
        .events()
        .filter_map(|event| match event {
            TraceEvent::ContextBudgetShrunk {
                attempt,
                from_budget_chars,
                to_budget_chars,
                ..
            } => Some((*attempt, *from_budget_chars, *to_budget_chars)),
            _ => None,
        })
        .collect();
    assert_eq!(
        shrinks,
        vec![(1, 4_000, 2_800), (2, 2_800, 1_960)],
        "每次重试都留脱敏日志且预算单调下降"
    );
    let trace_json = serde_json::to_string(&response.trace).unwrap();
    assert!(
        !trace_json.contains("XXXXXXXX"),
        "日志不得带注入内容: {trace_json}"
    );
}

/// The retry cap is a hard bound: when the provider keeps reporting the window
/// as exceeded, exactly `MAX_OVERFLOW_RETRIES` retries run and the original
/// error then stands.
#[tokio::test]
async fn overflow_recovery_stops_at_the_retry_cap() {
    let provider = WindowProbe::new(0, true);
    let spill_root = tempfile::tempdir().unwrap();
    let runtime = runtime_with(
        Arc::clone(&provider),
        "X".repeat(9_000),
        4_000,
        Some(spill_root.path()),
    )
    .await;

    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(IDENTITY))
        .await
        .expect_err("provider 持续报超窗时必须按原错误失败");

    assert_eq!(
        provider.calls(),
        1 + MAX_OVERFLOW_RETRIES as usize,
        "重试上限 {MAX_OVERFLOW_RETRIES}"
    );
    assert!(error.is_context_window_exceeded(), "{error}");
    assert!(matches!(error, RuntimeError::Provider(_)), "{error}");
}

/// The progress guard intercepts a retry that cannot change the assembly: with
/// no spill sink the overlays pass through unchanged at every budget, so the
/// first window-exceeded report fails the request instead of retrying.
#[tokio::test]
async fn progress_guard_blocks_retries_that_cannot_reassemble_differently() {
    let provider = WindowProbe::new(0, true);
    let runtime = runtime_with(Arc::clone(&provider), "X".repeat(9_000), 4_000, None).await;

    let error = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(IDENTITY))
        .await
        .expect_err("无进展的重试必须被拦截");

    assert_eq!(provider.calls(), 1, "组装结果不变 = 零重试");
    assert!(error.is_context_window_exceeded(), "按原错误失败: {error}");
}

/// Normal requests are untouched by overflow recovery: one provider call, one
/// response, zero recovery events.
#[tokio::test]
async fn normal_requests_do_not_enter_overflow_recovery() {
    let provider = WindowProbe::new(usize::MAX, false);
    let spill_root = tempfile::tempdir().unwrap();
    let runtime = runtime_with(
        Arc::clone(&provider),
        "X".repeat(50),
        4_000,
        Some(spill_root.path()),
    )
    .await;

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(IDENTITY))
        .await
        .unwrap();

    assert_eq!(response.text, "ok");
    assert_eq!(provider.calls(), 1, "正常请求零影响: 只调一次");
    assert_eq!(
        response
            .trace
            .events()
            .filter(|event| matches!(event, TraceEvent::ContextBudgetShrunk { .. }))
            .count(),
        0,
        "正常请求不产生溢出自愈事件"
    );
}
