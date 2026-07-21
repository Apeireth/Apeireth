"""v69_simulation_engine.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v69_simulation_engine import (
    V69_VERSION, SimulationStep, SimulationResult, V69SimulationEngine,
)


class TestV69:
    def test_init(self):
        sim = V69SimulationEngine()
        assert sim.simulations == []

    def test_run_simulation(self):
        sim = V69SimulationEngine()
        r = sim.run_simulation(
            initial_state="s0", actions=["a1", "a2"], reward_fn=lambda s, a: 0.5,
        )
        assert r.n_steps == 2
        assert r.total_reward == 1.0
        assert r.average_free_energy > 0

    def test_run_simulation_default_reward(self):
        sim = V69SimulationEngine()
        r = sim.run_simulation(initial_state="s0", actions=["a1"])
        assert r.total_reward == 1.0  # default

    def test_n_simulations(self):
        sim = V69SimulationEngine()
        sim.run_simulation(initial_state="s0", actions=["a1"])
        assert sim.n_simulations() == 1

    def test_stats(self):
        sim = V69SimulationEngine()
        sim.run_simulation(initial_state="s0", actions=["a1"])
        stats = sim.stats()
        assert stats["n_simulations"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])