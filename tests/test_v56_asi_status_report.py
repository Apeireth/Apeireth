"""v56_asi_status_report.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v56_asi_status_report import (
    V56_VERSION, ASIStatusReport, V43_V55_MODULES, V56ASIStatusReport,
)


class TestV56:
    def test_init(self):
        s = V56ASIStatusReport()
        assert s.reports == []

    def test_modules_count(self):
        assert len(V43_V55_MODULES) == 10

    def test_generate_report(self):
        s = V56ASIStatusReport()
        r = s.generate_report()
        assert r.n_total_modules == 10
        assert r.n_total_paradigms == 4

    def test_render(self):
        s = V56ASIStatusReport()
        s.generate_report()
        text = s.render()
        assert "ASI" in text
        assert "4 范式" in text

    def test_stats(self):
        s = V56ASIStatusReport()
        s.generate_report()
        stats = s.stats()
        assert stats["n_reports"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])