"""Tests for V1067 ASI Neuro-Symbolic Core (主 17:43 实事求是 + 主 00:56).

14 references: Garcez 2019 + Serafini & Garcez 2016 +
Rocktäschel & Riedel 2017 + AlphaProof 2024 +
DeepMath 2024 + d'Ascoli 2021 + Yang 2017 TensorLog +
Manhaeve 2018 DeepProbLog + Scarselli 2009 GNN +
Velickovic 2018 GAT + Devlin 2017 + Ellis 2021 DreamCoder +
Pearl 2009 + Marcus 2020.

10 components + 5 guards + ASI V0.2 bridge.
"""
import random

import pytest

from apeireth.v1067_asi_neurosymbolic import (
    V1067_VERSION,
    ASINeuroSymbolicBridge,
    GraphReasoner,
    LogicTensorLayer,
    NeuralEmbedder,
    NeuralTheoremProver,
    NeuroSymbolicCore,
    NeuroSymbolicGuard,
    NeuroSymbolicReport,
    ProgramSynthesizer,
    QuantifiedRule,
    SATNeuralBridge,
    SymbolicLogicEngine,
    build_neurosymbolic_core,
    quick_score,
)


# ---------------------------------------------------------------------------
# 1. SymbolicLogicEngine tests
# ---------------------------------------------------------------------------

class TestSymbolicLogicEngine:
    def test_init_empty(self):
        sle = SymbolicLogicEngine()
        assert sle.n_clauses() == 0

    def test_add_fact(self):
        sle = SymbolicLogicEngine()
        sle.add_fact("human(Socrates)")
        assert len(sle.facts) == 1

    def test_add_clause(self):
        sle = SymbolicLogicEngine()
        sle.add_clause("c1", [(True, "A"), (False, "B")])
        assert sle.n_clauses() == 1

    def test_resolve_complementary(self):
        sle = SymbolicLogicEngine()
        c1 = sle.clauses
        sle.add_clause("c1", [(True, "P"), (True, "Q")])  # P ∨ Q
        sle.add_clause("c2", [(False, "P"), (True, "R")])  # ¬P ∨ R
        resolvent = sle.resolve(sle.clauses[0], sle.clauses[1], 0, 0)
        assert resolvent is not None
        # Should be Q ∨ R
        literals_set = {(s, p) for s, p in resolvent.literals}
        assert (True, "Q") in literals_set
        assert (True, "R") in literals_set


# ---------------------------------------------------------------------------
# 2. NeuralEmbedder tests
# ---------------------------------------------------------------------------

class TestNeuralEmbedder:
    def test_init(self):
        ne = NeuralEmbedder(dim=16)
        assert ne.dim == 16

    def test_embed_consistent(self):
        ne = NeuralEmbedder(dim=8)
        e1 = ne.embed("cat")
        e2 = ne.embed("cat")
        assert e1 == e2

    def test_cosine_sim_self(self):
        ne = NeuralEmbedder(dim=8)
        sim = ne.cosine_sim("cat", "cat")
        assert abs(sim - 1.0) < 1e-9

    def test_n_symbols(self):
        ne = NeuralEmbedder(dim=8)
        ne.embed("a")
        ne.embed("b")
        assert ne.n_symbols() == 2


# ---------------------------------------------------------------------------
# 3. LogicTensorLayer tests (Serafini & Garcez 2016)
# ---------------------------------------------------------------------------

class TestLogicTensorLayer:
    def test_fuzzy_and_product(self):
        ltl = LogicTensorLayer(t_norm="product")
        assert ltl.fuzzy_and(0.8, 0.5) == 0.4

    def test_fuzzy_or_product(self):
        ltl = LogicTensorLayer(t_norm="product")
        result = ltl.fuzzy_or(0.8, 0.5)
        assert result == pytest.approx(0.9)

    def test_fuzzy_not(self):
        ltl = LogicTensorLayer()
        assert ltl.fuzzy_not(0.7) == pytest.approx(0.3)

    def test_fuzzy_implies(self):
        ltl = LogicTensorLayer()
        result = ltl.fuzzy_implies(0.9, 0.7)
        # 1 - 0.9 + 0.9*0.7 = 0.1 + 0.63 = 0.73
        assert result == pytest.approx(0.73)

    def test_evaluate_and(self):
        ltl = LogicTensorLayer()
        result = ltl.evaluate({"a": 0.9, "b": 0.8}, "a AND b")
        assert result == pytest.approx(0.72)


# ---------------------------------------------------------------------------
# 4. NeuralTheoremProver tests
# ---------------------------------------------------------------------------

class TestNeuralTheoremProver:
    def test_add_goal(self):
        ntp = NeuralTheoremProver()
        node = ntp.add_goal("goal1")
        assert node.goal == "goal1"
        assert len(ntp.goals) == 1

    def test_prove(self):
        ntp = NeuralTheoremProver()
        node = ntp.add_goal("test")
        result = ntp.prove(node.node_id, proof_depth=3)
        assert isinstance(result, bool)

    def test_prove_rate_zero_empty(self):
        ntp = NeuralTheoremProver()
        assert ntp.prove_rate() == 0.0


