"""Tests for V1064 ASI Continual Learning (主 17:43 实事求是 + 主 00:56)."""
from __future__ import annotations

import math
import random

import pytest

from apeireth.v1064_asi_continual_learning import (
    V1064_VERSION,
    ASIContinualLearningBridge,
    ContinualBuffer,
    ContinualLearningGuard,
    ContinualLearningPipeline,
    ContinualLearningReport,
    ContinualLearner,
    ContinualSample,
    ContinualTask,
    DistillationLoss,
    EWCRegularizer,
    ElasticWeight,
    RehearsalSampler,
    SimpleModel,
    SynapticIntelligence,
    build_continual_learner,
    build_pipeline,
    make_task,
    quick_score,
)


# ---------------------------------------------------------------------------
# 1. ContinualTask + ContinualSample tests
# ---------------------------------------------------------------------------

class TestContinualTask:
    def test_task_creation(self):
        task = make_task("t1", lambda x: [sum(x), sum(x) * 0.5])
        assert task.task_id == "t1"
        assert task.n_samples == 100

    def test_task_sample(self):
        task = make_task("t1", lambda x: [sum(x)])
        samples = task.sample(5)
        assert len(samples) == 5
        for s in samples:
            assert isinstance(s, ContinualSample)
            assert isinstance(s.x, list)
            assert isinstance(s.y, list)


# ---------------------------------------------------------------------------
# 2. ContinualBuffer tests (Lopez-Paz 2017)
# ---------------------------------------------------------------------------

class TestContinualBuffer:
    def test_buffer_init(self):
        b = ContinualBuffer(capacity=10)
        assert b.capacity == 10
        assert len(b) == 0

    def test_buffer_add(self):
        b = ContinualBuffer(capacity=10)
        for i in range(5):
            s = ContinualSample(x=[i], y=[i])
            b.add(s, f"task_{i}")
        assert len(b) == 5

    def test_buffer_fifo_eviction(self):
        b = ContinualBuffer(capacity=3)
        for i in range(5):
            s = ContinualSample(x=[i], y=[i])
            b.add(s, f"t_{i}")
        assert len(b) == 3
        # Oldest 2 should be evicted
        assert b.samples[0].x == [2]

    def test_buffer_sample(self):
        b = ContinualBuffer(capacity=10)
        for i in range(5):
            b.add(ContinualSample(x=[i], y=[i]), "t1")
        sampled = b.sample(3)
        assert len(sampled) == 3

    def test_buffer_sample_empty(self):
        b = ContinualBuffer()
        assert b.sample(5) == []

    def test_buffer_sample_more_than_available(self):
        b = ContinualBuffer()
        b.add(ContinualSample(x=[1.0], y=[1.0]), "t1")
        sampled = b.sample(5)
        assert len(sampled) == 1


# ---------------------------------------------------------------------------
# 3. ElasticWeight tests (Kirkpatrick 2017)
# ---------------------------------------------------------------------------

class TestElasticWeight:
    def test_init(self):
        e = ElasticWeight()
        assert e.importances == {}
        assert e.star_values == {}

    def test_update(self):
        e = ElasticWeight()
        e.update("w_0", 0.5)
        e.update("w_0", 0.3)
        assert e.importance("w_0") == pytest.approx(0.8, abs=1e-9)

    def test_importance_default(self):
        e = ElasticWeight()
        assert e.importance("nonexistent") == 0.0

    def test_set_star(self):
        e = ElasticWeight()
        e.set_star("w_0", 1.5)
        assert e.star_values["w_0"] == 1.5

    def test_penalty(self):
        e = ElasticWeight()
        e.update("w_0", 2.0)
        e.set_star("w_0", 1.0)
        # penalty = 2.0 * (1.5 - 1.0)^2 = 0.5
        penalty = e.penalty({"w_0": 1.5})
        assert penalty == pytest.approx(0.5, abs=1e-9)

    def test_num_tracked(self):
        e = ElasticWeight()
        e.update("w_0", 0.5)
        e.update("w_1", 0.3)
        assert e.num_tracked() == 2


