//! Bounded transport policy/state over the existing module and instance authorities.
use apeireth_core::kernel::{CapabilityId, PluginId, SessionId, TaskId};
use apeireth_governance::TurnSecurityContext;
use apeireth_plugin::agents::AgentPrincipal;
use apeireth_plugin::module_messages::*;
use apeireth_plugin::{ToolCancellation, ToolExecutionContext};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct ModuleMessageGrant {
    pub principal: Option<AgentPrincipal>,
    pub authorization_ref: String,
    pub targets: BTreeMap<PluginId, BTreeSet<CapabilityId>>,
    pub internal_tools: BTreeSet<CapabilityId>,
    pub allow_background: bool,
    pub allow_events: bool,
    pub allow_public_status: bool,
}
impl Default for ModuleMessageGrant {
    fn default() -> Self {
        Self {
            principal: None,
            authorization_ref: String::new(),
            targets: BTreeMap::new(),
            internal_tools: BTreeSet::new(),
            allow_background: false,
            allow_events: false,
            allow_public_status: false,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ModuleMessageLimits {
    pub in_flight: usize,
    pub max_payload_bytes: usize,
    pub max_hops: usize,
    pub idempotency_records: usize,
    pub timeout: Duration,
}
impl Default for ModuleMessageLimits {
    fn default() -> Self {
        Self {
            in_flight: 16,
            max_payload_bytes: 64 * 1024,
            max_hops: 16,
            idempotency_records: 128,
            timeout: Duration::from_secs(5),
        }
    }
}
#[derive(Clone, Default)]
pub struct ModuleMessagePolicy {
    pub limits: ModuleMessageLimits,
    pub grants: BTreeMap<PluginId, ModuleMessageGrant>,
}
impl ModuleMessagePolicy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let l = &self.limits;
        if l.in_flight == 0
            || l.in_flight > 1024
            || l.max_payload_bytes == 0
            || l.max_payload_bytes > 16 * 1024 * 1024
            || l.max_hops == 0
            || l.max_hops > 32
            || l.idempotency_records == 0
            || l.idempotency_records > 4096
            || l.timeout.is_zero()
        {
            return Err("invalid bounded module message policy".into());
        }
        if self
            .grants
            .values()
            .any(|g| g.authorization_ref.trim().is_empty() || g.authorization_ref.len() > 256)
        {
            return Err("message grants need an explicit bounded authorization reference".into());
        }
        Ok(())
    }
}
pub(crate) struct ModuleMessagePipeline {
    pub limits: ModuleMessageLimits,
    pub admission: Arc<tokio::sync::Semaphore>,
    records: Mutex<VecDeque<IdempotencyEntry>>,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IdempotencyKey {
    pub source: MessageModuleIdentity,
    pub target: MessageModuleIdentity,
    pub capability: CapabilityId,
    pub kind: ModuleMessageKind,
    pub authorization_ref: String,
    pub principal: Option<AgentPrincipal>,
    pub parent_session: Option<SessionId>,
    pub key: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeliveryState {
    InFlight,
    Complete,
    Indeterminate,
}
struct IdempotencyEntry {
    key: IdempotencyKey,
    digest: String,
    message: TaskId,
    state: DeliveryState,
}
pub(crate) struct IdempotencyReservation {
    pipeline: Arc<ModuleMessagePipeline>,
    message: TaskId,
    started: Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for IdempotencyReservation {
    fn drop(&mut self) {
        let mut records = self
            .pipeline
            .records
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(index) = records.iter().position(|r| r.message == self.message) {
            if records[index].state == DeliveryState::InFlight {
                if self.started.load(std::sync::atomic::Ordering::Acquire) {
                    records[index].state = DeliveryState::Indeterminate;
                } else {
                    records.remove(index);
                }
            }
        }
    }
}
impl IdempotencyReservation {
    pub fn finish(&self, certain: bool) {
        if let Some(r) = self
            .pipeline
            .records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter_mut()
            .find(|r| r.message == self.message)
        {
            r.state = if certain {
                DeliveryState::Complete
            } else {
                DeliveryState::Indeterminate
            };
        }
    }
}
impl ModuleMessagePipeline {
    pub fn new(limits: ModuleMessageLimits) -> Self {
        Self {
            admission: Arc::new(tokio::sync::Semaphore::new(limits.in_flight)),
            limits,
            records: Mutex::new(VecDeque::new()),
        }
    }
    pub fn reserve(
        self: &Arc<Self>,
        key: IdempotencyKey,
        digest: String,
        message: TaskId,
        started: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<IdempotencyReservation, ModuleMessageError> {
        let mut records = self.records.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = records.iter().find(|r| r.key == key) {
            let mut error = ModuleMessageError::new(
                if old.digest == digest {
                    ModuleMessageErrorCode::Duplicate
                } else {
                    ModuleMessageErrorCode::IdempotencyConflict
                },
                "retained idempotency key cannot execute another effect",
            );
            error.original_message = Some(old.message);
            error.may_have_executed = true;
            return Err(error);
        }
        if records.len() >= self.limits.idempotency_records {
            let Some(index) = records
                .iter()
                .position(|r| r.state == DeliveryState::Complete)
            else {
                return Err(ModuleMessageError::new(
                    ModuleMessageErrorCode::Backpressure,
                    "idempotency window contains unresolved operations",
                ));
            };
            records.remove(index);
        }
        records.push_back(IdempotencyEntry {
            key,
            digest,
            message,
            state: DeliveryState::InFlight,
        });
        Ok(IdempotencyReservation {
            pipeline: self.clone(),
            message,
            started,
        })
    }
}
#[derive(Clone)]
pub(crate) struct MessageInvocation {
    pub envelope: ModuleMessageEnvelope,
    pub receiver: MessageModuleIdentity,
    pub route: ModuleMessageRoute,
    pub security: Option<TurnSecurityContext>,
    pub parent: Option<Arc<ToolExecutionContext>>,
    pub deadline: Instant,
    pub cancellation: ToolCancellation,
    pub tools: BTreeSet<CapabilityId>,
    pub started: Arc<std::sync::atomic::AtomicBool>,
}
pub(crate) async fn cancelled(token: &ToolCancellation) {
    while !token.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}
