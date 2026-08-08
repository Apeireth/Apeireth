# V1383 — V1382 Cron-Driven Snapshot Tick + Archive-Health Dashboard

**Phase:** 1383
**Version:** 0.1.0
**Date:** 2026-08-09 (cron tick 235, 05:20 → 05:34)
**Post:** V1382 (V1375 × V1379 × V1381 archive-health overlay)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1383 is

V1383 is the **operational seam** between V1382 (archive-health overlay
snapshot) and a real cron lane. Where V1382 is a single-shot CLI
(`snapshot` / `summary` / `popper` / `version`), V1383 turns that into:

1. **`tick`** — compute the current V1382 snapshot, wrap it in a tick
   record (schema `v1383.tick/v1`), atomically append to a JSONL ledger
   `V1383_TICKS.jsonl`. Idempotent under repeated calls (deterministic
   tick_id = hash of (schema, ts, microsecond), no global counter).
2. **`ticker`** — `tick` + `V1382 --record` (dual-write: V1383 ledger
   gets the tick; V1382 history gets the raw snapshot). One cron line.
3. **`show-last [--n N]`** — print the last N tick records, one line each.
4. **`summary`** — one-line summary: tick count + latest integrity.
5. **`dashboard [--out PATH]`** — render a markdown dashboard from the
   ledger (latest tick header + drift + last-N table + integrity panel +
   10 GUARDS + known unknowns).
6. **`drift`** — emit JSON drift record (last two ticks).
7. **`popper`** — 52 Popper self-tests.
8. **`version`** — schema + version.

## Why V1383 (per V1380 next-step 3/3 + 2026-08-09 daily decision)

V1382 answered "what does the V1375 archive subsystem look like *right
now*?" V1383 answers the cron-side question: "what does it look like
*over time*, and is it drifting?"

This is the bridge between archive machinery (V1375/V1379/V1381/V1382)
and operational reality (a 5-minute cron lane that wants a ledger it
can replay). The cron side never re-derives V1382's logic — V1383 just
imports `snapshot_archive_health()` and wraps it.

The 2026-08-09 daily memory decided Option C (cron auto-run + dashboard)
over Rust R38+ and ASI 5-gap deep-dives, because:
- V1382 snapshot already works; needs operationalization, not new theory
- "任何人都能接手" = the dashboard is the human-readable artifact
- Rust R38+ is large engineering; needs toolchain readiness confirmation

## Tick schema (v1383.tick/v1)

```json
{
  "schema": "v1383.tick/v1",
  "tick_id": "tick-2026-08-09T05-25-00Z-f03d",
  "ts": "2026-08-09T05:25:00Z",
  "v1383_version": "0.1.0",
  "v1382_snapshot": { ... full V1382 snapshot ... },
  "tag": "cron-5min",
  "first_tick": false,
  "drift_from_previous": {
    "previous_tick_id": "tick-2026-08-09T05-20-00Z-b5e1",
    "previous_ts": "2026-08-09T05:20:00Z",
    "current_tick_id": "tick-2026-08-09T05-25-00Z-f03d",
    "archives_delta": 0,
    "integrity_status_delta": "ok->ok",
    "tier_distribution_delta": {"HOT": 0, "WARM": 0, "COLD": 0, "FROZEN": 0},
    "action_counts_delta": {"keep": 0, "compress": 0, "prune": 0},
    "integrity_changed": false,
    "archives_changed": false
  },
  "guards": [... 10 guards ...],
  "known_unknowns": [... 3 honest unknowns ...]
}
```

When `tick` is the first call (empty ledger), `first_tick: true` and
`drift_from_previous: null`.

## Dashboard schema (v1383.dashboard/v1)

Markdown document with:

- **Header**: latest tick id, ts, v1383_version, tag
- **Latest totals**: archives / indexed / manifested + integrity badge
- **Tier distribution**: HOT/WARM/COLD/FROZEN table
- **Rotation actions (V1381 plan)**: keep/compress/prune table
- **Drift panel**: last-vs-previous deltas (or "first tick" note)
- **Last N ticks table**: ts | tick_id | archives | integrity | tag
- **Guards list**: 10 GUARDS upheld
- **Known unknowns**: 3 honest unknowns

## CLI

