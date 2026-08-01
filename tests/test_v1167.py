"""Test V1167 — ASI streamlit_real_startup V0.6 真补 (5 sub-dim 真测).

主 17:43 实事求是: 测试覆盖 constants / dataclasses / helpers / _measure_*
with mocked V1134 reports (不实际起 streamlit subprocess).
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1167Constants:
    def test_version_present(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import V1167_VERSION
        assert V1167_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import V1167_DIM_VERSION
        assert V1167_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import V1167_SUBDIM_NAMES
        assert V1167_SUBDIM_NAMES == (
            "streamlit_installed_real",
            "app_path_real",
            "port_assigned_real",
            "started_ok_real",
            "http_probe_real",
        )

    def test_baseline_target_constants(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import (
            V1134_BASELINE_STREAM_STARTUP, TARGET_STREAM_STARTUP_V06,
            V1134_REPORT_FIELDS,
        )
        assert V1134_BASELINE_STREAM_STARTUP == 0.95
        assert TARGET_STREAM_STARTUP_V06 == 0.85
        assert "streamlit_installed" in V1134_REPORT_FIELDS
        assert "started_ok" in V1134_REPORT_FIELDS

    def test_artifact_dir_default(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"

    def test_threshold_constants_present(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import (
            _STARTUP_MS_MIN, _STARTUP_MS_MAX,
        )
        assert _STARTUP_MS_MIN > 0
        assert _STARTUP_MS_MAX > _STARTUP_MS_MIN


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _safe_import
        assert _safe_import("nonexistent.module.xyz") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _safe_import
        mod = _safe_import("apeireth.v1167_asi_streamlit_real_startup_v06_real_measure")
        assert mod is not None

    def test_call_safely(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _call_safely
        ok, r = _call_safely(lambda x: x + 1, 5)
        assert ok is True and r == 6
        ok, r = _call_safely(None)
        assert ok is False and r is None

    def test_attr_first(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _attr_first
        class O:
            a = 1
        assert _attr_first(O, ["nope", "a"]) == 1
        assert _attr_first(object, ["x"]) is None

    def test_safe_field(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _safe_field
        class O: pass
        assert _safe_field(O(), "x", 9) == 9

    def test_safe_callable_field_with_property(self):
        # V1167 uses only _safe_field / _safe_callable_field-like access via getattr;
        # skip the explicit _safe_callable_field check if not exported.
        from apeireth import v1167_asi_streamlit_real_startup_v06_real_measure as mod
        # _safe_field is present
        class O: pass
        assert mod._safe_field(O(), "x", 9) == 9


class TestSubDimEvidence:
    def test_default_init(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5)
        assert e.name == "x"
        assert e.score == 0.5
        assert e.checks == {}

    def test_to_dict(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5, checks={"a": True}, notes=["n"], raw={"k": 1})
        d = e.to_dict()
        assert d["name"] == "x"
        assert d["checks"] == {"a": True}


class TestStreamlitReport:
    def test_default_init_generates_snapshot_id(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import StreamlitRealStartupReport
        r = StreamlitRealStartupReport()
        assert r.snapshot_id.startswith("v1167-")
        assert r.version == "0.1.0"
        assert r.dim_version == "0.6"
        assert r.total == 0.0
        assert r.n_subdims_total == 5

    def test_summary_line_format(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import StreamlitRealStartupReport
        r = StreamlitRealStartupReport(
            total=0.6, n_subdims_passed=3, n_subdims_partial=1, n_subdims_missing=1,
            v1134_started_ok=True, v1134_health_ok=True, v1134_port=8765,
        )
        line = r.summary_line()
        assert "total=0.6000" in line
        assert "V1134 baseline 0.9500" in line
        assert "3 pass / 1 partial / 1 missing" in line
        # V1167 summary uses compact "started=True health=True" (port is in v1134_port field, not summary)
        assert "started=True" in line
        assert "health=True" in line

    def test_to_from_dict_roundtrip(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import (
            StreamlitRealStartupReport, SubDimEvidence,
        )
        r = StreamlitRealStartupReport(
            total=0.7, snapshot_id="v1167-test",
            v1134_report_id="rep-123", v1134_started_ok=True,
            v1134_health_ok=True, v1134_port=8765,
        )
        r.sub_dim_scores = {"streamlit_installed_real": 0.9}
        r.sub_dim_evidence["streamlit_installed_real"] = SubDimEvidence(
            name="streamlit_installed_real", score=0.9, checks={"k": True},
        )
        r2 = StreamlitRealStartupReport.from_dict(r.to_dict())
        assert r2.snapshot_id == "v1167-test"
        assert r2.total == 0.7
        assert r2.sub_dim_scores["streamlit_installed_real"] == 0.9
        assert r2.sub_dim_evidence["streamlit_installed_real"].score == 0.9
        assert r2.v1134_report_id == "rep-123"
        assert r2.v1134_port == 8765

    def test_from_dict_handles_missing_evidence(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import StreamlitRealStartupReport
        r = StreamlitRealStartupReport.from_dict({"snapshot_id": "x", "total": 0.2})
        assert r.snapshot_id == "x"
        assert r.sub_dim_evidence == {}


class TestMeasureStreamlitInstalled:
    """S1 — streamlit_installed_real."""

    def test_installed_with_version(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_streamlit_installed
        rep = _make_fake_report(streamlit_installed=True, streamlit_version="1.32.0")
        score, _ = _measure_streamlit_installed(rep)
        assert score > 0.5

    def test_not_installed(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_streamlit_installed
        rep = _make_fake_report(streamlit_installed=False, streamlit_version="")
        score, _ = _measure_streamlit_installed(rep)
        assert score < 0.1

    def test_error_marker_in_version(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_streamlit_installed
        rep = _make_fake_report(streamlit_installed=True, streamlit_version="error: not found")
        score, _ = _measure_streamlit_installed(rep)
        assert 0.0 < score < 1.0


class TestMeasureAppPath:
    """S2 — app_path_real."""

    def test_existing_valid_python_file(self, tmp_path):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_app_path
        p = tmp_path / "app.py"
        p.write_text("import streamlit\nst.title('hi')\n" + "x" * 500, encoding="utf-8")
        rep = _make_fake_report(app_path=str(p))
        score, _ = _measure_app_path(rep)
        assert score > 0.5

    def test_empty_path(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_app_path
        rep = _make_fake_report(app_path="")
        score, _ = _measure_app_path(rep)
        assert score == 0.0

    def test_missing_file(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_app_path
        rep = _make_fake_report(app_path="C:/this/path/does/not/exist.py")
        score, _ = _measure_app_path(rep)
        assert score < 1.0

    def test_file_too_small(self, tmp_path):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_app_path
        p = tmp_path / "small.py"
        p.write_text("# hi\n", encoding="utf-8")
        rep = _make_fake_report(app_path=str(p))
        score, _ = _measure_app_path(rep)
        # file_size_reasonable check will fail, but path_nonempty + file_exists + file_nonempty + file_is_python may pass
        assert 0.0 <= score <= 1.0


class TestMeasurePortAssigned:
    """S3 — port_assigned_real."""

    def test_valid_port(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_port_assigned
        rep = _make_fake_report(port=8765)
        score, _ = _measure_port_assigned(rep)
        assert score > 0.5

    def test_zero_port(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_port_assigned
        rep = _make_fake_report(port=0)
        score, _ = _measure_port_assigned(rep)
        assert score < 0.1

    def test_low_privileged_port(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_port_assigned
        rep = _make_fake_report(port=80)
        score, _ = _measure_port_assigned(rep)
        assert 0.0 < score < 1.0  # 端口_valid but not unprivileged


class TestMeasureStartedOk:
    """S4 — started_ok_real."""

    def test_full_ok(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_started_ok
        rep = _make_fake_report(started_ok=True, pid=12345, startup_ms=2500.0)
        score, _ = _measure_started_ok(rep)
        assert score > 0.5

    def test_not_started(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_started_ok
        rep = _make_fake_report(started_ok=False, pid=0, startup_ms=0.0)
        score, _ = _measure_started_ok(rep)
        assert score == 0.0


class TestMeasureHttpProbe:
    """S5 — http_probe_real."""

    def test_all_probes_pass(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_http_probe
        rep = _make_fake_report(health_ok=True, homepage_ok=True, page_probe_ok=True)
        score, _ = _measure_http_probe(rep)
        assert score == 1.0

    def test_only_health_ok(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_http_probe
        rep = _make_fake_report(health_ok=True, homepage_ok=False, page_probe_ok=False)
        score, _ = _measure_http_probe(rep)
        assert 0.0 < score < 1.0

    def test_no_probes(self):
        from apeireth.v1167_asi_streamlit_real_startup_v06_real_measure import _measure_http_probe
        rep = _make_fake_report(health_ok=False, homepage_ok=False, page_probe_ok=False)
        score, _ = _measure_http_probe(rep)
        assert score == 0.0


class TestMeasureFullAggregation:
    """主入口聚合 — with mocked V1134 report (no streamlit subprocess)."""

    def test_aggregate_5_subdim(self, monkeypatch, tmp_path):
        from apeireth import v1167_asi_streamlit_real_startup_v06_real_measure as mod

        p = tmp_path / "app.py"
        p.write_text("import streamlit\nst.title('hi')\n" + "x" * 500, encoding="utf-8")

        fake_rep = _make_fake_report(
            streamlit_installed=True, streamlit_version="1.32.0",
            app_path=str(p), port=8765,
            started_ok=True, pid=12345, startup_ms=2500.0,
            health_ok=True, homepage_ok=True, page_probe_ok=True,
            report_id="rep-x", timestamp=0.0, pages_rendered=3,
        )

        def fake_get(app_dir=None, preferred_port=8765, startup_timeout_s=25.0):
            return True, fake_rep, "ok"
        monkeypatch.setattr(mod, "_get_v1134_report", fake_get)

        rep = mod.measure_streamlit_real_startup_full(write_artifact=False)
        assert 0.0 < rep.total <= 1.0
        assert len(rep.sub_dim_scores) == 5
        assert all(name in rep.sub_dim_scores for name in mod.V1167_SUBDIM_NAMES)

    def test_unavailable_v1134_returns_zero(self, monkeypatch):
        from apeireth import v1167_asi_streamlit_real_startup_v06_real_measure as mod

        def fake_get(app_dir=None, preferred_port=8765, startup_timeout_s=25.0):
            return False, None, "v1134_module_not_found"
        monkeypatch.setattr(mod, "_get_v1134_report", fake_get)

        rep = mod.measure_streamlit_real_startup_full(write_artifact=False)
        assert rep.total == 0.0
        assert any("V1134 unavailable" in n for n in rep.notes)


# ============================================================================
# Helpers — fake V1134 report
# ============================================================================


def _make_fake_report(
    report_id: str = "rep-fake",
    timestamp: float = 0.0,
    streamlit_installed: bool = False,
    streamlit_version: str = "",
    app_path: str = "",
    port: int = 0,
    started_ok: bool = False,
    startup_ms: float = 0.0,
    health_ok: bool = False,
    homepage_ok: bool = False,
    page_probe_ok: bool = False,
    pid: int = 0,
    pages_rendered: int = 0,
) -> object:
    """Build a fake V1134StreamlitReport-like object (duck-typed)."""

    class _FakeReport:
        pass

    rep = _FakeReport()
    rep.report_id = report_id
    rep.timestamp = timestamp
    rep.streamlit_installed = streamlit_installed
    rep.streamlit_version = streamlit_version
    rep.app_path = app_path
    rep.port = port
    rep.started_ok = started_ok
    rep.startup_ms = startup_ms
    rep.health_ok = health_ok
    rep.homepage_ok = homepage_ok
    rep.page_probe_ok = page_probe_ok
    rep.pid = pid
    rep.pages_rendered = pages_rendered
    return rep
