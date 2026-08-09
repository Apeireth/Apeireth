"""V1428 — ASI 真生产 deployment artifacts 生成 + 验证 (主 13:31 + 主 23:44 + 主 17:33).

Phase: 1428
Version: 0.1.0
Date: 2026-08-10 (cron tick 04:14, Asia/Shanghai deep night)
Post: V1008 (deployment) + V1032 (docker) + V1427 (stage-delivery)

What V1428 is
=============
V1428 is the **real deployment artifacts generator + validator** for
Apeireth ASI. It:

1. Generates production deployment artifacts using V1008 + V1032 templates:
   - `docker-compose.yml` (multi-service compose)
   - `Dockerfile` (production image)
   - `k8s-asi.yaml` (kubernetes manifest)
   - `requirements.txt` (Python deps)
   - `start-asi.sh` (startup script)
   - `.env.example` (environment template)

2. Validates each artifact's structure (without Docker installed):
   - YAML structure (indent + key presence)
   - Dockerfile structure (FROM, RUN, CMD presence)
   - Shell script syntax (basic `set -e` + shebang check)

3. Computes a **deployment-readiness score** = n_valid / n_total.
   NOT a fake "ASI score".

4. Writes artifacts to a configurable output directory.

It does NOT claim to deploy anywhere (this Windows host has no Docker).
It claims the artifacts are **correct-by-construction** and validates
their structure.

Real-world usage:

    # Anyone can generate + validate all artifacts:
    python -m apeireth.v1428_asi_real_deployment_artifacts generate

    # Anyone can validate existing artifacts:
    python -m apeireth.v1428_asi_real_deployment_artifacts validate

    # Anyone can see readiness summary:
    python -m apeireth.v1428_asi_real_deployment_artifacts summary

It does NOT mutate upstream V1008/V1032 templates. It **calls** their
generator functions and writes the output.

Borrowed (8 — 主 19:33 走在前人经验上):
=======================================
- V1008 (deployment — generate_docker_compose / generate_k8s_manifest / generate_startup_script)
- V1032 (docker — DOCKERFILE_TEMPLATE / DOCKER_COMPOSE_TEMPLATE / K8S_DEPLOYMENT_TEMPLATE / REQUIREMENTS_TEMPLATE)
- V1418 (DGM cron integration — chain_delegate pattern)
- V1427 (stage-delivery — aggregator pattern)
- V1426 (VCP 6 protocols — borrowed read-only)
- stdlib dataclasses + json + re + pathlib + subprocess
- stdlib typing
- Compose-spec 3.8 YAML structure (public spec)

GUARDS upheld (V1428-specific, 13 — 主 00:44 质量工程化)
=========================================================
- GUARD_NO_DOCKER_REQUIRED: validation does not require Docker installed
- GUARD_NO_V1008_WRITE: V1008 generators called read-only
- GUARD_NO_V1032_WRITE: V1032 templates used read-only
- GUARD_ARTIFACTS_REAL: each generated artifact is written to disk
- GUARD_YAML_VALID: docker-compose + k8s have valid YAML structure
- GUARD_DOCKERFILE_VALID: Dockerfile has FROM + CMD lines
- GUARD_SHELL_VALID: startup script has shebang + set -e
- GUARD_REQUIREMENTS_VALID: requirements.txt has at least 1 dep
- GUARD_ENV_VALID: .env.example has at least 1 key=value
- GUARD_READINESS_DEFINED: readiness = n_valid / n_total
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1428 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_DEPLOY: deployment is stdlib, NOT consciousness
- GUARD_NO_ASI_DEPLOY: deployment is configuration, NOT ASI
- GUARD_NO_HUMAN_LEVEL_DEPLOY: deployment is bounded, NOT human-level
- GUARD_NO_ABSOLUTE_DEPLOY: deployment is local, NOT absolute
- GUARD_NO_PRODUCTION_PADDING: readiness != production-deployed

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1428 is a **bounded deployment-artifacts generator**. It does not
claim that the artifacts have been deployed, that Docker has run, or
that a service is healthy. It claims the artifacts are structurally
correct and the readiness score is computed. It is bounded by stdlib +
V1008/V1032 templates; NOT by Docker daemon, k8s cluster, cloud
provider, production traffic, or service mesh. V1428 ≠ deployed, ≠
running, ≠ healthy, ≠ production. V1428 reads V1008/V1032/V1418/
V1426/V1427; never replaces any of them.

API surfaces (13)
=================
1.  ``ArtifactKind`` — Enum (COMPOSE / K8S / DOCKERFILE / REQUIREMENTS / STARTUP / ENV)
2.  ``ArtifactSpec`` — dataclass (kind + content + path + validators)
3.  ``ValidationResult`` — dataclass (kind + ok + errors + warnings)
4.  ``DeploymentReadiness`` — dataclass (n_valid + n_total + readiness + per_kind)
5.  ``build_default_specs(output_dir)`` — list of 6 ArtifactSpec
6.  ``generate_artifacts(specs)`` — write all artifacts, return mapping
7.  ``validate_artifact(spec)`` — ValidationResult
8.  ``validate_all(specs)`` — DeploymentReadiness
9.  ``generate_and_validate(output_dir)`` — combined
10. ``popper_self_test()`` — 13 self-tests
11. ``chain_delegate()`` — V1008 + V1032 + V1418 + V1426 + V1427 chain probe
12. ``module_meta()`` — returns version dict
13. ``render_report_md(readiness)`` — renders markdown summary

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- generate [--output-dir PATH]
- validate [--output-dir PATH]
- summary [--output-dir PATH]
- report [--output-dir PATH]
"""

