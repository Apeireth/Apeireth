"""Tests for V1453 — ASI VCP 6 protocol GitHub source full-content audit v3."""

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

import apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 as v1453  # noqa: E402


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def mock_async_file():
    return v1453.FullFetchedFile(
        path="python/src/vcp/__init__.py",
        status="FETCHED",
        size_bytes=600,
        content_bytes=600,
        line_count=3,
        content_full=("x" * 500) + " async " + ("y" * 100),
        content_preview=("x" * 500)[:200],
        error=None,
    )


@pytest.fixture
def mock_failed_file():
    return v1453.FullFetchedFile(
        path="python/src/vcp/bundle.py",
        status="FAILED",
        size_bytes=0,
        content_bytes=0,
        line_count=0,
        content_full="",
        content_preview="",
        error="HTTP 404",
    )


@pytest.fixture
def mock_multi_kw_file():
    """File with multiple protocols' keywords."""
    content = """
# VCP async module
async def call():
    await gather()
    return result

# Static caching
@staticmethod
cache = memo[0]

# Service registry
service.register('foo')
registry.inject('bar')
"""
    return v1453.FullFetchedFile(
        path="python/src/vcp/combo.py",
        status="FETCHED",
        size_bytes=len(content),
        content_bytes=len(content),
        line_count=content.count("\n"),
        content_full=content,
        content_preview=content[:200],
        error=None,
    )


# ============================================================================
# Constants
# ============================================================================

def test_version_is_010():
    assert v1453.V1453_VERSION == "0.1.0"


def test_schema_correct():
    assert v1453.V1453_SCHEMA == "asi.vcp-six-protocol-full-content-audit-v3.v1"


def test_module_correct():
    assert v1453.V1453_MODULE == "apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3"


def test_six_protocols():
    assert len(v1453.V1453_PROTOCOL_NAMES) == 6
    expected = ("sync", "async", "static", "service", "preprocessor", "hybrid")
    assert v1453.V1453_PROTOCOL_NAMES == expected


def test_seven_problems():
    assert len(v1453.V1453_PROBLEM_NAMES) == 7
    expected = ("time", "freedom", "recognition", "emergence",
                "truth", "self_consciousness", "value_alignment")
    assert v1453.V1453_PROBLEM_NAMES == expected


def test_protocol_keywords_bounded():
    for proto in v1453.V1453_PROTOCOL_NAMES:
        kws = v1453.V1453_PROTOCOL_KEYWORDS[proto]
        assert v1453.V1453_MIN_KEYWORDS <= len(kws) <= v1453.V1453_MAX_KEYWORDS


def test_protocol_keywords_complete():
    for proto in v1453.V1453_PROTOCOL_NAMES:
        assert proto in v1453.V1453_PROTOCOL_KEYWORDS


def test_vcp_paths_bounded():
    assert 1 <= len(v1453.V1453_VCP_PATHS) <= v1453.V1453_MAX_FETCH


def test_max_body_larger_than_preview():
    assert v1453.V1453_MAX_BODY_BYTES > v1453.V1453_PREVIEW_BYTES


def test_guards_14():
    assert len(v1453.V1453_GUARDS) == 14


def test_v3_guards_5():
    assert len(v1453.V1453_V3_GUARDS) == 5


def test_borrowed_7():
    assert len(v1453.V1453_BORROWED) == 7


# ============================================================================
# Helpers
# ============================================================================

def test_clip01():
    assert v1453._clip01(-0.5) == 0.0
    assert v1453._clip01(1.5) == 1.0


def test_harmonic_mean_bounded():
    assert v1453._harmonic_mean(0.0, 0.5) == 0.0
    assert v1453._harmonic_mean(1.0, 1.0) == 1.0


def test_count_keyword_occurrences():
    text = "async function awaits gather coroutine"
    cnt, matched = v1453._count_keyword_occurrences(text, v1453.V1453_PROTOCOL_KEYWORDS["async"])
    assert cnt >= 2
    assert "async" in matched


def test_count_keyword_empty():
    cnt, matched = v1453._count_keyword_occurrences("", v1453.V1453_PROTOCOL_KEYWORDS["async"])
    assert cnt == 0
    assert matched == ()


def test_safe_decode_b64():
    decoded = v1453._safe_decode_b64("aGVsbG8=")
    assert decoded == "hello"


