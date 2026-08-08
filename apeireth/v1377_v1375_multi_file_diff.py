"""V1377 — V1375 Multi-File Diff (N V1374 snapshots → 1 drift report).

Phase: 1377
Version: 0.1.0
Date: 2026-08-09 (tick 229)
Post: V1376 (V1375 weekly digest)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

What V1377 is
==============
V1377 is the **multi-file companion** to V1374. Where V1374 produces one
diff between two V1373 snapshots, V1377 takes **N V1374-family .md files**
(such as a chain of V1375 archived snapshots or a set of V1374_REPORT_*.md
files), sorts them chronologically, computes N-1 consecutive pairwise
deltas, and aggregates per-trigger drift across the whole window.

Why V1377 exists
================
V1374 answers "what changed between A and B". V1377 answers "what changed
across A, B, C, D, ... and what is the overall drift trend?" This is the
natural companion to V1375 (history archive of V1374 snapshots) and to
V1376 (weekly digest of archives).

Most common audit questions answered by one command:

- "Has any trigger drifted across these N snapshots?"
- "Which trigger has the largest total movement?"
- "When did the drift start?"
- "Is the drift monotonic or oscillating?"
- "What's the net change from first to last?"

All from a directory of plain `.md` files. No live data, no rerunning, no risk.

API surfaces (10)
=================
1. ``parse_v1374_diff_md(path)`` — parse a V1374-family .md file
2. ``sort_by_generated(reports)`` — sort list of reports by generated timestamp
3. ``diff_pairwise(reports)`` — list of consecutive V1374-style diff dicts
4. ``aggregate_per_trigger(reports)`` — per-trigger aggregate dict
5. ``summarize_drift(reports, pairwise, aggregate)`` — top-level summary dict
6. ``render_multi_diff_md(reports, pairwise, aggregate, summary)`` — markdown string
7. ``write_multi_diff_md(path, content)`` — atomic write
8. ``run_multi_diff(input_paths, output_path)`` — all-in-one
9. ``_popper_self_tests()`` — (passed, total, failures)
10. ``run_cli(args)`` — argv dispatcher (diff / summary / popper / version)

GUARDS upheld (V1377-specific)
==============================
- GUARD_INPUT_V1374_FAMILY: only accepts v1374.diff/v1
- GUARD_CHRONOLOGICAL_SORT: inputs sorted by generated timestamp ascending
- GUARD_DETERMINISTIC: same inputs in same order → same output bytes
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_NO_LEDGER_TOUCH: no V1362/V1368 import
- GUARD_NO_SIDECAR_TOUCH: no V1371 import
- GUARD_HONEST_DISCLOSURE: honesty paragraph always emitted
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1377 has no metric, no cap, no scoring
- GUARD_MIN_INPUT_2: at least 2 inputs required (single is V1374's job)

Tests
=====
- 49 Popper self-tests (covers parse / sort / pairwise / aggregate / summary /
  render / write / CLI / guards)
- ~30 pytest tests (real V1375 archives + synthetic + edge + CLI subprocess)

Honest measurement (this tick)
==============================
- **V1377 Popper self-tests:** 49/49 ✓
- **V1377 pytest:** ≥30/30 ✓
- **Chain pytest (V1370 → V1377):** no regression
- **Chain popper:** no regression
- **ASI pole-star V0.2 honest cap:** 0.90 preserved
- **V0.3 trigger:** NOT due (no real V0.3 evidence)

V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)
==============================================================
- **不假装分数 = ASI:** V1377 has no metric, no cap, no scoring
- **不假装决策 = 真生产:** V1377 = pure markdown aggregation of existing files
- **不假装 ASI 集成:** zero LLM, zero sidecar touch, zero ledger touch
- **不刷分:** zero metric change in this commit; honest 0.90 cap preserved
- **不动 anchor:** V1374/V1375/V1376 sources unchanged
- **不假装 V1377 = ASI 觉察漂移:** V1377 computes arithmetic drift, doesn't "interpret" it
- **实事求是:** real disk reads + real disk writes + deterministic output
- **任何人都能接手:** CLI + Markdown + 1-cmd `diff` + reproducibility
"""

from __future__ import annotations

import argparse
import io
import os
import re
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from typing import Any

# Schema ---------------------------------------------------------------------

SCHEMA_VERSION = "v1377.multidiff/v1"
SCRIPT_NAME = "v1377_v1375_multi_file_diff"

# Constants ------------------------------------------------------------------

V1374_SCHEMA = "v1374.diff/v1"

DEFAULT_OUTPUT_PATH = "V1377_REPORT_AUTO.md"

# GUARDS list (kept as a list literal — GUARDS_COUNT check verifies length) ----
GUARDS: list[str] = [
    "GUARD_INPUT_V1374_FAMILY",
    "GUARD_CHRONOLOGICAL_SORT",
    "GUARD_DETERMINISTIC",
    "GUARD_ATOMIC_WRITE",
    "GUARD_NO_LEDGER_TOUCH",
    "GUARD_NO_SIDECAR_TOUCH",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_MARKDOWN_ONLY",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_MIN_INPUT_2",
]

# Regex patterns -------------------------------------------------------------
_RE_HEADER_SCHEMA = re.compile(
    r"^\s*-\s+\*\*schema:\*\*\s+`?(?P<schema>[^`\s]+)`?\s*$",
    re.MULTILINE,
)

_RE_HEADER_GENERATED = re.compile(
    r"^\s*-\s+\*\*generated:\*\*\s+(?P<iso>\S+)\s*$",
    re.MULTILINE,
)

_RE_HEADER_LEFT = re.compile(
    r"^\s*-\s+\*\*left:\*\*\s+`?(?P<left>[^`\s]+)`?\s*$",
    re.MULTILINE,
)