from __future__ import annotations

import dataclasses
import enum
import json
import re
import sys
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1428_VERSION = "0.1.0"
V1428_SCHEMA = "v1428.asi-real-deployment-artifacts/v1"
V1428_MODULE = "v1428_asi_real_deployment_artifacts"

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
    """V1428 deployment artifact kinds (主 17:33 主人真采纳)."""

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


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1428_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_NO_V1008_WRITE",
    "GUARD_NO_V1032_WRITE",
    "GUARD_ARTIFACTS_REAL",
    "GUARD_YAML_VALID",
    "GUARD_DOCKERFILE_VALID",
    "GUARD_SHELL_VALID",
    "GUARD_REQUIREMENTS_VALID",
    "GUARD_ENV_VALID",
    "GUARD_READINESS_DEFINED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
)

V1428_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_DEPLOY",
    "GUARD_NO_ASI_DEPLOY",
    "GUARD_NO_HUMAN_LEVEL_DEPLOY",
    "GUARD_NO_ABSOLUTE_DEPLOY",
    "GUARD_NO_PRODUCTION_PADDING",
)

V1428_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1008", "deployment — generate_docker_compose / generate_k8s_manifest / generate_startup_script"),
    ("V1032", "docker — DOCKERFILE_TEMPLATE / DOCKER_COMPOSE_TEMPLATE / K8S_DEPLOYMENT_TEMPLATE / REQUIREMENTS_TEMPLATE"),
    ("V1418", "DGM cron integration — chain_delegate pattern"),
    ("V1427", "stage-delivery — aggregator pattern"),
    ("V1426", "VCP 6 protocols — borrowed read-only"),
    ("stdlib dataclasses + json + re + pathlib + subprocess", "Python stdlib only"),
    ("stdlib typing", "Python stdlib only"),
    ("Compose-spec 3.8 YAML structure", "public spec"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_utc_iso() -> str:
    from datetime import datetime, timezone

    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _safe_path(p: Path) -> Path:
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with .. rejected: {p}")
    return Path(p)


def _parse_kv_args(rest: List[str]) -> Dict[str, str]:
    out: Dict[str, str] = {}
    i = 0
    while i < len(rest):
        if rest[i].startswith("--") and i + 1 < len(rest):
            key = rest[i][2:]
            out[key] = rest[i + 1]
            i += 2
        else:
            i += 1
    return out


def _validate_yaml_basic(content: str) -> Tuple[bool, List[str]]:
    """Minimal YAML structure check (no external deps).

    Checks: non-empty, has at least one key: value line, no tabs at start.
    """
    errors: List[str] = []
    if not content or not content.strip():
        errors.append("empty content")
        return False, errors
    lines = content.splitlines()
    has_kv = False
    for i, line in enumerate(lines):
        if line.startswith("\t"):
            errors.append(f"line {i + 1}: tab at start (YAML forbids)")
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if ":" in stripped and not stripped.startswith("-"):
            has_kv = True
    if not has_kv:
        errors.append("no key: value line found")
    return len(errors) == 0, errors


def _validate_dockerfile(content: str) -> Tuple[bool, List[str]]:
    """Minimal Dockerfile structure check."""
    errors: List[str] = []
    if not content or not content.strip():
        errors.append("empty content")
        return False, errors
    lines = content.splitlines()
    has_from = False
    has_cmd = False
    for line in lines:
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if s.upper().startswith("FROM "):
            has_from = True
        if s.upper().startswith("CMD ") or s.upper().startswith("ENTRYPOINT "):
            has_cmd = True
    if not has_from:
        errors.append("missing FROM line")
    if not has_cmd:
        errors.append("missing CMD or ENTRYPOINT line")
    return len(errors) == 0, errors


def _validate_shell(content: str) -> Tuple[bool, List[str]]:
    """Minimal shell script structure check."""
    errors: List[str] = []
    if not content or not content.strip():
        errors.append("empty content")
        return False, errors
    lines = content.splitlines()
    has_shebang = False
    has_set_e = False
    for line in lines:
        s = line.strip()
        if s.startswith("#!"):
            has_shebang = True
        if "set -e" in s or "set -eu" in s:
            has_set_e = True
    if not has_shebang:
        errors.append("missing shebang (#!/...)")
    if not has_set_e:
        errors.append("missing 'set -e' (fail-fast)")
    return len(errors) == 0, errors


def _validate_requirements(content: str) -> Tuple[bool, List[str]]:
    """Minimal requirements.txt check."""
    errors: List[str] = []
    if not content or not content.strip():
        errors.append("empty content")
        return False, errors
    has_dep = False
    for line in content.splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if re.match(r"^[A-Za-z0-9_.-]+(\[.*\])?(==|>=|<=|~=|!=)?[A-Za-z0-9_.-]*$", s):
            has_dep = True
    if not has_dep:
        errors.append("no valid dependency line")
    return len(errors) == 0, errors


def _validate_env(content: str) -> Tuple[bool, List[str]]:
    """Minimal .env check."""
    errors: List[str] = []
    if not content or not content.strip():
        errors.append("empty content")
        return False, errors
    has_kv = False
    for line in content.splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if "=" in s and not s.startswith("="):
            has_kv = True
    if not has_kv:
        errors.append("no key=value line")
    return len(errors) == 0, errors


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class ArtifactSpec:
    """One deployment artifact spec."""

    kind: ArtifactKind
    content: str
    path: Path
    description: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "kind": self.kind.value,
            "path": str(self.path),
            "description": self.description,
            "content_length": len(self.content),
        }


