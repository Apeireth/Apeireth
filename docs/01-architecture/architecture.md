# Apeireth Architecture

> 当前基线：根 Cargo workspace（13 个 crate）+ 独立的
> `frontend/companion-desktop` workspace。历史 donor 不属于生产依赖。

## Layer view

```text
frontend/companion-desktop
        │ HTTP / JSON
crates/adapters/{gateway,cli,sdk}
        │ transport adapters
crates/engine/runtime  ── governance / providers / storage
        │
crates/capabilities/tools  ── built-in capabilities + ProcessExecutor
        │
crates/foundation/{core,protocol,plugin,governance,credentials}
```

The dependency direction points inward: adapters call the runtime, the runtime
composes providers/tools/governance, and foundation crates define the stable
contracts. The current dependency graph is intentionally explicit in the root
`Cargo.toml`.

## Current crate groups

| Group | Crates | Ownership |
| --- | --- | --- |
| Foundation | `apeireth-core`, `apeireth-protocol`, `apeireth-plugin`, `apeireth-governance`, `apeireth-credentials` | Stable domain primitives, normalized protocol types, capability/plugin contracts, governance decisions, credential backends |
| Engine | `apeireth-runtime`, `apeireth-provider`, `apeireth-storage`, `apeireth-memory` | Runtime composition, vendor provider capabilities, SQLite/storage foundation, durable memory and retrieval |
| Capabilities | `apeireth-tools-canonical` | Built-in filesystem/search/repository/fetch/shell capabilities and the canonical process execution boundary |
| Adapters | `apeireth-gateway`, `apeireth-cli`, `apeireth-sdk` | HTTP/CLI/SDK entry surfaces; no second orchestration root |

There is no active product crate under `crates/modules/`. A future module is
created only when it has real implementation, an owner, a test/build reason,
and a documented dependency edge.

## Runtime and process ownership

`apeireth-runtime::canonical::Runtime` owns session lifecycle, governance
evaluation, provider selection, tool dispatch, continuation, and trace. The
gateway and CLI only translate their transport inputs and invoke the runtime.

`apeireth-tools-canonical::process::ProcessExecutor` is the sole process
execution owner. Its public structured request/result contract and current
platform behavior are frozen:

| Property | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Structured spawn, cwd, environment, timeout, bounded stdout/stderr | Enforced | Enforced | Enforced |
| Process-tree containment | Job Object, enforced | Process group, partial | Process group, partial |
| Pre-exec containment | `CREATE_SUSPENDED → JobObject → Resume` | Existing `pre_exec` setup | Existing `pre_exec` setup |

This cleanup does not add `ProcessSupervisor`, `ProcessTreeSnapshot`, a new
process-tree runtime model, filesystem/network sandboxing, or security engine.

## Desktop boundary

`frontend/companion-desktop/` is an independent Svelte 5 + Tauri 2 workspace.
Its Rust shell is thin and does not depend on the root Apeireth crates; its UI
uses the documented HTTP contract. Frontend tests and the mock upstream live
under `frontend/companion-desktop/tests/`.

## Historical material

- `legacy/donor/`, `legacy/archived/`, and `legacy/frozen/` are reference-only;
  current crates must not import them.
- `docs/archive/` and dated audit reports preserve historical context and may
  mention paths that existed at the time of the audit.
- The former nested `reconstruction_v2/` workspace was removed after its
  useful decisions had been captured in the canonical root workspace. Git
  history remains the recovery path.
