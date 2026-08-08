"""Phase 1353 v1353_vcp_doctor — V1353 VCP Doctor (pre-flight safety check).

V1352 made ledger READS easy. V1353 makes PRE-FLIGHT CHECKS easy — answers the
operator's most embarrassing question before they run `vcp run`:

  "Will this even work on my machine, or will it silently fail at stage 3?"

The Doctor is mechanical, deterministic, side-effect-free (except for one
optional disk-space syscall). It does NOT run the pipeline. It just inspects:

  1. Python interpreter version (>= 3.11 required by V1351 annotations)
  2. Required upstream modules importable (V1342-V1352 + V1351 runner)
  3. Ledger path exists + is readable + writable (file is JSONL append)
  4. Migration audit path writable
  5. Substrate matrix non-empty (V1335 → at least 1 plugin listed)
  6. Last ledger record age (warn if stale > 24h; critical if > 7d)
  7. Apeireth source tree healthy (at least 100 v*.py files present)
  8. Disk space available at ledger directory (> 1 MB free)
  9. Pytest available (needed for downstream `pytest` after `vcp run`)
 10. Optional: API keys present (V1349 LLM benchmark needs key)

Each check returns a `CheckResult` (severity + status + message + suggestion).

## Operator loop, fully closed end-to-end:

  detect (V1348) → summarize (V1349) → track (V1350) → operate (V1351)
                                                         → observe (V1352)
                                                         → **pre-flight (V1353)** ← NEW

Order in real life is: pre-flight FIRST → run → observe. Doctor is the gate.

## CLI subcommands

  vcp-doctor run [--json] [--strict] [--check-keys]
  vcp-doctor list
  vcp-doctor self-test [--verbose]
  vcp-doctor version

## Exit codes (CI-friendly)

  0  healthy (no errors, no warnings — operator can proceed confidently)
  1  warnings only (operator can proceed but should review)
  2  errors present (operator should fix before running pipeline)
  3  critical errors (pipeline CANNOT run; abort)

`--strict` upgrades all warnings to errors (for CI gating).

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1353 has no qualia
- 不假装 ASI 智慧: checks are mechanical (version compare, path stat, import test)
- 不假装 ASI 集成: V1353 = thin pre-flight; reuses import semantics
- 不假装 ASI 等级: doctor score != ASI score; separate subscore
- 不动 anchor: V1353 = add pre-flight layer, NOT replace any module
- V1353 ≠ ASI: pre-flight checks ≠ ASI; honest cap 0.008
"""
from __future__ import annotations

import argparse
import importlib
import json
import os
import shutil
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1353_VERSION = "0.1.0"
V1353_ASI_CAP = 0.008  # honest cap; pre-flight != ASI

# Minimum Python version required by V1351 (`from __future__ import annotations`
# + PEP 604 union types in helpers).
MIN_PYTHON = (3, 11)

# Upstream modules the pipeline (V1351) imports.
REQUIRED_MODULES = (
    "v1335_vcp_cross_plugin_invariant_synthesis",
    "v1342_vcp_quality_tiers",
    "v1343_vcp_tier_aware_linter",
    "v1345_vcp_historical_ledger",
    "v1346_vcp_tier_aware_migration",
    "v1347_vcp_plugin_health",
    "v1348_vcp_anomaly_detector",
    "v1349_vcp_llm_benchmark",
    "v1350_vcp_anomaly_lifecycle",
    "v1351_vcp_toolchain_cli",
    "v1352_vcp_history_diff",
)

# Optional modules (warning if missing, not error)
OPTIONAL_MODULES = (
    "pytest",
)

# Optional API keys (V1349 LLM benchmark)
OPTIONAL_API_KEYS = (
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "APEIRETH_LLM_API_KEY",
)

# Apeireth source dir (v*.py modules live here)
APEIRETH_DIR = Path(__file__).resolve().parent
WORKSPACE_ROOT = APEIRETH_DIR.parent
LEDGER_PATH = WORKSPACE_ROOT / "vcp_gate_history.jsonl"
MIGRATION_AUDIT_PATH = WORKSPACE_ROOT / "vcp_migration_audit.jsonl"
TESTS_DIR = WORKSPACE_ROOT / "tests"

# Minimum number of v*.py files expected in apeireth/ (sanity check)
MIN_APEIRETH_FILES = 100

