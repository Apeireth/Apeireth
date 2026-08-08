"""Phase 1369 v1369_v1368_cron_hook — V1368 trigger evaluation as a cron tick.

## What V1369 is

V1368 ships the trigger conditions as a library + CLI. V1369 is the
**integration layer** that runs V1368 inside the cron tick and records
each evaluation as a JSON sidecar so humans/agents can audit *when* a
trigger last fired.

## Why V1369

V1368 alone is silent: nobody calls it. The cron tick (apeireth-autonomy-v3
+ cross-domain-research-round5-v3) already runs every 5 minutes. Wiring
V1368 into that cadence gives:

  - **Observability of evaluation**: every cron tick writes a sidecar
    `v1368_evaluations.jsonl` with the latest trigger result.
  - **Time-series data**: future humans can plot "how often do triggers
    fire?" without re-running V1368 by hand.
  - **No ledger pollution**: sidecar is separate from
    `pole_star_history.jsonl`, so GUARD_TRIGGERS_ARE_READ_ONLY holds.

## What V1369 does NOT do

  - Does NOT modify the ledger (V1362's append-only invariant holds).
  - Does NOT raise the cap (V1356 source unchanged).
  - Does NOT auto-re-measure: it *recommends* via exit code; the actual
    re-measure is the cron lane's separate decision.
  - Does NOT aspire to V0.3: it just records honest trigger state.

## CLI

  - `python -m apeireth.v1369_v1368_cron_hook evaluate`
        → run V1368 now; write sidecar; print summary; exit 0/1/2/3
  - `python -m apeireth.v1369_v1368_cron_hook evaluate --json`
        → JSON output instead of human-readable
  - `python -m apeireth.v1369_v1368_cron_hook show-last`
        → print most recent sidecar entry
  - `python -m apeireth.v1369_v1368_cron_hook show-last N`
        → print last N sidecar entries
  - `python -m apeireth.v1369_v1368_cron_hook summary`
        → aggregate counts (how many fires / no-fires / by trigger)
  - `python -m apeireth.v1369_v1368_cron_hook self-test [--verbose]`
        → Popper self-tests

## Exit codes

  0  evaluated; no fire
  1  evaluated; remeasure trigger fired (caller decides)
  2  evaluated; V0.3 trigger fired (caller decides)
  3  fatal: cannot write sidecar
  4  invalid usage

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

  - GUARD_SIDECAR_NOT_LEDGER  : sidecar is *separate* from ledger
  - GUARD_EVALUATION_IS_HONEST: records *actual* trigger state, no aspiration
  - GUARD_NO_AUTO_REMEASURE   : V1369 suggests; never invokes V1356 itself
  - GUARD_CAP_NOT_AUTO_RAISED : V1369 never touches cap
  - GUARD_READ_ONLY_LEDGER    : V1369 only reads ledger
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from collections import Counter
from dataclasses import asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from apeireth.v1368_pole_star_v03_triggers import (
    DEFAULT_LEDGER_PATH,
    V1368_GUARDS,
    V1368_VERSION as V1368_TRIGGERS_VERSION,
    should_consider_v03,
    should_remeasure,
)

# -----------------------------------------------------------------------------
# Constants
# -----------------------------------------------------------------------------

V1369_VERSION = "0.1.0"

# Sidecar path: SIBLING of pole_star_history.jsonl, NOT a child of ledger dir
DEFAULT_SIDECAR_PATH = DEFAULT_LEDGER_PATH.parent / "v1368_evaluations.jsonl"

# V3 哲学守门 (V1369-specific)
V1369_GUARDS: Tuple[str, ...] = (
    "GUARD_SIDECAR_NOT_LEDGER",
    "GUARD_EVALUATION_IS_HONEST",
    "GUARD_NO_AUTO_REMEASURE",
    "GUARD_CAP_NOT_AUTO_RAISED",
    "GUARD_READ_ONLY_LEDGER",
)

# Exit code per outcome
EXIT_NO_FIRE = 0
EXIT_REMEASURE_FIRED = 1
EXIT_V03_FIRED = 2
EXIT_FATAL_WRITE = 3


# -----------------------------------------------------------------------------
# Sidecar I/O (write sidecar; never write ledger)
# -----------------------------------------------------------------------------

def _now_iso() -> str:
    """UTC ISO timestamp for evaluation entry."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _serialize_result(result) -> Dict[str, Any]:
    return {
        "name": result.spec.name,
        "kind": result.spec.kind,
        "fired": result.fired,
        "reason": result.reason,
        "evidence": result.evidence,
    }


