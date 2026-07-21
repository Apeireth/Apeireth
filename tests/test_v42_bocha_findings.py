"""v42_bocha_findings.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v42_bocha_findings import (
    V42_FINDINGS_VERSION, CognitiveArchFinding, BOCHA_FINDINGS,
    V42BochaFindings,
)


class TestV42Findings:
    def test_init(self):
        s = V42BochaFindings()
        assert len(s.findings) == 3

    def test_3_archs(self):
        s = V42BochaFindings()
        names = [f.arch_name for f in s.findings]
        for expected in ["OpenCog Hyperon", "AERA", "NARS"]:
            assert expected in names

    def test_avg_confidence(self):
        s = V42BochaFindings()
        avg = s.stats()["avg_confidence"]
        assert 0.8 < avg < 1.0

    def test_stats(self):
        s = V42BochaFindings()
        stats = s.stats()
        assert stats["n_findings"] == 3
        assert "OpenCog Hyperon" in stats["archs"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])