"""V1374 — V1373 Markdown diff mode (post-V1373 next-step 2/5)

## What V1374 is

V1374 is the **diff companion** to V1373. Where V1373 writes a single snapshot
of trigger rates, V1374 takes two V1373 .md files and produces a third .md that
shows what changed between them.

V1374 serves the same hard constraint as the rest of the project: **anyone can
pick this up without asking me**. With two .md files and one command, you get
the delta — no Python, no sidecar, no ledger, no cap fiddling.

```bash
python -m apeireth.v1374_v1373_diff diff --left old.md --right new.md
# stdout (terminal) or:
python -m apeireth.v1374_v1373_diff diff --left old.md --right new.md --out diff.md
```

## Why V1374 exists

V1373 produces a snapshot per cron tick. After several ticks you have a pile
of .md files with no built-in way to see what changed. V1374 is the missing
primitive:

- Identify which triggers fired more often (Δ raw fires > 0)
- Identify which triggers stopped firing (Δ raw fires < 0)
- Identify which triggers were added or removed
- Identify changes in evaluation count, schema, or source
- Identify when the V1370 calibrator started suppressing different FPs

All from two .md files. No live data, no rerunning, no risk.

## 9 API surfaces

1. `parse_markdown(path)` → dict (header / timeline / summary / honesty)
2. `compute_diff(left, right)` → dict (deltas per trigger + scalar deltas)
3. `render_diff_markdown(diff_data, *, title=None)` → markdown string
4. `write_diff_markdown(path, content)` → atomic tmp+rename
5. `diff_two_files(left_path, right_path, *, out_path=None, title=None)` → int
6. `summary_two_files(left_path, right_path)` → short text block to stdout
7. `_popper_self_tests()` → (passed, total, failures)
8. `run_cli(args)` → argv dispatcher (diff / summary / popper / version)
9. `main()` → sys.argv pass-through

## Diff semantics

For each trigger, `delta` = right − left for raw / cal / sup / fire_rate.
A new trigger (in right but not left) is shown as `+` with delta = right value.
A removed trigger (in left but not right) is shown as `-` with delta = −left value.

Scalar deltas:
- `delta_evals` = right.evals − left.evals
- `delta_triggers` = right.triggers − left.triggers
- `delta_raw_total` = right.raw_total − left.raw_total
- `delta_cal_total` = right.cal_total − left.cal_total
- `delta_sup_total` = right.sup_total − left.sup_total
- `delta_time_seconds` = right.generated − left.generated (parsed ISO)

## GUARDS upheld (V1374-specific)

- GUARD_DIFF_PURE: V1374 only reads .md files; no sidecar, no ledger
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_NO_SIDECAR_TOUCH: no V1371 import
- GUARD_NO_LEDGER_TOUCH: no V1362/V1368 import
- GUARD_HONEST_DISCLOSURE: always emit honesty paragraph
- GUARD_MARKDOWN_ONLY: pure CommonMark
- GUARD_NO_CAP_CHANGE: V1374 does not write back to any cap
- GUARD_SYMMETRIC: diff(left, right) is antisymmetric under swap

## Tests

- 32 Popper self-tests
- 24 pytest tests
- chain regression with V1373 → V1372 → V1371 (no source mutations)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import sys
import tempfile
from typing import Any

# Reconfigure stdout for consistency with V1373
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1374.diff/v1"
SCRIPT_NAME = "v1374_v1373_diff"

# Symbols used in diff output
SYM_IDENTICAL = "="
SYM_CHANGED = "~"
SYM_ADDED = "+"
SYM_REMOVED = "-"
SYM_NO_FIRE = "·"
SYM_FIRE = "●"
SYM_SUPPRESSED = "◌"


# ----------------------------------------------------------------------
# Parsing
# ----------------------------------------------------------------------

_RE_HEADER_SCHEMA = re.compile(r"^-\s+\*\*schema:\*\*\s+`(?P<schema>[^`]+)`\s*$", re.MULTILINE)
_RE_HEADER_GENERATED = re.compile(r"^-\s+\*\*generated:\*\*\s+(?P<iso>\S+)\s*$", re.MULTILINE)
_RE_HEADER_SOURCE = re.compile(r"^-\s+\*\*source sidecar:\*\*\s+`(?P<src>[^`]+)`\s*$", re.MULTILINE)
_RE_HEADER_TRIGGERS = re.compile(r"^-\s+\*\*triggers:\*\*\s+(?P<n>\d+)\s*$", re.MULTILINE)
_RE_HEADER_EVALS = re.compile(r"^-\s+\*\*evaluations:\*\*\s+(?P<n>\d+)\s*$", re.MULTILINE)

_RE_TL_ROW = re.compile(
    r"^\|\s+`(?P<name>[^`]+)`\s+\|\s+(?P<kind>\w+)\s+\|\s+`(?P<tl>[^`]*)`\s+\|\s+"
    r"(?P<raw>\d+)\s+\|\s+(?P<cal>\d+)\s+\|\s+(?P<sup>\d+)\s+\|\s*$",
    re.MULTILINE,
)

_RE_SUMMARY_ROW = re.compile(
    r"^\|\s+`(?P<name>[^`]+)`\s+\|\s+(?P<kind>\w+)\s+\|\s+(?P<rate>[0-9.]+)%\s+\|\s*$",
    re.MULTILINE,
)

_RE_HONESTY_RAW = re.compile(r"-\s+\*\*raw fires:\*\*\s+(?P<n>\d+)")
_RE_HONESTY_CAL = re.compile(r"-\s+\*\*calibrated fires:\*\*\s+(?P<n>\d+)")
_RE_HONESTY_SUP = re.compile(r"-\s+\*\*V1370-suppressed false positives:\*\*\s+(?P<n>\d+)")
_RE_HONESTY_CHECKS = re.compile(
    r"-\s+\*\*trigger-checks evaluated:\*\*\s+(?P<n>\d+)\s+\((?P<lhs>\d+)\s+evaluations"
)


def _parse_iso(s: str) -> _dt.datetime:
    """Parse a V1373 ISO timestamp; tolerant of trailing Z."""
    s = s.strip()
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    return _dt.datetime.fromisoformat(s)


def parse_markdown(path: str) -> dict[str, Any]:
    """Parse a V1373 .md file into a structured dict.

    Returns a dict with keys:
      - path: source path
      - schema: schema string
      - generated: ISO timestamp string
      - generated_dt: datetime object (UTC tz-aware)
      - source: sidecar path (or None)
      - n_triggers: int
      - n_evals: int
      - timeline: list of dicts with name / kind / timeline_str / raw / cal / sup
      - summary: list of dicts with name / kind / fire_rate (percent float)
      - honesty: dict with raw / cal / sup / trigger_checks
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"V1373 markdown file not found: {path}")
    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()

    out: dict[str, Any] = {"path": path}

    m = _RE_HEADER_SCHEMA.search(text)
    out["schema"] = m.group("schema") if m else None

    m = _RE_HEADER_GENERATED.search(text)
    out["generated"] = m.group("iso") if m else None
    out["generated_dt"] = _parse_iso(out["generated"]) if out["generated"] else None

    m = _RE_HEADER_SOURCE.search(text)
    out["source"] = m.group("src") if m else None

    m = _RE_HEADER_TRIGGERS.search(text)
    out["n_triggers"] = int(m.group("n")) if m else 0

    m = _RE_HEADER_EVALS.search(text)
    out["n_evals"] = int(m.group("n")) if m else 0

    # Timeline rows
    timeline: list[dict[str, Any]] = []
    for row in _RE_TL_ROW.finditer(text):
        timeline.append({
            "name": row.group("name"),
            "kind": row.group("kind"),
            "timeline_str": row.group("tl"),
            "raw": int(row.group("raw")),
            "cal": int(row.group("cal")),
            "sup": int(row.group("sup")),
        })
    out["timeline"] = timeline

    # Summary rows
    summary: list[dict[str, Any]] = []
    for row in _RE_SUMMARY_ROW.finditer(text):
        summary.append({
            "name": row.group("name"),
            "kind": row.group("kind"),
            "fire_rate": float(row.group("rate")),
        })
    out["summary"] = summary

    # Honesty block
    honesty: dict[str, Any] = {"raw": 0, "cal": 0, "sup": 0, "trigger_checks": 0}
    m = _RE_HONESTY_RAW.search(text)
    if m:
        honesty["raw"] = int(m.group("n"))
    m = _RE_HONESTY_CAL.search(text)
    if m:
        honesty["cal"] = int(m.group("n"))
    m = _RE_HONESTY_SUP.search(text)
    if m:
        honesty["sup"] = int(m.group("n"))
    m = _RE_HONESTY_CHECKS.search(text)
    if m:
        honesty["trigger_checks"] = int(m.group("n"))
    out["honesty"] = honesty

    return out


