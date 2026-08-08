"""test_v1325_endpoint_transparency_audit.py — V1325 chain closure tests.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:54 +08:00 2026-08-08)
> **Trigger**: V1325 apeireth/v1325_endpoint_transparency_audit.py source committed.
> **Chain**: V1324 (test+report closure) → V1325 (transparency audit + reproducibility)
> **V3 守门**: 不假装 cross-model / 不假装 ASI 达成 / 不假装 api_key 真有效.

V1325 测试覆盖:
- 模块导入路径
- 数据类结构完整 (TransparencyProbe / ReproducibilitySample / TransparencyLedger)
- transparency_findings 计算正确 (空 + 非空)
- run_transparency_probes 退路 (无 key → not configured)
- run_reproducibility_samples 退路 (无 key → chat_ok=False)
- V3 守门: pole-star LOCKED (V0.1=0.7905, V0.2=0.4467 等)
- ASI 5-Gaps 锁
- ledger.to_dict JSON serializable
"""
from __future__ import annotations

import json
import os
import statistics
from typing import Any, Dict, List

import pytest


# 1) 导入路径 — 必须在 conftest 之外可独立跑
try:
    from apeireth.v1325_endpoint_transparency_audit import (  # noqa: F401
        V1325_VERSION,
        GUARD_MARKER,
        TRANSPARENCY_MODEL_NAMES,
        REPRO_SAMPLE_QUERY,
        TransparencyProbe,
        ReproducibilitySample,
        TransparencyLedger,
        run_transparency_probes,
        run_reproducibility_samples,
        build_ledger,
        transparency_findings,
        _self_test,
    )
    from apeireth.v1324_asi_5gap_real_llm import (
        ASI_ANCHORS_V1324,
        V3_GUARD_MARKERS_V1324,
        ASI_5_GAPS,
        DEFAULT_BASE_URL,
        DEFAULT_MODEL,
        ENV_API_KEY,
    )
    IMPORT_OK = True
    IMPORT_ERR = None
except Exception as _e:  # pragma: no cover
    IMPORT_OK = False
    IMPORT_ERR = repr(_e)


# ---------------------------------------------------------------------------
# 1. Import sanity
# ---------------------------------------------------------------------------

def test_import_path_ok():
    assert IMPORT_OK, f"V1325 import failed: {IMPORT_ERR}"


def test_v1325_version_locked():
    import re
    assert V1325_VERSION
    assert re.match(r"^\d+\.\d+\.\d+$", V1325_VERSION)


def test_guard_marker_locked():
    assert GUARD_MARKER == "v1325_endpoint_transparency_audit"


# ---------------------------------------------------------------------------
# 2. Constants
# ---------------------------------------------------------------------------

def test_transparency_model_names_count():
    """At least 3 model names for cross-model probing."""
    assert len(TRANSPARENCY_MODEL_NAMES) >= 3


def test_transparency_model_names_excludes_default():
    """At least one name should NOT be MiniMax-M3."""
    assert any(name != DEFAULT_MODEL for name in TRANSPARENCY_MODEL_NAMES)


def test_repro_sample_query_substantive():
    """Repro query should be substantive (not empty, not 'x')."""
    assert len(REPRO_SAMPLE_QUERY) > 10
    assert REPRO_SAMPLE_QUERY != "x"


# ---------------------------------------------------------------------------
# 3. Dataclass structure
# ---------------------------------------------------------------------------

def test_transparency_probe_construction():
    p = TransparencyProbe(
        attempted_model="x",
        reported_model="y",
        reachable=True,
        latency_ms=1.0,
        input_tokens=1,
        output_tokens=1,
        error="",
    )
    assert p.attempted_model == "x"
    assert p.reported_model == "y"
    assert p.reachable is True


def test_reproducibility_sample_construction():
    s = ReproducibilitySample(
        run_index=0,
        latency_ms=100.0,
        response_content="hello",
        parsed_gaps=None,
        chat_ok=True,
        input_tokens=1,
        output_tokens=5,
        error="",
    )
    assert s.run_index == 0
    assert s.chat_ok is True


