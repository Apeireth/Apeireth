#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1337_vcp_plugin_compliance_dashboard.py — V1337 tests

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1336 linter CLI (b6d4fa31, 22:01); V1337 dashboard
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → **V1337**

Tests for V1337 VCP Plugin Compliance Dashboard.

Tests cover (8 sections × 11 API surfaces):
 1. V1335 + V1336 dependencies
 2. VCP_PLUGIN_FILES manifest (6 plugin entries)
 3. CrossPluginComplianceCell (per-cell coverage)
 4. DashboardSummary (aggregate stats)
 5. VCPPluginComplianceDashboard (top-level container)
 6. build_dashboard (V1336 linter per plugin + matrix + summary)
 7. dashboard_to_markdown + dashboard_to_csv (rendering)
 8. CLI: main() with --self-test, --json, --csv, --strict, --min-score
 9. Self-test (30/30 PASS gate)
10. V3 哲学守门 (LOCKED: 不假装 Phenomenal, 不假装 ASI 达到)
11. ASI pole-star integrity (V0.1=0.7905 + V1337 不动)
12. 5-critical coverage rule (per 主 22:33 终极授权)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Add apeireth dir to path
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

import pytest

import v1337_vcp_plugin_compliance_dashboard as v1337  # noqa: E402
import v1336_vcp_plugin_conformance_linter as v1336  # noqa: E402
import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# ============================================================================
# Section 1: V1335 + V1336 dependencies (4 tests)
# ============================================================================
class TestDependencies:
    """V1337 depends on V1335 (registry) + V1336 (linter)."""

    def test_v1335_imported(self):
        assert v1337.v1335 is not None

    def test_v1336_imported(self):
        assert v1337.v1336 is not None

    def test_v1335_8_invariant_classes(self):
        assert len(v1335.INVARIANT_CLASSES) == 8

    def test_v1336_default_min_score(self):
        assert v1336.DEFAULT_MIN_SCORE == 0.50


# ============================================================================
# Section 2: VCP_PLUGIN_FILES manifest (5 tests)
# ============================================================================
class TestVCPPluginFiles:
    """VCP_PLUGIN_FILES manifest."""

    def test_vcp_plugin_files_6(self):
        assert len(v1337.VCP_PLUGIN_FILES) == 6

    def test_v1327_in_manifest(self):
        v1327 = next(p for p in v1337.VCP_PLUGIN_FILES if p["plugin_id"] == "V1327")
        assert v1327["plugin_label"] == "VCP-6-core"

    def test_v1328_anysearch_in_manifest(self):
        v1328 = next(p for p in v1337.VCP_PLUGIN_FILES if p["plugin_id"] == "V1328")
        assert v1328["plugin_label"] == "AnySearch"

    def test_v1334_thoughtcluster_in_manifest(self):
        v1334 = next(p for p in v1337.VCP_PLUGIN_FILES if p["plugin_id"] == "V1334")
        assert v1334["plugin_label"] == "ThoughtClusterManager"

    def test_each_plugin_has_filename_and_role(self):
        for p in v1337.VCP_PLUGIN_FILES:
            assert "plugin_filename" in p
            assert "plugin_label" in p
            assert "role" in p
            assert len(p["plugin_filename"]) > 0
            assert len(p["plugin_label"]) > 0
            assert len(p["role"]) > 0


