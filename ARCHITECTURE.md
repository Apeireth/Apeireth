# Apeireth Architecture

Canonical ownership and dependency rules, frozen on branch `reconstruct_v2`.

This document describes the **canonical** architecture: the subsystems that are
authoritative going forward. The workspace also contains a large body of
historical implementation that predates this convergence. Where the two disagree,
this document wins, and the disagreement is recorded in
[`docs/01-architecture/reconstruct-v2-migration-map.md`](docs/01-architecture/reconstruct-v2-migration-map.md).

---

# Canonical Architecture Freeze

The canonical skeleton is frozen on `reconstruct_v2`. The rules in this section
are authoritative. Future features — including functionality migrated from
`master` or `reconstruction_v2/` — must adapt to this skeleton; the skeleton is
not reshaped around an imported feature.

## Freeze rules

1. **Runtime is the only orchestration root.** `Runtime::execute` owns the agent
   loop: session lifecycle, governance evaluation, provider selection and
   invocation, tool dispatch, tool continuation, failure persistence, and trace.
2. **Gateway and CLI are adapters.** They decode/encode transport DTOs and call
   `Runtime::execute`. They must not orchestrate providers, tools, sessions, or
   governance.
3. **Runtime never branches on vendor identity.** No `MiniMax`, `Anthropic`,
   `OpenAI`, `Claude`, `Gemini`, `Bearer`, `x-api-key`, `/chat/completions`, or
   `/messages` logic may exist in `apeireth-runtime::canonical`.
4. **ProviderCapability owns vendor invocation.** Vendor protocol adaptation,
   vendor wire model identity, vendor HTTP transport, and vendor authentication
   header construction live below the provider boundary.
5. **Providers obtain secrets only via CredentialResolver.** Providers resolve
   logical names such as `provider.anthropic.api_key` per turn and receive
   redacted `Secret` values. Long-lived `api_key: String` fields and hardcoded
   credentials are forbidden in canonical production code.
6. **Protocol contains canonical contracts, not vendor transport ownership.**
   `apeireth-protocol` translates DTOs; it does not construct an HTTP client, own
   credentials, or run retry/routing.
7. **Provider never executes tools.** Providers may emit canonical `ToolCall`
   requests and receive canonical `ToolResult` messages, but never resolve or
   invoke a `ToolCapability`.
8. **Runtime owns tool dispatch and continuation.** Tools are registered
   capabilities; the runtime resolves model-facing names through the capability
   registry, asks governance, dispatches the capability, and feeds the result
   back to the provider.
9. **PluginRegistry / CapabilityRegistry are the authoritative capability
   ownership system.** Capability declarations live only in plugin manifests;
   the capability registry is an index over those declarations. No second
   registry or manager may store ownership facts.
10. **Governance returns Allow / Deny / RequireApproval.** These three semantics
    are stable and distinct. Runtime must not recreate policy; providers must not
    make tool-policy decisions; gateway must not contain governance business
    rules.
11. **Session lifecycle belongs to Runtime.** One canonical session ownership
    path. Transport-local metadata in gateway/CLI must not become a competing
    session store.
12. **Structured trace never exposes raw chain-of-thought.** Public canonical
    contracts carry no `reasoning_cot`, `raw_chain_of_thought`, or equivalent.
    Diagnostics use `ExecutionTrace` / `SessionEvent` with provider, governance,
    capability, and round facts.

## Freeze means

After this commit, future features adapt to the skeleton. When functionality is
migrated from `master`, the migration direction is:

```text
master feature -> canonical ownership
```

not:

```text
canonical architecture -> reshape itself to match master
```

---

## 1. The three sentences

**Protocol translates. Plugins provide capability. Runtime orchestrates.**

Everything below is a consequence of those three, plus one more: *an LLM is one
of Apeireth's capability providers, not Apeireth itself.* No part of the runtime
names a vendor.

---

## 2. Canonical dependency graph

```text
                    apeireth-core
                    (primitives)
                          ^
              ------------+------------
             |            |            |
     apeireth-protocol    |    apeireth-governance
      (translation)       |         (policy)
             ^            |            ^
             |            |            |
        apeireth-plugin --+            |
        (capability)                   |
             ^                         |
             |                         |
             +---- apeireth-runtime ---+
                   (orchestration)
                          ^
              ------------+------------
             |            |            |
            cli        gateway      desktop
```

