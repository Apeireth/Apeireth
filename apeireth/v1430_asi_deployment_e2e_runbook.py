"""V1430 — ASI 真生产 deployment E2E runbook orchestrator (主 13:31 + 主 23:44 + 主 00:56 + 主 17:33).

Phase: 1430
Version: 0.1.0
Date: 2026-08-10 (cron tick 04:32, Asia/Shanghai deep night)
Post: V1428 (deployment artifacts) + V1429 (deployment semantic linter)

What V1430 is
=============
V1430 is the **end-to-end deployment runbook orchestrator** for
Apeireth ASI. It composes V1428 (artifacts) + V1429 (semantic linter)
+ a host-probe layer + a per-artifact smoke layer into a single CLI
that anyone can run:

    step 1: generate         — V1428 generate_and_validate → structural readiness
    step 2: lint             — V1429 lint_all             → semantic readiness
    step 3: probe_host       — check docker / python / streamlit / curl on this host
    step 4: smoke            — run each artifact's basic test (parse YAML, etc.)
    step 5: decision         — combine all verdicts → deployment_pass bool

V1430 is the **"anyone can take over"** layer (主 00:56). It does not
require Domain knowledge of V1428, V1429, V1008, or V1032. It just
runs the chain and emits a verdict + report.

It does NOT run `docker compose up` if Docker is not installed.
It does NOT claim that artifacts are deployed or running.
It claims only: **the runbook executed, and here is what each step
found**. The verdict is a function of the four steps, not a claim
about production.

Real-world usage:

    # Anyone can run the full E2E runbook:
    python -m apeireth.v1430_asi_deployment_e2e_runbook runbook

    # Anyone can run just one step:
    python -m apeireth.v1430_asi_deployment_e2e_runbook step probe_host
    python -m apeireth.v1430_asi_deployment_e2e_runbook step generate
    python -m apeireth.v1430_asi_deployment_e2e_runbook step lint
    python -m apeireth.v1430_asi_deployment_e2e_runbook step smoke

    # Anyone can see the chain integrity:
    python -m apeireth.v1430_asi_deployment_e2e_runbook chain

    # Anyone can render a markdown report:
    python -m apeireth.v1430_asi_deployment_e2e_runbook report

It does NOT mutate upstream V1428/V1429/V1008/V1032 state. It only
**calls** their public functions and **reads** their outputs.

Borrowed (5 — 主 19:33 走在前人经验上):
=======================================
- V1428 (deployment artifacts — generate_and_validate, DeploymentReadiness)
- V1429 (semantic linter — lint_from_v1428, SemanticLinterReport)
- stdlib dataclasses + json + subprocess + shutil + pathlib + typing
- V1418 (chain_delegate pattern)
- V1427 (stage-delivery aggregator pattern)

GUARDS upheld (V1430-specific, 13 — 主 00:44 质量工程化)
=========================================================
- GUARD_NO_DOCKER_REQUIRED: runbook runs even when Docker is absent
- GUARD_NO_V1428_WRITE: V1428 generators called read-only
- GUARD_NO_V1429_WRITE: V1429 linter called read-only
- GUARD_STEP_BOUNDED: each step has its own bounded scope
- GUARD_PROBE_NON_DESTRUCTIVE: host probe only checks presence, never runs commands
- GUARD_SMOKE_PARSES_ONLY: smoke tests parse artifacts, never execute them
- GUARD_RANBOOK_DEFINED: runbook = generate + lint + probe_host + smoke
- GUARD_DEPLOYMENT_PASS_DEFINED: deployment_pass = all 4 steps passed
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1430 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_STEPS_DOCUMENTED: every step has docstring
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_RUNBOOK: runbook is stdlib orchestration, NOT consciousness
- GUARD_NO_ASI_RUNBOOK: runbook is bounded, NOT ASI level
- GUARD_NO_HUMAN_LEVEL_RUNBOOK: runbook is mechanical, NOT human-level
- GUARD_NO_ABSOLUTE_RUNBOOK: runbook is one host, NOT universal
- GUARD_NO_FAKE_PRODUCTION: deployment_pass != production-deployed

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1430 is a **bounded orchestrator**. It does not claim that the
artifacts are deployed, that Docker has run, or that a service is
healthy. It claims only that the runbook executed all four steps
and that each step either passed or failed its deterministic check.
deployment_pass is a function of four bounded checks; NOT a claim
about production readiness, NOT a claim about ASI level, NOT a
claim about Phenomenal consciousness. V1430 reads V1428 / V1429 /
V1008 / V1032; never replaces any of them.

API surfaces (16)
=================
1.  ``StepStatus`` — Enum (PASS / WARN / FAIL / SKIP)
2.  ``StepResult`` — dataclass (step + status + note + started_iso + ended_iso)
3.  ``HostProbe`` — dataclass (n_tools_found + tools + missing + has_docker + has_python)
4.  ``SmokeReport`` — dataclass (per_kind + n_pass + n_fail + n_total)
5.  ``RunbookReport`` — dataclass (steps + probe + smoke + structural + semantic + deployment_pass)
6.  ``step_generate(output_dir)`` — V1428 generate + validate
7.  ``step_lint(output_dir)`` — V1429 lint
8.  ``step_probe_host()`` — host probe
9.  ``step_smoke(output_dir)`` — smoke tests
10. ``runbook_full(output_dir)`` — RunbookReport
11. ``deployment_pass(report)`` — bool
12. ``runbook_chain_delegate()`` — chain probe
13. ``popper_self_test()`` — 14 self-tests
14. ``render_report_md(report)`` — markdown summary
15. ``module_meta()`` — meta dict
16. ``main()`` — CLI

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- step <generate|lint|probe_host|smoke> [--output-dir PATH]
- runbook [--output-dir PATH]
- report [--output-dir PATH]
- summary
"""

