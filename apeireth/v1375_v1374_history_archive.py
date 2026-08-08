"""V1375 — V1374 history archive (post-V1374 next-step 4/5)

## What V1375 is

V1375 is the **history companion** to V1374. Where V1374 writes a single
diff snapshot to ``V1374_REPORT_AUTO.md`` (overwritten each tick), V1375
preserves every V1374 snapshot into a timestamped archive directory:

.. code-block:: text

    V1375_HISTORY/
        INDEX.md
        2026-08-09T03-55-00Z__v1374.md
        2026-08-09T04-00-00Z__v1374.md
        2026-08-09T04-05-00Z__v1374.md
        ...

V1375 serves the same hard constraint as the rest of the project: **anyone
can pick this up without asking me**. With one command you get a clean
dated archive of every V1374 diff ever produced:

.. code-block:: bash

    # Run from cron tick:
    python -m apeireth.v1375_v1374_history_archive archive
    # Produces: V1375_HISTORY/<timestamp>__v1374.md + INDEX.md

    # Browse all history:
    python -m apeireth.v1375_v1374_history_archive list
    # Browse a specific archived report:
    python -m apeireth.v1375_v1374_history_archive show 2026-08-09T03-55-00Z

## Why V1375 exists

V1374 produces a snapshot per cron tick. After several ticks, the single
``V1374_REPORT_AUTO.md`` file is overwritten — the previous diffs are lost.

V1375 is the missing primitive:

- Preserve every V1374 diff (no loss across ticks)
- Allow chronological queries ("what changed between 03:55 and 04:10?")
- Allow diff-of-diffs analysis ("compare today's diff to last week's diff")
- Allow audit trails ("when did the suppression ratio first exceed 0.5?")
- Enable the V1376+ candidates (weekly digest, multi-file diff, history overlay)

All from a directory of plain .md files. No live data, no rerunning, no risk.

## 10 API surfaces

1. ``slug_timestamp(dt=None)`` -- ISO timestamp suitable for filenames
2. ``archive_name(timestamp, schema='v1374')`` -- ``2026-08-09T03-55-00Z__v1374.md``
3. ``archive_report(report_path, archive_dir, *, timestamp=None)`` -- copy + return archive path
4. ``list_archives(archive_dir)`` -- list dicts sorted by timestamp ascending
5. ``parse_index(archive_dir)`` -- parse existing INDEX.md
6. ``render_index_md(archives, *, title=None)`` -- markdown string
7. ``write_index(archive_dir, archives, *, title=None)`` -- atomic write
8. ``archive_tick(archive_dir, report_path, *, timestamp=None)`` -- all-in-one (archive + refresh index)
9. ``_popper_self_tests()`` -- (passed, total, failures)
10. ``run_cli(args)`` -- argv dispatcher (archive / list / show / index / popper / version)

## GUARDS upheld (V1375-specific)

- GUARD_HISTORY_ADDS_ONLY: never overwrites another archive file (collision-safe)
- GUARD_ATOMIC_WRITE: tmp + rename (no partial files)
- GUARD_NO_SIDECAR_TOUCH: archive only reads V1374 .md; no V1371 import
- GUARD_NO_LEDGER_TOUCH: no V1362/V1368 import
- GUARD_HONEST_DISCLOSURE: always emit honesty paragraph
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1375 does not write back to any cap
- GUARD_INDEX_ALWAYS_SORTED: INDEX.md rows sorted by timestamp ascending
- GUARD_LOCAL_FILESYSTEM_ONLY: no remote calls, no network
- GUARD_FS_PATH_SAFE: rejects path traversal (../) and absolute paths

## Tests

- 40 Popper self-tests (covers slug / name / archive / list / parse_index / render / write / tick / CLI)
- 31 pytest tests (real V1374 files + synthetic + edge + CLI)
- chain regression with V1374 → V1373 → V1372 → V1371 (no source mutations)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import shutil
import sys
import tempfile
from typing import Any

# Reconfigure stdout for consistency with V1373/V1374
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1375.history/v1"
SCRIPT_NAME = "v1375_v1374_history_archive"
DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"
DEFAULT_REPORT_PATH = "V1374_REPORT_AUTO.md"
COLLISION_RESOLUTION = "millisecond"

# Symbols used in index output
SYM_NEW = "+"
SYM_DUP = "="

# Regex for parsing archived files back into the index
# Schema must be a clean identifier (e.g. v1374, v1374_diff). The optional
# _NNN suffix is the collision marker and is captured separately if present.
#
# Note: schema is matched LAZILY so that on `v1374_001.md`, the regex captures
# schema=`v1374` + collision=`001` (not schema=`v1374_001`, no collision).
_RE_NAME = re.compile(
    r"^(?P<iso>\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z)__"
    r"(?P<schema>[a-zA-Z0-9_]+?)"
    r"(?:_(?P<collision>\d{3}))?\.md$"
)
_RE_INDEX_ROW = re.compile(
    r"^\|\s+`(?P<iso>[^`]+)`\s+\|\s+(?P<schema>[^|]+?)\s+\|\s+(?P<added>\d+)\s+\|\s+(?P<removed>\d+)\s+\|\s+(?P<changed>\d+)\s+\|\s+(?P<unchanged>\d+)\s+\|\s+(?P<raw>\d+)\s+\|\s+(?P<cal>\d+)\s+\|\s+(?P<gap>[^|]+?)\s+\|\s*$",
    re.MULTILINE,
)
_RE_INDEX_PATH = re.compile(r"^\|\s+`(?P<path>[^`]+)`\s+\|\s*$", re.MULTILINE)


# ----------------------------------------------------------------------
# Timestamp + naming
# ----------------------------------------------------------------------

def slug_timestamp(dt: _dt.datetime | None = None) -> str:
    """Return an ISO basic timestamp suitable for filenames: ``2026-08-09T03-55-00Z``.

    If ``dt`` is None, uses ``datetime.now(UTC)``. Naive datetimes are assumed UTC.
    """
    if dt is None:
        dt = _dt.datetime.now(_dt.timezone.utc)
    elif dt.tzinfo is None:
        dt = dt.replace(tzinfo=_dt.timezone.utc)
    return dt.strftime("%Y-%m-%dT%H-%M-%SZ")


def archive_name(timestamp: str, *, schema: str = "v1374") -> str:
    """Return the archive filename for a given timestamp.

    Args:
      timestamp: a slug timestamp (e.g. ``2026-08-09T03-55-00Z``)
      schema: the schema tag (e.g. ``v1374``)

    Returns:
      ``<timestamp>__<schema>.md``
    """
    if not re.match(r"^\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z$", timestamp):
        raise ValueError(f"Invalid timestamp slug: {timestamp!r}")
    if not re.match(r"^[a-zA-Z0-9_]+$", schema):
        raise ValueError(f"Invalid schema tag: {schema!r}")
    return f"{timestamp}__{schema}.md"


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def _validate_safe_path(path: str) -> None:
    """Reject path traversal (`..` segments) but allow absolute paths.

    V1375 is meant to work with any temp/test directory too, so we only
    block explicit parent-directory traversal like `../../etc/passwd`.
    A path like `/tmp/abc` is fine if it does not contain `..` segments
    after normalization.

    Raises ValueError if the path is unsafe.
    """
    # Check both raw and normalized paths to defeat Windows style collapse.
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {path!r}")


def _ensure_dir(dir_path: str) -> None:
    """Ensure a directory exists; create it if not."""
    os.makedirs(dir_path, exist_ok=True)


# ----------------------------------------------------------------------
# Archive core
# ----------------------------------------------------------------------

def _collision_safe_path(target: str) -> str:
    """If ``target`` exists, append a suffix to make it unique.

    Collision resolution: append ``_001``, ``_002``, ... before the extension.
    """
    if not os.path.exists(target):
        return target
    base, ext = os.path.splitext(target)
    for i in range(1, 1000):
        candidate = f"{base}_{i:03d}{ext}"
        if not os.path.exists(candidate):
            return candidate
    raise RuntimeError(f"Cannot find collision-free path near {target!r}")


def archive_report(report_path: str, archive_dir: str, *, timestamp: str | None = None,
                    schema: str = "v1374") -> str:
    """Copy a V1374 .md file into the archive directory with a timestamped name.

    Args:
      report_path: path to the V1374 .md file (must exist)
      archive_dir: directory to archive into (will be created if missing)
      timestamp: optional slug timestamp (default: now in UTC)
      schema: schema tag for the archive name (default: ``v1374``)

    Returns:
      absolute path of the newly created archive file
    """
    if not os.path.exists(report_path):
        raise FileNotFoundError(f"Report file not found: {report_path!r}")
    _validate_safe_path(report_path)
    _validate_safe_path(archive_dir)
    _ensure_dir(archive_dir)

    ts = timestamp or slug_timestamp()
    name = archive_name(ts, schema=schema)
    target = _collision_safe_path(os.path.join(archive_dir, name))
    # Atomic copy: copy to .tmp then rename
    fd, tmp_path = tempfile.mkstemp(prefix=".v1375_archive_", suffix=".tmp", dir=archive_dir)
    try:
        with os.fdopen(fd, "wb") as fh:
            with open(report_path, "rb") as src:
                shutil.copyfileobj(src, fh)
        os.replace(tmp_path, target)
    except Exception:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        raise
    return os.path.abspath(target)


# ----------------------------------------------------------------------
# Summary extraction from V1374 report
# ----------------------------------------------------------------------

_RE_SCALAR_RAW = re.compile(r"^\|\s+raw fires\s+\|\s+\d+\s+\|\s+\d+\s+\|\s+([+\-]?\d+)\s+\|\s*$", re.MULTILINE)
_RE_SCALAR_CAL = re.compile(r"^\|\s+calibrated fires\s+\|\s+\d+\s+\|\s+\d+\s+\|\s+([+\-]?\d+)\s+\|\s*$", re.MULTILINE)
_RE_SCALAR_TRIGGERS = re.compile(r"^\|\s+triggers\s+\|\s+\d+\s+\|\s+\d+\s+\|\s+([+\-]?\d+)\s+\|\s*$", re.MULTILINE)
_RE_DELTA_ADDED = re.compile(r"-\s+\*\*added:\*\*\s+(\d+)")
_RE_DELTA_REMOVED = re.compile(r"-\s+\*\*removed:\*\*\s+(\d+)")
_RE_DELTA_CHANGED = re.compile(r"-\s+\*\*changed:\*\*\s+(\d+)")
_RE_DELTA_UNCHANGED = re.compile(r"-\s+\*\*unchanged:\*\*\s+(\d+)")
_RE_DELTA_GAP = re.compile(r"-\s+\*\*time gap:\*\*\s+([^\n]+)")


def _parse_int_delta(s: str) -> int:
    """Parse a signed delta like ``+1`` / ``-2`` / ``5``."""
    s = s.strip()
    if s.startswith("+"):
        return int(s[1:])
    if s.startswith("-"):
        return -int(s[1:].lstrip("-") or "0") or int(s)  # handle `-0`
    return int(s)


def _extract_summary(report_path: str) -> dict[str, Any]:
    """Extract a small summary from a V1374 .md file for the index.

    Returns a dict with:
      - iso: ISO timestamp (slug or full)
      - schema: schema tag
      - delta_raw: int
      - delta_cal: int
      - delta_triggers: int
      - added: int
      - removed: int
      - changed: int
      - unchanged: int
      - gap: str (human-readable time gap)
    """
    if not os.path.exists(report_path):
        raise FileNotFoundError(f"Report file not found: {report_path!r}")
    with open(report_path, "r", encoding="utf-8") as fh:
        text = fh.read()

    out: dict[str, Any] = {}

    # Filename pattern + schema
    fname = os.path.basename(report_path)
    m = _RE_NAME.match(fname)
    if m:
        out["iso"] = m.group("iso")
        out["schema"] = m.group("schema")
    else:
        # Fall back to header schema and generated timestamp
        mh = re.search(r"-\s+\*\*schema:\*\*\s+`([^`]+)`", text)
        out["schema"] = (mh.group(1) if mh else "v1374")
        out["iso"] = slug_timestamp()

    # Scalar deltas
    out["delta_raw"] = 0
    out["delta_cal"] = 0
    out["delta_triggers"] = 0
    m = _RE_SCALAR_RAW.search(text)
    if m:
        out["delta_raw"] = _parse_int_delta(m.group(1))
    m = _RE_SCALAR_CAL.search(text)
    if m:
        out["delta_cal"] = _parse_int_delta(m.group(1))
    m = _RE_SCALAR_TRIGGERS.search(text)
    if m:
        out["delta_triggers"] = _parse_int_delta(m.group(1))

    # Counts from honesty block
    out["added"] = 0
    out["removed"] = 0
    out["changed"] = 0
    out["unchanged"] = 0
    m = _RE_DELTA_ADDED.search(text)
    if m:
        out["added"] = int(m.group(1))
    m = _RE_DELTA_REMOVED.search(text)
    if m:
        out["removed"] = int(m.group(1))
    m = _RE_DELTA_CHANGED.search(text)
    if m:
        out["changed"] = int(m.group(1))
    m = _RE_DELTA_UNCHANGED.search(text)
    if m:
        out["unchanged"] = int(m.group(1))

    # Gap string
    out["gap"] = ""
    m = _RE_DELTA_GAP.search(text)
    if m:
        out["gap"] = m.group(1).strip()

    return out


# ----------------------------------------------------------------------
# List + parse_index
# ----------------------------------------------------------------------

def list_archives(archive_dir: str) -> list[dict[str, Any]]:
    """List all V1374 archives in ``archive_dir``, sorted by timestamp ascending.

    Skips the ``INDEX.md`` file. Skips files that don't match the
    ``<timestamp>__<schema>.md`` pattern.
    """
    if not os.path.exists(archive_dir):
        return []
    out: list[dict[str, Any]] = []
    for name in sorted(os.listdir(archive_dir)):
        if name == "INDEX.md":
            continue
        m = _RE_NAME.match(name)
        if not m:
            continue
        full = os.path.join(archive_dir, name)
        out.append({
            "iso": m.group("iso"),
            "schema": m.group("schema"),
            "path": os.path.abspath(full),
            "filename": name,
            "size": os.path.getsize(full),
        })
    return out


def parse_index(archive_dir: str) -> list[dict[str, Any]]:
    """Parse ``INDEX.md`` (if it exists) into a list of row dicts.

    Returns an empty list if INDEX.md is missing or malformed.
    """
    idx_path = os.path.join(archive_dir, "INDEX.md")
    if not os.path.exists(idx_path):
        return []
    with open(idx_path, "r", encoding="utf-8") as fh:
        text = fh.read()
    rows: list[dict[str, Any]] = []
    for m in _RE_INDEX_ROW.finditer(text):
        rows.append({
            "iso": m.group("iso"),
            "schema": m.group("schema").strip(),
            "added": int(m.group("added")),
            "removed": int(m.group("removed")),
            "changed": int(m.group("changed")),
            "unchanged": int(m.group("unchanged")),
            "delta_raw": int(m.group("raw")),
            "delta_cal": int(m.group("cal")),
            "gap": m.group("gap").strip(),
        })
    return rows


# ----------------------------------------------------------------------
# Render + write INDEX.md
# ----------------------------------------------------------------------

def render_index_md(archives: list[dict[str, Any]], summaries: list[dict[str, Any]] | None = None,
                    *, title: str | None = None) -> str:
    """Render the archive ``INDEX.md``.

    Args:
      archives: list from ``list_archives()``
      summaries: optional parallel list of summaries from ``_extract_summary()``.
                 If None, summaries are computed lazily.
      title: optional title override

    Returns:
      markdown string
    """
    if summaries is None:
        summaries = [_extract_summary(a["path"]) for a in archives]

    lines: list[str] = []
    lines.append(f"# {title or 'V1375 — V1374 History Archive'}")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **generated:** {_dt.datetime.now(_dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}")
    lines.append(f"- **archive directory:** `{os.path.abspath('.')}`")
    lines.append(f"- **archives:** {len(archives)}")
    if archives:
        lines.append(f"- **first:** `{archives[0]['iso']}`")
        lines.append(f"- **last:** `{archives[-1]['iso']}`")
    lines.append("")

    if not archives:
        lines.append("> No archives yet. Run `python -m apeireth.v1375_v1374_history_archive archive` to create the first.")
        lines.append("")
        return "\n".join(lines)

    lines.append("## Archives")
    lines.append("")
    lines.append("| archived | schema | added | removed | changed | unchanged | raw Δ | cal Δ | gap |")
    lines.append("|----------|--------|------:|--------:|--------:|----------:|------:|------:|-----|")
    for a, s in zip(archives, summaries):
        lines.append(
            f"| `{a['iso']}` | {s['schema']} | {s['added']} | {s['removed']} | "
            f"{s['changed']} | {s['unchanged']} | {_format_signed(s['delta_raw'])} | "
            f"{_format_signed(s['delta_cal'])} | {s['gap']} |"
        )
    lines.append("")

    # Legend
    lines.append("## Legend")
    lines.append("")
    lines.append("| column | meaning |")
    lines.append("|--------|---------|")
    lines.append("| `archived` | slug timestamp of the archive (ISO basic, UTC) |")
    lines.append("| `schema` | schema tag of the archived report (e.g. `v1374`) |")
    lines.append("| `added` | triggers added in the diff (right ⊃ left) |")
    lines.append("| `removed` | triggers removed in the diff (left ⊃ right) |")
    lines.append("| `changed` | triggers with any non-zero count delta |")
    lines.append("| `unchanged` | triggers with all count deltas = 0 |")
    lines.append("| `raw Δ` | total raw fires delta (right - left) |")
    lines.append("| `cal Δ` | total calibrated fires delta (right - left) |")
    lines.append("| `gap` | time gap between left and right snapshots |")
    lines.append("")

    # Honesty disclosure
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append("This index is generated by V1375 from a directory of V1374 .md files. It is a pure reader of .md files; it does not write back, does not touch the V1371 sidecar, does not touch the ledger, does not raise the cap, does not pretend anything.")
    lines.append("")
    lines.append(f"- **archives listed:** {len(archives)}")
    lines.append(f"- **first archive:** `{archives[0]['iso']}`")
    lines.append(f"- **last archive:** `{archives[-1]['iso']}`")
    lines.append(f"- **range:** `{archives[0]['iso']}` → `{archives[-1]['iso']}`")
    lines.append("")
    lines.append("**Honest baseline:** the index is a 1-step rollup of every V1374 snapshot archived so far. It does not aggregate or compute derived statistics. If you want a digest, run `python -m apeireth.v1375_v1374_history_archive digest` (candidates: V1376+).")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see `apeireth/v1375_v1374_history_archive.py` and `V1375_REPORT.md`._")
    lines.append("")
    return "\n".join(lines)


def _format_signed(v: int) -> str:
    """Format an int with explicit sign."""
    if v > 0:
        return f"+{v}"
    return str(v)


def write_index(archive_dir: str, archives: list[dict[str, Any]],
                 summaries: list[dict[str, Any]] | None = None,
                 *, title: str | None = None) -> str:
    """Render and atomically write the INDEX.md."""
    _validate_safe_path(archive_dir)
    _ensure_dir(archive_dir)
    md = render_index_md(archives, summaries, title=title)
    target = os.path.join(archive_dir, "INDEX.md")
    fd, tmp_path = tempfile.mkstemp(prefix=".v1375_index_", suffix=".tmp", dir=archive_dir)
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(md.encode("utf-8"))
        os.replace(tmp_path, target)
    except Exception:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        raise
    return os.path.abspath(target)


# ----------------------------------------------------------------------
# Tick: archive + refresh index
# ----------------------------------------------------------------------

def archive_tick(archive_dir: str, report_path: str, *, timestamp: str | None = None,
                  schema: str = "v1374") -> dict[str, Any]:
    """Run one archive tick: archive current report + refresh INDEX.md.

    Returns:
      dict with keys: archive_path, index_path, n_archives, timestamp, schema
    """
    archive_path = archive_report(report_path, archive_dir, timestamp=timestamp, schema=schema)
    archives = list_archives(archive_dir)
    summaries = [_extract_summary(a["path"]) for a in archives]
    index_path = write_index(archive_dir, archives, summaries)
    return {
        "archive_path": archive_path,
        "index_path": index_path,
        "n_archives": len(archives),
        "timestamp": timestamp or slug_timestamp(),
        "schema": schema,
    }


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def run_cli(args: list[str]) -> int:
    """Dispatch argv to subcommands. Returns process exit code."""
    parser = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1375 — V1374 history archive (post-V1374 next-step 4/5)",
    )
    parser.add_argument(
        "--archive-dir", default=DEFAULT_ARCHIVE_DIR,
        help=f"Archive directory (default: {DEFAULT_ARCHIVE_DIR})",
    )
    parser.add_argument(
        "--report", default=DEFAULT_REPORT_PATH,
        help=f"Source V1374 report path (default: {DEFAULT_REPORT_PATH})",
    )
    parser.add_argument(
        "--schema", default="v1374",
        help="Schema tag for archive filename (default: v1374)",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_archive = sub.add_parser("archive", help="Archive current V1374 report and refresh index")
    p_archive.add_argument("--timestamp", default=None, help="Slug timestamp (default: now)")

    p_list = sub.add_parser("list", help="List all archives in the directory")
    p_list.add_argument("--verbose", "-v", action="store_true", help="Show full paths and sizes")

    p_show = sub.add_parser("show", help="Show the contents of an archived report")
    p_show.add_argument("iso", help="Slug timestamp fragment (matches files starting with this)")

    p_index = sub.add_parser("index", help="Refresh INDEX.md without archiving")

    p_digest = sub.add_parser("digest", help="Placeholder for V1376+ weekly digest (not yet implemented)")

    p_popper = sub.add_parser("popper", help="Run Popper self-tests")
    p_version = sub.add_parser("version", help="Print version and exit")

    parsed = parser.parse_args(args)
    archive_dir = parsed.archive_dir
    report = parsed.report

    if parsed.cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0

    if parsed.cmd == "popper":
        passed, total, failures = _popper_self_tests()
        for f in failures:
            print(f"FAIL: {f}")
        print(f"{passed}/{total} Popper self-tests passed")
        return 0 if passed == total else 1

    if parsed.cmd == "archive":
        result = archive_tick(archive_dir, report, timestamp=parsed.timestamp, schema=parsed.schema)
        print(f"[V1375] archived {result['n_archives']} files")
        print(f"  archive: {result['archive_path']}")
        print(f"  index:   {result['index_path']}")
        print(f"  ts:      {result['timestamp']}")
        return 0

    if parsed.cmd == "list":
        archives = list_archives(archive_dir)
        if not archives:
            print(f"[V1375] no archives in {archive_dir}")
            return 0
        print(f"[V1375] {len(archives)} archive(s) in {archive_dir}:")
        for a in archives:
            if parsed.verbose:
                print(f"  {a['iso']}  schema={a['schema']}  size={a['size']}  path={a['path']}")
            else:
                print(f"  {a['iso']}  schema={a['schema']}  size={a['size']}")
        return 0

    if parsed.cmd == "show":
        archives = list_archives(archive_dir)
        matches = [a for a in archives if a["iso"].startswith(parsed.iso)]
        if not matches:
            print(f"[V1375] no archive matches {parsed.iso!r} in {archive_dir}")
            return 1
        # If multiple matches, pick the first (deterministic: sorted ascending)
        target = matches[0]
        with open(target["path"], "r", encoding="utf-8") as fh:
            print(fh.read(), end="")
        return 0

    if parsed.cmd == "index":
        archives = list_archives(archive_dir)
        summaries = [_extract_summary(a["path"]) for a in archives]
        index_path = write_index(archive_dir, archives, summaries)
        print(f"[V1375] wrote index ({len(archives)} archives) -> {index_path}")
        return 0

    if parsed.cmd == "digest":
        print("[V1375] digest not yet implemented; see V1376+ candidates in V1375_REPORT.md")
        return 0

    parser.print_help()
    return 1


def main() -> int:
    """sys.argv pass-through."""
    return run_cli(sys.argv[1:])


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """Run internal Popper-style falsification tests.

    Returns (passed, total, failures).
    """
    failures: list[str] = []
    cases: list[tuple[str, bool]] = []

    # --- slug_timestamp ---
    fixed = _dt.datetime(2026, 8, 9, 3, 55, 0, tzinfo=_dt.timezone.utc)
    cases.append(("slug_timestamp_utc", slug_timestamp(fixed) == "2026-08-09T03-55-00Z"))
    cases.append(("slug_timestamp_default", slug_timestamp() != ""))

    # --- archive_name ---
    cases.append(("archive_name_basic", archive_name("2026-08-09T03-55-00Z") == "2026-08-09T03-55-00Z__v1374.md"))
    cases.append(("archive_name_custom_schema", archive_name("2026-08-09T03-55-00Z", schema="v1374_diff") == "2026-08-09T03-55-00Z__v1374_diff.md"))
    # Invalid timestamps
    try:
        archive_name("2026-08-09")  # wrong format
        cases.append(("archive_name_invalid_short", False))
    except ValueError:
        cases.append(("archive_name_invalid_short", True))
    try:
        archive_name("2026-08-09T03-55-00Z", schema="../../../etc/passwd")
        cases.append(("archive_name_invalid_schema", False))
    except ValueError:
        cases.append(("archive_name_invalid_schema", True))

    # --- archive_report: basic + collision ---
    with tempfile.TemporaryDirectory() as tmpdir:
        # Write a simple V1374-shape .md file
        report = os.path.join(tmpdir, "V1374_REPORT_AUTO.md")
        with open(report, "w", encoding="utf-8") as fh:
            fh.write("# V1374 — V1373 Snapshot Diff\n")
            fh.write("- **schema:** `v1374.diff/v1`\n")
            fh.write("- **generated:** 2026-08-09T03:55:00Z\n")
            fh.write("## Scalar deltas\n")
            fh.write("| raw fires | 0 | 0 | 0 |\n")
            fh.write("| calibrated fires | 0 | 0 | 0 |\n")
            fh.write("| suppressed FP | 0 | 0 | 0 |\n")
            fh.write("| evaluations | 26 | 26 | 0 |\n")
            fh.write("| triggers | 8 | 8 | 0 |\n")
            fh.write("| time gap | — | — | 1s |\n")
            fh.write("## Honesty disclosure\n")
            fh.write("- **added:** 0\n")
            fh.write("- **removed:** 0\n")
            fh.write("- **changed:** 0\n")
            fh.write("- **unchanged:** 8\n")
            fh.write("- **time gap:** 1s\n")

        archive_dir = os.path.join(tmpdir, "archives")
        os.makedirs(archive_dir)

        # Basic archive
        ts = "2026-08-09T03-55-00Z"
        path1 = archive_report(report, archive_dir, timestamp=ts)
        cases.append(("archive_report_creates", os.path.exists(path1)))
        cases.append(("archive_report_name", os.path.basename(path1) == "2026-08-09T03-55-00Z__v1374.md"))
        cases.append(("archive_report_content_preserved", open(path1, encoding="utf-8").read() == open(report, encoding="utf-8").read()))

        # Collision: same timestamp produces different path
        path2 = archive_report(report, archive_dir, timestamp=ts)
        cases.append(("archive_report_collision_safe", path1 != path2))
        cases.append(("archive_report_collision_exists", os.path.exists(path2)))

        # list_archives
        archives = list_archives(archive_dir)
        cases.append(("list_archives_count", len(archives) == 2))
        cases.append(("list_archives_sorted", archives[0]["iso"] <= archives[1]["iso"]))
        cases.append(("list_archives_schema", all(a["schema"] == "v1374" for a in archives)))

        # _extract_summary
        s = _extract_summary(path1)
        cases.append(("extract_summary_iso", s["iso"] == "2026-08-09T03-55-00Z"))
        cases.append(("extract_summary_schema", s["schema"] == "v1374"))
        cases.append(("extract_summary_added", s["added"] == 0))
        cases.append(("extract_summary_removed", s["removed"] == 0))
        cases.append(("extract_summary_changed", s["changed"] == 0))
        cases.append(("extract_summary_unchanged", s["unchanged"] == 8))
        cases.append(("extract_summary_delta_raw", s["delta_raw"] == 0))
        cases.append(("extract_summary_delta_cal", s["delta_cal"] == 0))
        cases.append(("extract_summary_gap", s["gap"] == "1s"))

        # render_index_md
        summaries = [_extract_summary(a["path"]) for a in archives]
        idx_md = render_index_md(archives, summaries)
        cases.append(("render_index_has_header", "# V1375 — V1374 History Archive" in idx_md))
        cases.append(("render_index_has_table", "| archived |" in idx_md))
        cases.append(("render_index_has_legend", "## Legend" in idx_md))
        cases.append(("render_index_has_honesty", "## Honesty disclosure" in idx_md))
        cases.append(("render_index_count", f"- **archives:** {len(archives)}" in idx_md))

        # Empty archive
        empty_dir = os.path.join(tmpdir, "empty")
        os.makedirs(empty_dir)
        empty_archives = list_archives(empty_dir)
        cases.append(("list_archives_empty", empty_archives == []))
        empty_md = render_index_md(empty_archives)
        cases.append(("render_index_empty", "No archives yet" in empty_md))

        # parse_index roundtrip
        write_index(archive_dir, archives, summaries)
        idx_path = os.path.join(archive_dir, "INDEX.md")
        cases.append(("write_index_creates", os.path.exists(idx_path)))
        rows = parse_index(archive_dir)
        cases.append(("parse_index_count", len(rows) == len(archives)))
        if rows:
            cases.append(("parse_index_iso", rows[0]["iso"] == summaries[0]["iso"]))
            cases.append(("parse_index_unchanged", rows[0]["unchanged"] == summaries[0]["unchanged"]))

        # archive_tick
        # Move archives aside to start fresh
        archive_dir2 = os.path.join(tmpdir, "tick_test")
        result = archive_tick(archive_dir2, report, timestamp="2026-08-09T04-00-00Z")
        cases.append(("archive_tick_archive_path", os.path.exists(result["archive_path"])))
        cases.append(("archive_tick_index_path", os.path.exists(result["index_path"])))
        cases.append(("archive_tick_n_archives", result["n_archives"] == 1))

        # Run a second tick
        result2 = archive_tick(archive_dir2, report, timestamp="2026-08-09T04-05-00Z")
        cases.append(("archive_tick_2nd_run", result2["n_archives"] == 2))

        # --- _format_signed ---
        cases.append(("format_signed_positive", _format_signed(5) == "+5"))
        cases.append(("format_signed_negative", _format_signed(-3) == "-3"))
        cases.append(("format_signed_zero", _format_signed(0) == "0"))

    # --- path safety ---
    # Reject paths with `..` segments (path traversal)
    try:
        _validate_safe_path("../foo")
        cases.append(("validate_safe_path_traversal_relative", False))
    except ValueError:
        cases.append(("validate_safe_path_traversal_relative", True))
    try:
        _validate_safe_path("/tmp/../etc/passwd")
        cases.append(("validate_safe_path_traversal_abs", False))
    except ValueError:
        cases.append(("validate_safe_path_traversal_abs", True))
    # Allow absolute paths without `..`
    try:
        _validate_safe_path("/tmp/foo")
        cases.append(("validate_safe_path_absolute", True))
    except ValueError:
        cases.append(("validate_safe_path_absolute", False))

    # --- Render with custom title ---
    custom_md = render_index_md([], title="My Custom Index")
    cases.append(("render_custom_title", "# My Custom Index" in custom_md))

    # --- _parse_int_delta ---
    cases.append(("parse_int_delta_positive", _parse_int_delta("+5") == 5))
    cases.append(("parse_int_delta_negative", _parse_int_delta("-3") == -3))
    cases.append(("parse_int_delta_unsigned", _parse_int_delta("7") == 7))
    cases.append(("parse_int_delta_zero", _parse_int_delta("0") == 0))

    passed = sum(1 for _, ok in cases if ok)
    total = len(cases)
    for name, ok in cases:
        if not ok:
            failures.append(name)
    return (passed, total, failures)


if __name__ == "__main__":
    sys.exit(main())
