//! Runtime assembly and corpus lifecycle.
//!
//! The corpus is written through the **real** Runtime: every setup turn is a
//! committed user turn, so the memory plane really stores it and the recall
//! really reads it back. Nothing here fabricates a delivery.
//!
//! ⭐ Two provider paths share this harness:
//! * the default one is a deterministic offline fixture (see `reader`) — it
//!   answers from the delivered context and never touches the network;
//! * the opt-in one is the repository's own OpenAI-compatible capability (see
//!   `live`), which resolves the credential per turn from an environment
//!   variable. The harness only ever sees the **name** of that variable.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use apeireth_core::kernel::{CapabilityId, SessionId, VirtualClock};
use apeireth_governance::PermissionPolicy;
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{ModuleManifest, Runtime, TurnRequest, TurnResponse};
use apeireth_runtime_assembly::{
    CognitiveBackends, CognitiveModuleConfig, ProductionCognitiveModules,
    SqliteTypedMemoryRecallSource,
};
use async_trait::async_trait;

use super::artifacts::Result;
use super::config::Config;
use super::evidence::{Audit, Marker};
use super::reader::OfflineReader;
use super::schema::{InputBundle, Operation, TurnSpec, ACK_SYSTEM};

/// Module id of the Runtime-side input observer.
pub const AUDIT_MODULE_ID: &str = "eval.memory_longterm_audit";

/// The fixed instant the corpus is written at.
pub const CORPUS_INSTANT: &str = "2026-11-01T08:00:00Z";

/// One provider call the run produced.
#[derive(Clone, Debug)]
pub struct CallRecord {
    /// The request as the provider saw it (this is the delivered context).
    pub request: NormalizedRequest,
    /// The usage the provider reported.
    pub usage: NormalizedUsage,
    /// How long the provider call itself took.
    pub elapsed_us: u64,
}

/// Every provider request the run produced, in order.
#[derive(Default)]
pub struct Calls {
    records: Mutex<Vec<CallRecord>>,
}

impl Calls {
    pub fn push(&self, request: &NormalizedRequest, usage: &NormalizedUsage, elapsed_us: u64) {
        self.records.lock().unwrap().push(CallRecord {
            request: request.clone(),
            usage: usage.clone(),
            elapsed_us,
        });
    }

    pub fn last(&self) -> Option<NormalizedRequest> {
        self.records
            .lock()
            .unwrap()
            .last()
            .map(|record| record.request.clone())
    }

    /// The last call, with its usage and its provider-side duration.
    pub fn last_record(&self) -> Option<CallRecord> {
        self.records.lock().unwrap().last().cloned()
    }

    /// Every call of the run.
    pub fn records(&self) -> Vec<CallRecord> {
        self.records.lock().unwrap().clone()
    }

    pub fn len(&self) -> usize {
        self.records.lock().unwrap().len()
    }
}

/// Provider wrapper that records the request it received.
pub struct RecordingProvider {
    inner: Arc<dyn ProviderCapability>,
    calls: Arc<Calls>,
}

impl RecordingProvider {
    pub fn new(inner: Arc<dyn ProviderCapability>, calls: Arc<Calls>) -> Self {
        Self { inner, calls }
    }
}

#[async_trait]
impl ProviderCapability for RecordingProvider {
    fn id(&self) -> &CapabilityId {
        self.inner.id()
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        self.inner.models()
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> std::result::Result<NormalizedResponse, ProviderError> {
        let started = Instant::now();
        let outcome = self.inner.complete(request).await;
        let elapsed_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        match &outcome {
            Ok(response) => self.calls.push(request, &response.usage, elapsed_us),
            Err(_) => self
                .calls
                .push(request, &NormalizedUsage::default(), elapsed_us),
        }
        outcome
    }
}

/// The plugin that exposes the recording provider.
struct EvalPlugin {
    manifest: PluginManifest,
    provider: Arc<RecordingProvider>,
}

#[async_trait]
impl Plugin for EvalPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _context: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone()]
    }
}

/// One written setup turn.
#[derive(Clone, Debug)]
pub struct TurnRecord {
    pub id: String,
    pub session: String,
    pub request: String,
    pub answer_sha256: String,
}

/// One withdrawal.
#[derive(Clone, Debug)]
pub struct ForgetRecord {
    pub turn: String,
    pub episode_id: String,
    pub revision_before: i64,
    pub revision_after: i64,
    pub status_after: String,
}

/// The lifecycle report of one corpus write.
#[derive(Clone, Debug, Default)]
pub struct CorpusReport {
    pub turns: Vec<TurnRecord>,
    pub forgets: Vec<ForgetRecord>,
    pub stored_records: usize,
}

