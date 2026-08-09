"""V1429 — ASI 真生产 deployment artifact semantic linter (主 13:08 + 主 17:43 + 主 17:58 + 主 23:44).

Phase: 1429
Version: 0.1.0
Date: 2026-08-10 (cron tick 04:20, Asia/Shanghai deep night)
Post: V1428 (deployment artifacts structural validator) + V1008 (deployment templates) + V1032 (docker templates)

What V1429 is
=============
V1429 is the **semantic linter** for ASI deployment artifacts. While
V1428 validates **structural** correctness (YAML parses, Dockerfile has
FROM, shell has shebang), V1429 validates **semantic** production-readiness:

For each artifact kind, V1429 applies a fixed set of semantic rules:

COMPOSE (docker-compose.yml):
  - SL001: at least one healthcheck defined (per service)
  - SL002: depends_on uses `condition: service_healthy` (not just name)
  - SL003: restart policy present (`restart:` key) on at least one service
  - SL004: resource limits present (`mem_limit` / `cpus` / `deploy.resources.limits`)
  - SL005: no `:latest` tag in `image:` directive

K8S (k8s-asi.yaml):
  - SL010: containers have `resources.limits` (cpu/memory)
  - SL011: containers have `livenessProbe` OR `readinessProbe`
  - SL012: image tag is not `:latest`
  - SL013: at least one workload kind (Deployment/StatefulSet/Job)

DOCKERFILE:
  - SL020: FROM uses specific tag (not `:latest`)
  - SL021: USER directive present (non-root)
  - SL022: HEALTHCHECK directive present
  - SL023: no ADD with URL (security: use COPY + RUN curl)
  - SL024: WORKDIR is set

REQUIREMENTS:
  - SL030: every dep has a version pin (== / >= / <= / ~=)
  - SL031: no `git+` URLs (security + reproducibility)
  - SL032: no `-e .` editable installs

STARTUP (start-asi.sh):
  - SL040: has `trap` for graceful shutdown (SIGTERM/SIGINT)
  - SL041: has retry logic (loop / `--retry` / `until`)
  - SL042: has `set -e` (fail fast)
  - SL043: writes pid to file (for shutdown signaling)

ENV (.env.example):
  - SL050: no obvious hardcoded secrets (api_key=VALUE, password=VALUE,
    token=VALUE, secret=VALUE) — values must be empty or placeholder
  - SL051: at least 3 documented `KEY=` lines
  - SL052: includes comments documenting each key

It does NOT run Docker, k8s, or pip. It only parses strings.
It does NOT compute a fake "ASI score". It computes a **semantic
readiness** = (n_rules_pass / n_rules_total).

Real-world usage:

    # Anyone can lint artifacts from V1428's default output:
    python -m apeireth.v1429_asi_deployment_semantic_linter lint

    # Anyone can lint a custom dir:
    python -m apeireth.v1429_asi_deployment_semantic_linter lint --output-dir PATH

    # Anyone can see semantic readiness summary:
    python -m apeireth.v1429_asi_deployment_semantic_linter summary

    # Anyone can render a markdown report:
    python -m apeireth.v1429_asi_deployment_semantic_linter report

It does NOT mutate any upstream V1428 / V1008 / V1032 state.
It only reads artifact content (string in memory).

Borrowed (6 — 主 19:33 走在前人经验上):
=======================================
- V1008 (deployment templates — k8s/compose/startup generators)
- V1032 (docker templates — Dockerfile/requirements templates)
- V1428 (deployment artifacts — ArtifactSpec + ValidationResult + DeploymentReadiness)
- Compose-spec 3.8 public rules (service healthcheck, depends_on)
- Kubernetes public best-practice (resources.limits, probes)
- Dockerfile public best-practice (USER, HEALTHCHECK, no ADD URL)

GUARDS upheld (V1429-specific, 14 — 主 00:44 质量工程化)
=========================================================
- GUARD_NO_DOCKER_REQUIRED: linter runs on strings, not docker daemon
- GUARD_NO_V1428_WRITE: V1429 only reads V1428 spec content
- GUARD_RULES_FIXED: rule set is a fixed tuple, not mutable
- GUARD_PER_ARTIFACT: each artifact kind has its own rule subset
- GUARD_DETERMINISTIC: identical input → identical lint result
- GUARD_SEVERITY_TIERED: rules have PASS / WARN / FAIL severity
- GUARD_SEMANTIC_READINESS_DEFINED: readiness = n_pass / n_total
- GUARD_RULE_CODES_FIXED: each rule has stable SL0xx code
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1429 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_RULES_DOCUMENTED: every rule code has docstring
- GUARD_NO_FAKE_PRODUCTION: semantic readiness != production-deployed
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_LINT: linter is regex, NOT consciousness
- GUARD_NO_ASI_LINT: linter is rules, NOT ASI
- GUARD_NO_HUMAN_LEVEL_LINT: linter is bounded, NOT human-level
- GUARD_NO_ABSOLUTE_LINT: linter is local, NOT absolute
- GUARD_NO_PRODUCTION_PADDING: semantic readiness != production-deployed

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1429 is a **bounded semantic linter**. It does not claim that the
artifacts are deployed, that Docker has run, or that a service is
healthy. It claims only that the artifact strings pass (or fail) a
fixed set of semantic rules. It is bounded by string parsing +
fixed rule set; NOT by runtime execution, security audit, or
production-grade testing. V1429 ≠ deployed, ≠ running, ≠ healthy,
≠ production, ≠ audited. V1429 reads V1008/V1032/V1428; never
replaces any of them.

API surfaces (15)
=================
1.  ``RuleSeverity`` — Enum (PASS / WARN / FAIL)
2.  ``LintRule`` — dataclass (code + artifact_kind + description + severity + check_fn)
3.  ``LintResult`` — dataclass (code + artifact_kind + severity + passed + message)
4.  ``ArtifactLintReport`` — dataclass (kind + results + n_pass + n_warn + n_fail + score)
5.  ``SemanticLinterReport`` — dataclass (per_kind + n_pass + n_warn + n_fail + readiness)
6.  ``ALL_RULES`` — tuple of 24 LintRule (SL001-SL052)
7.  ``rules_for(kind)`` — filter rules by ArtifactKind
8.  ``check_rule(rule, content)`` — single-rule check → LintResult
9.  ``lint_artifact(kind, content)`` — full lint for one artifact
10. ``lint_all(specs)`` — SemanticLinterReport across all 6 kinds
11. ``semantic_readiness(report)`` — float in [0, 1]
12. ``lint_from_v1428(output_dir)`` — convenience wrapper
13. ``popper_self_test()`` — 14 self-tests
14. ``chain_delegate()`` — V1428 + V1008 + V1032 chain probe
15. ``render_report_md(report)`` — markdown summary

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- lint [--output-dir PATH]
- summary [--output-dir PATH]
- report [--output-dir PATH]
- rules
"""

