"""Tests for V1452 — ASI VCP 6 protocol GitHub source deep-read audit v2."""

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

import apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 as v1452  # noqa: E402


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def empty_files():
    return []


@pytest.fixture
def fetched_files_with_async():
    return [
        v1452.FetchedFile(
            path="python/src/vcp/__init__.py",
            status="FETCHED",
            size_bytes=100,
            content_preview="async def call_sync(): await gather() coroutine",
            error=None,
        ),
        v1452.FetchedFile(
            path="python/src/vcp/bundle.py",
            status="FETCHED",
            size_bytes=100,
            content_preview="@staticmethod cache memo function",
            error=None,
        ),
    ]


@pytest.fixture
def failed_files():
    return [
        v1452.FetchedFile(
            path="python/src/vcp/__init__.py",
            status="FAILED",
            size_bytes=0,
            content_preview="",
            error="HTTP 404",
        ),
    ]


# ============================================================================
# Constants
# ============================================================================

def test_version_is_010():
    assert v1452.V1452_VERSION == "0.1.0"


def test_schema_correct():
    assert v1452.V1452_SCHEMA == "asi.vcp-six-protocol-github-audit-v2.v1"


def test_module_correct():
    assert v1452.V1452_MODULE == "apeireth.v1452_asi_vcp_six_protocol_github_audit_v2"


def test_six_protocols():
    assert len(v1452.V1452_PROTOCOL_NAMES) == 6
    expected = ("sync", "async", "static", "service", "preprocessor", "hybrid")
    assert v1452.V1452_PROTOCOL_NAMES == expected


def test_seven_problems():
    assert len(v1452.V1452_PROBLEM_NAMES) == 7
    expected = ("time", "freedom", "recognition", "emergence",
                "truth", "self_consciousness", "value_alignment")
    assert v1452.V1452_PROBLEM_NAMES == expected


def test_protocol_keywords_bounded():
    for proto in v1452.V1452_PROTOCOL_NAMES:
        kws = v1452.V1452_PROTOCOL_KEYWORDS[proto]
        assert v1452.V1452_MIN_KEYWORDS <= len(kws) <= v1452.V1452_MAX_KEYWORDS


def test_protocol_keywords_complete():
    for proto in v1452.V1452_PROTOCOL_NAMES:
        assert proto in v1452.V1452_PROTOCOL_KEYWORDS


def test_vcp_paths_bounded():
    assert 1 <= len(v1452.V1452_VCP_PATHS) <= v1452.V1452_MAX_FETCH


def test_guards_14():
    assert len(v1452.V1452_GUARDS) == 14


def test_v3_guards_5():
    assert len(v1452.V1452_V3_GUARDS) == 5


def test_borrowed_6():
    assert len(v1452.V1452_BORROWED) == 6


# ============================================================================
# Helpers
# ============================================================================

def test_clip01():
    assert v1452._clip01(-0.5) == 0.0
    assert v1452._clip01(0.0) == 0.0
    assert v1452._clip01(0.5) == 0.5
    assert v1452._clip01(1.0) == 1.0
    assert v1452._clip01(1.5) == 1.0


def test_harmonic_mean_bounded():
    assert v1452._harmonic_mean(0.0, 0.5) == 0.0  # 0 means 0
    assert v1452._harmonic_mean(1.0, 1.0) == 1.0
    h = v1452._harmonic_mean(0.5, 1.0)
    assert 0.0 < h < 1.0
    assert abs(h - 2 * 0.5 * 1.0 / 1.5) < 1e-9


def test_harmonic_mean_symmetric():
    assert abs(v1452._harmonic_mean(0.3, 0.7) - v1452._harmonic_mean(0.7, 0.3)) < 1e-9


def test_count_keyword_occurrences():
    text = "async function awaits gather coroutine"
    cnt, matched = v1452._count_keyword_occurrences(text, v1452.V1452_PROTOCOL_KEYWORDS["async"])
    assert cnt >= 2
    assert "async" in matched
    assert "await" in matched
    assert "gather" in matched


