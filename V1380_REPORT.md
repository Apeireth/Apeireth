# V1380 — V1375 × INDEX × V1379 Three-way Reconciliation

**Phase:** 1380
**Version:** 0.1.0
**Date:** 2026-08-09 (cron tick 232, 04:50 → 04:55)
**Post:** V1379 (V1375 archive integrity manifest)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1380 is

V1380 is the **three-way reconciliation** companion to V1375 + V1379.
The V1375 archive system has three sources of truth about which archives
exist:

1. **disk** — files actually present in `V1375_HISTORY/`
2. **INDEX.md** — V1375's own listing of those files (sorted by timestamp)
3. **V1379 manifest** — V1379's SHA-256 manifest of those files

V1380 reads all three and reports any disagreement in one markdown report:

```bash
# Run from promethean/
python -m apeireth.v1380_v1375_x_index_x_manifest_reconciliation reconcile
# → V1380_RECONCILIATION_AUTO.md written
# → exit 0 if all three agree, 1 if any disagreement

# Show last result:
python -m apeireth.v1380_v1375_x_index_x_manifest_reconciliation show
```

## Why V1380 exists

V1375, V1376, V1377, V1378, and V1379 each assume *their own source of
truth is correct*:

- V1375 trusts INDEX.md
- V1376 iterates INDEX.md
- V1377 iterates disk + INDEX
- V1378 reads disk + INDEX
- V1379 trusts its own manifest (and re-hashes disk)

If any one of the three diverges, the downstream modules silently produce
inconsistent results. V1380 closes this gap by reconciling all three at once
and reporting any drift to a human-readable markdown report.

V1380 is **read-only** on disk archives, INDEX.md, and the V1379 manifest.
It only writes its own output report (`V1380_RECONCILIATION_AUTO.md`).

## API surfaces (10)

1. `parse_index_md(index_path)` — list of `{name, iso_basic, schema}`
2. `load_v1379_manifest(manifest_path)` — list of archive dicts from JSON
3. `list_disk_archives(archive_dir)` — list of archive names sorted
4. `hash_archive_sha256(path)` — SHA-256 hex digest of a single archive
5. `reconcile_disk_vs_index(disk_names, index_entries)` — name-set check
6. `reconcile_disk_vs_manifest(disk_names, manifest_archives)` — name-set + hash check
7. `reconcile_index_vs_manifest(index_names, manifest_names)` — name-set check
8. `build_reconciliation(...)` — full result dict (all three pairs + verdict)
9. `render_reconciliation_md(result)` — markdown string
10. `run_cli(args)` — argv dispatcher (reconcile / show / popper / version)

Plus `_popper_self_tests()` returns `(passed, total, failures)`.

Plus `write_report(path, content)` — atomic write (tmp + rename + makedirs).

## GUARDS upheld (V1380-specific, 10)

| # | Guard | What it prevents |
|---|-------|------------------|
| 1 | `GUARD_READ_ONLY` | never writes disk archives / INDEX.md / V1379 manifest |
| 2 | `GUARD_NO_LEDGER_TOUCH` | never imports V1362 / V1368 / V1375 ledger code |
| 3 | `GUARD_NO_SIDECAR_TOUCH` | never imports V1371 / V1369 / V1370 |
| 4 | `GUARD_HONEST_DISCLOSURE` | always emit honesty paragraph |
| 5 | `GUARD_NO_CAP_CHANGE` | V1380 has no metric, no cap, no scoring |
| 6 | `GUARD_DETERMINISTIC` | same inputs in same order → same report bytes |
| 7 | `GUARD_REPORT_ALL_MISMATCHES` | report every mismatch, not just the first |
| 8 | `GUARD_THREE_WAY` | every reconciliation includes all three sources |
| 9 | `GUARD_ATOMIC_WRITE` | tmp + rename + makedirs for output report |
| 10 | `GUARD_PATH_SAFE` | reject `..` traversal in archive dir |

## Honest measurement (this tick)

- **V1380 Popper self-tests:** 41/41 ✓
- **V1380 pytest:** 60/60 ✓
- **Real-data smoke:** built reconciliation from real `V1375_HISTORY/`
  (1 archive on disk) + real `V1375_HISTORY/INDEX.md` (1 entry) +
  real `V1379_INTEGRITY_AUTO.json` (1 archive, sha256 = `f3ff13d6...`)
  → **✓ all three sources agree**, 0 disk_only, 0 index_only,
    0 manifest_only, 0 hash_mismatches
