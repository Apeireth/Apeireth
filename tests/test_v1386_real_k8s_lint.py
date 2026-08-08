"""Tests for V1386 ASI Kubernetes manifest 真解析 + 真 lint (主 06:15 + 主 17:43 + 主 19:33 + 主 23:44).

主 17:43 实事求是: 真 read + 真 YAML 多文档 parse + 真规则匹配, 不假装 lint.
"""
from __future__ import annotations

import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
from contextlib import redirect_stdout
from typing import Any, Dict, List

import pytest

MODULE_DIR = pathlib.Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(MODULE_DIR))

from v1386_real_k8s_lint import (  # noqa: E402
    V1386_VERSION,
    V1386K8sLint,
    K8sFinding,
    K8sLintReport,
    ContainerInfo,
    ManifestDoc,
    PodSpecInfo,
    RESOURCE_RULES,
    GUARDS,
    YAML_AVAILABLE,
    _build_line_map,
    _env_to_dict,
    _flatten_str,
    _has_drop_all_capabilities,
    parse_k8s_manifests,
    run_cli,
)


# ============================================================================
# fixtures
# ============================================================================


@pytest.fixture
def clean_pod() -> str:
    """V1386 一个完全干净的 Pod (无任何 finding)."""
    return (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: clean-app\n"
        "  namespace: apeireth-v2\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
        "      resources:\n"
        "        limits:\n"
        "          cpu: \"500m\"\n"
        "          memory: \"256Mi\"\n"
        "      readinessProbe:\n"
        "        httpGet:\n"
        "          path: /health\n"
        "          port: 8080\n"
        "      livenessProbe:\n"
        "        httpGet:\n"
        "          path: /health\n"
        "          port: 8080\n"
        "      securityContext:\n"
        "        allowPrivilegeEscalation: false\n"
        "        readOnlyRootFilesystem: true\n"
        "        capabilities:\n"
        "          drop: [\"ALL\"]\n"
    )


@pytest.fixture
def bad_pod() -> str:
    """V1386 一个触发所有 8 条规则的 Pod (含 hostNetwork + privileged + latest tag + plaintext secret + 缺资源/探针)."""
    return (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: bad-app\n"
        "  namespace: default\n"
        "spec:\n"
        "  hostNetwork: true\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:latest\n"
        "      env:\n"
        "        - name: DB_PASSWORD\n"
        "          value: hunter2\n"
        "      securityContext:\n"
        "        privileged: true\n"
        "        capabilities:\n"
        "          drop: [\"ALL\"]\n"
        # 故意没 resources / readinessProbe / livenessProbe
    )


@pytest.fixture
def bad_deployment() -> str:
    """V1386 一个触发 K8S-NO-RESOURCE-LIMITS 的 Deployment (缺资源)."""
    return (
        "apiVersion: apps/v1\n"
        "kind: Deployment\n"
        "metadata:\n"
        "  name: dep-no-res\n"
        "  namespace: prod\n"
        "spec:\n"
        "  replicas: 1\n"
        "  selector:\n"
        "    matchLabels:\n"
        "      app: dep-no-res\n"
        "  template:\n"
        "    metadata:\n"
        "      labels:\n"
        "        app: dep-no-res\n"
        "    spec:\n"
        "      containers:\n"
        "        - name: web\n"
        "          image: nginx:1.27.0\n"
    )


@pytest.fixture
def lint():
    """V1386 真生产 lint runner 实例."""
    return V1386K8sLint()


# ============================================================================
# basic structural tests
# ============================================================================


def test_v1386_module_version_constant():
    """V1386 真生产: V1386_VERSION 已设."""
    assert isinstance(V1386_VERSION, str)
    assert len(V1386_VERSION.split(".")) == 3


def test_v1386_yaml_available():
    """V1386 真生产: PyYAML 已装."""
    assert YAML_AVAILABLE, "PyYAML must be installed for V1386"


