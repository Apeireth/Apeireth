"""Tests for V1065 ASI Self-Organizing Core (主 17:43 实事求是 + 主 00:56).

真借鉴 14 前人: Ashby 1956 + Maturana-Varela 1980 + Kauffman 1993 +
Prigogine 1977 + Haken 1983 + Gánti 1975 + Rosen 1959 + Fontana 1994 +
Sperry 1969 + Beer 2000 + Pearl 2009 + Holland 1995 + Gell-Mann 1994 +
Friston 2010.

10 真生产组件 + 5 守门 + ASI V0.2 桥接.
"""
from __future__ import annotations

import random

import pytest

from apeireth.v1065_asi_self_organizing_core import (
    V1065_VERSION,
    AdaptiveNetwork,
    Agent,
    ASISelfOrganizingCoreBridge,
    AutocatalyticSet,
    AutopoieticCycle,
    Chemoton,
    ClosureKind,
    DissipativeStructure,
    MRClosure,
    OrderParameter,
    RequisiteVariety,
    SelfOrganizingCore,
    SelfOrganizingGuard,
    SelfOrganizingReport,
    build_self_organizing_core,
    quick_score,
)


# ---------------------------------------------------------------------------
# 1. AutopoieticCycle tests (Maturana/Varela 1980)
# ---------------------------------------------------------------------------

class TestAutopoieticCycle:
    def test_init_empty(self):
        ac = AutopoieticCycle()
        assert ac.components == []
        assert ac.productions == []
        assert ac.closure_kind() == ClosureKind.NONE

    def test_add_production_creates_components(self):
        ac = AutopoieticCycle()
        ac.add_production("A", "B")
        assert "A" in ac.components
        assert "B" in ac.components

    def test_partial_closure(self):
        ac = AutopoieticCycle()
        ac.add_production("A", "B")
        assert ac.closure_kind() == ClosureKind.PARTIAL

    def test_autopoietic_closure(self):
        ac = AutopoieticCycle()
        for src, dst in [("A", "B"), ("B", "C"), ("C", "A")]:
            ac.add_production(src, dst)
        assert ac.closure_kind() == ClosureKind.AUTOPOIETIC

    def test_component_count(self):
        ac = AutopoieticCycle()
        ac.add_production("A", "B")
        ac.add_production("C", "D")
        assert ac.component_count() == 4


# ---------------------------------------------------------------------------
# 2. AutocatalyticSet tests (Kauffman 1993)
# ---------------------------------------------------------------------------

class TestAutocatalyticSet:
    def test_init_empty(self):
        raf = AutocatalyticSet()
        assert raf.n_reactions() == 0
        assert raf.is_raf(food=[]) is False

    def test_add_reaction_tracks_molecules(self):
        raf = AutocatalyticSet()
        raf.add_reaction(["a"], ["b"])
        assert "a" in raf.molecules
        assert "b" in raf.molecules

    def test_raf_with_food(self):
        raf = AutocatalyticSet()
        raf.add_reaction(["f1"], ["a"])
        raf.add_reaction(["a"], ["b"])
        raf.add_reaction(["b"], ["a"])
        assert raf.is_raf(food=["f1"]) is True

    def test_raf_no_food(self):
        raf = AutocatalyticSet()
        raf.add_reaction(["a"], ["b"])
        raf.add_reaction(["b"], ["a"])
        assert raf.is_raf(food=[]) is True  # all reactants are products

    def test_n_reactions(self):
        raf = AutocatalyticSet()
        raf.add_reaction(["a"], ["b"])
        raf.add_reaction(["b"], ["c"])
        assert raf.n_reactions() == 2


# ---------------------------------------------------------------------------
# 3. RequisiteVariety tests (Ashby 1956)
# ---------------------------------------------------------------------------

