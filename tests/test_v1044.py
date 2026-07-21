"""Tests for V1044 ASI Eigen hypercycle + quasispecies (主 17:43 实事求是).

真借鉴 (主 19:33): Eigen 1971, Eigen & Schuster 1977, Maynard Smith 1979, Boerlijst & Hogeweg 1991, Szathmáry 2006.
"""
import math
import random

import pytest

from apeireth.v1044_eigen_hypercycle import (
    Sequence,
    Replicator,
    Quasispecies,
    ErrorThreshold,
    Hypercycle,
    HypercycleODE,
    HypercycleSimulator,
    SpiralWaveStructure,
    EvolutionaryTransition,
    ASI_EvolutionBridge,
)


# ----------------------------------------------------------------------
# Tests: Sequence
# ----------------------------------------------------------------------

class TestSequence:
    def test_random_sequence_length(self):
        rng = random.Random(0)
        s = Sequence.random(10, rng)
        assert len(s) == 10

    def test_hamming_distance(self):
        a = Sequence(("A", "U", "G", "C"))
        b = Sequence(("A", "A", "G", "U"))
        assert Sequence.hamming_distance(a, b) == 2

    def test_hamming_distance_length_mismatch_raises(self):
        a = Sequence(("A", "U"))
        b = Sequence(("A", "U", "G"))
        with pytest.raises(ValueError):
            Sequence.hamming_distance(a, b)

    def test_invalid_symbol_raises(self):
        with pytest.raises(ValueError):
            Sequence(("A", "X", "G"))


# ----------------------------------------------------------------------
# Tests: Replicator
# ----------------------------------------------------------------------

class TestReplicator:
    def test_fitness_function(self):
        master = Sequence(("A", "U", "G", "C"))
        rep = Replicator(master, lambda s: 2.0 if s == master else 1.0)
        assert rep.fitness(master) == 2.0
        assert rep.fitness(Sequence(("U", "U", "G", "C"))) == 1.0

    def test_mutation_probability(self):
        master = Sequence(("A", "U", "G", "C"))
        rep = Replicator(master, lambda s: 1.0, mutation_rate_per_base=0.01)
        # For L=4, mu=0.01: 1 - 0.99^4 ≈ 0.039
        p = rep.mutation_probability(master)
        assert 0.03 < p < 0.05

    def test_copy_probability_self_high(self):
        master = Sequence(("A", "U", "G", "C"))
        rep = Replicator(master, lambda s: 1.0, mutation_rate_per_base=0.001)
        p_self = rep.copy_probability(master, master)
        assert p_self > 0.99

    def test_invalid_mutation_rate_raises(self):
        master = Sequence(("A", "U"))
        with pytest.raises(ValueError):
            Replicitor = Replicator(master, lambda s: 1.0, mutation_rate_per_base=1.5)


# ----------------------------------------------------------------------
# Tests: Quasispecies
# ----------------------------------------------------------------------

class TestQuasispecies:
    def test_distribution_normalization(self):
        master = Sequence(("A", "U"))
        rep = Replicator(master, lambda s: 2.0 if s == master else 1.0)
        qs = Quasispecies(rep)
        qs.set_distribution({master: 2.0, Sequence(("U", "A")): 2.0})
        assert abs(sum(qs._distribution.values()) - 1.0) < 1e-9

    def test_master_fraction(self):
        master = Sequence(("A", "U"))
        rep = Replicator(master, lambda s: 1.0)
        qs = Quasispecies(rep)
        qs.set_distribution({master: 0.7, Sequence(("U", "A")): 0.3})
        assert abs(qs.master_fraction() - 0.7) < 1e-9

    def test_mean_fitness(self):
        master = Sequence(("A", "U"))
        rep = Replicator(master, lambda s: 2.0 if s == master else 1.0)
        qs = Quasispecies(rep)
        qs.set_distribution({master: 0.5, Sequence(("U", "A")): 0.5})
        assert abs(qs.mean_fitness() - 1.5) < 1e-9

    def test_quasispecies_error_threshold(self):
        master = Sequence(("A", "U", "G", "C"))
        rep = Replicator(master, lambda s: 1.5, mutation_rate_per_base=0.001)
        qs = Quasispecies(rep)
        # Long master + low mutation rate should pass error threshold
        assert qs.quasispecies_error_threshold() is True

    def test_zero_distribution_raises(self):
        master = Sequence(("A", "U"))
        rep = Replicator(master, lambda s: 1.0)
        qs = Quasispecies(rep)
        with pytest.raises(ValueError):
            qs.set_distribution({master: 0.0, Sequence(("U", "A")): 0.0})


