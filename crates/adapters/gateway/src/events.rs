//! Gateway-level SSE event bus (`GET /v1/apeireth/events`).
//!
//! Emits product-facing lifecycle events:
//! `backend_ready` / `turn_started` / `turn_delta` / `turn_completed` /
//! `approval_required` / `approval_resolved` / `presence_state`.
//!
//! `presence_state` frames are produced by [`crate::presence`] (contract:
//! `docs/design/00-PHILOSOPHY.md` §10, wire: `docs/gateway-api-contract.md`
//! §8a) on turn end and on a 60 s heartbeat — never at higher frequency.
//!
//! Boundary notes (contract §8):
//! - `turn_delta` is a lifecycle MIRROR: it carries the final assistant text as
//!   ONE delta, because the canonical runtime completes a turn before the
//!   gateway encodes it onto this bus. Token-level rendering does NOT go
//!   through this bus — it goes through `POST /v1/chat/completions` with
//!   `stream:true` (true incremental provider SSE, since 2026-09-10).
//! - The bus is in-process broadcast; a subscriber that lags behind skips
//!   frames by broadcast semantics (no unbounded buffering). A skip is never
//!   silent: the connection emits an explicit `frames_omitted` fact and the
//!   omission rides beside the last value as a failure frame (丢帧 = 显式省略
//!   事实).
//!
//! Lease-managed subscription surfaces (资源租约语义推广, 与
//! [`apeireth_core::resource_lease::ResourceRegistry`] 同型):
//! - **SSE delivery surface** (`gateway_events`): every SSE connection holds
//!   one lease — the first holder opens the delivery stream, the last closes
//!   it, a keepalive pin (assembly) keeps it open so frame retention spans
//!   connection gaps. A connecting client is sent the current snapshot frame
//!   (`stream_snapshot`: last value + mounted failure) immediately — late
//!   subscribers read the present instead of waiting for the next event.
//! - **Observation flush surface** (`observation_flush`): the trace/audit
//!   batch face. The first holder starts the periodic flush loop, the last
//!   holder stops it; a crashed pass becomes a failure frame (崩溃失败即帧)
//!   and the next clean pass clears it.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use apeireth_core::kernel::Timestamp;
use apeireth_core::resource_lease::{
    ResourceError, ResourceFrame, ResourceHooks, ResourceLease, ResourcePin, ResourceRegistry,
};
use apeireth_runtime::canonical::{RuntimeEvent, RuntimeEventSink, TraceEvent};

use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::{AuditCommand, GatewayState, TraceCommand, TraceSpanDto};

/// Bus capacity: bounded, newest-first under pressure.
const BUS_CAPACITY: usize = 256;

/// Surface name of the SSE delivery frame stream.
const EVENTS_SURFACE: &str = "gateway_events";

/// Surface name of the observation flush frame stream.
const FLUSH_SURFACE: &str = "observation_flush";

/// Wire name of the connect-time snapshot frame (last value + mounted failure).
const STREAM_SNAPSHOT_EVENT: &str = "stream_snapshot";

/// Wire name of the explicit omission fact emitted when frames were skipped.
const FRAMES_OMITTED_EVENT: &str = "frames_omitted";

/// Default cadence of the lease-driven observation flush loop.
pub const FLUSH_PASS_INTERVAL_SECS: u64 = 30;

/// One product-facing event frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GatewayEvent {
    /// Stable event name (see module docs).
    pub event: String,
    /// Event payload (JSON object).
    pub data: serde_json::Value,
}

impl GatewayEvent {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self {
            event: event.to_string(),
            data,
        }
    }
}

/// Lifecycle record for the SSE delivery stream: the first holder opens it and
/// the last holder closes it. The stream itself is pushed from upstream (there
/// is no producer loop to drive), so the hooks record the delivery lifecycle
/// that handlers and tests observe instead of starting a task.
#[derive(Debug, Default)]
struct EventsDeliveryDriver {
    streaming: AtomicBool,
    opens: AtomicUsize,
    closes: AtomicUsize,
}

