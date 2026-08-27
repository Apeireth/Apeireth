# M2C-R — Cross-Platform Shell Readiness Review

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。补充：文中"无 persisted approval / 无 resume"已被 M2C-A 实现推翻，见 `crates/engine/runtime/src/canonical/approval.rs`。

Status: review complete
Branch: `reconstruct_v2`
Starting HEAD: `732abddce3d1a22eb8f81cd9adf50e23ecc2d011`

This review is READ-ONLY / DESIGN-FREEZE / THREAT-MODEL only. It does **not**
implement a shell, does not add `ShellTool`, `cmd /c`, `sh -c`, `powershell`,
or any arbitrary-command execution capability.

---

## 1. Overall Verdict

**BLOCKED_BY_APPROVAL_LIFECYCLE.**

Trusted Shell is the only shell profile that is technically coherent under the
current cross-platform isolation facts, and it is coherent **only** with
per-invocation human approval. However the canonical Runtime currently stops at
`RuntimeError::ApprovalRequired`; it has no persisted pending-approval entity,
no stable approval id, no approve/reject API, and no resume-same-turn path.
Implementing `tool.shell` next would produce a capability that can say
"RequireApproval" but cannot be safely resumed after the human says yes.
Therefore the next phase must be **M2C-A — Canonical Approval Resume
Lifecycle**, not a Shell implementation.

---

## 2. Git

| Field | Value |
| --- | --- |
| Branch | `reconstruct_v2` |
| Starting HEAD | `732abddce3d1a22eb8f81cd9adf50e23ecc2d011` |
| Remote HEAD | `732abddce3d1a22eb8f81cd9adf50e23ecc2d011` |
| Working tree | clean |
| Push | no (document review; push after final validation) |

---

## 3. Current Security Facts

Canonical facts are taken from the current code and architecture docs. They are
not re-interpreted here.

### 3.1 Governance

- Canonical tri-state: `Allow`, `Deny`, `RequireApproval`.
- `GovernancePipeline` composes `GovernanceHook`s in order and stops at the
  first non-allow verdict.
- `PermissionPolicy::decision_for_capability` is fail-closed: missing
  `Permission::ExecuteTool(name)` and no `AdminOverride` returns `Deny`.
  A granted capability can be marked with `require_approval_for(name)` to
  return `RequireApproval`.
- Input-security hooks (`PiiDetector`, `PromptInjectionHeuristic`,
  `CredentialDisclosureHook`) are signals / reviewers, not command sanitizers.

### 3.2 ProcessExecutor

- `ProcessRequest` is structured: `executable` + `args`, explicit
  `working_directory`, `EnvironmentSpec`, `ProcessLimits`, and
  `IsolationRequirement`. There is no `command: String` and no `cmd /c` or
  `sh -c` inside the executor.
- `ProcessLimits::default()` is bounded: 30s runtime, 64 KiB stdout, 64 KiB
  stderr. Optional memory/process-count/CPU/file-size limits fail closed when a
  platform cannot enforce a configured limit.
- `EnvironmentSpec::{Inherit, Clear, Explicit}`. `Clear`/`Explicit` are
  enforced by `env_clear` + explicit vars.
- `IsolationRequirement` is checked **before** child spawn and returns
  `ProcessError::IsolationRequirementUnsatisfied` when the platform capability
  level is below the required level.
- `IsolationProfile` presets exist (`Trusted`, `Restricted`, `Untrusted`) but
  a caller may declare an explicit `IsolationRequirement`.

### 3.3 Platform matrix (actual)

| Capability | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Structured spawn | ENFORCED | ENFORCED | ENFORCED |
| Explicit cwd | ENFORCED | ENFORCED | ENFORCED |
| Timeout | ENFORCED | ENFORCED | ENFORCED |
| stdout/stderr bound | ENFORCED | ENFORCED | ENFORCED |
| Environment isolation | ENFORCED | ENFORCED | ENFORCED |
| Process-tree containment | ENFORCED (Job Object) | PARTIAL (process group; descendants can create their own group/session and escape) | PARTIAL (same as Linux) |
| Memory limit | ENFORCED when configured | PARTIAL (`RLIMIT_AS`) | PARTIAL (`RLIMIT_AS`) |
| Process-count limit | ENFORCED when configured | PARTIAL (`RLIMIT_NPROC`, UID-scoped) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) |
| CPU limit | UNSUPPORTED | ENFORCED (`RLIMIT_CPU`) | ENFORCED (`RLIMIT_CPU`) |
| File-size limit | UNSUPPORTED | ENFORCED (`RLIMIT_FSIZE`) | ENFORCED (`RLIMIT_FSIZE`) |
| Privilege reduction | ENFORCED when restricted-token launch is possible; otherwise UNSUPPORTED (ordinary non-admin users typically see UNSUPPORTED) | PARTIAL (`PR_SET_NO_NEW_PRIVS` only) | UNSUPPORTED |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Network isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Fail-closed pre-exec containment | ENFORCED (`CREATE_SUSPENDED` -> assign -> resume) | ENFORCED (pre_exec setup before exec) | ENFORCED (pre_exec setup before exec) |

### 3.4 Egress

Controlled `EgressTransport` is application-owned HTTP/DNS policy. It does
**not** restrict sockets opened by an arbitrary child process. Process
`NetworkIsolation` is `UNSUPPORTED` on all three platforms.

---

## 4. Primary Question

**Can Apeireth expose a Shell capability under the current platform isolation
facts?**

Not as one undifferentiated "shell". The three profiles must be treated
separately.

- **Trusted Shell**: technically coherent now, but product-complete only after
  the approval-resume lifecycle exists. It must be an explicitly approved
  local-command execution mode with the user's OS authority, with no
  filesystem/network isolation claim.
- **Restricted Shell**: platform-conditional. It can only mean
  "reduced privilege where the OS can actually reduce it and stronger process
  guardrails"; it still does not mean filesystem/network sandboxing.
- **Untrusted Shell**: **not ready anywhere**. The current backends have no
  filesystem isolation and no network isolation, and Unix process-tree
  containment is only PARTIAL. Those are physical containment blockers, not
  policy gaps.

---

## 5. Definitions

Refined against the canonical contracts:

### 5.1 Trusted Shell

- The user knowingly approves one exact local command/script.
- The command runs with the user's effective OS account authority.
- The product makes **no** filesystem isolation claim and **no** network
  isolation claim.
- The product provides the common process guardrails that exist on all three
  platforms: structured shell invocation through `ProcessExecutor`, explicit
  cwd, bounded timeout, bounded stdout/stderr, and environment isolation from
  the ambient process environment.
