# reconstruct_v2 Convergence Baseline — Architecture Reality Audit

Status: recorded 2026-08-23. Baseline for the canonical-skeleton convergence work.

## 0. Git baseline

| Item | Value |
| --- | --- |
| Remote | `https://github.com/Apeireth/apeireth-rust.git` |
| Branch | `reconstruct_v2` (authoritative working baseline) |
| Starting HEAD | `b0a017f060841119b46d79e28870732ae80e1aed` |
| HEAD subject | `feat(frontier): complete real-time Voice Call UI, Screen Agent, Software Factory and MCP Hub` |
| Parent | `0dcb64cb9b241fc8334705cd69d2d4512204c9cb` |
| Working tree at clone | clean |
| `origin/master` | historical reference / donor only — **not** merged, **not** rebased |

## 1. The three concepts (do not conflate)

| | What it is | Role in this work |
| --- | --- | --- |
| A. `reconstruct_v2` **branch** | the Git baseline | the only working baseline |
| B. **root workspace** (`/Cargo.toml`, `crates/`) | the real, large, historical implementation | mature donor; convergence target |
| C. `reconstruction_v2/` **directory** | a *separate nested* Cargo workspace | idea donor only; not a target |

## 2. Measured reality

Measured on the clean clone, not estimated.

| Metric | root workspace | `reconstruction_v2/` |
| --- | --- | --- |
| Cargo workspace | `/Cargo.toml`, `resolver = "2"` | `reconstruction_v2/Cargo.toml`, **separate workspace** |
| Members (`cargo metadata --no-deps`) | **86** | 10 |
| `.rs` files | 1752 | 91 |
| Lines of Rust | **587,916** | 9,261 |
| `cargo check` | **PASS**, exit 0, 2m25s | not part of any root build |

Two consequences follow directly from this table, and both contradict what the
directory name suggests:

1. **`reconstruction_v2/` is not the product.** It is 1.6% of the Rust in the
   branch and is not a member of the root workspace, so no root `cargo check`,
   `cargo test`, or CI job ever compiles it. The 588k-line root workspace is the
   product.
2. **The branch has not converged.** Being on `reconstruct_v2` does not mean the
   architecture was reconstructed; it means two architectures coexist, one of
   which is unbuilt by the main workspace.

### Baseline validation

`cargo check --workspace` at the root exits 0. There is **no pre-existing
baseline failure** to excuse later breakage: any compile failure introduced from
here is a new regression.

## 3. Fragmentation evidence

Concrete duplicate ownership found in the root workspace (`rg` over `crates/`):

| Concept | Competing definitions |
| --- | --- |
| `PluginRegistry` | `apeireth-companion/src/plugin.rs:29`, `apeireth-evolution/src/traits.rs:182`, `_frozen/apeireth-plugin/src/lib.rs:579` |
| `Capability` (struct) | `apeireth-companion/src/capability.rs:142` (`CapabilityRegistry`), `apeireth-companion/src/capabilities_manifest.rs:41`, `apeireth-companion/src/runtime_capabilities.rs:45` |
| `ToolRegistry` | `apeireth-tool-registry/src/registry.rs:63`, `apeireth-pybridge/src/tool_self_loop.rs:326`, plus `reconstruction_v2/crates/apeireth-tools/src/lib.rs` |
| `Plugin` (trait) | `apeireth-companion/src/plugin.rs:18`, `apeireth-evolution/src/traits.rs:171` |
| Provider routing | `apeireth-api/src/llm/router.rs:18` (`MultiLlmRouter`), `apeireth-pipeline/src/model_router.rs`, `apeireth-gateway/src/semantic_router.rs`, `apeireth-api/src/llm/semantic_router.rs` |
| `PipelinePool` | only in `apeireth-companion/examples/companion_serve.rs:164` — an **example file**, not a library type |

There is no single source of truth for "what capabilities exist". That, not the
crate count, is the actual architectural defect.

### One thing that is *not* fragmented

`LlmProvider` has exactly one definition. `apeireth-api/src/llm/traits.rs` is a
thin re-export of `apeireth-llm-iface::traits`. This was already converged by an
earlier pass and should be preserved.

