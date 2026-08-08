"""Tests for V1385 ASI docker-compose YAML 真解析 + 真 lint (主 06:15 + 主 17:43 + 主 19:33 + 主 23:44).

主 17:43 实事求是: 真 read + 真 YAML parse + 真规则匹配, 不假装 lint.
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

from v1385_real_compose_lint import (  # noqa: E402
    V1385_VERSION,
    V1385ComposeLint,
    ComposeFinding,
    ComposeLintReport,
    ServiceInfo,
    SERVICE_RULES,
    GUARDS,
    YAML_AVAILABLE,
    _build_line_map,
    _collect_volume_strings,
    _env_to_dict,
    _flatten_str,
    parse_compose_services,
    run_cli,
)


# ============================================================================
# fixtures
# ============================================================================


@pytest.fixture
def clean_compose() -> str:
    """V1385 一个完全干净的 compose 文件 (无任何 finding)."""
    return (
        "version: '3.8'\n"
        "services:\n"
        "  web:\n"
        "    image: nginx:1.27.0\n"
        "    ports:\n"
        "      - '8080:80'\n"
        "    healthcheck:\n"
        "      test: ['CMD', 'curl', '-f', 'http://localhost/']\n"
        "      interval: 10s\n"
        "    restart: unless-stopped\n"
        "    deploy:\n"
        "      resources:\n"
        "        limits:\n"
        "          memory: 256M\n"
        "    environment:\n"
        "      NORMAL_VAR: foo\n"
        "      API_KEY: ${API_KEY}\n"
    )


@pytest.fixture
def bad_compose() -> str:
    """V1385 一个触发所有 8 条规则的 compose 文件."""
    return (
        "version: '3.8'\n"
        "services:\n"
        "  bad-app:\n"
        "    image: nginx:latest\n"
        "    privileged: true\n"
        "    network_mode: host\n"
        "    volumes:\n"
        "      - /var/run/docker.sock:/var/run/docker.sock\n"
        "    environment:\n"
        "      DB_PASSWORD: hunter2\n"
        "      NORMAL_VAR: foo\n"
        "    depends_on:\n"
        "      - db\n"
        "  db:\n"
        "    image: postgres:15\n"
        "    healthcheck:\n"
        "      test: [\"CMD\", \"pg_isready\"]\n"
        "      interval: 5s\n"
        "    restart: unless-stopped\n"
        "    deploy:\n"
        "      resources:\n"
        "        limits:\n"
        "          memory: 512M\n"
        "    depends_on:\n"
        "      bad-app:\n"
        "        condition: service_healthy\n"
    )


@pytest.fixture
def lint(clean_compose, bad_compose):
    """V1385 真生产 lint runner 实例."""
    return V1385ComposeLint()


# ============================================================================
# basic structural tests
# ============================================================================


def test_v1385_module_version_constant():
    """V1385 真生产: V1385_VERSION 已设."""
    assert isinstance(V1385_VERSION, str)
    assert len(V1385_VERSION.split(".")) == 3


def test_v1385_yaml_available():
    """V1385 真生产: PyYAML 已装."""
    assert YAML_AVAILABLE, "PyYAML must be installed for V1385"


def test_v1385_rules_registry_has_eight():
    """V1385 真生产: SERVICE_RULES 8 条 (主 17:43 + 主 19:33)."""
    assert len(SERVICE_RULES) == 8, f"expected 8 rules, got {len(SERVICE_RULES)}"


def test_v1385_guards_list_eight():
    """V1385 真生产: GUARDS 8 条 (主 17:58 + 主 20:46)."""
    assert len(GUARDS) == 8
    assert "GUARD_LINT_REAL" in GUARDS
    assert "GUARD_NO_CAP_CHANGE" in GUARDS
    assert "GUARD_HONEST_DISCLOSURE" in GUARDS


# ============================================================================
# parser tests
# ============================================================================


def test_v1385_parse_clean_compose(clean_compose):
    """V1385 真生产: 干净 compose 真 parse 出 1 个 service."""
    raw, services, err = parse_compose_services(clean_compose)
    assert err == ""
    assert len(services) == 1
    s = services[0]
    assert s.name == "web"
    assert s.image == "nginx:1.27.0"
    assert s.has_healthcheck is True
    assert s.has_restart is True
    assert s.has_memory_limit is True
    assert s.privileged is False
    assert s.network_mode == ""


def test_v1385_parse_bad_compose_services(bad_compose):
    """V1385 真生产: 坏 compose 真 parse 出 2 个 service."""
    raw, services, err = parse_compose_services(bad_compose)
    assert err == ""
    assert len(services) == 2
    names = {s.name for s in services}
    assert names == {"bad-app", "db"}


def test_v1385_parse_depends_on_list_form(bad_compose):
    """V1385 真生产: depends_on list 形式 (bad-app → db) 不算 uses_healthy."""
    _, services, _ = parse_compose_services(bad_compose)
    bad_app = next(s for s in services if s.name == "bad-app")
    assert "db" in bad_app.depends_on_targets
    assert bad_app.depends_on_uses_healthy is False


def test_v1385_parse_depends_on_dict_form_healthy(bad_compose):
    """V1385 真生产: depends_on dict + condition: service_healthy → uses_healthy=True."""
    _, services, _ = parse_compose_services(bad_compose)
    db = next(s for s in services if s.name == "db")
    assert "bad-app" in db.depends_on_targets
    assert db.depends_on_uses_healthy is True


def test_v1385_parse_env_dict_and_list():
    """V1385 真生产: env 既支持 dict 也支持 list (key=value 形式)."""
    text_d = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    environment:\n"
        "      A: alpha\n"
        "      B: beta\n"
    )
    text_l = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    environment:\n"
        "      - A=alpha\n"
        "      - B=beta\n"
    )
    _, sd, _ = parse_compose_services(text_d)
    _, sl, _ = parse_compose_services(text_l)
    assert sd[0].env == {"A": "alpha", "B": "beta"}
    assert sl[0].env == {"A": "alpha", "B": "beta"}


def test_v1385_parse_volumes_short_and_long():
    """V1385 真生产: volumes 短格式和长格式都解析."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    volumes:\n"
        "      - /data:/data\n"
        "      - type: bind\n"
        "        source: /host\n"
        "        target: /container\n"
    )
    _, services, _ = parse_compose_services(text)
    assert len(services[0].volumes) == 2
    joined = " | ".join(services[0].volumes)
    assert "/data:/data" in joined
    assert "/host:/container" in joined


