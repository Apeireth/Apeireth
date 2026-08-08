# V1375 — V1374 History Archive

**Phase:** 1375
**Version:** 0.1.0
**Date:** 2026-08-09 (tick 228)
**Post:** V1374 (V1373 markdown diff mode)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1375 is

V1375 is the **history companion** to V1374. Where V1374 writes a single
diff snapshot to ``V1374_REPORT_AUTO.md`` (overwritten each tick), V1375
preserves every V1374 snapshot into a timestamped archive directory:

```text
V1375_HISTORY/
    INDEX.md
    2026-08-08T20-06-51Z__v1374.md
    ...
```

V1375 serves the same hard constraint as the rest of the project: **anyone
can pick this up without asking me**. With one command you get a clean
dated archive of every V1374 diff ever produced:

```bash
# Run from cron tick:
python -m apeireth.v1375_v1374_history_archive archive
# Produces: V1375_HISTORY/<timestamp>__v1374.md + INDEX.md

# Browse all history:
python -m apeireth.v1375_v1374_history_archive list
# Browse a specific archived report:
python -m apeireth.v1375_v1374_history_archive show 2026-08-08T20-06-51Z
```

## Why V1375 exists

V1374 produces a snapshot per cron tick. After several ticks, the single
``V1374_REPORT_AUTO.md`` file is overwritten — the previous diffs are lost.

V1375 is the missing primitive:

- Preserve every V1374 diff (no loss across ticks)
- Allow chronological queries ("what changed between 04:00 and 04:10?")
- Allow diff-of-diffs analysis ("compare today's diff to last week's diff")
- Allow audit trails ("when did the suppression ratio first exceed 0.5?")
- Enable the V1376+ candidates (weekly digest, multi-file diff, history overlay)

All from a directory of plain `.md` files. No live data, no rerunning, no risk.

## 10 API surfaces

1. `slug_timestamp(dt=None)` — ISO timestamp suitable for filenames
2. `archive_name(timestamp, schema='v1374')` — `2026-08-08T20-06-51Z__v1374.md`
3. `archive_report(report_path, archive_dir, *, timestamp=None, schema='v1374')` — copy + collision-safe
4. `list_archives(archive_dir)` — list dicts sorted by timestamp ascending
5. `parse_index(archive_dir)` — parse existing INDEX.md
6. `render_index_md(archives, summaries=None, *, title=None)` — markdown string
7. `write_index(archive_dir, archives, summaries=None, *, title=None)` — atomic write
8. `archive_tick(archive_dir, report_path, *, timestamp=None, schema='v1374')` — all-in-one
9. `_popper_self_tests()` — (passed, total, failures)
10. `run_cli(args)` — argv dispatcher (archive / list / show / index / digest / popper / version)

## GUARDS upheld (V1375-specific)

- GUARD_HISTORY_ADDS_ONLY: never overwrites another archive file (collision-safe via `_001`-`_NNN`)
- GUARD_ATOMIC_WRITE: tmp + rename for both archive and INDEX.md
- GUARD_NO_SIDECAR_TOUCH: archive only reads V1374 `.md`; no V1371 import
- GUARD_NO_LEDGER_TOUCH: no V1362/V1368 import
- GUARD_HONEST_DISCLOSURE: always emit honesty paragraph
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1375 does not write back to any cap
- GUARD_INDEX_ALWAYS_SORTED: INDEX.md rows sorted by timestamp ascending
- GUARD_LOCAL_FILESYSTEM_ONLY: no remote calls, no network
- GUARD_FS_PATH_SAFE: rejects path traversal (`../` segments)

## Tests

- 49 Popper self-tests (covers slug / name / archive / list / parse_index / render / write / tick / CLI / path safety / collision resolution / custom schema / custom title / parse_int_delta / format_signed)
- 40 pytest tests (real V1374 files + synthetic + edge + CLI subprocess + collision schema separation)

## Honest measurement (this tick)

