//! `apeireth-runtime` - **Apeireth R147 end-to-end runtime orchestration**
//!
//! **Goal**: Unify 7 critical modules into a single living runtime. Before R147 each
//! module had rich standalone semantics but no end-to-end driver. This crate is the
//! concrete driver that proves the modules cooperate correctly.
//!
//! **7 modules wired together**:
//! 1. `HeartbeatScheduler` (`apeireth-supervisor`)        - time-driven tick source
//! 2. `AsyncTaskStore`     (`apeireth-tool-registry`)     - pending -> running -> completed
//! 3. `ChanneledBus`       (`apeireth-bus`)               - 3-channel fan-out publish/subscribe
//! 4. `ArbitrationLog`     (`apeireth-arbitration`)       - HASH-SQL append-only canonical timeline
//! 5. `SearchEngine`       (`apeireth-tool-search`)       - inverted-index full-text recall
//! 6. `GroupChat`          (`apeireth-council`)           - multi-agent room
//! 7. `EmotionEngine`      (`apeireth-consciousness`)     - PAD emotional resonance
//!
//! **End-to-end orchestration loop**:
//! ```text
//!   HeartbeatScheduler (Time tick)
//!     -> dispatch_async_task("classify", payload)
//!        -> AsyncTaskStore.register(Pending)
//!        -> worker (spawn) -> mark_running -> simulate progress -> complete(result)
//!        -> ChanneledBus.publish_multi(ChannelSet::BOTH, "async_result", msg)
//!     -> ArbitrationLog.append(EventSource::AgentComm, "runtime", "task_complete", payload)
//!     -> SearchEngine.index(doc)
//!     -> GroupChat.post(ChatMessage)
//!     -> EmotionEngine.apply(EmotionEvent::TaskSuccess)
//! ```
//!
//! **Compile-time guard**: `MODULES_ORCHESTRATED = 7`. Adding an 8th requires bumping it.

#![deny(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use apeireth_arbitration::{ArbitrationLog, EventSource};
use apeireth_consciousness::{BaseEmotion, EmotionEngine, EmotionSnapshot, Pad};
use apeireth_council::group_chat::{ChatMessage, GroupChat, Participant, ParticipantRole, TurnPolicy};
use apeireth_bus::{BusMessage, ChanneledBus, ChannelSet};
use apeireth_supervisor::{Heartbeat, HeartbeatPriority, HeartbeatScheduler, Schedule, WakeupContext, WakeupSource};
use apeireth_tool_registry::{AsyncTaskStore, NotifyChannel, TaskId, TaskStatus};
use apeireth_tool_search::{Document, SearchEngine};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;

pub const MODULES_ORCHESTRATED: usize = 7;
pub const DEFAULT_TICK_INTERVAL_SECS: u64 = 10;
pub const DEFAULT_ROOM_CAPACITY: usize = 16;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("arbitration error: {0}")]
    Arbitration(#[from] apeireth_arbitration::ArbError),
    #[error("runtime already started")]
    AlreadyStarted,
    #[error("runtime not started")]
    NotStarted,
    #[error("task error: {0}")]
    Task(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("group chat error: {0}")]
    GroupChat(String),
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub trace_id: u64,
    pub task_id: Option<TaskId>,
    pub tool_name: String,
    pub topic: String,
    pub payload_json: String,
    pub timestamp_ms: i64,
}

impl RuntimeEvent {
    pub fn new(
        trace_id: u64,
        task_id: Option<TaskId>,
        tool_name: impl Into<String>,
        topic: impl Into<String>,
        payload_json: impl Into<String>,
    ) -> Self {
        Self {
            trace_id,
            task_id,
            tool_name: tool_name.into(),
            topic: topic.into(),
            payload_json: payload_json.into(),
            timestamp_ms: now_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleReport {
    pub trace_id: u64,
    pub task_id: TaskId,
    pub arbitration_seq: i64,
    pub search_doc_id: u64,
    pub group_chat_message_id: String,
    pub emotion_dominant: BaseEmotion,
    pub emotion_intensity: f32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub tick_interval: Duration,
    pub room_name: String,
    pub room_topic: String,
    pub arbitration_path: Option<std::path::PathBuf>,
    pub emit_bus: bool,
    pub room_capacity: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(DEFAULT_TICK_INTERVAL_SECS),
            room_name: "runtime-living-day".into(),
            room_topic: "apeireth autonomous day".into(),
            arbitration_path: None,
            emit_bus: true,
            room_capacity: DEFAULT_ROOM_CAPACITY,
        }
    }
}

#[async_trait::async_trait]
pub trait AsyncWorker: Send + Sync + 'static {
    fn name(&self) -> &str;
    async fn execute(&self, task_id: TaskId, params_json: String) -> Result<String, String>;
}