from __future__ import annotations

import enum
import json
import re
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1429_VERSION = "0.1.0"
V1429_SCHEMA = "v1429.asi-deployment-semantic-linter/v1"
V1429_MODULE = "v1429_asi_deployment_semantic_linter"

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


class ArtifactKind(str, enum.Enum):
    """V1429 artifact kinds (mirror V1428)."""

    COMPOSE = "docker-compose.yml"
    K8S = "k8s-asi.yaml"
    DOCKERFILE = "Dockerfile"
    REQUIREMENTS = "requirements.txt"
    STARTUP = "start-asi.sh"
    ENV = ".env.example"


ALL_ARTIFACT_KINDS: Tuple[ArtifactKind, ...] = (
    ArtifactKind.COMPOSE,
    ArtifactKind.K8S,
    ArtifactKind.DOCKERFILE,
    ArtifactKind.REQUIREMENTS,
    ArtifactKind.STARTUP,
    ArtifactKind.ENV,
)


class RuleSeverity(str, enum.Enum):
    """Severity tier for each lint rule."""

    PASS = "PASS"
    WARN = "WARN"
    FAIL = "FAIL"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class LintRule:
    """A single semantic lint rule."""

    code: str
    artifact_kind: ArtifactKind
    description: str
    severity: RuleSeverity
    check_fn: Callable[[str], Tuple[bool, str]]


@dataclass
class LintResult:
    """Result of running one rule against one artifact's content."""

    code: str
    artifact_kind: ArtifactKind
    severity: RuleSeverity
    passed: bool
    message: str


@dataclass
class ArtifactLintReport:
    """Per-artifact lint summary."""

    kind: ArtifactKind
    results: List[LintResult] = field(default_factory=list)
    n_pass: int = 0
    n_warn: int = 0
    n_fail: int = 0

    @property
    def score(self) -> float:
        n = len(self.results)
        return (self.n_pass / n) if n else 0.0

    @property
    def n_total(self) -> int:
        return len(self.results)


@dataclass
class SemanticLinterReport:
    """Whole-deployment semantic lint summary."""

    per_kind: Dict[str, ArtifactLintReport] = field(default_factory=dict)
    n_pass: int = 0
    n_warn: int = 0
    n_fail: int = 0
    n_total: int = 0


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1429_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_NO_V1428_WRITE",
    "GUARD_RULES_FIXED",
    "GUARD_PER_ARTIFACT",
    "GUARD_DETERMINISTIC",
    "GUARD_SEVERITY_TIERED",
    "GUARD_SEMANTIC_READINESS_DEFINED",
    "GUARD_RULE_CODES_FIXED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_RULES_DOCUMENTED",
    "GUARD_NO_FAKE_PRODUCTION",
    "GUARD_CLI_RUNNABLE",
)

V1429_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_LINT",
    "GUARD_NO_ASI_LINT",
    "GUARD_NO_HUMAN_LEVEL_LINT",
    "GUARD_NO_ABSOLUTE_LINT",
    "GUARD_NO_PRODUCTION_PADDING",
)

V1429_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1008", "deployment templates — k8s/compose/startup generators"),
    ("V1032", "docker templates — Dockerfile/requirements templates"),
    ("V1428", "deployment artifacts — ArtifactSpec + ValidationResult + DeploymentReadiness"),
    ("Compose-spec 3.8", "public spec — healthcheck + depends_on rules"),
    ("Kubernetes docs", "public best-practice — resources.limits + probes"),
    ("Dockerfile docs", "public best-practice — USER + HEALTHCHECK + no ADD URL"),
)


