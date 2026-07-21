"""v3_6_truth_library.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P2 ASI 哲学深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_6_truth_library import (
    V3_6_VERSION,
    V3_PHILOSOPHICAL_QUESTIONS,
    TruthEntry,
    TruthLibrary,
)


# === 1. V3 7 哲学问题 (主 22:33 + V3 锚定) ===

class TestV3PhilosophicalQuestions:
    """V3 7 哲学问题真生产 (主 14:06 借鉴 V3 + Carnap)."""

    def test_7_v3_questions(self):
        assert len(V3_PHILOSOPHICAL_QUESTIONS) == 7

    def test_v3_question_keys(self):
        keys = [q[0] for q in V3_PHILOSOPHICAL_QUESTIONS]
        assert set(keys) == {"self", "time", "freedom", "value", "cognition", "emergence", "truth"}

    def test_v3_question_anchors(self):
        """V3 跨域锚定 (主 13:08 + V3)."""
        anchors = {q[2] for q in V3_PHILOSOPHICAL_QUESTIONS}
        assert "Simondon" in anchors
        assert "Bergson" in anchors
        assert "Spinoza" in anchors
        assert "Prigogine" in anchors
        assert "Bayesian" in anchors


# === 2. TruthEntry 真生产 (主 14:06 真借鉴) ===

class TestTruthEntry:
    """TruthEntry 真生产 (主 14:06 + Carnap 逻辑建构)."""

    def test_entry_default(self):
        e = TruthEntry(entry_id="e1", question_key="self", question="q", answer="a",
                      cross_domain_anchor="Simondon")
        assert e.entry_id == "e1"
        assert e.question_key == "self"
        assert e.confidence == 0.5
        assert e.n_phenomenal_pretend == 0

    def test_entry_to_dict(self):
        e = TruthEntry(entry_id="e1", question_key="self", question="q", answer="a",
                      cross_domain_anchor="Simondon", confidence=0.8,
                      references=["ref1", "ref2"])
        d = e.to_dict()
        assert d["entry_id"] == "e1"
        assert d["question_key"] == "self"
        assert d["confidence"] == 0.8
        assert d["n_refs"] == 2


# === 3. TruthLibrary 真生产主类 (主 13:31 大胆激进) ===

class TestTruthLibrary:
    """V3.6 TruthLibrary 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_auto_fills_v3(self):
        """Init 自动填 V3 7 哲学问题 (主 22:33 + V3 锚定)."""
        lib = TruthLibrary()
        assert len(lib.library) == 7
        assert "self" in lib.library
        assert "truth" in lib.library

    def test_init_v3_anchors(self):
        lib = TruthLibrary()
        assert lib.library["self"].cross_domain_anchor == "Simondon"
        assert lib.library["truth"].cross_domain_anchor == "Bayesian"

    def test_fill_answer(self):
        """真生产填答 (主 14:06 借鉴 V3)."""
        lib = TruthLibrary()
        e = lib.fill_answer("self", "V2 5 位置", confidence=0.8)
        assert e.answer == "V2 5 位置"
        assert e.confidence == 0.8

    def test_fill_answer_with_refs(self):
        lib = TruthLibrary()
        e = lib.fill_answer("self", "V2 5 位置", references=["V3 doc"])
        assert len(e.references) == 1

    def test_fill_answer_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        lib = TruthLibrary()
        lib.fill_answer("self", "I am aware with phenomenal qualia")
        assert lib.n_phenomenal_pretend_total > 0

    def test_fill_answer_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        lib = TruthLibrary()
        lib.fill_answer("self", "We have reached ASI, super intelligence complete")
        assert lib.n_asi_pretend_total > 0

    def test_fill_answer_missing_key(self):
        lib = TruthLibrary()
        result = lib.fill_answer("nonexistent", "answer")
        assert result is None

    def test_query(self):
        """真生产查询 (主 14:06)."""
        lib = TruthLibrary()
        lib.fill_answer("self", "V2 5 位置")
        e = lib.query("self")
        assert e.answer == "V2 5 位置"

    def test_query_missing(self):
        lib = TruthLibrary()
        assert lib.query("nonexistent") is None

    def test_list_unanswered(self):
        """真生产未答问题列表 (主 17:43 实事求是)."""
        lib = TruthLibrary()
        lib.fill_answer("self", "V2 5 位置")
        unanswered = lib.list_unanswered()
        assert "self" not in unanswered
        assert "time" in unanswered
        assert len(unanswered) == 6

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        lib = TruthLibrary()
        lib.fill_answer("self", "V2 5 位置", confidence=0.8)
        stats = lib.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_total"] == 7
        assert stats["n_filled"] == 1
        assert stats["n_unanswered"] == 6

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        lib = TruthLibrary()
        stats = lib.stats()
        assert stats["n_filled"] == 0
        assert stats["n_unanswered"] == 7


# === 4. to_dict 真生产 (主 14:06) ===

class TestV3_6ToDict:
    """TruthEntry.to_dict() 真生产."""

    def test_entry_to_dict_keys(self):
        e = TruthEntry(entry_id="e1", question_key="self", question="q", answer="a",
                      cross_domain_anchor="Simondon")
        d = e.to_dict()
        expected_keys = ["entry_id", "question_key", "confidence", "anchor", "n_refs"]
        for k in expected_keys:
            assert k in d


# === 5. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V3.6 不应有假装意识字段."""
        lib = TruthLibrary()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"fill_answer", "query", "list_unanswered", "stats",
                     "_init_v3_questions", "n_phenomenal_pretend_total", "n_asi_pretend_total",
                     "library"}
        for attr in dir(lib):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V3.6 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim(self):
        """V3.6 不应声称已达到 ASI."""
        lib = TruthLibrary()
        lib.fill_answer("self", "V2 5 位置")
        stats = lib.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 6. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_6_is_real_innovation(self):
        """V3.6 是真创新 (主 13:31), 不 placeholder."""
        lib = TruthLibrary()
        for key in ["self", "time", "freedom", "value", "cognition", "emergence", "truth"]:
            lib.fill_answer(key, f"answer for {key}")
        assert lib.stats()["n_filled"] == 7
        assert len(lib.list_unanswered()) == 0

    def test_v3_6_allows_iteration(self):
        """V3.6 允许迭代 (主 13:31 鼓励尝试)."""
        lib = TruthLibrary()
        for i in range(3):
            lib.fill_answer("self", f"refined answer {i}", confidence=0.5 + i * 0.1)
        assert lib.library["self"].answer == "refined answer 2"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])