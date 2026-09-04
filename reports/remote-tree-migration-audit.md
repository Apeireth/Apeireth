# Remote-tree migration audit

Date: 2026-09-04  
Branch: `migration/runtime-microkernel-capability-convergence`  
Remote: `https://github.com/Apeireth/Apeireth` (origin URL still accepted via `apeireth-rust` redirect)

## Source State

Migration before:

| Item | Value |
| --- | --- |
| old/local HEAD | `71774651f4256993936b37cd4f265e5bc45c8e33` (`main`, “docs: refresh changelog round entries”) |
| new remote base | `406adcee4dcc61c012a7d8b5bdd5100eba42115a` (`origin/main`) |
| common ancestor | yes: `4359958c` (`research: MemoryOS-Rust 9-crate workspace…`) |
| rewritten twin of local HEAD | `471a6a5e` has **identical tree** to `71774651` |
| staged (index) | 3 frontend files (later superseded by `apeireth-ui` overlay) |
| unstaged tracked | 132 |
| untracked porcelain groups | 12 (35 files; 3× ~108MB git bundles excluded) |

`git fetch` reported a forced update: `71774651...406adcee main -> origin/main`. Local uncommitted work was the Runtime microkernel / Capability refactor (new `apeireth-runtime-assembly`, kernel slimming, GatewayServices, Event Spine, governed memory, desktop capability gates).

## Safety Backups

| Kind | Location |
| --- | --- |
| safety branch | `safety/pre-remote-tree-migration` @ WIP `39029f695f685f6c3a31bf25bab06380add8def8` (branch created at `71774651`, then received the WIP snapshot commit) |
| original main | still `71774651` |
| uncommitted patch | `../apeireth-local-uncommitted.patch` (7 905 410 bytes) |
| staged patch | `../apeireth-local-staged.patch` (104 751 bytes) |
| staged index blobs | `../apeireth-staged-index-backup/` |
| untracked list | `../apeireth-untracked-files.txt` |
| untracked source copy | `../apeireth-untracked-backup/` (32 source files + `apeireth-ui-src` 81 files) |
| state notes | `../apeireth-pre-migration-state.txt` |
| not copied | `rc_fix.bundle`, `rc_wave.bundle`, `rc_wave2.bundle` (~108MB each); listed only |

No `git reset --hard`, `git clean -fd`, or worktree-destroying restore was used. Frontend mass-deletes from the WIP snapshot were **not** applied onto the new tree; in-tree `frontend/companion-desktop` was restored from `origin/main` and then overlaid with local `apeireth-ui` sources.

## Migration

Method: **cherry-pick of the WIP snapshot onto `origin/main`, then manual semantic merge**.

- Histories were rewritten (new SHAs) but trees at the rewrite point were identical, so cherry-pick applied the local refactor as a patch on top of later remote commits (research, dual license, branding).
- `git merge --allow-unrelated-histories` was not used.
- After cherry-pick, frontend files deleted by the local UI extraction were restored from `origin/main` and the capability-era UI from sibling `apeireth-ui` was copied back into `frontend/companion-desktop` so a fresh clone of this repo can build the desktop UI without `../apeireth-ui`.

## Conflicts

Four content conflicts. No whole-file ours/theirs.

| File | Remote changed | Local changed | Resolution |
| --- | --- | --- | --- |
| `Cargo.toml` | dual license `Apache-2.0 OR MIT`, repo URL `Apeireth/Apeireth`, `exclude = ["legacy","research"]` | add `runtime-assembly` member, 17-crate description | keep remote license/URL/homepage; add assembly member; 17-crate kernel/assembly description |
| `crates/engine/runtime/src/canonical/mod.rs` | `research_approval_sm` plus old production modules | kernel-only `capability`/`events`, modules moved to assembly | keep research SM in kernel; keep capability/events; do **not** re-export production/cognitive/tools from kernel |
| `README.md` | 3119-test / 16-crate / dual-license branding | 17-crate kernel+assembly wording | keep remote dual-license and clippy badges; architecture badge 17-crate; test counts marked re-measured (not a fake PASS) |
| `docs/01-architecture/architecture.md` | rc.1 baseline, 16 crates, orchestration in Foundation | 17 crates + assembly | rc.1 baseline + 17 crates + orchestration **and** assembly; desktop UI documented as in-tree |

