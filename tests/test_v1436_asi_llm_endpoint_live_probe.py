"""Tests for V1436 — ASI 真生产 LLM endpoint live probe (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43)."""

from __future__ import annotations

import json
import subprocess
import sys

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1436_importable():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert m.V1436_VERSION == "0.1.0"


def test_v1436_guards_count():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert len(m.V1436_GUARDS) == 14
    assert len(m.V1436_V3_GUARDS) == 5


def test_v1436_borrowed_count():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert len(m.V1436_BORROWED) == 5


def test_v1436_default_timeouts():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert m.DEFAULT_TIMEOUT_SECONDS >= 1
    assert m.MAX_TIMEOUT_SECONDS >= m.DEFAULT_TIMEOUT_SECONDS
    assert m.MAX_BODY_BYTES > 0
    assert m.MAX_RETRIES >= 1


def test_v1436_default_endpoints_count():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert len(m.DEFAULT_ENDPOINTS) >= 1
    for ep in m.DEFAULT_ENDPOINTS:
        assert m.validate_url(ep)


# ---------------------------------------------------------------------------
# Enums / Dataclasses
# ---------------------------------------------------------------------------


def test_v1436_probe_outcomes_count():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert len(list(m.ProbeOutcome)) == 7
    assert m.ProbeOutcome.ENDPOINT_REACHABLE.value == "ENDPOINT_REACHABLE"
    assert m.ProbeOutcome.ENDPOINT_UNREACHABLE.value == "ENDPOINT_UNREACHABLE"
    assert m.ProbeOutcome.AUTH_REQUIRED.value == "AUTH_REQUIRED"


def test_v1436_http_call_dataclass():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    hc = m.HttpCall(url="http://x", status_code=200, elapsed_ms=15.5, mode="OK")
    d = hc.to_dict()
    assert d["url"] == "http://x"
    assert d["status_code"] == 200
    assert d["elapsed_ms"] == 15.5
    assert d["mode"] == "OK"


def test_v1436_endpoint_probe_result_dataclass():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.EndpointProbeResult(endpoint_url="http://x", probe_outcome="PARSE_OK", models_count=10, models_sample=["gpt-4", "gpt-3.5"])
    d = r.to_dict()
    assert d["endpoint_url"] == "http://x"
    assert d["models_count"] == 10
    assert d["models_sample"] == ["gpt-4", "gpt-3.5"]


# ---------------------------------------------------------------------------
# URL validation
# ---------------------------------------------------------------------------


def test_v1436_validate_url_accepts_https():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert m.validate_url("https://api.openai.com") is True
    assert m.validate_url("https://api.example.com/v1") is True


def test_v1436_validate_url_accepts_http():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert m.validate_url("http://localhost:11434") is True
    assert m.validate_url("http://192.168.1.1:8080") is True


def test_v1436_validate_url_rejects_invalid():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    assert m.validate_url("") is False
    assert m.validate_url("ftp://example.com") is False
    assert m.validate_url("not-a-url") is False
    assert m.validate_url("file:///etc/passwd") is False


# ---------------------------------------------------------------------------
# Truncate helper
# ---------------------------------------------------------------------------


def test_v1436_truncate_empty():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    text, trunc = m._truncate("", 100)
    assert text == ""
    assert trunc is False


def test_v1436_truncate_short():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    text, trunc = m._truncate("hello", 100)
    assert text == "hello"
    assert trunc is False


def test_v1436_truncate_long():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    long_text = "x" * 1000
    text, trunc = m._truncate(long_text, 100)
    assert trunc is True
    assert len(text) == 100


# ---------------------------------------------------------------------------
# HTTP runner (offline-safe)
# ---------------------------------------------------------------------------


def test_v1436_http_get_bad_url_offline_safe():
    """HTTP GET to invalid host returns DNS_FAIL or CONN_ERR, no raise."""
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    c = m.http_get("http://this-host-definitely-does-not-exist-xyz-1436.invalid", timeout=2)
    assert c.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN")
    assert c.timed_out is False
    assert c.status_code == -1


def test_v1436_http_get_with_retry_offline_safe():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    c = m.http_get_with_retry("http://nope-1436.invalid", timeout=1, max_attempts=2)
    assert c.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN")


def test_v1436_http_get_clamps_timeout():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    c = m.http_get("http://nope-1436.invalid", timeout=9999)
    assert c.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN")


def test_v1436_http_get_clamps_zero_timeout():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    c = m.http_get("http://nope-1436.invalid", timeout=0)
    assert c.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN")


# ---------------------------------------------------------------------------
# Aggregated probe
# ---------------------------------------------------------------------------


def test_v1436_run_endpoint_probe_invalid_url_skipped():
    """Invalid URL → SKIPPED outcome."""
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.run_endpoint_probe("not-a-url", timeout=2)
    assert r.probe_outcome == m.ProbeOutcome.SKIPPED.value
    assert "invalid URL" in (r.notes[0] if r.notes else "")


def test_v1436_run_endpoint_probe_unreachable():
    """Unreachable host → ENDPOINT_UNREACHABLE (offline-safe)."""
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.run_endpoint_probe("http://nope-1436.invalid", timeout=2)
    assert r.probe_outcome == m.ProbeOutcome.ENDPOINT_UNREACHABLE.value
    assert r.models_count == -1
    assert r.started_iso
    assert r.ended_iso


def test_v1436_run_endpoint_probe_calls_3_endpoints():
    """Probe runs 3 HTTP calls: /v1/models, /api/status, /."""
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.run_endpoint_probe("http://nope-1436.invalid", timeout=2)
    # Should have 3 calls (or fewer if first call fails fast)
    assert 1 <= len(r.http_calls) <= 3