- **V1375 Popper self-tests:** 49/49 ✓
- **V1375 pytest:** 40/40 ✓
- **Chain pytest (V1368 → V1375):** 278/278 ✓ — no regression
- **Chain popper (V1368 + V1369 + V1370 + V1371 + V1372 + V1373 + V1374 + V1375):** 310/310 ✓
- **ASI pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)
- **No KPI inflation, no fake fires, no closed doors**

## What was actually archived

Real run from `promethean/`:

```text
$ python -m apeireth.v1375_v1374_history_archive archive
[V1375] archived 1 files
  archive: V1375_HISTORY/2026-08-08T20-06-51Z__v1374.md
  index:   V1375_HISTORY/INDEX.md
  ts:      2026-08-08T20-06-51Z
```

`V1375_HISTORY/INDEX.md` (excerpt):

```markdown
# V1375 — V1374 History Archive

- **schema:** `v1375.history/v1`
- **archives:** 1
- **first:** `2026-08-08T20-06-51Z`
- **last:**  `2026-08-08T20-06-51Z`

| archived | schema | added | removed | changed | unchanged | raw Δ | cal Δ | gap |
|----------|--------|------:|--------:|--------:|----------:|------:|------:|-----|
| `2026-08-08T20-06-51Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |
```

The honest baseline is preserved: 1 archive, 0 fires, 8 unchanged,
honest 0.90 cap. Adding more archive ticks will only grow the
deltas field; the cap and triggers remain unchanged.

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- **不假装分数 = ASI:** V1375 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1375 = atomic file copy + INDEX refresh; no logic
- **不假装 ASI 集成:** zero LLM, zero sidecar touch, zero ledger touch
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1368/V1371 sources unchanged
- **不假装 V1375 = ASI 觉察历史:** V1375 preserves file diffs, doesn't "interpret" them
- **实事求是:** real disk archive + real INDEX.md + real sha256 of inputs
- **任何人都能接手:** CLI + Markdown + 1-cmd archive + INDEX.md

## Bug fix during V1375 development

**Regex collision schema extraction** — first Popper run failed `list_archives_schema`
(1/49). Root cause: `v1374_001.md` was being parsed as `schema=v1374_001`
instead of `schema=v1374` + `collision=001`. Fix: changed schema group from
`v\d+(?:_[a-zA-Z0-9]+)*` (greedy) to `[a-zA-Z0-9_]+?` (lazy) so the optional
`_NNN` collision suffix is captured separately. All 49 Popper self-tests pass
after fix.

## Files

- `apeireth/v1375_v1374_history_archive.py` — ~33 KB, 10 API surfaces, 49 Popper self-tests
- `tests/test_v1375_v1374_history_archive.py` — ~17 KB, 40 pytest tests
- `V1375_REPORT.md` — this document
- `V1375_HISTORY/INDEX.md` — auto-generated index of archived V1374 reports
- `V1375_HISTORY/2026-08-08T20-06-51Z__v1374.md` — first archived V1374 snapshot

## Reproducibility

```bash
# Popper self-tests:
python -m apeireth.v1375_v1374_history_archive popper -v

# Pytest:
python -m pytest tests/test_v1375_v1374_history_archive.py -v

# Run from a project root that has V1374_REPORT_AUTO.md:
python -m apeireth.v1375_v1374_history_archive archive
python -m apeireth.v1375_v1374_history_archive --archive-dir V1375_HISTORY list -v
```

---

## Next (V1376+ candidates, open)

1. V1376 = V1375 weekly digest (roll up N archives into a single summary)
2. V1376 = V1375 multi-file diff (combine N .md files into one comparison)
3. V1376 = V1375 + V1362 history overlay (annotate diffs with ledger context)
4. V1376 = V1375 + V1361 streamlit dashboard integration
5. V1376 = V1375 history archival rotation (compress old archives; keep recent)

---

_Generated by Chu Ling (楚零) for Apeireth ASI — cron tick 228, 2026-08-09 04:04 +08:00._