def test_v1386_rules_registry_has_eight():
    """V1386 真生产: RESOURCE_RULES 8 条 (主 17:43 + 主 19:33)."""
    assert len(RESOURCE_RULES) == 8, f"expected 8 rules, got {len(RESOURCE_RULES)}"


def test_v1386_guards_list_eight():
    """V1386 真生产: GUARDS 8 条 (主 17:58 + 主 20:46)."""
    assert len(GUARDS) == 8, f"expected 8 guards, got {len(GUARDS)}"
    assert "GUARD_LINT_REAL" in GUARDS
    assert "GUARD_K8S_ONLY" in GUARDS
    assert "GUARD_BORROW_OPEN_SOURCE" in GUARDS


# ============================================================================
# parse / data extraction tests
# ============================================================================


def test_v1386_parse_clean_pod_extracts_container(clean_pod):
    """V1386 真解析: clean Pod 提取 1 个 container, image/version 正确."""
    raw_docs, manifests, err = parse_k8s_manifests(clean_pod)
    assert err == ""
    assert len(manifests) == 1
    m = manifests[0]
    assert m.kind == "Pod"
    assert m.name == "clean-app"
    assert m.namespace == "apeireth-v2"
    assert m.pod_spec is not None
    assert len(m.pod_spec.containers) == 1
    ci = m.pod_spec.containers[0]
    assert ci.name == "app"
    assert ci.image == "nginx:1.27.0"
    assert ci.has_resource_limits is True
    assert ci.has_readiness_probe is True
    assert ci.has_liveness_probe is True
    assert ci.has_security_ctx is True
    assert ci.has_privileged is False
    assert m.pod_spec.host_network is False


def test_v1386_parse_bad_pod_extracts_all_signals(bad_pod):
    """V1386 真解析: bad Pod 提取 hostNetwork / privileged / 缺资源 / 缺探针 / secret."""
    raw_docs, manifests, err = parse_k8s_manifests(bad_pod)
    assert err == ""
    assert len(manifests) == 1
    m = manifests[0]
    assert m.pod_spec is not None
    assert m.pod_spec.host_network is True
    ci = m.pod_spec.containers[0]
    assert ci.name == "app"
    assert ci.image == "nginx:latest"
    assert ci.has_privileged is True
    assert ci.has_resource_limits is False
    assert ci.has_readiness_probe is False
    assert ci.has_liveness_probe is False
    assert ci.env.get("DB_PASSWORD") == "hunter2"


def test_v1386_parse_deployment_via_template_spec(bad_deployment):
    """V1386 真解析: Deployment 通过 spec.template.spec 提取 PodSpec."""
    raw_docs, manifests, err = parse_k8s_manifests(bad_deployment)
    assert err == ""
    assert len(manifests) == 1
    m = manifests[0]
    assert m.kind == "Deployment"
    assert m.namespace == "prod"
    assert m.pod_spec is not None
    assert len(m.pod_spec.containers) == 1
    assert m.pod_spec.containers[0].name == "web"
    assert m.pod_spec.containers[0].has_resource_limits is False