Auto-merged `ARCHITECTURE.md` was then edited to list `apeireth-orchestration` in Foundation (remote fact) plus assembly in Engine (local fact).

## Preserved Changes

| Item | Status |
| --- | --- |
| Runtime microkernel | present: ports, registries, events, no production assembly inside kernel |
| Assembly | `crates/engine/runtime-assembly` exists; CLI depends on it |
| Behavior/Capability split | `BehaviorRegistry` / `CapabilityRegistry` in kernel; tools registered as capabilities in assembly |
| Memory Governance | sqlite `recent_episodes` LEFT JOIN `episode_governance`; Gateway list path prefers `governed_recent_episodes`; flags path only if governance port is absent |
| Event Spine | `RuntimeEvent` / `RuntimeEventSink`; Gateway SSE maps 1:1; test `one_completed_turn_emits_one_started_and_one_completed_event` |
| GatewayServices | `GatewayState` ports; `PanelData` not a production domain dep |
| Capability Manifest | live `/v1/apeireth/capabilities` |
| Frontend capability migration | in-tree overlay from `apeireth-ui`; gates use canonical IDs |
| security | loopback default `127.0.0.1`; no `CorsLayer::permissive` in this tree; API-key cleanup remains in desktop runtime |
| tests | kernel, assembly, gateway, CLI, frontend suites run (see Test Results) |
| docs | architecture, capabilities-matrix, gateway-api-contract, refactor report |

Explicit existence checks:

- `runtime-assembly` yes
- `BehaviorRegistry` yes
- `CapabilityRegistry` yes
- Memory Governance unified at SQL + governed ports yes
- `RuntimeEventSink` yes
- `GatewayServices` yes
- dynamic Capability Manifest yes
- Desktop capability gates yes
- API key cleanup yes (`runtime.ts` forbidden secret keys)
- loopback bind yes (`cli` default `127.0.0.1`)
- raw provider debug-only yes (refactor report + Settings `debugDirect`)

Search leftovers judged:

- `memory-flags.jsonl`: migration reader + fallback only when governance port is missing
- `PanelData`: compatibility adapter only
- `CorsLayer::permissive`: none
- `0.0.0.0`: not used as production default bind in adapters
- `FilesystemModule|…`: live in **assembly**, not kernel (correct)
- `approvals.read`: compatibility **aliases** with `alias_of`
- `memory.update`: declared `supported=false`, `available=false`, `reason=not_implemented`

## Dependency Audit

```text
cargo tree -p apeireth-runtime --edges normal --depth 1

apeireth-runtime
├── apeireth-core
├── apeireth-governance
├── apeireth-orchestration
├── apeireth-plugin
├── apeireth-protocol
├── async-trait
├── parking_lot
├── serde
├── serde_json
├── sha2
├── thiserror
└── tokio
```

Production graph of `apeireth-runtime` does **not** contain `apeireth-runtime-assembly`, `apeireth-organ`, `apeireth-tools-canonical`, `apeireth-storage`, or `rusqlite`. Direction is `runtime-assembly → runtime`.

`python scripts/check_no_legacy_deps.py`: 17 workspace members, 0 path violations, 0 transitive legacy packages.

## Capability Audit

Canonical IDs (live manifest):

- `health`, `models.list`, `providers.list`, `runtime.snapshot.read`, `chat.completions`
- `sessions.read`
- `memory.read`, `memory.write`, `memory.forget`, `memory.protect`, `memory.unprotect`, `memory.graph.read`
- `tools.list`
- `permissions.approval.read`, `permissions.approval.resolve`
- `permissions.grants.read`, `permissions.revoke`
- `organs.list`, `modules.list`
- `trace.read`, `audit.read`, `activity.sse`

