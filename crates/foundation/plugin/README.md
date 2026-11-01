# apeireth-plugin

> Foundation layer: the canonical plugin and capability model — a plugin is a **capability provider**, and the registries here are the single source of truth for what the runtime can do.

`apeireth-plugin` sits in the **foundation** layer of the Apeireth engine workspace. It
defines the contracts that every other layer builds on: how a plugin declares itself,
how a capability is addressed, and how the runtime looks up "who owns this capability id".

## Why it exists

The workspace it converges had three separate `PluginRegistry` types and two separate
tool registries. None was wrong on its own; the defect was that no single one could
answer *"what can this runtime do"*. This crate replaces that with one registry
(`PluginRegistry`) plus typed views over it (`PluginManager`), so there is exactly one
place where that question is answered.

## What is inside

- `plugin.rs` / `manifest.rs` — plugin identity and declaration surface
- `registry.rs` — `PluginRegistry`, the owner registry
- `manager.rs` — `PluginManager`, typed views over the registry
- `capability.rs` — capability descriptor model
- `tool.rs` — the `ToolCapability` contract that the builtin tool crate implements
- `provider.rs` / `llm_factory.rs` — provider and LLM factory boundaries
- `llm_orchestration_mirror.rs` — `MirrorLlmFactory`, the bridge from the plugin LLM
  trait to the orchestration LLM trait (one implementation shared by product and tests)
- `self_assessment.rs` — `SelfAssessmentCache` (canonical single source of truth)
- `memory_backend.rs` / `perception.rs` / `perception_backend.rs` — backend contracts
- `organ.rs` — the `OrganTrait` boundary that the engine organ crate implements
- `alias.rs`, `availability.rs`, `bounds.rs`, `credentials.rs`, `extension.rs`,
  `experience.rs`, `preference.rs`, `semver.rs`, `watch.rs` — supporting contracts

## Scope and honest limits

- **In-process, static plugins.** Dynamic library loading, WASM, hot reload, remote
  plugins and a marketplace are out of scope and are **not implemented**.
- The capability id-to-owner index (`CapabilityIndex`) is crate-internal; the public
  surface is `PluginRegistry` / `PluginManager`.

## Install

```toml
[dependencies]
apeireth-plugin = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/plugin>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/plugin/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
