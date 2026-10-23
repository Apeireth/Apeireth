# Apeireth Guard 3.0 — Intent-Aware Behavioral Defense

Status report for the current implementation on `feature/cognitive-infrastructure-vnext`.

## Architecture

The safety path is now:

```text
user-visible turn input
  -> RuleIntentInterpreter (once at turn start)
  -> TaskIntentEnvelopeV1 + TurnSecurityContext
  -> Runtime::execute_outcome
  -> GovernanceRequest (same context on completion and tool calls)
  -> FastGuard + IntentAlignmentGuard + ChainGuard
  -> AgentChainFeatureV2
  -> optional local classifier
  -> DecisionFusion
```

The raw request is transient interpreter input. It is not written to Guard
dataset records, behavior chains, or feature vectors.

## Implemented

- `TaskIntentEnvelopeV1` is versioned, trace-bound, provenance-labelled, and
  separates requested task semantics from intrinsic action risk.
- `RuleIntentInterpreter` recognizes common Chinese and English read, edit,
  test, network, publish, delete, credential, and shell expressions.
- Intent privilege monotonicity is enforced by `constrain_to_trusted`; a model
  proposal can only narrow an already trusted envelope.
- CLI and gateway create the envelope before invoking the canonical Runtime.
  Runtime carries the immutable context through completion and tool governance.
- `CapabilitySafetyDescriptor` and `CapabilitySafetyRegistry` provide operation,
  resource, source, sink, external, destructive, persistence, network, and
  credential semantics. Unknown capabilities use conservative fallback metadata.
- Behavior graph action nodes now carry operation/resource/data semantics,
  alignment, approval/execution state, persistent/destructive effects, and a
  redacted semantic effect fingerprint. Additional semantic edge types are
  available, including `SameEffect`.
- `IntentAlignmentGuard` detects read-only mutation, unrequested egress,
  credential access, deletion, shell expansion, protected-surface tampering,
  and unknown external effects. Alignment can raise risk but cannot lower a
  deterministic denial.
- `AgentChainFeatureV2` combines V1 chain counters with intent, alignment,
  operation, bounded taint, effect repetition, and cross-turn summary features.
- `JointRiskClassifier` consumes the versioned Intent × Behavior × Dataflow
  feature schema through a validated local JSON artifact. It is connected to
  the CLI production governance builder through `APEIRETH_GUARD_ML_MODEL` and
  `APEIRETH_GUARD_ML_MODE`.
- The default model mode is `SHADOW`: predictions are observable in Guard
  status/events and dataset records, while deterministic Guard decisions stay
  authoritative. Advisory/enforce modes are explicit opt-ins.
- Session monitoring keeps bounded counters and risk trend; repeated sensitive
  probing across turns is contained without retaining the prior full DAG.
- Dataset classification rows are `guard-dataset-v3` and store only structured
  features, taxonomy labels, and exact action correlation. Lifecycle outcomes
  remain normalized and action-scoped.
- Shell observations now emit structural features only: command family,
  counts, nesting, networking, privilege, recursion, destructive/persistence,
  download-execute, encoding, credential-path, and system-path indicators.
- `scripts/guard_ml/scenarios.jsonl` contains 32 bounded, sanitized scenarios
  spanning benign, hard-negative, scope-creep, exfiltration, credential,
  destructive, retry-bypass, tool-switching, cross-turn-probing, and
  security-tamper cases.
- `scripts/guard_ml/train_shadow.py` trains the standard-library shadow
  logistic model and exports `artifacts/guard-joint-shadow-v0.json` plus
  metrics. The generated fixture scores 32/32 labels correctly, but is a
  synthetic shadow evaluation and is not production-calibrated.
- Dry-run requests accept an optional intent envelope for intent/action tests.

## Truth status

| Area | Status | Evidence |
| --- | --- | --- |
| Intent envelope | `PRODUCTION_WIRED` | governance protocol + Runtime request context |
| Rule intent interpreter | `PRODUCTION_WIRED` | CLI and gateway turn-start construction |
| Intent alignment | `VERTICAL_TESTED` | Guard integration scenarios |
| Capability descriptors | `IMPLEMENTED` | conservative local fallback; provider metadata adapter remains open |
| Behavior graph V2 fields | `IMPLEMENTED` | action nodes, semantic edges, fingerprints |
| Cross-turn summary | `VERTICAL_TESTED` | repeated sensitive probing test |
| Dataset v3 | `IMPLEMENTED` | sanitization and lifecycle tests |
| Joint local classifier | `SHADOW_ONLY` | validated JSON artifact + local inference + shadow scenario test |
| Training/scenario pipeline | `IMPLEMENTED` | 32 sanitized scenarios, exporter, and metrics artifact |
| ONNX model backend/artifact | `DEFERRED` | JSON backend is intentionally dependency-free; ONNX remains future work |
| Desktop Inspector V3 | `DEFERRED` | status DTO exposes schema and mode fields; UI presentation remains |
| Remote CI / GitHub evidence | `DEFERRED` | not run from this local change |

## Threat model covered

The deterministic path covers intent scope creep, read-only-to-write changes,
credential access, sensitive-to-external flow, unknown external tools, retry and
alternate-tool escalation, cross-turn sensitive probing, destructive chains, and
protected governance/control-surface tampering.

It is not a malware sandbox, general content moderation layer, full DLP
platform, SIEM, or general jailbreak classifier.

## Branch and integration policy

```text
Target branch:
feature/cognitive-infrastructure-vnext

Main integration:
NOT REQUESTED

Guard 3.0 status:
NEXT BRANCH ONLY
```

No separate Guard v3 branch was created, and no PR targeting `main` is part of
this work.

## Verification

```text
cargo check -p apeireth-guard -p apeireth-runtime -p apeireth-cli -p apeireth-gateway --all-targets --locked  PASS
cargo test -p apeireth-guard --all-targets --locked                                                     PASS
cargo clippy -p apeireth-guard -p apeireth-runtime -p apeireth-cli -p apeireth-gateway --all-targets --locked -- -D warnings  PASS
```

The full workspace gates, frontend gates, remote CI, ONNX export/load, and
performance percentile report remain to be run or implemented and are
intentionally not represented as green here.
