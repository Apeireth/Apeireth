# Cognitive / Guard / Memory 2.x Convergence v3

**Date:** 2026-09-14
**Repository:** `Apeireth/Apeireth` (`Apeireth/apeireth-rust` remote)
**Branch:** `feature/cognitive-memory-convergence-v3`
**Worktree:** `H:\项目\CrossPlatform\Apeireth\apeireth-rust-cognitive-memory-convergence-v3`

This report distinguishes frozen donor evidence, current-main convergence evidence, and post-merge regression evidence. Frozen reports under `reports/` were kept as-is and were not rewritten to pretend they were generated on current main.

## Repository truth

```text
main starting SHA          = 8d225485c58e266ef7bbabe28eafb7cc95d326d6
donor SHA                  = be6afdb5db8e62b2d9caf15a8821f1e1e1754da8
backup freeze SHA          = be6afdb5db8e62b2d9caf15a8821f1e1e1754da8
merge-base                 = 7647d2c91d55901aeae7202f3842b65233bd053c
donor ahead of main        = 51
donor behind main          = 87
cognitive pre-FF SHA       = 9c7ae92757ea8317489964a1324b1ba2120c6365
cognitive post-FF SHA      = be6afdb5db8e62b2d9caf15a8821f1e1e1754da8
convergence merge SHA      = 8005c1f29a5eaa914a1ae72fe563aa4f8e3d9dc8
convergence docs commit    = docs(cognitive): record convergence evidence (this file)
```

`backup/memory-v2.2-final-freeze-20260914` was not modified, force-pushed, rebased, or deleted.

`feature/cognitive-infrastructure-vnext` was fast-forwarded `9c7ae927..be6afdb5` (31 Memory commits; 0 cognitive-only commits).

## Frozen donor evidence

Source: `reports/memory-v2.x-final-production-lifecycle-and-integrity.md` on donor `be6afdb5`.

```text
MEMORY_PRODUCTION_LIFECYCLE_COMPLETE = YES
MEMORY_INTEGRATION_READY = YES
MEMORY_FREEZE_READY = YES
CI_VERIFIED = YES (donor SHA; not reused as convergence proof)
TAURI_PACKAGING = BLOCKED_EXTERNAL_PREREQUISITE
```

Guard donor baselines that this convergence re-ran locally:

```text
guard_v31_correctness     = 20 passed
guard_v32_evaluation      = 9 passed
security_scenarios        = 2 passed
```

## Current-main convergence evidence

Created `feature/cognitive-memory-convergence-v3` from `origin/main` (`8d225485`) and merged `origin/feature/memory-v2.2-production-completion` (`be6afdb5`) with `--no-ff`.

Git recorded 10 content conflicts. Auto-merge handled Cargo.toml, Cargo.lock, runtime execute path, memory crate sources other than migrations, workflows, and most assembly files.

### Conflict map

| file | main semantic | donor semantic | resolution | reason |
|---|---|---|---|---|
| `crates/adapters/cli/src/lib.rs` | Default-on local read tools + DISABLE hatch; shell/fetch approval knobs; `PermissionPresetGovernanceHook` | Guard classifier + dataset recorder; 5-tuple runtime bootstrap with `BehaviorChainGuardHook` | Keep main knobs/preset wrapper and donor Guard hook/dataset path; local-read still uses `local_read_tools_enabled_from_env()` | Product contracts host Guard; Guard stays a governance hook, not the authorization source |
| `crates/adapters/cli/src/gateway_panels.rs` | `credentials: None` on panel services | `safety_guard` + `workbench` ports | Keep all three fields | Gateway must expose both hot-reload credentials and Guard/workbench surfaces |
| `crates/adapters/gateway/src/canonical_entry.rs` | `RequestId` for streaming error-code contract | Guard `IntentInterpreter` + `TurnSecurityContext` | Import both | Preserve streaming IDs and intent-aware turn security |
| `crates/adapters/gateway/src/panels.rs` | `GatewayServices.credentials` | `safety_guard` + `workbench` | Keep all three; `from_panel` sets extras to `None` | Presence of a port remains the capability fact |
| `crates/engine/memory/src/migrations.rs` | Released V9 research execution-state + V10 forgotten artifacts | Experimental V9–V12 Memory 2.2 tables | Keep main V9/V10; append donor SQL as V11–V14 | Main already shipped V9/V10 after shared V8; donor numbering cannot overwrite released schema |
| `crates/engine/runtime-assembly/src/canonical.rs` | `cost_telemetry` module | `guard_observer` module | Load both | Main cost telemetry and Guard dataset observer are independent |
| `crates/engine/runtime-assembly/src/lib.rs` | Preset hook + council/deferred slot exports | Memory typed sink/recall + Guard observer exports | Union re-exports | Assembly remains the composition root for both product and frozen capabilities |
| `frontend/companion-desktop/src/App.svelte` | Session model picker refresh | Guard status refresh | Run both | Desktop must keep P0/P1 model UX and Guard telemetry |
| `frontend/companion-desktop/src/lib/runtime.ts` | Wave-1 error/session/admin types | Guard/workbench types | Import both | Shared client contract |
| `frontend/companion-desktop/src/lib/types.ts` | `ModelInfo` / `ErrorCode` / session/admin patch | Guard status/events/dry-run + workbench turn | Keep both type families | Frontend must speak both contracts |

