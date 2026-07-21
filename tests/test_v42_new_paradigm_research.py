"""v42_new_paradigm_research.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v42_new_paradigm_research import (
    V42_VERSION, NewParadigmResearch,
    NEW_PARADIGM_RESEARCH_QUERIES, V42NewParadigmResearch,
)


class TestV42:
    def test_init(self):
        s = V42NewParadigmResearch()
        assert len(s.research_queries) == 8

    def test_n_research_areas(self):
        s = V42NewParadigmResearch()
        n = s.n_research_areas()
        assert n == 8

    def test_n_total_queries(self):
        s = V42NewParadigmResearch()
        n = s.n_total_queries()
        assert n >= 40  # 8 × 5 = 40

    def test_get_queries(self):
        s = V42NewParadigmResearch()
        queries = s.get_queries()
        assert "Cognitive Architecture Beyond LLM" in [q["name"] for q in queries]

    def test_render(self):
        s = V42NewParadigmResearch()
        text = s.render()
        assert "新范式" in text
        assert "主 19:17" in text
        assert "AnySearch" in text

    def test_stats(self):
        s = V42NewParadigmResearch()
        stats = s.stats()
        assert stats["n_research_areas"] == 8
        assert stats["n_total_queries"] >= 40


if __name__ == "__main__":
    pytest.main([__file__, "-v"])