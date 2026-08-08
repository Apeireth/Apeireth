#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1340_vcp_cookbook_validator.py — V1340 tests

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1339 cookbook (cbf4cc9a, 22:01); V1340 validator
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → V1337 → V1338 → V1339 → **V1340**

Tests for V1340 VCP Cookbook Validator.

Tests cover (8 sections × 8 API surfaces):
 1. V1335 + V1336 + V1339 dependencies
 2. ExampleValidationResult fields
 3. VCPValidationReport fields
 4. _run_example (subprocess execution)
 5. _validate_one_example (single example validation)
 6. validate_cookbook (batch validation)
 7. report_to_markdown (rendering)
 8. CLI: main() with --self-test, --cookbook-dir, --json
 9. Self-test (41/41 PASS gate)
10. V3 哲学守门 (LOCKED: 不假装 Phenomenal, 不假装 ASI 达到)
11. ASI pole-star integrity (V0.1=0.7905 + V1340 不动)
12. Closed loop: V1339 cookbook examples must cover their claimed class
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Add apeireth dir to path
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

import pytest

import v1340_vcp_cookbook_validator as v1340  # noqa: E402
import v1339_vcp_substrate_cookbook as v1339  # noqa: E402
import v1336_vcp_plugin_conformance_linter as v1336  # noqa: E402
import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# ============================================================================
# Section 1: V1335 + V1336 + V1339 dependencies (4 tests)
# ============================================================================
class TestDependencies:
    """V1340 depends on V1335 + V1336 + V1339."""

    def test_v1335_imported(self):
        assert v1340.v1335 is not None

    def test_v1336_imported(self):
        assert v1340.v1336 is not None

    def test_v1339_imported(self):
        assert v1340.v1339 is not None

    def test_v1335_8_invariant_classes(self):
        assert len(v1335.INVARIANT_CLASSES) == 8


# ============================================================================
# Section 2: ExampleValidationResult (5 tests)
# ============================================================================
class TestExampleValidationResult:
    """ExampleValidationResult dataclass."""

    def test_example_result_fields(self):
        r = v1340.ExampleValidationResult(
            example_filename="x.py",
            example_path="x.py",
            claimed_class_id="IC1_security",
            claimed_class_label="SecurityInvariants",
            safety_critical=True,
            exists=True,
            runnable=True,
            run_exit_code=0,
            run_stdout_contains_all_checks_pass=True,
            linter_verdict="PASS",
            linter_coverage_score=1.0,
            linter_pass_5_critical=True,
            linter_classes_covered=["IC1_security"],
            claims_class_covered=True,
            validation_pass=True,
            validation_notes=[],
        )
        assert r.example_filename == "x.py"
        assert r.validation_pass is True

    def test_example_result_to_dict(self):
        r = v1340.ExampleValidationResult(
            example_filename="x.py",
            example_path="x.py",
            claimed_class_id="IC2_file_handling",
            claimed_class_label="FileHandlingInvariants",
            safety_critical=True,
            exists=True,
            runnable=True,
            run_exit_code=0,
            run_stdout_contains_all_checks_pass=True,
            linter_verdict="PASS",
            linter_coverage_score=1.0,
            linter_pass_5_critical=True,
            linter_classes_covered=["IC2_file_handling"],
            claims_class_covered=True,
            validation_pass=True,
            validation_notes=[],
        )
        d = r.to_dict()
        assert "example_filename" in d
        assert d["claimed_class_id"] == "IC2_file_handling"

    def test_example_result_run_exit_code(self):
        r = v1340.ExampleValidationResult(
            example_filename="x.py",
            example_path="x.py",
            claimed_class_id="IC3_schema",
            claimed_class_label="SchemaInvariants",
            safety_critical=True,
            exists=True,
            runnable=False,
            run_exit_code=1,
            run_stdout_contains_all_checks_pass=False,
            linter_verdict="FAIL",
            linter_coverage_score=0.0,
            linter_pass_5_critical=False,
            linter_classes_covered=[],
            claims_class_covered=False,
            validation_pass=False,
            validation_notes=["failed"],
        )
        assert r.run_exit_code == 1

    def test_example_result_validation_notes(self):
        r = v1340.ExampleValidationResult(
            example_filename="x.py",
            example_path="x.py",
            claimed_class_id="IC4_ipc",
            claimed_class_label="IPCProtocolInvariants",
            safety_critical=True,
            exists=True,
            runnable=True,
            run_exit_code=0,
            run_stdout_contains_all_checks_pass=True,
            linter_verdict="PASS",
            linter_coverage_score=1.0,
            linter_pass_5_critical=True,
            linter_classes_covered=["IC4_ipc"],
            claims_class_covered=True,
            validation_pass=True,
            validation_notes=["note1", "note2"],
        )
        assert len(r.validation_notes) == 2

    def test_example_result_empty_notes(self):
        r = v1340.ExampleValidationResult(
            example_filename="x.py",
            example_path="x.py",
            claimed_class_id="IC5_error_handling",
            claimed_class_label="ErrorHandlingInvariants",
            safety_critical=False,
            exists=True,
            runnable=True,
            run_exit_code=0,
            run_stdout_contains_all_checks_pass=True,
            linter_verdict="PASS",
            linter_coverage_score=1.0,
            linter_pass_5_critical=True,
            linter_classes_covered=["IC5_error_handling"],
            claims_class_covered=True,
            validation_pass=True,
            validation_notes=[],
        )
        assert r.validation_notes == []


