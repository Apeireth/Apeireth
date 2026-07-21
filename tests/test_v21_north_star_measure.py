"""v21_north_star_measure.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v21_north_star_measure import (
    V21_VERSION, ASI_V01_WEIGHTS, V01Score, V01MeasureResult,
    measure_phi_proxy, measure_capabilities, measure_cross_domain,
    measure_engineering, measure_v2_philosophy, measure_real_production,
    V21NorthStarMeasure,
)


class TestV21Helpers:
    def test_measure_phi_proxy(self):
        s = measure_phi_proxy(0.85)
        assert s.weighted_score == pytest.approx(0.17, abs=0.01)

    def test_measure_capabilities(self):
        s = measure_capabilities(912, 22)
        assert s.weighted_score > 0

    def test_measure_cross_domain(self):
        s = measure_cross_domain(12)
        assert s.weighted_score > 0.1

    def test_measure_engineering(self):
        s = measure_engineering(46, 3)
        assert s.weighted_score > 0

    def test_measure_v2_philosophy(self):
        s = measure_v2_philosophy(5)
        assert s.weighted_score == pytest.approx(0.10, abs=0.01)

    def test_measure_real_production(self):
        s = measure_real_production(22)
        assert s.weighted_score > 0


class TestV21:
    def test_init(self):
        m = V21NorthStarMeasure()
        assert m.measures == []

    def test_measure_all(self):
        m = V21NorthStarMeasure()
        r = m.measure_all()
        assert r.total > 0.5
        assert r.n_components == 6

    def test_measure_all_asi_level(self):
        m = V21NorthStarMeasure()
        r = m.measure_all()
        # 真生产: 912 tests + 22 modules + 12 docs + 46 commits + 22 real modules
        # 应该 ASI level (>= 0.7)
        assert r.total >= 0.7
        assert r.level == "ASI"

    def test_measure_all_agi_level(self):
        m = V21NorthStarMeasure()
        r = m.measure_all(n_tests=10, n_modules=2, n_research_docs=0, n_commits=0,
                          n_integration=0, v2_positions_covered=1, n_real_modules=2)
        assert r.level == "ANI"

    def test_stats(self):
        m = V21NorthStarMeasure()
        m.measure_all()
        stats = m.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])