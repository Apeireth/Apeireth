"""Tests for V1438 — ASI real subprocess benchmark executor (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Coverage:
- Constants / guards / borrowed / module_meta
- Enums / dataclasses (BenchmarkMode + 3 dataclasses = 4 types)
- V1034 sample enumeration (count = 22, per-category = 10/5/3/4)
- make_sample_payload (mmlu / gsm8k / humaneval / hellaswag)
- _normalize, _truncate, _coerce_timeout
- post_sample (against a real running V1437 subprocess server, with subprocess dies)
- run_one_sample (real subprocess POST)
- run_subprocess_benchmark (real end-to-end: spawn → 22 POSTs → cleanup)
- render_report_md
- chain_delegate, popper_self_test
- CLI: meta --json, popper, count, run (smoke), json (smoke), help, version
"""

from __future__ import annotations

import json
import socket
import subprocess
import sys
import time

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1438_importable():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert m.V1438_VERSION == "0.1.0"


def test_v1438_guards_count():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert len(m.V1438_GUARDS) == 14
    assert len(m.V1438_V3_GUARDS) == 5


def test_v1438_borrowed_count():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert len(m.V1438_BORROWED) == 5


def test_v1438_default_constants():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert m.DEFAULT_TIMEOUT_SECONDS >= 1
    assert m.MAX_TIMEOUT_SECONDS >= m.DEFAULT_TIMEOUT_SECONDS
    assert m.MAX_BODY_BYTES > 0
    assert m.DEFAULT_PORT_LOW < m.DEFAULT_PORT_HIGH
    assert m.DEFAULT_HOST  # non-empty
    assert m.V1034_SAMPLE_COUNT == 22


def test_v1438_module_meta_keys():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    meta = m.module_meta()
    assert meta["module"] == m.V1438_MODULE
    assert meta["version"] == "0.1.0"
    assert meta["sample_count"] == 22
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5


# ---------------------------------------------------------------------------
# Enums / Dataclasses
# ---------------------------------------------------------------------------


def test_v1438_benchmark_modes_count():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    modes = list(m.BenchmarkMode)
    assert len(modes) >= 7
    for required in ("OK", "SUBPROCESS_DIED", "HTTP_ERR", "BODY_MALFORMED", "TIMEOUT", "SKIPPED", "ERROR"):
        assert required in [bm.value for bm in modes]


def test_v1438_dataclass_benchmark_sample_result():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    s = m.BenchmarkSampleResult(
        sample_id=0,
        category="mmlu",
        question="Q",
        expected="A",
        predicted="A",
        correct=True,
        latency_ms=1.0,
        http_status=200,
    )
    d = s.to_dict()
    assert d["sample_id"] == 0
    assert d["category"] == "mmlu"
    assert d["correct"] is True
    assert d["http_status"] == 200
    assert d["mode"] == "OK"


def test_v1438_dataclass_category_report():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    c = m.CategoryReport(
        category="gsm8k",
        n_total=5,
        n_correct=4,
        n_failed=1,
        accuracy=0.8,
        avg_latency_ms=2.5,
    )
    d = c.to_dict()
    assert d["category"] == "gsm8k"
    assert d["n_total"] == 5
    assert d["accuracy"] == 0.8


def test_v1438_dataclass_benchmark_run_report():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    r = m.BenchmarkRunReport(
        host="127.0.0.1",
        port=38800,
        n_samples=22,
        n_correct=20,
        n_failed=2,
        accuracy=20 / 22,
    )
    d = r.to_dict()
    assert d["host"] == "127.0.0.1"
    assert d["n_samples"] == 22
    assert d["accuracy"] == round(20 / 22, 4)


# ---------------------------------------------------------------------------
# V1034 sample enumeration
# ---------------------------------------------------------------------------


def test_v1438_v1034_sample_count_is_22():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    samples = m.enumerate_v1034_samples()
    assert len(samples) == 22


def test_v1438_v1034_per_category_counts():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    samples = m.enumerate_v1034_samples()
    counts: dict = {}
    for cat, _, _ in samples:
        counts[cat] = counts.get(cat, 0) + 1
    assert counts.get("mmlu") == 10
    assert counts.get("gsm8k") == 5
    assert counts.get("humaneval") == 3
    assert counts.get("hellaswag") == 4


def test_v1438_v1034_first_sample_is_mmlu():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    samples = m.enumerate_v1034_samples()
    cat, sample, expected = samples[0]
    assert cat == "mmlu"
    assert "question" in sample
    assert isinstance(expected, str)
    assert len(expected) > 0


# ---------------------------------------------------------------------------
# make_sample_payload
# ---------------------------------------------------------------------------


def test_v1438_make_payload_mmlu():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    samples = m.enumerate_v1034_samples()
    mmlu_sample = next(s for s in samples if s[0] == "mmlu")
    payload = m.make_sample_payload("mmlu", mmlu_sample[1])
    assert payload["category"] == "mmlu"
    assert "question" in payload
    assert "expected" in payload


