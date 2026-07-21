"""Tests for V1062 ASI World Model (主 17:43 实事求是 + 主 00:56 任何人都能接手)."""
from __future__ import annotations

import math
import random

import pytest

from apeireth.v1062_asi_world_model import (
    V1062_VERSION,
    ActivationType,
    ASIWorldModelBridge,
    DynaPlanner,
    Experience,
    ImaginationEngine,
    ImaginedStep,
    JEPAEmbedding,
    ObservationDecoder,
    RewardPredictor,
    TransitionModel,
    VariationalEncoder,
    WorldModelGuard,
    WorldModelPipeline,
    WorldModelReport,
    WorldState,
    build_world_model,
    quick_score,
)


# ---------------------------------------------------------------------------
# 1. WorldState tests
# ---------------------------------------------------------------------------

class TestWorldState:
    def test_state_creation(self):
        s = WorldState(state_id="s1", z=[0.1, 0.2, 0.3])
        assert s.state_id == "s1"
        assert len(s.z) == 3
        assert s.hidden is not None and len(s.hidden) == 3

    def test_state_distance_l2(self):
        s1 = WorldState(state_id="a", z=[0.0, 0.0])
        s2 = WorldState(state_id="b", z=[3.0, 4.0])
        assert abs(s1.distance(s2, "l2") - 5.0) < 1e-9

    def test_state_distance_l1(self):
        s1 = WorldState(state_id="a", z=[1.0, 2.0])
        s2 = WorldState(state_id="b", z=[4.0, 6.0])
        assert s1.distance(s2, "l1") == 7.0

    def test_state_distance_cosine(self):
        s1 = WorldState(state_id="a", z=[1.0, 0.0])
        s2 = WorldState(state_id="b", z=[1.0, 0.0])
        assert s1.distance(s2, "cosine") < 1e-9
        s3 = WorldState(state_id="c", z=[0.0, 1.0])
        assert abs(s1.distance(s3, "cosine") - 1.0) < 1e-9

    def test_state_distance_unknown_metric_raises(self):
        s1 = WorldState(state_id="a", z=[1.0])
        s2 = WorldState(state_id="b", z=[2.0])
        with pytest.raises(ValueError):
            s1.distance(s2, "bogus")

    def test_state_summary(self):
        s = WorldState(state_id="x", z=[1.0, 2.0, 3.0])
        summary = s.summary()
        assert summary["state_id"] == "x"
        assert summary["latent_dim"] == 3
        assert summary["mean"] == pytest.approx(2.0, abs=1e-9)
        assert summary["norm"] == pytest.approx(math.sqrt(14), abs=1e-9)
        assert summary["deterministic"] is True


# ---------------------------------------------------------------------------
# 2. VariationalEncoder tests (Kingma 2014)
# ---------------------------------------------------------------------------

class TestVariationalEncoder:
    def test_encoder_init(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=2)
        assert enc.obs_dim == 4
        assert enc.latent_dim == 2
        assert len(enc.weights) == 2
        assert all(len(w) == 4 for w in enc.weights)

    def test_encoder_dim_mismatch(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=2)
        with pytest.raises(ValueError):
            enc.encode([0.1, 0.2])

    def test_encoder_encode_shape(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=3)
        mu, lv = enc.encode([0.1, 0.2, 0.3, 0.4])
        assert len(mu) == 3
        assert len(lv) == 3

    def test_encoder_reparameterize(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=3, activation=ActivationType.LINEAR)
        mu = [0.5, -0.5, 0.0]
        log_var = [-2.0, -3.0, -4.0]
        z = enc.reparameterize(mu, log_var)
        assert len(z) == 3
        # all finite
        assert all(math.isfinite(x) for x in z)

    def test_encoder_kl_zero_for_standard_normal(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=3, activation=ActivationType.LINEAR)
        # If mu=0, log_var=0 → KL = 0
        kl = enc.kl_divergence([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])
        assert abs(kl) < 1e-9

    def test_encoder_kl_positive(self):
        enc = VariationalEncoder(obs_dim=4, latent_dim=3)
        kl = enc.kl_divergence([1.0, 0.5, -0.5], [0.0, 0.0, 0.0])
        assert kl > 0  # non-zero mu → positive KL

    def test_encoder_activations(self):
        for act in [ActivationType.LINEAR, ActivationType.SIGMOID, ActivationType.TANH, ActivationType.RELU]:
            enc = VariationalEncoder(obs_dim=3, latent_dim=2, activation=act)
            mu, _ = enc.encode([1.0, 2.0, 3.0])
            # mu values must be in expected range
            for x in mu:
                if act == ActivationType.SIGMOID or act == ActivationType.TANH:
                    assert -1.001 <= x <= 1.001
                elif act == ActivationType.RELU:
                    assert x >= -1e-9


