"""v16_end_to_end_report.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v16_end_to_end_report import V16_VERSION, EndToEndReport, V16EndToEndReport


class TestV16:
    def test_init(self):
        r = V16EndToEndReport()
        assert r.reports == []

    def test_generate(self):
        r = V16EndToEndReport()
        report = r.generate(n_commits=30, n_tests=877, n_modules=22)
        assert report.n_commits == 30
        assert report.n_tests == 877
        assert len(r.reports) == 1

    def test_render(self):
        r = V16EndToEndReport()
        report = r.generate(
            n_commits=30,
            n_tests=877,
            n_modules=22,
            v_components={"v3.1": 0.85},
            v_borrow_components={"hgt": 0.7},
            v_graph_stats={"n_nodes": 14},
            v_memory_stats={"n_entries": 7},
        )
        text = report.render()
        assert "V16" in text
        assert "877" in text
        assert "v3.1" in text

    def test_stats(self):
        r = V16EndToEndReport()
        r.generate(n_commits=1, n_tests=1, n_modules=1)
        stats = r.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])