//! Owner-bound direct module transport; no Provider reasoning or rival registry.
use super::*;
use crate::canonical::module_messages::*;
use apeireth_core::kernel::TaskId;
use apeireth_plugin::module_contract::ToolSurface;
use apeireth_plugin::module_messages::*;
use apeireth_plugin::ToolCancellation;
use sha2::{Digest, Sha256};
use std::time::Instant;

pub(super) fn bind(module: &BoundModule) -> Arc<dyn ModuleMessages> {
    Arc::new(BoundMessages {
        binding: module.binding.clone(),
        ready: module.ready.clone(),
        owner: module.owner.clone(),
        scope: module.scope.clone(),
        background: tokio::sync::Mutex::new(None),
        delegated: tokio::sync::Mutex::new(None),
    })
}
struct BoundMessages {
    binding: Arc<OnceLock<Weak<Inner>>>,
    ready: Arc<tokio::sync::Notify>,
    owner: AgentOwner,
    scope: ModuleTaskScope,
    background: tokio::sync::Mutex<Option<Arc<dyn AgentInstance>>>,
    delegated: tokio::sync::Mutex<Option<Arc<dyn AgentInstance>>>,
}
fn interrupted(
    code: ModuleMessageErrorCode,
    text: &str,
    started: &std::sync::atomic::AtomicBool,
) -> ModuleMessageError {
    let mut error = denied(code, text);
    error.may_have_executed = started.load(std::sync::atomic::Ordering::Acquire);
    error
}
fn denied(code: ModuleMessageErrorCode, text: &str) -> ModuleMessageError {
    ModuleMessageError::new(code, text)
}
#[async_trait]
impl ModuleMessages for BoundMessages {
    async fn send(&self, request: ModuleMessageRequest) -> ModuleMessageResult {
        self.scope
            .run(self.deliver(request, false))
            .await
            .map_err(|_| {
                denied(
                    ModuleMessageErrorCode::SourceStopped,
                    "source module stopped",
                )
            })?
    }
    async fn invoke_internal(&self, request: InternalToolRequest) -> ModuleMessageResult {
        let AgentOwner::Module { definition, .. } = &self.owner else {
            return Err(denied(
                ModuleMessageErrorCode::Unauthorized,
                "source is not a module",
            ));
        };
        let mut call = ModuleMessageRequest::command(
            definition.clone(),
            request.capability,
            request.arguments,
        );
        call.timeout = request.timeout;
        call.cancellation = request.cancellation;
        call.idempotency_key = request.idempotency_key;
        self.scope
            .run(self.deliver(call, true))
            .await
            .map_err(|_| {
                denied(
                    ModuleMessageErrorCode::SourceStopped,
                    "source module stopped",
                )
            })?
    }
}
impl BoundMessages {
    async fn deliver(&self, request: ModuleMessageRequest, internal: bool) -> ModuleMessageResult {
        let parent = apeireth_plugin::tool_execution::native_tool_context();
        let cancellation = ToolCancellation::joined(
            std::iter::once(request.cancellation.clone())
                .chain(parent.as_ref().map(|context| context.cancellation.clone()))
                .collect(),
        );
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let began = Instant::now();
        let requested = began.checked_add(request.timeout).ok_or_else(|| {
            denied(
                ModuleMessageErrorCode::InvalidEnvelope,
                "message timeout overflow",
            )
        })?;
        let deadline = requested.min(
            parent
                .as_ref()
                .map(|context| context.deadline)
                .unwrap_or(requested),
        );
        let work = self.deliver_bound(
            request,
            internal,
            parent,
            cancellation.clone(),
            started.clone(),
            began,
            deadline,
        );
        tokio::select! {
            biased;
            _=cancelled(&cancellation)=>Err(interrupted(ModuleMessageErrorCode::Cancelled,"message cancelled",&started)),
            result=tokio::time::timeout_at(tokio::time::Instant::from_std(deadline),work)=>result.unwrap_or_else(|_| Err(interrupted(ModuleMessageErrorCode::Timeout,"message deadline exceeded",&started))),
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn deliver_bound(
        &self,
        request: ModuleMessageRequest,
        internal: bool,
        parent: Option<Arc<apeireth_plugin::ToolExecutionContext>>,
        cancellation: ToolCancellation,
        started: Arc<std::sync::atomic::AtomicBool>,
        began: Instant,
        initial_deadline: Instant,
    ) -> ModuleMessageResult {
        if request.timeout.is_zero()
            || request.schema_version != 1
            || request.kind == ModuleMessageKind::Reply
            || !request.payload.is_object()
            || request
                .idempotency_key
                .as_ref()
                .is_some_and(|key| key.is_empty() || key.len() > 128)
        {
            return Err(denied(
                ModuleMessageErrorCode::InvalidEnvelope,
                "invalid message kind/schema/bounds",
            ));
        }
        if !self.scope.is_open() {
            return Err(denied(
                ModuleMessageErrorCode::SourceStopped,
                "source module stopped",
            ));
        }
        let inner = tokio::time::timeout(request.timeout, async {
            loop {
                let ready = self.ready.notified();
                if let Some(inner) = self.binding.get().and_then(Weak::upgrade) {
                    return Ok(inner);
                }
                ready.await;
            }
        })
        .await
        .map_err(|_| {
            denied(
                ModuleMessageErrorCode::Timeout,
                "host binding deadline exceeded",
            )
        })??;
        let pipeline = inner.message_pipeline.clone().ok_or_else(|| {
            denied(
                ModuleMessageErrorCode::Disabled,
                "module messages are disabled",
            )
        })?;
        let deadline = initial_deadline.min(
            began
                .checked_add(pipeline.limits.timeout)
                .unwrap_or(initial_deadline),
        );
        let authorization_ref = parent
            .as_ref()
            .map(|context| {
                format!(
                    "delegated:{}/{}",
                    context.session.map(|id| id.to_string()).unwrap_or_default(),
                    context
                        .authorization
                        .principal_epoch
                        .as_deref()
                        .unwrap_or("unmapped")
                )
            })
            .unwrap_or_else(|| {
                let AgentOwner::Module { definition, .. } = &self.owner else {
                    return String::new();
                };
                inner
                    .policy
                    .message_policy
                    .as_ref()
                    .and_then(|policy| policy.grants.get(definition))
                    .map(|grant| grant.authorization_ref.clone())
                    .unwrap_or_default()
            });
        let work = async {
            let _admission = pipeline
                .admission
                .clone()
                .try_acquire_owned()
                .map_err(|_| {
                    denied(
                        ModuleMessageErrorCode::Backpressure,
                        "message in-flight capacity reached",
                    )
                })?;
            let AgentOwner::Module {
                definition,
                instance,
            } = &self.owner
            else {
                return Err(denied(
                    ModuleMessageErrorCode::Unauthorized,
                    "module identity required",
                ));
            };
            let source = MessageModuleIdentity {
                definition: definition.clone(),
                instance: *instance,
            };
            let grant = inner
                .policy
                .message_policy
                .as_ref()
                .and_then(|p| p.grants.get(definition))
                .cloned()
                .ok_or_else(|| {
                    denied(
                        ModuleMessageErrorCode::Unauthorized,
                        "Host did not grant message routes",
                    )
                })?;
            let agent_grant = inner
                .policy
                .modules
                .get(definition)
                .cloned()
                .ok_or_else(|| {
                    denied(
                        ModuleMessageErrorCode::Unauthorized,
                        "Host did not grant module execution",
                    )
                })?;
            if !super::super::dispatcher::fits_json(
                &request.payload,
                pipeline.limits.max_payload_bytes,
            ) {
                return Err(denied(
                    ModuleMessageErrorCode::InvalidEnvelope,
                    "message payload limit exceeded",
                ));
            }
            if request.kind == ModuleMessageKind::Event && !grant.allow_events
                || request.visibility == MessageVisibility::PublicStatus
                    && !grant.allow_public_status
            {
                return Err(denied(
                    ModuleMessageErrorCode::Unauthorized,
                    "event/projection not granted",
                ));
            }
            let allowed = if internal {
                request.target_module == *definition
                    && grant.internal_tools.contains(&request.capability)
            } else {
                grant
                    .targets
                    .get(&request.target_module)
                    .is_some_and(|ids| ids.contains(&request.capability))
            };
            if !allowed || !agent_grant.tools.contains(&request.capability) {
                return Err(denied(
                    ModuleMessageErrorCode::Unauthorized,
                    "target is outside module authority",
                ));
            }
            let mut tools = grant
                .targets
                .values()
                .flat_map(|ids| ids.iter().cloned())
                .chain(grant.internal_tools.iter().cloned())
                .filter(|id| agent_grant.tools.contains(id))
                .collect::<BTreeSet<_>>();
            let module_tools = tools.clone();
            let security = if let Some(parent) = &parent {
                if parent.module != *definition
                    || parent.module_instance != Some(*instance)
                    || parent.principal != grant.principal
                    || parent.cancellation.is_cancelled()
                    || !parent.module_tasks.is_open()
                {
                    return Err(denied(
                        ModuleMessageErrorCode::Unauthorized,
                        "sender does not own the active delegation",
                    ));
                }
                if parent
                    .tool_scope
                    .as_ref()
                    .is_some_and(|scope| !scope.contains(&request.capability))
                    || parent.session.is_some_and(|session| {
                        !inner.dispatcher.allows(session, &request.capability)
                    })
                {
                    return Err(denied(
                        ModuleMessageErrorCode::Unauthorized,
                        "caller did not delegate target capability",
                    ));
                }
                tools.retain(|id| {
                    parent
                        .tool_scope
                        .as_ref()
                        .is_none_or(|scope| scope.contains(id))
                        && parent
                            .session
                            .is_none_or(|session| inner.dispatcher.allows(session, id))
                });
                match &parent.authorization.host_context {
                    Some(context) => Some(
                        context
                            .downcast_ref::<TurnSecurityContext>()
                            .ok_or_else(|| {
                                denied(
                                    ModuleMessageErrorCode::Unauthorized,
                                    "invalid Host security binding",
                                )
                            })?
                            .clone(),
                    ),
                    None => None,
                }
            } else {
                if !grant.allow_background {
                    return Err(denied(
                        ModuleMessageErrorCode::Unauthorized,
                        "standing background authority not granted",
                    ));
                }
                agent_grant.security.clone()
            };
            let (receiver, contract) = {
                let kernel = inner.kernel.read().await;
                if kernel.plugins.instance_id(definition).ok() != Some(*instance)
                    || kernel.plugins.state(definition).ok() != Some(Lifecycle::Active)
                {
                    return Err(denied(
                        ModuleMessageErrorCode::SourceStopped,
                        "stale source generation",
                    ));
                }
                let receiver_instance = kernel
                    .plugins
                    .instance_id(&request.target_module)
                    .map_err(|_| {
                        denied(
                            ModuleMessageErrorCode::TargetUnavailable,
                            "target module unavailable",
                        )
                    })?;
                if kernel.plugins.state(&request.target_module).ok() != Some(Lifecycle::Active) {
                    return Err(denied(
                        ModuleMessageErrorCode::TargetUnavailable,
                        "target module is not running",
                    ));
                }
                let registration = kernel
                    .capabilities
                    .registration(&request.capability)
                    .ok_or_else(|| {
                        denied(
                            ModuleMessageErrorCode::TargetUnavailable,
                            "target capability unavailable",
                        )
                    })?;
                if registration.module.as_ref() != Some(&request.target_module)
                    || !registration.scope.is_open()
                {
                    return Err(denied(
                        ModuleMessageErrorCode::TargetUnavailable,
                        "target owner mismatch or stopping",
                    ));
                }
                let tool = kernel
                    .capabilities
                    .capabilities()
                    .into_iter()
                    .find(|tool| tool.id() == &request.capability)
                    .ok_or_else(|| {
                        denied(
                            ModuleMessageErrorCode::TargetUnavailable,
                            "target not executable",
                        )
                    })?;
                (
                    MessageModuleIdentity {
                        definition: request.target_module.clone(),
                        instance: receiver_instance,
                    },
                    tool.module_contract(),
                )
            };
            if if internal {
                contract.surface != ToolSurface::Internal
            } else {
                contract.surface != ToolSurface::Logical || !contract.module_visible
            } {
                return Err(denied(
                    ModuleMessageErrorCode::PrivateTarget,
                    "target is not the authorized execution surface",
                ));
            }
            if request.kind == ModuleMessageKind::Query && !contract.query_safe {
                return Err(denied(
                    ModuleMessageErrorCode::Unauthorized,
                    "target is not query-safe",
                ));
            }
            let inherited = parent.as_ref().and_then(|ctx| ctx.message_route.clone());
            let mut route = inherited.unwrap_or_else(|| ModuleMessageRoute {
                visited: vec![source.clone()],
                depth: 0,
                trace: parent
                    .as_ref()
                    .and_then(|ctx| ctx.trace)
                    .unwrap_or_else(apeireth_core::kernel::TraceId::new),
                message: TaskId::new(),
            });
            if route.depth >= pipeline.limits.max_hops
                || !internal
                    && route
                        .visited
                        .iter()
                        .any(|hop| hop.instance == receiver.instance)
            {
                return Err(denied(
                    ModuleMessageErrorCode::Loop,
                    "module route loop/depth limit",
                ));
            }
            let causation = if route.depth > 0 {
                Some(route.message)
            } else {
                None
            };
            if !internal {
                route.visited.push(receiver.clone());
            }
            route.depth += 1;
            route.message = TaskId::new();
            if cancellation.is_cancelled() || Instant::now() >= deadline {
                return Err(denied(
                    ModuleMessageErrorCode::Cancelled,
                    "message is no longer active",
                ));
            }
            let authority_source = if parent.is_some() {
                MessageAuthoritySource::CallerDelegation
            } else {
                MessageAuthoritySource::ModuleStanding
            };
            let envelope = ModuleMessageEnvelope {
                protocol_version: 1,
                message_id: route.message,
                correlation_id: None,
                causation_id: causation,
                trace_id: route.trace,
                source: source.clone(),
                target_module: receiver.definition.clone(),
                target_capability: Some(request.capability.clone()),
                principal: grant.principal.clone(),
                authorization_ref: authorization_ref.clone(),
                authority_source,
                kind: request.kind,
                payload_schema_version: request.schema_version,
                payload: request.payload.clone(),
                deadline_unix_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    .saturating_add(
                        deadline
                            .saturating_duration_since(Instant::now())
                            .as_millis(),
                    )
                    .try_into()
                    .unwrap_or(u64::MAX),
                reply_mode: if request.kind == ModuleMessageKind::Event {
                    MessageReplyMode::Acknowledgement
                } else {
                    MessageReplyMode::Result
                },
                visibility: request.visibility,
            };
            let reservation = if let Some(key) = request.idempotency_key {
                Some(pipeline.reserve(
                    IdempotencyKey {
                        source: source.clone(),
                        target: receiver.clone(),
                        capability: request.capability.clone(),
                        kind: request.kind,
                        authorization_ref: authorization_ref.clone(),
                        principal: grant.principal.clone(),
                        parent_session: parent.as_ref().and_then(|ctx| ctx.session),
                        key,
                    },
                    format!(
                        "{:x}",
                        Sha256::digest(
                            serde_json::to_vec(&(request.schema_version, &request.payload))
                                .expect("JSON value")
                        )
                    ),
                    envelope.message_id,
                    started.clone(),
                )?)
            } else {
                None
            };
            let slot = if parent.is_some() {
                &self.delegated
            } else {
                &self.background
            };
            let mut slot = tokio::time::timeout(
                deadline.saturating_duration_since(Instant::now()),
                slot.lock(),
            )
            .await
            .map_err(|_| {
                denied(
                    ModuleMessageErrorCode::Timeout,
                    "mailbox admission deadline",
                )
            })?;
            if slot.is_none() {
                let module = BoundModule {
                    message_port: OnceLock::new(),
                    binding: self.binding.clone(),
                    ready: self.ready.clone(),
                    owner: self.owner.clone(),
                    scope: self.scope.clone(),
                };
                let actor = module
                    .spawn_agent(AgentConfig {
                        tools: module_tools.iter().cloned().collect(),
                        principal: grant.principal.clone(),
                        ..Default::default()
                    })
                    .await
                    .map_err(|_| {
                        denied(
                            ModuleMessageErrorCode::Unauthorized,
                            "module execution instance not granted/available",
                        )
                    })?;
                *slot = Some(actor);
            }
            let id = slot.as_ref().expect("created mailbox").id();
            drop(slot);
            let actor = inner
                .instances
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&id)
                .cloned()
                .ok_or_else(|| {
                    denied(
                        ModuleMessageErrorCode::SourceStopped,
                        "mailbox instance unavailable",
                    )
                })?;
            let invocation = Arc::new(MessageInvocation {
                envelope: envelope.clone(),
                receiver: receiver.clone(),
                route,
                security,
                parent,
                deadline,
                cancellation,
                tools,
                started: started.clone(),
            });
            let result = actor.run_message(invocation).await?;
            if let Some(reservation) = reservation {
                reservation.finish(result.is_ok());
            }
            if !super::super::dispatcher::fits_json(&result, pipeline.limits.max_payload_bytes) {
                let mut error = denied(
                    ModuleMessageErrorCode::ResultLimit,
                    "effect may have completed; reply exceeds payload bound",
                );
                error.may_have_executed = true;
                return Err(error);
            }
            let payload = if envelope.kind == ModuleMessageKind::Event {
                serde_json::json!({"delivered":result.is_ok()})
            } else {
                serde_json::to_value(&result).expect("tool result")
            };
            let reply = ModuleMessageEnvelope {
                protocol_version: 1,
                message_id: TaskId::new(),
                correlation_id: Some(envelope.message_id),
                causation_id: Some(envelope.message_id),
                trace_id: envelope.trace_id,
                source: receiver,
                target_module: source.definition,
                target_capability: None,
                principal: envelope.principal,
                authorization_ref: envelope.authorization_ref,
                authority_source: envelope.authority_source,
                kind: ModuleMessageKind::Reply,
                payload_schema_version: 1,
                payload,
                deadline_unix_ms: envelope.deadline_unix_ms,
                reply_mode: envelope.reply_mode,
                visibility: envelope.visibility,
            };
            Ok(ModuleMessageReply {
                envelope: reply,
                result: if envelope.kind == ModuleMessageKind::Event {
                    None
                } else {
                    Some(result)
                },
            })
        };
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), work)
            .await
            .unwrap_or_else(|_| {
                Err(interrupted(
                    ModuleMessageErrorCode::Timeout,
                    "message policy deadline exceeded",
                    &started,
                ))
            })
    }
}
impl Runtime {
    pub(in crate::canonical) fn messages_enabled(&self) -> bool {
        self.instances
            .upgrade()
            .is_some_and(|inner| inner.message_pipeline.is_some())
    }
}
impl ManagedAgent {
    async fn run_message(
        &self,
        invocation: Arc<MessageInvocation>,
    ) -> Result<apeireth_protocol::canonical::ToolResult, ModuleMessageError> {
        let _turn = self
            .turn
            .try_lock()
            .map_err(|_| denied(ModuleMessageErrorCode::Backpressure, "mailbox is busy"))?;
        if !self.scope.is_open()
            || self.scope.active_tasks() != 0
            || self.state.lock().unwrap_or_else(|e| e.into_inner()).0 != AgentStatus::Ready
        {
            return Err(denied(
                ModuleMessageErrorCode::SourceStopped,
                "mailbox is stopped or draining",
            ));
        }
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0 = AgentStatus::Running;
        let mut running = Running {
            agent: self,
            completed: false,
        };
        let inner = self
            .inner
            .upgrade()
            .ok_or_else(|| denied(ModuleMessageErrorCode::SourceStopped, "Host stopped"))?;
        let mut kernel = inner.kernel.read().await.execution_view();
        kernel.event_sink = Arc::new(RwLock::new(Arc::new(
            crate::canonical::events::NoopRuntimeEventSink,
        )));
        kernel.message_invocation = Some(invocation.clone());
        let work = kernel.execute_message_tool(self.session, self.id, invocation);
        let result = self.scope.run(work).await.map_err(|_| {
            denied(
                ModuleMessageErrorCode::SourceStopped,
                "source module/instance stopped",
            )
        })?;
        running.completed = true;
        if result.is_ok() {
            self.state.lock().unwrap_or_else(|e| e.into_inner()).0 = AgentStatus::Ready;
        } else {
            self.scope.close();
            *self.state.lock().unwrap_or_else(|e| e.into_inner()) = (
                AgentStatus::Stopping,
                Some("module message interrupted; context not reused".into()),
            );
        }
        result
    }
}
