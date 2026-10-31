# ALE / System One P0 baseline and gap ledger

```text
[Document-Meta]
Document: docs/04-internal/ale-system-one-p0-2026-10-29.md
Version: 0.1
Last-Modified: 2026-10-29
Status: P0 PASS; target feature pending P1–P4
Author: Codex
```

Baseline: `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc`, branch `feat/ale-system-one`, fetched origin/main and feature at the same SHA. No production behavior changes in P0. Authority: external SPEC v0.1 and user clarification (supervisor model adds review; deterministic enforcement stays mandatory). Source acquisition: repository only; no third-party code or model substitution.

| Plane | Existing execution evidence | Level / default / hardware | Gap and next exit |
| --- | --- | --- | --- |
| Main ALE | `crates/engine/runtime/src/canonical/execute.rs`, `instances.rs`, `instances/tasks.rs` | PRODUCTION WIRED; managed desktop default follows existing claims; hardware untested here | Reuse execute_outcome; no copied Provider loop |
| Catalog / module lifecycle | `crates/foundation/plugin/src/manager.rs`, `manifest.rs`; `crates/engine/runtime/src/canonical/capability.rs`, `module_host.rs` | PRODUCTION WIRED; no implicit tool permission grant | Typed Public/Internal contract missing; extend manifest metadata and existing views, no rival registry |
| Direct protected execution | `crates/capabilities/tools/src/shell.rs`, `shell/execution.rs`, `process.rs`; `crates/engine/runtime/src/canonical/dispatcher.rs` | Existing dispatcher and native scope PRODUCTION WIRED; shell default permissions unchanged | No System One; enroll thin Direct contract, preserve frozen invocation and deterministic guards |
| Other effects | Filesystem/search/repo; file/git/worktree write, network fetch/MCP/device/credential tool families in `crates/capabilities/tools/src`; Memory DB workers in `crates/engine/runtime-assembly/src/canonical/memory_module/execution.rs` | Existing capabilities vary per claims and production.rs; do not label all default enabled | Compatibility native handles/legacy tools require explicit migration ledger with exit conditions |
| Hooks | `crates/engine/runtime/src/canonical/registered_hooks.rs`, `behavior_adapter.rs`; fixed subscriptions and authenticated receipt | PRODUCTION WIRED; automatic calls share governance/frozen approval/dispatcher | New private tools must reject model guessing and unrelated module invocation |
| Module-owned ALE | `crates/foundation/plugin/src/agents.rs`; Memory `organizer.rs`; existing AgentRuntime | IMPLEMENTED Host composition; no external model call in this batch | Private independent Session exists; typed Accepted/TaskHandle and status query missing |
| Messages | `crates/engine/runtime-assembly/src/canonical/nervous_bus.rs`, ToolEventV1/SessionEvent in protocol/runtime | Domain bus and tool facts PRODUCTION WIRED; generic module pipeline NOT IMPLEMENTED | Typed/authenticated bounded Command/Query/Event/Reply, cancellation, loops, idempotency needed |
| Projection | `registered_hooks.rs`, `ModuleResultDelivery` in plugin/tool_execution.rs; Memory output whitelist | Provider vs AuditOnly IMPLEMENTED / existing registered behavior path PRODUCTION WIRED | Explicit task completion contract and private/public views needed |
| System One | Existing governance in `dispatch_admission.rs`, scope/schema/deadline/native lease in dispatcher; ObservedGate in tools | Existing deterministic enforcement PRODUCTION WIRED; System One NOT IMPLEMENTED, not default enabled, hardware untested | Mandatory shared execution seam; unavailable/malformed/timeout/deny/escalate cannot execute; model cannot grant authority |
| Mesh | `crates/foundation/protocol/src/p2p_mesh.rs`; active tree has no registered Mesh runtime module | Protocol data IMPLEMENTED; target Mesh module NOT IMPLEMENTED, not default enabled | Software-only opt-in vertical proof; do not represent archived/native network features as wired |

Raw inventory: `/workspace/apeireth-codex/ale-s1/logs/p0-inventory-{tools,effects,hooks,agents,projection}.log`. The effect grep is candidate discovery, not a proof that arbitrary Rust code is sandboxed. `p0-freeze.json` hashes Cargo/contract/claims/wiring/test-baseline. Existing historical benchmarks, S6 cold-start samples and LOCKED files are unchanged.

Trust classification / migration gate:

- Registered protected tool execution: target mandatory System One plus unchanged deterministic enforcement.
- Pure computation and runtime controls: no model supervision of arithmetic/state bookkeeping.
- Trusted infrastructure: session/audit persistence, Provider transport, module lifecycle cleanup and profile store remain their existing host-authorized surfaces; no new user authority.
- Compatibility Host raw capability handles and existing legacy capability families: not enrolled yet; cannot be claimed System One protected. Enroll per module only after real effect deny/unavailable/frozen approval/drain tests. No new feature may use a raw handle as a protected-effect bypass.

P0 regression exit: PASS, original five-stage gate_full (same test/thresholds; jobs=2, dev/test debug info=0 and incremental=0 to fit disk, assertions preserved), strict architecture and docs truth. Raw logs and actual exits under `/workspace/apeireth-codex/ale-s1/logs/`; toolchain Rust/Cargo 1.97.1, Linux x86_64. The full command includes existing S6/KC permission, provider, frozen approval, private Session, native drain and old reader tests; no ignore or test deletion.

Rollback: revert only this documentation commit; no runtime behavior to roll back. Residual debt: all feature gaps above remain pending P1–P4. No PR at P0; phase commit recorded in external STATE after commit (no self-referential SHA).

Baseline execution notes (2026-10-29): `p0-full` failed because the cached guard evaluation binary embedded a removed worktree manifest path. `p0-full-retry` passed main test/clippy/fmt, then the cached image profile regeneration check failed for the same reason. Both original tracked data files exist; no artifact was regenerated. The script's broad OOM attribution was false for these NotFound failures (D33). All files referencing compile-time manifest paths were touched to force current-checkout compilation, with zero content changes. Final full retry: `p0-full-current-paths` / `p0-gate-full-current-paths.log`; its actual final exit, rather than intermediate output, governs acceptance. Workspace doc tests: `p0-doc-tests` exit 0. Strict architecture, docs truth with block-stale, wiring, arch-doc, docs index and links passed (corresponding logs/exits). No test was deleted, ignored, weakened or relabeled.

Final exit evidence: `logs/p0-full-current-paths.exit = 0`; original five stages passed at 2026-10-29 23:26:51 Asia/Shanghai. Source content hashes match the P0 freeze. Baseline Rust/Cargo and benchmark content changes: none. P0 feature classification remains NOT IMPLEMENTED; this is baseline acceptance, not target feature acceptance.