# ============================================================================
# Section 3: VCPValidationReport (5 tests)
# ============================================================================
class TestVCPValidationReport:
    """VCPValidationReport dataclass."""

    def test_report_fields(self):
        r = v1340.VCPValidationReport(
            total_examples=8,
            examples_validated=8,
            examples_passed=8,
            examples_failed=0,
            examples_warned=0,
            overall_pass=True,
            overall_verdict="PASS",
            per_example=[],
            asi_pole_star=v1340.ASI_POLE_STAR,
        )
        assert r.total_examples == 8
        assert r.overall_pass is True

    def test_report_to_dict(self):
        r = v1340.VCPValidationReport(
            total_examples=8,
            examples_validated=8,
            examples_passed=8,
            examples_failed=0,
            examples_warned=0,
            overall_pass=True,
            overall_verdict="PASS",
            per_example=[],
            asi_pole_star=v1340.ASI_POLE_STAR,
        )
        d = r.to_dict()
        assert "overall_pass" in d
        assert "asi_pole_star" in d

    def test_report_failed_state(self):
        r = v1340.VCPValidationReport(
            total_examples=8,
            examples_validated=8,
            examples_passed=0,
            examples_failed=8,
            examples_warned=0,
            overall_pass=False,
            overall_verdict="FAIL",
            per_example=[],
            asi_pole_star=v1340.ASI_POLE_STAR,
        )
        assert r.overall_pass is False

    def test_report_warned_state(self):
        r = v1340.VCPValidationReport(
            total_examples=8,
            examples_validated=8,
            examples_passed=6,
            examples_failed=0,
            examples_warned=2,
            overall_pass=False,
            overall_verdict="PASS_WITH_WARNINGS",
            per_example=[],
            asi_pole_star=v1340.ASI_POLE_STAR,
        )
        assert r.examples_warned == 2

    def test_default_cookbook_dir(self):
        assert v1340.V1339_DEFAULT_COOKBOOK_DIR.name == "v1339_cookbook_examples"


