#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1339_vcp_substrate_cookbook.py — V1339 tests

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1338 migration tool (014f82ff, 22:01); V1339 cookbook
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → V1337 → V1338 → **V1339**

Tests for V1339 VCP Substrate-by-Example Cookbook.

Tests cover (10 sections × 8 API surfaces):
 1. V1335 dependency + 8 example templates
 2. CookbookExample (filename + content + line_count + byte_size)
 3. CookbookIndex (version + counts + examples + ASI pole-star)
 4. build_examples (8 examples)
 5. build_index (cookbook index)
 6. write_examples_to_dir (8 files on disk)
 7. write_index_to_dir (index.md on disk)
 8. CLI: main() with --self-test, --output-dir, --json
 9. Self-test (86/86 PASS gate)
10. V3 哲学守门 (LOCKED: 不假装 Phenomenal, 不假装 ASI 达到)
11. ASI pole-star integrity (V0.1=0.7905 + V1339 不动)
12. 5-critical coverage rule (5 SC classes in cookbook)
13. Examples are runnable (each example has __main__ + _self_test)
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

# Add apeireth dir to path
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

import pytest

import v1339_vcp_substrate_cookbook as v1339  # noqa: E402
import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# ============================================================================
# Section 1: V1335 dependency + 8 example templates (10 tests)
# ============================================================================
class TestDependencies:
    """V1339 depends on V1335 (invariant registry)."""

    def test_v1335_imported(self):
        assert v1339.v1335 is not None

    def test_v1335_8_invariant_classes(self):
        assert len(v1335.INVARIANT_CLASSES) == 8

    def test_example_templates_8(self):
        assert len(v1339.EXAMPLE_TEMPLATES) == 8

    def test_IC1_security_template(self):
        assert "IC1_security" in v1339.EXAMPLE_TEMPLATES
        assert "PathSanitizationSubstrate" in v1339.EXAMPLE_TEMPLATES["IC1_security"]

    def test_IC2_file_handling_template(self):
        assert "IC2_file_handling" in v1339.EXAMPLE_TEMPLATES
        assert "AtomicJsonWriteSubstrate" in v1339.EXAMPLE_TEMPLATES["IC2_file_handling"]

    def test_IC3_schema_template(self):
        assert "IC3_schema" in v1339.EXAMPLE_TEMPLATES
        assert "manifestVersion" in v1339.EXAMPLE_TEMPLATES["IC3_schema"]

    def test_IC4_ipc_template(self):
        assert "IC4_ipc" in v1339.EXAMPLE_TEMPLATES
        assert "jsonrpc" in v1339.EXAMPLE_TEMPLATES["IC4_ipc"].lower()

    def test_IC5_error_handling_template(self):
        assert "IC5_error_handling" in v1339.EXAMPLE_TEMPLATES
        assert "success" in v1339.EXAMPLE_TEMPLATES["IC5_error_handling"]

    def test_IC6_configuration_template(self):
        assert "IC6_configuration" in v1339.EXAMPLE_TEMPLATES
        assert "merge" in v1339.EXAMPLE_TEMPLATES["IC6_configuration"]

    def test_IC7_resource_bounds_template(self):
        assert "IC7_resource_bounds" in v1339.EXAMPLE_TEMPLATES
        assert "token" in v1339.EXAMPLE_TEMPLATES["IC7_resource_bounds"].lower()

    def test_IC8_lifecycle_template(self):
        assert "IC8_lifecycle" in v1339.EXAMPLE_TEMPLATES
        assert "_self_test" in v1339.EXAMPLE_TEMPLATES["IC8_lifecycle"]


# ============================================================================
# Section 2: CookbookExample (5 tests)
# ============================================================================
class TestCookbookExample:
    """CookbookExample dataclass."""

    def test_example_fields(self):
        e = v1339.CookbookExample(
            invariant_class_id="IC1_security",
            invariant_label="SecurityInvariants",
            safety_critical=True,
            filename="example_ic1_security.py",
            content="# Example",
            line_count=1,
            byte_size=8,
        )
        assert e.invariant_class_id == "IC1_security"
        assert e.safety_critical is True

    def test_example_to_dict(self):
        e = v1339.CookbookExample(
            invariant_class_id="IC2_file_handling",
            invariant_label="FileHandlingInvariants",
            safety_critical=True,
            filename="example_ic2_file_handling.py",
            content="x",
            line_count=1,
            byte_size=1,
        )
        d = e.to_dict()
        assert "invariant_class_id" in d
        assert d["filename"] == "example_ic2_file_handling.py"

    def test_example_line_count_positive(self):
        e = v1339.CookbookExample(
            invariant_class_id="IC3_schema",
            invariant_label="SchemaInvariants",
            safety_critical=True,
            filename="x.py",
            content="x\ny\nz",
            line_count=3,
            byte_size=5,
        )
        assert e.line_count == 3

    def test_example_byte_size_positive(self):
        e = v1339.CookbookExample(
            invariant_class_id="IC4_ipc",
            invariant_label="IPCProtocolInvariants",
            safety_critical=True,
            filename="x.py",
            content="abcdef",
            line_count=1,
            byte_size=6,
        )
        assert e.byte_size == 6

    def test_example_safety_critical_false(self):
        e = v1339.CookbookExample(
            invariant_class_id="IC8_lifecycle",
            invariant_label="LifecycleInvariants",
            safety_critical=False,
            filename="x.py",
            content="x",
            line_count=1,
            byte_size=1,
        )
        assert e.safety_critical is False


