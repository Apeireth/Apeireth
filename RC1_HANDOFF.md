# Apeireth 2.0 RC1 Preflight Handoff

**Date:** 2026-08-31
**Purpose:** Continue RC1 packaging and installed-product validation from the isolated candidate worktree.

## Current Candidate

- Primary repository (preserved): `H:\项目\CrossPlatform\Apeireth\apeireth-rust`
- Primary repository HEAD: `8db752d3c1a8e203e90990595593b7c2567441cf`
- RC worktree: `H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1`
- RC branch: `rc/2.0.0-rc1-final`
- RC base: `8db752d3c1a8e203e90990595593b7c2567441cf`
- Current RC commit: `f16e11baba117fa9e10bf25d3d91b7a230498ec3`
- Current RC commit subject: `chore(release): close RC1 packaging and release authority`

The primary checkout was not edited. Its original untracked recovery/audit artifacts remain in place.

The RC worktree currently has generated Tauri schema working-tree drift after a failed Tauri clippy attempt. The schema files have no content diff from the index; do not commit them as product changes. Check with:

```powershell
git status --short
git diff -- frontend/companion-desktop/src-tauri/gen/schemas/desktop-schema.json frontend/companion-desktop/src-tauri/gen/schemas/windows-schema.json
```

## Changes Already Committed On RC Branch

Commit `f16e11b` contains only release/packaging closure work:

- Target-aware, locked canonical CLI build and sidecar staging in `packaging/stage-sidecar.ps1`.
- Desktop staging now requires `pnpm install --frozen-lockfile` and a real frontend build; placeholder HTML fallback removed.
- Desktop staging invokes canonical sidecar staging.
- Desktop MSI/NSIS scripts use fail-fast behavior, clear target-qualified bundle output, invoke real `pnpm tauri build`, collect only target-qualified artifacts, and fail when an installer is absent.
- CLI ZIP/MSI packaging always builds and consumes the target-qualified binary instead of stale shared fallback output.
- MSI install/uninstall helper scripts now pass the requested MSI path to `msiexec` and report the real exit code.
- Scoop packaging requires a locally produced ZIP and no longer downloads a remote release to calculate a hash.
- Homebrew formula points to `v2.0.0-rc.1`; its source archive hash remains deliberately unresolved until a release archive is available.
- PowerShell and shell release-manifest generators derive active component versions instead of hard-coding historical `1.2.0` values.
- Active CI/release-prep workspace-version assertions use the current release authority rather than obsolete `1.2.0` checks.

No Tier 1 read models, UI redesign, direct tool execution path, governed write API, or architecture change was added.

## Validation Completed Against RC Commit

All results below are from the RC worktree and commit `f16e11b`, except where explicitly noted.

### Passed

```text
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
git diff --check

pnpm install --frozen-lockfile       (from frontend/companion-desktop)
pnpm check                           (from frontend/companion-desktop)
pnpm test                            (7 of 7 suites)
pnpm build                           (from frontend/companion-desktop)

cargo check --locked                 (from frontend/companion-desktop/src-tauri)
cargo test --locked                  (from frontend/companion-desktop/src-tauri)
```

The workspace test run included canonical gateway HTTP tests for tool execution, governance denial, provider/tool failure, pending approval, approval resolution/rejection/expiry, and event projection.

### Failed Then Corrected / Pending Rerun

`cargo clippy --all-targets --locked -- -D warnings` in the Tauri shell initially failed because the declared Tauri external binary did not exist:

```text
resource path `binaries\\apeireth-x86_64-pc-windows-msvc.exe` doesn't exist
```

The canonical sidecar has since been built and staged successfully. Rerun Tauri clippy now:

```powershell
cd H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\frontend\companion-desktop\src-tauri
cargo clippy --all-targets --locked -- -D warnings
```

The first frontend attempts were concurrent and hit Windows `EPERM` rename races in `node_modules`. The subsequent sequential frozen install passed.

## Canonical Sidecar Already Built

Source build:

```text
H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\target\x86_64-pc-windows-msvc\release\apeireth.exe
```

Staged Tauri sidecar:

```text
H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\frontend\companion-desktop\src-tauri\binaries\apeireth-x86_64-pc-windows-msvc.exe
```

