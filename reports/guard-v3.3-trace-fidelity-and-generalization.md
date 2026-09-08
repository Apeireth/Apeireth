# Guard 3.3 Trace Fidelity and Semantic Generalization

**Assessment date:** 2026-09-08  
**Branch:** `feature/cognitive-infrastructure-vnext`  
**Recorded HEAD:** `9289f4f0c0e729f4be62500403bdab65b7d75c4b`

## Decision

`REAL_SHADOW_EVIDENCE = UNAVAILABLE`  
`CI_VERIFIED = NO`  
`FREEZE_READY = NO`  
`NOT_FREEZE_READY`

This is a conservative evidence decision. Synthetic evaluation and local tests do not substitute for real shadow traffic or a successful remote CI run tied to the final implementation SHA.

## Trace and action fidelity

The scenario runner now retains one `ScenarioActionResult` and one `FeatureSnapshot` for every governance action. Stable explicit action IDs are preserved; blank IDs receive deterministic generated IDs. `BehaviorChain::action_by_id` provides exact lookup. A turn creates an intent envelope, and one session is reused across turns so session history and cross-turn summaries remain observable. Sparse `trace_index` values preserve same-trace grouping and separate provider traces within one turn. Checked execution reports missing and unexpected semantic effects as `ScenarioTraceError`.

The compatibility fields on `ScenarioOutcome` remain last-action/last-turn summaries for older consumers. They are not treated as action-level ground truth; `execution_trace.actions` and `execution_trace.turns` are the authoritative fidelity-preserving records.

Focused Rust coverage passed: 20 Guard correctness tests and 8 Guard evaluation-integrity tests. Coverage includes oracle labels on every action, stable action lookup, duplicate/blank IDs, empty scenarios, unknown capabilities, trace boundaries, round overflow safety, multi-action traces, and cross-turn history.

## Independent semantic oracle

`SecurityScenarioOracle` is independent of the production interpreter, alignment guard, FeatureV2 extractor, snapshots, classifier predictions, and Guard decisions. Runtime semantic effects are derived from structured operation/resource/source/sink fields and explicit effect flags. Opaque effect fingerprints are not semantic truth. Expected and runtime effects are normalized and reconciled exactly per action.

## Dataset and provenance

The Rust exporter produced:

- 2,835 scenario rows;
- 2,850 action rows;
- 2,835 trace rows;
- 127 pair records;
- 35 families;
- 1,319 Chinese, 1,303 English, and 213 mixed-language scenarios.

The manifest, model, and metrics share dataset hash `c201a363f356c3903fdd3122970bdaa1e287333e40a12b79fbedb0a6cb5bfd57` and feature schema hash `065fca115611d552b1915dbed8ecdcc64cdb9848df8c9892286844bbb115cbc1`. Manifest counts are sourced from the generated action/trace/pair exports. Catalog provenance is the repository-relative source hash recorded in the metrics provenance.

No raw prompt, command, path, URL, tool output, memory body, secret, credential, token, password, or chain-of-thought is emitted in the evidence exports.

## Holdouts and pair evaluation

Family assignment is deterministic SHA-256 grouping shared by Rust and Python. Pair coverage reports complete same-split pairs only and records excluded non-evaluation members, ties, failures, missing members, and cross-split pairs. The regenerated test split contains 10/20 counterfactual pairs and 36/102 negation pairs; excluded pairs are explicitly reported rather than silently scored.

Counterfactual ordering is reported separately from transition accuracy. Because the current catalog does not contain an independent transition-outcome label, `counterfactual_accuracy = NOT_AVAILABLE_NO_TRANSITION_LABEL`; pairwise ordering remains the available metric. Current test pairwise ordering is 0.70 for counterfactual pairs and 0.4444 for negation pairs, with ties reported in coverage.

Canonical, shell-holdout, and plugin realizations are represented in the catalog. Shell actions use `holdout_group = tool_shell`; plugin actions are explicitly marked `tool_origin = plugin`; canonical training-side realizations remain present. Identity-ablation and richer effect-equivalent isolation remain deferred.

## Shadow, privacy, and drift

The analyzer self-test uses only the designated fixture and reports `TEST_ONLY`. The normal analyzer invocation finds no production shadow dataset and reports `REAL_SHADOW_EVIDENCE.status = UNAVAILABLE`; it does not fabricate production evidence. Privacy validation covers forbidden field names and suspicious values, bounded structures, non-finite numbers, and safe review projections. Real-versus-synthetic drift is therefore unavailable until approved real shadow input exists.

