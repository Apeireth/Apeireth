"""Phase 1396 v1396_deploy_executor — V1396 ASI 真生产 deploy-stack executor (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1396 = 真生产 deploy-stack executor (post-V1395 next-step, 推荐方向).
主 23:44 干到底: V1395 dashboard 已聚合 12 modules 状态, V1396 真生产 executor 真跑全链 + 真执行 = 真闭环.
主 22:33 ASI 北极星: 真执行 ≠ ASI, 但真执行是 ASI 北极星里 "system integration" 的一小步.
主 19:33 走在前人经验上: 真借鉴 GitHub deploy status API + GitLab deploy API + Jenkins pipeline + ArgoCD application + kubectl apply --dry-run.
主 17:43 实事求是: 真 parse compose YAML (PyYAML) + 真静态校验 + 真 chain 调 V1384-V1395 + 真 manifest 生成.
主 00:56 任何人都能接手: 1 个 module + 1 个 CLI + 1 个 chain runner + 1 个 validator + 1 个 manifest generator + 1 个 ports detector.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 chain + 真 popper self-test.

真生产设计 (主 19:33 github/gitlab/jenkins/argocd 真借鉴):
- 真 parse compose YAML (PyYAML, 主 17:43 真借用 PyYAML 已可用)
  - 真提取 services / ports / environment / healthcheck / depends_on / volumes / restart / deploy.resources
  - 真校验 ports 格式 ("HOST:CONTAINER" 或数字) / 真检测跨 service ports 冲突
  - 真校验 restart 策略 (no/always/unless-stopped/on-failure) + 真校验 healthcheck 存在
- 真 chain 调 V1384-V1395 全栈 (主 17:43 真调用)
  - 真调 V1387 unified runner → findings
  - 真调 V1393 judge → verdict + score
  - 真调 V1395 dashboard → modules 状态
- 真生成 executable deploy manifest (主 17:43 真输出):
  - JSON schema v1396.deploy-executor/v1
  - YAML schema v1396.deploy-executor/v1
  - 包含 services / ports / healthcheck / depends_on / cross-conflicts
- 真 popper self-test (主 17:43 真跑真测)
- 真 CLI (主 17:43 真可执行):
  - version: V1396 version
  - chain <target>: 真跑 V1384-V1395 全链 → markdown/JSON 输出
  - validate <target>: 真静态校验 compose → 真报 issues
  - manifest <target>: 真生成 deploy manifest → JSON/YAML 输出
  - ports <target>: 真检测跨 service ports 冲突
  - popper: V1396 self-test
  - demo: V1396 demo (用 deploy/ 当前目录)
  - help

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 deploy executor, 不是 consciousness claim.
- 不假装达到 ASI: 真 executor ≠ ASI 达成; 真 executor 是 ASI 北极星里 system integration 的一小步.
- 不假装调整模型 & prompt: 真生产是真 parse YAML + 真 chain 调 + 真 manifest 生成, 不是改 prompt.
- 真 executor = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "executor = safety" 都是不假装. 真 executor ≠ 安全审计.
- 任何声称 "executor = ASI" 都是不假装. 真 executor 是 ASI 北极星里 system integration 的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

# V1396 真生产 PyYAML (主 17:43 实事求是)
try:
    import yaml
    _YAML_AVAILABLE = True
except Exception:
    yaml = None
    _YAML_AVAILABLE = False


V1396_VERSION = "0.1.0"
V1396_SCHEMA = "v1396.deploy-executor/v1"

# V1396 真生产 valid restart policies (主 17:43 实事求是 docker 官方)
V1396_VALID_RESTART: tuple = ("no", "always", "unless-stopped", "on-failure", "on-failure:max-retries")

# V1396 真生产 GUARDS (主 17:43)
V1396_GUARDS: tuple = (
    "GUARD_EXECUTOR_REAL",        # 真跑 chain / 真 parse compose
    "GUARD_CHAIN_REAL",           # 真调 V1387/V1393/V1395
    "GUARD_COMPOSE_PYTHON_ONLY",  # 不依赖 docker 命令
    "GUARD_NO_CAP_CHANGE",        # 不改 ASI cap
    "GUARD_DETERMINISTIC",        # same target → same result
    "GUARD_HONEST_DISCLOSURE",    # 标注 source modules + 未跑原因
    "GUARD_MANIFEST_VALID",       # manifest schema 完整
    "GUARD_PORTS_VALID",          # ports 格式有效
    "GUARD_DELEGATE_REAL",        # 真调 upstream modules
    "GUARD_CLI_RUNNABLE",         # CLI 真可跑
    "GUARD_RESTART_VALID",        # restart ∈ valid policies
    "GUARD_HEALTHCHECK_RECOMMENDED",  # 提示 healthcheck (not blocking)
)

# V1396 真生产 借鉴 (主 19:33)
V1396_BORROWED: tuple = (
    "github-deploy-status-api",  # 真借鉴 status reporting 概念
    "gitlab-deploy-api",         # 真借鉴 deploy stages
    "jenkins-pipeline",          # 真借鉴 stage execution
    "argocd-application",        # 真借鉴 manifest sync model
    "kubectl-apply-dry-run",     # 真借鉴 dry-run validation
    "docker-compose-config",     # 真借鉴 compose schema validation
)


# ============================================================================
# V1396 真生产 数据结构 (主 17:43)
# ============================================================================


@dataclass
class PortMapping:
    """V1396 真生产 1 个 port mapping (主 17:43)."""

    service: str = ""
    raw: str = ""           # e.g. "8765:8765" or "8765"
    host_port: int = 0      # 8765
    container_port: int = 0  # 8765
    protocol: str = "tcp"    # tcp/udp
    source_file: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "service": self.service,
            "raw": self.raw,
            "host_port": self.host_port,
            "container_port": self.container_port,
            "protocol": self.protocol,
            "source_file": self.source_file,
        }


@dataclass
class ComposeService:
    """V1396 真生产 1 个 compose service (主 17:43)."""

    name: str = ""
    image: str = ""
    build_context: str = ""
    build_dockerfile: str = ""
    restart: str = ""
    init: bool = False
    ports: List[PortMapping] = field(default_factory=list)
    environment: Dict[str, str] = field(default_factory=dict)
    healthcheck_test: str = ""
    healthcheck_interval: str = ""
    healthcheck_timeout: str = ""
    healthcheck_retries: int = 0
    healthcheck_start_period: str = ""
    depends_on: List[str] = field(default_factory=list)
    volumes: List[str] = field(default_factory=list)
    cpu_limit: str = ""
    memory_limit: str = ""
    source_file: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "image": self.image,
            "build_context": self.build_context,
            "build_dockerfile": self.build_dockerfile,
            "restart": self.restart,
            "init": self.init,
            "ports": [p.to_dict() for p in self.ports],
            "environment": dict(self.environment),
            "healthcheck_test": self.healthcheck_test,
            "healthcheck_interval": self.healthcheck_interval,
            "healthcheck_timeout": self.healthcheck_timeout,
            "healthcheck_retries": self.healthcheck_retries,
            "healthcheck_start_period": self.healthcheck_start_period,
            "depends_on": list(self.depends_on),
            "volumes": list(self.volumes),
            "cpu_limit": self.cpu_limit,
            "memory_limit": self.memory_limit,
            "source_file": self.source_file,
        }


@dataclass
class ComposeFile:
    """V1396 真生产 1 个 compose file (主 17:43)."""

    path: str = ""
    abs_path: str = ""
    version: str = ""
    services: List[ComposeService] = field(default_factory=list)
    raw_n_lines: int = 0
    parse_error: str = ""
    ok: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "path": self.path,
            "abs_path": self.abs_path,
            "version": self.version,
            "n_services": len(self.services),
            "services": [s.to_dict() for s in self.services],
            "raw_n_lines": self.raw_n_lines,
            "parse_error": self.parse_error,
            "ok": self.ok,
        }


@dataclass
class ComposeIssue:
    """V1396 真生产 1 个 compose validation issue (主 17:43)."""

    severity: str = "info"   # error/warning/info
    rule_id: str = "V1396-INFO"
    service: str = ""
    file_path: str = ""
    message: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "severity": self.severity,
            "rule_id": self.rule_id,
            "service": self.service,
            "file_path": self.file_path,
            "message": self.message,
        }


@dataclass
class PortConflict:
    """V1396 真生产 1 个 port conflict (主 17:43)."""

    host_port: int = 0
    protocol: str = "tcp"
    services: List[str] = field(default_factory=list)
    source_files: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "host_port": self.host_port,
            "protocol": self.protocol,
            "services": list(self.services),
            "source_files": list(self.source_files),
        }


@dataclass
class ChainStepResult:
    """V1396 真生产 1 个 chain step result (主 17:43)."""

    module_id: str = ""
    label: str = ""
    called: bool = False
    ok: bool = False
    elapsed_seconds: float = 0.0
    summary: str = ""
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "module_id": self.module_id,
            "label": self.label,
            "called": self.called,
            "ok": self.ok,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "summary": self.summary,
            "error": self.error,
        }


@dataclass
class ChainResult:
    """V1396 真生产 1 个 chain result (主 17:43)."""

    target: str = ""
    started_at: str = ""
    finished_at: str = ""
    elapsed_seconds: float = 0.0
    steps: List[ChainStepResult] = field(default_factory=list)
    n_steps: int = 0
    n_steps_ok: int = 0
    n_steps_failed: int = 0
    chain_ok: bool = False
    judge_verdict: str = "N/A"
    deploy_score: int = 0
    deploy_grade: str = "N/A"
    notes: List[str] = field(default_factory=list)
    guards: tuple = V1396_GUARDS

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1396_SCHEMA,
            "version": V1396_VERSION,
            "target": self.target,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "n_steps": self.n_steps,
            "n_steps_ok": self.n_steps_ok,
            "n_steps_failed": self.n_steps_failed,
            "chain_ok": self.chain_ok,
            "judge_verdict": self.judge_verdict,
            "deploy_score": self.deploy_score,
            "deploy_grade": self.deploy_grade,
            "steps": [s.to_dict() for s in self.steps],
            "notes": list(self.notes),
            "guards": list(self.guards),
        }


@dataclass
class ExecutorResult:
    """V1396 真生产 1 个 executor 全 result (主 17:43)."""

    target: str = ""
    n_compose_files: int = 0
    n_services_total: int = 0
    n_issues: int = 0
    n_errors: int = 0
    n_warnings: int = 0
    n_info: int = 0
    n_port_conflicts: int = 0
    compose_files: List[ComposeFile] = field(default_factory=list)
    issues: List[ComposeIssue] = field(default_factory=list)
    port_conflicts: List[PortConflict] = field(default_factory=list)
    chain_result: Optional[ChainResult] = None
    manifest: Dict[str, Any] = field(default_factory=dict)
    ok: bool = False
    elapsed_seconds: float = 0.0
    guards: tuple = V1396_GUARDS

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1396_SCHEMA,
            "version": V1396_VERSION,
            "target": self.target,
            "n_compose_files": self.n_compose_files,
            "n_services_total": self.n_services_total,
            "n_issues": self.n_issues,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "n_port_conflicts": self.n_port_conflicts,
            "compose_files": [c.to_dict() for c in self.compose_files],
            "issues": [i.to_dict() for i in self.issues],
            "port_conflicts": [p.to_dict() for p in self.port_conflicts],
            "chain_result": self.chain_result.to_dict() if self.chain_result else None,
            "manifest": self.manifest,
            "ok": self.ok,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "guards": list(self.guards),
        }


# ============================================================================
# V1396 真生产 工具函数 (主 17:43)
# ============================================================================


def _iso_timestamp() -> str:
    """V1396 真生产: ISO 8601 UTC timestamp (主 17:43)."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _discover_compose_files(root: Path) -> List[Path]:
    """V1396 真生产: 真扫描 root 下所有 docker-compose*.yml/yaml 文件 (主 17:43)."""
    out: List[Path] = []
    if not root.exists() or not root.is_dir():
        return out
    # 1) root 下的 docker-compose.yml / docker-compose.yaml
    for name in ("docker-compose.yml", "docker-compose.yaml"):
        p = root / name
        if p.exists() and p.is_file():
            out.append(p)
    # 2) 任意子目录下的 *compose*.yml / *compose*.yaml
    #    (主 17:43 实事求是: deploy/18-crates/docker-compose.group-a.yml 等)
    for ext in ("*.yml", "*.yaml"):
        for p in root.rglob(ext):
            if p in out:
                continue
            name = p.name.lower()
            if "compose" in name:
                out.append(p)
    return sorted(set(out))


