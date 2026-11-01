# apeireth-guard

> Engine layer: a two-stage agent behaviour-chain safety classifier, exposed through the canonical `GovernanceHook` interface.

`apeireth-guard` sits in the **engine** layer of the Apeireth engine workspace. It
provides a deterministic fast path for immediate risk rejection, combined with a
multi-step behaviour-chain guard for compound risk, escalation and data-egress
tracking.

## The two stages

- **Stage A — Fast Guard** (`fast_guard.rs`): deterministic, sub-millisecond risk
  rejection on a single action.
- **Stage B — Behaviour Chain Guard** (`chain.rs`, `chain_guard.rs`): multi-step chains
  for compound risk, escalation and egress.

The two are combined by `fusion.rs` (`DecisionFusion`) into the final decision.

## What is inside

- `hook.rs` — `BehaviorChainGuardHook`, the canonical `GovernanceHook` integration,
  plus `SessionBehaviorSummary` / `TurnRiskSummary`
- `classifier.rs` — `ChainRiskClassifier`, `JointRiskClassifier`, `ThresholdClassifier`,
  `ModelCalibration`, `NoClassifier`, artifact hashing and stamping helpers
- `features.rs` / `features_v2.rs` — `AgentChainFeatureV1` / `AgentChainFeatureV2`
- `decision.rs` / `enforcement.rs` — `GuardDecision`, `GuardStage`, `EnforcementDirective`
- `observation.rs` — `SafetyObservation`, `DataSensitivity`, `ResourceClass`,
  `SourceClass`, `SinkClass`
- `intent.rs` — `IntentInterpreter`, `IntentAlignmentGuard`, negation-aware operation
  extraction and trusted-intent constraining
- `command_effect.rs` — `CommandEffectAnalyzer`
- `dataset.rs` — desensitised ML dataset collection (`GUARD_DATASET_V3`)
- `oracle.rs` / `scenario.rs` — scenario oracle for evaluation
- `session.rs` / `snapshot.rs` / `introspection.rs` — session-scoped state, snapshots and
  the dry-run / status DTOs
- `immune_memory.rs` — **pure kernel, not wired** into the runtime decision path yet

## Honest limits

- The behavioural-mechanism family (`immune_memory`) is implemented as a kernel module
  but is **not yet wired** into the runtime decision path.
- Threshold and classifier artifacts must be present and calibrated; `NoClassifier`
  is the explicit no-model baseline rather than a silent default.

## Install

```toml
[dependencies]
apeireth-guard = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/guard>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/guard/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