pub struct SimulatedWorker {
    name: String,
}

impl SimulatedWorker {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl AsyncWorker for SimulatedWorker {
    fn name(&self) -> &str { &self.name }
    async fn execute(&self, task_id: TaskId, params_json: String) -> Result<String, String> {
        sleep(Duration::from_millis(20)).await;
        let result = serde_json::json!({
            "task_id": task_id,
            "tool": self.name,
            "input": params_json,
            "output": "ok-simulated",
            "completed_at_ms": now_ms(),
        });
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
}

pub struct LivingCycleHeartbeat {
    runtime: Arc<Runtime>,
}

impl LivingCycleHeartbeat {
    pub fn new(runtime: Arc<Runtime>) -> Self { Self { runtime } }
}

#[async_trait::async_trait]
impl Heartbeat for LivingCycleHeartbeat {
    fn id(&self) -> &str { "living_cycle" }
    fn priority(&self) -> HeartbeatPriority { HeartbeatPriority::Normal }
    fn accepts(&self) -> Vec<WakeupSource> {
        vec![WakeupSource::Time, WakeupSource::Event, WakeupSource::User]
    }
    async fn on_tick(&self, _ctx: &WakeupContext) -> apeireth_supervisor::HeartbeatResult<()> {
        let _ = self.runtime.run_one_cycle().await;
        Ok(())
    }
    async fn on_event(&self, _ctx: &WakeupContext) -> apeireth_supervisor::HeartbeatResult<()> {
        let _ = self.runtime.run_one_cycle().await;
        Ok(())
    }
    async fn on_user(&self, _ctx: &WakeupContext) -> apeireth_supervisor::HeartbeatResult<()> {
        let _ = self.runtime.run_one_cycle().await;
        Ok(())
    }
}

pub struct Runtime {
    pub scheduler: HeartbeatScheduler,
    pub task_store: AsyncTaskStore,
    pub bus: ChanneledBus<RuntimeEvent>,
    pub arbitration: Arc<ArbitrationLog>,
    pub search: Arc<SearchEngine>,
    pub group_chat: GroupChat,
    pub emotion: Arc<Mutex<EmotionEngine>>,
    pub config: RuntimeConfig,
    started: Arc<Mutex<bool>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        let arbitration = match &config.arbitration_path {
            Some(p) => ArbitrationLog::open(p).expect("open arbitration log"),
            None => ArbitrationLog::open_in_memory().expect("open in-memory arbitration"),
        };
        Self {
            scheduler: HeartbeatScheduler::new(),
            task_store: AsyncTaskStore::new(),
            bus: ChanneledBus::new(),
            arbitration: Arc::new(arbitration),
            search: Arc::new(SearchEngine::new()),
            group_chat: GroupChat::new(),
            emotion: Arc::new(Mutex::new(EmotionEngine::new())),
            config,
            started: Arc::new(Mutex::new(false)),
        }
    }

    pub fn bootstrap(&self) -> RuntimeResult<String> {
        let room_id = self.group_chat.create_room(
            self.config.room_name.clone(),
            self.config.room_topic.clone(),
            TurnPolicy::Free,
        );
        self.add_participant(&room_id, Participant::new("host_apeireth", "Apeireth", ParticipantRole::Host))?;
        self.add_participant(&room_id, Participant::new("agent_classifier", "Classifier", ParticipantRole::Agent))?;
        self.add_participant(&room_id, Participant::new("agent_searcher", "Searcher", ParticipantRole::Agent))?;
        Ok(room_id)
    }

    fn add_participant(&self, room_id: &str, p: Participant) -> RuntimeResult<()> {
        self.group_chat
            .add_participant_public(room_id, p)
            .map_err(|e| RuntimeError::GroupChat(e.to_string()))
    }