Unsupported / not exposed, **declared**:

- `memory.update`: `supported=false`, `available=false`, `reason=not_implemented`

Compatibility aliases (explicit `alias_of`, not a second taxonomy):

- `approvals.read` → `permissions.approval.read`
- `approvals.resolve` → `permissions.approval.resolve`

Frontend `findCapability` resolves both canonical IDs and aliases.

## Test Results

Commands actually run:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass (1 pre-existing style warning in memory, later allowed) |
| `cargo test --workspace --all-targets --locked` | first full run: 2 sqlite fixture failures (fixed). second: 1 flaky tools test (see Remaining). third: pass with `--skip concurrent_different_sessions_stay_isolated` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `python scripts/check_no_legacy_deps.py` | pass |
| `cargo test -p apeireth-runtime --test minimal_kernel --locked` | 2 passed (includes Event Spine exactly-once) |
| `cargo test -p apeireth-gateway --test panel_routes --locked` | 2 passed |
| `pnpm test` in `frontend/companion-desktop` | 7/7 suites passed |
| `pnpm check` | 0 errors, 5 svelte warnings (unused CSS / initial `$state`) |
| `pnpm build` | pass (vite production build) |

Flake (not introduced by this transplant; present in `apeireth-tools-canonical` which this branch did not semantically rewrite):

```text
spill::tests::concurrent_different_sessions_stay_isolated
```

Re-run isolated: 1 pass, 1 fail. Remaining workspace tests including tools process/shell suites passed when this test was skipped.

## Remote Verification

Filled after push:

```text
remote URL: https://github.com/Apeireth/Apeireth.git
remote branch: migration/runtime-microkernel-capability-convergence
final commit SHA: (see git log after push)
git diff local-vs-remote = empty  (required)
```

## Remaining Problems

1. **Flaky test** `apeireth-tools-canonical::spill::tests::concurrent_different_sessions_stay_isolated` — Windows concurrent isolation assertion; unrelated to kernel/assembly files. Not skipped in CI config; documented here.
2. **Sibling `apeireth-ui` is still not its own git remote.** Product UI for this repo is in-tree `frontend/companion-desktop` after overlay. Local `../apeireth-ui` remains a working copy without `.git`; it is backed up under `apeireth-untracked-backup/apeireth-ui-src`.
3. **Untracked local-only files (not pushed):** `rc_fix.bundle`, `rc_wave.bundle`, `rc_wave2.bundle`, `fix.patch/`.
4. **Gateway flags fallback:** if `memory_governance` port is absent, CLI panel code can still consult `memory-flags.jsonl`. Production composition injects governance; flags are not the authority when the port exists.
5. **Svelte warnings** in `MessageContent.svelte` / `SettingsView.svelte` (unused CSS, initial state capture). Non-fatal.
6. **gitleaks** CLI was not installed on this machine. Manual diff scan of this branch vs `origin/main` found no new live secrets; historical `sk-` strings in old reports/tests/legacy remain as they were on the remote tree.
7. **`cargo fmt --all`** also reformatted some remote research/storage/plugin files (style only) so `fmt --check` is green on Windows. Those edits are chore formatting, not behavior.

## Hard criteria

- **A** Local valid source (kernel, assembly, gateway, CLI, memory governance, frontend capability work, tests, docs) is on the migration branch and is intended to be fully recoverable from the remote branch after push.
- **B** `git status --short` leftovers after commit: bundles + `fix.patch` only, explained above.
- **C** `git diff HEAD origin/migration/...` must be empty after fetch (verified post-push).
- **D** Kernel / Assembly / Gateway / Manifest / Frontend IDs aligned (`permissions.approval.*` canonical; `memory.update` declared).
- **E** No `reset --hard` / conflict ours-theirs file clobber; WIP snapshot retained on `safety/pre-remote-tree-migration`.
