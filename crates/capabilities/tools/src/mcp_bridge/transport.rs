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

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, watch};

use apeireth_plugin::mcp::sse::{absolutize_endpoint, SseBuffer};
use apeireth_plugin::mcp::{Id, JsonRpcRequest, JsonRpcResponse};

use crate::mcp::McpError;

use super::config::{McpServerSpec, McpTransportKind};

/// The deadline one response wait may take before the transport gives up.
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

/// One JSON-RPC link to one external server.
#[async_trait]
pub trait McpChannel: Send + Sync {
    /// Send one request and await its response.
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

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
}

/// How many dropped malformed frames pass between two stderr lines.
///
/// A server that speaks a broken frame can flood the stream ⇒ one line per
/// frame would bury the rest of the run. **Every drop is still counted**; only
/// the log is throttled (the first drop, then every [`MALFORMED_FRAME_LOG_EVERY`]-th).
const MALFORMED_FRAME_LOG_EVERY: u64 = 100;

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

/// Reopens channels for one server: every (re)connect gets a fresh link.
#[async_trait]
pub trait McpChannelFactory: Send + Sync {
    /// Open one fresh channel.
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError>;
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
}

impl SpecChannelFactory {
    /// A factory for `spec`.
    pub fn new(spec: McpServerSpec) -> Self {
        Self { spec }
    }

    /// The spec this factory opens channels for.
    pub fn spec(&self) -> &McpServerSpec {
        &self.spec
    }
}

#[async_trait]
impl McpChannelFactory for SpecChannelFactory {
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        open_channel(&self.spec).await
    }
}

/// Open one channel matching `spec`'s transport kind.
pub async fn open_channel(spec: &McpServerSpec) -> Result<Arc<dyn McpChannel>, McpError> {
    match spec.transport {
        McpTransportKind::Stdio => {
            let command = spec.command.as_deref().unwrap_or("");
            StdioChannel::open(command, &spec.args).await
        }
        McpTransportKind::Http => {
            let url = spec.url.as_deref().unwrap_or("");
            HttpChannel::open(url)
        }
        McpTransportKind::Sse => {
            let url = spec.url.as_deref().unwrap_or("");
            SseChannel::open(url).await
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
    fn register(&self, id: &Id) -> oneshot::Receiver<JsonRpcResponse> {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id.clone(), tx);
        rx
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
    /// satisfy.
    async fn wait_for(
        &self,
        id: &Id,
        waiter: oneshot::Receiver<JsonRpcResponse>,
        deadline: Duration,
        source: &str,
    ) -> Result<JsonRpcResponse, McpError> {
        match tokio::time::timeout(deadline, waiter).await {
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

fn transport_error(context: &str, error: impl std::fmt::Display) -> McpError {
    McpError::Transport(format!("{context}: {error}"))
}

/// A local child process speaking newline-delimited JSON-RPC on stdio.
pub struct StdioChannel {
    stdin: tokio::sync::Mutex<ChildStdin>,
    child: Mutex<Option<Child>>,
    router: Arc<PendingRouter>,
    broken: Arc<AtomicBool>,
    malformed: Arc<AtomicU64>,
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
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| transport_error(&format!("spawning {command}"), e))?;
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
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader_malformed = Arc::clone(&malformed);
        tokio::spawn(async move {
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
                                reader_router.route(response);
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
            router,
            broken,
            malformed,
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
}

impl Drop for StdioChannel {
    fn drop(&mut self) {
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
    response_deadline: Duration,
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
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Accept", "text/event-stream")
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
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader_malformed = Arc::clone(&malformed);
        let base_url = url.to_string();
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = SseBuffer::new();
            while let Some(chunk) = stream.next().await {
                let Ok(bytes) = chunk else {
                    break;
                };
                buffer.push_bytes(&bytes);
                for frame in buffer.drain_frames() {
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
            response_deadline,
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
}

// Keep the base URL reachable for diagnostics without exposing it elsewhere.
impl SseChannel {
    /// The stream URL this channel was opened against.
    pub fn base_url(&self) -> &str {
        &self.base_url
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
        let router = PendingRouter::default();
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
        let router = PendingRouter::default();
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
        let router = PendingRouter::default();
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
        let router = PendingRouter::default();
        let id = request_id(11);
        router.register(&id);
        assert_eq!(router.pending_len(), 1);
        router.forget(&id);
        assert_eq!(router.pending_len(), 0);
    }
}
