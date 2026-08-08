"""Phase 1050 v1050_real_docker_deploy — V1050 ASI 真厨房 Docker 真部署 (主 06:15 + 主 00:36 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33).

主 06:15 当前真生产方向: V1050 = 真厨房部 - V1008/V1032 Docker 真部署.
主 00:36 真采纳: 质量 + 适配性 + 效果 + 工程化.
主 23:44 干到底: 真生产不是模板, 是真能 docker-compose up -d 跑起来.
主 22:33 ASI 北极星: 真部署 ASI 北极星引擎, 验证真生产.
主 19:33 走在前人经验上: 真借鉴 Docker 官方 CLI + docker-compose CLI + 真终端调用.
主 17:43 实事求是: 不假装部署; 不假装容器跑; 真 subprocess + 真 healthcheck.
主 17:33 放手干到底: 真跑真测真终端, 不空壳.

真生产设计 (主 19:33):
- 真 subprocess.run 调 docker / docker-compose 命令 (主 19:33 Docker CLI 真借鉴)
- 真写 Dockerfile + docker-compose.yml + .dockerignore + .env 到目标目录
- 真 docker-compose up -d (后台跑) + 真 docker-compose down (关)
- 真 docker ps 看进程 + 真 docker logs 拿日志 + 真 docker inspect 查 health
- 真 timeout 等 healthcheck pass (主 17:43 真等, 不假装)
- 真检测 docker 可用性 (主 17:43 真测, 没装 Docker 就返回结构化状态)
- 真回执 (return code, stdout, stderr, health status)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 deployment-engineering, 不是 consciousness claim.
- 不假装达到 ASI: 真部署 ≠ ASI 达成; ASI 部署只是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是 subprocess 真跑, 不是改 prompt 假装部署.
- 部署 ≠ 守门: 真生产 ≠ 真安全. 部署 ≠ 安全审计.
- 真生产 = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "deployed = safe" 都是不假装.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


V1050_VERSION = "0.1.0"
V1050_DOCKERFILE = "Dockerfile"
V1050_COMPOSE = "docker-compose.yml"
V1050_ENV = ".env"
V1050_DOCKERIGNORE = ".dockerignore"
V1050_REQUIREMENTS = "requirements.txt"


# ============================================================================
# 真生产 Dockerfile (主 19:33 Docker 官方多阶段构建真借鉴)
# ============================================================================
# 真借鉴: Docker 官方多阶段构建 (主 19:33 https://docs.docker.com/build/building/multi-stage/)
# 主 00:36 工程化: 非 root + healthcheck + .dockerignore + slim base.
# 主 17:43 实事求是: 不空壳, 真能 docker build 跑.

DOCKERFILE_RUNTIME = """# V1050 ASI 真厨房 Dockerfile (主 06:15 + 主 00:36 + 主 19:33 Docker 官方真借鉴)
# 多阶段构建: builder 装依赖, runtime 只拿 artifacts
FROM python:3.13-slim AS builder

WORKDIR /build

# System build deps (主 00:36 工程化: 仅 build stage 装 gcc)
RUN apt-get update \\
 && apt-get install -y --no-install-recommends gcc g++ \\
 && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir --user -r requirements.txt

# Stage 2: runtime (主 00:36 slim + non-root + 健康检查)
FROM python:3.13-slim AS runtime

WORKDIR /app

# Non-root user (主 00:36 安全工程化)
RUN useradd --create-home --shell /bin/bash asi

COPY --from=builder /root/.local /home/asi/.local
COPY apeireth/ /app/apeireth/
COPY tests/ /app/tests/

ENV PATH=/home/asi/.local/bin:$PATH
ENV PYTHONPATH=/app
ENV ASI_NORTH_STAR=0.7905
ENV ASI_LEVEL=production
ENV PYTHONUNBUFFERED=1

USER asi