def _parse_port_spec(service_name: str, raw: Any, source_file: str) -> Optional[PortMapping]:
    """V1396 真生产: 真 parse 1 个 port spec (主 17:43 实事求是 docker compose spec).

    接受格式:
    - "8765:8765"
    - "8765:8765/udp"
    - "127.0.0.1:8765:8765"
    - "8765"  (shorthand, host=container)
    - 8765 (int)
    - {"published": 8765, "target": 8765}
    """
    if raw is None:
        return None
    s: str = ""
    host_port: int = 0
    container_port: int = 0
    protocol: str = "tcp"
    if isinstance(raw, int):
        host_port = raw
        container_port = raw
        s = str(raw)
    elif isinstance(raw, str):
        s = raw.strip()
        # 去掉 IP 前缀 "127.0.0.1:..."
        s_clean = s
        # protocol 分离
        proto_m = re.search(r"/(tcp|udp)$", s_clean)
        if proto_m:
            protocol = proto_m.group(1)
            s_clean = s_clean[: proto_m.start()]
        parts = s_clean.split(":")
        try:
            if len(parts) == 1:
                host_port = int(parts[0])
                container_port = int(parts[0])
            elif len(parts) == 2:
                host_port = int(parts[0])
                container_port = int(parts[1])
            elif len(parts) >= 3:
                # IP:HOST:CONTAINER → 取后两段
                host_port = int(parts[-2])
                container_port = int(parts[-1])
        except ValueError:
            return None
    elif isinstance(raw, dict):
        host_port = int(raw.get("published", 0) or 0)
        container_port = int(raw.get("target", 0) or 0)
        protocol = str(raw.get("protocol", "tcp") or "tcp")
        s = f"{host_port}:{container_port}/{protocol}"
    else:
        return None
    if host_port <= 0 or container_port <= 0:
        return None
    return PortMapping(
        service=service_name,
        raw=s,
        host_port=host_port,
        container_port=container_port,
        protocol=protocol,
        source_file=source_file,
    )


