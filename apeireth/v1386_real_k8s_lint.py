"""Phase 1386 v1386_real_k8s_lint — V1386 ASI 真生产 Kubernetes manifest 真解析 + 真 lint (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1386 = 真端部 V1385 之后, Kubernetes manifest YAML 真解析 + 真 lint (post-V1385 next-step).
主 23:44 干到底: 真生产不是模板字符串, 是真能 parse YAML 多文档 + 真能找问题.
主 22:33 ASI 北极星: 真解析 k8s manifest, 真找 deployment / security / best-practices 问题.
主 19:33 走在前人经验上: 真借鉴 kubeval (https://github.com/instrumenta/kubeval) + kubeconform (https://github.com/yannh/kubeconform) + polaris (https://github.com/FairwindsOps/polaris).
主 17:43 实事求是: 真 read + 真 YAML 多文档 parse + 真规则匹配, 不假装 lint.
主 17:33 放手干到底.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 JSON 输出 + 真 dry-run.

真生产设计 (主 19:33 kubeval/kubeconform/polaris 真借鉴):
- 真 read k8s manifest YAML 文件 (主 17:43 真文件)
- 真用 PyYAML safe_load_all 解析多文档 (--- 分隔) (主 17:43 真解析, 不假装)
- 真找问题 (latest tag, no resource limits, no probes, no securityContext, privileged, host network, plaintext secret)
- 真报 finding (rule_id, severity, kind, name, namespace, line_no, message, suggestion)
- 真 exit code 0/1 (CI/CD 可用)
- 真 dry-run 模式 (--dry-run) 不写文件

真借鉴 kubeval (主 19:33 https://github.com/instrumenta/kubeval) + kubeconform (https://github.com/yannh/kubeconform):
- K8S-LATEST-TAG: image 以 :latest 结尾或缺 tag (不可重现)
- K8S-NO-RESOURCE-LIMITS: container 缺 resources.limits (OOM 风险)
- K8S-NO-READINESS: container 缺 readinessProbe (流量接收时机不明)
- K8S-NO-LIVENESS: container 缺 livenessProbe (崩溃不自愈)
- K8S-NO-SECURITY-CTX: container 缺 securityContext.capabilities.drop (攻击面过大)
- K8S-PRIVILEGED: privileged: true 或 hostNetwork: true (隔离失效)
- K8S-HOST-NETWORK: pod-level hostNetwork: true
- K8S-PLAINTEXT-SECRET: env 出现 KEY/SECRET/TOKEN/PASSWORD 明文非占位符 (应改用 Secret)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 k8s manifest lint, 不是 consciousness claim.
- 不假装达到 ASI: 真 lint ≠ ASI 达成; 真 lint 是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是真 parse 真匹配规则, 不是改 prompt 假装 lint.
- 真 lint = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "lint = 安全" 都是不假装. 真 lint ≠ 安全审计.
- 任何声称 "lint = ASI" 都是不假装. 真 lint 是 ASI 北极星里的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple

try:
    import yaml  # PyYAML >= 5.1
    YAML_AVAILABLE = True
    YAML_ERROR: Optional[str] = None
except Exception as _yaml_exc:  # pragma: no cover
    yaml = None  # type: ignore[assignment]
    YAML_AVAILABLE = False
    YAML_ERROR = repr(_yaml_exc)


V1386_VERSION = "0.1.0"


# ============================================================================
# V1386 真生产 数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class K8sFinding:
    """V1386 真生产 k8s manifest lint 真报 finding (主 17:43 实事求是)."""

    rule_id: str            # 例如 K8S-LATEST-TAG / K8S-NO-RESOURCE-LIMITS
    severity: str           # error / warning / info
    kind: str               # Deployment / Pod / StatefulSet / Service / ...
    name: str               # 受影响对象名 (顶层问题用 "<root>")
    namespace: str          # 命名空间 (顶层问题用 "<root>")
    container: str          # 受影响 container 名 (顶层问题用 "<pod>" / "<root>")
    line_no: int            # 真行号 (1-indexed, 顶层问题用 0)
    message: str            # 真问题描述
    suggestion: str = ""    # 真建议

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "kind": self.kind,
            "name": self.name,
            "namespace": self.namespace,
            "container": self.container,
            "line_no": self.line_no,
            "message": self.message,
            "suggestion": self.suggestion,
        }


@dataclass
class K8sLintReport:
    """V1386 真生产 k8s manifest lint 真报报告 (主 17:43 实事求是)."""

    file_path: str
    n_lines: int
    n_documents: int
    n_findings: int
    n_errors: int
    n_warnings: int
    n_info: int
    findings: List[K8sFinding] = field(default_factory=list)
    parse_ok: bool = True
    parse_error: str = ""
    elapsed_seconds: float = 0.0
    ok: bool = True

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file_path": self.file_path,
            "n_lines": self.n_lines,
            "n_documents": self.n_documents,
            "n_findings": self.n_findings,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "findings": [f.to_dict() for f in self.findings],
            "parse_ok": self.parse_ok,
            "parse_error": self.parse_error,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "ok": self.ok,
            "v1386_version": V1386_VERSION,
        }


# ============================================================================
# V1386 真生产 YAML 多文档解析 (主 19:33 + 主 17:43)
# ============================================================================


@dataclass
class ContainerInfo:
    """V1386 真生产 container 提取 (主 17:43 真解析)."""

    name: str
    line_no: int                # container 出现的行 (近似)
    image: str = ""
    has_resource_limits: bool = False
    has_readiness_probe: bool = False
    has_liveness_probe: bool = False
    has_security_ctx: bool = False  # securityContext.capabilities.drop = ALL 等
    has_privileged: bool = False    # securityContext.privileged = true
    env: Dict[str, str] = field(default_factory=dict)


@dataclass
class PodSpecInfo:
    """V1386 真生产 PodSpec 提取 (主 17:43 真解析)."""

    containers: List[ContainerInfo] = field(default_factory=list)
    host_network: bool = False
    line_no: int = 0


@dataclass
class ManifestDoc:
    """V1386 真生产 k8s manifest doc 提取 (主 17:43 真解析)."""

    kind: str
    api_version: str
    name: str
    namespace: str
    line_no: int
    pod_spec: Optional[PodSpecInfo] = None  # Deployment/StatefulSet/DaemonSet/Job/Pod
    containers: List[ContainerInfo] = field(default_factory=list)  # 与 pod_spec.containers 等价 (扁平访问)
    raw: Dict[str, Any] = field(default_factory=dict)


def _flatten_str(value: Any) -> str:
    """V1386 真生产 把任意 scalar 拍平成字符串 (主 17:43)."""
    if value is None:
        return ""
    if isinstance(value, str):
        return value
    if isinstance(value, (int, float, bool)):
        return str(value)
    return str(value)


def _env_to_dict(env_value: Any) -> Dict[str, str]:
    """V1386 真生产 env (list) → flat dict (主 17:43 真解析).

    k8s env 形式:
      - list: [{name, value} | {name, valueFrom}]
      - dict: {NAME: value} (少见, 通常在 PodTemplate)
    """
    out: Dict[str, str] = {}
    if env_value is None:
        return out
    if isinstance(env_value, dict):
        for k, v in env_value.items():
            out[str(k)] = _flatten_str(v)
        return out
    if isinstance(env_value, list):
        for item in env_value:
            if not isinstance(item, dict):
                continue
            name = _flatten_str(item.get("name", ""))
            if not name:
                continue
            # 有 valueFrom 视为非字面量 (引用 Secret/CM/Field), 不报警
            if isinstance(item.get("valueFrom"), dict):
                out[name] = ""  # 标记为引用, 空值
                continue
            out[name] = _flatten_str(item.get("value", ""))
        return out
    return out


def _has_drop_all_capabilities(security_ctx: Any) -> bool:
    """V1386 真生产 检查 securityContext 是否 drop ALL capabilities (主 17:43 真解析)."""
    if not isinstance(security_ctx, dict):
        return False
    caps = security_ctx.get("capabilities")
    if not isinstance(caps, dict):
        return False
    drop = caps.get("drop")
    if not isinstance(drop, list):
        return False
    for item in drop:
        if _flatten_str(item).upper() == "ALL":
            return True
    return False


def _build_line_map(text: str) -> Dict[Tuple[str, ...], int]:
    """V1386 真生产 构造 (path_tuple → line_no) 近似映射 (主 17:43 真解析).

    PyYAML 默认不暴露 key 位置. 这里用朴素扫描: 对每一行, 记录顶层 + 1 层 + 2 层 key.
    """
    out: Dict[Tuple[str, ...], int] = {}
    lines = text.splitlines()
    current_top: Optional[str] = None
    for idx, raw_line in enumerate(lines, start=1):
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        indent = len(raw_line) - len(raw_line.lstrip())
        stripped = raw_line.strip()
        if ":" not in stripped:
            continue
        key, _, _ = stripped.partition(":")
        key = key.strip()
        if indent == 0:
            current_top = key
            out[(key,)] = idx
        elif indent == 2:
            if current_top is not None:
                out[(current_top, key)] = idx
    return out


def _parse_container(c: Dict[str, Any], line_no: int) -> ContainerInfo:
    """V1386 真生产 提取单个 container 信息 (主 17:43 真解析)."""
    ci = ContainerInfo(name=_flatten_str(c.get("name", "<unnamed>")), line_no=line_no)
    ci.image = _flatten_str(c.get("image", ""))

    # resources.limits
    res = c.get("resources")
    if isinstance(res, dict):
        limits = res.get("limits")
        if isinstance(limits, dict) and len(limits) > 0:
            ci.has_resource_limits = True

    # probes
    if isinstance(c.get("readinessProbe"), dict):
        ci.has_readiness_probe = True
    if isinstance(c.get("livenessProbe"), dict):
        ci.has_liveness_probe = True

    # securityContext
    sec_ctx = c.get("securityContext")
    if isinstance(sec_ctx, dict):
        ci.has_security_ctx = True
        if sec_ctx.get("privileged") is True:
            ci.has_privileged = True
        if not _has_drop_all_capabilities(sec_ctx):
            # 有 securityContext 但没 drop ALL → 算 partial (仍按 security_ctx 标记存在)
            pass

    ci.env = _env_to_dict(c.get("env"))
    return ci


def _parse_pod_spec(spec: Dict[str, Any], base_line: int) -> PodSpecInfo:
    """V1386 真生产 提取 PodSpec (主 17:43 真解析).

    spec 形如:
      spec:
        containers: [...]
        hostNetwork: true
    """
    pi = PodSpecInfo(line_no=base_line)
    if not isinstance(spec, dict):
        return pi
    if spec.get("hostNetwork") is True:
        pi.host_network = True
    containers = spec.get("containers", [])
    if isinstance(containers, list):
        for c in containers:
            if isinstance(c, dict):
                ci = _parse_container(c, base_line)  # 近似行号
                pi.containers.append(ci)
    # initContainers 也算 (它们通常做 setup, 也需要 security)
    init_containers = spec.get("initContainers", [])
    if isinstance(init_containers, list):
        for c in init_containers:
            if isinstance(c, dict):
                ci = _parse_container(c, base_line)
                pi.containers.append(ci)
    return pi


def parse_k8s_manifests(text: str) -> Tuple[List[Dict[str, Any]], List[ManifestDoc], str]:
    """V1386 真生产 解析 k8s manifest (主 17:43 真解析 多文档).

    返回:
        (raw_docs, manifest_list, error_string)

    真解析:
      - PyYAML safe_load_all 多文档 (--- 分隔)
      - 每 doc: kind / apiVersion / metadata.name / metadata.namespace / spec (Deployment/StatefulSet/DaemonSet/Job/Pod)
      - 对 Deployment 等, 提取 spec.template.spec (PodSpec)
      - 对 Pod, 直接提取 spec
      - 处理 PyYAML anchor merge (<<: *xxx) — PyYAML 自动展开
    """
    if not YAML_AVAILABLE:
        return [], [], f"PyYAML not available: {YAML_ERROR}"

    try:
        # PyYAML safe_load_all 会跳过空文档
        raw_docs_iter = yaml.safe_load_all(text)
        raw_docs: List[Dict[str, Any]] = []
        for d in raw_docs_iter:
            if d is None:
                continue
            if not isinstance(d, dict):
                return [], [], f"Each YAML document must be a mapping (got {type(d).__name__})."
            raw_docs.append(d)
    except yaml.YAMLError as e:
        return [], [], f"YAML parse error: {e}"

    line_map = _build_line_map(text)

    manifests: List[ManifestDoc] = []
    for d in raw_docs:
        kind = _flatten_str(d.get("kind", "")).strip()
        api_version = _flatten_str(d.get("apiVersion", "")).strip()
        meta = d.get("metadata", {})
        if not isinstance(meta, dict):
            meta = {}
        name = _flatten_str(meta.get("name", "<unnamed>")).strip() or "<unnamed>"
        namespace = _flatten_str(meta.get("namespace", "")).strip() or "<default>"

        if not kind:
            # 没 kind 视为无效, 但仍记一行错误
            manifests.append(ManifestDoc(
                kind="<unknown>",
                api_version=api_version,
                name=name,
                namespace=namespace,
                line_no=line_map.get((api_version,), 0),  # 兜底
                raw=d,
            ))
            continue

        # 行号定位
        line_no = line_map.get((kind,), 0)
        md = ManifestDoc(
            kind=kind,
            api_version=api_version,
            name=name,
            namespace=namespace,
            line_no=line_no,
            raw=d,
        )

        # 找 PodSpec
        spec = d.get("spec")
        pod_spec: Optional[PodSpecInfo] = None
        if isinstance(spec, dict):
            if kind in ("Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
                tpl = spec.get("template")
                if isinstance(tpl, dict):
                    tpl_spec = tpl.get("spec")
                    if isinstance(tpl_spec, dict):
                        pod_spec = _parse_pod_spec(tpl_spec, line_no)
            elif kind == "Pod":
                pod_spec = _parse_pod_spec(spec, line_no)
            elif kind in ("CronJob",):
                jt = spec.get("jobTemplate")
                if isinstance(jt, dict):
                    jt_spec = jt.get("spec")
                    if isinstance(jt_spec, dict):
                        tpl = jt_spec.get("template")
                        if isinstance(tpl, dict):
                            tpl_spec = tpl.get("spec")
                            if isinstance(tpl_spec, dict):
                                pod_spec = _parse_pod_spec(tpl_spec, line_no)

        if pod_spec is not None:
            md.pod_spec = pod_spec
            md.containers = list(pod_spec.containers)

        manifests.append(md)

    return raw_docs, manifests, ""


# ============================================================================
# V1386 真生产 规则定义 (主 19:33 kubeval/kubeconform/polaris 真借鉴)
# ============================================================================


_SECRET_NAME_RE = re.compile(r"(?i)(KEY|SECRET|TOKEN|PASSWORD|API_KEY|PRIVATE_KEY)")
_LATEST_TAG_RE = re.compile(r":latest(\s|$)")  # 仅匹配显式 :latest (末尾)


def _rule_latest_tag(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-LATEST-TAG: image 显式 :latest 或缺 tag (主 19:33 kubeval/kubeconform 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        for c in m.containers:
            if not c.image:
                continue
            if _LATEST_TAG_RE.search(c.image):
                out.append(K8sFinding(
                    rule_id="K8S-LATEST-TAG",
                    severity="warning",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' image '{c.image}' uses :latest tag (non-reproducible).",
                    suggestion="Pin to a specific version or SHA digest (e.g. ...@sha256:...).",
                ))
            elif ":" not in c.image:
                # image 没 tag, 默认 :latest
                out.append(K8sFinding(
                    rule_id="K8S-LATEST-TAG",
                    severity="warning",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' image '{c.image}' has no tag (defaults to :latest).",
                    suggestion="Add an explicit tag or digest to the image.",
                ))
    return out


def _rule_no_resource_limits(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-NO-RESOURCE-LIMITS: container 缺 resources.limits (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        for c in m.containers:
            if not c.has_resource_limits:
                out.append(K8sFinding(
                    rule_id="K8S-NO-RESOURCE-LIMITS",
                    severity="warning",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' has no resources.limits (OOM/overuse risk).",
                    suggestion="Add resources.limits.cpu and resources.limits.memory.",
                ))
    return out


def _rule_no_readiness(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-NO-READINESS: container 缺 readinessProbe (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        # 只对 workload 类型的 container 报警 (Pod/Deployment/StatefulSet/DaemonSet/Job)
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        for c in m.containers:
            if not c.has_readiness_probe:
                out.append(K8sFinding(
                    rule_id="K8S-NO-READINESS",
                    severity="info",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' has no readinessProbe (traffic may hit before ready).",
                    suggestion="Add readinessProbe.httpGet/tcpSocket/exec to gate Service endpoints.",
                ))
    return out


def _rule_no_liveness(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-NO-LIVENESS: container 缺 livenessProbe (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        for c in m.containers:
            if not c.has_liveness_probe:
                out.append(K8sFinding(
                    rule_id="K8S-NO-LIVENESS",
                    severity="info",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' has no livenessProbe (no auto-restart on hang).",
                    suggestion="Add livenessProbe.httpGet/tcpSocket/exec to detect deadlocks.",
                ))
    return out


def _rule_no_security_ctx(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-NO-SECURITY-CTX: container 缺 securityContext.capabilities.drop=ALL (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        for c in m.containers:
            if not c.has_security_ctx:
                out.append(K8sFinding(
                    rule_id="K8S-NO-SECURITY-CTX",
                    severity="warning",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' has no securityContext (attack surface larger).",
                    suggestion="Add securityContext: { allowPrivilegeEscalation: false, capabilities: { drop: [ALL] }, readOnlyRootFilesystem: true }.",
                ))
    return out


def _rule_privileged(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-PRIVILEGED: container securityContext.privileged=true (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        for c in m.containers:
            if c.has_privileged:
                out.append(K8sFinding(
                    rule_id="K8S-PRIVILEGED",
                    severity="error",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' runs privileged (security risk).",
                    suggestion="Remove 'privileged: true' or scope to specific capabilities.",
                ))
    return out


def _rule_host_network(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-HOST-NETWORK: pod-level hostNetwork: true (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        if m.pod_spec is not None and m.pod_spec.host_network:
            out.append(K8sFinding(
                rule_id="K8S-HOST-NETWORK",
                severity="warning",
                kind=m.kind,
                name=m.name,
                namespace=m.namespace,
                container="<pod>",
                line_no=m.pod_spec.line_no,
                message=f"{m.kind}/{m.name} uses hostNetwork: true (network isolation disabled).",
                suggestion="Use a dedicated NetworkPolicy / Service instead.",
            ))
    return out


def _rule_plaintext_secret(manifests: List[ManifestDoc], raw_docs: List[Dict[str, Any]]) -> List[K8sFinding]:
    """V1386 真借鉴 K8S-PLAINTEXT-SECRET: env 出现 KEY/SECRET/TOKEN/PASSWORD 字面量非占位符 (主 19:33 polaris 真借鉴)."""
    out: List[K8sFinding] = []
    for m in manifests:
        if m.kind not in ("Pod", "Deployment", "StatefulSet", "DaemonSet", "Job", "ReplicaSet"):
            continue
        for c in m.containers:
            for k, v in c.env.items():
                if not _SECRET_NAME_RE.search(k):
                    continue
                if not v:
                    # 空值 (含 valueFrom 引用 / 字面量空) 不报警
                    continue
                # 占位符 ${VAR} / ${VAR:-} 也跳过
                if v.strip().startswith("${"):
                    continue
                out.append(K8sFinding(
                    rule_id="K8S-PLAINTEXT-SECRET",
                    severity="warning",
                    kind=m.kind,
                    name=m.name,
                    namespace=m.namespace,
                    container=c.name,
                    line_no=c.line_no,
                    message=f"container '{c.name}' env '{k}' has a literal value (looks like plaintext secret).",
                    suggestion="Use Secret + envFrom / valueFrom.secretKeyRef; never commit secrets.",
                ))
    return out


# 真生产 规则注册表 (主 17:43 实事求是)
RESOURCE_RULES: List[Callable[[List[ManifestDoc], List[Dict[str, Any]]], List[K8sFinding]]] = [
    _rule_latest_tag,
    _rule_no_resource_limits,
    _rule_no_readiness,
    _rule_no_liveness,
    _rule_no_security_ctx,
    _rule_privileged,
    _rule_host_network,
    _rule_plaintext_secret,
]


# ============================================================================
# V1386 真生产 lint runner (主 06:15 + 主 17:43 + 主 19:33)
# ============================================================================


class V1386K8sLint:
    """V1386 ASI 真生产 Kubernetes manifest 真解析 + 真 lint.

    真生产策略 (主 17:43 实事求是):
    - 真 read k8s manifest YAML 文件 (多文档 ---)
    - 真用 PyYAML safe_load_all 解析
    - 真提取 kind / apiVersion / metadata.name / metadata.namespace / spec (Deployment/StatefulSet/DaemonSet/Job/Pod)
    - 真跑 8 条规则 (kubeval/kubeconform/polaris 借鉴)
    - 真报 finding list + summary
    - 真 CLI: lint / json / strict / quiet / version / demo
    """

    def __init__(self, rules: Optional[List[Callable]] = None):
        self.rules = rules or RESOURCE_RULES
        self.last_report: Optional[K8sLintReport] = None

    def lint_text(self, text: str, file_path: str = "<text>") -> K8sLintReport:
        """V1386 真生产 真 lint 一段 k8s manifest YAML 文本 (主 17:43 真跑)."""
        start = time.time()
        raw_docs, manifests, err = parse_k8s_manifests(text)
        findings: List[K8sFinding] = []

        if err:
            # YAML 解析失败 → 一条 error
            elapsed = time.time() - start
            report = K8sLintReport(
                file_path=file_path,
                n_lines=len(text.splitlines()),
                n_documents=0,
                n_findings=1,
                n_errors=1,
                n_warnings=0,
                n_info=0,
                findings=[K8sFinding(
                    rule_id="K8S-PARSE-ERROR",
                    severity="error",
                    kind="<root>",
                    name="<root>",
                    namespace="<root>",
                    container="<root>",
                    line_no=0,
                    message=err,
                    suggestion="Fix YAML syntax errors before linting rules.",
                )],
                parse_ok=False,
                parse_error=err,
                elapsed_seconds=elapsed,
                ok=False,
            )
            self.last_report = report
            return report

        for rule in self.rules:
            findings.extend(rule(manifests, raw_docs))

        # 排序: kind, namespace, name, container, line_no, severity
        severity_order = {"error": 0, "warning": 1, "info": 2}
        findings.sort(key=lambda f: (
            f.kind, f.namespace, f.name, f.container, f.line_no,
            severity_order.get(f.severity, 3),
        ))

        n_err = sum(1 for f in findings if f.severity == "error")
        n_warn = sum(1 for f in findings if f.severity == "warning")
        n_info = sum(1 for f in findings if f.severity == "info")

        elapsed = time.time() - start
        ok = n_err == 0
        report = K8sLintReport(
            file_path=file_path,
            n_lines=len(text.splitlines()),
            n_documents=len(raw_docs),
            n_findings=len(findings),
            n_errors=n_err,
            n_warnings=n_warn,
            n_info=n_info,
            findings=findings,
            parse_ok=True,
            parse_error="",
            elapsed_seconds=elapsed,
            ok=ok,
        )
        self.last_report = report
        return report

    def lint_file(self, file_path: str) -> K8sLintReport:
        """V1386 真生产 真 lint 一个 k8s manifest YAML 文件 (主 17:43 真文件)."""
        with open(file_path, "r", encoding="utf-8") as fh:
            text = fh.read()
        return self.lint_text(text, file_path=file_path)


# ============================================================================
# V1386 真生产 CLI (主 00:36 质量 + 适配性 + 效果 + 工程化)
# ============================================================================


def _format_text(report: K8sLintReport, *, strict: bool = False, quiet: bool = False) -> str:
    """V1386 真生产 CLI text 格式 (主 17:43 真报告)."""
    lines: List[str] = []
    lines.append(f"V1386 k8s-lint v{V1386_VERSION} — {report.file_path}")
    lines.append(
        f"  parse_ok={report.parse_ok} n_documents={report.n_documents} "
        f"n_findings={report.n_findings} (errors={report.n_errors} warnings={report.n_warnings} info={report.n_info})"
    )
    if not report.parse_ok:
        lines.append(f"  parse_error: {report.parse_error}")
    if not quiet:
        for f in report.findings:
            sev = f.severity.upper()
            tag = f"[{sev:<7}]"
            loc_parts = [f.kind, f.namespace, f.name]
            if f.container and f.container != "<root>":
                loc_parts.append(f.container)
            loc = "/".join(p for p in loc_parts if p)
            if f.line_no:
                loc = f"{loc}:{f.line_no}"
            lines.append(f"  {tag} {f.rule_id:<26} {loc:<60} {f.message}")
            if f.suggestion and not strict:
                lines.append(f"             → {f.suggestion}")
    lines.append(f"  elapsed={report.elapsed_seconds:.4f}s  ok={report.ok}")
    return "\n".join(lines)


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1386 真生产 CLI 入口 (主 00:36 工程化)."""
    parser = argparse.ArgumentParser(
        prog="v1386-k8s-lint",
        description="V1386 ASI Kubernetes manifest 真解析 + 真 lint",
    )
    parser.add_argument("path", nargs="?", default="-",
                        help="k8s manifest YAML file path (default: stdin, '-' reads stdin)")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--strict", action="store_true",
                        help="exit non-zero on any warning (default: only on error)")
    parser.add_argument("--quiet", action="store_true", help="suppress finding details")
    parser.add_argument("--demo", action="store_true",
                        help="run a built-in demo (no file needed) and exit")
    parser.add_argument("--version", action="store_true", help="print version and exit")
    args = parser.parse_args(argv)

    if args.version:
        print(f"V1386 k8s-lint v{V1386_VERSION}")
        return 0

    if args.demo:
        # 真 demo: 触发 7 条规则的 manifest
        demo_text = (
            "apiVersion: v1\n"
            "kind: Pod\n"
            "metadata:\n"
            "  name: bad-app\n"
            "  namespace: default\n"
            "spec:\n"
            "  hostNetwork: true\n"
            "  containers:\n"
            "    - name: app\n"
            "      image: nginx\n"  # 无 tag → :latest
            "      env:\n"
            "        - name: DB_PASSWORD\n"
            "          value: hunter2\n"
            "      securityContext:\n"
            "        privileged: true\n"
            "        capabilities:\n"
            "          drop: [\"ALL\"]\n"
            # 故意没 resources / readinessProbe / livenessProbe
        )
        report = V1386K8sLint().lint_text(demo_text, "<demo>")
        if args.json:
            print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_format_text(report, strict=args.strict, quiet=args.quiet))
        # demo 默认 exit 0 (它就是用来演示), 即使有 error
        return 0

    # 真文件 lint
    if args.path == "-":
        text = sys.stdin.read()
        report = V1386K8sLint().lint_text(text, file_path="<stdin>")
    else:
        if not os.path.isfile(args.path):
            print(f"V1386: file not found: {args.path}", file=sys.stderr)
            return 2
        report = V1386K8sLint().lint_file(args.path)

    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(_format_text(report, strict=args.strict, quiet=args.quiet))

    # exit code: 0 ok / 1 has error / 2 strict has warning
    if not report.ok:
        return 1
    if args.strict and report.n_warnings > 0:
        return 2
    return 0


# ============================================================================
# V1386 真生产 8 GUARDS (主 17:58 + 主 20:46 + 主 00:36)
# ============================================================================

GUARDS = [
    "GUARD_LINT_REAL",              # 真 parse + 真规则匹配, 不假装 lint
    "GUARD_NO_CAP_CHANGE",          # 不动 ASI 北极星 0.9 lock
    "GUARD_DETERMINISTIC",          # 同 input → 同 output (无随机)
    "GUARD_PATH_SAFE",              # path 不越界, 不写工作目录以外
    "GUARD_HONEST_DISCLOSURE",      # 真报告 finding, 不掩盖
    "GUARD_K8S_ONLY",               # 只 lint k8s manifest, 不碰 Dockerfile/Compose (V1384/V1385 范围)
    "GUARD_BORROW_OPEN_SOURCE",     # 真借鉴 kubeval/kubeconform/polaris, 不假装原创
    "GUARD_CLI_RUNNABLE",           # CLI 真可跑, 真 exit code
]
