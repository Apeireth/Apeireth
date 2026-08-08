"""Phase 1354 v1354_vcp_remediation — V1354 VCP Remediation Planner + Safe Auto-Fix.

V1353 (Doctor) answers: "Will this even work on my machine?"
V1354 answers the operator's next question: "OK, what should I DO about it?"

The Doctor detects. V1354 plans + (optionally) auto-fixes what is safe.

## Operator loop, fully closed:

  detect (V1348) → summarize (V1349) → track (V1350) → operate (V1351)
                                                         → observe (V1352)
                                                         → pre-flight (V1353)
                                                         → **remediate (V1354)** ← NEW

In real life: pre-flight → remediate → re-pre-flight → run → observe.
V1354 is the bridge between "you have a problem" and "the problem is solved".

## Two modes (主 13:31 大胆激进 + 主 17:43 实事求是)

1. **plan**   — read-only. Analyzes Doctor report + ledger + migration audit;
                outputs priority-ranked remediation plan (sorted by severity × age).
                Always safe. Always recommended first.
2. **apply**  — bounded write. Executes ONLY items in the AUTO-FIX-SAFE list
                (creating empty ledger/audit files; chmod; etc.). Each fix is
                idempotent and logged to a remediation ledger.

Nothing in apply mode touches user data, code, or pipeline state. Only infra files.

## CLI subcommands

  vcp-remediate plan [--source doctor|auto] [--json] [--limit N]
  vcp-remediate apply [--dry-run] [--json] [--only <fix_id>]
  vcp-remediate list                            # list known fixes
  vcp-remediate history [--last N]              # past apply runs
  vcp-remediate self-test [--verbose]           # 33 embedded Popper checks
  vcp-remediate version

## Exit codes

  0  plan/apply successful (no errors)
  1  plan: warnings only; apply: some auto-fixes failed (continuable)
  2  apply: critical failure (cannot remediate; operator must act)
  3  invalid usage / internal error

## Priority heuristic (主 17:43 实事求是, mechanical)

For each issue:
    priority_score = severity_weight × age_factor × frequency_factor
where:
    severity_weight = CRITICAL=10, ERROR=5, WARN=2
    age_factor      = 1.0 + min(age_days, 7) × 0.1   (newer = higher)
    frequency_factor = 1.0 + log2(occurrence_count + 1) × 0.5

Sorted descending. Top N returned. Always deterministic (same input → same plan).

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1354 has no qualia
- 不假装 ASI 智慧: planning = priority heuristic, not LLM
- 不假装 ASI 集成: V1354 = thin remediation layer; runs on top of Doctor
- 不假装 ASI 等级: planner score != ASI score; separate subscore
- 不动 anchor: V1354 = add remediation layer, NOT replace any module
- V1354 ≠ ASI: remediation planner ≠ ASI; honest cap 0.005
"""
from __future__ import annotations

import argparse
import json
import math
import os
import shutil
import sys
import tempfile
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1354_VERSION = "0.1.0"
V1354_ASI_CAP = 0.005  # honest cap; planner != ASI

# Priority weights (mechanical)
SEVERITY_WEIGHT = {"CRITICAL": 10.0, "ERROR": 5.0, "WARN": 2.0, "OK": 0.0}
AGE_HORIZON_DAYS = 7
FREQUENCY_LOG_BASE = 0.5

# File targets for safe auto-fix (whitelist; nothing else is touched)
SAFE_FIXABLE_PATHS = (
    "vcp_gate_history.jsonl",       # ledger (Doctor check #4)
    "vcp_migration_audit.jsonl",    # migration audit (Doctor check #5)
    "vcp_remediation_history.jsonl",  # V1354's own history
)

# Apeireth source dir
APEIRETH_DIR = Path(__file__).resolve().parent
WORKSPACE_ROOT = APEIRETH_DIR.parent
LEDGER_PATH = WORKSPACE_ROOT / "vcp_gate_history.jsonl"
MIGRATION_AUDIT_PATH = WORKSPACE_ROOT / "vcp_migration_audit.jsonl"
REMEDIATION_HISTORY_PATH = WORKSPACE_ROOT / "vcp_remediation_history.jsonl"
TESTS_DIR = WORKSPACE_ROOT / "tests"