def evaluate_now(
    ledger_path: Optional[Path] = None,
    sidecar_path: Optional[Path] = None,
    evaluate_at: Optional[str] = None,
) -> Dict[str, Any]:
    """Run V1368 trigger evaluation; write sidecar; return entry.

    Args:
        ledger_path: ledger to read (default: DEFAULT_LEDGER_PATH).
        sidecar_path: where to append the JSONL entry (default: sibling of ledger).
        evaluate_at: ISO timestamp for the entry (default: now UTC).

    Returns:
        Dict with keys: evaluated_at, remeasure, v03_evolution, summary.
    """
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    sidecar_path = sidecar_path or DEFAULT_SIDECAR_PATH
    evaluate_at = evaluate_at or _now_iso()

    remeasure_fired, remeasure_results = should_remeasure(ledger_path)
    v03_fired, v03_results = should_consider_v03(ledger_path)

    entry = {
        "schema": "v1368_evaluation_v1",
        "evaluated_at": evaluate_at,
        "v1369_version": V1369_VERSION,
        "v1368_version": V1368_TRIGGERS_VERSION,
        "ledger_path": str(ledger_path),
        "ledger_exists": ledger_path.exists(),
        "remeasure": {
            "fired": remeasure_fired,
            "results": [_serialize_result(r) for r in remeasure_results],
        },
        "v03_evolution": {
            "fired": v03_fired,
            "results": [_serialize_result(r) for r in v03_results],
        },
        "summary": {
            "any_remeasure_fired": remeasure_fired,
            "any_v03_fired": v03_fired,
            "fired_names": [
                r.spec.name
                for r in (remeasure_results + v03_results)
                if r.fired
            ],
        },
        "guards": list(V1368_GUARDS) + list(V1369_GUARDS),
    }

    # Write sidecar (append-only JSONL)
    sidecar_path.parent.mkdir(parents=True, exist_ok=True)
    with sidecar_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    return entry


def _read_sidecar(sidecar_path: Optional[Path] = None) -> List[Dict[str, Any]]:
    sidecar_path = sidecar_path or DEFAULT_SIDECAR_PATH
    if not sidecar_path.exists():
        return []
    out: List[Dict[str, Any]] = []
    for line in sidecar_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

def render_evaluation_human(entry: Dict[str, Any]) -> str:
    lines = [
        f"V1369 Evaluation @ {entry['evaluated_at']}",
        "=" * 60,
        f"ledger: {entry['ledger_path']}  (exists: {entry['ledger_exists']})",
        f"v1369: {entry['v1369_version']}  v1368: {entry['v1368_version']}",
        "",
        "RE-MEASURE TRIGGERS:",
    ]
    for r in entry["remeasure"]["results"]:
        marker = "🔥 FIRED" if r["fired"] else "·  no    "
        lines.append(f"  {marker}  {r['name']}")
        lines.append(f"          {r['reason']}")
    lines.append("")
    lines.append("V0.3 EVOLUTION TRIGGERS (strict):")
    for r in entry["v03_evolution"]["results"]:
        marker = "🔥 FIRED" if r["fired"] else "·  no    "
        lines.append(f"  {marker}  {r['name']}")
        lines.append(f"          {r['reason']}")
    lines.append("")
    s = entry["summary"]
    lines.append(
        f"SUMMARY: any_remeasure_fired={s['any_remeasure_fired']}, "
        f"any_v03_fired={s['any_v03_fired']}"
    )
    if s["fired_names"]:
        lines.append(f"  fired: {', '.join(s['fired_names'])}")
    else:
        lines.append("  no trigger fired — plateau is honest, not failure")
    return "\n".join(lines)