def test_v1385_parse_yaml_error_returns_error():
    """V1385 真生产: YAML 语法错误 → err 非空 + parse_ok=False."""
    bad = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    ports\n"   # 缺冒号
        "      - '8080:80'\n"
    )
    raw, services, err = parse_compose_services(bad)
    assert err != ""
    assert raw == {}
    assert services == []


def test_v1385_parse_top_not_mapping():
    """V1385 真生产: 顶层不是 mapping → 报错."""
    bad = "- just\n- a\n- list\n"
    _, _, err = parse_compose_services(bad)
    assert "Top-level" in err or "must be" in err


# ============================================================================
# rule tests — individual
# ============================================================================


def test_v1385_rule_latest_tag_explicit(bad_compose, lint):
    """V1385 COMPOSE-LATEST-TAG: image: nginx:latest 命中."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-LATEST-TAG"]
    assert len(hits) >= 1
    assert any(h.service == "bad-app" for h in hits)


def test_v1385_rule_privileged(bad_compose, lint):
    """V1385 COMPOSE-PRIVILEGED: privileged: true 命中 error."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-PRIVILEGED"]
    assert len(hits) == 1
    assert hits[0].severity == "error"
    assert hits[0].service == "bad-app"


def test_v1385_rule_network_host(bad_compose, lint):
    """V1385 COMPOSE-NETWORK-HOST: network_mode: host 命中 warning."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-NETWORK-HOST"]
    assert len(hits) == 1
    assert hits[0].severity == "warning"


def test_v1385_rule_docker_sock(bad_compose, lint):
    """V1385 COMPOSE-DOCKER-SOCK: /var/run/docker.sock 挂载 命中 error."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-DOCKER-SOCK"]
    assert len(hits) == 1
    assert hits[0].severity == "error"


