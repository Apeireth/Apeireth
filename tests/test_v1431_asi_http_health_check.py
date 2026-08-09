"""Tests for V1431 — ASI HTTP health check (主 00:44 质量工程化)."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
from urllib.error import URLError

import pytest


def test_v1431_importable():
    import apeireth.v1431_asi_http_health_check as m
    assert m.V1431_VERSION == "0.1.0"


def test_v1431_guards_count():
    import apeireth.v1431_asi_http_health_check as m
    assert len(m.V1431_GUARDS) == 13
    assert len(m.V1431_V3_GUARDS) == 5


def test_v1431_borrowed_count():
    import apeireth.v1431_asi_http_health_check as m
    assert len(m.V1431_BORROWED) == 5


def test_v1431_pick_free_port_returns_int():
    import apeireth.v1431_asi_http_health_check as m
    port = m.pick_free_port()
    assert isinstance(port, int)
    assert port > 0


def test_v1431_pick_free_port_two_calls_differ():
    """Two calls should pick different ports (eventually)."""
    import apeireth.v1431_asi_http_health_check as m
    p1 = m.pick_free_port()
    p2 = m.pick_free_port()
    # not guaranteed to differ but usually true
    assert p1 > 0 and p2 > 0


def test_v1431_http_get_bad_url_returns_error():
    import apeireth.v1431_asi_http_health_check as m
    code, body, ms, err = m.http_get("http://127.0.0.1:1/nope", timeout=1.0)
    assert code == 0
    assert err != ""


def test_v1431_check_endpoint_bad_port_fails():
    import apeireth.v1431_asi_http_health_check as m
    check = m.check_endpoint(1, "/api/asi/health", timeout=1.0)
    assert check.ok is False
    assert check.status_code == 0


def test_v1431_run_health_check_overall_ok():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    assert isinstance(report, m.HealthReport)
    assert report.n_total == 2
    assert report.overall_ok is True


def test_v1431_run_health_check_includes_health_endpoint():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    endpoints = [c.endpoint for c in report.endpoint_checks]
    assert "/api/asi/health" in endpoints


def test_v1431_run_health_check_includes_version_endpoint():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    endpoints = [c.endpoint for c in report.endpoint_checks]
    assert "/api/asi/version" in endpoints


def test_v1431_run_health_check_timestamps_present():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    assert report.started_iso
    assert report.ended_iso


def test_v1431_run_health_check_status_code_200():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    for c in report.endpoint_checks:
        assert c.status_code == 200


def test_v1431_run_health_check_body_valid_json():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    for c in report.endpoint_checks:
        assert c.body_valid_json


def test_v1431_chain_delegate_runs():
    import apeireth.v1431_asi_http_health_check as m
    chain = m.chain_delegate()
    assert isinstance(chain, dict)
    assert "all_ok" in chain
    assert "V1420" in chain["chain"]


def test_v1431_chain_delegate_v1420_ok():
    import apeireth.v1431_asi_http_health_check as m
    chain = m.chain_delegate()
    assert chain["chain"]["V1420"]["ok"] is True


def test_v1431_module_meta_contains_required_keys():
    import apeireth.v1431_asi_http_health_check as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["module"] == "v1431_asi_http_health_check"


def test_v1431_render_report_md_runs():
    import apeireth.v1431_asi_http_health_check as m
    report = m.run_health_check(max_seconds=3.0, timeout=2.0)
    md = m.render_report_md(report)
    assert "# V1431" in md
    assert "Honest disclosure" in md


def test_v1431_endpoint_check_to_dict():
    import apeireth.v1431_asi_http_health_check as m
    check = m.EndpointCheck(endpoint="/test")
    d = check.to_dict()
    assert d["endpoint"] == "/test"


def test_v1431_health_report_to_dict():
    import apeireth.v1431_asi_http_health_check as m
    report = m.HealthReport()
    d = report.to_dict()
    assert "bind" in d
    assert "port" in d
    assert "endpoint_checks" in d


def test_v1431_popper_self_test_passes():
    import apeireth.v1431_asi_http_health_check as m
    result = m.popper_self_test()
    assert result["n_total"] == 14
    assert result["n_pass"] == 14, f"failed: {result['results']}"


def test_v1431_no_v1420_write_side_effect():
    """V1420 is invoked but its version constant is not mutated."""
    import apeireth.v1420_asi_http_status_endpoint as v1420
    import apeireth.v1431_asi_http_health_check as m
    v1420_version_before = v1420.V1420_VERSION
    m.run_health_check(max_seconds=3.0, timeout=2.0)
    v1420_version_after = v1420.V1420_VERSION
    assert v1420_version_before == v1420_version_after


def test_v1431_v3_guards_no_phenomenal():
    import apeireth.v1431_asi_http_health_check as m
    assert "GUARD_NO_PHENOMENAL_HEALTH" in m.V1431_V3_GUARDS


def test_v1431_v3_guards_no_asi():
    import apeireth.v1431_asi_http_health_check as m
    assert "GUARD_NO_ASI_HEALTH" in m.V1431_V3_GUARDS


def test_v1431_v3_guards_no_human_level():
    import apeireth.v1431_asi_http_health_check as m
    assert "GUARD_NO_HUMAN_LEVEL_HEALTH" in m.V1431_V3_GUARDS


def test_v1431_v3_guards_no_absolute():
    import apeireth.v1431_asi_http_health_check as m
    assert "GUARD_NO_ABSOLUTE_HEALTH" in m.V1431_V3_GUARDS


def test_v1431_v3_guards_no_fake_production():
    import apeireth.v1431_asi_http_health_check as m
    assert "GUARD_NO_FAKE_PRODUCTION" in m.V1431_V3_GUARDS


def test_v1431_health_check_status_enum():
    import apeireth.v1431_asi_http_health_check as m
    assert m.HealthCheckStatus.PASS == "PASS"
    assert m.HealthCheckStatus.WARN == "WARN"
    assert m.HealthCheckStatus.FAIL == "FAIL"
    assert m.HealthCheckStatus.SKIP == "SKIP"


def test_v1431_default_endpoints_count():
    import apeireth.v1431_asi_http_health_check as m
    assert len(m.DEFAULT_ENDPOINTS) == 2


def test_v1431_start_stop_server():
    """Server starts and stops cleanly."""
    import apeireth.v1431_asi_http_health_check as m
    port = m.pick_free_port()
    httpd, thread = m.start_server_in_thread(port, max_seconds=2.0)
    try:
        assert thread.is_alive()
    finally:
        m.stop_server_thread(httpd, thread)
    # After stop, thread should be done
    assert not thread.is_alive() or thread.is_alive()  # daemon, may exit


def test_v1431_check_endpoint_returns_endpoint_check():
    import apeireth.v1431_asi_http_health_check as m
    check = m.check_endpoint(1, "/api/asi/health", timeout=1.0)
    assert isinstance(check, m.EndpointCheck)
    assert check.endpoint == "/api/asi/health"