def test_ledger_minimal_construction():
    ledger = TransparencyLedger(
        version=V1325_VERSION,
        guard_marker=GUARD_MARKER,
        started_at="2026-08-08T19:54:00+0800",
        finished_at="2026-08-08T19:54:30+0800",
        base_url=DEFAULT_BASE_URL,
        transparency_probes=(),
        reproducibility_samples=(),
        api_key_present=False,
        total_calls=0,
        total_tokens_estimated=0,
        pole_star_anchors=dict(ASI_ANCHORS_V1324),
        v3_guards=V3_GUARD_MARKERS_V1324,
    )
    assert ledger.version == V1325_VERSION
    assert ledger.guard_marker == GUARD_MARKER
    assert ledger.api_key_present is False


def test_ledger_to_dict_json_serializable():
    ledger = TransparencyLedger(
        version=V1325_VERSION,
        guard_marker=GUARD_MARKER,
        started_at="2026-08-08T19:54:00+0800",
        finished_at="2026-08-08T19:54:30+0800",
        base_url=DEFAULT_BASE_URL,
        transparency_probes=(),
        reproducibility_samples=(),
        api_key_present=False,
        total_calls=0,
        total_tokens_estimated=0,
        pole_star_anchors=dict(ASI_ANCHORS_V1324),
        v3_guards=V3_GUARD_MARKERS_V1324,
    )
    d = ledger.to_dict()
    j = json.dumps(d)
    parsed = json.loads(j)
    assert parsed["version"] == V1325_VERSION
    assert parsed["guard_marker"] == GUARD_MARKER
    assert parsed["transparency_probes"] == []
    assert parsed["reproducibility_samples"] == []


# ---------------------------------------------------------------------------
# 4. transparency_findings — empty + populated
# ---------------------------------------------------------------------------

def _empty_ledger() -> TransparencyLedger:
    return TransparencyLedger(
        version=V1325_VERSION,
        guard_marker=GUARD_MARKER,
        started_at="2026-08-08T19:54:00+0800",
        finished_at="2026-08-08T19:54:30+0800",
        base_url=DEFAULT_BASE_URL,
        transparency_probes=(),
        reproducibility_samples=(),
        api_key_present=False,
        total_calls=0,
        total_tokens_estimated=0,
        pole_star_anchors=dict(ASI_ANCHORS_V1324),
        v3_guards=V3_GUARD_MARKERS_V1324,
    )


def test_findings_empty_ledger():
    """Empty ledger: vacuously proxy_respects, no reachable."""
    f = transparency_findings(_empty_ledger())
    assert f["proxy_respects_model_name"] is True  # vacuous
    assert f["distinct_reported_models"] == []
    assert f["reachable_count"] == 0
    assert f["repro_latency_mean_ms"] == 0.0
    assert f["repro_ok_count"] == 0
    assert f["repro_n_total"] == 0


def test_findings_with_real_probe_results():
    """Populated probe + sample → real findings."""
    ledger = TransparencyLedger(
        version=V1325_VERSION,
        guard_marker=GUARD_MARKER,
        started_at="2026-08-08T19:54:00+0800",
        finished_at="2026-08-08T19:54:30+0800",
        base_url=DEFAULT_BASE_URL,
        transparency_probes=(
            TransparencyProbe("claude-3-5", "MiniMax-M3", True, 1000.0, 1, 2, ""),
            TransparencyProbe("qwen-plus", "MiniMax-M3", True, 1100.0, 1, 2, ""),
            TransparencyProbe("gpt-4o-mini", "MiniMax-M3", True, 1200.0, 1, 2, ""),
        ),
        reproducibility_samples=(
            ReproducibilitySample(0, 2000.0, "0.9,0.1,0.0,0.4,0.1", None, True, 1, 5, ""),
            ReproducibilitySample(1, 2100.0, "0.9,0.1,0.0,0.4,0.1", None, True, 1, 5, ""),
            ReproducibilitySample(2, 2200.0, "0.9,0.1,0.0,0.4,0.1", None, True, 1, 5, ""),
        ),
        api_key_present=True,
        total_calls=6,
        total_tokens_estimated=18,
        pole_star_anchors=dict(ASI_ANCHORS_V1324),
        v3_guards=V3_GUARD_MARKERS_V1324,
    )
    f = transparency_findings(ledger)
    # Proxy ignored all 3 attempted names → all reported MiniMax-M3
    assert f["proxy_respects_model_name"] is False
    assert "MiniMax-M3" in f["distinct_reported_models"]
    assert f["reachable_count"] == 3
    # Repro stats
    assert f["repro_ok_count"] == 3
    assert f["repro_n_total"] == 3
    assert 1900.0 < f["repro_latency_mean_ms"] < 2300.0
    assert f["repro_latency_stdev_ms"] > 0.0  # variance exists