# ---------------------------------------------------------------------------
# 4. EWCRegularizer tests (Kirkpatrick 2017)
# ---------------------------------------------------------------------------

class TestEWCRegularizer:
    def test_init(self):
        ewc = ElasticWeight()
        reg = EWCRegularizer(ewc=ewc, lambda_ewc=0.5)
        assert reg.lambda_ewc == 0.5

    def test_loss_zero_no_data(self):
        reg = EWCRegularizer(ewc=ElasticWeight(), lambda_ewc=0.5)
        loss = reg.loss({"w_0": 1.0})
        assert loss == 0.0

    def test_loss_with_data(self):
        ewc = ElasticWeight()
        ewc.update("w_0", 2.0)
        ewc.set_star("w_0", 1.0)
        reg = EWCRegularizer(ewc=ewc, lambda_ewc=0.5)
        # (0.5 / 2) * 2.0 * (1.5 - 1.0)^2 = 0.25 * 0.5 = 0.125
        loss = reg.loss({"w_0": 1.5})
        assert loss == pytest.approx(0.125, abs=1e-9)

    def test_consolidate(self):
        ewc = ElasticWeight()
        reg = EWCRegularizer(ewc=ewc)
        reg.consolidate({"w_0": 1.5, "w_1": 2.5})
        assert ewc.star_values["w_0"] == 1.5
        assert ewc.star_values["w_1"] == 2.5


# ---------------------------------------------------------------------------
# 5. SynapticIntelligence tests (Zenke 2017)
# ---------------------------------------------------------------------------

class TestSynapticIntelligence:
    def test_init(self):
        si = SynapticIntelligence()
        assert si.omega == {}

    def test_step_update(self):
        si = SynapticIntelligence()
        si.step_update("w_0", grad=0.5, total_delta=1.0)
        omega_inc = -0.5 * 1.0 / (1.0 + 0.1)  # ≈ -0.4545
        assert si.omega["w_0"] == pytest.approx(omega_inc, abs=1e-4)

    def test_importance_default(self):
        si = SynapticIntelligence()
        assert si.importance("nonexistent") == 0.0

    def test_importance_nonneg(self):
        si = SynapticIntelligence()
        si.step_update("w_0", grad=-10.0, total_delta=0.0)
        # omega = -(-10) * 0 / (0 + 0.1) = 0 → max(0, 0) = 0
        # or could be negative before clamp
        assert si.importance("w_0") >= 0

    def test_penalty(self):
        si = SynapticIntelligence()
        si.step_update("w_0", grad=-1.0, total_delta=1.0)
        # omega = 1.0 / 1.1 ≈ 0.909
        # penalty = c * omega * (theta - star)^2 = 1.0 * 0.909 * 1.0 = 0.909
        p = si.penalty({"w_0": 1.0}, {"w_0": 0.0}, c=1.0)
        assert p > 0

    def test_num_tracked(self):
        si = SynapticIntelligence()
        si.step_update("w_0", 0.5, 1.0)
        si.step_update("w_1", 0.5, 1.0)
        assert si.num_tracked() == 2


# ---------------------------------------------------------------------------
# 6. DistillationLoss tests (Hinton 2015)
# ---------------------------------------------------------------------------

