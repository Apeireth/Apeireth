# Apeireth Memory 2.x Final Production Lifecycle and Integrity Report

**Date:** 2026-09-11
**Repository:** `Apeireth/Apeireth` (`Apeireth/apeireth-rust` remote)
**Worktree:** `H:\项目\CrossPlatform\Apeireth\apeireth-rust-memory-v2.2-production-completion`
**Branch:** `feature/memory-v2.2-production-completion`
**Final local/remote SHA at inspection:** `7f8fddd6cca4ae96463564c77d983aa53ca94130`

## Executive decision

This acceptance pass added real evidence and fixed two narrow extraction defects, but it does **not** satisfy the strict freeze gate. The implementation is reliable for the verified episodic/provider path and several durable typed-store lifecycles; it is not a complete five-type provider-recall system because commitment, persona, and temporal typed stores remain write-side projections without a unified coordinator recall bridge. Universal cross-store Forget is also not implemented.

```text
MEMORY_PRODUCTION_LIFECYCLE_COMPLETE = NO
MEMORY_INTEGRATION_READY = NO
MEMORY_FREEZE_READY = NO
CI_VERIFIED = YES  (for the inspected pre-change SHA f73fdb6c; new changes require new CI evidence)
TAURI_PACKAGING = BLOCKED_EXTERNAL_PREREQUISITE
```

No new Memory architecture, database, migration, model training, Guard ML, main/next merge, fake provider, fake E2E, or sidecar was introduced.

## Repository and CI evidence

| Check | Evidence |
|---|---|
| Fetch/rebase relation | `git fetch origin`; merge-base with `origin/feature/cognitive-infrastructure-vnext` = `9c7ae927`; `HEAD..next` count = `0` |
| Branch | `feature/memory-v2.2-production-completion` |
| Pre-change remote equality | `git ls-remote origin refs/heads/feature/memory-v2.2-production-completion` = `f73fdb6c` |
| Existing remote CI | GitHub run `34439806389` (Rust tests OS matrix) and formatter/deny/audit/lint runs for the inspected SHA all completed successfully |
| Final-SHA remote CI | GitHub run `34682298079`: Ubuntu, macOS, Windows nextest, hard-wall, and secret scan all completed successfully; formatter, deny, audit, and clippy runs for `7f8fddd6` also succeeded |

## Real canonical E2E evidence

`crates/engine/runtime-assembly/tests/memory_provider_e2e.rs` uses the canonical `ProductionCognitiveModules` assembly, a temporary **file-backed SQLite database**, a real `Runtime`, a recording provider, a full restart, and provider request inspection.

Turn 1 now contains the requested preference, relation, commitment, and location statements. After restart, Turn 2 asks about all four. The test passes and proves these statements survive as episodic user text and appear in a governed provider overlay. The test also verifies the assistant response is persisted. It does **not** claim typed commitment/persona/temporal provider recall, because the coordinator does not read those typed stores.

| Type | AfterTurn | Durable | Restart | Retrieved/overlay | Provider request |
|---|---:|---:|---:|---:|---:|
| Fact/location text | YES | YES | YES | YES | YES (episodic text) |
| Preference text | YES | YES | YES | YES | YES (episodic text; separate preference owner not asserted here) |
| Relation text | YES | YES | YES | YES | YES (episodic text) |
| Commitment text | YES | YES | YES | YES | YES (episodic text) |
| Typed temporal relation | candidate/sink only | durable-store tests | YES at store level | YES when `TypedMemoryRecallSource` is injected | YES in typed-source E2E; canonical provider wiring remains explicit-config only |
| Typed commitment | candidate/sink only | durable-store tests | YES at store level | YES when `TypedMemoryRecallSource` is injected | YES in typed-source E2E; canonical provider wiring remains explicit-config only |
| Typed persona | explicit identity fixture | durable-store tests | YES at store level | YES when an explicit profile fixture/source is injected | YES in typed-source E2E; missing identity remains skipped |

This follow-up also wires the typed source through the canonical assembly seam in test/embedding configurations; the canonical provider E2E now passes a real typed source for the same file-backed store and explicit identity. Typed IDs therefore enter the same provider overlay path rather than being only store-level fixtures.

## Typed recall bridge and universal forget status

This follow-up pass adds the provider-neutral `TypedMemoryRecallSource` port in `crates/engine/memory/src/scope.rs`, the existing-ranking-path integration in `MemoryCoordinator`, and the assembly-owned `SqliteTypedMemoryRecallSource` adapter. Typed commitment, persona, and temporal candidates now have stable IDs and can flow through the same retrieval/compiler path when an explicit `{persona_id, subject_id}` is supplied. The adapter never derives identity from a session.

