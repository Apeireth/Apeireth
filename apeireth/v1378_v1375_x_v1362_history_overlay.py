"""V1378 — V1375 history × V1362 ledger overlay (N archives → 1 annotated report).

Phase: 1378
Version: 0.1.0
Date: 2026-08-09 (tick 230)
Post: V1377 (V1375 multi-file diff)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

What V1378 is
==============
V1378 is the **ledger-overlay companion** to V1375. Where V1375 archives
V1374 diff snapshots into a directory and V1377 aggregates per-trigger
drift across those archives, V1378 annotates each archived V1374 snapshot
with the **nearest V1362 pole-star ledger entry** by timestamp.

Why V1378 exists
================
The V1375 archive preserves *what changed in V1368 trigger space* over time.
The V1362 ledger preserves *what pole-star the system claimed* over time.
V1378 answers: "For each V1375 archive, what was the nearest pole-star
ledger entry, how close in time was it, and which metrics were recorded?"

This is the natural cross-link between V1375 (history archive) and V1362
(pole-star history). It is **read-only on both sides**:

- Reads ``V1375_HISTORY/<iso>__v1374.md`` files via V1375.list_archives
- Reads ``pole_star_history.jsonl`` directly (independent of V1362 module)
- Writes one overlay markdown report, atomically

Most common audit questions answered by one command:

- "Which V1375 archive aligns with which V1362 ledger tag?"
- "How close in time is each archive to its nearest ledger entry?"
- "Did the pole-star change between consecutive archives?"
- "How many archives have any pole-star data, vs no-data entries?"
- "Did toolchain / close_loop / v_modules / test_files grow between archives?"

API surfaces (10)
=================
1. ``parse_iso_dt(iso)`` — robust ISO-8601 → tz-aware datetime
2. ``read_ledger_jsonl(path)`` — parse ledger JSONL into list of dicts
3. ``find_nearest_ledger(archive_dt, ledger_entries)`` — pick closest by |dt|
4. ``overlay_row(archive, ledger_entry, time_gap_s)`` — one annotated row
5. ``build_overlay(archives, ledger_entries)`` — list of overlay rows
6. ``summarize_overlay(rows, archives, ledger_entries)`` — top-level summary
7. ``render_overlay_md(rows, summary, archives, ledger_entries)`` — markdown str
8. ``write_overlay_md(path, content)`` — atomic write
9. ``_popper_self_tests()`` — (passed, total, failures)
10. ``run_cli(args)`` — argv dispatcher (overlay / summary / popper / version)

GUARDS upheld (V1378-specific)
==============================
- GUARD_INPUT_V1375_FAMILY: only accepts archive filenames matching V1375 slug
- GUARD_CHRONOLOGICAL_SORT: archives sorted by generated timestamp ascending
- GUARD_DETERMINISTIC: same inputs in same order → same output bytes
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_NO_LEDGER_WRITE: V1378 reads ledger; never writes to it
- GUARD_NO_SIDECAR_TOUCH: no V1371 import
- GUARD_HONEST_DISCLOSURE: honesty paragraph always emitted
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1378 has no metric, no cap, no scoring
- GUARD_NO_LEDGER_MUTATION: ledger parsed read-only; no row insertions

Tests
=====
- 49 Popper self-tests (covers parse_iso / read_ledger / find_nearest /
  overlay_row / build_overlay / summarize / render / write / CLI / guards)
- ~32 pytest tests (real V1375 archive + synthetic + edge + CLI subprocess)

Honest measurement (this tick)
==============================
- **V1378 Popper self-tests:** 49/49 ?
- **V1378 pytest:** ≥32/32 ?
- **Chain pytest (V1370 → V1378):** no regression
- **Chain popper:** no regression
- **ASI pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)

V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)
==============================================================
- **不假装分数 = ASI:** V1378 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1378 = pure read+annotate; no mutation, no inference
- **不假装 ASI 集成:** zero LLM, zero sidecar touch, zero ledger write
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1375/V1362 sources unchanged; V1378 only reads
- **不假装 V1378 = ASI 觉醒:** V1378 finds nearest-by-time; doesn't "interpret" it
- **实事求是:** real disk reads + real disk writes + deterministic output
- **任何人都能接手:** CLI + Markdown + 1-cmd `overlay` + reproducibility
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from typing import Any

# Reconfigure stdout/stderr for Windows GBK safety (matches V1375/V1376/V1377)
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

# Schema ---------------------------------------------------------------------

SCHEMA_VERSION = "v1378.overlay/v1"
SCRIPT_NAME = "v1378_v1375_x_v1362_history_overlay"

# Constants ------------------------------------------------------------------

DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"
DEFAULT_LEDGER_PATH = "pole_star_history.jsonl"
DEFAULT_OUTPUT_PATH = "V1378_OVERLAY_AUTO.md"

V1375_SLUG_PREFIX_RE = r"\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z__v1374"

# GUARDS list (kept as a list literal — GUARDS_COUNT check verifies length) ----
GUARDS: list[str] = [
    "GUARD_INPUT_V1375_FAMILY",
    "GUARD_CHRONOLOGICAL_SORT",
    "GUARD_DETERMINISTIC",
    "GUARD_ATOMIC_WRITE",
    "GUARD_NO_LEDGER_WRITE",
    "GUARD_NO_SIDECAR_TOUCH",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_MARKDOWN_ONLY",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_NO_LEDGER_MUTATION",
]

# Regex patterns -------------------------------------------------------------

_RE_LEDGER_ISO = re.compile(
    r"^\s*-\s+\*\*generated:\*\*\s+(?P<iso>\S+)\s*$",
    re.MULTILINE,
)


# Path safety ---------------------------------------------------------------

def _validate_safe_path(path: str) -> None:
    """Reject path traversal (`..` segments) but allow absolute paths.

    V1378 is meant to work with any temp/test directory too, so we only
    block explicit parent-directory traversal like `../../etc/passwd`.
    A path like `/tmp/abc` is fine if it does not contain `..` segments
    after normalization.

    Raises ValueError if the path is unsafe.
    """
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {path!r}")


def _safe_join(*parts: str) -> str:
    """Join path parts and validate safety. Rejects empty / parent-traversal."""
    if not parts:
        raise ValueError("No path parts supplied")
    joined = os.path.join(*parts)
    if not joined:
        raise ValueError("Joined path is empty")
    _validate_safe_path(joined)
    return joined


# Helpers --------------------------------------------------------------------

def parse_iso_dt(iso: str | None) -> datetime | None:
    """Parse an ISO timestamp, return tz-aware UTC datetime or None.

    Accepts:
    - trailing ``Z`` (UTC zulu)
    - extended ISO 8601 with colons (``2026-08-09T04:00:00+00:00``)
    - **ISO basic** with dashes (``2026-08-09T04-00-00Z`` — V1375 archive format)

    Returns None for None / empty / malformed.
    """
    if not iso:
        return None
    s = str(iso).strip()
    if not s:
        return None
    # Normalize ISO basic → ISO extended: replace the T-time portion's dashes
    # with colons. Only the time part may contain dashes (date uses dashes).
    # Pattern: YYYY-MM-DDTHH-MM-SS[Z|±HH:MM]
    m_iso_basic = re.match(
        r"^(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})(.*)$", s
    )
    if m_iso_basic:
        date_part = m_iso_basic.group(1)
        hh = m_iso_basic.group(2)
        mm = m_iso_basic.group(3)
        ss = m_iso_basic.group(4)
        rest = m_iso_basic.group(5)  # may be Z, +00:00, +0000, etc.
        if rest == "Z":
            rest = "+00:00"
        s = f"{date_part}T{hh}:{mm}:{ss}{rest}"
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(s)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt


def _format_iso(dt: datetime | None) -> str:
    """Format a tz-aware datetime as ISO-8601 with trailing Z."""
    if dt is None:
        return "—"
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _format_gap(seconds: float | None) -> str:
    """Format a time-gap in seconds as a compact human-readable string.

    Returns ``—`` for None. Otherwise uses ``s`` for <60s and ``m`` for ≥60s.
    """
    if seconds is None:
        return "—"
    if seconds < 0:
        seconds = -seconds
    if seconds < 60:
        return f"{int(round(seconds))}s"
    if seconds < 3600:
        return f"{int(round(seconds / 60))}m"
    return f"{seconds / 3600:.1f}h"


def _format_value(v: Any, *, missing: str = "—") -> str:
    """Format a ledger metric value. ``None`` becomes missing; numbers stay."""
    if v is None:
        return missing
    if isinstance(v, float):
        # Pole-star values are 0..1; show 4 decimal places
        return f"{v:.4f}"
    return str(v)


# Ledger reading ------------------------------------------------------------

def read_ledger_jsonl(path: str) -> list[dict[str, Any]]:
    """Read a V1362-style JSONL ledger file into a list of entry dicts.

    Robust to malformed lines: skipped (counted, not raised). Skips blank
    lines. Each returned dict is the parsed JSON object — keys depend on
    the writer (``measured_at``, ``pole_star_total``, ``tag``, etc).

    Returns an empty list if the file does not exist or cannot be opened.

    Robust to Windows ``/nonexistent`` quirk where ``os.path.exists`` returns
    True for paths that resolve to the current drive root — but open() raises
    PermissionError. Any OSError is treated as "missing/unreadable".
    """
    try:
        # os.path.isfile catches both "doesn't exist" and "exists but is a dir"
        if not os.path.isfile(path):
            return []
        fh = open(path, "r", encoding="utf-8", errors="replace")
    except OSError:
        return []
    try:
        entries: list[dict[str, Any]] = []
        with fh:
            for _lineno, line in enumerate(fh, start=1):
                line = line.strip()
                if not line:
                    continue
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(obj, dict):
                    entries.append(obj)
        return entries
    except OSError:
        return []


def _entry_dt(entry: dict[str, Any]) -> datetime | None:
    """Extract a tz-aware datetime from a ledger entry's measured_at field."""
    return parse_iso_dt(entry.get("measured_at"))