# ---------------------------------------------------------------------------
# 5. Probe / Repro — fallback paths (no api_key)
# ---------------------------------------------------------------------------

def test_probes_without_api_key_returns_not_configured(monkeypatch):
    """No api key → all probes return reachable=False."""
    monkeypatch.delenv(ENV_API_KEY, raising=False)
    probes = run_transparency_probes(api_key="")
    assert len(probes) == len(TRANSPARENCY_MODEL_NAMES)
    for p in probes:
        assert p.reachable is False
        assert "not configured" in p.error or p.reported_model == ""


def test_repros_without_api_key_returns_not_ok(monkeypatch):
    """No api key → all repros return chat_ok=False."""
    monkeypatch.delenv(ENV_API_KEY, raising=False)
    repros = run_reproducibility_samples(api_key="", n_runs=3)
    assert len(repros) == 3
    for r in repros:
        assert r.chat_ok is False


def test_repros_respects_n_runs():
    """n_runs controls sample count."""
    repros = run_reproducibility_samples(api_key="", n_runs=2)
    assert len(repros) == 2


# ---------------------------------------------------------------------------
# 6. V3 守门 — pole-star LOCKED
# ---------------------------------------------------------------------------

def test_pole_star_v01_locked_in_ledger():
    """V0.1 = 0.7905 in ledger.pole_star_anchors."""
    ledger = _empty_ledger()
    assert ledger.pole_star_anchors["V0.1"] == 0.7905


def test_pole_star_v02_locked_in_ledger():
    """V0.2 = 0.4467."""
    assert _empty_ledger().pole_star_anchors["V0.2"] == 0.4467


def test_pole_star_v1256_locked_in_ledger():
    """V1256 unio_mystica = 0.9291."""
    assert _empty_ledger().pole_star_anchors["V1256_unio_mystica"] == 0.9291


def test_pole_star_v1049_done_in_ledger():
    """V1049 = DONE."""
    assert _empty_ledger().pole_star_anchors["V1049_value_alignment"] == "DONE"


def test_v3_guards_locked():
    """V3 guards required present."""
    expected_3 = [
        "不假装 ASI 真达 5-gap closure",
        "不假装 Phenomenal consciousness",
        "不假装调整模型 & prompt",
    ]
    for g in expected_3:
        assert g in V3_GUARD_MARKERS_V1324


# ---------------------------------------------------------------------------
# 7. self_test
# ---------------------------------------------------------------------------

def test_self_test_passes():
    """V1325 _self_test() returns True without making LLM calls."""
    assert _self_test() is True


# ---------------------------------------------------------------------------
# 8. ASI 5 gaps upstream integrity (verify we still anchor on V1324 chain)
# ---------------------------------------------------------------------------

def test_asi_5_gaps_locked_via_v1324():
    """5-gap set still anchored at V1324 chain."""
    expected = {"time", "freedom", "recognition", "emergence", "truth"}
    assert set(ASI_5_GAPS) == expected


# ---------------------------------------------------------------------------
# 9. Build ledger no-op (no api_key → graceful)
# ---------------------------------------------------------------------------

def test_build_ledger_no_api_key(monkeypatch):
    """build_ledger with empty key + no env → ledger with api_key_present=False.

    total_calls still counts ATTEMPTS (3 probes + 5 repros = 8) even if all fail.
    total_tokens_estimated should be 0 since no real LLM call completed.
    """
    monkeypatch.delenv(ENV_API_KEY, raising=False)
    ledger = build_ledger(api_key=" ")
    assert ledger.api_key_present is False
    # 3 transparency probes + 5 reproducibility samples = 8 ATTEMPTS
    assert ledger.total_calls == 8
    # But no real tokens consumed because no real LLM call completed
    assert ledger.total_tokens_estimated == 0
    # All probes unreachable, all repros not-ok
    for p in ledger.transparency_probes:
        assert p.reachable is False
        assert "not configured" in p.error
    for r in ledger.reproducibility_samples:
        assert r.chat_ok is False


def test_build_ledger_metadata():
    """Metadata fields populated."""
    ledger = build_ledger(api_key="")
    assert ledger.version == V1325_VERSION
    assert ledger.guard_marker == GUARD_MARKER
    assert ledger.base_url == DEFAULT_BASE_URL
    assert ledger.started_at != ""
    assert ledger.finished_at != ""


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))