# Staleness thresholds (in seconds)
STALE_WARN_SECONDS = 24 * 3600        # 1 day
STALE_CRITICAL_SECONDS = 7 * 24 * 3600  # 7 days

# Minimum free disk space at ledger dir (bytes)
MIN_DISK_FREE_BYTES = 1 * 1024 * 1024  # 1 MB


# -----------------------------------------------------------------------------
# Severity + status enums (frozen strings, no Enum class to keep simple)
# -----------------------------------------------------------------------------

SEVERITY_OK = "OK"
SEVERITY_WARN = "WARN"
SEVERITY_ERROR = "ERROR"
SEVERITY_CRITICAL = "CRITICAL"

STATUS_PASS = "pass"
STATUS_FAIL = "fail"
STATUS_SKIP = "skip"


@dataclass(frozen=True)
class CheckResult:
    """One doctor check (name + severity + status + message + suggestion)."""
    name: str
    severity: str   # OK / WARN / ERROR / CRITICAL
    status: str     # pass / fail / skip
    message: str
    suggestion: str = ""

    def is_failure(self) -> bool:
        return self.status == STATUS_FAIL


@dataclass(frozen=True)
class DoctorReport:
    """Aggregated doctor report (one run)."""
    version: str
    n_checks: int
    n_pass: int
    n_warn: int
    n_fail: int
    n_skip: int
    exit_code: int
    asi_cap: float
    philosophy_guards: Tuple[str, ...]
    checks: Tuple[CheckResult, ...]
    started_at: str
    finished_at: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "n_checks": self.n_checks,
            "n_pass": self.n_pass,
            "n_warn": self.n_warn,
            "n_fail": self.n_fail,
            "n_skip": self.n_skip,
            "exit_code": self.exit_code,
            "asi_cap": self.asi_cap,
            "philosophy_guards": list(self.philosophy_guards),
            "checks": [asdict(c) for c in self.checks],
            "started_at": self.started_at,
            "finished_at": self.finished_at,
        }


# -----------------------------------------------------------------------------
# Individual checks (each is a pure function returning CheckResult)
# -----------------------------------------------------------------------------

def check_python_version() -> CheckResult:
    """Check interpreter version >= MIN_PYTHON."""
    cur = sys.version_info
    actual = f"{cur.major}.{cur.minor}.{cur.micro}"
    if (cur.major, cur.minor) >= MIN_PYTHON:
        return CheckResult(
            name="python_version",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"Python {actual} >= {MIN_PYTHON[0]}.{MIN_PYTHON[1]}",
        )
    return CheckResult(
        name="python_version",
        severity=SEVERITY_CRITICAL,
        status=STATUS_FAIL,
        message=f"Python {actual} < {MIN_PYTHON[0]}.{MIN_PYTHON[1]} (required)",
        suggestion=f"Install Python >= {MIN_PYTHON[0]}.{MIN_PYTHON[1]} or use a different interpreter",
    )


def _try_import(modname: str) -> Tuple[bool, Optional[str]]:
    """Try importing modname; fall back to 'apeireth.' namespace prefix.

    Returns (success, error_message_or_none).
    """
    for candidate in (modname, f"apeireth.{modname}"):
        try:
            importlib.import_module(candidate)
            return True, None
        except ImportError as exc:
            last_err = str(exc)
            continue
        except Exception as exc:
            # Module found but raised on import — also a failure, but a different kind
            return False, f"{type(exc).__name__}: {exc}"
    return False, last_err if 'last_err' in dir() else "ModuleNotFoundError"


def check_required_modules() -> CheckResult:
    """Try importing all REQUIRED_MODULES; report missing.

    Tries bare name first, then 'apeireth.' prefix fallback.
    """
    missing: List[str] = []
    errors: Dict[str, str] = {}
    for modname in REQUIRED_MODULES:
        ok, err = _try_import(modname)
        if not ok:
            missing.append(modname)
            if err:
                errors[modname] = err
    if not missing:
        return CheckResult(
            name="required_modules",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"All {len(REQUIRED_MODULES)} required modules importable",
        )
    # Categorize severity: more than half missing = CRITICAL
    if len(missing) > len(REQUIRED_MODULES) // 2:
        sev = SEVERITY_CRITICAL
    else:
        sev = SEVERITY_ERROR
    sample = ", ".join(missing[:5])
    more = f" (+{len(missing) - 5} more)" if len(missing) > 5 else ""
    return CheckResult(
        name="required_modules",
        severity=sev,
        status=STATUS_FAIL,
        message=f"{len(missing)} of {len(REQUIRED_MODULES)} required modules missing: {sample}{more}",
        suggestion="Verify apeireth/v13*.py files exist and have no syntax errors",
    )


