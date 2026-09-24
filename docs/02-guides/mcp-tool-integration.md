# MCP Tool Integration

Apeireth provides a canonical MCP client bridge for discovering and invoking tools exposed by an external Model Context Protocol (MCP) server.

The current implementation lives in:

- `crates/capabilities/tools/src/mcp.rs`
- package: `apeireth-tools-canonical`

The MCP client is transport-agnostic. Transport I/O is provided through the `McpTransport` trait rather than being hard-coded into the client.

## How MCP fits into Apeireth

MCP is exposed through Apeireth's capability system. The runtime owns capability dispatch, while the capability registry contains model-facing capabilities including MCP.

At the client level, the integration looks like:

```text
McpClient
    │
    ├── initialize
    │
    ├── tools/list
    │
    └── tools/call
         │
         ▼
    McpTransport
         │
         ▼
    External MCP Server
```

The client constructs and processes the MCP JSON-RPC requests and responses. The transport is responsible for sending and receiving those requests.

## Core types

The canonical MCP bridge provides the following main types:

* `McpClient` — manages the MCP client-side lifecycle and cached tools
* `McpTransport` — abstraction for MCP request/response transport
* `McpToolDescriptor` — metadata and input schema for a discovered tool
* `McpToolResult` — result returned by a tool call
* `McpContent` — content blocks returned by a tool
* `McpError` — MCP, transport, serialization, and tool lookup errors

The module also exposes the JSON-RPC request and response types used by the transport layer.

```rust
use apeireth_tools_canonical::mcp::{
    JsonRpcRequest,
    JsonRpcResponse,
    McpClient,
    McpContent,
    McpError,
    McpToolDescriptor,
    McpToolResult,
    McpTransport,
};
```

## MCP client lifecycle

The normal client flow is:

```text
McpClient::new()
      │
      ▼
initialize()
      │
      ├── initialize
      │
      └── tools/list
             │
             ▼
      cache discovered tools
             │
             ▼
       call_tool()
             │
             └── tools/call
                    │
                    ▼
             McpToolResult
```

### 1. Create the client

`McpClient` receives a server name and an `Arc<dyn McpTransport>`:

```rust
let mut client = McpClient::new("my-server", transport);
```

The transport is supplied separately so that the client does not depend on a specific transport implementation.

### 2. Initialize the MCP session

Call `initialize()` before using the client:

```rust
client.initialize().await?;
```

The initialization request performs protocol negotiation and advertises the client's tool capability.

After the handshake succeeds, the client automatically refreshes the remote tool list.

### 3. Discover available tools

Tools can also be refreshed explicitly:

```rust
let tools = client.refresh_tools().await?;

for tool in &tools {
    println!("{}: {}", tool.name, tool.description);
}
```

Each discovered tool is represented by an `McpToolDescriptor`:

```rust
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

The client caches discovered tools by name.

You can inspect a cached tool with:

```rust
if let Some(tool) = client.get_cached_tool("sqlite_query") {
    println!("{}", tool.description);
}
```

## Calling an MCP tool

Call a discovered tool with structured JSON arguments:

```rust
let result = client
    .call_tool(
        "sqlite_query",
        serde_json::json!({
            "query": "SELECT * FROM users;"
        }),
    )
    .await?;
```

The request uses the MCP `tools/call` method with the tool name and arguments.

A tool must exist in the client's cached tool list before it can be called. Unknown tool names are rejected with `McpError::ToolNotFound`.

## Handling results

Tool results are represented by `McpToolResult`:

```rust
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}
```

For text-oriented results, `extract_text()` can be used:

```rust
let text = result.extract_text();
println!("{text}");
```

MCP content blocks can represent text, images, or resources.

For example:

```rust
match result.content.first() {
    Some(McpContent::Text { text }) => {
        println!("{text}");
    }
    Some(McpContent::Image { data, mime_type }) => {
        println!("image: {mime_type}, {} bytes", data.len());
    }
    Some(McpContent::Resource { uri, text }) => {
        println!("resource: {uri}");

        if let Some(text) = text {
            println!("{text}");
        }
    }
    None => {
        println!("empty MCP result");
    }
}
```

The `is_error` field should also be checked when the remote tool reports an error result.

## Error handling

The canonical client exposes explicit error variants through `McpError`:

```rust
pub enum McpError {
    JsonRpc {
        code: i64,
        message: String,
    },
    ToolNotFound(String),
    Serialization(String),
    Transport(String),
    HandshakeFailed(String),
}
```

A caller can handle them explicitly:

```rust
match client
    .call_tool(
        "sqlite_query",
        serde_json::json!({
            "query": "SELECT * FROM users;"
        }),
    )
    .await
{
    Ok(result) => {
        println!("{}", result.extract_text());
    }
    Err(McpError::ToolNotFound(name)) => {
        eprintln!("unknown MCP tool: {name}");
    }
    Err(McpError::Transport(reason)) => {
        eprintln!("transport error: {reason}");
    }
    Err(error) => {
        eprintln!("MCP call failed: {error}");
    }
}
```

## Implementing a transport

`McpClient` does not provide a concrete transport implementation itself.

Instead, integrate the transport you need through `McpTransport`:

```rust
use std::sync::Arc;

use async_trait::async_trait;
use apeireth_tools_canonical::mcp::{
    JsonRpcRequest,
    JsonRpcResponse,
    McpClient,
    McpError,
    McpTransport,
};

