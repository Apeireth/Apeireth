#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1338_vcp_plugin_migration_tool.py — V1338 tests

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1337 dashboard (1aae1765, 22:01); V1338 migration tool
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → V1337 → **V1338**

Tests for V1338 VCP Plugin Migration Tool.

Tests cover (10 sections × 12 API surfaces):
 1. V1335 + V1336 + V1337 dependencies
 2. SubstrateSuggestion (target class + suggested names + skeleton template)
 3. MigrationRecommendation (original state + projected state + suggestions)
 4. _skeleton_template_for_class (8 templates, one per invariant class)
 5. _compute_projected_coverage (coverage score math)
 6. migrate_plugin_file (single file → migration recommendation)
 7. migrate_plugin_files (multiple files → list)
 8. recommendation_to_markdown (rendering)
 9. CLI: main() with --self-test, --json, --markdown
10. Self-test (28/28 PASS gate)
11. V3 哲学守门 (LOCKED: 不假装 Phenomenal, 不假装 ASI 达到)
12. ASI pole-star integrity (V0.1=0.7905 + V1338 不动)
13. 5-critical coverage rule (projected_pass_5_critical)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Add apeireth dir to path
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

import pytest

import v1338_vcp_plugin_migration_tool as v1338  # noqa: E402
import v1337_vcp_plugin_compliance_dashboard as v1337  # noqa: E402
import v1336_vcp_plugin_conformance_linter as v1336  # noqa: E402
import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# ============================================================================
# Section 1: V1335 + V1336 + V1337 dependencies (4 tests)
# ============================================================================
class TestDependencies:
    """V1338 depends on V1335 + V1336 + V1337."""

    def test_v1335_imported(self):
        assert v1338.v1335 is not None

    def test_v1336_imported(self):
        assert v1338.v1336 is not None

    def test_v1337_imported(self):
        assert v1338.v1337 is not None

    def test_v1335_8_invariant_classes(self):
        assert len(v1335.INVARIANT_CLASSES) == 8


# ============================================================================
# Section 2: SubstrateSuggestion (5 tests)
# ============================================================================
class TestSubstrateSuggestion:
    """SubstrateSuggestion dataclass."""

    def test_substrate_suggestion_fields(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC1_security",
            target_invariant_label="SecurityInvariants",
            safety_critical=True,
            suggested_substrate_names=["PathSanitizationSubstrate"],
            skeleton_template="class X: pass\n",
        )
        assert s.target_invariant_class_id == "IC1_security"
        assert s.safety_critical is True

    def test_substrate_suggestion_to_dict(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC2_file_handling",
            target_invariant_label="FileHandlingInvariants",
            safety_critical=True,
            suggested_substrate_names=["AtomicJsonWriteSubstrate"],
            skeleton_template="class X: pass\n",
        )
        d = s.to_dict()
        assert "target_invariant_class_id" in d
        assert d["target_invariant_label"] == "FileHandlingInvariants"

    def test_substrate_suggestion_safety_critical_true(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC7_resource_bounds",
            target_invariant_label="ResourceBoundsInvariants",
            safety_critical=True,
            suggested_substrate_names=["truncate_to_token_budget"],
            skeleton_template="def x(): pass\n",
        )
        assert s.safety_critical is True

    def test_substrate_suggestion_safety_critical_false(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC8_lifecycle",
            target_invariant_label="LifecycleInvariants",
            safety_critical=False,
            suggested_substrate_names=["_self_test"],
            skeleton_template="def x(): pass\n",
        )
        assert s.safety_critical is False

    def test_substrate_suggestion_empty_names(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC5_error_handling",
            target_invariant_label="ErrorHandlingInvariants",
            safety_critical=False,
            suggested_substrate_names=[],
            skeleton_template="",
        )
        assert s.suggested_substrate_names == []