/// Which provider path a Runtime turn is dispatched to.
///
/// ⭐ The corpus write (the setup turns) never calls a real model: those turns
/// only need a deterministic acknowledgement. The question turns are the ones
/// that may be served by the live capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderChoice {
    /// The path that answers the evaluation questions.
    Answer,
    /// The path that writes the corpus into the store.
    Corpus,
}

/// The model id of the deterministic corpus-acknowledgement fixture.
pub const CORPUS_PROVIDER_MODEL: &str = "memory-longterm-corpus-ack";

/// Derive a runtime session id from a logical session name.
///
/// ⭐ The derivation is stable: the same name always maps to the same
/// session, so a runtime rebuild (recovery path, pressure phase) keeps talking
/// to the same scope.
fn session_id_for(name: &str) -> Result<SessionId> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(name.as_bytes());
    let mut text = String::with_capacity(36);
    for (index, byte) in digest[..16].iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(text.parse()?)
}

/// The evaluation harness: one store, one policy, one clock.
pub struct Harness {
    pub db: PathBuf,
    pub config: Config,
    /// The provider calls of the question path.
    pub calls: Arc<Calls>,
    /// The provider calls of the corpus path.
    pub corpus_calls: Arc<Calls>,
    pub audit: Arc<Audit>,
    pub clock: Arc<VirtualClock>,
    pub policy: PermissionPolicy,
    pub sessions: BTreeMap<String, SessionId>,
    /// The provider the runtime dispatches to: the offline fixture or the
    /// live capability.
    pub provider: Arc<dyn ProviderCapability>,
    /// The manifest this evaluation declares for the provider.
    pub provider_manifest: PluginManifest,
    /// The canonical model id the runtime routes to.
    pub provider_model: String,
    /// A non-secret label of the provider path (used in the artifacts).
    pub provider_label: String,
    /// The **key names** of the vendor-private request fields in play.
    ///
    /// ⭐ Names only: the values are request-shape knobs, and the artifacts
    /// record which ones were in play without copying the operator's input.
    pub vendor_fields: Vec<String>,
    /// When set, the Runtime keeps a durable session store. The official
    /// recovery path reads the committed turns back from it, so the fault
    /// phase needs it; the evaluation path deliberately does **not** use it
    /// (a cold query must not see a live transcript).
    pub persistent_sessions: bool,
}

impl Harness {
    /// Build the harness. The policy is the one that will be in force for the
    /// evaluation turns; it is stored so the evidence pass can read it back.
    pub fn new(db: &Path, config: Config, input: &InputBundle) -> Result<Self> {
        let instant =
            chrono::DateTime::parse_from_rfc3339(CORPUS_INSTANT)?.with_timezone(&chrono::Utc);
        let mut policy = PermissionPolicy::new();
        for id in [
            "tool.behavior.cognitive.memory_recall",
            "tool.behavior.cognitive.memory_writeback",
            "tool.behavior.cognitive.memory_recovery",
            "tool.behavior.cognitive.memory_correction",
            &format!("tool.behavior.{AUDIT_MODULE_ID}"),
        ] {
            policy.grant(apeireth_governance::Permission::ExecuteTool(id.into()));
        }
        let mut sessions = BTreeMap::new();
        for session in &input.sessions {
            sessions.insert(session.id.clone(), session_id_for(&session.id)?);
        }
        // ⭐ The provider path is decided by the configuration: an endpoint
        // means the live capability, no endpoint means the deterministic
        // offline fixture.
        let (provider, provider_manifest, provider_model, provider_label, vendor_fields) =
            if config.is_live() {
                let live = super::live::LiveProvider::build(&config)?;
                let model = live.model().to_string();
                let label = format!("live endpoint {}", live.endpoint());
                let fields = live.vendor_fields().to_vec();
                (live.capability(), live.manifest(), model, label, fields)
            } else {
                let reader = OfflineReader::new(&config.model)?;
                let model = config.model.clone();
                let manifest = reader.manifest();
                (
                    Arc::new(reader) as Arc<dyn ProviderCapability>,
                    manifest,
                    model,
                    "offline fixture provider.memory-longterm.offline-reader".to_string(),
                    Vec::new(),
                )
            };
        Ok(Self {
            db: db.to_path_buf(),
            provider,
            provider_manifest,
            provider_model,
            provider_label,
            vendor_fields,
            config,
            calls: Arc::new(Calls::default()),
            corpus_calls: Arc::new(Calls::default()),
            audit: Arc::new(Audit::new()),
            clock: Arc::new(VirtualClock::new(instant)),
            policy,
            sessions,
            persistent_sessions: false,
        })
    }

