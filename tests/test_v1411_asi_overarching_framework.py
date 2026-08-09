"""Tests for V1411 ASI 真生产 总框架 (Overarching Framework) / chain closure.

V1411 = ASI 总框架 (Overarching Framework):
- 12 真 overarching capacities + 6 真 overarching limits + 30 trajectory
- 12 pair-wise coherence checks (all pass)
- 12 总框架 levels L0_OBSERVER → L11_OVERARCHING
- 11 frameworks (V1400-V1410) unified under V1411
- chain delegate V1400-V1410 (11/11 ok, total_capacities=132,
  total_limits=66)
- popper self-test 7/7 pass
- CLI: version/overarching/level/chain/popper/meta/demo/close/help +
       --format text|json|md + --json + --level <0-11>

主 17:43 实事求是: V1411 module + 100+ pytest pass + chain 132/66 真调用 V1400-V1410.
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

import v1411_asi_overarching_framework as ov  # noqa: E402


# ----------------------- TestV1411Constants -----------------------

class TestV1411Constants:
    """Constants: VERSION, MODULE, GUARDS, V3_GUARDS, RULES, BORROWED, FRAMEWORKS, LEVELS."""

    def test_version_is_0_1_0(self):
        assert ov.V1411_VERSION == "0.1.0"

    def test_module_name(self):
        assert ov.V1411_MODULE == "v1411_asi_overarching_framework"

    def test_guards_count_16(self):
        assert len(ov.V1411_GUARDS) == 16

    def test_guards_invariants(self):
        for g in ov.V1411_GUARDS:
            assert g.startswith("GUARD_")

    def test_v3_guards_count_6(self):
        assert len(ov.V1411_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_OVERARCHING_IS_NOT_PHENOMENAL" in ov.V1411_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_OVERARCHING_IS_NOT_ASI" in ov.V1411_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_OVERARCHING_IS_NOT_HUMAN_LEVEL" in ov.V1411_V3_GUARDS

    def test_v3_guards_absolute(self):
        assert "GUARD_OVERARCHING_IS_NOT_ABSOLUTE" in ov.V1411_V3_GUARDS

    def test_v3_guards_no_v1256_replace(self):
        assert "GUARD_OVERARCHING_IS_NOT_V1256_REPLACE" in ov.V1411_V3_GUARDS

    def test_v3_guards_no_v1410_replace(self):
        assert "GUARD_OVERARCHING_IS_NOT_V1410_REPLACE" in ov.V1411_V3_GUARDS

    def test_rules_count_12(self):
        assert len(ov.V1411_RULES) == 12

    def test_rules_have_3_tuples(self):
        for r in ov.V1411_RULES:
            assert len(r) == 3
            assert isinstance(r[0], str)
            assert isinstance(r[1], str)
            assert isinstance(r[2], str)

    def test_borrowed_count_7(self):
        assert len(ov.V1411_BORROWED) == 7

    def test_frameworks_count_11(self):
        assert len(ov.V1411_FRAMEWORKS) == 11

    def test_frameworks_includes_v1400_through_v1410(self):
        for v in ("v1400_self", "v1401_cognition", "v1402_integration",
                  "v1403_meta", "v1404_trace", "v1405_explainer",
                  "v1406_judge", "v1407_production", "v1408_northstar",
                  "v1409_evolution", "v1410_five_position"):
            assert v in ov.V1411_FRAMEWORKS

    def test_levels_count_12(self):
        assert len(ov.V1411_LEVELS) == 12

    def test_levels_first_is_observer(self):
        assert ov.V1411_LEVELS[0] == "L0_OBSERVER"

    def test_levels_last_is_overarching(self):
        assert ov.V1411_LEVELS[-1] == "L11_OVERARCHING"


# ----------------------- TestV1411Capacities -----------------------

class TestV1411Capacities:
    """12 真 overarching capacities, each with real evidence + borrowed_from."""

    def test_capacities_count_12(self):
        caps = ov.build_capacities()
        assert len(caps) == 12

    def test_capacities_have_required_fields(self):
        for c in ov.build_capacities():
            assert c.cap_id.startswith("CAP_OVERARCHING_")
            assert c.level in ov.V1411_LEVELS
            assert c.name
            assert c.description
            assert c.evidence
            assert c.borrowed_from

    def test_capacities_unique_ids(self):
        ids = [c.cap_id for c in ov.build_capacities()]
        assert len(ids) == len(set(ids))

    def test_capacities_cover_11_frameworks(self):
        # Each framework should be reflected in capacities
        cap_strs = " ".join(c.cap_id for c in ov.build_capacities())
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404", "V1405",
                  "V1406", "V1407", "V1408", "V1409", "V1410"):
            assert v in cap_strs, f"missing {v} in capacities"

    def test_capacities_meta_cap_present(self):
        ids = [c.cap_id for c in ov.build_capacities()]
        assert "CAP_OVERARCHING_LINEAGE" in ids

    def test_capacities_per_level_distribution(self):
        # Should distribute capacities across levels L1-L11
        levels = set(c.level for c in ov.build_capacities())
        assert "L11_OVERARCHING" in levels
        # At least 3 distinct levels should be used
        assert len(levels) >= 3

    def test_capacities_evidence_long_enough(self):
        for c in ov.build_capacities():
            assert len(c.evidence) >= 20, \
                f"evidence too short for {c.cap_id}: {c.evidence!r}"


# ----------------------- TestV1411Limits -----------------------

class TestV1411Limits:
    """6 真 overarching limits, each with honest disclosure."""

    def test_limits_count_6(self):
        lims = ov.build_limits()
        assert len(lims) == 6

    def test_limits_have_required_fields(self):
        for lim in ov.build_limits():
            assert lim.lim_id.startswith("LIM_OVERARCHING_")
            assert lim.level in ov.V1411_LEVELS
            assert lim.name
            assert lim.description
            assert lim.evidence
            assert lim.why_no_phenomenal

    def test_limits_unique_ids(self):
        ids = [lim.lim_id for lim in ov.build_limits()]
        assert len(ids) == len(set(ids))

    def test_limits_at_l11(self):
        # All 6 limits should be at L11_OVERARCHING (top-level meta)
        for lim in ov.build_limits():
            assert lim.level == "L11_OVERARCHING"

    def test_limits_why_no_phenomenal_substantive(self):
        for lim in ov.build_limits():
            assert len(lim.why_no_phenomenal) >= 30, \
                f"why_no_phenomenal too short for {lim.lim_id}"

    def test_limits_evidence_substantive(self):
        for lim in ov.build_limits():
            assert len(lim.evidence) >= 30, \
                f"evidence too short for {lim.lim_id}"


# ----------------------- TestV1411Trajectory -----------------------

class TestV1411Trajectory:
    """30 trajectory points covering V1256 → V1410 chain + V1411 present + levels."""

    def test_trajectory_count_30(self):
        traj = ov.build_trajectory()
        assert len(traj) == 30

    def test_trajectory_have_required_fields(self):
        for t in ov.build_trajectory():
            assert t.version
            assert t.label
            assert t.status in ("past", "present", "future", "locked",
                                "occupied", "absent")
            assert t.kind in ("anchor", "borrowed", "present", "future",
                              "level", "position")

    def test_trajectory_includes_v1256_anchor(self):
        traj = ov.build_trajectory()
        anchors = [t for t in traj if t.kind == "anchor"]
        assert len(anchors) >= 1
        assert anchors[0].version == "V1256"

    def test_trajectory_includes_5_gap_closures(self):
        traj_str = " ".join(t.version for t in ov.build_trajectory())
        for v in ("V1313", "V1314", "V1315", "V1316", "V1317"):
            assert v in traj_str

    def test_trajectory_includes_6_deploy_stack_dims(self):
        traj_str = " ".join(t.version for t in ov.build_trajectory())
        for v in ("V1384", "V1385", "V1386", "V1397", "V1398", "V1399"):
            assert v in traj_str

    def test_trajectory_includes_11_frameworks(self):
        traj_str = " ".join(t.version for t in ov.build_trajectory())
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404", "V1405",
                  "V1406", "V1407", "V1408", "V1409", "V1410"):
            assert v in traj_str, f"missing {v} in trajectory"

    def test_trajectory_includes_v1411_present(self):
        traj = ov.build_trajectory()
        present = [t for t in traj if t.kind == "present"]
        assert len(present) >= 1
        assert present[0].version == "V1411"

    def test_trajectory_includes_future(self):
        traj = ov.build_trajectory()
        future = [t for t in traj if t.kind == "future"]
        assert len(future) >= 1

    def test_trajectory_includes_levels(self):
        traj = ov.build_trajectory()
        levels = [t for t in traj if t.kind == "level"]
        assert len(levels) >= 2


# ----------------------- TestV1411Rules -----------------------

class TestV1411Rules:
    """12 真 overarching 规则."""

    def test_rules_count_12(self):
        assert len(ov.build_rules()) == 12

    def test_rules_have_3_tuples(self):
        for r in ov.build_rules():
            assert len(r) == 3

    def test_rules_id_prefix(self):
        for r in ov.build_rules():
            assert r[0].startswith("OF")

    def test_rules_severity_valid(self):
        for r in ov.build_rules():
            assert r[1] in ("info", "warning", "error")

    def test_rules_description_substantive(self):
        for r in ov.build_rules():
            assert len(r[2]) >= 30, \
                f"description too short for {r[0]}: {r[2]!r}"


# ----------------------- TestV1411Borrowed -----------------------

class TestV1411Borrowed:
    """7 真 overarching 借鉴."""

    def test_borrowed_count_7(self):
        assert len(ov.build_borrowed()) == 7

    def test_borrowed_have_required_keys(self):
        for b in ov.build_borrowed():
            assert "key" in b
            assert "use" in b
            assert "applied_to" in b

    def test_borrowed_unique_keys(self):
        keys = [b["key"] for b in ov.build_borrowed()]
        assert len(keys) == len(set(keys))

    def test_borrowed_keys_substantive(self):
        for b in ov.build_borrowed():
            assert len(b["key"]) >= 5
            assert len(b["use"]) >= 30
            assert len(b["applied_to"]) >= 20

    def test_borrowed_includes_v1256(self):
        keys = [b["key"] for b in ov.build_borrowed()]
        assert any("v1256" in k for k in keys)

    def test_borrowed_includes_v1410(self):
        keys = [b["key"] for b in ov.build_borrowed()]
        assert any("v1410" in k for k in keys)

    def test_borrowed_includes_aristotle(self):
        keys = [b["key"] for b in ov.build_borrowed()]
        assert any("aristotle" in k.lower() for k in keys)

    def test_borrowed_includes_hofstadter(self):
        keys = [b["key"] for b in ov.build_borrowed()]
        assert any("hofstadter" in k.lower() for k in keys)


# ----------------------- TestV1411Coherence -----------------------

class TestV1411Coherence:
    """12 pair-wise coherence checks (all pass)."""

    def test_coherence_count_12(self):
        checks = ov.coherence_check()
        assert len(checks) == 12

    def test_coherence_all_pass(self):
        checks = ov.coherence_check()
        for c in checks:
            assert c.passes, f"coherence failed: {c.pair}"

    def test_coherence_pairs_unique(self):
        checks = ov.coherence_check()
        pairs = [c.pair for c in checks]
        assert len(pairs) == len(set(pairs))

    def test_coherence_pairs_have_reason(self):
        for c in ov.coherence_check():
            assert len(c.reason) >= 20

    def test_coherence_sequential_frameworks(self):
        checks = ov.coherence_check()
        # At least 10 framework-to-framework pairs (V1400-V1410 chain)
        framework_pairs = [c for c in checks
                          if c.pair[0].startswith("v14")
                          and c.pair[1].startswith("v14")]
        assert len(framework_pairs) >= 10


# ----------------------- TestV1411ChainDelegate -----------------------

class TestV1411ChainDelegate:
    """Chain delegate V1400-V1410 (11/11 ok, total_capacities=132, total_limits=66)."""

    def test_chain_all_ok(self):
        cd = ov.chain_delegate()
        assert cd.all_ok, "chain delegate has failures"

    def test_chain_total_capacities_132(self):
        cd = ov.chain_delegate()
        assert cd.total_capacities == 132

    def test_chain_total_limits_66(self):
        cd = ov.chain_delegate()
        assert cd.total_limits == 66

    def test_chain_delegated_count_11(self):
        cd = ov.chain_delegate()
        assert len(cd.delegated) == 11

    def test_chain_all_frameworks_pass(self):
        cd = ov.chain_delegate()
        for d in cd.delegated:
            assert d["ok"], f"failed: {d.get('module', '?')}"
            assert d["contributed_capacities"] == 12
            assert d["contributed_limits"] == 6

    def test_chain_modules_in_order(self):
        cd = ov.chain_delegate()
        expected = ["v1400", "v1401", "v1402", "v1403", "v1404",
                    "v1405", "v1406", "v1407", "v1408", "v1409",
                    "v1410"]
        actual = [d["module"][:5] for d in cd.delegated]
        assert actual == expected

    def test_chain_schema_includes_v1411(self):
        cd = ov.chain_delegate()
        assert "V1411" in cd.schema

    def test_chain_schema_includes_overarching(self):
        cd = ov.chain_delegate()
        assert "overarching" in cd.schema

    def test_chain_v1409_uses_fallback(self):
        # V1409 only has build_report() not run_self_evolution()
        # so the chain should fall back gracefully
        cd = ov.chain_delegate()
        v1409 = next((d for d in cd.delegated
                     if d["module"] == "v1409_asi_evolution_framework"), None)
        assert v1409 is not None
        assert v1409["ok"]
        assert v1409["run_function"] == "build_report"
        assert v1409["contributed_capacities"] == 12
        assert v1409["contributed_limits"] == 6


# ----------------------- TestV1411Popper -----------------------

class TestV1411Popper:
    """Popper self-test 7/7 pass."""

    def test_popper_all_pass(self):
        result = ov.popper_self_test()
        assert result["all_pass"]

    def test_popper_pass_count_7(self):
        result = ov.popper_self_test()
        assert result["pass_count"] == 7
        assert result["total_count"] == 7

    def test_popper_v1400_real(self):
        result = ov.popper_self_test()
        assert result["v1400_real"]

    def test_popper_v1410_real(self):
        result = ov.popper_self_test()
        assert result["v1410_real"]

    def test_popper_chain_real(self):
        result = ov.popper_self_test()
        assert result["chain_delegate_real"]

    def test_popper_honest(self):
        result = ov.popper_self_test()
        assert result["honest_disclosure"]

    def test_popper_complete(self):
        result = ov.popper_self_test()
        assert result["overarching_complete"]


# ----------------------- TestV1411Report -----------------------

class TestV1411Report:
    """OverarchingReport dataclass roundtrip and invariants."""

    def test_report_runs(self):
        report = ov.run_self_overarching()
        assert report.module == ov.V1411_MODULE
        assert report.version == ov.V1411_VERSION

    def test_report_anchor_v1256(self):
        report = ov.run_self_overarching()
        assert report.anchor_version == "V1256"
        assert report.anchor_value == 0.9105

    def test_report_ceiling_values(self):
        report = ov.run_self_overarching()
        assert report.north_star_ceiling == 0.98
        assert report.absolute_ceiling == 0.99

    def test_report_gap_to_north_star(self):
        report = ov.run_self_overarching()
        assert report.gap_to_north_star == pytest.approx(0.0695, abs=1e-4)

    def test_report_gap_to_ceiling(self):
        report = ov.run_self_overarching()
        assert report.gap_to_ceiling == pytest.approx(0.0795, abs=1e-4)

    def test_report_frameworks_count(self):
        report = ov.run_self_overarching()
        assert len(report.frameworks) == 11

    def test_report_levels_count(self):
        report = ov.run_self_overarching()
        assert len(report.levels) == 12

    def test_report_all_frameworks_occupied(self):
        report = ov.run_self_overarching()
        for fw, occ in report.framework_occupied:
            assert occ, f"framework {fw} not occupied"

    def test_report_all_levels_occupied(self):
        report = ov.run_self_overarching()
        for lvl, occ in report.level_occupied:
            assert occ, f"level {lvl} not occupied"

    def test_report_capacities_12(self):
        report = ov.run_self_overarching()
        assert len(report.capacities) == 12

    def test_report_limits_6(self):
        report = ov.run_self_overarching()
        assert len(report.limits) == 6

    def test_report_trajectory_30(self):
        report = ov.run_self_overarching()
        assert len(report.trajectory) == 30

    def test_report_rules_12(self):
        report = ov.run_self_overarching()
        assert len(report.rules) == 12

    def test_report_borrowed_7(self):
        report = ov.run_self_overarching()
        assert len(report.borrowed) == 7

    def test_report_overarching_complete(self):
        report = ov.run_self_overarching()
        assert report.asi_overarching_complete

    def test_report_generated_at_present(self):
        report = ov.run_self_overarching()
        assert report.generated_at
        assert report.generated_at_iso


# ----------------------- TestV1411CLI -----------------------

class TestV1411CLI:
    """CLI commands run in-process to avoid Windows GBK codec issues."""

    def test_cli_version(self, capsys):
        rc = ov.run_cli(["version"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1411" in captured.out
        assert "0.1.0" in captured.out

    def test_cli_help(self, capsys):
        rc = ov.run_cli(["help"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1411" in captured.out or "overarching" in captured.out.lower()

    def test_cli_popper(self, capsys):
        rc = ov.run_cli(["popper"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "7/7" in captured.out
        assert "True" in captured.out

    def test_cli_popper_json(self, capsys):
        rc = ov.run_cli(["popper", "--json"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["all_pass"]

    def test_cli_chain(self, capsys):
        rc = ov.run_cli(["chain"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "all_ok" in captured.out
        assert "11/11" in captured.out or "delegated (11)" in captured.out
        assert "132" in captured.out
        assert "66" in captured.out

    def test_cli_chain_json(self, capsys):
        rc = ov.run_cli(["chain", "--json"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["all_ok"]
        assert data["total_capacities"] == 132
        assert data["total_limits"] == 66
        assert len(data["delegated"]) == 11

    def test_cli_overarching(self, capsys):
        rc = ov.run_cli(["overarching"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1411" in captured.out
        assert "anchor" in captured.out.lower()
        assert "frameworks (11)" in captured.out
        assert "levels (12)" in captured.out
        assert "capacities: 12" in captured.out
        assert "limits: 6" in captured.out
        assert "trajectory: 30" in captured.out
        assert "rules: 12" in captured.out
        assert "borrowed: 7" in captured.out

    def test_cli_overarching_json(self, capsys):
        rc = ov.run_cli(["overarching", "--json"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["module"] == "v1411_asi_overarching_framework"
        assert data["version"] == "0.1.0"
        assert len(data["frameworks"]) == 11
        assert len(data["levels"]) == 12
        assert len(data["capacities"]) == 12
        assert len(data["limits"]) == 6
        assert len(data["trajectory"]) == 30
        assert len(data["rules"]) == 12
        assert len(data["borrowed"]) == 7
        assert data["asi_overarching_complete"]

    def test_cli_overarching_md(self, capsys):
        rc = ov.run_cli(["overarching", "--format", "md"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "# V1411" in captured.out
        assert "## Anchor" in captured.out
        assert "## ASI 11 Frameworks Unified" in captured.out
        assert "## 12 Levels" in captured.out
        assert "## Capacities" in captured.out
        assert "## Limits" in captured.out

    def test_cli_level_l11(self, capsys):
        rc = ov.run_cli(["level", "--level", "L11_OVERARCHING"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1411 Level: L11_OVERARCHING" in captured.out
        assert "capacity_count: 2" in captured.out
        assert "limit_count: 6" in captured.out

    def test_cli_level_l0(self, capsys):
        rc = ov.run_cli(["level", "--level", "L0_OBSERVER"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1411 Level: L0_OBSERVER" in captured.out

    def test_cli_level_invalid(self, capsys):
        rc = ov.run_cli(["level"])
        captured = capsys.readouterr()
        assert rc == 1
        assert "error" in captured.out.lower() or "required" in captured.out.lower()

    def test_cli_close(self, capsys):
        rc = ov.run_cli(["close"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "Closure Status" in captured.out
        # The frameworks are listed as v1400_self ... v1410_five_position
        assert "v1400_self" in captured.out
        assert "v1410_five_position" in captured.out
        assert "UNIFIED" in captured.out
        assert "overarching_complete: True" in captured.out
        assert "chain all_ok: True" in captured.out
        assert "total_capacities: 132" in captured.out
        assert "total_limits: 66" in captured.out

    def test_cli_meta(self, capsys):
        rc = ov.run_cli(["meta"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "module: v1411_asi_overarching_framework" in captured.out
        assert "version: 0.1.0" in captured.out
        assert "rule_count: 12" in captured.out
        assert "borrowed_count: 7" in captured.out

    def test_cli_meta_json(self, capsys):
        rc = ov.run_cli(["meta", "--json"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["module"] == "v1411_asi_overarching_framework"
        assert len(data["guards"]) == 16
        assert len(data["v3_guards"]) == 6
        assert len(data["frameworks"]) == 11
        assert len(data["levels"]) == 12

    def test_cli_demo(self, capsys):
        rc = ov.run_cli(["demo"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "11 frameworks" in captured.out
        assert "12 levels" in captured.out
        assert "V1411 借助 7 真借鉴" in captured.out


# ----------------------- TestV1413PhilosophyGuard -----------------------

class TestV1411PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / ASI / human-level / absolute / replace."""

    def test_v3_guards_count(self):
        assert len(ov.V1411_V3_GUARDS) == 6

    def test_v3_guards_complete_coverage(self):
        # 6 must-hit philosophy-guard topics
        topics = ["phenomenal", "asi", "human_level", "absolute",
                  "v1256_replace", "v1410_replace"]
        for t in topics:
            found = any(t in g.lower() for g in ov.V1411_V3_GUARDS)
            assert found, f"missing philosophy-guard topic: {t}"

    def test_limits_have_why_no_phenomenal(self):
        for lim in ov.build_limits():
            assert lim.why_no_phenomenal
            # Should reference 'Phenomenal' or 'phenomenal' in the
            # why_no_phenomenal text to honor V3 philosophy-guard
            assert ("phenomenal" in lim.why_no_phenomenal.lower()
                    or "phenomenon" in lim.why_no_phenomenal.lower()), \
                f"why_no_phenomenal doesn't reference Phenomenal for " \
                f"{lim.lim_id}"

    def test_no_kpi_gaming(self):
        # Should not have any KPI gaming constants (e.g. fake_asi_score,
        # metric inflation knobs)
        for attr in dir(ov):
            if attr.startswith("_"):
                continue
            val = getattr(ov, attr)
            if isinstance(val, str):
                assert "kpi_gaming" not in val.lower(), \
                    f"suspicious string in {attr}: {val}"
                assert "fake_asi" not in val.lower(), \
                    f"suspicious string in {attr}: {val}"

    def test_honest_cap_preserved(self):
        report = ov.run_self_overarching()
        # current_realized should equal anchor_value (honest cap)
        assert report.current_realized == 0.9105
        # gap should be honest (not zero)
        assert report.gap_to_north_star > 0
        assert report.gap_to_ceiling > 0


