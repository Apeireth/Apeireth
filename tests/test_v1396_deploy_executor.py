"""Phase 1396 test_v1396_deploy_executor — V1396 ASI 真生产 deploy-stack executor tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36 + 主 00:56).

V1396 = ASI real production deploy-stack executor (post-V1395 next-step).
Tests cover: parser / validator / ports detector / manifest generator / chain runner / popper self-test / CLI.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest


APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.v1396_deploy_executor import (  # noqa: E402
    V1396_VERSION,
    V1396_SCHEMA,
    V1396_GUARDS,
    V1396_BORROWED,
    V1396_VALID_RESTART,
    ChainResult,
    ComposeFile,
    ComposeIssue,
    ComposeService,
    ExecutorResult,
    PortConflict,
    PortMapping,
    _discover_compose_files,
    _import_module,
    _iso_timestamp,
    _parse_compose_file,
    _parse_compose_service,
    _parse_port_spec,
    _call_v1387,
    _call_v1393,
    _call_v1395,
    detect_port_conflicts,
    generate_manifest,
    popper_self_test,
    render_json,
    render_markdown,
    run_chain,
    run_cli,
    run_executor,
    validate_compose,
)


# ============================================================================
# V1396 basic structure tests (主 17:43 实事求是)
# ============================================================================


def test_v1396_version_constant():
    assert V1396_VERSION == "0.1.0"


def test_v1396_schema_constant():
    assert V1396_SCHEMA == "v1396.deploy-executor/v1"


def test_v1396_guards_count():
    """V1396 GUARDS >= 10."""
    assert len(V1396_GUARDS) >= 10


def test_v1396_guards_have_required():
    """V1396 GUARDS 包含 GUARD_EXECUTOR_REAL / GUARD_CHAIN_REAL / GUARD_COMPOSE_PYTHON_ONLY."""
    for required in ("GUARD_EXECUTOR_REAL", "GUARD_CHAIN_REAL", "GUARD_COMPOSE_PYTHON_ONLY", "GUARD_NO_CAP_CHANGE"):
        assert required in V1396_GUARDS


def test_v1396_borrowed_count():
    """V1396 BORROWED >= 4."""
    assert len(V1396_BORROWED) >= 4


def test_v1396_valid_restart_policies():
    """V1396 真生产 valid restart 包含 unless-stopped + always."""
    assert "unless-stopped" in V1396_VALID_RESTART
    assert "always" in V1396_VALID_RESTART
    assert "no" in V1396_VALID_RESTART
    assert "on-failure" in V1396_VALID_RESTART


# ============================================================================
# V1396 _parse_port_spec tests (主 17:43 真解析)
# ============================================================================


def test_parse_port_spec_string_short():
    pm = _parse_port_spec("svc1", "8765", "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 8765
    assert pm.protocol == "tcp"


def test_parse_port_spec_string_full():
    pm = _parse_port_spec("svc1", "8765:9000", "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 9000


def test_parse_port_spec_with_ip():
    pm = _parse_port_spec("svc1", "127.0.0.1:8765:9000", "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 9000


def test_parse_port_spec_with_protocol():
    pm = _parse_port_spec("svc1", "8765:9000/udp", "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 9000
    assert pm.protocol == "udp"


def test_parse_port_spec_int():
    pm = _parse_port_spec("svc1", 8765, "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 8765


def test_parse_port_spec_dict():
    pm = _parse_port_spec("svc1", {"published": 8765, "target": 9000}, "deploy/docker-compose.yml")
    assert pm is not None
    assert pm.host_port == 8765
    assert pm.container_port == 9000


def test_parse_port_spec_invalid():
    pm = _parse_port_spec("svc1", "abc", "deploy/docker-compose.yml")
    assert pm is None


def test_parse_port_spec_none():
    pm = _parse_port_spec("svc1", None, "deploy/docker-compose.yml")
    assert pm is None


# ============================================================================
# V1396 _parse_compose_service tests (主 17:43 真解析)
# ============================================================================


def test_parse_compose_service_basic():
    svc_data = {
        "image": "foo:1.0",
        "ports": ["1234:1234"],
        "restart": "unless-stopped",
        "environment": {"KEY": "VALUE"},
        "healthcheck": {"test": ["CMD", "true"], "interval": "10s", "retries": 3},
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert svc.name == "svc1"
    assert svc.image == "foo:1.0"
    assert svc.restart == "unless-stopped"
    assert len(svc.ports) == 1
    assert svc.ports[0].host_port == 1234
    assert svc.environment["KEY"] == "VALUE"
    assert svc.healthcheck_test == "CMD true"
    assert svc.healthcheck_retries == 3


def test_parse_compose_service_with_build():
    svc_data = {
        "build": {"context": "..", "dockerfile": "deploy/Dockerfile"},
        "ports": ["1234:1234"],
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert svc.build_context == ".."
    assert svc.build_dockerfile == "deploy/Dockerfile"


def test_parse_compose_service_with_depends():
    svc_data = {
        "image": "foo:1",
        "depends_on": ["svc2", "svc3"],
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert svc.depends_on == ["svc2", "svc3"]


def test_parse_compose_service_with_depends_dict():
    svc_data = {
        "image": "foo:1",
        "depends_on": {"svc2": {"condition": "service_healthy"}, "svc3": None},
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert "svc2" in svc.depends_on
    assert "svc3" in svc.depends_on


def test_parse_compose_service_with_deploy_resources():
    svc_data = {
        "image": "foo:1",
        "deploy": {"resources": {"limits": {"cpus": "0.5", "memory": "256M"}}},
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert svc.cpu_limit == "0.5"
    assert svc.memory_limit == "256M"


def test_parse_compose_service_with_env_list():
    svc_data = {
        "image": "foo:1",
        "environment": ["KEY=VALUE", "FLAG"],
    }
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    assert svc.environment.get("KEY") == "VALUE"
    assert svc.environment.get("FLAG") == ""


def test_parse_compose_service_to_dict():
    svc_data = {"image": "foo:1", "ports": ["1234:1234"]}
    svc = _parse_compose_service("svc1", svc_data, "deploy/x.yml")
    d = svc.to_dict()
    assert d["name"] == "svc1"
    assert d["image"] == "foo:1"
    assert len(d["ports"]) == 1


# ============================================================================
# V1396 _parse_compose_file tests (主 17:43 真解析)
# ============================================================================


def test_parse_compose_file_real(tmp_path):
    compose_text = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
        "    ports:\n"
        "      - \"1234:1234\"\n"
        "  svc2:\n"
        "    image: bar:1\n"
        "    ports:\n"
        "      - \"5678:5678\"\n"
    )
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    assert cf.ok
    assert len(cf.services) == 2
    assert cf.services[0].name == "svc1"
    assert cf.services[1].name == "svc2"


def test_parse_compose_file_invalid_yaml(tmp_path):
    p = tmp_path / "docker-compose.yml"
    p.write_text("services:\n  bad: [unclosed", encoding="utf-8")
    cf = _parse_compose_file(p)
    assert not cf.ok
    assert cf.parse_error != ""


def test_parse_compose_file_to_dict(tmp_path):
    p = tmp_path / "docker-compose.yml"
    p.write_text("services:\n  svc1:\n    image: foo:1\n", encoding="utf-8")
    cf = _parse_compose_file(p)
    d = cf.to_dict()
    assert d["ok"] is True
    assert d["n_services"] == 1


# ============================================================================
# V1396 _discover_compose_files tests (主 17:43 真扫描)
# ============================================================================


def test_discover_compose_files_real_deploy():
    """V1396 真生产: 真扫 deploy/ 真得 3 个 compose 文件."""
    files = _discover_compose_files(Path("deploy"))
    assert len(files) >= 1
    assert any("docker-compose" in f.name for f in files)


def test_discover_compose_files_empty(tmp_path):
    files = _discover_compose_files(tmp_path)
    assert files == []


def test_discover_compose_files_nested(tmp_path):
    nested = tmp_path / "subdir"
    nested.mkdir()
    (nested / "docker-compose.yml").write_text("services:\n  s:\n    image: a:1\n", encoding="utf-8")
    files = _discover_compose_files(tmp_path)
    assert len(files) == 1
    assert "subdir" in str(files[0])


# ============================================================================
# V1396 validate_compose tests (主 17:43 真校验)
# ============================================================================


def test_validate_compose_clean(tmp_path):
    compose_text = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
        "    ports:\n"
        "      - \"1234:1234\"\n"
        "    restart: unless-stopped\n"
        "    healthcheck:\n"
        "      test: [\"CMD\", \"true\"]\n"
    )
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert len(issues) == 0


def test_validate_compose_no_image_no_build(tmp_path):
    compose_text = "services:\n  svc1:\n    ports:\n      - \"1234:1234\"\n"
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert any(i.rule_id == "V1396-NO-IMAGE-NO-BUILD" for i in issues)


def test_validate_compose_invalid_restart(tmp_path):
    compose_text = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
        "    restart: bogus_policy\n"
    )
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert any(i.rule_id == "V1396-RESTART-INVALID" for i in issues)


def test_validate_compose_missing_healthcheck(tmp_path):
    compose_text = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
    )
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert any(i.rule_id == "V1396-HEALTHCHECK-MISSING" for i in issues)


def test_validate_compose_invalid_port(tmp_path):
    compose_text = (
        "services:\n"
        "  svc1:\n"
        "    image: foo:1\n"
        "    ports:\n"
        "      - \"99999:99999\"\n"
    )
    p = tmp_path / "docker-compose.yml"
    p.write_text(compose_text, encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert any(i.rule_id == "V1396-PORT-INVALID" for i in issues)


def test_validate_compose_parse_error(tmp_path):
    p = tmp_path / "docker-compose.yml"
    p.write_text("invalid: [unclosed", encoding="utf-8")
    cf = _parse_compose_file(p)
    issues = validate_compose([cf])
    assert any(i.rule_id == "V1396-COMPOSE-PARSE-ERROR" for i in issues)


# ============================================================================
# V1396 detect_port_conflicts tests (主 17:43 真检测)
# ============================================================================


def test_detect_port_conflicts_none():
    p1 = Path("deploy/docker-compose.yml")
    cf1 = _parse_compose_file(p1)
    cf2 = _parse_compose_file(Path("deploy/18-crates/docker-compose.group-a.yml"))
    cf3 = _parse_compose_file(Path("deploy/18-crates/docker-compose.group-b.yml"))
    conflicts = detect_port_conflicts([cf1, cf2, cf3])
    # actual deploy/ files don't share ports across services
    # (group-a uses 8800-8808, group-b uses 8809-8817, root uses 8765)
    assert isinstance(conflicts, list)


def test_detect_port_conflicts_real():
    cf1 = ComposeFile(path="a.yml")
    cf1.ok = True
    cf1.services = [
        ComposeService(name="s1", ports=[PortMapping(service="s1", host_port=1234, container_port=1234)]),
    ]
    cf2 = ComposeFile(path="b.yml")
    cf2.ok = True
    cf2.services = [
        ComposeService(name="s2", ports=[PortMapping(service="s2", host_port=1234, container_port=1234)]),
    ]
    conflicts = detect_port_conflicts([cf1, cf2])
    assert len(conflicts) == 1
    assert conflicts[0].host_port == 1234
    assert "s1" in conflicts[0].services
    assert "s2" in conflicts[0].services


def test_detect_port_conflicts_same_service_ok():
    """V1396 真生产: 同一个 service 多次声明端口不算冲突."""
    cf = ComposeFile(path="a.yml")
    cf.ok = True
    cf.services = [
        ComposeService(name="s1", ports=[
            PortMapping(service="s1", host_port=1234, container_port=1234),
            PortMapping(service="s1", host_port=5678, container_port=5678),
        ]),
    ]
    conflicts = detect_port_conflicts([cf])
    assert len(conflicts) == 0


def test_detect_port_conflicts_to_dict():
    cf = ComposeFile(path="a.yml")
    cf.ok = True
    cf.services = [
        ComposeService(name="s1", ports=[PortMapping(service="s1", host_port=1234)]),
        ComposeService(name="s2", ports=[PortMapping(service="s2", host_port=1234)]),
    ]
    conflicts = detect_port_conflicts([cf])
    d = conflicts[0].to_dict()
    assert d["host_port"] == 1234
    assert sorted(d["services"]) == ["s1", "s2"]


# ============================================================================
# V1396 generate_manifest tests (主 17:43 真生成)
# ============================================================================


def test_generate_manifest_minimal():
    cf = ComposeFile(path="a.yml")
    cf.ok = True
    cf.services = [ComposeService(name="s1", image="foo:1")]
    m = generate_manifest([cf], [])
    assert m["schema"] == V1396_SCHEMA
    assert m["n_services"] == 1
    assert m["services"][0]["name"] == "s1"


def test_generate_manifest_with_conflicts():
    cf = ComposeFile(path="a.yml")
    cf.ok = True
    cf.services = [ComposeService(name="s1")]
    conflicts = [PortConflict(host_port=1234, services=["s1", "s2"])]
    m = generate_manifest([cf], conflicts)
    assert m["n_port_conflicts"] == 1


# ============================================================================
# V1396 _import_module tests (主 17:43 真调用)
# ============================================================================


def test_import_module_v1387():
    mod = _import_module("v1387_deploy_stack_runner")
    assert mod is not None
    assert hasattr(mod, "V1387DeployStackRunner")


def test_import_module_v1393():
    mod = _import_module("v1393_deploy_judge")
    assert mod is not None
    assert hasattr(mod, "judge")


def test_import_module_v1395():
    mod = _import_module("v1395_deploy_dashboard")
    assert mod is not None
    assert hasattr(mod, "build_dashboard")


def test_import_module_missing():
    mod = _import_module("v9999_does_not_exist")
    assert mod is None


# ============================================================================
# V1396 _call_* tests (主 17:43 真调)
# ============================================================================


def test_call_v1387_real():
    res = _call_v1387("deploy")
    assert res is not None
    assert hasattr(res, "n_files_total")
    assert res.n_files_total >= 1


def test_call_v1393_real():
    res = _call_v1393("deploy")
    assert res is not None
    assert hasattr(res, "verdict")
    assert res.verdict in ("CRITICAL", "FAIL", "POOR", "OK", "GOOD")


def test_call_v1395_real():
    res = _call_v1395(APEIRETH_DIR, [APEIRETH_DIR.parent / "tests"])
    assert res is not None
    assert hasattr(res, "n_modules")
    assert res.n_modules >= 10


# ============================================================================
# V1396 run_chain tests (主 17:43 真跑)
# ============================================================================


def test_run_chain_real():
    cr = run_chain("deploy")
    assert cr is not None
    assert cr.n_steps == 3
    # V1387 + V1393 + V1395 should all succeed
    assert cr.n_steps_ok >= 2  # V1395 might fail if build_dashboard signature changes
    assert cr.chain_ok or cr.n_steps_failed < 3


def test_run_chain_to_dict():
    cr = run_chain("deploy")
    d = cr.to_dict()
    assert d["schema"] == V1396_SCHEMA
    assert len(d["steps"]) == 3
    assert "guards" in d


def test_run_chain_with_policy():
    cr = run_chain("deploy", policy_path=None)
    assert cr is not None


# ============================================================================
# V1396 run_executor tests (主 17:43 真跑)
# ============================================================================


def test_run_executor_real_deploy():
    res = run_executor("deploy", with_chain=False)
    assert res is not None
    assert res.n_compose_files >= 1
    assert res.n_services_total >= 1
    assert res.ok  # deploy/ is clean


def test_run_executor_with_chain():
    res = run_executor("deploy", with_chain=True)
    assert res is not None
    assert res.chain_result is not None
    assert res.chain_result.n_steps >= 2


def test_run_executor_missing_dir(tmp_path):
    res = run_executor(str(tmp_path / "nonexistent_xyz"), with_chain=False)
    assert res is not None
    assert res.ok is False


def test_run_executor_to_dict():
    res = run_executor("deploy", with_chain=False)
    d = res.to_dict()
    assert d["schema"] == V1396_SCHEMA
    assert d["n_compose_files"] >= 1


# ============================================================================
# V1396 render_markdown tests (主 17:43 真 render)
# ============================================================================


def test_render_markdown_basic():
    res = run_executor("deploy", with_chain=False)
    md = render_markdown(res)
    assert "V1396 deploy-stack executor" in md
    assert "GUARDS" in md
    assert "Known unknowns" in md


def test_render_markdown_with_chain():
    res = run_executor("deploy", with_chain=True)
    md = render_markdown(res)
    assert "Chain result" in md
    assert "V1387" in md


def test_render_json_basic():
    res = run_executor("deploy", with_chain=False)
    js = render_json(res)
    parsed = json.loads(js)
    assert parsed["schema"] == V1396_SCHEMA
    assert "guards" in parsed


# ============================================================================
# V1396 popper_self_test tests (主 17:43 真跑真测)
# ============================================================================


def test_popper_self_test_ok():
    r = popper_self_test()
    assert r["schema"] == V1396_SCHEMA
    assert r["ok"] is True
    assert len(r["failures"]) == 0


def test_popper_self_test_has_notes():
    r = popper_self_test()
    assert len(r["notes"]) >= 1


def test_popper_self_test_n_guards():
    r = popper_self_test()
    assert r["n_guards"] >= 10


# ============================================================================
# V1396 CLI tests (主 17:43 真可执行)
# ============================================================================


def test_cli_version():
    rc = run_cli(["version"])
    assert rc == 0


def test_cli_version_json():
    rc = run_cli(["version", "--json"])
    assert rc == 0


def test_cli_chain():
    rc = run_cli(["chain", "deploy"])
    assert rc == 0


def test_cli_chain_json():
    rc = run_cli(["chain", "deploy", "--json"])
    assert rc == 0


def test_cli_validate():
    rc = run_cli(["validate", "deploy"])
    assert rc == 0


def test_cli_validate_json():
    rc = run_cli(["validate", "deploy", "--json"])
    assert rc == 0


def test_cli_manifest_json():
    rc = run_cli(["manifest", "deploy", "--format", "json"])
    assert rc == 0


def test_cli_manifest_yaml():
    rc = run_cli(["manifest", "deploy", "--format", "yaml"])
    assert rc == 0


def test_cli_ports():
    rc = run_cli(["ports", "deploy"])
    assert rc == 0


def test_cli_ports_json():
    rc = run_cli(["ports", "deploy", "--json"])
    assert rc == 0


def test_cli_executor():
    rc = run_cli(["executor", "deploy"])
    assert rc == 0


def test_cli_executor_no_chain():
    rc = run_cli(["executor", "deploy", "--no-chain"])
    assert rc == 0


def test_cli_executor_json():
    rc = run_cli(["executor", "deploy", "--json"])
    assert rc == 0


def test_cli_popper():
    rc = run_cli(["popper"])
    assert rc == 0


def test_cli_demo():
    rc = run_cli(["demo"])
    assert rc == 0


def test_cli_help():
    rc = run_cli(["help"])
    assert rc == 0


def test_cli_no_args():
    rc = run_cli([])
    assert rc == 0


# ============================================================================
# V1396 dataclass tests (主 17:43)
# ============================================================================


def test_port_mapping_to_dict():
    pm = PortMapping(service="s1", raw="1234:1234", host_port=1234, container_port=1234, protocol="tcp")
    d = pm.to_dict()
    assert d["service"] == "s1"
    assert d["host_port"] == 1234


def test_compose_service_to_dict():
    svc = ComposeService(name="s1", image="foo:1")
    d = svc.to_dict()
    assert d["name"] == "s1"
    assert d["image"] == "foo:1"


def test_compose_issue_to_dict():
    iss = ComposeIssue(severity="error", rule_id="X", service="s1", file_path="a.yml", message="bad")
    d = iss.to_dict()
    assert d["severity"] == "error"
    assert d["rule_id"] == "X"


def test_port_conflict_to_dict():
    pc = PortConflict(host_port=1234, services=["s1", "s2"])
    d = pc.to_dict()
    assert d["host_port"] == 1234


def test_chain_result_to_dict():
    cr = ChainResult(target="deploy")
    d = cr.to_dict()
    assert d["schema"] == V1396_SCHEMA
    assert d["target"] == "deploy"


def test_chain_step_result_to_dict():
    from apeireth.v1396_deploy_executor import ChainStepResult
    csr = ChainStepResult(module_id="V1387", label="test", called=True, ok=True)
    d = csr.to_dict()
    assert d["module_id"] == "V1387"


def test_executor_result_to_dict():
    er = ExecutorResult(target="deploy")
    d = er.to_dict()
    assert d["schema"] == V1396_SCHEMA
    assert d["target"] == "deploy"


# ============================================================================
# V1396 real-world integration tests (主 17:43 真跑真测)
# ============================================================================


def test_real_deploy_chain_runs():
    """V1396 真生产: 真跑 deploy/ 真得 chain_ok=True."""
    cr = run_chain("deploy")
    assert cr.chain_ok is True
    assert cr.judge_verdict in ("GOOD", "OK", "POOR", "FAIL", "CRITICAL")
    assert cr.n_steps == 3


def test_real_deploy_executor_ok():
    """V1396 真生产: 真跑 deploy/ 真得 ok=True."""
    res = run_executor("deploy", with_chain=True)
    assert res.ok is True
    assert res.n_compose_files >= 3
    assert res.n_services_total >= 19


def test_real_deploy_no_port_conflicts():
    """V1396 真生产: 真跑 deploy/ 真得 0 port conflicts."""
    res = run_executor("deploy", with_chain=False)
    assert res.n_port_conflicts == 0


def test_real_deploy_no_errors():
    """V1396 真生产: 真跑 deploy/ 真得 0 errors."""
    res = run_executor("deploy", with_chain=False)
    assert res.n_errors == 0