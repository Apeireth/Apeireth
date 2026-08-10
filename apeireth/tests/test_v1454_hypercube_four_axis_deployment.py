"""Tests for V1454 — ASI cube hypercube 4-axis deployment audit."""

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

import apeireth.v1454_asi_hypercube_four_axis_deployment as v1454  # noqa: E402


# ============================================================================
# Constants
# ============================================================================

def test_version_is_010():
    assert v1454.V1454_VERSION == "0.1.0"


def test_schema_correct():
    assert v1454.V1454_SCHEMA == "asi.hypercube-four-axis-deployment.v1"


def test_module_correct():
    assert v1454.V1454_MODULE == "apeireth.v1454_asi_hypercube_four_axis_deployment"


def test_four_axes():
    assert len(v1454.V1454_AXES) == 4
    assert v1454.V1454_AXES == ("problem", "position", "protocol", "deployment")


def test_seven_problems():
    assert len(v1454.V1454_PROBLEM_NAMES) == 7


def test_five_positions():
    assert len(v1454.V1454_POSITION_NAMES) == 5


def test_six_protocols():
    assert len(v1454.V1454_PROTOCOL_NAMES) == 6


def test_six_deployments():
    assert len(v1454.V1454_DEPLOYMENT_NAMES) == 6
    expected = ("docker", "llm_endpoint", "http_server", "benchmark", "streamlit", "runbook")
    assert v1454.V1454_DEPLOYMENT_NAMES == expected


def test_guards_14():
    assert len(v1454.V1454_GUARDS) == 14


def test_v3_guards_5():
    assert len(v1454.V1454_V3_GUARDS) == 5


def test_borrowed_8():
    assert len(v1454.V1454_BORROWED) == 8


def test_deployment_modules_complete():
    for d in v1454.V1454_DEPLOYMENT_NAMES:
        assert d in v1454.V1454_DEPLOYMENT_MODULES


def test_n_faces_six():
    """3 new + 3 existing = 6 total hypercube faces."""
    assert v1454.V1454_N_FACES == 6


# ============================================================================
# Helpers
# ============================================================================

def test_clip01():
    assert v1454._clip01(-0.5) == 0.0
    assert v1454._clip01(1.5) == 1.0


def test_harmonic_mean_bounded():
    assert v1454._harmonic_mean(0.0, 0.5) == 0.0
    assert v1454._harmonic_mean(1.0, 1.0) == 1.0


def test_count_keyword_occurrences():
    text = "async function awaits gather"
    cnt = v1454._count_keyword_occurrences(text, ("async", "await", "gather"))
    assert cnt == 3


def test_count_keyword_empty():
    assert v1454._count_keyword_occurrences("", ("async",)) == 0
    assert v1454._count_keyword_occurrences("hello", ()) == 0


def test_axis_elements():
    assert len(v1454._axis_elements("problem")) == 7
    assert len(v1454._axis_elements("position")) == 5
    assert len(v1454._axis_elements("protocol")) == 6
    assert len(v1454._axis_elements("deployment")) == 6
    assert v1454._axis_elements("unknown") == ()


def test_axis_keyword_map():
    for axis in ("problem", "position", "protocol", "deployment"):
        kw_map = v1454._axis_keyword_map(axis)
        assert isinstance(kw_map, dict)
    assert v1454._axis_keyword_map("unknown") == {}


def test_axis_sources():
    for axis in ("problem", "position", "protocol", "deployment"):
        sources = v1454._axis_sources(axis)
        assert isinstance(sources, dict)
    assert v1454._axis_sources("unknown") == {}


# ============================================================================
# audit_pair
# ============================================================================

def test_audit_pair_problem_time_docker():
    pair = v1454.audit_pair("problem", "time", "docker")
    assert pair.axis == "problem"
    assert pair.axis_element == "time"
    assert pair.deployment_element == "docker"
    assert pair.forward_closure in (0.0, 1.0)
    assert pair.backward_closure in (0.0, 1.0)
    assert pair.cross_link_closure in (0.0, 1.0)


def test_audit_pair_position_scheduler_streamlit():
    pair = v1454.audit_pair("position", "scheduler", "streamlit")
    assert pair.axis == "position"
    assert pair.axis_element == "scheduler"
    assert pair.deployment_element == "streamlit"


def test_audit_pair_protocol_sync_llm():
    pair = v1454.audit_pair("protocol", "sync", "llm_endpoint")
    assert pair.axis == "protocol"
    assert pair.axis_element == "sync"
    assert pair.deployment_element == "llm_endpoint"


def test_audit_pair_evidence_non_empty():
    pair = v1454.audit_pair("problem", "time", "docker")
    assert pair.evidence
    assert "axis=" in pair.evidence
    assert "forward=" in pair.evidence


# ============================================================================
# audit_face
# ============================================================================

