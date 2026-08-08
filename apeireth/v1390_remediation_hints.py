"""Phase 1390 v1390_remediation_hints — V1390 ASI 真生产 remediation hints (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31).

V1390 = real production remediation hints for V1384/V1385/V1386 findings.
- 30+ rule_ids: each maps to a real remediation hint with code example
- 真借鉴: hadolint explanations (https://github.com/hadolint/hadolint),
  compose-spec docs (https://compose-spec.io/), Kubernetes best practices
  (https://kubernetes.io/docs/concepts/), Dockle rules
- 任何人都能接手 (主 00:56): 1 个 dict 1 个 CLI, 0 magic
- 不假装 (主 17:58): 提示是建议, 不是 truth; 任何人可以扩展
- 实事求是 (主 17:43): 真给 fix example, 真 reference link

V1390 真生产 数据结构:
- RemediationHint: rule_id, severity, title, why (rationale), fix (code example), ref (link)
- get_hint(rule_id) -> Optional[RemediationHint]
- list_hints() -> Dict[str, RemediationHint]
- apply_to_findings(findings) -> List[RemediationHint] (deduplicated)
- render_markdown(findings) -> str (group by severity)
- run_cli(): main entry; rules / hint <rule_id> / apply <targets> / version
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional


V1390_VERSION = "0.1.0"
V1390_SCHEMA = "v1390.remediation-hints/v1"

# V1390 真生产 30+ rule_id 真 remediation (主 17:43 实事求是)
# 字段: severity, title, why, fix, ref
# 真借鉴: hadolint explanations / compose-spec / k8s best practices / Dockle
HINTS: Dict[str, Dict[str, str]] = {
    # ===== V1384 Dockerfile (hadolint + 6 自有) =====
    "DL3008": {
        "severity": "warning",
        "title": "Pin versions in apt-get install",
        "why": "Unpinned package versions make builds non-reproducible and can break when upstream updates. Pinning versions ensures the same image is built every time.",
        "fix": "RUN apt-get update && apt-get install -y --no-install-recommends \\\n    package=1.2.3 \\\n    package2=4.5.6 \\\n && rm -rf /var/lib/apt/lists/*",
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3008",
    },
    "DL3009": {
        "severity": "info",
        "title": "Delete apt-get lists after install",
        "why": "apt-get update leaves package lists in /var/lib/apt/lists/ which bloat the image. Removing them after install reduces image size by tens of MB.",
        "fix": "RUN apt-get update && apt-get install -y --no-install-recommends \\\n    package \\\n && rm -rf /var/lib/apt/lists/*",
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3009",
    },
    "DL3015": {
        "severity": "info",
        "title": "Use --no-install-recommends with apt-get install",
        "why": "By default apt-get install pulls in 'Recommends' packages which are often unnecessary. Using --no-install-recommends reduces image size.",
        "fix": "RUN apt-get update && apt-get install -y --no-install-recommends \\\n    package",
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3015",
    },
    "DL3020": {
        "severity": "info",
        "title": "Use COPY instead of ADD for local files",
        "why": "ADD has magic behaviors (URL fetch, tar extraction) that surprise users. COPY is explicit and safer for local files.",
        "fix": "COPY ./local-file.txt /app/local-file.txt",
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3020",
    },
    "DL3025": {
        "severity": "warning",
        "title": "Use JSON format for CMD",
        "why": "CMD ['sh', '-c', '...'] spawns an extra shell process and can fail with weird signal handling. Use exec form (JSON array) for proper signal handling.",
        "fix": 'CMD ["python3", "app.py"]',
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3025",
    },
    "DL4000": {
        "severity": "warning",
        "title": "Specify CMD or ENTRYPOINT",
        "why": "Without CMD or ENTRYPOINT the container will fail at run with 'no command specified'. Always declare how the container should run.",
        "fix": 'CMD ["python3", "app.py"]',
        "ref": "https://github.com/hadolint/hadolint/wiki/DL4000",
    },
    "V1384-NO-USER": {
        "severity": "error",
        "title": "USER directive missing",
        "why": "Containers running as root have full system privileges. If the container is compromised, the attacker has root inside the container. Switch to a non-root user.",
        "fix": "RUN adduser --system --no-create-home --uid 1001 appuser\nUSER appuser",
        "ref": "https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user",
    },
    "V1384-NO-HEALTHCHECK": {
        "severity": "warning",
        "title": "HEALTHCHECK directive missing",
        "why": "Without HEALTHCHECK, orchestrators (k8s, compose) cannot detect a degraded container. Define how to check the service is alive.",
        "fix": "HEALTHCHECK --interval=30s --timeout=3s --retries=3 \\\n    CMD curl -fsS http://localhost:8080/health || exit 1",
        "ref": "https://docs.docker.com/engine/reference/builder/#healthcheck",
    },
    "V1384-FROM-LATEST": {
        "severity": "error",
        "title": "FROM uses :latest tag",
        "why": "The :latest tag is mutable — every pull can get a different version, breaking reproducibility and audit trails. Pin to a specific version or digest.",
        "fix": "FROM python:3.11.7-slim-bookworm\n# or pin by digest:\n# FROM python:3.11.7-slim-bookworm@sha256:abcdef0123456789...",
        "ref": "https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#use-multi-stage-builds",
    },
    "V1384-FROM-NO-TAG": {
        "severity": "warning",
        "title": "FROM without explicit tag",
        "why": "If no tag is specified, Docker defaults to :latest. Pin to a specific version for reproducibility.",
        "fix": "FROM python:3.11.7-slim-bookworm",
        "ref": "https://docs.docker.com/engine/reference/builder/#from",
    },
    "V1384-ADD-INSECURE-URL": {
        "severity": "error",
        "title": "ADD with HTTP URL",
        "why": "HTTP downloads are not integrity-checked. If the upstream is compromised or serves different content, your build is poisoned. Use HTTPS or download + verify checksum.",
        "fix": "# Option 1: HTTPS\nADD https://example.com/file.tar.gz /tmp/\n# Option 2: download + verify\nRUN curl -fsSL https://example.com/file.tar.gz -o /tmp/file.tar.gz \\\n && echo \"<sha256sum>  /tmp/file.tar.gz\" | sha256sum -c -",
        "ref": "https://github.com/hadolint/hadolint/wiki/DL3020",
    },
    "V1384-ADD-NO-VERIFY": {
        "severity": "warning",
        "title": "ADD file with no checksum verification",
        "why": "If the source file is corrupted or tampered with, the build silently uses the bad content. Add sha256sum verification.",
        "fix": "RUN curl -fsSL https://example.com/file.tar.gz -o /tmp/file.tar.gz \\\n && echo \"<sha256sum>  /tmp/file.tar.gz\" | sha256sum -c -",
        "ref": "https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#using-ar_pip_etc",
    },
    "V1384-UNNECESSARY-SUDO": {
        "severity": "warning",
        "title": "Unnecessary sudo in container",
        "why": "Containers run as root by default — sudo is unnecessary and adds attack surface. Drop sudo and run as non-root user.",
        "fix": "# Remove sudo and ensure USER is set\nRUN apt-get purge -y sudo \\\n && rm -rf /var/lib/apt/lists/*\nUSER appuser",
        "ref": "https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user",
    },
    "V1384-ABS-PATH-WITHOUT-WORKDIR": {
        "severity": "warning",
        "title": "Absolute path used without WORKDIR",
        "why": "Using absolute paths (/app/bin) without WORKDIR makes the Dockerfile harder to read and refactor. Use WORKDIR + relative paths.",
        "fix": "WORKDIR /app\nCOPY app.py ./\nCMD [\"python3\", \"./app.py\"]",
        "ref": "https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#workdir",
    },
    # ===== V1385 Compose (compose-spec + 7 自有) =====
    "COMPOSE-LATEST-TAG": {
        "severity": "error",
        "title": "Image uses :latest tag",
        "why": "Mutable tags break reproducibility. Pin to a specific version of the base image.",
        "fix": "services:\n  app:\n    image: myapp:1.2.3\n    # or pin by digest:\n    # image: myapp:1.2.3@sha256:abcdef...",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#image",
    },
    "COMPOSE-PRIVILEGED": {
        "severity": "error",
        "title": "privileged: true disables container isolation",
        "why": "Privileged mode grants the container nearly all host capabilities. Only use if absolutely required (e.g., Docker-in-Docker).",
        "fix": "services:\n  app:\n    cap_add:\n      - SYS_ADMIN   # only add the specific capability you need\n    # or better: remove privileged and grant per-device access",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#privileged",
    },
    "COMPOSE-NETWORK-HOST": {
        "severity": "warning",
        "title": "network_mode: host bypasses network isolation",
        "why": "Host networking exposes all container ports on the host without port mapping. Use bridge networking with explicit ports.",
        "fix": "services:\n  app:\n    ports:\n      - \"8080:8080\"   # explicit port mapping\n    # remove network_mode: host",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#network_mode",
    },
    "COMPOSE-DOCKER-SOCK": {
        "severity": "error",
        "title": "Mounting /var/run/docker.sock is a security risk",
        "why": "Mounting the Docker socket grants the container full control of the host Docker daemon — equivalent to root on the host. Avoid unless you specifically need Docker-in-Docker.",
        "fix": "services:\n  app:\n    volumes:\n      - /var/run/docker.sock:/var/run/docker.sock:ro  # at least read-only if required\n    # better: use a separate Docker-in-Docker sidecar or rootless Docker",
        "ref": "https://docs.docker.com/engine/security/https/",
    },
    "COMPOSE-PLAINTEXT-SECRET": {
        "severity": "error",
        "title": "Plaintext secret in environment variable",
        "why": "Secrets in compose files get committed to git and leak. Use Docker secrets (.env files, vault, or external secret manager).",
        "fix": "services:\n  app:\n    environment:\n      - DB_PASSWORD_FILE=/run/secrets/db_password   # reference a file\n    secrets:\n      - db_password\n\nsecrets:\n  db_password:\n    file: ./secrets/db_password.txt",
        "ref": "https://docs.docker.com/compose/compose-file/09-secrets/",
    },
    "COMPOSE-MISSING-RESTART": {
        "severity": "info",
        "title": "Service has no restart policy",
        "why": "Without a restart policy, containers stay stopped after crash. Define restart policy so the orchestrator recovers.",
        "fix": "services:\n  app:\n    restart: unless-stopped   # or: always, on-failure[:max-retries]",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#restart",
    },
    "COMPOSE-MISSING-MEM-LIMIT": {
        "severity": "warning",
        "title": "Service has no memory limit",
        "why": "Without memory limits, a buggy container can exhaust host memory. Always set memory limits for production.",
        "fix": "services:\n  app:\n    deploy:\n      resources:\n        limits:\n          memory: 512M\n        reservations:\n          memory: 256M",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#deploy",
    },
    "COMPOSE-DEPENDS-NO-HEALTHY": {
        "severity": "warning",
        "title": "depends_on without condition: service_healthy",
        "why": "depends_on only waits for the container to start, not for the service to be ready. Use condition: service_healthy to wait for actual readiness.",
        "fix": "services:\n  app:\n    depends_on:\n      db:\n        condition: service_healthy\n  db:\n    image: postgres:15\n    healthcheck:\n      test: [\"CMD-SHELL\", \"pg_isready -U postgres\"]\n      interval: 5s\n      retries: 5",
        "ref": "https://docs.docker.com/compose/compose-file/05-services/#depends_on",
    },
    # ===== V1386 k8s (kubeval + kubeconform + polaris) =====
    "K8S-LATEST-TAG": {
        "severity": "error",
        "title": "Container image uses :latest tag",
        "why": "Mutable tags break rollouts and audit trails. Pin to immutable tag or digest.",
        "fix": "spec:\n  containers:\n    - name: app\n      image: myapp:1.2.3\n      # or pin by digest:\n      # image: myapp:1.2.3@sha256:abcdef0123456789...",
        "ref": "https://kubernetes.io/docs/concepts/containers/images/",
    },
    "K8S-NO-RESOURCE-LIMITS": {
        "severity": "warning",
        "title": "Container has no resource limits",
        "why": "Without CPU/memory limits, a pod can starve neighbors or be OOM-killed unpredictably. Set requests + limits.",
        "fix": "spec:\n  containers:\n    - name: app\n      resources:\n        requests:\n          cpu: 100m\n          memory: 128Mi\n        limits:\n          cpu: 500m\n          memory: 512Mi",
        "ref": "https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/",
    },
    "K8S-NO-READINESS": {
        "severity": "warning",
        "title": "Pod has no readiness probe",
        "why": "Without readiness probe, the pod receives traffic before it's ready to serve. Define how to check readiness.",
        "fix": "spec:\n  containers:\n    - name: app\n      readinessProbe:\n        httpGet:\n          path: /ready\n          port: 8080\n        initialDelaySeconds: 5\n        periodSeconds: 10",
        "ref": "https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/",
    },
    "K8S-NO-LIVENESS": {
        "severity": "warning",
        "title": "Pod has no liveness probe",
        "why": "Without liveness probe, a hung pod is never restarted. Define how to detect a dead container.",
        "fix": "spec:\n  containers:\n    - name: app\n      livenessProbe:\n        httpGet:\n          path: /health\n          port: 8080\n        initialDelaySeconds: 30\n        periodSeconds: 30",
        "ref": "https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/",
    },
    "K8S-NO-SECURITY-CTX": {
        "severity": "warning",
        "title": "Pod has no security context",
        "why": "Without securityContext, pods run as root with full capabilities. Set runAsNonRoot, readOnlyRootFilesystem, drop ALL capabilities.",
        "fix": "spec:\n  securityContext:\n    runAsNonRoot: true\n    runAsUser: 1001\n    fsGroup: 1001\n    seccompProfile:\n      type: RuntimeDefault\n  containers:\n    - name: app\n      securityContext:\n        allowPrivilegeEscalation: false\n        readOnlyRootFilesystem: true\n        capabilities:\n          drop:\n            - ALL",
        "ref": "https://kubernetes.io/docs/tasks/configure-pod-container/security-context/",
    },
    "K8S-PRIVILEGED": {
        "severity": "error",
        "title": "Pod runs in privileged mode",
        "why": "privileged: true disables nearly all container isolation. Equivalent to root on the host. Avoid unless absolutely necessary.",
        "fix": "spec:\n  containers:\n    - name: app\n      securityContext:\n        privileged: false\n        # If you need specific capabilities, grant them explicitly:\n        capabilities:\n          add:\n            - NET_ADMIN   # only what's needed",
        "ref": "https://kubernetes.io/docs/concepts/security/pod-security-standards/",
    },
    "K8S-HOST-NETWORK": {
        "severity": "warning",
        "title": "hostNetwork: true bypasses network isolation",
        "why": "Host network shares the node's network namespace. The pod can listen on any host port and is reachable from any pod on the node.",
        "fix": "spec:\n  hostNetwork: false   # remove or set to false\n  # Use a Service for external exposure",
        "ref": "https://kubernetes.io/docs/concepts/services-networking/dns-pod-service/",
    },
    "K8S-PLAINTEXT-SECRET": {
        "severity": "error",
        "title": "Plaintext secret in environment variable",
        "why": "Plaintext secrets in manifests are committed to git and visible to anyone with namespace read access. Use Secrets + sealed-secrets / external-secrets.",
        "fix": "spec:\n  containers:\n    - name: app\n      envFrom:\n        - secretRef:\n            name: db-credentials\n      # db-credentials is a Secret managed by external-secrets.io / sealed-secrets",
        "ref": "https://kubernetes.io/docs/concepts/configuration/secret/",
    },
}


# V1390 真生产 GUARDS (主 17:43 实事求是)
V1390_GUARDS: tuple = (
    "GUARD_HINTS_REAL",         # 真有 30+ rule_id → remediation
    "GUARD_NO_CAP_CHANGE",      # 不改 ASI cap
    "GUARD_DETERMINISTIC",      # 同样 rule_id → 同样 hint
    "GUARD_HONEST_DISCLOSURE",  # 提示带 ref, 标注"建议"
    "GUARD_FIX_HAS_CODE",       # 至少 18+ rule 带 fix example
    "GUARD_REF_HAS_URL",        # 至少 reference 是 https://
    "GUARD_CLI_RUNNABLE",       # CLI 可跑
    "GUARD_RENDER_MARKDOWN",    # render_markdown 真能拼 Markdown
)


# ============================================================================
# V1390 真生产 数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class RemediationHint:
    """V1390 真生产 单个 rule_id 的 remediation hint (主 17:43)."""

    rule_id: str
    severity: str               # error / warning / info
    title: str
    why: str
    fix: str
    ref: str

    def to_dict(self) -> Dict[str, str]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "title": self.title,
            "why": self.why,
            "fix": self.fix,
            "ref": self.ref,
        }


def get_hint(rule_id: str) -> Optional[RemediationHint]:
    """V1390 真生产: 根据 rule_id 返回 RemediationHint, None = 无提示."""
    data = HINTS.get(rule_id)
    if not data:
        return None
    return RemediationHint(
        rule_id=rule_id,
        severity=data["severity"],
        title=data["title"],
        why=data["why"],
        fix=data["fix"],
        ref=data["ref"],
    )


def list_hints() -> Dict[str, RemediationHint]:
    """V1390 真生产: 返回所有 rule_id → RemediationHint 字典."""
    return {rid: get_hint(rid) for rid in HINTS}  # type: ignore[misc]


def apply_to_findings(
    findings: Iterable[Dict[str, Any]],
) -> List[RemediationHint]:
    """V1390 真生产: 给 findings (list of dict with rule_id) 返回 deduplicated hints.

    真服务 V1388 diff result: {n_new, new_by_rule: {rule_id: count}, ...}
    当输入是 dict with 'rule_id' 字段时, 直接用.
    当输入是 dict with 'new_by_rule' (V1388 格式), 展开 rule_id → count.
    """
    seen: Dict[str, RemediationHint] = {}
    for f in findings:
        if not isinstance(f, dict):
            continue
        # Case 1: f has 'rule_id' directly
        if "rule_id" in f:
            rid = f["rule_id"]
            h = get_hint(rid)
            if h and rid not in seen:
                seen[rid] = h
        # Case 2: f has 'new_by_rule' (V1388 diff format)
        elif "new_by_rule" in f and isinstance(f["new_by_rule"], dict):
            for rid in f["new_by_rule"]:
                h = get_hint(rid)
                if h and rid not in seen:
                    seen[rid] = h
    # V1390 error-first 排序
    severity_order = {"error": 0, "warning": 1, "info": 2}
    return sorted(
        seen.values(),
        key=lambda h: (severity_order.get(h.severity, 3), h.rule_id),
    )


def render_markdown(hints: List[RemediationHint]) -> str:
    """V1390 真生产: 把 hints 渲染为 Markdown (主 17:43 实事求是)."""
    lines: List[str] = []
    lines.append(f"# V1390 Remediation Hints ({len(hints)} hints)")
    lines.append("")
    lines.append(f"Schema: `{V1390_SCHEMA}`  Version: `{V1390_VERSION}`")
    lines.append("")
    # V1390 by severity
    by_sev: Dict[str, List[RemediationHint]] = {"error": [], "warning": [], "info": []}
    for h in hints:
        by_sev.setdefault(h.severity, []).append(h)
    for sev in ("error", "warning", "info"):
        items = by_sev.get(sev, [])
        if not items:
            continue
        lines.append(f"## {sev.upper()} ({len(items)})")
        lines.append("")
        for h in items:
            lines.append(f"### {h.rule_id} — {h.title}")
            lines.append("")
            lines.append(f"**Why**: {h.why}")
            lines.append("")
            lines.append(f"**Fix**:")
            lines.append("")
            lines.append("```dockerfile")
            lines.append(h.fix)
            lines.append("```")
            lines.append("")
            lines.append(f"**Reference**: {h.ref}")
            lines.append("")
    return "\n".join(lines)


def popper_self_test() -> Dict[str, Any]:
    """V1390 真生产 Popper self-test (主 17:43 实事求是): 验证 hints 库内部一致.

    主 17:43: same point 真可 reproduce, 真可测试.
    """
    failures: List[str] = []
    # Test 1: 至少 30 rule_id
    if len(HINTS) < 30:
        failures.append(f"expected >=30 hints, got {len(HINTS)}")
    # Test 2: 每个 rule_id 都有 severity / title / why / fix / ref
    for rid, data in HINTS.items():
        for k in ("severity", "title", "why", "fix", "ref"):
            if k not in data or not data[k]:
                failures.append(f"{rid}: missing {k}")
        # Test 3: severity 必须 error/warning/info
        if data.get("severity") not in ("error", "warning", "info"):
            failures.append(f"{rid}: bad severity {data.get('severity')}")
        # Test 4: ref 必须 https://
        ref = data.get("ref", "")
        if not ref.startswith("https://"):
            failures.append(f"{rid}: ref not https:// ({ref[:60]})")
        # Test 5: fix 必须能 inline (>= 20 chars)
        fix = data.get("fix", "")
        if len(fix) < 20:
            failures.append(f"{rid}: fix too short ({len(fix)} chars)")
    # Test 6: get_hint / list_hints consistency
    all_listed = list_hints()
    if len(all_listed) != len(HINTS):
        failures.append(f"list_hints len {len(all_listed)} != HINTS len {len(HINTS)}")
    # Test 7: 已知 rule 真能 get
    for rid in ("DL3008", "COMPOSE-PRIVILEGED", "K8S-NO-RESOURCE-LIMITS"):
        h = get_hint(rid)
        if h is None or h.rule_id != rid:
            failures.append(f"get_hint({rid}) failed")
    # Test 8: 未知 rule 返回 None
    if get_hint("NOPE-9999") is not None:
        failures.append("get_hint(NOPE-9999) should be None")
    # Test 9: apply_to_findings 真能用
    findings = [
        {"rule_id": "DL3008"},
        {"rule_id": "COMPOSE-PRIVILEGED"},
        {"new_by_rule": {"K8S-NO-RESOURCE-LIMITS": 5, "DL3009": 1}},
    ]
    hints = apply_to_findings(findings)
    if len(hints) != 4:
        failures.append(f"apply_to_findings expected 4 hints, got {len(hints)}")
    # Test 10: render_markdown 真能拼
    md = render_markdown(hints)
    if "V1390" not in md or "Reference" not in md:
        failures.append("render_markdown missing required sections")
    return {
        "passed": len(failures) == 0,
        "failures": failures,
        "n_hints": len(HINTS),
        "n_tested": 10,
    }


# ============================================================================
# V1390 CLI (主 17:43 真可执行)
# ============================================================================


def _run_scan_v1387(target: str) -> List[Dict[str, Any]]:
    """V1390 真生产: 真跑 V1387 + V1388 拿 findings, 返回 List[dict with rule_id>."""
    try:
        from apeireth.v1388_v1387_baseline_diff import V1388BaselineDiff  # type: ignore
    except Exception:
        try:
            import sys as _sys
            _v1390_dir = Path(__file__).resolve().parent
            if str(_v1390_dir) not in _sys.path:
                _sys.path.insert(0, str(_v1390_dir))
            from v1388_v1387_baseline_diff import V1388BaselineDiff  # type: ignore
        except Exception:
            return []
    diff = V1388BaselineDiff()
    res = diff.run(target=target, baseline_path=None)
    # V1388 diff result has .new_by_rule which is {rule_id: count}
    findings: List[Dict[str, Any]] = []
    for rid, count in res.new_by_rule.items():
        for _ in range(count):
            findings.append({"rule_id": rid})
    return findings


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1390 真生产 CLI 主入口 (主 17:43 真可执行)."""
    parser = argparse.ArgumentParser(
        prog="v1390-remediation-hints",
        description=f"V1390 real production remediation hints (v{V1390_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="V1390 version")
    sub.add_parser("rules", help="list all rule_ids")
    sub.add_parser("stats", help="JSON stats")

    p_hint = sub.add_parser("hint", help="show hint for one rule_id")
    p_hint.add_argument("rule_id")

    p_apply = sub.add_parser("apply", help="apply hints to V1387/V1388 findings on TARGET")
    p_apply.add_argument("target", help="target directory to scan")
    p_apply.add_argument("--md", action="store_true", help="render as Markdown")

    p_self = sub.add_parser("popper", help="V1390 Popper self-test")
    p_md = sub.add_parser("markdown", help="render all hints as one Markdown")
    p_md.add_argument("--severity", default=None, help="filter severity: error/warning/info")

    args = parser.parse_args(argv)
    cmd = args.cmd or "version"

    if cmd == "version":
        print(f"V1390 remediation hints v{V1390_VERSION} (schema {V1390_SCHEMA})")
        return 0
    if cmd == "rules":
        for rid in sorted(HINTS.keys()):
            print(rid)
        return 0
    if cmd == "stats":
        sev_count: Dict[str, int] = {"error": 0, "warning": 0, "info": 0}
        for d in HINTS.values():
            sev_count[d["severity"]] = sev_count.get(d["severity"], 0) + 1
        data = {
            "version": V1390_VERSION,
            "schema": V1390_SCHEMA,
            "n_hints": len(HINTS),
            "by_severity": sev_count,
        }
        print(json.dumps(data, indent=2, ensure_ascii=False))
        return 0
    if cmd == "hint":
        h = get_hint(args.rule_id)
        if h is None:
            print(f"unknown rule_id: {args.rule_id}", file=sys.stderr)
            return 1
        print(json.dumps(h.to_dict(), indent=2, ensure_ascii=False))
        return 0
    if cmd == "apply":
        findings = _run_scan_v1387(args.target)
        if not findings:
            print(f"no findings (target scan unavailable or clean): {args.target}", file=sys.stderr)
            # Still print empty-but-valid output
            hints: List[RemediationHint] = []
        else:
            hints = apply_to_findings(findings)
        if args.md:
            print(render_markdown(hints))
        else:
            data = {
                "version": V1390_VERSION,
                "schema": V1390_SCHEMA,
                "target": args.target,
                "n_findings": len(findings),
                "n_hints": len(hints),
                "hints": [h.to_dict() for h in hints],
            }
            print(json.dumps(data, indent=2, ensure_ascii=False))
        return 0
    if cmd == "popper":
        result = popper_self_test()
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return 0 if result["passed"] else 1
    if cmd == "markdown":
        all_hints = list(list_hints().values())
        if args.severity:
            all_hints = [h for h in all_hints if h.severity == args.severity]
        print(render_markdown(all_hints))
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli())
