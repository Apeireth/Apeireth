# crates/ — Current Apeireth Product Workspace

Physical layout reflects canonical architecture.

## Foundation

Stable contracts and policy vocabulary. Inward-only dependency direction.

| Path | Package | Purpose |
| --- | --- | --- |
| `foundation/core` | `apeireth-core` | Stable identity / primitive vocabulary |
| `foundation/protocol` | `apeireth-protocol` | Normalized provider/tool DTOs and translation |
| `foundation/plugin` | `apeireth-plugin` | Plugin lifecycle, ToolCapability, registries |
| `foundation/governance` | `apeireth-governance` | Allow / Deny / RequireApproval policy contract |
| `foundation/credentials` | `apeireth-credentials` | Credential storage backends |

## Engine

Durable execution machinery.

| Path | Package | Purpose |
| --- | --- | --- |
| `engine/runtime` | `apeireth-runtime` | Microkernel sessions, orchestration, behavior/capability dispatch, events, and abstract ports |
| `engine/runtime-assembly` | `apeireth-runtime-assembly` | Production composition: cognitive behaviors, Organ bridge, tool capabilities, SQLite session adapter |
| `engine/provider` | `apeireth-provider` | Vendor-specific model HTTP/auth/wire adaptation |
| `engine/storage` | `apeireth-storage` | Low-level persistence foundation |
| `engine/memory` | `apeireth-memory` | Durable memory domain |

`apeireth-runtime` has no production dependency on the concrete memory, Organ,
tool, or SQLite implementations. `apeireth-runtime-assembly` depends on those
concrete crates and points inward to the kernel; the kernel never depends back
on the assembly.

## Capabilities

Model/runtime-facing actions.

| Path | Package | Purpose |
| --- | --- | --- |
| `capabilities/tools` | `apeireth-tools-canonical` | Canonical tools: filesystem, search, repo, shell, fetch |

## Future product modules

No product module is active in this baseline. When a feature earns a
canonical owner, create it under `crates/modules/<name>/` and add it to the
root workspace at the same time. Do not commit an empty architecture
directory or reintroduce the historical Companion donor as production code.

## Adapters

External surfaces into canonical Runtime.

| Path | Package | Purpose |
| --- | --- | --- |
| `adapters/gateway` | `apeireth-gateway` | HTTP adapter only |
| `adapters/cli` | `apeireth-cli` | Configuration/bootstrap/I/O only |
| `adapters/sdk` | `apeireth-sdk` | Public SDK surface |

## Legacy

All donor/archived/frozen code lives under `legacy/`, not here. Current product
code must not add new dependencies on `legacy/`. Existing legacy path
dependencies are tracked migration debt.
