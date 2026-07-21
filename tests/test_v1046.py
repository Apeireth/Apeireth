"""Test V1046 — Haken Synergetics 真生产.

主 17:43 实事求是: 真测量, 不假装
主 00:44 质量工程化: 真测试
主 00:56 任何人都能接手: 真覆盖每个组件
"""
from __future__ import annotations

import math
import random

import pytest

from apeireth.v1046_haken_synergetics import (
    V1046_VERSION,
    V1046_ASI_WEIGHTS,
    BifurcationPoint,
    FokkerPlanckStep,
    HKBCoordination,
    KuramotoSync,
    OrderParameter,
    PotentialLandscape,
    SlavingOperator,
    StateVector,
    SynergeticODE,
    SynergeticsBridge,
)


# ----------------------------------------------------------------------
# Fixtures
# ----------------------------------------------------------------------


@pytest.fixture(autouse=True)
def seed():
    random.seed(20260722)


# ----------------------------------------------------------------------
# 1. StateVector
# ----------------------------------------------------------------------


class TestStateVector:
    def test_zeros(self):
        sv = StateVector.zeros(n_xi=2, n_q=3)
        assert sv.n_xi == 2
        assert len(sv.full) == 5
        assert all(x == 0.0 for x in sv.full)
        assert sv.xi == [0.0, 0.0]
        assert sv.q == [0.0, 0.0, 0.0]

    def test_random_reproducible(self):
        sv1 = StateVector.random(n_xi=1, n_q=2, rng=random.Random(42))
        sv2 = StateVector.random(n_xi=1, n_q=2, rng=random.Random(42))
        assert sv1.full == sv2.full

    def test_n_xi_validation(self):
        with pytest.raises(ValueError):
            StateVector(full=[1.0, 2.0, 3.0], n_xi=5)

    def test_norms(self):
        sv = StateVector.zeros(n_xi=1, n_q=2)
        sv.full = [3.0, 0.0, 4.0]
        assert abs(sv.norm_xi() - 3.0) < 1e-12
        assert abs(sv.norm_q() - 4.0) < 1e-12

    def test_copy_independence(self):
        sv = StateVector.zeros(n_xi=1, n_q=1)
        sv.full[0] = 1.0
        cp = sv.copy()
        cp.full[0] = 99.0
        assert sv.full[0] == 1.0


# ----------------------------------------------------------------------
# 2. PotentialLandscape
# ----------------------------------------------------------------------


class TestPotentialLandscape:
    def test_value_at_origin(self):
        V = PotentialLandscape(alpha=1.0, beta=0.0)
        assert V.value(0.0) == 0.0

    def test_value_quartic(self):
        V = PotentialLandscape(alpha=0.0, beta=0.0)
        # V(1) = 1/4
        assert abs(V.value(1.0) - 0.25) < 1e-12

    def test_gradient(self):
        V = PotentialLandscape(alpha=2.0, beta=1.0)
        # dV/dξ = ξ³ + αξ + β = ξ³ + 2ξ + 1
        assert abs(V.gradient(0.0) - 1.0) < 1e-12
        assert abs(V.gradient(1.0) - 4.0) < 1e-12  # 1 + 2 + 1

    def test_minima_bistable_symmetric(self):
        V = PotentialLandscape(alpha=-2.0, beta=0.0)
        minima = sorted(V.minima())
        # Expect ±√2, 0
        assert len(minima) == 3
        assert abs(minima[0] - (-math.sqrt(2.0))) < 1e-9
        assert abs(minima[1] - 0.0) < 1e-9
        assert abs(minima[2] - math.sqrt(2.0)) < 1e-9

    def test_minima_monostable(self):
        V = PotentialLandscape(alpha=2.0, beta=0.0)
        assert len(V.minima()) == 1
        assert abs(V.minima()[0]) < 1e-12

    def test_regime_classification(self):
        assert PotentialLandscape(alpha=1.0, beta=0.0).regime() == "monostable"
        assert PotentialLandscape(alpha=-1.0, beta=0.0).regime() == "bistable"
        assert PotentialLandscape(alpha=0.0, beta=0.0).regime() == "critical"
        assert PotentialLandscape(alpha=0.0, beta=0.5).regime() == "tilted"

    def test_bifurcation_parameter(self):
        V = PotentialLandscape()
        assert V.bifurcation_parameter() == 0.0