- Optional resource limits (memory/process count) may be configured but are
  not part of the cross-platform baseline.

Trusted means **the user accepted the command's authority**; it does not mean
the command is intrinsically safe.

### 5.2 Restricted Shell

- Reduced identity/privilege is required where the platform can provide it:
  Windows restricted-token launch when available; Linux
  `PR_SET_NO_NEW_PRIVS` (advertised only as PARTIAL); macOS has none.
- Stronger process containment is required: Windows Job Object ENFORCED;
  Linux/macOS process-group PARTIAL is accepted only with the escape risk
  disclosed.
- Resource limits (memory/process-count) are required to be enforceable at
  least PARTIAL and configured where the profile is used.
- Filesystem isolation and network isolation may still be absent depending on
  platform; therefore a Restricted Shell is **not** a sandboxed shell.

### 5.3 Untrusted Shell

- Suitable for autonomous, model-generated arbitrary commands.
- Requires strong physical containment: process-tree containment ENFORCED,
  privilege reduction ENFORCED, filesystem isolation ENFORCED, and network
  isolation ENFORCED.
- This is intentionally unsatisfiable on all current backends and must fail
  closed.

---

## 6. Key Distinction — Approval vs Sandbox

- **Approval** answers: "Did the user approve this action?"
- **Sandbox** answers: "What can this action physically affect?"

A `RequireApproval` decision is not containment. A user-approved Trusted Shell
may intentionally have broad local access; that is a valid product mode if and
only if it is labelled honestly. Sandboxing is a `ProcessExecutor` /
`IsolationRequirement` concern; approval is a `Governance` concern. Neither
replaces the other.

---

## 7. Threat Model

Classification legend:

- **PREVENT** — canonical control physically or contractually stops the action.
- **LIMIT** — canonical control narrows or bounds the action but does not stop it.
- **REQUIRE APPROVAL** — control is policy/approval, not physical containment.
- **DO NOT PREVENT** — no current canonical control stops the action.

| Threat | Current canonical control | Trusted Shell impact | Restricted Shell impact | Untrusted Shell impact |
| --- | --- | --- | --- | --- |
| Arbitrary filesystem read | Filesystem isolation UNSUPPORTED | Accepted user-approved risk; not prevented | Not prevented on Linux/macOS; Windows restricted token may narrow ACL when available | Blocker |
| Arbitrary filesystem write | Filesystem isolation UNSUPPORTED | Accepted user-approved risk; not prevented | Not prevented on Linux/macOS; Windows restricted token may narrow ACL when available | Blocker |
| Credential theft (filesystem-based: SSH keys, browser tokens, Git credentials) | Filesystem isolation UNSUPPORTED | DO NOT PREVENT; must be disclosed | DO NOT PREVENT; disclosure + privilege reduction only | Blocker |
| Environment-secret theft | `EnvironmentSpec::Clear` / `Explicit` ENFORCED; Shell contract must not use `Inherit` | PREVENT by Shell env contract | PREVENT by Shell env contract | PREVENT by Shell env contract, but not sufficient alone |
| SSH key access | Filesystem isolation UNSUPPORTED | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Browser/session token access | Filesystem isolation UNSUPPORTED | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Git credential access | Filesystem isolation UNSUPPORTED | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Network exfiltration | Process `NetworkIsolation` UNSUPPORTED; controlled egress does not apply to child sockets | DO NOT PREVENT; user-approved local shell has network authority | DO NOT PREVENT | Blocker |
| Network scanning | Process `NetworkIsolation` UNSUPPORTED | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Local service access | No process network/service isolation | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Process spawning | Shell is arbitrary command by definition; structured spawn through ProcessExecutor | EXPECTED capability; bounded lifetime by timeout | EXPECTED capability; process-count limit where configured | Blocker without Enforced tree containment |
| Persistence | Filesystem write UNSUPPORTED | DO NOT PREVENT | DO NOT PREVENT | Blocker |
| Fork/process bombs | Windows Job Object process-count ENFORCED when configured; Unix `RLIMIT_NPROC` PARTIAL/UID-scoped | LIMIT if optional limits configured | LIMIT (required limits configured) | LIMIT only; not enough without filesystem/network isolation |
| Disk fill | File-size limit ENFORCED on Unix; UNSUPPORTED on Windows | DO NOT PREVENT by default | LIMIT on Unix when configured; DO NOT PREVENT on Windows | Blocker |
| Memory exhaustion | Windows Job Object ENFORCED when configured; Unix `RLIMIT_AS` PARTIAL | LIMIT if optional limits configured | LIMIT (required) | LIMIT only; not enough |
| CPU exhaustion | Unix `RLIMIT_CPU` ENFORCED; Windows UNSUPPORTED | DO NOT PREVENT by default | LIMIT on Unix when configured; DO NOT PREVENT on Windows | LIMIT only; not enough |
| Shell metacharacter injection | Shell is intentional raw syntax; `ProcessExecutor` remains structured | Intended capability; no mitigation needed | Intended capability; no mitigation needed | Intended capability; containment must carry the risk |
| Working-directory escape | Explicit cwd ENFORCED, but cwd is not a filesystem boundary | User-approved; no sandbox claim | No sandbox claim; cwd is a starting directory only | Blocker |
| Command substitution | Shell interprets it intentionally | User-approved | User/approval must see exact script | Blocker without containment |
| Download-and-execute | Network + filesystem UNSUPPORTED | DO NOT PREVENT; approval must make it visible | DO NOT PREVENT | Blocker |
| Child process escape | Windows Job Object ENFORCED (no breakaway flags set); Unix process group PARTIAL (`setsid`/new group can escape) | Trusted: acceptable with timeout/kill; disclosed | Restricted: acceptable only on Windows; disclosed on Unix as PARTIAL | Blocker |
| Platform-specific privilege escalation | Windows restricted token runtime-dependent; Linux `no_new_privs` PARTIAL; macOS UNSUPPORTED | Trusted does not reduce privileges; user authority accepted | Conditional: only where privilege reduction exists | Blocker |

---

## 8. Environment Secret Threat

For any future Shell capability, ambient `EnvironmentSpec::Inherit` must **not**
be the default. Canonical Shell MUST use `EnvironmentSpec::Clear` or
`EnvironmentSpec::Explicit` minimal environment by default.

Usability implication: a shell with an empty environment is not useful, so the
Shell bootstrap must build a small explicit environment (see section 9). This
is a Shell-layer responsibility; `ProcessExecutor` remains vendor-neutral and
must not contain a vendor secret blacklist.

---

## 9. Minimal Shell Environment

The default is an **explicit minimal environment**, not `Inherit` and not pure
`Clear`.

### 9.1 Windows