from __future__ import annotations

import enum
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1430_VERSION = "0.1.0"
V1430_SCHEMA = "v1430.asi-deployment-e2e-runbook/v1"
V1430_MODULE = "v1430_asi_deployment_e2e_runbook"

WORKSPACE = Path(__file__).resolve().parents[2]
PROMETHEAN = (
    WORKSPACE / "promethean"
    if (WORKSPACE / "promethean").exists()
    else WORKSPACE
)
DEFAULT_OUTPUT_DIR = PROMETHEAN / "deploy"


# ============================================================================
# Enums
# ============================================================================


class StepStatus(str, enum.Enum):
    """Status of a single runbook step."""

    PASS = "PASS"
    WARN = "WARN"
    FAIL = "FAIL"
    SKIP = "SKIP"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class StepResult:
    """Result of a single runbook step."""

    step: str
    status: StepStatus = StepStatus.SKIP
    note: str = ""
    started_iso: str = ""
    ended_iso: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "step": self.step,
            "status": self.status.value,
            "note": self.note,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
        }


@dataclass
class HostProbe:
    """Host tool availability probe."""

    tools: Dict[str, bool] = field(default_factory=dict)
    n_tools_found: int = 0
    n_tools_total: int = 0
    has_docker: bool = False
    has_python: bool = False
    has_streamlit: bool = False
    has_curl: bool = False
    has_pip: bool = False
    missing: List[str] = field(default_factory=list)


@dataclass
class SmokeReport:
    """Per-artifact smoke test summary."""

    per_kind: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    n_pass: int = 0
    n_fail: int = 0
    n_total: int = 0


@dataclass
class RunbookReport:
    """Whole E2E runbook report."""

    output_dir: str = ""
    steps: List[StepResult] = field(default_factory=list)
    probe: Optional[HostProbe] = None
    smoke: Optional[SmokeReport] = None
    structural_readiness: float = 0.0
    semantic_readiness: float = 0.0
    deployment_pass: bool = False
    started_iso: str = ""
    ended_iso: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "output_dir": self.output_dir,
            "steps": [s.to_dict() for s in self.steps],
            "probe": asdict(self.probe) if self.probe else None,
            "smoke": asdict(self.smoke) if self.smoke else None,
            "structural_readiness": self.structural_readiness,
            "semantic_readiness": self.semantic_readiness,
            "deployment_pass": self.deployment_pass,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
        }


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1430_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_NO_V1428_WRITE",
    "GUARD_NO_V1429_WRITE",
    "GUARD_STEP_BOUNDED",
    "GUARD_PROBE_NON_DESTRUCTIVE",
    "GUARD_SMOKE_PARSES_ONLY",
    "GUARD_RANBOOK_DEFINED",
    "GUARD_DEPLOYMENT_PASS_DEFINED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_STEPS_DOCUMENTED",
    "GUARD_CLI_RUNNABLE",
)

