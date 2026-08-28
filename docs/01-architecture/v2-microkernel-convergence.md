# Apeireth v2 Microkernel Architecture Convergence

## 1. Executive Summary

This architecture document records the finalized microkernel architecture for Apeireth v2.
The runtime has converged into its canonical shape:
- **Single User-Facing Main Loop**: `Runtime::execute` in `apeireth-runtime::canonical::execute` is the only user-facing loop.
- **Minimal Microkernel**: Pure session management, provider routing, module lifecycle dispatch, and governance integration. Boots with zero modules and serves plain chat turns.
- **Unified Module System**: `Module` trait (`manifest()`, `on_hook()`, `tools()`) and deterministic `ModuleRegistry`.
- **Module-Provided Tools & MCP**: All builtin tools (Filesystem, Search, Repo, Shell, Fetch) and dynamic MCP servers are provided through modules.
- **Bounded Module-Owned SubLoops**: `SubLoopSpec`, `SubLoopResult`, `SubLoopSpawner` executing on private ephemeral transcripts with strict capability allowlists and timeout/round bounds.
- **Single Canonical Capability/Governance/Provider Path**: Full integration without parallel paths or legacy bridges.

---

## 2. Microkernel Invariants

1. **Microkernel Purity**:
   - `Runtime` does not hardcode vendor identities, cognitive features, or concrete tool names.
   - Booting without standard modules produces a functioning microkernel that executes pure conversational turns.
2. **Single User-Facing Main Loop**:
   - Only the Main Loop mutates the persistent user session and emits user-facing responses/events.
3. **Module Ownership of Capabilities**:
   - Every cognitive component (Memory, Preference, Judge, Council, SelfAssessment, Experience) implements `Module`.
   - Every tool capability (Filesystem, Search, Repo, Shell, Fetch, MCP) is registered through a `Module`.
   - `McpModule` manages dynamic post-build tool registration via `RwLock` without creating a secondary registry.
4. **Governed SubLoop Isolation & Dispatch**:
   - SubLoops execute on private, ephemeral transcripts.
   - SubLoops never mutate the main session and never emit direct user frontend events.
   - All SubLoop tool calls flow through the **canonical capability governance path**:
     $$\text{effective permission} = \text{global governance} \land \text{subloop allowlist}$$
   - Global governance is always authoritative; allowlists can never relax a global deny.
   - Capabilities requiring interactive human approval fail cleanly inside SubLoops with structured errors.
   - SubLoop execution is bounded by explicit round limits (`max_rounds`), overall execution timeouts (`timeout`), shared turn invocation budgets (`ModuleTurnState`), and recursion depth guards (`DEFAULT_MAX_INVOCATION_DEPTH`).

---

## 3. Component Architecture

```text
+--------------------------------------------------------+
|               Single User-Facing Main Loop             |
|            (Runtime::execute in execute.rs)            |
+--------------------------+-----------------------------+
                           |
             +-------------+-------------+
             |                           |
             v                           v
+-------------------------+ +----------------------------+
|   Minimal Microkernel   | |    Governance Pipeline     |
| (Sessions, Providers,   | | (AllowAll, DenyCapability, |
|  Deterministic Registry)| |  ContentRisk, Approvals)   |
+------------+------------+ +-------------+--------------+
             |                            |
             |           +----------------+ (Authoritative Policy)
             v           v
+--------------------------------------------------------+
|                  Unified Module System                 |
|         (Cognitive Modules + Tool Modules + MCP)       |
+--------------------------+-----------------------------+
|   Cognitive Modules:     |   Tool Modules:             |
|   - MemoryRecall         |   - FilesystemModule        |
|   - PreferenceRecall     |   - SearchModule            |
|   - JudgeModule          |   - RepoModule              |
|   - CouncilModule        |   - ShellModule             |
|   - SelfAssessmentModule |   - FetchModule             |
|   - MemoryWriteback      |   - McpModule (dynamic MCP) |
+------------+-------------+-----------------------------+
             |
             v
+--------------------------------------------------------+
|               Bounded Module-Owned SubLoops            |
|  (SubLoopSpawner, Private Transcript, Governed Dispatch)|
+--------------------------------------------------------+
```

---

## 4. Verification Matrix

| Area | Invariant | Verified By |
| --- | --- | --- |
| Minimal Kernel | Zero modules boots and completes pure chat turn | `tests/minimal_kernel.rs` |
| Module System | General `Module` trait with hooks & tools | `tests/cognitive_module_abi.rs` |
| Tool Modules | Filesystem, Search, Repo, Shell, Fetch via modules | `tests/canonical_module_tools.rs` |
| MCP Integration | Dynamic MCP capabilities registered via `McpModule` | `tests/canonical_module_tools.rs` |
| SubLoop Governance | SubLoop tool calls evaluated against global governance | `tests/canonical_subloop.rs` |
| Hostile Capability | Denied tool call is blocked; tool invoke counter is 0 | `tests/canonical_subloop.rs` |
| SubLoop Timeout | Overall deadline aborts slow SubLoop; session intact | `tests/canonical_subloop.rs` |
| Backward Compat | Type aliases (`ProductionCognitiveModules`, `AgentModule`) | Entire test workspace |