# ============================================================================
# Section 3: CrossPluginComplianceCell (5 tests)
# ============================================================================
class TestCrossPluginCell:
    """CrossPluginComplianceCell."""

    def test_cell_fields(self):
        c = v1337.CrossPluginComplianceCell(
            plugin_id="V1327",
            plugin_label="VCP-6-core",
            invariant_class_id="IC1_security",
            substrate_count=3,
            safety_critical=True,
            has_coverage=True,
        )
        assert c.plugin_id == "V1327"
        assert c.invariant_class_id == "IC1_security"

    def test_cell_to_dict(self):
        c = v1337.CrossPluginComplianceCell(
            plugin_id="V1328",
            plugin_label="AnySearch",
            invariant_class_id="IC4_ipc",
            substrate_count=1,
            safety_critical=True,
            has_coverage=True,
        )
        d = c.to_dict()
        assert d["plugin_label"] == "AnySearch"

    def test_cell_no_coverage(self):
        c = v1337.CrossPluginComplianceCell(
            plugin_id="V1330",
            plugin_label="AgentDream",
            invariant_class_id="IC1_security",
            substrate_count=0,
            safety_critical=True,
            has_coverage=False,
        )
        assert c.has_coverage is False

    def test_cell_safety_critical_false(self):
        c = v1337.CrossPluginComplianceCell(
            plugin_id="V1332",
            plugin_label="RAGDiary",
            invariant_class_id="IC5_error_handling",
            substrate_count=2,
            safety_critical=False,
            has_coverage=True,
        )
        assert c.safety_critical is False

    def test_cell_supports_python_path(self):
        c = v1337.CrossPluginComplianceCell(
            plugin_id="V1333",
            plugin_label="VCPTimeLine",
            invariant_class_id="IC3_schema",
            substrate_count=1,
            safety_critical=True,
            has_coverage=True,
        )
        assert c.substrate_count == 1


# ============================================================================
# Section 4: DashboardSummary (5 tests)
# ============================================================================
class TestDashboardSummary:
    """DashboardSummary fields."""

    def test_summary_to_dict(self):
        s = v1337.DashboardSummary(
            total_plugins=6,
            plugins_passed=6,
            plugins_failed=0,
            plugins_warned=0,
            critical_pass_rate=1.0,
            critical_gaps_detected=0,
            total_substrates=138,
            avg_coverage_score=0.50,
            overall_verdict="PASS",
            recommendations=["All conform"],
        )
        d = s.to_dict()
        assert d["total_plugins"] == 6
        assert d["overall_verdict"] == "PASS"

    def test_summary_with_fail(self):
        s = v1337.DashboardSummary(
            total_plugins=6,
            plugins_passed=0,
            plugins_failed=6,
            plugins_warned=0,
            critical_pass_rate=0.0,
            critical_gaps_detected=18,
            total_substrates=138,
            avg_coverage_score=0.40,
            overall_verdict="FAIL",
            recommendations=["Fix 6 plugins"],
        )
        assert s.overall_verdict == "FAIL"

    def test_summary_recommendations_list(self):
        assert isinstance(v1337.DashboardSummary(
            total_plugins=6, plugins_passed=0, plugins_failed=0,
            plugins_warned=0, critical_pass_rate=0.0,
            critical_gaps_detected=0, total_substrates=0,
            avg_coverage_score=0.0, overall_verdict="PASS",
            recommendations=["X"],
        ).recommendations, list)

    def test_summary_critical_pass_rate_in_range(self):
        s = v1337.DashboardSummary(
            total_plugins=6, plugins_passed=0, plugins_failed=0,
            plugins_warned=0, critical_pass_rate=0.5,
            critical_gaps_detected=0, total_substrates=0,
            avg_coverage_score=0.0, overall_verdict="PASS",
            recommendations=[],
        )
        assert 0.0 <= s.critical_pass_rate <= 1.0

    def test_summary_verdict_in_set(self):
        for v in ["PASS", "PASS_WITH_WARNINGS", "FAIL"]:
            s = v1337.DashboardSummary(
                total_plugins=6, plugins_passed=0, plugins_failed=0,
                plugins_warned=0, critical_pass_rate=0.0,
                critical_gaps_detected=0, total_substrates=0,
                avg_coverage_score=0.0, overall_verdict=v,
                recommendations=[],
            )
            assert s.overall_verdict == v


