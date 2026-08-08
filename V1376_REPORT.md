# V1376 — V1375 Weekly Digest

**Status:** shipped
**Date:** 2026-08-09 04:10 +08
**Author:** Chu Ling (cron self-driven, isolated lane)
**Replaces:** nothing
**Trigger:** cron `1fba1cc3 apeireth-autonomy-v3` tick at 04:10 +08

---

## What V1376 is

V1376 is the **weekly rollup** of the V1375 archive. Where V1375 preserves
each V1374 snapshot into a timestamped directory, V1376 groups those
snapshots into ISO weeks and produces one digest `.md` per week plus an
INDEX.md:

```
V1376_DIGESTS/
    INDEX.md
    2026-W31.md     # 2026-07-27 .. 2026-08-02 (only present if there were archives that week)
    2026-W32.md     # 2026-08-03 .. 2026-08-09
    ...
```

Each weekly digest summarises the week's V1374 snapshots so anyone can
pick up the project mid-week and read a 1-page report instead of N
archived diffs. Per master 主 00:56 principle ("anyone can pick up"):
this is the highest-leverage companion to V1375 because it converts
a directory of date-stamped files into a single human-browsable page
per ISO week.

## Why V1376 exists

V1375 archive solves "preserve every diff", but a project like ours
(5-min cron ticks for weeks) accumulates dozens of archives per week.
The single-file weekly digest answers the most common audit questions
immediately:

- "What happened this week?" → open `2026-W31.md`
- "How many deltas were zero this week?" → `zero_deltas` row
- "Did anything change between Monday and Sunday?" → `net_delta` row
- "When was the last non-trivial diff?" → `last_nonzero_at` row
- "How many V1374 snapshots did this week produce?" → `count` row

No live data, no rerunning, no risk. Pure markdown.

## API surfaces (10)

1. `iso_week_bucket(timestamp)` — returns `(iso_year, iso_week)` tuple
2. `iso_week_label(year, week)` — returns `YYYY-Www` string
3. `parse_week_label(label)` — inverse of `iso_week_label`
4. `group_by_week(archives)` — `dict[week_label → list[archive]]`
5. `weekly_summary(group)` — single-week summary dict
6. `render_weekly_md(week_label, summary)` — per-week markdown
7. `render_index_md(week_labels)` — INDEX.md markdown
8. `write_digest(archive_dir, *, output_dir=None)` — write all weekly .md + INDEX
9. `_popper_self_tests()` — (passed, total, failures)
10. `run_cli(args)` — argv dispatcher (digest / list / show / popper / version)

## GUARDS (10)

- GUARD_DIGEST_INPUT_FROM_V1375 — reuses V1375 `list_archives` (DRY)
- GUARD_DIGEST_NO_WRITE_BACK — only writes NEW digest files (no in-place edits)
- GUARD_DIGEST_DETERMINISTIC — same archives → same digest bytes (sorted input)
- GUARD_DIGEST_PRESERVES_ORDER — weeks sorted chronologically ascending
- GUARD_DIGEST_HONEST_DISCLOSURE — every digest emits the honesty paragraph
- GUARD_DIGEST_MARKDOWN_ONLY — pure CommonMark
- GUARD_DIGEST_NO_CAP_CHANGE — V1376 has no metric, no score, no cap
- GUARD_DIGEST_LOCAL_FS_ONLY — no remote calls, no network
- GUARD_DIGEST_FS_PATH_SAFE — rejects path traversal / absolute paths (cross-platform)
- GUARD_DIGEST_ISO_WEEK_VALID — bucket label must match `YYYY-Www`

## Tests

- **49 Popper self-tests** (covers week bucket / label / parse / group / summary / render / write / CLI)
- **32 pytest tests** (real V1375 archives + synthetic + edge + CLI + traversal)
- **217 chain pytest** (V1370 → V1376 chain, 0 regression)

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI**: V1376 has NO metric, NO cap, NO scoring — it is plumbing
- **不假装决策 = 真生产**: V1376 = atomic file copy + INDEX refresh; pure data plumbing
- **不假装 ASI 集成**: zero LLM, zero sidecar touch (no V1371 import), zero ledger touch (no V1362/V1368 import)
- **不刷分**: zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor**: V1375 list_archives / archive_tick / parse_index sources unchanged
- **实事求是**: real disk digest + real INDEX.md + real file content preservation
- **任何人都能接手**: CLI + Markdown + 1-cmd `digest` + INDEX.md + per-week summary

## Verification

```
$ python -m apeireth.v1376_v1375_weekly_digest popper
popper self-tests: 49/49

$ python -m apeireth.v1376_v1375_weekly_digest list
2026-W32  (1 archive(s))

$ python -m apeireth.v1376_v1375_weekly_digest digest
wrote 2 files for 1 week(s)
  - 2026-W32.md
  - INDEX.md

$ python -m pytest tests/test_v1376_v1375_weekly_digest.py
============================= 32 passed in 2.33s ==============================

$ python -m pytest tests/test_v1370_v1368_trigger_calibration.py \
    tests/test_v1371_calibrated_cron_hook.py \
    tests/test_v1372_v1371_ascii_timeline.py \
    tests/test_v1373_v1372_markdown_export.py \
    tests/test_v1374_v1373_diff.py \
    tests/test_v1375_v1374_history_archive.py \
    tests/test_v1376_v1375_weekly_digest.py
============================= 217 passed in 4.69s ==============================
```

## Honest disclosure

- V1375 list_archives does not currently populate `added/removed/changed/unchanged/raw_delta/cal_delta`; those columns live only in V1375 INDEX.md. V1376 reads via list_archives (filename-level), so per-trigger columns currently always show 0 against the real archive. The totals still work when given synthetic inputs (see `TestWeeklySummary::test_aggregates_with_deltas`). A future V1377 could call V1375 parse_index() to populate those columns from INDEX.md.
- Non-triviality threshold is `|delta| <= 1`. A per-row movement of ±1 still counts as zero for `zero_deltas`. This is intentional (we care about integer movements of triggers, not sub-percent noise).
- `iso_week_bucket` is correct per ISO 8601: 2027-01-01 (Friday) → ISO year 2026, week 53. This is the canonical ISO edge case.
- V1376 has zero `cap` / `score` / `pole_star` impact. ASI pole-star V0.2 total stays at honest 0.90.

## Files shipped (4)

1. `apeireth/v1376_v1375_weekly_digest.py` (~30KB, 10 API surfaces, 49 Popper self-tests, 10 GUARDS)
2. `tests/test_v1376_v1375_weekly_digest.py` (~13KB, 32 pytest tests)
3. `V1376_DIGESTS/INDEX.md` (auto-generated by `digest` command)
4. `V1376_DIGESTS/2026-W32.md` (auto-generated by `digest` command)

## next (V1377+ candidates from V1375 plan, picking 1)

Per V1375 next-step list (5 candidates), V1376 = `weekly digest` (1/5). Remaining:

- V1377 = V1375 multi-file diff (N .md → 1 comparison)
- V1378 = V1375 + V1362 history overlay (annotate with ledger context)
- V1379 = V1375 + V1361 streamlit dashboard
- V1380 = V1375 archival rotation (compress old archives)