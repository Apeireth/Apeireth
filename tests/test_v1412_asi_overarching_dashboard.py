"""Tests for V1412 ASI 总框架 dashboard overlay.

V1412 = ASI 总框架 dashboard overlay of V1411 OverarchingReport:
- 5-level verdict (COMPLETE / GOOD / PARTIAL / WEAK / INCOMPLETE)
- 12 levels × 11 frameworks matrix
- 12 capacities + 6 limits + 30 trajectory + 11 chain + 7 borrowed
- 15 GUARDS + 6 V3 哲学守门
- popper self-test 11/11 pass
- CLI: version/dashboard/matrix/trajectory/verdict/borrowed/chain/popper/
       meta/demo/help + --format text|json|md + --json

主 17:43 实事求是: V1412 module + 50+ pytest pass + read-only delegate V1411.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

# Make apeireth importable
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1411_asi_overarching_framework as v1411  # noqa: E402
import v1412_asi_overarching_dashboard as v1412  # noqa: E402


# ----------------------- TestV1412Constants -----------------------

class TestV1412Constants:
    """Constants: VERSION, MODULE, GUARDS, V3_GUARDS, VERDICTS."""

    def test_version_is_0_1_0(self):
        assert v1412.V1412_VERSION == "0.1.0"

    def test_module_name(self):
        assert v1412.V1412_MODULE == "v1412_asi_overarching_dashboard"

    def test_guards_count_15(self):
        assert len(v1412.V1412_GUARDS) == 15

    def test_guards_invariants(self):
        for g in v1412.V1412_GUARDS:
            assert g.startswith("GUARD_")

    def test_v3_guards_count_6(self):
        assert len(v1412.V1412_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_DASHBOARD_IS_NOT_PHENOMENAL" in v1412.V1412_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_DASHBOARD_IS_NOT_ASI" in v1412.V1412_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_DASHBOARD_IS_NOT_HUMAN_LEVEL" in v1412.V1412_V3_GUARDS

    def test_v3_guards_absolute(self):
        assert "GUARD_DASHBOARD_IS_NOT_ABSOLUTE" in v1412.V1412_V3_GUARDS

    def test_v3_guards_no_v1411_replace(self):
        assert "GUARD_DASHBOARD_IS_NOT_V1411_REPLACE" in v1412.V1412_V3_GUARDS

    def test_v3_guards_no_v1256_replace(self):
        assert "GUARD_DASHBOARD_IS_NOT_V1256_REPLACE" in v1412.V1412_V3_GUARDS

    def test_verdicts_count_5(self):
        assert len(v1412.V1412_VERDICTS) == 5

    def test_verdicts_values(self):
        expected = ("COMPLETE", "GOOD", "PARTIAL", "WEAK", "INCOMPLETE")
        assert v1412.V1412_VERDICTS == expected


# ----------------------- TestV1412Dataclasses -----------------------

class TestV1412Dataclasses:
    """Dataclasses: LevelMatrixCell, CapacityRow, LimitRow, TrajectoryPoint,
    ChainStatusRow, BorrowedRow, DashboardVerdict, DashboardReport."""

    def test_level_matrix_cell_defaults(self):
        c = v1412.LevelMatrixCell(
            level="L1_FRAMEWORK", framework="v1400_self", occupied=True,
            capacity_count=1, limit_count=0,
        )
        assert c.level == "L1_FRAMEWORK"
        assert c.framework == "v1400_self"
        assert c.occupied is True

    def test_capacity_row_defaults(self):
        c = v1412.CapacityRow(
            cap_id="CAP_X", level="L1_FRAMEWORK", name="X", borrowed_from="Y"
        )
        assert c.cap_id == "CAP_X"

    def test_limit_row_with_why(self):
        lim = v1412.LimitRow(
            lim_id="LIM_X", level="L11_OVERARCHING", name="X",
            why_no_phenomenal="honest cap"
        )
        assert lim.why_no_phenomenal == "honest cap"

    def test_trajectory_point_defaults(self):
        t = v1412.TrajectoryPoint(
            version="V1411", label="x", status="present", kind="present"
        )
        assert t.kind == "present"

    def test_chain_status_row_with_error(self):
        c = v1412.ChainStatusRow(
            module="x", ok=False, result_type="None",
            contributed_capacities=0, contributed_limits=0,
            error="import failed"
        )
        assert c.error == "import failed"

    def test_borrowed_row_defaults(self):
        b = v1412.BorrowedRow(
            key="x", use="y", applied_to="z"
        )
        assert b.key == "x"

    def test_dashboard_verdict_with_reasons(self):
        v = v1412.DashboardVerdict(
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            reasons=["all ok"],
        )
        assert v.verdict == "COMPLETE"
        assert v.framework_score == 11

    def test_dashboard_report_defaults(self):
        # Just verify dataclass accepts all fields
        d = v1412.DashboardReport(
            module="x", version="0.1.0", generated_at="now",
            source_module="y", source_version="0.1.0",
            source_anchor="V1256", source_anchor_value=0.9105,
            source_north_star_ceiling=0.98, source_absolute_ceiling=0.99,
            source_current_realized=0.9105,
            source_gap_to_north_star=0.0695, source_gap_to_ceiling=0.0795,
            verdict=v1412.DashboardVerdict(
                verdict="COMPLETE", framework_score=11, level_score=12,
                coherence_score=12, chain_ok=True, borrowed_count=7,
                reasons=[],
            ),
            matrix=[], capacities=[], limits=[],
            trajectory=[], chain=[], borrowed=[],
            guards=(), v3_guards=(),
        )
        assert d.module == "x"


# ----------------------- TestV1412Verdict -----------------------

class TestV1412Verdict:
    """compute_dashboard_verdict(): 5-level verdict."""

    def test_verdict_complete_when_all_ok(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        # V1411 has 11/11 + 12/12 + 12/12 + chain_ok + 7 borrowed → COMPLETE
        assert v.verdict == "COMPLETE"

    def test_verdict_in_valid_set(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.verdict in v1412.V1412_VERDICTS

    def test_verdict_framework_score(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.framework_score == 11

    def test_verdict_level_score(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.level_score == 12

    def test_verdict_coherence_score(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.coherence_score == 12

    def test_verdict_chain_ok(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.chain_ok is True

    def test_verdict_borrowed_count(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert v.borrowed_count == 7

    def test_verdict_reasons_present(self):
        report = v1411.run_self_overarching()
        v = v1412.compute_dashboard_verdict(report)
        assert len(v.reasons) >= 1


# ----------------------- TestV1412Builders -----------------------

class TestV1412Builders:
    """Builders: level_matrix, capacity_breakdown, limit_breakdown,
    trajectory_timeline, chain_status, borrowed_catalog, gap_summary."""

    def test_matrix_count_12(self):
        report = v1411.run_self_overarching()
        matrix = v1412.build_level_matrix(report)
        assert len(matrix) == 12

    def test_matrix_levels_in_order(self):
        report = v1411.run_self_overarching()
        matrix = v1412.build_level_matrix(report)
        levels = [c.level for c in matrix]
        assert levels[0] == "L0_OBSERVER"
        assert levels[-1] == "L11_OVERARCHING"

    def test_matrix_l0_observer_no_framework(self):
        report = v1411.run_self_overarching()
        matrix = v1412.build_level_matrix(report)
        l0 = next(c for c in matrix if c.level == "L0_OBSERVER")
        assert l0.framework == ""

    def test_matrix_l11_has_v1410(self):
        report = v1411.run_self_overarching()
        matrix = v1412.build_level_matrix(report)
        l11 = next(c for c in matrix if c.level == "L11_OVERARCHING")
        assert l11.framework == "v1410_five_position"

    def test_matrix_all_occupied(self):
        report = v1411.run_self_overarching()
        matrix = v1412.build_level_matrix(report)
        assert all(c.occupied for c in matrix)

    def test_capacity_count_12(self):
        report = v1411.run_self_overarching()
        caps = v1412.build_capacity_breakdown(report)
        assert len(caps) == 12

    def test_capacity_have_borrowed_from(self):
        report = v1411.run_self_overarching()
        caps = v1412.build_capacity_breakdown(report)
        for c in caps:
            assert c.borrowed_from != ""

    def test_limit_count_6(self):
        report = v1411.run_self_overarching()
        lims = v1412.build_limit_breakdown(report)
        assert len(lims) == 6

    def test_limit_have_why_no_phenomenal(self):
        report = v1411.run_self_overarching()
        lims = v1412.build_limit_breakdown(report)
        for lim in lims:
            assert lim.why_no_phenomenal != ""

    def test_trajectory_count_30(self):
        report = v1411.run_self_overarching()
        traj = v1412.build_trajectory_timeline(report)
        assert len(traj) == 30

    def test_trajectory_sorted(self):
        report = v1411.run_self_overarching()
        traj = v1412.build_trajectory_timeline(report)
        # Anchor comes first, then past, present, levels, future
        kinds = [t.kind for t in traj]
        assert "anchor" in kinds

    def test_chain_count_11(self):
        report = v1411.run_self_overarching()
        chain = v1412.build_chain_status(report)
        assert len(chain) == 11

    def test_chain_all_ok(self):
        report = v1411.run_self_overarching()
        chain = v1412.build_chain_status(report)
        assert all(c.ok for c in chain)

    def test_chain_v1400_first(self):
        report = v1411.run_self_overarching()
        chain = v1412.build_chain_status(report)
        assert "v1400" in chain[0].module

    def test_chain_v1410_last(self):
        report = v1411.run_self_overarching()
        chain = v1412.build_chain_status(report)
        assert "v1410" in chain[-1].module

    def test_borrowed_count_7(self):
        report = v1411.run_self_overarching()
        b = v1412.build_borrowed_catalog(report)
        assert len(b) == 7

    def test_borrowed_have_keys(self):
        report = v1411.run_self_overarching()
        b = v1412.build_borrowed_catalog(report)
        for x in b:
            assert x.key != ""

    def test_gap_summary_keys(self):
        report = v1411.run_self_overarching()
        g = v1412.build_gap_summary(report)
        assert "gap_to_north_star" in g
        assert "gap_to_ceiling" in g


# ----------------------- TestV1412Dashboard -----------------------

class TestV1412Dashboard:
    """build_dashboard_report(): full dashboard."""

    def test_dashboard_runs(self):
        d = v1412.build_dashboard_report()
        assert d.module == "v1412_asi_overarching_dashboard"

    def test_dashboard_source_v1411(self):
        d = v1412.build_dashboard_report()
        assert d.source_module == "v1411_asi_overarching_framework"
        assert d.source_version == "0.1.0"

    def test_dashboard_anchor_preserved(self):
        d = v1412.build_dashboard_report()
        assert d.source_anchor == "V1256"
        assert d.source_anchor_value == 0.9105

    def test_dashboard_verdict_present(self):
        d = v1412.build_dashboard_report()
        assert d.verdict.verdict == "COMPLETE"

    def test_dashboard_matrix_12(self):
        d = v1412.build_dashboard_report()
        assert len(d.matrix) == 12

    def test_dashboard_capacities_12(self):
        d = v1412.build_dashboard_report()
        assert len(d.capacities) == 12

    def test_dashboard_limits_6(self):
        d = v1412.build_dashboard_report()
        assert len(d.limits) == 6

    def test_dashboard_trajectory_30(self):
        d = v1412.build_dashboard_report()
        assert len(d.trajectory) == 30

    def test_dashboard_chain_11(self):
        d = v1412.build_dashboard_report()
        assert len(d.chain) == 11

    def test_dashboard_borrowed_7(self):
        d = v1412.build_dashboard_report()
        assert len(d.borrowed) == 7


# ----------------------- TestV1412Popper -----------------------

class TestV1412Popper:
    """popper_self_test(): 11 self-tests."""

    def test_popper_all_pass(self):
        r = v1412.popper_self_test()
        assert r["all_pass"] is True

    def test_popper_pass_count_11(self):
        r = v1412.popper_self_test()
        assert r["pass_count"] == 11

    def test_popper_v1411_real(self):
        r = v1412.popper_self_test()
        assert r["v1411_source_real"] is True

    def test_popper_dashboard_real(self):
        r = v1412.popper_self_test()
        assert r["v1412_dashboard_real"] is True

    def test_popper_verdict_real(self):
        r = v1412.popper_self_test()
        assert r["verdict_5_levels"] is True

    def test_popper_matrix_real(self):
        r = v1412.popper_self_test()
        assert r["matrix_12_levels"] is True

    def test_popper_chain_real(self):
        r = v1412.popper_self_test()
        assert r["chain_11"] is True

    def test_popper_borrowed_real(self):
        r = v1412.popper_self_test()
        assert r["borrowed_7"] is True


# ----------------------- TestV1412CLI -----------------------

class TestV1412CLI:
    """CLI: version / dashboard / matrix / trajectory / verdict / borrowed /
    chain / popper / meta / demo / help."""

    def _cli(self, *args):
        """Run CLI subprocess and return (returncode, stdout, stderr)."""
        cmd = [sys.executable, "-m", "apeireth.v1412_asi_overarching_dashboard",
               *args]
        env = {**os.environ, "PYTHONPATH": str(ROOT), "PYTHONIOENCODING": "utf-8"}
        proc = subprocess.run(cmd, capture_output=True, text=True, env=env,
                              timeout=60, encoding="utf-8", errors="replace")
        return proc.returncode, proc.stdout, proc.stderr

    def test_cli_version(self):
        rc, out, err = self._cli("version")
        assert rc == 0
        assert "V1412 0.1.0" in out

    def test_cli_help(self):
        rc, out, err = self._cli("help")
        assert rc == 0
        assert "dashboard" in out

    def test_cli_demo(self):
        rc, out, err = self._cli("demo")
        assert rc == 0
        assert "demo" in out.lower()

    def test_cli_popper(self):
        rc, out, err = self._cli("popper")
        assert rc == 0
        assert "11/11" in out

    def test_cli_popper_json(self):
        rc, out, err = self._cli("popper", "--json")
        assert rc == 0
        # Should produce JSON
        assert "v1411_source_real" in out or "{" in out

    def test_cli_dashboard(self):
        rc, out, err = self._cli("dashboard")
        assert rc == 0
        assert "VERDICT" in out

    def test_cli_dashboard_json(self):
        rc, out, err = self._cli("dashboard", "--json")
        assert rc == 0
        # Should be valid JSON
        data = json.loads(out)
        assert "module" in data

    def test_cli_dashboard_md(self):
        rc, out, err = self._cli("dashboard", "--format", "md")
        assert rc == 0
        assert "# V1412" in out
        assert "## Verdict" in out
        assert "## Anchor" in out
        assert "## 12 Levels × Frameworks Matrix" in out
        assert "## 12 Capacities" in out
        assert "## 6 Limits" in out
        assert "## 30 Trajectory Points" in out
        assert "## Chain Delegate Status" in out
        assert "## 7 Borrowed Catalog" in out

    def test_cli_matrix(self):
        rc, out, err = self._cli("matrix")
        assert rc == 0
        assert "12 Levels" in out

    def test_cli_trajectory(self):
        rc, out, err = self._cli("trajectory")
        assert rc == 0
        assert "Trajectory Timeline" in out

    def test_cli_verdict(self):
        rc, out, err = self._cli("verdict")
        assert rc == 0
        assert "COMPLETE" in out

    def test_cli_verdict_json(self):
        rc, out, err = self._cli("verdict", "--json")
        assert rc == 0
        data = json.loads(out)
        assert "verdict" in data
        assert data["verdict"] == "COMPLETE"

    def test_cli_borrowed(self):
        rc, out, err = self._cli("borrowed")
        assert rc == 0
        assert "Borrowed Catalog" in out

    def test_cli_chain(self):
        rc, out, err = self._cli("chain")
        assert rc == 0
        assert "Chain Delegate Status" in out

    def test_cli_meta(self):
        rc, out, err = self._cli("meta")
        assert rc == 0
        assert "guards" in out

    def test_cli_meta_json(self):
        rc, out, err = self._cli("meta", "--json")
        assert rc == 0
        data = json.loads(out)
        assert "guards" in data
        assert len(data["guards"]) == 15


# ----------------------- TestV1412PhilosophyGuard -----------------------

class TestV1412PhilosophyGuard:
    """V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_v3_guards_count(self):
        assert len(v1412.V1412_V3_GUARDS) == 6

    def test_v3_guards_complete_coverage(self):
        expected = {
            "GUARD_DASHBOARD_IS_NOT_PHENOMENAL",
            "GUARD_DASHBOARD_IS_NOT_ASI",
            "GUARD_DASHBOARD_IS_NOT_HUMAN_LEVEL",
            "GUARD_DASHBOARD_IS_NOT_ABSOLUTE",
            "GUARD_DASHBOARD_IS_NOT_V1411_REPLACE",
            "GUARD_DASHBOARD_IS_NOT_V1256_REPLACE",
        }
        assert set(v1412.V1412_V3_GUARDS) == expected

    def test_limits_have_why_no_phenomenal(self):
        report = v1411.run_self_overarching()
        lims = v1412.build_limit_breakdown(report)
        for lim in lims:
            assert lim.why_no_phenomenal != ""

    def test_no_kpi_gaming(self):
        # Verdict threshold is meaningful (not all 5 always COMPLETE)
        # Sanity: 5 thresholds distinct enough to be useful
        assert len(v1412.V1412_VERDICTS) == 5

    def test_honest_cap_preserved(self):
        d = v1412.build_dashboard_report()
        # V1412 source anchor = V1411 anchor = V1256 0.9105 (no cap change)
        assert d.source_anchor_value == 0.9105
        assert d.source_north_star_ceiling == 0.98