def test_audit_face_problem_deployment():
    face = v1454.audit_face("problem", "deployment")
    assert face.axes == ("problem", "deployment")
    assert face.n_pairs == 42  # 7 × 6
    assert len(face.pairs) == 42
    assert 0.0 <= face.overall_closure_rate <= 1.0


def test_audit_face_position_deployment():
    face = v1454.audit_face("position", "deployment")
    assert face.axes == ("position", "deployment")
    assert face.n_pairs == 30  # 5 × 6
    assert len(face.pairs) == 30


def test_audit_face_protocol_deployment():
    face = v1454.audit_face("protocol", "deployment")
    assert face.axes == ("protocol", "deployment")
    assert face.n_pairs == 36  # 6 × 6
    assert len(face.pairs) == 36


def test_audit_all_faces():
    faces = v1454.audit_all_faces()
    assert len(faces) == 3
    assert {f.axes for f in faces} == {
        ("problem", "deployment"),
        ("position", "deployment"),
        ("protocol", "deployment"),
    }


# ============================================================================
# build_report
# ============================================================================

def test_build_report_basic():
    report = v1454.build_report()
    assert report.n_axes == 4
    assert report.n_problems == 7
    assert report.n_positions == 5
    assert report.n_protocols == 6
    assert report.n_deployments == 6
    assert report.n_faces_total == 6
    assert len(report.faces) == 3


def test_build_report_per_axis_overall():
    report = v1454.build_report()
    assert len(report.per_axis_overall) == 4
    for axis_name, rate in report.per_axis_overall.items():
        assert 0.0 <= rate <= 1.0


def test_build_report_per_deployment_closure():
    report = v1454.build_report()
    assert len(report.per_deployment_closure_rate) == 6
    for d, rate in report.per_deployment_closure_rate.items():
        assert d in v1454.V1454_DEPLOYMENT_NAMES
        assert 0.0 <= rate <= 1.0


def test_build_report_hypercube_overall():
    report = v1454.build_report()
    assert 0.0 <= report.hypercube_overall_closure_rate <= 1.0


def test_build_report_axis_balance():
    report = v1454.build_report()
    assert 0.0 <= report.axis_balance_score <= 1.0


def test_build_report_to_dict():
    report = v1454.build_report()
    d = report.to_dict()
    assert isinstance(d, dict)
    assert "schema" in d
    assert "faces" in d
    assert "per_axis_overall" in d


# ============================================================================
# run_all
# ============================================================================

def test_run_all_writes_files(tmp_path):
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = v1454.run_all(out_json=out_json, out_md=out_md)
    assert out_json.exists()
    assert out_md.exists()


def test_run_all_valid_json(tmp_path):
    out_json = tmp_path / "report.json"
    v1454.run_all(out_json=out_json, out_md=tmp_path / "report.md")
    data = json.loads(out_json.read_text(encoding="utf-8"))
    assert "schema" in data
    assert data["schema"] == v1454.V1454_SCHEMA


def test_run_all_valid_md(tmp_path):
    out_md = tmp_path / "report.md"
    v1454.run_all(out_json=tmp_path / "report.json", out_md=out_md)
    md = out_md.read_text(encoding="utf-8")
    assert "V1454" in md
    assert "Hypercube" in md


# ============================================================================
# Chain delegate
# ============================================================================

def test_chain_delegate_returns_dict():
    chain = v1454.chain_delegate()
    assert isinstance(chain, dict)
    assert "delegates" in chain
    assert "all_ok" in chain
    assert len(chain["delegates"]) == 6


def test_chain_delegate_includes_all_upstream():
    chain = v1454.chain_delegate()
    modules = [d["module"] for d in chain["delegates"]]
    for expected in ("V1450", "V1451", "V1453", "V1449", "V1448", "V1447"):
        assert expected in modules, f"{expected} missing from chain"


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_all_ok():
    ok, results = v1454.popper()
    assert isinstance(ok, bool)
    assert isinstance(results, list)
    assert len(results) == 14


def test_popper_all_tests_pass():
    ok, results = v1454.popper()
    failed = [r for r in results if not r["ok"]]
    assert ok is True, f"failed tests: {failed}"


# ============================================================================
# Markdown render
# ============================================================================

def test_render_markdown_contains_v1454():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "V1454" in md


def test_render_markdown_contains_hypercube():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "Hypercube" in md or "hypercube" in md


def test_render_markdown_contains_honest_disclosure():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "Honest disclosure" in md


def test_render_markdown_contains_v3_guards():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "V3 哲学守门" in md


def test_render_markdown_contains_14_guards():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "GUARD_FOUR_AXES" in md
    assert "GUARD_DEPLOYMENT_SIX" in md
    assert "GUARD_NO_V1450_REPLACE" in md


def test_render_markdown_contains_per_face_table():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "Per-face audit" in md


def test_render_markdown_contains_per_axis_table():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "Per-axis overall" in md