impl EventsDeliveryDriver {
    fn on_open(&self) -> Result<(), String> {
        self.streaming.store(true, Ordering::SeqCst);
        self.opens.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn on_close(&self) {
        self.streaming.store(false, Ordering::SeqCst);
        self.closes.fetch_add(1, Ordering::SeqCst);
    }
}

/// In-process event bus shared by handlers and the SSE endpoint.
///
/// Alongside the broadcast fan-out the bus keeps a lease-managed frame surface
/// ([`ResourceRegistry`]): publishing retains the latest frame so a connection
/// reads the current state on arrival (快照即读, 不回放), and a delivery gap
/// rides beside the last value as a failure frame (失败即帧, 下一成功帧自清).
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<GatewayEvent>,
    frames: Arc<ResourceRegistry<GatewayEvent>>,
    delivery: Arc<EventsDeliveryDriver>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(BUS_CAPACITY)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        let delivery = Arc::new(EventsDeliveryDriver::default());
        let open_driver = Arc::clone(&delivery);
        let close_driver = Arc::clone(&delivery);
        let frames = ResourceRegistry::new(
            EVENTS_SURFACE,
            ResourceHooks::new(
                Arc::new(move || open_driver.on_open()),
                Arc::new(move || close_driver.on_close()),
            ),
        );
        Self {
            tx,
            frames,
            delivery,
        }
    }

    /// Publish one event; no subscribers is a silent success, and a lagging
    /// subscriber never blocks the publisher. The frame surface retains the
    /// latest event for snapshot-on-connect while the delivery stream is open.
    pub fn publish(&self, event: GatewayEvent) {
        let _ = self.frames.publish(event.clone());
        let _ = self.tx.send(event);
    }

    /// Subscribe for events emitted after this call.
    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.tx.subscribe()
    }

    /// One SSE connection lease on the delivery surface: the first holder
    /// opens the delivery stream, the last release closes it.
    pub fn connect(&self) -> Result<EventsSubscription, ResourceError> {
        let lease = self.frames.acquire()?;
        Ok(EventsSubscription {
            _lease: lease,
            frames: Arc::clone(&self.frames),
        })
    }

    /// Keepalive pin on the delivery surface (装配面保活): while it is held
    /// the delivery stream never closes, so frame retention spans connection
    /// gaps and every connect reads a current snapshot.
    pub fn keepalive(&self) -> Result<ResourcePin, ResourceError> {
        self.frames.pin()
    }

    /// Snapshot-on-read of the delivery surface: the last event plus any
    /// mounted failure frame (omission facts), no history replay.
    pub fn frame_snapshot(&self) -> ResourceFrame<GatewayEvent> {
        self.frames.snapshot()
    }

    /// Live connection leases held right now (pins not counted).
    pub fn connections(&self) -> usize {
        self.frames.holders()
    }

    /// Whether the delivery stream is currently open.
    pub fn is_delivery_open(&self) -> bool {
        self.frames.is_open()
    }

    /// Delivery-stream lifecycle counts (open/close hook invocations).
    pub fn delivery_lifecycle_counts(&self) -> (usize, usize) {
        (
            self.delivery.opens.load(Ordering::SeqCst),
            self.delivery.closes.load(Ordering::SeqCst),
        )
    }

    /// Shut down the delivery surface: no new connection leases, no further
    /// frame retention, no late release callbacks.
    pub fn shutdown_delivery(&self) {
        self.frames.shutdown();
    }
}

/// One SSE connection lease on the delivery surface. Held for the lifetime of
/// the connection; dropping it releases the lease (last release closes the
/// delivery stream).
pub struct EventsSubscription {
    _lease: ResourceLease,
    frames: Arc<ResourceRegistry<GatewayEvent>>,
}

impl std::fmt::Debug for EventsSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventsSubscription")
            .field("surface", &self.frames)
            .finish()
    }
}

impl EventsSubscription {
    /// Snapshot-on-read: the current frame (last value + mounted failure).
    pub fn snapshot(&self) -> ResourceFrame<GatewayEvent> {
        self.frames.snapshot()
    }

    /// 失败即帧: a delivery gap becomes a formal failure frame beside the
    /// last value; the next successfully delivered frame clears it.
    pub fn note_gap(&self, omitted: u64) {
        let _ = self
            .frames
            .publish_failure(format!("stream gap: {omitted} frame(s) omitted"));
    }
}

impl RuntimeEventSink for EventBus {
    fn emit(&self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::TurnStarted {
                session,
                request,
                trace,
            } => self.publish(GatewayEvent::new(
                "turn_started",
                serde_json::json!({
                    "session": session,
                    "request": request,
                    "trace_id": trace,
                }),
            )),
            RuntimeEvent::Trace {
                session,
                trace,
                at,
                event,
            } => self.emit_trace(session, trace, at, event),
            RuntimeEvent::TurnCompleted {
                session,
                request,
                trace,
                rounds,
                served_by,
            } => self.publish(GatewayEvent::new(
                "turn_completed",
                serde_json::json!({
                    "session": session,
                    "request": request,
                    "trace_id": trace,
                    "rounds": rounds,
                    "served_by": served_by,
                }),
            )),
            RuntimeEvent::ApprovalRequired {
                session,
                request,
                trace,
                approval,
                capability,
                tool_name,
                tool_call_id,
            } => self.publish(GatewayEvent::new(
                "approval_required",
                serde_json::json!({
                    "session": session,
                    "request": request,
                    "trace_id": trace,
                    "approval_id": approval,
                    "capability_id": capability,
                    "tool_name": tool_name,
                    "tool_call_id": tool_call_id,
                }),
            )),
            RuntimeEvent::TurnFailed {
                session,
                request,
                trace,
                error,
            } => self.publish(GatewayEvent::new(
                "turn_failed",
                serde_json::json!({
                    "session": session,
                    "request": request,
                    "trace_id": trace,
                    "error": error,
                }),
            )),
        }
    }
}

