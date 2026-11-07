//! Channels to external MCP servers: one JSON-RPC link per configured server.
//!
//! The channel seam is deliberately thin — one request/response and one
//! notification operation — so the connection state machine above it never
//! knows how bytes travel. Three channels implement it:
//!
//! * [`StdioChannel`] — a local child process speaking newline-delimited
//!   JSON-RPC on its standard streams;
//! * [`HttpChannel`] — an HTTP endpoint answering JSON-RPC requests directly;
//! * [`SseChannel`] — an HTTP endpoint carrying server-sent events for
//!   responses plus message posts for requests, fed through the protocol
//!   library's SSE frame parser.
//!
//! [`open_channel`] builds the right channel from a [`McpServerSpec`], and
//! [`McpChannelFactory`] re-opens one per (re)connect so a dropped link is
//! rebuilt instead of reused.
//!
//! # Observability: a frame that is not JSON-RPC
//!
//! A server that speaks a broken frame leaves no trace in the response path:
//! the frame is dropped and the request that waited for it simply never gets
//! an answer. Both streaming channels therefore **count** every dropped frame
//! ([`McpChannel::malformed_frames`]) and write the first drop — plus every
//! hundredth one after it — to stderr. Dropping stays a drop: a bad frame
//! never closes the link.
//!
//! # Observability: a frame that names no event type
//!
//! The protocol library decides what a frame means: a frame is a message
//! only when it names the `message` event type
//! (`apeireth_plugin::mcp::sse::SseFrame::is_message`). A frame that carries
//! a payload but **no** event type therefore reaches neither branch of the
//! read loop: it is dropped **silently** — no count, no log — and the request
//! that waited for it never gets an answer.
//!
//! The event stream therefore **counts** those frames
//! ([`McpChannel::unrouted_frames`]) and writes the first one — plus every
//! hundredth one after it — to stderr. The count is separate from
//! [`McpChannel::malformed_frames`] on purpose: a frame without an event type
//! is not known to be malformed (its payload may be a perfectly valid
//! response), it is a frame the client could not **route**. Dropping stays a
//! drop: such a frame never closes the link, and it is still not a message.
//!
//! The **stdio** face loses frames the same way, and counts them the same
//! way: a server notification (a JSON-RPC message with no `id:`) parses
//! fine but has no caller waiting for it, so it is dropped — and it is
//! **counted** ([`McpChannel::unrouted_frames`]) and logged, exactly like the
//! event stream's unroutable frame.
//!
//! # Consumption: the stream-level fields (`id:` / `retry:`)
//!
//! Two SSE fields carry state that no response path consumes: `id:` (the
//! Last-Event-ID a resumed stream continues from) and `retry:` (the server's
//! reconnection hint). A channel that reads them but nobody consumes leaves
//! them dead on the wire. The channel therefore hands every frame it reads
//! to the observer its owner installed ([`McpFrameObserver`]), and the owner
//! decides what the stream-level state means.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, watch};

use apeireth_plugin::mcp::sse::{absolutize_endpoint, SseBuffer, SseFrame};
use apeireth_plugin::mcp::{Id, JsonRpcRequest, JsonRpcResponse};

use crate::mcp::McpError;

use super::config::{McpServerSpec, McpTransportKind};

/// The transport's own bound on one response wait, for a link that belongs to
/// **no** bridge deadline.
///
/// # The invariant this value serves
///
/// The transport must **never be the stricter** of the two deadlines: a
/// transport that gives up first cuts a call the bridge still allows, and the
/// failure then reads as a transport failure (retryable) instead of the
/// bridge's own `timeout.*` classification.
///
/// A link that belongs to a bridge therefore does **not** use this constant:
/// it uses [`response_deadline_for_call`], which **follows** the bridge's
/// per-call deadline. This constant stays what it has always been — the
/// **handshake tier** (the handshake is the one wait the bridge does not
/// cover) and the bound of a link opened without a bridge.
///
/// # Why this value
///
/// * It is the **same tier** as the bridge's per-call deadline
///   (`DEFAULT_CALL_TIMEOUT_MS`, 30 s): the transport must never be the
///   stricter of the two, or it would cut a call the bridge still allows.
///   The bridge arms its deadline **before** the request leaves, so in the
///   tool-call path the bridge's timer fires first; this bound is what keeps
///   the **handshake** — which the bridge does not cover — from hanging.
/// * It stays **above** the endpoint-announcement bound (10 s): the two waits
///   belong to different phases ("did the stream announce an endpoint" /
///   "did the server answer"), and their failures must stay tellable apart.
///
/// A peer that takes the request and never answers is a broken peer, not a
/// slow one: the wait ends with a transport error, never with a silent retry.
pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// The margin the transport's own bound keeps **above** the bridge's deadline.
///
/// The bridge arms its deadline **before** the request leaves, so its timer is
/// already running when the transport starts waiting — the bridge's timer is
/// the earlier one. The margin turns that ordering into a **strict** one: two
/// timers that expire inside the same millisecond would let the transport's
/// failure win the race, and the classification would degrade back into a
/// transport failure (retryable) instead of the bridge's `timeout.*`.
pub const TRANSPORT_DEADLINE_GRACE: Duration = Duration::from_millis(500);

