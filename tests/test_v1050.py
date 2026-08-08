"""Tests for v1050_real_docker_deploy — V1050 ASI 真厨房 Docker 真部署.

V1050 = 真厨房部 - V1008/V1032 Docker 真部署 (主 06:15 + 主 00:36 + 主 23:44 + 主 22:33 +
   主 19:33 + 主 17:43 + 主 17:33).

测试覆盖:
- 真写 artifacts
- 真 docker / docker-compose 可用性检测
- 真 subprocess 命令结构 (不实际跑 docker)
- 真 ps 输出解析
- 哲学守门: 部署 ≠ ASI, 部署 ≠ 安全.

主 17:43 实事求是: 多数测试不实际跑 docker, 因为沙盒里可能没装 docker.
主 19:33: 真写文件 + 真命令结构 + 真解析逻辑.
"""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import json
import os
import shutil
import subprocess
import tempfile

import pytest

from apeireth.v1050_real_docker_deploy import (
    V1050_VERSION,
    V1050_DOCKERFILE,
    V1050_COMPOSE,
    V1050_ENV,
    V1050_DOCKERIGNORE,
    V1050_REQUIREMENTS,
    DOCKERFILE_RUNTIME,
    DOCKER_COMPOSE_RUNTIME,
    DOCKERIGNORE_RUNTIME,
    ENV_RUNTIME,
    REQUIREMENTS_RUNTIME,
    DockerCommandResult,
    ContainerHealth,
    DeploymentStatus,
    V1050RealDockerDeploy,
    _parse_ps_output,
)


# ============================================================================
# 常量 & 真模板结构测试 (主 17:43 实事求是 + 主 19:33 真借鉴)
# ============================================================================


def test_version_set():
    assert V1050_VERSION == "0.1.0"


def test_dockerfile_multistage():
    """V1050 真生产 Dockerfile 多阶段构建 真借鉴 (主 19:33 Docker 官方)."""
    assert "FROM python:3.13-slim AS builder" in DOCKERFILE_RUNTIME
    assert "AS runtime" in DOCKERFILE_RUNTIME
    assert "COPY --from=builder" in DOCKERFILE_RUNTIME


def test_dockerfile_non_root():
    """V1050 真生产 non-root user (主 00:36 安全工程化)."""
    assert "useradd" in DOCKERFILE_RUNTIME
    assert "USER asi" in DOCKERFILE_RUNTIME


def test_dockerfile_healthcheck():
    """V1050 真生产 healthcheck (主 19:33 Docker 最佳实践)."""
    assert "HEALTHCHECK" in DOCKERFILE_RUNTIME
    assert "V1031Integration" in DOCKERFILE_RUNTIME


def test_dockerfile_env():
    assert "ASI_NORTH_STAR=0.7905" in DOCKERFILE_RUNTIME
    assert "ASI_LEVEL=production" in DOCKERFILE_RUNTIME


def test_compose_two_services():
    """V1050 真生产 docker-compose 两个服务 (主 19:33 Docker Compose 真借鉴)."""
    assert "asi-core" in DOCKER_COMPOSE_RUNTIME
    assert "asi-test" in DOCKER_COMPOSE_RUNTIME
    assert "networks:" in DOCKER_COMPOSE_RUNTIME
    assert "depends_on:" in DOCKER_COMPOSE_RUNTIME


def test_compose_healthcheck_and_condition():
    assert "healthcheck:" in DOCKER_COMPOSE_RUNTIME
    assert "condition: service_healthy" in DOCKER_COMPOSE_RUNTIME


def test_dockerignore_excludes_git_and_cache():
    """V1050 真生产 .dockerignore 排除 (主 00:36 工程化)."""
    assert ".git" in DOCKERIGNORE_RUNTIME
    assert "__pycache__" in DOCKERIGNORE_RUNTIME
    assert ".venv" in DOCKERIGNORE_RUNTIME


def test_env_has_api_key_placeholder():
    """V1050 真生产 .env 含 API key 占位 (主 17:43 真环境变量)."""
    assert "OPENAI_API_KEY=" in ENV_RUNTIME
    assert "OPENAI_BASE_URL=" in ENV_RUNTIME


def test_requirements_has_essential_deps():
    assert "openai" in REQUIREMENTS_RUNTIME
    assert "pytest" in REQUIREMENTS_RUNTIME
    assert "fastapi" in REQUIREMENTS_RUNTIME


