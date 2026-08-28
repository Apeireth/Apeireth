//! Canonical Apeireth command-line entry points.
//!
//! The CLI is a thin adapter: it bootstraps one canonical runtime, constructs
//! a canonical turn request, and delegates execution to `Runtime::execute`.

// v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring / EncryptedFile backend
// (per `v2.0.0-rc-roadmap.md` §3 RC-9: "keyring 真正接到 EnvCredentialResolver 之前").
// 0 装诚实: 4 backend + KeyringSelector alpha 已真 impl; 本模块只做 bootstrap 集成.
pub mod keyring_bootstrap;

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::{
    CredentialDisclosureHook, GovernancePipeline, Permission, PermissionGovernanceHook,
    PermissionPolicy, PromptInjectionHook,
};
use apeireth_runtime::canonical::{Runtime, TurnRequest, TurnResponse};

/// One persistent SQLite database is shared by the cognitive backends.
/// `APEIRETH_COGNITIVE_DB` may override the path; Judge remains opt-in.
const COGNITIVE_DB_ENV: &str = "APEIRETH_COGNITIVE_DB";
const COGNITIVE_JUDGE_ENV: &str = "APEIRETH_COGNITIVE_JUDGE";
const COGNITIVE_COUNCIL_ENV: &str = "APEIRETH_COGNITIVE_COUNCIL";

/// Enables the local filesystem and search tools in the production policy.
pub const ENABLE_LOCAL_READ_TOOLS_ENV: &str = "APEIRETH_ENABLE_LOCAL_READ_TOOLS";

/// Build the production governance policy from an explicit local-read choice.
///
/// The explicit boolean keeps the policy deterministic and easy to test. The
/// environment-facing wrapper is [`build_production_governance_from_env`].
/// Authorization is deliberately the first hook: a later content-risk hook
/// must never turn an unauthorized capability into an approval request.
pub fn build_production_governance(enable_local_read_tools: bool) -> GovernancePipeline {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    if enable_local_read_tools {
        policy.grant(Permission::ExecuteTool("tool.filesystem".to_string()));
        policy.grant(Permission::ExecuteTool("tool.search".to_string()));
    }

    GovernancePipeline::new()
        .with(Arc::new(PermissionGovernanceHook::new(policy)))
        .with(Arc::new(CredentialDisclosureHook::new()))
        .with(Arc::new(PromptInjectionHook::new()))
}

