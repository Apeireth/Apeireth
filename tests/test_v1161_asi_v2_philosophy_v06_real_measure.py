"""V1161 tests — ASI v2_philosophy V0.6 real measurement (5 sub-dim)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth import v1161_asi_v2_philosophy_v06_real_measure as v1161


class TestV1161ModuleAPI:
    def test_v1161_version_is_semver(self):
        parts = v1161.V1161_VERSION.split(".")
        assert len(parts) == 3

    def test_v1161_subdim_count_is_five(self):
        assert len(v1161.V1161_SUBDIM_NAMES) == 5

    def test_v1161_subdim_names_locked(self):
        locked = (
            "V1135_answers_real",
            "V1137_remaining_real",
            "PHILOSOPHY_9_KEYS_real",
            "ASI_7_QUESTIONS_real",
            "v3_guards_real",
        )
        assert v1161.V1161_SUBDIM_NAMES == locked


class TestV1161SubDims:
    def test_v1_v1135_answers_returns_in_range(self):
        s, ev = v1161._measure_V1135_answers_real()
        assert 0.0 <= s <= 1.0

    def test_v2_v1137_remaining_returns_in_range(self):
        s, ev = v1161._measure_V1137_remaining_real()
        assert 0.0 <= s <= 1.0

    def test_v3_philosophy_9_keys_returns_in_range(self):
        s, ev = v1161._measure_PHILOSOPHY_9_KEYS_real()
        assert 0.0 <= s <= 1.0

    def test_v4_asi_7_questions_returns_in_range(self):
        s, ev = v1161._measure_ASI_7_QUESTIONS_real()
        assert 0.0 <= s <= 1.0

    def test_v5_v3_guards_returns_in_range(self):
        s, ev = v1161._measure_v3_guards_real()
        assert 0.0 <= s <= 1.0


class TestV1161MeasureEntry:
    def test_measure_v06_returns_float(self):
        score = v1161.measure_v2_philosophy_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_total_equals_mean(self):
        rep = v1161.measure_v2_philosophy_v06_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written(self, tmp_path):
        rep = v1161.measure_v2_philosophy_v06_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert Path(rep.artifact_path).exists()


class TestV1161V1144Integration:
    def test_v1144_calls_v1161(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_v2_philosophy()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0


class TestV1161PhilosophyGuard:
    def test_subdims_no_phenomenology(self):
        for name in v1161.V1161_SUBDIM_NAMES:
            for forbidden in ["consciousness", "phenomenal", "qualia"]:
                assert forbidden not in name.lower()