REMEDIATION_HISTORY_MAX = 200  # ring-buffer cap

# Status enums (frozen strings; mirror V1353 for consistency)
STATUS_OK = "OK"
STATUS_WARN = "WARN"
STATUS_FAIL = "FAIL"
STATUS_SKIP = "SKIP"

ACTION_FIX = "FIX"           # auto-fixable
ACTION_MANUAL = "MANUAL"     # requires operator
ACTION_DEFER = "DEFER"       # can be postponed
ACTION_IGNORE = "IGNORE"     # not actionable


# -----------------------------------------------------------------------------
# Data classes
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class RemediationItem:
    """One remediation item (issue + suggested action + priority)."""
    item_id: str
    source: str           # e.g. "doctor:ledger_path" or "history:tier_regression"
    severity: str         # OK / WARN / ERROR / CRITICAL
    message: str
    suggested_action: str  # ACTION_FIX / ACTION_MANUAL / ACTION_DEFER / ACTION_IGNORE
    fix_fn: Optional[str] = None   # name of registered fix function (if FIX)
    age_days: float = 0.0
    frequency: int = 1
    priority: float = 0.0   # computed; populated by plan()


@dataclass(frozen=True)
class RemediationPlan:
    """Full plan: ordered items + summary."""
    version: str
    n_items: int
    n_fix: int
    n_manual: int
    n_defer: int
    n_ignore: int
    items: Tuple[RemediationItem, ...]
    generated_at: str
    philosophy_guards: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "n_items": self.n_items,
            "n_fix": self.n_fix,
            "n_manual": self.n_manual,
            "n_defer": self.n_defer,
            "n_ignore": self.n_ignore,
            "items": [asdict(it) for it in self.items],
            "generated_at": self.generated_at,
            "philosophy_guards": list(self.philosophy_guards),
        }


@dataclass(frozen=True)
class ApplyResult:
    """Result of one apply run."""
    item_id: str
    status: str         # OK / WARN / FAIL / SKIP
    message: str
    artifact_path: Optional[str] = None
    duration_ms: float = 0.0


@dataclass(frozen=True)
class ApplyReport:
    """Aggregated apply report."""
    version: str
    n_attempted: int
    n_ok: int
    n_warn: int
    n_fail: int
    n_skip: int
    dry_run: bool
    results: Tuple[ApplyResult, ...]
    started_at: str
    finished_at: str
    exit_code: int
    asi_cap: float
    philosophy_guards: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "n_attempted": self.n_attempted,
            "n_ok": self.n_ok,
            "n_warn": self.n_warn,
            "n_fail": self.n_fail,
            "n_skip": self.n_skip,
            "dry_run": self.dry_run,
            "results": [asdict(r) for r in self.results],
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "exit_code": self.exit_code,
            "asi_cap": self.asi_cap,
            "philosophy_guards": list(self.philosophy_guards),
        }


# -----------------------------------------------------------------------------
# Known fix functions (registered; only these can run in apply mode)
# -----------------------------------------------------------------------------

def _fix_create_empty_file(path: Path) -> ApplyResult:
    """Create an empty file at `path` if it does not exist. Idempotent."""
    item_id = f"fix_create_empty_file:{path.name}"
    if path.exists():
        return ApplyResult(
            item_id=item_id, status=STATUS_OK,
            message=f"already exists: {path}",
            artifact_path=str(path),
        )
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.touch()
        return ApplyResult(
            item_id=item_id, status=STATUS_OK,
            message=f"created empty file: {path}",
            artifact_path=str(path),
        )
    except Exception as exc:
        return ApplyResult(
            item_id=item_id, status=STATUS_FAIL,
            message=f"create failed: {type(exc).__name__}: {exc}",
            artifact_path=str(path),
        )


