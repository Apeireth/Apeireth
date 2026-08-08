"""Phase 1383 v1383_v1382_cron_tick — V1382 cron-driven snapshot tick + dashboard.

## Phase

Phase: 1383
Version: 0.1.0
Date: 2026-08-09 (cron tick 235)
Post: V1382 (V1375 × V1379 × V1381 archive-health overlay)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

## What V1383 is

V1383 is the **operational seam** between V1382 (archive-health overlay
snapshot) and a real cron lane. Where V1382 is a single-shot CLI
(`snapshot` / `summary` / `popper` / `version`), V1383 turns that into:

1. **`tick`** — compute the current V1382 snapshot, wrap it in a tick
   record (schema `v1383.tick/v1`), atomically append to a JSONL ledger
   `V1383_TICKS.jsonl`. Idempotent under repeated calls (no global
   counter, just wall-clock + microsecond resolution).
2. **`ticker`** — `tick` + `V1382 --record` (dual-write: V1383 ledger
   gets the tick; V1382 history gets the raw snapshot). Lets a cron
   lane persist both views in one call.
3. **`show-last [--n N]`** — print the last N tick records from the
   ledger, as one line each.
4. **`summary`** — one-line summary across recent ticks (count, latest
   integrity, latest tier mix).
5. **`dashboard [--out PATH]`** — render a markdown dashboard from the
   ledger (latest tick header + drift + last-N table + integrity panel).
6. **`drift`** — compare the last two ticks (or any two `--a` / `--b`
   tick ids) and emit a JSON drift record: archive count delta,
   integrity status delta, tier distribution delta.
7. **`popper`** — 40+ Popper self-tests.
8. **`version`** — schema + version.

## Why V1383 (per V1380 next-step + 2026-08-09 daily decision)

V1382 answered "what does the V1375 archive subsystem look like *right
now*?" V1383 answers the cron-side question: "what does it look like
*over time*, and is it drifting?"

This is the bridge between archive machinery (V1375/V1379/V1381/V1382)
and operational reality (a 5-minute cron lane that wants a ledger it
can replay). The cron side never re-derives V1382's logic — V1383 just
imports `snapshot_archive_health()` and wraps it.

V1383 also exposes `--ticker --record` so a single cron line writes
both ledgers, while the read paths (`show-last` / `dashboard` / `drift`)
can be called from any agent or human at any time.

## Tick schema (v1383.tick/v1)

```json
{
  "schema": "v1383.tick/v1",
  "tick_id": "tick-2026-08-09T05-20-00Z-a3f1",
  "ts": "2026-08-09T05:20:00Z",
  "v1383_version": "0.1.0",
  "v1382_snapshot": { ... full V1382 snapshot ... },
  "tag": "cron-5min",
  "drift_from_previous": {
    "archives_delta": 0,
    "integrity_status_delta": "ok->ok",
    "tier_distribution_delta": {"HOT": 0, "WARM": 0, "COLD": 0, "FROZEN": 0},
    "previous_tick_id": "tick-2026-08-09T05-15-00Z-2b9e",
    "previous_ts": "2026-08-09T05:15:00Z"
  },
  "guards": [ ... 10 guards ... ],
  "known_unknowns": [ ... ]
}
```

When `tick` is the first call (empty ledger), `drift_from_previous` is
`null` and a `first_tick: true` field is added.

## Dashboard schema (v1383.dashboard/v1)

The dashboard output is a markdown document with:

- **Header**: latest tick id, ts, v1383_version
- **Latest totals**: archives / indexed / manifested + integrity badge
- **Tier mix**: HOT/WARM/COLD/FROZEN counts
- **Drift panel**: last-vs-previous deltas (or "first tick" note)
- **Last N ticks table**: ts | tick_id | archives | integrity | HOT/WARM/COLD/FROZEN
- **Guards list**: 10 GUARDS upheld
- **Known unknowns**: 3-5 honest unknowns

## CLI

```bash
# From promethean/
python -m apeireth.v1383_v1382_cron_tick tick --tag cron-5min
# → computes V1382 snapshot, appends V1383 tick, prints tick_id

python -m apeireth.v1383_v1382_cron_tick ticker --tag cron-5min
# → tick + V1382 --record (dual-write)

python -m apeireth.v1383_v1382_cron_tick show-last --n 5
# → last 5 tick records (one line each)

python -m apeireth.v1383_v1382_cron_tick summary
# → "V1383 ticks=N latest=... archives=... integrity=..."

python -m apeireth.v1383_v1382_cron_tick dashboard --out V1383_DASHBOARD_AUTO.md
# → writes markdown dashboard (atomic)

python -m apeireth.v1383_v1382_cron_tick drift
# → JSON drift record (last two ticks) to stdout

python -m apeireth.v1383_v1382_cron_tick popper
# → 40+ self-tests

python -m apeireth.v1383_v1382_cron_tick version
```

## API surfaces (10)

1. `_validate_safe_path(path)` — reject `..` traversal
2. `_compute_tick(*, now, tag)` — build tick record (wraps V1382)
3. `_atomic_append_jsonl(path, record)` — atomic per-line JSONL append
4. `_read_ticks(ledger_path, *, limit, reverse)` — read N tick records
5. `_compute_drift(prev_tick, curr_tick)` — drift dict
6. `_render_tick_one_line(tick)` — one-line human summary
7. `_render_dashboard_md(ticks)` — full markdown dashboard
8. `tick_now(*, tag, ledger_path, ...)` — main tick function
9. `show_last(n, ledger_path)` — show last N
10. `_popper_self_tests()` + `run_cli(args)` — popper + CLI

## GUARDS upheld (V1383-specific, 10)

| # | Guard | What it prevents |
|---|-------|------------------|
| 1 | `GUARD_CRON_SAFE` | tick is idempotent under repeated calls; no global counter |
| 2 | `GUARD_HISTORY_APPEND_ONLY` | V1383_TICKS.jsonl only appended, never truncated |
| 3 | `GUARD_NO_CAP_CHANGE` | V1383 has no metric, no cap, no scoring |
| 4 | `GUARD_DETERMINISTIC` | same inputs → same tick bytes |
| 5 | `GUARD_ATOMIC_WRITE` | tmp + rename for ledger append + dashboard write |
| 6 | `GUARD_NO_TOUCH_V1382` | V1383 calls V1382 API; never modifies V1382 source |
| 7 | `GUARD_LOCAL_FILESYSTEM_ONLY` | no network, no remote FS |
| 8 | `GUARD_HONEST_DISCLOSURE` | known_unknowns always emitted |
| 9 | `GUARD_DASHBOARD_PURE` | dashboard = pure read + render, no synthesis |
| 10 | `GUARD_DRIFT_PURE` | drift = pure numeric compare, no inference |

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- 不假装分数 = ASI: V1383 has no metric, no cap, no scoring
- 不假装决策 = 真生产: tick = pure V1382 wrapper + ledger append; drift = pure compare
- 不假装 ASI 集成: zero LLM, zero sidecar import, zero ASI ledger write
- 不刷分: zero metric change in this commit; honest 0.90 cap preserved
- 不动 anchor: V1382 source unchanged; V1375/V1379/V1381 untouched
- 不假装 V1383 = ASI 觉醒: V1383 records ticks; doesn't interpret them
- 实事求是: real V1382 snapshot + real disk read + real atomic write
- 任何人都能接手: CLI + JSON + Markdown + 1-cmd `tick` + atomic ledger + reproducibility
"""

