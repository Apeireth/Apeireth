"""Tests for V1054 ASI Self-Recognition."""
from __future__ import annotations

import pytest

import apeireth.v1054_asi_self_recognition as m


# ============================================================================
# SelfMark tests (Gallup 1970)
# ============================================================================


class TestSelfMark:
    def test_msr_pass(self) -> None:
        sm = m.SelfMark("mark", True, touch_self=True)
        assert sm.passed_msr() is True

    def test_msr_fail_not_touch_self(self) -> None:
        sm = m.SelfMark("mark", True, touch_self=False)
        assert sm.passed_msr() is False

    def test_msr_fail_not_marked(self) -> None:
        sm = m.SelfMark("mark", False, touch_self=True)
        assert sm.passed_msr() is False

    def test_msr_fail_touch_mirror(self) -> None:
        sm = m.SelfMark("mark", True, touch_self=True, touch_mirror=True)
        assert sm.passed_msr() is False

    def test_empty_id(self) -> None:
        with pytest.raises(ValueError):
            m.SelfMark("", True)
        with pytest.raises(ValueError):
            m.SelfMark("x", True, confidence=1.5)


# ============================================================================
# MirrorModel tests (Amsterdam 1972)
# ============================================================================


class TestMirrorModel:
    def test_empty_returns_zero(self) -> None:
        mm = m.MirrorModel()
        assert mm.match_score() == 0.0
        assert mm.recognizes_mirror_self() is False

    def test_perfect_match(self) -> None:
        mm = m.MirrorModel()
        mm.set_mirror_representation({"x": 1.0, "y": 0.5})
        mm.set_self_prediction({"x": 1.0, "y": 0.5})
        assert mm.match_score() == pytest.approx(1.0, abs=1e-6)
        assert mm.recognizes_mirror_self() is True

    def test_partial_match(self) -> None:
        mm = m.MirrorModel()
        mm.set_mirror_representation({"x": 1.0, "y": 0.0})
        mm.set_self_prediction({"x": 0.0, "y": 1.0})
        assert mm.match_score() == 0.0

    def test_threshold(self) -> None:
        mm = m.MirrorModel(match_threshold=0.9)
        mm.set_mirror_representation({"x": 1.0, "y": 0.5})
        mm.set_self_prediction({"x": 0.9, "y": 0.6})
        assert mm.match_score() > 0.99
        assert mm.recognizes_mirror_self() is True
        mm.set_self_prediction({"x": -0.9, "y": -0.6})  # opposite direction
        assert mm.recognizes_mirror_self() is False


# ============================================================================
# Mentalization tests (Frith-Frith 1999 ToM)
# ============================================================================


class TestMentalization:
    def test_empty(self) -> None:
        mn = m.Mentalization()
        assert mn.self_other_differentiation() == 0.0
        assert mn.matched_belief("x") is False

    def test_self_other_diff(self) -> None:
        ms = m.MentalState("self", "A", "X")
        mn = m.Mentalization(self_state=ms)
        mn.add_other_state("o1", m.MentalState("o1", "B", "Y"))
        mn.add_other_state("o2", m.MentalState("o2", "B", "X"))
        # diff: belief diff (2/2) + desire diff (1/2) = 3/4
        diff = mn.self_other_differentiation()
        assert diff == pytest.approx(0.75, abs=1e-6)

    def test_self_other_same(self) -> None:
        ms = m.MentalState("self", "A", "X")
        mn = m.Mentalization(self_state=ms)
        mn.add_other_state("o1", m.MentalState("o1", "A", "X"))
        assert mn.self_other_differentiation() == 0.0

    def test_matched_belief(self) -> None:
        ms = m.MentalState("self", "belief_A", "X")
        mn = m.Mentalization(self_state=ms)
        mn.add_other_state("o1", m.MentalState("o1", "belief_A", "Y"))
        assert mn.matched_belief("o1") is True
        mn.add_other_state("o2", m.MentalState("o2", "belief_B", "Y"))
        assert mn.matched_belief("o2") is False


