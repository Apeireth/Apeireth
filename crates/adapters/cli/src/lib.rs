//! Canonical Apeireth command-line entry points.
//!
//! The CLI is a thin adapter: it bootstraps one canonical runtime, constructs
//! a canonical turn request, and delegates execution to
//! `Runtime::execute_outcome`. Pending approvals keep their `ApprovalId`.

// v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring / EncryptedFile backend
// (per `v2.0.0-rc-roadmap.md` §3 RC-9: "keyring 真正接到 EnvCredentialResolver 之前").
// 0 装诚实: 4 backend + KeyringSelector alpha 已真 impl; 本模块只做 bootstrap 集成.
pub mod gateway_panels;
pub mod keyring_bootstrap;
pub mod portable_bundle;

pub use portable_bundle::{PortableBundleManifest, PortableBundleSynthesizer};

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{ApprovalId, CapabilityId, SessionId};
use apeireth_governance::TurnSecurityContext;
use apeireth_governance::{
    CredentialDisclosureHook, GovernancePipeline, Permission, PermissionGovernanceHook,
    PermissionPolicy, PromptInjectionHook,
};
use apeireth_guard::{
    ClassifierEnforcementMode, DatasetRecorder, IntentInput, IntentInterpreter,
    JointRiskClassifier, RuleIntentInterpreter,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, PendingApprovalView, Runtime, SessionStore, TurnOutcome,
    TurnRequest, TurnResponse,
};
use apeireth_runtime_assembly::SqliteSessionStore;
use apeireth_tools_canonical::{FetchConfig, TrustedShellConfig};

/// One persistent SQLite database is shared by the cognitive backends.
/// `APEIRETH_COGNITIVE_DB` may override the path; Judge remains opt-in.
const COGNITIVE_DB_ENV: &str = "APEIRETH_COGNITIVE_DB";
const SESSION_DB_ENV: &str = "APEIRETH_SESSION_DB";
const COGNITIVE_JUDGE_ENV: &str = "APEIRETH_COGNITIVE_JUDGE";
const COGNITIVE_COUNCIL_ENV: &str = "APEIRETH_COGNITIVE_COUNCIL";
/// Enables the desensitized Guard dataset event stream when set to `1`.
pub const GUARD_DATASET_ENABLED_ENV: &str = "APEIRETH_GUARD_DATASET_ENABLED";
/// Optional JSONL path for the composition-owned Guard dataset recorder.
pub const GUARD_DATASET_PATH_ENV: &str = "APEIRETH_GUARD_DATASET_PATH";
/// Local Guard model mode: disabled, shadow, advisory, or enforce.
pub const GUARD_ML_MODE_ENV: &str = "APEIRETH_GUARD_ML_MODE";
/// Local JSON model artifact path. Paths are never returned in Guard status.
pub const GUARD_ML_MODEL_ENV: &str = "APEIRETH_GUARD_ML_MODEL";

/// Legacy opt-in for the local filesystem and search tools. Accepted for
/// compatibility; the tools are granted by default now.
pub const ENABLE_LOCAL_READ_TOOLS_ENV: &str = "APEIRETH_ENABLE_LOCAL_READ_TOOLS";
/// Privacy escape hatch: when set to `1`, the local filesystem and search
/// tools are NOT granted, even though they default to on.
pub const DISABLE_LOCAL_READ_TOOLS_ENV: &str = "APEIRETH_DISABLE_LOCAL_READ_TOOLS";
// 2026-09-08 用户旋钮 (per docs/01-architecture/forget-three-phase-production-spec 同批):
// shell/fetch 注册 + 策略 grant + require_approval (每次调用仍走人工审批);
// organs / preference_learning 为认知模块装配旋钮。
const ENABLE_SHELL_ENV: &str = "APEIRETH_ENABLE_SHELL";
const ENABLE_FETCH_ENV: &str = "APEIRETH_ENABLE_FETCH";
const ENABLE_ORGANS_ENV: &str = "APEIRETH_ENABLE_ORGANS";
const ENABLE_PREFERENCE_LEARNING_ENV: &str = "APEIRETH_ENABLE_PREFERENCE_LEARNING";
// 2026-10-06 W2 接线批 (engineering-review-handoff-2026-10-06.md §5 W2):
// 记忆召回三组旋钮 —— proactive recall 补缺 (已接线无开关)、typed 写读对称
// (默认开 + 逃生门)、语义向量阶段 (真实现 + opt-in, 词法回退兜底)。
const ENABLE_PROACTIVE_RECALL_ENV: &str = "APEIRETH_ENABLE_PROACTIVE_RECALL";
const TYPED_RECALL_DISABLE_ENV: &str = "APEIRETH_DISABLE_TYPED_RECALL";
const PERSONA_ID_ENV: &str = "APEIRETH_PERSONA_ID";
const SUBJECT_ID_ENV: &str = "APEIRETH_SUBJECT_ID";
const DEFAULT_PERSONA_ID: &str = "apeireth";
const DEFAULT_SUBJECT_ID: &str = "local-user";
// 2026-10-06 W2 记忆闭环批: donor 反幻觉注入格式 / 每轮记忆整理 / 失败闭环。
const ENABLE_MEMORY_INJECTION_ENV: &str = "APEIRETH_ENABLE_MEMORY_INJECTION";
const ENABLE_CONSOLIDATION_ENV: &str = "APEIRETH_ENABLE_CONSOLIDATION";
const ENABLE_REFLEXION_ENV: &str = "APEIRETH_ENABLE_REFLEXION";
const REFLEXION_DIR_ENV: &str = "APEIRETH_REFLEXION_DIR";

