"""v73_tool_execution_engine.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v73_tool_execution_engine import (
    V73_VERSION, ToolExecutionResult, V73ToolExecutionEngine,
)


class TestV73:
    def test_init(self):
        te = V73ToolExecutionEngine()
        assert te.tools == {}

    def test_register_tool(self):
        te = V73ToolExecutionEngine()
        te.register_tool("add", lambda a, b: a + b)
        assert te.n_tools() == 1

    def test_execute_tool(self):
        te = V73ToolExecutionEngine()
        te.register_tool("add", lambda a, b: a + b)
        eid = te.execute_tool("add", {"a": 1, "b": 2})
        assert te.executions[-1].result == 3

    def test_execute_unknown_tool(self):
        te = V73ToolExecutionEngine()
        eid = te.execute_tool("nonexistent", {})
        assert te.executions[-1].error != ""

    def test_execute_tool_with_exception(self):
        te = V73ToolExecutionEngine()
        def bad(x): raise ValueError("oops")
        te.register_tool("bad", bad)
        eid = te.execute_tool("bad", {"x": 1})
        assert "oops" in te.executions[-1].error

    def test_n_safe_executions(self):
        te = V73ToolExecutionEngine()
        te.register_tool("f", lambda: None)
        te.execute_tool("f", {}, safety_checked=True)
        assert te.n_safe_executions() == 1

    def test_stats(self):
        te = V73ToolExecutionEngine()
        te.register_tool("f", lambda: None)
        stats = te.stats()
        assert stats["n_tools"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])