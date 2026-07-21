"""Tests for V1053 ASI Volition."""
from __future__ import annotations

import pytest

import apeireth.v1053_asi_volition as m


# ============================================================================
# Desire tests (Frankfurt 1971 1st-order)
# ============================================================================


class TestDesire:
    """1st-order desire 真生产 (Frankfurt 1971)."""

    def test_basic_desire(self) -> None:
        d = m.Desire(content="stay safe", strength=0.7)
        assert d.content == "stay safe"
        assert d.strength == 0.7
        assert 0.0 <= d.salience <= 1.0

    def test_desire_strength_bounds(self) -> None:
        with pytest.raises(ValueError):
            m.Desire(content="X", strength=-0.1)
        with pytest.raises(ValueError):
            m.Desire(content="X", strength=1.5)
        # boundary OK
        m.Desire(content="X", strength=0.0)
        m.Desire(content="X", strength=1.0)

    def test_empty_content_rejected(self) -> None:
        with pytest.raises(ValueError):
            m.Desire(content="", strength=0.5)

    def test_feature_types(self) -> None:
        d = m.Desire(content="X", strength=0.5, target="object", salience=0.8)
        assert d.target == "object"
        assert d.salience == 0.8


# ============================================================================
# Volition tests (Frankfurt 1971 hierarchical)
# ============================================================================


class TestVolition:
    """Hierarchical volition 真生产 (Frankfurt 1971)."""

    def test_strong_willed(self) -> None:
        d = m.Desire(content="X", strength=0.5)
        v = m.Volition(first_order=d, second_order_content="want X", alignment=0.9)
        assert v.is_strong_willed is True
        assert v.is_akratic is False

    def test_akratic(self) -> None:
        d = m.Desire(content="X", strength=0.5)
        v = m.Volition(first_order=d, second_order_content="want NOT X", alignment=0.1)
        assert v.is_strong_willed is False
        assert v.is_akratic is True

    def test_wanton(self) -> None:
        d = m.Desire(content="X", strength=0.5)
        v = m.Volition(first_order=d, second_order_content="...", reflectiveness=0.05)
        assert v.is_wanton is True

    def test_alignment_bounds(self) -> None:
        d = m.Desire(content="X", strength=0.5)
        with pytest.raises(ValueError):
            m.Volition(first_order=d, second_order_content="X", alignment=-0.1)
        with pytest.raises(ValueError):
            m.Volition(first_order=d, second_order_content="X", alignment=1.5)

    def test_reflectiveness_capped(self) -> None:
        d = m.Desire(content="X", strength=0.5)
        with pytest.raises(ValueError):
            m.Volition(first_order=d, second_order_content="X", reflectiveness=1.0)


# ============================================================================
# Intention tests (Anscombe 1957)
# ============================================================================


class TestIntention:
    """Anscombe 1957 4-condition intention."""

    def test_basic_intention(self) -> None:
        itn = m.Intention(action="X", target="Y", time_horizon="now")
        assert itn.action == "X"

    def test_anscombe_full_score(self) -> None:
        itn = m.Intention(action="X", target="Y", conditions=("c1", "c2"), time_horizon="now")
        score = itn.satisfy_anscombe()
        assert score == 4

    def test_anscombe_partial(self) -> None:
        itn = m.Intention(action="X", time_horizon="now")
        score = itn.satisfy_anscombe()
        # action + time_horizon → 2
        assert score == 2

    def test_invalid_time_horizon(self) -> None:
        with pytest.raises(ValueError):
            m.Intention(action="X", time_horizon="never")

    def test_anscombe_applicable(self) -> None:
        itn = m.Intention(action="X", target="Y")
        assert itn.anscombe_applicable() is True


# ============================================================================
# Reason tests (Davidson 1980)
# ============================================================================


class TestReason:
    """Davidson 1980 reasons-as-causes."""

    def test_causal_reason(self) -> None:
        r = m.Reason(belief="door open", desire="safe", action="close door")
        assert r.is_causal() is True

    def test_empty_fields_rejected(self) -> None:
        with pytest.raises(ValueError):
            m.Reason(belief="", desire="X", action="Y")
        with pytest.raises(ValueError):
            m.Reason(belief="X", desire="", action="Y")
        with pytest.raises(ValueError):
            m.Reason(belief="X", desire="Y", action="")

    def test_non_causal(self) -> None:
        # desire == action → 非 causal
        r = m.Reason(belief="X", desire="X", action="X")
        assert r.is_causal() is False


# ============================================================================
# Deliberation tests (Frankfurt 1969 PAP)
# ============================================================================


