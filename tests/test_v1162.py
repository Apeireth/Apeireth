"""test_v1162 — V1162 ASI world_model V0.6 真测模块 tests.

主 17:43 实事求是 + 主 23:44 干到底 + 主 00:44 质量工程化.
真测 5 sub-dim: latent_quality / transition_accuracy / imagination_rollout /
              reward_prediction / jepa_predictive.

Usage:
    pytest tests/test_v1162.py -v
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import pytest

# 确保 apeireth 模块可导入
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

from apeireth.v1162_asi_world_model_v06_real_measure import (  # noqa: E402
    DEFAULT_ARTIFACT_DIR,
    TARGET_WORLD_MODEL_V06,
    V1144_BASELINE_WORLD_MODEL,
    V1162_DIM_VERSION,
    V1162_SUBDIM_NAMES,
    V1162_VERSION,
    SubDimEvidence,
    WorldModelReport,
    _call_safely,
    _loss_to_score,
    _measure_imagination_rollout,
    _measure_jepa_predictive,
    _measure_latent_quality,
    _measure_reward_prediction,
    _measure_transition_accuracy,
    _safe_import,
    _v1062_pipeline,
    measure_world_model_full,
    measure_world_model_v06,
    render_report_md,
)


# ---------------------------------------------------------------------------
# Constants & locks
# ---------------------------------------------------------------------------


class TestV1162Constants:
    def test_version_present(self):
        assert isinstance(V1162_VERSION, str) and len(V1162_VERSION) > 0

    def test_dim_version(self):
        assert V1162_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        # 主 19:33 走在前人经验上 — LOCKED 5 axis
        assert len(V1162_SUBDIM_NAMES) == 5
        assert tuple(V1162_SUBDIM_NAMES) == (
            "latent_quality",
            "transition_accuracy",
            "imagination_rollout",
            "reward_prediction",
            "jepa_predictive",
        )

    def test_baseline_and_target(self):
        # V1144 baseline 是 world_model V0.3 = 0.0000, target 是 0.75
        assert V1144_BASELINE_WORLD_MODEL == 0.0000
        assert TARGET_WORLD_MODEL_V06 == 0.7500

    def test_artifact_dir_default(self):
        assert DEFAULT_ARTIFACT_DIR == "artifacts"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


class TestV1162Helpers:
    def test_safe_import_returns_none_on_missing(self):
        m = _safe_import("apeireth.this_does_not_exist_anywhere_xyz")
        assert m is None

    def test_safe_import_returns_module_on_present(self):
        m = _safe_import("apeireth.v1162_asi_world_model_v06_real_measure")
        assert m is not None

    def test_call_safely_with_none(self):
        ok, val = _call_safely(None, default="x")
        assert ok is False
        assert val == "x"

    def test_call_safely_with_raising(self):
        def boom():
            raise RuntimeError("nope")

        ok, val = _call_safely(boom, default="d")
        assert ok is False
        assert val == "d"

    def test_loss_to_score_strict(self):
        # metric ≤ strict → 1.0
        assert _loss_to_score(0.1, loose=1.0, strict=0.5) == 1.0
        # metric ≥ loose → 0.0
        assert _loss_to_score(2.0, loose=1.0, strict=0.5) == 0.0
        # 在中间 → 线性
        s = _loss_to_score(0.75, loose=1.0, strict=0.5)
        assert math.isclose(s, 0.5, abs_tol=1e-6)
        # None → 0
        assert _loss_to_score(None, loose=1.0, strict=0.5) == 0.0

    def test_v1062_pipeline_callable_or_none(self):
        # 可能可用 (有依赖) 或不可用 (无依赖) — 但调用不抛
        ok, pipe = _v1062_pipeline()
        assert isinstance(ok, bool)
        # 如果 ok 则 pipe 不是 None
        if ok:
            assert pipe is not None


# ---------------------------------------------------------------------------
# SubDimEvidence + WorldModelReport dataclasses
# ---------------------------------------------------------------------------


class TestV1162Dataclasses:
    def test_subdim_evidence_default(self):
        ev = SubDimEvidence(name="latent_quality", score=0.42)
        assert ev.name == "latent_quality"
        assert ev.score == 0.42
        assert ev.checks == {}
        assert ev.notes == []
        assert ev.raw == {}
        d = ev.to_dict()
        assert d["name"] == "latent_quality"
        assert d["score"] == 0.42

    def test_world_model_report_defaults(self):
        rep = WorldModelReport()
        assert rep.version == V1162_VERSION
        assert rep.dim_version == V1162_DIM_VERSION
        assert rep.total == 0.0
        assert rep.sub_dim_scores == {}
        assert rep.sub_dim_evidence == {}
        assert rep.n_subdims_total == 5
        assert rep.n_subdims_passed == 0
        assert rep.n_subdims_partial == 0
        assert rep.n_subdims_missing == 0
        assert rep.v1144_baseline == V1144_BASELINE_WORLD_MODEL
        assert rep.target == TARGET_WORLD_MODEL_V06

    def test_world_model_report_to_dict_roundtrip(self):
        rep = WorldModelReport(total=0.5)
        rep.sub_dim_scores["latent_quality"] = 0.5
        rep.sub_dim_evidence["latent_quality"] = SubDimEvidence(
            name="latent_quality",
            score=0.5,
            checks={"a": True, "b": False},
            notes=["n1"],
            raw={"x": 1},
        )
        d = rep.to_dict()
        assert d["total"] == 0.5
        assert "latent_quality" in d["sub_dim_evidence"]
        rt = WorldModelReport.from_dict(d)
        assert rt.total == 0.5
        assert rt.sub_dim_scores["latent_quality"] == 0.5
        assert rt.sub_dim_evidence["latent_quality"].checks == {"a": True, "b": False}

    def test_world_model_report_summary_line(self):
        rep = WorldModelReport(total=0.6)
        rep.sub_dim_scores["latent_quality"] = 1.0
        rep.sub_dim_scores["transition_accuracy"] = 0.6
        rep.sub_dim_scores["imagination_rollout"] = 0.5
        rep.sub_dim_scores["reward_prediction"] = 0.5
        rep.sub_dim_scores["jepa_predictive"] = 0.4
        rep.n_subdims_passed = 1
        rep.n_subdims_partial = 4
        s = rep.summary_line()
        assert "V1162 world_model V0.6: total=0.6000" in s
        assert "snapshot=" in s


# ---------------------------------------------------------------------------
# 5 sub-dim 真测 (主 17:43 实事求是 — 不假装, 跑不通也不爆 0)
# ---------------------------------------------------------------------------


class TestV1162SubDims:
    """每个 sub-dim 必须返回 (float, SubDimEvidence) 且 score ∈ [0, 1]."""

    def test_latent_quality_returns_evidence(self):
        s, ev = _measure_latent_quality()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0
        assert isinstance(ev, SubDimEvidence)
        assert ev.name == "latent_quality"

    def test_transition_accuracy_returns_evidence(self):
        s, ev = _measure_transition_accuracy()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0
        assert ev.name == "transition_accuracy"

    def test_imagination_rollout_returns_evidence(self):
        s, ev = _measure_imagination_rollout()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0
        assert ev.name == "imagination_rollout"

    def test_reward_prediction_returns_evidence(self):
        s, ev = _measure_reward_prediction()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0
        assert ev.name == "reward_prediction"

    def test_jepa_predictive_returns_evidence(self):
        s, ev = _measure_jepa_predictive()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0
        assert ev.name == "jepa_predictive"


# ---------------------------------------------------------------------------
# 主入口
# ---------------------------------------------------------------------------


class TestV1162Main:
    def test_measure_world_model_v06_returns_float(self):
        s = measure_world_model_v06()
        assert isinstance(s, float)
        assert 0.0 <= s <= 1.0

    def test_measure_world_model_full_no_write(self):
        rep = measure_world_model_full(write_artifact=False)
        assert isinstance(rep, WorldModelReport)
        # 必须 5 sub-dim
        assert len(rep.sub_dim_scores) == 5
        assert len(rep.sub_dim_evidence) == 5
        for name in V1162_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert name in rep.sub_dim_evidence
        # total = mean
        expected_total = sum(rep.sub_dim_scores.values()) / 5.0
        assert math.isclose(rep.total, expected_total, abs_tol=1e-6)
        # 不假装 — n_pass/partial/missing 加起来 = 5
        assert (
            rep.n_subdims_passed
            + rep.n_subdims_partial
            + rep.n_subdims_missing
            == 5
        )

    def test_measure_world_model_full_writes_artifact(self, tmp_path):
        ad = str(tmp_path / "artifacts_v1162")
        rep = measure_world_model_full(write_artifact=True, artifact_dir=ad)
        assert rep.artifact_path != ""
        p = Path(rep.artifact_path)
        assert p.exists()
        # JSON 必须可解析
        data = json.loads(p.read_text(encoding="utf-8"))
        assert data["version"] == V1162_VERSION
        assert data["dim_version"] == V1162_DIM_VERSION
        assert "total" in data
        assert "sub_dim_scores" in data
        for name in V1162_SUBDIM_NAMES:
            assert name in data["sub_dim_scores"]

    def test_measure_world_model_full_target_visible(self):
        rep = measure_world_model_full(write_artifact=False)
        assert rep.target == TARGET_WORLD_MODEL_V06
        assert rep.v1144_baseline == V1144_BASELINE_WORLD_MODEL


# ---------------------------------------------------------------------------
# Markdown 报告渲染
# ---------------------------------------------------------------------------


class TestV1162ReportRendering:
    def test_render_report_md_basic(self):
        rep = WorldModelReport(total=0.42)
        for i, name in enumerate(V1162_SUBDIM_NAMES):
            rep.sub_dim_scores[name] = 0.5 - i * 0.05
            rep.sub_dim_evidence[name] = SubDimEvidence(
                name=name, score=0.5 - i * 0.05, checks={"a": i % 2 == 0}, notes=[f"n{i}"]
            )
        rep.n_subdims_passed = 0
        rep.n_subdims_partial = 5
        md = render_report_md(rep)
        assert "# V1162 world_model V0.6 真补报告" in md
        assert "latent_quality" in md
        assert "transition_accuracy" in md
        assert "imagination_rollout" in md
        assert "reward_prediction" in md
        assert "jepa_predictive" in md
        assert "0.4200" in md
        assert f"_Generated by V1162 {V1162_VERSION}_" in md

    def test_render_report_md_with_all_missing(self):
        rep = WorldModelReport(total=0.0)
        # n_subdims 不加 → 全 missing
        md = render_report_md(rep)
        assert "missing" in md


# ---------------------------------------------------------------------------
# V3 哲学守门 (主 17:58 + 20:46 不假装)
# ---------------------------------------------------------------------------


class TestV1162PhilosophyGuards:
    """V1162 必须不假装 — 5 sub-dim 是工程测量, 不冒充 ASI 真正世界模型."""

    def test_not_pretending_run_failure_yields_zero(self):
        # W1 若 V1062 pipeline 不可用 → score 应为 0.0
        s, ev = _measure_latent_quality()
        if "pipeline_unavailable" in str(ev.raw.get("reason", "")):
            assert s == 0.0

    def test_total_bounded_in_unit_interval(self):
        rep = measure_world_model_full(write_artifact=False)
        assert 0.0 <= rep.total <= 1.0

    def test_sub_dim_count_exactly_five(self):
        rep = measure_world_model_full(write_artifact=False)
        assert len(rep.sub_dim_scores) == 5
        assert rep.n_subdims_total == 5
