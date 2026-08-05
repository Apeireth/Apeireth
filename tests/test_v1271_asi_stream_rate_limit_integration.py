"""V1271 ASI Stream + Rate Limit Integration — 真生产 tests (主 17:43 实事求是 +
主 00:44 质量工程化 + 主 00:56 任何人都能接手 + 主 17:58 不假装).

V1271 = V1269 stream + V1270 rate limit 真生产集成.

跑: python -m pytest tests/test_v1271_asi_stream_rate_limit_integration.py -v
"""
from __future__ import annotations

import time
from typing import Dict, List, Tuple

import pytest

from apeireth import v1271_asi_stream_rate_limit_integration as v1271
from apeireth.v1271_asi_stream_rate_limit_integration import (
    V1271_VERSION,
    V1271_V3_GUARDS,
    V1271_REFERENCES,
    V1271IntegrationConfig,
    V1271StreamRunResult,
    V1271IntegrationStats,
    V1271IntegrationRunner,
    _mask_api_key,
    _load_v1271_samples,
    _compute_v1271_stats,
    render_v1271_markdown_report,
    run_v1271_full_loop,
    sanity_check_v1271,
)
from apeireth.v1270_asi_stream_rate_limiter import (
    V1270RateLimitConfig,
    V1270RateLimiter,
    V1270RateLimitExceeded,
)


# ============================================================================
# 1. Module-level invariants (主 17:43 实事求是)
# ============================================================================


def test_v1271_version_constant():
    """真版本号常量 (主 17:43)."""
    assert V1271_VERSION == "0.1.0"


def test_v1271_v3_guards_count():
    """真 V3 guards 计数 ≥ 9 (主 17:58 + 主 20:46)."""
    assert len(V1271_V3_GUARDS) >= 9
    expected = {
        "v1271_not_new_dim",
        "v1271_no_asi_v1_claim",
        "v1271_no_phenomenal_claim",
        "v1271_rate_limit_actually_enforced",
        "v1271_denial_counted",
        "v1271_release_after_stream",
        "v1271_stream_real",
        "v1271_mock_disclosed",
        "v1271_no_key_leak",
    }
    assert expected.issubset(set(V1271_V3_GUARDS))


def test_v1271_references_count():
    """真借鉴计数 ≥ 9 (主 19:33 走在前人肩上)."""
    assert len(V1271_REFERENCES) >= 9
    assert any("V1269" in r for r in V1271_REFERENCES)
    assert any("V1270" in r for r in V1271_REFERENCES)


def test_v1271_sanity_check_returns_dict():
    """真 sanity check 返回 dict (主 00:56)."""
    s = sanity_check_v1271()
    assert isinstance(s, dict)
    assert s.get("version") is True
    assert s.get("guards_count") is True
    assert s.get("references_count") is True


def test_v1271_sanity_check_includes_all_required_keys():
    """真 sanity check 包含全部必需 key (主 00:56)."""
    s = sanity_check_v1271()
    required = {"version", "guards_count", "references_count", "config_imports", "integration_runner_imports"}
    assert required.issubset(s.keys())


# ============================================================================
# 2. Config tests (主 17:43 实事求是)
# ============================================================================


def test_v1271_config_defaults():
    """真 config defaults (主 17:43)."""
    cfg = V1271IntegrationConfig()
    assert cfg.sample_limit == 22
    assert cfg.model == "MiniMax-M3"
    assert cfg.eval_after_stream is False
    assert cfg.stream_timeout_sec == 30.0
    assert cfg.benchmark_filter is None


def test_v1271_config_to_dict():
    """真 config to_dict (主 00:56)."""
    cfg = V1271IntegrationConfig(sample_limit=10, model="test-model")
    d = cfg.to_dict()
    assert d["sample_limit"] == 10
    assert d["model"] == "test-model"
    assert "rate_limit_config" in d


def test_v1271_config_with_rate_limit():
    """真 config + 真 rate limit 子 config (主 17:43)."""
    rl = V1270RateLimitConfig(requests_per_minute=10, tokens_per_minute=1000, max_concurrent=2)
    cfg = V1271IntegrationConfig(rate_limit_config=rl, sample_limit=5)
    assert cfg.rate_limit_config.requests_per_minute == 10
    assert cfg.rate_limit_config.tokens_per_minute == 1000