    pub async fn dispatch_async_task(&self, tool_name: &str, params_json: &str) -> TaskId {
        let worker = SimulatedWorker::new(tool_name);
        let tool = worker.name().to_string();
        let (task_id, _rec) = self.task_store.register(
            tool.clone(),
            params_json.to_string(),
            NotifyChannel::Both,
        ).await;
        self.task_store.mark_running(task_id).await.expect("mark running");
        let store = self.task_store.clone();
        let bus = self.bus.clone();
        let arbitration = self.arbitration.clone();
        let search = Arc::clone(&self.search);
        let group_chat = self.group_chat.clone();
        let emotion = self.emotion.clone();
        let emit_bus = self.config.emit_bus;
        let tool_name_owned = tool.clone();
        let params_owned: String = params_json.to_string();
        tokio::spawn(async move {
            let result = worker.execute(task_id, params_owned).await;
            match result {
                Ok(json) => {
                    let snap = match store.complete(task_id, json.clone()).await {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    if emit_bus {
                        let event = RuntimeEvent::new(
                            snap.task_id,
                            Some(snap.task_id),
                            tool_name_owned.clone(),
                            "task_completed",
                            json.clone(),
                        );
                        let msg = BusMessage::new(event);
                        let _ = bus.publish_multi(ChannelSet::BOTH, "async_result", msg).await;
                    }
                    let _ = arbitration.append(
                        EventSource::AgentComm,
                        "runtime",
                        "task_completed",
                        serde_json::to_string(&snap).unwrap_or_default(),
                    );
                    let doc = Document::new(0, "runtime", &tool_name_owned, &json);
                    search.index(doc);
                    if let Some(rid) = group_chat.list_rooms().first().cloned() {
                        let _ = group_chat.post_message_public(
                            &rid,
                            ChatMessage::new(
                                rid.clone(),
                                "agent_classifier",
                                format!("[{}] task #{} done: {}", tool_name_owned, task_id, json),
                            ),
                        );
                    }
                    emotion.lock().apply(EmotionEvent::TaskSuccess).ok();
                }
                Err(err) => {
                    let _ = store.fail(task_id, err.clone()).await;
                    emotion.lock().apply(EmotionEvent::ToolError).ok();
                }
            }
        });
        task_id
    }

    pub async fn run_one_cycle(&self) -> RuntimeResult<CycleReport> {
        let start = now_ms();
        let trace_id = apeireth_bus::next_trace_id();
        let task_id = self.dispatch_async_task("classify", "{}").await;
        let task = self
            .task_store
            .wait_for_completion(task_id, Duration::from_secs(5))
            .await
            .map_err(|e| RuntimeError::Task(e.to_string()))?;
        let arb_event = self.arbitration.append(
            EventSource::AgentComm,
            "runtime",
            "task_complete",
            serde_json::to_string(&task).unwrap_or_default(),
        )?;
        let doc = Document::new(
            0,
            "runtime",
            &task.tool_name,
            &format!("{} {}", task.params_json, task.result_json.clone().unwrap_or_default()),
        );
        let doc_id = self.search.index(doc);
        let room_id = self
            .group_chat
            .list_rooms()
            .first()
            .cloned()
            .ok_or_else(|| RuntimeError::GroupChat("no group chat room bootstrapped".into()))?;
        let msg = ChatMessage::new(
            room_id.clone(),
            "host_apeireth",
            format!("[cycle] task #{} status={}", task_id, task.status.as_str()),
        );
        let mid = msg.id.clone();
        self.group_chat.post_message_public(&room_id, msg).map_err(|e| RuntimeError::GroupChat(e.to_string()))?;
        let event = match task.status {
            TaskStatus::Completed => EmotionEvent::TaskSuccess,
            TaskStatus::Failed => EmotionEvent::TaskFailure,
            _ => EmotionEvent::ToolOk,
        };
        self.emotion.lock().apply(event).ok();
        let snap = self.emotion.lock().snapshot();
        let elapsed_ms = (now_ms() - start) as u64;
        Ok(CycleReport {
            trace_id,
            task_id,
            arbitration_seq: arb_event.seq,
            search_doc_id: doc_id,
            group_chat_message_id: mid,
            emotion_dominant: snap.dominant,
            emotion_intensity: snap.intensity,
            elapsed_ms,
        })
    }

    pub async fn wake(&self, source: WakeupSource, topic: impl Into<String>) -> RuntimeResult<usize> {
        let ctx = WakeupContext::new(source, topic, "{}");
        self.scheduler
            .trigger(ctx)
            .await
            .map_err(|e| RuntimeError::Task(e.to_string()))
    }

    pub async fn start(self: Arc<Self>) -> RuntimeResult<()> {
        let started_val = *self.started.lock();
        if started_val {
            return Err(RuntimeError::AlreadyStarted);
        }
        self.bootstrap()?;
        let hb = LivingCycleHeartbeat::new(self.clone());
        self.scheduler
            .register_interval(hb, Schedule::every(self.config.tick_interval))
            .await
            .map_err(|e| RuntimeError::Task(e.to_string()))?;
        self.scheduler
            .start()
            .await
            .map_err(|e| RuntimeError::Task(e.to_string()))?;
        *self.started.lock() = true;
        Ok(())
    }

    pub async fn shutdown(self: Arc<Self>) -> RuntimeResult<()> {
        self.scheduler
            .stop()
            .await
            .map_err(|e| RuntimeError::Task(e.to_string()))?;
        *self.started.lock() = false;
        Ok(())
    }

    pub async fn tick_count(&self) -> u64 {
        self.scheduler.tick_count().await
    }

    pub fn apply_emotion(&self, event: EmotionEvent) {
        self.emotion.lock().apply(event).ok();
    }

    pub fn emotion_snapshot(&self) -> EmotionSnapshot {
        self.emotion.lock().snapshot()
    }

    pub fn set_emotion_baseline(&self, baseline: Pad) {
        self.emotion.lock().set_baseline(baseline);
    }
}

impl Default for Runtime {
    fn default() -> Self { Self::new() }
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_modules_count() {
        assert_eq!(MODULES_ORCHESTRATED, 7);
    }

