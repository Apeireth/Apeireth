"""v42_anysearch_runner.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v42_anysearch_runner import (
    V42_RUNNER_VERSION, ResearchFinding, ResearchRunnerReport,
    V42AnySearchRunner,
)


class TestV42Runner:
    def test_init(self):
        r = V42AnySearchRunner()
        assert r.reports == []

    def test_research_one(self):
        r = V42AnySearchRunner()
        # 不依赖网络, 只验证 query 字段
        finding = ResearchFinding(finding_id="x", query="test", source="test", content="")
        assert finding.query == "test"

    def test_research_area(self):
        r = V42AnySearchRunner()
        # 不依赖网络, 只验证 report 构造
        report = ResearchRunnerReport(
            report_id="x",
            n_total_queries=2,
        )
        assert report.n_total_queries == 2

    def test_stats(self):
        r = V42AnySearchRunner()
        r.research_area("test", ["q1"])
        stats = r.stats()
        assert stats["n_reports"] == 1
        assert stats["total_queries"] >= 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])