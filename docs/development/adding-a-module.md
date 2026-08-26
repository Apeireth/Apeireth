# Adding a Module

This is the quick decision guide for contributors.

## Where does new work go?

| Work | Location | Rule |
| --- | --- | --- |
| New product feature module | `crates/modules/<name>/` | Must be built on canonical engine/foundation contracts |
| New model-facing tool capability | `crates/capabilities/tools/` module | Do NOT create `apeireth-tool-foo` by default |
| New provider capability | `crates/engine/provider/` | Vendor translation and vendor HTTP only |
| New adapter surface | `crates/adapters/<name>/` | Decode/encode and call Runtime; no orchestration |
| New stable primitive | `crates/foundation/<name>/` | Requires architecture review; inward-only deps |
| New durable engine machinery | `crates/engine/<name>/` | Requires architecture review |
| Historical/donor code | `legacy/donor/` | No new production development |

## New crate rule

A new crate requires:

- clear ownership boundary
- independent dependency direction
- meaningful isolation
- a test/build reason

A new feature does not automatically deserve a crate. Prefer a module inside an
existing canonical crate when boundaries do not demand a separate package.

## Multi-developer rule

Module developers may not independently create:

- Runtime
- Registry
- ApprovalManager
- Provider router
- HTTP transport
- ProcessExecutor
- Governance pipeline
- SessionStore architecture

without architecture review.

## Dependency layers

```text
foundation
   ↓
engine primitives / plugin contracts
   ↓
capabilities + modules
   ↓
runtime composition
   ↓
adapters
```

- foundation must not depend on modules/adapters.
- engine may depend on foundation.
- capabilities may depend on foundation and approved engine primitives.
- modules may depend on canonical contracts, not adapters or legacy. The
  `crates/modules/` directory currently has no active product module; historical
  Companion code is donor-only under `legacy/donor/apeireth-companion`.
- adapters may depend inward.
- product code must not depend on `legacy/`.

## Module ownership template

```text
Module:
Owner category:
Canonical dependencies:
Capabilities exposed:
Runtime integration:
Governance hooks:
Persistence:
External I/O:
Legacy donor source:
Forbidden dependencies:
```
