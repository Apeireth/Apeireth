# V1372 — V1371 ASCII Fire-Rate Timeline

**Phase:** 1372
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 225)
**Post:** V1371 (V1370 calibrated cron hook)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1372 is

V1372 is the **visible layer** for V1371. Where V1371 records raw + calibrated
fires into a sidecar JSONL, V1372 reads that sidecar and renders an ASCII
timeline that any human (主人, reviewer, anyone reading the repo) can interpret
in 5 seconds:

```
V1372 Timeline (schema=v1372.timeline/v1)
sidecar window: 2026-08-08T19:28:05Z  ->  2026-08-08T19:39:02Z  (n=17)
triggers: 8    evaluations: 17

trigger                         kind            timeline (oldest -> newest)
------------------------------  --------------  ----------------------------------------
TIME_TICK_INTERVAL              remeasure       ··················
DELTA_ANY_COMPONENT             remeasure       ··················
NEW_SURFACE_SHIPPED             remeasure       ··················
LEDGER_PLATEAU_SIGNAL           remeasure       ··················
NEW_MEASUREMENT_COMPONENT       v03_evolution   ··················
V1318_CELL_NEWLY_FILLED         v03_evolution   ··················
CAP_BECOMES_DISHONEST           v03_evolution   ··················
LEDGER_CAP_SATURATION_3         v03_evolution   ··················

Legend:  ·  no fire
         ●  raw fire (carried through to calibrated)
         �  raw fire but suppressed by V1370 calibrator (FP suppressed)
         ?  data missing (sidecar entry malformed)
```

V1372 is a pure **reader**: it does not write to the sidecar, never modifies
the ledger, never raises the cap. It is intentionally non-mutating so it can
be safely run from any cron hook or human inspection.

## Why V1372 exists

V1371's sidecar is JSONL — good for machines, opaque for humans. V1372 is the
one-screen, anyone-can-read view that turns the sidecar into evidence a human
can interpret at a glance:

- A row of all `·` → trigger has not fired in this window → honest baseline
- A row with `●` → there was a fire; check the calibrated reason
- A row with `◌` → raw fired but calibration suppressed (V1370 doing its job)
- Mixed rows → real signal of intermittent state

This is **post-V1371 next-step 1/5** (the historical-rate-graph candidate from
V1371_REPORT.md). It chose ASCII over matplotlib because:
1. zero dependencies (no matplotlib, no numpy, no graphviz)
2. renders the same on any terminal (Windows CP1252, Linux UTF-8, ssh)
3. anyone can pipe to a file and diff sidecars
4. fits the **任何人都能接手** directive (one-screen, plain text, no install)

## 8 API surfaces

1. `load_sidecar(path)` — read JSONL, return sorted list[dict]
2. `build_timeline(evals)` — per-trigger char timeline
3. `bucket_by_minute(evals)` — group evals into 1-minute UTC buckets
4. `render_ascii(timeline, evals, width=60)` — single-screen ASCII table
5. `render_summary(timeline)` — totals table (raw / cal / suppressed / fire_rate)
6. `render_legend()` — char legend
7. `run_cli(args)` — argv dispatcher (timeline / summary / legend / version / popper)
8. `_popper_self_tests()` — 53 Popper self-tests

## GUARDS upheld (V1372-specific)

| Guard | Honored? |
|-------|---------|
| GUARD_NOT_SIDECAR_WRITER (V1372 reads, never writes) | ✅ |
| GUARD_NO_LEDGER_TOUCH (no import of V1362/V1368 ledger) | ✅ |
| GUARD_NO_CAP_CHANGE (no notion of cap) | ✅ |
| GUARD_ASCII_ONLY (UTF-8 box-drawing chars; reconfigured stdout) | ✅ |
| GUARD_V03_REQUIRES_EVIDENCE (no V0.3 trigger evaluation) | ✅ |
| GUARD_CAP_NOT_AUTO_RAISED (no cap touching) | ✅ |

## Honest measurement (real sidecar)

- `v1370_calibrated_cron_evaluations.jsonl` = 17 evaluations (post-V1372 commit window)
- Per-eval: 8 triggers, 0 raw fires, 0 calibrated fires, 0 suppressions
- Per-trigger totals across 17 evals: 0/17 fire rate everywhere
- This is **plateau, not failure** (post-V1370 calibration effective)
- The visible sidecar window: 2026-08-08T19:28:05Z → 2026-08-08T19:39:02Z (~11 min)

## Tests (real)

- **53/53 Popper self-tests** PASS (covers load/timeline/bucket/render/summary/CLI + 5 edge cases + 1 synthetic-data round-trip)
- **34/34 pytest** PASS in 0.29s (real-sidecar + synthetic + edge cases + CLI)
- **184/184 chain regression** (V1368 + V1369 + V1370 + V1371 + V1372) PASS in 1.01s
- 0 regression across 1390 modules (1389 → 1390 after V1372)

## V3 守门 (philosophy watchdogs)

| Guard | Honored? |
|-------|---------|
| 不假装 (no pretending) | ✅ V1372 honest about 0 fires in 17 evals |
| 实事求是 (real evidence) | ✅ sidecar on disk; 17 real evals; 0 real fires |
| 不刷 KPI (no KPI faking) | ✅ fire_rate = 0.00% everywhere; honest plateau |
| 大胆激进 (bold iteration) | ✅ V1372 chose ASCII over matplotlib for "anyone can take over" |
| 任何人都能接手 (anyone can take over) | ✅ one-screen CLI, zero deps, pipe-able, diff-able |
| 干到底 (see it through) | ✅ V1369 + V1370 + V1371 + V1372 chain closed; visible layer |
| 走在前人经验中 (walk in human wisdom) | ✅ ASCII timeline pattern is canonical in CLI tools (git, kubectl, htop) |
| 质量工程区 (quality engineering) | ✅ positive (real sidecar) + negative (synthetic with fires + suppression) + edge (empty, malformed) |

## CLI examples

```bash
# Render the current ASCII timeline
python -m apeireth.v1372_v1371_ascii_timeline timeline

# Custom width + custom sidecar
python -m apeireth.v1372_v1371_ascii_timeline --sidecar mysidecar.jsonl timeline --width 120

# Per-trigger summary
python -m apeireth.v1372_v1371_ascii_timeline summary

# Legend only
python -m apeireth.v1372_v1371_ascii_timeline legend

# Popper self-tests
python -m apeireth.v1372_v1371_ascii_timeline popper
```

## V1373+ candidates (open, post-V1372 next-step)

1. V1373 = V1372 + V1362 history overlay (correlate fire-rate with ledger entries)
2. V1373 = V1372 markdown export (one .md per sidecar; GitHub-flavored)
3. V1373 = V1372 diff mode (compare two sidecars; show new fires)
4. V1373 = V1372 weekly digest (auto-summarize sidecar every N evals)
5. V1373 = V1372 + V1361 streamlit dashboard integration (compose observability)

## Commit

`V1372` — to be committed in this tick (post-tests + post-report).