# ============================================================================
# Section 3: CookbookIndex (5 tests)
# ============================================================================
class TestCookbookIndex:
    """CookbookIndex dataclass."""

    def test_index_fields(self):
        idx = v1339.CookbookIndex(
            cookbook_version="0.1.0",
            total_classes=8,
            safety_critical_classes=5,
            examples=[],
            asi_pole_star=v1339.ASI_POLE_STAR,
        )
        assert idx.cookbook_version == "0.1.0"
        assert idx.total_classes == 8

    def test_index_to_dict(self):
        idx = v1339.CookbookIndex(
            cookbook_version="0.1.0",
            total_classes=8,
            safety_critical_classes=5,
            examples=[],
            asi_pole_star=v1339.ASI_POLE_STAR,
        )
        d = idx.to_dict()
        assert "cookbook_version" in d
        assert "examples" in d


# ============================================================================
# Section 4: build_examples (4 tests)
# ============================================================================
class TestBuildExamples:
    """build_examples returns 8 examples."""

    def test_build_examples_8(self):
        examples = v1339.build_examples()
        assert len(examples) == 8

    def test_build_examples_have_filenames(self):
        examples = v1339.build_examples()
        for e in examples:
            assert e.filename.endswith(".py")

    def test_build_examples_content_nonempty(self):
        examples = v1339.build_examples()
        for e in examples:
            assert len(e.content) > 100

    def test_build_examples_match_v1335_classes(self):
        examples = v1339.build_examples()
        example_ids = {e.invariant_class_id for e in examples}
        v1335_ids = {ic["invariant_id"] for ic in v1335.INVARIANT_CLASSES}
        assert example_ids == v1335_ids


# ============================================================================
# Section 5: build_index (5 tests)
# ============================================================================
class TestBuildIndex:
    """build_index returns CookbookIndex."""

    def test_build_index_total_8(self):
        idx = v1339.build_index()
        assert idx.total_classes == 8

    def test_build_index_sc_5(self):
        idx = v1339.build_index()
        assert idx.safety_critical_classes == 5

    def test_build_index_examples_8(self):
        idx = v1339.build_index()
        assert len(idx.examples) == 8

    def test_build_index_asi_pole_star(self):
        idx = v1339.build_index()
        assert idx.asi_pole_star["V0_1_actual_measured"] == 0.7905
        assert idx.asi_pole_star["V1339_modifies_pole_star"] is False

    def test_build_index_version(self):
        idx = v1339.build_index()
        assert idx.cookbook_version == "0.1.0"