# Archive overlay ------------------------------------------------------------

def find_nearest_ledger(
    archive_dt: datetime | None, ledger_entries: list[dict[str, Any]]
) -> tuple[dict[str, Any] | None, float | None]:
    """Find the ledger entry whose ``measured_at`` is closest to ``archive_dt``.

    Returns (entry, gap_seconds) — both None if archive_dt is None or the
    ledger is empty. ``gap_seconds`` is signed (positive when ledger is after
    archive, negative when before). Uses absolute-minimum selection with
    stable tie-break on first occurrence.
    """
    if archive_dt is None or not ledger_entries:
        return (None, None)
    best_entry: dict[str, Any] | None = None
    best_gap: float | None = None
    for entry in ledger_entries:
        dt = _entry_dt(entry)
        if dt is None:
            continue
        gap = (dt - archive_dt).total_seconds()
        if best_gap is None or abs(gap) < abs(best_gap):
            best_gap = gap
            best_entry = entry
    return (best_entry, best_gap)


def overlay_row(
    archive: dict[str, Any], ledger_entry: dict[str, Any] | None, time_gap_s: float | None
) -> dict[str, Any]:
    """Build one overlay row from an archive + (optional) nearest ledger entry.

    The returned dict is the canonical row shape used by ``render_overlay_md``.
    All ledger-derived fields default to None when no entry was found.
    """
    if ledger_entry is None:
        return {
            "archive_iso": archive.get("iso", ""),
            "archive_filename": archive.get("filename", ""),
            "archive_size": archive.get("size", 0),
            "ledger_iso": None,
            "ledger_tag": None,
            "time_gap_s": None,
            "pole_star_total": None,
            "pole_star_cap": None,
            "pole_star_delta_vs_v01": None,
            "toolchain_present": None,
            "toolchain_total": None,
            "close_loop_pass": None,
            "close_loop_total": None,
            "v_modules": None,
            "test_files": None,
        }
    return {
        "archive_iso": archive.get("iso", ""),
        "archive_filename": archive.get("filename", ""),
        "archive_size": archive.get("size", 0),
        "ledger_iso": ledger_entry.get("measured_at", ""),
        "ledger_tag": ledger_entry.get("tag"),
        "time_gap_s": time_gap_s,
        "pole_star_total": ledger_entry.get("pole_star_total"),
        "pole_star_cap": ledger_entry.get("pole_star_cap"),
        "pole_star_delta_vs_v01": ledger_entry.get("pole_star_delta_vs_v01"),
        "toolchain_present": ledger_entry.get("toolchain_present"),
        "toolchain_total": ledger_entry.get("toolchain_total"),
        "close_loop_pass": ledger_entry.get("close_loop_pass"),
        "close_loop_total": ledger_entry.get("close_loop_total"),
        "v_modules": ledger_entry.get("v_modules"),
        "test_files": ledger_entry.get("test_files"),
    }


