"""v24_real_production_rate.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v24_real_production_rate import (
    V24_VERSION, RealProductionMetric,
    measure_n_commits, measure_n_tests, measure_n_modules,
    V24RealProductionRate,
)


class TestV24Helpers:
    def test_measure_n_commits(self):
        m = measure_n_commits(repo_dir=".")
        assert m.value > 0
        assert m.unit == "commits"

    def test_measure_n_modules(self):
        m = measure_n_modules(apeireth_dir="apeireth")
        assert m.value > 0
        assert m.unit == "modules"


class TestV24:
    def test_init(self):
        m = V24RealProductionRate()
        assert m.metrics == []

    def test_measure_all(self):
        m = V24RealProductionRate()
        metrics = m.measure_all()
        assert len(metrics) == 3

    def test_render(self):
        m = V24RealProductionRate()
        m.measure_all()
        text = m.render()
        assert "真生产率" in text
        assert "实事求是" in text

    def test_stats(self):
        m = V24RealProductionRate()
        m.measure_all()
        stats = m.stats()
        assert stats["n_metrics"] == 3
        assert "n_commits" in stats["metrics"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])