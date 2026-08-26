//! Sessions: the conversation the runtime is orchestrating over.
//!
//! A session owns the transcript and nothing else. It has no emotion, no memory
//! policy, no persona — those are companion concerns that consume a session, not
//! parts of one. The nested `reconstruction_v2` prototype fused them, which is
//! why its session type could not be reused by anything that was not the
//! companion.
//!
//! [`SessionStore`] is async because every real backend is. Retrofitting async
//! into a synchronous trait later would break every caller in the runtime, so it
//! is async from the start even though the only implementation here is in-memory.

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{
    ApprovalId, CapabilityId, Clock, RequestId, SessionId, Timestamp, TraceId,
};
use apeireth_protocol::canonical::NormalizedMessage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::approval::PendingApproval;
use super::error::{RuntimeError, RuntimeResult};

/// One conversation.
#[derive(Debug, Clone)]
pub struct Session {
    /// Stable identity.
    pub id: SessionId,
    /// The transcript, in order. Includes assistant tool-call messages and
    /// tool-result messages, so a resumed session can continue mid-tool-loop.
    pub messages: Vec<NormalizedMessage>,
    /// Structured execution facts needed to reconstruct denied and failed
    /// turns without polluting the provider transcript.
    pub events: Vec<SessionEvent>,
    /// Every approval this session has produced, keyed by stable [`ApprovalId`].
    /// Terminal approvals are retained for audit and idempotency.
    pub approvals: BTreeMap<ApprovalId, PendingApproval>,
    /// The currently active pending approval, when the session is paused.
    pub active_approval_id: Option<ApprovalId>,
    /// Monotonic state revision. It advances whenever a message or event is
    /// appended, including on a failed turn.
    pub revision: u64,
    /// When the session was created.
    pub created_at: Timestamp,
    /// When it was last written to.
    pub updated_at: Timestamp,
}

impl Session {
    /// A new, empty session.
    pub fn new(id: SessionId, clock: &dyn Clock) -> Self {
        let now = Timestamp::from_clock(clock);
        Self {
            id,
            messages: Vec::new(),
            events: Vec::new(),
            approvals: BTreeMap::new(),
            active_approval_id: None,
            revision: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Append a message and update the modification time.
    pub fn append(&mut self, message: NormalizedMessage, clock: &dyn Clock) {
        self.messages.push(message);
        self.touch(clock);
    }

    /// Append several messages, touching the modification time once.
    pub fn extend(
        &mut self,
        messages: impl IntoIterator<Item = NormalizedMessage>,
        clock: &dyn Clock,
    ) {
        self.messages.extend(messages);
        self.touch(clock);
    }

    /// Append one structured execution event.
    pub fn record(
        &mut self,
        request: RequestId,
        trace: TraceId,
        event: SessionEventKind,
        clock: &dyn Clock,
    ) {
        self.events.push(SessionEvent {
            at: Timestamp::from_clock(clock),
            request,
            trace,
            event,
        });
        self.touch(clock);
    }

    /// Number of messages in the transcript.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the transcript is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    fn touch(&mut self, clock: &dyn Clock) {
        self.revision = self.revision.saturating_add(1);
        self.updated_at = Timestamp::from_clock(clock);
    }
}

/// One persisted execution fact for a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// When it happened according to the runtime clock.
    pub at: Timestamp,
    /// Inbound request that caused it.
    pub request: RequestId,
    /// Execution trace that caused it.
    pub trace: TraceId,
    /// Structured event payload.
    pub event: SessionEventKind,
}

/// Persisted execution outcomes. No variant carries model chain-of-thought.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionEventKind {
    /// A user turn entered canonical execution.
    TurnStarted,
    /// Governance denied an action.
    GovernanceDenied {
        /// Hook that decided.
        hook: String,
        /// Stable action label.
        action: String,
        /// Stated denial reason.
        reason: String,
        /// Runtime round.
        round: u32,
    },
    /// Governance suspended an action for human approval.
    ApprovalRequired {
        /// Hook that decided.
        hook: String,
        /// Stable action label.
        action: String,
        /// What needs approval.
        reason: String,
        /// Runtime round.
        round: u32,
        /// The stable approval id that was minted for this pause.
        approval_id: ApprovalId,
    },
    /// A pending approval reached a terminal decision.
    ApprovalResolved {
        /// The stable approval id.
        approval_id: ApprovalId,
        /// `approved` / `rejected` / `expired`.
        decision: String,
        /// Round that produced the approval.
        round: u32,
        /// Optional human reason recorded at resolution time.
        human_reason: Option<String>,
    },
    /// Provider routing could not serve a round.
    ProviderFailed {
        /// Legible terminal routing/provider error.
        error: String,
        /// Runtime round.
        round: u32,
    },
    /// A requested tool did not complete successfully.
    ToolFailed {
        /// Resolved capability, absent when the model named no known tool.
        capability: Option<CapabilityId>,
        /// Model tool-call correlation id.
        tool_call_id: String,
        /// Legible failure.
        error: String,
        /// Runtime round.
        round: u32,
    },
    /// Execution failed outside provider/tool/governance handling.
    ExecutionFailed {
        /// Stable phase label.
        phase: String,
        /// Legible failure.
        error: String,
    },
    /// The turn reached a final assistant response.
    TurnCompleted {
        /// Provider round-trips taken.
        rounds: u32,
    },
}

/// Where sessions live.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load a session, or `None` if it does not exist.
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>>;

    /// Persist a session, creating or replacing it.
    async fn save(&self, session: &Session) -> RuntimeResult<()>;
}