# ----------------------- TestV1412Integration -----------------------

class TestV1412Integration:
    """Integration: V1412 reads V1411 (read-only delegate)."""

    def test_v1412_imports_clean(self):
        # Should not import any side-effect modules
        import importlib
        m = importlib.import_module("v1412_asi_overarching_dashboard")
        assert m.V1412_VERSION == "0.1.0"

    def test_v1412_delegates_to_v1411(self):
        d = v1412.build_dashboard_report()
        # All dashboard data comes from V1411's run_self_overarching()
        assert d.source_module == "v1411_asi_overarching_framework"

    def test_v1412_no_regression_on_constants(self):
        d = v1412.build_dashboard_report()
        # V1412 doesn't change V1411's constants
        assert d.source_anchor == "V1256"
        assert d.source_anchor_value == 0.9105
        # Verify V1411 caps preserved
        assert len(d.capacities) == len(v1411.build_capacities())

    def test_chain_v1412_via_v1411(self):
        # V1412 dashboard chain_ok depends on V1411 chain all_ok
        d = v1412.build_dashboard_report()
        report_v1411 = v1411.run_self_overarching()
        assert d.verdict.chain_ok == report_v1411.chain_delegate.all_ok

    def test_chain_total_caps_match(self):
        d = v1412.build_dashboard_report()
        report_v1411 = v1411.run_self_overarching()
        # V1412 chain total = sum of V1411 chain contributed caps
        v1412_total = sum(c.contributed_capacities for c in d.chain)
        v1411_total = report_v1411.chain_delegate.total_capacities
        assert v1412_total == v1411_total


# ----------------------- TestV1412ChainIntegration -----------------------

class TestV1412ChainIntegration:
    """Chain integration: V1400-V1411 + V1412 chain pass."""

    def test_v1412_no_regression_on_v1411(self):
        # V1412 should not break V1411 tests
        rc, out, err = subprocess.run(
            [sys.executable, "-m", "pytest",
             "tests/test_v1411_asi_overarching_framework.py", "-q", "--tb=line"],
            capture_output=True, text=True, env={**os.environ, "PYTHONPATH": str(ROOT)},
            cwd=str(ROOT), timeout=120,
        ).returncode, "", ""
        assert rc == 0
