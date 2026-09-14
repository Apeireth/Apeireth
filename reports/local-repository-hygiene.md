# Local Repository Hygiene

**Assessment date:** 2026-09-08  
**Repository:** `apeireth-rust`  
**Branch:** `feature/cognitive-infrastructure-vnext`  
**Recorded HEAD:** `9289f4f0c0e729f4be62500403bdab65b7d75c4b`

## Boundary and preservation policy

Work remained in `H:/项目/CrossPlatform/Apeireth/apeireth-rust`. No Guard branch was created, `main` was not modified or merged, no PR was created, and nothing was pushed. Destructive cleanup commands (`git clean -fdx`, `rm -rf *`, and `git reset --hard`) were not used.

Uncertain user files, patches, bundles, datasets, reports, artifacts, local configuration, credentials, and secrets were preserved. Only Python bytecode caches created by local compilation were removed after inventory; they are safe-regenerable and are not evidence.

## Inventory classification

### KEEP

- All tracked source, test, workflow, artifact, report, dataset, and configuration files.
- Modified Guard source and tests under `crates/engine/guard/`.
- Generated evaluation exports under `scripts/guard_ml/`.
- Model, metrics, and manifest artifacts under `artifacts/`.
- Existing reports and any user-authored files not created by this work.
- Other worktrees and branches listed by the repository inventory.

### REVIEW_REQUIRED

- New `.github/workflows/guard-v3.3.yml` until reviewed and committed intentionally.
- New action/trace/pair exports until their provenance is accepted by the repository owner.
- Any unrelated modified or untracked files discovered after this report.
- Existing local configuration, credentials, patches, bundles, or datasets whose ownership is not proven.

### SAFE_REGENERABLE

- `scripts/guard_ml/__pycache__/` and `.pyc` files created by `py_compile` or script execution. These were removed conservatively; no source or evidence file was removed.
- Ignored Rust and frontend build outputs (`target/` and `frontend/companion-desktop/src-tauri/target/`). These were not deleted.

## Dry-run cleanup result

The only cleanup performed was removal of known Python bytecode files under `scripts/guard_ml/__pycache__/` after confirming their names and origin. No broad cleanup was run. Ignored build directories remain intact.

## CI hygiene

The Guard 3.3 workflow now removes only the known `__pycache__` directories immediately after Python compilation, before its untracked-file hygiene gate. It does not skip tests, soften failures, or use `continue-on-error`.

## Status

Repository hygiene is locally documented, but final readiness remains blocked by the absence of a remote CI run for the final SHA and the absence of real shadow evidence.