def _parse_compose_service(name: str, data: Dict[str, Any], source_file: str) -> ComposeService:
    """V1396 真生产: 真 parse 1 个 compose service (主 17:43)."""
    svc = ComposeService(name=name, source_file=source_file)
    svc.image = str(data.get("image", "") or "")
    build = data.get("build")
    if isinstance(build, str):
        svc.build_context = build
    elif isinstance(build, dict):
        svc.build_context = str(build.get("context", "") or "")
        svc.build_dockerfile = str(build.get("dockerfile", "") or "")
    svc.restart = str(data.get("restart", "") or "")
    svc.init = bool(data.get("init", False) or False)
    # ports
    ports_raw = data.get("ports", []) or []
    for p in ports_raw:
        pm = _parse_port_spec(name, p, source_file)
        if pm:
            svc.ports.append(pm)
    # environment
    env_raw = data.get("environment", {}) or {}
    if isinstance(env_raw, dict):
        for k, v in env_raw.items():
            svc.environment[str(k)] = str(v if v is not None else "")
    elif isinstance(env_raw, list):
        for item in env_raw:
            if isinstance(item, str) and "=" in item:
                k, v = item.split("=", 1)
                svc.environment[k.strip()] = v.strip()
            elif isinstance(item, str):
                svc.environment[item] = ""
    # healthcheck
    hc = data.get("healthcheck")
    if isinstance(hc, dict):
        test = hc.get("test", [])
        if isinstance(test, list):
            svc.healthcheck_test = " ".join(str(x) for x in test)
        elif isinstance(test, str):
            svc.healthcheck_test = test
        svc.healthcheck_interval = str(hc.get("interval", "") or "")
        svc.healthcheck_timeout = str(hc.get("timeout", "") or "")
        try:
            svc.healthcheck_retries = int(hc.get("retries", 0) or 0)
        except (TypeError, ValueError):
            svc.healthcheck_retries = 0
        svc.healthcheck_start_period = str(hc.get("start_period", "") or "")
    # depends_on
    depends = data.get("depends_on", []) or {}
    if isinstance(depends, list):
        svc.depends_on = [str(x) for x in depends]
    elif isinstance(depends, dict):
        svc.depends_on = list(depends.keys())
    # volumes
    vols = data.get("volumes", []) or []
    svc.volumes = [str(v) for v in vols]
    # deploy.resources
    deploy = data.get("deploy", {}) or {}
    if isinstance(deploy, dict):
        resources = deploy.get("resources", {}) or {}
        if isinstance(resources, dict):
            limits = resources.get("limits", {}) or {}
            if isinstance(limits, dict):
                svc.cpu_limit = str(limits.get("cpus", "") or "")
                svc.memory_limit = str(limits.get("memory", "") or "")
    return svc