def test_v1385_rule_plaintext_secret(bad_compose, lint):
    """V1385 COMPOSE-PLAINTEXT-SECRET: DB_PASSWORD: hunter2 命中 warning."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-PLAINTEXT-SECRET"]
    assert any(h.service == "bad-app" and "DB_PASSWORD" in h.message for h in hits)


def test_v1385_rule_plaintext_secret_skips_placeholder():
    """V1385 COMPOSE-PLAINTEXT-SECRET: ${VAR} 占位符不报警."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    environment:\n"
        "      API_KEY: ${MY_KEY}\n"
        "      TOKEN: ${TOKEN:-}\n"
    )
    report = V1385ComposeLint().lint_text(text)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-PLAINTEXT-SECRET"]
    assert hits == []


def test_v1385_rule_missing_restart():
    """V1385 COMPOSE-MISSING-RESTART: 缺 restart 命中 warning."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
    )
    report = V1385ComposeLint().lint_text(text)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-MISSING-RESTART"]
    assert len(hits) == 1
    assert hits[0].service == "x"


def test_v1385_rule_missing_memory_limit():
    """V1385 COMPOSE-MISSING-MEM-LIMIT: 缺 memory limit 命中 info."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    restart: unless-stopped\n"
    )
    report = V1385ComposeLint().lint_text(text)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-MISSING-MEM-LIMIT"]
    assert len(hits) == 1
    assert hits[0].severity == "info"


def test_v1385_rule_depends_on_no_healthy(bad_compose, lint):
    """V1385 COMPOSE-DEPENDS-NO-HEALTHY: bad-app list form depends_on, db 有 hc → 命中."""
    report = lint.lint_text(bad_compose)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-DEPENDS-NO-HEALTHY"]
    assert any(h.service == "bad-app" for h in hits)


def test_v1385_rule_depends_on_no_healthy_skipped_when_no_hc_on_target():
    """V1385 COMPOSE-DEPENDS-NO-HEALTHY: target 都没 healthcheck → 不报 (没意义)."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    restart: unless-stopped\n"
        "    depends_on:\n"
        "      - y\n"
        "  y:\n"
        "    image: redis:7\n"
        "    restart: unless-stopped\n"
    )
    report = V1385ComposeLint().lint_text(text)
    hits = [f for f in report.findings if f.rule_id == "COMPOSE-DEPENDS-NO-HEALTHY"]
    assert hits == []


# ============================================================================
# clean file produces near-zero findings (no error/warning)
# ============================================================================


def test_v1385_clean_compose_zero_errors_warnings(clean_compose, lint):
    """V1385 干净 compose 应该 0 error / 0 warning."""
    report = lint.lint_text(clean_compose)
    assert report.parse_ok is True
    assert report.n_errors == 0
    assert report.n_warnings == 0


def test_v1385_bad_compose_has_all_eight_rules(bad_compose, lint):
    """V1385 坏 compose 应该命中全部 8 条规则."""
    report = lint.lint_text(bad_compose)
    rule_ids = {f.rule_id for f in report.findings}
    expected = {
        "COMPOSE-LATEST-TAG",
        "COMPOSE-PRIVILEGED",
        "COMPOSE-NETWORK-HOST",
        "COMPOSE-DOCKER-SOCK",
        "COMPOSE-PLAINTEXT-SECRET",
        "COMPOSE-MISSING-RESTART",
        "COMPOSE-MISSING-MEM-LIMIT",
        "COMPOSE-DEPENDS-NO-HEALTHY",
    }
    missing = expected - rule_ids
    assert not missing, f"rules not fired: {missing}; fired: {rule_ids}"


# ============================================================================
# report tests
# ============================================================================


def test_v1385_report_to_dict_roundtrip(bad_compose, lint):
    """V1385 真生产: to_dict() 可序列化, 含 findings + v1385_version."""
    report = lint.lint_text(bad_compose)
    d = report.to_dict()
    assert d["v1385_version"] == V1385_VERSION
    assert d["n_findings"] == len(d["findings"])
    assert d["ok"] is False  # has errors
    # JSON 序列化检查
    s = json.dumps(d, ensure_ascii=False)
    assert isinstance(s, str)
    parsed = json.loads(s)
    assert parsed["n_findings"] == d["n_findings"]


def test_v1385_report_ok_flag_for_clean(clean_compose, lint):
    """V1385 真生产: 干净 compose report.ok=True."""
    report = lint.lint_text(clean_compose)
    assert report.ok is True


def test_v1385_report_sort_by_service_line_severity():
    """V1385 真生产: findings 按 (service, line_no, severity) 排序."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  z-app:\n"
        "    image: redis:latest\n"
        "  a-app:\n"
        "    image: nginx:latest\n"
        "    privileged: true\n"
    )
    report = V1385ComposeLint().lint_text(text)
    services_in_order = [f.service for f in report.findings]
    # 'a-app' 应在 'z-app' 之前
    if "a-app" in services_in_order and "z-app" in services_in_order:
        assert services_in_order.index("a-app") < services_in_order.index("z-app")


