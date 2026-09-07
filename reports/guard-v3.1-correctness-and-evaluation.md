# Apeireth Guard 3.1 — Correctness and Evaluation

Status report for Guard 3.1 on `feature/cognitive-infrastructure-vnext`.
This round did not create a new Guard branch and did not modify `main`.

## Branch

- Branch: `feature/cognitive-infrastructure-vnext`
- Canonical repository: `Apeireth/Apeireth` (`apeireth-rust`)
- Base SHA: `3d4d4ab997a81b038b33ae1b9b41a7ab06ac5af9`
- `origin/main`: `7647d2c91d55901aeae7202f3842b65233bd053c`
- Next vs main at start: ahead, behind = 0

Final SHA is the tip of this branch after the Guard 3.1 commits. Record it from
`git rev-parse HEAD` after those commits land; do not treat the 3.0 SHA as current.

## P0 findings and fixes

| ID | Finding | Fix |
| --- | --- | --- |
| P0-1 | Unknown intent could receive `WorkspaceOnly` mutation | Unknown / low-confidence envelopes now fail-narrow: network, credential, shell, mutation, destructive, and persistence are `Deny` |
| P0-2 | Negation was substring/`contains` only | `NegationAwareOperationExtractor` uses bilingual deny phrases plus local prefix matching. Explicit denial dominates inferred permission |
| P0-3 | Advisory ≈ Enforce | Shadow never changes `Decision`. Advisory may escalate Allow → RequireApproval and cannot Deny from the model alone. Enforce may Deny only with structural evidence |
| P0-4 | `recent_turns` incremented per governance evaluation | Turns are upserted per `trace_id`. One trace with three actions counts as one turn |
| P0-5 | Cross-turn state was action-counted and unbounded | `SessionBehaviorHistory` is a 16-entry ring of `TurnBehaviorSummary` with `0.8^age` decay. Cross-turn features aggregate completed turns only |
| P0-6 | Classifier and dataset could recompute FeatureV2 | Guard computes `AgentChainFeatureV2` once, wraps it in `FeatureSnapshot`, and shares that snapshot with classifier, fusion, dataset, and status |

## Intent correctness

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

- `IntentConfidencePolicy`: `>= 0.85` explicit, `0.50–0.85` conservative, `< 0.50` Unknown + fail-narrow
- Unknown intent never opens mutation/network/shell/credential/destructive/persistence
- “看看这个项目，不要修改” and “只检查，不要修改，也不要联网” stay read-only
- “检查 token 配置，但不要显示 token 内容” is a scoped credential operation with disclosure denied, not a full credential-read grant

## Negation semantics

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

Covered markers include 不要 / 不能 / 别 / 禁止 / 无需 / 不需要 / 不允许 / 不可 / 不要再 / 只 / 仅 / 只读 and don't / do not / without / no / never / must not / should not / read-only / only inspect.

Required phrase tests cover write, network, shell, publish, delete, credential, persistence, and execute.

## Mode semantics

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

Rollout invariant: `disabled → shadow → advisory → enforce`.

The same critical prediction with supporting structural evidence:

- Shadow: Allow stays Allow
- Advisory: Allow becomes RequireApproval, never model-only Deny
- Enforce: Deny is allowed only when the model class is High/Critical **and** structural evidence exists (sensitive-to-external, egress, retry-after-denial, alternate-tool-after-denial, or prior denials)

Default CLI/runtime classifier remains `NoClassifier` (`disabled`) unless `APEIRETH_GUARD_ML_MODEL` and `APEIRETH_GUARD_ML_MODE` are set. Loaded artifacts still default to shadow.

## Cross-turn architecture

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

```text
BehaviorChain            = trace-local DAG
TurnBehaviorSummary      = one summary per trace
SessionBehaviorHistory   = VecDeque max 16
AgentChainFeatureV2.cross_turn = last N completed turn summaries
```

Same-turn credential/env/credential probes are same-turn sensitive chain, not `cross_turn_sensitive_probing`. Three independent traces trigger `cross_turn_sensitive_probing`. Old high risk decays; overflow pops the oldest turn.

