# V1379 — V1375 Archive Integrity Manifest

**Phase:** 1379
**Version:** 0.1.0
**Date:** 2026-08-09 (cron tick 231, 04:42 → 04:50)
**Post:** V1378 (V1375 × V1362 history overlay)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1379 is

V1379 is the **integrity companion** to V1375. Where V1375 writes timestamped
`.md` archives into `V1375_HISTORY/` (one archive per cron tick), and V1378
overlays those archives with V1362 pole-star ledger entries, **V1379 records
the SHA-256 hash of every archive and verifies it on every subsequent run.**

It is the missing primitive: *anyone can prove the V1375 archive has not been
tampered with, in one command.*

```bash
# Build the integrity manifest (atomic write):
python -m apeireth.v1379_v1375_archive_integrity build

# Verify the manifest against current disk state (any human can run):
python -m apeireth.v1379_v1375_archive_integrity verify
# → ✓ all good  /  ✗ N tampered (with details)
```

## Why V1379 exists

V1375 + V1376 + V1377 + V1378 all assume the V1375 archive is intact:

- V1375 archives V1374 diff snapshots
- V1376 produces weekly digests from those archives
- V1377 produces multi-file diffs across those archives
- V1378 overlays archives with pole-star ledger entries

If an archive is corrupted (truncated, partially overwritten, or replaced),
all downstream layers will silently mis-attribute. V1379 closes this gap:

- Each archive gets a SHA-256 content hash at build time
- Each archive is re-hashed at verify time
- Mismatches are reported with archive name, expected hash, actual hash, expected size, actual size
- Missing / extra archives are also reported (the manifest knows what's there)
- Verify is **read-only**: it never modifies the manifest or the archives

## API surfaces (10)

1. `hash_archive(path)` — SHA-256 hex digest of a single archive
2. `scan_archives(archive_dir)` — list of dicts (name, path, sha256, size, mtime, iso, schema)
3. `build_manifest(archives, *, archive_dir)` — build the manifest dict
4. `verify_against_manifest(manifest_path, archive_dir)` — (ok, mismatches, missing, extra)
5. `render_manifest_json(manifest)` — deterministic JSON string
6. `render_verify_report_md(verify_result, *, archive_dir)` — markdown report
7. `write_manifest(path, manifest)` — atomic write (tmp + rename)
8. `load_manifest(path)` — load manifest JSON from disk
9. `_popper_self_tests()` — 56 self-checks
10. `run_cli(args)` — argv dispatcher (build / verify / show / popper / version)

## GUARDS upheld (V1379-specific)

| # | Guard | What it prevents |
|---|-------|------------------|
| 1 | `GUARD_HASH_SHA256_ONLY` | only SHA-256 (no MD5/SHA1; no collisions tolerable) |
| 2 | `GUARD_ATOMIC_WRITE` | tmp + rename for the manifest (no partial writes) |
| 3 | `GUARD_NO_SIDECAR_TOUCH` | never imports V1371 / V1369 / V1370 |
| 4 | `GUARD_NO_LEDGER_TOUCH` | never imports V1362 / V1368 / V1375 |
| 5 | `GUARD_VERIFY_READ_ONLY` | verify never modifies the manifest or archives |
| 6 | `GUARD_REPORT_ALL_MISMATCHES` | verify reports every mismatch, not just the first |
| 7 | `GUARD_HONEST_DISCLOSURE` | always emit honesty paragraph |
| 8 | `GUARD_NO_CAP_CHANGE` | V1379 has no metric, no cap, no scoring |
| 9 | `GUARD_DETERMINISTIC` | same inputs in same order → same manifest bytes |
| 10 | `GUARD_NO_FAKE_REPAIR` | verify does not "auto-fix"; it only reports |

## Honest measurement (this tick)

- **V1379 Popper self-tests:** 56/56 ✓
- **V1379 pytest:** 71/71 ✓
- **Chain pytest (V1370 → V1379):** 416/416 ✓ (no regression)
- **Chain popper (V1370 → V1379):** 485/485 ✓ (no regression)
- **ASI pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)
- **Real-data smoke test:** built manifest from real `V1375_HISTORY/`
  (1 archive) → verified OK with 0 mismatches / 0 missing / 0 extra