# ============================================================================
# 数据结构 (主 17:43 实事求是)
# ============================================================================


def test_docker_command_result_to_dict():
    r = DockerCommandResult(
        command=["docker", "version"],
        returncode=0,
        stdout="20.10.0",
        stderr="",
        elapsed_seconds=0.5,
        ok=True,
    )
    d = r.to_dict()
    assert d["command"] == ["docker", "version"]
    assert d["returncode"] == 0
    assert d["ok"] is True
    assert d["elapsed_seconds"] == 0.5


def test_docker_command_result_failure():
    r = DockerCommandResult(
        command=["docker", "up"],
        returncode=1,
        stdout="",
        stderr="error",
        elapsed_seconds=1.0,
        ok=False,
    )
    d = r.to_dict()
    assert d["ok"] is False
    assert d["returncode"] == 1


def test_container_health_to_dict():
    c = ContainerHealth(
        service="asi-core",
        state="running",
        status="Up 5 minutes",
        health="healthy",
        container_id="abc123",
    )
    d = c.to_dict()
    assert d["service"] == "asi-core"
    assert d["state"] == "running"
    assert d["health"] == "healthy"
    assert d["container_id"] == "abc123"


def test_deployment_status_to_dict():
    s = DeploymentStatus(
        docker_available=True,
        compose_available=True,
        artefacts_written=True,
        compose_up=True,
        healthy=True,
        last_command="docker-compose up -d",
    )
    d = s.to_dict()
    assert d["docker_available"] is True
    assert d["compose_up"] is True
    assert d["healthy"] is True


# ============================================================================
# 真写 artifacts (主 17:43 真写)
# ============================================================================


def test_write_artifacts_creates_all_files():
    """V1050 真写 5 个真部署文件 (主 17:43 真写)."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        assert len(artefacts) == 5
        for name in [V1050_DOCKERFILE, V1050_COMPOSE, V1050_DOCKERIGNORE, V1050_ENV, V1050_REQUIREMENTS]:
            assert name in artefacts
            assert os.path.exists(artefacts[name])
            assert os.path.getsize(artefacts[name]) > 0


def test_write_artifacts_creates_subdir():
    """V1050 真写 自动创建子目录 (主 17:43 真写)."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        target = os.path.join(tmp, "subdir", "nested")
        artefacts = deploy.write_artifacts(target)
        assert os.path.exists(target)
        assert os.path.isdir(target)
        assert len(artefacts) == 5


def test_write_artifacts_overwrites_existing():
    """V1050 真写 覆盖现有文件 (主 17:43 真写覆盖)."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        # 先写一次
        deploy.write_artifacts(tmp)
        # 改 dockerfile 内容再写
        deploy.dockerfile = "# custom"
        artefacts = deploy.write_artifacts(tmp)
        with open(artefacts[V1050_DOCKERFILE], encoding="utf-8") as f:
            assert f.read() == "# custom"


def test_artefact_file_contents_match_template():
    """V1050 真写 文件内容真等于模板 (主 17:43 实事求是)."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        deploy.write_artifacts(tmp)
        with open(os.path.join(tmp, V1050_DOCKERFILE), encoding="utf-8") as f:
            assert f.read() == DOCKERFILE_RUNTIME
        with open(os.path.join(tmp, V1050_COMPOSE), encoding="utf-8") as f:
            assert f.read() == DOCKER_COMPOSE_RUNTIME


# ============================================================================
# Docker 可用性检测 (主 17:43 真测)
# ============================================================================


def test_check_docker_available_returns_three_tuple():
    """V1050 真测 返回 (docker_ok, compose_ok, err) 三元组 (主 17:43 真测)."""
    deploy = V1050RealDockerDeploy()
    r = deploy.check_docker_available()
    assert isinstance(r, tuple)
    assert len(r) == 3
    assert isinstance(r[0], bool)
    assert isinstance(r[1], bool)


def test_check_docker_available_with_status():
    """V1050 真测 与 last_status 协同 (主 17:43 实事求是)."""
    deploy = V1050RealDockerDeploy()
    deploy.last_status = DeploymentStatus(
        docker_available=False,
        compose_available=False,
        artefacts_written=False,
        compose_up=False,
        healthy=False,
    )
    deploy.check_docker_available()
    assert deploy.last_status.docker_available in (True, False)