Arrows point from dependency to dependent: `core` is depended on, `runtime`
depends. The graph is acyclic and must stay so.

---

## 3. Subsystem contracts

### `apeireth-core` — primitives

**Owns.** `SessionId`, `TaskId`, `TraceId`, `RequestId`, `PluginId`,
`CapabilityId`, `ModelId`, `Timestamp`, `Clock`, `Lifecycle`, `Event`, `Topic`,
`Metadata`, `CoreError`. Canonical surface: `apeireth_core::kernel`.

**May depend on.** Nothing in this workspace. Serialization, time, and UUID
libraries only.

**Must not depend on.** Anything. Core is the leaf.

**Must not contain.** HTTP, LLM implementations, SQLite, tools, memory engines,
gateways, MCP transport, companion cognition, provider implementations.

The membership test: *would two unrelated subsystems both need this in order to
talk to each other?* If only one needs it, it belongs to that one.

> **Status.** The crate also carries pre-convergence content — memory items,
> philosophy verdict caches, permission onions, a cognitive lifecycle — re-exported
> at the crate root and depended on by 38 crates. That content violates the rule
> above. It is confined to the crate root while the canonical vocabulary lives
> under `kernel`, and its eviction is migration item #2.

### `apeireth-protocol` — translation

**Owns.** The canonical interaction contract: `NormalizedRequest`,
`NormalizedResponse`, `NormalizedMessage`, `MessageRole`, `ContentPart`,
`ToolCall`, `ToolResult`, `NormalizedUsage`, `NormalizedFinishReason`,
`StreamEvent`, `ModelDescriptor`. Adapters for OpenAI Chat, OpenAI Responses,
Anthropic Messages, and Gemini. Canonical surface:
`apeireth_protocol::canonical`.

**May depend on.** `apeireth-core`.

**Must not depend on.** `apeireth-plugin`, `apeireth-runtime`, storage, gateway.

**Must not own.** Credentials, retry policy, routing, fallback, provider health,
connection pooling, quota, model selection, or an HTTP client's lifetime.

`ProtocolAdapter` is `adapt_request` / `adapt_response` over `serde_json::Value`.
It performs no I/O and holds no state. The signature to never write is:

```rust
// WRONG: hands the adapter the credential, the HTTP client, the retry
// decision and the connection lifetime, all at once.
async fn execute(&self, api_key: &str, req: &Request) -> Response;
```

### `apeireth-plugin` — capability

**Owns.** `Plugin`, `PluginManifest`, `PluginContext`, `CapabilityDescriptor`,
`CapabilityKind`, `ToolCapability`, `ProviderCapability`, `PluginRegistry`,
`CapabilityRegistry`, `PluginManager`, `CredentialResolver`, `Secret`.

**May depend on.** `apeireth-core`, `apeireth-protocol`.

**Must not depend on.** `apeireth-runtime`, gateway, storage. A capability that
knew about the runtime would invert the arrangement.

**Scope.** Static, in-process plugins. No dynamic library loading, WASM, hot
reload, remote plugins, or marketplace.

### `apeireth-governance` — policy

**Owns.** `GovernanceHook`, `Decision`, `Action`, `GovernanceRequest`,
`GovernanceVerdict`, `GovernancePipeline`.

**May depend on.** `apeireth-core`.

**Must not depend on.** Anything else. A policy that knows what it is guarding
cannot be reused to guard anything else.

**Is not.** A policy library in the orchestration sense. The canonical contract
stays thin; concrete detectors, audit-chain primitives, and permission primitives
become `GovernanceHook` implementations. M1C added `input_security`, `audit`, and
`permission` modules inside this crate as deterministic primitives, without adding a
second pipeline or decision enum.

### `apeireth-runtime::canonical` — orchestration

**Owns.** `Runtime` (the single application-level composition root),
`RuntimeBuilder`, `SessionManager`, `Session`, `SessionStore`, `ProviderRouter`,
`ExecutionTrace`, and `Runtime::execute` — the agent loop.

**May depend on.** `apeireth-core`, `apeireth-protocol`, `apeireth-plugin`,
`apeireth-governance`.

