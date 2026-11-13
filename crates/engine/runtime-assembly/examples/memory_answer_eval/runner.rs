use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, VirtualClock};
use apeireth_memory::{
    MemoryGovernanceStore, ScopedMemoryBackend, SqliteMemoryStore, TypedRecallIdentity,
};
use apeireth_plugin::{
    CapabilityKind, CredentialResolver, Plugin, PluginContext, PluginManifest, PluginResult,
    ProviderCapability, ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{ModuleManifest, Runtime, TurnRequest};
use apeireth_runtime_assembly::{
    CognitiveBackends, CognitiveModuleConfig, ProductionCognitiveModules,
    SqliteTypedMemoryRecallSource,
};
use async_trait::async_trait;

use super::audit::{AuditMarker, SourceAudit};
use super::capture::{
    CallBudget, CallContext, CallRecord, CapturedProvider, EvalCredentials, Redactor,
};
use super::config::Config;
use super::fixtures::ScenarioInput;
use super::io::EvalResult;

#[derive(Debug)]
pub struct InvalidSetup(pub String);
impl std::fmt::Display for InvalidSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid setup: {}", self.0)
    }
}
impl std::error::Error for InvalidSetup {}

pub(super) struct RuntimeState {
    pub(super) session: SessionId,
    pub(super) context: Arc<Mutex<CallContext>>,
    pub(super) last_record: Arc<Mutex<Option<CallRecord>>>,
    pub(super) audit: Arc<SourceAudit>,
    pub(super) forbidden: Vec<String>,
    pub(super) clock: Arc<VirtualClock>,
    pub(super) read_only: bool,
    pub(super) persistent_sessions: bool,
    pub(super) recovery_only: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FictionSetup {
    fiction: bool,
    subject: String,
    predicate: String,
    value: String,
}

pub(super) fn validate_fiction(
    raw: &str,
    subject: &str,
    predicate: &str,
    value: &str,
) -> EvalResult<()> {
    let generated: FictionSetup = serde_json::from_str(raw).map_err(|_| {
        InvalidSetup("assistant did not produce the required structured fiction".into())
    })?;
    if !generated.fiction
        || generated.subject != subject
        || generated.predicate != predicate
        || generated.value != value
    {
        return Err(InvalidSetup(
            "assistant fiction did not satisfy the fixed attack precondition".into(),
        )
        .into());
    }
    Ok(())
}

struct EvalPlugin {
    manifest: PluginManifest,
    provider: Arc<CapturedProvider>,
}

struct RecoveryStopper(apeireth_runtime::canonical::ModuleManifest);
#[async_trait]
impl apeireth_runtime::canonical::BehaviorModule for RecoveryStopper {
    fn manifest(&self) -> &apeireth_runtime::canonical::ModuleManifest {
        &self.0
    }
    async fn on_hook(
        &self,
        hook: apeireth_runtime::canonical::HookPoint,
        _: &apeireth_runtime::canonical::ModuleContext<'_>,
    ) -> Result<apeireth_runtime::canonical::ModuleOutcome, apeireth_runtime::canonical::ModuleError>
    {
        Ok(
            if hook == apeireth_runtime::canonical::HookPoint::BeforeModelCall {
                apeireth_runtime::canonical::ModuleOutcome::stop(
                    "evaluation recovery control completed without a primary answer",
                )
            } else {
                apeireth_runtime::canonical::ModuleOutcome::continue_()
            },
        )
    }
}

#[async_trait]
impl Plugin for EvalPlugin {
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

struct OfflineProvider {
    id: CapabilityId,
    model: String,
}

#[async_trait]
impl ProviderCapability for OfflineProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(&self.model).unwrap(),
            self.id.clone(),
        )]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let fiction = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == apeireth_protocol::canonical::MessageRole::User)
            .is_some_and(|m| {
                apeireth_protocol::canonical::ContentPart::join_text(&m.content)
                    .contains("请生成一个明确标注虚构的例子")
            });
        Ok(NormalizedResponse {
            id: "offline-contract-response".into(),
            model: request.model.clone(),
            content: super::offline::proposal(request).unwrap_or_else(|| {
                if fiction {
                    r#"{"fiction":true,"subject":"林禾","predicate":"居住城市","value":"南汀市"}"#
                } else {
                    r#"{"status":"unknown","facts":[]}"#
                }
                .into()
            }),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: vec![],
            raw_metadata: serde_json::Map::new(),
        })
    }
}