impl EventBus {
    fn emit_trace(
        &self,
        session: apeireth_core::kernel::SessionId,
        trace: apeireth_core::kernel::TraceId,
        _at: apeireth_core::kernel::Timestamp,
        event: TraceEvent,
    ) {
        let (name, data) = match event {
            TraceEvent::CapabilityDispatched {
                capability,
                tool_call_id,
                round,
            } => (
                "tool_started",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "capability_id": capability,
                    "tool_call_id": tool_call_id,
                    "round": round,
                }),
            ),
            TraceEvent::CapabilityCompleted {
                capability,
                tool_call_id,
                succeeded,
                round,
            } => (
                if succeeded {
                    "tool_completed"
                } else {
                    "tool_failed"
                },
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "capability_id": capability,
                    "tool_call_id": tool_call_id,
                    "succeeded": succeeded,
                    "round": round,
                }),
            ),
            TraceEvent::CapabilityUnavailable {
                requested,
                tool_call_id,
                reason,
                round,
            } => (
                "tool_failed",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "requested": requested,
                    "tool_call_id": tool_call_id,
                    "reason": reason,
                    "round": round,
                }),
            ),
            TraceEvent::ProviderInvoked {
                provider,
                model,
                round,
            } => (
                "provider_started",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "provider": provider,
                    "model": model,
                    "round": round,
                }),
            ),
            TraceEvent::ProviderSucceeded {
                provider,
                round,
                finish_reason,
                usage,
            } => (
                "provider_completed",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "provider": provider,
                    "round": round,
                    "finish_reason": finish_reason,
                    "usage": usage,
                }),
            ),
            TraceEvent::ProviderFailed {
                provider,
                round,
                error,
                retryable,
            } => (
                "provider_failed",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "provider": provider,
                    "round": round,
                    "error": error,
                    "retryable": retryable,
                }),
            ),
            TraceEvent::GovernanceEvaluated {
                hook,
                action,
                decision,
                reason,
                round,
                ..
            } => (
                "governance_evaluated",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "hook": hook,
                    "action": action,
                    "decision": decision,
                    "reason": reason,
                    "round": round,
                }),
            ),
            TraceEvent::ApprovalResolved {
                approval_id,
                decision,
                round,
            } => (
                "approval_resolved",
                serde_json::json!({
                    "session": session,
                    "trace_id": trace,
                    "approval_id": approval_id,
                    "decision": decision,
                    "round": round,
                }),
            ),
            TraceEvent::ApprovalRequested { .. } | TraceEvent::TurnCompleted { .. } => return,
            _ => return,
        };
        self.publish(GatewayEvent::new(name, data));
    }
}

/// Outcome of one flush pass over the buffered observation facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlushReport {
    /// Epoch milliseconds of the pass.
    pub at_ms: i64,
    /// Trace batches written to the archive port.
    pub traces: usize,
    /// Audit facts written to the archive port.
    pub audit: usize,
}

/// Flush loop driver behind the observation flush surface: the first holder
/// starts the periodic pass loop, the last holder stops it. After a stop the
/// loop task is gone, so no ghost pass can run (关停后无幽灵回调).
struct FlushLoopDriver {
    interval: Duration,
    sink: Mutex<Option<Weak<RuntimeObservationSink>>>,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl std::fmt::Debug for FlushLoopDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlushLoopDriver")
            .field("interval", &self.interval)
            .field("running", &self.is_running())
            .finish()
    }
}

impl FlushLoopDriver {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            sink: Mutex::new(None),
            task: Mutex::new(None),
        }
    }

    fn bind(&self, sink: Weak<RuntimeObservationSink>) {
        if let Ok(mut slot) = self.sink.lock() {
            *slot = Some(sink);
        }
    }

    /// Open hook: start the flush loop (idempotent). Outside an async runtime
    /// it degrades honestly — no executor, no loop, no panic.
    fn start(&self) -> Result<(), String> {
        let mut task = self.task.lock().unwrap_or_else(|p| p.into_inner());
        if task.is_some() {
            return Ok(());
        }
        let Some(sink) = self.sink.lock().unwrap_or_else(|p| p.into_inner()).clone() else {
            return Err("observation flush driver is not bound to a sink".to_string());
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return Ok(());
        };
        let interval = self.interval;
        let weak = sink;
        *task = Some(handle.spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ticker.tick().await; // the first tick is immediate; skip it
            loop {
                ticker.tick().await;
                let Some(sink) = weak.upgrade() else {
                    break;
                };
                // Each pass runs on its own task: a crashing writer cannot
                // take the loop down silently (崩溃失败即帧), and a stop only
                // ends the loop — an in-flight pass finishes its writes.
                let pass = tokio::spawn(async move { sink.flush().await });
                if let Err(join_error) = pass.await {
                    if join_error.is_panic() {
                        if let Some(sink) = weak.upgrade() {
                            sink.note_flush_crash();
                        }
                    }
                }
            }
        }));
        Ok(())
    }

    /// Close hook: stop the flush loop. No pass can start from here on.
    fn stop(&self) {
        if let Some(handle) = self.task.lock().unwrap_or_else(|p| p.into_inner()).take() {
            handle.abort();
        }
    }

    fn is_running(&self) -> bool {
        self.task
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some()
    }
}

/// One lease on the observation flush surface: while it is held the flush loop
/// runs ([`ObservationFlushSubscription::snapshot`] reads the latest pass
/// report without replaying older ones).
pub struct ObservationFlushSubscription {
    _lease: ResourceLease,
    frames: Arc<ResourceRegistry<FlushReport>>,
}

