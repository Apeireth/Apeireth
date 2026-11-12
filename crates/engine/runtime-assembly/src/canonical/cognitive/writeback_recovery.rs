//! Recovery is driven by its caller, never by a detached worker. A complete
//! frozen plan is durable before the first target store is touched.
use super::*;
use apeireth_memory::writeback_recovery::{
    operation_id, payload_hash, PreparedWritebackPlan, RecoveryJob, RecoveryStatus,
    SqliteWritebackJournal, WritebackOperation,
};
use apeireth_memory::{
    CommitmentCandidate, CommitmentSignal, MemoryError, MemoryMaterializationOutcome,
    MemoryTypedCandidate, PersonaProfileDelta, RelationCandidate,
};
use apeireth_runtime::canonical::module::{CommittedInputOrigin, CommittedTurnSnapshot};

const SOURCE_PAGE: usize = 16;
const MAX_PAGES: usize = 4;
const MAX_JOBS: usize = 8;
const MAX_OPERATIONS: usize = 128;

#[derive(Default)]
struct CaptureTyped(Mutex<Vec<MemoryTypedCandidate>>);
#[async_trait]
impl MemoryTypedMaterializationSink for CaptureTyped {
    async fn materialize_commitment(
        &self,
        candidate: &CommitmentCandidate,
        signal: &CommitmentSignal,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        lock_or_recover(&self.0).push(MemoryTypedCandidate::Commitment {
            candidate: candidate.clone(),
            signal: signal.clone(),
        });
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "captured for a durable plan; not applied".into(),
        })
    }
    async fn materialize_persona(
        &self,
        delta: &PersonaProfileDelta,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        lock_or_recover(&self.0).push(MemoryTypedCandidate::Persona {
            delta: delta.clone(),
        });
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "captured for a durable plan; not applied".into(),
        })
    }
    async fn materialize_relation(
        &self,
        candidate: &RelationCandidate,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        lock_or_recover(&self.0).push(MemoryTypedCandidate::Relation {
            candidate: candidate.clone(),
        });
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "captured for a durable plan; not applied".into(),
        })
    }
}

fn source_hash(snapshot: &CommittedTurnSnapshot) -> Result<String, MemoryError> {
    let mut stable = snapshot.clone();
    // Historical epochs remain audit evidence; a new Host lease authorizes
    // recovery and must not change the identity of the committed source.
    stable.input.authorization_epoch = None;
    payload_hash(&stable)
}

fn provenance(snapshot: &CommittedTurnSnapshot) -> apeireth_memory::MemoryProvenance {
    apeireth_memory::MemoryProvenance {
        source: "cognitive.memory_writeback".into(),
        source_session: Some(snapshot.input.session.to_string()),
        source_trace: Some(snapshot.input.trace.to_string()),
        source_request: Some(snapshot.input.request.to_string()),
    }
}

impl MemoryWritebackModule {
    /// TurnStart must settle accepted corrections before a new recall can
    /// present an older value. Ordinary AfterTurn backlog does not block it.
    pub fn ensure_corrections_settled(&self, session: &SessionId) -> Result<(), MemoryError> {
        let Some(journal) = &self.recovery_journal else {
            return Ok(());
        };
        let namespace = self.recovery_namespace(session);
        if lock_or_recover(&self.recovery_source_state).get(&namespace) != Some(&true) {
            return Err(MemoryError::Invalid(
                "committed correction sources are not caught up".into(),
            ));
        }
        if journal.has_pending_correction(&namespace)? {
            return Err(MemoryError::Invalid(
                "an accepted memory correction is still pending".into(),
            ));
        }
        Ok(())
    }

    pub fn attach_profile_mappings(
        &self,
        mappings: Arc<crate::profile_memory::ProfileMemoryMappings>,
    ) {
        *lock_or_recover(&self.recovery_mappings) = Some(mappings);
    }