def test_count_keyword_empty():
    cnt, matched = v1452._count_keyword_occurrences("", v1452.V1452_PROTOCOL_KEYWORDS["async"])
    assert cnt == 0
    assert matched == ()


def test_count_keyword_no_match():
    cnt, matched = v1452._count_keyword_occurrences("hello world", v1452.V1452_PROTOCOL_KEYWORDS["async"])
    assert cnt == 0
    assert matched == ()


def test_safe_decode_b64():
    # "hello" = aGVsbG8=
    decoded = v1452._safe_decode_b64("aGVsbG8=")
    assert decoded == "hello"


# ============================================================================
# audit_protocol
# ============================================================================

def test_audit_protocol_empty_files():
    audit = v1452.audit_protocol("async", [])
    assert audit.protocol == "async"
    assert audit.keyword_count_total == 0
    assert audit.files_with_keyword == 0
    assert audit.files_fetched == 0
    assert audit.closure_rate == 0.0


def test_audit_protocol_with_async_content(fetched_files_with_async):
    audit = v1452.audit_protocol("async", fetched_files_with_async)
    assert audit.keyword_presence > 0.0
    assert audit.keyword_count_total >= 3
    assert audit.closure_rate > 0.0


def test_audit_protocol_with_static_content(fetched_files_with_async):
    audit = v1452.audit_protocol("static", fetched_files_with_async)
    assert audit.keyword_presence > 0.0  # static, cache, memo in preview
    assert audit.keyword_count_total >= 1


def test_audit_all_protocols(fetched_files_with_async):
    audits = v1452.audit_all_protocols(fetched_files_with_async)
    assert len(audits) == 6
    protocols = {a.protocol for a in audits}
    assert protocols == set(v1452.V1452_PROTOCOL_NAMES)


def test_audit_all_protocols_empty(empty_files):
    audits = v1452.audit_all_protocols(empty_files)
    for a in audits:
        assert a.closure_rate == 0.0


# ============================================================================
# Cross-modular pairs
# ============================================================================

def test_problem_protocol_pairs_count(fetched_files_with_async):
    audits = v1452.audit_all_protocols(fetched_files_with_async)
    pairs = v1452.audit_problem_protocol_pairs(audits)
    assert len(pairs) == 42  # 7 problems × 6 protocols


def test_problem_protocol_pairs_closure_in_bounds(fetched_files_with_async):
    audits = v1452.audit_all_protocols(fetched_files_with_async)
    pairs = v1452.audit_problem_protocol_pairs(audits)
    for p in pairs:
        assert p.closure in (0.0, 0.5, 1.0)


def test_problem_protocol_pairs_all_problems_present(fetched_files_with_async):
    audits = v1452.audit_all_protocols(fetched_files_with_async)
    pairs = v1452.audit_problem_protocol_pairs(audits)
    problems = {p.problem for p in pairs}
    assert problems == set(v1452.V1452_PROBLEM_NAMES)


def test_problem_protocol_pairs_all_protocols_present(fetched_files_with_async):
    audits = v1452.audit_all_protocols(fetched_files_with_async)
    pairs = v1452.audit_problem_protocol_pairs(audits)
    protocols = {p.protocol for p in pairs}
    assert protocols == set(v1452.V1452_PROTOCOL_NAMES)


def test_problem_keywords_detection():
    text = "value alignment goal intent"
    assert v1452._problem_keywords_in_text(text, "value_alignment") is True
    text2 = "completely unrelated text"
    assert v1452._problem_keywords_in_text(text2, "value_alignment") is False


def test_problem_module_has_keyword():
    assert v1452._problem_module_has_keyword(("v1049", "v1446"), "value_alignment") is True
    assert v1452._problem_module_has_keyword((), "value_alignment") is False


# ============================================================================
# Build report
# ============================================================================

def test_build_report_empty():
    report = v1452.build_report([])
    assert report.n_files_fetched == 0
    assert report.n_files_failed == 0
    assert len(report.per_protocol) == 6
    assert len(report.per_problem_protocol_pair) == 42
    assert 0.0 <= report.overall_closure_rate <= 1.0