Observed properties:

- Size: `10,259,456` bytes
- Timestamp: `2026-08-31 18:47:00 +0800`
- `--version`: `apeireth 2.0.0-rc.1`
- Source and staged sidecar sizes match.

Recheck identity before packaging:

```powershell
& H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\target\x86_64-pc-windows-msvc\release\apeireth.exe --version
Get-FileHash H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\target\x86_64-pc-windows-msvc\release\apeireth.exe -Algorithm SHA256
Get-FileHash H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\frontend\companion-desktop\src-tauri\binaries\apeireth-x86_64-pc-windows-msvc.exe -Algorithm SHA256
```

## Next Required Steps

Run these sequentially from the RC worktree.

### 1. Tauri clippy

```powershell
cd H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\frontend\companion-desktop\src-tauri
cargo clippy --all-targets --locked -- -D warnings
```

### 2. Production Tauri build

The tracked packaging scripts are the intended entry points. They rebuild frontend and canonical sidecar and invoke Tauri from the desktop package directory:

```powershell
cd H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1
$env:APEIRETH_VERSION = '2.0.0-rc.1'
$env:APEIRETH_TARGET = 'x86_64-pc-windows-msvc'
.\packaging\desktop\build-desktop-msi.ps1 -Version 2.0.0-rc.1 -Target x86_64-pc-windows-msvc
.\packaging\desktop\build-desktop-nsis.ps1 -Version 2.0.0-rc.1 -Target x86_64-pc-windows-msvc
```

Or run the master desktop flow after confirming the two individual scripts work:

```powershell
.\packaging\desktop\build-desktop.ps1 -Version 2.0.0-rc.1 -Target x86_64-pc-windows-msvc
```

Expected source outputs are under:

```text
frontend/companion-desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi
frontend/companion-desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
```

The scripts copy final artifacts to:

```text
target/desktop-msi/
target/desktop-nsis/
```

Do not accept `[STAGING READY]` as success. The RC scripts now fail if the installer is absent.

### 3. Fresh release manifest

Stage only final artifacts in a clean release directory, then generate metadata from the exact final commit:

```powershell
$finalSha = git rev-parse HEAD
.\scripts\generate-release-manifest.ps1 -DistDir target/rc1-dist -Version 2.0.0-rc.1 -CommitSha $finalSha -ReleaseTag v2.0.0-rc.1
```

Do not overwrite or reuse the old ignored `dist/release-manifest.json`; it was generated at `86495bb9` and contained stale component versions.

### 4. Inspect payload and hashes

For each final MSI and NSIS setup EXE record:

- absolute path
- size
- SHA256
- timestamp
- source commit (`git rev-parse HEAD`)
- version
- signing status, if discoverable

Use MSI administrative extraction to verify the installed payload contains both `companion-desktop.exe` and adjacent `apeireth.exe`. This is only payload proof, not installed GUI E2E.

### 5. Installed human-path E2E

The existing harness is extraction-oriented only:

```powershell
cd H:\项目\CrossPlatform\Apeireth\apeireth-rust-rc1\frontend\companion-desktop
pwsh .\scripts\packaged-sidecar-e2e.ps1 -ProbeGateway
```

It does not prove install, launch, GUI chat, approval UX, restart, close/reopen, or uninstall. Those must be exercised against the actual installed MSI/NSIS product.

Required observations:

- clean install
- installed launch
- bundled backend auto-start
- backend/gateway readiness
- configured-provider chat
- second-turn same-session continuity
- safe tool path
- approval-required path
- approve path
- deny path
- diagnostics view
- log-directory opening
- backend restart
- post-restart chat
- full close and owned backend exit
- reopen and fresh backend startup
- uninstall and residue check

Provider-missing test must distinguish:

```text
Backend Process = Ready
Gateway = Ready
Provider = unavailable/not configured
Chat = unavailable
```

Do not infer provider readiness from `/v1/models` alone.

### 6. Production log check

Inspect:

```text
%LOCALAPPDATA%\\Apeireth\\logs\\apeireth-desktop.log
%LOCALAPPDATA%\\Apeireth\\logs\\apeireth-backend.log
```

