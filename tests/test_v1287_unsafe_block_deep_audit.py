"""V1287 — Unsafe Block Deep Audit 真生产测试套件

> 45 tests for v1287:
> - Version / constants: 6 tests
> - Risk assessment: 6 tests
> - Context extraction: 3 tests
> - Philosophy gate: 3 tests
> - Data structures: 4 tests
> - Scanner: 4 tests
> - Runner: 5 tests
> - Output (Markdown / JSON): 6 tests
> - CLI: 6 tests
> - Integration with V1285/V1286: 2 tests
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1287_unsafe_block_deep_audit import (
    V1287_VERSION,
    V1287_BUILD,
    V1287_ASI_NS_CURRENT,
    V1287_ASI_NS_LOCKED_PCT,
    V1287_UNSAFE_PATTERNS,
    V1287_CONTEXT_LINES,
    UnsafeFinding,
    CrateUnsafeMetrics,
    UnsafeLedger,
    _assess_safety_comment,
    _classify_risk,
    _extract_context,
    scan_crate_unsafe,
    run_unsafe_audit,
    _v1287_philosophy_gate,
    _to_markdown,
    _to_json_snapshot,
    main,
)


# ============================================================
# 1. Version / constants
# ============================================================

class TestVersion:
    def test_version_is_string(self):
        assert isinstance(V1287_VERSION, str) and len(V1287_VERSION) > 0

    def test_build_is_string(self):
        assert isinstance(V1287_BUILD, str) and "2026-08-05" in V1287_BUILD

    def test_asi_ns_current(self):
        assert V1287_ASI_NS_CURRENT == 0.7905
        assert V1287_ASI_NS_LOCKED_PCT == 92.91


class TestPatterns:
    def test_unsafe_patterns_count(self):
        assert len(V1287_UNSAFE_PATTERNS) == 5

    def test_unsafe_pattern_keys(self):
        expected = {"unsafe_block", "unsafe_fn", "unsafe_trait", "unsafe_impl", "unsafe_extern"}
        assert set(V1287_UNSAFE_PATTERNS.keys()) == expected

    def test_unsafe_block_pattern_matches(self):
        p = V1287_UNSAFE_PATTERNS["unsafe_block"]
        assert re.search(p, "unsafe {")
        assert re.search(p, "unsafe{")  # no space
        # The raw pattern matches even in comments — the scanner handles comment stripping
        # (the assertion below verifies that the scanner, not the raw regex, filters comments)
        assert re.search(p, "// unsafe { ... }")  # raw regex matches; scanner filters

    def test_context_lines(self):
        assert V1287_CONTEXT_LINES == 5


# ============================================================
# 2. Risk assessment
# ============================================================

class TestRiskAssessment:
    def test_safety_comment_found(self):
        ctx = ["// SAFETY: this is fine", "let x = 1;", "unsafe {"]
        has_safety, text = _assess_safety_comment(ctx)
        assert has_safety is True
        assert "SAFETY" in text.upper()

    def test_safety_comment_missing(self):
        ctx = ["let x = 1;", "unsafe {"]
        has_safety, text = _assess_safety_comment(ctx)
        assert has_safety is False
        assert text == ""

    def test_safety_case_insensitive(self):
        ctx = ["// safety: lowercase", "unsafe {"]
        has_safety, text = _assess_safety_comment(ctx)
        assert has_safety is True

    def test_safety_chinese_colon(self):
        ctx = ["// SAFETY：中文冒号", "unsafe {"]
        has_safety, text = _assess_safety_comment(ctx)
        assert has_safety is True

    def test_classify_unsafe_block_with_safety(self):
        assert _classify_risk("unsafe_block", True) == "justified"

    def test_classify_unsafe_block_no_safety(self):
        assert _classify_risk("unsafe_block", False) == "unjustified"

    def test_classify_unsafe_extern_always_questionable(self):
        # Even with SAFETY, extern FFI is questionable
        assert _classify_risk("unsafe_extern", True) == "questionable"
        assert _classify_risk("unsafe_extern", False) == "unjustified"

    def test_classify_unsafe_trait_no_safety(self):
        # trait without safety = questionable
        assert _classify_risk("unsafe_trait", False) == "questionable"


# ============================================================
# 3. Context extraction
# ============================================================

class TestContextExtraction:
    def test_extract_context_normal(self):
        lines = ["line 1", "line 2", "line 3", "line 4", "line 5", "TARGET", "line 7", "line 8"]
        before, after = _extract_context(lines, 5, n_before=2, n_after=2)
        # n_before=2 means start at idx 5-2=3, end at idx 5 (exclusive) → indices [3, 4] = 2 elements
        assert before == ["line 4", "line 5"]
        assert after == ["line 7", "line 8"]

    def test_extract_context_at_start(self):
        lines = ["TARGET", "line 2", "line 3"]
        before, after = _extract_context(lines, 0, n_before=5, n_after=2)
        assert before == []
        assert after == ["line 2", "line 3"]

    def test_extract_context_at_end(self):
        lines = ["line 1", "line 2", "TARGET"]
        before, after = _extract_context(lines, 2, n_before=5, n_after=5)
        assert before == ["line 1", "line 2"]
        assert after == []


# ============================================================
# 4. Philosophy gate
# ============================================================

class TestPhilosophyGate:
    def test_gate_has_33_entries(self):
        gate = _v1287_philosophy_gate()
        assert len(gate) == 33  # V1286 30 + V1287 3

    def test_gate_has_v1287_new(self):
        gate = _v1287_philosophy_gate()
        assert "v1287_extends_v1286_not_replaces" in gate
        assert "v1287_apeireth_only_not_vendor" in gate
        assert "v1287_audit_only_no_fix" in gate

    def test_all_gates_true(self):
        gate = _v1287_philosophy_gate()
        for k, v in gate.items():
            assert v is True, f"Gate {k} should be True"


# ============================================================
# 5. Data structures
# ============================================================

class TestLedger:
    def test_ledger_default(self):
        ledger = UnsafeLedger()
        assert ledger.total_unsafe == 0
        assert ledger.total_justified == 0
        assert ledger.total_questionable == 0
        assert ledger.total_unjustified == 0

    def test_unsafe_finding_to_dict(self):
        f = UnsafeFinding(
            crate_name="apeireth-test",
            pattern_id="unsafe_block",
            file_path="/tmp/lib.rs",
            line_number=10,
            line_text="unsafe { ... }",
            has_safety_comment=True,
            safety_comment_text="// SAFETY: ok",
            risk_level="justified",
        )
        d = f.to_dict()
        assert d["crate_name"] == "apeireth-test"
        assert d["pattern_id"] == "unsafe_block"
        assert d["line_number"] == 10
        assert d["risk_level"] == "justified"

    def test_crate_metrics_total(self):
        m = CrateUnsafeMetrics(
            crate_name="x", crate_src="/tmp/x",
            src_files_scanned=1, src_lines_scanned=100,
            n_unsafe_block=2, n_unsafe_fn=1, n_unsafe_trait=1, n_unsafe_impl=1, n_unsafe_extern=1,
        )
        assert m.n_total == 6

    def test_crate_metrics_risk_counts(self):
        m = CrateUnsafeMetrics(
            crate_name="x", crate_src="/tmp/x",
            src_files_scanned=1, src_lines_scanned=100,
        )
        m.findings = [
            UnsafeFinding(crate_name="x", pattern_id="unsafe_block", file_path="/x", line_number=1, line_text="a", risk_level="justified"),
            UnsafeFinding(crate_name="x", pattern_id="unsafe_block", file_path="/x", line_number=2, line_text="b", risk_level="questionable"),
            UnsafeFinding(crate_name="x", pattern_id="unsafe_block", file_path="/x", line_number=3, line_text="c", risk_level="unjustified"),
        ]
        assert m.n_justified == 1
        assert m.n_questionable == 1
        assert m.n_unjustified == 1


# ============================================================
# 6. Scanner
# ============================================================

class TestScanner:
    def test_scan_clean_crate(self, tmp_path):
        # Create a fake crate with no unsafe
        src = tmp_path / "src"
        src.mkdir()
        (src / "lib.rs").write_text("fn main() { println!(\"hello\"); }", encoding="utf-8")
        m = scan_crate_unsafe("apeireth-test", src)
        assert m.n_total == 0
        assert m.findings == []

    def test_scan_unsafe_block(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        (src / "lib.rs").write_text(
            "fn main() {\n// SAFETY: test\nunsafe { let x = 1; }\n}\n",
            encoding="utf-8",
        )
        m = scan_crate_unsafe("apeireth-test", src)
        assert m.n_unsafe_block >= 1
        # Verify it has SAFETY comment
        assert any(f.has_safety_comment for f in m.findings)

    def test_scan_unsafe_block_no_safety(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        (src / "lib.rs").write_text(
            "fn main() {\nunsafe { let x = 1; }\n}\n",
            encoding="utf-8",
        )
        m = scan_crate_unsafe("apeireth-test", src)
        assert m.n_unsafe_block >= 1
        # Verify it has no SAFETY comment
        assert any(not f.has_safety_comment for f in m.findings)

    def test_scan_with_comment_excluded(self, tmp_path):
        src = tmp_path / "src"
        src.mkdir()
        (src / "lib.rs").write_text(
            "fn main() {\n// don't write `unsafe impl Send`\nfn foo() {}\n}\n",
            encoding="utf-8",
        )
        m = scan_crate_unsafe("apeireth-test", src)
        # Should NOT count comment as unsafe
        assert m.n_unsafe_impl == 0


# ============================================================
# 7. Runner
# ============================================================

class TestRunner:
    def test_run_returns_ledger(self):
        ledger = run_unsafe_audit()
        assert isinstance(ledger, UnsafeLedger)
        assert ledger.n_crates_audited >= 42

    def test_run_finds_unsafe_in_apeireth_web(self):
        """V1285 found 1 unsafe in apeireth-web → V1287 must find ≥1."""
        ledger = run_unsafe_audit()
        web = next((m for m in ledger.crate_metrics if m.crate_name == "apeireth-web"), None)
        assert web is not None
        assert web.n_total >= 1

    def test_run_total_unsafe_at_least_1(self):
        ledger = run_unsafe_audit()
        assert ledger.total_unsafe >= 1

    def test_run_philosophy_gate(self):
        ledger = run_unsafe_audit()
        assert len(ledger.philosophy_gate) == 33

    def test_run_with_crate_filter(self):
        ledger = run_unsafe_audit(crate_filter=lambda c: c == "apeireth-web")
        assert ledger.n_crates_audited == 1
        assert ledger.crate_metrics[0].crate_name == "apeireth-web"


# ============================================================
# 8. Output
# ============================================================

class TestMarkdown:
    def test_markdown_contains_header(self):
        ledger = run_unsafe_audit()
        md = _to_markdown(ledger)
        assert "V1287 Unsafe Block Deep Audit" in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_unsafe_audit()
        md = _to_markdown(ledger)
        assert "V3 Philosophy Gate" in md
        assert "v1287_extends_v1286_not_replaces" in md

    def test_markdown_contains_disclaimers(self):
        ledger = run_unsafe_audit()
        md = _to_markdown(ledger)
        assert "VCP unsafe 深度审计" in md
        assert "audit ≠ fix" in md or "audit =/= fix" in md

    def test_markdown_contains_vcp_series(self):
        ledger = run_unsafe_audit()
        md = _to_markdown(ledger)
        for v in ["V1280", "V1284", "V1285", "V1286", "V1287"]:
            assert v in md, f"{v} should be referenced"


class TestJson:
    def test_json_valid(self):
        ledger = run_unsafe_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert "total_unsafe" in d
        assert "total_justified" in d
        assert "total_questionable" in d
        assert "total_unjustified" in d

    def test_json_includes_findings(self):
        ledger = run_unsafe_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        for m in d["per_crate_metrics"]:
            if m["n_total"] > 0:
                assert len(m["findings"]) > 0


# ============================================================
# 9. CLI
# ============================================================

class TestCLI:
    def test_probe_mode(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1287 Unsafe Block Deep Audit" in out
        assert "apeireth-" in out

    def test_run_mode(self, capsys):
        code = main(["--run"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1287 Unsafe Block Deep Audit" in out
        assert "Total unsafe usages" in out

    def test_json_mode(self, capsys):
        code = main(["--json"])
        out = capsys.readouterr().out
        assert code == 0
        d = json.loads(out)
        assert "total_unsafe" in d

    def test_report_mode(self, tmp_path):
        out_path = tmp_path / "v1287_report.md"
        code = main(["--report", str(out_path)])
        assert code == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1287 Unsafe Block Deep Audit" in content

    def test_risk_only_mode(self, capsys):
        code = main(["--risk-only"])
        out = capsys.readouterr().out
        assert code == 0
        assert "Risk-Only" in out

    def test_module_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1287_unsafe_block_deep_audit", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        combined = (result.stdout or "") + (result.stderr or "")
        assert "V1287 Unsafe Block Deep Audit" in combined


# ============================================================
# 10. Integration with V1285/V1286
# ============================================================

class TestIntegrationV1285V1286:
    def test_v1287_consistent_with_v1285_unsafe_count(self):
        """V1285 found 1 unsafe in apeireth-web. V1287 should find ≥1 (may find more if fn/trait)."""
        from apeireth.v1285_all42_crate_security_audit import run_all42_audit
        v1285_ledger = run_all42_audit()
        v1287_ledger = run_unsafe_audit()
        v1285_unsafe_total = sum(m.n_unsafe for m in v1285_ledger.crate_metrics)
        # V1287 should find >= V1285 (deeper scan)
        assert v1287_ledger.total_unsafe >= v1285_unsafe_total

    def test_v1287_unsafe_block_matches_v1285_unsafe_count(self):
        """V1285's unsafe is specifically `unsafe_block`. V1287's `unsafe_block` count >= V1285's total."""
        from apeireth.v1285_all42_crate_security_audit import run_all42_audit
        v1285_ledger = run_all42_audit()
        v1287_ledger = run_unsafe_audit()
        v1285_total = sum(m.n_unsafe for m in v1285_ledger.crate_metrics)
        v1287_block_total = sum(m.n_unsafe_block for m in v1287_ledger.crate_metrics)
        # V1285 counted only unsafe_block, V1287 may match or be more inclusive
        assert v1287_block_total >= v1285_total
