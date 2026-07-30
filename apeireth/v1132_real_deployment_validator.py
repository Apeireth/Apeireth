"""V1132 — 真生产 deployment validator (主 17:33 放手干到底 + 主 22:33 ASI 北极星 + 主 06:15 V1050+ 真部署).

主 06:15 06:32 真部署方向: V1008/V1032 Docker 真终端 docker-compose up -d + healthcheck 真跑.
主 17:43 实事求是: 本机未装 docker daemon, 真测能做的全做 (compose 解析 + subprocess 执行 + 配置一致性 + 真实 probe).
主 17:58 + 主 20:46 不假装: 不刷 KPI, 不假装 docker 在跑.

References:
    V1008: DeploymentConfig + generate_docker_compose + generate_k8s_manifest + generate_startup_script
    V1032: V1032Docker (render_dockerfile / render_docker_compose / render_k8s_manifest / render_requirements)
    docker-compose.r8.yml: 18-service R8 deployment manifest (294 lines)
"""
from __future__ import annotations

import os
import re
import shutil
import socket
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

V1132_VERSION = "0.1.0"

# ---------- result types ----------


@dataclass
class CheckResult:
    name: str
    passed: bool
    detail: str
    ms: float = 0.0


@dataclass
class V1132DeploymentReport:
    report_id: str = field(default_factory=lambda: f"rpt-{uuid.uuid4().hex[:8]}")
    timestamp: float = field(default_factory=time.time)
    docker_daemon_available: bool = False
    compose_files_parsed: int = 0
    services_seen: int = 0
    k8s_manifests_ok: int = 0
    dockerfile_valid: int = 0
    subprocess_runs_ok: int = 0
    subprocess_runs_failed: int = 0
    health_probes_ok: int = 0
    health_probes_failed: int = 0
    canonical_bundle_valid: bool = False
    checks: List[CheckResult] = field(default_factory=list)
    artefacts: Dict[str, str] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)

    @property
    def offline_valid(self) -> bool:
        """Static/subprocess validation only; this does not claim containers ran."""
        return (
            self.compose_files_parsed >= 2
            and self.services_seen >= 5
            and self.k8s_manifests_ok >= 3
            and self.dockerfile_valid >= 2
            and self.subprocess_runs_ok >= 2
            and self.subprocess_runs_failed == 0
            and self.canonical_bundle_valid
        )

    @property
    def runtime_valid(self) -> bool:
        """Strict runtime result: daemon and the canonical HTTP endpoint both ran."""
        return (
            self.offline_valid
            and self.docker_daemon_available
            and self.health_probes_ok >= 1
            and self.health_probes_failed == 0
        )

    @property
    def passed(self) -> bool:
        # Backwards-compatible strict verdict. Offline validation is reported separately.
        return self.runtime_valid

    def to_dict(self) -> Dict[str, Any]:
        return {
            "report_id": self.report_id,
            "timestamp": self.timestamp,
            "version": V1132_VERSION,
            "docker_daemon_available": self.docker_daemon_available,
            "compose_files_parsed": self.compose_files_parsed,
            "services_seen": self.services_seen,
            "k8s_manifests_ok": self.k8s_manifests_ok,
            "dockerfile_valid": self.dockerfile_valid,
            "subprocess_runs_ok": self.subprocess_runs_ok,
            "subprocess_runs_failed": self.subprocess_runs_failed,
            "health_probes_ok": self.health_probes_ok,
            "health_probes_failed": self.health_probes_failed,
            "canonical_bundle_valid": self.canonical_bundle_valid,
            "offline_valid": self.offline_valid,
            "runtime_valid": self.runtime_valid,
            "checks": [
                {"name": c.name, "passed": c.passed, "detail": c.detail, "ms": round(c.ms, 3)}
                for c in self.checks
            ],
            "passed": self.passed,
            "notes": list(self.notes),
        }


# ---------- helpers ----------


def _time_call(fn) -> Tuple[Any, float]:
    t0 = time.perf_counter()
    out = fn()
    return out, (time.perf_counter() - t0) * 1000.0


