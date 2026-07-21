"""v13_asi_dashboard.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v13_asi_dashboard import (
    V13_VERSION, Dashboard, render_dashboard,
)


class TestDashboard:
    def test_default(self):
        d = Dashboard(timestamp=0.0)
        assert d.n_commits == 0
        assert d.asi_demo_v8_success_rate == 0.0

    def test_to_dict(self):
        d = Dashboard(timestamp=0.0, n_commits=30, n_tests=864)
        dd = d.to_dict()
        assert dd["n_commits"] == 30
        assert dd["n_tests"] == 864


class TestRenderDashboard:
    def test_render(self):
        d = Dashboard(timestamp=0.0, n_commits=30, n_tests=864, n_modules=18)
        text = render_dashboard(d)
        assert "V13 ASI" in text
        assert "30" in text
        assert "864" in text


if __name__ == "__main__":
    pytest.main([__file__, "-v"])