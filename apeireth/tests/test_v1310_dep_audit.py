"""
V1310 — Dependency Audit Popper Self-Tests

Tests hypothesis-falsifiable claims about the dep audit:
- workspace root has Apeireth-rust/crates
- 91 crates scanned (V1307/V1309 anchor)
- parse errors = 0 (all Cargo.toml are valid TOML)
- external + workspace dep occurrence counts positive
- intra-workspace graph non-empty
- lock duplicate detection scans Cargo.lock
- findings JSON has all required keys
"""
import json
import sys
from pathlib import Path

# Add apeireth dir to sys.path so we can import v1310_dep_audit
ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1310_dep_audit as audit_mod  # noqa: E402

FINDINGS = ROOT / "apeireth" / "v1310_audit_findings.json"


def _load():
    return json.loads(FINDINGS.read_text(encoding="utf-8"))


def test_h1_workspace_root_has_crates_dir():
    """H1: Apeireth-rust/crates directory exists and contains crates."""
    assert audit_mod.RUST_CRATES.is_dir(), f"crates dir missing: {audit_mod.RUST_CRATES}"
    crates = list(audit_mod.RUST_CRATES.iterdir())
    assert len(crates) >= 80, f"expected 80+ crates, got {len(crates)}"


def test_h2_total_crates_91():
    """H2: workspace has 91 crates (V1307 修真后 anchor)."""
    data = _load()
    assert data["total_crates_scanned"] == 91, f"expected 91, got {data['total_crates_scanned']}"


def test_h3_parse_errors_zero():
    """H3: all 91 Cargo.toml parse via tomllib without errors."""
    data = _load()
    assert data["parse_errors"] == [], f"unexpected parse errors: {data['parse_errors']}"


def test_h4_external_dep_count_positive():
    """H4: external dep occurrences > 0 (workspace actually has external deps)."""
    data = _load()
    assert data["total_external_dep_occurrences"] > 500, (
        f"external dep count {data['total_external_dep_occurrences']} too low"
    )


def test_h5_workspace_path_deps_positive():
    """H5: workspace path-deps > 0 (intra-workspace graph exists)."""
    data = _load()
    assert data["total_workspace_dep_occurrences"] > 100, (
        f"workspace path-deps {data['total_workspace_dep_occurrences']} too low"
    )


def test_h6_intra_graph_edges_positive():
    """H6: intra-workspace graph has edges > 0."""
    data = _load()
    assert data["intra_workspace_graph_edges"] > 100, (
        f"intra-ws graph edges {data['intra_workspace_graph_edges']} too low"
    )


def test_h7_high_fan_in_deps_includes_serde():
    """H7: serde appears in >= 50 crates (high fan-in baseline)."""
    data = _load()
    deps = {d["dep"]: d["crate_count"] for d in data["high_fan_in_deps"]}
    assert deps.get("serde", 0) >= 50, f"serde fan-in: {deps.get('serde')}"


def test_h8_version_drifts_under_20():
    """H8: workspace version drifts <= 20 (drift = bounded, not out-of-control)."""
    data = _load()
    n = data["version_drift_count"]
    assert n <= 20, f"version drifts {n} > 20 (workspace drift out of control)"


def test_h9_lock_duplicate_count_reasonable():
    """H9: Cargo.lock duplicates include transitive deps but bounded (<200)."""
    data = _load()
    n = data["lock_duplicate_count"]
    assert 50 <= n <= 200, f"lock duplicate count {n} outside [50, 200]"


def test_h10_audit_decision_is_valid():
    """H10: audit decision in {HEALTHY, REVIEW}."""
    data = _load()
    assert data["audit_decision"] in {"HEALTHY", "REVIEW"}, (
        f"unexpected decision: {data['audit_decision']}"
    )


def test_h11_findings_json_has_all_keys():
    """H11: findings JSON has all required keys."""
    data = _load()
    required = [
        "workspace_root", "crates_root", "total_crates_scanned",
        "parse_errors", "workspace_dependencies_count",
        "total_external_dep_occurrences", "total_workspace_dep_occurrences",
        "intra_workspace_graph", "intra_workspace_graph_edges",
        "version_drift_count", "version_drifts",
        "lock_duplicate_count", "lock_duplicates",
        "high_fan_in_deps", "bare_version_count", "bare_versions_sample",
        "audit_decision", "audit_reason", "audit_action",
    ]
    for k in required:
        assert k in data, f"missing key: {k}"


def test_h12_audit_reason_references_v1310():
    """H12: audit_reason is a non-empty string mentioning V1310."""
    data = _load()
    assert "V1310" in data["audit_reason"], "audit_reason missing V1310 reference"
    assert len(data["audit_reason"]) > 30, "audit_reason too short"


def test_h13_no_panic_on_re_audit():
    """H13: re-running v1310_dep_audit.py doesn't crash."""
    import subprocess
    proc = subprocess.run(
        [sys.executable, str(ROOT / "apeireth" / "v1310_dep_audit.py")],
        capture_output=True, text=True, cwd=str(ROOT),
    )
    assert proc.returncode == 0, f"audit crashed: {proc.stderr[:500]}"


def test_h14_drift_list_excludes_intra_ws():
    """H14: version drifts do NOT include apeireth-* (intra-workspace is by design)."""
    data = _load()
    for d in data["version_drifts"]:
        assert not d["dep"].startswith("apeireth-"), (
            f"intra-workspace dep {d['dep']} incorrectly flagged as drift"
        )


def test_h15_workspace_dependencies_parsed():
    """H15: workspace.dependencies parsed > 10 entries (real workspace deps)."""
    data = _load()
    assert data["workspace_dependencies_count"] >= 10, (
        f"workspace deps {data['workspace_dependencies_count']} too low"
    )


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))