```bash
# From promethean/
python -m apeireth.v1383_v1382_cron_tick tick --tag cron-5min
# → 2026-08-08T21:23:46Z tick-...-b5e1 archives=1 integrity=OK tag=cron-5min

python -m apeireth.v1383_v1382_cron_tick ticker --tag cron-5min
# → tick + V1382 --record dual-write

python -m apeireth.v1383_v1382_cron_tick show-last --n 5
# → last 5 tick records (one line each)

python -m apeireth.v1383_v1382_cron_tick summary
# → V1383 ticks=2 latest=... archives=1 integrity=OK

python -m apeireth.v1383_v1382_cron_tick dashboard --out V1383_DASHBOARD_AUTO.md
# → writes markdown dashboard (atomic)

python -m apeireth.v1383_v1382_cron_tick drift
# → JSON drift record (last two ticks) to stdout

python -m apeireth.v1383_v1382_cron_tick popper
# → 52 self-tests

python -m apeireth.v1383_v1382_cron_tick version
```

NOTE: parent-level flags (`--now`, `--ledger-path`, etc.) must come
BEFORE the subcommand. `--now X tick` is valid; `tick --now X` is not
(argparse convention; documented).

## API surfaces (10)

1. `_validate_safe_path(path)` — reject `..` traversal
2. `_make_tick_id(ts)` — deterministic tick id from timestamp
3. `_v1382_snapshot(*, archive_dir, manifest_path, now)` — lazy V1382 import
4. `_compute_drift(prev, curr)` — pure numeric drift dict
5. `_compute_tick(*, now, tag, ...)` — build full tick record
6. `_atomic_append_jsonl(path, record)` — atomic per-line JSONL append
7. `_read_ticks(ledger_path, *, limit, reverse)` — read N tick records
8. `tick_now(*, tag, ledger_path, ...)` — main tick function (CLI entry)
9. `_render_tick_one_line(tick)` + `_render_dashboard_md(ticks)` — rendering
10. `_popper_self_tests()` + `run_cli(args)` — popper + CLI

## GUARDS upheld (V1383-specific, 10)

| # | Guard | What it prevents |
|---|-------|------------------|
| 1 | `GUARD_CRON_SAFE` | tick is idempotent under repeated calls (deterministic id) |
| 2 | `GUARD_HISTORY_APPEND_ONLY` | V1383_TICKS.jsonl only appended, never truncated |
| 3 | `GUARD_NO_CAP_CHANGE` | V1383 has no metric, no cap, no scoring |
| 4 | `GUARD_DETERMINISTIC` | same inputs → same tick bytes |
| 5 | `GUARD_ATOMIC_WRITE` | tmp + rename for ledger append + dashboard write |
| 6 | `GUARD_NO_TOUCH_V1382` | V1383 calls V1382 API; never modifies V1382 source |
| 7 | `GUARD_LOCAL_FILESYSTEM_ONLY` | no network, no remote FS |
| 8 | `GUARD_HONEST_DISCLOSURE` | known_unknowns always emitted |
| 9 | `GUARD_DASHBOARD_PURE` | dashboard = pure read + render, no synthesis |
| 10 | `GUARD_DRIFT_PURE` | drift = pure numeric compare, no inference |

## Honest measurement (this tick)

- **V1383 Popper self-tests:** 52/52 ✓
- **V1383 pytest:** 50/50 ✓ (incl. 2 real-data smoke tests)
- **Chain pytest V1370-V1383:** 615/615 ✓ (+50 from V1383)
- **Real-data smoke:**
  - `V1383_TICKS.jsonl` — 2 ticks recorded (tick-...-b5e1 + tick-...-f03d)
  - `V1383_DASHBOARD_AUTO.md` — 1772 bytes markdown rendered
  - drift: `archives_delta=0`, `integrity_status_delta=ok->ok`,
    `tier_distribution_delta={HOT:0, WARM:0, COLD:0, FROZEN:0}`
- **ASI pole-star V0.2 honest cap:** 0.90 preserved (V1383 has no metric)
- **V0.3 trigger:** NOT due (no real V0.3 evidence)

## Bugs hit during development

1. **`_popper_self_tests` count drift.** Initial `KNOWN_CHECKS = 40`
   but I wrote 52 actual checks. The popper CLI returns "Popper
   self-tests: 52/40" with a "check count drift" warning. Fixed by
   bumping `KNOWN_CHECKS` to 52.

