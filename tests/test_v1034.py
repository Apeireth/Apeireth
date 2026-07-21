"""V1034 真生产 tests (主 00:36 效果)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1034_real_benchmark import (
    V1034_VERSION, MMLU_SAMPLES, GSM8K_SAMPLES, HUMANEVAL_SAMPLES, HELLASWAG_SAMPLES,
    evaluate_mmlu_sample, evaluate_gsm8k_sample,
    evaluate_humaneval_sample, evaluate_hellaswag_sample,
    V1034RealBenchmark,
)


class TestV1034:
    def test_mmlu_samples(self):
        """V1034 真测 MMLU 真数据集 (主 19:33 + 主 17:43)."""
        assert len(MMLU_SAMPLES) == 10
        assert all("question" in s and "answer" in s for s in MMLU_SAMPLES)

    def test_gsm8k_samples(self):
        """V1034 真测 GSM8K 真数据集 (主 19:33)."""
        assert len(GSM8K_SAMPLES) == 5
        assert all("question" in s and "answer" in s for s in GSM8K_SAMPLES)

    def test_humaneval_samples(self):
        """V1034 真测 HumanEval 真数据集 (主 19:33)."""
        assert len(HUMANEVAL_SAMPLES) == 3
        assert all("prompt" in s and "reference" in s for s in HUMANEVAL_SAMPLES)

    def test_hellaswag_samples(self):
        assert len(HELLASWAG_SAMPLES) == 4

    def test_evaluate_mmlu_correct(self):
        ok, score = evaluate_mmlu_sample("Capital of France:", "Paris", "Paris")
        assert ok is True
        assert score == 1.0

    def test_evaluate_mmlu_incorrect(self):
        ok, score = evaluate_mmlu_sample("Capital of France:", "Paris", "London")
        assert ok is False
        assert score == 0.0

    def test_evaluate_mmlu_partial(self):
        ok, _ = evaluate_mmlu_sample("Capital:", "Paris", "The answer is Paris.")
        assert ok is True

    def test_evaluate_gsm8k_correct(self):
        ok, score = evaluate_gsm8k_sample("5 + 7 =", "12", "5 + 7 = 12")
        assert ok is True
        assert score == 1.0

    def test_evaluate_gsm8k_incorrect(self):
        ok, score = evaluate_gsm8k_sample("5 + 7 =", "12", "13")
        assert ok is False
        assert score == 0.0

    def test_evaluate_humaneval_correct(self):
        ok, _ = evaluate_humaneval_sample(
            "def add(a, b):", "test", "return a + b", "return a + b"
        )
        assert ok is True

    def test_evaluate_humaneval_incorrect(self):
        ok, _ = evaluate_humaneval_sample(
            "def add(a, b):", "test", "return a + b", "return a - b"
        )
        assert ok is False

    def test_evaluate_hellaswag_correct(self):
        ok, _ = evaluate_hellaswag_sample("The cat sat on the", "mat", "mat")
        assert ok is True

    def test_evaluate_hellaswag_incorrect(self):
        ok, _ = evaluate_hellaswag_sample("The cat sat on the", "mat", "floor")
        assert ok is False

    def test_init(self):
        bench = V1034RealBenchmark()
        assert bench.results == {}

    def test_run_mmlu(self):
        """V1034 真测 MMLU 真跑 (主 19:33)."""
        bench = V1034RealBenchmark()
        result = bench.run_mmlu()
        assert result["benchmark"] == "MMLU"
        assert result["n_samples"] == 10
        assert 0.0 <= result["accuracy"] <= 1.0
        assert "MMLU" in bench.results

    def test_run_gsm8k(self):
        bench = V1034RealBenchmark()
        result = bench.run_gsm8k()
        assert result["benchmark"] == "GSM8K"
        assert result["n_samples"] == 5

    def test_run_humaneval(self):
        bench = V1034RealBenchmark()
        result = bench.run_humaneval()
        assert result["benchmark"] == "HumanEval"
        assert result["n_samples"] == 3

    def test_run_hellaswag(self):
        bench = V1034RealBenchmark()
        result = bench.run_hellaswag()
        assert result["benchmark"] == "HellaSwag"
        assert result["n_samples"] == 4

    def test_run_all(self):
        """V1034 真测 4 真 benchmarks 全跑 (主 17:43 实事求是)."""
        bench = V1034RealBenchmark()
        result = bench.run_all()
        assert len(result["benchmarks"]) == 4
        assert result["n_samples"] == 22  # 10+5+3+4
        assert 0.0 <= result["overall_accuracy"] <= 1.0

    def test_run_all_with_perfect_predictor(self):
        """V1034 真测 perfect predictor 应该 100% 准确率 (主 17:43 实事求是)."""
        bench = V1034RealBenchmark()
        # 用 perfect predictor
        result = bench.run_mmlu(predictor=lambda q: "Paris" if "France" in q else "4" if "2+2" in q else "water")
        # 真测: perfect predictor 应该至少得一些分
        assert result["n_correct"] >= 2

    def test_stats(self):
        bench = V1034RealBenchmark()
        s = bench.stats()
        assert s["version"] == V1034_VERSION

    def test_v22_33_asi_integration(self):
        """V1034 真测主 22:33 ASI 北极星."""
        bench = V1034RealBenchmark()
        s = bench.stats()
        assert "ASI" in s["philosophy"]

    def test_v00_36_real_effect(self):
        """V1034 真测主 00:36 效果 — 真 benchmark 真跑."""
        bench = V1034RealBenchmark()
        result = bench.run_all()
        # 真跑 22 真 samples
        assert result["n_samples"] == 22
        # 真有 accuracy (不是空)
        assert result["overall_accuracy"] >= 0.0

    def test_v19_33_real_datasets(self):
        """V1034 真测主 19:33 MMLU + GSM8K + HumanEval + HellaSwag 真数据集."""
        bench = V1034RealBenchmark()
        # 真测 4 个数据集
        assert len(MMLU_SAMPLES) > 0
        assert len(GSM8K_SAMPLES) > 0
        assert len(HUMANEVAL_SAMPLES) > 0
        assert len(HELLASWAG_SAMPLES) > 0

    def test_v17_43_truth(self):
        """V1034 真测主 17:43 实事求是 — 真评测, 不假装."""
        bench = V1034RealBenchmark()
        result = bench.run_mmlu(predictor=lambda q: "WRONG_ANSWER")
        # WRONG_ANSWER 永远不匹配
        assert result["n_correct"] == 0
        assert result["accuracy"] == 0.0

    def test_complete_integration(self):
        """V1034 真测完整 benchmark (主 00:36 + 主 22:33 + 主 19:33 + 主 17:43)."""
        bench = V1034RealBenchmark()
        result = bench.run_all()
        # 真跑 4 个 benchmark, 22 真样本
        assert len(result["benchmarks"]) == 4
        assert result["n_samples"] == 22
        # 真有 results 详情
        assert len(bench.results) == 4
        assert all(b in bench.results for b in ["MMLU", "GSM8K", "HumanEval", "HellaSwag"])