## Bugs hit during development

1. **V1375 archive slug regex required extended ISO format** (with colons).
   The original test asserted that `2026-08-09T04:00:00Z__v1374.md` (with
   colons) would match. But V1375 only writes basic ISO (filesystem-safe,
   dashes instead of colons). Fixed by changing the test to assert
   *rejection* of the extended format, which is the correct behavior.

2. **`_parse_iso_basic("2026-08-09T12-00-00+0800")` returned wrong offset**
   in popper self-test. The function correctly converts to UTC via
   `astimezone(timezone.utc)`, which zeroes the offset for the assertion.
   The test was checking offset seconds *after* UTC conversion. Fixed by
   checking the converted UTC hour (12 - 8 = 4) instead.

3. **`manifest_json_archives_count` test counted the wrong substring.**
   The literal string `"sha256"` (with quotes) appears both as the key
   in each archive AND as the value of `hash_algorithm`. The test was
   splitting on `"archives":` to count inside the archives section, but
   the split boundary was off by one (the second half included everything
   after the archives array). Fixed by counting `"sha256":` (key + colon)
   which is unambiguous — only the per-archive keys use that pattern.

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI:** V1379 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1379 = pure read + hash + report; no inference
- **不假装 ASI 集成:** zero LLM, zero sidecar, zero ledger write
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1375 archives unchanged; V1379 only reads + records hashes
- **不假装 V1379 = ASI 觉醒:** V1379 reports integrity; doesn't "interpret" it
- **实事求是:** real disk reads + real disk writes + deterministic SHA-256
- **任何人都能接手:** CLI + JSON + Markdown + 1-cmd `verify` + reproducibility
- **不假装 popper 失败:** 3/56 popper failures hit during dev → all fixed
  before commit (slug_accepts_extended_iso, iso_basic_parses_offset,
  manifest_json_archives_count)

## Reproducibility

```bash
# Run from promethean/
python -m apeireth.v1379_v1375_archive_integrity popper
# → Popper self-tests: 56/56

python -m pytest tests/test_v1379_v1375_archive_integrity.py
# → 71 passed

# Build manifest from real V1375_HISTORY:
python -m apeireth.v1379_v1375_archive_integrity build --quiet
# → V1379_INTEGRITY_AUTO.json written

# Verify (read-only):
python -m apeireth.v1379_v1375_archive_integrity verify --quiet
# → V1379 verify OK: 1 archives, all hashes match
# → exit code 0 = pass, 1 = fail
```

## Files added

| Path | Bytes | Purpose |
|------|------:|---------|
| `apeireth/v1379_v1375_archive_integrity.py` | ~38 KB | source module (10 API + 10 GUARDS + 56 Popper) |
| `tests/test_v1379_v1375_archive_integrity.py` | ~33 KB | 71 pytest |
| `V1379_INTEGRITY_AUTO.json` | ~0.5 KB | auto-generated integrity manifest |
| `V1379_VERIFY_AUTO.md` | ~0.7 KB | auto-generated verify report |
| `V1379_REPORT.md` | this file | full honest disclosure |

## Next (V1380+ candidates, open)

From V1378's next-step list (now 4/5 done):

- **V1380** = V1375 archival rotation (compress old archives beyond N)
- **V1381** = V1375 + V1379 paired cross-check (verify overlay matches
  the integrity manifest hash for each archive)

Other candidates (from earlier V1365+ queue):

- V1364 had a `record-all` next-step; partially done in V1367
- VCP cookbook overlay extensions (V1366+): more VCP rings to validate
- Rust substrate: R38+ batch (current `Apeireth-rust/` work paused at R37-2)