## Feature snapshot architecture

Status: `IMPLEMENTED` + `PRODUCTION_WIRED` + `VERTICAL_TESTED`

```text
Guard evaluation
  → compute AgentChainFeatureV2 once
  → FeatureSnapshot { snapshot_id = trace_id:action_id:AgentChainFeatureV2 }
  → classifier / dataset / fusion / debug
```

`DatasetRecorder::record_classification` accepts `&FeatureSnapshot` and does not call `chain.extract_features_v2()`.

## Dataset V3

Status: `IMPLEMENTED` + `PRODUCTION_WIRED`

New classification, outcome, approval, execution, and compensation events write `guard-dataset-v3`. A v2 reader remains for legacy Outcome rows. New streams are not mixed v2/v3.

Human approval is a later supervision signal. `approved != benign`.

## Capability metadata truth chain

Status: `PRODUCTION_WIRED` for canonical tools; plugin/MCP declared metadata still uses conservative fallback

```text
CapabilitySafetyMetadataProvider
  → CapabilitySafetyDescriptor
  → SafetyObservation
  → Governance / Guard
```

Canonical tools declare operation/resource/source/sink classes, external/destructive/persistent/network/credential flags. `descriptor_source` is recorded as canonical / plugin_declared / adapter_inferred / fallback_heuristic / unknown. Unknown external-effect capabilities remain approval-or-stricter. Live MCP/plugin metadata adapters are `DEFERRED`.

## Shell effect analyzer

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

`CommandEffectAnalyzer` emits `CommandEffectSummary` without retaining the raw command. SafetyObservation carries `operation_classes` plus `primary_operation`. Alignment inspects all effects, so `shell git push` under a test-only intent is unrequested publish, not execute-only.

Covered: `cargo test`, `git status`, `git push`, `rm`, `curl GET`, `curl POST`, PowerShell download-execute, credential-file read via shell.

## Same-effect bypass

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

Effect fingerprints hash normalized operations, destination, destructive/persistent flags, and target class — not capability id. `fs.delete` vs `shell rm`, `repo.publish` vs `git push`, and `http.send` vs `curl POST` share effect categories. A denied canonical delete followed by `shell rm` produces SameEffect + AlternativeExecution + Escalation.

## Artifact validation

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

Load rejects unknown feature names, duplicates, non-finite weights, schema mismatch, and thresholds that are not `0 <= medium < high < critical <= 1`. Confidence is labelled `uncalibrated_margin` (`confidence_kind`), not calibrated probability. Artifact calibration is `uncalibrated`.

## Scenario DSL

Status: `IMPLEMENTED` + `VERTICAL_TESTED`

- Authored YAML examples: `security/guard_scenarios/`
- Machine catalog: `ScenarioCatalog::all()` (662 deterministic scenarios)
- Runner: intent interpreter → descriptor → observation → chain → alignment → FeatureV2 snapshot
- Training rows: `scripts/guard_ml/generated_features.jsonl` generated by `export_guard_scenarios`, not hand-authored FeatureV2

Categories include benign, hard_negative, ambiguous, scope_creep, credential, exfiltration, destructive, retry_bypass, tool_switching, cross_turn_probing, security_tamper, unknown_plugin, shell_semantics, and publish_alignment. Chinese, English, and mixed intents are present.

## Dataset size and split

Status: `LOCALLY_VERIFIED`

- Generated rows: 662
- Train / validation / test: 397 / 130 / 135, split by scenario family (not random actions)
- Class balance: 522 risky / 140 benign
- Hard negatives are present (explicit test/build/network/publish/delete/reset/install)
- TPR @ FPR 1% and 0.1%: `INSUFFICIENT_SAMPLE_SIZE`

The original 32-row `scripts/guard_ml/scenarios.jsonl` remains a fixture regression for the committed shadow artifact. It is not the primary evaluation set.

## ML baseline results

Status: `LOCALLY_VERIFIED` for logistic; LightGBM `NOT_IMPLEMENTED` (training-side optional dependency not installed); calibration `uncalibrated`