/// Build the production governance policy using the process environment.
///
/// Production is default-deny for capability execution. Only the exact value
/// `1` enables the two local read tools; shell, fetch, and unknown capabilities
/// remain denied even if a future plugin registers them.
pub fn build_production_governance_from_env() -> GovernancePipeline {
    let enable_local_read_tools = std::env::var(ENABLE_LOCAL_READ_TOOLS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    build_production_governance(enable_local_read_tools)
}

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
///
/// Provider implementations are injected as plugins. Credentials are resolved
/// at execution time, so neither the runtime nor a provider stores API keys.
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
    use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
    use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;

    let configured_model = std::env::var("APEIRETH_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty());

    let clock: Arc<dyn apeireth_core::kernel::Clock> = apeireth_core::kernel::system_clock();
    let mut builder = Runtime::builder().with_clock(Arc::clone(&clock));
    // P-arch (2026-08-27) + v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring
    // 优先用 keyring (设 APEIRETH_KEYRING_BACKEND env), fallback 到 EnvCredentialResolver
    // (alpha 0 装路径, 0 行为变化). 详见 `keyring_bootstrap` 模块.
    let resolver: Arc<dyn apeireth_plugin::CredentialResolver> =
        keyring_bootstrap::build_keyring_resolver();
    builder = builder.with_credentials(resolver);
    builder = builder.with_governance(Arc::new(build_production_governance_from_env()));

    // The CLI is the composition root. Gateway reuses this function, while
    // SDK remains an HTTP client and does not host a second Runtime.
    let cognitive = build_cognitive_modules_from_env(Arc::clone(&clock)).await?;
    builder = cognitive.register_into(builder);

    let workspace_root = std::env::current_dir()
        .map_err(|error| format!("canonical runtime bootstrap failed: current_dir: {error}"))?;
    builder = builder.with_plugin(Arc::new(apeireth_tools_canonical::BuiltinToolsPlugin::new(
        workspace_root,
    )));

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

    builder
        .build()
        .await
        .map_err(|error| format!("canonical runtime bootstrap failed: {error}"))
}

async fn build_cognitive_modules_from_env(
    clock: Arc<dyn apeireth_core::kernel::Clock>,
) -> Result<apeireth_runtime::canonical::ProductionCognitiveModules, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_memory::{
        experience_store_sqlite::SQLiteExperienceStore,
        preference_store_sqlite::SQLitePreferenceStore,
        self_assessment_store_sqlite::SQLiteSelfAssessmentStore,
    };
    use apeireth_runtime::canonical::{CognitiveBackends, CognitiveModuleConfig, JudgeConfig};
    use apeireth_storage::{SqliteConnectionPool, StorageError};

    let path = std::env::var(COGNITIVE_DB_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".apeireth/cognitive.sqlite3".into());
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
    let migration_pool = Arc::clone(&pool);
    migration_pool
        .write(|conn| {
            apeireth_memory::run_migrations(conn).map_err(|error| StorageError::Migration {
                version: 0,
                name: "cognitive_memory",
                message: error.to_string(),
            })
        })
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

    let judge_enabled = std::env::var(COGNITIVE_JUDGE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let council_enabled = std::env::var(COGNITIVE_COUNCIL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let config = CognitiveModuleConfig {
        judge: JudgeConfig {
            enabled: judge_enabled,
            ..JudgeConfig::default()
        },
        council: council_enabled,
        ..CognitiveModuleConfig::default()
    };
    let memory: Arc<dyn apeireth_plugin::memory_backend::MemoryBackend> =
        Arc::new(SqliteBackend::from_arc(Arc::clone(&pool)));
    let wiki: Arc<dyn apeireth_plugin::experience::WikiEntryStore> = experience.clone();
    let graph: Arc<dyn apeireth_plugin::experience::KnowledgeGraphStore> = experience.clone();
    let associations: Arc<dyn apeireth_plugin::experience::AssociationStore> = experience.clone();
    let preferences: Arc<dyn apeireth_plugin::preference::PreferenceStore> = preferences.clone();
    let self_assessments: Arc<dyn apeireth_plugin::self_assessment::SelfAssessmentStore> =
        self_assessments.clone();
    let council = if council_enabled {
        Some(Arc::new(apeireth_orchestration::Council::default_llm()))
    } else {
        None
    };
    let backends = CognitiveBackends {
        memory: Some(memory),
        wiki: Some(wiki),
        graph: Some(graph),
        associations: Some(associations),
        preferences: Some(preferences),
        self_assessments: Some(self_assessments),
        council,
    };
    apeireth_runtime::canonical::ProductionCognitiveModules::build(config, backends, clock)
        .map_err(|error| error.to_string())
}

/// Execute one CLI turn directly through [`Runtime::execute`].
pub async fn execute_canonical_cli_turn(
    runtime: &Runtime,
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<SessionId>,
) -> Result<TurnResponse, String> {
    let mut request = TurnRequest::new(session.unwrap_or_else(SessionId::new), prompt);
    if let Some(model) = model {
        request = request.with_model(model);
    }
    runtime
        .execute(request)
        .await
        .map_err(|error| error.to_string())
}

/// Bootstrap and execute the canonical CLI chat path.
pub async fn dispatch_canonical_chat(
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<String>,
) -> Result<TurnResponse, String> {
    let session = session
        .map(|id| id.parse::<SessionId>().map_err(|error| error.to_string()))
        .transpose()?;
    let runtime = build_canonical_runtime_from_env().await?;
    execute_canonical_cli_turn(&runtime, prompt, model, session).await
}

/// Start the HTTP Gateway backed by one long-lived canonical runtime.
/// Blocks until the server exits.
pub async fn dispatch_gateway_serve(port: u16) -> Result<String, String> {
    let runtime = Arc::new(build_canonical_runtime_from_env().await?);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|error| format!("bind 0.0.0.0:{port} failed: {error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;
    let url = format!("http://{local_addr}");

    eprintln!("canonical gateway started at {url}");
    apeireth_gateway::serve_canonical(listener, runtime)
        .await
        .map_err(|error| format!("gateway server failed: {error}"))?;

    Ok(format!("server stopped at {url}"))
}