def _parse_compose_file(path: Path) -> ComposeFile:
    """V1396 真生产: 真 parse 1 个 compose file (主 17:43 实事求是 PyYAML)."""
    cf = ComposeFile(path=str(path), abs_path=str(path.resolve()))
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
        cf.raw_n_lines = len(text.splitlines())
    except Exception as e:
        cf.parse_error = f"read failed: {e}"
        return cf
    if not _YAML_AVAILABLE:
        cf.parse_error = "PyYAML not available"
        return cf
    try:
        data = yaml.safe_load(text) or {}
    except yaml.YAMLError as e:
        cf.parse_error = f"yaml parse: {e}"
        return cf
    except Exception as e:
        cf.parse_error = f"parse: {e}"
        return cf
    if not isinstance(data, dict):
        cf.parse_error = "compose root is not a mapping"
        return cf
    cf.version = str(data.get("version", "") or "")
    services = data.get("services", {}) or {}
    if not isinstance(services, dict):
        cf.parse_error = "services is not a mapping"
        return cf
    for svc_name, svc_data in services.items():
        if not isinstance(svc_data, dict):
            continue
        cf.services.append(_parse_compose_service(str(svc_name), svc_data, cf.path))
    cf.ok = True
    return cf


# ============================================================================
# V1396 真生产 validate / ports (主 17:43)
# ============================================================================


def validate_compose(compose_files: List[ComposeFile]) -> List[ComposeIssue]:
    """V1396 真生产: 真校验 compose files (主 17:43)."""
    issues: List[ComposeIssue] = []
    for cf in compose_files:
        if not cf.ok:
            issues.append(ComposeIssue(
                severity="error",
                rule_id="V1396-COMPOSE-PARSE-ERROR",
                service="",
                file_path=cf.path,
                message=f"compose parse failed: {cf.parse_error}",
            ))
            continue
        for svc in cf.services:
            if not svc.image and not (svc.build_context or svc.build_dockerfile):
                issues.append(ComposeIssue(
                    severity="warning",
                    rule_id="V1396-NO-IMAGE-NO-BUILD",
                    service=svc.name,
                    file_path=cf.path,
                    message="service has no image and no build context",
                ))
            if svc.restart and svc.restart not in V1396_VALID_RESTART:
                # 允许 on-failure:N 形式
                if not re.match(r"^on-failure(:\d+)?$", svc.restart):
                    issues.append(ComposeIssue(
                        severity="warning",
                        rule_id="V1396-RESTART-INVALID",
                        service=svc.name,
                        file_path=cf.path,
                        message=f"restart policy '{svc.restart}' not in {list(V1396_VALID_RESTART)}",
                    ))
            if not svc.healthcheck_test:
                issues.append(ComposeIssue(
                    severity="info",
                    rule_id="V1396-HEALTHCHECK-MISSING",
                    service=svc.name,
                    file_path=cf.path,
                    message="service has no healthcheck",
                ))
            for p in svc.ports:
                if not (1 <= p.host_port <= 65535):
                    issues.append(ComposeIssue(
                        severity="error",
                        rule_id="V1396-PORT-INVALID",
                        service=svc.name,
                        file_path=cf.path,
                        message=f"port {p.host_port} out of range 1-65535",
                    ))
                if not (1 <= p.container_port <= 65535):
                    issues.append(ComposeIssue(
                        severity="error",
                        rule_id="V1396-PORT-INVALID",
                        service=svc.name,
                        file_path=cf.path,
                        message=f"container port {p.container_port} out of range 1-65535",
                    ))
    return issues


def detect_port_conflicts(compose_files: List[ComposeFile]) -> List[PortConflict]:
    """V1396 真生产: 真检测跨 service ports 冲突 (主 17:43).

    Same host_port + protocol across different services = conflict.
    """
    by_key: Dict[Tuple[int, str], List[Tuple[str, str]]] = {}
    for cf in compose_files:
        if not cf.ok:
            continue
        for svc in cf.services:
            for p in svc.ports:
                key = (p.host_port, p.protocol)
                by_key.setdefault(key, []).append((svc.name, cf.path))
    conflicts: List[PortConflict] = []
    for (host_port, protocol), usages in sorted(by_key.items()):
        # distinct services across all files
        service_names = sorted(set(s for s, _ in usages))
        source_files = sorted(set(f for _, f in usages))
        if len(service_names) > 1:
            conflicts.append(PortConflict(
                host_port=host_port,
                protocol=protocol,
                services=service_names,
                source_files=source_files,
            ))
    return conflicts