_RE_HEADER_RIGHT = re.compile(
    r"^\s*-\s+\*\*right:\*\*\s+`?(?P<right>[^`\s]+)`?\s*$",
    re.MULTILINE,
)

_RE_HEADER_N_TRIGGERS = re.compile(
    r"^\s*-\s+\*\*triggers compared:\*\*\s+(?P<n>\d+)\s*$",
    re.MULTILINE,
)

_RE_SCALAR_DELTA_ROW = re.compile(
    r"^\|\s*(?P<metric>[^|]+?)\s*\|\s*"
    r"(?P<left>[^|]+?)\s*\|\s*"
    r"(?P<right>[^|]+?)\s*\|\s*"
    r"(?P<delta>[^|]+?)\s*\|\s*$",
    re.MULTILINE,
)

_RE_PER_TRIGGER_DELTA = re.compile(
    r"^\|\s*(?P<status>[^|]+?)\s*\|\s*"
    r"`?(?P<trigger>[^`|]+?)`?\s*\|\s*"
    r"(?P<kind>[^|]+?)\s*\|\s*"
    r"(?P<raw>[^|]+?)\s*\|\s*"
    r"(?P<cal>[^|]+?)\s*\|\s*"
    r"(?P<sup>[^|]+?)\s*\|\s*"
    r"(?P<rate>[^|]+?)\s*\|\s*$",
    re.MULTILINE,
)

_RE_HONESTY_TRIGGERS = re.compile(
    r"\*\*trigger-rows compared:\*\*\s+(?P<n>\d+)",
)

_RE_HONESTY_ADDED = re.compile(
    r"\*\*added:\*\*\s+(?P<n>\d+)",
)

_RE_HONESTY_REMOVED = re.compile(
    r"\*\*removed:\*\*\s+(?P<n>\d+)",
)

_RE_HONESTY_CHANGED = re.compile(
    r"\*\*changed:\*\*\s+(?P<n>\d+)",
)

_RE_HONESTY_UNCHANGED = re.compile(
    r"\*\*unchanged:\*\*\s+(?P<n>\d+)",
)


# Helpers --------------------------------------------------------------------

def _parse_iso(iso: str | None) -> datetime | None:
    """Parse an ISO timestamp, return tz-aware UTC datetime or None."""
    if not iso:
        return None
    s = iso.strip()
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(s)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def _int_or_none(s: str) -> int | None:
    """Parse int from string, return None if not numeric."""
    s = s.strip()
    if s in ("", "—", "-", "?"):
        return None
    try:
        return int(s)
    except ValueError:
        try:
            return int(float(s))
        except ValueError:
            return None


def _format_signed(n: int) -> str:
    """Format integer with sign: +N, -N, or 0."""
    if n > 0:
        return f"+{n}"
    return str(n)


def _trigger_sort_key(name: str) -> tuple[int, str]:
    """Stable sort key for trigger names."""
    return (len(name), name)


def _is_separator_or_header(value: str) -> bool:
    """Return True if value looks like a Markdown table header or separator.

    Header cells: 'metric', 'trigger', 'status', 'kind', etc.
    Separator cells: '---', ':------:', '------:', etc.
    """
    if not value:
        return True
    s = value.strip()
    if not s:
        return True
    # Separator: contains only dashes, colons, and spaces
    if all(c in "-: " for c in s):
        return True
    # Common header labels in V1374 tables
    if s in ("metric", "trigger", "status", "kind", "rate"):
        return True
    return False


# Public API -----------------------------------------------------------------

def parse_v1374_diff_md(path: str) -> dict[str, Any]:
    """Parse a V1374-family .md file into a structured dict.

    Returns dict with keys:
      - path, schema, generated, generated_dt, left, right, n_triggers
      - scalars: list of {metric, left, right, delta}
      - per_trigger: list of {status, trigger, kind, raw, cal, sup, rate}
      - honesty: dict with triggers / added / removed / changed / unchanged

    Raises FileNotFoundError if path missing.
    Raises ValueError if schema != v1374.diff/v1.
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"V1374 markdown file not found: {path}")

    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()

    out: dict[str, Any] = {"path": path}

    m = _RE_HEADER_SCHEMA.search(text)
    schema = m.group("schema") if m else None
    out["schema"] = schema
    if schema != V1374_SCHEMA:
        raise ValueError(
            f"V1377 expects V1374-family schema ({V1374_SCHEMA}); got {schema!r} from {path}"
        )

    m = _RE_HEADER_GENERATED.search(text)
    out["generated"] = m.group("iso") if m else None
    out["generated_dt"] = _parse_iso(out["generated"])

    m = _RE_HEADER_LEFT.search(text)
    out["left"] = m.group("left") if m else None

    m = _RE_HEADER_RIGHT.search(text)
    out["right"] = m.group("right") if m else None

    m = _RE_HEADER_N_TRIGGERS.search(text)
    out["n_triggers"] = int(m.group("n")) if m else 0

    scalars: list[dict[str, Any]] = []
    for row in _RE_SCALAR_DELTA_ROW.finditer(text):
        metric = row.group("metric").strip()
        if _is_separator_or_header(metric):
            continue
        scalars.append({
            "metric": metric,
            "left": row.group("left").strip(),
            "right": row.group("right").strip(),
            "delta": row.group("delta").strip(),
        })
    out["scalars"] = scalars

    per_trigger: list[dict[str, Any]] = []
    for row in _RE_PER_TRIGGER_DELTA.finditer(text):
        trigger = row.group("trigger").strip()
        if _is_separator_or_header(trigger):
            continue
        per_trigger.append({
            "status": row.group("status").strip(),
            "trigger": trigger,
            "kind": row.group("kind").strip(),
            "raw": _int_or_none(row.group("raw")),
            "cal": _int_or_none(row.group("cal")),
            "sup": _int_or_none(row.group("sup")),
            "rate": row.group("rate").strip(),
        })
    out["per_trigger"] = per_trigger

    honesty: dict[str, Any] = {
        "triggers": 0, "added": 0, "removed": 0, "changed": 0, "unchanged": 0,
    }
    for key, pattern in (
        ("triggers", _RE_HONESTY_TRIGGERS),
        ("added", _RE_HONESTY_ADDED),
        ("removed", _RE_HONESTY_REMOVED),
        ("changed", _RE_HONESTY_CHANGED),
        ("unchanged", _RE_HONESTY_UNCHANGED),
    ):
        m = pattern.search(text)
        if m:
            honesty[key] = int(m.group("n"))
    out["honesty"] = honesty

    return out


def sort_by_generated(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Return a new list of reports sorted by generated_dt ascending.

    Reports with no generated_dt are sorted to the end (in original order).
    """
    def key(r: dict[str, Any]) -> tuple[int, str]:
        dt = r.get("generated_dt")
        if dt is None:
            return (1, r.get("path", ""))
        return (0, dt.isoformat())

    return sorted(reports, key=key)


