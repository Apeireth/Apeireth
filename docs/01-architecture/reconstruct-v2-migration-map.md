# Legacy Migration Map

Companion to [`ARCHITECTURE.md`](../../ARCHITECTURE.md) and
[`reconstruct-v2-audit.md`](reconstruct-v2-audit.md).

This is the historical migration map for the canonical skeleton. The root
workspace is now the only product workspace; the nested `reconstruction_v2/`
prototype described below was removed during the pre-freeze cleanup after its
surviving ideas had been captured. Historical decisions remain here for
traceability.

Decision vocabulary:

| | |
| --- | --- |
| `KEEP` | already canonical; do not rewrite |
| `PORT` | move the implementation to its canonical owner, largely as-is |
| `MERGE` | fold into an existing canonical component |
| `REDESIGN` | the responsibility survives, the shape does not |
| `DEPRECATE` | remove once callers are gone |

---

## 0. Priority order

The first three items unblock everything else. Items 1 and 2 are the two places
where a canonical crate currently shares a home with legacy content, which is
the only reason the canonical dependency rules cannot yet be enforced
mechanically.

| # | What | Why first |
| --- | --- | --- |
| 1 | Evict the legacy orchestrator from `apeireth-runtime` | until then the canonical runtime crate transitively depends on ten legacy crates |
| 2 | Drain `apeireth-core` down to `kernel` | until then core is not a primitives crate, and 38 crates depend on the difference |
| 3 | Wrap `apeireth-provider`'s five providers as `ProviderCapability` plugins | until then the canonical runtime has no real provider, only test doubles |

---

## 1. Crates that squat on a canonical name

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-runtime` (lib.rs, `g5_runtime_bridge`, `workflow_worker`) | seven-module orchestration driver: heartbeat, task store, bus, arbitration, search, group chat, emotion | new `apeireth-orchestration`, or absorbed into `apeireth-companion` | **REDESIGN** | move the driver out of the crate, leaving `canonical/` as the whole crate; only `apeireth-gateway` and `apeireth-tui` depend on it, so the blast radius is two Cargo.toml edits plus their use sites | decide whether the seven-module loop survives at all, or becomes companion cognition driven by `Runtime::execute` |
| `apeireth-core` (`memory`, `onion`, `philosophy`, `gate`, `lifecycle`, `eight_anchors`) | memory items, permission onions, philosophy verdict cache, five-gate guard, nine-phase cognitive lifecycle | `apeireth-storage` (memory), `apeireth-governance` hooks (onion/philosophy/gate), `apeireth-companion` (cognitive lifecycle) | **REDESIGN** | move module by module, leaving `pub use` shims at the crate root until each set of callers is updated, then delete the shims | one canonical owner crate must exist per destination; `apeireth-storage` does not yet |
| `apeireth-protocol` | four-protocol normalization, zero internal deps, no I/O | itself | **KEEP** | none; this is the canonical protocol layer | — |
| `apeireth-companion` (`plugin.rs`, `capability.rs`, `capabilities_manifest.rs`, `runtime_capabilities.rs`) | a `Plugin` trait, a `PluginRegistry`, and three separate `Capability` structs | `apeireth-plugin` | **MERGE** | re-express each as a `CapabilityDescriptor` in a manifest; delete the rival registry | callers audited — these are internal to companion, so this is the least risky registry merge |
| `crates/_frozen/apeireth-plugin` | frozen prior plugin system, 1,276 LOC, no callers | `apeireth-plugin` | **DEPRECATE** | mine for anything the canonical model lacks, then delete the frozen copy | none |
| `reconstruction_v2/` (10 crates, separate workspace) | parallel prototype, not built by the root workspace | — | **REMOVED** | deleted in the pre-freeze cleanup after the audit captured the useful governance and clock ideas; Git history remains the recovery path | no remaining product or build references |

---

## 2. Provider and protocol

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-llm-iface` | the single `LlmProvider` trait, capability bitflags, health, streaming | `apeireth-plugin::ProviderCapability` | **MERGE** | keep as the legacy contract; add a shim implementing `ProviderCapability` over any `LlmProvider`, so all five existing providers become capabilities at once | agree the shim's `LlmRequest` ↔ `NormalizedRequest` conversion |
| `apeireth-provider` | five merged provider implementations | plugins declaring `provider.*` | **PORT** | one plugin per vendor, each declaring `provider.<vendor>` and resolving its key through `CredentialResolver` | the `LlmProvider` shim above |
| `apeireth-api::llm::router` (`MultiLlmRouter`) | fallback order, health EMA, retryable classification | `apeireth-runtime::canonical::provider::ProviderRouter` | **PORT** | **algorithm already ported.** Delete the original once `apeireth-api`'s own callers move to the canonical router | `apeireth-api` callers migrated |
| `apeireth-api` (rest) | four-protocol handlers, chat pipeline, keep-alive pool | `apeireth-protocol` + provider plugins + `apps/gateway` | **REDESIGN** | split: protocol handling is already duplicated in `apeireth-protocol`; the pipeline overlaps `Runtime::execute`; the HTTP surface belongs to the gateway | the gateway app must exist |
| `apeireth-pipeline`, `apeireth-pipeline-g5` | five-stage chat pipeline; generic five-stage substrate | `apeireth-runtime::canonical::execute` | **MERGE** | the agent loop is the canonical pipeline; harvest token budgeting and the suppression window as loop concerns or governance hooks | decide whether the generic `Pipeline<T,I,O>` substrate has a user outside chat |
| `apeireth-http-client` | keep-alive LIFO connection pool | provider plugins | **KEEP** | stays as infrastructure a provider plugin uses; must never be reachable from `apeireth-protocol` | — |
| `apeireth-protocol::gateway`, `bridge`, `bridge_ext` | higher-level facades over the adapters | `apeireth-protocol` | **KEEP** | leave; they are translation-side facades, not routing | — |

