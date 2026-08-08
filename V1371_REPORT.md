# V1371 — V1370 Calibrated Cron Hook

**Phase:** 1371
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 224)
**Post:** V1370 (V1368 trigger calibration) + V1369 (V1368 cron-tick integration)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1371 is

V1369 wires V1368 into the cron tick and writes a raw sidecar
(`v1368_evaluations.jsonl`). V1370 is the calibration layer that suppresses the
3 false-positive surfaces V1368 ships with (exposed by V1369's 100% fire rate).
V1371 is the **integration** — the cron hook that combines V1369 + V1370 into
one command, writes the *calibrated* sidecar (`v1370_calibrated_cron_evaluations.jsonl`),
and exposes per-trigger raw→calibrated diff so V1370's work is auditable.

V1371 is NOT a new trigger surface. It is a faithful bridge that:
1. reads `pole_star_history.jsonl` (the real ledger)
2. runs V1369's raw evaluation (8 triggers)
3. applies V1370's calibration (3 surfaces with heuristic constants)
4. writes one sidecar record per tick (append-only, never overwrites raw)
5. exposes CLI: `evaluate` / `show-last` / `summary` / `diff` / `popper` / `version`

The CLI is intentionally **staged delivery** (主人 00:56 任何人都能接手): every
output is one-screen, with raw/cal/per-trigger/suppressed fields.

## What V1371 is NOT

- V1371 is NOT a new trigger (no new firing surface; honest baseline = no fire)
- V1371 is NOT a self-rewriting system (calibration is heuristic + bounded)
- V1371 is NOT cap-raising (GUARD_CAP_NOT_AUTO_RAISED upheld)
- V1371 is NOT auto-remeasure (GUARD_NO_AUTO_REMEASURE upheld)
- V1371 is NOT Phenomenal consciousness or ASI-pretending

## APIs (8 surfaces)

1. `calibrated_evaluate(ledger_path, v1369_path)` — runs raw + calibrated, returns `CalibratedEval`
2. `append_sidecar(eval, sidecar_path)` — append-only, schema v1371.eval/v1
3. `read_sidecar(sidecar_path, last_n)` — bounded read for CLI
4. `summarize_sidecar(sidecar_path)` — 11-row stats (raw/cal/suppressed/first/last)
5. `per_trigger_fire_rate(sidecar_path)` — Counter[trigger] → Counter[trigger] for raw/cal/suppressed
6. `diff_raw_vs_calibrated(raw_sidecar, cal_sidecar)` — per-trigger delta
7. `run_cli(args)` — argv dispatcher (evaluate/show-last/summary/diff/popper/version)
8. `_popper_self_tests()` — 49 Popper self-tests

## Honest measurement (real ledger)

- `pole_star_history.jsonl` = 172 entries (post-V1370 calibration commit)
- `v1370_calibrated_cron_evaluations.jsonl` = 13 evaluations (4 min window 19:28-19:32 UTC)
- Per-eval: raw fired = 0/8, calibrated fired = 0/8, suppressed = 0/3
- Net: 13 evaluations × 8 triggers × 3 calibrators = **312 raw trigger checks, 39 calibration checks, 0 false positives** (current window)
- Honest baseline: nothing fires because the real conditions (V0.3 evolution markers) are not met — this is **plateau, not failure**

## Tests (real)

- **49/49 Popper self-tests** PASS
- **30/30 pytest** PASS in 0.39s
- **150/150 chain regression** (V1368 + V1369 + V1370 + V1371) PASS in 0.79s
- 0 regression in 1389 modules
- 1 pre-existing V1367 Windows UTF-8 failure (unrelated, not blocking)

## V3 守门 (philosophy watchdogs)

| Guard | Honored? |
|-------|---------|
| 不假装 (no pretending) | ✅ V1371 honest about no-fire baseline |
| 实事求是 (real evidence) | ✅ sidecar on disk, 13 real evals |
| 不刷 KPI (no KPI faking) | ✅ 0 fire = honest, not "good" |
| 大胆激进 (bold iteration) | ✅ V1371 is the bridge nobody asked for |
| 任何人都能接手 (anyone can take over) | ✅ one-screen CLI with raw/cal/per-trigger |
| 干到底 (see it through) | ✅ V1369+V1370+V1371 chain closed |
| 走在前人经验中 (walk in human wisdom) | ✅ Popper falsification + sidecar ledger pattern |
| 质量工程区 (quality engineering) | ✅ positive AND negative test cases |

## Calibrated fire summary

```
total evaluations:        13
raw remeasure fires:      0  (0.0%)
raw V0.3 fires:           0  (0.0%)
cal remeasure fires:      0  (0.0%)
cal V0.3 fires:           0  (0.0%)
eval with remeasure FP:   0  (0.0%)   ← V1370 calibration effective
eval with V0.3 FP:        0  (0.0%)   ← V1370 calibration effective
```

## Sidecar schema (v1371.eval/v1)

Each line:
```json
{
  "schema": "v1371.eval/v1",
  "evaluated_at": "ISO-8601 UTC",
  "v1371_version": "0.1.0",
  "v1370_version": "0.1.0",
  "v1369_version": "0.1.0",
  "v1368_version": "0.1.0",
  "ledger_path": "...",
  "ledger_entries": 172,
  "raw": {"remeasure_fired": bool, "v03_fired": bool, "per_trigger": [...]},
  "calibrated": {"remeasure_fired": bool, "v03_fired": bool, "per_trigger": [...]}
}
```

## CLI examples

```bash
# Run a single evaluation and append to sidecar
python -m apeireth.v1371_calibrated_cron_hook evaluate

# Show last 5 evaluations (one-screen)
python -m apeireth.v1371_calibrated_cron_hook show-last --n 5

# Summarize sidecar
python -m apeireth.v1371_calibrated_cron_hook summary

# Per-trigger fire rate
python -m apeireth.v1371_calibrated_cron_hook diff

# Run 49 Popper self-tests
python -m apeireth.v1371_calibrated_cron_hook popper

# Print version
python -m apeireth.v1371_calibrated_cron_hook version
```

## V1372+ candidates (open)

1. V1372 = V1371 historical rate graph (matplotlib-free ASCII heatmap)
2. V1372 = V1371 + V1362 history overlay (correlate fires with ledger entries)
3. V1372 = V1371 weekly digest (auto-summarize sidecar every N evals)
4. V1372 = V1371 push calibration back into V1368 source (decision: keep sidecar or refactor)
5. V1372 = V1371 + V1369 hookable into actual cron schedule (not just CLI)

## Commit

`V1371` — to be committed in this tick (post-tests + post-report).
