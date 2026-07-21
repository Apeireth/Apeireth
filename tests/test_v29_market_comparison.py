"""v29_market_comparison.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v29_market_comparison import (
    V29_VERSION, VCP_PLUGIN_TYPES, VCP_CONTEXT_TYPES,
    VCP_NOTIFICATION_SYSTEMS, TAGMEMO_INSIGHTS, COMPARISON_ROWS,
    ComparisonRow, V29MarketComparison,
)


class TestV29Constants:
    def test_vcp_plugin_types_count(self):
        assert len(VCP_PLUGIN_TYPES) == 6

    def test_vcp_context_types_count(self):
        assert len(VCP_CONTEXT_TYPES) == 4

    def test_vcp_notification_systems_count(self):
        assert len(VCP_NOTIFICATION_SYSTEMS) == 3

    def test_tagmemo_insights_count(self):
        assert len(TAGMEMO_INSIGHTS) == 4


class TestV29:
    def test_init(self):
        s = V29MarketComparison()
        assert len(s.rows) > 0

    def test_n_critical_gaps(self):
        s = V29MarketComparison()
        n = s.n_critical_gaps()
        assert n >= 1

    def test_n_major_gaps(self):
        s = V29MarketComparison()
        n = s.n_major_gaps()
        assert n >= 1

    def test_render(self):
        s = V29MarketComparison()
        text = s.render()
        assert "VCP" in text
        assert "Apeireth" in text
        assert "主 18:40" in text

    def test_stats(self):
        s = V29MarketComparison()
        stats = s.stats()
        assert stats["n_rows"] >= 5
        assert stats["n_critical_gaps"] >= 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])