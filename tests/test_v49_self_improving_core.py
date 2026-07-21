"""v49_self_improving_core.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v49_self_improving_core import (
    V49_VERSION, DGMArchive, BanditArm, ucb1, Meta2Modification,
    V49SelfImprovingCore,
)


class TestV49Helpers:
    def test_ucb1_no_visits(self):
        arm = BanditArm(arm_id="x", parent_id="y")
        assert ucb1(arm, 10) == float("inf")

    def test_ucb1_with_visits(self):
        arm = BanditArm(arm_id="x", parent_id="y", n_visits=5, total_reward=2.5)
        score = ucb1(arm, 10)
        assert score > 0


class TestV49:
    def test_init(self):
        c = V49SelfImprovingCore()
        assert c.archive.generation == 0

    def test_add_agent_to_archive(self):
        c = V49SelfImprovingCore()
        c.add_agent_to_archive("a1", {"fitness": 0.5})
        assert "a1" in c.archive.agents

    def test_add_arm(self):
        c = V49SelfImprovingCore()
        arm_id = c.add_arm("parent")
        assert arm_id in c.arms

    def test_select_ucb1_empty(self):
        c = V49SelfImprovingCore()
        assert c.select_ucb1() is None

    def test_select_ucb1(self):
        c = V49SelfImprovingCore()
        a1 = c.add_arm("p")
        a2 = c.add_arm("p")
        c.record_reward(a1, 0.8)
        c.record_reward(a2, 0.5)
        selected = c.select_ucb1()
        assert selected is not None

    def test_record_reward(self):
        c = V49SelfImprovingCore()
        arm_id = c.add_arm("p")
        c.record_reward(arm_id, 0.7)
        assert c.arms[arm_id].n_visits == 1
        assert abs(c.arms[arm_id].fitness - 0.7) < 0.01

    def test_meta2_modify(self):
        c = V49SelfImprovingCore()
        mod_id = c.meta2_modify("target", "new_proc")
        assert mod_id in [m.modification_id for m in c.modifications]

    def test_meta2_chain(self):
        c = V49SelfImprovingCore()
        m1 = c.meta2_modify("t", "p1")
        m2 = c.meta2_modify(m1, "p2", parent_mod_id=m1)
        assert c.modifications[1].parent_mod_id == m1

    def test_stats(self):
        c = V49SelfImprovingCore()
        c.add_agent_to_archive("a1", {})
        c.add_arm("p")
        c.meta2_modify("t", "p")
        stats = c.stats()
        assert stats["n_agents_in_archive"] == 1
        assert stats["n_arms"] == 1
        assert stats["n_modifications"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])