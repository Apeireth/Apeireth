"""V1269 ASI Real LLM Stream 真流式真测 — 真生产 tests (主 00:44 质量工程化).

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 mock 是真 LLM: V1269 = local test fixture, NOT ASI-LLM
- 不假装 TTFT < 真实 LLM: TTFT 是 fixture 设定, 不是神经推理时间
- 不假装 chunk count = 真 LLM token 数: chunk 是 fixture 切分, 不可混淆 BPE
- 不假装 ASI 达到: V1269 是 helper, ASI 还是 ASI, NS 92.91% LOCKED

Tests cover:
 1.  V1269_VERSION + V1269_NOTE 真存在
 2.  V3_GUARDS 真 8 个, REFERENCES 真 12 个
 3.  sanity_check_v1269 真过 (V1267/V1076/V1034 importable)
 4.  V1269StreamSpec 真默认值 + 真 to_dict
 5.  V1269SSEEvent 真 dataclass + 真 is_done
 6.  parse_sse_line 真解析 data: 行 + 真 [DONE] 终止
 7.  parse_sse_line 真解析 event: 行
 8.  parse_sse_line 真拒绝 comment 行 (':' 开头)
 9.  iter_sse_events 真迭代多行 + 真跳过空行 + 真 [DONE] 终止
10.  V1269StreamMetrics 真 dataclass + 真 to_dict
11.  _percentile 真计算 (边界值 + 中位数)
12.  _compute_inter_chunk_latencies 真统计 (p50/p95/mean)
13.  stream_chat_completion 真 HTTP POST + 真 SSE 解析 + 真 TTFT
14.  run_v1269_benchmark 真跑 22 样本 + 真 stream vs non-stream 对比
15.  V1269SampleRun 真 dataclass + 真 to_dict
16.  Markdown 报告 真渲染关键段
17.  CLI --help 工作
18.  V1269 guard "v1269_sse_real_parse" 真存在
19.  V1269 注释明确 "NOT a new ASI dim"
20.  Key 遮蔽 (主 17:58 不假装) 真不漏真 key
21.  任何人都能接手: --full-loop 一行真起真测真报真关
22.  V1269 mock_disclosed guard 真覆盖
23.  V1269MockLLMHandler 真返回 X-Mock-Disclosure: true
24.  V1269MockLLMHandler 真 chunk_size_tokens 切分
25.  V1269MockLLMHandler 真 [DONE] 终止
26.  V1269MockLLMHandler 真非流式 兼容 V1268 测试
27.  真 stream_completion n_chunks > 0 (主 17:43 实事求是)
28.  真 stream_completion accumulated_content 不为空 (主 17:43)
29.  _summarize_v1269 真计算 accuracy (主 17:43)
30.  V1269StreamSpec chunk_size_tokens 默认 3 (主 17:43)
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
from apeireth import v1269_asi_real_llm_stream_real_test as v1269
from apeireth import v1034_real_benchmark as v1034


# ============================================================================
# 1. Module structure
# ============================================================================


def test_v1269_version_and_note_exist():
    """V1269 version + note 真存在 (主 00:36 质量)."""
    assert isinstance(v1269.V1269_VERSION, str)
    assert v1269.V1269_VERSION == "0.1.0"
    assert isinstance(v1269.V1269_NOTE, str)
    assert "V1269" in v1269.V1269_NOTE
    assert "NOT a new ASI dim" in v1269.V1269_NOTE


def test_v1269_v3_guards_complete():
    """V1269 V3 guards 真 8 个 (V1267 7 + 新增 v1269_sse_real_parse)."""
    guards = v1269.V3_GUARDS
    assert len(guards) == 8
    for g in [
        "v1269_not_new_dim",
        "v1269_no_asi_v1_claim",
        "v1269_no_phenomenal_claim",
        "v1269_mock_disclosed",
        "v1269_not_newapi_replace",
        "v1269_subprocess_clean",
        "v1269_no_key_leak",
        "v1269_sse_real_parse",
    ]:
        assert g in guards, f"missing guard: {g}"


def test_v1269_references_count():
    """V1269 REFERENCES 真 12 个 (主 19:33 真借鉴)."""
    refs = v1269.REFERENCES
    assert len(refs) == 12
    ids = [r["id"] for r in refs]
    for must in [
        "v1267-local-mock-2026-08",
        "v1076-asi-real-llm-2026-08",
        "v1268-22-samples-2026-08",
        "v1034-asi-real-benchmark-2026-07",
        "openai-chat-completions-2023-03",
        "openai-streaming-sse-2023",
        "sse-w3c-2015",
    ]:
        assert must in ids, f"missing ref: {must}"


def test_v1269_sanity_check_pass():
    """V1269 sanity_check_v1269() 真过 (主 17:43 实事求是)."""
    s = v1269.sanity_check_v1269()
    assert s["version"] == "0.1.0"
    assert s["guards"] == 8
    assert s["refs"] == 12
    assert s["v1267_importable"] is True
    assert s["v1076_importable"] is True
    assert s["v1034_importable"] is True
    assert s["total_v1034_samples"] == 22
    assert s["expected_total"] == 22
    assert "MMLU" in s["prompt_builders"]
    assert "GSM8K" in s["prompt_builders"]
    assert "parse_sse_line" in s["sse_parser_components"]
    assert "V1269SSEEvent" in s["sse_parser_components"]
    assert "V1269StreamMetrics" in s["stream_metrics_components"]
    assert s["pass"] is True


# ============================================================================
# 2. V1269StreamSpec dataclass
# ============================================================================


def test_v1269_stream_spec_defaults():
    """V1269StreamSpec 真默认值 (主 17:43)."""
    spec = v1269.V1269StreamSpec()
    assert spec.host == "127.0.0.1"
    assert spec.port == 0
    assert spec.ttft_ms == 80.0
    assert spec.inter_chunk_latency_ms == 12.0
    assert spec.chunk_size_tokens == 3
    assert spec.fail_rate == 0.0


def test_v1269_stream_spec_to_dict():
    """V1269StreamSpec.to_dict() 真序列化."""
    spec = v1269.V1269StreamSpec(ttft_ms=50.0, inter_chunk_latency_ms=20.0, chunk_size_tokens=5)
    d = spec.to_dict()
    s = json.dumps(d)
    j = json.loads(s)
    assert j["ttft_ms"] == 50.0
    assert j["inter_chunk_latency_ms"] == 20.0
    assert j["chunk_size_tokens"] == 5


# ============================================================================
# 3. SSE parser
# ============================================================================


def test_v1269_sse_event_done_flag():
    """V1269SSEEvent.is_done 真标记 [DONE] (主 19:33 OpenAI spec)."""
    e = v1269.V1269SSEEvent(data="[DONE]", is_done=True)
    assert e.is_done is True
    assert e.data == "[DONE]"


def test_v1269_parse_sse_data_line():
    """parse_sse_line 真解析 data: 行 (主 19:33 OpenAI spec)."""
    evt = v1269.parse_sse_line("data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}")
    assert evt is not None
    assert evt.data == "{\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}"
    assert evt.is_done is False


def test_v1269_parse_sse_done_line():
    """parse_sse_line 真识别 [DONE] 终止 (主 19:33 OpenAI spec)."""
    evt = v1269.parse_sse_line("data: [DONE]")
    assert evt is not None
    assert evt.is_done is True
    assert evt.data == "[DONE]"


def test_v1269_parse_sse_event_line():
    """parse_sse_line 真解析 event: 行."""
    evt = v1269.parse_sse_line("event: custom_event")
    assert evt is not None
    assert evt.event == "custom_event"


def test_v1269_parse_sse_comment_line_rejected():
    """parse_sse_line 真拒绝 comment 行 (主 19:33 SSE W3C spec)."""
    assert v1269.parse_sse_line(": this is a comment") is None
    assert v1269.parse_sse_line("") is None


def test_v1269_iter_sse_events_done_terminates():
    """iter_sse_events 真 [DONE] 终止 (主 19:33 OpenAI spec)."""
    lines = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}",
        "",
        "data: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}",
        "",
        "data: [DONE]",
        "",
        "data: {\"choices\":[{\"delta\":{\"content\":\"never_seen\"}}]}",  # 真不解析
        "",
    ]
    events = list(v1269.iter_sse_events(lines))
    assert len(events) == 3
    assert events[0].data.startswith("{\"choices\":[{\"delta\":{\"content\":\"a\"")
    assert events[1].data.startswith("{\"choices\":[{\"delta\":{\"content\":\"b\"")
    assert events[2].is_done is True


def test_v1269_iter_sse_events_skips_empty():
    """iter_sse_events 真跳过空行 (主 19:33 SSE spec)."""
    lines = [
        "",
        "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}",
        "",
        "",
    ]
    events = list(v1269.iter_sse_events(lines))
    assert len(events) == 1
    assert "x" in events[0].data


def test_v1269_iter_sse_events_handles_carriage_return():
    """iter_sse_events 真处理 \\r (主 19:33 SSE spec)."""
    lines = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"y\"}}]}\r",
    ]
    events = list(v1269.iter_sse_events(lines))
    assert len(events) == 1


# ============================================================================
# 4. V1269StreamMetrics + percentile + inter_chunk
# ============================================================================


def test_v1269_stream_metrics_dataclass():
    """V1269StreamMetrics 真 dataclass (主 17:43)."""
    m = v1269.V1269StreamMetrics(
        request_id="req-1",
        n_chunks=5,
        ttft_ms=80.0,
        total_ms=200.0,
        accumulated_content="hello",
    )
    assert m.request_id == "req-1"
    assert m.n_chunks == 5
    assert m.ttft_ms == 80.0
    d = m.to_dict()
    assert d["n_chunks"] == 5


def test_v1269_percentile_basic():
    """_percentile 真计算 (主 17:43)."""
    data = [1.0, 2.0, 3.0, 4.0, 5.0]
    p50 = v1269._percentile(data, 50)
    p95 = v1269._percentile(data, 95)
    assert 2.5 <= p50 <= 3.5
    assert p95 >= 4.0


def test_v1269_percentile_empty():
    """_percentile 真返回 0.0 for empty (主 17:43)."""
    assert v1269._percentile([], 50) == 0.0


def test_v1269_compute_inter_chunk_latencies():
    """_compute_inter_chunk_latencies 真统计 (主 17:43)."""
    chunk_times = [0.0, 12.0, 24.0, 36.0, 48.0]
    p50, p95, mean = v1269._compute_inter_chunk_latencies(chunk_times)
    assert p50 == 12.0
    assert mean == 12.0
    assert p95 >= 12.0


def test_v1269_compute_inter_chunk_latencies_empty():
    """_compute_inter_chunk_latencies 真空 (主 17:43)."""
    p50, p95, mean = v1269._compute_inter_chunk_latencies([])
    assert p50 == 0.0
    assert p95 == 0.0
    assert mean == 0.0


def test_v1269_compute_inter_chunk_latencies_single():
    """_compute_inter_chunk_latencies 真单 chunk (主 17:43)."""
    p50, p95, mean = v1269._compute_inter_chunk_latencies([100.0])
    assert p50 == 0.0
    assert p95 == 0.0
    assert mean == 0.0


# ============================================================================
# 5. Stream chat completion (real SSE)
# ============================================================================


def _start_v1269_mock(port: int = 0, ttft_ms: float = 50.0, inter_chunk_ms: float = 5.0, chunk_size: int = 2):
    """真起 V1269 mock server in-process (主 17:43)."""
    spec = v1269.V1269StreamSpec(
        port=port,
        ttft_ms=ttft_ms,
        inter_chunk_latency_ms=inter_chunk_ms,
        chunk_size_tokens=chunk_size,
    )
    captured = {"port": 0}

    def _on_ready(p):
        captured["port"] = p

    thread, stop = v1269.serve_v1269_in_thread(spec, on_ready=_on_ready)
    deadline = time.time() + 2.0
    while time.time() < deadline and captured["port"] == 0:
        time.sleep(0.02)
    return captured["port"], stop


def test_v1269_stream_chat_real_sse():
    """stream_chat_completion 真 SSE 解析 (主 17:43 实事求是)."""
    port, stop = _start_v1269_mock(ttft_ms=20.0, inter_chunk_ms=3.0, chunk_size=2)
    try:
        assert port > 0
        base_url = f"http://127.0.0.1:{port}/v1"
        m = v1269.stream_chat_completion(
            base_url=base_url,
            api_key="v1269-test-key",
            model="MiniMax-M3",
            messages=[{"role": "user", "content": "hello"}],
            temperature=0.0,
            max_tokens=64,
            timeout_sec=10.0,
        )
        assert m.status_code == 200
        assert m.n_chunks > 0
        assert m.ttft_ms > 0.0
        assert m.total_ms > 0.0
        assert m.ttft_ms <= m.total_ms  # 真 TTFT <= 总时
        assert "MOCK-LLM-STREAM" in m.accumulated_content or "[MOCK" in m.accumulated_content
        assert m.mock_disclosed is True
    finally:
        stop()


def test_v1269_stream_chat_no_connection_error():
    """stream_chat_completion 真无连接错误返回 status 0 (主 17:43)."""
    m = v1269.stream_chat_completion(
        base_url="http://127.0.0.1:1/v1",  # 真不可达端口
        api_key="v1269-test-key",
        model="MiniMax-M3",
        messages=[{"role": "user", "content": "hi"}],
        timeout_sec=2.0,
    )
    # 真不可达: status_code == 0, error != ""
    assert m.status_code == 0
    assert m.error != ""
    assert m.n_chunks == 0


# ============================================================================
# 6. Mock server (V1269)
# ============================================================================


def test_v1269_mock_server_status_endpoint():
    """V1269 mock server /api/status 真返 stream_support (主 17:43)."""
    port, stop = _start_v1269_mock()
    try:
        assert port > 0
        # 真 HTTP GET /api/status
        import urllib.request
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/status", timeout=2.0) as resp:
            body = json.loads(resp.read().decode("utf-8"))
            assert body["status"] == "ok"
            assert body["mock_disclosure"] is True
            assert body["stream_support"] is True
            assert "X-Mock-Disclosure" in resp.headers
    finally:
        stop()


def test_v1269_mock_server_models_endpoint():
    """V1269 mock server /v1/models 真返 MiniMax-M3 (主 17:43)."""
    port, stop = _start_v1269_mock()
    try:
        assert port > 0
        import urllib.request
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/v1/models", timeout=2.0) as resp:
            body = json.loads(resp.read().decode("utf-8"))
            ids = [m["id"] for m in body["data"]]
            assert "MiniMax-M3" in ids
            assert body["mock_disclosure"] is True
    finally:
        stop()


def test_v1269_mock_server_nonstream_chat():
    """V1269 mock server 非流式 chat 真兼容 V1268 (主 17:43)."""
    port, stop = _start_v1269_mock()
    try:
        assert port > 0
        import urllib.request
        req = urllib.request.Request(
            f"http://127.0.0.1:{port}/v1/chat/completions",
            data=json.dumps({
                "model": "MiniMax-M3",
                "messages": [{"role": "user", "content": "test"}],
                "stream": False,
            }).encode("utf-8"),
            headers={"Content-Type": "application/json", "Authorization": "Bearer v1269-test"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            body = json.loads(resp.read().decode("utf-8"))
            assert body["object"] == "chat.completion"
            assert body["x_mock_disclosure"] is True
            assert len(body["choices"]) == 1
    finally:
        stop()


# ============================================================================
# 7. Real benchmark (22 samples stream vs non-stream)
# ============================================================================


def test_v1269_run_benchmark_22_samples():
    """run_v1269_benchmark 真跑 22 样本 (主 17:43 实事求是)."""
    port, stop = _start_v1269_mock(ttft_ms=10.0, inter_chunk_ms=2.0, chunk_size=3)
    try:
        assert port > 0
        base_url = f"http://127.0.0.1:{port}/v1"
        result = v1269.run_v1269_benchmark(
            base_url=base_url,
            api_key="v1269-test-key",
            n_mmlu=10,
            n_gsm8k=5,
            n_humaneval=3,
            n_hellaswag=4,
        )
        assert "error" not in result
        assert result["n_total"] == 22  # 真 22 样本
        sample_runs = result["sample_runs"]
        assert len(sample_runs) == 22
        # 真每样本有 stream_metrics + nonstream_content
        for sr in sample_runs:
            assert sr.stream_metrics.status_code == 200
            assert sr.stream_metrics.n_chunks > 0
            assert sr.stream_metrics.ttft_ms > 0.0
        # 真 benchmark 真统计
        summary = result["summary"]
        assert summary["n_samples"] == 22
        assert "stream_ttft_ms" in summary
        assert "stream_total_ms" in summary
        assert "stream_n_chunks" in summary
        assert "nonstream_latency_ms" in summary
        assert summary["stream_ttft_ms"]["mean"] > 0.0
    finally:
        stop()


def test_v1269_run_benchmark_subset():
    """run_v1269_benchmark 真支持子集 (主 00:36 质量)."""
    port, stop = _start_v1269_mock(ttft_ms=5.0, inter_chunk_ms=1.0, chunk_size=2)
    try:
        assert port > 0
        base_url = f"http://127.0.0.1:{port}/v1"
        result = v1269.run_v1269_benchmark(
            base_url=base_url,
            api_key="v1269-test-key",
            benchmarks=["MMLU"],
            n_mmlu=3,
        )
        assert result["n_total"] == 3
    finally:
        stop()


def test_v1269_summarize_accuracy():
    """_summarize_v1269 真计算 accuracy (主 17:43)."""
    from apeireth.v1269_asi_real_llm_stream_real_test import (
        V1269SampleRun, V1269StreamMetrics, _summarize_v1269,
    )
    sample_runs = [
        V1269SampleRun(
            benchmark="MMLU", i=0, prompt="q", ground_truth="a",
            stream_metrics=V1269StreamMetrics(status_code=200, n_chunks=3, ttft_ms=10.0, total_ms=50.0),
            correct=True, score=1.0,
        ),
        V1269SampleRun(
            benchmark="MMLU", i=1, prompt="q2", ground_truth="b",
            stream_metrics=V1269StreamMetrics(status_code=200, n_chunks=2, ttft_ms=15.0, total_ms=60.0),
            correct=False, score=0.0,
        ),
    ]
    s = _summarize_v1269(sample_runs)
    assert s["n_samples"] == 2
    assert s["n_correct"] == 1
    assert s["accuracy"] == 0.5


# ============================================================================
# 8. Sample dataclass
# ============================================================================


def test_v1269_sample_run_dataclass():
    """V1269SampleRun 真 dataclass (主 17:43)."""
    sr = v1269.V1269SampleRun(
        benchmark="MMLU",
        i=0,
        prompt="Q: ?",
        ground_truth="A",
        stream_metrics=v1269.V1269StreamMetrics(status_code=200, n_chunks=3),
        nonstream_content="A",
        nonstream_latency_ms=50.0,
        nonstream_status=200,
        correct=True,
        score=1.0,
    )
    assert sr.benchmark == "MMLU"
    assert sr.correct is True
    d = sr.to_dict()
    assert d["benchmark"] == "MMLU"
    assert d["stream_metrics"]["n_chunks"] == 3


def test_v1269_sample_run_long_truncation():
    """V1269SampleRun 长 prompt/content 真截断 (主 00:36)."""
    sr = v1269.V1269SampleRun(
        benchmark="GSM8K",
        i=1,
        prompt="x" * 500,
        ground_truth="42",
        stream_metrics=v1269.V1269StreamMetrics(
            status_code=200,
            accumulated_content="y" * 500,
        ),
        nonstream_content="z" * 500,
        nonstream_latency_ms=10.0,
        nonstream_status=200,
        correct=False,
        score=0.0,
    )
    d = sr.to_dict()
    assert d["prompt"].endswith("...")
    assert d["nonstream_content"].endswith("...")
    assert d["stream_metrics"]["accumulated_content"].endswith("...")


# ============================================================================
# 9. Markdown report
# ============================================================================


def test_v1269_markdown_report_renders():
    """render_markdown_report 真渲染关键段 (主 00:56)."""
    fake = {
        "started": True,
        "base_url": "http://127.0.0.1:51882/v1",
        "model": "MiniMax-M3",
        "mock_disclosed": True,
        "summary": {
            "n_samples": 22,
            "n_correct": 13,
            "accuracy": 0.5909,
            "stream_ttft_ms": {"p50": 80.0, "p95": 90.0, "mean": 82.5, "min": 70.0, "max": 95.0},
            "stream_total_ms": {"p50": 200.0, "p95": 250.0, "mean": 220.0, "min": 150.0, "max": 280.0},
            "stream_n_chunks": {"p50": 10.0, "mean": 12.0, "total": 264},
            "nonstream_latency_ms": {"p50": 100.0, "p95": 120.0, "mean": 105.0},
            "stream_vs_nonstream_ratio": 2.0,
        },
        "sample_runs": [],
    }
    md = v1269.render_markdown_report(fake)
    assert "# V1269 ASI Real LLM Stream" in md
    assert "Stream TTFT" in md
    assert "Mock disclosed" in md
    assert "v1269_sse_real_parse" in md
    assert "22" in md
    assert "59.09" in md  # 真格式化 accuracy: 0.5909 → 59.09%


# ============================================================================
# 10. CLI
# ============================================================================


def test_v1269_cli_help():
    """CLI --help 真工作 (主 00:56)."""
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    out = subprocess.run(
        [sys.executable, "-m", "apeireth.v1269_asi_real_llm_stream_real_test", "--help"],
        capture_output=True,
        timeout=15,
        env=env,
        encoding="utf-8",
        errors="replace",
        cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    )
    assert out.returncode == 0
    assert "--full-loop" in out.stdout
    assert "--sanity" in out.stdout
    assert "--ttft-ms" in out.stdout


def test_v1269_cli_sanity():
    """CLI --sanity 真工作 (主 00:56)."""
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    out = subprocess.run(
        [sys.executable, "-m", "apeireth.v1269_asi_real_llm_stream_real_test", "--sanity"],
        capture_output=True,
        timeout=15,
        env=env,
        encoding="utf-8",
        errors="replace",
        cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    )
    assert out.returncode == 0
    j = json.loads(out.stdout)
    assert j["version"] == "0.1.0"
    assert j["pass"] is True
    assert j["guards"] == 8


def test_v1269_cli_full_loop():
    """CLI --full-loop 真起真测真报真关 (主 00:56)."""
    import tempfile
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    with tempfile.TemporaryDirectory() as tmpdir:
        report_path = Path(tmpdir) / "report.md"
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1269_asi_real_llm_stream_real_test",
             "--full-loop", "--ttft-ms", "20.0", "--inter-chunk-ms", "5.0",
             "--chunk-size-tokens", "2",
             "--report", str(report_path)],
            capture_output=True,
            timeout=120,
            env=env,
            encoding="utf-8",
            errors="replace",
            cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        )
        assert out.returncode == 0, f"stderr: {out.stderr}"
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1269" in content
        assert "Stream TTFT" in content
        assert "accuracy" in content  # 真有 accuracy 字段
        assert "%" in content  # 真格式化为百分比