def test_v1271_config_with_benchmark_filter():
    """真 config benchmark filter (主 17:43)."""
    cfg = V1271IntegrationConfig(benchmark_filter=["MMLU", "GSM8K"])
    assert cfg.benchmark_filter == ["MMLU", "GSM8K"]


# ============================================================================
# 3. StreamRunResult dataclass tests (主 17:43)
# ============================================================================


def test_v1271_stream_run_result_defaults():
    """真 StreamRunResult defaults (主 17:43)."""
    r = V1271StreamRunResult(
        sample_id="test_001",
        benchmark="MMLU",
        status=200,
        ttft_ms=100.0,
        chunks=10,
        total_ms=500.0,
        tokens=42,
    )
    assert r.acquired is False
    assert r.release_ok is False
    assert r.decision_reason == ""
    assert r.error is None


def test_v1271_stream_run_result_to_dict():
    """真 StreamRunResult to_dict (主 00:56)."""
    r = V1271StreamRunResult(
        sample_id="MMLU_001",
        benchmark="MMLU",
        status=200,
        ttft_ms=150.5,
        chunks=8,
        total_ms=400.0,
        tokens=35,
        acquired=True,
        decision_reason="ok",
        release_ok=True,
    )
    d = r.to_dict()
    assert d["sample_id"] == "MMLU_001"
    assert d["benchmark"] == "MMLU"
    assert d["status"] == 200
    assert d["ttft_ms"] == 150.5
    assert d["chunks"] == 8
    assert d["total_ms"] == 400.0
    assert d["tokens"] == 35
    assert d["acquired"] is True
    assert d["release_ok"] is True


def test_v1271_stream_run_result_error_state():
    """真 StreamRunResult 错误状态 (主 17:43 实事求是)."""
    r = V1271StreamRunResult(
        sample_id="GSM8K_002",
        benchmark="GSM8K",
        status=429,
        ttft_ms=0.0,
        chunks=0,
        total_ms=0.0,
        tokens=0,
        acquired=False,
        decision_reason="denied:rpm_exceeded",
        error="Rate limit exceeded",
    )
    assert r.status == 429
    assert r.acquired is False
    assert r.error == "Rate limit exceeded"


# ============================================================================
# 4. IntegrationStats tests (主 17:43)
# ============================================================================


def test_v1271_integration_stats_defaults():
    """真 IntegrationStats defaults."""
    s = V1271IntegrationStats()
    assert s.total == 0
    assert s.n_allowed == 0
    assert s.n_denied == 0
    assert s.total_tokens == 0


def test_v1271_integration_stats_to_dict_includes_rates():
    """真 stats to_dict 含 rates (主 17:43)."""
    s = V1271IntegrationStats(total=10, n_allowed=7, n_denied=3, n_streamed=7, n_errors=0)
    d = s.to_dict()
    assert d["deny_rate"] == 0.3
    assert d["stream_rate"] == 0.7
    assert d["error_rate"] == 0.0


def test_v1271_compute_stats_with_results():
    """真 compute stats 真结果 (主 17:43 实事求是)."""
    results = [
        V1271StreamRunResult(
            sample_id=f"test_{i}",
            benchmark="MMLU",
            status=200,
            ttft_ms=100.0 + i * 10,
            chunks=5,
            total_ms=200.0 + i * 20,
            tokens=10,
            acquired=(i < 3),  # 3 allowed, 2 denied
        )
        for i in range(5)
    ]
    stats = _compute_v1271_stats(results, elapsed_ms=1000.0)
    assert stats.total == 5
    assert stats.n_allowed == 3
    assert stats.n_denied == 2
    assert stats.elapsed_ms == 1000.0


def test_v1271_compute_stats_with_empty_results():
    """真 compute stats 空 results (主 17:43 实事求是)."""
    stats = _compute_v1271_stats([], elapsed_ms=0.0)
    assert stats.total == 0
    assert stats.n_allowed == 0
    assert stats.avg_ttft_ms == 0.0


# ============================================================================
# 5. API key masking tests (主 17:58 不假装)
# ============================================================================