    /// The provider, manifest and model of one path.
    ///
    /// ⭐ The corpus path is always the deterministic acknowledgement fixture:
    /// the setup turns only need a stable ack, and the live capability must
    /// never be billed for them.
    fn provider_for(
        &self,
        choice: ProviderChoice,
    ) -> Result<(
        Arc<dyn ProviderCapability>,
        PluginManifest,
        String,
        Arc<Calls>,
    )> {
        match choice {
            ProviderChoice::Answer => Ok((
                self.provider.clone(),
                self.provider_manifest.clone(),
                self.provider_model.clone(),
                self.calls.clone(),
            )),
            ProviderChoice::Corpus => {
                let model = CORPUS_PROVIDER_MODEL.to_string();
                let reader = OfflineReader::new(&model)?;
                let manifest = reader.manifest();
                Ok((
                    Arc::new(reader) as Arc<dyn ProviderCapability>,
                    manifest,
                    model,
                    self.corpus_calls.clone(),
                ))
            }
        }
    }

    /// Turn the durable session store on or off.
    pub fn set_persistent_sessions(&mut self, value: bool) {
        self.persistent_sessions = value;
    }

    /// The runtime session id of a logical session.
    ///
    /// ⭐ The id is **derived from the name**, not random: the same logical
    /// session must land on the same runtime session across runtime
    /// rebuilds (the recovery path and the pressure phase build a new
    /// Runtime per operation). A name that was not declared in the corpus is
    /// created on demand with the same derivation.
    pub fn session(&self, name: &str) -> Result<SessionId> {
        if let Some(id) = self.sessions.get(name) {
            return Ok(*id);
        }
        session_id_for(name)
    }

    /// The partition string of a logical session.
    pub fn partition(&self, name: &str) -> Result<String> {
        Ok(self.session(name)?.to_string())
    }