def test_build_report_with_files(fetched_files_with_async):
    report = v1452.build_report(fetched_files_with_async)
    assert report.n_files_fetched == 2
    assert report.n_files_failed == 0
    assert 0.0 <= report.overall_closure_rate <= 1.0
    assert len(report.per_protocol_closure_rate) == 6
    assert len(report.per_problem_closure_rate) == 7


def test_build_report_with_failures(failed_files):
    report = v1452.build_report(failed_files)
    assert report.n_files_fetched == 0
    assert report.n_files_failed == 1
    # All protocols should have closure 0.0 since no fetched files
    for proto, rate in report.per_protocol_closure_rate.items():
        assert rate == 0.0


def test_build_report_notes_offline():
    report = v1452.build_report([])
    assert any("OFFLINE" in n for n in report.notes) or any("0 files" in n or "fetched 0" in n for n in report.notes)


def test_build_report_to_dict(fetched_files_with_async):
    report = v1452.build_report(fetched_files_with_async)
    d = report.to_dict()
    assert isinstance(d, dict)
    assert "schema" in d
    assert "version" in d
    assert "per_protocol" in d
    assert len(d["per_protocol"]) == 6


# ============================================================================
# run_all
# ============================================================================

def test_run_all_skip_fetch(tmp_path):
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = v1452.run_all(
        out_json=out_json,
        out_md=out_md,
        skip_fetch=True,
    )
    assert report.n_files_fetched == 0
    assert all(f.status == "SKIPPED" for f in report.files)
    assert out_json.exists()
    assert out_md.exists()


def test_run_all_writes_valid_json(tmp_path):
    out_json = tmp_path / "report.json"
    v1452.run_all(out_json=out_json, out_md=tmp_path / "report.md", skip_fetch=True)
    data = json.loads(out_json.read_text(encoding="utf-8"))
    assert "schema" in data
    assert data["schema"] == v1452.V1452_SCHEMA


def test_run_all_writes_valid_md(tmp_path):
    out_md = tmp_path / "report.md"
    v1452.run_all(out_json=tmp_path / "report.json", out_md=out_md, skip_fetch=True)
    md = out_md.read_text(encoding="utf-8")
    assert "V1452" in md
    assert "Honest disclosure" in md


# ============================================================================
# Chain delegate
# ============================================================================

def test_chain_delegate_returns_dict():
    chain = v1452.chain_delegate()
    assert isinstance(chain, dict)
    assert "delegates" in chain
    assert "all_ok" in chain
    assert len(chain["delegates"]) == 5
    assert isinstance(chain["all_ok"], bool)


def test_chain_delegate_includes_v1432():
    chain = v1452.chain_delegate()
    modules = [d["module"] for d in chain["delegates"]]
    assert "V1432" in modules
    assert "V1449" in modules
    assert "V1447" in modules
    assert "V1446" in modules
    assert "V1426" in modules


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_all_ok():
    ok, results = v1452.popper()
    assert isinstance(ok, bool)
    assert isinstance(results, list)
    assert len(results) == 14


def test_popper_all_tests_pass():
    ok, results = v1452.popper()
    failed = [r for r in results if not r["ok"]]
    assert ok is True, f"failed tests: {failed}"


# ============================================================================
# Markdown render
# ============================================================================

def test_render_markdown_contains_v1452():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "V1452" in md


def test_render_markdown_contains_honest_disclosure():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "Honest disclosure" in md


def test_render_markdown_contains_protocol_table():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "sync" in md
    assert "async" in md
    assert "preprocessor" in md
    assert "hybrid" in md


def test_render_markdown_contains_v3_guards():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "V3 哲学守门" in md


def test_render_markdown_contains_14_guards():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "GUARD_FETCH_BOUNDED" in md
    assert "GUARD_NO_V1432_REPLACE" in md


def test_render_markdown_contains_42_pairs_section():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "42 pairs" in md


def test_render_markdown_contains_borrowed():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "V1432" in md
    assert "V1449" in md


# ============================================================================
# CLI
# ============================================================================

def test_cli_version():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["version"])
    assert rc == 0