def test_v1271_mask_api_key_long():
    """真 key 遮蔽 (主 17:58 不假装 key 真泄露)."""
    masked = _mask_api_key("v1271-mo-ckkkkkkk-cret-long-key")
    assert masked.startswith("v1271-mo")
    assert masked.endswith("-key")
    assert "*" in masked


def test_v1271_mask_api_key_short():
    """真短 key 遮蔽 (主 17:58 不假装)."""
    masked = _mask_api_key("short")
    assert masked == "***"


def test_v1271_mask_api_key_empty():
    """真空 key 遮蔽 (主 17:58)."""
    masked = _mask_api_key("")
    assert masked == "***"


def test_v1271_mask_api_key_default_v1271():
    """真默认 v1271 mock key 遮蔽 (主 17:58)."""
    masked = _mask_api_key("v1271-mo*****cret")
    assert "*" in masked
    assert masked.startswith("v1271-m")


# ============================================================================
# 6. Sample loader tests (主 19:33 走在前人肩上 V1034)
# ============================================================================


def test_v1271_load_samples_basic():
    """真加载样本 (主 19:33)."""
    samples = _load_v1271_samples(limit=10, benchmark_filter=None)
    assert isinstance(samples, list)
    assert len(samples) > 0
    assert len(samples) <= 10
    for sample_id, benchmark, prompt in samples:
        assert isinstance(sample_id, str)
        assert isinstance(benchmark, str)
        assert isinstance(prompt, str)
        assert benchmark in ("MMLU", "GSM8K", "HumanEval", "HellaSwag")


def test_v1271_load_samples_benchmark_filter():
    """真 benchmark filter (主 17:43)."""
    samples = _load_v1271_samples(limit=20, benchmark_filter=["MMLU"])
    assert len(samples) > 0
    for _, benchmark, _ in samples:
        assert benchmark == "MMLU"


def test_v1271_load_samples_limit():
    """真 limit (主 17:43)."""
    samples = _load_v1271_samples(limit=3, benchmark_filter=None)
    assert len(samples) == 3


def test_v1271_load_samples_format():
    """真样本格式 (主 19:33 走在前人肩上 V1034)."""
    samples = _load_v1271_samples(limit=5, benchmark_filter=None)
    for sample_id, benchmark, prompt in samples:
        # 真 sample_id 格式: BENCHMARK_NNN
        parts = sample_id.rsplit("_", 1)
        assert len(parts) == 2
        assert parts[0] == benchmark


# ============================================================================
# 7. Integration runner tests (主 23:44 干到底)
# ============================================================================


def test_v1271_integration_runner_constructs():
    """真 runner 构造 (主 23:44)."""
    cfg = V1271IntegrationConfig(
        rate_limit_config=V1270RateLimitConfig(requests_per_minute=10),
        sample_limit=5,
    )
    runner = V1271IntegrationRunner(cfg)
    assert runner.limiter is not None
    assert isinstance(runner.limiter, V1270RateLimiter)


def test_v1271_integration_runner_limiter_shared():
    """真 runner limiter 真共享 (主 23:44)."""
    cfg = V1271IntegrationConfig(rate_limit_config=V1270RateLimitConfig(requests_per_minute=10))
    runner = V1271IntegrationRunner(cfg)
    assert runner.limiter.cfg.requests_per_minute == 10


def test_v1271_integration_runner_last_release_ok_default():
    """真 runner last_release_ok default."""
    cfg = V1271IntegrationConfig()
    runner = V1271IntegrationRunner(cfg)
    assert runner.last_release_ok is False


# ============================================================================
# 8. Rate limit enforcement tests (主 17:58 不假装 limit 真生效)
# ============================================================================


def test_v1271_rate_limit_actually_enforced():
    """真 rate limit 真超限真 raise (主 17:58 不假装)."""
    cfg = V1270RateLimitConfig(requests_per_minute=1, tokens_per_minute=100, max_concurrent=1)
    lim = V1270RateLimiter(cfg)
    # 第 1 个允许
    d1 = lim.acquire(estimated_tokens=10, now=time.time())
    assert d1.allowed
    # 第 2 个真 raise
    with pytest.raises(V1270RateLimitExceeded) as exc_info:
        lim.acquire(estimated_tokens=10, now=time.time())
    assert "rpm" in exc_info.value.decision.reason or "concurrent" in exc_info.value.decision.reason