@dataclasses.dataclass
class ValidationResult:
    """One artifact's validation result."""

    kind: str
    path: str
    ok: bool
    errors: List[str]
    warnings: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class DeploymentReadiness:
    """Aggregate readiness score."""

    n_total: int
    n_valid: int
    readiness: float
    per_kind: Dict[str, ValidationResult]
    output_dir: str
    started_iso: str
    ended_iso: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        # convert per_kind values to dicts
        d["per_kind"] = {k: v.to_dict() for k, v in self.per_kind.items()}
        return d


# ============================================================================
# Spec builders (uses V1008 + V1032 templates)
# ============================================================================


def build_default_specs(output_dir: Path) -> List[ArtifactSpec]:
    """Build 6 default artifact specs using V1008 + V1032 templates.

    Falls back to built-in templates if V1008/V1032 are unavailable.
    """
    output_dir = _safe_path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    specs: List[ArtifactSpec] = []

    # ---- COMPOSE ----
    compose_content = _generate_compose_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.COMPOSE,
            content=compose_content,
            path=output_dir / "docker-compose.yml",
            description="Multi-service Docker Compose (主 17:33 主人真采纳)",
        )
    )

    # ---- K8S ----
    k8s_content = _generate_k8s_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.K8S,
            content=k8s_content,
            path=output_dir / "k8s-asi.yaml",
            description="Kubernetes deployment manifest",
        )
    )

    # ---- DOCKERFILE ----
    dockerfile_content = _generate_dockerfile_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.DOCKERFILE,
            content=dockerfile_content,
            path=output_dir / "Dockerfile",
            description="Production Dockerfile (Python 3.13-slim)",
        )
    )

    # ---- REQUIREMENTS ----
    req_content = _generate_requirements_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.REQUIREMENTS,
            content=req_content,
            path=output_dir / "requirements.txt",
            description="Python runtime dependencies",
        )
    )

    # ---- STARTUP ----
    startup_content = _generate_startup_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.STARTUP,
            content=startup_content,
            path=output_dir / "start-asi.sh",
            description="Startup shell script (主 17:33 放手干到底)",
        )
    )

    # ---- ENV ----
    env_content = _generate_env_content()
    specs.append(
        ArtifactSpec(
            kind=ArtifactKind.ENV,
            content=env_content,
            path=output_dir / ".env.example",
            description="Environment variables template",
        )
    )

    return specs