The follow-up also adds `UniversalForgetFacade` as an existing-store composition boundary and keeps generic episode governance persistent. It does not physically delete append-only typed provenance. The current matrix proves episodic governance and typed terminal/tombstone store semantics, but a complete cross-store principal-level forget transaction and persona tombstone lifecycle still require additional production wiring; therefore universal typed Forget remains `PARTIAL`, not a false PASS.

The provider E2E now extracts memory IDs from the actual rendered `[mem:<id>]` lines and asserts the receipt inclusion relation for the IDs that reached the provider. The test's retrieved/selected sets are derived from the exact selected overlay in this harness; a production receipt channel carrying independent pre-budget retrieved IDs remains a follow-up observability enhancement.


`crates/engine/runtime-assembly/tests/memory_typed_lifecycle.rs` uses one real SQLite file and the production `CanonicalMemoryTypedSink` plus official store/query APIs.

- Commitment: create Active, transition to Completed, close/drop all consumers, reopen the same file, and verify Completed.
- Temporal graph: append Wuhan, supersede with Shanghai, reopen, verify Shanghai is current at the later as-of time and Wuhan remains in immutable history.
- Persona: apply an explicit-identity delta, reopen/query the profile, verify same-user data; missing identity returns `Skipped`; another subject cannot read it.
- Existing commitment tests additionally cover CAS conflict and terminal-state protection.

These are **durable lifecycle proofs**, not provider-recall proofs.

## Extraction changes in this pass

- `RuleMemoryExtractor` now recognizes explicit completion (`I submitted…`, `I called…`, `I completed…`, etc.) and cancellation (`I no longer need…`, `I cancelled…`, etc.) as Event candidates while still rejecting hedged creates such as `I might call Ada sometime.`
- Typed relation materialization now accepts only the explicit conservative contract `relation: subject | predicate | object`; natural-language relation sentences remain generic episodic evidence and are not silently mis-split into a bogus graph triple.
- Added unit coverage for both behaviors.

## Governance / Forget / Protect matrix

| Path | Evidence | Status |
|---|---|---|
| Episodic recall | coordinator governance filtering | PASS |
| Fact/preference as episodic text | same governed episode path | PASS for episodic representation |
| Temporal typed store | source/tombstone-aware typed recall + store tests | PASS for active/current eligibility; universal principal transaction PARTIAL |
| Commitment typed store | active-only typed recall + terminal transition tests | PASS for active eligibility; universal principal transaction PARTIAL |
| Persona typed store | explicit identity/profile isolation | PARTIAL: no durable persona tombstone API |
| Activation bypass after episode Forget | file-backed high-activation test | PASS |
| Proactive recall after Forget | no cross-store typed target path | PARTIAL |
| Consolidation after Forget | governed consolidation tests | PASS |
| Context overlay after Forget | governed episodic overlay tests | PASS |
| Protect vs automatic retention/consolidation | file-backed tests | PASS |
| Protected provenance/history | append-only temporal/persona store tests | PARTIAL |

Hard invariant verified for governed episodic memory: forgotten episodes are excluded from retrieval, selection, overlay, consolidation, and post-restart activation ranking. It is **not** verified for all typed stores.

## Activation evidence

Existing `memory_integration.rs` drives access history through coordinator selection/overlay, persists selected-context events in a real SQLite access-history file, rebuilds the coordinator, and verifies the activation component for A exceeds B. The high-activation forgotten target is then excluded after restart. This satisfies the real episodic coordinator path; activation remains optional when an access-history backend is not injected.

| Evidence | Result |
|---|---|
| Access source | `AccessHistoryActivationSource` |
| Access event path | selected-context overlay path (not a direct test-only insert for the assertion) |
| Persistence | file-backed access-history SQLite |
| Restart score component | PASS in `memory_integration.rs` |
| Forget > activation | PASS in `memory_integration.rs` |
| Final ranking must be #1 | not required / not asserted |

## Proactive, trust boundary, context integrity

- Proactive recall remains default-disabled, bounded, candidate-only, and has no daemon. Existing service tests cover policy selection; full typed provider receipt remains unverified.
- Prompt-injection and credential-like memory text is rejected by the deterministic extractor; rendered memory is marked non-authoritative and sanitized by `ClosedWorldContextCompiler`.
- Latest user message, critical system policy, tool-call/tool-result ordering, and durable transcript immutability are covered by existing runtime tests (`canonical_agent_loop.rs`) and the runtime-owned projection tests.

## Fault and concurrency status