**Must not depend on.** Gateway, CLI, desktop, or any concrete provider adapter.

> **Status.** These modules obey the rule. The *crate* around them does not: it
> still carries the historical seven-module orchestration driver and its ten
> internal dependencies. The graph stays acyclic and the canonical code touches
> none of that, but the crate boundary is not yet clean. Migration item #1.

### `apeireth-storage` — durability foundation

**Owns.** Low-level durable persistence infrastructure: SQLite connection pool and configuration, versioned schema migrations, storage-level errors, and low-level persistence helpers. M1A created the foundation only.

**May depend on.** `apeireth-core`.

**Must not depend on.** Gateway, runtime, plugin, provider, companion, governance.

M1A scope: `SqliteConnectionPool`, `SqliteConfig`, versioned migrations (`PRAGMA user_version`), and `StorageError`. Vector, graph, and session persistence are later migration items; `InMemorySessionStore` remains the
runtime session seam until the storage backend is connected.

### `apeireth-memory` — durable memory domain

**Owns.** The canonical memory entity, the memory repository contract, memory-level errors, the SQLite-backed repository implementation, and deterministic retrieval semantics. M1B1 created the
canonical surface under `apeireth_memory::canonical`: `MemoryId`, `MemoryItem`,
`MemoryRepository`, `MemoryFilter`, `MemoryError`, and `SqliteMemoryRepository`.
M1B2 added `RetrievalOptions`, `MemoryHit`, `act_r_activation`, and `retrieve`.
M1B3 added in-memory `VectorIndex` (cosine top-k) and `MemoryGraph` primitives (nodes, weighted edges, neighbours, cycle-safe bounded traversal, shortest path).

**May depend on.** `apeireth-core`, `apeireth-storage`, `serde`, `thiserror`, `async-trait`.

**Must not depend on.** Runtime, gateway, CLI, provider, companion, plugin, governance.

**Is not.** A vector database, a knowledge-graph product, an embedding provider, or a product memory feature. Those are later migration items layered on top of this crate, not collapsed into it.

### `apeireth-companion` — cognition

**Owns.** Emotion, dreaming, presence, world model, epistemic self-repair.

**May depend on.** `apeireth-core`, `apeireth-protocol`.

**Must not depend on.** Gateway. Companion cognition consumes a session; it is
not part of one.

### `apps/` — entry points *(cli, gateway, desktop)* — **target layout, does not exist yet**

Today these live as `crates/apeireth-cli`, `crates/apeireth-gateway`,
`crates/apeireth-tui` and `crates/apeireth-web`. The rules below apply to them
where they are; the move under `apps/` is a migration item, not a precondition.

**May depend on.** `apeireth-runtime`, `apeireth-protocol`.

**Must not implement.** Session management, provider routing, tool dispatch,
memory orchestration, or an agent loop. A gateway that manages its own sessions
is a second runtime wearing an HTTP hat, and the two drift.

Everything goes through `runtime.execute(request)`.

---

## 4. Forbidden edges

```text
core       -> anything
protocol   -> plugin | runtime | storage | gateway
plugin     -> runtime | gateway
governance -> anything except core
storage    -> gateway | runtime
runtime    -> gateway | cli | desktop | concrete provider adapter
```

---

## 5. Concept ownership

| Concept | Owner |
| --- | --- |
| Core primitive (ids, time, lifecycle, event) | `apeireth-core::kernel` |
| Protocol DTO (request, response, message, tool call/result, stream event) | `apeireth-protocol::canonical` |
| Protocol adapter (vendor wire format) | `apeireth-protocol::adapters` |
| Plugin, manifest, lifecycle | `apeireth-plugin` |
| Capability (id, kind, declaration, registry) | `apeireth-plugin` |
| Tool implementation | a plugin, via `ToolCapability` |
| Process execution / containment | `apeireth-tools-canonical::process` *(canonical `ProcessRequest` / `IsolationRequirement` / `IsolationCapabilities`, platform backends)* |
| Provider implementation (credentials, HTTP, transport) | a plugin, via `ProviderCapability` |
| Provider routing, fallback, health | `apeireth-runtime::canonical::provider` |
| Session | `apeireth-runtime::canonical::session` |
| Execution loop | `apeireth-runtime::canonical::execute` |
| Execution trace | `apeireth-runtime::canonical::trace` |
| Composition root | `apeireth-runtime::canonical::runtime` |
| Policy decision | `apeireth-governance` |
| Durable storage | `apeireth-storage` *(SQLite pool, writer, migrations)* |
| Durable memory domain | `apeireth-memory::canonical` *(M1B1 entity + repository + SQLite; M1B2 retrieval/ACT-R; M1B3 vector/graph primitives)* |
| HTTP transport | `apps/gateway` |
| Companion cognition | `apeireth-companion` |
| Credential storage backend | `apeireth-credentials`, behind `plugin::CredentialResolver` |