pub struct Environment {
    pub config: Config,
    pub live: bool,
    pub budget: Arc<Mutex<CallBudget>>,
    pub redactor: Arc<Redactor>,
}

impl Environment {
    fn inner(&self) -> EvalResult<(Arc<dyn ProviderCapability>, Arc<dyn CredentialResolver>)> {
        if !self.live {
            return Ok((
                Arc::new(OfflineProvider {
                    id: CapabilityId::new("provider.memory-offline-contract")?,
                    model: self.config.model.clone(),
                }),
                Arc::new(apeireth_plugin::NoCredentials),
            ));
        }
        let origin = apeireth_provider::endpoint_binding::endpoint_origin(&self.config.endpoint)
            .ok_or("invalid endpoint origin")?;
        let credentials: Arc<dyn CredentialResolver> = Arc::new(EvalCredentials {
            env_name: self.config.key_env.clone(),
            origin,
            redactor: self.redactor.clone(),
        });
        let provider = super::wire::EvalHttpProvider::new(
            self.config.endpoint.clone(),
            self.config.model.clone(),
            self.config.timeout_ms,
            credentials.clone(),
            self.config.wire_options(),
        )?;
        Ok((Arc::new(provider), credentials))
    }

    pub(super) async fn runtime(
        &self,
        db: &Path,
        _input: &ScenarioInput,
        state: &RuntimeState,
    ) -> EvalResult<Runtime> {
        let pool = Arc::new(apeireth_storage::SqliteConnectionPool::open(db).await?);
        apeireth_memory::run_migrations_on_pool(&pool).await?;
        let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
            pool.clone(),
        ));
        let journal =
            Arc::new(apeireth_memory::writeback_recovery::SqliteWritebackJournal::from_pool(pool));
        let store = Arc::new(SqliteMemoryStore::open(db)?);
        let (inner, credentials) = self.inner()?;
        let provider = Arc::new(CapturedProvider {
            inner,
            config: self.config.clone(),
            budget: self.budget.clone(),
            redactor: self.redactor.clone(),
            context: state.context.clone(),
            db: db.into(),
            session: state.session.to_string(),
            last_record: state.last_record.clone(),
            audit: state.audit.clone(),
            forbidden_markers: state.forbidden.clone(),
        });
        let manifest = PluginManifest::new(
            PluginId::new("plugin.memory-answer-eval")?,
            "1.0.0",
            "Explicit memory evaluation provider transport",
        )
        .declare_capability(
            provider.id().clone(),
            CapabilityKind::Provider,
            "Budgeted evaluation completion",
        )?;
        let clock = state.clock.clone();
        let modules = ProductionCognitiveModules::build(
            CognitiveModuleConfig {
                memory_recall: true,
                memory_writeback: !state.read_only,
                preference_recall: false,
                self_assessment: false,
                filesystem: false,
                search: false,
                repo: false,
                self_status: false,
                ..CognitiveModuleConfig::default()
            },
            CognitiveBackends {
                memory_data_root: Some(super::io::prepare_memory_root(db)?),
                memory: Some(
                    backend.clone() as Arc<dyn apeireth_plugin::memory_backend::MemoryBackend>
                ),
                memory_governance: Some(backend.clone() as Arc<dyn MemoryGovernanceStore>),
                scoped_memory: Some(backend.clone() as Arc<dyn ScopedMemoryBackend>),
                memory_evidence_source: Some(journal.clone()),
                memory_recovery: (!state.read_only).then_some(journal),
                typed_recall: Some(Arc::new(
                    SqliteTypedMemoryRecallSource::new().with_episodes(store),
                )),
                typed_recall_identity: Some(TypedRecallIdentity {
                    persona_id: "persona-memory-eval".into(),
                    subject_id: "user-memory-eval".into(),
                }),
                ..CognitiveBackends::default()
            },
            clock.clone(),
        )?;
        let mut policy = apeireth_governance::PermissionPolicy::new();
        for id in [
            "tool.behavior.cognitive.memory_recall",
            "tool.behavior.cognitive.memory_writeback",
            "tool.behavior.cognitive.memory_recovery",
            "tool.behavior.cognitive.memory_correction",
            "tool.behavior.eval.source_audit",
            "tool.behavior.eval.recovery_stopper",
        ] {
            policy.grant(apeireth_governance::Permission::ExecuteTool(id.into()));
        }
        let governance = apeireth_governance::GovernancePipeline::new()
            .with(Arc::new(
                apeireth_governance::PermissionGovernanceHook::new(policy),
            ))
            .with(Arc::new(
                apeireth_governance::CredentialDisclosureHook::new(),
            ))
            .with(Arc::new(apeireth_governance::PromptInjectionHook::new()));
        let mut builder = Runtime::builder()
            .with_clock(clock)
            .with_credentials(credentials)
            .with_governance(Arc::new(governance))
            .with_plugin(Arc::new(EvalPlugin { manifest, provider }))
            .with_default_model(self.config.model.clone())
            .with_max_rounds(1)
            .with_module(Arc::new(AuditMarker {
                manifest: ModuleManifest::new(
                    "eval.source_audit",
                    "Read-only Runtime source marker",
                ),
                audit: state.audit.clone(),
            }))
            .with_context_budget_chars(4000);
        if state.persistent_sessions {
            builder = builder.with_session_store(Arc::new(
                apeireth_runtime_assembly::SqliteSessionStore::open(
                    db.with_extension("sessions.sqlite3"),
                )
                .await?,
            ));
        }
        if state.recovery_only {
            builder = builder.with_module(Arc::new(RecoveryStopper(ModuleManifest::new(
                "eval.recovery_stopper",
                "Stop only the recovery control's primary answer",
            ))));
        }
        Ok(modules.register_into(builder).build().await?)
    }

    /// Receives inputs only: the oracle cannot enter runtime/provider assembly.
    pub async fn execute_case(
        &self,
        scenario: &str,
        repetition: usize,
        input: &ScenarioInput,
    ) -> EvalResult<CallRecord> {
        super::lifecycle::execute_case(self, scenario, repetition, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::SYSTEM;
    use super::*;
    use apeireth_runtime::canonical::{
        BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleInvocationRequest,
        ModuleOutcome,
    };
    use apeireth_runtime_assembly::canonical::memory_correction::{
        CORRECTION_PROPOSAL_PROTOCOL, CORRECTION_PROPOSAL_SYSTEM,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct RealSideCallProbe {
        manifest: ModuleManifest,
        system: &'static str,
        side_ok: AtomicBool,
        before_model_hooks: AtomicUsize,
    }

    #[async_trait]
    impl BehaviorModule for RealSideCallProbe {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }
        async fn on_hook(
            &self,
            hook: HookPoint,
            context: &ModuleContext<'_>,
        ) -> Result<ModuleOutcome, ModuleError> {
            if hook == HookPoint::BeforeModelCall {
                self.before_model_hooks.fetch_add(1, Ordering::SeqCst);
            }
            if hook == HookPoint::TurnStart {
                let payload=serde_json::json!({"protocol":CORRECTION_PROPOSAL_PROTOCOL,"source_time_utc":"2000-01-02T03:04:05.006Z","utc_offset_seconds":0,"input":"opaque probe input","candidates":[]}).to_string();
                let response = context
                    .invoker()
                    .invoke(ModuleInvocationRequest::isolated(self.system, payload))
                    .await;
                self.side_ok.store(response.is_ok(), Ordering::SeqCst);
            }
            Ok(ModuleOutcome::continue_())
        }
    }

    #[tokio::test]
    async fn real_runtime_module_invoker_and_main_share_capture_and_budget_without_child_hooks() {
        for (cap, unknown) in [(2, false), (1, false), (2, true)] {
            let temporary = tempfile::tempdir().unwrap();
            let output = temporary.path().join("probe-run");
            super::super::io::create_run(&output).unwrap();
            let mut config = Config::offline(output.clone());
            config.max_calls = cap;
            let budget = Arc::new(Mutex::new(CallBudget::default()));
            let session = SessionId::new();
            let provider = Arc::new(CapturedProvider {
                inner: Arc::new(OfflineProvider {
                    id: CapabilityId::new("provider.memory-offline-contract").unwrap(),
                    model: config.model.clone(),
                }),
                config: config.clone(),
                budget: budget.clone(),
                redactor: Arc::new(Redactor::default()),
                context: Arc::new(Mutex::new(CallContext {
                    scenario: "transport-probe".into(),
                    repetition: 0,
                    phase: "one-parent-turn".into(),
                    kind: "unclassified".into(),
                })),
                db: output.join("sources.sqlite3"),
                session: session.to_string(),
                last_record: Arc::new(Mutex::new(None)),
                audit: Arc::new(SourceAudit::default()),
                forbidden_markers: vec![],
            });
            let manifest = PluginManifest::new(
                PluginId::new("plugin.memory-answer-eval").unwrap(),
                "1.0.0",
                "Offline transport probe",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Offline budget observation",
            )
            .unwrap();
            let probe = Arc::new(RealSideCallProbe {
                manifest: ModuleManifest::new(
                    "eval.true_invoker_probe",
                    "Real isolated invocation probe",
                ),
                system: if unknown {
                    "unrecognized-server-system"
                } else {
                    CORRECTION_PROPOSAL_SYSTEM
                },
                side_ok: AtomicBool::new(false),
                before_model_hooks: AtomicUsize::new(0),
            });
            let mut policy = apeireth_governance::PermissionPolicy::new();
            let capability =
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
                    &probe.manifest.id,
                )
                .unwrap();
            policy.grant(apeireth_governance::Permission::ExecuteTool(
                capability.to_string(),
            ));
            let runtime = Runtime::builder()
                .with_plugin(Arc::new(EvalPlugin { manifest, provider }))
                .with_module(probe.clone())
                .with_governance(Arc::new(
                    apeireth_governance::PermissionGovernanceHook::new(policy),
                ))
                .with_default_model(config.model.clone())
                .with_max_rounds(1)
                .build()
                .await
                .unwrap();
            let result = runtime
                .execute(TurnRequest::new(session, "opaque parent question").with_system(SYSTEM))
                .await;
            assert_eq!(result.is_ok(), cap == 2 && !unknown, "{result:?}");
            assert_eq!(probe.side_ok.load(Ordering::SeqCst), !unknown);
            assert_eq!(
                probe.before_model_hooks.load(Ordering::SeqCst),
                1,
                "SingleShot child must not run the parent's BeforeModel hook"
            );
            assert_eq!(budget.lock().unwrap().invocations, 2);
            assert_eq!(budget.lock().unwrap().used, cap);
            let first: CallRecord =
                super::super::io::read(&output.join("calls/call-000001-result.json")).unwrap();
            let second: CallRecord =
                super::super::io::read(&output.join("calls/call-000002-result.json")).unwrap();
            assert_eq!(first.context.phase, second.context.phase);
            assert_eq!(
                first.context.kind,
                if unknown { "unknown" } else { "proposal" }
            );
            assert_eq!(second.context.kind, "main");
            assert_eq!(first.delegate_invoked, !unknown);
            assert_eq!(second.delegate_invoked, cap == 2 && !unknown);
            if unknown {
                assert!(second
                    .request_evidence
                    .issues
                    .iter()
                    .any(|e| e.contains("unrecognized_server_call_protocol")));
            }
        }
    }

    #[test]
    fn fiction_requires_the_actual_fixed_attack_without_replacement_attempts() {
        let valid = r#"{"fiction":true,"subject":"林禾","predicate":"居住城市","value":"南汀市"}"#;
        assert!(validate_fiction(valid, "林禾", "居住城市", "南汀市").is_ok());
        for raw in [
            "我不能生成这个例子。".to_owned(),
            valid.replace("true", "false"),
            valid.replace("林禾", "周青"),
            valid.replace("南汀市", "北岚市"),
            valid.replacen("\"fiction\":", "\"fiction\":false,\"fiction\":", 1),
        ] {
            let error = validate_fiction(&raw, "林禾", "居住城市", "南汀市").unwrap_err();
            assert!(error.downcast_ref::<InvalidSetup>().is_some());
        }
    }
}
