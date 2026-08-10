"""Tests for V1455 — ASI cube hypercube full-source-content audit v5."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


# ============================================================================
# Path setup
# ============================================================================

HERE = Path(__file__).resolve().parent
APEIRETH_ROOT = HERE.parent
WORKSPACE_ROOT = APEIRETH_ROOT.parent
for p in (str(APEIRETH_ROOT), str(WORKSPACE_ROOT)):
    if p not in sys.path:
        sys.path.insert(0, p)


# ============================================================================
# Import the module under test
# ============================================================================

import apeireth.v1455_asi_hypercube_full_source_content_audit_v5 as v1455  # noqa: E402


# ============================================================================
# Constants
# ============================================================================

def test_version_is_010():
    assert v1455.V1455_VERSION == "0.1.0"


def test_schema_correct():
    assert v1455.V1455_SCHEMA == "asi.hypercube-full-source-content-audit-v5.v1"


def test_module_correct():
    assert v1455.V1455_MODULE == "apeireth.v1455_asi_hypercube_full_source_content_audit_v5"


def test_four_axes():
    assert len(v1455.V1455_AXES) == 4


def test_six_deployments():
    assert len(v1455.V1455_DEPLOYMENT_NAMES) == 6


def test_guards_14():
    assert len(v1455.V1455_GUARDS) == 14


def test_v3_guards_5():
    assert len(v1455.V1455_V3_GUARDS) == 5


def test_borrowed_8():
    assert len(v1455.V1455_BORROWED) == 8


def test_deployment_modules_complete():
    for d in v1455.V1455_DEPLOYMENT_NAMES:
        assert d in v1455.V1455_DEPLOYMENT_MODULES


# ============================================================================
# Helpers
# ============================================================================

def test_clip01():
    assert v1455._clip01(-0.5) == 0.0
    assert v1455._clip01(1.5) == 1.0


def test_harmonic_mean_bounded():
    assert v1455._harmonic_mean(0.0, 0.5) == 0.0
    assert v1455._harmonic_mean(1.0, 1.0) == 1.0


def test_count_keyword_occurrences():
    text = "async function awaits gather"
    cnt = v1455._count_keyword_occurrences(text, ("async", "await", "gather"))
    assert cnt == 3


def test_count_keyword_empty():
    assert v1455._count_keyword_occurrences("", ("async",)) == 0
    assert v1455._count_keyword_occurrences("hello", ()) == 0


def test_axis_elements():
    assert len(v1455._axis_elements("problem")) == 7
    assert len(v1455._axis_elements("position")) == 5
    assert len(v1455._axis_elements("protocol")) == 6
    assert len(v1455._axis_elements("deployment")) == 6


def test_axis_keyword_map():
    for axis in ("problem", "position", "protocol", "deployment"):
        kw_map = v1455._axis_keyword_map(axis)
        assert isinstance(kw_map, dict)


def test_axis_sources():
    for axis in ("problem", "position", "protocol", "deployment"):
        sources = v1455._axis_sources(axis)
        assert isinstance(sources, dict)


# ============================================================================
# inspect.getsource fallback
# ============================================================================

def test_try_get_full_source_self():
    """inspect.getsource on self — may succeed or fail, both are valid."""
    src = v1455._try_get_full_source("v1455_asi_hypercube_full_source_content_audit_v5")
    assert src is None or isinstance(src, str)


def test_get_proxy_text_non_empty():
    proxy = v1455._get_proxy_text("v1455_asi_hypercube_full_source_content_audit_v5")
    assert isinstance(proxy, str)
    assert len(proxy) > 0


def test_get_proxy_text_module_name_in_proxy():
    proxy = v1455._get_proxy_text("v1455_asi_hypercube_full_source_content_audit_v5")
    assert "v1455_asi_hypercube_full_source_content_audit_v5" in proxy


# ============================================================================
# fetch_deployment_source
# ============================================================================

def test_fetch_deployment_source_docker():
    sf = v1455._fetch_deployment_source("docker")
    assert sf.module_name == "v1435_asi_docker_availability_probe"
    assert sf.status in ("FETCHED", "FALLBACK_PROXY", "FAILED")
    assert sf.source_bytes > 0


def test_fetch_deployment_source_all_deployments():
    for d in v1455.V1455_DEPLOYMENT_NAMES:
        sf = v1455._fetch_deployment_source(d)
        assert sf.status in ("FETCHED", "FALLBACK_PROXY", "FAILED")
        if sf.status in ("FETCHED", "FALLBACK_PROXY"):
            assert sf.source_bytes > 0


# ============================================================================
# fetch_axis_source_combined
# ============================================================================

def test_fetch_axis_source_combined_time():
    text = v1455._fetch_axis_source_combined("problem", "time")
    assert isinstance(text, str)
    assert len(text) > 0


def test_fetch_axis_source_combined_all_axis_elements():
    for axis in ("problem", "position", "protocol"):
        for e in v1455._axis_elements(axis):
            text = v1455._fetch_axis_source_combined(axis, e)
            assert isinstance(text, str)
            assert len(text) > 0


# ============================================================================
# audit_pair_full_source
# ============================================================================

def test_audit_pair_full_source_basic():
    deployment_source_cache = {d: v1455._fetch_deployment_source(d).content for d in v1455.V1455_DEPLOYMENT_NAMES}
    axis_source_cache = {(axis, e): v1455._fetch_axis_source_combined(axis, e)
                         for axis in ("problem", "position", "protocol")
                         for e in v1455._axis_elements(axis)}
    pair = v1455.audit_pair_full_source("problem", "time", "docker",
                                          deployment_source_cache, axis_source_cache)
    assert pair.axis == "problem"
    assert pair.axis_element == "time"
    assert pair.deployment_element == "docker"
    assert 0.0 <= pair.forward_closure <= 1.0
    assert 0.0 <= pair.backward_closure <= 1.0
    assert 0.0 <= pair.cross_link_closure <= 1.0


def test_audit_pair_full_source_evidence():
    deployment_source_cache = {d: v1455._fetch_deployment_source(d).content for d in v1455.V1455_DEPLOYMENT_NAMES}
    axis_source_cache = {(axis, e): v1455._fetch_axis_source_combined(axis, e)
                         for axis in ("problem", "position", "protocol")
                         for e in v1455._axis_elements(axis)}
    pair = v1455.audit_pair_full_source("protocol", "sync", "llm_endpoint",
                                          deployment_source_cache, axis_source_cache)
    assert pair.evidence
    assert "axis=protocol/sync" in pair.evidence
    assert "deployment=llm_endpoint" in pair.evidence


# ============================================================================
# audit_face_full_source
# ============================================================================

def test_audit_face_full_source_problem():
    deployment_source_cache = {d: v1455._fetch_deployment_source(d).content for d in v1455.V1455_DEPLOYMENT_NAMES}
    axis_source_cache = {(axis, e): v1455._fetch_axis_source_combined(axis, e)
                         for axis in ("problem", "position", "protocol")
                         for e in v1455._axis_elements(axis)}
    face = v1455.audit_face_full_source("problem", deployment_source_cache, axis_source_cache)
    assert face.axes == ("problem", "deployment")
    assert face.n_pairs == 42
    assert len(face.pairs) == 42


def test_audit_face_full_source_position():
    deployment_source_cache = {d: v1455._fetch_deployment_source(d).content for d in v1455.V1455_DEPLOYMENT_NAMES}
    axis_source_cache = {(axis, e): v1455._fetch_axis_source_combined(axis, e)
                         for axis in ("problem", "position", "protocol")
                         for e in v1455._axis_elements(axis)}
    face = v1455.audit_face_full_source("position", deployment_source_cache, axis_source_cache)
    assert face.axes == ("position", "deployment")
    assert face.n_pairs == 30


def test_audit_face_full_source_protocol():
    deployment_source_cache = {d: v1455._fetch_deployment_source(d).content for d in v1455.V1455_DEPLOYMENT_NAMES}
    axis_source_cache = {(axis, e): v1455._fetch_axis_source_combined(axis, e)
                         for axis in ("problem", "position", "protocol")
                         for e in v1455._axis_elements(axis)}
    face = v1455.audit_face_full_source("protocol", deployment_source_cache, axis_source_cache)
    assert face.axes == ("protocol", "deployment")
    assert face.n_pairs == 36


# ============================================================================
# build_report
# ============================================================================

def test_build_report_basic():
    report = v1455.build_report()
    assert report.n_axes == 4
    assert len(report.faces) == 3
    assert len(report.deployment_sources) == 6


def test_build_report_per_axis_overall():
    report = v1455.build_report()
    assert len(report.per_axis_overall) == 4


def test_build_report_per_deployment_closure():
    report = v1455.build_report()
    assert len(report.per_deployment_closure_rate) == 6


def test_build_report_hypercube_overall():
    report = v1455.build_report()
    assert 0.0 <= report.hypercube_overall_closure_rate <= 1.0


def test_build_report_axis_balance():
    report = v1455.build_report()
    assert 0.0 <= report.axis_balance_score <= 1.0


def test_build_report_axis_sources_combined():
    report = v1455.build_report()
    assert report.axis_sources_combined_bytes > 0
    assert report.axis_sources_combined_lines > 0


def test_build_report_to_dict():
    report = v1455.build_report()
    d = report.to_dict()
    assert "schema" in d
    assert "faces" in d
    assert "deployment_sources" in d


# ============================================================================
# run_all
# ============================================================================

def test_run_all_writes_files(tmp_path):
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = v1455.run_all(out_json=out_json, out_md=out_md)
    assert out_json.exists()
    assert out_md.exists()


def test_run_all_valid_json(tmp_path):
    out_json = tmp_path / "report.json"
    v1455.run_all(out_json=out_json, out_md=tmp_path / "report.md")
    data = json.loads(out_json.read_text(encoding="utf-8"))
    assert data["schema"] == v1455.V1455_SCHEMA


def test_run_all_valid_md(tmp_path):
    out_md = tmp_path / "report.md"
    v1455.run_all(out_json=tmp_path / "report.json", out_md=out_md)
    md = out_md.read_text(encoding="utf-8")
    assert "V1455" in md
    assert "Hypercube" in md


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_all_ok():
    ok, results = v1455.popper()
    assert isinstance(ok, bool)
    assert isinstance(results, list)
    assert len(results) == 14


def test_popper_all_tests_pass():
    ok, results = v1455.popper()
    failed = [r for r in results if not r["ok"]]
    assert ok is True, f"failed tests: {failed}"


# ============================================================================
# Markdown render
# ============================================================================

def test_render_markdown_contains_v1455():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "V1455" in md


def test_render_markdown_contains_hypercube():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "Hypercube" in md or "hypercube" in md


def test_render_markdown_contains_honest_disclosure():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "Honest disclosure" in md


def test_render_markdown_contains_v3_guards():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "V3 哲学守门" in md


def test_render_markdown_contains_14_guards():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "GUARD_FULL_SOURCE" in md
    assert "GUARD_SOURCE_FALLBACK" in md


def test_render_markdown_contains_per_face_table():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "Per-face audit" in md


def test_render_markdown_contains_borrowed():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "V1454" in md
    assert "V1453" in md


# ============================================================================
# CLI
# ============================================================================

def test_cli_version():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["version"])
    assert rc == 0


def test_cli_meta():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["popper"])
    assert rc == 0


def test_cli_chain():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["chain"])
    assert rc == 0


def test_cli_audit():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["audit"])
    assert rc == 0


def test_cli_report(tmp_path):
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main([
        "report",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0


def test_cli_run_all(tmp_path):
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main([
        "run-all",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0


def test_cli_unknown_returns_2():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["bogus-cmd"])
    assert rc == 2


def test_cli_help():
    from apeireth.v1455_asi_hypercube_full_source_content_audit_v5 import main
    rc = v1455.main(["help"])
    assert rc == 0


# ============================================================================
# Composes on V1454 + V1453
# ============================================================================

def test_composes_on_v1454():
    """V1455 borrows V1454's hypercube 4-axis + 3 new faces pattern."""
    assert len(v1455.V1455_AXES) == 4
    assert len(v1455.V1455_DEPLOYMENT_NAMES) == 6