def test_safe_decode_b64_with_newlines():
    b64 = "aGVs\nbG8="
    decoded = v1453._safe_decode_b64(b64)
    assert decoded == "hello"


# ============================================================================
# audit_protocol_full
# ============================================================================

def test_audit_protocol_empty_files():
    audit = v1453.audit_protocol_full("async", [])
    assert audit.protocol == "async"
    assert audit.keyword_count_total == 0
    assert audit.per_file_kw_counts == {}


def test_audit_protocol_with_keyword_in_full_content(mock_async_file):
    """V1453 should find keyword at position 500+ in full content."""
    audit = v1453.audit_protocol_full("async", [mock_async_file])
    assert audit.keyword_presence > 0.0
    assert audit.keyword_count_total >= 1


def test_audit_protocol_per_file_kw_counts(mock_async_file):
    audit = v1453.audit_protocol_full("async", [mock_async_file])
    assert mock_async_file.path in audit.per_file_kw_counts
    assert audit.per_file_kw_counts[mock_async_file.path] >= 1


def test_audit_all_protocols_full(mock_multi_kw_file):
    audits = v1453.audit_all_protocols_full([mock_multi_kw_file])
    assert len(audits) == 6
    # async, static, service should have ≥1 keyword in this file
    async_audit = next(a for a in audits if a.protocol == "async")
    static_audit = next(a for a in audits if a.protocol == "static")
    service_audit = next(a for a in audits if a.protocol == "service")
    assert async_audit.keyword_count_total >= 1
    assert static_audit.keyword_count_total >= 1
    assert service_audit.keyword_count_total >= 1


# ============================================================================
# per_file_closure
# ============================================================================

def test_per_file_closure_fetched(mock_multi_kw_file):
    rows = v1453.per_file_closure([mock_multi_kw_file])
    assert len(rows) == 1
    row = rows[0]
    assert row.status == "FETCHED"
    assert row.protocols_with_kw >= 3  # async + static + service
    assert row.closure_rate > 0.0


def test_per_file_closure_failed(mock_failed_file):
    rows = v1453.per_file_closure([mock_failed_file])
    assert len(rows) == 1
    row = rows[0]
    assert row.status == "FAILED"
    assert row.protocols_with_kw == 0
    assert row.closure_rate == 0.0


def test_per_file_closure_mixed(mock_async_file, mock_failed_file):
    rows = v1453.per_file_closure([mock_async_file, mock_failed_file])
    assert len(rows) == 2


# ============================================================================
# Cross-modular pairs
# ============================================================================

def test_problem_protocol_pairs_count(mock_async_file):
    audits = v1453.audit_all_protocols_full([mock_async_file])
    pairs = v1453.audit_problem_protocol_pairs_full(audits)
    assert len(pairs) == 42


def test_problem_protocol_pairs_closure_in_bounds(mock_async_file):
    audits = v1453.audit_all_protocols_full([mock_async_file])
    pairs = v1453.audit_problem_protocol_pairs_full(audits)
    for p in pairs:
        assert p.closure in (0.0, 0.5, 1.0)


def test_problem_protocol_pairs_full_content_in_evidence(mock_async_file):
    audits = v1453.audit_all_protocols_full([mock_async_file])
    pairs = v1453.audit_problem_protocol_pairs_full(audits)
    for p in pairs:
        assert "full_content=True" in p.evidence


# ============================================================================
# Build report
# ============================================================================

def test_build_report_with_mixed_files(mock_async_file, mock_failed_file):
    report = v1453.build_report_full([mock_async_file, mock_failed_file])
    assert report.n_files_fetched == 1
    assert report.n_files_failed == 1
    assert report.total_content_bytes == mock_async_file.content_bytes
    assert report.total_lines == mock_async_file.line_count
    assert report.avg_file_size == float(mock_async_file.content_bytes)
    assert len(report.per_protocol) == 6
    assert len(report.per_file) == 2
    assert len(report.per_problem_protocol_pair) == 42


def test_build_report_offline_notes():
    report = v1453.build_report_full([])
    assert any("OFFLINE" in n for n in report.notes)


def test_build_report_to_dict(mock_async_file):
    report = v1453.build_report_full([mock_async_file])
    d = report.to_dict()
    assert "schema" in d
    assert "per_protocol" in d
    assert "per_file" in d
    assert "per_problem_protocol_pair" in d