class TestDistillationLoss:
    def test_softmax_sum_to_one(self):
        d = DistillationLoss()
        p = d.softmax([1.0, 2.0, 3.0])
        assert math.isclose(sum(p), 1.0, abs_tol=1e-9)
        # Monotonicity
        assert p[0] < p[1] < p[2]

    def test_softmax_temperature_smoothing(self):
        d = DistillationLoss(temperature=1.0)
        p1 = d.softmax([1.0, 5.0, 1.0])
        d2 = DistillationLoss(temperature=5.0)
        p5 = d2.softmax([1.0, 5.0, 1.0])
        # Higher T → smoother distribution
        # ratio of max to second should be smaller at higher T
        assert max(p5) < max(p1)

    def test_kl_zero_identical(self):
        d = DistillationLoss()
        kl = d.kl_divergence([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        assert kl < 1e-9

    def test_kl_positive_different(self):
        d = DistillationLoss()
        kl = d.kl_divergence([1.0, 2.0, 3.0], [3.0, 2.0, 1.0])
        assert kl > 0

    def test_total_loss(self):
        d = DistillationLoss(alpha=0.5)
        loss = d.total_loss([1.0, 2.0], [0.5, 1.5], [1.0, 2.0])
        # KL=0 (identical teacher/student), MSE > 0
        assert loss > 0


# ---------------------------------------------------------------------------
# 7. RehearsalSampler tests (Silver 2013)
# ---------------------------------------------------------------------------

class TestRehearsalSampler:
    def test_init(self):
        r = RehearsalSampler(alpha_old=0.5)
        assert r.alpha_old == 0.5

    def test_mix_empty_old(self):
        r = RehearsalSampler(alpha_old=0.5)
        new = [ContinualSample(x=[1.0], y=[1.0]) for _ in range(5)]
        mixed = r.mix(new, [], 4)
        assert len(mixed) == 4

    def test_mix_with_old(self):
        r = RehearsalSampler(alpha_old=0.5)
        new = [ContinualSample(x=[1.0], y=[1.0]) for _ in range(10)]
        old = [ContinualSample(x=[0.0], y=[0.0]) for _ in range(10)]
        mixed = r.mix(new, old, 10)
        assert len(mixed) == 10
        # alpha_old=0.5 → n_old = 5
        old_count = sum(1 for s in mixed if s.x == [0.0])
        assert old_count == 5

    def test_mix_empty_inputs(self):
        r = RehearsalSampler()
        mixed = r.mix([], [], 5)
        assert mixed == []


# ---------------------------------------------------------------------------
# 8. SimpleModel tests
# ---------------------------------------------------------------------------

class TestSimpleModel:
    def test_init(self):
        m = SimpleModel()
        assert m.weights == []
        assert m.in_dim == 0

    def test_init_params(self):
        m = SimpleModel()
        m.init_params(in_dim=3, out_dim=2)
        assert m.in_dim == 3
        assert m.out_dim == 2
        assert len(m.weights) == 6  # 3 * 2

    def test_named_params(self):
        m = SimpleModel()
        m.init_params(in_dim=3, out_dim=2)
        params = m.named_params()
        assert len(params) == 6
        assert all(k.startswith("w_") for k in params.keys())

    def test_predict(self):
        m = SimpleModel()
        m.init_params(in_dim=2, out_dim=2, seed=42)
        out = m.predict([1.0, 2.0])
        assert len(out) == 2

    def test_grad_squared(self):
        m = SimpleModel()
        m.init_params(in_dim=2, out_dim=1, seed=42)
        grads = m.grad_squared([1.0, 0.5], [0.5])
        assert len(grads) == 2  # 1 output × 2 inputs

    def test_sgd_step_runs(self):
        m = SimpleModel()
        m.init_params(in_dim=2, out_dim=1, seed=42)
        before = list(m.weights)
        m.sgd_step([1.0, 0.5], [0.5])
        # Weights should change
        assert m.weights != before


# ---------------------------------------------------------------------------
# 9. ContinualLearner tests
# ---------------------------------------------------------------------------

class TestContinualLearner:
    def test_init(self):
        learner = build_continual_learner(in_dim=3, out_dim=2)
        assert learner.lr == 0.01
        assert learner.use_distillation is True

    def test_train_task_runs(self):
        learner = build_continual_learner()
        task = make_task("t1", lambda x: [sum(x), sum(x) * -1])
        result = learner.train_task(task, n_samples=10, batch_size=3)
        assert result["task_id"] == "t1"
        assert result["n_samples"] == 10
        assert "buffer_size" in result
        assert "ewc_penalty" in result

    def test_train_task_updates_buffer(self):
        learner = build_continual_learner()
        task = make_task("t1", lambda x: [sum(x)])
        learner.train_task(task, n_samples=10, batch_size=3)
        assert len(learner.buffer) > 0

    def test_train_task_updates_ewc(self):
        learner = build_continual_learner()
        task = make_task("t1", lambda x: [sum(x)])
        learner.train_task(task, n_samples=10, batch_size=3)
        assert learner.ewc.ew.num_tracked() > 0

    def test_train_task_consolidates(self):
        learner = build_continual_learner()
        task = make_task("t1", lambda x: [sum(x)])
        learner.train_task(task, n_samples=10, batch_size=3)
        # star_values should be set
        assert len(learner.ewc.ew.star_values) > 0

    def test_evaluate_task(self):
        learner = build_continual_learner()
        task = make_task("t1", lambda x: [sum(x)])
        loss = learner.evaluate_task(task, n_samples=5)
        assert isinstance(loss, float)
        assert loss >= 0

    def test_backward_transfer(self):
        learner = build_continual_learner()
        task1 = make_task("t1", lambda x: [sum(x), max(x)])
        task2 = make_task("t2", lambda x: [min(x), sum(x) * 2])
        learner.train_task(task1, n_samples=10, batch_size=3)
        learner.train_task(task2, n_samples=10, batch_size=3)
        all_tasks = {task1.task_id: task1, task2.task_id: task2}
        losses = learner.backward_transfer(all_tasks, n_samples=5)
        assert len(losses) == 2


# ---------------------------------------------------------------------------
# 10. ContinualLearningReport tests (主 00:56)
# ---------------------------------------------------------------------------

class TestContinualLearningReport:
    def test_report_init(self):
        rep = ContinualLearningReport()
        assert rep.title == "ASI Continual Learning Report"

    def test_report_add_section(self):
        rep = ContinualLearningReport()
        rep.add_section("Test", "Body")
        assert ("Test", "Body") in rep.sections

    def test_report_render(self):
        rep = ContinualLearningReport(title="Test")
        rep.add_section("Components", "1. Task")
        md = rep.render()
        assert "# Test" in md
        assert "## Components" in md

    def test_summary_dict(self):
        s = ContinualLearningReport.summary_dict(3, 50, 20, 15)
        assert "3" in s
        assert "50" in s


# ---------------------------------------------------------------------------
# 11. ASIContinualLearningBridge tests
# ---------------------------------------------------------------------------

class TestASIContinualLearningBridge:
    def test_bridge_init(self):
        b = ASIContinualLearningBridge()
        assert sum(b.weights.values()) == pytest.approx(1.0, abs=1e-9)

    def test_score_zero(self):
        b = ASIContinualLearningBridge()
        r = b.score({})
        assert r["continual_learning_v0_2"] == 0.0

    def test_score_perfect(self):
        b = ASIContinualLearningBridge()
        perfect = {k: 1.0 for k in b.weights}
        r = b.score(perfect)
        assert r["continual_learning_v0_2"] == pytest.approx(1.0, abs=1e-9)

    def test_threshold_pass(self):
        b = ASIContinualLearningBridge()
        r = b.threshold_check(0.90)
        assert r["passed"] is True

    def test_threshold_fail(self):
        b = ASIContinualLearningBridge()
        r = b.threshold_check(0.5)
        assert r["passed"] is False


# ---------------------------------------------------------------------------
# 12. ContinualLearningGuard tests (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------

class TestContinualLearningGuard:
    def test_continual_no_forgetting_guard(self):
        g = ContinualLearningGuard.guard_continual_no_forgetting({"backward_transfer": 0.5})
        assert g["guard"] == "continual_no_forgetting"
        assert "McCloskey" in g["verdict"]

    def test_memory_understanding_guard(self):
        g = ContinualLearningGuard.guard_memory_understanding({"buffer_usage": 0.5})
        assert g["guard"] == "memory_understanding"

    def test_ewc_consciousness_guard(self):
        g = ContinualLearningGuard.guard_ewc_consciousness({"ewc_coverage": 0.7})
        assert "Kirkpatrick" in g["verdict"]

    def test_rehearsal_experience_guard(self):
        g = ContinualLearningGuard.guard_rehearsal_experience({"rehearsal_alpha": 0.5})
        assert g["guard"] == "rehearsal_experience"

    def test_asi_learns_continually_guard(self):
        g = ContinualLearningGuard.guard_asi_learns_continually(
            {"continual_learning_v0_2": 0.95})
        assert "structural" in g["verdict"]

    def test_all_guards(self):
        g = ContinualLearningGuard.all_guards({})
        assert len(g) == 5


# ---------------------------------------------------------------------------
# 13. ContinualLearningPipeline integration
# ---------------------------------------------------------------------------

class TestContinualLearningPipeline:
    def test_default_pipeline(self):
        p = ContinualLearningPipeline.default(in_dim=3, out_dim=2)
        assert p.learner.model.in_dim == 3
        assert p.learner.model.out_dim == 2

    def test_train_sequence(self):
        p = ContinualLearningPipeline.default()
        tasks = [
            make_task("t1", lambda x: [sum(x), max(x)]),
            make_task("t2", lambda x: [min(x), sum(x) * 2]),
        ]
        results = p.train_sequence(tasks)
        assert len(results) == 2
        assert p.learner.seen_tasks == ["t1", "t2"]

    def test_pipeline_report(self):
        p = ContinualLearningPipeline.default()
        md = p.report(n_tasks_trained=2)
        assert "ASI Continual Learning Report" in md
        assert "Kirkpatrick 2017" in md
        assert "V3 哲学守门" in md

    def test_build_pipeline_helper(self):
        p = build_pipeline(in_dim=4, out_dim=2)
        assert isinstance(p, ContinualLearningPipeline)

    def test_quick_score_runs(self):
        p = ContinualLearningPipeline.default()
        r = quick_score(p.learner, n_tasks=2)
        assert "continual_learning_v0_2" in r
        assert 0.0 <= r["continual_learning_v0_2"] <= 1.0


# ---------------------------------------------------------------------------
# 14. Sanity tests
# ---------------------------------------------------------------------------

class TestSanity:
    def test_version(self):
        assert V1064_VERSION == "0.1.0"

    def test_14_precedents_documented(self):
        import apeireth.v1064_asi_continual_learning as mod
        src = mod.__doc__ or ""
        expected = ["McCloskey 1989", "Ratcliff 1990", "Ring 1994", "Thrun 1996",
                    "Silver 2013", "Parisi 2019", "Schmidhuber 2013",
                    "Kirkpatrick 2017", "Zenke 2017", "Rusu 2016",
                    "Hinton 2015", "Lopez-Paz 2017", "Lee 2019", "Robins 1995"]
        for ref in expected:
            assert ref in src, f"missing: {ref}"

    def test_10_components_documented(self):
        import apeireth.v1064_asi_continual_learning as mod
        src = mod.__doc__ or ""
        for comp in ["ContinualTask", "ContinualBuffer", "ElasticWeight",
                     "EWCRegularizer", "SynapticIntelligence", "DistillationLoss",
                     "RehearsalSampler", "ContinualLearner",
                     "ContinualLearningReport", "ASIContinualLearningBridge"]:
            assert comp in src, f"missing: {comp}"

    def test_5_guards_documented(self):
        import apeireth.v1064_asi_continual_learning as mod
        src = mod.__doc__ or ""
        for guard in ["不假装 Continual Learning = Never Forgetting",
                      "不假装 Memory = Understanding",
                      "不假装 EWC = consciousness",
                      "不假装 rehearsal = experience",
                      "不假装 ASI learns continually"]:
            assert guard in src, f"missing: {guard}"

    def test_no_pretend_consciousness(self):
        import apeireth.v1064_asi_continual_learning as mod
        with open(mod.__file__, encoding="utf-8") as f:
            src = (mod.__doc__ or "") + f.read()
        forbidden_phrases = ["EWC IS consciousness",
                             "buffer IS memory",
                             "rehearsal == experience",
                             "SI == understanding"]
        for phrase in forbidden_phrases:
            assert phrase not in src

    def test_reproducibility(self):
        random.seed(42)
        learner1 = build_continual_learner()
        task1 = make_task("t1", lambda x: [sum(x), max(x)])
        learner1.train_task(task1, n_samples=5, batch_size=2)
        n1 = len(learner1.buffer)

        random.seed(42)
        learner2 = build_continual_learner()
        task2 = make_task("t1", lambda x: [sum(x), max(x)])
        learner2.train_task(task2, n_samples=5, batch_size=2)
        n2 = len(learner2.buffer)
        assert n1 == n2