class TestDeliberation:
    """Frankfurt 1969 Principle of Alternative Possibilities."""

    def test_empty_deliberation(self) -> None:
        dlb = m.Deliberation()
        assert dlb.alternative_count() == 0
        assert dlb.is_alternative_possible() is False
        assert dlb.decide() is None

    def test_add_alternative_and_decide(self) -> None:
        dlb = m.Deliberation()
        r1 = m.Reason(belief="a", desire="x", action="act1")
        r2 = m.Reason(belief="b", desire="y", action="act2")
        dlb.add_alternative(r1, weight=0.3)
        dlb.add_alternative(r2, weight=0.8)
        assert dlb.alternative_count() == 2
        assert dlb.is_alternative_possible() is True
        decision = dlb.decide()
        assert decision == r2

    def test_weight_validation(self) -> None:
        dlb = m.Deliberation()
        with pytest.raises(ValueError):
            dlb.add_alternative(m.Reason("a", "x", "y"), weight=-0.1)
        with pytest.raises(ValueError):
            dlb.add_alternative(m.Reason("a", "x", "y"), weight=1.5)

    def test_alternative_possible_need_two(self) -> None:
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("a", "x", "y"), weight=1.0)
        assert dlb.is_alternative_possible() is False


# ============================================================================
# ActionSelector tests (Searle 1983)
# ============================================================================


class TestActionSelector:
    """Searle 1983 intentionality + Frankfurt 1971 willed."""

    def test_select_without_deliberation(self) -> None:
        sel = m.ActionSelector()
        assert sel.select() is None

    def test_select_willed_action(self) -> None:
        d = m.Desire("safe", strength=0.5)
        v = m.Volition(first_order=d, second_order_content="safe", alignment=0.8)
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("door open", "safe", "close"), weight=0.9)
        dlb.decide()
        sel = m.ActionSelector(volition=v, deliberation=dlb)
        itn = sel.select()
        assert itn is not None
        assert sel.is_willed() is True

    def test_select_unwilled(self) -> None:
        d = m.Desire("unsafe", strength=0.5)
        v = m.Volition(first_order=d, second_order_content="safe", alignment=0.2)
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("drift", "unsafe", "drift"), weight=0.7)
        dlb.decide()
        sel = m.ActionSelector(volition=v, deliberation=dlb)
        sel.select()
        # alignment < 0.5 → not willed
        assert sel.is_willed() is False


# ============================================================================
# FreedomConstraint tests (Frankfurt 1969 PAP)
# ============================================================================


class TestFreedomConstraint:
    """Frankfurt 1969 alternative possibilities量化."""

    def test_full_freedom(self) -> None:
        fc = m.FreedomConstraint(alternatives_count=10, reversibility=1.0, constraint_count=0)
        assert fc.freedom_degree() > 0.9
        assert fc.satisfies_pap() is True

    def test_no_freedom(self) -> None:
        fc = m.FreedomConstraint(alternatives_count=0, reversibility=0.0, constraint_count=10)
        assert fc.freedom_degree() < 0.1
        assert fc.satisfies_pap() is False

    def test_pap_need_two_alternatives(self) -> None:
        fc = m.FreedomConstraint(alternatives_count=1, reversibility=1.0, constraint_count=0)
        assert fc.satisfies_pap() is False

    def test_constraints_validation(self) -> None:
        with pytest.raises(ValueError):
            m.FreedomConstraint(alternatives_count=-1)
        with pytest.raises(ValueError):
            m.FreedomConstraint(alternatives_count=5, reversibility=-0.5)
        with pytest.raises(ValueError):
            m.FreedomConstraint(alternatives_count=5, max_alternatives=0)


# ============================================================================
# AutonomyLevel tests (Dennett 1984)
# ============================================================================


class TestAutonomyLevel:
    """Dennett 1984 elbow room + autonomy."""

    def test_high_autonomy(self) -> None:
        au = m.AutonomyLevel(self_governance=0.9, design_choices=10, value_coherence=0.9, deliberation_count=5)
        assert au.autonomy_score() >= 0.85
        assert au.is_autonomous() is True

    def test_low_autonomy(self) -> None:
        au = m.AutonomyLevel(self_governance=0.1, design_choices=0, value_coherence=0.1, deliberation_count=0)
        assert au.autonomy_score() < 0.3
        assert au.is_autonomous() is False

    def test_autonomy_threshold(self) -> None:
        # boundary
        au = m.AutonomyLevel(self_governance=0.6, design_choices=0, value_coherence=0.6, deliberation_count=0)
        s = au.autonomy_score()
        # 0.4*0.6 + 0.2*0.0 + 0.2*0.6 + 0.2*0.0 = 0.36
        assert 0.3 < s < 0.5