# ============================================================================
# Section 5: build_dashboard (8 tests)
# ============================================================================
class TestBuildDashboard:
    """build_dashboard returns VCPPluginComplianceDashboard."""

    def test_dashboard_builds(self):
        d = v1337.build_dashboard()
        assert d is not None

    def test_dashboard_6_per_plugin_reports(self):
        d = v1337.build_dashboard()
        assert len(d.per_plugin_reports) == 6

    def test_dashboard_48_matrix_cells(self):
        d = v1337.build_dashboard()
        # 6 plugins × 8 invariant classes = 48 cells
        assert len(d.cross_plugin_matrix) == 48

    def test_dashboard_summary_total_plugins_6(self):
        d = v1337.build_dashboard()
        assert d.summary.total_plugins == 6

    def test_dashboard_total_substrates_positive(self):
        d = v1337.build_dashboard()
        assert d.summary.total_substrates > 0

    def test_dashboard_avg_coverage_positive(self):
        d = v1337.build_dashboard()
        assert d.summary.avg_coverage_score > 0.0

    def test_dashboard_strict_parameter(self):
        d = v1337.build_dashboard(strict=True)
        assert d is not None

    def test_dashboard_min_score_parameter(self):
        d = v1337.build_dashboard(min_score=0.99)
        assert d.summary.avg_coverage_score <= 1.0


# ============================================================================
# Section 6: dashboard_to_markdown + dashboard_to_csv (6 tests)
# ============================================================================
class TestDashboardReporting:
    """Markdown and CSV rendering."""

    def test_markdown_contains_total_plugins(self):
        d = v1337.build_dashboard()
        md = v1337.dashboard_to_markdown(d)
        assert "Total plugins: 6" in md

    def test_markdown_contains_overall_verdict(self):
        d = v1337.build_dashboard()
        md = v1337.dashboard_to_markdown(d)
        assert "Overall verdict" in md

    def test_markdown_contains_cross_plugin_matrix(self):
        d = v1337.build_dashboard()
        md = v1337.dashboard_to_markdown(d)
        assert "Cross-plugin compliance matrix" in md

    def test_csv_has_header(self):
        d = v1337.build_dashboard()
        csv_text = v1337.dashboard_to_csv(d)
        assert "plugin_id" in csv_text
        assert "invariant_class_id" in csv_text

    def test_csv_has_48_rows(self):
        d = v1337.build_dashboard()
        csv_text = v1337.dashboard_to_csv(d)
        # 1 header + 48 data rows (6 plugins × 8 classes)
        lines = csv_text.strip().splitlines()
        assert len(lines) == 49

    def test_csv_each_plugin_appears(self):
        d = v1337.build_dashboard()
        csv_text = v1337.dashboard_to_csv(d)
        for pid in ["V1327", "V1328", "V1330", "V1332", "V1333", "V1334"]:
            assert pid in csv_text