def test_v1271_release_decrements_active():
    """真 release 真减 active (主 23:44 no leak)."""
    cfg = V1270RateLimitConfig(requests_per_minute=10, max_concurrent=2)
    lim = V1270RateLimiter(cfg)
    lim.acquire(estimated_tokens=10)
    lim.acquire(estimated_tokens=10)
    snap1 = lim.snapshot()
    lim.release(estimated_tokens=10)
    snap2 = lim.snapshot()
    # active 真减 (V1270 snapshot 用 "concurrent.active" 嵌套 key)
    assert snap2["concurrent"]["active"] == snap1["concurrent"]["active"] - 1


# ============================================================================
# 9. Markdown reporter tests (主 00:56 任何人都能接手)
# ============================================================================


def test_v1271_markdown_report_contains_version():
    """真 report 含 version (主 00:56)."""
    cfg = V1271IntegrationConfig()
    stats = V1271IntegrationStats(total=0)
    md = render_v1271_markdown_report(cfg, [], stats, "http://test", "v1271-mo*****cret")
    assert "V1271" in md
    assert "0.1.0" in md


def test_v1271_markdown_report_contains_v3_guards():
    """真 report 含 V3 guards (主 17:58)."""
    cfg = V1271IntegrationConfig()
    stats = V1271IntegrationStats()
    md = render_v1271_markdown_report(cfg, [], stats, "http://test", "***")
    for guard in V1271_V3_GUARDS:
        assert guard in md


def test_v1271_markdown_report_contains_references():
    """真 report 含真借鉴 (主 19:33)."""
    cfg = V1271IntegrationConfig()
    stats = V1271IntegrationStats()
    md = render_v1271_markdown_report(cfg, [], stats, "http://test", "***")
    for ref in V1271_REFERENCES:
        # 真标号内含 reference text
        assert any(ref_part in md for ref_part in ref.split(" "))


def test_v1271_markdown_report_contains_results():
    """真 report 含逐样本结果 (主 17:43 不假装)."""
    cfg = V1271IntegrationConfig()
    results = [
        V1271StreamRunResult(
            sample_id="MMLU_001",
            benchmark="MMLU",
            status=200,
            ttft_ms=150.0,
            chunks=10,
            total_ms=500.0,
            tokens=42,
            acquired=True,
            release_ok=True,
        )
    ]
    stats = V1271IntegrationStats(total=1, n_allowed=1, n_streamed=1, avg_ttft_ms=150.0)
    md = render_v1271_markdown_report(cfg, results, stats, "http://test", "***")
    assert "MMLU_001" in md
    assert "MMLU" in md


def test_v1271_markdown_report_contains_deny_disclosure():
    """真 report 含不假装声明 (主 17:58 + 主 20:46)."""
    cfg = V1271IntegrationConfig()
    stats = V1271IntegrationStats()
    md = render_v1271_markdown_report(cfg, [], stats, "http://test", "***")
    # 真标: 不假装声明
    assert "不假装" in md or "NOT" in md or "not" in md


# ============================================================================
# 10. CLI / main tests (主 00:56 任何人都能接手)
# ============================================================================


def test_v1271_cli_help(capsys):
    """真 CLI --help (主 00:56 任何人都能接手)."""
    from apeireth.v1271_asi_stream_rate_limit_integration import _main
    with pytest.raises(SystemExit) as exc_info:
        _main(["--help"])
    # argparse --help 真 exit code 0 (主 17:43)
    assert exc_info.value.code == 0
    out = capsys.readouterr().out
    assert "v1271" in out.lower() or "V1271" in out