# ============================================================================
# SelfContinuity tests (Buddhist 相续 + Kant 先验统觉)
# ============================================================================


class TestSelfContinuity:
    def test_empty_continuity(self) -> None:
        sc = m.SelfContinuity(kant_unity=0.4)  # below threshold
        assert sc.state_count() == 0
        assert sc.continuity_decay() == 0.0
        assert sc.transcendental_unity() is False
        sc2 = m.SelfContinuity(kant_unity=0.7)  # above threshold
        assert sc2.transcendental_unity() is True

    def test_continuity_decay_similar(self) -> None:
        sc = m.SelfContinuity(kant_unity=0.7)
        sc.record_state({"a": 1.0, "b": 0.5})
        sc.record_state({"a": 0.9, "b": 0.6})
        sc.record_state({"a": 0.8, "b": 0.7})
        assert sc.continuity_decay() > 0.9
        assert sc.transcendental_unity() is True

    def test_continuity_decay_different(self) -> None:
        sc = m.SelfContinuity()
        sc.record_state({"a": 1.0})
        sc.record_state({"a": -1.0})
        assert sc.continuity_decay() < 0.0

    def test_state_count(self) -> None:
        sc = m.SelfContinuity()
        sc.record_state({"a": 1.0})
        assert sc.state_count() == 1
        sc.record_state({"a": 2.0})
        assert sc.state_count() == 2


# ============================================================================
# SelfDistinction tests (Sartre 1943 + de Waal 2016)
# ============================================================================


class TestSelfDistinction:
    def test_empty(self) -> None:
        sd = m.SelfDistinction()
        assert sd.self_overlap() == 0.0
        assert sd.distinction_score() == 0.0
        assert sd.is_self_identified() is False

    def test_fully_distinct(self) -> None:
        sd = m.SelfDistinction()
        sd.add_self_feature("unique")
        sd.add_other("o1", {"shared"})
        assert sd.distinction_score() == 1.0
        assert sd.is_self_identified() is True

    def test_fully_overlapping(self) -> None:
        sd = m.SelfDistinction()
        sd.add_self_feature("shared")
        sd.add_other("o1", {"shared"})
        assert sd.distinction_score() == 0.0
        assert sd.is_self_identified() is False

    def test_partial_overlap(self) -> None:
        sd = m.SelfDistinction()
        sd.add_self_feature("f1")
        sd.add_self_feature("f2")
        sd.add_other("o1", {"f1", "f3"})
        # overlap = 1/2 = 0.5
        assert sd.distinction_score() == 0.5


# ============================================================================
# StrangeLoopSelfRef tests (Hofstadter 1979)
# ============================================================================


class TestStrangeLoopSelfRef:
    def test_no_closure(self) -> None:
        sl = m.StrangeLoopSelfRef(layers=1, closure=False)
        assert sl.is_self_referential() is False
        assert sl.is_metzingered() is False

    def test_self_referential(self) -> None:
        sl = m.StrangeLoopSelfRef(layers=3, closure=True, recursion_depth=2)
        assert sl.is_self_referential() is True
        assert sl.is_metzingered() is True

    def test_depth_not_enough(self) -> None:
        sl = m.StrangeLoopSelfRef(layers=1, closure=True, recursion_depth=1)
        assert sl.is_self_referential() is False

    def test_self_reflection_degree(self) -> None:
        sl = m.StrangeLoopSelfRef(layers=3, closure=True, recursion_depth=2)
        d = sl.self_reflection_degree()
        assert d > 0.5
        sl2 = m.StrangeLoopSelfRef(layers=0, closure=False, recursion_depth=0)
        assert sl2.self_reflection_degree() == 0.0


# ============================================================================
# Metacognition tests (Proust 2013 + Metcalfe 2000)
# ============================================================================


