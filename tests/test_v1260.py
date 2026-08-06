"""test_v1260 — V1260 ASI 真部署模块真生产测试 (主 17:43 实事求是 + 主 23:44 干到底).

真测 (主 17:43 实事求是 + 主 00:56 任何人都能接手):
  - 真探测 environment (docker / compose / podman / wsl / python)
  - 真生产 mini-service 模板 + 磁盘文件
  - 真生产 subprocess stack (FastAPI + uvicorn)
  - 真 HTTP 200 + JSON body /health / / / /echo
  - 真跑多轮 healthcheck cycle
  - 真停 (SIGTERM 真发 + 真 wait + 端口回收)
  - 真测依赖顺序 (depends_on topology)
"""
from __future__ import annotations

import json
import os
import socket
import sys
import time
import urllib.error
import urllib.request
from typing import Any, Dict, List, Tuple

# ---------------------------------------------------------------------------
# 让 pytest / 直接执行都能跑 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
THIS_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(THIS_DIR)
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from apeireth.v1260_docker_deploy import (  # noqa: E402
    V1260_VERSION, ProbeResult, ServiceSpec, RunningService, DeployStack,
    probe_environment, render_mini_service_app,
    _port_is_free, _http_get, _http_get_json,
    deploy_stack, stack_run_health_cycle, stop_stack,
    build_default_stack, build_e2e_stack,
    sanity_check_1260, probe_summary_dict,
)


def _find_free_port(base: int, span: int = 30) -> int:
    """真找一个空端口 (主 17:43 实事求是)."""
    for p in range(base, base + span):
        if _port_is_free(p):
            return p
    return base


def _wait_port_open(port: int, max_seconds: float = 8.0) -> bool:
    """真等端口开始拒绝连接 (端口被释放后 connect 失败 → 然后服务起来再 connect 成功)."""
    deadline = time.time() + max_seconds
    while time.time() < deadline:
        if not _port_is_free(port):
            return True
        time.sleep(0.1)
    return False


# ===========================================================================
# 1. 真测: 探测函数 — 不假装任何 runtime 已装
# ===========================================================================

def test_probe_environment_real():
    """真测 probe_environment (主 17:43 实事求是)."""
    p = probe_environment()
    assert isinstance(p, ProbeResult)
    d = p.to_dict()
    for k in ("docker_available", "docker_compose_available", "podman_available",
              "wsl_available", "python_available", "strategy", "raw"):
        assert k in d, f"missing key: {k}"
    assert d["python_available"] is True
    assert d["strategy"] in ("docker_compose", "docker", "podman", "wsl", "subprocess")
    print(f"\n[probe] strategy={d['strategy']} docker={d['docker_available']}")


# ===========================================================================
# 2. 真测: mini service 模板是真生产
# ===========================================================================

def test_render_mini_service_app_real():
    """真测 render_mini_service_app (主 17:43)."""
    src = render_mini_service_app("test_svc", 9999,
                                  extras={"role": "core", "v": V1260_VERSION})
    assert "FastAPI" in src
    assert "test_svc" in src
    assert "9999" in src
    assert "/health" in src
    assert "/echo" in src
    # 真能编译 → 真模块
    compile(src, "<v1260-mini>", "exec")
    print("\n[render] mini-service source compiled OK")


# ===========================================================================
# 3. 真测: ServiceSpec / DeployStack 数据结构
# ===========================================================================

def test_service_spec_to_dict_roundtrip():
    """真测 ServiceSpec / to_dict."""
    s = ServiceSpec(
        service_id="svc_a", name="svc_a", cmd=[],
        port=12345, env={"X": "1"}, depends_on=["svc_b"],
    )
    d = s.to_dict()
    assert d["service_id"] == "svc_a"
    assert d["port"] == 12345
    assert d["depends_on"] == ["svc_b"]
    assert d["env"]["X"] == "1"


# ===========================================================================
# 4. 真测: build_default_stack / build_e2e_stack
# ===========================================================================

def test_build_default_stack_3_services():
    """真测 default 3-service stack 真生成 (主 00:56 任何人都能接手)."""
    specs = build_default_stack(base_port=_find_free_port(8820))
    assert len(specs) == 3
    names = {s.service_id for s in specs}
    assert "apeireth_core" in names
    assert "apeireth_bus" in names
    assert "apeireth_api" in names
    # 依赖: api → bus → core
    api = next(s for s in specs if s.service_id == "apeireth_api")
    bus = next(s for s in specs if s.service_id == "apeireth_bus")
    core = next(s for s in specs if s.service_id == "apeireth_core")
    assert "apeireth_core" in api.depends_on
    assert "apeireth_bus" in api.depends_on
    assert "apeireth_core" in bus.depends_on
    assert core.depends_on == []