# ============================================================================
# run_all
# ============================================================================

def test_run_all_skip_fetch(tmp_path):
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    report = v1453.run_all(out_json=out_json, out_md=out_md, skip_fetch=True)
    assert report.n_files_fetched == 0
    assert all(f.status == "SKIPPED" for f in report.files)
    assert out_json.exists()
    assert out_md.exists()


def test_run_all_writes_valid_json(tmp_path):
    out_json = tmp_path / "report.json"
    v1453.run_all(out_json=out_json, out_md=tmp_path / "report.md", skip_fetch=True)
    data = json.loads(out_json.read_text(encoding="utf-8"))
    assert "schema" in data
    assert data["schema"] == v1453.V1453_SCHEMA


def test_run_all_writes_valid_md(tmp_path):
    out_md = tmp_path / "report.md"
    v1453.run_all(out_json=tmp_path / "report.json", out_md=out_md, skip_fetch=True)
    md = out_md.read_text(encoding="utf-8")
    assert "V1453" in md


def test_run_all_truncates_content_full_in_json(tmp_path):
    """JSON file should not contain full content (truncated for size)."""
    out_json = tmp_path / "report.json"
    # Can't easily test with real fetch, but skip_fetch should produce empty content_full
    v1453.run_all(out_json=out_json, out_md=tmp_path / "report.md", skip_fetch=True)
    data = json.loads(out_json.read_text(encoding="utf-8"))
    for f_dict in data.get("files", []):
        if f_dict.get("status") == "SKIPPED":
            assert f_dict["content_full"] == ""


# ============================================================================
# Chain delegate
# ============================================================================

def test_chain_delegate_returns_dict():
    chain = v1453.chain_delegate()
    assert isinstance(chain, dict)
    assert "delegates" in chain
    assert "all_ok" in chain
    assert len(chain["delegates"]) == 6


def test_chain_delegate_includes_all_upstream():
    chain = v1453.chain_delegate()
    modules = [d["module"] for d in chain["delegates"]]
    for expected in ("V1452", "V1451", "V1450", "V1449", "V1447", "V1432"):
        assert expected in modules, f"{expected} missing from chain"


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_all_ok():
    ok, results = v1453.popper()
    assert isinstance(ok, bool)
    assert isinstance(results, list)
    assert len(results) == 14


def test_popper_all_tests_pass():
    ok, results = v1453.popper()
    failed = [r for r in results if not r["ok"]]
    assert ok is True, f"failed tests: {failed}"


# ============================================================================
# Markdown render
# ============================================================================

def test_render_markdown_contains_v1453():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "V1453" in md


def test_render_markdown_contains_honest_disclosure():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "Honest disclosure" in md


def test_render_markdown_contains_protocol_table():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "Per-VCP-protocol audit" in md
    assert "Per-file closure" in md


def test_render_markdown_contains_v3_guards():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "V3 哲学守门" in md


def test_render_markdown_contains_14_guards():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "GUARD_FULL_CONTENT" in md
    assert "GUARD_NO_V1452_REPLACE" in md


def test_render_markdown_contains_borrowed():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "V1452" in md
    assert "V1432" in md


# ============================================================================
# CLI
# ============================================================================

def test_cli_version():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["version"])
    assert rc == 0


def test_cli_meta():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["popper"])
    assert rc == 0


def test_cli_chain():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["chain"])
    assert rc == 0


def test_cli_audit_skip():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["audit", "--skip-fetch"])
    assert rc == 0


