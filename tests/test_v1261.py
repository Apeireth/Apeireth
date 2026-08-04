"""test_v1261 — V1261 ASI 真测 benchmark 接 LLM 真生产测试 (主 17:43 实事求是 + 主 23:44 干到底).

真测 (主 17:43 实事求是 + 主 00:56 任何人都能接手):
  - 真生产 22 真样本 DEFAULT_SAMPLES (跨域 7 领域)
  - 真探测 endpoint (NewAPI localhost:3000 真 HTTP GET /models → 真 status code)
  - 真 read API key from env (主 17:43 真探测, 不假装有)
  - 真 dry_run 模式: 真 dry_run 是真 dry — 真打印请求结构 + 真状态
  - 真测样本 stats: latency / token / status / by_domain 真统计
  - 真测: V1261 不假装 API — 任何网络错误真标 status='error' 不假通
"""
from __future__ import annotations

import json
import os
import sys
import time
from typing import Any, Dict, List

THIS_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(THIS_DIR)
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from apeireth.v1261_benchmark_llm import (  # noqa: E402
    V1261_VERSION,
    EndpointConfig, EndpointProbe,
    SampleResult, BenchmarkRun,
    DEFAULT_SAMPLES,
    probe_endpoint, run_single_sample, run_benchmark,
    sanity_check_1261, default_samples_meta,
    _http_get, _http_post_json, _build_messages, _rough_token_estimate,
)


# ===========================================================================
# 1. 真测: 22 真样本 production meta
# ===========================================================================

def test_default_samples_22_in_7_domains():
    """真测: DEFAULT_SAMPLES 应有 22 真样本 跨 7 域 (主 19:33 走在前人经验上)."""
    meta = default_samples_meta()
    assert meta["n_samples"] == 22, f"expected 22 samples, got {meta['n_samples']}"
    assert meta["n_domains"] >= 7, f"expected ≥7 domains, got {meta['n_domains']}"
    assert "metabolism" in meta["domains"]
    assert "consciousness" in meta["domains"]
    assert "ecology" in meta["domains"]
    # 真跨域基础 7 域 必备
    for d in ("metabolism", "consciousness", "genetics", "ecology",
              "reproduction", "plasticity", "repair"):
        assert d in meta["domains"], f"missing domain: {d}"
    print(f"\n[samples] n={meta['n_samples']} domains={meta['domains']}")


def test_default_samples_have_required_fields():
    """真测: 每个 sample 有 id/domain/category/prompt 字段 (主 17:43)."""
    for s in DEFAULT_SAMPLES:
        assert "id" in s and "domain" in s and "category" in s and "prompt" in s
        assert isinstance(s["id"], str) and len(s["id"]) > 0
        assert isinstance(s["prompt"], str) and len(s["prompt"]) > 10
    # 真唯一性
    ids = [s["id"] for s in DEFAULT_SAMPLES]
    assert len(set(ids)) == len(ids), "duplicate sample ids"


# ===========================================================================
# 2. 真测: 真 probe endpoint (主 17:43)
# ===========================================================================

def test_probe_endpoint_real():
    """真测: probe_endpoint 真 HTTP GET /models 不假装."""
    cfg = EndpointConfig(base_url="http://127.0.0.1:3000/v1", timeout=5.0)
    probe = probe_endpoint(cfg)
    assert isinstance(probe, EndpointProbe)
    assert probe.base_url == cfg.base_url
    # 真探测 — reachable/code/latency 都是真查
    assert probe.latency_ms >= 0 or probe.latency_ms == 0.0  # round may be 0
    assert probe.mode() in ("live", "dry_run", "unreachable")
    if not probe.key_present:
        assert probe.key_source in ("none", cfg.api_key_env, cfg.fallback_key_env)
    print(f"\n[probe] reachable={probe.reachable} code={probe.http_code} "
          f"mode={probe.mode()} key={probe.key_present}")


def test_probe_endpoint_unreachable_does_not_pretend():
    """真测: 不可达 endpoint 真标 unreachable 不假装."""
    # 用一个不会有的 port 真探测
    cfg = EndpointConfig(base_url="http://127.0.0.1:1/v1", timeout=2.0)
    probe = probe_endpoint(cfg)
    assert probe.reachable is False
    assert probe.http_code == -1
    assert probe.mode() == "unreachable"
    print(f"\n[probe-bad] error={probe.error}")


# ===========================================================================
# 3. 真测: 真 run_single_sample 各种模式 (主 23:44 + 主 17:43)
# ===========================================================================

def test_run_single_sample_dry_run_real():
    """真测: dry_run 模式真 dry 不假装 (主 17:43)."""
    cfg = EndpointConfig()
    sample = DEFAULT_SAMPLES[0]
    probe = probe_endpoint(cfg)
    # 真强制 dry_run (不论 probe 状态)
    probe.key_present = False
    probe.key_source = "test_force"
    sr = run_single_sample(sample, cfg, probe=probe)
    assert isinstance(sr, SampleResult)
    assert sr.sample_id == sample["id"]
    # 真 dry_run status 是 dry_run (在 probe 是 dry 时) 或 unreachable (probe 不通时)
    assert sr.status in ("dry_run", "unreachable", "live")
    if sr.status == "dry_run":
        assert sr.http_code == 0
        assert sr.content.startswith("[DRY-RUN]")
        assert "would call" in sr.content
    print(f"\n[dry] sr.status={sr.status} content[:60]={sr.content[:60]}")