/// Resolve the local read-tools switch from the process environment.
///
/// Semantics (fail-closed): `APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` disables
/// the tools; the legacy `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` explicitly
/// enables them; with neither set (or any other value) they default to on.
/// When both are set to `1`, DISABLE wins.
fn local_read_tools_enabled_from_env() -> bool {
    if std::env::var(DISABLE_LOCAL_READ_TOOLS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return false;
    }
    if std::env::var(ENABLE_LOCAL_READ_TOOLS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return true;
    }
    true
}

/// Opt-in proactive recall policy (2026-10-06 W2 接线批补缺: 该路径早已接进
/// `MemoryRecallModule`, 但 `Default = None` 且无任何旋钮可达)。
///
/// `APEIRETH_ENABLE_PROACTIVE_RECALL=1` → 确定性默认策略 (enabled, budget 2,
/// 阈值 0.10); 其余值或缺省 → `None` (默认关, 行为不变)。
pub fn proactive_recall_policy_from_env() -> Option<apeireth_memory::ProactiveRecallPolicy> {
    std::env::var(ENABLE_PROACTIVE_RECALL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
        .then(|| apeireth_memory::ProactiveRecallPolicy::default().enabled(true))
}

/// Typed (commitment/persona/relation) 召回读侧开关 (2026-10-06 W2 写读对称修复)。
///
/// **默认开**: 写侧 `CanonicalMemoryTypedSink` 早已在生产落库, 读侧此前恒缺
/// (数据入库永不召回) —— 开关默认开是对称修复而非新功能; `APEIRETH_DISABLE_TYPED_RECALL=1`
/// 为整体逃生门 (fail-closed 惯例)。
pub fn typed_recall_enabled_from_env() -> bool {
    !std::env::var(TYPED_RECALL_DISABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// Typed memory 的显式主体身份 (写侧 persona delta 与读侧 typed 召回共用)。
///
/// 本地产品单主体: 默认 `apeireth` / `local-user`, 多主体部署用
/// `APEIRETH_PERSONA_ID` / `APEIRETH_SUBJECT_ID` 覆写 (空白值视为未设)。
pub fn typed_recall_identity_from_env() -> apeireth_memory::TypedRecallIdentity {
    fn value(key: &str, default: &str) -> String {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default.to_string())
    }
    apeireth_memory::TypedRecallIdentity {
        persona_id: value(PERSONA_ID_ENV, DEFAULT_PERSONA_ID),
        subject_id: value(SUBJECT_ID_ENV, DEFAULT_SUBJECT_ID),
    }
}

/// 语义向量阶段 (2026-10-06 W2): 召回的 embedding 候选打分接线。
///
/// Fail-closed 语义: URL 与 MODEL **双缺** → `Ok(None)` (词法回退, 行为不变);
/// **只设其一** → 报错 (半配是配置事故, 大声失败绝不静默); 双全 → 构造
/// OpenAI-compatible embeddings transport 注入组装根 (构造不发网络)。
/// `APEIRETH_EMBEDDING_KEY` 可选 (本地免鉴权端点不设)。
pub fn embedding_provider_from_env(
) -> Result<Option<Arc<dyn apeireth_memory::EmbeddingProvider>>, apeireth_memory::EmbeddingError> {
    use apeireth_provider::embeddings::{
        OpenAiCompatibleEmbeddingProvider, EMBEDDING_MODEL_ENV, EMBEDDING_URL_ENV,
    };
    let url = std::env::var(EMBEDDING_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let model = std::env::var(EMBEDDING_MODEL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    match (url, model) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => Ok(Some(Arc::new(OpenAiCompatibleEmbeddingProvider::from_env()?))),
        _ => Err(apeireth_memory::EmbeddingError::Unavailable(format!(
            "partial embedding config: set both {EMBEDDING_URL_ENV} and {EMBEDDING_MODEL_ENV} or neither"
        ))),
    }
}

/// donor 反幻觉注入格式 (2026-10-06 W2): `APEIRETH_ENABLE_MEMORY_INJECTION=1`
/// 把记忆 overlay 换成编号证据清单 + 「禁止说『我记得我们以前聊过』」规则; 默认 XML 格式不变。
pub fn memory_injection_enabled_from_env() -> bool {
    std::env::var(ENABLE_MEMORY_INJECTION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 每轮记忆整理 (2026-10-06 W2): `APEIRETH_ENABLE_CONSOLIDATION=1` 时 AfterTurn
/// 跑确定性 consolidation 报告并把提炼洞察落库 (稳定 ID 幂等); 默认关。
pub fn consolidation_enabled_from_env() -> bool {
    std::env::var(ENABLE_CONSOLIDATION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// reflexion 失败闭环 (2026-10-06 W2): `APEIRETH_ENABLE_REFLEXION=1` 注册
/// `cognitive.reflexion` 模块 (TurnStart 教训注入 + AfterTurn 判定沉淀); 默认关。
pub fn reflexion_enabled_from_env() -> bool {
    std::env::var(ENABLE_REFLEXION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// reflexion 存储根目录: `APEIRETH_REFLEXION_DIR` 覆写, 默认 `<data>/reflexion`。
pub fn reflexion_store_root_from_env() -> PathBuf {
    std::env::var(REFLEXION_DIR_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_panel_data_dir().join("reflexion"))
}

/// Build the production governance policy from an explicit local-read choice.
///
/// The explicit boolean keeps the policy deterministic and easy to test. The
/// environment-facing wrapper is [`build_production_governance_from_env`].
/// Authorization is deliberately the first hook: a later content-risk hook
/// must never turn an unauthorized capability into an approval request.
pub fn build_production_governance(enable_local_read_tools: bool) -> GovernancePipeline {
    build_production_governance_parts(enable_local_read_tools).0
}

use apeireth_guard::BehaviorChainGuardHook;

/// Build the production pipeline and return the shared policy handle alongside
/// it, so the gateway can serve grants listing and session-scoped hot revoke
/// against the same policy the live hooks evaluate.
pub fn build_production_governance_parts(
    enable_local_read_tools: bool,
) -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    build_production_governance_parts_with_dataset(enable_local_read_tools, None)
}

fn build_production_governance_parts_with_dataset(
    enable_local_read_tools: bool,
    dataset: Option<Arc<DatasetRecorder>>,
) -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    if enable_local_read_tools {
        policy.grant(Permission::ExecuteTool("tool.filesystem".to_string()));
        policy.grant(Permission::ExecuteTool("tool.search".to_string()));
    }
    let policy = Arc::new(std::sync::Mutex::new(policy));
    let mut guard = BehaviorChainGuardHook::new();
    if let Some(dataset) = dataset {
        guard = guard.with_dataset_recorder(dataset);
    }
    guard = configure_guard_classifier(guard);
    let guard_hook = Arc::new(guard);

    let pipeline = GovernancePipeline::new()
        .with(Arc::new(PermissionGovernanceHook::new_shared(
            policy.clone(),
        )))
        .with(Arc::new(CredentialDisclosureHook::new()))
        .with(Arc::new(PromptInjectionHook::new()))
        .with(guard_hook.clone());
    (pipeline, policy, guard_hook)
}

/// Build the production governance policy using the process environment.
///
/// The local read tools (`tool.filesystem`/`tool.search`) are granted by
/// default, matching `tool.repo`. `APEIRETH_DISABLE_LOCAL_READ_TOOLS=1`
/// disables them (privacy escape hatch) and wins over the legacy
/// `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` opt-in. Shell, fetch, and unknown
/// capabilities remain denied even if a future plugin registers them.
pub fn build_production_governance_from_env() -> GovernancePipeline {
    build_production_governance_parts_from_env().0
}

fn configure_guard_classifier(mut guard: BehaviorChainGuardHook) -> BehaviorChainGuardHook {
    let mode = std::env::var(GUARD_ML_MODE_ENV)
        .ok()
        .and_then(|value| ClassifierEnforcementMode::parse(&value));
    let Some(mode) = mode else {
        return guard;
    };
    let Some(path) = std::env::var(GUARD_ML_MODEL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return guard;
    };
    if let Ok(classifier) = JointRiskClassifier::from_path(path).map(|model| model.with_mode(mode))
    {
        guard = guard.with_classifier(Arc::new(classifier));
    }
    guard
}

fn production_guard_dataset_recorder() -> Option<Arc<DatasetRecorder>> {
    let enabled = std::env::var(GUARD_DATASET_ENABLED_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    if !enabled {
        return None;
    }
    let path = std::env::var(GUARD_DATASET_PATH_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".apeireth/guard-dataset-v3.jsonl".to_string());
    let recorder = Arc::new(DatasetRecorder::new(path));
    recorder.set_enabled(true);
    Some(recorder)
}

fn build_production_governance_parts_from_env() -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    let (pipeline, policy, guard_hook) = build_production_governance_parts_with_dataset(
        enable_local_read_tools,
        production_guard_dataset_recorder(),
    );

    // 2026-09-08: shell/fetch 用户旋钮 = 注册 + 策略 grant + require_approval.
    // 语义 (fail-closed): 主人 env 显式授权"可以提议执行", 但**每次调用仍走
    // 人工审批流** (与既有 approval 生命周期一致; 无审批者时拒绝)。
    let enable_shell = std::env::var(ENABLE_SHELL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let enable_fetch = std::env::var(ENABLE_FETCH_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    if enable_shell || enable_fetch {
        let mut guard = policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)");
        if enable_shell {
            guard.grant(Permission::ExecuteTool("tool.shell".to_string()));
            guard.require_approval_for("tool.shell");
        }
        if enable_fetch {
            guard.grant(Permission::ExecuteTool("tool.fetch".to_string()));
            guard.require_approval_for("tool.fetch");
        }
    }
    (pipeline, policy, guard_hook)
}

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
///
/// Provider implementations are injected as plugins. Credentials are resolved
/// at execution time, so neither the runtime nor a provider stores API keys.
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    let (runtime, _, _, _, _) = build_canonical_runtime_with_sessions_from_env().await?;
    Ok(runtime)
}

/// Build the runtime and return the shared session-store, memory-backend and
/// permission-policy handles alongside it, so the gateway can serve the
/// sessions/memory/permissions introspection surfaces from the same durable
/// stores and the same live policy.
pub async fn build_canonical_runtime_with_sessions_from_env() -> Result<
    (
        Runtime,
        Arc<dyn SessionStore>,
        Arc<dyn MemoryBackend>,
        Arc<std::sync::Mutex<PermissionPolicy>>,
        Arc<BehaviorChainGuardHook>,
    ),
    String,
> {
    let session_store = production_session_store().await?;
    let clock: Arc<dyn apeireth_core::kernel::Clock> = apeireth_core::kernel::system_clock();
    let (cognitive, memory) = build_cognitive_modules_from_env(Arc::clone(&clock)).await?;
    let (runtime, policy, guard_hook) =
        build_canonical_runtime_with_parts(session_store.clone(), cognitive, clock).await?;
    Ok((runtime, session_store, memory, policy, guard_hook))
}

async fn build_canonical_runtime_with_parts(
    session_store: Arc<dyn SessionStore>,
    cognitive: apeireth_runtime_assembly::ProductionCognitiveModules,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
) -> Result<
    (
        Runtime,
        Arc<std::sync::Mutex<PermissionPolicy>>,
        Arc<BehaviorChainGuardHook>,
    ),
    String,
> {
    use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
    use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
    use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;

    let configured_model = std::env::var("APEIRETH_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty());

    let mut builder = Runtime::builder().with_clock(Arc::clone(&clock));
    // P-arch (2026-08-27) + v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring
    // 优先用 keyring (设 APEIRETH_KEYRING_BACKEND env), fallback 到 EnvCredentialResolver
    // (alpha 0 装路径, 0 行为变化). 详见 `keyring_bootstrap` 模块.
    let resolver: Arc<dyn apeireth_plugin::CredentialResolver> =
        keyring_bootstrap::build_keyring_resolver();
    builder = builder.with_credentials(resolver);
    let (governance, policy, guard_hook) = build_production_governance_parts_from_env();
    // Session-level permission presets (read_only/standard/full) layer on top
    // of the production policy. The wrapper reads the session's durable
    // settings on every capability dispatch and delegates everything else.
    let preset_governance =
        apeireth_runtime_assembly::canonical::PermissionPresetGovernanceHook::new(
            Arc::new(governance),
            Arc::clone(&session_store),
        );
    builder = builder.with_governance(Arc::new(preset_governance));
    builder = builder.with_session_store(session_store);

    // The CLI is the composition root. Gateway reuses this function, while
    // SDK remains an HTTP client and does not host a second Runtime.
    // Builtin tools are owned by ProductionModules, not BuiltinToolsPlugin.
    builder = cognitive.register_context_projection(builder);
    builder = cognitive.register_into(builder);

    let first_default_model: Option<String>;
    let mut fallback_order: Vec<CapabilityId> = Vec::new();

    let minimax = MinimaxProviderPlugin::from_env()
        .map_err(|error| format!("minimax provider activation failed: {error}"))?;
    first_default_model = minimax.model_ids().first().cloned();
    fallback_order.push(CapabilityId::new("provider.minimax").unwrap());
    builder = builder.with_plugin(Arc::new(minimax));

    let anthropic = AnthropicProviderPlugin::from_env()
        .map_err(|error| format!("anthropic provider activation failed: {error}"))?;
    fallback_order.push(CapabilityId::new("provider.anthropic").unwrap());
    builder = builder.with_plugin(Arc::new(anthropic));

    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .as_ref()
        .is_some_and(|models| !models.trim().is_empty())
    {
        let openai = OpenAiCompatibleProviderPlugin::from_env()
            .map_err(|error| format!("openai-compatible provider activation failed: {error}"))?;
        fallback_order.push(CapabilityId::new("provider.openai-compatible").unwrap());
        builder = builder.with_plugin(Arc::new(openai));
    }

    builder = builder.with_fallback_order(fallback_order);
    if let Some(model) = configured_model.or(first_default_model) {
        builder = builder.with_default_model(model);
    }

    let runtime = builder
        .build()
        .await
        .map_err(|error| format!("canonical runtime bootstrap failed: {error}"))?;
    if let Some(recorder) = guard_hook.dataset_recorder() {
        runtime.add_event_sink(Arc::new(
            apeireth_runtime_assembly::GuardDatasetObserver::new(recorder)
                .with_hook(guard_hook.clone()),
        ));
    }
    Ok((runtime, policy, guard_hook))
}

async fn production_session_store() -> Result<Arc<dyn apeireth_runtime::SessionStore>, String> {
    let path = std::env::var(SESSION_DB_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".apeireth/sessions.sqlite3".into());
    SqliteSessionStore::open(&path)
        .await
        .map(|store| Arc::new(store) as Arc<dyn apeireth_runtime::SessionStore>)
        .map_err(|error| format!("session store open failed: {error}"))
}

/// Build the direct CLI runtime with the same trace/audit observer used by the
/// HTTP gateway. The CLI has no SSE transport, but its turns must still be
/// observable and durable through the gateway bounded-context ports.
/// Build the direct CLI runtime together with its trace/audit observer.
///
/// Dataset recording is installed during the inner canonical bootstrap and
/// the returned observation sink is added additively, so both observers see
/// the same runtime event spine.
pub async fn build_canonical_runtime_from_env_with_observability(
) -> Result<(Arc<Runtime>, Arc<apeireth_gateway::RuntimeObservationSink>), String> {
    let (runtime, sessions, memory, policy, guard_hook) =
        build_canonical_runtime_with_sessions_from_env().await?;
    let runtime = Arc::new(runtime);
    let governance = Arc::new(
        apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
            .map_err(|error| format!("memory governance store open failed: {error}"))?,
    );
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    let panel = Arc::new(
        crate::gateway_panels::CliPanelData::new_with_runtime(
            Arc::clone(&runtime),
            sessions,
            memory,
            governance,
            policy,
            enable_local_read_tools,
            default_panel_data_dir(),
        )
        .with_guard(guard_hook.clone()),
    );
    let services = crate::gateway_panels::gateway_services(panel);
    let observer = Arc::new(apeireth_gateway::RuntimeObservationSink::new(
        services.trace_commands.clone(),
        services.audit_commands.clone(),
    ));
    runtime.add_event_sink(observer.clone());
    Ok((runtime, observer))
}

/// Completed turn or a pending approval that still has its [`ApprovalId`].
#[derive(Debug, Clone)]
pub enum CanonicalCliTurn {
    /// The turn reached a final assistant response.
    Completed(TurnResponse),
    /// The turn is suspended until a human resolves the returned approval.
    PendingApproval(PendingApprovalView),
}

async fn build_cognitive_modules_from_env(
    clock: Arc<dyn apeireth_core::kernel::Clock>,
) -> Result<
    (
        apeireth_runtime_assembly::ProductionCognitiveModules,
        Arc<dyn MemoryBackend>,
    ),
    String,
> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_memory::{
        experience_store_sqlite::SQLiteExperienceStore,
        preference_store_sqlite::SQLitePreferenceStore,
        self_assessment_store_sqlite::SQLiteSelfAssessmentStore,
    };
    use apeireth_runtime_assembly::{CognitiveBackends, CognitiveModuleConfig, JudgeConfig};
    use apeireth_storage::SqliteConnectionPool;

    let path = cognitive_db_path();
    let pool = Arc::new(
        SqliteConnectionPool::open(&path)
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );

    // The storage foundation has an older generic `episodes(id, data)` table.
    // Refuse that shape explicitly instead of letting an additive
    // `CREATE IF NOT EXISTS` migration produce a runtime write failure.
    pool.read(|conn| {
        let mut statement = conn.prepare("PRAGMA table_info(episodes)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !columns.is_empty()
            && ["id", "timestamp", "role", "content", "session_id"]
                .iter()
                .any(|required| !columns.iter().any(|column| column == required))
        {
            return Err(apeireth_storage::StorageError::InvalidConfiguration(
                "cognitive database uses incompatible generic episodes schema; migrate or choose a new APEIRETH_COGNITIVE_DB path".into(),
            ));
        }
        Ok(())
    })
    .map_err(|error| format!("cognitive database schema validation failed: {error}"))?;

    // Memory migrations own the episode and six-stream tables. The preference,
    // experience, and assessment stores own their additive tables. All use
    // this one injected pool; no module opens a connection itself.
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .map_err(|error| format!("cognitive memory schema failed: {error}"))?;

    let experience = Arc::new(SQLiteExperienceStore::from_arc(Arc::clone(&pool)));
    experience
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive experience schema failed: {error}"))?;
    let preferences = Arc::new(SQLitePreferenceStore::from_arc(Arc::clone(&pool)));
    preferences
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive preference schema failed: {error}"))?;
    let self_assessments = Arc::new(SQLiteSelfAssessmentStore::from_arc(Arc::clone(&pool)));
    self_assessments
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive assessment schema failed: {error}"))?;

    let access_history = Arc::new(apeireth_memory::SqliteAccessHistoryStore::from_arc(
        Arc::clone(&pool),
        256,
    ));
    let commitment_store = Arc::new(apeireth_memory::SqliteCommitmentStore::from_arc(
        Arc::clone(&pool),
    ));
    commitment_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive commitment schema failed: {error}"))?;
    let persona_store = Arc::new(apeireth_memory::SqlitePersonaProfileStore::new(
        (*pool).clone(),
    ));
    persona_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive persona schema failed: {error}"))?;
    let relation_store = Arc::new(apeireth_memory::SqliteTemporalGraphStore::from_arc(
        Arc::clone(&pool),
    ));
    relation_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive relation schema failed: {error}"))?;
    // 2026-10-06 W2 接线批: typed 写读对称修复。写侧 typed_sink 早已在生产
    // 落 commitment/persona/relation, 但读侧 typed_recall 恒 None —— 入库后
    // 永不进召回。此处把同一组 store 接给 SqliteTypedMemoryRecallSource。
    // episodes 槽**不**接: episodic 候选已由 scoped_memory 供, 且该槽吃
    // `SqliteMemoryStore` 与生产池 `SqliteBackend` 不同型, 不为接线引入第二套连接。
    let typed_identity = typed_recall_identity_from_env();
    let typed_recall_enabled = typed_recall_enabled_from_env();
    let typed_source: Arc<dyn apeireth_memory::TypedMemoryRecallSource> = Arc::new(
        apeireth_runtime_assembly::SqliteTypedMemoryRecallSource::new()
            .with_commitments(Arc::clone(&commitment_store))
            .with_persona(Arc::clone(&persona_store))
            .with_relations(Arc::clone(&relation_store)),
    );
    let persona_store: Arc<dyn apeireth_memory::PersonaProfileStore> = persona_store;
    let typed_sink = Arc::new(
        apeireth_runtime_assembly::CanonicalMemoryTypedSink::new()
            .with_commitments(Arc::clone(&commitment_store))
            .with_persona_store(Arc::clone(&persona_store))
            .with_identity(
                typed_identity.persona_id.clone(),
                typed_identity.subject_id.clone(),
            )
            .with_relations(Arc::clone(&relation_store)),
    );
    let typed_sink: Arc<dyn apeireth_memory::MemoryTypedMaterializationSink> = typed_sink;
    access_history
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive access history schema failed: {error}"))?;
    let judge_enabled = std::env::var(COGNITIVE_JUDGE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let council_enabled = std::env::var(COGNITIVE_COUNCIL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let organs_enabled = std::env::var(ENABLE_ORGANS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let preference_learning_enabled = std::env::var(ENABLE_PREFERENCE_LEARNING_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let shell_enabled = std::env::var(ENABLE_SHELL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let fetch_enabled = std::env::var(ENABLE_FETCH_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let reflexion_enabled = reflexion_enabled_from_env();
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config = CognitiveModuleConfig {
        judge: JudgeConfig {
            enabled: judge_enabled,
            ..JudgeConfig::default()
        },
        council: council_enabled,
        organs: organs_enabled,
        preference_learning: preference_learning_enabled,
        proactive_recall: proactive_recall_policy_from_env(),
        memory_injection: memory_injection_enabled_from_env(),
        consolidation: consolidation_enabled_from_env(),
        reflexion: reflexion_enabled,
        // shell/fetch 旋钮: 只注册工具; 执行许可由治理层 grant+approval 决定.
        // W1 §2.4 (2026-10-10): shell 沙箱全局旋钮, 默认**开** (设计拍板
        // "产品定位=桌面伴侣"); APEIRETH_SHELL_SANDBOX=0 显式裸跑自担风险。
        shell: shell_enabled.then(|| {
            TrustedShellConfig::new(workspace_root.clone())
                .with_sandbox(shell_sandbox_enabled_from_env())
        }),
        fetch: fetch_enabled.then(FetchConfig::public_internet_only),
        ..CognitiveModuleConfig::default()
    };
    let sqlite_backend = Arc::new(SqliteBackend::from_arc(Arc::clone(&pool)));
    let memory: Arc<dyn apeireth_plugin::memory_backend::MemoryBackend> = sqlite_backend.clone();
    let memory_governance: Arc<dyn apeireth_memory::MemoryGovernanceStore> = sqlite_backend.clone();
    let wiki: Arc<dyn apeireth_plugin::experience::WikiEntryStore> = experience.clone();
    let graph: Arc<dyn apeireth_plugin::experience::KnowledgeGraphStore> = experience.clone();
    let associations: Arc<dyn apeireth_plugin::experience::AssociationStore> = experience.clone();
    let preferences: Arc<dyn apeireth_plugin::preference::PreferenceStore> = preferences.clone();
    let self_assessments: Arc<dyn apeireth_plugin::self_assessment::SelfAssessmentStore> =
        self_assessments.clone();
    let council = if council_enabled {
        // 2026-09-08: Council 需要真 LlmFactory (镜像 trait 经 llm_mirror_adapter 桥).
        // 优先 openai-compatible (DeepSeek 等), 回退 MiniMax, 最后 Noop (0 装).
        Some(Arc::new(build_council_from_env()))
    } else {
        None
    };
    let scoped_memory: Arc<dyn apeireth_memory::ScopedMemoryBackend> = sqlite_backend.clone();
    let reflexion_store: Option<Arc<dyn apeireth_memory::reflexion::ReflexionStore>> =
        reflexion_enabled.then(|| -> Arc<dyn apeireth_memory::reflexion::ReflexionStore> {
            Arc::new(apeireth_memory::reflexion::FileReflexionStore::new(
                reflexion_store_root_from_env(),
            ))
        });
    let backends = CognitiveBackends {
        memory: Some(memory.clone()),
        memory_governance: Some(memory_governance),
        wiki: Some(wiki),
        graph: Some(graph),
        associations: Some(associations),
        preferences: Some(preferences),
        self_assessments: Some(self_assessments),
        council,
        workspace_root: std::env::current_dir().ok(),
        scoped_memory: Some(scoped_memory),
        embedding_provider: embedding_provider_from_env()
            .map_err(|error| format!("embedding provider config invalid: {error}"))?,
        access_history: Some(access_history),
        memory_extractor: None,
        memory_materializer: None,
        typed_recall: typed_recall_enabled.then(|| Arc::clone(&typed_source)),
        typed_recall_identity: typed_recall_enabled.then_some(typed_identity),
        typed_sink: Some(typed_sink),
        reflexion_store,
    };
    let modules =
        apeireth_runtime_assembly::ProductionCognitiveModules::build(config, backends, clock)
            .map_err(|error| error.to_string())?;
    Ok((modules, memory))
}

/// 构造 Council 后端（2026-09-08 用户旋钮批）：优先 OpenAI-compatible
/// （DeepSeek 等, env 配置时）→ 回退 MiniMax → 最后 Noop（0 装, advisors 会
/// 显式 NotImplemented 而非静默）。
fn build_council_from_env() -> apeireth_orchestration::Council {
    use apeireth_orchestration::Council;
    use apeireth_plugin::llm_factory::LlmFactory as PluginLlmFactory;

    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env()
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "deepseek-v4-flash".to_string());
            let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
                Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory)));
            return Council::with_factory(mirror, model);
        }
    }
    // MiniMax 回退仅在显式配了 MiniMax key 时成立 (0 装: 不凭空造一个会在
    // 调用时才失败的 council).
    if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) = apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env() {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "MiniMax-M3".to_string());
            let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
                Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory)));
            return Council::with_factory(mirror, model);
        }
    }
    Council::default_llm()
}

fn cognitive_db_path() -> PathBuf {
    std::env::var(COGNITIVE_DB_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".apeireth/cognitive.sqlite3"))
}

/// Execute one CLI turn directly through [`Runtime::execute_outcome`].
pub async fn execute_canonical_cli_turn(
    runtime: &Runtime,
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<SessionId>,
) -> Result<CanonicalCliTurn, String> {
    let session = session.unwrap_or_else(SessionId::new);
    let prompt = prompt.into();
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: session.to_string(),
        trace_id: String::new(),
        user_request: prompt.clone(),
        created_at_ms: 0,
    });
    let context = TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent);
    let mut request = TurnRequest::new(session, prompt).with_security_context(context);
    if let Some(model) = model {
        request = request.with_model(model);
    }
    match runtime
        .execute_outcome(request)
        .await
        .map_err(|error| error.to_string())?
    {
        TurnOutcome::Completed(response) => Ok(CanonicalCliTurn::Completed(response)),
        TurnOutcome::PendingApproval(view) => Ok(CanonicalCliTurn::PendingApproval(view)),
    }
}