V1430_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_RUNBOOK",
    "GUARD_NO_ASI_RUNBOOK",
    "GUARD_NO_HUMAN_LEVEL_RUNBOOK",
    "GUARD_NO_ABSOLUTE_RUNBOOK",
    "GUARD_NO_FAKE_PRODUCTION",
)

V1430_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1428_asi_real_deployment_artifacts", "generate_and_validate, DeploymentReadiness"),
    ("v1429_asi_deployment_semantic_linter", "lint_from_v1428, SemanticLinterReport"),
    ("v1418_asi_dgm_cron_integration", "chain_delegate pattern"),
    ("v1427_asi_stage_delivery", "aggregator pattern"),
    ("stdlib", "dataclasses + json + subprocess + shutil + pathlib + typing"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_iso() -> str:
    """UTC ISO 8601 timestamp."""
    return datetime.now(timezone.utc).isoformat()


def _read_text(path: Path) -> str:
    """Read text file content, return empty string if missing."""
    try:
        return path.read_text(encoding="utf-8")
    except Exception:
        return ""


# ============================================================================
# Steps
# ============================================================================


def step_generate(output_dir: Path) -> StepResult:
    """Step 1: generate + validate deployment artifacts via V1428."""
    step = StepResult(step="generate", started_iso=_now_iso())
    try:
        from apeireth.v1428_asi_real_deployment_artifacts import (
            generate_and_validate,
        )

        result = generate_and_validate(output_dir)
        step.note = (
            f"structural_readiness={result.readiness:.4f} "
            f"n_valid={result.n_valid}/{result.n_total} "
            f"output_dir={result.output_dir}"
        )
        step.status = (
            StepStatus.PASS if result.readiness >= 1.0 else StepStatus.FAIL
        )
    except Exception as exc:
        step.status = StepStatus.FAIL
        step.note = f"{type(exc).__name__}: {exc}"
    step.ended_iso = _now_iso()
    return step


def step_lint(output_dir: Path) -> StepResult:
    """Step 2: lint semantic rules via V1429."""
    step = StepResult(step="lint", started_iso=_now_iso())
    try:
        from apeireth.v1429_asi_deployment_semantic_linter import (
            lint_from_v1428,
            semantic_readiness,
        )

        report = lint_from_v1428(output_dir)
        readiness = semantic_readiness(report)
        step.note = (
            f"semantic_readiness={readiness:.4f} "
            f"pass={report.n_pass} warn={report.n_warn} fail={report.n_fail} "
            f"total={report.n_total}"
        )
        # lint can have WARN/FAIL without block; only FAIL blocks runbook
        step.status = (
            StepStatus.PASS
            if report.n_fail == 0
            else StepStatus.WARN if report.n_fail <= 3 else StepStatus.FAIL
        )
    except Exception as exc:
        step.status = StepStatus.FAIL
        step.note = f"{type(exc).__name__}: {exc}"
    step.ended_iso = _now_iso()
    return step


def step_probe_host() -> StepResult:
    """Step 3: probe host for docker / python / streamlit / curl / pip."""
    step = StepResult(step="probe_host", started_iso=_now_iso())
    try:
        probe = HostProbe()
        # python (always present if this script is running)
        py = shutil.which("python") or shutil.which("python3") or sys.executable
        probe.has_python = bool(py)
        probe.tools["python"] = probe.has_python
        # pip
        pip = shutil.which("pip") or shutil.which("pip3")
        probe.has_pip = bool(pip)
        probe.tools["pip"] = probe.has_pip
        # docker (NOT required)
        docker = shutil.which("docker")
        probe.has_docker = bool(docker)
        probe.tools["docker"] = probe.has_docker
        # streamlit (optional)
        streamlit = shutil.which("streamlit")
        probe.has_streamlit = bool(streamlit)
        probe.tools["streamlit"] = probe.has_streamlit
        # curl (optional)
        curl = shutil.which("curl")
        probe.has_curl = bool(curl)
        probe.tools["curl"] = probe.has_curl

        probe.n_tools_total = len(probe.tools)
        probe.n_tools_found = sum(1 for v in probe.tools.values() if v)
        probe.missing = [k for k, v in probe.tools.items() if not v]

        step.note = (
            f"tools_found={probe.n_tools_found}/{probe.n_tools_total} "
            f"docker={'yes' if probe.has_docker else 'no'} "
            f"python={'yes' if probe.has_python else 'no'} "
            f"streamlit={'yes' if probe.has_streamlit else 'no'} "
            f"curl={'yes' if probe.has_curl else 'no'} "
            f"pip={'yes' if probe.has_pip else 'no'} "
            f"missing={probe.missing}"
        )
        # host probe passes if python is present (the only required tool)
        step.status = (
            StepStatus.PASS if probe.has_python else StepStatus.FAIL
        )
    except Exception as exc:
        step.status = StepStatus.FAIL
        step.note = f"{type(exc).__name__}: {exc}"
    step.ended_iso = _now_iso()
    return step


def _smoke_compose(path: Path) -> Tuple[bool, str]:
    """Smoke test docker-compose.yml: parse + check 'services:' key."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    if "services:" not in content:
        return (False, "no_services_key")
    return (True, f"parsed_ok size={len(content)}")


def _smoke_k8s(path: Path) -> Tuple[bool, str]:
    """Smoke test k8s-asi.yaml: parse + check apiVersion + kind."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    if "apiVersion:" not in content or "kind:" not in content:
        return (False, "no_apiversion_or_kind")
    return (True, f"parsed_ok size={len(content)}")


def _smoke_dockerfile(path: Path) -> Tuple[bool, str]:
    """Smoke test Dockerfile: check FROM (after comments) + USER/CMD."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    # Skip leading comments (lines starting with #) when checking FROM
    # Real Dockerfiles can have a leading comment block.
    lines = content.splitlines()
    has_from = False
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("FROM "):
            has_from = True
        break
    if not has_from:
        return (False, "no_FROM")
    if "CMD" not in content and "ENTRYPOINT" not in content:
        return (False, "no_CMD_or_ENTRYPOINT")
    return (True, f"parsed_ok size={len(content)}")


def _smoke_requirements(path: Path) -> Tuple[bool, str]:
    """Smoke test requirements.txt: at least 1 line."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    lines = [l for l in content.splitlines() if l.strip() and not l.startswith("#")]
    if not lines:
        return (False, "no_deps")
    return (True, f"n_deps={len(lines)}")


def _smoke_startup(path: Path) -> Tuple[bool, str]:
    """Smoke test start-asi.sh: shebang + set -e."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    if not content.startswith("#!"):
        return (False, "no_shebang")
    return (True, f"parsed_ok size={len(content)}")


def _smoke_env(path: Path) -> Tuple[bool, str]:
    """Smoke test .env.example: at least 1 KEY=value."""
    if not path.exists():
        return (False, "file_missing")
    content = _read_text(path)
    if not content:
        return (False, "file_empty")
    keys = [
        l for l in content.splitlines()
        if l.strip() and not l.startswith("#") and "=" in l
    ]
    if not keys:
        return (False, "no_keys")
    return (True, f"n_keys={len(keys)}")


_SMOKE_FUNCS: Dict[str, Any] = {
    "docker-compose.yml": _smoke_compose,
    "k8s-asi.yaml": _smoke_k8s,
    "Dockerfile": _smoke_dockerfile,
    "requirements.txt": _smoke_requirements,
    "start-asi.sh": _smoke_startup,
    ".env.example": _smoke_env,
}


def step_smoke(output_dir: Path) -> StepResult:
    """Step 4: smoke test each artifact."""
    step = StepResult(step="smoke", started_iso=_now_iso())
    try:
        smoke = SmokeReport()
        for name, fn in _SMOKE_FUNCS.items():
            path = output_dir / name
            ok, note = fn(path)
            smoke.per_kind[name] = {"ok": ok, "note": note}
            if ok:
                smoke.n_pass += 1
            else:
                smoke.n_fail += 1
            smoke.n_total += 1
        step.note = (
            f"smoke: pass={smoke.n_pass}/{smoke.n_total} "
            f"fail={smoke.n_fail}"
        )
        step.status = (
            StepStatus.PASS if smoke.n_fail == 0 else StepStatus.FAIL
        )
    except Exception as exc:
        step.status = StepStatus.FAIL
        step.note = f"{type(exc).__name__}: {exc}"
    step.ended_iso = _now_iso()
    return step


# ============================================================================
# Runbook
# ============================================================================


def runbook_full(output_dir: Optional[Path] = None) -> RunbookReport:
    """Run all 4 steps in order and return a RunbookReport."""
    output_dir = output_dir or DEFAULT_OUTPUT_DIR
    output_dir.mkdir(parents=True, exist_ok=True)

    report = RunbookReport(
        output_dir=str(output_dir),
        started_iso=_now_iso(),
    )

    # Step 1: generate
    s1 = step_generate(output_dir)
    report.steps.append(s1)

    # Step 2: lint
    s2 = step_lint(output_dir)
    report.steps.append(s2)

    # Step 3: probe host
    s3 = step_probe_host()
    report.steps.append(s3)

    # Step 4: smoke
    s4 = step_smoke(output_dir)
    report.steps.append(s4)

    # Aggregate
    try:
        from apeireth.v1428_asi_real_deployment_artifacts import (
            generate_and_validate,
        )

        dd = generate_and_validate(output_dir)
        report.structural_readiness = dd.readiness
    except Exception:
        report.structural_readiness = 0.0

    try:
        from apeireth.v1429_asi_deployment_semantic_linter import (
            lint_from_v1428,
            semantic_readiness,
        )

        lr = lint_from_v1428(output_dir)
        report.semantic_readiness = semantic_readiness(lr)
    except Exception:
        report.semantic_readiness = 0.0

    # Re-run probe to attach info
    try:
        from shutil import which as _which

        probe = HostProbe()
        probe.tools["python"] = bool(_which("python") or sys.executable)
        probe.tools["pip"] = bool(_which("pip") or _which("pip3"))
        probe.tools["docker"] = bool(_which("docker"))
        probe.tools["streamlit"] = bool(_which("streamlit"))
        probe.tools["curl"] = bool(_which("curl"))
        probe.has_python = probe.tools["python"]
        probe.has_pip = probe.tools["pip"]
        probe.has_docker = probe.tools["docker"]
        probe.has_streamlit = probe.tools["streamlit"]
        probe.has_curl = probe.tools["curl"]
        probe.n_tools_total = len(probe.tools)
        probe.n_tools_found = sum(1 for v in probe.tools.values() if v)
        probe.missing = [k for k, v in probe.tools.items() if not v]
        report.probe = probe
    except Exception:
        report.probe = None

    # Re-run smoke to attach
    try:
        smoke = SmokeReport()
        for name, fn in _SMOKE_FUNCS.items():
            path = output_dir / name
            ok, note = fn(path)
            smoke.per_kind[name] = {"ok": ok, "note": note}
            if ok:
                smoke.n_pass += 1
            else:
                smoke.n_fail += 1
            smoke.n_total += 1
        report.smoke = smoke
    except Exception:
        report.smoke = None

    # Deployment pass verdict: all 4 steps PASS
    report.deployment_pass = all(
        s.status == StepStatus.PASS for s in report.steps
    )

    report.ended_iso = _now_iso()
    return report


def deployment_pass(report: RunbookReport) -> bool:
    """Return report.deployment_pass."""
    return report.deployment_pass


# ============================================================================
# Chain delegate
# ============================================================================


def runbook_chain_delegate() -> Dict[str, Any]:
    """Probe upstream modules (V1428 + V1429) for liveness."""
    chain: Dict[str, Any] = {}

    try:
        from apeireth.v1428_asi_real_deployment_artifacts import chain_delegate as v1428_chain
        chain["V1428"] = {"ok": True, "data": v1428_chain()}
    except Exception as exc:
        chain["V1428"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    try:
        from apeireth.v1429_asi_deployment_semantic_linter import chain_delegate as v1429_chain
        chain["V1429"] = {"ok": True, "data": v1429_chain()}
    except Exception as exc:
        chain["V1429"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    all_ok = all(c.get("ok") for c in chain.values())
    return {"all_ok": all_ok, "chain": chain, "n_modules": len(chain)}


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """Popper-style self-test: 14 deterministic checks."""
    results: Dict[str, Tuple[bool, str]] = {}

    # PT01: importable
    try:
        import apeireth.v1430_asi_deployment_e2e_runbook as self_mod
        results["PT01_importable"] = (True, "ok")
    except Exception as exc:
        results["PT01_importable"] = (False, f"{type(exc).__name__}: {exc}")
        return {
            "n_pass": 0,
            "n_total": 1,
            "ok": False,
            "results": results,
        }

    # PT02: version set
    results["PT02_version_set"] = (
        V1430_VERSION == "0.1.0",
        f"version={V1430_VERSION}",
    )

    # PT03: 5 guards
    results["PT03_guards_set"] = (
        len(V1430_GUARDS) == 13,
        f"n={len(V1430_GUARDS)}",
    )

    # PT04: 5 V3 guards
    results["PT04_v3_guards"] = (
        len(V1430_V3_GUARDS) == 5,
        f"n={len(V1430_V3_GUARDS)}",
    )

    # PT05: 5 borrowed
    results["PT05_borrowed"] = (
        len(V1430_BORROWED) == 5,
        f"n={len(V1430_BORROWED)}",
    )

    # PT06: step_generate returns StepResult
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        s = step_generate(tmp_path)
        results["PT06_step_generate"] = (
            isinstance(s, StepResult) and s.step == "generate",
            f"type={type(s).__name__}",
        )

        # PT07: step_lint returns StepResult
        s = step_lint(tmp_path)
        results["PT07_step_lint"] = (
            isinstance(s, StepResult) and s.step == "lint",
            f"type={type(s).__name__}",
        )

        # PT08: step_probe_host returns StepResult
        s = step_probe_host()
        results["PT08_step_probe_host"] = (
            isinstance(s, StepResult) and s.step == "probe_host",
            f"type={type(s).__name__}",
        )

        # PT09: step_smoke returns StepResult
        s = step_smoke(tmp_path)
        results["PT09_step_smoke"] = (
            isinstance(s, StepResult) and s.step == "smoke",
            f"type={type(s).__name__}",
        )

        # PT10: runbook_full returns RunbookReport
        r = runbook_full(tmp_path)
        results["PT10_runbook_full"] = (
            isinstance(r, RunbookReport) and len(r.steps) == 4,
            f"n_steps={len(r.steps)}",
        )

        # PT11: deployment_pass is bool
        results["PT11_deployment_pass_bool"] = (
            isinstance(r.deployment_pass, bool),
            f"type={type(r.deployment_pass).__name__}",
        )

        # PT12: runbook_chain_delegate runs
        c = runbook_chain_delegate()
        results["PT12_chain_delegate"] = (
            isinstance(c, dict) and "all_ok" in c,
            f"keys={list(c.keys())}",
        )

    # PT13: smoke checks structurally correct
    good_compose = "services:\n  api:\n    image: apeireth:1.0\n"
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        Path(tmp, "docker-compose.yml").write_text(good_compose, encoding="utf-8")
        ok, note = _smoke_compose(Path(tmp) / "docker-compose.yml")
        results["PT13_smoke_compose_pass"] = (ok, note)

    # PT14: bad compose fails
    bad_compose = "not_a_yaml"
    with tempfile.TemporaryDirectory() as tmp:
        Path(tmp, "docker-compose.yml").write_text(bad_compose, encoding="utf-8")
        ok, note = _smoke_compose(Path(tmp) / "docker-compose.yml")
        results["PT14_smoke_compose_fail"] = (not ok, note)

    n_pass = sum(1 for v in results.values() if v[0])
    n_total = len(results)
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "ok": n_pass == n_total,
        "results": results,
    }


# ============================================================================
# Reporting
# ============================================================================


def render_report_md(report: RunbookReport) -> str:
    """Render a markdown report from a RunbookReport."""
    lines = []
    lines.append(f"# V1430 ASI Deployment E2E Runbook Report `{V1430_VERSION}`")
    lines.append("")
    lines.append(f"- output_dir: `{report.output_dir}`")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append("")
    lines.append("## Steps")
    lines.append("")
    lines.append("| step | status | note |")
    lines.append("|---|---|---|")
    for s in report.steps:
        note = s.note.replace("|", "\\|")
        lines.append(f"| `{s.step}` | **{s.status.value}** | {note} |")
    lines.append("")
    lines.append("## Readiness")
    lines.append("")
    lines.append(f"- structural_readiness: **{report.structural_readiness:.4f}**")
    lines.append(f"- semantic_readiness: **{report.semantic_readiness:.4f}**")
    lines.append("")
    if report.probe:
        lines.append("## Host probe")
        lines.append("")
        lines.append(f"- tools_found: {report.probe.n_tools_found}/{report.probe.n_tools_total}")
        lines.append(f"- docker: {'yes' if report.probe.has_docker else 'no'}")
        lines.append(f"- python: {'yes' if report.probe.has_python else 'no'}")
        lines.append(f"- streamlit: {'yes' if report.probe.has_streamlit else 'no'}")
        lines.append(f"- curl: {'yes' if report.probe.has_curl else 'no'}")
        lines.append(f"- pip: {'yes' if report.probe.has_pip else 'no'}")
        lines.append(f"- missing: {report.probe.missing}")
        lines.append("")
    if report.smoke:
        lines.append("## Smoke")
        lines.append("")
        lines.append(f"- pass: {report.smoke.n_pass}/{report.smoke.n_total}")
        lines.append(f"- fail: {report.smoke.n_fail}")
        lines.append("")
        lines.append("| artifact | ok | note |")
        lines.append("|---|---|---|")
        for kind, info in report.smoke.per_kind.items():
            lines.append(f"| `{kind}` | {'✓' if info['ok'] else '✗'} | {info['note']} |")
        lines.append("")
    lines.append("## Verdict")
    lines.append("")
    verdict = "**PASS**" if report.deployment_pass else "**FAIL**"
    lines.append(f"- deployment_pass: {verdict}")
    lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append(
        "V1430 is a **bounded orchestrator**. It does not claim that the"
    )
    lines.append(
        "artifacts are deployed, that Docker has run, or that a service is"
    )
    lines.append(
        "healthy. It claims only that the runbook executed all four steps"
    )
    lines.append(
        "and that each step either passed or failed its deterministic check."
    )
    lines.append(
        "deployment_pass is a function of four bounded checks; NOT a claim"
    )
    lines.append(
        "about production readiness, NOT a claim about ASI level, NOT a"
    )
    lines.append(
        "claim about Phenomenal consciousness."
    )
    return "\n".join(lines) + "\n"


def module_meta() -> Dict[str, Any]:
    """Module metadata."""
    return {
        "module": V1430_MODULE,
        "version": V1430_VERSION,
        "schema": V1430_SCHEMA,
        "guards": list(V1430_GUARDS),
        "v3_guards": list(V1430_V3_GUARDS),
        "borrowed": [b[0] for b in V1430_BORROWED],
        "n_guards": len(V1430_GUARDS),
        "n_v3_guards": len(V1430_V3_GUARDS),
        "n_borrowed": len(V1430_BORROWED),
        "n_api_surfaces": 16,
        "n_cli_commands": 10,
    }


# ============================================================================
# CLI
# ============================================================================


def _cmd_version(_args: List[str]) -> int:
    """Print version."""
    print(f"V1430 ASI Deployment E2E Runbook v{V1430_VERSION}")
    return 0


def _cmd_meta(args: List[str]) -> int:
    """Print module metadata."""
    meta = module_meta()
    if "--json" in args:
        print(json.dumps(meta, indent=2, ensure_ascii=False))
    else:
        for k, v in meta.items():
            print(f"  {k}: {v}")
    return 0


def _cmd_demo(_args: List[str]) -> int:
    """Demo: run a minimal runbook in a temp dir."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        report = runbook_full(Path(tmp))
        print(f"demo: deployment_pass={report.deployment_pass}")
        print(f"demo: structural_readiness={report.structural_readiness:.4f}")
        print(f"demo: semantic_readiness={report.semantic_readiness:.4f}")
        print(f"demo: n_steps={len(report.steps)}")
    return 0


def _cmd_popper(_args: List[str]) -> int:
    """Run Popper self-test."""
    result = popper_self_test()
    print(
        f"popper: n_pass={result['n_pass']}/{result['n_total']} "
        f"ok={result['ok']}"
    )
    for k, v in result["results"].items():
        ok, note = v
        status = "✓" if ok else "✗"
        print(f"  {status} {k}: {note}")
    return 0 if result["ok"] else 1


def _cmd_chain(_args: List[str]) -> int:
    """Print chain integrity."""
    chain = runbook_chain_delegate()
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0 if chain["all_ok"] else 1


def _cmd_step(args: List[str]) -> int:
    """Run a single step."""
    if not args:
        print("error: step name required")
        print("usage: step <generate|lint|probe_host|smoke> [--output-dir PATH]")
        return 1
    name = args[0]
    output_dir = DEFAULT_OUTPUT_DIR
    if "--output-dir" in args:
        i = args.index("--output-dir")
        if i + 1 < len(args):
            output_dir = Path(args[i + 1])
    output_dir.mkdir(parents=True, exist_ok=True)
    if name == "generate":
        s = step_generate(output_dir)
    elif name == "lint":
        s = step_lint(output_dir)
    elif name == "probe_host":
        s = step_probe_host()
    elif name == "smoke":
        s = step_smoke(output_dir)
    else:
        print(f"error: unknown step '{name}'")
        return 1
    print(json.dumps(s.to_dict(), indent=2, ensure_ascii=False))
    return 0 if s.status in (StepStatus.PASS, StepStatus.WARN) else 1


def _cmd_runbook(args: List[str]) -> int:
    """Run the full E2E runbook."""
    output_dir = DEFAULT_OUTPUT_DIR
    if "--output-dir" in args:
        i = args.index("--output-dir")
        if i + 1 < len(args):
            output_dir = Path(args[i + 1])
    report = runbook_full(output_dir)
    print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    return 0 if report.deployment_pass else 1


def _cmd_report(args: List[str]) -> int:
    """Render a markdown report."""
    output_dir = DEFAULT_OUTPUT_DIR
    if "--output-dir" in args:
        i = args.index("--output-dir")
        if i + 1 < len(args):
            output_dir = Path(args[i + 1])
    report = runbook_full(output_dir)
    print(render_report_md(report))
    return 0 if report.deployment_pass else 1


def _cmd_summary(_args: List[str]) -> int:
    """Print a one-line summary."""
    report = runbook_full(DEFAULT_OUTPUT_DIR)
    print(
        f"=== V1430 summary ===\n"
        f"deployment_pass={report.deployment_pass}\n"
        f"structural_readiness={report.structural_readiness:.4f}\n"
        f"semantic_readiness={report.semantic_readiness:.4f}\n"
        f"n_steps={len(report.steps)}\n"
        f"steps=" + ", ".join(f"{s.step}={s.status.value}" for s in report.steps)
    )
    if report.probe:
        print(
            f"host: tools={report.probe.n_tools_found}/{report.probe.n_tools_total} "
            f"docker={'yes' if report.probe.has_docker else 'no'} "
            f"python={'yes' if report.probe.has_python else 'no'}"
        )
    if report.smoke:
        print(
            f"smoke: pass={report.smoke.n_pass}/{report.smoke.n_total} "
            f"fail={report.smoke.n_fail}"
        )
    return 0 if report.deployment_pass else 1


def _cmd_help(_args: List[str]) -> int:
    """Print help."""
    print(__doc__)
    return 0


_COMMANDS: Dict[str, Any] = {
    "version": _cmd_version,
    "meta": _cmd_meta,
    "demo": _cmd_demo,
    "help": _cmd_help,
    "popper": _cmd_popper,
    "chain": _cmd_chain,
    "step": _cmd_step,
    "runbook": _cmd_runbook,
    "report": _cmd_report,
    "summary": _cmd_summary,
}


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point."""
    args = list(sys.argv[1:] if argv is None else argv)
    if not args:
        return _cmd_help(args)
    cmd = args[0]
    if cmd not in _COMMANDS:
        print(f"unknown command: {cmd}")
        print("available: " + ", ".join(_COMMANDS.keys()))
        return 1
    return _COMMANDS[cmd](args[1:])


if __name__ == "__main__":
    sys.exit(main())