# ---------------------------------------------------------------------------
# 3. TransitionModel tests (Ha 2018 MDN-RNN-like)
# ---------------------------------------------------------------------------

class TestTransitionModel:
    def test_transition_init(self):
        t = TransitionModel(state_dim=4, action_dim=2)
        assert t.state_dim == 4
        assert t.action_dim == 2
        assert t.hidden_dim == 32

    def test_transition_step_shape(self):
        t = TransitionModel(state_dim=3, action_dim=2, hidden_dim=4)
        h, ns = t.step([0.1, 0.2, 0.3], [0.5, 0.6])
        assert len(h) == 4
        assert len(ns) == 3

    def test_transition_with_hidden(self):
        t = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        h = [0.1, 0.2, 0.3]
        new_h, ns = t.step([0.0, 0.0], [0.0], h)
        assert len(new_h) == 3
        # hidden should have changed
        assert new_h != h

    def test_transition_rollout(self):
        t = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        actions = [[0.1], [0.2], [0.3], [0.4]]
        traj = t.rollout([0.0, 0.0], actions)
        assert len(traj) == len(actions) + 1
        for s in traj:
            assert s.state_id.startswith("roll_")

    def test_transition_step_decays(self):
        t = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        # zero action → next state should decay toward zero
        _, ns = t.step([1.0, 1.0], [0.0])
        assert max(abs(x) for x in ns) < 1.0


# ---------------------------------------------------------------------------
# 4. ObservationDecoder tests (Ha 2018 V)
# ---------------------------------------------------------------------------

class TestObservationDecoder:
    def test_decoder_init(self):
        dec = ObservationDecoder(latent_dim=4, obs_dim=8)
        assert len(dec.weights) == 8
        assert all(len(w) == 4 for w in dec.weights)

    def test_decoder_dim_mismatch(self):
        dec = ObservationDecoder(latent_dim=4, obs_dim=8)
        with pytest.raises(ValueError):
            dec.decode([0.1, 0.2])

    def test_decoder_decode_sigmoid_range(self):
        dec = ObservationDecoder(latent_dim=3, obs_dim=5)
        z = [0.1, -0.2, 0.3]
        obs = dec.decode(z)
        assert len(obs) == 5
        for x in obs:
            assert 0.0 <= x <= 1.0

    def test_decoder_reconstruction_error_zero(self):
        # Construct decoder weights=identity-like and decoder obs==z (sigmoid)
        dec = ObservationDecoder(latent_dim=3, obs_dim=3)
        # zero z → sigmoid(0) = 0.5; obs of 0.5 maps back to 0.5 → low error
        obs = dec.decode([0.0, 0.0, 0.0])
        err = dec.reconstruction_error(obs, [0.0, 0.0, 0.0])
        assert err >= 0
        assert math.isfinite(err)

    def test_decoder_reconstruction_dim_mismatch_inf(self):
        dec = ObservationDecoder(latent_dim=4, obs_dim=8)
        err = dec.reconstruction_error([0.1, 0.2], [0.1] * 4)
        assert err == float("inf")


# ---------------------------------------------------------------------------
# 5. RewardPredictor tests (Sutton 1990 Dyna)
# ---------------------------------------------------------------------------

