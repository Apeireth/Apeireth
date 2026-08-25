# M2A — Canonical Simple Tool Capability Ports

Status: complete
Branch: `reconstruct_v2`
Starting HEAD: `d9796345`
Donor: `origin/master:reconstruction_v2/crates/apeireth-tools/src/builtin/`

M2A took three low-risk tool implementations from the master donor and adapted
them to the frozen canonical `ToolCapability` / `Plugin` / `CapabilityRegistry`
architecture. It did **not** port the donor `Tool` trait, `ToolRegistry`, or
runtime tool wiring.

## Tools ported

| Tool | Donor path | Strategy | Canonical implementation | Risk metadata | Status |
| --- | --- | --- | --- | --- | --- |
| Filesystem | `builtin/filesystem.rs` | ADAPT | `apeireth-tools-canonical::filesystem` | `medium` | Read/list/stat ported; write/delete deferred |
| Search | `builtin/search.rs` | ADAPT | `apeireth-tools-canonical::search` | `low` | Local deterministic substring search ported |
| Repo | `builtin/repo_tools.rs` | REIMPLEMENT | `apeireth-tools-canonical::repo` | `low` | Fixed read-only git operations ported |

## Donor provenance

### Filesystem

- Donor path: `reconstruction_v2/crates/apeireth-tools/src/builtin/filesystem.rs`
- Reused: operation model (`read`/`list`), configurable root concept, test
  structure.
- Adapted: read-only subset (`read`, `list`, `stat`), canonicalized path
  containment instead of a string `..` check, explicit file-size limit,
  structured UTF-8 errors, deterministic directory ordering, JSON results.
- Rejected: donor `write`/`delete` operations in M2A.
- Deferred: write/delete/rename/copy until M2B sandbox enforcement exists.

### Search

- Donor path: `reconstruction_v2/crates/apeireth-tools/src/builtin/search.rs`
- Reused: literal case-insensitive substring semantics, default/max result
  counts, 500KB per-file content limit, depth bound, hidden/target/node_modules
  skipping.
- Adapted: root-confined search path with canonicalization, deterministic
  (path, line, text) ordering, explicit `truncated` flag, `.git` also skipped,
  binary files skipped without fatal error.
- Rejected: none; donor search had no tests, so tests were newly written.
- Deferred: regex search, ignore-file support, and structured binary detection
  are not in M2A.

### Repo

- Donor path: `reconstruction_v2/crates/apeireth-tools/src/builtin/repo_tools.rs`
- Reused: fixed read-only git command set (`status`, `diff`, `log`, `branch`,
  `summary`).
- Adapted: explicit repository root (`git -C`), no arbitrary `args` field,
  fixed argument construction only, bounded output with explicit truncation,
  structured errors for non-git directory and git failures.
- Rejected: donor `args: Option<Vec<String>>` arbitrary-argument path.
- Deferred: any mutation operation, arbitrary revision/path selection, timeout
  and process containment (M2B).

## Security limitations

- Filesystem is read-only and root-confined by canonicalized path containment,
  but it is **not** a process/filesystem sandbox. M2B adds actual containment.
- Search reads local UTF-8 text files only; it is heuristic about binary files
  (invalid UTF-8 files are skipped).
- Repo runs fixed read-only `git` subprocesses. It is not a shell tool and
  accepts no arbitrary git arguments, but there is no process timeout or OS
  containment yet.
- Shell, browser, fetch/network, and MCP tools are not ported.
- No real sandbox is claimed.
