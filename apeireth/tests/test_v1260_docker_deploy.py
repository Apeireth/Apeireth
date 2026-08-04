"""Tests for V1260 docker_deploy (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

These tests verify that V1260 真生产:
- probe_environment returns valid ProbeResult
- build_default_stack + build_e2e_stack return valid ServiceSpec lists
- sanity_check_1260 returns all True
- real subprocess deploy on a free port + HTTP healthcheck succeeds
- graceful shutdown works
- deploy + stop cycle is idempotent within budget
"""
from __future__ import annotations

import os
import sys
import time

# Ensure in-package import works
import pytest

# Try both import styles
try:
    from apeireth import v1260_docker_deploy as v60
except Exception:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
    import v1260_docker_deploy as v60


def test_v1260_version():
    assert v60.V1260_VERSION == "0.1.0"


def test_v1260_sanity_all_true():
    sc = v60.sanity_check_1260()
    assert isinstance(sc, dict)
    assert all(sc.values()), f"failed sanity checks: {[k for k, v in sc.items() if not v]}"


def test_v1260_v3_guards_count():
    assert len(v60.V3_GUARDS) == 5
    for g in v60.V3_GUARDS:
        assert isinstance(g, str)
        assert len(g) > 0


def test_v1260_probe_environment_returns_probe():
    probe = v60.probe_environment()
    assert isinstance(probe, v60.ProbeResult)
    assert isinstance(probe.docker_available, bool)
    assert isinstance(probe.docker_compose_available, bool)
    assert isinstance(probe.python_available, bool)
    summary = v60.probe_summary_dict()
    assert "strategy" in summary
    assert summary["strategy"] in ("docker", "docker_compose", "podman", "subprocess", "wsl")


def test_v1260_build_default_stack_three_services():
    stack = v60.build_default_stack()
    assert isinstance(stack, list)
    assert len(stack) >= 3
    for svc in stack:
        assert isinstance(svc, v60.ServiceSpec)
        assert isinstance(svc.name, str)
        assert isinstance(svc.port, int)
        assert svc.port > 0


def test_v1260_build_e2e_stack_four_services():
    stack = v60.build_e2e_stack()
    assert isinstance(stack, list)
    assert len(stack) >= 4
    for svc in stack:
        assert isinstance(svc, v60.ServiceSpec)
        assert isinstance(svc.name, str)
        assert isinstance(svc.port, int)


def test_v1260_render_mini_service_app_is_python():
    app_source = v60.render_mini_service_app("test_svc", 12345)
    assert isinstance(app_source, str)
    assert "FastAPI" in app_source or "fastapi" in app_source
    assert "12345" in app_source


def test_v1260_port_is_free_helper():
    # Just verify helper runs without error
    free = v60._port_is_free(0)  # port 0 = ephemeral; should usually be free
    assert isinstance(free, bool)


def test_v1260_real_subprocess_deploy_one_service():
    """真部署 1 service + 真 HTTP healthcheck + 真 shutdown."""
    # Pick a high free port
    test_port = 18810
    while not v60._port_is_free(test_port):
        test_port += 1
    specs = v60.build_default_stack()[:1]
    specs[0].port = test_port
    try:
        result = v60.deploy_stack(specs, health_timeout=10.0)
        assert isinstance(result, v60.DeployStack)
        assert result.strategy == "subprocess"  # no docker on Windows host
        assert "apeireth_core" in result.services
        svc = result.services["apeireth_core"]
        assert svc.pid > 0
        assert svc.last_health_code == 200
        assert svc.health_success_count >= 1
        # 真 health cycle
        cycle = v60.stack_run_health_cycle(result, cycles=2, interval=0.3)
        assert isinstance(cycle, dict)
        assert cycle.get("all_healthy") is True
        # service_results 字典, key = service name
        assert "apeireth_core" in cycle["service_results"]
        sr = cycle["service_results"]["apeireth_core"]
        assert sr.get("healthy") is True
        assert sr.get("code") == 200
    finally:
        v60.stop_stack(result, timeout=3.0)


def test_v1260_stop_stack_idempotent():
    """真 graceful shutdown — 同一个 stack 多次 stop 不报错."""
    test_port = 18820
    while not v60._port_is_free(test_port):
        test_port += 1
    specs = v60.build_default_stack()[:1]
    specs[0].port = test_port
    result = v60.deploy_stack(specs, health_timeout=10.0)
    # First stop
    r1 = v60.stop_stack(result, timeout=3.0)
    assert r1["n_services"] >= 1
    # Second stop (should not crash)
    r2 = v60.stop_stack(result, timeout=3.0)
    assert isinstance(r2, dict)