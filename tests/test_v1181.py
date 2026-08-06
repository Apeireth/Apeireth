"""V1181 — ASI Real Docker Compose alt-runtime (V1050 spec) tests.

主 17:43 实事求是 + 主 17:58+20:46 不假装 + 主 00:56 任何人都能接手.

Tests:
  - compose parse real (3 files, 19 services expected)
  - subprocess boot real (alt runtime, 19 children)
  - port listen real (socket connect)
  - http probe real (/health 200)
  - graceful shutdown real (exit code 0)
  - alt runtime ≠ docker (compat factor 0.9)
  - YAML parser edge cases
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

import pytest

# 确保 apeireth 在 path 里
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1181_asi_docker_compose_real_v1050spec import (  # noqa: E402
    ALL_SUBDIMS,
    SUBDIM_BOOT,
    SUBDIM_LISTEN,
    SUBDIM_PARSE,
    SUBDIM_PROBE,
    SUBDIM_SHUTDOWN,
    V1181_COMPAT_FACTOR,
    V1181_EXPECTED_SERVICES,
    V1181_VERSION,
    ComposeService,
    V1181DockerComposeRealV1050Spec,
    V1181Report,
    _extract_host_port,
    _parse_simple_yaml,
    measure_real_compose_v1050,
)


# ---------------------------------------------------------------------------
# 1. YAML parser tests
# ---------------------------------------------------------------------------


class TestYamlParser:
    def test_parse_simple_keyvalue(self):
        text = "key: value\nfoo: bar\n"
        result = _parse_simple_yaml(text)
        assert result == {"key": "value", "foo": "bar"}

    def test_parse_nested_dict(self):
        text = "outer:\n  inner: 1\n  deeper:\n    key: val\n"
        result = _parse_simple_yaml(text)
        assert result == {"outer": {"inner": 1, "deeper": {"key": "val"}}}

    def test_parse_list(self):
        text = "items:\n  - a\n  - b\n  - c\n"
        result = _parse_simple_yaml(text)
        assert result == {"items": ["a", "b", "c"]}

    def test_parse_list_of_dicts(self):
        text = "ports:\n  - 8765:8765\n  - 9000:9000\n"
        result = _parse_simple_yaml(text)
        assert result == {"ports": ["8765:8765", "9000:9000"]}

    def test_parse_with_comment(self):
        text = "# this is a comment\nkey: value  # inline\n"
        result = _parse_simple_yaml(text)
        assert result == {"key": "value"}

    def test_parse_docker_compose_minimal(self):
        text = """services:
  asi-api:
    build:
      context: ..
    image: apeireth/asi:0.1
    ports:
      - "8765:8765"
    environment:
      ASI_VERSION: "0.1.0"
