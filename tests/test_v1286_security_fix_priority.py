"""V1286 — Security Fix Priority Queue 真生产测试套件

> 47 tests for v1286 (extends V1285):
> - Version / constants: 5 tests
> - Score computation: 8 tests
> - Severity helpers: 4 tests
> - Philosophy gate: 4 tests
> - Data structures: 4 tests
> - Runner: 5 tests
> - Output (Markdown / JSON): 8 tests
> - CLI: 6 tests
> - Integration with V1285: 3 tests
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1286_security_fix_priority import (
    V1286_VERSION,
    V1286_BUILD,
    V1286_SEVERITY_WEIGHTS,
    V1286_GOVERNANCE_CRATES,
    V1286_GOVERNANCE_BONUS,
    V1286_P0_THRESHOLD,
    V1286_P1_THRESHOLD,
    V1286_HAS_UNSAFE_BONUS,
    CratePriorityScore,
    FixPriorityLedger,
    compute_crate_score,
    run_fix_priority,
    _severity_of_pattern,
    _v1286_philosophy_gate,
    _to_markdown,
    _to_json_snapshot,
    main,
)
from apeireth.v1284_worst5_security_audit import (
    CrateSecurityMetrics,
    SecurityFinding,
)


# ============================================================
# 1. Version / constants
# ============================================================

class TestVersion:
    def test_version_is_string(self):
        assert isinstance(V1286_VERSION, str) and len(V1286_VERSION) > 0

    def test_build_is_string(self):
        assert isinstance(V1286_BUILD, str) and "2026-08-05" in V1286_BUILD


class TestConstants:
    def test_severity_weights_complete(self):
        assert V1286_SEVERITY_WEIGHTS == {"critical": 10, "important": 5, "info": 1}

    def test_governance_crates_count(self):
        assert len(V1286_GOVERNANCE_CRATES) == 5
        assert "apeireth-sovereignty" in V1286_GOVERNANCE_CRATES

    def test_governance_bonus_positive(self):
        assert V1286_GOVERNANCE_BONUS > 0
        assert V1286_GOVERNANCE_BONUS == 20

    def test_p0_threshold(self):
        assert V1286_P0_THRESHOLD == 100
        assert V1286_P1_THRESHOLD == 50
        assert V1286_P0_THRESHOLD > V1286_P1_THRESHOLD

    def test_unsafe_bonus(self):
        assert V1286_HAS_UNSAFE_BONUS == 50


# ============================================================
# 2. Severity helpers
# ============================================================

class TestSeverityHelper:
    def test_unwrap_critical(self):
        assert _severity_of_pattern("unwrap_call") == "critical"

    def test_panic_critical(self):
        assert _severity_of_pattern("panic_macro") == "critical"

    def test_expect_important(self):
        assert _severity_of_pattern("expect_call") == "important"

    def test_unknown_info(self):
        assert _severity_of_pattern("unknown_pattern") == "info"


# ============================================================
# 3. Score computation
# ============================================================

class TestComputeScore:
    def test_clean_crate_ok(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-clean",
            crate_src="/tmp/clean",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        score = compute_crate_score(m)
        assert score.priority == "OK"
        assert score.total_score == 0
        assert score.n_critical == 0

    def test_critical_only(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-c",
            crate_src="/tmp/c",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        # Add 10 unwrap findings
        for i in range(10):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-c",
                pattern_id="unwrap_call",
                file_path="/tmp/c/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 10
        score = compute_crate_score(m)
        # base = 10 * 10 = 100, no governance bonus
        assert score.base_score == 100
        assert score.total_score == 100
        assert score.priority == "P0"
        assert score.n_critical == 10

    def test_governance_bonus(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-sovereignty",  # governance
            crate_src="/tmp/s",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        for i in range(3):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-sovereignty",
                pattern_id="unwrap_call",
                file_path="/tmp/s/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 3
        score = compute_crate_score(m)
        # base = 30, governance = 20, total = 50 → P1
        assert score.base_score == 30
        assert score.governance_bonus == 20
        assert score.total_score == 50
        assert score.priority == "P1"

    def test_unsafe_bonus_p0_direct(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-unsafe",
            crate_src="/tmp/u",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        # 1 unsafe finding
        m.findings.append(SecurityFinding(
            crate_name="apeireth-unsafe",
            pattern_id="unsafe_block",
            file_path="/tmp/u/lib.rs",
            line_number=10,
            line_text="unsafe { ... }",
            severity="critical",
        ))
        m.n_unsafe = 1
        for i in range(2):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-unsafe",
                pattern_id="unwrap_call",
                file_path="/tmp/u/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 2
        score = compute_crate_score(m)
        # base = 3*10 (1 unsafe + 2 unwrap are all critical) + 50 (unsafe bonus) = 80
        assert score.unsafe_bonus == 50
        assert score.total_score == 80
        # n_unsafe > 0 → P0
        assert score.priority == "P0"

    def test_mixed_severity(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-mix",
            crate_src="/tmp/m",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        for i in range(5):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-mix",
                pattern_id="unwrap_call",
                file_path="/tmp/m/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 5
        for i in range(3):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-mix",
                pattern_id="expect_call",
                file_path="/tmp/m/lib.rs",
                line_number=10 + i,
                line_text=f"line {10 + i}",
                severity="important",
            ))
        m.n_expect = 3
        score = compute_crate_score(m)
        # base = 5*10 + 3*5 = 65 → P1
        assert score.base_score == 65
        assert score.n_critical == 5
        assert score.n_important == 3
        assert score.priority == "P1"

    def test_p2_small_score(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-p2",
            crate_src="/tmp/p2",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        for i in range(3):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-p2",
                pattern_id="unwrap_call",
                file_path="/tmp/p2/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 3
        score = compute_crate_score(m)
        # 30 → P2
        assert score.total_score == 30
        assert score.priority == "P2"

    def test_top_findings_capped_at_5(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-many",
            crate_src="/tmp/many",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        # Add 20 findings
        for i in range(20):
            m.findings.append(SecurityFinding(
                crate_name="apeireth-many",
                pattern_id="unwrap_call",
                file_path=f"/tmp/many/lib.rs",
                line_number=i,
                line_text=f"line {i}",
                severity="critical",
            ))
        m.n_unwrap = 20
        score = compute_crate_score(m)
        assert len(score.top_findings) == 5  # capped at 5

    def test_zero_severity_returns_ok(self):
        m = CrateSecurityMetrics(
            crate_name="apeireth-zero",
            crate_src="/tmp/z",
            src_files_scanned=1,
            src_lines_scanned=100,
        )
        score = compute_crate_score(m)
        assert score.priority == "OK"


# ============================================================
# 4. Philosophy gate
# ============================================================

class TestPhilosophyGate:
    def test_gate_has_30_entries(self):
        gate = _v1286_philosophy_gate()
        assert len(gate) == 30  # V1285 27 + V1286 3

    def test_gate_has_v1286_new(self):
        gate = _v1286_philosophy_gate()
        assert "v1286_extends_v1285_not_replaces" in gate
        assert "v1286_priority_only_no_fix" in gate
        assert "v1286_governance_weight_advisory" in gate

    def test_all_gates_true(self):
        gate = _v1286_philosophy_gate()
        for k, v in gate.items():
            assert v is True, f"Gate {k} should be True"


# ============================================================
# 5. Data structures
# ============================================================

class TestLedger:
    def test_ledger_default(self):
        ledger = FixPriorityLedger()
        assert ledger.n_p0 == 0
        assert ledger.n_p1 == 0
        assert ledger.n_p2 == 0
        assert ledger.n_ok == 0
        assert not ledger.has_p0
        assert not ledger.has_unsafe

    def test_score_to_dict(self):
        s = CratePriorityScore(crate_name="x", n_critical=5, total_score=50, priority="P1")
        d = s.to_dict()
        assert d["crate_name"] == "x"
        assert d["n_critical"] == 5
        assert d["total_score"] == 50
        assert d["priority"] == "P1"

    def test_ledger_with_p0(self):
        ledger = FixPriorityLedger()
        ledger.scores.append(CratePriorityScore(crate_name="p0_1", priority="P0"))
        ledger.scores.append(CratePriorityScore(crate_name="p0_2", priority="P0"))
        ledger.n_p0 = 2
        assert ledger.has_p0

    def test_ledger_with_unsafe(self):
        ledger = FixPriorityLedger()
        ledger.scores.append(CratePriorityScore(crate_name="u", priority="P0", n_unsafe=1))
        assert ledger.has_unsafe


# ============================================================
# 6. Runner
# ============================================================

class TestRunner:
    def test_run_returns_ledger(self):
        ledger = run_fix_priority()
        assert isinstance(ledger, FixPriorityLedger)
        assert ledger.n_crates_scored >= 42

    def test_run_sums_match(self):
        ledger = run_fix_priority()
        assert ledger.n_p0 + ledger.n_p1 + ledger.n_p2 + ledger.n_ok == ledger.n_crates_scored

    def test_run_has_p0_crates(self):
        """V1285 found 1173 hotspots → at least some P0."""
        ledger = run_fix_priority()
        assert ledger.n_p0 > 0  # apeireth-memory alone has 122 unwrap = 1220 score

    def test_run_has_clean_crates(self):
        """V1285 had 6 clean crates → V1286 should have ≥6 OK."""
        ledger = run_fix_priority()
        assert ledger.n_ok >= 6

    def test_run_unsafe_apeireth_web_p0(self):
        """V1285 found 1 unsafe in apeireth-web → V1286 must mark P0."""
        ledger = run_fix_priority()
        web_score = next((s for s in ledger.scores if s.crate_name == "apeireth-web"), None)
        assert web_score is not None
        assert web_score.n_unsafe > 0
        assert web_score.priority == "P0"


# ============================================================
# 7. Output (Markdown / JSON)
# ============================================================

class TestMarkdown:
    def test_markdown_contains_header(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "V1286 Security Fix Priority Queue" in md

    def test_markdown_contains_priority_sections(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "P0 (立即修)" in md
        assert "P1 (本 sprint)" in md
        assert "P2 (本季度)" in md
        assert "OK (clean, 无需修)" in md

    def test_markdown_contains_scoring_formula(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "Scoring Formula" in md
        assert "severity_weight" in md
        assert "P0" in md and "100" in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "V3 Philosophy Gate" in md
        assert "v1286_extends_v1285_not_replaces" in md

    def test_markdown_contains_disclaimers(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "Fix Priority Queue" in md and "已修完" in md
        assert "audit ≠ fix" in md or "audit ≠ fix" in md

    def test_markdown_contains_v1285_v1284_references(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        assert "V1285" in md
        assert "V1284" in md
        assert "V1283" in md  # ASI 5 + VCP #1-#7 section

    def test_markdown_contains_top_10(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger, top=10)
        assert "Top-10 Crates" in md

    def test_markdown_p0_detail(self):
        ledger = run_fix_priority()
        md = _to_markdown(ledger)
        if ledger.n_p0 > 0:
            assert "P0 Detail" in md


class TestJson:
    def test_json_valid(self):
        ledger = run_fix_priority()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert isinstance(d, dict)
        assert "n_p0" in d
        assert "n_p1" in d
        assert "n_p2" in d
        assert "n_ok" in d
        assert "scores" in d

    def test_json_scores_sorted_desc(self):
        ledger = run_fix_priority()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        scores = d["scores"]
        # Verify descending order
        for i in range(len(scores) - 1):
            assert scores[i]["total_score"] >= scores[i + 1]["total_score"]


# ============================================================
# 8. CLI
# ============================================================

class TestCLI:
    def test_probe_mode(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1286 Security Fix Priority" in out
        assert "Governance crates" in out
        assert "Severity weights" in out

    def test_run_mode(self, capsys):
        code = main(["--run"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1286 Security Fix Priority Queue" in out
        assert "P0 (立即修)" in out

    def test_json_mode(self, capsys):
        code = main(["--json"])
        out = capsys.readouterr().out
        assert code == 0
        d = json.loads(out)
        assert "n_p0" in d

    def test_report_mode(self, tmp_path):
        out_path = tmp_path / "v1286_report.md"
        code = main(["--report", str(out_path)])
        assert code == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1286 Security Fix Priority Queue" in content

    def test_p0_only_mode(self, capsys):
        code = main(["--p0-only"])
        out = capsys.readouterr().out
        assert code == 0
        assert "P0" in out
        assert "score=" in out

    def test_module_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1286_security_fix_priority", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        combined = (result.stdout or "") + (result.stderr or "")
        assert "V1286 Security Fix Priority" in combined


# ============================================================
# 9. Integration with V1285
# ============================================================

class TestIntegrationV1285:
    def test_v1286_total_hotspots_match_v1285(self):
        """V1286 score sums should match V1285 hotspot count (1173)."""
        from apeireth.v1285_all42_crate_security_audit import run_all42_audit
        v1285_ledger = run_all42_audit()
        v1286_ledger = run_fix_priority()
        # Sum of (n_critical * 10 + n_important * 5 + n_info * 1) across V1286 scores
        # should equal V1285 total_hotspots
        total = sum(s.n_critical + s.n_important + s.n_info for s in v1286_ledger.scores)
        assert total == v1285_ledger.total_hotspots

    def test_v1286_unsafe_crate_match_v1285(self):
        """V1286 should identify same unsafe crate as V1285 (apeireth-web)."""
        from apeireth.v1285_all42_crate_security_audit import run_all42_audit
        v1285_ledger = run_all42_audit()
        v1286_ledger = run_fix_priority()
        v1285_unsafe = [m for m in v1285_ledger.crate_metrics if m.n_unsafe > 0]
        v1286_unsafe = [s for s in v1286_ledger.scores if s.n_unsafe > 0]
        assert len(v1285_unsafe) == len(v1286_unsafe)
        if v1285_unsafe:
            assert v1285_unsafe[0].crate_name == v1286_unsafe[0].crate_name

    def test_v1286_crates_match_v1285(self):
        """V1286 should score same set of crates as V1285 audited."""
        from apeireth.v1285_all42_crate_security_audit import run_all42_audit
        v1285_ledger = run_all42_audit()
        v1286_ledger = run_fix_priority()
        v1285_names = {m.crate_name for m in v1285_ledger.crate_metrics}
        v1286_names = {s.crate_name for s in v1286_ledger.scores}
        assert v1285_names == v1286_names
