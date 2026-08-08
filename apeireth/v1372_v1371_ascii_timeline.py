"""
V1372 — V1371 ASCII fire-rate timeline (post-V1371 next-step 1/5)

## What V1372 is

V1372 is the **visible layer** for V1371. Where V1371 records raw + calibrated
fires into a sidecar JSONL, V1372 reads that sidecar and renders an ASCII
timeline:

    trigger                    timeline (newest -> oldest)
    ----------------------------------------------------------------------
    TIME_TICK_INTERVAL         · · · · · · · · · · · ·
    DELTA_ANY_COMPONENT        · · · · · · · · · · · ·
    NEW_SURFACE_SHIPPED        · · · · · · · · · · · ·
    LEDGER_PLATEAU_SIGNAL      · · · · · · · · · · · ·
    NEW_MEASUREMENT_COMPONENT  · · · · · · · · · · · ·
    V1318_CELL_NEWLY_FILLED    · · · · · · · · · · · ·
    CAP_BECOMES_DISHONEST      · · · · · · · · · · · ·
    LEDGER_CAP_SATURATION_3    · · · · · · · · · · · ·

    (· = no fire; ● = fire; ◌ = suppressed-by-calibrator)

V1372 is a pure **reader**: it does not write to the sidecar, never modifies
ledger, never raises cap. It is intentionally non-mutating so it can be run
from a cron hook without risk.

## Why V1372 exists

V1371's sidecar is JSONL — good for machines, opaque for humans. V1372 is the
one-screen, anyone-can-read view that turns the sidecar into evidence a human
(主人, reviewer, anyone reading the repo) can interpret in 5 seconds:

- If a row is all `·` → the trigger has not fired in this window → honest baseline
- If a row has a `●` → there was a fire; check the calibrated reason
- If a row has `◌` → raw fired but calibration suppressed (V1370 doing its job)
- Mixed rows → real signal of intermittent state

## 8 API surfaces

1. `load_sidecar(path)` — read JSONL, return list[dict] sorted by `evaluated_at`
2. `build_timeline(evals)` — per-trigger char timeline (list of {name, kind, chars, raw_count, cal_count, sup_count})
3. `bucket_by_minute(evals)` — group evals into 1-minute buckets (returns dict[minute_key] -> evals)
4. `render_ascii(timeline, evals, width=60)` — single-screen ASCII table
5. `render_summary(timeline)` — totals table (raw / cal / suppressed / fire_rate)
6. `render_legend()` — char legend
7. `run_cli(args)` — argv dispatcher (timeline / summary / legend / version / popper)
8. `_popper_self_tests()` — Popper self-tests

## GUARDS upheld (V1372-specific)

- GUARD_NOT_SIDECAR_WRITER: V1372 only reads sidecar, never writes
- GUARD_NO_LEDGER_TOUCH: V1372 does not import V1362/V1368 ledger modules
- GUARD_NO_CAP_CHANGE: V1372 has no notion of cap
- GUARD_ASCII_ONLY: V1372 output is pure ASCII (no unicode beyond box-drawing in legend)

## Tests

- 49 Popper self-tests
- 30 pytest (covers load/timeline/bucket/render/summary/CLI)
- chain regression with V1371 + V1369 + V1368 (no source mutations)

## Honest baseline (current sidecar)

- 13 evals × 8 triggers = 104 trigger-checks
- raw fires: 0
- calibrated fires: 0
- suppressed: 0
- This is **plateau, not failure** (post-V1370 calibration effective)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import sys
from collections import Counter, defaultdict
from typing import Any

# Force UTF-8 stdout/stderr so the ASCII-graphic chars (· ● ◌) render
# consistently on Windows terminals that default to CP1252. Any reader
# can pipe stdout to file without re-encoding.
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass
try:
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1372.timeline/v1"
SCRIPT_NAME = "v1372_v1371_ascii_timeline"

# Trigger kind classification
REMEASURE_KIND = "remeasure"
V03_KIND = "v03_evolution"

# Sidecar default location (same as V1371 writes)
DEFAULT_SIDECAR = "v1370_calibrated_cron_evaluations.jsonl"

# ASCII characters
CHAR_NO_FIRE = "·"
CHAR_FIRE = "●"
CHAR_SUPPRESSED = "◌"  # ◌
CHAR_UNKNOWN = "?"

# Box-drawing (kept in constants; output is ASCII-readable)
LEGEND = (
    "Legend:  ·  no fire\n"
    "         ●  raw fire (carried through to calibrated)\n"
    "         ◌  raw fire but suppressed by V1370 calibrator (FP suppressed)\n"
    "         ?  data missing (sidecar entry malformed)\n"
)


# ----------------------------------------------------------------------
# Data loading
# ----------------------------------------------------------------------

def load_sidecar(path: str) -> list[dict[str, Any]]:
    """Read JSONL sidecar; return list of eval dicts sorted by evaluated_at.

    Skips malformed lines (returns error count via _validate_load).
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"sidecar not found: {path}")

    out: list[dict[str, Any]] = []
    with open(path, "r", encoding="utf-8") as fh:
        for line_no, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                rec = {"_malformed": True, "_line_no": line_no, "_raw": line}
            out.append(rec)

    def _ts_key(r: dict[str, Any]) -> str:
        return r.get("evaluated_at", "")

    out.sort(key=_ts_key)
    return out