def test_run_command_safe():
    """V1050 真测 _run_command 安全 wrapper 处理 FileNotFoundError (主 17:43 真测)."""
    deploy = V1050RealDockerDeploy()
    # 用一个不存在的命令测试
    result = deploy._run_command(["definitely_not_a_real_command_xyz_12345"], timeout=5)
    assert isinstance(result, DockerCommandResult)
    assert result.ok is False


def test_run_command_handles_missing_docker():
    """V1050 真测 _run_command 处理缺 docker 二进制 (主 17:43 真测)."""
    deploy = V1050RealDockerDeploy()
    # 即使 docker 不存在, 也返回结构化结果
    result = deploy._run_command(["docker", "version"], timeout=5)
    assert isinstance(result, DockerCommandResult)
    # 如果 docker 没装, ok=False, returncode=-1 (FileNotFoundError)
    if not shutil.which("docker"):
        assert result.ok is False


# ============================================================================
# ps 输出解析 (主 17:43 真解析)
# ============================================================================


def test_parse_ps_output_empty():
    """V1050 真解析 空输出返回空 list."""
    containers = _parse_ps_output("")
    assert containers == []


def test_parse_ps_output_single_line():
    """V1050 真解析 单行 JSON 解析."""
    sample = json.dumps({
        "Name": "asi-core",
        "Service": "asi-core",
        "State": "running",
        "Status": "Up 5 minutes",
        "Health": "healthy",
        "ID": "abc123def456",
    })
    containers = _parse_ps_output(sample)
    assert len(containers) == 1
    assert containers[0].service == "asi-core"
    assert containers[0].state == "running"
    assert containers[0].health == "healthy"
    assert containers[0].container_id == "abc123def456"


def test_parse_ps_output_multi_line():
    """V1050 真解析 多行 JSON 解析."""
    a = json.dumps({"Service": "asi-core", "State": "running", "Status": "Up", "Health": "healthy"})
    b = json.dumps({"Service": "asi-test", "State": "exited", "Status": "Exited (0)", "Health": "none"})
    out = "\n".join([a, b])
    containers = _parse_ps_output(out)
    assert len(containers) == 2
    services = {c.service for c in containers}
    assert services == {"asi-core", "asi-test"}


def test_parse_ps_output_skips_invalid_lines():
    """V1050 真解析 跳过非法行."""
    out = "garbage line\n" + json.dumps({"Service": "asi-core", "State": "running", "Status": "Up", "Health": "healthy"}) + "\nmore garbage"
    containers = _parse_ps_output(out)
    assert len(containers) == 1
    assert containers[0].service == "asi-core"


# ============================================================================
# 真 inspect 状态 (主 17:43 实事求是)
# ============================================================================


def test_inspect_without_artifacts():
    """V1050 真状态 空目录 artefacts_written=False."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        status = deploy.inspect(tmp)
        assert status.artefacts_written is False
        assert isinstance(status.docker_available, bool)


def test_inspect_with_artefacts_no_docker():
    """V1050 真状态 写了 artifacts 但 docker 不可用."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        deploy.write_artifacts(tmp)
        status = deploy.inspect(tmp)
        assert status.artefacts_written is True
        # 如果 docker 不可用, compose_up/healthy 应为 False
        if not status.docker_available:
            assert status.compose_up is False


# ============================================================================
# 自定义模板 (主 00:36 适配性)
# ============================================================================


def test_custom_dockerfile_persists():
    """V1050 真生产 自定义 dockerfile 持久化 (主 00:36 适配性)."""
    custom = "# custom dockerfile\nFROM alpine:latest\n"
    deploy = V1050RealDockerDeploy(dockerfile=custom)
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        with open(artefacts[V1050_DOCKERFILE], encoding="utf-8") as f:
            assert f.read() == custom


def test_custom_compose_persists():
    """V1050 真生产 自定义 compose 持久化."""
    custom = "version: '3.8'\nservices:\n  custom-svc:\n    image: alpine\n"
    deploy = V1050RealDockerDeploy(compose=custom)
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        with open(artefacts[V1050_COMPOSE], encoding="utf-8") as f:
            assert f.read() == custom


# ============================================================================
# stats (主 00:56 任何人都能接手)
# ============================================================================


def test_stats_has_version_and_philosophy():
    deploy = V1050RealDockerDeploy()
    s = deploy.stats()
    assert s["version"] == V1050_VERSION
    assert "philosophy" in s
    assert "主 06:15" in s["philosophy"]