Post-conflict integration fix (same merge commit): `production_knobs.rs` now destructures the 5-tuple returned by `build_canonical_runtime_with_sessions_from_env()`.

Rustfmt was applied to the merged tree so `cargo fmt --all -- --check` passes.

### Migration map

Shared through V8 (`V8__research_derived_lineage`). Final ordering:

| version | name | origin |
|---:|---|---|
| 1–8 | unchanged shared history | merge-base |
| 9 | `V9__research_execution_state` | current main (kept) |
| 10 | `V10__research_forgotten_artifacts` | current main (kept) |
| 11 | `V11__episode_memory_scope_metadata` | donor experimental V9, renumbered |
| 12 | `V12__episode_metadata_scope_indices` | donor experimental V10, renumbered |
| 13 | `V13__memory_2_2_durable_tables` | donor experimental V11, renumbered |
| 14 | `V14__principal_ownership_and_persona_governance` | donor experimental V12, renumbered |

Existing main databases migrate forward: they already have V9/V10, then receive V11–V14. Donor-only databases that applied experimental V9–V12 under those numbers are not the production user-database authority; current main is.

No duplicate versions, no skipped versions, no downgrade.

## Post-merge regression evidence

Local gates on merge SHA `8005c1f2` (pre-docs commit; docs do not change code):

```text
cargo fmt --all -- --check                         PASS
cargo check --workspace --all-targets --locked     PASS
cargo test --workspace --all-targets --locked      PASS
  suites=119  passed=3386  failed=0  ignored=15
cargo clippy --workspace --all-targets --locked -- -D warnings
                                                   PASS
cargo deny check                                   PASS (existing deny.toml skip/unmatched warnings only)
cargo audit                                        PASS (allowed yanked chacha20 0.10.1 warning, same class as prior trees)
```

Targeted Guard:

```text
apeireth-guard lib unit tests                      30 passed
guard_tests.rs                                     10 passed
guard_v31_correctness                              20 passed
guard_v32_evaluation                               9 passed
security_scenarios                                 2 passed
```

Targeted Memory:

```text
apeireth-memory --tests                            PASS
  lib: 745 passed, 1 ignored
  sqlite / memory_2 / memory_integration / universal_forget / related integration files all passed
memory_provider_e2e                                PASS (file-backed SQLite restart + provider overlay)
memory_typed_lifecycle                             PASS (commitment/persona/temporal restart + identity skip)
universal_forget                                   PASS (Forget > activation/similarity path covered by memory + assembly tests)
ACT-R restart                                      PASS (access-history / typed lifecycle restart proofs)
```

Runtime / product:

```text
canonical_agent_loop                               20 passed
canonical_preference_learning                      14 passed
canonical_organ_module                             12 passed
cognitive_convergence_vertical                     12 passed
cognitive_vnext_production                         2 passed
convergence_production_integration                 5 passed
CLI production knobs / local-read / governance     PASS
gateway admin_config / panel routes                PASS
```

Frontend (`frontend/companion-desktop`):

```text
pnpm test     PASS (7/7 suites)
pnpm check    PASS (0 errors, 5 warnings)
pnpm build    PASS
```

Svelte warning delta vs known previous state (`5 existing Svelte warnings / 0 errors`): **0**. Same five warnings in `MessageContent.svelte` and `SettingsView.svelte`.

### Architecture

```text
RUNTIME_DEPENDENCY_WALL = PASS
```

`cargo tree -p apeireth-runtime --edges normal --depth 4` depends only on:

- `apeireth-core`
- `apeireth-governance`
- `apeireth-orchestration`
- `apeireth-plugin`
- `apeireth-protocol`
- plus ordinary crates (`tokio`, `serde`, `thiserror`, `async-trait`, `parking_lot`, `sha2`)

It does **not** depend on `apeireth-memory`, `apeireth-guard`, `apeireth-runtime-assembly`, `apeireth-storage`, `rusqlite`, `apeireth-tools-canonical`, or `apeireth-organ`.

Guard is composed through `GovernancePipeline` in CLI/assembly. `PermissionPresetGovernanceHook` wraps the inner pipeline: `full` / approval-remember rewrite `RequireApproval` to `Allow` but leave inner `Deny` intact, so Guard denials remain denials. Governance remains authorization authority.

### Packaging

```text
TAURI_PACKAGING = BLOCKED_EXTERNAL_PREREQUISITE
```

`frontend/companion-desktop/src-tauri/tauri.conf.json` still requires `externalBin: ["binaries/apeireth"]`. No sidecar binary was downloaded or fabricated.

## Status

```text
GUARD_RUNTIME_INTEGRATED = YES
GUARD_REGRESSION = PASS
MEMORY_RUNTIME_INTEGRATED = YES
MEMORY_PROVIDER_E2E = PASS
UNIVERSAL_FORGET = PASS
ACTIVATION_RESTART = PASS
MAIN_PRODUCT_REGRESSION = PASS
RUNTIME_DEPENDENCY_WALL = PASS
CI_VERIFIED = NO   # remote CI must run on this branch SHA; donor CI is not reused
TAURI_PACKAGING = BLOCKED_EXTERNAL_PREREQUISITE
```