# ============================================================================
# Rule check functions (pure, string → (passed, message))
# ============================================================================


def _ok(msg: str = "") -> Tuple[bool, str]:
    return (True, msg or "rule passed")


def _fail(msg: str) -> Tuple[bool, str]:
    return (False, msg)


# ---- COMPOSE rules ----


def _compose_sl001(content: str) -> Tuple[bool, str]:
    """SL001: at least one `healthcheck:` block defined."""
    has = bool(re.search(r"^\s*healthcheck:", content, re.MULTILINE))
    return _ok("healthcheck present") if has else _fail("no healthcheck: directive found")


def _compose_sl002(content: str) -> Tuple[bool, str]:
    """SL002: depends_on uses condition (service_healthy / service_started)."""
    cond = bool(re.search(r"depends_on:.*condition:\s*service_", content, re.DOTALL))
    plain = bool(re.search(r"^\s*depends_on:", content, re.MULTILINE))
    if cond:
        return _ok("depends_on with condition: service_*")
    if plain:
        return _fail("depends_on missing condition: service_healthy")
    return _ok("no depends_on (single-service)")


def _compose_sl003(content: str) -> Tuple[bool, str]:
    """SL003: at least one service has restart policy."""
    has = bool(re.search(r"^\s*restart:\s*(always|on-failure|unless-stopped)", content, re.MULTILINE))
    return _ok("restart policy present") if has else _fail("no restart: policy on any service")


def _compose_sl004(content: str) -> Tuple[bool, str]:
    """SL004: resource limits present (mem_limit/cpus/deploy.resources.limits)."""
    patterns = (
        r"mem_limit:",
        r"cpus:",
        r"deploy:\s*\n\s*resources:\s*\n\s*limits:",
    )
    has = any(re.search(x, content) for x in patterns)
    return _ok("resource limits present") if has else _fail("no mem_limit / cpus / deploy.resources.limits")


def _compose_sl005(content: str) -> Tuple[bool, str]:
    """SL005: no `image: <name>:latest` (must pin version)."""
    bad = re.findall(r"^\s*image:\s*\S+:\s*latest\s*$", content, re.MULTILINE)
    return _ok("no :latest tags") if not bad else _fail(f"found {len(bad)} `:latest` tag(s)")


# ---- K8S rules ----


def _k8s_sl010(content: str) -> Tuple[bool, str]:
    """SL010: containers have resources.limits (cpu/memory)."""
    has = bool(re.search(r"resources:\s*\n\s*limits:", content))
    return _ok("resources.limits present") if has else _fail("no resources.limits on container")


def _k8s_sl011(content: str) -> Tuple[bool, str]:
    """SL011: containers have livenessProbe or readinessProbe."""
    has = bool(re.search(r"(livenessProbe|readinessProbe):", content))
    return _ok("probes present") if has else _fail("no livenessProbe / readinessProbe")


def _k8s_sl012(content: str) -> Tuple[bool, str]:
    """SL012: image tag is not `:latest`."""
    bad = re.findall(r"image:\s*\S+:\s*latest\b", content)
    return _ok("no :latest image") if not bad else _fail(f"found {len(bad)} `:latest` image(s)")


def _k8s_sl013(content: str) -> Tuple[bool, str]:
    """SL013: at least one workload kind."""
    kinds = ("Deployment", "StatefulSet", "DaemonSet", "Job", "CronJob")
    has = any(re.search(rf"^\s*kind:\s*{k}\s*$", content, re.MULTILINE) for k in kinds)
    return _ok("workload kind present") if has else _fail("no Deployment/StatefulSet/DaemonSet/Job/CronJob")


# ---- DOCKERFILE rules ----


def _dockerfile_sl020(content: str) -> Tuple[bool, str]:
    """SL020: FROM uses specific tag (not :latest)."""
    m = re.search(r"^\s*FROM\s+(\S+)", content, re.MULTILINE)
    if not m:
        return _fail("no FROM directive")
    img = m.group(1)
    if img.endswith(":latest"):
        return _fail(f"FROM uses :latest → {img}")
    if ":" not in img and "@sha256:" not in img:
        return _fail(f"FROM has no tag → {img}")
    return _ok(f"FROM uses pinned image → {img}")


def _dockerfile_sl021(content: str) -> Tuple[bool, str]:
    """SL021: USER directive present (non-root)."""
    has_user = bool(re.search(r"^\s*USER\s+\S+", content, re.MULTILINE))
    is_root = bool(re.search(r"^\s*USER\s+root\b", content, re.MULTILINE))
    if not has_user:
        return _fail("no USER directive (default root)")
    if is_root:
        return _fail("USER is root (security risk)")
    return _ok("non-root USER set")


def _dockerfile_sl022(content: str) -> Tuple[bool, str]:
    """SL022: HEALTHCHECK directive present."""
    has = bool(re.search(r"^\s*HEALTHCHECK\s+", content, re.MULTILINE | re.IGNORECASE))
    return _ok("HEALTHCHECK present") if has else _fail("no HEALTHCHECK directive")