    #[test]
    fn t02_default_config() {
        let c = RuntimeConfig::default();
        assert_eq!(c.tick_interval, Duration::from_secs(DEFAULT_TICK_INTERVAL_SECS));
        assert_eq!(c.room_capacity, DEFAULT_ROOM_CAPACITY);
    }

    #[tokio::test]
    async fn t03_runtime_new_and_bootstrap() {
        let rt = Runtime::new();
        let room_id = rt.bootstrap().unwrap();
        assert_eq!(rt.group_chat.room_count(), 1);
        assert!(rt.group_chat.list_rooms().contains(&room_id));
    }

    #[tokio::test]
    async fn t04_run_one_cycle_end_to_end() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let report = rt.run_one_cycle().await.expect("cycle");
        assert!(report.task_id > 0);
        assert!(report.arbitration_seq > 0);
        assert!(report.search_doc_id > 0);
        assert!(!report.group_chat_message_id.is_empty());
        assert!(report.elapsed_ms < 5_000);
    }

    #[tokio::test]
    async fn t05_dispatch_async_task_returns_id() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let tid = rt.dispatch_async_task("test_tool", "{\"k\":1}").await;
        assert!(tid > 0);
        let rec = rt.task_store.wait_for_completion(tid, Duration::from_secs(3)).await.unwrap();
        assert_eq!(rec.status, TaskStatus::Completed);
        assert!(rec.result_json.is_some());
    }

    #[tokio::test]
    async fn t06_search_indexes_runtime_output() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let _ = rt.run_one_cycle().await.unwrap();
        let hits = rt.search.search("simulated", 10).unwrap();
        assert!(!hits.is_empty(), "search should find simulated worker output");
    }

    #[tokio::test]
    async fn t07_arbitration_records_events() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let _ = rt.run_one_cycle().await.unwrap();
        let n = rt.arbitration.len().unwrap();
        assert!(n >= 2, "expected >=2 arbitration events, got {}", n);
    }

    #[tokio::test]
    async fn t08_group_chat_receives_message() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let _ = rt.run_one_cycle().await.unwrap();
        let room_id = rt.group_chat.list_rooms().into_iter().next().unwrap();
        let room = rt.group_chat.get(&room_id).unwrap();
        assert!(room.room().message_count() >= 1);
    }

    #[tokio::test]
    async fn t09_emotion_changes_after_task() {
        let rt = Runtime::new();
        rt.bootstrap().unwrap();
        let before = rt.emotion_snapshot();
        let _ = rt.run_one_cycle().await.unwrap();
        let after = rt.emotion_snapshot();
        assert!(after.pad.p >= before.pad.p);
    }

    #[tokio::test]
    async fn t10_wake_scheduler_fires_cycle() {
        let rt = Arc::new(Runtime::new());
        rt.clone().start().await.unwrap();
        let fired = rt.wake(WakeupSource::User, "manual").await.unwrap();
        assert!(fired >= 1);
        rt.clone().shutdown().await.unwrap();
    }
}

pub use apeireth_bus::Channel as BusChannel;
pub use apeireth_bus::ChannelSet as BusChannelSet;
pub use apeireth_bus::next_trace_id;
pub use apeireth_consciousness::EmotionEvent;