/// The transport's response deadline for a bridge whose per-call deadline is
/// `call_timeout_ms`.
///
/// # The rule
///
/// The transport's bound **follows** the bridge's per-call deadline:
///
/// * the bridge's deadline is **shorter** ⇒ the bridge's timer fires first
///   (it was armed first) ⇒ the call is classified by the bridge
///   (`timeout.*`), and the transport's bound stays the handshake tier below;
/// * the bridge's deadline is **longer** ⇒ the transport's bound follows it
///   up, so the transport can no longer cut a call the bridge still allows.
///
/// The bound is therefore **never stricter** than the bridge's — that is the
/// invariant the transport's own comment declares — and it is **never below**
/// the handshake tier ([`RESPONSE_TIMEOUT`]): the handshake is the one wait
/// the bridge does not cover, so a tight per-call deadline must not shrink
/// it.
pub fn response_deadline_for_call(call_timeout_ms: u64) -> Duration {
    let bridge = Duration::from_millis(call_timeout_ms);
    let followed = bridge.saturating_add(TRANSPORT_DEADLINE_GRACE);
    followed.max(RESPONSE_TIMEOUT)
}

/// The transport error for a response wait that ran out of time.
///
/// The wording names the **wait** — "did not deliver the response" — so it
/// can be told apart from a connect failure ("opening event stream: …") and
/// from a stream that never announced its endpoint ("event stream did not
/// announce a message endpoint").
fn response_timeout(source: &str, deadline: Duration) -> McpError {
    McpError::Transport(format!(
        "{source} did not deliver the response within {deadline:?}"
    ))
}

/// A sink for the frames a channel reads.
///
/// The channel owns the bytes; the owner of a link owns the meaning. Two
/// event-stream fields carry state that **no response path consumes** —
/// `id:` (the Last-Event-ID a resumed stream continues from) and `retry:`
/// (the server's reconnection hint) — so a channel hands every frame it
/// reads to the observer its owner installed.
///
/// **Observation only**: an observer never changes what the channel does with
/// a frame. The routing decision stays in the channel.
pub trait McpFrameObserver: Send + Sync {
    /// Observe one frame read from the stream.
    fn observe_frame(&self, frame: &SseFrame);
}

/// An observer that ignores every frame.
///
/// The default for a link whose owner has no stream-level state to track.
struct IgnoredFrames;

impl McpFrameObserver for IgnoredFrames {
    fn observe_frame(&self, _frame: &SseFrame) {}
}

/// One JSON-RPC link to one external server.
#[async_trait]
pub trait McpChannel: Send + Sync {
    /// Send one request and await its response.
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

    /// Non-blocking invalidation; explicit close subsequently joins/reaps owned work.
    fn revoke(&self) {}

    /// Close owned transport resources; in-process channels need no cleanup.
    async fn close(&self) -> Result<(), McpError> {
        Ok(())
    }

    /// Send one notification (no response is expected).
    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError>;

    /// How many frames this channel dropped because they were not valid
    /// JSON-RPC responses.
    ///
    /// **Observability, not behaviour**: a dropped frame does **not** close
    /// the link — the next frames are read as usual. The count is what makes
    /// a server that speaks a broken frame visible (the frame itself leaves
    /// no trace in the response path: the request that waited for it simply
    /// never gets an answer).
    ///
    /// `0` = this channel never drops a frame silently (the HTTP channel
    /// answers a malformed body as a transport error instead).
    fn malformed_frames(&self) -> u64 {
        0
    }

    /// How many frames this channel could not **route**.
    ///
    /// **Observability, not behaviour**: such a frame is dropped exactly as
    /// before and the link stays open. What changes is that the drop is
    /// *counted* — without it the frame leaves no trace at all: the client
    /// cannot tell whether the payload was a JSON-RPC response, so the
    /// request that waited for it simply never gets an answer.
    ///
    /// The count is deliberately **separate** from
    /// [`McpChannel::malformed_frames`]: a frame the client could not route
    /// is not known to be malformed (its payload may be a perfectly valid
    /// response) — it is a frame the client could not **route**.
    ///
    /// The two streaming faces count the same kind of loss:
    ///
    /// * the **event-stream** face counts a frame that carried a payload but
    ///   named **no** event type — the protocol library dispatches on the
    ///   event type, so such a frame reaches neither branch;
    /// * the **stdio** face counts a JSON-RPC message that no caller is
    ///   waiting for — a server notification (no `id:`) or a reply whose
    ///   request is already gone.
    ///
    /// Two frame kinds are deliberately **not** counted here:
    ///
    /// * a frame that names an event type — an explicitly typed frame that
    ///   is not `message` is ignored **by design** (event-type dispatch);
    /// * a frame that carries no payload at all — that is a control frame
    ///   (`id:` / `retry:`), not a lost message.
    fn unrouted_frames(&self) -> u64 {
        0
    }
}

/// How many dropped malformed frames pass between two stderr lines.
///
/// A server that speaks a broken frame can flood the stream ⇒ one line per
/// frame would bury the rest of the run. **Every drop is still counted**; only
/// the log is throttled (the first drop, then every [`MALFORMED_FRAME_LOG_EVERY`]-th).
const MALFORMED_FRAME_LOG_EVERY: u64 = 100;

