# Development Guide

This guide describes the current repository, not the historical donor
workspaces. Start with [`repository-layout.md`](../development/repository-layout.md)
for ownership and dependency boundaries.

## Build and test loop

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
```

For the frozen process execution contract, run the focused integration suite:

```bash
cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture
```

The desktop frontend is an independent workspace:

```bash
cd frontend/companion-desktop
pnpm install --frozen-lockfile
pnpm check
pnpm build
cd src-tauri
cargo check --workspace --all-targets
```

The repository maintenance tools live under `scripts/`. The orphan check is
`scripts/audit/orphan-scan.ps1`; it reports candidates and never deletes code.

## Code map

| Concern | Owner | Start here |
| --- | --- | --- |
| Stable domain primitives | `apeireth-core` | `crates/foundation/core/src/kernel/` |
| Protocol DTOs and adapters | `apeireth-protocol` | `crates/foundation/protocol/src/canonical/` and `src/adapters/` |
| Plugin and capability registry | `apeireth-plugin` | `crates/foundation/plugin/src/` |
| Governance decisions | `apeireth-governance` | `crates/foundation/governance/src/` |
| Provider implementations and credentials resolution | `apeireth-provider` | `crates/engine/provider/src/` |
| Runtime orchestration | `apeireth-runtime` | `crates/engine/runtime/src/canonical/` |
| Storage and memory | `apeireth-storage` / `apeireth-memory` | `crates/engine/{storage,memory}/src/` |
| Built-in tools and process execution | `apeireth-tools-canonical` | `crates/capabilities/tools/src/` |
| HTTP and CLI surfaces | `apeireth-gateway` / `apeireth-cli` | `crates/adapters/{gateway,cli}/src/` |
| SDK surface | `apeireth-sdk` | `crates/adapters/sdk/src/` |
| Desktop UI and shell | `companion-desktop` | `frontend/companion-desktop/` |

## ProcessExecutor guardrails

`apeireth-tools-canonical::process::ProcessExecutor` is the only canonical
process execution owner. Keep these semantics unchanged when organizing code:

- requests remain structured (`executable`, argv, cwd, environment, limits);
- timeout and bounded stdout/stderr remain enforced;
- Windows keeps `CREATE_SUSPENDED → JobObject → Resume`;
- Linux and macOS keep their existing process-group and `pre_exec` behavior;
- callers do not receive a second process API from runtime, adapters, or UI.

Do not implement `ProcessSupervisor`, `ProcessTreeSnapshot`, a stronger
filesystem/network sandbox, or a new runtime-security layer as part of a
layout change.

## Contribution discipline

- Keep ownership changes separate from behavior changes.
- When moving a module, update all imports, manifests, CI paths, and docs in
  the same change.
- Before deleting a file, check imports, dynamic loading, feature flags, build
  scripts, fixtures, packaging, and platform conditionals.
- Prefer a module in an existing crate over a new crate unless the dependency
  boundary genuinely requires a package.
- Run the smallest relevant check after a move, then the full workspace checks
  before handoff.
- Record work that needs architecture or contract changes as deferred work;
  do not smuggle it into a cleanup commit.
