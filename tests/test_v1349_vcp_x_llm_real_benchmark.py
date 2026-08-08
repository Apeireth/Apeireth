#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1349_vcp_x_llm_real_benchmark.py — V1349 VCP × LLM Real Benchmark pytest suite.

- Tests: 30 pytest cases (constants + probe + prompt + benchmark + subscore + lift + run_full + guards).
- Goal: 0 regression against V1348 (anomaly detector) + V1084 (LLM engine).
- Import path: this file lives at `tests/test_v1349_vcp_x_llm_real_benchmark.py`,
  so it imports the canonical V1349 module via the shim
  `apeireth/v1349_vcp_x_llm_real_benchmark.py` (V1359 backward-compat shim).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

V1349_TESTS_DIR = Path(__file__).resolve().parent
V1349_APEIRETH_DIR = V1349_TESTS_DIR.parent / "apeireth"
sys.path.insert(0, str(V1349_APEIRETH_DIR))

# Import via the shim that V1356 measurement expects (V1359 plan item)
import v1349_vcp_x_llm_real_benchmark as v1349  # noqa: E402
import v1084_asi_real_llm_inference as v1084  # noqa: E402
import v1348_vcp_anomaly_detector as v1348  # noqa: E402


# --- Constants --------------------------------------------------------------

class TestV1349Constants:
    def test_version_is_semver(self):
        assert v1349.V1349_VERSION.count(".") == 2

    def test_subweights_sum_to_one(self):
        total = sum(v1349.V1349_V3_SUBWEIGHTS.values())
        assert abs(total - 1.0) < 1e-9

    def test_subweights_have_nine_keys(self):
        assert len(v1349.V1349_V3_SUBWEIGHTS) == 9

    def test_subweight_keys_match_components(self):
        expected = {
            "environment_probe", "anomaly_to_prompt", "real_inference",
            "token_cost_measure", "latency_measure", "audit_trail",
            "multi_call_aggregation", "philosophy_guards", "interoperability",
        }
        assert set(v1349.V1349_V3_SUBWEIGHTS.keys()) == expected

    def test_guards_have_five_items(self):
        assert len(v1349.V1349_GUARDS) == 5

    def test_guards_reject_consciousness_claims(self):
        # V3 哲学守门: 不假装 Phenomenal (主 17:58 + 20:46)
        # Guards should REJECT consciousness / phenomenal / sentient claims,
        # not contain them as positive attributes. Each guard should mention
        # at least one of "not"/"≠"/"no"/"reject" near these concepts.
        reject_words = ("not ", "not_", "no ", "reject", "≠", "guard_not")
        for guard in v1349.V1349_GUARDS:
            g = guard.lower()
            has_reject = any(w in g for w in reject_words)
            assert has_reject, f"guard lacks reject marker: {guard!r}"


# --- Probe ------------------------------------------------------------------

class TestProbeEndpoint:
    def test_force_mock_returns_mock_fallback(self, monkeypatch):
        # When force_mock=True, the probe should not try to connect
        cfg = v1084.LLMEndpointConfig(
            name="test", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="test-model",
        )
        result = v1349.probe_endpoint(cfg, force_mock=True)
        assert result.force_mock is True
        assert result.endpoint_name == "test"
        assert len(result.probe_id) == 16  # SHA256[:16]

    def test_probe_result_to_dict_roundtrip(self):
        cfg = v1084.LLMEndpointConfig(
            name="roundtrip", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="model-x",
        )
        result = v1349.probe_endpoint(cfg, force_mock=True)
        d = result.to_dict()
        assert d["endpoint_name"] == "roundtrip"
        assert d["force_mock"] is True
        assert "probe_id" in d


# --- Prompt builder ---------------------------------------------------------

class TestBuildAnomalyPrompt:
    def test_prompt_contains_all_plugins(self):
        report = v1349._make_synthetic_anomaly_report()
        prompt = v1349.build_anomaly_prompt(report)
        # All 4 synthetic plugins should be referenced
        for name in ("plugin.alpha", "plugin.beta", "plugin.gamma", "plugin.delta"):
            assert name in prompt

    def test_prompt_is_deterministic(self):
        # Same input → same output (主 17:43 实事求是)
        report = v1349._make_synthetic_anomaly_report()
        p1 = v1349.build_anomaly_prompt(report)
        p2 = v1349.build_anomaly_prompt(report)
        assert p1 == p2

    def test_prompt_has_known_markers(self):
        report = v1349._make_synthetic_anomaly_report()
        prompt = v1349.build_anomaly_prompt(report)
        # Should reference "anomaly" or "operator" or similar operator-brief markers
        assert any(m in prompt.lower() for m in ("anomaly", "operator", "plugin"))


# --- Benchmark --------------------------------------------------------------

class TestRunBenchmark:
    def _endpoint(self):
        return v1084.LLMEndpointConfig(
            name="bench-test", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="model-x",
        )

    def test_benchmark_with_force_mock(self, tmp_path):
        audit = tmp_path / "audit.jsonl"
        cfg = self._endpoint()
        prompt = "test prompt"
        report = v1349.run_benchmark(
            endpoint=cfg, prompt=prompt, n_calls=3,
            force_mock=True, audit_path=audit,
        )
        assert report.n_calls == 3
        assert all(m.status in ("ok", "mock", "error", "timeout") for m in report.measurements)
        assert str(report.audit_path) == str(audit)
        assert audit.exists()
        # Audit file should be valid JSONL
        lines = [l for l in audit.read_text().splitlines() if l.strip()]
        assert len(lines) == 3
        for line in lines:
            entry = json.loads(line)
            assert "call_index" in entry
            assert "request_id" in entry

    def test_benchmark_measurements_have_required_fields(self, tmp_path):
        cfg = self._endpoint()
        report = v1349.run_benchmark(
            endpoint=cfg, prompt="x", n_calls=2,
            force_mock=True, audit_path=tmp_path / "audit.jsonl",
        )
        for m in report.measurements:
            assert m.call_index >= 0
            assert m.status
            assert m.endpoint == "bench-test"
            assert m.model_id == "model-x"
            assert m.latency_ms >= 0
            assert m.cost_usd >= 0

    def test_benchmark_status_distribution_consistent(self, tmp_path):
        cfg = self._endpoint()
        report = v1349.run_benchmark(
            endpoint=cfg, prompt="x", n_calls=5,
            force_mock=True, audit_path=tmp_path / "audit.jsonl",
        )
        # n_ok + n_mock + n_error should sum to n_calls
        total = report.n_ok + report.n_mock + report.n_error
        assert total == report.n_calls