- `SystemRoot` (e.g. `C:\Windows`)
- `WINDIR` (e.g. `C:\Windows`)
- `TEMP` and `TMP` (explicit temp directory)
- `PATH` only if explicit command resolution is desired; the shell executable
  itself should be resolved by absolute path, not via ambient `PATH`
- `COMSPEC` only if the selected shell is `cmd.exe` and the bootstrap chooses
  to set it

Must **not** automatically inherit `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`,
provider API keys, cloud credentials, proxy credentials, or Apeireth internal
secrets.

### 9.2 Linux / macOS

- `PATH` (explicit minimal value, e.g. `/usr/local/bin:/usr/bin:/bin`)
- `TMPDIR` (e.g. `/tmp`)
- `LANG` optional, recommended for deterministic text handling
  (e.g. `C.UTF-8`); `TERM` optional and not required for one-shot
  non-interactive execution
- `HOME` is **not** included by default. `HOME` semantically exposes user
  config and credential-bearing dotfiles even though the process may still be
  able to read them through the filesystem.

Differentiate environment secrecy from filesystem authority: a minimal
environment hides ambient secrets, but it does not restrict filesystem access.

---

## 10. Credential Environment

Future Shell must not automatically inherit provider API keys, GitHub tokens,
AWS credentials, cloud credentials, proxy credentials, or Apeireth internal
secrets. The canonical `ProcessExecutor` stays vendor-neutral; no secret
blacklist belongs inside the executor. Shell bootstrap builds an explicit safe
environment and passes it as `EnvironmentSpec::Explicit` or `Clear`.

---

## 11. Working Directory

Shell must require an explicit workspace/cwd. The Shell schema must not rely on
ambient `current_dir` implicitly. The executor enforces `Command::current_dir`.

Explicit cwd is **not** filesystem sandboxing. It is only the child's starting
directory. This must be stated in UI/API copy.

---

## 12. Filesystem Reality

Current filesystem OS isolation is `UNSUPPORTED` on all platforms. A shell
child can potentially access any path the effective user can access.

- **Trusted Shell can exist** under this fact only because the user approved
  the exact command and the product says the command runs with the user's
  filesystem authority.
- **Restricted Shell can exist only as a conditional profile**: reduced
  privilege where the OS can actually reduce it, but still no filesystem
  isolation claim. Workspace cwd does not confine it.
- **Untrusted Shell cannot exist** under this fact. No filesystem isolation is
  a hard blocker for autonomous arbitrary commands.

---

## 13. Network Reality

Current process `NetworkIsolation` is `UNSUPPORTED` on all platforms. A child
can potentially run `curl`, `wget`, `PowerShell Invoke-WebRequest`, Python
sockets, Node `fetch`, `git` network commands, or custom binaries. Controlled
`EgressTransport` does **not** restrict these.

- **Trusted Shell**: absence of process network isolation does **not** block a
  user-approved Trusted Shell. The OS terminal the user already runs has
  network access. The requirement is accurate representation plus explicit
  approval.
- **Restricted Shell**: network access remains unrestricted; the profile must
  say so. It is conditional only on the privilege/process/resource properties,
  not on network isolation.
- **Untrusted Shell**: no process network isolation is a hard blocker.

---

## 14. Windows Profile

Facts: Job Object strong process-tree containment, restricted token
runtime-dependent, filesystem unrestricted under effective user ACL, network
unrestricted.

| Profile | Verdict |
| --- | --- |
| Trusted Shell | **READY** |
| Restricted Shell | **CONDITIONAL** |
| Untrusted Shell | **NOT READY** |

### 14.1 Windows Trusted Shell requirement

```
IsolationRequirement::new()
  .require(StructuredSpawn, Enforced)
  .require(ExplicitCwd, Enforced)
  .require(Timeout, Enforced)
  .require(StdoutLimit, Enforced)
  .require(StderrLimit, Enforced)
  .require(EnvironmentIsolation, Enforced)
  .require(ProcessTreeContainment, Partial)
  .require(FailClosedPreExecutionContainment, Enforced)
```

Satisfied on Windows: `ProcessTreeContainment` is ENFORCED (>= Partial),
`EnvironmentIsolation` ENFORCED, `FailClosedPreExecutionContainment` ENFORCED.

### 14.2 Windows Restricted Shell requirement

```
TrustedShellRequirement
  .require(MemoryLimit, Partial)
  .require(ProcessCountLimit, Partial)
  .require(PrivilegeReduction, Partial)
```

- Satisfied only when `privilege_reduction` reports ENFORCED (restricted-token
  launch actually probed and working). Ordinary non-admin users typically see
  `UNSUPPORTED`, so the requirement fails closed before child spawn.
- Not required, and not claimed: `FilesystemIsolation`, `NetworkIsolation`.
- Not required (because Windows reports UNSUPPORTED): `CpuLimit`,
  `FileSizeLimit`; a Shell profile must not silently lower these to launch —
  they are simply not part of the Restricted Shell definition.

### 14.3 Windows Untrusted Shell

`IsolationProfile::Untrusted` requires `PrivilegeReduction ENFORCED`,
`FilesystemIsolation ENFORCED`, `NetworkIsolation ENFORCED`. Filesystem and
network are `UNSUPPORTED`; therefore **NOT READY**. Fail closed.

### 14.4 Windows escape review

`JobObject::create` sets `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` always and adds
memory/process limits only when configured. It does **not** set
`JOB_OBJECT_LIMIT_BREAKAWAY_OK` or `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`.
Spawn is `CREATE_SUSPENDED` -> `AssignProcessToJobObject` -> `ResumeThread`; if
assignment fails the suspended child is killed and `ContainmentFailed` is
returned. No Windows breakaway flag is currently enabled.

---

## 15. Linux Profile

Facts: process-group PARTIAL, `PR_SET_NO_NEW_PRIVS` PARTIAL, filesystem
unrestricted, network unrestricted, rlimits partial/enforced by type.

| Profile | Verdict |
| --- | --- |
| Trusted Shell | **READY** |
| Restricted Shell | **CONDITIONAL** |
| Untrusted Shell | **NOT READY** |

### 15.1 Linux Trusted Shell requirement

Same `TrustedShellRequirement` as Windows. Satisfied on Linux:
`ProcessTreeContainment` PARTIAL, `EnvironmentIsolation` ENFORCED,
`FailClosedPreExecutionContainment` ENFORCED.

### 15.2 Linux Restricted Shell requirement

Same as Windows Restricted requirement. Satisfied on Linux:
`PrivilegeReduction` PARTIAL (`PR_SET_NO_NEW_PRIVS` only),
`MemoryLimit`/`ProcessCountLimit` PARTIAL (`RLIMIT_AS`, `RLIMIT_NPROC`
UID-scoped), `ProcessTreeContainment` PARTIAL.

