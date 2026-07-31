"""V1160 tests — ASI rubric_open V0.6 real measurement (5 sub-dim)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth import v1160_asi_rubric_open_v06_real_measure as v1160


class TestV1160ModuleAPI:
    def test_v1160_version_is_semver(self):
        parts = v1160.V1160_VERSION.split(".")
        assert len(parts) == 3

    def test_v1160_subdim_count_is_five(self):
        assert len(v1160.V1160_SUBDIM_NAMES) == 5

    def test_v1160_subdim_names_locked(self):
        locked = (
            "evaluate_week_real",
            "halting_signals_real",
            "dashboard_render_real",
            "v3_guards_real",
            "track_decision_real",
        )
        assert v1160.V1160_SUBDIM_NAMES == locked


class TestV1160SubDims:
    def test_r1_evaluate_week_returns_in_range(self):
        s, ev = v1160._measure_evaluate_week_real()
        assert 0.0 <= s <= 1.0

    def test_r2_halting_signals_returns_in_range(self):
        s, ev = v1160._measure_halting_signals_real()
        assert 0.0 <= s <= 1.0

    def test_r3_dashboard_render_returns_in_range(self):
        s, ev = v1160._measure_dashboard_render_real()
        assert 0.0 <= s <= 1.0

    def test_r4_v3_guards_returns_in_range(self):
        s, ev = v1160._measure_v3_guards_real()
        assert 0.0 <= s <= 1.0

    def test_r5_track_decision_returns_in_range(self):
        s, ev = v1160._measure_track_decision_real()
        assert 0.0 <= s <= 1.0


class TestV1160MeasureEntry:
    def test_measure_v06_returns_float(self):
        score = v1160.measure_rubric_open_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_total_equals_mean(self):
        rep = v1160.measure_rubric_open_v06_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written(self, tmp_path):
        rep = v1160.measure_rubric_open_v06_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert Path(rep.artifact_path).exists()


class TestV1160V1144Integration:
    def test_v1144_calls_v1160(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_rubric_open()
        assert score >= v1160.V1144_BASELINE_RUBRIC_OPEN


class TestV1160PhilosophyGuard:
    def test_subdims_no_phenomenology(self):
        for name in v1160.V1160_SUBDIM_NAMES:
            for forbidden in ["consciousness", "phenomenal", "qualia"]:
                assert forbidden not in name.lower()
