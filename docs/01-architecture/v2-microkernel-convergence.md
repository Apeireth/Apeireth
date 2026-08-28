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
4. **SubLoop Isolation**:
   - SubLoops execute on private, ephemeral transcripts.
   - SubLoops never mutate the main session.
   - SubLoops never emit direct user frontend events.
   - SubLoops enforce explicit capability allowlists.
   - SubLoops return structured `SubLoopResult` to their calling module.

---

## 3. Component Architecture

```text
┌────────────────────────────────────────────────────────┐
│               Single User-Facing Main Loop             │
│            (Runtime::execute in execute.rs)            │
└──────────────────────────┬─────────────────────────────┘
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
┌─────────────────────────┐ ┌────────────────────────────┐
│   Minimal Microkernel   │ │    Governance Pipeline     │
│ (Sessions, Providers,   │ │ (AllowAll, DenyCapability, │
│  Deterministic Registry)│ │  ContentRisk, Approvals)   │
└────────────┬────────────┘ └────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────┐
│                  Unified Module System                 │
│         (Cognitive Modules + Tool Modules + MCP)       │
├──────────────────────────┬─────────────────────────────┤
│   Cognitive Modules:     │   Tool Modules:             │
│   - MemoryRecall         │   - FilesystemModule        │
│   - PreferenceRecall     │   - SearchModule            │
│   - JudgeModule          │   - RepoModule              │
│   - CouncilModule        │   - ShellModule             │
│   - SelfAssessmentModule │   - FetchModule             │
│   - MemoryWriteback      │   - McpModule (dynamic MCP) │
└────────────┬─────────────┴─────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────┐
│               Bounded Module-Owned SubLoops            │
│    (SubLoopSpawner, Private Transcript, Tool Allowlist)│
└────────────────────────────────────────────────────────┘
```

---

## 4. Verification Matrix

| Area | Invariant | Verified By |
| --- | --- | --- |
| Minimal Kernel | Zero modules boots and completes pure chat turn | `tests/minimal_kernel.rs` |
| Module System | General `Module` trait with hooks & tools | `tests/cognitive_module_abi.rs` |
| Tool Modules | Filesystem, Search, Repo, Shell, Fetch via modules | `tests/canonical_module_tools.rs` |
| MCP Integration | Dynamic MCP capabilities registered via `McpModule` | `tests/canonical_module_tools.rs` |
| SubLoop Bounds | Private transcript, round limits, tool allowlist | `tests/canonical_subloop.rs` |
| Backward Compat | Type aliases (`ProductionCognitiveModules`, `AgentModule`) | Entire test workspace |