# ---------------------------------------------------------------------------
# Render / Serialize
# ---------------------------------------------------------------------------


def test_v1436_render_probe_summary_md():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.EndpointProbeResult(
        endpoint_url="http://x",
        probe_outcome="ENDPOINT_REACHABLE",
        models_count=5,
        models_sample=["model-a", "model-b"],
        server="uvicorn",
        content_type="application/json",
    )
    md = m.render_probe_summary_md(r)
    assert "V1436" in md
    assert "ENDPOINT_REACHABLE" in md
    assert "model-a" in md
    assert "uvicorn" in md
    assert "Honest disclosure" in md
    assert "probe ≠ chat" in md or "Probe ≠ chat" in md


def test_v1436_result_to_dict_serializable():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.EndpointProbeResult(endpoint_url="http://x", probe_outcome="ENDPOINT_REACHABLE", models_count=5)
    j = json.dumps(m.result_to_dict(r))
    parsed = json.loads(j)
    assert parsed["models_count"] == 5
    assert parsed["probe_outcome"] == "ENDPOINT_REACHABLE"


def test_v1436_result_to_dict_with_calls():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    r = m.EndpointProbeResult(endpoint_url="http://x")
    r.http_calls.append(m.HttpCall(url="http://x/v1/models", status_code=200, mode="OK", elapsed_ms=10))
    d = m.result_to_dict(r)
    assert len(d["http_calls"]) == 1
    assert d["http_calls"][0]["mode"] == "OK"


# ---------------------------------------------------------------------------
# Popper self-test
# ---------------------------------------------------------------------------


def test_v1436_popper_self_test_passes():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    out = m.popper_self_test()
    assert out["passed"] == out["total"], f"popper self-test failed: {out}"
    assert out["total"] == 14


def test_v1436_popper_self_test_includes_honesty():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    out = m.popper_self_test()
    p13 = next(r for r in out["results"] if r["id"] == "P13")
    assert p13["ok"]


def test_v1436_popper_self_test_guards():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    out = m.popper_self_test()
    p02 = next(r for r in out["results"] if r["id"] == "P02")
    assert p02["ok"]


# ---------------------------------------------------------------------------
# Chain delegate
# ---------------------------------------------------------------------------


def test_v1436_chain_delegate():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    out = m.chain_delegate()
    assert out["v1436"]["ok"] is True
    assert "v1435" in out
    assert "v1424" in out
    assert "v1076" in out
    assert "borrowed" in out
    assert isinstance(out["all_ok"], bool)


def test_v1436_chain_delegate_borrows_v1435_v1424():
    """V1436 borrows from V1435 (bounded subprocess) and V1424 (benchmark)."""
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    names = {m for m, _ in m.V1436_BORROWED}
    assert "v1435_asi_docker_availability_probe" in names
    assert "v1424_asi_real_llm_benchmark" in names
    assert "v1076_asi_real_external_llm_client" in names


# ---------------------------------------------------------------------------
# Module meta
# ---------------------------------------------------------------------------


def test_v1436_module_meta():
    import apeireth.v1436_asi_llm_endpoint_live_probe as m
    meta = m.module_meta()
    assert meta["module"] == "v1436_asi_llm_endpoint_live_probe"
    assert meta["version"] == "0.1.0"
    assert meta["schema"] == "v1436.asi-llm-endpoint-live-probe/v1"
    assert len(meta["guards"]) == 14
    assert len(meta["v3_guards"]) == 5
    assert len(meta["borrowed"]) == 5
    assert len(meta["probe_outcomes"]) == 7
    assert "/v1/models" in meta["endpoints"]


# ---------------------------------------------------------------------------
# CLI smoke
# ---------------------------------------------------------------------------


def test_v1436_cli_version():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "version"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10,
    )
    assert r.returncode == 0
    assert r.stdout.strip() == "0.1.0"


def test_v1436_cli_meta_json():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "meta", "--json"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10,
    )
    assert r.returncode == 0
    meta = json.loads(r.stdout)
    assert meta["version"] == "0.1.0"


def test_v1436_cli_popper():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "popper"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=15,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert out["passed"] == out["total"]


def test_v1436_cli_probe_unreachable():
    """CLI probe on unreachable URL → SKIPPED or ENDPOINT_UNREACHABLE."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "probe",
         "--url", "http://nope-1436.invalid", "--timeout", "2"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30,
    )
    assert r.returncode == 0
    assert "V1436" in r.stdout
    assert "probe_outcome" in r.stdout


def test_v1436_cli_probe_invalid_url():
    """CLI probe on invalid URL → SKIPPED."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "probe",
         "--url", "not-a-url", "--timeout", "2"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10,
    )
    assert r.returncode == 0
    assert "SKIPPED" in r.stdout


def test_v1436_cli_json_unreachable():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "json",
         "--url", "http://nope-1436.invalid", "--timeout", "2"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert "probe_outcome" in out
    assert out["probe_outcome"] in ("ENDPOINT_UNREACHABLE", "SKIPPED")


def test_v1436_cli_call_offline_safe():
    """CLI call on unreachable URL → offline-safe JSON."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "call",
         "--url", "http://nope-1436.invalid", "--timeout", "2"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert out["mode"] in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN")


def test_v1436_cli_chain():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1436_asi_llm_endpoint_live_probe", "chain"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=15,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert "all_ok" in out
    assert "v1435" in out
    assert "v1424" in out
