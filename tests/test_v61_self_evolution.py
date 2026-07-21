"""v61_self_evolution.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v61_self_evolution import (
    V61_VERSION, EvolutionCycle, V61SelfEvolution,
)


class TestV61:
    def test_init(self):
        core = V61SelfEvolution()
        assert core.cycles == []

    def test_bootstrap(self):
        core = V61SelfEvolution()
        core.bootstrap()
        assert core.four_paradigm.cognitive.n_atoms() >= 5

    def test_run_evolution_cycle(self):
        core = V61SelfEvolution()
        core.bootstrap()
        cycle = core.run_evolution_cycle(generation=1)
        assert cycle.generation == 1
        assert isinstance(cycle.improvement, float)

    def test_run_n_cycles(self):
        core = V61SelfEvolution()
        core.bootstrap()
        cycles = core.run_n_cycles(n=3)
        assert len(cycles) == 3
        assert core.n_cycles() == 3

    def test_average_improvement(self):
        core = V61SelfEvolution()
        core.bootstrap()
        core.run_n_cycles(n=2)
        avg = core.average_improvement()
        assert isinstance(avg, float)

    def test_stats(self):
        core = V61SelfEvolution()
        core.bootstrap()
        core.run_n_cycles(n=2)
        stats = core.stats()
        assert stats["n_cycles"] == 2


if __name__ == "__main__":
    pytest.main([__file__, "-v"])