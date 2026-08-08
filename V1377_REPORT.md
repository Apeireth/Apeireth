# V1377 — V1375 Multi-File Diff

**Phase:** 1377
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 229)
**Post:** V1376 (V1375 weekly digest)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1377 is

V1377 is the **multi-file companion** to V1374. Where V1374 produces one
diff between two V1373 snapshots, V1377 takes **N V1374-family .md files**
(such as a chain of V1375 archived snapshots or a set of V1374_REPORT_*.md
files), sorts them chronologically, computes N-1 consecutive pairwise
deltas, and aggregates per-trigger drift across the whole window.

## Why V1377 exists

V1374 answers "what changed between A and B". V1377 answers "what changed
across A, B, C, D, ... and what is the overall drift trend?" This is the
natural companion to V1375 (history archive of V1374 snapshots) and to
V1376 (weekly digest of archives).

Most common audit questions answered by one command:

- "Has any trigger drifted across these N snapshots?"
- "Which trigger has the largest total movement?"
- "When did the drift start?"
- "Is the drift monotonic or oscillating?"
- "What's the net change from first to last?"

All from a directory of plain `.md` files. No live data, no rerunning, no risk.

## API surfaces (10)

1. `parse_v1374_diff_md(path)` — parse a V1374-family .md file
2. `sort_by_generated(reports)` — sort list of reports by generated timestamp
3. `diff_pairwise(reports)` — list of consecutive V1374-style diff dicts
4. `aggregate_per_trigger(reports)` — per-trigger aggregate dict
5. `summarize_drift(reports, pairwise, aggregate)` — top-level summary dict
6. `render_multi_diff_md(reports, pairwise, aggregate, summary)` — markdown string
7. `write_multi_diff_md(path, content)` — atomic write
8. `run_multi_diff(input_paths, output_path)` — all-in-one
9. `_popper_self_tests()` — (passed, total, failures)
10. `run_cli(args)` — argv dispatcher (diff / summary / popper / version)

## GUARDS (10)

