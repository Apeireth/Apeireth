# ALE / System One P2 — mandatory internal execution gate

[Document-Meta]
Document: docs/04-internal/ale-system-one-p2-2026-10-30.md
Version: 0.1
Last-Modified: 2026-10-30
Status: P2 regression PASS (MainOnly); IMPLEMENTED; overall Partial
Author: Codex

The owner clarification is enforced: System One is an additional supervisor model. Its verdict cannot grant deterministic authority. Host composition explicitly selects the new module contract, audit sink, exact provider/model and service budget. Existing constructors, permissions and compatibility defaults remain. No live external model or hardware validation is claimed.

Execution: `module_contract::ContractTool` / scoped MemoryTool -> existing ToolDispatcher -> freeze exact effective invocation -> SystemOneGate -> bound version/policy/review-id/action-digest verdict -> audit acknowledgement -> current scope/principal epoch/cancel/deadline/generation revalidation -> per-call receipt -> original native executor. Logical Direct wraps one private executor in one existing catalog entry. Module-owned ALE and authenticated hooks converge at the same dispatcher seam. Pure logical status/orchestration does not acquire an unnecessary model gate; every registered Internal surface does.

Sources: `crates/foundation/plugin/src/system_one.rs`, `module_contract.rs`, `tool_execution.rs`; `crates/engine/runtime/src/canonical/system_one.rs`, `dispatcher/system_one.rs`, `dispatcher.rs`, `module_host.rs`, `accounting.rs`, `runtime.rs`; `crates/engine/runtime-assembly/src/system_one.rs`, `canonical/memory_module.rs`. Canonical concept registration and ADR-0023 retain the existing extension catalog, manager and registry. No baseline/threshold/guard code changes.

The supervisor adapter creates an existing tool-free AgentInstance and executes a private SingleShot task through the same Runtime. The selected provider is pinned by existing instance accounting/routing; fallback is not selected. Unexpected tool calls or nonconforming JSON fail closed. Missing gate/service, Deny, Escalate, invalid protocol/policy version, unrelated review id/digest, timeout, cancellation, revoked authority and audit failure cannot execute the enrolled effect. Escalate is an explicit structured refusal; it does not manufacture approval. Frozen human approval still needs System One and binds the exact effective payload.

FileSystemOneAudit appends, flushes and syncs review facts before allowing an effect. Records contain identities, permission class and digest, not arguments, frozen secrets, prompts or transcripts. Allowed is a review outcome, not native completion. Existing ToolEventV1 / Session facts retain authoritative execution outcomes and accounting. Cancellation before audit acknowledgement may leave an interrupted/pending invocation checkpoint, never a fabricated successful completion or rollback assertion. Explicit tokens, parent scopes and native leases keep their existing ownership/drain semantics.

Evidence command: `cargo test -p apeireth-runtime-assembly --test system_one_execution --test modular_ale --locked -j 2`; raw `logs/p2-targeted-final-fixed.log/.exit`, exit 0, with source manifest. Tests use real Runtime/instances, an actual filesystem effect and actual memory store; supervisor/provider fixtures are scripted and not live model evidence.

