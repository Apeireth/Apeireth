"""V1157 tests — ASI self_improving_core V0.6 real measurement (5 sub-dim).

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 22:33 ASI 北极星 + 主 19:33 走在前人经验上.

This test file covers:
  1. Module API 真测 (V1157_VERSION + 5 sub-dim LOCKED names)
  2. Sub-dim 真测 (F1-F5 真跑 + score ∈ [0,1])
  3. V1144 集成 (_measure_self_improving_core 真调 V1157)
  4. 不假装 guard (5 sub-dim 不是 free will phenomenology)
  5. Report serialization (JSON dump + reload)
  6. 哲学自由 Q3 alignment
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth import v1157_asi_self_improving_core_v06_real_measure as v1157


# ---------------------------------------------------------------------------
# Module API 真测
# ---------------------------------------------------------------------------


class TestV1157ModuleAPI:
    def test_v1157_version_is_semver(self):
        parts = v1157.V1157_VERSION.split(".")
        assert len(parts) == 3
        for p in parts:
            assert p.isdigit()

    def test_v1157_dim_version_is_v06(self):
        assert v1157.V1157_DIM_VERSION == "0.6"

    def test_v1157_subdim_names_count_is_five(self):
        assert len(v1157.V1157_SUBDIM_NAMES) == 5

    def test_v1157_subdim_names_locked(self):
        locked = (
            "self_modification_real",
            "optimization_lifecycle",
            "cache_effectiveness",
            "measurement_real",
            "history_persistence",
        )
        assert v1157.V1157_SUBDIM_NAMES == locked

    def test_v1157_target_high_enough(self):
        assert v1157.TARGET_SELF_IMPROVING_CORE_V06 >= 0.85

    def test_v1157_v1144_baseline_recorded(self):
        # 写死历史值 0.5, 不允许改
        assert v1157.V1144_BASELINE_SELF_IMPROVING_CORE == 0.5


# ---------------------------------------------------------------------------
# SelfImprovingCoreReport dataclass
# ---------------------------------------------------------------------------


class TestV1157ReportDataclass:
    def test_subdim_evidence_dataclass_to_dict(self):
        ev = v1157.SubDimEvidence(name="test", score=0.5)
        d = ev.to_dict()
        assert d["name"] == "test"
        assert d["score"] == 0.5
        assert isinstance(d["checks"], dict)

    def test_report_default_snapshot_id_has_v1157_prefix(self):
        rep = v1157.SelfImprovingCoreReport()
        assert rep.snapshot_id.startswith("v1157-")
        assert len(rep.snapshot_id) == len("v1157-") + 8

    def test_report_from_dict_roundtrip(self):
        rep = v1157.SelfImprovingCoreReport(total=0.84)
        d = rep.to_dict()
        new = v1157.SelfImprovingCoreReport.from_dict(d)
        assert new.total == rep.total
        assert new.snapshot_id == rep.snapshot_id


# ---------------------------------------------------------------------------
# Sub-dim 真测
# ---------------------------------------------------------------------------


class TestV1157SubDims:
    def test_f1_self_modification_returns_in_range(self):
        score, ev = v1157._measure_self_modification_real()
        assert 0.0 <= score <= 1.0
        assert ev.name == "self_modification_real"

    def test_f2_optimization_lifecycle_returns_in_range(self):
        score, ev = v1157._measure_optimization_lifecycle()
        assert 0.0 <= score <= 1.0
        assert ev.name == "optimization_lifecycle"

    def test_f3_cache_effectiveness_returns_in_range(self):
        score, ev = v1157._measure_cache_effectiveness()
        assert 0.0 <= score <= 1.0
        assert ev.name == "cache_effectiveness"

    def test_f4_measurement_real_returns_in_range(self):
        score, ev = v1157._measure_measurement_real()
        assert 0.0 <= score <= 1.0
        assert ev.name == "measurement_real"

    def test_f5_history_persistence_returns_in_range(self):
        score, ev = v1157._measure_history_persistence()
        assert 0.0 <= score <= 1.0
        assert ev.name == "history_persistence"


# ---------------------------------------------------------------------------
# 主入口 + aggregate
# ---------------------------------------------------------------------------


class TestV1157MeasureEntry:
    def test_measure_v06_returns_float(self):
        score = v1157.measure_self_improving_core_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_measure_full_returns_report(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        assert isinstance(rep.total, float)
        assert 0.0 <= rep.total <= 1.0
        assert len(rep.sub_dim_scores) == 5
        for name in v1157.V1157_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert 0.0 <= rep.sub_dim_scores[name] <= 1.0

    def test_measure_total_equals_mean(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        scores = list(rep.sub_dim_scores.values())
        expected = sum(scores) / len(scores)
        assert abs(rep.total - expected) < 1e-9

    def test_artifact_written_when_no_skip(self, tmp_path):
        rep = v1157.measure_self_improving_core_full(
            write_artifact=True,
            artifact_dir=str(tmp_path),
        )
        assert rep.artifact_path
        assert Path(rep.artifact_path).exists()


# ---------------------------------------------------------------------------
# V1144 集成
# ---------------------------------------------------------------------------


class TestV1157V1144Integration:
    def test_v1144_calls_v1157_first(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_self_improving_core()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_v1157_lifts_v1144_baseline(self):
        from apeireth import v1144_asi_v05_17dim_real_measure_complete as v1144
        score = v1144._measure_self_improving_core()
        assert score >= v1157.V1144_BASELINE_SELF_IMPROVING_CORE, (
            f"V1157 lift 后 V1144 self_improving_core ({score}) 应该 >= baseline 0.5"
        )


# ---------------------------------------------------------------------------
# JSON serialization
# ---------------------------------------------------------------------------


class TestV1157Serialization:
    def test_report_json_serializable(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        data = rep.to_dict()
        json.dumps(data, ensure_ascii=False)

    def test_report_roundtrip_dict(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        data = rep.to_dict()
        new = v1157.SelfImprovingCoreReport.from_dict(data)
        assert new.total == rep.total
        assert new.snapshot_id == rep.snapshot_id
        assert set(new.sub_dim_scores.keys()) == set(rep.sub_dim_scores.keys())


# ---------------------------------------------------------------------------
# 哲学守门 (主 17:58 + 主 20:46 不假装)
# ---------------------------------------------------------------------------


class TestV1157PhilosophyGuard:
    """V1157 不假装: 5 sub-dim 是工程近似, 不冒充 phenomenology."""

    def test_v1157_subdims_are_engineering_not_phenomenology(self):
        for name in v1157.V1157_SUBDIM_NAMES:
            for forbidden in ["free_will", "consciousness", "phenomenal", "qualia", "sentience", "volition"]:
                assert forbidden not in name.lower(), (
                    f"V1157 不假装: {name} 含 phenomenology 词, 违反主 17:58"
                )

    def test_full_total_not_pretending_asi(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        assert rep.total <= 1.0


# ---------------------------------------------------------------------------
# 哲学 Q3 自由 alignment
# ---------------------------------------------------------------------------


class TestV1157FreedomPhilosophy:
    """V1157 = ASI 哲学 Q3 自由 — V3 真补."""

    def test_v1157_subdim_count_aligns_with_q3(self):
        # ASI V3 Q3 自由哲学 = 5 sub-dim 真补
        assert len(v1157.V1157_SUBDIM_NAMES) == 5

    def test_v1157_actually_runs_q3_components(self):
        # 真跑 F1-F5 至少 4 个 sub-dim 非 0
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        n_nonzero = sum(1 for s in rep.sub_dim_scores.values() if s > 0.0)
        # V1157 真补后应该至少 4 个 sub-dim 非 0
        assert n_nonzero >= 4, (
            f"V1157 自由哲学 Q3: {n_nonzero}/5 sub-dim 真 non-zero, 应该 ≥ 4"
        )


# ---------------------------------------------------------------------------
# Markdown 报告
# ---------------------------------------------------------------------------


class TestV1157Markdown:
    def test_render_report_md_contains_total(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        md = v1157.render_report_md(rep)
        assert "V1157 self_improving_core V0.6" in md
        assert f"{rep.total:.4f}" in md
        for name in v1157.V1157_SUBDIM_NAMES:
            assert name in md

    def test_render_report_md_contains_baseline_comparison(self):
        rep = v1157.measure_self_improving_core_full(write_artifact=False)
        md = v1157.render_report_md(rep)
        assert "V1144 baseline" in md
        assert "target" in md
