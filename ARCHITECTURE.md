# Apeireth Architecture

This document describes the current product baseline in the root repository.
Historical design proposals and donor implementations live under `docs/archive/`,
`legacy/`, and `reports/`; they are not part of the built product workspace.

## Current structure

```text
apeireth-rust/
├── crates/
│   ├── foundation/       # stable domain types, protocol, plugins, policy, credentials
│   ├── engine/            # runtime orchestration, providers, storage, memory
│   ├── capabilities/      # canonical tools and process execution
│   └── adapters/          # gateway, CLI, SDK entry surfaces
├── frontend/companion-desktop/  # independent Svelte + Tauri workspace
├── scripts/               # validation, audit, release, and maintenance tooling
├── deploy/                # deployment-specific artifacts retained separately
├── packaging/             # package build metadata and installers
├── docs/                  # current guides, architecture, reference, and archive
├── legacy/                # donor/reference code; excluded from the root workspace
├── reports/               # durable audit and validation evidence
└── previews/              # design/reference assets
```

The root `Cargo.toml` is the only product Rust workspace. It contains thirteen
packages, grouped by responsibility rather than development history:

| Group | Packages | Responsibility |
| --- | --- | --- |
| Foundation | `apeireth-core`, `apeireth-protocol`, `apeireth-plugin`, `apeireth-governance`, `apeireth-credentials` | stable types, wire contracts, plugin contracts, policy, credential resolution |
| Engine | `apeireth-runtime`, `apeireth-provider`, `apeireth-storage`, `apeireth-memory` | execution orchestration, provider adapters, persistence, memory domain |
| Capabilities | `apeireth-tools-canonical` | built-in tools and the single process-execution boundary |
| Adapters | `apeireth-gateway`, `apeireth-cli`, `apeireth-sdk` | HTTP gateway, command-line entry point, SDK surface |

`frontend/companion-desktop/src-tauri` has its own workspace and is deliberately
not a member of the root workspace. `legacy/` is reference material only and is
never imported by current crates. The former nested `reconstruction_v2/`
workspace and the empty `crates/modules/` placeholder were removed during the
pre-freeze cleanup.

## Dependency direction

Dependencies flow inward toward stable contracts and downward through the
runtime layers:

```text
foundation/core ──┐
foundation/protocol ──┼──> foundation/plugin/governance/credentials
                     │
engine/storage ──────┼──> engine/memory
                     └──> engine/provider ──┐
engine/runtime ─────────────────────────────┼──> adapters/gateway
capabilities/tools ────────────────────────┘
adapters/cli ──> runtime + gateway + provider + tools
adapters/sdk ──> protocol
```

The effective package edges are:

```text
governance -> core
protocol -> core
plugin -> core, protocol
memory -> storage, core
provider -> core, plugin, protocol
tools-canonical -> core, plugin, protocol
runtime -> core, governance, plugin, protocol, storage
gateway -> core, protocol, runtime
cli -> core, gateway, plugin, provider, runtime, tools-canonical
sdk -> protocol
```

Foundation packages do not depend on adapters. Runtime owns orchestration;
gateway and CLI translate external requests and do not create a second runtime.

## Ownership boundaries

| Concern | Canonical owner | Boundary rule |
| --- | --- | --- |
| Core IDs, lifecycle, events, philosophy contract | `crates/foundation/core` | stable shared primitives only |
| Wire/request/response DTOs | `crates/foundation/protocol` | public protocol types and vendor wire adapters |
| Plugin and capability contracts | `crates/foundation/plugin` | declarations and lifecycle contracts, not tool implementations |
| Policy decisions | `crates/foundation/governance` | policy evaluation; no transport or process spawning |
| Credential resolution | `crates/foundation/credentials` | backend behind the plugin credential contract |
| Runtime/session/execution loop | `crates/engine/runtime` | owns orchestration, approvals, provider selection, and trace |
| Provider transport and routing inputs | `crates/engine/provider` | provider implementations and response normalization |
| Durable storage | `crates/engine/storage` | SQLite pool, writer, and migrations |
| Memory domain and retrieval | `crates/engine/memory` | memory entities, repository, retrieval, vector/graph primitives |
| Built-in tools and process execution | `crates/capabilities/tools` | one canonical tool/process boundary |
| HTTP gateway | `crates/adapters/gateway` | transport only; delegates execution to runtime |
| CLI | `crates/adapters/cli` | session, chat, and gateway commands |
| SDK | `crates/adapters/sdk` | client-facing integration surface |
| Desktop UI and shell | `frontend/companion-desktop` | independent frontend/Tauri workspace |

## Process execution boundary

`ProcessExecutor` is owned by `apeireth-tools-canonical` at
`crates/capabilities/tools/src/process/`. `RepoTool` and other built-in tools
must use this boundary; no adapter or frontend may create a competing executor.

The pre-freeze cleanup did not modify its contract or implementation. The
existing behavior remains the source of truth for:

- structured spawn, explicit `cwd` and environment handling;
- bounded stdout/stderr capture and timeout behavior;
- Windows `CREATE_SUSPENDED → JobObject → Resume` ordering;
- Linux process-group handling and existing partial containment;
- macOS process-group handling and existing partial containment.

`ProcessSupervisor`, observable process-tree snapshots, and stronger containment
are deferred work, not hidden responsibilities of this module.

## External boundaries

The CLI exposes the current product entry points:

```text
apeireth session
apeireth chat
apeireth gateway serve --port 8080
```

The gateway owns HTTP transport and exposes the health endpoint at `/health`.
Provider credentials are resolved by the provider/credentials path; the desktop
frontend remains a separate build and release boundary.

## Deferred work

This cleanup intentionally does not implement or redesign:

- `ProcessSupervisor` or a process-tree data model;
- runtime telemetry, risk scoring, ML risk engines, Sentinel, or EDR;
- filesystem or network isolation;
- stronger Linux cgroup containment or stronger macOS containment;
- a second runtime, scheduler semantics, public API, IPC/schema, or database migration;
- a frontend architecture rewrite or a new product module.

These items require separate design and ownership decisions. They should be
introduced only with an explicit contract, tests, and an owner under the current
layout.

## Verification baseline

From the repository root, the baseline checks are:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture
python scripts/check_no_legacy_deps.py
```

For the independent desktop workspace, run the checks described in
`frontend/companion-desktop/README.md`.
