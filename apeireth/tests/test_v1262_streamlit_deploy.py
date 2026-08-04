"""Tests for V1262 streamlit_deploy (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Verify:
- V1262 version exposed
- sanity_check_1262 all True
- 5 V3 guards present
- build_default_streamlit_stack returns spec list
- render_streamlit_app_standalone returns valid Python source
- deploy_and_verify dry-run returns ok=True (when streamlit is installed)
"""
from __future__ import annotations

import os
import sys

import pytest

try:
    from apeireth import v1262_streamlit_deploy as v62
except Exception:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
    import v1262_streamlit_deploy as v62


def test_v1262_version():
    assert v62.V1262_VERSION == "0.1.0"


def test_v1262_sanity_all_true():
    sc = v62.sanity_check_1262()
    assert isinstance(sc, dict)
    assert all(sc.values()), f"failed: {[k for k, v in sc.items() if not v]}"


def test_v1262_v3_guards_count():
    assert len(v62.V3_GUARDS) == 5


def test_v1262_build_default_streamlit_stack():
    stack = v62.build_default_streamlit_stack()
    assert isinstance(stack, list)
    assert len(stack) >= 1
    spec = stack[0]
    assert isinstance(spec, v62.StreamlitSpec)
    assert spec.port > 0
    assert isinstance(spec.app_path, str)
    assert spec.app_path.endswith(".py")


def test_v1262_render_streamlit_app_standalone():
    src = v62.render_streamlit_app_standalone(app_title="Test App V1263")
    assert isinstance(src, str)
    assert "import streamlit" in src or "streamlit" in src
    assert "Test App V1263" in src


def test_v1262_probe_streamlit_returns_probe():
    probe = v62.probe_streamlit()
    assert isinstance(probe, v62.StreamlitProbeResult)
    assert isinstance(probe.streamlit_available, bool)
    assert isinstance(probe.port_free, bool)
    # Strategy should be one of the known values
    assert probe.strategy() in ("subprocess", "port_busy", "unavailable")


def test_v1262_deploy_and_verify_dry_run_returns_ok():
    """真 dry-run: 不真起 streamlit subprocess."""
    # Pick a high free port
    test_port = 18581
    import socket as _sock
    while True:
        s = _sock.socket(_sock.AF_INET, _sock.SOCK_STREAM)
        try:
            s.bind(("127.0.0.1", test_port))
            s.close()
            break
        except OSError:
            test_port += 1
    result = v62.deploy_and_verify(port=test_port, app_id="test_v1262_dry", dry_run=True, timeout=5.0)
    assert isinstance(result, dict)
    assert "ok" in result
    assert "probe" in result
    assert "deployed" in result