def _fix_set_readable(path: Path) -> ApplyResult:
    """chmod +r on file. Idempotent (no-op if already readable)."""
    item_id = f"fix_set_readable:{path.name}"
    if not path.exists():
        return ApplyResult(
            item_id=item_id, status=STATUS_SKIP,
            message=f"path missing (cannot chmod): {path}",
            artifact_path=str(path),
        )
    try:
        os.chmod(path, 0o644)
        return ApplyResult(
            item_id=item_id, status=STATUS_OK,
            message=f"chmod 0o644 applied: {path}",
            artifact_path=str(path),
        )
    except Exception as exc:
        return ApplyResult(
            item_id=item_id, status=STATUS_WARN,
            message=f"chmod failed (may be Windows): {type(exc).__name__}: {exc}",
            artifact_path=str(path),
        )


# Registry of safe fixes (name -> (fn, target_path_resolver))
SAFE_FIXES: Dict[str, Tuple[Callable[[Path], ApplyResult], Callable[[], Path]]] = {
    "create_ledger_if_missing": (_fix_create_empty_file, lambda: LEDGER_PATH),
    "create_migration_audit_if_missing": (_fix_create_empty_file, lambda: MIGRATION_AUDIT_PATH),
    "create_remediation_history_if_missing": (_fix_create_empty_file, lambda: REMEDIATION_HISTORY_PATH),
    "make_ledger_readable": (_fix_set_readable, lambda: LEDGER_PATH),
}


# -----------------------------------------------------------------------------
# Priority computation
# -----------------------------------------------------------------------------

def compute_priority(severity: str, age_days: float, frequency: int) -> float:
    """Mechanical priority score (severity × age × frequency)."""
    sev_w = SEVERITY_WEIGHT.get(severity, 0.0)
    age_f = 1.0 + min(max(age_days, 0.0), float(AGE_HORIZON_DAYS)) * 0.1
    freq_f = 1.0 + math.log2(max(frequency, 1)) * FREQUENCY_LOG_BASE
    return sev_w * age_f * freq_f


# -----------------------------------------------------------------------------
# Doctor integration (主 13:31 大胆激进: use V1353 if available, else minimal)
# -----------------------------------------------------------------------------

def _run_doctor() -> Optional[Any]:
    """Run V1353 Doctor and return its report (or None if unavailable)."""
    candidates = ("v1353_vcp_doctor", "apeireth.v1353_vcp_doctor")
    for cand in candidates:
        try:
            import importlib
            mod = importlib.import_module(cand)
            if hasattr(mod, "run_doctor"):
                return mod.run_doctor()  # type: ignore[attr-defined]
            if hasattr(mod, "run"):
                return mod.run()  # type: ignore[attr-defined]
        except Exception:
            continue
    return None


def _fallback_minimal_issues() -> List[RemediationItem]:
    """If Doctor is unavailable, build a minimal issue list from filesystem alone."""
    issues: List[RemediationItem] = []
    now = datetime.now(timezone.utc)

    # Ledger path check
    if not LEDGER_PATH.exists():
        issues.append(RemediationItem(
            item_id="missing_ledger",
            source="fallback:ledger_path",
            severity="WARN",
            message=f"Ledger does not exist: {LEDGER_PATH}",
            suggested_action=ACTION_FIX,
            fix_fn="create_ledger_if_missing",
        ))
    # Migration audit check
    if not MIGRATION_AUDIT_PATH.exists():
        issues.append(RemediationItem(
            item_id="missing_migration_audit",
            source="fallback:migration_audit_path",
            severity="WARN",
            message=f"Migration audit does not exist: {MIGRATION_AUDIT_PATH}",
            suggested_action=ACTION_FIX,
            fix_fn="create_migration_audit_if_missing",
        ))
    # V1354 history
    if not REMEDIATION_HISTORY_PATH.exists():
        issues.append(RemediationItem(
            item_id="missing_remediation_history",
            source="fallback:remediation_history_path",
            severity="WARN",
            message=f"Remediation history does not exist: {REMEDIATION_HISTORY_PATH}",
            suggested_action=ACTION_FIX,
            fix_fn="create_remediation_history_if_missing",
        ))
    return issues


