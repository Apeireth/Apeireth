# Repository Layout

This is the current repository map for the `main` branch. New product
code belongs to one of the four ownership groups (`foundation`, `engine`,
`capabilities`, `adapters`) or to the independent frontend workspace; a crate
is created only when it has real code and an approved owner. No empty
placeholder directory is committed.

```text
Cargo.toml            # current product workspace (explicit members only)
crates/
├── foundation/       # stable contracts and policy vocabulary
│   ├── core/
│   ├── protocol/
│   ├── plugin/
│   ├── governance/
│   └── credentials/
├── engine/           # durable execution machinery
│   ├── runtime/
│   ├── provider/
│   ├── storage/
│   └── memory/
├── capabilities/     # model/runtime-facing actions
│   └── tools/        # package: apeireth-tools-canonical
└── adapters/         # external surfaces
    ├── gateway/
    ├── cli/
    └── sdk/
frontend/
└── companion-desktop/ # independent Svelte/Tauri workspace
scripts/              # maintenance, audit, release, and packaging tools
deploy/               # deployment manifests and service-specific images
packaging/            # platform package recipes
reports/              # committed validation and audit evidence
previews/             # committed visual preview assets
library/              # committed reference/library content
legacy/
├── Cargo.toml        # separate legacy workspace (reference-only)
├── donor/            # historical donor implementations
├── archived/         # obsolete historical code
└── frozen/           # intentionally untouched historical reference
docs/
├── 01-architecture/  # current architecture contracts and audits
├── 02-guides/        # quick start, user manual, deployment, development
├── 03-reference/     # crate index, API reference, glossary
├── 04-internal/      # design intent, policies, team material
├── development/      # contributor guides and layout rules
└── archive/          # historical design/round/decision records
```

## Physical vs logical

- Directory groups (`foundation`, `engine`, ...) express ownership category.
- Rust package names and import identities are unchanged.
- `apeireth-tools-canonical` lives at `crates/capabilities/tools` but its
  package name is still `apeireth-tools-canonical`.

## Ownership and dependency boundaries

| Area | Owns | May depend on | Must not own or depend on |
| --- | --- | --- | --- |
| `crates/foundation/` | stable domain types, protocol DTOs, plugin and governance contracts, credentials | standard library and approved external libraries; inward foundation dependencies | runtime orchestration, adapters, frontend, `legacy/` |
| `crates/engine/` | runtime composition, provider adapters, storage and memory machinery | foundation contracts and approved engine dependencies | HTTP entry routing, frontend UI, a second process executor |
| `crates/capabilities/tools/` | built-in tool capabilities and the canonical `ProcessExecutor` plus platform backends | foundation contracts and platform libraries | governance decisions, provider routing, `ProcessSupervisor`, process-tree model |
| `crates/adapters/` | CLI, gateway transport, and SDK entry surfaces | foundation, engine, and capabilities | business ownership, provider implementation, parallel orchestration |
| `frontend/companion-desktop/` | Svelte UI and thin Tauri shell; its own Node/Cargo workspaces | its frontend dependencies and the documented HTTP contract | root Cargo membership, runtime internals, direct process execution |
| `scripts/`, `deploy/`, `packaging/` | maintenance, validation, release, deployment, and package assembly | repository files and declared toolchains | product runtime logic or public API definitions |
| `docs/`, `reports/`, `previews/`, `library/` | documentation, evidence, visual assets, and reference content | source facts and generated evidence | production code ownership |
| `legacy/` | donor, archived, and frozen historical code | none from current product code | new production work or reverse dependencies into `crates/` |

`ProcessExecutor` remains the sole canonical process execution owner at
`crates/capabilities/tools/src/process/`. Its structured spawn, timeout,
bounded output, explicit cwd/env, and existing Windows/Linux/macOS containment
semantics are unchanged by repository organization work.

## Workspace

- Root `Cargo.toml` explicitly lists current product packages.
- `legacy/` is excluded from the root workspace and has its own workspace.
- Current product packages do not path-depend on `legacy/`; the dependency guard
  makes this boundary deterministic.
- Historical Companion code lives at `legacy/donor/apeireth-companion` and is
  intentionally outside the current workspace.
- `frontend/companion-desktop/src-tauri` is an independent Cargo workspace and
  is validated by its own CI workflow.
- `reconstruction_v2/` was a historical nested workspace and has been removed
  from git; an untracked local directory with leftover `target/` and database
  files may still exist on disk and is safe to delete.
- `crates/_archived/` holds untracked local build leftovers from the v1
  workspace; it is not repository content and is not part of any build.

## Deferred work

This cleanup does not implement or pre-create architecture for:

- `ProcessSupervisor` or an observable process-tree snapshot model;
- runtime telemetry, a risk/ML engine, Sentinel/EDR, filesystem isolation, or
  network isolation;
- stronger Linux cgroup containment or stronger macOS containment;
- broad frontend architecture changes, database migrations, or public contract
  changes.
