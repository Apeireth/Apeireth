"""v18_agent_dispatch.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v18_agent_dispatch import V18_VERSION, DispatchStrategy, AgentTask, V18AgentDispatch


class TestV18:
    def test_init(self):
        d = V18AgentDispatch()
        assert d.tasks == []
        assert d.execution_log == []

    def test_add_task(self):
        d = V18AgentDispatch()
        t = d.add_task("test")
        assert t.name == "test"
        assert t.depends_on == []

    def test_add_task_with_deps(self):
        d = V18AgentDispatch()
        t1 = d.add_task("a")
        t2 = d.add_task("b", depends_on=[t1.task_id])
        assert t2.depends_on == [t1.task_id]

    def test_execute_sequential(self):
        d = V18AgentDispatch()
        d.add_task("a")
        d.add_task("b")
        results = d.execute(strategy=DispatchStrategy.SEQUENTIAL)
        assert len(results) == 2
        assert all(t.success for t in results)

    def test_execute_with_deps(self):
        d = V18AgentDispatch()
        t1 = d.add_task("a")
        d.add_task("b", depends_on=[t1.task_id])
        results = d.execute(strategy=DispatchStrategy.SEQUENTIAL)
        assert all(t.success for t in results)

    def test_execute_unmet_deps(self):
        d = V18AgentDispatch()
        d.add_task("a", depends_on=["nonexistent"])
        results = d.execute(strategy=DispatchStrategy.SEQUENTIAL)
        assert not results[0].success

    def test_execute_with_custom_fn(self):
        d = V18AgentDispatch()
        d.add_task("a")

        def my_fn(task):
            return f"result_{task.name}"

        results = d.execute(execute_fn=my_fn)
        assert results[0].result == "result_a"

    def test_stats(self):
        d = V18AgentDispatch()
        d.add_task("a")
        d.execute()
        stats = d.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_success"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])