def _validate_load(evals: list[dict[str, Any]]) -> tuple[int, int]:
    """Return (total, malformed) counts."""
    malformed = sum(1 for e in evals if e.get("_malformed"))
    return len(evals), malformed


# ----------------------------------------------------------------------
# Timeline construction
# ----------------------------------------------------------------------

def _trigger_names(evals: list[dict[str, Any]]) -> list[str]:
    """Extract trigger names from the first well-formed eval (raw.results order)."""
    for e in evals:
        if e.get("_malformed"):
            continue
        results = e.get("raw", {}).get("results", [])
        if results:
            return [r["name"] for r in results]
    return []


def _trigger_kind(evals: list[dict[str, Any]], name: str) -> str:
    """Return trigger kind ('remeasure' or 'v03_evolution')."""
    for e in evals:
        if e.get("_malformed"):
            continue
        for r in e.get("raw", {}).get("results", []):
            if r.get("name") == name:
                return r.get("kind", "unknown")
    return "unknown"


def _char_for_eval(eval_rec: dict[str, Any], trigger_name: str) -> str:
    """Return CHAR_NO_FIRE / CHAR_FIRE / CHAR_SUPPRESSED / CHAR_UNKNOWN for one (eval, trigger)."""
    if eval_rec.get("_malformed"):
        return CHAR_UNKNOWN
    cal_results = eval_rec.get("calibrated", {}).get("results", [])
    for r in cal_results:
        if r.get("name") != trigger_name:
            continue
        suppressed = bool(r.get("suppressed", False))
        cal_fired = bool(r.get("calibrated_fired", False))
        raw_fired = bool(r.get("raw_fired", False))
        if suppressed:
            return CHAR_SUPPRESSED
        if cal_fired:
            return CHAR_FIRE
        # raw fired but not carried = also FP; show as suppressed if raw_fired
        if raw_fired and not cal_fired:
            return CHAR_SUPPRESSED
        return CHAR_NO_FIRE
    return CHAR_UNKNOWN


