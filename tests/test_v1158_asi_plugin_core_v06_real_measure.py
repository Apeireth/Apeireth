"""V1158 tests — ASI plugin_core V0.6 real measurement (5 sub-dim).

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 22:33 ASI 北极星.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth import v1158_asi_plugin_core_v06_real_measure as v1158


class TestV1158ModuleAPI:
    def test_v1158_version_is_semver(self):
        parts = v1158.V1158_VERSION.split(".")
        assert len(parts) == 3
        for p in parts:
            assert p.isdigit()

    def test_v1158_dim_version_is_v06(self):
        assert v1158.V1158_DIM_VERSION == "0.6"

    def test_v1158_subdim_names_count_is_five(self):
        assert len(v1158.V1158_SUBDIM_NAMES) == 5

    def test_v1158_subdim_names_locked(self):
        locked = (
            "plugin_discovery",
            "plugin_parse_rate",
            "plugin_validation_coverage",
            "plugin_capability_summary",
            "plugin_protocol_diversity",
        )
        assert v1158.V1158_SUBDIM_NAMES == locked

    def test_v1158_v1144_baseline_recorded(self):
        assert v1158.V1144_BASELINE_PLUGIN_CORE == 0.65


class TestV1158ReportDataclass:
    def test_subdim_evidence_to_dict(self):
        ev = v1158.SubDimEvidence(name="test", score=0.5)
        d = ev.to_dict()
        assert d["name"] == "test"

    def test_report_default_snapshot_id_prefix(self):
        rep = v1158.PluginCoreReport()
        assert rep.snapshot_id.startswith("v1158-")

    def test_report_from_dict_roundtrip(self):
        rep = v1158.PluginCoreReport(total=0.88)
        d = rep.to_dict()
        new = v1158.PluginCoreReport.from_dict(d)
        assert new.total == rep.total
        assert new.snapshot_id == rep.snapshot_id


class TestV1158SubDims:
    def test_p1_plugin_discovery_returns_in_range(self):
        score, ev = v1158._measure_plugin_discovery()
        assert 0.0 <= score <= 1.0
        assert ev.name == "plugin_discovery"

    def test_p2_plugin_parse_rate_returns_in_range(self):
        score, ev = v1158._measure_plugin_parse_rate()
        assert 0.0 <= score <= 1.0
        assert ev.name == "plugin_parse_rate"

    def test_p3_plugin_validation_returns_in_range(self):
        score, ev = v1158._measure_plugin_validation_coverage()
        assert 0.0 <= score <= 1.0
        assert ev.name == "plugin_validation_coverage"

    def test_p4_plugin_capability_summary_returns_in_range(self):
        score, ev = v1158._measure_plugin_capability_summary()
        assert 0.0 <= score <= 1.0
        assert ev.name == "plugin_capability_summary"

    def test_p5_plugin_protocol_diversity_returns_in_range(self):
        score, ev = v1158._measure_plugin_protocol_diversity()
        assert 0.0 <= score <= 1.0
        assert ev.name == "plugin_protocol_diversity"


class TestV1158MeasureEntry:
    def test_measure_v06_returns_float(self):
        score = v1158.measure_plugin_core_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_full_returns_report(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        assert isinstance(rep.total, float)
        assert 0.0 <= rep.total <= 1.0
        assert len(rep.sub_dim_scores) == 5

    def test_measure_total_equals_mean(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written(self, tmp_path):
        rep = v1158.measure_plugin_core_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert rep.artifact_path
        assert Path(rep.artifact_path).exists()


class TestV1158V1144Integration:
    def test_v1144_calls_v1158_first(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_plugin_core()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_v1158_lifts_v1144_baseline(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_plugin_core()
        assert score >= v1158.V1144_BASELINE_PLUGIN_CORE


class TestV1158Serialization:
    def test_report_json_serializable(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        data = rep.to_dict()
        json.dumps(data, ensure_ascii=False)

    def test_report_roundtrip_dict(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        data = rep.to_dict()
        new = v1158.PluginCoreReport.from_dict(data)
        assert new.total == rep.total


class TestV1158PhilosophyGuard:
    def test_v1158_subdims_are_engineering_not_phenomenology(self):
        for name in v1158.V1158_SUBDIM_NAMES:
            for forbidden in ["consciousness", "phenomenal", "qualia", "sentience", "free_will"]:
                assert forbidden not in name.lower()

    def test_full_total_not_pretending_asi(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        assert rep.total <= 1.0


class TestV1158Markdown:
    def test_render_report_md_contains_total(self):
        rep = v1158.measure_plugin_core_full(write_artifact=False)
        md = v1158.render_report_md(rep)
        assert "V1158 plugin_core V0.6" in md
        assert f"{rep.total:.4f}" in md