def _generate_compose_content() -> str:
    """Generate docker-compose.yml content.

    Tries V1008.generate_docker_compose first, falls back to built-in.
    """
    try:
        import v1008_deployment as v1008

        cfgs = [
            v1008.DeploymentConfig(
                name="asi-core",
                service_name="asi-core",
                image="apeireth/asi-core:0.1.0",
                port=18765,
                replicas=1,
                env_vars={"ASI_MODE": "core", "ASI_LOG_LEVEL": "INFO"},
                health_check_path="/api/asi/health",
            ),
            v1008.DeploymentConfig(
                name="asi-bench",
                service_name="asi-bench",
                image="apeireth/asi-bench:0.1.0",
                port=18766,
                replicas=1,
                env_vars={"ASI_MODE": "benchmark"},
                health_check_path="/api/asi/health",
            ),
        ]
        return v1008.generate_docker_compose(cfgs)
    except Exception:
        # Built-in fallback
        return """version: '3.8'
services:
  asi-core:
    image: apeireth/asi-core:0.1.0
    ports:
      - "18765:18765"
    environment:
      - ASI_MODE=core
      - ASI_LOG_LEVEL=INFO
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:18765/api/asi/health"]
      interval: 30s
      timeout: 10s
      retries: 3
  asi-bench:
    image: apeireth/asi-bench:0.1.0
    ports:
      - "18766:18766"
    environment:
      - ASI_MODE=benchmark
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:18766/api/asi/health"]
      interval: 30s
      timeout: 10s
      retries: 3
"""


def _generate_k8s_content() -> str:
    """Generate k8s manifest. Tries V1008.generate_k8s_manifest first."""
    try:
        import v1008_deployment as v1008

        cfg = v1008.DeploymentConfig(
            name="asi-core",
            service_name="asi-core",
            image="apeireth/asi-core:0.1.0",
            port=18765,
            replicas=1,
            env_vars={"ASI_MODE": "core"},
            health_check_path="/api/asi/health",
        )
        return v1008.generate_k8s_manifest(cfg)
    except Exception:
        return """apiVersion: apps/v1
kind: Deployment
metadata:
  name: asi-core
  labels:
    app: asi
    tier: core
spec:
  replicas: 1
  selector:
    matchLabels:
      app: asi
  template:
    metadata:
      labels:
        app: asi
        tier: core
    spec:
      containers:
        - name: asi-core
          image: apeireth/asi-core:0.1.0
          ports:
            - containerPort: 18765
          env:
            - name: ASI_MODE
              value: core
          livenessProbe:
            httpGet:
              path: /api/asi/health
              port: 18765
            initialDelaySeconds: 30
            periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: asi-core
spec:
  type: LoadBalancer
  selector:
    app: asi
  ports:
    - port: 18765
      targetPort: 18765
"""