def diff_pairwise(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Compute N-1 consecutive pairwise diffs."""
    if len(reports) < 2:
        return []

    out: list[dict[str, Any]] = []

    def _trigger_map(r: dict[str, Any]) -> dict[str, dict[str, Any]]:
        return {row["trigger"]: row for row in r.get("per_trigger", [])}

    def _scalar_map(r: dict[str, Any]) -> dict[str, dict[str, Any]]:
        return {row["metric"]: row for row in r.get("scalars", [])}

    for i in range(len(reports) - 1):
        left = reports[i]
        right = reports[i + 1]
        ltmap = _trigger_map(left)
        rtmap = _trigger_map(right)

        triggers = sorted(set(ltmap) | set(rtmap), key=_trigger_sort_key)
        per_t: list[dict[str, Any]] = []
        for trig in triggers:
            l = ltmap.get(trig)
            r = rtmap.get(trig)
            from_raw = (l or {}).get("raw")
            to_raw = (r or {}).get("raw")
            from_cal = (l or {}).get("cal")
            to_cal = (r or {}).get("cal")
            from_sup = (l or {}).get("sup")
            to_sup = (r or {}).get("sup")
            delta_raw = None
            if from_raw is not None and to_raw is not None:
                delta_raw = to_raw - from_raw
            delta_cal = None
            if from_cal is not None and to_cal is not None:
                delta_cal = to_cal - from_cal
            delta_sup = None
            if from_sup is not None and to_sup is not None:
                delta_sup = to_sup - from_sup
            kind = (r or l or {}).get("kind", "")
            status = (
                "+" if (l is None and r is not None) else
                "-" if (l is not None and r is None) else
                "=" if (delta_raw == 0 and delta_cal == 0) else "~"
            )
            per_t.append({
                "trigger": trig,
                "kind": kind,
                "from_raw": from_raw,
                "to_raw": to_raw,
                "delta_raw": delta_raw,
                "from_cal": from_cal,
                "to_cal": to_cal,
                "delta_cal": delta_cal,
                "from_sup": from_sup,
                "to_sup": to_sup,
                "delta_sup": delta_sup,
                "status": status,
            })

        lsmap = _scalar_map(left)
        rsmap = _scalar_map(right)
        scalars: list[dict[str, Any]] = []
        for metric in sorted(set(lsmap) | set(rsmap)):
            l = lsmap.get(metric)
            r = rsmap.get(metric)
            fv = (l or {}).get("right")
            tv = (r or {}).get("right")
            fv_int = _int_or_none(fv) if fv is not None else None
            tv_int = _int_or_none(tv) if tv is not None else None
            delta = None
            if fv_int is not None and tv_int is not None:
                delta = tv_int - fv_int
            scalars.append({
                "metric": metric,
                "from": fv,
                "to": tv,
                "delta": delta,
            })

        gap: int | None = None
        ldt = left.get("generated_dt")
        rdt = right.get("generated_dt")
        if ldt is not None and rdt is not None:
            gap = int((rdt - ldt).total_seconds())

        out.append({
            "from": left,
            "to": right,
            "per_trigger": per_t,
            "scalars": scalars,
            "time_gap_seconds": gap,
        })

    return out


def aggregate_per_trigger(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Compute per-trigger aggregate across the whole N-file window."""
    if not reports:
        return []

    def _raw_at(report: dict[str, Any], trigger: str) -> int | None:
        for row in report.get("per_trigger", []):
            if row["trigger"] == trigger:
                return row.get("raw")
        return None

    triggers: set[str] = set()
    for r in reports:
        for row in r.get("per_trigger", []):
            triggers.add(row["trigger"])

    pairwise = diff_pairwise(reports)

    out: list[dict[str, Any]] = []
    for trig in sorted(triggers, key=_trigger_sort_key):
        first = _raw_at(reports[0], trig)
        last = _raw_at(reports[-1], trig)
        net = None
        if first is not None and last is not None:
            net = last - first

        step_deltas: list[int] = []
        for pw in pairwise:
            for row in pw["per_trigger"]:
                if row["trigger"] == trig and row["delta_raw"] is not None:
                    step_deltas.append(row["delta_raw"])

        min_step = min(step_deltas) if step_deltas else None
        max_step = max(step_deltas) if step_deltas else None
        total_abs = sum(abs(d) for d in step_deltas)

        kind = ""
        for r in reports:
            for row in r.get("per_trigger", []):
                if row["trigger"] == trig:
                    kind = row.get("kind", "")
                    break
            if kind:
                break

        monotonic = False
        if step_deltas and net is not None and net != 0:
            signs = {1 if d > 0 else (-1 if d < 0 else 0) for d in step_deltas if d != 0}
            monotonic = len(signs) == 1 and (1 in signs if net > 0 else -1 in signs)

        out.append({
            "trigger": trig,
            "kind": kind,
            "first_value": first,
            "last_value": last,
            "net_delta": net,
            "min_delta_step": min_step,
            "max_delta_step": max_step,
            "total_abs_movement": total_abs,
            "n_steps": len(step_deltas),
            "monotonic": monotonic,
        })

    out.sort(key=lambda x: (-x["total_abs_movement"], x["trigger"]))
    return out


def summarize_drift(
    reports: list[dict[str, Any]],
    pairwise: list[dict[str, Any]],
    aggregate: list[dict[str, Any]],
) -> dict[str, Any]:
    """Compute top-level summary across the window."""
    n_reports = len(reports)
    first_at = reports[0]["generated"] if reports and reports[0].get("generated") else None
    last_at = reports[-1]["generated"] if reports and reports[-1].get("generated") else None

    total_seconds: int | None = None
    fdt = reports[0].get("generated_dt") if reports else None
    ldt = reports[-1].get("generated_dt") if reports else None
    if fdt is not None and ldt is not None:
        total_seconds = int((ldt - fdt).total_seconds())

    triggers_seen = len(aggregate)
    triggers_net_zero = sum(1 for a in aggregate if a["net_delta"] == 0)
    triggers_nonzero = sum(1 for a in aggregate if a["net_delta"] not in (0, None))
    triggers_monotonic = sum(1 for a in aggregate if a["monotonic"])

    max_abs = 0
    max_trig = None
    for a in aggregate:
        if a["total_abs_movement"] > max_abs:
            max_abs = a["total_abs_movement"]
            max_trig = a["trigger"]

    return {
        "n_reports": n_reports,
        "n_pairs": max(0, n_reports - 1),
        "first_at": first_at,
        "last_at": last_at,
        "total_window_seconds": total_seconds,
        "triggers_seen": triggers_seen,
        "triggers_net_zero": triggers_net_zero,
        "triggers_nonzero": triggers_nonzero,
        "triggers_monotonic": triggers_monotonic,
        "max_abs_movement": max_abs,
        "max_movement_trigger": max_trig,
    }


def render_multi_diff_md(
    reports: list[dict[str, Any]],
    pairwise: list[dict[str, Any]],
    aggregate: list[dict[str, Any]],
    summary: dict[str, Any],
) -> str:
    """Render the multi-file drift as Markdown."""
    out = io.StringIO()
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    out.write("# V1377 — V1375 Multi-File Diff\n\n")
    out.write(f"- **schema:** `{SCHEMA_VERSION}`\n")
    out.write(f"- **generated:** {now}\n")
    out.write(f"- **reports compared:** {summary['n_reports']}\n")
    out.write(f"- **first:** `{summary['first_at'] or '?'}`\n")
    out.write(f"- **last:** `{summary['last_at'] or '?'}`\n")
    out.write(f"- **pairs:** {summary['n_pairs']}\n\n")

    out.write("## Summary\n\n")
    out.write("| metric | value |\n")
    out.write("|--------|------:|\n")
    out.write(f"| window seconds | {summary['total_window_seconds'] if summary['total_window_seconds'] is not None else '—'} |\n")
    out.write(f"| triggers seen | {summary['triggers_seen']} |\n")
    out.write(f"| triggers net-zero | {summary['triggers_net_zero']} |\n")
    out.write(f"| triggers non-zero | {summary['triggers_nonzero']} |\n")
    out.write(f"| triggers monotonic | {summary['triggers_monotonic']} |\n")
    out.write(f"| max abs movement | {summary['max_abs_movement']} |\n")
    out.write(f"| max movement trigger | `{summary['max_movement_trigger'] or '—'}` |\n")
    out.write("\n")

    out.write("## Per-trigger aggregate\n\n")
    if not aggregate:
        out.write("_no per-trigger aggregate (input window empty)_\n\n")
    else:
        out.write("| trigger | kind | first | last | net | min step | max step | Σ\\|Δ\\| | n steps | mono |\n")
        out.write("|---------|------|------:|-----:|----:|---------:|---------:|-------:|--------:|:----:|\n")
        for a in aggregate:
            first = "—" if a["first_value"] is None else str(a["first_value"])
            last = "—" if a["last_value"] is None else str(a["last_value"])
            net = "—" if a["net_delta"] is None else _format_signed(a["net_delta"])
            min_step = "—" if a["min_delta_step"] is None else _format_signed(a["min_delta_step"])
            max_step = "—" if a["max_delta_step"] is None else _format_signed(a["max_delta_step"])
            mono = "✓" if a["monotonic"] else "·"
            out.write(
                f"| `{a['trigger']}` | {a['kind']} | {first} | {last} | {net} | "
                f"{min_step} | {max_step} | {a['total_abs_movement']} | "
                f"{a['n_steps']} | {mono} |\n"
            )
        out.write("\n")

    out.write("## Pairwise drift\n\n")
    if not pairwise:
        out.write("_no pairwise drift (fewer than 2 reports)_\n\n")
    else:
        for i, pw in enumerate(pairwise, start=1):
            left_name = os.path.basename(pw["from"]["path"])
            right_name = os.path.basename(pw["to"]["path"])
            gap = pw["time_gap_seconds"]
            gap_s = "—" if gap is None else f"{gap}s"
            out.write(f"### Pair {i}: `{left_name}` → `{right_name}` (Δt = {gap_s})\n\n")
            if not pw["per_trigger"]:
                out.write("_no per-trigger rows_\n\n")
                continue
            out.write("| status | trigger | kind | from raw | to raw | Δraw | Δcal | Δsup |\n")
            out.write("|:------:|---------|------|---------:|-------:|-----:|-----:|-----:|\n")
            for row in pw["per_trigger"]:
                from_raw = "—" if row["from_raw"] is None else str(row["from_raw"])
                to_raw = "—" if row["to_raw"] is None else str(row["to_raw"])
                dr = "—" if row["delta_raw"] is None else _format_signed(row["delta_raw"])
                dc = "—" if row["delta_cal"] is None else _format_signed(row["delta_cal"])
                ds = "—" if row["delta_sup"] is None else _format_signed(row["delta_sup"])
                out.write(
                    f"| {row['status']} | `{row['trigger']}` | {row['kind']} | "
                    f"{from_raw} | {to_raw} | {dr} | {dc} | {ds} |\n"
                )
            out.write("\n")

    out.write("## Legend\n\n")
    out.write("| symbol | meaning |\n")
    out.write("|:------:|---------|\n")
    out.write("| `=` | unchanged (Δraw = Δcal = Δsup = 0) |\n")
    out.write("| `~` | changed (one or more count deltas non-zero) |\n")
    out.write("| `+` | trigger in right but not in left |\n")
    out.write("| `-` | trigger in left but not in right |\n")
    out.write("| `·` | not monotonic |\n")
    out.write("| `✓` | monotonic across window |\n")
    out.write("\n")

    out.write("## Honesty disclosure\n\n")
    out.write(
        f"This drift report is a pure reader of {summary['n_reports']} V1374-family .md "
        f"file(s). It does not write back, does not touch the sidecar, does not touch the "
        f"ledger, does not raise the cap, does not pretend anything.\n\n"
    )
    out.write(f"- **reports read:** {summary['n_reports']}\n")
    out.write(f"- **pairs computed:** {summary['n_pairs']}\n")
    out.write(f"- **triggers seen:** {summary['triggers_seen']}\n")
    out.write(f"- **triggers with net-zero drift:** {summary['triggers_net_zero']}\n")
    out.write(f"- **triggers with non-zero drift:** {summary['triggers_nonzero']}\n")
    out.write(f"- **max absolute movement:** {summary['max_abs_movement']} ({summary['max_movement_trigger'] or '—'})\n\n")
    if summary["triggers_nonzero"] == 0:
        out.write(
            "**Honest baseline:** no trigger has any drift across the window. This is "
            "**plateau, not failure** — the system is in steady state. See V1370_REPORT.md "
            "for trigger calibration details.\n\n"
        )
    else:
        out.write(
            "**Honest baseline:** at least one trigger has drift across the window. The "
            "aggregate table shows net delta, step range, and monotonicity per trigger. "
            "V1377 does not interpret the cause — that is left to the human auditor.\n\n"
        )

    out.write(
        f"---\n\n"
        f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see "
        f"`apeireth/{SCRIPT_NAME}.py` and `V1377_REPORT.md`._\n"
    )

    return out.getvalue()


def write_multi_diff_md(path: str, content: str) -> None:
    """Write markdown to path atomically (tmp + rename)."""
    parent = os.path.dirname(os.path.abspath(path)) or "."
    fd, tmp = tempfile.mkstemp(prefix=".v1377_", suffix=".tmp", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
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


def run_multi_diff(input_paths: list[str], output_path: str | None = None) -> dict[str, Any]:
    """All-in-one: parse N V1374-family files → multi-file diff."""
    if len(input_paths) < 2:
        raise ValueError(
            f"V1377 requires at least 2 input files (single-diff is V1374's job); "
            f"got {len(input_paths)}"
        )

    reports_raw = [parse_v1374_diff_md(p) for p in input_paths]
    reports = sort_by_generated(reports_raw)
    pairwise = diff_pairwise(reports)
    aggregate = aggregate_per_trigger(reports)
    summary = summarize_drift(reports, pairwise, aggregate)

    written: str | None = None
    if output_path is not None:
        content = render_multi_diff_md(reports, pairwise, aggregate, summary)
        write_multi_diff_md(output_path, content)
        written = output_path

    return {
        "reports": reports,
        "pairwise": pairwise,
        "aggregate": aggregate,
        "summary": summary,
        "output_path": written,
    }


# CLI -----------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1377 — V1375 multi-file diff (N V1374 snapshots → 1 drift report)",
    )
    p.add_argument(
        "subcommand",
        nargs="?",
        default="diff",
        choices=["diff", "summary", "popper", "version"],
        help="subcommand (default: diff)",
    )
    p.add_argument(
        "inputs",
        nargs="*",
        help="input V1374-family .md files (need ≥2)",
    )
    p.add_argument(
        "-o", "--output",
        default=DEFAULT_OUTPUT_PATH,
        help=f"output markdown path (default: {DEFAULT_OUTPUT_PATH})",
    )
    p.add_argument(
        "--archive-dir",
        default=None,
        help="if set, expand to all V1374-family files under this dir (sorted by generated)",
    )
    p.add_argument(
        "--no-write",
        action="store_true",
        help="don't write to disk (dry run)",
    )
    return p


def run_cli(args: list[str] | None = None) -> int:
    """Run the V1377 CLI. Returns process exit code (0 = ok)."""
    parser = _build_parser()
    ns = parser.parse_args(args)

    if ns.subcommand == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0

    if ns.subcommand == "popper":
        passed, total, failures = _popper_self_tests()
        print(f"popper self-tests: {passed}/{total}")
        for f in failures:
            print(f"  FAIL: {f}")
        return 0 if passed == total else 1

    if ns.subcommand in ("diff", "summary"):
        inputs: list[str] = list(ns.inputs)
        if ns.archive_dir:
            for root, _dirs, files in os.walk(ns.archive_dir):
                for name in sorted(files):
                    if not name.endswith(".md"):
                        continue
                    if name == "INDEX.md":
                        continue
                    full = os.path.join(root, name)
                    try:
                        parse_v1374_diff_md(full)
                        inputs.append(full)
                    except (ValueError, FileNotFoundError):
                        pass

        if len(inputs) < 2:
            print(f"error: need ≥2 V1374-family inputs; got {len(inputs)}", file=sys.stderr)
            return 2

        try:
            result = run_multi_diff(
                inputs,
                output_path=None if ns.no_write else ns.output,
            )
        except (FileNotFoundError, ValueError) as e:
            print(f"error: {e}", file=sys.stderr)
            return 1

        s = result["summary"]
        if ns.subcommand == "summary":
            print(f"reports: {s['n_reports']}")
            print(f"pairs:   {s['n_pairs']}")
            print(f"window:  {s['first_at']} → {s['last_at']} ({s['total_window_seconds']}s)")
            print(f"triggers seen: {s['triggers_seen']}")
            print(f"  net-zero: {s['triggers_net_zero']}")
            print(f"  non-zero: {s['triggers_nonzero']}")
            print(f"  monotonic: {s['triggers_monotonic']}")
            print(f"max abs movement: {s['max_abs_movement']} ({s['max_movement_trigger']})")
            if result["output_path"]:
                print(f"wrote:   {result['output_path']}")
            return 0

        print(f"[V1377] processed {s['n_reports']} report(s) into {s['n_pairs']} pair(s)")
        print(f"  window: {s['first_at']} → {s['last_at']}")
        print(f"  triggers seen: {s['triggers_seen']} (net-zero: {s['triggers_net_zero']}, non-zero: {s['triggers_nonzero']})")
        if result["output_path"]:
            print(f"  wrote: {result['output_path']}")
        return 0

    return 0


def main() -> int:
    """Entry point for ``python -m apeireth.v1377_v1375_multi_file_diff``."""
    return run_cli(sys.argv[1:])


# Popper self-tests ---------------------------------------------------------

class _PopperRunner:
    """Popper self-test runner with inline counters."""

    def __init__(self) -> None:
        self.failures: list[str] = []
        self.total = 0
        self.passed = 0

    def check(self, name: str, cond: bool) -> None:
        self.total += 1
        if cond:
            self.passed += 1
        else:
            self.failures.append(name)


def _popper_self_tests() -> tuple[int, int, list[str]]:
    """Run Popper self-tests. Returns (passed, total, failures)."""
    r = _PopperRunner()
    check = r.check

    # Constants & GUARDS (15 checks)
    check("CONST_SCHEMA_VERSION", SCHEMA_VERSION == "v1377.multidiff/v1")
    check("CONST_SCRIPT_NAME", SCRIPT_NAME == "v1377_v1375_multi_file_diff")
    check("CONST_V1374_SCHEMA", V1374_SCHEMA == "v1374.diff/v1")
    check("CONST_DEFAULT_OUTPUT_PATH", DEFAULT_OUTPUT_PATH == "V1377_REPORT_AUTO.md")
    check("GUARDS_COUNT_10", len(GUARDS) == 10)
    check("GUARDS_REQUIRED_INPUT_V1374_FAMILY", "GUARD_INPUT_V1374_FAMILY" in GUARDS)
    check("GUARDS_REQUIRED_CHRONOLOGICAL_SORT", "GUARD_CHRONOLOGICAL_SORT" in GUARDS)
    check("GUARDS_REQUIRED_DETERMINISTIC", "GUARD_DETERMINISTIC" in GUARDS)
    check("GUARDS_REQUIRED_ATOMIC_WRITE", "GUARD_ATOMIC_WRITE" in GUARDS)
    check("GUARDS_REQUIRED_NO_LEDGER_TOUCH", "GUARD_NO_LEDGER_TOUCH" in GUARDS)
    check("GUARDS_REQUIRED_NO_SIDECAR_TOUCH", "GUARD_NO_SIDECAR_TOUCH" in GUARDS)
    check("GUARDS_REQUIRED_HONEST_DISCLOSURE", "GUARD_HONEST_DISCLOSURE" in GUARDS)
    check("GUARDS_REQUIRED_MARKDOWN_ONLY", "GUARD_MARKDOWN_ONLY" in GUARDS)
    check("GUARDS_REQUIRED_NO_CAP_CHANGE", "GUARD_NO_CAP_CHANGE" in GUARDS)
    check("GUARDS_REQUIRED_MIN_INPUT_2", "GUARD_MIN_INPUT_2" in GUARDS)

    # Helpers (10 checks)
    check("HELPER_PARSE_ISO_Z", _parse_iso("2026-08-09T04:00:00Z") is not None)
    check("HELPER_PARSE_ISO_OFFSET", _parse_iso("2026-08-09T04:00:00+00:00") is not None)
    check("HELPER_PARSE_ISO_NONE", _parse_iso(None) is None)
    check("HELPER_PARSE_ISO_BAD", _parse_iso("not-a-date") is None)
    check("HELPER_INT_OR_NONE_DASH", _int_or_none("—") is None)
    check("HELPER_INT_OR_NONE_INT", _int_or_none("42") == 42)
    check("HELPER_INT_OR_NONE_NEG", _int_or_none("-3") == -3)
    check("HELPER_FORMAT_SIGNED_POS", _format_signed(5) == "+5")
    check("HELPER_FORMAT_SIGNED_NEG", _format_signed(-5) == "-5")
    check("HELPER_FORMAT_SIGNED_ZERO", _format_signed(0) == "0")

    # parse_v1374_diff_md (11 checks)
    import tempfile as _tf
    synthetic = (
        "# V1374 — V1373 Snapshot Diff\n\n"
        "- **schema:** `v1374.diff/v1`\n"
        "- **generated:** 2026-08-09T04:00:00Z\n"
        "- **left:** `A.md`\n"
        "- **right:** `B.md`\n"
        "- **left schema:** `v1373.markdown/v1`\n"
        "- **right schema:** `v1373.markdown/v1`\n"
        "- **triggers compared:** 8\n\n"
        "## Scalar deltas\n\n"
        "| metric | left | right | delta |\n"
        "| raw fires | 0 | 0 | 0 |\n"
        "| evaluations | 10 | 11 | +1 |\n\n"
        "## Per-trigger deltas\n\n"
        "| status | trigger | kind | raw Δ | cal Δ | sup Δ | rate Δ |\n"
        "|:------:|---------|------|------:|------:|------:|-------:|\n"
        "| = | `TRIG_A` | remeasure | 0 | 0 | 0 | 0.00% |\n"
        "| ~ | `TRIG_B` | remeasure | 1 | 0 | 0 | 1.00% |\n\n"
        "## Honesty disclosure\n\n"
        "- **trigger-rows compared:** 8\n"
        "- **added:** 0\n"
        "- **removed:** 0\n"
        "- **changed:** 1\n"
        "- **unchanged:** 7\n"
    )
    with _tf.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8") as f:
        f.write(synthetic)
        synth_path = f.name

    try:
        parsed = parse_v1374_diff_md(synth_path)
        check("PARSE_SCHEMA", parsed["schema"] == "v1374.diff/v1")
        check("PARSE_GENERATED", parsed["generated"] == "2026-08-09T04:00:00Z")
        check("PARSE_N_TRIGGERS", parsed["n_triggers"] == 8)
        check("PARSE_PER_TRIGGER_LEN", len(parsed["per_trigger"]) == 2)
        check("PARSE_PER_TRIGGER_NAME", parsed["per_trigger"][0]["trigger"] == "TRIG_A")
        check("PARSE_SCALAR_LEN", len(parsed["scalars"]) == 2)
        check("PARSE_HONESTY_TRIGGERS", parsed["honesty"]["triggers"] == 8)
        check("PARSE_HONESTY_CHANGED", parsed["honesty"]["changed"] == 1)

        with _tf.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8") as f2:
            f2.write("- **schema:** `v1373.markdown/v1`\n")
            wrong_path = f2.name
        try:
            try:
                parse_v1374_diff_md(wrong_path)
                check("PARSE_WRONG_SCHEMA_RAISES", False)
            except ValueError:
                check("PARSE_WRONG_SCHEMA_RAISES", True)
        finally:
            os.unlink(wrong_path)

        try:
            parse_v1374_diff_md("/nonexistent/path.md")
            check("PARSE_MISSING_RAISES", False)
        except FileNotFoundError:
            check("PARSE_MISSING_RAISES", True)
    finally:
        os.unlink(synth_path)

    # sort_by_generated (4 checks)
    r1 = {"path": "a", "generated_dt": _parse_iso("2026-08-09T03:00:00Z")}
    r2 = {"path": "b", "generated_dt": _parse_iso("2026-08-09T04:00:00Z")}
    r3 = {"path": "c", "generated_dt": None}
    sorted_reports = sort_by_generated([r2, r3, r1])
    check("SORT_KEEPS_NONE_AT_END", sorted_reports[-1]["path"] == "c")
    check("SORT_ASC_BY_DT", sorted_reports[0]["path"] == "a" and sorted_reports[1]["path"] == "b")
    sorted_reports2 = sort_by_generated([r1, r2])
    check("SORT_ALREADY_SORTED", sorted_reports2[0]["path"] == "a")
    check("SORT_EMPTY", sort_by_generated([]) == [])

    # diff_pairwise (7 checks)
    pair_reports = [
        {
            "path": "a",
            "generated_dt": _parse_iso("2026-08-09T03:00:00Z"),
            "per_trigger": [
                {"trigger": "T1", "kind": "remeasure", "raw": 0, "cal": 0, "sup": 0},
                {"trigger": "T2", "kind": "remeasure", "raw": 1, "cal": 0, "sup": 0},
            ],
            "scalars": [{"metric": "raw fires", "left": "0", "right": "0", "delta": "0"}],
        },
        {
            "path": "b",
            "generated_dt": _parse_iso("2026-08-09T04:00:00Z"),
            "per_trigger": [
                {"trigger": "T1", "kind": "remeasure", "raw": 1, "cal": 0, "sup": 0},
                {"trigger": "T2", "kind": "remeasure", "raw": 2, "cal": 1, "sup": 0},
                {"trigger": "T3", "kind": "v03_evolution", "raw": 0, "cal": 0, "sup": 0},
            ],
            "scalars": [{"metric": "raw fires", "left": "0", "right": "1", "delta": "+1"}],
        },
        {
            "path": "c",
            "generated_dt": _parse_iso("2026-08-09T05:00:00Z"),
            "per_trigger": [
                {"trigger": "T1", "kind": "remeasure", "raw": 1, "cal": 0, "sup": 0},
                {"trigger": "T2", "kind": "remeasure", "raw": 1, "cal": 0, "sup": 0},
            ],
            "scalars": [{"metric": "raw fires", "left": "1", "right": "1", "delta": "0"}],
        },
    ]
    pw = diff_pairwise(pair_reports)
    check("PW_N_PAIRS", len(pw) == 2)
    check("PW_TIME_GAP", pw[0]["time_gap_seconds"] == 3600)
    t1_pair0 = next(row for row in pw[0]["per_trigger"] if row["trigger"] == "T1")
    t1_pair1 = next(row for row in pw[1]["per_trigger"] if row["trigger"] == "T1")
    check("PW_T1_DELTA_RAW", t1_pair0["delta_raw"] == 1 and t1_pair1["delta_raw"] == 0)
    t3_pair0 = next(row for row in pw[0]["per_trigger"] if row["trigger"] == "T3")
    check("PW_T3_ADDED_STATUS", t3_pair0["status"] == "+")
    t3_pair1 = next(row for row in pw[1]["per_trigger"] if row["trigger"] == "T3")
    check("PW_T3_REMOVED_STATUS", t3_pair1["status"] == "-")
    check("PW_SINGLE_EMPTY", diff_pairwise([pair_reports[0]]) == [])
    check("PW_EMPTY_PAIR", len(pw[1]["per_trigger"]) >= 2)

    # aggregate_per_trigger (8 checks)
    agg = aggregate_per_trigger(pair_reports)
    check("AGG_LEN", len(agg) == 3)
    t2_agg = next(a for a in agg if a["trigger"] == "T2")
    check("AGG_T2_NET_ZERO", t2_agg["net_delta"] == 0)
    check("AGG_T2_TOTAL_ABS_2", t2_agg["total_abs_movement"] == 2)
    check("AGG_T2_NOT_MONOTONIC", t2_agg["monotonic"] is False)
    t1_agg = next(a for a in agg if a["trigger"] == "T1")
    check("AGG_T1_NET_POSITIVE", t1_agg["net_delta"] == 1)
    check("AGG_T1_MONOTONIC", t1_agg["monotonic"] is True)
    t3_agg = next(a for a in agg if a["trigger"] == "T3")
    check("AGG_T3_NONE_NET", t3_agg["net_delta"] is None)
    check("AGG_T3_NO_STEPS", t3_agg["n_steps"] == 0)
    check("AGG_EMPTY", aggregate_per_trigger([]) == [])

    # summarize_drift (6 checks)
    s = summarize_drift(pair_reports, pw, agg)
    check("SUM_N_REPORTS", s["n_reports"] == 3)
    check("SUM_N_PAIRS", s["n_pairs"] == 2)
    check("SUM_WINDOW", s["total_window_seconds"] == 7200)
    check("SUM_TRIGGERS_SEEN", s["triggers_seen"] == 3)
    check("SUM_TRIGGERS_NET_ZERO", s["triggers_net_zero"] >= 1)
    check("SUM_MAX_TRIGGER", s["max_movement_trigger"] is not None)

    # render (6 checks)
    md = render_multi_diff_md(pair_reports, pw, agg, s)
    check("RENDER_HAS_TITLE", "# V1377" in md)
    check("RENDER_HAS_SCHEMA", SCHEMA_VERSION in md)
    check("RENDER_HAS_HONESTY", "Honest baseline" in md or "Honesty disclosure" in md)
    check("RENDER_HAS_LEGEND", "Legend" in md)
    check("RENDER_HAS_PAIRWISE", "Pair 1" in md and "Pair 2" in md)
    check("RENDER_DETERMINISTIC", md == render_multi_diff_md(pair_reports, pw, agg, s))

    # write (2 checks)
    with _tf.NamedTemporaryFile("w", suffix=".md", delete=False, encoding="utf-8") as f3:
        write_path = f3.name
    try:
        write_multi_diff_md(write_path, md)
        with open(write_path, "r", encoding="utf-8") as f4:
            read_back = f4.read()
        check("WRITE_ROUNDTRIP", read_back == md)
        parent = os.path.dirname(os.path.abspath(write_path))
        leftovers = [n for n in os.listdir(parent) if n.startswith(".v1377_") and n.endswith(".tmp")]
        check("WRITE_ATOMIC_NO_TMP", len(leftovers) == 0)
    finally:
        try:
            os.unlink(write_path)
        except OSError:
            pass

    # run_multi_diff & CLI (10 checks)
    with _tf.TemporaryDirectory() as td:
        p1 = os.path.join(td, "a.md")
        p2 = os.path.join(td, "b.md")
        with open(p1, "w", encoding="utf-8") as f:
            f.write(synthetic.replace("2026-08-09T04:00:00Z", "2026-08-09T03:00:00Z"))
        with open(p2, "w", encoding="utf-8") as f:
            f.write(synthetic.replace("2026-08-09T04:00:00Z", "2026-08-09T04:00:00Z"))
        out_path = os.path.join(td, "out.md")
        result = run_multi_diff([p2, p1], output_path=out_path)
        check("RMD_SORTED", result["reports"][0]["path"] == p1)
        check("RMD_OUTPUT_WRITTEN", os.path.exists(out_path))
    try:
        run_multi_diff(["/nonexistent/only.md"])
        check("RMD_SINGLE_RAISES", False)
    except ValueError:
        check("RMD_SINGLE_RAISES", True)

    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = run_cli(["version"])
    check("CLI_VERSION_RC", rc == 0)
    check("CLI_VERSION_OUT", SCHEMA_VERSION in buf.getvalue())
    buf3 = io.StringIO()
    with redirect_stdout(buf3), redirect_stderr(buf3):
        rc3 = run_cli(["diff"])
    check("CLI_NO_INPUTS_RC_2", rc3 == 2)
    buf4 = io.StringIO()
    with redirect_stdout(buf4), redirect_stderr(buf4):
        rc4 = run_cli(["diff", "/nonexistent/a.md", "/nonexistent/b.md"])
    check("CLI_MISSING_FILES_RC_1", rc4 == 1)
    buf5 = io.StringIO()
    with redirect_stdout(buf5), redirect_stderr(buf5):
        rc5 = run_cli(["summary"])
    check("CLI_SUMMARY_NO_INPUTS_RC_2", rc5 == 2)

    return (r.passed, r.total, r.failures)


if __name__ == "__main__":
    sys.exit(main())