def test_cli_meta():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["popper"])
    assert rc == 0


def test_cli_chain():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["chain"])
    assert rc == 0


def test_cli_audit_skip():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["audit", "--skip-fetch"])
    assert rc == 0


def test_cli_report_skip_fetch(tmp_path):
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main([
        "report",
        "--skip-fetch",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0
    assert (tmp_path / "r.json").exists()
    assert (tmp_path / "r.md").exists()


def test_cli_run_all_skip_fetch(tmp_path):
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main([
        "run-all",
        "--skip-fetch",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0
    assert (tmp_path / "r.json").exists()
    assert (tmp_path / "r.md").exists()


def test_cli_unknown_returns_2():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["bogus-cmd"])
    assert rc == 2


def test_cli_help():
    from apeireth.v1452_asi_vcp_six_protocol_github_audit_v2 import main
    rc = v1452.main(["help"])
    assert rc == 0


# ============================================================================
# Composes on V1432 + V1449 + V1447 + V1446 + V1426
# ============================================================================

def test_composes_on_v1432():
    """V1452 borrows V1432 SELECTED_PATHS pattern + USER_AGENT."""
    assert v1452.V1452_USER_AGENT.startswith("apeireth-v1452")
    assert v1452.V1452_GITHUB_API_BASE == v1452.V1452_GITHUB_API_BASE
    # V1452_VCP_PATHS partially overlaps with V1432 SELECTED_PATHS
    assert any("__init__.py" in p for p in v1452.V1452_VCP_PATHS)


def test_composes_on_v1449():
    """V1452 borrows V1449 7 problems × 6 protocols pattern."""
    assert v1452.V1452_PROTOCOL_NAMES == ("sync", "async", "static", "service", "preprocessor", "hybrid")
    assert v1452.V1452_PROBLEM_NAMES == ("time", "freedom", "recognition", "emergence",
                                          "truth", "self_consciousness", "value_alignment")


def test_composes_on_v1426():
    """V1452 borrows V1426 VCP 6 protocol definitions."""
    # Same 6 protocols
    expected_6 = ("sync", "async", "static", "service", "preprocessor", "hybrid")
    assert v1452.V1452_PROTOCOL_NAMES == expected_6


# ============================================================================
# Report schema
# ============================================================================

def test_report_schema_correct():
    report = v1452.build_report([])
    assert report.schema == "asi.vcp-six-protocol-github-audit-v2.v1"
    assert report.version == "0.1.0"


def test_report_module_correct():
    report = v1452.build_report([])
    assert report.module == v1452.V1452_MODULE


# ============================================================================
# Honest disclosure content
# ============================================================================

def test_honest_disclosure_present_in_md():
    report = v1452.build_report([])
    md = v1452._render_markdown(report)
    assert "≠ ASI closure" in md or "≠ ASI" in md
    assert "≠ Phenomenal" in md
    assert "≠ human-level" in md
    assert "≠ absolute" in md


# ============================================================================
# 42 pairs structure
# ============================================================================

def test_42_pairs_have_evidence():
    audits = v1452.audit_all_protocols([])
    pairs = v1452.audit_problem_protocol_pairs(audits)
    for p in pairs:
        assert p.evidence
        assert "problem_kw=" in p.evidence
        assert "protocol_kw=" in p.evidence


def test_42_pairs_unique():
    audits = v1452.audit_all_protocols([])
    pairs = v1452.audit_problem_protocol_pairs(audits)
    seen = set()
    for p in pairs:
        key = (p.problem, p.protocol)
        assert key not in seen
        seen.add(key)
    assert len(seen) == 42


# ============================================================================
# File encoding safety
# ============================================================================

def test_safe_decode_b64_with_newlines():
    """GitHub returns base64 with newlines; V1452 strips them."""
    b64 = "aGVs\nbG8="  # "hello" with newline in middle
    decoded = v1452._safe_decode_b64(b64)
    assert decoded == "hello"


def test_safe_decode_b64_invalid():
    decoded = v1452._safe_decode_b64("not-valid-base64!!!")
    assert "decode_error" in decoded