def _check_docker_daemon() -> Tuple[bool, str]:
    """Honest check: docker CLI on PATH AND docker daemon responding."""
    docker_bin = shutil.which("docker")
    if not docker_bin:
        return False, "docker CLI not on PATH (host has no docker installed)"
    try:
        proc = subprocess.run(
            [docker_bin, "info", "--format", "{{.ServerVersion}}"],
            capture_output=True,
            text=True,
            timeout=4,
        )
        if proc.returncode == 0:
            return True, f"docker daemon reachable, server_version={proc.stdout.strip() or 'unknown'}"
        return False, f"docker CLI present but daemon not responding: {proc.stderr.strip()[:160]}"
    except (subprocess.TimeoutExpired, FileNotFoundError) as e:
        return False, f"docker daemon probe failed: {type(e).__name__}: {e}"


def _parse_compose(path: str) -> Tuple[Optional[Dict[str, Any]], str]:
    """Parse a docker-compose YAML file via PyYAML."""
    try:
        import yaml  # type: ignore
    except ImportError:
        return None, "PyYAML not installed"
    if not os.path.isfile(path):
        return None, f"file not found: {path}"
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
        if not isinstance(data, dict):
            return None, f"top-level not a mapping (got {type(data).__name__})"
        if "services" not in data or not isinstance(data["services"], dict):
            return None, "no top-level 'services' mapping"
        return data, f"ok ({len(data['services'])} services)"
    except Exception as e:
        return None, f"yaml parse error: {type(e).__name__}: {e}"


_DOCKERFILE_FROM = re.compile(r"^\s*FROM\s+\S+", re.MULTILINE)


def _validate_dockerfile(text: str) -> Tuple[bool, str]:
    if not text or not text.strip():
        return False, "empty dockerfile"
    if not _DOCKERFILE_FROM.search(text):
        return False, "no FROM directive found"
    if "WORKDIR" not in text and "WORKDIR" not in text.upper():
        return False, "no WORKDIR directive"
    return True, f"FROM + WORKDIR present ({len(text)} chars)"


def _validate_k8s_yaml(text: str) -> Tuple[bool, str]:
    try:
        import yaml  # type: ignore
    except ImportError:
        return False, "PyYAML missing"
    try:
        docs = list(yaml.safe_load_all(text))
    except Exception as e:
        return False, f"yaml parse error: {type(e).__name__}: {e}"
    valid = [d for d in docs if isinstance(d, dict)]
    kinds = [d.get("kind", "?") for d in valid]
    if not valid:
        return False, "no valid YAML documents"
    return True, f"parsed {len(valid)} doc(s), kinds={kinds}"


def _subprocess_run_python(code: str, timeout: float = 30.0) -> Tuple[int, str, str]:
    """Run an inline python snippet in subprocess; return (rc, stdout, stderr)."""
    try:
        proc = subprocess.run(
            [sys.executable, "-c", code],
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        )
        return proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"
    except Exception as e:
        return 125, "", f"{type(e).__name__}: {e}"


def _http_probe(url: str, timeout: float = 2.0) -> Tuple[bool, str]:
    """Honest HTTP probe via urllib (no requests dependency).

    R11-SEC-001 (SSRF hardening):
      严格 scheme 白名单 (http/https) + host 白名单 (loopback).
      拒绝 file:// / gopher:// / ftp:// / data: + 任何非 loopback host.
      防止: 内部端口扫面 (127.0.0.1:3306) / 元数据接口 (169.254.169.254)
             / 任意网络外泄 / file:// 读取本地敏感文件.
    """
    try:
        from urllib.parse import urlparse
        from urllib.request import urlopen  # type: ignore
        from urllib.error import URLError, HTTPError  # type: ignore
    except ImportError:
        return False, "urllib unavailable"
    try:
        parsed = urlparse(url)
    except Exception as e:
        return False, f"url parse error: {type(e).__name__}: {e}"
    if parsed.scheme not in ("http", "https"):
        return False, f"refused: scheme={parsed.scheme!r} not in (http, https)"
    host = (parsed.hostname or "").lower()
    if host not in _LOOPBACK_HOSTS:
        return False, f"refused: host={host!r} not in loopback allowlist"
    if parsed.port is not None and parsed.port not in _LOOPBACK_PORTS:
        # 留口子: 显式 allowlist 端口(由调用方决定); 不在白名单 = 拒绝
        return False, f"refused: port={parsed.port} not in loopback allowlist"
    try:
        resp = urlopen(url, timeout=timeout)
        return (200 <= resp.status < 400), f"HTTP {resp.status}"
    except HTTPError as e:
        return False, f"HTTP {e.code}"
    except (URLError, socket.timeout, ConnectionRefusedError) as e:
        return False, f"{type(e).__name__}: {e}"


