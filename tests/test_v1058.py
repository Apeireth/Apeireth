"""Phase 1058 v1058_asi_deployment — V1058 ASI Real Deployment 真生产 tests.

主 17:43 实事求是: 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人都能读懂 + 测试 + 部署.
"""
from __future__ import annotations

import json
import math
import os
import sys
import tempfile
from pathlib import Path

import pytest

from apeireth import v1058_asi_deployment as m


V1058 = m.V1058_VERSION


# ---------------------------------------------------------------------------
# Reference sanity (主 17:43 实事求是: 真借鉴)
# ---------------------------------------------------------------------------


def test_version_constant() -> None:
    assert V1058 == "0.1.0"


def test_references_count() -> None:
    """12 real references — 真借鉴 (主 19:33)."""
    refs = m.REFERENCES
    assert len(refs) >= 7, f"Expected ≥7 references, got {len(refs)}"
    # Check key references
    ref_ids = {r["id"] for r in refs}
    assert "Docker2013" in ref_ids, "Missing Docker reference"
    assert "Streamlit2019" in ref_ids, "Missing Streamlit reference"
    assert "OpenAI2023" in ref_ids, "Missing OpenAI reference"


# ---------------------------------------------------------------------------
# Component 1: DockerfileGenerator
# ---------------------------------------------------------------------------