/// How many unroutable frames pass between two stderr lines.
///
/// Same reasoning as [`MALFORMED_FRAME_LOG_EVERY`]: the log is throttled, the
/// count is not.
const UNROUTED_FRAME_LOG_EVERY: u64 = 100;

/// Record one frame that could not be read as a JSON-RPC response.
///
/// **Only observation**: the frame is dropped exactly as before and the link
/// stays open. What changes is that the drop is *counted* (per link, queryable
/// through [`McpChannel::malformed_frames`]) and *logged* (throttled: the
/// first drop plus every [`MALFORMED_FRAME_LOG_EVERY`]-th one).
///
/// The line carries the running count and the parse error, **not the frame
/// payload**: a broken frame may still carry server-side content.
fn note_malformed_frame(counter: &AtomicU64, source: &str, error: &serde_json::Error) {
    let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
    if count == 1 || count % MALFORMED_FRAME_LOG_EVERY == 0 {
        eprintln!(
            "[mcp] {source}: frame is not a valid JSON-RPC response (dropped, {count} so far): {error}"
        );
    }
}

/// Record one frame that carried a payload but named no event type.
///
/// **Only observation**: the frame is dropped exactly as before (it is not a
/// message, so it is not routed) and the link stays open. What changes is
/// that the drop is *counted* (per link, queryable through
/// [`McpChannel::unrouted_frames`]) and *logged* (throttled: the first drop
/// plus every [`UNROUTED_FRAME_LOG_EVERY`]-th one).
///
/// The line carries the running count only, **not the frame payload**: the
/// payload may still carry server-side content.
fn note_unrouted_frame(counter: &AtomicU64, source: &str) {
    let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
    if count == 1 || count % UNROUTED_FRAME_LOG_EVERY == 0 {
        eprintln!(
            "[mcp] {source}: frame carries a payload but no event type (dropped, {count} so far)"
        );
    }
}

/// Record one frame the stdio link could not **route**.
///
/// The stdio face has no event type to dispatch on: every line is a
/// JSON-RPC message. A message the client cannot route is a **server
/// notification** (no `id:` ⇒ no caller is waiting for it) or a reply whose
/// request is already gone. Before this counter existed such a frame was
/// dropped **silently**: the operation side saw nothing at all, while the
/// event-stream face counted the very same kind of loss.
///
/// **Only observation**: the frame is dropped exactly as before and the link
/// stays open. What changes is that the drop is *counted* (per link,
/// queryable through [`McpChannel::unrouted_frames`]) and *logged* (throttled:
/// the first drop plus every [`UNROUTED_FRAME_LOG_EVERY`]-th one).
///
/// The line carries the running count only, **not the frame payload**: the
/// payload may still carry server-side content.
fn note_unrouted_stdio_frame(counter: &AtomicU64, source: &str) {
    let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
    if count == 1 || count % UNROUTED_FRAME_LOG_EVERY == 0 {
        eprintln!("[mcp] {source}: frame is not a routable response (dropped, {count} so far)");
    }
}

/// Reopens channels for one server: every (re)connect gets a fresh link.
#[async_trait]
pub trait McpChannelFactory: Send + Sync {
    /// The bound every response wait on this factory's links runs into.
    ///
    /// `None` = the factory states no bound of its own (a host-injected
    /// transport brings its own). The bridge's own factory states one: the
    /// bound **follows** the bridge's per-call deadline (see
    /// [`response_deadline_for_call`]).
    fn response_deadline(&self) -> Option<Duration> {
        None
    }

    /// Open one fresh channel.
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError>;

    /// Open one fresh channel that reports the frames it reads to `observer`
    /// and asks the server to resume from `last_event_id`.
    ///
    /// The default opens a plain channel: a transport that carries no frames
    /// (stdio / http) has nothing to report and nothing to resume from, so
    /// an owner that does not care about the stream-level fields keeps the
    /// zero-cost path.
    async fn open_with_observer(
        &self,
        observer: Arc<dyn McpFrameObserver>,
        last_event_id: Option<&str>,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        let _ = (observer, last_event_id);
        self.open().await
    }
}

/// A factory that replays one pre-built channel (in-process links, tests).
pub struct StaticChannelFactory {
    channel: Arc<dyn McpChannel>,
}

impl StaticChannelFactory {
    /// A factory always handing out `channel`.
    pub fn new(channel: Arc<dyn McpChannel>) -> Self {
        Self { channel }
    }
}

#[async_trait]
impl McpChannelFactory for StaticChannelFactory {
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(Arc::clone(&self.channel))
    }
}

/// A factory that opens channels according to one [`McpServerSpec`].
pub struct SpecChannelFactory {
    spec: McpServerSpec,
    response_deadline: Duration,
}

impl SpecChannelFactory {
    /// A factory for `spec`, bounding every response wait by
    /// [`RESPONSE_TIMEOUT`].
    pub fn new(spec: McpServerSpec) -> Self {
        Self {
            spec,
            response_deadline: RESPONSE_TIMEOUT,
        }
    }

