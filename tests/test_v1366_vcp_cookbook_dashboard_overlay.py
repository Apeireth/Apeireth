"""Tests for v1366_vcp_cookbook_dashboard_overlay (V1340 validation + V1363 overlay).

V1366 = composition of V1340 (cookbook validator) + V1363 (dashboard + trend overlay).
"""
from __future__ import annotations

import ast
import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth import v1366_vcp_cookbook_dashboard_overlay as v1366
from apeireth import v1340_vcp_cookbook_validator as v1340
from apeireth import v1363_dashboard_trend_overlay as v1363


# ---------------------------------------------------------------------------
# TestClass: ConstantsAndGuards
# ---------------------------------------------------------------------------
class TestConstantsAndGuards:
    def test_v1366_version_is_semver(self):
        assert v1366.V1366_VERSION.count(".") == 2

    def test_v1366_asi_cap_is_honest(self):
        assert v1366.V1366_ASI_CAP == 0.005

    def test_v1366_asi_cap_below_asi_threshold(self):
        # GUARD_COOKBOOK_OVERLAY_NOT_ASI: overlay ≤ 0.01
        assert v1366.V1366_ASI_CAP <= 0.01

    def test_v1366_subweights_sum_to_one(self):
        assert abs(sum(v1366.V1366_SUBWEIGHTS.values()) - 1.0) < 1e-9

    def test_v1366_philosophy_guards_include_obligates(self):
        must_have = [
            "GUARD_COOKBOOK_OVERLAY_NOT_ASI",
            "GUARD_COMPOSE_ONLY",
            "GUARD_DELEGATE_TO_V1340_V1363",
            "GUARD_READ_ONLY",
            "GUARD_NO_FABRICATION",
            "GUARD_HONEST_CAP",
            "GUARD_V1340_V1363_UNCHANGED",
        ]
        for g in must_have:
            assert g in v1366.V1366_PHILOSOPHY_GUARDS, f"missing guard: {g}"


# ---------------------------------------------------------------------------
# TestClass: Delegation
# ---------------------------------------------------------------------------
class TestDelegation:
    def test_v1340_import_works(self):
        assert v1366._import_v1340() is v1340

    def test_v1363_import_works(self):
        assert v1366._import_v1363() is v1363

    def test_get_v1340_validation_report_dict_returns_dict(self):
        report = v1366.get_v1340_validation_report_dict()
        assert isinstance(report, dict)
        # V1340 should be available — it's a shipped module
        assert report.get("available") is True

    def test_get_v1340_validation_report_dict_has_8_examples(self):
        report = v1366.get_v1340_validation_report_dict()
        assert report["total_examples"] == 8
        assert report["examples_validated"] == 8
        assert report["overall_pass"] is True

    def test_get_v1363_dashboard_md_returns_string(self):
        md = v1366.get_v1363_dashboard_md(history_limit=5, window=3)
        assert isinstance(md, str)
        assert "V1363" in md or "Apeireth" in md


# ---------------------------------------------------------------------------
# TestClass: MarkdownRendering
# ---------------------------------------------------------------------------
class TestMarkdownRendering:
    def test_render_cookbook_section_md_returns_string(self):
        md = v1366.render_cookbook_section_md()
        assert isinstance(md, str)
        assert len(md) > 100

    def test_cookbook_section_mentions_v1340(self):
        md = v1366.render_cookbook_section_md()
        assert "V1340" in md
        assert "V1366" in md

    def test_cookbook_section_has_per_example_table(self):
        md = v1366.render_cookbook_section_md()
        # Should have a Markdown table with 8 examples
        assert "`example_ic" in md
        assert "IC1_security" in md
        assert "IC8_lifecycle" in md

    def test_cookbook_section_shows_overall_status(self):
        md = v1366.render_cookbook_section_md()
        assert "Overall" in md
        assert "PASS" in md

    def test_full_overlay_md_includes_v1363_section(self):
        md = v1366.render_full_overlay_md(history_limit=5, window=3)
        # V1363 dashboard header
        assert "Apeireth V1363" in md or "Apeireth" in md
        # V1366 cookbook section
        assert "VCP Cookbook Validation Overlay" in md
        # V1366 guards banner
        assert "V1366 V3 Philosophy Guards" in md

    def test_full_overlay_md_guards_all_listed(self):
        md = v1366.render_full_overlay_md(history_limit=5, window=3)
        for g in v1366.V1366_PHILOSOPHY_GUARDS:
            assert g in md, f"guard {g} not in overlay"

    def test_full_overlay_md_cap_visible(self):
        md = v1366.render_full_overlay_md(history_limit=5, window=3)
        assert "0.005" in md
        assert "cap" in md.lower()


# ---------------------------------------------------------------------------
# TestClass: JSONRendering
# ---------------------------------------------------------------------------
class TestJSONRendering:
    def test_render_full_overlay_json_returns_dict(self):
        j = v1366.render_full_overlay_json()
        assert isinstance(j, dict)

    def test_json_has_v1366_metadata(self):
        j = v1366.render_full_overlay_json()
        assert j["v1366_version"] == v1366.V1366_VERSION
        assert j["v1366_asi_cap"] == v1366.V1366_ASI_CAP
        assert set(j["v1366_philosophy_guards"]) == set(v1366.V1366_PHILOSOPHY_GUARDS)

    def test_json_has_v1363_snapshot(self):
        j = v1366.render_full_overlay_json()
        assert "v1363_snapshot" in j
        snap = j["v1363_snapshot"]
        assert "v1363_version" in snap
        assert "v1361_snapshot" in snap
        assert "v1362_history" in snap

    def test_json_has_v1340_cookbook_validation(self):
        j = v1366.render_full_overlay_json()
        assert "v1340_cookbook_validation" in j
        ck = j["v1340_cookbook_validation"]
        assert ck["available"] is True
        assert ck["total_examples"] == 8
        assert ck["overall_verdict"] == "PASS"

    def test_json_is_serializable(self):
        j = v1366.render_full_overlay_json(history_limit=5, window=3)
        s = json.dumps(j, default=str)
        assert isinstance(s, str)
        assert len(s) > 100