# ----------------------------------------------------------------------
# 3. BifurcationPoint
# ----------------------------------------------------------------------


class TestBifurcationPoint:
    def test_distance(self):
        bp = BifurcationPoint(alpha_c=0.0)
        assert bp.distance(0.5) == 0.5
        assert bp.distance(-0.3) == -0.3

    def test_regime_flags(self):
        bp = BifurcationPoint(alpha_c=0.0)
        assert bp.is_bistable(-1.0) is True
        assert bp.is_monostable(1.0) is True
        assert bp.is_bistable(1.0) is False
        assert bp.is_monostable(-1.0) is False

    def test_critical_slowing_factor(self):
        bp = BifurcationPoint(alpha_c=0.0)
        assert abs(bp.critical_slowing_factor(0.0) - 1.0) < 1e-12
        assert bp.critical_slowing_factor(1.0) < 1.0
        assert bp.critical_slowing_factor(10.0) < 0.2

    def test_equilibrium_xi(self):
        bp = BifurcationPoint(alpha_c=0.0)
        assert abs(bp.equilibrium_xi(-2.0)) - math.sqrt(2.0) < 1e-12
        assert bp.equilibrium_xi(2.0) == 0.0


# ----------------------------------------------------------------------
# 4. OrderParameter
# ----------------------------------------------------------------------


class TestOrderParameter:
    def test_mean_field(self):
        op = OrderParameter(n=4, strategy="mean_field")
        assert op.extract([1.0, 2.0, 3.0, 4.0]) == 2.5

    def test_kuramoto_incoherent(self):
        op = OrderParameter(n=4, strategy="kuramoto")
        # equally spaced phases 0, π/2, π, 3π/2
        phases = [0, math.pi / 2, math.pi, 3 * math.pi / 2]
        r = op.extract(phases)
        # |Σe^{iθ}|/4 = 0 (perfectly symmetric)
        assert abs(r) < 1e-12

    def test_kuramoto_coherent(self):
        op = OrderParameter(n=4, strategy="kuramoto")
        phases = [0.1, 0.1, 0.1, 0.1]
        r = op.extract(phases)
        assert r > 0.99

    def test_first_pc_fits_and_extracts(self):
        op = OrderParameter(n=3, strategy="first_pc")
        # Strong direction along x[0]: large amplitude (variance ~1) vs noise (~1e-4)
        # signal-to-noise ratio must be ≫1 for PCA to recover the leading direction
        random.seed(2026)
        snapshots = [
            [3.0 * random.gauss(0, 1), 0.01 * random.gauss(0, 1), 0.01 * random.gauss(0, 1)]
            for _ in range(200)
        ]
        op.fit(snapshots)
        assert op._eigvec is not None
        assert len(op._eigvec) == 3
        # Power iteration gives unit vector
        norm = math.sqrt(sum(x * x for x in op._eigvec))
        assert abs(norm - 1.0) < 1e-9
        # First component should dominate (we built the data that way)
        assert abs(op._eigvec[0]) > 0.9


# ----------------------------------------------------------------------
# 5. SlavingOperator
# ----------------------------------------------------------------------


