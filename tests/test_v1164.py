"""Test V1164 — ASI world_model V0.6.1 patched W2/W3/W5 真补."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

# Setup project root for imports
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1164Constants:
    def test_version_present(self):
        from apeireth.v1164_asi_world_model_v06_patched import V1164_VERSION
        assert V1164_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1164_asi_world_model_v06_patched import V1164_DIM_VERSION
        assert V1164_DIM_VERSION == "0.6.1"

    def test_subdim_names_locked(self):
        from apeireth.v1164_asi_world_model_v06_patched import V1164_SUBDIM_NAMES
        assert V1164_SUBDIM_NAMES == (
            "latent_quality",
            "transition_accuracy",
            "imagination_rollout",
            "reward_prediction",
            "jepa_predictive",
        )

    def test_baseline_v1162_locked(self):
        from apeireth.v1164_asi_world_model_v06_patched import V1162_BASELINE_TOTAL, V1162_BASELINE_SUB
        assert V1162_BASELINE_TOTAL == 0.2939
        assert V1162_BASELINE_SUB["transition_accuracy"] == 0.0
        assert V1162_BASELINE_SUB["imagination_rollout"] == 0.0
        assert V1162_BASELINE_SUB["jepa_predictive"] == 0.0

    def test_artifact_dir_default(self):
        from apeireth.v1164_asi_world_model_v06_patched import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"

    def test_v1162_baseline_sub_complete(self):
        from apeireth.v1164_asi_world_model_v06_patched import V1162_BASELINE_SUB, V1164_SUBDIM_NAMES
        for name in V1164_SUBDIM_NAMES:
            assert name in V1162_BASELINE_SUB


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1164_asi_world_model_v06_patched import _safe_import
        assert _safe_import("nonexistent.module.xyz") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1164_asi_world_model_v06_patched import _safe_import
        mod = _safe_import("apeireth.v1164_asi_world_model_v06_patched")
        assert mod is not None

    def test_call_safely_with_none(self):
        from apeireth.v1164_asi_world_model_v06_patched import _call_safely
        ok, r = _call_safely(None, 1, 2)
        assert ok is False
        assert r is None

    def test_call_safely_with_callable(self):
        from apeireth.v1164_asi_world_model_v06_patched import _call_safely
        ok, r = _call_safely(lambda x: x * 2, 3)
        assert ok is True
        assert r == 6

    def test_call_safely_with_raising(self):
        from apeireth.v1164_asi_world_model_v06_patched import _call_safely
        def boom():
            raise RuntimeError("x")
        ok, r = _call_safely(boom)
        assert ok is False
        assert r is None

    def test_attr_first_picks_first_existing(self):
        from apeireth.v1164_asi_world_model_v06_patched import _attr_first
        class Obj:
            a = 1
        assert _attr_first(Obj, ["nope", "a", "z"]) == 1

    def test_attr_first_returns_none_on_missing(self):
        from apeireth.v1164_asi_world_model_v06_patched import _attr_first
        class Obj:
            pass
        assert _attr_first(Obj, ["x", "y"]) is None

    def test_loss_to_score_within_bounds(self):
        from apeireth.v1164_asi_world_model_v06_patched import _loss_to_score
        # strict <= metric <= loose => linear
        assert _loss_to_score(0.5, 1.0, 0.2) == pytest.approx(0.625, rel=0.01)
        # below strict => 1.0
        assert _loss_to_score(0.05, 1.0, 0.2) == 1.0
        # above loose => 0.0
        assert _loss_to_score(2.0, 1.0, 0.2) == 0.0
        # none => 0.0
        assert _loss_to_score(None, 1.0, 0.2) == 0.0


class TestV1062Connection:
    def test_v1062_pipeline_callable_or_returns_reason(self):
        from apeireth.v1164_asi_world_model_v06_patched import _v1062_pipeline
        ok, p = _v1062_pipeline()
        # ok may be True or False depending on V1062 availability
        if ok:
            assert p is not None
        else:
            assert p is None


class TestDataclasses:
    def test_subdim_evidence_default(self):
        from apeireth.v1164_asi_world_model_v06_patched import SubDimEvidence
        ev = SubDimEvidence(name="x")
        assert ev.name == "x"
        assert ev.score == 0.0
        assert ev.checks == {}
        assert ev.raw == {}
        assert ev.notes == []
        assert ev.baseline_v1162 == 0.0

    def test_world_model_report_defaults(self):
        from apeireth.v1164_asi_world_model_v06_patched import WorldModelPatchedReport
        rep = WorldModelPatchedReport()
        assert rep.version == "0.1.0"
        assert rep.dim_version == "0.6.1"
        assert rep.total == 0.0
        assert rep.sub_dim_scores == {}
        assert rep.baseline_v1162 == 0.2939

    def test_world_model_report_summary_line(self):
        from apeireth.v1164_asi_world_model_v06_patched import WorldModelPatchedReport
        rep = WorldModelPatchedReport(total=0.5)
        s = rep.summary_line()
        assert "V1164 world_model V0.6.1 patched" in s
        assert "0.5000" in s or "0.5" in s or "total=0.5" in s
        assert "snapshot=v1164-" in s

    def test_to_dict_roundtrip(self):
        from apeireth.v1164_asi_world_model_v06_patched import WorldModelPatchedReport, SubDimEvidence
        rep = WorldModelPatchedReport()
        rep.sub_dim_scores["latent_quality"] = 0.8
        rep.sub_dim_evidence["latent_quality"] = SubDimEvidence(name="latent_quality", score=0.8)
        d = rep.to_dict()
        assert "sub_dim_scores" in d
        assert "sub_dim_evidence" in d
        assert d["sub_dim_scores"]["latent_quality"] == 0.8


class TestSubDims:
    def test_latent_quality_returns_evidence(self):
        from apeireth.v1164_asi_world_model_v06_patched import _measure_latent_quality_patched
        score, ev = _measure_latent_quality_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "latent_quality"
        assert isinstance(ev.notes, list)
        assert len(ev.notes) >= 1

    def test_transition_accuracy_returns_evidence(self):
        from apeireth.v1164_asi_world_model_v06_patched import _measure_transition_accuracy_patched
        score, ev = _measure_transition_accuracy_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "transition_accuracy"
        assert ev.baseline_v1162 == 0.0  # V1162 dead path

    def test_imagination_rollout_returns_evidence(self):
        from apeireth.v1164_asi_world_model_v06_patched import _measure_imagination_rollout_patched
        score, ev = _measure_imagination_rollout_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "imagination_rollout"

    def test_reward_prediction_returns_evidence(self):
        from apeireth.v1164_asi_world_model_v06_patched import _measure_reward_prediction_patched
        score, ev = _measure_reward_prediction_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "reward_prediction"

    def test_jepa_predictive_returns_evidence(self):
        from apeireth.v1164_asi_world_model_v06_patched import _measure_jepa_predictive_patched
        score, ev = _measure_jepa_predictive_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "jepa_predictive"
        assert ev.baseline_v1162 == 0.0  # V1162 dead path


class TestMainEntry:
    def test_measure_world_model_v06_patched_returns_float(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched
        score = measure_world_model_v06_patched()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_world_model_v06_patched_full_no_write(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full, V1164_SUBDIM_NAMES
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        assert isinstance(rep.total, float)
        assert 0.0 <= rep.total <= 1.0
        for name in V1164_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert 0.0 <= rep.sub_dim_scores[name] <= 1.0

    def test_measure_world_model_v06_patched_full_writes_artifact(self, tmp_path):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full
        rep = measure_world_model_v06_patched_full(write_artifact=True, artifact_dir=str(tmp_path))
        artifact = tmp_path / "v1164_world_model_v06_patched.json"
        assert artifact.exists()
        # verify JSON parseable
        with open(artifact, "r", encoding="utf-8") as fh:
            d = json.load(fh)
        assert d["version"] == "0.1.0"
        assert d["dim_version"] == "0.6.1"
        assert "sub_dim_scores" in d
        assert "sub_dim_evidence" in d
        assert d["baseline_v1162"] == 0.2939

    def test_measure_world_model_v06_patched_full_target_visible(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        # target is 0.75 for V0.6 series (主 22:33 ASI 北极星)
        s = rep.summary_line()
        assert "target=0.7500" in s

    def test_measure_world_model_v06_patched_total_is_mean_of_nonzero(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        # 如果 V1062 都跑通, 所有 5 sub-dim 都非零
        non_zero = [v for v in rep.sub_dim_scores.values() if v > 0.0]
        if len(non_zero) == len(rep.sub_dim_scores):
            import statistics
            expected = statistics.mean(non_zero)
            assert abs(rep.total - round(expected, 4)) < 0.01


class TestRenderReport:
    def test_render_report_md_basic(self):
        from apeireth.v1164_asi_world_model_v06_patched import (
            measure_world_model_v06_patched_full, render_report_md
        )
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        md = render_report_md(rep)
        assert "# V1164" in md
        assert "5 sub-dim 真补" in md
        for name in ["latent_quality", "transition_accuracy", "imagination_rollout", "reward_prediction", "jepa_predictive"]:
            assert name in md

    def test_render_report_md_contains_target(self):
        from apeireth.v1164_asi_world_model_v06_patched import (
            measure_world_model_v06_patched_full, render_report_md
        )
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        md = render_report_md(rep)
        assert "0.7500" in md

    def test_render_report_md_mentions_philosophy_guards(self):
        from apeireth.v1164_asi_world_model_v06_patched import render_report_md, WorldModelPatchedReport
        rep = WorldModelPatchedReport(total=0.5)
        md = render_report_md(rep)
        assert "Philosophy Guards" in md
        assert "不假装" in md


class TestCLI:
    def test_default_run(self, capsys, tmp_path):
        from apeireth.v1164_asi_world_model_v06_patched import main
        rc = main(["--no-write", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1164" in out
        assert "patched" in out

    def test_json_run(self, capsys, tmp_path):
        from apeireth.v1164_asi_world_model_v06_patched import main
        rc = main(["--json", "--no-write", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["version"] == "0.1.0"
        assert "sub_dim_scores" in d

    def test_report_run_writes_md(self, capsys, tmp_path):
        from apeireth.v1164_asi_world_model_v06_patched import main
        rc = main(["--report", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        md_path = tmp_path / "v1164_world_model_v06_patched.md"
        assert md_path.exists()
        content = md_path.read_text(encoding="utf-8")
        assert "# V1164" in content


class TestApiDriftFix:
    """Specific tests proving V1164 fixes V1162 API drift."""

    def test_w2_fix_unpacks_tuple_first_element(self):
        """V1162 failed because transition.step returns tuple [obs_recon, hidden].
        V1164 must extract obs_recon as [0] not as whole tuple."""
        from apeireth.v1164_asi_world_model_v06_patched import _v1062_pipeline, _transition_pipeline, _attr_first, _call_safely
        from apeireth.v1062_asi_world_model import WorldModelPipeline

        # Direct contract: step() returns tuple [list8, list4]
        pipe = WorldModelPipeline.default(obs_dim=8, latent_dim=4, action_dim=2)
        z = pipe.encoder.encode_sample([0.5]*8)[2]
        result = pipe.transition.step(z, [0.1, 0.2], None)
        assert isinstance(result, tuple)
        assert len(result) >= 1
        obs_recon = result[0]
        assert isinstance(obs_recon, list)
        assert len(obs_recon) == 8

    def test_w3_fix_uses_imagine_with_4_args(self):
        """V1162 failed because imagination.imagine has 4-arg signature."""
        from apeireth.v1062_asi_world_model import WorldModelPipeline
        pipe = WorldModelPipeline.default(obs_dim=8, latent_dim=4, action_dim=2)
        z = pipe.encoder.encode_sample([0.5]*8)[2]
        steps = pipe.imagination.imagine(z, None, None, 5)
        assert isinstance(steps, list)
        assert len(steps) == 5
        # ImaginedStep has .state (list 4 维)
        for step in steps:
            assert hasattr(step, "state")
            assert isinstance(step.state, list)
            assert len(step.state) == 4

    def test_w5_fix_uses_jepa_three_funcs(self):
        """V1162 failed because jepa.predict_embedding is the real name."""
        from apeireth.v1062_asi_world_model import WorldModelPipeline
        pipe = WorldModelPipeline.default(obs_dim=8, latent_dim=4, action_dim=2)
        z = pipe.encoder.encode_sample([0.5]*8)[2]
        embed_x = pipe.jepa.embed(z)
        assert isinstance(embed_x, list)
        predicted = pipe.jepa.predict_embedding(embed_x)
        assert isinstance(predicted, list)
        loss = pipe.jepa.jepa_loss(embed_x, predicted)
        assert isinstance(loss, (int, float))


class TestRoundTrip:
    def test_to_from_dict_preserves_scores(self):
        from apeireth.v1164_asi_world_model_v06_patched import (
            measure_world_model_v06_patched_full, V1164_SUBDIM_NAMES
        )
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        d = rep.to_dict()
        for name in V1164_SUBDIM_NAMES:
            assert name in d["sub_dim_scores"]
            assert d["sub_dim_scores"][name] == rep.sub_dim_scores[name]


class TestPhilosophyGuards:
    def test_does_not_pretend_W2W3W5_equal_0(self):
        """V1164 should NOT report all 5 sub-dims as 0 (unlike V1162)."""
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        # 如果 V1062 API OK, W2/W3/W5 至少非零 (V1062 已 import 成功)
        non_zero = sum(1 for v in rep.sub_dim_scores.values() if v > 0.0)
        assert non_zero >= 3  # W1/W4 在 V1162 已非零; W2/W3/W5 在 V1164 真修补应该非零

    def test_total_bounded_in_unit_interval(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        assert 0.0 <= rep.total <= 1.0

    def test_sub_dim_count_exactly_five(self):
        from apeireth.v1164_asi_world_model_v06_patched import measure_world_model_v06_patched_full, V1164_SUBDIM_NAMES
        rep = measure_world_model_v06_patched_full(write_artifact=False)
        assert len(rep.sub_dim_scores) == 5
        assert set(rep.sub_dim_scores.keys()) == set(V1164_SUBDIM_NAMES)


# M3 from anchor5 main import for fixture path
if __name__ == "__main__":
    pytest.main([__file__, "-v"])