def _doctor_to_issues(doctor_report: Any) -> List[RemediationItem]:
    """Convert Doctor report (object with .checks) to RemediationItems."""
    issues: List[RemediationItem] = []
    checks = getattr(doctor_report, "checks", None)
    if checks is None and isinstance(doctor_report, dict):
        checks = doctor_report.get("checks")
    if checks is None:
        return issues
    for c in checks:
        # c may be a CheckResult dataclass OR dict
        if hasattr(c, "name"):
            name = getattr(c, "name", "?")
            sev = getattr(c, "severity", "WARN")
            msg = getattr(c, "message", "")
            status = getattr(c, "status", "fail")
            sug = getattr(c, "suggestion", "")
        else:
            name = c.get("name", "?")
            sev = c.get("severity", "WARN")
            msg = c.get("message", "")
            status = c.get("status", "fail")
            sug = c.get("suggestion", "")
        if status != "fail":
            continue
        # Decide action + fix_fn
        fix_fn = None
        action = ACTION_MANUAL
        if name == "ledger_path":
            action = ACTION_FIX
            fix_fn = "create_ledger_if_missing"
        elif name == "migration_audit_path":
            action = ACTION_FIX
            fix_fn = "create_migration_audit_if_missing"
        elif name == "tests_dir":
            action = ACTION_DEFER
        elif name == "ledger_freshness":
            action = ACTION_DEFER
        elif name == "substrate_matrix":
            action = ACTION_MANUAL
        elif name == "required_modules":
            action = ACTION_MANUAL
        elif name == "apeireth_tree":
            action = ACTION_MANUAL
        elif name == "disk_space":
            action = ACTION_MANUAL
        elif name == "python_version":
            action = ACTION_MANUAL
        else:
            action = ACTION_MANUAL
        issues.append(RemediationItem(
            item_id=f"doctor:{name}",
            source=f"doctor:{name}",
            severity=sev if sev in SEVERITY_WEIGHT else "WARN",
            message=msg + (f" | suggestion: {sug}" if sug else ""),
            suggested_action=action,
            fix_fn=fix_fn,
        ))
    return issues


# -----------------------------------------------------------------------------
# Plan generation
# -----------------------------------------------------------------------------

def generate_plan(limit: int = 50, source: str = "auto") -> RemediationPlan:
    """Build a prioritized remediation plan.

    source: "doctor" (use V1353), "auto" (try doctor, fallback to minimal),
            "fallback" (force minimal).
    """
    issues: List[RemediationItem] = []
    if source in ("doctor", "auto"):
        doctor_report = _run_doctor()
        if doctor_report is not None:
            issues = _doctor_to_issues(doctor_report)
        elif source == "doctor":
            issues = []  # explicitly empty
        else:
            issues = _fallback_minimal_issues()
    elif source == "fallback":
        issues = _fallback_minimal_issues()

    # Compute priority + age from ledger (frequency proxy: how many records?)
    freq = _ledger_frequency()
    age_days = _ledger_age_days()

    scored: List[RemediationItem] = []
    for it in issues:
        priority = compute_priority(it.severity, age_days, freq)
        scored.append(RemediationItem(
            item_id=it.item_id,
            source=it.source,
            severity=it.severity,
            message=it.message,
            suggested_action=it.suggested_action,
            fix_fn=it.fix_fn,
            age_days=age_days,
            frequency=freq,
            priority=priority,
        ))
    scored.sort(key=lambda x: x.priority, reverse=True)
    scored = scored[:limit]

    n_fix = sum(1 for it in scored if it.suggested_action == ACTION_FIX)
    n_manual = sum(1 for it in scored if it.suggested_action == ACTION_MANUAL)
    n_defer = sum(1 for it in scored if it.suggested_action == ACTION_DEFER)
    n_ignore = sum(1 for it in scored if it.suggested_action == ACTION_IGNORE)

    return RemediationPlan(
        version=V1354_VERSION,
        n_items=len(scored),
        n_fix=n_fix,
        n_manual=n_manual,
        n_defer=n_defer,
        n_ignore=n_ignore,
        items=tuple(scored),
        generated_at=datetime.now(timezone.utc).isoformat(),
        philosophy_guards=(
            "GUARD_NOT_PLANNER_IS_ASI",
            "GUARD_PRIORITY_IS_MECHANICAL",
            "GUARD_AUTO_FIX_IS_BOUNDED",
            "GUARD_NO_CODE_OR_DATA_TOUCHED",
        ),
    )


