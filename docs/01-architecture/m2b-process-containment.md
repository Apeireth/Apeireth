# M2B — Canonical Process Containment Foundation

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。

Status: complete
Branch: `reconstruct_v2`
Starting HEAD: `3ec66cd2c714178a0ce42e3218d28afdb247b22e`
Donor: `origin/master:reconstruction_v2/crates/apeireth-tools/src/sandbox.rs` and siblings

## 1. Donor reality audit

Donor sources inspected:

- `origin/master:reconstruction_v2/crates/apeireth-tools/src/sandbox.rs`
- `origin/master:reconstruction_v2/crates/apeireth-tools/src/builtin/shell.rs`
- `origin/master:reconstruction_v2/crates/apeireth-tools/src/builtin/repo_tools.rs`
- `origin/master:reconstruction_v2/crates/apeireth-tools/src/synthesis.rs`
- `origin/master:reconstruction_v2/crates/apeireth-tools/src/worktree.rs`
- `origin/master:reconstruction_v2/crates/apeireth-runtime/src/host.rs`

| Feature | Donor status | Canonical strategy | Final status | Evidence |
| --- | --- | --- | --- | --- |
| Job Object creation | REAL | REIMPLEMENT low-level primitive | ENFORCED | `process/windows.rs::JobObject::create` uses `CreateJobObjectW` + `SetInformationJobObject` |
| Child attachment | BROKEN / NOT INTEGRATED | REIMPLEMENT fail-closed spawn | ENFORCED | `process_executor::process_executor_attaches_child_to_a_real_job_object`; child's first instruction sees `IN_JOB` |
| Assignment timing | spawn-then-(never)-assign | CREATE_SUSPENDED → assign → ResumeThread | ENFORCED (fail-closed) | `process/windows.rs` `spawn_and_supervise` |
| Descendants in job | PARTIAL (Job Object semantics support it; donor never attached) | ADAPT Job Object tree semantics | ENFORCED | `windows_tests::timeout_terminates_the_whole_job_tree` |
| Kill-on-close | REAL (flag set on donor job) but never tested | REIMPLEMENT + real test | ENFORCED | `windows_tests::kill_on_job_close_terminates_a_running_child` |
| Memory limit | DOC-ONLY / PARTIAL (donor set 256MB but no child was ever assigned) | REIMPLEMENT opt-in | ENFORCED (opt-in) | `windows_tests::process_memory_limit_rejects_oversized_allocation` |
| Active process count | DONOR DID NOT SET | REIMPLEMENT opt-in | ENFORCED (opt-in) | `windows_tests::active_process_limit_blocks_extra_child_creation` |
| RestrictedToken creation | PARTIAL (created if `OpenProcessToken` succeeded) | DEFERRED | DEFERRED | Donor `sandbox.rs` calls `CreateRestrictedToken`; no launch path uses it |
| Restricted-token launch | NOT ENFORCED | DEFERRED | DEFERRED | Donor spawns via `std::process::Command` / `tokio::process::Command`; `restricted_token` is never used |
| Timeout | PARTIAL (only `ToolSynthesizer` used `tokio::time::timeout` on `cmd.output()`) | REIMPLEMENT deterministic supervision loop | ENFORCED | `timeout_terminates_the_child`; `timeout_terminates_the_whole_job_tree` |
| stdout/stderr bound | PARTIAL (synthesis truncated after full read) | REIMPLEMENT read-time bound + explicit truncation flags | ENFORCED | `stdout_limit_truncates_and_reports`, `stderr_limit_truncates_and_reports` |
| Worktree sandbox | REAL but workspace isolation only; not OS containment | DEFER (future workspace isolation primitive) | DEFERRED | `origin/master:.../worktree.rs` |
| ToolSynthesizer integration | BROKEN (stored `Arc<PlatformSandbox>` but never attached child) | DROP / not ported | NOT PORTED | `origin/master:.../synthesis.rs`; `sandbox` field is `#[allow(dead_code)]` |

Donor conclusion: `master` had a real Job Object and a restricted-token handle, but
the spawned shell/repo/synthesis child processes never called
`assign_process`, so the sandbox was decoration, not enforcement. M2B therefore
did **not** direct-port `PlatformSandbox`; it reused only the correct low-level
Windows primitives and built a fail-closed canonical executor.

## 2. Canonical owner

The process execution boundary lives in
`crates/apeireth-tools-canonical/src/process/`:

```text
process/
  mod.rs        ProcessRequest / ProcessLimits / ProcessResult / ProcessError
                ProcessExecutor + platform-neutral supervision loop
  windows.rs    Windows Job Object + CREATE_SUSPENDED -> assign -> resume
  platform.rs   Non-Windows guardrails-only implementation
```

It is inside the tool infrastructure crate because builtin tool capabilities
are its only current consumers. It depends only on the same crate's public
types and, on Windows, `windows-sys`. It does **not** depend on runtime,
gateway, provider, governance, or a second registry/manager.

## 3. Public contract

Actual names:

- `ProcessRequest { executable, args, working_directory, environment, limits }`
  - builder: `ProcessRequest::new("git").with_args([...]).with_working_directory(...).with_limits(...)`
  - no `command: String`; no `cmd /c`; no `sh -c`
