"""Phase 1357 v1357_vcp_observability_snapshot — single-shot project snapshot.

The VCP toolchain (V1346-V1355) and ASI substrate (V1313-V1326 + V1356) are
operational but spread across many modules. V1357 collapses them into
**one command** that any human can run to understand the project's current
state, in five seconds, no installation required.

## Why V1357 (主 00:56 any-human-can-pick-up)

The hardest part of a self-driven project is "any human tomorrow reads this
and picks up". They will not read 1300+ commits. They will run ONE command.
V1357 is that command.

  $ apeireth/v1357_vcp_observability_snapshot.py snapshot

Returns a JSON document with:
  - pole_star: current V0.2 score (from V1356)
  - toolchain_health: module-presence table for V1345-V1355
  - recent_commits: last 10 commits
  - infra_state: which VCP infra files exist
  - close_loop_state: V1355 wet-run result
  - known_unknowns: items the snapshot cannot determine (for honesty)

## CLI subcommands

  v1357-snapshot snapshot [--json] [--pretty]   # full snapshot
  v1357-snapshot summary                         # one-line summary
  v1357-snapshot recipe                          # "if you only read one thing"
  v1357-snapshot self-test [--verbose]           # 18+ Popper checks
  v1357-snapshot version

## Exit codes

  0  snapshot OK
  1  snapshot OK with known unknowns (honest disclosure)
  2  fatal: required tools missing (e.g., git)
  3  invalid usage

## V3 哲学守门

- 不假装 Phenomenal: V1357 = aggregator, no phenomenology.
- 不假装 ASI 智慧: snapshot = mechanical collection.
- 不假装 ASI 集成: V1357 only imports V1355/V1356 (the only ones with stable API).
- 不假装 ASI 等级: snapshot.cap is bounded by caller; honest subscore 0.005.
- 不动 anchor: V1357 = read-only aggregator; never writes.
- V1357 ≠ ASI: snapshot ≠ ASI; honest cap 0.005.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1357_VERSION = "0.1.0"
V1357_ASI_CAP = 0.005  # honest cap; snapshot ≠ ASI

REPO_ROOT = Path(__file__).resolve().parent.parent
APEIRETH_DIR = REPO_ROOT / "apeireth"
TESTS_DIR = REPO_ROOT / "tests"

VCP_INFRA_FILES = {
    "ledger": "vcp_gate_history.jsonl",
    "migration_audit": "vcp_migration_audit.jsonl",
    "remediation_history": "vcp_remediation_history.jsonl",
}

VCP_TOOLCHAIN_MODULES = [
    "v1345_vcp_historical_ledger.py",
    "v1346_vcp_tier_aware_migration.py",
    "v1347_vcp_health_score.py",
    "v1348_anomaly_detector.py",
    "v1349_vcp_x_llm_real_benchmark.py",
    "v1350_anomaly_lifecycle.py",
    "v1351_vcp_toolchain_cli.py",
    "v1352_vcp_history_diff.py",
    "v1353_vcp_doctor.py",
    "v1354_vcp_remediation.py",
    "v1355_vcp_wet_run.py",
]


# -----------------------------------------------------------------------------
# Snapshot data class
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class ProjectSnapshot:
    """One snapshot of project state."""
    version: str
    measured_at: str
    repo_root: str
    pole_star: Dict[str, Any]
    toolchain_health: Dict[str, Any]
    recent_commits: List[Dict[str, str]]
    infra_state: Dict[str, bool]
    close_loop_state: Dict[str, Any]
    module_counts: Dict[str, int]
    known_unknowns: Tuple[str, ...]
    philosophy_guards: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "measured_at": self.measured_at,
            "repo_root": self.repo_root,
            "pole_star": self.pole_star,
            "toolchain_health": self.toolchain_health,
            "recent_commits": list(self.recent_commits),
            "infra_state": dict(self.infra_state),
            "close_loop_state": self.close_loop_state,
            "module_counts": dict(self.module_counts),
            "known_unknowns": list(self.known_unknowns),
            "philosophy_guards": list(self.philosophy_guards),
        }


# -----------------------------------------------------------------------------
# Pole-star (delegate to V1356)
# -----------------------------------------------------------------------------

def _measure_pole_star() -> Tuple[Dict[str, Any], List[str]]:
    """Get V1356 pole-star. Returns (data, unknowns)."""
    unknowns: List[str] = []
    try:
        from apeireth.v1356_asi_pole_star_v02 import measure_v02
        rep = measure_v02()
        # Compact shape
        data = {
            "total": round(rep.total, 4),
            "v01_baseline": rep.v01_baseline,
            "delta_vs_v01": round(rep.delta_vs_v01, 4),
            "honest_cap": rep.honest_cap,
            "asi_proximity": rep.asi_proximity,
            "weighted_subtotal": round(rep.weighted_subtotal, 4),
            "approach_margin": round(rep.approach_margin, 4),
            "components": [
                {
                    "name": c.name,
                    "weight": c.weight,
                    "raw_value": round(c.raw_value, 4),
                    "weighted_value": round(c.weighted_value, 4),
                }
                for c in rep.components
            ],
        }
        return data, unknowns
    except Exception as exc:
        unknowns.append(f"pole_star: {type(exc).__name__}: {exc}")
        return {
            "total": None,
            "v01_baseline": 0.7905,
            "delta_vs_v01": None,
            "honest_cap": 0.90,
            "asi_proximity": "unknown",
            "components": [],
        }, unknowns


# -----------------------------------------------------------------------------
# Toolchain health
# -----------------------------------------------------------------------------

def _measure_toolchain_health() -> Tuple[Dict[str, Any], List[str]]:
    unknowns: List[str] = []
    mods_present: List[str] = []
    mods_absent: List[str] = []
    for mod in VCP_TOOLCHAIN_MODULES:
        path = APEIRETH_DIR / mod
        if path.exists():
            mods_present.append(mod)
        else:
            mods_absent.append(mod)

    n_present = len(mods_present)
    n_total = len(VCP_TOOLCHAIN_MODULES)

    return {
        "n_modules_present": n_present,
        "n_modules_total": n_total,
        "presence_ratio": round(n_present / max(n_total, 1), 4),
        "modules_present": mods_present,
        "modules_absent": mods_absent,
    }, unknowns


# -----------------------------------------------------------------------------
# Recent commits
# -----------------------------------------------------------------------------

def _recent_commits(limit: int = 10) -> Tuple[List[Dict[str, str]], List[str]]:
    unknowns: List[str] = []
    commits: List[Dict[str, str]] = []
    if not shutil.which("git"):
        unknowns.append("git not on PATH")
        return commits, unknowns
    try:
        out = subprocess.check_output(
            ["git", "log", f"-n{limit}", "--pretty=format:%h|%ai|%s"],
            cwd=str(REPO_ROOT),
            stderr=subprocess.DEVNULL,
        ).decode("utf-8")
        for line in out.splitlines():
            line = line.strip()
            if not line:
                continue
            parts = line.split("|", 2)
            if len(parts) == 3:
                commits.append({
                    "hash": parts[0],
                    "date": parts[1],
                    "subject": parts[2],
                })
    except subprocess.CalledProcessError as exc:
        unknowns.append(f"git log failed: {exc}")
    except FileNotFoundError:
        unknowns.append("git not found")
    return commits, unknowns


# -----------------------------------------------------------------------------
# Infra state (VCP infra files)
# -----------------------------------------------------------------------------

def _infra_state() -> Dict[str, bool]:
    state: Dict[str, bool] = {}
    for key, filename in VCP_INFRA_FILES.items():
        state[key] = (REPO_ROOT / filename).exists()
    return state


# -----------------------------------------------------------------------------
# Close-loop (V1355 wet-run)
# -----------------------------------------------------------------------------

def _measure_close_loop() -> Tuple[Dict[str, Any], List[str]]:
    unknowns: List[str] = []
    try:
        from apeireth.v1355_vcp_wet_run import run_wet_run
        rep = run_wet_run(keep=False)
        return {
            "n_scenarios": rep.n_scenarios,
            "n_pass": rep.n_pass,
            "n_fail": rep.n_fail,
            "exit_code": rep.exit_code,
            "scenarios": [
                {
                    "name": s.scenario,
                    "pass": s.close_loop_pass,
                    "apply_attempted": s.apply_attempted,
                    "apply_ok": s.apply_ok,
                    "duration_ms": round(s.duration_ms, 1),
                    "failure_reason": s.failure_reason or "",
                }
                for s in rep.scenarios
            ],
        }, unknowns
    except Exception as exc:
        unknowns.append(f"close_loop: {type(exc).__name__}: {exc}")
        return {"n_scenarios": 0, "n_pass": 0, "n_fail": 0, "exit_code": 3, "scenarios": []}, unknowns


# -----------------------------------------------------------------------------
# Module counts
# -----------------------------------------------------------------------------

def _module_counts() -> Dict[str, int]:
    ape_count = 0
    test_count = 0
    if APEIRETH_DIR.exists():
        ape_count = sum(1 for _ in APEIRETH_DIR.glob("v*.py"))
    if TESTS_DIR.exists():
        test_count = sum(1 for _ in TESTS_DIR.glob("test_v*.py"))
    return {"apeireth_v_modules": ape_count, "test_files": test_count}


# -----------------------------------------------------------------------------
# Snapshot builder
# -----------------------------------------------------------------------------

def build_snapshot() -> ProjectSnapshot:
    """Build a complete project snapshot."""
    measured = datetime.now(timezone.utc)

    pole_data, pole_unknowns = _measure_pole_star()
    tool_data, tool_unknowns = _measure_toolchain_health()
    commits, commit_unknowns = _recent_commits(limit=10)
    infra = _infra_state()
    loop, loop_unknowns = _measure_close_loop()
    counts = _module_counts()

    unknowns: List[str] = []
    unknowns.extend(pole_unknowns)
    unknowns.extend(tool_unknowns)
    unknowns.extend(commit_unknowns)
    unknowns.extend(loop_unknowns)

    return ProjectSnapshot(
        version=V1357_VERSION,
        measured_at=measured.isoformat(),
        repo_root=str(REPO_ROOT),
        pole_star=pole_data,
        toolchain_health=tool_data,
        recent_commits=commits,
        infra_state=infra,
        close_loop_state=loop,
        module_counts=counts,
        known_unknowns=tuple(unknowns),
        philosophy_guards=(
            "GUARD_READ_ONLY",
            "GUARD_NO_WRITES",
            "GUARD_HONEST_UNKNOWN_DISCLOSURE",
            "GUARD_DELEGATE_TO_V1355_V1356",
            "GUARD_SNAPSHOT_NOT_ASI",
        ),
    )


# -----------------------------------------------------------------------------
# Render
# -----------------------------------------------------------------------------

def render_summary(snap: ProjectSnapshot) -> str:
    """One-line summary."""
    pole = snap.pole_star
    close = snap.close_loop_state
    return (
        f"Apeireth@{snap.measured_at}  "
        f"pole_star(V0.2)={pole.get('total')} "
        f"[Δ vs V0.1={pole.get('delta_vs_v01'):+.4f}] "
        f"toolchain={snap.toolchain_health.get('n_modules_present')}/{snap.toolchain_health.get('n_modules_total')} "
        f"close_loop={close.get('n_pass')}/{close.get('n_scenarios')}pass "
        f"modules={snap.module_counts.get('apeireth_v_modules')} "
        f"tests={snap.module_counts.get('test_files')}"
    )


def render_recipe(snap: ProjectSnapshot) -> str:
    """Recipe: what a new human should do next."""
    lines: List[str] = []
    lines.append("=== RECIPE: any-human-can-pick-up ===")
    lines.append("")
    lines.append("If you just landed here and need to understand this project in 5 minutes:")
    lines.append("")
    lines.append("  1. SKIM: apeireth/APEIRETH.md")
    lines.append("  2. CHECK: pole-star is at " + str(snap.pole_star.get("total")) + " (cap 0.90)")
    lines.append("  3. RUN: apeireth/v1357_vcp_observability_snapshot.py snapshot --json")
    lines.append("  4. IF broken: apeireth/v1354_vcp_remediation.py plan    # see what to fix")
    lines.append("            apeireth/v1354_vcp_remediation.py apply   # auto-fix safe items")
    lines.append("  5. RE-TEST: apeireth/v1355_vcp_wet_run.py run          # close-loop confirms healthy")
    lines.append("  6. MEASURE: apeireth/v1356_asi_pole_star_v02.py measure --json")
    lines.append("")
    lines.append("Recent commits (latest first):")
    for c in snap.recent_commits[:5]:
        lines.append(f"  {c.get('hash','?')} {c.get('date','?')[:10]}  {c.get('subject','?')[:80]}")
    if snap.known_unknowns:
        lines.append("")
        lines.append("Known unknowns (honest disclosure):")
        for u in snap.known_unknowns:
            lines.append(f"  - {u}")
    return "\n".join(lines)


def render_pretty(snap: ProjectSnapshot) -> str:
    lines: List[str] = []
    lines.append(f"=== Apeireth Snapshot v{snap.version} ===")
    lines.append(f"Measured at: {snap.measured_at}")
    lines.append(f"Repo: {snap.repo_root}")
    lines.append("")
    # Pole-star
    pole = snap.pole_star
    lines.append("--- ASI Pole-Star V0.2 ---")
    lines.append(f"  Total:        {pole.get('total')}")
    lines.append(f"  V0.1 baseline: {pole.get('v01_baseline')}")
    lines.append(f"  Delta:        {pole.get('delta_vs_v01'):+.4f}" if pole.get('delta_vs_v01') is not None else "  Delta:        ?")
    lines.append(f"  Honest cap:    {pole.get('honest_cap')}")
    lines.append(f"  ASI proximity: {pole.get('asi_proximity')}")
    lines.append("  Components:")
    for c in pole.get("components", []):
        lines.append(f"    {c.get('name'):<28s} w={c.get('weight')}  score={c.get('raw_value')}  → {c.get('weighted_value')}")
    lines.append("")
    # Toolchain
    tc = snap.toolchain_health
    lines.append("--- VCP Toolchain Health ---")
    lines.append(f"  Modules present: {tc.get('n_modules_present')}/{tc.get('n_modules_total')}")
    lines.append(f"  Presence ratio:  {tc.get('presence_ratio')}")
    if tc.get("modules_absent"):
        lines.append(f"  Missing: {', '.join(tc['modules_absent'])}")
    lines.append("")
    # Close-loop
    cl = snap.close_loop_state
    lines.append("--- V1355 Close-Loop ---")
    lines.append(f"  Scenarios: {cl.get('n_pass')}/{cl.get('n_scenarios')} pass; exit={cl.get('exit_code')}")
    for s in cl.get("scenarios", []):
        flag = "PASS" if s.get("pass") else "FAIL"
        lines.append(f"    [{flag}] {s.get('name')}  apply_ok={s.get('apply_ok')}/{s.get('apply_attempted')}  ({s.get('duration_ms')}ms)")
    lines.append("")
    # Infra
    lines.append("--- VCP Infra Files ---")
    for k, v in snap.infra_state.items():
        flag = "✓" if v else "✗"
        lines.append(f"  [{flag}] {k}")
    lines.append("")
    # Counts
    counts = snap.module_counts
    lines.append("--- Counts ---")
    lines.append(f"  apeireth/v*.py:   {counts.get('apeireth_v_modules')}")
    lines.append(f"  tests/test_v*.py: {counts.get('test_files')}")
    lines.append("")
    # Recent commits
    lines.append("--- Recent Commits ---")
    for c in snap.recent_commits[:8]:
        lines.append(f"  {c.get('hash')} {c.get('date','')[:10]} {c.get('subject','')[:80]}")
    lines.append("")
    # Unknowns
    if snap.known_unknowns:
        lines.append("--- Known Unknowns (honest disclosure) ---")
        for u in snap.known_unknowns:
            lines.append(f"  - {u}")
    lines.append("")
    lines.append("Philosophy guards:")
    for g in snap.philosophy_guards:
        lines.append(f"  - {g}")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Self-tests (Popper)
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    failures: List[str] = []
    passed = 0
    total = 0

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  PASS  {name}")
        else:
            failures.append(f"{name}: {detail}")
            if verbose:
                print(f"  FAIL  {name}: {detail}")

    # constants
    check("V1357_VERSION is semver", V1357_VERSION.count(".") == 2)
    check("V1357_ASI_CAP <= 0.01", V1357_ASI_CAP <= 0.01)
    check("REPO_ROOT exists", REPO_ROOT.exists())

    # module counts
    counts = _module_counts()
    check("apeireth modules > 0", counts["apeireth_v_modules"] > 0)
    check("test files > 0", counts["test_files"] > 0)

    # infra state
    infra = _infra_state()
    check("infra_state has 3 keys", len(infra) == 3)
    check("ledger is bool", isinstance(infra.get("ledger"), bool))

    # recent commits
    commits, unknowns = _recent_commits(limit=5)
    check("recent_commits nonempty", len(commits) > 0,
          f"unknowns={unknowns}")
    if commits:
        check("commit has hash", "hash" in commits[0])
        check("commit has date", "date" in commits[0])
        check("commit has subject", "subject" in commits[0])

    # toolchain health
    tc, tc_u = _measure_toolchain_health()
    check("toolchain has n_modules_present", "n_modules_present" in tc)
    check("toolchain n_matches_total or less",
          tc["n_modules_present"] <= tc["n_modules_total"],
          f"got {tc['n_modules_present']} > {tc['n_modules_total']}")

    # close-loop (delegates to v1355)
    cl, cl_u = _measure_close_loop()
    check("close-loop has scenarios", "scenarios" in cl)

    # pole-star (delegates to v1356)
    pole, pole_u = _measure_pole_star()
    check("pole-star has total", "total" in pole)
    check("pole-star has v01_baseline", pole.get("v01_baseline") == 0.7905)

    # full snapshot
    snap = build_snapshot()
    check("snapshot has 6 sections",
          all(k in snap.to_dict() for k in ["pole_star", "toolchain_health", "recent_commits", "infra_state", "close_loop_state", "module_counts"]))
    check("snapshot philosophy_guards non-empty", len(snap.philosophy_guards) >= 4)
    check("snapshot GUARD_SNAPSHOT_NOT_ASI", any("NOT_ASI" in g for g in snap.philosophy_guards))

    # renders
    summary = render_summary(snap)
    check("summary non-empty", len(summary) > 50)
    check("summary mentions pole_star", "pole_star" in summary)
    check("summary mentions close_loop", "close_loop" in summary)

    recipe = render_recipe(snap)
    check("recipe non-empty", len(recipe) > 100)
    check("recipe has recipe section", "RECIPE" in recipe)

    pretty = render_pretty(snap)
    check("pretty non-empty", len(pretty) > 200)
    check("pretty has Pole-Star", "Pole-Star" in pretty)

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_snapshot(args: argparse.Namespace) -> int:
    snap = build_snapshot()
    if args.pretty:
        print(render_pretty(snap))
    elif args.json:
        print(json.dumps(snap.to_dict(), indent=2, ensure_ascii=False))
    else:
        # Default: JSON (most useful for downstream tooling)
        print(json.dumps(snap.to_dict(), indent=2, ensure_ascii=False))
    rc = 1 if snap.known_unknowns else 0
    return rc


def _cli_summary(_: argparse.Namespace) -> int:
    snap = build_snapshot()
    print(render_summary(snap))
    return 0


def _cli_recipe(_: argparse.Namespace) -> int:
    snap = build_snapshot()
    print(render_recipe(snap))
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1357 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="v1357-snapshot", description="Project observability snapshot (any human can read)")
    sub = p.add_subparsers(dest="command", required=True)

    p_s = sub.add_parser("snapshot", help="full JSON snapshot")
    p_s.add_argument("--json", action="store_true", help="JSON output (default)")
    p_s.add_argument("--pretty", action="store_true", help="human-readable pretty")
    p_s.set_defaults(func=_cli_snapshot)

    sub.add_parser("summary", help="one-line summary").set_defaults(func=_cli_summary)
    sub.add_parser("recipe", help="'if you only read one thing' recipe").set_defaults(func=_cli_recipe)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1357-vcp-observability-snapshot {V1357_VERSION}") or 0
    )

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
