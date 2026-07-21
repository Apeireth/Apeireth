"""v77_planning_htn.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v77_planning_htn import (
    V77_VERSION, HTNTask, Plan, V77PlanningHTN,
)


class TestV77:
    def test_init(self):
        p = V77PlanningHTN()
        assert p.tasks == {}

    def test_add_task_primitive(self):
        p = V77PlanningHTN()
        tid = p.add_task("test", is_primitive=True)
        assert tid in p.tasks

    def test_add_task_compound(self):
        p = V77PlanningHTN()
        tid = p.add_task("test", is_primitive=False)
        assert p.tasks[tid].is_primitive is False

    def test_plan_task_primitive(self):
        p = V77PlanningHTN()
        tid = p.add_task("test", is_primitive=True)
        plan = p.plan_task(tid)
        assert plan.total_steps == 1
        assert plan.is_valid

    def test_plan_task_compound(self):
        p = V77PlanningHTN()
        root = p.add_task("root", is_primitive=False)
        s1 = p.add_task("s1", is_primitive=True)
        s2 = p.add_task("s2", is_primitive=True)
        p.tasks[root].subtasks = [s1, s2]
        plan = p.plan_task(root)
        assert plan.total_steps == 3

    def test_plan_task_unknown(self):
        p = V77PlanningHTN()
        plan = p.plan_task("unknown_task")
        assert plan.is_valid is False

    def test_n_plans(self):
        p = V77PlanningHTN()
        tid = p.add_task("test", is_primitive=True)
        p.plan_task(tid)
        p.plan_task(tid)
        assert p.n_plans() == 2

    def test_stats(self):
        p = V77PlanningHTN()
        tid = p.add_task("test", is_primitive=True)
        p.plan_task(tid)
        stats = p.stats()
        assert stats["n_plans"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])