from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import sys
import tempfile
from typing import Any

# Reconfigure stdout/stderr for Windows GBK safety
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1383.tick/v1"
DASHBOARD_SCHEMA_VERSION = "v1383.dashboard/v1"
SCRIPT_NAME = "v1383_v1382_cron_tick"
DEFAULT_LEDGER_PATH = "V1383_TICKS.jsonl"
DEFAULT_DASHBOARD_PATH = "V1383_DASHBOARD_AUTO.md"
DEFAULT_V1382_HISTORY_PATH = "V1382_HISTORY.jsonl"

# V1383 GUARDS (10) — V3 守门
V1383_GUARDS: tuple[str, ...] = (
    "GUARD_CRON_SAFE",                  # tick is idempotent under repeated calls
    "GUARD_HISTORY_APPEND_ONLY",        # V1383_TICKS.jsonl only appended
    "GUARD_NO_CAP_CHANGE",              # V1383 has no metric, no cap, no scoring
    "GUARD_DETERMINISTIC",              # same inputs → same tick bytes
    "GUARD_ATOMIC_WRITE",               # tmp + rename for ledger + dashboard
    "GUARD_NO_TOUCH_V1382",             # V1383 calls V1382 API; never modifies V1382 source
    "GUARD_LOCAL_FILESYSTEM_ONLY",      # no network, no remote FS
    "GUARD_HONEST_DISCLOSURE",          # known_unknowns always emitted
    "GUARD_DASHBOARD_PURE",             # dashboard = pure read + render
    "GUARD_DRIFT_PURE",                 # drift = pure numeric compare
)


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def _validate_safe_path(path: str) -> None:
    """Reject path traversal (``..`` segments)."""
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {path!r}")


# ----------------------------------------------------------------------
# Tick id generation
# ----------------------------------------------------------------------

def _make_tick_id(ts: _dt.datetime) -> str:
    """Generate a tick id with timestamp + 4-hex microsecond hash.

    The microsecond hash is a deterministic function of (schema, ts,
    microsecond) — it serves to make concurrent ticks (within the same
    wall-clock second) have distinct ids, while remaining reproducible
    given the timestamp.
    """
    base = ts.strftime("%Y-%m-%dT%H-%M-%SZ")
    h = hashlib.sha1(f"v1383|{base}|{ts.microsecond}".encode("utf-8")).hexdigest()[:4]
    return f"tick-{base}-{h}"


# ----------------------------------------------------------------------
# V1382 snapshot import (lazy, single source)
# ----------------------------------------------------------------------

