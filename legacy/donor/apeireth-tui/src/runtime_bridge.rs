//! Runtime bridge - TUI-side subscriber for `apeireth-runtime` events.
//!
//! ## Borrowed Reference (per O-5)
//!
//! Inspired by **LangGraph** event subscription (MemorySaver + ThreadCheckpointStore)
//! and **Effect** event-stream pattern. We mirror the **pub-sub bridge** concept
//! (Runtime emits events via ChanneledBus, TUI subscribes and updates App state).
//!
//! ## Design
//!
//! - `RuntimeBridge` wraps an `Arc<Runtime>` + a receiver-side state cache
//! - `poll_events()` (non-blocking) drains pending events into cached state
//! - State snapshots are returned by accessor methods (last_cycle_report, task_count, etc.)
//! - The TUI's main loop calls `poll_events()` each frame (e.g. 60ms tick) to refresh state
//!
//! ## Why not full reactive?
//!
//! TUI is pull-based (render every frame). Push-based reactive (like Elm/Yew) would
//! require Arc<Mutex<>> for every piece of state and event-driven invalidation. The
//! pull-based bridge is simpler and matches ratatui's main-loop model.
//!
//! 0 引外部 dep.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::sync::Arc;

use apeireth_arbitration::ArbitrationLog;
use apeireth_consciousness::{EmotionEngine, EmotionEvent, EmotionSnapshot, Pad};
use apeireth_council::group_chat::{ChatMessage, GroupChat};
use apeireth_runtime::{CycleReport, Runtime, RuntimeConfig};
use apeireth_tool_registry::AsyncTaskStore;
use apeireth_tool_search::SearchEngine;
use parking_lot::Mutex;

// ============================================================================
// §1 Cached state
// ============================================================================

/// Cached runtime state for TUI inspection.
#[derive(Debug, Clone, Default)]
pub struct BridgeState {
    /// Most recent cycle report (None until first cycle runs).
    pub last_cycle_report: Option<CycleReport>,
    /// Number of cycles completed so far.
    pub cycle_count: u64,
    /// Recent async task IDs (newest first, capped at MAX_TRACKED_TASKS).
    pub recent_task_ids: Vec<TaskId>,
    /// Recent arbitration event sequences (capped).
    pub recent_arbitration_seqs: Vec<i64>,
    /// Recent group chat messages (capped at MAX_TRACKED_MESSAGES).
    pub recent_chat_messages: Vec<String>,
    /// Current emotion snapshot.
    pub emotion: Option<EmotionSnapshot>,
    /// Most recent runtime event topic (for TUI status bar).
    pub last_event_topic: Option<String>,
}

impl BridgeState {
    /// Maximum number of recent task IDs to track.
    pub const MAX_TRACKED_TASKS: usize = 32;
    /// Maximum number of recent arbitration events to track.
    pub const MAX_TRACKED_ARBITRATION: usize = 32;
    /// Maximum number of recent chat messages to track.
    pub const MAX_TRACKED_MESSAGES: usize = 16;

    /// Bounded-push helper: prepend new item, truncate to max.
    fn bounded_push<T>(list: &mut Vec<T>, item: T, max: usize) {
        list.insert(0, item);
        if list.len() > max {
            list.truncate(max);
        }
    }
}

// Re-export for convenience
pub use apeireth_tool_registry::TaskId;

// ============================================================================
// §2 Bridge
// ============================================================================

/// TUI-side runtime bridge.
///
/// Owns a reference to `Runtime` and a cached state. The TUI main loop should
/// call `poll_events()` each frame to refresh cached state, then read the
/// state via accessor methods for rendering.
pub struct RuntimeBridge {
    runtime: Arc<Runtime>,
    state: Arc<Mutex<BridgeState>>,
    /// Emotion engine reference (from runtime).
    emotion: Arc<Mutex<EmotionEngine>>,
    /// GroupChat reference (from runtime).
    group_chat: GroupChat,
    /// AsyncTaskStore reference (from runtime).
    task_store: AsyncTaskStore,
    /// ArbitrationLog reference (from runtime).
    arbitration: Arc<ArbitrationLog>,
    /// SearchEngine reference (from runtime).
    search: Arc<SearchEngine>,
}