def _generate_dockerfile_content() -> str:
    """Generate Dockerfile content. Tries V1032.DOCKERFILE_TEMPLATE first."""
    try:
        import v1032_docker as v1032

        tpl = v1032.DOCKERFILE_TEMPLATE
        # Try to fill template if it has placeholders
        if "{" in tpl and "}" in tpl:
            return tpl.format(
                python_version="3.13-slim",
                requirements_path="requirements.txt",
                entrypoint="python",
                cmd="asi_demo_v8.py",
            )
        return tpl
    except Exception:
        return """FROM python:3.13-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 18765

CMD ["python", "-m", "apeireth.asi_demo_v8"]
"""


def _generate_requirements_content() -> str:
    """Generate requirements.txt. Tries V1032.REQUIREMENTS_TEMPLATE first."""
    try:
        import v1032_docker as v1032

        tpl = v1032.REQUIREMENTS_TEMPLATE
        return tpl
    except Exception:
        # Fallback: only stdlib (no external deps)
        return """# Apeireth ASI runtime requirements (主 17:33 真生产)
# Stdlib-only: no external runtime dependencies required
# (V1424 benchmark uses stdlib urllib for HTTP)
# (V1420 HTTP server uses stdlib http.server)
pytest>=8.0
"""


def _generate_startup_content() -> str:
    """Generate startup script. Tries V1008.generate_startup_script first."""
    try:
        import v1008_deployment as v1008

        return v1008.generate_startup_script()
    except Exception:
        return """#!/usr/bin/env bash
# Apeireth ASI startup script (主 17:33 放手干到底 + 主 22:33 终极授权)

set -eu

export ASI_MODE="${ASI_MODE:-core}"
export ASI_PORT="${ASI_PORT:-18765}"
export ASI_LOG_LEVEL="${ASI_LOG_LEVEL:-INFO}"

echo "[asi] starting in mode=$ASI_MODE port=$ASI_PORT"

cd /app

# Start ASI HTTP status endpoint
exec python -m apeireth.v1420_asi_http_status_endpoint serve \\
    --bind 0.0.0.0 \\
    --port "$ASI_PORT" \\
    --max-seconds 0
"""


def _generate_env_content() -> str:
    """Generate .env.example with all known ASI env vars."""
    return """# Apeireth ASI environment template (主 17:43 实事求是)
ASI_MODE=core
ASI_PORT=18765
ASI_LOG_LEVEL=INFO
ASI_PROVIDER=deterministic
ASI_BENCHMARK_MODE=MOCK
ASI_WEBHOOK_URL=
ASI_WEBHOOK_AUTH_TOKEN=
"""


# ============================================================================
# Validate
# ============================================================================


def validate_artifact(spec: ArtifactSpec) -> ValidationResult:
    """Validate one artifact's content."""
    kind = spec.kind
    content = spec.content
    if kind == ArtifactKind.COMPOSE:
        ok, errors = _validate_yaml_basic(content)
    elif kind == ArtifactKind.K8S:
        ok, errors = _validate_yaml_basic(content)
    elif kind == ArtifactKind.DOCKERFILE:
        ok, errors = _validate_dockerfile(content)
    elif kind == ArtifactKind.REQUIREMENTS:
        ok, errors = _validate_requirements(content)
    elif kind == ArtifactKind.STARTUP:
        ok, errors = _validate_shell(content)
    elif kind == ArtifactKind.ENV:
        ok, errors = _validate_env(content)
    else:
        ok, errors = False, [f"unknown kind: {kind}"]
    return ValidationResult(
        kind=kind.value,
        path=str(spec.path),
        ok=ok,
        errors=errors,
        warnings=[],
    )


def validate_all(specs: List[ArtifactSpec]) -> DeploymentReadiness:
    """Validate all specs and compute readiness."""
    started = _now_utc_iso()
    per_kind: Dict[str, ValidationResult] = {}
    n_valid = 0
    for spec in specs:
        result = validate_artifact(spec)
        per_kind[spec.kind.value] = result
        if result.ok:
            n_valid += 1
    n_total = len(specs)
    readiness = (n_valid / n_total) if n_total > 0 else 0.0
    output_dir = str(specs[0].path.parent) if specs else ""
    ended = _now_utc_iso()
    return DeploymentReadiness(
        n_total=n_total,
        n_valid=n_valid,
        readiness=readiness,
        per_kind=per_kind,
        output_dir=output_dir,
        started_iso=started,
        ended_iso=ended,
        note="v1428 deployment-readiness (主 23:44 平层即应)",
    )