---

## 6. Standing rules

### 6.1 One source of truth for capability

`PluginRegistry` owns plugins. `CapabilityRegistry` owns the capability-id →
owner index, and is an **index, not a copy**: a capability's declaration lives
only in its owner's manifest.

Do not add `ToolRegistry`, `McpRegistry`, `ExtensionRegistry`, `SkillRegistry`,
`OrganRegistry`, or `ProviderRegistry` as *storage*. Typed views over the
canonical registries are welcome — `PluginManager::active_tools`,
`active_providers`, `tool_declarations` are exactly that. A view derives; a
second registry diverges.

### 6.2 MCP is a transport, not an ecosystem

```text
MCP client/server -> transport adapter -> capability model -> runtime
```

An MCP server's tools become `ToolCapability` instances provided by a plugin
declaring `transport.mcp`. There must be no `McpRegistry`, `McpRuntime`,
`McpAgent`, or `McpPermissionSystem` — those would be a second capability
ecosystem with its own registry, lifecycle and policy.

### 6.3 Credentials are injected, never located

Secrets are resolved at start-up through `CredentialResolver`, by *logical name*
(`provider.anthropic.api_key`), never from a fixed path. Forbidden: developer
home directories, `apikey.txt` at an absolute path, any machine-specific
location, and storing a secret as a plain `String` field on a long-lived struct.

`Secret`'s `Debug` and `Display` are redacted; reading a value requires
`expose()`, which is greppable.

### 6.4 Raw chain-of-thought is not a public contract

No canonical public type carries `reasoning_cot`, `raw_chain_of_thought`, or an
equivalent. Raw reasoning is unavailable from some providers, unstructured, and
describes the model rather than the runtime.

Diagnostics use `ExecutionTrace`, which records which provider served each round,
what governance decided, which capability ran, and how the turn ended. A test in
`apeireth-runtime` greps the serialized trace for `reasoning`,
`chain_of_thought`, `cot` and `thinking`, so a field cannot be added quietly.

### 6.5 Determinism

Nothing canonical reads the wall clock implicitly. `Clock` is injected
everywhere, including into provider latency measurement, which is what makes the
end-to-end test reproducible with no sleeps and no network.

### 6.6 One runtime, one loop

`Runtime::execute` is the only agent loop. If a second one appears anywhere, the
two will diverge at the first behaviour change.

### 6.7 Shell capability profiles

A future `tool.shell` capability may only ship under these readiness rules:

- **Trusted Shell** is the only v1 candidate: per-invocation `RequireApproval`,
  explicit cwd, `Clear`/`Explicit` environment, bounded timeout/output,
  `ProcessExecutor` guardrails, no filesystem or network isolation claim, and
  disabled for autonomous/background operation. Its
  `IsolationRequirement` must be satisfied on Windows, Linux, and macOS and
  must accept `ProcessTreeContainment: Partial` on Unix.
- **Restricted Shell** requires at least `PrivilegeReduction: Partial`,
  stronger process-tree containment, and resource limits; it is platform
  conditional and is not a sandbox.
- **Untrusted Shell** requires `PrivilegeReduction: Enforced`,
  `FilesystemIsolation: Enforced`, and `NetworkIsolation: Enforced`; it is
  unsatisfiable on all current backends and must fail closed.
- Approval is not containment; containment is not approval. Shell
  implementation is blocked until the canonical approval-resume lifecycle
  exists.

See `docs/01-architecture/m2c-shell-readiness-review.md` for the full review.


---

## 7. The minimal agent loop