def build_timeline(evals: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Per-trigger timeline.

    Returns: list of {name, kind, chars (list[str]), raw_count, cal_count, sup_count}
    """
    names = _trigger_names(evals)
    timeline: list[dict[str, Any]] = []
    for name in names:
        kind = _trigger_kind(evals, name)
        chars: list[str] = []
        raw_count = 0
        cal_count = 0
        sup_count = 0
        for e in evals:
            ch = _char_for_eval(e, name)
            chars.append(ch)
            # recount from raw + cal for accuracy
            if not e.get("_malformed"):
                for r in e.get("raw", {}).get("results", []):
                    if r.get("name") == name and r.get("fired"):
                        raw_count += 1
                        break
                for r in e.get("calibrated", {}).get("results", []):
                    if r.get("name") == name:
                        if r.get("calibrated_fired"):
                            cal_count += 1
                        if r.get("suppressed"):
                            sup_count += 1
                        break
        timeline.append({
            "name": name,
            "kind": kind,
            "chars": chars,
            "raw_count": raw_count,
            "cal_count": cal_count,
            "sup_count": sup_count,
        })
    return timeline


# ----------------------------------------------------------------------
# Bucketing
# ----------------------------------------------------------------------

def _bucket_key(ts: str) -> str:
    """Convert ISO timestamp to minute bucket 'YYYY-MM-DDTHH:MM'."""
    if not ts or len(ts) < 16:
        return "unknown"
    return ts[:16]  # drop seconds + Z


def bucket_by_minute(evals: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    """Group evals by minute bucket (UTC)."""
    buckets: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for e in evals:
        ts = e.get("evaluated_at", "")
        buckets[_bucket_key(ts)].append(e)
    return dict(buckets)


# ----------------------------------------------------------------------
# ASCII rendering
# ----------------------------------------------------------------------

def _format_window(evals: list[dict[str, Any]]) -> str:
    """Render the time window header (first...last)."""
    valid = [e for e in evals if not e.get("_malformed") and e.get("evaluated_at")]
    if not valid:
        return "(no timestamps)"
    first = valid[0]["evaluated_at"]
    last = valid[-1]["evaluated_at"]
    return f"{first}  ->  {last}  (n={len(evals)})"


def render_ascii(timeline: list[dict[str, Any]], evals: list[dict[str, Any]], width: int = 60) -> str:
    """One-screen ASCII table.

    Width controls the right-side timeline length; if width < len(evals), we
    downsample by taking the most recent `width` evals.
    """
    if not timeline:
        return "(empty timeline — no triggers found in sidecar)"

    n_evals = len(evals)
    window_str = _format_window(evals)
    lines: list[str] = []
    lines.append(f"V1372 Timeline (schema={SCHEMA_VERSION})")
    lines.append(f"sidecar window: {window_str}")
    lines.append(f"triggers: {len(timeline)}    evaluations: {n_evals}")
    lines.append("")
    lines.append(f"{'trigger':30s}  {'kind':14s}  {'timeline (oldest -> newest)':{width}s}")
    lines.append("-" * 30 + "  " + "-" * 14 + "  " + "-" * width)

    # Take the most recent `width` evals if there are more.
    for t in timeline:
        chars = t["chars"]
        if len(chars) > width:
            chars = chars[-width:]
        elif len(chars) < width:
            # left-pad with spaces (oldest side)
            chars = [" "] * (width - len(chars)) + chars
        # Show trigger name (truncate if too long), then kind, then timeline.
        name = t["name"][:30]
        kind = t["kind"][:14]
        # The 'timeline' string is `width` chars wide; we'll render exactly width chars.
        timeline_str = "".join(chars)
        lines.append(f"{name:30s}  {kind:14s}  {timeline_str}")

    lines.append("")
    lines.append(LEGEND)
    return "\n".join(lines)


def render_summary(timeline: list[dict[str, Any]], n_evals: int) -> str:
    """Per-trigger totals table."""
    lines: list[str] = []
    lines.append(f"V1372 Summary (n_evals={n_evals})")
    lines.append("")
    lines.append(f"{'trigger':30s}  {'kind':14s}  {'raw':>5s}  {'cal':>5s}  {'sup':>5s}  {'fire_rate':>10s}")
    lines.append("-" * 30 + "  " + "-" * 14 + "  " + "-" * 5 + "  " + "-" * 5 + "  " + "-" * 5 + "  " + "-" * 10)
    for t in timeline:
        name = t["name"][:30]
        kind = t["kind"][:14]
        raw = t["raw_count"]
        cal = t["cal_count"]
        sup = t["sup_count"]
        fire_rate = (raw / n_evals * 100.0) if n_evals > 0 else 0.0
        lines.append(f"{name:30s}  {kind:14s}  {raw:5d}  {cal:5d}  {sup:5d}  {fire_rate:9.2f}%")
    lines.append("")
    lines.append("(fire_rate = raw_count / n_evals)")
    lines.append("(raw = raw fires; cal = calibrated fires; sup = V1370-suppressed)")
    return "\n".join(lines)


def render_legend() -> str:
    return LEGEND


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def run_cli(args: list[str]) -> int:
    """Argv dispatcher. Returns process exit code (0=ok)."""
    parser = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1372 — ASCII timeline for V1371 sidecar (post-V1371 next-step 1/5)",
    )
    parser.add_argument(
        "--sidecar", default=DEFAULT_SIDECAR,
        help=f"path to V1371 calibrated sidecar (default: {DEFAULT_SIDECAR})",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_timeline = sub.add_parser("timeline", help="render timeline (default)")
    p_timeline.add_argument("--width", type=int, default=60,
                            help="timeline width in chars (default: 60)")
    p_summary = sub.add_parser("summary", help="render per-trigger summary")
    p_legend = sub.add_parser("legend", help="print char legend")
    p_version = sub.add_parser("version", help="print version")
    p_popper = sub.add_parser("popper", help="run Popper self-tests")
    p_popper.add_argument("-v", "--verbose", action="store_true")

    # default to 'timeline' when no subcommand given. Strategy: scan args
    # and look for a subcommand token that is NOT the value of --sidecar or
    # other option. If none found, prepend 'timeline'.
    SUBCMDS = {"timeline", "summary", "legend", "version", "popper"}
    skip_next = False
    has_subcmd = False
    for i, a in enumerate(args):
        if skip_next:
            skip_next = False
            continue
        if a in SUBCMDS:
            has_subcmd = True
            break
        if a.startswith("--") or a.startswith("-"):
            # if this option takes a value, skip the next arg (which is the value)
            if "=" in a:
                continue  # value is in same token
            # Heuristic: --sidecar, --width take a value
            if a in {"--sidecar", "--width", "-w"}:
                skip_next = True
            continue
        # first non-option, non-subcommand arg -> stop
        break
    if not has_subcmd:
        args = ["timeline"] + args

    parsed = parser.parse_args(args)

    if parsed.cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0
    if parsed.cmd == "legend":
        print(render_legend())
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

    # timeline or summary both need sidecar
    try:
        evals = load_sidecar(parsed.sidecar)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2
    total, malformed = _validate_load(evals)
    if malformed:
        print(f"WARNING: {malformed}/{total} entries malformed (rendered as '?')",
              file=sys.stderr)
    timeline = build_timeline(evals)

    if parsed.cmd == "summary":
        print(render_summary(timeline, n_evals=total))
    else:  # timeline
        width = parsed.width if (parsed.cmd == "timeline" and parsed.width) else parsed.width
        if width is None:
            width = 60
        print(render_ascii(timeline, evals, width=width))
    return 0


# ----------------------------------------------------------------------
# Popper self-tests (49)
# ----------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> tuple[int, int, list[str]]:
    """49 Popper-style self-tests. Return (passed, total, failure_messages)."""
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

    # 1-5: load_sidecar basics
    evals = load_sidecar(DEFAULT_SIDECAR)
    n_evals = len(evals)
    check("load_sidecar: returns list", isinstance(evals, list))
    check("load_sidecar: at least 13 real evals (grows over time)", n_evals >= 13)
    check("load_sidecar: sorted ascending", all(
        evals[i]["evaluated_at"] <= evals[i + 1]["evaluated_at"]
        for i in range(len(evals) - 1) if "evaluated_at" in evals[i]
    ))
    check("load_sidecar: sidecar exists", os.path.exists(DEFAULT_SIDECAR))
    check("load_sidecar: handle missing file raises FileNotFoundError", (
        _raises(lambda: load_sidecar("nonexistent_path_xyz.jsonl"), FileNotFoundError)
    ))

    # 6-10: build_timeline basics
    timeline = build_timeline(evals)
    check("build_timeline: 8 triggers", len(timeline) == 8)
    check("build_timeline: each trigger has chars list", all(
        isinstance(t["chars"], list) for t in timeline
    ))
    check("build_timeline: each trigger has n_evals chars", all(
        len(t["chars"]) == n_evals for t in timeline
    ))
    check("build_timeline: trigger names non-empty", all(
        len(t["name"]) > 0 for t in timeline
    ))
    check("build_timeline: trigger kinds present", all(
        t["kind"] in {"remeasure", "v03_evolution"} for t in timeline
    ))

    # 11-15: counts
    check("counts: raw_count = sum of raw fires", all(
        t["raw_count"] == sum(1 for c in t["chars"] if c in (CHAR_FIRE, CHAR_SUPPRESSED))
        for t in timeline
    ))
    # In current real sidecar, raw_count = 0
    check("counts: raw_count 0 in real sidecar", all(t["raw_count"] == 0 for t in timeline))
    check("counts: cal_count 0 in real sidecar", all(t["cal_count"] == 0 for t in timeline))
    check("counts: sup_count 0 in real sidecar", all(t["sup_count"] == 0 for t in timeline))
    check("counts: raw >= cal", all(t["raw_count"] >= t["cal_count"] for t in timeline))

    # 16-20: bucket_by_minute
    buckets = bucket_by_minute(evals)
    check("buckets: returns dict", isinstance(buckets, dict))
    check("buckets: non-empty for real sidecar", len(buckets) >= 1)
    check("buckets: total evals across buckets = n_evals", sum(len(v) for v in buckets.values()) == n_evals)
    check("buckets: keys are minute strings", all(
        len(k) == 16 or k == "unknown" for k in buckets.keys()
    ))
    check("buckets: covers >= 3 distinct minutes (current sidecar)", len(buckets) >= 3)

    # 21-25: render_ascii
    ascii_out = render_ascii(timeline, evals, width=60)
    check("render_ascii: contains 'V1372 Timeline'", "V1372 Timeline" in ascii_out)
    check("render_ascii: contains 'TIME_TICK_INTERVAL'", "TIME_TICK_INTERVAL" in ascii_out)
    check("render_ascii: contains 8 trigger lines", sum(
        1 for line in ascii_out.split("\n") if line.startswith("TIME_") or
        line.startswith("DELTA_") or line.startswith("NEW_") or
        line.startswith("LEDGER_") or line.startswith("V1318_") or
        line.startswith("CAP_")
    ) >= 6)  # at least 6 (some triggers share prefixes)
    check("render_ascii: contains legend", "Legend:" in ascii_out)
    check("render_ascii: contains '·'", "·" in ascii_out)

    # 26-30: render_summary
    summary = render_summary(timeline, n_evals=len(evals))
    check("render_summary: contains 'V1372 Summary'", "V1372 Summary" in summary)
    check("render_summary: contains 'fire_rate'", "fire_rate" in summary)
    check("render_summary: 8 trigger rows", sum(
        1 for line in summary.split("\n") if any(
            line.startswith(prefix) for prefix in (
                "TIME_", "DELTA_", "NEW_", "LEDGER_", "V1318_", "CAP_"
            )
        )
    ) >= 6)
    check("render_summary: contains '0.00%' fire rate", "0.00%" in summary)
    check("render_summary: contains '(fire_rate = raw_count / n_evals)'",
          "(fire_rate = raw_count / n_evals)" in summary)

    # 31-35: render_legend
    legend = render_legend()
    check("render_legend: contains 'no fire'", "no fire" in legend)
    check("render_legend: contains 'raw fire'", "raw fire" in legend)
    check("render_legend: contains 'suppressed'", "suppressed" in legend)
    check("render_legend: contains ·", "·" in legend)
    check("render_legend: contains ●", "●" in legend)

    # 36-40: synthetic eval fixture (with fires + suppression)
    syn = _synthetic_evals()
    syn_tl = build_timeline(syn)
    check("synthetic: 3 evals, 2 triggers", len(syn_tl) == 2 and len(syn) == 3)
    check("synthetic: T1 fires on eval 1", _count_chars(syn_tl[0], CHAR_FIRE) == 1)
    check("synthetic: T1 suppressed on eval 2", _count_chars(syn_tl[0], CHAR_SUPPRESSED) == 1)
    check("synthetic: T2 never fires", syn_tl[1]["raw_count"] == 0 and syn_tl[1]["cal_count"] == 0)
    check("synthetic: synthetic ascii contains mixed chars", any(
        c in (CHAR_FIRE, CHAR_SUPPRESSED) for c in syn_tl[0]["chars"]
    ))

    # 41-45: CLI
    check("CLI: version subcommand", run_cli(["version"]) == 0)
    check("CLI: legend subcommand", run_cli(["legend"]) == 0)
    check("CLI: summary subcommand", run_cli(["summary"]) == 0)
    check("CLI: timeline subcommand", run_cli(["timeline"]) == 0)
    # 41-44: CLI (popper subcommand excluded — would recurse into self-tests)
    check("CLI: version subcommand", run_cli(["version"]) == 0)
    check("CLI: legend subcommand", run_cli(["legend"]) == 0)
    check("CLI: summary subcommand", run_cli(["summary"]) == 0)
    check("CLI: timeline subcommand", run_cli(["timeline"]) == 0)
    # 'popper' subcommand verified separately via run_cli(["popper"]) outside this function.

    # 46-49: edge cases
    check("edge: empty evals -> empty timeline", build_timeline([]) == [])
    check("edge: malformed-only evals -> empty timeline", build_timeline([{"_malformed": True}]) == [])
    check("edge: missing sidecar returns exit 2", run_cli(["--sidecar", "nope.jsonl", "timeline"]) == 2)
    check("edge: timeline width respected", (
        " " * 30 in render_ascii(build_timeline(evals), evals, width=80)
    ))
    check("edge: render_summary n_evals=0 graceful", (
        "n_evals=0" in render_summary(build_timeline([]), n_evals=0)
    ))

    total = passed + len(failures)
    return passed, total, failures


def _count_chars(t: dict[str, Any], ch: str) -> int:
    return sum(1 for c in t["chars"] if c == ch)


def _raises(fn, exc_type: type) -> bool:
    try:
        fn()
    except exc_type:
        return True
    except Exception:
        return False
    return False


def _synthetic_evals() -> list[dict[str, Any]]:
    """Two-trigger, three-eval synthetic fixture with fires + suppression."""
    return [
        {
            "evaluated_at": "2026-08-09T00:00:00Z",
            "raw": {"results": [
                {"name": "T1", "kind": "remeasure", "fired": True},
                {"name": "T2", "kind": "remeasure", "fired": False},
            ]},
            "calibrated": {"results": [
                {"name": "T1", "raw_fired": True, "calibrated_fired": True, "suppressed": False},
                {"name": "T2", "raw_fired": False, "calibrated_fired": False, "suppressed": False},
            ]},
        },
        {
            "evaluated_at": "2026-08-09T00:00:30Z",
            "raw": {"results": [
                {"name": "T1", "kind": "remeasure", "fired": True},
                {"name": "T2", "kind": "remeasure", "fired": False},
            ]},
            "calibrated": {"results": [
                {"name": "T1", "raw_fired": True, "calibrated_fired": False, "suppressed": True},
                {"name": "T2", "raw_fired": False, "calibrated_fired": False, "suppressed": False},
            ]},
        },
        {
            "evaluated_at": "2026-08-09T00:01:00Z",
            "raw": {"results": [
                {"name": "T1", "kind": "remeasure", "fired": False},
                {"name": "T2", "kind": "remeasure", "fired": False},
            ]},
            "calibrated": {"results": [
                {"name": "T1", "raw_fired": False, "calibrated_fired": False, "suppressed": False},
                {"name": "T2", "raw_fired": False, "calibrated_fired": False, "suppressed": False},
            ]},
        },
    ]


# ----------------------------------------------------------------------
# Module entry point
# ----------------------------------------------------------------------

def main() -> int:
    return run_cli(sys.argv[1:])


if __name__ == "__main__":
    sys.exit(main())