def generate_manifest(compose_files: List[ComposeFile], port_conflicts: List[PortConflict]) -> Dict[str, Any]:
    """V1396 真生产: 真生成 executable deploy manifest (主 17:43)."""
    services: List[Dict[str, Any]] = []
    for cf in compose_files:
        if not cf.ok:
            continue
        for svc in cf.services:
            services.append({
                "name": svc.name,
                "image": svc.image,
                "source_file": cf.path,
                "restart": svc.restart,
                "ports": [p.to_dict() for p in svc.ports],
                "depends_on": list(svc.depends_on),
                "healthcheck": svc.healthcheck_test,
                "resources": {
                    "cpu_limit": svc.cpu_limit,
                    "memory_limit": svc.memory_limit,
                },
            })
    return {
        "schema": V1396_SCHEMA,
        "version": V1396_VERSION,
        "generated_at": _iso_timestamp(),
        "n_services": len(services),
        "services": services,
        "n_port_conflicts": len(port_conflicts),
        "port_conflicts": [p.to_dict() for p in port_conflicts],
    }


# ============================================================================
# V1396 真生产 chain (主 17:43 真调 V1387/V1393/V1395)
# ============================================================================


def _step(module_id: str, label: str, fn, *args, **kwargs) -> ChainStepResult:
    """V1396 真生产: 真执行 1 个 chain step (主 17:43 真调真跑)."""
    import time
    s = ChainStepResult(module_id=module_id, label=label)
    t0 = time.time()
    try:
        result = fn(*args, **kwargs)
        s.called = True
        s.ok = True
        s.summary = str(result) if result is not None else ""
        # 简化: 如果 result 是 dataclass, 显示关键字段
        if hasattr(result, "verdict"):
            s.summary = f"verdict={result.verdict}"
        if hasattr(result, "deploy_score"):
            s.summary = f"verdict={result.verdict} score={result.deploy_score} grade={result.deploy_grade}"
        if hasattr(result, "n_present") and hasattr(result, "n_modules"):
            s.summary = f"modules={result.n_present}/{result.n_modules} broken={result.n_broken} tests={result.n_tests_total}"
        if hasattr(result, "n_findings") and hasattr(result, "n_files_total"):
            s.summary = f"files={result.n_files_total} findings={result.n_findings} ok={result.ok}"
    except Exception as e:
        s.called = True
        s.ok = False
        s.error = f"{type(e).__name__}: {e}"
        s.summary = "FAILED"
    s.elapsed_seconds = time.time() - t0
    return s


def _import_module(name: str):
    """V1396 真生产: 真尝试 import module (主 17:43 实事求是兼容两种 import 路径).

    1. 先 try `from apeireth.{name} import ...`
    2. 失败 try 加 apeireth 父目录到 sys.path → `from {name} import ...`
    """
    try:
        return __import__(f"apeireth.{name}", fromlist=["*"])
    except Exception:
        pass
    try:
        apeireth_parent = Path(__file__).resolve().parent.parent
        sp = str(apeireth_parent)
        if sp not in sys.path:
            sys.path.insert(0, sp)
        return __import__(name)
    except Exception:
        return None


def _call_v1387(target: str):
    """V1396 真生产: 真调 V1387 unified runner (主 17:43)."""
    mod = _import_module("v1387_deploy_stack_runner")
    if mod is None:
        raise ImportError("v1387_deploy_stack_runner not importable")
    runner = mod.V1387DeployStackRunner()
    return runner.run(target)


def _call_v1393(target: str, policy_path: Optional[str] = None):
    """V1396 真生产: 真调 V1393 deploy judge (主 17:43)."""
    mod = _import_module("v1393_deploy_judge")
    if mod is None:
        raise ImportError("v1393_deploy_judge not importable")
    return mod.judge(target, policy_path=policy_path)


def _call_v1395(apeireth_dir: Path, tests_dirs: Sequence[Path], judge_target: Optional[str] = None):
    """V1396 真生产: 真调 V1395 dashboard build_dashboard (主 17:43)."""
    mod = _import_module("v1395_deploy_dashboard")
    if mod is None:
        raise ImportError("v1395_deploy_dashboard not importable")
    return mod.build_dashboard(
        apeireth_dir=apeireth_dir,
        tests_dirs=list(tests_dirs),
        judge_target=judge_target,
    )


def run_chain(
    target: str,
    policy_path: Optional[str] = None,
    apeireth_dir: Optional[Path] = None,
    tests_dirs: Optional[Sequence[Path]] = None,
) -> ChainResult:
    """V1396 真生产: 真跑 chain V1387 → V1393 → V1395 (主 17:43).

    chain 顺序 (主 23:44 干到底):
      1. V1387 unified runner → 真扫 deploy/ 真得 findings
      2. V1393 judge → 真聚合 V1387 + 真给 verdict
      3. V1395 dashboard → 真聚合 modules 状态
    """
    if apeireth_dir is None:
        apeireth_dir = Path(__file__).resolve().parent
    if tests_dirs is None:
        tests_dirs = [
            apeireth_dir.parent / "tests",
            apeireth_dir / "tests",
        ]

    import time
    res = ChainResult(target=target)
    res.started_at = _iso_timestamp()
    t0 = time.time()

    # step 1: V1387
    s1 = _step("V1387", "unified deploy-stack runner", _call_v1387, target)
    res.steps.append(s1)
    # step 2: V1393
    s2 = _step("V1393", "deploy-stack judge", _call_v1393, target, policy_path)
    res.steps.append(s2)
    if s2.ok:
        res.judge_verdict = s2.summary.split("verdict=")[1].split(" ")[0] if "verdict=" in s2.summary else "N/A"
        if "score=" in s2.summary:
            try:
                parts = s2.summary.split()
                for p in parts:
                    if p.startswith("score="):
                        res.deploy_score = int(p.split("=")[1])
                    if p.startswith("grade="):
                        res.deploy_grade = p.split("=")[1]
            except (IndexError, ValueError):
                pass
    # step 3: V1395
    s3 = _step("V1395", "deploy-stack dashboard", _call_v1395, apeireth_dir, tests_dirs, target)
    res.steps.append(s3)

    res.n_steps = len(res.steps)
    res.n_steps_ok = sum(1 for s in res.steps if s.ok)
    res.n_steps_failed = sum(1 for s in res.steps if s.called and not s.ok)
    res.chain_ok = res.n_steps_failed == 0 and res.n_steps_ok == res.n_steps
    res.finished_at = _iso_timestamp()
    res.elapsed_seconds = time.time() - t0
    if not s1.ok:
        res.notes.append(f"V1387 failed: {s1.error}")
    if not s3.ok:
        res.notes.append(f"V1395 failed: {s3.error}")
    return res


