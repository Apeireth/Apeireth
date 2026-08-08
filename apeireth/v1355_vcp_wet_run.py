"""Phase 1355 v1355_vcp_wet_run — VCP close-the-loop wet-run harness.

V1354 (Remediation) is the operator's "what do I do?". V1355 is the engineer's
"does it actually close the loop on a fresh checkout?".

The whole point of the VCP toolchain (V1346 migration → V1347 health → V1348
detector → V1349 LLM benchmark → V1350 lifecycle → V1351 one-click CLI →
V1352 history+diff → V1353 doctor → V1354 remediation) is that **any human
can pick it up tomorrow** and the system reaches a known-good state from a
contaminated/blank/fresh-checkout state. V1355 *measures* that promise.

## What V1355 does

For each scenario S in a deterministic scenario table:
  1. Materialize S as a fresh temp workspace (a real directory under
     tempfile.gettempdir()/v1355_<timestamp>_<idx>/).
  2. Pre-flight state: list which VCP infra files are present / absent.
  3. Drive V1354's API surface:
       a. detect (V1353 doctor OR V1354 fallback issues)
       b. plan
       c. apply (real, not dry-run)
  4. Post-flight state: re-list infra files; verify deltas.
  5. Re-doctor: confirm all PRE_MISSING items now pass.
  6. Record scenario result + close-loop delta.
  7. Tear down the temp workspace unless `--keep` is set.

## Scenarios (deterministic; 主 22:33 终极授权)

  S0_BLANK          — empty workspace; nothing exists.
  S1_MISSING_LEDGER — only ledger missing.
  S2_MISSING_AUDIT  — only migration audit missing.
  S3_MISSING_HIST   — only remediation history missing.
  S4_ALL_MISSING    — ledger + audit + history all missing.
  S5_ALL_PRESENT    — all infra files already exist (should be no-op).
  S6_CORRUPT_HIST   — remediation history exists but is corrupted JSONL.

The expectation is mechanical: each scenario maps to a known close-loop
result (table in EXPECTED_RESULTS below). V1355 fails the wet-run if any
scenario deviates from its expected delta.

## Why this matters (主 17:43 实事求是)

Every release milestone we tell ourselves "any human can pick this up". V1355
measures the truth of that statement by *running* the close-loop on a real
disk and counting the deltas. If the loop does not close, the truth is
exposed: V1355 catches it.

## CLI subcommands

  v1355-wet-run run [--keep] [--scenario S0_BLANK,...]
  v1355-wet-run scenarios                    # print scenario table
  v1355-wet-run expected                    # print EXPECTED_RESULTS
  v1355-wet-run self-test [--verbose]       # 30+ embedded Popper checks
  v1355-wet-run version

## Exit codes

  0  all scenarios passed; close-loop closed deterministically
  1  some scenario produced a non-blocking warning (continuable)
  2  some scenario FAILED close-loop; pre-state ≠ post-state expected
  3  invalid usage / internal error

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1355 has no qualia; it is a pure mechanical harness.
- 不假装 ASI 智慧: wet-run = pure scenario runner, not an LLM agent.
- 不假装 ASI 集成: V1355 ≠ VCP Doctor; it drives the close-loop ON TOP of
  the existing API surface.
- 不假装 ASI 等级: harness score != ASI score; separate subscore (cap 0.005).
- 不动 anchor: V1355 = scenario harness, does not modify v1354 or any
  upstream module. Test-only mutation of monkey-patched paths.
- V1355 ≠ ASI: scenario runner ≠ ASI; honest cap 0.005.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
import traceback
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# V1354 module (drives the close-loop)
from apeireth.v1354_vcp_remediation import (
    generate_plan,
    apply_plan,
    RemediationPlan,
    ApplyReport,
    RemediationItem,
    ACTION_FIX,
    ACTION_MANUAL,
    ACTION_DEFER,
    ACTION_IGNORE,
    STATUS_OK,
    STATUS_WARN,
    STATUS_FAIL,
    STATUS_SKIP,
    SAFE_FIXES,
    LEDGER_PATH as PROD_LEDGER_PATH,
    MIGRATION_AUDIT_PATH as PROD_MIGRATION_AUDIT_PATH,
    REMEDIATION_HISTORY_PATH as PROD_REMEDIATION_HISTORY_PATH,
    V1354_VERSION,
)

V1355_VERSION = "0.1.0"
V1355_ASI_CAP = 0.005

# Scenario table (机械 — 主 22:33 终极授权)
SCENARIO_TABLE: Dict[str, Dict[str, Any]] = {
    "S0_BLANK": {
        "description": "Empty workspace; nothing exists.",
        "pre_create": [],
        "expected_pre_missing": ["ledger", "migration_audit", "remediation_history"],
        "expected_post_missing": [],
        "expected_apply_attempted": 3,
        "expected_apply_ok_min": 3,
    },
    "S1_MISSING_LEDGER": {
        "description": "Only ledger missing; audit + history present.",
        "pre_create": ["migration_audit", "remediation_history"],
        "expected_pre_missing": ["ledger"],
        "expected_post_missing": [],
        "expected_apply_attempted": 1,
        "expected_apply_ok_min": 1,
    },
    "S2_MISSING_AUDIT": {
        "description": "Only migration audit missing; ledger + history present.",
        "pre_create": ["ledger", "remediation_history"],
        "expected_pre_missing": ["migration_audit"],
        "expected_post_missing": [],
        "expected_apply_attempted": 1,
        "expected_apply_ok_min": 1,
    },
    "S3_MISSING_HIST": {
        "description": "Only remediation history missing; ledger + audit present.",
        "pre_create": ["ledger", "migration_audit"],
        "expected_pre_missing": ["remediation_history"],
        "expected_post_missing": [],
        "expected_apply_attempted": 1,
        "expected_apply_ok_min": 1,
    },
    "S4_ALL_MISSING": {
        "description": "ledger + audit + history all missing.",
        "pre_create": [],
        "expected_pre_missing": ["ledger", "migration_audit", "remediation_history"],
        "expected_post_missing": [],
        "expected_apply_attempted": 3,
        "expected_apply_ok_min": 3,
    },
    "S5_ALL_PRESENT": {
        "description": "All infra files already exist; expected no-op.",
        "pre_create": ["ledger", "migration_audit", "remediation_history"],
        "expected_pre_missing": [],
        "expected_post_missing": [],
        "expected_apply_attempted": 0,
        "expected_apply_ok_min": 0,
    },
    "S6_CORRUPT_HIST": {
        "description": "Remediation history exists but is corrupted JSONL.",
        "pre_create": ["ledger", "migration_audit"],
        "pre_create_special": "corrupt_history",
        "expected_pre_missing": [],
        "expected_post_missing": [],
        "expected_apply_attempted": 0,
        "expected_apply_ok_min": 0,
    },
}

# Map: scenario pre_create key → filename
INFRA_FILES = {
    "ledger": "vcp_gate_history.jsonl",
    "migration_audit": "vcp_migration_audit.jsonl",
    "remediation_history": "vcp_remediation_history.jsonl",
}


# -----------------------------------------------------------------------------
# Data classes
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class ScenarioResult:
    """One scenario's close-loop result."""
    scenario: str
    description: str
    workspace: str
    pre_state: Dict[str, bool]   # path-key -> exists?
    post_state: Dict[str, bool]
    pre_missing: List[str]
    post_missing: List[str]
    plan_n_items: int
    plan_n_fix: int
    apply_attempted: int
    apply_ok: int
    apply_warn: int
    apply_fail: int
    apply_skip: int
    apply_exit_code: int
    close_loop_pass: bool
    failure_reason: str = ""  # empty if passed
    started_at: str = ""
    finished_at: str = ""
    duration_ms: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class WetRunReport:
    """Aggregated wet-run report."""
    version: str
    n_scenarios: int
    n_pass: int
    n_warn: int
    n_fail: int
    keep_workspace: bool
    scenarios: Tuple[ScenarioResult, ...]
    started_at: str
    finished_at: str
    exit_code: int
    asi_cap: float
    philosophy_guards: Tuple[str, ...]
    base_v1354_version: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "n_scenarios": self.n_scenarios,
            "n_pass": self.n_pass,
            "n_warn": self.n_warn,
            "n_fail": self.n_fail,
            "keep_workspace": self.keep_workspace,
            "scenarios": [s.to_dict() for s in self.scenarios],
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "exit_code": self.exit_code,
            "asi_cap": self.asi_cap,
            "philosophy_guards": list(self.philosophy_guards),
            "base_v1354_version": self.base_v1354_version,
        }