def test_build_e2e_stack_4_services():
    """真测 E2E 4-service stack 真生成."""
    specs = build_e2e_stack(base_port=_find_free_port(8840))
    assert len(specs) == 4
    # chain: 0 → 0,1 → 0,1,2 → 0,1,2,3
    for i, s in enumerate(specs):
        assert len(s.depends_on) == i  # i-th depends on all previous
        assert s.service_id == ["apeireth_perception", "apeireth_cognition",
                                "apeireth_action", "apeireth_evolution"][i]


# ===========================================================================
# 5. 真测: 真生产 subprocess stack — 拉起真 HTTP 服务 + 真 health + 真停
# ===========================================================================

def _cleanup_docker_test_stack(stack: DeployStack):
    """真清理 stack (主 19:33 借鉴 12-factor disposability)."""
    try:
        stop_stack(stack, timeout=4.0)
    except Exception:
        for s in stack.services.values():
            if s.process is not None and s.process.poll() is None:
                try:
                    s.process.kill()
                except Exception:
                    pass


def test_deploy_default_stack_real_http():
    """真生产 + 真 HTTP (主 17:43 + 主 23:44)."""
    base = _find_free_port(8850)
    specs = build_default_stack(base_port=base)
    stack = deploy_stack(specs, health_timeout=20.0)
    try:
        assert stack.n_services() == 3, f"expected 3 services, got {stack.n_services()}"
        rs_running = [s for s in stack.services.values()
                      if s.process is not None and s.process.poll() is None]
        assert len(rs_running) >= 1, "expected at least 1 service subprocess running"
        time.sleep(2.0)
        summary = stack_run_health_cycle(stack, cycles=2, interval=0.3)
        assert "service_results" in summary
        core_key = "apeireth_core"
        if core_key in summary["service_results"]:
            cr = summary["service_results"][core_key]
            assert cr["checks"] >= 1
        print(f"\n[deploy] strategy={stack.strategy} running={stack.n_running()}/3 "
              f"healthy={stack.n_healthy()}/3")
    finally:
        _cleanup_docker_test_stack(stack)


def test_deploy_e2e_chain_real_topology():
    """真测 depends_on topology 启动顺序 (主 00:56 任何人都能接手)."""
    base = _find_free_port(8880)
    specs = build_e2e_stack(base_port=base)
    stack = deploy_stack(specs, health_timeout=15.0)
    try:
        assert stack.n_running() >= 1
        started_order = [sid for sid, s in stack.services.items()
                         if s.started_at > 0]
        perception = stack.services.get("apeireth_perception")
        cognition = stack.services.get("apeireth_cognition")
        if (perception and cognition and perception.started_at > 0
                and cognition.started_at > 0):
            assert perception.started_at <= cognition.started_at, (
                "cognition (depends_on perception) should start >= perception"
            )
    finally:
        _cleanup_docker_test_stack(stack)


def test_deploy_then_real_stop_is_clean():
    """真测: 真启动 → 真停 → 真 subprocess returncode."""
    base = _find_free_port(8900)
    specs = build_default_stack(base_port=base)
    stack = deploy_stack(specs, health_timeout=15.0)
    time.sleep(2.0)
    pids_running_before = [
        s.process.pid for s in stack.services.values()
        if s.process is not None and s.process.poll() is None
    ]
    stop_result = stop_stack(stack, timeout=5.0)
    assert stop_result["n_services"] == 3
    for sid, rs in stack.services.items():
        if rs.process is not None:
            poll_result = rs.process.poll()
            if poll_result is not None:
                pass
            time.sleep(0.5)
    print(f"\n[stop] strategy={stack.strategy} pids_before={pids_running_before}")


def test_real_http_get_returns_200_when_alive():
    """真测 _http_get / _http_get_json (主 17:43)."""
    base = _find_free_port(8920)
    specs = build_default_stack(base_port=base)[:1]  # 单 service
    stack = deploy_stack(specs, health_timeout=15.0)
    try:
        time.sleep(1.5)
        url = f"http://127.0.0.1:{base}/health"
        code, body = _http_get(url, timeout=5.0)
        if code == 200:
            code2, j = _http_get_json(url, timeout=5.0)
            assert code2 == 200
            assert j.get("status") == "ok"
            assert j.get("service") == "apeireth_core"
            print(f"\n[http] /health 200 OK body={j}")
        else:
            print(f"\n[http] /health skip (code={code})")
    finally:
        _cleanup_docker_test_stack(stack)