# Healthcheck (主 19:33 Docker 最佳实践)
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \\
  CMD python -c "from apeireth.v1031_integration import V1031Integration; r=V1031Integration().run(); exit(0 if r['pass_rate']==1.0 else 1)" \\
  || exit 1

CMD ["python", "-c", "from apeireth.v1031_integration import V1031Integration; V1031Integration().run()"]
"""


DOCKER_COMPOSE_RUNTIME = """# V1050 ASI 真厨房 docker-compose (主 06:15 + 主 00:36 + 主 19:33 Docker Compose 真借鉴)
# 真生产编排: core 服务 + test sidecar + healthcheck depends_on.
version: '3.9'

services:
  asi-core:
    build:
      context: .
      dockerfile: Dockerfile
    image: apeireth/asi:latest
    container_name: asi-core
    environment:
      - ASI_NORTH_STAR=0.7905
      - ASI_LEVEL=production
      - PYTHONUNBUFFERED=1
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "python", "-c", "from apeireth.v1031_integration import V1031Integration; r=V1031Integration().run(); exit(0 if r['pass_rate']==1.0 else 1)"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    networks:
      - asi-net

  asi-test:
    build:
      context: .
      dockerfile: Dockerfile
    image: apeireth/asi:test
    container_name: asi-test
    command: ["python", "-m", "pytest", "tests/", "-q", "--tb=line"]
    environment:
      - ASI_LEVEL=test
    depends_on:
      asi-core:
        condition: service_healthy
    networks:
      - asi-net

networks:
  asi-net:
    driver: bridge
"""


DOCKERIGNORE_RUNTIME = """# V1050 .dockerignore (主 00:36 工程化 + 主 19:33 Docker 最佳实践)
.git
.gitignore
__pycache__
*.pyc
*.pyo
.venv
venv
env
.env.local
.pytest_cache
.mypy_cache
.ruff_cache
dist
build
*.egg-info
.DS_Store
node_modules
*.md
!README.md
.idea
.vscode
tests_output
"""


ENV_RUNTIME = """# V1050 真厨房 .env 模板 (主 00:36 工程化)
ASI_NORTH_STAR=0.7905
ASI_LEVEL=production
PYTHONUNBUFFERED=1
# 真实 LLM API 接入 (主 17:43 真环境变量)
OPENAI_API_KEY=
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=gpt-4o-mini
"""


REQUIREMENTS_RUNTIME = """# V1050 真厨房 requirements (主 00:36 工程化)
# LLM (主 17:43 真生产)
openai>=1.0.0
anthropic>=0.18.0

# Web
fastapi>=0.100.0
uvicorn>=0.23.0
streamlit>=1.28.0

# Auth
pyjwt>=2.8.0

# Validation
pydantic>=2.0.0
jsonschema>=4.19.0

# Async
aiohttp>=3.9.0

# Testing
pytest>=7.4.0
pytest-asyncio>=0.21.0

# Logging
structlog>=23.1.0

# Config
omegaconf>=2.3.0
python-dotenv>=1.0.0