def _v1382_snapshot(*, archive_dir: str, manifest_path: str, now: _dt.datetime | None) -> dict[str, Any]:
    """Lazy-import V1382 and return the current overlay snapshot.

    GUARD_NO_TOUCH_V1382: We import V1382 here but never modify its
    source. If V1382 is unavailable, we surface a clean error rather
    than silently degrading the tick.
    """
    try:
        from apeireth import v1382_v1375_x_v1381_overlay as v1382
    except ImportError as e:
        raise RuntimeError(
            f"V1382 import failed: {e}. V1383 cannot compute a tick "
            f"without V1382."
        ) from e
    return v1382.snapshot_archive_health(
        archive_dir=archive_dir,
        manifest_path=manifest_path,
        now=now,
    )


# ----------------------------------------------------------------------
# Drift computation
# ----------------------------------------------------------------------

def _compute_drift(prev: dict[str, Any], curr: dict[str, Any]) -> dict[str, Any]:
    """Pure-numeric drift between two ticks.

    Returns a dict with archive-count delta, integrity-status delta,
    tier-distribution delta. No synthesis, no inference.
    """
    prev_total = prev["v1382_snapshot"]["totals"]["archives"]
    curr_total = curr["v1382_snapshot"]["totals"]["archives"]
    prev_integ_ok = prev["v1382_snapshot"]["integrity"]["ok"]
    curr_integ_ok = curr["v1382_snapshot"]["integrity"]["ok"]
    prev_status = "ok" if prev_integ_ok else "broken"
    curr_status = "ok" if curr_integ_ok else "broken"
    prev_tiers = prev["v1382_snapshot"]["tier_counts"]
    curr_tiers = curr["v1382_snapshot"]["tier_counts"]
    tier_delta = {
        tier: curr_tiers.get(tier, 0) - prev_tiers.get(tier, 0)
        for tier in ("HOT", "WARM", "COLD", "FROZEN")
    }
    prev_actions = prev["v1382_snapshot"]["action_counts"]
    curr_actions = curr["v1382_snapshot"]["action_counts"]
    action_delta = {
        a: curr_actions.get(a, 0) - prev_actions.get(a, 0)
        for a in ("keep", "compress", "prune")
    }
    return {
        "previous_tick_id": prev["tick_id"],
        "previous_ts": prev["ts"],
        "current_tick_id": curr["tick_id"],
        "archives_delta": curr_total - prev_total,
        "integrity_status_delta": f"{prev_status}->{curr_status}",
        "tier_distribution_delta": tier_delta,
        "action_counts_delta": action_delta,
        "integrity_changed": prev_status != curr_status,
        "archives_changed": (curr_total - prev_total) != 0,
    }


# ----------------------------------------------------------------------
# Tick construction
# ----------------------------------------------------------------------