## 4. Capability matrix

Decision vocabulary: `KEEP` / `PORT` / `MERGE` / `REDESIGN` / `DEPRECATE`.

| Concept | Root implementation | `reconstruction_v2/` implementation | Maturity | Tests | Real E2E? | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| **core primitives** | `apeireth-core` — 7,437 LOC, but owns memory/onion/philosophy/gates/lifecycle; 38 dependents | `apeireth-core` — 516 LOC; raw `Uuid`, no ID newtypes; owns `Episode`/`Note`/`Session` | neither is a primitives crate | unit | n/a | **REDESIGN** — add canonical primitives; legacy content stays, scheduled for eviction |
| **clock** | `apeireth-core/src/clock.rs` | `clock.rs` — `Clock` trait + `SystemClock` + `VirtualClock` | good in both | unit | n/a | **PORT** the injectable-clock contract into canonical core |
| **event bus** | `apeireth-bus` (`ChanneledBus`, 3-channel) | `core/src/bus.rs` — broadcast, `payload: String` | root more mature | unit | yes | **KEEP** root; canonical core owns only the `Event` primitive |
| **protocol normalization** | `apeireth-protocol` — 5,962 LOC, **zero internal crate deps**, no HTTP, no credentials, 4 adapters (OpenAI Chat/Responses, Anthropic Messages, Gemini) | `apeireth-protocol` — `ProtocolAdapter::execute(&self, api_key, req)`: adapter owns HTTP **and** credentials | root is already the correct shape | ≥50 unit + wire-format | yes | root **KEEP** (canonical); nested **DEPRECATE** |
| **provider abstraction** | `apeireth-llm-iface::LlmProvider` — capabilities bitflags, health, `complete`, `complete_stream`; zero internal deps | none (adapter *is* the provider) | root mature | unit | yes | **KEEP** as the legacy provider contract; canonical `Provider` speaks normalized types |
| **provider routing** | `MultiLlmRouter` — fallback order, health EMA, retryable classification; lives inside the large `apeireth-api` | none | root mature but mislocated | unit | yes | **PORT** the algorithm to `apeireth-runtime::canonical::provider` |
| **credentials** | `apeireth-credentials` — OS keyring, `SecretBuf` zeroize, encrypted file backend | hardcoded `api_key: String` field on the host | root mature | unit | yes | **KEEP** root; canonical runtime takes an injected credential resolver |
| **tools** | `apeireth-tool-registry` + 10 `apeireth-tool-*` crates | `apeireth-tools` — `Tool` trait + flat `ToolRegistry` | root broader, nested simpler | unit | partial | **MERGE** behind the canonical capability model |
| **plugin / capability** | no canonical model; 3 rival `PluginRegistry`, 3 rival `Capability`; `apeireth-plugin` is frozen | none | absent | — | no | **REDESIGN** — new `apeireth-plugin` crate |
| **governance** | `apeireth-guard` (PII), `apeireth-council`, `apeireth-library-governance` — scattered | `apeireth-governance` — 5 gates (compile-time / runtime / council / physical-isolation / reflection-audit) | nested idea is cleaner; root parts are real | unit | partial | **PORT** the gate concept into a canonical `GovernanceHook` |
| **runtime composition root** | `apeireth-runtime` — 7-module orchestration driver, 10 internal deps, 2 dependents (gateway, tui) | `UnifiedRuntimeHost` — 20-field God object, hardcoded `MinimaxAdapter`, hardcoded model, `api_key: String` | neither is a composition root | unit | partial | **REDESIGN** — borrow only "one living runtime" |
| **agent loop (tool call)** | none end-to-end | **none** — `handle_chat_turn` calls the provider once; `tool_registry` is registered but never dispatched | absent in both | — | **no** | **REDESIGN** — this is the main gap |
| **MCP** | `apeireth-mcp` + `apeireth-tools` adapter | `apeireth-tools/src/mcp/{client,server,protocol,transport}` | both partial | unit | partial | **MERGE** as a transport capability, not a second ecosystem |
| **storage** | `apeireth-memory` (+ extensions), `apeireth-vector`, `apeireth-graph` | `apeireth-storage` — sqlite pool, migrations, ACT-R memory | root broader | unit | yes | **DEFER** — out of this phase's scope |
| **companion cognition** | `apeireth-companion` and ~15 organ crates | `apeireth-companion` — emotion/dream/emergence | root broader | unit | yes | **DEFER** — scope guard |
| **voice / screen / factory** | `apeireth-voice`, `apeireth-livekit` | `tools/vision`, `protocol/voice`, gateway frontier routes | recent | thin | no | **DEFER** — explicit scope guard, no further expansion |