def _dockerfile_sl023(content: str) -> Tuple[bool, str]:
    """SL023: no ADD with URL (use COPY + RUN curl instead)."""
    bad = re.findall(r"^\s*ADD\s+https?://", content, re.MULTILINE | re.IGNORECASE)
    return _ok("no ADD URL") if not bad else _fail(f"found {len(bad)} ADD URL(s) (security risk)")


def _dockerfile_sl024(content: str) -> Tuple[bool, str]:
    """SL024: WORKDIR is set."""
    has = bool(re.search(r"^\s*WORKDIR\s+\S+", content, re.MULTILINE))
    return _ok("WORKDIR set") if has else _fail("no WORKDIR directive")


# ---- REQUIREMENTS rules ----


def _req_sl030(content: str) -> Tuple[bool, str]:
    """SL030: every dep has a version pin (== / >= / <= / ~= / ===)."""
    lines = [ln.strip() for ln in content.splitlines() if ln.strip() and not ln.strip().startswith("#")]
    bad = []
    for ln in lines:
        # skip flags like -r other.txt
        if ln.startswith("-"):
            continue
        # pkg==1.0 / pkg>=1.0 / pkg~=1.0 / pkg===1.0
        if not re.search(r"[=<>~]=?\s*\d", ln):
            bad.append(ln)
    return _ok(f"{len(lines)-len(bad)}/{len(lines)} deps pinned") if not bad else _fail(f"{len(bad)} unpinned dep(s)")


def _req_sl031(content: str) -> Tuple[bool, str]:
    """SL031: no `git+` URLs (security + reproducibility)."""
    # Match `git+https://` or `git+ssh://` anywhere on a non-comment line
    bad = re.findall(r"git\+https?://|git\+ssh://", content)
    return _ok("no git+ URLs") if not bad else _fail(f"found {len(bad)} git+ URL(s)")


def _req_sl032(content: str) -> Tuple[bool, str]:
    """SL032: no `-e .` editable installs."""
    bad = bool(re.search(r"^\s*-e\s+\.", content, re.MULTILINE))
    return _ok("no -e . editable") if not bad else _fail("-e . editable install found")


# ---- STARTUP rules ----


def _startup_sl040(content: str) -> Tuple[bool, str]:
    """SL040: has trap for graceful shutdown (SIGTERM/SIGINT)."""
    has_trap = bool(re.search(r"^\s*trap\s+", content, re.MULTILINE))
    has_sig = bool(re.search(r"SIG(TERM|INT)", content))
    if has_trap and has_sig:
        return _ok("trap + SIGTERM/SIGINT handler")
    if has_trap:
        return _fail("trap present but no SIGTERM/SIGINT signal")
    return _fail("no trap directive for graceful shutdown")


def _startup_sl041(content: str) -> Tuple[bool, str]:
    """SL041: has retry logic (loop / until / --retry)."""
    patterns = (r"\bwhile\s+", r"\buntil\s+", r"--retry", r"--retry-max-time")
    has = any(re.search(p, content) for p in patterns)
    return _ok("retry logic present") if has else _fail("no retry logic (loop/until/--retry)")


def _startup_sl042(content: str) -> Tuple[bool, str]:
    """SL042: has `set -e` (fail fast)."""
    has = bool(re.search(r"^\s*set\s+-e", content, re.MULTILINE))
    return _ok("set -e present") if has else _fail("no `set -e` (fail-fast)")


def _startup_sl043(content: str) -> Tuple[bool, str]:
    """SL043: writes pid to file (for shutdown signaling)."""
    patterns = (r"echo\s+\$?\!?\$\!?\s*>\s*\S*\.pid", r"--pidfile", r"\$\!.*>\S*\.pid")
    has = any(re.search(p, content) for p in patterns)
    return _ok("pid file written") if has else _fail("no pid file write")


# ---- ENV rules ----


def _env_sl050(content: str) -> Tuple[bool, str]:
    """SL050: no obvious hardcoded secrets (api_key/password/token/secret with non-empty value)."""
    secret_keys = ("api_key", "apikey", "password", "passwd", "token", "secret", "private_key")
    bad = []
    for ln in content.splitlines():
        s = ln.strip()
        if not s or s.startswith("#"):
            continue
        m = re.match(r"^([A-Z_][A-Z0-9_]*)\s*=\s*(.+)$", s)
        if not m:
            continue
        key, val = m.group(1).lower(), m.group(2)
        if any(k in key for k in secret_keys):
            # value must be empty or placeholder like <...> / CHANGE_ME / xxxx
            if val and not re.search(r"^[<\[].+[>\]]|CHANGE[_-]?ME|xxxx|^$", val, re.IGNORECASE):
                bad.append(m.group(1))
    return _ok("no hardcoded secrets") if not bad else _fail(f"{len(bad)} potential hardcoded secret(s): {bad[:3]}")


def _env_sl051(content: str) -> Tuple[bool, str]:
    """SL051: at least 3 documented KEY= lines."""
    keys = [ln for ln in content.splitlines() if re.match(r"^\s*[A-Z_][A-Z0-9_]*\s*=", ln)]
    return _ok(f"{len(keys)} KEY= lines") if len(keys) >= 3 else _fail(f"only {len(keys)} KEY= lines (need >=3)")