    /// A factory for `spec`, bounding every response wait by
    /// `response_deadline`.
    ///
    /// The bridge's own wiring uses [`response_deadline_for_call`], so the
    /// transport's bound follows the bridge's per-call deadline instead of
    /// standing on its own.
    pub fn with_response_deadline(spec: McpServerSpec, response_deadline: Duration) -> Self {
        Self {
            spec,
            response_deadline,
        }
    }

    /// The spec this factory opens channels for.
    pub fn spec(&self) -> &McpServerSpec {
        &self.spec
    }

    /// The bound every response wait on this factory's links runs into.
    pub fn response_deadline(&self) -> Duration {
        self.response_deadline
    }
}

#[async_trait]
impl McpChannelFactory for SpecChannelFactory {
    fn response_deadline(&self) -> Option<Duration> {
        Some(self.response_deadline)
    }

    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        open_channel_with_deadline(&self.spec, self.response_deadline).await
    }

    async fn open_with_observer(
        &self,
        observer: Arc<dyn McpFrameObserver>,
        last_event_id: Option<&str>,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        open_channel_with_observer_and_deadline(
            &self.spec,
            self.response_deadline,
            observer,
            last_event_id,
        )
        .await
    }
}

/// Open one channel matching `spec`'s transport kind.
pub async fn open_channel(spec: &McpServerSpec) -> Result<Arc<dyn McpChannel>, McpError> {
    open_channel_with_deadline(spec, RESPONSE_TIMEOUT).await
}

/// Open one channel matching `spec`'s transport kind, bounding every response
/// wait by `response_deadline`.
pub async fn open_channel_with_deadline(
    spec: &McpServerSpec,
    response_deadline: Duration,
) -> Result<Arc<dyn McpChannel>, McpError> {
    open_channel_with_observer_and_deadline(spec, response_deadline, Arc::new(IgnoredFrames), None)
        .await
}

/// Open one channel matching `spec`'s transport kind, reporting every frame
/// the link reads to `observer` and asking the server to resume from
/// `last_event_id`.
pub async fn open_channel_with_observer(
    spec: &McpServerSpec,
    observer: Arc<dyn McpFrameObserver>,
    last_event_id: Option<&str>,
) -> Result<Arc<dyn McpChannel>, McpError> {
    open_channel_with_observer_and_deadline(spec, RESPONSE_TIMEOUT, observer, last_event_id).await
}

/// Open one channel matching `spec`'s transport kind, bounding every response
/// wait by `response_deadline`, reporting every frame the link reads to
/// `observer` and asking the server to resume from `last_event_id`.
///
/// `last_event_id` is the `id:` the server handed out on the previous link:
/// a resumed event stream continues from it. `None` = a fresh stream (the
/// first link of a server has nothing to resume from).
pub async fn open_channel_with_observer_and_deadline(
    spec: &McpServerSpec,
    response_deadline: Duration,
    observer: Arc<dyn McpFrameObserver>,
    last_event_id: Option<&str>,
) -> Result<Arc<dyn McpChannel>, McpError> {
    match spec.transport {
        McpTransportKind::Stdio => {
            let command = spec.command.as_deref().unwrap_or("");
            StdioChannel::open_with_response_deadline(command, &spec.args, response_deadline).await
        }
        McpTransportKind::Http => {
            let url = spec.url.as_deref().unwrap_or("");
            HttpChannel::open_with_response_deadline(url, response_deadline)
        }
        McpTransportKind::Sse => {
            let url = spec.url.as_deref().unwrap_or("");
            SseChannel::open_with_observer(url, response_deadline, observer, last_event_id).await
        }
    }
}

/// Pending request routing shared by the streaming channels: responses are
/// matched back to callers by JSON-RPC id.
#[derive(Default)]
struct PendingRouter {
    pending: Mutex<HashMap<Id, oneshot::Sender<JsonRpcResponse>>>,
}

impl PendingRouter {
    /// Register interest in the response for `id`.
    fn register(self: &Arc<Self>, id: &Id) -> PendingReply {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id.clone(), tx);
        PendingReply {
            receiver: Some(rx),
            router: Arc::downgrade(self),
            id: id.clone(),
        }
    }

    /// Deliver one response to its caller. Returns whether anyone was waiting.
    fn route(&self, response: JsonRpcResponse) -> bool {
        let Some(id) = response.id.clone() else {
            return false;
        };
        let sender = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id);
        match sender {
            Some(sender) => sender.send(response).is_ok(),
            None => false,
        }
    }

    /// Fail every waiter: the link is gone.
    fn fail_all(&self) {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    /// Drop the pending entry for `id`: nobody will ever answer it.
    ///
    /// A waiter that was abandoned — the deadline passed, or the request
    /// never made it out — must not stay in the routing table: the table
    /// would grow with every abandoned request, and a response that arrives
    /// late would be handed to a caller that already gave up.
    fn forget(&self, id: &Id) {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(id);
    }

    /// How many responses are still awaited.
    #[cfg(test)]
    fn pending_len(&self) -> usize {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Wait for the response registered for `id`, bounded by `deadline`.
    ///
    /// The wait ends in exactly one of three ways: the response arrives, the
    /// link dies (the sender is dropped), or the deadline passes. The
    /// deadline is the one that used to be missing — without it a peer that
    /// takes the request and never answers blocks the caller, and with it
    /// the startup wiring, forever.
    ///
    /// A deadline that passes **drops the pending entry**: the caller gives
    /// up, so the routing table must not keep a waiter nobody will ever
    /// satisfy. The [`PendingReply`] guard does that on its own — dropping
    /// the reply removes the entry — and the explicit [`PendingRouter::forget`]
    /// below is the belt-and-braces second removal.
    async fn wait_for(
        &self,
        id: &Id,
        waiter: PendingReply,
        deadline: Duration,
        source: &str,
    ) -> Result<JsonRpcResponse, McpError> {
        match tokio::time::timeout(deadline, waiter.receive()).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(McpError::Transport(format!(
                "{source} closed before the response arrived"
            ))),
            Err(_) => {
                self.forget(id);
                Err(response_timeout(source, deadline))
            }
        }
    }
}