# ============================================================================
# Section 4: _run_example (5 tests)
# ============================================================================
class TestRunExample:
    """_run_example subprocess wrapper."""

    def test_run_example_existing(self):
        # One of V1339 examples
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic1_security.py"
        if path.exists():
            exit_code, stdout = v1340._run_example(path)
            assert exit_code == 0
            assert "ALL CHECKS PASS" in stdout

    def test_run_example_nonexistent(self):
        fake = v1340.V1339_DEFAULT_COOKBOOK_DIR / "v9999_does_not_exist.py"
        exit_code, stdout = v1340._run_example(fake)
        assert exit_code == -1
        assert stdout == ""

    def test_run_example_exits_with_int(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic1_security.py"
        if path.exists():
            exit_code, _ = v1340._run_example(path)
            assert isinstance(exit_code, int)

    def test_run_example_stdout_is_string(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic1_security.py"
        if path.exists():
            _, stdout = v1340._run_example(path)
            assert isinstance(stdout, str)

    def test_run_example_returns_tuple(self):
        result = v1340._run_example(v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic1_security.py")
        assert isinstance(result, tuple)
        assert len(result) == 2


# ============================================================================
# Section 5: _validate_one_example (5 tests)
# ============================================================================
class TestValidateOneExample:
    """_validate_one_example validates single example."""

    def test_validate_ic1_security(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic1_security.py"
        if path.exists():
            r = v1340._validate_one_example(
                path, "IC1_security", "SecurityInvariants", True,
            )
            assert r.claims_class_covered is True

    def test_validate_ic2_file_handling(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic2_file_handling.py"
        if path.exists():
            r = v1340._validate_one_example(
                path, "IC2_file_handling", "FileHandlingInvariants", True,
            )
            assert r.claims_class_covered is True

    def test_validate_ic3_schema(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic3_schema.py"
        if path.exists():
            r = v1340._validate_one_example(
                path, "IC3_schema", "SchemaInvariants", True,
            )
            assert r.claims_class_covered is True

    def test_validate_ic7_resource_bounds(self):
        path = v1340.V1339_DEFAULT_COOKBOOK_DIR / "example_ic7_resource_bounds.py"
        if path.exists():
            r = v1340._validate_one_example(
                path, "IC7_resource_bounds", "ResourceBoundsInvariants", True,
            )
            assert r.claims_class_covered is True

    def test_validate_nonexistent_example(self):
        fake = v1340.V1339_DEFAULT_COOKBOOK_DIR / "v9999_does_not_exist.py"
        r = v1340._validate_one_example(
            fake, "IC1_security", "SecurityInvariants", True,
        )
        assert r.exists is False
        assert r.validation_pass is False


# ============================================================================
# Section 6: validate_cookbook (5 tests)
# ============================================================================
class TestValidateCookbook:
    """validate_cookbook returns VCPValidationReport."""

    def test_validate_cookbook_total_8(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.total_examples == 8

    def test_validate_cookbook_validated_8(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.examples_validated == 8

    def test_validate_cookbook_8_passed(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.examples_passed == 8

    def test_validate_cookbook_overall_pass(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.overall_pass is True

    def test_validate_cookbook_verdict_pass(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.overall_verdict == "PASS"


# ============================================================================
# Section 7: report_to_markdown (4 tests)
# ============================================================================
class TestReportToMarkdown:
    """report_to_markdown rendering."""

    def test_md_contains_total(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            md = v1340.report_to_markdown(r)
            assert "Total examples: 8" in md

    def test_md_contains_overall_verdict(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            md = v1340.report_to_markdown(r)
            assert "Overall verdict" in md

    def test_md_contains_per_example(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            md = v1340.report_to_markdown(r)
            assert "Per-example validation" in md

    def test_md_contains_example_filenames(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            md = v1340.report_to_markdown(r)
            for example in v1339.build_examples():
                assert example.filename in md


# ============================================================================
# Section 8: CLI (5 tests)
# ============================================================================
class TestCLI:
    """main() CLI entry point."""

    def test_cli_self_test(self, capsys):
        rc = v1340.main(["--self-test"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "ALL CHECKS PASS" in captured.out

    def test_cli_validate_default(self, capsys):
        rc = v1340.main([])
        captured = capsys.readouterr()
        assert "VCP Cookbook Validation Report" in captured.out

    def test_cli_validate_json(self, capsys):
        rc = v1340.main(["--json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert "total_examples" in data
        assert "per_example" in data

    def test_cli_validate_cookbook_dir(self, capsys):
        rc = v1340.main(["--cookbook-dir", str(v1340.V1339_DEFAULT_COOKBOOK_DIR)])
        captured = capsys.readouterr()
        assert "VCP Cookbook Validation Report" in captured.out

    def test_cli_returns_int(self):
        rc = v1340.main(["--self-test"])
        assert isinstance(rc, int)


# ============================================================================
# Section 9: Self-test (4 tests)
# ============================================================================
class TestRunAllSelfTest:
    """All 41 self-test checks must pass."""

    def test_self_test_returns_dict(self):
        results = v1340._self_test()
        assert isinstance(results, dict)
        assert len(results) >= 40

    def test_all_self_tests_pass(self):
        results = v1340._self_test()
        failed = [k for k, v in results.items() if not v]
        assert not failed, f"Failed: {failed}"

    def test_self_test_summary_41_pass(self):
        passed, failed, failed_names = v1340._self_test_summary()
        assert passed == 41
        assert failed == 0
        assert failed_names == []

    def test_self_test_at_least_40(self):
        results = v1340._self_test()
        assert len(results) >= 40


# ============================================================================
# Section 10: V3 哲学守门 (5 tests)
# ============================================================================
class TestV3PhilosophicalGuards:
    """V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43)."""

    def test_no_pretend_phenomenal(self):
        for name in dir(v1340):
            if name.startswith("_"):
                continue
            attr = getattr(v1340, name)
            if isinstance(attr, str):
                assert "phenomenal" not in attr.lower() or "guard" in attr.lower()

    def test_asi_pole_star_locked(self):
        assert v1340.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1340.ASI_POLE_STAR["V1340_modifies_pole_star"] is False

    def test_asi_achieved_still_false(self):
        assert v1340.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1049_value_alignment_done(self):
        assert v1340.ASI_POLE_STAR["V1049_value_alignment_done"] is True

    def test_V1256_unio_mystica_realized(self):
        assert v1340.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105


# ============================================================================
# Section 11: ASI pole-star integrity (4 tests)
# ============================================================================
class TestASIPoleStar:
    """ASI 北极星 LOCKED — V1340 不动."""

    def test_asi_pole_star_constants(self):
        assert v1340.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1340.ASI_POLE_STAR["V0_max_any_epoch"] == 0.9800
        assert v1340.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105

    def test_asi_achieved_still_false(self):
        assert v1340.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1340_does_not_modify_pole_star(self):
        assert v1340.ASI_POLE_STAR["V1340_modifies_pole_star"] is False

    def test_V1049_value_alignment_done(self):
        assert v1340.ASI_POLE_STAR["V1049_value_alignment_done"] is True


# ============================================================================
# Section 12: Closed loop (5 tests)
# ============================================================================
class TestClosedLoop:
    """Closed loop: V1339 cookbook × V1336 linter × V1335 registry."""

    def test_closed_loop_all_8_pass(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.examples_passed == 8

    def test_closed_loop_each_example_claims_class(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            for ex in r.per_example:
                assert ex.claims_class_covered is True

    def test_closed_loop_each_example_runnable(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            for ex in r.per_example:
                assert ex.runnable is True

    def test_closed_loop_each_example_all_checks_pass(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            for ex in r.per_example:
                assert ex.run_stdout_contains_all_checks_pass is True

    def test_closed_loop_overall_pass(self):
        if v1340.V1339_DEFAULT_COOKBOOK_DIR.exists():
            r = v1340.validate_cookbook(v1340.V1339_DEFAULT_COOKBOOK_DIR)
            assert r.overall_pass is True
            assert r.overall_verdict == "PASS"


# ============================================================================
# Section 13: Module docstring + API (3 tests)
# ============================================================================
class TestModuleInvariants:
    """V1340 module-level invariants."""

    def test_module_docstring_present(self):
        assert v1340.__doc__ is not None
        assert "V1340" in v1340.__doc__

    def test_V3_guards_present(self):
        for guard in ["不假装", "ASI 北极星", "Phenomenal consciousness", "调整模型 & prompt"]:
            assert guard in v1340.__doc__, f"Missing guard: {guard}"

    def test_chain_reference(self):
        assert "V1339" in v1340.__doc__
        assert "V1340" in v1340.__doc__