    fn recovery_authorized(
        &self,
        execution: Option<&apeireth_plugin::ToolExecutionContext>,
    ) -> Result<(), MemoryError> {
        if execution.is_some_and(|e| e.cancellation.is_cancelled() || Instant::now() >= e.deadline)
        {
            return Err(MemoryError::Invalid("recovery execution expired".into()));
        }
        match (
            &self.profile_partition,
            execution.and_then(|e| e.principal.as_ref()),
        ) {
            (None, None) => Ok(()),
            (Some(partition), Some(principal))
                if *partition == super::super::memory_module::namespace_key(principal) =>
            {
                let mappings = lock_or_recover(&self.recovery_mappings)
                    .clone()
                    .ok_or_else(|| MemoryError::Invalid("recovery mappings unavailable".into()))?;
                let lease = mappings
                    .lease(execution.expect("authenticated principal"))
                    .ok_or_else(|| MemoryError::Invalid("recovery authority revoked".into()))?;
                if !lease.authority.is_active()
                    || !lease.memory.as_ref().is_some_and(|access| {
                        access.writable_private
                            && self
                                .coordinator
                                .as_ref()
                                .is_some_and(|coord| Arc::ptr_eq(coord, &access.coordinator))
                    })
                {
                    return Err(MemoryError::Invalid(
                        "recovery store is not writable".into(),
                    ));
                }
                Ok(())
            }
            _ => Err(MemoryError::Invalid(
                "recovery execution owner mismatch".into(),
            )),
        }
    }

    pub fn with_recovery_journal(mut self, journal: Arc<SqliteWritebackJournal>) -> Self {
        self.recovery_journal = Some(journal);
        self
    }

    pub fn recovery_status(
        &self,
        session: &SessionId,
    ) -> Result<Option<RecoveryStatus>, MemoryError> {
        if let Some(error) = lock_or_recover(&self.recovery_error).clone() {
            return Err(MemoryError::Other(error));
        }
        self.recovery_journal
            .as_ref()
            .map(|j| j.status(&self.recovery_namespace(session)))
            .transpose()
    }

    fn recovery_namespace(&self, session: &SessionId) -> String {
        self.profile_partition
            .clone()
            .unwrap_or_else(|| session_text(session))
    }

    fn recovery_failed(&self, phase: &str) {
        self.metrics.warning();
        *lock_or_recover(&self.recovery_error) = Some(phase.to_owned());
    }

    /// The separate TurnStart module and AfterTurn writer share this exact
    /// instance, its serialization gate, and its official journal.
    pub async fn drain_recovery_authenticated(
        &self,
        ctx: &ModuleContext<'_>,
        execution: &apeireth_plugin::ToolExecutionContext,
    ) -> Result<ModuleOutcome, ModuleError> {
        if let Some(principal) = &execution.principal {
            if self.profile_partition.as_deref()
                != Some(super::super::memory_module::namespace_key(principal).as_str())
                || self.coordinator.is_none()
            {
                return Err(ModuleError::Message(
                    "memory recovery has no private Profile partition".into(),
                ));
            }
        }
        self.drain_recovery(ctx, Some(execution)).await
    }

