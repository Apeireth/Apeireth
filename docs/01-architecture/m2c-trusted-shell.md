# M2C-T — Cross-Platform Trusted Shell Capability

Status: complete (implementation; three-OS validation in M2C-XV)
Branch: `reconstruct_v2`

## 1. Product contract

`tool.shell` is a **Trusted Shell**. A model proposes a local shell script. A
human sees the exact effective invocation and explicitly approves it. Apeireth
then executes it once through the canonical `ProcessExecutor`.

Trusted Shell is **not** sandboxed. It runs with the user's OS account
filesystem and network authority. Filesystem isolation and network isolation
remain `UNSUPPORTED` on all platforms and are not claimed.

## 2. Default

Shell is **disabled by default**. `BuiltinToolsPlugin::new` registers only
`tool.filesystem`, `tool.search`, and `tool.repo`. Shell is registered only
when `BuiltinToolsOptions { shell: Some(TrustedShellConfig) }` is explicitly
provided.

## 3. Capability

- Capability id: `tool.shell`
- Model-facing name: `shell`
- Risk metadata: `"high"`

The declaration says: *"Executes a platform-native local shell command after
explicit user approval. Runs with the user's OS account authority; not a
filesystem or network sandbox."*

## 4. Schema

Model-supplied parameters:

```jsonc
{
  "command": "string (required, <= 64 KiB UTF-8)",
  "cwd": "string (optional, relative to trusted workspace root)",
  "timeout_ms": "integer (optional, default 30000, max 300000)"
}
```

Model cannot supply environment, shell executable, network mode, sandbox mode,
isolation requirement, or resource caps. Security configuration is
application-controlled.

## 5. Platform shell

| Platform | Default shell | Arguments |
| --- | --- | --- |
| Windows | `cmd.exe` | `/D /S /C <command>` |
| Linux | `/bin/sh` | `-c <command>` |
| macOS | `/bin/sh` | `-c <command>` |

PowerShell is deferred. `bash`/`zsh` are not assumed.

## 6. Environment

Never `EnvironmentSpec::Inherit`. The Shell bootstrap builds an explicit
minimal environment:

- Windows: `SystemRoot`, `WINDIR`, `TEMP`, `TMP`, `PATH`, `PATHEXT`,
  `COMSPEC` when present in the parent; no `USERPROFILE`/`HOMEDRIVE`/
  `HOMEPATH`.
- Linux/macOS: `PATH=/usr/local/bin:/usr/bin:/bin`, `TMPDIR=/tmp`,
  `LANG=C.UTF-8`; no `HOME`.

No provider API keys, GitHub tokens, AWS credentials, cloud credentials, proxy
credentials, or Apeireth internal secrets are inherited.

## 7. Working directory

Every invocation has an explicit cwd. The trusted config provides a workspace
root. A relative `cwd` is resolved under the root and canonicalized. Paths
that do not exist, are not directories, or escape the root (including symlink
escape) are rejected.

Explicit cwd is execution context, **not** filesystem sandboxing.

## 8. Limits

- Script size: 64 KiB UTF-8 default/max (configurable).
- Timeout: 30s default, 5m maximum, approval-visible.
- Output: 64 KiB stdout and 64 KiB stderr defaults; truncation flags returned.
- No zero or unlimited timeout.

## 9. Process requirement

`ShellTool` builds an explicit `IsolationRequirement`:

```text
StructuredSpawn Enforced
ExplicitCwd Enforced
Timeout Enforced
StdoutLimit Enforced
StderrLimit Enforced
EnvironmentIsolation Enforced
ProcessTreeContainment Partial
FailClosedPreExecutionContainment Enforced
```

No `FilesystemIsolation`, `NetworkIsolation`, or `PrivilegeReduction` is
required for Trusted mode.

## 10. Approval freeze and frozen execution

`ToolCapability` gained one optional canonical hook pair:

```rust
fn freeze_invocation(
    &self,
    call: &ToolCall,
) -> Result<Option<FrozenInvocation>, ToolResult>;

async fn invoke_frozen(
    &self,
    call: &ToolCall,
    frozen: Option<&FrozenInvocation>,
) -> ToolResult;
```

`FrozenInvocation` separates two concepts:

- `payload` — the opaque, versioned execution payload owned by the tool.
- `display` — the redacted, human-facing view used by `PendingApprovalView`.

`ShellTool` freezes a versioned `ShellFrozenInvocation` with the exact
effective execution fields:

- `version` (`1`)
- `shell_executable` (the selected shell path)
- `shell_args` (exact argv, including the exact script)
- `cwd` (canonicalized path)
- `timeout_ms` (effective timeout)
- `max_stdout_bytes` / `max_stderr_bytes`
- `environment` (actual explicit environment **values**)
- `isolation` (the exact `IsolationRequirement`)

The display view contains environment variable **names only**, never values.

Approved shell dispatch executes `ShellTool::invoke_frozen`, which deserializes
the shell-owned frozen payload and builds a `ProcessRequest` **from frozen
fields only**. It never calls `resolve_cwd_for`, `selected_shell`,
`minimal_environment`, or `resolve_timeout_ms` during approved execution.
If the frozen payload cannot be deserialized or its version is unsupported,
the tool returns a `ToolResult` error (`FrozenInvocationUnavailable`) instead
of silently using current configuration.

Invalid shell requests (`invalid cwd`, `empty command`, `oversized command`,
`invalid timeout`) return a `ToolResult` error from `freeze_invocation` and
therefore never create a `PendingApproval`.

## 11. Result contract

A shell result contains:

```jsonc
{
  "exit_code": 0,
  "timed_out": false,
  "stdout": "...",
  "stderr": "...",
  "stdout_truncated": false,
  "stderr_truncated": false
}
```

Nonzero exit is a normal shell result, not a process error. Spawn/containment
errors are returned as structured `ToolResult` failures.

## 12. Non-goals

No interactive shell, no PTY, no stdin streaming, no background/detached
process API, no persistent tasks, no SSH, no preset shell, no command
blacklist, no command rewriting/sanitizing, no PowerShell v1.

## 13. Tests

- `crates/apeireth-tools-canonical/src/shell.rs`: schema, size, timeout, cwd
  escape, declaration honesty.
- `crates/apeireth-tools-canonical/tests/shell_execution.rs`: real harmless
  echo, cwd, nonzero exit, bounded timeout, unicode.
- `crates/apeireth-runtime/tests/canonical_shell_approval_e2e.rs`: full
  runtime path from provider tool call -> governance `RequireApproval` ->
  pending approval -> approve -> real shell execution -> provider
  continuation; reject path; invalid shell never pends.
- `crates/apeireth-tools-canonical/src/shell.rs`: freeze validation, redacted
  display (environment names only), and frozen cwd execution after config
  mutation.