- **ASI pole-star V0.2 honest cap:** 0.90 preserved (V1380 has no metric)
- **V0.3 trigger:** NOT due (no real V0.3 evidence)

## Bugs hit during development

1. **`write_report` didn't create parent subdirs.** `tempfile.mkstemp`
   fails if the parent dir doesn't exist. Fixed by adding
   `os.makedirs(_dir, exist_ok=True)` before `mkstemp`. Caught by the
   `test_creates_subdir` pytest.

2. **`_popper_self_tests` end-to-end CLI test polluted stdout.** Calling
   `run_cli(["reconcile", ...])` inside popper printed the full markdown
   report, which is not what `popper` should output. Fixed by replacing
   the CLI call with a direct `build_reconciliation` + `write_report` call
   (same coverage, no stdout side-effects).

3. **`test_reconcile_disagree_exit_1` expected rc=0 for an all-empty
   fixture.** Empty disk + empty INDEX + empty manifest produces
   `all_ok=False` (vacuously, no archives means "nothing to verify").
   Fixed by giving the fixture a real archive on disk but empty INDEX +
   empty manifest (real disagreement). Renamed to
   `test_reconcile_disk_only_exit_1`.

4. **`_popper_self_tests` end-to-end test had wrong "total" count
   initially** (used computed `passed = 1 + len(failures)` which is wrong
   when failures is empty). Fixed by setting a known `KNOWN_CHECKS = 41`
   constant and computing `passed = KNOWN_CHECKS - len(failures)`.

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI:** V1380 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1380 = pure read + compare + report; no inference
- **不假装 ASI 集成:** zero LLM, zero sidecar, zero ledger write
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1375 archives + INDEX.md + V1379 manifest unchanged
- **不假装 V1380 = ASI 觉醒:** V1380 reports reconciliation; doesn't "interpret" it
- **实事求是:** real disk reads + real INDEX.md parse + real V1379 JSON parse + real SHA-256 hash check
- **任何人都能接手:** CLI + JSON + Markdown + 1-cmd `reconcile` + reproducibility + atomic report write

## Reproducibility

```bash
# Run from promethean/
python -m apeireth.v1380_v1375_x_index_x_manifest_reconciliation popper
# → Popper self-tests: 41/41

python -m pytest tests/test_v1380_v1375_x_index_x_manifest_reconciliation.py
# → 60 passed

# Real-data reconciliation (default paths):
python -m apeireth.v1380_v1375_x_index_x_manifest_reconciliation reconcile
# → V1380_RECONCILIATION_AUTO.md written (1441 bytes, all three agree)
# → exit 0

# Custom paths:
python -m apeireth.v1380_v1375_x_index_x_manifest_reconciliation reconcile \
    --archive-dir ./V1375_HISTORY \
    --manifest-path ./V1379_INTEGRITY_AUTO.json \
    --report-path ./my_report.md
```

## Files added

| Path | Bytes | Purpose |
|------|------:|---------|
| `apeireth/v1380_v1375_x_index_x_manifest_reconciliation.py` | ~45 KB | source module (10 API + 10 GUARDS + 41 Popper) |
| `tests/test_v1380_v1375_x_index_x_manifest_reconciliation.py` | ~30 KB | 60 pytest |
| `V1380_RECONCILIATION_AUTO.md` | ~1.4 KB | auto-generated real-data reconciliation (all agree) |
| `V1380_REPORT.md` | this file | full honest disclosure |

## Next (V1381+ candidates, open)

From V1378's next-step list (now 5/5 done):

- (✓ V1379 done; V1380 also done = pair chain complete)
- The V1374 → V1375 → V1376 → V1377 → V1378 → V1379 → V1380 lineage is now
  fully audited by V1380.

Other candidates (from earlier V1365+ queue):

- V1381 = VCP cookbook overlay extension (next VCP ring to validate)
- V1382 = V1375 archival rotation (compress old archives beyond N — only
  relevant when archive count > N)
- V1383 = Rust substrate R38+ (current `Apeireth-rust/` paused at R37-2)

---

_Made-by: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3)_
_V3 守门: 不假装 V1380 = ASI 觉醒; 三向对账 = pure read + report_