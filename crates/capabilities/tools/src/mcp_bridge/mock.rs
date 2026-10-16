//! In-process JSON-RPC server test double (no sockets, no subprocesses).
//!
//! The end-to-end tests must never touch a real network, so the server side
//! of every test lives here: a plain in-memory object answering the same
//! JSON-RPC envelopes the real channels carry, wired to the client through
//! [`InProcessChannel`]. It understands `initialize` (answered through the
//! protocol library's own handshake handler), `tools/list`, and `tools/call`,
//! and it can simulate a link drop (`drop_next_requests`) so reconnect and
//! re-discovery paths are exercised without any I/O.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use apeireth_plugin::mcp::{
    handle_initialize, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerInfo, TOOL_NOT_FOUND,
};

use crate::mcp::McpError;

use super::transport::{McpChannel, McpChannelFactory};

/// One tool the in-process server exposes.
#[derive(Debug, Clone)]
pub struct MockToolSpec {
    /// Server-side tool name (kebab-case).
    pub name: String,
    /// Description offered in `tools/list`.
    pub description: String,
    /// The server's read-only permission declaration.
    pub read_only: bool,
    /// Text block returned on success.
    pub reply_text: String,
    /// Whether `tools/call` answers with `isError`.
    pub reply_is_error: bool,
    /// Artificial reply delay (deadline tests).
    pub reply_delay: Duration,
    /// Raw `tools/call` result override (normalization tests).
    pub reply_raw: Option<serde_json::Value>,
}

impl MockToolSpec {
    /// A read-write tool answering with `reply_text`.
    pub fn new(name: impl Into<String>, reply_text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: "in-process test tool".to_string(),
            read_only: false,
            reply_text: reply_text.into(),
            reply_is_error: false,
            reply_delay: Duration::ZERO,
            reply_raw: None,
        }
    }

    /// Declare the tool read-only (server permission declaration).
    #[must_use]
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Set the offered description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Answer with `isError` and the given text.
    #[must_use]
    pub fn error_reply(mut self, text: impl Into<String>) -> Self {
        self.reply_is_error = true;
        self.reply_text = text.into();
        self
    }

    /// Delay every reply by `delay` (deadline tests).
    #[must_use]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.reply_delay = delay;
        self
    }

    /// Answer with a raw `tools/call` result instead of the text block.
    #[must_use]
    pub fn with_raw_result(mut self, result: serde_json::Value) -> Self {
        self.reply_raw = Some(result);
        self
    }
}

/// The in-process JSON-RPC server.
pub struct InProcessMcpServer {
    name: String,
    tools: Mutex<Vec<MockToolSpec>>,
    calls: Mutex<Vec<(String, serde_json::Value)>>,
    methods: Mutex<HashMap<String, usize>>,
    drop_requests: Mutex<u32>,
    opened_channels: AtomicUsize,
}

impl InProcessMcpServer {
    /// A server with the given identity and no tools yet.
    pub fn new(name: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            tools: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
            methods: Mutex::new(HashMap::new()),
            drop_requests: Mutex::new(0),
            opened_channels: AtomicUsize::new(0),
        })
    }

    /// Replace the served tool list (re-discovery tests swap it between
    /// reconnects).
    pub fn set_tools(&self, tools: Vec<MockToolSpec>) {
        *self
            .tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = tools;
    }

    /// Fail the next `count` requests with a transport-level link drop.
    pub fn drop_next_requests(&self, count: u32) {
        *self
            .drop_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = count;
    }

    /// Every `tools/call` received, in order: (tool name, arguments).
    pub fn call_log(&self) -> Vec<(String, serde_json::Value)> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// How often one JSON-RPC method was received.
    pub fn method_count(&self, method: &str) -> usize {
        self.methods
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(method)
            .copied()
            .unwrap_or(0)
    }

    /// How many channels were opened against this server.
    pub fn opened_channels(&self) -> usize {
        self.opened_channels.load(Ordering::SeqCst)
    }

    /// A channel straight into this server.
    pub fn channel(self: &Arc<Self>) -> Arc<dyn McpChannel> {
        self.opened_channels.fetch_add(1, Ordering::SeqCst);
        Arc::new(InProcessChannel {
            server: Arc::clone(self),
        })
    }

    /// A channel factory always opening channels into this server.
    pub fn factory(self: &Arc<Self>) -> Arc<dyn McpChannelFactory> {
        Arc::new(InProcessChannelFactory {
            server: Arc::clone(self),
        })
    }

    fn consume_drop(&self) -> bool {
        let mut remaining = self
            .drop_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *remaining > 0 {
            *remaining -= 1;
            true
        } else {
            false
        }
    }

    fn count_method(&self, method: &str) {
        *self
            .methods
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(method.to_string())
            .or_insert(0) += 1;
    }

    /// Answer one JSON-RPC request.
    pub async fn handle(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        self.count_method(&request.method);
        match request.method.as_str() {
            "initialize" => Ok(handle_initialize(
                &request,
                ServerInfo::for_server(self.name.clone()),
            )),
            "tools/list" => {
                let tools: Vec<serde_json::Value> = self
                    .tools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .map(|tool| {
                        serde_json::json!({
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": { "type": "object", "properties": {} },
                            "annotations": { "readOnlyHint": tool.read_only },
                        })
                    })
                    .collect();
                Ok(JsonRpcResponse::ok(
                    request.id.clone(),
                    serde_json::json!({ "tools": tools }),
                ))
            }
            "tools/call" => {
                let params = request
                    .params
                    .clone()
                    .unwrap_or_else(|| serde_json::json!({}));
                let name = params
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                self.calls
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push((name.clone(), arguments));
                let spec = self
                    .tools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .find(|tool| tool.name == name)
                    .cloned();
                let Some(spec) = spec else {
                    return Ok(JsonRpcResponse::err(
                        request.id.clone(),
                        JsonRpcError::new(TOOL_NOT_FOUND, format!("tool {name:?} not found")),
                    ));
                };
                if !spec.reply_delay.is_zero() {
                    tokio::time::sleep(spec.reply_delay).await;
                }
                let result = spec.reply_raw.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "content": [ { "type": "text", "text": spec.reply_text } ],
                        "isError": spec.reply_is_error,
                    })
                });
                Ok(JsonRpcResponse::ok(request.id.clone(), result))
            }
            "notifications/initialized" => Ok(JsonRpcResponse::ok(
                request.id.clone(),
                serde_json::json!({}),
            )),
            other => Ok(JsonRpcResponse::err(
                request.id.clone(),
                JsonRpcError::new(
                    JsonRpcError::CODE_METHOD_NOT_FOUND,
                    format!("method {other:?} not found"),
                ),
            )),
        }
    }
}

/// A channel straight into an [`InProcessMcpServer`].
pub struct InProcessChannel {
    server: Arc<InProcessMcpServer>,
}

#[async_trait]
impl McpChannel for InProcessChannel {
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        if self.server.consume_drop() {
            return Err(McpError::Transport("simulated link drop".to_string()));
        }
        self.server.handle(request).await
    }

    async fn notify(&self, notification: JsonRpcRequest) -> Result<(), McpError> {
        self.server.count_method(&notification.method);
        Ok(())
    }
}

/// A factory opening fresh [`InProcessChannel`]s into one server.
pub struct InProcessChannelFactory {
    server: Arc<InProcessMcpServer>,
}

#[async_trait]
impl McpChannelFactory for InProcessChannelFactory {
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(self.server.channel())
    }
}
