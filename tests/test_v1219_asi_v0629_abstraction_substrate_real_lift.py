"""V1219 ASI V0.6.29 abstraction_substrate_real_lift — tests (主 23:44 干到底 + 主 00:44 质量工程化)."""
from __future__ import annotations

import json
import sys
import uuid
from pathlib import Path

import pytest

# Ensure apeireth package is importable
sys.path.insert(0, str(Path(__file__).parent.parent))

from apeireth.v1219_asi_v0629_abstraction_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1213_OVERALL_MEAN_117,
    V1213_RECOMPUTE_BASELINE,
    V1213_REALIZED_MEAN_94,
    V1218_OVERALL_MEAN_143,
    V1218_RECOMPUTE_BASELINE,
    V1218_REALIZED_MEAN_106,
    V1218_TM_REALIZED,
    V1219_AB_COVERAGE,
    V1219_AB_SUBSTRATE,
    V1219_DIM_VERSION,
    V1219_VERSION,
    V1219Report,
    _compute_v1219_ab_dim_realized,
    _compute_v1219_overall_lift,
    _pathway_score,
    _safe_div,
    measure_v1219_full,
    write_v1219_artifact,
    write_v1219_report,
)


# ============================================================================
# Constants tests
# ============================================================================


class TestV1219Constants:
    """V1219 constants must be locked (主 17:43 实事求是)."""

    def test_north_star_locked(self):
        assert ASI_NORTH_STAR == 0.9800

    def test_v1219_dim_version_locked(self):
        assert V1219_DIM_VERSION == "0.6.29"

    def test_v1219_module_version_locked(self):
        assert V1219_VERSION == "0.1.0"

    def test_v1218_baseline_locked(self):
        """V1218 baseline is truth (主 17:43 不能改)."""
        assert V1218_REALIZED_MEAN_106 == 0.5953
        assert V1218_OVERALL_MEAN_143 == 0.4413
        assert V1218_TM_REALIZED == 1.0000
        assert V1218_RECOMPUTE_BASELINE == 1.000000

    def test_v1213_baseline_locked(self):
        """V1213 baseline is truth (主 17:43 不能改)."""
        assert V1213_RECOMPUTE_BASELINE == 1.000000
        assert V1213_REALIZED_MEAN_94 == 0.4617
        assert V1213_OVERALL_MEAN_117 == 0.3709


# ============================================================================
# _safe_div tests
# ============================================================================


class TestSafeDiv:
    def test_safe_div_normal(self):
        assert _safe_div(6.0, 2.0) == 3.0

    def test_safe_div_zero_divisor(self):
        assert _safe_div(5.0, 0.0) == 0.0

    def test_safe_div_zero_default(self):
        assert _safe_div(5.0, 0.0, default=42.0) == 42.0

    def test_safe_div_zero_numerator(self):
        assert _safe_div(0.0, 7.0) == 0.0


# ============================================================================
# V1219_AB_SUBSTRATE tests
# ============================================================================