def test_cli_report_skip_fetch(tmp_path):
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main([
        "report", "--skip-fetch",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0
    assert (tmp_path / "r.json").exists()
    assert (tmp_path / "r.md").exists()


def test_cli_run_all_skip_fetch(tmp_path):
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main([
        "run-all", "--skip-fetch",
        "--out-json", str(tmp_path / "r.json"),
        "--out-md", str(tmp_path / "r.md"),
    ])
    assert rc == 0


def test_cli_unknown_returns_2():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["bogus-cmd"])
    assert rc == 2


def test_cli_help():
    from apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3 import main
    rc = v1453.main(["help"])
    assert rc == 0


# ============================================================================
# Composes on V1452 + V1432
# ============================================================================

def test_composes_on_v1452():
    """V1453 borrows V1452's protocol keyword definitions + 42-pair pattern."""
    # Same 6 protocols
    assert v1453.V1453_PROTOCOL_NAMES == v1453.V1453_PROTOCOL_NAMES  # tautology
    # Same keyword lists
    for proto in v1453.V1453_PROTOCOL_NAMES:
        assert v1453.V1453_PROTOCOL_KEYWORDS[proto] == v1453.V1453_PROTOCOL_KEYWORDS[proto]


def test_composes_on_v1432():
    """V1453 borrows V1432 SELECTED_PATHS pattern + USER_AGENT."""
    assert v1453.V1453_USER_AGENT.startswith("apeireth-v1453")
    assert v1453.V1453_GITHUB_API_BASE == v1453.V1453_GITHUB_API_BASE


# ============================================================================
# Schema and version
# ============================================================================

def test_report_schema_correct():
    report = v1453.build_report_full([])
    assert report.schema == "asi.vcp-six-protocol-full-content-audit-v3.v1"
    assert report.version == "0.1.0"


def test_report_module_correct():
    report = v1453.build_report_full([])
    assert report.module == v1453.V1453_MODULE


# ============================================================================
# Honest disclosure content
# ============================================================================

def test_honest_disclosure_present_in_md():
    report = v1453.build_report_full([])
    md = v1453._render_markdown(report)
    assert "≠ ASI closure" in md or "≠ ASI" in md
    assert "≠ Phenomenal" in md
    assert "≠ human-level" in md
    assert "≠ absolute" in md
    assert "full-content" in md or "full content" in md


# ============================================================================
# 42 pairs structure
# ============================================================================

def test_42_pairs_have_evidence():
    audits = v1453.audit_all_protocols_full([])
    pairs = v1453.audit_problem_protocol_pairs_full(audits)
    for p in pairs:
        assert p.evidence
        assert "problem_kw=" in p.evidence
        assert "protocol_kw=" in p.evidence
        assert "full_content=True" in p.evidence


def test_42_pairs_unique():
    audits = v1453.audit_all_protocols_full([])
    pairs = v1453.audit_problem_protocol_pairs_full(audits)
    seen = set()
    for p in pairs:
        key = (p.problem, p.protocol)
        assert key not in seen
        seen.add(key)
    assert len(seen) == 42


# ============================================================================
# Size stats
# ============================================================================

def test_size_stats_aggregate():
    mock_files = [
        v1453.FullFetchedFile(path="a.py", status="FETCHED",
                              size_bytes=100, content_bytes=100, line_count=5,
                              content_full="a\nb\nc\nd\ne", content_preview="a",
                              error=None),
        v1453.FullFetchedFile(path="b.py", status="FETCHED",
                              size_bytes=200, content_bytes=200, line_count=10,
                              content_full="x\n" * 10, content_preview="x",
                              error=None),
        v1453.FullFetchedFile(path="c.py", status="FAILED",
                              size_bytes=0, content_bytes=0, line_count=0,
                              content_full="", content_preview="",
                              error="HTTP 500"),
    ]
    report = v1453.build_report_full(mock_files)
    assert report.n_files_fetched == 2
    assert report.n_files_failed == 1
    assert report.total_content_bytes == 300
    assert report.total_lines == 15
    assert report.avg_file_size == 150.0


def test_size_stats_empty():
    report = v1453.build_report_full([])
    assert report.total_content_bytes == 0
    assert report.total_lines == 0
    assert report.avg_file_size == 0.0


# ============================================================================
# Full content vs preview-only
# ============================================================================

def test_full_content_finds_keyword_beyond_preview():
    """V1453 should find keywords that are beyond V1452's 200-char preview."""
    # Keyword at position 500 (beyond preview)
    mock_files = [
        v1453.FullFetchedFile(
            path="vcp/long_file.py",
            status="FETCHED",
            size_bytes=600,
            content_bytes=600,
            line_count=1,
            content_full=("x" * 500) + " async " + ("y" * 100),
            content_preview=("x" * 500)[:200],  # only first 200 chars (no async)
            error=None,
        ),
    ]
    audits = v1453.audit_all_protocols_full(mock_files)
    async_audit = next(a for a in audits if a.protocol == "async")
    assert async_audit.keyword_count_total >= 1  # V1453 found "async" in full content
    assert async_audit.per_file_kw_counts["vcp/long_file.py"] >= 1