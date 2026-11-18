//! Read-only artifact candidate contract at the real HTTP entry.
//!
//! A candidate turn advertises no tools. The runtime refuses a provider tool
//! attempt before dispatch, and the handler never commits a revision. These
//! tests exercise the real gateway router, the real runtime and a scripted
//! provider; the artifact port is a bounded in-memory fake.

#![allow(dead_code)]
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_gateway::{canonical_router, canonical_router_with_services, GatewayServices};
use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::artifacts::{
    Artifact, ArtifactAccess, ArtifactCapabilities, ArtifactDocument, ArtifactError,
    ArtifactHistoryPage, ArtifactHistoryQuery, ArtifactListPage, ArtifactListQuery,
    ArtifactReceipt, ArtifactRevision, ArtifactService, CommitArtifactRequest,
    CreateArtifactRequest, RestoreArtifactRequest, ARTIFACT_SCHEMA,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome, Runtime,
};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const MODEL: &str = "fake-model-1";
const STORE: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86001";
const ARTIFACT: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86002";
const HEAD_REVISION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86003";
const OLD_REVISION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86004";
const NEW_REVISION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86005";
const OLD_CONTENT: &str = "# 原稿\n第一段。\n";
const HEAD_CONTENT: &str = "# 原稿\n第一段。用户已改。\n";
const NEW_CONTENT: &str = "# 原稿\n第一段。用户又改。\n";
const INSTRUCTION: &str = "把第一段改得更短。";

fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn artifact(head: &str, content: &str) -> Artifact {
    Artifact {
        artifact_id: ARTIFACT.into(),
        work_id: "local-work-1".into(),
        conversation_session_id: None,
        title: "文稿".into(),
        media_type: "text/markdown".into(),
        head_revision: head.into(),
        sequence: 3,
        content_sha256: digest(content),
        content_bytes: content.len() as u64,
        created_at: 100,
        updated_at: 300,
    }
}

fn revision(id: &str, content: &str, sequence: u64) -> ArtifactRevision {
    ArtifactRevision {
        revision_id: id.into(),
        artifact_id: ARTIFACT.into(),
        parent_revision: None,
        sequence,
        content: content.into(),
        content_sha256: digest(content),
        content_bytes: content.len() as u64,
        created_at: 100 + sequence as i64,
        author: ArtifactAccess::local_human().author,
        origin: None,
        restored_from: None,
    }
}

struct FakeStore {
    head_revision: String,
    revisions: BTreeMap<String, (String, u64)>,
}

struct FakeService {
    store: Mutex<FakeStore>,
    reads: AtomicUsize,
    creates: AtomicUsize,
    commits: AtomicUsize,
}

impl FakeService {
    fn new() -> Arc<Self> {
        let mut revisions = BTreeMap::new();
        revisions.insert(OLD_REVISION.into(), (OLD_CONTENT.into(), 1));
        revisions.insert(HEAD_REVISION.into(), (HEAD_CONTENT.into(), 2));
        revisions.insert(NEW_REVISION.into(), (NEW_CONTENT.into(), 3));
        Arc::new(Self {
            store: Mutex::new(FakeStore {
                head_revision: HEAD_REVISION.into(),
                revisions,
            }),
            reads: AtomicUsize::new(0),
            creates: AtomicUsize::new(0),
            commits: AtomicUsize::new(0),
        })
    }

    fn promote_to_new_head(&self) {
        self.store.lock().unwrap().head_revision = NEW_REVISION.into();
    }

    fn head(&self) -> String {
        self.store.lock().unwrap().head_revision.clone()
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }

    fn writes(&self) -> usize {
        self.creates.load(Ordering::SeqCst) + self.commits.load(Ordering::SeqCst)
    }

    fn document(&self, revision_id: Option<&str>) -> Result<ArtifactDocument, ArtifactError> {
        let store = self.store.lock().unwrap();
        let id = revision_id.unwrap_or(&store.head_revision);
        let (content, sequence) = store.revisions.get(id).ok_or(ArtifactError::NotFound)?;
        let (head_content, _) = store
            .revisions
            .get(&store.head_revision)
            .ok_or(ArtifactError::NotFound)?;
        Ok(ArtifactDocument {
            schema: ARTIFACT_SCHEMA,
            store_id: STORE.into(),
            artifact: artifact(&store.head_revision, head_content),
            revision: revision(id, content, *sequence),
        })
    }
}

#[async_trait]
impl ArtifactService for FakeService {
    async fn capabilities(
        &self,
        _access: ArtifactAccess,
    ) -> Result<ArtifactCapabilities, ArtifactError> {
        Err(ArtifactError::Invalid(
            "capabilities is not used here".into(),
        ))
    }

    async fn list(
        &self,
        _access: ArtifactAccess,
        _query: ArtifactListQuery,
    ) -> Result<ArtifactListPage, ArtifactError> {
        Err(ArtifactError::Invalid("list is not used here".into()))
    }