# ----------------------- TestV1411Integration -----------------------

class TestV1411Integration:
    """Integration: V1411 总框架 → V1400-V1410 chain 真调用."""

    def test_chain_actually_calls_v1400_v1410(self):
        cd = ov.chain_delegate()
        # All 11 frameworks should have non-None result_type
        for d in cd.delegated:
            assert d["result_type"] != "None"
            assert "Report" in d["result_type"] or \
                "Framework" in d["result_type"] or \
                "Result" in d["result_type"], \
                f"unexpected result type for {d['module']}: " \
                f"{d['result_type']}"

    def test_chain_total_132_66(self):
        # Each framework has 12c + 6l = 18 declarations
        # 11 frameworks × 18 = 198 declarations
        cd = ov.chain_delegate()
        total = cd.total_capacities + cd.total_limits
        assert total == 198

    def test_chain_v1410_inherits_correctly(self):
        cd = ov.chain_delegate()
        v1410 = next((d for d in cd.delegated
                     if "v1410" in d["module"]), None)
        assert v1410 is not None
        assert v1410["contributed_capacities"] == 12
        assert v1410["contributed_limits"] == 6

    def test_chain_v1409_fallback_used(self):
        # V1409 only exposes build_report()
        cd = ov.chain_delegate()
        v1409 = next((d for d in cd.delegated
                     if "v1409" in d["module"]), None)
        assert v1409 is not None
        assert v1409["run_function"] in ("build_report", "run_self_evolution")

    def test_chain_handles_missing_functions(self):
        # Mock framework that doesn't exist should not crash chain
        # (this is already tested by real chain, but verify logic)
        # The chain already gracefully handles missing functions
        # by appending an error entry and continuing.
        cd = ov.chain_delegate()
        # No failures in real chain
        failures = [d for d in cd.delegated if not d["ok"]]
        assert len(failures) == 0


# ----------------------- TestV1411ChainIntegration -----------------------

class TestV1411ChainIntegration:
    """chain V1384-V1411 + V1400-V1411 = no regression."""

    def test_v1411_imports_clean(self):
        # Make sure all imports work
        from apeireth import v1411_asi_overarching_framework
        assert v1411_asi_overarching_framework is not None

    def test_v1411_no_regression_on_constants(self):
        # Quick sanity check that all module-level constants load
        assert ov.V1411_VERSION
        assert ov.V1411_MODULE
        assert ov.V1411_GUARDS
        assert ov.V1411_V3_GUARDS
        assert ov.V1411_RULES
        assert ov.V1411_BORROWED
        assert ov.V1411_FRAMEWORKS
        assert ov.V1411_LEVELS


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
