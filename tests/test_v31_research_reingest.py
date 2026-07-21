"""v31_research_reingest.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v31_research_reingest import (
    V31_VERSION, ResearchSource, classify_research_topic,
    extract_keywords, measure_research_source, V31ResearchReingest,
)


class TestV31Helpers:
    def test_classify_philo(self):
        flags = classify_research_topic(["simondon", "bergson"])
        assert flags["is_philo_borrow"] is True

    def test_classify_ai(self):
        flags = classify_research_topic(["llm", "agent"])
        assert flags["is_ai_borrow"] is True

    def test_classify_science(self):
        flags = classify_research_topic(["friston", "complexity"])
        assert flags["is_science_borrow"] is True

    def test_classify_bio(self):
        flags = classify_research_topic(["hgt", "epigenetic"])
        assert flags["is_bio_borrow"] is True

    def test_classify_tech(self):
        flags = classify_research_topic(["rust", "sqlx"])
        assert flags["is_tech_borrow"] is True


class TestV31:
    def test_init(self):
        s = V31ResearchReingest(base_dir=".")
        assert s.sources == []

    def test_reingest_all(self):
        s = V31ResearchReingest(base_dir=".")
        sources = s.reingest_all()
        assert len(sources) >= 10

    def test_n_borrowed(self):
        s = V31ResearchReingest(base_dir=".")
        s.reingest_all()
        n = s.n_borrowed()
        assert n > 0

    def test_render(self):
        s = V31ResearchReingest(base_dir=".")
        s.reingest_all()
        text = s.render()
        assert "调研" in text
        assert "主 18:44" in text

    def test_stats(self):
        s = V31ResearchReingest(base_dir=".")
        s.reingest_all()
        stats = s.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_sources"] >= 10


if __name__ == "__main__":
    pytest.main([__file__, "-v"])