# ============================================================================
# Section 3: MigrationRecommendation (5 tests)
# ============================================================================
class TestMigrationRecommendation:
    """MigrationRecommendation fields."""

    def test_recommendation_fields(self):
        r = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="FAIL",
            original_coverage_score=0.0,
            original_classes_covered=[],
            original_critical_missing=["IC1_security"],
            suggestions=[],
            projected_coverage_score=0.2,
            projected_pass_5_critical=False,
            projected_classes_covered=["IC1_security"],
        )
        assert r.original_verdict == "FAIL"
        assert r.projected_pass_5_critical is False

    def test_recommendation_to_dict(self):
        r = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="PASS",
            original_coverage_score=1.0,
            original_classes_covered=["IC1_security"],
            original_critical_missing=[],
            suggestions=[],
            projected_coverage_score=1.0,
            projected_pass_5_critical=True,
            projected_classes_covered=["IC1_security"],
        )
        d = r.to_dict()
        assert "suggestions" in d
        assert "projected_pass_5_critical" in d

    def test_recommendation_empty_suggestions(self):
        r = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="PASS",
            original_coverage_score=1.0,
            original_classes_covered=["IC1_security"],
            original_critical_missing=[],
            suggestions=[],
            projected_coverage_score=1.0,
            projected_pass_5_critical=True,
            projected_classes_covered=["IC1_security"],
        )
        assert r.suggestions == []

    def test_recommendation_with_suggestions(self):
        s = v1338.SubstrateSuggestion(
            target_invariant_class_id="IC1_security",
            target_invariant_label="SecurityInvariants",
            safety_critical=True,
            suggested_substrate_names=["PathSanitizationSubstrate"],
            skeleton_template="class X: pass\n",
        )
        r = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="FAIL",
            original_coverage_score=0.0,
            original_classes_covered=[],
            original_critical_missing=["IC1_security"],
            suggestions=[s],
            projected_coverage_score=0.2,
            projected_pass_5_critical=False,
            projected_classes_covered=["IC1_security"],
        )
        assert len(r.suggestions) == 1

    def test_recommendation_project_score_in_range(self):
        r = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="FAIL",
            original_coverage_score=0.4,
            original_classes_covered=["IC1_security"],
            original_critical_missing=["IC2_file_handling"],
            suggestions=[],
            projected_coverage_score=0.4,
            projected_pass_5_critical=False,
            projected_classes_covered=["IC1_security"],
        )
        assert 0.0 <= r.projected_coverage_score <= 1.0


# ============================================================================
# Section 4: _skeleton_template_for_class (10 tests)
# ============================================================================
class TestSkeletonTemplate:
    """_skeleton_template_for_class returns minimal Python template."""

    def test_IC1_security_skeleton(self):
        t = v1338._skeleton_template_for_class("IC1_security", "SecurityInvariants")
        assert "class" in t
        assert "PathSanitizationSubstrate" in t

    def test_IC2_file_handling_skeleton(self):
        t = v1338._skeleton_template_for_class("IC2_file_handling", "FileHandlingInvariants")
        assert "class" in t
        assert "AtomicJsonWriteSubstrate" in t

    def test_IC3_schema_skeleton(self):
        t = v1338._skeleton_template_for_class("IC3_schema", "SchemaInvariants")
        assert "manifestVersion" in t

    def test_IC4_ipc_skeleton(self):
        t = v1338._skeleton_template_for_class("IC4_ipc", "IPCProtocolInvariants")
        assert "jsonrpc" in t.lower() or "JSON-RPC" in t

    def test_IC5_error_handling_skeleton(self):
        t = v1338._skeleton_template_for_class("IC5_error_handling", "ErrorHandlingInvariants")
        assert "success" in t
        assert "error" in t

    def test_IC6_configuration_skeleton(self):
        t = v1338._skeleton_template_for_class("IC6_configuration", "ConfigurationInvariants")
        assert "merge" in t or "config" in t.lower()

    def test_IC7_resource_bounds_skeleton(self):
        t = v1338._skeleton_template_for_class("IC7_resource_bounds", "ResourceBoundsInvariants")
        assert "token" in t.lower() or "budget" in t.lower()

    def test_IC8_lifecycle_skeleton(self):
        t = v1338._skeleton_template_for_class("IC8_lifecycle", "LifecycleInvariants")
        assert "_self_test" in t or "self_test" in t

    def test_unknown_class_returns_todo(self):
        t = v1338._skeleton_template_for_class("IC999", "Unknown")
        assert "TODO" in t

    def test_all_8_classes_have_templates(self):
        for ic in v1335.INVARIANT_CLASSES:
            t = v1338._skeleton_template_for_class(ic["invariant_id"], ic["label"])
            assert len(t) > 0


