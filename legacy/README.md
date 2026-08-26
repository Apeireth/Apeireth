# LEGACY / DONOR CODE IS NOT CANONICAL

Everything under `legacy/` is historical donor or archived code. It is kept for
functionality mining, compatibility reference, and migration comparison only.

## Rules

- NO NEW DEVELOPMENT HERE unless doing donor audit/history repair.
- Do not add production dependencies on `legacy/` crates.
- Do not register capabilities from `legacy/` crates.
- Do not import Runtime / Registry / Provider architecture from `legacy/`.
- Functionality may be selectively ported into canonical owners. Port
  semantics, not architecture.

## Layout

- `legacy/donor/` — historical donor implementations that may still be read or
  copied from during future functionality ports.
- `legacy/archived/` — obsolete historical code; no development expected.
- `legacy/frozen/` — historical reference code intentionally untouched.

## Current migration debt

`apeireth-runtime`, `apeireth-provider`, `apeireth-memory`,
`apeireth-gateway`, and `apeireth-cli` still path-depend on `legacy/donor`
crates for their non-canonical/legacy modules. This is tracked migration debt;
R0T does not refactor package internals.

## Canonical replacements (non-exhaustive)

| Legacy | Canonical |
| --- | --- |
| `legacy/donor/apeireth-tool-shell` | `crates/capabilities/tools` module `shell` |
| `legacy/donor/apeireth-tool-fetch` | `crates/capabilities/tools` module `fetch` |
| `legacy/donor/apeireth-tools` | `crates/capabilities/tools` |
| legacy protocol adapters | `crates/foundation/protocol` + `crates/engine/provider` |
| legacy registries | `crates/foundation/plugin` |

The package names under `legacy/donor/` are unchanged. Directory names reflect
status, not package identity.
