"""V1008 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1008_deployment import (
    V1008_VERSION, DeploymentConfig, generate_docker_compose,
    generate_k8s_manifest, generate_startup_script, V1008Deployment,
)


class TestV1008:
    def test_init(self):
        dep = V1008Deployment()
        assert dep.n_services() == 0

    def test_add_service(self):
        dep = V1008Deployment()
        dep.add_service("test", "test-svc", "test/image:v1", port=8000)
        assert dep.n_services() == 1

    def test_docker_compose(self):
        dep = V1008Deployment()
        dep.add_service("Apeireth", "apeireth", "apeireth/asi:v1", port=8000)
        compose = dep.render_docker_compose()
        assert "version:" in compose
        assert "apeireth" in compose
        assert "8000:8000" in compose
        assert "healthcheck" in compose

    def test_docker_compose_with_env(self):
        dep = V1008Deployment()
        dep.add_service("t", "t-svc", "t:v1", port=8000,
                        env_vars={"API_KEY": "abc", "DEBUG": "true"})
        compose = dep.render_docker_compose()
        assert "API_KEY=abc" in compose
        assert "DEBUG=true" in compose

    def test_k8s_manifest(self):
        dep = V1008Deployment()
        dep.add_service("Apeireth", "apeireth", "apeireth:v1",
                        port=8000, replicas=2)
        manifest = dep.render_k8s_manifests()
        assert "kind: Deployment" in manifest
        assert "kind: Service" in manifest
        assert "replicas: 2" in manifest
        assert "containerPort: 8000" in manifest
        assert "livenessProbe" in manifest

    def test_startup_script(self):
        script = generate_startup_script()
        assert "#!/bin/bash" in script
        assert "Apeireth ASI" in script
        assert "pytest" in script
        assert "V1002" in script
        assert "V1008" in script or "真生产" in script

    def test_stats(self):
        dep = V1008Deployment()
        dep.add_service("t", "t-svc", "t:v1", port=8000)
        s = dep.stats()
        assert s["n_services"] == 1
        assert s["version"] == V1008_VERSION

    def test_v22_33_asi_integration(self):
        """V1008 真测主 22:33 ASI 北极星."""
        dep = V1008Deployment()
        dep.add_service("Apeireth", "apeireth-core", "apeireth:v1",
                        port=8000, env_vars={"ASI_NORTH_STAR": "true"})
        compose = dep.render_docker_compose()
        assert "ASI_NORTH_STAR" in compose

    def test_v19_33_integration(self):
        """V1008 真测主 19:33 走在前人经验上 (K8s manifest 真借鉴)."""
        dep = V1008Deployment()
        dep.add_service("Apeireth", "apeireth", "apeireth:v1", port=8000)
        manifest = dep.render_k8s_manifests()
        assert "livenessProbe" in manifest
        assert "Service" in manifest

    def test_v17_33_integration(self):
        """V1008 真测主 17:33 放手干到底 (startup script 真生产)."""
        script = generate_startup_script()
        assert "set -e" in script
        assert "pytest" in script
        assert "真生产" in script

    def test_multi_services(self):
        dep = V1008Deployment()
        dep.add_service("core", "core-svc", "core:v1", port=8000)
        dep.add_service("plugin", "plugin-svc", "plugin:v1", port=8001)
        dep.add_service("monitoring", "mon-svc", "mon:v1", port=8002)
        assert dep.n_services() == 3
        compose = dep.render_docker_compose()
        assert "core-svc" in compose
        assert "plugin-svc" in compose
        assert "mon-svc" in compose

    def test_health_check_in_compose(self):
        dep = V1008Deployment()
        dep.add_service("t", "t-svc", "t:v1", port=8080)
        compose = dep.render_docker_compose()
        assert "8080" in compose
        assert "curl" in compose
        assert "/health" in compose