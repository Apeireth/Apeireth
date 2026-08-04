"""Tests for V1263 real_kitchen_integration (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Verify:
- V1263 sanity check 14/14 PASS
- KitchenConfig dataclass works
- KitchenReport dataclass + to_dict
- import_all_real_modules loads all 4 required modules
- run_kitchen with probe-only config returns valid report
- run_kitchen with bench-only config returns valid report
- run_kitchen full dry-run returns successful end-to-end (deploy + bench + streamlit dry-run)
- render_text_report + render_json_report both work
- 5 V3 guards present
"""
from __future__ import annotations

import json
import os
import sys
import tempfile

import pytest

try:
    from apeireth import v1263_real_kitchen_integration as v63
except Exception:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
    import v1263_real_kitchen_integration as v63


def test_v1263_version():
    assert v63.V1263_VERSION == "0.1.0"


def test_v1263_v3_guards_count():
    assert len(v63.V3_GUARDS) == 5


def test_v1263_sanity_all_true():
    sc = v63.sanity_check_1263()
    assert isinstance(sc, dict)
    failed = [k for k, v in sc.items() if not v]
    assert not failed, f"failed: {failed}"
    # Verify 14 checks
    assert len(sc) >= 14


def test_v1263_import_all_real_modules():
    """真 import V1258/V1260/V1261/V1262 — 不假装."""
    result = v63.import_all_real_modules()
    assert isinstance(result, dict)
    assert "modules" in result
    for name in ("v1258_substrate_status_reporter", "v1260_docker_deploy",
                 "v1261_benchmark_llm", "v1262_streamlit_deploy"):
        info = result["modules"][name]
        assert info["ok"] is True, f"{name} import failed: {info.get('error')}"


def test_v1263_kitchen_config_defaults():
    cfg = v63.KitchenConfig()
    assert cfg.enable_substrate is True
    assert cfg.enable_environment is True
    assert cfg.base_port > 0
    assert cfg.streamlit_port > 0
    assert cfg.deploy_timeout > 0


def test_v1263_kitchen_report_to_dict():
    cfg = v63.KitchenConfig()
    r = v63.KitchenReport(
        report_id="test",
        started_at=0.0,
        ended_at=0.0,
        duration_sec=0.0,
        config=cfg.__dict__,
    )
    d = r.to_dict()
    assert d["report_id"] == "test"
    assert "stages" in d
    assert "substrate" in d
    assert "benchmark" in d
    assert "deploy_default" in d
    assert "deploy_e2e" in d
    assert "streamlit" in d
    assert "health_cycles" in d
    assert "success" in d


def test_v1263_run_kitchen_probe_only():
    """真 probe-only: substrate + environment."""
    cfg = v63.KitchenConfig(
        enable_substrate=True,
        enable_environment=True,
        enable_deploy_default=False,
        enable_deploy_e2e=False,
        enable_benchmark=False,
        enable_streamlit=False,
        enable_health_cycle=False,
    )
    with tempfile.TemporaryDirectory() as tmp:
        cfg.artifacts_dir = tmp
        report = v63.run_kitchen(cfg)
        assert isinstance(report, v63.KitchenReport)
        assert report.success is True
        assert report.substrate is not None
        assert report.substrate.get("asi_north_star") == 0.98
        assert report.environment is not None
        assert report.environment.get("python_available") is True
        assert report.benchmark is None
        assert report.streamlit is None
        assert report.deploy_default is None


def test_v1263_run_kitchen_bench_only():
    """真 bench-only: substrate + benchmark dry-run."""
    cfg = v63.KitchenConfig(
        enable_substrate=True,
        enable_environment=False,
        enable_deploy_default=False,
        enable_deploy_e2e=False,
        enable_benchmark=True,
        benchmark_dry_run=True,
        benchmark_sample_limit=3,
        enable_streamlit=False,
        enable_health_cycle=False,
    )
    with tempfile.TemporaryDirectory() as tmp:
        cfg.artifacts_dir = tmp
        report = v63.run_kitchen(cfg)
        assert isinstance(report, v63.KitchenReport)
        assert report.success is True
        assert report.substrate is not None
        assert report.benchmark is not None
        assert report.benchmark.get("dry_run") is True
        assert report.benchmark.get("sample_count") == 3
        assert len(report.benchmark.get("samples", [])) == 3
        # No deploy, no streamlit
        assert report.deploy_default is None
        assert report.streamlit is None


def test_v1263_run_kitchen_full_dry():
    """真 full dry-run: deploy + benchmark + streamlit dry-run."""
    cfg = v63.KitchenConfig(
        enable_substrate=True,
        enable_environment=True,
        enable_deploy_default=True,
        enable_deploy_e2e=False,
        enable_benchmark=True,
        benchmark_dry_run=True,
        benchmark_sample_limit=2,
        enable_streamlit=True,
        streamlit_dry_run=True,
        streamlit_real_run=False,
        enable_health_cycle=True,
        health_cycles=2,
        deploy_timeout=12.0,
        streamlit_timeout=8.0,
        base_port=18900,  # high port to avoid collision
        streamlit_port=18589,  # high port
    )
    with tempfile.TemporaryDirectory() as tmp:
        cfg.artifacts_dir = tmp
        report = v63.run_kitchen(cfg)
        assert isinstance(report, v63.KitchenReport)
        assert report.success is True, f"kitchen failed: {report.error}, stages: {[(s.stage_name, s.error) for s in report.stages]}"
        assert report.substrate is not None
        assert report.environment is not None
        assert report.deploy_default is not None
        assert report.deploy_default.get("service_count") >= 3
        assert report.benchmark is not None
        assert report.streamlit is not None
        assert report.streamlit.get("mode") == "dry_run"
        # 真 artifacts 写出来了
        json_path = os.path.join(tmp, "kitchen_report.json")
        assert os.path.exists(json_path), f"JSON artifact not written to {json_path}"


def test_v1263_render_text_report():
    """真 text 渲染 — 不假装."""
    cfg = v63.KitchenConfig(enable_benchmark=False, enable_streamlit=False, enable_deploy_default=False)
    with tempfile.TemporaryDirectory() as tmp:
        cfg.artifacts_dir = tmp
        report = v63.run_kitchen(cfg)
        text = v63.render_text_report(report)
        assert isinstance(text, str)
        assert "V1263 ASI 真生产厨房报告" in text
        assert "success:" in text
        assert "V1263 verdict:" in text


def test_v1263_render_json_report():
    """真 JSON 渲染."""
    cfg = v63.KitchenConfig(enable_benchmark=False, enable_streamlit=False, enable_deploy_default=False)
    with tempfile.TemporaryDirectory() as tmp:
        cfg.artifacts_dir = tmp
        report = v63.run_kitchen(cfg)
        js = v63.render_json_report(report)
        assert isinstance(js, str)
        # 真可 parse
        d = json.loads(js)
        assert d["report_id"] == report.report_id
        assert "substrate" in d


def test_v1263_no_pretend_modules():
    """主 17:43 实事求是: 任何 module import fail → 整体 fail."""
    # We can't easily fake an import failure here, but we can at least verify
    # that import_all_real_modules doesn't lie about success.
    result = v63.import_all_real_modules()
    for name, info in result["modules"].items():
        # If ok, ok=True means truly importable (sys.modules has it).
        if info["ok"]:
            assert info.get("version") is not None or info.get("error") is None, (
                f"{name} reports ok but neither version nor error populated"
            )
        else:
            # ok=False, error should be populated
            assert info.get("error") is not None