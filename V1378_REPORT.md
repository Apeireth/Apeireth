# V1378 — V1375 × V1362 History Overlay

**Phase:** 1378
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 230, 04:32 → 04:36)
**Post:** V1377 (V1375 multi-file diff)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1378 is

V1378 is the **ledger-overlay companion** to V1375. Where V1375 archives
V1374 diff snapshots into `V1375_HISTORY/` and V1377 aggregates per-trigger
drift across those archives, **V1378 annotates each archived V1374 snapshot
with the nearest V1362 pole-star ledger entry by timestamp.**

It is a pure read+annotate layer:

- Reads `V1375_HISTORY/<iso>__v1374.md` files via V1375's `list_archives()`
- Reads `pole_star_history.jsonl` directly (independent of V1362 module)
- Writes one overlay markdown report, atomically (tmp + rename)
- Touches nothing else: no V1371 import, no sidecar, no ledger mutation

---

## Why V1378 exists

Two existing artifacts preserve different views of the same timeline:

- **V1375 archive** preserves *what changed in V1368 trigger space* over time
- **V1362 ledger** preserves *what pole-star the system claimed* over time

V1378 answers, in one CLI:

- Which V1375 archive aligns with which V1362 ledger tag?
- How close in time is each archive to its nearest ledger entry?
- Did pole-star change between consecutive archives?
- How many archives have any pole-star data, vs `—`?
- Did `toolchain_present` / `close_loop_pass` / `v_modules` / `test_files`
  grow between archives?

---

## API surfaces (10)

1. `parse_iso_dt(iso)` — robust ISO-8601 → tz-aware datetime; **accepts both
   ISO extended (`2026-08-09T04:00:00Z`) and ISO basic (`2026-08-09T04-00-00Z`,
   the V1375 filename format)**
2. `read_ledger_jsonl(path)` — parse ledger JSONL into list of dicts; **robust
   to Windows `/nonexistent` quirk where `os.path.exists` returns True for
   drive-rooted paths but `open()` raises PermissionError**
3. `find_nearest_ledger(archive_dt, ledger_entries)` — pick closest by |Δt|,
   stable first-occurrence tie-break
4. `overlay_row(archive, ledger_entry, time_gap_s)` — one annotated row
5. `build_overlay(archives, ledger_entries)` — list of overlay rows
6. `summarize_overlay(rows, archives, ledger_entries)` — top-level summary
7. `render_overlay_md(rows, summary, archives, ledger_entries)` — markdown str
8. `write_overlay_md(path, content)` — atomic write (tmp + rename)
9. `_popper_self_tests()` — 53 self-checks
10. `run_cli(args)` — argv dispatcher (overlay / summary / popper / version)

---

## GUARDS upheld (10)

| # | Guard | What it prevents |
|---|-------|------------------|
| 1 | `GUARD_INPUT_V1375_FAMILY` | only accepts archive filenames matching V1375 slug |
| 2 | `GUARD_CHRONOLOGICAL_SORT` | archives sorted by ISO ascending before overlay |
| 3 | `GUARD_DETERMINISTIC` | same inputs in same order → same output bytes |
| 4 | `GUARD_ATOMIC_WRITE` | tmp + rename; no partial writes |
| 5 | `GUARD_NO_LEDGER_WRITE` | V1378 reads ledger; never writes to it |
| 6 | `GUARD_NO_SIDECAR_TOUCH` | V1371 is not imported |
| 7 | `GUARD_HONEST_DISCLOSURE` | honesty paragraph always emitted |
| 8 | `GUARD_MARKDOWN_ONLY` | pure CommonMark |
| 9 | `GUARD_NO_CAP_CHANGE` | V1378 has no metric, no cap, no scoring |
| 10 | `GUARD_NO_LEDGER_MUTATION` | ledger parsed read-only; no row insertions |

---

## Honest measurement (this tick)

