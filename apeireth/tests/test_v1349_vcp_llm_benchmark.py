#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Tests for v1349_vcp_llm_benchmark.py — VCP × LLM Real Benchmark (post-V1348 anomaly detector)

Chain: V1335 → ... → V1348 → V1349

V1349 = real LLM benchmark harness:
- probe endpoint (TCP + HTTP)
- build anomaly prompt from V1348 EcosystemAnomalyReport
- run N consecutive inferences via V1084 (HTTP + mock fallback)
- audit JSONL (request_hash + response_hash + ts + latency + cost)
- compute V1349 subscore 0.0-1.0 (9 components)
- compute ASI V0.3 lift (cap 0.015)

This test file covers:
- 24+ pytest tests across 6 categories: probe, prompt builder, benchmark, subscore,
  audit, integration, edge cases, philosophy guards.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

# Bootstrap import path.
_TESTS_DIR = Path(__file__).resolve().parent
_REPO_DIR = _TESTS_DIR.parent
sys.path.insert(0, str(_REPO_DIR))

import v1084_asi_real_llm_inference as v1084  # noqa: E402
import v1348_vcp_anomaly_detector as v1348  # noqa: E402
import v1349_vcp_llm_benchmark as v1349  # noqa: E402


# --- Fixtures ---------------------------------------------------------------
def _mk_endpoint(name: str = "test-endpoint", base_url: str = "http://127.0.0.1:1/v1") -> v1084.LLMEndpointConfig:
    return v1084.LLMEndpointConfig(
        name=name,
        base_url=base_url,
        api_key="sk-test",
        model_id="test-model",
        timeout_s=5.0,
        max_retries=1,
        retry_backoff_s=0.1,
        mock_fallback=True,
        input_price_per_1k=0.001,
        output_price_per_1k=0.003,
    )


def _mk_anomaly_report(
    severities: tuple = ("HIGH", "MEDIUM", "LOW", "NONE"),
) -> v1348.EcosystemAnomalyReport:
    """Build a synthetic V1348 EcosystemAnomalyReport for testing."""
    plugins = []
    for idx, sev in enumerate(severities):
        ch_name = "tier_jump"
        rec = "review tier change in V1342; document justification"
        signals = [
            v1348.ChannelSignal(
                channel=ch_name,
                signal_score=1.0 if sev == "HIGH" else (0.67 if sev == "MEDIUM" else (0.34 if sev == "LOW" else 0.0)),
                severity=sev,
                evidence={"test": True, "idx": idx},
                recommendation=rec,
            ),
        ]
        anomaly_payload = {
            "plugin": f"plugin.test_{idx}",
            "plugin_severity": sev,
            "channels": [s.to_dict() for s in signals],
        }
        plugins.append(v1348.PluginAnomaly(
            plugin=f"plugin.test_{idx}",
            plugin_severity=sev,
            plugin_severity_rank=v1348.SEVERITY_ORDER.get(sev, 0),
            channels=signals,
            anomaly_id=v1349._stable_id(anomaly_payload),
        ))
    sev_set = [p.plugin_severity for p in plugins]
    eco_sev = v1348.max_severity(sev_set)
    breakdown = {s: 0 for s in v1348.SEVERITY_ORDER.keys()}
    for s in sev_set:
        breakdown[s] = breakdown.get(s, 0) + 1
    thresholds_used = dict(v1348.DEFAULT_THRESHOLDS)
    eco_payload = {
        "per_plugin": [p.to_dict() for p in plugins],
        "ecosystem_severity": eco_sev,
        "severity_breakdown": breakdown,
        "enabled_channels": list(v1348.ALL_CHANNELS),
        "thresholds_used": thresholds_used,
    }
    return v1348.EcosystemAnomalyReport(
        per_plugin=plugins,
        ecosystem_severity=eco_sev,
        ecosystem_severity_rank=v1348.SEVERITY_ORDER.get(eco_sev, 0),
        severity_breakdown=breakdown,
        total_plugins=len(plugins),
        enabled_channels=v1348.ALL_CHANNELS,
        thresholds_used=thresholds_used,
        report_id=v1349._stable_id(eco_payload),
        generated_at="2026-08-08T23:50:00+00:00",
    )