pub(super) fn transport_error(context: &str, error: impl std::fmt::Display) -> McpError {
    McpError::Transport(format!("{context}: {error}"))
}

// A cancelled request cannot leave a routing entry until an external reply.
struct PendingReply {
    receiver: Option<oneshot::Receiver<JsonRpcResponse>>,
    router: std::sync::Weak<PendingRouter>,
    id: Id,
}
impl PendingReply {
    async fn receive(mut self) -> Result<JsonRpcResponse, oneshot::error::RecvError> {
        self.receiver.take().expect("receiver used once").await
    }
}
impl Drop for PendingReply {
    fn drop(&mut self) {
        if let Some(router) = self.router.upgrade() {
            router
                .pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&self.id);
        }
    }
}

/// A local child process speaking newline-delimited JSON-RPC on stdio.
pub struct StdioChannel {
    stdin: tokio::sync::Mutex<ChildStdin>,
    child: Mutex<Option<Child>>,
    process_owner: Mutex<Option<super::process_owner::ProcessOwner>>,
    reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
    router: Arc<PendingRouter>,
    broken: Arc<AtomicBool>,
    malformed: Arc<AtomicU64>,
    unrouted: Arc<AtomicU64>,
    response_deadline: Duration,
}

impl StdioChannel {
    /// Spawn `command` with `args` and wrap its standard streams.
    pub async fn open(command: &str, args: &[String]) -> Result<Arc<dyn McpChannel>, McpError> {
        Self::open_with_response_deadline(command, args, RESPONSE_TIMEOUT).await
    }

    /// Spawn `command` with `args`, bounding every response wait on this
    /// link by `response_deadline`.
    ///
    /// The deadline is the transport's own bound on "how long the child may
    /// take to answer one request". The production wiring uses
    /// [`RESPONSE_TIMEOUT`]; an explicit deadline exists so the bound can be
    /// exercised without waiting for it.
    pub async fn open_with_response_deadline(
        command: &str,
        args: &[String],
        response_deadline: Duration,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        let mut process = Command::new(command);
        super::process_owner::ProcessOwner::configure(&mut process);
        let mut child = process
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| transport_error(&format!("spawning {command}"), e))?;
        let process_owner = super::process_owner::ProcessOwner::attach(&child)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport(format!("{command}: child has no stdin")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport(format!("{command}: child has no stdout")))?;

        let router = Arc::new(PendingRouter::default());
        let broken = Arc::new(AtomicBool::new(false));
        let malformed = Arc::new(AtomicU64::new(0));
        let unrouted = Arc::new(AtomicU64::new(0));
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader_malformed = Arc::clone(&malformed);
        let reader_unrouted = Arc::clone(&unrouted);
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        // 坏行照样丢, 但**记一笔**（可观测, 不改行为）。
                        match serde_json::from_str::<JsonRpcResponse>(&line) {
                            Ok(response) => {
                                // 服务器主动通知（**无 `id`**）能解析成功, 但没有
                                // 调用者在等它 ⇒ 路由不到 ⇒ **记一笔**（与 SSE 面
                                // 同口径: 路由不到的帧不许静默消失）。
                                if !reader_router.route(response) {
                                    note_unrouted_stdio_frame(&reader_unrouted, "stdio channel");
                                }
                            }
                            Err(error) => {
                                note_malformed_frame(&reader_malformed, "stdio channel", &error);
                            }
                        }
                    }
                    _ => break,
                }
            }
            reader_broken.store(true, Ordering::SeqCst);
            reader_router.fail_all();
        });

        Ok(Arc::new(Self {
            stdin: tokio::sync::Mutex::new(stdin),
            child: Mutex::new(Some(child)),
            process_owner: Mutex::new(Some(process_owner)),
            reader: Mutex::new(Some(reader)),
            router,
            broken,
            malformed,
            unrouted,
            response_deadline,
        }))
    }

    async fn write_line(&self, line: &str) -> Result<(), McpError> {
        if self.broken.load(Ordering::SeqCst) {
            return Err(McpError::Transport("stdio channel is closed".to_string()));
        }
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| transport_error("writing to child stdin", e))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| transport_error("writing to child stdin", e))?;
        stdin
            .flush()
            .await
            .map_err(|e| transport_error("flushing child stdin", e))
    }
}

