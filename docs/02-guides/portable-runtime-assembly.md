# Portable Runtime Assembly

[Document-Meta]
Document: docs/02-guides/portable-runtime-assembly.md
Version: 1.0
Last-Modified: 2026-10-28
Status: Implemented / Wired; validation recorded in the final acceptance report
Author: Codex

This entry composes the existing AgentRuntime. It does not provide another agent
loop, registry, permission system, provider client or scheduler. Hosts register
selected providers and modules on RuntimeBuilder and explicitly configure instance
tool scopes. No business tool is installed implicitly.

## Cargo feature matrix

| Assembly features | Included composition | Business tools automatically registered |
|---|---|---|
| default (`desktop`) | Existing desktop surface, memory, durable profile store, guard, organs, desktop tool implementations | None without Host composition |
| `default-features = false` | PortableHost + existing platform-neutral Runtime/Registry/Hook/Dispatcher/instances | None |
| minimal + `memory` | Existing MemoryModule and memory persistence dependencies | None; Host registers MemoryModule |
| minimal + `profile-store` | SQLite ProfileStore | None |
| minimal + both options | Namespace-aware MemoryModule + durable profile configuration | None |

The minimal normal dependency graph excludes apeireth-tools-canonical,
apeireth-memory, apeireth-storage, apeireth-organ, apeireth-guard, Gateway and
SQLite. It does not include the desktop Mesh/NAT/P2P implementation. Compiling a
platform-neutral executor API does not grant a process/shell tool to a model.
The default desktop feature preserves the existing public module/re-export paths;
the original composition source is in src/desktop.rs.

## Host entry

A host receives or constructs its provider-registered RuntimeBuilder. Permission
policy, credentials, event sink and optional module definitions are supplied there.
The following function does not create credentials or assume a provider exists:

```rust
use apeireth_plugin::agents::AgentConfig;
use apeireth_runtime::{AgentHostPolicy, RuntimeBuilder};
use apeireth_runtime_assembly::{AssemblyError, PortableHost};

async fn assemble(builder: RuntimeBuilder) -> Result<PortableHost, AssemblyError> {
    PortableHost::assemble(
        builder,
        AgentHostPolicy::default(),
        AgentConfig::default(), // empty tools: no business capability
    ).await
}
```

PortableHost supports submit, snapshot, explicit tool_scope, usage, tool_events,
bind_profile, cancel and shutdown. tool_events returns persisted ToolEventV1
facts for the current main session, including audited observations. RuntimeBuilder
still accepts RuntimeEventSink for live transport/UI observations. Provider
configuration stays with existing provider plugins; this crate does not read
Android settings or desktop environment variables for the minimal host.

cancel stops the current main instance; it is not a resumable interrupt. A profile
binding creates a fresh instance. Profile changes serialize separately from model
requests, stop/destroy the previous main role, and bind an explicit principal and
tool scope. A failed replacement leaves the previous instance stopped; it never
falls back to the previous private context. Loading a stored profile by itself
starts no agent. Hosts own approval UI and may use
AgentRuntime.resolve_agent_approval for pending instance approvals.

## Register a resource factory

A definition implements ModuleDefinition::manifest and instantiate. The factory
registers acquired resources immediately in ModuleResources; it returns an ordinary
Plugin with exactly the definition's manifest. Existing lifecycle validation and
Host permissions still apply. MCP uses this production path:

```rust
use std::sync::Arc;
use apeireth_plugin::resource_factory::module_from_definition;
use apeireth_tools_canonical::mcp_bridge::{McpDefinition, McpServerConfig, McpBridgeOptions};
use apeireth_runtime::RuntimeBuilder;

fn select_desktop_mcp(
    builder: RuntimeBuilder,
    config: McpServerConfig,
) -> Result<RuntimeBuilder, Box<dyn std::error::Error>> {
    let definition = Arc::new(McpDefinition::new(config, McpBridgeOptions::default())?);
    Ok(builder.with_plugin(module_from_definition(definition)))
}
```

That example selects an optional desktop module and requires the desktop tool
crate. Android minimal hosts do not select it. A single registry keeps at most
one live registration per Module ID; separate registries or later registrations
can instantiate the same shared definition with different ModuleInstanceIds.

After assembly, AgentRuntime provides bounded register_module,
refresh_module_tools, stop_module and unregister_module. To restart, unregister
only after successful stop/drain, then pass a new module_from_definition adapter.
A failed close retains ownership for retry; do not discard the registry entry or
force resource release. Old tool Arcs and factory approvals remain bound to the
old instance. Module-agent creation also requires an explicit ModuleAgentGrant;
registration does not inherit Host or parent-agent tools.

## Android boundary and verification

```text
cargo check -p apeireth-runtime-assembly --no-default-features --lib --target aarch64-linux-android --locked -j 1
cargo build -p apeireth-runtime-assembly --no-default-features --lib --target aarch64-linux-android --locked -j 1
cargo test -p apeireth-runtime-assembly --no-default-features --test portable_host --locked -j 2
cargo test -p apeireth-runtime-assembly --no-default-features --features memory,profile-store --test portable_host --locked -j 2
```

The first pair is actual target compilation of a Rust library. The second pair
runs offline Mock Provider turns on the Cloud Linux host. They are separate
evidence: no Android device, emulator, JNI or native UI is implied. Optional
SQLite builds for Android require an appropriate target C toolchain; the minimal
assembly does not require SQLite or the Android NDK linker to emit an rlib.

Native capabilities can implement the existing ToolCapability/Plugin contracts
and be selected by the host. JNI/FFI serialization and Android permission UI are
outside this batch. They must preserve explicit permission scopes, cancellation,
ToolEventV1 and audit boundaries.

Current verification and remaining production migration gaps are in the
[Kernel Convergence Final Acceptance Report](../04-internal/kernel-convergence-final-acceptance-2026-10-28.md).