/// Resolve a pending approval through the canonical runtime API.
pub async fn resolve_canonical_cli_approval(
    runtime: &Runtime,
    session: SessionId,
    approval: ApprovalId,
    decision: ApprovalDecision,
) -> Result<ApprovalResolution, String> {
    runtime
        .resolve_approval(session, approval, decision)
        .await
        .map_err(|error| error.to_string())
}

/// Bootstrap and execute the canonical CLI chat path.
pub async fn dispatch_canonical_chat(
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<String>,
) -> Result<CanonicalCliTurn, String> {
    let session = session
        .map(|id| id.parse::<SessionId>().map_err(|error| error.to_string()))
        .transpose()?;
    let (runtime, observer) = build_canonical_runtime_from_env_with_observability().await?;
    let result = execute_canonical_cli_turn(&runtime, prompt, model, session).await;
    observer.flush().await;
    result
}

/// Bootstrap and resolve a pending approval on the production session store.
pub async fn dispatch_canonical_approval(
    session: String,
    approval: String,
    decision: ApprovalDecision,
) -> Result<ApprovalResolution, String> {
    let session = session
        .parse::<SessionId>()
        .map_err(|error| error.to_string())?;
    let approval = approval
        .parse::<ApprovalId>()
        .map_err(|error| error.to_string())?;
    let (runtime, observer) = build_canonical_runtime_from_env_with_observability().await?;
    let result = resolve_canonical_cli_approval(&runtime, session, approval, decision).await;
    observer.flush().await;
    result
}

