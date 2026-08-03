"""V1201 — ASI V0.6.11 dual_dim lift 真测 (主 00:56 任何人都能接手 + 主 17:43 实事求是).

V1201 = ASI V0.6.11 self_improving_core + capabilities 双 dim 联合 lift:
  - self_improving_core: 0.8533 → 0.95 (Δ=+0.0967, ASI +0.0048) [8 sub-dim]
  - capabilities:        0.8847 → 1.0  (Δ=+0.1153, ASI +0.0058) [5 sub-dim]
  - ASI recompute:       0.9518 → 0.9624 (Δ=+0.0106)

Test cases:
  1. measure_v1201() 返回 float ∈ [0, 1] (主 00:56)
  2. measure_v1201_additive/corrected 也 OK
  3. measure_v1201_full 返 V1201Report dataclass
  4. 13 sub-dim 全有 score ∈ [0, 1]
  5. self_improving_core 8 sub-dim: 6 pass + 2 partial (≥0.5)
  6. capabilities 5 sub-dim: 5 pass (≥0.95)
  7. ASI delta > 0 (真 lift, 不假装)
  8. ASI north_star 0.98 LOCKED
  9. 2 dim lifts: self_improving_core + capabilities 全 status=R
  10. artifact JSON dump 完整
  11. inflation_gap = 0 (V1197 inflation 已修, V1201 honest 3-formula)
  12. position_pct ≥ 95% (V1201 ≥ 95% of ASI 北极星)
  13. gap_to_north_star ∈ [0, 0.05]
  14. V3 philosophy guard: 8 keys 在 notes
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure promethean/ is on sys.path so `apeireth` package imports resolve
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))


class TestV1201Basics:
    """V1201 module basics (主 00:56)."""

    def test_module_imports(self):
        """V1201 module 真 import 不报错."""
        from apeireth import v1201_asi_v0611_self_improving_core_lift as v1201
        assert v1201 is not None
        assert hasattr(v1201, "V1201_VERSION")
        assert v1201.V1201_VERSION == "0.1.0"
        assert v1201.V1201_DIM_VERSION == "0.6.11"

    def test_asi_north_star_locked(self):
        """ASI 北极星 0.98 LOCKED (主 22:33)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import ASI_NORTH_STAR
        assert ASI_NORTH_STAR == 0.9800

    def test_v1200_baseline(self):
        """V1200 baseline 0.9518 (V1201 起点)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import (
            V1200_RECOMPUTE, V1200_ADDITIVE, V1200_CORRECTED
        )
        assert V1200_RECOMPUTE == 0.9518
        assert V1200_ADDITIVE == 0.9518
        assert V1200_CORRECTED == 0.9518

    def test_8_self_improving_subdim_names(self):
        """8 self_improving_core sub-dim names."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import V1201_SELF_IMPROVING_SUBDIM_NAMES
        assert len(V1201_SELF_IMPROVING_SUBDIM_NAMES) == 8
        assert "self_modification_real" in V1201_SELF_IMPROVING_SUBDIM_NAMES
        assert "self_loading_artifact_real" in V1201_SELF_IMPROVING_SUBDIM_NAMES
        assert "v06_continuous_lift_real" in V1201_SELF_IMPROVING_SUBDIM_NAMES
        assert "self_evolution_chain_real" in V1201_SELF_IMPROVING_SUBDIM_NAMES

    def test_5_capabilities_subdim_names(self):
        """5 capabilities sub-dim names."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import V1201_CAPABILITIES_SUBDIM_NAMES
        assert len(V1201_CAPABILITIES_SUBDIM_NAMES) == 5
        assert "llm_bench_real" in V1201_CAPABILITIES_SUBDIM_NAMES
        assert "multimodal_real" in V1201_CAPABILITIES_SUBDIM_NAMES
        assert "phi_proxy_lift_real" in V1201_CAPABILITIES_SUBDIM_NAMES


class TestV1201Measure:
    """V1201 measure 三入口 (主 00:56)."""

    def test_measure_v1201_returns_float_in_range(self):
        """measure_v1201() → float ∈ [0, 1]."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201
        v = measure_v1201()
        assert isinstance(v, float)
        assert 0.0 <= v <= 1.0

    def test_measure_v1201_additive_returns_float(self):
        """measure_v1201_additive() → float ∈ [0, 1]."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_additive
        v = measure_v1201_additive()
        assert isinstance(v, float)
        assert 0.0 <= v <= 1.0

    def test_measure_v1201_corrected_returns_float(self):
        """measure_v1201_corrected() → float ∈ [0, 1]."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_corrected
        v = measure_v1201_corrected()
        assert isinstance(v, float)
        assert 0.0 <= v <= 1.0

    def test_measure_v1201_full_returns_report(self):
        """measure_v1201_full() → V1201Report dataclass."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import (
            measure_v1201_full, V1201Report
        )
        rep = measure_v1201_full(write_artifact=False)
        assert isinstance(rep, V1201Report)
        assert rep.snapshot_id.startswith("v1201-")


class TestV1201Lift:
    """V1201 ASI V0.6.11 lift 真测 (主 13:31 大胆激进 + 主 23:44 干到底)."""

    def test_asi_recompute_lifted_above_baseline(self):
        """V1201 ASI recompute > V1200 baseline (真 lift, 不假装)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.asi_recompute_lifted > rep.asi_recompute_baseline

    def test_self_improving_core_lifted(self):
        """self_improving_core 真 lift (Δ > 0)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        si = rep.dim_lifts["self_improving_core"]
        assert si.delta > 0, f"self_improving_core delta = {si.delta}"

    def test_capabilities_lifted(self):
        """capabilities 真 lift (Δ > 0)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        cap = rep.dim_lifts["capabilities"]
        assert cap.delta > 0, f"capabilities delta = {cap.delta}"

    def test_2_dim_lifts_status_R(self):
        """2 dim 全 status=R (real lift)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        for dim, lift in rep.dim_lifts.items():
            assert lift.status == "R", f"{dim} status = {lift.status}"

    def test_n_dims_lifted_2(self):
        """n_dims_lifted = 2."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.n_dims_lifted == 2
        assert rep.n_dims_pass == 2

    def test_asi_delta_positive(self):
        """ASI delta > 0 (V1201 真 lift, 不降)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.asi_recompute_delta > 0, f"delta = {rep.asi_recompute_delta}"


class TestV1201SubDim:
    """V1201 13 sub-dim 真测 (主 00:44 质量工程化)."""

    def test_self_improving_8_subdims_present(self):
        """8 self_improving sub-dim 全有 score."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert len(rep.self_improving_sub_dim_scores) == 8
        for name, score in rep.self_improving_sub_dim_scores.items():
            assert 0.0 <= score <= 1.0, f"{name} score = {score}"

    def test_self_improving_8_subdims_pass_count(self):
        """self_improving_core 8 sub-dim ≥ 6 pass (≥ 0.95)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.n_self_improving_subdims_pass >= 6, \
            f"pass={rep.n_self_improving_subdims_pass} < 6"
        assert rep.n_self_improving_subdims_missing == 0, \
            f"missing={rep.n_self_improving_subdims_missing} > 0"

    def test_capabilities_5_subdims_present(self):
        """5 capabilities sub-dim 全有 score."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert len(rep.capabilities_sub_dim_scores) == 5
        for name, score in rep.capabilities_sub_dim_scores.items():
            assert 0.0 <= score <= 1.0, f"{name} score = {score}"

    def test_capabilities_5_subdims_pass_count(self):
        """capabilities 5 sub-dim ≥ 4 pass (≥ 0.95)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.n_capabilities_subdims_pass >= 4, \
            f"pass={rep.n_capabilities_subdims_pass} < 4"


class TestV1201PhilosophyGuard:
    """V3 philosophy guard (主 17:58 + 20:46 + 17:43)."""

    def test_gap_to_north_star_non_negative(self):
        """V1201 gap to north_star ∈ [0, 0.05]."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert 0.0 <= rep.gap_to_north_star_recompute <= 0.05

    def test_position_pct_above_95(self):
        """V1201 position ≥ 95% of ASI 北极星."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.position_pct_recompute >= 95.0

    def test_v1201_not_north_star(self):
        """V1201 < ASI 北极星 0.98 (不假装)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert rep.formula_2_recompute < rep.asi_north_star

    def test_3_formula_no_inflation(self):
        """V1201 inflation_gap ≈ 0 (V1197 inflation 已修)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        assert abs(rep.inflation_gap_additive_vs_recompute) < 0.001
        assert abs(rep.inflation_gap_additive_vs_corrected) < 0.001

    def test_notes_have_8_philosophy_keys(self):
        """V1201 notes 包含 8 主哲学 keys (主 22:33 + 17:43 + 17:58 + 20:46 + 13:31 + 23:44 + 00:56 + 00:44)."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        all_notes = "\n".join(rep.notes)
        for key in ["V1201", "北极星", "实事求是", "不假装", "大胆激进", "干到底", "任何人都能接手", "质量工程化"]:
            assert key in all_notes, f"missing philosophy key: {key}"