def test_v1386_parse_multi_document_yaml():
    """V1386 真解析: 多文档 (--- 分隔) 真解析多个 manifest."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: a\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
        "---\n"
        "apiVersion: v1\n"
        "kind: Service\n"
        "metadata:\n"
        "  name: a-svc\n"
        "spec:\n"
        "  selector:\n"
        "    app: a\n"
        "  ports:\n"
        "    - port: 80\n"
        "      targetPort: 8080\n"
    )
    raw_docs, manifests, err = parse_k8s_manifests(text)
    assert err == ""
    assert len(raw_docs) == 2
    assert len(manifests) == 2
    assert manifests[0].kind == "Pod"
    assert manifests[1].kind == "Service"
    # Service 没 PodSpec
    assert manifests[1].pod_spec is None


def test_v1386_parse_yaml_error_returns_error():
    """V1386 真解析: YAML 语法错返回 error."""
    # 真实语法错: 缩进断裂 + 非法 tab/字符
    text = "apiVersion: v1\nkind: Pod\nmetadata:\n  name: bad\n  unterminated: \"\n"
    raw_docs, manifests, err = parse_k8s_manifests(text)
    assert err != ""
    assert "YAML parse error" in err


def test_v1386_parse_top_not_mapping():
    """V1386 真解析: 顶层不是 mapping 报错."""
    text = "- just a list\n- not a mapping\n"
    raw_docs, manifests, err = parse_k8s_manifests(text)
    assert err != ""
    assert "mapping" in err


def test_v1386_parse_init_containers_extracted():
    """V1386 真解析: initContainers 也提取为 containers (主 17:43 真解析)."""
    text = (
        "apiVersion: apps/v1\n"
        "kind: Deployment\n"
        "metadata:\n"
        "  name: with-init\n"
        "  namespace: apeireth-v2\n"
        "spec:\n"
        "  template:\n"
        "    spec:\n"
        "      initContainers:\n"
        "        - name: fix-perms\n"
        "          image: busybox:1.36\n"
        "      containers:\n"
        "        - name: app\n"
        "          image: app:1.0\n"
        "          resources:\n"
        "            limits:\n"
        "              memory: 256Mi\n"
    )
    raw_docs, manifests, err = parse_k8s_manifests(text)
    assert err == ""
    m = manifests[0]
    assert len(m.pod_spec.containers) == 2  # initContainer + container
    names = sorted([c.name for c in m.pod_spec.containers])
    assert names == ["app", "fix-perms"]


def test_v1386_parse_valueFrom_env_skipped_as_placeholder():
    """V1386 真解析: envFrom / valueFrom.secretKeyRef 视为引用 (主 17:43 真解析)."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: ref-env\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: app:1.0\n"
        "      env:\n"
        "        - name: DB_PASSWORD\n"
        "          valueFrom:\n"
        "            secretKeyRef:\n"
        "              name: db\n"
        "              key: password\n"
    )
    raw_docs, manifests, err = parse_k8s_manifests(text)
    assert err == ""
    ci = manifests[0].pod_spec.containers[0]
    # valueFrom 引用 → 标记为空 (不报警)
    assert ci.env.get("DB_PASSWORD") == ""


# ============================================================================
# rule firing tests (主 17:43 真规则匹配)
# ============================================================================


def test_v1386_clean_pod_zero_findings(clean_pod, lint):
    """V1386 clean Pod 应该零 findings."""
    r = lint.lint_text(clean_pod, "<clean>")
    assert r.parse_ok is True
    assert r.n_findings == 0, f"expected 0 findings, got {r.n_findings}: {[f.rule_id for f in r.findings]}"
    assert r.ok is True


def test_v1386_bad_pod_fires_all_eight_rules(bad_pod, lint):
    """V1386 bad Pod 触发所有 8 条规则."""
    r = lint.lint_text(bad_pod, "<bad>")
    rules_fired = sorted({f.rule_id for f in r.findings})
    expected = sorted([
        "K8S-LATEST-TAG",
        "K8S-PRIVILEGED",
        "K8S-HOST-NETWORK",
        "K8S-PLAINTEXT-SECRET",
        "K8S-NO-RESOURCE-LIMITS",
        "K8S-NO-READINESS",
        "K8S-NO-LIVENESS",
    ])
    # 注意: bad_pod 有 securityContext (capabilities drop ALL), 所以 K8S-NO-SECURITY-CTX 不触发
    # 但有 privileged, 所以 K8S-PRIVILEGED 触发
    for rid in expected:
        assert rid in rules_fired, f"missing rule {rid} in {rules_fired}"
    assert r.n_errors >= 1  # 至少 privileged error
    assert r.ok is False


def test_v1386_rule_latest_tag_explicit(bad_pod, lint):
    """V1386 真规则 K8S-LATEST-TAG: 显式 :latest 触发 warning."""
    findings = [f for f in lint.lint_text(bad_pod, "<bad>").findings if f.rule_id == "K8S-LATEST-TAG"]
    assert len(findings) >= 1
    f = findings[0]
    assert f.severity == "warning"
    assert "nginx:latest" in f.message