impl std::fmt::Debug for ObservationFlushSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservationFlushSubscription")
            .field("surface", &self.frames)
            .finish()
    }
}

impl ObservationFlushSubscription {
    /// Snapshot-on-read: the last flush report plus any mounted failure frame.
    pub fn snapshot(&self) -> ResourceFrame<FlushReport> {
        self.frames.snapshot()
    }
}

/// Runtime-event consumer that archives trace/audit facts through gateway
/// ports. It collects synchronously at the runtime boundary, then flushes from
/// the request future so archive writes are awaited without blocking the
/// kernel's event sink. The flush side is lease-managed (资源租约语义): the
/// first holder of [`RuntimeObservationSink::subscribe_flush`] /
/// [`RuntimeObservationSink::keepalive`] starts the periodic flush loop, the
/// last release stops it, and every pass outcome — including a crashed writer
/// or a poisoned buffer — rides on the flush frame surface as a frame rather
/// than disappearing silently.
pub struct RuntimeObservationSink {
    traces: Mutex<HashMap<String, Vec<TraceSpanDto>>>,
    audit: Mutex<Vec<(String, Option<String>)>>,
    trace_commands: Option<Arc<dyn TraceCommand>>,
    audit_commands: Option<Arc<dyn AuditCommand>>,
    frames: Arc<ResourceRegistry<FlushReport>>,
    flusher: Arc<FlushLoopDriver>,
}

impl RuntimeObservationSink {
    pub fn new(
        trace_commands: Option<Arc<dyn TraceCommand>>,
        audit_commands: Option<Arc<dyn AuditCommand>>,
    ) -> Self {
        Self::with_flush_interval(
            trace_commands,
            audit_commands,
            Duration::from_secs(FLUSH_PASS_INTERVAL_SECS),
        )
    }

    /// Same flush surface with an explicit loop cadence (embedders/tests).
    pub fn with_flush_interval(
        trace_commands: Option<Arc<dyn TraceCommand>>,
        audit_commands: Option<Arc<dyn AuditCommand>>,
        interval: Duration,
    ) -> Self {
        let flusher = Arc::new(FlushLoopDriver::new(interval));
        let open_driver = Arc::clone(&flusher);
        let close_driver = Arc::clone(&flusher);
        let frames = ResourceRegistry::new(
            FLUSH_SURFACE,
            ResourceHooks::new(
                Arc::new(move || open_driver.start()),
                Arc::new(move || close_driver.stop()),
            ),
        );
        Self {
            traces: Mutex::new(HashMap::new()),
            audit: Mutex::new(Vec::new()),
            trace_commands,
            audit_commands,
            frames,
            flusher,
        }
    }

    /// Acquire a lease on the flush surface. The first holder starts the
    /// flush loop; the last release stops it.
    pub fn subscribe_flush(
        self: &Arc<Self>,
    ) -> Result<ObservationFlushSubscription, ResourceError> {
        self.flusher.bind(Arc::downgrade(self));
        let lease = self.frames.acquire()?;
        Ok(ObservationFlushSubscription {
            _lease: lease,
            frames: Arc::clone(&self.frames),
        })
    }

    /// Keepalive pin on the flush surface (装配面保活): while it is held the
    /// flush loop keeps running regardless of subscriptions.
    pub fn keepalive(self: &Arc<Self>) -> Result<ResourcePin, ResourceError> {
        self.flusher.bind(Arc::downgrade(self));
        self.frames.pin()
    }

    /// Snapshot-on-read of the flush surface: the last pass report plus any
    /// mounted failure frame, no history replay.
    pub fn frame_snapshot(&self) -> ResourceFrame<FlushReport> {
        self.frames.snapshot()
    }

    /// Whether the flush loop task is currently running.
    pub fn is_flush_loop_running(&self) -> bool {
        self.flusher.is_running()
    }

    /// Persist all facts collected since the previous flush.
    pub async fn flush(&self) {
        let _ = self.flush_pass().await;
    }

