"""v54_asi_unified_measure.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v54_asi_unified_measure import (
    V54_VERSION, ASI_V54_WEIGHTS, ASIUnifiedScore,
    measure_asi_v54, V54ASIUnifiedMeasure,
)


class TestV54Helpers:
    def test_weights_count(self):
        assert len(ASI_V54_WEIGHTS) == 15

    def test_weights_sum(self):
        total = sum(ASI_V54_WEIGHTS.values())
        assert abs(total - 1.0) < 0.001

    def test_measure_asi_v54_basic(self):
        score = measure_asi_v54({"phi_proxy": 0.5, "capabilities": 0.5})
        assert 0 < score.total < 1.0

    def test_measure_asi_v54_asi_level(self):
        # 真生产: 15 项全 1.0 = 1.0 ASI level
        score = measure_asi_v54({k: 1.0 for k in ASI_V54_WEIGHTS})
        assert score.asi_level == "ASI"


class TestV54:
    def test_init(self):
        m = V54ASIUnifiedMeasure()
        assert m.scores == []

    def test_measure_v54(self):
        m = V54ASIUnifiedMeasure()
        s = m.measure_v54()
        assert s.n_components == 15
        assert s.asi_level == "ASI"  # 真生产: 高分 = ASI

    def test_stats(self):
        m = V54ASIUnifiedMeasure()
        m.measure_v54()
        stats = m.stats()
        assert stats["n_components"] == 15
        assert "latest" in stats


if __name__ == "__main__":
    pytest.main([__file__, "-v"])