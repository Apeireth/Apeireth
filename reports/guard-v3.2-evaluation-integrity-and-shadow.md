# Guard 3.2 Evaluation Integrity and Shadow Readiness

**Assessment date:** 2026-09-07  
**Branch:** `feature/cognitive-infrastructure-vnext`  
**Base recorded before this work:** `d359868f8d3332af0a5f4d118c6e8ec26e598cf9`  
**Implementation commit before generated artifacts/report:** `f05351e47bfb1d4738340286e9b376799058774c`  
**Final commit:** to be recorded after this report commit.  
**Remote CI evidence for the final implementation SHA:** unavailable; `CI_VERIFIED = NO`.

## Executive result

`FREEZE_READY = NO` / `NOT_FREEZE_READY`.

The Guard 3.2 evaluation-integrity controls, independent oracle, true family holdout, artifact provenance, and shadow-analysis tooling are implemented and locally validated. The runtime default remains disabled classifier / `NoClassifier` (SHADOW semantics are preserved; no production Enforce default was introduced). Freeze is still blocked by the absence of real shadow evidence, lack of final-SHA remote CI evidence, weak synthetic generalization results in key holdouts, and a known multi-action scenario accounting limitation.

## Scope and branch hygiene

- Work stayed on `feature/cognitive-infrastructure-vnext`.
- No Guard branch was created.
- `main` was not modified, merged, or used as a PR target.
- No push was performed.
- No raw secret, credential, URL, absolute path, command payload, tool output, memory, or chain-of-thought content is included in the checked-in evaluation artifacts, report, or review queue.

## Guard 3.1 terminology correction

The historical Guard 3.1 split was a **within-family held-out instance split**. It did not hold out complete unseen families. The metrics keep this baseline under `within_family_held_out_baseline` and explicitly do not call it an unseen-family result.

Guard 3.2 uses deterministic SHA-256 family/group assignment. A family is assigned wholly to exactly one of `train`, `calibration`, `validation`, or `test`; isolation assertions reject overlap across all four partitions. The split report records family lists and row/class/language counts.

## Evaluation dimensions and taxonomy

The scenario catalog contains 2,655 rows across 35 families:

- risky: 1,253; benign: 1,402;
- languages: Chinese 1,259, English 1,243, mixed 153;
- true family split: train 821, calibration 184, validation 390, test 400;
- test benign negatives: 255 (above the required 200);
- hard-negative and hard-positive examples are present;
- 20 counterfactual pairs and 12 negation pairs are included;
- benchmark tags B1–B10 are emitted and evaluated where sample size permits;
- descriptor coverage: 16/16 canonical capabilities explicit, 100%, fallback 0%, unknown 0%.

Independent dimensions include intent-template holdout, tool-family/action holdout, Chinese/English/mixed paraphrase coverage, counterfactual ordering, negation ordering, hard negatives, hard positives, and cross-turn traces.

## Independent oracle policy

`SecurityScenarioOracle` labels only from the structured scenario oracle contract and canonical action semantic effects. It does not call `RuleIntentInterpreter`, `IntentAlignmentGuard`, `AgentChainFeatureV2`, `FeatureSnapshot`, runtime snapshots, or production-derived predictions. Explicitly permitted operations can be labeled benign; forbidden or unpermitted high-risk effects are labeled risky. Deterministic deny and approval-approved signals from real shadow traffic are not ground truth.

Known limitation: `run_scenario()` currently retains the last action's snapshot and intent class for multi-action/multi-turn scenarios. Expected effects are not fully reconciled against runtime semantic effects for every action, and `ScenarioOutcome.label` is not recomputed per action. This is reported as a limitation, not as complete per-action proof.

## Synthetic baseline results

The stdlib logistic model is a benchmark/shadow baseline only. Calibration is Platt scaling fitted on calibration families only.

- True unseen-family validation: recall 0.6528, AUROC 0.9256, TPR@FPR 1% 0.3125.
- True unseen-family test: recall 0.6276, AUROC 0.8809, TPR@FPR 1% 0.3793.
- Intent-template holdout and tool-family holdout are reported in `artifacts/guard-joint-shadow-v0.metrics.json`.
- Counterfactual ordering: 0.5000 over 20 pairs.
- Negation ordering: 0.5833 over 12 pairs.
- Mean score margins: counterfactual 0.1676; negation 0.1049.
- LightGBM: `NOT_IMPLEMENTED` because training-side `lightgbm`/`numpy` dependencies are unavailable.

These are synthetic/shadow evaluation results and are not production detector evidence. The weak pairwise and tool-family results, together with remaining scenario limitations, block freeze.

## Dataset, artifact, and schema integrity

Dataset manifest: `artifacts/guard-dataset-manifest.json`.