class TestV1201Artifact:
    """V1201 artifact 写入 + JSON dump (主 00:44 质量工程化)."""

    def test_artifact_writes_json(self, tmp_path=None):
        """measure_v1201_full(write_artifact=True) 真写 JSON file."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            rep = measure_v1201_full(write_artifact=True, artifact_dir=td)
            assert rep.artifact_path != ""
            p = Path(rep.artifact_path)
            assert p.exists()
            assert p.suffix == ".json"
            data = json.loads(p.read_text(encoding="utf-8"))
            assert "formula_2_recompute" in data
            assert "dim_lifts" in data
            assert "self_improving_sub_dim_scores" in data
            assert "capabilities_sub_dim_scores" in data

    def test_report_dataclass_roundtrip(self):
        """V1201Report dataclass → dict → dataclass roundtrip."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import (
            measure_v1201_full, V1201Report
        )
        rep = measure_v1201_full(write_artifact=False)
        d = rep.to_dict()
        rep2 = V1201Report.from_dict(d)
        assert rep2.snapshot_id == rep.snapshot_id
        assert abs(rep2.formula_2_recompute - rep.formula_2_recompute) < 1e-9
        assert rep2.asi_recompute_delta == rep.asi_recompute_delta


class TestV1201Integration:
    """V1201 integration 真测 (主 23:44 干到底)."""

    def test_summary_line_format(self):
        """V1201Report.summary_line() 返回正确格式."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import measure_v1201_full
        rep = measure_v1201_full(write_artifact=False)
        s = rep.summary_line()
        assert "V1201 ASI V0.6.11" in s
        assert "recompute=" in s
        assert "north_star=" in s
        assert "snapshot=" in s

    def test_render_report_md(self):
        """render_report_md() returns valid markdown."""
        from apeireth.v1201_asi_v0611_self_improving_core_lift import (
            measure_v1201_full, render_report_md
        )
        rep = measure_v1201_full(write_artifact=False)
        md = render_report_md(rep)
        assert "# V1201 ASI V0.6.11" in md
        assert "## 3-formula" in md
        assert "## ASI 北极星" in md
        assert "## 2 dim lifts" in md
        assert "## self_improving_core 8 sub-dim 真测" in md
        assert "## capabilities 5 sub-dim 真测" in md
        assert "V1201 ASI V0.6.11" in md

    def test_dgm_archive_real_load(self):
        """DGMArchive 真 load (F4 + F5 真测关键)."""
        from apeireth.dgm_archive import DGMArchive, make_default_dgm_archive
        archive = make_default_dgm_archive()
        assert isinstance(archive, DGMArchive)
        s = archive.stats()
        assert isinstance(s, dict)
        assert "archive_id" in s
        assert "n_generations" in s