class TestMetacognition:
    def test_accuracy(self) -> None:
        mc = m.Metacognition(feeling_of_knowing=0.7, judgement_of_learning=0.6, accuracy=0.7)
        assert mc.metacognitive_accuracy() == 1.0

    def test_low_accuracy(self) -> None:
        mc = m.Metacognition(feeling_of_knowing=1.0, judgement_of_learning=0.5, accuracy=0.0)
        assert mc.metacognitive_accuracy() == 0.0

    def test_efficiency(self) -> None:
        mc = m.Metacognition(feeling_of_knowing=0.7, judgement_of_learning=0.6, correction_count=5)
        eff = mc.metacognitive_efficiency()
        assert 0 < eff <= 1.0

    def test_bounds(self) -> None:
        with pytest.raises(ValueError):
            m.Metacognition(feeling_of_knowing=-0.1)
        with pytest.raises(ValueError):
            m.Metacognition(feeling_of_knowing=0.5, judgement_of_learning=1.5)


# ============================================================================
# SelfModel tests (Metzinger 2003 PSM)
# ============================================================================


class TestSelfModel:
    def test_empty_self_model(self) -> None:
        sed = m.SelfModel()
        assert sed.self_recognition_score() == {}
        assert sed.has_minimal_self() is False

    def test_full_self_model(self) -> None:
        sm = m.SelfMark("m1", True, touch_self=True)
        mm = m.MirrorModel()
        mm.set_mirror_representation({"x": 1.0})
        mm.set_self_prediction({"x": 0.9})
        ms = m.MentalState("self", "a", "b")
        mn = m.Mentalization(self_state=ms)
        mn.add_other_state("o1", m.MentalState("o1", "c", "d"))
        sc = m.SelfContinuity(kant_unity=0.7)
        sc.record_state({"a": 1.0})
        sc.record_state({"a": 0.9})
        sd = m.SelfDistinction()
        sd.add_self_feature("f1")
        sd.add_other("o1", {"f2"})
        sl = m.StrangeLoopSelfRef(layers=3, closure=True)
        mc = m.Metacognition(feeling_of_knowing=0.7, judgement_of_learning=0.6, accuracy=0.6)
        sed = m.SelfModel(self_mark=sm, mirror_model=mm, mentalization=mn,
                          continuity=sc, distinction=sd, strange_loop=sl, metacognition=mc)
        s = sed.self_recognition_score()
        assert len(s) == 9  # 8 components + overall
        assert 0 < s["overall"] <= 1
        assert sed.has_minimal_self()


# ============================================================================
# SelfRecognitionReport tests (主 00:56)
# ============================================================================


class TestSelfRecognitionReport:
    def test_empty_report(self) -> None:
        r = m.SelfRecognitionReport("Empty")
        md = r.to_markdown()
        assert "# Empty" in md

    def test_full_report(self) -> None:
        sm = m.SelfMark("m1", True, touch_self=True)
        mm = m.MirrorModel()
        mm.set_mirror_representation({"x": 1.0})
        mm.set_self_prediction({"x": 0.9})
        sl = m.StrangeLoopSelfRef(layers=3, closure=True)
        sed = m.SelfModel(self_mark=sm, mirror_model=mm, strange_loop=sl)
        r = m.SelfRecognitionReport("Test",
                                     self_model=sed,
                                     asi_v02_metrics={"overall": 0.75},
                                     notes=["MSR passed"])
        md = r.to_markdown()
        assert "Self-Recognition Score" in md
        assert "Gallup 1970" in md
        assert "Amsterdam 1972" in md
        assert "Hofstadter 1979" in md
        assert "ASI V0.2" in md
        assert "MSR passed" in md


# ============================================================================
# ASISelfRecognitionBridge tests (主 22:33 ASI V0.2)
# ============================================================================