class TestRequisiteVariety:
    def test_init_zero(self):
        rv = RequisiteVariety()
        assert rv.variety_disturbance() == 0
        assert rv.variety_response() == 0
        assert rv.ratio() == 0.0

    def test_add_disturbance(self):
        rv = RequisiteVariety()
        rv.add_disturbance("d1")
        rv.add_disturbance("d2")
        assert rv.variety_disturbance() == 2

    def test_add_response(self):
        rv = RequisiteVariety()
        rv.add_response("r1")
        assert rv.variety_response() == 1

    def test_ratio(self):
        rv = RequisiteVariety()
        for i in range(4):
            rv.add_disturbance(f"d{i}")
            rv.add_response(f"r{i}")
        assert rv.ratio() == 1.0

    def test_meets_requisite(self):
        rv = RequisiteVariety()
        rv.add_disturbance("d1")
        rv.add_response("r1")
        rv.add_response("r2")
        assert rv.meets_requisite() is True


# ---------------------------------------------------------------------------
# 4. DissipativeStructure tests (Prigogine 1977)
# ---------------------------------------------------------------------------

class TestDissipativeStructure:
    def test_init_zero(self):
        ds = DissipativeStructure()
        assert ds.dissipation_rate() == 0.0
        assert ds.export_rate() == 0.0

    def test_step_accumulates(self):
        ds = DissipativeStructure()
        ds.step(internal_entropy=0.5, exported_entropy=0.7)
        assert ds.n_events == 1
        assert ds.entropy_in == 0.5
        assert ds.entropy_out == 0.7

    def test_dissipation_rate(self):
        ds = DissipativeStructure()
        ds.step(0.4, 0.6)
        ds.step(0.4, 0.6)
        assert ds.dissipation_rate() == pytest.approx(0.4)

    def test_export_rate(self):
        ds = DissipativeStructure()
        ds.step(0.3, 0.5)
        ds.step(0.3, 0.5)
        assert ds.export_rate() == pytest.approx(0.5)

    def test_is_dissipative(self):
        ds = DissipativeStructure()
        ds.step(0.2, 0.5)  # export > import
        assert ds.is_dissipative() is True

    def test_net_entropy(self):
        ds = DissipativeStructure()
        ds.step(0.3, 0.5)
        assert ds.net_entropy() == pytest.approx(-0.2)


# ---------------------------------------------------------------------------
# 5. OrderParameter tests (Haken 1983)
# ---------------------------------------------------------------------------

class TestOrderParameter:
    def test_init_zero(self):
        op = OrderParameter()
        assert op.magnitude == 0.0
        assert op.variance == 0.0

    def test_update_empty(self):
        op = OrderParameter()
        op.update([])
        assert op.magnitude == 0.0

    def test_update_sets_magnitude(self):
        op = OrderParameter()
        op.update([1.0, 2.0, 3.0])
        assert op.magnitude == pytest.approx(2.0)  # abs(mean)

    def test_dominance_at_criticality(self):
        op = OrderParameter()
        op.update([0.0, 5.0, -3.0, 4.0, -2.0])  # high variance
        assert op.dominance() > 0.5

    def test_is_critical(self):
        op = OrderParameter()
        op.update([1.0, 1.5, 0.5, 1.2, 0.8])  # low variance
        assert op.is_critical() is False


# ---------------------------------------------------------------------------
# 6. Chemoton tests (Gánti 1975)
# ---------------------------------------------------------------------------

class TestChemoton:
    def test_init_zero_coupling(self):
        cm = Chemoton()
        assert cm.coupling() == 0.0
        assert cm.is_viable() is False

    def test_set_subsystem(self):
        cm = Chemoton()
        cm.set_subsystem(metabolism=2.0, template=1.5, compartment=2.0)
        assert cm.metabolism_rate == 2.0

    def test_coupling_increases(self):
        cm = Chemoton()
        cm.set_subsystem(metabolism=2.5, template=1.5, compartment=2.0)
        assert cm.coupling() > 0.5

    def test_viable(self):
        cm = Chemoton()
        cm.set_subsystem(metabolism=1.0, template=1.0, compartment=1.0)
        assert cm.is_viable() is True

    def test_not_viable_without_metabolism(self):
        cm = Chemoton()
        cm.set_subsystem(metabolism=0.0, template=1.0, compartment=1.0)
        assert cm.is_viable() is False