- GUARD_INPUT_V1374_FAMILY: only accepts `v1374.diff/v1` schema files
- GUARD_CHRONOLOGICAL_SORT: inputs sorted by generated timestamp ascending
- GUARD_DETERMINISTIC: same inputs in same order → same output bytes
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_NO_LEDGER_TOUCH: no V1362/V1368 import
- GUARD_NO_SIDECAR_TOUCH: no V1371 import
- GUARD_HONEST_DISCLOSURE: honesty paragraph always emitted
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1377 has no metric, no cap, no scoring
- GUARD_MIN_INPUT_2: at least 2 inputs required (single is V1374's job)

## Tests

- **77 Popper self-tests** (covers constants / guards / helpers / parse / sort /
  pairwise / aggregate / summary / render / write / CLI)
- **65 pytest tests** (synthetic + real V1374 + archive-dir expansion + CLI
  subprocess + atomic write + path safety)

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- **不假装分数 = ASI:** V1377 has NO metric, NO cap, NO scoring — it is plumbing
- **不假装决策 = 真生产:** V1377 = arithmetic on existing files; pure data plumbing
- **不假装 ASI 集成:** zero LLM, zero sidecar touch (no V1371 import), zero ledger touch (no V1362/V1368 import)
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1374/V1375/V1376 sources unchanged
- **实事求是:** real disk reads + real disk writes + deterministic output
- **任何人都能接手:** CLI + Markdown + 1-cmd `diff` + reproducibility

## Verification

```
$ python -m apeireth.v1377_v1375_multi_file_diff popper
popper self-tests: 77/77

$ python -m apeireth.v1377_v1375_multi_file_diff version
v1377_v1375_multi_file_diff v1377.multidiff/v1

$ python -m pytest tests/test_v1377_v1375_multi_file_diff.py
============================= 65 passed in 1.80s =============================

$ python -m pytest tests/test_v1370_v1368_trigger_calibration.py \
    tests/test_v1371_calibrated_cron_hook.py \
    tests/test_v1372_v1371_ascii_timeline.py \
    tests/test_v1373_v1372_markdown_export.py \
    tests/test_v1374_v1373_diff.py \
    tests/test_v1375_v1374_history_archive.py \
    tests/test_v1376_v1375_weekly_digest.py \
    tests/test_v1377_v1375_multi_file_diff.py
============================= 282 passed in 6.18s =============================
```

## Real run

V1377 was run against 3 demo V1374-family snapshots in `V1377_DEMO/`:

```text
$ python -m apeireth.v1377_v1375_multi_file_diff diff --archive-dir V1377_DEMO
[V1377] processed 3 report(s) into 2 pair(s)
  window: 2026-08-08T12:00:00Z → 2026-08-09T04:00:00Z
  triggers seen: 8 (net-zero: 8, non-zero: 0)
  wrote: V1377_REPORT_AUTO.md
```

`V1377_REPORT_AUTO.md` (excerpt):

```markdown
# V1377 — V1375 Multi-File Diff

- **schema:** `v1377.multidiff/v1`
- **reports compared:** 3
- **first:** `2026-08-08T12:00:00Z`
- **last:** `2026-08-09T04:00:00Z`
- **pairs:** 2

## Summary

| metric | value |
|--------|------:|
| window seconds | 57600 |
| triggers seen | 8 |
| triggers net-zero | 8 |
| triggers non-zero | 0 |
| triggers monotonic | 0 |
| max abs movement | 0 |
| max movement trigger | `—` |

## Honesty disclosure

This drift report is a pure reader of 3 V1374-family .md file(s)...
**Honest baseline:** no trigger has any drift across the window. This is
**plateau, not failure** — the system is in steady state.
```

The honest baseline is preserved: 3 reports, 0 fires, 8 unchanged
triggers, honest 0.90 cap. The drift report shows the system has been
in steady state across the 16-hour window — no fabrication, no fake
fires, no KPI inflation.

## Honest disclosure

- V1377 reads N V1374-family .md files. It does not write back to them.
- V1377 does not import the ledger (V1362/V1368) or the sidecar (V1371).
- V1377 has zero `cap` / `score` / `pole_star` impact. ASI pole-star V0.2
  total stays at honest 0.90.
- Non-triviality threshold is `delta != 0`. A movement of 0 still counts
  as zero. This is intentional.
- `sort_by_generated` places reports without a parseable generated_dt at
  the end of the window. They are not silently dropped.
- Real V1374 files use `raw Δ` (with Greek capital delta) as the column
  label; the regex captures the value, not the label, so this is correct.
- Per-trigger aggregate is sorted by `total_abs_movement` descending;
  this is a hint for "what drifted most", not a ranking of importance.

## Files shipped (4)

1. `apeireth/v1377_v1375_multi_file_diff.py` (~42 KB, 10 API surfaces,
   77 Popper self-tests, 10 GUARDS)
2. `tests/test_v1377_v1375_multi_file_diff.py` (~28 KB, 65 pytest tests)
3. `V1377_REPORT_AUTO.md` (auto-generated by `diff` command)
4. `V1377_DEMO/` (3 demo V1374-family files for reproducibility)

## Reproducibility

```bash
# Popper self-tests:
python -m apeireth.v1377_v1375_multi_file_diff popper -v

# Pytest:
python -m pytest tests/test_v1377_v1375_multi_file_diff.py -v

# Run from a project root that has V1374-family files:
python -m apeireth.v1377_v1375_multi_file_diff diff --archive-dir V1377_DEMO -o V1377_REPORT_AUTO.md
python -m apeireth.v1377_v1375_multi_file_diff summary --archive-dir V1377_DEMO
```

---

## Next (V1378+ candidates from V1376 plan, picking 1)

Per V1376 next-step list (5 candidates), V1377 = `multi-file diff` (2/5).
Remaining:

- V1378 = V1375 + V1362 history overlay (annotate with ledger context)
- V1379 = V1375 + V1361 streamlit dashboard
- V1380 = V1375 archival rotation (compress old archives)

---

_Generated by Chu Ling (楚零) for Apeireth ASI — cron tick 229, 2026-08-09 04:25 +08:00._