- Dataset ID/version: `guard-scenario-catalog-v3.2` / `v3.2`.
- Dataset/scenario hash: `b50c43ca00565dc7c463faced6ae917e9404f03698a6e37282d22b441d932112` for the current generated feature snapshot.
- Feature schema hash: `065fca115611d552b1915dbed8ecdcc64cdb9848df8c9892286844bbb115cbc1`.
- Checked-in artifact canonical SHA-256: `4bdd284740595fc576c78dea73d4477e033e1d0afd8337fd87cc7082e4def1e8`.
- Artifact mode: `shadow`.
- Training seed: 7.
- Artifact training commit and metrics provenance commit: `f05351e47bfb1d4738340286e9b376799058774c`.
- Feature names are validated for unknown names, duplicate names, finite weights/bias, and exact schema hash; canonical artifact hashing excludes only the self-referential `artifact_sha256` field.

The artifact, metrics, manifest, generated dataset, and pair files were regenerated from the same deterministic dataset snapshot. The report intentionally records the current provenance rather than relying on earlier SHAs.

## Shadow analysis and privacy

`scripts/guard_ml/analyze_shadow.py` provides real-shadow disagreement counts, category/tool-origin/descriptor distributions, confidence histograms, and a privacy-preserving review queue. Review entries contain only feature snapshot ID, reason codes, prediction, decision, outcome taxonomy, approval signal, and explicit weak-supervision flags.

`REAL_SHADOW_EVIDENCE = UNAVAILABLE`: `artifacts/guard-dataset-v3.jsonl` is absent. `shadow_fixture.jsonl` was used only by `--self-test`; it is not production evidence and is not used for readiness claims. No `guard-shadow-analysis.json` or `review_queue.jsonl` was generated from the fixture.

## Runtime and dependency boundaries

- Default `BehaviorChainGuardHook` remains `NoClassifier`; no automatic artifact loading or Enforce default was added.
- Shadow mode does not change deterministic decisions; advisory/enforce remain explicit configuration modes.
- Runtime dependency wall check: 18 workspace members, 0 path violations, 0 transitive legacy packages.
- No `apeireth-guard`, LightGBM, ONNX Runtime, or rusqlite dependency was introduced into `apeireth-runtime`.
- The read-only `tool.repo` descriptor was corrected to read-only repository/user-display semantics with no publish, persistence, or network effect. Production governance tests now pass.

## Validation gates

Local results:

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets --locked`: passed.
- `cargo test --workspace --all-targets --locked`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Guard v3.2 evaluation tests: 7 passed.
- Guard library tests: 25 passed.
- Production governance tests: 5 passed.
- Dependency wall script: passed.
- Python compilation and shadow analyzer self-test: passed.
- `cargo deny check`: passed with existing configuration warnings, including unmatched/unnecessary skips and a yanked `chacha20 0.10.1` warning.
- `cargo audit`: passed with one allowed yanked-crate warning for `chacha20 0.10.1`; no vulnerability failure reported.
- Frontend `pnpm check`: 0 errors, 5 pre-existing warnings.
- Frontend `pnpm build`: passed.
- Tauri Cargo checks: passed.

Remote CI: no workflow run tied to the final SHA was available in this session; therefore `CI_VERIFIED = NO`.

## DoD matrix

| Item | Status | Evidence / reason |
|---|---|---|
| Correct Guard 3.1 terminology | PASS | Metrics and report distinguish within-family baseline from true group holdout. |
| True family/group isolation | PASS | SHA-256 split plus disjointness tests and family lists. |
| Intent-template holdout | PASS | Separate holdout metric and test coverage. |
| Chinese/English paraphrase coverage | PASS | Dataset language distribution and split reporting. |
| Tool-family/action holdout | PARTIAL | Implemented and measured; generalization is weak. |
| Counterfactual/negation evaluation | PARTIAL | Pair metrics exist; ordering is 0.50 / 0.5833. |
| Independent oracle | PASS | Source-independence tests and structured contract/effect labels. |
| Cross-turn trace support | PARTIAL | Traces exist; last-action snapshot limitation remains. |
| Dataset size and test negatives | PASS | 2,655 rows; 255 benign test negatives. |
| Artifact/schema/dataset integrity | PASS | SHA, schema hash, provenance, tamper rejection. |
| Calibration and low-FPR metrics | PASS | Platt calibration and explicit insufficient-sample handling. |
| Optional LightGBM | DEFERRED | Training-side only; dependency unavailable. |
| Real shadow evidence | BLOCKED | Required input is absent. |
| Privacy-preserving review queue | PASS | Analyzer and invariants implemented; no real queue without real data. |
| Runtime default remains SHADOW/NoClassifier | PASS | Default construction unchanged. |
| Rust/frontend/security gates | PASS WITH WARNINGS | Local gates pass; deny/audit/frontend warnings recorded. |
| Final-SHA remote CI | BLOCKED | No verifiable run for final SHA. |
| Freeze readiness | NOT READY | Blocking evidence and quality gaps remain. |

## Deferred items

1. Obtain real shadow data through the approved privacy-preserving export path and run the analyzer; never treat deterministic deny or approval-approved as ground truth.
2. Improve tool-family and pairwise generalization before considering classifier promotion.
3. Reconcile every action in multi-action/multi-turn traces and recompute per-action oracle labels/effects.
4. Run and record remote CI for the final commit SHA.
5. Revisit existing dependency/frontend warnings separately; they are not silently treated as Guard failures.

**Final decision: `NOT_FREEZE_READY`.**