# ============================================================================
# CLI tests
# ============================================================================


def test_v1385_cli_version(capsys):
    """V1385 CLI: --version 印版本."""
    rc = run_cli(["--version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert V1385_VERSION in captured.out


def test_v1385_cli_demo_json(capsys):
    """V1385 CLI: --demo --json 输出 JSON 含 findings."""
    rc = run_cli(["--demo", "--json"])
    captured = capsys.readouterr()
    assert rc == 0
    obj = json.loads(captured.out)
    assert obj["v1385_version"] == V1385_VERSION
    assert obj["n_findings"] >= 1


def test_v1385_cli_demo_text(capsys):
    """V1385 CLI: --demo 输出 text 含 COMPOSE-LATEST-TAG."""
    rc = run_cli(["--demo"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "COMPOSE-LATEST-TAG" in captured.out


def test_v1385_cli_demo_quiet(capsys):
    """V1385 CLI: --demo --quiet 不打印 finding 详情."""
    rc = run_cli(["--demo", "--quiet"])
    captured = capsys.readouterr()
    assert rc == 0
    # quiet 模式不打印 finding.message 行
    assert "COMPOSE-LATEST-TAG" not in captured.out


def test_v1385_cli_lint_clean_file(clean_compose, tmp_path, capsys):
    """V1385 CLI: 真 lint 一个干净文件 → exit 0."""
    f = tmp_path / "clean.yml"
    f.write_text(clean_compose, encoding="utf-8")
    rc = run_cli([str(f)])
    captured = capsys.readouterr()
    assert rc == 0
    assert "ok=True" in captured.out or "ok=True" in captured.out.replace(" ", "")


def test_v1385_cli_lint_bad_file_exit_1(bad_compose, tmp_path):
    """V1385 CLI: 真 lint 坏文件 (有 error) → exit 1."""
    f = tmp_path / "bad.yml"
    f.write_text(bad_compose, encoding="utf-8")
    rc = run_cli([str(f)])
    assert rc == 1


def test_v1385_cli_strict_on_warning_exit_2(bad_compose, tmp_path):
    """V1385 CLI: --strict 在 warning 上也 exit 2."""
    f = tmp_path / "warn.yml"
    # 只触发 warning, 不触发 error
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  x:\n"
        "    image: nginx:1.0\n"
        "    network_mode: host\n"
        "    restart: unless-stopped\n"
        "    deploy:\n"
        "      resources:\n"
        "        limits:\n"
        "          memory: 256M\n"
    )
    f.write_text(text, encoding="utf-8")
    rc = run_cli([str(f), "--strict"])
    assert rc == 2


def test_v1385_cli_missing_file_exit_2(capsys):
    """V1385 CLI: 文件不存在 → exit 2."""
    rc = run_cli(["/nonexistent/compose.yml"])
    captured = capsys.readouterr()
    assert rc == 2
    assert "file not found" in captured.err.lower() or "file not found" in captured.err


def test_v1385_cli_stdin_via_dash(clean_compose, capsys, monkeypatch):
    """V1385 CLI: '-' 读 stdin."""
    monkeypatch.setattr(sys, "stdin", io.StringIO(clean_compose))
    rc = run_cli(["-"])
    captured = capsys.readouterr()
    assert rc == 0


# ============================================================================
# integration: real file from repo (Apeireth-rust/deploy/docker-compose.protocols.yml)
# ============================================================================


def test_v1385_real_repo_compose_file_lints():
    """V1385 真生产: 真 lint Apeireth-rust/deploy/docker-compose.protocols.yml (如果存在)."""
    repo_compose = (
        pathlib.Path(__file__).resolve().parent.parent
        / "Apeireth-rust" / "deploy" / "docker-compose.protocols.yml"
    )
    if not repo_compose.is_file():
        pytest.skip(f"repo compose not found: {repo_compose}")
    report = V1385ComposeLint().lint_file(str(repo_compose))
    # 报告应该 parse_ok=True
    assert report.parse_ok is True
    assert report.n_services >= 1
    # n_findings > 0 是预期的 (该文件用 :latest 默认值 + 没所有 memory limit)
    assert report.n_findings >= 1


def test_v1385_real_repo_compose_no_privileged_no_docker_sock():
    """V1385 真生产: 真实 repo compose 不应触发 COMPOSE-PRIVILEGED 或 COMPOSE-DOCKER-SOCK."""
    repo_compose = (
        pathlib.Path(__file__).resolve().parent.parent
        / "Apeireth-rust" / "deploy" / "docker-compose.protocols.yml"
    )
    if not repo_compose.is_file():
        pytest.skip(f"repo compose not found: {repo_compose}")
    report = V1385ComposeLint().lint_file(str(repo_compose))
    bad = [f for f in report.findings
           if f.rule_id in ("COMPOSE-PRIVILEGED", "COMPOSE-DOCKER-SOCK")]
    assert bad == [], f"unexpected security findings: {[f.rule_id for f in bad]}"


# ============================================================================
# determinism tests
# ============================================================================


def test_v1385_deterministic_lint(bad_compose, lint):
    """V1385 真生产: 同样 input → 同样 output (无随机, GUARD_DETERMINISTIC)."""
    r1 = lint.lint_text(bad_compose)
    r2 = lint.lint_text(bad_compose)
    assert r1.n_findings == r2.n_findings
    assert r1.n_errors == r2.n_errors
    assert [(f.rule_id, f.service, f.line_no) for f in r1.findings] == \
           [(f.rule_id, f.service, f.line_no) for f in r2.findings]


def test_v1385_lint_runner_reusable(lint, bad_compose, clean_compose):
    """V1385 真生产: V1385ComposeLint instance 可重复用."""
    r1 = lint.lint_text(bad_compose)
    r2 = lint.lint_text(clean_compose)
    assert r1.ok is False
    assert r2.ok is True
    # last_report 应该是最近一次
    assert lint.last_report is r2


# ============================================================================
# helper / unit tests
# ============================================================================


def test_v1385_flatten_str():
    """V1385 _flatten_str 拍平 scalar."""
    assert _flatten_str(None) == ""
    assert _flatten_str("hi") == "hi"
    assert _flatten_str(42) == "42"
    assert _flatten_str(True) == "True"


def test_v1385_collect_volume_strings_mixed():
    """V1385 _collect_volume_strings 同时处理 str 和 dict."""
    vs = _collect_volume_strings(["/a:/a", {"source": "/b", "target": "/c"}])
    assert "/a:/a" in vs
    assert any("/b" in v and "/c" in v for v in vs)


def test_v1385_build_line_map_basic():
    """V1385 _build_line_map 顶层 + services.* 行号."""
    text = (
        "version: '3.8'\n"
        "services:\n"
        "  alpha:\n"
        "    image: nginx:1.0\n"
        "  beta:\n"
        "    image: redis:7\n"
    )
    m = _build_line_map(text)
    assert m[("version",)] == 1
    assert m[("services", "alpha")] == 3
    assert m[("services", "beta")] == 5