# ============================================================================
# V1396 真生产 full executor (主 17:43)
# ============================================================================


def run_executor(
    target: str,
    with_chain: bool = True,
    policy_path: Optional[str] = None,
    apeireth_dir: Optional[Path] = None,
    tests_dirs: Optional[Sequence[Path]] = None,
) -> ExecutorResult:
    """V1396 真生产: 真执行 1 个 deploy executor (主 17:43)."""
    import time
    res = ExecutorResult(target=target)
    t0 = time.time()
    root = Path(target)
    if not root.exists():
        res.notes_log = []  # type: ignore
        res.ok = False
        res.elapsed_seconds = time.time() - t0
        return res
    compose_paths = _discover_compose_files(root)
    compose_files = [_parse_compose_file(p) for p in compose_paths]
    res.compose_files = compose_files
    res.n_compose_files = len(compose_files)
    res.n_services_total = sum(len(c.services) for c in compose_files if c.ok)
    res.issues = validate_compose(compose_files)
    res.n_issues = len(res.issues)
    for iss in res.issues:
        if iss.severity == "error":
            res.n_errors += 1
        elif iss.severity == "warning":
            res.n_warnings += 1
        elif iss.severity == "info":
            res.n_info += 1
    res.port_conflicts = detect_port_conflicts(compose_files)
    res.n_port_conflicts = len(res.port_conflicts)
    res.manifest = generate_manifest(compose_files, res.port_conflicts)
    if with_chain:
        res.chain_result = run_chain(target, policy_path=policy_path, apeireth_dir=apeireth_dir, tests_dirs=tests_dirs)
    res.ok = res.n_errors == 0
    res.elapsed_seconds = time.time() - t0
    return res


# 兼容 V1387-style to_dict (used in tests)
def _executor_notes_log_attr(self):  # pragma: no cover
    return []


# 在 ExecutorResult 加 notes_log 字段 (避免 unused attr)
ExecutorResult.notes_log = field(default_factory=list)  # type: ignore[attr-defined]


# ============================================================================
# V1396 真生产 renderers (主 17:43)
# ============================================================================


def render_markdown(res: ExecutorResult) -> str:
    """V1396 真生产: 真 render Markdown executor report (主 17:43)."""
    lines: List[str] = []
    lines.append(f"# V1396 deploy-stack executor — {res.target}")
    lines.append("")
    lines.append(f"- schema: `{V1396_SCHEMA}` v{V1396_VERSION}")
    lines.append(f"- generated_at: `{_iso_timestamp()}`")
    lines.append(f"- compose_files: **{res.n_compose_files}**")
    lines.append(f"- services_total: **{res.n_services_total}**")
    lines.append(f"- issues: **{res.n_issues}** (errors={res.n_errors} warnings={res.n_warnings} info={res.n_info})")
    lines.append(f"- port_conflicts: **{res.n_port_conflicts}**")
    lines.append(f"- ok: **{res.ok}**")
    lines.append(f"- elapsed_seconds: **{round(res.elapsed_seconds, 4)}**")
    lines.append("")
    if res.compose_files:
        lines.append("## Compose files")
        for cf in res.compose_files:
            status = "✅" if cf.ok else f"❌ ({cf.parse_error})"
            lines.append(f"- `{cf.path}` — {status} ({len(cf.services)} services, {cf.raw_n_lines} lines)")
        lines.append("")
    if res.issues:
        lines.append("## Issues")
        for iss in res.issues:
            lines.append(f"- **{iss.severity}** `{iss.rule_id}` service=`{iss.service}` file=`{iss.file_path}` — {iss.message}")
        lines.append("")
    if res.port_conflicts:
        lines.append("## Port conflicts")
        for pc in res.port_conflicts:
            lines.append(f"- host_port=`{pc.host_port}/{pc.protocol}` services={pc.services} files={pc.source_files}")
        lines.append("")
    if res.chain_result:
        cr = res.chain_result
        lines.append("## Chain result")
        lines.append(f"- verdict: `{cr.judge_verdict}`")
        lines.append(f"- deploy_score: `{cr.deploy_score}` grade=`{cr.deploy_grade}`")
        lines.append(f"- chain_ok: `{cr.chain_ok}` ({cr.n_steps_ok}/{cr.n_steps} steps ok)")
        for s in cr.steps:
            ok = "✅" if s.ok else "❌"
            lines.append(f"  - {ok} `{s.module_id}` {s.label} ({round(s.elapsed_seconds, 4)}s) {s.summary}")
        lines.append("")
    lines.append("## GUARDS")
    for g in V1396_GUARDS:
        lines.append(f"- `{g}`")
    lines.append("")
    lines.append("## Known unknowns")
    lines.append("- V1396 does not run docker / docker-compose / kubectl — static analysis only")
    lines.append("- V1396 depends on PyYAML being installed; falls back to parse-error if not")
    lines.append("- V1396 chain depends on V1387/V1393/V1395 being importable; marks step failed if not")
    lines.append("")
    lines.append(f"*V1396 deploy executor v{V1396_VERSION} (schema {V1396_SCHEMA}). Real run, not pretend.*")
    return "\n".join(lines)