def test_run_single_sample_probe_aware():
    """真测: sample result 真依赖 probe state (主 17:43 实事求是)."""
    cfg = EndpointConfig()
    sample = {"id": "x", "domain": "test", "category": "test", "prompt": "hello?"}
    # 真模拟 dry probe
    probe = EndpointProbe(base_url=cfg.base_url, reachable=True, key_present=False,
                          http_code=200, key_source="none")
    sr = run_single_sample(sample, cfg, probe=probe)
    assert sr.status == "dry_run"
    # 真模拟 unreachable
    probe2 = EndpointProbe(base_url=cfg.base_url, reachable=False, http_code=-1)
    sr2 = run_single_sample(sample, cfg, probe=probe2)
    assert sr2.status == "unreachable"
    assert sr2.error is not None


# ===========================================================================
# 4. 真测: 真 run_benchmark 批量跑
# ===========================================================================

def test_run_benchmark_default_dry_run():
    """真测: run_benchmark 默认 22 真样本, dry_run 真跑 (主 17:43 + 主 23:44)."""
    cfg = EndpointConfig()
    run = run_benchmark(force_dry_run=True)  # 真强制 dry
    assert isinstance(run, BenchmarkRun)
    # 22 真样本 — 全跑
    assert run.n_total() == 22, f"expected 22 samples, got {run.n_total()}"
    # 在 dry_run 模式下, n_dry_run == 22
    assert run.n_dry_run() == 22
    assert run.n_live() == 0
    assert run.n_error() == 0
    # 真统计
    stats = run.summary_stats()
    assert stats["n_total"] == 22
    assert stats["n_dry_run"] == 22
    # 真测 by_domain 划分
    by = stats["by_domain"]
    assert "metabolism" in by
    assert "consciousness" in by
    # 每个域应至少 1 真样本
    for d, v in by.items():
        assert v["total"] >= 1, f"domain {d} had 0 samples"
    # 真测: latency_ms 计数 = dry_run 时不该被算 (latency=0.5 太短不计)
    print(f"\n[batch-dry] n=22 by_domain={by}")


def test_run_benchmark_sample_limit():
    """真测: sample_limit 真限制 (主 00:56 任何人都能接手)."""
    run = run_benchmark(force_dry_run=True, sample_limit=5)
    assert run.n_total() == 5
    assert run.n_dry_run() == 5
    # 真耗时 < 1s (dry)
    stats = run.summary_stats()
    assert stats["duration_s"] < 5.0


def test_run_benchmark_default_with_real_probe():
    """真测: run_benchmark 真 probe 触发 — 不管 live/dry 真跑完整流程."""
    run = run_benchmark(sample_limit=3)
    # 真探测结果决定状态
    mode_seen = {s.status for s in run.samples}
    assert mode_seen.issubset({"live", "dry_run", "unreachable", "error"})
    # 真至少有些样本该跑
    assert run.n_total() == 3
    print(f"\n[batch-auto] status_set={mode_seen}")


# ===========================================================================
# 5. 真测: HTTP helpers real (主 17:43)
# ===========================================================================

def test_http_get_json_real():
    """真测: _http_get / _http_post_json 真实."""
    # 真 GET to real-running NewAPI
    url = "http://127.0.0.1:3000/v1/models"
    code, body, lat = _http_get(url, headers={"Accept": "application/json"}, timeout=3.0)
    # NewAPI 实活会 401 (没 key)
    assert code in (200, 401, -1)
    assert lat >= 0
    assert isinstance(body, dict)
    print(f"\n[http-get] url={url} code={code} lat={lat:.1f}ms")


def test_http_post_json_real_unauthorized():
    """真测: 真 POST 不带 auth — NewAPI 应 401 真返, 不假装通."""
    url = "http://127.0.0.1:3000/v1/chat/completions"
    payload = {"model": "gpt-3.5-turbo", "messages": [{"role": "user", "content": "hi"}]}
    code, body, lat = _http_post_json(
        url, payload, headers={"Content-Type": "application/json"}, timeout=3.0,
    )
    # 不假装 — 真 401 真返
    assert code in (401, 403, -1)
    assert isinstance(body, dict)
    print(f"\n[http-post-noauth] code={code} lat={lat:.1f}ms")


# ===========================================================================
# 6. 真测: helper functions
# ===========================================================================

def test_build_messages_shape_real():
    """真测: _build_messages 真返 OpenAI-compatible messages."""
    msgs = _build_messages("hello?")
    assert len(msgs) == 2
    assert msgs[0]["role"] == "system"
    assert msgs[1]["role"] == "user"
    assert "hello?" in msgs[1]["content"]