#[async_trait]
impl McpChannel for StdioChannel {
    fn revoke(&self) {
        self.broken.store(true, Ordering::SeqCst);
        self.router.fail_all();
        if let Some(reader) = self
            .reader
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            reader.abort();
        }
        if let Some(owner) = self
            .process_owner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let _ = owner.terminate();
        }
    }

    async fn close(&self) -> Result<(), McpError> {
        self.broken.store(true, Ordering::SeqCst);
        self.router.fail_all();
        let reader = self.reader.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(reader) = reader {
            reader.abort();
            let _ = reader.await;
        }
        let owner = self
            .process_owner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(owner) = owner {
            owner.terminate()?;
        }
        let child = self.child.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut child) = child {
            if child
                .try_wait()
                .map_err(|e| transport_error("checking owned server", e))?
                .is_none()
            {
                let _ = child.start_kill();
            }
            child
                .wait()
                .await
                .map_err(|e| transport_error("reaping owned server", e))?;
        }
        Ok(())
    }

    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let Some(id) = request.id.clone() else {
            return Err(McpError::Transport(
                "requests must carry a JSON-RPC id".to_string(),
            ));
        };
        let line = serde_json::to_string(&request)
            .map_err(|e| transport_error("serializing request", e))?;
        let waiter = self.router.register(&id);
        if let Err(error) = self.write_line(&line).await {
            // The request never made it out: nobody will ever answer it.
            self.router.forget(&id);
            return Err(error);
        }
        self.router
            .wait_for(&id, waiter, self.response_deadline, "stdio channel")
            .await
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        let line = serde_json::to_string(&notification)
            .map_err(|e| transport_error("serializing notification", e))?;
        self.write_line(&line).await
    }

    fn malformed_frames(&self) -> u64 {
        self.malformed.load(Ordering::SeqCst)
    }

    fn unrouted_frames(&self) -> u64 {
        self.unrouted.load(Ordering::SeqCst)
    }
}

impl Drop for StdioChannel {
    fn drop(&mut self) {
        if let Some(reader) = self.reader.lock().unwrap_or_else(|e| e.into_inner()).take() {
            reader.abort();
        }
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.start_kill();
            }
        }
    }
}

/// An HTTP endpoint answering JSON-RPC requests directly.
pub struct HttpChannel {
    client: reqwest::Client,
    url: String,
    response_deadline: Duration,
}

impl HttpChannel {
    /// A channel posting every message to `url`.
    pub fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
        Self::open_with_response_deadline(url, RESPONSE_TIMEOUT)
    }

    /// A channel posting every message to `url`, bounding every response
    /// wait on this link by `response_deadline`.
    ///
    /// The deadline is the transport's own bound on "how long the endpoint
    /// may take to answer one request". The production wiring uses
    /// [`RESPONSE_TIMEOUT`]; an explicit deadline exists so the bound can be
    /// exercised without waiting for it.
    pub fn open_with_response_deadline(
        url: &str,
        response_deadline: Duration,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(Arc::new(Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
            response_deadline,
        }))
    }

    async fn post(&self, message: &JsonRpcRequest) -> Result<reqwest::Response, McpError> {
        let response = self
            .client
            .post(&self.url)
            .json(message)
            .send()
            .await
            .map_err(|e| transport_error("posting JSON-RPC message", e))?;
        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "endpoint answered HTTP {}",
                response.status()
            )));
        }
        Ok(response)
    }
}

#[async_trait]
impl McpChannel for HttpChannel {
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let response = self.post(&request).await?;
        // The body carries the answer: a server that sends the head and then
        // goes quiet used to block this read forever.
        match tokio::time::timeout(self.response_deadline, response.json::<JsonRpcResponse>()).await
        {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(error)) => Err(transport_error("decoding JSON-RPC response", error)),
            Err(_) => Err(response_timeout("http endpoint", self.response_deadline)),
        }
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        self.post(&notification).await.map(|_| ())
    }
}

/// An HTTP endpoint carrying server-sent events for responses, with requests
/// posted to the endpoint the stream announces.
pub struct SseChannel {
    client: reqwest::Client,
    base_url: String,
    endpoint: watch::Receiver<Option<String>>,
    router: Arc<PendingRouter>,
    broken: Arc<AtomicBool>,
    malformed: Arc<AtomicU64>,
    unrouted: Arc<AtomicU64>,
    response_deadline: Duration,
    reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SseChannel {
    /// Open the event stream at `url` and wait for its message endpoint.
    pub async fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
        Self::open_with_response_deadline(url, RESPONSE_TIMEOUT).await
    }

    /// Open the event stream at `url`, bounding every response wait on this
    /// link by `response_deadline`.
    ///
    /// The deadline is the transport's own bound on "how long the server may
    /// take to answer one request". The production wiring uses
    /// [`RESPONSE_TIMEOUT`]; an explicit deadline exists so the bound can be
    /// exercised without waiting for it.
    pub async fn open_with_response_deadline(
        url: &str,
        response_deadline: Duration,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        Self::open_with_observer(url, response_deadline, Arc::new(IgnoredFrames), None).await
    }

