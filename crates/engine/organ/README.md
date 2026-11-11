# apeireth-organ

> Engine layer: the implementation side of the nine-organ cognitive surface — the `OrganTrait` boundary lives in `apeireth-plugin`, the implementations live here.

`apeireth-organ` sits in the **engine** layer of the Apeireth engine workspace. The
`OrganTrait` abstraction is a foundation-layer contract (defined in `apeireth-plugin`);
this crate provides the concrete implementations that the runtime layer composes.

## The nine organ implementations

Each of these implements `OrganTrait`:

- `curiosity.rs` — E4 Curiosity
- `hypothesis.rs` — F4 Hypothesis
- `value_cases.rs` — F6 Value Cases
- `emotion_memory.rs` — F1 Emotion Memory
- `world_model.rs` — W1 World Model (**wired to a real LLM**)
- `causal_world_model.rs` — W2 Causal World Model (**wired to a real LLM**)
- `causal_world_model_edges.rs` — W3 Causal Edge Mining (deterministic, no LLM)
- `emergence.rs` — E7 Emergence (rhythm + boundary gating, deterministic decision path)
- `memory.rs` — Memory Merger

## Supporting modules

`attachment.rs`, `care.rs`, `circadian.rs`, `context_assembly.rs`, `experience_growth.rs`,
`goal.rs`, `homeostasis.rs`, `morphology.rs`, `motivation.rs`, `neuromodulation.rs`,
`play.rs`, `prompt_assembly.rs`, `ritual.rs`, `social_repair.rs`, `tone.rs`.

## Honest limits

- `E7 Emergence` is **deterministic and does not call an LLM**: its `llm_factory()`
  returns `None`. It is not the five-state machine
  (`Idle/Draft/Proposed/Ratified/Active`) — that state machine belongs to the evolution
  module, not to emergence. `PolicyStage` here is a forward declaration reserved for a
  future evolution integration.
- The Memory Merger is a **new v2 abstraction**; it has no v1 counterpart module and
  follows the v1 extraction-service dedup / weight / persist algorithm shape.
- Deterministic organs (E4, F6, F1, W3, E7) do **not** pretend to call an LLM.

## Install

```toml
[dependencies]
apeireth-organ = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/organ>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/organ/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
