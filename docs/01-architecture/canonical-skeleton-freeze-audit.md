# Canonical Skeleton Freeze — Reality Audit

Status: recorded before freeze corrections.
Branch: `reconstruct_v2`
Starting HEAD: `b34418834fce69fd2332c09580a1e214d18e3a01`

This audit records the current ownership facts for the canonical skeleton.
It is the baseline against which the freeze corrections are applied and the
invariant tests are judged.

## Method

Read the current canonical crates (`apeireth-core`, `apeireth-protocol`,
`apeireth-plugin`, `apeireth-governance`, `apeireth-provider`,
`apeireth-runtime::canonical`, `apeireth-gateway::canonical_entry`,
`apeireth-cli` canonical bootstrap), the production provider capabilities, and
the existing integration tests. The audit covers all components required by the
freeze brief.

## Provider migration precondition

| Provider | Direct `ProviderCapability`? | Credentials via `CredentialResolver`? | Production path? | Legacy bridge? |
| --- | --- | --- | --- | --- |
| MiniMax | yes (`apeireth-provider::canonical_minimax`) | yes, `provider.minimax.api_key` | yes, CLI + gateway bootstrap | no `LegacyLlmCapability` remains |
| Anthropic | yes (`apeireth-provider::canonical_anthropic`) | yes, `provider.anthropic.api_key` | yes, CLI + gateway bootstrap | no `LegacyLlmCapability` remains |
| OpenAI-compatible | yes (`apeireth-provider::canonical_openai_compatible`) | yes, `provider.openai-compatible.api_key` | yes when configured | no `LegacyLlmCapability` remains |

Search result: `LegacyLlmCapability` appears nowhere in `crates/**/*.rs`.
Production path does not use the legacy bridge. Phase 3 is complete.

## Component audit

