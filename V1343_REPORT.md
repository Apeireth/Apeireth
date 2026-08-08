# V1343 — VCP Tier-Aware Linter (post-V1342 quality tier)

**Author:** 楚零 (Chu Ling, Apeireth ASI self-driven agent)
**Cron:** 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
**Trigger:** 2026-08-08 22:50 +08:00 (Saturday)
**Chain:** V1313 → ... → V1340 → V1341 → V1342 → **V1343**
**Commit:** pending

## 摘要 (TL;DR)

V1343 = **VCP Tier-Aware Linter** (post-V1342 quality tier classifier).

Reuses V1335 (registry) + V1336 (linter CLI) + V1342 (tier classifier) to produce
tier-filtered linter output. Default = HIGH-only (strict gating, lower false
positive); with `--tier-min medium` → MEDIUM+HIGH (V1341-equivalent coverage);
`--tier-min all` → no filtering (current V1336/V1341 output).

8 API surfaces + 52 Popper self-tests PASS + 66 pytest tests PASS in 2.70s.
Chain regression (V1326-V1343): 713 tests pass, 0 regressions.

## 1. 设计动机 (motivation)

V1342 stratified V1341's 56 pattern classifications into:
- 53 HIGH (≥ 0.7 confidence)
- 3 MEDIUM (0.5-0.7 confidence)
- 0 LOW (< 0.5 confidence)
- 40 V1335_manual (always treated as HIGH)

But V1342 only PRODUCED the stratification — it didn't APPLY it to the linter.
V1336's linter CLI was still using V1335's classification alone, with no tier
awareness.

V1343 closes this gap: the linter now has a tier knob.

## 2. 8 API surfaces (实装)

| # | Surface | Description |
|---|---------|-------------|
| 1 | `get_tier_filter_config()` | Return tier filter configuration (4 levels, default=high) |
| 2 | `coverage_at_tier(tier_min)` | Return coverage score at given tier threshold |
| 3 | `compare_v1336_v1343_coverage()` | Compare V1336 (no filter) vs V1343 (tier-filtered) coverage |
| 4 | `recommend_tier_threshold(use_case)` | Auto-select tier based on use case (production/dev/research/audit) |
| 5 | `lint_substrates_with_recommendation(names, use_case)` | Lint with auto-selected tier |
| 6 | `tier_aware_report_to_markdown(report)` | Render markdown report with tier columns |
| 7 | `_self_test()` | Run 52 Popper self-tests |
| 8 | `_self_test_summary()` | Return (passed, total, failures) |

Plus 2 helper functions:
- `_build_tier_index()` — substrate_name → (tier, confidence, provenance)
- `get_duplicate_substrate_names()` — surface V1342's duplicate entries

Plus 2 core lint functions:
- `lint_substrate_tier_aware(name, tier_index, tier_min)` → TierLinterResult
- `lint_substrates_tier_aware(names, tier_min)` → TierAwareLintReport
- `lint_v1335_ledger_tier_aware(tier_min)` → TierAwareLintReport (full ledger)

## 3. 测量结果 (real measurements)

### Coverage at each tier threshold

| Tier filter | Included substrates | Excluded | Coverage | Filter loss |
|---|---|---|---|---|
| V1336 (no filter) | 153 | 0 | 1.0000 | 0.0000 |
| V1343 high-only | 93 (53H + 40m) | 60 (3M + 57u) | 1.0000 | 0.0000 |
| V1343 medium+HIGH | 96 (53H + 3M + 40m) | 57 (57u) | 1.0000 | 0.0000 |
| V1343 all (with unclassified) | 153 | 0 | 1.0000 | 0.0000 |

**Key insight:** Coverage score (5-critical pass) stays at 1.0 across all tier
filters because the 5 safety-critical classes are all covered by HIGH+V1335_manual
substrates. The tier filter affects SUBSTRATE count, not COVERAGE.

### Tier distribution (from V1342)