Confirm useful timestamps, PID/port, state transitions, request status, failure reason, and exit code. Confirm absence of API keys, Authorization values, master tokens, provider secrets, and full environment dumps.

## Bundle-Only Validation Classification

No bundle was imported into the product repository or RC worktree.

### RC bundles

Already reachable ancestors of current local main:

- `rc_fix.bundle` → `83596798`
- `rc_wave.bundle` → `c9ee7be2`
- `rc_wave2.bundle` → `7a12bff5`

### Canonical-entry validation chain

The chain ending at `91158501d236452d19b6a35b8fb8e402bb164e56` was inspected in a disposable audit repository.

- Canonical primitives/provider bridge/gateway cutover/CLI cutover/failed-turn persistence/real HTTP E2E: equivalent or strengthened in current reorganized code.
- `5711216d`: valuable missing test/CI-only coverage; not an RC-blocking product fix.
- `e24de0d8`: obsolete as a literal package-path patch.
- `91158501`: obsolete as a literal old Kani module-count test; old path no longer exists.
- Post-tip provider/plugin/governance hardening commits: equivalent in current code or test-only; no proven missing product fix.

Conclusion: no bundle commit requires merge/cherry-pick for RC1.

## Disconnected Origin

`origin/main` is `464ef9aae735fce0344ec9754a2a32791d967a2f` with no merge base to the RC lineage.

The origin tip contains a broad MIT OR Apache-2.0 legal migration, new license files, disclaimer, contribution policy, README legal sections, and widespread SPDX/header changes. It was not merged or cherry-picked.

Current RC intentionally remains Apache-2.0 and already has the active Apache license, NOTICE/attribution material, and release packaging references. Treat the origin dual-license migration as a separate legal decision; do not import disconnected history during RC packaging.

## Remote Windows Validator

Target:

```text
desktop-dcce212558a843ed-20260806111728416.tail10d158.ts.net
D:\apx\apeireth-rust
```

SSH access was retried during audit and failed with:

```text
Permission denied (publickey,password,keyboard-interactive)
```

No remote synchronization or mutation was performed. Remote branch, HEAD, status, build, and E2E remain unknown.

## Known Limitations / Release Blockers

1. Final Tauri clippy has not yet been rerun after sidecar staging.
2. Final Tauri MSI/NSIS build has not yet been completed from this RC commit.
3. No installed MSI or NSIS human-path GUI E2E has been proven.
4. No uninstall/residue validation has been proven.
5. No real-provider GUI E2E has been proven.
6. No remote Windows validation has been proven.
7. Homebrew source archive SHA256 remains a placeholder by design until a final source archive exists.
8. Scoop hash remains a placeholder until the final locally produced CLI ZIP is available; the build script now refuses remote fallback.
9. The RC still does not expose Tier 1 MemoryInspection, SessionReadModel, ToolCatalog, or AuditQuery surfaces; this is intentional and must not block this experience-testing RC.
10. `packaging/msi/apeireth.wxs` is a separate legacy CLI MSI identity from the Tauri desktop MSI; do not publish both without an explicit product/upgrade policy.

## Release Boundary

This candidate is:

```text
Apeireth 2.0 RC1
Technical Preview / Experience Testing build
```

It must not be described as v1 semantic parity, Tier 1 complete, or publicly released until the remaining installed-product and Windows validation evidence exists.

## Do Not Do

- Do not edit the primary checkout.
- Do not merge/rebase/reset onto disconnected `origin/main`.
- Do not import or cherry-pick the bundle-only chain without a new product-fix finding.
- Do not use stale `target/release` or existing MSI files as final proof.
- Do not invent Homebrew/Scoop hashes.
- Do not publish, push, tag, create a GitHub release, upload artifacts, or force-push.
- Do not add Tier 1 read models in this RC wave.

## Current Status

```text
RC source closure: PASS
Workspace validation: PASS
Frontend validation: PASS
Tauri check/test: PASS
Tauri clippy: PENDING RERUN AFTER SIDECAR STAGING
Tauri production build: PENDING
Final MSI/NSIS artifacts: PENDING
Installed experience: PENDING
Remote Windows validation: BLOCKED BY SSH AUTHENTICATION
Public release: NO
```