    /// Open the event stream at `url`, bounding every response wait on this
    /// link by `response_deadline`, reporting every frame the link reads to
    /// `observer`, and asking the server to resume from `last_event_id`.
    ///
    /// `last_event_id` is the `id:` the server handed out on the previous
    /// link: it travels as the `Last-Event-ID` request header, so a resumed
    /// stream continues where the dead one stopped. `None` = a fresh stream.
    ///
    /// The observer is **observation only**: it never changes what the link
    /// does with a frame.
    pub async fn open_with_observer(
        url: &str,
        response_deadline: Duration,
        observer: Arc<dyn McpFrameObserver>,
        last_event_id: Option<&str>,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        let client = reqwest::Client::new();
        let mut request = client.get(url).header("Accept", "text/event-stream");
        if let Some(id) = last_event_id {
            request = request.header("Last-Event-ID", id);
        }
        let response = request
            .send()
            .await
            .map_err(|e| transport_error("opening event stream", e))?;
        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "event stream answered HTTP {}",
                response.status()
            )));
        }

        let (endpoint_tx, endpoint_rx) = watch::channel::<Option<String>>(None);
        let router = Arc::new(PendingRouter::default());
        let broken = Arc::new(AtomicBool::new(false));
        let malformed = Arc::new(AtomicU64::new(0));
        let unrouted = Arc::new(AtomicU64::new(0));
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader_malformed = Arc::clone(&malformed);
        let reader_unrouted = Arc::clone(&unrouted);
        let base_url = url.to_string();
        let reader = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = SseBuffer::new();
            while let Some(chunk) = stream.next().await {
                let Ok(bytes) = chunk else {
                    break;
                };
                buffer.push_bytes(&bytes);
                for frame in buffer.drain_frames() {
                    // 流级提示（`id:` / `retry:`）不经过响应路径 ⇒ 交给观察者,
                    // 由链路的拥有者决定这些字段意味着什么。
                    observer.observe_frame(&frame);
                    if frame.is_endpoint() {
                        let endpoint = absolutize_endpoint(&base_url, &frame.data());
                        let _ = endpoint_tx.send(Some(endpoint));
                    } else if frame.is_message() {
                        // 坏帧照样丢, 但**记一笔**（可观测, 不改行为: 链路不断,
                        // 该次请求照旧等不到响应 —— 那是「响应等待无截止」那条
                        // 遗留的地盘, 本处不碰）。
                        match serde_json::from_str::<JsonRpcResponse>(&frame.data()) {
                            Ok(response) => {
                                reader_router.route(response);
                            }
                            Err(error) => {
                                note_malformed_frame(&reader_malformed, "event stream", &error);
                            }
                        }
                    } else if frame.event.is_none() && !frame.data_lines.is_empty() {
                        // 裸 `data:` 帧（**无** `event:`）: 协议库不把它当 message
                        // ⇒ 两个分支都不进 ⇒ 改前**连计数都不进**（完全无声）。
                        // 现在**记一笔**（可观测, 不改行为: 帧仍被丢弃, 链路不断,
                        // 该次请求照旧等不到响应 —— 行为变更另批, 本处不碰）。
                        note_unrouted_frame(&reader_unrouted, "event stream");
                    }
                }
            }
            reader_broken.store(true, Ordering::SeqCst);
            reader_router.fail_all();
        });

        let channel = Self {
            client,
            base_url: url.to_string(),
            endpoint: endpoint_rx,
            router,
            broken,
            malformed,
            unrouted,
            response_deadline,
            reader: Mutex::new(Some(reader)),
        };
        channel.wait_for_endpoint().await?;
        Ok(Arc::new(channel))
    }

    async fn wait_for_endpoint(&self) -> Result<String, McpError> {
        let mut endpoint = self.endpoint.clone();
        if let Some(value) = endpoint.borrow().as_ref() {
            return Ok(value.clone());
        }
        let deadline = std::time::Duration::from_secs(10);
        let wait = async {
            loop {
                if endpoint.changed().await.is_err() {
                    return None;
                }
                if let Some(value) = endpoint.borrow().as_ref() {
                    return Some(value.clone());
                }
            }
        };
        tokio::time::timeout(deadline, wait)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| {
                McpError::Transport("event stream did not announce a message endpoint".to_string())
            })
    }

    async fn post_to_endpoint(
        &self,
        message: &JsonRpcRequest,
    ) -> Result<reqwest::Response, McpError> {
        if self.broken.load(Ordering::SeqCst) {
            return Err(McpError::Transport("event stream is closed".to_string()));
        }
        let endpoint = self.wait_for_endpoint().await?;
        let response = self
            .client
            .post(&endpoint)
            .json(message)
            .send()
            .await
            .map_err(|e| transport_error("posting JSON-RPC message", e))?;
        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "message endpoint answered HTTP {}",
                response.status()
            )));
        }
        Ok(response)
    }
}