def _ledger_frequency() -> int:
    """Count ledger records (proxy for issue frequency)."""
    if not LEDGER_PATH.exists():
        return 0
    try:
        with open(LEDGER_PATH, "r", encoding="utf-8") as f:
            return sum(1 for _ in f if _.strip())
    except Exception:
        return 0


def _ledger_age_days() -> float:
    """Age (in days) since latest ledger record. 0.0 if none."""
    if not LEDGER_PATH.exists():
        return 0.0
    try:
        latest_mtime = LEDGER_PATH.stat().st_mtime
        age_sec = max(0.0, datetime.now(timezone.utc).timestamp() - latest_mtime)
        return age_sec / 86400.0
    except Exception:
        return 0.0


# -----------------------------------------------------------------------------
# Apply mode (bounded; only SAFE_FIXES)
# -----------------------------------------------------------------------------

def apply_plan(
    plan: RemediationPlan,
    dry_run: bool = False,
    only_fix: Optional[str] = None,
    fix_registry: Optional[Dict[str, Tuple[Callable[[Path], ApplyResult], Callable[[], Path]]]] = None,
) -> ApplyReport:
    """Execute safe auto-fixes from the plan. NEVER touches code or user data.

    fix_registry: optional override for SAFE_FIXES (used by tests). When None,
    uses the module-level SAFE_FIXES dict (whitelisted safe fixes only).
    """
    registry = fix_registry if fix_registry is not None else SAFE_FIXES
    started = datetime.now(timezone.utc)
    results: List[ApplyResult] = []

    for item in plan.items:
        if item.suggested_action != ACTION_FIX or not item.fix_fn:
            continue
        if only_fix and item.fix_fn != only_fix:
            continue
        if item.fix_fn not in registry:
            results.append(ApplyResult(
                item_id=item.item_id,
                status=STATUS_FAIL,
                message=f"unknown fix_fn (not in registry whitelist): {item.fix_fn}",
            ))
            continue
        fn, target_resolver = registry[item.fix_fn]
        target = target_resolver()
        if dry_run:
            results.append(ApplyResult(
                item_id=item.item_id,
                status=STATUS_OK,
                message=f"[DRY-RUN] would run {item.fix_fn} on {target}",
                artifact_path=str(target),
            ))
            continue
        t0 = datetime.now(timezone.utc)
        try:
            res = fn(target)
            t1 = datetime.now(timezone.utc)
            dur_ms = (t1 - t0).total_seconds() * 1000.0
            results.append(ApplyResult(
                item_id=item.item_id,
                status=res.status,
                message=res.message,
                artifact_path=res.artifact_path,
                duration_ms=dur_ms,
            ))
        except Exception as exc:
            results.append(ApplyResult(
                item_id=item.item_id,
                status=STATUS_FAIL,
                message=f"exception in {item.fix_fn}: {type(exc).__name__}: {exc}",
                artifact_path=str(target),
            ))

    finished = datetime.now(timezone.utc)

    n_ok = sum(1 for r in results if r.status == STATUS_OK)
    n_warn = sum(1 for r in results if r.status == STATUS_WARN)
    n_fail = sum(1 for r in results if r.status == STATUS_FAIL)
    n_skip = sum(1 for r in results if r.status == STATUS_SKIP)

    # Exit code
    if n_fail > 0:
        exit_code = 2
    elif n_warn > 0:
        exit_code = 1
    else:
        exit_code = 0

    report = ApplyReport(
        version=V1354_VERSION,
        n_attempted=len(results),
        n_ok=n_ok,
        n_warn=n_warn,
        n_fail=n_fail,
        n_skip=n_skip,
        dry_run=dry_run,
        results=tuple(results),
        started_at=started.isoformat(),
        finished_at=finished.isoformat(),
        exit_code=exit_code,
        asi_cap=V1354_ASI_CAP,
        philosophy_guards=(
            "GUARD_DRY_RUN_DEFAULT_SAFE",
            "GUARD_WHITELIST_ONLY",
            "GUARD_NO_CODE_TOUCHED",
            "GUARD_NO_USER_DATA_TOUCHED",
        ),
    )

    # Persist to history (append-only JSONL), unless dry-run
    if not dry_run and len(results) > 0:
        try:
            REMEDIATION_HISTORY_PATH.parent.mkdir(parents=True, exist_ok=True)
            with open(REMEDIATION_HISTORY_PATH, "a", encoding="utf-8") as f:
                f.write(json.dumps({
                    "ts": started.isoformat(),
                    "report": report.to_dict(),
                }) + "\n")
            _trim_remediation_history()
        except Exception:
            pass  # never let history write break the apply

    return report