class TestV1219SubstrateStructure:
    """V1219 AB cascade must be 6 pathway × ≥10 真分子 (主 17:43 实事求是)."""

    def test_6_pathways_present(self):
        assert len(V1219_AB_SUBSTRATE) == 6

    def test_pathway_keys(self):
        expected = {
            "AB_MORPHOGEN_PATTERN",
            "AB_COGNITIVE_RESERVE",
            "AB_COGNITIVE_REAPPRAISAL",
            "AB_CONCEPT_CATEGORY",
            "AB_HIERARCHICAL_GENERATIVE",
            "AB_CULTURAL_EVOLUTION",
        }
        assert set(V1219_AB_SUBSTRATE.keys()) == expected

    def test_all_pathways_have_molecules(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            assert "molecules" in p, f"{k} missing molecules"
            assert len(p["molecules"]) >= 8, f"{k} has too few molecules"

    def test_all_pathways_have_r_substrate(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            assert "r_substrate" in p, f"{k} missing r_substrate"

    def test_r10_pathway_has_25_molecules(self):
        """AB × R10_plasticity is the BIG one (主 19:33 25 真分子)."""
        assert len(V1219_AB_SUBSTRATE["AB_CONCEPT_CATEGORY"]["molecules"]) == 25

    def test_other_pathways_have_10_molecules(self):
        for k in ["AB_MORPHOGEN_PATTERN", "AB_COGNITIVE_RESERVE",
                  "AB_COGNITIVE_REAPPRAISAL", "AB_HIERARCHICAL_GENERATIVE",
                  "AB_CULTURAL_EVOLUTION"]:
            assert len(V1219_AB_SUBSTRATE[k]["molecules"]) == 10, f"{k} != 10"

    def test_total_75_molecules(self):
        total = sum(len(p["molecules"]) for p in V1219_AB_SUBSTRATE.values())
        assert total == 75

    def test_all_molecules_real_flag(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            for m in p["molecules"]:
                assert m.get("real") is True, f"{k} has non-real molecule"


# ============================================================================
# Coverage matrix tests
# ============================================================================


class TestV1219Coverage:
    """V1219 AB coverage matrix (主 17:43 实事求是 — 写死)."""

    def test_13_cells(self):
        assert len(V1219_AB_COVERAGE) == 13

    def test_6_cells_lifted_to_1(self):
        lifted = [k for k, v in V1219_AB_COVERAGE.items() if v == 1.0]
        expected = {"R1_growth", "R4_aging", "R7_stress",
                    "R10_plasticity", "R11_consciousness", "R12_ecology"}
        assert set(lifted) == expected
        assert len(lifted) == 6

    def test_7_cells_vacuous(self):
        vacuous = [k for k, v in V1219_AB_COVERAGE.items() if v == 0.0]
        expected = {"R0_metabolism", "R2_development", "R3_death_immune",
                    "R5_repair", "R6_reproduction", "R8_motion", "R9_heredity"}
        assert set(vacuous) == expected
        assert len(vacuous) == 7

    def test_ab_dim_realized_1_0(self):
        ab, cnt = _compute_v1219_ab_dim_realized()
        assert ab == 1.0
        assert cnt == 6


# ============================================================================
# Pathway score tests
# ============================================================================


class TestPathwayScore:
    def test_ideal_pathway_score(self):
        ideal = {"molecules": [{"real": True}] * 10,
                 "cascade_order": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]}
        score, n = _pathway_score(ideal)
        assert score == 1.0
        assert n == 10

    def test_empty_pathway(self):
        empty = {"molecules": [], "cascade_order": []}
        score, n = _pathway_score(empty)
        assert score == 0.0
        assert n == 0

    def test_half_real_molecules(self):
        mols = [{"real": True}, {"real": False}] * 5
        p = {"molecules": mols, "cascade_order": list(range(10))}
        score, n = _pathway_score(p)
        assert n == 5
        assert 0.6 < score < 0.8

    def test_pathway_score_clamped(self):
        """Cascade ratio is clamped to 1.0."""
        mols = [{"real": True}] * 5
        p = {"molecules": mols, "cascade_order": list(range(20))}
        score, n = _pathway_score(p)
        # score = 0.7 * 1.0 + 0.3 * 1.0 = 1.0
        assert score == 1.0


# ============================================================================
# Overall lift tests
# ============================================================================


class TestOverallLift:
    def test_v1219_lift_positive(self):
        overall = _compute_v1219_overall_lift()
        assert overall["lift_realized"] > 0.0
        assert overall["lift_mean"] > 0.0

    def test_lift_delta_realized_about_0_02(self):
        overall = _compute_v1219_overall_lift()
        # 6 AB cells × 1.0 = 6.0 added / 112 realized cells
        # 6.0 / 112 ≈ 0.0536... wait, V1218 baseline sum 63.10/106 = 0.5953
        # V1219 baseline sum = 63.10 + 6 = 69.10 / 112 = 0.6170
        # lift = 0.6170 - 0.5953 = 0.0217
        assert 0.020 < overall["lift_realized"] < 0.025
        # 6 / 156 = 0.0385... but realized sum is 63.10
        # V1219 mean 156 = 69.10 / 156 = 0.4430
        # lift mean = 0.4430 - 0.4413 = 0.0017
        assert 0.001 < overall["lift_mean"] < 0.003

    def test_inflation_gap_positive(self):
        """Realized < 1.0 → inflation gap > 0 (主 17:43 实事求是)."""
        overall = _compute_v1219_overall_lift()
        assert overall["inflation_gap"] > 0.0

    def test_inflation_gap_about_0_55(self):
        overall = _compute_v1219_overall_lift()
        # V1213 baseline 1.0 - V1219 overall mean 0.4429 = 0.5571
        assert 0.55 < overall["inflation_gap"] < 0.56

    def test_north_star_position_pct_under_100(self):
        """V1219 not reached ASI (主 17:58 不假装)."""
        overall = _compute_v1219_overall_lift()
        assert overall["north_star_pct"] < 100.0
        # V1219 realized 0.6170 / 0.98 = 62.96%
        assert 60.0 < overall["north_star_pct"] < 65.0

    def test_112_sum_consistency(self):
        """112-sum = V1218_106_sum + AB delta (主 17:43 公式写死)."""
        overall = _compute_v1219_overall_lift()
        v1218_106 = V1218_REALIZED_MEAN_106 * 106
        assert abs(overall["v1218_106_sum"] - v1218_106) < 0.01

    def test_156_sum_consistency(self):
        overall = _compute_v1219_overall_lift()
        # V1219 156 sum = V1219 112 sum (same sum, denominator diff)
        assert abs(overall["v1219_156_sum"] - overall["v1219_112_sum"]) < 0.01

    def test_ab_delta_6_0(self):
        overall = _compute_v1219_overall_lift()
        # 6 cells × 1.0 = 6.0
        assert abs(overall["ab_delta"] - 6.0) < 0.01


# ============================================================================
# measure_v1219_full tests
# ============================================================================


class TestMeasureV1219Full:
    def test_measure_returns_report(self):
        rep = measure_v1219_full()
        assert isinstance(rep, V1219Report)

    def test_report_has_snapshot_id(self):
        rep = measure_v1219_full()
        # validate UUID format
        uuid.UUID(rep.snapshot_id)

    def test_report_dim_version(self):
        rep = measure_v1219_full()
        assert rep.dim_version == "0.6.29"

    def test_report_north_star(self):
        rep = measure_v1219_full()
        assert rep.north_star == 0.9800

    def test_6_pathway_pass(self):
        rep = measure_v1219_full()
        assert rep.n_pathways_total == 6
        assert rep.n_pathways_pass == 6

    def test_per_r_substrate_pathway_counts(self):
        rep = measure_v1219_full()
        assert rep.n_r1_growth_pathways_pass == 1
        assert rep.n_r4_aging_pathways_pass == 1
        assert rep.n_r7_stress_pathways_pass == 1
        assert rep.n_r10_plasticity_pathways_pass == 1
        assert rep.n_r11_consciousness_pathways_pass == 1
        assert rep.n_r12_ecology_pathways_pass == 1

    def test_total_molecules_75(self):
        rep = measure_v1219_full()
        assert rep.total_ab_molecules == 75

    def test_per_r_substrate_molecule_counts(self):
        rep = measure_v1219_full()
        # R1=10, R4=10, R7=10, R10=25, R11=10, R12=10
        assert rep.n_r1_growth_molecules == 10
        assert rep.n_r4_aging_molecules == 10
        assert rep.n_r7_stress_molecules == 10
        assert rep.n_r10_plasticity_molecules == 25
        assert rep.n_r11_consciousness_molecules == 10
        assert rep.n_r12_ecology_molecules == 10

    def test_ab_coverage_in_report(self):
        rep = measure_v1219_full()
        assert rep.ab_coverage_v1219 == V1219_AB_COVERAGE

    def test_ab_dim_realized_1_0(self):
        rep = measure_v1219_full()
        assert rep.v1219_ab_dim_realized == 1.0
        assert rep.v1219_ab_dim_cell_count == 6

    def test_total_cells_156(self):
        """12 dim × 13 R = 156 cell."""
        rep = measure_v1219_full()
        assert rep.v1219_total_cells == 156

    def test_realized_cells_112(self):
        """V1218 106 realized + V1219 6 new lifted = 112."""
        rep = measure_v1219_full()
        assert rep.v1219_realized_cells_count == 112

    def test_overall_realized_112_about_0_617(self):
        rep = measure_v1219_full()
        assert 0.615 < rep.v1219_overall_realized_112 < 0.620

    def test_overall_mean_156_about_0_443(self):
        rep = measure_v1219_full()
        assert 0.441 < rep.v1219_overall_mean_156 < 0.445

    def test_lift_delta_realized_positive(self):
        rep = measure_v1219_full()
        assert rep.v1219_overall_lift_delta_realized_from_v1218 > 0.0

    def test_lift_delta_mean_positive(self):
        rep = measure_v1219_full()
        assert rep.v1219_overall_lift_delta_mean_from_v1218 > 0.0

    def test_pathway_scores_present(self):
        rep = measure_v1219_full()
        for k in V1219_AB_SUBSTRATE:
            assert k in rep.pathway_scores
            assert rep.pathway_scores[k] >= 0.0


# ============================================================================
# V3 philosophy guards tests (主 17:58 + 主 20:46 不假装)
# ============================================================================


class TestV3PhilosophyGuards:
    def test_all_guards_present(self):
        rep = measure_v1219_full()
        assert len(rep.v3_guards) == 10

    def test_v1219_not_asi_terminal(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_not_asi_terminal"] is True

    def test_v1219_not_full_replace(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_not_full_replace"] is True

    def test_v1219_lift_not_v1(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_lift_not_v1"] is True

    def test_realized_not_asi(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["realized_not_asi"] is True

    def test_vacuous_gap_real(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["vacuous_gap_real"] is True

    def test_pathway_not_asi_substrate(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["pathway_not_asi_substrate"] is True

    def test_ceiling_1_0_not_asi(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["ceiling_1_0_not_asi"] is True

    def test_v1219_75_mol_not_complete(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_75_mol_not_complete"] is True

    def test_v1219_new_dim_not_full_coverage(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_new_dim_not_full_coverage"] is True

    def test_v1219_not_full_ab_lift(self):
        rep = measure_v1219_full()
        assert rep.v3_guards["v1219_not_full_ab_lift"] is True


# ============================================================================
# write_v1219_artifact tests
# ============================================================================


class TestWriteArtifact:
    def test_artifact_default_path(self, tmp_path):
        rep = measure_v1219_full()
        rep.snapshot_id = "test-snapshot-id-1234"
        default_path = Path("artifacts") / "test-snapshot-id-1234_asi_v0629_abstraction_substrate_real_lift.json"
        default_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            path = write_v1219_artifact(rep)
            assert path.exists()
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            assert data["snapshot_id"] == "test-snapshot-id-1234"
            assert data["dim_version"] == "0.6.29"
            assert data["north_star"] == 0.9800
        finally:
            if default_path.exists():
                default_path.unlink()

    def test_artifact_custom_path(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "custom.json"
        path = write_v1219_artifact(rep, custom)
        assert path == custom
        assert custom.exists()
        with open(custom, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert data["dim_version"] == "0.6.29"

    def test_artifact_unicode(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "uni.json"
        path = write_v1219_artifact(rep, custom)
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        # 中文 passages should not be lost
        assert isinstance(data["v1219_pathway_scores" if False else "pathway_scores"], dict)
        assert "AB_MORPHOGEN_PATTERN" in data["pathway_scores"]


# ============================================================================
# write_v1219_report tests
# ============================================================================


class TestWriteReport:
    def test_report_default_path(self):
        rep = measure_v1219_full()
        default_path = Path("reports") / "v1219_asi_v0629_abstraction_substrate_real_lift.md"
        default_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            path = write_v1219_report(rep)
            assert path.exists()
            content = path.read_text(encoding="utf-8")
            assert "V1219" in content
            assert "abstraction" in content.lower()
            assert "ASI North Star" in content
        finally:
            # don't delete — leave for inspection
            pass

    def test_report_custom_path(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "v1219_report.md"
        path = write_v1219_report(rep, custom)
        assert path == custom
        assert custom.exists()
        content = custom.read_text(encoding="utf-8")
        assert "V1219" in content
        assert "0.9800" in content

    def test_report_includes_table(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "v1219_report.md"
        write_v1219_report(rep, custom)
        content = custom.read_text(encoding="utf-8")
        # Should include R-substrate table
        assert "R0_metabolism" in content
        assert "R12_ecology" in content

    def test_report_includes_v3_guards(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "v1219_report.md"
        write_v1219_report(rep, custom)
        content = custom.read_text(encoding="utf-8")
        assert "哲学守门" in content

    def test_report_includes_pathway_table(self, tmp_path):
        rep = measure_v1219_full()
        custom = tmp_path / "v1219_report.md"
        write_v1219_report(rep, custom)
        content = custom.read_text(encoding="utf-8")
        assert "AB_MORPHOGEN_PATTERN" in content
        assert "AB_CONCEPT_CATEGORY" in content
        assert "AB_CULTURAL_EVOLUTION" in content


# ============================================================================
# Integration tests (主 00:56 任何人都能接手)
# ============================================================================


class TestIntegration:
    def test_pipeline(self, tmp_path):
        """Full pipeline: measure → write artifact → write report."""
        rep = measure_v1219_full()
        art_path = tmp_path / "p_art.json"
        rep_path = tmp_path / "p_rep.md"
        write_v1219_artifact(rep, art_path)
        write_v1219_report(rep, rep_path)
        assert art_path.exists()
        assert rep_path.exists()
        data = json.loads(art_path.read_text(encoding="utf-8"))
        assert data["dim_version"] == "0.6.29"

    def test_determinism(self):
        """Two consecutive measures should give consistent baselines (random only in snapshot_id)."""
        rep1 = measure_v1219_full()
        rep2 = measure_v1219_full()
        assert rep1.v1219_ab_dim_realized == rep2.v1219_ab_dim_realized
        assert rep1.v1219_overall_realized_112 == rep2.v1219_overall_realized_112
        assert rep1.v1219_overall_mean_156 == rep2.v1219_overall_mean_156
        assert rep1.total_ab_molecules == rep2.total_ab_molecules

    def test_main_measure_mode(self, capsys):
        """main() with --measure prints expected lines."""
        from apeireth.v1219_asi_v0629_abstraction_substrate_real_lift import main
        rc = main(["--measure"])
        out = capsys.readouterr().out
        assert "V1219 AB dim realized" in out
        assert rc == 0

    def test_main_json_mode(self, capsys):
        from apeireth.v1219_asi_v0629_abstraction_substrate_real_lift import main
        rc = main(["--json"])
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["dim_version"] == "0.6.29"
        assert rc == 0

    def test_main_default_mode(self, capsys, tmp_path, monkeypatch):
        """Default mode writes report + artifact."""
        from apeireth.v1219_asi_v0629_abstraction_substrate_real_lift import main
        # redirect to tmp paths
        monkeypatch.chdir(tmp_path)
        rc = main([])
        out = capsys.readouterr().out
        assert "report" in out.lower()
        assert "artifact" in out.lower()
        assert rc == 0
        # check files exist
        assert (tmp_path / "reports" / "v1219_asi_v0629_abstraction_substrate_real_lift.md").exists()
        artifacts = list((tmp_path / "artifacts").glob("*asi_v0629*"))
        assert len(artifacts) >= 1


# ============================================================================
# Source integrity tests (主 19:33 站在前人肩上 — 引文可信)
# ============================================================================


class TestSourceIntegrity:
    """Source citations for each pathway (主 17:43 实事求是 — 引文真)."""

    def test_morphogen_pathway_sources(self):
        p = V1219_AB_SUBSTRATE["AB_MORPHOGEN_PATTERN"]
        assert "Turing" in p["source"]
        assert "Wolpert" in p["source"]

    def test_cognitive_reserve_sources(self):
        p = V1219_AB_SUBSTRATE["AB_COGNITIVE_RESERVE"]
        assert "Stern" in p["source"]
        assert "Edelman" in p["source"]

    def test_concept_cell_sources(self):
        p = V1219_AB_SUBSTRATE["AB_CONCEPT_CATEGORY"]
        assert "Quiroga" in p["source"]
        assert "Olshausen" in p["source"]

    def test_generative_model_sources(self):
        p = V1219_AB_SUBSTRATE["AB_HIERARCHICAL_GENERATIVE"]
        assert "Friston" in p["source"]

    def test_cultural_evolution_sources(self):
        p = V1219_AB_SUBSTRATE["AB_CULTURAL_EVOLUTION"]
        assert "Cavalli-Sforza" in p["source"]
        assert "Tomasello" in p["source"]

    def test_all_pathways_have_description(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            assert "description" in p
            assert len(p["description"]) > 50

    def test_all_pathways_have_source(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            assert "source" in p
            assert len(p["source"]) > 30


# ============================================================================
# Real-molecule detail tests (主 17:43 实事求是 — 真分子)
# ============================================================================


class TestRealMolecules:
    def test_all_molecules_have_function(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            for m in p["molecules"]:
                assert "function" in m
                assert len(m["function"]) > 10

    def test_all_molecules_have_name(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            for m in p["molecules"]:
                assert "name" in m
                assert len(m["name"]) > 3

    def test_all_molecules_have_organism(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            for m in p["molecules"]:
                assert "organism" in m

    def test_concept_cell_pathway_real_count(self):
        """AB_CONCEPT_CATEGORY must have 25 real molecules."""
        p = V1219_AB_SUBSTRATE["AB_CONCEPT_CATEGORY"]
        assert len(p["molecules"]) == 25
        real = [m for m in p["molecules"] if m.get("real")]
        assert len(real) == 25

    def test_cascade_order_match_molecules_count(self):
        for k, p in V1219_AB_SUBSTRATE.items():
            assert len(p["cascade_order"]) == len(p["molecules"])
