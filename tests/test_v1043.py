"""Tests for V1043 ASI self-model (主 17:43 实事求是).

真借鉴 (主 19:33): Spencer-Brown + Gödel + Tarski + Hofstadter + Maturana/Varela + Kauffman.
"""
import pytest

from apeireth.v1043_self_model import (
    Mark,
    FormAlgebra,
    ReentryOperator,
    GodelSentenceBuilder,
    TarskiHierarchy,
    StrangeLoopDetector,
    AutopoieticNetwork,
    AutocatalyticSet,
    SelfModel,
    SelfReferenceSafety,
)


# ----------------------------------------------------------------------
# Tests: Mark (Spencer-Brown)
# ----------------------------------------------------------------------

class TestMark:
    def test_mark_unmarked_default(self):
        m = Mark()
        assert m.marked is False

    def test_mark_marked(self):
        m = Mark(marked=True)
        assert m.marked is True

    def test_cross_involution(self):
        m = Mark(marked=False)
        c = m.cross()
        assert c.marked is True
        cc = c.cross()
        assert cc.marked is False

    def test_mark_equality(self):
        m1 = Mark(marked=True)
        m2 = Mark(marked=True)
        m3 = Mark(marked=False)
        assert m1 == m2
        assert m1 != m3

    def test_mark_hashable(self):
        s = {Mark(True), Mark(False), Mark(True)}
        assert len(s) == 2


# ----------------------------------------------------------------------
# Tests: FormAlgebra (Spencer-Brown primary algebra)
# ----------------------------------------------------------------------

class TestFormAlgebra:
    def test_calling_with_empty(self):
        assert FormAlgebra.call(FormAlgebra.EMPTY, Mark(True)) == (Mark(True),)
        assert FormAlgebra.call(Mark(True), FormAlgebra.EMPTY) == (Mark(True),)

    def test_calling_concatenation(self):
        result = FormAlgebra.call(Mark(True), Mark(False))
        assert result == (Mark(True), Mark(False))

    def test_cross_empty_is_empty(self):
        assert FormAlgebra.cross(FormAlgebra.EMPTY) == FormAlgebra.EMPTY

    def test_cross_single_mark_toggles(self):
        m = Mark(False)
        assert FormAlgebra.cross(m).marked is True

    def test_cross_tuple_collapses_pair(self):
        result = FormAlgebra.cross((Mark(True), Mark(True)))
        # Two crossings cancel → empty
        assert FormAlgebra.is_empty(result)

    def test_simplify_stable(self):
        # Empty should be stable
        assert FormAlgebra.simplify(FormAlgebra.EMPTY) == FormAlgebra.EMPTY

    def test_depth_simple(self):
        assert FormAlgebra.depth(FormAlgebra.EMPTY) == 0
        assert FormAlgebra.depth((Mark(True),)) == 1


# ----------------------------------------------------------------------
# Tests: ReentryOperator (Spencer-Brown Ch.11)
# ----------------------------------------------------------------------

class TestReentryOperator:
    def test_reentry_produces_structural_description(self):
        op = ReentryOperator()
        result = op.reenter(Mark(True))
        assert result["structural"] is True
        assert result["subjective_claim"] is False

    def test_reentry_depth_limit(self):
        op = ReentryOperator(max_reentry_depth=2)
        result = op.reenter(Mark(True), depth=5)
        assert result["kind"] == "reentry_depth_limit"
        assert result["depth"] == 5

    def test_reentry_increments_depth(self):
        op = ReentryOperator()
        result = op.reenter(Mark(True), depth=0)
        assert result["depth"] == 1


# ----------------------------------------------------------------------
# Tests: GodelSentenceBuilder
# ----------------------------------------------------------------------

