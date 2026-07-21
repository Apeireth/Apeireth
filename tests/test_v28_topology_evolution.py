"""v28_topology_evolution.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v28_topology_evolution import (
    V28_VERSION, IntegrationCycleResult, V28TopologyEvolutionIntegration,
)


class TestV28:
    def test_init(self):
        i = V28TopologyEvolutionIntegration()
        assert i.cycles == []
        assert i.topology.nodes == {}

    def test_bootstrap_v2_positions(self):
        i = V28TopologyEvolutionIntegration()
        positions = i.bootstrap_v2_positions()
        assert len(positions) == 5

    def test_run_cycle(self):
        i = V28TopologyEvolutionIntegration()
        i.bootstrap_v2_positions()
        c = i.run_cycle()
        assert c.topology_n_nodes == 5
        assert c.topology_klein_index > 0

    def test_run_n_cycles(self):
        i = V28TopologyEvolutionIntegration()
        i.bootstrap_v2_positions()
        cycles = i.run_n_cycles(n=3)
        assert len(cycles) == 3

    def test_stats(self):
        i = V28TopologyEvolutionIntegration()
        i.bootstrap_v2_positions()
        i.run_n_cycles(n=2)
        stats = i.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])