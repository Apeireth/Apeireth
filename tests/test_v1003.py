"""V1003 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1003_v4_philosophy_full import (
    V1003_VERSION, V4PhilosophyAnswer, V4_PHILOSOPHY_FULL, V1003V4PhilosophyFull,
)


class TestV1003:
    def test_7_answers(self):
        assert len(V4_PHILOSOPHY_FULL) == 7

    def test_all_questions(self):
        for key in ["self", "time", "freedom", "value",
                    "cognition", "emergence", "truth"]:
            assert key in V4_PHILOSOPHY_FULL

    def test_query(self):
        p = V1003V4PhilosophyFull()
        ans = p.query("self")
        assert ans is not None
        assert ans.confidence > 0.8

    def test_all_answers(self):
        p = V1003V4PhilosophyFull()
        all_a = p.all_answers()
        assert len(all_a) == 7
        for k, a in all_a.items():
            assert a.question_key == k
            assert a.answer
            assert a.anchor
            assert a.confidence > 0

    def test_avg_confidence(self):
        p = V1003V4PhilosophyFull()
        avg = p.average_confidence()
        assert 0.8 < avg < 1.0

    def test_references_present(self):
        p = V1003V4PhilosophyFull()
        for ans in p.all_answers().values():
            assert len(ans.references) > 0, f"{ans.question_key} has no references"

    def test_no_phenomenal_pretending(self):
        p = V1003V4PhilosophyFull()
        for ans in p.all_answers().values():
            text = (ans.answer + ans.anchor).lower()
            assert "i am conscious" not in text
            assert "phenomenal consciousness" not in text or "不假装" in text

    def test_truth_anchor_5_methods(self):
        p = V1003V4PhilosophyFull()
        truth = p.query("truth")
        text = truth.answer.lower()
        for method in ["popper", "kuhn", "lakatos", "feyerabend", "laudan"]:
            assert method in text, f"{method} not in truth answer"

    def test_self_includes_v2_5_positions(self):
        p = V1003V4PhilosophyFull()
        self_q = p.query("self")
        text = self_q.answer
        for pos in ["调度者", "思考者", "无数关系集合体", "最大权限", "ASI 位置占据者"]:
            assert pos in text

    def test_stats(self):
        p = V1003V4PhilosophyFull()
        s = p.stats()
        assert s["n_answers"] == 7
        assert s["total_references"] > 20
        assert s["version"] == V1003_VERSION

    def test_v23_integration_truth(self):
        p = V1003V4PhilosophyFull()
        truth = p.query("truth")
        # 真理 answer 应包含 V0.1/V0.2 公式真测引用
        assert "V0.1" in truth.answer or "V0.2" in truth.answer or "Bayesian" in truth.answer

    def test_main_19_33_integration(self):
        p = V1003V4PhilosophyFull()
        for ans in p.all_answers().values():
            assert "主 19:33" in ans.answer or "主 22:33" in ans.answer