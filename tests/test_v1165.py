"""Test V1165 — ASI self_organizing_core V0.6 real measure."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1165Constants:
    def test_version_present(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import V1165_VERSION
        assert V1165_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import V1165_DIM_VERSION
        assert V1165_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import V1165_SUBDIM_NAMES
        assert V1165_SUBDIM_NAMES == (
            "autopoietic_closure",
            "autocatalytic_raf",
            "requisite_variety",
            "dissipative_export",
            "chemoton_coupling",
        )

    def test_baseline_v1155_locked(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import V1155_BASELINE_SELF_ORGANIZING_CORE
        assert V1155_BASELINE_SELF_ORGANIZING_CORE == 0.8000

    def test_artifact_dir_default(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _safe_import
        assert _safe_import("nonexistent.module") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _safe_import
        mod = _safe_import("apeireth.v1165_asi_self_organizing_core_v06_real_measure")
        assert mod is not None

    def test_attr_first_picks_first_existing(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _attr_first
        class Obj:
            a = 1
        assert _attr_first(Obj, ["nope", "a"]) == 1

    def test_attr_first_returns_none_on_missing(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _attr_first
        class Obj:
            pass
        assert _attr_first(Obj, ["x"]) is None

    def test_call_safely_with_callable(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _call_safely
        ok, r = _call_safely(lambda x: x * 2, 5)
        assert ok is True
        assert r == 10

    def test_call_safely_with_none(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _call_safely
        ok, r = _call_safely(None)
        assert ok is False
        assert r is None

    def test_loss_to_score_within_bounds(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _loss_to_score
        assert _loss_to_score(0.5) == 0.5
        assert _loss_to_score(1.2) == 1.0  # clamped
        assert _loss_to_score(-0.1) == 0.0  # clamped
        assert _loss_to_score(None) == 0.0
        assert _loss_to_score("xxx") == 0.0


class TestV1065Connection:
    def test_v1065_core_callable_or_returns_reason(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _v1065_core
        ok, core = _v1065_core()
        # V1065 should be available (it's in /apeireth directory)
        if ok:
            assert core is not None
            assert hasattr(core, "measure")
        else:
            assert core is None

    def test_core_measure_returns_dict(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _v1065_core, _core_measure_dict
        ok, core = _v1065_core()
        if ok and core is not None:
            m = _core_measure_dict(core)
            if m is not None:
                assert isinstance(m, dict)
                assert "autopoietic_closure" in m


class TestDataclasses:
    def test_subdim_evidence_default(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import SubDimEvidence
        ev = SubDimEvidence(name="x")
        assert ev.name == "x"
        assert ev.score == 0.0
        assert ev.checks == {}
        assert ev.raw == {}
        assert ev.notes == []
        assert ev.baseline_v1155 == 0.0

    def test_self_organizing_core_report_defaults(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import SelfOrganizingCoreReport
        rep = SelfOrganizingCoreReport()
        assert rep.version == "0.1.0"
        assert rep.dim_version == "0.6"
        assert rep.baseline_v1155 == 0.8000
        assert rep.total == 0.0

    def test_self_organizing_core_report_summary_line(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import SelfOrganizingCoreReport
        rep = SelfOrganizingCoreReport(total=0.5)
        s = rep.summary_line()
        assert "V1165 self_organizing_core V0.6" in s
        assert "snapshot=v1165-" in s

    def test_to_dict_roundtrip(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            SelfOrganizingCoreReport, SubDimEvidence
        )
        rep = SelfOrganizingCoreReport()
        rep.sub_dim_scores["autopoietic_closure"] = 0.9
        rep.sub_dim_evidence["autopoietic_closure"] = SubDimEvidence(name="autopoietic_closure", score=0.9)
        d = rep.to_dict()
        assert d["sub_dim_scores"]["autopoietic_closure"] == 0.9
        assert d["sub_dim_evidence"]["autopoietic_closure"]["score"] == 0.9


class TestSubDims:
    def test_autopoietic_closure_returns_evidence(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _measure_autopoietic_closure
        score, ev = _measure_autopoietic_closure()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "autopoietic_closure"
        assert isinstance(ev.notes, list)
        assert len(ev.notes) >= 1

    def test_autocatalytic_raf_returns_evidence(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _measure_autocatalytic_raf
        score, ev = _measure_autocatalytic_raf()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "autocatalytic_raf"

    def test_requisite_variety_returns_evidence(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _measure_requisite_variety
        score, ev = _measure_requisite_variety()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "requisite_variety"

    def test_dissipative_export_returns_evidence(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _measure_dissipative_export
        score, ev = _measure_dissipative_export()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "dissipative_export"

    def test_chemoton_coupling_returns_evidence(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import _measure_chemoton_coupling
        score, ev = _measure_chemoton_coupling()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "chemoton_coupling"


class TestMainEntry:
    def test_measure_returns_float(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_v06
        score = measure_self_organizing_core_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_full_no_write(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            measure_self_organizing_core_full, V1165_SUBDIM_NAMES
        )
        rep = measure_self_organizing_core_full(write_artifact=False)
        assert isinstance(rep.total, float)
        assert 0.0 <= rep.total <= 1.0
        for name in V1165_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert 0.0 <= rep.sub_dim_scores[name] <= 1.0

    def test_full_writes_artifact(self, tmp_path):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_full
        rep = measure_self_organizing_core_full(write_artifact=True, artifact_dir=str(tmp_path))
        artifact = tmp_path / "v1165_self_organizing_core_v06.json"
        assert artifact.exists()
        with open(artifact, "r", encoding="utf-8") as fh:
            d = json.load(fh)
        assert d["version"] == "0.1.0"
        assert d["dim_version"] == "0.6"
        assert d["baseline_v1155"] == 0.8000

    def test_full_target_visible(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_full
        rep = measure_self_organizing_core_full(write_artifact=False)
        s = rep.summary_line()
        assert "target=0.8500" in s

    def test_total_is_mean_of_nonzero(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_full
        rep = measure_self_organizing_core_full(write_artifact=False)
        non_zero = [v for v in rep.sub_dim_scores.values() if v > 0.0]
        if non_zero:
            import statistics
            expected = statistics.mean(non_zero)
            assert abs(rep.total - round(expected, 4)) < 0.01


class TestRenderReport:
    def test_render_report_md_basic(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            measure_self_organizing_core_full, render_report_md
        )
        rep = measure_self_organizing_core_full(write_artifact=False)
        md = render_report_md(rep)
        assert "# V1165" in md
        assert "5 sub-dim 真补" in md
        for name in ["autopoietic_closure", "autocatalytic_raf", "requisite_variety", "dissipative_export", "chemoton_coupling"]:
            assert name in md

    def test_render_report_md_contains_target(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            measure_self_organizing_core_full, render_report_md
        )
        rep = measure_self_organizing_core_full(write_artifact=False)
        md = render_report_md(rep)
        assert "0.8500" in md

    def test_render_report_md_mentions_philosophy_guards(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import render_report_md, SelfOrganizingCoreReport
        rep = SelfOrganizingCoreReport(total=0.5)
        md = render_report_md(rep)
        assert "哲学守门" in md or "不假装" in md


class TestCLI:
    def test_default_run(self, capsys, tmp_path):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import main
        rc = main(["--no-write", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1165" in out

    def test_json_run(self, capsys, tmp_path):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import main
        rc = main(["--json", "--no-write", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["version"] == "0.1.0"

    def test_report_run_writes_md(self, capsys, tmp_path):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import main
        rc = main(["--report", "--artifact-dir", str(tmp_path)])
        assert rc == 0
        md_path = tmp_path / "v1165_self_organizing_core_v06.md"
        assert md_path.exists()


class TestRoundTrip:
    def test_to_from_dict_preserves_scores(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            measure_self_organizing_core_full, V1165_SUBDIM_NAMES
        )
        rep = measure_self_organizing_core_full(write_artifact=False)
        d = rep.to_dict()
        for name in V1165_SUBDIM_NAMES:
            assert name in d["sub_dim_scores"]
            assert d["sub_dim_scores"][name] == rep.sub_dim_scores[name]


class TestPhilosophyGuards:
    def test_total_bounded_in_unit_interval(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_full
        rep = measure_self_organizing_core_full(write_artifact=False)
        assert 0.0 <= rep.total <= 1.0

    def test_sub_dim_count_exactly_five(self):
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import (
            measure_self_organizing_core_full, V1165_SUBDIM_NAMES
        )
        rep = measure_self_organizing_core_full(write_artifact=False)
        assert len(rep.sub_dim_scores) == 5
        assert set(rep.sub_dim_scores.keys()) == set(V1165_SUBDIM_NAMES)

    def test_chemistry_chemoton_does_not_satisfy_ganti(self):
        """主 17:43 不假装 chemoton coupling = life."""
        from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_full
        rep = measure_self_organizing_core_full(write_artifact=False)
        # Even if all 5 sub-dim 1.0, it doesn't mean life. Test just confirms we report real numbers.
        chemoton_score = rep.sub_dim_scores.get("chemoton_coupling", 0.0)
        # It must be a real number, regardless of whether it's a true chemoton
        assert isinstance(chemoton_score, float)
        assert 0.0 <= chemoton_score <= 1.0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