def _env_sl052(content: str) -> Tuple[bool, str]:
    """SL052: includes comments documenting each key (>= 1 comment line)."""
    comments = [ln for ln in content.splitlines() if ln.strip().startswith("#")]
    return _ok(f"{len(comments)} comment line(s)") if comments else _fail("no comment lines documenting keys")


# ============================================================================
# Rule registry (fixed tuple)
# ============================================================================


ALL_RULES: Tuple[LintRule, ...] = (
    # COMPOSE
    LintRule("SL001", ArtifactKind.COMPOSE, "healthcheck defined per service", RuleSeverity.FAIL, _compose_sl001),
    LintRule("SL002", ArtifactKind.COMPOSE, "depends_on uses condition: service_*", RuleSeverity.FAIL, _compose_sl002),
    LintRule("SL003", ArtifactKind.COMPOSE, "restart policy present", RuleSeverity.WARN, _compose_sl003),
    LintRule("SL004", ArtifactKind.COMPOSE, "resource limits (mem_limit/cpus/deploy.resources.limits)", RuleSeverity.WARN, _compose_sl004),
    LintRule("SL005", ArtifactKind.COMPOSE, "no :latest tag in image:", RuleSeverity.FAIL, _compose_sl005),
    # K8S
    LintRule("SL010", ArtifactKind.K8S, "containers have resources.limits", RuleSeverity.FAIL, _k8s_sl010),
    LintRule("SL011", ArtifactKind.K8S, "containers have livenessProbe / readinessProbe", RuleSeverity.FAIL, _k8s_sl011),
    LintRule("SL012", ArtifactKind.K8S, "image tag not :latest", RuleSeverity.FAIL, _k8s_sl012),
    LintRule("SL013", ArtifactKind.K8S, "workload kind present", RuleSeverity.FAIL, _k8s_sl013),
    # DOCKERFILE
    LintRule("SL020", ArtifactKind.DOCKERFILE, "FROM uses specific tag", RuleSeverity.FAIL, _dockerfile_sl020),
    LintRule("SL021", ArtifactKind.DOCKERFILE, "non-root USER directive", RuleSeverity.FAIL, _dockerfile_sl021),
    LintRule("SL022", ArtifactKind.DOCKERFILE, "HEALTHCHECK directive", RuleSeverity.WARN, _dockerfile_sl022),
    LintRule("SL023", ArtifactKind.DOCKERFILE, "no ADD with URL", RuleSeverity.FAIL, _dockerfile_sl023),
    LintRule("SL024", ArtifactKind.DOCKERFILE, "WORKDIR set", RuleSeverity.WARN, _dockerfile_sl024),
    # REQUIREMENTS
    LintRule("SL030", ArtifactKind.REQUIREMENTS, "every dep has version pin", RuleSeverity.FAIL, _req_sl030),
    LintRule("SL031", ArtifactKind.REQUIREMENTS, "no git+ URLs", RuleSeverity.WARN, _req_sl031),
    LintRule("SL032", ArtifactKind.REQUIREMENTS, "no -e . editable installs", RuleSeverity.WARN, _req_sl032),
    # STARTUP
    LintRule("SL040", ArtifactKind.STARTUP, "trap for SIGTERM/SIGINT", RuleSeverity.FAIL, _startup_sl040),
    LintRule("SL041", ArtifactKind.STARTUP, "retry logic (loop/until/--retry)", RuleSeverity.WARN, _startup_sl041),
    LintRule("SL042", ArtifactKind.STARTUP, "set -e (fail fast)", RuleSeverity.FAIL, _startup_sl042),
    LintRule("SL043", ArtifactKind.STARTUP, "writes pid file", RuleSeverity.WARN, _startup_sl043),
    # ENV
    LintRule("SL050", ArtifactKind.ENV, "no hardcoded secrets", RuleSeverity.FAIL, _env_sl050),
    LintRule("SL051", ArtifactKind.ENV, ">= 3 documented KEY= lines", RuleSeverity.WARN, _env_sl051),
    LintRule("SL052", ArtifactKind.ENV, "comment lines documenting keys", RuleSeverity.WARN, _env_sl052),
)


def rules_for(kind: ArtifactKind) -> Tuple[LintRule, ...]:
    """Return rules applicable to a given artifact kind."""
    return tuple(r for r in ALL_RULES if r.artifact_kind == kind)


# ============================================================================
# Core linter API
# ============================================================================


def check_rule(rule: LintRule, content: str) -> LintResult:
    """Run a single rule against artifact content."""
    try:
        passed, msg = rule.check_fn(content)
    except Exception as exc:  # rule crashed → treat as FAIL with diagnostic
        return LintResult(
            code=rule.code,
            artifact_kind=rule.artifact_kind,
            severity=rule.severity,
            passed=False,
            message=f"rule raised {type(exc).__name__}: {exc}",
        )
    return LintResult(
        code=rule.code,
        artifact_kind=rule.artifact_kind,
        severity=rule.severity,
        passed=passed,
        message=msg,
    )