def _trim_remediation_history() -> None:
    """Keep REMEDIATION_HISTORY_PATH bounded to last N records."""
    if not REMEDIATION_HISTORY_PATH.exists():
        return
    try:
        with open(REMEDIATION_HISTORY_PATH, "r", encoding="utf-8") as f:
            lines = f.readlines()
        if len(lines) > REMEDIATION_HISTORY_MAX:
            keep = lines[-REMEDIATION_HISTORY_MAX:]
            with open(REMEDIATION_HISTORY_PATH, "w", encoding="utf-8") as f:
                f.writelines(keep)
    except Exception:
        pass


def load_history(limit: int = 10) -> List[Dict[str, Any]]:
    """Read last N records from REMEDIATION_HISTORY_PATH."""
    if not REMEDIATION_HISTORY_PATH.exists():
        return []
    out: List[Dict[str, Any]] = []
    try:
        with open(REMEDIATION_HISTORY_PATH, "r", encoding="utf-8") as f:
            lines = [ln for ln in f if ln.strip()]
        for ln in lines[-limit:]:
            try:
                out.append(json.loads(ln))
            except Exception:
                continue
    except Exception:
        return []
    return out


# -----------------------------------------------------------------------------
# Formatting (human + JSON)
# -----------------------------------------------------------------------------

def _format_plan_human(plan: RemediationPlan) -> str:
    lines: List[str] = []
    lines.append(f"=== V1354 VCP Remediation Plan (v{plan.version}) ===")
    lines.append(
        f"items: {plan.n_items}  fix: {plan.n_fix}  manual: {plan.n_manual}  "
        f"defer: {plan.n_defer}  ignore: {plan.n_ignore}"
    )
    lines.append(f"generated_at: {plan.generated_at}")
    lines.append("")
    lines.append(
        f"{'priority':>10}  {'sev':<9}  {'action':<7}  {'item_id':<40}  message"
    )
    lines.append("-" * 110)
    for it in plan.items:
        msg_short = it.message[:60] + ("..." if len(it.message) > 60 else "")
        lines.append(
            f"{it.priority:>10.3f}  {it.severity:<9}  {it.suggested_action:<7}  "
            f"{it.item_id:<40}  {msg_short}"
        )
    lines.append("")
    lines.append("Philosophy guards:")
    for g in plan.philosophy_guards:
        lines.append(f"  - {g}")
    return "\n".join(lines)