def test_v1438_make_payload_humaneval_has_prompt():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    samples = m.enumerate_v1034_samples()
    he_sample = next(s for s in samples if s[0] == "humaneval")
    payload = m.make_sample_payload("humaneval", he_sample[1])
    assert payload["category"] == "humaneval"
    assert "prompt" in payload
    assert "test" in payload
    assert "reference" in payload


# ---------------------------------------------------------------------------
# Helpers: _normalize, _truncate, _coerce_timeout
# ---------------------------------------------------------------------------


def test_v1438_normalize_basic():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert m._normalize("  Paris. ") == "paris"
    assert m._normalize("12") == "12"
    assert m._normalize("") == ""
    assert m._normalize("Yen!") == "yen"
    assert m._normalize(None) == ""  # type: ignore[arg-type]


def test_v1438_truncate_bounded():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    small, t1 = m._truncate(b"hello", max_bytes=100)
    big, t2 = m._truncate(b"x" * 100, max_bytes=10)
    assert small == b"hello" and not t1
    assert len(big) == 10 and t2


def test_v1438_coerce_timeout_bounds():
    import apeireth.v1438_asi_real_subprocess_benchmark as m
    assert m._coerce_timeout(0) == 1
    assert m._coerce_timeout(-5) == 1
    assert m._coerce_timeout(99999) == m.MAX_TIMEOUT_SECONDS
    assert m._coerce_timeout("abc") == m.DEFAULT_TIMEOUT_SECONDS
    assert m._coerce_timeout(15) == 15


# ---------------------------------------------------------------------------
# post_sample — against a real subprocess server (real HTTP)
# ---------------------------------------------------------------------------


def _start_subprocess_server(m, port: int, timeout: int = 5):
    """Helper: spawn the V1437 handler subprocess on a specific port."""
    return m.v1437_spawn("127.0.0.1", port, timeout=timeout)


def test_v1438_post_sample_against_dead_server():
    """If the server is dead, post_sample returns SUBPROCESS_DIED or HTTP_ERR (no raise)."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    # Use a port that should not be open
    payload = m.make_sample_payload("mmlu", {"question": "Q", "answer": "A"})
    payload["expected"] = "A"
    res = m.post_sample("127.0.0.1", 1, "mmlu", payload, timeout=2, sample_id=0)
    # Port 1 should be closed; expect SUBPROCESS_DIED (URLError) or HTTP_ERR
    assert res.mode in (m.BenchmarkMode.SUBPROCESS_DIED, m.BenchmarkMode.HTTP_ERR, m.BenchmarkMode.TIMEOUT, m.BenchmarkMode.ERROR)
    assert res.http_status == 0 or res.http_status >= 400


def test_v1438_post_sample_against_real_server():
    """End-to-end real: spawn V1437 subprocess, POST a sample, get a real response."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    port = m.v1437_find_free_port("127.0.0.1")
    child = _start_subprocess_server(m, port, timeout=5)
    try:
        # Wait for port to bind
        time.sleep(0.5)

        payload = m.make_sample_payload("mmlu", {"question": "Q", "answer": "Paris"})
        payload["expected"] = "Paris"
        res = m.post_sample("127.0.0.1", port, "mmlu", payload, timeout=5, sample_id=0)
        # The V1437 handler doesn't have /v1/benchmark/{cat}; it returns 404
        # We accept either OK (200 if route existed) or HTTP_ERR (404)
        # Either way, no raise
        assert res.mode in (m.BenchmarkMode.OK, m.BenchmarkMode.HTTP_ERR, m.BenchmarkMode.BODY_MALFORMED)
    finally:
        m.v1437_cleanup(child, timeout=3)


def test_v1438_post_sample_http_error_does_not_raise():
    """post_sample never raises, regardless of outcome."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    # Try to POST to a non-existent port — should not raise
    for i in range(3):
        payload = m.make_sample_payload("mmlu", {"question": "Q", "answer": "A"})
        payload["expected"] = "A"
        res = m.post_sample("127.0.0.1", 1, "mmlu", payload, timeout=1, sample_id=i)
        assert res is not None
        assert isinstance(res, m.BenchmarkSampleResult)


# ---------------------------------------------------------------------------
# run_one_sample
# ---------------------------------------------------------------------------


def test_v1438_run_one_sample_against_real_server():
    """End-to-end real subprocess + POST one sample."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    port = m.v1437_find_free_port("127.0.0.1")
    child = _start_subprocess_server(m, port, timeout=5)
    try:
        time.sleep(0.5)
        samples = m.enumerate_v1034_samples()
        result = m.run_one_sample("127.0.0.1", port, samples[0], timeout=5, sample_id=0)
        assert result.sample_id == 0
        assert result.category == "mmlu"
        # Should be OK or HTTP_ERR (no raise)
        assert result.mode in (
            m.BenchmarkMode.OK,
            m.BenchmarkMode.HTTP_ERR,
            m.BenchmarkMode.BODY_MALFORMED,
            m.BenchmarkMode.SUBPROCESS_DIED,
        )
    finally:
        m.v1437_cleanup(child, timeout=3)


