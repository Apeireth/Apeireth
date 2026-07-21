"""v41_ultimate_dashboard.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v41_ultimate_dashboard import (
    V41_VERSION, UltimateDashboardSnapshot,
    measure_git_log, measure_pytest_collect,
    measure_modules, measure_doc_md, measure_lines,
    V41UltimateDashboard,
)


class TestV41Helpers:
    def test_measure_git_log(self):
        n = measure_git_log(repo_dir=".")
        assert n > 0

    def test_measure_pytest(self):
        n = measure_pytest_collect()
        assert n > 0

    def test_measure_modules(self):
        n = measure_modules()
        assert n > 0

    def test_measure_doc_md(self):
        n = measure_doc_md()
        assert n > 0

    def test_measure_lines(self):
        n = measure_lines()
        assert n > 0


class TestV41:
    def test_init(self):
        d = V41UltimateDashboard()
        assert d.snapshots == []

    def test_measure_all(self):
        d = V41UltimateDashboard()
        s = d.measure_all()
        assert s.n_commits > 0
        assert s.n_tests > 0
        assert s.n_modules > 0
        assert s.asi_level == "ASI"
        assert s.philosophy_guard == "PASS"

    def test_render(self):
        d = V41UltimateDashboard()
        d.measure_all()
        text = d.render()
        assert "Dashboard" in text
        assert "主 18:52" in text

    def test_stats(self):
        d = V41UltimateDashboard()
        d.measure_all()
        stats = d.stats()
        assert stats["n_snapshots"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])