---

## 3. Tools, capabilities, and registries

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-tool-registry` | six-category tool enum, five orthogonal axes, token budget, hot reload, async task store | `apeireth-plugin` registries, plus a typed view | **MERGE** | the categories become `CapabilityKind` + metadata; the registry becomes a view over `CapabilityRegistry`; `AsyncTaskStore` is a runtime concern and moves with it | `AsyncTaskStore` has callers in `apeireth-runtime`; item #1 first |
| `apeireth-tool-runtime` | tool call parser, executor, record, privacy guard | `Runtime::execute` + governance hooks | **MERGE** | parsing and execution are the loop; the privacy guard becomes a `GovernanceHook` | — |
| `apeireth-tool-shell`, `-filesystem`, `-browser`, `-fetch`, `-search`, `-codesearch`, `-image-gen`, `-image-process` | eight concrete tools | plugins declaring `tool.*` | **PORT** | wrap each as a `ToolCapability` behind one plugin per tool, ids `tool.shell`, `tool.filesystem`, … | none; these are the easiest migrations and the best first proof |
| `apeireth-tool-approval` | tool approval flow | `apeireth-governance` | **PORT** | becomes a `GovernanceHook` returning `Decision::RequireApproval` — the variant exists precisely for this | — |
| `apeireth-tools` | five integration traits, file operator, MCP adapter | plugins + `transport.mcp` | **MERGE** | split the tool implementations from the MCP adapter | MCP ownership decided below |
| `apeireth-skills` | reusable capability declarations with schemas, `SkillRegistry` (dual-channel) | `apeireth-plugin` | **MERGE** | a skill is a capability declaration; its registry becomes a view. Do not keep a second store | reconcile skill semver rules with `PluginManifest::version` |
| `apeireth-extension` | six extension classes, `extension.toml` schema, sandbox, call audit | `apeireth-plugin` (`CapabilityKind::Extension`) | **MERGE** | `extension.toml` maps onto `PluginManifest`; the sandbox stays as infrastructure; the call audit becomes an observer capability | agree the manifest schema superset |
| `apeireth-evolution` (`traits.rs`) | a third `Plugin` trait, a third `PluginRegistry`, an `Extension` trait | `apeireth-plugin` | **MERGE** | delete the rival trait and registry; evolution keeps its state machine and consumes canonical capabilities | audit callers inside `apeireth-evolution` |
| `apeireth-mcp` | MCP client/server, JSON-RPC 2.0, stdio/SSE transport, tool-registry bridge | a plugin declaring `transport.mcp` | **PORT** | the plugin exposes each remote MCP tool as a `ToolCapability` with id `tool.mcp.<server>.<name>`. **No** `McpRegistry`, `McpRuntime`, `McpAgent`, or `McpPermissionSystem` — those would be a second capability ecosystem | dynamic capability registration: MCP tools are discovered at connect time, and `PluginManager` currently indexes capabilities at register time |
| `apeireth-agent` | agent alias resolution, LRU cache, hot reload, subagents | `apeireth-runtime` | **REDESIGN** | an agent is a configured `Runtime` plus a system prompt and a capability subset, not a separate machine | canonical support for scoping a runtime to a capability subset |

> **Known gap.** MCP is the one migration that needs a canonical feature that does
> not exist yet: capabilities appearing *after* a plugin is active. The registries
> deliberately reject late duplicate registration, so adding dynamic capabilities
> needs an explicit, checked API rather than a hole in the existing one.

---

## 4. Governance

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-guard` | PII detection, scrubbing, audit | `apeireth-governance` hook | **PORT** | implement `GovernanceHook`; deny with the detector's reason | — |
| `apeireth-council` | seven mandatory advisors, hold mechanism, synthesis | `apeireth-governance` hook | **PORT** | implement `GovernanceHook`, returning `RequireApproval` when consensus is not reached | council currently owns a mock LLM provider; that should become a `ProviderCapability` instead |
| `apeireth-library-governance` | policy strategy, verification invariants, consistency checks | `apeireth-governance` hook | **PORT** | wrap each policy as a hook and compose with `GovernancePipeline` | — |
| `apeireth-onion`, `apeireth-sovereignty` | principle/permission onions, tenant sovereignty DSL | `apeireth-governance` hooks | **PORT** | the onion layers become an ordered pipeline — the shape `GovernancePipeline` already has | resolve the overlap with `apeireth-core`'s onion copy (item #2) |
| `apeireth-constraint`, `apeireth-rate-limiter` | constraints, rate limiting | `apeireth-governance` hooks | **PORT** | both are `Decision::Deny` with a reason | — |
| `apeireth-arbitration` | HASH-SQL append-only canonical timeline | `apeireth-storage` + an observer capability | **REDESIGN** | the audit chain is a consumer of `ExecutionTrace`, not a parallel record of its own | `apeireth-storage` must exist |