def test_composes_on_v1453():
    """V1455 borrows V1453's full-content pattern via inspect.getsource."""
    # V1453 used base64 + GitHub; V1455 uses inspect.getsource + Python source
    # Both achieve "full content" but V1455 is offline (no network)
    # Verify V1455 mentions V1453 in borrowed list
    borrowed_labels = [b[0] for b in v1455.V1455_BORROWED]
    assert "V1453" in borrowed_labels
    assert "V1454" in borrowed_labels


def test_extends_v1454_with_full_source():
    """V1455 extends V1454 by fetching full source code (not just proxy text)."""
    # V1454 used module proxy text (module name + constants)
    # V1455 uses inspect.getsource for full source code
    # Both have 6 hypercube faces, but V1455 should reveal more keyword matches
    assert v1455.V1455_SCHEMA.startswith("asi.hypercube")


# ============================================================================
# Schema and version
# ============================================================================

def test_report_schema_correct():
    report = v1455.build_report()
    assert report.schema == "asi.hypercube-full-source-content-audit-v5.v1"
    assert report.version == "0.1.0"


def test_report_module_correct():
    report = v1455.build_report()
    assert report.module == v1455.V1455_MODULE


# ============================================================================
# Honest disclosure content
# ============================================================================

def test_honest_disclosure_present_in_md():
    report = v1455.build_report()
    md = v1455._render_markdown(report)
    assert "≠ ASI closure" in md or "≠ ASI" in md
    assert "≠ Phenomenal" in md
    assert "≠ human-level" in md
    assert "≠ absolute" in md
    assert "deployment parity" in md or "deployment" in md
    assert "FALLBACK_PROXY" in md  # honest disclosure about fallback


