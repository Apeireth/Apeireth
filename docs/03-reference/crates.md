# Apeireth Crate Index

This index lists the 16 members of the root Cargo workspace. The independent
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
| `apeireth-orchestration` | `crates/foundation/orchestration` | Multi-agent coordination, council advisor orchestration, consensus protocols, and ambient context state machine (`ambient_context`) |

## Engine

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-runtime` | `crates/engine/runtime` | Canonical session runtime, provider selection, governance, tool dispatch, continuation, and trace |
| `apeireth-provider` | `crates/engine/provider` | Anthropic, MiniMax, and OpenAI-compatible provider capabilities |
| `apeireth-storage` | `crates/engine/storage` | SQLite pool, migrations, storage configuration, and errors |
| `apeireth-memory` | `crates/engine/memory` | Durable memory, retrieval, vector/graph index, Brier intent calibration, thought clustering, meta-thinking chains, and procedural habit memory |
| `apeireth-perception` | `crates/engine/perception` | Multimodal perception backends: Voice (Whisper HTTP, emotion-conditioned acoustic synthesis) and Vision (Xcap screen capture) |
| `apeireth-organ` | `crates/engine/organ` | 9 cognitive organs (W1..W3 world models, E4 curiosity, F1 emotion, F4 hypothesis, F6 values, E7 emergence, memory merger, persona tone synthesizer) |

## Capabilities

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-tools-canonical` | `crates/capabilities/tools` | Built-in filesystem, search, and repository capabilities (enabled by default); fetch and shell capabilities (opt-in, disabled by default); education calculus substitute checker; spill store isolation; egress policy; sole `ProcessExecutor` owner |

## Adapters

| Package | Path | Responsibility |
| --- | --- | --- |
| `apeireth-gateway` | `crates/adapters/gateway` | HTTP transport adapter for canonical runtime, SSE streaming completions, and real-time full-duplex voice barge-in controller |
| `apeireth-cli` | `crates/adapters/cli` | CLI bootstrap and command dispatch through the canonical runtime |
| `apeireth-sdk` | `crates/adapters/sdk` | Public Rust/FFI SDK surface（stub 模式：真 HTTP 未接，显式 `unimplemented!()` 守门） |

## Independent frontend workspace

`frontend/companion-desktop/` contains the Svelte 5 UI and thin Tauri 2 shell.
It is deliberately outside the root Cargo workspace and is checked by
`.github/workflows/companion-desktop-ci.yml`.

## Excluded historical material

`legacy/donor/`, `legacy/archived/`, and `legacy/frozen/` contain historical
implementations and references. They are not product crates and current code
must not depend on them. The former nested `reconstruction_v2/` workspace was
removed from git after its useful ideas were captured in the root workspace;
an untracked local directory may remain on disk and is safe to delete.
`crates/_archived/` holds untracked local build leftovers and is not
repository content.