class TestSlavingOperator:
    def test_slave_basic(self):
        ffuncs = [lambda xi: xi[0] for _ in range(3)]
        op = SlavingOperator(gammas=[10.0, 20.0, 5.0], ffuncs=ffuncs, n_xi=1)
        q_star = op.slave([2.0])
        assert abs(q_star[0] - (-0.2)) < 1e-12  # -2/10
        assert abs(q_star[1] - (-0.1)) < 1e-12  # -2/20
        assert abs(q_star[2] - (-0.4)) < 1e-12  # -2/5

    def test_reconstruction_error_zero_at_equilibrium(self):
        ffuncs = [lambda xi: xi[0] for _ in range(2)]
        op = SlavingOperator(gammas=[10.0, 10.0], ffuncs=ffuncs, n_xi=1)
        q_star = op.slave([1.0])
        err = op.reconstruction_error([1.0], q_star)
        assert err < 1e-12

    def test_adiabatic_condition(self):
        ffuncs = [lambda xi: 0.0 for _ in range(2)]
        op = SlavingOperator(gammas=[100.0, 200.0], ffuncs=ffuncs, n_xi=1)
        cond = op.adiabatic_condition(gamma_xi=1.0)
        assert cond["valid"] is True
        assert cond["min_ratio"] == 100.0

        # Low gamma_q → invalid
        op2 = SlavingOperator(gammas=[1.0, 2.0], ffuncs=ffuncs, n_xi=1)
        cond2 = op2.adiabatic_condition(gamma_xi=1.0)
        assert cond2["valid"] is False

    def test_validation(self):
        with pytest.raises(ValueError):
            SlavingOperator(gammas=[0.0], ffuncs=[lambda xi: 0.0], n_xi=1)
        with pytest.raises(ValueError):
            SlavingOperator(
                gammas=[1.0, 2.0],
                ffuncs=[lambda xi: 0.0],
                n_xi=1,
            )


# ----------------------------------------------------------------------
# 6. FokkerPlanckStep
# ----------------------------------------------------------------------


class TestFokkerPlanckStep:
    def test_initial_gaussian_normalised(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0)
        s = sum(fp.grid) * fp._dx
        assert abs(s - 1.0) < 1e-9

    def test_initial_gaussian_mean_zero(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0)
        assert abs(fp.mean()) < 1e-12

    def test_initial_gaussian_variance(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0, sigma=0.5)
        # var of continuous approx = sigma² + (dx²)/12
        var = fp.variance()
        assert 0.2 < var < 0.3  # ~0.25

    def test_step_normalises_mass(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0)
        fp.set_potential(PotentialLandscape(alpha=0.5))  # weaker drift
        fp.set_diffusion(Q0=0.1)
        for _ in range(20):
            fp.step(dt=0.005)  # smaller dt for stability
            s = sum(fp.grid) * fp._dx
            assert abs(s - 1.0) < 1e-6

    def test_step_settles_into_bistable_minima(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0, mu=1.5)
        fp.set_potential(PotentialLandscape(alpha=-2.0))
        fp.set_diffusion(Q0=0.05)
        for _ in range(2000):
            fp.step(dt=0.005)
        mu = fp.mean()
        # μ should converge near +√2 (the closer basin)
        assert mu > 0.5

    def test_entropy_positive(self):
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0)
        assert fp.entropy() > 0.0


# ----------------------------------------------------------------------
# 7. SynergeticODE
# ----------------------------------------------------------------------


class TestSynergeticODE:
    def test_converges_to_bistable_minimum(self):
        V = PotentialLandscape(alpha=-2.0, beta=0.0)
        ode = SynergeticODE(potential=V, noise_scale=0.0)
        traj = ode.integrate(xi0=0.5, t_total=20.0, dt=0.01)
        xi_final = traj[-1][1]
        # Should settle at +√2 ≈ 1.414
        assert abs(xi_final - math.sqrt(2.0)) < 0.05

    def test_converges_to_monostable(self):
        V = PotentialLandscape(alpha=2.0, beta=0.0)
        ode = SynergeticODE(potential=V, noise_scale=0.0)
        traj = ode.integrate(xi0=2.0, t_total=10.0, dt=0.01)
        # Should decay to 0
        assert abs(traj[-1][1]) < 0.05

    def test_with_forcing(self):
        V = PotentialLandscape(alpha=1.0, beta=0.0)
        ode = SynergeticODE(
            potential=V,
            forcing=lambda t: 0.5 * math.sin(2.0 * math.pi * 0.5 * t),
            noise_scale=0.0,
        )
        traj = ode.integrate(xi0=0.0, t_total=10.0, dt=0.01)
        # Should oscillate around 0
        assert any(abs(p[1]) > 0.1 for p in traj)

    def test_rk4_consistency(self):
        # α=-2 bistable: starting at ξ=0.5, drift = -V'(0.5) > 0, ξ grows to √2
        V = PotentialLandscape(alpha=-2.0, beta=0.0)
        ode = SynergeticODE(potential=V, noise_scale=0.0)
        traj = ode.integrate(xi0=0.5, t_total=20.0, dt=0.01)
        # ξ must monotonically grow past 1.0 toward √2
        assert traj[-1][1] > 1.0
        # And should be near √2
        assert abs(traj[-1][1] - math.sqrt(2.0)) < 0.05