| Component | Current owner | Should own | Must not own | Current violations | Required action |
| --- | --- | --- | --- | --- | --- |
| Core | `apeireth-core::kernel` for canonical primitives; crate root still carries legacy memory/onion/philosophy content | stable ids, generic metadata, lifecycle primitives, clock/time abstraction, generic domain/kernel errors, generic events | HTTP, LLM vendor, provider routing, tool execution, Axum, CLI, credentials backend | canonical `kernel` is clean; crate root still re-exports legacy content (already documented as migration item) | none in this phase; freeze documents the boundary |
| Protocol | `apeireth-protocol::canonical` for canonical DTOs; `apeireth-protocol::adapters` for vendor wire translation | canonical request/response DTOs, message/content representation, tool-call/tool-result representation, usage, finish reason, stream event contract | reqwest client, API key, Bearer token, x-api-key, vendor endpoint, provider health, provider routing, retry loop, Runtime | `reqwest` and `tokio` are declared as normal dependencies but are used only by examples/tests; this makes the dependency graph claim the protocol owns an HTTP client | move `reqwest` and `tokio` to `[dev-dependencies]` |
| Plugin | `apeireth-plugin` | `Plugin` trait, plugin lifecycle, `PluginManifest`, capability declarations, capability implementation registration, capability ownership, canonical capability registry | runtime, gateway, storage | none found; `PluginManager::register` validates manifest vs implementation and rejects duplicate capability ids and duplicate model-facing tool names | none in this phase |
| Capability Registry | `apeireth-plugin::registry::CapabilityRegistry` + `PluginRegistry` | authoritative id→owner index; plugin registry owns plugin state | second copies of capability declarations, `ProviderRegistry2`/`ToolRegistry2`/`RuntimeRegistry`/`VendorRegistry`/`PluginManagerV2`/`CapabilityManager` | none found; duplicate ownership fails closed; iteration is id-ordered | none in this phase |
| Governance | `apeireth-governance` | `Allow` / `Deny` / `RequireApproval` decision semantics, `GovernanceHook`, `GovernancePipeline` | runtime, provider, gateway, concrete policy libraries | none found; decisions are serialized with stable tags and preserved distinctly | none in this phase |
| Provider | `apeireth-provider::canonical_*` implementations | vendor protocol adaptation, vendor wire model identity, vendor HTTP transport, vendor authentication header construction, provider-local configuration, `ProviderCapability` implementation | session, runtime loop, tool execution, governance, gateway routing, global retry orchestration | none found in the three canonical providers; they resolve keys per turn via `CredentialResolver` and own their `reqwest::Client` | none in this phase |
| Tool | `apeireth-plugin::ToolCapability` (canonical contract); runtime owns dispatch | tool identity, schema, execution interface, tool result, risk/capability metadata | provider must never execute tools directly; tool must not own session/loop | none found in canonical path; model-facing name collisions are rejected at registration | none in this phase |
| Runtime | `apeireth-runtime::canonical` | session lifecycle, execution rounds, governance evaluation, provider selection, provider invocation, tool dispatch, tool continuation, failure persistence, execution trace | vendor implementation, tool implementation, HTTP transport, credential store, UI | canonical modules are clean; the surrounding crate still carries the historical seven-module driver and its deps (documented migration item #1) | add source-level invariant guard for vendor-free canonical modules; keep crate-boundary cleanup as a migration item |
| Gateway | `apeireth-gateway::canonical_entry` for production chat HTTP | transport adapter: HTTP → parse → canonical Request → `Runtime::execute` → canonical Response → HTTP response | directly call provider, directly execute tools, directly mutate session repository, own memory orchestration, implement governance, contain vendor branches | `canonical_entry` is clean; the surrounding crate still carries historical gateway/semantic router code outside the canonical path | none in this phase; freeze documents canonical entry as the production chat path |
| CLI | `apeireth-cli::build_canonical_runtime_from_env` + `execute_canonical_cli_turn` | configuration, bootstrap, input/output adapter | agent loop, provider retry, tool loop, session business logic, governance | canonical chat path is clean; historical CLI commands remain but are not the canonical chat path | none in this phase; freeze documents canonical chat path |
| Credentials | `apeireth-plugin::CredentialResolver`/`Secret`; production resolver `apeireth-provider::credentials::EnvCredentialResolver` | resolution contract, secret redaction, secret boundary | long-lived `api_key: String` on runtime/gateway/provider; hardcoded credentials; fixed paths | none found in canonical production path; `apeireth-credentials` store backends are still a pending drop-in behind `CredentialResolver` (documented P1) | none in this phase beyond freeze documentation |
| Session | `apeireth-runtime::canonical::session` | session lifecycle, transcript, structured session events, persistence seam | gateway session managers, provider session state, CLI session stores | none found in canonical path; `SessionManager` is the only lifecycle owner; `InMemorySessionStore` is the seam | none in this phase |
| Trace | `apeireth-runtime::canonical::trace` | structured execution trace: provider invoked/succeeded/failed, governance decision, tool invoked/succeeded/failed, round completed | raw private chain-of-thought, `reasoning_cot`, `raw_chain_of_thought`, `internal_reasoning` | none found; trace type has no reasoning fields and an existing test guards serialized form | none in this phase |

## Dependency direction

Canonical edges observed:

```text
core
  <- protocol
  <- plugin
  <- governance (core only)
  <- provider (core/protocol/plugin)
  <- runtime::canonical (core/protocol/plugin/governance)
  <- gateway / cli (runtime + protocol/core)
```

No forbidden canonical edge (`core -> runtime`, `protocol -> runtime`,
`plugin -> runtime`, `governance -> runtime`, `provider -> gateway/cli`) exists.
The one correction required is the protocol crate's unused normal dependencies.

## Violation classification

| # | Finding | Class | Disposition |
| --- | --- | --- | --- |
| 1 | `apeireth-protocol` declares `reqwest` as a normal dependency although no library code constructs a client | P1 | fix now: move to `[dev-dependencies]` |
| 2 | `apeireth-protocol` declares `tokio` as a normal dependency although only tests use it | P1 | fix now: move to `[dev-dependencies]` |
| 3 | `apeireth-runtime` crate root still contains legacy vendor-naming code (`LlmWorker`, `api_key: String`) outside `canonical` | P1 | do not rewrite in this phase; already documented as migration item #1; add an invariant test that canonical modules stay vendor-free |
| 4 | `apeireth-core` crate root still carries legacy content outside `kernel` | P1 | do not rewrite in this phase; already documented as migration item #2; freeze documents boundary |

No P0 violations were found in the canonical production path.

## E2E baseline

Before any freeze correction, the existing deterministic E2E tests pass:

- `cargo test -p apeireth-runtime --test canonical_agent_loop` — 17 passed
- `cargo test -p apeireth-gateway --test canonical_entry_e2e` — 4 passed

The required success loop and the Deny / RequireApproval / ProviderFailure /
ToolFailure cases are already covered by these two test targets.