#[async_trait]
impl McpChannel for SseChannel {
    fn revoke(&self) {
        self.broken.store(true, Ordering::SeqCst);
        self.router.fail_all();
        if let Some(reader) = self
            .reader
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            reader.abort();
        }
    }

    async fn close(&self) -> Result<(), McpError> {
        self.broken.store(true, Ordering::SeqCst);
        self.router.fail_all();
        let reader = self.reader.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(reader) = reader {
            reader.abort();
            let _ = reader.await;
        }
        Ok(())
    }

    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let Some(id) = request.id.clone() else {
            return Err(McpError::Transport(
                "requests must carry a JSON-RPC id".to_string(),
            ));
        };
        let waiter = self.router.register(&id);
        if let Err(error) = self.post_to_endpoint(&request).await {
            // The request never made it out: nobody will ever answer it.
            self.router.forget(&id);
            return Err(error);
        }
        self.router
            .wait_for(&id, waiter, self.response_deadline, "event stream")
            .await
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        self.post_to_endpoint(&notification).await.map(|_| ())
    }

    fn malformed_frames(&self) -> u64 {
        self.malformed.load(Ordering::SeqCst)
    }

    fn unrouted_frames(&self) -> u64 {
        self.unrouted.load(Ordering::SeqCst)
    }
}

// Keep the base URL reachable for diagnostics without exposing it elsewhere.
impl SseChannel {
    /// The stream URL this channel was opened against.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for SseChannel {
    fn drop(&mut self) {
        if let Some(reader) = self.reader.lock().unwrap_or_else(|e| e.into_inner()).take() {
            reader.abort();
        }
        self.router.fail_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_id(n: i64) -> Id {
        Id::Num(n)
    }

    fn answer(id: &Id) -> JsonRpcResponse {
        JsonRpcResponse::ok(Some(id.clone()), serde_json::json!({ "ok": true }))
    }

    /// A response wait that runs out of time ends in an error and leaves **no**
    /// waiter behind: the caller gave up, so the routing table must not keep
    /// an entry nobody will ever satisfy.
    #[tokio::test]
    async fn a_response_wait_that_runs_out_of_time_leaves_no_pending_waiter() {
        let router = Arc::new(PendingRouter::default());
        let id = request_id(7);
        let waiter = router.register(&id);
        assert_eq!(router.pending_len(), 1);

        let error = router
            .wait_for(&id, waiter, Duration::ZERO, "event stream")
            .await
            .expect_err("a zero deadline must not be met by a peer that never answers");

        assert_eq!(router.pending_len(), 0, "the abandoned waiter must be gone");
        match error {
            McpError::Transport(reason) => {
                assert!(
                    reason.contains("did not deliver the response"),
                    "the error must name the wait: {reason}"
                );
            }
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    /// The timeout error is tellable apart from the two neighbouring
    /// failures: a stream that never announced its endpoint, and a link that
    /// died before the answer came.
    #[tokio::test]
    async fn the_timeout_error_is_tellable_apart_from_the_neighbouring_failures() {
        let router = Arc::new(PendingRouter::default());
        let id = request_id(8);
        let waiter = router.register(&id);
        let timed_out = router
            .wait_for(&id, waiter, Duration::ZERO, "event stream")
            .await
            .expect_err("the deadline must not be met");

        let McpError::Transport(reason) = timed_out else {
            panic!("expected a transport error");
        };
        assert!(reason.contains("did not deliver the response"));
        assert!(!reason.contains("did not announce a message endpoint"));
        assert!(!reason.contains("closed before the response arrived"));

        // The link died: the sender is dropped, the waiter is woken.
        let router = Arc::new(PendingRouter::default());
        let id = request_id(9);
        let waiter = router.register(&id);
        router.fail_all();
        let closed = router
            .wait_for(&id, waiter, Duration::from_secs(30), "event stream")
            .await
            .expect_err("a dead link must not answer");
        let McpError::Transport(reason) = closed else {
            panic!("expected a transport error");
        };
        assert!(reason.contains("closed before the response arrived"));
        assert!(!reason.contains("did not deliver the response"));
    }

    /// The normal path is untouched: an answer that arrives before the
    /// deadline is handed to the caller, and the routing table is empty
    /// afterwards.
    #[tokio::test]
    async fn a_response_that_arrives_before_the_deadline_is_returned() {
        let router = Arc::new(PendingRouter::default());
        let id = request_id(10);
        let waiter = router.register(&id);
        let answer_router = Arc::clone(&router);
        let answer_id = id.clone();
        tokio::spawn(async move {
            answer_router.route(answer(&answer_id));
        });

        let response = router
            .wait_for(&id, waiter, Duration::from_secs(30), "event stream")
            .await
            .expect("the answer must reach the caller");
        assert_eq!(response.id, Some(id));
        assert_eq!(router.pending_len(), 0);
    }

    /// A request that never made it out leaves no waiter behind either.
    #[tokio::test]
    async fn an_abandoned_request_leaves_no_pending_waiter() {
        let router = Arc::new(PendingRouter::default());
        let id = request_id(11);
        // The reply handle is **held**: `PendingReply` drops the routing entry
        // when it dies, so a discarded handle would remove the entry right
        // away and the assertion below would no longer test the abandon path.
        let _waiter = router.register(&id);
        assert_eq!(router.pending_len(), 1);
        router.forget(&id);
        assert_eq!(router.pending_len(), 0);
    }
}