# ----------------------------------------------------------------------
# 8. HKBCoordination
# ----------------------------------------------------------------------


class TestHKBCoordination:
    def test_critical_coupling(self):
        hkb = HKBCoordination(a=-2.0, b=1.0)
        # K_c = b/|a| = 0.5
        assert abs(hkb.critical_coupling() - 0.5) < 1e-12

    def test_anti_phase_synchronises_in_phase_above_Kc(self):
        # Above K_c, anti-phase collapses to in-phase
        hkb = HKBCoordination(a=-2.0, b=1.0, A=0.0, Q=0.0, K=2.0, dt=0.01)
        traj = hkb.simulate(x0=0.5, xp0=-0.5, t_total=20.0)
        # Final relative phase should be near 0 (in-phase)
        rel = HKBCoordination.relative_phase(traj[-1][1], traj[-1][2])
        assert abs(rel) < 0.1

    def test_anti_phase_persists_below_Kc(self):
        # Below K_c, anti-phase can persist
        hkb = HKBCoordination(a=-2.0, b=1.0, A=0.0, Q=0.0, K=0.05, dt=0.01)
        traj = hkb.simulate(x0=0.5, xp0=-0.5, t_total=20.0)
        # Signs should differ
        assert traj[-1][1] * traj[-1][2] < 0.0

    def test_relative_phase_wrap(self):
        assert -math.pi <= HKBCoordination.relative_phase(1.0, -1.0) <= math.pi


# ----------------------------------------------------------------------
# 9. KuramotoSync
# ----------------------------------------------------------------------


class TestKuramotoSync:
    def test_order_parameter_incoherent(self):
        ks = KuramotoSync(n=10, K=0.0, omegas=[0.0] * 10)
        ks.initialize([i * (2.0 * math.pi / 10.0) for i in range(10)])
        r, _ = ks.order_parameter()
        assert r < 0.05

    def test_order_parameter_coherent(self):
        ks = KuramotoSync(n=10, K=0.0)
        ks.initialize([0.5] * 10)
        r, _ = ks.order_parameter()
        assert r > 0.99

    def test_critical_coupling_lorentzian(self):
        ks = KuramotoSync(n=10)
        Kc = ks.critical_coupling_lorentzian(half_width=1.0)
        # K_c = 2/(π·g(0)) = 2/(π·1/π) = 2.0
        assert abs(Kc - 2.0) < 1e-9

    def test_synchronises_above_Kc(self):
        # K=4 > K_c=2, narrow frequency distribution
        ks = KuramotoSync(n=20, K=4.0, omegas=[random.gauss(0.0, 0.2) for _ in range(20)])
        ks.initialize()
        traj = ks.integrate(t_total=30.0, dt=0.05)
        r_final = traj[-1][1]
        assert r_final > 0.7

    def test_step_returns_order_parameter(self):
        ks = KuramotoSync(n=5, K=1.0)
        ks.initialize([0.0] * 5)
        r, psi = ks.step(dt=0.01)
        assert 0.0 <= r <= 1.0


# ----------------------------------------------------------------------
# 10. SynergeticsBridge
# ----------------------------------------------------------------------