    async fn read(
        &self,
        _access: ArtifactAccess,
        store_id: String,
        artifact_id: String,
        revision_id: Option<String>,
    ) -> Result<ArtifactDocument, ArtifactError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if store_id != STORE || artifact_id != ARTIFACT {
            return Err(ArtifactError::StoreMismatch);
        }
        self.document(revision_id.as_deref())
    }

    async fn history(
        &self,
        _access: ArtifactAccess,
        _artifact_id: String,
        _query: ArtifactHistoryQuery,
    ) -> Result<ArtifactHistoryPage, ArtifactError> {
        Err(ArtifactError::Invalid("history is not used here".into()))
    }

    async fn operation(
        &self,
        _access: ArtifactAccess,
        _store_id: String,
        _operation_id: String,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        Err(ArtifactError::Invalid("operation is not used here".into()))
    }

    async fn create(
        &self,
        _access: ArtifactAccess,
        _request: CreateArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        Err(ArtifactError::Invalid("candidate must not create".into()))
    }

    async fn commit(
        &self,
        _access: ArtifactAccess,
        _artifact_id: String,
        _request: CommitArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        Err(ArtifactError::Invalid("candidate must not commit".into()))
    }

    async fn restore(
        &self,
        _access: ArtifactAccess,
        _artifact_id: String,
        _request: RestoreArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        Err(ArtifactError::Invalid("restore is not used here".into()))
    }
}

enum Step {
    Say(&'static str),
    ToolCalls(Vec<ToolCall>),
}

struct ScriptedProvider {
    id: CapabilityId,
    steps: Vec<Step>,
    calls: AtomicUsize,
    advertised_tools: AtomicUsize,
    promote_on_call: Option<usize>,
    service: Option<Arc<FakeService>>,
}

impl ScriptedProvider {
    fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
            advertised_tools: AtomicUsize::new(0),
            promote_on_call: None,
            service: None,
        })
    }

    fn promoting(steps: Vec<Step>, service: Arc<FakeService>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
            advertised_tools: AtomicUsize::new(0),
            promote_on_call: Some(0),
            service: Some(service),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn advertised_tools(&self) -> usize {
        self.advertised_tools.load(Ordering::SeqCst)
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
        self.advertised_tools
            .fetch_add(request.tools.len(), Ordering::SeqCst);
        if self.promote_on_call == Some(index) {
            if let Some(service) = &self.service {
                service.promote_to_new_head();
            }
        }
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
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for CountingTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "demo".into(),
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
                vec![Arc::new(CountingTool {
                    id: CapabilityId::new("tool.demo").unwrap(),
                    invocations: Arc::clone(invocations),
                }) as Arc<dyn ToolCapability>]
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

struct HookRecorder {
    manifest: ModuleManifest,
    hooks: Arc<AtomicUsize>,
}

#[async_trait]
impl BehaviorModule for HookRecorder {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        _hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hooks.fetch_add(1, Ordering::SeqCst);
        Ok(ModuleOutcome::continue_())
    }
}

async fn build_runtime(provider: Arc<ScriptedProvider>, invocations: Arc<AtomicUsize>) -> Runtime {
    build_runtime_with_recorder(provider, invocations, None).await
}

async fn build_runtime_with_recorder(
    provider: Arc<ScriptedProvider>,
    invocations: Arc<AtomicUsize>,
    hooks: Option<Arc<AtomicUsize>>,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.demo".into()));
    policy.grant(Permission::ExecuteTool(
        "tool.behavior.module.hook_recorder".into(),
    ));
    let mut builder = Runtime::builder()
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations))
        .with_plugin(TestPlugin::provider(provider))
        .with_default_model(MODEL);
    if let Some(hooks) = hooks {
        builder = builder.with_module(Arc::new(HookRecorder {
            manifest: ModuleManifest::new("module.hook_recorder", "records hooks"),
            hooks,
        }));
    }
    builder.build().await.unwrap()
}

fn router(runtime: Runtime, service: Option<Arc<FakeService>>) -> axum::Router {
    let services = GatewayServices {
        artifacts: service.map(|service| service as Arc<dyn ArtifactService>),
        ..GatewayServices::default()
    };
    canonical_router_with_services(Arc::new(runtime), services)
}

fn request(base_revision: &str, base_sha256: &str, instruction: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/v1/artifacts/{ARTIFACT}/candidates"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "store_id": STORE,
                "base_revision": base_revision,
                "base_sha256": base_sha256,
                "instruction": instruction,
            }))
            .unwrap(),
        ))
        .unwrap()
}