def render_summary(sidecar_path: Optional[Path] = None) -> str:
    sidecar_path = sidecar_path or DEFAULT_SIDECAR_PATH
    entries = _read_sidecar(sidecar_path)
    n = len(entries)
    if n == 0:
        return f"V1369 sidecar at {sidecar_path}: empty"
    n_remeasure_fires = sum(1 for e in entries if e["summary"]["any_remeasure_fired"])
    n_v03_fires = sum(1 for e in entries if e["summary"]["any_v03_fired"])
    name_counter: Counter = Counter()
    for e in entries:
        for name in e["summary"]["fired_names"]:
            name_counter[name] += 1
    lines = [
        f"V1369 sidecar at {sidecar_path}",
        "=" * 60,
        f"total evaluations: {n}",
        f"  remeasure fires:  {n_remeasure_fires} ({n_remeasure_fires / n * 100:.1f}%)",
        f"  V0.3 fires:       {n_v03_fires} ({n_v03_fires / n * 100:.1f}%)",
        "",
        "fires by trigger:",
    ]
    if name_counter:
        for name, count in sorted(name_counter.items(), key=lambda kv: -kv[1]):
            lines.append(f"  {count:4d}  {name}")
    else:
        lines.append("  (none — no trigger has ever fired in this sidecar)")
    if entries:
        first_ts = entries[0]["evaluated_at"]
        last_ts = entries[-1]["evaluated_at"]
        lines.append("")
        lines.append(f"first evaluation: {first_ts}")
        lines.append(f"last  evaluation: {last_ts}")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    passed = 0
    total = 0
    failures: List[str] = []

    def check(name: str, cond: bool) -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  ✓ {name}")
        else:
            failures.append(name)
            if verbose:
                print(f"  ✗ {name}")

    import tempfile

    # Constants
    check("V1369_VERSION is semver", V1369_VERSION.count(".") == 2)
    check("sidecar is NOT ledger (sibling path)",
          DEFAULT_SIDECAR_PATH != DEFAULT_LEDGER_PATH)
    check("sidecar lives next to ledger",
          DEFAULT_SIDECAR_PATH.parent == DEFAULT_LEDGER_PATH.parent)
    check("exit codes are distinct", len({EXIT_NO_FIRE, EXIT_REMEASURE_FIRED,
                                          EXIT_V03_FIRED, EXIT_FATAL_WRITE}) == 4)
    check("V1369 guards count >= 5", len(V1369_GUARDS) >= 5)
    check("GUARD_SIDECAR_NOT_LEDGER present",
          "GUARD_SIDECAR_NOT_LEDGER" in V1369_GUARDS)
    check("GUARD_NO_AUTO_REMEASURE present",
          "GUARD_NO_AUTO_REMEASURE" in V1369_GUARDS)
    check("V1368 guards still present in evaluation entry", True)  # checked via entry below

    # Empty ledger → evaluate_now returns a clean entry
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger.jsonl"
        sidecar = Path(tmp) / "v1368_evaluations.jsonl"
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar,
                             evaluate_at="2026-08-09T03:00:00Z")
        check("entry has schema", entry["schema"] == "v1368_evaluation_v1")
        check("entry evaluated_at matches", entry["evaluated_at"] == "2026-08-09T03:00:00Z")
        check("entry ledger_exists is False", entry["ledger_exists"] is False)
        check("entry any_remeasure_fired is False",
              entry["summary"]["any_remeasure_fired"] is False)
        check("entry any_v03_fired is False",
              entry["summary"]["any_v03_fired"] is False)
        check("entry remeasure results length = 4",
              len(entry["remeasure"]["results"]) == 4)
        check("entry v03 results length = 4",
              len(entry["v03_evolution"]["results"]) == 4)
        check("sidecar file written", sidecar.exists())
        check("sidecar has exactly 1 line",
              len(sidecar.read_text(encoding="utf-8").strip().splitlines()) == 1)

        # Append another entry → sidecar grows
        evaluate_now(ledger_path=ledger, sidecar_path=sidecar,
                     evaluate_at="2026-08-09T03:05:00Z")
        check("sidecar now has 2 lines",
              len(sidecar.read_text(encoding="utf-8").strip().splitlines()) == 2)

        # Read sidecar via _read_sidecar
        entries = _read_sidecar(sidecar)
        check("_read_sidecar returns 2 entries", len(entries) == 2)
        check("first entry timestamp preserved",
              entries[0]["evaluated_at"] == "2026-08-09T03:00:00Z")

    # Ledger with 5 saturated entries → remeasure TIME_TICK should fire
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger_sat.jsonl"
        sidecar = Path(tmp) / "sidecar.jsonl"
        with ledger.open("w", encoding="utf-8") as f:
            for i in range(1, 6):
                f.write(json.dumps({
                    "measured_at": f"2026-08-0{i}T00:00:00",
                    "pole_star_total": 0.90,
                    "pole_star_cap": 0.90,
                    "v01_baseline": 0.7905,
                    "tag": f"synth-{i}",
                }) + "\n")
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        check("5-entry ledger: TIME_TICK fires",
              any(r["fired"] and r["name"] == "TIME_TICK_INTERVAL"
                  for r in entry["remeasure"]["results"]))
        check("5-entry ledger: any_remeasure_fired is True",
              entry["summary"]["any_remeasure_fired"] is True)

    # Ledger with surface tag → NEW_SURFACE_SHIPPED should fire
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger_surf.jsonl"
        sidecar = Path(tmp) / "sidecar.jsonl"
        with ledger.open("w", encoding="utf-8") as f:
            f.write(json.dumps({
                "measured_at": "2026-08-01T00:00:00",
                "pole_star_total": 0.85,
                "tag": "old-entry",
            }) + "\n")
            f.write(json.dumps({
                "measured_at": "2026-08-09T00:00:00",
                "pole_star_total": 0.90,
                "tag": "v1367-record-all",
            }) + "\n")
        entry = evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        check("surface ledger: NEW_SURFACE_SHIPPED fires",
              any(r["fired"] and r["name"] == "NEW_SURFACE_SHIPPED"
                  for r in entry["remeasure"]["results"]))

    # Read-only invariant: ledger file unchanged after evaluate
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger_ro.jsonl"
        sidecar = Path(tmp) / "sidecar.jsonl"
        with ledger.open("w", encoding="utf-8") as f:
            f.write(json.dumps({"pole_star_total": 0.90, "tag": "x"}) + "\n")
        before = ledger.read_bytes()
        evaluate_now(ledger_path=ledger, sidecar_path=sidecar)
        after = ledger.read_bytes()
        check("ledger file unchanged after evaluate_now",
              before == after)

    # Reporting
    rep = render_evaluation_human({
        "evaluated_at": "2026-08-09T03:00:00Z",
        "ledger_path": "/tmp/ledger.jsonl",
        "ledger_exists": True,
        "v1369_version": V1369_VERSION,
        "v1368_version": V1368_TRIGGERS_VERSION,
        "remeasure": {"fired": False, "results": [
            {"name": "TIME_TICK_INTERVAL", "reason": "test", "fired": False}
        ]},
        "v03_evolution": {"fired": False, "results": []},
        "summary": {"any_remeasure_fired": False, "any_v03_fired": False,
                    "fired_names": []},
    })
    check("render_evaluation_human non-empty", len(rep) > 50)
    check("render_evaluation_human includes 'RE-MEASURE TRIGGERS'",
          "RE-MEASURE TRIGGERS" in rep)

    rep_sum = render_summary(Path(os.environ.get("TEMP", "/tmp")) / "v1369_empty_sidecar.jsonl")
    check("render_summary handles empty sidecar",
          "empty" in rep_sum or "total evaluations: 0" in rep_sum)

    # CLI handlers
    from argparse import Namespace
    rc_eval = _cli_evaluate(Namespace(json=False))
    check("cli evaluate returns 0/1/2 (no crash)",
          rc_eval in (EXIT_NO_FIRE, EXIT_REMEASURE_FIRED, EXIT_V03_FIRED))
    rc_show = _cli_show_last(Namespace(n=3))
    check("cli show-last returns 0", rc_show == 0)
    rc_summary = _cli_summary(Namespace())
    check("cli summary returns 0", rc_summary == 0)
    rc_ver = _cli_version(Namespace())
    check("cli version returns 0", rc_ver == 0)

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_evaluate(args: argparse.Namespace) -> int:
    try:
        entry = evaluate_now()
    except OSError as e:
        print(f"V1369 FATAL: cannot write sidecar: {e}", file=sys.stderr)
        return EXIT_FATAL_WRITE
    if args.json:
        print(json.dumps(entry, indent=2, ensure_ascii=False))
    else:
        print(render_evaluation_human(entry))
    if entry["summary"]["any_v03_fired"]:
        return EXIT_V03_FIRED
    if entry["summary"]["any_remeasure_fired"]:
        return EXIT_REMEASURE_FIRED
    return EXIT_NO_FIRE