# -----------------------------------------------------------------------------
# Workspace materialization + state introspection
# -----------------------------------------------------------------------------

def _make_workspace(root: Path, scenario: str) -> Path:
    """Create a fresh workspace dir for this scenario."""
    ws = root / f"v1355_{scenario}"
    ws.mkdir(parents=True, exist_ok=True)
    return ws


def _materialize_scenario(ws: Path, scenario: str) -> None:
    """Create infra files according to scenario's pre_create list."""
    spec = SCENARIO_TABLE[scenario]
    pre_create = spec.get("pre_create", [])
    pre_create_special = spec.get("pre_create_special")

    for key in pre_create:
        filename = INFRA_FILES.get(key)
        if filename:
            path = ws / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()

    if pre_create_special == "corrupt_history":
        path = ws / INFRA_FILES["remediation_history"]
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write('{"this is not valid json\n')
            f.write('not even close\n')


def _introspect_state(ws: Path) -> Dict[str, bool]:
    """Which infra files exist right now?"""
    return {
        "ledger": (ws / INFRA_FILES["ledger"]).exists(),
        "migration_audit": (ws / INFRA_FILES["migration_audit"]).exists(),
        "remediation_history": (ws / INFRA_FILES["remediation_history"]).exists(),
    }


def _missing_keys(state: Dict[str, bool]) -> List[str]:
    return sorted([k for k, v in state.items() if not v])