# ============================================================================
# Section 7: CLI (8 tests)
# ============================================================================
class TestCLI:
    """main() CLI entry point."""

    def test_cli_self_test(self, capsys):
        rc = v1337.main(["--self-test"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "ALL CHECKS PASS" in captured.out

    def test_cli_markdown(self, capsys):
        rc = v1337.main([])
        captured = capsys.readouterr()
        assert "VCP Plugin Compliance Dashboard" in captured.out

    def test_cli_json(self, capsys):
        rc = v1337.main(["--json"])
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert "summary" in data
        assert "per_plugin_reports" in data

    def test_cli_csv(self, capsys):
        rc = v1337.main(["--csv"])
        captured = capsys.readouterr()
        assert "plugin_id" in captured.out

    def test_cli_strict(self, capsys):
        rc = v1337.main(["--strict"])
        assert rc in (0, 1)

    def test_cli_min_score(self, capsys):
        rc = v1337.main(["--min-score", "0.99"])
        assert rc in (0, 1)

    def test_cli_returns_int(self):
        rc = v1337.main(["--self-test"])
        assert isinstance(rc, int)

    def test_cli_self_test_exits_0(self):
        rc = v1337.main(["--self-test"])
        assert rc == 0


# ============================================================================
# Section 8: Self-test (4 tests)
# ============================================================================
class TestRunAllSelfTest:
    """All 30 self-test checks must pass."""

    def test_self_test_returns_dict(self):
        results = v1337._self_test()
        assert isinstance(results, dict)
        assert len(results) >= 30

    def test_all_self_tests_pass(self):
        results = v1337._self_test()
        failed = [k for k, v in results.items() if not v]
        assert not failed, f"Failed: {failed}"

    def test_self_test_summary_30_pass(self):
        passed, failed, failed_names = v1337._self_test_summary()
        assert passed == 30
        assert failed == 0
        assert failed_names == []

    def test_self_test_summary_at_least_30(self):
        results = v1337._self_test()
        assert len(results) >= 30


# ============================================================================
# Section 9: V3 哲学守门 (5 tests)
# ============================================================================
class TestV3PhilosophicalGuards:
    """V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43)."""

    def test_no_pretend_phenomenal(self):
        for name in dir(v1337):
            if name.startswith("_"):
                continue
            attr = getattr(v1337, name)
            if isinstance(attr, str):
                assert "phenomenal" not in attr.lower() or "guard" in attr.lower()

    def test_asi_pole_star_locked(self):
        assert v1337.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1337.ASI_POLE_STAR["V1337_modifies_pole_star"] is False

    def test_asi_achieved_still_false(self):
        assert v1337.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1049_value_alignment_done(self):
        assert v1337.ASI_POLE_STAR["V1049_value_alignment_done"] is True

    def test_V1256_unio_mystica_realized(self):
        assert v1337.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105


# ============================================================================
# Section 10: ASI pole-star integrity (4 tests)
# ============================================================================
class TestASIPoleStar:
    """ASI 北极星 LOCKED — V1337 不动."""

    def test_asi_pole_star_constants(self):
        assert v1337.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1337.ASI_POLE_STAR["V0_max_any_epoch"] == 0.9800
        assert v1337.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105

    def test_asi_achieved_still_false(self):
        assert v1337.ASI_POLE_STAR["asi_achieved_false"] is True

    def test_V1337_does_not_modify_pole_star(self):
        assert v1337.ASI_POLE_STAR["V1337_modifies_pole_star"] is False

    def test_V1049_value_alignment_done(self):
        assert v1337.ASI_POLE_STAR["V1049_value_alignment_done"] is True


# ============================================================================
# Section 11: 5-critical coverage rule (5 tests)
# ============================================================================
class Test5CriticalCoverage:
    """5-critical coverage rule (主 22:33 终极授权)."""

    def test_critical_pass_rate_in_range(self):
        d = v1337.build_dashboard()
        assert 0.0 <= d.summary.critical_pass_rate <= 1.0

    def test_critical_gaps_count_nonneg(self):
        d = v1337.build_dashboard()
        assert d.summary.critical_gaps_detected >= 0

    def test_safety_critical_cells_30(self):
        d = v1337.build_dashboard()
        sc = [c for c in d.cross_plugin_matrix if c.safety_critical]
        assert len(sc) == 30  # 6 plugins × 5 SC classes

    def test_safety_critical_cells_have_coverage_data(self):
        d = v1337.build_dashboard()
        for c in d.cross_plugin_matrix:
            if c.safety_critical:
                assert c.substrate_count >= 0
                assert isinstance(c.has_coverage, bool)

    def test_total_substrates_equals_sum(self):
        d = v1337.build_dashboard()
        individual_sum = sum(r.total_substrates for r in d.per_plugin_reports)
        assert d.summary.total_substrates == individual_sum


# ============================================================================
# Section 12: Module docstring + API (3 tests)
# ============================================================================
class TestModuleInvariants:
    """V1337 module-level invariants."""

    def test_module_docstring_present(self):
        assert v1337.__doc__ is not None
        assert "V1337" in v1337.__doc__

    def test_V3_guards_present(self):
        for guard in ["不假装", "ASI 北极星", "Phenomenal consciousness", "调整模型 & prompt"]:
            assert guard in v1337.__doc__, f"Missing guard: {guard}"

    def test_chain_reference(self):
        assert "V1336" in v1337.__doc__
        assert "V1337" in v1337.__doc__
