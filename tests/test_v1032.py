"""V1032 真生产 tests (主 00:36 效果 + 工程化)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import os
import tempfile
import pytest
from apeireth.v1032_docker import V1032_VERSION, V1032Docker


class TestV1032:
    def test_init(self):
        docker = V1032Docker()
        assert docker.n_artefacts() == 0

    def test_render_dockerfile(self):
        """V1032 真测 Dockerfile 多阶段构建 真借鉴 (主 19:33 Docker 官方)."""
        docker = V1032Docker()
        df = docker.render_dockerfile()
        assert "FROM python:3.13-slim" in df
        assert "AS builder" in df
        assert "AS runtime" in df  # 多阶段
        assert "COPY --from=builder" in df

    def test_dockerfile_user(self):
        """V1032 真测 non-root user (主 00:36 安全工程化)."""
        docker = V1032Docker()
        df = docker.render_dockerfile()
        assert "USER asi" in df
        assert "useradd" in df

    def test_dockerfile_healthcheck(self):
        """V1032 真测 healthcheck (主 19:33 Docker 最佳实践真借鉴)."""
        docker = V1032Docker()
        df = docker.render_dockerfile()
        assert "HEALTHCHECK" in df
        assert "V1031Integration" in df  # 整合 V1031 真测试

    def test_dockerfile_env(self):
        docker = V1032Docker()
        df = docker.render_dockerfile()
        assert "ASI_NORTH_STAR=0.7905" in df
        assert "ASI_LEVEL=production" in df

    def test_render_docker_compose(self):
        """V1032 真测 docker-compose (主 19:33 真借鉴)."""
        docker = V1032Docker()
        dc = docker.render_docker_compose()
        assert "version: '3.9'" in dc
        assert "asi-core" in dc
        assert "asi-test" in dc
        assert "networks:" in dc

    def test_docker_compose_healthcheck(self):
        docker = V1032Docker()
        dc = docker.render_docker_compose()
        assert "healthcheck:" in dc
        assert "condition: service_healthy" in dc

    def test_render_k8s_manifest(self):
        """V1032 真测 K8s deployment (主 19:33 Kubernetes 真借鉴)."""
        docker = V1032Docker()
        k8s = docker.render_k8s_manifest()
        assert "apiVersion: apps/v1" in k8s
        assert "kind: Deployment" in k8s
        assert "kind: Service" in k8s
        assert "kind: HorizontalPodAutoscaler" in k8s

    def test_k8s_replicas(self):
        """V1032 真测 K8s replicas 真借鉴 (主 00:36 工程化)."""
        docker = V1032Docker()
        k8s = docker.render_k8s_manifest()
        assert "replicas: 3" in k8s
        assert "minReplicas: 3" in k8s
        assert "maxReplicas: 10" in k8s

    def test_k8s_resources(self):
        docker = V1032Docker()
        k8s = docker.render_k8s_manifest()
        assert "memory:" in k8s
        assert "cpu:" in k8s

    def test_k8s_liveness_probe(self):
        """V1032 真测 liveness probe (主 19:33 K8s 最佳实践真借鉴)."""
        docker = V1032Docker()
        k8s = docker.render_k8s_manifest()
        assert "livenessProbe:" in k8s
        assert "readinessProbe:" in k8s
        assert "V1031Integration" in k8s

    def test_render_requirements(self):
        """V1032 真测 requirements.txt (主 00:36 工程化)."""
        docker = V1032Docker()
        req = docker.render_requirements()
        assert "openai" in req
        assert "fastapi" in req
        assert "pyjwt" in req
        assert "pydantic" in req
        assert "pytest" in req

    def test_render_all(self):
        docker = V1032Docker()
        files = docker.render_all()
        assert "Dockerfile" in files
        assert "docker-compose.yml" in files
        assert "k8s-deployment.yaml" in files
        assert "requirements.txt" in files

    def test_write_all(self):
        """V1032 真测 write files 真借鉴 (主 17:43 实事求是)."""
        docker = V1032Docker()
        with tempfile.TemporaryDirectory() as tmp:
            written = docker.write_all(tmp)
            assert len(written) == 4
            # 真文件存在
            assert os.path.exists(os.path.join(tmp, "Dockerfile"))
            assert os.path.exists(os.path.join(tmp, "docker-compose.yml"))
            assert os.path.exists(os.path.join(tmp, "k8s-deployment.yaml"))
            assert os.path.exists(os.path.join(tmp, "requirements.txt"))
            # 真内容
            with open(os.path.join(tmp, "Dockerfile"), encoding="utf-8") as f:
                content = f.read()
            assert "FROM python:3.13-slim" in content
        assert docker.n_artefacts() == 4

    def test_stats(self):
        docker = V1032Docker()
        s = docker.stats()
        assert s["n_artefacts"] == 0
        assert s["version"] == V1032_VERSION

    def test_v22_33_asi_integration(self):
        """V1032 真测主 22:33 ASI 北极星."""
        docker = V1032Docker()
        s = docker.stats()
        assert "ASI" in s["philosophy"]

    def test_v00_36_effect_engineering(self):
        """V1032 真测主 00:36 效果 + 工程化 — 真能跑."""
        docker = V1032Docker()
        df = docker.render_dockerfile()
        # 效果: 有 HEALTHCHECK (真检测)
        assert "HEALTHCHECK" in df
        # 工程化: 多阶段构建 (生产效率)
        assert df.count("FROM ") == 2

    def test_v19_33_docker_k8s(self):
        """V1032 真测主 19:33 Docker + Kubernetes 真借鉴."""
        docker = V1032Docker()
        df = docker.render_dockerfile()
        k8s = docker.render_k8s_manifest()
        # Docker 多阶段
        assert "AS builder" in df and "AS runtime" in df
        # K8s Deployment + Service + HPA
        assert "kind: Deployment" in k8s
        assert "kind: Service" in k8s
        assert "kind: HorizontalPodAutoscaler" in k8s

    def test_v17_43_truth(self):
        """V1032 真测主 17:43 实事求是 — 真文件真写."""
        docker = V1032Docker()
        with tempfile.TemporaryDirectory() as tmp:
            written = docker.write_all(tmp)
            # 真文件大小 > 0
            for path in written.values():
                assert os.path.getsize(path) > 0
            # 真读 Dockerfile 内容
            with open(written["Dockerfile"], encoding="utf-8") as f:
                content = f.read()
            # 真的 FROM 指令
            assert content.startswith("# ASI")

    def test_complete_integration(self):
        """V1032 真测完整 Docker (主 00:36 + 主 22:33 + 主 19:33 + 主 17:43)."""
        docker = V1032Docker()
        files = docker.render_all()
        # 质量: 4 真文件
        assert len(files) == 4
        # 工程化: 包含 Dockerfile + compose + k8s + deps
        assert "Dockerfile" in files
        assert "docker-compose.yml" in files
        assert "k8s-deployment.yaml" in files
        assert "requirements.txt" in files
        # 效果: HEALTHCHECK 真检测 V1031 integration
        assert "V1031Integration" in files["Dockerfile"]
        assert "V1031Integration" in files["docker-compose.yml"]