# ============================================================================
# Category 1: Self-tests (import + module shape)
# ============================================================================
def test_v1349_imports_clean():
    """Module exposes expected symbols."""
    assert hasattr(v1349, "V1349_VERSION")
    assert hasattr(v1349, "V1349_V3_SUBWEIGHTS")
    assert hasattr(v1349, "V1349_GUARDS")
    assert hasattr(v1349, "probe_endpoint")
    assert hasattr(v1349, "build_anomaly_prompt")
    assert hasattr(v1349, "run_benchmark")
    assert hasattr(v1349, "v1349_subscore")
    assert hasattr(v1349, "v1349_asi_lift")


def test_v1349_subweights_sum_to_one():
    """Subscore weights must sum to 1.0."""
    s = sum(v1349.V1349_V3_SUBWEIGHTS.values())
    assert abs(s - 1.0) < 1e-9, f"weights sum {s} != 1.0"


def test_v1349_guards_count_five():
    """5 V3 philosophy guards."""
    assert len(v1349.V1349_GUARDS) == 5


def test_v1349_guards_are_strings():
    """Guards are non-empty strings."""
    for g in v1349.V1349_GUARDS:
        assert isinstance(g, str)
        assert len(g) > 30


def test_v1349_cap_value():
    """ASI V0.3 lift cap is 0.015 (主 22:33)."""
    info = v1349.v1349_asi_lift(1.0)
    assert info["v1349_cap"] == 0.015
    assert info["v1349_asi_lift"] == 0.015  # cap = ceiling


def test_v1349_cap_zero():
    """0 subscore → 0 lift."""
    info = v1349.v1349_asi_lift(0.0)
    assert info["v1349_asi_lift"] == 0.0


def test_v1349_cap_proportional():
    """Lift = sub * cap (clipped)."""
    info = v1349.v1349_asi_lift(0.5)
    expected = 0.5 * 0.015  # 0.0075
    assert abs(info["v1349_asi_lift"] - expected) < 1e-9


# ============================================================================
# Category 2: probe_endpoint
# ============================================================================
def test_v1349_probe_unreachable_returns_tcp_false():
    """Endpoint with bad URL → tcp_reachable=False."""
    ep = _mk_endpoint(base_url="http://127.0.0.1:1/v1")
    probe = v1349.probe_endpoint(ep)
    assert probe.tcp_reachable is False
    assert probe.http_ok is False
    assert probe.mock_fallback_enabled is True
    assert probe.api_key_set is True
    assert len(probe.probe_id) == 16


def test_v1349_probe_idempotent():
    """Same endpoint → same probe_id (content-addressed)."""
    ep = _mk_endpoint()
    p1 = v1349.probe_endpoint(ep)
    p2 = v1349.probe_endpoint(ep)
    assert p1.probe_id == p2.probe_id


def test_v1349_probe_force_mock_field():
    """force_mock flag propagates."""
    ep = _mk_endpoint()
    probe = v1349.probe_endpoint(ep, force_mock=True)
    assert probe.force_mock is True


# ============================================================================
# Category 3: AnomalyPromptBuilder
# ============================================================================
def test_v1349_build_prompt_contains_severity():
    """Prompt contains ecosystem severity string."""
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    assert "VCP Anomaly Report" in prompt
    assert rep.ecosystem_severity in prompt
    assert rep.report_id in prompt


def test_v1349_build_prompt_contains_plugins():
    """Prompt contains all plugin names sorted worst-first."""
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    for p in rep.per_plugin:
        assert p.plugin in prompt


def test_v1349_build_prompt_deterministic():
    """Same report → same prompt (no timestamps inside template body)."""
    rep = _mk_anomaly_report()
    p1 = v1349.build_anomaly_prompt(rep)
    p2 = v1349.build_anomaly_prompt(rep)
    # generated_at field is in report.to_dict() but the template uses fixed fields
    assert p1 == p2


def test_v1349_build_prompt_has_word_limit_instruction():
    """Prompt includes ≤180 words instruction (主 13:31 大胆激进 + Google SRE 启发)."""
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    assert "180" in prompt