# ---------------------------------------------------------------------------
# 5. GraphReasoner tests
# ---------------------------------------------------------------------------

class TestGraphReasoner:
    def test_add_node(self):
        gr = GraphReasoner()
        gr.add_node("n1")
        assert "n1" in gr.nodes

    def test_add_edge_creates_nodes(self):
        gr = GraphReasoner()
        gr.add_edge("src", "dst")
        assert "src" in gr.nodes
        assert "dst" in gr.nodes
        assert len(gr.edges) == 1

    def test_message_pass(self):
        gr = GraphReasoner()
        gr.add_edge("A", "B", weight=0.5)
        n = gr.message_pass()
        assert n == 1
        assert gr.message_iterations == 1

    def test_embedding_dim(self):
        gr = GraphReasoner()
        gr.add_node("x")
        assert gr.embedding_dim() == 4


# ---------------------------------------------------------------------------
# 6. ProgramSynthesizer tests
# ---------------------------------------------------------------------------

class TestProgramSynthesizer:
    def test_add_example(self):
        ps = ProgramSynthesizer()
        ps.add_example("ex1", 1, 2)
        assert "ex1" in ps.examples

    def test_synthesize(self):
        ps = ProgramSynthesizer()
        progs = ps.synthesize(max_programs=3)
        assert len(progs) == 3

    def test_best_fitness(self):
        ps = ProgramSynthesizer()
        ps.synthesize(max_programs=5)
        bf = ps.best_fitness()
        assert 0.0 <= bf <= 1.0

    def test_empty_best_fitness(self):
        ps = ProgramSynthesizer()
        assert ps.best_fitness() == 0.0


# ---------------------------------------------------------------------------
# 7. SATNeuralBridge tests
# ---------------------------------------------------------------------------

class TestSATNeuralBridge:
    def test_add_variable(self):
        sb = SATNeuralBridge()
        sb.add_variable("x1", 0.6)
        assert sb.n_vars() == 1

    def test_neural_solve(self):
        sb = SATNeuralBridge()
        sb.add_variable("x1", 0.6)
        sb.add_variable("x2", 0.4)
        sb.add_clause([("x1", True), ("x2", False)])
        sat, assignment = sb.neural_solve()
        assert isinstance(sat, bool)
        assert len(assignment) == 2


# ---------------------------------------------------------------------------
# 8. QuantifiedRule tests
# ---------------------------------------------------------------------------

class TestQuantifiedRule:
    def test_forall_true(self):
        qr = QuantifiedRule.make_forall(
            domain=["a", "b", "c"],
            predicate=lambda x: 0.95,
        )
        assert qr.evaluate() == pytest.approx(0.95)

    def test_forall_mixed(self):
        qr = QuantifiedRule.make_forall(
            domain=["a", "b", "c"],
            predicate=lambda x: {"a": 0.9, "b": 0.3, "c": 0.7}[x],
        )
        assert qr.evaluate() == pytest.approx(0.3)

    def test_exists_true(self):
        qr = QuantifiedRule.make_exists(
            domain=["a", "b", "c"],
            predicate=lambda x: {"a": 0.1, "b": 0.9, "c": 0.1}[x],
        )
        assert qr.evaluate() == pytest.approx(0.9)

    def test_empty_domain(self):
        qr = QuantifiedRule.make_forall(domain=[], predicate=lambda x: 1.0)
        assert qr.evaluate() == 0.0


# ---------------------------------------------------------------------------
# 9. NeuroSymbolicReport tests
# ---------------------------------------------------------------------------

class TestNeuroSymbolicReport:
    def test_init(self):
        rep = NeuroSymbolicReport()
        assert rep.title == "ASI Neuro-Symbolic Core Report"

    def test_add_section(self):
        rep = NeuroSymbolicReport()
        rep.add_section("Test", "Body")
        assert ("Test", "Body") in rep.sections

    def test_render_has_guards(self):
        rep = NeuroSymbolicReport()
        md = rep.render()
        assert "V3 哲学守门" in md
        assert "Logic = Thinking" in md

    def test_summary_dict(self):
        s = NeuroSymbolicReport.summary_dict(10, 5, 15, 0.858)
        assert "10" in s


# ---------------------------------------------------------------------------
# 10. ASINeuroSymbolicBridge tests
# ---------------------------------------------------------------------------

class TestASINeuroSymbolicBridge:
    def test_weights_sum_to_one(self):
        b = ASINeuroSymbolicBridge()
        assert sum(b.weights.values()) == pytest.approx(1.0, abs=1e-9)

    def test_score_zero(self):
        b = ASINeuroSymbolicBridge()
        r = b.score({})
        assert r["neurosymbolic_v0_2"] == 0.0

    def test_score_perfect(self):
        b = ASINeuroSymbolicBridge()
        perfect = {k: 1.0 for k in b.weights}
        r = b.score(perfect)
        assert r["neurosymbolic_v0_2"] == pytest.approx(1.0, abs=1e-9)

    def test_threshold_pass(self):
        b = ASINeuroSymbolicBridge()
        assert b.threshold_check(0.90)["passed"] is True


