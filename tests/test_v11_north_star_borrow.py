"""v11_north_star_borrow.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v11_north_star_borrow import (
    V11_VERSION,
    BorrowComponent,
    BORROW_WEIGHTS,
    BorrowMeasurement,
    V11NorthStarBorrow,
)


class TestBorrowComponents:
    def test_7_components(self):
        assert {c.value for c in BorrowComponent} == {
            "portable_seed", "hgt", "epigenetic", "waddington",
            "prion", "autocatalytic", "dissipative"
        }

    def test_borrow_weights_sum(self):
        assert abs(sum(BORROW_WEIGHTS.values()) - 0.70) < 0.01


class TestBorrowMeasurement:
    def test_default(self):
        m = BorrowMeasurement(component=BorrowComponent.HGT)
        assert m.component == BorrowComponent.HGT
        assert m.score == 0.0


class TestV11Measurements:
    def test_measure_portable_seed(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_portable_seed(genome_size=500)
        assert m.score == 0.5

    def test_measure_hgt(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_hgt(n_events=10, n_success=5)
        assert m.score == pytest.approx(0.5)

    def test_measure_epigenetic(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_epigenetic(n_marks=4, n_inherited=2)
        assert m.score == 0.5

    def test_measure_waddington(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_waddington(n_states=4, n_canalized=2)
        assert m.score == 0.5

    def test_measure_prion(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_prion(n_proteins=10, n_misfolded=5)
        assert m.score == 0.5

    def test_measure_autocatalytic_raf(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_autocatalytic(n_reactions=2, is_raf=True)
        assert m.score == 1.0

    def test_measure_dissipative(self):
        v11 = V11NorthStarBorrow()
        m = v11.measure_dissipative(n_structures=4, n_far_eq=2)
        assert m.score == 0.5


class TestV11Compute:
    def test_compute_total_zero(self):
        v11 = V11NorthStarBorrow()
        assert v11.compute_total() == 0.0

    def test_compute_total_partial(self):
        v11 = V11NorthStarBorrow()
        v11.measure_portable_seed(genome_size=1000)  # 1.0
        assert v11.compute_total() == pytest.approx(0.10, abs=0.01)

    def test_compute_total_full(self):
        v11 = V11NorthStarBorrow()
        v11.measure_portable_seed(genome_size=10000)  # 1.0
        v11.measure_hgt(n_events=10, n_success=10)
        v11.measure_epigenetic(n_marks=10, n_inherited=10)
        v11.measure_waddington(n_states=10, n_canalized=10)
        v11.measure_prion(n_proteins=10, n_misfolded=10)
        v11.measure_autocatalytic(n_reactions=2, is_raf=True)
        v11.measure_dissipative(n_structures=10, n_far_eq=10)
        assert v11.compute_total() == pytest.approx(0.70, abs=0.01)


class TestV3Guard:
    def test_no_phenomenal_in_stats(self):
        v11 = V11NorthStarBorrow()
        v11.measure_hgt(n_events=1, n_success=1)
        stats = v11.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])