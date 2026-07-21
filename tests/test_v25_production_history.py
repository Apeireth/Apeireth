"""v25_production_history.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v25_production_history import (
    V25_VERSION, ProductionSnapshot, take_snapshot,
    count_lines_python, count_doc_md, V25ProductionHistory,
)


class TestV25Helpers:
    def test_count_lines_python(self):
        n = count_lines_python()
        assert n > 0

    def test_count_doc_md(self):
        n = count_doc_md()
        assert n > 0


class TestV25:
    def test_init(self):
        h = V25ProductionHistory(history_file=".apeireth_history_test.json")
        assert isinstance(h.snapshots, list)

    def test_take_snapshot(self):
        s = take_snapshot(label="test")
        assert s.n_commits > 0
        assert s.n_tests > 0
        assert s.n_modules > 0

    def test_take_and_store(self):
        h = V25ProductionHistory(history_file=".apeireth_history_test.json")
        s = h.take_and_store(label="snap1")
        assert len(h.snapshots) >= 1
        assert s.n_tests > 0

    def test_growth(self):
        h = V25ProductionHistory(history_file=".apeireth_history_test.json")
        h.take_and_store(label="g1")
        h.take_and_store(label="g2")
        g = h.growth()
        assert "commits_growth" in g

    def test_render(self):
        h = V25ProductionHistory(history_file=".apeireth_history_test.json")
        h.take_and_store(label="r1")
        text = h.render()
        assert "真生产率" in text
        assert "实事求是" in text

    def test_stats(self):
        h = V25ProductionHistory(history_file=".apeireth_history_test.json")
        h.take_and_store(label="s1")
        stats = h.stats()
        assert stats["n_snapshots"] >= 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])