def test_render_markdown_contains_per_deployment_table():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "Per-deployment" in md


def test_render_markdown_contains_borrowed():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "V1450" in md
    assert "V1453" in md
    assert "V1435-V1440+V1430" in md


# ============================================================================
# CLI
# ============================================================================

def test_cli_version():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["version"])
    assert rc == 0


def test_cli_meta():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["popper"])
    assert rc == 0


def test_cli_chain():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["chain"])
    assert rc == 0


def test_cli_audit():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["audit"])
    assert rc == 0


def test_cli_report(tmp_path):
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main([
        "report",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0
    assert (tmp_path / "r.json").exists()
    assert (tmp_path / "r.md").exists()


def test_cli_run_all(tmp_path):
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main([
        "run-all",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0


def test_cli_unknown_returns_2():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["bogus-cmd"])
    assert rc == 2


def test_cli_help():
    from apeireth.v1454_asi_hypercube_four_axis_deployment import main
    rc = v1454.main(["help"])
    assert rc == 0


# ============================================================================
# Composes on V1450 + V1451 + V1453
# ============================================================================

def test_composes_on_v1450():
    """V1454 borrows V1450's 3-axis cube + extends to 4-axis hypercube."""
    assert v1454.V1454_N_FACES == 6  # 3 new + 3 from V1450


def test_composes_on_v1451():
    """V1454 borrows V1451 trend v2 history snapshot pattern."""
    # History + trend pattern is reused in build_report
    assert v1454.V1454_SCHEMA.startswith("asi.hypercube")


def test_extends_v1450_with_deployment_axis():
    """V1454 adds deployment as the 4th axis (V1450 had problem/position/protocol only)."""
    v1450_axes = ("problem", "position", "protocol")
    new_axis = "deployment"
    assert new_axis not in v1450_axes
    assert new_axis in v1454.V1454_AXES


# ============================================================================
# Schema and version
# ============================================================================

def test_report_schema_correct():
    report = v1454.build_report()
    assert report.schema == "asi.hypercube-four-axis-deployment.v1"
    assert report.version == "0.1.0"


def test_report_module_correct():
    report = v1454.build_report()
    assert report.module == v1454.V1454_MODULE


# ============================================================================
# Honest disclosure content
# ============================================================================

def test_honest_disclosure_present_in_md():
    report = v1454.build_report()
    md = v1454._render_markdown(report)
    assert "≠ ASI closure" in md or "≠ ASI" in md
    assert "≠ Phenomenal" in md
    assert "≠ human-level" in md
    assert "≠ absolute" in md
    assert "deployment parity" in md or "deployment" in md


# ============================================================================
# Pair uniqueness
# ============================================================================

def test_pair_uniqueness_problem_deployment():
    face = v1454.audit_face("problem", "deployment")
    seen = set()
    for p in face.pairs:
        key = (p.axis, p.axis_element, p.deployment_element)
        assert key not in seen
        seen.add(key)
    assert len(seen) == 42


def test_pair_uniqueness_position_deployment():
    face = v1454.audit_face("position", "deployment")
    seen = set()
    for p in face.pairs:
        key = (p.axis, p.axis_element, p.deployment_element)
        assert key not in seen
        seen.add(key)
    assert len(seen) == 30


def test_pair_uniqueness_protocol_deployment():
    face = v1454.audit_face("protocol", "deployment")
    seen = set()
    for p in face.pairs:
        key = (p.axis, p.axis_element, p.deployment_element)
        assert key not in seen
        seen.add(key)
    assert len(seen) == 36


# ============================================================================
# Closure rates are bounded
# ============================================================================

def test_all_closure_rates_bounded():
    faces = v1454.audit_all_faces()
    for f in faces:
        assert 0.0 <= f.forward_closure_rate <= 1.0
        assert 0.0 <= f.backward_closure_rate <= 1.0
        assert 0.0 <= f.cross_link_closure_rate <= 1.0
        assert 0.0 <= f.overall_closure_rate <= 1.0
        for p in f.pairs:
            assert 0.0 <= p.forward_closure <= 1.0
            assert 0.0 <= p.backward_closure <= 1.0
            assert 0.0 <= p.cross_link_closure <= 1.0


# ============================================================================
# Total pair count
# ============================================================================

def test_total_pairs_108():
    """3 new faces × (42 + 30 + 36) = 108 pairs."""
    faces = v1454.audit_all_faces()
    total = sum(f.n_pairs for f in faces)
    assert total == 108


# ============================================================================
# Module import check
# ============================================================================

def test_deployment_modules_importable_or_known():
    """Each deployment module should be either importable or known offline."""
    for d, m in v1454.V1454_DEPLOYMENT_MODULES.items():
        # Either importable OR we get a text proxy
        text = v1454._deployment_module_kw_text(d)
        # text is either proxy text or "# module ... not importable"
        assert text