| Scenario | Test in `system_one_execution.rs` | Behavior checked |
| --- | --- | --- |
| Allow / audit / accounting | allow_is_audited_before_the_real_effect_and_usage_is_not_duplicated | File effect reads durable Allow record before writing; model adapter uses its actual private instance; usage counted once |
| Deny / Escalate / malformed | deny_escalate_unavailable_and_forged_verdicts_never_reach_the_native_effect | Native entry remains untouched, file absent, review refusal recorded |
| Unavailable / raw API | missing_service_and_raw_invocation_fail_closed_while_conversation_still_runs | Missing gate or unbound service refuses enrolled effect; raw wrapper entries refuse; ordinary conversation continues |
| Timeout / audit outage | supervisor_timeout_and_audit_failure_do_not_default_to_allow | No native entry on supervisor timeout or failed audit acknowledgement |
| Deterministic enforcement | model_allow_cannot_override_deterministic_scope_governance_or_approval | Scope/Governance denies or existing PendingApproval occurs before any review/effect |
| Revocation / cancel | scope_revocation_cancel_and_supervisor_replacement_are_checked_after_await | Awaiting review cannot preserve revoked authority |
| Direct / Agentic convergence | direct_and_owned_agent_memory_writes_share_the_same_mandatory_denial | Same memory action digest, independent real agents/reviews, no DB write, no duplicate usage |
| Model binding | exact_supervisor_model_has_no_fallback_and_cannot_dispatch_its_own_tool_calls | Unselected provider untouched; SingleShot rejects proposed tool calls and malformed decisions |
| Hook / privacy | registered_private_hook_uses_the_gate_and_audit_only_does_not_enter_main_context | Actual private hook effect requires gate; audit-only facts stay out of main messages |
| Approval / frozen action | frozen_approval_still_requires_review_and_binds_the_exact_effective_payload | Before approval no review/effect; after approval exact digest/display reaches supervisor, whose Deny still prevents effect |
| Bounds | review_payload_bound_refuses_before_contacting_a_supervisor | Oversize review refuses before contacting a model |

`modular_ale.rs` additionally covers real Shell execution with one review, organizer/private Session, Accepted/Handle/status and internal-name guessing. Main regression uses the existing `gate_full.ps1 -MainOnly` stages; P4 repeats the original complete five-stage gate. P0/P1 full image-family evidence and historical benchmark/cold-start records are preserved. Final phase exit and source/doc SHAs are appended in the external report after commit.

Migration ledger (open entries block claiming universal/default System One protection):

| ID / owner | Existing boundary and classification | Exit condition |
| --- | --- | --- |
| C01 / capability owners | Default compatibility tools in existing ToolModule/CapabilityProvider and legacy behavior adapters retain original Governance/Dispatcher/native guards; not enrolled in System One | Per-module explicit contract, real deny/unavailable/no-effect and frozen approval/native drain tests; classify every protected helper before default rollout |
| C02 / Host API owners | Existing raw concrete capability handles such as ShellModule.tool and registry Host reads remain trusted in-process Host APIs; new enrolled wrappers reject unscoped effect calls | Migrate consumers to governed registered invocations or retain documented Host-only infrastructure authority; no new feature may use a raw handle to bypass an enrolled seam |
| C03 / runtime Host owners | Background permission is explicit ModuleAgentGrant; it is not a union with caller authority | Message phase must authenticate sender/receiver and propagate delegated scope separately from independent background authority |
| T01 / infrastructure owners | Session/audit persistence, provider transport, lifecycle cleanup and profile stores remain trusted infrastructure under their existing deterministic policies | Preserve cancellation/egress/ownership tests; do not recursively invoke supervisor for its own audit/provider I/O |

No unregistered bypass was added to the enrolled Shell/Memory/hook paths. This is an in-process Host contract, not an untrusted native-code sandbox. The review receipt is a non-serializable trusted Host API; model payloads cannot supply it. The Host owns audit retention/rotation and must provision the supervisor's budget and existing turn/provider capacity; exhaustion fails closed, without increasing limits automatically. Compatibility enrollment/default production composition remains open, so the overall SPEC status is Partial.

Rollback: normal revert P2 source/docs commits, then P1 if desired. Do not rewrite logs, benchmark samples, main or remote history. New contract constructors remain optional, old tool/result wire ABI and defaults are preserved. No Cargo/LOCKED changes or new external dependencies. PR is not opened at this phase; commit SHAs and final PR link are recorded in STATE.

Phase exit: original `pwsh scripts/gate_full.ps1 -MainOnly` passed at 2026-10-30 00:31:02 Asia/Shanghai, `logs/p2-main-gate.exit = 0`; source_unchanged true. Strict canonical architecture, arch-doc, docs links and docs index passed. Full image-family rerun remains P4 closure, not silently counted here. Initial fixture compile errors and pending-approval expectation failure are preserved in earlier raw logs; production approval behavior was not weakened.