def test_v1349_build_prompt_does_not_invent_anomalies():
    """Prompt explicitly tells LLM not to invent anomalies."""
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    assert "Do NOT invent" in prompt


# ============================================================================
# Category 4: run_benchmark
# ============================================================================
def test_v1349_run_benchmark_basic():
    """Run 3 mock calls, all should succeed (offline fallback)."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        report = v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=3,
            max_tokens=64,
            force_mock=True,
            audit_path=audit_path,
        )
    assert report.n_calls == 3
    assert report.n_mock == 3
    assert report.n_ok == 0
    assert report.n_error == 0
    assert report.total_tokens > 0
    assert report.total_cost_usd >= 0
    assert report.latency_mean_ms > 0
    assert len(report.measurements) == 3
    assert len(report.benchmark_id) == 16


def test_v1349_run_benchmark_force_mock_no_http():
    """force_mock=True → mock engine, no HTTP attempted."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        report = v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=2,
            force_mock=True,
            audit_path=audit_path,
        )
    assert all(m.status == "mock" for m in report.measurements)
    assert all(m.endpoint == ep.name for m in report.measurements)


def test_v1349_run_benchmark_writes_audit_jsonl():
    """Audit JSONL has N lines after N calls."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=4,
            force_mock=True,
            audit_path=audit_path,
        )
        lines = [l for l in audit_path.read_text(encoding="utf-8").splitlines() if l.strip()]
        assert len(lines) == 4
        for line in lines:
            entry = json.loads(line)
            assert "request_hash" in entry
            assert "response_hash" in entry
            assert "status" in entry
            assert entry["v1349_version"] == v1349.V1349_VERSION


def test_v1349_run_benchmark_latency_stats():
    """latency_mean, stddev, min, max are consistent."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        report = v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=5,
            force_mock=True,
            audit_path=audit_path,
        )
    assert report.latency_min_ms <= report.latency_mean_ms <= report.latency_max_ms
    assert report.latency_stddev_ms >= 0