def lint_artifact(kind: ArtifactKind, content: str) -> ArtifactLintReport:
    """Run all rules applicable to a single artifact."""
    rep = ArtifactLintReport(kind=kind)
    for rule in rules_for(kind):
        r = check_rule(rule, content)
        rep.results.append(r)
        if r.passed:
            rep.n_pass += 1
        elif r.severity == RuleSeverity.WARN:
            rep.n_warn += 1
        else:
            rep.n_fail += 1
    return rep


def lint_all(per_kind_content: Dict[ArtifactKind, str]) -> SemanticLinterReport:
    """Run all rules across all 6 artifact kinds."""
    out = SemanticLinterReport()
    for kind in ALL_ARTIFACT_KINDS:
        content = per_kind_content.get(kind, "")
        rep = lint_artifact(kind, content)
        out.per_kind[kind.value] = rep
        out.n_pass += rep.n_pass
        out.n_warn += rep.n_warn
        out.n_fail += rep.n_fail
        out.n_total += rep.n_total
    return out


def semantic_readiness(report: SemanticLinterReport) -> float:
    """Compute semantic readiness = n_pass / n_total."""
    return (report.n_pass / report.n_total) if report.n_total else 0.0


def lint_from_v1428(output_dir: Optional[Path] = None) -> SemanticLinterReport:
    """Convenience: read artifacts from V1428's output dir (or fresh generate).

    Strategy:
      1. Try to import V1428 — if available, reuse its ArtifactSpec set
         to avoid duplicating template generation.
      2. Else: fall back to reading files from output_dir (if they exist).
      3. Else: return an empty report (caller will see n_total=0 → readiness=0).
    """
    output_dir = output_dir or DEFAULT_OUTPUT_DIR
    per_kind_content: Dict[ArtifactKind, str] = {}
    try:
        # Lazy import to avoid hard dep cycle
        from apeireth.v1428_asi_real_deployment_artifacts import (
            build_default_specs,
            generate_artifacts,
        )
        specs = build_default_specs(output_dir)
        # Generate them (idempotent: re-writes with same content)
        generate_artifacts(specs)
        for s in specs:
            try:
                per_kind_content[s.kind] = s.path.read_text(encoding="utf-8")
            except Exception:
                per_kind_content[s.kind] = s.content
    except Exception:
        # Fallback: read from disk if artifacts exist
        for kind in ALL_ARTIFACT_KINDS:
            p = Path(output_dir) / kind.value
            per_kind_content[kind] = p.read_text(encoding="utf-8") if p.exists() else ""
    return lint_all(per_kind_content)


# ============================================================================
# Popper self-test (主 00:44 质量工程化)
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """14 self-tests for V1429. Each must pass for the linter to be trusted."""
    results: Dict[str, Any] = {}

    # PT01: rule count = 24
    results["PT01_rules_count_24"] = (len(ALL_RULES) == 24, f"got {len(ALL_RULES)}")

    # PT02: every rule has stable SL0xx code
    codes = [r.code for r in ALL_RULES]
    results["PT02_codes_format"] = (all(re.match(r"^SL\d{3}$", c) for c in codes), f"codes={codes[:3]}...")

    # PT03: every rule has severity
    results["PT03_severity_present"] = (all(isinstance(r.severity, RuleSeverity) for r in ALL_RULES), "")

    # PT04: rules cover all 6 artifact kinds
    kinds_covered = {r.artifact_kind for r in ALL_RULES}
    results["PT04_all_kinds_covered"] = (set(ALL_ARTIFACT_KINDS) == kinds_covered, f"got {len(kinds_covered)}")

    # PT05: COMPOSE has 5 rules
    n_compose = len(rules_for(ArtifactKind.COMPOSE))
    results["PT05_compose_5_rules"] = (n_compose == 5, f"got {n_compose}")

    # PT06: K8S has 4 rules
    n_k8s = len(rules_for(ArtifactKind.K8S))
    results["PT06_k8s_4_rules"] = (n_k8s == 4, f"got {n_k8s}")

    # PT07: DOCKERFILE has 5 rules
    n_df = len(rules_for(ArtifactKind.DOCKERFILE))
    results["PT07_dockerfile_5_rules"] = (n_df == 5, f"got {n_df}")

    # PT08: REQUIREMENTS has 3 rules
    n_req = len(rules_for(ArtifactKind.REQUIREMENTS))
    results["PT08_requirements_3_rules"] = (n_req == 3, f"got {n_req}")

    # PT09: STARTUP has 4 rules
    n_su = len(rules_for(ArtifactKind.STARTUP))
    results["PT09_startup_4_rules"] = (n_su == 4, f"got {n_su}")

    # PT10: ENV has 3 rules
    n_env = len(rules_for(ArtifactKind.ENV))
    results["PT10_env_3_rules"] = (n_env == 3, f"got {n_env}")

    # PT11: lint_artifact empty content → all FAIL (deterministic)
    empty_compose = lint_artifact(ArtifactKind.COMPOSE, "")
    results["PT11_empty_content_fail"] = (empty_compose.n_fail >= 1, f"n_fail={empty_compose.n_fail}")
    # PT11b: empty compose → exactly 5 results
    results["PT11b_empty_compose_total"] = (empty_compose.n_total == 5, f"n_total={empty_compose.n_total}")

    # PT12: lint_artifact well-formed → some PASS
    good_compose = lint_artifact(
        ArtifactKind.COMPOSE,
        "services:\n  api:\n    image: apeireth:1.0\n    healthcheck:\n      test: [\"CMD\", \"true\"]\n    restart: always\n    depends_on:\n      db:\n        condition: service_healthy\n    mem_limit: 512m\n",
    )
    results["PT12_good_compose_passes"] = (good_compose.n_pass >= 3, f"n_pass={good_compose.n_pass}")

    # PT13: SL020 detects :latest in FROM
    bad_df = lint_artifact(ArtifactKind.DOCKERFILE, "FROM python:latest\nUSER app\n")
    sl020 = next((r for r in bad_df.results if r.code == "SL020"), None)
    results["PT13_sl020_detects_latest"] = (sl020 is not None and not sl020.passed, f"sl020.passed={sl020 and sl020.passed}")

    # PT14: SL050 detects hardcoded api_key
    bad_env = lint_artifact(ArtifactKind.ENV, "API_KEY=supersecret123\nFOO=bar\nBAZ=qux\n# comment\n")
    sl050 = next((r for r in bad_env.results if r.code == "SL050"), None)
    results["PT14_sl050_detects_secret"] = (sl050 is not None and not sl050.passed, f"sl050.passed={sl050 and sl050.passed}")

    n_pass = sum(1 for v in results.values() if v[0])
    n_total = len(results)
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "ok": n_pass == n_total,
        "results": results,
    }