/// Start the HTTP Gateway backed by one long-lived canonical runtime.
/// Blocks until the server exits.
/// **W2 §4.1 dreaming 触发载体 (决策点 D2)**: 显式命令 `apeireth dream` = 显式授权。
///
/// 语义: 取近 N 条 episodes 为梦境素材 (显式 `--session`; 未给 = 空素材, 走引擎的
/// "常规认知结构自整定"路径) → 6 阶段做梦循环 (LLM 思考器 + 确定性降级链) →
/// 苏醒写日记 (`<data>/diary`, source = `dream`)。**无旋钮**: 做梦只在此命令下
/// 发生 (W2 验收门命令变体: 显式命令即授权 + 默认不自动跑)。
pub async fn dispatch_dream(
    session: Option<String>,
    limit: usize,
    date: Option<String>,
) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_memory::diary::FileDiaryStore;
    use apeireth_memory::dream_wiring::{dream_and_journal, DeterministicMetaThinker};
    use apeireth_memory::dreaming::{DreamEngine, DreamEngineConfig};
    use apeireth_memory::meta_thinking::MetaThinker;
    use apeireth_memory::procedural::InMemoryProceduralStore;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::{FallbackMetaThinker, LlmMetaThinker};
    use apeireth_storage::SqliteConnectionPool;

    let path = cognitive_db_path();
    let pool = Arc::new(
        SqliteConnectionPool::open(&path)
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let recent: Vec<String> = match &session {
        Some(session_id) => backend
            .recent_episodes(session_id, limit)
            .map_err(|error| format!("read recent episodes failed: {error}"))?
            .into_iter()
            .map(|episode| format!("[{}] {}", episode.role, episode.content))
            .collect(),
        None => Vec::new(),
    };

    // 决策点 D1: LLM 思考器 + 确定性降级链; 未配 LLM = 纯规则 (不造假可用)。
    let thinker: Arc<dyn MetaThinker> = match build_dream_llm_thinker() {
        Ok(llm) => Arc::new(FallbackMetaThinker::new(llm, DeterministicMetaThinker)),
        Err(_) => Arc::new(DeterministicMetaThinker),
    };

    let mut engine = DreamEngine::new(DreamEngineConfig::default());
    let procedural = InMemoryProceduralStore::new(1000);
    let diary_root = default_panel_data_dir().join("diary");
    let diary = FileDiaryStore::new(diary_root.clone());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    // 决策点 D3: DreamReport 落日记 (引擎"苏醒阶段写入日记"本意)。
    let date = date.unwrap_or_else(|| {
        chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    });
    let report = dream_and_journal(
        &mut engine,
        &recent,
        thinker.as_ref(),
        &procedural,
        &diary,
        &date,
        now_ms,
    )?;
    Ok(format!(
        "dream {} 完成 (素材 {} 条, 日记 {}): 6 阶段循环结束\n\n{}",
        report.dream_id,
        recent.len(),
        diary_root.display(),
        report.to_markdown()
    ))
}

