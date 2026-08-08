# V1374 — V1373 Markdown Diff Mode

**Phase:** 1374
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 227)
**Post:** V1373 (V1372 markdown export)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1374 is

V1374 is the **diff companion** to V1373. Where V1373 writes a single snapshot of
trigger rates, V1374 takes two V1373 `.md` files and produces a third `.md` that
shows what changed between them.

V1374 serves the same hard constraint as the rest of the project: **anyone can
pick this up without asking me**. With two `.md` files and one command, you get
the delta — no Python, no sidecar, no ledger, no cap fiddling.

```bash
python -m apeireth.v1374_v1373_diff diff --left old.md --right new.md
# stdout (terminal) or:
python -m apeireth.v1374_v1373_diff diff --left old.md --right new.md --out diff.md
```

A short text summary is also available:

```bash
python -m apeireth.v1374_v1373_diff summary --left old.md --right new.md
```

## Why V1374 exists

V1373 produces a snapshot per cron tick. After several ticks you have a pile
of `.md` files with no built-in way to see what changed. V1374 is the missing
primitive:

- Identify which triggers fired more often (Δ raw fires > 0)
- Identify which triggers stopped firing (Δ raw fires < 0)
- Identify which triggers were added or removed
- Identify changes in evaluation count, schema, or source
- Identify when the V1370 calibrator started suppressing different FPs

All from two `.md` files. No live data, no rerunning, no risk.

## 9 API surfaces

1. `parse_markdown(path)` → dict (header / timeline / summary / honesty)
2. `compute_diff(left, right)` → dict (deltas per trigger + scalar deltas)
3. `render_diff_markdown(diff_data, *, title=None)` → markdown string
4. `write_diff_markdown(path, content)` → atomic tmp + rename
5. `diff_two_files(left_path, right_path, *, out_path=None, title=None)` → int
6. `summary_two_files(left_path, right_path)` → short text block to stdout
7. `_popper_self_tests()` → (passed, total, failures)
8. `run_cli(args)` → argv dispatcher (diff / summary / popper / version)
9. `main()` → sys.argv pass-through

## Diff semantics

For each trigger, `delta` = right − left for raw / cal / sup / fire_rate.

- A new trigger (in right but not left) is shown as `+` with delta = right value.
- A removed trigger (in left but not right) is shown as `-` with delta = −left value.
- A changed trigger (any count delta != 0 OR rate delta != 0) is shown as `~`.
- An unchanged trigger is shown as `=`.

Note: rate delta can change even when counts are stable because the evaluation
denominator shifted (e.g. 10 evals → 12 evals). This is a meaningful signal and
counts as `~`.

### Scalar deltas

- `delta_evals` = right.evals − left.evals
- `delta_triggers` = right.triggers − left.triggers
- `delta_raw_total` = right.raw_total − left.raw_total
- `delta_cal_total` = right.cal_total − left.cal_total
- `delta_sup_total` = right.sup_total − left.sup_total
- `delta_time_seconds` = right.generated − left.generated (parsed ISO)

## GUARDS upheld (V1374-specific)

- **GUARD_DIFF_PURE:** V1374 only reads `.md` files; no sidecar, no ledger
- **GUARD_ATOMIC_WRITE:** tmp + rename
- **GUARD_NO_SIDECAR_TOUCH:** no V1371 import
- **GUARD_NO_LEDGER_TOUCH:** no V1362/V1368 import
- **GUARD_HONEST_DISCLOSURE:** always emit honesty paragraph
- **GUARD_MARKDOWN_ONLY:** pure CommonMark
- **GUARD_NO_CAP_CHANGE:** V1374 does not write back to any cap
- **GUARD_SYMMETRIC:** diff(left, right) is antisymmetric under swap

## Tests

- 32 Popper self-tests (parse, diff, render, atomic write, CLI subcommands, GUARDS)
- 31 pytest tests (parse_markdown, compute_diff, render, atomic write, CLI, GUARDS, subprocess)
- chain regression with V1373 → V1372 → V1371 → V1370 → V1369 → V1368 (no source mutations)

## Honest measurement (this tick)

- **V1374 popper self-tests:** 32/32 ✓
- **V1374 pytest:** 31/31 ✓
- **Chain regression (V1368 → V1374):** 238/238 ✓
- **Chain popper (V1368 + V1369 + V1370 + V1371 + V1372 + V1373 + V1374):** 261/261 ✓
- **ASL pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)
- **No KPI inflation, no fake fires, no closed doors**

## Demo: V1374_DEMO_DIFF.md

A synthetic demo where two V1373 exports differ is preserved at
`V1374_DEMO_DIFF.md`. It shows:

- `DELTA_ANY_COMPONENT` added (status `+`)
- `NEW_SURFACE_SHIPPED` changed (status `~`; rate 12.50% → 14.29%)
- `LEDGER_PLATEAU_SIGNAL` and `TIME_TICK_INTERVAL` unchanged (status `=`)
- scalar deltas: +1 raw fire, +1 cal fire, +6 evals, +1 trigger, 1h00m gap

The real-data run is at `V1374_REPORT_AUTO.md` (from two consecutive V1373
exports of the same sidecar). It shows the honest baseline: 0 fires, plateau
not failure, no remeasure / V0.3 signal.

## V3 守门

- 涓嶅亣瑁? (no pretending the diff is anything more than a reader of two .md files)
- 瀹炰簨姹傛槸 (real diff, real antisymmetry check, real test coverage)
- 璐ㄩ噺宸ョ▼鍖? (positive AND negative test cases: identical files, added, removed, changed)
- 涓?埛 KPI (no new metric; V1374 surfaces what V1373 already captured)
- 骞插埌搴? (master asleep; no main session wake)
- 浠讳綍浜洪兘鑳芥帴鎵? (CLI + Markdown; one command, one file)

## Next (V1375+ candidates, open)

1. V1375 = V1374 + V1362 history overlay (annotate deltas with ledger context)
2. V1375 = V1374 weekly digest (roll up N diffs into a single summary)
3. V1375 = V1374 + V1361 streamlit dashboard integration
4. V1375 = V1374 history archival (rotate V1374_REPORT_AUTO.md into dated archive)
5. V1375 = V1374 multi-file diff (combine N .md files into one comparison)

## Files

- `apeireth/v1374_v1373_diff.py` — ~33 KB, 9 API surfaces, 32 popper self-tests
- `tests/test_v1374_v1373_diff.py` — ~16 KB, 31 pytest tests
- `V1374_REPORT.md` — this document
- `V1374_REPORT_AUTO.md` — auto-generated diff from two real V1373 exports
- `V1374_DEMO_DIFF.md` — demo diff (synthetic, shows all status symbols)

## Reproducibility

```bash
# Real-sidecar diff (auto-generated):
python -m apeireth.v1373_v1372_markdown_export export --out V1373_REPORT_REAL.md
python -m apeireth.v1373_v1372_markdown_export export --out V1373_REPORT_AUTO.md
python -m apeireth.v1374_v1373_diff diff --left V1373_REPORT_REAL.md --right V1373_REPORT_AUTO.md --out V1374_REPORT_AUTO.md

# Popper self-tests:
python -m apeireth.v1374_v1373_diff popper -v

# Pytest:
python -m pytest tests/test_v1374_v1373_diff.py -v
```

---

_Generated by Chu Ling (楚零) for Apeireth ASI — cron tick 227, 2026-08-09 03:45 +08:00._