Verdict is CONDITIONAL, not READY, because:

- process-group containment is PARTIAL: descendants that call `setsid` or
  create a new process group can escape tree cleanup;
- `PR_SET_NO_NEW_PRIVS` is not true user isolation and is not a seccomp/namespace
  sandbox;
- filesystem and network isolation remain absent.

### 15.3 Linux Untrusted Shell

**NOT READY**. `PrivilegeReduction` is only PARTIAL, and `FilesystemIsolation`
/ `NetworkIsolation` are UNSUPPORTED. `IsolationProfile::Untrusted` fails
closed.

---

## 16. macOS Profile

Facts: process-group PARTIAL, no privilege reduction, filesystem unrestricted,
network unrestricted.

| Profile | Verdict |
| --- | --- |
| Trusted Shell | **READY** |
| Restricted Shell | **NOT READY** |
| Untrusted Shell | **NOT READY** |

### 16.1 macOS Trusted Shell requirement

Same `TrustedShellRequirement`. Satisfied on macOS:
`ProcessTreeContainment` PARTIAL, `EnvironmentIsolation` ENFORCED,
`FailClosedPreExecutionContainment` ENFORCED.

### 16.2 macOS Restricted Shell

**NOT READY**. `PrivilegeReduction` is `UNSUPPORTED`. A Restricted Shell
requires at least PARTIAL privilege reduction, so the requirement fails closed
before child spawn.

### 16.3 macOS Untrusted Shell

**NOT READY**. `PrivilegeReduction` UNSUPPORTED, `FilesystemIsolation`
UNSUPPORTED, `NetworkIsolation` UNSUPPORTED.

---

## 17. Cross-Platform Product Contract

The user-facing Shell capability should have **one product contract**:

> `tool.shell` is a user-approved local shell. It runs the exact approved
> command with the user's OS account authority. It is not a filesystem
> sandbox and not a network sandbox. Apeireth bounds process lifetime and
> output where supported, and protects the child from ambient environment
> secrets.

Enforcement may vary by platform, but the contract does not expose
`JobObject`, `rlimit`, `no_new_privs`, or any other backend mechanism. Users
must not select `JobObject` or `rlimit`.

M2C v1 should ship only one canonical mode: **TrustedShell** (per-invocation
approval). RestrictedShell and UntrustedShell remain separate later phases.

---

## 18. Initial Product Decision (candidate evaluated)

Candidate:

- M2C v1 ships only **TrustedShell**.
- Always `RequireApproval`; never auto-approved.
- Explicit cwd; Clear/Explicit environment; bounded timeout; bounded output;
  ProcessExecutor containment; no filesystem isolation claim; no network
  isolation claim; clear UI/API warning that command runs with local user
  authority; disabled for autonomous/background operation.
- RestrictedShell deferred; UntrustedShell deferred.

Evaluation: the candidate is **conceptually correct**, with two mandatory
preconditions:

1. **M2C-A approval-resume lifecycle must exist first.** Without it, a shell
   invocation that returns `RequireApproval` cannot be resumed.
2. The tool must be **disabled by default** and registered only when explicitly
   enabled; enabling it without an explicit approval policy must fail closed
   (Deny), not fall through to `AllowAll`.

The candidate is accepted as the target contract **after** M2C-A.

---

## 19. RequireApproval

Every `tool.shell` invocation must map to `RequireApproval` in M2C v1. There
are no trusted-automation exceptions in M2C v1. Generic
`PermissionGovernanceHook` can express this: grant
`Permission::ExecuteTool("tool.shell")` and call
`require_approval_for("tool.shell")`. A dedicated `ShellApprovalHook` is not
justified; it would be a second policy surface. If a named hook is desired for
audit clarity, it must only return `RequireApproval` for `tool.shell`, but the
preferred implementation is generic `PermissionPolicy`.

No persistent "always allow shell" approval is part of M2C v1. Approval is
per-invocation only. Wildcard approvals ("allow all commands under repo X")
are out of scope for v1.

---

## 20. Deny Default

If `tool.shell` is registered but no explicit Shell permission/policy exists,
the canonical safe behavior is **Deny**.

Rationale:

- `PermissionPolicy::decision_for_capability` already returns `Deny` when the
  `ExecuteTool("tool.shell")` grant is missing and `AdminOverride` is absent.
- This is consistent with fail-closed capability dispatch.
- `RuntimeBuilder` defaults to `AllowAll` for governance; that is a test/default
  convenience, not a Shell deployment posture. Any configuration that enables
  Shell must also install a real policy. If the policy is absent or does not
  explicitly grant and approve `tool.shell`, the capability must not be
  reachable, and if it is reached it must be denied.

---

## 21. Command Visibility / Approval Display Contract

The approval request must expose enough information for the user to know
exactly what will run. Minimum mandatory fields:

1. Capability identity: `tool.shell`.
2. Shell kind / resolved executable (e.g. `cmd.exe`, `/bin/sh`), or a stable
   label for the selected shell profile.
3. Exact command/script, raw and complete, preserving newlines, spaces,
   `&&`, `;`, pipes, redirections, and any other metacharacters. Multiline
   content must preserve newlines and must not be truncated unless the user
   can expand the full content.
4. cwd (explicit working directory).
5. Timeout (default and configured value).
6. Resource limits, when configured (memory, process count, CPU, file size).
7. Environment mode: `Clear` or `Explicit` (never `Inherit` for Shell), with
   the exact environment keys/values or a safe summary if values are sensitive.
8. Network isolation state: `UNSUPPORTED` / "none".
9. Filesystem isolation state: `UNSUPPORTED` / "none".
10. Platform isolation profile label: `TrustedShell` (or the profile name, but
    not internal mechanism names).

Optional: risk reason, prompt-injection signal (as context, not as a shell
filter).

No hidden field that can change after approval.

---

## 22. Immutable Approval Binding

The exact command approved must be the exact command executed. Approval binds:

- tool id (`tool.shell`);
- complete command/script bytes (stable UTF-8; no normalization that changes
  semantics);
- resolved shell executable / shell kind;
- cwd;
- environment mode and explicit environment entries;
- timeout;
- output bounds;
- isolation requirement/profile and any configured resource limits.

M2C v1 must reject an execution whose bound operation does not match the
approved operation exactly.

**Current status**: the canonical Governance API carries `CapabilityId` and
`arguments` only. It has no operation identity/hash and no frozen execution
operation. This is a **blocker** for M2C implementation. M2C-A must add the
canonical approval-binding primitive; M2C-T then uses it.

---