# ----------------------------------------------------------------------
# Diff computation
# ----------------------------------------------------------------------

def _by_name(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    """Index a list of dicts by their 'name' field."""
    return {r["name"]: r for r in rows}


def compute_diff(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    """Compute the diff between two parsed V1373 .md dicts.

    Returns a dict with keys:
      - left_path, right_path: source paths
      - left_dt, right_dt: datetime objects
      - delta_evals, delta_triggers: scalar int deltas
      - delta_raw_total, delta_cal_total, delta_sup_total: scalar int deltas
      - delta_time_seconds: int (right - left)
      - trigger_diffs: list of dicts with
          name, kind, status, raw_l, raw_r, raw_delta,
          cal_l, cal_r, cal_delta, sup_l, sup_r, sup_delta,
          rate_l, rate_r, rate_delta
        where status is '=' / '~' / '+' / '-'
      - added: list of trigger names (in right but not left)
      - removed: list of trigger names (in left but not right)
    """
    out: dict[str, Any] = {
        "left_path": left.get("path"),
        "right_path": right.get("path"),
        "left_dt": left.get("generated_dt"),
        "right_dt": right.get("generated_dt"),
        "left_schema": left.get("schema"),
        "right_schema": right.get("schema"),
        "left_source": left.get("source"),
        "right_source": right.get("source"),
        "left_n_evals": left.get("n_evals", 0),
        "right_n_evals": right.get("n_evals", 0),
        "left_n_triggers": left.get("n_triggers", 0),
        "right_n_triggers": right.get("n_triggers", 0),
    }

    # Scalar deltas
    out["delta_evals"] = right.get("n_evals", 0) - left.get("n_evals", 0)
    out["delta_triggers"] = right.get("n_triggers", 0) - left.get("n_triggers", 0)

    lh = left.get("honesty", {})
    rh = right.get("honesty", {})
    out["delta_raw_total"] = rh.get("raw", 0) - lh.get("raw", 0)
    out["delta_cal_total"] = rh.get("cal", 0) - lh.get("cal", 0)
    out["delta_sup_total"] = rh.get("sup", 0) - lh.get("sup", 0)

    # Time delta in seconds
    lt = left.get("generated_dt")
    rt = right.get("generated_dt")
    if lt is not None and rt is not None:
        try:
            delta = (rt - lt).total_seconds()
        except Exception:
            delta = 0
        out["delta_time_seconds"] = int(delta)
    else:
        out["delta_time_seconds"] = 0

    # Per-trigger diff
    lt_tl = _by_name(left.get("timeline", []))
    rt_tl = _by_name(right.get("timeline", []))
    lt_sm = _by_name(left.get("summary", []))
    rt_sm = _by_name(right.get("summary", []))

    all_names = sorted(set(lt_tl) | set(rt_tl))
    trigger_diffs: list[dict[str, Any]] = []
    added: list[str] = []
    removed: list[str] = []

    for name in all_names:
        in_l = name in lt_tl
        in_r = name in rt_tl
        if in_l and in_r:
            l_row = lt_tl[name]
            r_row = rt_tl[name]
            l_sm = lt_sm.get(name, {})
            r_sm = rt_sm.get(name, {})
            raw_delta = r_row["raw"] - l_row["raw"]
            cal_delta = r_row["cal"] - l_row["cal"]
            sup_delta = r_row["sup"] - l_row["sup"]
            rate_l = l_sm.get("fire_rate", 0.0)
            rate_r = r_sm.get("fire_rate", 0.0)
            rate_delta = rate_r - rate_l
            # Changed if any count delta != 0 OR rate delta != 0
            # (rate delta can change even when counts are stable because the
            # evaluation denominator shifted — that is itself a meaningful signal)
            unchanged = (
                raw_delta == 0
                and cal_delta == 0
                and sup_delta == 0
                and abs(rate_delta) < 1e-9
            )
            status = SYM_IDENTICAL if unchanged else SYM_CHANGED
            trigger_diffs.append({
                "name": name,
                "kind": r_row["kind"],
                "status": status,
                "raw_l": l_row["raw"],
                "raw_r": r_row["raw"],
                "raw_delta": raw_delta,
                "cal_l": l_row["cal"],
                "cal_r": r_row["cal"],
                "cal_delta": cal_delta,
                "sup_l": l_row["sup"],
                "sup_r": r_row["sup"],
                "sup_delta": sup_delta,
                "rate_l": rate_l,
                "rate_r": rate_r,
                "rate_delta": rate_delta,
                "timeline_l": l_row["timeline_str"],
                "timeline_r": r_row["timeline_str"],
            })
        elif in_r and not in_l:
            r_row = rt_tl[name]
            r_sm = rt_sm.get(name, {})
            trigger_diffs.append({
                "name": name,
                "kind": r_row["kind"],
                "status": SYM_ADDED,
                "raw_l": 0,
                "raw_r": r_row["raw"],
                "raw_delta": r_row["raw"],
                "cal_l": 0,
                "cal_r": r_row["cal"],
                "cal_delta": r_row["cal"],
                "sup_l": 0,
                "sup_r": r_row["sup"],
                "sup_delta": r_row["sup"],
                "rate_l": 0.0,
                "rate_r": r_sm.get("fire_rate", 0.0),
                "rate_delta": r_sm.get("fire_rate", 0.0),
                "timeline_l": "",
                "timeline_r": r_row["timeline_str"],
            })
            added.append(name)
        else:  # in_l and not in_r
            l_row = lt_tl[name]
            l_sm = lt_sm.get(name, {})
            trigger_diffs.append({
                "name": name,
                "kind": l_row["kind"],
                "status": SYM_REMOVED,
                "raw_l": l_row["raw"],
                "raw_r": 0,
                "raw_delta": -l_row["raw"],
                "cal_l": l_row["cal"],
                "cal_r": 0,
                "cal_delta": -l_row["cal"],
                "sup_l": l_row["sup"],
                "sup_r": 0,
                "sup_delta": -l_row["sup"],
                "rate_l": l_sm.get("fire_rate", 0.0),
                "rate_r": 0.0,
                "rate_delta": -l_sm.get("fire_rate", 0.0),
                "timeline_l": l_row["timeline_str"],
                "timeline_r": "",
            })
            removed.append(name)

    out["trigger_diffs"] = trigger_diffs
    out["added"] = added
    out["removed"] = removed
    return out


# ----------------------------------------------------------------------
# Markdown rendering
# ----------------------------------------------------------------------

def _format_delta(v: int) -> str:
    """Format an int delta with explicit sign."""
    if v > 0:
        return f"+{v}"
    return str(v)


def _format_delta_pct(v: float) -> str:
    """Format a percent delta with explicit sign."""
    if v > 0:
        return f"+{v:.2f}%"
    if v < 0:
        return f"{v:.2f}%"
    return "0.00%"


def _format_duration(seconds: int) -> str:
    """Format a duration in seconds as a human-readable string.

    Convention: negative values get a leading `-`; positive values are unsigned.
    Zero is `0s`. The sign is implicit except for negative durations.
    """
    if seconds == 0:
        return "0s"
    s = abs(seconds)
    if s < 60:
        return f"-{s}s" if seconds < 0 else f"{s}s"
    if s < 3600:
        m = s // 60
        r = s % 60
        return f"-{m}m{r:02d}s" if seconds < 0 else f"{m}m{r:02d}s"
    h = s // 3600
    m = (s % 3600) // 60
    return f"-{h}h{m:02d}m" if seconds < 0 else f"{h}h{m:02d}m"


def render_diff_markdown(diff_data: dict[str, Any], *, title: str | None = None,
                          left_path: str | None = None,
                          right_path: str | None = None) -> str:
    """Render the diff as a single Markdown document.

    Args:
      diff_data: dict from compute_diff()
      title: optional title override
      left_path / right_path: defaults used in the header if not in diff_data
    """
    lines: list[str] = []
    title = title or "V1374 — V1373 Snapshot Diff"
    lp = left_path or diff_data.get("left_path") or "(left)"
    rp = right_path or diff_data.get("right_path") or "(right)"

    lines.append(f"# {title}")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **generated:** {_dt.datetime.now(_dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}")
    lines.append(f"- **left:** `{lp}`")
    lines.append(f"- **right:** `{rp}`")
    lines.append(f"- **left schema:** `{diff_data.get('left_schema') or '?'}`")
    lines.append(f"- **right schema:** `{diff_data.get('right_schema') or '?'}`")
    lines.append(f"- **triggers compared:** {len(diff_data.get('trigger_diffs', []))}")
    lines.append("")

    # Scalar delta block
    lines.append("## Scalar deltas")
    lines.append("")
    lines.append("| metric | left | right | delta |")
    lines.append("|--------|-----:|------:|------:|")
    # Use the n_evals from the headers via diff_data (we stored delta only)
    # Reconstruct left/right values from the trigger table sums
    left_raw_total = sum(t["raw_l"] for t in diff_data.get("trigger_diffs", []))
    right_raw_total = sum(t["raw_r"] for t in diff_data.get("trigger_diffs", []))
    left_cal_total = sum(t["cal_l"] for t in diff_data.get("trigger_diffs", []))
    right_cal_total = sum(t["cal_r"] for t in diff_data.get("trigger_diffs", []))
    left_sup_total = sum(t["sup_l"] for t in diff_data.get("trigger_diffs", []))
    right_sup_total = sum(t["sup_r"] for t in diff_data.get("trigger_diffs", []))
    # We need the header-level counts to render left/right for evals/triggers.
    # compute_diff() stored only deltas; reconstruct from the source dicts.
    left_evals = diff_data.get("left_n_evals", 0)
    right_evals = diff_data.get("right_n_evals", 0)
    left_triggers = diff_data.get("left_n_triggers", 0)
    right_triggers = diff_data.get("right_n_triggers", 0)

    lines.append(f"| raw fires | {left_raw_total} | {right_raw_total} | {_format_delta(diff_data['delta_raw_total'])} |")
    lines.append(f"| calibrated fires | {left_cal_total} | {right_cal_total} | {_format_delta(diff_data['delta_cal_total'])} |")
    lines.append(f"| suppressed FP | {left_sup_total} | {right_sup_total} | {_format_delta(diff_data['delta_sup_total'])} |")
    lines.append(f"| evaluations | {left_evals} | {right_evals} | {_format_delta(diff_data['delta_evals'])} |")
    lines.append(f"| triggers | {left_triggers} | {right_triggers} | {_format_delta(diff_data['delta_triggers'])} |")
    lines.append(f"| time gap | — | — | {_format_duration(diff_data['delta_time_seconds'])} |")
    lines.append("")
    if diff_data.get("added"):
        lines.append(f"**Added triggers:** {', '.join('`' + n + '`' for n in diff_data['added'])}")
        lines.append("")
    if diff_data.get("removed"):
        lines.append(f"**Removed triggers:** {', '.join('`' + n + '`' for n in diff_data['removed'])}")
        lines.append("")

    # Per-trigger delta table
    lines.append("## Per-trigger deltas")
    lines.append("")
    lines.append("| status | trigger | kind | raw Δ | cal Δ | sup Δ | rate Δ |")
    lines.append("|:------:|---------|------|------:|------:|------:|-------:|")
    for t in diff_data.get("trigger_diffs", []):
        sym = t["status"]
        safe_name = t["name"].replace("|", "\\|")
        safe_kind = t["kind"].replace("|", "\\|")
        lines.append(
            f"| {sym} | `{safe_name}` | {safe_kind} | "
            f"{_format_delta(t['raw_delta'])} | {_format_delta(t['cal_delta'])} | "
            f"{_format_delta(t['sup_delta'])} | {_format_delta_pct(t['rate_delta'])} |"
        )
    lines.append("")

    # Per-trigger timeline deltas (only for changed ones)
    changed = [t for t in diff_data.get("trigger_diffs", []) if t["status"] == SYM_CHANGED]
    if changed:
        lines.append("## Changed-trigger timeline detail")
        lines.append("")
        lines.append("| trigger | left timeline | right timeline |")
        lines.append("|---------|---------------|----------------|")
        for t in changed:
            safe_name = t["name"].replace("|", "\\|")
            tl_l = t["timeline_l"] or "(empty)"
            tl_r = t["timeline_r"] or "(empty)"
            lines.append(f"| `{safe_name}` | `{tl_l}` | `{tl_r}` |")
        lines.append("")

    # Legend
    lines.append("## Legend")
    lines.append("")
    lines.append("| symbol | meaning |")
    lines.append("|:------:|---------|")
    lines.append(f"| `{SYM_IDENTICAL}` | unchanged (no raw/cal/sup delta) |")
    lines.append(f"| `{SYM_CHANGED}` | changed (one or more count deltas non-zero) |")
    lines.append(f"| `{SYM_ADDED}` | trigger in right but not in left |")
    lines.append(f"| `{SYM_REMOVED}` | trigger in left but not in right |")
    lines.append("")

    # Honesty disclosure
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append(
        "This diff is a pure reader of two V1373 .md files. It does not write back, "
        "does not touch the sidecar, does not touch the ledger, does not raise the cap, "
        "does not pretend anything."
    )
    lines.append("")
    lines.append(
        f"- **trigger-rows compared:** {len(diff_data.get('trigger_diffs', []))}"
    )
    lines.append(f"- **added:** {len(diff_data.get('added', []))}")
    lines.append(f"- **removed:** {len(diff_data.get('removed', []))}")
    lines.append(f"- **changed:** {len(changed)}")
    lines.append(f"- **unchanged:** {sum(1 for t in diff_data.get('trigger_diffs', []) if t['status'] == SYM_IDENTICAL)}")
    lines.append("")
    if diff_data.get("delta_raw_total", 0) == 0 and diff_data.get("delta_cal_total", 0) == 0 and not diff_data.get("added") and not diff_data.get("removed"):
        lines.append(
            "**Honest baseline:** no scalar fire-count change, no added/removed triggers. "
            "This is **plateau, not failure** — between the two snapshots there is no "
            "remeasure signal and no V0.3 evolution signal. See V1370_REPORT.md for "
            "calibration details."
        )
        lines.append("")
    lines.append(
        "**Antisymmetry check:** swapping `left` and `right` should negate every "
        "scalar delta and every per-trigger delta. Run twice (left↔right, right↔left) "
        "and compare if you want to verify."
    )
    lines.append("")

    # Footer
    lines.append("---")
    lines.append("")
    lines.append(
        f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see "
        f"`apeireth/v1374_v1373_diff.py` and `V1374_REPORT.md`._"
    )
    lines.append("")

    return "\n".join(lines)


# ----------------------------------------------------------------------
# File I/O
# ----------------------------------------------------------------------

def write_diff_markdown(path: str, content: str) -> None:
    """Atomic write: tmp + rename, UTF-8 encoding."""
    d = os.path.dirname(os.path.abspath(path)) or "."
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1374_", suffix=".md.tmp", dir=d)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(content)
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def diff_two_files(left_path: str, right_path: str, *, out_path: str | None = None,
                   title: str | None = None) -> int:
    """Full pipeline: parse both → compute diff → write markdown.

    If out_path is None, prints to stdout.
    Returns 0 on success, 2 on file error.
    """
    try:
        left = parse_markdown(left_path)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2
    try:
        right = parse_markdown(right_path)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2

    diff_data = compute_diff(left, right)
    md = render_diff_markdown(diff_data, title=title, left_path=left_path, right_path=right_path)
    if out_path:
        write_diff_markdown(out_path, md)
        print(f"wrote: {out_path}")
    else:
        sys.stdout.write(md)
    return 0


def summary_two_files(left_path: str, right_path: str) -> int:
    """Print a short text summary of the diff to stdout."""
    try:
        left = parse_markdown(left_path)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2
    try:
        right = parse_markdown(right_path)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2

    diff_data = compute_diff(left, right)
    print(f"V1374 diff summary")
    print(f"  left:  {left_path}")
    print(f"  right: {right_path}")
    print(f"  time gap: {_format_duration(diff_data['delta_time_seconds'])}")
    print(f"  delta raw fires: {_format_delta(diff_data['delta_raw_total'])}")
    print(f"  delta cal fires: {_format_delta(diff_data['delta_cal_total'])}")
    print(f"  delta suppressed FP: {_format_delta(diff_data['delta_sup_total'])}")
    print(f"  added: {len(diff_data['added'])}, removed: {len(diff_data['removed'])}")
    changed = [t for t in diff_data['trigger_diffs'] if t['status'] == SYM_CHANGED]
    print(f"  changed: {len(changed)}, unchanged: {sum(1 for t in diff_data['trigger_diffs'] if t['status'] == SYM_IDENTICAL)}")
    return 0


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def run_cli(args: list[str]) -> int:
    """Argv dispatcher. Returns process exit code."""
    parser = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1374 — Diff two V1373 markdown exports (post-V1373 next-step 2/5)",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_diff = sub.add_parser("diff", help="compute diff and write markdown (default)")
    p_diff.add_argument("--left", required=True, help="left V1373 .md path")
    p_diff.add_argument("--right", required=True, help="right V1373 .md path")
    p_diff.add_argument("--out", default=None, help="output path (default: stdout)")
    p_diff.add_argument("--title", default=None, help="custom title")

    p_summary = sub.add_parser("summary", help="short text summary to stdout")
    p_summary.add_argument("--left", required=True)
    p_summary.add_argument("--right", required=True)

    p_version = sub.add_parser("version", help="print version")
    p_popper = sub.add_parser("popper", help="run Popper self-tests")
    p_popper.add_argument("-v", "--verbose", action="store_true")

    # Default subcommand if none given
    SUBCMDS = {"diff", "summary", "version", "popper"}
    has_subcmd = any(a in SUBCMDS for a in args)
    if not has_subcmd:
        args = ["diff"] + args

    parsed = parser.parse_args(args)

    if parsed.cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0
    if parsed.cmd == "popper":
        passed, total, failures = _popper_self_tests(verbose=parsed.verbose)
        print(f"Popper self-tests: {passed}/{total} passed")
        if failures:
            print("FAILURES:")
            for f in failures:
                print(f"  - {f}")
            return 1
        return 0
    if parsed.cmd == "summary":
        return summary_two_files(parsed.left, parsed.right)

    # diff
    return diff_two_files(parsed.left, parsed.right, out_path=parsed.out, title=parsed.title)


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> tuple[int, int, list[str]]:
    """32 Popper-style self-tests. Returns (passed, total, failures)."""
    failures: list[str] = []
    passed = 0

    def check(name: str, cond: bool) -> None:
        nonlocal passed
        if cond:
            passed += 1
            if verbose:
                print(f"  OK   {name}")
        else:
            failures.append(name)
            if verbose:
                print(f"  FAIL {name}")

    # Synthetic V1373 markdown A
    md_a = (
        "# V1373 — V1372 Markdown Export\n"
        "\n"
        "- **schema:** `v1373.markdown/v1`\n"
        "- **generated:** 2026-08-08T19:00:00Z\n"
        "- **source sidecar:** `sidecar_a.jsonl`\n"
        "- **triggers:** 2\n"
        "- **evaluations:** 10\n"
        "\n"
        "## Per-trigger timeline\n"
        "\n"
        "| trigger | kind | timeline | raw | cal | sup |\n"
        "|---------|------|----------|----:|----:|----:|\n"
        "| `T_FIRE` | remeasure | `·5●5` | 5 | 5 | 0 |\n"
        "| `T_QUIET` | remeasure | `·10` | 0 | 0 | 0 |\n"
        "\n"
        "## Summary\n"
        "\n"
        "| trigger | kind | fire_rate |\n"
        "|---------|------|----------:|\n"
        "| `T_FIRE` | remeasure | 50.00% |\n"
        "| `T_QUIET` | remeasure | 0.00% |\n"
        "\n"
        "## Legend\n"
        "\n"
        "| char | meaning |\n"
        "|------|---------|\n"
        "| `·` | no fire |\n"
        "| `●` | raw fire |\n"
        "\n"
        "## Honesty disclosure\n"
        "\n"
        "- **trigger-checks evaluated:** 20 (10 evaluations × 2 triggers)\n"
        "- **raw fires:** 5\n"
        "- **calibrated fires:** 5\n"
        "- **V1370-suppressed false positives:** 0\n"
        "\n"
    )

    md_b = (
        "# V1373 — V1372 Markdown Export\n"
        "\n"
        "- **schema:** `v1373.markdown/v1`\n"
        "- **generated:** 2026-08-08T20:00:00Z\n"
        "- **source sidecar:** `sidecar_b.jsonl`\n"
        "- **triggers:** 3\n"
        "- **evaluations:** 12\n"
        "\n"
        "## Per-trigger timeline\n"
        "\n"
        "| trigger | kind | timeline | raw | cal | sup |\n"
        "|---------|------|----------|----:|----:|----:|\n"
        "| `T_FIRE` | remeasure | `·7●5` | 5 | 5 | 0 |\n"
        "| `T_QUIET` | remeasure | `·12` | 0 | 0 | 0 |\n"
        "| `T_NEW` | v03_evolution | `·12` | 0 | 0 | 0 |\n"
        "\n"
        "## Summary\n"
        "\n"
        "| trigger | kind | fire_rate |\n"
        "|---------|------|----------:|\n"
        "| `T_FIRE` | remeasure | 41.67% |\n"
        "| `T_QUIET` | remeasure | 0.00% |\n"
        "| `T_NEW` | v03_evolution | 0.00% |\n"
        "\n"
        "## Legend\n"
        "\n"
        "| char | meaning |\n"
        "|------|---------|\n"
        "| `·` | no fire |\n"
        "| `●` | raw fire |\n"
        "\n"
        "## Honesty disclosure\n"
        "\n"
        "- **trigger-checks evaluated:** 36 (12 evaluations × 3 triggers)\n"
        "- **raw fires:** 5\n"
        "- **calibrated fires:** 5\n"
        "- **V1370-suppressed false positives:** 0\n"
        "\n"
    )

    with tempfile.TemporaryDirectory() as td:
        a_path = os.path.join(td, "a.md")
        b_path = os.path.join(td, "b.md")
        with open(a_path, "w", encoding="utf-8") as fh:
            fh.write(md_a)
        with open(b_path, "w", encoding="utf-8") as fh:
            fh.write(md_b)

        # 1-7: parse_markdown on A
        a = parse_markdown(a_path)
        check("parse A: schema", a["schema"] == "v1373.markdown/v1")
        check("parse A: generated", a["generated"] == "2026-08-08T19:00:00Z")
        check("parse A: source", a["source"] == "sidecar_a.jsonl")
        check("parse A: n_triggers", a["n_triggers"] == 2)
        check("parse A: n_evals", a["n_evals"] == 10)
        check("parse A: timeline len", len(a["timeline"]) == 2)
        check("parse A: honesty raw", a["honesty"]["raw"] == 5)

        # 8-14: parse_markdown on B
        b = parse_markdown(b_path)
        check("parse B: schema", b["schema"] == "v1373.markdown/v1")
        check("parse B: generated", b["generated"] == "2026-08-08T20:00:00Z")
        check("parse B: n_triggers", b["n_triggers"] == 3)
        check("parse B: n_evals", b["n_evals"] == 12)
        check("parse B: timeline len", len(b["timeline"]) == 3)
        check("parse B: honesty raw", b["honesty"]["raw"] == 5)
        check("parse B: contains T_NEW", any(t["name"] == "T_NEW" for t in b["timeline"]))

        # 15-22: compute_diff
        d = compute_diff(a, b)
        check("diff: delta_time_seconds == 3600", d["delta_time_seconds"] == 3600)
        check("diff: delta_raw_total == 0", d["delta_raw_total"] == 0)
        check("diff: delta_cal_total == 0", d["delta_cal_total"] == 0)
        check("diff: delta_evals == 2", d["delta_evals"] == 2)
        check("diff: delta_triggers == 1", d["delta_triggers"] == 1)
        check("diff: added contains T_NEW", d["added"] == ["T_NEW"])
        check("diff: removed is empty", d["removed"] == [])
        check("diff: T_FIRE changed", any(t["status"] == SYM_CHANGED and t["name"] == "T_FIRE" for t in d["trigger_diffs"]))

        # 23-26: antisymmetry check
        d_rev = compute_diff(b, a)
        check("diff antisym: delta_time_seconds negated", d_rev["delta_time_seconds"] == -3600)
        check("diff antisym: delta_evals negated", d_rev["delta_evals"] == -2)
        check("diff antisym: delta_triggers negated", d_rev["delta_triggers"] == -1)
        check("diff antisym: added/removed swapped", d_rev["added"] == [] and d_rev["removed"] == ["T_NEW"])

        # 27-29: render_diff_markdown
        md = render_diff_markdown(d, left_path=a_path, right_path=b_path)
        check("render: starts with # ", md.startswith("# "))
        check("render: contains schema", "v1374.diff/v1" in md)
        check("render: no HTML", "<" not in md and ">" not in md)

        # 30-32: atomic write + round-trip via diff_two_files
        out_path = os.path.join(td, "diff.md")
        rc = diff_two_files(a_path, b_path, out_path=out_path)
        check("diff_two_files: returns 0", rc == 0)
        check("diff_two_files: out file exists", os.path.exists(out_path))
        check("diff_two_files: out file non-empty", os.path.getsize(out_path) > 200)

    total = passed + len(failures)
    return passed, total, failures


# ----------------------------------------------------------------------
# Module entry point
# ----------------------------------------------------------------------

def main() -> int:
    return run_cli(sys.argv[1:])


if __name__ == "__main__":
    sys.exit(main())
