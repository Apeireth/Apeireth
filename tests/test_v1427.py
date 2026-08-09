"""Tests for V1427 — ASI 总框架 stage-delivery report V1411-V1426."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1427_asi_stage_delivery as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1427_VERSION == "0.1.0"
    assert m.V1427_SCHEMA == "v1427.asi-stage-delivery/v1"
    assert m.V1427_MODULE == "v1427_asi_stage_delivery"


def test_upstream_modules_count():
    """Must contain 16 modules (V1411-V1426)."""
    assert len(m.UPSTREAM_MODULES) == 16


def test_upstream_modules_covers_v1411_v1426():
    versions = sorted({mod.split("_")[0] for mod in m.UPSTREAM_MODULES})
    expected = [f"v14{i:02d}" for i in range(11, 27)]
    assert versions == expected


def test_upstream_modules_contains_v1424_v1425_v1426():
    assert "v1424_asi_real_llm_benchmark" in m.UPSTREAM_MODULES
    assert "v1425_asi_five_philosophical_gaps" in m.UPSTREAM_MODULES
    assert "v1426_vcp_six_protocol_dispatcher" in m.UPSTREAM_MODULES


def test_guards_well_formed():
    assert len(m.V1427_GUARDS) >= 14
    expected = {
        "GUARD_NO_UPSTREAM_WRITE",
        "GUARD_COVERAGE_DEFINED",
        "GUARD_REPORT_REAL",
        "GUARD_JSON_ATOMIC",
        "GUARD_MD_RENDERED",
        "GUARD_FRAMES_LISTED",
        "GUARD_CHAIN_OK",
        "GUARD_PROBES_COLLECTED",
        "GUARD_PROTOCOLS_COLLECTED",
        "GUARD_BENCHMARK_INCLUDED",
        "GUARD_POPPER_RUNS",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_CLI_RUNNABLE",
        "GUARD_NO_FAKE_SCORE",
    }
    for g in expected:
        assert g in m.V1427_GUARDS


def test_v3_guards_well_formed():
    assert len(m.V1427_V3_GUARDS) >= 5


def test_borrowed_real():
    assert len(m.V1427_BORROWED) >= 12
    keys = [b[0] for b in m.V1427_BORROWED]
    for v in ("V1411", "V1418", "V1424", "V1425", "V1426"):
        assert v in keys


# ============================================================================
# Config
# ============================================================================


def test_build_default_config():
    cfg = m.build_default_config()
    assert "benchmark_jsonl" in cfg
    assert "v1425_report" in cfg
    assert "v1426_report" in cfg
    assert "report_path" in cfg
    assert "md_path" in cfg


def test_validate_config_accepts_default():
    cfg = m.build_default_config()
    cfg2 = m.validate_config(cfg)
    assert cfg2 is cfg


def test_validate_config_rejects_missing_key():
    with pytest.raises(ValueError):
        m.validate_config({})


# ============================================================================
# Helpers
# ============================================================================


def test_compute_coverage_empty_zero():
    assert m.compute_coverage({}) == 0.0


def test_compute_coverage_all_ok_one():
    fake = {f"m{i}": {"ok": True} for i in range(16)}
    assert m.compute_coverage(fake) == 1.0


def test_compute_coverage_half_half():
    fake = {f"m{i}": {"ok": i % 2 == 0} for i in range(16)}
    assert abs(m.compute_coverage(fake) - 0.5) < 1e-9


def test_atomic_write_json(tmp_path):
    p = tmp_path / "test.json"
    m._atomic_write_json(p, {"x": 1, "y": [1, 2, 3]})
    assert p.exists()
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data == {"x": 1, "y": [1, 2, 3]}


def test_safe_load_jsonl_nonexistent(tmp_path):
    p = tmp_path / "nope.jsonl"
    assert m._safe_load_jsonl(p) == []


def test_safe_load_jsonl_skips_malformed(tmp_path):
    p = tmp_path / "x.jsonl"
    p.write_text('{"a": 1}\nnot json\n{"b": 2}\n', encoding="utf-8")
    loaded = m._safe_load_jsonl(p)
    assert len(loaded) == 2


def test_safe_load_json_nonexistent(tmp_path):
    p = tmp_path / "nope.json"
    assert m._safe_load_json(p) is None


# ============================================================================
# Collectors
# ============================================================================


def test_collect_chain_status_returns_16_modules():
    cs = m.collect_chain_status()
    assert isinstance(cs, dict)
    assert len(cs) == 16


def test_collect_chain_status_has_ok_field():
    cs = m.collect_chain_status()
    for k, v in cs.items():
        assert "ok" in v


def test_collect_probe_results_returns_dict():
    pr = m.collect_probe_results()
    assert isinstance(pr, dict)


def test_collect_probe_results_has_5_gaps():
    pr = m.collect_probe_results()
    for gap in ("time", "freedom", "recognition", "emergence", "truth"):
        assert gap in pr


def test_collect_protocol_results_returns_dict():
    pcr = m.collect_protocol_results()
    assert isinstance(pcr, dict)


def test_collect_protocol_results_has_6_protocols():
    pcr = m.collect_protocol_results()
    protocols = pcr.get("protocols", [])
    assert len(protocols) == 6


def test_collect_benchmark_stats_returns_dict():
    cfg = m.build_default_config()
    bs = m.collect_benchmark_stats(cfg)
    assert isinstance(bs, dict)
    assert "n_records" in bs
    assert "n_correct" in bs
    assert "accuracy" in bs
    assert "total_cost_usd" in bs


def test_collect_benchmark_stats_accuracy_in_range():
    cfg = m.build_default_config()
    bs = m.collect_benchmark_stats(cfg)
    assert 0.0 <= bs["accuracy"] <= 1.0


# ============================================================================
# build_stage_report
# ============================================================================


def test_build_stage_report_returns_dataclass():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    assert isinstance(report, m.StageDeliveryReport)


def test_build_stage_report_n_modules_total_16():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    assert report.n_modules_total == 16


def test_build_stage_report_coverage_in_range():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    assert 0.0 <= report.coverage <= 1.0


def test_build_stage_report_started_before_ended():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    assert report.started_iso <= report.ended_iso


def test_build_stage_report_includes_chain_probes_protocols_benchmark():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    assert len(report.chain_status) == 16
    assert "time" in report.probe_results
    assert "protocols" in report.protocol_results
    assert report.benchmark_stats["n_records"] >= 0


def test_stage_delivery_report_to_dict():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    d = report.to_dict()
    assert "coverage" in d
    assert "chain_status" in d
    assert "probe_results" in d
    assert "protocol_results" in d
    assert "benchmark_stats" in d


# ============================================================================
# render_report_md
# ============================================================================


def test_render_report_md_returns_string():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    md = m.render_report_md(report)
    assert isinstance(md, str)
    assert len(md) > 100


def test_render_report_md_contains_all_16_modules():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    md = m.render_report_md(report)
    for modname in m.UPSTREAM_MODULES:
        assert modname in md


def test_render_report_md_contains_5_gaps():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    md = m.render_report_md(report)
    for gap in ("time", "freedom", "recognition", "emergence", "truth"):
        assert gap in md


def test_render_report_md_contains_6_protocols():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    md = m.render_report_md(report)
    for proto in ("sync", "async", "static", "service", "preprocessor", "hybrid"):
        assert proto in md


def test_render_report_md_contains_coverage_label():
    cfg = m.build_default_config()
    cfg = m.validate_config(cfg)
    report = m.build_stage_report(cfg)
    md = m.render_report_md(report)
    assert "coverage" in md.lower()


# ============================================================================
# popper_self_test
# ============================================================================


def test_popper_self_test_passes():
    ok, n, checks = m.popper_self_test()
    assert ok is True
    assert n == 14
    for c in checks:
        assert c["ok"] is True, f"failed: {c}"


def test_popper_self_test_returns_14_checks():
    ok, n, checks = m.popper_self_test()
    assert len(checks) == 14


# ============================================================================
# chain_delegate
# ============================================================================


def test_chain_delegate_v1427_true():
    result = m.chain_delegate()
    assert result["v1427"] is True


def test_chain_delegate_all_ok_true():
    result = m.chain_delegate()
    assert result["all_ok"] is True


def test_chain_delegate_includes_upstream_modules():
    result = m.chain_delegate()
    for v in ("V1411", "V1418", "V1424", "V1425", "V1426"):
        assert v in result


# ============================================================================
# module_meta
# ============================================================================


def test_module_meta_returns_dict():
    meta = m.module_meta()
    assert isinstance(meta, dict)
    assert meta["version"] == "0.1.0"
    assert meta["n_upstream_modules"] == 16


# ============================================================================
# run_cli
# ============================================================================


def test_run_cli_version():
    rc = m.run_cli(["version"])
    assert rc == 0


def test_run_cli_meta():
    rc = m.run_cli(["meta"])
    assert rc == 0


def test_run_cli_meta_json():
    rc = m.run_cli(["meta", "--json", "true"])
    assert rc == 0


def test_run_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_run_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_run_cli_popper():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_run_cli_chain():
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_run_cli_summary():
    rc = m.run_cli(["summary"])
    assert rc == 0


def test_run_cli_collect():
    rc = m.run_cli(["collect"])
    assert rc == 0


def test_run_cli_report():
    rc = m.run_cli(["report"])
    assert rc == 0


def test_run_cli_unknown_command():
    rc = m.run_cli(["bogus"])
    assert rc != 0


def test_run_cli_empty_argv_defaults_to_help():
    rc = m.run_cli([])
    assert rc == 0