//! One connection per configured server: lifecycle, discovery, calls.
//!
//! The connection consumes the protocol library's pieces instead of copying
//! them: [`ClientSession`] drives the initialize lifecycle state machine
//! (New → Initializing → Ready, with `reset_for_reconnect` after a drop),
//! [`ReconnectState`] carries the disconnect budget and backoff, [`McpTool`]
//! / [`normalize_mcp_result`] normalize the wire shapes, and
//! [`is_valid_mcp_name`] polices tool names. The channel underneath is
//! whatever [`McpChannelFactory`] opens; a transport failure tears the link
//! down and rebuilds it, then re-discovers the tool list so the dynamic tool
//! catalog is never stale after a reconnect.

use std::sync::{Arc, Mutex};

use apeireth_plugin::mcp::{
    is_valid_mcp_name, negotiate_protocol_version, normalize_mcp_result, ClientCapabilities,
    ClientInfo, ClientSession, Id, JsonRpcRequest, McpTool, ReconnectState, ServerInfo,
    SessionState, ToolCallResult, MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSIONS,
};

use crate::mcp::McpError;

use super::config::{McpBridgeOptions, McpServerSpec};
use super::transport::{McpChannel, McpChannelFactory, SpecChannelFactory};

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

/// One live connection to one external server.
pub struct McpServerConnection {
    spec: McpServerSpec,
    factory: Arc<dyn McpChannelFactory>,
    channel: Mutex<Option<Arc<dyn McpChannel>>>,
    session: Mutex<ClientSession>,
    reconnect: Mutex<ReconnectState>,
    reconnects: std::sync::atomic::AtomicUsize,
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
            reconnect: Mutex::new(ReconnectState::new(options.reconnect.clone())),
            reconnects: std::sync::atomic::AtomicUsize::new(0),
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

    /// Open a fresh link, run the initialize handshake, and discover the
    /// tool list. Used for the first connect and for every reconnect.
    pub async fn connect(&self) -> Result<Vec<DiscoveredTool>, McpError> {
        let channel = self.factory.open().await?;
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
        self.connect().await.map(|_| ())
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
