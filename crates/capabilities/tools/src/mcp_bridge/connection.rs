//! One connection per configured server: lifecycle, discovery, calls.
//!
//! The connection consumes the protocol library's pieces instead of copying
//! them: [`ClientSession`] drives the initialize lifecycle state machine
//! (New → Initializing → Ready, with `reset_for_reconnect` after a drop),
//! [`ReconnectState`] carries the disconnect budget and backoff, [`McpTool`]
//! / [`normalize_mcp_result`] normalize the wire shapes, and
//! [`is_valid_mcp_name`] polices tool names. The channel underneath is
//! whatever [`McpChannelFactory`] opens; a transport failure tears the link
//! down and rebuilds it, then re-discovers the tool list and hands that list
//! to the owner through [`McpRediscoverySink`] — a dropped list would leave
//! the dynamic tool catalog stale after a reconnect.
//!
//! # The stream-level fields (`id:` / `retry:`)
//!
//! Two event-stream fields carry state that no response path consumes: `id:`
//! (the Last-Event-ID a resumed stream continues from) and `retry:` (the
//! server's reconnection hint). The connection installs an observer on every
//! link it opens, so those fields are **observed** while the stream runs —
//! not after the link is already gone — and a disconnect hands them to the
//! protocol library's [`ReconnectState`] through its `observe_frame` entry
//! point. Without that hand-over the server's `retry:` was parsed and then
//! dropped on the floor: the reconnect always used the locally computed
//! backoff.
//!
//! The server's hint **replaces** the computed wait, so it is bounded by
//! [`bounded_server_retry`]: a server that asked for an hour would
//! otherwise pin the client for an hour. The number of attempts stays
//! bounded by the policy either way.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_plugin::mcp::{
    is_valid_mcp_name, negotiate_protocol_version, normalize_mcp_result, ClientCapabilities,
    ClientInfo, ClientSession, Id, JsonRpcRequest, McpTool, ReconnectPolicy, ReconnectState,
    ServerInfo, SessionState, SseFrame, ToolCallResult, MCP_PROTOCOL_VERSION,
    SUPPORTED_PROTOCOL_VERSIONS,
};

use crate::mcp::McpError;

use super::config::{McpBridgeOptions, McpServerSpec};
use super::transport::{McpChannel, McpChannelFactory, McpFrameObserver, SpecChannelFactory};

/// One tool a server exposes through `tools/list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTool {
    /// The server-side tool name (kebab-case, as declared).
    pub name: String,
    /// The server's description, when given.
    pub description: Option<String>,
    /// The declared input schema, when given.
    pub input_schema: Option<serde_json::Value>,
    /// The server's permission declaration for this tool: `true` only when
    /// the tool explicitly declares itself read-only
    /// (`annotations.readOnlyHint`).
    pub declared_read_only: bool,
}

/// The owner side of the reconnect hand-over.
///
/// The connection owns the wire, the owner owns the directory of live tools.
/// A reconnect re-runs `tools/list`, and that list is the only fresh
/// knowledge about the server's tool set — so the connection hands it over
/// instead of dropping it. A dropped list is a stale directory: the model
/// keeps seeing a tool the server has dropped, and never sees the one it
/// has added.
pub trait McpRediscoverySink: Send + Sync {
    /// Adopt the tool list a reconnect re-discovered for `server`.
    ///
    /// A refused adoption leaves the owner's directory as it is — never
    /// half-applied, never cleared. The refusal is reported back so it can
    /// be observed instead of swallowed.
    fn on_rediscovery(&self, server: &str, tools: Vec<DiscoveredTool>) -> Result<(), String>;
}

/// The stream-level state a server hands out through `id:` / `retry:`.
///
/// Both fields are **observed** while the stream runs (they arrive on frames
/// no response path looks at) and are consumed when the link drops: the
/// `id:` resumes the next link from the same place, the `retry:` becomes the
/// reconnect wait.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StreamHints {
    /// The last `id:` the server sent — the Last-Event-ID.
    pub last_event_id: Option<String>,
    /// The last `retry:` the server sent, **already bounded** by the policy
    /// ceiling (see [`bounded_server_retry`]).
    pub server_retry: Option<Duration>,
}