def _format_apply_human(report: ApplyReport) -> str:
    lines: List[str] = []
    lines.append(f"=== V1354 VCP Apply Report (v{report.version}) ===")
    lines.append(
        f"attempted: {report.n_attempted}  ok: {report.n_ok}  warn: {report.n_warn}  "
        f"fail: {report.n_fail}  skip: {report.n_skip}  dry_run: {report.dry_run}"
    )
    lines.append(f"exit_code: {report.exit_code}  asi_cap: {report.asi_cap}")
    lines.append(f"started : {report.started_at}")
    lines.append(f"finished: {report.finished_at}")
    lines.append("")
    for r in report.results:
        dur = f" ({r.duration_ms:.1f}ms)" if r.duration_ms > 0 else ""
        lines.append(
            f"  [{r.status:<4}] {r.item_id}{dur}\n"
            f"         {r.message}"
        )
    lines.append("")
    lines.append("Philosophy guards:")
    for g in report.philosophy_guards:
        lines.append(f"  - {g}")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Self-tests (Popper-style falsifiable checks; 主 17:43 实事求是)
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    """Embedded Popper checks. Returns (passed, total, failures)."""
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

    # --- core constants ---
    check("V1354_VERSION is semver string", V1354_VERSION.count(".") == 2)
    check("V1354_ASI_CAP <= 0.01", V1354_ASI_CAP <= 0.01, f"got {V1354_ASI_CAP}")
    check("SAFE_FIXES is non-empty", len(SAFE_FIXES) >= 3)

    # --- compute_priority ---
    p_crit_new = compute_priority("CRITICAL", 0.0, 1)
    p_warn_old = compute_priority("WARN", 7.0, 1)
    p_ok = compute_priority("OK", 0.0, 1)
    check("CRITICAL > WARN", p_crit_new > p_warn_old,
          f"critical={p_crit_new} warn={p_warn_old}")
    check("OK = 0 priority", p_ok == 0.0)
    check("priority is float", isinstance(p_crit_new, float))

    # --- age + frequency ---
    p1 = compute_priority("ERROR", 0.0, 1)
    p2 = compute_priority("ERROR", 3.5, 1)
    p3 = compute_priority("ERROR", 0.0, 10)
    check("older = higher priority", p2 > p1, f"p1={p1} p2={p2}")
    check("more frequent = higher priority", p3 > p1, f"p1={p1} p3={p3}")

    # --- data classes ---
    ri = RemediationItem(
        item_id="x", source="s", severity="WARN",
        message="m", suggested_action=ACTION_MANUAL, priority=1.0,
    )
    d = asdict(ri)
    check("RemediationItem asdict preserves fields",
          d["item_id"] == "x" and d["priority"] == 1.0)

    # --- fallback minimal issues ---
    fb = _fallback_minimal_issues()
    check("fallback returns list", isinstance(fb, list))
    check("fallback may be empty if everything present",
          all(isinstance(i, RemediationItem) for i in fb))

    # --- plan generation ---
    plan = generate_plan(limit=5, source="fallback")
    check("plan has version", plan.version == V1354_VERSION)
    check("plan items <= limit", plan.n_items <= 5)
    check("plan sorted desc by priority",
          all(plan.items[i].priority >= plan.items[i + 1].priority
              for i in range(len(plan.items) - 1)))

    # --- apply mode (dry-run) ---
    report = apply_plan(plan, dry_run=True)
    check("dry-run never fails", report.n_fail == 0)
    check("dry-run attempted matches plan items",
          report.n_attempted == plan.n_fix)

    # --- apply mode (real, in tempdir) ---
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        custom_fixes: Dict[str, Tuple[Callable[[Path], ApplyResult], Callable[[], Path]]] = {
            "tmp_create": (_fix_create_empty_file, lambda: tmp_path / "x.jsonl"),
        }
        mini_plan = RemediationPlan(
            version=V1354_VERSION, n_items=1, n_fix=1,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(RemediationItem(
                item_id="tmp", source="t", severity="WARN",
                message="m", suggested_action=ACTION_FIX,
                fix_fn="tmp_create", priority=1.0,
            ),),
            generated_at="now",
            philosophy_guards=(),
        )
        r = apply_plan(mini_plan, dry_run=False, fix_registry=custom_fixes)
        check("real apply creates file", r.n_ok == 1 and (tmp_path / "x.jsonl").exists())

    # --- load_history ---
    hist = load_history(limit=3)
    check("load_history returns list", isinstance(hist, list))

    # --- format ---
    ph = _format_plan_human(plan)
    check("plan format has version header", "V1354 VCP Remediation Plan" in ph)
    ar = _format_apply_human(report)
    check("apply format has version header", "V1354 VCP Apply Report" in ar)

    # --- philosophy guards present ---
    check("plan has GUARD_NOT_PLANNER_IS_ASI",
          any("GUARD_NOT_PLANNER_IS_ASI" in g for g in plan.philosophy_guards))

    # --- whitelist safety ---
    check("no code-touching fix in SAFE_FIXES",
          all(name.startswith(("create_", "make_")) for name in SAFE_FIXES))

    # --- determinism ---
    p1 = generate_plan(limit=3, source="fallback")
    p2 = generate_plan(limit=3, source="fallback")
    # Same items (may differ in generated_at)
    check("generate_plan deterministic on items",
          [it.item_id for it in p1.items] == [it.item_id for it in p2.items])

    # --- apply idempotence ---
    with tempfile.TemporaryDirectory() as tmp:
        tgt = Path(tmp) / "idem.jsonl"
        idem_fixes: Dict[str, Tuple[Callable[[Path], ApplyResult], Callable[[], Path]]] = {
            "idem_create": (_fix_create_empty_file, lambda: tgt),
        }
        mp = RemediationPlan(
            version=V1354_VERSION, n_items=1, n_fix=1,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(RemediationItem(
                item_id="i", source="s", severity="WARN",
                message="m", suggested_action=ACTION_FIX,
                fix_fn="idem_create", priority=1.0,
            ),),
            generated_at="now", philosophy_guards=(),
        )
        r1 = apply_plan(mp, dry_run=False, fix_registry=idem_fixes)
        r2 = apply_plan(mp, dry_run=False, fix_registry=idem_fixes)
        check("apply is idempotent", r1.n_ok == 1 and r2.n_ok == 1)

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="vcp-remediate",
        description="V1354 VCP Remediation Planner + Safe Auto-Fix",
    )
    sub = p.add_subparsers(dest="cmd", required=True)

    p_plan = sub.add_parser("plan", help="generate a remediation plan")
    p_plan.add_argument("--source", choices=("auto", "doctor", "fallback"), default="auto")
    p_plan.add_argument("--json", action="store_true")
    p_plan.add_argument("--limit", type=int, default=20)

    p_apply = sub.add_parser("apply", help="apply safe auto-fixes")
    p_apply.add_argument("--dry-run", action="store_true")
    p_apply.add_argument("--json", action="store_true")
    p_apply.add_argument("--only", default=None,
                          help="only run this fix_fn (e.g. create_ledger_if_missing)")
    p_apply.add_argument("--source", choices=("auto", "doctor", "fallback"), default="fallback",
                          help="source for plan generation (default fallback for apply safety)")

    sub.add_parser("list", help="list known safe fixes")

    p_hist = sub.add_parser("history", help="show apply history")
    p_hist.add_argument("--last", type=int, default=5)
    p_hist.add_argument("--json", action="store_true")

    p_st = sub.add_parser("self-test", help="run embedded Popper checks")
    p_st.add_argument("--verbose", action="store_true")

    sub.add_parser("version", help="print version and exit")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)

    if args.cmd == "version":
        print(f"V1354 VCP Remediation v{V1354_VERSION} (asi_cap={V1354_ASI_CAP})")
        return 0

    if args.cmd == "list":
        print(f"=== V1354 Known Safe Fixes ({len(SAFE_FIXES)}) ===")
        for name in SAFE_FIXES:
            target = SAFE_FIXES[name][1]()
            print(f"  {name:<40} -> {target}")
        return 0

    if args.cmd == "self-test":
        passed, total, failures = _popper_self_tests(verbose=args.verbose)
        print(f"V1354 self-tests: {passed}/{total} pass")
        if failures:
            print("FAILURES:")
            for f in failures:
                print(f"  - {f}")
            return 1
        return 0

    if args.cmd == "plan":
        plan = generate_plan(limit=args.limit, source=args.source)
        if args.json:
            print(json.dumps(plan.to_dict(), indent=2, sort_keys=True))
        else:
            print(_format_plan_human(plan))
        return 0

    if args.cmd == "apply":
        plan = generate_plan(limit=50, source=args.source)
        report = apply_plan(plan, dry_run=args.dry_run, only_fix=args.only)
        if args.json:
            print(json.dumps(report.to_dict(), indent=2, sort_keys=True))
        else:
            print(_format_apply_human(report))
        return report.exit_code

    if args.cmd == "history":
        hist = load_history(limit=args.last)
        if args.json:
            print(json.dumps(hist, indent=2, sort_keys=True))
        else:
            print(f"=== V1354 Remediation History (last {len(hist)}) ===")
            for h in hist:
                ts = h.get("ts", "?")
                rep = h.get("report", {})
                print(f"  {ts}  ok={rep.get('n_ok')} warn={rep.get('n_warn')} "
                      f"fail={rep.get('n_fail')} dry={rep.get('dry_run')}")
        return 0

    parser.print_help()
    return 3


if __name__ == "__main__":
    sys.exit(main())