## 5. Findings that drive the design

1. **The nested `reconstruct_v2` protocol layer is the wrong shape and the root
   one is right.** `ProtocolAdapter::execute(&self, api_key: &str, ...)` makes
   the adapter own credentials, the HTTP client, and its lifetime. The root
   `ProtocolAdapter` is `adapt_request` / `adapt_response` over
   `serde_json::Value` with no I/O at all. Canonical protocol = the root crate.

2. **No agent loop exists anywhere.** `UnifiedRuntimeHost::handle_chat_turn`
   builds a prompt, calls one provider once, and returns the text. It constructs
   a `ToolRegistry` with four tools and never dispatches to it. There is no
   second provider round-trip. The "minimal tool-call loop" therefore has to be
   built, not ported.

3. **Runtime is bound to one concrete vendor.** `UnifiedRuntimeHost::new` hard-codes
   `MinimaxAdapter::new()` and `"MiniMax-Text-01"`, and stores `api_key: String`
   in the struct. This is exactly the provider coupling the canonical layer must
   forbid.

4. **Raw chain-of-thought is a public contract in the nested design.**
   `ChatTurnOutput.reasoning_cot: Option<String>` is a required public field, and
   the fast path fabricates a value for it. Canonical output must not carry raw
   CoT.

5. **`apeireth-core` is not a core.** 7,437 LOC of memory items, philosophy
   verdict caches, permission onions, and a 9-phase *cognitive* lifecycle, with
   38 dependents. It cannot be gutted in this pass without breaking the whole
   workspace; it can be given a canonical primitives namespace now and drained
   later.

6. **Credentials are clean enough.** No developer-specific absolute path is
   embedded in production code. `crates/apeireth-companion/src/daemon.rs` uses
   `C:/Users/u/...` only inside platform-path unit tests, and the `apikey.txt`
   references resolve relative to `$HOME`/`%USERPROFILE%`, not to a fixed machine.
   An earlier PII gate already redacted embedded usernames.

## 6. Convergence decision

Converge **inside the root workspace**, because the goal is one root workspace
and the root workspace is where the 588k lines and the passing build already are.

Name availability in `crates/` decides the mechanism per crate:

| Canonical crate | Name status in root `crates/` | Mechanism |
| --- | --- | --- |
| `apeireth-core` | occupied by legacy (38 dependents) | evolve in place: add a canonical primitives namespace, leave legacy re-exports untouched |
| `apeireth-protocol` | occupied, **already correct** | keep as canonical |
| `apeireth-plugin` | free (only `_frozen/apeireth-plugin`) | create |
| `apeireth-governance` | free | create |
| `apeireth-runtime` | occupied by legacy (2 dependents) | evolve in place: add the canonical composition root |
| `apeireth-storage` | free | deferred — not in this phase's priority list |

`reconstruction_v2/` is left on disk untouched as a donor and is scheduled for
removal in the migration map. No third architecture is created: no
`reconstruction_v3/`, no `canonical_v2/`, no parallel root directory.

### Known transitional impurity, stated up front

The canonical composition root lives in `crates/apeireth-runtime`, which still
carries the legacy 7-module orchestrator and therefore still depends on
`apeireth-council`, `apeireth-consciousness`, `apeireth-arbitration`, and six
other crates. The canonical *modules* depend only on core / protocol / plugin /
governance, and the crate-level DAG stays acyclic, but the crate boundary is not
yet clean. Evicting the legacy orchestrator is migration item #1. This is
recorded rather than hidden because pretending the boundary is already clean
would be the failure mode this whole exercise exists to fix.