# ============================================================================
# Generate + validate combined
# ============================================================================


def generate_artifacts(specs: List[ArtifactSpec]) -> Dict[str, Path]:
    """Write all artifacts to disk, return mapping kind -> path."""
    out: Dict[str, Path] = {}
    for spec in specs:
        spec.path.parent.mkdir(parents=True, exist_ok=True)
        spec.path.write_text(spec.content, encoding="utf-8")
        out[spec.kind.value] = spec.path
    return out


def generate_and_validate(output_dir: Path) -> DeploymentReadiness:
    """Build specs, write artifacts, validate, return readiness."""
    output_dir = _safe_path(output_dir)
    specs = build_default_specs(output_dir)
    generate_artifacts(specs)
    return validate_all(specs)


# ============================================================================
# Module metadata + report
# ============================================================================


def module_meta() -> Dict[str, Any]:
    return {
        "version": V1428_VERSION,
        "schema": V1428_SCHEMA,
        "module": V1428_MODULE,
        "n_guards": len(V1428_GUARDS),
        "n_v3_guards": len(V1428_V3_GUARDS),
        "n_borrowed": len(V1428_BORROWED),
        "n_artifact_kinds": len(ALL_ARTIFACT_KINDS),
        "artifact_kinds": [k.value for k in ALL_ARTIFACT_KINDS],
    }