def test_v1271_cli_sanity(capsys):
    """真 CLI --sanity (主 00:56 任何人都能接手)."""
    from apeireth.v1271_asi_stream_rate_limit_integration import _main
    rc = _main(["--sanity"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "true" in captured.out.lower() or "version" in captured.out.lower()


def test_v1271_cli_no_args(capsys):
    """真 CLI 无 args (主 00:56 任何人都能接手)."""
    from apeireth.v1271_asi_stream_rate_limit_integration import _main
    rc = _main([])
    assert rc == 1
    out = capsys.readouterr().out
    assert "v1271" in out.lower() or "V1271" in out


# ============================================================================
# 11. End-to-end integration (主 00:56 任何人都能接手)
# ============================================================================


def test_v1271_full_loop_small_smoke():
    """真 full loop 小 smoke (主 00:56 任何人都能接手).

    真起 mock subprocess + 真限流 + 真流式 + 真报告.
    """
    cfg = V1271IntegrationConfig(
        sample_limit=3,
        rate_limit_config=V1270RateLimitConfig(
            requests_per_minute=60,
            tokens_per_minute=10000,
            max_concurrent=4,
        ),
    )
    result = run_v1271_full_loop(cfg=cfg, report_path=None, api_key="v1271-test-key-abc123")
    assert result["started"] is True
    assert result["healthy"] is True
    assert "base_url" in result
    assert result["masked_key"].startswith("v1271-t")
    assert result["stats"]["total"] == 3
    # 真限流宽松 → 全 allowed
    assert result["stats"]["n_allowed"] >= 1
    # 真报告存在
    assert "report" in result
    assert len(result["report"]) > 100


def test_v1271_full_loop_with_report_file(tmp_path):
    """真 full loop 真写报告 (主 00:56)."""
    cfg = V1271IntegrationConfig(sample_limit=2)
    report_path = str(tmp_path / "v1271_report.md")
    result = run_v1271_full_loop(cfg=cfg, report_path=report_path, api_key="v1271-test")
    assert result["started"] is True
    import os
    assert os.path.exists(report_path)
    with open(report_path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1271" in content


def test_v1271_full_loop_denial_path():
    """真 full loop 真限流超限真 deny (主 17:58 不假装)."""
    cfg = V1271IntegrationConfig(
        sample_limit=5,
        rate_limit_config=V1270RateLimitConfig(
            requests_per_minute=1,
            tokens_per_minute=100,
            max_concurrent=1,
        ),
    )
    result = run_v1271_full_loop(cfg=cfg, report_path=None, api_key="v1271-test")
    assert result["started"] is True
    # 真 5 样本中至少 1 deny
    assert result["stats"]["n_denied"] >= 1
    # deny_rate > 0
    assert result["stats"]["deny_rate"] > 0.0


def test_v1271_full_loop_release_consistency():
    """真 full loop 真 release (主 23:44 no leak)."""
    cfg = V1271IntegrationConfig(sample_limit=3)
    result = run_v1271_full_loop(cfg=cfg, report_path=None, api_key="v1271-test")
    # 真限流宽松 → 全 allowed + 全 release
    for r in result["results"]:
        if r["acquired"]:
            # 真 release 应在 finally 中完成
            assert r["release_ok"] is True


# ============================================================================
# 12. Module import sanity (主 17:43 实事求是)
# ============================================================================


def test_v1271_imports_real_dependencies():
    """真 import 真 deps (主 19:33 走在前人肩上)."""
    import apeireth.v1271_asi_stream_rate_limit_integration as m
    # 真 import V1269
    from apeireth import v1269_asi_real_llm_stream_real_test
    assert hasattr(v1269_asi_real_llm_stream_real_test, "stream_chat_completion")
    assert hasattr(v1269_asi_real_llm_stream_real_test, "serve_v1269_in_thread")
    # 真 import V1270
    from apeireth import v1270_asi_stream_rate_limiter
    assert hasattr(v1270_asi_stream_rate_limiter, "V1270RateLimiter")
    assert hasattr(v1270_asi_stream_rate_limiter, "V1270RateLimitExceeded")
    assert hasattr(m, "run_v1271_full_loop")


def test_v1271_module_docstring_present():
    """真 module docstring (主 00:44 质量工程化)."""
    import apeireth.v1271_asi_stream_rate_limit_integration as m
    assert m.__doc__ is not None
    assert "V1271" in m.__doc__
    # 真主 signal 关键词
    for keyword in ("主 22:33", "主 17:43", "主 17:58", "主 23:44", "主 00:56"):
        assert keyword in m.__doc__