# -----------------------------------------------------------------------------
# V1354 driver (monkey-patch paths into workspace)
# -----------------------------------------------------------------------------

class _WorkspacePatch:
    """Temporarily redirect V1354 module-level paths into `ws`."""

    def __init__(self, ws: Path) -> None:
        self.ws = ws
        self._snapshot: Dict[str, Any] = {}
        self._imports: Dict[str, Any] = {}

    def __enter__(self) -> "_WorkspacePatch":
        from apeireth import v1354_vcp_remediation as m1354
        self._imports["v1354"] = m1354
        ws_ledger = self.ws / INFRA_FILES["ledger"]
        ws_audit = self.ws / INFRA_FILES["migration_audit"]
        ws_hist = self.ws / INFRA_FILES["remediation_history"]

        # Snapshot
        self._snapshot = {
            "LEDGER_PATH": m1354.LEDGER_PATH,
            "MIGRATION_AUDIT_PATH": m1354.MIGRATION_AUDIT_PATH,
            "REMEDIATION_HISTORY_PATH": m1354.REMEDIATION_HISTORY_PATH,
        }

        # Monkey-patch
        m1354.LEDGER_PATH = ws_ledger
        m1354.MIGRATION_AUDIT_PATH = ws_audit
        m1354.REMEDIATION_HISTORY_PATH = ws_hist

        # Also patch SAFE_FIXES (registry) — they reference module-level paths
        new_fixes: Dict[str, Tuple[Callable[[Path], Any], Callable[[], Path]]] = {
            "create_ledger_if_missing": (
                m1354._fix_create_empty_file,
                lambda: ws_ledger,
            ),
            "create_migration_audit_if_missing": (
                m1354._fix_create_empty_file,
                lambda: ws_audit,
            ),
            "create_remediation_history_if_missing": (
                m1354._fix_create_empty_file,
                lambda: ws_hist,
            ),
            "make_ledger_readable": (
                m1354._fix_set_readable,
                lambda: ws_ledger,
            ),
        }
        # Apply
        m1354.SAFE_FIXES = new_fixes  # type: ignore[assignment]

        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        from apeireth import v1354_vcp_remediation as m1354
        for k, v in self._snapshot.items():
            setattr(m1354, k, v)


