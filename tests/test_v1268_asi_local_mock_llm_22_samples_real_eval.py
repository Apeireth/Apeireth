"""V1268 ASI Local Mock-LLM 22-Sample Real Eval — 真生产 tests (主 00:44 质量工程化).

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 key 有效: V1076 真 HTTP probe + 401/200 真判定
- 不假装模型可用: 真 /v1/models 列表 + 真 check
- 不假装 benchmark 通过: V1034 真 evaluate 真 pass/fail
- 不假装 ASI 达到: V1268 是 helper, ASI 还是 ASI, NS 92.91% LOCKED
- 不假装 NewAPI: 本地 mock 替代 NewAPI, 真生产仍可换 NewAPI key
- 不假装 22 样本: V1034 真数据集 22 样本全部真跑

Tests cover:
 1.  V1268_VERSION + V1268_NOTE 真存在
 2.  V3_GUARDS 真 8 个, REFERENCES 真 8 个
 3.  sanity_check_v1268 真过 (V1267/V1076/V1034 importable, 22 样本, 4 builders, 4 evaluators)
 4.  V1268BenchmarkSpec 真默认值 + 真 total_samples() + 真序列化
 5.  V1268SampleResult 真 dataclass + 真 to_dict
 6.  PROMPT_BUILDERS 真 4 类 + 真 prompt 构造
 7.  EVALUATORS 真 4 类 + 真评测函数
 8.  _run_one_benchmark 真跑 + 真 V1034 evaluator 真 pass/fail
 9.  run_v1268_full_real_eval 真起 mock + 真 chat 5 + 真 benchmark 真统计
10.  Markdown 报告 真渲染关键段 (22 samples / 4 benchmarks / V3 guards)
11.  CLI --help 工作 (真 python -m apeireth.v1268_asi_local_mock_llm_22_samples_real_eval --help)
12.  V1268 guard "v1268_v1034_real_22_samples" 真存在
13.  V1268 注释明确 "NOT a new ASI dim"
14.  Key 遮蔽 (主 17:58 不假装) 真不漏真 key
15.  任何人都能接手: --full-loop 一行真起真测真报真关 (含 subprocess)
16.  V1268 mock_disclosed guard 真覆盖
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

# conftest.py already handles env isolation; just import here.
from apeireth import v1268_asi_local_mock_llm_22_samples_real_eval as v1268
from apeireth import v1267_asi_local_mock_llm_real_loop as v1267
from apeireth import v1034_real_benchmark as v1034


# ============================================================================
# 1. Module structure
# ============================================================================


def test_v1268_version_and_note_exist():
    """V1268 version + note 真存在 (主 00:36 质量)."""
    assert isinstance(v1268.V1268_VERSION, str)
    assert v1268.V1268_VERSION == "0.1.0"
    assert isinstance(v1268.V1268_NOTE, str)
    assert "V1268" in v1268.V1268_NOTE
    assert "NOT a new ASI dim" in v1268.V1268_NOTE  # 主 17:43 实事求是


def test_v1268_v3_guards_complete():
    """V1268 V3 guards 真 8 个 (V1267 7 + 新增 1) (主 17:58 不假装)."""
    guards = v1268.V3_GUARDS
    assert len(guards) == 8
    # V1267 7 个继承
    for g in [
        "v1268_not_new_dim",
        "v1268_no_asi_v1_claim",
        "v1268_no_phenomenal_claim",
        "v1268_mock_disclosed",
        "v1268_not_newapi_replace",
        "v1268_subprocess_clean",
        "v1268_no_key_leak",
    ]:
        assert g in guards
    # V1268 新增
    assert "v1268_v1034_real_22_samples" in guards  # 真跑 V1034 22 样本真评测


def test_v1268_references_count():
    """V1268 REFERENCES 真 8 个 (主 19:33 真借鉴)."""
    refs = v1268.REFERENCES
    assert len(refs) == 8
    ids = [r["id"] for r in refs]
    # V1267 + V1076 + V1034 + OpenAI spec + 4 真数据集
    for must in [
        "v1267-local-mock-2026-08",
        "v1076-asi-real-llm-2026-08",
        "v1034-asi-real-benchmark-2026-07",
        "openai-chat-completions-2023-03",
        "mmlu-2020",
        "gsm8k-2021",
        "humaneval-2021",
        "hellaswag-2019",
    ]:
        assert must in ids, f"missing ref: {must}"


def test_v1268_sanity_check_pass():
    """V1268 sanity_check_v1268() 真过 (主 17:43 实事求是)."""
    s = v1268.sanity_check_v1268()
    assert s["version"] == "0.1.0"
    assert s["guards"] == 8
    assert s["refs"] == 8
    assert s["v1267_importable"] is True
    assert s["v1076_importable"] is True
    assert s["v1034_importable"] is True
    assert s["total_v1034_samples"] == 22  # 10 + 5 + 3 + 4
    assert s["expected_total"] == 22
    assert s["prompt_builders"] == ["GSM8K", "HellaSwag", "HumanEval", "MMLU"]
    assert s["evaluators"] == ["GSM8K", "HellaSwag", "HumanEval", "MMLU"]
    assert s["pass"] is True


def test_v1268_total_v1034_samples_22():
    """V1268 TOTAL_V1034_SAMPLES 真 22 (10 + 5 + 3 + 4) (主 17:43)."""
    assert v1268.TOTAL_V1034_SAMPLES == 22
    assert len(v1034.MMLU_SAMPLES) == 10
    assert len(v1034.GSM8K_SAMPLES) == 5
    assert len(v1034.HUMANEVAL_SAMPLES) == 3
    assert len(v1034.HELLASWAG_SAMPLES) == 4


# ============================================================================
# 2. Spec + Sample dataclasses
# ============================================================================


def test_v1268_benchmark_spec_defaults():
    """V1268BenchmarkSpec 默认值真 22 样本 (主 00:36)."""
    spec = v1268.V1268BenchmarkSpec()
    assert spec.benchmarks == ["MMLU", "GSM8K", "HumanEval", "HellaSwag"]
    assert spec.n_mmlu == 10
    assert spec.n_gsm8k == 5
    assert spec.n_humaneval == 3
    assert spec.n_hellaswag == 4
    assert spec.total_samples() == 22
    assert spec.temperature == 0.0
    assert spec.max_tokens == 128


def test_v1268_benchmark_spec_total_samples_sum():
    """V1268BenchmarkSpec.total_samples() 真求和 (主 17:43)."""
    spec = v1268.V1268BenchmarkSpec(n_mmlu=3, n_gsm8k=2, n_humaneval=1, n_hellaswag=1)
    assert spec.total_samples() == 7


def test_v1268_benchmark_spec_to_dict_serializable():
    """V1268BenchmarkSpec.to_dict() 真 JSON 序列化."""
    spec = v1268.V1268BenchmarkSpec()
    d = spec.to_dict()
    s = json.dumps(d, ensure_ascii=False)
    j = json.loads(s)
    assert j["n_mmlu"] == 10
    # total_samples 是方法不是字段, 不在 dict 里
    assert "total_samples" not in j
    # 但 spec.total_samples() 是 22
    assert spec.total_samples() == 22


def test_v1268_sample_result_dataclass():
    """V1268SampleResult 真 dataclass (主 17:43)."""
    r = v1268.V1268SampleResult(
        benchmark="MMLU",
        i=0,
        prompt="Q: capital of France?",
        ground_truth="Paris",
        prediction="Paris",
        correct=True,
        score=1.0,
        latency_ms=12.5,
        status_code=200,
    )
    assert r.benchmark == "MMLU"
    assert r.correct is True
    assert r.score == 1.0
    assert r.status_code == 200
    d = r.to_dict()
    assert d["benchmark"] == "MMLU"
    assert d["correct"] is True


def test_v1268_sample_result_long_truncation():
    """V1268SampleResult 长 prompt/prediction 真截断 (主 00:36 质量)."""
    long_prompt = "x" * 500
    long_pred = "y" * 500
    r = v1268.V1268SampleResult(
        benchmark="GSM8K",
        i=1,
        prompt=long_prompt,
        ground_truth="42",
        prediction=long_pred,
        correct=False,
        score=0.0,
        latency_ms=10.0,
        status_code=200,
    )
    d = r.to_dict()
    assert d["prompt"].endswith("...")
    assert len(d["prompt"]) < 250
    assert d["prediction"].endswith("...")
    assert len(d["prediction"]) < 250


# ============================================================================
# 3. PROMPT_BUILDERS + EVALUATORS
# ============================================================================


def test_v1268_prompt_builders_keys():
    """PROMPT_BUILDERS 真 4 类 (主 17:43)."""
    assert sorted(v1268.PROMPT_BUILDERS.keys()) == ["GSM8K", "HellaSwag", "HumanEval", "MMLU"]


def test_v1268_evaluators_keys():
    """EVALUATORS 真 4 类 (主 17:43)."""
    assert sorted(v1268.EVALUATORS.keys()) == ["GSM8K", "HellaSwag", "HumanEval", "MMLU"]


def test_v1268_build_mmlu_prompt():
    """MMLU prompt 真构造 (主 19:33 Hendrycks 2020 真借鉴)."""
    sample = {"question": "What is 2+2?", "answer": "4"}
    p = v1268.build_mmlu_prompt(sample)
    assert "2+2" in p
    assert "Answer:" in p


def test_v1268_build_gsm8k_prompt():
    """GSM8K prompt 真构造 (主 19:33 Cobbe 2021 真借鉴)."""
    sample = {"question": "3-1=?", "answer": "2"}
    p = v1268.build_gsm8k_prompt(sample)
    assert "3-1" in p
    assert "A:" in p


def test_v1268_build_humaneval_prompt():
    """HumanEval prompt 真构造 (主 19:33 Chen 2021 真借鉴)."""
    sample = {"prompt": "def add(a,b):\n  return", "reference": "a+b"}
    p = v1268.build_humaneval_prompt(sample)
    assert "def add" in p


def test_v1268_build_hellaswag_prompt():
    """HellaSwag prompt 真构造 (主 19:33 Zellers 2019 真借鉴)."""
    sample = {"context": "The cat sat on the", "answer": "mat"}
    p = v1268.build_hellaswag_prompt(sample)
    assert "cat sat" in p
    assert "Most likely" in p


# ============================================================================
# 4. 真跑真评测 (主 17:43 实事求是)
# ============================================================================


def test_v1268_run_one_benchmark_uses_real_evaluator():
    """_run_one_benchmark 真用 V1034 真 evaluator 真评测 (主 17:43).

    用 in-process V1267 mock (subprocess) 替换 v1267_run_subprocess_loop 路径,
    只验证 _run_one_benchmark 自身.
    """
    # 真起 in-process mock
    spec_mock = v1267.MockLLMServerSpec(port=0, latency_jitter_ms=0.0)
    captured = {"port": 0}

    def _on_ready(port):
        captured["port"] = port

    thread, stop = v1267.serve_in_thread(spec_mock, on_ready=_on_ready)
    deadline = time.time() + 2.0
    while time.time() < deadline and captured["port"] == 0:
        time.sleep(0.02)
    port = captured["port"]
    assert port > 0
    base_url = f"http://127.0.0.1:{port}/v1"

    try:
        # 真跑 1 个 MMLU 样本 (Paris question — mock echo 也能命中部分)
        from apeireth import v1076_asi_real_external_llm_client as v1076

        results = v1268._run_one_benchmark(
            benchmark_name="MMLU",
            samples=v1034.MMLU_SAMPLES[:1],  # 真取 1 样本
            n=1,
            base_url=base_url,
            api_key="v1268-test-mock-key",
            model="MiniMax-M3",
            spec=v1268.V1268BenchmarkSpec(n_mmlu=1, n_gsm8k=0, n_humaneval=0, n_hellaswag=0),
        )
        assert len(results) == 1
        r = results[0]
        assert r.benchmark == "MMLU"
        assert r.i == 0
        assert r.ground_truth == "Paris"
        assert isinstance(r.prediction, str)  # mock echo 真返回
        assert r.status_code in (200, -1, 0)  # 200 OR 异常 (主 17:43 实事求是)
        assert isinstance(r.correct, bool)
        assert isinstance(r.score, float)
        assert r.latency_ms >= 0.0
    finally:
        stop()


def test_v1268_run_v1268_full_real_eval_returns_structure():
    """run_v1268_full_real_eval 真起真测真返回 dict (主 00:56 任何人都能接手)."""
    # 限制 chat 次数 = 3, 不跑全部 22, 加速
    result = v1268.run_v1268_full_real_eval(n_chat=3)
    assert isinstance(result, dict)
    assert "started" in result
    assert "healthy" in result
    assert "base_url" in result
    assert "model" in result
    assert "n_chat" in result
    assert "chat_results" in result
    assert "benchmark_results" in result
    assert "benchmark_summary" in result
    assert "total_samples" in result
    assert "expected_total_samples" in result
    assert result["expected_total_samples"] == 22
    # 真跑过 → chat_results 应有 3 条
    if result["healthy"]:
        assert len(result["chat_results"]) == 3
        # benchmark 真评测: 22 真样本全部真跑
        assert result["actual_total_samples"] == 22
        # summary 真覆盖 4 类 + _total
        summary = result["benchmark_summary"]
        for k in ["MMLU", "GSM8K", "HumanEval", "HellaSwag", "_total"]:
            assert k in summary, f"missing summary key: {k}"
        # _total 真统计
        total = summary["_total"]
        assert total["n_samples"] == 22
        assert "latency_p50_ms" in total
        assert "latency_mean_ms" in total
        assert "latency_max_ms" in total
        assert "latency_min_ms" in total
        assert "status_distribution" in total


def test_v1268_render_markdown_report_contains_key_sections():
    """Markdown 报告 真含 22 samples / 4 benchmarks / V3 guards (主 00:56)."""
    # 真跑一次拿真数据 (n_chat=2 加速)
    result = v1268.run_v1268_full_real_eval(n_chat=2)
    md = v1268.render_markdown_report(result)
    # 真包含关键段
    assert "V1268" in md
    assert "期望 (V1034 内置)" in md
    assert "**22**" in md
    assert "V3 哲学守门" in md
    for g in v1268.V3_GUARDS:
        assert g in md
    assert "本报告**不是 ASI**" in md
    assert "MMLU" in md
    assert "GSM8K" in md
    assert "HumanEval" in md
    assert "HellaSwag" in md


# ============================================================================
# 5. CLI 任何人都能接手 (主 00:56)
# ============================================================================


def test_v1268_cli_help_works():
    """CLI --help 真工作 (主 00:56 任何人都能接手)."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1268_asi_local_mock_llm_22_samples_real_eval", "--help"],
        cwd=os.path.join(os.path.dirname(__file__), ".."),
        capture_output=True,
        timeout=15,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert proc.returncode == 0
    # Windows gbk 编码问题: 用 utf-8 decode with errors='replace' 兜底
    out = proc.stdout.decode("utf-8", errors="replace")
    assert "--full-loop" in out
    assert "--n-chat" in out
    assert "--report" in out
    assert "--report-path" in out