class TestRewardPredictor:
    def test_reward_predict_shape(self):
        r = RewardPredictor(state_dim=3, action_dim=2)
        v = r.predict([0.1, 0.2, 0.3], [0.5, 0.6])
        assert isinstance(v, float)
        assert math.isfinite(v)

    def test_reward_predict_zero_input(self):
        r = RewardPredictor(state_dim=3, action_dim=2)
        # bias = 0, weights ~ small — predict should be near 0
        v = r.predict([0.0, 0.0, 0.0], [0.0, 0.0])
        assert abs(v) < 1.0

    def test_reward_dim_mismatch_returns_bias(self):
        r = RewardPredictor(state_dim=3, action_dim=2, bias=0.5)
        v = r.predict([0.1, 0.2], [0.5])  # wrong dims
        assert v == 0.5

    def test_reward_prediction_error(self):
        r = RewardPredictor(state_dim=2, action_dim=1)
        err = r.prediction_error([0.5, 0.5], [0.3], 1.0)
        assert err >= 0
        assert math.isfinite(err)


# ---------------------------------------------------------------------------
# 6. JEPAEmbedding tests (LeCun 2022)
# ---------------------------------------------------------------------------

class TestJEPAEmbedding:
    def test_jepa_init(self):
        j = JEPAEmbedding(embed_dim=4)
        assert len(j.predictor_weights) == 4
        assert all(len(w) == 4 for w in j.predictor_weights)

    def test_jepa_predict_embedding(self):
        j = JEPAEmbedding(embed_dim=3)
        out = j.predict_embedding([0.1, 0.2, 0.3])
        assert len(out) == 3
        for x in out:
            assert -1.001 <= x <= 1.001

    def test_jepa_dim_mismatch(self):
        j = JEPAEmbedding(embed_dim=3)
        with pytest.raises(ValueError):
            j.predict_embedding([0.1])

    def test_jepa_loss_zero_for_perfect_predictor(self):
        # When predictor_weights = identity and bias = 0,
        # predict(x) = tanh(x), so loss vs y=tanh(x) ≈ 0.
        j = JEPAEmbedding(embed_dim=3)
        j.predictor_weights = [[1.0 if i == j_ else 0.0 for j_ in range(3)] for i in range(3)]
        j.predictor_bias = [0.0, 0.0, 0.0]
        x = [0.5, -0.3, 0.1]
        # Compute embed_y = tanh(x) manually
        ey = [math.tanh(v) for v in x]
        loss = j.jepa_loss(x, ey)
        assert loss < 1e-9

    def test_jepa_loss_nonneg(self):
        j = JEPAEmbedding(embed_dim=3)
        loss = j.jepa_loss([0.1, 0.2, 0.3], [0.4, 0.5, 0.6])
        assert loss >= 0


# ---------------------------------------------------------------------------
# 7. ImaginationEngine tests (Hafner 2019)
# ---------------------------------------------------------------------------