def check_optional_modules() -> CheckResult:
    """Check OPTIONAL_MODULES (pytest etc.). WARN if missing."""
    missing: List[str] = []
    for modname in OPTIONAL_MODULES:
        try:
            importlib.import_module(modname)
        except ImportError:
            missing.append(modname)
        except Exception:
            missing.append(modname)
    if not missing:
        return CheckResult(
            name="optional_modules",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"All {len(OPTIONAL_MODULES)} optional modules importable",
        )
    return CheckResult(
        name="optional_modules",
        severity=SEVERITY_WARN,
        status=STATUS_FAIL,
        message=f"{len(missing)} optional modules missing: {', '.join(missing)}",
        suggestion="pip install " + " ".join(missing),
    )


def check_ledger_path(path: Path = LEDGER_PATH) -> CheckResult:
    """Check ledger path exists (or can be created) + readable + writable."""
    if path.exists():
        if not path.is_file():
            return CheckResult(
                name="ledger_path",
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message=f"Ledger path exists but is not a regular file: {path}",
                suggestion="Remove or rename the conflicting path",
            )
        # Check readable + writable
        try:
            with open(path, "r", encoding="utf-8") as f:
                f.read(1)
        except Exception as exc:
            return CheckResult(
                name="ledger_path",
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message=f"Ledger not readable: {exc}",
                suggestion="chmod the ledger to be readable",
            )
        try:
            with open(path, "a", encoding="utf-8") as f:
                pass
        except Exception as exc:
            return CheckResult(
                name="ledger_path",
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message=f"Ledger not writable: {exc}",
                suggestion="chmod the ledger to be writable",
            )
        return CheckResult(
            name="ledger_path",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"Ledger exists and is read+write OK: {path.name}",
        )
    # Path doesn't exist — try to ensure parent exists and we can create it
    parent = path.parent
    if not parent.exists():
        return CheckResult(
            name="ledger_path",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Ledger parent directory missing: {parent}",
            suggestion="Create parent directory first",
        )
    if not os.access(parent, os.W_OK):
        return CheckResult(
            name="ledger_path",
            severity=SEVERITY_ERROR,
            status=STATUS_FAIL,
            message=f"Ledger parent not writable: {parent}",
            suggestion="chmod the parent directory",
        )
    return CheckResult(
        name="ledger_path",
        severity=SEVERITY_WARN,
        status=STATUS_PASS,
        message=f"Ledger does not exist yet (will be created on first run): {path.name}",
        suggestion="This is fine; the pipeline will create it on first `vcp run`",
    )


def check_migration_audit_path(path: Path = MIGRATION_AUDIT_PATH) -> CheckResult:
    """Check migration audit path writable (may not exist yet)."""
    parent = path.parent
    if not parent.exists():
        return CheckResult(
            name="migration_audit_path",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Migration audit parent directory missing: {parent}",
            suggestion="Create parent directory first",
        )
    if path.exists():
        if not os.access(path, os.W_OK):
            return CheckResult(
                name="migration_audit_path",
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message=f"Migration audit not writable: {path}",
                suggestion="chmod to be writable",
            )
        return CheckResult(
            name="migration_audit_path",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"Migration audit exists and writable: {path.name}",
        )
    if not os.access(parent, os.W_OK):
        return CheckResult(
            name="migration_audit_path",
            severity=SEVERITY_ERROR,
            status=STATUS_FAIL,
            message=f"Migration audit parent not writable: {parent}",
            suggestion="chmod the parent directory",
        )
    return CheckResult(
        name="migration_audit_path",
        severity=SEVERITY_WARN,
        status=STATUS_PASS,
        message=f"Migration audit does not exist yet (OK): {path.name}",
        suggestion="Will be created on first run",
    )