---

## 5. Storage and memory

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-memory`, `apeireth-memory-extensions` | memory engine, nine provider backends | `apeireth-storage`, exposed as `memory.*` capabilities | **PORT** | the nine backends become `CapabilityKind::Memory` capabilities behind one plugin each | `apeireth-storage` created |
| `apeireth-vector`, `apeireth-graph`, `apeireth-graph-primitive` | vector retrieval, graph orchestration | `apeireth-storage` | **MERGE** | fold into storage as retrieval backends | as above |
| `apeireth-context-fold` | context folding | `apeireth-runtime` or `apeireth-storage` | **REDESIGN** | decide whether folding is a transcript concern (runtime) or a recall concern (storage) | — |
| `apeireth-credentials` | OS keyring, `SecretBuf` zeroize, encrypted file backend | itself, behind `plugin::CredentialResolver` | **KEEP** | add a `CredentialResolver` implementation backed by this crate; it is the intended production backend for the trait | none — this is a small, high-value next step |
| `apeireth-state`, `apeireth-config` | shared state patterns, configuration | `apeireth-runtime` composition | **MERGE** | configuration feeds `RuntimeBuilder`; the state patterns are superseded by the runtime owning its own state | — |

---

## 6. Entry points

| Legacy | Current responsibility | Canonical owner | Decision | Migration strategy | Prerequisite |
| --- | --- | --- | --- | --- | --- |
| `apeireth-gateway` | HTTP/WS surface, observability routes, semantic router | `apps/gateway` | **REDESIGN** | must stop doing session management, provider routing and tool dispatch itself and call `runtime.execute` instead; `semantic_router.rs` is a fourth router and should become provider-router policy | item #1, since the gateway depends on `apeireth-runtime` |
| `apeireth-cli` | CLI | `apps/cli` | **PORT** | move under `apps/`, drive `Runtime::execute` | — |
| `apeireth-tui`, `apeireth-tui-e2e` | terminal UI | `apps/` or a UI crate | **KEEP** | move under `apps/`; must call the runtime rather than reimplementing chat | item #1 |
| `apeireth-web`, `crates/_frozen/apeireth-tauri-stub` | Leptos web surface; frozen Tauri reference | `apps/desktop` | **REDESIGN** | out of scope this phase; decide the desktop story before moving either | — |
| `apeireth-sdk`, `apeireth-sdk-*` | pure-Rust SDK client and language bindings | own crates | **KEEP** | retarget at the canonical contract types | canonical types stable |

---

## 7. Long tail

The remaining crates are cognition, infrastructure, or domain content and are
**not** blocked on the convergence. They are listed so the map is complete.

| Group | Crates | Decision | Note |
| --- | --- | --- | --- |
| Companion cognition | `apeireth-cognition`, `-consciousness`, `-perception`, `-motivation`, `-emotion` (in companion), `-life-force`, `-value`, `-experience`, `-asi`, `-central` | **MERGE** into `apeireth-companion` over time | several are already transparent re-exports of one another |
| Orchestration support | `apeireth-supervisor`, `-cron`, `-bus`, `-workflow`, `-team-lead`, `-acp` | **KEEP** pending item #1 | their canonical home follows the legacy-orchestrator decision |
| Infrastructure | `apeireth-host`, `-telemetry`, `-i18n`, `-repo-tools`, `-environment`, `-upgrade`, `-guard` | **KEEP** | infrastructure a plugin or app may use |
| Quality and tooling | `apeireth-test`, `-eval`, `-bench`, `-verify`, `-integration-e2e`, `release-tools`, `apeireth-naming-v05`, `apeireth-blueprint-impl` | **KEEP** | no canonical implications |
| Domain and frontier | `apeireth-voice`, `-livekit`, `-lark`, `-stock`, `-wiki`, `-pybridge` | **KEEP**, frozen | scope guard: audited and retained, not extended |
| Already archived | `crates/_archived/*`, `crates/_frozen/*` | **DEPRECATE** | delete when nothing references them |

---

## 8. Rules for executing this map

1. **One migration per commit.** A commit that moves two responsibilities cannot
   be reverted independently.
2. **Shim, migrate callers, then delete.** Never move a type and update forty
   call sites in one change.
3. **Do not delete a legacy crate while it still has a dependent.** `cargo tree
   -i` before every removal.
4. **A migration that cannot state its prerequisite is not ready.** Every row
   above has one, or an explicit dash.
5. **The canonical registries never gain a rival.** If a migration seems to need
   a second registry, it needs a typed view instead.
