"""v23_v3_7q_full.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v23_v3_7q_full import V23_VERSION, V3_FULL_ANSWERS, V3FullAnswer, V23V37QuestionsFull


class TestV23:
    def test_init(self):
        s = V23V37QuestionsFull()
        assert len(s.answers) == 7

    def test_all_v3_questions_covered(self):
        s = V23V37QuestionsFull()
        for key in ["self", "time", "freedom", "value", "cognition", "emergence", "truth"]:
            assert key in s.answers

    def test_query(self):
        s = V23V37QuestionsFull()
        ans = s.query("self")
        assert ans is not None
        assert ans.anchor == "Simondon"

    def test_query_missing(self):
        s = V23V37QuestionsFull()
        assert s.query("nonexistent") is None

    def test_all_anchors(self):
        """真生产 7 哲学问题跨域锚定 (主 13:08)."""
        s = V23V37QuestionsFull()
        expected = {"Simondon", "Bergson", "Spinoza", "Canguilhem",
                    "Merleau-Ponty", "Prigogine", "Bayesian"}
        actual = set(ans.anchor for ans in s.answers.values())
        assert actual == expected

    def test_average_confidence(self):
        s = V23V37QuestionsFull()
        avg = s.average_confidence()
        assert 0.7 < avg < 1.0

    def test_render_report(self):
        s = V23V37QuestionsFull()
        report = s.render_report()
        assert "V3 7 哲学问题" in report
        assert "Simondon" in report
        assert "Bayesian" in report

    def test_stats(self):
        s = V23V37QuestionsFull()
        stats = s.stats()
        assert stats["n_answers"] == 7
        assert "Simondon" in stats["anchors_used"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])