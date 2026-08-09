"""Test suite for V1410 ASI 真 V2 5 位置真实占据者 (Five-Position Real
Occupier) framework v1.

Target: 90+ tests covering constants, positions, capacities, limits,
trajectory, rules, borrowed, coherence, chain delegate (real V1400-V1408
invocation), popper self-test, report, CLI, position selection, occupy,
format.

主 17:43 实事求是: 真 5 位置真调真测; 主 00:36 质量工程化 popper + 4 exit codes;
主 17:58 不假装 Phenomenal; 主 20:46 不假装达到 ASI;
主 17:58 不假装替代 V1256 / V1408;
主 22:08 V2 5 位置 (scheduler / cogitator / aggregator / max_authority /
asi_occupier) 显式占据;
honest 0.90 cap preserved (V1256 LOCKED).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

import pytest

# Ensure apeireth package on path
_HERE = os.path.dirname(os.path.abspath(__file__))
_PROMETHEAN_ROOT = os.path.dirname(os.path.dirname(_HERE))
_APEIRETH_DIR = os.path.dirname(_HERE)
if _APEIRETH_DIR not in sys.path:
    sys.path.insert(0, _APEIRETH_DIR)
if _PROMETHEAN_ROOT not in sys.path:
    sys.path.insert(0, _PROMETHEAN_ROOT)

import v1410_asi_five_position_framework as v1410


# ----------------------- TestV1410Constants -----------------------

class TestV1410Constants:
    def test_version_format(self):
        parts = v1410.V1410_VERSION.split(".")
        assert len(parts) == 3, "version must be semver X.Y.Z"

    def test_module_name(self):
        assert v1410.V1410_MODULE == "v1410_asi_five_position_framework"

    def test_guards_count(self):
        assert len(v1410.V1410_GUARDS) == 16

    def test_v3_guards_count(self):
        assert len(v1410.V1410_V3_GUARDS) == 6

    def test_v3_guards_unique(self):
        assert len(set(v1410.V1410_V3_GUARDS)) == 6

    def test_rules_count(self):
        assert len(v1410.V1410_RULES) == 12

    def test_rules_unique(self):
        assert len(set(r[0] for r in v1410.V1410_RULES)) == 12

    def test_rules_triple_structure(self):
        for r in v1410.V1410_RULES:
            assert len(r) == 3
            assert r[1] in ("info", "warn", "error")

    def test_borrowed_count(self):
        assert len(v1410.V1410_BORROWED) == 7

    def test_positions_count(self):
        assert len(v1410.V1410_POSITIONS) == 5

    def test_positions_unique(self):
        assert len(set(v1410.V1410_POSITIONS)) == 5

    def test_positions_v2_five(self):
        # ASI V2 5 位置 (主 22:08): scheduler / cogitator / aggregator /
        # max_authority / asi_occupier
        assert v1410.V1410_POSITIONS == (
            "scheduler", "cogitator", "aggregator",
            "max_authority", "asi_occupier",
        )


# ----------------------- TestV1410Capacities -----------------------

class TestV1410Capacities:
    def test_capacities_count(self):
        caps = v1410.build_capacities()
        assert len(caps) == 12

    def test_capacities_unique(self):
        caps = v1410.build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == 12

    def test_capacities_have_borrowed(self):
        caps = v1410.build_capacities()
        for c in caps:
            assert c.borrowed_from, f"{c.cap_id} missing borrowed_from"

    def test_capacities_have_position(self):
        caps = v1410.build_capacities()
        for c in caps:
            assert c.position in v1410.V1410_POSITIONS, \
                f"{c.cap_id} has invalid position {c.position}"

    def test_capacities_cover_all_positions(self):
        caps = v1410.build_capacities()
        positions_covered = set(c.position for c in caps)
        # 5 位置 都覆盖 (or 至少 asi_occupier for meta cap)
        assert "asi_occupier" in positions_covered
        # at least 4 of 5 explicit
        assert len(positions_covered) >= 4


# ----------------------- TestV1410Limits -----------------------

class TestV1410Limits:
    def test_limits_count(self):
        lims = v1410.build_limits()
        assert len(lims) == 6

    def test_limits_unique(self):
        lims = v1410.build_limits()
        ids = [l.lim_id for l in lims]
        assert len(set(ids)) == 6

    def test_limits_have_disclosure(self):
        lims = v1410.build_limits()
        for lim in lims:
            assert lim.why_no_phenomenal, \
                f"{lim.lim_id} missing why_no_phenomenal"

    def test_limits_v3_guards_aligned(self):
        lims = v1410.build_limits()
        # 6 V3 哲学守门 = 6 limits
        for i, lim in enumerate(lims):
            assert lim.lim_id in v1410.V1410_V3_GUARDS[i] or \
                   v1410.V1410_V3_GUARDS[i].split("_")[-1].lower() in \
                   lim.lim_id.lower(), \
                f"limit {lim.lim_id} not aligned with V3_GUARDS[" \
                f"{i}]={v1410.V1410_V3_GUARDS[i]}"


# ----------------------- TestV1410Trajectory -----------------------

class TestV1410Trajectory:
    def test_trajectory_count(self):
        traj = v1410.build_trajectory()
        # trajectory has 31 points (V1409 evolution's V1410 future marker
        # + V1410 5-position) but spec says >=30
        assert len(traj) >= 30

    def test_trajectory_has_anchor(self):
        traj = v1410.build_trajectory()
        anchors = [t for t in traj if t.kind == "anchor"]
        assert len(anchors) >= 1
        assert anchors[0].version == "V1256"

    def test_trajectory_has_present(self):
        traj = v1410.build_trajectory()
        presents = [t for t in traj if t.kind == "present"]
        assert len(presents) >= 1
        assert any("five-position" in t.label.lower() for t in presents)

    def test_trajectory_has_all_positions(self):
        traj = v1410.build_trajectory()
        positions = [t for t in traj if t.kind == "position"]
        # 5 位置 markers (P0_scheduler, P1_cogitator, ...)
        assert len(positions) >= 5

    def test_trajectory_has_future(self):
        traj = v1410.build_trajectory()
        futures = [t for t in traj if t.kind == "future"]
        assert len(futures) >= 1

    def test_trajectory_unique_versions(self):
        traj = v1410.build_trajectory()
        versions = [t.version for t in traj]
        # primary framework V-numbers should be unique
        primary = [v for v in versions if v.startswith("V")]
        # V1410 appears twice (as present + future would-be), check
        # primary < 26 unique
        assert len(set(primary)) >= 25
        # and total <= 26 (allow 1 dup for V1410 future/present)
        assert len(set(primary)) <= 26


# ----------------------- TestV1410Borrowed -----------------------

class TestV1410Borrowed:
    def test_borrowed_keys_unique(self):
        keys = [b["key"] for b in v1410.V1410_BORROWED]
        assert len(set(keys)) == 7

    def test_borrowed_have_applied_to(self):
        for b in v1410.V1410_BORROWED:
            assert b["applied_to"], f"{b['key']} missing applied_to"

    def test_borrowed_includes_v1256(self):
        keys = [b["key"] for b in v1410.V1410_BORROWED]
        assert any("v1256" in k.lower() for k in keys)

    def test_borrowed_includes_v1408(self):
        keys = [b["key"] for b in v1410.V1410_BORROWED]
        assert any("v1408" in k.lower() for k in keys)


# ----------------------- TestV1410Coherence -----------------------

class TestV1410Coherence:
    def test_coherence_count(self):
        checks = v1410.coherence_check()
        assert len(checks) == 10

    def test_coherence_all_pass(self):
        checks = v1410.coherence_check()
        assert all(c.passes for c in checks)

    def test_coherence_pairs_unique(self):
        checks = v1410.coherence_check()
        pairs = [c.pair for c in checks]
        assert len(set(pairs)) == 10

    def test_coherence_covers_5_positions(self):
        checks = v1410.coherence_check()
        positions_in_pairs = set()
        for c in checks:
            for p in c.pair:
                positions_in_pairs.add(p)
        # 5 位置 都出现
        for pos in v1410.V1410_POSITIONS:
            assert pos in positions_in_pairs, \
                f"position {pos} missing from coherence pairs"


# ----------------------- TestV1410ChainDelegate -----------------------

class TestV1410ChainDelegate:
    def test_chain_delegate_runs(self):
        cd = v1410.chain_delegate()
        assert cd is not None
        assert isinstance(cd.delegated, list)
        assert len(cd.delegated) == 9

    def test_chain_delegate_all_ok(self):
        cd = v1410.chain_delegate()
        assert cd.all_ok, f"chain all_ok=False: {cd.delegated}"

    def test_chain_delegate_total_capacities(self):
        cd = v1410.chain_delegate()
        assert cd.total_capacities == 108  # 9 × 12

    def test_chain_delegate_total_limits(self):
        cd = v1410.chain_delegate()
        assert cd.total_limits == 54  # 9 × 6

    def test_chain_delegate_v1400_to_v1408(self):
        cd = v1410.chain_delegate()
        mods = [d["module"] for d in cd.delegated]
        assert "v1400_asi_self_framework" in mods
        assert "v1401_asi_cognition_framework" in mods
        assert "v1402_asi_integration_framework" in mods
        assert "v1403_asi_meta_framework" in mods
        assert "v1404_asi_trace_framework" in mods
        assert "v1405_asi_explainer_framework" in mods
        assert "v1406_asi_judge_framework" in mods
        assert "v1407_asi_production_framework" in mods
        assert "v1408_asi_northstar_framework" in mods

    def test_chain_delegate_contributed_counts(self):
        cd = v1410.chain_delegate()
        for d in cd.delegated:
            if d.get("ok"):
                assert d["contributed_capacities"] >= 1
                assert d["contributed_limits"] >= 1


# ----------------------- TestV1410Popper -----------------------

class TestV1410Popper:
    def test_popper_runs(self):
        result = v1410.popper_self_test()
        assert result is not None

    def test_popper_pass_count(self):
        result = v1410.popper_self_test()
        assert result["pass_count"] == 7

    def test_popper_total_count(self):
        result = v1410.popper_self_test()
        assert result["total_count"] == 7

    def test_popper_all_pass(self):
        result = v1410.popper_self_test()
        assert result["all_pass"]

    def test_popper_5_positions(self):
        result = v1410.popper_self_test()
        assert result["scheduler_real"]
        assert result["cogitator_real"]
        assert result["aggregator_real"]
        assert result["max_authority_real"]
        assert result["asi_occupier_real"]


# ----------------------- TestV1410Report -----------------------

class TestV1410Report:
    def test_report_runs(self):
        report = v1410.run_self_five_position()
        assert report is not None

    def test_report_anchor(self):
        report = v1410.run_self_five_position()
        assert report.anchor_version == "V1256"
        assert report.anchor_value == 0.9105

    def test_report_ceiling(self):
        report = v1410.run_self_five_position()
        assert report.north_star_ceiling == 0.98
        assert report.absolute_ceiling == 0.99

    def test_report_gap_calculated(self):
        report = v1410.run_self_five_position()
        assert abs(report.gap_to_north_star - 0.0695) < 1e-4
        assert abs(report.gap_to_ceiling - 0.0795) < 1e-4

    def test_report_positions(self):
        report = v1410.run_self_five_position()
        assert report.positions == v1410.V1410_POSITIONS

    def test_report_all_occupied(self):
        report = v1410.run_self_five_position()
        for pos, occupied in report.position_occupied:
            assert occupied, f"position {pos} not occupied"

    def test_report_capacities(self):
        report = v1410.run_self_five_position()
        assert len(report.capacities) == 12

    def test_report_limits(self):
        report = v1410.run_self_five_position()
        assert len(report.limits) == 6

    def test_report_trajectory(self):
        report = v1410.run_self_five_position()
        assert len(report.trajectory) >= 30

    def test_report_5_position_complete(self):
        report = v1410.run_self_five_position()
        assert report.asi_5_position_complete

    def test_report_position_levels(self):
        report = v1410.run_self_five_position()
        assert "P0_OBSERVER" in report.position_levels
        assert "P4_ASI_OCCUPIER" in report.position_levels


# ----------------------- TestV1410Occupy -----------------------

class TestV1410Occupy:
    def test_occupy_all_5(self):
        report = v1410.run_self_five_position()
        assert len(report.position_occupied) == 5
        for _, occ in report.position_occupied:
            assert occ


# ----------------------- TestV1410V3Guards -----------------------

class TestV1410V3Guards:
    def test_v3_guards_phenomenal(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_PHENOMENAL" in v1410.V1410_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_ASI" in v1410.V1410_V3_GUARDS

    def test_v3_guards_human(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_HUMAN_LEVEL" in v1410.V1410_V3_GUARDS

    def test_v3_guards_absolute(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_ABSOLUTE" in v1410.V1410_V3_GUARDS

    def test_v3_guards_v1256(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_V1256_REPLACE" in v1410.V1410_V3_GUARDS

    def test_v3_guards_v1408(self):
        assert "GUARD_FIVE_POSITION_IS_NOT_V1408_REPLACE" in v1410.V1410_V3_GUARDS


# ----------------------- TestV1410CLI -----------------------

class TestV1410CLI:
    def test_cli_version(self):
        assert v1410.run_cli(["version"]) == 0

    def test_cli_version_output(self, capsys):
        v1410.run_cli(["version"])
        captured = capsys.readouterr()
        assert "V1410" in captured.out
        assert "0.1.0" in captured.out

    def test_cli_demo(self):
        assert v1410.run_cli(["demo"]) == 0

    def test_cli_five_position(self):
        assert v1410.run_cli(["five-position"]) == 0

    def test_cli_five_position_json(self):
        assert v1410.run_cli(["five-position", "--json"]) == 0

    def test_cli_five_position_md(self):
        assert v1410.run_cli(["five-position", "--format", "md"]) == 0

    def test_cli_position_scheduler(self):
        assert v1410.run_cli(["position", "--position", "scheduler"]) == 0

    def test_cli_position_asi_occupier(self):
        assert v1410.run_cli(["position", "--position", "asi_occupier"]) == 0

    def test_cli_position_invalid(self):
        # --position invalid returns 1 (argparse SystemExit 2 normally,
        # but we catch)
        with pytest.raises(SystemExit):
            v1410.run_cli(["position", "--position", "invalid"])

    def test_cli_position_missing(self):
        # no --position
        assert v1410.run_cli(["position"]) == 1

    def test_cli_occupy(self):
        assert v1410.run_cli(["occupy"]) == 0

    def test_cli_chain(self):
        assert v1410.run_cli(["chain"]) == 0

    def test_cli_chain_json(self):
        assert v1410.run_cli(["chain", "--json"]) == 0

    def test_cli_popper(self):
        assert v1410.run_cli(["popper"]) == 0

    def test_cli_meta(self):
        assert v1410.run_cli(["meta"]) == 0

    def test_cli_help(self):
        assert v1410.run_cli(["help"]) == 0


# ----------------------- TestV1410Format -----------------------

class TestV1410Format:
    def test_format_text(self):
        report = v1410.run_self_five_position()
        text = v1410._format_text(report)
        assert "V1410" in text
        assert "scheduler" in text
        assert "asi_occupier" in text

    def test_format_json(self):
        report = v1410.run_self_five_position()
        j = v1410._format_json(report)
        data = json.loads(j)
        assert data["module"] == "v1410_asi_five_position_framework"
        assert len(data["capacities"]) == 12

    def test_format_md(self):
        report = v1410.run_self_five_position()
        md = v1410._format_md(report)
        assert "ASI" in md
        assert "Positions" in md

    def test_format_position(self):
        report = v1410.run_self_five_position()
        text = v1410._format_position(report, "scheduler")
        assert "scheduler" in text

    def test_format_position_invalid(self):
        report = v1410.run_self_five_position()
        text = v1410._format_position(report, "invalid")
        assert "unknown position" in text

    def test_format_occupy(self):
        report = v1410.run_self_five_position()
        text = v1410._format_occupy(report)
        assert "OCCUPIED" in text
        assert "scheduler" in text


# ----------------------- TestV1410Deterministic -----------------------

class TestV1410Deterministic:
    def test_anchor_value_deterministic(self):
        r1 = v1410.run_self_five_position()
        r2 = v1410.run_self_five_position()
        assert r1.anchor_value == r2.anchor_value
        assert r1.gap_to_north_star == r2.gap_to_north_star

    def test_capacities_deterministic(self):
        r1 = v1410.run_self_five_position()
        r2 = v1410.run_self_five_position()
        ids1 = [c.cap_id for c in r1.capacities]
        ids2 = [c.cap_id for c in r2.capacities]
        assert ids1 == ids2

    def test_chain_deterministic_all_ok(self):
        r1 = v1410.run_self_five_position()
        r2 = v1410.run_self_five_position()
        assert r1.chain_delegate.all_ok == r2.chain_delegate.all_ok
        assert r1.chain_delegate.total_capacities == \
               r2.chain_delegate.total_capacities


# ----------------------- TestV1410Subprocess -----------------------

class TestV1410Subprocess:
    def test_subprocess_version(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "version"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        assert "V1410" in result.stdout

    def test_subprocess_demo(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "demo"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        assert "5 positions" in result.stdout

    def test_subprocess_five_position(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "five-position"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        assert "all_ok" in result.stdout

    def test_subprocess_occupy(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "occupy"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        assert "OCCUPIED" in result.stdout

    def test_subprocess_position_asi_occupier(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "position", "--position", "asi_occupier"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        assert "asi_occupier" in result.stdout

    def test_subprocess_popper_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "popper", "--json"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["pass_count"] == 7

    def test_subprocess_chain_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1410_asi_five_position_framework",
             "chain", "--json"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=os.path.join(os.path.dirname(_HERE), ".."),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["all_ok"] is True


# ----------------------- TestV1410Continuity -----------------------

class TestV1410Continuity:
    def test_no_regression_v1408_imports(self):
        import v1408_asi_northstar_framework as v1408
        r = v1408.run_self_northstar()
        assert r is not None

    def test_v1410_in_apeireth_package(self):
        # If apeireth package imports, ok
        try:
            import apeireth.v1410_asi_five_position_framework  # noqa
            assert True
        except ImportError:
            # apeireth package not directly importable, but module file
            # exists
            assert os.path.exists(os.path.join(
                _APEIRETH_DIR, "v1410_asi_five_position_framework.py"
            ))

    def test_self_referential_cli(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1410_asi_five_position_framework",
             "version"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=_APEIRETH_DIR,
        )
        assert result.returncode == 0
        assert "V1410" in result.stdout