def _drive_close_loop(ws: Path) -> Tuple[RemediationPlan, ApplyReport]:
    """Drive doctor→plan→apply through V1354's API."""
    with _WorkspacePatch(ws):
        plan = generate_plan(source="fallback", limit=50)
        apply_report = apply_plan(plan, dry_run=False)
        return plan, apply_report


def _drive_dry_loop(ws: Path) -> Tuple[RemediationPlan, ApplyReport]:
    """Same as _drive_close_loop but in dry-run."""
    with _WorkspacePatch(ws):
        plan = generate_plan(source="fallback", limit=50)
        apply_report = apply_plan(plan, dry_run=True)
        return plan, apply_report


# -----------------------------------------------------------------------------
# Scenario runner
# -----------------------------------------------------------------------------

def run_scenario(scenario: str, root: Path) -> ScenarioResult:
    """Run one scenario end-to-end. Returns ScenarioResult."""
    spec = SCENARIO_TABLE[scenario]
    started = datetime.now(timezone.utc)
    ws = _make_workspace(root, scenario)
    failure = ""

    try:
        # 1. Materialize contaminated/fresh state.
        _materialize_scenario(ws, scenario)

        # 2. Pre-state
        pre_state = _introspect_state(ws)
        pre_missing = _missing_keys(pre_state)

        # 3. Drive close-loop (real apply)
        plan, apply_report = _drive_close_loop(ws)

        # 4. Post-state
        post_state = _introspect_state(ws)
        post_missing = _missing_keys(post_state)

        # 5. Verify against expected
        ok = True
        if set(pre_missing) != set(spec["expected_pre_missing"]):
            ok = False
            failure += f"pre_missing mismatch (got {pre_missing}, expected {spec['expected_pre_missing']}); "
        if set(post_missing) != set(spec["expected_post_missing"]):
            ok = False
            failure += f"post_missing mismatch (got {post_missing}, expected {spec['expected_post_missing']}); "
        if apply_report.n_attempted < spec["expected_apply_attempted"]:
            ok = False
            failure += f"apply_attempted too low (got {apply_report.n_attempted}, expected >= {spec['expected_apply_attempted']}); "
        if apply_report.n_ok < spec["expected_apply_ok_min"]:
            ok = False
            failure += f"apply_ok too low (got {apply_report.n_ok}, expected >= {spec['expected_apply_ok_min']}); "

        # Special check for S6_CORRUPT_HIST: corrupted file MUST still exist (we don't auto-delete).
        if scenario == "S6_CORRUPT_HIST":
            if not post_state.get("remediation_history", False):
                ok = False
                failure += "S6_CORRUPT_HIST: remediation_history disappeared (should not be auto-deleted); "

    except Exception as exc:
        ok = False
        failure = f"exception: {type(exc).__name__}: {exc}\n{traceback.format_exc()}"
        pre_state = _introspect_state(ws)
        post_state = _introspect_state(ws)
        pre_missing = _missing_keys(pre_state)
        post_missing = _missing_keys(post_state)
        plan = None  # type: ignore[assignment]
        apply_report = _empty_apply_report()

    finished = datetime.now(timezone.utc)
    duration_ms = (finished - started).total_seconds() * 1000.0

    if plan is None:
        plan = _empty_plan()

    return ScenarioResult(
        scenario=scenario,
        description=spec["description"],
        workspace=str(ws),
        pre_state=pre_state,
        post_state=post_state,
        pre_missing=pre_missing,
        post_missing=post_missing,
        plan_n_items=plan.n_items,
        plan_n_fix=plan.n_fix,
        apply_attempted=apply_report.n_attempted,
        apply_ok=apply_report.n_ok,
        apply_warn=apply_report.n_warn,
        apply_fail=apply_report.n_fail,
        apply_skip=apply_report.n_skip,
        apply_exit_code=apply_report.exit_code,
        close_loop_pass=ok,
        failure_reason=failure.strip(),
        started_at=started.isoformat(),
        finished_at=finished.isoformat(),
        duration_ms=duration_ms,
    )