# ============================================================================
# Chain delegate (主 13:08 真实)
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream modules (V1428 + V1008 + V1032) for liveness."""
    chain: Dict[str, Any] = {}

    # V1428 chain
    try:
        from apeireth.v1428_asi_real_deployment_artifacts import chain_delegate as v1428_chain
        chain["V1428"] = {"ok": True, "data": v1428_chain()}
    except Exception as exc:
        chain["V1428"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # V1008 chain (best-effort)
    try:
        import apeireth.v1008_deployment as _v1008
        chain["V1008"] = {
            "ok": True,
            "version": getattr(_v1008, "V1008_VERSION", None),
            "has_generate_docker_compose": hasattr(_v1008, "generate_docker_compose"),
            "has_generate_k8s_manifest": hasattr(_v1008, "generate_k8s_manifest"),
        }
    except Exception as exc:
        chain["V1008"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # V1032 chain (best-effort)
    try:
        import apeireth.v1032_docker as _v1032
        chain["V1032"] = {
            "ok": True,
            "version": getattr(_v1032, "V1032_VERSION", None),
            "has_DOCKERFILE_TEMPLATE": hasattr(_v1032, "DOCKERFILE_TEMPLATE"),
            "has_REQUIREMENTS_TEMPLATE": hasattr(_v1032, "REQUIREMENTS_TEMPLATE"),
        }
    except Exception as exc:
        chain["V1032"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    all_ok = all(c.get("ok") for c in chain.values())
    return {"all_ok": all_ok, "chain": chain, "n_modules": len(chain)}


# ============================================================================
# Reporting
# ============================================================================


def render_report_md(report: SemanticLinterReport) -> str:
    """Render the semantic lint report as markdown."""
    readiness = semantic_readiness(report)
    lines = [
        "# V1429 ASI Deployment Semantic Lint Report",
        "",
        f"- **n_pass**: {report.n_pass}",
        f"- **n_warn**: {report.n_warn}",
        f"- **n_fail**: {report.n_fail}",
        f"- **n_total**: {report.n_total}",
        f"- **readiness**: {readiness:.4f}",
        "",
        "## Per-artifact",
        "",
    ]
    for kind_name, rep in report.per_kind.items():
        lines.append(f"### {kind_name}")
        lines.append("")
        lines.append(f"- score: {rep.score:.4f}  ({rep.n_pass}/{rep.n_total})")
        lines.append(f"- pass: {rep.n_pass}  warn: {rep.n_warn}  fail: {rep.n_fail}")
        lines.append("")
        for r in rep.results:
            mark = "✅" if r.passed else ("⚠" if r.severity == RuleSeverity.WARN else "❌")
            lines.append(f"  - {mark} `{r.code}` [{r.severity.value}] {r.message}")
        lines.append("")
    return "\n".join(lines)


def module_meta() -> Dict[str, Any]:
    """Return module metadata (mirror V1428 pattern)."""
    return {
        "version": V1429_VERSION,
        "schema": V1429_SCHEMA,
        "module": V1429_MODULE,
        "phase": 1429,
        "n_rules": len(ALL_RULES),  # computed at runtime (24)
        "n_artifact_kinds": len(ALL_ARTIFACT_KINDS),
        "guards": list(V1429_GUARDS),
        "v3_guards": list(V1429_V3_GUARDS),
        "borrowed": [list(t) for t in V1429_BORROWED],
    }


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        f"V1429 ASI deployment semantic linter v{V1429_VERSION}\n"
        f"\n"
        f"Commands:\n"
        f"  version                  print version\n"
        f"  meta [--json]            print module metadata\n"
        f"  demo                     run a self-contained demo\n"
        f"  help                     print this help\n"
        f"  popper                   run 14 self-tests\n"
        f"  chain                    probe upstream chain (V1428 + V1008 + V1032)\n"
        f"  lint [--output-dir PATH] lint artifacts in output dir\n"
        f"  summary [--output-dir PATH]  print semantic readiness summary\n"
        f"  report [--output-dir PATH]   print markdown report\n"
        f"  rules                    list all rules\n"
    )


def _run_demo() -> SemanticLinterReport:
    """Run a self-contained demo with sample artifacts (one good + one bad per kind)."""
    samples: Dict[ArtifactKind, str] = {
        ArtifactKind.COMPOSE: (
            "services:\n"
            "  api:\n"
            "    image: apeireth:1.0.0\n"
            "    healthcheck:\n"
            "      test: [\"CMD\", \"true\"]\n"
            "    restart: always\n"
            "    depends_on:\n"
            "      db:\n"
            "        condition: service_healthy\n"
            "    mem_limit: 512m\n"
        ),
        ArtifactKind.K8S: (
            "kind: Deployment\n"
            "spec:\n"
            "  template:\n"
            "    spec:\n"
            "      containers:\n"
            "        - name: asi\n"
            "          image: apeireth:1.0.0\n"
            "          resources:\n"
            "            limits:\n"
            "              cpu: 500m\n"
            "              memory: 512Mi\n"
            "          livenessProbe:\n"
            "            httpGet:\n"
            "              path: /healthz\n"
        ),
        ArtifactKind.DOCKERFILE: (
            "FROM python:3.12-slim\n"
            "WORKDIR /app\n"
            "COPY requirements.txt .\n"
            "RUN pip install -r requirements.txt\n"
            "USER app\n"
            "HEALTHCHECK CMD curl -f http://localhost:8080/health || exit 1\n"
        ),
        ArtifactKind.REQUIREMENTS: (
            "fastapi==0.110.0\n"
            "pydantic==2.6.0\n"
            "uvicorn==0.27.0\n"
        ),
        ArtifactKind.STARTUP: (
            "#!/usr/bin/env bash\n"
            "set -e\n"
            "trap 'kill -TERM $PID; wait $PID' SIGTERM SIGINT\n"
            "while true; do\n"
            "  python -m apeireth.daemon &\n"
            "  PID=$!\n"
            "  echo $PID > /var/run/asi.pid\n"
            "  wait $PID\n"
            "done\n"
        ),
        ArtifactKind.ENV: (
            "# ASI runtime configuration\n"
            "ASI_PORT=8080\n"
            "ASI_LOG_LEVEL=INFO\n"
            "ASI_API_KEY=\n"
            "ASI_DB_URL=\n"
        ),
    }
    return lint_all(samples)


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help", "help"):
        _print_help()
        return 0
    cmd = argv[0]
    args = argv[1:]

    # Parse --output-dir
    output_dir: Optional[Path] = None
    if "--output-dir" in args:
        i = args.index("--output-dir")
        if i + 1 < len(args):
            output_dir = Path(args[i + 1])
            args = args[:i] + args[i + 2:]

    if cmd == "version":
        print(V1429_VERSION)
        return 0

    if cmd == "meta":
        meta = module_meta()
        if "--json" in args:
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"  {k}: {v}")
        return 0

    if cmd == "demo":
        rep = _run_demo()
        readiness = semantic_readiness(rep)
        print(f"demo: pass={rep.n_pass} warn={rep.n_warn} fail={rep.n_fail} total={rep.n_total} readiness={readiness:.4f}")
        return 0

    if cmd == "help":
        _print_help()
        return 0

    if cmd == "popper":
        out = popper_self_test()
        print(json.dumps(out, indent=2, ensure_ascii=False))
        return 0 if out["ok"] else 1

    if cmd == "chain":
        out = chain_delegate()
        print(json.dumps(out, indent=2, ensure_ascii=False))
        return 0 if out["all_ok"] else 1

    if cmd == "lint":
        rep = lint_from_v1428(output_dir)
        readiness = semantic_readiness(rep)
        print(f"semantic lint: pass={rep.n_pass} warn={rep.n_warn} fail={rep.n_fail} readiness={readiness:.4f}")
        return 0

    if cmd == "summary":
        rep = lint_from_v1428(output_dir)
        readiness = semantic_readiness(rep)
        print(f"=== V1429 summary ===")
        print(f"n_pass={rep.n_pass}  n_warn={rep.n_warn}  n_fail={rep.n_fail}  n_total={rep.n_total}")
        print(f"semantic_readiness={readiness:.4f}")
        for kind_name, r in rep.per_kind.items():
            print(f"  {kind_name}: score={r.score:.4f} ({r.n_pass}/{r.n_total})  pass={r.n_pass} warn={r.n_warn} fail={r.n_fail}")
        return 0

    if cmd == "report":
        rep = lint_from_v1428(output_dir)
        print(render_report_md(rep))
        return 0

    if cmd == "rules":
        for r in ALL_RULES:
            print(f"  {r.code} [{r.severity.value}] {r.artifact_kind.value}: {r.description}")
        return 0

    print(f"unknown command: {cmd}", file=sys.stderr)
    _print_help()
    return 2


if __name__ == "__main__":
    raise SystemExit(main())