"""
        result = _parse_simple_yaml(text)
        assert "services" in result
        assert "asi-api" in result["services"]
        svc = result["services"]["asi-api"]
        assert svc["image"] == "apeireth/asi:0.1"
        assert "ports" in svc

    def test_parse_booleans_and_nulls(self):
        text = "a: true\nb: false\nc: null\nd: ~\n"
        result = _parse_simple_yaml(text)
        assert result == {"a": True, "b": False, "c": None, "d": None}

    def test_parse_quoted_strings(self):
        text = 'a: "x y"\nb: \'literal\'\n'
        result = _parse_simple_yaml(text)
        assert result == {"a": "x y", "b": "literal"}

    def test_parse_numbers(self):
        text = "intval: 42\nfloatval: 3.14\nneg: -1\n"
        result = _parse_simple_yaml(text)
        assert result["intval"] == 42
        assert result["floatval"] == 3.14
        assert result["neg"] == -1


class TestExtractHostPort:
    def test_string_port_mapping(self):
        assert _extract_host_port(["8765:8765"]) == 8765
        assert _extract_host_port(["9000:9000", "9100:9100"]) == 9000

    def test_dict_port_mapping(self):
        assert _extract_host_port([{"published": 8765, "target": 8765}]) == 8765

    def test_no_ports(self):
        assert _extract_host_port([]) is None
        assert _extract_host_port(None) is None

    def test_invalid_string(self):
        assert _extract_host_port(["abc"]) is None

    def test_invalid_dict(self):
        assert _extract_host_port([{"published": "abc"}]) is None


# ---------------------------------------------------------------------------
# 2. ComposeService dataclass
# ---------------------------------------------------------------------------


class TestComposeService:
    def test_to_dict(self):
        svc = ComposeService(
            name="asi-core",
            source_file="deploy/docker-compose.yml",
            host_port=8765,
            image="apeireth/asi:0.1",
            has_healthcheck=True,
        )
        d = svc.to_dict()
        assert d["name"] == "asi-core"
        assert d["host_port"] == 8765
        assert d["has_healthcheck"] is True


# ---------------------------------------------------------------------------
# 3. V1181Runner — main integration tests (slower, real subprocess)
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def v1181_runner():
    """Module-scoped runner to avoid restart 19 subprocess per test."""
    runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
    return runner


class TestV1181Parse:
    def test_parse_returns_list(self, v1181_runner):
        services = v1181_runner.parse_compose_files()
        assert isinstance(services, list)
        assert len(services) > 0

    def test_parse_finds_19_services(self, v1181_runner):
        services = v1181_runner.parse_compose_files()
        assert len(services) == V1181_EXPECTED_SERVICES

    def test_parse_services_have_required_fields(self, v1181_runner):
        services = v1181_runner.parse_compose_files()
        for svc in services:
            assert isinstance(svc.name, str) and svc.name
            assert isinstance(svc.host_port, int) and svc.host_port > 0
            assert svc.source_file.endswith(".yml")

    def test_parse_sources_are_3_files(self, v1181_runner):
        services = v1181_runner.parse_compose_files()
        sources = sorted({s.source_file for s in services})
        assert len(sources) == 3
        assert any("docker-compose.yml" in s for s in sources)
        assert any("group-a.yml" in s for s in sources)
        assert any("group-b.yml" in s for s in sources)


class TestV1181Boot:
    def test_boot_subprocess_alive(self, v1181_runner):
        if v1181_runner.children:
            # already booted (from prior test)
            return
        v1181_runner.boot_services()
        assert len(v1181_runner.children) == V1181_EXPECTED_SERVICES
        # 给子进程一点时间启动
        time.sleep(0.6)
        # 大多数应该还活着
        alive = sum(1 for p in v1181_runner.children if p.poll() is None)
        assert alive >= V1181_EXPECTED_SERVICES - 1  # 允许 1 个 race


class TestV1181PortListen:
    def test_verify_ports_listening_returns_19(self, v1181_runner):
        if not v1181_runner.children:
            v1181_runner.boot_services()
            time.sleep(0.6)
        n = v1181_runner.verify_ports_listening()
        assert n == V1181_EXPECTED_SERVICES


class TestV1181HttpProbe:
    def test_http_probe_health_returns_19(self, v1181_runner):
        if not v1181_runner.children:
            v1181_runner.boot_services()
            time.sleep(0.6)
        n = v1181_runner.http_probe_health()
        assert n == V1181_EXPECTED_SERVICES


class TestV1181Shutdown:
    def test_graceful_shutdown_all_returns_19(self, v1181_runner):
        if not v1181_runner.children:
            v1181_runner.boot_services()
            time.sleep(0.6)
        n = v1181_runner.graceful_shutdown_all(grace_seconds=3.0)
        assert n == V1181_EXPECTED_SERVICES
        # 子进程都已退出
        for proc in v1181_runner.children:
            assert proc.poll() is not None
        # 清理 runner 状态, 避免后续测试复用
        v1181_runner.children = []


# ---------------------------------------------------------------------------
# 4. measure_full integration
# ---------------------------------------------------------------------------


class TestV1181MeasureFull:
    def test_measure_full_runs(self):
        runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
        report = runner.measure_full()
        assert isinstance(report, V1181Report)
        assert report.version == V1181_VERSION
        assert 0.0 <= report.total <= 1.0
        runner.cleanup()

    def test_measure_full_5_subdim_present(self):
        runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
        report = runner.measure_full()
        for k in ALL_SUBDIMS:
            assert k in report.sub_dim_scores
            assert 0.0 <= report.sub_dim_scores[k] <= 1.0
        runner.cleanup()

    def test_measure_full_total_uses_compat_factor(self):
        runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
        report = runner.measure_full()
        # 5 sub-dim 都 1.0 → total = 1.0 × 0.9 = 0.9
        raw_mean = sum(report.sub_dim_scores.values()) / len(report.sub_dim_scores)
        expected = round(raw_mean * V1181_COMPAT_FACTOR, 4)
        assert report.total == expected
        runner.cleanup()

    def test_measure_full_services_count(self):
        runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
        report = runner.measure_full()
        assert report.n_services_total == V1181_EXPECTED_SERVICES
        runner.cleanup()


# ---------------------------------------------------------------------------
# 5. 哲学守门 (主 17:58+20:46 不假装)
# ---------------------------------------------------------------------------


class TestV1181PhilosophyGuard:
    def test_v3_guards_present(self):
        from apeireth.v1181_asi_docker_compose_real_v1050spec import V3_GUARDS
        assert "alt_is_not_docker" in V3_GUARDS
        assert "compat_factor_honest" in V3_GUARDS

    def test_compat_factor_is_0_9(self):
        # 主 17:58 不假装: alt runtime ≠ docker, 0.9 标注
        assert V1181_COMPAT_FACTOR == 0.9

    def test_expected_services_19(self):
        # V1132 (1) + group-a (9) + group-b (9) = 19
        assert V1181_EXPECTED_SERVICES == 19

    def test_alt_runtime_does_not_pretend_docker(self):
        # measure_full 报告里必须有 notes 标注 alt ≠ docker
        runner = V1181DockerComposeRealV1050Spec(repo_root=str(ROOT))
        report = runner.measure_full()
        joined = " ".join(report.notes).lower()
        assert "alt" in joined
        assert "docker" in joined
        assert "0.9" in joined or "compat" in joined
        runner.cleanup()


# ---------------------------------------------------------------------------
# 6. CLI & artifact (smoke)
# ---------------------------------------------------------------------------


class TestV1181CLI:
    def test_main_no_args_runs(self, capsys):
        from apeireth.v1181_asi_docker_compose_real_v1050spec import main
        rc = main([])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1181" in out
        assert "Total" in out

    def test_main_json_runs(self, capsys):
        from apeireth.v1181_asi_docker_compose_real_v1050spec import main
        rc = main(["--json"])
        assert rc == 0
        out = capsys.readouterr().out
        # 验证是合法 JSON
        data = json.loads(out)
        assert "total" in data
        assert "sub_dim_scores" in data
        assert "snapshot_id" in data


# ---------------------------------------------------------------------------
# 7. measure_real_compose_v1050 主入口
# ---------------------------------------------------------------------------


class TestV1181MainEntry:
    def test_measure_returns_float(self):
        score = measure_real_compose_v1050()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0