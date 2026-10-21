# Apeireth Architecture

This document describes the current product baseline in the root repository.
Historical design proposals and archived implementations live under `docs/archive/`,
`legacy/`, and `reports/`; they are not part of the built product workspace.

> **机器可读依赖图（R0）**：[docs/architecture/ARCHITECTURE.generated.md](docs/architecture/ARCHITECTURE.generated.md)
> ——由 `scripts/arch_doc.py --gen` 从 [contract.toml](docs/architecture/contract.toml) + `cargo metadata` 生成，
> CI 用 `--check` 防漂移；手写依赖表以生成物为准（历史教训：文档声明 32 条 vs 实测 118 条）。

## Current structure

```text
apeireth/
> Workspace taxonomy: **workspace = 30 members（29 product crates + 1 bench-only harness）**
> (foundation 6 / engine 7 + image family 11 / capabilities 1 / adapters 4 = 29 + `benches` harness)。
> crate 口径统一见本文件下表与 [INSTALL.md](INSTALL.md)（2026-10-20 R4b 归档 `apeireth-perception` 后更新；历史口径 31 members 见 [AGENTS.md](AGENTS.md) §3 与 CHANGELOG）。

├── crates/
│   ├── foundation/       # stable domain types, protocol, plugins, policy, credentials
│   ├── engine/            # runtime kernel, assembly, providers, storage, memory
│   ├── capabilities/      # canonical tools and process execution
│   └── adapters/          # gateway, CLI, SDK entry surfaces
├── frontend/companion-desktop/  # independent Svelte + Tauri workspace
├── scripts/               # validation, audit, release, and maintenance tooling
├── deploy/                # deployment-specific artifacts retained separately
├── packaging/             # package build metadata and installers
├── docs/                  # current guides, architecture, reference, and archive
├── legacy/                # archived/reference code; excluded from the root workspace
├── reports/               # durable audit and validation evidence
└── previews/              # design/reference assets
```

The root `Cargo.toml` is the only product Rust workspace. It contains thirty-one
workspace members — thirty product crates plus the bench-only harness
(`benches`, `publish = false`).

**结构/依赖表已退役为生成物（R0，2026-10-20）**：层→crate 清单、118 条生产依赖边、
层方向判定、装配根登记全部由
[docs/architecture/ARCHITECTURE.generated.md](docs/architecture/ARCHITECTURE.generated.md)
从 [contract.toml](docs/architecture/contract.toml) + `cargo metadata` 生成；
CI 用 `scripts/arch_doc.py --check` 拦漂移。本文件只保留人读叙述与所有权边界——
**依赖边以生成物为准，禁止在此手写第二份**（历史教训：本文件曾声明 32 条边，实测 118 条）。

`frontend/companion-desktop/src-tauri` has its own workspace and is deliberately
not a member of the root workspace. `legacy/` is reference material only and is
never imported by current crates. The former nested `reconstruction_v2/`
workspace and the empty `crates/modules/` placeholder were removed during the
pre-freeze cleanup.

## Dependency direction

Dependencies flow inward toward stable contracts and downward through the runtime layers.
The **authoritative** production edge list and layer-direction verdict are generated in
[`ARCHITECTURE.generated.md`](docs/architecture/ARCHITECTURE.generated.md)
(2026-10-20 实测：31 members / 118 production edges)。手改该生成物或在此手写第二份依赖表
均视为契约违规：增删依赖边的唯一方式是改代码 + `Cargo.toml`，然后由 `arch_guard` 与
`arch_doc --gen` 反映。

人工边界规则（生成器表达不了的部分，仍需人守）：

- foundation 包不依赖 adapters；
- runtime 拥有编排与抽象端口；runtime-assembly 拥有具体装配；
- gateway / CLI 只做外部请求翻译，不创建第二个 runtime。

## Ownership boundaries

| Concern | Canonical owner | Boundary rule |
| --- | --- | --- |
| Core IDs, lifecycle, events, philosophy contract | `crates/foundation/core` | stable shared primitives only |
| Wire/request/response DTOs | `crates/foundation/protocol` | public protocol types and vendor wire adapters |
| Plugin and capability contracts | `crates/foundation/plugin` | declarations and lifecycle contracts, not tool implementations |
| Policy decisions | `crates/foundation/governance` | policy evaluation; no transport or process spawning |
| Credential resolution | `crates/foundation/credentials` | backend behind the plugin credential contract |
| Runtime/session/execution loop | `crates/engine/runtime` | mechanism kernel: orchestration, approvals, provider selection, behavior/capability registries, events, and abstract ports |
| Production runtime assembly | `crates/engine/runtime-assembly` | concrete cognitive behaviors, Organ bridge, tool capabilities, composition, and SQLite session adapter |
| Behavior-chain safety guard | `crates/engine/guard` | two-stage behavior-chain safety classifier consumed by the production governance pipeline; owns no tool implementations |
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
must use this boundary; `apeireth-runtime-assembly` registers those tools as
capabilities, and no adapter or frontend may create a competing executor.

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
apeireth gateway serve --bind 127.0.0.1 --port 8080
```

The gateway defaults to loopback (`127.0.0.1`); explicitly requested
non-loopback binding is warned about at startup. It exposes `/health`, live
capability/provider/model projections, runtime diagnostics, canonical chat, and
compatibility panel routes. Provider credentials are resolved by the
provider/credentials path; the desktop frontend remains a separate build and
release boundary.
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