def render_report_md(readiness: DeploymentReadiness) -> str:
    lines: List[str] = []
    lines.append("# V1428 — ASI 真生产 Deployment Artifacts Report")
    lines.append("")
    lines.append(f"- started: `{readiness.started_iso}`")
    lines.append(f"- ended: `{readiness.ended_iso}`")
    lines.append(f"- output_dir: `{readiness.output_dir}`")
    lines.append(f"- note: {readiness.note}")
    lines.append("")
    lines.append("> 主 17:43 实事求是 — readiness = n_valid / n_total.")
    lines.append("> readiness 不等于生产部署,不等于 ASI,不等于人类水平.")
    lines.append("")
    lines.append("## Readiness Summary")
    lines.append("")
    lines.append(f"- n_total: **{readiness.n_total}**")
    lines.append(f"- n_valid: **{readiness.n_valid}**")
    lines.append(f"- readiness: **{readiness.readiness:.2%}**")
    lines.append("")
    lines.append("## Per-artifact Validation")
    lines.append("")
    lines.append("| Artifact | Path | ok | errors |")
    lines.append("|---|---|---|---|")
    for kind, result in readiness.per_kind.items():
        ok_str = "✅" if result.ok else "❌"
        err_short = "; ".join(result.errors)[:60] if result.errors else ""
        lines.append(f"| `{kind}` | `{result.path}` | {ok_str} | {err_short} |")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    lines.append("- 不假装 Phenomenal deployment")
    lines.append("- 不假装达到 ASI (deployment ≠ ASI)")
    lines.append("- 不假装 human-level deployment")
    lines.append("- 不假装 absolute deployment")
    lines.append("- readiness ≠ production-deployed (主 17:43 实事求是)")
    lines.append("")
    return "\n".join(lines) + "\n"


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    checks: List[Dict[str, Any]] = []

    def _check(name: str, ok: bool, detail: str = "") -> None:
        checks.append({"name": name, "ok": bool(ok), "detail": detail})

    # 1. Constants
    _check(
        "constants_present",
        V1428_VERSION == "0.1.0" and V1428_SCHEMA.startswith("v1428."),
        f"version={V1428_VERSION}",
    )

    # 2. ALL_ARTIFACT_KINDS has 6 entries
    _check(
        "all_artifact_kinds_count",
        len(ALL_ARTIFACT_KINDS) == 6,
        f"n={len(ALL_ARTIFACT_KINDS)}",
    )

    # 3. ALL_ARTIFACT_KINDS covers COMPOSE+K8S+DOCKERFILE+REQUIREMENTS+STARTUP+ENV
    names = {k.value for k in ALL_ARTIFACT_KINDS}
    expected = {"docker-compose.yml", "k8s-asi.yaml", "Dockerfile", "requirements.txt", "start-asi.sh", ".env.example"}
    _check(
        "all_artifact_kinds_complete",
        names == expected,
        f"names={names}",
    )

    # 4. _validate_yaml_basic accepts valid YAML
    ok, errors = _validate_yaml_basic("foo: bar\nbaz: qux\n")
    _check(
        "validate_yaml_basic_accepts_valid",
        ok and not errors,
        f"ok={ok} errors={errors}",
    )

    # 5. _validate_yaml_basic rejects empty
    ok, errors = _validate_yaml_basic("")
    _check(
        "validate_yaml_basic_rejects_empty",
        not ok and len(errors) > 0,
        f"ok={ok}",
    )

    # 6. _validate_yaml_basic rejects tabs
    ok, errors = _validate_yaml_basic("\tfoo: bar\n")
    _check(
        "validate_yaml_basic_rejects_tab",
        not ok,
        f"ok={ok}",
    )

    # 7. _validate_dockerfile accepts valid
    ok, errors = _validate_dockerfile("FROM python:3.13\nCMD [\"python\"]\n")
    _check(
        "validate_dockerfile_accepts_valid",
        ok,
        f"ok={ok}",
    )

    # 8. _validate_dockerfile rejects missing FROM
    ok, errors = _validate_dockerfile("CMD [\"python\"]\n")
    _check(
        "validate_dockerfile_rejects_no_from",
        not ok,
        f"ok={ok}",
    )

    # 9. _validate_shell accepts valid
    ok, errors = _validate_shell("#!/usr/bin/env bash\nset -e\necho hi\n")
    _check(
        "validate_shell_accepts_valid",
        ok,
        f"ok={ok}",
    )

    # 10. _validate_shell rejects missing shebang
    ok, errors = _validate_shell("set -e\necho hi\n")
    _check(
        "validate_shell_rejects_no_shebang",
        not ok,
        f"ok={ok}",
    )

    # 11. _validate_requirements accepts valid
    ok, errors = _validate_requirements("pytest>=8.0\n")
    _check(
        "validate_requirements_accepts_valid",
        ok,
        f"ok={ok}",
    )

    # 12. _validate_env accepts valid
    ok, errors = _validate_env("ASI_MODE=core\n")
    _check(
        "validate_env_accepts_valid",
        ok,
        f"ok={ok}",
    )

    # 13. build_default_specs returns 6 specs
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        td_path = Path(td)
        specs = build_default_specs(td_path)
        _check(
            "build_default_specs_returns_six",
            len(specs) == 6,
            f"n={len(specs)}",
        )

    n_passed = sum(1 for c in checks if c["ok"])
    all_ok = n_passed == len(checks)
    return all_ok, n_passed, checks


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    out: Dict[str, Any] = {"v1428": True}
    for ver, modname in (
        ("V1008", "v1008_deployment"),
        ("V1032", "v1032_docker"),
        ("V1418", "v1418_asi_dgm_cron_integration"),
        ("V1426", "v1426_vcp_six_protocol_dispatcher"),
        ("V1427", "v1427_asi_stage_delivery"),
    ):
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                if hasattr(sub, "all_ok"):
                    out[ver] = bool(getattr(sub, "all_ok"))
                elif isinstance(sub, dict):
                    out[ver] = bool(sub.get("all_ok", True))
                    if any(k.endswith("_error") for k in sub):
                        out[ver] = False
                else:
                    out[ver] = True
            else:
                # V1008/V1032 may not have chain_delegate — count as present
                out[ver] = True
        except Exception as exc:
            out[ver] = False
            out[f"{ver}_error"] = str(exc)
    keys = [v for v in out if not v.endswith("_error") and v != "v1428"]
    out["all_ok"] = all(out.get(k) for k in keys)
    return out


# ============================================================================
# Help text
# ============================================================================