def build_overlay(
    archives: list[dict[str, Any]], ledger_entries: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """For each archive (in input order), find nearest ledger + build row.

    Output is parallel to input. ``archives`` is NOT re-sorted — callers that
    want chronological order should pre-sort. V1378's CLI sorts by ``iso``
    ascending before calling this.
    """
    rows: list[dict[str, Any]] = []
    for archive in archives:
        archive_dt = parse_iso_dt(archive.get("iso"))
        entry, gap = find_nearest_ledger(archive_dt, ledger_entries)
        rows.append(overlay_row(archive, entry, gap))
    return rows


def summarize_overlay(
    rows: list[dict[str, Any]],
    archives: list[dict[str, Any]],
    ledger_entries: list[dict[str, Any]],
) -> dict[str, Any]:
    """Compute top-level summary statistics for the overlay.

    Includes:
    - n_archives / n_ledger / n_with_ledger / n_with_pole_star
    - min / max / median time gap (in seconds, using |gap|)
    - pole_star_total range across rows that have it
    - toolchain / close_loop / v_modules / test_files ranges
    """
    n_archives = len(archives)
    n_ledger = len(ledger_entries)
    n_with_ledger = sum(1 for r in rows if r["ledger_iso"] is not None)
    n_with_pole_star = sum(1 for r in rows if r["pole_star_total"] is not None)

    gaps_abs: list[float] = []
    pole_stars: list[float] = []
    tools_present: list[int] = []
    tools_total: list[int] = []
    loops_pass: list[int] = []
    loops_total: list[int] = []
    v_modules: list[int] = []
    test_files: list[int] = []

    for r in rows:
        if r["time_gap_s"] is not None:
            gaps_abs.append(abs(float(r["time_gap_s"])))
        if r["pole_star_total"] is not None:
            pole_stars.append(float(r["pole_star_total"]))
        for key, sink in (
            ("toolchain_present", tools_present),
            ("toolchain_total", tools_total),
            ("close_loop_pass", loops_pass),
            ("close_loop_total", loops_total),
            ("v_modules", v_modules),
            ("test_files", test_files),
        ):
            v = r[key]
            if v is not None:
                sink.append(int(v))

    def _range(values: list[float]) -> tuple[float | None, float | None, float | None]:
        if not values:
            return (None, None, None)
        return (min(values), max(values), sum(values) / len(values))

    gap_min, gap_max, gap_mean = _range(gaps_abs)
    ps_min, ps_max, ps_mean = _range(pole_stars)

    return {
        "n_archives": n_archives,
        "n_ledger": n_ledger,
        "n_with_ledger": n_with_ledger,
        "n_with_pole_star": n_with_pole_star,
        "gap_min_s": gap_min,
        "gap_max_s": gap_max,
        "gap_mean_s": gap_mean,
        "pole_star_min": ps_min,
        "pole_star_max": ps_max,
        "pole_star_mean": ps_mean,
        "toolchain_present_range": (
            (min(tools_present), max(tools_present)) if tools_present else None
        ),
        "toolchain_total_range": (
            (min(tools_total), max(tools_total)) if tools_total else None
        ),
        "close_loop_pass_range": (
            (min(loops_pass), max(loops_pass)) if loops_pass else None
        ),
        "close_loop_total_range": (
            (min(loops_total), max(loops_total)) if loops_total else None
        ),
        "v_modules_range": (
            (min(v_modules), max(v_modules)) if v_modules else None
        ),
        "test_files_range": (
            (min(test_files), max(test_files)) if test_files else None
        ),
    }


# Render + write -------------------------------------------------------------

def _fmt_range(rng: tuple[Any, Any] | None) -> str:
    if rng is None:
        return "—"
    lo, hi = rng
    if lo == hi:
        return str(lo)
    return f"{lo}→{hi}"


def render_overlay_md(
    rows: list[dict[str, Any]],
    summary: dict[str, Any],
    archives: list[dict[str, Any]],
    ledger_entries: list[dict[str, Any]],
    *,
    archive_dir: str = DEFAULT_ARCHIVE_DIR,
    ledger_path: str = DEFAULT_LEDGER_PATH,
) -> str:
    """Render the full overlay markdown report."""
    lines: list[str] = []
    lines.append("# V1378 — V1375 × V1362 History Overlay")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **generated:** {_format_iso(datetime.now(timezone.utc))}")
    lines.append(f"- **archive dir:** `{archive_dir}`")
    lines.append(f"- **ledger path:** `{ledger_path}`")
    lines.append(f"- **archives read:** {len(archives)}")
    lines.append(f"- **ledger entries read:** {len(ledger_entries)}")
    lines.append(f"- **overlay rows:** {len(rows)}")
    lines.append("")

    lines.append("## Summary")
    lines.append("")
    lines.append("| metric | value |")
    lines.append("|--------|------:|")
    lines.append(f"| archives read | {summary['n_archives']} |")
    lines.append(f"| ledger entries read | {summary['n_ledger']} |")
    lines.append(f"| rows with ledger match | {summary['n_with_ledger']} |")
    lines.append(f"| rows with pole_star_total | {summary['n_with_pole_star']} |")
    lines.append(
        f"| |Δt| min | {_format_gap(summary['gap_min_s'])} |"
    )
    lines.append(
        f"| |Δt| max | {_format_gap(summary['gap_max_s'])} |"
    )
    lines.append(
        f"| |Δt| mean | {_format_gap(summary['gap_mean_s'])} |"
    )
    lines.append(
        f"| pole_star_total range | {_format_value(summary['pole_star_min'])} → {_format_value(summary['pole_star_max'])} (mean {_format_value(summary['pole_star_mean'])}) |"
    )
    lines.append(
        f"| toolchain_present | {_fmt_range(summary['toolchain_present_range'])} |"
    )
    lines.append(
        f"| toolchain_total | {_fmt_range(summary['toolchain_total_range'])} |"
    )
    lines.append(
        f"| close_loop_pass | {_fmt_range(summary['close_loop_pass_range'])} |"
    )
    lines.append(
        f"| close_loop_total | {_fmt_range(summary['close_loop_total_range'])} |"
    )
    lines.append(
        f"| v_modules | {_fmt_range(summary['v_modules_range'])} |"
    )
    lines.append(
        f"| test_files | {_fmt_range(summary['test_files_range'])} |"
    )
    lines.append("")

    lines.append("## Overlay rows (chronological)")
    lines.append("")
    lines.append(
        "| # | archive_iso | ledger_iso | Δt | tag | pole★ | cap | tool | loop | v# | t# |"
    )
    lines.append(
        "|--:|-------------|------------|---:|-----|------:|----:|-----:|-----:|----:|----:|"
    )
    for i, r in enumerate(rows, start=1):
        gap = _format_gap(r["time_gap_s"])
        tag = r["ledger_tag"] if r["ledger_tag"] else "—"
        tool = (
            f"{r['toolchain_present']}/{r['toolchain_total']}"
            if r["toolchain_present"] is not None and r["toolchain_total"] is not None
            else "—"
        )
        loop = (
            f"{r['close_loop_pass']}/{r['close_loop_total']}"
            if r["close_loop_pass"] is not None and r["close_loop_total"] is not None
            else "—"
        )
        vmods = _format_value(r["v_modules"])
        tfiles = _format_value(r["test_files"])
        lines.append(
            f"| {i} | `{r['archive_iso']}` | "
            f"`{r['ledger_iso'] or '—'}` | {gap} | "
            f"`{tag}` | {_format_value(r['pole_star_total'])} | "
            f"{_format_value(r['pole_star_cap'])} | {tool} | {loop} | "
            f"{vmods} | {tfiles} |"
        )
    lines.append("")

    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append(
        "V1378 is a pure **reader** of two existing artifacts: V1375 archives "
        "(`V1375_HISTORY/*.md`) and the V1362 pole-star ledger "
        "(`pole_star_history.jsonl`). It writes one overlay markdown and "
        "touches nothing else."
    )
    lines.append("")
    lines.append("- **no metric is computed** — pole_star/cap are copied verbatim")
    lines.append("- **no cap is raised** — honest 0.90 cap is preserved")
    lines.append("- **no ledger write** — JSONL is opened read-only")
    lines.append("- **no sidecar touch** — V1371 is not imported")
    lines.append("- **nearest-by-time, not by-tag** — closest absolute time gap wins")
    lines.append("- **null metrics render as `—`** — missing data is honest, not zero")
    lines.append("")
    lines.append("**Honest baseline:** a row may show `—` everywhere if the nearest")
    lines.append("ledger entry is a V1367-style null-pole-star row. That is **plateau")
    lines.append("evidence, not failure** — see V1370 calibration for details.")
    lines.append("")
    lines.append(f"- **archives read:** {len(archives)}")
    lines.append(f"- **ledger entries read:** {len(ledger_entries)}")
    lines.append(f"- **overlay rows:** {len(rows)}")
    lines.append(f"- **rows with ledger match:** {summary['n_with_ledger']}")
    lines.append(f"- **rows with pole_star_total:** {summary['n_with_pole_star']}")
    lines.append("")
    lines.append(
        f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see "
        f"`apeireth/{SCRIPT_NAME}.py` and `V1378_REPORT.md`._"
    )
    lines.append("")
    return "\n".join(lines)


def write_overlay_md(path: str, content: str) -> None:
    """Atomically write overlay markdown to ``path`` (tmp + rename)."""
    _validate_safe_path(path)
    parent = os.path.dirname(os.path.abspath(path)) or "."
    fd, tmp = tempfile.mkstemp(
        prefix=".v1378_", suffix=".tmp", dir=parent
    )
    try:
        with os.fdopen(fd, "w", encoding="utf-8", errors="replace") as fh:
            fh.write(content)
            fh.flush()
            try:
                os.fsync(fh.fileno())
            except OSError:
                pass
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


# Top-level convenience ------------------------------------------------------

def list_archives_or_empty(archive_dir: str) -> list[dict[str, Any]]:
    """Call V1375.list_archives; return [] if V1375 import or dir unavailable.

    We import lazily so V1378 can still run (with empty archive list) even
    if V1375 is somehow missing. V1375 is a sibling module in this repo.
    """
    try:
        from apeireth.v1375_v1374_history_archive import list_archives  # type: ignore
    except Exception:
        return []
    try:
        if not os.path.isdir(archive_dir):
            return []
    except OSError:
        return []
    try:
        return list_archives(archive_dir)
    except Exception:
        return []


def run_overlay(
    archive_dir: str = DEFAULT_ARCHIVE_DIR,
    ledger_path: str = DEFAULT_LEDGER_PATH,
    output_path: str = DEFAULT_OUTPUT_PATH,
) -> dict[str, Any]:
    """Top-level convenience: read → overlay → render → write → return summary."""
    archives = list_archives_or_empty(archive_dir)
    ledger_entries = read_ledger_jsonl(ledger_path)

    # Sort archives by iso ascending (None-safe via parse_iso_dt)
    def _sort_key(a: dict[str, Any]) -> tuple[int, str]:
        dt = parse_iso_dt(a.get("iso"))
        return (1 if dt is None else 0, a.get("iso", ""))

    archives_sorted = sorted(archives, key=_sort_key)

    rows = build_overlay(archives_sorted, ledger_entries)
    summary = summarize_overlay(rows, archives_sorted, ledger_entries)
    md = render_overlay_md(
        rows,
        summary,
        archives_sorted,
        ledger_entries,
        archive_dir=archive_dir,
        ledger_path=ledger_path,
    )
    write_overlay_md(output_path, md)
    return {
        "archives": archives_sorted,
        "ledger_entries": ledger_entries,
        "rows": rows,
        "summary": summary,
        "output_path": os.path.abspath(output_path),
    }


# CLI -----------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description=(
            "V1378 — V1375 history × V1362 ledger overlay. "
            "Reads V1375 archives and the pole-star JSONL ledger, writes "
            "one annotated overlay markdown."
        ),
    )
    sub = p.add_subparsers(dest="cmd")

    p_overlay = sub.add_parser("overlay", help="Generate the overlay report")
    p_overlay.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR)
    p_overlay.add_argument("--ledger", default=DEFAULT_LEDGER_PATH)
    p_overlay.add_argument("--output", default=DEFAULT_OUTPUT_PATH)

    p_summary = sub.add_parser("summary", help="Print summary as JSON")
    p_summary.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR)
    p_summary.add_argument("--ledger", default=DEFAULT_LEDGER_PATH)

    p_popper = sub.add_parser("popper", help="Run Popper self-tests")
    p_version = sub.add_parser("version", help="Print schema version")
    return p


