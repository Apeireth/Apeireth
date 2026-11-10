# MCP tool integration

Status reviewed on 2026-10-29 against current protected main. This guide extracts the client/transport explanation from PR #19 and reconciles it with the current host implementation.

## Choose the integration layer

| Layer | Source | Current scope |
|---|---|---|
| Client library | `crates/capabilities/tools/src/mcp.rs` | **IMPLEMENTED** transport-agnostic initialize/list/call client |
| Host tool bridge | `crates/capabilities/tools/src/mcp_bridge/` and `crates/engine/runtime-assembly/src/canonical/production_mcp.rs` | **PRODUCTION WIRED**, **DEFAULT OFF**; module-owned tools, permission mapping and canonical Runtime dispatch |
| Protocol primitives | `crates/foundation/plugin/src/mcp/` | Reusable types and algorithms; they do not independently register tools or run a production host |

For ordinary external-server configuration, follow [MCP external tools](mcp-external-tools.md). For Host assembly and runtime ownership, see [portable runtime assembly](portable-runtime-assembly.md). Creating a client alone does not grant permission or make a tool available to an agent.

`ProductionModulesConfig.mcp` defaults to `false`. The CLI enables its host slot with `APEIRETH_ENABLE_MCP=1`; server configuration still has to parse and validate. Enabling the slot does not authorize every discovered action. No live external-server or hardware validation is claimed by this documentation extract.

## Client and transport contract

The library exposes `McpClient`, `McpTransport`, `McpToolDescriptor`, `McpToolResult`, `McpContent`, `McpError`, `JsonRpcRequest` and `JsonRpcResponse` from `apeireth_tools_canonical::mcp`.

`McpTransport::send_request` accepts an owned JSON-RPC request and asynchronously returns a decoded response or `McpError`. Implementations supply I/O; the library client itself does not open a process or socket. The production host bridge provides its own connection lifecycle and stdio/HTTP/SSE adapters, so an ordinary host does not need to invent another transport.

The client's sequence is:

```text
McpClient::new(server_name, Arc<dyn McpTransport>)
    initialize -> tools/list -> cache descriptors
    call_tool(name, structured_arguments) -> tools/call -> McpToolResult
```

`initialize(&mut self)` sends an initialize request and then refreshes the tool list. In this library implementation an error response becomes `HandshakeFailed`; it does not perform the full host connection state machine. The host bridge separately drives `ClientSession`, protocol-version validation, initialized notification and reconnect handling. Do not describe the simpler client as providing all those host guarantees.

`refresh_tools(&mut self)` replaces the cached descriptors after a valid list response. A descriptor contains `name`, `description` and `input_schema`; the wire schema key is `inputSchema`. `get_cached_tool(name)` reads that cache. `call_tool` rejects a name absent from the cache with `ToolNotFound` before sending a request.

For a caller that already has an authorized transport:

```rust
use std::sync::Arc;
use apeireth_tools_canonical::mcp::{McpClient, McpError, McpTransport};

async fn call_known_tool(transport: Arc<dyn McpTransport>) -> Result<String, McpError> {
    let mut client = McpClient::new("notes", transport);
    client.initialize().await?;
    let result = client.call_tool(
        "read_note",
        serde_json::json!({"name": "example"}),
    ).await?;
    if result.is_error {
        return Err(McpError::Transport("remote tool returned isError".into()));
    }
    Ok(result.extract_text())
}
```

This is a client-level example, not a production permission or registration adapter. It requires `read_note` to appear in the server's discovered list. `extract_text()` joins text blocks; image and resource blocks remain in `result.content` and need explicit handling if the caller supports them.

## Errors and governance

| Result | Caller action |
|---|---|
| `HandshakeFailed` | Refuse initialization; report the failure |
| `JsonRpc { code, message }` | Preserve the protocol error; do not treat it as a tool success |
| `Serialization` | Refuse a missing or malformed response payload |
| `Transport` | Surface I/O failure; a host may apply its bounded reconnect policy |
| `ToolNotFound` | Refresh discovery or correct the requested name |
| `McpToolResult.is_error` | Handle the remote tool error even though JSON-RPC transport succeeded |

Production actions use the host bridge's module ownership, permission mapping, monotonic deny guard, timeout and output contract. Those checks surround canonical dispatch; bypassing them by directly calling a library client is not a valid agent integration. Keep arguments structured, keep credentials out of raw tool arguments and logs, and use the host's redacted startup summaries.

Protocol primitives are not a second host. Historical implementation paths under `legacy/` are reference material and are outside this current integration guide. Future protocol or transport features need their own implementation and verification evidence.

## Reproduce the local evidence

Run from the repository root with the locked toolchain and dependency graph:

```sh
cargo test -p apeireth-tools-canonical --lib mcp:: --locked -j 2
cargo test -p apeireth-tools-canonical --test mcp_bridge_e2e --locked -j 2
cargo test -p apeireth-runtime-assembly --test mcp_bridge_runtime_e2e --locked -j 2
python scripts/docs_truth.py --verify --strict
python scripts/docs_links.py --check
python scripts/docs_index.py --check
```

These fixtures exercise local clients/servers and assembly contracts; passing them does not prove a live third-party server or hardware deployment. The workspace [test baseline](../03-reference/test-baseline.md) remains the source of repository-wide test totals.