/// The reconnection wait a server's `retry:` hint may cause.
///
/// The hint **replaces** the locally computed backoff (that is what the
/// server asks for: "wait this long before you come back"), so it must stay
/// inside the policy's own envelope. The envelope is
/// [`ReconnectPolicy::max_backoff`] — the very ceiling the computed backoff
/// is already capped to — so a hint can shorten the wait but never push the
/// client outside the window the policy allows. Without the bound a server
/// could pin the client for an arbitrary time (`retry: 3600000` = one hour).
///
/// The **number** of attempts is bounded separately: the policy's
/// `max_attempts` is checked before the wait is handed out, so a hint can
/// never extend the reconnect budget.
pub fn bounded_server_retry(hint: Duration, max_backoff: Duration) -> Duration {
    hint.min(max_backoff)
}

/// The observer a connection installs on every link it opens.
///
/// It records the two stream-level fields and nothing else: **observation
/// only** — the link's routing decisions stay in the transport.
struct HintObserver {
    hints: Arc<Mutex<StreamHints>>,
    max_backoff: Duration,
}

impl McpFrameObserver for HintObserver {
    fn observe_frame(&self, frame: &SseFrame) {
        let mut hints = self
            .hints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(id) = &frame.id {
            if !id.is_empty() {
                hints.last_event_id = Some(id.clone());
            }
        }
        if let Some(retry) = frame.retry {
            hints.server_retry = Some(bounded_server_retry(retry, self.max_backoff));
        }
    }
}

/// One live connection to one external server.
pub struct McpServerConnection {
    spec: McpServerSpec,
    factory: Arc<dyn McpChannelFactory>,
    channel: Mutex<Option<Arc<dyn McpChannel>>>,
    session: Mutex<ClientSession>,
    reconnect: Arc<Mutex<ReconnectState>>,
    policy: ReconnectPolicy,
    hints: Arc<Mutex<StreamHints>>,
    reconnects: std::sync::atomic::AtomicUsize,
    sink: Mutex<Option<Arc<dyn McpRediscoverySink>>>,
    rediscovery_error: Mutex<Option<String>>,
}

impl McpServerConnection {
    /// A connection for `spec`, opening channels through `factory`.
    pub fn new(
        spec: McpServerSpec,
        factory: Arc<dyn McpChannelFactory>,
        options: &McpBridgeOptions,
    ) -> Self {
        Self {
            spec,
            factory,
            channel: Mutex::new(None),
            session: Mutex::new(ClientSession::new(ClientInfo::new(
                "apeireth-mcp-bridge",
                env!("CARGO_PKG_VERSION"),
            ))),
            reconnect: Arc::new(Mutex::new(ReconnectState::new(options.reconnect.clone()))),
            policy: options.reconnect.clone(),
            hints: Arc::new(Mutex::new(StreamHints::default())),
            reconnects: std::sync::atomic::AtomicUsize::new(0),
            sink: Mutex::new(None),
            rediscovery_error: Mutex::new(None),
        }
    }

    /// A connection for `spec` that opens channels matching its transport.
    pub fn from_spec(spec: McpServerSpec, options: &McpBridgeOptions) -> Self {
        let factory = Arc::new(SpecChannelFactory::new(spec.clone()));
        Self::new(spec, factory, options)
    }

    /// The configured server name.
    pub fn server_name(&self) -> &str {
        &self.spec.name
    }

    /// The configured spec.
    pub fn spec(&self) -> &McpServerSpec {
        &self.spec
    }

    /// Current lifecycle state of the client session.
    pub fn session_state(&self) -> SessionState {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .state()
    }