| Area | Status | Evidence |
|---|---|---|
| Commitment sink failure | PARTIAL | fail-open module behavior; no complete injected sink matrix |
| Persona CAS conflict | TESTED | persona store revision-conflict test |
| Temporal write failure | PARTIAL | store validation/CAS tests; no full runtime fault injection |
| Access-history failure | TESTED/PARTIAL | recall path is fail-open and warning-counted |
| Embedding failure fallback | TESTED | lexical fallback tests |
| Proactive failure | PARTIAL | policy/service tests; no full provider fault harness |
| Context projection failure | PARTIAL | safe projection contracts; no exhaustive injected failure matrix |
| Same-user concurrent sessions | PARTIAL | scope/store isolation tests |
| Cross-user isolation | TESTED for persona and preference stores | identity/session tests |
| Concurrent commitment completion | TESTED at CAS store level | commitment lifecycle CAS test |
| Concurrent persona update | TESTED at CAS store level | persona revision conflict test |
| Concurrent relation supersede | PARTIAL | append-only revision checks; no concurrent runtime harness |

## Validation run

| Gate | Result in this pass |
|---|---|
| `cargo fmt --all -- --check` | PASS after formatting |
| `cargo check --workspace --all-targets --locked` | PASS before this pass; rerun required for final SHA |
| `cargo test -p apeireth-memory --tests --locked` | PASS: library and integration suites; extraction rerun 720 library tests + integration suites |
| `cargo test -p apeireth-runtime-assembly --test memory_provider_e2e --locked` | PASS; typed source injected through canonical assembly seam |
| `cargo test -p apeireth-runtime-assembly --test memory_typed_lifecycle --locked` | PASS: 2 tests, including typed-source candidate recall |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS on final bridge commit |
| `cargo deny check` | PASS on final bridge commit; existing configuration/yanked warnings only |
| `cargo audit` | PASS on final bridge commit; allowed yanked warning only |
| Guard correctness/evaluation/security | PASS in workspace/remote hard-wall and secret-scan evidence |
| Frontend gates | PASS: 7/7 tests, check/build with 5 existing warnings |
| Runtime dependency wall | PASS from `cargo tree -p apeireth-runtime --edges normal --depth 4` |
| Tauri | BLOCKED_EXTERNAL_PREREQUISITE; no fake sidecar |

## Strict DoD answers

| Requirement | Status |
|---|---|
| Real file-backed SQLite E2E | YES |
| Canonical runtime assembly | YES |
| Real AfterTurn materializer invocation | YES for generic episodic path |
| Canonical typed sink used | YES in typed durable lifecycle test; CLI composition has sink available but no explicit identity |
| Fact/preference/relation/commitment text durable and provider-received | YES as episodic text |
| Typed commitment/persona/temporal provider receipt | YES when explicit typed source + identity are injected; canonical CLI defaults remain explicit-config only |
| Provider received subset selected subset retrieved | YES in the canonical overlay ID harness; independent pre-budget retrieved receipt is PARTIAL |
| Wuhan→Shanghai revision and Wuhan history | YES at typed store level |
| Commitment create/complete/cancel | PARTIAL: create/complete/cancel extraction and store APIs covered; full canonical conversational transition is not complete |
| Hedged commitment rejected | YES |
| Ambiguous completion skipped | NOT_TESTED |
| Persona same-user/cross-user/missing identity | YES at explicit sink/store level |
| Preference duplicate owner absent | YES for existing preference upsert contract; not re-proven in canonical multi-type E2E |
| Episodic Forget / high-activation bypass | YES |
| Universal typed Forget matrix | PARTIAL: facade + active/tombstone eligibility; persona durable tombstone and atomic cross-store transaction remain |
| Protect/consolidation episodic lifecycle | YES |
| ACT-R actual selection and restart score | YES for episodic coordinator path |
| Proactive relevant/irrelevant/default disabled | PARTIAL/YES for service policy; provider receipt NO |
| Prompt-injection memory untrusted | YES for extractor/compiler boundary |
| Context/tool/transcript invariants | YES in existing runtime tests |
| Critical fault/concurrency matrix | PARTIAL |
| Migration/idempotence/query plan | YES from existing tests |
| Frontend | YES with 5 pre-existing warnings |
| Runtime dependency wall | YES on pre-change evidence |
| Branch clean / final remote SHA / final-SHA CI | YES after final docs commit and CI run `34682298079` |

## Freeze decision

`MEMORY_FREEZE_READY = NO`. The strict stop rule is still not met: typed stores now have a unified provider-neutral recall bridge and real ID inclusion evidence, but Universal Forget is only PARTIAL because persona tombstone lifecycle and one atomic principal-scoped cross-store operation are not yet present. The correct next action is to finish that governance contract rather than claim a false freeze.