```text
   request -> transcript + active tool declarations
        -> governance (completion)
        -> provider router -> provider
        -> tool calls?
             no  -> final response
             yes -> governance (dispatch)
                 -> capability lookup -> plugin dispatch
                 -> tool result -> transcript -> provider again
```

Proved end to end in
[`crates/apeireth-runtime/tests/canonical_agent_loop.rs`](crates/apeireth-runtime/tests/canonical_agent_loop.rs).
The production HTTP wiring is proved separately through the real gateway router
in
[`crates/apeireth-gateway/tests/canonical_entry_e2e.rs`](crates/apeireth-gateway/tests/canonical_entry_e2e.rs).

Behavioural rules the loop encodes:

- A tool problem — unknown name, governance refusal, tool failure — produces a
  `ToolResult` the model sees, and the turn continues. The model is the party
  that can recover.
- Governance denying the *completion*, requiring human approval, or the round
  limit being hit does end the turn. None of those is recoverable by retrying.
- A failed turn still persists the user's message.

---

## 8. What this phase deliberately did not do

Voice, screen agent, software factory, MCP feature work, dream evolution, emotion
engines, memory engines, graph engines, vector databases, desktop redesign, TUI
work, new providers, distributed agents, plugin marketplaces, and dynamic WASM
plugins. Existing implementations of all of these remain, are audited, and are
scheduled — not extended.

Structure first, features later.

---

## 9. Current migration status

### DONE — canonical product entry ownership

The canonical runtime is now the production execution owner for the migrated
chat paths.

```text
apeireth CLI `chat`
    -> canonical TurnRequest
    -> Runtime::execute

apeireth CLI `gateway serve`
    -> apeireth-gateway HTTP router
    -> canonical TurnRequest
    -> Runtime::execute
```

The gateway exposes `/v1/chat` and `/v1/chat/completions` as transport adapters.
They decode HTTP, construct canonical requests, invoke the runtime directly, and
encode canonical outcomes. They do not route providers, dispatch tools, manage
plugin lifecycle, or run an agent loop.

Before this cutover, `gateway serve` assembled `apeireth-api::AppState` and sent
requests through the historical API pipeline and LLM router. The CLI's
historical `session` command also executed its own pre-canonical action flow.
The migrated `chat` and `gateway serve` paths no longer use those execution
paths. Unmigrated administrative and historical commands remain in place.

The real-entry deterministic E2E exercises an Axum request through the
production gateway router, then proves:

```text
user request
    -> fake provider (tool call)
    -> calculator capability
    -> tool result in the same session and trace
    -> fake provider (final answer)
    -> HTTP response
```

It requires no network provider, API key, wall-clock sleep, or developer-machine
filesystem.

### DONE — reconstructable failures

The session persists the user message and a structured `TurnStarted` event before
governance or provider execution. Governance denial, approval requirement,
provider failure, tool failure, structural execution failure, and successful
completion advance the session revision with structured events. Failed
execution therefore remains observable without inserting audit prose into the
provider transcript.

### DONE — production provider capability migration (Phase 1 + 2 + 3)

Three ordinary HTTP provider families now reach the runtime as first-class
canonical `ProviderCapability` implementations, with the legacy compatibility
bridge **removed**:

```text
Runtime
  -> ProviderRouter
  -> MinimaxProviderCapability           (apeireth-provider, OpenAI Chat Completions)
  -> AnthropicProviderCapability          (apeireth-provider, Anthropic Messages API)
  -> OpenAiCompatibleProviderCapability   (apeireth-provider, generic OpenAI Chat Completions)
  -> CredentialResolver                   (EnvCredentialResolver, logical name → env var)
  -> vendor HTTP                         (reqwest)
```

The runtime names no vendor. `apeireth-provider::canonical_minimax`,
`canonical_anthropic`, and `canonical_openai_compatible` are the only places
that know each vendor's protocol shape. The OpenAI Chat Completions protocol
primitives shared by minimax and the generic provider live in a small
provider-internal helper, `apeireth-provider::openai_chat` — not a runtime
abstraction. Each capability owns its `reqwest::Client` and the
canonical↔vendor translation; the router owns cross-provider fallback;
credentials arrive per-turn through `CredentialResolver`, never as a stored
`String`. Legacy internal retry loops were dropped so retry has one owner per
layer (the router).