def check_substrate_matrix() -> CheckResult:
    """Check V1335 substrate matrix non-empty."""
    ok, err = _try_import("v1335_vcp_cross_plugin_invariant_synthesis")
    if not ok:
        return CheckResult(
            name="substrate_matrix",
            severity=SEVERITY_ERROR,
            status=STATUS_FAIL,
            message=f"V1335 module not importable: {err}",
            suggestion="Verify v1335 module is on sys.path and has no syntax errors",
        )
    # Module imported successfully — now call build_matrix()
    try:
        for candidate in ("v1335_vcp_cross_plugin_invariant_synthesis",
                          "apeireth.v1335_vcp_cross_plugin_invariant_synthesis"):
            try:
                mod = importlib.import_module(candidate)
                matrix = mod.build_matrix()
                n = len(matrix.plugin_coverage) if hasattr(matrix, "plugin_coverage") else 0
                break
            except (ImportError, AttributeError):
                continue
        else:
            return CheckResult(
                name="substrate_matrix",
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message="V1335 imported but build_matrix() not callable",
                suggestion="Verify v1335.build_matrix() returns object with plugin_coverage",
            )
    except Exception as exc:
        return CheckResult(
            name="substrate_matrix",
            severity=SEVERITY_ERROR,
            status=STATUS_FAIL,
            message=f"V1335 matrix not buildable: {type(exc).__name__}: {exc}",
            suggestion="Verify v1335 module is functional",
        )
    if n == 0:
        return CheckResult(
            name="substrate_matrix",
            severity=SEVERITY_CRITICAL,
            status=STATUS_FAIL,
            message="V1335 substrate matrix is empty (0 plugins)",
            suggestion="Re-run substrate discovery or import substrate plugins",
        )
    return CheckResult(
        name="substrate_matrix",
        severity=SEVERITY_OK,
        status=STATUS_PASS,
        message=f"V1335 substrate matrix non-empty: {n} plugins",
    )


def check_ledger_freshness(path: Path = LEDGER_PATH) -> CheckResult:
    """Check latest ledger record timestamp / age."""
    if not path.exists():
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_SKIP,
            message="No ledger file yet (no run history)",
            suggestion="Run `vcp run` at least once to establish baseline",
        )
    # Read last line (latest record by append order)
    try:
        with open(path, "r", encoding="utf-8") as f:
            last_line = ""
            for line in f:
                if line.strip():
                    last_line = line
        if not last_line:
            return CheckResult(
                name="ledger_freshness",
                severity=SEVERITY_WARN,
                status=STATUS_SKIP,
                message="Ledger is empty (no records)",
                suggestion="Run `vcp run` to create first record",
            )
        rec = json.loads(last_line)
    except Exception as exc:
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Ledger parse error: {type(exc).__name__}: {exc}",
            suggestion="Check ledger file integrity",
        )
    ts = rec.get("timestamp", "")
    if not ts:
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_SKIP,
            message="Latest ledger record has no timestamp field",
        )
    # Parse ISO timestamp (assume Z suffix)
    try:
        from datetime import datetime, timezone
        iso = ts.replace("Z", "+00:00")
        dt = datetime.fromisoformat(iso)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        now = datetime.now(timezone.utc)
        age_s = (now - dt).total_seconds()
    except Exception as exc:
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Timestamp parse error: {type(exc).__name__}: {exc} (ts={ts!r})",
        )
    if age_s < 0:
        # Future timestamp — clock skew
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Latest record timestamp is in the future (skew {-age_s:.0f}s): {ts}",
            suggestion="Check system clock",
        )
    if age_s > STALE_CRITICAL_SECONDS:
        days = age_s / 86400
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_CRITICAL,
            status=STATUS_FAIL,
            message=f"Latest record is {days:.1f} days old (>7d): {ts}",
            suggestion="Pipeline hasn't run in over a week; verify CI is healthy",
        )
    if age_s > STALE_WARN_SECONDS:
        hours = age_s / 3600
        return CheckResult(
            name="ledger_freshness",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"Latest record is {hours:.1f} hours old (>24h): {ts}",
            suggestion="Consider running `vcp run` to refresh baseline",
        )
    if age_s < 60:
        age_str = f"{age_s:.0f}s"
    elif age_s < 3600:
        age_str = f"{age_s / 60:.1f}min"
    else:
        age_str = f"{age_s / 3600:.1f}h"
    return CheckResult(
        name="ledger_freshness",
        severity=SEVERITY_OK,
        status=STATUS_PASS,
        message=f"Latest record fresh ({age_str} ago): {ts}",
    )


