# Repository Layout

Physical topology matches the canonical architecture.

```text
Cargo.toml            # current product workspace (explicit members only)
crates/
├── foundation/       # stable contracts and policy vocabulary
│   ├── core/
│   ├── protocol/
│   ├── plugin/
│   ├── governance/
│   └── credentials/
├── engine/           # durable execution machinery
│   ├── runtime/
│   ├── provider/
│   ├── storage/
│   └── memory/
├── capabilities/     # model/runtime-facing actions
│   └── tools/        # package: apeireth-tools-canonical
├── modules/          # product feature modules
│   └── companion/
└── adapters/         # external surfaces
    ├── gateway/
    ├── cli/
    └── sdk/
legacy/
├── Cargo.toml        # separate legacy workspace (reference-only)
├── donor/            # historical donor implementations
├── archived/         # obsolete historical code
└── frozen/           # intentionally untouched historical reference
docs/
├── 01-architecture/  # current architecture contracts
└── development/      # contributor guides
```

## Physical vs logical

- Directory groups (`foundation`, `engine`, ...) express ownership category.
- Rust package names and import identities are unchanged.
- `apeireth-tools-canonical` lives at `crates/capabilities/tools` but its
  package name is still `apeireth-tools-canonical`.

## Workspace

- Root `Cargo.toml` explicitly lists current product packages.
- `legacy/` is excluded from the root workspace and has its own workspace.
- Current product packages still path-depend on some `legacy/donor` packages;
  that is tracked migration debt, not an architecture target.
