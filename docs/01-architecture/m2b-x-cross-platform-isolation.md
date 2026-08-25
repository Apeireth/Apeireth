# M2B-X — Cross-Platform Process Isolation Foundation

Status: complete (Windows tested locally; Linux/macOS CI-validated per matrix)
Branch: `reconstruct_v2`

## 1. Canonical contract

Public layer (`crates/apeireth-tools-canonical/src/process/mod.rs`) only
expresses platform-agnostic types:

- `ProcessRequest` — structured executable + args + explicit cwd + environment policy + limits + isolation requirement
- `ProcessLimits` — timeout, stdout/stderr bounds, optional memory / process-count / CPU / file-size limits
- `IsolationProfile` — `Trusted`, `Restricted`, `Untrusted` presets
- `IsolationRequirement` — caller-declared minimum `(IsolationCapability, EnforcementLevel)` pairs
- `IsolationCapabilities` — observable platform capability set
- `ProcessResult` — termination, bounded stdout/stderr, truncation flags, `enforcement`
- `PlatformEnforcement` — platform kind + capabilities actually in effect for the run
- `ProcessError` — including `IsolationRequirementUnsatisfied` and `UnsupportedLimit`

Backend modules (`windows`, `linux`, `macos`) contain all platform mechanisms.
No JobObject / RestrictedToken / CreateProcessW / cgroup / seccomp / Landlock /
namespace / setrlimit / Seatbelt / sandbox-exec name appears in the public
contract source.

## 2. Backend model

```text
ProcessExecutor
    |
    +-- WindowsProcessBackend   (windows.rs, cfg(windows))
    |
    +-- LinuxProcessBackend     (linux.rs, cfg(target_os = "linux"))
    |
    +-- MacOsProcessBackend     (macos.rs, cfg(target_os = "macos"))
```

No `PlatformManager`, `SandboxManager`, `BackendRegistry`, or `SecurityFacade`
was created. Static `cfg` dispatch is used.

## 3. Enforcement levels

`Unsupported < Partial < Enforced`.

A requirement `(capability, required_level)` is satisfied only when the
platform's actual level for that capability is `>= required_level`. Missing
requirements fail before any child is spawned, with
`ProcessError::IsolationRequirementUnsatisfied`.

## 4. Platform matrix

| Capability | Windows | Linux | macOS | Evidence |
| --- | --- | --- | --- | --- |
| Structured spawn | ENFORCED | ENFORCED | ENFORCED | `structured_args_are_preserved_with_unicode_and_spaces` (all platforms) |
| Explicit cwd | ENFORCED | ENFORCED | ENFORCED | `working_directory_is_explicit_not_ambient` (all platforms) |
| Timeout | ENFORCED | ENFORCED | ENFORCED | `timeout_terminates_the_child` (all platforms) |
| stdout/stderr bound | ENFORCED | ENFORCED | ENFORCED | `stdout_limit_truncates_and_reports`, `stderr_limit_truncates_and_reports` |
| Environment isolation | ENFORCED | ENFORCED | ENFORCED | `environment_clearing_denies_ambient_secrets` |
| Process-tree containment | ENFORCED (Job Object) | PARTIAL (process group; descendants that create a new group/session can escape) | PARTIAL (same) | Windows: `timeout_terminates_the_whole_job_tree`; Linux/macOS: `timeout_terminates_the_whole_process_group_tree` |
| Memory limit | ENFORCED when configured (Job Object) | PARTIAL (`RLIMIT_AS`) | PARTIAL (`RLIMIT_AS`) | Windows: `process_memory_limit_rejects_oversized_allocation`; Unix: setrlimit before exec |
| Process-count limit | ENFORCED when configured (Job Object) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) | Windows: `active_process_limit_blocks_extra_child_creation`; Unix tests deliberately avoid global UID dependence |
| CPU limit | UNSUPPORTED | ENFORCED (`RLIMIT_CPU`) | ENFORCED (`RLIMIT_CPU`) | Unix: pre-exec setrlimit |
| File-size limit | UNSUPPORTED | ENFORCED (`RLIMIT_FSIZE`) | ENFORCED (`RLIMIT_FSIZE`) | Unix: pre-exec setrlimit |
| Privilege reduction | ENFORCED when restricted-token launch is possible; otherwise UNSUPPORTED | PARTIAL (`PR_SET_NO_NEW_PRIVS`) | UNSUPPORTED | Windows: `privilege_reduction_requirement_is_enforced_or_fails_closed`; Linux: `privilege_reduction_requirement_sets_no_new_privs`; macOS: `privilege_reduction_is_honestly_unsupported` |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | `current_platform_capabilities` runtime detection |
| Network isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | `unsupported_network_requirement_fails_closed_before_child_starts` |
| Fail-closed pre-exec containment | ENFORCED (CREATE_SUSPENDED -> assign -> resume) | ENFORCED (pre_exec setup before exec) | ENFORCED (pre_exec setup before exec) | spawn path in each backend; fail-closed tests use a marker file to prove the child never started |

## 5. Profiles

- `Trusted`: structured spawn, explicit cwd, timeout, stdout/stderr bounds.
- `Restricted`: Trusted + environment isolation, process-tree containment
  PARTIAL, memory/process-count/CPU/file-size PARTIAL, privilege reduction
  PARTIAL, fail-closed pre-exec containment ENFORCED.
- `Untrusted`: Restricted + privilege reduction ENFORCED + filesystem
  isolation ENFORCED + network isolation ENFORCED. On current backends this
  profile is unsatisfiable; that is intentional and fail-closed.

## 6. Fail-closed rules

- Requirement check happens before child spawn.
- Optional limits that a platform cannot enforce are rejected with
  `ProcessError::UnsupportedLimit` before child spawn.
- There is no fallback to a weaker mode when a requirement is not met.
- Security-sensitive profiles never run in a degraded mode silently.

## 7. Security claims

- Filesystem isolation: `UNSUPPORTED` on all three platforms in this phase.
- Network isolation: `UNSUPPORTED` on all three platforms in this phase.
- Privilege isolation:
  - Windows: restricted-token child launch when the account holds the needed
    privilege; otherwise `UNSUPPORTED` (ordinary non-admin users typically see
    `UNSUPPORTED`).
  - Linux: `PR_SET_NO_NEW_PRIVS` only; advertised as PARTIAL and explicitly
    not called a seccomp or namespace sandbox.
  - macOS: `UNSUPPORTED`.

## 8. Remaining gaps

- Linux/macOS process-tree containment is process-group based; descendants
  that create their own process group/session can escape.
- No Linux cgroup v2 / Landlock / namespace / seccomp enforcement in this
  phase. Linux cgroup v2 and Landlock remain future candidates with
  runtime-detect capability when implemented.
- macOS has no supported filesystem/network sandbox mechanism for arbitrary
  subprocesses; no private API is used.
- Windows filesystem/network isolation is not implemented.
- Arbitrary shell is not ready on any platform until egress and filesystem
  isolation requirements are settled (M2D+).
