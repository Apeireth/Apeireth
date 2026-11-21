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

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, watch};

use apeireth_plugin::mcp::sse::SseBuffer;
use apeireth_plugin::mcp::{Id, JsonRpcRequest, JsonRpcResponse};

use crate::egress::{EgressDestination, EgressPolicy};
use crate::mcp::McpError;

use super::config::{remote_target, McpServerSpec, McpTransportKind};

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
}

pub(super) fn transport_error(context: &str, error: impl std::fmt::Display) -> McpError {
    McpError::Transport(format!("{context}: {error}"))
}

fn stdio_command(command: &str) -> Command {
    let mut process = Command::new(command);
    process.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "TEMP",
        "TMP",
        "TMPDIR",
        "LANG",
    ] {
        if let Some(value) = std::env::var_os(key) {
            process.env(key, value);
        }
    }
    process
}

const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

async fn read_frame(
    reader: &mut (impl AsyncBufRead + Unpin),
    frame: &mut Vec<u8>,
    limit: usize,
) -> std::io::Result<usize> {
    frame.clear();
    let size = reader
        .take(limit as u64 + 1)
        .read_until(b'\n', frame)
        .await?;
    if size > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "MCP frame exceeds its size limit",
        ));
    }
    Ok(size)
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
}

impl StdioChannel {
    /// Spawn `command` with `args` and wrap its standard streams.
    pub async fn open(command: &str, args: &[String]) -> Result<Arc<dyn McpChannel>, McpError> {
        let mut process = stdio_command(command);
        // Unix：子进程归独立进程组 ⇒ 终止时能整组 kill（非 Unix 侧由 Job Object 收敛）。
        #[cfg(unix)]
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
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut frame = Vec::new();
            while let Ok(1..) = read_frame(&mut reader, &mut frame, MAX_MESSAGE_BYTES).await {
                if let Ok(response) = serde_json::from_slice::<JsonRpcResponse>(&frame) {
                    reader_router.route(response);
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
        let waiter = self.router.register(&id);
        let line = serde_json::to_string(&request)
            .map_err(|e| transport_error("serializing request", e))?;
        self.write_line(&line).await?;
        waiter.receive().await.map_err(|_| {
            McpError::Transport("stdio channel closed before the response arrived".to_string())
        })
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        let line = serde_json::to_string(&notification)
            .map_err(|e| transport_error("serializing notification", e))?;
        self.write_line(&line).await
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
    target: url::Url,
    destination: EgressDestination,
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

fn parse_remote_target(raw: &str) -> Result<(url::Url, EgressDestination), McpError> {
    remote_target(raw).ok_or_else(|| McpError::Transport("invalid remote endpoint".to_string()))
}

async fn pinned_http_client(destination: &EgressDestination) -> Result<reqwest::Client, McpError> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT);
    if destination.address.is_none() {
        let addrs = tokio::time::timeout(
            CONNECT_TIMEOUT,
            tokio::net::lookup_host((destination.host.as_str(), destination.port)),
        )
        .await
        .map_err(|e| transport_error("resolving remote endpoint", e))?
        .map_err(|e| transport_error("resolving remote endpoint", e))?
        .collect::<Vec<_>>();
        EgressPolicy::PublicInternetOnly
            .validate_resolved(destination, &addrs)
            .map_err(|e| transport_error("validating remote endpoint", e))?;
        builder = builder.resolve_to_addrs(&destination.host, &addrs);
    }
    builder
        .build()
        .map_err(|e| transport_error("opening remote transport", e.without_url()))
}

fn same_origin_endpoint(base: &url::Url, endpoint: &str) -> Option<url::Url> {
    let target = base.join(endpoint).ok()?;
    if target.origin() != base.origin()
        || !target.username().is_empty()
        || target.password().is_some()
        || target.fragment().is_some()
    {
        return None;
    }
    Some(target)
}

impl HttpChannel {
    /// A channel posting every message to `url`.
    pub fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
        let (target, destination) = parse_remote_target(url)?;
        Ok(Arc::new(Self {
            target,
            destination,
        }))
    }

    async fn post(&self, message: &JsonRpcRequest) -> Result<reqwest::Response, McpError> {
        let response = pinned_http_client(&self.destination)
            .await?
            .post(self.target.clone())
            .timeout(HTTP_RESPONSE_TIMEOUT)
            .json(message)
            .send()
            .await
            .map_err(|e| transport_error("posting JSON-RPC message", e.without_url()))?;
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
        let mut response = self.post(&request).await?;
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| transport_error("reading JSON-RPC response", e.without_url()))?
        {
            if chunk.len() > MAX_MESSAGE_BYTES.saturating_sub(body.len()) {
                return Err(McpError::Transport(
                    "MCP response exceeds its size limit".into(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body)
            .map_err(|_| McpError::Transport("invalid JSON-RPC response".into()))
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
    endpoint: watch::Receiver<Option<url::Url>>,
    router: Arc<PendingRouter>,
    broken: Arc<AtomicBool>,
    reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SseChannel {
    /// Open the event stream at `url` and wait for its message endpoint.
    pub async fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
        let (base, destination) = parse_remote_target(url)?;
        let client = pinned_http_client(&destination).await?;
        let response = tokio::time::timeout(
            HTTP_RESPONSE_TIMEOUT,
            client
                .get(base.clone())
                .header("Accept", "text/event-stream")
                .send(),
        )
        .await
        .map_err(|e| transport_error("opening event stream", e))?
        .map_err(|e| transport_error("opening event stream", e.without_url()))?;
        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "event stream answered HTTP {}",
                response.status()
            )));
        }

        let (endpoint_tx, endpoint_rx) = watch::channel::<Option<url::Url>>(None);
        let router = Arc::new(PendingRouter::default());
        let broken = Arc::new(AtomicBool::new(false));
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        let reader = tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = SseBuffer::new();
            while let Some(chunk) = stream.next().await {
                let Ok(bytes) = chunk else {
                    break;
                };
                if bytes.len() > MAX_MESSAGE_BYTES.saturating_sub(buffer.pending_len()) {
                    break;
                }
                let text = String::from_utf8_lossy(&bytes);
                if text.len() > MAX_MESSAGE_BYTES.saturating_sub(buffer.pending_len()) {
                    break;
                }
                buffer.push_str(&text);
                for frame in buffer.drain_frames() {
                    if frame.is_endpoint() {
                        let Some(endpoint) = same_origin_endpoint(&base, &frame.data()) else {
                            reader_broken.store(true, Ordering::SeqCst);
                            reader_router.fail_all();
                            return;
                        };
                        let _ = endpoint_tx.send(Some(endpoint));
                    } else if frame.is_message() {
                        if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&frame.data())
                        {
                            reader_router.route(response);
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
            reader: Mutex::new(Some(reader)),
        };
        channel.wait_for_endpoint().await?;
        Ok(Arc::new(channel))
    }

    async fn wait_for_endpoint(&self) -> Result<url::Url, McpError> {
        let mut endpoint = self.endpoint.clone();
        if let Some(value) = endpoint.borrow().as_ref() {
            return Ok(value.clone());
        }
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
        tokio::time::timeout(CONNECT_TIMEOUT, wait)
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
            .post(endpoint)
            .timeout(HTTP_RESPONSE_TIMEOUT)
            .json(message)
            .send()
            .await
            .map_err(|e| transport_error("posting JSON-RPC message", e.without_url()))?;
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
        self.post_to_endpoint(&request).await?;
        waiter.receive().await.map_err(|_| {
            McpError::Transport("event stream closed before the response arrived".to_string())
        })
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        self.post_to_endpoint(&notification).await.map(|_| ())
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

    #[tokio::test]
    async fn stdio_reader_stops_at_the_size_bound() {
        let mut reader = BufReader::new(&b"{}\n123456\n"[..]);
        let mut frame = Vec::new();
        assert_eq!(read_frame(&mut reader, &mut frame, 3).await.unwrap(), 3);
        assert_eq!(frame, b"{}\n");
        let error = read_frame(&mut reader, &mut frame, 4).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(frame.len(), 5);
    }

    #[test]
    fn configured_targets_require_public_https_or_literal_loopback() {
        for url in [
            "https://tools.example.test/mcp",
            "http://127.0.0.1:8080/mcp",
            "http://[::1]:8080/mcp",
        ] {
            assert!(HttpChannel::open(url).is_ok(), "{url}");
        }
        for url in [
            "http://tools.example.test/mcp",
            "https://192.0.2.1/mcp",
            "https://[fc00::1]/mcp",
            "https://user@tools.example.test/mcp",
            "https://tools.example.test/mcp#fragment",
        ] {
            assert!(HttpChannel::open(url).is_err(), "{url}");
        }
    }

    #[test]
    fn stream_message_endpoint_stays_on_configured_origin() {
        let base = url::Url::parse("https://tools.example.test/events").unwrap();
        assert_eq!(
            same_origin_endpoint(&base, "/messages?session=demo")
                .unwrap()
                .as_str(),
            "https://tools.example.test/messages?session=demo"
        );
        for endpoint in [
            "http://tools.example.test/messages",
            "https://tools.example.test:8443/messages",
            "https://other.example.test/messages",
            "https://user@tools.example.test/messages",
        ] {
            assert!(
                same_origin_endpoint(&base, endpoint).is_none(),
                "{endpoint}"
            );
        }
    }
}