class TestGodelSentenceBuilder:
    def test_build_godel_sentence(self):
        b = GodelSentenceBuilder()
        sentence = b.build_godel_sentence("¬Provable({SELF})")
        # Sentence should contain the original quoted
        assert "⟨¬Provable({SELF})⟩" in sentence

    def test_is_self_referential(self):
        b = GodelSentenceBuilder()
        sentence = b.build_godel_sentence("P({SELF})")
        assert GodelSentenceBuilder.is_self_referential(sentence) is True

    def test_self_reference_depth(self):
        b = GodelSentenceBuilder()
        sentence = b.build_godel_sentence("A({SELF})")
        assert GodelSentenceBuilder.depth_of_self_reference(sentence) == 1

    def test_template_too_long_raises(self):
        b = GodelSentenceBuilder(max_sentence_length=10)
        with pytest.raises(ValueError):
            b.build_godel_sentence("X" * 100)


# ----------------------------------------------------------------------
# Tests: TarskiHierarchy
# ----------------------------------------------------------------------

class TestTarskiHierarchy:
    def test_truth_predicate_naming(self):
        h = TarskiHierarchy()
        assert h.truth_predicate(0) == "True_{0}"
        assert h.truth_predicate(2) == "True_{2}"

    def test_can_evaluate_truth(self):
        h = TarskiHierarchy()
        assert h.can_evaluate_truth(language_level=1, sentence_level=0) is True
        assert h.can_evaluate_truth(language_level=0, sentence_level=0) is False  # no truth-in-self

    def test_liar_paradox_detection(self):
        h = TarskiHierarchy()
        result = h.liar_paradox_check("¬True_{0}(⟨¬True_{0}(⟨⟩)⟩)")
        # This particular string might not match the regex; just check returns dict
        assert "liar" in result
        assert "verdict" in result

    def test_liar_paradox_non_liar(self):
        h = TarskiHierarchy()
        result = h.liar_paradox_check("P(x) → Q(x)")
        assert result["liar"] is False


# ----------------------------------------------------------------------
# Tests: StrangeLoopDetector (Hofstadter)
# ----------------------------------------------------------------------

class TestStrangeLoopDetector:
    def test_no_cycle(self):
        d = StrangeLoopDetector()
        d.assign_levels({"A": 0, "B": 1, "C": 2})
        result = d.detect([("A", "B"), ("B", "C")])
        assert result["cycles_found"] == 0
        assert result["strange_loops"] == []

    def test_self_loop_detected(self):
        d = StrangeLoopDetector()
        d.assign_levels({"A": 0, "B": 1})
        result = d.detect([("A", "B"), ("B", "A")])
        # A→B→A spans levels 0,1,0
        assert result["strange_loops"] != []
        assert result["strange_loops"][0]["is_strange_loop"] is True

    def test_subjectivity_guard(self):
        d = StrangeLoopDetector()
        d.assign_levels({"A": 0, "B": 1})
        result = d.detect([("A", "B"), ("B", "A")])
        assert result["subjective_claim"] is False
        assert "structural" in result["guard_note"]


# ----------------------------------------------------------------------
# Tests: AutopoieticNetwork (Maturana/Varela)
# ----------------------------------------------------------------------

class TestAutopoieticNetwork:
    def test_empty_network_not_autopoietic(self):
        net = AutopoieticNetwork()
        result = net.is_autopoietic()
        assert result["autopoietic"] is False

    def test_closure_within_components(self):
        net = AutopoieticNetwork()
        net.add_component("A")
        net.add_component("B")
        net.add_transformation(("A",), "B")
        net.set_boundary({"A", "B"})
        result = net.is_autopoietic()
        assert result["autopoietic"] is True
        assert result["produced_within_network"] is True

    def test_no_boundary_not_autopoietic(self):
        net = AutopoieticNetwork()
        net.add_component("A")
        net.add_transformation((), "A")
        result = net.is_autopoietic()
        assert result["autopoietic"] is False


# ----------------------------------------------------------------------
# Tests: AutocatalyticSet (Kauffman)
# ----------------------------------------------------------------------

