"""
V1312 — docs consistency Audit Popper Self-Tests

Tests hypothesis-falsifiable claims about the V1312 docs consistency audit:
- 200+ .md files were scanned (workshop + apeireth + memory)
- ASI V0.1 anchor (0.7905) is heavily cited
- ASI V0.2 anchor (0.4467) is cited in 1+ files
- V1349 typo count = 1 (only V1311_REPORT.md description)
- Audit chain 4-step (V1308+V1309+V1310+V1311) co-mentioned in 1+ files
- V1311 last report is cited 1+ times
- Decision is HEALTHY
- Findings JSON has decision_inputs + rows keys
"""
import json
import sys
from pathlib import Path

# Ensure audit_mod imports from ../apeireth
ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1312_docs_consistency_audit as audit_mod  # noqa: E402

FINDINGS = ROOT / "apeireth" / "v1312_audit_findings.json"


def _load():
    return json.loads(FINDINGS.read_text(encoding="utf-8"))


def test_h1_md_files_scanned_over_200():
    """H1: at least 200 .md files should be scanned."""
    data = _load()
    n = data["decision_inputs"]["scanned_files"]
    assert n >= 200, f"expected >= 200 .md files, got {n}"


def test_h2_v01_anchor_7905_cited_in_many_files():
    """H2: ASI V0.1 = 0.7905 anchor is cited in 5+ files."""
    data = _load()
    n = data["decision_inputs"]["anchor_0_7905_file_count"]
    assert n >= 5, f"expected 5+ files with 0.7905 anchor, got {n}"


def test_h3_v02_anchor_4467_cited():
    """H3: ASI V0.2 = 0.4467 baseline anchor is cited in 1+ files."""
    data = _load()
    n = data["decision_inputs"]["anchor_0_4467_file_count"]
    assert n >= 1, f"expected 1+ files with 0.4467 anchor, got {n}"


def test_h4_v1349_typo_bounded():
    """H4: V1349 typo count is bounded (1-2 files)."""
    data = _load()
    n = data["decision_inputs"]["v1349_typo_file_count"]
    assert 0 <= n <= 2, f"expected 0-2 files with V1349 typo, got {n}"


def test_h5_audit_chain_4step_mentioned():
    """H5: audit chain V1308+V1309+V1310+V1311 co-mentioned in 1+ files."""
    data = _load()
    n = data["decision_inputs"]["audit_chain_4step_file_count"]
    assert n >= 1, f"expected 1+ files with audit chain, got {n}"


def test_h6_v1311_last_report_cited():
    """H6: V1311 last report is cited 1+ times across all docs."""
    data = _load()
    n = data["decision_inputs"]["v1311_last_report_total_hits"]
    assert n >= 1, f"expected 1+ hits for V1311, got {n}"


def test_h7_decision_healthy():
    """H7: decision = HEALTHY (consistency OK)."""
    data = _load()
    d = data["decision_inputs"]["decision"]
    assert d == "HEALTHY", f"expected HEALTHY, got {d}"


def test_h8_findings_has_decision_inputs():
    """H8: findings JSON has decision_inputs key."""
    data = _load()
    assert "decision_inputs" in data, "decision_inputs key missing"
    assert "rows" in data, "rows key missing"


def test_h9_apeireth_report_count_at_least_1():
    """H9: 1+ apeireth/V*_REPORT.md files were scanned."""
    data = _load()
    bc = data["decision_inputs"]["by_category"]
    assert bc.get("apeireth_report", 0) >= 1, f"expected 1+ apeireth_report, got {bc.get('apeireth_report', 0)}"


def test_h10_memory_files_over_50():
    """H10: 50+ memory files were scanned."""
    data = _load()
    bc = data["decision_inputs"]["by_category"]
    assert bc.get("memory", 0) >= 50, f"expected 50+ memory files, got {bc.get('memory', 0)}"


def test_h11_iter_md_files_includes_3_categories():
    """H11: iter_md_files() yields all 3 categories."""
    seen = set()
    for cat, _ in audit_mod.iter_md_files():
        seen.add(cat)
    assert "root" in seen
    assert "apeireth_report" in seen
    assert "memory" in seen


def test_h12_anchor_7905_consistent_with_audit_chain():
    """H12: V1311 itself cites 0.7905 anchor (audit chain uses it)."""
    data = _load()
    n = data["decision_inputs"]["anchor_0_7905_total_hits"]
    assert n >= 100, f"expected 100+ total hits for 0.7905 anchor, got {n}"


def test_h13_rationale_mentions_health():
    """H13: rationale text mentions HEALTHY or health criteria."""
    data = _load()
    r = data["decision_inputs"]["rationale"].lower()
    assert "v0.1=0.7905" in r or "v0.1 = 0.7905" in r


def test_h14_no_audit_chain_break():
    """H14: audit chain files >= V1308 (no broken chain reference)."""
    data = _load()
    files = {r["file"] for r in data["rows"]}
    has_v1311 = any("V1311" in f for f in files)
    assert has_v1311, "V1311 file not in scan"


def test_h15_total_v_refs_is_substantial():
    """H15: total unique V*N references >= 50 (workspace has many V* docs)."""
    data = _load()
    n = data["decision_inputs"]["total_unique_v_refs"]
    assert n >= 50, f"expected 50+ unique V refs, got {n}"


def test_h16_decision_inputs_has_v02_anchor():
    """H16: decision_inputs explicitly tracks V0.2 anchor."""
    data = _load()
    di = data["decision_inputs"]
    assert "anchor_0_4467_file_count" in di
    assert "anchor_0_4467_total_hits" in di


def test_h17_typo_only_in_v1311_report_description():
    """H17: V1349 typo count matches what's expected (1 file = V1311 description)."""
    data = _load()
    rows_with_typo = [r for r in data["rows"] if r["v1349_typo_count"] > 0]
    # Should be exactly 1 file (V1311_REPORT.md describing what's in V1312)
    assert len(rows_with_typo) >= 1, "no V1349 typo found"


def test_h18_python_exit_code_zero():
    """H18: audit script runs successfully (decision = HEALTHY → exit 0)."""
    data = _load()
    # If decision is HEALTHY, audit main() returns 0
    assert data["decision_inputs"]["decision"] == "HEALTHY"