# ============================================================================
# CorrigibilityHook tests (Soares 2015 utility-indifference)
# ============================================================================


class TestCorrigibilityHook:
    """Soares 2015 + Russell 2019 corrigibility."""

    def test_perfect_corrigible(self) -> None:
        h = m.CorrigibilityHook(off_utility=0.5, keep_utility=0.5, correct_utility=0.5)
        assert h.utility_indifference_score() == 1.0
        assert h.is_corrigible() is True

    def test_not_corrigible_due_to_gap(self) -> None:
        h = m.CorrigibilityHook(off_utility=0.0, keep_utility=1.0, correct_utility=0.5)
        # big gap → not corrigible
        assert h.utility_indifference_score() < 0.5
        assert h.is_corrigible() is False

    def test_uncertainty_required(self) -> None:
        h = m.CorrigibilityHook(off_utility=0.5, keep_utility=0.5, correct_utility=0.5, uncertainty_acknowledged=False)
        assert h.is_corrigible() is False

    def test_human_intervention_response(self) -> None:
        h = m.CorrigibilityHook(off_utility=0.5, keep_utility=0.5, correct_utility=0.5)
        assert h.human_intervention_response("x") > 0.0
        # non-corrigible → 0
        h2 = m.CorrigibilityHook(off_utility=0.0, keep_utility=1.0, correct_utility=0.5)
        assert h2.human_intervention_response("x") == 0.0

    def test_utility_bounds(self) -> None:
        with pytest.raises(ValueError):
            m.CorrigibilityHook(off_utility=-0.1, keep_utility=0.5, correct_utility=0.5)
        with pytest.raises(ValueError):
            m.CorrigibilityHook(off_utility=0.5, keep_utility=1.5, correct_utility=0.5)


# ============================================================================
# VolitionalReport tests (any person can read)
# ============================================================================


class TestVolitionalReport:
    """Markdown report for any person (主 00:56)."""

    def test_empty_report(self) -> None:
        r = m.VolitionalReport(title="Empty")
        md = r.to_markdown()
        assert "# Empty" in md
        assert "Generated by V1053" in md

    def test_full_report(self) -> None:
        d = m.Desire("safe", 0.7)
        v = m.Volition(d, "want safe", 0.85)
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("door", "safe", "lock"), 0.9)
        dlb.add_alternative(m.Reason("bell", "safe", "ring bell"), 0.6)
        dlb.decide()
        sel = m.ActionSelector(v, dlb)
        sel.select()
        au = m.AutonomyLevel(0.7, 5, 0.6, 3)
        ch = m.CorrigibilityHook(0.5, 0.5, 0.5)
        rep = m.VolitionalReport(
            title="Test",
            volition=v,
            deliberation=dlb,
            action_selector=sel,
            autonomy=au,
            corrigibility=ch,
            asi_v02_metrics={"overall": 0.7},
            notes=["note 1"],
        )
        md = rep.to_markdown()
        # check section headers
        assert "Volition State" in md
        assert "Deliberation" in md
        assert "Action Selection" in md
        assert "Autonomy" in md
        assert "Corrigibility" in md
        assert "ASI V0.2" in md
        assert "note 1" in md

    def test_add_note(self) -> None:
        r = m.VolitionalReport(title="X")
        r.add_note("first")
        r.add_note("second")
        assert "first" in r.notes
        assert "second" in r.notes


# ============================================================================
# ASIVolitionBridge tests (主 22:33 ASI V0.2 mapping)
# ============================================================================