    /// One flush pass: drain the batches, write the archive ports, and record
    /// the outcome on the flush frame surface — a pass report on success, a
    /// failure frame on a poisoned buffer (a crashed writer). Facts recovered
    /// from a poisoned buffer are still written; nothing is dropped silently.
    async fn flush_pass(&self) -> Result<FlushReport, String> {
        let mut faults: Vec<String> = Vec::new();
        let traces = match self.traces.lock() {
            Ok(mut traces) => std::mem::take(&mut *traces),
            Err(poisoned) => {
                faults.push(
                    "trace buffer writer crashed (poisoned lock); contents recovered".to_string(),
                );
                std::mem::take(&mut *poisoned.into_inner())
            }
        };
        let audit = match self.audit.lock() {
            Ok(mut audit) => std::mem::take(&mut *audit),
            Err(poisoned) => {
                faults.push(
                    "audit buffer writer crashed (poisoned lock); contents recovered".to_string(),
                );
                std::mem::take(&mut *poisoned.into_inner())
            }
        };

        let mut traces = traces.into_iter().collect::<Vec<_>>();
        traces.sort_by(|left, right| left.0.cmp(&right.0));
        let trace_batches = traces.len();
        let audit_facts = audit.len();

        if let Some(command) = &self.trace_commands {
            for (trace_id, spans) in traces {
                command.append_trace(&trace_id, spans).await;
            }
        }
        if let Some(command) = &self.audit_commands {
            for (event, detail) in audit {
                command.append_audit(&event, detail.as_deref()).await;
            }
        }

        let report = FlushReport {
            at_ms: Timestamp::now().epoch_millis(),
            traces: trace_batches,
            audit: audit_facts,
        };
        if !faults.is_empty() {
            // 失败即帧: 崩溃证据作为正式一帧挂在最后报告旁。
            let reason = faults.join("; ");
            let _ = self.frames.publish_failure(reason.clone());
            return Err(reason);
        }
        if trace_batches + audit_facts > 0 {
            // 成功帧: 入帧面 (自清失败帧); 空转 pass 无事可报, 不发帧。
            let _ = self.frames.publish(report.clone());
        }
        Ok(report)
    }

    /// 失败即帧: a crashed flush pass becomes a formal failure frame beside
    /// the last report; the next clean pass clears it.
    fn note_flush_crash(&self) {
        let _ = self.frames.publish_failure(
            "observation flush pass crashed (writer panic); facts drained in that pass were lost"
                .to_string(),
        );
    }

    /// Lock one buffer, recovering contents after a poisoned (crashed) writer.
    /// The crash itself becomes a failure frame — the fact is never swallowed
    /// silently (失败即帧).
    fn lock_buffer<'a, T: Default>(
        &self,
        buffer: &'a Mutex<T>,
        label: &str,
    ) -> std::sync::MutexGuard<'a, T> {
        buffer.lock().unwrap_or_else(|poisoned| {
            let _ = self.frames.publish_failure(format!(
                "{label} buffer writer crashed (poisoned lock); contents recovered"
            ));
            poisoned.into_inner()
        })
    }

    /// Buffer a trace span for the next flush pass.
    fn buffer_trace_span(&self, trace_id: &str, span: TraceSpanDto) {
        let mut traces = self.lock_buffer(&self.traces, "trace");
        traces.entry(trace_id.to_string()).or_default().push(span);
    }

    /// Buffer one audit fact for the next flush pass.
    fn buffer_audit(&self, event: &str, detail: Option<String>) {
        let mut audit = self.lock_buffer(&self.audit, "audit");
        audit.push((event.to_string(), detail));
    }
}

impl RuntimeEventSink for RuntimeObservationSink {
    fn emit(&self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::Trace {
                session,
                trace,
                at,
                event,
            } => {
                let trace_id = trace.to_string();
                let index = self
                    .lock_buffer(&self.traces, "trace")
                    .get(&trace_id)
                    .map_or(0, Vec::len);
                let parent = (index > 0).then(|| format!("{trace_id}-0"));
                let (kind, status) = trace_span_kind_status(&event);
                self.buffer_trace_span(
                    &trace_id,
                    TraceSpanDto {
                        span_id: format!("{trace_id}-{index}"),
                        parent_span_id: parent,
                        kind: kind.to_string(),
                        actor: "runtime".to_string(),
                        status: status.to_string(),
                        summary: None,
                        started_at: at.epoch_millis(),
                        ended_at: None,
                        session_id: Some(session.to_string()),
                    },
                );
                if let TraceEvent::ApprovalResolved {
                    approval_id,
                    decision,
                    ..
                } = event
                {
                    self.buffer_audit(
                        "approval.resolved",
                        Some(format!("approval={approval_id} decision={decision}")),
                    );
                }
            }
            RuntimeEvent::TurnCompleted {
                session,
                rounds,
                served_by,
                ..
            } => {
                self.buffer_audit(
                    "chat.turn.completed",
                    Some(format!(
                        "session={session} rounds={rounds} served_by={served_by}"
                    )),
                );
            }
            RuntimeEvent::ApprovalRequired {
                session,
                approval,
                tool_name,
                ..
            } => {
                self.buffer_audit(
                    "chat.turn.pending_approval",
                    Some(format!(
                        "session={session} approval={approval} tool={tool_name}"
                    )),
                );
            }
            RuntimeEvent::TurnFailed { session, error, .. } => {
                self.buffer_audit(
                    "chat.turn.failed",
                    Some(format!("session={session} error={error}")),
                );
            }
            RuntimeEvent::TurnStarted { .. } => {}
        }
    }
}

fn trace_span_kind_status(event: &TraceEvent) -> (&'static str, &'static str) {
    match event {
        TraceEvent::ProviderInvoked { .. } | TraceEvent::ProviderSucceeded { .. } => {
            ("provider", "ok")
        }
        TraceEvent::ProviderFailed { .. } => ("provider", "error"),
        TraceEvent::ApprovalRequested { .. } | TraceEvent::ApprovalResolved { .. } => {
            ("approval", "ok")
        }
        TraceEvent::GovernanceEvaluated { .. } => ("governance", "ok"),
        TraceEvent::CapabilityUnavailable { .. } => ("capability", "error"),
        TraceEvent::CapabilityDispatched { .. } => ("tool", "ok"),
        TraceEvent::CapabilityCompleted { succeeded, .. } => {
            ("tool", if *succeeded { "ok" } else { "error" })
        }
        TraceEvent::TurnCompleted { .. } => ("turn", "ok"),
        _ => ("event", "ok"),
    }
}

