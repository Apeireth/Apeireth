# 0023. Modular ALE contracts and mandatory internal supervision

[Document-Meta]
Document: docs/adr/0023-modular-ale-system-one.md
Version: 0.1
Last-Modified: 2026-10-29
Status: Accepted implementation direction under the owner's SPEC v0.1 task; delivery remains phase-evidenced
Author: Codex

Authority is the owner's Modular ALE Runtime & System One SPEC v0.1 and clarification that a decision/supervisor model adds review while deterministic enforcement remains mandatory. The branch is feat/ale-system-one, based on main at 9ff493ee. This decision extends ADR-0019/0020/0021/0022 without replacing their registries, lifecycle, approval state machine or execution algorithm.

The module manifest's existing metadata carries a typed, versioned ModuleToolContract. CapabilityRegistry derives model/public/internal views from the same active entries. PluginManager rejects malformed or inconsistent declarations before activation. Compatibility remains the constructor default; explicit Host composition selects the new contract. The Direct adapter retains the old tool name, schema, executor and FrozenInvocation. Its logical wrapper and private executor are one catalog entry, rather than duplicate competing registrations. A registered Internal surface is hidden from models and cross-module public discovery.

Memory Organizer remains module-owned AgentInstance execution through Runtime.execute_outcome. The opt-in contract adds an accepted task handle using independent TaskId, PluginId and ModuleInstanceId. Status lookup checks the instance generation and full principal; it projects status only. Old agent_id/namespace/status fields remain for compatibility. Task state is module-local and not durable across unload/restart. AuditOnly observations continue to use the existing hook path; an explicit model call always receives a correlated ToolResult.

System One is a supervisor model service, rather than a new permission authority. The next phase adds a versioned request/verdict and an audit-before-effect gate at the existing dispatcher execution seam. Allow can never override deterministic scope, schema, principal, approval, cancellation, lifecycle or native policy. Deny, Escalate, absent service, invalid response, audit failure and timeout refuse execution. Escalate may return an explicit refusal without manufacturing an approval. The Host must select the supervisor's model/provider; no fallback model is selected by this feature. A supervisor adapter must reuse existing isolated/single-shot AgentInstance execution and accounting.

The typed single-process message transport planned for P3 routes through the same registry, governance and dispatcher, with owner-bound senders and explicit grants. It is not another module/lifecycle authority and does not replace NervousBus. Bounded admission, payloads, deadlines, cancellation, causal hop tracking and local idempotency must be verified before its exit. P4 is a software Mesh proof; no encryption, device, NAT traversal or external network validation can be inferred from loopback tests.

Existing compatibility capability families and trusted infrastructure remain classified in the [P0 effect ledger](../04-internal/ale-system-one-p0-2026-10-29.md). Enrollment requires deny/unavailable/no-effect, frozen approval, owner drain and real Runtime evidence; a compatibility exception is not a claim of System One protection. No new protected feature may invoke a raw native helper to bypass its enrolled gate. Untrusted native code isolation is outside this in-process contract.

Default permissions, environment switches, LOCKED values, dependency edges and historical benchmark/cold-start records stay governed by their existing authorities. No new external dependency or registry is authorized by this decision. Phase-specific ports/concepts are registered in contract.toml when their implementation lands.

Rollback is a normal revert of phase commits in reverse dependency order; no force push or branch protection change. New event variants, if needed by later phases, retain old reader support and must explicitly document forward-reader requirements. No migration rewrites historical records.

O-6: 总体最优—build on current convergence evidence and verify each phase; 拒 a rewrite because it would discard proven ownership and approval behavior. 系统最优—put supervision at the common effect boundary and keep projections private; 拒 granting authority through model verdicts or message delivery. 架构最优—add typed metadata and bound ports over existing authorities; 拒 a second registry, lifecycle manager or Provider loop.