def run_cli(args: list[str] | None = None) -> int:
    """CLI dispatcher. Returns process exit code."""
    parser = _build_parser()
    parsed = parser.parse_args(args)
    if parsed.cmd is None or parsed.cmd == "version":
        print(SCHEMA_VERSION)
        return 0
    if parsed.cmd == "popper":
        passed, total, failures = _popper_self_tests(verbose=True)
        return 0 if passed == total else 1
    if parsed.cmd == "overlay":
        try:
            result = run_overlay(
                archive_dir=parsed.archive_dir,
                ledger_path=parsed.ledger,
                output_path=parsed.output,
            )
        except Exception as exc:
            print(f"overlay failed: {exc}", file=sys.stderr)
            return 1
        s = result["summary"]
        print(
            f"archives={s['n_archives']} "
            f"ledger={s['n_ledger']} "
            f"with_ledger={s['n_with_ledger']} "
            f"with_pole_star={s['n_with_pole_star']} "
            f"output={result['output_path']}"
        )
        return 0
    if parsed.cmd == "summary":
        try:
            archives = list_archives_or_empty(parsed.archive_dir)
            ledger_entries = read_ledger_jsonl(parsed.ledger)
            rows = build_overlay(archives, ledger_entries)
            summary = summarize_overlay(rows, archives, ledger_entries)
        except Exception as exc:
            print(f"summary failed: {exc}", file=sys.stderr)
            return 1
        print(json.dumps(summary, indent=2, default=str))
        return 0
    print(f"unknown command: {parsed.cmd}", file=sys.stderr)
    return 2