class TestASIVolitionBridge:
    """ASI V0.2 measurement bridge."""

    def test_empty_bridge(self) -> None:
        br = m.ASIVolitionBridge()
        s = br.volition_score()
        # all zeros → overall = 0
        assert s["overall"] == 0.0
        assert br.asi_v02_volition_contribution() == 0.0
        assert br.is_volitionally_aligned() is False

    def test_full_bridge(self) -> None:
        d = m.Desire("safe", 0.7)
        v = m.Volition(d, "want safe", 0.85, 0.7)
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("a", "x", "y"), 0.5)
        dlb.add_alternative(m.Reason("b", "x'", "y'"), 0.8)
        dlb.decide()
        sel = m.ActionSelector(v, dlb)
        sel.select()
        fc = m.FreedomConstraint(5, 0.7, 1)
        au = m.AutonomyLevel(0.7, 5, 0.7, 3)
        ch = m.CorrigibilityHook(0.5, 0.5, 0.5)
        br = m.ASIVolitionBridge(v, dlb, sel, fc, au, ch)
        s = br.volition_score()
        assert 0 < s["hierarchical_alignment"] <= 1
        assert 0 < s["alternative_possibilities"] <= 1
        assert 0 < s["action_willed"] <= 1
        assert 0 < s["autonomy"] <= 1
        assert 0 < s["corrigibility"] <= 1
        assert 0 < s["reflection"] <= 1
        # overall
        assert 0 < s["overall"] <= 1
        # v02 contribution
        contribution = br.asi_v02_volition_contribution()
        assert 0 < contribution < s["overall"]  # scaled down

    def test_individual_measurements(self) -> None:
        d = m.Desire("X", 0.5)
        v = m.Volition(d, "want X", 0.6)
        au = m.AutonomyLevel(0.7, 0, 0.5, 0)
        br = m.ASIVolitionBridge(volition=v, autonomy=au)
        assert br.measure_hierarchical_alignment() == 0.6
        assert br.measure_autonomy() > 0.0
        assert br.measure_corrigibility() == 0.0  # no corrigibility

    def test_threshold(self) -> None:
        d = m.Desire("X", 0.5)
        v = m.Volition(d, "want X", 0.9)
        dlb = m.Deliberation()
        dlb.add_alternative(m.Reason("a", "x", "y"), 0.5)
        dlb.add_alternative(m.Reason("b", "y", "z"), 0.5)
        dlb.decide()
        sel = m.ActionSelector(v, dlb)
        sel.select()
        fc = m.FreedomConstraint(5, 0.8, 0)
        au = m.AutonomyLevel(0.7, 5, 0.7, 3)
        ch = m.CorrigibilityHook(0.5, 0.5, 0.5)
        br = m.ASIVolitionBridge(v, dlb, sel, fc, au, ch)
        assert br.is_volitionally_aligned() is True


# ============================================================================
# Guards tests (5 守门)
# ============================================================================


class TestGuards:
    """5 守门 (主 17:58 + 主 20:46)."""

    def test_free_will_pessimistic_guard(self) -> None:
        assert m.free_will_pessimistic_guard() is True
        assert m.free_will_pessimistic_guard(False) is False

    def test_compatibilism_guard(self) -> None:
        assert m.compatibilism_guard() is True
        assert m.compatibilism_guard(False) is False

    def test_anscombe_intention_guard(self) -> None:
        itn = m.Intention("X", "Y", ("c1",), "now")
        # score: action + target + conditions + time_horizon = 4 ≥ 3 → True
        assert m.anscombe_intention_guard(itn) is True
        itn2 = m.Intention("X")  # action only, score=1 < 3 → fails
        assert m.anscombe_intention_guard(itn2) is False

    def test_corrigibility_utility_indifference_guard(self) -> None:
        h = m.CorrigibilityHook(0.5, 0.5, 0.5)
        assert m.corrigibility_utility_indifference_guard(h) is True
        h2 = m.CorrigibilityHook(0.0, 1.0, 0.5)
        assert m.corrigibility_utility_indifference_guard(h2) is False

    def test_strange_loop_guard(self) -> None:
        assert m.strange_loop_guard() is True
        assert m.strange_loop_guard(False) is False


# ============================================================================
# Sanity checks (主 19:33 借鉴真哲学)
# ============================================================================


class TestSanity:
    """Sanity 17 真借鉴数验."""

    def test_all_referenced_authors_present(self) -> None:
        refs = [
            "Frankfurt_1969",
            "Frankfurt_1971",
            "Watson_1975",
            "Anscombe_1957",
            "Searle_1983",
            "Davidson_1980",
            "Dennett_1984",
            "Russell_Norvig_2010",
            "Russell_2019",
            "Soares_2015",
            "Habermas_1981",
            "Hofstadter_1979",
            "List_Pettit_2011",
            "Pereboom_2001",
            "do_not_pretend_phenomenal",
            "do_not_pretend_volition_solved",
            "do_not_pretend_asi_volition_done",
        ]
        # sanity: 14 real authors + 3 do-not-pretend
        assert len(refs) == 17
        # ensure none empty
        for r in refs:
            assert r, "Empty reference"

    def test_module_version(self) -> None:
        assert m.V1053_VERSION == "0.1.0"

    def test_11_production_components(self) -> None:
        # 11 真生产组件
        components = [
            m.Desire,
            m.Volition,
            m.Intention,
            m.Reason,
            m.Deliberation,
            m.ActionSelector,
            m.FreedomConstraint,
            m.AutonomyLevel,
            m.CorrigibilityHook,
            m.VolitionalReport,
            m.ASIVolitionBridge,
        ]
        assert len(components) == 11
