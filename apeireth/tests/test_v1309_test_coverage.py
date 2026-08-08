"""
V1309 - Test Coverage Real Audit Tests (Post-V1308 Cargo.lock audit)

Popper hypotheses (each test is a falsifiable claim):
1. total_crates_scanned == 91 (workspace 修真后 92 - 1 tauri stub = 91 active)
2. p0_critical_no_tests == [] (所有 critical path 都有 tests)
3. p1_no_tests count == 1 (tauri-stub 是 intentional stub)
4. p1_no_tests[0] == "apeireth-tauri-stub"
5. p3_well_tested count >= 80 (≥80% 覆盖率)
6. integration_test_files total >= 100
7. unit_tests total >= 1000
8. with_examples count >= 70
9. with_benches count >= 10
10. P0_CRITICAL 全部 has at least 1 unit test (P0 修真逻辑自洽)
"""
import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
FINDINGS = ROOT / "apeireth" / "v1309_audit_findings.json"


@pytest.fixture(scope="module")
def audit():
    if not FINDINGS.exists():
        pytest.skip(f"V1309 audit findings not found: {FINDINGS}")
    return json.loads(FINDINGS.read_text(encoding="utf-8"))


def test_h1_total_crates_scanned_is_91(audit):
    assert audit["total_crates_scanned"] == 91, f"got {audit['total_crates_scanned']}"


def test_h2_p0_critical_no_tests_is_empty(audit):
    assert audit["p0_critical_no_tests"] == [], f"critical no-tests: {audit['p0_critical_no_tests']}"


def test_h3_p1_count_is_one(audit):
    assert len(audit["p1_no_tests_non_critical"]) == 1


def test_h4_p1_is_tauri_stub(audit):
    assert audit["p1_no_tests_non_critical"][0] == "apeireth-tauri-stub"


def test_h5_p3_count_at_least_80(audit):
    assert audit["class_counts"]["P3"] >= 80, f"P3 count = {audit['class_counts']['P3']}"


def test_h6_integration_test_files_at_least_100(audit):
    total_int = sum(r["integration_test_files"] for r in audit["audit_rows"])
    assert total_int >= 100, f"integration test files = {total_int}"


def test_h7_unit_tests_at_least_1000(audit):
    total_unit = sum(r["unit_tests"] for r in audit["audit_rows"])
    assert total_unit >= 1000, f"unit tests total = {total_unit}"


def test_h8_with_examples_at_least_70(audit):
    count = sum(1 for r in audit["audit_rows"] if r["has_examples"])
    assert count >= 70, f"with examples = {count}"


def test_h9_with_benches_at_least_10(audit):
    count = sum(1 for r in audit["audit_rows"] if r["has_benches"])
    assert count >= 10, f"with benches = {count}"


def test_h10_p0_critical_all_have_tests(audit):
    """P0 critical crates all have at least 1 unit test."""
    p0_critical = {r["crate"] for r in audit["audit_rows"] if r["is_p0_critical"]}
    for c in p0_critical:
        row = next(r for r in audit["audit_rows"] if r["crate"] == c)
        assert row["unit_tests"] > 0, f"P0 critical {c} has 0 unit tests"


def test_class_counts_sum_to_total(audit):
    cc = audit["class_counts"]
    total = cc["P0"] + cc["P1"] + cc["P2"] + cc["P3"]
    assert total == audit["total_crates_scanned"], f"class sum {total} != total {audit['total_crates_scanned']}"


def test_workspace_is_healthy(audit):
    """Workspace test maturity ratio >= 80%."""
    healthy = audit["class_counts"]["P3"] + audit["class_counts"]["P2"]
    ratio = healthy / audit["total_crates_scanned"]
    assert ratio >= 0.85, f"healthy ratio = {ratio:.3f}"


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-v"]))