# R11-SEC-001: SSRF 防护 — host 白名单仅 loopback(127.0.0.1 / localhost / ::1)
# 端口允许列表覆盖历史生成器端口和 canonical V1075 端口 8765.
_LOOPBACK_HOSTS = frozenset({"127.0.0.1", "localhost", "::1", "0.0.0.0", "0:0:0:0:0:0:0:1"})
_LOOPBACK_PORTS = frozenset({80, 443, 8080, 8081, 8082, 8132, 8765})


# ---------- main orchestrator ----------


class V1132DeploymentValidator:
    """V1132 real deployment validator — runs subprocess, parses real compose files,
    renders V1008/V1032 artefacts and reports what is verified vs. what is blocked
    by missing docker daemon (主 17:43 实事求是)."""

    def __init__(self, repo_root: Optional[str] = None):
        self.repo_root = repo_root or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.apeireth_dir = os.path.join(self.repo_root, "apeireth")
        self.report = V1132DeploymentReport()

    # ---- individual checks ----

    def check_docker_daemon(self) -> CheckResult:
        ok, detail = _check_docker_daemon()
        self.report.docker_daemon_available = ok
        if not ok:
            self.report.notes.append(
                "docker daemon not reachable; container-level checks (docker-compose up -d, "
                "container healthchecks) are NOT executed. Config-level checks still run."
            )
        return CheckResult("docker_daemon_probe", ok, detail)

    def check_compose_files(self) -> List[CheckResult]:
        results: List[CheckResult] = []
        candidates = [
            os.path.join(self.repo_root, "docker-compose.r8.yml"),
            os.path.join(self.repo_root, "docker-compose.yml"),
            os.path.join(self.repo_root, "deploy", "docker-compose.yml"),
        ]
        for path in candidates:
            name = f"compose_parse[{os.path.basename(path)}]"
            (data, detail), ms = _time_call(lambda p=path: _parse_compose(p))
            ok = data is not None
            if ok:
                self.report.compose_files_parsed += 1
                self.report.services_seen += len(data["services"])
                results.append(CheckResult(name, True, f"{detail} ({path})", ms))
            else:
                # File missing is not a hard fail; only flag if it exists but is broken
                if "not found" in detail:
                    results.append(CheckResult(name, True, f"absent (ok): {detail}"))
                else:
                    results.append(CheckResult(name, False, detail, ms))
        return results

    def check_v1008_render(self) -> CheckResult:
        code = (
            "from apeireth.v1008_deployment import ("
            "DeploymentConfig, generate_docker_compose, generate_k8s_manifest, generate_startup_script);"
            " cfg = DeploymentConfig(name='v1132', service_name='v1132-svc', image='python:3.13-slim', port=8132, replicas=1);"
            " compose = generate_docker_compose([cfg]); k8s = generate_k8s_manifest(cfg); sh = generate_startup_script();"
            " print(f'{len(compose)}|{len(k8s)}|{len(sh)}')"
        )
        t0 = time.perf_counter()
        rc, out, err = _subprocess_run_python(code, timeout=20.0)
        ms = (time.perf_counter() - t0) * 1000.0
        if rc != 0:
            self.report.subprocess_runs_failed += 1
            return CheckResult("v1008_subprocess_render", False, f"rc={rc} stderr={err.strip()[:200]}", ms)
        self.report.subprocess_runs_ok += 1
        compose_n, k8s_n, sh_n = out.strip().split("|")
        # store artefact (parse V1008 output)
        from apeireth.v1008_deployment import DeploymentConfig, generate_docker_compose as gdc, generate_k8s_manifest as gkm, generate_startup_script as gss
        cfg = DeploymentConfig(name="v1132", service_name="v1132-svc", image="python:3.13-slim", port=8132)
        self.report.artefacts["v1008_compose.yml"] = gdc([cfg])
        self.report.artefacts["v1008_k8s.yaml"] = gkm(cfg)
        self.report.artefacts["v1008_startup.sh"] = gss()
        ok, kdetail = _validate_k8s_yaml(self.report.artefacts["v1008_k8s.yaml"])
        if ok:
            self.report.k8s_manifests_ok += 1
        return CheckResult("v1008_subprocess_render", True, f"compose={compose_n}chars, k8s={k8s_n}chars, sh={sh_n}chars, k8s_parse={kdetail}", ms)

    def check_v1032_render(self) -> CheckResult:
        code = (
            "from apeireth.v1032_docker import V1032Docker;"
            " d = V1032Docker();"
            " files = d.render_all();"
            " print('|'.join(f'{k}={len(v)}' for k,v in sorted(files.items())))"
        )
        t0 = time.perf_counter()
        rc, out, err = _subprocess_run_python(code, timeout=20.0)
        ms = (time.perf_counter() - t0) * 1000.0
        if rc != 0:
            self.report.subprocess_runs_failed += 1
            return CheckResult("v1032_subprocess_render", False, f"rc={rc} stderr={err.strip()[:200]}", ms)
        self.report.subprocess_runs_ok += 1
        # validate dockerfile
        from apeireth.v1032_docker import V1032Docker as V1032
        d = V1032()
        all_arts = d.render_all()
        self.report.artefacts.update({f"v1032_{k}": v for k, v in all_arts.items()})
        ok, detail = _validate_dockerfile(all_arts["Dockerfile"])
        if ok:
            self.report.dockerfile_valid += 1
        ok2, kdetail = _validate_k8s_yaml(all_arts["k8s-deployment.yaml"])
        if ok2:
            self.report.k8s_manifests_ok += 1
        return CheckResult("v1032_subprocess_render", ok and ok2, f"files={out.strip()}; dockerfile={detail}; k8s={kdetail}", ms)

    def check_canonical_bundle(self) -> CheckResult:
        """Validate deploy/Dockerfile + Compose + Kubernetes as one offline bundle."""
        deploy_dir = os.path.join(self.repo_root, "deploy")
        dockerfile_path = os.path.join(deploy_dir, "Dockerfile")
        compose_path = os.path.join(deploy_dir, "docker-compose.yml")
        k8s_path = os.path.join(deploy_dir, "k8s-asi.yaml")
        requirements_path = os.path.join(deploy_dir, "requirements.txt")
        required_paths = (dockerfile_path, compose_path, k8s_path, requirements_path)
        missing = [p for p in required_paths if not os.path.isfile(p)]
        if missing:
            return CheckResult("canonical_bundle", False, f"missing files: {missing}")

        try:
            import yaml  # type: ignore
            with open(dockerfile_path, encoding="utf-8") as f:
                dockerfile = f.read()
            with open(k8s_path, encoding="utf-8") as f:
                k8s_text = f.read()
            with open(requirements_path, encoding="utf-8") as f:
                requirements = f.read()
            compose, compose_detail = _parse_compose(compose_path)
            k8s_ok, k8s_detail = _validate_k8s_yaml(k8s_text)
            dockerfile_ok, dockerfile_detail = _validate_dockerfile(dockerfile)
            if compose is None or not k8s_ok or not dockerfile_ok:
                return CheckResult(
                    "canonical_bundle", False,
                    f"compose={compose_detail}; dockerfile={dockerfile_detail}; k8s={k8s_detail}",
                )

            service = (compose.get("services") or {}).get("asi-api") or {}
            build = service.get("build") or {}
            compose_ports = {str(p) for p in service.get("ports") or []}
            health_text = str(service.get("healthcheck") or {})
            env = service.get("environment") or {}

            docs = [d for d in yaml.safe_load_all(k8s_text) if isinstance(d, dict)]
            deployment = next((d for d in docs if d.get("kind") == "Deployment"), None)
            k8s_service = next((d for d in docs if d.get("kind") == "Service"), None)
            if deployment is None or k8s_service is None:
                return CheckResult("canonical_bundle", False, "Kubernetes requires Deployment + Service")

            spec = deployment.get("spec") or {}
            pod_spec = (((spec.get("template") or {}).get("spec")) or {})
            containers = pod_spec.get("containers") or []
            container = containers[0] if containers else {}
            selector = (spec.get("selector") or {}).get("matchLabels") or {}
            pod_labels = ((spec.get("template") or {}).get("metadata") or {}).get("labels") or {}
            svc_spec = k8s_service.get("spec") or {}
            svc_ports = svc_spec.get("ports") or []
            probes = [container.get(name) or {} for name in ("startupProbe", "readinessProbe", "livenessProbe")]

            assertions = {
                "pinned_python_base": "FROM python:3.13.14-slim-bookworm" in dockerfile,
                "runtime_requirements_copied": "COPY deploy/requirements.txt" in dockerfile,
                "non_root_image": "USER 10001:10001" in dockerfile,
                "dockerfile_port": "EXPOSE 8765" in dockerfile,
                "dockerfile_server": "apeireth.v1075_asi_real_deployment_run" in dockerfile,
                "dependencies_pinned": "fastapi==" in requirements and "uvicorn==" in requirements,
                "compose_context": isinstance(build, dict) and build.get("context") == ".." and build.get("dockerfile") == "deploy/Dockerfile",
                "compose_image": service.get("image") == "apeireth-asi:0.1.0",
                "compose_port": "8765:8765" in compose_ports,
                "compose_health": "8765/health" in health_text,
                "compose_env": isinstance(env, dict) and str(env.get("V1075_PORT")) == "8765",
                "k8s_selector": bool(selector) and selector.items() <= pod_labels.items() and svc_spec.get("selector") == selector,
                "k8s_image": container.get("image") == service.get("image"),
                "k8s_port": any(p.get("containerPort") == 8765 for p in container.get("ports") or []),
                "k8s_service_port": any(p.get("targetPort") == 8765 for p in svc_ports),
                "k8s_probes": all((p.get("httpGet") or {}).get("port") == 8765 and (p.get("httpGet") or {}).get("path") == "/health" for p in probes),
                "k8s_non_root": (pod_spec.get("securityContext") or {}).get("runAsNonRoot") is True,
                "k8s_rollout": spec.get("revisionHistoryLimit") == 3 and (spec.get("strategy") or {}).get("type") == "RollingUpdate",
            }
            failed = sorted(name for name, passed in assertions.items() if not passed)
            if failed:
                return CheckResult("canonical_bundle", False, f"semantic checks failed: {failed}")

            self.report.canonical_bundle_valid = True
            self.report.dockerfile_valid += 1
            self.report.k8s_manifests_ok += 1
            return CheckResult(
                "canonical_bundle", True,
                f"{len(assertions)}/{len(assertions)} semantic checks passed; image=apeireth-asi:0.1.0 port=8765",
            )
        except Exception as e:
            return CheckResult("canonical_bundle", False, f"{type(e).__name__}: {e}")

    def check_health_probes(self) -> List[CheckResult]:
        """Probe only the canonical deploy/ endpoint; generated examples are offline-only."""
        results: List[CheckResult] = []
        targets = [("http://127.0.0.1:8765/health", "canonical-v1075")]
        for url, label in targets:
            (ok, detail), ms = _time_call(lambda u=url: _http_probe(u, timeout=1.5))
            if ok:
                self.report.health_probes_ok += 1
                results.append(CheckResult(f"probe[{label}]", True, f"{detail} ({url})", ms))
            else:
                self.report.health_probes_failed += 1
                results.append(CheckResult(
                    f"probe[{label}]", False,
                    f"runtime not verified at {url}: {detail}", ms,
                ))
        return results

    def check_consistency(self) -> CheckResult:
        """Cross-check: V1008 compose & V1032 compose & r8 compose share core service names."""
        if "v1008_compose.yml" not in self.report.artefacts:
            return CheckResult("consistency_check", False, "no v1008 compose artefact to cross-check")
        try:
            import yaml  # type: ignore
        except ImportError:
            return CheckResult("consistency_check", False, "PyYAML missing")
        v1008 = yaml.safe_load(self.report.artefacts["v1008_compose.yml"])
        v1032 = yaml.safe_load(self.report.artefacts["v1032_docker-compose.yml"])
        s1008 = set((v1008.get("services") or {}).keys())
        s1032 = set((v1032.get("services") or {}).keys())
        r8_path = os.path.join(self.repo_root, "docker-compose.r8.yml")
        r8_data, _ = _parse_compose(r8_path)
        sr8 = set((r8_data or {}).get("services", {}).keys()) if r8_data else set()
        # We require at least 1 common key across all three, or document the divergence
        common = (s1008 & s1032) | (s1008 & sr8) | (s1032 & sr8)
        if not common and (s1008 or s1032 or sr8):
            self.report.notes.append(
                f"historical V1008/V1032/R8 examples use distinct service names; canonical deploy/ bundle is checked separately: "
                f"v1008={sorted(s1008)} v1032={sorted(s1032)} r8_n={len(sr8)}"
            )
            return CheckResult("consistency_check", True,
                               f"historical generators isolated; canonical bundle governs deploy/: v1008={sorted(s1008)} v1032={sorted(s1032)} r8_n={len(sr8)}")
        return CheckResult("consistency_check", True, f"shared_service_keys={sorted(common)}")

    # ---- orchestration ----

    def run_full_validation(self) -> V1132DeploymentReport:
        self.report.checks.append(self.check_docker_daemon())
        self.report.checks.extend(self.check_compose_files())
        self.report.checks.append(self.check_v1008_render())
        self.report.checks.append(self.check_v1032_render())
        self.report.checks.append(self.check_canonical_bundle())
        self.report.checks.append(self.check_consistency())
        self.report.checks.extend(self.check_health_probes())
        return self.report


