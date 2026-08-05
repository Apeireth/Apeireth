"""V1288 — Governance Core Deep Audit 真生产测试套件

> 40 tests for v1288:
> - Version / constants: 5 tests
> - Function detection: 6 tests
> - Function-line mapping: 4 tests
> - Function grouping: 4 tests
> - Philosophy gate: 3 tests
> - Data structures: 3 tests
> - Runner: 5 tests
> - Output: 5 tests
> - CLI: 5 tests
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1288_governance_core_deep_audit import (
    V1288_VERSION,
    V1288_BUILD,
    V1288_ASI_NS_CURRENT,
    V1288_ASI_NS_LOCKED_PCT,
    V1288_GOVERNANCE_CRATES,
    V1288_GOVERNANCE_WEIGHT,
    FN_DEF_RE,
    FunctionHotspot,
    CrateGovernanceMetrics,
    GovernanceLedger,
    _detect_functions_in_file,
    _find_function_for_line,
    _group_findings_by_function,
    run_governance_audit,
    _v1288_philosophy_gate,
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
        assert isinstance(V1288_VERSION, str) and len(V1288_VERSION) > 0

    def test_build_is_string(self):
        assert isinstance(V1288_BUILD, str) and "2026-08-05" in V1288_BUILD

    def test_asi_ns_current(self):
        assert V1288_ASI_NS_CURRENT == 0.7905
        assert V1288_ASI_NS_LOCKED_PCT == 92.91


class TestConstants:
    def test_governance_crates_count(self):
        assert len(V1288_GOVERNANCE_CRATES) == 5

    def test_governance_crates_correct(self):
        expected = {
            "apeireth-sovereignty", "apeireth-upgrade",
            "apeireth-evolution", "apeireth-asi", "apeireth-council",
        }
        assert set(V1288_GOVERNANCE_CRATES) == expected

    def test_governance_weight(self):
        assert V1288_GOVERNANCE_WEIGHT == 50


# ============================================================
# 2. Function detection
# ============================================================

class TestFunctionDetection:
    def test_detect_simple_fn(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("fn main() {}\nfn foo() {}\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert len(funcs) == 2
        assert funcs[0] == ("main", 1)
        assert funcs[1] == ("foo", 2)

    def test_detect_pub_fn(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("pub fn bar() {}\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert funcs == [("bar", 1)]

    def test_detect_async_fn(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("pub async fn baz() {}\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert funcs == [("baz", 1)]

    def test_detect_unsafe_fn(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("pub unsafe fn qux() {}\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert funcs == [("qux", 1)]

    def test_detect_const_fn(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("pub const fn quux() -> i32 { 42 }\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert funcs == [("quux", 1)]

    def test_no_functions(self, tmp_path):
        f = tmp_path / "lib.rs"
        f.write_text("let x = 1;\n// just a comment\n", encoding="utf-8")
        funcs = _detect_functions_in_file(f)
        assert funcs == []


# ============================================================
# 3. Function-line mapping (主 17:43 实事求是)
# ============================================================

class TestFunctionLineMapping:
    def test_module_fallback(self):
        # No functions defined
        fname, fstart = _find_function_for_line([], 100)
        assert fname == "<module>"
        assert fstart == 100

    def test_before_first_function(self):
        funcs = [("foo", 50), ("bar", 100)]
        fname, fstart = _find_function_for_line(funcs, 30)
        assert fname == "<module>"

    def test_inside_function(self):
        funcs = [("foo", 50), ("bar", 100)]
        fname, fstart = _find_function_for_line(funcs, 75)
        assert fname == "foo"
        assert fstart == 50

    def test_after_last_function(self):
        funcs = [("foo", 50), ("bar", 100)]
        # After bar (last function)
        fname, fstart = _find_function_for_line(funcs, 200)
        # Still picks bar as most-recent
        assert fname == "bar"
        assert fstart == 100


# ============================================================
# 4. Function grouping
# ============================================================

class TestFunctionGrouping:
    def test_group_empty(self, tmp_path):
        m = CrateSecurityMetrics(
            crate_name="apeireth-test", crate_src=str(tmp_path / "src"),
            src_files_scanned=0, src_lines_scanned=0,
        )
        (tmp_path / "src").mkdir()
        grouped = _group_findings_by_function(m)
        assert grouped == []

    def test_group_by_function(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        rs = src / "lib.rs"
        rs.write_text(
            "fn foo() {\n    x.unwrap();\n}\nfn bar() {\n    y.unwrap();\n    z.expect(\"x\");\n}\n",
            encoding="utf-8",
        )
        m = CrateSecurityMetrics(
            crate_name="apeireth-test", crate_src=str(src),
            src_files_scanned=1, src_lines_scanned=6,
        )
        m.findings = [
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=2, line_text="x.unwrap()", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=5, line_text="y.unwrap()", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="expect_call", file_path=str(rs), line_number=6, line_text="z.expect(\"x\")", severity="important"),
        ]
        m.n_unwrap = 2
        m.n_expect = 1
        grouped = _group_findings_by_function(m)
        assert len(grouped) == 2  # foo + bar
        foo = next((h for h in grouped if h.function_name == "foo"), None)
        bar = next((h for h in grouped if h.function_name == "bar"), None)
        assert foo is not None and foo.n_findings == 1
        assert bar is not None and bar.n_findings == 2

    def test_group_sorted_by_findings(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        rs = src / "lib.rs"
        rs.write_text("fn foo() {}\nfn bar() {}\n", encoding="utf-8")
        m = CrateSecurityMetrics(
            crate_name="apeireth-test", crate_src=str(src),
            src_files_scanned=1, src_lines_scanned=2,
        )
        # 1 finding in foo, 3 in bar
        m.findings = [
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=1, line_text="a", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=2, line_text="b", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=2, line_text="c", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=2, line_text="d", severity="critical"),
        ]
        grouped = _group_findings_by_function(m)
        # bar should come first (3 findings > 1)
        assert grouped[0].function_name == "bar"
        assert grouped[1].function_name == "foo"

    def test_group_function_counts(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        rs = src / "lib.rs"
        rs.write_text("fn foo() {}\n", encoding="utf-8")
        m = CrateSecurityMetrics(
            crate_name="apeireth-test", crate_src=str(src),
            src_files_scanned=1, src_lines_scanned=1,
        )
        m.findings = [
            SecurityFinding(crate_name="apeireth-test", pattern_id="unwrap_call", file_path=str(rs), line_number=1, line_text="a", severity="critical"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="expect_call", file_path=str(rs), line_number=1, line_text="b", severity="important"),
            SecurityFinding(crate_name="apeireth-test", pattern_id="panic_macro", file_path=str(rs), line_number=1, line_text="c", severity="critical"),
        ]
        m.n_unwrap = 1
        m.n_expect = 1
        m.n_panic = 1
        grouped = _group_findings_by_function(m)
        assert len(grouped) == 1
        h = grouped[0]
        assert h.n_unwrap == 1
        assert h.n_expect == 1
        assert h.n_panic == 1


# ============================================================
# 5. Philosophy gate
# ============================================================

class TestPhilosophyGate:
    def test_gate_has_36_entries(self):
        gate = _v1288_philosophy_gate()
        assert len(gate) == 36  # V1287 33 + V1288 3

    def test_gate_has_v1288_new(self):
        gate = _v1288_philosophy_gate()
        assert "v1288_extends_v1287_not_replaces" in gate
        assert "v1288_governance_5_only" in gate
        assert "v1288_function_grouping_advisory" in gate

    def test_all_gates_true(self):
        gate = _v1288_philosophy_gate()
        for k, v in gate.items():
            assert v is True


# ============================================================
# 6. Data structures
# ============================================================

class TestDataStructures:
    def test_function_hotspot_to_dict(self):
        h = FunctionHotspot(
            function_name="foo", file_path="/tmp/lib.rs",
            start_line=10, end_line=20, n_findings=5,
            n_unwrap=3, n_expect=2, n_panic=0, n_todo=0, n_unsafe=0,
        )
        d = h.to_dict()
        assert d["function_name"] == "foo"
        assert d["n_findings"] == 5
        # span_lines is a property, not in dict
        assert h.span_lines == 10

    def test_governance_metrics_weighted_score(self):
        m = CrateGovernanceMetrics(
            crate_name="apeireth-test",
            crate_src="/tmp",
            src_files_scanned=1,
            src_lines_scanned=100,
            metrics=CrateSecurityMetrics(
                crate_name="apeireth-test", crate_src="/tmp",
                src_files_scanned=1, src_lines_scanned=100,
                n_unwrap=5, n_expect=2,  # 50 + 10 = 60 base + 50 weight = 110
            ),
        )
        assert m.weighted_score == 110

    def test_ledger_defaults(self):
        ledger = GovernanceLedger()
        assert ledger.total_findings == 0
        assert ledger.total_functions_with_findings == 0


# ============================================================
# 7. Runner
# ============================================================

class TestRunner:
    def test_run_audits_5_governance_crates(self):
        ledger = run_governance_audit()
        assert ledger.n_crates_audited == 5

    def test_run_finds_findings(self):
        ledger = run_governance_audit()
        assert ledger.total_findings > 0

    def test_run_total_panic(self):
        """V1285 found: sovereignty 11 + upgrade 15 + evolution 0 + asi 0 + council 0 = 26 panic"""
        ledger = run_governance_audit()
        assert ledger.total_panic == 26

    def test_run_function_grouping(self):
        ledger = run_governance_audit()
        assert ledger.total_functions_with_findings > 0

    def test_run_with_crate_filter(self):
        ledger = run_governance_audit(crate_filter=lambda c: c == "apeireth-sovereignty")
        assert ledger.n_crates_audited == 1
        assert ledger.crate_metrics[0].crate_name == "apeireth-sovereignty"


# ============================================================
# 8. Output
# ============================================================

class TestMarkdown:
    def test_markdown_contains_header(self):
        ledger = run_governance_audit()
        md = _to_markdown(ledger)
        assert "V1288 Governance Core Deep Audit" in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_governance_audit()
        md = _to_markdown(ledger)
        assert "V3 Philosophy Gate" in md
        assert "v1288_extends_v1287_not_replaces" in md

    def test_markdown_contains_spectrum(self):
        ledger = run_governance_audit()
        md = _to_markdown(ledger)
        assert "V1284 (worst-5)" in md
        assert "V1288 (governance-5)" in md
        assert "ratio" in md

    def test_markdown_contains_v1284_reference(self):
        ledger = run_governance_audit()
        md = _to_markdown(ledger)
        assert "V1284" in md
        assert "V1287" in md
        assert "V1288" in md

    def test_markdown_contains_disclaimers(self):
        ledger = run_governance_audit()
        md = _to_markdown(ledger)
        assert "governance" in md
        assert "audit ≠ fix" in md or "audit =/= fix" in md


class TestJson:
    def test_json_valid(self):
        ledger = run_governance_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert "total_findings" in d
        assert "total_unwrap" in d
        assert "total_functions_with_findings" in d

    def test_json_includes_function_hotspots(self):
        ledger = run_governance_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        for m in d["per_crate_metrics"]:
            assert "function_hotspots" in m
            # Each function hotspot has function_name
            for fh in m["function_hotspots"]:
                assert "function_name" in fh
                assert "n_findings" in fh


# ============================================================
# 9. CLI
# ============================================================

class TestCLI:
    def test_probe_mode(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1288 Governance Core Deep Audit" in out
        assert "Governance crates" in out

    def test_run_mode(self, capsys):
        code = main(["--run"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1288 Governance Core Deep Audit" in out
        assert "Total findings" in out

    def test_json_mode(self, capsys):
        code = main(["--json"])
        out = capsys.readouterr().out
        assert code == 0
        d = json.loads(out)
        assert "total_findings" in d

    def test_report_mode(self, tmp_path):
        out_path = tmp_path / "v1288_report.md"
        code = main(["--report", str(out_path)])
        assert code == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1288 Governance Core Deep Audit" in content

    def test_module_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1288_governance_core_deep_audit", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        combined = (result.stdout or "") + (result.stderr or "")
        assert "V1288 Governance Core Deep Audit" in combined