# ============================================================================
# Category 5: subscore
# ============================================================================
def test_v1349_subscore_in_range():
    """subscore ∈ [0, 1]."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        bench = v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=5,
            force_mock=True,
            audit_path=audit_path,
        )
    probe = v1349.probe_endpoint(ep, force_mock=True)
    prompt_hash = "deadbeef" * 8
    sub, parts = v1349.v1349_subscore(probe, bench, prompt_hash, rep.report_id)
    assert 0.0 <= sub <= 1.0
    assert len(parts) == 9
    assert abs(sum(parts[k] * v1349.V1349_V3_SUBWEIGHTS[k] for k in v1349.V1349_V3_SUBWEIGHTS) - sub) < 1e-4


def test_v1349_subscore_components_all_present():
    """All 9 subscore components returned."""
    ep = _mk_endpoint()
    probe = v1349.probe_endpoint(ep, force_mock=True)
    rep = _mk_anomaly_report()
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        bench = v1349.run_benchmark(
            endpoint=ep,
            prompt="ping",
            n_calls=5,
            force_mock=True,
            audit_path=audit_path,
        )
    sub, parts = v1349.v1349_subscore(probe, bench, "h" * 64, rep.report_id)
    expected_keys = {
        "environment_probe",
        "anomaly_to_prompt",
        "real_inference",
        "token_cost_measure",
        "latency_measure",
        "audit_trail",
        "multi_call_aggregation",
        "philosophy_guards",
        "interoperability",
    }
    assert set(parts.keys()) == expected_keys
    assert abs(sub - round(sub, 4)) < 1e-9


def test_v1349_subscore_higher_for_more_calls():
    """5 calls → higher multi_call_aggregation than 3 calls."""
    ep = _mk_endpoint()
    probe = v1349.probe_endpoint(ep, force_mock=True)
    rep = _mk_anomaly_report()
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        bench5 = v1349.run_benchmark(ep, "p", n_calls=5, force_mock=True, audit_path=audit_path)
        sub5, parts5 = v1349.v1349_subscore(probe, bench5, "h" * 64, rep.report_id)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        bench3 = v1349.run_benchmark(ep, "p", n_calls=3, force_mock=True, audit_path=audit_path)
        sub3, parts3 = v1349.v1349_subscore(probe, bench3, "h" * 64, rep.report_id)
    assert parts5["multi_call_aggregation"] > parts3["multi_call_aggregation"]


# ============================================================================
# Category 6: integration with V1348 + V1084
# ============================================================================
def test_v1349_default_endpoint_real_config():
    """Default endpoint is honest (api_key=sk-replace-me, no api_key_set)."""
    ep = v1349._make_default_endpoint()
    assert ep.mock_fallback is True
    probe = v1349.probe_endpoint(ep)
    assert probe.api_key_set is False  # honest: not set


def test_v1349_synthetic_anomaly_report_is_valid_v1348():
    """Synthetic report conforms to V1348 invariants."""
    rep = v1349._make_synthetic_anomaly_report()
    assert rep.ecosystem_severity in v1348.SEVERITY_ORDER
    assert rep.total_plugins == len(rep.per_plugin)
    # All plugin anomaly_ids are 16 hex chars
    for p in rep.per_plugin:
        assert len(p.anomaly_id) == 16
        assert all(c in "0123456789abcdef" for c in p.anomaly_id)
    # All channels must be valid
    valid_channels = set(v1348.ALL_CHANNELS)
    for p in rep.per_plugin:
        for ch in p.channels:
            assert ch.channel in valid_channels


def test_v1349_run_full_produces_report():
    """run_full writes report file with all required sections."""
    ep = _mk_endpoint()
    with tempfile.TemporaryDirectory() as tmp:
        report_path = Path(tmp) / "report.md"
        audit_path = Path(tmp) / "audit.jsonl"
        probe, bench, sub, parts, lift = v1349.run_full(
            endpoint=ep,
            n_calls=3,
            max_tokens=64,
            force_mock=True,
            report_path=report_path,
            audit_path=audit_path,
        )
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1349 VCP × LLM Real Benchmark Report" in content
        assert "Environment Probe" in content
        assert "Anomaly Source" in content
        assert "V1349 Subscore" in content
        assert "V1349 → ASI V0.3 Lift" in content
        assert "V3 Philosophy Guards" in content
        assert audit_path.exists()
    assert 0.0 <= sub <= 1.0
    assert lift["v1349_cap"] == 0.015


# ============================================================================
# Category 7: edge cases
# ============================================================================
def test_v1349_anomaly_report_with_only_none_severity():
    """All-NONE anomaly report → ecosystem_severity = NONE."""
    rep = _mk_anomaly_report(severities=("NONE", "NONE"))
    assert rep.ecosystem_severity == v1348.SEVERITY_NONE
    # Prompt still generated without crash
    prompt = v1349.build_anomaly_prompt(rep)
    assert "NONE" in prompt


def test_v1349_stable_id_format():
    """_stable_id produces 16 hex chars."""
    sid = v1349._stable_id({"a": 1, "b": "test"})
    assert len(sid) == 16
    assert all(c in "0123456789abcdef" for c in sid)


def test_v1349_stable_id_order_invariant():
    """_stable_id is sort_keys based (order-invariant)."""
    a = v1349._stable_id({"a": 1, "b": 2})
    b = v1349._stable_id({"b": 2, "a": 1})
    assert a == b


def test_v1349_no_network_in_unit_test_path():
    """run_benchmark with force_mock=True makes zero HTTP calls (V3 哲学守门)."""
    ep = _mk_endpoint()
    rep = _mk_anomaly_report()
    prompt = v1349.build_anomaly_prompt(rep)
    with tempfile.TemporaryDirectory() as tmp:
        audit_path = Path(tmp) / "audit.jsonl"
        report = v1349.run_benchmark(
            endpoint=ep,
            prompt=prompt,
            n_calls=2,
            force_mock=True,
            audit_path=audit_path,
        )
    # All measurements should have status="mock" and no error
    assert all(m.status == "mock" for m in report.measurements)
    assert all(m.error is None for m in report.measurements)