/// 从 env 找真 LLM 工厂 (openai-compatible 优先, MiniMax 回退) + 首个模型 id。
/// 都未配 → Err (0 装: 不造"调用时才失败"的假可用)。
fn llm_factory_from_env(
) -> Result<(Arc<dyn apeireth_plugin::llm_factory::LlmFactory>, String), String> {
    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env()
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "deepseek-v4-flash".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) = apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env() {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "MiniMax-M3".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    Err("no llm factory configured".to_string())
}

/// **council 旋钮** (2026-10-10 拍板, 默认 3): `APEIRETH_COUNCIL_ADVISORS=N` (1-7) =
/// 裁决顾问数 (规范序取前 N: Safety/Performance/Philosophy/History/Strategy/
/// Ethics/Legal, Safety 恒首位); `APEIRETH_COUNCIL_TIMEOUT_MS` = 单顾问超时
/// (默认 30000, 台账 #46 实测: 思考型模型 reasoning 余量下 10s 不够)。
fn council_config_from_env() -> apeireth_orchestration::CouncilConfig {
    use apeireth_orchestration::CouncilConfig;
    use std::time::Duration;
    let max_advisors = std::env::var("APEIRETH_COUNCIL_ADVISORS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map(|n| n.clamp(1, 7))
        .unwrap_or(3);
    let per_advisor_timeout = std::env::var("APEIRETH_COUNCIL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(30_000));
    CouncilConfig {
        max_advisors,
        per_advisor_timeout,
        ..CouncilConfig::default()
    }
}

/// **council 决策环节 D —— 显式咨询** (2026-10-10 拍板): `apeireth council "<议题>"`。
///
/// council 语义改造 (主人 2026-10-10): 从"每轮评审器"改为"决策环节顾问"。触发位点:
/// A 升级/部署批准、B 高危操作 (L3+)、C 待裁冲突批量裁决 (空闲时) —— 随对应模块
/// (upgrade_cycle ratification / 洋葱 L3-L5 / Nightwatch 守夜人) 接入; D = 本命令
/// (显式授权, 免旋钮)。一般轮次不再常开 council (深度档 = 深思直答)。
pub async fn dispatch_council(topic: String) -> Result<String, String> {
    use apeireth_core::kernel::SessionId;
    use apeireth_orchestration::{Council, CouncilVerdict, Proposal};

    let (factory, model) = llm_factory_from_env()?;
    let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
        Arc::new(apeireth_plugin::MirrorLlmFactory::new(factory));
    let council =
        Council::with_factory(mirror, model.clone()).with_config(council_config_from_env());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let proposal = Proposal {
        id: format!("council-{now_ms}"),
        proposer: "cli-explicit".to_string(),
        payload: serde_json::json!({ "topic": topic, "kind": "explicit_consultation" }),
        submitted_at: now_ms,
        session_id: SessionId::new(),
    };
    let verdict = council.decide(&proposal).await;
    let summary = match &verdict {
        CouncilVerdict::Approved => "Approved (多数 Allow, 无强反对)".to_string(),
        CouncilVerdict::Vetoed { by, reason } => format!("Vetoed (by: {by:?}): {reason}"),
        CouncilVerdict::DeferToHuman { reason } => format!("DeferToHuman: {reason}"),
    };
    Ok(format!(
        "council 裁决 — 议题「{topic}」 (顾问 {} 个, 模型 {model})\n  {summary}\n\n完整判定: {verdict:?}",
        council.advisors().len()
    ))
}

/// **W1 §2.4 全局旋钮** (2026-10-10, 默认**开**): shell 沙箱 (工作区限定 + 断网)。
/// `APEIRETH_SHELL_SANDBOX=0` = 显式裸跑 (本机全权, 自担风险)。
fn shell_sandbox_enabled_from_env() -> bool {
    !std::env::var("APEIRETH_SHELL_SANDBOX")
        .ok()
        .is_some_and(|v| v.trim() == "0")
}

/// 构造做梦 LLM 思考器 (工厂未配则 Err, 调用方降级确定性思考器)。
fn build_dream_llm_thinker() -> Result<apeireth_runtime_assembly::canonical::LlmMetaThinker, String>
{
    let (factory, model) =
        llm_factory_from_env().map_err(|e| format!("{e} (dream uses deterministic thinker)"))?;
    Ok(apeireth_runtime_assembly::canonical::LlmMetaThinker::new(
        factory, model,
    ))
}

pub async fn dispatch_gateway_serve(port: u16) -> Result<String, String> {
    dispatch_gateway_serve_on("127.0.0.1", port).await
}

/// Start the HTTP Gateway on an explicitly selected bind address.
///
/// Loopback is the safe default. Binding a non-loopback address is an
/// intentional operator decision and is called out before the listener starts.
pub async fn dispatch_gateway_serve_on(bind: &str, port: u16) -> Result<String, String> {
    let (runtime, sessions, memory, policy, guard_hook) =
        build_canonical_runtime_with_sessions_from_env().await?;
    let runtime = Arc::new(runtime);
    let governance = Arc::new(
        apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
            .map_err(|error| format!("memory governance store open failed: {error}"))?,
    );
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    let panel = Arc::new(
        crate::gateway_panels::CliPanelData::new_with_runtime(
            Arc::clone(&runtime),
            sessions,
            memory,
            governance,
            policy,
            enable_local_read_tools,
            default_panel_data_dir(),
        )
        .with_guard(guard_hook),
    );
    let mut services = crate::gateway_panels::gateway_services(panel);
    // Wire the hot-reload api_key writer to the same keyring backend the
    // runtime's credential resolver reads (or None on the env-resolver path).
    services.credentials = crate::keyring_bootstrap::build_keyring_credential_writer();
    let address = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .map_err(|error| format!("bind {address} failed: {error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;
    let url = format!("http://{local_addr}");

    if local_addr.ip().is_loopback() {
        eprintln!("canonical gateway started at {url}");
    } else {
        eprintln!(
            "WARNING: canonical gateway is exposed on non-loopback address {local_addr}; use this only on a trusted network"
        );
    }
    apeireth_gateway::serve_canonical_with_services(listener, runtime, services)
        .await
        .map_err(|error| format!("gateway server failed: {error}"))?;

    Ok(format!("server stopped at {url}"))
}

/// Data directory for panel archives. `APEIRETH_DATA_DIR` overrides; the
/// default is `~/.apeireth` (same place as the keyring and session dbs).
fn default_panel_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth")
}