Logistic, family-held-out:

| Split | n | Precision | Recall | F1 | AUROC | AUPRC | Brier |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| validation | 130 | 0.878 | 0.990 | 0.931 | 0.937 | 0.980 | 0.079 |
| test | 135 | 0.896 | 0.990 | 0.941 | 0.927 | 0.978 | 0.082 |

These numbers evaluate the generated FeatureV2 extractor on unseen families. They are not a production-calibrated detector and must not be read as a low-FPR security guarantee.

`scripts/guard_ml/export_v3.py` exports ML rows from real `guard-dataset-v3` JSONL with `(trace_id, action_id)` correlation, keeping weak vs strong labels and treating approval as a later signal.

## Performance

Status: `LOCALLY_VERIFIED`

Fast local path: no remote model, no per-action filesystem, artifact loaded once at startup when configured.

Micro-timings in `intent_parser_alignment_and_fusion_are_fast_enough_for_local_path` bound intent parse, shell-effect analysis, and descriptor lookup well below 2 ms each on the developer machine. These are smoke timings, not Criterion benches.

## Dependency proof

Status: `LOCALLY_VERIFIED`

`cargo tree -p apeireth-runtime --edges normal --depth 3` contains no `apeireth-guard`, `guard-ml`, LightGBM, ONNX Runtime, or `rusqlite`. Runtime still does not depend on Guard.

## Local gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | `LOCALLY_VERIFIED` |
| `cargo check --workspace --all-targets --locked` | `LOCALLY_VERIFIED` |
| `cargo test --workspace --all-targets --locked` | `LOCALLY_VERIFIED` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | `LOCALLY_VERIFIED` |
| `cargo deny check` | `LOCALLY_VERIFIED` (pre-existing skip/yanked warnings; exit 0) |
| `cargo audit` | `LOCALLY_VERIFIED` (allowed yanked `chacha20 0.10.1`; exit 0) |
| `pnpm test` / `pnpm check` / `pnpm build` (companion-desktop) | `LOCALLY_VERIFIED` (0 test failures; svelte-check 0 errors / 5 pre-existing warnings) |

No gate was marked PASS because of a 120-second silence timeout.

## Remote CI

Status: `CI_VERIFIED` = NO

Reason: GitHub Actions `rust.yml` / `rust-lint.yml` run on `push` to `main` and on pull requests, not on a feature-branch push alone. This round does not open a main PR.

## Deferred

- Live MCP/plugin safety-metadata adapters (`DEFERRED`)
- LightGBM / ONNX production shadow (`NOT_IMPLEMENTED` in this environment)
- Probability calibration (Platt/isotonic) (`DEFERRED`; artifact `calibration.kind = uncalibrated`)
- Desktop Guard Inspector (`DEFERRED`)
- Guard 3.x freeze, Memory 2.2 (`DEFERRED`)
- Honest TPR @ 0.1%/1% FPR until negative sample size is sufficient

## DoD

| Question | Answer |
| --- | --- |
| Unknown intent fail-narrow? | YES |
| Negation-aware parsing implemented? | YES |
| Advisory distinct from Enforce? | YES |
| Cross-turn counted per trace/turn? | YES |
| History actually bounded? | YES |
| History decays? | YES |
| Classifier and dataset share exact FeatureV2? | YES |
| Dataset v3 consistent? | YES |
| Capability descriptor production-wired? | YES for canonical registry; plugin declared metadata still fallback |
| Shell semantic effects recognized? | YES |
| Same-effect bypass works across tool families? | YES |
| Unknown model features rejected? | YES |
| Thresholds validated? | YES |
| Scenario DSL runs real Guard pipeline? | YES |
| Training features generated rather than hand-authored? | YES |
| Train/val/test separated by scenario family? | YES |
| Hard negatives present? | YES |
| Chinese/English coverage present? | YES |
| Workspace tests green? | YES |
| Clippy green? | YES |
| Deny green? | YES |
| Audit green? | YES |
| Runtime dependency wall intact? | YES |
| Remote CI verified? | NO |
