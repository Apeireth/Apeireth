# ALE / System One P3 — bounded module messages

[Document-Meta]
Document: docs/04-internal/ale-system-one-p3-2026-10-30.md
Version: 0.1
Last-Modified: 2026-10-30
Status: IMPLEMENTED, explicit Host opt-in; P3 MainOnly regression PASS; overall Partial
Author: Codex

ModuleMessages is a typed owner-bound port with Command/Query/Event/Reply and a runtime-minted versioned Envelope. Existing PluginManager generations and CapabilityRegistry entries remain the only registration/lifecycle facts. ModuleMessagePolicy stores route grants and bounded transport state only; it is not a second registry or Provider loop. Default AgentHostPolicy.message_policy is None. No production default or hardware claim follows from these tests.

Real path: module Rust logic -> bound port -> source scope -> bounded admission -> route grant intersected with existing ModuleAgentGrant and caller delegation, when present -> active receiver generation/public contract -> existing private mailbox AgentInstance/Session -> original dispatch_one_tool -> deterministic admission -> System One for protected/Internal effects -> original native executor -> one private ToolEventV1 -> correlated Reply. Mailboxes perform no Provider reasoning. Source and receiver definitions/generations, principal, trace and authorization source are Host-minted. Direct own-Internal calls use the same path and gate, never public cross-module addressing.

Caller scope is checked both as a snapshot and as the current Dispatcher scope; principal/epoch, deadline, cancellation and complete delegation are rechecked before and after review. The original opaque security context is propagated. Independent standing background authority needs a distinct explicit Host grant and never fills gaps in a caller delegation. Standing/delegated mailbox identities are separate and consume existing explicit instance quotas; limits are never raised automatically. Internal/public status remains module-owned. Query-safe tools must be pure Direct logical tools.

In-flight admission, payload/result bytes, hop depth, policy/request deadlines and retained idempotency records are bounded. Saturation returns Backpressure. Local idempotency refuses repeat effects and returns the original message ID; changed payload returns IdempotencyConflict. It does not replay private results, survive process restart, promise exactly-once, or prevent duplicates after completed-entry eviction. Failed/uncertain executions remain retained conservatively, so unresolved windows can require Host recovery rather than silent replay. Cancellation cannot promise rollback of already-started native effects. Pre-start reservations are removed; started interruptions retain uncertainty. Explicit module stop drains existing scopes/native leases before recording private interruption facts, and stale generations remain invalid after re-registration.

Events require explicit route authorization and return an acknowledgement without receiver business results. PublicStatus additionally needs Host permission; the receiver must whitelist its public fields. Messages have Noop main event sink and private Sessions, so neither raw receiver data nor internal transcript is automatically placed in Main. A logical relay must choose its projection. Unsolicited Reply input is invalid; the Runtime correlates replies. There is no implicit subscription replay, persistence transport, cross-process transport, NervousBus replacement or device claim.

Compatibility: old ToolEventV1 Model/Module variants, Frozen Approval, old Session variants and default constructors remain. New opt-in Session ModuleMessageStarted/Interrupted and ToolEventV1 ModuleMessage source require the updated reader for newly produced records. Historical logs are not rewritten. ToolCancellation now supports parent observation without cancelling parents when a child completes. Default-off tool contexts avoid capturing message delegation when the pipeline is absent.

Sources: `crates/foundation/plugin/src/module_messages.rs`, `tool_execution.rs`, `module_contract.rs`; `crates/engine/runtime/src/canonical/module_messages.rs`, `instances/messages.rs`, `message_execution.rs`, `dispatcher/system_one.rs`, `module_host.rs`, `instances/audit.rs`; protocol `canonical/tool_event.rs`. ADR-0023 and canonical concepts register the contract without changing extension families, thresholds or baselines.

Evidence: `cargo test -p apeireth-runtime-assembly --test module_message_pipeline --test modular_ale --test system_one_execution --locked -j 2`, raw `logs/p3-tests-complete.log/.exit = 0`. Source manifests capture actual tested source bytes; fixtures drive real Runtime/Dispatcher/file/store paths, not a live external model. Tests in `module_message_pipeline.rs`:

| Test | Evidence |
| --- | --- |
| command_reply_uses_authenticated_generations_gate_and_one_private_fact_without_main_reasoning | Real file effect, one review/fact/usage, real generations and trace, zero Main Provider requests |
| query_event_private_surface_and_forged_payload_cannot_borrow_authority | Query restrictions, Event acknowledgement, unsolicited Reply/private addressing refusal, schema rejects forged payload identity |
| messages_default_disabled_and_missing_supervisor_never_execute_effects | Default-off refusal and absent supervisor no-effect |
| loop_detection_precedes_reentrant_mailbox_and_idempotency_refuses_duplicate_effects | Authenticated A-B-A loop refused before lock; duplicate/conflicting keys never repeat effect |
| bounded_admission_cancellation_and_timeout_report_uncertain_execution | Saturation, token cancellation/deadline, dropped native future no delayed effect, uncertain execution explicitly reported |
| caller_delegation_intersects_scope_and_never_uses_standing_grant_as_fallback | Explicit root scope intersection, no standing fallback, filtered main result, original delegate retained |
| owner_internal_port_requires_its_own_grant_and_the_same_supervisor | Private own-tool Allow/Deny and audit; public path cannot address it |
| stopping_source_drains_private_mailbox_and_old_generation_cannot_reacquire_authority | Actual unregister/drain, one private interruption record, stale port rejected, new generation runs |

Original main regression, clippy/fmt and strict architecture results are recorded in the external phase report after phase exit. P4 repeats all original image-family stages. Existing P1/P2 scenarios are rerun here. Earlier fixture/expectation failures remain in raw logs; security checks were retained. Compatibility enrollment C01/C02 remains open and blocks universal/default protection claims; P3 resolves the C03 distinction for the new pipeline, not every historical helper.

Rollback: normal reverse-order revert of P3 docs/source commits; disable the optional Host policy to retain prior composition. Persisted new variants must be read with the new reader before reverting an active writer; do not rewrite historical evidence. No external dependencies, Cargo.lock/workspace version or LOCKED values changed. Commit SHAs and final PR link are in the external STATE/report.

Phase exit: original MainOnly gate passed at 2026-10-30 01:23:06 Asia/Shanghai; raw logs/p3-main-gate.log/.exit = 0, source_unchanged true. Strict architecture/arch-doc, docs links/index and historical/test preservation passed. Source commit: 4ff8853bd (pushed). Claims point to this source SHA; docs commit follows. Final PR link will be added in STATE.