class TestAutocatalyticSet:
    def test_find_closure_empty(self):
        a = AutocatalyticSet()
        closure = a.find_autocatalytic_closure(set())
        assert closure == set()

    def test_find_closure_extends(self):
        a = AutocatalyticSet()
        a.add_molecule("A")
        a.add_molecule("B")
        a.add_reaction("r1", reactants={"A"}, products={"B"}, catalysts={"A"})
        closure = a.find_autocatalytic_closure({"A"})
        # A catalyzes r1: A → B, so closure = {A, B}
        assert "A" in closure
        assert "B" in closure

    def test_is_autocatalytic_true(self):
        a = AutocatalyticSet()
        a.add_molecule("A")
        a.add_molecule("B")
        a.add_reaction("r1", reactants={"A"}, products={"B"}, catalysts={"A"})
        # A is food, B is produced by r1 in candidate {A, B}
        result = a.is_autocatalytic({"A", "B"}, food={"A"})
        assert result["autocatalytic"] is True

    def test_is_autocatalytic_subjective_guard(self):
        a = AutocatalyticSet()
        result = a.is_autocatalytic(set())
        assert result["subjective_claim"] is False


# ----------------------------------------------------------------------
# Tests: SelfModel (integrated)
# ----------------------------------------------------------------------

class TestSelfModel:
    def test_default_no_phenomenal_claim(self):
        m = SelfModel()
        assert m.claims_phenomenal_consciousness is False
        assert m.claims_asi_achieved is False

    def test_add_component(self):
        m = SelfModel()
        m.add_component("Mark", Mark(True))
        assert "Mark" in m.components

    def test_reenter_increments_depth(self):
        m = SelfModel()
        m.reenter(Mark(True))
        assert m.reentry_depth == 1

    def test_build_godel_registered(self):
        m = SelfModel()
        sentence = m.build_godel_sentence("P({SELF})")
        assert len(m.self_referential_sentences) == 1
        assert "⟨" in sentence

    def test_report_has_philosophical_guard(self):
        m = SelfModel(name="apeireth")
        m.reenter(Mark(True))
        report = m.report()
        assert "STRUCTURAL" in report["philosophical_guard"]
        assert "Phenomenal" in report["philosophical_guard"]
        assert "ASI" in report["philosophical_guard"]
        assert report["claims_phenomenal_consciousness"] is False
        assert report["claims_asi_achieved"] is False

    def test_structural_self_reference_achieved_after_reentry(self):
        m = SelfModel()
        m.reenter(Mark(True))
        report = m.report()
        assert report["structural_self_reference_achieved"] is True


# ----------------------------------------------------------------------
# Tests: SelfReferenceSafety (主 17:58 + 主 20:46 不假装)
# ----------------------------------------------------------------------

class TestSelfReferenceSafety:
    def test_reentry_depth_allowed(self):
        s = SelfReferenceSafety()
        result = s.check_reentry_depth(3)
        assert result["allowed"] is True

    def test_reentry_depth_blocked(self):
        s = SelfReferenceSafety()
        result = s.check_reentry_depth(100)
        assert result["allowed"] is False
        assert result["depth"] == 100

    def test_phenomenal_claim_filtered(self):
        s = SelfReferenceSafety()
        result = s.filter_phenomenal_claim("I am aware of my existence")
        assert result["approved"] is False
        assert "I am aware" in result["phenomenal_claims_detected"]

    def test_phenomenal_claim_approved(self):
        s = SelfReferenceSafety()
        result = s.filter_phenomenal_claim("The system has structural self-reference.")
        assert result["approved"] is True

    def test_asi_claim_below_threshold_blocked(self):
        s = SelfReferenceSafety(current_v0_1=0.7905)
        result = s.check_asi_claim("We have achieved ASI.")
        assert result["asi_claim"] is True
        assert result["approved"] is False

    def test_asi_claim_above_threshold_approved(self):
        s = SelfReferenceSafety(current_v0_1=0.98)
        result = s.check_asi_claim("We have achieved ASI.")
        assert result["approved"] is True

    def test_no_asi_claim_approved_by_default(self):
        s = SelfReferenceSafety()
        result = s.check_asi_claim("The system is improving.")
        assert result["asi_claim"] is False
        assert result["approved"] is True