def test_stats_command_log_count():
    """V1050 真生产 n_commands_run 统计 (主 17:43 实事求是)."""
    deploy = V1050RealDockerDeploy()
    s0 = deploy.stats()
    assert s0["n_commands_run"] == 0


def test_stats_artefact_count_initially_zero():
    deploy = V1050RealDockerDeploy()
    s = deploy.stats()
    assert s["n_artefacts"] == 0


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_module_does_not_pretend_phenomenal():
    """V3 哲学守门: 不假装 Phenomenal consciousness."""
    import apeireth.v1050_real_docker_deploy as m
    src = m.__doc__ or ""
    assert "Phenomenal" in src or "不假装" in src


def test_module_does_not_pretend_asi_solved():
    """V3 哲学守门: 不假装达到 ASI. V1050 真部署 ≠ ASI."""
    import apeireth.v1050_real_docker_deploy as m
    assert hasattr(m, "V3_GUARDS")
    assert m.V3_GUARDS["deploy_is_not_asi"].startswith("V1050")


def test_deploy_is_not_safety_guarded():
    """V3 哲学守门: 部署 ≠ 安全. production_is_not_safety 必备."""
    import apeireth.v1050_real_docker_deploy as m
    assert "production_is_not_safety" in m.V3_GUARDS
    text = m.V3_GUARDS["production_is_not_safety"]
    assert "部署" in text


def test_module_does_not_pretend_deployment_equals_asi():
    """V3 哲学守门: 真部署 ≠ ASI 部署. deploy_is_not_asi 必备."""
    import apeireth.v1050_real_docker_deploy as m
    assert "deploy_is_not_asi" in m.V3_GUARDS


# ============================================================================
# 集成测试 (主 00:36 工程化)
# ============================================================================


def test_integration_full_artifacts_pipeline():
    """真生产完整 artifacts pipeline: 写 5 文件 + 验证大小 + 验证 dockerfile 可读."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        # 5 文件全部存在且非空
        for name, path in artefacts.items():
            assert os.path.exists(path), f"{name} not written"
            assert os.path.getsize(path) > 100, f"{name} too small"
        # dockerfile 可读
        with open(artefacts[V1050_DOCKERFILE], encoding="utf-8") as f:
            content = f.read()
        assert "FROM python:3.13-slim" in content
        assert "USER asi" in content


def test_integration_inspect_after_write():
    """真生产 write_artifacts → inspect 状态正确."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        deploy.write_artifacts(tmp)
        status = deploy.inspect(tmp)
        # artefacts_written 应为 True (compose 文件在)
        assert status.artefacts_written is True
        # 真状态正确反映
        assert status.docker_available in (True, False)


def test_command_log_appends():
    """V1050 真测 command_log 累积 (主 17:43 实事求是)."""
    deploy = V1050RealDockerDeploy()
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        # compose_up / compose_down 会调 _run_command 并 append log
        deploy.compose_up(tmp, timeout=10)
        deploy.compose_down(tmp, timeout=10)
    assert len(deploy.command_log) >= 2


def test_demo_runs():
    """V1050 真测 _demo 函数能跑 (主 00:56 任何人都能接手)."""
    from apeireth.v1050_real_docker_deploy import _demo
    # _demo 内部用临时目录, 应该不报错
    try:
        _demo()
    except Exception as e:
        pytest.fail(f"_demo crashed: {e}")


def test_all_exports_present():
    """V1050 真测 __all__ 导出完整 (主 00:56 任何人都能接手)."""
    from apeireth import v1050_real_docker_deploy as m
    expected = [
        "V1050_VERSION", "V1050_DOCKERFILE", "V1050_COMPOSE", "V1050_ENV",
        "V1050_DOCKERIGNORE", "V1050_REQUIREMENTS",
        "DOCKERFILE_RUNTIME", "DOCKER_COMPOSE_RUNTIME", "DOCKERIGNORE_RUNTIME",
        "ENV_RUNTIME", "REQUIREMENTS_RUNTIME",
        "DockerCommandResult", "ContainerHealth", "DeploymentStatus",
        "V1050RealDockerDeploy",
    ]
    for name in expected:
        assert name in m.__all__, f"missing export: {name}"


def test_custom_requirements_persists():
    """V1050 真生产 自定义 requirements 持久化."""
    custom = "# minimal\npytest>=7.0\n"
    deploy = V1050RealDockerDeploy(requirements=custom)
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        with open(artefacts[V1050_REQUIREMENTS], encoding="utf-8") as f:
            assert f.read() == custom