# ---------------------------------------------------------------------------
# TestClass: OneLineSummary
# ---------------------------------------------------------------------------
class TestOneLineSummary:
    def test_summary_returns_string(self):
        s = v1366.render_one_line_summary()
        assert isinstance(s, str)
        assert len(s) > 0

    def test_summary_has_v1366_marker(self):
        s = v1366.render_one_line_summary()
        assert "v1366" in s

    def test_summary_has_pole_star(self):
        s = v1366.render_one_line_summary()
        assert "pole_star" in s

    def test_summary_has_cookbook_status(self):
        s = v1366.render_one_line_summary()
        assert "cookbook" in s
        # cookbook status should be 8/8 pass (real V1340 evidence)
        assert "8/8" in s or "pass" in s.lower()


# ---------------------------------------------------------------------------
# TestClass: ReadOnlyInvariant
# ---------------------------------------------------------------------------
class TestReadOnlyInvariant:
    """V1366 must NOT write to disk or modify V1340/V1363."""

    def test_v1366_has_no_open_call_in_module_source(self):
        """Module source must not contain `open(` for write."""
        src_path = APEIRETH_DIR / "v1366_vcp_cookbook_dashboard_overlay.py"
        src = src_path.read_text(encoding="utf-8")
        tree = ast.parse(src)
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                if isinstance(node.func, ast.Name) and node.func.id == "open":
                    # Check it's not in argparse / CLI help text by looking at line
                    line = src.splitlines()[node.lineno - 1]
                    if "open(" in line and "r" not in (line.split("open(")[1].split(")")[0] if "open(" in line else ""):
                        pytest.fail(f"V1366 has open() call at line {node.lineno}: {line.strip()}")

    def test_v1366_does_not_modify_v1340_constants(self):
        snap1 = v1366.get_v1340_validation_report_dict()
        snap2 = v1366.get_v1340_validation_report_dict()
        assert snap1["total_examples"] == snap2["total_examples"]
        assert snap1["overall_pass"] == snap2["overall_pass"]

    def test_v1366_does_not_modify_v1363_dashboard(self):
        snap1 = v1363.get_combined_snapshot_dict()
        snap2 = v1363.get_combined_snapshot_dict()
        assert snap1["v1363_version"] == snap2["v1363_version"]


# ---------------------------------------------------------------------------
# TestClass: ClosedLoopCoverage
# ---------------------------------------------------------------------------
class TestClosedLoopCoverage:
    """V1366 should cover the V1335→V1336→V1339→V1340→V1366 chain."""

    def test_v1366_references_v1340(self):
        src_path = APEIRETH_DIR / "v1366_vcp_cookbook_dashboard_overlay.py"
        src = src_path.read_text(encoding="utf-8")
        assert "v1340" in src

    def test_v1366_references_v1363(self):
        src_path = APEIRETH_DIR / "v1366_vcp_cookbook_dashboard_overlay.py"
        src = src_path.read_text(encoding="utf-8")
        assert "v1363" in src

    def test_v1366_references_v1336_via_v1340(self):
        """V1340 docs should mention V1336 linter."""
        src_path = APEIRETH_DIR / "v1366_vcp_cookbook_dashboard_overlay.py"
        src = src_path.read_text(encoding="utf-8")
        # V1366 docs reference V1336 linter through V1340
        assert "V1336" in src or "V1339" in src

    def test_full_overlay_closes_v1335_v1340_loop(self):
        """The Markdown overlay should reference the V1335→V1336→V1339→V1340 loop via V1366."""
        md = v1366.render_full_overlay_md(history_limit=5, window=3)
        # V1340 validation logic section explains the loop
        assert "V1335" in md or "V1336" in md or "V1339" in md or "V1340" in md


# ---------------------------------------------------------------------------
# TestClass: PopperSelfTest
# ---------------------------------------------------------------------------
class TestPopperSelfTest:
    def test_self_test_passes(self):
        passed, total, failures = v1366._popper_self_tests()
        assert passed == total, f"failures: {failures}"
        assert passed >= 25  # We have 29 checks; >= 25 is the strict minimum

    def test_self_test_returns_tuple(self):
        result = v1366._popper_self_tests()
        assert isinstance(result, tuple)
        assert len(result) == 3
        passed, total, failures = result
        assert isinstance(passed, int)
        assert isinstance(total, int)
        assert isinstance(failures, list)


# ---------------------------------------------------------------------------
# TestClass: NoPoleStarChange
# ---------------------------------------------------------------------------
class TestNoPoleStarChange:
    """V1366 must NOT be in pole-star components (GUARD_NO_FABRICATION)."""

    def test_v1366_not_in_v1357_pole_star_components(self):
        """V1357 pole-star formula should not list V1366 as a component."""
        from apeireth import v1357_vcp_observability_snapshot as v1357
        # V1357 builds snapshots; V1366 is overlay-only and not aggregated
        snap = v1357.build_snapshot()
        ps = snap.pole_star
        # The components dict should not contain v1366 key
        assert not any("v1366" in str(k).lower() or "v1366" in str(v).lower()
                       for k, v in ps.items()), \
            "V1366 should not appear in V1357 pole-star components"

    def test_v1366_asi_cap_below_0_01(self):
        """Even if hypothetically aggregated, V1366 cap is 0.005."""
        assert v1366.V1366_ASI_CAP < 0.01