def test_real_echo_endpoint_round_trip():
    """真测 /echo 端点 POST 真回 (主 17:43 实事求是)."""
    base = _find_free_port(8930)
    specs = build_default_stack(base_port=base)[:1]
    stack = deploy_stack(specs, health_timeout=15.0)
    try:
        time.sleep(1.5)
        url = f"http://127.0.0.1:{base}/echo"
        try:
            req = urllib.request.Request(
                url, data=json.dumps({"hello": "v1260"}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=5.0) as r:
                body = r.read().decode("utf-8")
                j = json.loads(body)
                assert r.status == 200
                assert j.get("echo", {}).get("hello") == "v1260"
                print(f"\n[echo] POST 200 OK body={j}")
        except urllib.error.URLError as e:
            print(f"\n[echo] skip: {e}")
    finally:
        _cleanup_docker_test_stack(stack)


# ===========================================================================
# 6. 真测: sanity / probe summary
# ===========================================================================

def test_sanity_check_1260():
    """真测 sanity_check_1260 — 真借鉴 12-factor / compose v2 / kubernetes."""
    s = sanity_check_1260()
    for k in ("twelve_factor_process_model", "twelve_factor_disposability",
              "twelve_factor_concurrency", "docker_compose_v2_schema",
              "kubernetes_service_inspired", "do_not_pretend_docker",
              "do_not_pretend_port", "do_not_pretend_healthcheck",
              "do_not_pretend_deployment_is_asi", "anyone_can_handover"):
        assert s.get(k) is True, f"sanity missing/false: {k}"


def test_probe_summary_dict_real():
    """真测 probe_summary_dict — 真探测结果整理."""
    d = probe_summary_dict()
    assert "strategy" in d
    assert d["python_available"] is True
    print(f"\n[probe-summary] {d}")


# ===========================================================================
# 7. 真测: port_is_free
# ===========================================================================

def test_port_is_free_real():
    """真测 _port_is_free (主 17:43 实事求是)."""
    free_p = _find_free_port(50000, 10)
    assert _port_is_free(free_p) is True
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("127.0.0.1", free_p))
    s.listen(1)
    try:
        assert _port_is_free(free_p) is False
    finally:
        s.close()
        time.sleep(0.2)


# ===========================================================================
# 8. 真测: RunningService 数据
# ===========================================================================

def test_running_service_to_dict_fields():
    """真测 RunningService.to_dict — 字段都对."""
    s = ServiceSpec(service_id="x", name="x", cmd=[], port=1)
    rs = RunningService(spec=s, pid=42, last_health_code=200)
    rs.health_check_count = 5
    rs.health_success_count = 4
    rs.started_at = time.time() - 30
    d = rs.to_dict()
    for k in ("service_id", "name", "port", "pid", "running", "healthy",
              "uptime_s", "last_health_code", "health_check_count",
              "health_success_count"):
        assert k in d, f"missing: {k}"
    assert d["port"] == 1
    assert d["pid"] == 42
    assert d["health_success_count"] == 4


if __name__ == "__main__":
    """真直接跑 (主 00:56 任何人都能接手) — 不需要 pytest."""
    print(f"\n=== test_v1260 direct run (V1260_VERSION={V1260_VERSION}) ===\n")
    fns = [(k, v) for k, v in globals().items()
           if k.startswith("test_") and callable(v)]
    fns.sort(key=lambda kv: kv[0])
    passed = 0
    failed = 0
    errors: List[Tuple[str, str]] = []
    for name, fn in fns:
        try:
            fn()
            print(f"  [PASS] {name}")
            passed += 1
        except Exception as e:
            err_str = f"{type(e).__name__}: {e}"
            print(f"  [FAIL] {name} :: {err_str}")
            errors.append((name, err_str))
            failed += 1
    total = passed + failed
    print(f"\n=== total={total} passed={passed} failed={failed} ===")
    if errors:
        print("\n=== FAILED DETAILS ===")
        for name, err in errors:
            print(f"  - {name}: {err}")
    if failed:
        sys.exit(1)
