# apeireth-tools-canonical

> Capabilities layer: the canonical builtin tool capabilities, implemented against the `apeireth-plugin` tool contract.

`apeireth-tools-canonical` sits in the **capabilities** layer of the Apeireth engine
workspace. The tools own their identity, input schema, execution implementation and
structured result. They do **not** own the runtime, the gateway, sessions, providers,
governance or the dispatch loop — the runtime reaches them only through the canonical
plugin/capability registry path.

## Ownership and direction

Builtin tools are plugin capabilities. They depend on the canonical
plugin/protocol/core contracts, never on the runtime or the gateway. The governance edge
is one-way and interface-only: the execution chain's guard stage consults the deny-only
guard interface, and no policy is owned here.

## Default switch table

- **Default enabled**: filesystem / search / repo
- **Opt-in, default off**: shell / fetch
- Egress policy and the single `ProcessExecutor` boundary (Windows Job Object +
  `CREATE_SUSPENDED`, three-OS isolation) gate the process-facing surfaces

## What is inside

- `filesystem.rs`, `search.rs`, `repo.rs`, `repo_map.rs` — the default-on read/search faces
- `shell.rs`, `fetch.rs`, `egress.rs` — the opt-in faces and the egress policy
- `process/` — the single `ProcessExecutor` boundary (`ProcessRequest`, `ProcessLimits`)
- `apply_patch.rs`, `apply_patch_tool.rs`, `git_write.rs` — controlled write surfaces
- `doc_ops.rs`, `sheet_ops.rs`, `image_ops.rs` — document / sheet / image creation faces
- `backup.rs` — directory-snapshot backup with a per-file SHA-256 manifest
- `mcp.rs`, `mcp_bridge/` — MCP bridge
- `exec_pipeline.rs`, `guardrail.rs`, `observed_gate.rs`, `sensitive_path.rs`,
  `write_intent.rs`, `redaction_bridge.rs`, `escalation.rs` — the five-stage execution
  pipeline and its gates
- `content_feed/`, `device/`, `voice/`, `recon/`, `spill/`, `worktree_apply.rs`,
  `skill/`, `external.rs`, `proposal.rs`, `proposal_push.rs`, `supervisor.rs` — the
  remaining capability faces

## Honest limits

- `ecosystem_health.rs` (tool-ecosystem diversity / redundancy / overlap metrics) is an
  implemented kernel module that is **not yet wired** into the runtime.
- Several faces (doc ops, sheet ops, device sensing, image ops, content feed) are
  **register-gated and default off**, fail-closed, and require approval on their write
  paths. Their presence in this crate does not mean they are enabled in your install.
- Image operations delegate to the engine image core; this crate no longer depends on
  an external image library directly.

## Install

```toml
[dependencies]
apeireth-tools-canonical = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/capabilities/tools>
- Manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/capabilities/tools/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