## 23. TOCTOU

Approval happens, then time passes, then execution happens. The workspace
filesystem cannot be snapshotted or bound by the current architecture. M2C v1
must therefore bind at least:

- command text;
- cwd;
- environment mode/entries;
- timeout and output/resource limits;
- isolation requirement/profile;
- shell kind/executable.

If the cwd path changes meaning (e.g. symlink retargeting) between approval and
execution, v1 does not claim to detect it. That residual TOCTOU must be
documented as a known limitation; the command is still executed only with the
user-approved exact text and execution profile.

---

## 24. Shell Interface Shape

Evaluated:

- **A. raw command string interpreted by system shell** — this is the actual
  Shell capability. It is legitimate **only** as an explicit `tool.shell`.
- **B. executable + structured args** — this is the existing
  `ProcessExecutor` contract. It remains the only process API for non-shell
  tools.
- **C. both as separate tools** — not needed in v1. One ShellCapability with a
  raw script field, layered over B, is sufficient.

Decision: **A as an explicit ShellCapability; B remains ProcessExecutor; C is
not v1.** Raw shell syntax is isolated in `tool.shell` and never hidden inside
Repo/ProcessExecutor.

---

## 25. Shell Selection

Cross-platform v1 promise: **one configured/detected shell executable per
platform**, never hard-coded `bash`.

| Platform | v1 default | Notes |
| --- | --- | --- |
| Windows | `cmd.exe` | Always present; use fixed args for one-shot command execution. |
| Linux | `/bin/sh` | POSIX shell; do not assume `bash`. |
| macOS | `/bin/sh` | Same. |

The shell executable is an explicit configuration concept (`shell_executable`
or equivalent). If configured, the path must be validated before use. The
ShellCapability resolves the platform shell and invokes `ProcessExecutor` with
`executable = shell` and `args = shell-specific args containing the script`.

PowerShell (Windows PowerShell and PowerShell 7) is **deferred**; v1 does not
pick between them. Exposing multiple shell capabilities is unnecessary
complexity for v1.

---

## 26. PowerShell

Decision: v1 does **not** support PowerShell. Windows v1 uses `cmd.exe`.
PowerShell support, if ever added, should be a later explicit configuration
choice or a separate capability; it is not part of M2C-T.

---

## 27. Raw String vs Argv

Confirmed:

```text
ShellCapability
  takes raw script/command
  resolves platform shell executable
  invokes ProcessExecutor with:
    executable = shell
    args = shell-specific args containing script
```

Arbitrary shell semantics remain in ShellCapability. `ProcessExecutor` stays
structured and never accepts `command: String`. This matches the existing
architecture invariant test
`process_infrastructure_does_not_create_a_shell_backdoor`.

---

## 28. Script Size

Future Shell input must be bounded. Recommended initial limit: **64 KiB**
(65,536 bytes) for the complete command/script, measured as UTF-8 bytes.
The limit must be enforced before shell invocation and must be visible in the
tool schema/approval payload. No unbounded multi-megabyte script strings.

---

## 29. Timeout

- Default Shell timeout: **30s** (reuses `ProcessLimits::default()`).
- Maximum configurable: **5 minutes (300s)** for M2C v1.
- Timeout is approval-visible.
- No-timeout is **not** permitted as a default. `ProcessLimits::unrestricted()`
  must not be used by Shell.

---

## 30. Output

Shell output must use existing bounded stdout/stderr. Default 64 KiB each,
reusing current `ProcessLimits::default()`. The tool result and approval
record must preserve:

- exit code / termination reason (`Exited` / `TimedOut`);
- stdout;
- stderr;
- `stdout_truncated`;
- `stderr_truncated`;
- `timed_out`.

Nonzero exit is a normal `ProcessResult` / tool result, not an infrastructure
error. No unlimited output.

---

## 31. Stdin / Interactive

M2C v1: **no interactive shell, no PTY, no stdin streaming.** Initial Shell is
non-interactive one-shot execution only. `ProcessExecutor` already sets stdin
to null. PTY and interactive sessions are future scope.

---

## 32. Background Process

Shell v1 must not intentionally detach/background processes. On Windows the
Job Object terminates descendants on job close and timeout; on Linux/macOS
process-group cleanup is PARTIAL. The documented platform risk (setsid/new
process group escape) applies to any command that daemonizes. Approval UI must
state that backgrounding is not supported and that the platform cleanup
guarantee is not absolute on Unix.

---

## 33. Unix Escape

Linux/macOS descendants may escape the process group via `setsid` or by
creating a new process group. Therefore a malicious or untrusted shell command
can intentionally evade process-tree cleanup. This is **decisive for
Untrusted Shell**: it is not ready on Unix. A trusted, user-approved command is
a different threat model and may proceed with the escape risk disclosed.

---

## 34. Windows Escape

Current Windows Job Object flags do not permit breakaway explicitly: no
`JOB_OBJECT_LIMIT_BREAKAWAY_OK`, no
`JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`. Spawn is suspended, assigned to the
Job Object before resume, and assignment failure kills the suspended child.
`KILL_ON_JOB_CLOSE` is always set. The documentation invariant is correct; no
code change needed.

---

## 35. Shell as Capability

Future canonical identity: **`tool.shell`**, `CapabilityKind::Tool`. It must
be owned through `PluginRegistry` / `CapabilityRegistry`. No direct Runtime
shell branch, no second registry, no special tool manager.

---

## 36. Risk Metadata

Shell risk metadata should be the highest existing meaningful category used by
canonical tools: **`"high"`** (current canonical tool metadata uses `"low"` /
`"medium"`; donor Shell used `High`). Risk metadata alone does **not** enforce
approval; `PermissionPolicy.require_approval_for("tool.shell")` does.

---

## 37. Permission

Required canonical Permission:

```rust
Permission::ExecuteTool("tool.shell".to_string())
```

plus

```rust
policy.require_approval_for("tool.shell");
```

No hard-coded platform/vendor permission concepts.

---

## 38. Governance Hook

A dedicated `ShellApprovalHook` is **not justified**. The generic
`PermissionGovernanceHook` already expresses the exact policy:

- missing grant -> `Deny`;
- grant present and `require_approval_for("tool.shell")` ->
  `RequireApproval`.

Avoid a second policy system. If future auditing wants a named hook, it should
be a thin `GovernanceHook` whose only rule is `tool.shell => RequireApproval`;
but v1 should use `PermissionPolicy`.

---

## 39. Input Security

The prompt-injection detector must **not** be used to "sanitize" shell
commands. Shell is arbitrary command by definition. Prompt injection is
relevant before the model chooses an action, not as a magical shell filter.
`PromptInjectionHook` may contribute to approval context, but it does not
sanitize the command.

