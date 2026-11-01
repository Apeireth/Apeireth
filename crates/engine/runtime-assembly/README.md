# apeireth-runtime-assembly

> Engine layer: the production composition of the runtime kernel — concrete Memory, Organ, Tool and SQLite wiring.

`apeireth-runtime-assembly` sits in the **engine** layer of the Apeireth engine
workspace. The runtime crate itself exposes only the lifecycle and port contracts;
this crate owns the concrete wiring that turns those ports into a running system.

## What is inside

- `canonical.rs` + `canonical/` — the canonical module graph: memory, organ, filesystem,
  search, repo, shell, fetch, doc ops, image ops, sheet ops, MCP, judge, reflexion,
  preference learning, self-assessment, self-status and council modules, assembled into
  `ProductionModules`
- `canonical/production.rs` — the production wiring and the effective default switch
  table for the built-in capabilities
- `sqlite_session.rs` — durable session ledger (SQLite)
- `sqlite_task_queue.rs` — persistent task queue, stored in the same session database
- `domain_plane.rs` — per-user data plane (three faces stored per domain, cross-domain
  access refused)
- `egress_gate.rs` / `egress_ledger.rs` — outbound data-egress policy and ledger
- `accountability.rs` / `accountability_probe.rs` / `consequence.rs` — accountability and
  consequence surfaces
- `startup.rs` — cold-start timeline with per-phase timing (`StartupTimeline`,
  `SystemStartupTimer`) and lazy-module build probes
- `notification_duty.rs`, `weather_probe.rs` — auxiliary duty and observation surfaces

## Honest limits

- Assembly is where "IMPLEMENTED" meets "PRODUCTION WIRED": a module existing in this
  crate does not mean its capability is enabled by default. The effective default
  switch table lives in `canonical/production.rs`.
- Individual capabilities carry their own approval requirements and fail-closed
  behaviour; the tool contract itself lives in `apeireth-tools-canonical`.

## Install

```toml
[dependencies]
apeireth-runtime-assembly = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/runtime-assembly>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/runtime-assembly/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