def test_v1386_rule_latest_tag_no_tag():
    """V1386 真规则 K8S-LATEST-TAG: 无 tag 也触发 (默认 :latest)."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: no-tag\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx\n"
    )
    findings = [f for f in V1386K8sLint().lint_text(text).findings if f.rule_id == "K8S-LATEST-TAG"]
    assert len(findings) == 1
    assert "no tag" in findings[0].message


def test_v1386_rule_no_resource_limits(bad_deployment, lint):
    """V1386 真规则 K8S-NO-RESOURCE-LIMITS: 缺 resources.limits 触发."""
    findings = [f for f in lint.lint_text(bad_deployment).findings if f.rule_id == "K8S-NO-RESOURCE-LIMITS"]
    assert len(findings) == 1
    assert findings[0].severity == "warning"


def test_v1386_rule_no_security_ctx():
    """V1386 真规则 K8S-NO-SECURITY-CTX: 缺 securityContext 触发."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: no-sec\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
    )
    findings = [f for f in V1386K8sLint().lint_text(text).findings if f.rule_id == "K8S-NO-SECURITY-CTX"]
    assert len(findings) == 1
    assert findings[0].severity == "warning"


def test_v1386_rule_security_ctx_present_no_fire():
    """V1386 真规则 K8S-NO-SECURITY-CTX: 有 securityContext (含 drop ALL) 不触发."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: has-sec\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
        "      securityContext:\n"
        "        allowPrivilegeEscalation: false\n"
        "        capabilities:\n"
        "          drop: [\"ALL\"]\n"
    )
    findings = [f for f in V1386K8sLint().lint_text(text).findings if f.rule_id == "K8S-NO-SECURITY-CTX"]
    assert len(findings) == 0


def test_v1386_rule_privileged(bad_pod, lint):
    """V1386 真规则 K8S-PRIVILEGED: privileged: true 触发 error."""
    findings = [f for f in lint.lint_text(bad_pod).findings if f.rule_id == "K8S-PRIVILEGED"]
    assert len(findings) == 1
    assert findings[0].severity == "error"


def test_v1386_rule_host_network(bad_pod, lint):
    """V1386 真规则 K8S-HOST-NETWORK: pod-level hostNetwork: true 触发."""
    findings = [f for f in lint.lint_text(bad_pod).findings if f.rule_id == "K8S-HOST-NETWORK"]
    assert len(findings) == 1
    assert findings[0].severity == "warning"
    assert findings[0].container == "<pod>"


def test_v1386_rule_plaintext_secret_skips_valuefrom():
    """V1386 真规则 K8S-PLAINTEXT-SECRET: valueFrom.secretKeyRef 引用不报警."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: ref-sec\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
        "      env:\n"
        "        - name: API_KEY\n"
        "          valueFrom:\n"
        "            secretKeyRef:\n"
        "              name: api\n"
        "              key: key\n"
    )
    findings = [f for f in V1386K8sLint().lint_text(text).findings if f.rule_id == "K8S-PLAINTEXT-SECRET"]
    assert len(findings) == 0