Phase 2 was the protocol-diversity proof: the Anthropic provider's wire shape
differs from OpenAI's (different endpoint `/v1/messages`, `x-api-key` +
`anthropic-version` headers instead of Bearer, top-level `system` field instead
of a messages entry, `content[].type=="text"` response, `stop_reason`/`input_tokens`/
`output_tokens`), yet all of those differences are contained **below** the
Runtime boundary. The runtime, router, gateway, session, governance, and tool
loop are unchanged; no vendor branch was added.

Phase 3 converged the remaining ordinary HTTP provider (generic
openai-compatible) and retired the `LegacyLlmCapability` bridge. The generic
provider's identity is a protocol family — `provider.openai-compatible`, not
`provider.openai` (which would misleadingly imply vendor == OpenAI); base_url,
model list, and credential key are configuration, and the protocol family is
distinct from the vendor (§8/§9). With all three providers canonical and zero
production consumers of the bridge, the migration scaffolding was deleted.

Hardening (Phase 2 Goal A) separated canonical model identity from the vendor
wire model name: a provider-local `ProviderModel` pairs a canonical
`ModelDescriptor` (routing identity) with an explicit `wire_name` (HTTP body
identity), so a request for `minimax-m3` sends `MiniMax-M3` to the vendor, and
`Qwen/Qwen3-32B` becomes canonical id `qwen-qwen3-32b` (the forbidden `/` is
folded to `-`) while its wire name stays verbatim. `ModelDescriptor::display_name`
stays presentational. Advertised `ModelFeature`s are truthful — no provider
claims `Streaming`/`ToolCalls`/`Vision` it does not implement (fail-closed on
unsupported content).

Production bootstrap (`apeireth-cli::build_canonical_runtime_from_env`) is
config-driven: it registers every canonical provider whose configuration is
present (minimax + anthropic always; openai-compatible only when models are
configured, since the generic provider has no hardcoded model default), with a
deterministic fallback order, and routes purely by `supports_model` + health —
no vendor heuristics. Ambiguous model ownership (two providers claiming the same
model) resolves deterministically by the explicit fallback order, never by
insertion order; two plugins declaring the same capability id are rejected at
registration.

Provider migration status:

```text
Canonical Entry Cutover             DONE
CredentialResolver Production       DONE  (EnvCredentialResolver; apeireth-credentials store = P1)
  EnvCredentialResolver             DONE
  keyring/file backend              PENDING
Provider Migration:
  minimax                           DONE  (canonical, real-entry tested, model-id/wire split)
  anthropic                         DONE  (canonical, real-entry tested, protocol-diversity proof)
  openai-compatible                 DONE  (canonical, real-entry tested, protocol-family identity)
  (descriptors claude_code/codex/copilot/gemini_cli/opencode + http_dispatch)  NOT IN CANONICAL PATH
LegacyLlmCapability                 REMOVED (zero production consumers; deleted)
```

Migration matrix:

| Provider | Canonical capability? | Canonical credentials? | Protocol family | Legacy bridge? | Deterministic transport? | Runtime E2E? | Gateway E2E? | Remaining blocker |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| minimax | yes | yes (EnvCredentialResolver) | OpenAI Chat Completions | n/a (bridge removed) | yes | yes | yes | none |
| anthropic | yes | yes (EnvCredentialResolver) | Anthropic Messages API | n/a (bridge removed) | yes | yes | yes | none |
| openai-compatible | yes | yes (EnvCredentialResolver) | OpenAI Chat Completions (generic) | n/a (bridge removed) | yes | yes | yes | none |
| (descriptors + http_dispatch) | no | n/a | mixed | n/a | descriptor tests | no | no | not in canonical path; separate task |
| LegacyLlmCapability | n/a | n/a | n/a | REMOVED | n/a | n/a | n/a | deleted — zero production consumers |

### DONE — M1C canonical governance donor primitives

`apeireth-governance` now hosts deterministic security primitives without adding
a second pipeline, second decision enum, or runtime special-case policy:

```text
input_security: PiiDetector (email/phone/credential-like, structured findings,
redaction), PromptInjectionHeuristic (structured signals),
PromptInjectionHook and CredentialDisclosureHook -> RequireApproval for
capability arguments.

audit: AuditHashChain with explicit Timestamp injection, SHA-256, stable
canonical serialization, genesis zero-hash, and tamper-evident verification.
This is an integrity primitive; ExecutionTrace remains the runtime trace owner.

permission: PermissionSet, PermissionPolicy, and PermissionGovernanceHook map
capability dispatch to Allow / Deny / RequireApproval.
```

Detection is separated from decision; the runtime still only sees canonical
governance decisions.

### DONE — M2A canonical simple tool capability ports

`apeireth-tools-canonical` now provides three low-risk builtin tools through
the canonical plugin path:

```text
tool.filesystem  read/list/stat inside an explicit workspace root
tool.search      deterministic local substring search
tool.repo        read-only git inspection (status/diff/log/branch/summary)
```

The tools own their identity, schema, execution, and result. Runtime owns
dispatch and governance. No second tool registry, no runtime tool branching,
and no shell/network/browser tool was introduced.

M2B-X completed the cross-platform process isolation foundation. The public
contract in `apeireth-tools-canonical::process` now exposes
`IsolationCapabilities`, `IsolationRequirement`, `IsolationProfile`,
`ProcessLimits`, `ProcessRequest`, `ProcessResult`, and
`PlatformEnforcement`. Platform backends report their real enforcement level
per capability, callers declare minimum requirements on `ProcessRequest`, and
the executor fails closed before spawn when a requirement cannot be met.

| Capability | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Structured spawn / explicit cwd | ENFORCED | ENFORCED | ENFORCED |
| Timeout / stdout / stderr | ENFORCED | ENFORCED | ENFORCED |
| Environment isolation | ENFORCED | ENFORCED | ENFORCED |
| Process-tree containment | ENFORCED (Job Object) | PARTIAL (process group) | PARTIAL (process group) |
| Memory limit | ENFORCED (Job Object, opt-in) | PARTIAL (`RLIMIT_AS`) | PARTIAL (`RLIMIT_AS`) |
| Process-count limit | ENFORCED (Job Object, opt-in) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) |
| CPU / file-size limit | UNSUPPORTED | ENFORCED (`setrlimit`) | ENFORCED (`setrlimit`) |
| Privilege reduction | ENFORCED when restricted-token launch is available, otherwise UNSUPPORTED | PARTIAL (`PR_SET_NO_NEW_PRIVS`) | UNSUPPORTED |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Network isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Fail-closed pre-exec containment | ENFORCED (suspended spawn) | ENFORCED (pre_exec setup) | ENFORCED (pre_exec setup) |

See
[`docs/01-architecture/m2b-x-cross-platform-isolation.md`](docs/01-architecture/m2b-x-cross-platform-isolation.md).

M2D added the canonical controlled-egress boundary in
`apeireth-tools-canonical::egress`: `EgressPolicy`, `EgressAllowList`,
`ControlledEgress`, and structured `EgressError`. It validates scheme,
host, port, resolved addresses, pins DNS through reqwest, disables
ambient proxies, and revalidates every redirect hop. Process-level
`NetworkIsolation` remains `UNSUPPORTED` on all platforms.
See [`docs/01-architecture/m2d-network-egress-enforcement.md`](docs/01-architecture/m2d-network-egress-enforcement.md).

### PENDING

- Integrate `apeireth-credentials` backends (file store / keyring) behind
  `plugin::CredentialResolver`; the production resolver is currently
  `EnvCredentialResolver`. A `CredentialsStore`-backed resolver is a drop-in
  once wired — the contract a provider sees is identical.
- Retire the historical runtime modules after their remaining consumers migrate.
- Consolidate legacy registries only when each remaining caller has moved to the
  canonical `PluginRegistry` and `CapabilityRegistry` ownership path.
- Finish protocol crate cleanup without moving vendor DTOs into runtime.
- Adapt dynamic MCP discovery into canonical capabilities; do not create a
  parallel MCP runtime or registry.
- Remove the nested `reconstruction_v2/` workspace only after provider migration
  and product stabilization.

### DEPRECATED

The nested `reconstruction_v2/` workspace remains a historical architecture
donor. It is not a production execution path and is intentionally not deleted
during this cutover.
