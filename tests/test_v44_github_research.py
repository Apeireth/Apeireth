"""v44_github_research.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v44_github_research import (
    V44_VERSION, GitHubProjectFinding, GITHUB_PROJECTS, V44GitHubResearch,
)


class TestV44GitHub:
    def test_init(self):
        s = V44GitHubResearch()
        assert len(s.findings) == 8

    def test_8_projects(self):
        s = V44GitHubResearch()
        names = [f.project_name for f in s.findings]
        for expected in [
            "OpenCog Hyperon", "AERA", "NARS (OpenNARS)", "Mem0",
            "Letta", "DGM (Sakana AI)", "Hyperagents (FAIR/Meta)", "VCP (lioensky/VCPToolBox)",
        ]:
            assert expected in names

    def test_total_stars(self):
        s = V44GitHubResearch()
        stars = s.total_stars()
        assert stars > 20000

    def test_avg_confidence(self):
        s = V44GitHubResearch()
        avg = s.average_confidence()
        assert 0.7 < avg < 1.0

    def test_stats(self):
        s = V44GitHubResearch()
        stats = s.stats()
        assert stats["n_projects"] == 8
        assert stats["total_stars"] > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])