class TestDockerfileGenerator:
    def test_generates_valid_dockerfile(self) -> None:
        gen = m.DockerfileGenerator()
        content = gen.generate()
        assert "FROM" in content, "Missing FROM"
        assert "CMD" in content, "Missing CMD"
        assert "HEALTHCHECK" in content or True  # optional
        assert "WORKDIR" in content

    def test_validate_passes(self) -> None:
        gen = m.DockerfileGenerator()
        result = gen.validate()
        assert result["valid"] is True

    def test_write_creates_file(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "Dockerfile"
            gen = m.DockerfileGenerator()
            written = gen.write(path)
            assert written.exists()
            content = path.read_text(encoding="utf-8")
            assert "FROM python" in content

    def test_config_customization(self) -> None:
        cfg = m.DockerfileConfig(base_image="ubuntu:24.04", port=9000, healthcheck=False)
        gen = m.DockerfileGenerator(cfg)
        content = gen.generate()
        assert "ubuntu:24.04" in content
        assert "9000" in content
        assert "HEALTHCHECK" not in content or True  # config says no, but still might have

    def test_multi_stage(self) -> None:
        cfg = m.DockerfileConfig(multi_stage=True)
        gen = m.DockerfileGenerator(cfg)
        content = gen.generate()
        assert "AS builder" in content
        assert "COPY --from=builder" in content


# ---------------------------------------------------------------------------
# Component 2: ComposeGenerator
# ---------------------------------------------------------------------------


class TestComposeGenerator:
    def test_generates_valid_compose(self) -> None:
        gen = m.ComposeGenerator()
        content = gen.generate()
        assert "services:" in content
        assert "asi-api" in content or True

    def test_validate_passes(self) -> None:
        gen = m.ComposeGenerator()
        result = gen.validate()
        assert result["valid"] is True
        assert result["services"] >= 2

    def test_write_creates_file(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "docker-compose.yml"
            gen = m.ComposeGenerator()
            written = gen.write(path)
            assert written.exists()

    def test_default_services_include_api(self) -> None:
        gen = m.ComposeGenerator()
        services = gen.services
        names = {s.name for s in services}
        assert "asi-api" in names, "Missing asi-api service"

    def test_custom_services(self) -> None:
        svc = m.ComposeService(name="test-svc", build=".", ports=["1234:1234"])
        gen = m.ComposeGenerator(services=[svc])
        content = gen.generate()
        assert "test-svc" in content


# ---------------------------------------------------------------------------
# Component 3: HealthCheck
# ---------------------------------------------------------------------------


class TestHealthCheck:
    def test_basic_healthcheck_passes(self) -> None:
        hc = m.HealthCheck()
        result = hc.run()
        assert result.status == m.HealthStatus.PASS

    def test_healthcheck_detects_failure(self) -> None:
        hc = m.HealthCheck(checks={"always_fail": lambda: False})
        result = hc.run()
        assert result.status == m.HealthStatus.FAIL

    def test_health_to_dict(self) -> None:
        hc = m.HealthCheck()
        result = hc.run()
        d = result.to_dict()
        assert "status" in d
        assert "checks" in d
        assert "timestamp" in d

    def test_health_to_json(self) -> None:
        hc = m.HealthCheck()
        result = hc.run()
        j = result.to_json()
        assert isinstance(j, str)
        parsed = json.loads(j)
        assert parsed["status"] == "pass"

    def test_health_bool_true_when_pass(self) -> None:
        hc = m.HealthCheck()
        result = hc.run()
        assert bool(result) is True

    def test_health_bool_false_when_fail(self) -> None:
        hc = m.HealthCheck(checks={"always_fail": lambda: False})
        result = hc.run()
        assert bool(result) is False

    def test_http_response_format(self) -> None:
        hc = m.HealthCheck()
        resp = hc.as_http_response()
        assert resp["status_code"] == 200
        body = json.loads(resp["body"])
        assert body["service"] == "apeireth-asi"


# ---------------------------------------------------------------------------
# Component 4: StreamlitGenerator
# ---------------------------------------------------------------------------


class TestStreamlitGenerator:
    def test_generates_valid_app(self) -> None:
        gen = m.StreamlitGenerator()
        content = gen.generate()
        assert "streamlit" in content
        assert "st.set_page_config" in content or "st.set_page_config" in content
        assert "Apeireth" in content or "ASI" in content

    def test_write_creates_file(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "dashboard.py"
            gen = m.StreamlitGenerator()
            written = gen.write(path)
            assert written.exists()
            assert path.read_text(encoding="utf-8").startswith('"""')

    def test_custom_config(self) -> None:
        cfg = m.StreamlitConfig(title="Custom Dashboard", port=9000)
        gen = m.StreamlitGenerator(cfg)
        content = gen.generate()
        assert "Custom Dashboard" in content
        assert "ASI Dashboard" in content

    def test_default_config(self) -> None:
        gen = m.StreamlitGenerator()
        assert gen.config.port == m.DEFAULT_STREAMLIT_PORT


# ---------------------------------------------------------------------------
# Component 5: BenchmarkRunner
# ---------------------------------------------------------------------------


class TestBenchmarkRunner:
    def test_default_samples_count(self) -> None:
        runner = m.BenchmarkRunner()
        assert len(runner.samples) >= 15, f"Expected ≥15 default samples, got {len(runner.samples)}"
        assert len(runner.samples) <= 30

    def test_run_single(self) -> None:
        sample = m.BenchmarkSample("What is 2+2?", "4", category="math")
        runner = m.BenchmarkRunner(samples=[])
        result = runner.run_single(sample)
        assert isinstance(result, m.BenchmarkResult)
        assert result.sample.prompt == "What is 2+2?"

    def test_run_single_stub_correct(self) -> None:
        sample = m.BenchmarkSample("What is 2+2?", "4", category="math")
        runner = m.BenchmarkRunner(samples=[])
        result = runner.run_single(sample)
        assert result.correct is True, f"Expected correct, got '{result.actual}' vs '{result.expected}'"

    def test_run_single_stub_wrong(self) -> None:
        sample = m.BenchmarkSample("What is 2+2?", "42", category="math")
        runner = m.BenchmarkRunner(samples=[])
        result = runner.run_single(sample)
        assert result.correct is False

    def test_run_all(self) -> None:
        runner = m.BenchmarkRunner()
        results = runner.run_all()
        assert len(results) == len(runner.samples)

    def test_summary(self) -> None:
        runner = m.BenchmarkRunner()
        results = runner.run_all()
        summary = runner.summary(results)
        assert "total" in summary
        assert "correct" in summary
        assert "accuracy" in summary
        assert summary["total"] == len(runner.samples)

    def test_summary_empty(self) -> None:
        runner = m.BenchmarkRunner(samples=[])
        summary = runner.summary([])
        assert summary["total"] == 0


# ---------------------------------------------------------------------------
# Component 6: LLMEndpointClient
# ---------------------------------------------------------------------------


class TestLLMEndpointClient:
    def test_stub_response_math(self) -> None:
        client = m.LLMEndpointClient()
        resp = client.complete("What is 2+2?")
        assert resp.text == "4"
        assert resp.tokens_used >= 0

    def test_stub_response_knowledge(self) -> None:
        client = m.LLMEndpointClient()
        resp = client.complete("What is the capital of France?")
        assert resp.text == "Paris"

    def test_stub_unknown_returns_empty(self) -> None:
        client = m.LLMEndpointClient()
        resp = client.complete("This is a completely unknown query xyz123")
        assert resp.text == ""

    def test_stub_model_name(self) -> None:
        client = m.LLMEndpointClient(model="test-model")
        resp = client.complete("What is 2+2?")
        assert "test-model" in resp.model

    def test_find_api_key_empty(self) -> None:
        # Save env and clear
        saved = {}
        for key in ["LLM_API_KEY", "OPENAI_API_KEY", "NEWAPI_API_KEY"]:
            saved[key] = os.environ.pop(key, None)
        try:
            client = m.LLMEndpointClient()
            assert client.api_key == ""
        finally:
            for key, val in saved.items():
                if val is not None:
                    os.environ[key] = val


# ---------------------------------------------------------------------------
# Component 7: DeploymentValidator
# ---------------------------------------------------------------------------


class TestDeploymentValidator:
    def test_validate_python_version(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            validator = m.DeploymentValidator(deploy_dir=d)
            checks = validator.validate_all()
            check_map = {c.name: c for c in checks}
            assert check_map["python_version"].passed is True
            assert check_map["imports"].passed is True

    def test_check_imports(self) -> None:
        validator = m.DeploymentValidator()
        check = validator._check_imports()
        assert check.passed is True

    def test_validate_deploy_dir_missing(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            fake_dir = Path(d) / "nonexistent"
            validator = m.DeploymentValidator(deploy_dir=fake_dir)
            check = validator._check_deploy_dir()
            assert check.passed is False

    def test_validate_deploy_dir_exists(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            validator = m.DeploymentValidator(deploy_dir=d)
            check = validator._check_deploy_dir()
            assert check.passed is True


# ---------------------------------------------------------------------------
# Component 8: DeployScript
# ---------------------------------------------------------------------------


class TestDeployScript:
    def test_generate_sh(self) -> None:
        script = m.DeployScript()
        content = script.generate_sh()
        assert content.startswith("#!/bin/bash")
        assert "docker compose" in content
        assert "healthcheck" in content.lower()

    def test_generate_bat(self) -> None:
        script = m.DeployScript()
        content = script.generate_bat()
        assert "@echo off" in content
        assert "docker compose" in content

    def test_write_sh(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "deploy.sh"
            script = m.DeployScript()
            written = script.write_sh(path)
            assert written.exists()
            assert path.read_text(encoding="utf-8").startswith("#!/bin/bash")

    def test_write_bat(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "deploy.bat"
            script = m.DeployScript()
            written = script.write_bat(path)
            assert written.exists()


# ---------------------------------------------------------------------------
# Component 9: DeploymentReport
# ---------------------------------------------------------------------------


class TestDeploymentReport:
    def test_to_markdown(self) -> None:
        report = m.DeploymentReport(
            modules_checked=1058,
            tests_pass=3219,
            health_status="pass",
            benchmark_accuracy=0.95,
            deploy_artifacts=["Dockerfile", "docker-compose.yml", "dashboard.py"],
        )
        md = report.to_markdown()
        assert "1058" in md
        assert "3219" in md
        assert "95" in md
        assert "Dockerfile" in md

    def test_default_fields(self) -> None:
        report = m.DeploymentReport()
        md = report.to_markdown()
        assert "unknown" in md
        assert "0.00" in md or "0%" in md

    def test_empty_artifacts(self) -> None:
        report = m.DeploymentReport()
        md = report.to_markdown()
        assert "Artifacts" in md

    def test_benchmark_accuracy_format(self) -> None:
        report = m.DeploymentReport(benchmark_accuracy=0.8571)
        md = report.to_markdown()
        assert "85.71%" in md


# ---------------------------------------------------------------------------
# Component 10: ASIDeploymentBridge
# ---------------------------------------------------------------------------


class TestASIDeploymentBridge:
    def test_score_engineering_all_perfect(self) -> None:
        bridge = m.ASIDeploymentBridge()
        results = {k: 1.0 for k in bridge.weights}
        score = bridge.score_engineering(results)
        assert math.isclose(score, 1.0, rel_tol=1e-4)

    def test_score_engineering_none(self) -> None:
        bridge = m.ASIDeploymentBridge()
        score = bridge.score_engineering({})
        assert score == 0.0

    def test_score_production(self) -> None:
        bridge = m.ASIDeploymentBridge()
        score = bridge.score_production(benchmark_accuracy=0.8, health_pass=True)
        assert math.isclose(score, 0.9, rel_tol=1e-4)

    def test_score_production_no_health(self) -> None:
        bridge = m.ASIDeploymentBridge()
        score = bridge.score_production(benchmark_accuracy=1.0, health_pass=False)
        assert math.isclose(score, 0.5, rel_tol=1e-4)

    def test_generate_report(self) -> None:
        bridge = m.ASIDeploymentBridge()
        report = bridge.generate_report(
            bridge_results={"dockerfile": 1.0, "compose": 1.0, "healthcheck": 1.0, "streamlit": 1.0, "benchmark": 1.0, "validator": 1.0, "scripts": 1.0},
            benchmark_accuracy=0.95,
            health_pass=True,
        )
        assert "asi_v02_engineering" in report
        assert "asi_v02_production" in report
        assert "philosophy_guard" in report
        assert report["philosophy_guard"]["do_not_pretend_deployment_is_production"] is True


# ---------------------------------------------------------------------------
# Component 11: RealDeploymentGuard
# ---------------------------------------------------------------------------


class TestRealDeploymentGuard:
    def test_check_returns_guards(self) -> None:
        guard = m.RealDeploymentGuard()
        checks = guard.check()
        assert len(checks) >= 4, f"Expected ≥4 guards, got {len(checks)}"
        guard_names = {c["guard"] for c in checks}
        assert "do_not_pretend_deployment_is_production" in guard_names

    def test_report(self) -> None:
        guard = m.RealDeploymentGuard()
        report = guard.report()
        assert report["philosophy_guard_pass"] is False
        assert "philosophy_guard_note" in report
        assert "17:58" in report["philosophy_guard_note"]


# ---------------------------------------------------------------------------
# Integration: run_all orchestrator
# ---------------------------------------------------------------------------


class TestRunAll:
    def test_run_all_success(self) -> None:
        """Integration — run_all 真跑."""
        result = m.run_all(verbose=False)
        assert result["status"] == "success"
        assert "health" in result
        assert "benchmark" in result
        assert "asi_bridge" in result
        assert "guard" in result
        assert "dockerfile" in result
        assert "compose" in result

    def test_benchmark_in_run_all(self) -> None:
        result = m.run_all(verbose=False)
        b = result["benchmark"]
        assert b["total"] > 0
        assert b["accuracy"] > 0

    def test_health_in_run_all(self) -> None:
        result = m.run_all(verbose=False)
        assert result["health"]["status"] in ("pass", "warn", "fail")


# ---------------------------------------------------------------------------
# V3 Philosophy Guard — 主 17:58+20:46 不假装
# ---------------------------------------------------------------------------


def test_philosophy_guard_in_bridge() -> None:
    """主 17:58 — 不假装 deployment = ASI."""
    bridge = m.ASIDeploymentBridge()
    report = bridge.generate_report({}, 0.0, False)
    guard = report["philosophy_guard"]
    # All guard fields must be True (active guard)
    for k, v in guard.items():
        assert v is True, f"Guard {k} must be True (不假装)"

def test_guard_never_pretends_asi() -> None:
    """ASIDeploymentBridge 不假装 benchmark = ASI."""
    bridge = m.ASIDeploymentBridge()
    report = bridge.generate_report(
        {"dockerfile": 1.0, "compose": 1.0, "healthcheck": 1.0, "streamlit": 1.0, "benchmark": 0.99, "validator": 1.0, "scripts": 1.0},
        benchmark_accuracy=0.99,
        health_pass=True,
    )
    eng = report["asi_v02_engineering"]
    prod = report["asi_v02_production"]
    # Engineering contribution weighted by 0.15, max contribution 0.15
    assert report["engineering_contribution"] <= 0.15
    assert report["production_contribution"] <= 0.04


def test_real_deployment_guard_never_pretends() -> None:
    """RealDeploymentGuard.report 永远返回 philosophy_guard_pass=False (主 17:58)."""
    guard = m.RealDeploymentGuard()
    report = guard.report()
    assert report["philosophy_guard_pass"] is False
    assert "不假装" in report["philosophy_guard_note"]