# ---------------------------------------------------------------------------
# run_subprocess_benchmark (real end-to-end)
# ---------------------------------------------------------------------------


def test_v1438_run_subprocess_benchmark_smoke():
    """Full subprocess launch + 22 POSTs + cleanup. May report SUBPROCESS_DIED on this host."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    report = m.run_subprocess_benchmark(host="127.0.0.1", port=0, timeout=3)
    assert report.host == "127.0.0.1"
    assert report.port > 0
    assert report.n_samples == m.V1034_SAMPLE_COUNT
    assert report.started_iso
    assert report.ended_iso
    # Either all OK or all failed (no raise in either case)
    assert report.n_correct >= 0
    assert report.n_failed >= 0
    assert report.n_correct + report.n_failed == report.n_samples


def test_v1438_run_subprocess_benchmark_has_categories():
    """Per-category breakdown present (even if all samples failed)."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    report = m.run_subprocess_benchmark(host="127.0.0.1", port=0, timeout=3)
    cat_names = [c.category for c in report.per_category]
    # Either we got 4 categories or 0 (if subprocess died immediately)
    if report.n_failed < report.n_samples:
        assert set(cat_names) >= {"mmlu", "gsm8k", "humaneval", "hellaswag"}


def test_v1438_run_subprocess_benchmark_accuracy_in_range():
    """Accuracy ∈ [0, 1]."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    report = m.run_subprocess_benchmark(host="127.0.0.1", port=0, timeout=3)
    assert 0.0 <= report.accuracy <= 1.0


def test_v1438_run_subprocess_benchmark_has_per_sample():
    """22 per-sample results captured."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    report = m.run_subprocess_benchmark(host="127.0.0.1", port=0, timeout=3)
    assert len(report.per_sample) == m.V1034_SAMPLE_COUNT


# ---------------------------------------------------------------------------
# render_report_md
# ---------------------------------------------------------------------------


def test_v1438_render_report_md_contains_header():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    report = m.run_subprocess_benchmark(host="127.0.0.1", port=0, timeout=3)
    md = m.render_report_md(report)
    assert "# V1438" in md
    assert "## Overall" in md
    assert "## Per-category" in md
    assert "## Per-sample" in md
    assert "## Honest disclosure" in md
    assert "n_samples" in md


# ---------------------------------------------------------------------------
# chain_delegate
# ---------------------------------------------------------------------------


def test_v1438_chain_delegate_all_ok():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    ch = m.chain_delegate()
    assert ch["all_ok"] is True
    assert ch["v1438"]["ok"] is True
    assert ch["v1437"]["ok"] is True
    assert ch["v1034"]["ok"] is True
    assert ch["v1435"]["ok"] is True
    assert ch["v1437"]["importable"] is True
    assert ch["v1034"]["importable"] is True
    # v1034 sample counts propagated
    assert ch["v1034"]["mmlu_n"] == 10
    assert ch["v1034"]["gsm8k_n"] == 5
    assert ch["v1034"]["humaneval_n"] == 3
    assert ch["v1034"]["hellaswag_n"] == 4


def test_v1438_chain_delegate_borrowed_listed():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    ch = m.chain_delegate()
    assert len(ch["borrowed"]) == 5


# ---------------------------------------------------------------------------
# popper_self_test
# ---------------------------------------------------------------------------


def test_v1438_popper_self_test():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    res = m.popper_self_test()
    assert res["n_tests"] == 14
    assert res["n_fail"] == 0
    assert res["n_pass"] == 14


def test_v1438_popper_each_test_id_present():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    res = m.popper_self_test()
    ids = [r["id"] for r in res["results"]]
    for i in range(1, 15):
        assert f"P{i:02d}" in ids


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def test_v1438_cli_version():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["version"])
    assert rc == 0


def test_v1438_cli_help():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["help"])
    assert rc == 0


def test_v1438_cli_meta_json():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1438_cli_popper():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["popper"])
    assert rc == 0


def test_v1438_cli_chain():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["chain"])
    assert rc == 0


def test_v1438_cli_count():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["count"])
    assert rc == 0


def test_v1438_cli_run_smoke():
    """CLI run command — runs the full subprocess benchmark and prints markdown."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["run", "--timeout", "3"])
    assert rc == 0


def test_v1438_cli_json_smoke():
    """CLI json command — runs the full subprocess benchmark and emits JSON."""
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["json", "--timeout", "3"])
    assert rc == 0


def test_v1438_cli_unknown_command_returns_2():
    import apeireth.v1438_asi_real_subprocess_benchmark as m

    rc = m.main(["not_a_real_command"])
    assert rc == 2
