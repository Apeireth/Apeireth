"""V1159 tests — ASI engineering V0.6 real measurement (5 sub-dim)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth import v1159_asi_engineering_v06_real_measure as v1159


class TestV1159ModuleAPI:
    def test_v1159_version_is_semver(self):
        parts = v1159.V1159_VERSION.split(".")
        assert len(parts) == 3

    def test_v1159_dim_version_is_v06(self):
        assert v1159.V1159_DIM_VERSION == "0.6"

    def test_v1159_subdim_count_is_five(self):
        assert len(v1159.V1159_SUBDIM_NAMES) == 5

    def test_v1159_subdim_names_locked(self):
        locked = (
            "test_coverage_real",
            "capability_density_real",
            "module_organization",
            "code_total_real",
            "score_engineering_real",
        )
        assert v1159.V1159_SUBDIM_NAMES == locked


class TestV1159SubDims:
    def test_e1_test_coverage_returns_in_range(self):
        score, ev = v1159._measure_test_coverage_real()
        assert 0.0 <= score <= 1.0

    def test_e2_capability_density_returns_in_range(self):
        score, ev = v1159._measure_capability_density_real()
        assert 0.0 <= score <= 1.0

    def test_e3_module_organization_returns_in_range(self):
        score, ev = v1159._measure_module_organization()
        assert 0.0 <= score <= 1.0

    def test_e4_code_total_returns_in_range(self):
        score, ev = v1159._measure_code_total_real()
        assert 0.0 <= score <= 1.0

    def test_e5_score_engineering_returns_in_range(self):
        score, ev = v1159._measure_score_engineering_real()
        assert 0.0 <= score <= 1.0


class TestV1159MeasureEntry:
    def test_measure_v06_returns_float(self):
        score = v1159.measure_engineering_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_total_equals_mean(self):
        rep = v1159.measure_engineering_v06_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written(self, tmp_path):
        rep = v1159.measure_engineering_v06_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert Path(rep.artifact_path).exists()


class TestV1159V1144Integration:
    def test_v1144_calls_v1159(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_engineering()
        assert score >= v1159.V1144_BASELINE_ENGINEERING


class TestV1159PhilosophyGuard:
    def test_subdims_no_phenomenology(self):
        for name in v1159.V1159_SUBDIM_NAMES:
            for forbidden in ["consciousness", "phenomenal", "qualia"]:
                assert forbidden not in name.lower()