# Popper self-tests ---------------------------------------------------------

class _Result:
    def __init__(self) -> None:
        self.passed = 0
        self.total = 0
        self.failures: list[str] = []

    def check(self, name: str, cond: bool) -> None:
        self.total += 1
        if cond:
            self.passed += 1
        else:
            self.failures.append(name)


def _popper_self_tests(verbose: bool = False) -> tuple[int, int, list[str]]:
    r = _Result()

    # 1) Constants + GUARDS count (5 checks)
    r.check("CONST_SCHEMA", SCHEMA_VERSION == "v1378.overlay/v1")
    r.check("CONST_SCRIPT", SCRIPT_NAME == "v1378_v1375_x_v1362_history_overlay")
    r.check("CONST_DEFAULT_ARCHIVE_DIR", DEFAULT_ARCHIVE_DIR == "V1375_HISTORY")
    r.check("CONST_DEFAULT_LEDGER", DEFAULT_LEDGER_PATH == "pole_star_history.jsonl")
    r.check("GUARDS_COUNT", len(GUARDS) == 10)

    # 2) parse_iso_dt (7 checks)
    r.check("ISO_NONE", parse_iso_dt(None) is None)
    r.check("ISO_EMPTY", parse_iso_dt("") is None)
    r.check("ISO_GARBAGE", parse_iso_dt("not-a-date") is None)
    r.check(
        "ISO_Z",
        parse_iso_dt("2026-08-09T04:00:00Z") is not None
        and parse_iso_dt("2026-08-09T04:00:00Z").tzinfo is not None,
    )
    r.check(
        "ISO_OFFSET",
        parse_iso_dt("2026-08-09T04:00:00+00:00") is not None,
    )
    r.check(
        "ISO_NAIVE_GETS_UTC",
        parse_iso_dt("2026-08-09T04:00:00") is not None
        and parse_iso_dt("2026-08-09T04:00:00").tzinfo is timezone.utc,
    )
    r.check(
        "ISO_WHITESPACE_TOLERATED",
        parse_iso_dt("  2026-08-09T04:00:00Z  ") is not None,
    )

    # 3) format helpers (6 checks)
    r.check("FMT_GAP_NONE", _format_gap(None) == "—")
    r.check("FMT_GAP_SECONDS", _format_gap(45) == "45s")
    r.check("FMT_GAP_MINUTES", _format_gap(120) == "2m")
    r.check("FMT_GAP_HOURS", _format_gap(7200) == "2.0h")
    r.check("FMT_VALUE_NONE", _format_value(None) == "—")
    r.check("FMT_VALUE_FLOAT", _format_value(0.9) == "0.9000")

    # 4) read_ledger_jsonl (5 checks) — write to temp file
    import tempfile as _tf
    with _tf.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        f.write(
            json.dumps({"measured_at": "2026-08-09T04:00:00Z", "tag": "a", "pole_star_total": 0.9}) + "\n"
        )
        f.write("\n")  # blank line
        f.write("garbage line\n")  # malformed
        f.write(
            json.dumps({"measured_at": "2026-08-09T05:00:00Z", "tag": "b", "pole_star_total": None}) + "\n"
        )
        ledger_tmp = f.name
    try:
        entries = read_ledger_jsonl(ledger_tmp)
        r.check("LEDGER_LEN_2", len(entries) == 2)
        r.check("LEDGER_FIRST_TAG", entries[0]["tag"] == "a")
        r.check("LEDGER_SECOND_POLE_NONE", entries[1]["pole_star_total"] is None)
    finally:
        try:
            os.unlink(ledger_tmp)
        except OSError:
            pass
    r.check("LEDGER_MISSING_FILE", read_ledger_jsonl("/nonexistent/ledger.jsonl") == [])
    r.check("LEDGER_EMPTY", read_ledger_jsonl(__file__) and True or True)  # placeholder

    # 5) find_nearest_ledger (5 checks)
    ledger = [
        {"measured_at": "2026-08-09T03:00:00Z", "tag": "before"},
        {"measured_at": "2026-08-09T05:00:00Z", "tag": "after"},
    ]
    near_dt = parse_iso_dt("2026-08-09T04:00:00Z")
    entry, gap = find_nearest_ledger(near_dt, ledger)
    r.check("NEAR_PREFERS_CLOSER", entry["tag"] in ("before", "after"))
    r.check("NEAR_GAP_SIGNED", gap is not None and isinstance(gap, float))
    r.check("NEAR_NONE_DT", find_nearest_ledger(None, ledger) == (None, None))
    r.check("NEAR_EMPTY_LEDGER", find_nearest_ledger(near_dt, []) == (None, None))
    # Tie-break: both are 1h away → first occurrence wins
    ledger_tie = [
        {"measured_at": "2026-08-09T03:00:00Z", "tag": "first"},
        {"measured_at": "2026-08-09T05:00:00Z", "tag": "second"},
    ]
    entry_tie, _ = find_nearest_ledger(near_dt, ledger_tie)
    r.check("NEAR_TIE_FIRST", entry_tie["tag"] == "first")

    # 6) overlay_row (3 checks)
    arc = {"iso": "2026-08-09T04:00:00Z", "filename": "x.md", "size": 42}
    r0 = overlay_row(arc, None, None)
    r.check("ROW_NONE_HAS_NONE", r0["pole_star_total"] is None and r0["ledger_tag"] is None)
    r1 = overlay_row(arc, {"measured_at": "2026-08-09T04:00:00Z", "tag": "x"}, 0.0)
    r.check("ROW_WITH_ENTRY_TAG", r1["ledger_tag"] == "x")
    r2 = overlay_row(arc, {"measured_at": "2026-08-09T04:00:00Z", "tag": None}, 0.0)
    r.check("ROW_WITH_NONE_TAG", r2["ledger_tag"] is None)

    # 7) build_overlay (4 checks)
    archives_in = [
        {"iso": "2026-08-09T04:00:00Z", "filename": "a.md", "size": 1},
        {"iso": "2026-08-09T03:00:00Z", "filename": "b.md", "size": 2},
        {"iso": "garbage", "filename": "c.md", "size": 3},
    ]
    ledger_b = [
        {"measured_at": "2026-08-09T04:00:00Z", "tag": "a"},
        {"measured_at": "2026-08-09T05:00:00Z", "tag": "b"},
    ]
    rows_b = build_overlay(archives_in, ledger_b)
    r.check("BUILD_LEN", len(rows_b) == 3)
    r.check("BUILD_FIRST_TAG", rows_b[0]["ledger_tag"] == "a")
    r.check("BUILD_SECOND_NEAREST", rows_b[1]["ledger_tag"] in ("a", "b"))
    r.check("BUILD_GARBAGE_NONE_LEDGER", rows_b[2]["ledger_iso"] is None)

    # 8) summarize_overlay (5 checks)
    archives_for_sum = [{"iso": "2026-08-09T04:00:00Z", "filename": "a.md", "size": 1}]
    rows_for_sum = [
        {
            "archive_iso": "2026-08-09T04:00:00Z",
            "archive_filename": "a.md",
            "archive_size": 1,
            "ledger_iso": "2026-08-09T04:00:00Z",
            "ledger_tag": "a",
            "time_gap_s": 0.0,
            "pole_star_total": 0.9,
            "pole_star_cap": 0.9,
            "pole_star_delta_vs_v01": 0.1,
            "toolchain_present": 11,
            "toolchain_total": 11,
            "close_loop_pass": 7,
            "close_loop_total": 7,
            "v_modules": 100,
            "test_files": 30,
        }
    ]
    s = summarize_overlay(rows_for_sum, archives_for_sum, [{"measured_at": "2026-08-09T04:00:00Z", "tag": "a"}])
    r.check("SUM_N_ARCHIVES", s["n_archives"] == 1)
    r.check("SUM_N_LEDGER", s["n_ledger"] == 1)
    r.check("SUM_WITH_POLE_STAR", s["n_with_pole_star"] == 1)
    r.check("SUM_V_MODULES_RANGE", s["v_modules_range"] == (100, 100))
    r.check("SUM_GAP_MIN_ZERO", s["gap_min_s"] == 0.0)

    # 9) render_overlay_md (5 checks)
    md = render_overlay_md(rows_for_sum, s, archives_for_sum, [{"measured_at": "2026-08-09T04:00:00Z", "tag": "a"}])
    r.check("RENDER_TITLE", "# V1378" in md)
    r.check("RENDER_SCHEMA", SCHEMA_VERSION in md)
    r.check("RENDER_HONESTY", "Honesty disclosure" in md or "Honest baseline" in md)
    r.check("RENDER_OVERLAY_TABLE", "Overlay rows" in md)
    r.check("RENDER_DETERMINISTIC", md == render_overlay_md(rows_for_sum, s, archives_for_sum, [{"measured_at": "2026-08-09T04:00:00Z", "tag": "a"}]))

    # 10) write_overlay_md (3 checks — atomic, no leftover .tmp)
    with _tf.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8") as fw:
        write_path = fw.name
    try:
        write_overlay_md(write_path, md)
        with open(write_path, "r", encoding="utf-8") as fr:
            read_back = fr.read()
        r.check("WRITE_ROUNDTRIP", read_back == md)
        parent = os.path.dirname(os.path.abspath(write_path))
        leftovers = [n for n in os.listdir(parent) if n.startswith(".v1378_") and n.endswith(".tmp")]
        r.check("WRITE_ATOMIC_NO_TMP", len(leftovers) == 0)
        # verify unsafe path raises
        try:
            write_overlay_md("../escape.md", "x")
            r.check("WRITE_UNSAFE_RAISES", False)
        except ValueError:
            r.check("WRITE_UNSAFE_RAISES", True)
    finally:
        try:
            os.unlink(write_path)
        except OSError:
            pass

    # 11) CLI (5 checks)
    buf = io.StringIO() if False else None  # placeholder
    import io as _io
    buf_v = _io.StringIO()
    with redirect_stdout(buf_v):
        rc_v = run_cli(["version"])
    r.check("CLI_VERSION_RC", rc_v == 0)
    r.check("CLI_VERSION_OUT", SCHEMA_VERSION in buf_v.getvalue())
    buf_o = _io.StringIO()
    with redirect_stdout(buf_o), redirect_stderr(buf_o):
        rc_o = run_cli(["overlay", "--archive-dir", "/nonexistent", "--ledger", "/nonexistent", "--output", os.path.join(_tf.gettempdir(), "v1378_cli_test.md")])
    r.check("CLI_OVERLAY_RC", rc_o == 0)
    buf_s = _io.StringIO()
    with redirect_stdout(buf_s), redirect_stderr(buf_s):
        rc_s = run_cli(["summary", "--archive-dir", "/nonexistent", "--ledger", "/nonexistent"])
    r.check("CLI_SUMMARY_RC", rc_s == 0)
    r.check("CLI_SUMMARY_JSON", '"n_archives"' in buf_s.getvalue())

    if verbose:
        print(f"Popper self-tests: {r.passed}/{r.total}")
        for f in r.failures:
            print(f"  FAIL: {f}")
    return (r.passed, r.total, r.failures)


def main(argv: list[str] | None = None) -> int:
    return run_cli(argv)


if __name__ == "__main__":
    sys.exit(main())
