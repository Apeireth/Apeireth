"""v75_multi_agent.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v75_multi_agent import (
    V75_VERSION, Agent, CoordinationResult, V75MultiAgent,
)


class TestV75:
    def test_init(self):
        ma = V75MultiAgent()
        assert ma.agents == {}

    def test_spawn_agent(self):
        ma = V75MultiAgent()
        aid = ma.spawn_agent("test_agent", "worker")
        assert aid in ma.agents

    def test_spawn_agent_with_parent(self):
        ma = V75MultiAgent()
        parent = ma.spawn_agent("parent", "coordinator")
        child = ma.spawn_agent("child", "worker", parent_id=parent)
        assert ma.agents[child].parent_id == parent

    def test_coordinate(self):
        ma = V75MultiAgent()
        a1 = ma.spawn_agent("a1", "worker")
        a2 = ma.spawn_agent("a2", "worker")
        result = ma.coordinate([a1, a2], n_rounds=2)
        assert result.coordination_rounds == 2
        assert ma.n_coordinations() == 1

    def test_n_agents(self):
        ma = V75MultiAgent()
        ma.spawn_agent("a1")
        ma.spawn_agent("a2")
        ma.spawn_agent("a3")
        assert ma.n_agents() == 3

    def test_stats(self):
        ma = V75MultiAgent()
        ma.spawn_agent("a")
        stats = ma.stats()
        assert stats["n_agents"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])