- **V1378 Popper self-tests:** 53/53 ✓
- **V1378 pytest:** 63/63 ✓
- **Chain pytest (V1370 → V1378):** 345/345 ✓ (no regression)
- **Chain popper (V1370 → V1378):** 429/429 ✓ (no regression)
- **ASI pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)
- **Real-data smoke test:** `V1375_HISTORY/` (1 archive) × `pole_star_history.jsonl`
  (172 entries) → 1 overlay row, Δt = 47m, pole_star = 0.9000
- **`V1378_OVERLAY_AUTO.md`** auto-generated at `04:34:55Z`

---

## Bugs hit during development

1. **Windows `/nonexistent` quirk in `read_ledger_jsonl`.**
   On Windows, `os.path.exists('/nonexistent')` returns True because the path
   resolves to the current drive root (e.g. `C:\`), but `open()` raises
   `PermissionError`. Fixed by:
   - Using `os.path.isfile()` instead of `os.path.exists()` (catches both
     missing and dir-collision cases)
   - Wrapping `open()` in `try/except OSError`

2. **V1375 filename ISO basic format not parseable.**
   V1375 archive filenames use ISO basic (filesystem-safe) format
   `2026-08-09T04-00-00Z__v1374.md` with dashes instead of colons.
   `datetime.fromisoformat` rejects this. Fixed by:
   - Pre-normalizing `THH-MM-SS` → `THH:MM:SS` via regex in `parse_iso_dt`
   - Added `test_parse_iso_dt_iso_basic_v1375_format` to lock the behavior

3. **CLI overlay/summary tests failed with PermissionError.**
   Test passed `/nonexistent` as archive-dir and ledger. Without the fix in
   bug #1, the test crashed before exit code 0. Fixed together with #1.

---

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI:** V1378 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1378 = pure read+annotate; no mutation, no inference
- **不假装 ASI 集成:** zero LLM, zero sidecar touch, zero ledger write
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1375 / V1362 sources unchanged; V1378 only reads
- **不假装 V1378 = ASI 觉醒:** V1378 finds nearest-by-time; doesn't "interpret"
- **实事求是:** real disk reads + real disk writes + deterministic output
- **任何人都能接手:** CLI + Markdown + 1-cmd `overlay` + reproducibility
- **不假装 popper 失败:** 3/53 popper failures hit during dev → all fixed
  before commit (CLI overlay RC, CLI summary RC, CLI summary JSON)

---

## Reproducibility

```bash
# Run from promethean/
python -m apeireth.v1378_v1375_x_v1362_history_overlay popper
# → Popper self-tests: 53/53

python -m pytest tests/test_v1378_v1375_x_v1362_history_overlay.py
# → 63 passed

python -m apeireth.v1378_v1375_x_v1362_history_overlay overlay
# → archives=1 ledger=172 with_ledger=1 with_pole_star=1
# → output=V1378_OVERLAY_AUTO.md

python -m apeireth.v1378_v1375_x_v1362_history_overlay summary
# → {n_archives: 1, n_ledger: 172, n_with_ledger: 1, ...}
```

---

## Files added

| Path | Bytes | Purpose |
|------|------:|---------|
| `apeireth/v1378_v1375_x_v1362_history_overlay.py` | ~38 KB | source module (10 API + 10 GUARDS + 53 Popper) |
| `tests/test_v1378_v1375_x_v1362_history_overlay.py` | ~24 KB | 63 pytest |
| `V1378_OVERLAY_AUTO.md` | ~2 KB | auto-generated overlay from real V1375 × V1362 data |
| `V1378_REPORT.md` | this file | full honest disclosure |

---

## Next (V1379+ candidates, open)

From V1376's next-step list (now 3/5 done):

- **V1379** = V1375 + V1361 streamlit dashboard overlay
  (annotate V1375 archives with V1361 trend-stream context)
- **V1380** = V1375 archival rotation (compress old archives beyond N)
- **V1381** = V1375 + V1378 paired cross-check (verify overlay matches
  sidecar observations)

Other candidates (from earlier V1365+ queue):

- V1364 had a `record-all` next-step; partially done in V1367
- VCP cookbook overlay extensions (V1366+): more VCP rings to validate
- Rust substrate: R38+ batch (current `Apeireth-rust/` work paused at R37-2)