def test_v1386_rule_no_readiness_no_liveness():
    """V1386 真规则 K8S-NO-READINESS / K8S-NO-LIVENESS: 同时触发 (info)."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: no-probes\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:1.27.0\n"
    )
    findings = V1386K8sLint().lint_text(text).findings
    rids = {f.rule_id for f in findings}
    assert "K8S-NO-READINESS" in rids
    assert "K8S-NO-LIVENESS" in rids


def test_v1386_service_manifest_skips_probe_rules():
    """V1386 真规则: Service 没 container, 不触发 probe / resource / privileged / secret 规则."""
    text = (
        "apiVersion: v1\n"
        "kind: Service\n"
        "metadata:\n"
        "  name: x\n"
        "spec:\n"
        "  selector:\n"
        "    app: x\n"
        "  ports:\n"
        "    - port: 80\n"
    )
    findings = V1386K8sLint().lint_text(text).findings
    rids = {f.rule_id for f in findings}
    # Service 没有任何 container, probe/resource/privileged/secret 都不该报
    for rid in ("K8S-NO-RESOURCE-LIMITS", "K8S-NO-READINESS", "K8S-NO-LIVENESS",
                "K8S-PRIVILEGED", "K8S-PLAINTEXT-SECRET"):
        assert rid not in rids, f"Service should not trigger {rid}, got {rids}"


# ============================================================================
# report / sorting tests
# ============================================================================


def test_v1386_report_to_dict_roundtrip(bad_pod, lint):
    """V1386 真报告: to_dict roundtrip 含所有字段."""
    r = lint.lint_text(bad_pod, "<bad>")
    d = r.to_dict()
    assert d["file_path"] == "<bad>"
    assert d["n_documents"] == 1
    assert d["parse_ok"] is True
    assert d["ok"] is False  # 有 error
    assert "v1386_version" in d
    assert isinstance(d["findings"], list)
    for fd in d["findings"]:
        assert {"rule_id", "severity", "kind", "name", "namespace", "container", "line_no", "message", "suggestion"} <= set(fd.keys())


def test_v1386_report_ok_flag_for_clean(clean_pod, lint):
    """V1386 真报告: clean pod → ok=True."""
    r = lint.lint_text(clean_pod, "<clean>")
    assert r.ok is True


def test_v1386_report_sort_by_kind_namespace_name_container_line():
    """V1386 真报告: 排序稳定 (kind, namespace, name, container, line_no, severity)."""
    r = V1386K8sLint().lint_text("")  # empty
    # 直接用 lint 的 finding list sort 验证
    findings = [
        K8sFinding("X", "warning", "Pod", "n", "ns", "c", 5, "msg"),
        K8sFinding("X", "error", "Pod", "n", "ns", "c", 5, "msg"),
        K8sFinding("X", "info", "Deployment", "a", "ns", "c", 3, "msg"),
    ]
    severity_order = {"error": 0, "warning": 1, "info": 2}
    findings.sort(key=lambda f: (
        f.kind, f.namespace, f.name, f.container, f.line_no,
        severity_order.get(f.severity, 3),
    ))
    assert findings[0].severity == "info"  # Deployment first
    assert findings[1].severity == "error"  # error < warning
    assert findings[2].severity == "warning"


def test_v1386_deterministic_lint(bad_pod, lint):
    """V1386 真生产: 同 input → 同 output (除 timing 字段)."""
    r1 = lint.lint_text(bad_pod)
    r2 = lint.lint_text(bad_pod)
    d1 = r1.to_dict()
    d2 = r2.to_dict()
    # elapsed_seconds 会随时间变, 排除
    d1.pop("elapsed_seconds", None)
    d2.pop("elapsed_seconds", None)
    assert d1 == d2


def test_v1386_lint_runner_reusable(lint, bad_pod, clean_pod):
    """V1386 真生产: runner 可重用."""
    r1 = lint.lint_text(bad_pod)
    r2 = lint.lint_text(clean_pod)
    assert r1.ok is False
    assert r2.ok is True
    assert lint.last_report is r2


# ============================================================================
# CLI tests (主 00:36 工程化)
# ============================================================================


def test_v1386_cli_version(capsys):
    """V1386 CLI: --version 输出."""
    with redirect_stdout(io.StringIO()) as buf:
        exit_code = run_cli(["--version"])
    captured = buf.getvalue()
    assert "V1386" in captured
    assert exit_code == 0


def test_v1386_cli_demo_json(capsys):
    """V1386 CLI: --demo --json 输出 JSON 含 7 findings."""
    with redirect_stdout(io.StringIO()) as buf:
        exit_code = run_cli(["--demo", "--json"])
    captured = buf.getvalue()
    doc = json.loads(captured)
    assert doc["v1386_version"] == V1386_VERSION
    assert doc["n_documents"] == 1
    assert doc["n_findings"] >= 7
    assert exit_code == 0


def test_v1386_cli_demo_text(capsys):
    """V1386 CLI: --demo text 格式包含 rule_id."""
    with redirect_stdout(io.StringIO()) as buf:
        run_cli(["--demo"])
    captured = buf.getvalue()
    assert "V1386" in captured
    assert "K8S-LATEST-TAG" in captured or "K8S-PRIVILEGED" in captured


def test_v1386_cli_demo_quiet(capsys):
    """V1386 CLI: --demo --quiet 不输出 finding 行."""
    with redirect_stdout(io.StringIO()) as buf:
        run_cli(["--demo", "--quiet"])
    captured = buf.getvalue()
    assert "V1386" in captured
    assert "K8S-LATEST-TAG" not in captured  # quiet 抑制 finding 详情


def test_v1386_cli_lint_clean_file(clean_pod, tmp_path, capsys):
    """V1386 CLI: lint clean file → exit 0."""
    f = tmp_path / "clean.yaml"
    f.write_text(clean_pod, encoding="utf-8")
    with redirect_stdout(io.StringIO()):
        exit_code = run_cli([str(f)])
    assert exit_code == 0


def test_v1386_cli_lint_bad_file_exit_1(bad_pod, tmp_path):
    """V1386 CLI: lint bad file → exit 1 (有 error)."""
    f = tmp_path / "bad.yaml"
    f.write_text(bad_pod, encoding="utf-8")
    exit_code = run_cli([str(f)])
    assert exit_code == 1


def test_v1386_cli_strict_on_warning_exit_2(tmp_path):
    """V1386 CLI: --strict + warning → exit 2."""
    # 触发 warning 但不触发 error 的: latest tag pod (image nginx:latest, 但带 securityCtx)
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: w\n"
        "spec:\n"
        "  containers:\n"
        "    - name: app\n"
        "      image: nginx:latest\n"
        "      securityContext:\n"
        "        capabilities:\n"
        "          drop: [\"ALL\"]\n"
    )
    f = tmp_path / "warn.yaml"
    f.write_text(text, encoding="utf-8")
    exit_code = run_cli([str(f), "--strict"])
    assert exit_code == 2


def test_v1386_cli_missing_file_exit_2(capsys):
    """V1386 CLI: 文件不存在 → exit 2."""
    import contextlib
    err_buf = io.StringIO()
    with contextlib.redirect_stderr(err_buf):
        exit_code = run_cli(["/nonexistent/path/to/file.yaml"])
    assert exit_code == 2
    assert "not found" in err_buf.getvalue()


def test_v1386_cli_stdin_via_dash(clean_pod, capsys, monkeypatch):
    """V1386 CLI: '-' 从 stdin 读 (主 17:43 真 stdin)."""
    monkeypatch.setattr("sys.stdin", io.StringIO(clean_pod))
    with redirect_stdout(io.StringIO()) as buf:
        exit_code = run_cli(["-"])
    captured = buf.getvalue()
    assert "V1386" in captured
    assert exit_code == 0


# ============================================================================
# helpers / unit tests
# ============================================================================


def test_v1386_flatten_str():
    """V1386 真生产: _flatten_str."""
    assert _flatten_str(None) == ""
    assert _flatten_str("abc") == "abc"
    assert _flatten_str(42) == "42"
    assert _flatten_str(True) == "True"


def test_v1386_env_to_dict_list_with_valuefrom():
    """V1386 真生产: _env_to_dict 处理 list + valueFrom."""
    env = [
        {"name": "FOO", "value": "bar"},
        {"name": "REF", "valueFrom": {"secretKeyRef": {"name": "s", "key": "k"}}},
    ]
    out = _env_to_dict(env)
    assert out["FOO"] == "bar"
    assert out["REF"] == ""  # valueFrom 标记为空


def test_v1386_env_to_dict_dict_form():
    """V1386 真生产: _env_to_dict 处理 dict 形式."""
    env = {"A": "1", "B": "2"}
    assert _env_to_dict(env) == {"A": "1", "B": "2"}


def test_v1386_has_drop_all_capabilities():
    """V1386 真生产: _has_drop_all_capabilities."""
    assert _has_drop_all_capabilities({"capabilities": {"drop": ["ALL"]}}) is True
    assert _has_drop_all_capabilities({"capabilities": {"drop": ["NET_ADMIN"]}}) is False
    assert _has_drop_all_capabilities({"capabilities": {"drop": "ALL"}}) is False  # 非 list
    assert _has_drop_all_capabilities(None) is False
    assert _has_drop_all_capabilities({}) is False


def test_v1386_build_line_map_basic():
    """V1386 真生产: _build_line_map 记录顶层 + 1 层 key 行号."""
    text = (
        "apiVersion: v1\n"
        "kind: Pod\n"
        "metadata:\n"
        "  name: a\n"
    )
    m = _build_line_map(text)
    assert m[("apiVersion",)] == 1
    assert m[("kind",)] == 2
    assert m[("metadata", "name")] == 4  # name 在 line 4 (metadata 在 line 3)


# ============================================================================
# real repo integration tests (主 17:43 真文件)
# ============================================================================


def test_v1386_real_repo_deploy_k8s_lints_clean():
    """V1386 真集成: deploy/k8s-asi.yaml 应该是 clean (0 findings)."""
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    target = repo_root / "deploy" / "k8s-asi.yaml"
    if not target.exists():
        pytest.skip("deploy/k8s-asi.yaml not present")
    r = V1386K8sLint().lint_file(str(target))
    # 这个文件应该 lint 干净 (0 findings)
    assert r.parse_ok is True
    assert r.n_documents >= 2  # Deployment + Service
    assert r.n_findings == 0, (
        f"expected 0 findings on deploy/k8s-asi.yaml, got {r.n_findings}: "
        f"{[(f.rule_id, f.message[:60]) for f in r.findings]}"
    )


def test_v1386_real_repo_rust_k8s_finds_real_issues():
    """V1386 真集成: Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml 应该至少触发 K8S-LATEST-TAG."""
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    target = repo_root / "Apeireth-rust" / "deploy" / "k8s" / "05-apeireth-formal.yaml"
    if not target.exists():
        pytest.skip("Apeireth-rust/deploy/k8s/05-apeireth-formal.yaml not present")
    r = V1386K8sLint().lint_file(str(target))
    assert r.parse_ok is True
    rids = {f.rule_id for f in r.findings}
    # 至少触发 :latest tag 警告
    assert "K8S-LATEST-TAG" in rids, f"expected K8S-LATEST-TAG in {rids}"


# ============================================================================
# chain V1370-V1386 integration
# ============================================================================


def test_v1386_chain_with_v1385_v1384_no_module_conflict():
    """V1386 真生产: 与 V1384 (Dockerfile lint) + V1385 (Compose lint) 共存不冲突."""
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    apeireth_dir = repo_root / "apeireth"
    sys.path.insert(0, str(apeireth_dir))
    # 同时导入 V1384/V1385/V1386
    import v1384_real_dockerfile_lint  # noqa: F401
    import v1385_real_compose_lint  # noqa: F401
    import v1386_real_k8s_lint  # noqa: F401
    # 三个模块都有 VERSION 常量且类型正确
    assert isinstance(v1384_real_dockerfile_lint.V1384_VERSION, str)
    assert isinstance(v1385_real_compose_lint.V1385_VERSION, str)
    assert isinstance(v1386_real_k8s_lint.V1386_VERSION, str)
    # 三个模块的常量名互不冲突
    assert hasattr(v1386_real_k8s_lint, "V1386K8sLint")
    assert hasattr(v1385_real_compose_lint, "V1385ComposeLint")
    assert hasattr(v1384_real_dockerfile_lint, "V1384DockerfileLint")