# ---------------------------------------------------------------------------
# 7. MRClosure tests (Rosen 1959)
# ---------------------------------------------------------------------------

class TestMRClosure:
    def test_init_not_closed(self):
        mr = MRClosure()
        assert mr.is_closed() is False

    def test_add_state(self):
        mr = MRClosure()
        mr.add_state("x", 0.0)
        assert mr.n_states() == 1

    def test_add_transform(self):
        mr = MRClosure()
        mr.add_state("x", 0.0)
        mr.add_transform("mapping", "x", "y")
        assert mr.mapping == {"x": "y"}

    def test_closure_when_fully_mapped(self):
        mr = MRClosure()
        mr.add_state("x", 0.0)
        mr.add_state("y", 0.0)
        mr.add_transform("mapping", "x", "y")
        mr.add_transform("mapping", "y", "x")
        assert mr.is_closed() is True

    def test_not_closed_without_mappings(self):
        mr = MRClosure()
        mr.add_state("x", 0.0)
        mr.add_state("y", 0.0)
        assert mr.is_closed() is False


# ---------------------------------------------------------------------------
# 8. AdaptiveNetwork tests (Holland 1995)
# ---------------------------------------------------------------------------

class TestAdaptiveNetwork:
    def test_init_empty(self):
        an = AdaptiveNetwork()
        assert an.effective_diversity() == 0
        assert an.mean_fitness() == 0.0

    def test_add_agent(self):
        an = AdaptiveNetwork()
        an.add_agent(tag="t1", strategy="s1", fitness=0.5)
        assert len(an.agents) == 1

    def test_effective_diversity(self):
        an = AdaptiveNetwork()
        for i in range(5):
            an.add_agent(tag=f"t{i}", strategy=f"s{i % 2}", fitness=0.0)
        assert an.effective_diversity() == 2

    def test_tick_runs(self):
        an = AdaptiveNetwork(selection_pressure=1.0)
        for i in range(3):
            an.add_agent(tag=f"t{i}", strategy=f"s{i}", fitness=0.5)
        n_updated = an.tick()
        assert n_updated == 3  # all updated due to pressure=1.0

    def test_mean_fitness(self):
        an = AdaptiveNetwork()
        an.add_agent(tag="t1", strategy="s1", fitness=0.6)
        an.add_agent(tag="t2", strategy="s2", fitness=0.4)
        assert an.mean_fitness() == pytest.approx(0.5)


# ---------------------------------------------------------------------------
# 9. SelfOrganizingReport tests (主 00:56)
# ---------------------------------------------------------------------------

class TestSelfOrganizingReport:
    def test_report_init(self):
        rep = SelfOrganizingReport()
        assert rep.title == "ASI Self-Organizing Core Report"
        assert rep.sections == []

    def test_report_add_section(self):
        rep = SelfOrganizingReport()
        rep.add_section("Test", "Body")
        assert ("Test", "Body") in rep.sections

    def test_report_render_has_title(self):
        rep = SelfOrganizingReport(title="Custom")
        rep.add_section("Components", "1. Cycle")
        md = rep.render()
        assert "# Custom" in md
        assert "## Components" in md

    def test_report_render_has_guards(self):
        rep = SelfOrganizingReport()
        md = rep.render()
        assert "V3 哲学守门" in md
        assert "Autopoiesis = Self-Awareness" in md

    def test_summary_dict(self):
        s = SelfOrganizingReport.summary_dict(10, 20, 5, "AUTOPOIETIC", 0.8667)
        assert "10" in s
        assert "AUTOPOIETIC" in s