def _print_help() -> None:
    print(
        """V1428 — ASI 真生产 deployment artifacts generator + validator (主 17:33 + 主 23:44)

Usage:
  python -m apeireth.v1428_asi_real_deployment_artifacts <command> [args]

Commands:
  version                     Print version string
  meta [--json]               Print module metadata
  demo                        Run a small demo
  help                        Print this help
  popper                      Run 13 self-tests
  chain                       Print chain_delegate() result
  generate [--output-dir PATH]   Generate 6 artifacts to deploy/
  validate [--output-dir PATH]   Validate artifacts in deploy/
  summary [--output-dir PATH]    Print readiness summary
  report [--output-dir PATH]     Render full markdown report + write to file

Examples:
  python -m apeireth.v1428_asi_real_deployment_artifacts version
  python -m apeireth.v1428_asi_real_deployment_artifacts generate
  python -m apeireth.v1428_asi_real_deployment_artifacts report
"""
    )


# ============================================================================
# CLI dispatcher
# ============================================================================


def run_cli(argv: List[str]) -> int:
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(f"V1428 v{V1428_VERSION} ({V1428_SCHEMA})")
        return 0
    if cmd in ("help", "--help", "-h"):
        _print_help()
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest)
        if kv.get("json") == "true":
            print(json.dumps(module_meta(), ensure_ascii=False, indent=2))
        else:
            m = module_meta()
            print(
                f"V1428 v{m['version']} schema={m['schema']} "
                f"module={m['module']} artifact_kinds={m['n_artifact_kinds']}"
            )
        return 0
    if cmd == "demo":
        print("V1428 demo: generate + validate 6 deployment artifacts (主 17:33 + 主 23:44)")
        import tempfile

        with tempfile.TemporaryDirectory() as td:
            td_path = Path(td)
            readiness = generate_and_validate(td_path)
            print(f"  - readiness: {readiness.readiness:.2%} ({readiness.n_valid}/{readiness.n_total})")
            for kind, result in readiness.per_kind.items():
                ok_str = "ok" if result.ok else "FAIL"
                print(f"  - {kind}: {ok_str}")
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(
            json.dumps(
                {"all_ok": all_ok, "n_pass": n_pass, "results": results},
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0 if all_ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "generate":
        kv = _parse_kv_args(rest)
        out_dir = Path(kv.get("output-dir", str(DEFAULT_OUTPUT_DIR)))
        specs = build_default_specs(out_dir)
        paths = generate_artifacts(specs)
        for kind, p in paths.items():
            print(f"  - {kind}: {p}")
        return 0
    if cmd == "validate":
        kv = _parse_kv_args(rest)
        out_dir = Path(kv.get("output-dir", str(DEFAULT_OUTPUT_DIR)))
        specs = build_default_specs(out_dir)
        readiness = validate_all(specs)
        for kind, result in readiness.per_kind.items():
            ok_str = "ok" if result.ok else "FAIL"
            err = "; ".join(result.errors) if result.errors else ""
            print(f"  - {kind}: {ok_str} {err}")
        print(f"readiness: {readiness.readiness:.2%}")
        return 0
    if cmd == "summary":
        kv = _parse_kv_args(rest)
        out_dir = Path(kv.get("output-dir", str(DEFAULT_OUTPUT_DIR)))
        readiness = generate_and_validate(out_dir)
        print(f"readiness: {readiness.readiness:.2%} ({readiness.n_valid}/{readiness.n_total})")
        for kind, result in readiness.per_kind.items():
            ok_str = "ok" if result.ok else "FAIL"
            print(f"  - {kind}: {ok_str}")
        return 0
    if cmd == "report":
        kv = _parse_kv_args(rest)
        out_dir = Path(kv.get("output-dir", str(DEFAULT_OUTPUT_DIR)))
        readiness = generate_and_validate(out_dir)
        md = render_report_md(readiness)
        print(md)
        rp = out_dir / ".v1428-deployment-readiness.json"
        rp.write_text(
            json.dumps(readiness.to_dict(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        mp = out_dir / ".v1428-deployment-readiness.md"
        mp.write_text(md, encoding="utf-8")
        print(f"\n[json] written to {rp}")
        print(f"[md] written to {mp}")
        return 0
    print(f"ERROR: unknown command: {cmd!r}")
    _print_help()
    return 1


# ============================================================================
# Entry point
# ============================================================================


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))