struct MyTransport;

#[async_trait]
impl McpTransport for MyTransport {
    async fn send_request(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        // Send the JSON-RPC request using the chosen transport.
        // Return the decoded JSON-RPC response.
        todo!()
    }
}

async fn example() -> Result<(), McpError> {
    let transport = Arc::new(MyTransport);
    let mut client = McpClient::new("my-server", transport);

    client.initialize().await?;

    let tools = client.refresh_tools().await?;

    for tool in tools {
        println!("{}: {}", tool.name, tool.description);
    }

    let result = client
        .call_tool(
            "my_tool",
            serde_json::json!({
                "input": "example"
            }),
        )
        .await?;

    println!("{}", result.extract_text());

    Ok(())
}
```

This keeps the client independent from the concrete transport mechanism.

For a transport-independent implementation example, see the `MockMcpTransport` used by the unit tests in:

```text
crates/capabilities/tools/src/mcp.rs
```

## The basic MCP request flow

At a protocol level, the canonical client follows this sequence:

### Initialize

The client sends an `initialize` JSON-RPC request containing the MCP protocol version, supported capabilities, and client information.

```text
initialize
    │
    ▼
MCP server
    │
    ▼
initialize response
```

### Discover tools

After initialization, the client requests the available tools:

```text
tools/list
    │
    ▼
{
    "tools": [...]
}
```

The returned descriptors are cached by the client.

### Invoke a tool

A discovered tool can then be invoked using:

```text
tools/call
    │
    ├── name
    └── arguments
         │
         ▼
      MCP server
         │
         ▼
    McpToolResult
```

This makes the client-side flow straightforward:

```text
initialize
    ↓
tools/list
    ↓
select discovered tool
    ↓
tools/call
    ↓
process McpToolResult
```

## Structured tool arguments

Tool arguments should be passed as structured JSON values rather than manually constructing JSON-RPC strings.

For example:

```rust
let arguments = serde_json::json!({
    "path": "/tmp/example.txt",
    "mode": "read"
});

let result = client
    .call_tool("read_file", arguments)
    .await?;
```

The tool's `input_schema` returned by `tools/list` describes the expected argument structure.

A consumer can inspect the schema before constructing a call:

```rust
if let Some(tool) = client.get_cached_tool("read_file") {
    println!("Input schema: {}", tool.input_schema);
}
```

## Security and integration boundaries

MCP tools represent external capabilities and should be treated accordingly.

When integrating an MCP transport:

* keep tool arguments structured rather than constructing shell commands or other executable strings;
* validate and constrain inputs at the appropriate capability boundary;
* do not place credentials or secrets directly into tool arguments;
* preserve Apeireth's existing capability and governance boundaries around external actions;
* handle transport and protocol failures explicitly rather than treating them as successful execution.

The MCP client does not replace Apeireth's governance or capability model. It provides the protocol bridge used to discover and invoke MCP tools.

## Current implementation boundaries

The canonical client in:

```text
crates/capabilities/tools/src/mcp.rs
```

currently implements the client-side:

```text
initialize
    ↓
tools/list
    ↓
tools/call
```

flow.

The transport remains abstracted behind `McpTransport`.

A separate MCP module exists under:

```text
crates/foundation/plugin/src/mcp/
```

This module contains reusable MCP protocol primitives and is default-off. It should not be treated as a second production MCP host or runtime dispatch implementation.

The current production client and the default-off protocol primitives therefore have different roles:

```text
Canonical MCP client
crates/capabilities/tools/src/mcp.rs
        │
        ▼
   McpTransport
        │
        ▼
 External MCP server
```

versus:

```text
MCP protocol primitives
crates/foundation/plugin/src/mcp/
        │
        └── reusable protocol support
             default-off
```

There is also an older implementation under:

```text
legacy/donor/apeireth-mcp/
```

That directory is historical/reference material and should not be used as the current MCP integration path.

When working on current MCP functionality, prefer:

```text
crates/capabilities/tools/src/mcp.rs
```

and the current runtime/capability architecture rather than the legacy donor implementation.

## A minimal integration pattern

A minimal consumer can therefore follow this pattern:

```rust
use std::sync::Arc;

use apeireth_tools_canonical::mcp::McpClient;

async fn run(transport: Arc<dyn apeireth_tools_canonical::mcp::McpTransport>) {
    let mut client = McpClient::new("my-server", transport);

    client.initialize().await.unwrap();

    let tools = client.refresh_tools().await.unwrap();

    for tool in &tools {
        println!("{}: {}", tool.name, tool.description);
    }

    let result = client
        .call_tool(
            "my_tool",
            serde_json::json!({
                "input": "example"
            }),
        )
        .await
        .unwrap();

    println!("{}", result.extract_text());
}
```

The important responsibilities are separated clearly:

```text
Consumer
   │
   ▼
McpClient
   │
   ├── initialize
   ├── discover tools
   ├── cache tool descriptors
   └── invoke tools
         │
         ▼
   McpTransport
         │
         ▼
   Concrete transport
         │
         ▼
   MCP server
```

## Related source

* `crates/capabilities/tools/src/mcp.rs`
* `crates/capabilities/tools/Cargo.toml`
* `crates/foundation/plugin/src/mcp/`
* `crates/engine/runtime/`
* `crates/engine/runtime-assembly/`
* `legacy/donor/apeireth-mcp/`