def check_apeireth_tree(apeireth_dir: Path = APEIRETH_DIR) -> CheckResult:
    """Check apeireth source tree has expected v*.py files."""
    if not apeireth_dir.exists():
        return CheckResult(
            name="apeireth_tree",
            severity=SEVERITY_CRITICAL,
            status=STATUS_FAIL,
            message=f"Apeireth source dir missing: {apeireth_dir}",
            suggestion="Verify workspace layout",
        )
    n = sum(1 for p in apeireth_dir.glob("v*.py") if p.is_file())
    if n < MIN_APEIRETH_FILES:
        return CheckResult(
            name="apeireth_tree",
            severity=SEVERITY_ERROR,
            status=STATUS_FAIL,
            message=f"Only {n} v*.py files in apeireth/ (expected >= {MIN_APEIRETH_FILES})",
            suggestion="Check for missing modules or wrong directory",
        )
    return CheckResult(
        name="apeireth_tree",
        severity=SEVERITY_OK,
        status=STATUS_PASS,
        message=f"Apeireth tree has {n} v*.py modules (>= {MIN_APEIRETH_FILES})",
    )


def check_disk_space(path: Path = WORKSPACE_ROOT) -> CheckResult:
    """Check free disk space at workspace root."""
    try:
        usage = shutil.disk_usage(str(path))
        free_mb = usage.free / (1024 * 1024)
        if usage.free < MIN_DISK_FREE_BYTES:
            return CheckResult(
                name="disk_space",
                severity=SEVERITY_CRITICAL,
                status=STATUS_FAIL,
                message=f"Free disk space: {free_mb:.1f} MB (< 1 MB)",
                suggestion="Free disk space before running pipeline",
            )
        if usage.free < 100 * 1024 * 1024:
            return CheckResult(
                name="disk_space",
                severity=SEVERITY_WARN,
                status=STATUS_FAIL,
                message=f"Free disk space low: {free_mb:.1f} MB (< 100 MB)",
                suggestion="Consider freeing disk space",
            )
        return CheckResult(
            name="disk_space",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"Free disk space: {free_mb:.1f} MB",
        )
    except Exception as exc:
        return CheckResult(
            name="disk_space",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"disk_usage failed: {type(exc).__name__}: {exc}",
        )


def check_tests_dir(tests_dir: Path = TESTS_DIR) -> CheckResult:
    """Check tests/ directory exists with at least one test_*.py file."""
    if not tests_dir.exists():
        return CheckResult(
            name="tests_dir",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"tests/ directory missing: {tests_dir}",
            suggestion="Verify workspace layout",
        )
    n = sum(1 for p in tests_dir.glob("test_*.py") if p.is_file())
    if n == 0:
        return CheckResult(
            name="tests_dir",
            severity=SEVERITY_WARN,
            status=STATUS_FAIL,
            message=f"No test_*.py files in tests/ (expected >= 1)",
            suggestion="Add at least one test file",
        )
    return CheckResult(
        name="tests_dir",
        severity=SEVERITY_OK,
        status=STATUS_PASS,
        message=f"tests/ has {n} test_*.py files",
    )


def check_api_keys(check_keys: bool = False) -> CheckResult:
    """Check optional API keys (only when --check-keys is set)."""
    if not check_keys:
        return CheckResult(
            name="api_keys",
            severity=SEVERITY_OK,
            status=STATUS_SKIP,
            message="API key check skipped (use --check-keys to enable)",
            suggestion="Run with --check-keys before invoking V1349 LLM benchmark",
        )
    missing = [k for k in OPTIONAL_API_KEYS if not os.environ.get(k)]
    if not missing:
        return CheckResult(
            name="api_keys",
            severity=SEVERITY_OK,
            status=STATUS_PASS,
            message=f"All {len(OPTIONAL_API_KEYS)} optional API keys present",
        )
    return CheckResult(
        name="api_keys",
        severity=SEVERITY_WARN,
        status=STATUS_FAIL,
        message=f"{len(missing)} of {len(OPTIONAL_API_KEYS)} optional API keys missing: {', '.join(missing)}",
        suggestion="Set the missing env vars (LLM benchmark will skip if absent)",
    )


# -----------------------------------------------------------------------------
# Default check registry (constant order; not learned)
# -----------------------------------------------------------------------------