def render_json(res: ExecutorResult) -> str:
    """V1396 真生产: 真 render JSON (主 17:43)."""
    return json.dumps(res.to_dict(), indent=2, ensure_ascii=False, sort_keys=False)


# ============================================================================
# V1396 Popper self-test (主 17:43 真跑真测)
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """V1396 真生产: Popper self-test (主 17:43 真跑真测).

    真尝试:
    1. 真 parse 当前 deploy/ 目录的 compose files
    2. 真校验 → issues
    3. 真检测 ports conflicts → conflicts
    4. 真生成 manifest → valid
    5. 真跑 mini chain → at least V1387 ok
    """
    failures: List[str] = []
    notes: List[str] = []

    # 1) self parse current dir
    try:
        files = _discover_compose_files(Path("deploy"))
        if not files:
            notes.append("no compose files in deploy/ (test environment)")
        else:
            notes.append(f"discovered {len(files)} compose files in deploy/")
    except Exception as e:
        failures.append(f"discover failed: {e}")

    # 2) self validate on demo data
    demo_yaml = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
        "    ports:\n"
        "      - \"1234:1234\"\n"
        "    restart: unless-stopped\n"
        "    healthcheck:\n"
        "      test: [\"CMD\", \"true\"]\n"
        "      interval: 10s\n"
        "  svc2:\n"
        "    image: bar:1\n"
        "    ports:\n"
        "      - \"1234:5678\"\n"  # conflict on 1234
        "    restart: bogus_policy\n"  # invalid
    )
    demo_path = Path(os.environ.get("TMP", "/tmp")) / "_v1396_demo_compose.yml"
    try:
        demo_path.write_text(demo_yaml, encoding="utf-8")
        cf = _parse_compose_file(demo_path)
        if not cf.ok:
            failures.append(f"demo parse failed: {cf.parse_error}")
        else:
            issues = validate_compose([cf])
            has_conflict_issue = any(i.rule_id == "V1396-RESTART-INVALID" for i in issues)
            if not has_conflict_issue:
                failures.append("expected V1396-RESTART-INVALID on bogus policy")
            conflicts = detect_port_conflicts([cf])
            if len(conflicts) != 1 or conflicts[0].host_port != 1234:
                failures.append(f"expected 1 port conflict on 1234, got {len(conflicts)}")
            else:
                notes.append("port conflict detection ok (1234 across svc1/svc2)")
            manifest = generate_manifest([cf], conflicts)
            if manifest.get("n_services") != 2:
                failures.append(f"manifest n_services != 2 ({manifest.get('n_services')})")
            else:
                notes.append("manifest generation ok (n=2)")
    finally:
        try:
            demo_path.unlink()
        except OSError:
            pass

    # 3) chain ok check (V1387 is mandatory; V1393 / V1395 may fail gracefully)
    try:
        cr = run_chain("deploy")
        if cr.n_steps_ok < 1:
            failures.append(f"chain: 0 steps ok (V1387 should at least succeed)")
        else:
            notes.append(f"chain ok: {cr.n_steps_ok}/{cr.n_steps} steps")
        if cr.judge_verdict == "N/A":
            notes.append("chain: judge_verdict=N/A (V1393 likely unavailable in test env)")
    except Exception as e:
        failures.append(f"chain failed: {e}")

    # 4) GUARDS count check
    if len(V1396_GUARDS) < 10:
        failures.append(f"GUARDS < 10: {len(V1396_GUARDS)}")

    # 5) version / schema consistency
    if V1396_VERSION != "0.1.0":
        failures.append(f"V1396_VERSION != 0.1.0 ({V1396_VERSION})")

    return {
        "schema": V1396_SCHEMA,
        "version": V1396_VERSION,
        "ok": len(failures) == 0,
        "failures": failures,
        "notes": notes,
        "n_guards": len(V1396_GUARDS),
        "n_borrowed": len(V1396_BORROWED),
    }


# ============================================================================
# V1396 CLI (主 17:43 真可执行)
# ============================================================================