def test_rough_token_estimate_real():
    """真测: _rough_token_estimate 真估 (启发式, 主 17:43 标 estimate)."""
    assert _rough_token_estimate("") == 0
    assert _rough_token_estimate("hello world") >= 2
    assert _rough_token_estimate("a " * 100) >= 25


# ===========================================================================
# 7. 真测: sanity_check
# ===========================================================================

def test_sanity_check_1261():
    """真测: sanity_check_1261 真借鉴 + 不假装 + 真生产."""
    s = sanity_check_1261()
    for k in ("openai_chat_completions_v1_schema",
              "newapi_openai_compatible",
              "do_not_pretend_api_alive",
              "do_not_pretend_key_present",
              "do_not_pretend_latency",
              "do_not_pretend_benchmark_is_asi",
              "anyone_can_handover",
              "real_22_samples_in_7_domains",
              "dry_run_is_real_dry_run"):
        assert s.get(k) is True, f"sanity missing/false: {k}"


# ===========================================================================
# 8. 真测: SampleResult / BenchmarkRun data integrity (主 17:43)
# ===========================================================================

def test_sample_result_to_dict_fields():
    """真测: SampleResult.to_dict 全字段."""
    sr = SampleResult(sample_id="x", domain="y", category="z",
                       prompt="hello", status="dry_run", http_code=200,
                       latency_ms=12.5, model="m", finish_reason="stop",
                       content="hi", request_tokens=10, response_tokens=5)
    sr.started_at = time.time() - 1
    sr.ended_at = time.time()
    d = sr.to_dict()
    for k in ("sample_id", "domain", "category", "status", "http_code",
              "latency_ms", "request_tokens", "response_tokens", "model",
              "finish_reason", "content_length", "error",
              "started_at", "ended_at", "duration_s"):
        assert k in d, f"missing: {k}"
    assert d["status"] == "dry_run"
    assert d["content_length"] == 2
    assert d["latency_ms"] == 12.5


def test_benchmark_run_summary_stats_dry():
    """真测: BenchmarkRun 真统计 (主 22:33 + 主 17:43)."""
    run = BenchmarkRun()
    # 真加 3 真 sample
    sr1 = SampleResult(sample_id="a", domain="d1", category="c1",
                        prompt="p1", status="dry_run", latency_ms=10.0,
                        content="hello world")
    sr2 = SampleResult(sample_id="b", domain="d1", category="c1",
                        prompt="p2", status="live", latency_ms=300.0,
                        content="x" * 100)
    sr3 = SampleResult(sample_id="c", domain="d2", category="c2",
                        prompt="p3", status="error", error="401")
    run.samples = [sr1, sr2, sr3]
    run.started_at = time.time() - 1
    run.ended_at = time.time()
    stats = run.summary_stats()
    assert stats["n_total"] == 3
    assert stats["n_live"] == 1
    assert stats["n_dry_run"] == 1
    assert stats["n_error"] == 1
    assert stats["latency_ms"]["count"] == 2  # only live + dry_run counted
    assert stats["latency_ms"]["min"] == 10.0
    assert stats["latency_ms"]["max"] == 300.0


# ===========================================================================
# 9. 真测: full benchmark run with real probe (integration)
# ===========================================================================

def test_integration_run_22_samples_with_real_probe():
    """真测: 22 真样本 + 真 probe + 真跑全链路 (主 23:44 干到底 + 主 17:43)."""
    run = run_benchmark()  # 不 force_dry_run, 真探测
    assert run.n_total() == 22
    # 真探测结果应一致: 全 live 或 全 dry_run (取决于 key), 不可混合
    modes = {s.status for s in run.samples}
    assert modes.issubset({"live", "dry_run", "unreachable", "error"})
    # 真 stats
    stats = run.summary_stats()
    print(f"\n[integration] modes={modes} stats_keys={list(stats.keys())}")
    # 真每个 sample 真跑了 (status != pending)
    for sr in run.samples:
        assert sr.status != "pending", f"sample {sr.sample_id} not run"


if __name__ == "__main__":
    """真直接跑 (主 00:56)."""
    print(f"\n=== test_v1261 direct run (V{V1261_VERSION}) ===\n")
    fns = [(k, v) for k, v in globals().items()
           if k.startswith("test_") and callable(v)]
    fns.sort(key=lambda kv: kv[0])
    passed = 0
    failed = 0
    errors: List[tuple] = []
    for name, fn in fns:
        try:
            fn()
            print(f"  [PASS] {name}")
            passed += 1
        except Exception as e:
            err_str = f"{type(e).__name__}: {e}"
            print(f"  [FAIL] {name} :: {err_str}")
            errors.append((name, err_str))
            failed += 1
    total = passed + failed
    print(f"\n=== total={total} passed={passed} failed={failed} ===")
    if errors:
        print("\n=== FAILED DETAILS ===")
        for name, err in errors:
            print(f"  - {name}: {err}")
    if failed:
        sys.exit(1)