    /// Reconnect attempts recorded since the last successful connect.
    pub fn reconnect_attempts(&self) -> u32 {
        self.reconnect
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .attempts()
    }

    /// How many disconnect recoveries this connection has run in total.
    pub fn reconnect_count(&self) -> usize {
        self.reconnects.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The last event id the server handed out on this link's stream.
    ///
    /// `None` = the server never sent an `id:` field (the first link of a
    /// server, or a server that does not resume streams).
    pub fn last_event_id(&self) -> Option<String> {
        self.hints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .last_event_id
            .clone()
    }

    /// The reconnection wait the server asked for on its last `retry:` frame.
    ///
    /// `None` = the server never sent a `retry:` field ⇒ the reconnect uses
    /// the locally computed backoff. The value is **already bounded** by the
    /// policy ceiling (see [`bounded_server_retry`]).
    pub fn server_retry(&self) -> Option<Duration> {
        self.hints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .server_retry
    }

    /// Install the owner side of the reconnect hand-over.
    ///
    /// The bridge installs one sink per connection: it is the only way a
    /// re-discovered tool list reaches the dynamic directory. Without a sink
    /// the connection keeps its own list to itself (a standalone connection
    /// has no directory to update).
    pub fn set_rediscovery_sink(&self, sink: Arc<dyn McpRediscoverySink>) {
        *self
            .sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(sink);
    }

    /// The last refusal of the owner side of a reconnect hand-over.
    ///
    /// `None` means every re-discovered list was adopted (or that no sink is
    /// installed). A refusal leaves the directory untouched; it is reported
    /// here rather than swallowed.
    pub fn last_rediscovery_error(&self) -> Option<String> {
        self.rediscovery_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Open a fresh link, run the initialize handshake, and discover the
    /// tool list. Used for the first connect and for every reconnect.
    pub async fn connect(&self) -> Result<Vec<DiscoveredTool>, McpError> {
        // The link observes the stream-level fields (`id:` / `retry:`) as it
        // reads them, and resumes from the last event id the dead link
        // handed out.
        let observer = Arc::new(HintObserver {
            hints: Arc::clone(&self.hints),
            max_backoff: self.policy.max_backoff,
        });
        let last_event_id = self
            .hints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .last_event_id
            .clone();
        let channel = self
            .factory
            .open_with_observer(observer, last_event_id.as_deref())
            .await?;
        // Fresh link, fresh handshake: close whatever state the previous
        // link left behind before the lifecycle machine starts again.
        {
            let mut session = self
                .session
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match session.state() {
                SessionState::New => {}
                SessionState::Closed => {
                    session
                        .reset_for_reconnect()
                        .map_err(|e| McpError::HandshakeFailed(e.to_string()))?;
                }
                _ => {
                    session.close();
                    session
                        .reset_for_reconnect()
                        .map_err(|e| McpError::HandshakeFailed(e.to_string()))?;
                }
            }
        }

        self.handshake(&channel).await?;
        let tools = self.list_tools(&channel).await?;

        *self
            .channel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(channel);
        self.reconnect
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .on_connected();
        Ok(tools)
    }

    /// Re-discover the tool list on the current link.
    pub async fn discover(&self) -> Result<Vec<DiscoveredTool>, McpError> {
        let channel = self.current_channel()?;
        self.list_tools(&channel).await
    }

    /// Call one server-side tool and normalize the result structure.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, McpError> {
        match self.call_once(tool_name, arguments.clone()).await {
            Err(McpError::Transport(reason)) => {
                // The link is gone: rebuild it (backoff budget from the
                // reconnect policy), re-discover, and retry exactly once.
                self.recover(&reason).await?;
                let result = self.call_once(tool_name, arguments).await?;
                Ok(result)
            }
            other => other,
        }
    }

    async fn recover(&self, reason: &str) -> Result<(), McpError> {
        self.reconnects
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Hand the stream-level state the dead link observed to the protocol
        // library's reconnect state: the `id:` resumes the next link, the
        // `retry:` becomes the wait below. **Before** the wait is handed out
        // — afterwards it is too late for this disconnect.
        self.observe_stream_hints();
        {
            let mut channel = self
                .channel
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *channel = None;
        }
        {
            let mut session = self
                .session
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            session.close();
            session
                .reset_for_reconnect()
                .map_err(|e| McpError::Transport(e.to_string()))?;
        }
        let wait = self
            .reconnect
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .on_disconnect()
            .ok_or_else(|| {
                McpError::Transport(format!(
                    "reconnect budget exhausted after transport failure: {reason}"
                ))
            })?;
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        // The reconnect re-ran `tools/list`: that list is the only fresh
        // knowledge about the server's tool set, so it flows back to the
        // owner (the dynamic directory) instead of being dropped here. A
        // dropped list is a stale directory — the model would keep seeing a
        // tool the server dropped, and miss the one it added.
        let tools = self.connect().await?;
        self.publish_rediscovery(tools);
        Ok(())
    }

    /// Hand the observed stream-level state to the reconnect state.
    ///
    /// The protocol library keeps the Last-Event-ID and the server's
    /// reconnection hint in [`ReconnectState`] — this is the entry point
    /// that fills them (`observe_frame`). The hint is already bounded (see
    /// [`bounded_server_retry`]); the event id is the server's own value.
    ///
    /// Doing this here — **before** `on_disconnect()` hands out the wait —
    /// is what makes the server's `retry:` reach the reconnect: without it
    /// the wait was always the locally computed backoff.
    fn observe_stream_hints(&self) {
        let hints = self
            .hints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if hints.last_event_id.is_none() && hints.server_retry.is_none() {
            return;
        }
        let mut reconnect = self
            .reconnect
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reconnect.observe_frame(&SseFrame {
            id: hints.last_event_id,
            retry: hints.server_retry,
            ..SseFrame::default()
        });
    }

    /// Hand a re-discovered tool list to the owner.
    ///
    /// The sink is taken out of its slot before it is called, so the owner
    /// never runs while the connection's slot is locked. A refusal is kept
    /// as the connection's last rediscovery error; it never fails the call
    /// that triggered the reconnect.
    fn publish_rediscovery(&self, tools: Vec<DiscoveredTool>) {
        let sink = self
            .sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(sink) = sink else {
            return;
        };
        let outcome = sink.on_rediscovery(self.spec.name.as_str(), tools);
        *self
            .rediscovery_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = outcome.err();
    }

    async fn call_once(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, McpError> {
        let channel = self.current_channel()?;
        let id = self.next_id()?;
        let request = JsonRpcRequest::new(
            "tools/call",
            Some(serde_json::json!({ "name": tool_name, "arguments": arguments })),
            id,
        );
        let response = channel.request(request).await?;
        let value = response.into_result().map_err(|e| McpError::JsonRpc {
            code: i64::from(e.code),
            message: e.message,
        })?;
        normalize_mcp_result(value).map_err(McpError::Serialization)
    }

    fn current_channel(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .ensure_ready()
            .map_err(|e| McpError::Transport(e.to_string()))?;
        self.channel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .ok_or_else(|| McpError::Transport("no live channel".to_string()))
    }

    fn next_id(&self) -> Result<Id, McpError> {
        let mut session = self
            .session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        session
            .ensure_ready()
            .map_err(|e| McpError::Transport(format!("session not ready for request ids: {e}")))?;
        Ok(session.next_id())
    }

    async fn handshake(&self, channel: &Arc<dyn McpChannel>) -> Result<(), McpError> {
        let request = {
            let mut session = self
                .session
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            session
                .begin_initialize(&ClientCapabilities::default())
                .map_err(|e| McpError::HandshakeFailed(e.to_string()))?
        };
        let response = channel.request(request).await.map_err(|e| {
            self.fail_handshake();
            e
        })?;
        let value = response.into_result().map_err(|e| {
            self.fail_handshake();
            McpError::HandshakeFailed(format!("code={}: {}", e.code, e.message))
        })?;
        let info: ServerInfo = serde_json::from_value(value).map_err(|e| {
            self.fail_handshake();
            McpError::HandshakeFailed(format!("initialize result malformed: {e}"))
        })?;
        let negotiated = negotiate_protocol_version(MCP_PROTOCOL_VERSION, &info.protocolVersion);
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&negotiated.as_str()) {
            self.fail_handshake();
            return Err(McpError::HandshakeFailed(format!(
                "server protocol version {} is not supported from {}",
                info.protocolVersion, MCP_PROTOCOL_VERSION
            )));
        }
        {
            let mut session = self
                .session
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            session
                .complete_initialize(info)
                .map_err(|e| McpError::HandshakeFailed(e.to_string()))?;
        }
        channel
            .notify(JsonRpcRequest::notification(
                "notifications/initialized",
                None,
            ))
            .await
    }

    fn fail_handshake(&self) {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .fail_initialize();
    }

    async fn list_tools(
        &self,
        channel: &Arc<dyn McpChannel>,
    ) -> Result<Vec<DiscoveredTool>, McpError> {
        let id = self.next_id()?;
        let request = JsonRpcRequest::new("tools/list", None, id);
        let response = channel.request(request).await?;
        let value = response.into_result().map_err(|e| McpError::JsonRpc {
            code: i64::from(e.code),
            message: e.message,
        })?;
        parse_tool_list(value)
    }
}

/// Parse one `tools/list` result into discovered tools.
///
/// The wire shape goes through the protocol library's [`McpTool`] (so both
/// `inputSchema` and `input_schema` spellings are accepted); the server's
/// permission declaration is read from `annotations.readOnlyHint`. A tool
/// whose name violates the MCP name grammar is a hard error — a malformed
/// catalog never reaches the dynamic tool registry.
pub fn parse_tool_list(value: serde_json::Value) -> Result<Vec<DiscoveredTool>, McpError> {
    let tools = value
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            McpError::Serialization("tools/list result has no tools array".to_string())
        })?;
    let mut out = Vec::with_capacity(tools.len());
    for raw in tools {
        let descriptor: McpTool = serde_json::from_value(raw.clone())
            .map_err(|e| McpError::Serialization(format!("tool descriptor malformed: {e}")))?;
        if !is_valid_mcp_name(&descriptor.name) {
            return Err(McpError::Serialization(format!(
                "tool name {:?} violates the MCP name grammar",
                descriptor.name
            )));
        }
        let declared_read_only = raw
            .get("annotations")
            .and_then(|a| a.get("readOnlyHint"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        out.push(DiscoveredTool {
            name: descriptor.name,
            description: descriptor.description,
            input_schema: descriptor.input_schema,
            declared_read_only,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_list_reads_names_schemas_and_read_only_declarations() {
        let value = serde_json::json!({
            "tools": [
                {
                    "name": "lookup-item",
                    "description": "Look one item up",
                    "inputSchema": { "type": "object", "properties": {} },
                    "annotations": { "readOnlyHint": true }
                },
                {
                    "name": "send-item",
                    "input_schema": { "type": "object" }
                }
            ]
        });
        let tools = parse_tool_list(value).unwrap();
        assert_eq!(tools.len(), 2);
        assert!(tools[0].declared_read_only);
        assert!(!tools[1].declared_read_only);
        assert_eq!(tools[0].description.as_deref(), Some("Look one item up"));
    }

    #[test]
    fn parse_tool_list_rejects_illegal_names_and_missing_array() {
        let bad = serde_json::json!({ "tools": [{ "name": "CamelCase" }] });
        assert!(parse_tool_list(bad).is_err());
        assert!(parse_tool_list(serde_json::json!({ "tools": 7 })).is_err());
    }
}