    pub async fn drain_recovery(
        &self,
        ctx: &ModuleContext<'_>,
        execution: Option<&apeireth_plugin::ToolExecutionContext>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let Some(journal) = &self.recovery_journal else {
            return Ok(ModuleOutcome::continue_());
        };
        let _owned = self.recovery_lock.lock().await;
        let namespace = self.recovery_namespace(ctx.session_id);
        lock_or_recover(&self.recovery_source_state).insert(namespace.clone(), false);
        *lock_or_recover(&self.recovery_error) = None;
        if self.memory.kind() != apeireth_plugin::memory_backend::BackendKind::Sqlite {
            self.recovery_failed("memory backend has no durable recovery adapter");
            return Ok(ModuleOutcome::continue_());
        }
        let result: Result<(), MemoryError> = async {
            self.recovery_authorized(execution)?;
            for _ in 0..MAX_PAGES {
                if execution.is_some_and(|e| e.cancellation.is_cancelled()) {
                    return Ok(());
                }
                let cursor = journal.cursor(&namespace)?;
                let page = ctx
                    .invoker
                    .committed_turns(cursor.clone(), SOURCE_PAGE)
                    .await
                    .map_err(|_| {
                        MemoryError::Other("committed recovery source unavailable".into())
                    })?;
                let empty = page.turns.is_empty() && page.prepared_operations.is_empty();
                let mut jobs = Vec::new();
                for snapshot in page.turns {
                    self.check_recovery_owner(ctx, execution, &snapshot)?;
                    // Task evidence stays in its canonical SessionStore. A
                    // skipped projection must not copy its body into this DB.
                    if snapshot.input.origin == CommittedInputOrigin::AgentTask {
                        continue;
                    }
                    let hash = payload_hash(&(
                        "memory-writeback-v1",
                        &namespace,
                        snapshot.input.session,
                        snapshot.input.request,
                    ))?;
                    jobs.push(RecoveryJob {
                        id: format!("writeback-{hash}"),
                        namespace: namespace.clone(),
                        source_session: snapshot.input.session.to_string(),
                        source_request: snapshot.input.request.to_string(),
                        source_hash: source_hash(&snapshot)?,
                        source: serde_json::to_value(&snapshot)?,
                        created_ms: snapshot.committed_at.epoch_millis(),
                        plan: None,
                        next_operation: 0,
                        state: "pending".into(),
                        attempts: 0,
                        last_error: None,
                    });
                }
                for intent in page.prepared_operations {
                    if intent.module_id != "cognitive.memory_correction" {
                        continue;
                    }
                    let source = CommittedTurnSnapshot {
                        input: intent.input.clone(),
                        response_id: String::new(),
                        response_content: String::new(),
                        committed_at: intent.prepared_at,
                        cursor: String::new(),
                    };
                    self.check_recovery_owner(ctx, execution, &source)?;
                    let plan: apeireth_memory::automatic_correction::AutomaticCorrectionPlan =
                        serde_json::from_value(intent.payload.clone())?;
                    if intent.input.origin != CommittedInputOrigin::DirectUser
                        || plan.namespace != namespace
                        || plan.operation_id != intent.operation_id
                        || payload_hash(&intent.payload)? != intent.payload_hash
                        || payload_hash(&plan)? != intent.payload_hash
                        || plan.provenance.source_session.as_deref()
                            != Some(intent.input.session.to_string().as_str())
                        || plan.provenance.source_request.as_deref()
                            != Some(intent.input.request.to_string().as_str())
                        || plan.provenance.source_trace.as_deref()
                            != Some(intent.input.trace.to_string().as_str())
                        || plan.evidence.content
                            != intent
                                .input
                                .messages
                                .iter()
                                .filter(|m| m.role == MessageRole::User)
                                .map(|m| ContentPart::join_text(&m.content))
                                .collect::<Vec<_>>()
                                .join("\n")
                    {
                        return Err(MemoryError::Invalid(
                            "prepared correction evidence mismatch".into(),
                        ));
                    }
                    jobs.push(RecoveryJob {
                        id: format!("correction/{}", plan.operation_id),
                        namespace: namespace.clone(),
                        source_session: intent.input.session.to_string(),
                        source_request: intent.input.request.to_string(),
                        source_hash: intent.payload_hash,
                        source: serde_json::to_value(&plan)?,
                        created_ms: plan.recorded_at_ms,
                        plan: None,
                        next_operation: 0,
                        state: "pending".into(),
                        attempts: 0,
                        last_error: None,
                    });
                }
                self.recovery_authorized(execution)?;
                journal.register_page(&namespace, cursor, &jobs, page.next_cursor)?;
                if empty {
                    lock_or_recover(&self.recovery_source_state).insert(namespace.clone(), true);
                    break;
                }
            }
            let mut operations_left = MAX_OPERATIONS;
            for mut job in journal.pending(&namespace, MAX_JOBS)? {
                if execution.is_some_and(|e| e.cancellation.is_cancelled()) {
                    break;
                }
                if !self.revalidate_recovery_source(ctx, &job).await? {
                    journal.suppress(&job.id)?;
                    continue;
                }
                if job.plan.is_none() {
                    match self.prepare_recovery_plan(&job).await {
                        Ok(plan) => {
                            self.recovery_authorized(execution)?;
                            journal.save_plan(&job.id, &plan)?;
                            job.plan = Some(plan);
                        }
                        Err(_) => {
                            journal.failed(
                                &job.id,
                                "prepare",
                                self.clock.now().timestamp_millis(),
                            )?;
                            self.metrics.warning();
                            continue;
                        }
                    }
                }
                let plan = job.plan.as_ref().expect("prepared plan");
                if plan.operations.is_empty() {
                    journal.finish_empty(&job.id, self.clock.now().timestamp_millis())?;
                    continue;
                }
                for (index, operation) in
                    plan.operations.iter().enumerate().skip(job.next_operation)
                {
                    if operations_left == 0
                        || execution.is_some_and(|e| e.cancellation.is_cancelled())
                    {
                        return Ok(());
                    }
                    operations_left -= 1;
                    if !journal.is_pending(&job.id)? {
                        break;
                    }
                    if !self.revalidate_recovery_source(ctx, &job).await?
                        || self.recovery_source_forgotten(plan)?
                    {
                        journal.suppress(&job.id)?;
                        break;
                    }
                    let id = operation_id(&job.id, index);
                    self.recovery_authorized(execution)?;
                    match self
                        .apply_recovery_operation(journal, &job, operation, &id, execution)
                        .await
                    {
                        Ok(()) => journal.advance(
                            &job.id,
                            index,
                            plan.operations.len(),
                            self.clock.now().timestamp_millis(),
                        )?,
                        Err(_) => {
                            journal.failed(
                                &job.id,
                                &format!("operation/{index}"),
                                self.clock.now().timestamp_millis(),
                            )?;
                            self.metrics.warning();
                            break;
                        }
                    }
                }
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            lock_or_recover(&self.recovery_source_state).insert(namespace, false);
            self.recovery_failed("memory recovery source or journal unavailable");
        }
        Ok(ModuleOutcome::continue_())
    }

    fn check_recovery_owner(
        &self,
        ctx: &ModuleContext<'_>,
        execution: Option<&apeireth_plugin::ToolExecutionContext>,
        snapshot: &CommittedTurnSnapshot,
    ) -> Result<(), MemoryError> {
        match (&self.profile_partition, &snapshot.input.principal) {
            (Some(partition), Some(principal))
                if super::super::memory_module::namespace_key(principal) == *partition =>
            {
                if execution
                    .and_then(|e| e.principal.as_ref())
                    .is_some_and(|current| current != principal)
                {
                    return Err(MemoryError::Invalid("recovery principal mismatch".into()));
                }
            }
            (None, None) if snapshot.input.session == *ctx.session_id => {}
            _ => return Err(MemoryError::Invalid("recovery owner mismatch".into())),
        }
        Ok(())
    }

    async fn revalidate_recovery_source(
        &self,
        ctx: &ModuleContext<'_>,
        job: &RecoveryJob,
    ) -> Result<bool, MemoryError> {
        let session = job
            .source_session
            .parse()
            .map_err(|_| MemoryError::Invalid("invalid recovery source session".into()))?;
        let request = job
            .source_request
            .parse()
            .map_err(|_| MemoryError::Invalid("invalid recovery source request".into()))?;
        let Some(current) = ctx
            .invoker
            .committed_input_for(session, request)
            .await
            .map_err(|_| MemoryError::Other("recovery source authorization unavailable".into()))?
        else {
            return Ok(false);
        };
        if let Ok(snapshot) = serde_json::from_value::<CommittedTurnSnapshot>(job.source.clone()) {
            let original = &snapshot.input;
            return Ok(current.session == original.session
                && current.request == original.request
                && current.trace == original.trace
                && current.principal == original.principal
                && current.at == original.at
                && current.origin == original.origin
                && current.messages == original.messages);
        }
        Ok(current.origin == CommittedInputOrigin::DirectUser)
    }

    fn recovery_source_forgotten(&self, plan: &PreparedWritebackPlan) -> Result<bool, MemoryError> {
        if let Some(coord) = &self.coordinator {
            for id in &plan.source_episode_ids {
                if coord
                    .governance()
                    .get_governed(id)
                    .map_err(|e| MemoryError::Other(e.to_string()))?
                    .is_some_and(|g| g.status == MemoryGovernanceStatus::Forgotten)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn prepare_recovery_plan(
        &self,
        job: &RecoveryJob,
    ) -> Result<PreparedWritebackPlan, MemoryError> {
        let Ok(snapshot) = serde_json::from_value::<CommittedTurnSnapshot>(job.source.clone())
        else {
            let plan: apeireth_memory::automatic_correction::AutomaticCorrectionPlan =
                serde_json::from_value(job.source.clone())?;
            let mut dependencies = vec![plan.evidence.id.clone()];
            dependencies.extend(plan.targets.iter().map(|t| t.id.clone()));
            return Ok(PreparedWritebackPlan {
                operations: vec![WritebackOperation::Correction(plan)],
                source_episode_ids: dependencies,
            });
        };
        if source_hash(&snapshot)? != job.source_hash {
            return Err(MemoryError::Invalid(
                "frozen writeback source hash changed".into(),
            ));
        }
        if snapshot.input.origin != CommittedInputOrigin::DirectUser {
            return Ok(PreparedWritebackPlan{operations:vec![WritebackOperation::PolicySkipped{reason:"agent-task evidence remains in its canonical session; no personal-memory projection".into()}],source_episode_ids:Vec::new()});
        }
        let namespace = &job.namespace;
        let provenance = provenance(&snapshot);
        let scope = MemoryScope::session(namespace);
        let mut raw = Vec::new();
        if let Some(user) = snapshot
            .input
            .messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
        {
            let content = ContentPart::join_text(&user.content);
            if !content.is_empty() {
                raw.push(Episode {
                    id: format!("{}-user", job.id),
                    timestamp: snapshot.input.at.epoch_millis() / 1000,
                    role: "user".into(),
                    content,
                    session_id: namespace.clone(),
                });
            }
        }
        raw.push(Episode {
            id: format!("{}-assistant", job.id),
            timestamp: snapshot.committed_at.epoch_millis() / 1000,
            role: "assistant".into(),
            content: snapshot.response_content.clone(),
            session_id: namespace.clone(),
        });
        let input = BoundedMemoryInput {
            scope: scope.clone(),
            source_session: provenance.source_session.clone(),
            source_trace: provenance.source_trace.clone(),
            source_request: provenance.source_request.clone(),
            messages: raw
                .iter()
                .filter(|e| {
                    apeireth_memory::extraction::is_direct_memory_statement(&e.role, &e.content)
                })
                .map(|e| apeireth_memory::MemoryExtractionMessage {
                    role: e.role.clone(),
                    content: e.content.clone(),
                })
                .collect(),
            max_messages: 2,
            max_message_chars: 4096,
        };
        let at = job.created_ms / 1000;
        let mut plan = PreparedWritebackPlan {
            operations: raw
                .iter()
                .map(|e| WritebackOperation::Episode {
                    episode: e.clone(),
                    scope: scope.clone(),
                    provenance: provenance.clone(),
                })
                .collect(),
            source_episode_ids: raw.iter().map(|e| e.id.clone()).collect(),
        };
        for mut item in self
            .materializer
            .materialize_episodes(input.clone(), at)
            .await?
        {
            if item.candidate.scope != scope {
                return Err(MemoryError::Invalid(
                    "materialized recovery scope mismatch".into(),
                ));
            }
            item.episode.session_id = namespace.clone();
            plan.source_episode_ids.push(item.episode.id.clone());
            plan.operations.push(WritebackOperation::DerivedEpisode {
                metadata:serde_json::json!({"extraction_class":item.candidate.class,"extraction":item.candidate,"derived_from_episode_id":raw[0].id}),
                fingerprint: episode_fingerprint(&item.episode.role, &item.episode.content),
                episode: item.episode,
                scope: scope.clone(),
                provenance: provenance.clone(),
            });
        }
        if let Some(sink) = &self.typed_sink {
            let capture = CaptureTyped::default();
            self.materializer
                .materialize_typed(input, at, &capture)
                .await?;
            let candidates = lock_or_recover(&capture.0).clone();
            if !candidates.is_empty()
                && (!sink.supports_durable_recovery()
                    || !sink.supports_recovery_journal(
                        self.recovery_journal.as_ref().expect("journal"),
                    ))
            {
                return Err(MemoryError::Invalid(
                    "configured typed sink cannot recover durably".into(),
                ));
            }
            for mut candidate in candidates {
                match &mut candidate {
                    MemoryTypedCandidate::Commitment { candidate, .. } => {
                        if candidate.scope != scope {
                            return Err(MemoryError::Invalid(
                                "typed recovery scope mismatch".into(),
                            ));
                        }
                        candidate.provenance = provenance.clone();
                    }
                    MemoryTypedCandidate::Relation { candidate } => {
                        if candidate.scope != scope {
                            return Err(MemoryError::Invalid(
                                "typed recovery scope mismatch".into(),
                            ));
                        }
                        candidate.provenance = provenance.clone();
                    }
                    MemoryTypedCandidate::Persona { delta } => {
                        delta.provenance = provenance.clone()
                    }
                }
                let mut context = sink.prepare_recovery(&candidate).await?;
                if !context.is_object() {
                    context = serde_json::json!({});
                }
                context["source_episode_id"] = serde_json::json!(raw[0].id);
                plan.operations.push(WritebackOperation::Typed {
                    candidate,
                    at_ms: job.created_ms,
                    context,
                });
            }
        }
        if self.wiki.is_some() && self.graph.is_some() && self.associations.is_some() {
            for episode in raw.iter().filter(|e| {
                apeireth_memory::extraction::is_direct_memory_statement(&e.role, &e.content)
            }) {
                let artifacts = extract_experience(episode).map_err(MemoryError::Other)?;
                plan.operations.extend(
                    artifacts
                        .wiki_entries
                        .into_iter()
                        .map(WritebackOperation::Wiki),
                );
                plan.operations
                    .extend(artifacts.facts.into_iter().map(WritebackOperation::Fact));
                plan.operations
                    .extend(artifacts.links.into_iter().map(WritebackOperation::Link));
                plan.operations
                    .extend(artifacts.associations.into_iter().map(|a| {
                        WritebackOperation::Association {
                            from: a.from_entity,
                            to: a.to_entity,
                            episode_id: a.source_episode_id,
                        }
                    }));
            }
        }
        let cadence = crate::canonical::self_tuning_wire::consolidation_cadence_turns();
        if self.consolidation
            && self
                .recovery_journal
                .as_ref()
                .expect("journal")
                .turn_ordinal(&job.id)?
                % cadence
                == 0
        {
            let mut episodes = self
                .persona_source_episodes(namespace)
                .map_err(MemoryError::Other)?;
            episodes.extend(raw.iter().cloned());
            episodes.sort_by(|a, b| a.id.cmp(&b.id));
            episodes.dedup_by(|a, b| a.id == b.id);
            plan.source_episode_ids
                .extend(episodes.iter().map(|e| e.id.clone()));
            let mut segments: Vec<_> = episodes
                .iter()
                .filter(|e| matches!(e.role.as_str(), "user" | "assistant" | "tool"))
                .map(|e| format!("{}\n{}", e.role, e.content))
                .collect();
            segments.sort();
            segments.dedup();
            let material = segments.join("\n\n");
            let source = format!("consolidation:{namespace}");
            let skipped = if let Some(gate) = &self.ingest_gate {
                matches!(
                    gate.check(&source, &material).map_err(MemoryError::Other)?,
                    apeireth_memory::GateDecision::SkipDuplicate { .. }
                )
            } else {
                false
            };
            if !skipped {
                if self.coordinator.is_some() {
                    let report = apeireth_memory::consolidation::MemoryConsolidationJob::new()
                        .consolidate(namespace, &episodes);
                    for insight in report.extracted_insights {
                        let episode = Episode {
                            id: format!(
                                "consolidation-{}",
                                payload_hash(&(namespace, &insight, &job.id))?
                            ),
                            timestamp: at,
                            role: "consolidation".into(),
                            content: format!("[记忆整理洞察] {insight}"),
                            session_id: namespace.clone(),
                        };
                        plan.source_episode_ids.push(episode.id.clone());
                        plan.operations.push(WritebackOperation::DerivedEpisode {
                            metadata:serde_json::json!({"extraction_class":"Experience","derived_from_episode_id":raw[0].id}),
                            fingerprint: episode_fingerprint(&episode.role, &episode.content),
                            episode,
                            scope: scope.clone(),
                            provenance: provenance.clone(),
                        });
                    }
                }
                if self.persona.is_some() {
                    plan.operations
                        .push(WritebackOperation::PersonaConsolidation {
                            extract: apeireth_memory::persona_layer::PersonaExtractor
                                .extract(&episodes),
                            at_secs: at,
                        });
                }
            }
            if self.ingest_gate.is_some()
                && !material.is_empty()
                && (skipped || self.coordinator.is_some() || self.persona.is_some())
            {
                plan.operations.push(WritebackOperation::IngestGate {
                    source,
                    content: material,
                    at_ms: job.created_ms,
                    skipped,
                });
            }
        }
        plan.source_episode_ids.sort();
        plan.source_episode_ids.dedup();
        Ok(plan)
    }

    async fn apply_recovery_operation(
        &self,
        journal: &SqliteWritebackJournal,
        job: &RecoveryJob,
        operation: &WritebackOperation,
        id: &str,
        execution: Option<&apeireth_plugin::ToolExecutionContext>,
    ) -> Result<(), MemoryError> {
        match operation {
            WritebackOperation::Correction(plan) => {
                self.coordinator
                    .as_ref()
                    .ok_or_else(|| {
                        MemoryError::Invalid("correction coordinator unavailable".into())
                    })?
                    .governance()
                    .apply_automatic_correction(plan)
                    .map_err(|e| MemoryError::Other(e.to_string()))?;
            }
            WritebackOperation::Episode {
                episode,
                scope,
                provenance,
            } => self.persist_recovery_episode(episode, scope, provenance)?,
            WritebackOperation::DerivedEpisode {
                episode,
                scope,
                provenance,
                fingerprint,
                metadata,
            } => {
                if fingerprint.is_empty()
                    || !journal.dedup_committed(
                        &job.namespace,
                        fingerprint,
                        job.created_ms,
                        self.dedup.window_ms(),
                    )?
                {
                    if let Some(coord) = &self.coordinator {
                        coord.writeback_scoped_episode_with_metadata(
                            episode,
                            scope,
                            provenance,
                            metadata.clone(),
                        )?;
                    } else {
                        let mut metadata = metadata.clone();
                        metadata["scope"] = serde_json::to_value(scope)?;
                        metadata["provenance"] = serde_json::to_value(provenance)?;
                        metadata["layer"] = serde_json::json!("episodic");
                        metadata["content_hash"] = serde_json::json!(
                            apeireth_memory::canonical::vector::content_hash(&episode.content)
                        );
                        self.memory
                            .put_episode_with_metadata(episode, metadata)
                            .map_err(|e| MemoryError::Other(e.to_string()))?;
                    }
                    if !fingerprint.is_empty() {
                        journal.commit_dedup(&job.namespace, fingerprint, job.created_ms)?;
                    }
                } else {
                    self.dedup_skipped.fetch_add(1, Ordering::Relaxed);
                }
            }
            WritebackOperation::Typed {
                candidate,
                at_ms,
                context,
            } => {
                let sink = self.typed_sink.as_ref().ok_or_else(|| {
                    MemoryError::Invalid("typed recovery sink unavailable".into())
                })?;
                let guarded = super::super::memory_typed_sink::ProfileTypedSink {
                    inner: sink.as_ref(),
                    scope: MemoryScope::session(&job.namespace),
                    provenance: match candidate {
                        MemoryTypedCandidate::Commitment { candidate, .. } => {
                            candidate.provenance.clone()
                        }
                        MemoryTypedCandidate::Relation { candidate } => {
                            candidate.provenance.clone()
                        }
                        MemoryTypedCandidate::Persona { delta } => delta.provenance.clone(),
                    },
                    cancellation: execution
                        .map(|e| e.cancellation.clone())
                        .unwrap_or_default(),
                };
                let target: &dyn MemoryTypedMaterializationSink =
                    if self.profile_partition.is_some() {
                        &guarded
                    } else {
                        sink.as_ref()
                    };
                target
                    .materialize_recovery(candidate, id, *at_ms, context)
                    .await?;
            }
            WritebackOperation::Wiki(entry) => self
                .wiki
                .as_ref()
                .ok_or_else(|| MemoryError::Invalid("wiki recovery sink unavailable".into()))?
                .put_wiki(entry)
                .map_err(|e| MemoryError::Other(e.to_string()))?,
            WritebackOperation::Fact(fact) => self
                .graph
                .as_ref()
                .ok_or_else(|| MemoryError::Invalid("graph recovery sink unavailable".into()))?
                .put_fact(fact)
                .map_err(|e| MemoryError::Other(e.to_string()))?,
            WritebackOperation::Link(link) => self
                .graph
                .as_ref()
                .ok_or_else(|| MemoryError::Invalid("graph recovery sink unavailable".into()))?
                .put_link(link)
                .map_err(|e| MemoryError::Other(e.to_string()))?,
            WritebackOperation::Association {
                from,
                to,
                episode_id,
            } => self
                .associations
                .as_ref()
                .ok_or_else(|| {
                    MemoryError::Invalid("association recovery sink unavailable".into())
                })?
                .record_cooccurrence(from, to, episode_id)
                .map_err(|e| MemoryError::Other(e.to_string()))?,
            WritebackOperation::PersonaConsolidation { extract, at_secs } => self
                .persona
                .as_ref()
                .ok_or_else(|| MemoryError::Invalid("persona recovery sink unavailable".into()))?
                .consolidate_once(id, extract, *at_secs)?,
            WritebackOperation::IngestGate {
                source,
                content,
                at_ms,
                skipped,
            } => self
                .ingest_gate
                .as_ref()
                .ok_or_else(|| MemoryError::Invalid("ingest recovery sink unavailable".into()))?
                .record_recovery(id, source, content, *at_ms, *skipped)
                .map_err(MemoryError::Other)?,
            WritebackOperation::PolicySkipped { .. } => {}
        }
        Ok(())
    }

    fn persist_recovery_episode(
        &self,
        episode: &Episode,
        scope: &MemoryScope,
        provenance: &apeireth_memory::MemoryProvenance,
    ) -> Result<(), MemoryError> {
        if let Some(coord) = &self.coordinator {
            coord
                .writeback_scoped_episode(episode, scope, provenance)
                .map(|_| ())
        } else {
            self.memory.put_episode_with_metadata(episode,serde_json::json!({"scope":scope,"provenance":provenance,"layer":"episodic","content_hash":apeireth_memory::canonical::vector::content_hash(&episode.content)})).map(|_|()).map_err(|e|MemoryError::Other(e.to_string()))
        }
    }
}