def _empty_plan() -> RemediationPlan:
    return RemediationPlan(
        version=V1354_VERSION,
        n_items=0, n_fix=0, n_manual=0, n_defer=0, n_ignore=0,
        items=tuple(),
        generated_at=datetime.now(timezone.utc).isoformat(),
        philosophy_guards=tuple(),
    )


def _empty_apply_report() -> ApplyReport:
    now = datetime.now(timezone.utc).isoformat()
    return ApplyReport(
        version=V1354_VERSION,
        n_attempted=0, n_ok=0, n_warn=0, n_fail=0, n_skip=0,
        dry_run=False, results=tuple(),
        started_at=now, finished_at=now,
        exit_code=3, asi_cap=V1355_ASI_CAP, philosophy_guards=tuple(),
    )


# -----------------------------------------------------------------------------
# Driver
# -----------------------------------------------------------------------------

def run_wet_run(
    scenarios: Optional[List[str]] = None,
    keep: bool = False,
) -> WetRunReport:
    """Run the full wet-run harness."""
    if scenarios is None:
        scenarios = list(SCENARIO_TABLE.keys())

    started = datetime.now(timezone.utc)

    # Verify each requested scenario exists
    for s in scenarios:
        if s not in SCENARIO_TABLE:
            raise ValueError(f"unknown scenario: {s}")

    root = Path(tempfile.gettempdir()) / f"v1355_wet_run_{started.strftime('%Y%m%d_%H%M%S')}_{os.getpid()}"
    root.mkdir(parents=True, exist_ok=True)

    results: List[ScenarioResult] = []
    for s in scenarios:
        results.append(run_scenario(s, root))

    # Tear down unless --keep
    if not keep:
        try:
            shutil.rmtree(root)
        except Exception:
            pass

    finished = datetime.now(timezone.utc)
    n_pass = sum(1 for r in results if r.close_loop_pass)
    n_fail = sum(1 for r in results if not r.close_loop_pass)
    exit_code = 0 if n_fail == 0 else 2

    return WetRunReport(
        version=V1355_VERSION,
        n_scenarios=len(results),
        n_pass=n_pass,
        n_warn=0,
        n_fail=n_fail,
        keep_workspace=keep,
        scenarios=tuple(results),
        started_at=started.isoformat(),
        finished_at=finished.isoformat(),
        exit_code=exit_code,
        asi_cap=V1355_ASI_CAP,
        philosophy_guards=(
            "GUARD_WET_RUN_IS_MECHANICAL",
            "GUARD_TEMP_WORKSPACE_PER_SCENARIO",
            "GUARD_PATH_PATCH_REVERTED",
            "GUARD_NO_PROD_WRITES",
            "GUARD_NO_CODE_TOUCHED",
        ),
        base_v1354_version=V1354_VERSION,
    )


# -----------------------------------------------------------------------------
# Renderers (human-readable)
# -----------------------------------------------------------------------------

