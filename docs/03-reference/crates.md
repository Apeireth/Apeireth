# Apeireth Crate Index

This index lists the 13 members of the root Cargo workspace. The independent
Tauri shell is documented with the frontend, and `legacy/` is excluded from
the product workspace.

## Foundation

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-core` | `crates/foundation/core` | Stable domain primitives, IDs, events, lifecycle, and clock vocabulary |
| `apeireth-protocol` | `crates/foundation/protocol` | Canonical normalized requests/results and provider protocol translation |
| `apeireth-plugin` | `crates/foundation/plugin` | Plugin lifecycle, capability descriptors, registries, and provider/tool contracts |
| `apeireth-governance` | `crates/foundation/governance` | Allow, deny, approval, input-security, and audit policy hooks |
| `apeireth-credentials` | `crates/foundation/credentials` | Credential storage backends and secret handling |

## Engine

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-runtime` | `crates/engine/runtime` | Canonical session runtime, provider selection, governance, tool dispatch, continuation, and trace |
| `apeireth-provider` | `crates/engine/provider` | Anthropic, MiniMax, and OpenAI-compatible provider capabilities |
| `apeireth-storage` | `crates/engine/storage` | SQLite pool, migrations, storage configuration, and errors |
| `apeireth-memory` | `crates/engine/memory` | Durable memory domain, retrieval, graph primitives, and vector indexing |

## Capabilities

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-tools-canonical` | `crates/capabilities/tools` | Built-in filesystem, search, repository, fetch, shell, and egress capabilities; sole `ProcessExecutor` owner |

## Adapters

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-gateway` | `crates/adapters/gateway` | HTTP transport adapter for the canonical runtime |
| `apeireth-cli` | `crates/adapters/cli` | CLI bootstrap and command dispatch through the canonical runtime |
| `apeireth-sdk` | `crates/adapters/sdk` | Public Rust/FFI SDK surface |

## Independent frontend workspace

`frontend/companion-desktop/` contains the Svelte 5 UI and thin Tauri 2 shell.
It is deliberately outside the root Cargo workspace and is checked by
`.github/workflows/companion-desktop-ci.yml`.

## Excluded historical material

`legacy/donor/`, `legacy/archived/`, and `legacy/frozen/` contain historical
implementations and references. They are not product crates and current code
must not depend on them. The former nested `reconstruction_v2/` workspace was
removed after its useful ideas were captured in the root workspace.
