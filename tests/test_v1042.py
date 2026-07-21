"""V1042 真生产 tests (主 00:36 质量 + 主 22:33 + 主 19:33 + 主 17:43)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import random
import pytest
from apeireth.v1042_causal_reasoning import (
    V1042_VERSION,
    CausalNode,
    CausalEdge,
    CausalDAG,
    StructuralCausalModel,
    DoOperator,
    BackdoorCriterion,
    InstrumentalVariableEstimator,
    CounterfactualEngine,
    Refuter,
    CausalEstimator,
    CausalReport,
    V1042CausalReasoning,
)


# ----------------------------------------------------------------------
# Helper: classic confounding DAG Z -> X, Z -> Y, X -> Y
# ----------------------------------------------------------------------

def make_confounding_dag() -> CausalDAG:
    return CausalDAG(
        nodes=["Z", "X", "Y"],
        edges=[("Z", "X"), ("Z", "Y"), ("X", "Y")],
    )


def make_confounding_scm() -> StructuralCausalModel:
    dag = make_confounding_dag()

    def f_x(p: dict, u: float) -> float:
        return 0.5 * p["Z"] + u

    def f_y(p: dict, u: float) -> float:
        return 2.0 * p["X"] + 1.5 * p["Z"] + u

    def f_z(p: dict, u: float) -> float:
        return u

    return StructuralCausalModel(
        dag=dag,
        equations={"Z": f_z, "X": f_x, "Y": f_y},
        noise_std={"Z": 1.0, "X": 0.5, "Y": 0.5},
    )


# ----------------------------------------------------------------------
# Tests: CausalNode / CausalEdge
# ----------------------------------------------------------------------

class TestCausalNode:
    def test_valid_node(self):
        n = CausalNode("X")
        assert n.name == "X"

    def test_empty_name_raises(self):
        with pytest.raises(ValueError):
            CausalNode("")

    def test_node_equality(self):
        assert CausalNode("X") == CausalNode("X")


class TestCausalEdge:
    def test_valid_edge(self):
        e = CausalEdge("X", "Y")
        assert e.cause == "X" and e.effect == "Y"

    def test_self_loop_raises(self):
        with pytest.raises(ValueError):
            CausalEdge("X", "X")


# ----------------------------------------------------------------------
# Tests: CausalDAG
# ----------------------------------------------------------------------

class TestCausalDAG:
    def test_init_valid(self):
        dag = make_confounding_dag()
        assert set(dag.nodes) == {"X", "Y", "Z"}
        assert len(dag.edges) == 3

    def test_init_empty_nodes_raises(self):
        with pytest.raises(ValueError):
            CausalDAG([], [])

    def test_init_duplicate_nodes_raises(self):
        with pytest.raises(ValueError):
            CausalDAG(["X", "X"], [])

    def test_init_edge_not_in_nodes_raises(self):
        with pytest.raises(ValueError):
            CausalDAG(["X"], [("X", "Y")])

    def test_init_cycle_raises(self):
        with pytest.raises(ValueError):
            CausalDAG(["X", "Y"], [("X", "Y"), ("Y", "X")])

    def test_parents_children(self):
        dag = make_confounding_dag()
        assert set(dag.parents("X")) == {"Z"}
        assert set(dag.children("Z")) == {"X", "Y"}

    def test_ancestors(self):
        dag = make_confounding_dag()
        assert dag.ancestors("Y") == {"X", "Z"}
        assert dag.ancestors("Z") == set()

    def test_descendants(self):
        dag = make_confounding_dag()
        assert dag.descendants("Z") == {"X", "Y"}
        assert dag.descendants("Y") == set()

    def test_topological_sort(self):
        dag = make_confounding_dag()
        topo = dag.topological_sort()
        # Z must come before X and Y; X must come before Y
        idx = {n: i for i, n in enumerate(topo)}
        assert idx["Z"] < idx["X"]
        assert idx["Z"] < idx["Y"]
        assert idx["X"] < idx["Y"]

    def test_d_separated_no_conditioning(self):
        dag = make_confounding_dag()
        # X -> Y is an open path (no conditioning)
        assert not dag.is_d_separated("X", "Y")

    def test_d_separated_conditioning_collider(self):
        # Build: X -> M <- Y (collider, open when conditioning on M)
        dag = CausalDAG(["X", "M", "Y"], [("X", "M"), ("Y", "M")])
        # Without conditioning: X and Y are d-separated (collider closed)
        assert dag.is_d_separated("X", "Y", set())
        # Conditioning on M: opens collider
        assert not dag.is_d_separated("X", "Y", {"M"})

    def test_d_separated_chain_blocked(self):
        # X -> M -> Y (chain blocked by M)
        dag = CausalDAG(["X", "M", "Y"], [("X", "M"), ("M", "Y")])
        assert not dag.is_d_separated("X", "Y")
        assert dag.is_d_separated("X", "Y", {"M"})

    def test_d_separated_fork_blocked(self):
        # X <- Z -> Y (fork blocked by Z)
        dag = CausalDAG(["X", "Z", "Y"], [("Z", "X"), ("Z", "Y")])
        assert not dag.is_d_separated("X", "Y")
        assert dag.is_d_separated("X", "Y", {"Z"})


# ----------------------------------------------------------------------
# Tests: StructuralCausalModel
# ----------------------------------------------------------------------

class TestStructuralCausalModel:
    def test_init_missing_equation_raises(self):
        dag = make_confounding_dag()
        with pytest.raises(ValueError):
            StructuralCausalModel(
                dag,
                equations={"Z": lambda p, u: u, "X": lambda p, u: u},
                noise_std={"Z": 1.0, "X": 0.5, "Y": 0.5},
            )

    def test_observational_sample(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=42)
        assert set(data.keys()) == {"X", "Y", "Z"}
        assert len(data["X"]) == 500
        # Y should correlate positively with X (true effect = 2.0)
        xs = data["X"]; ys = data["Y"]
        mx = sum(xs) / len(xs); my = sum(ys) / len(ys)
        num = sum((a - mx) * (b - my) for a, b in zip(xs, ys))
        sx = (sum((a - mx) ** 2 for a in xs) / len(xs)) ** 0.5
        sy = (sum((b - my) ** 2 for b in ys) / len(ys)) ** 0.5
        corr = num / (len(xs) * sx * sy) if sx and sy else 0
        assert corr > 0.3  # positive correlation

    def test_interventional_sample(self):
        scm = make_confounding_scm()
        s1 = scm.interventional_sample(n=500, do_var="X", do_val=1.0, seed=42)
        s0 = scm.interventional_sample(n=500, do_var="X", do_val=0.0, seed=42)
        # X should be ~constant in each sample
        assert all(abs(v - 1.0) < 1e-9 for v in s1["X"])
        assert all(abs(v - 0.0) < 1e-9 for v in s0["X"])


# ----------------------------------------------------------------------
# Tests: DoOperator
# ----------------------------------------------------------------------

class TestDoOperator:
    def test_intervened_dag_removes_edges_into_x(self):
        scm = make_confounding_scm()
        op = DoOperator(scm)
        mutilated = op.intervened_dag("X")
        # Original has Z -> X; mutilated should not
        edge_targets = [e.effect for e in mutilated.edges]
        assert "X" not in edge_targets

    def test_interventional_mean(self):
        scm = make_confounding_scm()
        op = DoOperator(scm)
        # E[Y | do(X=1)] should be ~2.0 (true ATE coefficient)
        m1 = op.interventional_mean("X", 1.0, "Y", n=2000, seed=0)
        m0 = op.interventional_mean("X", 0.0, "Y", n=2000, seed=1)
        # True ATE = 2.0 (coefficient on X in Y equation)
        assert abs((m1 - m0) - 2.0) < 0.5


# ----------------------------------------------------------------------
# Tests: BackdoorCriterion
# ----------------------------------------------------------------------

class TestBackdoorCriterion:
    def test_is_backdoor_valid_set(self):
        dag = make_confounding_dag()
        bd = BackdoorCriterion(dag)
        assert bd.is_backdoor("X", "Y", {"Z"})

    def test_is_backdoor_invalid_with_descendant(self):
        dag = make_confounding_dag()
        bd = BackdoorCriterion(dag)
        # Y is descendant of X, can't condition on it
        assert not bd.is_backdoor("X", "Y", {"Y"})

    def test_find_adjustment_set(self):
        dag = make_confounding_dag()
        bd = BackdoorCriterion(dag)
        adj = bd.find_adjustment_set("X", "Y")
        assert adj is not None
        assert "Z" in adj


# ----------------------------------------------------------------------
# Tests: InstrumentalVariableEstimator
# ----------------------------------------------------------------------

class TestInstrumentalVariableEstimator:
    def test_two_stage_least_squares(self):
        rng = random.Random(42)
        n = 1000
        z = [rng.gauss(0, 1) for _ in range(n)]
        u = [rng.gauss(0, 0.5) for _ in range(n)]
        # True model: X = Z + U, Y = 2*X + U_noise
        x = [zi + ui for zi, ui in zip(z, u)]
        u2 = [rng.gauss(0, 0.5) for _ in range(n)]
        y = [2.0 * xi + ui2 for xi, ui2 in zip(x, u2)]
        data = {"Z": z, "X": x, "Y": y}
        est = InstrumentalVariableEstimator(data)
        res = est.two_stage_least_squares("Z", "X", "Y")
        # iv_coef should recover ~2.0
        assert 1.5 < res["iv_coef"] < 2.5
        assert res["n"] == n

    def test_ols_helper(self):
        a, b = InstrumentalVariableEstimator._ols([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        assert abs(a) < 1e-9 and abs(b - 1.0) < 1e-9


# ----------------------------------------------------------------------
# Tests: CounterfactualEngine
# ----------------------------------------------------------------------

class TestCounterfactualEngine:
    def test_abduct_returns_noise_dict(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=200, seed=0)
        engine = CounterfactualEngine(scm)
        evidence = {v: data[v][10] for v in ["X", "Y", "Z"]}
        noise = engine.abduct(evidence, data)
        assert set(noise.keys()) == {"X", "Y", "Z"}

    def test_act_and_predict_uses_abducted_noise(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=200, seed=0)
        engine = CounterfactualEngine(scm)
        evidence = {v: data[v][10] for v in ["X", "Y", "Z"]}
        noise = engine.abduct(evidence, data)
        # With same noise, predicting X=evidence[X] should yield ~evidence[Y]
        # (since Y = 2X + 1.5Z + U_Y)
        # Approximate test: changing X by 1 should change Y by ~2
        y_at_x = engine.act_and_predict("X", evidence["X"], "Y", noise=noise)
        y_at_x1 = engine.act_and_predict("X", evidence["X"] + 1.0, "Y", noise=noise)
        assert abs((y_at_x1 - y_at_x) - 2.0) < 0.01

    def test_act_and_predict_no_noise_raises(self):
        scm = make_confounding_scm()
        engine = CounterfactualEngine(scm)
        with pytest.raises(ValueError):
            engine.act_and_predict("X", 1.0, "Y")


# ----------------------------------------------------------------------
# Tests: Refuter
# ----------------------------------------------------------------------

class TestRefuter:
    def test_placebo_refute(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=0)
        rng = random.Random(0)
        res = Refuter.placebo_refute(scm._dag, data, "X", "Y", rng)
        assert "placebo_effect" in res

    def test_random_common_cause_refute(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=0)
        rng = random.Random(0)
        res = Refuter.random_common_cause_refute(scm._dag, data, "X", "Y", rng)
        assert "placebo_effect" in res

    def test_data_subset_refute(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=0)
        rng = random.Random(0)
        res = Refuter.data_subset_refute(scm._dag, data, "X", "Y", 0.8, rng)
        assert "subset_effect" in res

    def test_bootstrap_refute(self):
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=0)
        rng = random.Random(0)
        res = Refuter.bootstrap_refute(scm._dag, data, "X", "Y", 30, rng)
        assert "bootstrap_mean" in res
        assert "bootstrap_std" in res


# ----------------------------------------------------------------------
# Tests: CausalEstimator
# ----------------------------------------------------------------------

class TestCausalEstimator:
    def test_average_treatment_effect(self):
        scm = make_confounding_scm()
        est = CausalEstimator(scm)
        ate = est.average_treatment_effect("X", "Y", 1.0, 0.0, n=3000, seed=42)
        # True ATE = 2.0 (coefficient on X)
        assert 1.5 < ate < 2.5

    def test_conditional_average_treatment_effect(self):
        scm = make_confounding_scm()
        est = CausalEstimator(scm)
        # CATE should be ~2.0 regardless of Z (X's effect is constant here)
        cate = est.conditional_average_treatment_effect(
            "X", "Y", "Z", 0.0, 1.0, 0.0, n=2000, seed=0
        )
        assert 1.0 < cate < 3.0


# ----------------------------------------------------------------------
# Tests: CausalReport
# ----------------------------------------------------------------------

class TestCausalReport:
    def test_init(self):
        r = CausalReport("Test")
        assert r.title == "Test"
        assert r.render().startswith("# Test")

    def test_add_section_renders(self):
        r = CausalReport("T")
        r.add_section("Findings", "ATE = 2.0")
        text = r.render()
        assert "## Findings" in text
        assert "ATE = 2.0" in text


# ----------------------------------------------------------------------
# Tests: V1042CausalReasoning orchestrator
# ----------------------------------------------------------------------

class TestV1042CausalReasoning:
    def test_init(self):
        v = V1042CausalReasoning()
        assert v.n_dags() == 0
        assert v.n_scms() == 0
        assert v.n_reports() == 0

    def test_register_dag_scm(self):
        v = V1042CausalReasoning()
        v.register_dag("d1", make_confounding_dag())
        v.register_scm("s1", make_confounding_scm())
        assert v.n_dags() == 1
        assert v.n_scms() == 1

    def test_run_4_step_analysis(self):
        v = V1042CausalReasoning()
        scm = make_confounding_scm()
        v.register_scm("conf", scm)
        report = v.run_4_step_analysis("conf", "X", "Y", 1.0, 0.0, n=1000, seed=0)
        text = report.render()
        assert "Step 1: Model" in text
        assert "Step 2: Identify" in text
        assert "Step 3: Estimate" in text
        assert "Step 4: Refute" in text
        assert "ATE" in text
        assert v.n_reports() == 1

    def test_run_4_step_analysis_unknown_scm_raises(self):
        v = V1042CausalReasoning()
        with pytest.raises(KeyError):
            v.run_4_step_analysis("nonexistent", "X", "Y", 1.0, 0.0)

    def test_version(self):
        assert V1042_VERSION == "0.1.0"


# ----------------------------------------------------------------------
# Integration test: full causal pipeline
# ----------------------------------------------------------------------

class TestIntegration:
    def test_full_4_step_pipeline(self):
        """真生产 E2E: 构建 DAG → SCM → 4-step 分析 → 报告."""
        v = V1042CausalReasoning()
        scm = make_confounding_scm()
        v.register_scm("conf", scm)
        report = v.run_4_step_analysis(
            "conf", "X", "Y", x_treat=1.0, x_control=0.0, n=2000, seed=42
        )
        text = report.render()
        # ATE should be close to 2.0 (true effect)
        assert "ATE" in text
        assert v.n_reports() == 1

    def test_do_vs_observational(self):
        """do(X) vs observational: do should give unbiased ATE."""
        scm = make_confounding_scm()
        est = CausalEstimator(scm)
        ate = est.average_treatment_effect("X", "Y", 1.0, 0.0, n=5000, seed=42)
        # True ATE = 2.0
        assert 1.7 < ate < 2.3

    def test_counterfactual_consistency(self):
        """Counterfactual: changing X by delta should change Y by 2*delta (true model)."""
        scm = make_confounding_scm()
        data = scm.observational_sample(n=500, seed=42)
        engine = CounterfactualEngine(scm)
        evidence = {v: data[v][100] for v in ["X", "Y", "Z"]}
        noise = engine.abduct(evidence, data)
        y0 = engine.act_and_predict("X", evidence["X"], "Y", noise=noise)
        y1 = engine.act_and_predict("X", evidence["X"] + 0.5, "Y", noise=noise)
        # Y = 2X + 1.5Z + U; so delta_Y = 2*delta_X
        assert abs((y1 - y0) - 1.0) < 0.01