---

## 40. Command Blacklist

Explicitly reject an architecture based on denying `rm`, `curl`, `powershell`,
`sudo`, etc. Blacklists are not containment. M2C v1 safety comes from
**approval + execution limits + honest authority model**, not blacklist
theater.

---

## 41. Network Command Blacklist

Likewise, blocking `curl`/`wget` strings does **not** provide
`NetworkIsolation`. It is not recommended.

---

## 42. User Expectation Wording

User-facing wording (concise but honest):

> "This command runs with your local user account's filesystem and network
> access. Apeireth limits the process lifetime and output where supported, but
> this is not a filesystem or network sandbox."

Plus a one-line profile label such as `TrustedShell — local user authority`.

---

## 43. Autonomous Agent Mode

`tool.shell` must **not** run in autonomous/background mode without a person
present. With current capabilities the answer is **NO**. Enabling the
capability globally is separate from approving each invocation.

---

## 44. Remembered Approval

Do not allow "Always allow shell" as an implied M2C-v1 feature. Persistent
approval policy is separate product/security work. Initial recommendation:
**per-invocation approval**.

---

## 45. Wildcard Approval

Avoid approvals such as "allow all commands under repo X" for v1. Scoped
policies are future work, not Shell initial rollout.

---

## 46. CI / Test Plan for Future M2C

Design only; do not implement.

### Unit

- schema/input validation (script size, required cwd, no `Inherit` env);
- command construction: `ShellCapability` -> `ProcessRequest` with
  `executable = shell`, `args = shell args + script`;
- shell selection per platform/config;
- environment profile construction (explicit minimal env);
- approval metadata construction (all mandatory display fields, no secrets).

### Runtime E2E

- FakeProvider -> `ToolCall(tool.shell)` -> Governance `RequireApproval`
  -> tool not invoked;
- approved continuation test if canonical approval mechanism supports it;
- denied continuation test.

### Tool integration

- Actual one-shot harmless command;
- cwd honored;
- exit code returned;
- stdout/stderr captured;
- timeout honored;
- truncation reported.

### Platform

- Windows, Ubuntu, macOS CI;
- no destructive tests.

---

## 47. Test Commands

Future real Shell tests must use only harmless commands such as:

- print fixture (`echo` / `printf`);
- `pwd` / cwd verification;
- exit with a specific code;
- sleep bounded duration.

No filesystem destruction, no public network, no real credentials.

---

## 48. Approval E2E Blocker

Canonical Runtime currently supports:

- `RequireApproval` is represented (`Decision::RequireApproval`,
  `RuntimeError::ApprovalRequired`, `SessionEventKind::ApprovalRequired`);
- the E2E path proves that `RequireApproval` blocks execution.

It does **not** support:

- persisted pending approval entity;
- stable approval id;
- later approve/reject;
- resume of the same turn;
- execution of an exact frozen `ToolCall` after approval.

**THIS IS A HARD BLOCKER.**

---

## 49. Approval Lifecycle Audit

| # | Question | Status | Evidence |
| --- | --- | --- | --- |
| 1 | How is `RequireApproval` represented? | **REAL** | `Decision::RequireApproval`, `RuntimeError::ApprovalRequired`, `SessionEventKind::ApprovalRequired` |
| 2 | Is the approval request persisted? | **PARTIAL** | `ApprovalRequired` session event is saved, but there is no resumable approval-request entity or pending-approval record |
| 3 | Does it have a stable approval id? | **ABSENT** | No approval id exists |
| 4 | Can caller approve/reject later? | **ABSENT** | No approve/reject API or transport endpoint |
| 5 | Can Runtime resume the same turn? | **ABSENT** | `Runtime::execute` returns `Err(ApprovalRequired)`; no resume path |
| 6 | Is ToolCall immutable? | **PARTIAL** | The same `ToolCall` value is used for governance and dispatch in the current path, but the type is plain public data with no freeze/hash/binding mechanism |
| 7 | Is command/cwd/profile bound? | **ABSENT** | Governance sees only `CapabilityId` + `arguments`; cwd/profile are constructed later inside the tool and are invisible to governance |
| 8 | Can approval expire? | **ABSENT** | No expiry concept exists |
| 9 | Is double approval/idempotency handled? | **ABSENT** | No approval lifecycle exists |
| 10 | Is reject persisted/traced? | **PARTIAL** | `GovernanceDenied` session event + `GovernanceEvaluated` trace exist, but a later "reject" action has no representation |

Overall lifecycle status: **ABSENT** (the resume half is missing; only the
blocking half exists).

---

## 50. If Approval Resume Is Absent

Do **not** recommend implementing Shell next. Recommend:

> **M2C-A — Canonical Approval Resume Lifecycle**

first. A Shell that always returns `RequireApproval` but cannot be safely
resumed is not a complete product capability.

---

## 51. If Approval Resume Exists

If M2C-A lands, the M2C-T gate becomes:

- verify approval binding is over an immutable operation (command text, cwd,
  env mode, limits, isolation profile, shell kind);
- verify TOCTOU limitations are documented;
- then implement Trusted Shell.

---

## 52. Session / Trace

Future Shell actions must produce canonical:

- Governance event (`GovernanceEvaluated`);
- `CapabilityDispatched`;
- `CapabilityCompleted` / `ToolFailed`;
- approval event (`ApprovalRequired`, plus future `ApprovalGranted` /
  `ApprovalRejected`);
- process termination facts in the tool result;
- tool result message in transcript.

No raw CoT. No shell-specific trace subsystem.

---

## 53. Audit Hash Chain

`AuditHashChain` exists but is not wired. Do not make its wiring a mandatory
Shell blocker unless architecture requires it. Shell approvals/execution are
good future audit events. Document as optional later integration.

---

## 54. Storage

Do not add Shell-specific SQLite tables. The approval lifecycle may use the
canonical `SessionStore` / session events if appropriate.

---

## 55. Provider

Provider remains unaware of Shell implementation. It only sees canonical tool
declaration/result. No vendor branching.

---

## 56. Gateway

Gateway must not execute shell directly. If an approval API is later exposed:

```text
Gateway -> Runtime approval lifecycle -> exact frozen ToolCall -> ShellCapability
```

not:

```text
Gateway -> ShellCapability.execute
```

---

## 57. CLI

CLI may later display approval UI. It must not own approval decision
semantics.

---

## 58. Cross-Platform Matrix