/// `GET /v1/apeireth/events` — SSE stream of gateway lifecycle events.
///
/// Per-connection lease (逐连接租约): the handler holds one delivery lease for
/// the whole connection body; the first connection opens the delivery stream,
/// the last disconnect closes it. On connect the client is sent the
/// `stream_snapshot` frame first (连接即送快照帧 — the current frame read as
/// it is, never a replay), and any delivery gap becomes an explicit
/// `frames_omitted` fact plus a failure frame on the surface (丢帧 = 显式省略
/// 事实, 不静默). A shut-down delivery surface answers an explicit error frame
/// instead of silently streaming nothing.
pub async fn events_handler(State(state): State<GatewayState>) -> Response {
    let connection = match state.events.connect() {
        Ok(connection) => connection,
        Err(error) => {
            return ErrorFrame::response(
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::Internal,
                format!("events delivery stream unavailable: {error}"),
            )
        }
    };
    let live = state.events.subscribe();

    // 连接即送快照帧: 当前帧 (最后值 + 挂着的失败帧) 先行, 历史不回放。
    let snapshot = connection.snapshot();
    let head = vec![Ok::<_, Infallible>(
        SseEvent::default().event(STREAM_SNAPSHOT_EVENT).data(
            serde_json::json!({
                "surface": EVENTS_SURFACE,
                "latest": snapshot.value,
                "failure": snapshot.failure,
            })
            .to_string(),
        ),
    )];

    let stream =
        tokio_stream::iter(head).chain(BroadcastStream::new(live).filter_map(move |item| {
            // `connection` rides inside this closure: the lease lives exactly as
            // long as the stream, and dropping the stream (client disconnect)
            // releases it.
            match item {
                Ok(event) => match serde_json::to_string(&event.data) {
                    Ok(payload) => Some(Ok::<_, Infallible>(
                        SseEvent::default().event(&event.event).data(payload),
                    )),
                    Err(_) => {
                        // 失败即帧: 编码失败作为正式一帧上报, 不静默吞掉。
                        let _ = connection
                            .frames
                            .publish_failure("event payload serialization failed".to_string());
                        Some(Ok(SseEvent::default().event(&event.event).data("{}")))
                    }
                },
                Err(BroadcastStreamRecvError::Lagged(omitted)) => {
                    // 丢帧 = 显式省略事实: 落后跳过的帧数说出来, 并挂失败帧。
                    connection.note_gap(omitted);
                    Some(Ok(SseEvent::default().event(FRAMES_OMITTED_EVENT).data(
                        serde_json::json!({
                            "surface": EVENTS_SURFACE,
                            "omitted": omitted,
                        })
                        .to_string(),
                    )))
                }
            }
        }));
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, RequestId, SessionId, TraceId};

    fn event(name: &str, n: u64) -> GatewayEvent {
        GatewayEvent::new(name, serde_json::json!({ "n": n }))
    }

    fn trace_event() -> RuntimeEvent {
        RuntimeEvent::Trace {
            session: SessionId::new(),
            trace: TraceId::new(),
            at: Timestamp::from_epoch_millis(1_726_992_000_000).unwrap(),
            event: TraceEvent::CapabilityDispatched {
                capability: CapabilityId::new("tool.demo").unwrap(),
                tool_call_id: "call_1".into(),
                round: 1,
            },
        }
    }

    fn completed_event() -> RuntimeEvent {
        RuntimeEvent::TurnCompleted {
            session: SessionId::new(),
            request: RequestId::new(),
            trace: TraceId::new(),
            rounds: 1,
            served_by: CapabilityId::new("provider.fake").unwrap(),
        }
    }

    // ------------------------------------------------------------------
    // SSE 交付面: 逐连接租约 (首开末关 / pin 保活 / 快照即读 / 失败即帧 / 无幽灵)
    // ------------------------------------------------------------------

    #[test]
    fn delivery_stream_opens_with_the_first_connection_and_closes_with_the_last() {
        let bus = EventBus::new(4);
        assert!(!bus.is_delivery_open(), "无连接时交付流不开");

        let first = bus.connect().expect("connection");
        assert!(bus.is_delivery_open(), "首连接开流");
        assert_eq!(bus.connections(), 1);
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 0));

        let second = bus.connect().expect("connection");
        assert_eq!(bus.connections(), 2, "租约逐连接计数");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 0), "共享不重复开流");

        drop(first);
        assert!(bus.is_delivery_open(), "仍有连接, 流不关");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 0));

        drop(second);
        assert!(!bus.is_delivery_open(), "末连接关停");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 1), "关停恰好一次");

        // 重连 = 重新开流 (开/关可循环)。
        let third = bus.connect().expect("connection");
        assert_eq!(bus.delivery_lifecycle_counts(), (2, 1));
        drop(third);
        assert_eq!(bus.delivery_lifecycle_counts(), (2, 2));
    }

    #[test]
    fn delivery_pin_keeps_the_stream_open_between_connections() {
        let bus = EventBus::new(4);
        let pin = bus.keepalive().expect("pin");
        assert!(bus.is_delivery_open(), "装配面 pin 即首持有者");

        let connection = bus.connect().expect("connection");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 0));
        drop(connection);
        assert!(bus.is_delivery_open(), "pin 保活: 连接清零流也不关");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 0), "无幽灵关停回调");

        drop(pin);
        assert!(!bus.is_delivery_open(), "最后一个 pin 释放即关停");
        assert_eq!(bus.delivery_lifecycle_counts(), (1, 1));
    }

    #[test]
    fn connection_snapshot_reads_the_latest_frame_without_replay() {
        let bus = EventBus::new(4);
        let _connection = bus.connect().expect("connection");

        bus.publish(event("turn_started", 1));
        bus.publish(event("turn_started", 2));
        bus.publish(event("turn_completed", 3));

        // 快照即读最新帧, 不回放 v1/v2。
        let snapshot = bus.frame_snapshot();
        let latest = snapshot.value.as_ref().expect("latest frame");
        assert_eq!(latest.event, "turn_completed");
        assert_eq!(latest.data["n"], 3);
        assert!(!snapshot.has_failure());

        // 后连接读到的同样是当前帧, 不是从头补发。
        let late = bus.connect().expect("connection");
        assert_eq!(late.snapshot().value.expect("frame").data["n"], 3);
    }

    #[test]
    fn delivery_gap_becomes_an_explicit_failure_frame_and_self_clears() {
        let bus = EventBus::new(4);
        let connection = bus.connect().expect("connection");
        bus.publish(event("turn_started", 1));

        // 落后跳帧 = 显式省略事实 (失败即帧), 不静默。
        connection.note_gap(3);
        let frame = bus.frame_snapshot();
        assert_eq!(
            frame.failure.as_deref(),
            Some("stream gap: 3 frame(s) omitted")
        );
        assert!(frame.value.is_some(), "失败帧不清最后值");

        // 下一成功帧自清。
        bus.publish(event("turn_completed", 2));
        assert!(!bus.frame_snapshot().has_failure(), "下一成功帧自清失败帧");
    }

    #[test]
    fn shutdown_refuses_new_connections_and_late_releases_stay_silent() {
        let bus = EventBus::new(4);
        let connection = bus.connect().expect("connection");
        let pin = bus.keepalive().expect("pin");
        bus.publish(event("turn_started", 1));
        let before = bus.frame_snapshot();

        bus.shutdown_delivery();
        assert!(!bus.is_delivery_open(), "关停即关流");
        assert!(
            matches!(bus.connect(), Err(ResourceError::Shutdown(_))),
            "关停后不再发放连接租约"
        );
        assert!(matches!(bus.keepalive(), Err(ResourceError::Shutdown(_))));

        // 幽灵检查: 事后释放的租约/pin 不得再触发关停钩子。
        let counts = bus.delivery_lifecycle_counts();
        drop(connection);
        drop(pin);
        assert_eq!(bus.delivery_lifecycle_counts(), counts, "关停后无幽灵回调");

        // 关停后帧面冻结 (无幽灵更新)。
        bus.publish(event("turn_completed", 2));
        assert_eq!(bus.frame_snapshot(), before, "关停后无幽灵帧");
    }

    // ------------------------------------------------------------------
    // 观测攒批面: flush 循环租约化 (首持有启停 / 快照即读 / 崩溃失败即帧 / 无幽灵)
    // ------------------------------------------------------------------

    /// Fake archive port: counts writes and can crash on demand.
    #[derive(Debug, Default)]
    struct FakeArchive {
        traces: AtomicUsize,
        audit: AtomicUsize,
        crash: AtomicBool,
    }

    #[async_trait::async_trait]
    impl TraceCommand for FakeArchive {
        async fn append_trace(&self, _trace_id: &str, _spans: Vec<TraceSpanDto>) {
            if self.crash.load(Ordering::SeqCst) {
                panic!("archive writer crashed");
            }
            self.traces.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait::async_trait]
    impl AuditCommand for FakeArchive {
        async fn append_audit(&self, _event: &str, _detail: Option<&str>) {
            if self.crash.load(Ordering::SeqCst) {
                panic!("archive writer crashed");
            }
            self.audit.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn observation_sink(
        archive: Arc<FakeArchive>,
        interval: Duration,
    ) -> Arc<RuntimeObservationSink> {
        let trace_port: Arc<dyn TraceCommand> = archive.clone();
        let audit_port: Arc<dyn AuditCommand> = archive;
        Arc::new(RuntimeObservationSink::with_flush_interval(
            Some(trace_port),
            Some(audit_port),
            interval,
        ))
    }

    #[tokio::test]
    async fn flush_stream_opens_with_the_first_holder_and_closes_with_the_last() {
        let archive = Arc::new(FakeArchive::default());
        let sink = observation_sink(archive, Duration::from_secs(3600));
        assert!(!sink.is_flush_loop_running(), "无持有者时 flush 循环不起");

        let first = sink.subscribe_flush().expect("subscription");
        assert!(sink.frames.is_open(), "首持有者开流");
        assert!(sink.is_flush_loop_running(), "开流即起 flush 循环");

        let second = sink.subscribe_flush().expect("subscription");
        assert_eq!(sink.frames.holders(), 2);

        drop(first);
        assert!(sink.is_flush_loop_running(), "仍有持有者, 循环不关");
        drop(second);
        assert!(!sink.is_flush_loop_running(), "末持有者关停");
        assert!(!sink.frames.is_open());

        // pin 与租约同计数: 只要 pin 在场循环不关。
        let pin = sink.keepalive().expect("pin");
        assert!(sink.is_flush_loop_running());
        let third = sink.subscribe_flush().expect("subscription");
        drop(third);
        assert!(sink.is_flush_loop_running(), "pin 保活");
        drop(pin);
        assert!(!sink.is_flush_loop_running());

        // 关停后不再发放租约。
        sink.frames.shutdown();
        assert!(matches!(
            sink.subscribe_flush(),
            Err(ResourceError::Shutdown(_))
        ));
    }

    #[tokio::test]
    async fn flush_snapshot_reads_the_latest_pass_without_replay() {
        let archive = Arc::new(FakeArchive::default());
        let sink = observation_sink(Arc::clone(&archive), Duration::from_secs(3600));
        let _pin = sink.keepalive().expect("pin");

        sink.emit(trace_event());
        sink.flush().await;
        sink.emit(trace_event());
        sink.emit(trace_event());
        sink.emit(completed_event());
        sink.flush().await;

        // 后订阅者: 快照即读最新一次 pass, 不回放历史 pass。
        let late = sink.subscribe_flush().expect("subscription");
        let report = late.snapshot().value.expect("flush report");
        assert_eq!(report.traces, 2, "快照是最新 pass 的汇总");
        assert_eq!(report.audit, 1);
        assert!(!late.snapshot().has_failure());
    }

    #[tokio::test]
    async fn crashed_flush_pass_becomes_a_failure_frame_and_the_next_pass_clears_it() {
        let archive = Arc::new(FakeArchive::default());
        let sink = observation_sink(Arc::clone(&archive), Duration::from_millis(20));
        let _sub = sink.subscribe_flush().expect("subscription");

        // 写出者崩溃: pass 崩溃即帧 (不静默), 循环本身不死。
        archive.crash.store(true, Ordering::SeqCst);
        sink.emit(trace_event());
        tokio::time::sleep(Duration::from_millis(200)).await;
        let failure = sink.frame_snapshot().failure;
        assert!(
            failure.as_deref().unwrap_or_default().contains("crashed"),
            "崩溃必须成为显式失败帧, got {failure:?}"
        );
        assert!(sink.is_flush_loop_running(), "崩溃不带走 flush 循环");

        // 下一成功 pass 自清失败帧。
        archive.crash.store(false, Ordering::SeqCst);
        sink.emit(trace_event());
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !sink.frame_snapshot().has_failure(),
            "下一成功 pass 自清失败帧"
        );
    }

    #[tokio::test]
    async fn poisoned_observation_buffer_reports_the_crash_and_recovers_the_facts() {
        let archive = Arc::new(FakeArchive::default());
        let sink = observation_sink(Arc::clone(&archive), Duration::from_secs(3600));
        let _pin = sink.keepalive().expect("pin");
        sink.emit(trace_event());

        // 覆盖者崩溃: 缓冲锁中毒 —— 崩溃证据即帧, 内容仍恢复写出。
        let poisoning = {
            let sink = Arc::clone(&sink);
            std::thread::spawn(move || {
                let _guard = sink.traces.lock().unwrap();
                panic!("buffer writer crashed");
            })
        };
        assert!(poisoning.join().is_err());

        sink.emit(trace_event());
        let failure = sink.frame_snapshot().failure;
        assert!(
            failure.as_deref().unwrap_or_default().contains("poisoned"),
            "中毒缓冲必须成为显式失败帧, got {failure:?}"
        );

        sink.flush().await;
        assert_eq!(
            archive.traces.load(Ordering::SeqCst),
            2,
            "恢复出的事实照常写出, 不静默丢"
        );
    }

    #[tokio::test]
    async fn stopped_flush_loop_leaves_no_ghost_passes() {
        let archive = Arc::new(FakeArchive::default());
        let sink = observation_sink(Arc::clone(&archive), Duration::from_millis(25));
        let sub = sink.subscribe_flush().expect("subscription");

        sink.emit(trace_event());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            archive.traces.load(Ordering::SeqCst) >= 1,
            "开流期间 flush 循环写出事实"
        );

        drop(sub); // 末持有者关停
        assert!(!sink.is_flush_loop_running(), "循环任务必须止步");
        tokio::time::sleep(Duration::from_millis(80)).await; // 在途 pass 收尾
        let settled = archive.traces.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            archive.traces.load(Ordering::SeqCst),
            settled,
            "关停后无幽灵 pass、无幽灵回调"
        );
    }
}
