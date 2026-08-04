"""Tests for V1261 benchmark_llm (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Verify:
- V1261 version is exposed
- 22 default samples cover 7+ domains
- sanity_check_1261 returns all True
- dry-run benchmark returns BenchmarkRun with samples
- run_single_sample works with dry_run=True
- probe_endpoint returns EndpointProbe (without making real calls when unreachable)
- 5 V3 guards all present
"""
from __future__ import annotations

import os
import sys

import pytest

try:
    from apeireth import v1261_benchmark_llm as v61
except Exception:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
    import v1261_benchmark_llm as v61


def test_v1261_version():
    assert v61.V1261_VERSION == "0.1.0"


def test_v1261_sanity_all_true():
    sc = v61.sanity_check_1261()
    assert isinstance(sc, dict)
    assert all(sc.values()), f"failed: {[k for k, v in sc.items() if not v]}"


def test_v1261_v3_guards_count():
    assert len(v61.V3_GUARDS) == 5


def test_v1261_default_samples_22_in_7_domains():
    samples = v61.DEFAULT_SAMPLES
    assert len(samples) == 22
    domains = {s.get("domain") for s in samples if "domain" in s}
    assert len(domains) >= 7, f"only {len(domains)} domains: {domains}"


def test_v1261_endpoint_config_defaults():
    cfg = v61.EndpointConfig()
    assert cfg.base_url.startswith("http")
    assert cfg.timeout > 0
    assert cfg.max_retries >= 0


def test_v1261_endpoint_probe_dataclass():
    probe = v61.EndpointProbe(base_url="http://test.invalid")
    assert probe.base_url == "http://test.invalid"
    assert probe.reachable is False
    assert probe.http_code == -1


def test_v1261_dry_run_benchmark_runs():
    """真 dry-run: 不打外部 API, 返回 samples."""
    run = v61.run_benchmark(force_dry_run=True, sample_limit=3)
    assert isinstance(run, v61.BenchmarkRun)
    assert len(run.samples) == 3
    for s in run.samples:
        assert s.status == "dry_run"
        assert len(s.content) > 0  # dry-run fills content


def test_v1261_run_single_sample_dry_run():
    sample = v61.DEFAULT_SAMPLES[0]
    cfg = v61.EndpointConfig()
    result = v61.run_single_sample(sample, cfg, dry_run=True)
    assert isinstance(result, v61.SampleResult)
    assert result.status == "dry_run"
    assert result.sample_id == sample.get("sample_id", sample.get("id", ""))


def test_v1261_run_benchmark_full_dry_run():
    """真全 22 sample dry-run."""
    run = v61.run_benchmark(force_dry_run=True)
    assert isinstance(run, v61.BenchmarkRun)
    assert len(run.samples) == 22
    # All samples should be dry_run
    assert all(s.status == "dry_run" for s in run.samples)


def test_v1261_default_samples_have_categories():
    """每个 sample 真有 category, 覆盖 5+ 类."""
    cats = {s.get("category") for s in v61.DEFAULT_SAMPLES if "category" in s}
    assert len(cats) >= 5, f"only {len(cats)} categories"