# ---------------------------------------------------------------------------
# 10. ASISelfOrganizingCoreBridge tests
# ---------------------------------------------------------------------------

class TestASISelfOrganizingCoreBridge:
    def test_bridge_init_weights_sum_to_one(self):
        b = ASISelfOrganizingCoreBridge()
        assert sum(b.weights.values()) == pytest.approx(1.0, abs=1e-9)

    def test_score_zero(self):
        b = ASISelfOrganizingCoreBridge()
        r = b.score({})
        assert r["self_organizing_core_v0_2"] == 0.0

    def test_score_perfect(self):
        b = ASISelfOrganizingCoreBridge()
        perfect = {k: 1.0 for k in b.weights}
        r = b.score(perfect)
        assert r["self_organizing_core_v0_2"] == pytest.approx(1.0, abs=1e-9)

    def test_score_contributions(self):
        b = ASISelfOrganizingCoreBridge()
        r = b.score({"autopoietic_closure": 0.5})
        assert "contributions" in r
        assert "autopoietic_closure" in r["contributions"]

    def test_threshold_pass(self):
        b = ASISelfOrganizingCoreBridge()
        r = b.threshold_check(0.90)
        assert r["passed"] is True

    def test_threshold_fail(self):
        b = ASISelfOrganizingCoreBridge()
        r = b.threshold_check(0.5)
        assert r["passed"] is False


# ---------------------------------------------------------------------------
# 11. SelfOrganizingGuard tests (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------

class TestSelfOrganizingGuard:
    def test_autopoiesis_not_awareness(self):
        g = SelfOrganizingGuard.guard_autopoiesis_not_awareness(
            {"autopoietic_closure": 0.5})
        assert g["guard"] == "autopoiesis_not_awareness"
        assert "Maturana" in g["verdict"]

    def test_closure_not_consciousness(self):
        g = SelfOrganizingGuard.guard_closure_not_consciousness({"mr_closure": 0.5})
        assert g["guard"] == "closure_not_consciousness"
        assert "Chalmers" in g["verdict"]

    def test_autocatalytic_not_intelligence(self):
        g = SelfOrganizingGuard.guard_autocatalytic_not_intelligence(
            {"autocatalytic_raf": 0.5})
        assert g["guard"] == "autocatalytic_not_intelligence"
        assert "Kauffman" in g["verdict"]

    def test_emergence_not_asi(self):
        g = SelfOrganizingGuard.guard_emergence_not_asi(
            {"order_param_dominance": 0.5})
        assert g["guard"] == "emergence_not_asi"
        assert "Haken" in g["verdict"]

    def test_self_org_not_understanding(self):
        g = SelfOrganizingGuard.guard_self_org_not_understanding(
            {"adaptive_diversity": 0.5})
        assert g["guard"] == "self_org_not_understanding"
        assert "Holland" in g["verdict"]

    def test_all_guards_returns_5(self):
        g = SelfOrganizingGuard.all_guards({})
        assert len(g) == 5

    def test_high_score_does_not_pretend(self):
        """Even with score=1.0, guard verdict must NOT claim ASI consciousness."""
        g = SelfOrganizingGuard.all_guards({
            "autopoietic_closure": 1.0,
            "mr_closure": 1.0,
            "autocatalytic_raf": 1.0,
            "order_param_dominance": 1.0,
            "adaptive_diversity": 1.0,
        })
        for entry in g:
            assert "NOT" in entry["verdict"] or "≠" in entry["verdict"]


# ---------------------------------------------------------------------------
# 12. SelfOrganizingCore pipeline integration
# ---------------------------------------------------------------------------

