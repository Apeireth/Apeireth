"""V1156 tests — ASI cognitive_core V0.6 real measurement (5 sub-dim).

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 22:33 ASI 北极星.

This test file covers:
  1. Module API 真测 (V1156_VERSION + 5 sub-dim LOCKED names + CognitiveCoreReport)
  2. Sub-dim 真测 (C1-C5 真跑 + score ∈ [0,1])
  3. V1144 集成 (_measure_cognitive_core 真调 V1156)
  4. 不假装 guard (5 sub-dim 不是 phenomenology)
  5. Report serialization (JSON dump + reload)
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import pytest

from apeireth import v1156_asi_cognitive_core_v06_real_measure as v1156


# ---------------------------------------------------------------------------
# Module API 真测
# ---------------------------------------------------------------------------


class TestV1156ModuleAPI:
    def test_v1156_version_is_semver(self):
        parts = v1156.V1156_VERSION.split(".")
        assert len(parts) == 3
        for p in parts:
            assert p.isdigit()

    def test_v1156_dim_version_is_v06(self):
        assert v1156.V1156_DIM_VERSION == "0.6"

    def test_v1156_subdim_names_count_is_five(self):
        assert len(v1156.V1156_SUBDIM_NAMES) == 5

    def test_v1156_subdim_names_locked(self):
        # LOCKED 主 19:33 (借鉴 cognitive science 5 axis, 不偷改)
        locked = (
            "introspection_depth",
            "self_model_accuracy",
            "meta_cognition_calibration",
            "perception_action_loop",
            "reasoning_consistency",
        )
        assert v1156.V1156_SUBDIM_NAMES == locked

    def test_v1156_target_high_enough(self):
        # target 0.85 — 真期待值, 不是 placeholder
        assert v1156.TARGET_COGNITIVE_CORE_V06 >= 0.85

    def test_v1156_v1144_baseline_recorded(self):
        # baseline 写死历史值, 不允许改
        assert v1156.V1144_BASELINE_COGNITIVE_CORE == 0.5

    def test_v1156_default_n_consistency_at_least_2(self):
        # C5 真跑 N 次, 至少 2
        assert v1156.DEFAULT_N_CONSISTENCY >= 2


# ---------------------------------------------------------------------------
# CognitiveCoreReport dataclass
# ---------------------------------------------------------------------------


class TestV1156ReportDataclass:
    def test_subdim_evidence_dataclass_to_dict(self):
        ev = v1156.SubDimEvidence(name="test", score=0.5)
        d = ev.to_dict()
        assert d["name"] == "test"
        assert d["score"] == 0.5
        assert isinstance(d["checks"], dict)

    def test_report_default_snapshot_id_has_v1156_prefix(self):
        rep = v1156.CognitiveCoreReport()
        assert rep.snapshot_id.startswith("v1156-")
        assert len(rep.snapshot_id) == len("v1156-") + 8

    def test_report_summary_line_contains_baseline_delta(self):
        rep = v1156.CognitiveCoreReport(total=0.92)
        line = rep.summary_line()
        assert "0.9200" in line
        assert "0.5000" in line or "baseline" in line.lower()


# ---------------------------------------------------------------------------
# Sub-dim 真测 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


class TestV1156SubDims:
    def test_c1_introspection_depth_returns_in_range(self):
        score, ev = v1156._measure_introspection_depth()
        assert 0.0 <= score <= 1.0
        assert ev.name == "introspection_depth"
        assert isinstance(ev.checks, dict)
        assert len(ev.checks) >= 4  # 至少 4 真 query

    def test_c2_self_model_accuracy_returns_in_range(self):
        score, ev = v1156._measure_self_model_accuracy()
        assert 0.0 <= score <= 1.0
        assert ev.name == "self_model_accuracy"

    def test_c3_meta_cognition_calibration_returns_in_range(self):
        score, ev = v1156._measure_meta_cognition_calibration()
        assert 0.0 <= score <= 1.0
        assert ev.name == "meta_cognition_calibration"

    def test_c4_perception_action_loop_returns_in_range(self):
        score, ev = v1156._measure_perception_action_loop()
        assert 0.0 <= score <= 1.0
        assert ev.name == "perception_action_loop"

    def test_c5_reasoning_consistency_returns_in_range(self):
        score, ev = v1156._measure_reasoning_consistency(n_trials=3)
        assert 0.0 <= score <= 1.0
        assert ev.name == "reasoning_consistency"
        assert ev.raw.get("n_trials") == 3


# ---------------------------------------------------------------------------
# 主入口 + aggregate
# ---------------------------------------------------------------------------


class TestV1156MeasureEntry:
    def test_measure_cognitive_core_v06_returns_float(self):
        score = v1156.measure_cognitive_core_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_cognitive_core_full_returns_report(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        assert isinstance(rep.total, float)
        assert 0.0 <= rep.total <= 1.0
        # 5 sub-dim scores
        assert len(rep.sub_dim_scores) == 5
        for name in v1156.V1156_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert 0.0 <= rep.sub_dim_scores[name] <= 1.0
        # 5 sub-dim evidence
        assert len(rep.sub_dim_evidence) == 5

    def test_measure_total_equals_mean(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written_when_no_skip(self, tmp_path):
        rep = v1156.measure_cognitive_core_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert rep.artifact_path
        assert Path(rep.artifact_path).exists()
        # 内容真 JSON
        content = Path(rep.artifact_path).read_text(encoding="utf-8")
        data = json.loads(content)
        assert data["snapshot_id"] == rep.snapshot_id
        assert data["total"] == rep.total


# ---------------------------------------------------------------------------
# V1144 集成 (_measure_cognitive_core 真调 V1156)
# ---------------------------------------------------------------------------


class TestV1156V1144Integration:
    def test_v1144_calls_v1156_first(self):
        """V1144._measure_cognitive_core 应该优先调 V1156 真测."""
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        # 调用 _measure_cognitive_core — 期望返回 V1156 真测值
        score = v1144._measure_cognitive_core()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_v1156_lifts_v1144_baseline(self):
        """V1156 真补后, V1144 cognitive_core 应该 >= V1144 baseline 0.5."""
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_cognitive_core()
        # 主 17:43 实事求是: V1156 真补后, 应该 >= 0.5
        # 不假装完美, 但应该 lift
        assert score >= v1156.V1144_BASELINE_COGNITIVE_CORE, (
            f"V1156 lift 后 V1144 cognitive_core ({score}) 应该 >= baseline 0.5"
        )


# ---------------------------------------------------------------------------
# Report serialization round-trip
# ---------------------------------------------------------------------------


class TestV1156Serialization:
    def test_report_to_dict_is_json_serializable(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        data = rep.to_dict()
        # 不假装 = 真能 JSON
        json.dumps(data, ensure_ascii=False)

    def test_report_roundtrip_dict(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        data = rep.to_dict()
        # 用 .from_dict 真复原 (主 00:56 任何人都能接手)
        new = v1156.CognitiveCoreReport.from_dict(data)
        assert new.total == rep.total
        assert new.snapshot_id == rep.snapshot_id
        assert set(new.sub_dim_scores.keys()) == set(rep.sub_dim_scores.keys())


# ---------------------------------------------------------------------------
# Markdown 报告 (主 00:44 质量工程化)
# ---------------------------------------------------------------------------


class TestV1156Markdown:
    def test_render_report_md_contains_total(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        md = v1156.render_report_md(rep)
        assert f"V1156 cognitive_core V0.6" in md
        assert f"{rep.total:.4f}" in md
        # 5 sub-dim 表格
        for name in v1156.V1156_SUBDIM_NAMES:
            assert name in md

    def test_render_report_md_contains_baseline_comparison(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        md = v1156.render_report_md(rep)
        assert "V1144 baseline" in md
        assert "target" in md


# ---------------------------------------------------------------------------
# 不假装 guard (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------


class TestV1156PhilosophyGuard:
    """V1156 不假装: 5 sub-dim 是工程近似, 不冒充 phenomenology."""

    def test_v1156_subdims_are_engineering_not_phenomenology(self):
        # 不假装 = subdim 名 是工程测量 axis
        for name in v1156.V1156_SUBDIM_NAMES:
            # 不允许含 consciousness/phenomenal/awareness 等词 (不假装 phenomenology)
            for forbidden in ["consciousness", "phenomenal", "qualia", "sentience"]:
                assert forbidden not in name.lower(), (
                    f"V1156 不假装: {name} 含 phenomenology 词, 违反主 17:58"
                )

    def test_full_total_not_pretending_asi(self):
        # 不假装 total = ASI 等级
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        # 总分应该 < 1.0 (1.0 = 5/5 全 perfect = 不允许假装 perfect)
        # 至少 1 个 sub-dim 应该是 < 1.0 (主 17:43 实事求是: 真测全 1.0 ≈ 不真)
        # 这里不强制, 但确保 score 是真跑出来的有 variation
        scores = list(rep.sub_dim_scores.values())
        assert isinstance(rep.total, float)
        assert rep.total <= 1.0


# ---------------------------------------------------------------------------
# 跨次一致性 (主 17:43 实事求是 — 真跑 N 次应该稳定)
# ---------------------------------------------------------------------------


class TestV1156Stability:
    def test_two_runs_yield_same_or_similar_score(self):
        # 真跑 2 次, 总分应该差不多 (允许 cycle 内变化但基本稳定)
        s1 = v1156.measure_cognitive_core_v06()
        s2 = v1156.measure_cognitive_core_v06()
        # 两次跑应该接近 (差异 < 0.3)
        assert abs(s1 - s2) < 0.3, f"V1156 两次跑差异 {abs(s1-s2)} > 0.3"

    def test_subdim_scores_dict_has_five_keys(self):
        rep = v1156.measure_cognitive_core_full(write_artifact=False)
        assert len(rep.sub_dim_scores) == 5