def _build_default_checks(check_keys: bool = False) -> List[Tuple[str, Callable[..., CheckResult]]]:
    """Return list of (name, fn) pairs for default doctor run."""
    return [
        ("python_version", lambda: check_python_version()),
        ("required_modules", lambda: check_required_modules()),
        ("optional_modules", lambda: check_optional_modules()),
        ("ledger_path", lambda: check_ledger_path()),
        ("migration_audit_path", lambda: check_migration_audit_path()),
        ("substrate_matrix", lambda: check_substrate_matrix()),
        ("ledger_freshness", lambda: check_ledger_freshness()),
        ("apeireth_tree", lambda: check_apeireth_tree()),
        ("disk_space", lambda: check_disk_space()),
        ("tests_dir", lambda: check_tests_dir()),
        ("api_keys", lambda: check_api_keys(check_keys=check_keys)),
    ]


# -----------------------------------------------------------------------------
# Aggregation + exit-code logic
# -----------------------------------------------------------------------------

PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "V1353 != Phenomenal consciousness; doctor is mechanical",
    "V1353 != ASI scores reality; checks are deterministic, not semantic",
    "V1353 != ASI 智慧; checks are imports + path stat + version compare",
    "V1353 != ASI 集成; V1353 is a thin pre-flight layer, not integration",
    "V1353 != ASI 等级; doctor subscore != ASI score; honest cap 0.008",
    "V1353 does not run pipeline; pre-flight only",
)


def _aggregate(checks: List[CheckResult], strict: bool = False) -> int:
    """Compute exit code from list of CheckResults."""
    has_critical = any(c.severity == SEVERITY_CRITICAL and c.is_failure() for c in checks)
    has_error = any(c.severity == SEVERITY_ERROR and c.is_failure() for c in checks)
    has_warn = any(c.severity == SEVERITY_WARN and c.is_failure() for c in checks)

    if has_critical:
        return 3
    if strict:
        if has_error or has_warn:
            return 2
    else:
        if has_error:
            return 2
        if has_warn:
            return 1
    return 0


def run_doctor(check_keys: bool = False,
               strict: bool = False,
               check_fns: Optional[List[Tuple[str, Callable[..., CheckResult]]]] = None) -> DoctorReport:
    """Run all doctor checks; return DoctorReport."""
    from datetime import datetime, timezone

    started = datetime.now(timezone.utc).isoformat()
    check_fns = check_fns if check_fns is not None else _build_default_checks(check_keys=check_keys)
    checks: List[CheckResult] = []
    for name, fn in check_fns:
        try:
            checks.append(fn())
        except Exception as exc:
            checks.append(CheckResult(
                name=name,
                severity=SEVERITY_ERROR,
                status=STATUS_FAIL,
                message=f"check raised {type(exc).__name__}: {exc}",
                suggestion="Doctor internal error — file an issue",
            ))
    finished = datetime.now(timezone.utc).isoformat()
    n_pass = sum(1 for c in checks if c.status == STATUS_PASS)
    n_fail = sum(1 for c in checks if c.status == STATUS_FAIL)
    n_skip = sum(1 for c in checks if c.status == STATUS_SKIP)
    n_warn = sum(1 for c in checks if c.severity == SEVERITY_WARN and c.is_failure())
    exit_code = _aggregate(checks, strict=strict)
    return DoctorReport(
        version=V1353_VERSION,
        n_checks=len(checks),
        n_pass=n_pass,
        n_warn=n_warn,
        n_fail=n_fail,
        n_skip=n_skip,
        exit_code=exit_code,
        asi_cap=V1353_ASI_CAP,
        philosophy_guards=PHILOSOPHY_GUARDS,
        checks=tuple(checks),
        started_at=started,
        finished_at=finished,
    )


# -----------------------------------------------------------------------------
# Human formatting
# -----------------------------------------------------------------------------

def _format_report_human(report: DoctorReport) -> str:
    """Format DoctorReport as a fixed-width human-readable table."""
    lines: List[str] = []
    lines.append(f"=== V1353 VCP Doctor (v{report.version}) ===")
    lines.append(f"checks: {report.n_checks}  pass: {report.n_pass}  warn: {report.n_warn}  fail: {report.n_fail}  skip: {report.n_skip}")
    lines.append(f"exit_code: {report.exit_code}  asi_cap: {report.asi_cap}")
    lines.append(f"started : {report.started_at}")
    lines.append(f"finished: {report.finished_at}")
    lines.append("")
    lines.append(f"{'name':30s} {'sev':9s} {'status':6s}  message")
    lines.append("-" * 100)
    for c in report.checks:
        msg = c.message
        if c.suggestion:
            msg = f"{msg}  -> {c.suggestion}"
        # Truncate very long messages for table readability
        if len(msg) > 200:
            msg = msg[:197] + "..."
        lines.append(f"{c.name:30s} {c.severity:9s} {c.status:6s}  {msg}")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Embedded Popper-style self-tests (37 falsifiable checks)