def render_markdown(report: V1132DeploymentReport) -> str:
    lines = [
        "# V1132 真部署 validator 报告 (主 06:15 V1050+ 真部署方向 + 主 17:43 实事求是)",
        "",
        f"- report_id: `{report.report_id}`",
        f"- timestamp: {report.timestamp}",
        f"- docker_daemon_available: **{report.docker_daemon_available}**",
        f"- compose_files_parsed: **{report.compose_files_parsed}**",
        f"- services_seen: **{report.services_seen}**",
        f"- k8s_manifests_ok: **{report.k8s_manifests_ok}**",
        f"- dockerfile_valid: **{report.dockerfile_valid}**",
        f"- subprocess_runs_ok / failed: **{report.subprocess_runs_ok}** / {report.subprocess_runs_failed}",
        f"- health_probes_ok / failed: **{report.health_probes_ok}** / {report.health_probes_failed}",
        f"- canonical_bundle_valid: **{report.canonical_bundle_valid}**",
        f"- offline_valid: **{report.offline_valid}** (static/subprocess only; no container claim)",
        f"- runtime_valid: **{report.runtime_valid}** (requires daemon + live canonical health probe)",
        f"- passed: **{report.passed}** (strict runtime verdict)",
        "",
        "## Checks",
        "",
        "| name | passed | detail |",
        "|------|--------|--------|",
    ]
    for c in report.checks:
        d = c.detail.replace("|", "\\|")
        lines.append(f"| {c.name} | {c.passed} | {d[:200]} |")
    if report.notes:
        lines += ["", "## Notes", ""]
        for n in report.notes:
            lines.append(f"- {n}")
    return "\n".join(lines) + "\n"


def main(argv: Optional[List[str]] = None) -> int:
    v = V1132DeploymentValidator()
    rep = v.run_full_validation()
    md = render_markdown(rep)
    print(md)
    return 0 if rep.passed else 1


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