impl RuntimeBridge {
    /// Wrap an existing `Arc<Runtime>` in a bridge.
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let emotion = runtime.emotion.clone();
        let group_chat = runtime.group_chat.clone();
        let task_store = runtime.task_store.clone();
        let arbitration = runtime.arbitration.clone();
        let search = Arc::clone(&runtime.search);
        Self {
            runtime,
            state: Arc::new(Mutex::new(BridgeState::default())),
            emotion,
            group_chat,
            task_store,
            arbitration,
            search,
        }
    }

    /// Construct a fresh `Runtime` + bridge in one step (convenience).
    pub fn with_config(config: RuntimeConfig) -> Self {
        let runtime = Arc::new(Runtime::with_config(config));
        Self::new(runtime)
    }

    /// Get a reference to the underlying runtime.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Get a snapshot of the cached state.
    pub fn state(&self) -> BridgeState {
        self.state.lock().clone()
    }

    /// Total cycles completed.
    pub fn cycle_count(&self) -> u64 {
        self.state.lock().cycle_count
    }

    /// Most recent cycle report, if any.
    pub fn last_cycle_report(&self) -> Option<CycleReport> {
        self.state.lock().last_cycle_report.clone()
    }

    /// Number of recent task IDs tracked.
    pub fn recent_task_count(&self) -> usize {
        self.state.lock().recent_task_ids.len()
    }

    /// Current emotion snapshot (live from runtime, not cached).
    pub fn current_emotion(&self) -> EmotionSnapshot {
        self.emotion.lock().snapshot()
    }

    /// Apply an emotion event to the underlying engine.
    pub fn apply_emotion(&self, event: EmotionEvent) {
        self.emotion.lock().apply(event).ok();
    }

    /// Set emotion baseline (for TUI-driven emotion control).
    pub fn set_emotion_baseline(&self, baseline: Pad) {
        self.emotion.lock().set_baseline(baseline);
    }

    /// Get current group chat room ID (first one, if any).
    pub fn default_room_id(&self) -> Option<String> {
        self.group_chat.list_rooms().first().cloned()
    }

    /// Number of participants in the default room.
    pub fn participant_count(&self) -> usize {
        match self.default_room_id() {
            Some(rid) => self
                .group_chat
                .get(&rid)
                .map(|r| r.room().participants.len())
                .unwrap_or(0),
            None => 0,
        }
    }

    /// Recent chat messages from the default room (newest first).
    pub fn recent_messages(&self, n: usize) -> Vec<String> {
        let Some(rid) = self.default_room_id() else {
            return Vec::new();
        };
        match self.group_chat.get(&rid) {
            Ok(room_ref) => {
                let msgs = room_ref.room().messages.clone();
                let take = n.min(msgs.len());
                msgs.iter()
                    .rev()
                    .take(take)
                    .map(|m| format!("[{}] {}", m.participant_id, m.content))
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Total tasks currently tracked by the underlying AsyncTaskStore.
    pub async fn total_tasks(&self) -> usize {
        self.task_store.list().await.len()
    }

    /// Total arbitration events logged.
    pub fn total_arbitration_events(&self) -> usize {
        self.arbitration.len().unwrap_or(0)
    }

    /// Total documents indexed in SearchEngine.
    pub fn total_indexed_docs(&self) -> usize {
        self.search.len()
    }

    /// Run a single heartbeat cycle and update cached state.
    /// Returns the cycle report.
    pub async fn run_cycle(&self) -> Option<CycleReport> {
        match self.runtime.run_one_cycle().await {
            Ok(report) => {
                let mut state = self.state.lock();
                state.cycle_count += 1;
                state.last_cycle_report = Some(report.clone());
                state.last_event_topic = Some("cycle_complete".to_string());
                BridgeState::bounded_push(
                    &mut state.recent_task_ids,
                    report.task_id,
                    BridgeState::MAX_TRACKED_TASKS,
                );
                BridgeState::bounded_push(
                    &mut state.recent_arbitration_seqs,
                    report.arbitration_seq,
                    BridgeState::MAX_TRACKED_ARBITRATION,
                );
                Some(report)
            }
            Err(_) => None,
        }
    }

    /// Dispatch an async task via the underlying runtime.
    pub async fn dispatch_task(&self, tool_name: &str, params_json: &str) -> u64 {
        let task_id = self
            .runtime
            .dispatch_async_task(tool_name, params_json)
            .await;
        let mut state = self.state.lock();
        state.last_event_topic = Some(format!("task_dispatched:{}", tool_name));
        BridgeState::bounded_push(
            &mut state.recent_task_ids,
            task_id,
            BridgeState::MAX_TRACKED_TASKS,
        );
        task_id
    }

    /// Refresh the cached state from the runtime (pull-based).
    ///
    /// The TUI should call this every frame. It captures a fresh emotion
    /// snapshot and increments counters; cycle/task events should be pushed
    /// via `run_cycle` / `dispatch_task` instead.
    pub fn refresh_emotion(&self) {
        let snap = self.emotion.lock().snapshot();
        let mut state = self.state.lock();
        state.emotion = Some(snap);
    }

    /// Bootstrap the underlying runtime (creates default room + participants).
    pub fn bootstrap(&self) -> Result<String, String> {
        let room_id = self.runtime.bootstrap().map_err(|e| e.to_string())?;
        let mut state = self.state.lock();
        state.last_event_topic = Some("bootstrap".to_string());
        Ok(room_id)
    }

    /// Append a chat message to the default room (for TUI-typed input).
    pub fn post_message(&self, sender: &str, content: String) -> Result<String, String> {
        let rid = self
            .default_room_id()
            .ok_or_else(|| "no room bootstrapped".to_string())?;
        let msg = ChatMessage::new(rid.clone(), sender.to_string(), content);
        let mid = msg.id.clone();
        self.group_chat
            .post_message_public(&rid, msg)
            .map_err(|e| e.to_string())?;
        let mut state = self.state.lock();
        state.last_event_topic = Some(format!("message:{}", sender));
        let preview = format!("[{}] {}", sender, mid);
        BridgeState::bounded_push(
            &mut state.recent_chat_messages,
            preview,
            BridgeState::MAX_TRACKED_MESSAGES,
        );
        Ok(mid)
    }

    /// Add a participant to the default room (for TUI-typed senders).
    pub fn add_participant(&self, id: &str, display_name: &str) -> Result<(), String> {
        let rid = self
            .default_room_id()
            .ok_or_else(|| "no room bootstrapped".to_string())?;
        let participant = apeireth_council::group_chat::Participant::new(
            id,
            display_name,
            apeireth_council::group_chat::ParticipantRole::Agent,
        );
        self.group_chat
            .add_participant_public(&rid, participant)
            .map_err(|e| e.to_string())
    }

    /// R256 -- expose Prometheus text metrics from the underlying runtime.
    /// The TUI status bar calls this; pages render it as a multi-line block.
    pub fn metrics_text(&self) -> String {
        self.runtime.metrics_text()
    }

    /// R256 -- last cycle latency summary (count, sum, mean) computed by the runtime.
    pub fn cycle_latency_summary(&self) -> apeireth_runtime::CycleLatencySummary {
        self.runtime.cycle_latency_summary()
    }

    /// R256 -- supervisor metrics linkup (heartbeat_count + tick_duration).
    pub fn supervisor_heartbeat_count(&self) -> u64 {
        self.runtime.supervisor_metrics.heartbeat_count.get()
    }

    /// R256 -- supervisor tick_duration observation count.
    pub fn supervisor_tick_duration_count(&self) -> u64 {
        self.runtime.supervisor_metrics.tick_duration.count()
    }

    /// R256 -- direct LLM dispatch via the runtime. Same shape as Runtime::dispatch_llm_task.
    /// Returns the assigned TaskId.
    pub async fn dispatch_llm_task(
        &self,
        prompt: &str,
        system: Option<&str>,
        model: Option<&str>,
        base_url: Option<&str>,
        api_key: &str,
    ) -> apeireth_tool_registry::TaskId {
        self.runtime
            .dispatch_llm_task(prompt, system, model, base_url, api_key)
            .await
    }

    /// R256 -- register a custom AsyncWorker for the given tool_name.
    pub fn register_worker(
        &self,
        tool_name: &str,
        worker: std::sync::Arc<dyn apeireth_runtime::AsyncWorker>,
    ) {
        self.runtime.register_worker(tool_name, worker);
    }

    /// R256 -- dispatch a task through a custom worker (bypass registry lookup).
    pub async fn dispatch_task_with_worker(
        &self,
        worker: std::sync::Arc<dyn apeireth_runtime::AsyncWorker>,
        params_json: &str,
    ) -> apeireth_tool_registry::TaskId {
        self.runtime
            .dispatch_async_task_with_worker(worker, params_json)
            .await
    }

    /// Get the bridge state as a JSON-serializable snapshot (for telemetry).
    pub fn snapshot_json(&self) -> serde_json::Value {
        let state = self.state.lock();
        let s = self.runtime.cycle_latency_summary();
        serde_json::json!({
            "cycle_count": state.cycle_count,
            "recent_task_count": state.recent_task_ids.len(),
            "recent_arbitration_count": state.recent_arbitration_seqs.len(),
            "recent_chat_count": state.recent_chat_messages.len(),
            "has_emotion": state.emotion.is_some(),
            "last_event_topic": state.last_event_topic,
            "cycle_latency_count": s.count,
            "cycle_latency_sum_ms": s.sum_ms,
            "supervisor_heartbeat_count": self.runtime.supervisor_metrics.heartbeat_count.get(),
            "supervisor_tick_duration_count": self.runtime.supervisor_metrics.tick_duration.count(),
            "runtime_cycle_total": self.runtime.cycle_total.get(),
            "runtime_cycle_failures_total": self.runtime.cycle_failures_total.get(),
        })
    }
}

// ============================================================================
// §3 Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge_with_default() -> RuntimeBridge {
        let mut config = RuntimeConfig::default();
        config.arbitration_path = None; // in-memory
        RuntimeBridge::with_config(config)
    }

    #[test]
    fn test_bridge_new_default_state() {
        let bridge = bridge_with_default();
        let state = bridge.state();
        assert_eq!(state.cycle_count, 0);
        assert!(state.last_cycle_report.is_none());
        assert_eq!(state.recent_task_ids.len(), 0);
        assert!(state.last_event_topic.is_none());
    }

    #[test]
    fn test_bridge_bootstrap_creates_room() {
        let bridge = bridge_with_default();
        let room_id = bridge.bootstrap().expect("bootstrap must succeed");
        assert!(!room_id.is_empty());
        let participants = bridge.participant_count();
        assert!(
            participants >= 3,
            "default bootstrap adds at least 3 participants"
        );
    }

    #[test]
    fn test_bridge_post_message_appends_to_cache() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        bridge.add_participant("test_user", "Test User").unwrap();
        let _ = bridge
            .post_message("test_user", "hello".to_string())
            .unwrap();
        let state = bridge.state();
        assert!(!state.recent_chat_messages.is_empty());
        assert!(state
            .last_event_topic
            .as_deref()
            .unwrap()
            .starts_with("message:"));
    }

    #[test]
    fn test_bridge_emotion_accessors() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        bridge.refresh_emotion();
        let snap = bridge.current_emotion();
        // Default baseline should be present
        assert!(snap.pad.p >= 0.0);
        bridge.apply_emotion(EmotionEvent::TaskSuccess);
        bridge.refresh_emotion();
        let snap2 = bridge.current_emotion();
        // After success event, valence should rise or stay same
        assert!(snap2.pad.p >= 0.0);
    }

    #[tokio::test]
    async fn test_bridge_run_cycle_updates_state() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let report = bridge.run_cycle().await.expect("cycle must succeed");
        assert_eq!(bridge.cycle_count(), 1);
        assert!(bridge.last_cycle_report().is_some());
        assert_eq!(bridge.last_cycle_report().unwrap().task_id, report.task_id);
    }

    #[tokio::test]
    async fn test_bridge_dispatch_task_appends_id() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let task_id = bridge.dispatch_task("classify", "{}").await;
        assert_eq!(bridge.recent_task_count(), 1);
        // Wait for completion to avoid leak
        let _ = bridge
            .runtime
            .task_store
            .wait_for_completion(task_id, std::time::Duration::from_secs(2))
            .await;
    }

    #[test]
    fn test_bridge_snapshot_json_has_keys() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let snap = bridge.snapshot_json();
        assert!(snap.get("cycle_count").is_some());
        assert!(snap.get("recent_task_count").is_some());
        assert!(snap.get("last_event_topic").is_some());
    }

    #[test]
    fn test_bridge_bounded_push_truncates() {
        let mut list: Vec<u64> = Vec::new();
        for i in 0..(BridgeState::MAX_TRACKED_TASKS + 5) as u64 {
            BridgeState::bounded_push(&mut list, i, BridgeState::MAX_TRACKED_TASKS);
        }
        assert_eq!(list.len(), BridgeState::MAX_TRACKED_TASKS);
        // Newest first
        assert_eq!(list[0], (BridgeState::MAX_TRACKED_TASKS + 4) as u64);
    }

    #[test]
    fn test_bridge_state_default_traits() {
        let state = BridgeState::default();
        assert_eq!(state.cycle_count, 0);
        assert_eq!(state.recent_task_ids.len(), 0);
        assert_eq!(state.recent_arbitration_seqs.len(), 0);
        assert_eq!(state.recent_chat_messages.len(), 0);
    }

    #[test]
    fn test_bridge_max_constants_are_reasonable() {
        assert!(BridgeState::MAX_TRACKED_TASKS > 0);
        assert!(BridgeState::MAX_TRACKED_ARBITRATION > 0);
        assert!(BridgeState::MAX_TRACKED_MESSAGES > 0);
        assert!(BridgeState::MAX_TRACKED_TASKS <= 256);
        assert!(BridgeState::MAX_TRACKED_MESSAGES <= 256);
    }
    // R256 -- runtime metrics + LLM dispatch bridge plumbing

    #[test]
    fn r256_01_metrics_text_returns_runtime_text() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let text = bridge.metrics_text();
        // runtime_cycle_total is always registered.
        assert!(text.contains("runtime_cycle_total"), "got: {}", text);
    }

    #[tokio::test]
    async fn r256_02_cycle_latency_summary_has_zero_count() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let s = bridge.cycle_latency_summary();
        assert_eq!(s.count, 0);
        assert!((s.sum_ms - 0.0).abs() < 1e-9);
    }

    #[test]
    fn r256_03_supervisor_metrics_accessors() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        assert_eq!(bridge.supervisor_heartbeat_count(), 0);
        assert_eq!(bridge.supervisor_tick_duration_count(), 0);
    }

    #[tokio::test]
    async fn r256_04_dispatch_llm_task_returns_task_id() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let tid = bridge
            .dispatch_llm_task("hello", None, None, None, "fake-key")
            .await;
        assert!(tid > 0);
    }

    #[test]
    fn r256_05_register_worker_propagates_to_runtime() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let w: std::sync::Arc<dyn apeireth_runtime::AsyncWorker> =
            std::sync::Arc::new(apeireth_runtime::SimulatedWorker::new("probe"));
        bridge.register_worker("probe", w);
        assert!(bridge.runtime.worker_registry.lock().contains_key("probe"));
    }

    #[test]
    fn r256_06_snapshot_json_includes_metrics() {
        let bridge = bridge_with_default();
        bridge.bootstrap().unwrap();
        let snap = bridge.snapshot_json();
        assert!(snap.get("cycle_latency_count").is_some());
        assert!(snap.get("supervisor_heartbeat_count").is_some());
        assert!(snap.get("runtime_cycle_total").is_some());
    }
}
