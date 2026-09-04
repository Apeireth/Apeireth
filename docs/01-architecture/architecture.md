# Apeireth Architecture

> 当前基线：默认分支 `main`，tag `v2.0.0-rc.1`（测试基线见 `research/baselines/`）。
> 根 Cargo workspace（17 个 crate）+ 独立的
> `frontend/companion-desktop` workspace。历史 donor 不属于生产依赖。

## Layer view

```text
frontend/companion-desktop
        │ HTTP / JSON
crates/adapters/{gateway,cli,sdk}
        │ transport adapters
crates/engine/runtime  ── kernel: governance / provider router / ports / events
        │
crates/engine/runtime-assembly ── concrete cognition / tools / Organ / SQLite
        │
crates/capabilities/tools  ── built-in capabilities + ProcessExecutor
        │
crates/foundation/{core,protocol,plugin,governance,credentials,orchestration}
```

The dependency direction points inward: adapters call the runtime, the runtime
composes providers/tools/governance, and foundation crates define the stable
contracts. The current dependency graph is intentionally explicit in the root
`Cargo.toml`.

## Current crate groups

| Group | Crates | Ownership |
| --- | --- | --- |
| Foundation | `apeireth-core`, `apeireth-protocol`, `apeireth-plugin`, `apeireth-governance`, `apeireth-credentials`, `apeireth-orchestration` | Stable domain primitives, normalized protocol types, capability/plugin contracts, governance decisions, credential backends, orchestration/council/context |
| Engine | `apeireth-runtime`, `apeireth-runtime-assembly`, `apeireth-provider`, `apeireth-storage`, `apeireth-memory`, `apeireth-perception`, `apeireth-organ` | Runtime kernel, concrete production assembly, vendor provider capabilities, SQLite/storage foundation, durable memory/retrieval, perception, and Organ domain |
| Capabilities | `apeireth-tools-canonical` | Built-in filesystem/search/repository/fetch/shell capabilities and the canonical process execution boundary |
| Adapters | `apeireth-gateway`, `apeireth-cli`, `apeireth-sdk` | HTTP/CLI/SDK entry surfaces; no second orchestration root |

There is no active product crate under `crates/modules/`. A future module is
created only when it has real implementation, an owner, a test/build reason,
and a documented dependency edge.

## Runtime and process ownership

`apeireth-runtime::canonical::Runtime` is the single user-facing Main Loop and
minimal microkernel. It owns session lifecycle, governance evaluation, provider
routing, behavior lifecycle dispatch, capability dispatch, event emission, and
continuation through abstract ports. `apeireth-runtime-assembly` is the
composition root that installs concrete behavior modules, tool capabilities,
Organ bridges, memory services, and SQLite session storage.

The runtime exposes separate registries:
- `BehaviorModule` / `BehaviorRegistry` contains lifecycle behaviors such as Memory, Preference, Judge, Council, SelfAssessment, and Organ cognition.
- `CapabilityProvider` / `CapabilityRegistry` contains model-facing actions such as Filesystem, Search, Repo, Shell, Fetch, and MCP.
- The microkernel boots with zero behaviors and zero capabilities and can execute plain chat turns without concrete product implementations.
- Modules can initiate bounded **SubLoops** (`apeireth_runtime::SubLoopSpawner`) running on private, ephemeral transcripts with explicit capability allowlists, never mutating the main session or emitting direct user chat output.

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

`frontend/companion-desktop/` is the in-tree Tauri 2 + Svelte 5 product UI.
Its Rust shell is thin and does not depend on the root Apeireth crates; its UI
uses the documented HTTP contract and live `/v1/apeireth/capabilities` gates.
Frontend tests and the mock upstream live under `frontend/companion-desktop/tests/`.

## Historical material

- `legacy/donor/`, `legacy/archived/`, and `legacy/frozen/` are reference-only;
  current crates must not import them.
- `docs/archive/` and dated audit reports preserve historical context and may
  mention paths that existed at the time of the audit.
- The former nested `reconstruction_v2/` workspace was removed from git after
  its useful decisions had been captured in the canonical root workspace. An
  untracked local directory (leftover `target/` and database files) may remain
  on disk and is safe to delete. Git history remains the recovery path.
- `crates/_archived/` holds untracked local build leftovers from the v1
  workspace; it is not repository content and is not part of any build.