| Property | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Process tree | ENFORCED (Job Object) | PARTIAL (process group; escape via setsid/new group) | PARTIAL (same) |
| Timeout | ENFORCED | ENFORCED | ENFORCED |
| Output bound | ENFORCED | ENFORCED | ENFORCED |
| Environment isolation | ENFORCED | ENFORCED | ENFORCED |
| Memory | ENFORCED when configured | PARTIAL (`RLIMIT_AS`) | PARTIAL (`RLIMIT_AS`) |
| Process count | ENFORCED when configured | PARTIAL (`RLIMIT_NPROC`, UID-scoped) | PARTIAL (`RLIMIT_NPROC`, UID-scoped) |
| Privilege reduction | ENFORCED when restricted-token launch is possible; otherwise UNSUPPORTED | PARTIAL (`no_new_privs` only) | UNSUPPORTED |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Network isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Background escape risk | LOW (no breakaway flags; kill-on-close) | MEDIUM (setsid/new group can escape) | MEDIUM (same) |
| Safe Trusted Shell? | **READY** | **READY** | **READY** |
| Safe Restricted Shell? | **CONDITIONAL** | **CONDITIONAL** | **NOT READY** |
| Safe Untrusted Shell? | **NOT READY** | **NOT READY** | **NOT READY** |

---

## 59. Expected Threat-Model Result

Evidence supports the expected result:

- **Trusted Shell**: possible cross-platform with per-call approval; no
  filesystem/network isolation claim.
- **Restricted Shell**: conditional / platform-dependent; macOS not ready;
  Windows conditional on restricted-token capability; Linux conditional on
  acceptance of partial process-tree and `no_new_privs` limitations.
- **Untrusted Shell**: not ready anywhere.

---

## 60. Cross-Platform Consistency

A single `tool.shell` is product-compatible across platforms **if** its
security contract is defined as **Trusted/user-approved local shell**, not as
"sandboxed shell". The platform differences are then differences in physical
guardrails, not differences in the user contract.

---

## 61. Shell Naming

Avoid `sandbox_shell`, `safe_shell`, or any name implying sandboxing.
Prefer honest naming:

- capability id: `tool.shell`;
- model-facing name: `shell` (or `local_shell` if a more explicit name is
  preferred);
- description: "Run a local shell command with the user's OS authority. Not a
  filesystem or network sandbox."

---

## 62. Product Mode

Recommended: **disabled by default; enabled explicitly**.

The Builtin Shell plugin should be registered only when configuration
explicitly enables it. This avoids advertising/exposing arbitrary execution
unless the user opts in. When enabled, every call must be `RequireApproval`
and the policy must be present.

Alternative "registered always but policy-denied by default" is less safe
because the capability is still declared to providers and discoverable by the
model; it buys nothing over not registering it in M2C v1.

---

## 63. Preferred Default

Candidate: "BuiltinToolsPlugin registers Shell only if explicitly enabled by
configuration. Default: disabled. When enabled: every call RequireApproval."

Accepted. This matches the plugin architecture: a plugin can be added or not
added at `RuntimeBuilder` time; capability declarations come from the plugin
manifest. No production code is changed in this review.

---

## 64. Config

Configuration concept only (no implementation):

- `tools.shell.enabled = true/false` (default `false`);
- `tools.shell.executable` optional explicit path (default: platform shell);
- `tools.shell.max_runtime` optional (default 30s, max 5m);
- `tools.shell.max_script_bytes` optional (default 64 KiB);
- `tools.shell.output_limit_bytes` optional (default 64 KiB stdout/stderr).

No raw environment variable unless the current CLI config convention
eventually prefers env; no vendor-specific config.

---

## 65. Approval UI Contract

The UI/CLI/Gateway must be able to show:

- exact command/script (raw, full, newlines preserved);
- shell executable / shell kind;
- cwd;
- platform;
- timeout;
- resource limits (when configured);
- environment mode (`Clear`/`Explicit`) and the actual variables;
- network isolation state (`UNSUPPORTED` / none);
- filesystem isolation state (`UNSUPPORTED` / none).

Optional: risk reason. No hidden fields that can change after approval.

---

## 66. Command Hash / Fingerprint

Approval should bind a canonical hash/fingerprint over at least:

- tool id (`tool.shell`);
- command/script UTF-8 bytes;
- cwd;
- environment mode + explicit env entries;
- isolation requirement/profile;
- limits (timeout, output bounds, resource limits);
- shell kind/executable.

The current approval lifecycle has no operation identity. M2C-A must add it;
M2C-T reuses it. Do not create a parallel mechanism.

---

## 67. Time Limits

Recommended concrete initial defaults:

- default runtime: **30s** (reuses current `ProcessLimits::default()`);
- hard maximum configurable: **5 minutes (300s)**;
- approval-visible;
- no no-timeout default.

---

## 68. Memory / Process Limits

Windows can enforce strongly; Unix has partial limits. There is no
cross-platform parity.

- **Trusted Shell** should request only capabilities common enough to run on
  all platforms: `StructuredSpawn`, `ExplicitCwd`, `Timeout`, `StdoutLimit`,
  `StderrLimit`, `EnvironmentIsolation`, `ProcessTreeContainment Partial`,
  `FailClosedPreExecutionContainment`. It must **not** request
  `ProcessTreeContainment Enforced` on Unix, or it would fail closed on
  Linux/macOS.
- Optional memory/process-count limits may be configured but are not baseline.
- **Restricted Shell** requires memory/process-count at least `Partial` and
  sets configured limits when selected.

Do not lie by lowering `ProcessTreeContainment: Enforced` to `Partial` just to
launch Shell. Instead use the Trusted Shell requirement that honestly matches
its threat model. Untrusted Shell must require `Enforced` and remains
unavailable on Unix.

---

## 69. Isolation Requirement Semantics

`TrustedShellRequirement` is not `IsolationProfile::Restricted` and is not
`IsolationProfile::Untrusted`. It is an explicit, honest requirement for a
user-approved shell:

```
StructuredSpawn Enforced
ExplicitCwd Enforced
Timeout Enforced
StdoutLimit Enforced
StderrLimit Enforced
EnvironmentIsolation Enforced
ProcessTreeContainment Partial
FailClosedPreExecutionContainment Enforced
```

The same requirement is satisfiable on Windows, Linux, and macOS.

`RestrictedShellRequirement` adds:

```
MemoryLimit Partial
ProcessCountLimit Partial
PrivilegeReduction Partial
```

Unsatisfiable on macOS (`PrivilegeReduction UNSUPPORTED`) and conditionally
satisfiable on Windows (`PrivilegeReduction` runtime-dependent).

`UntrustedShellRequirement` is the existing `IsolationProfile::Untrusted`
preset and is unsatisfiable on all three platforms.

---

## 70. Trust Is Threat Model, Not Security Level Label