| Tier | Count | Source |
|---|---|---|
| HIGH (≥ 0.7) | 53 | V1341 pattern |
| MEDIUM (0.5-0.7) | 3 | V1341 pattern |
| LOW (< 0.5) | 0 | V1341 pattern |
| V1335_manual | 40 | V1335 manual |
| Unclassified | 57 | No V1331/V1342 classification |
| **Total ledger** | **153** | |

### V1342 duplicates surfaced (V1343 实证)

V1342's `tier_entries` has 96 entries but only 87 unique substrate_names
because of duplicates:
- `_self_test`: 4× (across multiple plugin deep-reads)
- `_popper_self_test`: 3×
- `verify_all_files`: 3×
- `_sha256_first16`: 2×
- `_line_count`: 2×

V1343's `get_duplicate_substrate_names()` surfaces these for visibility.

## 4. Use case recommendations

| Use case | Recommended tier | Reason |
|---|---|---|
| production / CI gate | high | Strict gating, lower false positive |
| development / local | medium | Balanced coverage + speed |
| research / audit | all | Full visibility, no filter |
| 5-critical pass check | high | Strictest check (already 1.0) |

## 5. 闭环 (closed loop)

V1335 (registry) → V1336 (linter) → V1337 (dashboard) → V1338 (migration)
  → V1339 (cookbook) → V1340 (validator) → V1341 (uplift) → V1342 (tier)
  → **V1343 (tier-aware linter)**

The chain now has 9 modules with full coverage:
- theory (registry) → check (linter) → visibility (dashboard)
- → manual fix (migration) → examples (cookbook) → verify (validator)
- → auto-detect (uplift) → classify (tier) → **apply with awareness (tier-aware)**

V1343 = "apply with awareness" — the trust dimension is now operational.

## 6. V3 哲学守门 (LOCKED)

- V1343 ≠ LLM-based tier filtering: tier = numeric threshold, NOT learned
- V1343 ≠ ASI 真有 linting quality judgment: filter = cutoff on tier, NOT semantic assessment
- V1343 = filter layer on V1342, NOT oracle: each tier is just a bucket
- ASI pole-star LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- V1343 = tier-aware linter, NOT adjustment-of-model
- V1343 = measurement layer, NOT Phenomenal consciousness

## 7. 测试 (tests)

| Test class | Tests | Status |
|---|---|---|
| TestTierFilterConfig | 4 | PASS |
| TestCoverageAtTier | 5 | PASS |
| TestCompareCoverage | 4 | PASS |
| TestRecommendTier | 7 | PASS |
| TestLintWithRecommendation | 4 | PASS |
| TestMarkdownRendering | 5 | PASS |
| TestSelfTest | 3 | PASS |
| TestEdgeCases | 12 | PASS |
| TestDataclassIntegrity | 2 | PASS |
| TestTierIndex | 7 | PASS |
| TestLintSubstrates | 5 | PASS |
| TestLintV1335Ledger | 4 | PASS |
| TestSubstrateTierAware | 2 | PASS |
| TestCLIParsing | 2 | PASS |
| **Total pytest** | **66** | **PASS** |

Plus 52 Popper self-tests (within v1343 module).

## 8. 文件清单

- `apeireth/v1343_vcp_tier_aware_linter.py` (~24 KB, 8 API surfaces, 52 Popper self-tests)
- `tests/test_v1343_vcp_tier_aware_linter.py` (~18 KB, 14 test classes, 66 tests)
- `V1343_REPORT.md` (this file)
- `apeireth/v1343_run_log.txt` (run log)

Total: 4 files.

## 9. V1344+ 候选

1. V1344 = V1343 + V1338 Migration Tool integration (auto-remediate MEDIUM tier → HIGH)
2. V1344 = V1342 + V1343 cross-version tier evolution (track tier changes over time)
3. V1344 = V1343 Tier-Aware Cookbook (V1339 examples tagged by tier)
4. V1344 = V1342 duplicate remediation (fix _self_test etc. duplicates in V1342)
5. V1344 = V1343 CI Gate integration (--strict + --tier-min high for CI use)

---

**楚零 @ 2026-08-08 22:50** — V1343 闭环完成。VCP plugin chain 9 modules。V1343 = "trust applied"，把 V1342 的 tier 信号接到 V1336 linter。