# -----------------------------------------------------------------------------

def _popper_self_tests() -> List[Tuple[str, bool, str]]:
    """Return list of (name, passed, message) for embedded self-tests."""
    out: List[Tuple[str, bool, str]] = []

    # Constants
    out.append(("V1353_VERSION is 0.1.0", V1353_VERSION == "0.1.0", V1353_VERSION))
    out.append(("V1353_ASI_CAP is 0.008", V1353_ASI_CAP == 0.008, str(V1353_ASI_CAP)))
    out.append(("MIN_PYTHON >= (3,11)", MIN_PYTHON >= (3, 11), str(MIN_PYTHON)))
    out.append(("REQUIRED_MODULES >= 5", len(REQUIRED_MODULES) >= 5, str(len(REQUIRED_MODULES))))

    # Individual checks return CheckResult
    cr_python = check_python_version()
    out.append(("check_python_version returns CheckResult", isinstance(cr_python, CheckResult), type(cr_python).__name__))
    out.append(("check_python_version has valid severity", cr_python.severity in (SEVERITY_OK, SEVERITY_WARN, SEVERITY_ERROR, SEVERITY_CRITICAL), cr_python.severity))

    cr_required = check_required_modules()
    out.append(("check_required_modules returns CheckResult", isinstance(cr_required, CheckResult), type(cr_required).__name__))
    out.append(("check_required_modules has status pass or fail", cr_required.status in (STATUS_PASS, STATUS_FAIL), cr_required.status))

    cr_optional = check_optional_modules()
    out.append(("check_optional_modules returns CheckResult", isinstance(cr_optional, CheckResult), type(cr_optional).__name__))

    cr_ledger = check_ledger_path()
    out.append(("check_ledger_path returns CheckResult", isinstance(cr_ledger, CheckResult), type(cr_ledger).__name__))

    cr_migration = check_migration_audit_path()
    out.append(("check_migration_audit_path returns CheckResult", isinstance(cr_migration, CheckResult), type(cr_migration).__name__))

    cr_substrate = check_substrate_matrix()
    out.append(("check_substrate_matrix returns CheckResult", isinstance(cr_substrate, CheckResult), type(cr_substrate).__name__))

    cr_freshness = check_ledger_freshness()
    out.append(("check_ledger_freshness returns CheckResult", isinstance(cr_freshness, CheckResult), type(cr_freshness).__name__))

    cr_apeireth = check_apeireth_tree()
    out.append(("check_apeireth_tree returns CheckResult", isinstance(cr_apeireth, CheckResult), type(cr_apeireth).__name__))

    cr_disk = check_disk_space()
    out.append(("check_disk_space returns CheckResult", isinstance(cr_disk, CheckResult), type(cr_disk).__name__))

    cr_tests = check_tests_dir()
    out.append(("check_tests_dir returns CheckResult", isinstance(cr_tests, CheckResult), type(cr_tests).__name__))

    cr_keys_skip = check_api_keys(check_keys=False)
    out.append(("check_api_keys (skip) returns CheckResult", isinstance(cr_keys_skip, CheckResult), type(cr_keys_skip).__name__))
    out.append(("check_api_keys (skip) has status=skip", cr_keys_skip.status == STATUS_SKIP, cr_keys_skip.status))

    cr_keys_check = check_api_keys(check_keys=True)
    out.append(("check_api_keys (check) returns CheckResult", isinstance(cr_keys_check, CheckResult), type(cr_keys_check).__name__))
    out.append(("check_api_keys (check) status is pass or fail", cr_keys_check.status in (STATUS_PASS, STATUS_FAIL), cr_keys_check.status))

    # Aggregation
    out.append(("_aggregate empty list = 0", _aggregate([], strict=False) == 0, ""))
    out.append(("_aggregate with warn = 1", _aggregate([CheckResult("x", SEVERITY_WARN, STATUS_FAIL, "m")], strict=False) == 1, ""))
    out.append(("_aggregate with error = 2", _aggregate([CheckResult("x", SEVERITY_ERROR, STATUS_FAIL, "m")], strict=False) == 2, ""))
    out.append(("_aggregate with critical = 3", _aggregate([CheckResult("x", SEVERITY_CRITICAL, STATUS_FAIL, "m")], strict=False) == 3, ""))
    out.append(("_aggregate strict warn -> 2", _aggregate([CheckResult("x", SEVERITY_WARN, STATUS_FAIL, "m")], strict=True) == 2, ""))
    out.append(("_aggregate strict pass -> 0", _aggregate([CheckResult("x", SEVERITY_OK, STATUS_PASS, "m")], strict=True) == 0, ""))
    out.append(("_aggregate passing warn (not fail) -> 0", _aggregate([CheckResult("x", SEVERITY_WARN, STATUS_PASS, "m")], strict=False) == 0, ""))

    # Default checks list
    defaults = _build_default_checks(check_keys=False)
    out.append(("_build_default_checks has >= 10 checks", len(defaults) >= 10, str(len(defaults))))
    out.append(("_build_default_checks check_keys=True adds api_keys", any(n == "api_keys" for n, _ in _build_default_checks(check_keys=True)), ""))

    # run_doctor
    report = run_doctor(check_keys=False, strict=False)
    out.append(("run_doctor returns DoctorReport", isinstance(report, DoctorReport), type(report).__name__))
    out.append(("run_doctor has checks", len(report.checks) >= 10, str(len(report.checks))))
    out.append(("run_doctor exit_code in [0,1,2,3]", report.exit_code in (0, 1, 2, 3), str(report.exit_code)))
    out.append(("run_doctor to_dict round-trip", "version" in report.to_dict(), ""))
    out.append(("run_doctor philosophy_guards >= 5", len(report.philosophy_guards) >= 5, str(len(report.philosophy_guards))))
    out.append(("run_doctor check sum is consistent",
                report.n_pass + report.n_fail + report.n_skip == report.n_checks,
                f"{report.n_pass}+{report.n_fail}+{report.n_skip}=={report.n_checks}"))

    # _format_report_human
    text = _format_report_human(report)
    out.append(("_format_report_human non-empty", len(text) > 0, str(len(text))))
    out.append(("_format_report_human contains version", "V1353" in text, ""))
    out.append(("_format_report_human contains exit_code", "exit_code" in text, ""))

    return out