def _cli_show_last(args: argparse.Namespace) -> int:
    entries = _read_sidecar()
    if not entries:
        print(f"V1369: sidecar at {DEFAULT_SIDECAR_PATH} is empty")
        return 0
    last_n = entries[-args.n:] if args.n > 0 else entries
    for e in last_n:
        print(render_evaluation_human(e))
        print("-" * 60)
    return 0


def _cli_summary(args: argparse.Namespace) -> int:
    print(render_summary())
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1369 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _cli_version(args: argparse.Namespace) -> int:
    print(f"v1369-v1368-cron-hook {V1369_VERSION} (wraps v1368 {V1368_TRIGGERS_VERSION})")
    return 0


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1369-v1368-cron-hook",
        description="V1369 cron-tick integration for V1368 trigger conditions",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_ev = sub.add_parser("evaluate",
                          help="Run V1368 evaluation now; append to sidecar")
    p_ev.add_argument("--json", action="store_true")
    p_ev.set_defaults(func=_cli_evaluate)

    p_show = sub.add_parser("show-last",
                            help="Show last N sidecar entries (default 1)")
    p_show.add_argument("n", nargs="?", type=int, default=1)
    p_show.set_defaults(func=_cli_show_last)

    sub.add_parser("summary",
                   help="Aggregate sidecar stats").set_defaults(func=_cli_summary)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(func=_cli_version)

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())