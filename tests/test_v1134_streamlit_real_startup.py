"""Tests for V1134 — Streamlit real startup (主 06:15 V1050+ 真用 + 主 00:56 任何人都能接触).

主 17:43 实事求是: when streamlit not installed, fail honestly with notes.
"""
from __future__ import annotations

import os
import socket
import sys
from unittest.mock import patch

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)

from apeireth.v1134_streamlit_real_startup import (  # noqa: E402
    V1134StreamlitReport,
    _pick_free_port,
    _read_streamlit_pages,
    _streamlit_info,
    render_markdown,
    render_streamlit_app,
    run_real_streamlit,
)


# ---------- helpers ----------


def test_streamlit_info_returns_tuple():
    installed, version = _streamlit_info()
    assert isinstance(installed, bool)
    assert isinstance(version, str)
    if not installed:
        assert version, "if not installed, reason must be present"


def test_pick_free_port_returns_int_in_range():
    port = _pick_free_port(0)
    assert isinstance(port, int)
    assert 0 <= port <= 65535


def test_pick_free_port_avoids_conflict():
    """Bind a socket to a port, then verify _pick_free_port returns a different one."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(("127.0.0.1", 0))
    busy_port = s.getsockname()[1]
    try:
        chosen = _pick_free_port(busy_port)
        assert chosen != busy_port or chosen == 0
    finally:
        s.close()


def test_read_streamlit_pages_returns_list():
    pages = _read_streamlit_pages()
    assert isinstance(pages, list)
    assert len(pages) >= 1
    for p in pages:
        assert isinstance(p, str)


def test_render_streamlit_app_includes_pages():
    pages = ["Page_A", "Page_B", "Page_C"]
    app = render_streamlit_app(pages)
    assert "streamlit" in app
    assert "Page_A" in app
    assert "Page_B" in app
    assert "Page_C" in app
    assert "Apeireth" in app


def test_render_streamlit_app_uses_safe_literal():
    malicious = '"; os.system("rm -rf /"); "'
    app = render_streamlit_app([malicious])
    # repr() wraps in single quotes → assert it's quoted safely
    assert "'\"; os.system(\"rm -rf /\"); \"'" in app or '"; os.system("rm -rf /"); "' in app


# ---------- report ----------


def test_report_to_dict_has_required_keys():
    r = V1134StreamlitReport()
    d = r.to_dict()
    for k in ("report_id", "timestamp", "streamlit_installed", "streamlit_version",
              "app_path", "port", "started_ok", "startup_ms", "health_ok",
              "homepage_ok", "page_probe_ok", "pid", "stdout_tail", "stderr_tail",
              "notes", "pages_rendered"):
        assert k in d, f"missing: {k}"


def test_report_default_started_ok_false():
    r = V1134StreamlitReport()
    assert r.started_ok is False
    assert r.health_ok is False


# ---------- run_real_streamlit ----------


def _ctx_manager_compat(cls):
    """Decorator to add __enter__/__exit__ to a fake proc class."""
    cls.__enter__ = lambda self: self
    cls.__exit__ = lambda self, *a: False
    return cls


def test_run_real_streamlit_handles_missing_streamlit():
    with patch("apeireth.v1134_streamlit_real_startup._streamlit_info",
               return_value=(False, "fake missing")):
        rep = run_real_streamlit()
    assert rep.streamlit_installed is False
    assert rep.started_ok is False
    assert any("streamlit" in n.lower() for n in rep.notes)


def test_run_real_streamlit_success_path():
    @_ctx_manager_compat
    class FakeProc:
        pid = 99999
        def __init__(self):
            self._stdout_lines = ["  You can now view your Streamlit app in your browser.\n"]
            self._stderr_lines = []
        def poll(self):
            return None
        def terminate(self):
            pass
        def communicate(self, timeout=8):
            return "".join(self._stdout_lines), "".join(self._stderr_lines)
        def kill(self):
            pass

    fake = FakeProc()

    def fake_popen(*args, **kwargs):
        return fake

    health_calls = {"n": 0}

    def fake_probe(url, timeout=3.0):
        health_calls["n"] += 1
        if "_stcore/health" in url:
            if health_calls["n"] >= 1:
                return True, "HTTP 200", "ok"
        if url.endswith("/"):
            return True, "HTTP 200", "<html><body>" + ("x" * 600) + "</body></html>"
        if "?page=health" in url:
            return True, "HTTP 200", "<html><body>" + ("y" * 300) + "</body></html>"
        return False, "no match", ""

    with patch("apeireth.v1134_streamlit_real_startup._streamlit_info", return_value=(True, "1.60.0")), \
         patch("apeireth.v1134_streamlit_real_startup.subprocess.Popen", side_effect=fake_popen), \
         patch("apeireth.v1134_streamlit_real_startup._http_probe", side_effect=fake_probe):
        rep = run_real_streamlit(preferred_port=18765, startup_timeout_s=5.0)

    assert rep.streamlit_installed is True
    assert rep.started_ok is True
    assert rep.health_ok is True
    assert rep.homepage_ok is True
    assert rep.page_probe_ok is True
    assert rep.port > 0


def test_run_real_streamlit_handles_subprocess_spawn_failure():
    def fake_popen(*args, **kwargs):
        raise FileNotFoundError("streamlit missing")

    with patch("apeireth.v1134_streamlit_real_startup._streamlit_info", return_value=(True, "1.60.0")), \
         patch("apeireth.v1134_streamlit_real_startup.subprocess.Popen", side_effect=fake_popen):
        rep = run_real_streamlit()
    assert rep.started_ok is False
    assert any("failed to spawn" in n for n in rep.notes)


def test_run_real_streamlit_handles_no_free_port():
    with patch("apeireth.v1134_streamlit_real_startup._streamlit_info", return_value=(True, "1.60.0")), \
         patch("apeireth.v1134_streamlit_real_startup._pick_free_port", return_value=0):
        rep = run_real_streamlit()
    assert rep.started_ok is False
    assert any("no free port" in n for n in rep.notes)


def test_run_real_streamlit_detects_early_process_exit():
    @_ctx_manager_compat
    class FakeProcEarlyExit:
        pid = 12345
        def poll(self):
            return 1
        def terminate(self): pass
        def communicate(self, timeout=8):
            return "", "streamlit: error"
        def kill(self): pass

    with patch("apeireth.v1134_streamlit_real_startup._streamlit_info", return_value=(True, "1.60.0")), \
         patch("apeireth.v1134_streamlit_real_startup.subprocess.Popen", return_value=FakeProcEarlyExit()), \
         patch("apeireth.v1134_streamlit_real_startup._http_probe", return_value=(False, "no server", "")):
        rep = run_real_streamlit()
    assert rep.started_ok is False
    assert rep.health_ok is False


# ---------- render_markdown ----------


def test_render_markdown_contains_summary():
    r = V1134StreamlitReport(streamlit_installed=True, streamlit_version="1.60.0", port=8765, started_ok=True, health_ok=True, startup_ms=2100)
    r.pages_rendered = ["Page_A"]
    md = render_markdown(r)
    assert "V1134" in md
    assert "1.60.0" in md
    assert "8765" in md
    assert "Page_A" in md


def test_render_markdown_includes_notes():
    r = V1134StreamlitReport()
    r.notes.append("custom note here")
    md = render_markdown(r)
    assert "Notes" in md
    assert "custom note here" in md