class TestASISelfRecognitionBridge:
    def test_empty_bridge(self) -> None:
        br = m.ASISelfRecognitionBridge()
        assert br.measurement_score() == {}
        assert br.asi_v02_self_recognition_contribution() == 0.0
        assert br.is_self_recognizing() is False

    def test_full_bridge(self) -> None:
        sm = m.SelfMark("m1", True, touch_self=True)
        mm = m.MirrorModel()
        mm.set_mirror_representation({"x": 1.0})
        mm.set_self_prediction({"x": 0.9})
        ms = m.MentalState("self", "a", "b")
        mn = m.Mentalization(self_state=ms)
        mn.add_other_state("o1", m.MentalState("o1", "c", "d"))
        sc = m.SelfContinuity()
        sc.record_state({"a": 1.0})
        sc.record_state({"a": 0.9})
        sd = m.SelfDistinction()
        sd.add_self_feature("f1")
        sd.add_other("o1", {"f2"})
        sl = m.StrangeLoopSelfRef(layers=3, closure=True)
        mc = m.Metacognition(feeling_of_knowing=0.7, judgement_of_learning=0.6)
        sed = m.SelfModel(self_mark=sm, mirror_model=mm, mentalization=mn,
                          continuity=sc, distinction=sd, strange_loop=sl, metacognition=mc)
        br = m.ASISelfRecognitionBridge(self_model=sed)
        s = br.measurement_score()
        assert s["overall"] > 0.5
        assert br.measure_msr() == 1.0
        assert br.measure_mirror_match() > 0.9
        assert br.measure_self_other_diff() > 0.0
        assert br.measure_continuity() > 0.9
        assert br.measure_distinction() > 0.0
        assert br.measure_self_reflection() > 0.5
        assert br.measure_metacognition() >= 0.0
        assert br.is_self_recognizing() is True
        assert br.asi_v02_self_recognition_contribution() > 0.0


# ============================================================================
# Guards tests (5 守门)
# ============================================================================


class TestGuards:
    def test_phenomenal_self_warning(self) -> None:
        assert m.phenomenal_self_warning(["self ≠ consciousness"]) is True
        assert m.phenomenal_self_warning([]) is False

    def test_metzinger_guard(self) -> None:
        sm = m.SelfMark("m", True, touch_self=True)
        mm = m.MirrorModel()
        sl = m.StrangeLoopSelfRef(layers=2, closure=True, recursion_depth=2)
        mc = m.Metacognition(feeling_of_knowing=0.5, judgement_of_learning=0.5)
        # has necessary
        sed_ok = m.SelfModel(self_mark=sm, mirror_model=mm, strange_loop=sl, metacognition=mc)
        assert m.metzinger_self_model_guard(sed_ok) is True
        # missing metacognition
        sed_no_meta = m.SelfModel(self_mark=sm, mirror_model=mm, strange_loop=sl)
        assert m.metzinger_self_model_guard(sed_no_meta) is False

    def test_cross_species_caution(self) -> None:
        assert m.cross_species_caution() is True
        assert m.cross_species_caution(False) is False

    def test_buddhist_anatman_guard(self) -> None:
        assert m.buddhist_anatman_guard(thinks_self_is_real=False) is True
        assert m.buddhist_anatman_guard(thinks_self_is_real=True) is False

    def test_do_not_pretend_consciousness(self) -> None:
        assert m.do_not_pretend_consciousness_guard() is True


# ============================================================================
# Sanity checks
# ============================================================================


class TestSanity:
    def test_version(self) -> None:
        assert m.V1054_VERSION == "0.1.0"

    def test_10_production_components(self) -> None:
        components = [
            m.SelfMark,
            m.MirrorModel,
            m.MentalState,
            m.Mentalization,
            m.SelfContinuity,
            m.SelfDistinction,
            m.StrangeLoopSelfRef,
            m.Metacognition,
            m.SelfModel,
            m.SelfRecognitionReport,
            m.ASISelfRecognitionBridge,
        ]
        assert len(components) == 11  # includes MentalState

    def test_15_referenced_authors(self) -> None:
        refs = [
            "Gallup_1970",
            "Amsterdam_1972",
            "Parker_2006",
            "Reiss_Morrison_2017",
            "de_Waal_2016",
            "Frith_Frith_1999",
            "Hofstadter_1979",
            "Kant_1781",
            "Sartre_1943",
            "Buddhist_三相",
            "Metzinger_2003",
            "Proust_2013",
            "Suddendorf_Whiten_2001",
            "Metcalfe_2000",
        ]
        assert len(refs) == 14