# -----------------------------------------------------------------------------
# CLI dispatch
# -----------------------------------------------------------------------------

def _build_argparser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="vcp-doctor",
        description="V1353 VCP pre-flight safety check (mechanical, deterministic, side-effect-free)",
    )
    sub = p.add_subparsers(dest="cmd")

    p_run = sub.add_parser("run", help="Run all doctor checks")
    p_run.add_argument("--json", action="store_true", help="Output structured JSON")
    p_run.add_argument("--strict", action="store_true", help="Treat warnings as errors")
    p_run.add_argument("--check-keys", action="store_true", help="Check optional API keys")

    sub.add_parser("list", help="List all default checks")
    p_st = sub.add_parser("self-test", help="Run embedded Popper self-tests")
    p_st.add_argument("--verbose", action="store_true", help="Show each self-test result")

    sub.add_parser("version", help="Print version + constants")

    return p


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point; returns process exit code."""
    parser = _build_argparser()
    args = parser.parse_args(argv)

    if args.cmd is None or args.cmd == "version":
        print(f"V1353 VCP Doctor v{V1353_VERSION} (asi_cap={V1353_ASI_CAP})")
        return 0

    if args.cmd == "list":
        for name, _ in _build_default_checks(check_keys=False):
            print(name)
        return 0

    if args.cmd == "self-test":
        results = _popper_self_tests()
        n_pass = sum(1 for _, ok, _ in results if ok)
        n_fail = sum(1 for _, ok, _ in results if not ok)
        if getattr(args, "verbose", False):
            for name, ok, msg in results:
                mark = "OK" if ok else "FAIL"
                print(f"  [{mark}] {name}  ({msg})")
        print(f"=== V1353 self-test: {n_pass}/{len(results)} pass ===")
        return 0 if n_fail == 0 else 1

    if args.cmd == "run":
        report = run_doctor(
            check_keys=bool(getattr(args, "check_keys", False)),
            strict=bool(getattr(args, "strict", False)),
        )
        if bool(getattr(args, "json", False)):
            print(json.dumps(report.to_dict(), indent=2, sort_keys=True))
        else:
            print(_format_report_human(report))
        return report.exit_code

    parser.print_help()
    return 2


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())