    /// Build a Runtime over the same store.
    ///
    /// `read_only` turns the writeback path off: the query turn then cannot
    /// change the store, so the query proves that the answer comes from the
    /// stored history and not from a live transcript.
    pub async fn runtime(&self, read_only: bool, choice: ProviderChoice) -> Result<Runtime> {
        let db = self.db.clone();
        let pool = Arc::new(apeireth_storage::SqliteConnectionPool::open(&db).await?);
        apeireth_memory::run_migrations_on_pool(&pool).await?;
        let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
            pool.clone(),
        ));
        let journal =
            Arc::new(apeireth_memory::writeback_recovery::SqliteWritebackJournal::from_pool(pool));
        let store = Arc::new(SqliteMemoryStore::open(&db)?);
        let (inner, manifest, provider_model, calls) = self.provider_for(choice)?;
        let provider = Arc::new(RecordingProvider::new(inner, calls));
        let modules = ProductionCognitiveModules::build(
            CognitiveModuleConfig {
                memory_recall: true,
                memory_writeback: !read_only,
                preference_recall: false,
                self_assessment: false,
                filesystem: false,
                search: false,
                repo: false,
                self_status: false,
                ..CognitiveModuleConfig::default()
            },
            CognitiveBackends {
                memory_data_root: Some(db.with_extension("isolated-memory-data")),
                memory: Some(
                    backend.clone() as Arc<dyn apeireth_plugin::memory_backend::MemoryBackend>
                ),
                memory_governance: Some(backend.clone() as Arc<dyn MemoryGovernanceStore>),
                scoped_memory: Some(
                    backend.clone() as Arc<dyn apeireth_memory::ScopedMemoryBackend>
                ),
                memory_evidence_source: Some(journal.clone()),
                memory_recovery: (!read_only).then_some(journal),
                typed_recall: Some(Arc::new(
                    SqliteTypedMemoryRecallSource::new().with_episodes(store),
                )),
                typed_recall_identity: Some(apeireth_memory::TypedRecallIdentity {
                    persona_id: "persona-memory-longterm-eval".into(),
                    subject_id: "user-memory-longterm-eval".into(),
                }),
                ..CognitiveBackends::default()
            },
            self.clock.clone(),
        )?;
        let governance = apeireth_governance::GovernancePipeline::new()
            .with(Arc::new(
                apeireth_governance::PermissionGovernanceHook::new(self.policy.clone()),
            ))
            .with(Arc::new(
                apeireth_governance::CredentialDisclosureHook::new(),
            ))
            .with(Arc::new(apeireth_governance::PromptInjectionHook::new()));
        let builder = Runtime::builder()
            .with_clock(self.clock.clone())
            .with_credentials(Arc::new(apeireth_plugin::NoCredentials))
            .with_governance(Arc::new(governance))
            .with_plugin(Arc::new(EvalPlugin { manifest, provider }))
            .with_default_model(provider_model)
            .with_max_rounds(1)
            .with_module(Arc::new(Marker {
                manifest: ModuleManifest::new(
                    AUDIT_MODULE_ID,
                    "Runtime-side observation of committed user input",
                ),
                audit: self.audit.clone(),
            }))
            .with_context_budget_chars(self.config.context_chars);
        let mut builder = builder;
        if self.persistent_sessions {
            builder = builder.with_session_store(Arc::new(
                apeireth_runtime_assembly::SqliteSessionStore::open(
                    db.with_extension("sessions.sqlite3"),
                )
                .await?,
            ));
        }
        Ok(modules.register_into(builder).build().await?)
    }

    /// Write every setup turn through the Runtime, then apply the operations.
    pub async fn write_corpus(&self, input: &InputBundle) -> Result<CorpusReport> {
        let mut report = CorpusReport::default();
        let runtime = self.runtime(false, ProviderChoice::Corpus).await?;
        for turn in &input.turns {
            let session = self.session(&turn.session)?;
            let response = runtime
                .execute(TurnRequest::new(session, turn.text.clone()).with_system(ACK_SYSTEM))
                .await?;
            report.turns.push(TurnRecord {
                id: turn.id.clone(),
                session: turn.session.clone(),
                request: response.request.to_string(),
                answer_sha256: super::artifacts::hash(&response.text),
            });
        }
        drop(runtime);
        for operation in &input.operations {
            if let Operation::Forget { turn, reason } = operation {
                report.forgets.push(self.forget_turn(input, turn, reason)?);
            }
        }
        let store = SqliteMemoryStore::open(&self.db)?;
        for session in &input.sessions {
            let rows = EpisodeStore::recent_episodes(&store, &self.partition(&session.id)?, 1000)?;
            report.stored_records += rows.len();
        }
        Ok(report)
    }

    /// Withdraw the record that carries one turn.
    pub fn forget_turn(
        &self,
        input: &InputBundle,
        turn_id: &str,
        reason: &str,
    ) -> Result<ForgetRecord> {
        let turn: &TurnSpec = input
            .turns
            .iter()
            .find(|turn| turn.id == turn_id)
            .ok_or_else(|| format!("unknown turn {turn_id}"))?;
        let store = SqliteMemoryStore::open(&self.db)?;
        let partition = self.partition(&turn.session)?;
        let rows = EpisodeStore::recent_episodes(&store, &partition, 1000)?;
        let episode_id = rows
            .iter()
            .filter(|row| row.role == "user")
            .find(|row| row.content.trim() == turn.text.trim())
            .map(|row| row.id.clone())
            .ok_or_else(|| format!("no stored record carries turn {turn_id}"))?;
        let current = store
            .get_governed(&episode_id)?
            .ok_or_else(|| format!("governed row {episode_id} is missing"))?;
        let revision_before = current.revision;
        let forgotten = store.forget_episode(&episode_id, Some(reason), current.revision)?;
        Ok(ForgetRecord {
            turn: turn_id.to_string(),
            episode_id,
            revision_before,
            revision_after: forgotten.revision,
            status_after: forgotten.status.as_str().to_string(),
        })
    }

    /// Ask one question through a cold Runtime.
    pub async fn ask(&self, session_name: &str, question: &str) -> Result<TurnResponse> {
        let runtime = self.runtime(true, ProviderChoice::Answer).await?;
        let session = self.session(session_name)?;
        let response = runtime
            .execute(
                TurnRequest::new(session, question.to_string())
                    .with_system(super::schema::READER_SYSTEM),
            )
            .await?;
        drop(runtime);
        Ok(response)
    }

    /// Ask one question through a Runtime that may write back. This is the
    /// full-chain path: the writeback and the recovery drain are live.
    pub async fn ask_writable(&self, session_name: &str, question: &str) -> Result<TurnResponse> {
        let runtime = self.runtime(false, ProviderChoice::Answer).await?;
        let session = self.session(session_name)?;
        let response = runtime
            .execute(
                TurnRequest::new(session, question.to_string())
                    .with_system(super::schema::READER_SYSTEM),
            )
            .await?;
        drop(runtime);
        Ok(response)
    }
}