# --- Subscore ---------------------------------------------------------------

class TestV1349Subscore:
    def _perfect_inputs(self, tmp_path):
        cfg = v1084.LLMEndpointConfig(
            name="sub-test", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="m",
        )
        probe = v1349.ProbeResult(
            endpoint_name="sub-test", base_url=cfg.base_url, model_id="m",
            tcp_reachable=True, http_ok=True,
            mock_fallback_enabled=True, force_mock=False,
            api_key_set=True, probe_id="x" * 16,
        )
        report = v1349.run_benchmark(
            endpoint=cfg, prompt="x", n_calls=5,
            force_mock=True, audit_path=tmp_path / "audit.jsonl",
        )
        return probe, report

    def test_perfect_subscore_is_positive(self, tmp_path):
        probe, report = self._perfect_inputs(tmp_path)
        sub, parts = v1349.v1349_subscore(probe, report, "hash123", "report-id")
        assert 0.0 <= sub <= 1.0
        assert sub > 0.5

    def test_subscore_parts_sum_to_subscore(self, tmp_path):
        probe, report = self._perfect_inputs(tmp_path)
        sub, parts = v1349.v1349_subscore(probe, report, "h", "r")
        weighted = sum(parts[k] * v1349.V1349_V3_SUBWEIGHTS[k] for k in v1349.V1349_V3_SUBWEIGHTS)
        assert abs(weighted - sub) < 1e-9

    def test_subscore_with_unreachable_endpoint_lower(self, tmp_path):
        # All-fail probe + benchmark → lower subscore
        probe = v1349.ProbeResult(
            endpoint_name="bad", base_url="http://0.0.0.0:0/v1", model_id="m",
            tcp_reachable=False, http_ok=False,
            mock_fallback_enabled=False, force_mock=True,
            api_key_set=False, probe_id="0" * 16,
        )
        cfg = v1084.LLMEndpointConfig(
            name="bad", base_url="http://0.0.0.0:0/v1", api_key="sk", model_id="m",
        )
        report = v1349.run_benchmark(
            endpoint=cfg, prompt="x", n_calls=1,
            force_mock=True, audit_path=tmp_path / "audit.jsonl",
        )
        sub, _ = v1349.v1349_subscore(probe, report, "h", "r")
        assert sub < 1.0

    def test_subscore_requires_prompt_hash(self, tmp_path):
        probe, report = self._perfect_inputs(tmp_path)
        # Empty prompt_hash → anomaly_to_prompt = 0
        sub, parts = v1349.v1349_subscore(probe, report, "", "report-id")
        assert parts["anomaly_to_prompt"] == 0.0
        assert parts["interoperability"] == 0.0  # no prompt_hash AND report_id given


# --- ASI lift ---------------------------------------------------------------

class TestV1349AsiLift:
    def test_lift_capped_at_0_015(self):
        info = v1349.v1349_asi_lift(1.0)
        assert info["v1349_asi_lift"] == 0.015
        assert info["v1349_cap"] == 0.015

    def test_lift_zero_for_zero_subscore(self):
        info = v1349.v1349_asi_lift(0.0)
        assert info["v1349_asi_lift"] == 0.0

    def test_lift_proportional_in_middle_range(self):
        info = v1349.v1349_asi_lift(0.5)
        expected = round(min(0.015, 0.5 * 0.015), 6)
        assert info["v1349_asi_lift"] == expected


# --- run_full integration ---------------------------------------------------

class TestRunFull:
    def test_run_full_returns_five_tuple(self, tmp_path):
        cfg = v1084.LLMEndpointConfig(
            name="full", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="m",
        )
        probe, bench, sub, parts, lift = v1349.run_full(
            endpoint=cfg, n_calls=3, force_mock=True,
            report_path=tmp_path / "report.md",
            audit_path=tmp_path / "audit.jsonl",
        )
        assert isinstance(probe, v1349.ProbeResult)
        assert isinstance(bench, v1349.BenchmarkReport)
        assert 0.0 <= sub <= 1.0
        assert isinstance(parts, dict)
        assert "v1349_asi_lift" in lift
        assert lift["v1349_cap"] == 0.015

    def test_run_full_creates_audit_file(self, tmp_path):
        cfg = v1084.LLMEndpointConfig(
            name="full2", base_url="http://127.0.0.1:1/v1",
            api_key="sk-test", model_id="m",
        )
        v1349.run_full(
            endpoint=cfg, n_calls=2, force_mock=True,
            audit_path=tmp_path / "audit.jsonl",
        )
        assert (tmp_path / "audit.jsonl").exists()


# --- CLI --------------------------------------------------------------------

class TestV1349CLI:
    def test_main_help(self, capsys):
        with pytest.raises(SystemExit) as exc_info:
            v1349.main(["--help"])
        # --help exits with 0
        assert exc_info.value.code == 0
        captured = capsys.readouterr()
        assert "v1349" in captured.out.lower() or "usage" in captured.out.lower()