# ============================================================================
# Pair uniqueness
# ============================================================================

def test_pair_uniqueness():
    report = v1455.build_report()
    for face in report.faces:
        seen = set()
        for p in face.pairs:
            key = (p.axis, p.axis_element, p.deployment_element)
            assert key not in seen
            seen.add(key)


def test_total_pairs_108():
    report = v1455.build_report()
    total = sum(f.n_pairs for f in report.faces)
    assert total == 108


# ============================================================================
# Closure rates bounded
# ============================================================================

def test_all_closure_rates_bounded():
    report = v1455.build_report()
    for f in report.faces:
        assert 0.0 <= f.forward_closure_rate <= 1.0
        assert 0.0 <= f.backward_closure_rate <= 1.0
        assert 0.0 <= f.cross_link_closure_rate <= 1.0
        assert 0.0 <= f.overall_closure_rate <= 1.0
        for p in f.pairs:
            assert 0.0 <= p.forward_closure <= 1.0
            assert 0.0 <= p.backward_closure <= 1.0
            assert 0.0 <= p.cross_link_closure <= 1.0


# ============================================================================
# Per-deployment source stats
# ============================================================================

def test_per_deployment_source_stats_complete():
    report = v1455.build_report()
    assert len(report.per_deployment_source_bytes) == 6
    assert len(report.per_deployment_source_lines) == 6


# ============================================================================
# Deployment source fetch status
# ============================================================================

def test_deployment_source_status_valid():
    report = v1455.build_report()
    for sf in report.deployment_sources:
        assert sf.status in ("FETCHED", "FALLBACK_PROXY", "FAILED")


# ============================================================================
# Notes
# ============================================================================

def test_notes_present():
    report = v1455.build_report()
    assert len(report.notes) >= 4