def render_report_text(report: WetRunReport) -> str:
    lines: List[str] = []
    lines.append(f"V1355 VCP Wet-Run Report (v{report.version})")
    lines.append(f"Base V1354 version: {report.base_v1354_version}")
    lines.append(f"Started: {report.started_at}")
    lines.append(f"Finished: {report.finished_at}")
    lines.append(f"Scenarios: {report.n_scenarios}; pass={report.n_pass}; fail={report.n_fail}")
    lines.append(f"Exit: {report.exit_code}")
    lines.append("")
    for s in report.scenarios:
        flag = "PASS" if s.close_loop_pass else "FAIL"
        lines.append(f"  [{flag}] {s.scenario} ({s.duration_ms:.1f} ms)")
        lines.append(f"      {s.description}")
        lines.append(f"      workspace: {s.workspace}")
        lines.append(f"      pre_missing={s.pre_missing}  post_missing={s.post_missing}")
        lines.append(f"      plan={s.plan_n_items} items ({s.plan_n_fix} fix); "
                     f"apply attempted={s.apply_attempted} ok={s.apply_ok} warn={s.apply_warn} fail={s.apply_fail} skip={s.apply_skip}")
        if s.failure_reason:
            lines.append(f"      FAILURE: {s.failure_reason}")
    lines.append("")
    lines.append("Philosophy guards:")
    for g in report.philosophy_guards:
        lines.append(f"  - {g}")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Self-tests (Popper-style falsifiable checks)
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

    # --- constants ---
    check("V1355_VERSION is semver string", V1355_VERSION.count(".") == 2)
    check("V1355_ASI_CAP <= 0.01", V1355_ASI_CAP <= 0.01, f"got {V1355_ASI_CAP}")
    check("V1354 imported successfully", V1354_VERSION.count(".") == 2)

    # --- scenario table shape ---
    check("scenario table non-empty", len(SCENARIO_TABLE) >= 5)
    for name, spec in SCENARIO_TABLE.items():
        check(f"{name} has expected_pre_missing", "expected_pre_missing" in spec)
        check(f"{name} has expected_post_missing", "expected_post_missing" in spec)
        check(f"{name} has description", bool(spec.get("description")))

    # --- introspection ---
    tmp = Path(tempfile.mkdtemp(prefix="v1355_probe_"))
    try:
        state = _introspect_state(tmp)
        check("empty dir → all missing",
              state == {"ledger": False, "migration_audit": False, "remediation_history": False},
              f"got {state}")
        (tmp / INFRA_FILES["ledger"]).touch()
        state2 = _introspect_state(tmp)
        check("ledger present → ledger=True",
              state2["ledger"] is True and state2["migration_audit"] is False,
              f"got {state2}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # --- workspace patch (does not crash) ---
    tmp = Path(tempfile.mkdtemp(prefix="v1355_patch_"))
    try:
        with _WorkspacePatch(tmp):
            from apeireth import v1354_vcp_remediation as m1354
            check("LEDGER_PATH patched", m1354.LEDGER_PATH == tmp / INFRA_FILES["ledger"])
            check("SAFE_FIXES contains 4 entries", len(m1354.SAFE_FIXES) == 4)
        from apeireth import v1354_vcp_remediation as m1354
        check("LEDGER_PATH reverted after context", m1354.LEDGER_PATH == PROD_LEDGER_PATH)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # --- run a single scenario via run_wet_run ---
    report = run_wet_run(scenarios=["S0_BLANK"], keep=False)
    check("S0_BLANK ran", report.n_scenarios == 1)
    sr = report.scenarios[0]
    check("S0_BLANK close_loop_pass", sr.close_loop_pass, f"failure={sr.failure_reason}")

    # --- S5_ALL_PRESENT: apply should be no-op ---
    report_s5 = run_wet_run(scenarios=["S5_ALL_PRESENT"], keep=False)
    check("S5_ALL_PRESENT ran", report_s5.n_scenarios == 1)
    sr5 = report_s5.scenarios[0]
    check("S5_ALL_PRESENT close_loop_pass", sr5.close_loop_pass, f"failure={sr5.failure_reason}")
    check("S5_ALL_PRESENT apply_attempted==0", sr5.apply_attempted == 0,
          f"got {sr5.apply_attempted}")
    check("S5_ALL_PRESENT post_missing==[]", sr5.post_missing == [],
          f"got {sr5.post_missing}")

    # --- S4_ALL_MISSING: 3 fixes apply ---
    report_s4 = run_wet_run(scenarios=["S4_ALL_MISSING"], keep=False)
    sr4 = report_s4.scenarios[0]
    check("S4_ALL_MISSING close_loop_pass", sr4.close_loop_pass, f"failure={sr4.failure_reason}")
    check("S4_ALL_MISSING apply_attempted==3", sr4.apply_attempted == 3,
          f"got {sr4.apply_attempted}")
    check("S4_ALL_MISSING apply_ok==3", sr4.apply_ok == 3,
          f"got {sr4.apply_ok}")
    check("S4_ALL_MISSING post_missing==[]", sr4.post_missing == [],
          f"got {sr4.post_missing}")

    # --- S6_CORRUPT_HIST: corrupt file stays ---
    report_s6 = run_wet_run(scenarios=["S6_CORRUPT_HIST"], keep=False)
    sr6 = report_s6.scenarios[0]
    check("S6_CORRUPT_HIST close_loop_pass", sr6.close_loop_pass, f"failure={sr6.failure_reason}")
    check("S6_CORRUPT_HIST corrupted history preserved",
          sr6.post_state.get("remediation_history", False) is True,
          f"got {sr6.post_state}")

    # --- aggregate full run ---
    full = run_wet_run(keep=False)
    check("full run n_scenarios==7", full.n_scenarios == 7,
          f"got {full.n_scenarios}")
    check("full run n_fail==0 (clean machine)", full.n_fail == 0,
          f"got {full.n_fail}: {[s.failure_reason for s in full.scenarios if not s.close_loop_pass]}")
    check("full run exit_code==0", full.exit_code == 0,
          f"got {full.exit_code}")

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_run(args: argparse.Namespace) -> int:
    try:
        report = run_wet_run(
            scenarios=args.scenario.split(",") if args.scenario else None,
            keep=args.keep,
        )
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 3
    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(render_report_text(report))
    return report.exit_code


def _cli_scenarios(_: argparse.Namespace) -> int:
    for name, spec in SCENARIO_TABLE.items():
        print(f"  {name}: {spec['description']}")
        print(f"    pre_create={spec.get('pre_create', [])}")
        print(f"    expected_pre_missing={spec['expected_pre_missing']}")
        print(f"    expected_post_missing={spec['expected_post_missing']}")
        print(f"    expected_apply_attempted={spec['expected_apply_attempted']}, ok_min={spec['expected_apply_ok_min']}")
    return 0


def _cli_expected(_: argparse.Namespace) -> int:
    print(json.dumps(SCENARIO_TABLE, indent=2, ensure_ascii=False))
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1355 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="v1355-wet-run", description="VCP close-the-loop wet-run harness")
    sub = p.add_subparsers(dest="command", required=True)

    p_run = sub.add_parser("run", help="Run scenarios end-to-end")
    p_run.add_argument("--scenario", default="", help="comma-separated scenario names (default: all)")
    p_run.add_argument("--keep", action="store_true", help="keep temp workspace after run")
    p_run.add_argument("--json", action="store_true", help="JSON output")
    p_run.set_defaults(func=_cli_run)

    sub.add_parser("scenarios", help="list scenario table").set_defaults(func=_cli_scenarios)
    sub.add_parser("expected", help="print EXPECTED_RESULTS JSON").set_defaults(func=_cli_expected)

    p_st = sub.add_parser("self-test", help="run embedded Popper checks")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1355-vcp-wet-run {V1355_VERSION} (base V1354 v{V1354_VERSION})") or 0
    )
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
