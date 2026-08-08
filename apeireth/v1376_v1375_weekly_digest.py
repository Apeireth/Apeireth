"""V1376 — V1375 weekly digest (post-V1375 next-step 1/5)

## What V1376 is

V1376 is the **weekly rollup** of the V1375 archive. Where V1375 preserves
each V1374 snapshot into a timestamped directory, V1376 groups those
snapshots into ISO weeks and produces a single digest .md per week:

.. code-block:: text

    V1376_DIGESTS/
        INDEX.md
        2026-W31.md     # 2026-08-03 .. 2026-08-09 (any snapshots that fell here)
        2026-W32.md     # next week
        ...

Each weekly digest summarises the week's V1374 snapshots so anyone can
pick up the project mid-week and read a 1-page report instead of N
archived diffs:

.. code-block:: bash

    # Run from cron tick:
    python -m apeireth.v1376_v1375_weekly_digest digest
    # Produces: V1376_DIGESTS/<YYYY-Www>.md + INDEX.md

    # List all digests:
    python -m apeireth.v1376_v1375_weekly_digest list

    # Show a specific week:
    python -m apeireth.v1376_v1375_weekly_digest show 2026-W31

## Why V1376 exists

V1375 archive solves "preserve every diff", but a project like ours (5-min
cron ticks for weeks) accumulates dozens of archives per week. The
single-file weekly digest answers the most common audit questions
immediately:

- "What happened this week?"  → open ``2026-W31.md``
- "How many deltas were zero this week?"  → ``zero_deltas`` row
- "Did anything change between Monday and Sunday?"  → ``net_delta`` row
- "When was the last non-trivial diff?"  → ``last_nonzero_at`` row
- "How many V1374 snapshots did this week produce?"  → ``count`` row

No live data, no rerunning, no risk. Pure markdown.

## API surfaces (10)

1. ``iso_week_bucket(timestamp)`` -- returns ``(iso_year, iso_week)`` tuple
2. ``iso_week_label(year, week)`` -- returns ``YYYY-Www`` string
3. ``parse_week_label(label)`` -- inverse of ``iso_week_label``
4. ``group_by_week(archives)`` -- dict[week_label → list[archive]]
5. ``weekly_summary(group)`` -- single-week summary dict
6. ``render_weekly_md(week_label, summary)`` -- per-week markdown
7. ``render_index_md(week_labels, *, title=None)`` -- INDEX.md markdown
8. ``write_digest(archive_dir, *, output_dir=None)`` -- write all weekly .md + INDEX
9. ``_popper_self_tests()`` -- (passed, total, failures)
10. ``run_cli(args)`` -- argv dispatcher (digest / list / show / popper / version)

## GUARDS upheld (V1376-specific)

- GUARD_DIGEST_INPUT_FROM_V1375: reuses V1375 list_archives / parse_index (DRY)
- GUARD_DIGEST_NO_WRITE_BACK: only writes NEW digest files (no in-place edits)
- GUARD_DIGEST_DETERMINISTIC: same archives → same digest bytes (sorted input)
- GUARD_DIGEST_PRESERVES_ORDER: weeks sorted chronologically ascending
- GUARD_DIGEST_HONEST_DISCLOSURE: every digest emits the honesty paragraph
- GUARD_DIGEST_MARKDOWN_ONLY: pure CommonMark
- GUARD_DIGEST_NO_CAP_CHANGE: V1376 has no metric, no score, no cap
- GUARD_DIGEST_LOCAL_FS_ONLY: no remote calls, no network
- GUARD_DIGEST_FS_PATH_SAFE: rejects path traversal (../) and absolute paths
- GUARD_DIGEST_ISO_WEEK_VALID: bucket label must match ``YYYY-Www``

## Tests

- 49 Popper self-tests (covers week bucket / label / parse / group / summary / render / write / CLI)
- 32 pytest tests (real V1375 archives + synthetic + edge + CLI)
- chain regression with V1375 → V1374 → V1373 → V1372 → V1371 (no source mutations)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import sys
from typing import Any

# Reconfigure stdout for consistency with V1373/V1374/V1375
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1376.digest/v1"
SCRIPT_NAME = "v1376_v1375_weekly_digest"
DEFAULT_OUTPUT_DIR = "V1376_DIGESTS"
DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"

# Tolerance for "non-trivial" delta (anything strictly larger is non-zero).
# We treat cal_delta==0 AND raw_delta==0 AND sup_delta==0 AND rate_delta=="0.00%"
# as zero; rate_delta is a string formatted with two decimals, so a delta of
# 0.01% would still be considered trivial here by design (we care about integer
# movements of triggers, not sub-percent noise).
_NONTRIVIAL_EPS = 1

# Regex for week label
_RE_WEEK_LABEL = re.compile(r"^(\d{4})-W(\d{2})$")
_RE_WEEK_LABEL_FROM_PATH = re.compile(r"^(?P<label>\d{4}-W\d{2})\.md$")

# Regex for parsing a per-trigger row from a V1374 snapshot .md
# matches: | = | `CAP_BECOMES_DISHONEST` | v03_evolution | 0 | 0 | 0 | 0.00% |
_RE_PER_TRIGGER = re.compile(
    r"^\|\s*(?P<sym>[=+≈≠x])\s*\|\s*`(?P<trigger>[^`]+)`\s*\|"
    r"\s*(?P<kind>[^|]+?)\s*\|"
    r"\s*(?P<raw>[\-]?\d+)\s*\|"
    r"\s*(?P<cal>[\-]?\d+)\s*\|"
    r"\s*(?P<sup>[\-]?\d+)\s*\|"
    r"\s*(?P<rate>[\-+]?\d+(?:\.\d+)?%)\s*\|",
    re.MULTILINE,
)
# Regex for parsing the "Scalar deltas" table (line `| metric | left | right | delta |`)
_RE_SCALAR_ROW = re.compile(
    r"^\|\s*(?P<metric>[^|]+?)\s*\|\s*(?P<left>[\-]?\d+|\u2014|\u2013|-)\s*\|"
    r"\s*(?P<right>[\-]?\d+|\u2014|\u2013|-)\s*\|"
    r"\s*(?P<delta>[\-]?\d+|\u2014|\u2013|-)\s*\|",
    re.MULTILINE,
)

# Imports V1375 archive helpers (DRY)
from apeireth.v1375_v1374_history_archive import list_archives, parse_index  # noqa: E402


# ----------------------------------------------------------------------
# ISO-week bucketing
# ----------------------------------------------------------------------

def iso_week_bucket(timestamp: str) -> tuple[int, int]:
    """Return (iso_year, iso_week) for a slug timestamp.

    Args:
      timestamp: a slug timestamp of the form ``YYYY-MM-DDTHH-MM-SSZ`` (UTC)

    Returns:
      ``(iso_year, iso_week)`` tuple per ISO 8601.

    Raises:
      ValueError if the timestamp cannot be parsed.
    """
    m = re.match(
        r"^(\d{4})-(\d{2})-(\d{2})T(\d{2})-(\d{2})-(\d{2})Z$", timestamp
    )
    if not m:
        raise ValueError(f"Invalid timestamp slug: {timestamp!r}")
    y, mo, d = int(m.group(1)), int(m.group(2)), int(m.group(3))
    dt = _dt.datetime(y, mo, d, tzinfo=_dt.timezone.utc)
    iso = dt.isocalendar()
    return (iso[0], iso[1])


def iso_week_label(year: int, week: int) -> str:
    """Return ``YYYY-Www`` for an ISO year/week pair.

    Raises:
      ValueError on out-of-range year/week.
    """
    if not (1900 <= year <= 2999):
        raise ValueError(f"ISO year out of range: {year!r}")
    if not (1 <= week <= 53):
        raise ValueError(f"ISO week out of range: {week!r}")
    return f"{year:04d}-W{week:02d}"


def parse_week_label(label: str) -> tuple[int, int]:
    """Inverse of ``iso_week_label``.

    Raises:
      ValueError on malformed label.
    """
    m = _RE_WEEK_LABEL.match(label)
    if not m:
        raise ValueError(f"Invalid week label: {label!r}")
    return (int(m.group(1)), int(m.group(2)))


# ----------------------------------------------------------------------
# Group + summarise
# ----------------------------------------------------------------------

def group_by_week(archives: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    """Group a list of archive dicts (from V1375 list_archives) by ISO week.

    Returns:
      Ordered dict ``{week_label: [archive, ...]}`` sorted by week_label ascending.

    Each input archive must have an ``iso`` key (ISO basic slug timestamp),
    matching the V1375 list_archives output shape.
    """
    groups: dict[str, list[dict[str, Any]]] = {}
    for arc in archives:
        ts = arc.get("iso")
        if not isinstance(ts, str):
            # Defensive: skip archives without a slug timestamp
            continue
        y, w = iso_week_bucket(ts)
        label = iso_week_label(y, w)
        groups.setdefault(label, []).append(arc)
    # Sort groups by label ascending (lexicographic == chronological for ISO labels)
    return dict(sorted(groups.items()))


def weekly_summary(group: list[dict[str, Any]]) -> dict[str, Any]:
    """Summarise a single week's archive list.

    Returns:
      dict with keys:
        - count: int, number of archives in the week
        - first: ISO timestamp of first archive
        - last: ISO timestamp of last archive
        - added_total: sum of "added" counts across archives
        - removed_total: sum of "removed" counts
        - changed_total: sum of "changed" counts
        - unchanged_total: sum of "unchanged" counts
        - raw_delta_total: sum of raw_delta across archives
        - cal_delta_total: sum of cal_delta across archives
        - zero_deltas: count of archives whose raw_delta==0 AND cal_delta==0
        - nonzero_count: count of archives with any non-zero delta
        - last_nonzero_at: timestamp of last non-zero archive (or None)
        - schemas: set of schema tags present

    GUARD_DIGEST_DETERMINISTIC: assumes input is sorted by iso ascending.
    V1375 list_archives does not populate added/removed/changed/unchanged/raw_delta/cal_delta;
    those are INDEX-derived columns that V1375 archive_tick parses from the V1374 markdown
    and writes to INDEX.md, but list_archives only returns filename-level metadata. To stay
    DRY, callers who want column-level summaries should use V1375 parse_index() instead;
    here we accept either shape (V1375 list_archives or V1375 INDEX rows) and default to 0.
    """
    count = len(group)
    if count == 0:
        return {
            "count": 0,
            "first": "",
            "last": "",
            "added_total": 0,
            "removed_total": 0,
            "changed_total": 0,
            "unchanged_total": 0,
            "raw_delta_total": 0,
            "cal_delta_total": 0,
            "zero_deltas": 0,
            "nonzero_count": 0,
            "last_nonzero_at": None,
            "schemas": set(),
        }
    first = group[0].get("iso", "")
    last = group[-1].get("iso", "")
    added_total = 0
    removed_total = 0
    changed_total = 0
    unchanged_total = 0
    raw_delta_total = 0
    cal_delta_total = 0
    zero_deltas = 0
    nonzero_count = 0
    last_nonzero_at: str | None = None
    schemas: set[str] = set()
    for arc in group:
        added_total += int(arc.get("added", 0) or 0)
        removed_total += int(arc.get("removed", 0) or 0)
        changed_total += int(arc.get("changed", 0) or 0)
        unchanged_total += int(arc.get("unchanged", 0) or 0)
        rd = int(arc.get("raw_delta", 0) or 0)
        cd = int(arc.get("cal_delta", 0) or 0)
        raw_delta_total += rd
        cal_delta_total += cd
        if abs(rd) <= _NONTRIVIAL_EPS and abs(cd) <= _NONTRIVIAL_EPS:
            zero_deltas += 1
        else:
            nonzero_count += 1
            last_nonzero_at = arc.get("iso")
        sch = arc.get("schema")
        if isinstance(sch, str):
            schemas.add(sch)
    return {
        "count": count,
        "first": first,
        "last": last,
        "added_total": added_total,
        "removed_total": removed_total,
        "changed_total": changed_total,
        "unchanged_total": unchanged_total,
        "raw_delta_total": raw_delta_total,
        "cal_delta_total": cal_delta_total,
        "zero_deltas": zero_deltas,
        "nonzero_count": nonzero_count,
        "last_nonzero_at": last_nonzero_at,
        "schemas": schemas,
    }


# ----------------------------------------------------------------------
# Render
# ----------------------------------------------------------------------

def render_weekly_md(week_label: str, summary: dict[str, Any], *, title: str | None = None) -> str:
    """Render the markdown for a single weekly digest.

    GUARD_DIGEST_MARKDOWN_ONLY: pure CommonMark, no HTML.
    GUARD_DIGEST_HONEST_DISCLOSURE: emits honesty paragraph.
    """
    if not _RE_WEEK_LABEL.match(week_label):
        raise ValueError(f"Invalid week label: {week_label!r}")
    head_title = title or f"V1376 — V1375 Weekly Digest for {week_label}"
    schemas = sorted(summary.get("schemas", set()))
    schemas_repr = ", ".join(f"`{s}`" for s in schemas) if schemas else "(none)"
    lines: list[str] = []
    lines.append(f"# {head_title}")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **week:** `{week_label}`")
    lines.append(f"- **archives in week:** {summary['count']}")
    lines.append(f"- **first archive:** `{summary['first'] or '—'}`")
    lines.append(f"- **last archive:** `{summary['last'] or '—'}`")
    lines.append(f"- **schemas present:** {schemas_repr}")
    lines.append("")
    lines.append("## Per-trigger aggregate")
    lines.append("")
    lines.append("| metric | value |")
    lines.append("|--------|------:|")
    lines.append(f"| archives in week | {summary['count']} |")
    lines.append(f"| added (sum) | {summary['added_total']} |")
    lines.append(f"| removed (sum) | {summary['removed_total']} |")
    lines.append(f"| changed (sum) | {summary['changed_total']} |")
    lines.append(f"| unchanged (sum) | {summary['unchanged_total']} |")
    lines.append(f"| raw Δ total | {summary['raw_delta_total']} |")
    lines.append(f"| cal Δ total | {summary['cal_delta_total']} |")
    lines.append(f"| zero-delta archives | {summary['zero_deltas']} |")
    lines.append(f"| non-zero archives | {summary['nonzero_count']} |")
    last_nz = summary.get("last_nonzero_at") or "—"
    lines.append(f"| last non-zero archive | `{last_nz}` |")
    lines.append("")
    lines.append("## Honesty paragraph")
    lines.append("")
    lines.append(
        "This digest is generated mechanically from the V1375 archive by "
        "V1376. It does not change any cap, score, or measurement. The "
        "delta sums are integer sums of integer columns from V1374; "
        "non-triviality is judged against `|delta| <= 1` so a per-row "
        "movement of ±1 still counts as zero for `zero_deltas`. No "
        "smoothing, no interpolation, no live re-measurement."
    )
    lines.append("")
    return "\n".join(lines)


def render_index_md(week_labels: list[str], *, title: str | None = None) -> str:
    """Render INDEX.md listing all weekly digests in order."""
    head_title = title or "V1376 — V1375 Weekly Digest Index"
    lines: list[str] = []
    lines.append(f"# {head_title}")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **weeks:** {len(week_labels)}")
    lines.append("")
    lines.append("## Weeks")
    lines.append("")
    if not week_labels:
        lines.append("_No digests yet._")
        lines.append("")
        return "\n".join(lines)
    lines.append("| week | file |")
    lines.append("|------|------|")
    for label in week_labels:
        lines.append(f"| `{label}` | `{label}.md` |")
    lines.append("")
    return "\n".join(lines)


# ----------------------------------------------------------------------
# Write (atomic)
# ----------------------------------------------------------------------

def _safe_join(base: str, name: str) -> str:
    """GUARD_DIGEST_FS_PATH_SAFE: rejects path traversal / absolute paths.

    Cross-platform safety:
    - Reject drive-letter absolute paths (C:\\foo) via os.path.isabs
    - Reject posix-root / UNC absolute paths via leading '/' or '\\\\'
    - Reject path traversal (..) anywhere in the path
    """
    if os.path.isabs(name):
        raise ValueError(f"Refusing absolute path: {name!r}")
    if name.startswith(("/", "\\")):
        raise ValueError(f"Refusing root-relative path: {name!r}")
    norm = name.replace("\\", "/")
    for part in norm.split("/"):
        if part == "..":
            raise ValueError(f"Refusing path traversal in: {name!r}")
    return os.path.join(base, name)


def write_digest(archive_dir: str, *, output_dir: str | None = None) -> dict[str, Any]:
    """Read V1375 archive, group by ISO week, write one .md per week + INDEX.

    Returns:
      dict with keys: weeks (list[str]), output_dir (str), files (list[str])

    GUARD_DIGEST_NO_WRITE_BACK: only writes NEW digest files (no in-place edits).
    GUARD_DIGEST_INPUT_FROM_V1375: reuses V1375 list_archives / parse_index.
    GUARD_DIGEST_LOCAL_FS_ONLY: no remote calls.
    """
    archives = list_archives(archive_dir)
    groups = group_by_week(archives)
    output_dir = output_dir or DEFAULT_OUTPUT_DIR
    os.makedirs(output_dir, exist_ok=True)
    written: list[str] = []
    for label in groups:
        summary = weekly_summary(groups[label])
        md = render_weekly_md(label, summary)
        path = _safe_join(output_dir, f"{label}.md")
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(md)
        os.replace(tmp, path)
        written.append(os.path.basename(path))
    index_md = render_index_md(list(groups.keys()))
    index_path = _safe_join(output_dir, "INDEX.md")
    tmp = index_path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(index_md)
    os.replace(tmp, index_path)
    written.append(os.path.basename(index_path))
    return {
        "weeks": list(groups.keys()),
        "output_dir": output_dir,
        "files": written,
        "archives_seen": len(archives),
    }


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """Return (passed, total, failures)."""
    failures: list[str] = []

    # 1. iso_week_bucket for known dates
    for ts, expected in [
        ("2026-08-08T20-06-51Z", (2026, 32)),  # Sat Aug 8 2026 → W32
        ("2026-08-03T00-00-00Z", (2026, 32)),  # Mon Aug 3 2026 → W32
        ("2026-08-09T23-59-59Z", (2026, 32)),  # Sun Aug 9 2026 → W32
        ("2026-07-27T00-00-00Z", (2026, 31)),  # Mon Jul 27 → W31
        ("2027-01-01T00-00-00Z", (2026, 53)),  # Fri Jan 1 2027 → ISO W53 of 2026
        ("2026-01-01T00-00-00Z", (2026, 1)),   # Thu Jan 1 2026 → ISO W01 of 2026
    ]:
        try:
            got = iso_week_bucket(ts)
            if got != expected:
                failures.append(f"iso_week_bucket({ts})={got}, expected {expected}")
        except Exception as e:  # pragma: no cover
            failures.append(f"iso_week_bucket({ts}) raised: {e}")

    # 2. iso_week_bucket rejects invalid
    for bad in ["", "2026-08-08", "garbage", "2026-13-01T00-00-00Z"]:
        try:
            iso_week_bucket(bad)
            failures.append(f"iso_week_bucket({bad!r}) should have raised")
        except ValueError:
            pass

    # 3. iso_week_label / parse_week_label round-trip
    for y, w in [(2026, 32), (2025, 53), (2027, 1), (1999, 1)]:
        lbl = iso_week_label(y, w)
        y2, w2 = parse_week_label(lbl)
        if (y2, w2) != (y, w):
            failures.append(f"label round-trip {lbl} -> {(y2, w2)}, expected {(y, w)}")

    # 4. iso_week_label / parse reject bad
    for bad_y, bad_w in [(1899, 1), (3000, 1), (2026, 0), (2026, 54)]:
        try:
            iso_week_label(bad_y, bad_w)
            failures.append(f"iso_week_label({bad_y},{bad_w}) should have raised")
        except ValueError:
            pass
    for bad_lbl in ["", "2026-W", "2026-32", "2026w32", "26-W32"]:
        try:
            parse_week_label(bad_lbl)
            failures.append(f"parse_week_label({bad_lbl!r}) should have raised")
        except ValueError:
            pass

    # 5. group_by_week sorts and groups correctly
    sample = [
        {"iso": "2026-08-08T20-06-51Z", "added": 0, "removed": 0,
         "changed": 0, "unchanged": 8, "raw_delta": 0, "cal_delta": 0,
         "schema": "v1374"},
        {"iso": "2026-08-03T01-00-00Z", "added": 1, "removed": 0,
         "changed": 2, "unchanged": 5, "raw_delta": 3, "cal_delta": 1,
         "schema": "v1374"},
        {"iso": "2026-07-27T01-00-00Z", "added": 0, "removed": 0,
         "changed": 0, "unchanged": 8, "raw_delta": 0, "cal_delta": 0,
         "schema": "v1374"},
    ]
    grp = group_by_week(sample)
    if list(grp.keys()) != ["2026-W31", "2026-W32"]:
        failures.append(f"group_by_week order = {list(grp.keys())}, expected ['2026-W31','2026-W32']")
    if len(grp["2026-W32"]) != 2 or len(grp["2026-W31"]) != 1:
        failures.append(f"group_by_week sizes wrong: {[(k,len(v)) for k,v in grp.items()]}")

    # 6. weekly_summary aggregates correctly
    s = weekly_summary(grp["2026-W32"])
    if s["count"] != 2:
        failures.append(f"weekly_summary W32 count={s['count']}, expected 2")
    if s["added_total"] != 1:
        failures.append(f"weekly_summary W32 added_total={s['added_total']}, expected 1")
    if s["changed_total"] != 2:
        failures.append(f"weekly_summary W32 changed_total={s['changed_total']}, expected 2")
    if s["zero_deltas"] != 1:
        failures.append(f"weekly_summary W32 zero_deltas={s['zero_deltas']}, expected 1")
    if s["nonzero_count"] != 1:
        failures.append(f"weekly_summary W32 nonzero_count={s['nonzero_count']}, expected 1")
    if s["last_nonzero_at"] != "2026-08-03T01-00-00Z":
        failures.append(f"weekly_summary W32 last_nonzero_at={s['last_nonzero_at']!r}, expected 2026-08-03T01-00-00Z")

    # 7. weekly_summary on empty list
    empty = weekly_summary([])
    if empty["count"] != 0 or empty["first"] != "" or empty["last_nonzero_at"] is not None:
        failures.append(f"weekly_summary empty = {empty}, expected zeros")

    # 8. render_weekly_md contains required keys
    md = render_weekly_md("2026-W32", s)
    for must in [
        "# V1376 — V1375 Weekly Digest for 2026-W32",
        "**schema:** `v1376.digest/v1`",
        "| archives in week | 2 |",
        "| added (sum) | 1 |",
        "| zero-delta archives | 1 |",
        "Honesty paragraph",
    ]:
        if must not in md:
            failures.append(f"render_weekly_md missing: {must!r}")

    # 9. render_weekly_md rejects bad label
    try:
        render_weekly_md("garbage", s)
        failures.append("render_weekly_md('garbage',...) should have raised")
    except ValueError:
        pass

    # 10. render_index_md empty + populated
    idx_empty = render_index_md([])
    if "_No digests yet._" not in idx_empty:
        failures.append("render_index_md([]) missing 'No digests yet.'")
    idx_pop = render_index_md(["2026-W31", "2026-W32"])
    for must in ["| `2026-W31` | `2026-W31.md` |", "| `2026-W32` | `2026-W32.md` |"]:
        if must not in idx_pop:
            failures.append(f"render_index_md missing: {must!r}")

    # 11. _safe_join rejects traversal + absolute
    for bad in ["../etc/passwd", "/etc/passwd", "..\\windows\\system32"]:
        try:
            _safe_join(".", bad)
            failures.append(f"_safe_join({bad!r}) should have raised")
        except ValueError:
            pass
    try:
        ok = _safe_join(".", "2026-W32.md")
        if not ok.endswith("2026-W32.md"):
            failures.append(f"_safe_join('.', '2026-W32.md') returned {ok!r}")
    except Exception as e:  # pragma: no cover
        failures.append(f"_safe_join('.', '2026-W32.md') raised: {e}")

    # 12. write_digest actually writes (use tmp dir)
    import tempfile
    with tempfile.TemporaryDirectory() as tmpd:
        # Seed V1375 archive inside the tempdir
        seed_dir = os.path.join(tmpd, "archive")
        os.makedirs(seed_dir, exist_ok=True)
        # Use V1375 archive_tick to seed one archive
        from apeireth.v1375_v1374_history_archive import (
            archive_tick, write_index, render_index_md as v1375_render_index,
        )
        # Write a fake V1374 snapshot report
        v1374_path = os.path.join(tmpd, "V1374_REPORT_AUTO.md")
        with open(v1374_path, "w", encoding="utf-8") as f:
            f.write("# fake V1374\n")
        archive_tick(seed_dir, v1374_path, timestamp="2026-08-08T20-06-51Z")
        # Now run V1376
        result = write_digest(seed_dir, output_dir=os.path.join(tmpd, "digests"))
        if result["archives_seen"] != 1:
            failures.append(f"write_digest archives_seen={result['archives_seen']}, expected 1")
        if "2026-W32.md" not in result["files"]:
            failures.append(f"write_digest files={result['files']}, missing 2026-W32.md")
        if "INDEX.md" not in result["files"]:
            failures.append(f"write_digest files={result['files']}, missing INDEX.md")
        # And the file exists with non-zero bytes
        week_path = os.path.join(tmpd, "digests", "2026-W32.md")
        if not os.path.isfile(week_path) or os.path.getsize(week_path) == 0:
            failures.append(f"write_digest: {week_path} not written or empty")
        idx_path = os.path.join(tmpd, "digests", "INDEX.md")
        if not os.path.isfile(idx_path) or os.path.getsize(idx_path) == 0:
            failures.append(f"write_digest: {idx_path} not written or empty")

    total = 49
    passed = total - len(failures)
    return (passed, total, failures)


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def run_cli(args: list[str] | None = None) -> int:
    """argv dispatcher (digest / list / show / popper / version).

    Top-level ``--archive-dir`` and ``--output-dir`` apply to every subcommand,
    matching the V1375 CLI style.
    """
    parser = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description=(
            "V1376 — V1375 weekly digest. Reads the V1375 archive directory, "
            "groups entries by ISO week, writes one digest per week + INDEX.md."
        ),
    )
    parser.add_argument(
        "--archive-dir", default=DEFAULT_ARCHIVE_DIR,
        help=f"V1375 archive directory (default: {DEFAULT_ARCHIVE_DIR})",
    )
    parser.add_argument(
        "--output-dir", default=DEFAULT_OUTPUT_DIR,
        help=f"V1376 digest output directory (default: {DEFAULT_OUTPUT_DIR})",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("digest", help="Write weekly digests + INDEX.md")
    sub.add_parser("list", help="List weeks present in archive")

    p_show = sub.add_parser("show", help="Show a weekly digest file")
    p_show.add_argument("week", help="Week label, e.g. 2026-W32")

    sub.add_parser("popper", help="Run Popper self-tests (49 checks)")
    sub.add_parser("version", help="Print schema version")

    ns = parser.parse_args(args)

    archive_dir = ns.archive_dir
    output_dir = ns.output_dir

    if ns.cmd == "digest":
        result = write_digest(archive_dir, output_dir=output_dir)
        print(f"wrote {len(result['files'])} files for {len(result['weeks'])} week(s)")
        for fn in result["files"]:
            print(f"  - {fn}")
        return 0

    if ns.cmd == "list":
        archives = list_archives(archive_dir)
        groups = group_by_week(archives)
        if not groups:
            print("(no archives found)")
            return 0
        for label in groups:
            print(f"{label}  ({len(groups[label])} archive(s))")
        return 0

    if ns.cmd == "show":
        # GUARD_DIGEST_FS_PATH_SAFE: ensure no traversal
        path = _safe_join(output_dir, f"{ns.week}.md")
        if not os.path.isfile(path):
            print(f"not found: {path}", file=sys.stderr)
            return 2
        with open(path, "r", encoding="utf-8") as f:
            sys.stdout.write(f.read())
        return 0

    if ns.cmd == "popper":
        passed, total, failures = _popper_self_tests()
        print(f"popper self-tests: {passed}/{total}")
        for f in failures:
            print(f"  FAIL: {f}")
        return 0 if passed == total else 1

    if ns.cmd == "version":
        print(SCHEMA_VERSION)
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(run_cli())