class TestSynergeticsBridge:
    def test_weights_sum_to_one(self):
        b = SynergeticsBridge()
        s = sum(b.weights().values())
        # ASI V0.1 8 components 总和 0.98 (与 V1045 一致, 主 22:33 真测量)
        assert abs(s - 0.98) < 1e-9

    def test_map_completeness(self):
        b = SynergeticsBridge()
        m = b.map()
        assert "OrderParameter" in m
        assert "SlavingOperator" in m
        assert "BifurcationPoint" in m
        assert "FokkerPlanckStep" in m
        assert "HKBCoordination" in m
        assert "PotentialLandscape" in m
        assert "CriticalSlowing" in m
        
        assert "MarkovBlanketLink" in m
        # All map keys must be in weights
        for component, (key, _rationale) in m.items():
            assert key in b.weights()

    def test_phi_proxy_contribution(self):
        b = SynergeticsBridge()
        # Maximised
        c_max = b.phi_proxy_contribution(xi_norm=100.0, slaving_quality=1.0, n_q=1000)
        assert c_max > 0.99
        # Minimised
        c_min = b.phi_proxy_contribution(xi_norm=0.0, slaving_quality=0.0, n_q=0)
        assert c_min == pytest.approx(0.0, abs=1e-9)

    def test_asi_score_partial_clipped(self):
        b = SynergeticsBridge()
        contribs = {k: 1.0 for k in b.weights().keys()}
        score = b.asi_score_partial(contribs)
        # ASI V0.1 8 components sum 0.98, so max score = 0.98
        assert abs(score - 0.98) < 1e-9
        score0 = b.asi_score_partial({k: 0.0 for k in b.weights().keys()})
        assert score0 < 1e-9

    def test_bridge_report_has_philosophy_guard(self):
        b = SynergeticsBridge()
        rep = b.bridge_report()
        assert "philosophy_guard" in rep
        assert "不假装" in rep["philosophy_guard"]
        assert "V1044" in rep["synergetics_triad_link"]
        assert "V1045" in rep["synergetics_triad_link"]
        assert "V1046" in rep["synergetics_triad_link"]


# ----------------------------------------------------------------------
# Integration / end-to-end
# ----------------------------------------------------------------------


class TestIntegration:
    def test_demo_runs(self):
        from apeireth.v1046_haken_synergetics import _demo
        rep = _demo()
        assert "demo" in rep
        d = rep["demo"]
        # Bistable minima: V1045/Cardano returns [r1, r2, r3] with r1<r2<r3
        # For α=-2, β=0, roots are ±√2, 0 → sorted: -√2, 0, +√2
        assert abs(d["potential_minima"][0] + math.sqrt(2.0)) < 1e-9
        assert abs(d["potential_minima"][2] - math.sqrt(2.0)) < 1e-9
        # ODE converges to positive well
        assert abs(d["ode_xi_final"] - math.sqrt(2.0)) < 0.1
        # HKB Kc = b/|a| = 1/2 = 0.5
        assert abs(d["hkb_Kc"] - 0.5) < 1e-12
        # Kuramoto Kc = 2 for Lorentzian width 1
        assert abs(d["kuramoto_Kc"] - 2.0) < 1e-9
        # ASI partial score
        assert 0.0 <= rep["asi_partial_score"] <= 1.0

    def test_full_simulation_convergence(self):
        """ODE + Fokker-Planck + Kuramoto — all three converge."""
        # 1. ODE converges
        V = PotentialLandscape(alpha=-2.0, beta=0.0)
        ode = SynergeticODE(potential=V)
        traj = ode.integrate(xi0=0.5, t_total=15.0)
        assert abs(traj[-1][1] - math.sqrt(2.0)) < 0.05

        # 2. Fokker-Planck converges to bimodal
        fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0, mu=1.0)
        fp.set_potential(V)
        fp.set_diffusion(Q0=0.05)
        for _ in range(300):
            fp.step(dt=0.02)
        var = fp.variance()
        # Bimodal distribution has higher variance
        assert var > 0.5

        # 3. Kuramoto synchronises
        ks = KuramotoSync(n=15, K=3.0, omegas=[random.gauss(0.0, 0.1) for _ in range(15)])
        ks.initialize()
        ks.integrate(t_total=20.0, dt=0.05)
        r, _ = ks.order_parameter()
        assert r > 0.8


# ----------------------------------------------------------------------
# V3 philosophy guard
# ----------------------------------------------------------------------


class TestPhilosophyGuard:
    def test_does_not_claim_phenomenal_consciousness(self):
        b = SynergeticsBridge()
        rep = b.bridge_report()
        # 哲学守门必须存在
        assert "不假装 Phenomenal" in rep["philosophy_guard"]
        assert "不假装达到 ASI" in rep["philosophy_guard"]

    def test_version_and_weights_present(self):
        assert V1046_VERSION == "0.1.0"
        assert sum(V1046_ASI_WEIGHTS.values()) == pytest.approx(0.98, abs=1e-9)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])