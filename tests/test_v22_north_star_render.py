"""v22_north_star_render.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v22_north_star_render import V22_VERSION, render_measure_result
from apeireth.v21_north_star_measure import V21NorthStarMeasure


class TestV22:
    def test_render(self):
        m = V21NorthStarMeasure()
        r = m.measure_all()
        text = render_measure_result(r)
        assert "ASI 北极星" in text
        assert "phi_proxy" in text
        assert str(round(r.total, 4)) in text


if __name__ == "__main__":
    pytest.main([__file__, "-v"])