class TestSelfOrganizingCore:
    def test_default_build(self):
        soc = build_self_organizing_core()
        assert isinstance(soc, SelfOrganizingCore)
        assert soc.autopoietic.component_count() > 0
        assert soc.autocatalytic.n_reactions() > 0
        assert soc.variety.variety_disturbance() == 5

    def test_measure_runs(self):
        soc = build_self_organizing_core()
        m = soc.measure()
        assert "autopoietic_closure" in m
        assert "mr_closure" in m
        assert "report_readability" in m

    def test_score_runs(self):
        soc = build_self_organizing_core()
        s = soc.score()
        assert "self_organizing_core_v0_2" in s
        assert 0.0 <= s["self_organizing_core_v0_2"] <= 1.0

    def test_score_target(self):
        """Default builder targets ≥0.85 self_organizing_core (V0.2 dimension)."""
        soc = build_self_organizing_core()
        s = soc.score()
        # Build default is engineered for ≥0.85 (per ASI V0.2 dimension target)
        assert s["self_organizing_core_v0_2"] >= 0.85

    def test_threshold_pass(self):
        soc = build_self_organizing_core()
        assert soc.threshold_pass(target=0.85) is True

    def test_quick_score(self):
        r = quick_score()
        assert "self_organizing_core_v0_2" in r
        assert 0.0 <= r["self_organizing_core_v0_2"] <= 1.0

    def test_make_report(self):
        soc = build_self_organizing_core()
        md = soc.make_report()
        assert "# ASI Self-Organizing Core Report" in md
        assert "Ashby 1956" in md
        assert "V3 哲学守门" in md
        assert "Maturana/Varela" in md


# ---------------------------------------------------------------------------
# 13. Sanity tests (主 17:43 实事求是)
# ---------------------------------------------------------------------------

class TestSanity:
    def test_version(self):
        assert V1065_VERSION == "0.1.0"

    def test_14_precedents_documented(self):
        import apeireth.v1065_asi_self_organizing_core as mod
        src = mod.__doc__ or ""
        expected = ["Ashby 1956", "Maturana", "Varela", "Kauffman 1993",
                    "Prigogine 1977", "Haken 1983", "Gánti 1975",
                    "Rosen 1959", "Fontana 1994", "Sperry 1969",
                    "Beer 2000", "Pearl 2009", "Holland 1995",
                    "Gell-Mann 1994", "Friston 2010"]
        for ref in expected:
            assert ref in src, f"missing: {ref}"

    def test_10_components_documented(self):
        import apeireth.v1065_asi_self_organizing_core as mod
        src = mod.__doc__ or ""
        for comp in ["AutopoieticCycle", "AutocatalyticSet",
                     "RequisiteVariety", "DissipativeStructure",
                     "OrderParameter", "Chemoton", "MRClosure",
                     "AdaptiveNetwork", "SelfOrganizingReport",
                     "ASISelfOrganizingCoreBridge"]:
            assert comp in src, f"missing: {comp}"

    def test_5_guards_documented(self):
        import apeireth.v1065_asi_self_organizing_core as mod
        src = mod.__doc__ or ""
        for guard in ["Autopoiesis = Self-Awareness",
                      "Closure = Consciousness",
                      "Autocatalytic = Intelligence",
                      "Emergence = ASI",
                      "Self-Organization = Understanding"]:
            assert guard in src, f"missing: {guard}"

    def test_no_pretend_consciousness(self):
        import apeireth.v1065_asi_self_organizing_core as mod
        with open(mod.__file__, encoding="utf-8") as f:
            src = (mod.__doc__ or "") + f.read()
        forbidden_phrases = ["Autopoiesis IS awareness",
                             "RAF IS intelligence",
                             "emergence == ASI",
                             "self-organization == understanding"]
        for phrase in forbidden_phrases:
            assert phrase not in src

    def test_reproducibility(self):
        """Same build → same score."""
        random.seed(42)
        soc1 = build_self_organizing_core()
        s1 = soc1.score()["self_organizing_core_v0_2"]
        soc2 = build_self_organizing_core()
        s2 = soc2.score()["self_organizing_core_v0_2"]
        assert s1 == s2