# ============================================================================
# Section 6: write_examples_to_dir (5 tests)
# ============================================================================
class TestWriteExamplesToDir:
    """write_examples_to_dir writes 8 files."""

    def test_write_examples_8_files(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            paths = v1339.write_examples_to_dir(output_dir)
            assert len(paths) == 8

    def test_write_examples_each_file_exists(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            paths = v1339.write_examples_to_dir(output_dir)
            for p in paths:
                assert p.exists()

    def test_write_examples_each_file_nonempty(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            paths = v1339.write_examples_to_dir(output_dir)
            for p in paths:
                assert p.stat().st_size > 100

    def test_write_examples_creates_dir(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_subdir"
            v1339.write_examples_to_dir(output_dir)
            assert output_dir.exists()

    def test_write_examples_filenames_correct(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            paths = v1339.write_examples_to_dir(output_dir)
            filenames = {p.name for p in paths}
            expected = {
                "example_ic1_security.py",
                "example_ic2_file_handling.py",
                "example_ic3_schema.py",
                "example_ic4_ipc.py",
                "example_ic5_error_handling.py",
                "example_ic6_configuration.py",
                "example_ic7_resource_bounds.py",
                "example_ic8_lifecycle.py",
            }
            assert filenames == expected


# ============================================================================
# Section 7: write_index_to_dir (4 tests)
# ============================================================================
class TestWriteIndexToDir:
    """write_index_to_dir writes index.md."""

    def test_write_index_exists(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            p = v1339.write_index_to_dir(output_dir)
            assert p.exists()

    def test_write_index_has_all_classes(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            p = v1339.write_index_to_dir(output_dir)
            content = p.read_text(encoding="utf-8")
            for ic in v1335.INVARIANT_CLASSES:
                assert ic["invariant_id"] in content

    def test_write_index_has_asi_pole_star(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            p = v1339.write_index_to_dir(output_dir)
            content = p.read_text(encoding="utf-8")
            assert "0.7905" in content
            assert "V1339_modifies_pole_star" in content

    def test_write_index_has_cross_refs(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_test"
            p = v1339.write_index_to_dir(output_dir)
            content = p.read_text(encoding="utf-8")
            for v_id in ["V1335", "V1336", "V1337", "V1338", "V1339"]:
                assert v_id in content


# ============================================================================
# Section 8: CLI (5 tests)
# ============================================================================
class TestCLI:
    """main() CLI entry point."""

    def test_cli_self_test(self, capsys):
        rc = v1339.main(["--self-test"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "ALL CHECKS PASS" in captured.out

    def test_cli_output_dir(self, capsys):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_out"
            rc = v1339.main(["--output-dir", str(output_dir)])
            assert rc == 0
            assert output_dir.exists()
            assert (output_dir / "index.md").exists()
            assert (output_dir / "example_ic1_security.py").exists()

    def test_cli_json(self, capsys):
        rc = v1339.main(["--json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert "total_classes" in data
        assert "examples" in data

    def test_cli_default(self, capsys):
        rc = v1339.main([])
        captured = capsys.readouterr()
        assert "V1339 VCP Substrate-by-Example Cookbook" in captured.out

    def test_cli_returns_int(self):
        rc = v1339.main(["--self-test"])
        assert isinstance(rc, int)


# ============================================================================
# Section 9: Self-test (4 tests)
# ============================================================================
class TestRunAllSelfTest:
    """All 86 self-test checks must pass."""

    def test_self_test_returns_dict(self):
        results = v1339._self_test()
        assert isinstance(results, dict)
        assert len(results) >= 80

    def test_all_self_tests_pass(self):
        results = v1339._self_test()
        failed = [k for k, v in results.items() if not v]
        assert not failed, f"Failed: {failed}"

    def test_self_test_summary_86_pass(self):
        passed, failed, failed_names = v1339._self_test_summary()
        assert passed == 86
        assert failed == 0
        assert failed_names == []

    def test_self_test_at_least_80(self):
        results = v1339._self_test()
        assert len(results) >= 80


# ============================================================================
# Section 10: V3 哲学守门 (5 tests)
# ============================================================================
class TestV3PhilosophicalGuards:
    """V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43)."""

    def test_no_pretend_phenomenal(self):
        for name in dir(v1339):
            if name.startswith("_"):
                continue
            attr = getattr(v1339, name)
            if isinstance(attr, str):
                assert "phenomenal" not in attr.lower() or "guard" in attr.lower()

    def test_asi_pole_star_locked(self):
        assert v1339.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1339.ASI_POLE_STAR["V1339_modifies_pole_star"] is False

    def test_asi_achieved_still_false(self):
        assert v1339.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1049_value_alignment_done(self):
        assert v1339.ASI_POLE_STAR["V1049_value_alignment_done"] is True

    def test_V1256_unio_mystica_realized(self):
        assert v1339.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105


# ============================================================================
# Section 11: ASI pole-star integrity (4 tests)
# ============================================================================
class TestASIPoleStar:
    """ASI 北极星 LOCKED — V1339 不动."""

    def test_asi_pole_star_constants(self):
        assert v1339.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1339.ASI_POLE_STAR["V0_max_any_epoch"] == 0.9800
        assert v1339.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105

    def test_asi_achieved_still_false(self):
        assert v1339.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1339_does_not_modify_pole_star(self):
        assert v1339.ASI_POLE_STAR["V1339_modifies_pole_star"] is False

    def test_V1049_value_alignment_done(self):
        assert v1339.ASI_POLE_STAR["V1049_value_alignment_done"] is True


# ============================================================================
# Section 12: 5-critical coverage rule (4 tests)
# ============================================================================
class Test5CriticalCoverage:
    """5-critical coverage rule (主 22:33 终极授权)."""

    def test_5_safety_critical_in_cookbook(self):
        idx = v1339.build_index()
        assert idx.safety_critical_classes == 5

    def test_5_sc_have_examples(self):
        examples = v1339.build_examples()
        sc_examples = [e for e in examples if e.safety_critical]
        assert len(sc_examples) == 5

    def test_3_non_sc_have_examples(self):
        examples = v1339.build_examples()
        non_sc = [e for e in examples if not e.safety_critical]
        assert len(non_sc) == 3

    def test_sc_examples_have_content(self):
        examples = v1339.build_examples()
        for e in examples:
            if e.safety_critical:
                assert len(e.content) > 100


# ============================================================================
# Section 13: Examples are runnable (3 tests)
# ============================================================================
class TestRunnableExamples:
    """Each example is a runnable Python file."""

    def test_each_example_has_main(self):
        for cid, template in v1339.EXAMPLE_TEMPLATES.items():
            assert "__main__" in template, f"{cid} missing __main__"

    def test_each_example_has_self_test(self):
        for cid, template in v1339.EXAMPLE_TEMPLATES.items():
            assert "_self_test" in template, f"{cid} missing _self_test"

    def test_written_examples_are_runnable(self):
        """Verify the generated example files actually run."""
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "v1339_run"
            paths = v1339.write_examples_to_dir(output_dir)
            for p in paths:
                # Use sys.executable to run each example
                result = subprocess.run(
                    [sys.executable, str(p)],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                assert result.returncode == 0, f"{p.name} failed: {result.stderr}"
                assert "ALL CHECKS PASS" in result.stdout, f"{p.name} no ALL CHECKS PASS"