- `ProcessLimits { max_runtime, max_stdout_bytes, max_stderr_bytes, max_process_memory_bytes, max_active_processes, kill_on_job_close }`
  - `ProcessLimits::default()` is bounded: 30s, 64KiB stdout, 64KiB stderr, kill-on-close on
  - `ProcessLimits::unrestricted()` is explicit opt-out and is not used by builtin tools
- `EnvironmentSpec { Inherit, Clear, Explicit(Vec<(OsString, OsString)>) }`
- `ProcessResult { termination, stdout, stderr, stdout_truncated, stderr_truncated, enforcement }`
- `TerminationReason { Exited { code }, TimedOut { code } }`
- `PlatformEnforcement { platform, containment, fail_closed_spawn }`
- `ProcessError { InvalidConfiguration, SpawnFailed, ContainmentFailed, PlatformUnsupported, Io }`

A non-zero child exit is a `ProcessResult`, never a `ProcessError`.

## 4. Enforcement matrix

| Boundary | Windows | Non-Windows |
| --- | --- | --- |
| Timeout | ENFORCED via Job Object `TerminateJobObject` (whole tree) | ENFORCED for direct child (`child.kill`) |
| Output bound | ENFORCED read-time cap + truncation flags | ENFORCED read-time cap + truncation flags |
| Working directory | ENFORCED (`Command::current_dir`) | ENFORCED (`Command::current_dir`) |
| Environment policy | ENFORCED (`env_clear` / explicit vars) | ENFORCED (`env_clear` / explicit vars) |
| Job Object attachment | ENFORCED before first instruction (`CREATE_SUSPENDED`) | Not applicable |
| Kill-on-job-close | ENFORCED | Not applicable |
| Descendant containment | ENFORCED (Job Object tree semantics + real test) | PARTIAL (direct-child kill only; no namespaces/cgroups/seccomp) |
| Memory limit | ENFORCED when configured (opt-in, tested) | DEFERRED |
| Active process limit | ENFORCED when configured (opt-in, tested) | DEFERRED |
| Restricted token | DEFERRED | DEFERRED |
| Filesystem OS isolation | NOT ENFORCED (not claimed) | NOT ENFORCED |
| Network isolation | NOT ENFORCED (not claimed) | NOT ENFORCED |

Non-Windows is accurately described as **process execution guardrails only**,
not an OS privilege sandbox.

## 5. Windows safe-spawn path

```text
std::process::Command
  + creation_flags(CREATE_SUSPENDED)
  + stdout/stderr/stdin pipes
        |
        v
CreateProcessW (suspended)
        |
        v
AssignProcessToJobObject
        |
        v
ResumeThread (main thread via Toolhelp snapshot)
        |
        v
supervision loop (try_wait + timeout + bounded output readers)
```

The child cannot execute before job attachment. The test
`process_executor_attaches_child_to_a_real_job_object` runs a helper whose first
instruction calls `IsProcessInJob`; it observes `IN_JOB`.

## 6. Tests

Real child tests live in `crates/apeireth-tools-canonical/tests/process_executor.rs`
and use the test-only helper binary `sandbox-test-child`
(`src/bin/sandbox_test_child.rs`). The helper accepts fixed modes only and
parses no user command strings.

Generic (all platforms):

- successful execution + stdout capture
- structured args preserved (Unicode and spaces)
- explicit working directory
- stderr capture
- non-zero exit is a result, not executor error
- spawn failure is executor error
- timeout terminates child
- stdout limit truncates and reports
- stderr limit truncates and reports
- simultaneous large stdout + stderr does not deadlock
- environment clearing
- environment explicit mode

Windows-only (`#[cfg(windows)]`):

- child attached to a real Job Object
- kill-on-job-close terminates a running child
- timeout terminates the whole job tree (parent + descendant)
- active process limit blocks extra child creation
- process memory limit rejects an oversized allocation

No mock process executor exists in these tests.

## 7. Limitations

- `std::process::Command` is used for command-line quoting, Unicode, working
  directory, environment, and pipes. The custom Windows code only adds
  suspended creation, job assignment, and thread resume.
- If the current process is already inside a Job Object that does not permit
  nested jobs, `AssignProcessToJobObject` fails; the executor returns
  `ContainmentFailed` and kills the suspended child rather than continuing
  uncontained.
- On non-Windows, timeout kills the direct child only. A child that daemonizes
  descendants may leave them behind. Non-Windows is guardrails-only.
- If a descendant inherits stdout/stderr pipes and keeps them open after the
  direct child exits on non-Windows, the output reader is bounded by a channel
  timeout so the executor does not hang forever; the tail of output may be
  dropped in that edge case.
- Restricted-token launch is not implemented. A Job Object is a process
  containment/resource boundary, **not** a filesystem or network boundary.

## 8. Future M2C requirements

M2C Shell may only begin if the process boundary is accepted as suitable for
arbitrary command execution. Before that:

- The fail-closed Windows spawn path must remain intact and tested.
- `max_runtime`, output bounds, and job attachment must remain defaults for any
  M2C command tool.
- Environment policy for shell must be explicit/cleared, not ambient inherit.
- Restricted-token or equivalent identity/ACL hardening should be re-evaluated
  for untrusted command execution.
- Network egress is still M2D and must not be claimed.