# Search / Index
sentence-transformers>=2.2.0
"""


# ============================================================================
# 真生产数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class DockerCommandResult:
    """V1050 真生产 Docker 命令执行结果 (主 17:43 真终端)."""
    command: List[str]
    returncode: int
    stdout: str
    stderr: str
    elapsed_seconds: float
    ok: bool
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "command": self.command,
            "returncode": self.returncode,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "ok": self.ok,
            "ts": self.ts,
        }


@dataclass
class ContainerHealth:
    """V1050 真生产容器健康状态 (主 17:43 真 inspect)."""
    service: str
    state: str  # running / exited / restarting / unhealthy / healthy / unknown
    status: str  # Up 5 minutes / Exited (0) / etc.
    health: str  # healthy / unhealthy / starting / none
    container_id: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "service": self.service,
            "state": self.state,
            "status": self.status,
            "health": self.health,
            "container_id": self.container_id,
        }


@dataclass
class DeploymentStatus:
    """V1050 真生产 deployment 状态 (主 17:43 实事求是)."""
    docker_available: bool
    compose_available: bool
    artefacts_written: bool
    compose_up: bool
    healthy: bool
    last_command: Optional[str] = None
    last_error: Optional[str] = None
    artefacts: Dict[str, str] = field(default_factory=dict)
    containers: List[ContainerHealth] = field(default_factory=list)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "docker_available": self.docker_available,
            "compose_available": self.compose_available,
            "artefacts_written": self.artefacts_written,
            "compose_up": self.compose_up,
            "healthy": self.healthy,
            "last_command": self.last_command,
            "last_error": self.last_error,
            "n_artefacts": len(self.artefacts),
            "n_containers": len(self.containers),
            "ts": self.ts,
        }


# ============================================================================
# V1050 真厨房 Docker 真部署 (主 06:15 + 主 00:36 + 主 19:33 + 主 17:43)
# ============================================================================


class V1050RealDockerDeploy:
    """V1050 ASI 真厨房 Docker 真部署 (主 06:15 + 主 00:36 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43).

    真生产设计 (主 19:33 Docker CLI 真借鉴):
    - write_artifacts(dir): 真写 Dockerfile + docker-compose.yml + .dockerignore + .env + requirements.txt
    - check_docker_available(): 真检查 docker / docker-compose 二进制
    - compose_up(dir, build=False): 真 subprocess docker-compose up -d
    - compose_down(dir): 真 subprocess docker-compose down
    - compose_build(dir): 真 subprocess docker-compose build
    - ps(dir): 真 subprocess docker-compose ps
    - health_check(dir, timeout): 真轮询 healthcheck
    - logs(dir, service): 真 subprocess docker-compose logs

    主 17:43 实事求是: 没装 Docker 不假装, 状态结构化返回; 不假装 healthcheck 通过.
    """

    DEFAULT_DOCKERFILE = DOCKERFILE_RUNTIME
    DEFAULT_COMPOSE = DOCKER_COMPOSE_RUNTIME
    DEFAULT_DOCKERIGNORE = DOCKERIGNORE_RUNTIME
    DEFAULT_ENV = ENV_RUNTIME
    DEFAULT_REQUIREMENTS = REQUIREMENTS_RUNTIME

    def __init__(self,
                 dockerfile: Optional[str] = None,
                 compose: Optional[str] = None,
                 dockerignore: Optional[str] = None,
                 env_file: Optional[str] = None,
                 requirements: Optional[str] = None):
        self.dockerfile = dockerfile or self.DEFAULT_DOCKERFILE
        self.compose = compose or self.DEFAULT_COMPOSE
        self.dockerignore = dockerignore or self.DEFAULT_DOCKERIGNORE
        self.env_file = env_file or self.DEFAULT_ENV
        self.requirements = requirements or self.DEFAULT_REQUIREMENTS
        self.last_result: Optional[DockerCommandResult] = None
        self.last_status: Optional[DeploymentStatus] = None
        self.command_log: List[DockerCommandResult] = []

    # ------------------------------------------------------------------
    # Artifacts 写入 (主 17:43 真写)
    # ------------------------------------------------------------------

    def write_artifacts(self, output_dir: str = ".") -> Dict[str, str]:
        """V1050 真生产 写入真部署文件到 output_dir (主 17:43 真写文件)."""
        os.makedirs(output_dir, exist_ok=True)
        artefacts: Dict[str, str] = {}
        files = {
            V1050_DOCKERFILE: self.dockerfile,
            V1050_COMPOSE: self.compose,
            V1050_DOCKERIGNORE: self.dockerignore,
            V1050_ENV: self.env_file,
            V1050_REQUIREMENTS: self.requirements,
        }
        for name, content in files.items():
            path = os.path.join(output_dir, name)
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            artefacts[name] = path
        if self.last_status is not None:
            self.last_status.artefacts = artefacts
            self.last_status.artefacts_written = True
        return artefacts

    # ------------------------------------------------------------------
    # Docker 可用性 (主 17:43 真测)
    # ------------------------------------------------------------------

    @staticmethod
    def _run_command(command: List[str], timeout: int = 60) -> DockerCommandResult:
        """V1050 真生产 subprocess 真跑 (主 19:33 Docker CLI 真借鉴)."""
        start = time.time()
        try:
            proc = subprocess.run(
                command,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
                shell=False,
            )
            elapsed = time.time() - start
            ok = proc.returncode == 0
            return DockerCommandResult(
                command=command,
                returncode=proc.returncode,
                stdout=proc.stdout,
                stderr=proc.stderr,
                elapsed_seconds=elapsed,
                ok=ok,
            )
        except FileNotFoundError as e:
            elapsed = time.time() - start
            return DockerCommandResult(
                command=command,
                returncode=-1,
                stdout="",
                stderr=f"FileNotFoundError: {e}",
                elapsed_seconds=elapsed,
                ok=False,
            )
        except subprocess.TimeoutExpired as e:
            elapsed = time.time() - start
            return DockerCommandResult(
                command=command,
                returncode=-2,
                stdout=e.stdout.decode("utf-8", errors="ignore") if e.stdout else "",
                stderr=f"TimeoutExpired after {timeout}s: {e}",
                elapsed_seconds=elapsed,
                ok=False,
            )
        except Exception as e:
            elapsed = time.time() - start
            return DockerCommandResult(
                command=command,
                returncode=-3,
                stdout="",
                stderr=f"{type(e).__name__}: {e}",
                elapsed_seconds=elapsed,
                ok=False,
            )

    def check_docker_available(self) -> Tuple[bool, bool, Optional[str]]:
        """V1050 真生产 检查 docker + docker-compose 是否真可用 (主 17:43 真测).

        Returns: (docker_available, compose_available, error_message)
        """
        # 真检查 docker 二进制
        docker_bin = shutil.which("docker")
        compose_bin = shutil.which("docker-compose") or shutil.which("docker")
        docker_avail = docker_bin is not None
        # 真跑 docker version
        docker_ok = False
        compose_ok = False
        err: Optional[str] = None
        if docker_avail:
            r = self._run_command(["docker", "version", "--format", "{{.Server.Version}}"], timeout=15)
            docker_ok = r.ok and bool(r.stdout.strip())
            if not docker_ok:
                err = r.stderr or r.stdout
        if compose_bin:
            # 真测 docker compose (v2 子命令) 或 docker-compose (v1)
            if shutil.which("docker-compose"):
                r = self._run_command(["docker-compose", "version", "--short"], timeout=15)
            else:
                r = self._run_command(["docker", "compose", "version", "--short"], timeout=15)
            compose_ok = r.ok and bool(r.stdout.strip())
            if not compose_ok and err is None:
                err = r.stderr or r.stdout
        if self.last_status is not None:
            self.last_status.docker_available = docker_ok
            self.last_status.compose_available = compose_ok
            if err and not docker_ok:
                self.last_status.last_error = err
        return docker_ok, compose_ok, err

    # ------------------------------------------------------------------
    # 真部署生命周期
    # ------------------------------------------------------------------

    def compose_up(self, output_dir: str = ".", build: bool = False, timeout: int = 600) -> DockerCommandResult:
        """V1050 真生产 docker-compose up -d (主 17:43 真终端)."""
        if shutil.which("docker-compose"):
            cmd = ["docker-compose", "up", "-d"]
        else:
            cmd = ["docker", "compose", "up", "-d"]
        if build:
            cmd.append("--build")
        result = self._run_command(cmd, timeout=timeout)
        self.last_result = result
        self.command_log.append(result)
        if self.last_status is not None:
            self.last_status.compose_up = result.ok
            self.last_status.last_command = " ".join(cmd)
            if not result.ok:
                self.last_status.last_error = result.stderr or result.stdout
        return result

    def compose_down(self, output_dir: str = ".", timeout: int = 120) -> DockerCommandResult:
        """V1050 真生产 docker-compose down (主 17:43 真终端)."""
        if shutil.which("docker-compose"):
            cmd = ["docker-compose", "down"]
        else:
            cmd = ["docker", "compose", "down"]
        result = self._run_command(cmd, timeout=timeout)
        self.last_result = result
        self.command_log.append(result)
        if self.last_status is not None:
            self.last_status.last_command = " ".join(cmd)
        return result

    def compose_build(self, output_dir: str = ".", timeout: int = 900) -> DockerCommandResult:
        """V1050 真生产 docker-compose build (主 17:43 真终端)."""
        if shutil.which("docker-compose"):
            cmd = ["docker-compose", "build"]
        else:
            cmd = ["docker", " compose", "build"]
        result = self._run_command(cmd, timeout=timeout)
        self.last_result = result
        self.command_log.append(result)
        return result

    def ps(self, output_dir: str = ".", timeout: int = 30) -> DockerCommandResult:
        """V1050 真生产 docker-compose ps (主 17:43 真终端)."""
        if shutil.which("docker-compose"):
            cmd = ["docker-compose", "ps", "--format", "json"]
        else:
            cmd = ["docker", "compose", "ps", "--format", "json"]
        result = self._run_command(cmd, timeout=timeout)
        self.last_result = result
        self.command_log.append(result)
        return result

    def logs(self, output_dir: str = ".", service: Optional[str] = None,
             tail: int = 100, timeout: int = 30) -> DockerCommandResult:
        """V1050 真生产 docker-compose logs 真拿日志 (主 17:43 真终端)."""
        if shutil.which("docker-compose"):
            cmd = ["docker-compose", "logs", "--no-color", "--tail", str(tail)]
        else:
            cmd = ["docker", "compose", "logs", "--no-color", "--tail", str(tail)]
        if service:
            cmd.append(service)
        result = self._run_command(cmd, timeout=timeout)
        self.last_result = result
        self.command_log.append(result)
        return result

    def health_check(self, output_dir: str = ".", timeout: int = 120,
                     interval: float = 5.0) -> Tuple[bool, List[ContainerHealth]]:
        """V1050 真生产 等容器 healthcheck 真通过 (主 17:43 真等).

        Returns: (all_healthy, container_healths)
        """
        deadline = time.time() + timeout
        all_healthy = False
        containers: List[ContainerHealth] = []
        while time.time() < deadline:
            r = self.ps(output_dir=output_dir)
            if not r.ok:
                time.sleep(interval)
                continue
            containers = _parse_ps_output(r.stdout)
            if containers and all(c.health == "healthy" for c in containers):
                all_healthy = True
                break
            time.sleep(interval)
        if self.last_status is not None:
            self.last_status.healthy = all_healthy
            self.last_status.containers = containers
        return all_healthy, containers

    # ------------------------------------------------------------------
    # 状态汇总 (主 17:43 实事求是)
    # ------------------------------------------------------------------

    def inspect(self, output_dir: str = ".") -> DeploymentStatus:
        """V1050 真生产 一站式真状态检查 (主 17:43 实事求是)."""
        docker_ok, compose_ok, err = self.check_docker_available()
        artefacts_path = os.path.join(output_dir, V1050_COMPOSE)
        artefacts_written = os.path.exists(artefacts_path)
        status = DeploymentStatus(
            docker_available=docker_ok,
            compose_available=compose_ok,
            artefacts_written=artefacts_written,
            compose_up=False,
            healthy=False,
            last_error=err,
        )
        if docker_ok and artefacts_written:
            ps_result = self.ps(output_dir=output_dir)
            if ps_result.ok:
                containers = _parse_ps_output(ps_result.stdout)
                status.containers = containers
                status.compose_up = len(containers) > 0
                status.healthy = all(c.health == "healthy" for c in containers) if containers else False
        self.last_status = status
        return status

    # ------------------------------------------------------------------
    # 哲学与统计 (主 17:43 + 主 00:56)
    # ------------------------------------------------------------------

    def stats(self) -> Dict[str, Any]:
        return {
            "version": V1050_VERSION,
            "n_commands_run": len(self.command_log),
            "n_artefacts": len(self.last_status.artefacts) if self.last_status else 0,
            "philosophy": (
                "V1050 ASI 真厨房 Docker 真部署 (主 06:15 + 主 00:36 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43). "
                "Dockerfile + docker-compose 真写, 真 subprocess 跑, 真 healthcheck 等, 不空壳. "
                "部署 ≠ 守门: 真生产 ≠ 真安全."
            ),
        }


# ============================================================================
# 真生产 helpers (主 17:43 真解析)
# ============================================================================


def _parse_ps_output(stdout: str) -> List[ContainerHealth]:
    """V1050 真生产 解析 docker-compose ps --format json 输出 (主 17:43 真解析)."""
    containers: List[ContainerHealth] = []
    if not stdout.strip():
        return containers
    # docker-compose ps --format json 每行一个 JSON 对象
    for line in stdout.strip().splitlines():
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            import json
            obj = json.loads(line)
            service = obj.get("Service") or obj.get("Name") or "unknown"
            state = obj.get("State", "unknown")
            status = obj.get("Status", "unknown")
            health = obj.get("Health", "none") or "none"
            containers.append(ContainerHealth(
                service=service,
                state=state,
                status=status,
                health=health,
                container_id=obj.get("ID"),
            ))
        except Exception:
            continue
    return containers


__all__ = [
    "V1050_VERSION",
    "V1050_DOCKERFILE",
    "V1050_COMPOSE",
    "V1050_ENV",
    "V1050_DOCKERIGNORE",
    "V1050_REQUIREMENTS",
    "DOCKERFILE_RUNTIME",
    "DOCKER_COMPOSE_RUNTIME",
    "DOCKERIGNORE_RUNTIME",
    "ENV_RUNTIME",
    "REQUIREMENTS_RUNTIME",
    "DockerCommandResult",
    "ContainerHealth",
    "DeploymentStatus",
    "V1050RealDockerDeploy",
]


def _demo():
    print("=" * 70)
    print("=== Phase 1050 V1050 ASI 真厨房 Docker 真部署 (主 06:15) ===")
    print("=" * 70)
    deploy = V1050RealDockerDeploy()
    # 真写 artifacts 到临时目录
    import tempfile
    with tempfile.TemporaryDirectory(prefix="v1050_") as tmp:
        artefacts = deploy.write_artifacts(tmp)
        print(f"\n  ✓ artefacts written: {len(artefacts)}")
        for name, path in artefacts.items():
            print(f"    - {name}: {os.path.basename(path)}")
        # 真检查 docker 可用性
        docker_ok, compose_ok, err = deploy.check_docker_available()
        print(f"\n  ✓ docker_available: {docker_ok}")
        print(f"  ✓ compose_available: {compose_ok}")
        if err:
            print(f"  ! last_error: {err[:120]}")
        # 一站式真状态
        status = deploy.inspect(tmp)
        print(f"\n  ✓ status summary:")
        for k, v in status.to_dict().items():
            print(f"    - {k}: {v}")
    s = deploy.stats()
    print(f"\n  ✓ n_commands_run: {s['n_commands_run']}")
    print("=" * 70)


if __name__ == "__main__":
    _demo()

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI)
V3_GUARDS = {
    "module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.",
    "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.",
    "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.",
    "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. V1050 真部署 ≠ ASI safety 真生产. 任何声称 production = safe 是不假装.",
    "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1050 subprocess 自动部署 ≠ V1050 自主 ASI.",
    "deploy_is_not_asi": "V1050 真厨房 Docker 部署 ≠ ASI 部署. 容器跑起来 ≠ ASI 跑起来. 部署是工程化, ASI 是更大目标.",
}