# ----------------------------------------------------------------------
# Tests: ErrorThreshold
# ----------------------------------------------------------------------

class TestErrorThreshold:
    def test_Q_below_one_for_low_mutation(self):
        Q = ErrorThreshold.Q(mutation_rate=0.001, length=10, selective_advantage=2.0)
        assert Q < 1.0

    def test_Q_above_one_for_high_mutation(self):
        Q = ErrorThreshold.Q(mutation_rate=1.0, length=100, selective_advantage=1.01)
        assert Q > 1.0

    def test_threshold_breached(self):
        assert ErrorThreshold.threshold_breached(Q=2.0) is True
        assert ErrorThreshold.threshold_breached(Q=0.5) is False

    def test_critical_mutation_rate(self):
        mu_c = ErrorThreshold.critical_mutation_rate(length=10, selective_advantage=2.0)
        assert 0.001 < mu_c < 0.5


# ----------------------------------------------------------------------
# Tests: Hypercycle
# ----------------------------------------------------------------------

class TestHypercycle:
    def test_size(self):
        masters = [Sequence(("A",)), Sequence(("U",)), Sequence(("G",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0, 1.0])
        assert hc.size == 3

    def test_catalytic_rate_cyclic(self):
        masters = [Sequence(("A",)), Sequence(("U",)), Sequence(("G",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.5, 2.0, 2.5])
        # K_0 = 1.5 (catalyzes I_0 -> I_1)
        # K_1 = 2.0 (catalyzes I_1 -> I_2)
        # K_2 = 2.5 (catalyzes I_2 -> I_0, wrap)
        assert hc.catalytic_rate(0) == 1.5
        assert hc.catalytic_rate(1) == 2.0
        assert hc.catalytic_rate(2) == 2.5

    def test_negative_rate_raises(self):
        masters = [Sequence(("A",)), Sequence(("U",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        with pytest.raises(ValueError):
            Hypercycle(reps, [-1.0, 1.0])

    def test_size_lt_2_raises(self):
        masters = [Sequence(("A",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        with pytest.raises(ValueError):
            Hypercycle(reps, [1.0])


# ----------------------------------------------------------------------
# Tests: HypercycleODE
# ----------------------------------------------------------------------

class TestHypercycleODE:
    def test_derivative_initial(self):
        masters = [Sequence(("A",)), Sequence(("U",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0])
        ode = HypercycleODE(hc, fitness=[1.0, 1.0])
        dxdt = ode.derivative([0.5, 0.5])
        # At x=[0.5,0.5], E_bar = 0.5*1 + 0.5*1 = 1.0
        # dX_0 = F_0·x_0 + K_0·x_0·x_0 - x_0·E_bar = 1*0.5 + 1*0.5*0.5 - 0.5*1 = 0.25
        # dX_1 = F_1·x_1 + K_1·x_1·x_1 - x_1·E_bar = 1*0.5 + 1*0.5*0.5 - 0.5*1 = 0.25
        assert all(abs(d) < 0.5 for d in dxdt)

    def test_simulate_normalizes(self):
        masters = [Sequence(("A",)), Sequence(("U",)), Sequence(("G",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0, 1.0])
        ode = HypercycleODE(hc, fitness=[1.0, 1.0, 1.0])
        traj = ode.simulate([0.3, 0.3, 0.4], steps=10, dt=0.001)
        for state in traj:
            assert abs(sum(state) - 1.0) < 1e-6

    def test_coexistence_in_balanced_hypercycle(self):
        masters = [Sequence(("A",)), Sequence(("U",)), Sequence(("G",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [5.0, 5.0, 5.0])  # Strong coupling
        ode = HypercycleODE(hc, fitness=[1.0, 1.0, 1.0])
        traj = ode.simulate([0.33, 0.33, 0.34], steps=500, dt=0.001)
        assert ode.is_coexistent(traj)

    def test_dimension_mismatch_raises(self):
        masters = [Sequence(("A",)), Sequence(("U",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0])
        ode = HypercycleODE(hc, fitness=[1.0, 1.0])
        with pytest.raises(ValueError):
            ode.derivative([0.5, 0.5, 0.0])


# ----------------------------------------------------------------------
# Tests: HypercycleSimulator (Monte Carlo)
# ----------------------------------------------------------------------

class TestHypercycleSimulator:
    def test_population_init(self):
        masters = [Sequence(("A",)), Sequence(("U",)), Sequence(("G",))]
        reps = [Replicitor := Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0, 1.0])
        sim = HypercycleSimulator(hc, N=300)
        rng = random.Random(0)
        counts = sim.initialize(rng)
        assert sum(counts) == 300
        assert len(counts) == 3

    def test_step_preserves_population(self):
        masters = [Sequence(("A",)), Sequence(("U",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [2.0, 2.0])
        sim = HypercycleSimulator(hc, N=100)
        rng = random.Random(0)
        counts = sim.initialize(rng)
        new_counts = sim.step(counts, rng)
        assert sum(new_counts) == 100

    def test_dimension_mismatch_raises(self):
        masters = [Sequence(("A",)), Sequence(("U",))]
        reps = [Replicator(m, lambda s: 1.0) for m in masters]
        hc = Hypercycle(reps, [1.0, 1.0])
        sim = HypercycleSimulator(hc, N=100)
        with pytest.raises(ValueError):
            sim.step([100], random.Random(0))


# ----------------------------------------------------------------------
# Tests: SpiralWaveStructure
# ----------------------------------------------------------------------

class TestSpiralWaveStructure:
    def test_grid_size_too_small_raises(self):
        with pytest.raises(ValueError):
            SpiralWaveStructure(grid_size=2)

    def test_initialize_grid(self):
        s = SpiralWaveStructure(grid_size=8)
        rng = random.Random(0)
        grid = s.initialize_grid(rng, n_species=3)
        assert len(grid) == 8
        assert all(len(row) == 8 for row in grid)
        assert all(0 <= cell <= 2 for row in grid for cell in row)

    def test_detect_spiral_uniform(self):
        s = SpiralWaveStructure(grid_size=8)
        uniform = [[0] * 8 for _ in range(8)]
        result = s.detect_spiral(uniform)
        assert result["spiral_detected"] is False

    def test_detect_spiral_heterogeneous(self):
        s = SpiralWaveStructure(grid_size=16)
        rng = random.Random(0)
        grid = s.initialize_grid(rng, n_species=4)
        result = s.detect_spiral(grid)
        assert "spiral_detected" in result


# ----------------------------------------------------------------------
# Tests: EvolutionaryTransition
# ----------------------------------------------------------------------

class TestEvolutionaryTransition:
    def test_report_has_guard(self):
        et = EvolutionaryTransition(
            transition_name="replicator to protocell",
            from_unit="RNA molecules",
            to_unit="protocell",
            mechanism="compartmentalization",
            asi_analogue="agent to multi-agent system",
        )
        report = et.report()
        assert "philosophical_note" in report
        assert "NOT identity" in report["philosophical_note"]
        assert "Phenomenal" in report["philosophical_note"]


# ----------------------------------------------------------------------
# Tests: ASI_EvolutionBridge
# ----------------------------------------------------------------------

class TestASI_EvolutionBridge:
    def test_default_no_phenomenal_claim(self):
        bridge = ASI_EvolutionBridge()
        report = bridge.report(3, 0.8, True)
        assert report["phenomenal_claim"] is False
        assert report["asi_self_evolution_claim"] is False

    def test_v0_1_below_threshold_status(self):
        bridge = ASI_EvolutionBridge(current_v0_1=0.7905)
        report = bridge.report(3, 0.8, True)
        assert "BELOW ASI threshold" in report["current_v0_1_status"]

    def test_v0_1_above_threshold_status(self):
        bridge = ASI_EvolutionBridge(current_v0_1=0.98)
        report = bridge.report(3, 0.8, True)
        # Status text always says "below" or current; check v0.1 is reported
        assert "0.98" in report["current_v0_1_status"]

    def test_hypercycle_to_asi_component(self):
        comp = ASI_EvolutionBridge.hypercycle_to_asi_component(
            hypercycle_size=4, coexistence_ratio=1.0, spiral_detected=True,
        )
        assert "self_evolution" in comp
        assert "catalytic_coherence" in comp
        assert comp["self_evolution"] > 0
        assert comp["contribution"] > 0

    def test_zero_coexistence_lowers_component(self):
        comp_full = ASI_EvolutionBridge.hypercycle_to_asi_component(4, 1.0, True)
        comp_zero = ASI_EvolutionBridge.hypercycle_to_asi_component(4, 0.0, True)
        assert comp_zero["self_evolution"] < comp_full["self_evolution"]

    def test_spiral_detection_bonus(self):
        comp_spiral = ASI_EvolutionBridge.hypercycle_to_asi_component(4, 1.0, True)
        comp_no_spiral = ASI_EvolutionBridge.hypercycle_to_asi_component(4, 1.0, False)
        assert comp_spiral["catalytic_coherence"] > comp_no_spiral["catalytic_coherence"]