"""V1040 真生产 tests (主 00:56 任何人都能接手)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import os
import tempfile
import pytest
from apeireth.v1040_cicd import V1040_VERSION, V1040CICD


class TestV1040:
    def test_init(self):
        cicd = V1040CICD()
        assert cicd.n_artefacts() == 0

    def test_render_github_actions(self):
        """V1040 真测 GitHub Actions YAML 真借鉴 (主 19:33)."""
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "name: ASI CI" in yml
        assert "on:" in yml
        assert "jobs:" in yml

    def test_github_actions_jobs(self):
        """V1040 真测 7 真 jobs (主 00:56 任何人都能接手)."""
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        jobs = ["lint", "unit-tests", "integration-tests", "benchmark",
                "health-check", "docker-build", "build-status"]
        for job in jobs:
            assert job in yml, f"missing job: {job}"

    def test_github_actions_python_version(self):
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "PYTHON_VERSION" in yml
        assert "3.13" in yml

    def test_github_actions_asi_north_star(self):
        """V1040 真测 ASI 北极星 (主 22:33)."""
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "ASI_NORTH_STAR" in yml
        assert "0.7905" in yml

    def test_github_actions_runs_integration(self):
        """V1040 真测 runs V1031 integration (主 19:33 + 主 17:43 实事求是)."""
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "v1031_integration" in yml
        assert "pass_rate" in yml

    def test_github_actions_runs_health_check(self):
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "v1036_health_check" in yml

    def test_github_actions_runs_benchmark(self):
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "v1034_real_benchmark" in yml

    def test_github_actions_docker(self):
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "docker" in yml.lower()
        assert "Dockerfile" in yml

    def test_github_actions_test_excludes(self):
        """V1040 真测 excludes 真空壳 (主 23:42 真反思)."""
        cicd = V1040CICD()
        yml = cicd.render_github_actions()
        assert "test_v121_v150" in yml
        assert "test_v251_v500" in yml
        assert "test_v501_v1000" in yml

    def test_render_gitlab_ci(self):
        """V1040 真测 GitLab CI (主 19:33 真借鉴)."""
        cicd = V1040CICD()
        yml = cicd.render_gitlab_ci()
        assert "stages:" in yml
        assert "lint" in yml
        assert "test" in yml
        assert "deploy" in yml

    def test_gitlab_ci_asi_north_star(self):
        cicd = V1040CICD()
        yml = cicd.render_gitlab_ci()
        assert "ASI_NORTH_STAR" in yml

    def test_render_all(self):
        cicd = V1040CICD()
        files = cicd.render_all()
        assert ".github/workflows/asi-ci.yml" in files
        assert ".gitlab-ci.yml" in files

    def test_write_all(self):
        """V1040 真测 write files (主 17:43 实事求是)."""
        cicd = V1040CICD()
        with tempfile.TemporaryDirectory() as tmp:
            written = cicd.write_all(tmp)
            assert len(written) == 2
            assert os.path.exists(os.path.join(tmp, ".github", "workflows", "asi-ci.yml"))
            assert os.path.exists(os.path.join(tmp, ".gitlab-ci.yml"))
            # 真内容
            with open(written[".github/workflows/asi-ci.yml"], encoding="utf-8") as f:
                content = f.read()
            assert "name: ASI CI" in content
        assert cicd.n_artefacts() == 2

    def test_stats(self):
        cicd = V1040CICD()
        s = cicd.stats()
        assert s["n_artefacts"] == 0
        assert s["version"] == V1040_VERSION

    def test_v22_33_asi_integration(self):
        """V1040 真测主 22:33 ASI 北极星."""
        cicd = V1040CICD()
        s = cicd.stats()
        assert "ASI" in s["philosophy"]

    def test_v00_56_handoff(self):
        """V1040 真测主 00:56 任何人都能接手."""
        cicd = V1040CICD()
        files = cicd.render_all()
        # 真 CI pipeline 7 jobs
        yml = files[".github/workflows/asi-ci.yml"]
        assert yml.count("runs-on: ubuntu-latest") == 7

    def test_v19_33_github_actions(self):
        """V1040 真测主 19:33 GitHub Actions + GitLab CI 真借鉴."""
        cicd = V1040CICD()
        yml_gh = cicd.render_github_actions()
        yml_gl = cicd.render_gitlab_ci()
        assert "actions/checkout" in yml_gh
        assert "actions/setup-python" in yml_gh
        assert "stages:" in yml_gl

    def test_v17_43_truth(self):
        """V1040 真测主 17:43 实事求是 — 真文件真写."""
        cicd = V1040CICD()
        with tempfile.TemporaryDirectory() as tmp:
            written = cicd.write_all(tmp)
            for path in written.values():
                assert os.path.getsize(path) > 0

    def test_complete_integration(self):
        """V1040 真测完整 CI/CD (主 00:56 + 主 22:33 + 主 19:33 + 主 17:43)."""
        cicd = V1040CICD()
        files = cicd.render_all()
        # 2 真 CI 文件
        assert len(files) == 2
        # GitHub Actions 7 真 jobs
        yml = files[".github/workflows/asi-ci.yml"]
        for job in ["lint", "unit-tests", "integration-tests", "benchmark",
                    "health-check", "docker-build", "build-status"]:
            assert job in yml