# ============================================================================
# Section 5: _compute_projected_coverage (5 tests)
# ============================================================================
class TestProjectedCoverage:
    """_compute_projected_coverage."""

    def test_project_no_new_classes(self):
        score = v1338._compute_projected_coverage(["IC1_security"], [])
        assert score == 0.2  # 1 of 5 SC classes

    def test_project_full_5_critical(self):
        score = v1338._compute_projected_coverage(
            ["IC1_security"],
            ["IC2_file_handling", "IC3_schema", "IC4_ipc", "IC7_resource_bounds"],
        )
        assert score == 1.0

    def test_project_empty(self):
        score = v1338._compute_projected_coverage([], [])
        assert score == 0.0

    def test_project_no_sc_classes(self):
        score = v1338._compute_projected_coverage([], ["IC8_lifecycle"])
        # IC8_lifecycle is not safety-critical
        assert score == 0.0

    def test_project_all_5_already_covered(self):
        all_5 = ["IC1_security", "IC2_file_handling", "IC3_schema", "IC4_ipc", "IC7_resource_bounds"]
        score = v1338._compute_projected_coverage(all_5, [])
        assert score == 1.0


# ============================================================================
# Section 6: migrate_plugin_file (6 tests)
# ============================================================================
class TestMigratePluginFile:
    """migrate_plugin_file."""

    def test_migrate_v1335_works(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        assert rec is not None

    def test_migrate_v1335_has_suggestions(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        # V1335 has 0.2 coverage, so should have ≥1 suggestion
        assert len(rec.suggestions) > 0

    def test_migrate_v1335_original_verdict_fail(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        assert rec.original_verdict == "FAIL"

    def test_migrate_v1335_projected_pass_5_critical_true(self):
        """If all 5 critical classes are filled, projected pass should be True."""
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        # V1335 is missing 4 critical classes (IC1, IC3, IC4, IC7)
        # After filling all 4, projected_pass_5_critical should be True
        assert rec.projected_pass_5_critical is True

    def test_migrate_missing_file_verdict_fail(self):
        fake = APEIRETH_DIR / "v9999_does_not_exist.py"
        rec = v1338.migrate_plugin_file(fake)
        assert rec.original_verdict == "FAIL"

    def test_migrate_recommendation_path(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        assert rec.plugin_filename == p.name


# ============================================================================
# Section 7: migrate_plugin_files (3 tests)
# ============================================================================
class TestMigratePluginFiles:
    """migrate_plugin_files."""

    def test_migrate_multiple_returns_n(self):
        paths = [
            APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py",
            APEIRETH_DIR / "v1334_thoughtclustermanager_plugin_deep_read.py",
        ]
        recs = v1338.migrate_plugin_files(paths)
        assert len(recs) == 2

    def test_migrate_each_path(self):
        paths = [
            APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py",
            APEIRETH_DIR / "v1333_vcptimeline_plugin_deep_read.py",
            APEIRETH_DIR / "v1332_ragdiary_plugin_deep_read.py",
        ]
        recs = v1338.migrate_plugin_files(paths)
        assert len(recs) == 3
        for i, rec in enumerate(recs):
            assert rec.plugin_filename == paths[i].name

    def test_migrate_empty_list(self):
        recs = v1338.migrate_plugin_files([])
        assert recs == []


# ============================================================================
# Section 8: recommendation_to_markdown (5 tests)
# ============================================================================
class TestRecommendationToMarkdown:
    """recommendation_to_markdown rendering."""

    def test_md_contains_filename(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        md = v1338.recommendation_to_markdown(rec)
        assert "v1335_vcp_cross_plugin_invariant_synthesis.py" in md

    def test_md_contains_projected_state(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        md = v1338.recommendation_to_markdown(rec)
        assert "Projected state" in md

    def test_md_contains_skeleton_template(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        md = v1338.recommendation_to_markdown(rec)
        assert "```python" in md

    def test_md_contains_suggestions(self):
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        md = v1338.recommendation_to_markdown(rec)
        assert "Migration suggestions" in md

    def test_md_no_suggestions_message(self):
        rec = v1338.MigrationRecommendation(
            plugin_path="x.py",
            plugin_filename="x.py",
            original_verdict="PASS",
            original_coverage_score=1.0,
            original_classes_covered=["IC1_security"],
            original_critical_missing=[],
            suggestions=[],
            projected_coverage_score=1.0,
            projected_pass_5_critical=True,
            projected_classes_covered=["IC1_security"],
        )
        md = v1338.recommendation_to_markdown(rec)
        assert "No suggestions needed" in md


# ============================================================================
# Section 9: CLI (5 tests)
# ============================================================================
class TestCLI:
    """main() CLI entry point."""

    def test_cli_self_test(self, capsys):
        rc = v1338.main(["--self-test"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "ALL CHECKS PASS" in captured.out

    def test_cli_lint_v1335(self, capsys):
        rc = v1338.main(["apeireth/v1335_vcp_cross_plugin_invariant_synthesis.py"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "Migration" in captured.out

    def test_cli_lint_json(self, capsys):
        rc = v1338.main(["apeireth/v1335_vcp_cross_plugin_invariant_synthesis.py", "--json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert isinstance(data, list)

    def test_cli_lint_with_markdown(self, capsys):
        rc = v1338.main(["apeireth/v1335_vcp_cross_plugin_invariant_synthesis.py", "--markdown"])
        captured = capsys.readouterr()
        assert "Migration" in captured.out

    def test_cli_no_files_error(self, capsys):
        rc = v1338.main([])
        assert rc == 1


# ============================================================================
# Section 10: Self-test (4 tests)
# ============================================================================
class TestRunAllSelfTest:
    """All 28 self-test checks must pass."""

    def test_self_test_returns_dict(self):
        results = v1338._self_test()
        assert isinstance(results, dict)
        assert len(results) >= 28

    def test_all_self_tests_pass(self):
        results = v1338._self_test()
        failed = [k for k, v in results.items() if not v]
        assert not failed, f"Failed: {failed}"

    def test_self_test_summary_28_pass(self):
        passed, failed, failed_names = v1338._self_test_summary()
        assert passed == 28
        assert failed == 0
        assert failed_names == []

    def test_self_test_at_least_28(self):
        results = v1338._self_test()
        assert len(results) >= 28


# ============================================================================
# Section 11: V3 哲学守门 (5 tests)
# ============================================================================
class TestV3PhilosophicalGuards:
    """V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43)."""

    def test_no_pretend_phenomenal(self):
        for name in dir(v1338):
            if name.startswith("_"):
                continue
            attr = getattr(v1338, name)
            if isinstance(attr, str):
                assert "phenomenal" not in attr.lower() or "guard" in attr.lower()

    def test_asi_pole_star_locked(self):
        assert v1338.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1338.ASI_POLE_STAR["V1338_modifies_pole_star"] is False

    def test_asi_achieved_still_false(self):
        assert v1338.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1049_value_alignment_done(self):
        assert v1338.ASI_POLE_STAR["V1049_value_alignment_done"] is True

    def test_V1256_unio_mystica_realized(self):
        assert v1338.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105


# ============================================================================
# Section 12: ASI pole-star integrity (4 tests)
# ============================================================================
class TestASIPoleStar:
    """ASI 北极星 LOCKED — V1338 不动."""

    def test_asi_pole_star_constants(self):
        assert v1338.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1338.ASI_POLE_STAR["V0_max_any_epoch"] == 0.9800
        assert v1338.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105

    def test_asi_achieved_still_false(self):
        assert v1338.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1338_does_not_modify_pole_star(self):
        assert v1338.ASI_POLE_STAR["V1338_modifies_pole_star"] is False

    def test_V1049_value_alignment_done(self):
        assert v1338.ASI_POLE_STAR["V1049_value_alignment_done"] is True


# ============================================================================
# Section 13: 5-critical coverage rule (3 tests)
# ============================================================================
class Test5CriticalCoverage:
    """5-critical coverage rule (主 22:33 终极授权)."""

    def test_project_v1335_passes_5_critical(self):
        """V1335 fills all 5 critical classes after migration."""
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        assert rec.projected_pass_5_critical is True

    def test_v1338_projects_correctly_for_5_critical(self):
        """Projected coverage after filling all 5 SC classes = 1.0."""
        p = APEIRETH_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
        rec = v1338.migrate_plugin_file(p)
        # If projected passes 5-critical, projected score should be 1.0
        if rec.projected_pass_5_critical:
            assert rec.projected_coverage_score == 1.0

    def test_min_5_critical_coverage_rule(self):
        """Always 5 SC classes expected."""
        assert len(v1336._expected_safety_critical_classes()) == 5


# ============================================================================
# Section 14: Module docstring + API (3 tests)
# ============================================================================
class TestModuleInvariants:
    """V1338 module-level invariants."""

    def test_module_docstring_present(self):
        assert v1338.__doc__ is not None
        assert "V1338" in v1338.__doc__

    def test_V3_guards_present(self):
        for guard in ["不假装", "ASI 北极星", "Phenomenal consciousness", "调整模型 & prompt"]:
            assert guard in v1338.__doc__, f"Missing guard: {guard}"

    def test_chain_reference(self):
        assert "V1337" in v1338.__doc__
        assert "V1338" in v1338.__doc__