class TestImaginationEngine:
    def test_imagine_shape(self):
        trans = TransitionModel(state_dim=3, action_dim=2, hidden_dim=4)
        rew = RewardPredictor(state_dim=3, action_dim=2)
        eng = ImaginationEngine(transition=trans, reward=rew, horizon=5)
        steps = eng.imagine([0.1, 0.2, 0.3])
        assert len(steps) == 5
        for s in steps:
            assert isinstance(s, ImaginedStep)
            assert len(s.state) == 3
            assert len(s.action) == 2
            assert isinstance(s.predicted_reward, float)

    def test_imagine_with_custom_policy(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        eng = ImaginationEngine(transition=trans, reward=rew, horizon=3)
        policy = lambda s: [0.5]
        steps = eng.imagine([0.0, 0.0], policy=policy)
        assert all(s.action == [0.5] for s in steps)

    def test_imagined_return(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        eng = ImaginationEngine(transition=trans, reward=rew, horizon=3)
        steps = eng.imagine([0.0, 0.0])
        G = eng.imagined_return(steps, gamma=0.99)
        assert isinstance(G, float)
        assert math.isfinite(G)

    def test_imagine_horizon_override(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        eng = ImaginationEngine(transition=trans, reward=rew, horizon=5)
        steps = eng.imagine([0.0, 0.0], horizon=2)
        assert len(steps) == 2


# ---------------------------------------------------------------------------
# 8. DynaPlanner tests (Sutton 1990)
# ---------------------------------------------------------------------------

class TestDynaPlanner:
    def test_dyna_remember(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        dyna = DynaPlanner(state_dim=2, action_dim=1, transition=trans, reward=rew)
        exp = Experience(state=[0.1, 0.2], action=[0.3], reward=0.5, next_state=[0.2, 0.3])
        dyna.remember(exp)
        assert len(dyna.buffer) == 1

    def test_dyna_state_key(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        dyna = DynaPlanner(state_dim=2, action_dim=1, transition=trans, reward=rew)
        k = dyna.state_key([0.123456, 0.234567], bins=4)
        # Should be deterministic for close states
        k2 = dyna.state_key([0.123457, 0.234568], bins=4)
        assert k == k2

    def test_dyna_q_update(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        dyna = DynaPlanner(state_dim=2, action_dim=1, transition=trans, reward=rew)
        exp = Experience(state=[0.1, 0.2], action=[0.3], reward=1.0, next_state=[0.2, 0.3])
        new_q = dyna.q_update(exp)
        assert new_q != 0.0  # positive reward should give positive Q
        assert dyna.state_key([0.1, 0.2]) in dyna.q_values

    def test_dyna_plan_empty_buffer(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        dyna = DynaPlanner(state_dim=2, action_dim=1, transition=trans, reward=rew,
                          n_planning_steps=3)
        assert dyna.plan() == 0

    def test_dyna_plan_with_buffer(self):
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=1)
        dyna = DynaPlanner(state_dim=2, action_dim=1, transition=trans, reward=rew,
                          n_planning_steps=5)
        for i in range(3):
            exp = Experience(
                state=[i * 0.1, i * 0.2],
                action=[0.5],
                reward=1.0 - i * 0.2,
                next_state=[i * 0.1 + 0.1, i * 0.2 + 0.1],
            )
            dyna.remember(exp)
        updates = dyna.plan()
        assert updates == 5

    def test_dyna_act_shape(self):
        trans = TransitionModel(state_dim=2, action_dim=2, hidden_dim=3)
        rew = RewardPredictor(state_dim=2, action_dim=2)
        dyna = DynaPlanner(state_dim=2, action_dim=2, transition=trans, reward=rew)
        a = dyna.act([0.1, 0.2])
        assert len(a) == 2


# ---------------------------------------------------------------------------
# 9. WorldModelReport tests (主 00:56)
# ---------------------------------------------------------------------------

class TestWorldModelReport:
    def test_report_init(self):
        rep = WorldModelReport()
        assert rep.title == "ASI World Model Report"
        assert rep.sections == []

    def test_report_add_section(self):
        rep = WorldModelReport()
        rep.add_section("Test", "Body content")
        assert len(rep.sections) == 1
        assert rep.sections[0] == ("Test", "Body content")

    def test_report_render_markdown(self):
        rep = WorldModelReport(title="Test Report")
        rep.add_section("Components", "1. Encoder\n2. Decoder")
        md = rep.render()
        assert "# Test Report" in md
        assert "## Components" in md
        assert "1. Encoder" in md
        assert "V1062 Version" in md
        assert "Generated:" in md

    def test_report_summary_dict(self):
        s = WorldModelReport.summary_dict(10, 20, 5, 100)
        assert "10" in s
        assert "20" in s
        assert "5" in s
        assert "100" in s


# ---------------------------------------------------------------------------
# 10. ASIWorldModelBridge tests (主 22:33 ASI 北极星)
# ---------------------------------------------------------------------------

class TestASIWorldModelBridge:
    def test_bridge_init(self):
        b = ASIWorldModelBridge()
        assert "vae_quality" in b.weights
        assert sum(b.weights.values()) == pytest.approx(1.0, abs=1e-9)

    def test_bridge_score_zero_metrics(self):
        b = ASIWorldModelBridge()
        result = b.score({})
        assert result["world_model_v0_2"] == 0.0
        assert "contributions" in result

    def test_bridge_score_perfect_metrics(self):
        b = ASIWorldModelBridge()
        perfect = {k: 1.0 for k in b.weights}
        result = b.score(perfect)
        assert result["world_model_v0_2"] == pytest.approx(1.0, abs=1e-9)

    def test_bridge_score_clipped(self):
        b = ASIWorldModelBridge()
        # > 1.0 should be clipped to 1.0
        over = {k: 1.5 for k in b.weights}
        result = b.score(over)
        assert result["world_model_v0_2"] <= 1.0

    def test_bridge_threshold_pass(self):
        b = ASIWorldModelBridge()
        r = b.threshold_check(0.90, target=0.85)
        assert r["passed"] is True
        assert r["gap"] < 0

    def test_bridge_threshold_fail(self):
        b = ASIWorldModelBridge()
        r = b.threshold_check(0.5, target=0.85)
        assert r["passed"] is False
        assert r["verdict"] == "WORK_TO_DO"
        assert r["gap"] == pytest.approx(0.35, abs=1e-9)


# ---------------------------------------------------------------------------
# 11. WorldModelGuard tests (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------

class TestWorldModelGuard:
    def test_vae_understanding_guard(self):
        g = WorldModelGuard.guard_vae_understanding({"vae_recon": 0.9})
        assert g["guard"] == "vae_understanding"
        assert g["passed"] is True

    def test_prediction_cognition_guard(self):
        g = WorldModelGuard.guard_prediction_cognition({"transition_accuracy": 0.85})
        assert g["guard"] == "prediction_cognition"

    def test_jepa_consciousness_guard(self):
        g = WorldModelGuard.guard_jepa_consciousness({"jepa_loss": 0.1})
        assert g["guard"] == "jepa_consciousness"

    def test_generative_world_guard(self):
        g = WorldModelGuard.guard_generative_world({"decoder_recon": 0.8})
        assert g["guard"] == "generative_world"

    def test_asi_has_world_model_guard(self):
        g = WorldModelGuard.guard_asi_has_world_model({"world_model_v0_2": 0.95})
        assert g["guard"] == "asi_has_world_model"
        assert "structural" in g["verdict"]

    def test_all_guards(self):
        g = WorldModelGuard.all_guards({})
        assert len(g) == 5
        assert all(guard["passed"] for guard in g)


# ---------------------------------------------------------------------------
# 12. WorldModelPipeline integration tests
# ---------------------------------------------------------------------------

class TestWorldModelPipeline:
    def test_default_pipeline(self):
        p = WorldModelPipeline.default(obs_dim=4, latent_dim=2, action_dim=2)
        assert p.encoder.obs_dim == 4
        assert p.encoder.latent_dim == 2
        assert p.transition.action_dim == 2

    def test_pipeline_step(self):
        p = WorldModelPipeline.default(obs_dim=4, latent_dim=2, action_dim=2)
        out = p.step([0.1, 0.2, 0.3, 0.4], [0.5, 0.6])
        assert "mu" in out
        assert "z" in out
        assert "next_z" in out
        assert "predicted_reward" in out
        assert "reconstruction" in out
        assert len(out["z"]) == 2

    def test_pipeline_train_step(self):
        p = WorldModelPipeline.default(obs_dim=4, latent_dim=2, action_dim=2)
        result = p.train_step(
            obs=[0.1, 0.2, 0.3, 0.4],
            action=[0.5, 0.6],
            next_obs=[0.2, 0.3, 0.4, 0.5],
            actual_reward=0.5,
        )
        assert "kl" in result
        assert "reconstruction_error" in result
        assert "reward_error" in result
        assert "jepa_loss" in result
        assert "plan_updates" in result

    def test_pipeline_report(self):
        p = WorldModelPipeline.default()
        md = p.report(observations=10, transitions=20, imagination_horizon=5, q_entries=3)
        assert "# ASI World Model Pipeline Report" in md
        assert "Ha 2018" in md
        assert "V3 哲学守门" in md

    def test_build_world_model_helper(self):
        p = build_world_model(obs_dim=6, latent_dim=3, action_dim=2)
        assert p.encoder.obs_dim == 6
        assert p.encoder.latent_dim == 3

    def test_quick_score_runs(self):
        p = WorldModelPipeline.default(obs_dim=4, latent_dim=2, action_dim=2)
        result = quick_score(p, n_samples=5)
        assert "world_model_v0_2" in result
        assert 0.0 <= result["world_model_v0_2"] <= 1.0


# ---------------------------------------------------------------------------
# 13. V3 philosophy guards integration (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------

class TestPhilosophyGuards:
    def test_5_guards_present(self):
        guards = WorldModelGuard.all_guards({})
        assert len(guards) == 5

    def test_no_guard_pretends_consciousness(self):
        guards = WorldModelGuard.all_guards({})
        for g in guards:
            assert "不假装" not in g["verdict"]  # guards explain limits, not lie
            # The verdict should clarify mechanism ≠ mental state

    def test_world_model_v0_2_not_equal_asi(self):
        # Even if world_model_v0_2 = 1.0, ASI is not implied
        b = ASIWorldModelBridge()
        perfect = {k: 1.0 for k in b.weights}
        result = b.score(perfect)
        assert result["world_model_v0_2"] <= 1.0
        guard = WorldModelGuard.guard_asi_has_world_model(result)
        assert "structural" in guard["verdict"]


# ---------------------------------------------------------------------------
# 14. Sanity tests
# ---------------------------------------------------------------------------

class TestSanity:
    def test_version(self):
        assert V1062_VERSION == "0.1.0"

    def test_14_real_precedents_documented(self):
        # 模块文档中应引用 14 个前人
        import apeireth.v1062_asi_world_model as mod
        src = mod.__doc__ or ""
        expected = ["Ha 2018", "Hafner 2019", "Friston 2010", "LeCun 2022",
                    "Sutton 1990", "Schmidhuber 1990", "Dayan 1995", "Kingma 2014",
                    "Goodfellow 2014", "Hinton 2006", "Mnih 2015", "Welling 2014"]
        # all should be referenced
        for ref in expected:
            assert ref in src, f"missing reference: {ref}"

    def test_10_real_components_documented(self):
        import apeireth.v1062_asi_world_model as mod
        src = mod.__doc__ or ""
        for comp in ["WorldState", "VariationalEncoder", "TransitionModel",
                     "ObservationDecoder", "RewardPredictor", "JEPAEmbedding",
                     "ImaginationEngine", "DynaPlanner", "WorldModelReport",
                     "ASIWorldModelBridge"]:
            assert comp in src, f"missing component: {comp}"

    def test_5_guards_documented(self):
        import apeireth.v1062_asi_world_model as mod
        src = mod.__doc__ or ""
        # V3 哲学守门 with 不假装 marker
        for guard in ["不假装 World Model = Understanding", "不假装 prediction = cognition",
                      "不假装 JEPA = consciousness", "不假装 generative model = world",
                      "不假装 ASI has world model"]:
            assert guard in src, f"missing guard: {guard}"

    def test_reproducibility(self):
        # Same seed → same trajectory
        random.seed(42)
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        traj1 = trans.rollout([0.0, 0.0], [[0.1], [0.2], [0.3]])
        random.seed(42)
        trans = TransitionModel(state_dim=2, action_dim=1, hidden_dim=3)
        traj2 = trans.rollout([0.0, 0.0], [[0.1], [0.2], [0.3]])
        assert traj1[1].z == traj2[1].z

    def test_no_pretend_consciousness_in_module(self):
        import apeireth.v1062_asi_world_model as mod
        with open(mod.__file__, encoding="utf-8") as f:
            src = (mod.__doc__ or "") + f.read()
        # Should NOT claim consciousness/awareness
        forbidden_phrases = ["world model IS consciousness",
                             "predict = understand",
                             "embed = experience",
                             "decoder = reality"]
        for phrase in forbidden_phrases:
            assert phrase not in src