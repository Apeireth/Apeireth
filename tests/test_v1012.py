"""V1012 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1012_agent_benchmark import (
    V1012_VERSION, BenchmarkQuestion, BenchmarkResult, V1012AgentBenchmark,
)


class TestV1012:
    def test_init(self):
        b = V1012AgentBenchmark()
        assert b.n_questions() == 9

    def test_mmlu_questions(self):
        """V1012 真测 MMLU 真借鉴."""
        b = V1012AgentBenchmark()
        mmlu = b.get_by_benchmark("MMLU")
        assert len(mmlu) == 5

    def test_humaneval_questions(self):
        """V1012 真测 HumanEval 真借鉴."""
        b = V1012AgentBenchmark()
        he = b.get_by_benchmark("HumanEval")
        assert len(he) == 2

    def test_hellaswag_questions(self):
        """V1012 真测 HellaSwag 真借鉴."""
        b = V1012AgentBenchmark()
        hs = b.get_by_benchmark("HellaSwag")
        assert len(hs) == 2

    def test_evaluate_correct(self):
        b = V1012AgentBenchmark()
        r = b.evaluate("mmlu_0", "Paris")
        assert r.correct is True
        assert r.score == 1.0

    def test_evaluate_incorrect(self):
        b = V1012AgentBenchmark()
        r = b.evaluate("mmlu_0", "London")
        assert r.correct is False
        assert r.score == 0.0

    def test_evaluate_partial_match(self):
        """V1012 真测部分匹配 (主 17:43 实事求是)."""
        b = V1012AgentBenchmark()
        r = b.evaluate("mmlu_0", "The answer is Paris.")
        assert r.correct is True

    def test_evaluate_all(self):
        b = V1012AgentBenchmark()
        preds = {f"mmlu_{i}": gt for i, gt in enumerate(["Paris", "4", "water", "George Orwell", "Jupiter"])}
        result = b.evaluate_all(preds)
        assert result["n_correct"] == 5
        assert result["accuracy"] == 1.0

    def test_unknown_question(self):
        b = V1012AgentBenchmark()
        with pytest.raises(ValueError):
            b.evaluate("unknown", "x")

    def test_register_question(self):
        b = V1012AgentBenchmark()
        b.register_question(BenchmarkQuestion(
            question_id="custom", benchmark="Custom",
            prompt="?", ground_truth="x",
        ))
        assert b.n_questions() == 10

    def test_n_results(self):
        b = V1012AgentBenchmark()
        b.evaluate("mmlu_0", "Paris")
        b.evaluate("mmlu_1", "4")
        assert b.n_results() == 2

    def test_stats(self):
        b = V1012AgentBenchmark()
        s = b.stats()
        assert s["n_questions"] == 9
        assert "MMLU" in s["benchmarks"]
        assert "HumanEval" in s["benchmarks"]
        assert "HellaSwag" in s["benchmarks"]

    def test_v22_33_asi_integration(self):
        """V1012 真测主 22:33 ASI 北极星."""
        b = V1012AgentBenchmark()
        s = b.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_github_integration(self):
        """V1012 真测主 19:33 别忘了github这个宝库."""
        b = V1012AgentBenchmark()
        assert len(b.get_by_benchmark("MMLU")) > 0
        assert len(b.get_by_benchmark("HumanEval")) > 0
        assert len(b.get_by_benchmark("HellaSwag")) > 0

    def test_v17_43_truth(self):
        """V1012 真测主 17:43 实事求是 (真评分, 不假装)."""
        b = V1012AgentBenchmark()
        # 真测正确 vs 错误
        assert b.evaluate("mmlu_0", "Paris").correct is True
        assert b.evaluate("mmlu_0", "London").correct is False

    def test_complete_integration(self):
        """V1012 真测完整 benchmark (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        b = V1012AgentBenchmark()
        s = b.stats()
        assert s["n_questions"] == 9
        # 真测 evaluate_all
        preds = {qid: q.ground_truth for qid, q in b.questions.items()}
        result = b.evaluate_all(preds)
        assert result["accuracy"] == 1.0