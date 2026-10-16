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

use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, watch};

use apeireth_plugin::mcp::sse::{absolutize_endpoint, SseBuffer};
use apeireth_plugin::mcp::{Id, JsonRpcRequest, JsonRpcResponse};

use crate::mcp::McpError;

use super::config::{McpServerSpec, McpTransportKind};

/// One JSON-RPC link to one external server.
#[async_trait]
pub trait McpChannel: Send + Sync {
    /// Send one request and await its response.
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

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
}

impl StdioChannel {
    /// Spawn `command` with `args` and wrap its standard streams.
    pub async fn open(command: &str, args: &[String]) -> Result<Arc<dyn McpChannel>, McpError> {
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
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                            reader_router.route(response);
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
        let waiter = self.router.register(&id);
        let line = serde_json::to_string(&request)
            .map_err(|e| transport_error("serializing request", e))?;
        self.write_line(&line).await?;
        waiter.await.map_err(|_| {
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
}

impl HttpChannel {
    /// A channel posting every message to `url`.
    pub fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(Arc::new(Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
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
        response
            .json::<JsonRpcResponse>()
            .await
            .map_err(|e| transport_error("decoding JSON-RPC response", e))
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
}

impl SseChannel {
    /// Open the event stream at `url` and wait for its message endpoint.
    pub async fn open(url: &str) -> Result<Arc<dyn McpChannel>, McpError> {
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
        let reader_router = Arc::clone(&router);
        let reader_broken = Arc::clone(&broken);
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
        self.post_to_endpoint(&request).await?;
        waiter.await.map_err(|_| {
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
