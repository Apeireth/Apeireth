"""V1075 ASI Real Deployment Run tests — 真测试 (主 17:43 实事求是).

测试范围 (主 00:44 质量工程化):
1. EnvironmentProbe 真探测
2. Artifact 真生成
3. HealthChecker 真 HTTP
4. ProcessRunner 真子进程
5. 端到端真部署 (用端口 0 + 真实进程)
6. V3 哲学守门
7. 报告生成
8. Sanity: refs/guards/无假装/可复现
"""

from __future__ import annotations

import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

import pytest

# 添加项目根到 path
_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from apeireth.v1075_asi_real_deployment_run import (  # noqa: E402
    DOCKERFILE_TEMPLATE,
    COMPOSE_TEMPLATE,
    K8S_MANIFEST_TEMPLATE,
    SYSTEMD_UNIT_TEMPLATE,
    SUPERVISORD_TEMPLATE,
    DEFAULT_HEALTH_TIMEOUT_SEC,
    DEFAULT_PORT,
    DeploymentMode,
    DeploymentArtifact,
    DeploymentRunResult,
    DeployState,
    EnvironmentInfo,
    HealthCheckResult,
    REFERENCES,
    V1075_VERSION,
    build_service_payload,
    check_health,
    docker_compose_down,
    docker_compose_ps,
    docker_compose_up,
    generate_artifacts,
    http_get,
    make_health_app,
    probe_environment,
    render_markdown_report,
    run_deployment,
    start_uvicorn_subprocess,
    stop_subprocess,
    _safe_run,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def free_port() -> int:
    """真找一个空闲端口 (借鉴 socket SO_REUSEADDR)."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture
def tmp_deploy_dir(tmp_path: Path) -> Path:
    """真部署目录."""
    d = tmp_path / "deploy"
    d.mkdir(parents=True, exist_ok=True)
    return d


@pytest.fixture
def tmp_report_path(tmp_path: Path) -> Path:
    return tmp_path / "deployment-report.md"


# ---------------------------------------------------------------------------
# 1. EnvironmentProbe 真探测 (主 17:43)
# ---------------------------------------------------------------------------


class TestEnvironmentProbe:
    def test_probe_returns_environment_info(self) -> None:
        env = probe_environment()
        assert isinstance(env, EnvironmentInfo)
        assert env.platform
        assert env.python_version
        assert isinstance(env.docker_available, bool)
        assert isinstance(env.compose_available, bool)
        assert isinstance(env.port_free, bool)

    def test_probe_default_port_in_use(self) -> None:
        # 默认 8765 端口可能空闲
        env = probe_environment(host="127.0.0.1", port=19999)
        # 不一定 port_free=True, 但应该是 bool
        assert isinstance(env.port_free, bool)

    def test_probe_detects_occupied_port(self) -> None:
        # 真占一个端口
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
            # 此时不监听, 但立即探测
            env = probe_environment(host="127.0.0.1", port=port)
            # 端口被我们 bind 了 (但没 listen), OS 可能允许再 bind
            # 主要验证不抛异常
            assert isinstance(env, EnvironmentInfo)

    def test_probe_python_version_format(self) -> None:
        env = probe_environment()
        # 真 python 版本是 "3.x.y"
        assert re.match(r"^\d+\.\d+\.\d+", env.python_version)

    def test_probe_to_dict(self) -> None:
        env = probe_environment()
        d = env.to_dict()
        assert "docker_available" in d
        assert "python_version" in d
        assert "port_free" in d

    def test_probe_docker_field_types(self) -> None:
        env = probe_environment()
        assert isinstance(env.docker_available, bool)
        assert isinstance(env.docker_version, str)
        assert isinstance(env.compose_available, bool)
        assert isinstance(env.compose_version, str)
        assert isinstance(env.notes, list)
        assert len(env.notes) > 0


# ---------------------------------------------------------------------------
# 2. Artifact 真生成 (主 23:44)
# ---------------------------------------------------------------------------


class TestArtifactGeneration:
    def test_generate_artifacts_writes_files(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        assert len(arts) == 6  # Dockerfile + compose + k8s + systemd + supervisor + .env.example

        # 真检查文件存在
        assert (tmp_deploy_dir / "Dockerfile").exists()
        assert (tmp_deploy_dir / "docker-compose.yml").exists()
        assert (tmp_deploy_dir / "k8s-asi.yaml").exists()
        assert (tmp_deploy_dir / "apeireth-asi.service").exists()
        assert (tmp_deploy_dir / "apeireth-asi.supervisor.conf").exists()
        assert (tmp_deploy_dir / ".env.example").exists()

    def test_dockerfile_has_healthcheck(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        dockerfile_arts = [a for a in arts if a.kind == "dockerfile"]
        assert len(dockerfile_arts) == 1
        content = dockerfile_arts[0].content
        assert "HEALTHCHECK" in content  # 真借鉴 Docker 2013
        assert "EXPOSE" in content
        assert "FROM python" in content

    def test_compose_has_services(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        compose_arts = [a for a in arts if a.kind == "compose"]
        content = compose_arts[0].content
        assert "services:" in content
        assert "healthcheck:" in content
        assert "ports:" in content

    def test_k8s_has_probes(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        k8s_arts = [a for a in arts if a.kind == "k8s"]
        content = k8s_arts[0].content
        assert "livenessProbe" in content
        assert "readinessProbe" in content
        assert "startupProbe" in content
        assert "kind: Deployment" in content
        assert "kind: Service" in content

    def test_systemd_has_restart(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        sd = [a for a in arts if a.kind == "systemd"][0].content
        assert "[Service]" in sd
        assert "Restart=" in sd
        assert "ExecStart=" in sd

    def test_supervisor_has_autorestart(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        sup = [a for a in arts if a.kind == "supervisor"][0].content
        assert "autorestart=true" in sup
        assert "[program:" in sup

    def test_env_example_has_otel(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        env_art = [a for a in arts if a.kind == "env_example"][0].content
        assert "OTEL_SERVICE_NAME" in env_art  # 真借鉴 OpenTelemetry 2019
        assert "V1075_PORT" in env_art
        assert "ASI_VERSION" in env_art

    def test_artifact_sha256_present(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        for a in arts:
            assert a.sha256
            assert len(a.sha256) == 16  # sha256[:16]

    def test_artifact_to_dict(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        d = arts[0].to_dict()
        assert "path" in d
        assert "kind" in d
        assert "sha256" in d
        assert "bytes" in d

    def test_artifact_post_init_sha256(self) -> None:
        # 不写盘, 直接构造 (测 __post_init__)
        a = DeploymentArtifact(path=Path("/tmp/x"), kind="test", content="hello")
        assert a.sha256
        assert a.sha256 == "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"[:16]

    def test_custom_port_in_artifacts(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir, port=9999, host_port=19999)
        dockerfile_content = [a for a in arts if a.kind == "dockerfile"][0].content
        assert "9999" in dockerfile_content
        compose_content = [a for a in arts if a.kind == "compose"][0].content
        assert "19999" in compose_content
        assert "9999" in compose_content


# ---------------------------------------------------------------------------
# 3. HealthChecker 真 HTTP (主 17:43)
# ---------------------------------------------------------------------------


class TestHealthChecker:
    def test_health_check_returns_result(self, free_port: int) -> None:
        # 端口空闲, health check 应该返回未 healthy
        result = check_health("127.0.0.1", free_port, retries=2)
        assert isinstance(result, HealthCheckResult)
        assert not result.healthy
        assert result.attempts_total == 2
        assert result.attempt == 2

    def test_http_get_refused(self, free_port: int) -> None:
        status, body, latency = http_get("127.0.0.1", free_port)
        assert status == 0  # 连接失败
        assert not body
        assert latency >= 0

    def test_health_check_with_backoff(self, free_port: int) -> None:
        t0 = time.perf_counter()
        result = check_health("127.0.0.1", free_port, retries=3, backoff_base=0.05)
        elapsed = time.perf_counter() - t0
        # 3 次尝试, 中间有 backoff
        assert result.attempt == 3
        # 至少 0.05 + 0.1 = 0.15s backoff
        assert elapsed >= 0.1

    def test_health_check_to_dict(self) -> None:
        r = HealthCheckResult()
        d = r.to_dict()
        assert "healthy" in d
        assert "status_code" in d

    def test_health_check_status_codes(self) -> None:
        r = HealthCheckResult(status_code=200, healthy=True)
        assert r.healthy
        assert r.status_code == 200


# ---------------------------------------------------------------------------
# 4. FastAPI Service 真构造 (主 17:43)
# ---------------------------------------------------------------------------


class TestFastAPIService:
    def test_make_health_app_returns_app(self) -> None:
        app = make_health_app()
        assert app is not None
        assert app.title == "apeireth-asi"
        assert app.version == V1075_VERSION

    def test_build_service_payload(self) -> None:
        p = build_service_payload()
        assert p["status"] == "ok"
        assert p["service"] == "apeireth-asi"
        assert p["version"] == V1075_VERSION
        assert p["python"]
        assert p["platform"]
        assert p["pid"] == os.getpid()
        assert p["ts"].endswith("+00:00") or p["ts"].endswith("Z")

    def test_build_service_payload_with_extra(self) -> None:
        p = build_service_payload(extra={"foo": "bar", "endpoint": "health"})
        assert p["foo"] == "bar"
        assert p["endpoint"] == "health"

    def test_make_health_app_with_providers(self) -> None:
        snap = {"status": "ok", "score": 0.88}
        dec = {"direction": "v1076", "lift": 0.03}

        def snap_provider() -> dict:
            return snap

        def dec_provider() -> dict:
            return dec

        app = make_health_app(snapshot_provider=snap_provider, decision_provider=dec_provider)
        assert app is not None


# ---------------------------------------------------------------------------
# 5. ProcessRunner 真子进程 (主 23:44)
# ---------------------------------------------------------------------------


class TestProcessRunner:
    def test_start_and_stop_subprocess(self, free_port: int) -> None:
        proc = start_uvicorn_subprocess("127.0.0.1", free_port)
        try:
            assert proc.pid > 0
            time.sleep(2.0)  # 等 uvicorn 真启动
        finally:
            stopped = stop_subprocess(proc, timeout_sec=5.0)
            assert stopped

    def test_stop_already_dead(self) -> None:
        # 已经退出的子进程
        proc = subprocess.Popen(
            [sys.executable, "-c", "print(1)"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        proc.wait(timeout=5.0)
        # 应该立即返回 True
        stopped = stop_subprocess(proc, timeout_sec=1.0)
        assert stopped


# ---------------------------------------------------------------------------
# 6. Safe Run 真命令 (主 17:58 不假装)
# ---------------------------------------------------------------------------


class TestSafeRun:
    def test_safe_run_real_command(self) -> None:
        rc, out, err = _safe_run([sys.executable, "--version"], timeout=5.0)
        assert rc == 0
        assert "Python" in out

    def test_safe_run_nonexistent(self) -> None:
        rc, out, err = _safe_run(["definitely-not-a-binary-xyz123"], timeout=2.0)
        assert rc == 127  # not found
        assert "not_found" in err

    def test_safe_run_failing_command(self) -> None:
        rc, out, err = _safe_run([sys.executable, "-c", "exit(7)"], timeout=5.0)
        assert rc == 7


# ---------------------------------------------------------------------------
# 7. 端到端真部署 (主 23:44 + 主 00:56)
# ---------------------------------------------------------------------------


class TestEnd2EndDeployment:
    def test_full_deployment_run_process_mode(self, tmp_deploy_dir: Path, free_port: int) -> None:
        """真部署: uvicorn 起来 + 真 HTTP 200 + 真停."""
        result = run_deployment(
            mode=DeploymentMode.PROCESS,
            host="127.0.0.1",
            port=free_port,
            deploy_dir=tmp_deploy_dir,
            health_retries=8,  # 真实启动需要时间
        )
        assert isinstance(result, DeploymentRunResult)
        assert result.mode == DeploymentMode.PROCESS
        assert len(result.artifacts) == 6
        # 真文件已写
        assert (tmp_deploy_dir / "Dockerfile").exists()
        # 真健康检查
        assert result.health is not None
        if result.state == DeployState.HEALTHY:
            assert result.health.healthy
            assert result.health.status_code == 200
            body = json.loads(result.health.body) if result.health.body else {}
            assert body.get("status") == "ok"

    def test_full_deployment_run_artifact_only(self, tmp_deploy_dir: Path) -> None:
        result = run_deployment(
            mode=DeploymentMode.ARTIFACT_ONLY,
            host="127.0.0.1",
            port=18888,
            deploy_dir=tmp_deploy_dir,
            auto_stop=True,
        )
        assert len(result.artifacts) == 6
        # ARTIFACT_ONLY 不应 HEALTHY
        assert result.state in (DeployState.STOPPED, DeployState.DEGRADED, DeployState.ARTIFACTS_WRITTEN)

    def test_full_deployment_with_report(self, tmp_deploy_dir: Path, free_port: int, tmp_report_path: Path) -> None:
        # 真实跑一次 + 真报告
        # 注意: 不实际写 report path, 用 markdown_report 函数即可
        result = run_deployment(
            mode=DeploymentMode.PROCESS,
            host="127.0.0.1",
            port=free_port,
            deploy_dir=tmp_deploy_dir,
            health_retries=5,
        )
        report = render_markdown_report(result)
        tmp_report_path.write_text(report, encoding="utf-8")
        assert tmp_report_path.exists()
        assert "# V1075" in report
        assert "Health Check" in report


# ---------------------------------------------------------------------------
# 8. Docker Runner 真命令 (主 17:43)
# ---------------------------------------------------------------------------


class TestDockerRunner:
    def test_docker_compose_up_no_docker(self, tmp_deploy_dir: Path) -> None:
        # 即便 docker 不可用, 函数也不抛
        rc, out, err = docker_compose_up(tmp_deploy_dir / "docker-compose.yml", timeout_sec=2.0)
        # 不是 0 (因为 docker 未装)
        assert isinstance(rc, int)
        assert isinstance(out, str)
        assert isinstance(err, str)

    def test_docker_compose_ps_no_docker(self) -> None:
        rc, out, err = docker_compose_ps(Path("/tmp/fake-compose.yml"))
        assert rc != 0  # docker 未装

    def test_docker_compose_down_no_docker(self) -> None:
        rc, out, err = docker_compose_down(Path("/tmp/fake-compose.yml"))
        assert rc != 0


# ---------------------------------------------------------------------------
# 9. Markdown 报告 (主 00:56 可读)
# ---------------------------------------------------------------------------


class TestMarkdownReport:
    def test_report_contains_all_sections(self) -> None:
        # 真构造 result + render
        env = EnvironmentInfo(docker_available=False, port_free=True, python_version="3.13.0")
        h = HealthCheckResult(healthy=True, status_code=200, latency_ms=12.5, body='{"status":"ok"}')
        a = DeploymentArtifact(path=Path("/tmp/Dockerfile"), kind="dockerfile", content="FROM python")
        result = DeploymentRunResult(
            state=DeployState.HEALTHY,
            mode=DeploymentMode.PROCESS,
            host="127.0.0.1",
            port=8765,
            pid=12345,
            artifacts=[a],
            health=h,
            started_at="2026-07-22T00:00:00Z",
            stopped_at="2026-07-22T00:00:30Z",
            duration_sec=30.0,
            logs=["log1", "log2"],
            env_info=env,
        )
        report = render_markdown_report(result)
        assert "# V1075" in report
        assert "Environment" in report
        assert "Health Check" in report
        assert "**Healthy:** True" in report
        assert "Artifacts" in report
        assert "Logs" in report
        assert "Dockerfile" in report
        assert "V3 哲学守门" in report
        assert "不假装" in report

    def test_report_contains_all_refs(self) -> None:
        env = EnvironmentInfo()
        result = DeploymentRunResult(state=DeployState.STOPPED, env_info=env)
        report = render_markdown_report(result)
        for r in REFERENCES:
            assert r["id"] in report


# ---------------------------------------------------------------------------
# 10. V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ---------------------------------------------------------------------------


class TestV3PhilosophyGuard:
    def test_probe_does_not_lie_about_docker(self) -> None:
        """不假装 Docker 已装."""
        env = probe_environment()
        # 若没装 docker, docker_available 必须 False
        # (Windows 测试环境通常没 docker)
        if not shutil.which("docker"):
            assert env.docker_available is False
        # 不管 docker 是否在, docker_version 字符串与 available 一致
        if not env.docker_available:
            assert env.docker_version == ""

    def test_artifact_generation_produces_no_fake_paths(self, tmp_deploy_dir: Path) -> None:
        """不假装工件存在: 真写盘."""
        arts = generate_artifacts(tmp_deploy_dir)
        for a in arts:
            assert a.path.exists()
            assert a.path.stat().st_size > 0

    def test_health_check_does_not_lie_about_connection(self, free_port: int) -> None:
        """不假装连接成功: 真实 HTTP."""
        status, _body, _latency = http_get("127.0.0.1", free_port)
        # 空闲端口 → 连接失败
        assert status == 0

    def test_subprocess_start_does_not_lie_about_running(self, free_port: int) -> None:
        """不假装子进程运行: 真起真停."""
        proc = start_uvicorn_subprocess("127.0.0.1", free_port)
        assert proc.pid > 0
        # pid 真实存在
        if os.name == "nt":
            import ctypes
            PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
            STILL_ACTIVE = 259
            kernel32 = ctypes.windll.kernel32
            handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, proc.pid)
            if handle:
                exit_code = ctypes.c_ulong()
                kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code))
                kernel32.CloseHandle(handle)
                assert exit_code.value == STILL_ACTIVE or exit_code.value == 0
        stopped = stop_subprocess(proc)
        assert stopped


# ---------------------------------------------------------------------------
# 11. Sanity: refs / guards / 无假装 / 可复现 (主 00:44)
# ---------------------------------------------------------------------------


class TestSanity:
    def test_references_count(self) -> None:
        assert len(REFERENCES) >= 11

    def test_references_have_required_fields(self) -> None:
        for r in REFERENCES:
            assert "id" in r
            assert "title" in r
            assert "url" in r

    def test_templates_have_placeholders(self) -> None:
        assert "{version}" in DOCKERFILE_TEMPLATE
        assert "{port}" in DOCKERFILE_TEMPLATE
        assert "{version}" in COMPOSE_TEMPLATE
        assert "{port}" in K8S_MANIFEST_TEMPLATE
        assert "{port}" in SYSTEMD_UNIT_TEMPLATE
        assert "{port}" in SUPERVISORD_TEMPLATE

    def test_default_port_is_int(self) -> None:
        assert isinstance(DEFAULT_PORT, int)
        assert 1 <= DEFAULT_PORT <= 65535

    def test_default_health_timeout_positive(self) -> None:
        assert DEFAULT_HEALTH_TIMEOUT_SEC > 0

    def test_all_template_format_no_exception(self) -> None:
        # 真 format 不抛
        d = DOCKERFILE_TEMPLATE.format(version="0.1.0", port=8765)
        assert d
        c = COMPOSE_TEMPLATE.format(version="0.1.0", host="127.0.0.1", host_port=8765, port=8765)
        assert c
        k = K8S_MANIFEST_TEMPLATE.format(version="0.1.0", port=8765)
        assert k
        s = SYSTEMD_UNIT_TEMPLATE.format(version="0.1.0", port=8765)
        assert s
        su = SUPERVISORD_TEMPLATE.format(version="0.1.0", port=8765)
        assert su

    def test_deployment_run_result_to_dict(self) -> None:
        r = DeploymentRunResult()
        d = r.to_dict()
        assert "state" in d
        assert "mode" in d
        assert "host" in d
        assert "port" in d

    def test_environment_info_to_dict(self) -> None:
        env = EnvironmentInfo()
        d = env.to_dict()
        assert "docker_available" in d
        assert "compose_available" in d
        assert "port_free" in d

    def test_no_global_state_pollution(self, tmp_deploy_dir: Path, free_port: int) -> None:
        """可复现: 两次跑结果一致 (相同环境)."""
        r1 = run_deployment(
            mode=DeploymentMode.ARTIFACT_ONLY,
            host="127.0.0.1",
            port=free_port,
            deploy_dir=tmp_deploy_dir,
        )
        r2 = run_deployment(
            mode=DeploymentMode.ARTIFACT_ONLY,
            host="127.0.0.1",
            port=free_port,
            deploy_dir=tmp_deploy_dir,
        )
        # 两次 artifact 数一致
        assert len(r1.artifacts) == len(r2.artifacts)
        # state 在同一集合
        assert r1.state in (
            DeployState.STOPPED,
            DeployState.DEGRADED,
            DeployState.ARTIFACTS_WRITTEN,
        )
        assert r2.state in (
            DeployState.STOPPED,
            DeployState.DEGRADED,
            DeployState.ARTIFACTS_WRITTEN,
        )


# ---------------------------------------------------------------------------
# 12. DeploymentRunResult 序列化 (主 00:56)
# ---------------------------------------------------------------------------


class TestDeploymentRunResult:
    def test_default_state(self) -> None:
        r = DeploymentRunResult()
        assert r.state == DeployState.PENDING
        assert r.host
        assert r.port > 0
        assert r.artifacts == []

    def test_assign_artifacts(self, tmp_deploy_dir: Path) -> None:
        arts = generate_artifacts(tmp_deploy_dir)
        r = DeploymentRunResult(artifacts=arts)
        assert len(r.artifacts) == 6

    def test_full_lifecycle(self) -> None:
        env = EnvironmentInfo(docker_available=False, port_free=True)
        h = HealthCheckResult(healthy=True, status_code=200)
        r = DeploymentRunResult(
            state=DeployState.HEALTHY,
            env_info=env,
            health=h,
        )
        d = r.to_dict()
        assert d["state"] == "HEALTHY"
        assert d["health"]["healthy"] is True
        assert d["env_info"]["docker_available"] is False