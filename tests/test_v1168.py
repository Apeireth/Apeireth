"""Test V1168 — ASI philosophy_5gaps V0.6 follow-up (5 sub-dim 真补).

主 17:43 实事求是: 测试覆盖 constants / dataclasses / helpers / _measure_*
with monkeypatched underlying modules (V1151/V1053/V1054/V1056) (不实际真 instantiate).
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1168Constants:
    def test_version_present(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import V1168_VERSION
        assert V1168_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import V1168_DIM_VERSION
        assert V1168_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import V1168_SUBDIM_NAMES
        assert V1168_SUBDIM_NAMES == (
            "time_philo_real",
            "volition_real",
            "self_recognition_real",
            "emergence_real",
            "truth_value_real",
        )

    def test_baseline_target_constants(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import (
            V1155_BASELINE_PHILOSOPHY_5GAPS,
            TARGET_PHILOSOPHY_5GAPS_V06,
        )
        assert V1155_BASELINE_PHILOSOPHY_5GAPS == 0.0
        assert TARGET_PHILOSOPHY_5GAPS_V06 == 0.7

    def test_artifact_dir_default(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"

    def test_per_p_baselines(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import (
            P1_TIME_BASELINE, P2_VOLITION_BASELINE, P3_SELF_REC_BASELINE,
            P4_EMERGENCE_BASELINE, P5_TRUTH_BASELINE,
        )
        assert 0.0 < P1_TIME_BASELINE < 1.0
        assert 0.0 < P2_VOLITION_BASELINE < 1.0
        assert 0.0 < P3_SELF_REC_BASELINE < 1.0
        assert 0.0 < P4_EMERGENCE_BASELINE < 1.0
        assert 0.0 < P5_TRUTH_BASELINE < 1.0


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _safe_import
        assert _safe_import("nonexistent.module.xyz") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _safe_import
        mod = _safe_import("apeireth.v1168_asi_philosophy_5gaps_v06_real_measure")
        assert mod is not None

    def test_call_safely_none(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _call_safely
        ok, r = _call_safely(None, 1, 2)
        assert ok is False
        assert r is None

    def test_call_safely_callable(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _call_safely
        ok, r = _call_safely(lambda x: x + 10, 5)
        assert ok is True
        assert r == 15

    def test_call_safely_raising(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _call_safely
        def boom():
            raise RuntimeError("x")
        ok, r = _call_safely(boom)
        assert ok is False
        assert r is None

    def test_attr_first(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _attr_first
        class O:
            a = 1
        assert _attr_first(O, ["nope", "a"]) == 1
        assert _attr_first(object, ["x"]) is None

    def test_inspect_signature_safe(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import inspect_signature_safe
        def f(a, b, c=1):
            pass
        params = inspect_signature_safe(f)
        assert params == ["a", "b", "c"]

    def test_inspect_signature_safe_invalid(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import inspect_signature_safe
        # lambda has different signature behavior — just ensure no throw
        params = inspect_signature_safe(lambda x: x)
        assert isinstance(params, list)


class TestSubDimEvidence:
    def test_default_init(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5)
        assert e.name == "x"
        assert e.score == 0.5

    def test_to_dict(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5, checks={"a": True}, notes=["n"], raw={"k": 1})
        d = e.to_dict()
        assert d["name"] == "x"
        assert d["checks"] == {"a": True}


class TestPhilosophy5GapsReport:
    def test_default_init(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import Philosophy5GapsReport
        r = Philosophy5GapsReport()
        assert r.snapshot_id.startswith("v1168-")
        assert r.version == "0.1.0"
        assert r.dim_version == "0.6"
        assert r.total == 0.0
        assert r.n_subdims_total == 5

    def test_summary_line(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import Philosophy5GapsReport
        r = Philosophy5GapsReport(
            total=0.7, n_subdims_passed=3, n_subdims_partial=2,
            v1154_score=1.0, v1053_score=0.6, v1054_score=0.8,
            v1056_score=0.5, v1051_score=0.5,
        )
        line = r.summary_line()
        assert "total=0.7000" in line
        assert "v1154=1.0000" in line
        assert "v1053=0.6000" in line
        assert "3 pass / 2 partial / 0 missing" in line

    def test_to_from_dict_roundtrip(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import (
            Philosophy5GapsReport, SubDimEvidence,
        )
        r = Philosophy5GapsReport(
            total=0.6, snapshot_id="v1168-test",
            v1154_score=0.8, v1053_score=0.5, v1054_score=0.7,
            v1056_score=0.6, v1051_score=0.4,
        )
        r.sub_dim_scores = {"time_philo_real": 0.8}
        r.sub_dim_evidence["time_philo_real"] = SubDimEvidence(
            name="time_philo_real", score=0.8, checks={"k": True},
        )
        r2 = Philosophy5GapsReport.from_dict(r.to_dict())
        assert r2.snapshot_id == "v1168-test"
        assert r2.total == 0.6
        assert r2.sub_dim_scores["time_philo_real"] == 0.8
        assert r2.sub_dim_evidence["time_philo_real"].score == 0.8
        assert r2.v1154_score == 0.8

    def test_from_dict_handles_missing_evidence(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import Philosophy5GapsReport
        r = Philosophy5GapsReport.from_dict({"snapshot_id": "x", "total": 0.3})
        assert r.snapshot_id == "x"
        assert r.sub_dim_evidence == {}


class TestMeasureTimePhilo:
    """P1 — time_philo_real."""

    def test_returns_valid_score(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _measure_time_philo
        score, ev = _measure_time_philo()
        assert 0.0 <= score <= 1.0
        assert ev.name == "time_philo_real"

    def test_v1154_module_unavailable(self, monkeypatch):
        from apeireth import v1168_asi_philosophy_5gaps_v06_real_measure as mod
        monkeypatch.setattr(mod, "_safe_import", lambda name: None)
        score, ev = mod._measure_time_philo()
        assert score == 0.0
        assert any("not importable" in n for n in ev.notes)


class TestMeasureVolition:
    """P2 — volition_real."""

    def test_returns_valid_score(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _measure_volition
        score, ev = _measure_volition()
        assert 0.0 <= score <= 1.0
        assert ev.name == "volition_real"

    def test_v1053_module_unavailable(self, monkeypatch):
        from apeireth import v1168_asi_philosophy_5gaps_v06_real_measure as mod
        monkeypatch.setattr(mod, "_safe_import", lambda name: None)
        score, ev = mod._measure_volition()
        assert score == 0.0
        assert any("not importable" in n for n in ev.notes)


class TestMeasureSelfRecognition:
    """P3 — self_recognition_real."""

    def test_returns_valid_score(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _measure_self_recognition
        score, ev = _measure_self_recognition()
        assert 0.0 <= score <= 1.0
        assert ev.name == "self_recognition_real"

    def test_v1054_module_unavailable(self, monkeypatch):
        from apeireth import v1168_asi_philosophy_5gaps_v06_real_measure as mod
        monkeypatch.setattr(mod, "_safe_import", lambda name: None)
        score, ev = mod._measure_self_recognition()
        assert score == 0.0


class TestMeasureEmergence:
    """P4 — emergence_real."""

    def test_returns_valid_score(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _measure_emergence
        score, ev = _measure_emergence()
        assert 0.0 <= score <= 1.0
        assert ev.name == "emergence_real"

    def test_v1056_module_unavailable(self, monkeypatch):
        from apeireth import v1168_asi_philosophy_5gaps_v06_real_measure as mod
        monkeypatch.setattr(mod, "_safe_import", lambda name: None)
        score, ev = mod._measure_emergence()
        assert score == 0.0


class TestMeasureTruthValue:
    """P5 — truth_value_real."""

    def test_returns_valid_score(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _measure_truth_value
        score, ev = _measure_truth_value()
        assert 0.0 <= score <= 1.0
        assert ev.name == "truth_value_real"

    def test_v1051_module_unavailable(self, monkeypatch):
        from apeireth import v1168_asi_philosophy_5gaps_v06_real_measure as mod
        monkeypatch.setattr(mod, "_safe_import", lambda name: None)
        score, ev = mod._measure_truth_value()
        assert score == 0.0


class TestMeasureFullAggregation:
    """主入口聚合."""

    def test_aggregate_5_subdim(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import (
            measure_philosophy_5gaps_v06_full, V1168_SUBDIM_NAMES,
        )
        rep = measure_philosophy_5gaps_v06_full(write_artifact=False)
        assert 0.0 <= rep.total <= 1.0
        assert len(rep.sub_dim_scores) == 5
        assert all(name in rep.sub_dim_scores for name in V1168_SUBDIM_NAMES)

    def test_artifact_written(self, tmp_path, monkeypatch):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import (
            measure_philosophy_5gaps_v06_full,
        )
        rep = measure_philosophy_5gaps_v06_full(write_artifact=True, artifact_dir=str(tmp_path))
        assert rep.artifact_path
        artifact = Path(rep.artifact_path)
        assert artifact.exists()
        assert artifact.parent == tmp_path
        import json
        data = json.loads(artifact.read_text(encoding="utf-8"))
        assert data["snapshot_id"] == rep.snapshot_id
        assert data["version"] == "0.1.0"

    def test_main_entry_returns_float(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import measure_philosophy_5gaps_v06
        score = measure_philosophy_5gaps_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0


class TestCLI:
    def test_no_write_argv(self):
        from apeireth.v1168_asi_philosophy_5gaps_v06_real_measure import _cli
        # Just test that it returns int 0 (success)
        # Don't actually capture stdout to keep test simple
        result = _cli(["--no-write"])
        assert result == 0
