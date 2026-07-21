"""v63_ultimate_measure.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v63_ultimate_measure import (
    V63_VERSION, UltimateMeasureResult,
    measure_git_log, measure_pytest_collect,
    measure_v_modules, measure_total_lines, measure_doc_md,
    V63UltimateMeasure,
)


class TestV63Helpers:
    def test_measure_git_log(self):
        n = measure_git_log(repo_dir=".")
        assert n > 0

    def test_measure_pytest(self):
        n = measure_pytest_collect()
        assert n > 0

    def test_measure_v_modules(self):
        n = measure_v_modules()
        assert n > 0

    def test_measure_total_lines(self):
        n = measure_total_lines()
        assert n > 0

    def test_measure_doc_md(self):
        n = measure_doc_md()
        assert n > 0


class TestV63:
    def test_init(self):
        m = V63UltimateMeasure()
        assert m.measures == []

    def test_measure_all(self):
        m = V63UltimateMeasure()
        r = m.measure_all()
        assert r.n_commits > 0
        assert r.n_tests > 0
        assert r.v54_asi_level == "ASI"

    def test_render(self):
        m = V63UltimateMeasure()
        m.measure_all()
        text = m.render()
        assert "ASI" in text
        assert "主 22:33" in text

    def test_stats(self):
        m = V63UltimateMeasure()
        m.measure_all()
        stats = m.stats()
        assert stats["n_measures"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])