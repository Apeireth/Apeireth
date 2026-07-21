"""v36_hqb_benchmark.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v36_hqb_benchmark import (
    V36_VERSION, HQBScore,
    measure_self_consistency, measure_noise_resistance,
    measure_evolvability, measure_cross_domain_transfer,
    V36HQBBenchmark,
)


class TestV36Helpers:
    def test_measure_sc_constant(self):
        sc = measure_self_consistency(lambda x: 1.0, n_trials=5)
        assert sc == 1.0

    def test_measure_sc_variable(self):
        sc = measure_self_consistency(lambda x: 0.5 if x == "a" else 0.3, n_trials=10, input_data="a")
        assert 0 <= sc <= 1.0

    def test_measure_nr(self):
        nr = measure_noise_resistance(lambda x: 1.0, inputs=["a", "b", "c"])
        assert nr == 1.0

    def test_measure_ev_increase(self):
        ev = measure_evolvability(0.5, 0.7)
        assert ev > 0.5

    def test_measure_ev_decrease(self):
        ev = measure_evolvability(0.5, 0.3)
        assert ev < 0.5

    def test_measure_ev_zero_prev(self):
        assert measure_evolvability(0.0, 0.5) == 0.0

    def test_measure_cdt(self):
        cdt = measure_cross_domain_transfer({"a": 0.5, "b": 0.7, "c": 0.9})
        assert cdt == pytest.approx(0.7, abs=0.01)

    def test_measure_cdt_empty(self):
        assert measure_cross_domain_transfer({}) == 0.0


class TestV36:
    def test_init(self):
        b = V36HQBBenchmark()
        assert b.scores == []

    def test_run_benchmark(self):
        b = V36HQBBenchmark()
        s = b.run_benchmark()
        assert s.sc >= 0
        assert s.nr >= 0
        assert s.ev >= 0
        assert s.cdt >= 0

    def test_run_benchmark_total(self):
        b = V36HQBBenchmark()
        s = b.run_benchmark()
        assert 0 <= s.total <= 1.0

    def test_render(self):
        b = V36HQBBenchmark()
        b.run_benchmark()
        text = b.render()
        assert "HQB" in text
        assert "主 18:52" in text

    def test_stats(self):
        b = V36HQBBenchmark()
        b.run_benchmark()
        stats = b.stats()
        assert stats["n_scores"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])