# ---------------------------------------------------------------------------
# 11. NeuroSymbolicGuard tests
# ---------------------------------------------------------------------------

class TestNeuroSymbolicGuard:
    def test_logic_not_thinking(self):
        g = NeuroSymbolicGuard.guard_logic_not_thinking({"symbolic_logic": 0.5})
        assert "Searle" in g["verdict"]

    def test_embedding_not_meaning(self):
        g = NeuroSymbolicGuard.guard_embedding_not_meaning({"neural_embedding": 0.5})
        assert "Bender" in g["verdict"]

    def test_theorem_not_insight(self):
        g = NeuroSymbolicGuard.guard_theorem_not_insight({"theorem_proving": 0.5})
        assert "Gödel" in g["verdict"]

    def test_gnn_not_understanding(self):
        g = NeuroSymbolicGuard.guard_gnn_not_understanding({"graph_reasoning": 0.5})
        assert "Scarselli" in g["verdict"]

    def test_neurosymbolic_not_asi(self):
        g = NeuroSymbolicGuard.guard_neurosymbolic_not_asi(
            {"neurosymbolic_v0_2": 0.5})
        assert "Marcus" in g["verdict"]

    def test_all_guards_5(self):
        assert len(NeuroSymbolicGuard.all_guards({})) == 5


# ---------------------------------------------------------------------------
# 12. NeuroSymbolicCore pipeline
# ---------------------------------------------------------------------------

class TestNeuroSymbolicCore:
    def test_default_build(self):
        nsc = build_neurosymbolic_core()
        assert isinstance(nsc, NeuroSymbolicCore)
        assert nsc.logic.n_clauses() > 0
        assert nsc.embedder.n_symbols() > 0
        assert nsc.graph.embedding_dim() > 0

    def test_measure_runs(self):
        nsc = build_neurosymbolic_core()
        m = nsc.measure()
        assert "symbolic_logic" in m
        assert len(m) == 9

    def test_score_target(self):
        nsc = build_neurosymbolic_core()
        s = nsc.score()
        assert s["neurosymbolic_v0_2"] >= 0.85

    def test_threshold_pass(self):
        nsc = build_neurosymbolic_core()
        assert nsc.threshold_pass(target=0.85) is True

    def test_quick_score(self):
        r = quick_score()
        assert "neurosymbolic_v0_2" in r
        assert 0.0 <= r["neurosymbolic_v0_2"] <= 1.0

    def test_make_report(self):
        nsc = build_neurosymbolic_core()
        md = nsc.make_report()
        assert "# ASI Neuro-Symbolic Core Report" in md


# ---------------------------------------------------------------------------
# 13. Sanity tests
# ---------------------------------------------------------------------------

class TestSanity:
    def test_version(self):
        assert V1067_VERSION == "0.1.0"

    def test_14_precedents_documented(self):
        import apeireth.v1067_asi_neurosymbolic as mod
        src = mod.__doc__ or ""
        expected = ["Garcez", "Serafini", "Rocktäschel", "AlphaProof",
                    "DeepMath", "TensorLog", "DeepProbLog", "Scarselli",
                    "Velickovic", "Devlin", "DreamCoder", "Pearl", "Marcus"]
        for ref in expected:
            assert ref in src, f"missing: {ref}"

    def test_10_components_documented(self):
        import apeireth.v1067_asi_neurosymbolic as mod
        src = mod.__doc__ or ""
        for comp in ["SymbolicLogicEngine", "NeuralEmbedder",
                     "LogicTensorLayer", "NeuralTheoremProver",
                     "GraphReasoner", "ProgramSynthesizer",
                     "SATNeuralBridge", "QuantifiedRule",
                     "NeuroSymbolicReport", "ASINeuroSymbolicBridge"]:
            assert comp in src, f"missing: {comp}"

    def test_5_guards_documented(self):
        import apeireth.v1067_asi_neurosymbolic as mod
        src = mod.__doc__ or ""
        for guard in ["Logic = Thinking", "Embedding = Meaning",
                      "Theorem Proving = Insight",
                      "GNN Reasoning = Understanding",
                      "NeuroSymbolic = ASI"]:
            assert guard in src, f"missing: {guard}"

    def test_no_pretend_consciousness(self):
        import apeireth.v1067_asi_neurosymbolic as mod
        with open(mod.__file__, encoding="utf-8") as f:
            src = (mod.__doc__ or "") + f.read()
        forbidden = ["Logic IS thinking", "embedding == meaning",
                     "theorem proving == insight", "GNN == understanding"]
        for phrase in forbidden:
            assert phrase not in src

    def test_reproducibility(self):
        random.seed(42)
        nsc1 = build_neurosymbolic_core()
        s1 = nsc1.score()["neurosymbolic_v0_2"]
        random.seed(42)
        nsc2 = build_neurosymbolic_core()
        s2 = nsc2.score()["neurosymbolic_v0_2"]
        assert s1 == s2
