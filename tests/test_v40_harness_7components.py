"""v40_harness_7components.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v40_harness_7components import (
    V40_VERSION, HarnessComponent, HARNESS_7_COMPONENTS,
    HarnessDashboardSnapshot, measure_harness_component,
    V40Harness7Components,
)


class TestV40:
    def test_init(self):
        h = V40Harness7Components()
        assert len(h.components) == 7

    def test_7_components(self):
        h = V40Harness7Components()
        names = [c.name for c in h.components]
        expected = ["System Rules", "Tool Descriptions", "Tool Implementations",
                    "Middleware", "Skills", "Sub-Agents", "Long-Term Memory"]
        for e in expected:
            assert e in names

    def test_average_coverage(self):
        h = V40Harness7Components()
        avg = h.average_coverage()
        assert 0.5 < avg <= 1.0

    def test_take_snapshot(self):
        h = V40Harness7Components()
        snap = h.take_snapshot(n_tests=1085, n_modules=40, v0_1_total=0.7905)
        assert snap.asi_level == "ASI"

    def test_take_snapshot_ani(self):
        h = V40Harness7Components()
        snap = h.take_snapshot(v0_1_total=0.2)
        assert snap.asi_level == "ANI"

    def test_render(self):
        h = V40Harness7Components()
        h.take_snapshot(n_tests=1085, n_modules=40, v0_1_total=0.7905)
        text = h.render()
        assert "7 组件" in text
        assert "主 18:52" in text

    def test_stats(self):
        h = V40Harness7Components()
        h.take_snapshot()
        stats = h.stats()
        assert stats["n_components"] == 7


if __name__ == "__main__":
    pytest.main([__file__, "-v"])