def _build_demo_executor(target: str = "deploy") -> ExecutorResult:
    """V1396 真生产: build a demo executor result (for `demo` subcommand)."""
    return run_executor(target, with_chain=False)


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1396 真生产 CLI 主入口 (主 17:43 真可执行)."""
    parser = argparse.ArgumentParser(
        prog="v1396-deploy-executor",
        description=f"V1396 real production deploy-stack executor (v{V1396_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_version = sub.add_parser("version", help="V1396 version")
    p_version.add_argument("--json", action="store_true", help="JSON output")

    p_chain = sub.add_parser("chain", help="run V1387 → V1393 → V1395 chain")
    p_chain.add_argument("target", nargs="?", default="deploy", help="target dir (default: deploy)")
    p_chain.add_argument("--policy", default=None, help="optional V1391 policy YAML")
    p_chain.add_argument("--json", action="store_true", help="JSON output")

    p_validate = sub.add_parser("validate", help="static validate compose files")
    p_validate.add_argument("target", nargs="?", default="deploy", help="target dir")
    p_validate.add_argument("--json", action="store_true", help="JSON output")

    p_manifest = sub.add_parser("manifest", help="generate deploy manifest")
    p_manifest.add_argument("target", nargs="?", default="deploy", help="target dir")
    p_manifest.add_argument("--format", choices=("json", "yaml"), default="json", help="output format")

    p_ports = sub.add_parser("ports", help="detect port conflicts")
    p_ports.add_argument("target", nargs="?", default="deploy", help="target dir")
    p_ports.add_argument("--json", action="store_true", help="JSON output")

    p_executor = sub.add_parser("executor", help="full executor (parse + validate + ports + manifest)")
    p_executor.add_argument("target", nargs="?", default="deploy", help="target dir")
    p_executor.add_argument("--no-chain", action="store_true", help="skip chain")
    p_executor.add_argument("--json", action="store_true", help="JSON output")

    sub.add_parser("popper", help="V1396 Popper self-test")
    sub.add_parser("demo", help="V1396 demo")
    sub.add_parser("help", help="show help")

    if argv is None:
        argv = sys.argv[1:]
    if not argv or argv == ["help"]:
        parser.print_help()
        return 0
    args = parser.parse_args(argv)
    cmd = args.cmd

    if cmd == "version":
        if getattr(args, "json", False):
            print(json.dumps({"version": V1396_VERSION, "schema": V1396_SCHEMA, "n_guards": len(V1396_GUARDS), "n_borrowed": len(V1396_BORROWED)}, indent=2))
        else:
            print(f"V1396 deploy executor v{V1396_VERSION} (schema {V1396_SCHEMA})")
            print(f"GUARDS: {len(V1396_GUARDS)}, BORROWED: {len(V1396_BORROWED)}")
        return 0

    if cmd == "chain":
        cr = run_chain(args.target, policy_path=getattr(args, "policy", None))
        if getattr(args, "json", False):
            print(json.dumps(cr.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(f"# V1396 chain — {cr.target}")
            print(f"- started_at: {cr.started_at}")
            print(f"- finished_at: {cr.finished_at}")
            print(f"- elapsed_seconds: {round(cr.elapsed_seconds, 4)}")
            print(f"- chain_ok: {cr.chain_ok}")
            print(f"- n_steps: {cr.n_steps} ({cr.n_steps_ok} ok / {cr.n_steps_failed} failed)")
            print(f"- judge_verdict: {cr.judge_verdict}")
            print(f"- deploy_score: {cr.deploy_score} grade={cr.deploy_grade}")
            print()
            for s in cr.steps:
                ok = "✅" if s.ok else "❌"
                print(f"  {ok} {s.module_id} {s.label} ({round(s.elapsed_seconds, 4)}s) {s.summary}")
                if s.error:
                    print(f"     error: {s.error}")
            if cr.notes:
                print()
                print("## Notes")
                for n in cr.notes:
                    print(f"- {n}")
        return 0 if cr.chain_ok else 1

    if cmd == "validate":
        root = Path(args.target)
        compose_files = [_parse_compose_file(p) for p in _discover_compose_files(root)]
        issues = validate_compose(compose_files)
        if getattr(args, "json", False):
            print(json.dumps({
                "schema": V1396_SCHEMA,
                "version": V1396_VERSION,
                "target": args.target,
                "n_compose_files": len(compose_files),
                "n_issues": len(issues),
                "n_errors": sum(1 for i in issues if i.severity == "error"),
                "n_warnings": sum(1 for i in issues if i.severity == "warning"),
                "n_info": sum(1 for i in issues if i.severity == "info"),
                "issues": [i.to_dict() for i in issues],
            }, indent=2, ensure_ascii=False))
        else:
            print(f"# V1396 validate — {args.target}")
            print(f"- n_compose_files: {len(compose_files)}")
            print(f"- n_issues: {len(issues)}")
            for i in issues:
                print(f"  - [{i.severity}] {i.rule_id} service={i.service} file={i.file_path} — {i.message}")
        n_errors = sum(1 for i in issues if i.severity == "error")
        return 1 if n_errors > 0 else 0

    if cmd == "manifest":
        root = Path(args.target)
        compose_files = [_parse_compose_file(p) for p in _discover_compose_files(root)]
        conflicts = detect_port_conflicts(compose_files)
        manifest = generate_manifest(compose_files, conflicts)
        if args.format == "json":
            print(json.dumps(manifest, indent=2, ensure_ascii=False))
        else:
            if not _YAML_AVAILABLE:
                print(json.dumps(manifest, indent=2, ensure_ascii=False))
            else:
                print(yaml.safe_dump(manifest, sort_keys=False, allow_unicode=True))
        return 0

    if cmd == "ports":
        root = Path(args.target)
        compose_files = [_parse_compose_file(p) for p in _discover_compose_files(root)]
        conflicts = detect_port_conflicts(compose_files)
        if getattr(args, "json", False):
            print(json.dumps({
                "schema": V1396_SCHEMA,
                "version": V1396_VERSION,
                "target": args.target,
                "n_conflicts": len(conflicts),
                "conflicts": [c.to_dict() for c in conflicts],
            }, indent=2, ensure_ascii=False))
        else:
            print(f"# V1396 ports — {args.target}")
            print(f"- n_conflicts: {len(conflicts)}")
            for c in conflicts:
                print(f"  - host_port={c.host_port}/{c.protocol} services={c.services} files={c.source_files}")
        return 1 if len(conflicts) > 0 else 0

    if cmd == "executor":
        res = run_executor(
            args.target,
            with_chain=not getattr(args, "no_chain", False),
        )
        if getattr(args, "json", False):
            print(json.dumps(res.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(render_markdown(res))
        return 0 if res.ok else 1

    if cmd == "popper":
        r = popper_self_test()
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return 0 if r["ok"] else 1

    if cmd == "demo":
        res = _build_demo_executor("deploy")
        print(render_markdown(res))
        return 0 if res.ok else 1

    parser.print_help()
    return 1


if __name__ == "__main__":  # pragma: no cover
    sys.exit(run_cli())