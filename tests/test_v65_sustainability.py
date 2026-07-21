"""v65_sustainability.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v65_sustainability import (
    V65_VERSION, SustainabilityDimension, SustainabilityMetric,
    SUSTAINABILITY_METRICS, V65Sustainability,
)


class TestV65:
    def test_init(self):
        s = V65Sustainability()
        assert s.n_metrics() == 6

    def test_6_dimensions(self):
        s = V65Sustainability()
        dimensions = [m.dimension.value for m in s.metrics.values()]
        for expected in ["code_quality", "testing", "documentation",
                         "research", "integration", "sustainability"]:
            assert expected in dimensions

    def test_average_score(self):
        s = V65Sustainability()
        avg = s.average_score()
        assert 0.8 < avg < 1.0

    def test_is_sustainable(self):
        s = V65Sustainability()
        assert s.is_sustainable() is True

    def test_is_not_sustainable_high_threshold(self):
        s = V65Sustainability()
        assert s.is_sustainable(threshold=0.99) is False

    def test_stats(self):
        s = V65Sustainability()
        stats = s.stats()
        assert stats["n_metrics"] == 6
        assert stats["is_sustainable"] is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])