/// A session store held in process memory.
///
/// Real durability belongs to `apeireth-storage`, which is out of scope for this
/// phase. This exists so the runtime can be composed and tested end to end
/// without a database, and so that the seam a database will slot into is real
/// rather than hypothetical.
#[derive(Debug, Default)]
pub struct InMemorySessionStore {
    sessions: Mutex<BTreeMap<SessionId, Session>>,
}

impl InMemorySessionStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many sessions are held.
    pub async fn len(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// Whether the store is empty.
    pub async fn is_empty(&self) -> bool {
        self.sessions.lock().await.is_empty()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        Ok(self.sessions.lock().await.get(id).cloned())
    }

    async fn save(&self, session: &Session) -> RuntimeResult<()> {
        self.sessions
            .lock()
            .await
            .insert(session.id, session.clone());
        Ok(())
    }
}

/// Loads, creates, and persists sessions against a [`SessionStore`].
///
/// Owns the clock so that a session's timestamps come from the same source as
/// the rest of the runtime's, rather than from `Utc::now()` scattered across
/// call sites.
pub struct SessionManager {
    store: Arc<dyn SessionStore>,
    clock: Arc<dyn Clock>,
}

impl SessionManager {
    /// Build a manager over a store.
    pub fn new(store: Arc<dyn SessionStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// Load `id`, returning `None` when it does not exist.
    pub async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        self.store
            .load(id)
            .await
            .map_err(|e| RuntimeError::session_load(*id, e.to_string()))
    }

    /// Load `id`, creating an empty session if it does not exist yet.
    pub async fn load_or_create(&self, id: SessionId) -> RuntimeResult<Session> {
        match self.store.load(&id).await {
            Ok(Some(session)) => Ok(session),
            Ok(None) => Ok(Session::new(id, self.clock.as_ref())),
            Err(e) => Err(RuntimeError::session_load(id, e.to_string())),
        }
    }

    /// Persist a session.
    pub async fn save(&self, session: &Session) -> RuntimeResult<()> {
        self.store
            .save(session)
            .await
            .map_err(|e| RuntimeError::session_save(session.id, e.to_string()))
    }

    /// The clock this manager stamps sessions with.
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::VirtualClock;

    fn clock() -> Arc<dyn Clock> {
        Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ))
    }

    #[tokio::test]
    async fn a_missing_session_is_created_rather_than_erroring() {
        let manager = SessionManager::new(Arc::new(InMemorySessionStore::new()), clock());
        let id = SessionId::new();

        let session = manager.load_or_create(id).await.unwrap();
        assert_eq!(session.id, id);
        assert!(session.is_empty());
    }

    #[tokio::test]
    async fn a_saved_session_round_trips_with_its_transcript() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());
        let id = SessionId::new();

        let mut session = manager.load_or_create(id).await.unwrap();
        session.append(NormalizedMessage::user("hello"), clock().as_ref());
        session.append(NormalizedMessage::assistant("hi"), clock().as_ref());
        manager.save(&session).await.unwrap();

        let reloaded = manager.load_or_create(id).await.unwrap();
        assert_eq!(reloaded.len(), 2);
        assert!(
            matches!(
                &reloaded.messages[0].content[0],
                apeireth_protocol::canonical::ContentPart::Text { text } if text == "hello"
            ),
            "the transcript must survive the round trip intact"
        );
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn saving_twice_replaces_rather_than_duplicating() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());
        let id = SessionId::new();

        let mut session = manager.load_or_create(id).await.unwrap();
        session.append(NormalizedMessage::user("one"), clock().as_ref());
        manager.save(&session).await.unwrap();
        session.append(NormalizedMessage::user("two"), clock().as_ref());
        manager.save(&session).await.unwrap();

        assert_eq!(store.len().await, 1);
        assert_eq!(manager.load_or_create(id).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn messages_and_events_advance_one_monotonic_revision() {
        let clock = clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        let request = RequestId::new();
        let trace = TraceId::new();

        session.append(NormalizedMessage::user("attempt"), clock.as_ref());
        session.record(
            request,
            trace,
            SessionEventKind::ProviderFailed {
                error: "offline".into(),
                round: 1,
            },
            clock.as_ref(),
        );

        assert_eq!(session.revision, 2);
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.events.len(), 1);
        assert_eq!(session.events[0].request, request);
        assert_eq!(session.events[0].trace, trace);
    }

    #[tokio::test]
    async fn appending_advances_the_modification_time_from_the_injected_clock() {
        let virtual_clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );
        let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        let created = session.created_at;

        virtual_clock.advance(chrono::Duration::seconds(30));
        session.append(NormalizedMessage::user("later"), clock.as_ref());

        assert_eq!(session.created_at, created, "creation time must not move");
        assert_eq!(
            session.updated_at.epoch_millis() - created.epoch_millis(),
            30_000
        );
    }

    #[tokio::test]
    async fn two_sessions_do_not_share_a_transcript() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());

        let mut a = manager.load_or_create(SessionId::new()).await.unwrap();
        let b = manager.load_or_create(SessionId::new()).await.unwrap();
        a.append(NormalizedMessage::user("only in a"), clock().as_ref());
        manager.save(&a).await.unwrap();
        manager.save(&b).await.unwrap();

        assert_eq!(manager.load_or_create(a.id).await.unwrap().len(), 1);
        assert_eq!(manager.load_or_create(b.id).await.unwrap().len(), 0);
        assert_eq!(store.len().await, 2);
    }
}
