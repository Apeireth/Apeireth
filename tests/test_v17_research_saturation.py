"""v17_research_saturation.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v17_research_saturation import (
    V17_VERSION, ResearchDocument, RESEARCH_DOCUMENTS,
    extract_keywords, extract_summary, V17ResearchSaturation,
)


class TestV17Helpers:
    def test_extract_keywords(self):
        text = "The ASI agent uses Apeireth philosophy. Apeireth is real. ASI is approaching."
        kws = extract_keywords(text)
        # 真生产: 至少有一些关键词被提取
        assert len(kws) > 0

    def test_extract_summary(self):
        text = "A long text " * 100
        summary = extract_summary(text, max_chars=50)
        assert len(summary) <= 60


class TestV17:
    def test_init(self):
        s = V17ResearchSaturation()
        assert s.documents == []

    def test_research_documents_count(self):
        assert len(RESEARCH_DOCUMENTS) == 12

    def test_scan(self):
        s = V17ResearchSaturation()
        docs = s.scan_documents()
        assert len(docs) > 0

    def test_aggregate(self):
        s = V17ResearchSaturation()
        s.scan_documents()
        agg = s.aggregate_keywords()
        assert isinstance(agg, dict)

    def test_stats(self):
        s = V17ResearchSaturation()
        s.scan_documents()
        stats = s.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_documents"] > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])