async fn send(app: axum::Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_real_candidate_returns_text_and_writes_no_revision() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("# 候选稿\n第一段已缩短。\n")]);
    let app = router(
        build_runtime(provider.clone(), invocations.clone()).await,
        Some(service.clone()),
    );

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{reply}");
    assert_eq!(reply["schema"], 1);
    assert_eq!(reply["kind"], "artifact_candidate");
    assert_eq!(reply["artifact_id"], ARTIFACT);
    assert_eq!(reply["base_revision"], HEAD_REVISION);
    assert_eq!(reply["text"], "# 候选稿\n第一段已缩短。\n");
    assert_eq!(reply["text_sha256"], digest("# 候选稿\n第一段已缩短。\n"));
    assert_eq!(reply["text_bytes"], 34);
    assert_eq!(reply["base_is_current"], true);
    assert_eq!(reply["head_revision"], HEAD_REVISION);
    assert_eq!(reply["head_sha256"], digest(HEAD_CONTENT));
    assert_eq!(reply["tool_calls"], 0);
    assert_eq!(reply["artifact_writes"], 0);
    assert!(reply["request_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    assert!(reply["trace_id"].as_str().is_some_and(|id| !id.is_empty()));
    assert_eq!(provider.call_count(), 1);
    assert_eq!(
        provider.advertised_tools(),
        0,
        "candidate turn advertises no tools"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert_eq!(service.writes(), 0);
    assert_eq!(service.head(), HEAD_REVISION);
}

#[tokio::test]
async fn a_stale_base_is_refused_before_the_provider_is_called() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("must not run")]);
    let app = router(
        build_runtime(provider.clone(), invocations).await,
        Some(service),
    );

    let (status, reply) = send(
        app,
        request(OLD_REVISION, &digest(OLD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{reply}");
    assert_eq!(
        reply["error"]["details"]["artifact_error"],
        "stale_base_revision"
    );
    assert_eq!(reply["error"]["details"]["head_revision"], HEAD_REVISION);
    assert_eq!(
        reply["error"]["details"]["head_sha256"],
        digest(HEAD_CONTENT)
    );
    assert_eq!(provider.call_count(), 0);
}

#[tokio::test]
async fn a_base_digest_mismatch_is_refused_before_the_provider_is_called() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("must not run")]);
    let app = router(
        build_runtime(provider.clone(), invocations).await,
        Some(service),
    );

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest("not the saved body"), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{reply}");
    assert_eq!(
        reply["error"]["details"]["artifact_error"],
        "base_digest_mismatch"
    );
    assert_eq!(
        reply["error"]["details"]["actual_sha256"],
        digest(HEAD_CONTENT)
    );
    assert_eq!(provider.call_count(), 0);
}

#[tokio::test]
async fn a_provider_tool_attempt_is_refused_before_dispatch_and_writes_nothing() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls(vec![ToolCall {
        id: "call-1".into(),
        name: "demo".into(),
        arguments: serde_json::json!({ "command": "echo hi" }),
    }])]);
    let app = router(
        build_runtime(provider.clone(), invocations.clone()).await,
        Some(service.clone()),
    );

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{reply}");
    assert_eq!(
        reply["error"]["details"]["artifact_error"],
        "candidate_tool_call_rejected"
    );
    assert_eq!(reply["error"]["details"]["requested_tool_calls"], 1);
    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert_eq!(service.writes(), 0);
    assert_eq!(service.head(), HEAD_REVISION);
}

#[tokio::test]
async fn a_head_change_during_generation_is_reported_and_not_overwritten() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::promoting(
        vec![Step::Say("# 候选稿\n基于旧头生成。\n")],
        service.clone(),
    );
    let app = router(
        build_runtime(provider, invocations).await,
        Some(service.clone()),
    );

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{reply}");
    assert_eq!(reply["base_is_current"], false);
    assert_eq!(reply["head_revision"], NEW_REVISION);
    assert_eq!(reply["head_sha256"], digest(NEW_CONTENT));
    assert_eq!(reply["artifact_writes"], 0);
    assert_eq!(service.head(), NEW_REVISION);
    assert_eq!(service.writes(), 0);
}

#[tokio::test]
async fn an_empty_candidate_body_is_refused_as_invalid_output() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("")]);
    let app = router(build_runtime(provider, invocations).await, Some(service));

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{reply}");
    assert_eq!(
        reply["error"]["details"]["artifact_error"],
        "candidate_invalid_output"
    );
}

#[tokio::test]
async fn an_oversized_instruction_is_refused_before_generation() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("must not run")]);
    let app = router(
        build_runtime(provider.clone(), invocations).await,
        Some(service),
    );
    let instruction = "x".repeat(64 * 1024 + 1);

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), &instruction),
    )
    .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{reply}");
    assert_eq!(
        reply["error"]["details"]["artifact_error"],
        "candidate_envelope_too_large"
    );
    assert_eq!(provider.call_count(), 0);
}

#[tokio::test]
async fn a_no_tools_candidate_turn_skips_behavior_module_hooks() {
    let service = FakeService::new();
    let invocations = Arc::new(AtomicUsize::new(0));
    let hooks = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("# 原稿\n第一段。\n")]);
    let runtime =
        build_runtime_with_recorder(provider, invocations, Some(Arc::clone(&hooks))).await;
    let app = router(runtime, Some(service));

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{reply}");
    assert_eq!(
        hooks.load(Ordering::SeqCst),
        0,
        "a read-only candidate turn must not run behavior hooks"
    );
}

#[tokio::test]
async fn a_missing_artifact_service_is_explicitly_unsupported() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say("must not run")]);
    let app = canonical_router(Arc::new(build_runtime(provider, invocations).await));

    let (status, reply) = send(
        app,
        request(HEAD_REVISION, &digest(HEAD_CONTENT), INSTRUCTION),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{reply}");
    assert_eq!(reply["error"]["details"]["artifact_error"], "unsupported");
}