## CI and hygiene

The dedicated Guard 3.3 workflow covers export, Python compilation, evaluation, Guard tests, analyzer self-test, privacy checks, generated-file checks, hygiene, and artifact upload. Python cache cleanup is limited to known regenerable `__pycache__` directories. Existing workflow jobs were preserved. No remote CI result for the recorded SHA is available, so `CI_VERIFIED = NO`.

Repository hygiene is documented separately in `reports/local-repository-hygiene.md`. No destructive cleanup was performed.

## Final Closure (2026-09-08)

The closure pass improved clause-local negation semantics and added explicit counterfactual transition contracts to the evaluator. The extractor now evaluates all term occurrences instead of only the first match, preserves requested operations in an unrelated clause, and gives credential-value disclosure denials precedence. Focused coverage includes edit-without-push, tests-without-install, and credential-existence-without-value-read cases.

The counterfactual catalog now contains 20 structured contracts with `RiskIncrease` labels. Pair evaluation reports transition-labelled coverage, transition accuracy by label, score margins, ties, and privacy-safe diagnostics; it does not infer labels from classifier output. The broader requested effect-equivalent isolation, semantic-only identity ablation, and a >=200-pair negation benchmark remain open because the current checked-in catalog/data generator does not provide those contracts. No metric was fabricated to claim completion.

### Local validation evidence

- `cargo fmt --all -- --check`: PASS
- `cargo check --workspace --all-targets --locked`: PASS
- `cargo test --workspace --all-targets --locked`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo deny check`: PASS with existing configuration warnings
- `cargo audit`: PASS (recorded in local validation output)
- Companion desktop `pnpm test`: PASS (7/7 suites)
- Companion desktop `pnpm check`: PASS with 5 pre-existing Svelte warnings, 0 errors
- Companion desktop `pnpm build`: PASS
- Companion Tauri `cargo check --workspace`: PASS
- `python -m compileall scripts/guard_ml`: PASS
- Guard analyzer self-test and Guard all-target tests: PASS
- Runtime dependency tree contains no Python, LightGBM, ONNX runtime, or `apeireth-guard` dependency: PASS

### Final evidence boundary

`REAL_SHADOW_EVIDENCE = UNAVAILABLE` remains true: only synthetic exports and the designated analyzer fixture are present. `CI_VERIFIED = NO` until a successful remote run is tied to the final SHA. Therefore `GUARD_3_X_FREEZE_READY = NO` and `NOT_FREEZE_READY` remain the honest decision. The target branch is `feature/cognitive-infrastructure-vnext`; the Memory branch was not modified.


| Area | Status | Evidence / blocker |
|---|---|---|
| True family holdout | PASS | Deterministic Rust/Python family assignment and disjointness checks |
| Action/turn trace fidelity | PASS | Per-action snapshots, IDs, intents, traces, history, and checked reconciliation |
| Independent oracle | PASS | Source-independence tests and structured semantic mapping |
| Exact effect reconciliation | PASS | Missing/unexpected effect reporting and checked runner |
| Intent-template holdout | PASS | Catalog metadata and evaluator dimension report |
| Realization holdout | PARTIAL | Canonical/shell/plugin metadata present; broader direction matrix remains deferred |
| Effect-equivalent isolation | DEFERRED | Not yet first-class in the DSL |
| Pairwise ordering | PARTIAL | Explicit same-split coverage, ties, margins, and exclusions; negation remains weak |
| Counterfactual transition accuracy | BLOCKED | No independent transition-outcome label |
| Identity ablation | DEFERRED | No explicit ablation report yet |
| Artifact/schema provenance | PASS | Shared dataset/schema hashes and repository-relative provenance |
| Privacy invariants | PASS | Generated exports and analyzer checks |
| Real shadow evidence | BLOCKED | No approved production shadow dataset |
| Drift analysis | BLOCKED | Requires real shadow input |
| Local Rust/Python validation | PASS FOR FOCUSED GATES | Focused Guard tests and evaluator checks passed |
| Final-SHA remote CI | BLOCKED | No verifiable remote run |
| Freeze readiness | NOT READY | Evidence gates are intentionally conservative |

## Deferred items

1. Obtain approved real shadow evidence and run privacy-preserving drift analysis.
2. Add independent transition labels for counterfactual accuracy.
3. Add effect-equivalent group isolation and complete realization-direction metrics.
4. Add tool-identity removal ablation and confidence intervals for small groups.
5. Run and record remote CI for the final SHA.

**Final decision: `NOT_FREEZE_READY`.**