2. **`compute_tick` does NOT raise on missing archive dir.** V1382's
   `snapshot_archive_health()` returns `archives=0, integrity=ok=False,
   reason='no manifest found'` for missing dirs — it does NOT raise.
   Initial popper test assumed it would raise; fixed by replacing the
   `try/except` with a `try/else` that asserts the empty snapshot
   fields. This documents V1382's actual contract: missing inputs are
   reported, not crashed.

3. **`--now` argparse position.** `--now X tick` works; `tick --now X`
   fails because argparse does not parse parent-level args after a
   subcommand. Documented in the CLI section.

4. **Empty-ledger dashboard.** Initial test asserted
   `assert "v1383.dashboard/v1" in content` for the empty case, but
   the empty-ledger dashboard only contains the "no ticks" message.
   Fixed test to assert `"no ticks" in content.lower()` instead.

5. **Dashboard "Last N ticks" cap.** Dashboard always caps at 10 even
   if more ticks exist (for readability). Test asserted "Last 12 ticks"
   for a 12-tick input; fixed to expect "Last 10 ticks".

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI:** V1383 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** tick = pure V1382 wrapper + ledger append; drift = pure numeric compare
- **不假装 ASI 集成:** zero LLM, zero sidecar import, zero ASI ledger write
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1382 source unchanged; V1375/V1379/V1381 untouched
- **不假装 V1383 = ASI 觉醒:** V1383 records ticks; doesn't interpret them
- **实事求是:** real V1382 snapshot + real disk read + real atomic write
- **任何人都能接手:** CLI + JSON + Markdown + 1-cmd `tick` + atomic ledger + reproducibility

## Reproducibility

```bash
# Run from promethean/
python -m apeireth.v1383_v1382_cron_tick popper
# → Popper self-tests: 52/52

python -m pytest tests/test_v1383_v1382_cron_tick.py
# → 50 passed

python -m pytest tests/test_v1370_v1368_trigger_calibration.py \
    tests/test_v1371_calibrated_cron_hook.py \
    tests/test_v1372_v1371_ascii_timeline.py \
    tests/test_v1373_v1372_markdown_export.py \
    tests/test_v1374_v1373_diff.py \
    tests/test_v1375_v1374_history_archive.py \
    tests/test_v1376_v1375_weekly_digest.py \
    tests/test_v1377_v1375_multi_file_diff.py \
    tests/test_v1378_v1375_x_v1362_history_overlay.py \
    tests/test_v1379_v1375_archive_integrity.py \
    tests/test_v1380_v1375_x_index_x_manifest_reconciliation.py \
    tests/test_v1381_v1375_archival_rotation.py \
    tests/test_v1382_v1375_x_v1381_overlay.py \
    tests/test_v1383_v1382_cron_tick.py
# → 615 passed (chain V1370-V1383)

# Real-data tick + dashboard:
python -m apeireth.v1383_v1382_cron_tick tick --tag cron-5min
# → writes 1 tick to V1383_TICKS.jsonl

python -m apeireth.v1383_v1382_cron_tick dashboard --out V1383_DASHBOARD_AUTO.md
# → writes markdown dashboard (1772 bytes)
```

## Files added

| Path | Bytes | Purpose |
|------|------:|---------|
| `apeireth/v1383_v1382_cron_tick.py` | ~40 KB | source module (10 API + 10 GUARDS + 52 Popper) |
| `tests/test_v1383_v1382_cron_tick.py` | ~30 KB | 50 pytest (incl. 2 real-data smoke) |
| `V1383_DASHBOARD_AUTO.md` | ~1.7 KB | auto-generated real-data dashboard |
| `V1383_TICKS.jsonl` | ~2.7 KB | auto-generated tick ledger (2 ticks) |
| `V1383_REPORT.md` | this file | full honest disclosure |

## Next (V1384+ candidates, open)

From 2026-08-09 daily memory decision:

- **V1384 = V1383 cron hook** (V1371-style integration):
  - 5-min cron auto-tick via `openclaw cron` add command
  - Pure V1383 wrapper (no new logic; just `tick` + V1382 --record)
  - "任何人都能接手" made concrete
- V1385 = V1383 dashboard history overlay (V1383 ledger × V1362 pole-star)
- V1386 = VCP cookbook overlay extension (V1380 next-step 1/3)
- V1387+ = Rust R38+ (V1380 next-step 3/3, large engineering)

---

_Made-by: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3)_
_V3 守门: 不假装 V1383 = ASI 觉醒; cron tick = pure V1382 wrapper + ledger append; drift = pure numeric compare_