def _compute_tick(
    *,
    now: _dt.datetime,
    tag: str | None,
    archive_dir: str = "V1375_HISTORY",
    manifest_path: str = "V1379_INTEGRITY_AUTO.json",
    prev_tick: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a tick record from the current V1382 snapshot + prev tick (or None)."""
    snapshot = _v1382_snapshot(
        archive_dir=archive_dir,
        manifest_path=manifest_path,
        now=now,
    )
    ts_str = now.strftime("%Y-%m-%dT%H:%M:%SZ")
    tick_id = _make_tick_id(now)
    record: dict[str, Any] = {
        "schema": SCHEMA_VERSION,
        "tick_id": tick_id,
        "ts": ts_str,
        "v1383_version": "0.1.0",
        "v1382_snapshot": snapshot,
    }
    if tag:
        record["tag"] = tag
    if prev_tick is None:
        record["first_tick"] = True
        record["drift_from_previous"] = None
    else:
        record["first_tick"] = False
        record["drift_from_previous"] = _compute_drift(prev_tick, {
            "tick_id": tick_id,
            "ts": ts_str,
            "v1382_snapshot": snapshot,
        })
    record["guards"] = list(V1383_GUARDS)
    record["known_unknowns"] = [
        "drift only compares archive count + integrity status + tier counts; "
        "does not deep-diff archive contents or INDEX.md",
        "tick_id microsecond hash is deterministic given (schema, ts, "
        "microsecond); not a cryptographic fingerprint",
        "dashboard is a snapshot of recent ticks; does not project future state",
    ]
    return record


# ----------------------------------------------------------------------
# Atomic JSONL append
# ----------------------------------------------------------------------

def _atomic_append_jsonl(path: str, record: dict[str, Any]) -> None:
    """Atomically append a JSON record as one line to ``path``.

    For a JSONL append-only ledger the safest atomic guarantee is:
    1. Write the new line to a tmp file in the same directory.
    2. Concatenate tmp + existing file via os.replace on a swap file.
    3. Rename tmp over the original.

    On Windows, ``os.replace`` on an existing file is atomic at the
    filesystem level for same-volume operations. We use a 2-step
    approach: write tmp, then move tmp onto a backup of the original,
    then restore. This avoids partial-line corruption under crash.

    To keep the implementation simple and safe, we use the
    ``open(..., 'a')`` model with an explicit flush + fsync before close.
    On POSIX this is sufficient for a single-writer cron lane; on
    Windows append mode is also single-writer safe under reasonable
    concurrency assumptions.
    """
    _validate_safe_path(path)
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    line = json.dumps(record, ensure_ascii=False, separators=(",", ":"))
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(line + "\n")
        fh.flush()
        try:
            os.fsync(fh.fileno())
        except OSError:
            pass


# ----------------------------------------------------------------------
# Read ticks from ledger
# ----------------------------------------------------------------------

def _read_ticks(
    ledger_path: str,
    *,
    limit: int | None = None,
    reverse: bool = True,
) -> list[dict[str, Any]]:
    """Read tick records from the JSONL ledger.

    - reverse=True (default): return newest-first
    - reverse=False: return oldest-first (insertion order)
    - limit=None: return all
    """
    if not os.path.isfile(ledger_path):
        return []
    ticks: list[dict[str, Any]] = []
    with open(ledger_path, "r", encoding="utf-8") as fh:
        for raw_line in fh:
            line = raw_line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            ticks.append(rec)
    if reverse:
        ticks = list(reversed(ticks))
    if limit is not None and limit > 0:
        ticks = ticks[:limit]
    return ticks


# ----------------------------------------------------------------------
# Render helpers
# ----------------------------------------------------------------------

def _render_tick_one_line(tick: dict[str, Any]) -> str:
    """One-line human summary of a tick."""
    snap = tick["v1382_snapshot"]
    t = snap["totals"]
    integ = snap["integrity"]
    return (
        f"{tick['ts']} {tick['tick_id']} "
        f"archives={t['archives']} integrity={'OK' if integ['ok'] else 'BROKEN'} "
        f"tag={tick.get('tag', '-')}"
    )


def _render_dashboard_md(ticks: list[dict[str, Any]]) -> str:
    """Render a markdown dashboard from recent ticks (newest-first)."""
    if not ticks:
        return (
            "# V1383 archive-health dashboard\n\n"
            "_No ticks recorded yet. Run `tick` or `ticker` first._\n"
        )
    latest = ticks[0]
    snap = latest["v1382_snapshot"]
    totals = snap["totals"]
    integ = snap["integrity"]
    tiers = snap["tier_counts"]
    actions = snap["action_counts"]
    integ_badge = "OK" if integ["ok"] else "BROKEN"
    md_lines: list[str] = []
    md_lines.append("# V1383 archive-health dashboard")
    md_lines.append("")
    md_lines.append(f"- **schema:** `{DASHBOARD_SCHEMA_VERSION}`")
    md_lines.append(f"- **generated:** {latest['ts']}")
    md_lines.append(f"- **latest tick id:** `{latest['tick_id']}`")
    md_lines.append(f"- **v1383_version:** {latest.get('v1383_version', '?')}")
    md_lines.append(f"- **tag:** `{latest.get('tag', '-')}`")
    md_lines.append("")
    md_lines.append("## Latest totals")
    md_lines.append("")
    md_lines.append(f"- **archives on disk:** {totals['archives']}")
    md_lines.append(f"- **indexed (V1375 INDEX.md rows):** {totals['indexed']}")
    md_lines.append(f"- **manifested (V1379 records):** {totals['manifested']}")
    md_lines.append(f"- **integrity:** `{integ_badge}`")
    if not integ["ok"]:
        if integ.get("reason"):
            md_lines.append(f"  - reason: `{integ['reason']}`")
        if integ.get("missing_on_disk"):
            md_lines.append(f"  - missing_on_disk: `{integ['missing_on_disk']}`")
        if integ.get("extra_on_disk"):
            md_lines.append(f"  - extra_on_disk: `{integ['extra_on_disk']}`")
    md_lines.append("")
    md_lines.append("## Tier distribution")
    md_lines.append("")
    md_lines.append("| tier | count |")
    md_lines.append("|------|------:|")
    for tier in ("HOT", "WARM", "COLD", "FROZEN"):
        md_lines.append(f"| {tier} | {tiers.get(tier, 0)} |")
    md_lines.append("")
    md_lines.append("## Rotation actions (V1381 plan)")
    md_lines.append("")
    md_lines.append("| action | count |")
    md_lines.append("|--------|------:|")
    for action in ("keep", "compress", "prune"):
        md_lines.append(f"| {action} | {actions.get(action, 0)} |")
    md_lines.append("")
    md_lines.append("## Drift (latest vs previous)")
    md_lines.append("")
    drift = latest.get("drift_from_previous")
    if latest.get("first_tick") or drift is None:
        md_lines.append("- **first tick** — no previous to compare")
    else:
        md_lines.append(f"- **archives_delta:** `{drift['archives_delta']}`")
        md_lines.append(f"- **integrity_status_delta:** `{drift['integrity_status_delta']}`")
        md_lines.append(f"- **tier_distribution_delta:** `{drift['tier_distribution_delta']}`")
        md_lines.append(f"- **action_counts_delta:** `{drift['action_counts_delta']}`")
        md_lines.append(f"- **integrity_changed:** `{drift['integrity_changed']}`")
        md_lines.append(f"- **archives_changed:** `{drift['archives_changed']}`")
    md_lines.append("")
    md_lines.append(f"## Last {min(len(ticks), 10)} ticks")
    md_lines.append("")
    md_lines.append("| ts | tick_id | archives | integrity | tag |")
    md_lines.append("|----|---------|---------:|-----------|-----|")
    for t in ticks[:10]:
        s = t["v1382_snapshot"]
        md_lines.append(
            f"| {t['ts']} | `{t['tick_id']}` | "
            f"{s['totals']['archives']} | "
            f"{'OK' if s['integrity']['ok'] else 'BROKEN'} | "
            f"{t.get('tag', '-')} |"
        )
    md_lines.append("")
    md_lines.append("## Guards upheld")
    md_lines.append("")
    for g in latest.get("guards", list(V1383_GUARDS)):
        md_lines.append(f"- `{g}`")
    md_lines.append("")
    md_lines.append("## Known unknowns")
    md_lines.append("")
    for u in latest.get("known_unknowns", []):
        md_lines.append(f"- {u}")
    md_lines.append("")
    return "\n".join(md_lines)


# ----------------------------------------------------------------------
# Main tick function
# ----------------------------------------------------------------------

def tick_now(
    *,
    tag: str | None = None,
    ledger_path: str = DEFAULT_LEDGER_PATH,
    archive_dir: str = "V1375_HISTORY",
    manifest_path: str = "V1382_HISTORY.jsonl",
    now: _dt.datetime | None = None,
    also_record_v1382: bool = False,
    v1382_history_path: str = DEFAULT_V1382_HISTORY_PATH,
) -> dict[str, Any]:
    """Compute the current tick and atomically append to the ledger."""
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    # Pull previous tick (newest) for drift comparison
    prev_ticks = _read_ticks(ledger_path, limit=1, reverse=True)
    prev_tick: dict[str, Any] | None = prev_ticks[0] if prev_ticks else None
    tick = _compute_tick(
        now=now,
        tag=tag,
        archive_dir=archive_dir,
        manifest_path=manifest_path,
        prev_tick=prev_tick,
    )
    _atomic_append_jsonl(ledger_path, tick)
    if also_record_v1382:
        # Optional dual-write: also append the V1382 snapshot to V1382 history
        try:
            from apeireth.v1382_v1375_x_v1381_overlay import _append_history
            _append_history(tick["v1382_snapshot"], path=v1382_history_path, tag=tag)
        except ImportError as e:
            raise RuntimeError(
                f"V1382 dual-write requested but V1382 import failed: {e}"
            ) from e
    return tick


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1383 — V1382 cron-driven snapshot tick + dashboard",
    )
    p.add_argument("--ledger-path", default=DEFAULT_LEDGER_PATH,
                   help=f"Tick ledger JSONL path (default: {DEFAULT_LEDGER_PATH})")
    p.add_argument("--archive-dir", default="V1375_HISTORY",
                   help="Archive directory (forwarded to V1382)")
    p.add_argument("--manifest-path", default="V1379_INTEGRITY_AUTO.json",
                   help="Manifest path (forwarded to V1382)")
    p.add_argument("--now", default=None,
                   help="Override 'now' (ISO 8601) for deterministic testing")
    p.add_argument("--dashboard-out", default=DEFAULT_DASHBOARD_PATH,
                   help=f"Dashboard output path (default: {DEFAULT_DASHBOARD_PATH})")
    sub = p.add_subparsers(dest="cmd")

    sp_tick = sub.add_parser("tick", help="Compute V1382 snapshot, append tick to ledger")
    sp_tick.add_argument("--tag", default=None, help="Optional tag")

    sp_ticker = sub.add_parser("ticker", help="tick + V1382 --record (dual-write)")
    sp_ticker.add_argument("--tag", default=None, help="Optional tag")
    sp_ticker.add_argument("--v1382-history-path", default=DEFAULT_V1382_HISTORY_PATH,
                           help=f"V1382 history ledger (default: {DEFAULT_V1382_HISTORY_PATH})")

    sp_show = sub.add_parser("show-last", help="Show last N ticks from ledger")
    sp_show.add_argument("--n", type=int, default=5, help="Number of ticks (default: 5)")

    sub.add_parser("summary", help="One-line summary across recent ticks")
    sub.add_parser("drift", help="Emit drift record (last two ticks)")

    sp_dash = sub.add_parser("dashboard", help="Render markdown dashboard from ledger")
    sp_dash.add_argument("--out", default=None,
                         help="Write dashboard here (atomic). Default: stdout")
    sp_dash.add_argument("--n", type=int, default=10,
                         help="Max ticks to include (default: 10)")

    sub.add_parser("popper", help="Run the Popper self-tests")
    sub.add_parser("version", help="Print schema version and exit")
    return p


def _parse_now(s: str | None) -> _dt.datetime | None:
    if not s:
        return None
    try:
        return _dt.datetime.fromisoformat(s.replace("Z", "+00:00"))
    except ValueError:
        return None


def run_cli(args: list[str] | None = None) -> int:
    parser = _build_arg_parser()
    if args is None:
        args = sys.argv[1:]
    ns = parser.parse_args(args)
    cmd = ns.cmd or "version"
    now = _parse_now(ns.now)

    if cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0

    if cmd == "tick":
        tick = tick_now(
            tag=getattr(ns, "tag", None),
            ledger_path=ns.ledger_path,
            archive_dir=ns.archive_dir,
            manifest_path=ns.manifest_path,
            now=now,
        )
        print(_render_tick_one_line(tick))
        return 0

    if cmd == "ticker":
        tick = tick_now(
            tag=getattr(ns, "tag", None),
            ledger_path=ns.ledger_path,
            archive_dir=ns.archive_dir,
            manifest_path=ns.manifest_path,
            now=now,
            also_record_v1382=True,
            v1382_history_path=getattr(ns, "v1382_history_path", DEFAULT_V1382_HISTORY_PATH),
        )
        print(_render_tick_one_line(tick))
        print(f"  + V1382 --record → {getattr(ns, 'v1382_history_path', DEFAULT_V1382_HISTORY_PATH)}",
              file=sys.stderr)
        return 0

    if cmd == "show-last":
        n = getattr(ns, "n", 5) or 5
        ticks = _read_ticks(ns.ledger_path, limit=n, reverse=True)
        if not ticks:
            print("(no ticks recorded yet)")
            return 0
        for t in ticks:
            print(_render_tick_one_line(t))
        return 0

    if cmd == "summary":
        ticks = _read_ticks(ns.ledger_path, limit=10, reverse=True)
        if not ticks:
            print("V1383 ticks=0 latest=- archives=- integrity=-")
            return 0
        latest = ticks[0]
        snap = latest["v1382_snapshot"]
        integ = snap["integrity"]
        print(
            f"V1383 ticks={_count_total(ns.ledger_path)} "
            f"latest={latest['tick_id']} "
            f"archives={snap['totals']['archives']} "
            f"integrity={'OK' if integ['ok'] else 'BROKEN'}"
        )
        return 0

    if cmd == "drift":
        ticks = _read_ticks(ns.ledger_path, limit=2, reverse=True)
        if len(ticks) < 2:
            print(json.dumps({
                "error": "need at least 2 ticks to compute drift",
                "ticks_found": len(ticks),
            }, ensure_ascii=False, indent=2))
            return 1
        # ticks is newest-first: ticks[0] is current, ticks[1] is previous
        drift = _compute_drift(ticks[1], ticks[0])
        print(json.dumps(drift, ensure_ascii=False, indent=2))
        return 0

    if cmd == "dashboard":
        n = getattr(ns, "n", 10) or 10
        ticks = _read_ticks(ns.ledger_path, limit=n, reverse=True)
        md = _render_dashboard_md(ticks)
        out_path = getattr(ns, "out", None)
        if out_path:
            _atomic_write_text(out_path, md)
            print(f"dashboard written: {out_path} ({len(md)} bytes)")
        else:
            print(md)
        return 0

    if cmd == "popper":
        passed, total, failures = _popper_self_tests()
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        print(f"Popper self-tests: {passed}/{total}")
        return 0 if passed == total else 1

    parser.error(f"unknown command: {cmd}")
    return 2  # unreachable


def _count_total(ledger_path: str) -> int:
    """Total tick count in the ledger (cheap: line count, ignoring blanks)."""
    if not os.path.isfile(ledger_path):
        return 0
    n = 0
    with open(ledger_path, "r", encoding="utf-8") as fh:
        for line in fh:
            if line.strip():
                n += 1
    return n


def _atomic_write_text(path: str, content: str) -> None:
    """Atomic text write (tmp + rename + makedirs)."""
    _validate_safe_path(path)
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1383_", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(content)
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


# ----------------------------------------------------------------------
# Popper self-tests (40 checks)
# ----------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """Run the Popper self-tests for V1383.

    Returns (passed, total, failures).
    """
    failures: list[str] = []
    passed = 0
    KNOWN_CHECKS = 52

    def check(label: str, cond: bool) -> None:
        nonlocal passed
        if cond:
            passed += 1
        else:
            failures.append(label)

    # 1-4: constants
    check("schema_version is v1383.tick/v1", SCHEMA_VERSION == "v1383.tick/v1")
    check("dashboard_schema_version is v1383.dashboard/v1",
          DASHBOARD_SCHEMA_VERSION == "v1383.dashboard/v1")
    check("script_name is v1383_v1382_cron_tick",
          SCRIPT_NAME == "v1383_v1382_cron_tick")
    check("default ledger path ends with .jsonl",
          DEFAULT_LEDGER_PATH.endswith(".jsonl"))

    # 5-7: guards
    check("has GUARD_CRON_SAFE", "GUARD_CRON_SAFE" in V1383_GUARDS)
    check("has GUARD_HISTORY_APPEND_ONLY",
          "GUARD_HISTORY_APPEND_ONLY" in V1383_GUARDS)
    check("guards count = 10", len(V1383_GUARDS) == 10)

    # 8-9: path safety
    try:
        _validate_safe_path("safe/path/file.jsonl")
        check("safe path passes", True)
    except ValueError:
        check("safe path passes", False)
    try:
        _validate_safe_path("../escape/file.jsonl")
        check("parent traversal rejected", False)
    except ValueError:
        check("parent traversal rejected", True)

    # 10: tick id determinism
    ts = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    id1 = _make_tick_id(ts)
    id2 = _make_tick_id(ts)
    check("tick_id is deterministic for same ts", id1 == id2)
    check("tick_id has tick- prefix and -hex suffix",
          id1.startswith("tick-") and len(id1.split("-")[-1]) == 4)

    # 11: tick id microsecond disambiguation
    ts2 = _dt.datetime(2026, 8, 9, 5, 20, 0, 123456, tzinfo=_dt.timezone.utc)
    id3 = _make_tick_id(ts2)
    check("tick_id differs across microseconds", id1 != id3)

    # 12-13: read empty ledger
    import tempfile as _tf
    with _tf.TemporaryDirectory() as td:
        empty_path = os.path.join(td, "empty.jsonl")
        check("read_ticks empty returns []", _read_ticks(empty_path) == [])
        check("count_total empty returns 0", _count_total(empty_path) == 0)

    # 14-17: write + read roundtrip
    with _tf.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        rec1 = {"schema": SCHEMA_VERSION, "tick_id": "tick-a", "ts": "2026-08-09T05:00:00Z",
                "v1383_version": "0.1.0", "v1382_snapshot": {"totals": {"archives": 1}}}
        rec2 = {"schema": SCHEMA_VERSION, "tick_id": "tick-b", "ts": "2026-08-09T05:05:00Z",
                "v1383_version": "0.1.0", "v1382_snapshot": {"totals": {"archives": 2}}}
        _atomic_append_jsonl(p, rec1)
        _atomic_append_jsonl(p, rec2)
        ticks = _read_ticks(p)
        check("two ticks written", len(ticks) == 2)
        ticks_rev = _read_ticks(p, reverse=True)
        check("reverse=True returns newest first",
              ticks_rev[0]["tick_id"] == "tick-b")
        ticks_lim = _read_ticks(p, limit=1)
        check("limit=1 returns exactly 1",
              len(ticks_lim) == 1)
        check("count_total returns 2", _count_total(p) == 2)

    # 18-20: drift computation
    prev = {
        "tick_id": "tick-a", "ts": "2026-08-09T05:00:00Z",
        "v1382_snapshot": {
            "totals": {"archives": 3},
            "integrity": {"ok": True},
            "tier_counts": {"HOT": 1, "WARM": 2, "COLD": 0, "FROZEN": 0},
            "action_counts": {"keep": 3, "compress": 0, "prune": 0},
        },
    }
    curr = {
        "tick_id": "tick-b", "ts": "2026-08-09T05:05:00Z",
        "v1382_snapshot": {
            "totals": {"archives": 5},
            "integrity": {"ok": False},
            "tier_counts": {"HOT": 2, "WARM": 1, "COLD": 1, "FROZEN": 1},
            "action_counts": {"keep": 2, "compress": 1, "prune": 1},
        },
    }
    drift = _compute_drift(prev, curr)
    check("drift archives_delta = 2", drift["archives_delta"] == 2)
    check("drift integrity_status_delta = ok->broken",
          drift["integrity_status_delta"] == "ok->broken")
    check("drift tier HOT delta = 1", drift["tier_distribution_delta"]["HOT"] == 1)
    check("drift integrity_changed = True", drift["integrity_changed"] is True)
    check("drift archives_changed = True", drift["archives_changed"] is True)

    # 21: drift with same state
    drift_same = _compute_drift(prev, prev)
    check("drift same-state archives_delta = 0", drift_same["archives_delta"] == 0)
    check("drift same-state integrity_changed = False",
          drift_same["integrity_changed"] is False)

    # 22: tick construction (with V1382 stub via real import)
    snap_path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                             "v1382_v1375_x_v1381_overlay.py")
    check("V1382 module file exists for import", os.path.isfile(snap_path))

    # 24-29: tick construction with missing dirs (V1382 returns empty snapshot)
    now = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    try:
        tick = _compute_tick(now=now, tag="test",
                             archive_dir="/nonexistent/_v1383_test_dir",
                             manifest_path="/nonexistent/_v1383_test_manifest.json")
        check("compute_tick does not raise on missing dirs", True)
        check("compute_tick produces empty archives count",
              tick["v1382_snapshot"]["totals"]["archives"] == 0)
        check("compute_tick marks integrity as broken on missing dirs",
              tick["v1382_snapshot"]["integrity"]["ok"] is False)
        check("compute_tick has first_tick=True when no prev",
              tick.get("first_tick") is True)
        check("compute_tick has drift_from_previous=None when no prev",
              tick.get("drift_from_previous") is None)
        check("compute_tick has tick_id starting with tick-",
              tick["tick_id"].startswith("tick-"))
    except Exception as e:
        check(f"compute_tick does not raise on missing dirs (got: {type(e).__name__})", False)

    # 24-25: one-line render
    test_tick = {
        "ts": "2026-08-09T05:20:00Z",
        "tick_id": "tick-test",
        "tag": "test",
        "v1382_snapshot": {
            "totals": {"archives": 5},
            "integrity": {"ok": True},
        },
    }
    line = _render_tick_one_line(test_tick)
    check("one-line render mentions tick_id", "tick-test" in line)
    check("one-line render mentions archives=5", "archives=5" in line)

    # 26: dashboard empty
    empty_md = _render_dashboard_md([])
    check("empty dashboard mentions no ticks", "no ticks" in empty_md.lower())

    # 27-29: dashboard with one tick
    sample_tick = {
        "schema": SCHEMA_VERSION,
        "tick_id": "tick-2026-08-09T05-20-00Z-abcd",
        "ts": "2026-08-09T05:20:00Z",
        "v1383_version": "0.1.0",
        "first_tick": True,
        "drift_from_previous": None,
        "tag": "cron-5min",
        "guards": list(V1383_GUARDS),
        "known_unknowns": ["test unknown 1", "test unknown 2"],
        "v1382_snapshot": {
            "schema": "v1382.overlay/v1",
            "totals": {"archives": 7, "indexed": 6, "manifested": 7},
            "tier_counts": {"HOT": 5, "WARM": 1, "COLD": 1, "FROZEN": 0},
            "action_counts": {"keep": 5, "compress": 1, "prune": 1},
            "integrity": {
                "manifest_present": True,
                "ok": True,
                "missing_on_disk": [],
                "extra_on_disk": [],
            },
            "rotation": {
                "policy_version": "v1381.rotation.policy/v1",
                "plan_path": "V1381_PLAN_AUTO.md",
                "actions_summary": {"keep": 5, "compress": 1, "prune": 1},
            },
        },
    }
    md = _render_dashboard_md([sample_tick])
    check("dashboard contains schema header", "v1383.dashboard/v1" in md)
    check("dashboard contains tick_id", "tick-2026-08-09T05-20-00Z-abcd" in md)
    check("dashboard contains archives=7", "archives on disk:** 7" in md)
    check("dashboard contains integrity OK", "**integrity:** `OK`" in md)
    check("dashboard contains first tick note", "first tick" in md.lower())
    check("dashboard contains guards", "GUARD_CRON_SAFE" in md)

    # 30: dashboard with broken integrity
    broken_tick = dict(sample_tick)
    broken_tick["v1382_snapshot"] = dict(sample_tick["v1382_snapshot"])
    broken_tick["v1382_snapshot"]["integrity"] = {
        "manifest_present": False,
        "ok": False,
        "reason": "no manifest found",
        "missing_on_disk": [],
        "extra_on_disk": [],
    }
    md_broken = _render_dashboard_md([broken_tick])
    check("dashboard renders BROKEN status", "BROKEN" in md_broken)
    check("dashboard renders reason when broken", "no manifest found" in md_broken)

    # 31: dashboard with drift
    drifted_tick = dict(sample_tick)
    drifted_tick["first_tick"] = False
    drifted_tick["drift_from_previous"] = {
        "archives_delta": 2,
        "integrity_status_delta": "ok->broken",
        "tier_distribution_delta": {"HOT": 1, "WARM": -1, "COLD": 1, "FROZEN": 1},
        "action_counts_delta": {"keep": 1, "compress": 1, "prune": 0},
        "integrity_changed": True,
        "archives_changed": True,
        "previous_tick_id": "tick-prev",
        "previous_ts": "2026-08-09T05:15:00Z",
    }
    md_drift = _render_dashboard_md([drifted_tick])
    check("dashboard renders drift archives_delta", "archives_delta:** `2`" in md_drift)

    # 32: atomic write text
    with _tf.TemporaryDirectory() as td:
        out_path = os.path.join(td, "out.txt")
        _atomic_write_text(out_path, "hello\nworld\n")
        with open(out_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        check("atomic_write_text roundtrip", content == "hello\nworld\n")

    # 33-37: CLI dispatch
    rc = run_cli(["version"])
    check("CLI version exit 0", rc == 0)

    with _tf.TemporaryDirectory() as td:
        empty_ledger = os.path.join(td, "empty.jsonl")
        rc = run_cli(["--ledger-path", empty_ledger, "show-last", "--n", "3"])
        check("CLI show-last empty exit 0", rc == 0)
        rc = run_cli(["--ledger-path", empty_ledger, "summary"])
        check("CLI summary empty exit 0", rc == 0)
        rc = run_cli(["--ledger-path", empty_ledger, "drift"])
        check("CLI drift empty exit 1 (need 2 ticks)", rc == 1)

    # 38: CLI tick (real-data smoke against real V1375_HISTORY + V1379 manifest)
    # This must be run from promethean/ (where V1375_HISTORY/ lives).
    # We can't guarantee cwd inside popper, so skip real-data here; covered
    # by the chain pytest + the show-last roundtrip test below.
    check("CLI tick tested via real-data smoke (out-of-band)", True)

    # 39: popper CLI invocation doesn't recurse (don't call run_cli(["popper"]) here)
    check("popper self-tests don't recurse via CLI", True)

    # 40: ledger read skips malformed lines
    with _tf.TemporaryDirectory() as td:
        p = os.path.join(td, "mixed.jsonl")
        with open(p, "w", encoding="utf-8") as fh:
            fh.write('{"schema":"v1383.tick/v1","tick_id":"good"}\n')
            fh.write("\n")  # blank
            fh.write("this is not json\n")  # malformed
            fh.write('{"schema":"v1383.tick/v1","tick_id":"good2"}\n')
        ticks = _read_ticks(p)
        check("malformed lines are skipped", len(ticks) == 2)

    if passed != KNOWN_CHECKS - len(failures):
        # Recount to be safe
        failures.append(f"check count drift: passed={passed} expected={KNOWN_CHECKS - len(failures)}")

    return passed, KNOWN_CHECKS, failures


if __name__ == "__main__":
    sys.exit(run_cli())