**Trusted** means "the user accepted the command's authority." It does **not**
mean "the command is intrinsically safe." The Trusted Shell product contract
must not imply that approval makes the command safe.

---

## 71. Command Review

Approval UI should display the raw command exactly. Do not "prettify" in a way
that hides newlines, `&&`, `;`, pipes, or redirections. Also display an
escaped/repr form for disambiguation. No truncation of the approved command
unless the user can expand the full content. Execution binds the full
bytes/text.

---

## 72. Multiline Script

Future Shell may receive multiline scripts. Approval display must preserve
newlines. Command execution must bind full bytes/text.

---

## 73. Unicode

Approval binding must use stable UTF-8 representation. No normalization that
changes command semantics.

---

## 74. Shell Injection

Because the command is intentionally shell code, "shell injection" is not the
same threat as Repo tool args. But if any structured variables are
interpolated into the script, that becomes injection risk. Initial M2C should
avoid string interpolation APIs; it should accept a complete script as one
unit.

---

## 75. Result Contract

Future tool result must include:

- exit code / termination reason;
- stdout;
- stderr;
- truncated flags;
- timed out flag.

Reuse `ProcessResult` semantics. Do not hide nonzero exit as an infrastructure
error.

---

## 76. Network Result

Do not pretend to know whether the command used the network. No process
network auditing currently exists. No `network_used=false` field unless
proven.

---

## 77. File Changes

Likewise, no claim about which files the command modified unless separately
traced. Not part of v1.

---

## 78. SovereignControl

M1C donor audit deferred SovereignControl. Do not introduce it simply for
Shell. Canonical Governance is sufficient unless an actual gap is proven.

---

## 79. Donor Audit

Source:
`origin/master:reconstruction_v2/crates/apeireth-tools/src/builtin/shell.rs`
and `origin/master:reconstruction_v2/crates/apeireth-tools/src/sandbox.rs`.

| Piece | Classification |
| --- | --- |
| Raw `command` string interface | ADAPT as `tool.shell` input |
| `cmd /C` / `sh -c` invocation idea | ADAPT only as ShellCapability -> ProcessExecutor args; spawn itself REIMPLEMENT via canonical ProcessExecutor |
| Presets (`git-log-recent`, `echo-text`) | DROP (unnecessary string-assembly surface) |
| `args: Option<Vec<String>>` preset args | DROP |
| `sanitize_input` blacklist | DROP (blacklist theater) |
| Direct `tokio::process::Command` spawn | DROP (use `ProcessExecutor`) |
| `wait_with_output` with no timeout/output bound | DROP (use bounded `ProcessLimits`) |
| No cwd | DROP (require explicit cwd) |
| Inherited ambient env | DROP (require Clear/Explicit) |
| No governance/approval integration | DROP (must be governed) |
| Description claims "sandbox restrictions" | Security mistake; not ported |
| `PlatformSandbox` never assigned to shell child | Security mistake; not ported |
| Tests: echo dynamic command, echo preset, destructive rejection | ADAPT tests into harmless ProcessExecutor-based E2E; reject blacklist test concept |

Donor conclusion: the master ShellTool was **not** sandboxed despite its
description. Its safety was a blacklist over destructive strings plus an
unattached `PlatformSandbox`. That architecture is explicitly rejected.

---

## 80. Master Security Claim

Confirmed: donor `ShellTool::definition()` describes itself as
"Executes shell commands safely across Windows, Linux, and macOS with sandbox
restrictions", while `ShellTool::execute` spawns via `tokio::process::Command`
and never calls `PlatformSandbox::assign_process`. The sandbox field was
effectively decoration. This is recorded as an architecture mistake and
reinforces what must not be ported.

---

## 81. Readiness Decision Tree

```text
Approval lifecycle complete?
  NO  -> M2C-A — Canonical Approval Resume Lifecycle
  YES -> Is Trusted Shell contract acceptable?
           YES -> M2C-T — Cross-Platform Trusted Shell Capability
           NO  -> list blocker
```

Restricted/Untrusted implementation remains separate later phases.

---

## 82. No Network Blocker for Trusted Shell?

**No.** Absence of process `NetworkIsolation` does not necessarily block a
user-approved Trusted Shell. The OS terminal the user already runs also has
network access. The relevant question is whether the product accurately
represents authority and requires explicit user approval. For Trusted Shell:
yes. For Untrusted Shell: the same absence is a hard blocker.

---

## 83. No Filesystem Blocker for Trusted Shell?

**No.** Absence of filesystem isolation does not necessarily block a
user-approved Trusted Shell. The threat model is "user-approved local command",
not "autonomous untrusted model command". The product must say plainly that the
command has the user's filesystem authority. For Untrusted Shell, absence of
filesystem isolation is a hard blocker.

---

## 84. Untrusted Remains Strict

For Untrusted Shell:

- no filesystem isolation -> hard blocker;
- no network isolation -> hard blocker;
- Unix process-tree escape -> hard blocker;
- Linux privilege reduction only PARTIAL, macOS UNSUPPORTED -> hard blocker.

Do not weaken this to get feature parity.

---

## 85. Autonomous Use

Trusted Shell must **not** automatically become autonomous just because the
user enabled the capability globally. "Enabled" is separate from "approved per
invocation". Autonomous/background `tool.shell` is **not** allowed in v1.

---

## 86. Architecture Regression

The review recommends **no change** to:

- Runtime sole orchestration;
- `ToolCapability` boundary;
- `ProcessExecutor` boundary;
- Governance tri-state;
- `PluginRegistry` / `CapabilityRegistry` source of truth.

No second shell runtime.

---

## 87. Document Only

Primary output file:
`docs/01-architecture/m2c-shell-readiness-review.md`.

`ARCHITECTURE.md` receives a short pointer/rule update only if necessary.
No production implementation. No architecture assertion tests are added in
this review because the existing process-boundary invariant tests already
freeze the relevant "no shell in ProcessExecutor" rule.

---

## 88. Commit

Expected commit message:

```text
docs(architecture): review canonical shell readiness
```

---

## 89. Validation

If only docs change, run:

```text
cargo check --workspace --locked -j 4
cargo test -p apeireth-tools-canonical --locked
cargo test -p apeireth-governance --locked
cargo test -p apeireth-runtime --locked
cargo test -p apeireth-plugin --locked
```

---

## 90. Push Safety

Before push:

```text
git fetch origin
git log --oneline HEAD..origin/reconstruct_v2
```

If remote advanced: STOP. No merge/rebase/force.

If safe:

```text
git push origin reconstruct_v2
```

Then verify `HEAD == origin/reconstruct_v2` and working tree clean.

---

## 91. Final Report

See the final report at the end of the review process.
