# Cognitive module wiring ledger

> Active architecture record for v2.0.0-rc.1. This document maps existing
> cognitive, memory, orchestration, and perception concepts onto the existing
> `AgentModule` ABI. It does not introduce a second ABI or a second agent loop.

## Ownership and composition

`apeireth-runtime::canonical::ProductionCognitiveModules` is the one module
composition root. It accepts `Arc<dyn ...>` capability handles, validates
required dependencies, assigns the stable slot ids below, and registers modules
in one explicit order. The CLI is the current concrete adapter root. Gateway
calls the CLI bootstrap and therefore receives the same runtime. The SDK is a
client surface; it does not host a `Runtime` and must not register modules.

Embedding callers may construct the same `ProductionCognitiveModules` directly
and add caller-owned modules through `RuntimeBuilder::with_module`. Such
additions are explicit and remain subject to the runtime's duplicate-id check.

## Active slot ledger

| Slot / stable id | Owner | Hook | Status | Dependency / note |
|---|---|---|---|---|
| `cognitive.memory_recall` | runtime cognitive adapter | `TurnStart` | WIRED | `Arc<dyn MemoryBackend>`; optional Experience reads; transient overlay only |
| `cognitive.preference_recall` | runtime cognitive adapter | `TurnStart` | WIRED | `Arc<dyn PreferenceStore>`; transient overlay only |
| `cognitive.judge` | runtime cognitive adapter | `AfterModelResponse` | WIRED, OFF by default | one `ModuleInvoker` side-call at most per judge hook; typed JSON; no tools |
| `cognitive.council` | runtime cognitive adapter | `AfterModelResponse` | WIRED, OFF by default | bounded typed advisor path through `ModuleInvoker`; no tool dispatch |
| `cognitive.self_assessment` | runtime cognitive adapter | `AfterTurn` | WIRED, Judge-backed | records only a real Judge result; no fabricated heuristic score |
| `cognitive.memory_writeback` | runtime cognitive adapter | `AfterTurn` | WIRED | successful final turn only; append-only user/assistant Episodes |
| `cognitive.preference_learning` | deferred, no owner yet | — | DEFERRED | no evidence-extraction side-call or implicit preference mutation |
| `cognitive.critic` | Judge owner | — | DEFERRED INTO JUDGE | Judge's bounded critique is the single critique path; no duplicate evaluator |
| `cognitive.reflection` | SelfAssessment owner | `AfterTurn` | DEFERRED INTO SELF-ASSESSMENT | current-turn assessment is distinct from durable memory; long-term reflection pipeline remains future work |
| `cognitive.planner` | orchestration service | — | NOT AN AGENT MODULE | no per-turn planner loop; future adapter must remain an adapter |
| `cognitive.orchestrator` | `apeireth-orchestration::Orchestrator` service | — | NOT AN AGENT MODULE | long-running Planner → Implementer → Reviewer service; never called from the canonical turn |
| `cognitive.perception` | perception adapter | — | NOT AN AGENT MODULE | `PerceptionInput` becomes `TurnRequest` through `turn_request_from_perception`; only text payload is implemented |

Registration order is deterministic:

```text
TurnStart:          memory_recall -> preference_recall
AfterModelResponse: judge -> council
AfterTurn:          self_assessment -> memory_writeback
```

The runtime remains responsible for hook lifecycle, directive precedence,
round budget, approval state, and side-call budget. Modules do not access a
mutable runtime, session store, governance hook, capability registry, raw
provider, or tool executor.

## Backend wiring

The CLI opens one `SqliteConnectionPool` for the cognitive backends and runs
the memory, preference, Experience, and self-assessment schemas during boot.
The same pool is injected into:

- `SqliteBackend` as `MemoryBackend`;
- `SQLiteExperienceStore` as Wiki, knowledge-graph, and association traits;
- `SQLitePreferenceStore` as `PreferenceStore`;
- `SQLiteSelfAssessmentStore` as `SelfAssessmentStore`.

No module opens SQLite, reads environment variables, constructs a provider, or
executes a tool. The environment is read only by the adapter's small production
config: `APEIRETH_COGNITIVE_DB`, `APEIRETH_COGNITIVE_JUDGE=1`, and
`APEIRETH_COGNITIVE_COUNCIL=1`. Judge is disabled unless explicitly enabled;
normal memory recall/writeback has no additional model cost.

`LlmFactory` remains the future logical advisor/subagent factory. It is not
called by these modules. All current cognitive side-calls use the existing
runtime-owned `ModuleInvoker`, preserving depth and per-turn budget controls.

Experience storage and extraction are real and wired. After a durable
`AfterTurn` episode commit, the conservative extractor materializes only a
bounded summary plus explicitly marked `fact:`, `link:`, and `associate:`
records. Every artifact carries the source episode id; SQLite association
observations are idempotent. The production default does not make a hidden
model call, and extraction failures fail open with telemetry warnings. This
release does not claim a complete long-term reflection or preference-learning
pipeline.

## AI evaluates AI contract

`cognitive.judge` evaluates only final, tool-free candidates. Its request is an
isolated `ModuleInvocationRequest` with no tool declarations and no hook
recursion. The response must be an object with exactly `score`, `verdict`, and
`critique`; unknown fields, malformed JSON, non-finite/out-of-range scores, and
oversized critique fail closed as module errors. `retry` is emitted only below
the configured threshold and only within `max_retries`, so a retry cannot evade
the canonical round budget. Exhaustion stops the turn.

The candidate, Judge critique, and side-call response are not written to
memory by Judge. Self-assessment receives only the typed in-process result and
records it after commit. The telemetry sink stores module id, hook, directive,
duration, and side-call count, never prompt or response content.

`cognitive.council` uses the same isolated `ModuleInvoker` boundary. The
default runtime adapter exposes seven named advisor slots, bounds selection
and concurrency, applies a 10-second per-advisor timeout and a 60-second
overall timeout, and aggregates typed `allow`/`retry`/`stop`/`abstain`
decisions deterministically. Provider, malformed-response, and timeout
failures are explicit and defer to human review. The CLI keeps this path
disabled by default; fake-invoker tests cover the runtime wiring and the
orchestration package, while real provider E2E remains credential-gated.

## Non-goals preserved

This wiring does not modify the cognitive ABI, approval lifecycle, governance
policy, 13-key philosophy cache, immutable spines, workspace version, or R11
baseline. It also does not turn `PerceptionInput` into a module, does not make
Orchestrator a hidden loop, and does not claim screen/audio support before the
existing explicit `NotImplemented` paths are replaced.
